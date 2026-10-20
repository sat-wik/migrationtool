#![allow(clippy::unwrap_used, clippy::expect_used)]
//! BUILD-01 canonical `compile_commands.json`: Bear and CMake exports become one
//! sorted, gated form that names exactly the objects the build made.
//!
//! The fixtures are the recorded shapes of the two exporters (RESEARCH Pattern
//! 6): Bear's `arguments` arrays with and without `-o`, and CMake's `command`
//! strings with a sub-directory target whose `output` is relative to the
//! top-level build directory. Existence of an object is decided by a set, not by
//! the filesystem.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use mt_capture::cc::{
    Canonical, CcEntry, CcError, CcPolicy, HELPER_OPTION_PREFIXES, RawEntry, canonical_output,
    canonicalise, check_write_paths, find_helper_option, parse_raw, to_json,
};
use mt_capture::manifest::BASELINE_CFLAGS;

const ROOT: &str = "/work/run/fixture/default";
const GCC: &str = "/opt/arm-gnu-toolchain/bin/arm-none-eabi-gcc";

const LZ4_DIR: &str = "/work/run/fixture/default/src/cachedObjs/ab12cd";
const CORE_OBJ: &str = "/work/run/fixture/default/build/CMakeFiles/core.dir/core.c.obj";
const PAHO_OBJ: &str = "/work/run/fixture/default/build/MQTTPacket/src/CMakeFiles/paho-embed.dir/MQTTConnectClient.c.obj";
const DUMP_FILE: &str = "/work/run/fixture/default/src/tools/dump.c";

fn policy() -> CcPolicy {
    CcPolicy {
        allowed_compilers: vec![GCC.to_owned()],
        run_root: PathBuf::from(ROOT),
    }
}

fn fixture(name: &str) -> Vec<RawEntry> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/cc")
        .join(name);
    parse_raw(&std::fs::read(path).unwrap()).unwrap()
}

fn exists_in(objects: &[&str]) -> impl Fn(&Path) -> bool {
    let set: BTreeSet<PathBuf> = objects.iter().map(PathBuf::from).collect();
    move |path: &Path| set.contains(path)
}

fn raw(directory: &str, file: &str, arguments: &[&str]) -> RawEntry {
    RawEntry {
        directory: directory.to_owned(),
        file: file.to_owned(),
        arguments: Some(arguments.iter().map(|s| (*s).to_owned()).collect()),
        command: None,
        output: None,
    }
}

fn all_exist(_: &Path) -> bool {
    true
}

fn canon(raw: &[RawEntry]) -> Result<Canonical, CcError> {
    canonicalise(raw, &policy(), &all_exist)
}

#[test]
fn build_01_cmake_command_strings_split_into_argv() {
    let entries = fixture("cmake-raw.json");
    let result = canonicalise(&entries, &policy(), &exists_in(&[CORE_OBJ, PAHO_OBJ])).unwrap();
    let core = result
        .entries
        .iter()
        .find(|entry| entry.output == CORE_OBJ)
        .unwrap();
    // The quotes are consumed exactly as a shell would, and no shell ran.
    assert_eq!(
        core.arguments,
        [
            GCC,
            "-DNAME=a b",
            "-I/work/run/fixture/default/src/include",
            "-mthumb",
            "-Os",
            "-o",
            "CMakeFiles/core.dir/core.c.obj",
            "-c",
            "/work/run/fixture/default/src/core.c",
        ]
    );
    // An injected command separator is just text inside one argument.
    let mut hostile = entries[0].clone();
    hostile.command = Some(format!("{GCC} -c -o x.o 'a; touch /tmp/pwned' core.c"));
    let result = canon(&[hostile]).unwrap();
    assert_eq!(result.entries[0].arguments[4], "a; touch /tmp/pwned");
}

#[test]
fn build_01_bear_arguments_are_kept_verbatim() {
    let entries = fixture("bear-raw.json");
    let result = canon(&entries).unwrap();
    for entry in &result.entries {
        let original = entries.iter().find(|raw| raw.file == entry.file).unwrap();
        assert_eq!(Some(&entry.arguments), original.arguments.as_ref());
        assert_eq!(entry.directory, original.directory);
    }
    assert_eq!(result.entries.len(), 2);
}

