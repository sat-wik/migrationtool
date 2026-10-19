//! Make and CMake capture drivers with the silent-wrong-build gates (BUILD-01,
//! CONTEXT D-05, D-06, D-07, D-22).
//!
//! A capture builds one configuration of one fetched target with the project's
//! own build system and records what really happened:
//!
//! - Make runs under Bear, `bear --output <raw> -- make -j1 <vars> <goals>`. The
//!   baseline flags travel in `CC` (`CC={cc} {cflags}`), so the Makefile keeps its
//!   own `CFLAGS` (RESEARCH Pitfall 6).
//! - CMake gets a per-configuration toolchain file whose `CMAKE_C_FLAGS_INIT`
//!   holds the baseline and the configuration's flags, and is configured with
//!   `CMAKE_EXPORT_COMPILE_COMMANDS=ON` and `CMAKE_BUILD_TYPE=MinSizeRel`. It is
//!   never given `-DCMAKE_C_FLAGS`, which would silently replace the toolchain
//!   flags (Pitfalls 1 and 3).
//!
//! The export is then canonicalised and gated: the compiler must be the pinned
//! Arm compiler, every object must be an ELF32 Cortex-M4 hard-float object,
//! every object file the build created must be declared by a compile command
//! (Bear fails silently, Pitfall 7), and no captured source may use
//! `__TIMESTAMP__`, which depends on a file's modification time and so cannot be
//! reproduced. A failed gate is a verdict and a cause in the report, never a
//! pass. Every subprocess runs through the `mt-toolchain` runner.
//!
//! # Two stages and the trust model (review warning WR-04)
//!
//! A capture is a build stage followed by a record stage, and [`capture`] is
//! exactly the two in one process.
//!
//! - [`capture_build`] runs the untrusted upstream build (Makefiles and CMake
//!   scripts). It writes only inside its pair's run directory
//!   (`<work>/run/<id>/<config>`): the source copy, the committed inputs, the
//!   build tree, and under `stage/` the exporter's raw file, the CMake toolchain
//!   file and the build record ([`BuildRecord`]). It never creates, reads or
//!   writes `<work>/capture`, the reports or the output directory.
//! - [`capture_record`] runs no upstream code and alone writes the artifacts that
//!   replay and the determinism step trust (`<work>/capture/<id>/<config>`:
//!   `capture.json`, the canonical compile commands, the object copies).
//!
//! The record stage trusts nothing the build could have changed except the
//! build's own outputs, which it reads as untrusted data and checks exactly as a
//! single-process capture always did. It verifies the source cache again, takes
//! the tools from the pins, hashes the committed inputs from `testbed/` and
//! refuses a run directory whose inputs copy differs (`inputs_changed`), and
//! recomputes the toolchain file from the pins and the configuration. It reads the
//! run directory only through [`read_run_file`], which never follows a symlink:
//! the build record, the raw export, every object, every configure-time file and
//! every file of the inputs copy. A refused build record is `not_proven` with the
//! cause `no_build_record`; any other refused read is `capture_failed` with the
//! cause `run_dir_symlink`.
//!
//! Not covered by that rule, as before: the compiler's own reads during the entry
//! check, the defining-TU read of the include prefix (entries.rs), the
//! `__TIMESTAMP__` scan of the sources (it reads, never copies, and a source tree
//! may legitimately hold symlinks), and the readelf runs on objects the record
//! stage has already opened as regular files.
//!
//! The record stage takes from the build record only what describes the build
//! itself: the build steps' run records, the `not_proven` and `build_failed`
//! flags, and the `before` listing. Those can be falsified only by the same
//! pair's own build, which could equally have built different objects, so they
//! cannot alter another pair's evidence, the source verification, the tools, the
//! inputs check or the replay comparison. A forged record can therefore make its
//! own pair read as built, failed or not proven, and nothing else.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use mt_toolchain::pins::Pins;
use mt_toolchain::runner::{RunRequest, RunnerConfig, RunnerError, run};
use serde::{Deserialize, Serialize};

use crate::cc::{CcEntry, CcError, CcPolicy, canonicalise, parse_raw, to_json};
use crate::elf::{check_elf_header, code_digest, parse_readelf_a};
use crate::entries::{EntryError, check_entry};
use crate::fetch::{
    FetchContext, FetchError, TAR_PATH, cache_paths, is_safe_component, pinned_tool, verify_cached,
};
use crate::manifest::{BuildArgs, BuildSystem, Config, PLACEHOLDERS, Target};
use crate::replay::relative_key;
use crate::report::{
    CaptureReport, CaptureVerdict, CapturedObject, CompileCommandsInfo, FetchReport,
    NormalisedFile, SourceRef, Step,
};
use crate::treehash::{TreeHashError, copy_tree, sha256_hex};

/// The most a single step may run before the capture reports `not_proven`.
const STEP_TIMEOUT: Duration = Duration::from_secs(900);

/// The directory of the run root that holds the build stage's own files.
const STAGE_DIR: &str = "stage";
/// The build record inside [`STAGE_DIR`].
const BUILD_RECORD_FILE: &str = "build-record.json";
/// The exporter's file, in the stage directory and again in the capture directory.
const RAW_EXPORT_FILE: &str = "compile_commands.raw.json";
/// The CMake toolchain file, in the stage directory and again in the capture directory.
const TOOLCHAIN_FILE: &str = "toolchain.cmake";

/// Version of the serialised [`BuildRecord`].
pub const BUILD_RECORD_SCHEMA_VERSION: u32 = 1;

/// The tool keys whose pinned versions a capture report records.
const REPORTED_TOOLS: [&str; 5] = ["arm_gcc", "bear", "clang", "cmake", "make"];

/// CMake's own diagnostic logs: they hold scratch-directory names that change
/// from run to run and are not an input of any compile command, so they are not
/// hashed into the report.
const CONFIGURE_LOGS: [&str; 3] = [
    "CMakeFiles/CMakeConfigureLog.yaml",
    "CMakeFiles/CMakeOutput.log",
    "CMakeFiles/CMakeError.log",
];

/// The make driver CMake generates at the top of the build directory. Its bytes
/// are not stable, so it is hashed after [`normalise_make_driver`] instead of
/// byte for byte (gap G-02-2).
const MAKE_DRIVER: &str = "CMakeFiles/Makefile2";

/// The rule [`normalise_make_driver`] applies, recorded next to the hash in the
/// capture report.
pub const MAKE_DRIVER_RULE: &str = "each run of consecutive prerequisite lines of one target (<target>: <prerequisite>) sorted bytewise; every other line in place";

/// The prerequisite target of a make driver line, or `None` when the line is not
/// a prerequisite line: it starts with a tab, a space or `#`, or has no `": "`
/// with a non-empty target (no whitespace, no earlier colon) before it and a
/// non-empty prerequisite after it. `.PHONY : x` is not one (a space before the
/// colon).
fn prerequisite_target(line: &[u8]) -> Option<&[u8]> {
    if matches!(line.first(), None | Some(b'\t' | b' ' | b'#')) {
        return None;
    }
    let colon = line.iter().position(|byte| *byte == b':')?;
    let (target, rest) = line.split_at(colon);
    let prerequisite = rest.strip_prefix(b": ")?;
    if target.is_empty() || target.iter().any(u8::is_ascii_whitespace) || prerequisite.is_empty() {
        return None;
    }
    Some(target)
}

/// The bytes of `CMakeFiles/Makefile2` with each maximal run of consecutive
/// prerequisite lines of one target sorted bytewise, every other line (and the
/// trailing newline) where it was ([`MAKE_DRIVER_RULE`]).
///
/// CMake emits the prerequisite lines of equal-rank targets in pointer order, so
/// two configures of one tree can differ only in the order of two lines. Sorting
/// within a run removes exactly that: a changed prerequisite, a line moved into
/// another target's run and reordered runs of different targets still change the
/// result.
///
/// This is configure-time evidence only. It is applied to that one file, never to
/// an object: the object comparison and the object normalisation of D-09 are
/// untouched.
#[must_use]
pub fn normalise_make_driver(bytes: &[u8]) -> Vec<u8> {
    let mut lines: Vec<&[u8]> = bytes.split(|byte| *byte == b'\n').collect();
    let mut start = 0;
    while start < lines.len() {
        let Some(target) = prerequisite_target(lines[start]) else {
            start += 1;
            continue;
        };
        let mut end = start + 1;
        while end < lines.len() && prerequisite_target(lines[end]) == Some(target) {
            end += 1;
        }
        lines[start..end].sort_unstable();
        start = end;
    }
    lines.join(&b'\n')
}

/// Re-check the configure-time evidence of a CMake capture against the files in
/// `run_root`: the first raw `configure_files` entry whose bytes no longer hash to
/// the recorded value, else the first `normalised_configure_files` entry whose
/// normalised bytes no longer do, as text naming the key. A file that cannot be
/// read is a change.
#[must_use]
pub fn configure_evidence_changed(run_root: &Path, report: &CaptureReport) -> Option<String> {
    for (key, expected) in &report.configure_files {
        match fs::read(run_root.join(key)) {
            Ok(bytes) if sha256_hex(&bytes) == *expected => {}
            Ok(_) => return Some(format!("{key} has changed since the capture")),
            Err(error) => return Some(format!("{key} cannot be read: {error}")),
        }
    }
    for (key, expected) in &report.normalised_configure_files {
        match fs::read(run_root.join(key)) {
            Ok(bytes) if sha256_hex(&normalise_make_driver(&bytes)) == expected.sha256 => {}
            Ok(_) => return Some(format!("{key} has changed since the capture")),
            Err(error) => return Some(format!("{key} cannot be read: {error}")),
        }
    }
    None
}

