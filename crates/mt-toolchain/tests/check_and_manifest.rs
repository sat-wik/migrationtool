#![allow(clippy::unwrap_used, clippy::expect_used)]
//! TOOL-01 and TOOL-02 check rules (pure evaluation over recorded tool output),
//! TOOL-03 manifest and hash helper. Nothing here starts a process or touches
//! the network: observations are built from `fixtures/tool-outputs.toml`.

use std::collections::BTreeMap;

use mt_toolchain::check::{self, CheckReport, Observation, ToolRow, ToolStatus, evaluate};
use mt_toolchain::fetch;
use mt_toolchain::manifest::{self, MANIFEST_SCHEMA_VERSION};
use mt_toolchain::pins::Pins;
use serde::Deserialize;

const FULL: &str = include_str!("fixtures/pins-full.toml");
const OUTPUTS: &str = include_str!("fixtures/tool-outputs.toml");

/// One recorded tool invocation.
#[derive(Debug, Deserialize)]
struct Recorded {
    stdout: String,
    stderr: String,
}

fn recorded() -> BTreeMap<String, Recorded> {
    toml::from_str(OUTPUTS).expect("tool-outputs.toml parses")
}

fn pins() -> Pins {
    Pins::parse(FULL).expect("pins-full.toml parses")
}

fn observation(name: &str) -> Observation {
    let all = recorded();
    let rec = all
        .get(name)
        .unwrap_or_else(|| panic!("fixture table {name} missing"));
    Observation::ran(name, &rec.stdout, &rec.stderr)
}

/// Observations of a healthy container, taken from the recorded outputs.
fn good_observations() -> BTreeMap<String, Observation> {
    let mut map = BTreeMap::new();
    for name in [
        "arm_gcc",
        "bear",
        "cargo_mutants",
        "clang",
        "dpkg_query",
        "klee",
        "llvm_config",
        "qemu_arm",
        "qemu_system_arm",
        "rustc_bitcode",
        "rustc_tool",
        "rustup",
        "rustup_default",
        "rustup_targets_bitcode",
        "rustup_targets_tool",
    ] {
        map.insert(name.to_owned(), observation(name));
    }
    map.insert(
        "c2rust".to_owned(),
        observation("c2rust").with_probe(observation("c2rust_ldd")),
    );
    map
}

fn eval(observations: &BTreeMap<String, Observation>) -> CheckReport {
    evaluate(&pins(), observations)
}

fn row<'a>(report: &'a CheckReport, key: &str) -> &'a ToolRow {
    report
        .rows
        .iter()
        .find(|r| r.key == key)
        .unwrap_or_else(|| panic!("no row {key}; rows: {:?}", keys(report)))
}

fn keys(report: &CheckReport) -> Vec<&str> {
    report.rows.iter().map(|r| r.key.as_str()).collect()
}

/// Replace `from` with `to` in the stdout of observation `key`, asserting it is there.
fn edit_stdout(map: &mut BTreeMap<String, Observation>, key: &str, from: &str, to: &str) {
    let obs = map.get_mut(key).expect("observation exists");
    assert!(
        obs.stdout.contains(from),
        "{key} stdout does not contain {from:?}"
    );
    obs.stdout = obs.stdout.replace(from, to);
}

fn failing_keys(report: &CheckReport) -> Vec<&str> {
    report
        .rows
        .iter()
        .filter(|r| !r.status.is_pass())
        .map(|r| r.key.as_str())
        .collect()
}

#[test]
fn tool_02_check_accepts_the_recorded_healthy_container() {
    let report = eval(&good_observations());
    assert!(
        report.passed(),
        "failing rows: {:?}\n{}",
        failing_keys(&report),
        report.render_table()
    );
    for tool in ["clang", "klee", "llvm_config", "c2rust"] {
        assert_eq!(row(&report, tool).llvm_major, Some(16), "{tool}");
    }
}

