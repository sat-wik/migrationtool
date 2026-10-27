#![allow(clippy::unwrap_used, clippy::expect_used)]
//! EQV-04: the layout of a state type comes from the pinned GCC's DWARF (CONTEXT
//! D-09, D-29; RESEARCH Q4 and Pitfall 2). Pure tests over the committed GCC 14.3
//! dump of the self-test probe: a 1-byte enum, a 3-bit bitfield, a 5-byte array,
//! a union and a typedef.

use std::path::Path;

use mt_harness::layout::{CType, Kind, LayoutError, layout_summary, parse_dwarfdump};
use mt_harness::probe::{probe_flags, render_probe};

fn fixture() -> String {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dwarf/selftest-probe-gcc14.txt");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

fn members(ty: &CType) -> &[mt_harness::layout::Member] {
    match &ty.kind {
        Kind::Struct { members } | Kind::Union { members } => members,
        other => panic!("{} is not a struct or union: {other:?}", ty.text),
    }
}

#[test]
fn eqv_04_layout_parser() {
    let model = parse_dwarfdump(&fixture()).unwrap();
    let record = model.type_of_variable("probe_global").unwrap();
    // The typedef is resolved to the struct and keeps its own name as the type.
    assert_eq!(record.text, "probe_record_t");
    // The struct size is the fixture's DW_AT_byte_size (0x0c).
    assert_eq!(record.size, 12);
    let Kind::Struct { members: fields } = &record.kind else {
        panic!("probe_record_t must resolve to a struct: {record:?}");
    };
    let names: Vec<&str> = fields.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, ["kind", "flags", "tag", "value"]);

    // GCC lays an enum out in one byte for this target (Pitfall 2); clang's four
    // would shift every later member.
    let kind = &fields[0];
    assert_eq!((kind.offset, kind.ty.size), (0, 1));
    assert_eq!(kind.ty.text, "enum probe_kind");
    assert!(
        matches!(kind.ty.kind, Kind::Enum { signed: false }),
        "{:?}",
        kind.ty
    );
    assert!(kind.bits.is_none());

    // A 3-bit bitfield at bit 8 of an unsigned int.
    let flags = &fields[1];
    let bits = flags.bits.as_ref().expect("flags is a bitfield");
    assert_eq!((bits.offset, bits.size), (8, 3));
    assert_eq!(flags.ty.text, "unsigned int");
    assert!(matches!(flags.ty.kind, Kind::Int { signed: false }));

    // A 5-byte array of unsigned char at byte 2.
    let tag = &fields[2];
    assert_eq!((tag.offset, tag.ty.size), (2, 5));
    assert_eq!(tag.ty.text, "unsigned char[5]");
    match &tag.ty.kind {
        Kind::Array { elem, count } => {
            assert_eq!(*count, 5);
            assert_eq!(elem.size, 1);
        }
        other => panic!("tag must be an array: {other:?}"),
    }

    // The union records every member at offset 0.
    let value = &fields[3];
    assert_eq!((value.offset, value.ty.size), (8, 4));
    assert_eq!(value.ty.text, "union probe_value");
    let union_members = members(&value.ty);
    assert!(matches!(value.ty.kind, Kind::Union { .. }));
    let union_names: Vec<&str> = union_members.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(union_names, ["word", "bytes"]);
    assert!(union_members.iter().all(|m| m.offset == 0));
}

#[test]
fn eqv_04_layout_summary_is_ordered_text() {
    let model = parse_dwarfdump(&fixture()).unwrap();
    let record = model.type_of_variable("probe_global").unwrap();
    assert_eq!(
        layout_summary(&record),
        "probe_record_t size=12 kind@0:1 flags@1:4:8:3 tag@2:5 value@8:4 value.word@8:4 value.bytes@8:4"
    );
}

#[test]
fn eqv_04_layout_rejects_unsupported_tags() {
    let text = fixture();
    // The array type of the union's `bytes` member becomes a tag nothing reads.
    let edited = text.replacen("DW_TAG_array_type", "DW_TAG_coarray_type", 1);
    assert_ne!(edited, text);
    let model = parse_dwarfdump(&edited).unwrap();
    let error = model.type_of_variable("probe_global").unwrap_err();
    assert!(
        matches!(&error, LayoutError::Unsupported(tag) if tag.contains("DW_TAG_coarray_type")),
        "{error:?}"
    );
    assert!(error.to_string().contains("DW_TAG_coarray_type"), "{error}");

    // A dump that ends before the compile unit's children are closed is refused.
    let lines: Vec<&str> = text.lines().collect();
    let cut = lines[..lines.len() - 3].join("\n");
    assert!(matches!(
        parse_dwarfdump(&cut),
        Err(LayoutError::Truncated(_))
    ));
    // So is text with no DWARF at all.
    assert!(matches!(
        parse_dwarfdump("not a dump"),
        Err(LayoutError::Truncated(_))
    ));
}

#[test]
fn eqv_04_layout_rejects_a_cycle_without_a_pointer() {
    let text = fixture();
    // The member `kind` of probe_record now has type probe_record.
    let edited = text.replacen(
        "DW_AT_type\t(0x0000001e \"probe_kind\")\n                  DW_AT_data_member_location\t(0x00)",
        "DW_AT_type\t(0x0000007d \"probe_record\")\n                  DW_AT_data_member_location\t(0x00)",
        1,
    );
    assert_ne!(edited, text, "the edit must apply");
    let model = parse_dwarfdump(&edited).unwrap();
    assert!(matches!(
        model.type_of_variable("probe_global"),
        Err(LayoutError::Cycle(_))
    ));
}

#[test]
fn eqv_04_layout_unknown_variable_is_an_error() {
    let model = parse_dwarfdump(&fixture()).unwrap();
    assert!(matches!(
        model.type_of_variable("nope"),
        Err(LayoutError::NoSuchVariable(name)) if name == "nope"
    ));
}

#[test]
fn eqv_04_probe_source_declares_one_used_global_per_state() {
    let lines = vec![
        "#include <stdint.h>".to_owned(),
        "#include \"/fixed/coap.h\"".to_owned(),
    ];
    let source = render_probe(
        &lines,
        &[
            ("pkt".to_owned(), "coap_packet_t".to_owned()),
            ("it".to_owned(), "struct item".to_owned()),
        ],
    );
    assert_eq!(
        source,
        "#include <stdint.h>\n#include \"/fixed/coap.h\"\ncoap_packet_t mt_probe_pkt __attribute__((used));\nstruct item mt_probe_it __attribute__((used));\n"
    );
}

#[test]
fn eqv_04_probe_flags_are_the_driver_flags_plus_debug_info() {
    let driver = vec!["-I/x".to_owned(), "-DA=1".to_owned()];
    let flags = probe_flags(&driver);
    let text = flags.join(" ");
    // The baseline first, then the driver's flags, then the debug options; -O0
    // comes after the baseline's -Os so it wins.
    assert!(
        text.starts_with(
            "-mcpu=cortex-m4 -mthumb -mfloat-abi=hard -mfpu=fpv4-sp-d16 -Os -I/x -DA=1 "
        ),
        "{text}"
    );
    assert!(text.ends_with("-std=gnu11 -g -gdwarf-5 -O0 -c"), "{text}");
}
