#![allow(clippy::unwrap_used, clippy::expect_used)]
//! EQV-03: the floor of 10 drivable targets and the adapter drop log (CONTEXT
//! D-13, D-22, D-24, D-28). Committed files only: no tool runs, no network.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use mt_capture::droplog::{DropLog, MIN_RETAINED};
use mt_capture::manifest::Manifest;
use mt_harness::adapter_drops::{
    ADAPTER_FLOOR, AdapterDropCause, AdapterDropLog, check_adapter_floor, drivable_targets,
};
use mt_seeds::index::SeedIndex;

/// The targets the phase's floor is made of (plans 03-04 to 03-06).
const FLOOR_TARGETS: [&str; 10] = [
    "corejson",
    "lz4",
    "libfdt",
    "microcoap",
    "nimble",
    "tinycbor",
    "zlib",
    "paho",
    "nanopb",
    "cjson",
];

fn testbed() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testbed")
}

fn manifest() -> Manifest {
    let manifest = Manifest::load(&testbed().join("manifest.toml")).unwrap();
    manifest.validate().unwrap();
    manifest
}

fn seed_index(seeds: &Path, target: &str) -> SeedIndex {
    let bytes = std::fs::read(seeds.join(target).join("index.json")).unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

/// How many seeds of `target` list `entry`.
fn seeds_for(seeds: &Path, target: &str, entry: &str) -> usize {
    seed_index(seeds, target)
        .seeds
        .iter()
        .filter(|seed| seed.entries.iter().any(|name| name == entry))
        .count()
}

#[test]
fn eqv_03_adapter_floor_is_ten() {
    // ROADMAP Phase 3 criterion 3: at least 10 test-bed targets are drivable.
    assert_eq!(ADAPTER_FLOOR, 10);
    assert_eq!(
        ADAPTER_FLOOR, MIN_RETAINED,
        "the floor is the Phase 2 floor"
    );
    let manifest = manifest();
    let phase2 = DropLog::load_from_testbed_dir(&testbed()).unwrap();
    let adapter = AdapterDropLog::load_from_testbed_dir(&testbed()).unwrap();
    let seeds = testbed().join("seeds");
    let drivable = drivable_targets(&manifest, &seeds, &phase2, &adapter);
    assert!(
        drivable.len() >= ADAPTER_FLOOR,
        "{} drivable targets: {drivable:?}",
        drivable.len()
    );
    // Every counted target is retained and has a seeded, undropped entry with roles.
    for target in &drivable {
        assert!(phase2.target_dropped(target).is_none(), "{target}");
        let entries = &manifest.target[target].entry;
        assert!(
            entries.iter().any(|entry| {
                !entry.arg.is_empty()
                    && seeds_for(&seeds, target, &entry.name) > 0
                    && !adapter
                        .dropped
                        .iter()
                        .any(|row| row.target == *target && row.entry == entry.name)
            }),
            "{target} has no seeded, undropped entry with roles"
        );
    }
    // Each floor target without an adapter drop row is counted.
    for target in FLOOR_TARGETS {
        let dropped = adapter.dropped.iter().any(|row| row.target == target);
        if !dropped {
            assert!(drivable.contains(target), "{target} is not drivable");
        }
    }
    assert_eq!(
        check_adapter_floor(&manifest, &seeds, &phase2, &adapter).unwrap(),
        drivable.len()
    );
}

#[test]
fn eqv_03_no_seeds_entries_are_logged() {
    // D-22, D-28: an entry with no seed is logged `no_seeds` and never borrows
    // another entry's seeds.
    let manifest = manifest();
    let adapter = AdapterDropLog::load_from_testbed_dir(&testbed()).unwrap();
    let seeds = testbed().join("seeds");
    let mut without_seeds = BTreeSet::new();
    for (target, entry) in manifest
        .target
        .iter()
        .flat_map(|(id, target)| target.entry.iter().map(move |entry| (id, entry)))
    {
        if seeds_for(&seeds, target, &entry.name) == 0 {
            without_seeds.insert((target.clone(), entry.name.clone()));
        }
    }
    let expected: BTreeSet<(String, String)> = [
        ("zcbor", "cbor_decode_Pet"),
        ("httpparser", "http_parser_parse_url"),
        ("mbedtls", "mbedtls_asn1_get_tag"),
    ]
    .iter()
    .map(|(target, entry)| ((*target).to_owned(), (*entry).to_owned()))
    .collect();
    assert_eq!(
        without_seeds, expected,
        "the entries whose index lists no seed"
    );
    let logged: BTreeSet<(String, String)> = adapter
        .dropped
        .iter()
        .filter(|row| row.cause == AdapterDropCause::NoSeeds)
        .map(|row| (row.target.clone(), row.entry.clone()))
        .collect();
    assert_eq!(
        logged, without_seeds,
        "every such entry is logged no_seeds, and only those"
    );
    for row in adapter
        .dropped
        .iter()
        .filter(|row| row.cause == AdapterDropCause::NoSeeds)
    {
        assert_eq!(
            row.attempt, "none: the seed corpus lists no seed for this entry (D-28)",
            "{}/{}",
            row.target, row.entry
        );
    }
}

#[test]
fn eqv_03_adapter_drop_log_is_consistent() {
    let manifest = manifest();
    let adapter = AdapterDropLog::load_from_testbed_dir(&testbed()).unwrap();
    adapter.validate(&manifest).unwrap();
    adapter.check_consistency(&manifest).unwrap();
    let mut seen = BTreeSet::new();
    for row in &adapter.dropped {
        let at = format!("{}/{}", row.target, row.entry);
        let entry = manifest
            .target
            .get(&row.target)
            .and_then(|target| target.entry.iter().find(|entry| entry.name == row.entry))
            .unwrap_or_else(|| panic!("{at} is not an entry of the manifest"));
        assert!(!row.detail.trim().is_empty(), "{at}: detail");
        assert!(!row.attempt.trim().is_empty(), "{at}: attempt");
        assert!(entry.arg.is_empty(), "{at}: a logged entry has no roles");
        assert!(seen.insert(at.clone()), "{at} is logged twice");
        if row.cause == AdapterDropCause::NotAttempted {
            assert_eq!(row.detail, "phase budget (D-24)", "{at}");
            assert_eq!(
                row.attempt, "none: stopped at the phase budget before the attempt",
                "{at}"
            );
        }
    }
    assert!(
        !adapter.dropped.is_empty(),
        "the three no_seeds rows are there"
    );
}

const GOOD_ROW: &str = "[[dropped]]\ntarget = \"zcbor\"\nentry = \"cbor_decode_Pet\"\ncause = \"no_seeds\"\ndetail = \"the seed index lists no seed\"\nattempt = \"none: the seed corpus lists no seed for this entry (D-28)\"\n";

fn log_of(rows: &str) -> String {
    format!("schema_version = 1\n\n{rows}\n")
}

#[test]
fn eqv_03_adapter_drop_log_validation_rejects_bad_rows() {
    let manifest = manifest();
    let accepts = |rows: &str| -> Result<(), String> {
        let log = AdapterDropLog::parse(&log_of(rows)).map_err(|e| e.to_string())?;
        log.validate(&manifest).map_err(|e| e.to_string())?;
        log.check_consistency(&manifest).map_err(|e| e.to_string())
    };
    accepts(GOOD_ROW).unwrap();
    let must_fail = |rows: String, words: &[&str]| {
        let error = accepts(&rows).unwrap_err();
        assert!(
            words.iter().all(|word| error.contains(word)),
            "{words:?} not in {error}"
        );
    };
    must_fail(
        GOOD_ROW.replace("no_seeds", "borrowed_seeds"),
        &["borrowed_seeds"],
    );
    must_fail(GOOD_ROW.replace("zcbor", "nothing"), &["nothing"]);
    must_fail(
        GOOD_ROW.replace("cbor_decode_Pet", "cbor_nothing"),
        &["cbor_nothing"],
    );
    must_fail(
        GOOD_ROW.replace(
            "attempt = \"none: the seed corpus lists no seed for this entry (D-28)\"",
            "attempt = \"\"",
        ),
        &["attempt"],
    );
    must_fail(
        GOOD_ROW.replace(
            "detail = \"the seed index lists no seed\"",
            "detail = \" \"",
        ),
        &["detail"],
    );
    must_fail(format!("{GOOD_ROW}{GOOD_ROW}"), &["twice"]);
    must_fail(
        GOOD_ROW.replace("no_seeds", "not_attempted"),
        &["not_attempted", "phase budget"],
    );
    // An entry with roles is in the log or has roles, never both.
    must_fail(
        GOOD_ROW
            .replace("zcbor", "zlib")
            .replace("cbor_decode_Pet", "uncompress")
            .replace("no_seeds", "attempt_failed"),
        &["zlib/uncompress", "roles"],
    );
    // The closed set of causes and the spelling of the exact not_attempted texts.
    let budget = "[[dropped]]\ntarget = \"zcbor\"\nentry = \"cbor_decode_Pet\"\ncause = \"not_attempted\"\ndetail = \"phase budget (D-24)\"\nattempt = \"none: stopped at the phase budget before the attempt\"\n";
    accepts(budget).unwrap();
}

/// A copy of the committed seed indexes in a temporary `seeds` directory.
fn copy_indexes(to: &Path) {
    let from = testbed().join("seeds");
    for item in std::fs::read_dir(&from).unwrap() {
        let item = item.unwrap();
        let index = item.path().join("index.json");
        if index.is_file() {
            let dir = to.join(item.file_name());
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::copy(&index, dir.join("index.json")).unwrap();
        }
    }
}

#[test]
fn eqv_03_floor_counts_only_seeded_undropped_entries() {
    // T-03-23: an entry with no seed, a logged entry and a dropped target do not
    // count, and a count below the floor is an error naming both numbers.
    let manifest = manifest();
    let phase2 = DropLog::load_from_testbed_dir(&testbed()).unwrap();
    let none = AdapterDropLog::default();
    let tmp = tempfile::tempdir().unwrap();
    let seeds = tmp.path().join("seeds");
    copy_indexes(&seeds);
    let baseline = drivable_targets(&manifest, &seeds, &phase2, &none);
    assert!(baseline.len() >= 10, "{baseline:?}");
    assert!(baseline.contains("zlib") && baseline.contains("cjson"));

    // zlib's only entry with roles loses its seeds: zlib does not count, though
    // its other entry (inflate) has seeds, because inflate has no roles.
    let path = seeds.join("zlib/index.json");
    let mut json: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    for seed in json["seeds"].as_array_mut().unwrap() {
        let entries = seed["entries"].as_array_mut().unwrap();
        entries.retain(|name| name != "uncompress");
    }
    std::fs::write(&path, serde_json::to_vec(&json).unwrap()).unwrap();
    let after = drivable_targets(&manifest, &seeds, &phase2, &none);
    assert!(!after.contains("zlib"), "{after:?}");
    assert_eq!(after.len(), baseline.len() - 1);

    // A logged entry does not count.
    copy_indexes(&seeds);
    let log = AdapterDropLog::parse(&log_of(
        "[[dropped]]\ntarget = \"cjson\"\nentry = \"cJSON_ParseWithLength\"\ncause = \"attempt_failed\"\ndetail = \"x\"\nattempt = \"y\"\n",
    ))
    .unwrap();
    let after = drivable_targets(&manifest, &seeds, &phase2, &log);
    assert!(!after.contains("cjson"), "{after:?}");
    assert_eq!(after.len(), baseline.len() - 1);

    // A target the Phase 2 drop log drops does not count.
    let dropped_lz4 = DropLog::parse(
        "schema_version = 1\n[policy]\nvariant_failure = \"drop_variant\"\n[[dropped]]\ntarget = \"lz4\"\ncause = \"build_failed\"\ndetail = \"d\"\nfix_attempted = \"f\"\nci_run = 1\n",
    )
    .unwrap();
    let after = drivable_targets(&manifest, &seeds, &dropped_lz4, &none);
    assert!(!after.contains("lz4"), "{after:?}");

    // Below the floor the check is an error that names the count and the floor.
    let rows: String = [
        ("lz4", "LZ4_decompress_safe"),
        ("libfdt", "fdt_check_full"),
        ("microcoap", "coap_parse"),
        ("nimble", "ble_uuid_init_from_buf"),
    ]
    .iter()
    .map(|(target, entry)| {
        format!("[[dropped]]\ntarget = \"{target}\"\nentry = \"{entry}\"\ncause = \"attempt_failed\"\ndetail = \"x\"\nattempt = \"y\"\n")
    })
    .collect();
    let log = AdapterDropLog::parse(&log_of(&rows)).unwrap();
    let error = check_adapter_floor(&manifest, &seeds, &phase2, &log)
        .unwrap_err()
        .to_string();
    assert!(error.contains('6') && error.contains("10"), "{error}");
}
