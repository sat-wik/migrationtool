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
    // Every entry is drivable or logged (plan 03-08), so a logged entry prints its
    // cause and no entry prints a no_roles row.
    assert!(
        rows.contains(&"nanopb/pb_decode_ex dropped (not_expressible)"),
        "a logged entry prints a dropped row: {}",
        outcome.stdout
    );
    assert!(
        !rows.iter().any(|row| row.contains(" no_roles ")),
        "no committed entry is left without roles and without a drop row: {}",
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

#[test]
fn eqv_04_adapters_cli_checks_the_floor() {
    // D-13, D-24: `--check-floor` adds the count of drivable targets and fails
    // below the floor of 10.
    let ok = run_mt(&[
        "vectors",
        "adapters",
        "--manifest",
        "testbed/manifest.toml",
        "--check-floor",
    ]);
    assert_eq!(ok.code, Some(0), "stderr: {}", ok.stderr);
    let count = ok
        .stdout
        .lines()
        .find_map(|line| {
            line.strip_prefix("drivable targets: ")?
                .strip_suffix(" (floor 10)")?
                .parse::<usize>()
                .ok()
        })
        .unwrap_or_else(|| panic!("no floor line in {}", ok.stdout));
    assert!(count >= 10, "{count}");
    assert!(
        ok.stdout
            .lines()
            .any(|row| row == "zcbor/cbor_decode_Pet dropped (no_seeds)"),
        "a logged entry prints a dropped row: {}",
        ok.stdout
    );

    // A copy of the testbed whose adapter drop log drops six drivable targets, so the
    // count falls below the floor of 10 however many targets later plans add.
    let tmp = tempfile::tempdir().unwrap();
    let copy = tmp.path().join("testbed");
    let from = repo_root().join("testbed");
    std::fs::create_dir_all(&copy).unwrap();
    for file in ["manifest.toml", "dropped.toml"] {
        std::fs::copy(from.join(file), copy.join(file)).unwrap();
    }
    for item in std::fs::read_dir(from.join("seeds")).unwrap() {
        let item = item.unwrap();
        let index = item.path().join("index.json");
        if index.is_file() {
            let dir = copy.join("seeds").join(item.file_name());
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::copy(&index, dir.join("index.json")).unwrap();
        }
    }
    // Every entry with roles of six targets is logged, so each of them falls out of
    // the count however many entries later plans give them.
    let manifest_of = mt_capture::manifest::Manifest::load(&from.join("manifest.toml")).unwrap();
    let mut log = String::from("schema_version = 1\n");
    for (target, entry) in ["lz4", "libfdt", "microcoap", "nimble", "nanopb", "zlib"]
        .iter()
        .flat_map(|target| {
            manifest_of.target[*target]
                .entry
                .iter()
                .filter(|entry| !entry.arg.is_empty())
                .map(move |entry| (*target, entry.name.clone()))
        })
    {
        log.push_str(&format!(
            "\n[[dropped]]\ntarget = \"{target}\"\nentry = \"{entry}\"\ncause = \"attempt_failed\"\ndetail = \"x\"\nattempt = \"y\"\n"
        ));
    }
    std::fs::write(copy.join("adapter-dropped.toml"), log).unwrap();
    let manifest = copy.join("manifest.toml");
    let below = run_mt(&[
        "vectors",
        "adapters",
        "--manifest",
        manifest.to_str().unwrap(),
        "--check-floor",
    ]);
    assert_eq!(below.code, Some(1), "stdout: {}", below.stdout);
    assert!(
        below
            .stdout
            .lines()
            .any(|l| l.starts_with("nanopb/pb_decode_ex no_roles")),
        "an entry without roles and without a drop row prints a no_roles row: {}",
        below.stdout
    );
    assert!(
        below
            .stdout
            .lines()
            .any(|l| *l == format!("drivable targets: {} (floor 10)", count - 6)),
        "{}",
        below.stdout
    );
    assert!(
        below
            .stdout
            .lines()
            .last()
            .is_some_and(|l| l == "result: failed")
    );
}

// ---------------------------------------------------------------------------
// Plan 03-08: `mt vectors sample` (CONTEXT D-21)
// ---------------------------------------------------------------------------

mod sample {
    use std::collections::BTreeMap;
    use std::path::Path;

    use mt_vectors::corpus::{
        HasUbSource, Provenance, ProvenanceBudgets, read_corpus, write_corpus,
    };
    use mt_vectors::vector::Vector;

    use super::run_mt;

    const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HASH_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const HASH_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

    fn vector(argv: &[&str], stdin: &str) -> Vector {
        let argv_json = argv
            .iter()
            .map(|word| format!("{word:?}"))
            .collect::<Vec<_>>()
            .join(",");
        let text = format!(
            "{{\"argv\":[{argv_json}],\"stdin\":{stdin:?},\"env\":{{}},\"stdout\":{{\"pattern\":\"\",\"is_regex\":false}},\"stderr\":{{\"pattern\":\"outcome=ok\",\"is_regex\":false}},\"rc\":0,\"lib_state_in\":{{}},\"lib_state_out\":{{}}}}"
        );
        Vector::from_json_bytes(text.as_bytes()).unwrap()
    }

    fn provenance(seed: &str, scalars: &[(&str, i64)]) -> Provenance {
        Provenance {
            seed: seed.to_owned(),
            scalars: scalars
                .iter()
                .map(|(name, value)| ((*name).to_owned(), *value))
                .collect::<BTreeMap<_, _>>(),
            objects: BTreeMap::from([("build/x.o".to_owned(), HASH_B.to_owned())]),
            archive: HASH_A.to_owned(),
            driver_source: HASH_B.to_owned(),
            driver_elf: HASH_A.to_owned(),
            image: "ghcr.io/example/toolchain@sha256:0000".to_owned(),
            executor_version: "0.1.0".to_owned(),
            contract_version: "0.1.0".to_owned(),
            policy_sha256: HASH_B.to_owned(),
            budgets: ProvenanceBudgets {
                timeout_ms: 10_000,
                reserved_va: 0x1000_0000,
                heap_arena: 1 << 20,
                sentinel: 0xA5,
                slack_fill: 0x5A,
                max_input: 65_536,
            },
            outcome: "ok".to_owned(),
            has_ub_source: HasUbSource::Unchecked,
            unobserved: 0,
            discarded_output_bytes: 0,
            replay: "identical".to_owned(),
        }
    }

    /// A corpus of `<target>/<config>/<entry>` with a vector for each seed and each
    /// query index (`queries` > 0) or one per seed (`queries` = 0).
    fn write(root: &Path, key: &str, seeds: &[(&str, &str)], queries: usize) {
        let argv0 = key.to_owned();
        let mut set = Vec::new();
        for (hash, input) in seeds {
            let combos: Vec<Option<usize>> = if queries == 0 {
                vec![None]
            } else {
                (0..queries).map(Some).collect()
            };
            for combo in combos {
                let length = input.len().to_string();
                let mut argv = vec![argv0.as_str(), length.as_str()];
                let query = combo.map(|index| format!("query{index}"));
                if let Some(text) = &query {
                    argv.push(text.as_str());
                }
                let scalars: Vec<(&str, i64)> = combo
                    .map(|index| ("query", i64::try_from(index).unwrap()))
                    .into_iter()
                    .collect();
                set.push((vector(&argv, input), provenance(hash, &scalars)));
            }
        }
        let dir = root.join(key);
        std::fs::create_dir_all(dir.parent().unwrap()).unwrap();
        write_corpus(&dir, &set).unwrap();
    }

    #[test]
    fn eqv_04_sample_cli_selects_one_vector_per_target() {
        let tmp = tempfile::tempdir().unwrap();
        let corpus = tmp.path().join("corpus");
        // corejson: JSON_SearchT sweeps a string (three queries); its seeds are 4, 2,
        // 1 and 1 bytes, the two shortest told apart by their sha256. Another
        // entry and another configuration must not be chosen.
        write(
            &corpus,
            "corejson/default/JSON_SearchT",
            &[
                (HASH_B, "aaaa"),
                (HASH_C, "bb"),
                (HASH_B, "c"),
                (HASH_A, "d"),
            ],
            3,
        );
        write(
            &corpus,
            "corejson/default/JSON_Validate",
            &[(HASH_A, "x")],
            0,
        );
        write(
            &corpus,
            "corejson/depth-8/JSON_SearchT",
            &[(HASH_A, "e")],
            3,
        );
        // lz4: the first entry by name is LZ4F_getFrameInfo, which has no scalar.
        write(
            &corpus,
            "lz4/default/LZ4F_getFrameInfo",
            &[(HASH_A, "abc"), (HASH_B, "xy")],
            0,
        );
        write(
            &corpus,
            "lz4/default/LZ4_decompress_safe",
            &[(HASH_A, "z")],
            0,
        );

        let out = tmp.path().join("out");
        let args = |out: &Path, ids: &[&str]| -> Vec<String> {
            let mut args: Vec<String> = vec![
                "vectors".into(),
                "sample".into(),
                "--manifest".into(),
                "testbed/manifest.toml".into(),
                "--corpus-dir".into(),
                corpus.to_str().unwrap().into(),
                "--out".into(),
                out.to_str().unwrap().into(),
            ];
            args.extend(ids.iter().map(|id| (*id).to_owned()));
            args
        };
        let run = |out: &Path, ids: &[&str]| {
            let owned = args(out, ids);
            let refs: Vec<&str> = owned.iter().map(String::as_str).collect();
            run_mt(&refs)
        };

        let selected = run(&out, &["corejson", "lz4"]);
        assert_eq!(selected.code, Some(0), "stderr: {}", selected.stderr);
        let rows: Vec<&str> = selected.stdout.lines().collect();
        assert_eq!(rows.len(), 3, "{}", selected.stdout);
        assert!(
            rows[0].starts_with("corejson/default/JSON_SearchT ")
                && rows[0].contains(&format!("seed={HASH_A} bytes=1")),
            "the smallest seed, ties by sha256: {}",
            rows[0]
        );
        assert!(
            rows[1].starts_with("lz4/default/LZ4F_getFrameInfo ")
                && rows[1].contains(&format!("seed={HASH_B} bytes=2")),
            "{}",
            rows[1]
        );
        assert_eq!(rows[2], "result: passed (2 rows)");

        // One vector per target, in a one-vector corpus whose provenance is the
        // CI corpus's own, byte for byte.
        let sampled = read_corpus(&out.join("corejson/default/JSON_SearchT")).unwrap();
        assert_eq!(sampled.vectors.len(), 1);
        let (sha, picked) = sampled.vectors.iter().next().unwrap();
        assert!(rows[0].contains(sha.as_str()), "{} {sha}", rows[0]);
        assert_eq!(picked.stdin, "d");
        let record = &sampled.index.vectors[sha];
        assert_eq!(record.seed, HASH_A);
        assert_eq!(
            record.scalars,
            BTreeMap::from([("query".to_owned(), 0)]),
            "the first combination"
        );
        let original = std::fs::read(
            corpus
                .join("corejson/default/JSON_SearchT")
                .join(format!("{sha}.json")),
        )
        .unwrap();
        let copy = std::fs::read(
            out.join("corejson/default/JSON_SearchT")
                .join(format!("{sha}.json")),
        )
        .unwrap();
        assert_eq!(original, copy, "the vector file is copied unchanged");
        let names = |dir: &Path| -> Vec<String> {
            let mut names: Vec<String> = std::fs::read_dir(dir)
                .unwrap()
                .map(|item| item.unwrap().file_name().to_string_lossy().into_owned())
                .collect();
            names.sort();
            names
        };
        assert_eq!(names(&out), ["corejson", "lz4"]);
        assert_eq!(names(&out.join("corejson")), ["default"]);
        assert_eq!(names(&out.join("corejson/default")), ["JSON_SearchT"]);
        assert_eq!(names(&out.join("lz4/default")), ["LZ4F_getFrameInfo"]);

        // It fails closed: a drivable target without a corpus is an error and the
        // output directory is not created; a non-empty output directory is refused.
        let incomplete = tmp.path().join("incomplete");
        let missing = run(&incomplete, &[]);
        assert_eq!(missing.code, Some(2), "stdout: {}", missing.stdout);
        assert!(missing.stderr.contains("cjson"), "{}", missing.stderr);
        assert!(
            !incomplete.exists(),
            "nothing is written when the selection fails"
        );
        let again = run(&out, &["corejson", "lz4"]);
        assert_eq!(again.code, Some(2), "stdout: {}", again.stdout);
    }

    #[test]
    fn eqv_04_sample_cli_needs_its_options() {
        let outcome = run_mt(&["vectors", "sample", "--manifest", "testbed/manifest.toml"]);
        assert_eq!(outcome.code, Some(2), "stderr: {}", outcome.stderr);
        assert!(
            outcome.stderr.contains("--corpus-dir"),
            "{}",
            outcome.stderr
        );
    }
}
