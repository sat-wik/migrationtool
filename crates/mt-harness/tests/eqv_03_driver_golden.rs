#![allow(clippy::unwrap_used, clippy::expect_used)]
//! EQV-03: the generated drivers are pinned byte for byte (CONTEXT D-11,
//! T-03-14). The include lines are fixed by the test; everything else comes from
//! the shared template and the entry's roles in the committed manifest.
//!
//! A change of the template or of the roles changes a golden file; the diff is
//! the review. `MT_BLESS_GOLDEN=1 cargo test -p mt-harness --test
//! eqv_03_driver_golden` rewrites the files.

use std::path::{Path, PathBuf};

use mt_capture::manifest::{ArgRole, Entry, LocalObject, Manifest, Role, SetupStep};
use mt_harness::adapter::{Adapter, AdapterError};
use mt_harness::driver::render_entry_driver;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn golden_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/golden")
        .join(name)
}

fn adapter_of(target: &str, entry: &str) -> Adapter {
    let manifest = Manifest::load(&repo_root().join("testbed/manifest.toml")).unwrap();
    let entry = manifest.target[target]
        .entry
        .iter()
        .find(|candidate| candidate.name == entry)
        .unwrap();
    Adapter::from_entry(target, "default", entry).unwrap()
}

fn lines(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

#[test]
fn eqv_03_driver_golden() {
    let cases = [
        (
            "corejson",
            "JSON_Validate",
            "corejson-JSON_Validate.driver.c",
            lines(&[
                "#include <stddef.h>",
                "#include <stdint.h>",
                "#include \"/fixed/src/include/core_json.h\"",
            ]),
        ),
        (
            "lz4",
            "LZ4_decompress_safe",
            "lz4-LZ4_decompress_safe.driver.c",
            lines(&["#include \"/fixed/lib/lz4.h\""]),
        ),
        (
            "microcoap",
            "coap_parse",
            "microcoap-coap_parse.driver.c",
            lines(&[
                "#include <stddef.h>",
                "#include <stdint.h>",
                "#include \"/fixed/coap.h\"",
            ]),
        ),
        (
            "zlib",
            "uncompress",
            "zlib-uncompress.driver.c",
            lines(&["#include \"/fixed/zlib.h\""]),
        ),
        // D-12: a declared setup step (http_parser_init) before the entry.
        (
            "httpparser",
            "http_parser_execute",
            "httpparser-http_parser_execute.driver.c",
            lines(&[
                "#include <stddef.h>",
                "#include <stdint.h>",
                "#include \"/fixed/http_parser.h\"",
            ]),
        ),
    ];
    for (target, entry, file, include_lines) in cases {
        let source = render_entry_driver(&adapter_of(target, entry), &include_lines).unwrap();
        let path = golden_path(file);
        if std::env::var_os("MT_BLESS_GOLDEN").is_some() {
            std::fs::write(&path, &source).unwrap();
        }
        let golden = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        assert!(
            source == golden,
            "{target}/{entry}: the generated driver differs from {}",
            path.display()
        );
        assert!(!source.contains("@MT_"), "no placeholder is left");
    }
}

fn role_of(role: Role, name: Option<&str>) -> ArgRole {
    ArgRole {
        role,
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

fn set_step(path: &str, value: &str) -> SetupStep {
    SetupStep {
        call: None,
        args: Vec::new(),
        expect: None,
        set: Some(path.to_owned()),
        value: Some(value.to_owned()),
    }
}

/// A test-only declaration shaped like coreMQTT's `MQTT_DeserializeAck`: the packet
/// information is a state whose fields are set from the input, and a local array
/// holds a second one. The committed manifest does not declare it (its packets are
/// built by the library's own framing); the golden pins the `set` rendering that
/// later plans' declarations use.
fn coremqtt_set_step_entry(value_of_remaining: &str) -> Entry {
    let manifest = Manifest::load(&repo_root().join("testbed/manifest.toml")).unwrap();
    let mut entry = manifest.target["coremqtt"]
        .entry
        .iter()
        .find(|candidate| candidate.name == "MQTT_DeserializeAck")
        .unwrap()
        .clone();
    entry.arg = vec![
        role_of(Role::State, Some("packet")),
        role_of(Role::OutScalar, Some("packet_id")),
        role_of(Role::State, Some("reason")),
        role_of(Role::State, Some("props")),
        role_of(Role::State, Some("conn")),
    ];
    entry.local = vec![LocalObject {
        name: "scratch".to_owned(),
        type_text: "MQTTPacketInfo_t".to_owned(),
        count: Some(2),
    }];
    entry.setup = vec![
        set_step("packet.type", "64"),
        set_step("packet.pRemainingData", "$in"),
        set_step("packet.remainingLength", value_of_remaining),
        set_step("scratch.pRemainingData", "$in+1"),
        set_step("scratch.remainingLength", "$len-1"),
    ];
    entry
}

#[test]
fn eqv_03_driver_golden_set_step_is_rendered_and_closed() {
    // D-12: field assignments from the input pointer and length, on a state and on a
    // local array, between the driver's allocation and its snapshot.
    let entry = coremqtt_set_step_entry("$len");
    let adapter = Adapter::from_entry("coremqtt", "default", &entry).unwrap();
    let source = render_entry_driver(
        &adapter,
        &lines(&[
            "#include <stddef.h>",
            "#include <stdint.h>",
            "#include \"/fixed/core_mqtt_serializer.h\"",
        ]),
    )
    .unwrap();
    let path = golden_path("coremqtt-set-step.driver.c");
    if std::env::var_os("MT_BLESS_GOLDEN").is_some() {
        std::fs::write(&path, &source).unwrap();
    }
    let golden = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    assert!(
        source == golden,
        "the generated driver differs from {}",
        path.display()
    );
    assert!(!source.contains("@MT_"), "no placeholder is left");
    // Locals are states after the parameters' own; the array local is one mapping.
    assert!(
        source.contains("sizeof(MQTTPacketInfo_t) * 2u"),
        "a counted local is sized by its count"
    );

    // The value grammar is closed: nothing else is rendered.
    for bad in ["$in*2", "$len-x", "$in+", "1+1", "$cap"] {
        let result = Adapter::from_entry("coremqtt", "default", &coremqtt_set_step_entry(bad));
        assert!(
            matches!(result, Err(AdapterError::Invalid(ref why)) if why.contains("value")),
            "{bad:?}: {result:?}"
        );
    }
}

#[test]
fn eqv_03_driver_names_never_reach_rendering() {
    // The validator refuses a name that is not a C identifier before anything
    // is rendered.
    let text = std::fs::read_to_string(repo_root().join("testbed/manifest.toml")).unwrap();
    let edited = text.replacen(
        "name = \"JSON_Validate\"",
        "name = \"JSON_Validate; int evil\"",
        1,
    );
    assert_ne!(edited, text);
    let manifest = Manifest::parse(&edited).unwrap();
    let error = manifest.validate().unwrap_err().to_string();
    assert!(error.contains("C identifier"), "{error}");

    // And the adapter refuses it too, for a caller that skipped the validator.
    let mut entry = manifest.target["corejson"].entry[0].clone();
    entry.name = "JSON_Validate; int evil".to_owned();
    assert!(matches!(
        Adapter::from_entry("corejson", "default", &entry),
        Err(AdapterError::Invalid(_))
    ));

    // An include line is one directive and nothing else.
    let adapter = adapter_of("corejson", "JSON_Validate");
    for bad in [
        "#include <a.h>\nint evil;",
        "int evil;",
        "#pragma once",
        "#define",
        "  #include <a.h>\r",
    ] {
        let result = render_entry_driver(&adapter, &lines(&[bad]));
        assert!(result.is_err(), "{bad:?} must be refused");
    }
    assert!(render_entry_driver(&adapter, &lines(&["#include <a.h>"])).is_ok());
}
