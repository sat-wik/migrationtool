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
    // pb_decode_ex is not driven yet; JSON_SearchT has roles since plan 03-06.
    let scope = scope_for(
        "nanopb",
        "default",
        "pb_decode_ex",
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
    assert_eq!(rows[0].render(), "nanopb/default/pb_decode_ex no_roles");
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

/// Build, run twice and compare two full runs of one pair and entry; returns the
/// summary and the directory of the first corpus.
fn run_pair_twice(
    target: &str,
    config: &str,
    entry: &str,
    tmp: &Path,
) -> (EntryRunSummary, PathBuf) {
    let scope = scope_for(target, config, entry, &tmp.join("build"), &local_work_dir());
    build_ok(&scope);
    let first = run_into(&scope, &tmp.join("a"), &tmp.join("slots-a"));
    let second = run_into(&scope, &tmp.join("b"), &tmp.join("slots-b"));
    assert_eq!(
        first, second,
        "{target}/{config}/{entry}: two full runs agree"
    );
    assert!(first.vectors > 0 && first.twice_identical, "{first:?}");
    let comparison = check_corpora(&tmp.join("a"), &tmp.join("b")).unwrap();
    assert!(comparison.identical(), "{:?}", comparison.problems);
    println!("{target}/{config}/{entry} {}", first.render());
    println!(
        "{target}/{config}/{entry} vectors={} identical",
        first.vectors
    );
    (first, tmp.join(format!("a/{target}/{config}/{entry}")))
}

/// Every pointer of `value` is null or a byte offset into the region `region`.
fn assert_pointers_only_into(value: &StateValue, region: &str, what: &str) {
    let mut found = Vec::new();
    pointers_of(value, &mut found);
    for pointer in found {
        match pointer {
            Pointer::Null => {}
            Pointer::Buffer { region: found, .. } => {
                assert_eq!(found, region, "{what}: {value:?}");
            }
            Pointer::Symbol(name) => panic!("{what}: unexpected function {name}"),
        }
    }
}

#[test]
#[ignore = "reads the captures of an earlier local run; set MT_LOCAL_CAPTURES"]
fn eqv_04_out_scalars_and_pointers_are_recorded() {
    // D-06, D-11: paho's deserializers return their results through pointers to
    // scalars, a pointer to a pointer and a string struct.
    let tmp = tempfile::tempdir().unwrap();
    let (_, dir) = run_pair_twice(
        "paho",
        "default",
        "MQTTDeserialize_connack",
        &tmp.path().join("connack"),
    );
    let corpus = read_corpus(&dir).unwrap();
    let mut parsed = 0usize;
    for (sha, vector) in &corpus.vectors {
        let input_len = decode_latin1(&vector.stdin).unwrap().len();
        assert_eq!(
            vector.argv,
            [
                "paho/default/MQTTDeserialize_connack".to_owned(),
                input_len.to_string()
            ],
            "{sha}: argv holds the input length only"
        );
        for name in ["sessionPresent", "connack_rc"] {
            assert!(
                is_all_zero(vector.lib_state_in.get(name).expect("lib_state_in")),
                "{sha}: {name} starts zero"
            );
            assert!(
                matches!(vector.lib_state_out.get(name), Some(StateValue::Int { c_type, .. }) if c_type == "unsigned char"),
                "{sha}: {name} is recorded after the call: {:?}",
                vector.lib_state_out.get(name)
            );
        }
        if vector.rc == 1 {
            parsed += 1;
        }
    }
    assert!(parsed > 0, "at least one seed is a connack packet");

    let (_, dir) = run_pair_twice(
        "paho",
        "default",
        "MQTTDeserialize_publish",
        &tmp.path().join("publish"),
    );
    let corpus = read_corpus(&dir).unwrap();
    let (mut parsed, mut with_payload) = (0usize, 0usize);
    let (mut faulted, mut beyond_the_input) = (0usize, 0usize);
    for (sha, vector) in &corpus.vectors {
        let input_len = decode_latin1(&vector.stdin).unwrap().len() as u64;
        if vector.has_ub.is_some() {
            // A run that ends in a signal has no result frame, so no state.
            faulted += 1;
            assert!(vector.lib_state_out.is_empty(), "{sha}");
            continue;
        }
        for name in ["dup", "qos", "retained", "packetid", "payloadlen"] {
            assert!(
                matches!(vector.lib_state_out.get(name), Some(StateValue::Int { .. })),
                "{sha}: {name}: {:?}",
                vector.lib_state_out.get(name)
            );
        }
        assert!(
            matches!(vector.lib_state_out.get("packetid"), Some(StateValue::Int { c_type, .. }) if c_type == "short unsigned int")
        );
        let topic = vector.lib_state_out.get("topicName").expect("topicName");
        assert!(
            matches!(topic, StateValue::Struct { c_type, .. } if c_type == "MQTTString"),
            "{sha}: {topic:?}"
        );
        assert_pointers_only_into(topic, "in", sha);
        let payload = vector.lib_state_out.get("payload").expect("payload");
        if let StateValue::Unobserved { c_type, reason } = payload {
            // A packet whose declared length runs past the input leaves payload
            // pointing beyond the end of the input (the function reports success
            // without checking it against buflen): no region holds the pointer, so
            // it is recorded as an unobserved mark with the fixed reason (D-06, D-09).
            assert_eq!(c_type, "unsigned char *", "{sha}");
            assert_eq!(
                reason,
                mt_harness::decode::UNRESOLVED_POINTER_REASON,
                "{sha}"
            );
            beyond_the_input += 1;
            continue;
        }
        assert!(
            matches!(payload, StateValue::Pointer { c_type, .. } if c_type == "unsigned char *"),
            "{sha}: {payload:?}"
        );
        assert_pointers_only_into(payload, "in", sha);
        if vector.rc == 1 {
            parsed += 1;
            if let StateValue::Pointer {
                pointer: Pointer::Buffer { offset, .. },
                ..
            } = payload
            {
                assert!(*offset <= input_len, "{sha}");
                with_payload += 1;
            }
            // A parsed publish names its topic by a pointer into the input.
            let mut pointers = Vec::new();
            pointers_of(topic, &mut pointers);
            assert!(
                pointers
                    .iter()
                    .any(|pointer| matches!(pointer, Pointer::Buffer { .. })),
                "{sha}: a parsed publish has a topic pointer"
            );
        }
    }
    println!(
        "paho/default/MQTTDeserialize_publish faulted={faulted} payload_beyond_the_input={beyond_the_input}"
    );
    assert!(parsed > 0, "at least one seed is a publish packet");
    assert!(
        with_payload > 0,
        "at least one parsed packet has a payload pointer"
    );
}

#[test]
#[ignore = "reads the captures of an earlier local run; set MT_LOCAL_CAPTURES"]
fn eqv_04_struct_return_is_recorded() {
    // D-06, D-11: nanopb's pb_istream_from_buffer returns a struct by value,
    // which the contract records as lib_state_out.ret and rc 0.
    for config in ["default", "buffer-only"] {
        let tmp = tempfile::tempdir().unwrap();
        let (_, dir) = run_pair_twice("nanopb", config, "pb_istream_from_buffer", tmp.path());
        let corpus = read_corpus(&dir).unwrap();
        let mut with_callback = 0usize;
        for (sha, vector) in &corpus.vectors {
            let input_len = decode_latin1(&vector.stdin).unwrap().len();
            assert_eq!(vector.rc, 0, "{sha}: a struct return gives rc 0");
            assert!(
                vector.lib_state_in.get("ret").is_none(),
                "{sha}: nothing is returned before the call"
            );
            let ret = vector.lib_state_out.get("ret").expect("lib_state_out.ret");
            let StateValue::Struct { c_type, fields } = ret else {
                panic!("{sha}: ret must be a struct: {ret:?}");
            };
            assert_eq!(c_type, "pb_istream_t");
            assert!(
                matches!(&fields["state"], StateValue::Pointer { pointer: Pointer::Buffer { region, offset: 0 }, .. } if region == "in"),
                "{sha}: state points at the start of the input: {:?}",
                fields["state"]
            );
            assert_eq!(
                int_field(fields, "bytes_left"),
                i64::try_from(input_len).unwrap(),
                "{sha}"
            );
            let callback = &fields["callback"];
            if config == "default" {
                assert!(
                    matches!(
                        callback,
                        StateValue::Pointer { c_type, pointer: Pointer::Symbol(_) } if c_type.contains("(*)")
                    ),
                    "{sha}: callback is a function: {callback:?}"
                );
                with_callback += 1;
            } else {
                // PB_BUFFER_ONLY replaces the function pointer by an unused int *.
                assert!(
                    matches!(
                        callback,
                        StateValue::Pointer { c_type, pointer: Pointer::Null } if c_type == "int *"
                    ),
                    "{sha}: callback is the unused int pointer: {callback:?}"
                );
            }
        }
        if config == "default" {
            assert_eq!(
                with_callback,
                corpus.vectors.len(),
                "the stream has a callback"
            );
        }
    }
}

#[test]
#[ignore = "reads the captures of an earlier local run; set MT_LOCAL_CAPTURES"]
fn eqv_04_heap_tree_is_recorded() {
    // D-06, D-09: cJSON_ParseWithLength returns a heap tree and records its
    // error position in a writable global.
    for config in ["default", "no-locale"] {
        let tmp = tempfile::tempdir().unwrap();
        let (_, dir) = run_pair_twice("cjson", config, "cJSON_ParseWithLength", tmp.path());
        let corpus = read_corpus(&dir).unwrap();
        let (mut parsed, mut failed, mut strings) = (0usize, 0usize, 0usize);
        for (sha, vector) in &corpus.vectors {
            let ret = vector.lib_state_out.get("ret").expect("lib_state_out.ret");
            let StateValue::Pointer { c_type, pointer } = ret else {
                panic!("{sha}: ret must be a pointer: {ret:?}");
            };
            assert_eq!(c_type, "cJSON *");
            match pointer {
                Pointer::Buffer { region, offset } => {
                    parsed += 1;
                    assert_eq!(vector.rc, 1, "{sha}");
                    assert_eq!(region, "heap", "{sha}");
                    let heap = vector.lib_state_out.get("heap").expect("heap");
                    let StateValue::Struct { fields, .. } = heap else {
                        panic!("{sha}: heap must be struct-shaped: {heap:?}");
                    };
                    let root = fields
                        .get(&offset.to_string())
                        .unwrap_or_else(|| panic!("{sha}: no object at {offset}: {fields:?}"));
                    assert!(
                        matches!(root, StateValue::Struct { c_type, .. } if c_type == "cJSON"),
                        "{sha}: {root:?}"
                    );
                    assert_pointers_only_into(heap, "heap", sha);
                    strings += fields
                        .values()
                        .filter(|object| matches!(object, StateValue::Bytes { .. }))
                        .count();
                }
                Pointer::Null => {
                    failed += 1;
                    assert_eq!(vector.rc, 0, "{sha}");
                    let globals = vector
                        .lib_state_out
                        .get("globals")
                        .expect("a failed parse changes global_error");
                    let StateValue::Struct { fields, .. } = globals else {
                        panic!("{sha}: globals must be struct-shaped");
                    };
                    let (key, error) = fields
                        .iter()
                        .find(|(key, _)| key.ends_with(":global_error"))
                        .expect("global_error");
                    let StateValue::Struct { fields: error, .. } = error else {
                        panic!("{sha}: {key} must be typed: {error:?}");
                    };
                    assert!(
                        matches!(&error["json"], StateValue::Pointer { pointer: Pointer::Buffer { region, .. }, .. } if region == "in"),
                        "{sha}: the error points into the input: {:?}",
                        error["json"]
                    );
                    let before = vector
                        .lib_state_in
                        .get("globals")
                        .expect("the global is recorded before the call too");
                    assert!(
                        matches!(before, StateValue::Struct { fields, .. } if fields.keys().any(|key| key.ends_with(":global_error")) && is_all_zero(before)),
                        "{sha}: the error starts empty: {before:?}"
                    );
                    assert!(vector.lib_state_out.get("heap").is_none(), "{sha}");
                }
                Pointer::Symbol(name) => panic!("{sha}: ret is the function {name}"),
            }
        }
        assert!(parsed > 0, "at least one seed parses");
        assert!(strings > 0, "at least one tree holds a string");
        println!(
            "cjson/{config}/cJSON_ParseWithLength parsed={parsed} failed={failed} strings={strings}"
        );
        assert!(failed > 0, "at least one seed fails to parse");
    }
}

#[test]
#[ignore = "reads the captures of an earlier local run; set MT_LOCAL_CAPTURES"]
fn eqv_04_string_argument_and_mutable_input() {
    // D-05, D-11: JSON_SearchT takes a query string whose declared values are
    // data in the request, and a mutable input whose bytes after the call are
    // recorded.
    for config in ["default", "depth-8"] {
        let tmp = tempfile::tempdir().unwrap();
        let (_, dir) = run_pair_twice("corejson", config, "JSON_SearchT", tmp.path());
        let corpus = read_corpus(&dir).unwrap();
        let mut queries = std::collections::BTreeMap::<String, (usize, usize)>::new();
        for (sha, vector) in &corpus.vectors {
            let input = decode_latin1(&vector.stdin).unwrap();
            assert_eq!(vector.argv.len(), 4, "{sha}: {:?}", vector.argv);
            assert_eq!(vector.argv[0], format!("corejson/{config}/JSON_SearchT"));
            assert_eq!(vector.argv[1], input.len().to_string(), "{sha}");
            let query = vector.argv[2].clone();
            assert!(
                ["foo", "hello", "[0]"].contains(&query.as_str()),
                "{sha}: {query}"
            );
            assert_eq!(vector.argv[3], query.len().to_string(), "{sha}");
            let after = vector.lib_state_out.get("in").expect("lib_state_out.in");
            assert!(
                matches!(after, StateValue::Bytes { hex, .. } if *hex == input.iter().map(|b| format!("{b:02x}")).collect::<String>()),
                "{sha}: the input after the call: {after:?}"
            );
            assert!(vector.lib_state_in.get("in").is_none(), "{sha}");
            for name in ["outValueLength", "outType"] {
                assert!(
                    matches!(vector.lib_state_out.get(name), Some(StateValue::Int { .. })),
                    "{sha}: {name}"
                );
            }
            let out_value = vector.lib_state_out.get("outValue").expect("outValue");
            assert_pointers_only_into(out_value, "in", sha);
            let entry = queries.entry(query).or_default();
            entry.0 += 1;
            if matches!(
                out_value,
                StateValue::Pointer {
                    pointer: Pointer::Buffer { .. },
                    ..
                }
            ) {
                entry.1 += 1;
            }
        }
        for (query, (runs, hits)) in &queries {
            println!("corejson/{config}/JSON_SearchT query={query} runs={runs} hits={hits}");
        }
        assert_eq!(queries.len(), 3, "every declared query is run");
        assert!(
            queries.values().any(|(_, hits)| *hits > 0),
            "at least one query finds a value"
        );
    }
}

/// The struct members of a recorded state value.
fn struct_fields<'a>(
    value: &'a StateValue,
    what: &str,
) -> &'a std::collections::BTreeMap<String, StateValue> {
    match value {
        StateValue::Struct { fields, .. } => fields,
        other => panic!("{what} must be a recorded struct, found {other:?}"),
    }
}

