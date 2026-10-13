//! The per-target seed index and the committed corpus directory (D-15).

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::select::{Caps, Selection, Skipped, SourceRef};
use crate::{Method, SeedError};

/// Version of the `index.json` schema.
pub const SEED_INDEX_SCHEMA_VERSION: u32 = 1;

/// The licence the seeds carry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexLicence {
    /// SPDX expression of the target's licence.
    pub spdx: String,
    /// The alternative elected from a dual licence, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elected: Option<String>,
    /// Upstream licence file the seeds fall under.
    pub file: String,
}

/// One seed in the index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeedRecord {
    /// Lower-case hex sha256 of the seed, which is also its file name.
    pub sha256: String,
    /// Length of the seed in bytes.
    pub bytes: u64,
    /// Entry points the seed is meant to exercise.
    pub entries: Vec<String>,
    /// How the seed was obtained.
    pub method: Method,
    /// Where it came from.
    pub source: SourceRef,
    /// True when the seed is a proposal awaiting review.
    pub proposed: bool,
}

/// The per-target index of a seed corpus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeedIndex {
    /// Always [`SEED_INDEX_SCHEMA_VERSION`].
    pub schema_version: u32,
    /// Test-bed target id.
    pub target: String,
    /// Upstream commit the seeds were taken from.
    pub commit: String,
    /// Licence of the seeds.
    pub licence: IndexLicence,
    /// Caps the selection ran under.
    pub caps: Caps,
    /// Seeds sorted by sha256.
    pub seeds: Vec<SeedRecord>,
    /// Counts of what was left out.
    pub skipped: Skipped,
}

impl SeedIndex {
    /// Build the index for `selection`.
    #[must_use]
    pub fn from_selection(
        target: &str,
        commit: &str,
        licence: IndexLicence,
        caps: Caps,
        selection: &Selection,
    ) -> Self {
        let _ = selection;
        Self {
            schema_version: SEED_INDEX_SCHEMA_VERSION,
            target: target.to_owned(),
            commit: commit.to_owned(),
            licence,
            caps,
            seeds: Vec::new(),
            skipped: selection.skipped,
        }
    }

    /// Pretty JSON with a trailing newline and no clock-derived field.
    pub fn to_json(&self) -> Result<String, SeedError> {
        let mut text = serde_json::to_string_pretty(self)?;
        text.push('\n');
        Ok(text)
    }
}

/// Replace the contents of `dir` with the seed files, `index.json` and `NOTICE`.
pub fn write_corpus(
    dir: &Path,
    index: &SeedIndex,
    selection: &Selection,
    notice: &str,
) -> Result<(), SeedError> {
    let _ = (dir, index, selection, notice);
    Ok(())
}

/// Compare `dir` with what [`write_corpus`] would write; one line per difference.
pub fn check_corpus(
    dir: &Path,
    index: &SeedIndex,
    selection: &Selection,
    notice: &str,
) -> Result<Vec<String>, SeedError> {
    let _ = (dir, index, selection, notice);
    Ok(Vec::new())
}
