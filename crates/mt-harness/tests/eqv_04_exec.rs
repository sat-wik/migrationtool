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
use mt_harness::replay::{ReplayOptions, check_corpora, replay_corpus};
use mt_harness::run::{
    BUILD_JSON, BuildOptions, BuildRecord, BuildState, EntryRunSummary, RunOptions, RunState,
    Scope, build_entries, layout_lines, run_entries,
};
use mt_harness::tools::ExecTools;
use mt_toolchain::runner::RunnerConfig;
use mt_vectors::bytes::decode_latin1;
use mt_vectors::corpus::{HasUbSource, read_corpus};
use mt_vectors::sha256_hex;
use mt_vectors::state::{Pointer, StateValue};
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

fn scope_for(target: &str, config: &str, entry: &str, build_root: &Path, work: &Path) -> Scope {
    let mut scope = Scope::new(
        repo_root().join("testbed/manifest.toml"),
        work.to_path_buf(),
    );
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
    let entry_a = corpus_a.join("corejson/default/JSON_Validate");
    let corpus = read_corpus(&entry_a).unwrap();
    assert_eq!(corpus.index.target, "corejson");
    assert_eq!(corpus.index.config, "default");
    assert_eq!(corpus.index.entry, "JSON_Validate");
    assert_eq!(corpus.vectors.len(), first.vectors);
    for (sha, vector) in &corpus.vectors {
        // The stored file parses at the reference level and is its own hash.
        let bytes = std::fs::read(entry_a.join(format!("{sha}.json"))).unwrap();
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
        assert_eq!(
            vector.stdout.pattern, "",
            "no output buffer: stdout is empty"
        );

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
    println!("corejson/default/JSON_Validate {}", first.render());
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

/// Build, then run into `corpus`, and return the entry's corpus directory.
fn build_and_run(
    target: &str,
    config: &str,
    entry: &str,
    tmp: &Path,
    corpus: &Path,
) -> (Scope, EntryRunSummary, PathBuf) {
    let scope = scope_for(target, config, entry, &tmp.join("build"), &local_work_dir());
    build_ok(&scope);
    let summary = run_into(&scope, corpus, &tmp.join("slots"));
    let dir = corpus.join(target).join(config).join(entry);
    (scope, summary, dir)
}

#[test]
#[ignore = "reads the captures of an earlier local run; set MT_LOCAL_CAPTURES"]
fn eqv_04_sentinel_prefill_recorded() {
    let tmp = tempfile::tempdir().unwrap();
    let (_, summary, dir) = build_and_run(
        "lz4",
        "default",
        "LZ4_decompress_safe",
        tmp.path(),
        &tmp.path().join("corpus"),
    );
    assert!(summary.vectors > 0, "{summary:?}");
    let corpus = read_corpus(&dir).unwrap();
    // seed bytes -> capacity -> (rc, recorded buffer)
    let mut by_seed: std::collections::BTreeMap<
        String,
        std::collections::BTreeMap<u32, (i32, Vec<u8>)>,
    > = std::collections::BTreeMap::new();
    for vector in corpus.vectors.values() {
        assert_eq!(
            vector.argv.len(),
            3,
            "argv0, the input length, the capacity"
        );
        let cap: u32 = vector.argv[2].parse().unwrap();
        let recorded = decode_latin1(&vector.stdout.pattern).unwrap();
        assert_eq!(
            recorded.len(),
            cap as usize,
            "D-07: the whole output buffer is recorded, capacity {cap}"
        );
        by_seed
            .entry(vector.stdin.clone())
            .or_default()
            .insert(cap, (vector.rc, recorded));
    }
    // LZ4's decoder copies in wide steps, so it can write a few bytes past the
    // length it returns. The contract records the whole buffer exactly because
    // of this; the test pins how far it goes (measured: 3) with a margin.
    const OVERSHOOT_MARGIN: usize = 16;
    let mut successes = 0usize;
    let mut cut_short = 0usize;
    let mut partial_writes = 0usize;
    let mut overshoot = 0usize;
    for (seed, caps) in &by_seed {
        assert_eq!(
            caps.keys().copied().collect::<Vec<_>>(),
            [16, 256, 4096],
            "every seed runs once per capacity (seed {seed:?})"
        );
        // An error path may have written part of the output before failing.
        for cap in [16u32, 256, 4096] {
            let (rc, recorded) = &caps[&cap];
            if *rc < 0 && recorded.iter().any(|byte| *byte != 0xA5) {
                partial_writes += 1;
            }
        }
        let (rc_big, big) = &caps[&4096];
        if *rc_big < 0 {
            continue;
        }
        let produced = usize::try_from(*rc_big).unwrap();
        successes += 1;
        let last = big
            .iter()
            .rposition(|byte| *byte != 0xA5)
            .map_or(0, |at| at + 1);
        overshoot = overshoot.max(last.saturating_sub(produced));
        // With room to spare, everything beyond the decoder's overshoot is
        // untouched sentinel.
        assert!(
            big[produced + OVERSHOOT_MARGIN..]
                .iter()
                .all(|byte| *byte == 0xA5),
            "bytes after {produced} produced bytes and the margin are 0xA5"
        );
        for cap in [16u32, 256] {
            let (rc, recorded) = &caps[&cap];
            if produced <= cap as usize {
                assert_eq!(*rc, *rc_big, "capacity {cap}");
                assert_eq!(&recorded[..produced], &big[..produced]);
            } else {
                // The block does not fit: the error path, whatever it wrote
                // before failing is part of the vector.
                assert!(*rc < 0, "capacity {cap} cannot hold {produced} bytes");
                cut_short += 1;
            }
        }
    }
    println!(
        "lz4/default/LZ4_decompress_safe sentinel: seeds={} produced_ok={successes} cut_short={cut_short} partial_writes={partial_writes} max_overshoot={overshoot}",
        by_seed.len()
    );
    assert!(successes > 0, "at least one seed decompresses");
    assert!(
        cut_short > 0,
        "the small capacity cuts at least one block short"
    );
    assert!(
        partial_writes > 0,
        "an error path leaves a partial write in the recorded buffer (D-07)"
    );
}

#[test]
#[ignore = "reads the captures of an earlier local run; set MT_LOCAL_CAPTURES"]
fn eqv_04_flat_targets_produce_vectors() {
    let work = local_work_dir();
    let cases = [
        ("corejson", ["default", "depth-8"], "JSON_Validate"),
        ("lz4", ["default", "heap-small"], "LZ4_decompress_safe"),
        ("libfdt", ["default", "assume-valid"], "fdt_check_full"),
    ];
    for (target, configs, entry) in cases {
        for config in configs {
            let tmp = tempfile::tempdir().unwrap();
            let scope = scope_for(target, config, entry, &tmp.path().join("build"), &work);
            build_ok(&scope);
            let first = run_into(&scope, &tmp.path().join("a"), &tmp.path().join("slots-a"));
            let second = run_into(&scope, &tmp.path().join("b"), &tmp.path().join("slots-b"));
            assert_eq!(first, second, "{target}/{config}/{entry}");
            assert!(first.vectors > 0 && first.twice_identical, "{first:?}");
            let comparison = check_corpora(&tmp.path().join("a"), &tmp.path().join("b")).unwrap();
            assert!(comparison.identical(), "{:?}", comparison.problems);
            println!(
                "{target}/{config}/{entry} vectors={} identical",
                first.vectors
            );
        }
    }
}

#[test]
#[ignore = "reads the captures of an earlier local run; set MT_LOCAL_CAPTURES"]
fn eqv_04_replay_reproduces_stored_vectors() {
    let tmp = tempfile::tempdir().unwrap();
    let (scope, summary, dir) = build_and_run(
        "corejson",
        "default",
        "JSON_Validate",
        tmp.path(),
        &tmp.path().join("corpus"),
    );
    let options = ReplayOptions {
        scope,
        tools: host_tools(),
        cfg: runner_config(),
        scratch_dir: tmp.path().join("replay-slots"),
    };
    let report = replay_corpus(&options, &dir).unwrap();
    assert_eq!(report.vectors, summary.vectors);
    assert_eq!(report.identical, summary.vectors);
    assert!(report.passed(), "{}", report.render());
    assert_eq!(
        report.render(),
        format!("{} identical {}", dir.display(), summary.vectors)
    );

    // A stored vector edited by one byte is reported as differing.
    let victim = std::fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            path.extension().is_some_and(|ext| ext == "json")
                && path.file_name().is_some_and(|name| name != "index.json")
        })
        .unwrap();
    let sha = victim.file_stem().unwrap().to_str().unwrap().to_owned();
    let mut bytes = std::fs::read(&victim).unwrap();
    let text = String::from_utf8(bytes.clone()).unwrap();
    let at = text.find("\"rc\": ").unwrap() + "\"rc\": ".len();
    bytes[at] = if bytes[at] == b'7' { b'8' } else { b'7' };
    std::fs::write(&victim, &bytes).unwrap();
    let edited = replay_corpus(&options, &dir).unwrap();
    assert!(!edited.passed());
    assert_eq!(edited.differing, std::slice::from_ref(&sha));
    assert_eq!(edited.identical, summary.vectors - 1);
    assert_eq!(edited.render(), format!("{} differs {sha}", dir.display()));

    // A provenance record that names another driver is refused, even though
    // the vectors still reproduce.
    std::fs::write(&victim, text.as_bytes()).unwrap();
    let index_path = dir.join("index.json");
    let index = std::fs::read_to_string(&index_path).unwrap();
    let marker = "\"driver_elf\": \"";
    let at = index.find(marker).unwrap() + marker.len();
    let mut changed = index.into_bytes();
    changed[at] = if changed[at] == b'0' { b'1' } else { b'0' };
    std::fs::write(&index_path, &changed).unwrap();
    let mismatched = replay_corpus(&options, &dir).unwrap();
    assert!(!mismatched.passed());
    assert!(!mismatched.provenance_mismatch.is_empty());
    assert!(
        mismatched.render().contains("provenance_mismatch"),
        "{}",
        mismatched.render()
    );
}

