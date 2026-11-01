#![allow(clippy::unwrap_used, clippy::expect_used)]
//! EQV-04 (T-03-17, WR-01): replay compares the provenance fields it can
//! re-derive offline, not only the build hashes. Offline: the committed sample,
//! the committed manifest and the committed policies only, no tool runs.

use std::path::{Path, PathBuf};

use mt_capture::manifest::Manifest;
use mt_harness::adapter::Adapter;
use mt_harness::budget::Budgets;
use mt_harness::replay::{check_corpus_location, corpus_dirs_below, provenance_mismatches};
use mt_vectors::corpus::{Corpus, Provenance, read_corpus};
use mt_vectors::sha256_hex;
use mt_vectors::vector::Vector;

fn testbed() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testbed")
}

/// An edit of one provenance field.
type Edit = fn(&mut Provenance);

struct Case {
    provenance: Provenance,
    vector: Vector,
    adapter: Adapter,
    policy_sha256: String,
}

/// The first vector of every committed corpus, with its adapter and the sha256 of
/// its module's committed policy.
fn cases() -> Vec<(PathBuf, Corpus, Vec<Case>)> {
    let manifest = Manifest::load(&testbed().join("manifest.toml")).unwrap();
    let mut found = Vec::new();
    for dir in corpus_dirs_below(&testbed().join("vectors-sample")).unwrap() {
        let corpus = read_corpus(&dir).unwrap();
        let index = &corpus.index;
        let entry = manifest.target[&index.target]
            .entry
            .iter()
            .find(|entry| entry.name == index.entry)
            .unwrap();
        let adapter = Adapter::from_entry(&index.target, &index.config, entry).unwrap();
        let policy_sha256 = sha256_hex(
            &std::fs::read(
                testbed()
                    .join("policy")
                    .join(format!("{}.toml", index.target)),
            )
            .unwrap(),
        );
        let cases = corpus
            .vectors
            .iter()
            .map(|(sha, vector)| Case {
                provenance: index.vectors[sha].clone(),
                vector: vector.clone(),
                adapter: adapter.clone(),
                policy_sha256: policy_sha256.clone(),
            })
            .collect();
        found.push((dir, corpus, cases));
    }
    assert!(!found.is_empty());
    found
}

fn mismatches(case: &Case, provenance: &Provenance, vector: &Vector) -> Vec<&'static str> {
    provenance_mismatches(
        provenance,
        vector,
        &case.adapter,
        &Budgets::DEFAULT,
        Some(&case.policy_sha256),
    )
}

#[test]
fn eqv_04_replay_provenance_accepts_every_committed_vector() {
    let mut checked = 0usize;
    for (dir, _, cases) in cases() {
        for case in cases {
            let fields = mismatches(&case, &case.provenance, &case.vector);
            assert!(fields.is_empty(), "{}: {fields:?}", dir.display());
            checked += 1;
        }
    }
    assert!(checked >= 15, "{checked} vectors checked");
}

#[test]
fn eqv_04_replay_provenance_flags_each_edited_field() {
    let (_, _, cases) = cases().into_iter().next().unwrap();
    let case = &cases[0];

    let edits: [(&str, Edit); 6] = [
        ("seed", |p| p.seed = "0".repeat(64)),
        ("executor_version", |p| {
            p.executor_version = "9.9.9".to_owned()
        }),
        ("contract_version", |p| {
            p.contract_version = "9.9.9".to_owned()
        }),
        ("budgets", |p| p.budgets.timeout_ms += 1),
        ("policy_sha256", |p| p.policy_sha256 = "0".repeat(64)),
        ("record", |p| {
            p.outcome = if p.outcome == "timeout" {
                "ok".to_owned()
            } else {
                "timeout".to_owned()
            };
        }),
    ];
    for (field, edit) in edits {
        let mut provenance = case.provenance.clone();
        edit(&mut provenance);
        assert_eq!(
            mismatches(case, &provenance, &case.vector),
            [field],
            "editing {field}"
        );
    }

    // A scalar edited together with the record that names it.
    let mut provenance = case.provenance.clone();
    provenance.scalars.insert("edited".to_owned(), 1);
    assert_eq!(
        mismatches(case, &provenance, &case.vector),
        ["scalars"],
        "editing scalars"
    );

    // The module's policy file is unreadable: it never matches.
    let fields = provenance_mismatches(
        &case.provenance,
        &case.vector,
        &case.adapter,
        &Budgets::DEFAULT,
        None,
    );
    assert_eq!(fields, ["policy_sha256"]);
}

#[test]
fn eqv_04_replay_provenance_flags_a_vector_edited_with_its_record_intact() {
    // The vector's own stdin is edited; the record still names the old seed.
    let (_, _, cases) = cases().into_iter().next().unwrap();
    let case = &cases[0];
    let mut value = serde_json::to_value(&case.vector).unwrap();
    value["stdin"] = serde_json::Value::String("edited".to_owned());
    let vector: Vector = serde_json::from_value(value).unwrap();
    let fields = mismatches(case, &case.provenance, &vector);
    assert!(fields.contains(&"seed"), "{fields:?}");
}

#[test]
fn eqv_04_replay_location_accepts_the_committed_sample_and_refuses_a_moved_corpus() {
    for (dir, corpus, _) in cases() {
        check_corpus_location(&dir, &corpus.index).unwrap();
    }
    let (dir, corpus, _) = cases().into_iter().next().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let moved = tmp.path().join("elsewhere");
    std::fs::create_dir_all(&moved).unwrap();
    for item in std::fs::read_dir(&dir).unwrap() {
        let item = item.unwrap();
        std::fs::copy(item.path(), moved.join(item.file_name())).unwrap();
    }
    let error = check_corpus_location(&moved, &corpus.index).unwrap_err();
    assert!(error.to_string().contains("not at"), "{error}");
}
