#![allow(clippy::unwrap_used, clippy::expect_used)]
//! EQV-02, D-06, D-08: the library-state value model and the comparison rule.

use mt_vectors::compare::{Comparison, compare};
use mt_vectors::state::{LibState, Pointer, RESERVED_NAMES, StateValue};
use mt_vectors::vector::{HasUbLabel, Vector};
use serde_json::{Value, json};

fn int(value: i64) -> Value {
    json!({"type": "int", "value": value})
}

fn state_value(shape: &Value) -> Result<StateValue, serde_json::Error> {
    serde_json::from_value(shape.clone())
}

fn lib_state(shape: &Value) -> Result<LibState, serde_json::Error> {
    serde_json::from_value(shape.clone())
}

fn vector(rc: i32, stdout: &str, stderr: &str, state_out: &Value) -> Vector {
    let value = json!({
        "argv": ["corejson/default/JSON_Validate", "7"],
        "stdin": "in",
        "env": {},
        "stdout": {"pattern": stdout, "is_regex": false},
        "stderr": {"pattern": stderr, "is_regex": false},
        "rc": rc,
        "lib_state_in": {},
        "lib_state_out": state_out
    });
    Vector::from_json_bytes(&serde_json::to_vec(&value).unwrap()).unwrap()
}

fn status(code: i64, len: i64) -> Value {
    json!({
        "st": {
            "type": "struct S",
            "fields": {"code": int(code), "len": int(len)}
        }
    })
}

#[test]
fn eqv_02_lib_state_shapes_round_trip() {
    let shapes = [
        int(-5),
        json!({"type": "unsigned int", "value": 4_294_967_295_u64}),
        json!({"type": "uint64_t", "value": "18446744073709551615"}),
        json!({"type": "long long", "value": "-9223372036854775808"}),
        json!({"type": "float", "bits": "7fc00000"}),
        json!({"type": "double", "bits": "3ff0000000000000"}),
        json!({"type": "uint8_t [4]", "hex": "00ff10a5"}),
        json!({"type": "uint8_t [0]", "hex": ""}),
        json!({"type": "int [2]", "items": [int(1), int(2)]}),
        json!({
            "type": "struct S",
            "fields": {"a": int(1), "b": {"type": "char *", "pointer": null}}
        }),
        json!({"type": "const uint8_t *", "pointer": {"buffer": "in", "offset": 4}}),
        json!({"type": "uint8_t *", "pointer": {"buffer": "out", "offset": 0}}),
        json!({"type": "struct T *", "pointer": {"buffer": "state_a", "offset": 8}}),
        json!({"type": "void *", "pointer": {"buffer": "heap", "offset": 16}}),
        json!({"type": "int *", "pointer": {"buffer": "global:lib/x.c.obj:counter", "offset": 0}}),
        json!({"type": "void (*)(int)", "pointer": {"symbol": "callback"}}),
        json!({"type": "int", "unobserved": "padding"}),
    ];
    for shape in &shapes {
        let value = state_value(shape).unwrap_or_else(|e| panic!("{shape}: {e}"));
        assert_eq!(&serde_json::to_value(&value).unwrap(), shape);
    }
    assert_eq!(
        state_value(&json!({"type": "char *", "pointer": null})).unwrap(),
        StateValue::Pointer {
            c_type: "char *".to_owned(),
            pointer: Pointer::Null
        }
    );

    assert_eq!(RESERVED_NAMES, ["globals", "heap", "in", "ret"]);
    let whole = json!({
        "pkt": {"type": "struct P", "fields": {"code": int(1)}},
        "in": {"type": "uint8_t [3]", "hex": "010203"},
        "ret": {"type": "struct R", "fields": {"v": int(2)}},
        "globals": {"type": "globals", "fields": {"lib/x.c.obj:counter": int(3)}},
        "heap": {"type": "heap", "fields": {"16": {"type": "char *", "hex": "6100"}}}
    });
    let parsed = lib_state(&whole).unwrap();
    assert_eq!(serde_json::to_value(&parsed).unwrap(), whole);
}

