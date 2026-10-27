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
    let text = "schema_version = 2\ncontract_version = \"0.1.0\"\nmodule = \"corejson\"\n\n\
         [ub]\ndefault_action = \"return_error\"\n";
    let err = verdict(text).unwrap_err();
    assert!(err.contains("schema_version"), "{err}");
    // The same text with version 1 is fine.
    verdict(&text.replace("schema_version = 2", "schema_version = 1")).unwrap();
}

// ---------------------------------------------------------------------------
// Task 2: the full rules (D-16, D-18, D-29).

use mt_contract::Site;

/// Like `verdict`, with the UB log's known sites.
fn verdict_with(text: &str, sites: &[Site]) -> Result<Policy, String> {
    let policy = Policy::parse(text).map_err(|e| e.to_string())?;
    policy.validate(sites).map_err(|e| e.to_string())?;
    Ok(policy)
}

fn site(function: &str, kind: &str, location: &str) -> Site {
    Site {
        function: function.to_owned(),
        kind: kind.to_owned(),
        location: location.to_owned(),
    }
}

const DC_MODULE: &str = "module = \"corejson\"";

/// A `[ub]` table with a changed default and its record.
fn changed_default(action: &str, record: &str) -> String {
    with_ub(
        "",
        &format!("default_action = \"{action}\"\n[ub.default_change]\n{record}"),
    )
}

const FULL_RECORD: &str =
    "module = \"corejson\"\nreason = \"r\"\napprover = \"a\"\ndate = \"2026-10-21\"\n";

#[test]
fn eqv_01_default_change_required_to_change_default() {
    for action in ["panic", "preserve"] {
        // No record: refused, and the issue names the key and the decision.
        let bare = with_ub("", &format!("default_action = \"{action}\"\n"));
        let err = verdict(&bare).unwrap_err();
        assert!(
            err.contains("default_change") && err.contains("D-16"),
            "{err}"
        );
        // A complete record: accepted.
        verdict(&changed_default(action, FULL_RECORD)).unwrap();
    }
    // Each part of the record is required to be non-empty, and the module to
    // be the file's own.
    let cases = [
        (
            "reason",
            FULL_RECORD.replace("reason = \"r\"", "reason = \"\""),
        ),
        (
            "approver",
            FULL_RECORD.replace("approver = \"a\"", "approver = \"  \""),
        ),
        (
            "module",
            FULL_RECORD.replace(DC_MODULE, "module = \"zlib\""),
        ),
        ("module", FULL_RECORD.replace(DC_MODULE, "module = \"\"")),
        ("date", FULL_RECORD.replace("2026-10-21", "")),
    ];
    for (key, record) in cases {
        let err = verdict(&changed_default("panic", &record)).unwrap_err();
        assert!(
            err.contains(key) && err.contains("D-29"),
            "{key}: the issue must name the key and D-29: {err}"
        );
    }
    // A key missing from the record is refused at parse time, naming it.
    for key in ["module", "reason", "approver", "date"] {
        let record: String = FULL_RECORD
            .lines()
            .filter(|l| !l.starts_with(key))
            .map(|l| format!("{l}\n"))
            .collect();
        let err = verdict(&changed_default("panic", &record)).unwrap_err();
        assert!(err.contains(key), "{key}: {err}");
    }
}

#[test]
fn eqv_01_default_change_only_with_a_changed_default() {
    let text = changed_default("return_error", FULL_RECORD);
    let err = verdict(&text).unwrap_err();
    assert!(
        err.contains("default_change") && err.contains("return_error"),
        "{err}"
    );
}

const OVERRIDE_KEYS: [&str; 7] = [
    "function", "kind", "location", "action", "reason", "approver", "date",
];

/// One override with every key, with `edit` applied to the value of `key`.
fn override_text(edit: impl Fn(&str, &str) -> Option<String>) -> String {
    let defaults = [
        ("function", "JSON_Validate"),
        ("kind", "signed_overflow"),
        ("location", "source/core_json.c:123"),
        ("action", "preserve"),
        ("reason", "the checksum relies on wrapping"),
        ("approver", "founder"),
        ("date", "2026-10-21"),
    ];
    let mut text = String::from("[[ub.override]]\n");
    for (key, value) in defaults {
        if let Some(value) = edit(key, value) {
            text.push_str(&format!("{key} = \"{value}\"\n"));
        }
    }
    text
}

fn logged() -> Vec<Site> {
    vec![site(
        "JSON_Validate",
        "signed_overflow",
        "source/core_json.c:123",
    )]
}

fn policy_with_overrides(overrides: &str) -> String {
    with_ub(
        "",
        &format!("default_action = \"return_error\"\n{overrides}"),
    )
}

