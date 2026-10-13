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

use mt_capture::cc::{Canonical, CcError, CcPolicy, RawEntry, canonicalise, parse_raw, to_json};

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