#[test]
fn build_01_default_output_is_source_stem_object() {
    let entries = fixture("bear-raw.json");
    let result = canon(&entries).unwrap();
    // lz4's Makefile compiles with no -o inside cachedObjs/<hash>.
    let lz4 = result
        .entries
        .iter()
        .find(|entry| entry.file.ends_with("lz4.c"))
        .unwrap();
    assert_eq!(lz4.output, format!("{LZ4_DIR}/lz4.o"));
    // `-o` and the joined `-o<path>` form both resolve against the directory.
    let spaced = raw(
        &format!("{ROOT}/b"),
        "x.c",
        &[GCC, "-c", "-o", "out/x.o", "x.c"],
    );
    let joined = raw(
        &format!("{ROOT}/b"),
        "y.c",
        &[GCC, "-c", "-oout/y.o", "y.c"],
    );
    let result = canon(&[spaced, joined]).unwrap();
    let outputs: Vec<&str> = result.entries.iter().map(|e| e.output.as_str()).collect();
    assert_eq!(
        outputs,
        [format!("{ROOT}/b/out/x.o"), format!("{ROOT}/b/out/y.o")]
    );
}

#[test]
fn build_01_cmake_output_field_is_not_trusted() {
    let entries = fixture("cmake-raw.json");
    let result = canonicalise(&entries, &policy(), &exists_in(&[CORE_OBJ, PAHO_OBJ])).unwrap();
    // Paho-like: `output` is relative to the top-level build directory, the
    // object is next to the sub-directory target.
    let paho = result
        .entries
        .iter()
        .find(|entry| entry.file.ends_with("MQTTConnectClient.c"))
        .unwrap();
    assert_eq!(paho.output, PAHO_OBJ);
    // An exported output that contradicts the arguments is ignored.
    let mut liar = raw(ROOT, "a.c", &[GCC, "-c", "-o", "real.o", "a.c"]);
    liar.output = Some("/etc/passwd".to_owned());
    let result = canon(&[liar]).unwrap();
    assert_eq!(result.entries[0].output, format!("{ROOT}/real.o"));
}

#[test]
fn build_01_cmake_entries_for_unbuilt_targets_are_dropped() {
    let entries = fixture("cmake-raw.json");
    let result = canonicalise(&entries, &policy(), &exists_in(&[CORE_OBJ, PAHO_OBJ])).unwrap();
    assert_eq!(result.entries_exported, 3);
    assert_eq!(result.entries_built, 2);
    assert_eq!(result.entries.len(), 2);
    assert_eq!(result.dropped_unbuilt, [DUMP_FILE]);
}

#[test]
fn build_01_entries_sorted_canonically() {
    let mut entries = fixture("cmake-raw.json");
    entries.extend(fixture("bear-raw.json"));
    let forward = canon(&entries).unwrap();
    let mut reversed = entries.clone();
    reversed.reverse();
    let mut rotated = entries.clone();
    rotated.rotate_left(2);

    let expected = to_json(&forward.entries).unwrap();
    assert_eq!(
        to_json(&canon(&reversed).unwrap().entries).unwrap(),
        expected
    );
    assert_eq!(
        to_json(&canon(&rotated).unwrap().entries).unwrap(),
        expected
    );
    assert!(expected.ends_with("]\n"));
    let files: Vec<&str> = forward.entries.iter().map(|e| e.file.as_str()).collect();
    let mut sorted = files.clone();
    sorted.sort_unstable();
    assert_eq!(files, sorted);
}

