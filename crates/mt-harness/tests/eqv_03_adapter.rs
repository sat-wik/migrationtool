#![allow(clippy::unwrap_used, clippy::expect_used)]
//! EQV-03: the adapter an entry's manifest roles turn into (CONTEXT D-05, D-11,
//! D-14) and the flags a driver is compiled with (D-01). Pure: no tool runs.

use std::path::{Path, PathBuf};

use mt_capture::manifest::{Entry, Manifest};
use mt_harness::adapter::{Adapter, AdapterError, ReturnKind, combinations, driver_flags};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn entry_of(target: &str, name: &str) -> Entry {
    let manifest = Manifest::load(&repo_root().join("testbed/manifest.toml")).unwrap();
    manifest.target[target]
        .entry
        .iter()
        .find(|entry| entry.name == name)
        .unwrap_or_else(|| panic!("{target} has no entry {name}"))
        .clone()
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

#[test]
fn eqv_03_adapter_maps_corejson_validate() {
    let entry = entry_of("corejson", "JSON_Validate");
    let adapter = Adapter::from_entry("corejson", "default", &entry).unwrap();
    assert_eq!(adapter.argv0(), "corejson/default/JSON_Validate");
    assert_eq!(adapter.return_kind(), ReturnKind::Integer);
    assert_eq!(
        adapter.call_expression(),
        "(int32_t)(JSON_Validate((const char *)mt_in, (size_t)mt_in_len))"
    );
    // No scalar and no output buffer: one combination with no value.
    let combos = combinations(&adapter).unwrap();
    assert_eq!(combos.len(), 1);
    assert!(combos[0].values.is_empty());
    // D-05: argv[0] names the entry, then every non-pointer parameter in
    // parameter order; the input length is the stdin byte count.
    assert_eq!(
        adapter.argv(17, &combos[0]),
        ["corejson/default/JSON_Validate", "17"]
    );
    assert_eq!(adapter.output_capacity(&combos[0]), 0);
    assert!(adapter.scalar_words(&combos[0]).is_empty());
    assert_eq!(adapter.max_input(), None);
}

#[test]
fn eqv_03_adapter_refuses_an_entry_without_roles() {
    let entry = entry_of("corejson", "JSON_SearchT");
    assert_eq!(
        Adapter::from_entry("corejson", "default", &entry).unwrap_err(),
        AdapterError::NoRoles
    );
}

#[test]
fn eqv_03_adapter_return_kinds() {
    let mut entry = entry_of("corejson", "JSON_Validate");
    for (signature, kind, call) in [
        (
            "void (const char *, size_t)",
            ReturnKind::Void,
            "((void)(JSON_Validate((const char *)mt_in, (size_t)mt_in_len)), (int32_t)0)",
        ),
        (
            "cJSON *(const char *, size_t)",
            ReturnKind::Pointer,
            "((JSON_Validate((const char *)mt_in, (size_t)mt_in_len)) != 0 ? (int32_t)1 : (int32_t)0)",
        ),
        (
            "JSONStatus_t (const char *, size_t)",
            ReturnKind::Integer,
            "(int32_t)(JSON_Validate((const char *)mt_in, (size_t)mt_in_len))",
        ),
    ] {
        entry.signature = signature.to_owned();
        let adapter = Adapter::from_entry("corejson", "default", &entry).unwrap();
        assert_eq!(adapter.return_kind(), kind, "{signature}");
        assert_eq!(adapter.call_expression(), call, "{signature}");
    }
    // A return the 32-bit rc cannot hold is refused, not truncated.
    for signature in [
        "double (const char *, size_t)",
        "float (const char *, size_t)",
    ] {
        entry.signature = signature.to_owned();
        assert!(
            matches!(
                Adapter::from_entry("corejson", "default", &entry),
                Err(AdapterError::UnsupportedReturn(_))
            ),
            "{signature}"
        );
    }
}

#[test]
fn eqv_03_driver_flags_keep_include_define_and_abi_flags() {
    let arguments = strings(&[
        "/opt/arm/bin/arm-none-eabi-gcc",
        "-I/work/src/include",
        "-I",
        "rel/inc",
        "-isystem",
        "/work/sys",
        "-iquote",
        "/work/q",
        "-include",
        "pre.h",
        "-DNDEBUG",
        "-D",
        "JSON_MAX_DEPTH=8",
        "-UFOO",
        "-mcpu=cortex-m4",
        "-mthumb",
        "-mfloat-abi=hard",
        "-mfpu=fpv4-sp-d16",
        "-Os",
        "-Os",
        "-O3",
        "-g",
        "-fPIC",
        "-fpic",
        "-std=c89",
        "-fstack-protector-strong",
        "-fvisibility=hidden",
        "-ffunction-sections",
        "-fdata-sections",
        "-pedantic",
        "-Wall",
        "-Werror=format-security",
        "-MD",
        "-MMD",
        "-MP",
        "-MF",
        "x.d",
        "-MT",
        "x.o",
        "-fshort-enums",
        "-funsigned-char",
        "-fpack-struct=2",
        "-o",
        "CMakeFiles/x.o",
        "-c",
        "/work/src/x.c",
    ]);
    assert_eq!(
        driver_flags(&arguments).unwrap(),
        strings(&[
            "-I/work/src/include",
            "-I",
            "rel/inc",
            "-isystem",
            "/work/sys",
            "-iquote",
            "/work/q",
            "-include",
            "pre.h",
            "-DNDEBUG",
            "-D",
            "JSON_MAX_DEPTH=8",
            "-UFOO",
            "-fshort-enums",
            "-funsigned-char",
            "-fpack-struct=2",
        ])
    );
}

#[test]
fn eqv_03_driver_flags_fail_closed_on_an_unhandled_option() {
    for flag in [
        "-ffreestanding",
        "-fno-builtin",
        "-mno-unaligned-access",
        "-mcpu=cortex-m7",
        "-mfpu=vfp",
    ] {
        let arguments = strings(&["gcc", "-mthumb", flag, "-c", "x.c"]);
        assert_eq!(
            driver_flags(&arguments).unwrap_err(),
            AdapterError::UnhandledFlag(flag.to_owned()),
            "{flag}"
        );
    }
    // The operand of an option that takes one is never read as an option.
    let arguments = strings(&[
        "gcc",
        "-D",
        "-ffreestanding",
        "-o",
        "-fno-builtin",
        "-c",
        "x.c",
    ]);
    assert_eq!(
        driver_flags(&arguments).unwrap(),
        strings(&["-D", "-ffreestanding"])
    );
}
