#![allow(clippy::unwrap_used, clippy::expect_used)]
//! EQV-04 (T-03-13, T-03-17): replay applies the corpus structure checks before
//! it runs anything. Offline: the committed sample and the committed manifest
//! only, no tool runs.

use std::path::{Path, PathBuf};

use mt_capture::manifest::Manifest;
use mt_harness::replay::{check_corpus_shape, corpus_dirs_below};
use mt_vectors::corpus::CorpusIndex;

fn testbed() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testbed")
}

fn sample_dir() -> PathBuf {
    testbed().join("vectors-sample/zlib/default/uncompress")
}

fn load(dir: &Path) -> (CorpusIndex, Manifest) {
    let index = serde_json::from_slice(&std::fs::read(dir.join("index.json")).unwrap()).unwrap();
    let manifest = Manifest::load(&testbed().join("manifest.toml")).unwrap();
    (index, manifest)
}

fn copy_sample() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    for item in std::fs::read_dir(sample_dir()).unwrap() {
        let item = item.unwrap();
        std::fs::copy(item.path(), tmp.path().join(item.file_name())).unwrap();
    }
    tmp
}

#[test]
fn eqv_04_replay_shape_accepts_the_committed_sample() {
    let (index, manifest) = load(&sample_dir());
    check_corpus_shape(&sample_dir(), &index, &manifest).unwrap();
}

#[test]
fn eqv_04_replay_shape_refuses_a_stray_file() {
    let tmp = copy_sample();
    let stray = "0".repeat(64);
    std::fs::write(tmp.path().join(format!("{stray}.json")), b"{}").unwrap();
    let (index, manifest) = load(tmp.path());
    let error = check_corpus_shape(tmp.path(), &index, &manifest).unwrap_err();
    assert!(error.to_string().contains("not in the index"), "{error}");
}

#[test]
fn eqv_04_replay_shape_refuses_a_traversing_index_key() {
    let tmp = copy_sample();
    let (mut index, manifest) = load(tmp.path());
    let (_, provenance) = index.vectors.pop_first().unwrap();
    index.vectors.insert("../../outside".to_owned(), provenance);
    let error = check_corpus_shape(tmp.path(), &index, &manifest).unwrap_err();
    assert!(
        error.to_string().contains("not a lowercase sha256"),
        "{error}"
    );
}

#[test]
fn eqv_04_replay_shape_refuses_an_unknown_configuration() {
    let tmp = copy_sample();
    let (mut index, manifest) = load(tmp.path());
    index.config = "../default".to_owned();
    let error = check_corpus_shape(tmp.path(), &index, &manifest).unwrap_err();
    assert!(error.to_string().contains("no configuration"), "{error}");
}

#[cfg(unix)]
#[test]
fn eqv_04_replay_shape_refuses_a_symlink() {
    let tmp = copy_sample();
    std::os::unix::fs::symlink("index.json", tmp.path().join("link.json")).unwrap();
    let (index, manifest) = load(tmp.path());
    let error = check_corpus_shape(tmp.path(), &index, &manifest).unwrap_err();
    assert!(error.to_string().contains("not a regular file"), "{error}");
}

#[test]
fn eqv_04_replay_shape_accepts_every_committed_corpus() {
    let manifest = Manifest::load(&testbed().join("manifest.toml")).unwrap();
    manifest.validate().unwrap();
    let dirs = corpus_dirs_below(&testbed().join("vectors-sample")).unwrap();
    assert!(!dirs.is_empty());
    for dir in dirs {
        let (index, _) = load(&dir);
        check_corpus_shape(&dir, &index, &manifest).unwrap();
    }
}
