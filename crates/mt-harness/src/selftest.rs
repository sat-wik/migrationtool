//! The executor's self-test (CONTEXT D-30): the cases the shared runtime
//! template is proven with, run end to end through the runner.
//!
//! The cases are executor fixtures, rendered from constants in this file into
//! the template's cases placeholder. They are not per-target C. The same
//! function runs on a developer host and, in CI, inside the pinned image, where
//! it measures what research could only measure on other tool versions: the
//! QEMU and GCC versions, guard-page faults, the fault handler, the memory
//! budget, run-to-run determinism, the presence of the analysis tools and the
//! pinned GCC's DWARF for a one-byte enum (D-29).

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use mt_capture::manifest::BASELINE_CFLAGS;
use mt_toolchain::runner::{self, RunOutput, RunRequest, RunnerConfig};

use crate::HarnessError;
use crate::budget::Budgets;
use crate::classify::{Outcome, classify};
use crate::driver::{DriverBuild, build_driver};
use crate::exec::{Request, Slot, qemu_options, run_driver, run_driver_with};
use crate::frame::{Frame, parse_frame};
use crate::runtime::{Placeholders, render};
use crate::tools::ExecTools;

/// One self-test case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelftestCase {
    /// Reverses its input into the output buffer, prints through libc and
    /// returns the input length.
    Ok,
    /// Reads one byte past the guarded input buffer.
    OverrunInput,
    /// Writes one byte past the guarded output buffer.
    OverrunOutput,
    /// Writes the byte before the right-aligned input buffer (guard slack).
    SlackWrite,
    /// Calls `abort()`.
    Abort,
    /// Loops forever; ends by the time budget.
    Loop,
    /// Maps memory above the reserved address space.
    BeyondReserve,
    /// Recurses without bound into the stack guard.
    StackOverflow,
}

/// The id of the output-buffer region in the frame.
const OUTPUT_REGION: u32 = 1;

/// Output capacity of every case.
const OUT_CAP: u32 = 16;

/// What the ok case prints through libc: `mt` and a newline.
const OK_PRINTED_BYTES: u32 = 3;

/// The time budget of the loop case, in milliseconds.
const LOOP_TIMEOUT_MS: u64 = 2000;

/// Runs of the ok case the determinism row compares.
const DETERMINISM_RUNS: usize = 100;

impl SelftestCase {
    /// Every case, in report order.
    pub const ALL: [Self; 8] = [
        Self::Ok,
        Self::OverrunInput,
        Self::OverrunOutput,
        Self::SlackWrite,
        Self::Abort,
        Self::Loop,
        Self::BeyondReserve,
        Self::StackOverflow,
    ];

    /// The case name used in rows.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::OverrunInput => "overrun-input",
            Self::OverrunOutput => "overrun-output",
            Self::SlackWrite => "slack-write",
            Self::Abort => "abort",
            Self::Loop => "loop",
            Self::BeyondReserve => "beyond-reserve",
            Self::StackOverflow => "stack-overflow",
        }
    }

    /// The selector the generated `mt_call` switches on.
    #[must_use]
    pub fn selector(self) -> u32 {
        match self {
            Self::Ok => 0,
            Self::OverrunInput => 1,
            Self::OverrunOutput => 2,
            Self::SlackWrite => 3,
            Self::Abort => 4,
            Self::Loop => 5,
            Self::BeyondReserve => 6,
            Self::StackOverflow => 7,
        }
    }

    /// The input bytes of the request.
    #[must_use]
    pub fn input(self) -> Vec<u8> {
        vec![1, 2, 3, 4, 5]
    }

    /// The budgets this case runs under. Only the loop case has its own.
    #[must_use]
    pub fn budgets(self) -> Budgets {
        match self {
            Self::Loop => Budgets {
                timeout_ms: LOOP_TIMEOUT_MS,
                ..Budgets::DEFAULT
            },
            _ => Budgets::DEFAULT,
        }
    }

    pub(crate) fn request(self) -> Request {
        Request {
            fields: vec![
                self.selector().to_le_bytes().to_vec(),
                self.input(),
                OUT_CAP.to_le_bytes().to_vec(),
            ],
        }
    }

    /// The `case` arm of `mt_call`.
    fn fragment(self) -> String {
        let body = match self {
            Self::Ok => "        puts(\"mt\");\n\
                 \x20       for (i = 0; i < in_len && i < out_cap; i++) {\n\
                 \x20           out[i] = in[in_len - 1u - i];\n\
                 \x20       }\n\
                 \x20       return in_len;\n"
                .to_owned(),
            Self::OverrunInput => "        return (u32)*(volatile u8 *)(in + in_len);\n".to_owned(),
            Self::OverrunOutput => {
                "        *(volatile u8 *)(out + out_cap) = 1u;\n        return 0u;\n".to_owned()
            }
            Self::SlackWrite => {
                "        *(volatile u8 *)(in - 1) = 0u;\n        return 0u;\n".to_owned()
            }
            Self::Abort => "        abort();\n".to_owned(),
            Self::Loop => {
                "        for (;;) {\n            __asm__ volatile(\"\");\n        }\n".to_owned()
            }
            Self::BeyondReserve => format!(
                "        {{\n            struct guarded unreachable;\n            mt_guard_alloc(&unreachable, {:#x}u, 16u);\n        }}\n        return 0u;\n",
                Budgets::DEFAULT.reserved_va
            ),
            Self::StackOverflow => "        return mt_recurse(0u);\n".to_owned(),
        };
        format!("    case {}u:\n{body}", self.selector())
    }
}