#[test]
#[ignore = "reads the captures of an earlier local run; set MT_LOCAL_CAPTURES"]
fn eqv_04_setup_sequence_runs_before_the_entry() {
    // D-12: httpparser's http_parser_execute needs a parser that http_parser_init
    // has filled in. The declared setup step runs before the entry; the state after
    // it is lib_state_in and the state after the entry is lib_state_out.
    for config in ["default", "strict"] {
        let tmp = tempfile::tempdir().unwrap();
        let (summary, dir) =
            run_pair_twice("httpparser", config, "http_parser_execute", tmp.path());
        assert_eq!(summary.setup_failed, 0, "http_parser_init returns void");
        let corpus = read_corpus(&dir).unwrap();
        let mut parsed_some = false;
        for (sha, vector) in &corpus.vectors {
            let input_len = decode_latin1(&vector.stdin).unwrap().len();
            assert_eq!(
                vector.argv,
                [
                    format!("httpparser/{config}/http_parser_execute"),
                    input_len.to_string()
                ],
                "{sha}: argv holds the input length"
            );
            // lib_state_in is the state after http_parser_init: the type field of
            // HTTP_BOTH (2), the start state, and every other member zero.
            let before = struct_fields(
                vector
                    .lib_state_in
                    .get("parser")
                    .expect("lib_state_in.parser"),
                "lib_state_in.parser",
            );
            assert_eq!(int_field(before, "type"), 2, "{sha}: HTTP_BOTH");
            assert!(int_field(before, "state") > 0, "{sha}: a start state");
            for (name, value) in before {
                if !matches!(name.as_str(), "type" | "state") {
                    assert!(is_all_zero(value), "{sha}: {name} is zero after init");
                }
            }
            // The settings have no callbacks: all zero before and after.
            for state in [&vector.lib_state_in, &vector.lib_state_out] {
                let settings = state.get("settings").expect("settings is recorded");
                assert!(is_all_zero(settings), "{sha}: no callbacks are set");
            }
            // lib_state_out is the state after parsing, bitfields as integers.
            let after = struct_fields(
                vector
                    .lib_state_out
                    .get("parser")
                    .expect("lib_state_out.parser"),
                "lib_state_out.parser",
            );
            assert_eq!(
                after.keys().collect::<Vec<_>>(),
                before.keys().collect::<Vec<_>>()
            );
            // rc is the number of bytes parsed.
            let parsed = usize::try_from(vector.rc).expect("rc is the parsed byte count");
            assert!(parsed <= input_len, "{sha}: {parsed} of {input_len}");
            if parsed > 0 {
                parsed_some = true;
                assert_ne!(
                    vector.lib_state_in.get("parser"),
                    vector.lib_state_out.get("parser"),
                    "{sha}: parsing changes the parser"
                );
            }
        }
        assert!(parsed_some, "at least one message is parsed");
    }
}

