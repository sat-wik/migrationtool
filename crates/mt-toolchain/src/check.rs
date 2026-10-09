//! Pins-driven version check: run every pinned tool's version command through
//! the runner, extract the version with the pinned regex and compare.

use std::path::Path;
use std::time::Duration;

use crate::pins::{Pins, ToolPinStatus};
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
}

/// The result of a whole check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckReport {
    /// One row per tool, in key order.
    pub rows: Vec<ToolRow>,
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
        });
    }
    Ok(CheckReport { rows })
}
