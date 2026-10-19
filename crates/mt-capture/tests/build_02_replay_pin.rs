#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Replay is bound to the committed manifest pin and the pinned tools (review
//! WR-02, CONTEXT D-17, D-22).
//!
//! A capture report is read from the work directory, which upstream builds can
//! reach, so replay may only accept it when its target, configuration, source
//! commit, tree sha256 and recorded tool versions equal what the manifest and
//! the pins file say now. Otherwise the pair is `not_proven`, with a reason
//! that names the field, before any cache or tool is touched. Plain
//! `cargo test`: nothing here starts a tool, because every report below is
//! refused before the cache is verified, or has no cache to verify.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use mt_capture::capture::{RunLayout, ToolPaths};
use mt_capture::report::{CaptureReport, CaptureVerdict};
use mt_capture::testbed::{Selection, TestbedOptions, run_replay};
use mt_toolchain::pins::Pins;
use tempfile::TempDir;

const SMALL: &str = include_str!("fixtures/manifest-small.toml");

/// `demo_make`'s pin in the fixture manifest.
const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
const TREE: &str = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";

fn repo_file(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

struct Bed {
    opts: TestbedOptions,
    _tmp: TempDir,
}

/// A bed of the two-target fixture manifest with no drop log, selecting
/// `demo_make` by id (a partial run, so it is not held to the 10-target floor).
fn make_bed() -> Bed {
    let tmp = TempDir::new().unwrap();
    let testbed = tmp.path().join("testbed");
    fs::create_dir_all(&testbed).unwrap();
    let manifest_path = testbed.join("manifest.toml");
    fs::write(&manifest_path, SMALL).unwrap();
    let opts = TestbedOptions {
        manifest_path,
        pins_path: repo_file("container/pins.toml"),
        testbed_dir: testbed,
        work_dir: tmp.path().join("work"),
        out_dir: tmp.path().join("out"),
        selection: Selection::Ids(vec!["demo_make".to_owned()]),
        image: None,
    };
    Bed { opts, _tmp: tmp }
}

/// The tool versions a genuine capture of a Make target records: the pinned
/// version of each reported tool, without CMake (Bear serves Make targets).
fn make_tools() -> BTreeMap<String, String> {
    let pins = Pins::load(&repo_file("container/pins.toml")).unwrap();
    let mut tools = ToolPaths::from_pins(&pins).unwrap().versions;
    tools.remove("cmake");
    assert!(tools.contains_key("arm_gcc"), "{tools:?}");
    assert!(tools.contains_key("bear"), "{tools:?}");
    tools
}

/// A report that matches the manifest pin and the pinned tools in every field
/// replay compares; each test changes the one field it is about.
fn genuine() -> CaptureReport {
    let mut report = CaptureReport::new("demo_make", "default");
    report.verdict = CaptureVerdict::Captured;
    report.source.commit = COMMIT.to_owned();
    report.source.tree_sha256 = TREE.to_owned();
    report.tools = make_tools();
    report
}

fn write_report(bed: &Bed, report: &CaptureReport) {
    let layout = RunLayout::new(&bed.opts.work_dir, "demo_make", "default");
    let path = layout.report_path();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, report.to_json().unwrap()).unwrap();
}

/// The `demo_make/default` row of a replay of the bed.
fn default_row(bed: &Bed) -> String {
    let summary = run_replay(&bed.opts).unwrap();
    assert!(!summary.passed, "{summary:?}");
    summary
        .rows
        .get("demo_make/default")
        .unwrap_or_else(|| panic!("no demo_make/default row: {summary:?}"))
        .clone()
}

fn assert_refused(row: &str, kind: &str, field: &str) {
    assert!(row.starts_with("not_proven: "), "{row}");
    assert!(
        row.contains(&format!("different {kind}")) && row.contains(field),
        "the row must say the report is from a different {kind} and name {field}: {row}"
    );
    assert!(
        !row.contains("cache"),
        "the report was refused before any cache was looked at: {row}"
    );
}

#[test]
fn build_02_replay_refuses_a_capture_from_another_pin() {
    let bed = make_bed();

    // Another commit: the cache of the pinned commit does not exist here, so an
    // unchecked report would be answered with the missing cache instead.
    let mut report = genuine();
    report.source.commit = "fedcba9876543210fedcba9876543210fedcba98".to_owned();
    write_report(&bed, &report);
    assert_refused(&default_row(&bed), "source pin", "source.commit");

    // The right commit and another tree.
    let mut report = genuine();
    report.source.tree_sha256 = "f".repeat(64);
    write_report(&bed, &report);
    assert_refused(&default_row(&bed), "source pin", "source.tree_sha256");

    // A commit that is a path: it is a pin mismatch, never a component.
    let mut report = genuine();
    report.source.commit = "../../../../etc".to_owned();
    write_report(&bed, &report);
    assert_refused(&default_row(&bed), "source pin", "source.commit");
    assert!(
        !bed.opts.work_dir.join("cache").exists(),
        "nothing under the work directory's cache was looked at"
    );
}

#[test]
fn build_02_replay_refuses_a_capture_made_with_other_tools() {
    let bed = make_bed();

    let mut report = genuine();
    report
        .tools
        .insert("arm_gcc".to_owned(), "0.0.Rel0".to_owned());
    write_report(&bed, &report);
    assert_refused(&default_row(&bed), "tool set", "arm_gcc");

    // A tool the pins file records and the report lacks.
    let mut report = genuine();
    report.tools.remove("bear");
    write_report(&bed, &report);
    assert_refused(&default_row(&bed), "tool set", "bear");

    // A tool a Make capture does not use (CMake serves CMake targets).
    let mut report = genuine();
    report.tools.insert("cmake".to_owned(), "3.28.3".to_owned());
    write_report(&bed, &report);
    assert_refused(&default_row(&bed), "tool set", "cmake");
}

#[test]
fn build_02_replay_refuses_a_report_for_another_pair() {
    let bed = make_bed();

    let mut report = genuine();
    report.target = "demo_cmake".to_owned();
    write_report(&bed, &report);
    assert_refused(&default_row(&bed), "pair", "target");

    let mut report = genuine();
    report.config = "no-float".to_owned();
    write_report(&bed, &report);
    assert_refused(&default_row(&bed), "pair", "config");
}

#[test]
fn build_02_replay_checks_the_pin_before_the_cache() {
    // A report that matches the pin, the tools and the pair passes the new
    // check and is then answered by the cache check, as before: this source
    // was never fetched into the work directory.
    let bed = make_bed();
    write_report(&bed, &genuine());
    let row = default_row(&bed);
    assert!(row.starts_with("not_proven: "), "{row}");
    assert!(
        row.contains("the cached tree is missing"),
        "a genuine report must reach the cache check: {row}"
    );
    assert!(!row.contains("different"), "{row}");
}
