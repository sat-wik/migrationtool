//! The drop log, `testbed/dropped.toml` (CONTEXT D-11, RESEARCH Pattern 13).
//!
//! A target or configuration that still fails after its one documented fix
//! attempt is dropped, never silently removed: it stays named in the manifest
//! and in this log with its cause, the objects and sections that differed, the
//! fix that was tried and the CI run that showed the failure. The log decides
//! what is *retained*, and [`MIN_RETAINED`] retained targets is the floor below
//! which the phase fails.
//!
//! The granularity was chosen by the founder at the D-03 checkpoint
//! ([`DropPolicy`]): with `drop_variant` a variant that fails drops only that
//! variant, with `drop_target` it drops the whole target. A failing `default`
//! configuration always drops the target. A variant that changes no compiled
//! code is dropped as a variant under either policy (D-05).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::manifest::Manifest;

/// The only schema version this crate understands.
pub const DROP_LOG_SCHEMA_VERSION: u32 = 1;

/// The fewest targets that must remain after the drops (CONTEXT D-11).
pub const MIN_RETAINED: usize = 10;

/// What a variant that fails after its fix attempt drops (research Open
/// Question 4, answered at the D-03 checkpoint).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VariantFailure {
    /// Only the failing variant is dropped.
    DropVariant,
    /// The whole target is dropped.
    DropTarget,
}

/// The founder's drop granularity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DropPolicy {
    /// What a failing variant drops.
    pub variant_failure: VariantFailure,
}

/// Why a target or configuration was dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DropCause {
    /// The source could not be fetched or did not match its pin.
    FetchFailed,
    /// The upstream build failed.
    BuildFailed,
    /// A capture gate refused the build.
    CaptureFailed,
    /// The replayed objects differ from the captured ones.
    Differs,
    /// The result could not be established (timeout, missing artifact).
    NotProven,
    /// An entry function was not found as declared.
    EntryMismatch,
    /// A variant that changes no compiled code (CONTEXT D-05).
    VariantNoCodeChange,
    /// A licence problem found after approval.
    Licence,
}

impl DropCause {
    /// The cause as written in `dropped.toml` and in summary rows.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FetchFailed => "fetch_failed",
            Self::BuildFailed => "build_failed",
            Self::CaptureFailed => "capture_failed",
            Self::Differs => "differs",
            Self::NotProven => "not_proven",
            Self::EntryMismatch => "entry_mismatch",
            Self::VariantNoCodeChange => "variant_no_code_change",
            Self::Licence => "licence",
        }
    }
}

/// One dropped target (`config` absent) or one dropped variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dropped {
    /// A target id of the manifest.
    pub target: String,
    /// The dropped variant's name; absent when the whole target is dropped.
    #[serde(default)]
    pub config: Option<String>,
    /// Why.
    pub cause: DropCause,
    /// The cause as the report stated it.
    pub detail: String,
    /// The objects that differed, when the cause is `differs`.
    #[serde(default)]
    pub differing_objects: Vec<String>,
    /// The ELF sections that differed.
    #[serde(default)]
    pub differing_sections: Vec<String>,
    /// The one fix that was tried (CONTEXT D-11), never empty.
    pub fix_attempted: String,
    /// The CI run (testbed workflow run id) that showed the failure.
    pub ci_run: u64,
}

impl Dropped {
    /// The heading of this entry in `DROPPED.md`: `<target>` or
    /// `<target>/<config>`.
    #[must_use]
    pub fn heading(&self) -> String {
        match &self.config {
            None => self.target.clone(),
            Some(config) => format!("{}/{config}", self.target),
        }
    }
}

/// The parsed `testbed/dropped.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DropLog {
    /// [`DROP_LOG_SCHEMA_VERSION`].
    pub schema_version: u32,
    /// The founder's granularity.
    pub policy: DropPolicy,
    /// The entries, in the order they were written.
    #[serde(default)]
    pub dropped: Vec<Dropped>,
}

