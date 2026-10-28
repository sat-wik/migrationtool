//! Building the drivers of a selection and running the seeds into corpora
//! (CONTEXT D-01, D-02, D-05, D-07, D-14, D-19, D-20, D-22, D-26, D-28).
//!
//! [`build_entries`] writes, for every entry with roles of every selected
//! (target, configuration) pair, `driver.c`, `pair.a`, `driver.elf` and
//! `build.json` under `<build root>/<target>/<config>/<entry>/`. [`run_entries`]
//! loads and validates the target's policy, selects the seeds whose index lists
//! the entry, runs every (seed, scalar combination) twice, requires the two
//! vectors to be byte-identical, and writes the entry's corpus with its
//! provenance in one step, after every run has finished.

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use mt_capture::capture::RunLayout;
use mt_capture::cc::CcEntry;
use mt_capture::entries::{
    IncludePrefix, filter_flags, probe_source, probe_source_with_prefix, unit_include_prefix,
};
use mt_capture::manifest::{Entry, Manifest};
use mt_contract::{CONTRACT_VERSION, Policy};
use mt_seeds::index::SeedIndex;
use mt_toolchain::runner::{RunOutput, RunRecord, RunnerConfig};
use mt_vectors::bytes::encode_latin1;
use mt_vectors::corpus::{HasUbSource, Provenance, ProvenanceBudgets, write_corpus};
use mt_vectors::sha256_hex;
use mt_vectors::state::{LibState, StateValue};
use mt_vectors::vector::{HasUbLabel, Output, Vector};
use serde::{Deserialize, Serialize};

use crate::adapter::{
    Adapter, AdapterError, Combination, RET_NAME, ReturnKind, combinations, driver_flags,
};
use crate::adapter_drops::AdapterDropLog;
use crate::budget::Budgets;
use crate::classify::{Outcome, classify};
use crate::decode::{Regions, apply_marks, check_marks, decode_value};
use crate::driver::render_entry_driver;
use crate::exec::{Request, Slot, run_driver};
use crate::frame::{Frame, parse_frame};
use crate::globals::{
    DumpRange, function_symbols, locate_globals, record_globals, writable_globals,
};
use crate::heap::walk_heap;
use crate::layout::{CType, Kind, layout_summary};
use crate::link::{LinkOutcome, LinkRequest, archive, link_driver, verify_objects};
use crate::probe::{GlobalTarget, GlobalType, LayoutRequest, layout_for_entry, layout_for_globals};
use crate::tools::ExecTools;
use crate::{EXECUTOR_VERSION, HarnessError};

/// The file in a build directory that records how the driver was built.
pub const BUILD_JSON: &str = "build.json";

/// Version of `build.json`.
pub const BUILD_SCHEMA_VERSION: u32 = 1;

/// The region id of the whole output buffer in the result frame.
const OUT_REGION: u32 = 1;

/// The region id of dump range `i` before the call is this plus `i`; after the
/// call, [`DUMP_AFTER_REGION`] plus `i` (the numbers `runtime.c` writes).
const DUMP_BEFORE_REGION: u32 = 0x1000;

/// See [`DUMP_BEFORE_REGION`].
const DUMP_AFTER_REGION: u32 = 0x2000;

/// The region of the addresses of the driver's own objects: the input and output
/// buffers (address, length), the heap arena in use (address, used bytes), then
/// each state (address, size), all as 32-bit words.
const ADDRESSES_REGION: u32 = 2;

/// The input buffer after the call (a `mutable` input is recorded from it).
const IN_AFTER_REGION: u32 = 3;

/// The used bytes of the heap arena after the call, from its first byte.
const HEAP_REGION: u32 = 4;

/// State `i` before the call is this plus `i`; after the call,
/// [`STATE_AFTER_REGION`] plus `i`.
const STATE_BEFORE_REGION: u32 = 0x3000;

/// See [`STATE_BEFORE_REGION`].
const STATE_AFTER_REGION: u32 = 0x4000;

/// Which entries to build or run, and where things are.
#[derive(Debug, Clone)]
pub struct Scope {
    /// `testbed/manifest.toml`; the directory holding it also holds the
    /// `policy` and `seeds` directories.
    pub manifest_path: PathBuf,
    /// The work directory: `capture/<id>/<config>` and `run/<id>/<config>` are
    /// read from it.
    pub work_dir: PathBuf,
    /// The (target, configuration) pairs, in the order they are processed.
    pub pairs: Vec<(String, String)>,
    /// Restrict to these entry names; `None` takes every entry.
    pub entries: Option<Vec<String>>,
    /// Where the drivers are built (default `<work>/vectors-build`).
    pub build_root: PathBuf,
}

impl Scope {
    /// A scope over `work_dir` with no pair selected and the default build root.
    #[must_use]
    pub fn new(manifest_path: PathBuf, work_dir: PathBuf) -> Self {
        let build_root = work_dir.join("vectors-build");
        Self {
            manifest_path,
            work_dir,
            pairs: Vec::new(),
            entries: None,
            build_root,
        }
    }

    /// The committed `testbed/` directory.
    #[must_use]
    pub fn testbed_dir(&self) -> PathBuf {
        match self.manifest_path.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
            _ => PathBuf::from("."),
        }
    }

    fn entry_dir(&self, target: &str, config: &str, entry: &str) -> PathBuf {
        self.build_root.join(target).join(config).join(entry)
    }
}

