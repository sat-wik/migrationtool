#![allow(clippy::unwrap_used, clippy::expect_used)]
//! EQV-01: the built `mt vectors policy-check` command line, launched through
//! the runner. Only argument handling, exit codes and messages are exercised;
//! the rules are tested in `mt-contract`.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use mt_capture::manifest::Manifest;
use mt_toolchain::runner::{self, RunRequest, RunnerConfig};

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

fn run_mt(args: &[&str]) -> Outcome {
    let cfg = RunnerConfig::new("/usr/bin:/bin", "1791158400");
    let mut argv: Vec<OsString> = vec![env!("CARGO_BIN_EXE_mt").into()];
    argv.extend(args.iter().map(OsString::from));
    let request = RunRequest {
        tool_name: "mt".to_owned(),
        argv,
        cwd: repo_root(),
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

#[test]
fn eqv_01_policy_check_cli_accepts_the_corejson_policy() {
    let outcome = run_mt(&["vectors", "policy-check", "testbed/policy/corejson.toml"]);
    assert_eq!(outcome.code, Some(0), "stderr: {}", outcome.stderr);
    assert!(
        outcome
            .stdout
            .lines()
            .any(|l| l == "corejson valid default=return_error overrides=0 error_codes=0"),
        "stdout: {}",
        outcome.stdout
    );
}

#[test]
fn eqv_01_policy_check_cli_rejects_a_bad_policy() {
    let dir = tempfile::tempdir().unwrap();
    let bad = "schema_version = 1\ncontract_version = \"0.1.0\"\nmodule = \"corejson\"\n\n\
               [ub]\ndefault_action = \"maybe\"\n";
    let path = dir.path().join("corejson.toml");
    std::fs::write(&path, bad).unwrap();
    let outcome = run_mt(&["vectors", "policy-check", path.to_str().unwrap()]);
    assert_eq!(outcome.code, Some(2), "stderr: {}", outcome.stderr);
    assert!(
        outcome.stderr.contains("default_action") || outcome.stderr.contains("maybe"),
        "the refusal must name the key: {}",
        outcome.stderr
    );
    assert!(
        !outcome.stderr.contains("unrecognized"),
        "the subcommand must exist: {}",
        outcome.stderr
    );
}

/// The ids of the manifest's targets, in id order.
fn manifest_ids() -> Vec<String> {
    Manifest::load(&repo_root().join("testbed/manifest.toml"))
        .unwrap()
        .target
        .keys()
        .cloned()
        .collect()
}

/// A tempdir holding a copy of the manifest and of `testbed/policy`.
fn testbed_copy() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let src = repo_root().join("testbed");
    std::fs::copy(src.join("manifest.toml"), dir.path().join("manifest.toml")).unwrap();
    std::fs::create_dir(dir.path().join("policy")).unwrap();
    for entry in std::fs::read_dir(src.join("policy")).unwrap() {
        let path = entry.unwrap().path();
        std::fs::copy(
            &path,
            dir.path().join("policy").join(path.file_name().unwrap()),
        )
        .unwrap();
    }
    dir
}

#[test]
fn eqv_01_policy_check_cli_checks_every_target() {
    let ids = manifest_ids();
    assert_eq!(ids.len(), 16);
    let outcome = run_mt(&[
        "vectors",
        "policy-check",
        "--manifest",
        "testbed/manifest.toml",
    ]);
    assert_eq!(outcome.code, Some(0), "stderr: {}", outcome.stderr);
    let rows: Vec<&str> = outcome.stdout.lines().collect();
    assert_eq!(rows.len(), 16, "stdout: {}", outcome.stdout);
    for (row, id) in rows.iter().zip(&ids) {
        assert_eq!(
            *row,
            format!("{id} valid default=return_error overrides=0 error_codes=0")
        );
    }

    // A target without a policy.
    let missing = testbed_copy();
    std::fs::remove_file(missing.path().join("policy/zlib.toml")).unwrap();
    let manifest = missing.path().join("manifest.toml");
    let outcome = run_mt(&[
        "vectors",
        "policy-check",
        "--manifest",
        manifest.to_str().unwrap(),
    ]);
    assert_eq!(outcome.code, Some(2), "stderr: {}", outcome.stderr);
    assert!(
        outcome.stderr.contains("zlib"),
        "must name the target: {}",
        outcome.stderr
    );

    // A policy that names no target.
    let stray = testbed_copy();
    let text = std::fs::read_to_string(stray.path().join("policy/corejson.toml")).unwrap();
    std::fs::write(
        stray.path().join("policy/stray.toml"),
        text.replace("module = \"corejson\"", "module = \"stray\""),
    )
    .unwrap();
    let manifest = stray.path().join("manifest.toml");
    let outcome = run_mt(&[
        "vectors",
        "policy-check",
        "--manifest",
        manifest.to_str().unwrap(),
    ]);
    assert_eq!(outcome.code, Some(2), "stderr: {}", outcome.stderr);
    assert!(
        outcome.stderr.contains("stray"),
        "must name the file: {}",
        outcome.stderr
    );

    // Neither a manifest nor a file.
    let outcome = run_mt(&["vectors", "policy-check"]);
    assert_eq!(outcome.code, Some(2), "stderr: {}", outcome.stderr);
}