/// Why a drop log could not be used.
#[derive(Debug, thiserror::Error)]
pub enum DropLogError {
    /// The file could not be read.
    #[error("cannot read {}: {source}", path.display())]
    Read {
        /// The path involved.
        path: PathBuf,
        /// The underlying error.
        #[source]
        source: std::io::Error,
    },
    /// Malformed TOML, an unknown key or a wrong type.
    #[error("invalid drop log: {0}")]
    Parse(#[from] toml::de::Error),
    /// The schema version is not the one this crate reads.
    #[error("unsupported drop log schema_version {found}; expected {expected}")]
    SchemaVersion {
        /// The version in the file.
        found: u32,
        /// The version this crate reads.
        expected: u32,
    },
    /// Values that break the rules, each starting with its location.
    #[error("invalid drop log:\n{}", issues.join("\n"))]
    Invalid {
        /// Every issue found, sorted.
        issues: Vec<String>,
    },
}

impl Default for DropLog {
    /// No entries, the founder's `drop_variant` granularity: what a testbed
    /// directory without a `dropped.toml` means.
    fn default() -> Self {
        Self {
            schema_version: DROP_LOG_SCHEMA_VERSION,
            policy: DropPolicy {
                variant_failure: VariantFailure::DropVariant,
            },
            dropped: Vec::new(),
        }
    }
}

impl DropLog {
    /// Parse a drop log from TOML text (shape only; see [`DropLog::validate`]).
    ///
    /// # Errors
    /// [`DropLogError::Parse`] on malformed TOML, unknown keys or wrong types,
    /// [`DropLogError::SchemaVersion`] on another schema version.
    pub fn parse(text: &str) -> Result<Self, DropLogError> {
        let log: Self = toml::from_str(text)?;
        if log.schema_version != DROP_LOG_SCHEMA_VERSION {
            return Err(DropLogError::SchemaVersion {
                found: log.schema_version,
                expected: DROP_LOG_SCHEMA_VERSION,
            });
        }
        Ok(log)
    }

