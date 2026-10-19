//! The test-bed pipeline: validate, fetch, capture and replay over the selected
//! targets (CONTEXT D-21, D-22).
//!
//! Each `run_*` function does one container step for every selected target and
//! returns a [`StepSummary`]: one row per target (fetch) or per target
//! configuration (capture, replay), whose value is the verdict word, and a
//! `passed` flag that is true only when every row is a pass. A failing build,
//! gate or comparison is a row, never an error; [`TestbedError`] is only for
//! inputs that cannot be used (an invalid manifest or pins file, an unknown
//! target, a target with no build recipe) and for tools that cannot start.
//!
//! Outputs, all deterministic JSON with no timestamp:
//! `<out>/fetch/<id>.json`, `<out>/capture/<id>/<config>.json`,
//! `<out>/capture/<id>/<config>.compile_commands.json`,
//! `<out>/capture/<id>/variant.json`, `<out>/replay/<id>/<config>.json` and
//! `<out>/summary-<step>.json`. The work directory keeps the sources, the run
//! trees and the capture artifacts replay reads.
//!
//! Every external tool runs through the `mt-toolchain` runner; this module
//! spawns nothing itself.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use mt_toolchain::pins::{Pins, PinsError};
use mt_toolchain::runner::{RunnerConfig, RunnerError};
use serde::{Deserialize, Serialize};

use crate::capture::{
    CaptureContext, CaptureError, RunLayout, ToolPaths, VariantDiff, capture,
    configure_evidence_changed, reported_tools, variant_code_diff,
};
use crate::cc::{CcEntry, CcPolicy};
use crate::droplog::{DropLog, DropLogError, Dropped, MIN_RETAINED};
use crate::fetch::{
    FetchContext, FetchError, PinnedSource, cache_paths, fetch, pinned_source, verify_cached,
};
use crate::manifest::{BuildSystem, Config, Manifest, ManifestError, Target};
use crate::replay::{Normaliser, ReplayError, ReplayPlan, replay};
use crate::report::{CaptureReport, CaptureVerdict, REPORT_SCHEMA_VERSION, Verdict};
use crate::treehash::sha256_hex;

/// Version of the serialised [`StepSummary`].
pub const SUMMARY_SCHEMA_VERSION: u32 = 1;

/// Wall-clock limit of each fetch step.
pub(crate) const FETCH_TIMEOUT: Duration = Duration::from_secs(600);

/// Wall-clock limit of each build, replay and normalisation step.
const BUILD_TIMEOUT: Duration = Duration::from_secs(900);

/// The longest a row value may be; longer causes are cut.
const ROW_LIMIT: usize = 400;

/// Which targets a step runs on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    /// Every target of the manifest.
    All,
    /// These target ids.
    Ids(Vec<String>),
}

/// Everything a step needs from the command line.
#[derive(Debug, Clone)]
pub struct TestbedOptions {
    /// `testbed/manifest.toml`.
    pub manifest_path: PathBuf,
    /// `container/pins.toml` (not read by validation).
    pub pins_path: PathBuf,
    /// The committed `testbed/` directory, holding `targets/<id>/` inputs.
    pub testbed_dir: PathBuf,
    /// The work directory (`/work` in the image).
    pub work_dir: PathBuf,
    /// Where reports and summaries are written.
    pub out_dir: PathBuf,
    /// The targets to run.
    pub selection: Selection,
    /// The toolchain image reference stamped into each capture report.
    pub image: Option<String>,
}

