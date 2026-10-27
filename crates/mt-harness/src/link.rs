//! Linking the generated driver against the exact objects Phase 2 captured and
//! replayed (CONTEXT D-01, D-19; RESEARCH Q3).
//!
//! The objects are never rebuilt. [`verify_objects`] recomputes every object's
//! sha256 and requires it to equal `capture.json`, so what is linked is what
//! was captured; [`archive`] puts exactly those files into a deterministic
//! archive; [`link_driver`] compiles the generated source with the defining
//! unit's include, define and ABI flags and links it with the archive. Every
//! program runs through the runner.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

use mt_capture::manifest::BASELINE_CFLAGS;
use mt_toolchain::runner::{self, RunOutput, RunRequest, RunnerConfig};

use crate::driver::DriverBuild;
use crate::tools::ExecTools;
use crate::{HarnessError, sha256_hex};

/// The largest `capture.json` or object file read.
const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;

/// The most stderr text kept in a build error.
const STDERR_EXCERPT: usize = 2000;

/// The objects of a pair, each checked against `capture.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedObjects {
    /// The toolchain image the capture recorded (`local` for a local run).
    pub image: String,
    /// `<capture dir>/objects`.
    pub objects_dir: PathBuf,
    /// sha256 of each object, by its path relative to `objects_dir`.
    pub objects: BTreeMap<String, String>,
}

fn capture_error(capture: &Path, detail: impl Into<String>) -> HarnessError {
    HarnessError::Capture {
        path: capture.to_path_buf(),
        detail: detail.into(),
    }
}

fn io(action: &'static str, path: &Path, source: std::io::Error) -> HarnessError {
    HarnessError::Io {
        action,
        path: path.to_path_buf(),
        source,
    }
}

/// Read a regular file of at most [`MAX_FILE_BYTES`].
fn read_regular(path: &Path) -> Result<Vec<u8>, HarnessError> {
    let metadata = std::fs::symlink_metadata(path).map_err(|e| io("cannot stat", path, e))?;
    if !metadata.is_file() || metadata.len() > MAX_FILE_BYTES {
        return Err(HarnessError::Capture {
            path: path.to_path_buf(),
            detail: "is not a regular file of at most 64 MiB".to_owned(),
        });
    }
    std::fs::read(path).map_err(|e| io("cannot read", path, e))
}

/// A path relative to the objects directory with only plain components.
fn is_plain_relative(path: &str) -> bool {
    !path.is_empty()
        && !path.contains('\0')
        && Path::new(path)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

/// Check the pair's objects against its `capture.json` (CONTEXT D-19).
///
/// The capture's verdict must be `captured`; every recorded object must be a
/// regular file, reached through no symbolic link, whose recomputed sha256
/// equals the recorded one.
///
/// # Errors
/// [`HarnessError::Capture`] for a verdict other than `captured`, an unreadable
/// or malformed `capture.json`, an object path that leaves the directory, a
/// symbolic link or a file that is not regular; [`HarnessError::ObjectMismatch`]
/// naming the object and both hashes.
pub fn verify_objects(capture_dir: &Path) -> Result<VerifiedObjects, HarnessError> {
    let json_path = capture_dir.join("capture.json");
    let text = read_regular(&json_path)?;
    let json: serde_json::Value = serde_json::from_slice(&text)
        .map_err(|e| capture_error(capture_dir, format!("capture.json is not JSON: {e}")))?;
    let verdict = json
        .get("verdict")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| capture_error(capture_dir, "capture.json has no verdict"))?;
    if verdict != "captured" {
        return Err(capture_error(
            capture_dir,
            format!("the capture verdict is {verdict}; only captured objects may be linked"),
        ));
    }
    let image = json
        .get("image")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| capture_error(capture_dir, "capture.json has no image"))?
        .to_owned();
    let recorded = json
        .get("objects")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| capture_error(capture_dir, "capture.json has no objects"))?;
    if recorded.is_empty() {
        return Err(capture_error(capture_dir, "capture.json records no object"));
    }
    let objects_dir = capture_dir.join("objects");
    let mut objects = BTreeMap::new();
    for (relative, record) in recorded {
        if !is_plain_relative(relative) {
            return Err(capture_error(
                capture_dir,
                format!("object path {relative:?} is not a plain relative path"),
            ));
        }
        let expected = record
            .get("sha256")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                capture_error(capture_dir, format!("object {relative} has no sha256"))
            })?;
        // Neither the file nor any directory above it may be a symbolic link.
        let mut walked = objects_dir.clone();
        for component in Path::new(relative).components() {
            walked.push(component);
            let metadata =
                std::fs::symlink_metadata(&walked).map_err(|e| io("cannot stat", &walked, e))?;
            if metadata.file_type().is_symlink() {
                return Err(capture_error(
                    capture_dir,
                    format!(
                        "object {relative} is reached through a symlink at {}",
                        walked.display()
                    ),
                ));
            }
        }
        let bytes = read_regular(&objects_dir.join(relative))?;
        let actual = sha256_hex(&bytes);
        if actual != expected {
            return Err(HarnessError::ObjectMismatch {
                capture: capture_dir.to_path_buf(),
                object: relative.clone(),
                expected: expected.to_owned(),
                actual,
            });
        }
        objects.insert(relative.clone(), actual);
    }
    Ok(VerifiedObjects {
        image,
        objects_dir,
        objects,
    })
}