fn input(what: impl Into<String>, detail: impl Into<String>) -> HarnessError {
    HarnessError::Input {
        what: what.into(),
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

/// One line of text from an arbitrary message.
fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The manifest, validated.
fn load_manifest(scope: &Scope) -> Result<Manifest, HarnessError> {
    let manifest = Manifest::load(&scope.manifest_path)
        .map_err(|e| input(scope.manifest_path.display().to_string(), e.to_string()))?;
    manifest
        .validate()
        .map_err(|e| input(scope.manifest_path.display().to_string(), e.to_string()))?;
    Ok(manifest)
}

/// A selected entry.
struct Selected {
    target: String,
    config: String,
    entry: Entry,
}

/// The entries of the scope, in pair order and then name order.
fn selected_entries(manifest: &Manifest, scope: &Scope) -> Result<Vec<Selected>, HarnessError> {
    let mut selected = Vec::new();
    for (id, config) in &scope.pairs {
        let target = manifest
            .target
            .get(id)
            .ok_or_else(|| input("selection", format!("the manifest has no target {id}")))?;
        if !target.config.iter().any(|c| &c.name == config) {
            return Err(input(
                "selection",
                format!("target {id} has no configuration {config}"),
            ));
        }
        let mut entries: Vec<&Entry> = target.entry.iter().collect();
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        for entry in entries {
            if scope
                .entries
                .as_ref()
                .is_some_and(|names| !names.contains(&entry.name))
            {
                continue;
            }
            selected.push(Selected {
                target: id.clone(),
                config: config.clone(),
                entry: entry.clone(),
            });
        }
    }
    Ok(selected)
}

/// Resolve `.` and `..` without touching the file system.
fn lexical(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => match out.components().next_back() {
                Some(Component::Normal(_)) => {
                    out.pop();
                }
                Some(Component::RootDir | Component::Prefix(_)) => {}
                _ => out.push(".."),
            },
            other => out.push(other.as_os_str()),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Build
// ---------------------------------------------------------------------------

/// How the build of one entry's driver ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildState {
    /// `driver.elf` was built.
    Built {
        /// Its sha256.
        driver_sha256: String,
    },
    /// The linker found symbols undefined.
    LinkUnresolved(Vec<String>),
    /// A compile flag of the defining unit that is neither kept nor dropped.
    UnhandledFlag(String),
    /// The entry declares no roles; not a failure.
    NoRoles,
    /// The pair or the entry cannot be built, with the reason.
    Refused(String),
}

/// One row of a build: the state of one (target, configuration, entry).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildRow {
    /// The target id.
    pub target: String,
    /// The configuration name.
    pub config: String,
    /// The entry function.
    pub entry: String,
    /// How it ended.
    pub state: BuildState,
}

impl BuildRow {
    /// The row text: `<t>/<c>/<e> <state>`.
    #[must_use]
    pub fn render(&self) -> String {
        let key = format!("{}/{}/{}", self.target, self.config, self.entry);
        match &self.state {
            BuildState::Built { driver_sha256 } => {
                format!("{key} built driver_sha256={driver_sha256}")
            }
            BuildState::LinkUnresolved(names) => {
                format!("{key} link_unresolved ({})", names.join(", "))
            }
            BuildState::UnhandledFlag(flag) => format!("{key} unhandled_flag ({flag})"),
            BuildState::NoRoles => format!("{key} no_roles"),
            BuildState::Refused(reason) => format!("{key} refused ({})", one_line(reason)),
        }
    }

    /// True for every state but `built` and `no_roles`.
    #[must_use]
    pub const fn is_failure(&self) -> bool {
        !matches!(self.state, BuildState::Built { .. } | BuildState::NoRoles)
    }
}

/// The record `build.json` holds: how a driver was built, and what it was built
/// from (CONTEXT D-19).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildRecord {
    /// [`BUILD_SCHEMA_VERSION`].
    pub schema_version: u32,
    /// The target id.
    pub target: String,
    /// The configuration name.
    pub config: String,
    /// The entry function.
    pub entry: String,
    /// The toolchain image the capture recorded.
    pub image: String,
    /// `identical` when the pair's replay report was checked, else `not_checked`.
    pub replay: String,
    /// sha256 of each object that went into the archive, by path.
    pub objects: BTreeMap<String, String>,
    /// sha256 of `pair.a`.
    pub archive_sha256: String,
    /// sha256 of `driver.c`.
    pub driver_source_sha256: String,
    /// sha256 of `driver.elf`.
    pub driver_elf_sha256: String,
    /// The include, define and undef lines before the entry header.
    pub include_lines: Vec<String>,
    /// The flags the driver was compiled with besides the baseline.
    pub driver_flags: Vec<String>,
    /// The run record of each build step: `ar`, the compile and the link.
    pub steps: Vec<RunRecord>,
    /// The writable globals present in the linked driver: the dump plan
    /// (CONTEXT D-10).
    #[serde(default)]
    pub globals: Vec<GlobalRecord>,
    /// The layout of each state of the entry, by name, from the pinned GCC's
    /// DWARF (CONTEXT D-09).
    #[serde(default)]
    pub layouts: BTreeMap<String, CType>,
    /// The type of each writable global of the dump plan, by key, from the DWARF
    /// of its defining unit, or the reason it has none (CONTEXT D-10).
    #[serde(default)]
    pub global_types: BTreeMap<String, GlobalType>,
    /// How the return value is recorded, resolved from the DWARF of the return
    /// type; absent in a record made before a typedef was resolved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_kind: Option<ReturnKind>,
    /// The types the pointers of the states point at, by pointee text: the table
    /// the heap walk reads (CONTEXT D-09).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub pointees: BTreeMap<String, CType>,
}

/// One writable global of the dump plan, as `build.json` records it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GlobalRecord {
    /// `<object path>:<symbol>`.
    pub key: String,
    /// The address in the linked driver.
    pub address: u32,
    /// Size in bytes.
    pub size: u32,
    /// The image's initial bytes, lowercase hex.
    pub initial: String,
}

impl GlobalRecord {
    fn from_range(range: &DumpRange) -> Self {
        Self {
            key: range.key.clone(),
            address: range.address,
            size: range.size,
            initial: crate::hex(&range.initial),
        }
    }

    fn to_range(&self) -> Result<DumpRange, String> {
        let digits = self.initial.as_bytes();
        let nibble = |byte: u8| match byte {
            b'0'..=b'9' => Some(byte - b'0'),
            b'a'..=b'f' => Some(byte - b'a' + 10),
            _ => None,
        };
        let mut initial = Vec::with_capacity(digits.len() / 2);
        for pair in digits.chunks(2) {
            let [high, low] = pair else {
                return Err(format!(
                    "{}: the initial bytes are not whole bytes",
                    self.key
                ));
            };
            match (nibble(*high), nibble(*low)) {
                (Some(h), Some(l)) => initial.push(h << 4 | l),
                _ => return Err(format!("{}: the initial bytes are not hex", self.key)),
            }
        }
        if initial.len() != self.size as usize {
            return Err(format!(
                "{}: the initial bytes do not match the size",
                self.key
            ));
        }
        Ok(DumpRange {
            key: self.key.clone(),
            address: self.address,
            size: self.size,
            initial,
        })
    }
}

/// What a build needs.
#[derive(Debug, Clone)]
pub struct BuildOptions {
    /// Which entries.
    pub scope: Scope,
    /// The pinned tools.
    pub tools: ExecTools,
    /// The runner configuration.
    pub cfg: RunnerConfig,
    /// The directory of Phase 2's replay reports (`<dir>/<id>/<config>.json`);
    /// with it a pair is built only when its report starts `identical`.
    pub replay_dir: Option<PathBuf>,
}

/// What is shared by the entries of one pair.
struct PairInputs {
    layout: RunLayout,
    verified: crate::link::VerifiedObjects,
    canonical: Vec<CcEntry>,
    replay: &'static str,
}

/// Load a pair's evidence, or say why the pair cannot be built.
fn load_pair(opts: &BuildOptions, target: &str, config: &str) -> Result<PairInputs, String> {
    let layout = RunLayout::new(&opts.scope.work_dir, target, config);
    let verified = verify_objects(&layout.capture_dir).map_err(|e| e.to_string())?;
    let canonical_path = layout.canonical_path();
    let bytes = std::fs::read(&canonical_path)
        .map_err(|e| format!("cannot read {}: {e}", canonical_path.display()))?;
    let capture_json = std::fs::read(layout.capture_dir.join("capture.json"))
        .map_err(|e| format!("cannot read capture.json: {e}"))?;
    let recorded = serde_json::from_slice::<serde_json::Value>(&capture_json)
        .ok()
        .and_then(|json| {
            json.get("compile_commands")?
                .get("canonical_sha256")?
                .as_str()
                .map(str::to_owned)
        });
    if recorded.as_deref() != Some(&sha256_hex(&bytes)) {
        return Err(
            "compile_commands.json does not match the canonical_sha256 of capture.json".to_owned(),
        );
    }
    let canonical: Vec<CcEntry> = serde_json::from_slice(&bytes)
        .map_err(|e| format!("compile_commands.json is not canonical: {e}"))?;
    let replay = match &opts.replay_dir {
        None => "not_checked",
        Some(dir) => {
            let path = dir.join(target).join(format!("{config}.json"));
            let text = std::fs::read(&path)
                .map_err(|e| format!("no replay report {}: {e}", path.display()))?;
            let verdict = serde_json::from_slice::<serde_json::Value>(&text)
                .ok()
                .and_then(|json| json.get("verdict")?.as_str().map(str::to_owned))
                .ok_or_else(|| format!("{} has no verdict", path.display()))?;
            if !verdict.starts_with("identical") {
                return Err(format!(
                    "the replay verdict of {target}/{config} is {verdict}, not identical"
                ));
            }
            "identical"
        }
    };
    Ok(PairInputs {
        layout,
        verified,
        canonical,
        replay,
    })
}

