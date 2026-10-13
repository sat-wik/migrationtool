//! Clean-copy replay of captured compile commands (BUILD-02, CONTEXT D-26).
//!
//! A replay deletes the source tree of a finished build, restores the pristine
//! tree, removes every declared output and re-runs each captured compile as an
//! argv vector through the runner, never through a shell. Each rebuilt object
//! is then compared byte for byte with the original. The same code replays the
//! real Arm builds in CI and the two-file host project in the offline tests.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use mt_toolchain::runner::{RunRequest, RunnerConfig, RunnerError, run};

use crate::cc::{CcEntry, CcError, CcPolicy, canonical_output, lexically_under, normalise_lexical};
use crate::elf::{differing_sections, normalisable};
use crate::report::{
    NORMALISATION_RULE, ObjectComparison, ObjectStatus, REPORT_SCHEMA_VERSION, ReplayReport, Step,
    Verdict,
};
use crate::treehash::{TreeHashError, copy_tree, sha256_hex};

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
        source: io::Error,
    },
}

fn io_error(action: &'static str, path: &Path, source: io::Error) -> ReplayError {
    ReplayError::Io {
        action,
        path: path.to_path_buf(),
        source,
    }
}

fn invalid(message: impl Into<String>) -> ReplayError {
    ReplayError::InvalidPlan(message.into())
}

/// The run-root-relative key of `path`, with `/` separators.
fn relative_key(run_root: &Path, path: &Path) -> Option<String> {
    let normal = normalise_lexical(path);
    let relative = normal.strip_prefix(normalise_lexical(run_root)).ok()?;
    let mut parts = Vec::new();
    for component in relative.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_str()?),
            _ => return None,
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("/"))
    }
}

/// Check the plan before anything is deleted, copied or spawned.
fn validate(plan: &ReplayPlan) -> Result<(), ReplayError> {
    if !plan.run_root.is_absolute() {
        return Err(invalid("the run root must be an absolute path"));
    }
    if normalise_lexical(&plan.policy.run_root) != normalise_lexical(&plan.run_root) {
        return Err(invalid(
            "the policy run root differs from the plan run root",
        ));
    }
    let mut src_parts = Path::new(&plan.src_dir_name).components();
    match (src_parts.next(), src_parts.next()) {
        (Some(Component::Normal(_)), None) => {}
        _ => return Err(invalid("src_dir_name must be a single directory name")),
    }
    let src_dir = plan.run_root.join(&plan.src_dir_name);
    for keep in &plan.keep_dirs {
        if !lexically_under(&plan.run_root, keep) || lexically_under(&src_dir, keep) {
            return Err(invalid(format!(
                "keep directory {} must lie under the run root and outside the source directory",
                keep.display()
            )));
        }
    }
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for entry in &plan.entries {
        plan.policy.check_entry(entry)?;
        if canonical_output(entry)? != entry.output {
            return Err(invalid(format!(
                "the output of {} does not match its arguments",
                entry.file
            )));
        }
        if !seen.insert(entry.output.as_str()) {
            return Err(invalid(format!("two entries write {}", entry.output)));
        }
        if relative_key(&plan.run_root, Path::new(&entry.output)).is_none() {
            return Err(invalid(format!(
                "output {} has no run-root-relative key",
                entry.output
            )));
        }
    }
    for key in plan.original_objects.keys() {
        let path = Path::new(key);
        let clean = !key.is_empty()
            && path
                .components()
                .all(|component| matches!(component, Component::Normal(_)));
        if !clean {
            return Err(invalid(format!(
                "object key {key:?} is not a relative path"
            )));
        }
    }
    Ok(())
}

fn remove_file_if_present(path: &Path) -> Result<(), ReplayError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(io_error("remove", path, source)),
    }
}