#[test]
fn build_01_rejects_non_allowlisted_compiler() {
    let entry = raw(ROOT, "a.c", &["/usr/bin/evil", "-c", "a.c"]);
    let error = canon(&[entry]).unwrap_err();
    assert!(
        matches!(&error, CcError::CompilerNotAllowed { argv0 } if argv0 == "/usr/bin/evil"),
        "{error:?}"
    );
    // The same through a command string.
    let mut entry = raw(ROOT, "a.c", &[]);
    entry.arguments = None;
    entry.command = Some("evilcc -c a.c".to_owned());
    assert!(matches!(
        canon(&[entry]).unwrap_err(),
        CcError::CompilerNotAllowed { .. }
    ));
}

#[test]
fn build_01_rejects_lto_and_profile_flags() {
    for flag in [
        "-flto",
        "-flto=auto",
        "-fprofile-generate",
        "-fprofile-use=x.gcda",
    ] {
        let entry = raw(ROOT, "a.c", &[GCC, flag, "-c", "a.c"]);
        let error = canon(&[entry]).unwrap_err();
        assert!(
            matches!(&error, CcError::ForbiddenFlag { flag: found } if found == flag),
            "{flag}: {error:?}"
        );
    }
}

#[test]
fn build_01_rejects_non_compile_commands() {
    // A link command has no -c.
    let link = raw(ROOT, "a.c", &[GCC, "-o", "a.out", "a.c"]);
    assert!(matches!(
        canon(&[link]).unwrap_err(),
        CcError::NotCompileOnly
    ));
    let empty = raw(ROOT, "a.c", &[]);
    assert!(matches!(
        canon(&[empty]).unwrap_err(),
        CcError::NotCompileOnly
    ));
    // Neither arguments nor command.
    let mut neither = raw(ROOT, "a.c", &[]);
    neither.arguments = None;
    assert!(matches!(canon(&[neither]).unwrap_err(), CcError::NoCommand));
    // An unbalanced quote cannot be split.
    let mut broken = raw(ROOT, "a.c", &[]);
    broken.arguments = None;
    broken.command = Some(format!("{GCC} -c \"unterminated a.c"));
    assert!(matches!(
        canon(&[broken]).unwrap_err(),
        CcError::Unsplittable
    ));
    // A trailing -o has no path.
    let dangling = raw(ROOT, "a.c", &[GCC, "-c", "a.c", "-o"]);
    assert!(matches!(
        canon(&[dangling]).unwrap_err(),
        CcError::DanglingOutput
    ));
    // Not a JSON array.
    assert!(matches!(parse_raw(b"{}").unwrap_err(), CcError::Json(_)));
}

#[test]
fn build_01_rejects_paths_outside_run_root() {
    let climbs = raw(ROOT, "a.c", &[GCC, "-c", "-o", "../../escape.o", "a.c"]);
    let absolute = raw(ROOT, "a.c", &[GCC, "-c", "-o", "/tmp/x.o", "a.c"]);
    let directory = raw("/etc", "a.c", &[GCC, "-c", "-o", "a.o", "a.c"]);
    let sibling = raw(
        "/work/run/fixture/default-other",
        "a.c",
        &[GCC, "-c", "a.c"],
    );
    for entry in [climbs, absolute, directory, sibling] {
        let error = canon(&[entry]).unwrap_err();
        assert!(matches!(error, CcError::OutsideRunRoot { .. }), "{error:?}");
    }
}

#[test]
fn build_01_empty_capture_is_a_failure() {
    // Nothing exported.
    assert!(matches!(canon(&[]).unwrap_err(), CcError::Empty));
    // Entries exported but no object built (Bear with loopback down).
    let entries = fixture("cmake-raw.json");
    let result = canonicalise(&entries, &policy(), &exists_in(&[]));
    assert!(matches!(result.unwrap_err(), CcError::Empty));
}

#[test]
fn build_01_duplicate_output_is_rejected() {
    let first = raw(ROOT, "a.c", &[GCC, "-c", "-o", "out/x.o", "a.c"]);
    let second = raw(ROOT, "b.c", &[GCC, "-c", "-o", "out/x.o", "b.c"]);
    let error = canon(&[first, second]).unwrap_err();
    assert!(
        matches!(&error, CcError::DuplicateOutput { output } if output == &format!("{ROOT}/out/x.o")),
        "{error:?}"
    );
}

