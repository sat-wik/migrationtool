#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The drop log (CONTEXT D-11, RESEARCH Pattern 13): what a committed
//! `testbed/dropped.toml` may say, how a dropped pair is reported (never as
//! passing), that `DROPPED.md` stays in step with the log, and that at least
//! [`MIN_RETAINED`] targets remain. Plain `cargo test`: the committed files are
//! read, and the small beds are built in temporary directories with no network
//! and no external build tool (every pair that would run is dropped or has no
//! capture, so no compiler starts).

use std::fs;
use std::path::{Path, PathBuf};

use mt_capture::droplog::{
    DROP_LOG_SCHEMA_VERSION, DropCause, DropLog, DropLogError, Dropped, MIN_RETAINED,
    VariantFailure,
};
use mt_capture::manifest::Manifest;
use mt_capture::seeding::run_seeds;
use mt_capture::testbed::{
    Selection, StepSummary, TestbedOptions, run_capture, run_fetch, run_replay,
};
use tempfile::TempDir;

const SMALL: &str = include_str!("fixtures/manifest-small.toml");

fn repo_file(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn committed_manifest() -> Manifest {
    let manifest = Manifest::load(&repo_file("testbed/manifest.toml")).expect("manifest parses");
    manifest.validate().expect("manifest is valid");
    manifest
}

fn committed_log() -> DropLog {
    DropLog::load(&repo_file("testbed/dropped.toml")).expect("dropped.toml parses")
}

fn small_manifest() -> Manifest {
    let manifest = Manifest::parse(SMALL).expect("the fixture parses");
    manifest.validate().expect("the fixture is valid");
    manifest
}

fn log_text(policy: &str, entries: &str) -> String {
    format!("schema_version = 1\n\n[policy]\nvariant_failure = \"{policy}\"\n\n{entries}\n")
}

/// A well-formed entry for `demo_make`'s variant, with one field replaced.
fn variant_entry(replace: &[(&str, &str)]) -> String {
    let mut fields: Vec<(&str, String)> = vec![
        ("target", "\"demo_make\"".to_owned()),
        ("config", "\"no-float\"".to_owned()),
        ("cause", "\"differs\"".to_owned()),
        ("detail", "\"one object differs\"".to_owned()),
        ("differing_objects", "[\"src/a.o\"]".to_owned()),
        ("differing_sections", "[\".text\"]".to_owned()),
        ("fix_attempted", "\"added -ffile-prefix-map\"".to_owned()),
        ("ci_run", "123".to_owned()),
    ];
    for (key, value) in replace {
        if value.is_empty() {
            fields.retain(|(name, _)| name != key);
        } else if let Some(slot) = fields.iter_mut().find(|(name, _)| name == key) {
            slot.1 = (*value).to_owned();
        } else {
            fields.push((key, (*value).to_owned()));
        }
    }
    let mut text = String::from("[[dropped]]\n");
    for (key, value) in fields {
        text.push_str(&format!("{key} = {value}\n"));
    }
    text
}

/// Parse and validate a drop log against the two-target fixture manifest.
fn checked(text: &str) -> Result<DropLog, DropLogError> {
    let log = DropLog::parse(text)?;
    log.validate(&small_manifest())?;
    Ok(log)
}

#[test]
fn build_02_drop_log_keeps_at_least_ten_targets() {
    let manifest = committed_manifest();
    let log = committed_log();
    log.validate(&manifest)
        .expect("the committed drop log is valid");
    assert_eq!(log.schema_version, DROP_LOG_SCHEMA_VERSION);
    let retained = log.retained_targets(&manifest);
    assert!(
        retained.len() >= MIN_RETAINED,
        "only {} targets are retained, the floor is {MIN_RETAINED}: {retained:?}",
        retained.len()
    );
    // Every retained target is a manifest target, and a dropped target is still
    // named in the manifest (never silently removed).
    let dropped = log.dropped_targets();
    assert_eq!(
        retained.len() + dropped.len(),
        manifest.target.len(),
        "retained {retained:?} and dropped {dropped:?} do not add up to the manifest"
    );
    for id in &dropped {
        assert!(manifest.target.contains_key(id), "{id} left the manifest");
    }
    // The founder's drop granularity (02-03-SUMMARY: drop_granularity variant).
    assert_eq!(log.policy.variant_failure, VariantFailure::DropVariant);
}

#[test]
fn build_02_drop_log_entries_name_manifest_targets() {
    let manifest = committed_manifest();
    let log = committed_log();
    for entry in &log.dropped {
        let target = manifest
            .target
            .get(&entry.target)
            .unwrap_or_else(|| panic!("{} is not a manifest target", entry.target));
        if let Some(config) = &entry.config {
            assert!(
                target.config.iter().any(|c| &c.name == config),
                "{}/{config} is not a configuration of the target",
                entry.target
            );
        }
    }
    // An entry for a target the manifest does not have is refused: a dropped
    // target stays named in the manifest.
    let text = log_text(
        "drop_variant",
        &format!(
            "{}\n",
            variant_entry(&[("target", "\"ghost\""), ("config", "")])
        ),
    );
    let error = checked(&text).expect_err("an unknown target is refused");
    assert!(
        error.to_string().contains("not a target of the manifest"),
        "{error}"
    );
}

#[test]
fn build_02_drop_log_variant_entries_drop_only_the_variant() {
    let manifest = small_manifest();
    let text = log_text(
        "drop_variant",
        &format!(
            "{}\n[[dropped]]\ntarget = \"demo_cmake\"\ncause = \"build_failed\"\n\
             detail = \"cmake configure failed\"\nfix_attempted = \"added a define\"\n\
             ci_run = 7\n",
            variant_entry(&[])
        ),
    );
    let log = checked(&text).expect("a variant entry and a target entry are valid");
    assert_eq!(
        log.retained_targets(&manifest),
        vec!["demo_make".to_owned()]
    );
    assert_eq!(log.dropped_targets(), vec!["demo_cmake".to_owned()]);
    assert!(log.is_dropped("demo_make", "no-float").is_some());
    assert!(
        log.is_dropped("demo_make", "default").is_none(),
        "a dropped variant must leave the default retained"
    );
    assert!(log.is_dropped("demo_cmake", "default").is_some());
    assert!(log.is_dropped("demo_cmake", "depth-8").is_some());
    assert_eq!(
        log.is_dropped("demo_make", "no-float").map(|d| d.cause),
        Some(DropCause::Differs)
    );
    assert_eq!(
        DropCause::VariantNoCodeChange.as_str(),
        "variant_no_code_change"
    );
    assert_eq!(DropCause::EntryMismatch.as_str(), "entry_mismatch");
    assert_eq!(DropCause::NotProven.as_str(), "not_proven");
}

#[test]
fn build_02_drop_log_rejects_bad_entries() {
    // The reference entries are accepted, so each rejection below is caused by
    // the one change it makes.
    checked(&log_text("drop_variant", &variant_entry(&[]))).expect("a good variant entry");
    checked(&log_text(
        "drop_target",
        &variant_entry(&[("cause", "\"variant_no_code_change\"")]),
    ))
    .expect("variant_no_code_change drops a variant under either granularity");

    let bad: Vec<(&str, String)> = vec![
        (
            "an unknown target",
            log_text(
                "drop_variant",
                &variant_entry(&[("target", "\"ghost\""), ("config", "")]),
            ),
        ),
        (
            "the default configuration dropped alone",
            log_text("drop_variant", &variant_entry(&[("config", "\"default\"")])),
        ),
        (
            "a configuration the target does not have",
            log_text("drop_variant", &variant_entry(&[("config", "\"ghost\"")])),
        ),
        (
            "a failing variant under drop_target",
            log_text("drop_target", &variant_entry(&[])),
        ),
        (
            "an empty fix_attempted",
            log_text(
                "drop_variant",
                &variant_entry(&[("fix_attempted", "\"  \"")]),
            ),
        ),
        (
            "a missing fix_attempted",
            log_text("drop_variant", &variant_entry(&[("fix_attempted", "")])),
        ),
        (
            "a non-numeric ci_run",
            log_text("drop_variant", &variant_entry(&[("ci_run", "\"abc\"")])),
        ),
        (
            "a zero ci_run",
            log_text("drop_variant", &variant_entry(&[("ci_run", "0")])),
        ),
        (
            "a duplicate (target, config)",
            log_text(
                "drop_variant",
                &format!("{}\n{}", variant_entry(&[]), variant_entry(&[])),
            ),
        ),
        (
            "an unknown key",
            log_text("drop_variant", &variant_entry(&[("reason", "\"because\"")])),
        ),
        (
            "an unknown cause",
            log_text("drop_variant", &variant_entry(&[("cause", "\"unlucky\"")])),
        ),
        (
            "variant_no_code_change without a configuration",
            log_text(
                "drop_variant",
                &variant_entry(&[("cause", "\"variant_no_code_change\""), ("config", "")]),
            ),
        ),
        (
            "a variant entry next to an entry for its whole target",
            log_text(
                "drop_variant",
                &format!(
                    "{}\n{}",
                    variant_entry(&[]),
                    variant_entry(&[("config", "")])
                ),
            ),
        ),
        (
            "another schema version",
            log_text("drop_variant", "").replace("schema_version = 1", "schema_version = 2"),
        ),
        (
            "an unknown policy key",
            log_text("drop_variant", "").replace(
                "variant_failure = \"drop_variant\"",
                "variant_failure = \"drop_variant\"\nextra = 1",
            ),
        ),
        ("an unknown granularity", log_text("drop_everything", "")),
    ];
    for (what, text) in &bad {
        assert!(checked(text).is_err(), "{what} was accepted:\n{text}");
    }
}

#[test]
fn build_02_dropped_md_matches_drop_log() {
    let log = committed_log();
    let markdown = fs::read_to_string(repo_file("testbed/DROPPED.md")).expect("DROPPED.md exists");
    log.check_markdown(&markdown)
        .expect("DROPPED.md matches dropped.toml");
    assert!(
        markdown.contains("D-11"),
        "DROPPED.md must name the rule it applies"
    );
    for word in ["certified", "compliant"] {
        assert!(
            !markdown.to_lowercase().contains(word),
            "DROPPED.md uses the word {word}"
        );
    }

    // A log with two entries needs exactly their two headings.
    let two = checked(&log_text(
        "drop_variant",
        &format!(
            "{}\n[[dropped]]\ntarget = \"demo_cmake\"\ncause = \"build_failed\"\n\
             detail = \"cmake configure failed\"\nfix_attempted = \"added a define\"\n\
             ci_run = 7\n",
            variant_entry(&[])
        ),
    ))
    .unwrap();
    let both = "# Dropped\n\n# Entries\n\n## demo_make/no-float\n\ntext\n\n## demo_cmake\n\ntext\n";
    assert!(two.check_markdown(both).is_ok());
    let missing = "# Dropped\n\n# Entries\n\n## demo_cmake\n\ntext\n";
    let error = two
        .check_markdown(missing)
        .expect_err("a missing heading is refused");
    assert!(error.to_string().contains("demo_make/no-float"), "{error}");
    let extra = format!("{both}\n## demo_make\n\ntext\n");
    let error = two
        .check_markdown(&extra)
        .expect_err("a heading without an entry is refused");
    assert!(error.to_string().contains("demo_make"), "{error}");
    // Text before the Entries marker may use any headings.
    let prose =
        "# Dropped\n\n## Policy\n\nwords\n\n# Entries\n\n## demo_make/no-float\n\n## demo_cmake\n";
    assert!(two.check_markdown(prose).is_ok());
}

struct Bed {
    opts: TestbedOptions,
    _tmp: TempDir,
}

/// A bed of the two-target fixture manifest with the given `dropped.toml`.
fn make_bed(dropped: &str) -> Bed {
    let tmp = TempDir::new().unwrap();
    let testbed = tmp.path().join("testbed");
    fs::create_dir_all(&testbed).unwrap();
    let manifest_path = testbed.join("manifest.toml");
    fs::write(&manifest_path, SMALL).unwrap();
    fs::write(testbed.join("dropped.toml"), dropped).unwrap();
    let opts = TestbedOptions {
        manifest_path,
        pins_path: repo_file("container/pins.toml"),
        testbed_dir: testbed,
        work_dir: tmp.path().join("work"),
        out_dir: tmp.path().join("out"),
        selection: Selection::All,
        image: None,
    };
    Bed { opts, _tmp: tmp }
}

fn whole_target(target: &str, cause: &str) -> String {
    format!(
        "[[dropped]]\ntarget = \"{target}\"\ncause = \"{cause}\"\ndetail = \"test\"\n\
         fix_attempted = \"a documented fix\"\nci_run = 42\n"
    )
}

fn dropped_entry(cause: DropCause) -> Dropped {
    Dropped {
        target: "demo_make".to_owned(),
        config: Some("no-float".to_owned()),
        cause,
        detail: "test".to_owned(),
        differing_objects: Vec::new(),
        differing_sections: Vec::new(),
        fix_attempted: "a documented fix".to_owned(),
        ci_run: 42,
    }
}

#[test]
fn build_02_dropped_pairs_are_never_reported_as_passing() {
    // The row of a dropped pair says so and never counts for or against `passed`.
    let mut summary = StepSummary::new("replay");
    summary.add("a/default", "identical", true);
    summary.add_dropped("b/no-float", &dropped_entry(DropCause::Differs));
    assert_eq!(
        summary.rows.get("b/no-float").map(String::as_str),
        Some("dropped (differs)")
    );
    assert!(summary.passed, "a dropped pair must not fail the step");
    summary.add("c/default", "differs", false);
    assert!(!summary.passed, "a retained failure still fails the step");
    assert!(summary.render_table().contains("dropped (differs)"));

    // Every target dropped: every step reports every row as dropped, and none as
    // a pass, without starting a tool or touching the network.
    let bed = make_bed(&log_text(
        "drop_variant",
        &format!(
            "{}\n{}",
            whole_target("demo_make", "differs"),
            whole_target("demo_cmake", "build_failed")
        ),
    ));
    let fetch = run_fetch(&bed.opts).unwrap();
    assert_eq!(
        fetch.rows.values().map(String::as_str).collect::<Vec<_>>(),
        vec!["dropped (build_failed)", "dropped (differs)"],
        "{fetch:?}"
    );
    assert!(fetch.passed);
    for summary in [
        run_capture(&bed.opts).unwrap(),
        run_replay(&bed.opts).unwrap(),
    ] {
        assert_eq!(
            summary.rows.len(),
            if summary.step == "capture" { 6 } else { 4 }
        );
        for (row, value) in &summary.rows {
            let expected = if row.starts_with("demo_make/") {
                "dropped (differs)"
            } else {
                "dropped (build_failed)"
            };
            assert_eq!(value, expected, "{} {row}", summary.step);
        }
        assert!(summary.passed, "{summary:?}");
    }

    // Only a variant dropped: the default is still run (here it has no capture
    // report, so it is not proven and fails the step) and the variant row says
    // dropped.
    let bed = make_bed(&log_text(
        "drop_variant",
        &variant_entry(&[("cause", "\"variant_no_code_change\"")]),
    ));
    let replay = run_replay(&bed.opts).unwrap();
    assert_eq!(
        replay.rows.get("demo_make/no-float").map(String::as_str),
        Some("dropped (variant_no_code_change)"),
        "{replay:?}"
    );
    let default = replay.rows.get("demo_make/default").expect("default row");
    assert!(default.starts_with("not_proven"), "{default}");
    assert!(!replay.passed, "a retained pair that is not proven fails");
}

#[test]
fn build_03_seeds_check_skips_fully_dropped_targets() {
    let bed = make_bed(&log_text(
        "drop_variant",
        &format!(
            "{}\n{}",
            whole_target("demo_make", "differs"),
            whole_target("demo_cmake", "fetch_failed")
        ),
    ));
    let seeds_dir = bed.opts.testbed_dir.join("seeds");
    let summary = run_seeds(&bed.opts, &seeds_dir, true).unwrap();
    assert_eq!(
        summary.rows.get("demo_make").map(String::as_str),
        Some("dropped (differs)"),
        "{summary:?}"
    );
    assert_eq!(
        summary.rows.get("demo_cmake").map(String::as_str),
        Some("dropped (fetch_failed)")
    );
    assert!(summary.passed);
    assert!(
        !seeds_dir.exists(),
        "check mode must not create a corpus for a dropped target"
    );
}
