#![allow(clippy::unwrap_used, clippy::expect_used)]
//! EQV-04: the listings `mt vectors build` and `mt vectors run` print. The testbed
//! workflow publishes each listing as one annotation, and GitHub keeps at most 50
//! annotations per job, so the rows of entries without roles are counted in one
//! line instead of being repeated for every configuration. Pure tests.

use mt_harness::run::{
    BuildRow, BuildState, EntryRunSummary, RunRow, RunState, render_build_listing,
    render_run_listing,
};

fn build_row(entry: &str, state: BuildState) -> BuildRow {
    BuildRow {
        target: "t".to_owned(),
        config: "default".to_owned(),
        entry: entry.to_owned(),
        state,
    }
}

fn run_row(entry: &str, state: RunState) -> RunRow {
    RunRow {
        target: "t".to_owned(),
        config: "default".to_owned(),
        entry: entry.to_owned(),
        state,
    }
}

#[test]
fn eqv_04_build_listing_counts_no_roles_rows() {
    let rows = vec![
        build_row("a", BuildState::NoRoles),
        build_row(
            "b",
            BuildState::Built {
                driver_sha256: "ab".repeat(32),
            },
        ),
        build_row("c", BuildState::NoRoles),
        build_row("d", BuildState::LinkUnresolved(vec!["x".to_owned()])),
    ];
    assert_eq!(
        render_build_listing(&rows),
        [
            format!("t/default/b built driver_sha256={}", "ab".repeat(32)),
            "t/default/d link_unresolved (x)".to_owned(),
            "no_roles: 2".to_owned(),
        ],
        "every row but the no_roles rows, then their count"
    );
    // No tally line when every entry has roles.
    let rows = vec![build_row("b", BuildState::LinkUnresolved(vec![]))];
    assert_eq!(
        render_build_listing(&rows),
        ["t/default/b link_unresolved ()".to_owned()]
    );
}

#[test]
fn eqv_04_run_listing_counts_no_roles_rows() {
    let summary = EntryRunSummary {
        vectors: 3,
        ok: 3,
        fault: 0,
        abort: 0,
        timeout: 0,
        setup_failed: 0,
        skipped: 0,
        skipped_notes: Vec::new(),
        twice_identical: true,
    };
    let rows = vec![
        run_row("a", RunState::NoRoles),
        run_row("b", RunState::Ran(summary.clone())),
        run_row("c", RunState::Failed("boom".to_owned())),
        run_row("d", RunState::NoRoles),
    ];
    assert_eq!(
        render_run_listing(&rows),
        [
            format!("t/default/b {}", summary.render()),
            "t/default/c failed (boom)".to_owned(),
            "no_roles: 2".to_owned(),
        ]
    );
}
