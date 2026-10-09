//! Pins-driven version check: run every pinned tool's version command through
//! the runner, extract the version with the pinned regex and compare.

use std::path::Path;
use std::time::Duration;

use std::collections::BTreeMap;

use crate::pins::{LlvmPins, Pins, PinsError, ToolPinStatus};
use crate::runner::{self, RunRequest, RunnerConfig, RunnerError};
use crate::version::{self, VersionError};

/// Wall-clock limit for one version command.
const VERSION_TIMEOUT: Duration = Duration::from_secs(120);

/// Outcome for one tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolStatus {
    /// The live version equals the pin.
    Ok,
    /// The tool ran but its version differs, could not be read, or it failed or timed out.
    Mismatch,
    /// The program does not exist.
    Missing,
    /// The tool is deliberately absent (CONTEXT D-10); never a failure.
    NotInstalled,
    /// The run timed out, could not be supervised, or its output was not drained.
    NotProven,
}

impl ToolStatus {
    /// The word printed in the table.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::Mismatch => "MISMATCH",
            Self::Missing => "MISSING",
            Self::NotInstalled => "not_installed",
            Self::NotProven => "not proven",
        }
    }

    /// True for statuses that do not fail a report.
    #[must_use]
    pub fn is_pass(self) -> bool {
        matches!(self, Self::Ok | Self::NotInstalled)
    }
}

/// One table row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolRow {
    /// The tool key from the pins file.
    pub key: String,
    /// The pinned version.
    pub expected: String,
    /// The version extracted from the live tool, when there was one.
    pub actual: Option<String>,
    /// The verdict.
    pub status: ToolStatus,
    /// The LLVM major the row reports, where one applies.
    pub llvm_major: Option<u32>,
    /// Why the row has its status, built from parsed values only.
    pub detail: Option<String>,
}

/// The result of a whole check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckReport {
    /// One row per item, in key order.
    pub rows: Vec<ToolRow>,
    /// The LLVM pin the rows were judged against.
    pub llvm: LlvmPins,
}

/// What one command run produced, before any rule is applied to it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Observation {
    /// The observation key.
    pub key: String,
    /// Captured standard output (lossy UTF-8).
    pub stdout: String,
    /// Captured standard error (lossy UTF-8).
    pub stderr: String,
    /// Exit code; `None` when the child was killed by a signal.
    pub exit_code: Option<i32>,
    /// True when the runner killed the child for exceeding its timeout.
    pub timed_out: bool,
    /// True when the program does not exist.
    pub missing: bool,
    /// A supervision failure (spawn refused, output not drained, i/o error).
    pub error: Option<String>,
    /// A second command observed for this one (the LLVM probe).
    pub probe: Option<Box<Observation>>,
}

impl Observation {
    /// A command that ran and exited 0 with the given output.
    #[must_use]
    pub fn ran(key: &str, stdout: &str, stderr: &str) -> Self {
        Self {
            key: key.to_owned(),
            stdout: stdout.to_owned(),
            stderr: stderr.to_owned(),
            exit_code: Some(0),
            ..Self::default()
        }
    }

    /// A command that was killed for exceeding its timeout.
    #[must_use]
    pub fn timed_out(key: &str) -> Self {
        Self {
            key: key.to_owned(),
            timed_out: true,
            ..Self::default()
        }
    }

    /// A command whose program does not exist.
    #[must_use]
    pub fn missing(key: &str) -> Self {
        Self {
            key: key.to_owned(),
            missing: true,
            ..Self::default()
        }
    }

    /// This observation with `probe` attached.
    #[must_use]
    pub fn with_probe(mut self, probe: Self) -> Self {
        self.probe = Some(Box::new(probe));
        self
    }
}

impl CheckReport {
    /// True when every row is [`ToolStatus::Ok`] or [`ToolStatus::NotInstalled`].
    #[must_use]
    pub fn passed(&self) -> bool {
        self.rows.iter().all(|row| row.status.is_pass())
    }

    /// One line per tool: key, expected, actual (`-` when unknown), status.
    #[must_use]
    pub fn render_table(&self) -> String {
        let actual_text = |row: &ToolRow| row.actual.clone().unwrap_or_else(|| "-".to_owned());
        let key_w = self.rows.iter().map(|r| r.key.len()).max().unwrap_or(0);
        let exp_w = self
            .rows
            .iter()
            .map(|r| r.expected.len())
            .max()
            .unwrap_or(0);
        let act_w = self
            .rows
            .iter()
            .map(|r| actual_text(r).len())
            .max()
            .unwrap_or(0);
        let mut out = String::new();
        for row in &self.rows {
            out.push_str(&format!(
                "{:<key_w$}  {:<exp_w$}  {:<act_w$}  {}\n",
                row.key,
                row.expected,
                actual_text(row),
                row.status.as_str(),
            ));
        }
        out
    }