#[test]
#[ignore = "reads the captures of an earlier local run; set MT_LOCAL_CAPTURES"]
fn eqv_04_setup_failure_is_a_compared_outcome() {
    // D-12, T-03-26: a setup call whose return differs from its expectation stops
    // the sequence. The vector records outcome setup_failed with the step in stderr
    // and the step's return value in rc, no label, and is compared like any other.
    // The manifest here is the committed one with a setup step added to corejson
    // JSON_Validate: it validates the input and expects JSONSuccess (0), so the
    // seeds that are not valid JSON (a JSONStatus_t above 0) end in setup_failed.
    use mt_capture::manifest::SetupStep;
    use mt_vectors::vector::OutcomeClass;

    let work = local_work_dir();
    let tmp = tempfile::tempdir().unwrap();
    let testbed = tmp.path().join("testbed");
    std::fs::create_dir_all(&testbed).unwrap();
    for name in ["seeds", "policy", "targets"] {
        std::os::unix::fs::symlink(repo_root().join("testbed").join(name), testbed.join(name))
            .unwrap();
    }
    let mut manifest = Manifest::load(&repo_root().join("testbed/manifest.toml")).unwrap();
    let entry = manifest
        .target
        .get_mut("corejson")
        .unwrap()
        .entry
        .iter_mut()
        .find(|entry| entry.name == "JSON_Validate")
        .unwrap();
    entry.setup = vec![SetupStep {
        call: Some("JSON_Validate".to_owned()),
        args: vec!["$in".to_owned(), "$len".to_owned()],
        expect: Some("0".to_owned()),
        set: None,
        value: None,
    }];
    manifest.validate().unwrap();
    std::fs::write(
        testbed.join("manifest.toml"),
        toml::to_string(&manifest).unwrap(),
    )
    .unwrap();

    let mut scope = Scope::new(testbed.join("manifest.toml"), work);
    scope.pairs = vec![("corejson".to_owned(), "default".to_owned())];
    scope.entries = Some(vec!["JSON_Validate".to_owned()]);
    scope.build_root = tmp.path().join("build");
    build_ok(&scope);
    let first = run_into(&scope, &tmp.path().join("a"), &tmp.path().join("slots-a"));
    let second = run_into(&scope, &tmp.path().join("b"), &tmp.path().join("slots-b"));
    assert_eq!(first, second, "two full runs agree");
    let comparison = check_corpora(&tmp.path().join("a"), &tmp.path().join("b")).unwrap();
    assert!(comparison.identical(), "{:?}", comparison.problems);
    assert!(first.setup_failed > 0 && first.ok > 0, "{first:?}");
    assert_eq!(first.ok + first.setup_failed, first.vectors, "{first:?}");

    let corpus = read_corpus(&tmp.path().join("a/corejson/default/JSON_Validate")).unwrap();
    let (mut failed, mut passed) = (0usize, 0usize);
    for (sha, vector) in &corpus.vectors {
        let provenance = &corpus.index.vectors[sha];
        match vector.outcome_class().unwrap() {
            OutcomeClass::SetupFailed => {
                failed += 1;
                assert_eq!(
                    vector.stderr.pattern, "outcome=setup_failed step=0",
                    "{sha}"
                );
                assert!(
                    vector.rc > 0,
                    "{sha}: the step's return value, a JSONStatus_t other than JSONSuccess"
                );
                assert!(
                    vector.has_ub.is_none(),
                    "{sha}: a setup failure has no label"
                );
                assert_eq!(provenance.outcome, "setup_failed");
                assert_eq!(provenance.has_ub_source, HasUbSource::Unchecked);
                assert_eq!(vector.lib_state_in, vector.lib_state_out, "{sha}");
            }
            OutcomeClass::Ok => {
                passed += 1;
                assert_eq!(vector.stderr.pattern, "outcome=ok", "{sha}");
                assert_eq!(vector.rc, 0, "{sha}");
            }
            other => panic!("{sha}: unexpected class {other:?}"),
        }
    }
    assert_eq!((failed, passed), (first.setup_failed, first.ok));
    println!(
        "corejson/default/JSON_Validate setup: {failed} setup_failed, {passed} ok, twice identical"
    );
}

