//! Fetch a test-bed source by commit or by archive, with a verified cache
//! (CONTEXT D-17, D-18, D-19).
//!
//! A source enters the work directory in exactly two ways: a shallow fetch of
//! one pinned commit, or a pinned release archive. Either way nothing is
//! accepted until the commit (git) or the archive digest (archive) and the tree
//! digest agree with the manifest, and the same checks run again every time the
//! cache is reused. A cache that no longer matches is an error that names the
//! path, never a silent refetch.
//!
//! Every subprocess (git, curl, tar) runs through the `mt-toolchain` runner with
//! an explicit environment. Nothing here writes outside the work directory.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use mt_toolchain::fetch::curl_argv;
use mt_toolchain::pins::{Pins, ToolPinStatus};
use mt_toolchain::runner::{RunRequest, RunnerConfig, RunnerError, run};

use crate::manifest::{Source, Target};
use crate::report::{FetchReport, REPORT_SCHEMA_VERSION, Step};
use crate::treehash::{TreeHashError, sha256_hex, tree_sha256};

/// The tar in the Debian base image (it is not a pinned tool of its own).
pub const TAR_PATH: &str = "/usr/bin/tar";

/// The most standard error text kept in an error message.
const STDERR_EXCERPT: usize = 400;

/// Bound on the output curl and tar may store; both write to files, so their
/// streams hold diagnostics only.
const DIAGNOSTIC_CAP: usize = 64 * 1024;

/// Everything a fetch needs besides the runner configuration.
#[derive(Debug, Clone)]
pub struct FetchContext {
    /// The work directory (`.mt-work` or `$MT_WORK_DIR`; `/work` in the image).
    pub work_dir: PathBuf,
    /// The pinned git.
    pub git: PathBuf,
    /// The pinned curl.
    pub curl: PathBuf,
    /// tar, [`TAR_PATH`] in the image.
    pub tar: PathBuf,
    /// Value of `GIT_ALLOW_PROTOCOL`: `https`, and `file` only in tests.
    pub allowed_protocols: String,
    /// Wall-clock limit of each step: 600 s.
    pub timeout: Duration,
}

/// Where a fetched source lives inside the work directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachePaths {
    /// `<work>/cache/<id>/<commit>`.
    pub dir: PathBuf,
    /// `<dir>/tree`, the verified source tree (with its `.git` for git sources).
    pub tree: PathBuf,
    /// `<dir>/archive.tar.bz2`, kept so a reuse can check its digest.
    pub archive: PathBuf,
    /// `<dir>/fetch.json`, the report of the fetch that created the tree.
    pub report: PathBuf,
}

