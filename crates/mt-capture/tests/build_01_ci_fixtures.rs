#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Offline tests over the fixtures captured from CI (CONTEXT D-26).
//!
//! `tests/fixtures/ci/` holds files that the `testbed` workflow published as
//! `fixture ...` notices from the pinned toolchain image, copied verbatim; see
//! `tests/fixtures/ci/SOURCE.md` for the run, the head commit, the image digest
//! and the notice title of each file. They are never edited by hand: a failing
//! test here is evidence about the code or about the run.
//!
//! A notice drops the final newline of the file it carries, so the text
//! fixtures lack the trailing newline that `to_json` writes. The round-trip
//! tests therefore compare the serialised form with the fixture plus one
//! newline. Plain `cargo test`: no Docker, no network, no external process.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use mt_capture::cc::{CcEntry, to_json};
use mt_capture::elf::{
    check_elf_header, code_digest, differing_sections, normalisable, parse_readelf_a,
};
use mt_capture::entries::{normalise_signature, parse_ast_stream};
use mt_capture::manifest::Manifest;
use mt_capture::report::{CaptureReport, CaptureVerdict, ReplayReport, Verdict};

/// The only compiler a capture in the image may name.
const PINNED_COMPILER: &str = "/opt/arm-gnu-toolchain/bin/arm-none-eabi-gcc";

/// The run the fixtures came from (plan 02-06 Task 2, testbed job 115239376216).
const FIXTURE_RUN_ID: &str = "38380382970";

fn ci_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/ci")
        .join(name)
}

fn ci_text(name: &str) -> String {
    fs::read_to_string(ci_path(name)).unwrap()
}

fn ci_bytes(name: &str) -> Vec<u8> {
    fs::read(ci_path(name)).unwrap()
}

#[test]
fn build_01_ci_compile_commands_are_canonical() {
    for name in [
        "qcbor-default.compile_commands.json",
        "corejson-default.compile_commands.json",
    ] {
        let text = ci_text(name);
        let entries: Vec<CcEntry> = serde_json::from_str(&text).unwrap();
        assert!(!entries.is_empty(), "{name} has no entries");
        assert_eq!(
            to_json(&entries).unwrap(),
            format!("{text}\n"),
            "{name} does not re-serialise to the same bytes"
        );
        let keys: Vec<(&[u8], &[u8])> = entries
            .iter()
            .map(|entry| (entry.file.as_bytes(), entry.output.as_bytes()))
            .collect();
        assert!(
            keys.windows(2).all(|pair| pair[0] <= pair[1]),
            "{name} is not sorted by (file, output)"
        );
        let outputs: BTreeSet<&str> = entries.iter().map(|entry| entry.output.as_str()).collect();
        assert_eq!(outputs.len(), entries.len(), "{name} repeats an output");
        for entry in &entries {
            assert_eq!(
                entry.arguments.first().map(String::as_str),
                Some(PINNED_COMPILER),
                "{name}: {} is not compiled by the pinned compiler",
                entry.file
            );
            assert!(
                entry.arguments.iter().any(|argument| argument == "-c"),
                "{name}: {} is not a compile-only command",
                entry.file
            );
        }
    }
}

#[test]
fn build_02_ci_reports_round_trip() {
    let capture_text = ci_text("corejson-default.capture.json");
    let capture: CaptureReport = serde_json::from_str(&capture_text).unwrap();
    assert_eq!(capture.target, "corejson");
    assert_eq!(capture.config, "default");
    assert_eq!(capture.kind, "capture");
    assert_eq!(capture.verdict, CaptureVerdict::Captured);
    assert!(capture.gates.compiler_allow_listed && capture.gates.arch_ok);
    assert!(!capture.steps.is_empty(), "the capture carries run records");
    assert_eq!(capture.to_json().unwrap(), format!("{capture_text}\n"));

    let replay_text = ci_text("corejson-default.replay.json");
    let replay: ReplayReport = serde_json::from_str(&replay_text).unwrap();
    assert_eq!(replay.target, "corejson");
    assert_eq!(replay.config, "default");
    assert_eq!(replay.kind, "replay");
    assert_eq!(replay.verdict, Verdict::Identical);
    assert!(!replay.normalisation_applied);
    assert!(!replay.steps.is_empty(), "the replay carries run records");
    assert_eq!(replay.to_json().unwrap(), format!("{replay_text}\n"));

    // The replay compares the objects the capture recorded.
    let captured: BTreeSet<&String> = capture.objects.keys().collect();
    let replayed: BTreeSet<&String> = replay.objects.keys().collect();
    assert_eq!(captured, replayed);
    for (key, object) in &capture.objects {
        assert_eq!(
            replay.objects[key].original.as_deref(),
            Some(object.sha256.as_str()),
            "{key}: the replay's original is not the captured object"
        );
    }
}

