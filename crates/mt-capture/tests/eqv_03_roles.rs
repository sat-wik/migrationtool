#![allow(clippy::unwrap_used, clippy::expect_used)]
//! EQV-03: the argument roles of a manifest entry (CONTEXT D-11, D-14).
//!
//! Pure tests: the manifest text is the committed one with edits made in the
//! test. Entries without `arg` tables validate exactly as in Phase 2, and the
//! manifest schema version stays 1 (D-29).

use std::path::PathBuf;

use mt_capture::manifest::{
    ArgRole, LocalObject, MANIFEST_SCHEMA_VERSION, Manifest, ManifestError, Role, SetValue,
    SetupArg, SetupStep, TYPE_TEXT_GRAMMAR, UnobservedMark, is_closed_type_text, split_parameters,
    state_pointee_type,
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
        of: None,
        mutable: false,
        values: Vec::new(),
        count: None,
        ptr_field: None,
        len_field: None,
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
    // JSON_SearchT has roles since plan 03-06; an entry the harness cannot drive
    // yet (nanopb's pb_decode_ex) stays without arg tables.
    let search = corejson
        .entry
        .iter()
        .find(|entry| entry.name == "JSON_SearchT")
        .unwrap();
    assert_eq!(search.arg.len(), 7);
    let undriven = manifest.target["nanopb"]
        .entry
        .iter()
        .find(|entry| entry.name == "pb_decode_ex")
        .unwrap();
    assert!(
        undriven.arg.is_empty(),
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
        of: None,
        mutable: false,
        values: Vec::new(),
        count: None,
        ptr_field: None,
        len_field: None,
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

fn state(name: Option<&str>) -> ArgRole {
    ArgRole {
        role: Role::State,
        name: name.map(str::to_owned),
        default: None,
        sweep: Vec::new(),
        max: None,
        of: None,
        mutable: false,
        values: Vec::new(),
        count: None,
        ptr_field: None,
        len_field: None,
    }
}

#[test]
fn eqv_03_roles_state_needs_a_pointer_and_a_name() {
    // D-09, D-11: a `state` is a named pointer to a struct, union or enum type.
    let signature = "int (coap_packet_t *, const uint8_t *, size_t)";
    let roles = |first: ArgRole| vec![first, role(Role::InBuf), role(Role::InLen)];
    with_entry(signature, roles(state(Some("pkt"))))
        .validate()
        .unwrap();

    // A name is required, and it is a C identifier.
    let nameless = with_entry(signature, roles(state(None)));
    assert!(has_issue(&nameless, &["state", "name"]));
    let bad_name = with_entry(signature, roles(state(Some("p kt"))));
    assert!(has_issue(&bad_name, &["identifier"]));

    // It needs a pointer parameter.
    let value = with_entry(
        "int (int, const uint8_t *, size_t)",
        roles(state(Some("pkt"))),
    );
    assert!(has_issue(&value, &["state", "pointer"]));

    // It takes no default, sweep or max.
    let mut with_default = state(Some("pkt"));
    with_default.default = Some(1);
    assert!(has_issue(
        &with_entry(signature, roles(with_default)),
        &["default", "state"]
    ));
    let mut with_max = state(Some("pkt"));
    with_max.max = Some(4);
    assert!(has_issue(&with_entry(signature, roles(with_max)), &["max"]));

    // The pointee must be a plain type name before it is written into C.
    for bad in [
        "void *",
        "coap_packet_t **",
        "unsigned char *",
        "int (*)(void) *",
    ] {
        let manifest = with_entry(
            &format!("int ({bad}, const uint8_t *, size_t)"),
            roles(state(Some("pkt"))),
        );
        assert!(has_issue(&manifest, &["state", "pointee"]), "{bad}");
    }
    // A const qualifier and a tag keyword are accepted.
    for good in [
        "const coap_packet_t *",
        "struct item *",
        "union u *",
        "enum e *",
    ] {
        with_entry(
            &format!("int ({good}, const uint8_t *, size_t)"),
            roles(state(Some("pkt"))),
        )
        .validate()
        .unwrap();
    }

    // The name is not a reserved one and is not repeated.
    let reserved = with_entry(signature, roles(state(Some("in"))));
    assert!(has_issue(&reserved, &["reserved"]));
    let twice = with_entry(
        "int (coap_packet_t *, const uint8_t *, size_t, coap_packet_t *)",
        vec![
            state(Some("pkt")),
            role(Role::InBuf),
            role(Role::InLen),
            state(Some("pkt")),
        ],
    );
    assert!(has_issue(&twice, &["duplicate"]));
}

#[test]
fn eqv_03_state_pointee_type() {
    assert_eq!(
        state_pointee_type("coap_packet_t *").as_deref(),
        Some("coap_packet_t")
    );
    assert_eq!(
        state_pointee_type("const struct foo *").as_deref(),
        Some("struct foo")
    );
    assert_eq!(state_pointee_type("union u*").as_deref(), Some("union u"));
    assert_eq!(state_pointee_type("T * const").as_deref(), Some("T"));
    for bad in [
        "T **",
        "void *",
        "unsigned char *",
        "int",
        "T",
        "struct *",
        "1T *",
    ] {
        assert_eq!(state_pointee_type(bad), None, "{bad}");
    }
}

fn mark(field: &str, reason: &str) -> UnobservedMark {
    UnobservedMark {
        field: field.to_owned(),
        reason: reason.to_owned(),
    }
}

/// The corejson entry as a packet parser with a state called `pkt` and `marks`.
fn with_marks(marks: Vec<UnobservedMark>) -> Manifest {
    let signature = "int (coap_packet_t *, const uint8_t *, size_t)";
    let mut manifest = with_entry(
        signature,
        vec![state(Some("pkt")), role(Role::InBuf), role(Role::InLen)],
    );
    let target = manifest.target.get_mut("corejson").unwrap();
    let entry = target
        .entry
        .iter_mut()
        .find(|entry| entry.name == "JSON_Validate")
        .unwrap();
    entry.unobserved = marks;
    manifest
}

#[test]
fn eqv_03_unobserved_marks_need_a_reason_and_a_declared_state() {
    // D-09: a field may be marked unobserved only with a recorded reason.
    with_marks(vec![
        mark("pkt.payload", "points at a buffer the caller owns"),
        mark("pkt.opts[0].buf.p", "internal"),
        mark("pkt[3]", "an index straight after the state name"),
    ])
    .validate()
    .unwrap();
    with_marks(Vec::new()).validate().unwrap();

    for reason in ["", "   ", "\t"] {
        let manifest = with_marks(vec![mark("pkt.payload", reason)]);
        assert!(
            has_issue(&manifest, &["unobserved", "reason"]),
            "reason {reason:?}"
        );
    }
    // The state must be declared by a `state` role of the same entry.
    let manifest = with_marks(vec![mark("other.payload", "r")]);
    assert!(has_issue(&manifest, &["unobserved", "declared"]));
    // The path is `<state>(.<member>|[<index>])+` in C identifiers and decimals.
    for bad in [
        "pkt",
        "pkt.",
        "pkt..x",
        "pkt.pay load",
        "pkt.1x",
        "pkt.a-b",
        "pkt[x]",
        "pkt[]",
        "pkt[0",
        "pkt[-1]",
        "pkt.x[01a]",
        ".pkt.x",
        "",
    ] {
        let manifest = with_marks(vec![mark(bad, "r")]);
        assert!(
            has_issue(&manifest, &["unobserved", "field"]),
            "field {bad:?} must be refused"
        );
    }
    // The same field is marked once.
    let manifest = with_marks(vec![mark("pkt.payload", "a"), mark("pkt.payload", "b")]);
    assert!(has_issue(&manifest, &["unobserved", "duplicate"]));
    // An entry with marks but no state has nothing to mark.
    let mut stateless = committed();
    let target = stateless.target.get_mut("corejson").unwrap();
    let entry = target
        .entry
        .iter_mut()
        .find(|entry| entry.name == "JSON_Validate")
        .unwrap();
    entry.unobserved = vec![mark("pkt.payload", "r")];
    assert!(has_issue(&stateless, &["unobserved", "declared"]));
}

#[test]
fn eqv_03_unobserved_marks_parse_from_toml_and_stay_absent_otherwise() {
    let text = committed_text();
    let marker = "[[target.microcoap.config]]";
    let edited = text.replacen(
        marker,
        "[[target.microcoap.entry.unobserved]]\nfield = \"pkt.payload\"\nreason = \"the payload is checked through the input\"\n\n[[target.microcoap.config]]",
        1,
    );
    assert_ne!(edited, text);
    let manifest = Manifest::parse(&edited).unwrap();
    manifest.validate().unwrap();
    let entry = &manifest.target["microcoap"].entry[0];
    assert_eq!(
        entry.unobserved,
        [mark(
            "pkt.payload",
            "the payload is checked through the input"
        )]
    );
    // Entries without marks serialize without the key, as before.
    let plain = serde_json::to_value(&committed().target["corejson"].entry[0]).unwrap();
    assert!(plain.get("unobserved").is_none(), "{plain}");
    // An unknown key in a mark is refused.
    let unknown = edited.replacen("reason = ", "why = ", 1);
    assert!(Manifest::parse(&unknown).is_err());
}

fn inout_len(name: Option<&str>, default: Option<i64>, sweep: &[i64], of: Option<&str>) -> ArgRole {
    ArgRole {
        role: Role::InoutLen,
        name: name.map(str::to_owned),
        default,
        sweep: sweep.to_vec(),
        max: None,
        of: of.map(str::to_owned),
        mutable: false,
        values: Vec::new(),
        count: None,
        ptr_field: None,
        len_field: None,
    }
}

#[test]
fn eqv_03_roles_inout_len_may_size_the_output() {
    // D-11, D-14: zlib's uncompress takes the capacity of its output buffer
    // through a pointer to a length; `of = "out"` makes that length the capacity.
    let signature = "int (Bytef *, uLongf *, const Bytef *, uLong)";
    let roles = |len: ArgRole| {
        vec![
            role(Role::OutBuf),
            len,
            role(Role::InBuf),
            role(Role::InLen),
        ]
    };
    let sizing = inout_len(Some("destLen"), Some(256), &[16, 4096], Some("out"));
    with_entry(signature, roles(sizing.clone()))
        .validate()
        .unwrap();

    // The length is the capacity: an out_cap as well is a second capacity.
    let both = with_entry(
        "int (Bytef *, uLongf *, const Bytef *, uLong, uLong)",
        vec![
            role(Role::OutBuf),
            sizing.clone(),
            role(Role::InBuf),
            role(Role::InLen),
            {
                let mut cap = role(Role::OutCap);
                cap.default = Some(8);
                cap
            },
        ],
    );
    assert!(
        has_issue(&both, &["inout_len", "out_cap"]),
        "{:?}",
        issues_of(&both)
    );

    // `of` names the output buffer or nothing.
    let nothing = with_entry(
        signature,
        roles(inout_len(
            Some("destLen"),
            Some(256),
            &[],
            Some("elsewhere"),
        )),
    );
    assert!(
        has_issue(&nothing, &["of", "elsewhere"]),
        "{:?}",
        issues_of(&nothing)
    );

    // `of = "out"` needs an out_buf in the entry.
    let no_buffer = with_entry(
        "int (const Bytef *, uLongf *, uLong)",
        vec![
            role(Role::InBuf),
            inout_len(Some("destLen"), Some(256), &[], Some("out")),
            role(Role::InLen),
        ],
    );
    assert!(
        has_issue(&no_buffer, &["of", "out_buf"]),
        "{:?}",
        issues_of(&no_buffer)
    );

    // It is a pointer to a value, with a name and a default.
    let value = with_entry(
        "int (Bytef *, uLong, const Bytef *, uLong)",
        roles(sizing.clone()),
    );
    assert!(
        has_issue(&value, &["inout_len", "pointer"]),
        "{:?}",
        issues_of(&value)
    );
    let nameless = with_entry(
        signature,
        roles(inout_len(None, Some(256), &[], Some("out"))),
    );
    assert!(has_issue(&nameless, &["inout_len", "name"]));
    let no_default = with_entry(
        signature,
        roles(inout_len(Some("destLen"), None, &[], Some("out"))),
    );
    assert!(has_issue(&no_default, &["inout_len", "default"]));

    // Its values are capacities, so they lie within 0..=MAX_OUT_CAP, and the
    // name is not reserved or repeated.
    let huge = with_entry(
        signature,
        roles(inout_len(
            Some("destLen"),
            Some(1_000_000),
            &[],
            Some("out"),
        )),
    );
    assert!(has_issue(&huge, &["inout_len", "65536"]));
    let reserved = with_entry(
        signature,
        roles(inout_len(Some("heap"), Some(256), &[], Some("out"))),
    );
    assert!(has_issue(&reserved, &["reserved"]));

    // The values of the length join the combination count.
    let many = with_entry(
        signature,
        roles(inout_len(
            Some("destLen"),
            Some(1),
            &[2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17],
            Some("out"),
        )),
    );
    assert!(has_issue(&many, &["combinations"]));
}

#[test]
fn eqv_03_roles_inout_len_parses_from_toml() {
    let arg: ArgRole = toml::from_str(
        "role = \"inout_len\"\nname = \"destLen\"\ndefault = 256\nsweep = [16, 4096]\nof = \"out\"\n",
    )
    .unwrap();
    assert_eq!(arg.role, Role::InoutLen);
    assert_eq!(arg.name.as_deref(), Some("destLen"));
    assert_eq!(arg.of.as_deref(), Some("out"));
    assert_eq!(arg.sweep, [16, 4096]);
    // `of` is absent from the serialised form when it is not set.
    let plain: ArgRole = toml::from_str("role = \"in_buf\"\n").unwrap();
    assert_eq!(plain.of, None);
    assert!(!toml::to_string(&plain).unwrap().contains("of"));
}

fn str_role(name: Option<&str>, values: &[&str]) -> ArgRole {
    ArgRole {
        role: Role::Str,
        name: name.map(str::to_owned),
        default: None,
        sweep: Vec::new(),
        max: None,
        of: None,
        mutable: false,
        values: values.iter().map(|value| (*value).to_owned()).collect(),
        count: None,
        ptr_field: None,
        len_field: None,
    }
}

fn str_len(of: Option<&str>) -> ArgRole {
    let mut role = role(Role::StrLen);
    role.of = of.map(str::to_owned);
    role
}

const SEARCH_SIGNATURE: &str = "JSONStatus_t (const char *, size_t, const char *, size_t)";

fn search_roles(query: ArgRole, length: ArgRole) -> Manifest {
    with_entry(
        SEARCH_SIGNATURE,
        vec![role(Role::InBuf), role(Role::InLen), query, length],
    )
}

#[test]
fn eqv_03_roles_str_values_are_printable_and_bounded() {
    // D-11, T-03-24: a str is a declared list of printable ASCII values, passed
    // as data; its length is derived.
    search_roles(
        str_role(Some("query"), &["foo", "hello", "[0]", " "]),
        str_len(Some("query")),
    )
    .validate()
    .unwrap();

    let bad =
        |values: &[&str]| search_roles(str_role(Some("query"), values), str_len(Some("query")));
    // A control character, a byte above 0x7E and a 65-byte value are refused.
    for value in ["a\tb", "a\nb", "a\u{7f}", "caf\u{e9}", "a\u{0}b"] {
        let manifest = bad(&[value]);
        assert!(
            has_issue(&manifest, &["str", "printable"]),
            "{value:?}: {:?}",
            issues_of(&manifest)
        );
    }
    let long = "x".repeat(65);
    let manifest = bad(&[long.as_str()]);
    assert!(
        has_issue(&manifest, &["str", "64"]),
        "{:?}",
        issues_of(&manifest)
    );
    let exact = "x".repeat(64);
    bad(&[exact.as_str()]).validate().unwrap();
    // The list has one to eight values and no repeat.
    let manifest = bad(&[]);
    assert!(
        has_issue(&manifest, &["str", "values"]),
        "{:?}",
        issues_of(&manifest)
    );
    let nine = ["a", "b", "c", "d", "e", "f", "g", "h", "i"];
    let manifest = bad(&nine);
    assert!(
        has_issue(&manifest, &["str", "8"]),
        "{:?}",
        issues_of(&manifest)
    );
    bad(&nine[..8]).validate().unwrap();
    let manifest = bad(&["a", "a"]);
    assert!(
        has_issue(&manifest, &["str", "repeat"]),
        "{:?}",
        issues_of(&manifest)
    );
    // It needs a name, and values belong to str alone.
    let nameless = search_roles(str_role(None, &["a"]), str_len(Some("query")));
    assert!(has_issue(&nameless, &["str", "name"]));
    let mut stray = role(Role::InLen);
    stray.values = vec!["a".to_owned()];
    let manifest = with_roles(vec![role(Role::InBuf), stray]);
    assert!(
        has_issue(&manifest, &["values", "str"]),
        "{:?}",
        issues_of(&manifest)
    );

    // str_len names its str with `of`, and has no other key.
    let manifest = search_roles(str_role(Some("query"), &["a"]), str_len(None));
    assert!(
        has_issue(&manifest, &["str_len", "of"]),
        "{:?}",
        issues_of(&manifest)
    );
    let manifest = search_roles(str_role(Some("query"), &["a"]), str_len(Some("other")));
    assert!(
        has_issue(&manifest, &["str_len", "other"]),
        "{:?}",
        issues_of(&manifest)
    );
    let mut with_default = str_len(Some("query"));
    with_default.default = Some(1);
    let manifest = search_roles(str_role(Some("query"), &["a"]), with_default);
    assert!(has_issue(&manifest, &["default"]));
    // A str_len is a value parameter.
    let manifest = with_entry(
        "JSONStatus_t (const char *, size_t, const char *, size_t *)",
        vec![
            role(Role::InBuf),
            role(Role::InLen),
            str_role(Some("query"), &["a"]),
            str_len(Some("query")),
        ],
    );
    assert!(
        has_issue(&manifest, &["str_len", "value"]),
        "{:?}",
        issues_of(&manifest)
    );
    // A str is a pointer parameter.
    let manifest = with_entry(
        "JSONStatus_t (const char *, size_t, int, size_t)",
        vec![
            role(Role::InBuf),
            role(Role::InLen),
            str_role(Some("query"), &["a"]),
            str_len(Some("query")),
        ],
    );
    assert!(
        has_issue(&manifest, &["str", "pointer"]),
        "{:?}",
        issues_of(&manifest)
    );
    // The values are a combination dimension: nine str values of three strs
    // exceed the cap.
    let manifest = with_entry(
        "JSONStatus_t (const char *, size_t, const char *, const char *, const char *)",
        vec![
            role(Role::InBuf),
            role(Role::InLen),
            str_role(Some("a"), &["1", "2", "3", "4"]),
            str_role(Some("b"), &["1", "2", "3", "4"]),
            str_role(Some("c"), &["1", "2"]),
        ],
    );
    assert!(
        has_issue(&manifest, &["combinations"]),
        "{:?}",
        issues_of(&manifest)
    );
}

#[test]
fn eqv_03_roles_mutable_input_needs_a_non_const_pointer() {
    // D-11: `mutable` records the input's bytes after the call, so the C code
    // must be allowed to write them.
    let mut buf = role(Role::InBuf);
    buf.mutable = true;
    with_entry(
        "JSONStatus_t (char *, size_t)",
        vec![buf.clone(), role(Role::InLen)],
    )
    .validate()
    .unwrap();
    let manifest = with_entry(
        "JSONStatus_t (const char *, size_t)",
        vec![buf.clone(), role(Role::InLen)],
    );
    assert!(
        has_issue(&manifest, &["mutable", "const"]),
        "{:?}",
        issues_of(&manifest)
    );
    // Only an in_buf is mutable.
    let mut len = role(Role::InLen);
    len.mutable = true;
    let manifest = with_roles(vec![role(Role::InBuf), len]);
    assert!(
        has_issue(&manifest, &["mutable", "in_buf"]),
        "{:?}",
        issues_of(&manifest)
    );
    let mut out = role(Role::OutBuf);
    out.mutable = true;
    let manifest = with_entry(
        "JSONStatus_t (char *, size_t, size_t)",
        vec![out, role(Role::InLen), {
            let mut cap = role(Role::OutCap);
            cap.default = Some(4);
            cap
        }],
    );
    assert!(has_issue(&manifest, &["mutable", "in_buf"]));
}

fn out_role(role: Role, name: Option<&str>) -> ArgRole {
    let mut arg = self::role(role);
    arg.name = name.map(str::to_owned);
    arg
}

#[test]
fn eqv_03_roles_out_roles_need_pointers_and_names() {
    // D-11: out_scalar is a pointer to a scalar, out_ptr a pointer to a pointer;
    // both are recorded under a name.
    let signature = "JSONStatus_t (const char *, size_t, char **, size_t *, JSONTypes_t *)";
    let roles = |ptr: ArgRole, scalar: ArgRole, kind: ArgRole| {
        vec![role(Role::InBuf), role(Role::InLen), ptr, scalar, kind]
    };
    let good = || {
        roles(
            out_role(Role::OutPtr, Some("outValue")),
            out_role(Role::OutScalar, Some("outValueLength")),
            out_role(Role::OutScalar, Some("outType")),
        )
    };
    with_entry(signature, good()).validate().unwrap();

    // Names: required, C identifiers, not reserved, not repeated.
    for (role_word, which) in [("out_ptr", 2usize), ("out_scalar", 3)] {
        let mut roles = good();
        roles[which].name = None;
        let manifest = with_entry(signature, roles);
        assert!(
            has_issue(&manifest, &[role_word, "name"]),
            "{role_word}: {:?}",
            issues_of(&manifest)
        );
    }
    let mut roles = good();
    roles[3].name = Some("outValue".to_owned());
    assert!(has_issue(&with_entry(signature, roles), &["duplicate"]));
    let mut roles = good();
    roles[3].name = Some("heap".to_owned());
    assert!(has_issue(&with_entry(signature, roles), &["reserved"]));
    let mut roles = good();
    roles[3].name = Some("a b".to_owned());
    assert!(has_issue(&with_entry(signature, roles), &["identifier"]));

    // out_scalar is a pointer to a plain value; out_ptr a pointer to a pointer.
    let manifest = with_entry(
        "JSONStatus_t (const char *, size_t, char *, size_t *, JSONTypes_t *)",
        good(),
    );
    assert!(
        has_issue(&manifest, &["out_ptr", "pointer to a pointer"]),
        "{:?}",
        issues_of(&manifest)
    );
    let manifest = with_entry(
        "JSONStatus_t (const char *, size_t, char **, size_t, JSONTypes_t *)",
        good(),
    );
    assert!(
        has_issue(&manifest, &["out_scalar", "pointer"]),
        "{:?}",
        issues_of(&manifest)
    );
    let manifest = with_entry(
        "JSONStatus_t (const char *, size_t, char **, size_t **, JSONTypes_t *)",
        good(),
    );
    assert!(
        has_issue(&manifest, &["out_scalar", "plain"]),
        "{:?}",
        issues_of(&manifest)
    );
    let manifest = with_entry(
        "JSONStatus_t (const char *, size_t, char **, const size_t *, JSONTypes_t *)",
        good(),
    );
    assert!(
        has_issue(&manifest, &["out_scalar", "const"]),
        "{:?}",
        issues_of(&manifest)
    );

    // Neither takes a default, sweep, max, values, of or mutable.
    let mut roles = good();
    roles[3].default = Some(1);
    assert!(has_issue(
        &with_entry(signature, roles),
        &["default", "out_scalar"]
    ));
    let mut roles = good();
    roles[2].sweep = vec![1];
    assert!(has_issue(
        &with_entry(signature, roles),
        &["sweep", "out_ptr"]
    ));
    let mut roles = good();
    roles[3].values = vec!["a".to_owned()];
    assert!(has_issue(&with_entry(signature, roles), &["values", "str"]));
    let mut roles = good();
    roles[3].of = Some("out".to_owned());
    assert!(has_issue(&with_entry(signature, roles), &["of"]));
}

// ---------------------------------------------------------------------------
// Plan 03-07: declared setup sequences and local objects (D-12)
// ---------------------------------------------------------------------------

const HTTP_SIGNATURE: &str =
    "size_t (http_parser *, const http_parser_settings *, const char *, size_t)";

fn call_step(call: &str, args: &[&str], expect: Option<&str>) -> SetupStep {
    SetupStep {
        call: Some(call.to_owned()),
        args: args.iter().map(|arg| (*arg).to_owned()).collect(),
        expect: expect.map(str::to_owned),
        set: None,
        value: None,
    }
}

fn set_step(path: &str, value: Option<&str>) -> SetupStep {
    SetupStep {
        call: None,
        args: Vec::new(),
        expect: None,
        set: Some(path.to_owned()),
        value: value.map(str::to_owned),
    }
}

fn local(name: &str, type_text: &str, count: Option<u32>) -> LocalObject {
    LocalObject {
        name: name.to_owned(),
        type_text: type_text.to_owned(),
        count,
    }
}

/// The roles of httpparser `http_parser_execute` as the Tracer declares them:
/// two states, then the input and its length.
fn http_roles() -> Vec<ArgRole> {
    vec![
        out_role(Role::State, Some("parser")),
        out_role(Role::State, Some("settings")),
        role(Role::InBuf),
        role(Role::InLen),
    ]
}

fn http_setup() -> Vec<SetupStep> {
    vec![call_step(
        "http_parser_init",
        &["&parser", "2"],
        Some("void"),
    )]
}

/// The committed manifest with the httpparser `http_parser_execute` entry given
/// `roles`, `setup` and `locals`; the declaration is inline, not the committed one.
fn with_http(roles: Vec<ArgRole>, setup: Vec<SetupStep>, locals: Vec<LocalObject>) -> Manifest {
    let mut manifest = committed();
    let entry = manifest
        .target
        .get_mut("httpparser")
        .unwrap()
        .entry
        .iter_mut()
        .find(|entry| entry.name == "http_parser_execute")
        .unwrap();
    assert_eq!(entry.signature, HTTP_SIGNATURE);
    entry.arg = roles;
    entry.setup = setup;
    entry.local = locals;
    manifest
}

fn http_issue(manifest: &Manifest, words: &[&str]) -> bool {
    issues_of(manifest).iter().any(|issue| {
        issue.contains("http_parser_execute") && words.iter().all(|w| issue.contains(w))
    })
}

#[test]
fn eqv_03_roles_setup_grammar_is_closed() {
    // D-12, T-03-25: a setup step is a call of a C identifier with arguments from
    // the closed grammar and an expected return, or a set of a field path; nothing
    // else reaches the generated driver.
    with_http(http_roles(), http_setup(), Vec::new())
        .validate()
        .unwrap();

    // The call name is a C identifier.
    for bad in [
        "http_parser_init; evil()",
        "a b",
        "",
        "1abc",
        "f(",
        "a.b",
        "a-b",
    ] {
        let manifest = with_http(
            http_roles(),
            vec![call_step(bad, &["&parser", "2"], Some("void"))],
            Vec::new(),
        );
        assert!(
            http_issue(&manifest, &["setup[0]", "identifier"]),
            "{bad:?}: {:?}",
            issues_of(&manifest)
        );
    }

    // Every argument is a token of the closed grammar that names something declared.
    for bad in [
        "$in;",
        "&nosuch",
        "nosuch",
        "1+1",
        "$arg:",
        "$arg:nosuch",
        "&",
        "$IN",
        "0x10",
        "01",
        "--1",
        " 2",
        "2 ",
        "$arg:parser",
        "parser.type",
        "",
    ] {
        let manifest = with_http(
            http_roles(),
            vec![call_step(
                "http_parser_init",
                &["&parser", bad],
                Some("void"),
            )],
            Vec::new(),
        );
        assert!(
            http_issue(&manifest, &["setup[0]", "argument"]),
            "{bad:?}: {:?}",
            issues_of(&manifest)
        );
    }
    // The grammar accepts each token form that names something declared: the input,
    // its length, the capacity, a decimal, the address of a state or local and the
    // value of a pointer local.
    with_http(
        http_roles(),
        vec![call_step(
            "http_parser_init",
            &["&parser", "$in", "$len", "$cap", "-5", "&blob", "ptr"],
            Some("0"),
        )],
        vec![
            local("blob", "http_parser", None),
            local("ptr", "const char *", None),
        ],
    )
    .validate()
    .unwrap();
    // A pointer local is passed by value; a local that is not a pointer is not.
    let manifest = with_http(
        http_roles(),
        vec![call_step("http_parser_init", &["blob", "2"], Some("void"))],
        vec![local("blob", "http_parser", None)],
    );
    assert!(
        http_issue(&manifest, &["setup[0]", "argument", "blob"]),
        "{:?}",
        issues_of(&manifest)
    );

    // Exactly one of call and set; a call has an expectation; a set has a value.
    let both = SetupStep {
        call: Some("http_parser_init".to_owned()),
        args: Vec::new(),
        expect: Some("void".to_owned()),
        set: Some("parser.type".to_owned()),
        value: Some("1".to_owned()),
    };
    let neither = SetupStep {
        call: None,
        args: Vec::new(),
        expect: None,
        set: None,
        value: None,
    };
    for step in [both, neither] {
        let manifest = with_http(http_roles(), vec![step], Vec::new());
        assert!(
            http_issue(&manifest, &["setup[0]", "exactly one of call or set"]),
            "{:?}",
            issues_of(&manifest)
        );
    }
    for expect in [
        None,
        Some("maybe"),
        Some("1.5"),
        Some("2147483648"),
        Some(""),
    ] {
        let manifest = with_http(
            http_roles(),
            vec![call_step("http_parser_init", &["&parser", "2"], expect)],
            Vec::new(),
        );
        assert!(
            http_issue(&manifest, &["setup[0]", "expect"]),
            "{expect:?}: {:?}",
            issues_of(&manifest)
        );
    }
    for expect in ["void", "0", "-2", "17"] {
        with_http(
            http_roles(),
            vec![call_step(
                "http_parser_init",
                &["&parser", "2"],
                Some(expect),
            )],
            Vec::new(),
        )
        .validate()
        .unwrap();
    }
    let mut mixed = set_step("parser.type", Some("1"));
    mixed.args = vec!["2".to_owned()];
    let manifest = with_http(http_roles(), vec![mixed], Vec::new());
    assert!(
        http_issue(&manifest, &["setup[0]", "args"]),
        "{:?}",
        issues_of(&manifest)
    );
    let mut mixed = call_step("http_parser_init", &["&parser", "2"], Some("void"));
    mixed.value = Some("1".to_owned());
    let manifest = with_http(http_roles(), vec![mixed], Vec::new());
    assert!(
        http_issue(&manifest, &["setup[0]", "value"]),
        "{:?}",
        issues_of(&manifest)
    );

    // A set path is <name>.<member>(.<member>)* in C identifiers over a declared
    // local or state; its value is one of the closed forms.
    for bad in [
        "parser.type; x",
        "parser",
        "nosuch.type",
        "parser..type",
        ".type",
        "parser.1a",
    ] {
        let manifest = with_http(http_roles(), vec![set_step(bad, Some("1"))], Vec::new());
        assert!(
            http_issue(&manifest, &["setup[0]", "path"]),
            "{bad:?}: {:?}",
            issues_of(&manifest)
        );
    }
    for bad in [
        "$in*2", "$len-x", "$in+", "$in+-1", "$len-", "1+1", "$cap", "",
    ] {
        let manifest = with_http(
            http_roles(),
            vec![set_step("parser.data", Some(bad))],
            Vec::new(),
        );
        assert!(
            http_issue(&manifest, &["setup[0]", "value"]),
            "{bad:?}: {:?}",
            issues_of(&manifest)
        );
    }
    for good in ["$in", "$in+4", "$len", "$len-2", "12", "-1"] {
        with_http(
            http_roles(),
            vec![set_step("parser.data", Some(good))],
            Vec::new(),
        )
        .validate()
        .unwrap();
    }

    // The step that is wrong is the one named.
    let manifest = with_http(
        http_roles(),
        vec![
            call_step("http_parser_init", &["&parser", "2"], Some("void")),
            call_step("http_parser_init", &["&parser", "1+1"], Some("void")),
        ],
        Vec::new(),
    );
    assert!(http_issue(&manifest, &["setup[1]", "argument"]));
    assert!(!http_issue(&manifest, &["setup[0]"]));

    // At most 16 steps.
    let steps: Vec<SetupStep> = (0..17).map(|_| http_setup().remove(0)).collect();
    let manifest = with_http(http_roles(), steps, Vec::new());
    assert!(
        http_issue(&manifest, &["setup", "at most 16"]),
        "{:?}",
        issues_of(&manifest)
    );

    // The token parsers themselves.
    assert_eq!(SetupArg::parse("$in"), Some(SetupArg::In));
    assert_eq!(SetupArg::parse("$len"), Some(SetupArg::Len));
    assert_eq!(SetupArg::parse("$cap"), Some(SetupArg::Cap));
    assert_eq!(
        SetupArg::parse("$arg:flags"),
        Some(SetupArg::Scalar("flags".to_owned()))
    );
    assert_eq!(SetupArg::parse("-7"), Some(SetupArg::Int(-7)));
    assert_eq!(SetupArg::parse("0"), Some(SetupArg::Int(0)));
    assert_eq!(
        SetupArg::parse("&parser"),
        Some(SetupArg::AddressOf("parser".to_owned()))
    );
    assert_eq!(
        SetupArg::parse("parser"),
        Some(SetupArg::Local("parser".to_owned()))
    );
    for bad in [
        "$in;", "1+1", "$arg:", "&", "", "007", "+1", "&a&b", "&1a", "a b",
    ] {
        assert_eq!(SetupArg::parse(bad), None, "{bad:?}");
    }
    assert_eq!(SetValue::parse("$in"), Some(SetValue::In));
    assert_eq!(SetValue::parse("$in+4"), Some(SetValue::InPlus(4)));
    assert_eq!(SetValue::parse("$len"), Some(SetValue::Len));
    assert_eq!(SetValue::parse("$len-2"), Some(SetValue::LenMinus(2)));
    assert_eq!(SetValue::parse("12"), Some(SetValue::Int(12)));
    assert_eq!(SetValue::parse("-1"), Some(SetValue::Int(-1)));
    for bad in [
        "$in*2", "$len-x", "$in+", "$in+-1", "$len-", "", "$cap", "1+1",
    ] {
        assert_eq!(SetValue::parse(bad), None, "{bad:?}");
    }
}

#[test]
fn eqv_03_roles_types_reaching_c_are_closed() {
    // D-11, T-03-25: every type text that reaches generated C matches the closed
    // grammar; the local objects are where a new type text enters in this plan.
    assert_eq!(
        TYPE_TEXT_GRAMMAR,
        r"^(const )?(struct |union |enum )?[A-Za-z_][A-Za-z0-9_]*( \*)?$"
    );
    for good in [
        "CborParser",
        "struct Pet",
        "const char *",
        "union u",
        "enum e",
        "uint8_t *",
        "const struct x *",
        "_x1",
    ] {
        assert!(is_closed_type_text(good), "{good:?}");
    }
    for bad in [
        "int (*)(void)",
        "struct x; y",
        "char **",
        "unsigned char",
        "const",
        "struct",
        "x y",
        "1x",
        "",
        "char*",
        "char *\n",
        "x /* c */",
        " x",
        "x ",
        "struct struct x",
        "x[3]",
    ] {
        assert!(!is_closed_type_text(bad), "{bad:?}");
    }

    // Through the manifest: a local of an open type is refused, naming the local.
    for good in ["CborParser", "struct Pet", "const char *"] {
        with_http(
            http_roles(),
            http_setup(),
            vec![local("scratch", good, None)],
        )
        .validate()
        .unwrap();
    }
    for bad in ["int (*)(void)", "struct x; y", "char **", "x y"] {
        let manifest = with_http(
            http_roles(),
            http_setup(),
            vec![local("scratch", bad, None)],
        );
        assert!(
            http_issue(&manifest, &["local[0]", "type"]),
            "{bad:?}: {:?}",
            issues_of(&manifest)
        );
    }

    // A local's count is 1 to 64; its name is a C identifier, not reserved, and
    // unique among the roles' names and the other locals.
    for count in [Some(1), Some(64), None] {
        with_http(
            http_roles(),
            http_setup(),
            vec![local("scratch", "CborParser", count)],
        )
        .validate()
        .unwrap();
    }
    for count in [0u32, 65, 1000] {
        let manifest = with_http(
            http_roles(),
            http_setup(),
            vec![local("scratch", "CborParser", Some(count))],
        );
        assert!(
            http_issue(&manifest, &["local[0]", "count"]),
            "{count}: {:?}",
            issues_of(&manifest)
        );
    }
    for (name, word) in [
        ("a b", "identifier"),
        ("heap", "reserved"),
        ("parser", "duplicate"),
    ] {
        let manifest = with_http(
            http_roles(),
            http_setup(),
            vec![local(name, "CborParser", None)],
        );
        assert!(
            http_issue(&manifest, &["local[0]", word]),
            "{name:?}: {:?}",
            issues_of(&manifest)
        );
    }
    let manifest = with_http(
        http_roles(),
        http_setup(),
        vec![
            local("scratch", "CborParser", None),
            local("scratch", "CborParser", None),
        ],
    );
    assert!(http_issue(&manifest, &["local[1]", "duplicate"]));

    // A setup step may name a local; the entry still declares it.
    let manifest = with_http(
        http_roles(),
        vec![call_step(
            "http_parser_init",
            &["&scratch", "2"],
            Some("void"),
        )],
        vec![local("scratch", "http_parser", None)],
    );
    manifest.validate().unwrap();
}

#[test]
fn eqv_03_roles_setup_and_local_parse_from_toml_and_stay_absent_otherwise() {
    // D-12: the new tables are additive; an entry without them serialises as before.
    let mut plain = committed();
    for target in plain.target.values_mut() {
        for entry in &mut target.entry {
            entry.setup.clear();
            entry.local.clear();
        }
    }
    let json = plain.canonical_json().unwrap();
    assert!(!json.contains("\"setup\""), "no setup key when empty");
    assert!(!json.contains("\"local\""), "no local key when empty");

    // http_parser_parse_url has no roles in the committed manifest.
    let marker = "signature = \"int (const char *, size_t, int, struct http_parser_url *)\"\n";
    let text = committed_text();
    assert!(text.contains(marker), "the marker line is in the manifest");
    let block = "\n[[target.httpparser.entry.arg]]\nrole = \"in_buf\"\n\n[[target.httpparser.entry.arg]]\nrole = \"in_len\"\n\n[[target.httpparser.entry.arg]]\nrole = \"scalar\"\nname = \"is_connect\"\ndefault = 0\n\n[[target.httpparser.entry.arg]]\nrole = \"state\"\nname = \"url\"\n\n[[target.httpparser.entry.local]]\nname = \"scratch\"\ntype = \"http_parser\"\ncount = 2\n\n[[target.httpparser.entry.setup]]\ncall = \"http_parser_url_init\"\nargs = [\"&url\"]\nexpect = \"void\"\n\n[[target.httpparser.entry.setup]]\nset = \"scratch.data\"\nvalue = \"$in\"\n";
    let edited = text.replacen(marker, &format!("{marker}{block}"), 1);
    let manifest = Manifest::parse(&edited).unwrap();
    manifest.validate().unwrap();
    let entry = manifest.target["httpparser"]
        .entry
        .iter()
        .find(|entry| entry.name == "http_parser_parse_url")
        .unwrap();
    assert_eq!(entry.local, [local("scratch", "http_parser", Some(2))]);
    assert_eq!(entry.setup.len(), 2);
    assert_eq!(entry.setup[0].call.as_deref(), Some("http_parser_url_init"));
    assert_eq!(entry.setup[0].args, ["&url"]);
    assert_eq!(entry.setup[1].set.as_deref(), Some("scratch.data"));

    // Unknown keys are refused, as everywhere in the manifest.
    let unknown = edited.replacen("expect = \"void\"", "expect = \"void\"\nretry = 3", 1);
    assert!(Manifest::parse(&unknown).is_err());
}
