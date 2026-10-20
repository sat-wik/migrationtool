#![allow(clippy::unwrap_used, clippy::expect_used)]
//! BUILD-01 capture verdict rule (review warning WR-11, plan 02-20, threat
//! T-02-68): `captured` is the one word that lets replay start, so it needs no
//! cause, both gates passed and at least one object. These tests are pure; they
//! run without the Arm tools.

use mt_capture::capture::settle_verdict;
use mt_capture::report::{CaptureReport, CaptureVerdict, CapturedObject};

fn report(allow_listed: bool, arch_ok: bool, objects: usize) -> CaptureReport {
    let mut report = CaptureReport::new("demo", "default");
    report.gates.compiler_allow_listed = allow_listed;
    report.gates.arch_ok = arch_ok;
    for index in 0..objects {
        report.objects.insert(
            format!("src/o{index}.o"),
            CapturedObject {
                sha256: "0".repeat(64),
                arch_ok: true,
                opt_goal: None,
                code_digest: None,
            },
        );
    }
    report
}

#[test]
fn build_01_captured_verdict_needs_passed_gates_and_objects() {
    // (a) Nothing evaluated: a gate that was not evaluated reads as failed.
    let mut a = CaptureReport::new("demo", "default");
    settle_verdict(&mut a, false, false);
    assert_eq!(a.verdict, CaptureVerdict::CaptureFailed);
    assert_eq!(a.causes, ["gates_not_passed"]);
    assert_eq!(a.cause_details.len(), 1);
    let detail = &a.cause_details[0];
    assert!(detail.starts_with("gates_not_passed: "), "{detail}");
    for part in ["compiler_allow_listed", "arch_ok", "objects"] {
        assert!(detail.contains(part), "{detail}");
    }

    // (b) Only the compiler gate passed.
    let mut b = report(true, false, 1);
    settle_verdict(&mut b, false, false);
    assert_eq!(b.verdict, CaptureVerdict::CaptureFailed);
    assert_eq!(b.causes, ["gates_not_passed"]);

    // (c) Both gates passed, no object.
    let mut c = report(true, true, 0);
    settle_verdict(&mut c, false, false);
    assert_eq!(c.verdict, CaptureVerdict::CaptureFailed);
    assert_eq!(c.causes, ["gates_not_passed"]);

    // (d) Both gates passed and an object: captured, no cause.
    let mut d = report(true, true, 1);
    settle_verdict(&mut d, false, false);
    assert_eq!(d.verdict, CaptureVerdict::Captured);
    assert!(d.causes.is_empty() && d.cause_details.is_empty());

    // (e) Not proven wins, and adds no cause of its own.
    let mut e = report(true, true, 1);
    settle_verdict(&mut e, true, false);
    assert_eq!(e.verdict, CaptureVerdict::NotProven);
    assert!(e.causes.is_empty());

    // (f) A failed build.
    let mut f = report(true, true, 1);
    settle_verdict(&mut f, false, true);
    assert_eq!(f.verdict, CaptureVerdict::BuildFailed);
    assert!(f.causes.is_empty());

    // (g) An existing cause explains the failure, so gates_not_passed is not added.
    let mut g = report(false, false, 0);
    g.causes.push("arch_gate".to_owned());
    settle_verdict(&mut g, false, false);
    assert_eq!(g.verdict, CaptureVerdict::CaptureFailed);
    assert_eq!(g.causes, ["arch_gate"]);
}