/// Why a step could not run at all.
#[derive(Debug, thiserror::Error)]
pub enum TestbedError {
    /// The manifest could not be read or is invalid.
    #[error(transparent)]
    Manifest(#[from] ManifestError),
    /// The pins file could not be read or is invalid.
    #[error(transparent)]
    Pins(#[from] PinsError),
    /// The drop log could not be read or does not fit the manifest.
    #[error(transparent)]
    DropLog(#[from] DropLogError),
    /// A selected id is not in the manifest.
    #[error("unknown target {0:?}: it is not in the manifest")]
    UnknownTarget(String),
    /// No target was selected.
    #[error("no target selected: name at least one id or select all")]
    NothingSelected,
    /// A run over the whole bed whose manifest, minus the fully dropped targets,
    /// holds fewer than [`MIN_RETAINED`] targets (WR-08, D-11).
    #[error(
        "the bed retains {retained} targets, below the floor of {floor} (D-11): a run over the whole bed cannot pass"
    )]
    BelowFloor {
        /// The targets the manifest keeps after the fully dropped ones.
        retained: usize,
        /// [`MIN_RETAINED`].
        floor: usize,
    },
    /// Capture and replay need a recipe.
    #[error("target {0:?} has no build_args recipe; it cannot be captured or replayed yet")]
    NoRecipe(String),
    /// The fetch tools or inputs cannot be used.
    #[error(transparent)]
    Fetch(#[from] FetchError),
    /// The capture inputs or tools cannot be used.
    #[error(transparent)]
    Capture(#[from] CaptureError),
    /// A tool could not be started or supervised.
    #[error(transparent)]
    Runner(#[from] RunnerError),
    /// A report or summary could not be serialised.
    #[error("cannot serialise a report: {0}")]
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

fn io_error(action: &'static str, path: &Path, source: io::Error) -> TestbedError {
    TestbedError::Io {
        action,
        path: path.to_path_buf(),
        source,
    }
}

/// The result of one step: a verdict word per row and an overall pass flag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepSummary {
    /// [`SUMMARY_SCHEMA_VERSION`].
    pub schema_version: u32,
    /// `validate`, `fetch`, `capture` or `replay`.
    pub step: String,
    /// Row name to verdict text, in sorted order.
    pub rows: BTreeMap<String, String>,
    /// True only when every row is a pass.
    pub passed: bool,
}

impl StepSummary {
    /// An empty summary that has passed so far.
    #[must_use]
    pub fn new(step: &str) -> Self {
        Self {
            schema_version: SUMMARY_SCHEMA_VERSION,
            step: step.to_owned(),
            rows: BTreeMap::new(),
            passed: true,
        }
    }

    /// Add a row; a row that is not `ok` makes the step fail.
    pub fn add(&mut self, name: &str, value: &str, ok: bool) {
        self.rows.insert(name.to_owned(), one_line(value));
        self.passed &= ok;
    }

    /// Pretty JSON plus a trailing newline. Holds no timestamp.
    ///
    /// # Errors
    /// [`serde_json::Error`] if serialisation fails.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        let mut text = serde_json::to_string_pretty(self)?;
        text.push('\n');
        Ok(text)
    }

    /// Add the row of a dropped pair, `dropped (<cause>)` (D-11). A dropped pair
    /// is not run, so it neither passes nor fails the step: it is never counted
    /// as a pass and it cannot hide a failure of a retained pair.
    pub fn add_dropped(&mut self, name: &str, dropped: &Dropped) {
        self.rows.insert(
            name.to_owned(),
            format!("dropped ({})", dropped.cause.as_str()),
        );
    }

    /// The table printed by `mt testbed`: the step, one aligned row per entry
    /// and a result line.
    #[must_use]
    pub fn render_table(&self) -> String {
        let width = self.rows.keys().map(String::len).max().unwrap_or(0);
        let mut text = format!("step: {}\n", self.step);
        for (name, value) in &self.rows {
            text.push_str(&format!("{name:<width$}  {value}\n"));
        }
        let result = if self.passed { "passed" } else { "failed" };
        text.push_str(&format!("result: {result} ({} rows)\n", self.rows.len()));
        text
    }
}

/// A value on one line: runs of whitespace become one space, and a very long
/// value is cut.
fn one_line(value: &str) -> String {
    let joined = value.split_whitespace().collect::<Vec<_>>().join(" ");
    match joined.char_indices().nth(ROW_LIMIT) {
        Some((cut, _)) => format!("{}...", &joined[..cut]),
        None => joined,
    }
}

/// The manifest, the pins and the selected ids of a step.
pub(crate) struct Inputs {
    pub(crate) manifest: Manifest,
    pub(crate) pins: Pins,
    pub(crate) ids: Vec<String>,
    /// `<testbed_dir>/dropped.toml`, validated against the manifest.
    pub(crate) drops: DropLog,
}

pub(crate) fn target_of<'a>(manifest: &'a Manifest, id: &str) -> Result<&'a Target, TestbedError> {
    manifest
        .target
        .get(id)
        .ok_or_else(|| TestbedError::UnknownTarget(id.to_owned()))
}

fn resolve_ids(manifest: &Manifest, selection: &Selection) -> Result<Vec<String>, TestbedError> {
    match selection {
        Selection::All => Ok(manifest.target.keys().cloned().collect()),
        Selection::Ids(ids) => {
            if ids.is_empty() {
                return Err(TestbedError::NothingSelected);
            }
            let mut sorted = ids.clone();
            sorted.sort();
            sorted.dedup();
            for id in &sorted {
                target_of(manifest, id)?;
            }
            Ok(sorted)
        }
    }
}

/// The number of targets a run over the whole bed has, or the refusal when it
/// is below [`MIN_RETAINED`] (WR-08, D-11).
///
/// A target counts unless the drop log drops it whole; a dropped variant leaves
/// its target retained. Every step of an `--all` run calls this before it writes
/// or starts anything, so the step that produces the evidence cannot print a pass
/// for a bed below the floor. There is no option to lower it.
///
/// # Errors
/// [`TestbedError::BelowFloor`] naming the retained count and the floor.
pub fn check_floor(manifest: &Manifest, drops: &DropLog) -> Result<usize, TestbedError> {
    let retained = drops.retained_targets(manifest).len();
    if retained < MIN_RETAINED {
        return Err(TestbedError::BelowFloor {
            retained,
            floor: MIN_RETAINED,
        });
    }
    Ok(retained)
}

pub(crate) fn load_inputs(
    opts: &TestbedOptions,
    need_recipe: bool,
) -> Result<Inputs, TestbedError> {
    let manifest = Manifest::load(&opts.manifest_path)?;
    manifest.validate()?;
    let pins = Pins::load(&opts.pins_path)?;
    pins.validate()?;
    let drops = DropLog::load_from_testbed_dir(&opts.testbed_dir)?;
    drops.validate(&manifest)?;
    // Only the whole bed is evidence of the floor; named ids are partial runs.
    if opts.selection == Selection::All {
        check_floor(&manifest, &drops)?;
    }
    let ids = resolve_ids(&manifest, &opts.selection)?;
    if need_recipe {
        for id in &ids {
            if target_of(&manifest, id)?.build_args.is_none() {
                return Err(TestbedError::NoRecipe(id.clone()));
            }
        }
    }
    Ok(Inputs {
        manifest,
        pins,
        ids,
        drops,
    })
}

pub(crate) fn runner_config(pins: &Pins, timeout: Duration) -> RunnerConfig {
    RunnerConfig {
        timeout,
        ..RunnerConfig::new(
            pins.image.tool_path.clone(),
            pins.image.source_date_epoch.clone(),
        )
    }
}

/// The configurations of a target, `default` first and then by name.
fn ordered_configs(target: &Target) -> Vec<&Config> {
    let mut configs: Vec<&Config> = target.config.iter().collect();
    configs.sort_by(|a, b| (a.name != "default", &a.name).cmp(&(b.name != "default", &b.name)));
    configs
}

fn write_text(path: &Path, text: &str) -> Result<(), TestbedError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| io_error("create", parent, source))?;
    }
    fs::write(path, text).map_err(|source| io_error("write", path, source))
}

