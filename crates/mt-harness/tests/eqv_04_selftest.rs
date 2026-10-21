#![allow(clippy::unwrap_used, clippy::expect_used)]
//! EQV-04: the executor's self-test, run end to end with the host Arm tools.
//!
//! These tests need `/usr/bin/qemu-arm` and `/usr/bin/arm-none-eabi-gcc` with
//! newlib. When one is missing a test prints a line starting `skipping:` and
//! returns; the plan's verify commands require that no such line appears.

use std::path::Path;

use mt_harness::selftest::{SelftestCase, run_case, run_selftest};
use mt_harness::tools::ExecTools;
use mt_toolchain::runner::RunnerConfig;

fn host_tools() -> Option<ExecTools> {
    let needed = [
        "/usr/bin/qemu-arm",
        "/usr/bin/arm-none-eabi-gcc",
        "/usr/bin/arm-none-eabi-ar",
        "/usr/lib/arm-none-eabi/include",
    ];
    if let Some(missing) = needed.iter().find(|path| !Path::new(path).exists()) {
        eprintln!("skipping: {missing} is not installed on this host");
        return None;
    }
    Some(ExecTools::at(
        Path::new("/usr/bin"),
        Path::new("/usr/bin/qemu-arm"),
        Path::new("/usr/bin/llvm-dwarfdump"),
    ))
}

fn runner_config() -> RunnerConfig {
    RunnerConfig::new("/usr/bin:/bin", "1791158400")
}

#[test]
fn eqv_04_selftest_ok_case_runs_end_to_end() {
    let Some(tools) = host_tools() else {
        return;
    };
    let work = tempfile::tempdir().unwrap();
    let report = run_selftest(&tools, &runner_config(), work.path()).unwrap();
    assert!(report.passed, "{}", report.render());

    let row = report
        .rows
        .iter()
        .find(|row| row.text.starts_with("case ok "))
        .expect("a row for the ok case");
    assert_eq!(row.text, "case ok outcome=ok input_len=5");

    let run = report
        .cases
        .iter()
        .find(|run| run.case == SelftestCase::Ok)
        .expect("the ok case was run");
    let argv = &run.output.record.argv;
    let flags = ["-cpu", "cortex-m4", "-R", "0x10000000", "-seed", "0"];
    assert!(
        argv.windows(flags.len()).any(|window| window == flags),
        "qemu-arm argv lacks {flags:?}: {argv:?}"
    );
    let keys: Vec<&str> = run
        .output
        .record
        .env
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(keys, ["LANG", "PATH", "SOURCE_DATE_EPOCH", "TZ"]);
    assert_eq!(
        Path::new(&run.output.record.cwd),
        run.slot_dir.as_path(),
        "the run's working directory is the fixed slot"
    );
    assert!(run.slot_dir.join("req.bin").is_file());
}

#[test]
fn eqv_04_selftest_runs_twice_identically() {
    let Some(tools) = host_tools() else {
        return;
    };
    let work = tempfile::tempdir().unwrap();
    let first = run_case(&tools, &runner_config(), work.path(), SelftestCase::Ok).unwrap();
    let second = run_case(&tools, &runner_config(), work.path(), SelftestCase::Ok).unwrap();
    assert_eq!(
        first.output.record.to_json().unwrap(),
        second.output.record.to_json().unwrap(),
        "the run records differ between two runs"
    );
    assert_eq!(first.output.stdout, second.output.stdout);
    assert!(!first.output.stdout.is_empty());
}
