//! The adapter drop log and the floor of drivable targets (CONTEXT D-13, D-22,
//! D-24, D-28).
//!
//! Every retained target's entries are attempted. An entry the roles cannot drive
//! is logged here, in `testbed/adapter-dropped.toml`, with its cause, a detail and
//! the attempt that was made, like Phase 2's drop log: it is never removed from the
//! manifest. [`drivable_targets`] counts the targets that can produce reference
//! vectors from the committed files alone, and [`check_adapter_floor`] requires
//! at least [`ADAPTER_FLOOR`] of them. No option lowers the floor.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use mt_capture::droplog::DropLog;
use mt_capture::manifest::Manifest;
use mt_seeds::index::SeedIndex;
use serde::{Deserialize, Serialize};

use crate::HarnessError;

/// The only schema version this module reads.
pub const ADAPTER_DROP_SCHEMA_VERSION: u32 = 1;

/// The fewest drivable targets (the phase's success line; equal to Phase 2's
/// `MIN_RETAINED`, which a test ties).
pub const ADAPTER_FLOOR: usize = 10;

/// The `detail` of a `not_attempted` row.
pub const NOT_ATTEMPTED_DETAIL: &str = "phase budget (D-24)";

/// The `attempt` of a `not_attempted` row.
pub const NOT_ATTEMPTED_ATTEMPT: &str = "none: stopped at the phase budget before the attempt";

/// The `attempt` of a `no_seeds` row.
pub const NO_SEEDS_ATTEMPT: &str = "none: the seed corpus lists no seed for this entry (D-28)";

/// Why an entry is not drivable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterDropCause {
    /// The seed corpus lists no seed for the entry (D-28); seeds are never borrowed.
    NoSeeds,
    /// The roles cannot express the entry; the detail names the missing form.
    NotExpressible,
    /// The driver does not link against the captured objects.
    LinkUnresolved,
    /// The driver was built and run, and a check failed in the pinned image.
    AttemptFailed,
    /// Two runs of the entry gave different vectors.
    Nondeterministic,
    /// The entry was left unattempted at a phase budget stop (D-24).
    NotAttempted,
}

impl AdapterDropCause {
    /// The cause as written in the log and in listings.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoSeeds => "no_seeds",
            Self::NotExpressible => "not_expressible",
            Self::LinkUnresolved => "link_unresolved",
            Self::AttemptFailed => "attempt_failed",
            Self::Nondeterministic => "nondeterministic",
            Self::NotAttempted => "not_attempted",
        }
    }
}

/// One entry that is not drivable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdapterDrop {
    /// A target id of the manifest.
    pub target: String,
    /// An entry function of that target.
    pub entry: String,
    /// Why.
    pub cause: AdapterDropCause,
    /// The cause as the attempt stated it.
    pub detail: String,
    /// The attempt that was made; never empty (`none: ...` when none was).
    pub attempt: String,
    /// The testbed run that showed the failure, when there was one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ci_run: Option<u64>,
}

/// The parsed `testbed/adapter-dropped.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdapterDropLog {
    /// [`ADAPTER_DROP_SCHEMA_VERSION`].
    pub schema_version: u32,
    /// The rows, in the order they were written.
    #[serde(default)]
    pub dropped: Vec<AdapterDrop>,
}

impl Default for AdapterDropLog {
    /// No row: what a testbed directory without the file means.
    fn default() -> Self {
        Self {
            schema_version: ADAPTER_DROP_SCHEMA_VERSION,
            dropped: Vec::new(),
        }
    }
}

/// Why an adapter drop log could not be used.
#[derive(Debug, thiserror::Error)]
pub enum AdapterDropError {
    /// The file could not be read.
    #[error("cannot read {}: {source}", path.display())]
    Read {
        /// The path involved.
        path: PathBuf,
        /// The underlying error.
        #[source]
        source: std::io::Error,
    },
    /// Malformed TOML, an unknown key, an unknown cause or a wrong type.
    #[error("invalid adapter drop log: {0}")]
    Parse(#[from] toml::de::Error),
    /// The schema version is not the one this module reads.
    #[error("unsupported adapter drop log schema_version {found}; expected {expected}")]
    SchemaVersion {
        /// The version in the file.
        found: u32,
        /// The version this module reads.
        expected: u32,
    },
    /// Values that break the rules, each starting with its location.
    #[error("invalid adapter drop log:\n{}", issues.join("\n"))]
    Invalid {
        /// Every issue found, sorted.
        issues: Vec<String>,
    },
}

impl AdapterDropLog {
    /// Parse a log from TOML text (shape only; see [`AdapterDropLog::validate`]).
    ///
    /// # Errors
    /// [`AdapterDropError::Parse`] and [`AdapterDropError::SchemaVersion`].
    pub fn parse(text: &str) -> Result<Self, AdapterDropError> {
        let log: Self = toml::from_str(text)?;
        if log.schema_version != ADAPTER_DROP_SCHEMA_VERSION {
            return Err(AdapterDropError::SchemaVersion {
                found: log.schema_version,
                expected: ADAPTER_DROP_SCHEMA_VERSION,
            });
        }
        Ok(log)
    }