#[test]
fn eqv_01_override_needs_every_field() {
    // The complete record is accepted when its site is logged.
    let full = policy_with_overrides(&override_text(|_, v| Some(v.to_owned())));
    verdict_with(&full, &logged()).unwrap();
    for missing in OVERRIDE_KEYS {
        // A record missing the key is refused and the message names it.
        let absent =
            policy_with_overrides(&override_text(|k, v| (k != missing).then(|| v.to_owned())));
        let err = verdict_with(&absent, &logged()).unwrap_err();
        assert!(err.contains(missing), "missing {missing}: {err}");
        // So is one holding an empty (or blank) value.
        for blank in ["", "   "] {
            let empty = policy_with_overrides(&override_text(|k, v| {
                Some(if k == missing {
                    blank.to_owned()
                } else {
                    v.to_owned()
                })
            }));
            let err = verdict_with(&empty, &logged()).unwrap_err();
            assert!(err.contains(missing), "empty {missing}: {err}");
        }
    }
}

#[test]
fn eqv_01_rejects_bad_dates() {
    let bad = [
        "2026-02-29",
        "2026-13-01",
        "2026-04-31",
        "2026-00-10",
        "26-1-1",
        "1900-02-29",
        "2026-10-21T00:00:00Z",
        "2026/10/21",
        "2026-10-2",
        "+026-10-21",
        "2026-1a-21",
    ];
    let good = ["2028-02-29", "2000-02-29", "2026-10-21", "2026-12-31"];
    for date in bad {
        let in_override = policy_with_overrides(&override_text(|k, v| {
            Some(if k == "date" {
                date.to_owned()
            } else {
                v.to_owned()
            })
        }));
        let err = verdict_with(&in_override, &logged()).unwrap_err();
        assert!(err.contains("date"), "override {date}: {err}");
        let record = FULL_RECORD.replace("2026-10-21", date);
        let err = verdict(&changed_default("panic", &record)).unwrap_err();
        assert!(err.contains("date"), "default change {date}: {err}");
    }
    for date in good {
        let in_override = policy_with_overrides(&override_text(|k, v| {
            Some(if k == "date" {
                date.to_owned()
            } else {
                v.to_owned()
            })
        }));
        verdict_with(&in_override, &logged()).unwrap();
        let record = FULL_RECORD.replace("2026-10-21", date);
        verdict(&changed_default("panic", &record)).unwrap();
    }
}

#[test]
fn eqv_01_rejects_duplicate_sites() {
    let one = override_text(|_, v| Some(v.to_owned()));
    let twice = policy_with_overrides(&format!("{one}{one}"));
    let err = verdict_with(&twice, &logged()).unwrap_err();
    assert!(err.contains("duplicate") && err.contains("D-16"), "{err}");
    // A site differing in any one of the three strings is a different site.
    let other = override_text(|k, v| {
        Some(if k == "location" {
            "source/core_json.c:124".to_owned()
        } else {
            v.to_owned()
        })
    });
    let two_sites = policy_with_overrides(&format!("{one}{other}"));
    let mut sites = logged();
    sites.push(site(
        "JSON_Validate",
        "signed_overflow",
        "source/core_json.c:124",
    ));
    verdict_with(&two_sites, &sites).unwrap();
}

#[test]
fn eqv_01_override_naming_no_logged_site_is_rejected() {
    let text = policy_with_overrides(&override_text(|_, v| Some(v.to_owned())));
    // Phase 3 passes an empty list, so every override fails.
    let err = verdict(&text).unwrap_err();
    assert!(
        err.contains("JSON_Validate")
            && err.contains("signed_overflow")
            && err.contains("source/core_json.c:123")
            && err.contains("D-16"),
        "the issue must name the site and the decision: {err}"
    );
    // A log holding only a near miss does not help: identity is exact.
    for near in [
        site("JSON_Validate", "signed_overflow", "source/core_json.c:124"),
        site("JSON_Validate", "shift", "source/core_json.c:123"),
        site("JSON_Search", "signed_overflow", "source/core_json.c:123"),
    ] {
        assert!(verdict_with(&text, &[near]).is_err());
    }
}

