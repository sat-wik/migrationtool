#![allow(clippy::unwrap_used, clippy::expect_used)]
//! EQV-04: the exact-objects property (CONTEXT D-01, D-19), the deterministic
//! archive and the unresolved-symbol parse. The capture directories here are
//! built in a temporary directory from a committed Arm object fixture.

use std::path::{Path, PathBuf};

use mt_harness::globals::{GlobalKind, writable_globals};
use mt_harness::link::{archive, unresolved_symbols, verify_objects};
use mt_harness::tools::ExecTools;
use mt_toolchain::runner::RunnerConfig;
use mt_vectors::sha256_hex;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn fixture_object() -> Vec<u8> {
    std::fs::read(repo_root().join("crates/mt-capture/tests/fixtures/ci/arm-small.o")).unwrap()
}

/// A capture directory holding one object `build/x.o` and a capture.json that
/// names its sha256 with `verdict`.
fn capture_dir(verdict: &str) -> (tempfile::TempDir, PathBuf, String) {
    let dir = tempfile::tempdir().unwrap();
    let capture = dir.path().join("capture/t/default");
    let objects = capture.join("objects/build");
    std::fs::create_dir_all(&objects).unwrap();
    let bytes = fixture_object();
    std::fs::write(objects.join("x.o"), &bytes).unwrap();
    let sha = sha256_hex(&bytes);
    let json = format!(
        "{{\"schema_version\": 1, \"kind\": \"capture\", \"target\": \"t\", \"config\": \"default\", \
         \"verdict\": \"{verdict}\", \"image\": \"local\", \
         \"objects\": {{\"build/x.o\": {{\"sha256\": \"{sha}\"}}}}}}"
    );
    std::fs::write(capture.join("capture.json"), json).unwrap();
    (dir, capture, sha)
}

#[test]
fn eqv_04_verifies_untouched_objects() {
    let (_dir, capture, sha) = capture_dir("captured");
    let verified = verify_objects(&capture).unwrap();
    assert_eq!(verified.image, "local");
    assert_eq!(
        verified.objects.iter().collect::<Vec<_>>(),
        [(&"build/x.o".to_owned(), &sha)]
    );
    assert_eq!(verified.objects_dir, capture.join("objects"));
}

#[test]
fn eqv_04_refuses_modified_object() {
    let (_dir, capture, sha) = capture_dir("captured");
    let path = capture.join("objects/build/x.o");
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[10] ^= 0x01;
    std::fs::write(&path, &bytes).unwrap();
    let actual = sha256_hex(&bytes);
    let message = verify_objects(&capture).unwrap_err().to_string();
    assert!(message.contains("build/x.o"), "{message}");
    assert!(message.contains(&sha), "the recorded hash: {message}");
    assert!(message.contains(&actual), "the recomputed hash: {message}");
}

#[test]
fn eqv_04_refuses_a_capture_that_is_not_captured() {
    for verdict in ["build_failed", "not_proven", "capture_failed"] {
        let (_dir, capture, _) = capture_dir(verdict);
        let message = verify_objects(&capture).unwrap_err().to_string();
        assert!(
            message.contains(verdict) && message.contains("captured"),
            "{verdict}: {message}"
        );
    }
}

#[test]
fn eqv_04_refuses_a_symlinked_object() {
    let (_dir, capture, _) = capture_dir("captured");
    let path = capture.join("objects/build/x.o");
    let elsewhere = capture.join("elsewhere.o");
    std::fs::rename(&path, &elsewhere).unwrap();
    std::os::unix::fs::symlink(&elsewhere, &path).unwrap();
    let message = verify_objects(&capture).unwrap_err().to_string();
    assert!(message.contains("build/x.o"), "{message}");
    assert!(message.contains("symlink"), "{message}");
}

#[test]
fn eqv_04_refuses_an_object_path_that_leaves_the_directory() {
    let (_dir, capture, sha) = capture_dir("captured");
    let json = std::fs::read_to_string(capture.join("capture.json"))
        .unwrap()
        .replace("build/x.o", "../x.o");
    std::fs::write(capture.join("capture.json"), json).unwrap();
    let message = verify_objects(&capture).unwrap_err().to_string();
    assert!(message.contains("../x.o"), "{message} ({sha})");
}