/// Why a source could not be fetched or trusted.
#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    /// An identifier, URL, commit or digest that cannot be used safely.
    #[error("invalid fetch input: {0}")]
    InvalidInput(String),
    /// A tool the fetch needs is not pinned or not installed.
    #[error("the pins have no installed tool {0:?}")]
    ToolMissing(String),
    /// The runner could not start or supervise a tool.
    #[error(transparent)]
    Runner(#[from] RunnerError),
    /// A step exceeded its timeout; nothing is proven.
    #[error("{step} timed out; the fetch is not proven")]
    TimedOut {
        /// The step name.
        step: String,
    },
    /// A step's output did not reach end-of-file; nothing is proven.
    #[error("{step} left an incomplete output stream; the fetch is not proven")]
    Incomplete {
        /// The step name.
        step: String,
    },
    /// A step exited non-zero.
    #[error("{step} failed (exit code {exit_code:?}): {stderr}")]
    Failed {
        /// The step name.
        step: String,
        /// Its exit code, `None` when a signal ended it.
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
    /// The cache no longer matches its pin; it is not replaced silently.
    #[error("the cache at {} does not match its pin: {detail}", path.display())]
    CacheMismatch {
        /// The cached tree or archive.
        path: PathBuf,
        /// What differs.
        detail: String,
    },
    /// A tree could not be hashed.
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
    /// The report could not be serialised.
    #[error("cannot serialise the fetch report: {0}")]
    Json(#[from] serde_json::Error),
}

fn io_error(action: &'static str, path: &Path, source: io::Error) -> FetchError {
    FetchError::Io {
        action,
        path: path.to_path_buf(),
        source,
    }
}

fn invalid(message: impl Into<String>) -> FetchError {
    FetchError::InvalidInput(message.into())
}

pub(crate) fn pinned_tool(pins: &Pins, key: &str) -> Result<PathBuf, FetchError> {
    match pins.tool.get(key) {
        Some(spec) if spec.status == ToolPinStatus::Installed && !spec.bin.is_empty() => {
            Ok(PathBuf::from(&spec.bin))
        }
        _ => Err(FetchError::ToolMissing(key.to_owned())),
    }
}

impl FetchContext {
    /// The context for `work_dir`: git and curl from the pins, tar from the
    /// base image, only `https` allowed, a 600 s timeout per step.
    ///
    /// # Errors
    /// [`FetchError::ToolMissing`] when git or curl is not an installed pin.
    pub fn from_pins(pins: &Pins, work_dir: &Path) -> Result<Self, FetchError> {
        Ok(Self {
            work_dir: work_dir.to_path_buf(),
            git: pinned_tool(pins, "git")?,
            curl: pinned_tool(pins, "curl")?,
            tar: PathBuf::from(TAR_PATH),
            allowed_protocols: "https".to_owned(),
            timeout: Duration::from_secs(600),
        })
    }
}

/// The cache paths of a source: `<work>/cache/<id>/<commit>/{tree,
/// archive.tar.bz2, fetch.json}`.
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

/// A single path component made of letters, digits, `.`, `_` and `-`, not
/// starting with `.` or `-`.
pub(crate) fn is_safe_component(text: &str) -> bool {
    !text.is_empty()
        && !text.starts_with(['.', '-'])
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

fn is_hex(text: &str, length: usize) -> bool {
    text.len() == length && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

fn check_url(url: &str) -> Result<(), FetchError> {
    let scheme_ok = url.starts_with("https://") || url.starts_with("file://");
    if !scheme_ok || url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(invalid(format!(
            "url {url:?} must be https:// or file:// without whitespace"
        )));
    }
    Ok(())
}

/// The fields of a source that fetch and verify use, checked once.
struct Pin<'a> {
    url: &'a str,
    commit: &'a str,
    tree_sha256: &'a str,
    /// `(sha256, strip_components)` of an archive source.
    archive: Option<(&'a str, u32)>,
}

/// What the manifest pins for a target's source: the commit that names its cache
/// (the tag commit, for provenance, on an archive source) and the tree sha256.
///
/// Both are validated as 40 and 64 lowercase hex digits, so the commit is always
/// a safe path component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinnedSource {
    /// The pinned commit.
    pub commit: String,
    /// The pinned tree sha256.
    pub tree_sha256: String,
}

/// The manifest's pin of a target's source, validated.
///
/// # Errors
/// [`FetchError::InvalidInput`] when the id is not a safe path component, the
/// url is not `https://` or `file://`, or a digest is not lowercase hex of the
/// right length.
pub fn pinned_source(id: &str, target: &Target) -> Result<PinnedSource, FetchError> {
    let pin = pin_of(id, target)?;
    Ok(PinnedSource {
        commit: pin.commit.to_owned(),
        tree_sha256: pin.tree_sha256.to_owned(),
    })
}

fn pin_of<'a>(id: &str, target: &'a Target) -> Result<Pin<'a>, FetchError> {
    if !is_safe_component(id) {
        return Err(invalid(format!(
            "target id {id:?} is not a safe path component"
        )));
    }
    let pin = match &target.source {
        Source::Git {
            url,
            commit,
            tree_sha256,
            ..
        } => Pin {
            url,
            commit,
            tree_sha256,
            archive: None,
        },
        Source::Archive {
            url,
            sha256,
            strip_components,
            commit,
            tree_sha256,
            ..
        } => Pin {
            url,
            commit,
            tree_sha256,
            archive: Some((sha256, *strip_components)),
        },
    };
    check_url(pin.url)?;
    if !is_hex(pin.commit, 40) {
        return Err(invalid(format!(
            "commit {:?} must be 40 lowercase hex digits",
            pin.commit
        )));
    }
    if !is_hex(pin.tree_sha256, 64) {
        return Err(invalid("the tree sha256 must be 64 lowercase hex digits"));
    }
    if let Some((sha256, _)) = pin.archive
        && !is_hex(sha256, 64)
    {
        return Err(invalid(
            "the archive sha256 must be 64 lowercase hex digits",
        ));
    }
    Ok(pin)
}

fn excerpt(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    let trimmed = text.trim();
    match trimmed.char_indices().nth(STDERR_EXCERPT) {
        Some((end, _)) => format!("{}...", &trimmed[..end]),
        None => trimmed.to_owned(),
    }
}

/// Runs the steps of one fetch with one working directory and environment, and
/// keeps their run records.
struct Stepper {
    cfg: RunnerConfig,
    cwd: PathBuf,
    env: BTreeMap<String, String>,
    steps: Vec<Step>,
}

