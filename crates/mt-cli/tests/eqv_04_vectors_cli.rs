#![allow(clippy::unwrap_used, clippy::expect_used)]
//! EQV-03, EQV-04: the built `mt vectors adapters` command line, launched
//! through the runner. Only argument handling, exit codes and rows are
//! exercised; the rules are tested in `mt-capture` and `mt-harness`.

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
fn eqv_04_adapters_cli_lists_entries() {
    let outcome = run_mt(&["vectors", "adapters", "--manifest", "testbed/manifest.toml"]);
    assert_eq!(outcome.code, Some(0), "stderr: {}", outcome.stderr);
    let rows: Vec<&str> = outcome.stdout.lines().collect();
    assert!(
        rows.contains(&"corejson/JSON_Validate roles=2 seeds=108 drivable"),
        "stdout: {}",
        outcome.stdout
    );
    assert!(
        rows.iter()
            .any(|row| row.starts_with("corejson/JSON_SearchT no_roles")),
        "an entry without roles prints a no_roles row: {}",
        outcome.stdout
    );
    let last = rows.last().copied().unwrap_or_default();
    assert!(
        last.starts_with("result: passed ("),
        "the output ends with the result line: {last}"
    );
}

#[test]
fn eqv_04_adapters_cli_needs_a_manifest() {
    let outcome = run_mt(&["vectors", "adapters"]);
    assert_eq!(outcome.code, Some(2), "stderr: {}", outcome.stderr);
    assert!(
        outcome.stderr.contains("--manifest"),
        "the refusal names the missing option: {}",
        outcome.stderr
    );
}
