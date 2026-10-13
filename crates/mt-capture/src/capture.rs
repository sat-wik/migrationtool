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

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::io;
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
    CaptureReport, CaptureVerdict, CapturedObject, CompileCommandsInfo, SourceRef, Step,
};
use crate::treehash::{TreeHashError, copy_tree, sha256_hex};

/// The most a single step may run before the capture reports `not_proven`.
const STEP_TIMEOUT: Duration = Duration::from_secs(900);

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

/// The make driver CMake generates at the top of the build directory: not an
/// input of any compile command, and not byte-stable ([`is_cmake_product`]).
const MAKE_DRIVER: &str = "CMakeFiles/Makefile2";

/// Whether a file of the CMake build directory (`relative` to it) is something
/// the build or CMake's own probes produced rather than an input of a compile
/// command, so it is left out of the configure-time file hashes.
///
/// Objects, static archives and shared libraries are outputs, and archives embed
/// member times. CMake's diagnostic logs hold scratch-directory names. The
/// compiler-ABI probe (`CMakeFiles/<version>/CMakeDetermineCompilerABI_C.bin`) is
/// the output of a test compile, and its bytes differ from run to run.
///
/// `CMakeFiles/Makefile2` is the make driver of the Unix Makefiles generator: it
/// schedules the targets and holds their prerequisite lists and progress counters.
/// No compile command reads it (a replay runs the captured argv, and the flags and
/// definitions of each object live in `flags.make`, which stays hashed), and its
/// bytes are not stable: CMake emits the prerequisite lines of equal-rank targets
/// in pointer order, so two configures of the same tree can differ only in the
/// order of two lines (zcbor's `app.dir/all` prerequisites, measured with plain
/// `cmake` runs and seen in testbed run B). That one path is left out; every other
/// generated file stays in the hashes, so a real change still shows.
#[must_use]
pub fn is_cmake_product(relative: &str) -> bool {
    if [".o", ".obj", ".a", ".so"]
        .iter()
        .any(|suffix| relative.ends_with(suffix))
        || CONFIGURE_LOGS.contains(&relative)
        || relative == MAKE_DRIVER
    {
        return true;
    }
    let mut parts = relative.split('/');
    let probe = parts.next_back().is_some_and(|name| {
        name.starts_with("CMakeDetermineCompilerABI_") && name.ends_with(".bin")
    });
    probe && relative.starts_with("CMakeFiles/")
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

    /// The exporter's file exactly as written: Bear's output or a copy of
    /// CMake's `compile_commands.json`.
    #[must_use]
    pub fn raw_path(&self) -> PathBuf {
        self.capture_dir.join("compile_commands.raw.json")
    }

    /// The canonical compile commands replay executes.
    #[must_use]
    pub fn canonical_path(&self) -> PathBuf {
        self.capture_dir.join("compile_commands.json")
    }

    /// The generated CMake toolchain file.
    #[must_use]
    pub fn toolchain_path(&self) -> PathBuf {
        self.capture_dir.join("toolchain.cmake")
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

    /// Verify the cache, lay out a clean run and capture directory, copy the
    /// source and the committed inputs.
    fn prepare(&mut self, id: &str, target: &Target) -> Result<(), CaptureError> {
        let ctx = self.ctx;
        // No curl, tar or protocol: verifying a cache touches only the files and
        // git's own object store.
        let fetch_ctx = FetchContext {
            work_dir: ctx.work_dir.clone(),
            git: ctx.tools.git.clone(),
            curl: PathBuf::new(),
            tar: PathBuf::from(TAR_PATH),
            allowed_protocols: String::new(),
            timeout: ctx.timeout,
        };
        let verified = verify_cached(&self.cfg, &fetch_ctx, id, target)?;
        self.report.source = SourceRef {
            commit: verified.commit.clone(),
            tree_sha256: verified.tree_sha256.clone(),
        };
        self.report.steps.extend(verified.steps);
        for key in REPORTED_TOOLS {
            // Bear serves Make targets and CMake serves CMake targets.
            let unused = matches!(
                (key, target.build),
                ("bear", BuildSystem::Cmake) | ("cmake", BuildSystem::Make)
            );
            if !unused && let Some(version) = ctx.tools.versions.get(key) {
                self.report.tools.insert(key.to_owned(), version.clone());
            }
        }

        for dir in [&self.layout.run_root, &self.layout.capture_dir] {
            if !dir.starts_with(&ctx.work_dir) || dir == &ctx.work_dir {
                return Err(invalid(format!(
                    "{} is not inside the work directory",
                    dir.display()
                )));
            }
            match fs::remove_dir_all(dir) {
                Ok(()) => {}
                Err(source) if source.kind() == io::ErrorKind::NotFound => {}
                Err(source) => return Err(io_error("remove", dir, source)),
            }
            fs::create_dir_all(dir).map_err(|source| io_error("create", dir, source))?;
        }
        for dir in [ctx.work_dir.join("home"), ctx.work_dir.join("tmp")] {
            fs::create_dir_all(&dir).map_err(|source| io_error("create", &dir, source))?;
        }

        let cache = cache_paths(&ctx.work_dir, id, &verified.commit);
        copy_tree(&cache.tree, &self.layout.src)?;
        let committed = ctx.testbed_dir.join("targets").join(id);
        if committed.is_dir() {
            copy_tree(&committed, &self.layout.inputs)?;
            for key in list_files(&self.layout.inputs)? {
                let path = self.layout.inputs.join(&key);
                let bytes = fs::read(&path).map_err(|source| io_error("read", &path, source))?;
                self.report.inputs.insert(key, sha256_hex(&bytes));
            }
        } else {
            fs::create_dir_all(&self.layout.inputs)
                .map_err(|source| io_error("create", &self.layout.inputs, source))?;
        }
        Ok(())
    }

    /// Run the build and leave the exporter's file at the raw path (if there is
    /// one).
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
                self.report.toolchain_file_sha256 = Some(sha256_hex(text.as_bytes()));
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

    /// Canonicalise the export and write the canonical file.
    fn canonicalise(&mut self) -> Result<Option<Vec<CcEntry>>, CaptureError> {
        let raw_path = self.layout.raw_path();
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
        let exists = |path: &Path| fs::symlink_metadata(path).is_ok_and(|meta| meta.is_file());
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
            let bytes = fs::read(&output).map_err(|source| io_error("read", &output, source))?;
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

    /// Hash every file under the CMake build directory that is not a build
    /// product ([`is_cmake_product`]): the generated headers and other
    /// configure-time inputs of the compile commands (Pattern 7).
    fn configure_files(&mut self) -> Result<(), CaptureError> {
        let declared: BTreeSet<&String> = self.report.objects.keys().collect();
        let mut hashes = BTreeMap::new();
        for relative in list_files(&self.layout.build)? {
            let key = format!("build/{relative}");
            if declared.contains(&key) || is_cmake_product(&relative) {
                continue;
            }
            let path = self.layout.build.join(&relative);
            let bytes = fs::read(&path).map_err(|source| io_error("read", &path, source))?;
            hashes.insert(key, sha256_hex(&bytes));
        }
        self.report.configure_files = hashes;
        Ok(())
    }
}

/// Capture one configuration of one target (BUILD-01).
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
    if !is_safe_component(id) || !is_safe_component(&config.name) {
        return Err(invalid(format!(
            "target id {id:?} and configuration {:?} must be safe path components",
            config.name
        )));
    }
    let args = target
        .build_args
        .as_ref()
        .ok_or_else(|| invalid(format!("target {id} has no build_args recipe")))?;
    let layout = RunLayout::new(&ctx.work_dir, id, &config.name);
    let home = ctx.work_dir.join("home");
    let tmp = ctx.work_dir.join("tmp");
    let env = BTreeMap::from([
        ("HOME".to_owned(), path_str(&home)?.to_owned()),
        ("TMPDIR".to_owned(), path_str(&tmp)?.to_owned()),
    ]);
    let mut session = Session {
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
    };
    session.prepare(id, target)?;
    if matches!(session.build(args, config, target.build)?, Flow::Continue)
        && let Some(entries) = session.canonicalise()?
    {
        session.gates(&entries)?;
        session.check_entries(target, &entries)?;
        if target.build == BuildSystem::Cmake {
            session.configure_files()?;
        }
    }
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
