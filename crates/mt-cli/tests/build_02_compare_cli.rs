#![allow(clippy::unwrap_used, clippy::expect_used)]
//! BUILD-02 (WR-05): the built `mt testbed compare` command line, launched through
//! the runner. The comparison rules are tested in the library crate; here only the
//! argument handling, the verdict text and the exit codes are exercised.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use mt_toolchain::runner::{self, RunRequest, RunnerConfig};

struct Outcome {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../mt-capture/tests/fixtures/ci")
        .canonicalize()
        .unwrap()
}

/// Run the built `mt` with `args` (everything after the program name).
fn run_mt(cwd: &Path, args: &[OsString]) -> Outcome {
    let cfg = RunnerConfig::new("/usr/bin:/bin", "1791158400");
    let mut argv: Vec<OsString> = vec![env!("CARGO_BIN_EXE_mt").into()];
    argv.extend(args.iter().cloned());
    let request = RunRequest {
        tool_name: "mt".to_owned(),
        argv,
        cwd: cwd.to_path_buf(),
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

fn compare(cwd: &Path, first: &Path, second: &Path) -> Outcome {
    run_mt(
        cwd,
        &[
            OsString::from("testbed"),
            OsString::from("compare"),
            OsString::from("--first"),
            first.as_os_str().to_owned(),
            OsString::from("--second"),
            second.as_os_str().to_owned(),
        ],
    )
}

/// A capture output tree holding the committed CI capture report and its
/// compile_commands file under `<id>/`.
fn tree_with_ci_fixtures(root: &Path) {
    let dir = root.join("corejson");
    std::fs::create_dir_all(&dir).unwrap();
    for name in [
        "corejson-default.capture.json",
        "corejson-default.compile_commands.json",
    ] {
        std::fs::copy(fixture_dir().join(name), dir.join(name)).unwrap();
    }
}

#[test]
fn build_02_compare_cli_identical_trees_exit_zero() {
    let work = tempfile::tempdir().unwrap();
    let (first, second) = (work.path().join("a"), work.path().join("b"));
    tree_with_ci_fixtures(&first);
    tree_with_ci_fixtures(&second);

    let outcome = compare(work.path(), &first, &second);
    assert_eq!(outcome.code, Some(0), "stderr: {}", outcome.stderr);
    assert!(
        outcome.stdout.contains("compared 2 files"),
        "{}",
        outcome.stdout
    );
    assert!(
        outcome.stdout.contains("result: identical"),
        "{}",
        outcome.stdout
    );
}

#[test]
fn build_02_compare_cli_fails_closed_on_unparseable_reports() {
    // WR-05: the same truncated report on both sides made `jq ... | cmp -s` say
    // "identical". It must be a difference.
    let work = tempfile::tempdir().unwrap();
    for side in ["a", "b"] {
        let dir = work.path().join(side).join("corejson");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("corejson-default.json"),
            "{\"kind\": \"capture\", \"steps\": [",
        )
        .unwrap();
    }

    let outcome = compare(work.path(), &work.path().join("a"), &work.path().join("b"));
    assert_eq!(
        outcome.code,
        Some(1),
        "stdout: {} stderr: {}",
        outcome.stdout,
        outcome.stderr
    );
    assert!(
        outcome.stdout.contains("cannot parse"),
        "{}",
        outcome.stdout
    );
    assert!(
        outcome.stdout.contains("result: differences found"),
        "{}",
        outcome.stdout
    );
}

#[test]
fn build_02_compare_cli_missing_tree_exits_two() {
    let work = tempfile::tempdir().unwrap();
    let first = work.path().join("a");
    tree_with_ci_fixtures(&first);
    let missing = work.path().join("absent-tree");

    let outcome = compare(work.path(), &first, &missing);
    assert_eq!(outcome.code, Some(2), "stdout: {}", outcome.stdout);
    // The refusal must come from the comparison naming the tree, not from the
    // argument parser rejecting an unknown subcommand.
    assert!(
        outcome.stderr.contains("absent-tree"),
        "stderr: {}",
        outcome.stderr
    );
    assert!(
        !outcome.stderr.contains("unrecognized subcommand"),
        "stderr: {}",
        outcome.stderr
    );
}
