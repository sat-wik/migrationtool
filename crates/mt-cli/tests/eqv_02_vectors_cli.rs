#![allow(clippy::unwrap_used, clippy::expect_used)]
//! EQV-02: the built `mt vectors validate` command line, launched through the
//! runner. Only argument handling, exit codes and messages are exercised; the
//! rules are tested in `mt-vectors`.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use mt_toolchain::runner::{self, RunRequest, RunnerConfig};

const FIXTURE: &str = "crates/mt-vectors/tests/fixtures/own/reference.json";

struct Outcome {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn run_mt(args: &[&str]) -> Outcome {
    let cfg = RunnerConfig::new("/usr/bin:/bin", "1791158400");
    let mut argv: Vec<OsString> = vec![env!("CARGO_BIN_EXE_mt").into()];
    argv.extend(args.iter().map(OsString::from));
    let request = RunRequest {
        tool_name: "mt".to_owned(),
        argv,
        cwd: repo_root(),
        extra_env: BTreeMap::new(),
        tool_version: None,
    };
    let output = runner::run(&cfg, &request).unwrap();
    assert!(!output.record.exit.timed_out, "mt timed out");
    Outcome {
        code: output.record.exit.code,
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

#[test]
fn eqv_02_validate_cli_accepts_the_reference_fixture() {
    let outcome = run_mt(&["vectors", "validate", FIXTURE]);
    assert_eq!(outcome.code, Some(0), "stderr: {}", outcome.stderr);
    let row = format!("{FIXTURE} valid reference");
    assert!(
        outcome.stdout.lines().any(|l| l == row),
        "stdout: {}",
        outcome.stdout
    );
}

#[test]
fn eqv_02_validate_cli_rejects_a_merged_lib_state() {
    let text = std::fs::read_to_string(repo_root().join(FIXTURE)).unwrap();
    let merged = text.replace(
        "\"lib_state_in\": {},\n  \"lib_state_out\": {}",
        "\"lib_state\": {}",
    );
    assert_ne!(merged, text, "the edit must change the fixture");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("merged.json");
    std::fs::write(&path, merged).unwrap();

    let outcome = run_mt(&["vectors", "validate", path.to_str().unwrap()]);
    assert_eq!(outcome.code, Some(2), "stderr: {}", outcome.stderr);
    assert!(
        outcome.stderr.contains("lib_state"),
        "the refusal must name the key: {}",
        outcome.stderr
    );
    assert!(
        !outcome.stderr.contains("unrecognized"),
        "the subcommand must exist: {}",
        outcome.stderr
    );
}

#[test]
fn eqv_02_validate_cli_tractor_level() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("lib_case.json");
    std::fs::write(
        &path,
        r#"{"lib_state_in": {"x": 1}, "lib_state_out": {"returns": 2}, "has_ub": 7, "note": "free text"}"#,
    )
    .unwrap();
    let arg = path.to_str().unwrap();

    let outcome = run_mt(&["vectors", "validate", "--tractor", arg]);
    assert_eq!(outcome.code, Some(0), "stderr: {}", outcome.stderr);
    let row = format!("{arg} valid tractor");
    assert!(
        outcome.stdout.lines().any(|l| l == row),
        "stdout: {}",
        outcome.stdout
    );

    // The same file is not a reference vector.
    let strict = run_mt(&["vectors", "validate", arg]);
    assert_eq!(strict.code, Some(2), "stderr: {}", strict.stderr);
}

#[test]
fn eqv_02_validate_cli_corpus() {
    use mt_vectors::corpus::{HasUbSource, Provenance, ProvenanceBudgets, write_corpus};
    use mt_vectors::vector::Vector;

    let hash = "ab".repeat(32);
    let mut set = Vec::new();
    for rc in 0..3 {
        let text = format!(
            r#"{{"argv": ["corejson/default/JSON_Validate"], "stdin": "", "env": {{}},
              "stdout": {{"pattern": "", "is_regex": false}},
              "stderr": {{"pattern": "outcome=ok", "is_regex": false}},
              "rc": {rc}, "lib_state_in": {{}}, "lib_state_out": {{}}}}"#
        );
        let vector = Vector::from_json_bytes(text.as_bytes()).unwrap();
        let provenance = Provenance {
            seed: hash.clone(),
            scalars: BTreeMap::new(),
            objects: BTreeMap::new(),
            archive: hash.clone(),
            driver_source: hash.clone(),
            driver_elf: hash.clone(),
            image: "ghcr.io/example/toolchain@sha256:0000".to_owned(),
            executor_version: "0.1.0".to_owned(),
            contract_version: "0.1.0".to_owned(),
            policy_sha256: hash.clone(),
            budgets: ProvenanceBudgets {
                timeout_ms: 10_000,
                reserved_va: 0x1000_0000,
                heap_arena: 1 << 20,
                sentinel: 0xA5,
                slack_fill: 0x5A,
                max_input: 65_536,
            },
            outcome: "ok".to_owned(),
            has_ub_source: HasUbSource::Unchecked,
            unobserved: 0,
            discarded_output_bytes: 0,
            replay: "not_checked".to_owned(),
        };
        set.push((vector, provenance));
    }
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("corpus");
    write_corpus(&dir, &set).unwrap();
    let arg = dir.to_str().unwrap();

