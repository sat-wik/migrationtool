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
    assert_eq!(record.schema_version, 2);
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
    assert!(!record.exit.descendants_killed);
    assert_eq!(record.stdout.sha256, sha256_hex(b"out"));
    assert_eq!(record.stderr.sha256, sha256_hex(b"err"));
    assert_eq!(record.stdout.bytes_total, 3);
    assert_eq!(record.stderr.bytes_total, 3);
    assert!(record.stdout.complete && record.stderr.complete);
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
        assert!(stream.complete, "{name}");
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
        // An empty stream that reached EOF is complete, unlike one never read.
        assert!(stream.complete);
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
fn tool_01_runner_timeout_kills_descendants_and_reports_timed_out() {
    // The shell starts a background sleep that inherits the output pipes, then
    // a foreground sleep. Killing only the shell would leave the background
    // sleep holding the pipes, and the run would wait out the drain grace.
    // Killing the process group ends both at once.
    let mut cfg = config();
    cfg.timeout = Duration::from_millis(300);
    let started = Instant::now();
    let output = run_ok(
        &cfg,
        &request(&["/bin/sh", "-c", "sleep 30 & sleep 30; wait"]),
    );

    assert!(
        started.elapsed() < Duration::from_secs(4),
        "descendants outlived the timeout: {:?}",
        started.elapsed()
    );
    assert!(output.record.exit.timed_out);
    assert_eq!(output.record.exit.code, None);
    assert!(!output.record.stdout.truncated);
    assert!(!output.record.stderr.truncated);
    assert!(output.record.stdout.complete);
    assert!(output.record.stderr.complete);
}

#[cfg(target_os = "linux")]
/// True when `/proc/<pid>` is gone or the process is a zombie (state `Z`).
fn gone_or_zombie(pid: u32) -> bool {
    match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
        Err(_) => true,
        Ok(text) => text
            .rsplit_once(')')
            .and_then(|(_, rest)| rest.split_whitespace().next().map(|s| s == "Z"))
            .unwrap_or(false),
    }
}

#[cfg(target_os = "linux")]
#[test]
fn tool_01_runner_timeout_kills_a_setsid_descendant() {
    // The background sleep leaves the child's process group with setsid, so a
    // group kill alone misses it and it keeps both output pipes open.
    let mut cfg = config();
    cfg.timeout = Duration::from_millis(300);
    let started = Instant::now();
    let output = run_ok(
        &cfg,
        &request(&["/bin/sh", "-c", "setsid sleep 30 & echo $!; sleep 30"]),
    );

    assert!(
        started.elapsed() < Duration::from_secs(4),
        "a setsid descendant outlived the timeout: {:?}",
        started.elapsed()
    );
    assert!(output.record.exit.timed_out);
    assert_eq!(output.record.exit.code, None);
    assert!(!output.record.stdout.truncated);
    assert!(!output.record.stderr.truncated);
    assert!(output.record.stdout.complete);
    assert!(output.record.stderr.complete);

    let text = String::from_utf8(output.stdout.clone()).unwrap();
    let pid: u32 = text.trim().parse().unwrap();
    // Give the kernel a moment to finish tearing the killed process down.
    let deadline = Instant::now() + Duration::from_secs(2);
    while !gone_or_zombie(pid) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(gone_or_zombie(pid), "escaped pid {pid} is still running");
}

#[cfg(target_os = "linux")]
#[test]
fn tool_01_runner_timeout_kills_a_forking_setsid_subtree() {
    // The escaped shell has its own children, which stay in the escaped
    // session and group. All of them hold the output pipes.
    let mut cfg = config();
    cfg.timeout = Duration::from_millis(300);
    let started = Instant::now();
    let output = run_ok(
        &cfg,
        &request(&[
            "/bin/sh",
            "-c",
            "setsid /bin/sh -c \"sleep 30 & sleep 30\" & sleep 30",
        ]),
    );

    assert!(
        started.elapsed() < Duration::from_secs(4),
        "an escaped subtree outlived the timeout: {:?}",
        started.elapsed()
    );
    assert!(output.record.exit.timed_out);
    assert!(!output.record.stdout.truncated);
    assert!(!output.record.stderr.truncated);
    assert!(output.record.stdout.complete);
    assert!(output.record.stderr.complete);
}

#[cfg(target_os = "linux")]
#[test]
fn tool_01_runner_marks_a_stream_held_by_an_orphaned_escapee_incomplete() {
    // Documented limit: the escapee is orphaned before the timeout, so no ppid
    // chain leads to it. The run is still returned, timed_out, with the stream
    // the escapee holds marked incomplete. Nothing was read from it, so the
    // counts are zero and the digest is the empty-input digest, but `complete`
    // says that this is "never reached EOF", not "the tool wrote nothing", and
    // `truncated` stays false because no cap was exceeded.
    let mut cfg = config();
    cfg.timeout = Duration::from_millis(300);
    let started = Instant::now();
    let output = run_ok(
        &cfg,
        &request(&["/bin/sh", "-c", "(setsid sleep 12 2>/dev/null &); sleep 30"]),
    );

    assert!(
        started.elapsed() < Duration::from_secs(9),
        "the run waited for the orphaned escapee: {:?}",
        started.elapsed()
    );
    assert!(output.record.exit.timed_out);
    assert!(!output.record.stdout.complete);
    assert!(!output.record.stdout.truncated);
    assert_eq!(output.record.stdout.bytes_total, 0);
    assert_eq!(output.record.stdout.sha256, EMPTY_SHA256);
    assert!(output.record.stderr.complete);
}

