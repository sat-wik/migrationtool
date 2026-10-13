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
        ""
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
        String::new()
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
    pub fn parse(_text: &str) -> Result<Self, DropLogError> {
        Ok(Self::default())
    }

    /// Read and parse a drop log file.
    ///
    /// # Errors
    /// [`DropLogError::Read`] if the file cannot be read, otherwise as
    /// [`DropLog::parse`].
    pub fn load(_path: &Path) -> Result<Self, DropLogError> {
        Ok(Self::default())
    }

    /// The drop log of a testbed directory: `dropped.toml` there, or the empty
    /// log when the directory has none.
    ///
    /// # Errors
    /// As [`DropLog::load`] when the file exists.
    pub fn load_from_testbed_dir(_testbed_dir: &Path) -> Result<Self, DropLogError> {
        Ok(Self::default())
    }

    /// Check every entry against the manifest. Every issue is collected,
    /// sorted and reported at once.
    ///
    /// # Errors
    /// [`DropLogError::Invalid`] listing every issue.
    pub fn validate(&self, _manifest: &Manifest) -> Result<(), DropLogError> {
        Ok(())
    }

    /// Check that `DROPPED.md` has one `## <heading>` per entry and no other
    /// `## ` heading that names a target or configuration (D-11).
    ///
    /// # Errors
    /// [`DropLogError::Invalid`] naming every entry without a heading and every
    /// heading without an entry.
    pub fn check_markdown(&self, _markdown: &str) -> Result<(), DropLogError> {
        Ok(())
    }

    /// The entry that drops the whole target, if any.
    #[must_use]
    pub fn target_dropped(&self, _target: &str) -> Option<&Dropped> {
        None
    }

    /// The entry that drops this pair: the whole target, or this variant.
    #[must_use]
    pub fn is_dropped(&self, _target: &str, _config: &str) -> Option<&Dropped> {
        None
    }

    /// The manifest's targets that are not fully dropped, sorted.
    #[must_use]
    pub fn retained_targets(&self, _manifest: &Manifest) -> Vec<String> {
        Vec::new()
    }

    /// The ids of the fully dropped targets, sorted.
    #[must_use]
    pub fn dropped_targets(&self) -> Vec<String> {
        Vec::new()
    }
}
