#![allow(clippy::unwrap_used, clippy::expect_used)]
//! EQV-02: the reference level of the vector schema. The fixture is our own
//! hand-made vector (no TRACTOR content), written in canonical form.

use mt_vectors::bytes::decode_latin1;
use mt_vectors::vector::{
    HasUbLabel, MAX_VECTOR_BYTES, OUTCOME_CLASSES, OutcomeClass, TractorVector, Vector, VectorError,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const FIXTURE: &[u8] = include_bytes!("fixtures/own/reference.json");

const NINE: [&str; 9] = [
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

fn fixture_value() -> Value {
    serde_json::from_slice(FIXTURE).unwrap()
}

fn parse_value(value: &Value) -> Result<Vector, VectorError> {
    Vector::from_json_bytes(&serde_json::to_vec(value).unwrap())
}

fn error_text(value: &Value) -> String {
    parse_value(value).unwrap_err().to_string()
}

#[test]
fn eqv_02_reference_vector_round_trips_byte_for_byte() {
    let vector = Vector::from_json_bytes(FIXTURE).unwrap();
    let canonical = vector.to_canonical_json().unwrap();
    assert_eq!(canonical, FIXTURE, "canonical bytes differ from the file");

    let text = String::from_utf8(canonical).unwrap();
    let mut last = 0;
    for key in &NINE[..8] {
        let at = text
            .find(&format!("\"{key}\":"))
            .unwrap_or_else(|| panic!("key {key} is missing"));
        assert!(at >= last, "key {key} is out of order");
        last = at;
    }

    let digest = Sha256::digest(FIXTURE);
    let want: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(vector.sha256(), want);
    assert_eq!(vector.sha256().len(), 64);

    assert_eq!(vector.outcome_class().unwrap(), OutcomeClass::Ok);
    assert_eq!(vector.rc, 1);
    assert_eq!(
        decode_latin1(&vector.stdin).unwrap(),
        vec![0x00, 0x22, 0x5C, 0x7F, 0x80, 0xFF, b'o', b'k']
    );
}

#[test]
fn eqv_02_outcome_classes_are_the_contract_set() {
    assert_eq!(
        OUTCOME_CLASSES,
        ["ok", "fault", "abort", "timeout", "setup_failed"]
    );
    for word in OUTCOME_CLASSES {
        let class = OutcomeClass::from_word(word).unwrap();
        assert_eq!(class.word(), word);
    }
    assert!(OutcomeClass::from_word("executor_error").is_none());
}

#[test]
fn eqv_02_rejects_merged_lib_state() {
    let mut value = fixture_value();
    let object = value.as_object_mut().unwrap();
    object.remove("lib_state_in");
    object.remove("lib_state_out");
    object.insert("lib_state".to_owned(), json!({}));
    let text = error_text(&value);
    assert!(text.contains("lib_state"), "the error must name it: {text}");
}

#[test]
fn eqv_02_rejects_unknown_key() {
    let mut extra = fixture_value();
    extra
        .as_object_mut()
        .unwrap()
        .insert("extra".to_owned(), json!(1));
    assert!(error_text(&extra).contains("extra"));

    let mut note = fixture_value();
    note.as_object_mut()
        .unwrap()
        .insert("note".to_owned(), json!("free text"));
    assert!(error_text(&note).contains("note"));

    let mut inner = fixture_value();
    inner["stdout"]
        .as_object_mut()
        .unwrap()
        .insert("flags".to_owned(), json!("x"));
    assert!(error_text(&inner).contains("flags"));
}

#[test]
fn eqv_02_has_ub_absent_means_no_observed_ub() {
    let vector = Vector::from_json_bytes(FIXTURE).unwrap();
    let text = String::from_utf8(vector.to_canonical_json().unwrap()).unwrap();
    assert!(!text.contains("has_ub"), "no observed UB is the absent key");
    assert!(!vector.excluded_from_comparison());

    let mut labelled = vector.clone();
    labelled.has_ub = Some(HasUbLabel::new("crash:SIGSEGV").unwrap());
    assert!(labelled.excluded_from_comparison());
    let bytes = labelled.to_canonical_json().unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["has_ub"], json!("crash:SIGSEGV"));
    assert!(value["has_ub"].is_string());
    let text = String::from_utf8(bytes.clone()).unwrap();
    assert!(
        text.find("\"has_ub\"").unwrap() > text.find("\"lib_state_out\"").unwrap(),
        "has_ub comes last"
    );
    assert_eq!(Vector::from_json_bytes(&bytes).unwrap(), labelled);

    for bad in [json!(true), json!(false), json!(null), json!(3)] {
        let mut value = fixture_value();
        value
            .as_object_mut()
            .unwrap()
            .insert("has_ub".to_owned(), bad.clone());
        assert!(
            parse_value(&value).is_err(),
            "has_ub {bad} must be refused at the reference level"
        );
    }
    for bad in [
        "",
        "Crash",
        "crash:",
        "crash:SIG SEGV",
        "1crash",
        "crash::x",
    ] {
        assert!(
            HasUbLabel::new(bad).is_err(),
            "label {bad:?} must be refused"
        );
    }
    for good in [
        "timeout",
        "oob-write:guard-slack",
        "crash:SIGABRT",
        "a:b:c_d.e-f",
    ] {
        assert!(
            HasUbLabel::new(good).is_ok(),
            "label {good:?} must be accepted"
        );
    }
}

#[test]
fn eqv_02_requires_every_reference_field() {
    for key in &NINE[..8] {
        let mut value = fixture_value();
        value.as_object_mut().unwrap().remove(*key);
        let text = error_text(&value);
        assert!(text.contains(key), "removing {key} must name it: {text}");
    }

    let mut env = fixture_value();
    env["env"] = json!({"PATH": "/bin"});
    assert!(error_text(&env).contains("env"));

    let mut regex = fixture_value();
    regex["stdout"]["is_regex"] = json!(true);
    assert!(error_text(&regex).contains("is_regex"));

    for bad in [
        "corejson",
        "corejson/default",
        "Corejson/default/f",
        "a/b/c/d",
        "a/b/1x",
        "/b/c",
    ] {
        let mut value = fixture_value();
        value["argv"] = json!([bad]);
        assert!(error_text(&value).contains("argv"), "argv[0] {bad:?}");
    }
    let mut empty = fixture_value();
    empty["argv"] = json!([]);
    assert!(error_text(&empty).contains("argv"));
    let mut scalars = fixture_value();
    scalars["argv"] = json!(["corejson/default/JSON_Validate", "12", "0"]);
    assert!(parse_value(&scalars).is_ok());

    for bad in ["", "ok", "outcome=", "outcome=executor_error", "outcome=OK"] {
        let mut value = fixture_value();
        value["stderr"]["pattern"] = json!(bad);
        assert!(error_text(&value).contains("stderr"), "stderr {bad:?}");
    }
    let mut detail = fixture_value();
    detail["stderr"]["pattern"] = json!("outcome=setup_failed step=2");
    let parsed = parse_value(&detail).unwrap();
    assert_eq!(parsed.outcome_class().unwrap(), OutcomeClass::SetupFailed);
}

#[test]
fn eqv_02_rejects_a_file_above_the_size_cap() {
    let big = vec![b' '; MAX_VECTOR_BYTES + 1];
    let text = Vector::from_json_bytes(&big).unwrap_err().to_string();
    assert!(text.contains("above"), "{text}");
}

// ---- the TRACTOR-compatible level (built from literals we write, shaped like
// the research's field survey; no TRACTOR content) ----

fn load_tractor(value: &Value) -> (TractorVector, Value) {
    TractorVector::load_unchanged(&serde_json::to_vec(value).unwrap()).unwrap()
}

#[test]
fn eqv_02_tractor_level_accepts_every_optional_shape() {
    let shapes = [
        json!({}),
        json!({
            "lib_state_in": {"x": 1, "f": 1.5, "s": "text", "n": null, "a": [1, 2], "o": {"k": true}},
            "lib_state_out": {"returns": 0}
        }),
        json!({
            "argv": [],
            "stdin": "a\u{0000}b",
            "stdout": {"pattern": "hello"},
            "stderr": {"pattern": ".*fault", "is_regex": true},
            "rc": -11
        }),
        json!({"argv": ["1", "2"], "env": {"A": "b"}, "stdout": {"pattern": "x", "is_regex": false}, "note": "free text"}),
        json!({"lib_state_in": {}, "lib_state_out": {}, "stdout": {"pattern": ""}, "rc": 0}),
    ];
    for shape in &shapes {
        let (vector, parsed) = load_tractor(shape);
        assert_eq!(&parsed, shape, "the parsed value is the file's value");
        assert_eq!(
            serde_json::to_value(&vector).unwrap(),
            parsed,
            "re-serialization must be JSON-equal for {shape}"
        );
    }
    let (with_note, _) = load_tractor(&shapes[3]);
    assert_eq!(with_note.note.as_deref(), Some("free text"));
}

#[test]
fn eqv_02_tractor_has_ub_presence_means_ub() {
    for present in [json!("crash:SIGSEGV"), json!(7), json!(false), json!(null)] {
        let shape = json!({"stdout": {"pattern": "x"}, "has_ub": present});
        let (vector, parsed) = load_tractor(&shape);
        assert!(vector.has_ub_present(), "has_ub {present} is present");
        assert_eq!(serde_json::to_value(&vector).unwrap(), parsed);
        assert!(parsed.as_object().unwrap().contains_key("has_ub"));
    }
    let (absent, _) = load_tractor(&json!({"stdout": {"pattern": "x"}}));
    assert!(!absent.has_ub_present());
}

#[test]
fn eqv_02_tractor_level_rejects_unknown_keys() {
    let top = serde_json::to_vec(&json!({"stdout": {"pattern": "x"}, "extra": 1})).unwrap();
    let text = TractorVector::load_unchanged(&top).unwrap_err().to_string();
    assert!(text.contains("extra"), "{text}");

    let inner = serde_json::to_vec(&json!({"stdout": {"pattern": "x", "flags": "i"}})).unwrap();
    let text = TractorVector::load_unchanged(&inner)
        .unwrap_err()
        .to_string();
    assert!(text.contains("flags"), "{text}");

    // The merged key is unknown at this level too.
    let merged = serde_json::to_vec(&json!({"lib_state": {}})).unwrap();
    assert!(TractorVector::load_unchanged(&merged).is_err());

    let big = vec![b' '; MAX_VECTOR_BYTES + 1];
    assert!(TractorVector::load_unchanged(&big).is_err());
}