impl Stepper {
    fn new(
        cfg: &RunnerConfig,
        ctx: &FetchContext,
        cwd: &Path,
        git: bool,
    ) -> Result<Self, FetchError> {
        let home = ctx.work_dir.join("home");
        let tmp = ctx.work_dir.join("tmp");
        for dir in [&home, &tmp] {
            fs::create_dir_all(dir).map_err(|source| io_error("create", dir, source))?;
        }
        let text = |path: &Path| path.to_string_lossy().into_owned();
        let mut env = BTreeMap::new();
        env.insert("HOME".to_owned(), text(&home));
        env.insert("TMPDIR".to_owned(), text(&tmp));
        if git {
            env.insert("GIT_TERMINAL_PROMPT".to_owned(), "0".to_owned());
            env.insert("GIT_CONFIG_NOSYSTEM".to_owned(), "1".to_owned());
            env.insert("GIT_CONFIG_GLOBAL".to_owned(), "/dev/null".to_owned());
            env.insert(
                "GIT_ALLOW_PROTOCOL".to_owned(),
                ctx.allowed_protocols.clone(),
            );
        }
        Ok(Self {
            cfg: RunnerConfig {
                timeout: ctx.timeout,
                ..cfg.clone()
            },
            cwd: cwd.to_path_buf(),
            env,
            steps: Vec::new(),
        })
    }

    /// Run one step. A timeout, an incomplete stream or a non-zero exit is an
    /// error; the step's record is kept either way.
    fn run(
        &mut self,
        name: &str,
        tool_name: &str,
        argv: Vec<OsString>,
        cap: Option<usize>,
    ) -> Result<Vec<u8>, FetchError> {
        let mut cfg = self.cfg.clone();
        if let Some(cap) = cap {
            cfg.output_cap = cap;
        }
        let request = RunRequest {
            tool_name: tool_name.to_owned(),
            argv,
            cwd: self.cwd.clone(),
            extra_env: self.env.clone(),
            tool_version: None,
        };
        let output = run(&cfg, &request)?;
        let exit = output.record.exit.clone();
        let complete = output.record.stdout.complete && output.record.stderr.complete;
        self.steps.push(Step {
            name: name.to_owned(),
            run: output.record,
        });
        if exit.timed_out {
            return Err(FetchError::TimedOut {
                step: name.to_owned(),
            });
        }
        if !complete {
            return Err(FetchError::Incomplete {
                step: name.to_owned(),
            });
        }
        if exit.code != Some(0) {
            return Err(FetchError::Failed {
                step: name.to_owned(),
                exit_code: exit.code,
                stderr: excerpt(&output.stderr),
            });
        }
        Ok(output.stdout)
    }
}

fn os(text: &str) -> OsString {
    OsString::from(text)
}

fn remove_dir_if_present(path: &Path) -> Result<(), FetchError> {
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(io_error("remove", path, source)),
    }
}

fn remove_file_if_present(path: &Path) -> Result<(), FetchError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(io_error("remove", path, source)),
    }
}

fn report_of(id: &str, pin: &Pin<'_>, reused: bool, steps: Vec<Step>) -> FetchReport {
    FetchReport {
        schema_version: REPORT_SCHEMA_VERSION,
        kind: "fetch".to_owned(),
        target: id.to_owned(),
        source_kind: if pin.archive.is_some() {
            "archive"
        } else {
            "git"
        }
        .to_owned(),
        commit: pin.commit.to_owned(),
        tree_sha256: pin.tree_sha256.to_owned(),
        archive_sha256: pin.archive.map(|(sha256, _)| sha256.to_owned()),
        reused,
        steps,
    }
}