fn report_skeleton(plan: &ReplayPlan, verdict: Verdict, detail: Option<&str>) -> ReplayReport {
    ReplayReport {
        schema_version: REPORT_SCHEMA_VERSION,
        kind: "replay".to_owned(),
        target: plan.target.clone(),
        config: plan.config.clone(),
        verdict,
        normalisation_rule: NORMALISATION_RULE.to_owned(),
        normalisation_applied: false,
        objects: BTreeMap::new(),
        steps: Vec::new(),
        detail: detail.map(str::to_owned),
    }
}

fn comparison(
    original: Option<String>,
    replayed: Option<String>,
    status: ObjectStatus,
) -> ObjectComparison {
    ObjectComparison {
        original,
        replay: replayed,
        status,
        normalised_original: None,
        normalised_replay: None,
        differing_sections: Vec::new(),
        detail: None,
    }
}

/// Replay `plan` and report.
///
/// The steps are: refuse an invalid plan; delete the source directory; restore
/// the pristine tree; remove every declared output; run each entry in the given
/// order; hash every output and compare it with the original. A compile that
/// times out or leaves an incomplete stream makes the verdict `not_proven`, a
/// compile that exits non-zero makes it `build_failed`, and the remaining
/// entries still run so the report lists every failure.
///
/// # Errors
/// [`ReplayError`] only for invalid inputs, a refused compiler or path, or a
/// subprocess that cannot be started. A target-configuration failure is data in
/// the report.
pub fn replay(cfg: &RunnerConfig, plan: &ReplayPlan) -> Result<ReplayReport, ReplayError> {
    validate(plan)?;
    if plan.entries.is_empty() {
        return Ok(report_skeleton(
            plan,
            Verdict::BuildFailed,
            Some("empty_capture"),
        ));
    }
    let metadata = fs::metadata(&plan.pristine_src)
        .map_err(|source| io_error("read the pristine tree", &plan.pristine_src, source))?;
    if !metadata.is_dir() {
        return Err(invalid("the pristine source is not a directory"));
    }

    // Clean copy: nothing of the original build's source directory survives.
    let src_dir = plan.run_root.join(&plan.src_dir_name);
    match fs::remove_dir_all(&src_dir) {
        Ok(()) => {}
        Err(source) if source.kind() == io::ErrorKind::NotFound => {}
        Err(source) => return Err(io_error("remove", &src_dir, source)),
    }
    fs::create_dir_all(&plan.run_root)
        .map_err(|source| io_error("create", &plan.run_root, source))?;
    copy_tree(&plan.pristine_src, &src_dir)?;

    // Every declared output goes, so a stale object can never pass.
    for entry in &plan.entries {
        remove_file_if_present(Path::new(&entry.output))?;
    }
    for entry in &plan.entries {
        let directory = Path::new(&entry.directory);
        fs::create_dir_all(directory).map_err(|source| io_error("create", directory, source))?;
        if let Some(parent) = Path::new(&entry.output).parent() {
            fs::create_dir_all(parent).map_err(|source| io_error("create", parent, source))?;
        }
    }

    let mut report = report_skeleton(plan, Verdict::Identical, None);
    let mut not_proven = false;
    let mut build_failed = false;
    for entry in &plan.entries {
        let request = RunRequest {
            tool_name: plan.compiler_tool_name.clone(),
            argv: entry.arguments.iter().map(OsString::from).collect(),
            cwd: PathBuf::from(&entry.directory),
            extra_env: BTreeMap::new(),
            tool_version: None,
        };
        let output = run(cfg, &request)?;
        let record = output.record;
        if record.exit.timed_out || !record.stdout.complete || !record.stderr.complete {
            not_proven = true;
        } else if record.exit.code != Some(0) {
            build_failed = true;
        }
        report.steps.push(Step {
            name: format!("compile {}", entry.file),
            run: record,
        });
    }

    // Hash what the entries produced.
    let mut replayed: BTreeMap<String, PathBuf> = BTreeMap::new();
    for entry in &plan.entries {
        let path = PathBuf::from(&entry.output);
        let is_file = fs::symlink_metadata(&path).is_ok_and(|meta| meta.is_file());
        if is_file && let Some(key) = relative_key(&plan.run_root, &path) {
            replayed.insert(key, path);
        }
    }
    let keys: BTreeSet<&String> = plan
        .original_objects
        .keys()
        .chain(replayed.keys())
        .collect();
    for key in keys {
        let original = plan.original_objects.get(key).cloned();
        let replay_path = replayed.get(key);
        let replay_hash = match replay_path {
            Some(path) => {
                let bytes = fs::read(path).map_err(|source| io_error("read", path, source))?;
                Some(sha256_hex(&bytes))
            }
            None => None,
        };
        let object = match (original, replay_hash) {
            (Some(original), Some(replay_hash)) if original == replay_hash => {
                comparison(Some(original), Some(replay_hash), ObjectStatus::Identical)
            }
            (Some(original), Some(replay_hash)) => {
                let outcome =
                    compare_differing(cfg, plan, key, original, replay_hash, &mut report.steps)?;
                not_proven |= outcome.not_proven;
                report.normalisation_applied |= outcome.normalised;
                outcome.object
            }
            (Some(original), None) => {
                comparison(Some(original), None, ObjectStatus::MissingInReplay)
            }
            (None, Some(replay_hash)) => {
                comparison(None, Some(replay_hash), ObjectStatus::MissingInOriginal)
            }
            (None, None) => continue,
        };
        report.objects.insert(key.clone(), object);
    }

    report.verdict = verdict_of(&report, not_proven, build_failed);
    report.detail = match report.verdict {
        Verdict::NotProven => Some("compile_timeout_or_incomplete_stream".to_owned()),
        Verdict::BuildFailed if report.objects.is_empty() => Some("empty_capture".to_owned()),
        Verdict::BuildFailed => Some("compile_failed".to_owned()),
        _ => None,
    };
    Ok(report)
}

