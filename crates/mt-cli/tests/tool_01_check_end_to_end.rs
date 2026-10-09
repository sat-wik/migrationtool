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

/// Every schema v1 section with well-formed values, so `validate` accepts the
/// file; only the one fake tool differs between tests.
fn pins_text(bin: &str, expect: &str) -> String {
    format!(
        r#"schema_version = 1

[image]
platform = "linux/amd64"
base = "debian:bookworm-20261005-slim"
base_digest = "sha256:7c7b2c966bc9ee8cedfeef67e0e279108992c77681fa595db4a9d65c06ccc587"
snapshot_timestamp = "20261005T000000Z"
source_date_epoch = "1791158400"
tool_path = "/usr/bin:/bin"
dpkg_query = "/usr/bin/dpkg-query"

[llvm]
major = 16
max_major = 19
status = "provisional"
decision = "D-15"

[rust]
rustup_home = "/opt/rustup"
cargo_home = "/opt/cargo"
host = "x86_64-unknown-linux-gnu"

[rust.tool]
version = "1.99.0"
commit = "b940084d7eb6a299eb4bfeb8e34901bc051e7ac4"
channel_manifest_sha256 = "ce6dddc886364f8d786514771212cebe9b731ba82d6b859951c6b0ccc516b6a2"
components = ["clippy", "rustfmt"]
targets = ["thumbv7em-none-eabihf"]

[rust.bitcode]
version = "1.72.1"
commit = "d5c2e9c342b358556da91d61ed4133f6f50fc0c3"
llvm = "16.0.5"
channel_manifest_sha256 = "77113b9660855a5ab49e5ff029c998be93fdcea41b062b40420675cf1f4ea229"
decision = "D-15"

[ci]
cargo_deny_version = "0.20.2"
buildkit_image = "moby/buildkit:v0.34.0@sha256:b059f8d7226d0b326bc871af489a5dddf65e36af4d20d14251b072974d9ee87c"

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