#[test]
#[ignore = "reads the captures of an earlier local run; set MT_LOCAL_CAPTURES"]
fn eqv_04_wrapped_struct_argument() {
    // D-11, D-12: qcbor's QCBORDecode_Init takes the input as a `UsefulBufC` passed
    // by value. The `wrap` role builds it from the input pointer and length; the
    // decode mode is swept (normal, strings only, map as array).
    for config in ["default", "no-hw-float"] {
        let tmp = tempfile::tempdir().unwrap();
        let (summary, dir) = run_pair_twice("qcbor", config, "QCBORDecode_Init", tmp.path());
        assert_eq!(summary.setup_failed, 0, "no setup steps");
        let corpus = read_corpus(&dir).unwrap();
        let mut modes_of: std::collections::BTreeMap<String, Vec<i64>> =
            std::collections::BTreeMap::new();
        for (sha, vector) in &corpus.vectors {
            let input_len = decode_latin1(&vector.stdin).unwrap().len();
            assert_eq!(vector.argv.len(), 3, "{sha}: argv0, length, mode");
            assert_eq!(vector.argv[0], format!("qcbor/{config}/QCBORDecode_Init"));
            assert_eq!(vector.argv[1], input_len.to_string(), "{sha}");
            let mode: i64 = vector.argv[2].parse().unwrap();
            assert!([0, 1, 2].contains(&mode), "{sha}: mode {mode}");
            modes_of.entry(vector.stdin.clone()).or_default().push(mode);

            // lib_state_in.ctx is the zero-filled context; lib_state_out.ctx holds
            // every field the initialisation sets.
            let before = vector.lib_state_in.get("ctx").expect("lib_state_in.ctx");
            assert!(is_all_zero(before), "{sha}: the context starts zero-filled");
            let after = struct_fields(
                vector.lib_state_out.get("ctx").expect("lib_state_out.ctx"),
                "lib_state_out.ctx",
            );
            assert_eq!(
                struct_fields(before, "lib_state_in.ctx")
                    .keys()
                    .collect::<Vec<_>>(),
                after.keys().collect::<Vec<_>>(),
                "{sha}: the same members before and after"
            );
            assert_eq!(int_field(after, "uDecodeMode"), mode, "{sha}");
            // InBuf.UB is the wrapped UsefulBufC: pointer {in, 0} and the length.
            let in_buf = struct_fields(&after["InBuf"], "ctx.InBuf");
            let useful = struct_fields(&in_buf["UB"], "ctx.InBuf.UB");
            assert_eq!(int_field(useful, "len"), input_len as i64, "{sha}");
            match &useful["ptr"] {
                StateValue::Pointer { pointer, .. } if input_len > 0 => assert_eq!(
                    *pointer,
                    Pointer::Buffer {
                        region: "in".to_owned(),
                        offset: 0
                    },
                    "{sha}: the wrapped pointer is the input"
                ),
                StateValue::Pointer { .. } => {}
                other => panic!("{sha}: ctx.InBuf.UB.ptr must be a pointer, found {other:?}"),
            }
            // Every other pointer of the context is null or into the context itself.
            let mut found = Vec::new();
            pointers_of(&after["nesting"], &mut found);
            for pointer in found {
                match pointer {
                    Pointer::Null => {}
                    Pointer::Buffer { region, .. } => assert_eq!(region, "ctx", "{sha}"),
                    Pointer::Symbol(name) => panic!("{sha}: unexpected function {name}"),
                }
            }
        }
        assert!(!modes_of.is_empty(), "{config}: vectors");
        for (input, mut modes) in modes_of {
            modes.sort_unstable();
            assert_eq!(
                modes,
                [0, 1, 2],
                "{config}: input {input:?} runs every mode"
            );
        }
    }
}

