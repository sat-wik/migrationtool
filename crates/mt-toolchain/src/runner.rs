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
//! success. The child leads its own process group. A timeout stops that group,
//! finds descendants that left it (for example through `setsid`) by walking
//! `/proc` parent links, and kills the group and each of those descendants
//! through `rustix` (group signals and pidfds), so none of them can keep the
//! output pipes open. No process other than the requested tool is ever started.
//!
//! A child that exits on its own while a descendant still holds an output pipe
//! is handled the same way: on Linux the exited child is kept unreaped (so its
//! pid and group id cannot be reused) while the pipes get `drain_grace` to
//! close; if they do not, the tree is killed, the child is reaped and the record
//! is returned with `exit.descendants_killed`. A stream that still does not
//! reach end-of-file is returned with `complete: false` and its partial bytes.
//!
//! The `/proc` walk and the pidfds exist only on Linux. On other unix systems
//! (for example macOS, where the founder may build and run `mt` locally) the
//! runner compiles and a timeout kills the child's process group through
//! `rustix`'s portable `kill_process_group`; a descendant that left the group
//! is not found there. The pinned container, and so every evidence run, is
//! Linux.
//!
//! The returned [`RunRecord`] is versioned, uses sorted maps and holds no
//! wall-clock data, so identical runs serialize to identical JSON.
//!
//! The runner adds no network isolation (CONTEXT D-19): it controls
//! environment, capture and time only.

#[cfg(not(unix))]
compile_error!(
    "mt-toolchain's runner needs a unix host (process groups and signals); \
     build and run mt on Linux or macOS, or inside the pinned container"
);

use std::collections::BTreeMap;
#[cfg(target_os = "linux")]
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::io::{self, Read};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, mpsc};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(target_os = "linux")]
use rustix::fd::OwnedFd;
use rustix::process::{Pid, Signal, kill_process_group};
#[cfg(target_os = "linux")]
use rustix::process::{PidfdFlags, pidfd_open, pidfd_send_signal};
use serde::Serialize;
use sha2::{Digest, Sha256};

/// Version of the serialized [`RunRecord`] schema.
pub const RUN_RECORD_SCHEMA_VERSION: u32 = 2;

/// Default per-stream storage cap (CONTEXT D-18): 16 MiB.
pub const DEFAULT_OUTPUT_CAP: usize = 16 * 1024 * 1024;

/// Environment keys the runner always sets itself; callers may not override them.
pub const BASE_ENV_KEYS: [&str; 4] = ["LANG", "PATH", "SOURCE_DATE_EPOCH", "TZ"];

/// Default for [`RunnerConfig::drain_grace`]: how long to wait for a pipe reader
/// after the child has exited (Pitfall 15).
const DRAIN_GRACE: Duration = Duration::from_secs(5);

/// How often the wait loop polls the child.
const POLL_INTERVAL: Duration = Duration::from_millis(5);

/// Size of the buffer used when draining a pipe.
const READ_CHUNK: usize = 64 * 1024;

/// Most `/proc` scans [`kill_tree`] makes while looking for escaped descendants.
#[cfg(target_os = "linux")]
const ESCAPE_SCAN_ROUNDS: usize = 16;

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
    /// How long the output pipes may stay open after the child has exited
    /// before the runner kills what still holds them (Pitfall 15), and then how
    /// long it waits for them to close before it returns a partial capture.
    pub drain_grace: Duration,
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
            drain_grace: DRAIN_GRACE,
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
    /// True when the child exited on its own but a descendant kept an output
    /// pipe open past the drain grace, so the runner killed the child's process
    /// tree. `code` and `signal` are the child's own exit.
    pub descendants_killed: bool,
}

/// What was captured from one output stream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StreamRecord {
    /// Lowercase hex sha256 of every byte read from the pipe. It covers every
    /// byte the child wrote only when `complete` is true.
    pub sha256: String,
    /// Total bytes read from the pipe.
    pub bytes_total: u64,
    /// Bytes kept in memory (at most the cap).
    pub bytes_stored: u64,
    /// True when `bytes_total` exceeded the cap: bytes were read and hashed but
    /// not stored. It says nothing about `complete`.
    pub truncated: bool,
    /// True when the pipe reached end-of-file, so the record covers the whole
    /// stream. False means a process the runner could not find still held the
    /// pipe open when the runner stopped waiting: `sha256` and the byte counts
    /// then describe only the partial output read up to that point (an empty
    /// stream is a different thing: `complete` true with zero bytes), and the
    /// stream must not be read as the tool's whole output.
    pub complete: bool,
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

