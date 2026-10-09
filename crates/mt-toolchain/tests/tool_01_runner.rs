#![allow(clippy::unwrap_used, clippy::expect_used)]
//! TOOL-01 runner contract: environment policy, collision rejection, output cap
//! boundary, empty streams, timeout and byte-stable records.
//!
//! Tests drive only `mt_toolchain::runner::run` and use programs that exist on
//! the cloud session and on ubuntu-24.04 runners.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use mt_toolchain::runner::{
    RUN_RECORD_SCHEMA_VERSION, RunOutput, RunRequest, RunnerConfig, RunnerError, run,
};
use sha2::{Digest, Sha256};

const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn config() -> RunnerConfig {
    RunnerConfig::new("/usr/bin:/bin", "1791158400")
}

fn request(argv: &[&str]) -> RunRequest {
    RunRequest {
        tool_name: "test_tool".to_owned(),
        argv: argv.iter().map(OsString::from).collect(),
        cwd: PathBuf::from("/"),
        extra_env: BTreeMap::new(),
        tool_version: Some("0.0.0".to_owned()),
    }
}

fn run_ok(cfg: &RunnerConfig, req: &RunRequest) -> RunOutput {
    run(cfg, req).unwrap()
}

#[test]
fn tool_01_runner_records_argv_env_exit_and_output_hashes() {
    let req = request(&["/bin/sh", "-c", "printf out; printf err 1>&2; exit 3"]);
    let output = run_ok(&config(), &req);
    let record = &output.record;

    assert_eq!(record.schema_version, RUN_RECORD_SCHEMA_VERSION);
    assert_eq!(record.schema_version, 1);
    assert_eq!(
        record.argv,
        vec!["/bin/sh", "-c", "printf out; printf err 1>&2; exit 3"]
    );
    assert_eq!(record.cwd, "/");
    assert_eq!(record.tool.name, "test_tool");
    assert_eq!(record.tool.path, "/bin/sh");
    assert_eq!(record.tool.version.as_deref(), Some("0.0.0"));
    assert_eq!(record.exit.code, Some(3));
    assert_eq!(record.exit.signal, None);
    assert!(!record.exit.timed_out);
    assert_eq!(record.stdout.sha256, sha256_hex(b"out"));
    assert_eq!(record.stderr.sha256, sha256_hex(b"err"));
    assert_eq!(record.stdout.bytes_total, 3);
    assert_eq!(record.stderr.bytes_total, 3);
    assert_eq!(output.stdout, b"out");
    assert_eq!(output.stderr, b"err");
}

#[test]
fn tool_01_runner_clears_environment_and_applies_fixed_base() {
    let mut req = request(&["/usr/bin/env"]);
    req.extra_env
        .insert("MT_TEST_EXTRA".to_owned(), "1".to_owned());
    let output = run_ok(&config(), &req);

    let text = String::from_utf8(output.stdout.clone()).unwrap();
    let seen: BTreeMap<String, String> = text
        .lines()
        .map(|line| {
            let (k, v) = line.split_once('=').unwrap();
            (k.to_owned(), v.to_owned())
        })
        .collect();
    let keys: BTreeSet<&str> = seen.keys().map(String::as_str).collect();
    let expected: BTreeSet<&str> = ["LANG", "MT_TEST_EXTRA", "PATH", "SOURCE_DATE_EPOCH", "TZ"]
        .into_iter()
        .collect();
    assert_eq!(keys, expected, "child saw: {text}");
    assert!(!seen.contains_key("CARGO_MANIFEST_DIR"));
    assert!(!seen.contains_key("HOME"));
    assert_eq!(seen["LANG"], "C.UTF-8");
    assert_eq!(seen["TZ"], "UTC");
    assert_eq!(seen["PATH"], "/usr/bin:/bin");
    assert_eq!(seen["SOURCE_DATE_EPOCH"], "1791158400");
    assert_eq!(seen["MT_TEST_EXTRA"], "1");
    // The record holds exactly the effective environment.
    assert_eq!(output.record.env, seen);
}

#[test]
fn tool_01_runner_rejects_extra_env_that_overrides_base() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("ran");
    let mut req = request(&["/bin/sh", "-c", "touch ran"]);
    req.cwd = dir.path().to_path_buf();
    req.extra_env
        .insert("PATH".to_owned(), "/somewhere/else".to_owned());

    match run(&config(), &req) {
        Err(RunnerError::EnvConflict { key }) => assert_eq!(key, "PATH"),
        other => panic!("expected EnvConflict, got {other:?}"),
    }
    assert!(!marker.exists(), "the program ran despite the conflict");
}