// ---------------------------------------------------------------------------
// Plan 02-20, gap G-02-6 (threats T-02-67 and T-02-70): the gate refuses GCC
// driver options that run or load another program, and bounds every path the
// compiler writes. The same entry must be refused by `canonicalise` (capture)
// and by `CcPolicy::check_entry` (replay, before any spawn).
// ---------------------------------------------------------------------------

fn strings(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| (*part).to_owned()).collect()
}

/// `[GCC, <inserted>, -c, -o, out/a.o, a.c]` in `ROOT`.
fn with_inserted(inserted: &[&str]) -> Vec<String> {
    let mut arguments = vec![GCC.to_owned()];
    arguments.extend(strings(inserted));
    arguments.extend(strings(&["-c", "-o", "out/a.o", "a.c"]));
    arguments
}

/// `[GCC, -c, a.c, <tail>]` in `ROOT`: the tail decides the output.
fn with_tail(tail: &[&str]) -> Vec<String> {
    let mut arguments = strings(&[GCC, "-c", "a.c"]);
    arguments.extend(strings(tail));
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

/// The canonical entry the arguments would produce, built without any gate.
fn entry_of(arguments: &[String]) -> CcEntry {
    let mut entry = CcEntry {
        directory: ROOT.to_owned(),
        file: "a.c".to_owned(),
        arguments: arguments.to_vec(),
        output: String::new(),
    };
    entry.output = canonical_output(&entry).unwrap();
    entry
}

/// Both gates on one argv: the error of `canonicalise` and of `check_entry`.
fn both_errors(arguments: &[String]) -> (CcError, CcError) {
    let at_capture = canon(&[raw_of(arguments)]).unwrap_err();
    let at_replay = policy().check_entry(&entry_of(arguments)).unwrap_err();
    (at_capture, at_replay)
}

fn assert_helper(arguments: &[String], expected: &str, label: &str) {
    let (capture, replay) = both_errors(arguments);
    for error in [capture, replay] {
        assert!(
            matches!(&error, CcError::HelperOption { flag } if flag == expected),
            "{label}: expected HelperOption {{ {expected} }}, got {error:?}"
        );
    }
}

fn assert_outside(arguments: &[String], expected: &str, label: &str) {
    let (capture, replay) = both_errors(arguments);
    for error in [capture, replay] {
        assert!(
            matches!(&error, CcError::OutsideRunRoot { path } if path == expected),
            "{label}: expected OutsideRunRoot {{ {expected} }}, got {error:?}"
        );
    }
}

fn assert_redirect(arguments: &[String], expected: &str, label: &str) {
    let (capture, replay) = both_errors(arguments);
    for error in [capture, replay] {
        assert!(
            matches!(&error, CcError::WriteRedirect { flag } if flag == expected),
            "{label}: expected WriteRedirect {{ {expected} }}, got {error:?}"
        );
    }
}

#[test]
fn build_01_rejects_helper_executing_driver_options() {
    // The set cannot shrink silently.
    assert_eq!(
        HELPER_OPTION_PREFIXES,
        [
            "-wrapper",
            "-B",
            "--prefix",
            "-specs",
            "--specs",
            "-fplugin",
            "-iplugindir",
            "-fcompare-debug=",
            "@",
        ]
    );
    // (inserted arguments, the argument the error must name)
    let cases: [(&[&str], &str); 17] = [
        (&["-wrapper", "/tmp/w.sh"], "-wrapper"),
        (&["-B/tmp/b/"], "-B/tmp/b/"),
        (&["-B", "/tmp/b/"], "-B"),
        (&["--prefix=/tmp/b/"], "--prefix=/tmp/b/"),
        (&["--prefix", "/tmp/b/"], "--prefix"),
        (&["-specs=x.specs"], "-specs=x.specs"),
        (&["-specs", "x.specs"], "-specs"),
        (&["--specs=nano.specs"], "--specs=nano.specs"),
        (&["--specs", "x.specs"], "--specs"),
        (&["-fplugin=/tmp/p.so"], "-fplugin=/tmp/p.so"),
        (&["-fplugin-arg-p-key=v"], "-fplugin-arg-p-key=v"),
        (&["-iplugindir=/tmp/p"], "-iplugindir=/tmp/p"),
        (&["-fcompare-debug=-B/tmp/b/"], "-fcompare-debug=-B/tmp/b/"),
        (
            &["@/work/run/fixture/default/opts.rsp"],
            "@/work/run/fixture/default/opts.rsp",
        ),
        (&["-Wp,-fplugin=/tmp/p.so"], "-Wp,-fplugin=/tmp/p.so"),
        (&["-Wp,-MMD,dep.d,@opts.rsp"], "-Wp,-MMD,dep.d,@opts.rsp"),
        (
            &["-Xpreprocessor", "-fplugin=/tmp/p.so"],
            "-fplugin=/tmp/p.so",
        ),
    ];
    for (inserted, expected) in cases {
        let arguments = with_inserted(inserted);
        assert_eq!(
            find_helper_option(&arguments).map(String::as_str),
            Some(expected),
            "{inserted:?}"
        );
        assert_helper(&arguments, expected, &format!("{inserted:?}"));
    }
    // A CMake `command` string takes the same road.
    let mut cmake = raw(ROOT, "a.c", &[]);
    cmake.arguments = None;
    cmake.command = Some(format!("{GCC} -wrapper /tmp/w.sh -c a.c"));
    let error = canon(&[cmake]).unwrap_err();
    assert!(
        matches!(&error, CcError::HelperOption { flag } if flag == "-wrapper"),
        "{error:?}"
    );
}

#[test]
fn build_01_driver_option_gate_keeps_ordinary_flags() {
    let mut inserted: Vec<&str> = BASELINE_CFLAGS.to_vec();
    inserted.extend([
        "--sysroot=/usr/lib/arm-none-eabi",
        "-isysroot",
        "/x",
        "-Wa,-mthumb",
        "-Xassembler",
        "-mthumb",
        "-fcompare-debug",
        "-fPIC",
        "-fstack-protector-strong",
        "-fvisibility=hidden",
        "-include",
        "x.h",
        "-iquote",
        "inc",
        "-DAT=@",
        // The CMake depfile shape.
        "-MD",
        "-MT",
        "CMakeFiles/x.dir/a.c.o",
        "-MF",
        "CMakeFiles/x.dir/a.c.o.d",
        "-MMD",
        "-MQ",
        "/anything",
        "-MTtgt",
        "--write-dependencies",
        "-dumpdir",
        "out/",
        "-dumpbase",
        "a.c",
        "-dumpbase-ext",
        ".c",
        "-save-temps=obj",
        "-fstack-usage",
        "-fdump-tree-all",
        "-fopt-info-vec=stderr",
        "-aux-info=out/aux.txt",
        "--output=out/a.o",
    ]);
    let mut arguments = vec![GCC.to_owned()];
    arguments.extend(strings(&inserted));
    arguments.extend(strings(&["-c", "a.c"]));
    assert_eq!(find_helper_option(&arguments), None);
    let result = canon(&[raw_of(&arguments)]).unwrap();
    assert_eq!(result.entries[0].output, format!("{ROOT}/out/a.o"));
    policy().check_entry(&result.entries[0]).unwrap();
    policy().check_entry(&entry_of(&arguments)).unwrap();
    check_write_paths(ROOT, &arguments, &policy().run_root).unwrap();

    // Every entry of the two committed CI compile commands passes the whole gate.
    for (name, id) in [
        ("qcbor-default.compile_commands.json", "qcbor"),
        ("corejson-default.compile_commands.json", "corejson"),
    ] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/ci")
            .join(name);
        let entries: Vec<CcEntry> = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert!(!entries.is_empty(), "{name}");
        let ci_policy = CcPolicy {
            allowed_compilers: vec![GCC.to_owned()],
            run_root: PathBuf::from(format!("/work/run/{id}/default")),
        };
        for entry in &entries {
            ci_policy.check_entry(entry).unwrap_or_else(|error| {
                panic!("{name}: {} is refused: {error:?}", entry.file);
            });
            assert_eq!(find_helper_option(&entry.arguments), None, "{name}");
            assert_eq!(canonical_output(entry).unwrap(), entry.output, "{name}");
        }
    }
}