    /// Read and parse a log file.
    ///
    /// # Errors
    /// [`AdapterDropError::Read`], otherwise as [`AdapterDropLog::parse`].
    pub fn load(path: &Path) -> Result<Self, AdapterDropError> {
        let text = std::fs::read_to_string(path).map_err(|source| AdapterDropError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse(&text)
    }

    /// The log of a testbed directory: `adapter-dropped.toml` there, or the empty
    /// log when there is none.
    ///
    /// # Errors
    /// As [`AdapterDropLog::load`] when the file exists.
    pub fn load_from_testbed_dir(testbed_dir: &Path) -> Result<Self, AdapterDropError> {
        let path = testbed_dir.join("adapter-dropped.toml");
        if path.is_file() {
            Self::load(&path)
        } else {
            Ok(Self::default())
        }
    }

    /// The row for `entry` of `target`, if it is logged.
    #[must_use]
    pub fn row(&self, target: &str, entry: &str) -> Option<&AdapterDrop> {
        self.dropped
            .iter()
            .find(|row| row.target == target && row.entry == entry)
    }

    /// Check every row against the manifest: the target and entry exist, the
    /// detail and attempt are not blank, no entry is logged twice, and the two
    /// causes with fixed texts carry them. Every issue is collected and sorted.
    ///
    /// # Errors
    /// [`AdapterDropError::Invalid`] listing every issue.
    pub fn validate(&self, manifest: &Manifest) -> Result<(), AdapterDropError> {
        let mut issues = Vec::new();
        let mut seen: BTreeSet<(&str, &str)> = BTreeSet::new();
        for (index, row) in self.dropped.iter().enumerate() {
            let at = format!("dropped[{index}] ({}/{})", row.target, row.entry);
            let Some(target) = manifest.target.get(&row.target) else {
                issues.push(format!(
                    "{at}.target: {:?} is not a target of the manifest",
                    row.target
                ));
                continue;
            };
            if !target.entry.iter().any(|entry| entry.name == row.entry) {
                issues.push(format!(
                    "{at}.entry: {:?} is not an entry of target {:?}",
                    row.entry, row.target
                ));
            }
            if !seen.insert((&row.target, &row.entry)) {
                issues.push(format!("{at}: the entry is logged twice"));
            }
            if row.detail.trim().is_empty() {
                issues.push(format!("{at}.detail: must not be empty"));
            }
            if row.attempt.trim().is_empty() {
                issues.push(format!(
                    "{at}.attempt: must not be empty; write `none: ...` when no attempt was made"
                ));
            }
            match row.cause {
                AdapterDropCause::NotAttempted
                    if row.detail != NOT_ATTEMPTED_DETAIL
                        || row.attempt != NOT_ATTEMPTED_ATTEMPT =>
                {
                    issues.push(format!(
                        "{at}: a not_attempted row has the detail {NOT_ATTEMPTED_DETAIL:?} and the attempt {NOT_ATTEMPTED_ATTEMPT:?} (D-24)"
                    ));
                }
                AdapterDropCause::NoSeeds if row.attempt != NO_SEEDS_ATTEMPT => {
                    issues.push(format!(
                        "{at}: a no_seeds row has the attempt {NO_SEEDS_ATTEMPT:?} (D-28)"
                    ));
                }
                _ => {}
            }
        }
        issues.sort();
        issues.dedup();
        if issues.is_empty() {
            Ok(())
        } else {
            Err(AdapterDropError::Invalid { issues })
        }
    }

    /// An entry is in the log or has roles, never both (D-13). Rows that name
    /// an entry the manifest does not have are left to [`AdapterDropLog::validate`].
    ///
    /// # Errors
    /// [`AdapterDropError::Invalid`] naming every logged entry that has roles.
    pub fn check_consistency(&self, manifest: &Manifest) -> Result<(), AdapterDropError> {
        let mut issues = Vec::new();
        for row in &self.dropped {
            let has_roles = manifest
                .target
                .get(&row.target)
                .and_then(|target| target.entry.iter().find(|entry| entry.name == row.entry))
                .is_some_and(|entry| !entry.arg.is_empty());
            if has_roles {
                issues.push(format!(
                    "{}/{} is logged ({}) and has roles; an entry is in the log or has roles, never both",
                    row.target,
                    row.entry,
                    row.cause.as_str()
                ));
            }
        }
        issues.sort();
        if issues.is_empty() {
            Ok(())
        } else {
            Err(AdapterDropError::Invalid { issues })
        }
    }
}

/// How many committed seeds of `target` list `entry`; 0 when the target has no
/// readable index (fail closed). Seeds are never borrowed from another entry.
fn seed_count(seeds_dir: &Path, target: &str, entry: &str) -> usize {
    std::fs::read(seeds_dir.join(target).join("index.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<SeedIndex>(&bytes).ok())
        .map_or(0, |index| {
            index
                .seeds
                .iter()
                .filter(|seed| seed.entries.iter().any(|name| name == entry))
                .count()
        })
}

/// The targets that can produce reference vectors from the committed files: those
/// the Phase 2 drop log retains that have at least one entry with roles, at least
/// one seed whose index lists that entry, and no adapter drop row for it.
#[must_use]
pub fn drivable_targets(
    manifest: &Manifest,
    seeds_dir: &Path,
    phase2_drops: &DropLog,
    adapter_drops: &AdapterDropLog,
) -> BTreeSet<String> {
    phase2_drops
        .retained_targets(manifest)
        .into_iter()
        .filter(|id| {
            manifest.target.get(id).is_some_and(|target| {
                target.entry.iter().any(|entry| {
                    !entry.arg.is_empty()
                        && adapter_drops.row(id, &entry.name).is_none()
                        && seed_count(seeds_dir, id, &entry.name) > 0
                })
            })
        })
        .collect()
}

/// The floor was not reached.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "{count} drivable targets, below the floor of {floor}: {}",
    targets.join(", ")
)]
pub struct AdapterFloorError {
    /// The drivable targets found.
    pub count: usize,
    /// [`ADAPTER_FLOOR`].
    pub floor: usize,
    /// Their ids.
    pub targets: Vec<String>,
}

/// The number of drivable targets, when it is at least [`ADAPTER_FLOOR`].
///
/// # Errors
/// [`AdapterFloorError`] naming the count and the floor below it.
pub fn check_adapter_floor(
    manifest: &Manifest,
    seeds_dir: &Path,
    phase2_drops: &DropLog,
    adapter_drops: &AdapterDropLog,
) -> Result<usize, AdapterFloorError> {
    let drivable = drivable_targets(manifest, seeds_dir, phase2_drops, adapter_drops);
    if drivable.len() < ADAPTER_FLOOR {
        return Err(AdapterFloorError {
            count: drivable.len(),
            floor: ADAPTER_FLOOR,
            targets: drivable.into_iter().collect(),
        });
    }
    Ok(drivable.len())
}

/// The drivable targets of the testbed next to `manifest_path`: its manifest, seed
/// indexes, Phase 2 drop log and adapter drop log. The adapter drop log is
/// validated against the manifest first.
///
/// # Errors
/// [`HarnessError::Input`] when a file cannot be read or a log is invalid.
pub fn testbed_drivable_targets(manifest_path: &Path) -> Result<BTreeSet<String>, HarnessError> {
    let input = |what: &Path, detail: String| HarnessError::Input {
        what: what.display().to_string(),
        detail,
    };
    let manifest =
        Manifest::load(manifest_path).map_err(|e| input(manifest_path, e.to_string()))?;
    manifest
        .validate()
        .map_err(|e| input(manifest_path, e.to_string()))?;
    let testbed = match manifest_path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    let phase2 = DropLog::load_from_testbed_dir(testbed)
        .map_err(|e| input(&testbed.join("dropped.toml"), e.to_string()))?;
    let adapter = AdapterDropLog::load_from_testbed_dir(testbed)
        .map_err(|e| input(&testbed.join("adapter-dropped.toml"), e.to_string()))?;
    adapter
        .validate(&manifest)
        .map_err(|e| input(&testbed.join("adapter-dropped.toml"), e.to_string()))?;
    Ok(drivable_targets(
        &manifest,
        &testbed.join("seeds"),
        &phase2,
        &adapter,
    ))
}
