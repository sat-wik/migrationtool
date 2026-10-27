#![allow(clippy::unwrap_used, clippy::expect_used)]
//! EQV-02, D-19, D-21: the sha256-named corpus and its provenance index.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use mt_vectors::corpus::{
    CORPUS_INDEX_SCHEMA_VERSION, HasUbSource, Provenance, ProvenanceBudgets, read_corpus,
    write_corpus,
};
use mt_vectors::vector::{HasUbLabel, Vector};
use serde_json::json;

const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HASH_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn vector(rc: i32, stderr: &str) -> Vector {
    let value = json!({
        "argv": ["corejson/default/JSON_Validate", "3"],
        "stdin": "abc",
        "env": {},
        "stdout": {"pattern": "", "is_regex": false},
        "stderr": {"pattern": stderr, "is_regex": false},
        "rc": rc,
        "lib_state_in": {},
        "lib_state_out": {"st": {"type": "int", "unobserved": "padding"}}
    });
    Vector::from_json_bytes(&serde_json::to_vec(&value).unwrap()).unwrap()
}

fn provenance(outcome: &str, source: HasUbSource) -> Provenance {
    Provenance {
        seed: HASH_A.to_owned(),
        scalars: BTreeMap::from([("len".to_owned(), 3)]),
        objects: BTreeMap::from([("build/cJSON.o".to_owned(), HASH_B.to_owned())]),
        archive: HASH_A.to_owned(),
        driver_source: HASH_B.to_owned(),
        driver_elf: HASH_A.to_owned(),
        image: "ghcr.io/example/toolchain@sha256:0000".to_owned(),
        executor_version: "0.1.0".to_owned(),
        contract_version: "0.1.0".to_owned(),
        policy_sha256: HASH_B.to_owned(),
        budgets: ProvenanceBudgets {
            timeout_ms: 10_000,
            reserved_va: 0x1000_0000,
            heap_arena: 1 << 20,
            sentinel: 0xA5,
            slack_fill: 0x5A,
            max_input: 65_536,
        },
        outcome: outcome.to_owned(),
        has_ub_source: source,
        unobserved: 1,
        discarded_output_bytes: 0,
        replay: "identical".to_owned(),
    }
}

/// Three distinct vectors: one clean, one crash (labelled), one timeout.
fn three() -> Vec<(Vector, Provenance)> {
    let ok = vector(0, "outcome=ok");
    let mut crash = vector(-11, "outcome=fault");
    crash.has_ub = Some(HasUbLabel::new("crash:SIGSEGV").unwrap());
    let mut timeout = vector(-9, "outcome=timeout");
    timeout.has_ub = Some(HasUbLabel::new("timeout").unwrap());
    vec![
        (ok, provenance("ok", HasUbSource::Unchecked)),
        (crash, provenance("fault", HasUbSource::ObservedCrash)),
        (timeout, provenance("timeout", HasUbSource::Timeout)),
    ]
}

fn tree(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    fs::read_dir(dir)
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            (
                e.file_name().to_string_lossy().into_owned(),
                fs::read(e.path()).unwrap(),
            )
        })
        .collect()
}

fn written() -> (tempfile::TempDir, std::path::PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("corpus");
    write_corpus(&dir, &three()).unwrap();
    (tmp, dir)
}

fn sha_files(dir: &Path) -> Vec<String> {
    tree(dir)
        .into_keys()
        .filter(|name| name != "index.json")
        .collect()
}

