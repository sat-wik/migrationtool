//! Pins-driven toolchain check (TOOL-01, TOOL-02).
//!
//! The check has two halves so its rules can be tested without processes:
//! [`observe`] runs every pinned command through the runner and records what
//! happened as [`Observation`]s, and [`evaluate`] is a pure function from pins
//! and observations to a [`CheckReport`]. [`run_check`] is both, after the pins
//! have passed [`Pins::validate_complete`].
//!
//! Honest reporting: a run that timed out, could not be supervised, exited
//! non-zero or printed something unparseable is never `OK`. Timeouts and
//! supervision failures read `not proven`, absent programs `MISSING`, everything
//! else off-pin `MISMATCH`. Only deliberately uninstalled tools (CONTEXT D-10)
//! pass without a measurement.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::path::Path;
use std::time::Duration;

use regex::Regex;
use serde::Serialize;

use crate::pins::{
    LlvmPins, LlvmRule, LlvmSource, LlvmStatus, Pins, PinsError, ToolPinStatus, ToolSpec,
};
use crate::runner::{self, RunRequest, RunnerConfig, RunnerError};
use crate::version::{self, DpkgPackage};

/// Wall-clock limit for one version command.
const VERSION_TIMEOUT: Duration = Duration::from_secs(120);

/// Longest value copied into a table cell or detail.
const MAX_TEXT: usize = 100;

/// Observation keys for the items that are not `[tool.*]` entries.
const OBS_RUSTC_TOOL: &str = "rustc_tool";
const OBS_RUSTC_BITCODE: &str = "rustc_bitcode";
const OBS_RUSTUP_DEFAULT: &str = "rustup_default";
const OBS_RUSTUP_TARGETS_TOOL: &str = "rustup_targets_tool";
const OBS_RUSTUP_TARGETS_BITCODE: &str = "rustup_targets_bitcode";
const OBS_DPKG: &str = "dpkg_query";

/// The `dpkg-query` format: `Package`, `Version` and `db:Status-Status` per line.
/// `dpkg-query` itself expands the `\t` and `\n` escapes.
const DPKG_FORMAT: &str = "-f=${Package}\\t${Version}\\t${db:Status-Status}\\n";

/// Outcome for one item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolStatus {
    /// The live value equals the pin.
    Ok,
    /// The item ran but is off its pin, exited non-zero, or printed something unparseable.
    Mismatch,
    /// The program or package does not exist.
    Missing,
    /// The tool is deliberately absent (CONTEXT D-10); never a failure.
    NotInstalled,
    /// The run timed out, could not be supervised, or was never observed.
    NotProven,
}

impl ToolStatus {
    /// The word printed in the table.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::Mismatch => "MISMATCH",
            Self::Missing => "MISSING",
            Self::NotInstalled => "not_installed",
            Self::NotProven => "not proven",
        }
    }

    /// True for statuses that do not fail a report.
    #[must_use]
    pub fn is_pass(self) -> bool {
        matches!(self, Self::Ok | Self::NotInstalled)
    }

    /// How bad a status is, so combined findings keep the worst one.
    fn severity(self) -> u8 {
        match self {
            Self::Ok | Self::NotInstalled => 0,
            Self::Mismatch => 1,
            Self::NotProven => 2,
            Self::Missing => 3,
        }
    }
}

/// One table row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolRow {
    /// The item key: a tool key, `apt:<package>`, `llvm_packages` or a Rust toolchain row.
    pub key: String,
    /// The pinned value.
    pub expected: String,
    /// The value extracted from the live system, when there was one.
    pub actual: Option<String>,
    /// The verdict.
    pub status: ToolStatus,
    /// The LLVM major the row reports, where one applies.
    pub llvm_major: Option<u32>,
    /// Why the row has its status, built from parsed values and fixed text only.
    pub detail: Option<String>,
}

/// The result of a whole check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckReport {
    /// One row per item, sorted by key.
    pub rows: Vec<ToolRow>,
    /// The LLVM pin the rows were judged against.
    pub llvm: LlvmPins,
}