/// What a reader thread has read from one pipe so far. The thread updates it
/// per chunk, so the supervisor can take a snapshot (the bytes, the byte count
/// and the running digest) even when the pipe never reaches end-of-file.
struct Partial {
    hasher: Sha256,
    stored: Vec<u8>,
    total: u64,
}

/// Lock the shared state. A poisoned lock only means a reader thread panicked
/// after a complete update, so the data is still consistent.
fn lock(shared: &Mutex<Partial>) -> MutexGuard<'_, Partial> {
    shared.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Drain a pipe to EOF: hash every byte, store only the first `cap` bytes.
fn drain<R: Read>(mut reader: R, cap: usize, shared: &Mutex<Partial>) -> io::Result<()> {
    let mut buf = vec![0u8; READ_CHUNK];
    loop {
        let n = match reader.read(&mut buf) {
            Ok(0) => return Ok(()),
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        let chunk = &buf[..n];
        let mut state = lock(shared);
        state.hasher.update(chunk);
        state.total += n as u64;
        let room = cap.saturating_sub(state.stored.len());
        state.stored.extend_from_slice(&chunk[..n.min(room)]);
    }
}

/// The reader thread of one pipe: its shared state and a channel that signals
/// end-of-file (or a read error).
struct StreamReader {
    shared: Arc<Mutex<Partial>>,
    done: mpsc::Receiver<io::Result<()>>,
    finished: Option<io::Result<()>>,
}

/// Start a reader thread for one pipe.
fn spawn_reader<R: Read + Send + 'static>(reader: R, cap: usize) -> StreamReader {
    let shared = Arc::new(Mutex::new(Partial {
        hasher: Sha256::new(),
        stored: Vec::new(),
        total: 0,
    }));
    let (tx, done) = mpsc::channel();
    let thread_shared = Arc::clone(&shared);
    thread::spawn(move || {
        // The receiver may already be gone after an abandoned run; nothing to do then.
        let _ = tx.send(drain(reader, cap, &thread_shared));
    });
    StreamReader {
        shared,
        done,
        finished: None,
    }
}

impl StreamReader {
    /// Wait until the pipe reaches end-of-file or `until` passes. True when the
    /// reader has finished (successfully or with a read error).
    fn wait(&mut self, until: Instant) -> bool {
        if self.finished.is_some() {
            return true;
        }
        let left = until.saturating_duration_since(Instant::now());
        match self.done.recv_timeout(left) {
            Ok(result) => self.finished = Some(result),
            Err(mpsc::RecvTimeoutError::Timeout) => return false,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                self.finished = Some(Err(io::Error::other(
                    "reader thread ended without a result",
                )));
            }
        }
        true
    }

    /// Everything read so far. `complete` is true only when the pipe reached
    /// end-of-file; otherwise the digest and counts describe the bytes read up
    /// to now (partial), and the reader thread is left to finish on its own.
    ///
    /// # Errors
    /// [`RunnerError::Io`] when the reader hit a read error.
    fn into_captured(self, stream: &'static str) -> Result<Captured, RunnerError> {
        let complete = match self.finished {
            Some(Ok(())) => true,
            Some(Err(source)) => {
                return Err(RunnerError::Io(io::Error::new(
                    source.kind(),
                    format!("reading {stream}: {source}"),
                )));
            }
            None => false,
        };
        let mut state = lock(&self.shared);
        let bytes_stored = state.stored.len() as u64;
        let record = StreamRecord {
            sha256: hex(&state.hasher.clone().finalize()),
            bytes_total: state.total,
            bytes_stored,
            truncated: state.total > bytes_stored,
            complete,
        };
        let stored = std::mem::take(&mut state.stored);
        Ok(Captured { record, stored })
    }
}

