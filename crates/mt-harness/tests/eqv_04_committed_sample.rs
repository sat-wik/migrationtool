#![allow(clippy::unwrap_used, clippy::expect_used)]
//! EQV-04 (CONTEXT D-19, D-21; T-03-29): the committed sample of reference
//! vectors, `testbed/vectors-sample`. One vector per drivable target, made by CI in
//! the pinned image from the project's own committed seeds. Offline: committed
//! files only, no tool runs, no network.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use mt_capture::droplog::DropLog;
use mt_capture::manifest::Manifest;
use mt_harness::adapter_drops::{AdapterDropLog, drivable_targets};
use mt_harness::replay::corpus_dirs_below;
use mt_vectors::bytes::decode_latin1;
use mt_vectors::corpus::read_corpus;
use mt_vectors::sha256_hex;

fn testbed() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testbed")
}

/// Every file below `dir`, by path relative to it.
fn files_below(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        for item in std::fs::read_dir(&current).unwrap() {
            let item = item.unwrap();
            let kind = item.file_type().unwrap();
            if kind.is_dir() {
                stack.push(item.path());
            } else {
                assert!(
                    kind.is_file(),
                    "{} is not a regular file",
                    item.path().display()
                );
                found.push(item.path().strip_prefix(dir).unwrap().to_path_buf());
            }
        }
    }
    found.sort();
    found
}

#[test]
fn eqv_04_committed_sample_is_valid() {
    let sample = testbed().join("vectors-sample");
    let notice = sample.join("NOTICE");
    if !sample.exists() {
        // Before the sample commit the directory is absent, and so is its NOTICE.
        assert!(!notice.exists(), "a NOTICE without a sample");
        return;
    }
    let manifest = Manifest::load(&testbed().join("manifest.toml")).unwrap();
    manifest.validate().unwrap();
    let phase2 = DropLog::load_from_testbed_dir(&testbed()).unwrap();
    let adapter = AdapterDropLog::load_from_testbed_dir(&testbed()).unwrap();
    let seeds = testbed().join("seeds");
    let drivable = drivable_targets(&manifest, &seeds, &phase2, &adapter);
    assert!(drivable.len() >= 10, "{drivable:?}");

    // The NOTICE states where the inputs come from, who produces the files, and
    // the licence of every sampled target.
    let text = std::fs::read_to_string(&notice).unwrap();
    for needle in [
        "testbed/seeds/",
        "pinned image",
        "by hand",
        "vectors-sample",
    ] {
        assert!(text.contains(needle), "the NOTICE lacks {needle:?}");
    }
    let lowered = text.to_lowercase();
    for word in ["certified", "compliant"] {
        assert!(!lowered.contains(word), "the NOTICE says {word}");
    }
    for target in &drivable {
        assert!(text.contains(target.as_str()), "the NOTICE omits {target}");
        let elected = &manifest.target[target].licence.elected;
        assert!(
            text.contains(elected.as_str()),
            "{target}: licence {elected}"
        );
    }

    // Nothing but the NOTICE lies outside a corpus directory.
    let corpora: Vec<PathBuf> = corpus_dirs_below(&sample).unwrap();
    let corpus_set: BTreeSet<PathBuf> = corpora.iter().cloned().collect();
    for file in files_below(&sample) {
        if file == Path::new("NOTICE") {
            continue;
        }
        let parent = sample.join(file.parent().unwrap());
        assert!(
            corpus_set.contains(&parent),
            "{} lies outside a corpus directory",
            file.display()
        );
    }

    let mut covered = BTreeSet::new();
    for dir in &corpora {
        let relative = dir.strip_prefix(&sample).unwrap();
        let parts: Vec<String> = relative
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            parts.len(),
            3,
            "{}: <target>/default/<entry>",
            relative.display()
        );
        let (target, config, entry) = (&parts[0], &parts[1], &parts[2]);
        assert_eq!(config, "default", "{target}: the default configuration");
        let corpus = read_corpus(dir).unwrap();
        assert_eq!(
            corpus.vectors.len(),
            1,
            "{}: one vector",
            relative.display()
        );
        assert_eq!(
            (
                &corpus.index.target,
                &corpus.index.config,
                &corpus.index.entry
            ),
            (target, config, entry)
        );

        // The first drivable entry of the target by name.
        let mut candidates: Vec<&str> = manifest.target[target]
            .entry
            .iter()
            .filter(|e| !e.arg.is_empty() && adapter.row(target, &e.name).is_none())
            .map(|e| e.name.as_str())
            .collect();
        candidates.sort_unstable();
        assert_eq!(
            candidates.first().copied(),
            Some(entry.as_str()),
            "{target}"
        );

        let (sha, vector) = corpus.vectors.iter().next().unwrap();
        let provenance = &corpus.index.vectors[sha];
        assert_eq!(provenance.replay, "identical", "{target}: replay identical");
        assert!(
            provenance.image.contains(&manifest.image.digest),
            "{target}: the image {:?} is not the pinned digest",
            provenance.image
        );
        assert_eq!(
            provenance.issues(vector),
            Vec::<String>::new(),
            "{target}: has_ub source against the label"
        );
        assert_eq!(
            vector.argv.first().map(String::as_str),
            Some(format!("{target}/{config}/{entry}").as_str())
        );

        // The input is one of the target's own committed seeds, byte for byte, and
        // the smallest one that lists the entry (ties by sha256).
        let index: serde_json::Value =
            serde_json::from_slice(&std::fs::read(seeds.join(target).join("index.json")).unwrap())
                .unwrap();
        let mut listed: Vec<(u64, String)> = index["seeds"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|seed| {
                seed["entries"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|name| name == entry.as_str())
            })
            .map(|seed| {
                (
                    seed["bytes"].as_u64().unwrap(),
                    seed["sha256"].as_str().unwrap().to_owned(),
                )
            })
            .collect();
        listed.sort();
        let stdin = decode_latin1(&vector.stdin).unwrap();
        assert_eq!(
            sha256_hex(&stdin),
            provenance.seed,
            "{target}: the seed hash"
        );
        let file = std::fs::read(seeds.join(target).join(&provenance.seed)).unwrap();
        assert_eq!(
            file, stdin,
            "{target}: the input is the committed seed file"
        );
        assert_eq!(
            listed.first().map(|(_, sha)| sha.as_str()),
            Some(provenance.seed.as_str()),
            "{target}: the smallest seed of {entry}"
        );
        covered.insert(target.clone());
    }
    assert_eq!(covered, drivable, "one sampled target per drivable target");
}

#[test]
fn eqv_04_replay_tree_finds_the_corpus_directories() {
    // `mt vectors replay --tree` replays every directory below the tree that holds an
    // index.json, in path order, and nothing else.
    let tmp = tempfile::tempdir().unwrap();
    let make = |relative: &str, file: &str| {
        let dir = tmp.path().join(relative);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(file), b"{}\n").unwrap();
    };
    make("b/default/entry2", "index.json");
    make("a/default/entry1", "index.json");
    make("a/default/entry1x", "other.json");
    std::fs::create_dir_all(tmp.path().join("empty/dir")).unwrap();
    let found = corpus_dirs_below(tmp.path()).unwrap();
    assert_eq!(
        found,
        [
            tmp.path().join("a/default/entry1"),
            tmp.path().join("b/default/entry2")
        ]
    );
    assert!(corpus_dirs_below(&tmp.path().join("missing")).is_err());
}