impl CheckReport {
    /// True when every row is [`ToolStatus::Ok`] or [`ToolStatus::NotInstalled`].
    #[must_use]
    pub fn passed(&self) -> bool {
        self.rows.iter().all(|row| row.status.is_pass())
    }

    /// How many rows fail the report.
    #[must_use]
    pub fn failed_count(&self) -> usize {
        self.rows.iter().filter(|row| !row.status.is_pass()).count()
    }

    /// The `llvm pin:` line: the pinned major, its status and decision, the
    /// bitcode bound, and, while the pin is provisional, when it will be revisited.
    #[must_use]
    pub fn pin_line(&self) -> String {
        let llvm = &self.llvm;
        let status = match llvm.status {
            LlvmStatus::Provisional => "provisional",
            LlvmStatus::Final => "final",
        };
        let mut line = format!(
            "llvm pin: {major} ({status}, decision {decision}); bitcode rustc must report LLVM {major}.x and at most {max}",
            major = llvm.major,
            decision = llvm.decision,
            max = llvm.max_major,
        );
        if llvm.status == LlvmStatus::Provisional {
            line.push_str("; provisional until the Phase 6 R1 verdict");
        }
        line
    }

    /// Header, one line per row (key, expected, actual, LLVM major, status and
    /// detail), the pin line and the result line. Byte-stable for equal reports.
    #[must_use]
    pub fn render_table(&self) -> String {
        let mut cells: Vec<[String; 5]> = vec![[
            "KEY".to_owned(),
            "EXPECTED".to_owned(),
            "ACTUAL".to_owned(),
            "LLVM".to_owned(),
            "STATUS".to_owned(),
        ]];
        for row in &self.rows {
            cells.push([
                row.key.clone(),
                row.expected.clone(),
                row.actual.clone().unwrap_or_else(|| "-".to_owned()),
                row.llvm_major
                    .map_or_else(|| "-".to_owned(), |major| major.to_string()),
                row.status.as_str().to_owned(),
            ]);
        }
        let mut widths = [0usize; 5];
        for line in &cells {
            for (width, cell) in widths.iter_mut().zip(line) {
                *width = (*width).max(cell.chars().count());
            }
        }
        let mut out = String::new();
        for (index, line) in cells.iter().enumerate() {
            let mut text = String::new();
            for (cell, width) in line.iter().zip(widths) {
                text.push_str(&format!("{cell:<width$}  "));
            }
            if let Some(detail) = index
                .checked_sub(1)
                .and_then(|i| self.rows.get(i))
                .and_then(|row| row.detail.as_deref())
            {
                text.push_str(detail);
            }
            out.push_str(text.trim_end());
            out.push('\n');
        }
        out.push_str(&self.pin_line());
        out.push('\n');
        if self.passed() {
            out.push_str("result: OK\n");
        } else {
            out.push_str(&format!("result: FAILED ({} rows)\n", self.failed_count()));
        }
        out
    }
}

/// What one command run produced, before any rule is applied to it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Observation {
    /// The observation key.
    pub key: String,
    /// Captured standard output (lossy UTF-8).
    pub stdout: String,
    /// Captured standard error (lossy UTF-8).
    pub stderr: String,
    /// Exit code; `None` when the child was killed by a signal.
    pub exit_code: Option<i32>,
    /// True when the runner killed the child for exceeding its timeout.
    pub timed_out: bool,
    /// True when the program does not exist.
    pub missing: bool,
    /// A supervision failure (spawn refused, output not drained, i/o error).
    pub error: Option<String>,
    /// A second command observed for this one (the LLVM probe).
    pub probe: Option<Box<Observation>>,
}

impl Observation {
    /// A command that ran and exited 0 with the given output.
    #[must_use]
    pub fn ran(key: &str, stdout: &str, stderr: &str) -> Self {
        Self {
            key: key.to_owned(),
            stdout: stdout.to_owned(),
            stderr: stderr.to_owned(),
            exit_code: Some(0),
            ..Self::default()
        }
    }

    /// A command that was killed for exceeding its timeout.
    #[must_use]
    pub fn timed_out(key: &str) -> Self {
        Self {
            key: key.to_owned(),
            timed_out: true,
            ..Self::default()
        }
    }