    /// Read and parse a drop log file.
    ///
    /// # Errors
    /// [`DropLogError::Read`] if the file cannot be read, otherwise as
    /// [`DropLog::parse`].
    pub fn load(path: &Path) -> Result<Self, DropLogError> {
        let text = std::fs::read_to_string(path).map_err(|source| DropLogError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse(&text)
    }

    /// The drop log of a testbed directory: `dropped.toml` there, or the empty
    /// log when the directory has none.
    ///
    /// # Errors
    /// As [`DropLog::load`] when the file exists.
    pub fn load_from_testbed_dir(testbed_dir: &Path) -> Result<Self, DropLogError> {
        let path = testbed_dir.join("dropped.toml");
        if path.is_file() {
            Self::load(&path)
        } else {
            Ok(Self::default())
        }
    }

    /// Check every entry against the manifest. Every issue is collected,
    /// sorted and reported at once.
    ///
    /// # Errors
    /// [`DropLogError::Invalid`] listing every issue.
    pub fn validate(&self, manifest: &Manifest) -> Result<(), DropLogError> {
        let mut issues: Vec<String> = Vec::new();
        let mut seen: BTreeSet<(String, Option<String>)> = BTreeSet::new();
        for (index, entry) in self.dropped.iter().enumerate() {
            let at = format!("dropped[{index}] ({})", entry.heading());
            let Some(target) = manifest.target.get(&entry.target) else {
                issues.push(format!(
                    "{at}.target: {:?} is not a target of the manifest; a dropped target stays named in it",
                    entry.target
                ));
                continue;
            };
            if !seen.insert((entry.target.clone(), entry.config.clone())) {
                issues.push(format!(
                    "{at}: duplicate entry for the same target and configuration"
                ));
            }
            match entry.config.as_deref() {
                Some("default") => issues.push(format!(
                    "{at}.config: the default configuration cannot be dropped alone; drop the target (omit config)"
                )),
                Some(name) => {
                    if target.config.iter().all(|c| c.name != name) {
                        issues.push(format!(
                            "{at}.config: {name:?} is not a configuration of target {:?}",
                            entry.target
                        ));
                    } else if self.policy.variant_failure == VariantFailure::DropTarget
                        && entry.cause != DropCause::VariantNoCodeChange
                    {
                        issues.push(format!(
                            "{at}.config: the policy is drop_target, so a failing variant drops the whole target (only variant_no_code_change drops a variant)"
                        ));
                    }
                }
                None => {
                    if entry.cause == DropCause::VariantNoCodeChange {
                        issues.push(format!(
                            "{at}.cause: variant_no_code_change drops a variant, so config must be named"
                        ));
                    }
                }
            }
            if entry.fix_attempted.trim().is_empty() {
                issues.push(format!(
                    "{at}.fix_attempted: must not be empty; every drop follows one documented fix attempt (D-11)"
                ));
            }
            if entry.detail.trim().is_empty() {
                issues.push(format!("{at}.detail: must not be empty"));
            }
            if entry.ci_run == 0 {
                issues.push(format!("{at}.ci_run: must be the numeric id of a CI run"));
            }
        }
        // A variant entry next to an entry for its whole target says nothing new
        // and hides which of the two is the cause.
        for entry in &self.dropped {
            if entry.config.is_some()
                && self
                    .dropped
                    .iter()
                    .any(|other| other.config.is_none() && other.target == entry.target)
            {
                issues.push(format!(
                    "dropped ({}): the whole target {:?} is already dropped",
                    entry.heading(),
                    entry.target
                ));
            }
        }
        issues.sort();
        issues.dedup();
        if issues.is_empty() {
            Ok(())
        } else {
            Err(DropLogError::Invalid { issues })
        }
    }

    /// Check that `DROPPED.md` has one `## <heading>` per entry under its
    /// `# Entries` section and no other `## ` heading there (D-11). Headings
    /// before `# Entries` explain the policy and are not checked.
    ///
    /// # Errors
    /// [`DropLogError::Invalid`] naming every entry without a heading, every
    /// heading without an entry, and a missing `# Entries` section.
    pub fn check_markdown(&self, markdown: &str) -> Result<(), DropLogError> {
        let mut issues: Vec<String> = Vec::new();
        let mut in_entries = false;
        let mut found_section = false;
        let mut headings: Vec<String> = Vec::new();
        for line in markdown.lines() {
            let line = line.trim_end();
            if line == "# Entries" {
                in_entries = true;
                found_section = true;
            } else if in_entries && line.starts_with("# ") {
                in_entries = false;
            } else if in_entries && let Some(heading) = line.strip_prefix("## ") {
                headings.push(heading.trim().to_owned());
            }
        }
        if !found_section {
            issues.push("DROPPED.md has no `# Entries` section".to_owned());
        }
        let expected: BTreeSet<String> = self.dropped.iter().map(Dropped::heading).collect();
        let present: BTreeSet<String> = headings.iter().cloned().collect();
        if present.len() != headings.len() {
            issues.push("DROPPED.md repeats an entry heading".to_owned());
        }
        for heading in expected.difference(&present) {
            issues.push(format!(
                "DROPPED.md has no `## {heading}` heading for an entry of dropped.toml"
            ));
        }
        for heading in present.difference(&expected) {
            issues.push(format!(
                "DROPPED.md heading `## {heading}` has no entry in dropped.toml"
            ));
        }
        issues.sort();
        issues.dedup();
        if issues.is_empty() {
            Ok(())
        } else {
            Err(DropLogError::Invalid { issues })
        }
    }

    /// The entry that drops the whole target, if any.
    #[must_use]
    pub fn target_dropped(&self, target: &str) -> Option<&Dropped> {
        self.dropped
            .iter()
            .find(|entry| entry.target == target && entry.config.is_none())
    }

    /// The entry that drops this pair: the whole target, or this variant.
    #[must_use]
    pub fn is_dropped(&self, target: &str, config: &str) -> Option<&Dropped> {
        self.target_dropped(target).or_else(|| {
            self.dropped
                .iter()
                .find(|entry| entry.target == target && entry.config.as_deref() == Some(config))
        })
    }

    /// The manifest's targets that are not fully dropped, sorted.
    #[must_use]
    pub fn retained_targets(&self, manifest: &Manifest) -> Vec<String> {
        manifest
            .target
            .keys()
            .filter(|id| self.target_dropped(id).is_none())
            .cloned()
            .collect()
    }

    /// The ids of the fully dropped targets, sorted.
    #[must_use]
    pub fn dropped_targets(&self) -> Vec<String> {
        let mut ids: Vec<String> = self
            .dropped
            .iter()
            .filter(|entry| entry.config.is_none())
            .map(|entry| entry.target.clone())
            .collect();
        ids.sort();
        ids.dedup();
        ids
    }
}