/// What came out of comparing one object whose bytes differ.
struct Outcome {
    object: ObjectComparison,
    /// A normalisation step timed out or left an incomplete stream.
    not_proven: bool,
    /// objcopy ran on this object.
    normalised: bool,
}

fn differs(
    original: String,
    replay_hash: String,
    sections: Vec<String>,
    detail: Option<String>,
) -> ObjectComparison {
    ObjectComparison {
        differing_sections: sections,
        detail,
        ..comparison(Some(original), Some(replay_hash), ObjectStatus::Differs)
    }
}

/// One objcopy run of the fixed normalisation: `--strip-debug` and removal of
/// `.comment`, from `input` into `output`. Returns whether it ran to a clean
/// exit, and whether it must be reported as not proven.
fn run_objcopy(
    cfg: &RunnerConfig,
    normaliser: &Normaliser,
    side: &str,
    key: &str,
    input: &Path,
    steps: &mut Vec<Step>,
) -> Result<(PathBuf, bool, bool), ReplayError> {
    let output = normaliser.scratch.join(side).join(key);
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|source| io_error("create", parent, source))?;
    }
    let request = RunRequest {
        tool_name: "objcopy".to_owned(),
        argv: vec![
            normaliser.objcopy.clone().into_os_string(),
            OsString::from("--strip-debug"),
            OsString::from("--remove-section=.comment"),
            input.as_os_str().to_owned(),
            output.as_os_str().to_owned(),
        ],
        cwd: normaliser.scratch.clone(),
        extra_env: BTreeMap::new(),
        tool_version: None,
    };
    let record = run(cfg, &request)?.record;
    let incomplete = record.exit.timed_out || !record.stdout.complete || !record.stderr.complete;
    let clean = !incomplete && record.exit.code == Some(0);
    steps.push(Step {
        name: format!("normalise {side} {key}"),
        run: record,
    });
    Ok((output, clean, incomplete))
}