/// True when every byte and pointer of `value` is zero or null.
fn is_all_zero(value: &StateValue) -> bool {
    match value {
        StateValue::Int { value, .. } => *value == 0,
        StateValue::Wide { value, .. } => value == "0",
        StateValue::Float { bits, .. } => bits.bytes().all(|b| b == b'0'),
        StateValue::Bytes { hex, .. } => hex.bytes().all(|b| b == b'0'),
        StateValue::Items { items, .. } => items.iter().all(is_all_zero),
        StateValue::Struct { fields, .. } => fields.values().all(is_all_zero),
        StateValue::Pointer { pointer, .. } => *pointer == Pointer::Null,
        StateValue::Unobserved { .. } => false,
    }
}

/// Every pointer in `value`, in field order.
fn pointers_of<'a>(value: &'a StateValue, found: &mut Vec<&'a Pointer>) {
    match value {
        StateValue::Pointer { pointer, .. } => found.push(pointer),
        StateValue::Items { items, .. } => items.iter().for_each(|item| pointers_of(item, found)),
        StateValue::Struct { fields, .. } => {
            fields.values().for_each(|field| pointers_of(field, found));
        }
        _ => {}
    }
}

fn int_field(fields: &std::collections::BTreeMap<String, StateValue>, name: &str) -> i64 {
    match fields.get(name) {
        Some(StateValue::Int { value, .. }) => *value,
        other => panic!("field {name} must be an integer, found {other:?}"),
    }
}

