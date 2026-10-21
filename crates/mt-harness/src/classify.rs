//! Mapping a finished run to exactly one outcome (CONTEXT D-03, D-25, D-27).
//!
//! The decision uses only the runner's record and the result frame, never text
//! the guest printed.

use mt_toolchain::runner::RunOutput;

use crate::frame::parse_frame;

/// How one executor run ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The call returned and the result frame is well formed.
    Ok {
        /// The raw return value reinterpreted as `i32`.
        rc: i32,
    },
    /// The run could not be judged; never becomes a vector.
    ExecutorError {
        /// Why.
        reason: String,
    },
}

impl Outcome {
    /// The class word written to a vector's stderr and used in reports.
    #[must_use]
    pub fn class_word(&self) -> &'static str {
        match self {
            Self::Ok { .. } => "ok",
            Self::ExecutorError { .. } => "executor_error",
        }
    }

    /// The `rc` a vector records, when the run yields one.
    #[must_use]
    pub fn rc(&self) -> Option<i32> {
        match self {
            Self::Ok { rc } => Some(*rc),
            Self::ExecutorError { .. } => None,
        }
    }

    /// The `has_ub` label this outcome writes, if any.
    #[must_use]
    pub fn has_ub_label(&self) -> Option<&'static str> {
        match self {
            Self::Ok { .. } | Self::ExecutorError { .. } => None,
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
    let streams_whole = record.stdout.complete && !record.stdout.truncated;
    if record.exit.code == Some(0)
        && record.exit.signal.is_none()
        && !record.exit.timed_out
        && !record.exit.tree_kill_sent
        && streams_whole
    {
        return match parse_frame(&output.stdout) {
            Ok(frame) if frame.total_len as u64 == record.stdout.bytes_total => {
                if frame.status == 0 {
                    Outcome::Ok {
                        rc: i32::from_le_bytes(frame.ret.to_le_bytes()),
                    }
                } else {
                    error(format!("unexpected frame status word {}", frame.status))
                }
            }
            Ok(_) => error("frame length differs from the bytes the driver wrote"),
            Err(err) => error(format!("malformed result frame: {err}")),
        };
    }
    error(format!(
        "run not classified (exit code {:?}, signal {:?})",
        record.exit.code, record.exit.signal
    ))
}