pub(crate) fn write_summary(
    opts: &TestbedOptions,
    summary: &StepSummary,
) -> Result<(), TestbedError> {
    let path = opts.out_dir.join(format!("summary-{}.json", summary.step));
    write_text(&path, &summary.to_json()?)
}

fn make_work_dir(opts: &TestbedOptions) -> Result<(), TestbedError> {
    fs::create_dir_all(&opts.work_dir).map_err(|source| io_error("create", &opts.work_dir, source))
}

/// Check the manifest and report how much it holds (CONTEXT D-21).
///
/// # Errors
/// [`TestbedError::Manifest`] when the file cannot be read or is invalid.
pub fn run_validate(manifest_path: &Path) -> Result<StepSummary, TestbedError> {
    let manifest = Manifest::load(manifest_path)?;
    manifest.validate()?;
    let configurations: usize = manifest.target.values().map(|t| t.config.len()).sum();
    let mut summary = StepSummary::new("validate");
    summary.add(
        "manifest",
        &format!(
            "valid ({} targets, {configurations} configurations)",
            manifest.target.len()
        ),
        true,
    );
    Ok(summary)
}

/// Fetch every selected source into the work directory and verify it (D-17,
/// D-19). Only this step may have network access.
///
/// A fetch that fails (a tree hash that is not the pin, a git error, a timeout)
/// is a `failed: <cause>` row; the report is written for every source that was
/// fetched or reused.
///
/// # Errors
/// [`TestbedError`] for unusable inputs or a tool that cannot start.
pub fn run_fetch(opts: &TestbedOptions) -> Result<StepSummary, TestbedError> {
    let inputs = load_inputs(opts, false)?;
    let cfg = runner_config(&inputs.pins, FETCH_TIMEOUT);
    make_work_dir(opts)?;
    let ctx = FetchContext::from_pins(&inputs.pins, &opts.work_dir)?;
    let mut summary = StepSummary::new("fetch");
    for id in &inputs.ids {
        if let Some(dropped) = inputs.drops.target_dropped(id) {
            summary.add_dropped(id, dropped);
            continue;
        }
        let target = target_of(&inputs.manifest, id)?;
        match fetch(&cfg, &ctx, id, target) {
            Ok(report) => {
                write_text(
                    &opts.out_dir.join("fetch").join(format!("{id}.json")),
                    &report.to_json()?,
                )?;
                summary.add(id, if report.reused { "reused" } else { "fetched" }, true);
            }
            Err(error @ (FetchError::InvalidInput(_) | FetchError::ToolMissing(_))) => {
                return Err(error.into());
            }
            Err(error) => summary.add(id, &format!("failed: {error}"), false),
        }
    }
    write_summary(opts, &summary)?;
    Ok(summary)
}