#[test]
#[ignore = "reads the captures of an earlier local run; set MT_LOCAL_CAPTURES"]
fn eqv_04_state_array() {
    // D-12, T-03-27: zcbor's zcbor_new_decode_state fills an array of three states
    // in one guarded mapping; the elements are recorded as items, and the payload
    // pointers are offsets into the input. The out buffer's capacity is swept (0 and
    // 64 bytes).
    for config in ["default", "stop-on-error"] {
        let tmp = tempfile::tempdir().unwrap();
        let (summary, dir) = run_pair_twice("zcbor", config, "zcbor_new_decode_state", tmp.path());
        assert_eq!(summary.setup_failed, 0, "no setup steps");
        let corpus = read_corpus(&dir).unwrap();
        let mut capacities = std::collections::BTreeSet::new();
        for (sha, vector) in &corpus.vectors {
            let input_len = decode_latin1(&vector.stdin).unwrap().len();
            // argv: name, n_states, input length, elem_count, out capacity.
            assert_eq!(vector.argv.len(), 5, "{sha}: {:?}", vector.argv);
            assert_eq!(vector.argv[1], "3", "{sha}: n_states equals the count");
            assert_eq!(vector.argv[2], input_len.to_string(), "{sha}");
            assert_eq!(vector.argv[3], "1", "{sha}");
            capacities.insert(vector.argv[4].clone());

            let before = vector
                .lib_state_in
                .get("states")
                .expect("lib_state_in.states");
            assert!(is_all_zero(before), "{sha}: the array starts zero-filled");
            let after = vector
                .lib_state_out
                .get("states")
                .expect("lib_state_out.states");
            let StateValue::Items { items, .. } = after else {
                panic!("{sha}: lib_state_out.states must be an array, found {after:?}");
            };
            assert_eq!(items.len(), 3, "{sha}: three elements");
            // Pointers are null or offsets into a region; never a raw address.
            let mut found = Vec::new();
            pointers_of(after, &mut found);
            let mut into_input = false;
            let mut into_states = false;
            for pointer in found {
                match pointer {
                    Pointer::Null => {}
                    Pointer::Buffer { region, offset } => {
                        assert!(
                            ["in", "states", "out"].contains(&region.as_str()),
                            "{sha}: pointer into {region}"
                        );
                        into_input |= region == "in" && *offset == 0;
                        into_states |= region == "states";
                    }
                    Pointer::Symbol(name) => panic!("{sha}: unexpected function {name}"),
                }
            }
            assert!(into_states, "{sha}: constant_state points into the array");
            if input_len > 0 {
                assert!(into_input, "{sha}: payload is the input at offset 0");
            }
        }
        assert_eq!(
            capacities.into_iter().collect::<Vec<_>>(),
            ["0", "64"],
            "{config}: the out buffer sweep"
        );
    }
}

