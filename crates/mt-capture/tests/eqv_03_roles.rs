#![allow(clippy::unwrap_used, clippy::expect_used)]
//! EQV-03: the argument roles of a manifest entry (CONTEXT D-11, D-14).
//!
//! Pure tests: the manifest text is the committed one with edits made in the
//! test. Entries without `arg` tables validate exactly as in Phase 2, and the
//! manifest schema version stays 1 (D-29).

use std::path::PathBuf;

use mt_capture::manifest::{
    ArgRole, MANIFEST_SCHEMA_VERSION, Manifest, ManifestError, Role, split_parameters,
};

fn repo_file(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn committed_text() -> String {
    std::fs::read_to_string(repo_file("testbed/manifest.toml")).unwrap()
}

fn committed() -> Manifest {
    Manifest::parse(&committed_text()).unwrap()
}

fn role(role: Role) -> ArgRole {
    ArgRole {
        role,
        name: None,
        default: None,
        sweep: Vec::new(),
        max: None,
    }
}

/// The committed manifest with the corejson `JSON_Validate` entry's `arg`
/// tables replaced by `roles`.
fn with_roles(roles: Vec<ArgRole>) -> Manifest {
    let mut manifest = committed();
    let target = manifest.target.get_mut("corejson").unwrap();
    let entry = target
        .entry
        .iter_mut()
        .find(|entry| entry.name == "JSON_Validate")
        .unwrap();
    entry.arg = roles;
    manifest
}

fn issues_of(manifest: &Manifest) -> Vec<String> {
    match manifest.validate() {
        Err(ManifestError::Invalid { issues }) => issues,
        other => panic!("the manifest must be invalid, got {other:?}"),
    }
}

#[test]
fn eqv_03_roles_accept_the_committed_manifest() {
    let manifest = committed();
    manifest.validate().unwrap();
    assert_eq!(manifest.schema_version, MANIFEST_SCHEMA_VERSION);
    assert_eq!(MANIFEST_SCHEMA_VERSION, 1, "D-29: no schema bump");
    let corejson = &manifest.target["corejson"];
    let validate = corejson
        .entry
        .iter()
        .find(|entry| entry.name == "JSON_Validate")
        .unwrap();
    let roles: Vec<Role> = validate.arg.iter().map(|arg| arg.role).collect();
    assert_eq!(
        roles,
        [Role::InBuf, Role::InLen],
        "corejson JSON_Validate declares in_buf then in_len (D-11)"
    );
    let search = corejson
        .entry
        .iter()
        .find(|entry| entry.name == "JSON_SearchT")
        .unwrap();
    assert!(
        search.arg.is_empty(),
        "an entry without arg tables stays without roles"
    );

    // The flat entries of plan 03-04 (D-11, D-14): roles in parameter order.
    let roles_of = |target: &str, name: &str| -> Vec<Role> {
        manifest.target[target]
            .entry
            .iter()
            .find(|entry| entry.name == name)
            .unwrap()
            .arg
            .iter()
            .map(|arg| arg.role)
            .collect()
    };
    assert_eq!(
        roles_of("libfdt", "fdt_check_full"),
        [Role::InBuf, Role::InLen]
    );
    assert_eq!(
        roles_of("lz4", "LZ4_decompress_safe"),
        [Role::InBuf, Role::OutBuf, Role::InLen, Role::OutCap]
    );
    let lz4 = manifest.target["lz4"]
        .entry
        .iter()
        .find(|entry| entry.name == "LZ4_decompress_safe")
        .unwrap();
    let cap = &lz4.arg[3];
    assert_eq!(cap.default, Some(256));
    assert_eq!(cap.sweep, [16, 4096]);
}

#[test]
fn eqv_03_roles_reject_an_unknown_role() {
    let marker = "signature = \"JSONStatus_t (const char *, size_t)\"\n";
    let text = committed_text();
    assert!(text.contains(marker), "the marker line is in the manifest");
    let edited = text.replacen(
        marker,
        &format!("{marker}\n[[target.corejson.entry.arg]]\nrole = \"magic\"\n"),
        1,
    );
    let error = Manifest::parse(&edited).unwrap_err();
    let text = error.to_string();
    assert!(
        text.contains("magic"),
        "the unknown role is named in the error: {text}"
    );
}

#[test]
fn eqv_03_roles_reject_a_wrong_parameter_count() {
    let manifest = with_roles(vec![role(Role::InBuf)]);
    let issues = issues_of(&manifest);
    let issue = issues
        .iter()
        .find(|issue| issue.contains("JSON_Validate"))
        .unwrap_or_else(|| panic!("an issue names the entry: {issues:?}"));
    assert!(issue.contains("1 arg tables"), "{issue}");
    assert!(issue.contains("2 parameters"), "{issue}");
    assert!(issue.contains("D-11"), "{issue}");
}

#[test]
fn eqv_03_roles_reject_a_pointer_role_on_a_value() {
    // The second parameter is a size_t, not a pointer.
    let manifest = with_roles(vec![role(Role::InBuf), role(Role::InBuf)]);
    let issues = issues_of(&manifest);
    assert!(
        issues
            .iter()
            .any(|issue| issue.contains("JSON_Validate") && issue.contains("pointer")),
        "{issues:?}"
    );
}

#[test]
fn eqv_03_roles_reject_a_missing_length() {
    let mut scalar = role(Role::Scalar);
    scalar.name = Some("n".to_owned());
    scalar.default = Some(1);
    let manifest = with_roles(vec![role(Role::InBuf), scalar]);
    let issues = issues_of(&manifest);
    assert!(
        issues
            .iter()
            .any(|issue| issue.contains("JSON_Validate") && issue.contains("in_len")),
        "{issues:?}"
    );
}

#[test]
fn eqv_03_split_parameters() {
    let (ret, params) = split_parameters("int (const char *, size_t)").unwrap();
    assert_eq!(ret, "int");
    assert_eq!(params, ["const char *", "size_t"]);

    let (ret, params) = split_parameters(
        "void (zcbor_state_t *, size_t, const uint8_t *, size_t, size_t, uint8_t *, size_t)",
    )
    .unwrap();
    assert_eq!(ret, "void");
    assert_eq!(
        params,
        [
            "zcbor_state_t *",
            "size_t",
            "const uint8_t *",
            "size_t",
            "size_t",
            "uint8_t *",
            "size_t"
        ]
    );

    let (ret, params) = split_parameters("int (void)").unwrap();
    assert_eq!(ret, "int");
    assert!(params.is_empty(), "(void) is zero parameters");

    let (ret, params) = split_parameters("cJSON *(const char *, size_t)").unwrap();
    assert_eq!(ret, "cJSON *");
    assert_eq!(params, ["const char *", "size_t"]);

    // A function-pointer parameter keeps its own commas and parentheses.
    let (ret, params) = split_parameters("int (int (*)(int, int), void *)").unwrap();
    assert_eq!(ret, "int");
    assert_eq!(params, ["int (*)(int, int)", "void *"]);

    // Whitespace runs collapse, and text that is not a function type is refused.
    let (_, params) = split_parameters("int (  const char  *,   size_t )").unwrap();
    assert_eq!(params, ["const char *", "size_t"]);
    assert!(split_parameters("int").is_err());
    assert!(split_parameters("int (const char *").is_err());
}

/// The committed manifest with the corejson `JSON_Validate` entry given
/// `signature` and `roles`.
fn with_entry(signature: &str, roles: Vec<ArgRole>) -> Manifest {
    let mut manifest = with_roles(roles);
    let target = manifest.target.get_mut("corejson").unwrap();
    let entry = target
        .entry
        .iter_mut()
        .find(|entry| entry.name == "JSON_Validate")
        .unwrap();
    entry.signature = signature.to_owned();
    manifest
}

fn scalar(name: &str, default: Option<i64>, sweep: &[i64]) -> ArgRole {
    ArgRole {
        role: Role::Scalar,
        name: Some(name.to_owned()),
        default,
        sweep: sweep.to_vec(),
        max: None,
    }
}

fn has_issue(manifest: &Manifest, words: &[&str]) -> bool {
    issues_of(manifest)
        .iter()
        .any(|issue| issue.contains("JSON_Validate") && words.iter().all(|w| issue.contains(w)))
}

#[test]
fn eqv_03_roles_reject_a_missing_bound_on_a_narrow_length() {
    for narrow in [
        "uint8_t",
        "int8_t",
        "unsigned char",
        "char",
        "signed char",
        "uint16_t",
        "int16_t",
        "unsigned short",
        "short",
    ] {
        let signature = format!("JSONStatus_t (const char *, {narrow})");
        let manifest = with_entry(&signature, vec![role(Role::InBuf), role(Role::InLen)]);
        assert!(
            has_issue(&manifest, &["max", narrow]),
            "{narrow}: {:?}",
            issues_of(&manifest)
        );
        // With a bound the declaration passes.
        let mut bounded = role(Role::InLen);
        bounded.max = Some(255);
        with_entry(&signature, vec![role(Role::InBuf), bounded])
            .validate()
            .unwrap();
    }
    // A 32-bit length needs no bound, and may carry one.
    let mut bounded = role(Role::InLen);
    bounded.max = Some(1000);
    with_roles(vec![role(Role::InBuf), bounded])
        .validate()
        .unwrap();
    // A bound on any other role is refused.
    let mut buf = role(Role::InBuf);
    buf.max = Some(4);
    assert!(has_issue(
        &with_roles(vec![buf, role(Role::InLen)]),
        &["max"]
    ));
}

#[test]
fn eqv_03_roles_reject_a_scalar_without_default() {
    let signature = "JSONStatus_t (const char *, size_t, int)";
    let ok = with_entry(
        signature,
        vec![
            role(Role::InBuf),
            role(Role::InLen),
            scalar("n", Some(3), &[]),
        ],
    );
    ok.validate().unwrap();
    let no_default = with_entry(
        signature,
        vec![role(Role::InBuf), role(Role::InLen), scalar("n", None, &[])],
    );
    assert!(has_issue(&no_default, &["default"]));
    let mut unnamed = scalar("n", Some(3), &[]);
    unnamed.name = None;
    let no_name = with_entry(
        signature,
        vec![role(Role::InBuf), role(Role::InLen), unnamed],
    );
    assert!(has_issue(&no_name, &["name"]));
    // A name that is not a C identifier would reach the generated C and the
    // provenance; it is refused here.
    for bad in ["a b", "1x", "x;y", ""] {
        let bad_name = with_entry(
            signature,
            vec![
                role(Role::InBuf),
                role(Role::InLen),
                scalar(bad, Some(3), &[]),
            ],
        );
        assert!(has_issue(&bad_name, &["identifier"]), "{bad:?}");
    }
    // A buffer role takes no name.
    let mut named = role(Role::InBuf);
    named.name = Some("data".to_owned());
    assert!(has_issue(
        &with_roles(vec![named, role(Role::InLen)]),
        &["name"]
    ));
}

#[test]
fn eqv_03_roles_reject_reserved_and_duplicate_names() {
    let signature = "JSONStatus_t (const char *, size_t, int, int)";
    for reserved in ["in", "out", "globals", "heap", "ret"] {
        let manifest = with_entry(
            signature,
            vec![
                role(Role::InBuf),
                role(Role::InLen),
                scalar(reserved, Some(1), &[]),
                scalar("other", Some(1), &[]),
            ],
        );
        assert!(has_issue(&manifest, &["reserved", reserved]), "{reserved}");
    }
    let duplicate = with_entry(
        signature,
        vec![
            role(Role::InBuf),
            role(Role::InLen),
            scalar("n", Some(1), &[]),
            scalar("n", Some(2), &[]),
        ],
    );
    assert!(has_issue(&duplicate, &["duplicate", "n"]));
    // The out_cap role is recorded as out_cap unless it is named, so a scalar
    // of that name collides with it.
    let signature = "int (const char *, char *, size_t, size_t, int)";
    let mut cap = role(Role::OutCap);
    cap.default = Some(8);
    let collides = with_entry(
        signature,
        vec![
            role(Role::InBuf),
            role(Role::OutBuf),
            role(Role::InLen),
            cap,
            scalar("out_cap", Some(1), &[]),
        ],
    );
    assert!(has_issue(&collides, &["duplicate", "out_cap"]));
    // A sweep may not repeat the default or itself.
    let repeats = with_entry(
        "JSONStatus_t (const char *, size_t, int)",
        vec![
            role(Role::InBuf),
            role(Role::InLen),
            scalar("n", Some(1), &[1]),
        ],
    );
    assert!(has_issue(&repeats, &["sweep"]));
}

#[test]
fn eqv_03_roles_reject_too_many_combinations() {
    let signature = "JSONStatus_t (const char *, size_t, int)";
    let roles = |sweep: &[i64]| {
        vec![
            role(Role::InBuf),
            role(Role::InLen),
            scalar("n", Some(0), sweep),
        ]
    };
    // default + 15 = 16 combinations pass; default + 16 = 17 fail.
    let fifteen: Vec<i64> = (1..=15).collect();
    with_entry(signature, roles(&fifteen)).validate().unwrap();
    let sixteen: Vec<i64> = (1..=16).collect();
    let manifest = with_entry(signature, roles(&sixteen));
    assert!(has_issue(&manifest, &["17", "16", "combinations"]));
    // Two dimensions multiply: 4 x 5 = 20.
    let signature = "JSONStatus_t (const char *, size_t, int, int)";
    let manifest = with_entry(
        signature,
        vec![
            role(Role::InBuf),
            role(Role::InLen),
            scalar("a", Some(0), &[1, 2, 3]),
            scalar("b", Some(0), &[1, 2, 3, 4]),
        ],
    );
    assert!(has_issue(&manifest, &["20", "combinations"]));
}

#[test]
fn eqv_03_roles_reject_out_buf_without_capacity() {
    let signature = "int (const char *, char *, size_t)";
    let no_cap = with_entry(
        signature,
        vec![role(Role::InBuf), role(Role::OutBuf), role(Role::InLen)],
    );
    assert!(has_issue(&no_cap, &["out_buf", "out_cap"]));
    // A capacity without a buffer is the same mistake from the other side.
    let signature = "int (const char *, size_t, size_t)";
    let mut cap = role(Role::OutCap);
    cap.default = Some(8);
    let no_buf = with_entry(
        signature,
        vec![role(Role::InBuf), role(Role::InLen), cap.clone()],
    );
    assert!(has_issue(&no_buf, &["out_cap", "out_buf"]));
    // The capacity needs a default, and a value inside the executor's output
    // limit, so that every combination can run.
    let signature = "int (const char *, char *, size_t, size_t)";
    let mut no_default = cap.clone();
    no_default.default = None;
    let manifest = with_entry(
        signature,
        vec![
            role(Role::InBuf),
            role(Role::OutBuf),
            role(Role::InLen),
            no_default,
        ],
    );
    assert!(has_issue(&manifest, &["out_cap", "default"]));
    let mut too_big = cap.clone();
    too_big.sweep = vec![65_537];
    let manifest = with_entry(
        signature,
        vec![
            role(Role::InBuf),
            role(Role::OutBuf),
            role(Role::InLen),
            too_big,
        ],
    );
    assert!(has_issue(&manifest, &["out_cap", "65536"]));
    // The complete declaration passes.
    with_entry(
        signature,
        vec![
            role(Role::InBuf),
            role(Role::OutBuf),
            role(Role::InLen),
            cap,
        ],
    )
    .validate()
    .unwrap();
}