/// What `variant.json` holds: the code comparison of a variant with its default.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariantReport {
    /// [`REPORT_SCHEMA_VERSION`].
    pub schema_version: u32,
    /// Always `variant`.
    pub kind: String,
    /// Test-bed target id.
    pub target: String,
    /// The default configuration name.
    pub default: String,
    /// The variant's name.
    pub variant: String,
    /// The comparison.
    pub diff: VariantDiff,
}

fn verdict_word(verdict: CaptureVerdict) -> &'static str {
    match verdict {
        CaptureVerdict::Captured => "captured",
        CaptureVerdict::CaptureFailed => "capture_failed",
        CaptureVerdict::BuildFailed => "build_failed",
        CaptureVerdict::NotProven => "not_proven",
    }
}

/// The causes of a refused capture as one row value.
fn capture_row(report: &CaptureReport) -> String {
    if report.verdict == CaptureVerdict::Captured {
        return "captured".to_owned();
    }
    format!(
        "{} ({}): {}",
        verdict_word(report.verdict),
        report.causes.join(","),
        report.cause_details.join(" | ")
    )
}

/// Capture every configuration of every selected target, build and entry check
/// included, and compare each variant with its default (BUILD-01, BED-02).
///
/// Run it with no network. `opts.image` is stamped into each report.
///
/// # Errors
/// [`TestbedError`] for unusable inputs (including a target with no recipe) or
/// a tool that cannot start.
pub fn run_capture(opts: &TestbedOptions) -> Result<StepSummary, TestbedError> {
    let inputs = load_inputs(opts, true)?;
    let cfg = runner_config(&inputs.pins, BUILD_TIMEOUT);
    make_work_dir(opts)?;
    let tools = ToolPaths::from_pins(&inputs.pins)?;
    let ctx = CaptureContext::new(
        &opts.work_dir,
        &opts.testbed_dir,
        tools,
        inputs.manifest.baseline.cflags.clone(),
    );
    let mut summary = StepSummary::new("capture");
    for id in &inputs.ids {
        let target = target_of(&inputs.manifest, id)?;
        let mut captured: Vec<(String, CaptureReport)> = Vec::new();
        for config in ordered_configs(target) {
            let row = format!("{id}/{}", config.name);
            if let Some(dropped) = inputs.drops.is_dropped(id, &config.name) {
                summary.add_dropped(&row, dropped);
                continue;
            }
            let mut report = match capture(&cfg, &ctx, id, target, config) {
                Ok(report) => report,
                Err(CaptureError::Fetch(error)) => {
                    summary.add(&row, &format!("not_proven: {error}"), false);
                    continue;
                }
                Err(error) => return Err(error.into()),
            };
            report.image.clone_from(&opts.image);
            let layout = RunLayout::new(&opts.work_dir, id, &config.name);
            let json = report.to_json()?;
            write_text(&layout.report_path(), &json)?;
            let base = opts.out_dir.join("capture").join(id);
            write_text(&base.join(format!("{}.json", config.name)), &json)?;
            match fs::read_to_string(layout.canonical_path()) {
                Ok(text) => write_text(
                    &base.join(format!("{}.compile_commands.json", config.name)),
                    &text,
                )?,
                Err(source) if source.kind() == io::ErrorKind::NotFound => {}
                Err(source) => return Err(io_error("read", &layout.canonical_path(), source)),
            }
            summary.add(
                &row,
                &capture_row(&report),
                report.verdict == CaptureVerdict::Captured,
            );
            captured.push((config.name.clone(), report));
        }
        if let Some(variant) = target.variant() {
            let row = format!("{id}/variant");
            if let Some(dropped) = inputs.drops.is_dropped(id, &variant.name) {
                summary.add_dropped(&row, dropped);
                continue;
            }
            let pair = (
                captured.iter().find(|(name, _)| name == "default"),
                captured.iter().find(|(name, _)| *name == variant.name),
            );
            let value = match pair {
                (Some((_, base)), Some((_, other)))
                    if base.verdict == CaptureVerdict::Captured
                        && other.verdict == CaptureVerdict::Captured =>
                {
                    let diff = variant_code_diff(base, other);
                    let changes = diff.changes_code;
                    let report = VariantReport {
                        schema_version: REPORT_SCHEMA_VERSION,
                        kind: "variant".to_owned(),
                        target: id.clone(),
                        default: "default".to_owned(),
                        variant: variant.name.clone(),
                        diff,
                    };
                    let mut json = serde_json::to_string_pretty(&report)?;
                    json.push('\n');
                    write_text(
                        &opts.out_dir.join("capture").join(id).join("variant.json"),
                        &json,
                    )?;
                    format!("changes_code {changes}")
                }
                _ => "not_evaluated".to_owned(),
            };
            summary.add(&row, &value, true);
        }
    }
    write_summary(opts, &summary)?;
    Ok(summary)
}

