#![allow(clippy::unwrap_used, clippy::expect_used)]
//! EQV-01: the contract document and the code cannot drift apart unnoticed.
//! `docs/specs/equivalence-contract.md` carries a version line that must equal
//! `CONTRACT_VERSION`, the rules the plan names, and no claim of assurance
//! the tool does not give.

use std::path::{Path, PathBuf};

use mt_contract::CONTRACT_VERSION;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn read(relative: &str) -> String {
    let path = repo_root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

const DOCUMENT: &str = "docs/specs/equivalence-contract.md";

#[test]
fn eqv_01_contract_document_has_version_and_changelog() {
    let text = read(DOCUMENT);
    let versions: Vec<&str> = text
        .lines()
        .filter_map(|l| l.strip_prefix("**Version:** "))
        .collect();
    assert_eq!(
        versions.len(),
        1,
        "exactly one `**Version:**` line: {versions:?}"
    );
    assert_eq!(
        versions[0], CONTRACT_VERSION,
        "the document and CONTRACT_VERSION differ"
    );
    let lines: Vec<&str> = text.lines().collect();
    let heading = lines
        .iter()
        .position(|l| *l == "## Changelog")
        .expect("a `## Changelog` heading");
    assert!(
        lines[heading + 1..]
            .iter()
            .any(|l| l.starts_with("- 0.1.0")),
        "a `- 0.1.0` entry below the changelog heading"
    );
}

#[test]
fn eqv_01_contract_document_states_the_rules() {
    let text = read(DOCUMENT);
    let headings = [
        "## Purpose and non-claims",
        "## Definitions",
        "## Refinement",
        "## Observation point",
        "## Field convention",
        "## Library state",
        "## Comparison rule",
        "## UB action menu and policy",
        "## Outcomes",
        "## Reference executor",
        "## Out of scope",
        "## Changelog",
    ];
    let lines: Vec<&str> = text.lines().collect();
    let mut previous = 0;
    for heading in headings {
        let at = lines
            .iter()
            .position(|l| *l == heading)
            .unwrap_or_else(|| panic!("missing heading {heading:?}"));
        assert!(at > previous, "{heading:?} is out of order");
        previous = at;
    }
    let fields = [
        "argv",
        "stdin",
        "env",
        "stdout",
        "stderr",
        "rc",
        "lib_state_in",
        "lib_state_out",
        "has_ub",
    ];
    for field in fields {
        assert!(
            text.contains(&format!("`{field}`")),
            "the field `{field}` is not named"
        );
    }
    for needle in [
        "return_error",
        "panic",
        "preserve",
        "default_change",
        "Latin-1",
        "is_regex",
        "-signal",
        "sentinel",
        "UB unchecked",
        "setup_failed",
        "executor_error",
        "unobserved",
        "globals",
        "{buffer, offset}",
    ] {
        assert!(
            text.contains(needle),
            "the contract does not state {needle:?}"
        );
    }
}

#[test]
fn eqv_01_contract_document_makes_no_claim() {
    // Built from parts so this file is not itself a match.
    let words = [["certi", "fied"].concat(), ["compl", "iant"].concat()];
    let mut files = vec![DOCUMENT.to_owned()];
    for entry in std::fs::read_dir(repo_root().join("testbed/policy")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "toml") {
            files.push(format!(
                "testbed/policy/{}",
                path.file_name().unwrap().to_str().unwrap()
            ));
        }
    }
    for file in files {
        let text = read(&file).to_lowercase();
        for word in &words {
            assert!(
                !text.contains(word.as_str()),
                "{file} contains the forbidden word {word:?}"
            );
        }
    }
}
