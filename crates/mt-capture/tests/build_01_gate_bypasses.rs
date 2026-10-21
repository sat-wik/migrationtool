#![allow(clippy::unwrap_used, clippy::expect_used)]
//! BUILD-01 and BUILD-02, plan 02-21 (gap G-02-8): the review's gate bypasses
//! CR-01, CR-02 and CR-03, reproduced exactly as 02-REVIEW.md writes them.
//!
//! Every case must be refused at both ends: by `canonicalise` (the record
//! stage) and by `CcPolicy::check_entry` (replay, before anything is deleted,
//! copied or spawned). The tests use only API that exists before and after the
//! allow-list gate, so a failure here is an accepted bypass, not a compile error.
//! Each test reports every case it found accepted, not only the first one.

use std::path::PathBuf;

use mt_capture::cc::{CcEntry, CcError, CcPolicy, RawEntry, canonicalise};

/// The run root of the review's reproduction.
const ROOT: &str = "/work/run/t/default";
const GCC: &str = "/opt/arm-gnu-toolchain/bin/arm-none-eabi-gcc";

fn policy() -> CcPolicy {
    CcPolicy {
        allowed_compilers: vec![GCC.to_owned()],
        run_root: PathBuf::from(ROOT),
    }
}

fn strings(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| (*part).to_owned()).collect()
}

/// `[GCC, <case>, -c, -o, out/a.o, a.c]`.
fn base_with(case: &[&str]) -> Vec<String> {
    let mut arguments = vec![GCC.to_owned()];
    arguments.extend(strings(case));
    arguments.extend(strings(&["-c", "-o", "out/a.o", "a.c"]));
    arguments
}

fn raw_of(arguments: &[String]) -> RawEntry {
    RawEntry {
        directory: ROOT.to_owned(),
        file: "a.c".to_owned(),
        arguments: Some(arguments.to_vec()),
        command: None,
        output: None,
    }
}

/// A replay entry with its output set by hand, as the record stage would have
/// written it: no gate runs while it is built.
fn entry_of(arguments: &[String]) -> CcEntry {
    CcEntry {
        directory: ROOT.to_owned(),
        file: "a.c".to_owned(),
        arguments: arguments.to_vec(),
        output: format!("{ROOT}/out/a.o"),
    }
}

fn at_capture(arguments: &[String]) -> Result<(), CcError> {
    canonicalise(&[raw_of(arguments)], &policy(), &|_| true).map(|_| ())
}

fn at_replay(arguments: &[String]) -> Result<(), CcError> {
    policy().check_entry(&entry_of(arguments))
}

/// The cases that one end or the other accepted, one line each.
fn accepted(cases: &[Vec<String>]) -> Vec<String> {
    let mut found = Vec::new();
    for arguments in cases {
        if at_capture(arguments).is_ok() {
            found.push(format!("accepted at capture: {:?}", &arguments[1..]));
        }
        if at_replay(arguments).is_ok() {
            found.push(format!("accepted at replay: {:?}", &arguments[1..]));
        }
    }
    found
}

#[test]
fn build_01_gate_refuses_long_option_spellings() {
    // CR-01, the review's table: the driver rewrites `--<name>` into
    // `-f<name>` before cc1 sees it.
    let cases: Vec<Vec<String>> = [
        "--plugin=/work/run/t/default/evil.so",
        "--compare-debug=-wrapper,/work/run/t/default/w.sh",
        "--opt-info-all=/out/leak.txt",
        "--dump-tree-all=/out/leak",
        "--dump-final-insns=/out/f",
        "--profile-use=/out/p",
        "--profile-dir=/out/pd",
    ]
    .iter()
    .map(|case| base_with(&[case]))
    .collect();
    let found = accepted(&cases);
    assert!(found.is_empty(), "CR-01 bypasses:\n{}", found.join("\n"));
}

#[test]
fn build_01_gate_refuses_assembler_carrier_spellings() {
    // CR-02 and its fix list: GNU as reads long options with one dash and
    // abbreviated, so a carrier can write a dependency file anywhere.
    let cases: Vec<Vec<String>> = [
        &["-Wa,-MD,/out/e.d"][..],
        &["-Wa,-MD=/out/e.d"],
        &["-Wa,--M,/out/e.d"],
        &["-Wa,-M,/out/e.d"],
        &["-Xassembler", "-MD", "-Xassembler", "/out/e.d"],
        &["--for-assembler=-MD,/out/e.d"],
    ]
    .iter()
    .map(|case| base_with(case))
    .collect();
    let found = accepted(&cases);
    assert!(found.is_empty(), "CR-02 bypasses:\n{}", found.join("\n"));
}

#[test]
fn build_01_gate_decides_compile_only_from_the_walk() {
    // CR-03: `-MT` takes `-c` as its target, so the driver links.
    let review = strings(&[
        GCC,
        "-MD",
        "-MT",
        "-c",
        "a.c",
        "-o",
        "out/a.o",
        "-nostdlib",
        "-nostartfiles",
        "-Wl,-Map=/out/m.map",
        "-Wl,-e,0",
    ]);
    let minimal = strings(&[GCC, "-MD", "-MT", "-c", "a.c", "-o", "out/a.o"]);
    let plugin = strings(&[
        GCC,
        "-c",
        "a.c",
        "-o",
        "out/a.o",
        "-Wl,-plugin,/work/run/t/default/p.so",
    ]);
    let mut found = accepted(&[review, minimal.clone(), plugin]);
    // The minimal vector is refused for the reason that matters.
    for (end, result) in [
        ("capture", at_capture(&minimal)),
        ("replay", at_replay(&minimal)),
    ] {
        if let Err(error) = result
            && !matches!(error, CcError::NotCompileOnly)
        {
            found.push(format!(
                "minimal CR-03 vector at {end}: expected NotCompileOnly, got {error:?}"
            ));
        }
    }
    // IN-13: replay repeats the compile-only test itself.
    let no_compile = strings(&[GCC, "-o", "out/a.o", "a.c"]);
    match at_replay(&no_compile) {
        Err(CcError::NotCompileOnly) => {}
        other => found.push(format!(
            "an entry without -c at replay: expected NotCompileOnly, got {other:?}"
        )),
    }
    assert!(found.is_empty(), "CR-03 / IN-13:\n{}", found.join("\n"));
}

#[test]
fn build_01_gate_refuses_response_files_in_operand_position() {
    // T-02-72, a guard. The driver expands every argv element that starts
    // with `@` before it parses options, operands included: it splices the
    // file's words in place of the argument, so the first word becomes the
    // operand and the rest are parsed as options. Measured at execution with
    // the host arm-none-eabi-gcc 13.2.1 (`-###`, which runs nothing) and the
    // host cc 13.3.0 (a real compile): with x.rsp holding
    // `-I. -wrapper <script>`, each of `-I @x.rsp`, `-MT @x.rsp`, `-MF @x.rsp`
    // and `-o @x.rsp` ran every subcommand under the script.
    let cases: Vec<Vec<String>> = [
        ["-I", "@x.rsp"],
        ["-MT", "@x.rsp"],
        ["-MF", "@x.rsp"],
        ["-o", "@x.rsp"],
    ]
    .iter()
    .map(|case| base_with(case))
    .collect();
    let found = accepted(&cases);
    assert!(
        found.is_empty(),
        "response files in operand position:\n{}",
        found.join("\n")
    );
}
