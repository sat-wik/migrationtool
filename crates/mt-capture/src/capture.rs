//! Make and CMake capture drivers with the silent-wrong-build gates (BUILD-01).
//!
//! RED stub: the public shape exists, the behaviour does not yet.

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use mt_toolchain::pins::Pins;
use mt_toolchain::runner::{RunnerConfig, RunnerError};
use serde::{Deserialize, Serialize};

use crate::cc::CcEntry;
use crate::fetch::FetchError;
use crate::manifest::{BuildArgs, Config, Target};
use crate::report::CaptureReport;
use crate::treehash::TreeHashError;

/// The absolute paths of the tools a capture runs.
#[derive(Debug, Clone)]
pub struct ToolPaths {
    /// The pinned `arm-none-eabi-gcc`; also the compiler allow-list.
    pub arm_gcc: PathBuf,
    /// Bear.
    pub bear: PathBuf,
    /// CMake.
    pub cmake: PathBuf,
    /// GNU Make.
    pub make: PathBuf,
    /// clang, for the entry check.
    pub clang: PathBuf,
    /// `arm-none-eabi-readelf`.
    pub readelf: PathBuf,
    /// `arm-none-eabi-objcopy`.
    pub objcopy: PathBuf,
    /// The `arm-none-eabi` sysroot of the Arm toolchain.
    pub sysroot: PathBuf,
    /// git, to re-verify a cached source.
    pub git: PathBuf,
    /// Pinned versions by tool key.
    pub versions: BTreeMap<String, String>,
}

/// The directories of one target configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunLayout {
    /// `<work>/run/<id>/<config>`.
    pub run_root: PathBuf,
    /// `<run_root>/src`.
    pub src: PathBuf,
    /// `<run_root>/build`.
    pub build: PathBuf,
    /// `<run_root>/inputs`.
    pub inputs: PathBuf,
    /// `<work>/capture/<id>/<config>`.
    pub capture_dir: PathBuf,
    /// `<capture_dir>/objects`.
    pub objects_dir: PathBuf,
}

/// Everything a capture needs besides the runner configuration.
#[derive(Debug, Clone)]
pub struct CaptureContext {
    /// The work directory.
    pub work_dir: PathBuf,
    /// The committed `testbed/` directory.
    pub testbed_dir: PathBuf,
    /// The tools.
    pub tools: ToolPaths,
    /// The baseline flags of the manifest.
    pub baseline: Vec<String>,
    /// Wall-clock limit of each step: 900 s.
    pub timeout: Duration,
}

/// The values the build-string placeholders take.
#[derive(Debug, Clone, Copy)]
pub struct Expansion<'a> {
    /// `{cc}`.
    pub cc: &'a str,
    /// `{cflags}`.
    pub cflags: &'a str,
    /// `{src}`.
    pub src: &'a str,
    /// `{inputs}`.
    pub inputs: &'a str,
}

/// The comparison of a variant with its default configuration (BED-02).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariantDiff {
    /// Objects present in both, by run-relative path.
    pub common_objects: Vec<String>,
    /// Common objects whose code digests differ.
    pub code_differs: Vec<String>,
    /// Objects only the default builds.
    pub only_default: Vec<String>,
    /// Objects only the variant builds.
    pub only_variant: Vec<String>,
    /// True when the variant compiles different code.
    pub changes_code: bool,
}

/// Why a capture could not be attempted. A target-configuration failure is data
/// in the report, never an error.
#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    /// Not implemented yet.
    #[error("not implemented")]
    NotImplemented,
    /// The inputs contradict themselves or are unsafe.
    #[error("invalid capture input: {0}")]
    InvalidInput(String),
    /// A runner failure.
    #[error(transparent)]
    Runner(#[from] RunnerError),
    /// The cached source could not be verified.
    #[error(transparent)]
    Fetch(#[from] FetchError),
    /// A tree could not be hashed or copied.
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
        source: io::Error,
    },
}

