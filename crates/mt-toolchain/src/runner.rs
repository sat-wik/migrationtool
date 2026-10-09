#![allow(clippy::disallowed_types)]
// This module is the single permitted user of the process API: clippy.toml
// bans std::process::Command everywhere else, so every external tool is
// launched, bounded, captured and recorded here. (The PROJECT.md decision
// recording this exception is added in plan 01-06.)

//! The single subprocess runner.
//!
//! Every child starts from an empty environment plus a fixed base (`PATH`,
//! `LANG=C.UTF-8`, `TZ=UTC`, `SOURCE_DATE_EPOCH`) and only the named extras the
//! caller passes. Output is hashed in full but stored only up to a cap. A run
//! that exceeds its timeout is killed and recorded as `timed_out`, never as a
//! success. The returned [`RunRecord`] is versioned, uses sorted maps and holds
//! no wall-clock data, so identical runs serialize to identical JSON.
//!
//! The runner adds no network isolation (CONTEXT D-19): it controls
//! environment, capture and time only.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io::{self, Read};
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;
use sha2::{Digest, Sha256};

/// Version of the serialized [`RunRecord`] schema.
pub const RUN_RECORD_SCHEMA_VERSION: u32 = 1;

/// Default per-stream storage cap (CONTEXT D-18): 16 MiB.
pub const DEFAULT_OUTPUT_CAP: usize = 16 * 1024 * 1024;

/// Environment keys the runner always sets itself; callers may not override them.
pub const BASE_ENV_KEYS: [&str; 4] = ["LANG", "PATH", "SOURCE_DATE_EPOCH", "TZ"];

/// How long to wait for a pipe reader after the child has exited (Pitfall 15).
const DRAIN_GRACE: Duration = Duration::from_secs(5);

/// How often the wait loop polls the child.
const POLL_INTERVAL: Duration = Duration::from_millis(5);

/// Size of the buffer used when draining a pipe.
const READ_CHUNK: usize = 64 * 1024;

/// Run-wide settings that do not change per call.
#[derive(Debug, Clone)]
pub struct RunnerConfig {
    /// Value of `PATH` inside the child.
    pub path: String,
    /// Value of `SOURCE_DATE_EPOCH` inside the child.
    pub source_date_epoch: String,
    /// Maximum bytes stored per stream; every byte is still hashed.
    pub output_cap: usize,
    /// Wall-clock limit after which the child is killed.
    pub timeout: Duration,
}

impl RunnerConfig {
    /// A config with the default output cap and a 120 s timeout.
    #[must_use]
    pub fn new(path: impl Into<String>, source_date_epoch: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            source_date_epoch: source_date_epoch.into(),
            output_cap: DEFAULT_OUTPUT_CAP,
            timeout: Duration::from_secs(120),
        }
    }
}

/// One subprocess invocation.
#[derive(Debug, Clone)]
pub struct RunRequest {
    /// Logical tool name recorded in the run record.
    pub tool_name: String,
    /// Program followed by its arguments. Never a shell string.
    pub argv: Vec<OsString>,
    /// Working directory of the child.
    pub cwd: PathBuf,
    /// Named environment variables added on top of the fixed base.
    pub extra_env: BTreeMap<String, String>,
    /// Version of the tool, when the caller knows it.
    pub tool_version: Option<String>,
}

/// The tool identity stamped into a record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ToolStamp {
    /// Logical tool name.
    pub name: String,
    /// Program path as given in `argv[0]`.
    pub path: String,
    /// Tool version, if known.
    pub version: Option<String>,
}

/// How the child ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExitRecord {
    /// Exit code, absent when the child was killed by a signal.
    pub code: Option<i32>,
    /// Terminating signal, if any.
    pub signal: Option<i32>,
    /// True when the runner killed the child for exceeding the timeout.
    pub timed_out: bool,
}

/// What was captured from one output stream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StreamRecord {
    /// Lowercase hex sha256 of every byte the child wrote.
    pub sha256: String,
    /// Total bytes the child wrote.
    pub bytes_total: u64,
    /// Bytes kept in memory (at most the cap).
    pub bytes_stored: u64,
    /// True when `bytes_total` exceeded the cap.
    pub truncated: bool,
}

