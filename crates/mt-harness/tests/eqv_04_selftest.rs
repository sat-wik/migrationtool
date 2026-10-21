#![allow(clippy::unwrap_used, clippy::expect_used)]
//! EQV-04: the executor's self-test, run end to end with the host Arm tools.
//!
//! These tests need `/usr/bin/qemu-arm` and `/usr/bin/arm-none-eabi-gcc` with
//! newlib. When one is missing a test prints a line starting `skipping:` and
//! returns; the plan's verify commands require that no such line appears.

use std::path::Path;

use mt_capture::manifest::Manifest;
use mt_harness::budget::Budgets;
use mt_harness::runtime::{
    PLACEHOLDER_NAMES, Placeholders, RUNTIME_TEMPLATE, render, render_template,
};
use mt_harness::selftest::{
    CaseRun, SelftestCase, determinism, dwarf_probe, run_case, run_selftest,
};
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
    let keys: Vec<&str> = run.output.record.env.keys().map(String::as_str).collect();
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

fn repo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// Run one case in a fresh work directory.
fn run(case: SelftestCase) -> Option<CaseRun> {
    let tools = host_tools()?;
    let work = tempfile::tempdir().unwrap();
    let run = run_case(&tools, &runner_config(), work.path(), case).unwrap();
    // The slot is inside the temporary directory, which is gone after this
    // function; copy what the callers inspect before returning.
    let listing: Vec<String> = std::fs::read_dir(&run.slot_dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        listing,
        ["req.bin"],
        "{}: the slot holds only req.bin (no core file)",
        case.name()
    );
    Some(run)
}

#[test]
fn eqv_04_overrun_is_a_fault() {
    for case in [SelftestCase::OverrunInput, SelftestCase::OverrunOutput] {
        let Some(run) = run(case) else {
            return;
        };
        assert_eq!(run.outcome.class_word(), "fault", "{}", case.name());
        assert_eq!(run.outcome.rc(), Some(-11), "{}", case.name());
        assert_eq!(
            run.outcome.has_ub_label(),
            Some("crash:SIGSEGV"),
            "{}",
            case.name()
        );
        assert_eq!(run.output.record.exit.code, Some(139));
        assert_eq!(run.output.record.exit.signal, None);
    }
}

#[test]
fn eqv_04_slack_write_is_detected() {
    let Some(run) = run(SelftestCase::SlackWrite) else {
        return;
    };
    assert_eq!(run.outcome.class_word(), "fault");
    assert_eq!(run.outcome.has_ub_label(), Some("oob-write:guard-slack"));
}

#[test]
fn eqv_04_abort_is_outcome_abort() {
    let Some(run) = run(SelftestCase::Abort) else {
        return;
    };
    assert_eq!(run.outcome.class_word(), "abort");
    assert_eq!(run.outcome.rc(), Some(-6));
    assert_eq!(run.outcome.has_ub_label(), Some("crash:SIGABRT"));
}

#[test]
fn eqv_04_timeout_is_recorded() {
    assert_eq!(SelftestCase::Loop.budgets().timeout_ms, 2000);
    let Some(run) = run(SelftestCase::Loop) else {
        return;
    };
    assert!(run.output.record.exit.timed_out);
    assert_eq!(run.outcome.class_word(), "timeout");
    assert_eq!(run.outcome.rc(), Some(-9));
    assert_eq!(run.outcome.has_ub_label(), Some("timeout"));
    assert_ne!(run.outcome.class_word(), "ok", "a timeout is never a pass");
}

#[test]
fn eqv_04_mapping_beyond_reserve_is_executor_error() {
    let Some(run) = run(SelftestCase::BeyondReserve) else {
        return;
    };
    assert_eq!(run.output.record.exit.code, Some(98));
    assert_eq!(run.outcome.class_word(), "executor_error");
    assert!(!run.outcome.is_vector_outcome());
}

#[test]
fn eqv_04_stack_overflow_is_a_fault() {
    let Some(run) = run(SelftestCase::StackOverflow) else {
        return;
    };
    assert_eq!(run.outcome.class_word(), "fault");
    assert_eq!(run.outcome.has_ub_label(), Some("crash:SIGSEGV"));
}