#[test]
#[ignore = "reads the captures of an earlier local run; set MT_LOCAL_CAPTURES"]
fn eqv_04_state_is_recorded_field_by_field() {
    let work = local_work_dir();
    let tmp = tempfile::tempdir().unwrap();
    let scope = scope_for(
        "microcoap",
        "default",
        "coap_parse",
        &tmp.path().join("build"),
        &work,
    );
    build_ok(&scope);
    let first = run_into(&scope, &tmp.path().join("a"), &tmp.path().join("slots-a"));
    let second = run_into(&scope, &tmp.path().join("b"), &tmp.path().join("slots-b"));
    assert_eq!(first, second, "two full runs give the same summary");
    assert!(first.vectors > 0 && first.twice_identical, "{first:?}");
    let comparison = check_corpora(&tmp.path().join("a"), &tmp.path().join("b")).unwrap();
    assert!(comparison.identical(), "{:?}", comparison.problems);

    let dir = tmp.path().join("a/microcoap/default/coap_parse");
    let corpus = read_corpus(&dir).unwrap();
    let mut parsed = 0usize;
    let mut with_pointers = 0usize;
    for (sha, vector) in &corpus.vectors {
        let input_len = decode_latin1(&vector.stdin).unwrap().len() as u64;
        let before = vector.lib_state_in.get("pkt").expect("lib_state_in.pkt");
        assert!(
            is_all_zero(before),
            "the state starts zero-filled: {before:?}"
        );
        let after = vector.lib_state_out.get("pkt").expect("lib_state_out.pkt");

        // No raw address: every pointer is null or a byte offset into `in`.
        let mut pointers = Vec::new();
        pointers_of(after, &mut pointers);
        for pointer in &pointers {
            match pointer {
                Pointer::Null => {}
                Pointer::Buffer { region, offset } => {
                    assert_eq!(region, "in", "{sha}");
                    assert!(
                        *offset <= input_len,
                        "{sha}: offset {offset} of {input_len}"
                    );
                }
                Pointer::Symbol(_) => panic!("{sha}: coap_packet_t holds no function pointer"),
            }
        }
        assert_eq!(
            corpus.index.vectors[sha].unobserved,
            u64::try_from(
                vector.lib_state_in.unobserved_count() + vector.lib_state_out.unobserved_count()
            )
            .unwrap(),
            "the provenance counts the marks"
        );

        if vector.rc != 0 {
            continue;
        }
        parsed += 1;
        let StateValue::Struct { c_type, fields } = after else {
            panic!("pkt must be a struct: {after:?}");
        };
        assert_eq!(c_type, "coap_packet_t");
        let names: Vec<&str> = fields.keys().map(String::as_str).collect();
        assert_eq!(names, ["hdr", "numopts", "opts", "payload", "tok"]);
        let StateValue::Struct { fields: hdr, .. } = &fields["hdr"] else {
            panic!("hdr must be a struct");
        };
        assert_eq!(
            int_field(hdr, "ver"),
            1,
            "{sha}: a parsed packet is version 1"
        );
        for name in ["t", "tkl", "code"] {
            let _ = int_field(hdr, name);
        }
        assert!(matches!(&hdr["id"], StateValue::Bytes { hex, .. } if hex.len() == 4));
        let StateValue::Items { items, .. } = &fields["opts"] else {
            panic!("opts must be an array of structs");
        };
        assert!(!items.is_empty());
        let numopts = int_field(fields, "numopts");
        let mut with_buffer = 0i64;
        for (index, option) in items.iter().enumerate() {
            let StateValue::Struct { fields: option, .. } = option else {
                panic!("opts[{index}] must be a struct");
            };
            let StateValue::Struct { fields: buf, .. } = &option["buf"] else {
                panic!("opts[{index}].buf must be a struct");
            };
            if let StateValue::Pointer {
                pointer: Pointer::Buffer { .. },
                ..
            } = &buf["p"]
            {
                with_buffer += 1;
            }
        }
        assert!(with_buffer >= numopts.min(i64::try_from(items.len()).unwrap()) || numopts == 0);
        let StateValue::Struct {
            fields: payload, ..
        } = &fields["payload"]
        else {
            panic!("payload must be a struct");
        };
        if int_field(payload, "len") > 0 {
            with_pointers += 1;
            assert!(
                matches!(&payload["p"], StateValue::Pointer { pointer: Pointer::Buffer { region, .. }, .. } if region == "in"),
                "{sha}: a payload points into the input"
            );
        }
    }
    assert!(parsed > 0, "at least one seed parses");
    assert!(with_pointers > 0, "at least one seed carries a payload");
    println!("microcoap/default/coap_parse {}", first.render());
    println!(
        "microcoap/default/coap_parse vectors={} identical",
        first.vectors
    );
}