impl ToolPaths {
    /// The tools of the pins.
    ///
    /// # Errors
    /// [`CaptureError`] when a tool is not an installed pin.
    pub fn from_pins(_pins: &Pins) -> Result<Self, CaptureError> {
        Err(CaptureError::NotImplemented)
    }
}

impl RunLayout {
    /// The layout of `<work>/run/<id>/<config>` and `<work>/capture/<id>/<config>`.
    #[must_use]
    pub fn new(work_dir: &Path, _id: &str, _config: &str) -> Self {
        Self {
            run_root: work_dir.to_path_buf(),
            src: work_dir.to_path_buf(),
            build: work_dir.to_path_buf(),
            inputs: work_dir.to_path_buf(),
            capture_dir: work_dir.to_path_buf(),
            objects_dir: work_dir.to_path_buf(),
        }
    }
}

/// Baseline, then the defines, then the configuration's flags.
#[must_use]
pub fn effective_cflags(_baseline: &[String], _config: &Config) -> Vec<String> {
    Vec::new()
}

/// Expand the four placeholders.
///
/// # Errors
/// [`CaptureError::InvalidInput`] for any other brace text.
pub fn expand(_template: &str, _values: &Expansion<'_>) -> Result<String, CaptureError> {
    Err(CaptureError::NotImplemented)
}

/// The Make capture command line.
///
/// # Errors
/// [`CaptureError`] for an unknown placeholder or an unsafe path.
pub fn make_argv(
    _tools: &ToolPaths,
    _layout: &RunLayout,
    _cflags: &[String],
    _args: &BuildArgs,
    _config: &Config,
) -> Result<Vec<String>, CaptureError> {
    Err(CaptureError::NotImplemented)
}

/// The CMake toolchain file text.
///
/// # Errors
/// [`CaptureError`] for a flag that cannot be written safely.
pub fn toolchain_file(_tools: &ToolPaths, _cflags: &[String]) -> Result<String, CaptureError> {
    Err(CaptureError::NotImplemented)
}

/// The CMake configure command line.
///
/// # Errors
/// [`CaptureError`] for a forbidden option or an unknown placeholder.
pub fn cmake_configure_argv(
    _tools: &ToolPaths,
    _layout: &RunLayout,
    _cflags: &[String],
    _args: &BuildArgs,
    _config: &Config,
) -> Result<Vec<String>, CaptureError> {
    Err(CaptureError::NotImplemented)
}

/// The CMake build command line.
///
/// # Errors
/// [`CaptureError`] when no CMake target is named.
pub fn cmake_build_argv(
    _tools: &ToolPaths,
    _layout: &RunLayout,
    _args: &BuildArgs,
    _config: &Config,
) -> Result<Vec<String>, CaptureError> {
    Err(CaptureError::NotImplemented)
}

/// Every file under `root`, as sorted relative paths.
///
/// # Errors
/// [`CaptureError::Io`].
pub fn list_files(_root: &Path) -> Result<BTreeSet<String>, CaptureError> {
    Ok(BTreeSet::new())
}

/// Objects created by the build that no command declares.
#[must_use]
pub fn undeclared_objects(
    _before: &BTreeSet<String>,
    _after: &BTreeSet<String>,
    _declared: &BTreeSet<String>,
) -> Vec<String> {
    Vec::new()
}

/// Captured sources that use `__TIMESTAMP__`.
///
/// # Errors
/// [`CaptureError::Io`] for a source that cannot be read.
pub fn timestamp_macro_sources(_entries: &[CcEntry]) -> Result<Vec<String>, CaptureError> {
    Ok(Vec::new())
}

/// Capture one configuration of one target.
///
/// # Errors
/// [`CaptureError`].
pub fn capture(
    _cfg: &RunnerConfig,
    _ctx: &CaptureContext,
    _id: &str,
    _target: &Target,
    _config: &Config,
) -> Result<CaptureReport, CaptureError> {
    Err(CaptureError::NotImplemented)
}

/// Compare the code of a variant with its default configuration.
#[must_use]
pub fn variant_code_diff(_default: &CaptureReport, _variant: &CaptureReport) -> VariantDiff {
    VariantDiff::default()
}