#[cfg(target_os = "linux")]
#[test]
fn tool_01_runner_normal_exit_with_a_lingering_descendant_returns_the_record() {
    // The shell prints the pid of a background sleep that inherits both output
    // pipes and exits 0 at once. The runner must not drop the result with an
    // error or leave the sleeper running: it kills the tree while the shell's
    // pid is still held, reaps the shell, and returns the record.
    let mut cfg = config();
    cfg.drain_grace = Duration::from_millis(300);
    let started = Instant::now();
    let output = run_ok(
        &cfg,
        &request(&["/bin/sh", "-c", "sleep 30 & echo $!; exit 0"]),
    );

    assert!(
        started.elapsed() < Duration::from_secs(4),
        "the run waited for the lingering descendant: {:?}",
        started.elapsed()
    );
    // The shell's own exit is kept.
    assert_eq!(output.record.exit.code, Some(0));
    assert!(!output.record.exit.timed_out);
    assert!(output.record.exit.descendants_killed);
    // Killing the sleeper closed the pipes, so both streams are whole.
    assert!(output.record.stdout.complete);
    assert!(output.record.stderr.complete);

    let text = String::from_utf8(output.stdout.clone()).unwrap();
    let pid: u32 = text.trim().parse().unwrap();
    assert_eq!(output.record.stdout.sha256, sha256_hex(text.as_bytes()));
    let deadline = Instant::now() + Duration::from_secs(2);
    while !gone_or_zombie(pid) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(gone_or_zombie(pid), "lingering pid {pid} is still running");
}

#[cfg(target_os = "linux")]
#[test]
fn tool_01_runner_normal_exit_with_an_orphaned_escapee_is_incomplete_not_an_error() {
    // Documented limit: the escapee left the group with setsid and is orphaned
    // when the shell exits, so neither the group kill nor the ppid walk finds
    // it. The record of the finished shell is still returned, stdout marked
    // incomplete.
    let mut cfg = config();
    cfg.drain_grace = Duration::from_millis(300);
    let started = Instant::now();
    let output = run_ok(
        &cfg,
        &request(&["/bin/sh", "-c", "setsid sleep 6 2>/dev/null & exit 0"]),
    );

    assert!(
        started.elapsed() < Duration::from_secs(4),
        "the run waited for the escapee: {:?}",
        started.elapsed()
    );
    assert_eq!(output.record.exit.code, Some(0));
    assert!(!output.record.exit.timed_out);
    assert!(output.record.exit.descendants_killed);
    assert!(!output.record.stdout.complete);
    // The escapee's stderr goes to /dev/null, so only stdout stays open.
    assert!(output.record.stderr.complete);
}

#[cfg(target_os = "linux")]
#[test]
fn tool_01_runner_keeps_partial_output_of_an_incomplete_stream() {
    // An orphaned escapee writes a line and then keeps stdout open. The bytes
    // read before the runner gave up, and the digest of exactly those bytes,
    // are kept and the stream is marked incomplete.
    let mut cfg = config();
    cfg.timeout = Duration::from_millis(500);
    let output = run_ok(
        &cfg,
        &request(&[
            "/bin/sh",
            "-c",
            "(setsid sh -c 'echo partial; sleep 7' 2>/dev/null &); sleep 30",
        ]),
    );

    assert!(output.record.exit.timed_out);
    let stdout = &output.record.stdout;
    assert!(!stdout.complete);
    assert!(!stdout.truncated);
    assert_eq!(output.stdout, b"partial\n");
    assert_eq!(stdout.bytes_total, 8);
    assert_eq!(stdout.bytes_stored, 8);
    assert_eq!(stdout.sha256, sha256_hex(b"partial\n"));
    assert!(output.record.stderr.complete);
}

#[test]
fn tool_01_runner_signals_without_spawning_a_kill_program() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/runner.rs");
    let text = std::fs::read_to_string(path).unwrap();
    let code: String = text
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(
        code.matches("Command::new(").count(),
        1,
        "runner.rs may start only the requested tool"
    );
    assert!(!code.contains("/bin/kill"));
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
    let expected_exit: BTreeSet<&str> = ["code", "descendants_killed", "signal", "timed_out"]
        .into_iter()
        .collect();
    assert_eq!(exit_keys, expected_exit);
    for stream in ["stdout", "stderr"] {
        let keys: BTreeSet<&str> = value[stream]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        let expected: BTreeSet<&str> = [
            "bytes_stored",
            "bytes_total",
            "complete",
            "sha256",
            "truncated",
        ]
        .into_iter()
        .collect();
        assert_eq!(keys, expected, "{stream}");
        assert_eq!(value[stream]["complete"], true, "{stream}");
    }
    assert_eq!(value["schema_version"], 2);
    for banned in ["duration", "elapsed", "timestamp", "started", "finished"] {
        assert!(!first.contains(banned), "record mentions {banned}");
    }
}
