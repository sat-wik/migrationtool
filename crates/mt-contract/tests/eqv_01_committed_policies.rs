#![allow(clippy::unwrap_used, clippy::expect_used)]
//! EQV-01: the 16 committed policies, one per test-bed target (D-15, D-16).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use mt_contract::{Action, CONTRACT_VERSION, Policy};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// The target ids of `testbed/manifest.toml`, read without `mt-capture`.
fn manifest_ids() -> BTreeSet<String> {
    let text = std::fs::read_to_string(repo_root().join("testbed/manifest.toml")).unwrap();
    let table: toml::Table = text.parse().unwrap();
    table["target"]
        .as_table()
        .unwrap()
        .keys()
        .cloned()
        .collect()
}

#[test]
fn eqv_01_committed_policies_cover_every_target() {
    let ids = manifest_ids();
    assert_eq!(ids.len(), 16, "the manifest has 16 targets: {ids:?}");
    let dir = repo_root().join("testbed/policy");
    let mut stems = BTreeSet::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "toml") {
            stems.insert(path.file_stem().unwrap().to_str().unwrap().to_owned());
        }
    }
    assert_eq!(stems, ids, "one policy per manifest target, and no other");
    for id in &ids {
        let path = dir.join(format!("{id}.toml"));
        let policy = Policy::load(&path).unwrap_or_else(|e| panic!("{id}: {e}"));
        // Phase 3 passes an empty known-site list, so any override fails here.
        policy.validate(&[]).unwrap_or_else(|e| panic!("{id}: {e}"));
        assert_eq!(&policy.module, id);
        assert_eq!(policy.contract_version, CONTRACT_VERSION, "{id}");
        assert_eq!(policy.ub.default_action, Action::ReturnError, "{id}");
        assert!(
            policy.ub.default_change.is_none(),
            "{id}: no default change"
        );
        assert!(policy.ub.error_code.is_empty(), "{id}: no error code yet");
        assert!(
            policy.ub.r#override.is_empty(),
            "{id}: no override in Phase 3"
        );
    }
}
