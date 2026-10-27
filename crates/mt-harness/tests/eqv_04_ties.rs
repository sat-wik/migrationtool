#![allow(clippy::unwrap_used, clippy::expect_used)]
//! EQV-04: the executor's vocabulary is tied to the schema and the contract
//! text, so a class, a label or a budget cannot be added in one place only.

use std::path::{Path, PathBuf};

use mt_capture::manifest::MAX_OUT_CAP;
use mt_harness::budget::Budgets;
use mt_harness::classify::{OUTCOME_CLASSES as EXECUTOR_CLASSES, Outcome};
use mt_harness::run::budgets_record;
use mt_vectors::vector::OUTCOME_CLASSES as SCHEMA_CLASSES;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// Every outcome the executor can produce, one of each shape.
fn every_outcome() -> Vec<Outcome> {
    let mut outcomes = vec![
        Outcome::Ok { rc: 0 },
        Outcome::Abort,
        Outcome::Timeout,
        Outcome::SetupFailed { step: 1, rc: -1 },
        Outcome::Fault {
            signal: None,
            rc: 0,
        },
        Outcome::ExecutorError {
            reason: "x".to_owned(),
        },
    ];
    for signal in [4, 6, 7, 8, 11] {
        outcomes.push(Outcome::Fault {
            signal: Some(signal),
            rc: -signal,
        });
    }
    outcomes
}

#[test]
fn eqv_04_outcome_classes_match_the_schema() {
    assert_eq!(
        EXECUTOR_CLASSES, SCHEMA_CLASSES,
        "the executor and the schema list the same classes in the same order"
    );
    for outcome in every_outcome() {
        if outcome.is_vector_outcome() {
            assert!(
                SCHEMA_CLASSES.contains(&outcome.class_word()),
                "{outcome:?} writes a class the schema does not know"
            );
        } else {
            assert!(
                !SCHEMA_CLASSES.contains(&outcome.class_word()),
                "executor_error is never a schema class"
            );
        }
    }
}

#[test]
fn eqv_01_contract_names_every_outcome_class_and_label() {
    let text =
        std::fs::read_to_string(repo_root().join("docs/specs/equivalence-contract.md")).unwrap();
    for class in SCHEMA_CLASSES {
        assert!(text.contains(class), "the contract does not name {class}");
    }
    assert!(text.contains("executor_error"));
    let mut labels = Vec::new();
    for outcome in every_outcome() {
        if let Some(label) = outcome.has_ub_label() {
            labels.push(label);
        }
    }
    labels.sort_unstable();
    labels.dedup();
    assert!(labels.len() >= 7, "{labels:?}");
    for label in labels {
        assert!(
            text.contains(label),
            "the contract does not name the label {label}"
        );
    }
}

#[test]
fn eqv_04_budgets_are_in_the_provenance() {
    // Exhaustive destructuring: a new budget field fails to compile here until
    // the provenance carries it.
    let Budgets {
        timeout_ms,
        reserved_va,
        heap_arena,
        sentinel,
        slack_fill,
        max_input,
    } = Budgets::DEFAULT;
    let record = budgets_record(&Budgets::DEFAULT);
    assert_eq!(record.timeout_ms, timeout_ms);
    assert_eq!(record.reserved_va, u64::from(reserved_va));
    assert_eq!(record.heap_arena, u64::from(heap_arena));
    assert_eq!(record.sentinel, sentinel);
    assert_eq!(record.slack_fill, slack_fill);
    assert_eq!(record.max_input, u64::from(max_input));
}

#[test]
fn eqv_04_manifest_output_limit_equals_the_budget() {
    // The manifest validator cannot see the executor's budgets, so the limit it
    // applies to an output capacity is a constant of its own.
    assert_eq!(MAX_OUT_CAP, u64::from(Budgets::DEFAULT.max_input));
}