/// The archive of a pair's objects.
#[derive(Debug, Clone)]
pub struct ArchiveBuild {
    /// The archive file.
    pub path: PathBuf,
    /// Its sha256.
    pub sha256: String,
    /// The run record of the `ar` step.
    pub record: runner::RunRecord,
}

fn healthy(output: &RunOutput) -> bool {
    let record = &output.record;
    record.exit.code == Some(0)
        && !record.exit.timed_out
        && !record.exit.tree_kill_sent
        && record.stdout.complete
        && record.stderr.complete
}

fn excerpt(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .chars()
        .take(STDERR_EXCERPT)
        .collect()
}

fn failure(step: &str, output: &RunOutput) -> HarnessError {
    let record = &output.record;
    HarnessError::Build {
        step: step.to_owned(),
        detail: format!(
            "exit code {:?}, signal {:?}, timed out {}; stderr: {}",
            record.exit.code,
            record.exit.signal,
            record.exit.timed_out,
            excerpt(&output.stderr)
        ),
    }
}

/// Put `members` (paths relative to `objects_dir`) into a deterministic archive
/// at `out`: `ar qcsD` with the members sorted, run in `objects_dir` through the
/// runner. `q` appends without replacing, so two members with one base name both
/// stay.
///
/// # Errors
/// [`HarnessError::Io`], [`HarnessError::Runner`] or [`HarnessError::Build`] when
/// `ar` cannot run or fails.
pub fn archive(
    tools: &ExecTools,
    cfg: &RunnerConfig,
    objects_dir: &Path,
    members: &[String],
    out: &Path,
) -> Result<ArchiveBuild, HarnessError> {
    let mut sorted: Vec<&String> = members.iter().collect();
    sorted.sort();
    if out.exists() {
        std::fs::remove_file(out).map_err(|e| io("cannot remove", out, e))?;
    }
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).map_err(|e| io("cannot create", parent, e))?;
    }
    let mut argv: Vec<OsString> = vec![
        tools.ar.clone().into_os_string(),
        OsString::from("qcsD"),
        out.as_os_str().to_owned(),
    ];
    argv.extend(sorted.iter().map(|member| OsString::from(member.as_str())));
    let output = runner::run(
        cfg,
        &RunRequest {
            tool_name: "arm_ar".to_owned(),
            argv,
            cwd: objects_dir.to_path_buf(),
            extra_env: BTreeMap::new(),
            tool_version: None,
        },
    )?;
    if !healthy(&output) {
        return Err(failure("arm-none-eabi-ar of pair.a", &output));
    }
    let bytes = std::fs::read(out).map_err(|e| io("cannot read", out, e))?;
    Ok(ArchiveBuild {
        path: out.to_path_buf(),
        sha256: sha256_hex(&bytes),
        record: output.record,
    })
}

/// The names the linker reported as undefined, sorted and without repeats. Both
/// the ASCII quotes of the C locale and the typographic quotes of a UTF-8
/// locale are read.
#[must_use]
pub fn unresolved_symbols(stderr: &str) -> Vec<String> {
    const MARKER: &str = "undefined reference to ";
    let mut names = std::collections::BTreeSet::new();
    for line in stderr.lines() {
        let Some(at) = line.find(MARKER) else {
            continue;
        };
        let rest = &line[at + MARKER.len()..];
        let name: String = rest
            .trim_start_matches(['`', '\'', '\u{2018}'])
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '.' || *c == '$')
            .collect();
        if !name.is_empty() {
            names.insert(name);
        }
    }
    names.into_iter().collect()
}

