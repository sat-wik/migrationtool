//! The executor's self-test (CONTEXT D-30): the cases the shared runtime
//! template is proven with, run end to end through the runner.
//!
//! The cases are executor fixtures, rendered from constants in this file into
//! the template's cases placeholder. They are not per-target C. The same
//! function runs on a developer host and, in CI, inside the pinned image, where
//! it measures what research could only measure on other tool versions.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use mt_toolchain::runner::{self, RunOutput, RunRequest, RunnerConfig};

use crate::HarnessError;
use crate::budget::Budgets;
use crate::classify::{Outcome, classify};
use crate::driver::{DriverBuild, build_driver};
use crate::exec::{Request, Slot, run_driver};
use crate::frame::{Frame, parse_frame};
use crate::runtime::{Placeholders, render};
use crate::tools::ExecTools;

/// One self-test case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelftestCase {
    /// Reverses its input into the output buffer, prints through libc and
    /// returns the input length.
    Ok,
}

/// The id of the output-buffer region in the frame.
const OUTPUT_REGION: u32 = 1;

/// Output capacity of every case.
const OUT_CAP: u32 = 16;

/// What the ok case prints through libc: `mt` and a newline.
const OK_PRINTED_BYTES: u32 = 3;

impl SelftestCase {
    /// Every case, in report order.
    pub const ALL: [Self; 1] = [Self::Ok];

    /// The case name used in rows.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Ok => "ok",
        }
    }

    /// The selector the generated `mt_call` switches on.
    #[must_use]
    pub fn selector(self) -> u32 {
        match self {
            Self::Ok => 0,
        }
    }

    /// The input bytes of the request.
    #[must_use]
    pub fn input(self) -> Vec<u8> {
        match self {
            Self::Ok => vec![1, 2, 3, 4, 5],
        }
    }

    /// The budgets this case runs under.
    #[must_use]
    pub fn budgets(self) -> Budgets {
        match self {
            Self::Ok => Budgets::DEFAULT,
        }
    }

    fn request(self) -> Request {
        Request {
            fields: vec![
                self.selector().to_le_bytes().to_vec(),
                self.input(),
                OUT_CAP.to_le_bytes().to_vec(),
            ],
        }
    }

    fn fragment(self) -> &'static str {
        match self {
            Self::Ok => {
                "    case 0u:\n\
                 \x20       puts(\"mt\");\n\
                 \x20       for (i = 0; i < in_len && i < out_cap; i++) {\n\
                 \x20           out[i] = in[in_len - 1u - i];\n\
                 \x20       }\n\
                 \x20       return in_len;\n"
            }
        }
    }
}

/// The C source of `mt_call`, the generated part of the template.
fn cases_source() -> String {
    let mut text = String::from(
        "static u32 mt_call(u32 sel, u8 *in, u32 in_len, u8 *out, u32 out_cap)\n{\n    u32 i;\n    switch (sel) {\n",
    );
    for case in SelftestCase::ALL {
        text.push_str(case.fragment());
    }
    text.push_str("    default:\n        mt_exit(97);\n    }\n}\n");
    text
}

/// The result of running one case.
#[derive(Debug, Clone)]
pub struct CaseRun {
    /// The case.
    pub case: SelftestCase,
    /// The fixed working directory of the run; holds `req.bin`.
    pub slot_dir: PathBuf,
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

/// Run a program through the runner and return its output.
fn run_tool(
    cfg: &RunnerConfig,
    name: &str,
    argv: Vec<std::ffi::OsString>,
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

/// The first non-empty line of stdout, when the run exited 0.
fn first_line(output: &RunOutput) -> Option<String> {
    if output.record.exit.code != Some(0) {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_owned)
}

fn check_ok_case(run: &CaseRun) -> (bool, String) {
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
        (true, format!("case ok outcome=ok input_len={expected_len}"))
    } else {
        (
            false,
            format!(
                "case ok outcome={} (expected ok with input_len={expected_len}): {:?}",
                run.outcome.class_word(),
                run.outcome
            ),
        )
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
    std::fs::create_dir_all(&dir).map_err(|source| HarnessError::Io {
        action: "cannot create",
        path: dir.clone(),
        source,
    })?;
    let mut rows = Vec::new();

    let version = run_tool(
        cfg,
        "qemu_arm",
        vec![tools.qemu_arm.clone().into_os_string(), "--version".into()],
        &dir,
    )?;
    match first_line(&version) {
        Some(line) => rows.push(SelftestRow {
            text: format!("tool qemu-arm {line}"),
            ok: true,
        }),
        None => rows.push(SelftestRow {
            text: "tool qemu-arm version could not be read".to_owned(),
            ok: false,
        }),
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
        let (ok, text) = match case {
            SelftestCase::Ok => check_ok_case(&run),
        };
        rows.push(SelftestRow { text, ok });
        cases.push(run);
    }

    let passed = rows.iter().all(|row| row.ok);
    Ok(SelftestReport {
        rows,
        cases,
        passed,
    })
}