#[test]
#[ignore = "reads the captures of an earlier local run; set MT_LOCAL_CAPTURES"]
fn eqv_04_union_state_is_recorded() {
    let work = local_work_dir();
    for config in ["default", "peripheral"] {
        let tmp = tempfile::tempdir().unwrap();
        let scope = scope_for(
            "nimble",
            config,
            "ble_uuid_init_from_buf",
            &tmp.path().join("build"),
            &work,
        );
        build_ok(&scope);
        let first = run_into(&scope, &tmp.path().join("a"), &tmp.path().join("slots-a"));
        let second = run_into(&scope, &tmp.path().join("b"), &tmp.path().join("slots-b"));
        assert_eq!(first, second, "nimble/{config}");
        assert!(first.vectors > 0 && first.twice_identical, "{first:?}");
        let comparison = check_corpora(&tmp.path().join("a"), &tmp.path().join("b")).unwrap();
        assert!(comparison.identical(), "{:?}", comparison.problems);

        let dir = tmp
            .path()
            .join(format!("a/nimble/{config}/ble_uuid_init_from_buf"));
        let corpus = read_corpus(&dir).unwrap();
        let mut parsed = 0usize;
        for (sha, vector) in &corpus.vectors {
            let after = vector
                .lib_state_out
                .get("uuid")
                .expect("lib_state_out.uuid");
            let before = vector.lib_state_in.get("uuid").expect("lib_state_in.uuid");
            assert!(is_all_zero(before), "{sha}: the union starts zero-filled");
            let StateValue::Struct { c_type, fields } = after else {
                panic!("{sha}: a union is recorded as a struct-shaped value: {after:?}");
            };
            assert_eq!(c_type, "ble_uuid_any_t");
            let names: Vec<&str> = fields.keys().map(String::as_str).collect();
            assert_eq!(
                names,
                ["u", "u128", "u16", "u32"],
                "every member of the union is recorded, decoded from the same bytes"
            );
            let input = decode_latin1(&vector.stdin).unwrap();
            if vector.rc == 0 && input.len() == 2 {
                parsed += 1;
                let StateValue::Struct {
                    fields: u16_fields, ..
                } = &fields["u16"]
                else {
                    panic!("u16 must be a struct");
                };
                let want = i64::from(input[0]) | (i64::from(input[1]) << 8);
                assert_eq!(
                    int_field(u16_fields, "value"),
                    want,
                    "{sha}: the 16-bit UUID"
                );
            }
        }
        assert!(
            parsed > 0,
            "nimble/{config}: at least one 16-bit UUID parses"
        );
        println!(
            "nimble/{config}/ble_uuid_init_from_buf vectors={} identical",
            first.vectors
        );
    }
}