#[test]
fn build_01_ci_readelf_passes_the_arch_gate() {
    let attributes = parse_readelf_a(&ci_text("arm-object.readelf-A.txt"));
    assert_eq!(attributes.cpu_arch.as_deref(), Some("v7E-M"));
    assert_eq!(attributes.thumb_isa_use.as_deref(), Some("Thumb-2"));
    assert_eq!(attributes.fp_arch.as_deref(), Some("VFPv4-D16"));
    assert_eq!(attributes.abi_vfp_args.as_deref(), Some("VFP registers"));
    assert!(attributes.is_cortex_m4_hard_float());
    // Recorded, never part of the gate.
    assert_eq!(
        attributes.optimization_goals.as_deref(),
        Some("Aggressive Size")
    );
}

#[test]
fn build_02_ci_arm_objects_differ_only_in_comment() {
    let plain = ci_bytes("arm-small.o");
    let commented = ci_bytes("arm-small-comment.o");
    assert_ne!(plain, commented, "the two objects must differ");
    check_elf_header(&plain).unwrap();
    check_elf_header(&commented).unwrap();

    let differing = differing_sections(&plain, &commented).unwrap();
    assert_eq!(differing, vec![".comment".to_owned()]);
    assert!(normalisable(&differing));
    assert_eq!(
        code_digest(&plain).unwrap(),
        code_digest(&commented).unwrap(),
        "the executable sections must be equal"
    );
}

#[test]
fn bed_01_ci_ast_dump_contains_the_entry_signature() {
    let manifest = Manifest::load(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testbed/manifest.toml"),
    )
    .unwrap();
    let entry = manifest.target["qcbor"]
        .entry
        .iter()
        .find(|entry| entry.name == "QCBORDecode_Init")
        .expect("the manifest names QCBORDecode_Init for qcbor");

    let declarations = parse_ast_stream(&ci_bytes("qcbor-entry.ast.json")).unwrap();
    let found: Vec<_> = declarations
        .iter()
        .filter(|declaration| declaration.name == entry.name)
        .collect();
    assert!(!found.is_empty(), "QCBORDecode_Init is not in the AST dump");
    for declaration in found {
        assert_eq!(
            normalise_signature(&declaration.qual_type),
            normalise_signature(&entry.signature)
        );
        assert!(
            declaration
                .file
                .as_deref()
                .is_some_and(|file| file.ends_with(entry.header.as_str())),
            "declared in {:?}, not in {}",
            declaration.file,
            entry.header
        );
    }
}

#[test]
fn build_01_ci_source_md_names_every_fixture() {
    let source = ci_text("SOURCE.md");
    assert!(
        source.contains(FIXTURE_RUN_ID),
        "SOURCE.md does not name run {FIXTURE_RUN_ID}"
    );
    let mut names = Vec::new();
    for item in fs::read_dir(ci_path("")).unwrap() {
        let name = item.unwrap().file_name().into_string().unwrap();
        if name != "SOURCE.md" {
            names.push(name);
        }
    }
    assert_eq!(names.len(), 8, "expected eight fixtures, found {names:?}");
    for name in names {
        assert!(source.contains(&name), "SOURCE.md does not name {name}");
    }
}