/// What the configure-time file hashes leave out, one line each. These lines are
/// the complete set: [`excluded_from_configure_hashes`] implements exactly them,
/// and the determinism comment of `testbed.yml` is held to them (plan 02-17).
pub const CONFIGURE_HASH_EXCLUSIONS: [&str; 5] = [
    "objects named in the capture report",
    "CMakeFiles/CMakeConfigureLog.yaml, CMakeFiles/CMakeOutput.log and CMakeFiles/CMakeError.log",
    "CMakeFiles/<version>/CMakeDetermineCompilerABI_<C or CXX>.bin",
    "CMakeFiles/<version>/CompilerId<C or CXX>/CMake<C or CXX>CompilerId.o",
    "static archives and shared libraries (.a, .so) created by the cmake build step",
];

/// The configure-time files that are hashed after a normalisation instead of byte
/// for byte, in words (plan 02-17 ties the `testbed.yml` determinism comment to
/// this).
pub const NORMALISED_CONFIGURE_FILES: [&str; 1] =
    ["CMakeFiles/Makefile2, hashed with each run of prerequisite lines of one target sorted"];

/// Whether `relative` (to the CMake build directory) is one of the files CMake's
/// own probes write, which hold scratch-directory names or the bytes of a test
/// compile and so differ from run to run, and are not an input of any compile
/// command.
///
/// Exactly these paths, with no suffix test at any depth:
/// - `CMakeFiles/CMakeConfigureLog.yaml`, `CMakeFiles/CMakeOutput.log` and
///   `CMakeFiles/CMakeError.log`;
/// - `CMakeFiles/<version>/CMakeDetermineCompilerABI_<LANG>.bin`;
/// - `CMakeFiles/<version>/CompilerId<LANG>/CMake<LANG>CompilerId.o`;
///
/// where `<version>` is non-empty and made only of ASCII digits and dots and
/// `<LANG>` is `C` or `CXX` (the same in both places of the compiler-id path).
///
/// Objects named in the capture report and archives or shared libraries the build
/// step created are left out by [`excluded_from_configure_hashes`], by declaration
/// and by provenance. `CMakeFiles/Makefile2` is hashed after
/// [`normalise_make_driver`], not left out.
#[must_use]
pub fn is_cmake_product(relative: &str) -> bool {
    if CONFIGURE_LOGS.contains(&relative) {
        return true;
    }
    let version_ok = |version: &str| {
        !version.is_empty() && version.bytes().all(|b| b.is_ascii_digit() || b == b'.')
    };
    match relative.split('/').collect::<Vec<_>>().as_slice() {
        ["CMakeFiles", version, name] => {
            version_ok(version)
                && matches!(
                    *name,
                    "CMakeDetermineCompilerABI_C.bin" | "CMakeDetermineCompilerABI_CXX.bin"
                )
        }
        ["CMakeFiles", version, dir, name] => {
            version_ok(version)
                && matches!(
                    (*dir, *name),
                    ("CompilerIdC", "CMakeCCompilerId.o")
                        | ("CompilerIdCXX", "CMakeCXXCompilerId.o")
                )
        }
        _ => false,
    }
}

/// Whether a file of the CMake build directory is left out of the configure-time
/// hashes. Pure: no file system access and no tool.
///
/// `relative` is relative to the build directory. `declared` holds the run-root
/// keys (`build/...`) of the objects named in the capture report, and `before` the
/// run-root listing taken after `cmake configure` and before `cmake build`. True
/// exactly when `build/<relative>` is declared, or [`is_cmake_product`], or
/// `relative` ends in `.a` or `.so` and `build/<relative>` is not in `before`,
/// that is, the build step created it. [`CONFIGURE_HASH_EXCLUSIONS`] is the same
/// set in words.
#[must_use]
pub fn excluded_from_configure_hashes(
    relative: &str,
    declared: &BTreeSet<String>,
    before: &BTreeSet<String>,
) -> bool {
    let key = format!("build/{relative}");
    declared.contains(&key)
        || is_cmake_product(relative)
        || ((relative.ends_with(".a") || relative.ends_with(".so")) && !before.contains(&key))
}

/// The macro that makes an object depend on a file's modification time.
const TIMESTAMP_MACRO: &[u8] = b"__TIMESTAMP__";

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
    /// `arm-none-eabi-readelf`, next to the compiler.
    pub readelf: PathBuf,
    /// `arm-none-eabi-objcopy`, next to the compiler.
    pub objcopy: PathBuf,
    /// The `arm-none-eabi` sysroot of the Arm toolchain install.
    pub sysroot: PathBuf,
    /// git, to verify a cached source again.
    pub git: PathBuf,
    /// The pinned (`expect`) version of each tool in [`REPORTED_TOOLS`].
    pub versions: BTreeMap<String, String>,
}

/// The directories of one target configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunLayout {
    /// `<work>/run/<id>/<config>`: the original build and every replay happen here.
    pub run_root: PathBuf,
    /// `<run_root>/src`: a copy of the verified cached tree.
    pub src: PathBuf,
    /// `<run_root>/build`: the CMake build directory.
    pub build: PathBuf,
    /// `<run_root>/inputs`: the committed files of `testbed/targets/<id>/`.
    pub inputs: PathBuf,
    /// `<work>/capture/<id>/<config>`: the capture artifacts.
    pub capture_dir: PathBuf,
    /// `<capture_dir>/objects`: byte copies of the original objects.
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
    /// `{cc}`: the pinned compiler.
    pub cc: &'a str,
    /// `{cflags}`: the effective flags, space separated.
    pub cflags: &'a str,
    /// `{src}`: the absolute source root of the run.
    pub src: &'a str,
    /// `{inputs}`: the absolute directory of the committed inputs.
    pub inputs: &'a str,
}

/// The comparison of a variant with its default configuration (BED-02, D-05).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariantDiff {
    /// Objects present in both configurations, by run-relative path.
    pub common_objects: Vec<String>,
    /// Common objects whose code digests differ, or cannot be compared.
    pub code_differs: Vec<String>,
    /// Objects only the default configuration builds.
    pub only_default: Vec<String>,
    /// Objects only the variant builds.
    pub only_variant: Vec<String>,
    /// True when the variant compiles different code.
    pub changes_code: bool,
}

/// What the build stage leaves for the record stage
/// (`<run_root>/stage/build-record.json`, WR-04).
///
/// It describes the build itself and nothing the record stage relies on for
/// evidence: the run records of the build steps, the cause lines the build stage
/// raised, the `not_proven` and `build_failed` flags, and the listing of the run
/// root just before the build. `target`, `config` and `source` let the record
/// stage refuse a record made for another pair or another source. The file is
/// written by code that runs after the upstream build and is read as untrusted
/// data (see the module documentation for what a forged record can and cannot
/// change).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildRecord {
    /// [`BUILD_RECORD_SCHEMA_VERSION`].
    pub schema_version: u32,
    /// Test-bed target id.
    pub target: String,
    /// Configuration name.
    pub config: String,
    /// The verified source the build stage copied.
    pub source: SourceRef,
    /// The build steps, in the order they ran.
    pub steps: Vec<Step>,
    /// The machine tokens the build stage raised (`build_failed`, `not_proven`).
    pub causes: Vec<String>,
    /// One line per cause.
    pub cause_details: Vec<String>,
    /// The run-root listing taken just before the build (after `cmake configure`
    /// for CMake), the base of the undeclared-object gate.
    pub before: BTreeSet<String>,
    /// A build step timed out or left an incomplete output stream.
    pub not_proven: bool,
    /// A build step exited non-zero.
    pub build_failed: bool,
    /// Every build step succeeded, so the exporter's file is expected.
    pub built: bool,
}

/// Why [`read_run_file`] did not return a file's bytes.
#[derive(Debug, thiserror::Error)]
pub enum RunReadError {
    /// The path is not a regular file reached without a symbolic link: it is
    /// absolute, holds a `.`, `..` or empty component, or a component from the
    /// run root down (the root included) is a symlink, or a directory component
    /// is not a directory, or the last component is not a regular file (a
    /// symlink, directory, socket, FIFO or device). `path` is the first
    /// offending path.
    #[error(
        "{} is not a regular file reached without a symbolic link under the run directory",
        path.display()
    )]
    NotRegular {
        /// The path that was refused.
        path: PathBuf,
    },
    /// The file system failed, or a component is missing.
    #[error("cannot read {}: {source}", path.display())]
    Io {
        /// The path involved.
        path: PathBuf,
        /// The underlying error.
        #[source]
        source: io::Error,
    },
}

/// Read `relative` (`/`-separated, relative to `run_root`) without ever following
/// a symlink (WR-04).
///
/// The run root and every component below it are inspected with
/// `fs::symlink_metadata`: each directory component must be a real directory and
/// the last component a regular file, so a symlink anywhere on the way, a
/// directory, a socket, a FIFO or a device is refused. An absolute `relative`, an
/// empty one, or one with a `.`, `..` or empty component is refused too. The
/// record stage reads every file of the untrusted run directory through this
/// function.
///
/// # Errors
/// [`RunReadError::NotRegular`] for a refused path, [`RunReadError::Io`] for a
/// missing component or a failing read.
pub fn read_run_file(run_root: &Path, relative: &str) -> Result<Vec<u8>, RunReadError> {
    let refused = |path: PathBuf| RunReadError::NotRegular { path };
    let io_failure = |path: &Path, source: io::Error| RunReadError::Io {
        path: path.to_path_buf(),
        source,
    };
    if relative.starts_with('/') {
        return Err(refused(PathBuf::from(relative)));
    }
    let parts: Vec<&str> = relative.split('/').collect();
    if parts
        .iter()
        .any(|part| part.is_empty() || *part == "." || *part == "..")
    {
        return Err(refused(run_root.join(relative)));
    }
    let root = fs::symlink_metadata(run_root).map_err(|source| io_failure(run_root, source))?;
    if !root.is_dir() {
        return Err(refused(run_root.to_path_buf()));
    }
    let mut current = run_root.to_path_buf();
    for (index, part) in parts.iter().enumerate() {
        current.push(part);
        let meta = fs::symlink_metadata(&current).map_err(|source| io_failure(&current, source))?;
        let acceptable = if index + 1 == parts.len() {
            meta.is_file()
        } else {
            meta.is_dir()
        };
        if !acceptable {
            return Err(refused(current));
        }
    }
    let mut file = fs::File::open(&current).map_err(|source| io_failure(&current, source))?;
    let opened = file
        .metadata()
        .map_err(|source| io_failure(&current, source))?;
    if !opened.is_file() {
        return Err(refused(current));
    }
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|source| io_failure(&current, source))?;
    Ok(bytes)
}