/// The include lines for an entry's driver: the defining unit's reproducible
/// prefix and the line that includes the entry header (D-23, Phase 2's probe).
fn include_lines(pair: &PairInputs, entry: &Entry, unit: &CcEntry) -> Result<Vec<String>, String> {
    let defining = lexical(&pair.layout.src.join(&entry.source));
    let header = lexical(&pair.layout.src.join(&entry.header));
    let flags = filter_flags(unit.arguments.get(1..).unwrap_or_default());
    let cwd = Path::new(&unit.directory);
    let text = match unit_include_prefix(&defining, &header, cwd, &flags) {
        IncludePrefix::Reproduced(lines) => probe_source_with_prefix(&lines, &header),
        IncludePrefix::HeaderFirst | IncludePrefix::NotReproducible(_) => probe_source(&header),
    }
    .map_err(|e| e.to_string())?;
    Ok(text.lines().map(str::to_owned).collect())
}

/// The defining unit's compile command.
fn defining_unit<'a>(pair: &'a PairInputs, entry: &Entry) -> Option<&'a CcEntry> {
    let defining = lexical(&pair.layout.src.join(&entry.source));
    pair.canonical.iter().find(|candidate| {
        lexical(&Path::new(&candidate.directory).join(&candidate.file)) == defining
    })
}

/// Build the driver of every entry with roles of the scope.
///
/// # Errors
/// [`HarnessError`] when the manifest cannot be used or a tool cannot run. A pair
/// or an entry that cannot be built is a [`BuildRow`], not an error.
pub fn build_entries(opts: &BuildOptions) -> Result<Vec<BuildRow>, HarnessError> {
    let manifest = load_manifest(&opts.scope)?;
    let selected = selected_entries(&manifest, &opts.scope)?;
    let mut rows = Vec::new();
    let mut pair_cache: BTreeMap<(String, String), Result<PairInputs, String>> = BTreeMap::new();
    for item in selected {
        let row = |state| BuildRow {
            target: item.target.clone(),
            config: item.config.clone(),
            entry: item.entry.name.clone(),
            state,
        };
        let adapter = match Adapter::from_entry(&item.target, &item.config, &item.entry) {
            Ok(adapter) => adapter,
            Err(AdapterError::NoRoles) => {
                rows.push(row(BuildState::NoRoles));
                continue;
            }
            Err(error) => {
                rows.push(row(BuildState::Refused(error.to_string())));
                continue;
            }
        };
        let key = (item.target.clone(), item.config.clone());
        let pair = pair_cache
            .entry(key)
            .or_insert_with(|| load_pair(opts, &item.target, &item.config));
        let pair = match pair {
            Ok(pair) => &*pair,
            Err(reason) => {
                rows.push(row(BuildState::Refused(reason.clone())));
                continue;
            }
        };
        let state = build_entry(opts, pair, &item, &adapter)?;
        rows.push(row(state));
    }
    Ok(rows)
}

fn build_entry(
    opts: &BuildOptions,
    pair: &PairInputs,
    item: &Selected,
    adapter: &Adapter,
) -> Result<BuildState, HarnessError> {
    let Some(unit) = defining_unit(pair, &item.entry) else {
        return Ok(BuildState::Refused(format!(
            "the defining unit {} is not in the canonical compile commands",
            item.entry.source
        )));
    };
    let flags = match driver_flags(&unit.arguments) {
        Ok(flags) => flags,
        Err(AdapterError::UnhandledFlag(flag)) => return Ok(BuildState::UnhandledFlag(flag)),
        Err(error) => return Ok(BuildState::Refused(error.to_string())),
    };
    let lines = match include_lines(pair, &item.entry, unit) {
        Ok(lines) => lines,
        Err(reason) => return Ok(BuildState::Refused(reason)),
    };
    let dir = opts
        .scope
        .entry_dir(&item.target, &item.config, &item.entry.name);
    std::fs::create_dir_all(&dir).map_err(|e| io("cannot create", &dir, e))?;
    // The probe lays out every state and, for a return that is not void, the
    // returned value: a typedef name does not say whether it is an integer, an
    // enum or a struct, so the DWARF of the entry's header decides (D-06, D-11).
    let mut probes: Vec<(String, String)> = adapter
        .states()
        .into_iter()
        .map(|state| (state.name, state.type_text))
        .collect();
    let resolve_return =
        adapter.return_kind() == ReturnKind::Integer && adapter.return_text() != "void";
    if resolve_return {
        probes.push((RET_NAME.to_owned(), adapter.return_text().to_owned()));
    }
    let mut layouts = match layout_for_entry(
        &opts.tools,
        &opts.cfg,
        &LayoutRequest {
            dir: &dir,
            compile_cwd: Path::new(&unit.directory),
            include_lines: &lines,
            flags: &flags,
            states: &probes,
        },
    ) {
        Ok(layouts) => layouts,
        Err(error @ (HarnessError::Layout { .. } | HarnessError::Build { .. })) => {
            return Ok(BuildState::Refused(error.to_string()));
        }
        Err(error) => return Err(error),
    };
    let mut adapter = adapter.clone();
    if resolve_return {
        let kind = match layouts.states.get(RET_NAME).map(|ty| &ty.kind) {
            Some(Kind::Int { .. } | Kind::Enum { .. }) | None => ReturnKind::Integer,
            Some(Kind::Struct { .. } | Kind::Union { .. }) => ReturnKind::Struct,
            Some(Kind::Pointer { .. }) => ReturnKind::Pointer,
            Some(Kind::Float | Kind::Array { .. }) => {
                return Ok(BuildState::Refused(format!(
                    "the return type {} cannot be recorded as a 32-bit rc or as a returned value",
                    adapter.return_text()
                )));
            }
        };
        if kind == ReturnKind::Integer {
            layouts.states.remove(RET_NAME);
        }
        adapter = adapter.with_return_kind(kind);
    }
    if let Err(reason) = check_marks(&layouts.states, adapter.marks()) {
        return Ok(BuildState::Refused(format!("unobserved mark {reason}")));
    }
    let source = render_entry_driver(&adapter, &lines)?;
    let members: Vec<String> = pair.verified.objects.keys().cloned().collect();
    let archived = archive(
        &opts.tools,
        &opts.cfg,
        &pair.verified.objects_dir,
        &members,
        &dir.join("pair.a"),
    )?;
    let request = LinkRequest {
        dir: &dir,
        source: &source,
        flags: &flags,
        compile_cwd: Path::new(&unit.directory),
        archive: &archived.path,
    };
    match link_driver(&opts.tools, &opts.cfg, &request)? {
        LinkOutcome::Unresolved(names) => Ok(BuildState::LinkUnresolved(names)),
        LinkOutcome::Linked(build) => {
            let plan = match dump_plan(pair, &build.elf_path) {
                Ok(plan) => plan,
                Err(reason) => return Ok(BuildState::Refused(reason)),
            };
            let global_types = match global_types_of(opts, pair, &dir, &plan) {
                Ok(types) => types,
                Err(error @ (HarnessError::Layout { .. } | HarnessError::Build { .. })) => {
                    return Ok(BuildState::Refused(error.to_string()));
                }
                Err(error) => return Err(error),
            };
            let mut steps = vec![archived.record.clone()];
            steps.extend(build.records.iter().cloned());
            let record = BuildRecord {
                schema_version: BUILD_SCHEMA_VERSION,
                target: item.target.clone(),
                config: item.config.clone(),
                entry: item.entry.name.clone(),
                image: pair.verified.image.clone(),
                replay: pair.replay.to_owned(),
                objects: pair.verified.objects.clone(),
                archive_sha256: archived.sha256.clone(),
                driver_source_sha256: build.source_sha256.clone(),
                driver_elf_sha256: build.elf_sha256.clone(),
                include_lines: lines,
                driver_flags: flags,
                steps,
                globals: plan.iter().map(GlobalRecord::from_range).collect(),
                layouts: layouts.states,
                global_types,
                return_kind: Some(adapter.return_kind()),
                pointees: layouts.pointees,
            };
            let path = dir.join(BUILD_JSON);
            let mut text = serde_json::to_string_pretty(&record)
                .map_err(|e| input(path.display().to_string(), e.to_string()))?;
            text.push('\n');
            std::fs::write(&path, text).map_err(|e| io("cannot write", &path, e))?;
            Ok(BuildState::Built {
                driver_sha256: build.elf_sha256.clone(),
            })
        }
    }
}