/// Helpers the case arms call.
const CASE_HELPERS: &str = "\
static u32 mt_recurse(u32 depth) __attribute__((noinline));
static u32 mt_recurse(u32 depth)
{
    volatile u8 pad[128];
    pad[0] = (u8)depth;
    __asm__ volatile(\"\" ::: \"memory\");
    return mt_recurse(depth + 1u) + pad[0];
}

";

/// The C source of `mt_call`, the generated part of the template.
fn cases_source() -> String {
    let mut text = String::from(CASE_HELPERS);
    text.push_str(
        "static u32 mt_call(u32 sel, u8 *in, u32 in_len, u8 *out, u32 out_cap)\n{\n    u32 i;\n    switch (sel) {\n",
    );
    for case in SelftestCase::ALL {
        text.push_str(&case.fragment());
    }
    text.push_str("    default:\n        mt_exit(97);\n    }\n}\n");
    text
}

/// The source of the DWARF probe: an enum of two values, a struct holding the
/// enum, a 3-bit bitfield, a 5-byte array and a union, a typedef, and one global
/// of the typedef.
const PROBE_SOURCE: &str = "\
enum probe_kind { PROBE_A, PROBE_B };
union probe_value { unsigned int word; unsigned char bytes[4]; };
typedef struct probe_record {
    enum probe_kind kind;
    unsigned int flags : 3;
    unsigned char tag[5];
    union probe_value value;
} probe_record_t;
probe_record_t probe_global;
";

/// The result of running one case.
#[derive(Debug, Clone)]
pub struct CaseRun {
    /// The case.
    pub case: SelftestCase,
    /// The fixed working directory of the run; holds `req.bin`.
    pub slot_dir: PathBuf,
    /// The file names in the slot right after the run, sorted. Only `req.bin`
    /// is expected; a core file would show here.
    pub slot_files: Vec<String>,
    /// What the runner returned.
    pub output: RunOutput,
    /// The classification of the run.
    pub outcome: Outcome,
    /// The parsed frame, when stdout held a well-formed one.
    pub frame: Option<Frame>,
}

/// One line of the report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelftestRow {
    /// The row text.
    pub text: String,
    /// True when the row's check held.
    pub ok: bool,
}

/// The report of a self-test run.
#[derive(Debug, Clone)]
pub struct SelftestReport {
    /// The rows, in fixed order.
    pub rows: Vec<SelftestRow>,
    /// Every case run, in report order.
    pub cases: Vec<CaseRun>,
    /// True when every row's check held.
    pub passed: bool,
}

