//! Pins-driven version check: run every pinned tool's version command through
//! the runner, extract the version with the pinned regex and compare.

use std::path::Path;
use std::time::Duration;

use regex::{Regex, RegexBuilder};

use crate::pins::{Pins, ToolSpec};
use crate::runner::{self, RunRequest, RunnerConfig, RunnerError};

/// Compiled-regex size limit (threat T-01-05): 1 MiB.
const REGEX_SIZE_LIMIT: usize = 1024 * 1024;

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
}

impl ToolStatus {
    /// The word printed in the table.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::Mismatch => "MISMATCH",
            Self::Missing => "MISSING",
        }
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
    /// True only when every row is [`ToolStatus::Ok`].
    #[must_use]
    pub fn passed(&self) -> bool {
        self.rows.iter().all(|row| row.status == ToolStatus::Ok)
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
    /// A tool's `version_regex` is not a valid (or is an oversized) regex.
    #[error("tool {key}: invalid version_regex: {source}")]
    Regex {
        /// The tool key.
        key: String,
        /// The regex error.
        #[source]
        source: regex::Error,
    },
    /// A tool's `version_regex` has no `version` named group.
    #[error("tool {key}: version_regex has no `version` named group")]
    NoVersionGroup {
        /// The tool key.
        key: String,
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

fn compile(key: &str, spec: &ToolSpec) -> Result<Regex, CheckError> {
    let regex = RegexBuilder::new(&spec.version_regex)
        .size_limit(REGEX_SIZE_LIMIT)
        .build()
        .map_err(|source| CheckError::Regex {
            key: key.to_owned(),
            source,
        })?;
    if regex
        .capture_names()
        .flatten()
        .any(|name| name == "version")
    {
        Ok(regex)
    } else {
        Err(CheckError::NoVersionGroup {
            key: key.to_owned(),
        })
    }
}

fn extract(regex: &Regex, text: &str) -> Option<String> {
    regex
        .captures(text)
        .and_then(|caps| caps.name("version"))
        .map(|m| m.as_str().to_owned())
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
        let regex = compile(key, spec)?;
        let mut argv = vec![spec.bin.clone().into()];
        argv.extend(spec.version_args.iter().map(Into::into));
        let request = RunRequest {
            tool_name: key.clone(),
            argv,
            cwd: cwd.to_path_buf(),
            extra_env: std::collections::BTreeMap::new(),
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
                    match extract(&regex, &stdout).or_else(|| extract(&regex, &stderr)) {
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