/// What to compile and link.
#[derive(Debug, Clone)]
pub struct LinkRequest<'a> {
    /// The build directory: `driver.c`, `driver.o` and `driver.elf` are written
    /// here.
    pub dir: &'a Path,
    /// The generated driver source.
    pub source: &'a str,
    /// The defining unit's include, define and ABI flags ([`crate::adapter::driver_flags`]).
    pub flags: &'a [String],
    /// The working directory of the compile: the defining unit's directory, so
    /// relative `-I` paths resolve as they did.
    pub compile_cwd: &'a Path,
    /// The archive of the pair's objects.
    pub archive: &'a Path,
}

/// How a link ended.
#[derive(Debug, Clone)]
pub enum LinkOutcome {
    /// `driver.elf` exists.
    Linked(Box<DriverBuild>),
    /// The linker found these symbols undefined (sorted).
    Unresolved(Vec<String>),
}

fn gcc_request(tools: &ExecTools, argv: Vec<OsString>, cwd: &Path) -> RunRequest {
    RunRequest {
        tool_name: "arm_gcc".to_owned(),
        argv,
        cwd: cwd.to_path_buf(),
        extra_env: BTreeMap::new(),
        tool_version: tools.versions.get("arm_gcc").cloned(),
    }
}

/// Compile the driver source and link it with the archive: two runner steps,
/// the pinned compiler with the baseline flags and `-std=gnu11` for the compile,
/// and `-nostartfiles --specs=nosys.specs ... -lm` for the link (never
/// `--specs=linux.specs`, never `--gc-sections`).
///
/// # Errors
/// [`HarnessError::Io`], [`HarnessError::Runner`] and [`HarnessError::Build`]
/// when the compile fails, or when the link fails for a reason other than
/// undefined symbols.
pub fn link_driver(
    tools: &ExecTools,
    cfg: &RunnerConfig,
    request: &LinkRequest<'_>,
) -> Result<LinkOutcome, HarnessError> {
    let dir = request.dir;
    std::fs::create_dir_all(dir).map_err(|e| io("cannot create", dir, e))?;
    let source_path = dir.join("driver.c");
    let object_path = dir.join("driver.o");
    let elf_path = dir.join("driver.elf");
    std::fs::write(&source_path, request.source)
        .map_err(|e| io("cannot write", &source_path, e))?;
    // A stale binary must never be mistaken for the result of this build.
    for stale in [&object_path, &elf_path] {
        if stale.exists() {
            std::fs::remove_file(stale).map_err(|e| io("cannot remove", stale, e))?;
        }
    }

    let mut compile: Vec<OsString> = vec![tools.arm_gcc.clone().into_os_string()];
    compile.extend(BASELINE_CFLAGS.iter().map(OsString::from));
    compile.extend(request.flags.iter().map(OsString::from));
    compile.push(OsString::from("-std=gnu11"));
    compile.push(OsString::from("-c"));
    compile.push(source_path.as_os_str().to_owned());
    compile.push(OsString::from("-o"));
    compile.push(object_path.as_os_str().to_owned());
    let compiled = runner::run(cfg, &gcc_request(tools, compile, request.compile_cwd))?;
    if !healthy(&compiled) {
        return Err(failure("arm-none-eabi-gcc compile of driver.c", &compiled));
    }

    let mut link: Vec<OsString> = vec![tools.arm_gcc.clone().into_os_string()];
    link.extend(BASELINE_CFLAGS.iter().map(OsString::from));
    for flag in ["-nostartfiles", "--specs=nosys.specs", "-o"] {
        link.push(OsString::from(flag));
    }
    link.push(elf_path.as_os_str().to_owned());
    link.push(object_path.as_os_str().to_owned());
    link.push(request.archive.as_os_str().to_owned());
    link.push(OsString::from("-lm"));
    let linked = runner::run(cfg, &gcc_request(tools, link, dir))?;
    if !healthy(&linked) {
        let names = unresolved_symbols(&String::from_utf8_lossy(&linked.stderr));
        if !names.is_empty() && linked.record.exit.code == Some(1) {
            return Ok(LinkOutcome::Unresolved(names));
        }
        return Err(failure("arm-none-eabi-gcc link of driver.elf", &linked));
    }
    let elf = std::fs::read(&elf_path).map_err(|e| io("cannot read", &elf_path, e))?;
    Ok(LinkOutcome::Linked(Box::new(DriverBuild {
        dir: dir.to_path_buf(),
        source_path,
        elf_path,
        source_sha256: sha256_hex(request.source.as_bytes()),
        elf_sha256: sha256_hex(&elf),
        records: vec![compiled.record, linked.record],
    })))
}