/// The writable globals of the pair's objects that are present in the linked
/// driver, with their addresses and initial bytes (CONTEXT D-10). The objects
/// are read again and checked against the verified hashes.
fn dump_plan(pair: &PairInputs, elf_path: &Path) -> Result<Vec<DumpRange>, String> {
    let mut symbols = Vec::new();
    for (relative, expected) in &pair.verified.objects {
        let path = pair.verified.objects_dir.join(relative);
        let bytes = std::fs::read(&path).map_err(|e| format!("cannot read {relative}: {e}"))?;
        if &sha256_hex(&bytes) != expected {
            return Err(format!("{relative} changed while the driver was built"));
        }
        symbols.extend(writable_globals(relative, &bytes).map_err(|e| e.to_string())?);
    }
    let elf = std::fs::read(elf_path).map_err(|e| format!("cannot read driver.elf: {e}"))?;
    locate_globals(&symbols, &elf).map_err(|e| e.to_string())
}

/// Type each global of the dump plan from the DWARF of the unit that defines it:
/// the canonical compile command whose output is the global's object is run
/// again with `-g -gdwarf-5 -O0` into a scratch object (CONTEXT D-10). A global
/// whose object no canonical command writes is untyped.
fn global_types_of(
    opts: &BuildOptions,
    pair: &PairInputs,
    dir: &Path,
    plan: &[DumpRange],
) -> Result<BTreeMap<String, GlobalType>, HarnessError> {
    let mut targets = Vec::new();
    let mut types = BTreeMap::new();
    for range in plan {
        let Some((object, symbol)) = range.key.rsplit_once(':') else {
            types.insert(
                range.key.clone(),
                GlobalType::Untyped {
                    reason: "the key is not <object>:<symbol>".to_owned(),
                },
            );
            continue;
        };
        let wanted = lexical(&pair.layout.run_root.join(object));
        match pair
            .canonical
            .iter()
            .find(|candidate| lexical(Path::new(&candidate.output)) == wanted)
        {
            Some(unit) => targets.push(GlobalTarget {
                key: range.key.clone(),
                symbol: symbol.to_owned(),
                unit: unit.clone(),
            }),
            None => {
                types.insert(
                    range.key.clone(),
                    GlobalType::Untyped {
                        reason: "no canonical compile command writes that object".to_owned(),
                    },
                );
            }
        }
    }
    types.extend(layout_for_globals(&opts.tools, &opts.cfg, dir, &targets)?);
    Ok(types)
}

/// The lines `mt vectors build` prints: every row but the `no_roles` rows, then
/// `no_roles: <n>` when there were any. The testbed workflow publishes the listing
/// as one annotation and GitHub keeps at most 50 annotations per job, so the many
/// rows of entries without roles (one per configuration) are counted in one line.
#[must_use]
pub fn render_build_listing(rows: &[BuildRow]) -> Vec<String> {
    let mut lines: Vec<String> = rows
        .iter()
        .filter(|row| !matches!(row.state, BuildState::NoRoles))
        .map(BuildRow::render)
        .collect();
    let skipped = rows.len() - lines.len();
    if skipped > 0 {
        lines.push(format!("no_roles: {skipped}"));
    }
    lines
}

// ---------------------------------------------------------------------------
// Run
// ---------------------------------------------------------------------------

/// What a run needs.
#[derive(Debug, Clone)]
pub struct RunOptions {
    /// Which entries.
    pub scope: Scope,
    /// The pinned tools.
    pub tools: ExecTools,
    /// The runner configuration.
    pub cfg: RunnerConfig,
    /// Where the corpora go: `<dir>/<target>/<config>/<entry>/`. Each entry's
    /// directory must be absent or empty.
    pub corpus_dir: PathBuf,
    /// Where the run's scratch slots go.
    pub scratch_dir: PathBuf,
}

/// The counts of one entry's run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryRunSummary {
    /// Vectors written.
    pub vectors: usize,
    /// Vectors with outcome `ok`.
    pub ok: usize,
    /// Vectors with outcome `fault`.
    pub fault: usize,
    /// Vectors with outcome `abort`.
    pub abort: usize,
    /// Vectors with outcome `timeout`.
    pub timeout: usize,
    /// Vectors with outcome `setup_failed`.
    pub setup_failed: usize,
    /// (seed, combination) pairs that were not run, and why.
    pub skipped: usize,
    /// The distinct reasons seeds were skipped, sorted.
    pub skipped_notes: Vec<String>,
    /// Every vector's two runs were byte-identical (a difference fails the
    /// run, so this is true in every summary that exists).
    pub twice_identical: bool,
}

impl EntryRunSummary {
    /// The row text after the entry key.
    #[must_use]
    pub fn render(&self) -> String {
        let notes = if self.skipped_notes.is_empty() {
            String::new()
        } else {
            format!(" ({})", self.skipped_notes.join("; "))
        };
        format!(
            "vectors={} ok={} fault={} abort={} timeout={} setup_failed={} skipped={}{notes} twice={}",
            self.vectors,
            self.ok,
            self.fault,
            self.abort,
            self.timeout,
            self.setup_failed,
            self.skipped,
            if self.twice_identical {
                "identical"
            } else {
                "differs"
            }
        )
    }
}

/// How the run of one entry ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunState {
    /// The vectors were written.
    Ran(EntryRunSummary),
    /// The entry declares no roles; not a failure.
    NoRoles,
    /// The driver was not built (or changed since it was built).
    NotBuilt(String),
    /// The run failed, with the seed named where one is at fault; nothing was
    /// written for the entry.
    Failed(String),
}

/// One row of a run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunRow {
    /// The target id.
    pub target: String,
    /// The configuration name.
    pub config: String,
    /// The entry function.
    pub entry: String,
    /// How it ended.
    pub state: RunState,
}