impl SelftestReport {
    /// The rows followed by the result line. Deterministic text.
    #[must_use]
    pub fn render(&self) -> String {
        let mut text = String::new();
        for row in &self.rows {
            text.push_str(&row.text);
            text.push('\n');
        }
        if self.passed {
            text.push_str(&format!("result: passed ({} rows)\n", self.rows.len()));
        } else {
            text.push_str("result: failed\n");
        }
        text
    }
}

/// The outcome of repeating one case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Determinism {
    /// How many runs were made.
    pub runs: usize,
    /// The distinct sha256 values of the runs' stdout; one means identical.
    pub hashes: BTreeSet<String>,
}

/// The pinned GCC's DWARF text for the probe.
#[derive(Debug, Clone)]
pub struct DwarfProbe {
    /// The `llvm-dwarfdump --debug-info` text.
    pub text: String,
    /// Where the text was written, for the CI fixture step.
    pub path: PathBuf,
}

fn io_error(action: &'static str, path: &Path, source: std::io::Error) -> HarnessError {
    HarnessError::Io {
        action,
        path: path.to_path_buf(),
        source,
    }
}

fn selftest_dir(work_dir: &Path) -> PathBuf {
    work_dir.join("vectors-selftest")
}

/// Render the template with every case and link `driver.elf` under the self-test
/// directory.
fn build_selftest_driver(
    tools: &ExecTools,
    cfg: &RunnerConfig,
    work_dir: &Path,
) -> Result<DriverBuild, HarnessError> {
    let source = render(&Placeholders::from_budgets(
        &Budgets::DEFAULT,
        cases_source(),
    ))?;
    build_driver(tools, cfg, &selftest_dir(work_dir).join("build"), &source)
}

fn slot_listing(dir: &Path) -> Result<Vec<String>, HarnessError> {
    let mut names = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|err| io_error("cannot list", dir, err))? {
        let entry = entry.map_err(|err| io_error("cannot list", dir, err))?;
        names.push(entry.file_name().to_string_lossy().into_owned());
    }
    names.sort();
    Ok(names)
}

fn run_built(
    tools: &ExecTools,
    cfg: &RunnerConfig,
    work_dir: &Path,
    build: &DriverBuild,
    case: SelftestCase,
) -> Result<CaseRun, HarnessError> {
    let slot = Slot::create(&selftest_dir(work_dir).join("slot"))?;
    let output = run_driver(
        tools,
        cfg,
        &case.budgets(),
        &build.elf_path,
        &slot,
        &case.request(),
    )?;
    let outcome = classify(&output);
    let frame = parse_frame(&output.stdout).ok();
    Ok(CaseRun {
        case,
        slot_dir: slot.dir().to_path_buf(),
        slot_files: slot_listing(slot.dir())?,
        output,
        outcome,
        frame,
    })
}

/// Build the driver and run one case.
///
/// # Errors
/// [`HarnessError`] when the driver cannot be built or a program cannot run. A
/// bad result is the case's outcome, not an error.
pub fn run_case(
    tools: &ExecTools,
    cfg: &RunnerConfig,
    work_dir: &Path,
    case: SelftestCase,
) -> Result<CaseRun, HarnessError> {
    let build = build_selftest_driver(tools, cfg, work_dir)?;
    run_built(tools, cfg, work_dir, &build, case)
}

/// Run `case` `runs` times against one build and collect the stdout hashes.
///
/// # Errors
/// As [`run_case`].
pub fn determinism(
    tools: &ExecTools,
    cfg: &RunnerConfig,
    work_dir: &Path,
    case: SelftestCase,
    runs: usize,
) -> Result<Determinism, HarnessError> {
    let build = build_selftest_driver(tools, cfg, work_dir)?;
    determinism_of(tools, cfg, work_dir, &build, case, runs)
}

fn determinism_of(
    tools: &ExecTools,
    cfg: &RunnerConfig,
    work_dir: &Path,
    build: &DriverBuild,
    case: SelftestCase,
    runs: usize,
) -> Result<Determinism, HarnessError> {
    let mut hashes = BTreeSet::new();
    for _ in 0..runs {
        let run = run_built(tools, cfg, work_dir, build, case)?;
        hashes.insert(run.output.record.stdout.sha256);
    }
    Ok(Determinism { runs, hashes })
}