/// Compare an original and a replayed object whose hashes differ (CONTEXT D-09).
///
/// The sections are compared first. Only when every differing section is
/// `.comment` or a debug section, and the plan carries a normaliser, are both
/// objects passed through the fixed objcopy and compared again; anything else is
/// `differs` with the section names recorded and no objcopy run. An object that
/// cannot be read is `differs` with the reader error in `detail`.
fn compare_differing(
    cfg: &RunnerConfig,
    plan: &ReplayPlan,
    key: &str,
    original: String,
    replay_hash: String,
    steps: &mut Vec<Step>,
) -> Result<Outcome, ReplayError> {
    let original_path = plan.original_objects_dir.join(key);
    let replay_path = plan.run_root.join(key);
    let unchanged = |object| Outcome {
        object,
        not_proven: false,
        normalised: false,
    };
    let (original_bytes, replay_bytes) = match (fs::read(&original_path), fs::read(&replay_path)) {
        (Ok(a), Ok(b)) => (a, b),
        (Err(source), _) => {
            let detail = format!(
                "cannot read the original object {}: {source}",
                original_path.display()
            );
            return Ok(unchanged(differs(
                original,
                replay_hash,
                Vec::new(),
                Some(detail),
            )));
        }
        (_, Err(source)) => {
            let detail = format!(
                "cannot read the replayed object {}: {source}",
                replay_path.display()
            );
            return Ok(unchanged(differs(
                original,
                replay_hash,
                Vec::new(),
                Some(detail),
            )));
        }
    };
    let sections = match differing_sections(&original_bytes, &replay_bytes) {
        Ok(sections) => sections,
        Err(error) => {
            return Ok(unchanged(differs(
                original,
                replay_hash,
                Vec::new(),
                Some(error.to_string()),
            )));
        }
    };
    let Some(normaliser) = plan.normaliser.as_ref().filter(|_| normalisable(&sections)) else {
        let detail = sections
            .is_empty()
            .then(|| "sections are equal but the file bytes differ".to_owned());
        return Ok(unchanged(differs(original, replay_hash, sections, detail)));
    };

    let (stripped_original, original_clean, original_incomplete) =
        run_objcopy(cfg, normaliser, "original", key, &original_path, steps)?;
    let (stripped_replay, replay_clean, replay_incomplete) =
        run_objcopy(cfg, normaliser, "replay", key, &replay_path, steps)?;
    let not_proven = original_incomplete || replay_incomplete;
    if !(original_clean && replay_clean) {
        let detail = if not_proven {
            "normalisation_not_proven"
        } else {
            "normalisation_failed"
        };
        return Ok(Outcome {
            object: differs(original, replay_hash, sections, Some(detail.to_owned())),
            not_proven,
            normalised: true,
        });
    }
    let read_normalised = |path: &Path| {
        fs::read(path)
            .map(|bytes| sha256_hex(&bytes))
            .map_err(|source| io_error("read", path, source))
    };
    let normalised_original = read_normalised(&stripped_original)?;
    let normalised_replay = read_normalised(&stripped_replay)?;
    let status = if normalised_original == normalised_replay {
        ObjectStatus::IdenticalAfterNormalisation
    } else {
        ObjectStatus::Differs
    };
    Ok(Outcome {
        object: ObjectComparison {
            normalised_original: Some(normalised_original),
            normalised_replay: Some(normalised_replay),
            differing_sections: sections,
            ..comparison(Some(original), Some(replay_hash), status)
        },
        not_proven: false,
        normalised: true,
    })
}

/// Verdict precedence: not proven, build failed, differs, identical after
/// normalisation, identical. A configuration with no objects at all is never
/// identical.
fn verdict_of(report: &ReplayReport, not_proven: bool, build_failed: bool) -> Verdict {
    if not_proven {
        return Verdict::NotProven;
    }
    if build_failed || report.objects.is_empty() {
        return Verdict::BuildFailed;
    }
    let statuses = report.objects.values().map(|object| object.status);
    let mut normalised = false;
    for status in statuses {
        match status {
            ObjectStatus::Differs
            | ObjectStatus::MissingInReplay
            | ObjectStatus::MissingInOriginal => {
                return Verdict::Differs;
            }
            ObjectStatus::IdenticalAfterNormalisation => normalised = true,
            ObjectStatus::Identical => {}
        }
    }
    if normalised {
        Verdict::IdenticalAfterNormalisation
    } else {
        Verdict::Identical
    }
}