#[test]
fn eqv_04_selftest_runs_are_byte_identical() {
    let Some(tools) = host_tools() else {
        return;
    };
    let work = tempfile::tempdir().unwrap();
    let result = determinism(&tools, &runner_config(), work.path(), SelftestCase::Ok, 100).unwrap();
    assert_eq!(result.runs, 100);
    assert_eq!(
        result.hashes.len(),
        1,
        "100 runs must give one stdout sha256: {:?}",
        result.hashes
    );
}

#[test]
fn eqv_04_dwarf_probe_records_a_one_byte_enum() {
    let Some(tools) = host_tools() else {
        return;
    };
    if !tools.llvm_dwarfdump.exists() {
        eprintln!(
            "skipping: {} is not installed",
            tools.llvm_dwarfdump.display()
        );
        return;
    }
    let work = tempfile::tempdir().unwrap();
    let probe = dwarf_probe(&tools, &runner_config(), work.path()).unwrap();
    let text = probe.text;
    let enum_at = text
        .find("DW_TAG_enumeration_type")
        .expect("an enumeration type in the dump");
    let after_enum = &text[enum_at..];
    let size_line = after_enum
        .lines()
        .find(|line| line.contains("DW_AT_byte_size"))
        .expect("the enum has a byte size");
    assert!(
        size_line.contains("(0x01)"),
        "GCC lays the enum out in 1 byte: {size_line}"
    );
    for tag in [
        "DW_AT_data_bit_offset",
        "DW_TAG_union_type",
        "DW_TAG_typedef",
        "DW_TAG_array_type",
    ] {
        assert!(text.contains(tag), "the dump lacks {tag}");
    }
    assert!(probe.path.is_file());
}

#[test]
fn eqv_04_runtime_template_is_target_neutral() {
    let manifest = Manifest::load(&repo_root().join("testbed/manifest.toml")).unwrap();
    let mut names = Vec::new();
    for (id, target) in &manifest.target {
        names.push(id.clone());
        for entry in &target.entry {
            names.push(entry.name.clone());
            names.push(entry.header.clone());
        }
    }
    assert!(
        names.len() > 16,
        "the manifest names its targets and entries"
    );
    for name in &names {
        assert!(
            !RUNTIME_TEMPLATE.contains(name.as_str()),
            "the template names {name}"
        );
    }
    // Only the closed placeholder set appears.
    let mut rest = RUNTIME_TEMPLATE;
    let mut found = 0;
    while let Some(start) = rest.find("@MT_") {
        let after = &rest[start + 1..];
        let end = after.find('@').expect("a placeholder is closed");
        assert!(
            PLACEHOLDER_NAMES.contains(&&after[..end]),
            "unknown placeholder @{}@",
            &after[..end]
        );
        found += 1;
        rest = &after[end + 1..];
    }
    assert_eq!(found, PLACEHOLDER_NAMES.len());
}

#[test]
fn eqv_04_runtime_render_replaces_each_placeholder_once() {
    let values = Placeholders::from_budgets(&Budgets::DEFAULT, "/* cases */".to_owned());
    let text = render(&values).unwrap();
    assert!(!text.contains("@MT_"));
    assert!(text.contains("#define MT_RESERVED_VA 0x10000000u"));
    assert!(text.contains("/* cases */"));
}

#[test]
fn eqv_04_runtime_render_rejects_unknown_and_leftover_tokens() {
    let values = Placeholders::from_budgets(&Budgets::DEFAULT, String::new());
    // An unknown token in the template.
    let unknown = format!("{RUNTIME_TEMPLATE}\n@MT_EXTRA@\n");
    assert!(render_template(&unknown, &values).is_err());
    // A placeholder that is missing, or appears twice.
    let missing = RUNTIME_TEMPLATE.replacen("@MT_SENTINEL@", "165", 1);
    assert!(render_template(&missing, &values).is_err());
    let twice = format!("{RUNTIME_TEMPLATE}\n@MT_SENTINEL@\n");
    assert!(render_template(&twice, &values).is_err());
    // A generated fragment that smuggles a token in.
    let smuggled = Placeholders::from_budgets(&Budgets::DEFAULT, "@MT_HEAP_ARENA@".to_owned());
    assert!(render(&smuggled).is_err());
}
