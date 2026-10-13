//! Manifest-driven seed extraction and the committed corpus check (CONTEXT D-13,
//! D-15, D-16, D-21).
//!
//! RED stub: the real extraction arrives with the next commit.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use mt_seeds::index::{IndexLicence, SeedIndex};
use mt_seeds::select::{Caps, Selection as SeedSelection};

use crate::manifest::Target;
use crate::testbed::{StepSummary, TestbedError, TestbedOptions};

/// Why the seeds of a target could not be extracted.
#[derive(Debug, thiserror::Error)]
pub enum SeedingError {
    /// A declared seed path is not in the tree.
    #[error("seed source {path:?} is not in the tree")]
    SourceMissing {
        /// The declared path.
        path: String,
    },
    /// A declared seed path goes through a symlink.
    #[error("seed source {path:?} goes through a symlink")]
    SourceSymlink {
        /// The declared path.
        path: String,
    },
}

/// What a seeds run found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedsRun {
    /// One row per target.
    pub summary: StepSummary,
    /// Per target, the difference lines of a check that found drift.
    pub drift: BTreeMap<String, Vec<String>>,
}

/// Extract the seeds of one target.
///
/// # Errors
/// [`SeedingError`] when a declared source cannot be used.
pub fn seed_target(
    id: &str,
    _tree: &Path,
    commit: &str,
    target: &Target,
    _hand_made_dir: &Path,
    caps: Caps,
) -> Result<(SeedIndex, SeedSelection), SeedingError> {
    let selection = SeedSelection::default();
    let licence = IndexLicence {
        spdx: target.licence.spdx.clone(),
        elected: Some(target.licence.elected.clone()),
        file: target.licence.file.clone(),
    };
    let index = SeedIndex::from_selection(id, commit, licence, caps, &selection);
    Ok((index, selection))
}

/// Extract or check the seeds of the selected targets.
///
/// # Errors
/// [`TestbedError`] for unusable inputs.
pub fn run_seeds_detailed(
    _opts: &TestbedOptions,
    _seeds_dir: &Path,
    _check: bool,
) -> Result<SeedsRun, TestbedError> {
    Ok(SeedsRun {
        summary: StepSummary::new("seeds"),
        drift: BTreeMap::new(),
    })
}

/// [`run_seeds_detailed`] without the difference lines.
///
/// # Errors
/// [`TestbedError`] for unusable inputs.
pub fn run_seeds(
    opts: &TestbedOptions,
    seeds_dir: &Path,
    check: bool,
) -> Result<StepSummary, TestbedError> {
    run_seeds_detailed(opts, seeds_dir, check).map(|run| run.summary)
}

/// The hand-made directory of a target under the committed testbed directory.
#[must_use]
pub fn hand_made_dir(testbed_dir: &Path, id: &str) -> PathBuf {
    testbed_dir.join("targets").join(id).join("hand-made")
}