    /// A command whose program does not exist.
    #[must_use]
    pub fn missing(key: &str) -> Self {
        Self {
            key: key.to_owned(),
            missing: true,
            ..Self::default()
        }
    }

    /// This observation with `probe` attached.
    #[must_use]
    pub fn with_probe(mut self, probe: Self) -> Self {
        self.probe = Some(Box::new(probe));
        self
    }

    /// Why this run cannot be used as a measurement, if it cannot.
    fn problem(&self) -> Option<(ToolStatus, String)> {
        if self.missing {
            return Some((ToolStatus::Missing, "program not found".to_owned()));
        }
        if self.timed_out {
            return Some((ToolStatus::NotProven, "timed out".to_owned()));
        }
        if let Some(error) = &self.error {
            return Some((
                ToolStatus::NotProven,
                format!("could not be run: {}", clean(error)),
            ));
        }
        match self.exit_code {
            Some(0) => None,
            Some(code) => Some((ToolStatus::Mismatch, format!("exit code {code}"))),
            None => Some((ToolStatus::Mismatch, "ended by a signal".to_owned())),
        }
    }
}

/// Why a check could not be carried out (as opposed to an item being off its pin).
#[derive(Debug, thiserror::Error)]
pub enum CheckError {
    /// The runner failed in a way that points at the request, not the tool.
    #[error("{key}: {source}")]
    Runner {
        /// The observation key.
        key: String,
        /// The runner error.
        #[source]
        source: RunnerError,
    },
    /// The pins are not complete or valid enough to check a container against.
    #[error(transparent)]
    Pins(#[from] PinsError),
    /// The capture file could not be rendered.
    #[error("cannot render the capture file: {0}")]
    Capture(#[from] toml::ser::Error),
}

/// Validate `pins` completely, observe every item through the runner and judge
/// the observations.
///
/// # Errors
/// [`CheckError::Pins`] when the pins fail [`Pins::validate_complete`];
/// [`CheckError::Runner`] when a command request is itself invalid.
pub fn run_check(pins: &Pins, cwd: &Path) -> Result<CheckReport, CheckError> {
    run_check_observed(pins, cwd).map(|(report, _)| report)
}

/// [`run_check`], also returning the observations (for `--capture-outputs`).
///
/// # Errors
/// As [`run_check`].
pub fn run_check_observed(
    pins: &Pins,
    cwd: &Path,
) -> Result<(CheckReport, BTreeMap<String, Observation>), CheckError> {
    pins.validate_complete()?;
    let observations = observe(pins, cwd)?;
    let report = evaluate(pins, &observations);
    Ok((report, observations))
}

/// Run every pinned command through the runner and record what happened.
///
/// Observation keys are the `[tool.*]` keys plus `rustc_tool`, `rustc_bitcode`,
/// `rustup_default`, `rustup_targets_tool`, `rustup_targets_bitcode` and
/// `dpkg_query`. A program that does not exist is recorded as missing, and a
/// timeout or supervision failure is recorded as such; none of them is an error
/// here, so the table can show every item.
///
/// # Errors
/// [`CheckError::Runner`] when the runner rejects a request (empty argv, or an
/// environment key that overrides the fixed base).
pub fn observe(pins: &Pins, cwd: &Path) -> Result<BTreeMap<String, Observation>, CheckError> {
    let cfg = RunnerConfig {
        timeout: VERSION_TIMEOUT,
        ..RunnerConfig::new(
            pins.image.tool_path.clone(),
            pins.image.source_date_epoch.clone(),
        )
    };
    let request = |key: &str, argv: Vec<OsString>, env: BTreeMap<String, String>| RunRequest {
        tool_name: key.to_owned(),
        argv,
        cwd: cwd.to_path_buf(),
        extra_env: env,
        tool_version: None,
    };
    let mut out = BTreeMap::new();

    for (key, spec) in &pins.tool {
        if spec.status == ToolPinStatus::NotInstalled {
            continue;
        }
        let mut argv: Vec<OsString> = vec![spec.bin.clone().into()];
        argv.extend(spec.version_args.iter().map(Into::into));
        let mut observation = run_one(
            &cfg,
            RunRequest {
                tool_version: Some(spec.expect.clone()),
                ..request(key, argv, spec.env.clone())
            },
        )?;
        if let Some(rule) = &spec.llvm
            && rule.source == LlvmSource::Probe
        {
            let probe_key = probe_key(key, rule);
            let argv = rule.argv.iter().map(Into::into).collect();
            let probe = run_one(&cfg, request(&probe_key, argv, BTreeMap::new()))?;
            observation.probe = Some(Box::new(probe));
        }
        out.insert(key.clone(), observation);
    }

    let rust = &pins.rust;
    let rustc = |version: &str| -> Vec<OsString> {
        vec![
            format!(
                "{}/toolchains/{}-{}/bin/rustc",
                rust.rustup_home, version, rust.host
            )
            .into(),
            "-vV".into(),
        ]
    };
    // Pitfall 9: the runner clears the environment, so rustup gets its homes explicitly.
    let rustup_env = || -> BTreeMap<String, String> {
        BTreeMap::from([
            ("CARGO_HOME".to_owned(), rust.cargo_home.clone()),
            ("RUSTUP_HOME".to_owned(), rust.rustup_home.clone()),
        ])
    };
    let rustup = format!("{}/bin/rustup", rust.cargo_home);
    let targets = |version: &str| -> Vec<OsString> {
        vec![
            rustup.clone().into(),
            "target".into(),
            "list".into(),
            "--installed".into(),
            "--toolchain".into(),
            version.into(),
        ]
    };
    let requests = [
        request(OBS_RUSTC_TOOL, rustc(&rust.tool.version), BTreeMap::new()),
        request(
            OBS_RUSTC_BITCODE,
            rustc(&rust.bitcode.version),
            BTreeMap::new(),
        ),
        request(
            OBS_RUSTUP_DEFAULT,
            vec![rustup.clone().into(), "default".into()],
            rustup_env(),
        ),
        request(
            OBS_RUSTUP_TARGETS_TOOL,
            targets(&rust.tool.version),
            rustup_env(),
        ),
        request(
            OBS_RUSTUP_TARGETS_BITCODE,
            targets(&rust.bitcode.version),
            rustup_env(),
        ),
        request(
            OBS_DPKG,
            vec![
                pins.image.dpkg_query.clone().into(),
                "-W".into(),
                DPKG_FORMAT.into(),
            ],
            BTreeMap::new(),
        ),
    ];
    for req in requests {
        let key = req.tool_name.clone();
        out.insert(key, run_one(&cfg, req)?);
    }
    Ok(out)
}

/// The observation key of a tool's LLVM probe: the tool key plus the probe
/// program's file name (`c2rust_ldd`).
fn probe_key(tool_key: &str, rule: &LlvmRule) -> String {
    let program = rule
        .argv
        .first()
        .and_then(|argv0| Path::new(argv0).file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("probe");
    format!("{tool_key}_{program}")
}

/// Run one request and turn the outcome into an [`Observation`].
fn run_one(cfg: &RunnerConfig, request: RunRequest) -> Result<Observation, CheckError> {
    let key = request.tool_name.clone();
    match runner::run(cfg, &request) {
        Ok(output) => Ok(Observation {
            key,
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            exit_code: output.record.exit.code,
            timed_out: output.record.exit.timed_out,
            ..Observation::default()
        }),
        Err(err) if err.is_not_found() => Ok(Observation::missing(&key)),
        Err(
            err @ (RunnerError::Spawn { .. }
            | RunnerError::OutputNotDrained { .. }
            | RunnerError::Io(_)),
        ) => Ok(Observation {
            key,
            error: Some(err.to_string()),
            ..Observation::default()
        }),
        Err(source) => Err(CheckError::Runner { key, source }),
    }
}

#[derive(Serialize)]
struct CaptureEntry<'a> {
    source: &'a str,
    stdout: &'a str,
    stderr: &'a str,
}

/// Render observations as a TOML capture file: one table per observation key
/// (probes included) with `source`, `stdout` and `stderr`, the layout of
/// `tests/fixtures/tool-outputs.toml`, so a CI capture can replace the
/// representative fixtures.
///
/// # Errors
/// [`CheckError::Capture`] if TOML serialization fails.
pub fn capture_toml(
    observations: &BTreeMap<String, Observation>,
    label: &str,
) -> Result<String, CheckError> {
    let mut tables: BTreeMap<&str, CaptureEntry<'_>> = BTreeMap::new();
    for (key, obs) in observations {
        tables.insert(key, capture_entry(obs, label));
        if let Some(probe) = &obs.probe {
            tables.insert(&probe.key, capture_entry(probe, label));
        }
    }
    Ok(toml::to_string(&tables)?)
}

fn capture_entry<'a>(observation: &'a Observation, label: &'a str) -> CaptureEntry<'a> {
    CaptureEntry {
        source: label,
        stdout: &observation.stdout,
        stderr: &observation.stderr,
    }
}

/// Judge `observations` against `pins`. Pure: no process is started.
///
/// One row per item, sorted by key; an item with no observation is `not proven`,
/// never a pass.
#[must_use]
pub fn evaluate(pins: &Pins, observations: &BTreeMap<String, Observation>) -> CheckReport {
    let mut rows: BTreeMap<String, ToolRow> = BTreeMap::new();
    for (key, spec) in &pins.tool {
        insert_row(&mut rows, tool_row(pins, key, spec, observations.get(key)));
    }
    for which in [Toolchain::Tool, Toolchain::Bitcode] {
        let (rustc_key, targets_key) = which.observation_keys();
        insert_row(
            &mut rows,
            rustc_row(pins, which, observations.get(rustc_key)),
        );
        insert_row(
            &mut rows,
            targets_row(pins, which, observations.get(targets_key)),
        );
    }
    insert_row(
        &mut rows,
        default_row(pins, observations.get(OBS_RUSTUP_DEFAULT)),
    );
    let dpkg = dpkg_state(observations.get(OBS_DPKG));
    for (name, pinned) in &pins.apt {
        insert_row(&mut rows, apt_row(name, pinned, &dpkg));
    }
    insert_row(&mut rows, llvm_packages_row(pins, &dpkg));
    CheckReport {
        rows: rows.into_values().collect(),
        llvm: pins.llvm.clone(),
    }
}

/// Add a row; a second row with the same key makes the first one fail rather
/// than silently replacing it.
fn insert_row(rows: &mut BTreeMap<String, ToolRow>, row: ToolRow) {
    match rows.get_mut(&row.key) {
        Some(existing) => {
            if existing.status.is_pass() {
                existing.status = ToolStatus::Mismatch;
            }
            existing.detail = Some("row key is defined more than once".to_owned());
        }
        None => {
            rows.insert(row.key.clone(), row);
        }
    }
}

/// Accumulates findings for one row, keeping the worst status.
struct Verdict {
    status: ToolStatus,
    notes: Vec<String>,
}

impl Verdict {
    fn new() -> Self {
        Self {
            status: ToolStatus::Ok,
            notes: Vec::new(),
        }
    }

    fn fail(&mut self, status: ToolStatus, note: impl Into<String>) {
        if status.severity() > self.status.severity() {
            self.status = status;
        }
        self.notes.push(note.into());
    }

    fn into_row(
        self,
        key: &str,
        expected: String,
        actual: Option<String>,
        llvm_major: Option<u32>,
    ) -> ToolRow {
        ToolRow {
            key: key.to_owned(),
            expected,
            actual,
            status: self.status,
            llvm_major,
            detail: if self.notes.is_empty() {
                None
            } else {
                Some(self.notes.join("; "))
            },
        }
    }
}

/// Single-line, bounded text taken from a tool or a pin.
fn clean(text: &str) -> String {
    let mut out: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(MAX_TEXT + 1)
        .collect();
    if out.chars().count() > MAX_TEXT {
        out = out.chars().take(MAX_TEXT).collect();
        out.push_str("...");
    }
    out
}

/// The first twelve characters of a commit hash.
fn short(commit: &str) -> String {
    commit.chars().take(12).collect()
}

/// The observation, if it exists and ran cleanly.
fn usable(observation: Option<&Observation>) -> Result<&Observation, (ToolStatus, String)> {
    let observation =
        observation.ok_or((ToolStatus::NotProven, "no observation was made".to_owned()))?;
    match observation.problem() {
        Some(problem) => Err(problem),
        None => Ok(observation),
    }
}

/// The first match of named group `group` in stdout, then stderr.
fn extract_either(re: &Regex, group: &str, observation: &Observation) -> Option<String> {
    version::extract(re, group, &observation.stdout)
        .or_else(|| version::extract(re, group, &observation.stderr))
}

fn tool_row(pins: &Pins, key: &str, spec: &ToolSpec, observation: Option<&Observation>) -> ToolRow {
    if spec.status == ToolPinStatus::NotInstalled {
        return ToolRow {
            key: key.to_owned(),
            expected: "-".to_owned(),
            actual: None,
            status: ToolStatus::NotInstalled,
            llvm_major: None,
            detail: spec.reason.as_deref().map(clean),
        };
    }
    let mut verdict = Verdict::new();
    let mut actual = None;
    let mut llvm_major = None;
    match usable(observation) {
        Err((status, note)) => verdict.fail(status, note),
        Ok(observation) => {
            match version::compile_pattern(&spec.version_regex, "version") {
                Err(_) => verdict.fail(ToolStatus::Mismatch, "version_regex is unusable"),
                Ok(re) => match extract_either(&re, "version", observation) {
                    None => verdict.fail(ToolStatus::Mismatch, "no version found in the output"),
                    Some(found) => {
                        if found != spec.expect {
                            verdict.fail(ToolStatus::Mismatch, "version differs from the pin");
                        }
                        actual = Some(clean(&found));
                    }
                },
            }
            if let Some(rule) = &spec.llvm {
                match rule_major(rule, actual.as_deref(), observation) {
                    Ok(major) => {
                        llvm_major = Some(major);
                        if major != pins.llvm.major {
                            verdict.fail(
                                ToolStatus::Mismatch,
                                format!(
                                    "LLVM major {major} differs from the pinned {}",
                                    pins.llvm.major
                                ),
                            );
                        }
                    }
                    Err((status, note)) => verdict.fail(status, note),
                }
            }
        }
    }
    verdict.into_row(key, spec.expect.clone(), actual, llvm_major)
}

/// The LLVM major a tool reports under its pinned rule.
fn rule_major(
    rule: &LlvmRule,
    version_text: Option<&str>,
    observation: &Observation,
) -> Result<u32, (ToolStatus, String)> {
    match rule.source {
        LlvmSource::Version => version_text
            .and_then(version::llvm_major_from_version)
            .ok_or((
                ToolStatus::Mismatch,
                "no version to take the LLVM major from".to_owned(),
            )),
        LlvmSource::Output => regex_major(rule, observation),
        LlvmSource::Probe => {
            let probe = usable(observation.probe.as_deref())
                .map_err(|(status, note)| (status, format!("LLVM probe: {note}")))?;
            regex_major(rule, probe)
        }
    }
}

fn regex_major(rule: &LlvmRule, observation: &Observation) -> Result<u32, (ToolStatus, String)> {
    let unusable = || {
        (
            ToolStatus::Mismatch,
            "LLVM rule has no usable regex".to_owned(),
        )
    };
    let pattern = rule.regex.as_deref().ok_or_else(unusable)?;
    let re = version::compile_pattern(pattern, "major").map_err(|_| unusable())?;
    extract_either(&re, "major", observation)
        .and_then(|major| major.parse().ok())
        .ok_or((
            ToolStatus::Mismatch,
            "no LLVM major found in the output".to_owned(),
        ))
}

/// The two Rust toolchains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Toolchain {
    /// Builds the tool and the `no_std` crates.
    Tool,
    /// Emits LLVM bitcode for KLEE.
    Bitcode,
}

impl Toolchain {
    /// The observation keys of its `rustc -vV` and its installed targets.
    fn observation_keys(self) -> (&'static str, &'static str) {
        match self {
            Self::Tool => (OBS_RUSTC_TOOL, OBS_RUSTUP_TARGETS_TOOL),
            Self::Bitcode => (OBS_RUSTC_BITCODE, OBS_RUSTUP_TARGETS_BITCODE),
        }
    }
}

fn rustc_row(pins: &Pins, which: Toolchain, observation: Option<&Observation>) -> ToolRow {
    let rust = &pins.rust;
    let (key, pin_version, pin_commit) = match which {
        Toolchain::Tool => ("rustc_tool", &rust.tool.version, &rust.tool.commit),
        Toolchain::Bitcode => ("rustc_bitcode", &rust.bitcode.version, &rust.bitcode.commit),
    };
    let mut expected = format!("{pin_version} {}", short(pin_commit));
    if which == Toolchain::Bitcode {
        expected.push_str(&format!(" LLVM {}", rust.bitcode.llvm));
    }
    let mut verdict = Verdict::new();
    let mut actual = None;
    let mut llvm_major = None;
    match usable(observation) {
        Err((status, note)) => verdict.fail(status, note),
        Ok(observation) => match version::parse_rustc_vv(&observation.stdout) {
            None => verdict.fail(ToolStatus::Mismatch, "rustc -vV output was not recognised"),
            Some(vv) => {
                let mut text = format!("{} {}", clean(&vv.release), short(&vv.commit_hash));
                if &vv.release != pin_version {
                    verdict.fail(ToolStatus::Mismatch, "release differs from the pin");
                }
                if &vv.commit_hash != pin_commit {
                    verdict.fail(ToolStatus::Mismatch, "commit-hash differs from the pin");
                }
                llvm_major = vv.llvm.map(|llvm| llvm.major);
                if which == Toolchain::Bitcode {
                    match vv.llvm {
                        None => verdict.fail(ToolStatus::Mismatch, "no LLVM version line"),
                        Some(llvm) => {
                            text.push_str(&format!(" LLVM {llvm}"));
                            if llvm.to_string() != rust.bitcode.llvm {
                                verdict.fail(
                                    ToolStatus::Mismatch,
                                    format!(
                                        "LLVM version {llvm} differs from the pinned {}",
                                        rust.bitcode.llvm
                                    ),
                                );
                            }
                            if llvm.major != pins.llvm.major {
                                verdict.fail(
                                    ToolStatus::Mismatch,
                                    format!(
                                        "LLVM major {} differs from the pinned {}",
                                        llvm.major, pins.llvm.major
                                    ),
                                );
                            }
                            if llvm.major > pins.llvm.max_major {
                                verdict.fail(
                                    ToolStatus::Mismatch,
                                    format!(
                                        "LLVM major {} is above the maximum {}",
                                        llvm.major, pins.llvm.max_major
                                    ),
                                );
                            }
                        }
                    }
                }
                actual = Some(text);
            }
        },
    }
    verdict.into_row(key, expected, actual, llvm_major)
}

fn default_row(pins: &Pins, observation: Option<&Observation>) -> ToolRow {
    let expected = format!("{}-{}", pins.rust.tool.version, pins.rust.host);
    let mut verdict = Verdict::new();
    let mut actual = None;
    match usable(observation) {
        Err((status, note)) => verdict.fail(status, note),
        Ok(observation) => match observation.stdout.split_whitespace().next() {
            None => verdict.fail(ToolStatus::Mismatch, "rustup default printed nothing"),
            Some(token) => {
                if token != expected {
                    verdict.fail(
                        ToolStatus::Mismatch,
                        "the default toolchain is not the tool toolchain (D-12)",
                    );
                }
                actual = Some(clean(token));
            }
        },
    }
    verdict.into_row("rust_default", expected, actual, None)
}

fn targets_row(pins: &Pins, which: Toolchain, observation: Option<&Observation>) -> ToolRow {
    let rust = &pins.rust;
    let mut wanted: BTreeSet<String> = BTreeSet::from([rust.host.clone()]);
    let key = match which {
        Toolchain::Tool => {
            wanted.extend(rust.tool.targets.iter().cloned());
            "rust_targets_tool"
        }
        Toolchain::Bitcode => "rust_targets_bitcode",
    };
    let join = |set: &BTreeSet<String>| set.iter().cloned().collect::<Vec<_>>().join(",");
    let mut verdict = Verdict::new();
    let mut actual = None;
    match usable(observation) {
        Err((status, note)) => verdict.fail(status, note),
        Ok(observation) => {
            let found: BTreeSet<String> = observation
                .stdout
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(clean)
                .collect();
            let missing: Vec<&String> = wanted.difference(&found).collect();
            let extra: Vec<&String> = found.difference(&wanted).collect();
            if !missing.is_empty() {
                verdict.fail(
                    ToolStatus::Mismatch,
                    format!("missing targets: {}", join_refs(&missing)),
                );
            }
            if !extra.is_empty() {
                verdict.fail(
                    ToolStatus::Mismatch,
                    format!("unexpected targets: {}", join_refs(&extra)),
                );
            }
            actual = Some(join(&found));
        }
    }
    verdict.into_row(key, join(&wanted), actual, None)
}

fn join_refs(items: &[&String]) -> String {
    items
        .iter()
        .map(|item| item.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// What `dpkg-query` said, or why it cannot be relied on.
enum Dpkg {
    Rows(Vec<DpkgPackage>),
    Unusable(ToolStatus, String),
}

fn dpkg_state(observation: Option<&Observation>) -> Dpkg {
    match usable(observation) {
        Ok(observation) => Dpkg::Rows(version::parse_dpkg_query(&observation.stdout)),
        Err((status, note)) => Dpkg::Unusable(status, note),
    }
}

fn apt_row(name: &str, pinned: &str, dpkg: &Dpkg) -> ToolRow {
    let key = format!("apt:{name}");
    let mut verdict = Verdict::new();
    let mut actual = None;
    match dpkg {
        Dpkg::Unusable(status, note) => verdict.fail(*status, format!("dpkg-query: {note}")),
        Dpkg::Rows(rows) => {
            // A multiarch package can appear twice; prefer the installed entry.
            let found = rows
                .iter()
                .filter(|package| package.name == name)
                .min_by_key(|package| package.status != "installed");
            match found {
                None => verdict.fail(ToolStatus::Missing, "not in the dpkg database"),
                Some(package) if package.status != "installed" => verdict.fail(
                    ToolStatus::Missing,
                    format!("dpkg status is {}", clean(&package.status)),
                ),
                Some(package) => {
                    if package.version != pinned {
                        verdict.fail(ToolStatus::Mismatch, "version differs from the pin");
                    }
                    actual = Some(clean(&package.version));
                }
            }
        }
    }
    verdict.into_row(&key, pinned.to_owned(), actual, None)
}

fn llvm_packages_row(pins: &Pins, dpkg: &Dpkg) -> ToolRow {
    let pinned = pins.llvm.major;
    let mut verdict = Verdict::new();
    let mut actual = None;
    let mut llvm_major = None;
    match dpkg {
        Dpkg::Unusable(status, note) => verdict.fail(*status, format!("dpkg-query: {note}")),
        Dpkg::Rows(rows) => {
            let majors = version::llvm_family_majors(rows);
            let distinct: BTreeSet<u32> = majors.values().copied().collect();
            if majors.is_empty() {
                verdict.fail(
                    ToolStatus::Mismatch,
                    "no installed llvm/clang-family package found",
                );
            }
            let offenders: Vec<String> = majors
                .iter()
                .filter(|(_, major)| **major != pinned)
                .map(|(name, major)| format!("{} (LLVM {major})", clean(name)))
                .collect();
            if !offenders.is_empty() {
                verdict.fail(
                    ToolStatus::Mismatch,
                    format!("packages of another LLVM major: {}", offenders.join(", ")),
                );
            }
            if !distinct.is_empty() {
                actual = Some(
                    distinct
                        .iter()
                        .map(u32::to_string)
                        .collect::<Vec<_>>()
                        .join(","),
                );
            }
            if distinct.len() == 1 {
                llvm_major = distinct.first().copied();
            }
        }
    }
    verdict.into_row("llvm_packages", pinned.to_string(), actual, llvm_major)
}
