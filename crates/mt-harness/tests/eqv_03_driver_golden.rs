#![allow(clippy::unwrap_used, clippy::expect_used)]
//! EQV-03: the generated drivers are pinned byte for byte (CONTEXT D-11,
//! T-03-14). The include lines are fixed by the test; everything else comes from
//! the shared template and the entry's roles in the committed manifest.
//!
//! A change of the template or of the roles changes a golden file; the diff is
//! the review. `MT_BLESS_GOLDEN=1 cargo test -p mt-harness --test
//! eqv_03_driver_golden` rewrites the files.

use std::path::{Path, PathBuf};

use mt_capture::manifest::Manifest;
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