/// Wait for both readers until `until`, then take what each has read. A pipe
/// that is still open (a process the runner could not find holds it) is
/// reported as incomplete, with the partial bytes, never discarded.
fn collect_streams(
    stdout: StreamReader,
    stderr: StreamReader,
    until: Instant,
) -> Result<(Captured, Captured), RunnerError> {
    let (mut stdout, mut stderr) = (stdout, stderr);
    stdout.wait(until);
    stderr.wait(until);
    Ok((
        stdout.into_captured("stdout")?,
        stderr.into_captured("stderr")?,
    ))
}

/// The fields of `/proc/<pid>/stat` that [`kill_tree`] needs.
#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ProcStat {
    /// Parent process id (field 4).
    ppid: i32,
    /// Process group id (field 5).
    pgrp: i32,
    /// Session id (field 6).
    session: i32,
    /// Start time in clock ticks since boot (field 22). With the pid it names
    /// one process for the life of the boot.
    start_time: u64,
}

/// Parse one `/proc/<pid>/stat` line. The command name (field 2) may contain
/// spaces and parentheses, so the fields are read after the LAST `)`.
#[cfg(target_os = "linux")]
fn parse_stat(line: &str) -> Option<ProcStat> {
    let (_, rest) = line.rsplit_once(')')?;
    // `rest` starts at field 3 (the state), so field N is index N - 3.
    let fields: Vec<&str> = rest.split_whitespace().collect();
    Some(ProcStat {
        ppid: fields.get(1)?.parse().ok()?,
        pgrp: fields.get(2)?.parse().ok()?,
        session: fields.get(3)?.parse().ok()?,
        start_time: fields.get(19)?.parse().ok()?,
    })
}

/// Read and parse `/proc/<pid>/stat`; `None` when it is gone or unreadable.
#[cfg(target_os = "linux")]
fn read_stat(pid: i32) -> Option<ProcStat> {
    parse_stat(&std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?)
}

/// Every process visible in `/proc`, by pid. Unreadable entries are skipped.
#[cfg(target_os = "linux")]
fn scan_proc() -> BTreeMap<i32, ProcStat> {
    let mut table = BTreeMap::new();
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return table;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(pid) = name.to_str().and_then(|n| n.parse::<i32>().ok()) else {
            continue;
        };
        if let Some(stat) = read_stat(pid) {
            table.insert(pid, stat);
        }
    }
    table
}

/// Pids of every descendant of `root` reachable through parent links.
#[cfg(target_os = "linux")]
fn descendants(table: &BTreeMap<i32, ProcStat>, root: i32) -> BTreeSet<i32> {
    let mut children: BTreeMap<i32, Vec<i32>> = BTreeMap::new();
    for (pid, stat) in table {
        children.entry(stat.ppid).or_default().push(*pid);
    }
    let mut found = BTreeSet::new();
    let mut queue = vec![root];
    while let Some(parent) = queue.pop() {
        for child in children.get(&parent).into_iter().flatten() {
            if found.insert(*child) {
                queue.push(*child);
            }
        }
    }
    found
}