/// The versioned record of one run (becomes evidence in Phase 11).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RunRecord {
    /// [`RUN_RECORD_SCHEMA_VERSION`].
    pub schema_version: u32,
    /// Arguments in call order, lossily converted to strings.
    pub argv: Vec<String>,
    /// Working directory, lossily converted to a string.
    pub cwd: String,
    /// The effective environment, sorted by key.
    pub env: BTreeMap<String, String>,
    /// Tool identity.
    pub tool: ToolStamp,
    /// Exit information.
    pub exit: ExitRecord,
    /// Standard output capture.
    pub stdout: StreamRecord,
    /// Standard error capture.
    pub stderr: StreamRecord,
}

impl RunRecord {
    /// Pretty JSON plus a trailing newline. Holds no wall-clock or duration data.
    ///
    /// # Errors
    /// Returns [`RunnerError::Serialize`] if serialization fails.
    pub fn to_json(&self) -> Result<String, RunnerError> {
        let mut text = serde_json::to_string_pretty(self)?;
        text.push('\n');
        Ok(text)
    }
}

/// A finished run: the record plus the stored output bytes.
#[derive(Debug, Clone)]
pub struct RunOutput {
    /// The versioned record.
    pub record: RunRecord,
    /// Stored stdout bytes (at most the cap).
    pub stdout: Vec<u8>,
    /// Stored stderr bytes (at most the cap).
    pub stderr: Vec<u8>,
}

/// Why a run could not be completed.
#[derive(Debug, thiserror::Error)]
pub enum RunnerError {
    /// `argv` held no program.
    #[error("empty argv: nothing to run")]
    EmptyArgv,
    /// A named extra tried to override one of the fixed base variables.
    #[error("environment key {key} is part of the fixed base and cannot be overridden")]
    EnvConflict {
        /// The offending key.
        key: String,
    },
    /// The OS refused to start the program.
    #[error("cannot start {program}: {source}")]
    Spawn {
        /// The program as given.
        program: String,
        /// The underlying OS error.
        #[source]
        source: io::Error,
    },
    /// A pipe did not reach end-of-file after the child ended.
    #[error("{stream} was not drained within the grace period")]
    OutputNotDrained {
        /// `stdout` or `stderr`.
        stream: &'static str,
    },
    /// Any other I/O failure while supervising the child.
    #[error("i/o error while running a tool: {0}")]
    Io(#[from] io::Error),
    /// The record could not be serialized.
    #[error("cannot serialize run record: {0}")]
    Serialize(#[from] serde_json::Error),
}

impl RunnerError {
    /// True when the program does not exist (a spawn failure with `NotFound`).
    #[must_use]
    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::Spawn { source, .. } if source.kind() == io::ErrorKind::NotFound)
    }
}

struct Captured {
    record: StreamRecord,
    stored: Vec<u8>,
}

/// Lowercase hex. sha2 0.11 digests do not implement `LowerHex` (Pitfall 6).
fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        // Writing into a String cannot fail.
        let _ = write!(text, "{byte:02x}");
    }
    text
}

/// Drain a pipe to EOF: hash every byte, store only the first `cap` bytes.
fn drain<R: Read>(mut reader: R, cap: usize) -> io::Result<Captured> {
    let mut hasher = Sha256::new();
    let mut stored: Vec<u8> = Vec::new();
    let mut total: u64 = 0;
    let mut buf = vec![0u8; READ_CHUNK];
    loop {
        let n = match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        let chunk = &buf[..n];
        hasher.update(chunk);
        total += n as u64;
        let room = cap.saturating_sub(stored.len());
        stored.extend_from_slice(&chunk[..n.min(room)]);
    }
    let bytes_stored = stored.len() as u64;
    Ok(Captured {
        record: StreamRecord {
            sha256: hex(&hasher.finalize()),
            bytes_total: total,
            bytes_stored,
            truncated: total > bytes_stored,
        },
        stored,
    })
}