#[test]
fn eqv_04_unresolved_symbols_are_parsed_from_the_linker() {
    let stderr = "\
/usr/lib/gcc/arm-none-eabi/13.2.1/../../../../arm-none-eabi/bin/ld: driver.o: in function `mt_main':
driver.c:(.text.mt_main+0x1a): undefined reference to `ble_sm_alg_c1'
/usr/lib/gcc/arm-none-eabi/13.2.1/../../../../arm-none-eabi/bin/ld: pair.a(x.obj): in function `bar':
x.c:(.text+0x4): undefined reference to `ble_sm_alg_s1'
x.c:(.text+0x8): undefined reference to `ble_sm_alg_c1'
x.c:(.text+0xc): undefined reference to \u{2018}quoted\u{2019}
collect2: error: ld returned 1 exit status
";
    assert_eq!(
        unresolved_symbols(stderr),
        ["ble_sm_alg_c1", "ble_sm_alg_s1", "quoted"]
    );
    assert!(unresolved_symbols("collect2: error: ld returned 1 exit status\n").is_empty());
}

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/obj")
            .join(name),
    )
    .unwrap()
}

#[test]
fn eqv_04_globals_are_found_in_data_and_bss() {
    // `arm-none-eabi-nm -S globals.o` lists counter (D, 4), table (d, 7),
    // hidden_bss (b, 4), zeroed (B, 12), ro (R, read-only data) and use (T).
    let symbols = writable_globals("globals.o", &fixture("globals.o")).unwrap();
    let rows: Vec<(&str, GlobalKind, u64, bool)> = symbols
        .iter()
        .map(|symbol| (symbol.name.as_str(), symbol.kind, symbol.size, symbol.local))
        .collect();
    assert_eq!(
        rows,
        [
            ("counter", GlobalKind::Data, 4, false),
            ("hidden_bss", GlobalKind::Bss, 4, true),
            ("table", GlobalKind::Data, 7, true),
            ("zeroed", GlobalKind::Bss, 12, false),
        ]
    );
    assert!(symbols.iter().all(|symbol| symbol.object == "globals.o"));
    // A local symbol is joined through the source file that precedes it.
    for symbol in symbols.iter().filter(|symbol| symbol.local) {
        assert_eq!(symbol.file.as_deref(), Some("globals.c"), "{}", symbol.name);
    }
    assert!(
        symbols
            .iter()
            .filter(|symbol| !symbol.local)
            .all(|symbol| symbol.file.is_none())
    );

    // The Phase 2 Arm fixture has one .bss symbol and no .data symbol.
    let small = writable_globals(
        "arm-small.o",
        &std::fs::read(repo_root().join("crates/mt-capture/tests/fixtures/ci/arm-small.o"))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(small.len(), 1);
    assert_eq!(small[0].name, "ieee754_dummy_place_holder");
    assert_eq!(small[0].kind, GlobalKind::Bss);
    assert_eq!(small[0].size, 4);

    // Bytes that are not an object are an error, never an empty list.
    assert!(writable_globals("junk.o", b"not an elf file at all").is_err());
}

fn host_tools() -> Option<ExecTools> {
    let needed = [
        "/usr/bin/qemu-arm",
        "/usr/bin/arm-none-eabi-gcc",
        "/usr/bin/arm-none-eabi-ar",
    ];
    if let Some(missing) = needed.iter().find(|path| !Path::new(path).exists()) {
        eprintln!("skipping: {missing} is not installed on this host");
        return None;
    }
    Some(ExecTools::at(
        Path::new("/usr/bin"),
        Path::new("/usr/bin/qemu-arm"),
        Path::new("/usr/bin/llvm-dwarfdump"),
    ))
}

#[test]
fn eqv_04_archive_keeps_members_with_the_same_name() {
    let Some(tools) = host_tools() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let objects = dir.path().join("objects");
    for sub in ["a", "b"] {
        std::fs::create_dir_all(objects.join(sub)).unwrap();
        std::fs::write(objects.join(sub).join("x.o"), fixture("globals.o")).unwrap();
    }
    let cfg = RunnerConfig::new("/usr/bin:/bin", "1791158400");
    let members = vec!["b/x.o".to_owned(), "a/x.o".to_owned()];
    let first = archive(&tools, &cfg, &objects, &members, &dir.path().join("one.a")).unwrap();
    let bytes = std::fs::read(&first.path).unwrap();
    // Two members, both named x.o: ar keeps the base name, not the directory.
    let named = bytes.windows(4).filter(|window| window == b"x.o/").count();
    assert_eq!(named, 2, "both members stay in the archive");
    assert_eq!(first.sha256, sha256_hex(&bytes));

    // The archive is deterministic: the same members in another order, written
    // again, give the same bytes (members sorted, `D` zeroes the headers).
    let reversed: Vec<String> = members.iter().rev().cloned().collect();
    let second = archive(&tools, &cfg, &objects, &reversed, &dir.path().join("two.a")).unwrap();
    assert_eq!(first.sha256, second.sha256);
    // An existing archive is replaced, not appended to.
    let again = archive(&tools, &cfg, &objects, &members, &first.path).unwrap();
    assert_eq!(again.sha256, first.sha256);
}