/// Whether `path` is a real directory (`Ok(true)`), absent (`Ok(false)`), or
/// something a walk must not start from: a symlink or a non-directory.
fn walk_root(path: &Path) -> Result<bool, RunReadError> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() => Ok(true),
        Ok(_) => Err(RunReadError::NotRegular {
            path: path.to_path_buf(),
        }),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(source) => Err(RunReadError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

/// Why a capture could not be attempted. A target-configuration failure is
/// data in the report, never an error.
#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    /// The inputs contradict themselves or cannot be used safely.
    #[error("invalid capture input: {0}")]
    InvalidInput(String),
    /// A runner failure: a tool that cannot be started or supervised.
    #[error(transparent)]
    Runner(#[from] RunnerError),
    /// The cached source is missing or no longer matches its pin.
    #[error(transparent)]
    Fetch(#[from] FetchError),
    /// A tree could not be hashed or copied.
    #[error(transparent)]
    Tree(#[from] TreeHashError),
    /// The canonical compile commands could not be written.
    #[error(transparent)]
    Cc(#[from] CcError),
    /// An entry could not be checked at all.
    #[error(transparent)]
    Entry(#[from] EntryError),
    /// The report could not be serialised.
    #[error("cannot serialise the capture report: {0}")]
    Json(#[from] serde_json::Error),
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

fn io_error(action: &'static str, path: &Path, source: io::Error) -> CaptureError {
    CaptureError::Io {
        action,
        path: path.to_path_buf(),
        source,
    }
}

fn invalid(message: impl Into<String>) -> CaptureError {
    CaptureError::InvalidInput(message.into())
}

fn path_str(path: &Path) -> Result<&str, CaptureError> {
    path.to_str()
        .ok_or_else(|| invalid(format!("{} is not valid UTF-8", path.display())))
}

/// The tool versions a capture of a `build` target records in its report: the
/// pinned version of each reported tool, except the one the build system does not
/// use (Bear serves Make targets, CMake serves CMake targets).
///
/// Capture records this map and replay compares a report with it, so the two
/// cannot disagree about what a genuine report holds.
#[must_use]
pub fn reported_tools(tools: &ToolPaths, build: BuildSystem) -> BTreeMap<String, String> {
    let mut reported = BTreeMap::new();
    for key in REPORTED_TOOLS {
        let unused = matches!(
            (key, build),
            ("bear", BuildSystem::Cmake) | ("cmake", BuildSystem::Make)
        );
        if !unused && let Some(version) = tools.versions.get(key) {
            reported.insert(key.to_owned(), version.clone());
        }
    }
    reported
}

impl ToolPaths {
    /// The tools of the pins. `readelf`, `objcopy` and the sysroot are derived
    /// from the install directory of the pinned Arm compiler.
    ///
    /// # Errors
    /// [`CaptureError`] when a tool is not an installed pin or the compiler path
    /// has no install directory.
    pub fn from_pins(pins: &Pins) -> Result<Self, CaptureError> {
        let arm_gcc = pinned_tool(pins, "arm_gcc")?;
        let bin_dir = arm_gcc
            .parent()
            .ok_or_else(|| invalid("the pinned arm_gcc has no directory"))?;
        let install = bin_dir
            .parent()
            .ok_or_else(|| invalid("the pinned arm_gcc has no install directory"))?;
        let mut versions = BTreeMap::new();
        for key in REPORTED_TOOLS {
            if let Some(spec) = pins.tool.get(key)
                && !spec.expect.is_empty()
            {
                versions.insert(key.to_owned(), spec.expect.clone());
            }
        }
        Ok(Self {
            readelf: bin_dir.join("arm-none-eabi-readelf"),
            objcopy: bin_dir.join("arm-none-eabi-objcopy"),
            sysroot: install.join("arm-none-eabi"),
            bear: pinned_tool(pins, "bear")?,
            cmake: pinned_tool(pins, "cmake")?,
            make: pinned_tool(pins, "make")?,
            clang: pinned_tool(pins, "clang")?,
            git: pinned_tool(pins, "git")?,
            arm_gcc,
            versions,
        })
    }
}

impl RunLayout {
    /// The layout of `<work>/run/<id>/<config>` and
    /// `<work>/capture/<id>/<config>`.
    #[must_use]
    pub fn new(work_dir: &Path, id: &str, config: &str) -> Self {
        let run_root = work_dir.join("run").join(id).join(config);
        let capture_dir = work_dir.join("capture").join(id).join(config);
        Self {
            src: run_root.join("src"),
            build: run_root.join("build"),
            inputs: run_root.join("inputs"),
            objects_dir: capture_dir.join("objects"),
            run_root,
            capture_dir,
        }
    }

    /// `<run_root>/stage`: the only directory the build stage writes outside the
    /// source, build and inputs trees (WR-04).
    #[must_use]
    pub fn stage_dir(&self) -> PathBuf {
        self.run_root.join(STAGE_DIR)
    }

    /// The build record the build stage leaves for the record stage.
    #[must_use]
    pub fn build_record_path(&self) -> PathBuf {
        self.stage_dir().join(BUILD_RECORD_FILE)
    }

    /// The exporter's file exactly as the build wrote it: Bear's output or a copy
    /// of CMake's `compile_commands.json`. It lies in the run directory, which an
    /// untrusted build can change; the record stage copies it to
    /// [`RunLayout::raw_copy_path`] and canonicalises that copy.
    #[must_use]
    pub fn raw_path(&self) -> PathBuf {
        self.stage_dir().join(RAW_EXPORT_FILE)
    }

    /// The evidence copy of the exporter's file, written by the record stage.
    #[must_use]
    pub fn raw_copy_path(&self) -> PathBuf {
        self.capture_dir.join(RAW_EXPORT_FILE)
    }

    /// The canonical compile commands replay executes.
    #[must_use]
    pub fn canonical_path(&self) -> PathBuf {
        self.capture_dir.join("compile_commands.json")
    }

    /// The generated CMake toolchain file the build stage configures with.
    #[must_use]
    pub fn toolchain_path(&self) -> PathBuf {
        self.stage_dir().join(TOOLCHAIN_FILE)
    }

    /// The evidence copy of the toolchain file: the record stage writes the text
    /// it recomputes from the pins and the configuration, not the build's file.
    #[must_use]
    pub fn toolchain_copy_path(&self) -> PathBuf {
        self.capture_dir.join(TOOLCHAIN_FILE)
    }

    /// The capture report.
    #[must_use]
    pub fn report_path(&self) -> PathBuf {
        self.capture_dir.join("capture.json")
    }
}

impl CaptureContext {
    /// A context with the default 900 s step timeout.
    #[must_use]
    pub fn new(
        work_dir: &Path,
        testbed_dir: &Path,
        tools: ToolPaths,
        baseline: Vec<String>,
    ) -> Self {
        Self {
            work_dir: work_dir.to_path_buf(),
            testbed_dir: testbed_dir.to_path_buf(),
            tools,
            baseline,
            timeout: STEP_TIMEOUT,
        }
    }
}

/// The compiler flags of a configuration: the baseline, then `-D<define>` for
/// each define, then the configuration's own flags (D-06).
#[must_use]
pub fn effective_cflags(baseline: &[String], config: &Config) -> Vec<String> {
    baseline
        .iter()
        .cloned()
        .chain(config.defines.iter().map(|define| format!("-D{define}")))
        .chain(config.cflags.iter().cloned())
        .collect()
}

/// Expand `{cc}`, `{cflags}`, `{inputs}` and `{src}` in `template`.
///
/// # Errors
/// [`CaptureError::InvalidInput`] for any other brace text, including a lone
/// `}` or an unterminated `{`.
pub fn expand(template: &str, values: &Expansion<'_>) -> Result<String, CaptureError> {
    let replacements = [
        ("{cc}", values.cc),
        ("{cflags}", values.cflags),
        ("{inputs}", values.inputs),
        ("{src}", values.src),
    ];
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(at) = rest.find(['{', '}']) {
        out.push_str(&rest[..at]);
        let tail = &rest[at..];
        match replacements
            .iter()
            .find(|(placeholder, _)| tail.starts_with(placeholder))
        {
            Some((placeholder, value)) => {
                out.push_str(value);
                rest = &tail[placeholder.len()..];
            }
            None => {
                let snippet: String = tail.chars().take(12).collect();
                return Err(invalid(format!(
                    "brace text {snippet:?} in {template:?} is not one of {PLACEHOLDERS:?}"
                )));
            }
        }
    }
    out.push_str(rest);
    Ok(out)
}

/// A flag the manifest gives may name `{src}` or `{inputs}` (an Mbed TLS
/// configuration file lives in the source tree) but not `{cc}` or `{cflags}`,
/// which would refer to the flags being built. Control characters are refused.
///
/// # Errors
/// [`CaptureError::InvalidInput`].
pub fn expand_flags(
    flags: &[String],
    tools: &ToolPaths,
    layout: &RunLayout,
) -> Result<Vec<String>, CaptureError> {
    let values = Expansion {
        cc: path_str(&tools.arm_gcc)?,
        cflags: "",
        src: path_str(&layout.src)?,
        inputs: path_str(&layout.inputs)?,
    };
    flags
        .iter()
        .map(|flag| {
            if flag.chars().any(char::is_control) {
                return Err(invalid(format!("flag {flag:?} holds a control character")));
            }
            if flag.contains("{cc}") || flag.contains("{cflags}") {
                return Err(invalid(format!(
                    "flag {flag:?} may not refer to {{cc}} or {{cflags}}"
                )));
            }
            expand(flag, &values)
        })
        .collect()
}

/// The build directory: `src/<dir>`, lexically inside `src`.
fn build_dir(layout: &RunLayout, args: &BuildArgs) -> Result<PathBuf, CaptureError> {
    let relative = Path::new(&args.dir);
    let mut dir = layout.src.clone();
    for component in relative.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => dir.push(part),
            _ => {
                return Err(invalid(format!(
                    "build directory {:?} must be a relative path inside the source tree",
                    args.dir
                )));
            }
        }
    }
    Ok(dir)
}

fn expansion<'a>(
    tools: &'a ToolPaths,
    layout: &'a RunLayout,
    joined_flags: &'a str,
) -> Result<Expansion<'a>, CaptureError> {
    Ok(Expansion {
        cc: path_str(&tools.arm_gcc)?,
        cflags: joined_flags,
        src: path_str(&layout.src)?,
        inputs: path_str(&layout.inputs)?,
    })
}

/// A Make variable assignment: `NAME=value` with an optional `:`, `+` or `?`
/// before the `=`. Anything else could be read by Make as an option.
fn is_make_assignment(text: &str) -> bool {
    let Some((name, _)) = text.split_once('=') else {
        return false;
    };
    let name = name.trim_end_matches([':', '+', '?']);
    let mut bytes = name.bytes();
    bytes
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// The Make capture command line (D-07).
///
/// `bear --output <raw> -- make -j1 <make_vars> <goals>`: the target's variables
/// and goals first, then the configuration's. `-j1` keeps Bear's entry order
/// stable (Pitfall 8). `cflags` must already be expanded ([`expand_flags`]).
///
/// # Errors
/// [`CaptureError`] for an unknown placeholder, a variable that is not a
/// `NAME=value` assignment, a goal that starts with `-`, or a non-UTF-8 path.
pub fn make_argv(
    tools: &ToolPaths,
    layout: &RunLayout,
    cflags: &[String],
    args: &BuildArgs,
    config: &Config,
) -> Result<Vec<String>, CaptureError> {
    let joined = cflags.join(" ");
    let values = expansion(tools, layout, &joined)?;
    let mut argv = vec![
        path_str(&tools.bear)?.to_owned(),
        "--output".to_owned(),
        path_str(&layout.raw_path())?.to_owned(),
        "--".to_owned(),
        path_str(&tools.make)?.to_owned(),
        "-j1".to_owned(),
    ];
    for variable in args.make_vars.iter().chain(&config.make_vars) {
        let text = expand(variable, &values)?;
        if !is_make_assignment(&text) {
            return Err(invalid(format!(
                "make variable {text:?} is not a NAME=value assignment"
            )));
        }
        argv.push(text);
    }
    for goal in args.goals.iter().chain(&config.goals) {
        let text = expand(goal, &values)?;
        if text.is_empty() || text.starts_with('-') {
            return Err(invalid(format!("make goal {text:?} is not a goal name")));
        }
        argv.push(text);
    }
    Ok(argv)
}

/// A value written into the toolchain file inside double quotes.
fn cmake_quote(text: &str) -> Result<String, CaptureError> {
    if text
        .chars()
        .any(|c| c.is_control() || matches!(c, '"' | '\\' | '$' | ';'))
    {
        return Err(invalid(format!(
            "{text:?} cannot be written into a CMake toolchain file"
        )));
    }
    Ok(format!("\"{text}\""))
}

/// The CMake toolchain file of one configuration (D-06, D-07).
///
/// `CMAKE_C_FLAGS_INIT` holds the baseline and the configuration's flags, so no
/// `-DCMAKE_C_FLAGS` is ever needed. `CMAKE_TRY_COMPILE_TARGET_TYPE
/// STATIC_LIBRARY` stops the compiler probe from linking an executable against
/// newlib stubs (Pattern 5). The text depends only on its arguments, so its
/// hash in the report is reproducible. `cflags` must already be expanded.
///
/// # Errors
/// [`CaptureError::InvalidInput`] for a flag or path holding a quote,
/// backslash, `$`, `;` or control character.
pub fn toolchain_file(tools: &ToolPaths, cflags: &[String]) -> Result<String, CaptureError> {
    let bin_dir = tools
        .arm_gcc
        .parent()
        .ok_or_else(|| invalid("the pinned arm_gcc has no directory"))?;
    let compiler = cmake_quote(path_str(&tools.arm_gcc)?)?;
    let ar = cmake_quote(path_str(&bin_dir.join("arm-none-eabi-ar"))?)?;
    let ranlib = cmake_quote(path_str(&bin_dir.join("arm-none-eabi-ranlib"))?)?;
    let flags = cmake_quote(&cflags.join(" "))?;
    Ok(format!(
        "# Generated by migrationtool for one target configuration. Do not edit.\n\
         set(CMAKE_SYSTEM_NAME Generic)\n\
         set(CMAKE_SYSTEM_PROCESSOR arm)\n\
         set(CMAKE_C_COMPILER {compiler})\n\
         set(CMAKE_AR {ar})\n\
         set(CMAKE_RANLIB {ranlib})\n\
         set(CMAKE_TRY_COMPILE_TARGET_TYPE STATIC_LIBRARY)\n\
         set(CMAKE_C_FLAGS_INIT {flags})\n\
         set(CMAKE_FIND_ROOT_PATH_MODE_PROGRAM NEVER)\n\
         set(CMAKE_FIND_ROOT_PATH_MODE_LIBRARY ONLY)\n\
         set(CMAKE_FIND_ROOT_PATH_MODE_INCLUDE ONLY)\n"
    ))
}

/// Refuse a CMake option that sets what the driver owns: the C flags, the
/// build type, the toolchain file, the compiler or the export switch (RESEARCH
/// Pitfalls 1 and 3). A detached `-D` is refused because its name is the next
/// argument.
fn check_cmake_option(option: &str) -> Result<(), CaptureError> {
    if option == "-D" {
        return Err(invalid(
            "a detached -D hides the variable name; write -DNAME=value",
        ));
    }
    if let Some(definition) = option.strip_prefix("-D") {
        let name = definition.split(['=', ':']).next().unwrap_or_default();
        let owned = name.starts_with("CMAKE_C_FLAGS")
            || matches!(
                name,
                "CMAKE_BUILD_TYPE"
                    | "CMAKE_TOOLCHAIN_FILE"
                    | "CMAKE_MAKE_PROGRAM"
                    | "CMAKE_C_COMPILER"
                    | "CMAKE_EXPORT_COMPILE_COMMANDS"
            );
        if owned {
            return Err(invalid(format!(
                "cmake option {option:?} sets {name}, which the capture driver owns"
            )));
        }
    }
    Ok(())
}

/// The CMake configure command line.
///
/// `cmake -S src/<dir> -B build -G "Unix Makefiles" -DCMAKE_TOOLCHAIN_FILE=...
/// -DCMAKE_EXPORT_COMPILE_COMMANDS=ON -DCMAKE_BUILD_TYPE=MinSizeRel <options>`.
/// `MinSizeRel` gives `-Os` and does not append `-O3` after the baseline
/// (Pitfall 3). `cflags` is used only for `{cflags}` in the options.
///
/// # Errors
/// [`CaptureError`] for an option the driver owns, an unknown placeholder or a
/// non-UTF-8 path.
pub fn cmake_configure_argv(
    tools: &ToolPaths,
    layout: &RunLayout,
    cflags: &[String],
    args: &BuildArgs,
    config: &Config,
) -> Result<Vec<String>, CaptureError> {
    let joined = cflags.join(" ");
    let values = expansion(tools, layout, &joined)?;
    let mut argv = vec![
        path_str(&tools.cmake)?.to_owned(),
        "-S".to_owned(),
        path_str(&build_dir(layout, args)?)?.to_owned(),
        "-B".to_owned(),
        path_str(&layout.build)?.to_owned(),
        "-G".to_owned(),
        "Unix Makefiles".to_owned(),
        format!(
            "-DCMAKE_TOOLCHAIN_FILE={}",
            path_str(&layout.toolchain_path())?
        ),
        "-DCMAKE_EXPORT_COMPILE_COMMANDS=ON".to_owned(),
        "-DCMAKE_BUILD_TYPE=MinSizeRel".to_owned(),
        // A cache entry: set in the toolchain file it is not honoured by CMake 3.28.
        format!("-DCMAKE_MAKE_PROGRAM={}", path_str(&tools.make)?),
    ];
    for option in args.cmake_options.iter().chain(&config.cmake_options) {
        let text = expand(option, &values)?;
        check_cmake_option(&text)?;
        argv.push(text);
    }
    Ok(argv)
}

/// The CMake build command line: `cmake --build build --target <T> -j1`, the
/// configuration's target when it names one, else the target's.
///
/// # Errors
/// [`CaptureError`] when neither names a CMake target.
pub fn cmake_build_argv(
    tools: &ToolPaths,
    layout: &RunLayout,
    args: &BuildArgs,
    config: &Config,
) -> Result<Vec<String>, CaptureError> {
    let target = config
        .cmake_target
        .as_ref()
        .or(args.cmake_target.as_ref())
        .ok_or_else(|| invalid("a cmake build needs a cmake_target"))?;
    if target.is_empty() || target.starts_with('-') {
        return Err(invalid(format!(
            "cmake target {target:?} is not a target name"
        )));
    }
    Ok(vec![
        path_str(&tools.cmake)?.to_owned(),
        "--build".to_owned(),
        path_str(&layout.build)?.to_owned(),
        "--target".to_owned(),
        target.clone(),
        "-j1".to_owned(),
    ])
}

/// Every file (and symlink) under `root` as sorted `/`-separated relative
/// paths. Symlinks are listed, never followed. A missing root is empty.
///
/// # Errors
/// [`CaptureError::Io`] when a directory cannot be read.
pub fn list_files(root: &Path) -> Result<BTreeSet<String>, CaptureError> {
    let mut found = BTreeSet::new();
    let mut stack = vec![PathBuf::new()];
    while let Some(relative) = stack.pop() {
        let dir = root.join(&relative);
        let listing = match fs::read_dir(&dir) {
            Ok(listing) => listing,
            Err(source)
                if source.kind() == io::ErrorKind::NotFound && relative.as_os_str().is_empty() =>
            {
                return Ok(found);
            }
            Err(source) => return Err(io_error("read", &dir, source)),
        };
        for entry in listing {
            let entry = entry.map_err(|source| io_error("read", &dir, source))?;
            let child = relative.join(entry.file_name());
            let file_type = entry
                .file_type()
                .map_err(|source| io_error("inspect", &entry.path(), source))?;
            if file_type.is_dir() {
                stack.push(child);
            } else {
                let key = child
                    .to_str()
                    .ok_or_else(|| invalid(format!("{} is not valid UTF-8", child.display())))?;
                found.insert(key.to_owned());
            }
        }
    }
    Ok(found)
}

fn is_object_name(key: &str) -> bool {
    key.ends_with(".o") || key.ends_with(".obj")
}

/// Object files (`.o`, `.obj`) the build created that no compile command
/// declares, in sorted order (RESEARCH Pitfall 7: Bear can fail silently, and a
/// build can also write an object by a rule Bear does not see). Keys are
/// run-root-relative.
#[must_use]
pub fn undeclared_objects(
    before: &BTreeSet<String>,
    after: &BTreeSet<String>,
    declared: &BTreeSet<String>,
) -> Vec<String> {
    after
        .difference(before)
        .filter(|key| is_object_name(key) && !declared.contains(*key))
        .cloned()
        .collect()
}

/// The `file` of each captured entry whose source contains `__TIMESTAMP__`
/// (the Reproduction table: it expands to a file's modification time, which
/// `SOURCE_DATE_EPOCH` does not control). A relative `file` is resolved against
/// the entry's directory. Headers are not scanned.
///
/// # Errors
/// [`CaptureError::Io`] for a source that cannot be read: an unreadable source
/// is not a pass.
pub fn timestamp_macro_sources(entries: &[CcEntry]) -> Result<Vec<String>, CaptureError> {
    let stamped = timestamp_entries(entries)?;
    let files: BTreeSet<String> = stamped
        .into_iter()
        .map(|entry| entry.file.clone())
        .collect();
    Ok(files.into_iter().collect())
}

/// The entries whose source contains `__TIMESTAMP__`, in input order.
fn timestamp_entries(entries: &[CcEntry]) -> Result<Vec<&CcEntry>, CaptureError> {
    let mut found = Vec::new();
    for entry in entries {
        let path = Path::new(&entry.directory).join(&entry.file);
        let bytes = fs::read(&path).map_err(|source| io_error("read", &path, source))?;
        if bytes
            .windows(TIMESTAMP_MACRO.len())
            .any(|window| window == TIMESTAMP_MACRO)
        {
            found.push(entry);
        }
    }
    Ok(found)
}

/// Compare the code of a variant with its default configuration (BED-02, D-05).
///
/// Objects with the same run-relative path are compared by `code_digest`, which
/// covers the executable sections only. A variant that builds a different set of
/// objects, or whose digest cannot be read on either side, changes code. A
/// variant that changes only data or names reports `changes_code: false`.
#[must_use]
pub fn variant_code_diff(default: &CaptureReport, variant: &CaptureReport) -> VariantDiff {
    let mut diff = VariantDiff::default();
    for (key, base) in &default.objects {
        match variant.objects.get(key) {
            Some(other) => {
                diff.common_objects.push(key.clone());
                let same = base.code_digest.is_some() && base.code_digest == other.code_digest;
                if !same {
                    diff.code_differs.push(key.clone());
                }
            }
            None => diff.only_default.push(key.clone()),
        }
    }
    diff.only_variant = variant
        .objects
        .keys()
        .filter(|key| !default.objects.contains_key(*key))
        .cloned()
        .collect();
    diff.changes_code = !(diff.code_differs.is_empty()
        && diff.only_default.is_empty()
        && diff.only_variant.is_empty());
    diff
}

/// The machine token for a refused compile-commands file.
fn cause_of(error: &CcError) -> &'static str {
    match error {
        CcError::Unsplittable => "unsplittable_command",
        CcError::NoCommand => "no_command",
        CcError::DanglingOutput => "dangling_output",
        CcError::BadFile => "bad_file",
        CcError::NotCompileOnly => "not_compile_only",
        CcError::CompilerNotAllowed { .. } => "compiler_not_allowed",
        CcError::ForbiddenFlag { .. } => "forbidden_flag",
        CcError::OutsideRunRoot { .. } => "outside_run_root",
        CcError::Empty => "empty_capture",
        CcError::DuplicateOutput { .. } => "duplicate_output",
        CcError::Json(_) => "invalid_compile_commands",
    }
}

/// The last bytes of a stream as one line of text (runs of whitespace become
/// one space), for a failure message.
fn tail_excerpt(bytes: &[u8]) -> String {
    const KEEP: usize = 1500;
    let tail = bytes
        .get(bytes.len().saturating_sub(KEEP)..)
        .unwrap_or_default();
    String::from_utf8_lossy(tail)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// How a step ended.
enum Outcome {
    /// Exit code 0 with complete streams; the stored standard output.
    Success(Vec<u8>),
    /// A timeout or an incomplete stream: nothing is proven.
    NotProven,
    /// A non-zero exit (or a signal), with the tail of its standard error as
    /// one line of text so a failure can be diagnosed from the report alone.
    Failed(Option<i32>, String),
}

/// Whether the pipeline goes on after a phase.
enum Flow {
    Continue,
    Halt,
}

/// One capture in progress.
struct Session<'a> {
    cfg: RunnerConfig,
    ctx: &'a CaptureContext,
    layout: RunLayout,
    env: BTreeMap<String, String>,
    report: CaptureReport,
    /// Set when any step or gate could not be proven.
    not_proven: bool,
    /// Set when the build exited non-zero.
    build_failed: bool,
    /// The files under the run root just before the build.
    before: BTreeSet<String>,
}

impl Session<'_> {
    fn cause(&mut self, token: &str, detail: String) {
        if !self.report.causes.iter().any(|existing| existing == token) {
            self.report.causes.push(token.to_owned());
        }
        self.report.cause_details.push(format!("{token}: {detail}"));
    }

    fn halt(&mut self, token: &str, detail: String) -> Flow {
        self.cause(token, detail);
        Flow::Halt
    }

    /// Run one step and keep its record.
    fn exec(
        &mut self,
        name: &str,
        tool: &str,
        argv: &[String],
        cwd: &Path,
    ) -> Result<Outcome, CaptureError> {
        let request = RunRequest {
            tool_name: tool.to_owned(),
            argv: argv.iter().map(OsString::from).collect(),
            cwd: cwd.to_path_buf(),
            extra_env: self.env.clone(),
            tool_version: self.ctx.tools.versions.get(tool).cloned(),
        };
        let output = run(&self.cfg, &request)?;
        let exit = output.record.exit.clone();
        let complete = output.record.stdout.complete && output.record.stderr.complete;
        self.report.steps.push(Step {
            name: name.to_owned(),
            run: output.record,
        });
        if exit.timed_out || !complete {
            Ok(Outcome::NotProven)
        } else if exit.code == Some(0) {
            Ok(Outcome::Success(output.stdout))
        } else {
            Ok(Outcome::Failed(exit.code, tail_excerpt(&output.stderr)))
        }
    }

    /// Run a build step; anything but success halts the capture.
    fn build_step(
        &mut self,
        name: &str,
        tool: &str,
        argv: &[String],
        cwd: &Path,
    ) -> Result<Flow, CaptureError> {
        match self.exec(name, tool, argv, cwd)? {
            Outcome::Success(_) => Ok(Flow::Continue),
            Outcome::NotProven => {
                self.not_proven = true;
                Ok(self.halt(
                    "not_proven",
                    format!("{name} timed out or left an incomplete output stream"),
                ))
            }
            Outcome::Failed(code, excerpt) => {
                self.build_failed = true;
                let detail = if excerpt.is_empty() {
                    format!("{name} exited with {code:?}")
                } else {
                    format!("{name} exited with {code:?}: {excerpt}")
                };
                Ok(self.halt("build_failed", detail))
            }
        }
    }

    /// Verify the cached source again (no curl, tar or protocol: it touches only
    /// the files and git's own object store).
    fn verify(&self, id: &str, target: &Target) -> Result<FetchReport, CaptureError> {
        let ctx = self.ctx;
        let fetch_ctx = FetchContext {
            work_dir: ctx.work_dir.clone(),
            git: ctx.tools.git.clone(),
            curl: PathBuf::new(),
            tar: PathBuf::from(TAR_PATH),
            allowed_protocols: String::new(),
            timeout: ctx.timeout,
        };
        Ok(verify_cached(&self.cfg, &fetch_ctx, id, target)?)
    }

    /// Refuse a directory that is not strictly inside the work directory.
    fn inside_work(&self, dir: &Path) -> Result<(), CaptureError> {
        if !dir.starts_with(&self.ctx.work_dir) || dir == self.ctx.work_dir {
            return Err(invalid(format!(
                "{} is not inside the work directory",
                dir.display()
            )));
        }
        Ok(())
    }

    /// Create `<work>/home` and `<work>/tmp` when they are missing: a stage may
    /// run in a work directory the other stage never wrote.
    fn ensure_home_and_tmp(&self) -> Result<(), CaptureError> {
        for dir in [
            self.ctx.work_dir.join("home"),
            self.ctx.work_dir.join("tmp"),
        ] {
            fs::create_dir_all(&dir).map_err(|source| io_error("create", &dir, source))?;
        }
        Ok(())
    }

    /// Build stage: empty the run directory in place (it may be a bind mount),
    /// make its stage directory, copy the verified source and the committed
    /// inputs. The capture directory is not touched.
    fn prepare_run(&mut self, id: &str, verified: &FetchReport) -> Result<(), CaptureError> {
        let ctx = self.ctx;
        self.inside_work(&self.layout.run_root)?;
        empty_directory(&self.layout.run_root)?;
        let stage = self.layout.stage_dir();
        fs::create_dir_all(&stage).map_err(|source| io_error("create", &stage, source))?;
        self.ensure_home_and_tmp()?;

        let cache = cache_paths(&ctx.work_dir, id, &verified.commit);
        copy_tree(&cache.tree, &self.layout.src)?;
        let committed = ctx.testbed_dir.join("targets").join(id);
        if committed.is_dir() {
            copy_tree(&committed, &self.layout.inputs)?;
        } else {
            fs::create_dir_all(&self.layout.inputs)
                .map_err(|source| io_error("create", &self.layout.inputs, source))?;
        }
        Ok(())
    }

    /// Record stage: a fresh capture directory, and the home and tmp directories.
    fn prepare_record(&mut self) -> Result<(), CaptureError> {
        let dir = self.layout.capture_dir.clone();
        self.inside_work(&dir)?;
        match fs::remove_dir_all(&dir) {
            Ok(()) => {}
            Err(source) if source.kind() == io::ErrorKind::NotFound => {}
            Err(source) => return Err(io_error("remove", &dir, source)),
        }
        fs::create_dir_all(&dir).map_err(|source| io_error("create", &dir, source))?;
        self.ensure_home_and_tmp()
    }

    /// Note a read of the run directory the record stage refused (a symlink or a
    /// file that is not regular): the cause `run_dir_symlink`, naming `key`.
    fn refused(&mut self, key: &str, error: &RunReadError) {
        self.cause(
            "run_dir_symlink",
            format!(
                "{key}: the build left a symbolic link or a file that is not regular where the record stage reads ({error})"
            ),
        );
    }

    /// Read `key` of the run directory without following a symlink. A refused
    /// read is a cause and `None`; a missing file or an I/O failure is an error.
    fn read_or_refuse(&mut self, key: &str) -> Result<Option<Vec<u8>>, CaptureError> {
        match read_run_file(&self.layout.run_root, key) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error @ RunReadError::NotRegular { .. }) => {
                self.refused(key, &error);
                Ok(None)
            }
            Err(RunReadError::Io { path, source }) => Err(io_error("read", &path, source)),
        }
    }

    /// Run the build and leave the exporter's file at the raw path in the stage
    /// directory (if there is one). Build stage only.
    fn build(
        &mut self,
        args: &BuildArgs,
        config: &Config,
        build: BuildSystem,
    ) -> Result<Flow, CaptureError> {
        // Copy the shared reference out of `self` so the tools stay borrowable
        // while the session is borrowed mutably for each step.
        let ctx = self.ctx;
        let tools = &ctx.tools;
        let flags = expand_flags(
            &effective_cflags(&ctx.baseline, config),
            tools,
            &self.layout,
        )?;
        let directory = build_dir(&self.layout, args)?;
        match build {
            BuildSystem::Make => {
                let argv = make_argv(tools, &self.layout, &flags, args, config)?;
                self.before = list_files(&self.layout.run_root)?;
                self.build_step("build", "bear", &argv, &directory)
            }
            BuildSystem::Cmake => {
                let text = toolchain_file(tools, &flags)?;
                let path = self.layout.toolchain_path();
                fs::write(&path, &text).map_err(|source| io_error("write", &path, source))?;
                let configure = cmake_configure_argv(tools, &self.layout, &flags, args, config)?;
                let run_root = self.layout.run_root.clone();
                if matches!(
                    self.build_step("cmake configure", "cmake", &configure, &run_root)?,
                    Flow::Halt
                ) {
                    return Ok(Flow::Halt);
                }
                // Configure-time objects (compiler probes) are not the build's.
                self.before = list_files(&self.layout.run_root)?;
                let build_argv = cmake_build_argv(tools, &self.layout, args, config)?;
                if matches!(
                    self.build_step("cmake build", "cmake", &build_argv, &run_root)?,
                    Flow::Halt
                ) {
                    return Ok(Flow::Halt);
                }
                let export = self.layout.build.join("compile_commands.json");
                match fs::copy(&export, self.layout.raw_path()) {
                    Ok(_) => {}
                    Err(source) if source.kind() == io::ErrorKind::NotFound => {}
                    Err(source) => return Err(io_error("copy", &export, source)),
                }
                Ok(Flow::Continue)
            }
        }
    }

    /// Copy the exporter's file from the stage directory to the capture
    /// directory, reading it without following a symlink. A missing file copies
    /// nothing (canonicalising then reports `empty_capture`); a refused one is a
    /// cause and halts.
    fn copy_raw_export(&mut self) -> Result<Flow, CaptureError> {
        let key = format!("{STAGE_DIR}/{RAW_EXPORT_FILE}");
        match read_run_file(&self.layout.run_root, &key) {
            Ok(bytes) => {
                let path = self.layout.raw_copy_path();
                fs::write(&path, bytes).map_err(|source| io_error("write", &path, source))?;
                Ok(Flow::Continue)
            }
            Err(RunReadError::Io { source, .. }) if source.kind() == io::ErrorKind::NotFound => {
                Ok(Flow::Continue)
            }
            Err(error @ RunReadError::NotRegular { .. }) => {
                self.refused(&key, &error);
                Ok(Flow::Halt)
            }
            Err(RunReadError::Io { path, source }) => Err(io_error("read", &path, source)),
        }
    }

    /// Canonicalise the evidence copy of the export and write the canonical file.
    fn canonicalise(&mut self) -> Result<Option<Vec<CcEntry>>, CaptureError> {
        let raw_path = self.layout.raw_copy_path();
        let bytes = match fs::read(&raw_path) {
            Ok(bytes) => bytes,
            Err(source) if source.kind() == io::ErrorKind::NotFound => {
                self.cause(
                    "empty_capture",
                    "the build wrote no compile commands (Bear or CMake exported nothing)"
                        .to_owned(),
                );
                return Ok(None);
            }
            Err(source) => return Err(io_error("read", &raw_path, source)),
        };
        self.report.compile_commands.raw_sha256 = Some(sha256_hex(&bytes));
        let raw = match parse_raw(&bytes) {
            Ok(raw) => raw,
            Err(error) => {
                self.cause(cause_of(&error), error.to_string());
                return Ok(None);
            }
        };
        self.report.compile_commands.entries_exported = raw.len();
        let policy = CcPolicy {
            allowed_compilers: vec![path_str(&self.ctx.tools.arm_gcc)?.to_owned()],
            run_root: self.layout.run_root.clone(),
        };
        // A symlink where an object should be counts as present, so that the
        // record stage refuses it loudly (run_dir_symlink) instead of dropping
        // the entry as unbuilt.
        let exists = |path: &Path| {
            fs::symlink_metadata(path)
                .is_ok_and(|meta| meta.is_file() || meta.file_type().is_symlink())
        };
        let canonical = match canonicalise(&raw, &policy, &exists) {
            Ok(canonical) => canonical,
            Err(error) => {
                self.cause(cause_of(&error), error.to_string());
                return Ok(None);
            }
        };
        let text = to_json(&canonical.entries)?;
        let path = self.layout.canonical_path();
        fs::write(&path, &text).map_err(|source| io_error("write", &path, source))?;
        self.report.compile_commands = CompileCommandsInfo {
            raw_sha256: self.report.compile_commands.raw_sha256.clone(),
            canonical_sha256: Some(sha256_hex(text.as_bytes())),
            entries_exported: canonical.entries_exported,
            entries_built: canonical.entries_built,
            dropped_unbuilt: canonical.dropped_unbuilt,
        };
        self.report.gates.compiler_allow_listed = true;
        Ok(Some(canonical.entries))
    }

    /// The run-root-relative key of `path`, or the path text when it has none.
    fn key_or_text(&self, path: &Path) -> String {
        relative_key(&self.layout.run_root, path)
            .unwrap_or_else(|| path.to_string_lossy().into_owned())
    }

    /// Apply the object, undeclared-object and timestamp gates. Failures
    /// accumulate as causes; none of them stops the others.
    fn gates(&mut self, entries: &[CcEntry]) -> Result<(), CaptureError> {
        let after = list_files(&self.layout.run_root)?;
        let mut declared = BTreeSet::new();
        for entry in entries {
            if let Some(key) = relative_key(&self.layout.run_root, Path::new(&entry.output)) {
                declared.insert(key);
            }
        }
        let undeclared = undeclared_objects(&self.before, &after, &declared);
        if !undeclared.is_empty() {
            self.cause(
                "created_undeclared",
                format!("objects created but not declared by any compile command: {undeclared:?}"),
            );
            self.report.gates.created_undeclared = undeclared;
        }

        let stamped = timestamp_entries(entries)?;
        if !stamped.is_empty() {
            // Keys, not bare file names: two directories may hold a `t.c`.
            let keys: BTreeSet<String> = stamped
                .iter()
                .map(|entry| self.key_or_text(&Path::new(&entry.directory).join(&entry.file)))
                .collect();
            let keys: Vec<String> = keys.into_iter().collect();
            self.cause(
                "timestamp_macro",
                format!(
                    "sources use __TIMESTAMP__, which depends on file modification times: {keys:?}"
                ),
            );
            self.report.gates.timestamp_macro = keys;
        }

        let mut arch_ok = true;
        for entry in entries {
            let output = PathBuf::from(&entry.output);
            let key = self.key_or_text(&output);
            let Some(bytes) = self.read_or_refuse(&key)? else {
                arch_ok = false;
                continue;
            };
            let copy = self.layout.objects_dir.join(&key);
            if let Some(parent) = copy.parent() {
                fs::create_dir_all(parent).map_err(|source| io_error("create", parent, source))?;
            }
            fs::write(&copy, &bytes).map_err(|source| io_error("write", &copy, source))?;

            let header = check_elf_header(&bytes);
            let readelf_argv = vec![
                path_str(&self.ctx.tools.readelf)?.to_owned(),
                "-A".to_owned(),
                path_str(&output)?.to_owned(),
            ];
            let run_root = self.layout.run_root.clone();
            let mut attributes = None;
            match self.exec(
                &format!("readelf {key}"),
                "readelf",
                &readelf_argv,
                &run_root,
            )? {
                Outcome::Success(stdout) => {
                    attributes = Some(parse_readelf_a(&String::from_utf8_lossy(&stdout)));
                }
                Outcome::NotProven => self.not_proven = true,
                Outcome::Failed(..) => {}
            }
            let object_ok = header.is_ok()
                && attributes
                    .as_ref()
                    .is_some_and(|attrs| attrs.is_cortex_m4_hard_float());
            if !object_ok {
                arch_ok = false;
                let reason = match (&header, &attributes) {
                    (Err(error), _) => error.to_string(),
                    (Ok(()), Some(attrs)) => format!("build attributes {attrs:?}"),
                    (Ok(()), None) => "readelf -A did not run to completion".to_owned(),
                };
                self.cause(
                    "arch_gate",
                    format!("{key} is not a Cortex-M4 hard-float object: {reason}"),
                );
            }
            self.report.objects.insert(
                key,
                CapturedObject {
                    sha256: sha256_hex(&bytes),
                    arch_ok: object_ok,
                    opt_goal: attributes.and_then(|attrs| attrs.optimization_goals),
                    code_digest: code_digest(&bytes).ok(),
                },
            );
        }
        self.report.gates.arch_ok = arch_ok;
        Ok(())
    }

    /// Check every manifest entry against the captured build (D-23). A failed
    /// entry is the cause `entry_mismatch`; a check that timed out leaves the
    /// capture `not_proven`.
    fn check_entries(
        &mut self,
        target: &Target,
        canonical: &[CcEntry],
    ) -> Result<(), CaptureError> {
        let mut failed = Vec::new();
        for entry in &target.entry {
            let checked = check_entry(
                &self.cfg,
                &self.ctx.tools,
                &self.layout,
                &self.env,
                canonical,
                entry,
            )?;
            self.report.steps.extend(checked.steps);
            if checked.not_proven {
                self.not_proven = true;
            }
            if !checked.result.passed() {
                failed.push(format!(
                    "{}: {}",
                    entry.name,
                    checked.result.detail.as_deref().unwrap_or("check failed")
                ));
            }
            self.report
                .entries
                .insert(entry.name.clone(), checked.result);
        }
        if !failed.is_empty() {
            self.cause("entry_mismatch", failed.join(" | "));
        }
        Ok(())
    }

    /// Hash every file under the CMake build directory that is not left out by
    /// [`excluded_from_configure_hashes`]: the generated headers and other
    /// configure-time inputs of the compile commands (Pattern 7). The make driver
    /// is hashed after [`normalise_make_driver`] and recorded with its rule; every
    /// other file is hashed byte for byte.
    fn configure_files(&mut self) -> Result<(), CaptureError> {
        let declared: BTreeSet<String> = self.report.objects.keys().cloned().collect();
        let mut hashes = BTreeMap::new();
        let mut normalised = BTreeMap::new();
        // `list_files` never descends into a symlinked directory, but it would
        // start in one: check the walk root first.
        let listing = match walk_root(&self.layout.build) {
            Ok(true) => list_files(&self.layout.build)?,
            Ok(false) => BTreeSet::new(),
            Err(error @ RunReadError::NotRegular { .. }) => {
                self.refused("build", &error);
                BTreeSet::new()
            }
            Err(RunReadError::Io { path, source }) => {
                return Err(io_error("inspect", &path, source));
            }
        };
        for relative in listing {
            if excluded_from_configure_hashes(&relative, &declared, &self.before) {
                continue;
            }
            let key = format!("build/{relative}");
            let Some(bytes) = self.read_or_refuse(&key)? else {
                continue;
            };
            if relative == MAKE_DRIVER {
                normalised.insert(
                    key,
                    NormalisedFile {
                        rule: MAKE_DRIVER_RULE.to_owned(),
                        sha256: sha256_hex(&normalise_make_driver(&bytes)),
                    },
                );
            } else {
                hashes.insert(key, sha256_hex(&bytes));
            }
        }
        self.report.configure_files = hashes;
        self.report.normalised_configure_files = normalised;
        Ok(())
    }

    /// Hash the committed inputs of the target from `testbed/targets/<id>/` and
    /// compare them, file set and bytes, with the run directory's copy, which the
    /// build could have changed: any difference is the cause `inputs_changed`
    /// naming the files. The copy is read without following a symlink.
    fn check_inputs(&mut self, id: &str) -> Result<(), CaptureError> {
        let committed = self.ctx.testbed_dir.join("targets").join(id);
        let mut expected = BTreeMap::new();
        if committed.is_dir() {
            for key in list_files(&committed)? {
                let path = committed.join(&key);
                let bytes = fs::read(&path).map_err(|source| io_error("read", &path, source))?;
                expected.insert(key, sha256_hex(&bytes));
            }
        }
        let found = match walk_root(&self.layout.inputs) {
            Ok(true) => list_files(&self.layout.inputs)?,
            Ok(false) => BTreeSet::new(),
            Err(error @ RunReadError::NotRegular { .. }) => {
                self.refused("inputs", &error);
                self.report.inputs = expected;
                return Ok(());
            }
            Err(RunReadError::Io { path, source }) => {
                return Err(io_error("inspect", &path, source));
            }
        };
        let mut changed: Vec<String> = Vec::new();
        for (key, hash) in &expected {
            let run_key = format!("inputs/{key}");
            match read_run_file(&self.layout.run_root, &run_key) {
                Ok(bytes) if sha256_hex(&bytes) == *hash => {}
                Ok(_) => changed.push(format!("{key} differs")),
                Err(RunReadError::Io { source, .. })
                    if source.kind() == io::ErrorKind::NotFound =>
                {
                    changed.push(format!("{key} is missing"));
                }
                Err(error @ RunReadError::NotRegular { .. }) => self.refused(&run_key, &error),
                Err(RunReadError::Io { path, source }) => {
                    return Err(io_error("read", &path, source));
                }
            }
        }
        for key in found.difference(&expected.keys().cloned().collect()) {
            changed.push(format!("{key} was added"));
        }
        if !changed.is_empty() {
            self.cause(
                "inputs_changed",
                format!(
                    "the run copy of the committed inputs of {id} differs from testbed/targets/{id}: {}",
                    changed.join(", ")
                ),
            );
        }
        self.report.inputs = expected;
        Ok(())
    }

    /// The build record of the build stage, read without following a symlink and
    /// checked against this pair and the verified source.
    fn load_build_record(&self, id: &str, config: &str, source: &SourceRef) -> RecordState {
        let bytes = match read_run_file(&self.layout.run_root, &build_record_key()) {
            Ok(bytes) => bytes,
            Err(error) => return RecordState::Missing(error.to_string()),
        };
        let record: BuildRecord = match serde_json::from_slice(&bytes) {
            Ok(record) => record,
            Err(error) => {
                return RecordState::Missing(format!("the build record cannot be parsed: {error}"));
            }
        };
        if record.schema_version != BUILD_RECORD_SCHEMA_VERSION {
            return RecordState::Missing(format!(
                "the build record has schema_version {}, this tool reads {BUILD_RECORD_SCHEMA_VERSION}",
                record.schema_version
            ));
        }
        let different = |what: &str| {
            RecordState::Mismatch(format!(
                "the build record was made for a different {what} than this record stage"
            ))
        };
        if record.target != id {
            return different("target");
        }
        if record.config != config {
            return different("configuration");
        }
        if record.source != *source {
            return different("source");
        }
        RecordState::Accepted(Box::new(record))
    }
}