#[test]
#[ignore = "reads the captures of an earlier local run; set MT_LOCAL_CAPTURES"]
fn eqv_04_globals_are_typed_at_build_time() {
    let work = local_work_dir();
    let tmp = tempfile::tempdir().unwrap();
    let scope = scope_for(
        "microcoap",
        "default",
        "coap_parse",
        &tmp.path().join("build"),
        &work,
    );
    build_ok(&scope);
    let path = tmp
        .path()
        .join("build/microcoap/default/coap_parse")
        .join(BUILD_JSON);
    let record: BuildRecord = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let keys: Vec<&str> = record.globals.iter().map(|g| g.key.as_str()).collect();
    assert_eq!(keys, ["src/endpoints.o:light", "src/endpoints.o:rsp"]);
    // Every writable global of the pair has a type or a reason it has none.
    let typed: Vec<&str> = record.global_types.keys().map(String::as_str).collect();
    assert_eq!(typed, keys);
    for (key, global_type) in &record.global_types {
        assert!(
            matches!(global_type, mt_harness::probe::GlobalType::Typed { .. }),
            "{key} must be typed from its unit's DWARF: {global_type:?}"
        );
    }
    println!(
        "microcoap/default/coap_parse globals typed: {}",
        keys.join(" ")
    );
}

#[test]
#[ignore = "reads the captures of an earlier local run; set MT_LOCAL_CAPTURES"]
fn eqv_04_print_local_layouts() {
    let work = local_work_dir();
    let tmp = tempfile::tempdir().unwrap();
    let build_root = tmp.path().join("build");
    for (target, configs, entry) in [
        ("microcoap", ["default", "release"], "coap_parse"),
        (
            "nimble",
            ["default", "peripheral"],
            "ble_uuid_init_from_buf",
        ),
    ] {
        for config in configs {
            let scope = scope_for(target, config, entry, &build_root, &work);
            build_ok(&scope);
        }
    }
    let lines = layout_lines(&build_root).unwrap();
    assert!(
        lines.len() >= 4,
        "one line per built entry and state: {lines:?}"
    );
    for line in &lines {
        // The same text `mt vectors layouts` prints, for the CI cross-check.
        println!("layout {line}");
    }
}