#[test]
fn build_01_handles_write_path_options() {
    // Output resolution: `--output` is `-o`, and the last output wins.
    let output_of = |arguments: &[String]| canonical_output(&entry_of(arguments)).unwrap();
    assert_eq!(
        output_of(&strings(&[GCC, "-c", "a.c", "-o", "a.o", "--output=b.o"])),
        format!("{ROOT}/b.o")
    );
    assert_eq!(
        output_of(&strings(&[GCC, "-c", "a.c", "--output", "out/c.o"])),
        format!("{ROOT}/out/c.o")
    );
    assert_eq!(
        output_of(&strings(&[GCC, "-c", "a.c", "--output=b.o", "-o", "c.o"])),
        format!("{ROOT}/c.o")
    );
    assert_eq!(
        output_of(&strings(&[GCC, "-c", "a.c", "-o/abs/x.o"])),
        "/abs/x.o"
    );

    // A write path that leaves the run root: the lexically resolved path.
    let outside: [(Vec<String>, &str); 18] = [
        (with_tail(&["--output=/out/x"]), "/out/x"),
        (with_tail(&["--output", "/out/x"]), "/out/x"),
        (with_tail(&["-o/out/x.o"]), "/out/x.o"),
        (with_inserted(&["-MF", "/out/x.d"]), "/out/x.d"),
        (with_inserted(&["-MF/out/x.d"]), "/out/x.d"),
        (with_inserted(&["-MF", "../../../../out/x.d"]), "/out/x.d"),
        (with_inserted(&["-dumpdir", "/out/dd/"]), "/out/dd"),
        (with_inserted(&["--dumpdir", "/out/dd/"]), "/out/dd"),
        (with_inserted(&["-dumpbase", "/out/b.c"]), "/out/b.c"),
        (with_inserted(&["--dumpbase", "/out/b.c"]), "/out/b.c"),
        (with_inserted(&["-aux-info", "/out/a.txt"]), "/out/a.txt"),
        (with_inserted(&["-aux-info=/out/a.txt"]), "/out/a.txt"),
        (with_inserted(&["-fdump-tree-all=/out/d"]), "/out/d"),
        (with_inserted(&["-fopt-info-vec=/out/o"]), "/out/o"),
        (with_inserted(&["-fdump-final-insns=/out/f"]), "/out/f"),
        (with_inserted(&["-fdeps-file=/out/d.json"]), "/out/d.json"),
        // The decoy: a name operand that looks like -o must not hide the real one.
        (
            with_tail(&["-o", "/out/x.o", "-MT", "-oinside.o"]),
            "/out/x.o",
        ),
        // Every occurrence is checked, not only the last.
        (
            with_inserted(&["-MF", "/out/first.d", "-MF", "ok.d"]),
            "/out/first.d",
        ),
    ];
    for (arguments, expected) in &outside {
        assert_outside(arguments, expected, &format!("{arguments:?}"));
    }

    // Write options that cannot be bounded are refused outright.
    let redirects: [(Vec<String>, &str); 13] = [
        (with_inserted(&["-Wp,-MD,/out/wp.d"]), "-Wp,-MD,/out/wp.d"),
        (with_inserted(&["-Wp,-MMD,dep.d"]), "-Wp,-MMD,dep.d"),
        (with_inserted(&["-Wp,-MF", "-Wp,/out/split.d"]), "-Wp,-MF"),
        (
            with_inserted(&["-Xpreprocessor", "-MF", "-Xpreprocessor", "/out/xp.d"]),
            "-MF",
        ),
        (
            with_inserted(&["-Wa,-alh=/out/l.txt"]),
            "-Wa,-alh=/out/l.txt",
        ),
        (with_inserted(&["-Wa,--MD,/out/as.d"]), "-Wa,--MD,/out/as.d"),
        (with_inserted(&["-Wa,@/tmp/as.rsp"]), "-Wa,@/tmp/as.rsp"),
        (
            with_inserted(&["-Xassembler", "-o", "-Xassembler", "/out/a.o"]),
            "-o",
        ),
        (
            with_inserted(&["--for-assembler=-alh=/out/l"]),
            "--for-assembler=-alh=/out/l",
        ),
        (
            with_inserted(&["--for-assembler", "-alh=/out/l"]),
            "-alh=/out/l",
        ),
        (with_inserted(&["-dumpdir=/out/dd/"]), "-dumpdir=/out/dd/"),
        (with_tail(&["-MF"]), "-MF"),
        (with_tail(&["-dumpbase"]), "-dumpbase"),
    ];
    for (arguments, expected) in &redirects {
        assert_redirect(arguments, expected, &format!("{arguments:?}"));
    }
}