#[test]
fn eqv_01_override_of_a_logged_site_is_accepted() {
    let text = policy_with_overrides(&override_text(|_, v| Some(v.to_owned())));
    let policy = verdict_with(&text, &logged()).unwrap();
    assert_eq!(policy.ub.r#override.len(), 1);
    assert_eq!(policy.ub.r#override[0].action, Action::Preserve);
    // The site list may hold other sites too.
    let mut more = vec![site("other", "shift", "x.c:1")];
    more.extend(logged());
    verdict_with(&text, &more).unwrap();
}

#[test]
fn eqv_01_empty_override_table_is_accepted() {
    let policy = verdict(BASE_WITH_UB).unwrap();
    assert!(policy.ub.r#override.is_empty());
    assert!(policy.ub.error_code.is_empty());
    assert!(policy.ub.default_change.is_none());
}

const BASE_WITH_UB: &str = "schema_version = 1\ncontract_version = \"0.1.0\"\n\
    module = \"corejson\"\n\n[ub]\ndefault_action = \"return_error\"\n";

fn error_codes(entries: &str) -> String {
    policy_with_overrides(entries)
}

#[test]
fn eqv_01_error_codes_are_checked() {
    let entry = |function: &str, value: &str| {
        format!("[[ub.error_code]]\nfunction = \"{function}\"\nvalue = {value}\n")
    };
    // Accepted: a decimal integer (as text or as a TOML integer) and a C
    // identifier naming a constant.
    for value in [
        "\"-1\"",
        "\"12\"",
        "\"JSONIllegalDocument\"",
        "\"_E1\"",
        "-1",
        "0",
    ] {
        let text = error_codes(&entry("JSON_Validate", value));
        let policy = verdict(&text).unwrap_or_else(|e| panic!("{value}: {e}"));
        assert_eq!(policy.ub.error_code.len(), 1);
    }
    // Refused values.
    for value in [
        "\"\"",
        "\"  \"",
        "\"1.5\"",
        "\"0x10\"",
        "\"bad value\"",
        "\"- 1\"",
        "\"1bad\"",
        "\"--1\"",
    ] {
        let err = verdict(&error_codes(&entry("JSON_Validate", value))).unwrap_err();
        assert!(err.contains("value"), "{value}: {err}");
    }
    // Refused functions: not a C identifier.
    for function in ["", "1bad", "has space", "a-b", "a.b", "a(b)"] {
        let err = verdict(&error_codes(&entry(function, "\"-1\""))).unwrap_err();
        assert!(err.contains("function"), "{function:?}: {err}");
    }
    // Two entries for one function.
    let twice = format!(
        "{}{}",
        entry("JSON_Validate", "\"-1\""),
        entry("JSON_Validate", "\"-2\"")
    );
    let err = verdict(&error_codes(&twice)).unwrap_err();
    assert!(
        err.contains("duplicate") && err.contains("JSON_Validate"),
        "{err}"
    );
    // Two functions are fine.
    let two = format!(
        "{}{}",
        entry("JSON_Validate", "\"-1\""),
        entry("JSON_Search", "\"-2\"")
    );
    verdict(&error_codes(&two)).unwrap();
}

#[test]
fn eqv_01_override_report_lists_every_override() {
    let a = override_text(|k, v| {
        Some(match k {
            "function" => "zeta".to_owned(),
            "reason" => "line one\\nline two".to_owned(),
            _ => v.to_owned(),
        })
    });
    let b = override_text(|k, v| {
        Some(if k == "function" {
            "alpha".to_owned()
        } else {
            v.to_owned()
        })
    });
    let c = override_text(|k, v| {
        Some(match k {
            "function" => "alpha".to_owned(),
            "kind" => "shift".to_owned(),
            "action" => "panic".to_owned(),
            _ => v.to_owned(),
        })
    });
    // Declared in the order a, b, c; reported by (function, kind, location).
    let text = with_ub(
        "",
        &format!("default_action = \"panic\"\n[ub.default_change]\n{FULL_RECORD}{a}{b}{c}"),
    );
    let policy = Policy::parse(&text).unwrap();
    let report = policy.override_report();
    assert_eq!(
        report,
        policy.override_report(),
        "the report is deterministic"
    );
    let lines: Vec<&str> = report.lines().collect();
    for line in &lines {
        assert!(line.contains('='), "one key=value field per line: {line:?}");
    }
    let at = |wanted: &str| {
        lines
            .iter()
            .position(|l| *l == wanted)
            .unwrap_or_else(|| panic!("{wanted:?} missing from:\n{report}"))
    };
    // The header: module, default, the default change.
    let order = [
        at("module=corejson"),
        at("default_action=panic"),
        at("default_change.module=corejson"),
        at("default_change.reason=r"),
        at("default_change.approver=a"),
        at("default_change.date=2026-10-21"),
        at("overrides=3"),
    ];
    assert!(order.windows(2).all(|w| w[0] < w[1]), "{report}");
    // Ordered by (function, kind, location): alpha/shift, alpha/signed_overflow, zeta.
    let first = at("override[0].function=alpha");
    assert!(first > order[6]);
    assert_eq!(lines[first + 1], "override[0].kind=shift");
    at("override[1].function=alpha");
    at("override[1].kind=signed_overflow");
    at("override[2].function=zeta");
    // All seven fields of every override.
    for i in 0..3 {
        for key in OVERRIDE_KEYS {
            let prefix = format!("override[{i}].{key}=");
            assert!(
                lines.iter().any(|l| l.starts_with(&prefix)),
                "{prefix} missing:\n{report}"
            );
        }
    }
    // A multi-line value stays on one line.
    at("override[2].reason=line one\\nline two");
    // Without a default change the report says so.
    let plain = Policy::parse(BASE_WITH_UB).unwrap().override_report();
    assert!(plain.lines().any(|l| l == "default_change=none"), "{plain}");
    assert!(plain.lines().any(|l| l == "overrides=0"), "{plain}");
}
