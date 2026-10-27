#![allow(clippy::unwrap_used, clippy::expect_used)]
//! EQV-02, D-25: the Latin-1 identity codec that carries bytes in the text
//! fields `stdin` and `stdout.pattern`.

use mt_vectors::bytes::{CodecError, decode_latin1, encode_latin1};
use mt_vectors::vector::{Output, Vector};

fn all_bytes() -> Vec<u8> {
    (0..=255u8).collect()
}

#[test]
fn eqv_02_byte_codec_round_trips_every_byte() {
    let bytes = all_bytes();
    let text = encode_latin1(&bytes);
    assert_eq!(text.chars().count(), 256);
    for (b, ch) in bytes.iter().zip(text.chars()) {
        assert_eq!(u32::from(ch), u32::from(*b), "byte {b:#04x}");
    }
    assert_eq!(decode_latin1(&text).unwrap(), bytes);
    assert_eq!(decode_latin1("").unwrap(), Vec::<u8>::new());
    assert_eq!(encode_latin1(&[]), "");
}

#[test]
fn eqv_02_byte_codec_rejects_wide_characters() {
    let err = decode_latin1("ab\u{0100}c").unwrap_err();
    assert!(matches!(
        err,
        CodecError::WideChar {
            index: 2,
            ch: '\u{0100}'
        }
    ));
    assert!(err.to_string().contains("position 2"), "{err}");

    let err = decode_latin1("\u{20AC}").unwrap_err();
    assert!(matches!(
        err,
        CodecError::WideChar {
            index: 0,
            ch: '\u{20AC}'
        }
    ));
    assert!(err.to_string().contains("position 0"), "{err}");

    // The highest accepted character is U+00FF.
    assert_eq!(decode_latin1("\u{00FF}").unwrap(), vec![0xFF]);
}

#[test]
fn eqv_02_byte_codec_in_stdin_and_stdout() {
    let bytes = all_bytes();
    let reversed: Vec<u8> = bytes.iter().rev().copied().collect();
    let vector = Vector::from_json_bytes(
        &Vector {
            argv: vec!["corejson/default/JSON_Validate".to_owned()],
            stdin: encode_latin1(&bytes),
            env: Default::default(),
            stdout: Output {
                pattern: encode_latin1(&reversed),
                is_regex: false,
            },
            stderr: Output {
                pattern: "outcome=ok".to_owned(),
                is_regex: false,
            },
            rc: 0,
            lib_state_in: Default::default(),
            lib_state_out: Default::default(),
            has_ub: None,
        }
        .to_canonical_json()
        .unwrap(),
    )
    .unwrap();
    assert_eq!(decode_latin1(&vector.stdin).unwrap(), bytes);
    assert_eq!(decode_latin1(&vector.stdout.pattern).unwrap(), reversed);

    let first = vector.to_canonical_json().unwrap();
    let second = Vector::from_json_bytes(&first)
        .unwrap()
        .to_canonical_json()
        .unwrap();
    assert_eq!(first, second, "canonical bytes must be stable");

    // A wide character in a stored vector is refused when the vector is read.
    let text = String::from_utf8(first).unwrap();
    let broken = text.replacen("\"stdin\": \"", "\"stdin\": \"\u{20AC}", 1);
    let err = Vector::from_json_bytes(broken.as_bytes())
        .unwrap_err()
        .to_string();
    assert!(err.contains("stdin"), "{err}");
}
