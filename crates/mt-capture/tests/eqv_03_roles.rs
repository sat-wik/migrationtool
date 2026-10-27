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