// ---------------------------------------------------------------------------
// Plan 03-08: the bounded attempts at the entries the earlier plans left (CONTEXT
// D-13, D-24). An attempted entry ends in identical vectors twice or in one row of
// testbed/adapter-dropped.toml with its cause, detail and the attempt made.
// ---------------------------------------------------------------------------

/// The drop row of `target`/`entry` in the committed adapter drop log, if any.
fn adapter_drop_row(target: &str, entry: &str) -> Option<mt_harness::adapter_drops::AdapterDrop> {
    let log = mt_harness::adapter_drops::AdapterDropLog::load_from_testbed_dir(
        &repo_root().join("testbed"),
    )
    .unwrap();
    log.row(target, entry).cloned()
}

/// Run one attempted entry in each configuration, or accept its drop row. Returns
/// the directories of the first corpus of each configuration it ran (empty when the
/// entry is logged).
fn attempt(
    target: &str,
    entry: &str,
    configs: &[&str],
) -> Vec<(String, PathBuf, tempfile::TempDir)> {
    if let Some(row) = adapter_drop_row(target, entry) {
        assert!(
            !row.detail.trim().is_empty() && !row.attempt.trim().is_empty(),
            "{target}/{entry}: a drop row names its detail and the attempt made"
        );
        println!(
            "{target}/{entry} logged {} ({})",
            row.cause.as_str(),
            row.detail
        );
        return Vec::new();
    }
    let mut ran = Vec::new();
    for config in configs {
        let tmp = tempfile::tempdir().unwrap();
        let (summary, dir) = run_pair_twice(target, config, entry, tmp.path());
        assert!(
            summary.vectors > 0,
            "{target}/{config}/{entry}: {summary:?}"
        );
        ran.push(((*config).to_owned(), dir, tmp));
    }
    ran
}

