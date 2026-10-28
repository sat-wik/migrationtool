#![allow(clippy::unwrap_used, clippy::expect_used)]
//! EQV-04: the classification table. Every way a run can end maps to exactly
//! one outcome, decided only from the runner's record and the result frame.
//! The runs here are built from record literals and frame bytes, so no tool is
//! needed.

use mt_harness::classify::{OUTCOME_CLASSES, Outcome, classify};
use mt_harness::frame::SETUP_STATUS_BASE;
use mt_toolchain::runner::{RunOutput, RunRecord};
use serde_json::json;

/// A well-formed result frame with one region, encoded independently of the
/// library so the parser is judged against a separate oracle.
fn frame_bytes(status: u32, ret: u32, slack: u32) -> Vec<u8> {
    let mut out = b"MTRS".to_vec();
    for word in [1u32, status, ret, 3, slack, 1, 1, 4] {
        out.extend_from_slice(&word.to_le_bytes());
    }
    out.extend_from_slice(&[9, 8, 7, 6]);
    out.extend_from_slice(b"MTRE");
    out
}

struct Run {
    code: Option<i32>,
    signal: Option<i32>,
    timed_out: bool,
    tree_kill_sent: bool,
    stdout: Vec<u8>,
    stdout_complete: bool,
    stdout_truncated: bool,
    bytes_total: Option<u64>,
}

impl Run {
    fn exit(code: i32, stdout: Vec<u8>) -> Self {
        Self {
            code: Some(code),
            signal: None,
            timed_out: false,
            tree_kill_sent: false,
            stdout,
            stdout_complete: true,
            stdout_truncated: false,
            bytes_total: None,
        }
    }

    fn output(&self) -> RunOutput {
        let total = self.bytes_total.unwrap_or(self.stdout.len() as u64);
        let stream = |bytes_total: u64, complete: bool, truncated: bool| {
            json!({
                "sha256": "0".repeat(64),
                "bytes_total": bytes_total,
                "bytes_stored": bytes_total,
                "truncated": truncated,
                "complete": complete,
            })
        };
        let record: RunRecord = serde_json::from_value(json!({
            "schema_version": 2,
            "argv": ["qemu-arm"],
            "cwd": "/slot",
            "env": {},
            "tool": {"name": "qemu_arm", "path": "qemu-arm", "version": null},
            "exit": {
                "code": self.code,
                "signal": self.signal,
                "timed_out": self.timed_out,
                "tree_kill_sent": self.tree_kill_sent,
            },
            "stdout": stream(total, self.stdout_complete, self.stdout_truncated),
            "stderr": stream(0, true, false),
        }))
        .unwrap();
        RunOutput {
            record,
            stdout: self.stdout.clone(),
            stderr: Vec::new(),
        }
    }
}

/// The class word, the rc and the has_ub label of a run.
fn judged(run: &Run) -> (&'static str, Option<i32>, Option<&'static str>) {
    let outcome = classify(&run.output());
    (outcome.class_word(), outcome.rc(), outcome.has_ub_label())
}

#[test]
fn eqv_04_classify_maps_each_runner_result_to_one_class() {
    let good = frame_bytes(0, 7, 0);

    // Exit 0 with a well-formed frame is ok; the rc is the raw return value as i32.
    assert_eq!(judged(&Run::exit(0, good.clone())), ("ok", Some(7), None));
    assert_eq!(
        judged(&Run::exit(0, frame_bytes(0, 0xFFFF_FFFF, 0))),
        ("ok", Some(-1), None)
    );

    // Fault exits from the in-guest handler: 128 + signal, rc = -signal (D-25).
    let faults = [
        (139, -11, "crash:SIGSEGV"),
        (135, -7, "crash:SIGBUS"),
        (132, -4, "crash:SIGILL"),
        (136, -8, "crash:SIGFPE"),
    ];
    for (code, rc, label) in faults {
        assert_eq!(
            judged(&Run::exit(code, Vec::new())),
            ("fault", Some(rc), Some(label)),
            "exit {code}"
        );
    }

    // abort() and a failed assert end in SIGABRT: outcome abort (D-27).
    assert_eq!(
        judged(&Run::exit(134, Vec::new())),
        ("abort", Some(-6), Some("crash:SIGABRT"))
    );

    // The runner's timeout is the outcome timeout, never ok (D-02).
    let mut timed_out = Run::exit(0, Vec::new());
    timed_out.code = None;
    timed_out.signal = Some(9);
    timed_out.timed_out = true;
    assert_eq!(judged(&timed_out), ("timeout", Some(-9), Some("timeout")));
    let mut timed_out_with_frame = Run::exit(0, good.clone());
    timed_out_with_frame.timed_out = true;
    assert_eq!(
        judged(&timed_out_with_frame).0,
        "timeout",
        "a timeout is never ok, even with a frame"
    );

    // The driver's own failure codes, an unrelated exit code and 160 are executor errors.
    for code in [95, 96, 97, 98, 99, 1, 160] {
        assert_eq!(
            judged(&Run::exit(code, Vec::new())),
            ("executor_error", None, None),
            "exit {code}"
        );
    }

    // A host signal on qemu-arm itself is an executor error.
    let mut killed = Run::exit(0, Vec::new());
    killed.code = None;
    killed.signal = Some(11);
    assert_eq!(judged(&killed), ("executor_error", None, None));

    // A tree kill, an incomplete or a truncated stdout are executor errors.
    let mut tree_kill = Run::exit(0, good.clone());
    tree_kill.tree_kill_sent = true;
    assert_eq!(judged(&tree_kill).0, "executor_error");
    let mut incomplete = Run::exit(0, good.clone());
    incomplete.stdout_complete = false;
    assert_eq!(judged(&incomplete).0, "executor_error");
    let mut truncated = Run::exit(0, good.clone());
    truncated.stdout_truncated = true;
    assert_eq!(judged(&truncated).0, "executor_error");

    // Exit 0 without a well-formed frame is an executor error.
    assert_eq!(judged(&Run::exit(0, Vec::new())).0, "executor_error");
    let mut bad_magic = good.clone();
    bad_magic[0] = b'X';
    assert_eq!(judged(&Run::exit(0, bad_magic)).0, "executor_error");
    assert_eq!(
        judged(&Run::exit(0, good[..good.len() - 6].to_vec())).0,
        "executor_error"
    );
    let mut trailing = good.clone();
    trailing.extend_from_slice(b"extra");
    assert_eq!(judged(&Run::exit(0, trailing)).0, "executor_error");
    let mut wrong_length = Run::exit(0, good.clone());
    wrong_length.bytes_total = Some(good.len() as u64 + 1);
    assert_eq!(judged(&wrong_length).0, "executor_error");

    // A changed slack byte is a fault with its own label; the rc stays the frame's.
    assert_eq!(
        judged(&Run::exit(0, frame_bytes(0, 5, 1))),
        ("fault", Some(5), Some("oob-write:guard-slack"))
    );
}

