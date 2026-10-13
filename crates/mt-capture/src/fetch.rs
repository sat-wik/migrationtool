//! Fetch a test-bed source by commit or by archive, with a verified cache
//! (CONTEXT D-17, D-18, D-19).
//!
//! RED stub: the types and paths exist, the behaviour does not yet.

use std::path::{Path, PathBuf};
use std::time::Duration;

use mt_toolchain::pins::Pins;
use mt_toolchain::runner::{RunnerConfig, RunnerError};

use crate::manifest::Target;
use crate::report::FetchReport;
use crate::treehash::TreeHashError;

/// The tar in the Debian base image.
pub const TAR_PATH: &str = "/usr/bin/tar";

/// Everything a fetch needs besides the runner configuration.
#[derive(Debug, Clone)]
pub struct FetchContext {
    /// The work directory (`.mt-work` or `$MT_WORK_DIR`; `/work` in the image).
    pub work_dir: PathBuf,
    /// The pinned git.
    pub git: PathBuf,
    /// The pinned curl.
    pub curl: PathBuf,
    /// tar.
    pub tar: PathBuf,
    /// Value of `GIT_ALLOW_PROTOCOL`.
    pub allowed_protocols: String,
    /// Wall-clock limit of each step.
    pub timeout: Duration,
}

/// Where a fetched source lives inside the work directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachePaths {
    /// `<work>/cache/<id>/<commit>`.
    pub dir: PathBuf,
    /// `<dir>/tree`.
    pub tree: PathBuf,
    /// `<dir>/archive.tar.bz2`.
    pub archive: PathBuf,
    /// `<dir>/fetch.json`.
    pub report: PathBuf,
}

/// Why a source could not be fetched or trusted.
#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    /// Not implemented yet.
    #[error("not implemented")]
    NotImplemented,
    /// An identifier, URL or commit that cannot be used safely.
    #[error("invalid fetch input: {0}")]
    InvalidInput(String),
    /// A step exited non-zero.
    #[error("{step} failed (exit code {exit_code:?}): {stderr}")]
    Failed {
        /// The step name.
        step: String,
        /// Its exit code.
        exit_code: Option<i32>,
        /// The start of its standard error.
        stderr: String,
    },
    /// The checked-out commit is not the pinned one.
    #[error("the fetched commit {found} is not the pinned commit {expected}")]
    CommitMismatch {
        /// The pin.
        expected: String,
        /// What `git rev-parse HEAD` printed.
        found: String,
    },
    /// The tree hash is not the pinned one.
    #[error("the tree sha256 is {found}, the manifest pins {expected}")]
    TreeHashMismatch {
        /// The pin.
        expected: String,
        /// The computed digest.
        found: String,
    },
    /// The archive digest is not the pinned one.
    #[error("the archive sha256 is {found}, the manifest pins {expected}")]
    ArchiveMismatch {
        /// The pin.
        expected: String,
        /// The computed digest.
        found: String,
    },
    /// The cache no longer matches its pin.
    #[error("the cache at {} does not match its pin: {detail}", path.display())]
    CacheMismatch {
        /// The cached tree or archive.
        path: PathBuf,
        /// What differs.
        detail: String,
    },
    /// The runner could not start or supervise a tool.
    #[error(transparent)]
    Runner(#[from] RunnerError),
    /// A tree could not be hashed or copied.
    #[error(transparent)]
    Tree(#[from] TreeHashError),
}

impl FetchContext {
    /// The context for `work_dir` with git and curl taken from the pins.
    ///
    /// # Errors
    /// [`FetchError`] when a tool is missing from the pins.
    pub fn from_pins(_pins: &Pins, work_dir: &Path) -> Result<Self, FetchError> {
        Ok(Self {
            work_dir: work_dir.to_path_buf(),
            git: PathBuf::new(),
            curl: PathBuf::new(),
            tar: PathBuf::from(TAR_PATH),
            allowed_protocols: "https".to_owned(),
            timeout: Duration::from_secs(600),
        })
    }
}

/// The cache paths of a source.
#[must_use]
pub fn cache_paths(work_dir: &Path, id: &str, commit: &str) -> CachePaths {
    let dir = work_dir.join("cache").join(id).join(commit);
    CachePaths {
        tree: dir.join("tree"),
        archive: dir.join("archive.tar.bz2"),
        report: dir.join("fetch.json"),
        dir,
    }
}

/// Fetch the source of `target` into the cache and verify it.
///
/// # Errors
/// [`FetchError`].
pub fn fetch(
    _cfg: &RunnerConfig,
    _ctx: &FetchContext,
    _id: &str,
    _target: &Target,
) -> Result<FetchReport, FetchError> {
    Err(FetchError::NotImplemented)
}

/// Verify a cached source without the network.
///
/// # Errors
/// [`FetchError`].
pub fn verify_cached(
    _cfg: &RunnerConfig,
    _ctx: &FetchContext,
    _id: &str,
    _target: &Target,
) -> Result<FetchReport, FetchError> {
    Err(FetchError::NotImplemented)
}