/// mbedtls `mbedtls_x509_crt_parse_der`: a setup call initialises the chain, the
/// parsed certificate and its heap parts are recorded (CONTEXT D-06, D-09, D-12).
fn check_mbedtls_x509(config: &str, dir: &Path) {
    let corpus = read_corpus(dir).unwrap();
    let (mut parsed, mut failed, mut heap_objects) = (0usize, 0usize, 0usize);
    for (sha, vector) in &corpus.vectors {
        let input_len = decode_latin1(&vector.stdin).unwrap().len();
        assert_eq!(
            vector.argv,
            [
                format!("mbedtls/{config}/mbedtls_x509_crt_parse_der"),
                input_len.to_string()
            ],
            "{sha}: argv holds the input length"
        );
        // lib_state_in is the chain after mbedtls_x509_crt_init: all zero.
        let before = vector.lib_state_in.get("crt").expect("lib_state_in.crt");
        assert!(is_all_zero(before), "{sha}: the chain starts initialised");
        let Some(after) = vector.lib_state_out.get("crt") else {
            // A fault or timeout has no frame; none is expected here.
            panic!("{sha}: lib_state_out.crt is not recorded");
        };
        if vector.rc == 0 {
            parsed += 1;
            // Every pointer of the parsed chain is null or points into the input,
            // the chain itself or the heap the library allocated.
            let mut found = Vec::new();
            pointers_of(after, &mut found);
            let mut into_heap = false;
            for pointer in found {
                match pointer {
                    Pointer::Null => {}
                    Pointer::Buffer { region, .. } => {
                        assert!(
                            ["in", "crt", "heap"].contains(&region.as_str()),
                            "{sha}: pointer into {region}"
                        );
                        into_heap |= region == "heap";
                    }
                    Pointer::Symbol(name) => panic!("{sha}: unexpected function {name}"),
                }
            }
            assert!(
                into_heap,
                "{sha}: the certificate copies its DER into the heap"
            );
            let heap = vector
                .lib_state_out
                .get("heap")
                .expect("lib_state_out.heap");
            let objects = struct_fields(heap, "lib_state_out.heap");
            assert!(!objects.is_empty(), "{sha}: the heap holds the chain parts");
            heap_objects += objects.len();
            assert_ne!(
                Some(after),
                Some(before),
                "{sha}: parsing changes the chain"
            );
        } else {
            failed += 1;
            assert!(
                vector.rc < 0,
                "{sha}: a library error code is negative, found {}",
                vector.rc
            );
        }
    }
    println!(
        "mbedtls/{config}/mbedtls_x509_crt_parse_der parsed={parsed} failed={failed} heap_objects={heap_objects}"
    );
    assert!(parsed > 0, "{config}: at least one certificate parses");
    assert!(failed > 0, "{config}: at least one input is refused");
}

/// coreMQTT `MQTT_DeserializePublish` and `MQTT_DeserializeAck`: the packet
/// information is a state filled by set steps from the input (the type byte, the
/// remaining data two bytes in, the length less two), the property builder is NULL
/// (CONTEXT D-12, D-14). `ack` also fixes the connection properties by set steps.
fn check_mqtt(config: &str, entry: &str, dir: &Path) {
    let corpus = read_corpus(dir).unwrap();
    let (mut success, mut refused) = (0usize, 0usize);
    for (sha, vector) in &corpus.vectors {
        let input = decode_latin1(&vector.stdin).unwrap();
        assert_eq!(
            vector.argv[0],
            format!("coremqtt/{config}/{entry}"),
            "{sha}"
        );
        let packet = struct_fields(
            vector
                .lib_state_in
                .get("packet")
                .expect("lib_state_in.packet"),
            "lib_state_in.packet",
        );
        let first = input.first().map_or(0, |byte| i64::from(*byte));
        assert_eq!(int_field(packet, "type"), first, "{sha}: type is $in[0]");
        if input.len() >= 2 {
            assert_eq!(
                int_field(packet, "remainingLength"),
                input.len() as i64 - 2,
                "{sha}: remainingLength is $len-2"
            );
            assert!(
                matches!(
                    &packet["pRemainingData"],
                    StateValue::Pointer { pointer: Pointer::Buffer { region, offset }, .. }
                        if region == "in" && *offset == 2
                ),
                "{sha}: pRemainingData is the input at 2: {:?}",
                packet["pRemainingData"]
            );
        }
        if entry == "MQTT_DeserializeAck" {
            let connect = struct_fields(
                vector
                    .lib_state_in
                    .get("connectProperties")
                    .expect("lib_state_in.connectProperties"),
                "lib_state_in.connectProperties",
            );
            assert_eq!(int_field(connect, "maxPacketSize"), 4096, "{sha}");
        }
        if vector.rc == 0 {
            success += 1;
        } else {
            refused += 1;
        }
    }
    println!("coremqtt/{config}/{entry} success={success} refused={refused}");
    assert!(
        success > 0,
        "{config}/{entry}: at least one packet is accepted"
    );
    assert!(
        refused > 0,
        "{config}/{entry}: at least one packet is refused"
    );
}

