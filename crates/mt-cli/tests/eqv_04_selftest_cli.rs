#![allow(clippy::unwrap_used, clippy::expect_used)]
//! EQV-04: the built `mt vectors selftest` command line, launched through the
//! runner. Only argument handling, exit codes and messages are exercised; the
//! self-test itself runs in `mt-harness`'s tests.

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
fn eqv_04_selftest_cli_needs_pins() {
    // Without --pins the command is refused as invalid input.
    let outcome = run_mt(&["vectors", "selftest"]);
    assert_eq!(outcome.code, Some(2), "stderr: {}", outcome.stderr);
    assert!(
        outcome.stderr.contains("--pins"),
        "the refusal must name the missing --pins: {}",
        outcome.stderr
    );

    // A pins file whose qemu_arm binary does not exist is invalid input, and
    // the message names the missing tool.
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing").join("qemu-arm");
    let text = std::fs::read_to_string(repo_root().join("container/pins.toml")).unwrap();
    let rewritten = text.replace(
        "bin = \"/usr/bin/qemu-arm\"",
        &format!("bin = \"{}\"", missing.display()),
    );
    assert_ne!(rewritten, text, "container/pins.toml no longer pins qemu-arm");
    let pins = dir.path().join("pins.toml");
    std::fs::write(&pins, rewritten).unwrap();
    let work = dir.path().join("work");
    let outcome = run_mt(&[
        "vectors",
        "selftest",
        "--pins",
        pins.to_str().unwrap(),
        "--work-dir",
        work.to_str().unwrap(),
    ]);
    assert_eq!(outcome.code, Some(2), "stderr: {}", outcome.stderr);
    assert!(
        outcome.stderr.contains("qemu_arm"),
        "the error must name the missing tool: {}",
        outcome.stderr
    );
    assert!(
        !outcome.stdout.contains("result: passed"),
        "nothing may pass: {}",
        outcome.stdout
    );
}
