#![allow(clippy::unwrap_used, clippy::expect_used)]
//! TOOL-01 tracer: the built `mt` binary, launched through the runner, drives
//! `mt toolchain check` against a pins file whose one tool is a fake `/bin/sh`
//! version command.

use std::collections::BTreeMap;
use std::path::Path;

use mt_toolchain::runner::{self, RunRequest, RunnerConfig};

struct Outcome {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

fn pins_text(bin: &str, expect: &str) -> String {
    format!(
        r#"schema_version = 1

[image]
source_date_epoch = "1791158400"
tool_path = "/usr/bin:/bin"

[tool.fake_tool]
bin = "{bin}"
version_args = ["-c", "echo fake-tool 1.2.3"]
version_regex = 'fake-tool (?P<version>\d+\.\d+\.\d+)'
expect = "{expect}"
"#
    )
}

/// Write `pins` to a temp dir and run `mt toolchain check --pins <file>` through the runner.
fn run_mt(pins: &str) -> Outcome {
    let dir = tempfile::tempdir().unwrap();
    let pins_path = dir.path().join("pins.toml");
    std::fs::write(&pins_path, pins).unwrap();
    run_mt_with_path(dir.path(), &pins_path)
}

fn run_mt_with_path(cwd: &Path, pins_path: &Path) -> Outcome {
    let cfg = RunnerConfig::new("/usr/bin:/bin", "1791158400");
    let request = RunRequest {
        tool_name: "mt".to_owned(),
        argv: vec![
            env!("CARGO_BIN_EXE_mt").into(),
            "toolchain".into(),
            "check".into(),
            "--pins".into(),
            pins_path.as_os_str().to_owned(),
        ],
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

#[test]
fn tool_01_check_end_to_end_reports_ok_and_exits_zero() {
    let out = run_mt(&pins_text("/bin/sh", "1.2.3"));
    assert_eq!(
        out.code,
        Some(0),
        "stdout: {} stderr: {}",
        out.stdout,
        out.stderr
    );
    assert!(out.stdout.contains("fake_tool"), "stdout: {}", out.stdout);
    assert!(out.stdout.contains("OK"), "stdout: {}", out.stdout);
}

#[test]
fn tool_01_check_end_to_end_reports_mismatch_and_exits_one() {
    let out = run_mt(&pins_text("/bin/sh", "9.9.9"));
    assert_eq!(
        out.code,
        Some(1),
        "stdout: {} stderr: {}",
        out.stdout,
        out.stderr
    );
    assert!(out.stdout.contains("fake_tool"), "stdout: {}", out.stdout);
    assert!(out.stdout.contains("MISMATCH"), "stdout: {}", out.stdout);
    assert!(out.stdout.contains("1.2.3"), "stdout: {}", out.stdout);
}

#[test]
fn tool_01_check_end_to_end_reports_missing_tool_and_exits_one() {
    let out = run_mt(&pins_text("/nonexistent/fake-tool", "1.2.3"));
    assert_eq!(
        out.code,
        Some(1),
        "stdout: {} stderr: {}",
        out.stdout,
        out.stderr
    );
    assert!(out.stdout.contains("fake_tool"), "stdout: {}", out.stdout);
    assert!(out.stdout.contains("MISSING"), "stdout: {}", out.stdout);
}

#[test]
fn tool_01_check_end_to_end_rejects_invalid_pins_with_exit_two() {
    let out = run_mt("this is = not [valid toml");
    assert_eq!(
        out.code,
        Some(2),
        "stdout: {} stderr: {}",
        out.stdout,
        out.stderr
    );
    assert!(out.stderr.contains("pins"), "stderr: {}", out.stderr);
    assert!(out.stdout.is_empty(), "stdout: {}", out.stdout);
}