#[test]
fn eqv_02_lib_state_rejects_bad_shapes() {
    let bad = [
        ("uppercase hex", json!({"type": "uint8_t [1]", "hex": "AB"})),
        (
            "odd-length hex",
            json!({"type": "uint8_t [1]", "hex": "abc"}),
        ),
        (
            "uppercase bits",
            json!({"type": "float", "bits": "7FC00000"}),
        ),
        ("short bits", json!({"type": "float", "bits": "7fc0"})),
        (
            "bare integer pointer",
            json!({"type": "int *", "pointer": 4096}),
        ),
        (
            "raw address key",
            json!({"type": "int *", "pointer": {"addr": 4096}}),
        ),
        (
            "pointer without offset",
            json!({"type": "int *", "pointer": {"buffer": "in"}}),
        ),
        (
            "pointer with a negative offset",
            json!({"type": "int *", "pointer": {"buffer": "in", "offset": -1}}),
        ),
        (
            "bad region",
            json!({"type": "int *", "pointer": {"buffer": "0x1000", "offset": 0}}),
        ),
        (
            "global region without a symbol",
            json!({"type": "int *", "pointer": {"buffer": "global:obj", "offset": 0}}),
        ),
        (
            "empty unobserved reason",
            json!({"type": "int", "unobserved": ""}),
        ),
        (
            "64-bit value as a number above 2^32",
            json!({"type": "uint64_t", "value": 5_000_000_000_u64}),
        ),
        (
            "32-bit value below -2^31",
            json!({"type": "int", "value": -2_147_483_649_i64}),
        ),
        (
            "hex text for a 64-bit value",
            json!({"type": "uint64_t", "value": "0x10"}),
        ),
        ("leading zero", json!({"type": "uint64_t", "value": "01"})),
        ("fractional value", json!({"type": "int", "value": 1.5})),
        ("boolean value", json!({"type": "int", "value": true})),
        (
            "unknown key",
            json!({"type": "int", "value": 1, "extra": 2}),
        ),
        ("missing type", json!({"value": 1})),
        (
            "two shapes at once",
            json!({"type": "int", "value": 1, "hex": "00"}),
        ),
        ("no shape", json!({"type": "int"})),
        (
            "unknown key in a struct member",
            json!({"type": "struct S", "fields": {"a": {"type": "int", "value": 1, "x": 1}}}),
        ),
    ];
    for (what, shape) in &bad {
        assert!(
            state_value(shape).is_err(),
            "{what} must be refused: {shape}"
        );
    }

    let reserved_bad = [
        ("in is not bytes", json!({"in": int(1)})),
        ("globals is not a struct", json!({"globals": int(1)})),
        (
            "globals key without a symbol",
            json!({"globals": {"type": "globals", "fields": {"counter": int(1)}}}),
        ),
        (
            "heap key is not a decimal offset",
            json!({"heap": {"type": "heap", "fields": {"0x10": int(1)}}}),
        ),
    ];
    for (what, shape) in &reserved_bad {
        assert!(lib_state(shape).is_err(), "{what} must be refused: {shape}");
    }
}

#[test]
fn eqv_02_lib_state_counts_unobserved() {
    let shape = json!({
        "a": {"type": "int", "unobserved": "padding"},
        "b": {
            "type": "struct S",
            "fields": {
                "x": int(1),
                "y": {"type": "int", "unobserved": "reserved"},
                "z": {
                    "type": "struct U",
                    "fields": {"w": {"type": "int", "unobserved": "reserved"}}
                }
            }
        },
        "c": {
            "type": "int [2]",
            "items": [int(1), {"type": "int", "unobserved": "reserved"}]
        }
    });
    assert_eq!(lib_state(&shape).unwrap().unobserved_count(), 4);
    assert_eq!(LibState::default().unobserved_count(), 0);
}

#[test]
fn eqv_02_has_ub_vectors_are_excluded_from_comparison() {
    let mut expected = vector(0, "ab", "outcome=fault", &status(1, 2));
    expected.has_ub = Some(HasUbLabel::new("crash:SIGSEGV").unwrap());
    let actual = vector(5, "zz", "outcome=ok", &status(9, 9));
    assert_eq!(
        compare(&expected, &actual),
        Comparison::Excluded {
            label: HasUbLabel::new("crash:SIGSEGV").unwrap()
        }
    );
    // The label of the expected vector decides, not the actual one's.
    let mut labelled_actual = actual.clone();
    labelled_actual.has_ub = Some(HasUbLabel::new("timeout").unwrap());
    let plain = vector(0, "ab", "outcome=ok", &status(1, 2));
    assert_ne!(
        compare(&plain, &labelled_actual),
        Comparison::Excluded {
            label: HasUbLabel::new("timeout").unwrap()
        }
    );
}

#[test]
fn eqv_02_compares_only_the_observation_point() {
    let expected = vector(0, "ab", "outcome=ok", &status(1, 2));

    // Not compared: stderr detail text, lib_state_in, argv, stdin.
    let mut same = vector(0, "ab", "outcome=ok detail words", &status(1, 2));
    same.argv.push("99".to_owned());
    same.stdin = "other".to_owned();
    same.lib_state_in = lib_state(&json!({"pre": int(4)})).unwrap();
    assert_eq!(compare(&expected, &same), Comparison::Equal);

    let differs = |actual: &Vector, fields: &[&str]| {
        assert_eq!(
            compare(&expected, actual),
            Comparison::Differs {
                fields: fields.iter().map(|f| (*f).to_owned()).collect()
            }
        );
    };
    differs(&vector(3, "ab", "outcome=ok", &status(1, 2)), &["rc"]);
    differs(
        &vector(0, "aB", "outcome=ok", &status(1, 2)),
        &["stdout.pattern"],
    );
    differs(
        &vector(0, "ab", "outcome=ok", &status(7, 2)),
        &["lib_state_out.st.code"],
    );
    differs(
        &vector(0, "ab", "outcome=fault", &status(1, 2)),
        &["stderr.outcome"],
    );
    differs(
        &vector(1, "ab", "outcome=timeout", &status(1, 2)),
        &["rc", "stderr.outcome"],
    );
    // A top-level field present on one side only.
    let mut extra = status(1, 2);
    extra["more"] = int(1);
    differs(
        &vector(0, "ab", "outcome=ok", &extra),
        &["lib_state_out.more"],
    );

    // A field marked unobserved in the expected vector is not compared.
    let mut shape = status(1, 2);
    shape["st"]["fields"]["len"] = json!({"type": "int", "unobserved": "padding"});
    let lenient = vector(0, "ab", "outcome=ok", &shape);
    assert_eq!(
        compare(&lenient, &vector(0, "ab", "outcome=ok", &status(1, 99))),
        Comparison::Equal
    );
    assert_eq!(
        compare(&lenient, &vector(0, "ab", "outcome=ok", &status(5, 99))),
        Comparison::Differs {
            fields: vec!["lib_state_out.st.code".to_owned()]
        }
    );
}