#[test]
fn tool_02_check_fails_when_any_tool_llvm_major_differs() {
    let mut obs = good_observations();
    edit_stdout(
        &mut obs,
        "klee",
        "LLVM version 16.0.6",
        "LLVM version 17.0.1",
    );
    let report = eval(&obs);
    assert!(!report.passed());
    let klee = row(&report, "klee");
    assert_eq!(klee.status, ToolStatus::Mismatch);
    assert_eq!(klee.llvm_major, Some(17));
    assert_eq!(failing_keys(&report), ["klee"]);

    // c2rust links LLVM 17 according to its probe while its own version is on pin.
    let mut obs = good_observations();
    let probe = obs
        .get_mut("c2rust")
        .and_then(|o| o.probe.as_mut())
        .expect("c2rust has a probe");
    probe.stdout = probe
        .stdout
        .replace("libclang-cpp.so.16", "libclang-cpp.so.17");
    let report = eval(&obs);
    let c2rust = row(&report, "c2rust");
    assert_eq!(c2rust.status, ToolStatus::Mismatch);
    assert_eq!(c2rust.llvm_major, Some(17));
    assert_eq!(c2rust.actual.as_deref(), Some("0.22.1"));
    assert!(!report.passed());
}

#[test]
fn tool_02_bitcode_rustc_llvm_above_19_fails() {
    // The 1.99.0 compiler (LLVM 23.1.1) installed as the bitcode toolchain.
    let mut obs = good_observations();
    obs.insert("rustc_bitcode".to_owned(), observation("rustc_tool"));
    let report = eval(&obs);
    assert_eq!(row(&report, "rustc_bitcode").status, ToolStatus::Mismatch);
    assert_eq!(row(&report, "rustc_bitcode").llvm_major, Some(23));
    assert!(!report.passed());

    // Only the LLVM line differs: the release and commit are on pin.
    let mut obs = good_observations();
    edit_stdout(
        &mut obs,
        "rustc_bitcode",
        "LLVM version: 16.0.5",
        "LLVM version: 23.1.1",
    );
    let report = eval(&obs);
    let bitcode = row(&report, "rustc_bitcode");
    assert_eq!(bitcode.status, ToolStatus::Mismatch);
    let detail = bitcode.detail.as_deref().unwrap_or_default();
    assert!(
        detail.contains("19"),
        "detail should name the bound: {detail}"
    );

    // 17.0.2 is within 16..=19 but is not the pinned major.
    let mut obs = good_observations();
    edit_stdout(
        &mut obs,
        "rustc_bitcode",
        "LLVM version: 16.0.5",
        "LLVM version: 17.0.2",
    );
    let report = eval(&obs);
    assert_eq!(row(&report, "rustc_bitcode").status, ToolStatus::Mismatch);
    assert!(!report.passed());
}

#[test]
fn tool_02_bitcode_rustc_llvm_16_passes() {
    let report = eval(&good_observations());
    let bitcode = row(&report, "rustc_bitcode");
    assert_eq!(bitcode.status, ToolStatus::Ok, "{bitcode:?}");
    assert_eq!(bitcode.llvm_major, Some(16));
    // The tool toolchain's LLVM is shown but never constrained.
    let tool = row(&report, "rustc_tool");
    assert_eq!(tool.status, ToolStatus::Ok, "{tool:?}");
    assert_eq!(tool.llvm_major, Some(23));
}

#[test]
fn tool_02_check_reports_unparseable_llvm_as_failure() {
    let mut obs = good_observations();
    obs.insert(
        "clang".to_owned(),
        Observation::ran("clang", "no version line here\n", ""),
    );
    let report = eval(&obs);
    assert_eq!(row(&report, "clang").status, ToolStatus::Mismatch);
    assert_eq!(row(&report, "clang").llvm_major, None);
    assert!(!report.passed());

    // c2rust itself is on pin but the ldd probe names neither libclang-cpp nor libLLVM.
    let mut obs = good_observations();
    obs.insert(
        "c2rust".to_owned(),
        observation("c2rust").with_probe(Observation::ran(
            "c2rust_ldd",
            "\tlinux-vdso.so.1 (0x00007ffc4a5f1000)\n\tlibc.so.6 => /lib/x86_64-linux-gnu/libc.so.6 (0x1)\n",
            "",
        )),
    );
    let report = eval(&obs);
    let c2rust = row(&report, "c2rust");
    assert_eq!(c2rust.status, ToolStatus::Mismatch);
    assert_eq!(c2rust.llvm_major, None);
    assert!(!report.passed());

    // klee prints its version but no LLVM line.
    let mut obs = good_observations();
    obs.insert(
        "klee".to_owned(),
        Observation::ran("klee", "KLEE 3.2 (https://klee-se.org)\n", ""),
    );
    let report = eval(&obs);
    assert_eq!(row(&report, "klee").status, ToolStatus::Mismatch);
    assert!(!report.passed());

    // A bitcode rustc whose -vV has no LLVM line cannot be proven within the bound.
    let mut obs = good_observations();
    edit_stdout(&mut obs, "rustc_bitcode", "LLVM version: 16.0.5\n", "");
    let report = eval(&obs);
    assert_eq!(row(&report, "rustc_bitcode").status, ToolStatus::Mismatch);
    assert!(!report.passed());
}

