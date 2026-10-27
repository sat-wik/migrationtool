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
