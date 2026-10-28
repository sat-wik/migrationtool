#![allow(clippy::unwrap_used, clippy::expect_used)]
//! EQV-04: the decoder turns the bytes of a state object into the contract's
//! typed values (CONTEXT D-06, D-09, D-10). Pure tests: the types are built in
//! code and the bytes are written out.

use std::collections::BTreeMap;

use mt_capture::manifest::{ArgRole, Role, UnobservedMark};
use mt_harness::adapter::{Adapter, ReturnKind};
use mt_harness::decode::{
    Regions, UNRESOLVED_POINTER_REASON, apply_marks, check_marks, classify_pointer, decode_value,
};
use mt_harness::globals::{DumpRange, record_globals};
use mt_harness::heap::{HEAP_CAP_REASON, HEAP_WALK_CAP, walk_heap};
use mt_harness::layout::{BitField, CType, Kind, Member};
use mt_harness::probe::GlobalType;
use mt_vectors::state::{LibState, Pointer, StateValue};

fn int(text: &str, size: u32, signed: bool) -> CType {
    CType {
        text: text.to_owned(),
        size,
        kind: Kind::Int { signed },
    }
}

fn pointer(text: &str) -> CType {
    CType {
        text: text.to_owned(),
        size: 4,
        kind: Kind::Pointer { function: false },
    }
}

fn member(name: &str, ty: CType, offset: u32) -> Member {
    Member {
        name: name.to_owned(),
        ty,
        offset,
        bits: None,
    }
}

fn bitfield(name: &str, ty: CType, bit_offset: u32, bit_size: u32) -> Member {
    Member {
        name: name.to_owned(),
        ty,
        offset: bit_offset / 8,
        bits: Some(BitField {
            offset: bit_offset,
            size: bit_size,
        }),
    }
}

fn record(text: &str, size: u32, members: Vec<Member>) -> CType {
    CType {
        text: text.to_owned(),
        size,
        kind: Kind::Struct { members },
    }
}

fn fields_of(value: &StateValue) -> &BTreeMap<String, StateValue> {
    match value {
        StateValue::Struct { fields, .. } => fields,
        other => panic!("not a struct: {other:?}"),
    }
}

