//! Clean-copy replay of captured compile commands (BUILD-02, CONTEXT D-26).

use std::collections::BTreeMap;
use std::path::PathBuf;

use mt_toolchain::runner::{RunnerConfig, RunnerError};

use crate::cc::{CcEntry, CcError, CcPolicy};
use crate::report::{NORMALISATION_RULE, REPORT_SCHEMA_VERSION, ReplayReport, Verdict};
use crate::treehash::TreeHashError;

/// Everything a replay needs, with no hidden state.
#[derive(Debug, Clone)]
pub struct ReplayPlan {
    /// Test-bed target id, recorded in the report.
    pub target: String,
    /// Configuration name, recorded in the report.
    pub config: String,
    /// The run root. Every directory and output lies under it.
    pub run_root: PathBuf,
    /// The pristine source tree, restored into `run_root/<src_dir_name>`.
    pub pristine_src: PathBuf,
    /// Name of the source directory below the run root (`src`).
    pub src_dir_name: String,
    /// Directories below the run root that survive the replay (a CMake build
    /// directory); only declared outputs inside them are removed.
    pub keep_dirs: Vec<PathBuf>,
    /// The canonical entries, in canonical order.
    pub entries: Vec<CcEntry>,
    /// sha256 of every original object, keyed by run-root-relative path.
    pub original_objects: BTreeMap<String, String>,
    /// Directory holding the original objects under the same relative keys.
    pub original_objects_dir: PathBuf,
    /// The allow-list and run root checked before anything is spawned.
    pub policy: CcPolicy,
    /// Tool name recorded in the run records (`arm_gcc` in the pipeline, `cc` in tests).
    pub compiler_tool_name: String,
    /// The normaliser, when the fixed normalisation may be applied.
    pub normaliser: Option<Normaliser>,
}

/// The tool and scratch space the fixed normalisation uses.
#[derive(Debug, Clone)]
pub struct Normaliser {
    /// The `objcopy` that matches the compiler.
    pub objcopy: PathBuf,
    /// Writable scratch directory for the normalised copies.
    pub scratch: PathBuf,
}

/// Why a replay could not be attempted. A target-configuration failure is data
/// in the report, never an error.
#[derive(Debug, thiserror::Error)]
pub enum ReplayError {
    /// An entry broke the compiler allow-list or the run-root containment.
    #[error(transparent)]
    Entry(#[from] CcError),
    /// The plan contradicts itself.
    #[error("invalid replay plan: {0}")]
    InvalidPlan(String),
    /// A subprocess could not be started or supervised.
    #[error(transparent)]
    Runner(#[from] RunnerError),
    /// The pristine tree could not be copied.
    #[error(transparent)]
    Tree(#[from] TreeHashError),
    /// A file operation failed.
    #[error("cannot {action} {}: {source}", path.display())]
    Io {
        /// What was attempted.
        action: &'static str,
        /// The path involved.
        path: PathBuf,
        /// The underlying error.
        #[source]
        source: std::io::Error,
    },
}

/// Replay `plan` and report.
///
/// # Errors
/// [`ReplayError`] only for invalid inputs; see the type.
pub fn replay(_cfg: &RunnerConfig, plan: &ReplayPlan) -> Result<ReplayReport, ReplayError> {
    Ok(ReplayReport {
        schema_version: REPORT_SCHEMA_VERSION,
        kind: "replay".to_owned(),
        target: plan.target.clone(),
        config: plan.config.clone(),
        verdict: Verdict::BuildFailed,
        normalisation_rule: NORMALISATION_RULE.to_owned(),
        normalisation_applied: false,
        objects: BTreeMap::new(),
        steps: Vec::new(),
        detail: Some("not implemented".to_owned()),
    })
}