/// Every `<id>/<cfg>/compile_commands.json` under `captures`.
fn local_capture_files(captures: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for id in std::fs::read_dir(captures).unwrap() {
        let id = id.unwrap().path();
        if !id.is_dir() {
            continue;
        }
        for cfg in std::fs::read_dir(&id).unwrap() {
            let file = cfg.unwrap().path().join("compile_commands.json");
            if file.is_file() {
                found.push(file);
            }
        }
    }
    found.sort();
    found
}

/// Explicit run only: `MT_LOCAL_CAPTURES=<dir> cargo test ... -- --ignored`.
/// It fails, never passes vacuously, when the variable is unset or the
/// directory holds no capture.
#[test]
#[ignore = "reads the captures of an earlier local run; set MT_LOCAL_CAPTURES"]
fn build_01_local_captures_pass_the_compile_command_gate() {
    let captures = std::env::var("MT_LOCAL_CAPTURES")
        .expect("MT_LOCAL_CAPTURES must name the directory of the local captures");
    let captures = PathBuf::from(captures);
    let files = local_capture_files(&captures);
    assert!(
        !files.is_empty(),
        "no compile_commands.json under {}",
        captures.display()
    );
    let (mut raw_entries, mut canonical_entries, mut refused) = (0_usize, 0_usize, 0_usize);
    for file in &files {
        let cfg_dir = file.parent().unwrap();
        let id = cfg_dir.parent().unwrap().file_name().unwrap();
        let cfg = cfg_dir.file_name().unwrap();
        let run_root = captures.parent().unwrap().join("run").join(id).join(cfg);
        let entries: Vec<CcEntry> = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
        let compilers: Vec<String> = entries
            .iter()
            .map(|entry| entry.arguments[0].clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let local_policy = CcPolicy {
            allowed_compilers: compilers,
            run_root,
        };
        for entry in &entries {
            canonical_entries += 1;
            if let Err(error) = local_policy.check_entry(entry) {
                refused += 1;
                eprintln!("REFUSED {}: {} {error:?}", file.display(), entry.file);
            }
            if canonical_output(entry).unwrap() != entry.output {
                refused += 1;
                eprintln!("OUTPUT {}: {}", file.display(), entry.file);
            }
        }
        let raw_path = cfg_dir.join("compile_commands.raw.json");
        let raw = parse_raw(&std::fs::read(&raw_path).unwrap()).unwrap();
        raw_entries += raw.len();
        // Every per-entry gate runs before the duplicate-output check, so a
        // DuplicateOutput result means that no entry was refused by a gate.
        match canonicalise(&raw, &local_policy, &all_exist) {
            Ok(_) | Err(CcError::DuplicateOutput { .. }) => {}
            Err(error) => {
                refused += 1;
                eprintln!("RAW REFUSED {}: {error:?}", raw_path.display());
            }
        }
    }
    println!(
        "files={} raw_entries={raw_entries} canonical_entries={canonical_entries} refused={refused}",
        files.len()
    );
    assert_eq!(refused, 0);
}