impl RunRow {
    /// The row text: `<t>/<c>/<e> <state>`.
    #[must_use]
    pub fn render(&self) -> String {
        let key = format!("{}/{}/{}", self.target, self.config, self.entry);
        match &self.state {
            RunState::Ran(summary) => format!("{key} {}", summary.render()),
            RunState::NoRoles => format!("{key} no_roles"),
            RunState::NotBuilt(reason) => format!("{key} not_built ({})", one_line(reason)),
            RunState::Failed(reason) => format!("{key} failed ({})", one_line(reason)),
        }
    }

    /// True for `not_built` and `failed`.
    #[must_use]
    pub const fn is_failure(&self) -> bool {
        matches!(self.state, RunState::NotBuilt(_) | RunState::Failed(_))
    }
}

/// The lines `mt vectors run` prints: every row but the `no_roles` rows, then
/// `no_roles: <n>` when there were any (see [`render_build_listing`]).
#[must_use]
pub fn render_run_listing(rows: &[RunRow]) -> Vec<String> {
    let mut lines: Vec<String> = rows
        .iter()
        .filter(|row| !matches!(row.state, RunState::NoRoles))
        .map(RunRow::render)
        .collect();
    let skipped = rows.len() - lines.len();
    if skipped > 0 {
        lines.push(format!("no_roles: {skipped}"));
    }
    lines
}

/// A linked driver, its dump plan and its state layouts, ready to run.
#[derive(Debug, Clone, Default)]
pub struct PreparedDriver {
    /// The linked `driver.elf`.
    pub elf_path: PathBuf,
    /// The writable globals to snapshot around each call.
    pub plan: Vec<DumpRange>,
    /// The layout of each state of the entry, by name.
    pub layouts: BTreeMap<String, CType>,
    /// The type of each writable global of `plan`, by key.
    pub global_types: BTreeMap<String, GlobalType>,
    /// The functions of the linked driver: address with the Thumb bit cleared
    /// to name.
    pub functions: BTreeMap<u32, String>,
    /// The types the pointers of the states point at, by pointee text (the heap
    /// walk's table).
    pub pointees: BTreeMap<String, CType>,
}

/// One run of one request, turned into the vector it would store.
#[derive(Debug, Clone)]
pub struct Observed {
    /// The vector.
    pub vector: Vector,
    /// Bytes the C code wrote through libc and the driver discarded.
    pub discarded: u32,
    /// How the run ended.
    pub outcome: Outcome,
}

fn has_ub_source(outcome: &Outcome) -> HasUbSource {
    match outcome {
        Outcome::Fault {
            signal: Some(_), ..
        } => HasUbSource::ObservedCrash,
        Outcome::Fault { signal: None, .. } => HasUbSource::OobWrite,
        Outcome::Abort => HasUbSource::ObservedAbort,
        Outcome::Timeout => HasUbSource::Timeout,
        Outcome::Ok { .. } | Outcome::SetupFailed { .. } | Outcome::ExecutorError { .. } => {
            HasUbSource::Unchecked
        }
    }
}

/// The executor's own text of a vector's `stderr`: `outcome=<class>`, and for a
/// failed setup step ` step=<k>` (CONTEXT D-12).
fn stderr_pattern(outcome: &Outcome) -> String {
    match outcome {
        Outcome::SetupFailed { step, .. } => {
            format!("outcome={} step={step}", outcome.class_word())
        }
        _ => format!("outcome={}", outcome.class_word()),
    }
}

/// Judge one run and build the vector, or say why no vector can be made.
fn observe(
    adapter: &Adapter,
    combination: &Combination,
    seed: &[u8],
    driver: &PreparedDriver,
    output: &RunOutput,
) -> Result<Observed, String> {
    let outcome = classify(output);
    if let Outcome::ExecutorError { reason } = &outcome {
        return Err(format!("executor_error: {reason}"));
    }
    let (out_bytes, discarded, lib_state_in, lib_state_out) = match &outcome {
        Outcome::Ok { .. } | Outcome::Fault { signal: None, .. } | Outcome::SetupFailed { .. } => {
            let frame = parse_frame(&output.stdout).map_err(|e| format!("result frame: {e}"))?;
            let out = frame
                .region(OUT_REGION)
                .map(|region| region.bytes.clone())
                .ok_or_else(|| "the result frame has no output region".to_owned())?;
            // A failed setup step means the entry did not run: nothing was returned.
            let entry_ran = !matches!(outcome, Outcome::SetupFailed { .. });
            let (before, after) = record_state(adapter, driver, &frame, entry_ran)?;
            (out, frame.discarded, before, after)
        }
        _ => (Vec::new(), 0, LibState::default(), LibState::default()),
    };
    let has_ub = outcome
        .has_ub_label()
        .map(HasUbLabel::new)
        .transpose()
        .map_err(|e| e.to_string())?;
    let vector = Vector {
        argv: adapter.argv(seed.len(), combination),
        stdin: encode_latin1(seed),
        env: BTreeMap::new(),
        stdout: Output {
            pattern: encode_latin1(&out_bytes),
            is_regex: false,
        },
        stderr: Output {
            pattern: stderr_pattern(&outcome),
            is_regex: false,
        },
        rc: outcome.rc().unwrap_or(0),
        lib_state_in,
        lib_state_out,
        has_ub,
    };
    Ok(Observed {
        vector,
        discarded,
        outcome,
    })
}

/// What one run needs besides the driver and the request.
#[derive(Debug, Clone, Copy)]
pub struct Exec<'a> {
    /// The pinned tools.
    pub tools: &'a ExecTools,
    /// The runner configuration.
    pub cfg: &'a RunnerConfig,
    /// The budgets of the run.
    pub budgets: &'a Budgets,
}

/// Run one seed with one scalar combination twice through the driver, judge both
/// runs and require the two vectors to be byte-identical (CONTEXT D-02). The
/// inner `Err` says why no vector can be stored: an executor error, an
/// `untyped_global_changed`, or two runs that differ. Nothing is retried.
///
/// # Errors
/// [`HarnessError`] when the request cannot be written or `qemu-arm` cannot be
/// started.
pub fn run_vector_twice(
    exec: &Exec<'_>,
    adapter: &Adapter,
    driver: &PreparedDriver,
    slot_dir: &Path,
    seed: &[u8],
    combination: &Combination,
) -> Result<Result<Observed, String>, HarnessError> {
    let ranges: Vec<(u32, u32)> = driver
        .plan
        .iter()
        .map(|range| (range.address, range.size))
        .collect();
    let request = Request::entry(
        seed,
        adapter.output_capacity(combination),
        &adapter.scalar_words(combination),
    )
    .with_dump_plan(&ranges)
    .with_strings(&adapter.strings(combination));
    let mut observed = Vec::with_capacity(2);
    for _ in 0..2 {
        let slot = Slot::create(slot_dir)?;
        let output = run_driver(
            exec.tools,
            exec.cfg,
            exec.budgets,
            &driver.elf_path,
            &slot,
            &request,
        )?;
        match observe(adapter, combination, seed, driver, &output) {
            Ok(done) => observed.push(done),
            Err(reason) => return Ok(Err(reason)),
        }
    }
    let (Some(second), Some(first)) = (observed.pop(), observed.pop()) else {
        return Ok(Err("fewer than two runs were observed".to_owned()));
    };
    let same = match (
        first.vector.to_canonical_json(),
        second.vector.to_canonical_json(),
    ) {
        (Ok(a), Ok(b)) => a == b && first.discarded == second.discarded,
        _ => false,
    };
    if same {
        Ok(Ok(first))
    } else {
        Ok(Err("the two runs differ".to_owned()))
    }
}