/// Start a reader thread for one pipe and return the channel its result arrives on.
fn spawn_reader<R: Read + Send + 'static>(
    reader: R,
    cap: usize,
) -> mpsc::Receiver<io::Result<Captured>> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        // The receiver may already be gone after a timeout; nothing to do then.
        let _ = tx.send(drain(reader, cap));
    });
    rx
}

fn collect(
    rx: &mpsc::Receiver<io::Result<Captured>>,
    stream: &'static str,
) -> Result<Captured, RunnerError> {
    match rx.recv_timeout(DRAIN_GRACE) {
        Ok(result) => Ok(result?),
        Err(mpsc::RecvTimeoutError::Timeout) => Err(RunnerError::OutputNotDrained { stream }),
        Err(mpsc::RecvTimeoutError::Disconnected) => Err(RunnerError::Io(io::Error::other(
            format!("{stream} reader thread ended without a result"),
        ))),
    }
}

/// Run one subprocess under the runner's environment, capture and time policy.
///
/// # Errors
/// - [`RunnerError::EmptyArgv`] and [`RunnerError::EnvConflict`] before anything is spawned.
/// - [`RunnerError::Spawn`] when the OS cannot start the program (see
///   [`RunnerError::is_not_found`]).
/// - [`RunnerError::OutputNotDrained`] when a pipe stays open after the child ended.
/// - [`RunnerError::Io`] for any other supervision failure.
pub fn run(cfg: &RunnerConfig, req: &RunRequest) -> Result<RunOutput, RunnerError> {
    let (program, args) = req.argv.split_first().ok_or(RunnerError::EmptyArgv)?;
    if let Some(key) = req
        .extra_env
        .keys()
        .find(|key| BASE_ENV_KEYS.contains(&key.as_str()))
    {
        return Err(RunnerError::EnvConflict { key: key.clone() });
    }

    let mut env: BTreeMap<String, String> = BTreeMap::new();
    env.insert("LANG".to_owned(), "C.UTF-8".to_owned());
    env.insert("PATH".to_owned(), cfg.path.clone());
    env.insert(
        "SOURCE_DATE_EPOCH".to_owned(),
        cfg.source_date_epoch.clone(),
    );
    env.insert("TZ".to_owned(), "UTC".to_owned());
    env.extend(req.extra_env.iter().map(|(k, v)| (k.clone(), v.clone())));

    let mut child = Command::new(program)
        .args(args)
        .env_clear()
        .envs(&env)
        .current_dir(&req.cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|source| RunnerError::Spawn {
            program: program.to_string_lossy().into_owned(),
            source,
        })?;

    let (Some(stdout_pipe), Some(stderr_pipe)) = (child.stdout.take(), child.stderr.take()) else {
        // Both pipes were requested above, so this is unreachable in practice;
        // do not leave a child running if it ever happens.
        let _ = child.kill();
        let _ = child.wait();
        return Err(RunnerError::Io(io::Error::other(
            "child pipes were not created",
        )));
    };
    let stdout_rx = spawn_reader(stdout_pipe, cfg.output_cap);
    let stderr_rx = spawn_reader(stderr_pipe, cfg.output_cap);

    let started = Instant::now();
    let mut timed_out = false;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if !timed_out && started.elapsed() >= cfg.timeout {
            child.kill()?;
            timed_out = true;
        }
        thread::sleep(POLL_INTERVAL);
    };

    let stdout = collect(&stdout_rx, "stdout")?;
    let stderr = collect(&stderr_rx, "stderr")?;

    let record = RunRecord {
        schema_version: RUN_RECORD_SCHEMA_VERSION,
        argv: req
            .argv
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect(),
        cwd: req.cwd.to_string_lossy().into_owned(),
        env,
        tool: ToolStamp {
            name: req.tool_name.clone(),
            path: program.to_string_lossy().into_owned(),
            version: req.tool_version.clone(),
        },
        exit: ExitRecord {
            code: status.code(),
            signal: status.signal(),
            timed_out,
        },
        stdout: stdout.record,
        stderr: stderr.record,
    };
    Ok(RunOutput {
        record,
        stdout: stdout.stored,
        stderr: stderr.stored,
    })
}