/// The value of a recorded integer.
fn recorded_int(state: &mt_vectors::state::LibState, name: &str) -> i64 {
    match state.get(name) {
        Some(StateValue::Int { value, .. }) => *value,
        other => panic!("{name} must be a recorded integer, found {other:?}"),
    }
}

#[test]
#[ignore = "reads the captures of an earlier local run; set MT_LOCAL_CAPTURES"]
fn eqv_04_inout_len_is_recorded_in_and_out() {
    // D-05, D-06, D-07, D-11: zlib's uncompress takes the capacity of its output
    // buffer through a pointer to a length and reports the produced length back
    // through it.
    let work = local_work_dir();
    for config in ["default", "no-gzip"] {
        let tmp = tempfile::tempdir().unwrap();
        let scope = scope_for(
            "zlib",
            config,
            "uncompress",
            &tmp.path().join("build"),
            &work,
        );
        build_ok(&scope);
        let first = run_into(&scope, &tmp.path().join("a"), &tmp.path().join("slots-a"));
        let second = run_into(&scope, &tmp.path().join("b"), &tmp.path().join("slots-b"));
        assert_eq!(first, second, "zlib/{config}: two full runs agree");
        assert!(first.vectors > 0 && first.twice_identical, "{first:?}");
        let comparison = check_corpora(&tmp.path().join("a"), &tmp.path().join("b")).unwrap();
        assert!(comparison.identical(), "{:?}", comparison.problems);

        let dir = tmp.path().join(format!("a/zlib/{config}/uncompress"));
        let corpus = read_corpus(&dir).unwrap();
        let (mut ok, mut buf_error) = (0usize, 0usize);
        let mut capacities = std::collections::BTreeSet::new();
        for (sha, vector) in &corpus.vectors {
            let input_len = decode_latin1(&vector.stdin).unwrap().len();
            assert_eq!(vector.argv.len(), 3, "{sha}: {:?}", vector.argv);
            assert_eq!(vector.argv[0], format!("zlib/{config}/uncompress"));
            let capacity: i64 = vector.argv[1].parse().unwrap();
            assert_eq!(
                vector.argv[2],
                input_len.to_string(),
                "{sha}: argv holds the initial destLen, then sourceLen, in parameter order"
            );
            capacities.insert(capacity);
            assert_eq!(
                recorded_int(&vector.lib_state_in, "destLen"),
                capacity,
                "{sha}: lib_state_in holds the declared initial value"
            );
            let produced = recorded_int(&vector.lib_state_out, "destLen");
            let output = decode_latin1(&vector.stdout.pattern).unwrap();
            assert_eq!(
                output.len() as i64,
                capacity,
                "{sha}: stdout is the whole output buffer, whose size is the initial destLen"
            );
            match vector.rc {
                0 => {
                    ok += 1;
                    let produced = usize::try_from(produced).unwrap();
                    assert!(produced <= output.len(), "{sha}");
                    assert!(
                        output[produced..].iter().all(|byte| *byte == 0xA5),
                        "{sha}: the sentinel tail follows the produced length"
                    );
                }
                -5 => {
                    buf_error += 1;
                    assert!(capacity < 4096, "{sha}: only a small buffer overflows");
                    assert!(
                        output.iter().any(|byte| *byte != 0xA5),
                        "{sha}: the partial write is recorded"
                    );
                }
                _ => {}
            }
            assert_eq!(corpus.index.vectors[sha].scalars["destLen"], capacity);
        }
        assert_eq!(
            capacities,
            [16, 256, 4096].into_iter().collect(),
            "the declared capacities are swept"
        );
        assert!(ok > 0, "at least one stream decompresses");
        assert!(
            buf_error > 0,
            "a capacity of 16 overflows on a longer output"
        );
        println!("zlib/{config}/uncompress {}", first.render());
        println!(
            "zlib/{config}/uncompress vectors={} identical",
            first.vectors
        );
    }
}