fn words(values: &[u32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

#[test]
fn eqv_04_decode_classifies_every_region() {
    let regions = Regions::new()
        .with_region("in", 0x0400_1000, 16)
        .with_region("out", 0x0500_0ff0, 16)
        .with_region("pkt", 0x0600_0f00, 64)
        .with_region("heap", 0x0010_0000, 4096)
        .with_region("global:src/x.o:counter", 0x0020_0000, 4)
        .with_function(0x0001_0000, "handler");

    let names = [
        "null", "in", "out_end", "state", "heap", "global", "function", "wild", "past",
    ];
    let values = [
        0u32,
        0x0400_1005,
        0x0500_1000, // one past the end of `out`
        0x0600_0f03,
        0x0010_0008,
        0x0020_0000,
        0x0001_0001, // the function with the Thumb bit set
        0x0030_0000,
        0x0400_1011, // beyond `in` by one
    ];
    let ty = record(
        "struct refs",
        36,
        names
            .iter()
            .enumerate()
            .map(|(i, name)| member(name, pointer("char *"), u32::try_from(i).unwrap() * 4))
            .collect(),
    );
    let decoded = decode_value(&ty, &words(&values), &regions).unwrap();
    let fields = fields_of(&decoded);
    let pointer_of = |name: &str| match &fields[name] {
        StateValue::Pointer { pointer, .. } => pointer.clone(),
        other => panic!("{name}: {other:?}"),
    };
    assert_eq!(pointer_of("null"), Pointer::Null);
    assert_eq!(
        pointer_of("in"),
        Pointer::Buffer {
            region: "in".to_owned(),
            offset: 5
        }
    );
    assert_eq!(
        pointer_of("out_end"),
        Pointer::Buffer {
            region: "out".to_owned(),
            offset: 16
        }
    );
    assert_eq!(
        pointer_of("state"),
        Pointer::Buffer {
            region: "pkt".to_owned(),
            offset: 3
        }
    );
    assert_eq!(
        pointer_of("heap"),
        Pointer::Buffer {
            region: "heap".to_owned(),
            offset: 8
        }
    );
    assert_eq!(
        pointer_of("global"),
        Pointer::Buffer {
            region: "global:src/x.o:counter".to_owned(),
            offset: 0
        }
    );
    assert_eq!(
        pointer_of("function"),
        Pointer::Symbol("handler".to_owned())
    );
    // Any other value is an unobserved mark with the fixed reason, never an address.
    for name in ["wild", "past"] {
        assert_eq!(
            fields[name],
            StateValue::Unobserved {
                c_type: "char *".to_owned(),
                reason: UNRESOLVED_POINTER_REASON.to_owned()
            },
            "{name}"
        );
    }
    assert_eq!(UNRESOLVED_POINTER_REASON, "pointer outside known regions");
    assert_eq!(classify_pointer(0x0030_0000, &regions), None);
    assert_eq!(decoded.unobserved_count(), 2);
}

#[test]
fn eqv_04_decode_union_records_every_member() {
    let u32_ty = int("unsigned int", 4, false);
    let bytes_ty = CType {
        text: "unsigned char[4]".to_owned(),
        size: 4,
        kind: Kind::Array {
            elem: Box::new(int("unsigned char", 1, false)),
            count: 4,
        },
    };
    let union = CType {
        text: "union u".to_owned(),
        size: 4,
        kind: Kind::Union {
            members: vec![member("word", u32_ty, 0), member("bytes", bytes_ty, 0)],
        },
    };
    let decoded = decode_value(&union, &[1, 2, 3, 4], &Regions::new()).unwrap();
    let StateValue::Struct { c_type, fields } = &decoded else {
        panic!("a union decodes to a struct-shaped value: {decoded:?}");
    };
    assert_eq!(c_type, "union u");
    assert_eq!(
        fields["word"],
        StateValue::Int {
            c_type: "unsigned int".to_owned(),
            value: 0x0403_0201
        }
    );
    assert_eq!(
        fields["bytes"],
        StateValue::Bytes {
            c_type: "unsigned char[4]".to_owned(),
            hex: "01020304".to_owned()
        }
    );
}

#[test]
fn eqv_04_decode_bitfields_and_signed_values() {
    let ty = record(
        "struct mix",
        40,
        vec![
            member("neg", int("int8_t", 1, true), 0),
            bitfield("s", int("int", 4, true), 8, 3),
            bitfield("u", int("unsigned int", 4, false), 14, 4),
            member(
                "kind",
                CType {
                    text: "enum k".to_owned(),
                    size: 1,
                    kind: Kind::Enum { signed: false },
                },
                3,
            ),
            member("big", int("uint64_t", 8, false), 8),
            member("small", int("int64_t", 8, true), 16),
            member(
                "f",
                CType {
                    text: "float".to_owned(),
                    size: 4,
                    kind: Kind::Float,
                },
                24,
            ),
            member(
                "d",
                CType {
                    text: "double".to_owned(),
                    size: 8,
                    kind: Kind::Float,
                },
                32,
            ),
        ],
    );
    let mut bytes = vec![0u8; 40];
    bytes[0] = 0xff; // int8_t -1
    bytes[1] = 0b1100_0101; // s = 0b101 (-3 in 3 bits); u's low two bits are 11
    bytes[2] = 0b0000_0001; // u's high two bits are 01, so u = 0b0111
    bytes[3] = 2; // the enum value
    bytes[8..16].copy_from_slice(&u64::MAX.to_le_bytes());
    bytes[16..24].copy_from_slice(&(-2i64).to_le_bytes());
    bytes[24..28].copy_from_slice(&1.5f32.to_le_bytes());
    bytes[32..40].copy_from_slice(&1.0f64.to_le_bytes());
    let decoded = decode_value(&ty, &bytes, &Regions::new()).unwrap();
    let fields = fields_of(&decoded);
    let int_of = |name: &str| match &fields[name] {
        StateValue::Int { value, .. } => *value,
        other => panic!("{name}: {other:?}"),
    };
    assert_eq!(int_of("neg"), -1, "a signed 8-bit -1 is sign-extended");
    assert_eq!(int_of("s"), -3, "a signed bitfield is sign-extended");
    assert_eq!(int_of("u"), 7, "a bitfield may span two bytes");
    assert_eq!(int_of("kind"), 2, "a 1-byte enum is an integer of its size");
    let wide = |name: &str| match &fields[name] {
        StateValue::Wide { value, .. } => value.clone(),
        other => panic!("{name}: {other:?}"),
    };
    assert_eq!(wide("big"), "18446744073709551615");
    assert_eq!(wide("small"), "-2");
    let bits = |name: &str| match &fields[name] {
        StateValue::Float { bits, .. } => bits.clone(),
        other => panic!("{name}: {other:?}"),
    };
    assert_eq!(bits("f"), "3fc00000");
    assert_eq!(bits("d"), "3ff0000000000000");
    // The decoded state is a valid library state.
    LibState::new(BTreeMap::from([("mix".to_owned(), decoded)])).unwrap();
}

fn packet_type() -> CType {
    let buffer = record(
        "coap_buffer_t",
        8,
        vec![
            member("p", pointer("const uint8_t *"), 0),
            member("len", int("size_t", 4, false), 4),
        ],
    );
    let option = record(
        "coap_option_t",
        12,
        vec![
            member("num", int("uint8_t", 1, false), 0),
            member("buf", buffer.clone(), 4),
        ],
    );
    record(
        "coap_packet_t",
        8 + 24,
        vec![
            member("payload", buffer, 0),
            member(
                "opts",
                CType {
                    text: "coap_option_t[2]".to_owned(),
                    size: 24,
                    kind: Kind::Array {
                        elem: Box::new(option),
                        count: 2,
                    },
                },
                8,
            ),
        ],
    )
}

fn mark(field: &str, reason: &str) -> UnobservedMark {
    UnobservedMark {
        field: field.to_owned(),
        reason: reason.to_owned(),
    }
}

#[test]
fn eqv_04_unobserved_marks_replace_fields_and_are_counted() {
    let ty = packet_type();
    let regions = Regions::new().with_region("in", 0x0400_0000, 16);
    let mut bytes = vec![0u8; 32];
    bytes[0..4].copy_from_slice(&0x0400_0004u32.to_le_bytes()); // payload.p
    bytes[4..8].copy_from_slice(&3u32.to_le_bytes()); // payload.len
    bytes[8 + 12 + 4..8 + 12 + 8].copy_from_slice(&0x0300_0000u32.to_le_bytes()); // opts[1].buf.p, wild
    let decoded = decode_value(&ty, &bytes, &regions).unwrap();
    // One automatic mark: the wild pointer.
    assert_eq!(decoded.unobserved_count(), 1);

    let marked = apply_marks(
        "pkt",
        decoded.clone(),
        &[
            mark("pkt.payload", "points into a buffer the caller owns"),
            mark("pkt.opts[0].num", "not part of the interface"),
            mark("other.thing", "belongs to another state"),
        ],
    )
    .unwrap();
    let fields = fields_of(&marked);
    assert_eq!(
        fields["payload"],
        StateValue::Unobserved {
            c_type: "coap_buffer_t".to_owned(),
            reason: "points into a buffer the caller owns".to_owned()
        },
        "a mark replaces the field with its reason and keeps the type"
    );
    let StateValue::Items { items, .. } = &fields["opts"] else {
        panic!("opts must stay an array");
    };
    assert!(matches!(
        &fields_of(&items[0])["num"],
        StateValue::Unobserved { reason, .. } if reason == "not part of the interface"
    ));
    // The marks are counted with the automatic ones (1 wild pointer + 2 marks).
    let state = LibState::new(BTreeMap::from([("pkt".to_owned(), marked)])).unwrap();
    assert_eq!(state.unobserved_count(), 3);

    // A mark is checked against the layout: an unknown member or index is an
    // error that names it.
    let layouts = BTreeMap::from([("pkt".to_owned(), ty)]);
    check_marks(
        &layouts,
        &[mark("pkt.payload", "r"), mark("pkt.opts[1].buf.p", "r")],
    )
    .unwrap();
    let error = check_marks(&layouts, &[mark("pkt.nope", "r")]).unwrap_err();
    assert!(error.contains("nope"), "{error}");
    let error = check_marks(&layouts, &[mark("pkt.opts[2]", "r")]).unwrap_err();
    assert!(error.contains("opts[2]"), "{error}");
    let error = check_marks(&layouts, &[mark("pkt.payload.p.x", "r")]).unwrap_err();
    assert!(error.contains("payload.p.x"), "{error}");
    let error = check_marks(&layouts, &[mark("ghost.x", "r")]).unwrap_err();
    assert!(error.contains("ghost"), "{error}");
}

fn counter_plan() -> Vec<DumpRange> {
    vec![DumpRange {
        key: "x.o:counter".to_owned(),
        address: 0x0020_0000,
        size: 4,
        initial: vec![5, 0, 0, 0],
    }]
}

#[test]
fn eqv_04_changed_global_is_recorded_typed() {
    let plan = counter_plan();
    let typed = BTreeMap::from([(
        "x.o:counter".to_owned(),
        GlobalType::Typed {
            ty: int("int", 4, true),
        },
    )]);
    let regions = Regions::new();
    let initial: &[u8] = &[5, 0, 0, 0];
    let changed: &[u8] = &[8, 0, 0, 0];

    // Unchanged: nothing is recorded.
    assert_eq!(
        record_globals(&plan, &typed, &[initial], &[initial], &regions).unwrap(),
        None
    );

    // Changed by the call: both sides record the global, decoded with its type.
    let (before, after) = record_globals(&plan, &typed, &[initial], &[changed], &regions)
        .unwrap()
        .expect("a changed global is recorded");
    let int_value = |value: &StateValue| match &fields_of(value)["x.o:counter"] {
        StateValue::Int { c_type, value } => (c_type.clone(), *value),
        other => panic!("{other:?}"),
    };
    assert_eq!(int_value(&before), ("int".to_owned(), 5));
    assert_eq!(int_value(&after), ("int".to_owned(), 8));
    // The pair is the shape LibState accepts under `globals`.
    LibState::new(BTreeMap::from([("globals".to_owned(), after)])).unwrap();

    // Changed and untyped: executor_error untyped_global_changed, naming it.
    for types in [
        BTreeMap::new(),
        BTreeMap::from([(
            "x.o:counter".to_owned(),
            GlobalType::Untyped {
                reason: "no unique variable".to_owned(),
            },
        )]),
    ] {
        let reason = record_globals(&plan, &types, &[initial], &[changed], &regions).unwrap_err();
        assert!(reason.contains("untyped_global_changed"), "{reason}");
        assert!(reason.contains("x.o:counter"), "{reason}");
    }
    // Untyped but unchanged is fine.
    assert_eq!(
        record_globals(&plan, &BTreeMap::new(), &[initial], &[initial], &regions).unwrap(),
        None
    );
}

const HEAP_BASE: u32 = 0x0010_0000;

fn heap_pointer(c_type: &str, offset: u64) -> StateValue {
    StateValue::Pointer {
        c_type: c_type.to_owned(),
        pointer: Pointer::Buffer {
            region: "heap".to_owned(),
            offset,
        },
    }
}

/// `struct node { u32 value; struct node *next; char *name; }`, 12 bytes.
fn node_type() -> CType {
    record(
        "struct node",
        12,
        vec![
            member("value", int("unsigned int", 4, false), 0),
            member("next", pointer("struct node *"), 4),
            member("name", pointer("char *"), 8),
        ],
    )
}

fn node_pointees() -> BTreeMap<String, CType> {
    BTreeMap::from([
        ("struct node".to_owned(), node_type()),
        ("char".to_owned(), int("char", 1, true)),
    ])
}

#[test]
fn eqv_04_heap_walk_follows_typed_edges() {
    // D-06, D-09: objects reachable from recorded pointers through typed edges
    // are recorded by decimal offset; a char pointee is a NUL-terminated run; a
    // cycle is decoded once.
    let mut arena = Vec::new();
    arena.extend(words(&[7, HEAP_BASE + 12, HEAP_BASE + 24]));
    arena.extend(words(&[9, HEAP_BASE, 0]));
    arena.extend(b"hi\0\0");
    let regions = Regions::new()
        .with_region("heap", HEAP_BASE, u32::try_from(arena.len()).unwrap())
        .with_region("in", 0x0400_0000, 8);
    let mut root = heap_pointer("struct node *", 0);
    let mut other = StateValue::Pointer {
        c_type: "struct node *".to_owned(),
        pointer: Pointer::Buffer {
            region: "in".to_owned(),
            offset: 4,
        },
    };
    let objects = walk_heap(
        &arena,
        &node_pointees(),
        &regions,
        &mut [&mut root, &mut other],
    )
    .unwrap();
    let keys: Vec<&str> = objects.keys().map(String::as_str).collect();
    assert_eq!(
        keys,
        ["0", "12", "24"],
        "three objects, the cycle decoded once"
    );

    let first = fields_of(&objects["0"]);
    assert!(matches!(&first["value"], StateValue::Int { value: 7, .. }));
    assert_eq!(
        first["next"],
        heap_pointer("struct node *", 12),
        "the next pointer is a region offset"
    );
    assert_eq!(first["name"], heap_pointer("char *", 24));
    let second = fields_of(&objects["12"]);
    assert_eq!(second["next"], heap_pointer("struct node *", 0));
    assert!(matches!(
        &second["name"],
        StateValue::Pointer {
            pointer: Pointer::Null,
            ..
        }
    ));
    assert_eq!(
        objects["24"],
        StateValue::Bytes {
            c_type: "char[3]".to_owned(),
            hex: "686900".to_owned()
        },
        "a char pointee is the NUL-terminated bytes in hex"
    );
    // The roots keep their classified pointers; one into `in` is not walked.
    assert_eq!(root, heap_pointer("struct node *", 0));
    assert!(
        matches!(&other, StateValue::Pointer { pointer: Pointer::Buffer { region, .. }, .. } if region == "in")
    );

    // A pointer past the used bytes, to an unknown type or to a void pointee,
    // stays a classified pointer with no object.
    let mut unknown = heap_pointer("struct unknown *", 0);
    let mut beyond = heap_pointer("struct node *", 1000);
    let mut nothing = heap_pointer("void *", 0);
    let objects = walk_heap(
        &arena,
        &node_pointees(),
        &regions,
        &mut [&mut unknown, &mut beyond, &mut nothing],
    )
    .unwrap();
    assert!(objects.is_empty(), "{objects:?}");
}

#[test]
fn eqv_04_heap_walk_cap_is_an_unobserved_mark() {
    // T-03-22: the walk stops after HEAP_WALK_CAP objects, and the first pointer
    // not followed is recorded as unobserved with the cap's reason.
    assert_eq!(HEAP_WALK_CAP, 4096);
    let nodes = HEAP_WALK_CAP + 4;
    let node = record(
        "struct chain",
        8,
        vec![
            member("next", pointer("struct chain *"), 0),
            member("pad", int("unsigned int", 4, false), 4),
        ],
    );
    let mut arena = Vec::new();
    for index in 0..nodes {
        let next = if index + 1 < nodes {
            HEAP_BASE + u32::try_from((index + 1) * 8).unwrap()
        } else {
            0
        };
        arena.extend(words(&[next, 0]));
    }
    let regions =
        Regions::new().with_region("heap", HEAP_BASE, u32::try_from(arena.len()).unwrap());
    let pointees = BTreeMap::from([("struct chain".to_owned(), node)]);
    let mut root = heap_pointer("struct chain *", 0);
    let objects = walk_heap(&arena, &pointees, &regions, &mut [&mut root]).unwrap();
    assert_eq!(objects.len(), HEAP_WALK_CAP);
    let last = (HEAP_WALK_CAP - 1) * 8;
    let before_last = fields_of(&objects[&(last - 8).to_string()]);
    assert_eq!(
        before_last["next"],
        heap_pointer("struct chain *", u64::try_from(last).unwrap()),
        "the pointer to the last object is followed"
    );
    let capped = fields_of(&objects[&last.to_string()]);
    assert_eq!(
        capped["next"],
        StateValue::Unobserved {
            c_type: "struct chain *".to_owned(),
            reason: HEAP_CAP_REASON.to_owned()
        },
        "the first pointer not followed is an unobserved mark"
    );
    assert_eq!(HEAP_CAP_REASON, "heap walk cap reached");
}

#[test]
fn eqv_04_struct_return_is_ret() {
    // D-06, D-11: a struct returned by value is stored in a state called `ret`
    // after the declared states; rc is 0 and `ret` is decoded like any struct.
    let arg = |role| ArgRole {
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
    };
    let manifest = mt_capture::manifest::Manifest::load(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testbed/manifest.toml"),
    )
    .unwrap();
    let mut entry = manifest.target["nanopb"]
        .entry
        .iter()
        .find(|entry| entry.name == "pb_istream_from_buffer")
        .unwrap()
        .clone();
    entry.arg = vec![arg(Role::InBuf), arg(Role::InLen)];
    // The typedef name alone does not say it is a struct; the build resolves it
    // from the DWARF of the entry's header.
    let adapter = Adapter::from_entry("nanopb", "default", &entry).unwrap();
    assert_eq!(adapter.return_kind(), ReturnKind::Integer);
    assert!(adapter.states().is_empty());
    let adapter = adapter.with_return_kind(ReturnKind::Struct);
    assert_eq!(adapter.return_kind(), ReturnKind::Struct);
    assert_eq!(
        adapter.call_expression(),
        "(*(pb_istream_t *)mt_state(0) = pb_istream_from_buffer((const pb_byte_t *)mt_in, (size_t)mt_in_len), (int32_t)0)"
    );
    let states = adapter.states();
    assert_eq!(states.len(), 1);
    assert_eq!(states[0].name, "ret");
    assert_eq!(states[0].type_text, "pb_istream_t");
    assert_eq!(states[0].init_word, None);
    assert!(
        adapter
            .states_fragment()
            .contains("(u32)sizeof(pb_istream_t)")
    );

    let stream = record(
        "pb_istream_t",
        16,
        vec![
            member("callback", pointer("bool (*)(...)"), 0),
            member("state", pointer("void *"), 4),
            member("bytes_left", int("size_t", 4, false), 8),
            member("errmsg", pointer("const char *"), 12),
        ],
    );
    let regions = Regions::new()
        .with_region("in", 0x0400_0000, 5)
        .with_function(0x0001_0000, "buf_read");
    let bytes = words(&[0x0001_0001, 0x0400_0000, 5, 0]);
    let ret = decode_value(&stream, &bytes, &regions).unwrap();
    let fields = fields_of(&ret);
    assert!(
        matches!(&fields["callback"], StateValue::Pointer { pointer: Pointer::Symbol(name), .. } if name == "buf_read")
    );
    assert!(
        matches!(&fields["state"], StateValue::Pointer { pointer: Pointer::Buffer { region, offset: 0 }, .. } if region == "in")
    );
    assert!(matches!(
        &fields["bytes_left"],
        StateValue::Int { value: 5, .. }
    ));
    // `ret` is a reserved top-level name that accepts any value.
    let state = LibState::new(BTreeMap::from([("ret".to_owned(), ret)])).unwrap();
    assert_eq!(state.unobserved_count(), 0);
}