    /// The `llvm pin:` line.
    #[must_use]
    pub fn pin_line(&self) -> String {
        String::new()
    }
}

/// Why a check could not be carried out (as opposed to a tool being off its pin).
#[derive(Debug, thiserror::Error)]
pub enum CheckError {
    /// A tool's `version_regex` is unusable: invalid, oversized, or without a `version` group.
    #[error("tool {key}: version_regex: {source}")]
    Version {
        /// The tool key.
        key: String,
        /// Why the pattern cannot be used.
        #[source]
        source: VersionError,
    },
    /// The runner failed for a reason other than the program being absent.
    #[error("tool {key}: {source}")]
    Runner {
        /// The tool key.
        key: String,
        /// The runner error.
        #[source]
        source: RunnerError,
    },
    /// The pins are not complete enough to check a container against.
    #[error(transparent)]
    Pins(#[from] PinsError),
    /// The capture file could not be rendered.
    #[error("cannot render the capture file: {0}")]
    Capture(#[from] toml::ser::Error),
}

/// Check every tool in `pins` and report one row per tool.
///
/// Every version command goes through [`runner::run`]. A program that does not
/// exist is `MISSING`; a non-zero exit, a timeout, or output without a match is
/// `MISMATCH` (never a pass).
///
/// # Errors
/// [`CheckError`] when a regex is unusable or the runner fails for a reason
/// other than the program being absent.
pub fn run_check(pins: &Pins, cwd: &Path) -> Result<CheckReport, CheckError> {
    let cfg = RunnerConfig {
        timeout: VERSION_TIMEOUT,
        ..RunnerConfig::new(
            pins.image.tool_path.clone(),
            pins.image.source_date_epoch.clone(),
        )
    };
    let mut rows = Vec::with_capacity(pins.tool.len());
    for (key, spec) in &pins.tool {
        if spec.status == ToolPinStatus::NotInstalled {
            rows.push(ToolRow {
                key: key.clone(),
                expected: "-".to_owned(),
                actual: None,
                status: ToolStatus::NotInstalled,
                llvm_major: None,
                detail: None,
            });
            continue;
        }
        let regex = version::compile_pattern(&spec.version_regex, "version").map_err(|source| {
            CheckError::Version {
                key: key.clone(),
                source,
            }
        })?;
        let mut argv = vec![spec.bin.clone().into()];
        argv.extend(spec.version_args.iter().map(Into::into));
        let request = RunRequest {
            tool_name: key.clone(),
            argv,
            cwd: cwd.to_path_buf(),
            extra_env: spec.env.clone(),
            tool_version: Some(spec.expect.clone()),
        };
        let (actual, status) = match runner::run(&cfg, &request) {
            Err(err) if err.is_not_found() => (None, ToolStatus::Missing),
            Err(source) => {
                return Err(CheckError::Runner {
                    key: key.clone(),
                    source,
                });
            }
            Ok(output) => {
                let exit = &output.record.exit;
                if exit.timed_out || exit.code != Some(0) {
                    (None, ToolStatus::Mismatch)
                } else {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let found = version::extract(&regex, "version", &stdout)
                        .or_else(|| version::extract(&regex, "version", &stderr));
                    match found {
                        Some(version) if version == spec.expect => (Some(version), ToolStatus::Ok),
                        Some(version) => (Some(version), ToolStatus::Mismatch),
                        None => (None, ToolStatus::Mismatch),
                    }
                }
            }
        };
        rows.push(ToolRow {
            key: key.clone(),
            expected: spec.expect.clone(),
            actual,
            status,
            llvm_major: None,
            detail: None,
        });
    }
    Ok(CheckReport {
        rows,
        llvm: pins.llvm.clone(),
    })
}

/// Run every pinned command through the runner and record what happened.
///
/// # Errors
/// [`CheckError`] when a command cannot be set up.
pub fn observe(_pins: &Pins, _cwd: &Path) -> Result<BTreeMap<String, Observation>, CheckError> {
    // Placeholder for the RED commit; the real implementation follows.
    Ok(BTreeMap::new())
}

/// Render observations as a TOML capture file (one table per observation key).
///
/// # Errors
/// [`CheckError::Capture`] if TOML serialization fails.
pub fn capture_toml(
    _observations: &BTreeMap<String, Observation>,
    _label: &str,
) -> Result<String, CheckError> {
    // Placeholder for the RED commit; the real implementation follows.
    Ok(String::new())
}

/// Judge `observations` against `pins`. Pure: no process is started.
#[must_use]
pub fn evaluate(pins: &Pins, _observations: &BTreeMap<String, Observation>) -> CheckReport {
    // Placeholder for the RED commit; the real implementation follows.
    CheckReport {
        rows: Vec::new(),
        llvm: pins.llvm.clone(),
    }
}
