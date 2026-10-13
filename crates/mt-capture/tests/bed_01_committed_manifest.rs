#![allow(clippy::unwrap_used, clippy::expect_used)]
//! BED-01 and BED-02 over the committed `testbed/manifest.toml`: the
//! founder-approved target list (CONTEXT D-03) is valid, pinned, licensed and
//! covers the beachhead categories, and the first-RTOS decision is recorded.
//! Plain `cargo test`: no Docker, no network, no external process.

use std::collections::BTreeSet;
use std::path::PathBuf;

use mt_capture::manifest::{
    BEACHHEAD_CATEGORIES, Category, EntryForm, Manifest, PERMISSIVE_LICENCES, Source,
};

/// The 16 ids the founder approved at the D-03 checkpoint
/// (`target_list_approval: approve-as-proposed`, no edits).
const APPROVED_IDS: [&str; 16] = [
    "cjson",
    "corejson",
    "coremqtt",
    "httpparser",
    "isotp",
    "libfdt",
    "lz4",
    "mbedtls",
    "microcoap",
    "nanopb",
    "nimble",
    "paho",
    "qcbor",
    "tinycbor",
    "zcbor",
    "zlib",
];

fn repo_file(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn committed() -> Manifest {
    Manifest::load(&repo_file("testbed/manifest.toml")).expect("testbed/manifest.toml parses")
}

fn is_lower_hex(text: &str, len: usize) -> bool {
    text.len() == len && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

fn ids_with_category(manifest: &Manifest, category: Category) -> BTreeSet<&str> {
    manifest
        .target
        .iter()
        .filter(|(_, target)| target.category.contains(&category))
        .map(|(id, _)| id.as_str())
        .collect()
}

#[test]
fn bed_01_committed_manifest_validates() {
    let manifest = committed();
    manifest
        .validate()
        .expect("the committed manifest passes every validation rule");
    let ids: Vec<&str> = manifest.target.keys().map(String::as_str).collect();
    assert_eq!(
        ids, APPROVED_IDS,
        "target ids must equal the founder-approved list"
    );
    // The canonical form is stable text, so later plans can hash it.
    let first = manifest.canonical_json().expect("canonical json");
    let second = committed().canonical_json().expect("canonical json");
    assert_eq!(first, second);
}

#[test]
fn bed_01_manifest_pins_every_target_by_commit() {
    let manifest = committed();
    let mut archives = Vec::new();
    for (id, target) in &manifest.target {
        match &target.source {
            Source::Git {
                url,
                commit,
                tree_sha256,
                ..
            } => {
                assert!(url.starts_with("https://"), "{id}: url {url}");
                assert!(is_lower_hex(commit, 40), "{id}: commit {commit}");
                assert!(is_lower_hex(tree_sha256, 64), "{id}: tree {tree_sha256}");
            }
            Source::Archive {
                url,
                sha256,
                strip_components,
                reference,
                commit,
                tree_sha256,
            } => {
                assert!(url.starts_with("https://"), "{id}: url {url}");
                assert!(is_lower_hex(sha256, 64), "{id}: archive sha256 {sha256}");
                assert!(is_lower_hex(commit, 40), "{id}: commit {commit}");
                assert!(is_lower_hex(tree_sha256, 64), "{id}: tree {tree_sha256}");
                archives.push((id.as_str(), *strip_components, reference.as_str()));
            }
        }
    }
    // PROJECT D-20: Mbed TLS is the one archive source.
    assert_eq!(archives, [("mbedtls", 1, "mbedtls-3.6.7")]);
    match &manifest.target["mbedtls"].source {
        Source::Archive {
            sha256,
            commit,
            tree_sha256,
            ..
        } => {
            assert_eq!(
                sha256,
                "a7e8bcbec0e6f761b4af24f25677626b35f762f68eef79c08677a363212d11f6"
            );
            assert_eq!(commit, "068ff080b369adfac81509f9b57b2afabaf82dc5");
            assert_eq!(
                tree_sha256,
                "00a636ffc99e447237abd6dee684a9ba0fe3727c1ea6d266cbe68989bb408248"
            );
        }
        Source::Git { .. } => panic!("mbedtls must be an archive source"),
    }
    // Every pin is distinct: a copy-paste slip would repeat one.
    let commits: BTreeSet<String> = manifest
        .target
        .values()
        .map(|t| match &t.source {
            Source::Git { commit, .. } | Source::Archive { commit, .. } => commit.clone(),
        })
        .collect();
    assert_eq!(commits.len(), manifest.target.len());
}

#[test]
fn bed_01_manifest_has_between_10_and_20_targets() {
    let count = committed().target.len();
    assert!((10..=20).contains(&count), "found {count} targets");
    assert_eq!(count, APPROVED_IDS.len());
}

#[test]
fn bed_01_every_target_records_permissive_licence_and_file() {
    let manifest = committed();
    for (id, target) in &manifest.target {
        let licence = &target.licence;
        assert!(
            PERMISSIVE_LICENCES.contains(&licence.elected.as_str()),
            "{id}: elected {} is not permissive",
            licence.elected
        );
        assert!(
            licence.spdx.split(" OR ").any(|o| o == licence.elected),
            "{id}: elected {} is not an operand of {}",
            licence.elected,
            licence.spdx
        );
        assert!(!licence.file.is_empty(), "{id}: licence file missing");
    }
    // The D-27 elections the founder approved.
    let elected = |id: &str| manifest.target[id].licence.elected.as_str();
    assert_eq!(elected("mbedtls"), "Apache-2.0");
    assert_eq!(elected("paho"), "BSD-3-Clause");
    assert_eq!(elected("libfdt"), "BSD-2-Clause");
    assert_eq!(manifest.target["paho"].licence.file, "edl-v10");
    assert_eq!(manifest.target["lz4"].licence.file, "lib/LICENSE");
    assert_eq!(manifest.target["libfdt"].licence.file, "BSD-2-Clause");
}

#[test]
fn bed_01_every_target_has_a_buffer_in_entry() {
    let manifest = committed();
    let mut total = 0;
    let mut wrapped = BTreeSet::new();
    for (id, target) in &manifest.target {
        assert!(
            (1..=3).contains(&target.entry.len()),
            "{id}: {} entries",
            target.entry.len()
        );
        total += target.entry.len();
        for entry in &target.entry {
            assert_eq!(
                entry.form == EntryForm::Wrapped,
                entry.accepted_wrapped,
                "{id}.{}: accepted_wrapped must match the wrapped form",
                entry.name
            );
            if entry.form == EntryForm::Wrapped {
                wrapped.insert(format!("{id}.{}", entry.name));
            }
        }
    }
    // 28 entries in the research table minus libfdt fdt_check_header.
    assert_eq!(total, 27);
    assert!(
        !manifest.target["libfdt"]
            .entry
            .iter()
            .any(|e| e.name == "fdt_check_header")
    );
    // The six wrapped entries the founder accepted (D-28).
    let expected: BTreeSet<String> = [
        "coremqtt.MQTT_DeserializeAck",
        "coremqtt.MQTT_DeserializePublish",
        "nanopb.pb_decode_ex",
        "qcbor.QCBORDecode_Init",
        "tinycbor.cbor_value_validate",
        "zlib.inflate",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect();
    assert_eq!(wrapped, expected);
    let range: Vec<&str> = manifest
        .target
        .values()
        .flat_map(|t| &t.entry)
        .filter(|e| e.form == EntryForm::Range)
        .map(|e| e.name.as_str())
        .collect();
    assert_eq!(range, ["mbedtls_asn1_get_tag"]);
}

#[test]
fn bed_01_every_beachhead_category_is_covered() {
    let manifest = committed();
    for category in BEACHHEAD_CATEGORIES {
        assert!(
            !ids_with_category(&manifest, category).is_empty(),
            "no target covers {category:?}"
        );
    }
    // D-29: TLV and the OTA header are carried by stand-ins.
    let tlv: Vec<&str> = ids_with_category(&manifest, Category::Tlv)
        .into_iter()
        .collect();
    assert_eq!(tlv, ["libfdt", "mbedtls", "microcoap", "nimble"]);
    let ota: Vec<&str> = ids_with_category(&manifest, Category::OtaHeader)
        .into_iter()
        .collect();
    assert_eq!(ota, ["libfdt"]);
}

#[test]
fn bed_02_every_target_declares_default_and_at_most_one_variant() {
    let manifest = committed();
    let mut variants = BTreeSet::new();
    for (id, target) in &manifest.target {
        let default = target
            .default_config()
            .unwrap_or_else(|| panic!("{id}: no default configuration"));
        assert_eq!(default.name, "default");
        assert!(
            (1..=2).contains(&target.config.len()),
            "{id}: {} configurations",
            target.config.len()
        );
        if let Some(variant) = target.variant() {
            variants.insert(format!("{id}.{}", variant.name));
        }
    }
    // Table 3: one variant per target, none for isotp; 31 target-configuration pairs.
    assert_eq!(variants.len(), 15);
    assert!(manifest.target["isotp"].variant().is_none());
    let pairs: usize = manifest.target.values().map(|t| t.config.len()).sum();
    assert_eq!(pairs, 31);
    assert!(variants.contains("tinycbor.no-float"));
    assert!(variants.contains("paho.server"));
    assert!(variants.contains("mbedtls.suite-b"));
    assert!(variants.contains("zlib.no-gzip"));
}

#[test]
fn bed_02_first_rtos_decision_recorded() {
    let manifest = committed();
    assert_eq!(manifest.decisions.first_rtos, "deferred");
    assert_eq!(manifest.decisions.project_decision, "D-18");
    assert_eq!(
        manifest.decisions.first_rtos_candidates,
        ["freertos", "zephyr"]
    );
    assert!(
        manifest
            .decisions
            .first_rtos_reason
            .contains("design-partner interviews")
    );

    let project = std::fs::read_to_string(repo_file(".planning/PROJECT.md")).expect("PROJECT.md");
    let line = project
        .lines()
        .find(|l| l.starts_with("- **D-18:**"))
        .expect("PROJECT.md has a D-18 decision line");
    for needle in ["deferred", "Zephyr", "FreeRTOS"] {
        assert!(line.contains(needle), "D-18 line lacks {needle:?}: {line}");
    }
    let row = project
        .lines()
        .find(|l| l.starts_with("| First RTOS ecosystem"))
        .expect("Key Decisions has the First RTOS ecosystem row");
    assert!(row.contains("Deferred (D-18)"), "row: {row}");
}

#[test]
fn bed_01_project_records_dual_licence_and_archive_decisions() {
    let project = std::fs::read_to_string(repo_file(".planning/PROJECT.md")).expect("PROJECT.md");
    let d19 = project
        .lines()
        .find(|l| l.starts_with("- **D-19:**"))
        .expect("PROJECT.md has a D-19 decision line");
    assert!(d19.contains("elected"), "D-19: {d19}");
    let d20 = project
        .lines()
        .find(|l| l.starts_with("- **D-20:**"))
        .expect("PROJECT.md has a D-20 decision line");
    assert!(
        d20.contains("Mbed TLS") && d20.contains("archive"),
        "D-20: {d20}"
    );
}

#[test]
fn bed_01_tracer_targets_have_build_args() {
    let manifest = committed();
    // The two tracer recipes keep the shape the tracer proved in CI (plan 02-06).
    let qcbor = manifest.target["qcbor"]
        .build_args
        .as_ref()
        .expect("recipe");
    assert_eq!(qcbor.dir, ".");
    assert_eq!(qcbor.goals, ["libqcbor.a"]);
    assert_eq!(qcbor.make_vars, ["CC={cc} {cflags}"]);

    let corejson = manifest.target["corejson"]
        .build_args
        .as_ref()
        .expect("recipe");
    assert_eq!(corejson.dir, ".");
    assert_eq!(corejson.cmake_target.as_deref(), Some("core_json"));
}

/// Plan 02-09 (D-07): every approved target now carries its recipe, so the
/// completeness rule that plan 02-03 left unenforced holds. (This replaces the
/// 02-03 expectation that only the two tracer targets had a recipe.)
#[test]
fn bed_01_committed_manifest_is_complete() {
    let manifest = committed();
    manifest
        .validate_complete()
        .expect("every target has a build recipe and the manifest is valid");
    for (id, target) in &manifest.target {
        let args = target.build_args.as_ref().expect("recipe");
        let configured_target = args.cmake_target.is_some()
            || target
                .config
                .iter()
                .any(|config| config.cmake_target.is_some());
        match target.build {
            mt_capture::manifest::BuildSystem::Cmake => {
                assert!(configured_target, "{id}: a CMake recipe names a target");
            }
            mt_capture::manifest::BuildSystem::Make => {
                assert!(!args.goals.is_empty(), "{id}: a Make recipe names goals");
            }
        }
    }
    // Mbed TLS builds with the committed testbed user configuration.
    let mbedtls = &manifest.target["mbedtls"];
    let recipe = mbedtls.build_args.as_ref().expect("recipe");
    assert!(
        recipe
            .make_vars
            .iter()
            .any(|v| v.contains("MBEDTLS_USER_CONFIG_FILE") && v.contains("{inputs}/user_config.h")),
        "mbedtls must build with the testbed user config: {:?}",
        recipe.make_vars
    );
    let config = std::fs::read_to_string(repo_file("testbed/targets/mbedtls/user_config.h"))
        .expect("user_config.h exists");
    for name in [
        "MBEDTLS_NET_C",
        "MBEDTLS_TIMING_C",
        "MBEDTLS_FS_IO",
        "MBEDTLS_HAVE_TIME_DATE",
        "MBEDTLS_HAVE_TIME",
        "MBEDTLS_PSA_ITS_FILE_C",
        "MBEDTLS_PSA_CRYPTO_STORAGE_C",
        "MBEDTLS_THREADING_C",
        "MBEDTLS_THREADING_PTHREAD",
    ] {
        assert!(
            config.lines().any(|l| l.trim() == format!("#undef {name}")),
            "user_config.h must #undef {name}"
        );
    }
    assert!(config.contains("#define MBEDTLS_NO_PLATFORM_ENTROPY"));
}