/// The words of the addresses region.
fn address_words(frame: &Frame) -> Result<Vec<u32>, String> {
    let region = frame
        .region(ADDRESSES_REGION)
        .ok_or_else(|| "the result frame has no address region".to_owned())?;
    if region.bytes.len() % 4 != 0 {
        return Err("the address region is not whole words".to_owned());
    }
    Ok(region
        .bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|word| u32::from_le_bytes(*word))
        .collect())
}

/// The regions a pointer of the state can point into (CONTEXT D-06): the input
/// and output buffers, each state by its name, the heap arena in use, each
/// dumped global as `global:<object>:<symbol>`, and the functions of the driver.
/// The addresses come from the driver's own frame and its symbol table, so no
/// address is assumed.
fn regions_of(
    words: &[u32],
    states: &[crate::adapter::StateParam],
    driver: &PreparedDriver,
) -> Result<Regions, String> {
    let word = |index: usize| {
        words
            .get(index)
            .copied()
            .ok_or_else(|| "the address region is too short".to_owned())
    };
    let mut regions = Regions::new()
        .with_region("in", word(0)?, word(1)?)
        .with_region("out", word(2)?, word(3)?)
        .with_region("heap", word(4)?, word(5)?);
    for (index, state) in states.iter().enumerate() {
        regions = regions.with_region(&state.name, word(6 + 2 * index)?, word(7 + 2 * index)?);
    }
    for range in &driver.plan {
        regions = regions.with_region(&format!("global:{}", range.key), range.address, range.size);
    }
    Ok(regions.with_functions(&driver.functions))
}

/// The bytes of region `base + index`.
fn region_bytes(frame: &Frame, base: u32, index: usize, what: &str) -> Result<Vec<u8>, String> {
    let index = u32::try_from(index).map_err(|_| "too many regions".to_owned())?;
    frame
        .region(base + index)
        .map(|region| region.bytes.clone())
        .ok_or_else(|| format!("the result frame has no region for {what}"))
}

/// Decode every state of the entry and every changed global, before and after the
/// call, apply the entry's unobserved marks and return `lib_state_in` and
/// `lib_state_out` (CONTEXT D-06, D-09, D-10).
fn record_state(
    adapter: &Adapter,
    driver: &PreparedDriver,
    frame: &Frame,
    entry_ran: bool,
) -> Result<(LibState, LibState), String> {
    let states = adapter.states();
    let words = address_words(frame)?;
    let regions = regions_of(&words, &states, driver)?;
    let mut before = BTreeMap::new();
    let mut after = BTreeMap::new();
    for (index, state) in states.iter().enumerate() {
        let ty = driver
            .layouts
            .get(&state.name)
            .ok_or_else(|| format!("state {} has no layout in build.json", state.name))?;
        for (base, target) in [
            (STATE_BEFORE_REGION, &mut before),
            (STATE_AFTER_REGION, &mut after),
        ] {
            if state.name == RET_NAME && (base == STATE_BEFORE_REGION || !entry_ran) {
                // Nothing is returned before the call, nor when setup stopped first.
                continue;
            }
            let bytes = region_bytes(frame, base, index, &format!("state {}", state.name))?;
            if bytes.len() != ty.size as usize {
                return Err(format!(
                    "executor_error: state {} is {} bytes in the driver but {} in the DWARF layout",
                    state.name,
                    bytes.len(),
                    ty.size
                ));
            }
            let value = decode_value(ty, &bytes, &regions)
                .map_err(|e| format!("state {}: {e}", state.name))?;
            let value = apply_marks(&state.name, value, adapter.marks())
                .map_err(|e| format!("state {}: {e}", state.name))?;
            target.insert(state.name.clone(), value);
        }
    }
    let mut before_bytes = Vec::new();
    let mut after_bytes = Vec::new();
    for (index, range) in driver.plan.iter().enumerate() {
        before_bytes.push(region_bytes(frame, DUMP_BEFORE_REGION, index, &range.key)?);
        after_bytes.push(region_bytes(frame, DUMP_AFTER_REGION, index, &range.key)?);
    }
    let before_refs: Vec<&[u8]> = before_bytes.iter().map(Vec::as_slice).collect();
    let after_refs: Vec<&[u8]> = after_bytes.iter().map(Vec::as_slice).collect();
    if let Some((globals_before, globals_after)) = record_globals(
        &driver.plan,
        &driver.global_types,
        &before_refs,
        &after_refs,
        &regions,
    )? {
        before.insert("globals".to_owned(), globals_before);
        after.insert("globals".to_owned(), globals_after);
    }
    // The objects of the heap that the recorded pointers reach (D-06, D-09).
    let arena = region_bytes(frame, HEAP_REGION, 0, "the heap arena")?;
    let objects = {
        let mut roots: Vec<&mut StateValue> = after.values_mut().collect();
        walk_heap(&arena, &driver.pointees, &regions, &mut roots)
            .map_err(|e| format!("heap: {e}"))?
    };
    if !objects.is_empty() {
        after.insert(
            "heap".to_owned(),
            StateValue::Struct {
                c_type: "heap".to_owned(),
                fields: objects,
            },
        );
    }
    // A mutable input is recorded as it is after the call.
    if adapter.mutable_input() {
        let bytes = region_bytes(frame, IN_AFTER_REGION, 0, "the input after the call")?;
        after.insert(
            "in".to_owned(),
            StateValue::Bytes {
                c_type: format!("{}[{}]", adapter.input_element_type(), bytes.len()),
                hex: crate::hex(&bytes),
            },
        );
    }
    let before = LibState::new(before).map_err(|e| e.to_string())?;
    let after = LibState::new(after).map_err(|e| e.to_string())?;
    Ok((before, after))
}

fn policy_of(scope: &Scope, target: &str) -> Result<String, String> {
    let path = scope
        .testbed_dir()
        .join("policy")
        .join(format!("{target}.toml"));
    let bytes = std::fs::read(&path).map_err(|e| format!("no policy {}: {e}", path.display()))?;
    let policy = Policy::load(&path).map_err(|e| format!("policy {}: {e}", path.display()))?;
    policy
        .validate(&[])
        .map_err(|e| format!("policy {}: {e}", path.display()))?;
    Ok(sha256_hex(&bytes))
}

/// The budgets of a run as the provenance records them (CONTEXT D-19).
#[must_use]
pub fn budgets_record(budgets: &Budgets) -> ProvenanceBudgets {
    ProvenanceBudgets {
        timeout_ms: budgets.timeout_ms,
        reserved_va: u64::from(budgets.reserved_va),
        heap_arena: u64::from(budgets.heap_arena),
        sentinel: budgets.sentinel,
        slack_fill: budgets.slack_fill,
        max_input: u64::from(budgets.max_input),
    }
}

/// Run every entry with roles of the scope.
///
/// # Errors
/// [`HarnessError`] when the manifest cannot be used or a tool cannot run. An
/// entry that cannot be run, or whose two runs of a vector differ, is a
/// [`RunRow`] with state [`RunState::Failed`]; nothing is written for it.
pub fn run_entries(opts: &RunOptions) -> Result<Vec<RunRow>, HarnessError> {
    let manifest = load_manifest(&opts.scope)?;
    let selected = selected_entries(&manifest, &opts.scope)?;
    let mut rows = Vec::new();
    for item in selected {
        let state = run_entry(opts, &item)?;
        rows.push(RunRow {
            target: item.target,
            config: item.config,
            entry: item.entry.name,
            state,
        });
    }
    Ok(rows)
}