/// The run-root-relative key of the build record.
fn build_record_key() -> String {
    format!("{STAGE_DIR}/{BUILD_RECORD_FILE}")
}

/// What the record stage found where the build record should be.
enum RecordState {
    /// Missing, unreadable, refused (a symlink) or not a build record; the text
    /// says which.
    Missing(String),
    /// A build record of another pair or another source.
    Mismatch(String),
    /// A record for this pair and this verified source.
    Accepted(Box<BuildRecord>),
}

/// Empty `dir` in place, keeping the directory itself (it may be a bind mount):
/// create it when absent, replace a non-directory by one, remove every entry of
/// a directory (a symlink is removed, never followed).
fn empty_directory(dir: &Path) -> Result<(), CaptureError> {
    match fs::symlink_metadata(dir) {
        Err(source) if source.kind() == io::ErrorKind::NotFound => {
            return fs::create_dir_all(dir).map_err(|source| io_error("create", dir, source));
        }
        Err(source) => return Err(io_error("inspect", dir, source)),
        Ok(meta) if meta.is_dir() => {}
        Ok(_) => {
            fs::remove_file(dir).map_err(|source| io_error("remove", dir, source))?;
            return fs::create_dir_all(dir).map_err(|source| io_error("create", dir, source));
        }
    }
    let listing = fs::read_dir(dir).map_err(|source| io_error("read", dir, source))?;
    for entry in listing {
        let entry = entry.map_err(|source| io_error("read", dir, source))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|source| io_error("inspect", &path, source))?;
        if file_type.is_dir() {
            fs::remove_dir_all(&path).map_err(|source| io_error("remove", &path, source))?;
        } else {
            fs::remove_file(&path).map_err(|source| io_error("remove", &path, source))?;
        }
    }
    Ok(())
}