/// Best-effort kill of the process group led by `child` and of every descendant
/// that escaped the group (for example through `setsid`). Starts no process.
///
/// It must be called while the child has not been reaped: an unreaped child
/// keeps its pid, and so the group id, from being reused. Order:
/// 1. SIGSTOP the group, so its members stop forking.
/// 2. Up to [`ESCAPE_SCAN_ROUNDS`] times, scan `/proc` for descendants of the
///    child, open a pidfd for each new one, keep it only if the process start
///    time still equals the scanned one (so the fd names the scanned process,
///    not a reused pid), and SIGSTOP it through the pidfd. Stopped processes
///    cannot fork, so the set converges.
/// 3. SIGKILL every held pidfd, then the group.
///
/// A bare pid is never signalled. All errors are ignored; callers also kill the
/// direct child.
///
/// Residual limits:
/// - A descendant whose ancestry was cut before the timeout is not found, for
///   example a double fork whose middle process already exited, so the
///   descendant was reparented to init or a subreaper. If it holds a pipe, the
///   run is still returned as `timed_out`, with that stream marked
///   `complete: false` and holding the partial output read so far.
/// - Without `/proc` or pidfd support (kernel below 5.3, or a seccomp filter),
///   only the child's group is signalled.
/// - `PR_SET_CHILD_SUBREAPER` was rejected: it is process-wide, and it would
///   make this process reap arbitrary orphans, racing `Child::wait` in
///   concurrent runs.
/// - Processes of another uid cannot be signalled.
#[cfg(target_os = "linux")]
fn kill_tree(child: &Child) {
    let group = Pid::from_child(child);
    let root = group.as_raw_pid();
    // Freeze the group so its members stop forking while the tree is walked.
    let _ = kill_process_group(group, Signal::STOP);

    let mut held: BTreeMap<i32, OwnedFd> = BTreeMap::new();
    for _ in 0..ESCAPE_SCAN_ROUNDS {
        let table = scan_proc();
        let mut added = false;
        for pid in descendants(&table, root) {
            if held.contains_key(&pid) {
                continue;
            }
            let (Some(scanned), Some(target)) = (table.get(&pid), Pid::from_raw(pid)) else {
                continue;
            };
            let Ok(fd) = pidfd_open(target, PidfdFlags::empty()) else {
                continue;
            };
            // The fd names the scanned process only if the start time is
            // unchanged now that the fd is held.
            if read_stat(pid).map(|now| now.start_time) != Some(scanned.start_time) {
                continue;
            }
            let _ = pidfd_send_signal(&fd, Signal::STOP);
            held.insert(pid, fd);
            added = true;
        }
        if !added {
            break;
        }
    }

    for fd in held.values() {
        let _ = pidfd_send_signal(fd, Signal::KILL);
    }
    let _ = kill_process_group(group, Signal::KILL);
}

/// Non-Linux unix: there is no `/proc` or pidfd, so only the process group the
/// child leads is killed, through `rustix`'s portable API. A descendant that
/// left the group (for example through `setsid`) is not found.
///
/// It must be called while the child has not been reaped, so the pid, and with
/// it the group id, cannot be reused.
#[cfg(not(target_os = "linux"))]
fn kill_tree(child: &Child) {
    let _ = kill_process_group(Pid::from_child(child), Signal::KILL);
}

/// Kill the tree and the direct child and reap the child. Used on every early
/// return so that no supervision failure leaves a running or zombie child.
fn abort(child: &mut Child) {
    kill_tree(child);
    let _ = child.kill();
    let _ = child.wait();
}

/// True when the child has exited. On Linux the child is NOT reaped: it stays a
/// zombie, so its pid, and with it the process group it leads, cannot be reused
/// until the caller reaps it with `Child::wait`. That is what lets the runner
/// kill the group of a child that exited while a descendant is still running.
#[cfg(target_os = "linux")]
fn child_exited(child: &mut Child) -> io::Result<bool> {
    use rustix::io::Errno;
    use rustix::process::{WaitId, WaitIdOptions, waitid};

    let options = WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT;
    match waitid(WaitId::Pid(Pid::from_child(child)), options) {
        Ok(status) => Ok(status.is_some()),
        // Interrupted by a signal: ask again on the next poll.
        Err(Errno::INTR) => Ok(false),
        Err(errno) => Err(io::Error::from(errno)),
    }
}

/// Other unix: `try_wait` reaps the child as soon as it has exited, so a
/// descendant that outlives it cannot be killed safely afterwards (the group id
/// may be reused). See [`HOLDS_EXITED_CHILD`].
#[cfg(not(target_os = "linux"))]
fn child_exited(child: &mut Child) -> io::Result<bool> {
    Ok(child.try_wait()?.is_some())
}

/// Whether [`child_exited`] leaves an exited child unreaped, so that the tree
/// can still be killed after a normal exit.
const HOLDS_EXITED_CHILD: bool = cfg!(target_os = "linux");

/// Wait until the child has exited, killing its process tree once `timeout`
/// passes. Returns whether the runner killed it for exceeding the timeout. On
/// Linux the exited child is still unreaped when this returns (see
/// [`child_exited`]); the caller reaps it.
fn supervise(child: &mut Child, timeout: Duration) -> io::Result<bool> {
    let started = Instant::now();
    let mut timed_out = false;
    loop {
        if child_exited(child)? {
            return Ok(timed_out);
        }
        if !timed_out && started.elapsed() >= timeout {
            // The child has not exited, so it is unreaped and holds its pid.
            kill_tree(child);
            child.kill()?;
            timed_out = true;
        }
        thread::sleep(POLL_INTERVAL);
    }
}

