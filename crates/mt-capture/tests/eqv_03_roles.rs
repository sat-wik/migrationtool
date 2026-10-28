#![allow(clippy::unwrap_used, clippy::expect_used)]
//! EQV-03: the argument roles of a manifest entry (CONTEXT D-11, D-14).
//!
//! Pure tests: the manifest text is the committed one with edits made in the
//! test. Entries without `arg` tables validate exactly as in Phase 2, and the
//! manifest schema version stays 1 (D-29).

use std::path::PathBuf;

use mt_capture::manifest::{
    ArgRole, MANIFEST_SCHEMA_VERSION, Manifest, ManifestError, Role, UnobservedMark,
    split_parameters, state_pointee_type,
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
        of: None,
        mutable: false,
        values: Vec::new(),
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
