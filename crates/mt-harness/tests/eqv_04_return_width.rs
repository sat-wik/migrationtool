#![allow(clippy::unwrap_used, clippy::expect_used)]
//! EQV-04: a return type that DWARF resolves to an integer wider than 32 bits
//! is refused instead of being truncated into `rc`.

use mt_harness::adapter::ReturnKind;
use mt_harness::layout::{CType, Kind};
use mt_harness::run::resolved_return_kind;

fn int(text: &str, size: u32, signed: bool) -> CType {
    CType {
        text: text.to_owned(),
        size,
        kind: Kind::Int { signed },
    }
}

#[test]
fn eqv_04_wide_typedef_return_is_refused() {
    for text in ["intmax_t", "uintmax_t", "int_fast64_t", "off64_t"] {
        let ty = int(text, 8, true);
        let reason = resolved_return_kind(Some(&ty), text).unwrap_err();
        assert!(reason.contains("8 bytes"), "{reason}");
    }
}

#[test]
fn eqv_04_wide_enum_return_is_refused() {
    let ty = CType {
        text: "big_t".to_owned(),
        size: 8,
        kind: Kind::Enum { signed: false },
    };
    assert!(resolved_return_kind(Some(&ty), "big_t").is_err());
}

#[test]
fn eqv_04_narrow_integer_returns_stay_integer() {
    for size in [1, 2, 4] {
        let ty = int("my_t", size, false);
        assert_eq!(
            resolved_return_kind(Some(&ty), "my_t").unwrap(),
            ReturnKind::Integer
        );
    }
    assert_eq!(
        resolved_return_kind(None, "int").unwrap(),
        ReturnKind::Integer
    );
}

#[test]
fn eqv_04_pointer_and_float_returns_keep_their_mapping() {
    let ptr = CType {
        text: "T *".to_owned(),
        size: 4,
        kind: Kind::Pointer { function: false },
    };
    assert_eq!(
        resolved_return_kind(Some(&ptr), "T *").unwrap(),
        ReturnKind::Pointer
    );
    let float = CType {
        text: "float".to_owned(),
        size: 4,
        kind: Kind::Float,
    };
    assert!(resolved_return_kind(Some(&float), "float").is_err());
}