/// Run one subprocess under the runner's environment, capture and time policy.
///
/// # Errors
/// - [`RunnerError::EmptyArgv`] and [`RunnerError::EnvConflict`] before anything is spawned.
/// - [`RunnerError::Spawn`] when the OS cannot start the program (see
///   [`RunnerError::is_not_found`]).
/// - [`RunnerError::Io`] for any other supervision failure.
///
/// A pipe that is still open after the child ended is not an error. If the
/// child exited on its own and a descendant keeps a pipe open for
/// [`RunnerConfig::drain_grace`], the runner kills the child's process tree
/// while the child's pid is still held (Linux only), reaps the child, and
/// returns the record with `exit.descendants_killed` set. A stream that still
/// has not reached end-of-file (a process the runner could not find holds it)
/// is returned with `complete: false` and its partial bytes. Callers that need
/// the whole output must check `complete`.
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
        // Own process group (pgid == pid) so a timeout can kill descendants.
        .process_group(0)
        .spawn()
        .map_err(|source| RunnerError::Spawn {
            program: program.to_string_lossy().into_owned(),
            source,
        })?;

    let (Some(stdout_pipe), Some(stderr_pipe)) = (child.stdout.take(), child.stderr.take()) else {
        // Both pipes were requested above, so this is unreachable in practice;
        // do not leave a child running if it ever happens.
        abort(&mut child);
        return Err(RunnerError::Io(io::Error::other(
            "child pipes were not created",
        )));
    };
    let mut stdout_reader = spawn_reader(stdout_pipe, cfg.output_cap);
    let mut stderr_reader = spawn_reader(stderr_pipe, cfg.output_cap);

    let timed_out = match supervise(&mut child, cfg.timeout) {
        Ok(timed_out) => timed_out,
        Err(err) => {
            abort(&mut child);
            return Err(RunnerError::Io(err));
        }
    };

    // The child has exited but (on Linux) is not reaped yet. Give the pipes a
    // chance to reach end-of-file; if a descendant still holds one, kill the
    // tree now, while the pid and so the group id are held, then reap.
    let mut descendants_killed = false;
    if !timed_out && HOLDS_EXITED_CHILD {
        let until = Instant::now() + cfg.drain_grace;
        let stdout_done = stdout_reader.wait(until);
        let stderr_done = stderr_reader.wait(until);
        if !(stdout_done && stderr_done) {
            kill_tree(&child);
            descendants_killed = true;
        }
    }
    let status = match child.wait() {
        Ok(status) => status,
        Err(err) => {
            abort(&mut child);
            return Err(RunnerError::Io(err));
        }
    };

    let (stdout, stderr) = collect_streams(
        stdout_reader,
        stderr_reader,
        Instant::now() + cfg.drain_grace,
    )?;

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
            descendants_killed,
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

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::{ProcStat, parse_stat};

    #[test]
    fn tool_01_proc_stat_parse_reads_fields_after_the_last_paren() {
        // Field 22 is the 20th token after the closing parenthesis.
        let line = "4242 ((a) b (c)) S 17 4242 4242 0 -1 4194304 100 0 0 0 1 2 0 0 20 0 1 0 9876543 \
                    1000 100 18446744073709551615 0 0 0 0 0 0 0 0 0 0 0 0 17 0 0 0 0 0 0";
        assert_eq!(
            parse_stat(line),
            Some(ProcStat {
                ppid: 17,
                pgrp: 4242,
                session: 4242,
                start_time: 9_876_543,
            })
        );
    }

    #[test]
    fn tool_01_proc_stat_parse_rejects_malformed_lines() {
        assert_eq!(parse_stat(""), None);
        assert_eq!(parse_stat("no parenthesis here"), None);
        assert_eq!(parse_stat("1 (sh) S 1 1"), None);
        let non_numeric = "1 (sh) S x 1 1 0 -1 0 0 0 0 0 0 0 0 0 20 0 1 0 5 0 0";
        assert_eq!(parse_stat(non_numeric), None);
        let bad_start = "1 (sh) S 1 1 1 0 -1 0 0 0 0 0 0 0 0 0 20 0 1 0 zz 0 0";
        assert_eq!(parse_stat(bad_start), None);
    }
}