/// Fetch the source of `target` into `<work>/cache/<id>/<commit>/tree` and
/// verify it (D-17).
///
/// A git source is initialised empty, fetched shallowly by commit with no tags
/// and no submodules, checked out detached, and accepted only when
/// `git rev-parse HEAD` equals the pin and the tree sha256 equals the manifest.
/// An archive source is downloaded with the hardened curl flags of
/// `mt_toolchain::fetch::curl_argv`, its sha256 is checked before anything is
/// extracted, and the extracted tree must match the manifest. Only then is the
/// tree moved to `tree`. When `tree` already exists the cache is verified again
/// instead ([`verify_cached`]) and reported as `reused`.
///
/// # Errors
/// [`FetchError`]: invalid input, a failed, timed-out or incomplete step, a
/// commit, tree or archive mismatch, or a cache that no longer matches.
pub fn fetch(
    cfg: &RunnerConfig,
    ctx: &FetchContext,
    id: &str,
    target: &Target,
) -> Result<FetchReport, FetchError> {
    let pin = pin_of(id, target)?;
    let paths = cache_paths(&ctx.work_dir, id, pin.commit);
    match fs::symlink_metadata(&paths.tree) {
        Ok(_) => return verify_cached(cfg, ctx, id, target),
        Err(source) if source.kind() == io::ErrorKind::NotFound => {}
        Err(source) => return Err(io_error("inspect", &paths.tree, source)),
    }
    fs::create_dir_all(&paths.dir).map_err(|source| io_error("create", &paths.dir, source))?;
    let partial = paths.dir.join("tree.partial");
    remove_dir_if_present(&partial)?;
    let mut stepper = Stepper::new(cfg, ctx, &paths.dir, pin.archive.is_none())?;
    let outcome = match pin.archive {
        None => fetch_git(ctx, &mut stepper, &pin, &partial),
        Some((sha256, strip)) => {
            fetch_archive(ctx, &mut stepper, &pin, sha256, strip, &paths, &partial)
        }
    };
    if let Err(error) = outcome {
        // Best effort: a rejected tree must not stay where a later run could
        // mistake it for a verified one.
        let _ = fs::remove_dir_all(&partial);
        if pin.archive.is_some() {
            let _ = fs::remove_file(&paths.archive);
        }
        return Err(error);
    }
    fs::rename(&partial, &paths.tree).map_err(|source| io_error("rename", &partial, source))?;
    let report = report_of(id, &pin, false, stepper.steps);
    fs::write(&paths.report, report.to_json()?)
        .map_err(|source| io_error("write", &paths.report, source))?;
    Ok(report)
}

fn check_tree(partial: &Path, expected: &str) -> Result<(), FetchError> {
    let found = tree_sha256(partial)?;
    if found != expected {
        return Err(FetchError::TreeHashMismatch {
            expected: expected.to_owned(),
            found,
        });
    }
    Ok(())
}

fn fetch_git(
    ctx: &FetchContext,
    stepper: &mut Stepper,
    pin: &Pin<'_>,
    partial: &Path,
) -> Result<(), FetchError> {
    let git = ctx.git.clone().into_os_string();
    let partial_arg = partial.as_os_str().to_owned();
    stepper.run(
        "git init",
        "git",
        vec![git.clone(), os("init"), os("--quiet"), partial_arg.clone()],
        None,
    )?;
    stepper.run(
        "git fetch",
        "git",
        vec![
            git.clone(),
            os("-C"),
            partial_arg.clone(),
            os("fetch"),
            os("--quiet"),
            os("--depth"),
            os("1"),
            os("--no-tags"),
            os("--no-recurse-submodules"),
            os(pin.url),
            os(pin.commit),
        ],
        None,
    )?;
    stepper.run(
        "git checkout",
        "git",
        vec![
            git.clone(),
            os("-C"),
            partial_arg.clone(),
            os("checkout"),
            os("--quiet"),
            os("--detach"),
            os("FETCH_HEAD"),
        ],
        None,
    )?;
    let head = stepper.run(
        "git rev-parse",
        "git",
        vec![git, os("-C"), partial_arg, os("rev-parse"), os("HEAD")],
        None,
    )?;
    let found = String::from_utf8_lossy(&head).trim().to_owned();
    if found != pin.commit {
        return Err(FetchError::CommitMismatch {
            expected: pin.commit.to_owned(),
            found,
        });
    }
    check_tree(partial, pin.tree_sha256)
}

fn fetch_archive(
    ctx: &FetchContext,
    stepper: &mut Stepper,
    pin: &Pin<'_>,
    expected_sha256: &str,
    strip_components: u32,
    paths: &CachePaths,
    partial: &Path,
) -> Result<(), FetchError> {
    let download = paths.dir.join("archive.tar.bz2.partial");
    remove_file_if_present(&download)?;
    let mut argv: Vec<OsString> = curl_argv(pin.url).iter().map(|a| os(a)).collect();
    // curl_argv names the program "curl"; the pinned absolute path replaces it.
    argv[0] = ctx.curl.clone().into_os_string();
    argv.push(os("-o"));
    argv.push(download.as_os_str().to_owned());
    let fetched = stepper.run("curl", "curl", argv, Some(DIAGNOSTIC_CAP));
    let verified = fetched.and_then(|_| {
        let bytes = fs::read(&download).map_err(|source| io_error("read", &download, source))?;
        let found = sha256_hex(&bytes);
        if found == expected_sha256 {
            Ok(())
        } else {
            Err(FetchError::ArchiveMismatch {
                expected: expected_sha256.to_owned(),
                found,
            })
        }
    });
    if let Err(error) = verified {
        let _ = fs::remove_file(&download);
        return Err(error);
    }
    // Only a verified archive is extracted, and only into a fresh directory.
    fs::rename(&download, &paths.archive)
        .map_err(|source| io_error("rename", &download, source))?;
    fs::create_dir_all(partial).map_err(|source| io_error("create", partial, source))?;
    stepper.run(
        "tar",
        "tar",
        vec![
            ctx.tar.clone().into_os_string(),
            os("-xjf"),
            paths.archive.as_os_str().to_owned(),
            OsString::from(format!("--strip-components={strip_components}")),
            os("--no-same-owner"),
            os("-C"),
            partial.as_os_str().to_owned(),
        ],
        Some(DIAGNOSTIC_CAP),
    )?;
    check_tree(partial, pin.tree_sha256)
}