/// The checks every stage makes before it does anything: safe path components,
/// and a recipe. Returns the recipe.
fn pair_args<'a>(
    id: &str,
    target: &'a Target,
    config: &Config,
) -> Result<&'a BuildArgs, CaptureError> {
    if !is_safe_component(id) || !is_safe_component(&config.name) {
        return Err(invalid(format!(
            "target id {id:?} and configuration {:?} must be safe path components",
            config.name
        )));
    }
    target
        .build_args
        .as_ref()
        .ok_or_else(|| invalid(format!("target {id} has no build_args recipe")))
}

fn new_session<'a>(
    cfg: &RunnerConfig,
    ctx: &'a CaptureContext,
    id: &str,
    config: &Config,
) -> Result<Session<'a>, CaptureError> {
    let layout = RunLayout::new(&ctx.work_dir, id, &config.name);
    let home = ctx.work_dir.join("home");
    let tmp = ctx.work_dir.join("tmp");
    let env = BTreeMap::from([
        ("HOME".to_owned(), path_str(&home)?.to_owned()),
        ("TMPDIR".to_owned(), path_str(&tmp)?.to_owned()),
    ]);
    Ok(Session {
        cfg: RunnerConfig {
            timeout: ctx.timeout,
            ..cfg.clone()
        },
        ctx,
        layout,
        env,
        report: CaptureReport::new(id, &config.name),
        not_proven: false,
        build_failed: false,
        before: BTreeSet::new(),
    })
}