/// Run a program through the runner and return its output.
fn run_tool(
    cfg: &RunnerConfig,
    name: &str,
    argv: Vec<OsString>,
    cwd: &Path,
) -> Result<RunOutput, HarnessError> {
    let request = RunRequest {
        tool_name: name.to_owned(),
        argv,
        cwd: cwd.to_path_buf(),
        extra_env: BTreeMap::new(),
        tool_version: None,
    };
    Ok(runner::run(cfg, &request)?)
}

/// A version line from stdout, when the run exited 0: the `LLVM version` line
/// when there is one (llvm-dwarfdump prints a banner first), else the first
/// non-empty line.
fn version_line(output: &RunOutput) -> Option<String> {
    if output.record.exit.code != Some(0) {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    lines
        .iter()
        .find(|line| line.contains("LLVM version"))
        .or_else(|| lines.first())
        .map(|line| (*line).to_owned())
}

/// Compile the probe with the pinned GCC and dump its DWARF with `llvm-dwarfdump`
/// through the runner, writing the text under `<work>/vectors-selftest/dwarf/`.
///
/// # Errors
/// [`HarnessError::Build`] when the compile or the dump fails; [`HarnessError`]
/// for file and runner failures.
pub fn dwarf_probe(
    tools: &ExecTools,
    cfg: &RunnerConfig,
    work_dir: &Path,
) -> Result<DwarfProbe, HarnessError> {
    let dir = selftest_dir(work_dir).join("dwarf");
    std::fs::create_dir_all(&dir).map_err(|err| io_error("cannot create", &dir, err))?;
    let source = dir.join("probe.c");
    std::fs::write(&source, PROBE_SOURCE).map_err(|err| io_error("cannot write", &source, err))?;

    let mut argv: Vec<OsString> = vec![tools.arm_gcc.clone().into_os_string()];
    argv.extend(BASELINE_CFLAGS.iter().map(OsString::from));
    argv.extend(["-g", "-gdwarf-5", "-O0", "-c", "probe.c", "-o", "probe.o"].map(OsString::from));
    let compiled = run_tool(cfg, "arm_gcc", argv, &dir)?;
    if compiled.record.exit.code != Some(0) {
        return Err(HarnessError::Build {
            step: "arm-none-eabi-gcc compile of the DWARF probe".to_owned(),
            detail: String::from_utf8_lossy(&compiled.stderr)
                .chars()
                .take(2000)
                .collect(),
        });
    }
    let dumped = run_tool(
        cfg,
        "llvm_dwarfdump",
        vec![
            tools.llvm_dwarfdump.clone().into_os_string(),
            "--debug-info".into(),
            "probe.o".into(),
        ],
        &dir,
    )?;
    if dumped.record.exit.code != Some(0) || !dumped.record.stdout.complete {
        return Err(HarnessError::Build {
            step: "llvm-dwarfdump of the DWARF probe".to_owned(),
            detail: String::from_utf8_lossy(&dumped.stderr)
                .chars()
                .take(2000)
                .collect(),
        });
    }
    let text = String::from_utf8_lossy(&dumped.stdout).into_owned();
    let path = dir.join("probe.txt");
    std::fs::write(&path, &text).map_err(|err| io_error("cannot write", &path, err))?;
    Ok(DwarfProbe { text, path })
}

/// The `DW_AT_byte_size` of the first enumeration type in a DWARF text.
#[must_use]
pub fn enum_byte_size(text: &str) -> Option<u32> {
    let at = text.find("DW_TAG_enumeration_type")?;
    for line in text[at..].lines().skip(1) {
        if line.contains("DW_TAG_") {
            return None;
        }
        if line.contains("DW_AT_byte_size") {
            let start = line.find("(0x")? + 3;
            let end = line[start..].find(')')? + start;
            return u32::from_str_radix(&line[start..end], 16).ok();
        }
    }
    None
}

/// The most characters of a stderr excerpt.
const STDERR_EXCERPT_CHARS: usize = 500;

/// The start of a stderr stream on one line: whitespace runs become single
/// spaces and the text is cut at 500 characters with `...`. For failure rows
/// only; no verdict ever depends on it.
#[must_use]
pub fn stderr_excerpt(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.is_empty() {
        return "(empty)".to_owned();
    }
    if flat.chars().count() > STDERR_EXCERPT_CHARS {
        let head: String = flat.chars().take(STDERR_EXCERPT_CHARS).collect();
        return format!("{head}...");
    }
    flat
}

/// The end of a stderr stream on one line: whitespace runs become single spaces
/// and, when the text is longer than `max` characters, only the last `max` are
/// kept, after `...`. For a `qemu-arm -strace` log, whose last lines name the
/// system call a run ended on.
#[must_use]
pub fn stderr_tail(stderr: &[u8], max: usize) -> String {
    let text = String::from_utf8_lossy(stderr);
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.is_empty() {
        return "(empty)".to_owned();
    }
    let count = flat.chars().count();
    if count > max {
        let tail: String = flat.chars().skip(count - max).collect();
        return format!("...{tail}");
    }
    flat
}

/// The `-flag value` pair removed from a `qemu-arm` option list.
fn without_option(options: &[OsString], flag: &str) -> Vec<OsString> {
    let mut kept = Vec::new();
    let mut skip = false;
    for option in options {
        if skip {
            skip = false;
        } else if option == flag {
            skip = true;
        } else {
            kept.push(option.clone());
        }
    }
    kept
}

/// The option list with the CPU model replaced.
fn with_cpu(options: &[OsString], cpu: &str) -> Vec<OsString> {
    let mut changed = options.to_vec();
    if let Some(at) = changed.iter().position(|option| option == "-cpu")
        && let Some(model) = changed.get_mut(at + 1)
    {
        *model = OsString::from(cpu);
    }
    changed
}

/// Run the ok case once per `qemu-arm` option variant and describe each result.
/// Used when the ok case did not end ok, so the failure report says what changes
/// the outcome. The production run never uses these options, and no row decides
/// whether the self-test passed.
///
/// # Errors
/// [`HarnessError`] when the driver cannot be built or `qemu-arm` cannot be
/// started.
pub fn diagnose(
    tools: &ExecTools,
    cfg: &RunnerConfig,
    work_dir: &Path,
) -> Result<Vec<SelftestRow>, HarnessError> {
    let build = build_selftest_driver(tools, cfg, work_dir)?;
    diagnose_built(tools, cfg, work_dir, &build)
}

fn diagnose_built(
    tools: &ExecTools,
    cfg: &RunnerConfig,
    work_dir: &Path,
    build: &DriverBuild,
) -> Result<Vec<SelftestRow>, HarnessError> {
    let budgets = Budgets::DEFAULT;
    let base = qemu_options(&budgets);
    let mut guest_errors = vec![OsString::from("-d"), OsString::from("guest_errors,unimp")];
    guest_errors.extend(base.iter().cloned());
    let mut strace = vec![OsString::from("-strace")];
    strace.extend(base.iter().cloned());
    // The last variant shows the end of its log (the last system calls); the
    // others show the start of stderr.
    let variants: [(&str, Vec<OsString>, bool); 7] = [
        ("as-is", base.clone(), false),
        ("no-reserve", without_option(&base, "-R"), false),
        ("no-seed", without_option(&base, "-seed"), false),
        ("cpu-cortex-a15", with_cpu(&base, "cortex-a15"), false),
        ("cpu-max", with_cpu(&base, "max"), false),
        ("guest-error-log", guest_errors, false),
        ("strace", strace, true),
    ];
    let mut rows = Vec::new();
    for (name, options, tail) in variants {
        let slot = Slot::create(&selftest_dir(work_dir).join("slot"))?;
        let output = run_driver_with(
            tools,
            cfg,
            &budgets,
            &options,
            &build.elf_path,
            &slot,
            &SelftestCase::Ok.request(),
        )?;
        let exit = &output.record.exit;
        let show = |value: Option<i32>| value.map_or_else(|| "none".to_owned(), |v| v.to_string());
        rows.push(SelftestRow {
            text: format!(
                "diag {name}: exit={} signal={} stdout_bytes={} stderr={}",
                show(exit.code),
                show(exit.signal),
                output.record.stdout.bytes_total,
                if tail {
                    stderr_tail(&output.stderr, STDERR_EXCERPT_CHARS)
                } else {
                    stderr_excerpt(&output.stderr)
                }
            ),
            ok: true,
        });
    }
    Ok(rows)
}

fn describe(outcome: &Outcome) -> String {
    let mut text = format!("outcome={}", outcome.class_word());
    if let Some(rc) = outcome.rc() {
        text.push_str(&format!(" rc={rc}"));
    }
    if let Some(label) = outcome.has_ub_label() {
        text.push_str(&format!(" label={label}"));
    }
    text
}

/// The row of one case and whether it ended as expected.
fn check_case(run: &CaseRun) -> SelftestRow {
    let case = run.case;
    let described = describe(&run.outcome);
    let (ok, text) = match case {
        SelftestCase::Ok => return check_ok_case(run),
        SelftestCase::OverrunInput | SelftestCase::OverrunOutput | SelftestCase::StackOverflow => (
            run.outcome
                == Outcome::Fault {
                    signal: Some(11),
                    rc: -11,
                },
            format!("case {} {described}", case.name()),
        ),
        SelftestCase::SlackWrite => (
            matches!(run.outcome, Outcome::Fault { signal: None, .. }),
            format!("case {} {described}", case.name()),
        ),
        SelftestCase::Abort => (
            run.outcome == Outcome::Abort,
            format!("case {} {described}", case.name()),
        ),
        SelftestCase::Loop => (
            run.outcome == Outcome::Timeout && run.output.record.exit.timed_out,
            format!("case {} {described}", case.name()),
        ),
        SelftestCase::BeyondReserve => (
            matches!(run.outcome, Outcome::ExecutorError { .. })
                && run.output.record.exit.code == Some(98),
            format!(
                "case {} outcome={} exit={}",
                case.name(),
                run.outcome.class_word(),
                run.output
                    .record
                    .exit
                    .code
                    .map_or_else(|| "none".to_owned(), |code| code.to_string())
            ),
        ),
    };
    SelftestRow {
        text: if ok {
            text
        } else {
            format!("{text} (unexpected for this case)")
        },
        ok,
    }
}

fn check_ok_case(run: &CaseRun) -> SelftestRow {
    let expected_len = run.case.input().len();
    let mut expected_out: Vec<u8> = run.case.input().into_iter().rev().collect();
    expected_out.resize(OUT_CAP as usize, Budgets::DEFAULT.sentinel);
    let good = match (&run.outcome, &run.frame) {
        (Outcome::Ok { rc }, Some(frame)) => {
            usize::try_from(*rc).is_ok_and(|len| len == expected_len)
                && frame.discarded == OK_PRINTED_BYTES
                && frame.slack == 0
                && frame
                    .region(OUTPUT_REGION)
                    .is_some_and(|region| region.bytes == expected_out)
        }
        _ => false,
    };
    if good {
        SelftestRow {
            text: format!("case ok outcome=ok input_len={expected_len}"),
            ok: true,
        }
    } else {
        SelftestRow {
            text: format!(
                "case ok outcome={} (expected ok with input_len={expected_len}): {:?} stderr={}",
                run.outcome.class_word(),
                run.outcome,
                stderr_excerpt(&run.output.stderr)
            ),
            ok: false,
        }
    }
}

/// Run the executor's self-test under `work_dir/vectors-selftest/`.
///
/// # Errors
/// [`HarnessError`] when the driver cannot be built or a program cannot be
/// started. A failing check is a failing row, not an error.
pub fn run_selftest(
    tools: &ExecTools,
    cfg: &RunnerConfig,
    work_dir: &Path,
) -> Result<SelftestReport, HarnessError> {
    let dir = selftest_dir(work_dir);
    std::fs::create_dir_all(&dir).map_err(|err| io_error("cannot create", &dir, err))?;
    let mut rows = Vec::new();

    for (label, path) in [
        ("qemu-arm", &tools.qemu_arm),
        ("arm-none-eabi-gcc", &tools.arm_gcc),
        ("arm-none-eabi-ar", &tools.ar),
        ("arm-none-eabi-nm", &tools.nm),
        ("llvm-dwarfdump", &tools.llvm_dwarfdump),
    ] {
        let output = run_tool(
            cfg,
            label,
            vec![path.clone().into_os_string(), "--version".into()],
            &dir,
        );
        // A tool that cannot even be started is a failing row; the rest of the
        // self-test still runs.
        let row = match output.as_ref().ok().and_then(version_line) {
            Some(line) => SelftestRow {
                text: format!("tool {label} {line}"),
                ok: true,
            },
            None => SelftestRow {
                text: format!("tool {label} is missing or its version could not be read"),
                ok: false,
            },
        };
        rows.push(row);
    }

    let cpus = run_tool(
        cfg,
        "qemu_arm",
        vec![
            tools.qemu_arm.clone().into_os_string(),
            "-cpu".into(),
            "help".into(),
        ],
        &dir,
    )?;
    // `qemu-arm -cpu help` lists the models and then exits 1 (QEMU 8.2) or 0, so
    // only the listing is judged, not the exit code.
    let listed = matches!(cpus.record.exit.code, Some(0 | 1))
        && cpus.record.stdout.complete
        && String::from_utf8_lossy(&cpus.stdout)
            .lines()
            .any(|line| line.split_whitespace().any(|word| word == "cortex-m4"));
    rows.push(SelftestRow {
        text: if listed {
            "cpu cortex-m4 listed".to_owned()
        } else {
            "cpu cortex-m4 not listed by qemu-arm -cpu help".to_owned()
        },
        ok: listed,
    });

    let build = build_selftest_driver(tools, cfg, work_dir)?;
    let mut cases = Vec::new();
    for case in SelftestCase::ALL {
        let run = run_built(tools, cfg, work_dir, &build, case)?;
        rows.push(check_case(&run));
        cases.push(run);
    }

    let overrun_slots_clean = cases
        .iter()
        .filter(|run| {
            matches!(
                run.case,
                SelftestCase::OverrunInput | SelftestCase::OverrunOutput
            )
        })
        .all(|run| run.slot_files == [crate::exec::REQUEST_FILE]);
    rows.push(SelftestRow {
        text: if overrun_slots_clean {
            "no core file after overrun".to_owned()
        } else {
            "a file besides req.bin is left in the slot after an overrun".to_owned()
        },
        ok: overrun_slots_clean,
    });

    let repeated = determinism_of(
        tools,
        cfg,
        work_dir,
        &build,
        SelftestCase::Ok,
        DETERMINISM_RUNS,
    )?;
    let identical = repeated.hashes.len() == 1;
    rows.push(SelftestRow {
        text: format!(
            "determinism {} runs {} hash{}",
            repeated.runs,
            repeated.hashes.len(),
            if identical { "" } else { "es" }
        ),
        ok: identical,
    });

    let probe = dwarf_probe(tools, cfg, work_dir);
    let enum_size = probe.as_ref().ok().and_then(|p| enum_byte_size(&p.text));
    let has_layout_tags = probe.as_ref().is_ok_and(|p| {
        [
            "DW_AT_data_bit_offset",
            "DW_TAG_union_type",
            "DW_TAG_typedef",
            "DW_TAG_array_type",
        ]
        .iter()
        .all(|tag| p.text.contains(tag))
    });
    rows.push(SelftestRow {
        text: match enum_size {
            Some(size) => format!("dwarf enum byte_size {size}"),
            None => "dwarf enum byte_size could not be read".to_owned(),
        },
        ok: enum_size == Some(1) && has_layout_tags,
    });

    let passed = rows.iter().all(|row| row.ok);
    if cases
        .iter()
        .any(|run| run.case == SelftestCase::Ok && !matches!(run.outcome, Outcome::Ok { .. }))
    {
        // The ok case itself failed: say which qemu-arm option changes that.
        rows.extend(diagnose_built(tools, cfg, work_dir, &build)?);
    }
    Ok(SelftestReport {
        rows,
        cases,
        passed,
    })
}