#[test]
fn eqv_02_corpus_round_trips_with_provenance() {
    let (_tmp, dir) = written();
    let files = tree(&dir);
    assert_eq!(files.len(), 4, "three vectors and the index");
    assert!(files.contains_key("index.json"));

    let set = three();
    for (vector, _) in &set {
        let name = format!("{}.json", vector.sha256());
        assert_eq!(
            files.get(&name).map(Vec::as_slice),
            Some(vector.to_canonical_json().unwrap().as_slice()),
            "{name} holds the canonical bytes"
        );
    }

    let corpus = read_corpus(&dir).unwrap();
    assert_eq!(corpus.index.schema_version, CORPUS_INDEX_SCHEMA_VERSION);
    assert_eq!(corpus.index.target, "corejson");
    assert_eq!(corpus.index.config, "default");
    assert_eq!(corpus.index.entry, "JSON_Validate");
    assert_eq!(corpus.vectors.len(), 3);
    for (vector, prov) in &set {
        let sha = vector.sha256();
        assert_eq!(corpus.vectors.get(&sha), Some(vector));
        assert_eq!(corpus.index.vectors.get(&sha), Some(prov));
    }

    // The index is deterministic: no clock, sorted keys, a trailing newline.
    let other_tmp = tempfile::tempdir().unwrap();
    let other = other_tmp.path().join("again");
    let mut reversed = three();
    reversed.reverse();
    write_corpus(&other, &reversed).unwrap();
    assert_eq!(tree(&dir), tree(&other), "byte-identical trees");
    assert!(files["index.json"].ends_with(b"\n"));
}

#[test]
fn eqv_02_corpus_refuses_a_non_empty_directory() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("stray.txt"), b"x").unwrap();
    let err = write_corpus(tmp.path(), &three()).unwrap_err();
    assert!(err.to_string().contains("not empty"), "{err}");
    assert_eq!(tree(tmp.path()).len(), 1, "nothing was written");

    // An existing empty directory is accepted.
    let empty = tempfile::tempdir().unwrap();
    write_corpus(empty.path(), &three()).unwrap();

    // So are an empty set, mixed entries and duplicates refused.
    let again = tempfile::tempdir().unwrap();
    assert!(write_corpus(&again.path().join("c"), &[]).is_err());
    let mut mixed = three();
    mixed[1].0.argv[0] = "corejson/default/Other".to_owned();
    assert!(write_corpus(&again.path().join("m"), &mixed).is_err());
    let mut duplicate = three();
    duplicate.push(duplicate[0].clone());
    assert!(write_corpus(&again.path().join("d"), &duplicate).is_err());
}

#[test]
fn eqv_02_corpus_detects_a_foreign_entry() {
    // An extra file.
    let (_tmp, dir) = written();
    fs::write(dir.join("notes.txt"), b"x").unwrap();
    assert!(read_corpus(&dir).is_err(), "extra file");

    // A subdirectory.
    let (_tmp, dir) = written();
    fs::create_dir(dir.join("sub")).unwrap();
    assert!(read_corpus(&dir).is_err(), "subdirectory");

    // A symlink under a valid name.
    let (_tmp, dir) = written();
    let victim = sha_files(&dir)[0].clone();
    fs::remove_file(dir.join(&victim)).unwrap();
    std::os::unix::fs::symlink(dir.join("index.json"), dir.join(&victim)).unwrap();
    let err = read_corpus(&dir).unwrap_err().to_string();
    assert!(err.contains("symlink") || err.contains("regular"), "{err}");

    // A name that is not a sha256.
    let (_tmp, dir) = written();
    let victim = sha_files(&dir)[0].clone();
    fs::rename(dir.join(&victim), dir.join("abc.json")).unwrap();
    assert!(read_corpus(&dir).is_err(), "name is not a sha256");

    // Upper-case hex in a name.
    let (_tmp, dir) = written();
    let victim = sha_files(&dir)[0].clone();
    let upper = format!("{}.json", victim.trim_end_matches(".json").to_uppercase());
    fs::rename(dir.join(&victim), dir.join(upper)).unwrap();
    assert!(read_corpus(&dir).is_err(), "upper-case name");

    // A file whose sha256 differs from its name.
    let (_tmp, dir) = written();
    let victim = sha_files(&dir)[0].clone();
    let mut bytes = fs::read(dir.join(&victim)).unwrap();
    bytes.push(b'\n');
    fs::write(dir.join(&victim), bytes).unwrap();
    let err = read_corpus(&dir).unwrap_err().to_string();
    assert!(err.contains("sha256"), "{err}");

    // An index key without a file.
    let (_tmp, dir) = written();
    let victim = sha_files(&dir)[0].clone();
    fs::remove_file(dir.join(&victim)).unwrap();
    let err = read_corpus(&dir).unwrap_err().to_string();
    assert!(err.contains(victim.trim_end_matches(".json")), "{err}");

    // A vector file without an index key.
    let (_tmp, dir) = written();
    let index: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.join("index.json")).unwrap()).unwrap();
    let victim = sha_files(&dir)[0].clone();
    let mut edited = index;
    edited["vectors"]
        .as_object_mut()
        .unwrap()
        .remove(victim.trim_end_matches(".json"));
    fs::write(
        dir.join("index.json"),
        serde_json::to_vec_pretty(&edited).unwrap(),
    )
    .unwrap();
    assert!(read_corpus(&dir).is_err(), "file without an index key");

    // A missing index.
    let (_tmp, dir) = written();
    fs::remove_file(dir.join("index.json")).unwrap();
    assert!(read_corpus(&dir).is_err(), "missing index");
}