/// The build stage of a capture (BUILD-01, WR-04): run the untrusted upstream
/// build of one configuration of one target and leave its outputs, and a
/// [`BuildRecord`], in `<work>/run/<id>/<config>`.
///
/// The cached source is verified again first ([`verify_cached`], no network). An
/// existing run directory is kept and emptied (it may be a bind mount), the
/// verified source and the committed inputs are copied into it, and the build runs
/// there; the exporter's raw file and the CMake toolchain file are written under
/// `stage/`. Nothing is created, read or written under `<work>/capture`,
/// `<work>/reports` or an output directory. A failing build is data in the record
/// (`build_failed`, `not_proven`), not an error, and the record is written in every
/// such case.
///
/// # Errors
/// [`CaptureError`] for invalid inputs (an unsafe id or configuration name, a
/// target without a recipe), a cache that is missing or no longer matches its
/// pin, or a tool that cannot be started.
pub fn capture_build(
    cfg: &RunnerConfig,
    ctx: &CaptureContext,
    id: &str,
    target: &Target,
    config: &Config,
) -> Result<BuildRecord, CaptureError> {
    let args = pair_args(id, target, config)?;
    let mut session = new_session(cfg, ctx, id, config)?;
    let verified = session.verify(id, target)?;
    session.prepare_run(id, &verified)?;
    let flow = session.build(args, config, target.build)?;
    let record = BuildRecord {
        schema_version: BUILD_RECORD_SCHEMA_VERSION,
        target: id.to_owned(),
        config: config.name.clone(),
        source: SourceRef {
            commit: verified.commit,
            tree_sha256: verified.tree_sha256,
        },
        steps: session.report.steps,
        causes: session.report.causes,
        cause_details: session.report.cause_details,
        before: session.before,
        not_proven: session.not_proven,
        build_failed: session.build_failed,
        built: matches!(flow, Flow::Continue),
    };
    let path = session.layout.build_record_path();
    let mut text = serde_json::to_string_pretty(&record)?;
    text.push('\n');
    fs::write(&path, text).map_err(|source| io_error("write", &path, source))?;
    Ok(record)
}