/// A built entry: its adapter, its build record and the driver ready to run.
#[derive(Debug, Clone)]
pub(crate) struct LoadedDriver {
    pub adapter: Adapter,
    pub build: BuildRecord,
    pub driver: PreparedDriver,
}

/// Load the build of `entry` from `scope.build_root` and check `driver.elf`
/// against the hash `build.json` records. The error is the [`RunState`] the row
/// takes.
pub(crate) fn load_driver(
    scope: &Scope,
    target: &str,
    config: &str,
    entry: &Entry,
) -> Result<LoadedDriver, RunState> {
    let mut adapter = match Adapter::from_entry(target, config, entry) {
        Ok(adapter) => adapter,
        Err(AdapterError::NoRoles) => return Err(RunState::NoRoles),
        Err(error) => return Err(RunState::Failed(error.to_string())),
    };
    let key = adapter.argv0();
    let dir = scope.entry_dir(target, config, &entry.name);
    let build_path = dir.join(BUILD_JSON);
    let build: BuildRecord = match std::fs::read(&build_path) {
        Ok(bytes) => match serde_json::from_slice(&bytes) {
            Ok(build) => build,
            Err(error) => return Err(RunState::NotBuilt(format!("{key}: build.json: {error}"))),
        },
        Err(error) => {
            return Err(RunState::NotBuilt(format!(
                "{key}: no {}: {error}",
                build_path.display()
            )));
        }
    };
    if let Some(kind) = build.return_kind {
        adapter = adapter.with_return_kind(kind);
    }
    let elf_path = dir.join("driver.elf");
    let (elf_hash, functions) = match std::fs::read(&elf_path) {
        Ok(bytes) => match function_symbols(&bytes) {
            Ok(functions) => (sha256_hex(&bytes), functions),
            Err(error) => return Err(RunState::NotBuilt(format!("{key}: {error}"))),
        },
        Err(error) => {
            return Err(RunState::NotBuilt(format!(
                "{key}: no {}: {error}",
                elf_path.display()
            )));
        }
    };
    if elf_hash != build.driver_elf_sha256 {
        return Err(RunState::NotBuilt(format!(
            "{key}: driver.elf has sha256 {elf_hash}, build.json records {}",
            build.driver_elf_sha256
        )));
    }
    let mut plan = Vec::new();
    for record in &build.globals {
        match record.to_range() {
            Ok(range) => plan.push(range),
            Err(reason) => return Err(RunState::NotBuilt(format!("{key}: build.json: {reason}"))),
        }
    }
    let layouts = build.layouts.clone();
    let global_types = build.global_types.clone();
    let pointees = build.pointees.clone();
    for state in adapter.states() {
        if !layouts.contains_key(&state.name) {
            return Err(RunState::NotBuilt(format!(
                "{key}: build.json has no layout for state {}",
                state.name
            )));
        }
    }
    Ok(LoadedDriver {
        adapter,
        build,
        driver: PreparedDriver {
            elf_path,
            plan,
            layouts,
            global_types,
            functions,
            pointees,
        },
    })
}

fn run_entry(opts: &RunOptions, item: &Selected) -> Result<RunState, HarnessError> {
    let loaded = match load_driver(&opts.scope, &item.target, &item.config, &item.entry) {
        Ok(loaded) => loaded,
        Err(state) => return Ok(state),
    };
    let LoadedDriver {
        adapter,
        build,
        driver,
    } = loaded;
    let key = adapter.argv0();
    let policy_sha256 = match policy_of(&opts.scope, &item.target) {
        Ok(sha) => sha,
        Err(reason) => return Ok(RunState::Failed(format!("{key}: {reason}"))),
    };
    let combos = match combinations(&adapter) {
        Ok(combos) => combos,
        Err(error) => return Ok(RunState::Failed(format!("{key}: {error}"))),
    };
    let seeds_dir = opts.scope.testbed_dir().join("seeds").join(&item.target);
    let index_path = seeds_dir.join("index.json");
    let index: SeedIndex = match std::fs::read(&index_path) {
        Ok(bytes) => match serde_json::from_slice(&bytes) {
            Ok(index) => index,
            Err(error) => {
                return Ok(RunState::Failed(format!(
                    "{key}: {}: {error}",
                    index_path.display()
                )));
            }
        },
        Err(error) => {
            return Ok(RunState::Failed(format!(
                "{key}: no {}: {error}",
                index_path.display()
            )));
        }
    };
    // D-22, D-28: only the seeds whose index lists this entry, never borrowed.
    let seeds: Vec<_> = index
        .seeds
        .iter()
        .filter(|seed| seed.entries.contains(&item.entry.name))
        .collect();
    if seeds.is_empty() {
        return Ok(RunState::Failed(format!(
            "{key}: no seed of {} lists the entry",
            item.target
        )));
    }

    let budgets = Budgets::DEFAULT;
    let exec = Exec {
        tools: &opts.tools,
        cfg: &opts.cfg,
        budgets: &budgets,
    };
    let slot_dir = opts.scratch_dir.join(format!(
        "{}-{}-{}",
        item.target, item.config, item.entry.name
    ));
    let mut summary = EntryRunSummary {
        vectors: 0,
        ok: 0,
        fault: 0,
        abort: 0,
        timeout: 0,
        setup_failed: 0,
        skipped: 0,
        skipped_notes: Vec::new(),
        twice_identical: true,
    };
    let mut set: Vec<(Vector, Provenance)> = Vec::new();
    for seed in seeds {
        let seed_path = seeds_dir.join(&seed.sha256);
        let bytes = std::fs::read(&seed_path).map_err(|e| io("cannot read", &seed_path, e))?;
        if sha256_hex(&bytes) != seed.sha256 || bytes.len() as u64 != seed.bytes {
            return Ok(RunState::Failed(format!(
                "{key}: seed {} does not match its index record",
                seed.sha256
            )));
        }
        let too_long = if bytes.len() > budgets.max_input as usize {
            Some(format!("longer than max_input {}", budgets.max_input))
        } else {
            adapter
                .max_input()
                .filter(|max| bytes.len() as u64 > *max)
                .map(|max| format!("longer than the in_len max {max}"))
        };
        if let Some(reason) = too_long {
            summary.skipped += combos.len();
            if !summary.skipped_notes.contains(&reason) {
                summary.skipped_notes.push(reason);
            }
            continue;
        }
        for combo in &combos {
            let first = match run_vector_twice(&exec, &adapter, &driver, &slot_dir, &bytes, combo)?
            {
                Ok(first) => first,
                Err(reason) => {
                    return Ok(RunState::Failed(format!(
                        "{key} seed {} combination {:?}: {reason}",
                        seed.sha256, combo.values
                    )));
                }
            };
            let outcome = &first.outcome;
            match outcome {
                Outcome::Ok { .. } => summary.ok += 1,
                Outcome::Fault { .. } => summary.fault += 1,
                Outcome::Abort => summary.abort += 1,
                Outcome::Timeout => summary.timeout += 1,
                Outcome::SetupFailed { .. } => summary.setup_failed += 1,
                Outcome::ExecutorError { .. } => {}
            }
            let provenance = Provenance {
                seed: seed.sha256.clone(),
                scalars: adapter.scalars_by_name(combo),
                objects: build.objects.clone(),
                archive: build.archive_sha256.clone(),
                driver_source: build.driver_source_sha256.clone(),
                driver_elf: build.driver_elf_sha256.clone(),
                image: build.image.clone(),
                executor_version: EXECUTOR_VERSION.to_owned(),
                contract_version: CONTRACT_VERSION.to_owned(),
                policy_sha256: policy_sha256.clone(),
                budgets: budgets_record(&budgets),
                outcome: outcome.class_word().to_owned(),
                has_ub_source: has_ub_source(outcome),
                unobserved: (first.vector.lib_state_in.unobserved_count()
                    + first.vector.lib_state_out.unobserved_count())
                    as u64,
                discarded_output_bytes: u64::from(first.discarded),
                replay: build.replay.clone(),
            };
            set.push((first.vector.clone(), provenance));
        }
    }
    summary.skipped_notes.sort();
    if set.is_empty() {
        return Ok(RunState::Failed(format!(
            "{key}: every seed was skipped; no vector to store"
        )));
    }
    summary.vectors = set.len();
    let corpus = opts
        .corpus_dir
        .join(&item.target)
        .join(&item.config)
        .join(&item.entry.name);
    if let Err(error) = write_corpus(&corpus, &set) {
        return Ok(RunState::Failed(format!("{key}: {error}")));
    }
    Ok(RunState::Ran(summary))
}