    let outcome = run_mt(&["vectors", "validate", arg]);
    assert_eq!(outcome.code, Some(0), "stderr: {}", outcome.stderr);
    let row = format!("{arg} valid corpus 3 vectors");
    assert!(
        outcome.stdout.lines().any(|l| l == row),
        "stdout: {}",
        outcome.stdout
    );

    std::fs::write(dir.join("stray.txt"), b"x").unwrap();
    let broken = run_mt(&["vectors", "validate", arg]);
    assert_eq!(broken.code, Some(2), "stderr: {}", broken.stderr);
}

const COMMIT: &str = "37960ee08a7c644ee66e29855eca33903f702e67";

/// Three of our own JSON files at pinned paths under a work directory's tractor
/// tree, and a pin file naming them. `mismatch` files get a hash that is not
/// theirs.
fn own_tractor_setup(mismatch: usize) -> (tempfile::TempDir, PathBuf, PathBuf) {
    use std::fmt::Write as _;

    let dir = tempfile::tempdir().unwrap();
    let work = dir.path().join("work");
    let tree = work.join("tractor").join(COMMIT).join("tree");
    let files = [
        (
            "Public-Tests/B/one/test_vectors/a.json",
            r#"{"stdout": {"pattern": "x"}}"#,
        ),
        (
            "Public-Tests/B/two/test_vectors/b.json",
            r#"{"lib_state_in": {"a": 1}}"#,
        ),
        (
            "Public-Tests/B/three/test_vectors/c.json",
            r#"{"argv": [], "rc": -11}"#,
        ),
    ];
    let mut pin = format!(
        "schema_version = 1\nrepo = \"https://example.invalid/corpus\"\ncommit = \"{COMMIT}\"\n\
         licence_status = \"unchecked; local use only until the Phase 12 licence check\"\n"
    );
    for (i, (path, content)) in files.iter().enumerate() {
        let full = tree.join(path);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(&full, content).unwrap();
        let hash = if i < mismatch {
            "00".repeat(32)
        } else {
            sha256_of(content.as_bytes())
        };
        write!(pin, "\n[[file]]\npath = \"{path}\"\nsha256 = \"{hash}\"\n").unwrap();
    }
    let pin_path = dir.path().join("tractor-pin.toml");
    std::fs::write(&pin_path, pin).unwrap();
    (dir, pin_path, work)
}

fn sha256_of(bytes: &[u8]) -> String {
    mt_vectors::sha256_hex(bytes)
}

#[test]
fn eqv_02_tractor_check_cli_needs_three() {
    let (_dir, pin, work) = own_tractor_setup(1);
    let outcome = run_mt(&[
        "vectors",
        "tractor",
        "check",
        "--pin",
        pin.to_str().unwrap(),
        "--work-dir",
        work.to_str().unwrap(),
    ]);
    assert_eq!(outcome.code, Some(1), "stderr: {}", outcome.stderr);
    assert!(
        outcome
            .stdout
            .lines()
            .any(|l| l == "loaded: 2 of 3 (at least 3 required)"),
        "stdout: {}",
        outcome.stdout
    );
}

#[test]
fn eqv_02_tractor_check_cli_loads_pinned_files() {
    let (_dir, pin, work) = own_tractor_setup(0);
    let outcome = run_mt(&[
        "vectors",
        "tractor",
        "check",
        "--pin",
        pin.to_str().unwrap(),
        "--work-dir",
        work.to_str().unwrap(),
    ]);
    assert_eq!(outcome.code, Some(0), "stderr: {}", outcome.stderr);
    assert!(
        outcome
            .stdout
            .lines()
            .any(|l| l == "loaded: 3 of 3 (at least 3 required)"),
        "stdout: {}",
        outcome.stdout
    );
    assert_eq!(outcome.stdout.matches("round_trip=json-equal").count(), 3);
}

#[test]
fn eqv_02_tractor_check_cli_rejects_a_bad_pin_and_a_missing_tree() {
    let (dir, pin, work) = own_tractor_setup(0);
    let text = std::fs::read_to_string(&pin).unwrap();
    let bad = dir.path().join("bad.toml");
    std::fs::write(
        &bad,
        text.replace("Public-Tests/B/one", "Public-Tests/../one"),
    )
    .unwrap();
    let outcome = run_mt(&[
        "vectors",
        "tractor",
        "check",
        "--pin",
        bad.to_str().unwrap(),
        "--work-dir",
        work.to_str().unwrap(),
    ]);
    assert_eq!(outcome.code, Some(2), "stderr: {}", outcome.stderr);

    let empty = dir.path().join("empty");
    let outcome = run_mt(&[
        "vectors",
        "tractor",
        "check",
        "--pin",
        pin.to_str().unwrap(),
        "--work-dir",
        empty.to_str().unwrap(),
    ]);
    assert_eq!(outcome.code, Some(2), "stderr: {}", outcome.stderr);
    assert!(
        outcome.stderr.contains("tractor fetch"),
        "{}",
        outcome.stderr
    );
}