/// The record stage of a capture (BUILD-01, WR-04): run no upstream code, read
/// what the build stage left in `<work>/run/<id>/<config>` as untrusted data and
/// write the capture artifacts to `<work>/capture/<id>/<config>`.
///
/// It creates `<work>/home` and `<work>/tmp` when they are missing, verifies the
/// source cache again, takes the tools from the pins, reads the build record
/// (missing, unreadable, refused or unparseable: `not_proven`, cause
/// `no_build_record`; for another pair or source: `not_proven`, cause
/// `build_record_mismatch`), compares the run directory's copy of the committed
/// inputs with `testbed/` (`inputs_changed`), recomputes the toolchain file, and
/// then canonicalises and gates the build's outputs, checks the manifest entries
/// and hashes the configure-time files exactly as a single-process capture does.
/// A run-directory read that is refused is the cause `run_dir_symlink`.
/// `capture.json` is written in every case.
///
/// # Errors
/// [`CaptureError`] for invalid inputs (an unsafe id or configuration name, a
/// target without a recipe), a cache that is missing or no longer matches its
/// pin, or a tool that cannot be started.
pub fn capture_record(
    cfg: &RunnerConfig,
    ctx: &CaptureContext,
    id: &str,
    target: &Target,
    config: &Config,
) -> Result<CaptureReport, CaptureError> {
    pair_args(id, target, config)?;
    let mut session = new_session(cfg, ctx, id, config)?;
    session.prepare_record()?;
    let verified = session.verify(id, target)?;
    let source = SourceRef {
        commit: verified.commit.clone(),
        tree_sha256: verified.tree_sha256.clone(),
    };
    session.report.source = source.clone();
    session.report.tools = reported_tools(&ctx.tools, target.build);
    match session.load_build_record(id, &config.name, &source) {
        RecordState::Missing(detail) => {
            session.not_proven = true;
            session.cause("no_build_record", detail);
        }
        RecordState::Mismatch(detail) => {
            session.not_proven = true;
            session.cause("build_record_mismatch", detail);
        }
        RecordState::Accepted(record) => {
            session.report.steps.extend(record.steps);
            session.report.causes = record.causes;
            session.report.cause_details = record.cause_details;
            session.before = record.before;
            session.not_proven = record.not_proven;
            session.build_failed = record.build_failed;
            session.check_inputs(id)?;
            if target.build == BuildSystem::Cmake {
                let flags = expand_flags(
                    &effective_cflags(&ctx.baseline, config),
                    &ctx.tools,
                    &session.layout,
                )?;
                let text = toolchain_file(&ctx.tools, &flags)?;
                session.report.toolchain_file_sha256 = Some(sha256_hex(text.as_bytes()));
                let path = session.layout.toolchain_copy_path();
                fs::write(&path, text).map_err(|source| io_error("write", &path, source))?;
            }
            if record.built
                && matches!(session.copy_raw_export()?, Flow::Continue)
                && let Some(entries) = session.canonicalise()?
            {
                session.gates(&entries)?;
                session.check_entries(target, &entries)?;
                if target.build == BuildSystem::Cmake {
                    session.configure_files()?;
                }
            }
        }
    }
    session.report.steps.extend(verified.steps);
    let mut report = session.report;
    report.verdict = if session.not_proven {
        CaptureVerdict::NotProven
    } else if session.build_failed {
        CaptureVerdict::BuildFailed
    } else if report.causes.is_empty() {
        CaptureVerdict::Captured
    } else {
        CaptureVerdict::CaptureFailed
    };
    let path = session.layout.report_path();
    fs::write(&path, report.to_json()?).map_err(|source| io_error("write", &path, source))?;
    Ok(report)
}

/// Capture one configuration of one target (BUILD-01): [`capture_build`] followed
/// by [`capture_record`] in one process.
///
/// The cached source is verified again first ([`verify_cached`], no network);
/// the configuration is built in a fresh copy under `<work>/run/<id>/<config>`
/// and its artifacts are written to `<work>/capture/<id>/<config>`. A failing
/// build or gate is the report's verdict (`build_failed`, `capture_failed`,
/// `not_proven`) with its causes, and `capture.json` is written in every case.
///
/// # Errors
/// [`CaptureError`] for invalid inputs (an unsafe id or configuration name, a
/// target without a recipe), a cache that is missing or no longer matches its
/// pin, or a tool that cannot be started.
pub fn capture(
    cfg: &RunnerConfig,
    ctx: &CaptureContext,
    id: &str,
    target: &Target,
    config: &Config,
) -> Result<CaptureReport, CaptureError> {
    capture_build(cfg, ctx, id, target, config)?;
    capture_record(cfg, ctx, id, target, config)
}
