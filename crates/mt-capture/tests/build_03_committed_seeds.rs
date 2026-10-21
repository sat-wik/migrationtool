#![allow(clippy::unwrap_used, clippy::expect_used)]
//! BUILD-03 over the committed data: `testbed/seeds/<id>/` holds a deterministic,
//! capped, provenance-carrying seed corpus for every target of
//! `testbed/manifest.toml`, taken only from the paths the manifest declares or
//! hand-made and marked proposed (CONTEXT D-13, D-15, D-16, RESEARCH Pitfall 18).
//! Plain `cargo test`: it reads committed files only. That the corpora equal what
//! the pinned sources regenerate is checked by `mt testbed seeds --check` in the
//! container (the testbed workflow), which needs the fetched trees.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use mt_capture::droplog::DropLog;
use mt_capture::manifest::{Manifest, Source, Target};
use mt_seeds::index::SeedIndex;
use mt_seeds::{Method, sha256_hex};

fn repo_file(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn manifest() -> Manifest {
    let manifest = Manifest::load(&repo_file("testbed/manifest.toml")).expect("manifest parses");
    manifest.validate().expect("manifest is valid");
    manifest
}

fn drop_log() -> DropLog {
    let log = DropLog::load(&repo_file("testbed/dropped.toml")).expect("dropped.toml parses");
    log.validate(&manifest())
        .expect("dropped.toml is valid for the manifest");
    log
}

/// The targets that are not fully dropped, with their manifest entries.
fn retained(manifest: &Manifest) -> Vec<(&String, &Target)> {
    let kept = drop_log().retained_targets(manifest);
    manifest
        .target
        .iter()
        .filter(|(id, _)| kept.contains(id))
        .collect()
}

fn corpus_dir(id: &str) -> PathBuf {
    repo_file("testbed/seeds").join(id)
}

fn index_of(id: &str) -> SeedIndex {
    let path = corpus_dir(id).join("index.json");
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} cannot be read: {error}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn commit_of(target: &Target) -> &str {
    match &target.source {
        Source::Git { commit, .. } | Source::Archive { commit, .. } => commit,
    }
}

#[test]
fn build_03_every_retained_target_has_a_seed_corpus() {
    let manifest = manifest();
    let retained = drop_log().retained_targets(&manifest);
    assert!(
        !retained.is_empty(),
        "no target is retained, so no corpus was checked"
    );
    for id in &retained {
        let dir = corpus_dir(id);
        assert!(dir.join("index.json").is_file(), "{id}: no index.json");
        assert!(dir.join("NOTICE").is_file(), "{id}: no NOTICE");
        let index = index_of(id);
        assert_eq!(index.target, *id);
        assert!(!index.seeds.is_empty(), "{id}: the corpus holds no seed");
    }
    // No corpus for a target that is not retained (a dropped target must take
    // its seeds with it), and none for a name the manifest does not have.
    let known: BTreeSet<&str> = retained.iter().map(String::as_str).collect();
    for entry in fs::read_dir(repo_file("testbed/seeds")).unwrap() {
        let name = entry.unwrap().file_name().into_string().unwrap();
        assert!(
            known.contains(name.as_str()),
            "stray corpus directory {name}"
        );
    }
}

#[test]
fn build_03_dropped_targets_have_no_seed_corpus() {
    let manifest = manifest();
    let log = drop_log();
    let dropped = log.dropped_targets();
    for id in &dropped {
        assert!(
            manifest.target.contains_key(id),
            "{id} left the manifest instead of being dropped"
        );
        assert!(
            !corpus_dir(id).exists(),
            "{id} is dropped, so testbed/seeds/{id} must be removed"
        );
    }
    // Every seed corpus belongs to a retained target and every retained target
    // has one; the two sets are the same.
    let on_disk: BTreeSet<String> = fs::read_dir(repo_file("testbed/seeds"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    let retained: BTreeSet<String> = log.retained_targets(&manifest).into_iter().collect();
    assert_eq!(on_disk, retained);
    assert_eq!(retained.len() + dropped.len(), manifest.target.len());
}

#[test]
fn build_03_hand_made_seeds_marked_proposed_and_at_least_ten() {
    let manifest = manifest();
    let mut with_hand_made = 0;
    for (id, _) in retained(&manifest) {
        let index = index_of(id);
        let hand_made = index
            .seeds
            .iter()
            .filter(|seed| seed.method == Method::HandMade)
            .count();
        let extracted = index.seeds.len() - hand_made;
        for seed in &index.seeds {
            assert_eq!(
                seed.proposed,
                seed.method == Method::HandMade,
                "{id}: {} is {:?} with proposed {}; exactly the hand-made seeds are proposed",
                seed.sha256,
                seed.method,
                seed.proposed
            );
        }
        if extracted == 0 {
            assert!(
                hand_made >= 10,
                "{id} has no extracted seed and only {hand_made} hand-made ones; at least 10 are needed"
            );
        }
        if hand_made > 0 {
            with_hand_made += 1;
        }
        // Every hand-made .hex file under testbed/targets is a seed of the index
        // (or a duplicate of another seed), starts with a comment naming its
        // specification, and nothing else is in that directory.
        let dir = repo_file("testbed/targets").join(id).join("hand-made");
        if !dir.is_dir() {
            assert_eq!(
                hand_made, 0,
                "{id}: hand-made seeds without a hand-made directory"
            );
            continue;
        }
        let mut files = 0;
        for entry in fs::read_dir(&dir).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name().into_string().unwrap();
            assert!(name.ends_with(".hex"), "{id}: {name} is not a .hex file");
            let text = fs::read_to_string(entry.path()).unwrap();
            let first = text.lines().next().unwrap_or("");
            assert!(
                first.starts_with("# ") && first.len() > 12,
                "{id}/{name}: the first line must be a comment naming the specification"
            );
            files += 1;
        }
        assert_eq!(
            files, hand_made,
            "{id}: .hex files and indexed hand-made seeds differ"
        );
    }
    assert!(
        with_hand_made >= 6,
        "paho, microcoap, libfdt, isotp, lz4 and zlib carry hand-made sets, found {with_hand_made} targets"
    );
}

#[test]
fn build_03_seed_provenance_records_source_path_commit_and_method() {
    let manifest = manifest();
    for (id, target) in retained(&manifest) {
        let index = index_of(id);
        assert_eq!(
            index.commit,
            commit_of(target),
            "{id}: the index commit is not the pin"
        );
        assert_eq!(
            index.schema_version,
            mt_seeds::index::SEED_INDEX_SCHEMA_VERSION
        );
        let mut expected: BTreeSet<String> = index.seeds.iter().map(|s| s.sha256.clone()).collect();
        assert_eq!(
            expected.len(),
            index.seeds.len(),
            "{id}: duplicate seed in the index"
        );
        let sorted: Vec<&str> = index.seeds.iter().map(|s| s.sha256.as_str()).collect();
        let mut check = sorted.clone();
        check.sort_unstable();
        assert_eq!(sorted, check, "{id}: seeds are not sorted by sha256");
        for seed in &index.seeds {
            assert!(
                !seed.source.path.is_empty(),
                "{id}: seed without a source path"
            );
            match seed.method {
                Method::CArray | Method::CString => {
                    assert!(
                        seed.source.line.is_some(),
                        "{id}: {} has no line",
                        seed.sha256
                    );
                }
                Method::ZipMember => {
                    assert!(
                        seed.source.member.is_some(),
                        "{id}: {} has no member",
                        seed.sha256
                    );
                }
                _ => {}
            }
            assert!(
                !seed.entries.is_empty(),
                "{id}: {} names no entry",
                seed.sha256
            );
            for entry in &seed.entries {
                assert!(
                    target.entry.iter().any(|e| &e.name == entry),
                    "{id}: {entry} is not an entry of the target"
                );
            }
            let path = corpus_dir(id).join(&seed.sha256);
            let bytes = fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            assert_eq!(
                sha256_hex(&bytes),
                seed.sha256,
                "{id}: a file is not named by its sha256"
            );
            assert_eq!(
                bytes.len() as u64,
                seed.bytes,
                "{id}: {} has another length",
                seed.sha256
            );
        }
        // The directory holds exactly the indexed files plus index.json and NOTICE.
        for entry in fs::read_dir(corpus_dir(id)).unwrap() {
            let entry = entry.unwrap();
            assert!(
                entry.file_type().unwrap().is_file(),
                "{id}: a non-file in the corpus"
            );
            let name = entry.file_name().into_string().unwrap();
            if name != "index.json" && name != "NOTICE" {
                assert!(expected.remove(&name), "{id}: {name} is not in the index");
            }
        }
        assert!(
            expected.is_empty(),
            "{id}: indexed seeds missing on disk: {expected:?}"
        );
    }
}

#[test]
fn build_03_committed_seeds_respect_the_caps() {
    let manifest = manifest();
    let caps = manifest.seed_caps;
    for (id, _) in retained(&manifest) {
        let index = index_of(id);
        assert_eq!(index.caps.max_seeds, caps.max_seeds, "{id}");
        assert_eq!(index.caps.max_total_bytes, caps.max_total_bytes, "{id}");
        assert_eq!(index.caps.max_seed_bytes, caps.max_seed_bytes, "{id}");
        assert!(
            index.seeds.len() as u64 <= caps.max_seeds,
            "{id}: too many seeds"
        );
        let total: u64 = index.seeds.iter().map(|s| s.bytes).sum();
        assert!(
            total <= caps.max_total_bytes,
            "{id}: {total} bytes in total"
        );
        assert!(
            index.seeds.iter().all(|s| s.bytes <= caps.max_seed_bytes),
            "{id}: a seed is larger than max_seed_bytes"
        );
    }
}

#[test]
fn build_03_seed_licence_matches_manifest() {
    let manifest = manifest();
    for (id, target) in retained(&manifest) {
        let index = index_of(id);
        assert_eq!(index.licence.spdx, target.licence.spdx, "{id}");
        assert_eq!(
            index.licence.elected.as_deref(),
            Some(target.licence.elected.as_str()),
            "{id}"
        );
        assert_eq!(index.licence.file, target.licence.file, "{id}");
        let notice = fs::read_to_string(corpus_dir(id).join("NOTICE")).unwrap();
        for needle in [
            target.licence.spdx.as_str(),
            target.licence.elected.as_str(),
            target.licence.file.as_str(),
            commit_of(target),
            "hand_made",
            "written for this project",
        ] {
            assert!(
                notice.contains(needle),
                "{id}: NOTICE does not contain {needle:?}"
            );
        }
        let url = match &target.source {
            Source::Git { url, .. } | Source::Archive { url, .. } => url,
        };
        assert!(
            notice.contains(url.as_str()),
            "{id}: NOTICE does not name the upstream URL"
        );
        for word in ["certified", "compliant"] {
            assert!(
                !notice.to_lowercase().contains(word),
                "{id}: NOTICE uses the word {word}"
            );
        }
    }
}

#[test]
fn build_03_seeds_come_only_from_declared_sources() {
    let manifest = manifest();
    for (id, target) in retained(&manifest) {
        let index = index_of(id);
        let hand_made_prefix = format!("testbed/targets/{id}/hand-made/");
        for seed in &index.seeds {
            if seed.method == Method::HandMade {
                assert!(
                    seed.source.path.starts_with(&hand_made_prefix),
                    "{id}: hand-made seed with the source path {}",
                    seed.source.path
                );
                assert!(
                    repo_file(&seed.source.path).is_file(),
                    "{id}: {} is not a committed file",
                    seed.source.path
                );
                continue;
            }
            let declared = target.seed_source.iter().any(|source| {
                seed.source.path == source.path
                    || seed.source.path.starts_with(&format!("{}/", source.path))
            });
            assert!(
                declared,
                "{id}: {} ({:?}) is not under any seed_source path",
                seed.source.path, seed.method
            );
        }
    }
    // The mixed-licence repositories give no upstream seed at all (Pitfall 18):
    // lz4 outside lib/ and dtc outside libfdt/ are GPL, so only hand-made seeds.
    let kept = drop_log().retained_targets(&manifest);
    for id in ["lz4", "libfdt"] {
        // A dropped target has no corpus to inspect; its declared sources stay
        // empty in the manifest either way.
        assert!(
            manifest.target[id].seed_source.is_empty(),
            "{id}: a copyleft part of the repository must not be a seed source"
        );
        if !kept.iter().any(|k| k == id) {
            continue;
        }
        assert!(
            index_of(id)
                .seeds
                .iter()
                .all(|s| s.method == Method::HandMade),
            "{id}: only hand-made seeds are allowed"
        );
    }
}
