//! Static TOOL-02 and TOOL-03 checks over the real `container/pins.toml`.
//!
//! These run under plain `cargo test` with no container and no network
//! (CONTEXT D-26): the committed pins must be well formed, the provisional LLVM
//! and bitcode-rustc pins must be backed by a matching PROJECT.md decision
//! (CONTEXT D-21), `rust-toolchain.toml` must agree with the tool toolchain pin,
//! and the version each tool is expected to print must agree with the source it
//! is built from.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use mt_toolchain::pins::{LlvmStatus, Pins, SourcePin, ToolPinStatus};

/// The repository root: two levels above this crate's manifest directory.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// Read a repository file, failing the test (never skipping) when it is absent.
fn read_repo_file(relative: &str) -> String {
    let path = repo_root().join(relative);
    fs::read_to_string(&path).unwrap_or_else(|err| panic!("cannot read {}: {err}", path.display()))
}

/// The committed container pins, parsed but not yet validated.
fn container_pins() -> Pins {
    Pins::parse(&read_repo_file("container/pins.toml")).unwrap()
}

/// The `- **D-NN:**` line of a PROJECT.md decisions block, if present.
fn decision_line<'a>(project: &'a str, id: &str) -> Option<&'a str> {
    let prefix = format!("- **{id}:**");
    project.lines().find(|line| line.starts_with(&prefix))
}

/// True when `line` carries `token` as a whole whitespace-separated word.
fn has_token(line: &str, token: &str) -> bool {
    line.split_whitespace()
        .map(|word| word.trim_end_matches([',', ';', '.', ')']))
        .any(|word| word == token)
}

/// Every way the LLVM and bitcode-rustc pins fail to match their decisions in
/// `project` (the text of PROJECT.md). Empty when the gate passes (CONTEXT D-21).
fn decision_gate_issues(pins: &Pins, project: &str) -> Vec<String> {
    let mut issues = Vec::new();
    let checks = [
        (
            "llvm.decision",
            pins.llvm.decision.as_str(),
            format!("llvm={}", pins.llvm.major),
        ),
        (
            "rust.bitcode.decision",
            pins.rust.bitcode.decision.as_str(),
            format!("bitcode-rustc={}", pins.rust.bitcode.version),
        ),
    ];
    for (field, id, token) in checks {
        match decision_line(project, id) {
            None => issues.push(format!("{field}: PROJECT.md has no `- **{id}:**` line")),
            Some(line) if !has_token(line, &token) => {
                issues.push(format!("{field}: decision {id} does not carry `{token}`"));
            }
            Some(_) => {}
        }
    }
    issues
}

/// Sorted copy of a name list, for set comparison.
fn as_set(names: &[String]) -> BTreeSet<&str> {
    names.iter().map(String::as_str).collect()
}

#[test]
fn tool_03_container_pins_are_well_formed() {
    let pins = container_pins();
    pins.validate().unwrap();
}

#[test]
fn tool_02_container_pins_llvm_major_in_range_and_provisional() {
    let pins = container_pins();
    assert!(
        (16..=19).contains(&pins.llvm.major),
        "llvm.major {} is outside 16..=19",
        pins.llvm.major
    );
    assert_eq!(pins.llvm.max_major, 19);
    assert_eq!(
        pins.llvm.status,
        LlvmStatus::Provisional,
        "the pin stays provisional until the Phase 6 R1 verdict"
    );
}

#[test]
fn tool_02_llvm_and_bitcode_pins_have_matching_project_decision() {
    let pins = container_pins();
    let project = read_repo_file(".planning/PROJECT.md");
    assert_eq!(
        decision_gate_issues(&pins, &project),
        Vec::<String>::new(),
        "the pins and the PROJECT.md decision lines disagree"
    );

    // Changing the bitcode rustc without a new decision must fail the gate.
    let mut moved = pins.clone();
    moved.rust.bitcode.version = "1.73.0".to_owned();
    let issues = decision_gate_issues(&moved, &project);
    assert!(
        issues.iter().any(|i| i.contains("bitcode-rustc=1.73.0")),
        "bitcode 1.73.0 must fail the gate, got {issues:?}"
    );

    // So must changing the LLVM major.
    let mut moved = pins.clone();
    moved.llvm.major = 17;
    let issues = decision_gate_issues(&moved, &project);
    assert!(
        issues.iter().any(|i| i.contains("llvm=17")),
        "llvm 17 must fail the gate, got {issues:?}"
    );

    // A decision ID with no line in PROJECT.md fails, and so does an empty file.
    let mut unknown = pins.clone();
    unknown.llvm.decision = "D-99".to_owned();
    assert!(!decision_gate_issues(&unknown, &project).is_empty());
    assert!(!decision_gate_issues(&pins, "").is_empty());
}

#[test]
fn tool_03_rust_toolchain_file_matches_pins() {
    let pins = container_pins();
    let text = read_repo_file("rust-toolchain.toml");
    let file: toml::Table = toml::from_str(&text).unwrap();
    let toolchain = file["toolchain"].as_table().unwrap();
    let strings = |key: &str| -> Vec<String> {
        toolchain[key]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_owned())
            .collect()
    };
    assert_eq!(
        toolchain["channel"].as_str().unwrap(),
        pins.rust.tool.version,
        "rust-toolchain.toml channel must equal [rust.tool].version"
    );
    assert_eq!(
        as_set(&strings("components")),
        as_set(&pins.rust.tool.components),
        "rust-toolchain.toml components must equal [rust.tool].components"
    );
    assert_eq!(
        as_set(&strings("targets")),
        as_set(&pins.rust.tool.targets),
        "rust-toolchain.toml targets must equal [rust.tool].targets"
    );
}

#[test]
fn tool_03_tool_expectations_agree_with_sources() {
    let pins = container_pins();
    let expect = |tool: &str| -> &str { &pins.tool[tool].expect };

    let SourcePin::Git { tag, .. } = &pins.source["klee"] else {
        panic!("source.klee must be a git source");
    };
    assert_eq!(
        tag.strip_prefix('v'),
        Some(expect("klee")),
        "klee expect must equal the source tag without its leading v"
    );

    for (tool, source) in [("c2rust", "c2rust"), ("cargo_mutants", "cargo_mutants")] {
        let SourcePin::Crate { version, .. } = &pins.source[source] else {
            panic!("source.{source} must be a crate source");
        };
        assert_eq!(expect(tool), version, "{tool} expect must equal its crate");
    }

    let SourcePin::Archive { version, .. } = &pins.source["arm_gnu"] else {
        panic!("source.arm_gnu must be an archive source");
    };
    assert_eq!(
        expect("arm_gcc").to_lowercase(),
        version.to_lowercase(),
        "arm_gcc expect must equal the Arm archive version"
    );

    let SourcePin::Archive { version, .. } = &pins.source["rustup"] else {
        panic!("source.rustup must be an archive source");
    };
    assert_eq!(
        expect("rustup"),
        version,
        "rustup expect must equal rustup-init"
    );

    for (key, spec) in &pins.tool {
        if spec.status != ToolPinStatus::Installed {
            continue;
        }
        if let Some(value) = spec.env.get("RUSTUP_HOME") {
            assert_eq!(value, &pins.rust.rustup_home, "tool.{key}.env RUSTUP_HOME");
        }
        if let Some(value) = spec.env.get("CARGO_HOME") {
            assert_eq!(value, &pins.rust.cargo_home, "tool.{key}.env CARGO_HOME");
        }
    }
}
