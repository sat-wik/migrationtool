#![allow(clippy::unwrap_used, clippy::expect_used)]
//! EQV-01: the built `mt vectors policy-check` command line, launched through
//! the runner. Only argument handling, exit codes and messages are exercised;
//! the rules are tested in `mt-contract`.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use mt_toolchain::runner::{self, RunRequest, RunnerConfig};

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
fn eqv_01_policy_check_cli_accepts_the_corejson_policy() {
    let outcome = run_mt(&["vectors", "policy-check", "testbed/policy/corejson.toml"]);
    assert_eq!(outcome.code, Some(0), "stderr: {}", outcome.stderr);
    assert!(
        outcome
            .stdout
            .lines()
            .any(|l| l == "corejson valid default=return_error overrides=0 error_codes=0"),
        "stdout: {}",
        outcome.stdout
    );
}

#[test]
fn eqv_01_policy_check_cli_rejects_a_bad_policy() {
    let dir = tempfile::tempdir().unwrap();
    let bad = "schema_version = 1\ncontract_version = \"0.1.0\"\nmodule = \"corejson\"\n\n\
               [ub]\ndefault_action = \"maybe\"\n";
    let path = dir.path().join("corejson.toml");
    std::fs::write(&path, bad).unwrap();
    let outcome = run_mt(&["vectors", "policy-check", path.to_str().unwrap()]);
    assert_eq!(outcome.code, Some(2), "stderr: {}", outcome.stderr);
    assert!(
        outcome.stderr.contains("default_action") || outcome.stderr.contains("maybe"),
        "the refusal must name the key: {}",
        outcome.stderr
    );
    assert!(
        !outcome.stderr.contains("unrecognized"),
        "the subcommand must exist: {}",
        outcome.stderr
    );
}