#[test]
fn eqv_02_provenance_has_ub_source_unchecked_for_unlabelled_vectors() {
    let tmp = tempfile::tempdir().unwrap();
    let mut n = 0;
    let mut try_write = |set: Vec<(Vector, Provenance)>| {
        n += 1;
        write_corpus(&tmp.path().join(format!("c{n}")), &set)
    };

    // An unlabelled vector must say unchecked or phase4.
    let plain = vector(0, "outcome=ok");
    for source in [
        HasUbSource::ObservedCrash,
        HasUbSource::ObservedAbort,
        HasUbSource::OobWrite,
        HasUbSource::Timeout,
    ] {
        let err = try_write(vec![(plain.clone(), provenance("ok", source))]).unwrap_err();
        assert!(err.to_string().contains("has_ub"), "{err}");
    }
    assert!(
        try_write(vec![(
            plain.clone(),
            provenance("ok", HasUbSource::Unchecked)
        )])
        .is_ok()
    );
    assert!(try_write(vec![(plain.clone(), provenance("ok", HasUbSource::Phase4))]).is_ok());

    // A labelled vector must say how the label was observed, or phase4.
    let mut crash = vector(-11, "outcome=fault");
    crash.has_ub = Some(HasUbLabel::new("crash:SIGSEGV").unwrap());
    let err = try_write(vec![(
        crash.clone(),
        provenance("fault", HasUbSource::Unchecked),
    )])
    .unwrap_err();
    assert!(err.to_string().contains("has_ub"), "{err}");
    assert!(
        try_write(vec![(
            crash.clone(),
            provenance("fault", HasUbSource::ObservedCrash)
        )])
        .is_ok()
    );
    assert!(
        try_write(vec![(
            crash.clone(),
            provenance("fault", HasUbSource::Phase4)
        )])
        .is_ok()
    );

    // The recorded outcome must be the vector's own.
    let err = try_write(vec![(
        plain.clone(),
        provenance("fault", HasUbSource::Unchecked),
    )])
    .unwrap_err();
    assert!(err.to_string().contains("outcome"), "{err}");

    // The unobserved count must be the number of marks in the vector's state.
    let mut counted = provenance("ok", HasUbSource::Unchecked);
    counted.unobserved = 5;
    let err = try_write(vec![(plain.clone(), counted)]).unwrap_err();
    assert!(err.to_string().contains("unobserved"), "{err}");

    // Malformed provenance text.
    let mut bad = provenance("ok", HasUbSource::Unchecked);
    bad.archive = "not-a-hash".to_owned();
    assert!(try_write(vec![(plain.clone(), bad)]).is_err());
    let mut bad = provenance("ok", HasUbSource::Unchecked);
    bad.replay = "maybe".to_owned();
    assert!(try_write(vec![(plain, bad)]).is_err());

    // An index edited after writing is caught on reading.
    let (_tmp2, dir) = written();
    let text = fs::read_to_string(dir.join("index.json")).unwrap();
    let edited = text.replace(
        "\"has_ub_source\": \"unchecked\"",
        "\"has_ub_source\": \"observed_crash\"",
    );
    assert_ne!(text, edited);
    fs::write(dir.join("index.json"), edited).unwrap();
    assert!(read_corpus(&dir).is_err());
}
