#![allow(clippy::unwrap_used, clippy::expect_used)]
//! EQV-03, EQV-04: the built `mt vectors adapters` command line, launched
//! through the runner. Only argument handling, exit codes and rows are
//! exercised; the rules are tested in `mt-capture` and `mt-harness`.

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
fn eqv_04_adapters_cli_lists_entries() {
    let outcome = run_mt(&["vectors", "adapters", "--manifest", "testbed/manifest.toml"]);
    assert_eq!(outcome.code, Some(0), "stderr: {}", outcome.stderr);
    let rows: Vec<&str> = outcome.stdout.lines().collect();
    assert!(
        rows.contains(&"corejson/JSON_Validate roles=2 seeds=108 drivable"),
        "stdout: {}",
        outcome.stdout
    );
    assert!(
        rows.iter()
            .any(|row| row.starts_with("corejson/JSON_SearchT no_roles")),
        "an entry without roles prints a no_roles row: {}",
        outcome.stdout
    );
    let last = rows.last().copied().unwrap_or_default();
    assert!(
        last.starts_with("result: passed ("),
        "the output ends with the result line: {last}"
    );
}

#[test]
fn eqv_04_adapters_cli_needs_a_manifest() {
    let outcome = run_mt(&["vectors", "adapters"]);
    assert_eq!(outcome.code, Some(2), "stderr: {}", outcome.stderr);
    assert!(
        outcome.stderr.contains("--manifest"),
        "the refusal names the missing option: {}",
        outcome.stderr
    );
}

fn write_tree(root: &Path, files: &[(&str, &str)]) {
    for (name, text) in files {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
}

#[test]
fn eqv_04_check_cli_fails_closed() {
    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("first");
    let second = dir.path().join("second");
    let files = [("t/c/e/index.json", "{}\n"), ("t/c/e/aaaa.json", "one\n")];
    write_tree(&first, &files);
    write_tree(&second, &files);
    let run = |first: &Path, second: &Path| {
        run_mt(&[
            "vectors",
            "check",
            "--first",
            first.to_str().unwrap(),
            "--second",
            second.to_str().unwrap(),
        ])
    };

    // Identical trees: exit 0 and the count.
    let same = run(&first, &second);
    assert_eq!(same.code, Some(0), "stderr: {}", same.stderr);
    assert!(
        same.stdout.lines().any(|l| l == "identical 2 files"),
        "{}",
        same.stdout
    );
    assert!(same.stdout.contains("result: passed"), "{}", same.stdout);

    // A differing byte, a missing file and an extra file each exit 1 and name
    // the file.
    write_tree(&second, &[("t/c/e/aaaa.json", "ONE\n")]);
    let differing = run(&first, &second);
    assert_eq!(differing.code, Some(1), "stdout: {}", differing.stdout);
    assert!(
        differing.stdout.contains("t/c/e/aaaa.json"),
        "{}",
        differing.stdout
    );
    assert!(
        differing.stdout.contains("result: failed"),
        "{}",
        differing.stdout
    );

    write_tree(&second, &[("t/c/e/aaaa.json", "one\n")]);
    std::fs::remove_file(second.join("t/c/e/index.json")).unwrap();
    let missing = run(&first, &second);
    assert_eq!(missing.code, Some(1), "stdout: {}", missing.stdout);
    assert!(
        missing.stdout.contains("t/c/e/index.json"),
        "{}",
        missing.stdout
    );

    write_tree(
        &second,
        &[("t/c/e/index.json", "{}\n"), ("t/c/e/bbbb.json", "x\n")],
    );
    let extra = run(&first, &second);
    assert_eq!(extra.code, Some(1), "stdout: {}", extra.stdout);
    assert!(extra.stdout.contains("t/c/e/bbbb.json"), "{}", extra.stdout);

    // An empty pair of trees compares nothing, which is not a pass; a tree that
    // is not there is invalid input.
    let empty_a = dir.path().join("empty-a");
    let empty_b = dir.path().join("empty-b");
    std::fs::create_dir_all(&empty_a).unwrap();
    std::fs::create_dir_all(&empty_b).unwrap();
    assert_eq!(run(&empty_a, &empty_b).code, Some(1));
    let absent = run(&first, &dir.path().join("absent"));
    assert_eq!(absent.code, Some(2), "stderr: {}", absent.stderr);
}

#[test]
fn eqv_04_layouts_cli_prints_the_layouts_of_built_entries() {
    let work = tempfile::tempdir().unwrap();
    // A minimal build.json: the states' layouts are all `mt vectors layouts` reads.
    let one = |target: &str, config: &str, entry: &str, name: &str, size: u32| {
        let text = format!(
            r#"{{"schema_version":1,"target":"{target}","config":"{config}","entry":"{entry}","image":"local","replay":"not_checked","objects":{{}},"archive_sha256":"","driver_source_sha256":"","driver_elf_sha256":"","include_lines":[],"driver_flags":[],"steps":[],"layouts":{{"{name}":{{"text":"struct pair","size":{size},"kind":{{"kind":"struct","members":[{{"name":"a","ty":{{"text":"uint8_t","size":1,"kind":{{"kind":"int","signed":false}}}},"offset":0,"bits":null}}]}}}}}}}}"#
        );
        let dir = work
            .path()
            .join("vectors-build")
            .join(target)
            .join(config)
            .join(entry);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("build.json"), text).unwrap();
    };
    one("t2", "default", "g", "st", 8);
    one("t1", "default", "f", "state", 4);
    let outcome = run_mt(&[
        "vectors",
        "layouts",
        "--work-dir",
        work.path().to_str().unwrap(),
    ]);
    assert_eq!(outcome.code, Some(0), "stderr: {}", outcome.stderr);
    let rows: Vec<&str> = outcome.stdout.lines().collect();
    assert_eq!(
        rows,
        [
            "t1/default/f state struct pair size=4 a@0:1",
            "t2/default/g st struct pair size=8 a@0:1",
            "result: passed (2 rows)",
        ],
        "rows are in target, configuration and entry order"
    );
}

#[test]
fn eqv_04_layouts_cli_fails_closed_without_a_build() {
    let work = tempfile::tempdir().unwrap();
    let outcome = run_mt(&[
        "vectors",
        "layouts",
        "--work-dir",
        work.path().to_str().unwrap(),
    ]);
    assert_eq!(outcome.code, Some(1), "stderr: {}", outcome.stderr);
    assert!(
        outcome.stdout.lines().last() == Some("result: failed"),
        "a missing layer is never green: {}",
        outcome.stdout
    );
}