#[test]
fn tool_01_runner_caps_output_without_deadlock() {
    let mut cfg = config();
    cfg.output_cap = 1024;
    let req = request(&[
        "/bin/sh",
        "-c",
        "head -c 5000000 /dev/zero; head -c 5000000 /dev/zero 1>&2",
    ]);
    let output = run_ok(&cfg, &req);
    let expected = sha256_hex(&vec![0u8; 5_000_000]);

    for (name, stream, stored) in [
        ("stdout", &output.record.stdout, &output.stdout),
        ("stderr", &output.record.stderr, &output.stderr),
    ] {
        assert_eq!(stream.bytes_total, 5_000_000, "{name}");
        assert_eq!(stream.bytes_stored, 1024, "{name}");
        assert!(stream.truncated, "{name}");
        assert_eq!(stream.sha256, expected, "{name}");
        assert_eq!(stored.len(), 1024, "{name}");
    }
    assert_eq!(output.record.exit.code, Some(0));
}

#[test]
fn tool_01_runner_cap_boundary_is_exact() {
    let mut cfg = config();
    cfg.output_cap = 1024;

    let exact = run_ok(&cfg, &request(&["/bin/sh", "-c", "head -c 1024 /dev/zero"]));
    assert!(!exact.record.stdout.truncated);
    assert_eq!(exact.record.stdout.bytes_total, 1024);
    assert_eq!(exact.record.stdout.bytes_stored, 1024);
    assert_eq!(exact.stdout.len(), 1024);

    let over = run_ok(&cfg, &request(&["/bin/sh", "-c", "head -c 1025 /dev/zero"]));
    assert!(over.record.stdout.truncated);
    assert_eq!(over.record.stdout.bytes_total, 1025);
    assert_eq!(over.record.stdout.bytes_stored, 1024);
    assert_eq!(over.stdout.len(), 1024);
    // The hash covers every byte, not only the stored ones.
    assert_eq!(over.record.stdout.sha256, sha256_hex(&vec![0u8; 1025]));
}

#[test]
fn tool_01_runner_empty_output_and_empty_argv() {
    let output = run_ok(&config(), &request(&["/bin/true"]));
    for stream in [&output.record.stdout, &output.record.stderr] {
        assert_eq!(stream.sha256, EMPTY_SHA256);
        assert_eq!(stream.bytes_total, 0);
        assert_eq!(stream.bytes_stored, 0);
        assert!(!stream.truncated);
    }
    assert_eq!(output.record.exit.code, Some(0));

    let mut empty = request(&["/bin/true"]);
    empty.argv.clear();
    assert!(matches!(
        run(&config(), &empty),
        Err(RunnerError::EmptyArgv)
    ));
}

#[test]
fn tool_01_runner_timeout_is_reported_not_passed() {
    let mut cfg = config();
    cfg.timeout = Duration::from_millis(200);
    let started = Instant::now();
    let output = run_ok(&cfg, &request(&["/bin/sleep", "30"]));

    assert!(
        started.elapsed() < Duration::from_secs(10),
        "run did not stop at the timeout"
    );
    assert!(output.record.exit.timed_out);
    assert_eq!(output.record.exit.signal, Some(9));
    assert_eq!(output.record.exit.code, None);
}

#[test]
fn tool_01_runner_missing_program_is_typed_spawn_error() {
    let err = run(&config(), &request(&["/nonexistent/tool"])).unwrap_err();
    assert!(matches!(err, RunnerError::Spawn { .. }), "got {err:?}");
    assert!(err.is_not_found());
}

#[test]
fn tool_01_run_record_json_is_deterministic() {
    let req = request(&["/bin/sh", "-c", "printf hello"]);
    let first = run_ok(&config(), &req).record.to_json().unwrap();
    let second = run_ok(&config(), &req).record.to_json().unwrap();
    assert_eq!(first, second);
    assert!(first.ends_with("}\n"));

    // Env keys appear in sorted order.
    let positions: Vec<usize> = ["\"LANG\"", "\"PATH\"", "\"SOURCE_DATE_EPOCH\"", "\"TZ\""]
        .iter()
        .map(|key| first.find(key).unwrap())
        .collect();
    assert!(positions.windows(2).all(|w| w[0] < w[1]), "{first}");

    // No wall-clock or duration fields anywhere in the record.
    let value: serde_json::Value = serde_json::from_str(&first).unwrap();
    let top: BTreeSet<&str> = value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    let expected_top: BTreeSet<&str> = [
        "argv",
        "cwd",
        "env",
        "exit",
        "schema_version",
        "stderr",
        "stdout",
        "tool",
    ]
    .into_iter()
    .collect();
    assert_eq!(top, expected_top);
    let exit_keys: BTreeSet<&str> = value["exit"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    let expected_exit: BTreeSet<&str> = ["code", "signal", "timed_out"].into_iter().collect();
    assert_eq!(exit_keys, expected_exit);
    for banned in ["duration", "elapsed", "timestamp", "started", "finished"] {
        assert!(!first.contains(banned), "record mentions {banned}");
    }
}