#[test]
fn tool_02_check_flags_second_llvm_major_package() {
    let report = eval(&good_observations());
    let packages = row(&report, "llvm_packages");
    assert_eq!(packages.status, ToolStatus::Ok, "{packages:?}");
    assert_eq!(packages.llvm_major, Some(16));

    let mut obs = good_observations();
    edit_stdout(
        &mut obs,
        "dpkg_query",
        "libllvm18\t1:18.1.8-12\tnot-installed",
        "libllvm18\t1:18.1.8-12\tinstalled",
    );
    let report = eval(&obs);
    let packages = row(&report, "llvm_packages");
    assert_eq!(packages.status, ToolStatus::Mismatch);
    assert!(
        packages
            .detail
            .as_deref()
            .unwrap_or_default()
            .contains("libllvm18"),
        "{packages:?}"
    );
    assert!(!report.passed());
}

#[test]
fn tool_02_check_table_rows_sorted_and_stable() {
    let obs = good_observations();
    let report = eval(&obs);
    let mut expected: Vec<String> = [
        "arm_gcc",
        "bear",
        "c2rust",
        "cargo_mutants",
        "clang",
        "hayroll",
        "kani",
        "klee",
        "llvm_config",
        "llvm_packages",
        "qemu_arm",
        "qemu_system_arm",
        "rust_default",
        "rust_targets_bitcode",
        "rust_targets_tool",
        "rustc_bitcode",
        "rustc_tool",
        "rustup",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect();
    expected.extend(pins().apt.keys().map(|name| format!("apt:{name}")));
    expected.sort();
    let got: Vec<String> = report.rows.iter().map(|r| r.key.clone()).collect();
    assert_eq!(got, expected, "one row per item, sorted by key");

    let first = report.render_table();
    let second = eval(&obs).render_table();
    assert_eq!(first, second, "two renders must be byte-identical");
    assert_eq!(
        first.lines().count(),
        report.rows.len() + 3,
        "header, one line per row, pin line, result line:\n{first}"
    );
    assert!(first.contains("result: OK"), "{first}");
}

#[test]
fn tool_02_check_labels_llvm_pin_provisional() {
    let table = eval(&good_observations()).render_table();
    let pin_line = table
        .lines()
        .find(|l| l.starts_with("llvm pin:"))
        .unwrap_or_else(|| panic!("no pin line in:\n{table}"));
    assert!(pin_line.contains("16"), "{pin_line}");
    assert!(pin_line.contains("provisional"), "{pin_line}");
    assert!(pin_line.contains("D-15"), "{pin_line}");
    assert!(pin_line.contains("Phase 6 R1"), "{pin_line}");
    assert!(pin_line.contains("19"), "{pin_line}");

    // Once the pin is final the label says so and no longer says provisional.
    let final_pins = Pins::parse(&FULL.replace("status = \"provisional\"", "status = \"final\""))
        .expect("final pins parse");
    let table = evaluate(&final_pins, &good_observations()).render_table();
    let pin_line = table
        .lines()
        .find(|l| l.starts_with("llvm pin:"))
        .expect("pin line");
    assert!(pin_line.contains("final"), "{pin_line}");
    assert!(!pin_line.contains("provisional"), "{pin_line}");
}

#[test]
fn tool_01_check_reports_timeout_as_not_proven() {
    let mut obs = good_observations();
    obs.insert("clang".to_owned(), Observation::timed_out("clang"));
    let report = eval(&obs);
    assert_eq!(row(&report, "clang").status, ToolStatus::NotProven);
    assert!(!report.passed());
    let table = report.render_table();
    assert!(table.contains("not proven"), "{table}");
    assert!(table.contains("result: FAILED (1 rows)"), "{table}");

    // A timed-out probe is just as unproven.
    let mut obs = good_observations();
    obs.insert(
        "c2rust".to_owned(),
        observation("c2rust").with_probe(Observation::timed_out("c2rust_ldd")),
    );
    let report = eval(&obs);
    assert_eq!(row(&report, "c2rust").status, ToolStatus::NotProven);
    assert!(!report.passed());
}

#[test]
fn tool_01_check_not_installed_tools_never_fail() {
    let report = eval(&good_observations());
    for (key, reason) in [("hayroll", "licence unconfirmed"), ("kani", "deferred")] {
        let r = row(&report, key);
        assert_eq!(r.status, ToolStatus::NotInstalled, "{key}");
        assert!(
            r.detail.as_deref().unwrap_or_default().contains(reason),
            "{key}: {r:?}"
        );
    }
    assert!(report.passed());
    let table = report.render_table();
    assert!(table.contains("not_installed"), "{table}");
}

#[test]
fn tool_01_check_unobserved_failed_and_missing_runs_never_pass() {
    // No observations at all: every installed item fails, none passes by default.
    let report = eval(&BTreeMap::new());
    assert!(!report.passed());
    for r in &report.rows {
        assert!(
            matches!(
                r.status,
                ToolStatus::NotInstalled | ToolStatus::NotProven | ToolStatus::Missing
            ),
            "{} is {:?} without any observation",
            r.key,
            r.status
        );
    }

    let mut obs = good_observations();
    let mut failed = observation("bear");
    failed.exit_code = Some(3);
    obs.insert("bear".to_owned(), failed);
    obs.insert("klee".to_owned(), Observation::missing("klee"));
    let mut broken = observation("clang");
    broken.error = Some("permission denied".to_owned());
    obs.insert("clang".to_owned(), broken);
    let report = eval(&obs);
    assert_eq!(row(&report, "bear").status, ToolStatus::Mismatch);
    assert_eq!(row(&report, "klee").status, ToolStatus::Missing);
    assert_eq!(row(&report, "clang").status, ToolStatus::NotProven);
    assert_eq!(failing_keys(&report), ["bear", "clang", "klee"]);
}

#[test]
fn tool_02_check_flags_wrong_rust_default_and_targets() {
    // The bitcode toolchain must never be the default (D-12).
    let mut obs = good_observations();
    edit_stdout(&mut obs, "rustup_default", "1.99.0-x86_64", "1.72.1-x86_64");
    let report = eval(&obs);
    assert_eq!(row(&report, "rust_default").status, ToolStatus::Mismatch);
    assert!(!report.passed());

    // The bitcode toolchain has the host target only (D-13).
    let mut obs = good_observations();
    edit_stdout(
        &mut obs,
        "rustup_targets_bitcode",
        "x86_64-unknown-linux-gnu\n",
        "thumbv7em-none-eabihf\nx86_64-unknown-linux-gnu\n",
    );
    let report = eval(&obs);
    assert_eq!(
        row(&report, "rust_targets_bitcode").status,
        ToolStatus::Mismatch
    );
    assert_eq!(row(&report, "rust_targets_tool").status, ToolStatus::Ok);

    // The tool toolchain lost a pinned target.
    let mut obs = good_observations();
    edit_stdout(
        &mut obs,
        "rustup_targets_tool",
        "x86_64-unknown-linux-musl\n",
        "",
    );
    let report = eval(&obs);
    assert_eq!(
        row(&report, "rust_targets_tool").status,
        ToolStatus::Mismatch
    );
}

#[test]
fn tool_03_check_apt_rows_flag_missing_and_wrong_versions() {
    let report = eval(&good_observations());
    assert_eq!(row(&report, "apt:clang-16").status, ToolStatus::Ok);
    assert_eq!(
        row(&report, "apt:clang-16").actual.as_deref(),
        Some("1:16.0.6-15~deb12u1")
    );

    let mut obs = good_observations();
    edit_stdout(&mut obs, "dpkg_query", "bear\t3.1.1-1\tinstalled\n", "");
    edit_stdout(
        &mut obs,
        "dpkg_query",
        "qemu-user\t1:7.2+dfsg-7+deb12u18",
        "qemu-user\t1:7.2+dfsg-7+deb12u17",
    );
    edit_stdout(
        &mut obs,
        "dpkg_query",
        "libz3-dev\t4.8.12-3.1\tinstalled",
        "libz3-dev\t4.8.12-3.1\tconfig-files",
    );
    let report = eval(&obs);
    assert_eq!(row(&report, "apt:bear").status, ToolStatus::Missing);
    assert_eq!(row(&report, "apt:qemu-user").status, ToolStatus::Mismatch);
    assert_eq!(row(&report, "apt:libz3-dev").status, ToolStatus::Missing);
    assert_eq!(row(&report, "apt:llvm-16").status, ToolStatus::Ok);
    assert!(!report.passed());

    // dpkg-query itself timing out leaves every package unproven.
    let mut obs = good_observations();
    obs.insert(
        "dpkg_query".to_owned(),
        Observation::timed_out("dpkg_query"),
    );
    let report = eval(&obs);
    assert_eq!(row(&report, "apt:bear").status, ToolStatus::NotProven);
    assert_eq!(row(&report, "llvm_packages").status, ToolStatus::NotProven);
}

#[test]
fn tool_01_check_capture_roundtrips_observations() {
    let obs = good_observations();
    let text = check::capture_toml(&obs, "captured in test").expect("capture renders");
    let parsed: BTreeMap<String, toml::Table> = toml::from_str(&text).expect("capture parses");
    // One table per observation key, the c2rust probe included.
    assert!(parsed.contains_key("clang"), "{text}");
    assert!(parsed.contains_key("c2rust_ldd"), "{text}");
    let klee = parsed.get("klee").expect("klee table");
    assert_eq!(klee["source"].as_str(), Some("captured in test"));
    assert_eq!(
        klee["stdout"].as_str(),
        Some(obs["klee"].stdout.as_str()),
        "stdout survives the round trip"
    );
    assert_eq!(klee["stderr"].as_str(), Some(""));
    let again = check::capture_toml(&obs, "captured in test").expect("capture renders");
    assert_eq!(text, again, "capture is deterministic");
}

/// The serialized manifest for a set of observations.
fn manifest_text(observations: &BTreeMap<String, Observation>) -> String {
    let pins = pins();
    let report = evaluate(&pins, observations);
    manifest::to_json(&manifest::build(&pins, &report)).expect("manifest serializes")
}

#[test]
fn tool_03_manifest_is_byte_stable_and_diff_detects_change() {
    let first = manifest_text(&good_observations());
    let second = manifest_text(&good_observations());
    assert_eq!(first, second, "same inputs must give identical bytes");
    assert!(first.ends_with("}\n"), "{first}");
    assert_eq!(
        manifest::first_difference(&first, &second).unwrap(),
        None,
        "identical manifests have no difference"
    );

    // One observed version moves: the row's dotted key path is reported.
    let mut changed = good_observations();
    edit_stdout(&mut changed, "clang", "version 16.0.6", "version 16.0.7");
    let moved = manifest_text(&changed);
    assert_ne!(first, moved);
    assert_eq!(
        manifest::first_difference(&first, &moved)
            .unwrap()
            .as_deref(),
        Some("observed.clang.actual")
    );

    // A row present on one side only is located at the row.
    let mut shorter: serde_json::Value = serde_json::from_str(&first).unwrap();
    shorter["observed"]
        .as_object_mut()
        .unwrap()
        .remove("bear")
        .unwrap();
    assert_eq!(
        manifest::first_difference(&first, &shorter.to_string())
            .unwrap()
            .as_deref(),
        Some("observed.bear")
    );

    // Arrays are compared by index; anything that is not JSON is an error,
    // never "no difference".
    assert_eq!(
        manifest::first_difference("[1,2]", "[1,3]")
            .unwrap()
            .as_deref(),
        Some("[1]")
    );
    assert!(manifest::first_difference(&first, "not json").is_err());
}

#[test]
fn tool_03_manifest_contains_no_raw_output_or_timestamps() {
    let text = manifest_text(&good_observations());
    for raw in [
        "Host CPU",
        "skylake",
        "Build mode",
        "/usr/lib/llvm",
        "linux-vdso",
    ] {
        assert!(
            !text.contains(raw),
            "raw tool output {raw:?} leaked:\n{text}"
        );
    }
    assert!(
        text.contains(&format!("\"schema_version\": {MANIFEST_SCHEMA_VERSION}")),
        "{text}"
    );
    assert!(text.contains("\"status\": \"provisional\""), "{text}");
    assert!(text.contains("\"decision\": \"D-15\""), "{text}");

    // No key records a run, a time or an output; no value looks like a date.
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    let date = regex::Regex::new(r"\d{4}-\d{2}-\d{2}").unwrap();
    let mut stack = vec![(String::new(), &value)];
    while let Some((path, node)) = stack.pop() {
        match node {
            serde_json::Value::Object(map) => {
                for (key, child) in map {
                    let lower = key.to_lowercase();
                    for banned in [
                        "time", "date", "stdout", "stderr", "argv", "env", "exit", "hash",
                    ] {
                        assert!(
                            !lower.contains(banned) || key == "snapshot_timestamp",
                            "key {path}.{key} looks like run data"
                        );
                    }
                    stack.push((format!("{path}.{key}"), child));
                }
            }
            serde_json::Value::Array(items) => {
                for (i, child) in items.iter().enumerate() {
                    stack.push((format!("{path}[{i}]"), child));
                }
            }
            serde_json::Value::String(s) => {
                assert!(!date.is_match(s), "{path} looks like a date: {s}");
            }
            _ => {}
        }
    }
}

#[test]
fn tool_03_manifest_carries_pins_sources_and_parsed_rows_with_sorted_keys() {
    let text = manifest_text(&good_observations());
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(value["image"]["platform"], "linux/amd64");
    assert_eq!(value["llvm"]["major"], 16);
    assert_eq!(value["llvm"]["max_major"], 19);
    assert_eq!(value["rust"]["bitcode"]["llvm"], "16.0.5");
    assert_eq!(
        value["sources"]["klee"]["commit"],
        "92ee8201a050184aacaaeef645ade491f5c93c41"
    );
    assert_eq!(value["sources"]["klee"]["kind"], "git");
    assert_eq!(value["observed"]["klee"]["status"], "OK");
    assert_eq!(value["observed"]["klee"]["actual"], "3.2");
    assert_eq!(value["observed"]["klee"]["llvm_major"], 16);
    assert_eq!(value["observed"]["rustc_bitcode"]["llvm_major"], 16);
    assert_eq!(value["observed"]["hayroll"]["status"], "not_installed");
    assert!(
        value["observed"]["hayroll"]["detail"]
            .as_str()
            .unwrap()
            .contains("licence unconfirmed")
    );
    // Every package row and every tool row is present.
    assert_eq!(
        value["observed"]["apt:clang-16"]["actual"],
        "1:16.0.6-15~deb12u1"
    );
    assert_eq!(value["observed"].as_object().unwrap().len(), 27);

    // Top-level keys appear in sorted order in the text.
    let top: Vec<&str> = text
        .lines()
        .filter(|l| l.starts_with("  \""))
        .filter_map(|l| l.trim_start().split('"').nth(1))
        .collect();
    let mut sorted = top.clone();
    sorted.sort_unstable();
    assert_eq!(top, sorted);
    assert_eq!(
        top,
        [
            "image",
            "llvm",
            "observed",
            "rust",
            "schema_version",
            "sources"
        ]
    );
}

/// Lowercase hex sha256 computed in the test, independently of the helper.
fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn file_url(path: &std::path::Path) -> String {
    format!("file://{}", path.display())
}

#[test]
fn tool_03_hash_helper_matches_sha256_of_content() {
    let dir = tempfile::tempdir().unwrap();
    let timeout = std::time::Duration::from_secs(60);

    // Larger than the runner's pipe chunk, so streaming is exercised.
    let big: Vec<u8> = (0..300_000u32).map(|i| (i % 251) as u8).collect();
    let small = b"pin bump\n".to_vec();
    for (name, bytes) in [
        ("big.bin", big),
        ("small.txt", small),
        ("empty", Vec::new()),
    ] {
        let path = dir.path().join(name);
        std::fs::write(&path, &bytes).unwrap();
        let digest = fetch::sha256_of_url(&file_url(&path), timeout)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(digest, sha256_hex(&bytes), "{name}");
    }
    // The well-known digest of empty input, as a cross-check of the test helper itself.
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn tool_03_hash_helper_rejects_non_https_urls() {
    let timeout = std::time::Duration::from_secs(5);
    for url in [
        "http://example.com/pin.tar.xz",
        "ftp://example.com/pin.tar.xz",
        "/etc/passwd",
        "relative/path.tar",
        "file:/etc/passwd",
        "HTTPS://example.com/x",
        "-o/tmp/overwritten",
        "gopher://example.com/",
        "",
    ] {
        match fetch::sha256_of_url(url, timeout) {
            Err(fetch::FetchError::UnsupportedScheme { url: rejected }) => {
                assert_eq!(rejected, url);
            }
            other => panic!("{url:?} should be refused before curl runs, got {other:?}"),
        }
    }
}

#[test]
fn tool_03_hash_helper_never_reports_a_digest_for_a_failed_fetch() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("does-not-exist");
    match fetch::sha256_of_url(&file_url(&missing), std::time::Duration::from_secs(30)) {
        Err(fetch::FetchError::Failed { exit_code }) => {
            assert!(exit_code.is_some_and(|code| code != 0), "{exit_code:?}");
        }
        other => panic!("a failed fetch must be an error, got {other:?}"),
    }
}