/// Why a configuration cannot be replayed, as a `not_proven` row value.
fn unproven(reason: impl std::fmt::Display) -> String {
    format!("not_proven: {reason}")
}

fn verdict_text(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Identical => "identical",
        Verdict::IdenticalAfterNormalisation => "identical_after_normalisation",
        Verdict::Differs => "differs",
        Verdict::NotProven => "not_proven",
        Verdict::BuildFailed => "build_failed",
    }
}

/// Build the replay plan of one captured configuration from its capture
/// artifacts in the work directory, or say why it cannot be replayed.
fn plan_of(
    opts: &TestbedOptions,
    tools: &ToolPaths,
    id: &str,
    target: &Target,
    config: &str,
    captured: &CaptureReport,
    pin: &PinnedSource,
) -> Result<ReplayPlan, String> {
    let layout = RunLayout::new(&opts.work_dir, id, config);
    let bytes = fs::read(layout.canonical_path())
        .map_err(|error| format!("cannot read the canonical compile commands: {error}"))?;
    if captured.compile_commands.canonical_sha256.as_deref() != Some(&sha256_hex(&bytes)) {
        return Err("the canonical compile commands no longer match the capture report".to_owned());
    }
    let entries: Vec<CcEntry> = serde_json::from_slice(&bytes)
        .map_err(|error| format!("the canonical compile commands are not valid: {error}"))?;
    let keep_dirs = if target.build == BuildSystem::Cmake {
        if let Some(changed) = configure_evidence_changed(&layout.run_root, captured) {
            return Err(format!(
                "a configure-time file is not as captured: {changed}"
            ));
        }
        vec![layout.build.clone()]
    } else {
        Vec::new()
    };
    let arm_gcc = tools
        .arm_gcc
        .to_str()
        .ok_or_else(|| "the pinned compiler path is not valid UTF-8".to_owned())?
        .to_owned();
    Ok(ReplayPlan {
        target: id.to_owned(),
        config: config.to_owned(),
        run_root: layout.run_root.clone(),
        pristine_src: cache_paths(&opts.work_dir, id, &pin.commit).tree,
        src_dir_name: "src".to_owned(),
        keep_dirs,
        entries,
        original_objects: captured
            .objects
            .iter()
            .map(|(key, object)| (key.clone(), object.sha256.clone()))
            .collect(),
        original_objects_dir: layout.objects_dir.clone(),
        policy: CcPolicy {
            allowed_compilers: vec![arm_gcc],
            run_root: layout.run_root,
        },
        compiler_tool_name: "arm_gcc".to_owned(),
        normaliser: Some(Normaliser {
            objcopy: tools.objcopy.clone(),
            scratch: opts.work_dir.join("scratch").join(id).join(config),
        }),
    })
}

