#![allow(clippy::unwrap_used, clippy::expect_used)]
//! BUILD-03: the built `mt testbed seeds` command line, launched through the
//! runner. The source tree is a pre-filled cache of an archive-kind target, which
//! is verified again from files alone, so nothing here needs git, a network or a
//! container.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use mt_capture::treehash::{sha256_hex, tree_sha256};
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

fn os(text: impl Into<OsString>) -> OsString {
    text.into()
}

fn write(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

/// A testbed directory with one archive-kind target and its verified cache.
fn make_bed(root: &Path) -> (PathBuf, PathBuf) {
    let commit = "89abcdef0123456789abcdef0123456789abcdef";
    let work = root.join("work");
    let cache = work.join("cache/demo").join(commit);
    write(&cache.join("tree/tests/a.bin"), &[1, 2, 3]);
    let archive = b"pretend archive";
    write(&cache.join("archive.tar.bz2"), archive);
    let tree_hash = tree_sha256(&cache.join("tree")).unwrap();
    let manifest = format!(
        "schema_version = 1\n\
         [decisions]\n\
         first_rtos = \"deferred\"\n\
         first_rtos_reason = \"fixture\"\n\
         first_rtos_candidates = [\"freertos\"]\n\
         project_decision = \"D-18\"\n\
         [image]\n\
         digest = \"sha256:e83006c0a7e37f788afcbd1c95554e8c4dfc61c3b68da3ff964caf47fb3851b9\"\n\
         [baseline]\n\
         cflags = [\"-mcpu=cortex-m4\", \"-mthumb\", \"-mfloat-abi=hard\", \"-mfpu=fpv4-sp-d16\", \"-Os\"]\n\
         [seed_caps]\n\
         max_seeds = 10\n\
         max_total_bytes = 1000\n\
         max_seed_bytes = 100\n\
         [target.demo]\n\
         category = [\"tlv\"]\n\
         build = \"make\"\n\
         [target.demo.source]\n\
         kind = \"archive\"\n\
         url = \"https://example.com/demo.tar.bz2\"\n\
         sha256 = \"{archive_sha}\"\n\
         strip_components = 1\n\
         ref = \"v1\"\n\
         commit = \"{commit}\"\n\
         tree_sha256 = \"{tree_hash}\"\n\
         [target.demo.licence]\n\
         spdx = \"MIT\"\n\
         elected = \"MIT\"\n\
         file = \"LICENSE\"\n\
         [[target.demo.entry]]\n\
         name = \"demo_parse\"\n\
         header = \"demo.h\"\n\
         source = \"demo.c\"\n\
         form = \"direct\"\n\
         signature = \"int (const unsigned char *, unsigned long)\"\n\
         [[target.demo.config]]\n\
         name = \"default\"\n\
         [[target.demo.seed_source]]\n\
         path = \"tests\"\n\
         methods = [\"file_copy\"]\n",
        archive_sha = sha256_hex(archive),
    );
    let manifest_path = root.join("testbed/manifest.toml");
    write(&manifest_path, manifest.as_bytes());
    (manifest_path, work)
}

fn seeds_args(manifest: &Path, work: &Path, root: &Path, check: bool) -> Vec<OsString> {
    let mut args = vec![
        os("testbed"),
        os("seeds"),
        os("--manifest"),
        manifest.as_os_str().to_owned(),
        os("--pins"),
        repo_root().join("container/pins.toml").into_os_string(),
        os("--work-dir"),
        work.as_os_str().to_owned(),
        os("--out"),
        root.join("out").into_os_string(),
    ];
    if check {
        args.push(os("--check"));
    }
    args.push(os("--all"));
    args
}

#[test]
fn build_03_seeds_help_lists_check_and_seeds_dir() {
    let outcome = run_mt(&repo_root(), &[os("testbed"), os("seeds"), os("--help")]);
    assert_eq!(outcome.code, Some(0), "stderr: {}", outcome.stderr);
    assert!(outcome.stdout.contains("--check"), "{}", outcome.stdout);
    assert!(outcome.stdout.contains("--seeds-dir"), "{}", outcome.stdout);
}

#[test]
fn build_03_seeds_cli_exits_one_on_drift_and_zero_when_equal() {
    let tmp = tempfile::tempdir().unwrap();
    let (manifest, work) = make_bed(tmp.path());

    // Nothing is committed: drift, exit 1, the missing files on standard error.
    let outcome = run_mt(tmp.path(), &seeds_args(&manifest, &work, tmp.path(), true));
    assert_eq!(outcome.code, Some(1), "stdout: {}", outcome.stdout);
    assert!(outcome.stdout.contains("drift"), "{}", outcome.stdout);
    assert!(
        outcome.stderr.contains("drift: demo: missing index.json"),
        "{}",
        outcome.stderr
    );

    // Writing succeeds and defaults the seeds directory to `seeds` next to the manifest.
    let outcome = run_mt(tmp.path(), &seeds_args(&manifest, &work, tmp.path(), false));
    assert_eq!(outcome.code, Some(0), "stderr: {}", outcome.stderr);
    assert!(outcome.stdout.contains("seeds 1"), "{}", outcome.stdout);
    let corpus = tmp.path().join("testbed/seeds/demo");
    assert!(corpus.join("index.json").is_file());
    assert!(corpus.join("NOTICE").is_file());

    let outcome = run_mt(tmp.path(), &seeds_args(&manifest, &work, tmp.path(), true));
    assert_eq!(outcome.code, Some(0), "stderr: {}", outcome.stderr);

    // One byte of one seed changes: exit 1 again, and the file is named.
    fs::write(corpus.join(sha256_hex(&[1, 2, 3])), [1, 2, 4]).unwrap();
    let outcome = run_mt(tmp.path(), &seeds_args(&manifest, &work, tmp.path(), true));
    assert_eq!(outcome.code, Some(1), "stdout: {}", outcome.stdout);
    assert!(
        outcome
            .stderr
            .contains(&format!("drift: demo: changed {}", sha256_hex(&[1, 2, 3]))),
        "{}",
        outcome.stderr
    );
}

#[test]
fn build_03_seeds_cli_rejects_an_unknown_target_with_exit_two() {
    let tmp = tempfile::tempdir().unwrap();
    let (manifest, work) = make_bed(tmp.path());
    let mut args = seeds_args(&manifest, &work, tmp.path(), true);
    args.pop();
    args.push(os("nosuchtarget"));
    let outcome = run_mt(tmp.path(), &args);
    assert_eq!(outcome.code, Some(2), "stdout: {}", outcome.stdout);
    assert!(
        outcome.stderr.contains("nosuchtarget"),
        "{}",
        outcome.stderr
    );
}
