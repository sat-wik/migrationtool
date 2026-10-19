#![allow(clippy::unwrap_used, clippy::expect_used)]
//! BUILD-01 (WR-04): the built `mt testbed capture --stage build|record` and
//! `mt testbed pairs` command lines, launched through the runner. The bed is the
//! two-target fixture manifest with a drop log that drops one variant; nothing
//! here fetches or builds, so no container or Arm tool is needed.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use mt_toolchain::runner::{self, RunRequest, RunnerConfig};

const SMALL: &str = include_str!("../../mt-capture/tests/fixtures/manifest-small.toml");

const DROPPED: &str = "schema_version = 1\n\n[policy]\nvariant_failure = \"drop_variant\"\n\n\
    [[dropped]]\ntarget = \"demo_make\"\nconfig = \"no-float\"\ncause = \"differs\"\n\
    detail = \"one object differs\"\ndiffering_objects = [\"src/a.o\"]\n\
    differing_sections = [\".text\"]\nfix_attempted = \"added -ffile-prefix-map\"\nci_run = 123\n";

struct Outcome {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn run_mt(cwd: &Path, args: &[OsString]) -> Outcome {
    let cfg = RunnerConfig::new("/usr/bin:/bin", "1791158400");
    let mut argv: Vec<OsString> = vec![env!("CARGO_BIN_EXE_mt").into()];
    argv.extend(args.iter().cloned());
    let request = RunRequest {
        tool_name: "mt".to_owned(),
        argv,
        cwd: cwd.to_path_buf(),
        extra_env: BTreeMap::new(),
        tool_version: None,
    };
    let output = runner::run(&cfg, &request).unwrap();
    assert!(!output.record.exit.timed_out, "mt timed out");
    Outcome {
        code: output.record.exit.code,
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

struct Bed {
    _tmp: tempfile::TempDir,
    root: PathBuf,
    manifest: PathBuf,
    work: PathBuf,
}

fn make_bed() -> Bed {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    let testbed = root.join("testbed");
    fs::create_dir_all(&testbed).unwrap();
    let manifest = testbed.join("manifest.toml");
    fs::write(&manifest, SMALL).unwrap();
    fs::write(testbed.join("dropped.toml"), DROPPED).unwrap();
    let work = root.join("work");
    Bed {
        _tmp: tmp,
        root,
        manifest,
        work,
    }
}

/// `mt testbed <subcommand> --manifest .. --pins .. --work-dir .. --out .. <extra>`.
fn testbed(bed: &Bed, subcommand: &[&str], extra: &[&str]) -> Outcome {
    let mut args: Vec<OsString> = vec!["testbed".into()];
    args.extend(subcommand.iter().map(OsString::from));
    for (flag, value) in [
        ("--manifest", bed.manifest.clone()),
        ("--pins", repo_root().join("container/pins.toml")),
        ("--work-dir", bed.work.clone()),
        ("--out", bed.root.join("out")),
    ] {
        args.push(flag.into());
        args.push(value.into_os_string());
    }
    args.extend(extra.iter().map(OsString::from));
    run_mt(&bed.root, &args)
}

#[test]
fn build_01_capture_stage_build_needs_one_pair() {
    let bed = make_bed();
    for extra in [
        vec!["--stage", "build", "--config", "default", "--all"],
        vec![
            "--stage",
            "build",
            "--config",
            "default",
            "demo_make",
            "demo_cmake",
        ],
        vec!["--stage", "build", "demo_make"],
    ] {
        let outcome = testbed(&bed, &["capture"], &extra);

        assert_eq!(
            outcome.code,
            Some(2),
            "{extra:?}: stdout {}",
            outcome.stdout
        );
        assert!(
            outcome.stderr.contains("--stage build")
                && outcome.stderr.contains("exactly one target id")
                && outcome.stderr.contains("--config"),
            "{extra:?}: {}",
            outcome.stderr
        );
        assert!(
            !bed.work.exists(),
            "{extra:?}: the work directory was touched"
        );
    }
}

#[test]
fn build_01_capture_config_needs_stage_build() {
    let bed = make_bed();
    for extra in [
        vec!["--config", "default", "demo_make"],
        vec!["--stage", "record", "--config", "default", "demo_make"],
    ] {
        let outcome = testbed(&bed, &["capture"], &extra);

        assert_eq!(
            outcome.code,
            Some(2),
            "{extra:?}: stdout {}",
            outcome.stdout
        );
        assert!(
            outcome
                .stderr
                .contains("--config is only valid with --stage build"),
            "{extra:?}: {}",
            outcome.stderr
        );
        assert!(
            !bed.work.exists(),
            "{extra:?}: the work directory was touched"
        );
    }
    // The help lists both new arguments.
    let help = run_mt(
        &repo_root(),
        &["testbed".into(), "capture".into(), "--help".into()],
    );
    assert_eq!(help.code, Some(0), "{}", help.stderr);
    assert!(
        help.stdout.contains("--stage") && help.stdout.contains("--config"),
        "{}",
        help.stdout
    );
}

#[test]
fn build_01_pairs_lists_retained_pairs_default_first() {
    let bed = make_bed();

    let outcome = testbed(&bed, &["pairs"], &["demo_make", "demo_cmake"]);

    assert_eq!(outcome.code, Some(0), "stderr: {}", outcome.stderr);
    let lines: Vec<&str> = outcome.stdout.lines().collect();
    assert_eq!(
        lines,
        [
            "demo_cmake default",
            "demo_cmake depth-8",
            "demo_make default"
        ],
        "the dropped variant demo_make/no-float is left out"
    );
    assert!(
        !bed.work.exists(),
        "listing the pairs touched the work directory"
    );
}

#[test]
fn build_01_capture_stage_build_of_a_dropped_pair_does_nothing() {
    let bed = make_bed();

    let outcome = testbed(
        &bed,
        &["capture"],
        &["--stage", "build", "--config", "no-float", "demo_make"],
    );

    assert_eq!(outcome.code, Some(0), "stderr: {}", outcome.stderr);
    assert!(
        outcome.stdout.contains("dropped (differs)"),
        "{}",
        outcome.stdout
    );
    assert!(
        !bed.work.exists(),
        "a dropped pair wrote under the work directory"
    );
    assert!(
        !bed.root.join("out").exists(),
        "a dropped pair wrote an output file"
    );
}

#[test]
fn build_01_capture_stage_record_without_build_records_fails() {
    let bed = make_bed();

    let outcome = testbed(
        &bed,
        &["capture"],
        &["--stage", "record", "demo_make", "demo_cmake"],
    );

    assert_eq!(
        outcome.code,
        Some(1),
        "stdout {} stderr {}",
        outcome.stdout,
        outcome.stderr
    );
    let rows: BTreeMap<&str, String> = outcome
        .stdout
        .lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            let name = words.next()?;
            name.contains('/')
                .then(|| (name, words.collect::<Vec<_>>().join(" ")))
        })
        .collect();
    for retained in [
        "demo_cmake/default",
        "demo_cmake/depth-8",
        "demo_make/default",
    ] {
        let value = rows
            .get(retained)
            .unwrap_or_else(|| panic!("{retained}: {rows:?}"));
        assert!(value.starts_with("not_proven"), "{retained}: {value}");
    }
    assert_eq!(
        rows.get("demo_make/no-float").map(String::as_str),
        Some("dropped (differs)"),
        "{rows:?}"
    );
    assert!(
        outcome.stdout.contains("result: failed"),
        "{}",
        outcome.stdout
    );
}