/// Why a capture report is not the one the manifest and the pins file describe,
/// or `None` when it is (WR-02). The report is read from the work directory, so
/// nothing in it is trusted: its pair, its source pin and its tool versions must
/// equal the current ones, and no value of it becomes a path.
fn report_mismatch(
    captured: &CaptureReport,
    id: &str,
    config: &str,
    pin: &PinnedSource,
    tools: &BTreeMap<String, String>,
) -> Option<String> {
    let different = |what: &str, field: &str| {
        Some(format!(
            "the capture report was made from a different {what}: {field}"
        ))
    };
    if captured.target != id {
        return different("pair", "target");
    }
    if captured.config != config {
        return different("pair", "config");
    }
    if captured.source.commit != pin.commit {
        return different("source pin", "source.commit");
    }
    if captured.source.tree_sha256 != pin.tree_sha256 {
        return different("source pin", "source.tree_sha256");
    }
    if &captured.tools != tools {
        let named: Vec<&str> = captured
            .tools
            .keys()
            .chain(tools.keys())
            .filter(|key| captured.tools.get(*key) != tools.get(*key))
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        return different("tool set", &format!("tools ({})", named.join(", ")));
    }
    None
}

/// Replay every captured configuration of every selected target in a clean copy
/// of its verified source and compare each object byte for byte (BUILD-02).
///
/// The captured build is read from the work directory, so `capture` must have
/// run on the same work directory. A configuration that was not captured, whose
/// report was made for another pair, source pin or tool set than the manifest
/// and the pins file name now (WR-02), whose artifacts changed, or whose source
/// cache no longer matches its pin is `not_proven`. The pristine source is the
/// cache of the manifest's pinned commit, never a path read from the report. Run
/// it with no network.
///
/// # Errors
/// [`TestbedError`] for unusable inputs (including a target with no recipe) or
/// a tool that cannot start.
pub fn run_replay(opts: &TestbedOptions) -> Result<StepSummary, TestbedError> {
    let inputs = load_inputs(opts, true)?;
    let cfg = runner_config(&inputs.pins, BUILD_TIMEOUT);
    make_work_dir(opts)?;
    let tools = ToolPaths::from_pins(&inputs.pins)?;
    let verify_ctx = FetchContext {
        work_dir: opts.work_dir.clone(),
        git: tools.git.clone(),
        curl: PathBuf::new(),
        tar: PathBuf::from(crate::fetch::TAR_PATH),
        allowed_protocols: String::new(),
        timeout: FETCH_TIMEOUT,
    };
    let mut summary = StepSummary::new("replay");
    for id in &inputs.ids {
        let target = target_of(&inputs.manifest, id)?;
        for config in ordered_configs(target) {
            let row = format!("{id}/{}", config.name);
            if let Some(dropped) = inputs.drops.is_dropped(id, &config.name) {
                summary.add_dropped(&row, dropped);
                continue;
            }
            let layout = RunLayout::new(&opts.work_dir, id, &config.name);
            let captured = match fs::read(layout.report_path())
                .map_err(|error| error.to_string())
                .and_then(|bytes| {
                    serde_json::from_slice::<CaptureReport>(&bytes)
                        .map_err(|error| error.to_string())
                }) {
                Ok(report) => report,
                Err(error) => {
                    summary.add(
                        &row,
                        &unproven(format!("no readable capture report: {error}")),
                        false,
                    );
                    continue;
                }
            };
            if captured.verdict != CaptureVerdict::Captured {
                summary.add(
                    &row,
                    &unproven(format!(
                        "the capture verdict is {}",
                        verdict_word(captured.verdict)
                    )),
                    false,
                );
                continue;
            }
            let pin = match pinned_source(id, target) {
                Ok(pin) => pin,
                Err(error) => {
                    summary.add(&row, &unproven(error), false);
                    continue;
                }
            };
            if let Some(reason) = report_mismatch(
                &captured,
                id,
                &config.name,
                &pin,
                &reported_tools(&tools, target.build),
            ) {
                summary.add(&row, &unproven(reason), false);
                continue;
            }
            if let Err(error) = verify_cached(&cfg, &verify_ctx, id, target) {
                summary.add(&row, &unproven(error), false);
                continue;
            }
            let plan = match plan_of(opts, &tools, id, target, &config.name, &captured, &pin) {
                Ok(plan) => plan,
                Err(reason) => {
                    summary.add(&row, &unproven(reason), false);
                    continue;
                }
            };
            match replay(&cfg, &plan) {
                Ok(report) => {
                    write_text(
                        &opts
                            .out_dir
                            .join("replay")
                            .join(id)
                            .join(format!("{}.json", config.name)),
                        &report.to_json()?,
                    )?;
                    let ok = matches!(
                        report.verdict,
                        Verdict::Identical | Verdict::IdenticalAfterNormalisation
                    );
                    summary.add(&row, verdict_text(report.verdict), ok);
                }
                Err(ReplayError::Runner(error)) => return Err(error.into()),
                Err(error) => summary.add(&row, &unproven(error), false),
            }
        }
    }
    write_summary(opts, &summary)?;
    Ok(summary)
}
