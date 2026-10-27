#![allow(clippy::unwrap_used, clippy::expect_used)]
//! BED-01 and BUILD-01: the built `mt testbed` command line, launched through the
//! runner. Only the argument handling, the exit codes and the messages are
//! exercised; nothing here fetches, builds or needs a container.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

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

/// Run the built `mt` with `args` (everything after the program name).
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

fn os(text: &str) -> OsString {
    OsString::from(text)
}

fn committed_manifest() -> String {
    std::fs::read_to_string(repo_root().join("testbed/manifest.toml")).unwrap()
}

#[test]
fn bed_01_testbed_validate_accepts_the_committed_manifest() {
    let manifest = repo_root().join("testbed/manifest.toml");
    let outcome = run_mt(
        &repo_root(),
        &[
            os("testbed"),
            os("validate"),
            os("--manifest"),
            manifest.into_os_string(),
        ],
    );
    assert_eq!(outcome.code, Some(0), "stderr: {}", outcome.stderr);
    assert!(
        outcome.stdout.contains("step: validate"),
        "{}",
        outcome.stdout
    );
    assert!(
        outcome.stdout.contains("valid (16 targets"),
        "{}",
        outcome.stdout
    );
    assert!(
        outcome.stdout.contains("result: passed"),
        "{}",
        outcome.stdout
    );
}

#[test]
fn bed_01_testbed_validate_rejects_an_invalid_manifest_with_exit_two() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("manifest.toml");
    let text = committed_manifest();
    // The image digest moves with each published image (README `## Current
    // image`), so read it from the committed manifest instead of naming it here.
    let good = text
        .lines()
        .find_map(|line| {
            line.strip_prefix("digest = \"")
                .and_then(|rest| rest.strip_suffix('"'))
        })
        .filter(|digest| digest.starts_with("sha256:"))
        .unwrap_or_default();
    assert!(
        !good.is_empty() && text.contains(good),
        "the committed manifest names the image"
    );
    std::fs::write(&path, text.replace(good, "sha256:not-a-digest")).unwrap();

    let outcome = run_mt(
        dir.path(),
        &[
            os("testbed"),
            os("validate"),
            os("--manifest"),
            path.into_os_string(),
        ],
    );
    assert_eq!(outcome.code, Some(2), "stdout: {}", outcome.stdout);
    assert!(
        outcome
            .stderr
            .contains("error: invalid manifest: image.digest"),
        "{}",
        outcome.stderr
    );

    let missing = dir.path().join("absent.toml");
    let outcome = run_mt(
        dir.path(),
        &[
            os("testbed"),
            os("validate"),
            os("--manifest"),
            missing.into_os_string(),
        ],
    );
    assert_eq!(outcome.code, Some(2));
}

#[test]
fn build_01_testbed_capture_of_a_target_without_build_args_exits_two() {
    let dir = tempfile::tempdir().unwrap();
    let text = committed_manifest();
    let start = text
        .find("[target.qcbor.build_args]")
        .expect("the tracer target has a recipe");
    let end = start + text[start..].find("[[target.qcbor.entry]]").unwrap();
    let stripped = format!("{}{}", &text[..start], &text[end..]);
    assert!(!stripped.contains("[target.qcbor.build_args]"));
    let manifest = dir.path().join("manifest.toml");
    std::fs::write(&manifest, stripped).unwrap();
    let pins = repo_root().join("container/pins.toml");

    for step in ["capture", "replay"] {
        let outcome = run_mt(
            dir.path(),
            &[
                os("testbed"),
                os(step),
                os("--manifest"),
                manifest.clone().into_os_string(),
                os("--pins"),
                pins.clone().into_os_string(),
                os("--work-dir"),
                dir.path().join("work").into_os_string(),
                os("--out"),
                dir.path().join("out").into_os_string(),
                os("qcbor"),
            ],
        );
        assert_eq!(outcome.code, Some(2), "{step}: stdout {}", outcome.stdout);
        assert!(
            outcome.stderr.contains("qcbor") && outcome.stderr.contains("no build_args recipe"),
            "{step}: {}",
            outcome.stderr
        );
        assert!(
            !dir.path().join("work").exists(),
            "{step} touched the work directory before refusing"
        );
    }
}