#[test]
fn eqv_04_classify_executor_error_never_has_a_vector_class() {
    assert_eq!(
        OUTCOME_CLASSES,
        ["ok", "fault", "abort", "timeout", "setup_failed"]
    );

    let executor_errors = [
        classify(&Run::exit(98, Vec::new()).output()),
        classify(&Run::exit(0, Vec::new()).output()),
        Outcome::ExecutorError {
            reason: "constructed".to_owned(),
        },
    ];
    for outcome in executor_errors {
        assert_eq!(outcome.class_word(), "executor_error");
        assert!(!outcome.is_vector_outcome(), "{outcome:?}");
        assert!(!OUTCOME_CLASSES.contains(&outcome.class_word()));
        assert_eq!(outcome.rc(), None);
    }

    let vector_outcomes = [
        classify(&Run::exit(0, frame_bytes(0, 1, 0)).output()),
        classify(&Run::exit(139, Vec::new()).output()),
        classify(&Run::exit(134, Vec::new()).output()),
        Outcome::SetupFailed { step: 1, rc: 3 },
    ];
    for outcome in vector_outcomes {
        assert!(outcome.is_vector_outcome(), "{outcome:?}");
        assert!(
            OUTCOME_CLASSES.contains(&outcome.class_word()),
            "{outcome:?}"
        );
    }
    assert_eq!(
        Outcome::SetupFailed { step: 1, rc: 3 }.class_word(),
        "setup_failed"
    );
}

#[test]
fn eqv_04_classify_setup_failure_is_its_own_class() {
    // D-12, T-03-26: a setup step that returns something other than its
    // expectation ends the sequence. The frame's status word names the step and
    // the raw return value is the step's return value; the run is a vector of its
    // own class, never labelled as undefined behavior and never an executor error.
    let failed =
        classify(&Run::exit(0, frame_bytes(SETUP_STATUS_BASE, (-2i32) as u32, 0)).output());
    assert_eq!(failed, Outcome::SetupFailed { step: 0, rc: -2 });
    assert_eq!(failed.class_word(), "setup_failed");
    assert_eq!(failed.rc(), Some(-2));
    assert_eq!(failed.has_ub_label(), None, "a setup failure has no label");
    assert!(failed.is_vector_outcome());
    assert!(OUTCOME_CLASSES.contains(&failed.class_word()));

    // The step index is the status word minus the base.
    assert_eq!(
        classify(&Run::exit(0, frame_bytes(SETUP_STATUS_BASE + 3, 9, 0)).output()),
        Outcome::SetupFailed { step: 3, rc: 9 }
    );

    // A status word that is neither 0 nor a step of the grammar's 16 is an
    // executor error, as before.
    for status in [
        1,
        SETUP_STATUS_BASE - 1,
        SETUP_STATUS_BASE + 16,
        0xFFFF_FFFF,
    ] {
        assert_eq!(
            classify(&Run::exit(0, frame_bytes(status, 0, 0)).output()).class_word(),
            "executor_error",
            "status {status:#x}"
        );
    }

    // A changed guard-slack byte is a fault even when a setup step failed.
    assert_eq!(
        classify(&Run::exit(0, frame_bytes(SETUP_STATUS_BASE, 5, 1)).output()),
        Outcome::Fault {
            signal: None,
            rc: 5
        }
    );

    // A fault, abort or timeout during setup is the same fault, abort or timeout.
    assert_eq!(
        judged(&Run::exit(139, Vec::new())),
        ("fault", Some(-11), Some("crash:SIGSEGV"))
    );
}
