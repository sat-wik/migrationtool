#![allow(clippy::unwrap_used, clippy::expect_used)]
//! EQV-01: the per-module policy file. Strict loading, the schema and
//! contract version checks, and the module default (CONTEXT D-15, D-16).

use std::path::{Path, PathBuf};

use mt_contract::{Action, CONTRACT_VERSION, Policy};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// The smallest valid policy for the module `corejson`.
const BASE: &str = "schema_version = 1\ncontract_version = \"0.1.0\"\nmodule = \"corejson\"\n";

fn with_ub(head: &str, ub: &str) -> String {
    format!("{BASE}{head}\n[ub]\n{ub}")
}

/// Parse, then validate with no logged site; the error is the message text.
fn verdict(text: &str) -> Result<Policy, String> {
    let policy = Policy::parse(text).map_err(|e| e.to_string())?;
    policy.validate(&[]).map_err(|e| e.to_string())?;
    Ok(policy)
}

#[test]
fn eqv_01_policy_loads_and_defaults_to_return_error() {
    let path = repo_root().join("testbed/policy/corejson.toml");
    let text = std::fs::read_to_string(&path).unwrap();
    let parsed = Policy::parse(&text).unwrap();
    let loaded = Policy::load(&path).unwrap();
    assert_eq!(parsed, loaded);
    assert_eq!(loaded.ub.default_action, Action::ReturnError);
    assert_eq!(loaded.ub.default_action.as_str(), "return_error");
    loaded.validate(&[]).unwrap();
    assert_eq!(loaded.module, "corejson");
    assert_eq!(loaded.contract_version, CONTRACT_VERSION);
}

#[test]
fn eqv_01_rejects_unknown_key() {
    let full_override = "[[ub.override]]\nfunction = \"f\"\nkind = \"k\"\nlocation = \"l\"\n\
        action = \"preserve\"\nreason = \"r\"\napprover = \"a\"\ndate = \"2026-10-21\"\n";
    let cases = [
        (
            "top",
            with_ub("bogus_top = 1\n", "default_action = \"return_error\"\n"),
            "bogus_top",
        ),
        (
            "ub",
            with_ub("", "default_action = \"return_error\"\nbogus_ub = 1\n"),
            "bogus_ub",
        ),
        (
            "default_change",
            with_ub(
                "",
                "default_action = \"panic\"\n[ub.default_change]\nmodule = \"corejson\"\n\
                 reason = \"r\"\napprover = \"a\"\ndate = \"2026-10-21\"\nbogus_dc = 1\n",
            ),
            "bogus_dc",
        ),
        (
            "override",
            with_ub(
                "",
                &format!("default_action = \"return_error\"\n{full_override}bogus_ov = 1\n"),
            ),
            "bogus_ov",
        ),
        (
            "error_code",
            with_ub(
                "",
                "default_action = \"return_error\"\n[[ub.error_code]]\nfunction = \"f\"\n\
                 value = \"-1\"\nbogus_ec = 1\n",
            ),
            "bogus_ec",
        ),
    ];
    for (level, text, key) in cases {
        let err = Policy::parse(&text).expect_err(level).to_string();
        assert!(
            err.contains(key),
            "{level}: the error must name {key}: {err}"
        );
    }
}

#[test]
fn eqv_01_rejects_other_schema_version() {
    let text = format!(
        "schema_version = 2\ncontract_version = \"0.1.0\"\nmodule = \"corejson\"\n\n\
         [ub]\ndefault_action = \"return_error\"\n"
    );
    let err = verdict(&text).unwrap_err();
    assert!(err.contains("schema_version"), "{err}");
    // The same text with version 1 is fine.
    verdict(&text.replace("schema_version = 2", "schema_version = 1")).unwrap();
}
