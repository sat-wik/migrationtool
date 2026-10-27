//! Mapping a finished run to exactly one outcome (CONTEXT D-03, D-25, D-27).
//!
//! The decision uses only the runner's record and the result frame, never text
//! the guest printed. The table:
//!
//! | Runner result | Outcome |
//! |---|---|
//! | `timed_out` | `timeout`, rc -9, label `timeout` (never ok) |
//! | host signal on `qemu-arm`, `tree_kill_sent`, incomplete or truncated stream | `executor_error` |
//! | exit 0, well-formed frame whose length equals the whole stdout | `ok` (rc from the frame) |
//! | exit 0, slack check set in the frame | `fault`, label `oob-write:guard-slack` |
//! | exit 132, 135, 136, 139 (handler: 128 + SIGILL, SIGBUS, SIGFPE, SIGSEGV) | `fault`, rc = -signal |
//! | exit 134 (abort or a failed assert) | `abort`, rc -6, label `crash:SIGABRT` |
//! | driver exit 95..=99, any other code, malformed or missing frame | `executor_error` |
//!
//! `executor_error` never becomes a vector. `setup_failed` is constructed from
//! the frame's status word by the plan that adds setup steps; it is listed so
//! the class set is complete.

use mt_toolchain::runner::RunOutput;

use crate::frame::parse_frame;

/// The outcome classes a vector can record, in report order. `executor_error`
/// is not among them.
pub const OUTCOME_CLASSES: [&str; 5] = ["ok", "fault", "abort", "timeout", "setup_failed"];

/// The signal the runner kills with on a timeout.
const TIMEOUT_RC: i32 = -9;

/// `rc` of an abort: -SIGABRT.
const ABORT_RC: i32 = -6;

/// `has_ub` label of a changed guard-slack byte.
const SLACK_LABEL: &str = "oob-write:guard-slack";

/// How one executor run ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The call returned and the result frame is well formed.
    Ok {
        /// The raw return value reinterpreted as `i32`.
        rc: i32,
    },
    /// The C code faulted, or wrote outside its buffer.
    Fault {
        /// The signal of the fault; `None` for a guard-slack write, which is
        /// found after the call returned.
        signal: Option<i32>,
        /// `-signal`, or the return value for a slack write.
        rc: i32,
    },
    /// `abort()` or a failed `assert`.
    Abort,
    /// The time budget ran out.
    Timeout,
    /// A setup step before the entry failed.
    SetupFailed {
        /// Which setup step.
        step: u32,
        /// What it returned.
        rc: i32,
    },
    /// The run could not be judged; never becomes a vector.
    ExecutorError {
        /// Why.
        reason: String,
    },
}

/// The name of a signal the in-guest handler covers.
fn signal_label(signal: i32) -> Option<&'static str> {
    match signal {
        4 => Some("crash:SIGILL"),
        6 => Some("crash:SIGABRT"),
        7 => Some("crash:SIGBUS"),
        8 => Some("crash:SIGFPE"),
        11 => Some("crash:SIGSEGV"),
        _ => None,
    }
}

impl Outcome {
    /// The class word written to a vector's stderr and used in reports.
    #[must_use]
    pub fn class_word(&self) -> &'static str {
        match self {
            Self::Ok { .. } => "ok",
            Self::Fault { .. } => "fault",
            Self::Abort => "abort",
            Self::Timeout => "timeout",
            Self::SetupFailed { .. } => "setup_failed",
            Self::ExecutorError { .. } => "executor_error",
        }
    }

    /// The `rc` a vector records, when the run yields one.
    #[must_use]
    pub fn rc(&self) -> Option<i32> {
        match self {
            Self::Ok { rc } | Self::Fault { rc, .. } | Self::SetupFailed { rc, .. } => Some(*rc),
            Self::Abort => Some(ABORT_RC),
            Self::Timeout => Some(TIMEOUT_RC),
            Self::ExecutorError { .. } => None,
        }
    }

    /// The `has_ub` label this outcome writes, if any (D-03, D-20).
    #[must_use]
    pub fn has_ub_label(&self) -> Option<&'static str> {
        match self {
            Self::Fault {
                signal: Some(signal),
                ..
            } => signal_label(*signal),
            Self::Fault { signal: None, .. } => Some(SLACK_LABEL),
            Self::Abort => Some("crash:SIGABRT"),
            Self::Timeout => Some("timeout"),
            Self::Ok { .. } | Self::SetupFailed { .. } | Self::ExecutorError { .. } => None,
        }
    }

    /// True when the outcome may become a vector; an executor error never does.
    #[must_use]
    pub fn is_vector_outcome(&self) -> bool {
        !matches!(self, Self::ExecutorError { .. })
    }
}

fn error(reason: impl Into<String>) -> Outcome {
    Outcome::ExecutorError {
        reason: reason.into(),
    }
}

/// Classify a finished run.
#[must_use]
pub fn classify(output: &RunOutput) -> Outcome {
    let record = &output.record;
    let exit = &record.exit;
    if exit.timed_out {
        return Outcome::Timeout;
    }
    if exit.tree_kill_sent {
        return error("a descendant held an output pipe open; the runner sent a tree kill");
    }
    if let Some(signal) = exit.signal {
        return error(format!("qemu-arm itself died of host signal {signal}"));
    }
    let Some(code) = exit.code else {
        return error("the runner reported neither an exit code nor a signal");
    };
    if !record.stdout.complete || record.stdout.truncated {
        return error("stdout is incomplete or truncated");
    }
    if !record.stderr.complete {
        return error("stderr is incomplete");
    }
    match code {
        0 => classify_frame(output),
        132 | 135 | 136 | 139 => {
            let signal = code - 128;
            Outcome::Fault {
                signal: Some(signal),
                rc: -signal,
            }
        }
        134 => Outcome::Abort,
        95 => error("the C code called exit"),
        96 => error("the driver could not open or read its request file"),
        97 => error("the driver found its request malformed"),
        98 => error("a driver mapping failed (including one beyond the reserved address space)"),
        99 => error("the driver failed during setup"),
        other => error(format!("unexpected driver exit code {other}")),
    }
}

/// An exit-0 run: the result frame decides.
fn classify_frame(output: &RunOutput) -> Outcome {
    match parse_frame(&output.stdout) {
        Ok(frame) if frame.total_len as u64 == output.record.stdout.bytes_total => {
            let rc = i32::from_le_bytes(frame.ret.to_le_bytes());
            if frame.status != 0 {
                error(format!("unexpected frame status word {}", frame.status))
            } else if frame.slack != 0 {
                Outcome::Fault { signal: None, rc }
            } else {
                Outcome::Ok { rc }
            }
        }
        Ok(_) => error("frame length differs from the bytes the driver wrote"),
        Err(err) => error(format!("malformed result frame: {err}")),
    }
}
