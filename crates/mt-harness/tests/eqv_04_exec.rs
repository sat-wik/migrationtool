#![allow(clippy::unwrap_used, clippy::expect_used)]
//! EQV-03, EQV-04: the entry drivers built from the manifest's roles and the
//! captured objects, run twice, and the corpora they produce.
//!
//! Every test here is `#[ignore]`d: it needs the local captures of an earlier
//! run (`MT_LOCAL_CAPTURES=<work>/capture`) and the host Arm tools. Run with
//! `-- --ignored --nocapture`. Under `--ignored` a test fails, not skips, when
//! the variable or a tool is missing.

use std::path::{Path, PathBuf};

use mt_capture::manifest::Manifest;
use mt_contract::CONTRACT_VERSION;
use mt_harness::EXECUTOR_VERSION;
use mt_harness::replay::check_corpora;
use mt_harness::run::{
    BuildOptions, BuildState, EntryRunSummary, RunOptions, RunState, Scope, build_entries,
    run_entries,
};
use mt_harness::tools::ExecTools;
use mt_toolchain::runner::RunnerConfig;
use mt_vectors::bytes::decode_latin1;
use mt_vectors::corpus::{HasUbSource, read_corpus};
use mt_vectors::sha256_hex;
use mt_vectors::vector::Vector;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// The work directory whose `capture` directory is `MT_LOCAL_CAPTURES`.
fn local_work_dir() -> PathBuf {
    let captures = std::env::var("MT_LOCAL_CAPTURES")
        .expect("MT_LOCAL_CAPTURES must name the directory of the local captures");
    assert!(!captures.is_empty(), "MT_LOCAL_CAPTURES is empty");
    let captures = PathBuf::from(captures);
    assert!(
        captures.ends_with("capture"),
        "MT_LOCAL_CAPTURES must be <work>/capture, found {}",
        captures.display()
    );
    captures
        .parent()
        .expect("the captures directory has a parent")
        .to_path_buf()
}

fn host_tools() -> ExecTools {
    for needed in [
        "/usr/bin/qemu-arm",
        "/usr/bin/arm-none-eabi-gcc",
        "/usr/bin/arm-none-eabi-ar",
        "/usr/lib/arm-none-eabi/include",
    ] {
        assert!(
            Path::new(needed).exists(),
            "{needed} is not installed on this host"
        );
    }
    ExecTools::at(
        Path::new("/usr/bin"),
        Path::new("/usr/bin/qemu-arm"),
        Path::new("/usr/bin/llvm-dwarfdump"),
    )
}

fn runner_config() -> RunnerConfig {
    RunnerConfig::new("/usr/bin:/bin", "1791158400")
}

fn scope_for(
    target: &str,
    config: &str,
    entry: &str,
    build_root: &Path,
    work: &Path,
) -> Scope {
    let mut scope = Scope::new(repo_root().join("testbed/manifest.toml"), work.to_path_buf());
    scope.pairs = vec![(target.to_owned(), config.to_owned())];
    scope.entries = Some(vec![entry.to_owned()]);
    scope.build_root = build_root.to_path_buf();
    scope
}

fn build_ok(scope: &Scope) {
    let rows = build_entries(&BuildOptions {
        scope: scope.clone(),
        tools: host_tools(),
        cfg: runner_config(),
        replay_dir: None,
    })
    .unwrap();
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert!(
        matches!(rows[0].state, BuildState::Built { .. }),
        "the driver must build: {rows:?}"
    );
}

fn run_into(scope: &Scope, corpus: &Path, scratch: &Path) -> EntryRunSummary {
    let rows = run_entries(&RunOptions {
        scope: scope.clone(),
        tools: host_tools(),
        cfg: runner_config(),
        corpus_dir: corpus.to_path_buf(),
        scratch_dir: scratch.to_path_buf(),
    })
    .unwrap();
    assert_eq!(rows.len(), 1, "{rows:?}");
    match rows.into_iter().next().unwrap().state {
        RunState::Ran(summary) => summary,
        other => panic!("the entry must run, got {other:?}"),
    }
}