fn mismatch(path: &Path, detail: impl Into<String>) -> FetchError {
    FetchError::CacheMismatch {
        path: path.to_path_buf(),
        detail: detail.into(),
    }
}

/// Verify a cached source again, with no network (D-17, D-18).
///
/// A git source needs `git rev-parse HEAD` in the cached tree to equal the pin;
/// an archive source needs the cached archive to have the pinned sha256; both
/// need the tree sha256 to equal the manifest. Any miss is
/// [`FetchError::CacheMismatch`] naming the path. The cache is never repaired or
/// replaced here: whoever tampered with it has to be looked at.
///
/// # Errors
/// [`FetchError`]: invalid input, a missing or mismatching cache, or a failed,
/// timed-out or incomplete git step.
pub fn verify_cached(
    cfg: &RunnerConfig,
    ctx: &FetchContext,
    id: &str,
    target: &Target,
) -> Result<FetchReport, FetchError> {
    let pin = pin_of(id, target)?;
    let paths = cache_paths(&ctx.work_dir, id, pin.commit);
    match fs::symlink_metadata(&paths.tree) {
        Ok(meta) if meta.is_dir() => {}
        Ok(_) => return Err(mismatch(&paths.tree, "the cached tree is not a directory")),
        Err(source) if source.kind() == io::ErrorKind::NotFound => {
            return Err(mismatch(&paths.tree, "the cached tree is missing"));
        }
        Err(source) => return Err(io_error("inspect", &paths.tree, source)),
    }
    let mut stepper = Stepper::new(cfg, ctx, &paths.dir, pin.archive.is_none())?;
    match pin.archive {
        None => verify_cached_commit(ctx, &mut stepper, &pin, &paths)?,
        Some((expected, _)) => {
            let bytes = match fs::read(&paths.archive) {
                Ok(bytes) => bytes,
                Err(source) if source.kind() == io::ErrorKind::NotFound => {
                    return Err(mismatch(&paths.archive, "the cached archive is missing"));
                }
                Err(source) => return Err(io_error("read", &paths.archive, source)),
            };
            let found = sha256_hex(&bytes);
            if found != expected {
                return Err(mismatch(
                    &paths.archive,
                    format!("archive sha256 is {found}, the manifest pins {expected}"),
                ));
            }
        }
    }
    let found = tree_sha256(&paths.tree)?;
    if found != pin.tree_sha256 {
        return Err(mismatch(
            &paths.tree,
            format!(
                "tree sha256 is {found}, the manifest pins {}",
                pin.tree_sha256
            ),
        ));
    }
    Ok(report_of(id, &pin, true, stepper.steps))
}

fn verify_cached_commit(
    ctx: &FetchContext,
    stepper: &mut Stepper,
    pin: &Pin<'_>,
    paths: &CachePaths,
) -> Result<(), FetchError> {
    // Name the repository explicitly: without a `.git` of its own the tree
    // would be searched upwards and could report an enclosing repository's HEAD.
    let git_dir = paths.tree.join(".git");
    match fs::symlink_metadata(&git_dir) {
        Ok(meta) if meta.is_dir() => {}
        _ => {
            return Err(mismatch(
                &paths.tree,
                "the cached tree has no .git directory",
            ));
        }
    }
    let head = stepper.run(
        "git rev-parse",
        "git",
        vec![
            ctx.git.clone().into_os_string(),
            os("--git-dir"),
            git_dir.into_os_string(),
            os("rev-parse"),
            os("HEAD"),
        ],
        None,
    )?;
    let found = String::from_utf8_lossy(&head).trim().to_owned();
    if found != pin.commit {
        return Err(mismatch(
            &paths.tree,
            format!("HEAD is {found}, the manifest pins {}", pin.commit),
        ));
    }
    Ok(())
}
