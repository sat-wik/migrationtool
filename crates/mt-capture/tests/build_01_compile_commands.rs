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
    ALLOWED_OPTIONS, Canonical, CcEntry, CcError, CcPolicy, CommandWalk, Operand, OperandKind,
    RawEntry, SHIPPED_SPECS, canonical_output, canonicalise, check_write_paths, parse_raw, to_json,
    walk_arguments,
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
// Plans 02-20 and 02-21 (gaps G-02-6 and G-02-8; threats T-02-67, T-02-70,
// T-02-72, T-02-73): the gate is an allow-list walk. Only the spellings of
// `ALLOWED_OPTIONS` pass, one operand-aware walk decides compile-only, the
// output and the write paths, and the same entry must be refused by
// `canonicalise` (capture) and by `CcPolicy::check_entry` (replay, before any
// spawn).
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
/// When the walk refuses the arguments, the output is the one the record
/// stage would have written for `-o out/a.o`, so that replay's own gate is
/// what is tested.
fn entry_of(arguments: &[String]) -> CcEntry {
    let mut entry = CcEntry {
        directory: ROOT.to_owned(),
        file: "a.c".to_owned(),
        arguments: arguments.to_vec(),
        output: String::new(),
    };
    entry.output = canonical_output(&entry).unwrap_or_else(|_| format!("{ROOT}/out/a.o"));
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

fn assert_not_allowed(arguments: &[String], expected: &str, label: &str) {
    let (capture, replay) = both_errors(arguments);
    for error in [capture, replay] {
        assert!(
            matches!(&error, CcError::OptionNotAllowed { flag } if flag == expected),
            "{label}: expected OptionNotAllowed {{ {expected} }}, got {error:?}"
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

fn assert_not_compile_only(arguments: &[String], label: &str) {
    let (capture, replay) = both_errors(arguments);
    for error in [capture, replay] {
        assert!(
            matches!(&error, CcError::NotCompileOnly),
            "{label}: expected NotCompileOnly, got {error:?}"
        );
    }
}

#[test]
fn build_01_allow_list_is_the_measured_set() {
    // Measured from the 30 retained pairs' recorded compile commands, plus the
    // founder's depfile flags and `-specs=`. Either list changes only here.
    let spellings: Vec<&str> = ALLOWED_OPTIONS
        .iter()
        .map(|option| option.spelling)
        .collect();
    assert_eq!(
        spellings,
        [
            "-c",
            "-g",
            "-O3",
            "-Os",
            "-fPIC",
            "-fstack-protector-strong",
            "-fvisibility=hidden",
            "-mcpu=cortex-m4",
            "-mfloat-abi=hard",
            "-mfpu=fpv4-sp-d16",
            "-mthumb",
            "-pedantic",
            "-std=c89",
            "-std=c99",
            "-Wall",
            "-Wc++-compat",
            "-Wcast-align",
            "-Wcast-qual",
            "-Wconversion",
            "-Wdouble-promotion",
            "-Werror",
            "-Werror=format-security",
            "-Werror=implicit-function-declaration",
            "-Werror=incompatible-pointer-types",
            "-Werror=int-conversion",
            "-Wextra",
            "-Wformat-overflow",
            "-Wformat=2",
            "-Winit-self",
            "-Wmissing-prototypes",
            // CI-only: cjson adds it when the compiler accepts it, and GCC 14
            // does (testbed run 39453293771, cjson/default and cjson/no-locale).
            "-Wmissing-variable-declarations",
            "-Wnested-externs",
            "-Wno-format-nonliteral",
            "-Wparentheses",
            "-Wpointer-arith",
            "-Wredundant-decls",
            "-Wshadow",
            "-Wsign-compare",
            "-Wstrict-overflow=2",
            "-Wstrict-prototypes",
            "-Wsuggest-attribute=format",
            "-Wswitch-default",
            "-Wswitch-enum",
            "-Wundef",
            "-Wunused-macros",
            "-Wwrite-strings",
            "-MD",
            "-MMD",
            "-MP",
            "-o",
            "-I",
            "-D",
            "-MF",
            "-MT",
            "-MQ",
            "-specs=",
        ]
    );
    assert_eq!(SHIPPED_SPECS.to_vec(), Vec::<&str>::new());
}

#[test]
fn build_01_allow_list_refuses_everything_else() {
    // CR-01 and CR-02, each as OptionNotAllowed with the whole argument.
    for flag in [
        "--plugin=/work/run/fixture/default/evil.so",
        "--compare-debug=-wrapper,/work/run/fixture/default/w.sh",
        "--opt-info-all=/out/leak.txt",
        "--dump-tree-all=/out/leak",
        "--dump-final-insns=/out/f",
        "--profile-use=/out/p",
        "--profile-dir=/out/pd",
        "-Wa,-MD,/out/e.d",
        "-Wa,-MD=/out/e.d",
        "-Wa,--M,/out/e.d",
        "-Wa,-M,/out/e.d",
        "--for-assembler=-MD,/out/e.d",
    ] {
        assert_not_allowed(&with_inserted(&[flag]), flag, flag);
    }
    assert_not_allowed(
        &with_inserted(&["-Xassembler", "-MD", "-Xassembler", "/out/e.d"]),
        "-Xassembler",
        "-Xassembler -MD",
    );
    // CR-03: the review's vector stops at the first unlisted option; the
    // minimal one is not a compile-only command.
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
    assert_not_allowed(&review, "-nostdlib", "CR-03");
    assert_not_compile_only(
        &strings(&[GCC, "-MD", "-MT", "-c", "a.c", "-o", "out/a.o"]),
        "minimal CR-03",
    );
    // What 02-20's keeps-ordinary-flags test accepted and the allow-list now
    // refuses: (inserted arguments, the argument the error must name).
    let refused: [(&[&str], &str); 17] = [
        (
            &["--sysroot=/usr/lib/arm-none-eabi"],
            "--sysroot=/usr/lib/arm-none-eabi",
        ),
        (&["-isysroot", "/x"], "-isysroot"),
        (&["-Wa,-mthumb"], "-Wa,-mthumb"),
        (&["-Xassembler", "-mthumb"], "-Xassembler"),
        (&["-fcompare-debug"], "-fcompare-debug"),
        (&["-include", "x.h"], "-include"),
        (&["-iquote", "inc"], "-iquote"),
        (&["--write-dependencies"], "--write-dependencies"),
        (&["-dumpdir", "out/"], "-dumpdir"),
        (&["-dumpbase", "a.c"], "-dumpbase"),
        (&["-dumpbase-ext", ".c"], "-dumpbase-ext"),
        (&["-save-temps=obj"], "-save-temps=obj"),
        (&["-fstack-usage"], "-fstack-usage"),
        (&["-fdump-tree-all"], "-fdump-tree-all"),
        (&["-fopt-info-vec=stderr"], "-fopt-info-vec=stderr"),
        (&["-aux-info=out/aux.txt"], "-aux-info=out/aux.txt"),
        (&["--output=out/a.o"], "--output=out/a.o"),
    ];
    for (inserted, expected) in refused {
        assert_not_allowed(&with_inserted(inserted), expected, &format!("{inserted:?}"));
    }
    // Never measured: other modes, carriers, unmeasured values and bare dashes.
    let unmeasured: [(&[&str], &str); 18] = [
        (&["-x", "c"], "-x"),
        (&["-isystem", "/x"], "-isystem"),
        (&["-E"], "-E"),
        (&["-S"], "-S"),
        (&["-M"], "-M"),
        (&["-MM"], "-MM"),
        (&["-MG"], "-MG"),
        (&["-D"], "-D"),
        (&["-mfloat-abi=soft"], "-mfloat-abi=soft"),
        (&["-O2"], "-O2"),
        (&["-Wl,-Map=x"], "-Wl,-Map=x"),
        (&["-Xlinker", "-Map"], "-Xlinker"),
        (&["--for-linker=x"], "--for-linker=x"),
        (&["-Wp,-DX"], "-Wp,-DX"),
        (&["-Xpreprocessor", "-DX"], "-Xpreprocessor"),
        (&["-"], "-"),
        (&["--"], "--"),
        (&["-wrapper", "/bin/echo"], "-wrapper"),
    ];
    for (inserted, expected) in unmeasured {
        assert_not_allowed(&with_inserted(inserted), expected, &format!("{inserted:?}"));
    }
    // A trailing -I has no directory.
    assert_not_allowed(&with_tail(&["-o", "out/a.o", "-I"]), "-I", "trailing -I");
}

#[test]
fn build_01_one_walk_serves_compile_only_output_and_write_paths() {
    // IN-14: one table, one walk. Every operand-taking spelling consumes its
    // separate operand exactly once, and its joined form consumes nothing.
    let mut checked = 0;
    for option in &ALLOWED_OPTIONS {
        let spelling = option.spelling;
        match option.operand {
            Operand::None => {}
            Operand::JoinedOrSeparate(kind) => {
                checked += 1;
                // The separate operand is `-c`, so nothing compiles.
                let separate = strings(&[GCC, spelling, "-c", "a.c"]);
                let walk: CommandWalk = walk_arguments(&separate).unwrap();
                assert!(!walk.compile_only, "{spelling}: -c read as an option");
                assert_not_compile_only(&separate, spelling);
                // The joined form leaves the next argument alone.
                let joined = strings(&[GCC, &format!("{spelling}out/j"), "-c", "a.c"]);
                let walk = walk_arguments(&joined).unwrap();
                assert!(walk.compile_only, "{spelling}: the joined form consumed -c");
                // The path operands are collected as written, in order.
                let tail = strings(&[GCC, "-c", "a.c", spelling, "out/s"]);
                let walk = walk_arguments(&tail).unwrap();
                match kind {
                    OperandKind::OutputPath => {
                        assert_eq!(walk.outputs, ["out/s"], "{spelling}");
                        assert!(walk.dep_files.is_empty(), "{spelling}");
                    }
                    OperandKind::DepfilePath => {
                        assert_eq!(walk.dep_files, ["out/s"], "{spelling}");
                        assert!(walk.outputs.is_empty(), "{spelling}");
                    }
                    _ => {
                        assert!(walk.outputs.is_empty(), "{spelling}");
                        assert!(walk.dep_files.is_empty(), "{spelling}");
                    }
                }
                // canonical_output, check_write_paths and check_command agree.
                let expected = if kind == OperandKind::OutputPath {
                    format!("{ROOT}/out/s")
                } else {
                    format!("{ROOT}/a.o")
                };
                assert_eq!(canonical_output(&entry_of(&tail)).unwrap(), expected);
                check_write_paths(ROOT, &tail, &policy().run_root).unwrap();
                assert_eq!(
                    policy().check_command(ROOT, "a.c", &tail).unwrap(),
                    expected,
                    "{spelling}"
                );
                policy().check_entry(&entry_of(&tail)).unwrap();
                // A path operand outside the run root is refused by all three.
                if matches!(kind, OperandKind::OutputPath | OperandKind::DepfilePath) {
                    let outside = strings(&[GCC, "-c", "a.c", spelling, "/out/s"]);
                    let walk = walk_arguments(&outside).unwrap();
                    assert!(walk.compile_only, "{spelling}");
                    for error in [
                        check_write_paths(ROOT, &outside, &policy().run_root).unwrap_err(),
                        policy().check_command(ROOT, "a.c", &outside).unwrap_err(),
                    ] {
                        assert!(
                            matches!(&error, CcError::OutsideRunRoot { path } if path == "/out/s"),
                            "{spelling}: {error:?}"
                        );
                    }
                    assert_outside(&outside, "/out/s", spelling);
                }
            }
            Operand::JoinedOnly(kind) => {
                checked += 1;
                let value = match kind {
                    OperandKind::SpecName => match SHIPPED_SPECS.first() {
                        Some(name) => *name,
                        None => continue,
                    },
                    _ => "X",
                };
                let joined = strings(&[GCC, &format!("{spelling}{value}"), "-c", "a.c"]);
                let walk = walk_arguments(&joined).unwrap();
                assert!(walk.compile_only, "{spelling}: the joined form consumed -c");
                assert!(walk.outputs.is_empty() && walk.dep_files.is_empty());
            }
        }
    }
    assert_eq!(checked, 7, "-o, -I, -D, -MF, -MT, -MQ and -specs=");
}

#[test]
fn build_01_rejects_helper_executing_driver_options() {
    // (inserted arguments, the argument the error must name). A spec file and
    // a response file make the driver read commands or options from a file
    // (HelperOption); everything else is simply not on the allow-list.
    let helper: [(&[&str], &str); 3] = [
        (&["-specs=x.specs"], "-specs=x.specs"),
        (&["-specs", "x.specs"], "-specs"),
        (
            &["@/work/run/fixture/default/opts.rsp"],
            "@/work/run/fixture/default/opts.rsp",
        ),
    ];
    for (inserted, expected) in helper {
        assert_helper(&with_inserted(inserted), expected, &format!("{inserted:?}"));
    }
    let not_allowed: [(&[&str], &str); 14] = [
        (&["-wrapper", "/tmp/w.sh"], "-wrapper"),
        (&["-B/tmp/b/"], "-B/tmp/b/"),
        (&["-B", "/tmp/b/"], "-B"),
        (&["--prefix=/tmp/b/"], "--prefix=/tmp/b/"),
        (&["--prefix", "/tmp/b/"], "--prefix"),
        (&["--specs=nano.specs"], "--specs=nano.specs"),
        (&["--specs", "x.specs"], "--specs"),
        (&["-fplugin=/tmp/p.so"], "-fplugin=/tmp/p.so"),
        (&["-fplugin-arg-p-key=v"], "-fplugin-arg-p-key=v"),
        (&["-iplugindir=/tmp/p"], "-iplugindir=/tmp/p"),
        (&["-fcompare-debug=-B/tmp/b/"], "-fcompare-debug=-B/tmp/b/"),
        (&["-Wp,-fplugin=/tmp/p.so"], "-Wp,-fplugin=/tmp/p.so"),
        (&["-Wp,-MMD,dep.d,@opts.rsp"], "-Wp,-MMD,dep.d,@opts.rsp"),
        (&["-Xpreprocessor", "-fplugin=/tmp/p.so"], "-Xpreprocessor"),
    ];
    for (inserted, expected) in not_allowed {
        assert_not_allowed(&with_inserted(inserted), expected, &format!("{inserted:?}"));
    }
    // A CMake `command` string takes the same road.
    let mut cmake = raw(ROOT, "a.c", &[]);
    cmake.arguments = None;
    cmake.command = Some(format!("{GCC} -wrapper /tmp/w.sh -c a.c"));
    let error = canon(&[cmake]).unwrap_err();
    assert!(
        matches!(&error, CcError::OptionNotAllowed { flag } if flag == "-wrapper"),
        "{error:?}"
    );
}

#[test]
fn build_01_driver_option_gate_keeps_ordinary_flags() {
    // Every exact measured spelling, then the operand forms the bed uses.
    let mut inserted: Vec<&str> = ALLOWED_OPTIONS
        .iter()
        .filter(|option| matches!(option.operand, Operand::None))
        .map(|option| option.spelling)
        .collect();
    inserted.extend(BASELINE_CFLAGS);
    inserted.extend([
        "-DAT=@",
        "-DNAME=1",
        "-Iinc",
        "-I",
        "inc",
        // The CMake depfile shape.
        "-MD",
        "-MT",
        "CMakeFiles/x.dir/a.c.o",
        "-MF",
        "CMakeFiles/x.dir/a.c.o.d",
        "-MMD",
        "-MP",
        "-MQ",
        "/anything",
        "-MTtgt",
        "-MFout/a.d",
        "-o",
        "out/a.o",
    ]);
    let mut arguments = vec![GCC.to_owned()];
    arguments.extend(strings(&inserted));
    arguments.extend(strings(&["-c", "a.c"]));
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
            assert_eq!(canonical_output(entry).unwrap(), entry.output, "{name}");
        }
    }
}

#[test]
fn build_01_handles_write_path_options() {
    // Output resolution: the last output wins, joined or separate.
    let output_of = |arguments: &[String]| canonical_output(&entry_of(arguments)).unwrap();
    assert_eq!(
        output_of(&strings(&[GCC, "-c", "a.c", "-o", "a.o", "-o", "b.o"])),
        format!("{ROOT}/b.o")
    );
    assert_eq!(
        output_of(&strings(&[GCC, "-c", "a.c", "-o/abs/x.o"])),
        "/abs/x.o"
    );

    // A write path that leaves the run root: the lexically resolved path.
    let outside: [(Vec<String>, &str); 7] = [
        (with_tail(&["-o/out/x.o"]), "/out/x.o"),
        (with_tail(&["-o", "/out/x.o"]), "/out/x.o"),
        (with_inserted(&["-MF", "/out/x.d"]), "/out/x.d"),
        (with_inserted(&["-MF/out/x.d"]), "/out/x.d"),
        (with_inserted(&["-MF", "../../../../out/x.d"]), "/out/x.d"),
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

    // A depfile option without an operand cannot be bounded.
    for flag in ["-MF", "-MT", "-MQ"] {
        assert_redirect(&with_tail(&["-o", "out/a.o", flag]), flag, flag);
    }

    // Every other write option is simply not on the allow-list.
    let not_allowed: [(Vec<String>, &str); 8] = [
        (with_tail(&["--output=/out/x"]), "--output=/out/x"),
        (with_tail(&["--output", "/out/x"]), "--output"),
        (with_inserted(&["-dumpdir", "/out/dd/"]), "-dumpdir"),
        (with_inserted(&["-aux-info", "/out/a.txt"]), "-aux-info"),
        (
            with_inserted(&["-fdump-tree-all=/out/d"]),
            "-fdump-tree-all=/out/d",
        ),
        (
            with_inserted(&["-fdeps-file=/out/d.json"]),
            "-fdeps-file=/out/d.json",
        ),
        (with_inserted(&["-Wp,-MD,/out/wp.d"]), "-Wp,-MD,/out/wp.d"),
        (
            with_inserted(&["-Xpreprocessor", "-MF", "-Xpreprocessor", "/out/xp.d"]),
            "-Xpreprocessor",
        ),
    ];
    for (arguments, expected) in &not_allowed {
        assert_not_allowed(arguments, expected, &format!("{arguments:?}"));
    }
}

/// Every `<id>/<cfg>` directory under `captures`.
fn local_capture_dirs(captures: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for id in std::fs::read_dir(captures).unwrap() {
        let id = id.unwrap().path();
        if !id.is_dir() {
            continue;
        }
        for cfg in std::fs::read_dir(&id).unwrap() {
            let cfg = cfg.unwrap().path();
            if cfg.is_dir() {
                found.push(cfg);
            }
        }
    }
    found.sort();
    found
}

/// A policy that allows exactly the compilers the file names, under `run_root`.
fn policy_for(compilers: impl Iterator<Item = String>, run_root: PathBuf) -> CcPolicy {
    CcPolicy {
        allowed_compilers: compilers.collect::<BTreeSet<_>>().into_iter().collect(),
        run_root,
    }
}

/// Every per-entry gate runs before the duplicate-output check, so a
/// DuplicateOutput result means that no entry was refused by a gate.
fn raw_passes(raw: &[RawEntry], policy: &CcPolicy, label: &Path) -> bool {
    match canonicalise(raw, policy, &all_exist) {
        Ok(_) | Err(CcError::DuplicateOutput { .. }) => true,
        Err(error) => {
            eprintln!("RAW REFUSED {}: {error:?}", label.display());
            false
        }
    }
}

/// The argv[0] of a raw entry, from its arguments or its command string.
fn raw_compiler(raw: &RawEntry) -> String {
    match (&raw.arguments, &raw.command) {
        (Some(arguments), _) => arguments[0].clone(),
        (None, Some(command)) => shlex::split(command).unwrap()[0].clone(),
        (None, None) => String::new(),
    }
}

/// Explicit run only: `MT_LOCAL_CAPTURES=<dir> cargo test ... -- --ignored`.
/// It fails, never passes vacuously, when the variable is unset or the
/// directory holds no capture.
#[test]
#[ignore = "reads the captures of an earlier local run; set MT_LOCAL_CAPTURES"]
fn build_01_local_captures_pass_the_compile_command_gate() {
    let captures = std::env::var("MT_LOCAL_CAPTURES")
        .expect("MT_LOCAL_CAPTURES must name the directory of the local captures");
    assert!(!captures.is_empty(), "MT_LOCAL_CAPTURES is empty");
    let captures = PathBuf::from(captures);
    let runs = captures.parent().unwrap().join("run");
    let (mut files, mut raw_entries, mut canonical_entries, mut refused) =
        (0_usize, 0_usize, 0_usize, 0_usize);
    let (mut run_exports, mut run_export_entries) = (0_usize, 0_usize);
    for cfg_dir in local_capture_dirs(&captures) {
        let id = cfg_dir.parent().unwrap().file_name().unwrap();
        let cfg = cfg_dir.file_name().unwrap();
        let run_root = runs.join(id).join(cfg);
        let file = cfg_dir.join("compile_commands.json");
        let raw_path = cfg_dir.join("compile_commands.raw.json");
        if file.is_file() {
            files += 1;
            let entries: Vec<CcEntry> =
                serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
            let local_policy = policy_for(
                entries.iter().map(|entry| entry.arguments[0].clone()),
                run_root.clone(),
            );
            for entry in &entries {
                canonical_entries += 1;
                if let Err(error) = local_policy.check_entry(entry) {
                    refused += 1;
                    eprintln!("REFUSED {}: {} {error:?}", file.display(), entry.file);
                }
                if canonical_output(entry).ok().as_deref() != Some(entry.output.as_str()) {
                    refused += 1;
                    eprintln!("OUTPUT {}: {}", file.display(), entry.file);
                }
            }
        }
        if raw_path.is_file() {
            let raw = parse_raw(&std::fs::read(&raw_path).unwrap()).unwrap();
            raw_entries += raw.len();
            let raw_policy = policy_for(raw.iter().map(raw_compiler), run_root);
            if !raw_passes(&raw, &raw_policy, &raw_path) {
                refused += 1;
            }
        } else {
            // No raw file (tinycbor's local build failed on the host newlib):
            // the run export, every output treated as existing.
            let export = run_root.join("build/compile_commands.json");
            if export.is_file() {
                run_exports += 1;
                let raw = parse_raw(&std::fs::read(&export).unwrap()).unwrap();
                run_export_entries += raw.len();
                let export_policy = policy_for(raw.iter().map(raw_compiler), run_root);
                if !raw_passes(&raw, &export_policy, &export) {
                    refused += 1;
                }
            }
        }
    }
    assert!(
        files > 0,
        "no compile_commands.json under {}",
        captures.display()
    );
    println!(
        "files={files} raw_entries={raw_entries} canonical_entries={canonical_entries} run_exports={run_exports} run_export_entries={run_export_entries} refused={refused}"
    );
    assert_eq!(refused, 0);
}