#[test]
#[ignore = "reads the captures of an earlier local run; set MT_LOCAL_CAPTURES"]
fn eqv_04_run_twice_is_byte_identical() {
    let work = local_work_dir();
    let tmp = tempfile::tempdir().unwrap();
    let scope = scope_for(
        "corejson",
        "default",
        "JSON_Validate",
        &tmp.path().join("build"),
        &work,
    );
    build_ok(&scope);
    let corpus_a = tmp.path().join("a");
    let corpus_b = tmp.path().join("b");
    let first = run_into(&scope, &corpus_a, &tmp.path().join("slots-a"));
    let second = run_into(&scope, &corpus_b, &tmp.path().join("slots-b"));
    assert_eq!(first, second, "two full runs give the same summary");
    assert!(first.vectors >= 10, "{first:?}");
    assert!(first.twice_identical, "{first:?}");

    // Two separate runs of every seed give byte-identical corpora.
    let comparison = check_corpora(&corpus_a, &corpus_b).unwrap();
    assert!(comparison.identical(), "{:?}", comparison.problems);
    assert!(comparison.files > first.vectors, "{comparison:?}");

    let policy_bytes = std::fs::read(repo_root().join("testbed/policy/corejson.toml")).unwrap();
    let policy_sha = sha256_hex(&policy_bytes);
    let corpus = read_corpus(&corpus_a).unwrap();
    assert_eq!(corpus.index.target, "corejson");
    assert_eq!(corpus.index.config, "default");
    assert_eq!(corpus.index.entry, "JSON_Validate");
    assert_eq!(corpus.vectors.len(), first.vectors);
    for (sha, vector) in &corpus.vectors {
        // The stored file parses at the reference level and is its own hash.
        let bytes = std::fs::read(corpus_a.join(format!("{sha}.json"))).unwrap();
        assert_eq!(&Vector::from_json_bytes(&bytes).unwrap(), vector);
        assert_eq!(&sha256_hex(&bytes), sha);

        let stdin_len = decode_latin1(&vector.stdin).unwrap().len();
        assert_eq!(
            vector.argv,
            [
                "corejson/default/JSON_Validate".to_owned(),
                stdin_len.to_string()
            ],
            "D-05: argv[0] names the entry, argv[1] is the input length"
        );
        assert!(vector.env.is_empty());
        assert_eq!(vector.stdout.pattern, "", "no output buffer: stdout is empty");

        let class = vector.outcome_class().unwrap().word();
        let provenance = &corpus.index.vectors[sha];
        let labelled = matches!(class, "fault" | "abort" | "timeout");
        assert_eq!(
            vector.has_ub.is_some(),
            labelled,
            "a label only on fault, abort or timeout (class {class})"
        );
        if !labelled {
            assert_eq!(
                provenance.has_ub_source,
                HasUbSource::Unchecked,
                "an unlabelled vector is UB unchecked (D-20)"
            );
        }
        assert_eq!(provenance.policy_sha256, policy_sha);
        assert_eq!(provenance.image, "local");
        assert_eq!(provenance.replay, "not_checked");
        assert_eq!(provenance.contract_version, CONTRACT_VERSION);
        assert_eq!(provenance.executor_version, EXECUTOR_VERSION);
        assert_eq!(provenance.outcome, class);
        assert!(!provenance.objects.is_empty());
        assert!(provenance.scalars.is_empty(), "JSON_Validate has no scalar");
    }
    println!(
        "corejson/default/JSON_Validate vectors={} identical",
        first.vectors
    );
}

#[test]
#[ignore = "reads the captures of an earlier local run; set MT_LOCAL_CAPTURES"]
fn eqv_04_build_refuses_a_pair_without_roles() {
    let work = local_work_dir();
    let tmp = tempfile::tempdir().unwrap();
    let scope = scope_for(
        "corejson",
        "default",
        "JSON_SearchT",
        &tmp.path().join("build"),
        &work,
    );
    let rows = build_entries(&BuildOptions {
        scope,
        tools: host_tools(),
        cfg: runner_config(),
        replay_dir: None,
    })
    .unwrap();
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0].state, BuildState::NoRoles);
    assert_eq!(rows[0].render(), "corejson/default/JSON_SearchT no_roles");
}

#[test]
fn eqv_04_scope_selects_entries_with_roles() {
    // Pure: the scope's defaults are the work directory's build root and every
    // entry with roles (no capture is read).
    let scope = Scope::new(PathBuf::from("testbed/manifest.toml"), PathBuf::from("/w"));
    assert_eq!(scope.build_root, PathBuf::from("/w/vectors-build"));
    assert!(scope.entries.is_none());
    assert!(scope.pairs.is_empty());
    let manifest = Manifest::load(&repo_root().join("testbed/manifest.toml")).unwrap();
    assert!(manifest.target.contains_key("corejson"));
}