// ---------------------------------------------------------------------------
// Layouts
// ---------------------------------------------------------------------------

/// The layouts of every built entry under `build_root`, one line per state:
/// `<target>/<config>/<entry> <state> <type> size=<n> <member>@<offset>:<size>...`
/// (see [`layout_summary`]), in target, configuration, entry and state-name order.
/// A missing build root gives no line.
///
/// # Errors
/// [`HarnessError`] when a directory or a `build.json` cannot be read.
pub fn layout_lines(build_root: &Path) -> Result<Vec<String>, HarnessError> {
    fn sorted_dirs(dir: &Path) -> Result<Vec<PathBuf>, HarnessError> {
        let mut paths = Vec::new();
        for item in std::fs::read_dir(dir).map_err(|e| io("cannot read", dir, e))? {
            let item = item.map_err(|e| io("cannot read", dir, e))?;
            if item.file_type().is_ok_and(|kind| kind.is_dir()) {
                paths.push(item.path());
            }
        }
        paths.sort();
        Ok(paths)
    }
    let mut lines = Vec::new();
    if !build_root.is_dir() {
        return Ok(lines);
    }
    for target in sorted_dirs(build_root)? {
        for config in sorted_dirs(&target)? {
            for entry in sorted_dirs(&config)? {
                let path = entry.join(BUILD_JSON);
                let Ok(bytes) = std::fs::read(&path) else {
                    continue;
                };
                let record: BuildRecord = serde_json::from_slice(&bytes)
                    .map_err(|e| input(path.display().to_string(), e.to_string()))?;
                for (name, ty) in &record.layouts {
                    lines.push(format!(
                        "{}/{}/{} {name} {}",
                        record.target,
                        record.config,
                        record.entry,
                        layout_summary(ty)
                    ));
                }
            }
        }
    }
    Ok(lines)
}

// ---------------------------------------------------------------------------
// Listing
// ---------------------------------------------------------------------------

/// Whether an entry can be driven.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterState {
    /// The roles map to an adapter and at least one seed lists the entry.
    Drivable,
    /// The entry declares no roles.
    NoRoles,
    /// The roles are valid but no seed lists the entry (D-22).
    NoSeeds,
    /// The roles do not make an adapter, with the reason.
    Invalid(String),
    /// The entry is in the adapter drop log (D-13), with its cause.
    Dropped(String),
}

/// One row of `mt vectors adapters`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterRow {
    /// The target id.
    pub target: String,
    /// The entry function.
    pub entry: String,
    /// The number of `arg` tables.
    pub roles: usize,
    /// The number of committed seeds that list the entry.
    pub seeds: usize,
    /// Whether the entry can be driven.
    pub state: AdapterState,
}

impl AdapterRow {
    /// The row text: `<target>/<entry> roles=<n> seeds=<n> <state>`, or
    /// `<target>/<entry> no_roles seeds=<n>`.
    #[must_use]
    pub fn render(&self) -> String {
        let key = format!("{}/{}", self.target, self.entry);
        match &self.state {
            AdapterState::NoRoles => format!("{key} no_roles seeds={}", self.seeds),
            AdapterState::Dropped(cause) => format!("{key} dropped ({cause})"),
            AdapterState::Drivable => {
                format!("{key} roles={} seeds={} drivable", self.roles, self.seeds)
            }
            AdapterState::NoSeeds => {
                format!("{key} roles={} seeds={} no_seeds", self.roles, self.seeds)
            }
            AdapterState::Invalid(reason) => format!(
                "{key} roles={} seeds={} invalid ({})",
                self.roles,
                self.seeds,
                one_line(reason)
            ),
        }
    }

    /// True for an entry whose roles are invalid.
    #[must_use]
    pub const fn is_failure(&self) -> bool {
        matches!(self.state, AdapterState::Invalid(_))
    }
}

/// The seeds of `target` that list `entry`, from the committed index next to the
/// manifest; 0 when the target has no index.
fn seed_count(testbed_dir: &Path, target: &str, entry: &str) -> usize {
    let path = testbed_dir.join("seeds").join(target).join("index.json");
    std::fs::read(&path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<SeedIndex>(&bytes).ok())
        .map_or(0, |index| {
            index
                .seeds
                .iter()
                .filter(|seed| seed.entries.iter().any(|name| name == entry))
                .count()
        })
}

/// Every entry of the manifest with its roles state, by target id and then
/// entry name.
///
/// # Errors
/// [`HarnessError::Input`] when the manifest cannot be read or is invalid.
pub fn list_adapters(manifest_path: &Path) -> Result<Vec<AdapterRow>, HarnessError> {
    let scope = Scope::new(manifest_path.to_path_buf(), PathBuf::from("."));
    let manifest = load_manifest(&scope)?;
    let testbed = scope.testbed_dir();
    let drops = AdapterDropLog::load_from_testbed_dir(&testbed).map_err(|e| {
        input(
            testbed.join("adapter-dropped.toml").display().to_string(),
            e.to_string(),
        )
    })?;
    drops.validate(&manifest).map_err(|e| {
        input(
            testbed.join("adapter-dropped.toml").display().to_string(),
            e.to_string(),
        )
    })?;
    let mut rows = Vec::new();
    for (id, target) in &manifest.target {
        let mut entries: Vec<&Entry> = target.entry.iter().collect();
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        for entry in entries {
            let seeds = seed_count(&testbed, id, &entry.name);
            let state = match Adapter::from_entry(id, "default", entry) {
                Err(AdapterError::NoRoles) if drops.row(id, &entry.name).is_some() => {
                    let cause = drops
                        .row(id, &entry.name)
                        .map_or("", |row| row.cause.as_str());
                    AdapterState::Dropped(cause.to_owned())
                }
                Err(AdapterError::NoRoles) => AdapterState::NoRoles,
                Err(error) => AdapterState::Invalid(error.to_string()),
                Ok(adapter) => match combinations(&adapter) {
                    Err(error) => AdapterState::Invalid(error.to_string()),
                    Ok(_) if seeds == 0 => AdapterState::NoSeeds,
                    Ok(_) => AdapterState::Drivable,
                },
            };
            rows.push(AdapterRow {
                target: id.clone(),
                entry: entry.name.clone(),
                roles: entry.arg.len(),
                seeds,
                state,
            });
        }
    }
    Ok(rows)
}