/// lz4 `LZ4F_getFrameInfo`: the decompression context is a pointer local made by a
/// setup call, the source size a pointer-carried length initialised from the input
/// (CONTEXT D-11, D-12).
fn check_lz4f(config: &str, dir: &Path) {
    let corpus = read_corpus(dir).unwrap();
    let (mut accepted, mut refused) = (0usize, 0usize);
    for (sha, vector) in &corpus.vectors {
        let input_len = decode_latin1(&vector.stdin).unwrap().len();
        assert_eq!(
            vector.argv,
            [
                format!("lz4/{config}/LZ4F_getFrameInfo"),
                input_len.to_string()
            ],
            "{sha}: argv holds the initial source size"
        );
        assert_eq!(
            recorded_int(&vector.lib_state_in, "srcSize"),
            input_len as i64,
            "{sha}: srcSize starts as the input length"
        );
        let consumed = recorded_int(&vector.lib_state_out, "srcSize");
        assert!(
            (0..=input_len as i64).contains(&consumed),
            "{sha}: {consumed} bytes consumed of {input_len}"
        );
        if vector.rc >= 0 {
            accepted += 1;
        } else {
            refused += 1;
        }
    }
    println!("lz4/{config}/LZ4F_getFrameInfo accepted={accepted} refused={refused}");
    assert!(accepted > 0, "{config}: at least one frame header is read");
}

#[test]
#[ignore = "reads the captures of an earlier local run; set MT_LOCAL_CAPTURES"]
fn eqv_04_tier_c_attempts() {
    for (config, dir, _tmp) in attempt(
        "mbedtls",
        "mbedtls_x509_crt_parse_der",
        &["default", "suite-b"],
    ) {
        check_mbedtls_x509(&config, &dir);
    }
    for entry in ["MQTT_DeserializePublish", "MQTT_DeserializeAck"] {
        for (config, dir, _tmp) in attempt("coremqtt", entry, &["default", "sub-vectors-2"]) {
            check_mqtt(&config, entry, &dir);
        }
    }
    for (config, dir, _tmp) in attempt("lz4", "LZ4F_getFrameInfo", &["default", "heap-small"]) {
        check_lz4f(&config, &dir);
    }
    // nimble's advertising parser: one link attempt. Either it builds and runs
    // identically twice, or the entry is logged with the unresolved names.
    for (config, _dir, _tmp) in attempt(
        "nimble",
        "ble_hs_adv_parse_fields",
        &["default", "peripheral"],
    ) {
        println!("nimble/{config}/ble_hs_adv_parse_fields driven");
    }
    // isotp, zlib inflate and nanopb pb_decode_ex: isotp has no local objects
    // (its roles are proven in the image); the other two must be logged.
    for (target, entry) in [("zlib", "inflate"), ("nanopb", "pb_decode_ex")] {
        let row = adapter_drop_row(target, entry).unwrap_or_else(|| {
            panic!("{target}/{entry} must be logged: its attempt is a reasoned drop")
        });
        println!(
            "{target}/{entry} logged {} ({})",
            row.cause.as_str(),
            row.detail
        );
    }
}

#[test]
#[ignore = "reads the captures of an earlier local run; set MT_LOCAL_CAPTURES"]
fn eqv_04_sample_is_chosen_and_replays_from_a_tree() {
    // D-21: the sample is chosen from a corpus by the fixed rule, written as one-vector
    // corpora, and replayed from the tree (`mt vectors replay --tree`) against the
    // driver of the same build.
    use mt_harness::replay::{ReplayOptions, replay_tree};
    use mt_harness::sample::{load_inputs, select_sample, write_sample};

    let work = local_work_dir();
    let tmp = tempfile::tempdir().unwrap();
    let scope = scope_for(
        "corejson",
        "default",
        "JSON_SearchT",
        &tmp.path().join("build"),
        &work,
    );
    build_ok(&scope);
    let corpus = tmp.path().join("corpus");
    run_into(&scope, &corpus, &tmp.path().join("slots"));

    let (manifest, drops) = load_inputs(&repo_root().join("testbed/manifest.toml")).unwrap();
    let choices = select_sample(&corpus, &manifest, &drops, &["corejson".to_owned()]).unwrap();
    assert_eq!(choices.len(), 1, "{choices:?}");
    assert_eq!(
        (
            choices[0].target.as_str(),
            choices[0].config.as_str(),
            choices[0].entry.as_str()
        ),
        ("corejson", "default", "JSON_SearchT")
    );
    let sample = tmp.path().join("sample");
    write_sample(&corpus, &choices, &sample).unwrap();
    let written = read_corpus(&sample.join("corejson/default/JSON_SearchT")).unwrap();
    assert_eq!(written.vectors.len(), 1);
    assert!(written.vectors.contains_key(&choices[0].vector_sha256));

    let options = ReplayOptions {
        scope,
        tools: host_tools(),
        cfg: runner_config(),
        scratch_dir: tmp.path().join("replay-slots"),
    };
    let reports = replay_tree(&options, &sample).unwrap();
    assert_eq!(reports.len(), 1);
    assert!(reports[0].passed(), "{}", reports[0].render());
    println!(
        "{}",
        reports[0]
            .render()
            .replace(tmp.path().to_str().unwrap(), "<tmp>")
    );
    assert!(
        replay_tree(&options, &tmp.path().join("slots")).is_err(),
        "no corpus below"
    );
}
