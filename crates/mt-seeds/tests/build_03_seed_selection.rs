//! BUILD-03: zip members, deterministic selection under caps, the seed index
//! and the corpus writer and checker (D-15, D-16).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeSet;
use std::fs;
use std::io::{Cursor, Write};
use std::path::Path;

use mt_seeds::index::{
    IndexLicence, SEED_INDEX_SCHEMA_VERSION, SeedIndex, check_corpus, write_corpus,
};
use mt_seeds::select::{Candidate, Caps, Selection, SourceRef, select};
use mt_seeds::zipsrc::zip_members;
use mt_seeds::{Method, SeedError, sha256_hex};

const NOTICE: &str = "SPDX-License-Identifier: MIT\nUpstream licence file: LICENSE\n";

fn cand(bytes: &[u8], method: Method, path: &str, line: u32) -> Candidate {
    Candidate {
        bytes: bytes.to_vec(),
        method,
        source: SourceRef {
            path: path.to_owned(),
            line: Some(line),
            member: None,
        },
        entries: vec!["entry".to_owned()],
        proposed: false,
    }
}

fn caps(max_seeds: u64, max_total_bytes: u64, max_seed_bytes: u64) -> Caps {
    Caps {
        max_seeds,
        max_total_bytes,
        max_seed_bytes,
    }
}

fn wide_caps() -> Caps {
    caps(1000, 1 << 20, 1 << 16)
}

fn licence() -> IndexLicence {
    IndexLicence {
        spdx: "MIT".to_owned(),
        elected: None,
        file: "LICENSE".to_owned(),
    }
}

fn index_for(selection: &Selection, caps: Caps) -> SeedIndex {
    SeedIndex::from_selection("demo", "0123abcd", licence(), caps, selection)
}

fn paths_of(selection: &Selection) -> Vec<String> {
    selection
        .seeds
        .iter()
        .map(|s| s.candidate.source.path.clone())
        .collect()
}

/// A mixed fixture with ties on method, path and line.
fn fixture() -> Vec<Candidate> {
    vec![
        cand(b"\x01\x02", Method::CArray, "t/a.c", 10),
        cand(b"\x03", Method::CArray, "t/a.c", 20),
        cand(b"\x04\x05\x06", Method::CArray, "t/b.c", 5),
        cand(b"one", Method::CString, "t/a.c", 30),
        cand(b"two", Method::CString, "t/a.c", 30),
        cand(b"\xaa", Method::HexText, "t/x.cborhex", 1),
        cand(b"\xbb\xcc", Method::HexText, "t/y.cborhex", 1),
        cand(b"file", Method::FileCopy, "t/in/one", 1),
        cand(b"", Method::FileCopy, "t/in/empty", 1),
        cand(b"\x01\x02", Method::CString, "t/c.c", 3),
        cand(b"zz", Method::HandMade, "research/gen.py", 1),
        Candidate {
            bytes: b"member".to_vec(),
            method: Method::ZipMember,
            source: SourceRef {
                path: "fuzz/corpus.zip".to_owned(),
                line: None,
                member: Some("m1".to_owned()),
            },
            entries: Vec::new(),
            proposed: false,
        },
    ]
}

fn list_dir(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn build_03_seed_names_are_sha256_of_contents() {
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );

    let selection = select(fixture(), &wide_caps(), 0);
    let index = index_for(&selection, wide_caps());
    assert!(!selection.seeds.is_empty());

    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("seeds/demo");
    // A stale file from an earlier run disappears.
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("stale"), b"old").unwrap();

    write_corpus(&dir, &index, &selection, NOTICE).unwrap();

    let mut expected: BTreeSet<String> = selection.seeds.iter().map(|s| s.sha256.clone()).collect();
    expected.insert("index.json".to_owned());
    expected.insert("NOTICE".to_owned());
    assert_eq!(
        list_dir(&dir),
        expected.iter().cloned().collect::<Vec<_>>(),
        "the directory holds exactly the seed files, index.json and NOTICE"
    );
    for seed in &selection.seeds {
        let on_disk = fs::read(dir.join(&seed.sha256)).unwrap();
        assert_eq!(on_disk, seed.candidate.bytes);
        assert_eq!(sha256_hex(&on_disk), seed.sha256, "file name is the digest");
    }
    assert_eq!(fs::read_to_string(dir.join("NOTICE")).unwrap(), NOTICE);
    assert_eq!(
        fs::read_to_string(dir.join("index.json")).unwrap(),
        index.to_json().unwrap()
    );

    // A subdirectory is never deleted by regeneration.
    fs::create_dir(dir.join("sub")).unwrap();
    let err = write_corpus(&dir, &index, &selection, NOTICE).unwrap_err();
    assert!(matches!(err, SeedError::UnexpectedDirectory(_)), "{err}");
    assert!(dir.join("sub").is_dir());

    // An index that does not describe the selection is refused.
    fs::remove_dir(dir.join("sub")).unwrap();
    let other = select(
        vec![cand(b"other", Method::CArray, "o.c", 1)],
        &wide_caps(),
        0,
    );
    let wrong = index_for(&other, wide_caps());
    let err = write_corpus(&dir, &wrong, &selection, NOTICE).unwrap_err();
    assert!(matches!(err, SeedError::Inconsistent(_)), "{err}");
}

#[test]
fn build_03_seed_index_is_deterministic_and_versioned() {
    let base = fixture();
    let mut reversed = base.clone();
    reversed.reverse();
    let mut rotated = base.clone();
    rotated.rotate_left(5);
    let mut interleaved: Vec<Candidate> = base.iter().step_by(2).cloned().collect();
    interleaved.extend(base.iter().skip(1).step_by(2).cloned());

    let reference = select(base.clone(), &wide_caps(), 2);
    let reference_json = index_for(&reference, wide_caps()).to_json().unwrap();
    for (name, shuffled) in [
        ("reversed", reversed),
        ("rotated", rotated),
        ("interleaved", interleaved),
    ] {
        let selection = select(shuffled, &wide_caps(), 2);
        assert_eq!(selection, reference, "{name}: same selection");
        assert_eq!(
            index_for(&selection, wide_caps()).to_json().unwrap(),
            reference_json,
            "{name}: byte-identical index"
        );
    }

    let json: serde_json::Value = serde_json::from_str(&reference_json).unwrap();
    assert_eq!(json["schema_version"], SEED_INDEX_SCHEMA_VERSION);
    assert_eq!(SEED_INDEX_SCHEMA_VERSION, 1);
    assert_eq!(json["target"], "demo");
    assert_eq!(json["commit"], "0123abcd");
    assert_eq!(json["licence"]["spdx"], "MIT");
    assert_eq!(json["caps"]["max_seeds"], 1000);
    assert_eq!(json["skipped"]["unsupported"], 2);
    for key in ["duplicate", "too_large", "over_cap"] {
        assert!(json["skipped"][key].is_u64(), "skipped.{key}");
    }
    let seeds = json["seeds"].as_array().unwrap();
    let digests: Vec<&str> = seeds
        .iter()
        .map(|s| s["sha256"].as_str().unwrap())
        .collect();
    let mut sorted = digests.clone();
    sorted.sort_unstable();
    assert_eq!(digests, sorted, "seeds are sorted by sha256");
    for seed in seeds {
        for key in ["sha256", "bytes", "entries", "method", "source", "proposed"] {
            assert!(!seed[key].is_null(), "seed.{key}");
        }
    }
    let methods: BTreeSet<&str> = seeds
        .iter()
        .map(|s| s["method"].as_str().unwrap())
        .collect();
    assert_eq!(
        methods,
        BTreeSet::from([
            "hand_made",
            "file_copy",
            "hex_text",
            "c_array",
            "c_string",
            "zip_member"
        ])
    );
    let member = seeds.iter().find(|s| s["method"] == "zip_member").unwrap();
    assert_eq!(member["source"]["member"], "m1");
    assert!(member["source"]["line"].is_null());
    assert!(reference_json.ends_with("}\n"));
}

#[test]
fn build_03_seed_caps_are_enforced_deterministically() {
    // One large file next to two small ones: the round robin keeps one
    // candidate from each of the three files.
    let mut candidates: Vec<Candidate> = (0u8..10)
        .map(|i| cand(&[0x10, i], Method::CArray, "t/a.c", 10 + u32::from(i)))
        .collect();
    candidates.push(cand(&[0x20], Method::CArray, "t/b.c", 1));
    candidates.push(cand(&[0x30], Method::CArray, "t/c.c", 1));

    let limit = caps(3, 1 << 20, 1 << 16);
    let selection = select(candidates.clone(), &limit, 0);
    assert_eq!(selection.seeds.len(), 3);
    assert_eq!(paths_of(&selection), ["t/a.c", "t/b.c", "t/c.c"]);
    // The first candidate of the large file (by line) is the one kept.
    assert_eq!(selection.seeds[0].candidate.bytes, vec![0x10, 0]);
    assert_eq!(selection.skipped.over_cap, 9);

    let mut reversed = candidates;
    reversed.reverse();
    assert_eq!(
        select(reversed, &limit, 0),
        selection,
        "input order is irrelevant"
    );

    // Total cap: exactly at the cap is accepted, one byte over is not.
    let four = [
        cand(b"aaaa", Method::CArray, "a.c", 1),
        cand(b"bbbb", Method::CArray, "b.c", 1),
        cand(b"cccc", Method::CArray, "c.c", 1),
    ];
    let selection = select(four.to_vec(), &caps(100, 8, 100), 0);
    assert_eq!(selection.seeds.len(), 2, "8 bytes exactly fits in 8");
    assert_eq!(selection.skipped.over_cap, 1);
    let selection = select(four.to_vec(), &caps(100, 7, 100), 0);
    assert_eq!(selection.seeds.len(), 1, "the second seed would make 8 > 7");
    assert_eq!(selection.skipped.over_cap, 2);

    // Selection prefers methods in rank order, whatever the input order.
    let ranked = vec![
        cand(b"6", Method::ZipMember, "z.zip", 1),
        cand(b"5", Method::CString, "c.c", 1),
        cand(b"4", Method::CArray, "c.c", 2),
        cand(b"3", Method::HexText, "h.hex", 1),
        cand(b"2", Method::FileCopy, "f", 1),
        cand(b"1", Method::HandMade, "g.py", 1),
    ];
    let selection = select(ranked.clone(), &caps(2, 100, 100), 0);
    let kept: Vec<Method> = selection.seeds.iter().map(|s| s.candidate.method).collect();
    assert_eq!(kept, [Method::HandMade, Method::FileCopy]);
    assert_eq!(selection.skipped.over_cap, 4);

    // The unsupported count passed in is copied through.
    assert_eq!(select(ranked, &wide_caps(), 7).skipped.unsupported, 7);
}

#[test]
fn build_03_duplicate_seeds_collapse_to_one() {
    let bytes = b"same bytes";
    // The first in selection order wins: a file copy outranks a C array and
    // paths order within a method, whatever order the candidates arrive in.
    let candidates = vec![
        cand(bytes, Method::CArray, "a.c", 1),
        cand(bytes, Method::FileCopy, "z/fixture", 1),
        cand(bytes, Method::CArray, "b.c", 1),
    ];
    let selection = select(candidates.clone(), &wide_caps(), 0);
    assert_eq!(selection.seeds.len(), 1);
    assert_eq!(selection.skipped.duplicate, 2);
    assert_eq!(selection.seeds[0].candidate.method, Method::FileCopy);
    assert_eq!(selection.seeds[0].sha256, sha256_hex(bytes));

    let mut reversed = candidates;
    reversed.reverse();
    assert_eq!(select(reversed, &wide_caps(), 0), selection);
}

#[test]
fn build_03_seed_size_caps_are_inclusive() {
    // A seed of exactly max_seed_bytes is accepted; one byte more is too_large.
    let candidates = vec![
        cand(b"1234", Method::CArray, "a.c", 1),
        cand(b"12345", Method::CArray, "b.c", 1),
    ];
    let selection = select(candidates, &caps(10, 100, 4), 0);
    assert_eq!(selection.seeds.len(), 1);
    assert_eq!(selection.seeds[0].candidate.bytes, b"1234");
    assert_eq!(selection.skipped.too_large, 1);
    assert_eq!(selection.skipped.over_cap, 0);

    // max_seeds is inclusive too: exactly max_seeds seeds fit.
    let two = vec![
        cand(b"a", Method::CArray, "a.c", 1),
        cand(b"b", Method::CArray, "b.c", 1),
    ];
    let selection = select(two, &caps(2, 100, 100), 0);
    assert_eq!(selection.seeds.len(), 2);
    assert_eq!(selection.skipped.over_cap, 0);
}

#[test]
fn build_03_empty_input_is_one_seed() {
    let candidates = vec![
        cand(b"", Method::CString, "a.c", 1),
        cand(b"", Method::CString, "b.c", 2),
        cand(b"x", Method::CString, "a.c", 3),
    ];
    // The empty seed costs no bytes and appears once; the 1-byte total cap
    // still has room for the one-byte seed after it.
    let selection = select(candidates, &caps(10, 1, 10), 0);
    assert_eq!(selection.seeds.len(), 2);
    assert_eq!(selection.skipped.duplicate, 1);
    let empty = selection
        .seeds
        .iter()
        .find(|s| s.candidate.bytes.is_empty())
        .unwrap();
    assert_eq!(
        empty.sha256,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );

    let tmp = tempfile::tempdir().unwrap();
    let index = index_for(&selection, caps(10, 1, 10));
    write_corpus(tmp.path(), &index, &selection, NOTICE).unwrap();
    assert_eq!(fs::read(tmp.path().join(&empty.sha256)).unwrap().len(), 0);
}

fn zip_with(members: &[(&str, &[u8], bool)], directories: &[&str]) -> Vec<u8> {
    use zip::write::SimpleFileOptions;
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for dir in directories {
        writer
            .add_directory(*dir, SimpleFileOptions::default())
            .unwrap();
    }
    for (name, bytes, deflate) in members {
        let method = if *deflate {
            zip::CompressionMethod::Deflated
        } else {
            zip::CompressionMethod::Stored
        };
        writer
            .start_file(
                *name,
                SimpleFileOptions::default().compression_method(method),
            )
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

#[test]
fn build_03_zip_members_are_read_without_touching_disk() {
    let tmp = tempfile::tempdir().unwrap();
    let absolute = format!("{}/pwned", tmp.path().display());
    let big = vec![b'x'; 200];
    let huge_but_compressible = vec![0u8; 8 * 1024 * 1024];
    let archive = zip_with(
        &[
            ("a", b"AAAA", false),
            ("dir/b", b"BB", true),
            ("../evil", b"E", false),
            ("big", &big, false),
            ("bomb", &huge_but_compressible, true),
        ],
        &["somedir/"],
    );

    let result = zip_members(&archive, 100, u64::MAX).unwrap();
    let names: Vec<&str> = result.members.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, ["a", "dir/b"], "archive order, directories skipped");
    assert_eq!(result.members[0].bytes, b"AAAA");
    assert_eq!(result.members[1].bytes, b"BB");
    assert_eq!(result.unsupported, 1, "../evil is not an enclosed name");
    assert_eq!(result.too_large, 2, "big and the compressed bomb");

    // An absolute name, and a name the zip crate would silently fold
    // (`x/../y` becomes `y`), are unsupported; nothing appears on disk.
    let archive = zip_with(
        &[
            (absolute.as_str(), b"P", false),
            ("x/../y", b"F", false),
            ("ok", b"K", false),
        ],
        &[],
    );
    let result = zip_members(&archive, 100, u64::MAX).unwrap();
    assert_eq!(result.members.len(), 1);
    assert_eq!(result.members[0].name, "ok");
    assert_eq!(result.unsupported, 2);
    assert!(!Path::new(&absolute).exists());
    assert!(
        list_dir(tmp.path()).is_empty(),
        "no member was written to disk"
    );

    // The size limit is inclusive.
    let archive = zip_with(
        &[("exact", &[7u8; 100], false), ("over", &[7u8; 101], false)],
        &[],
    );
    let result = zip_members(&archive, 100, u64::MAX).unwrap();
    assert_eq!(result.members.len(), 1);
    assert_eq!(result.members[0].name, "exact");
    assert_eq!(result.too_large, 1);

    // Not an archive at all.
    let err = zip_members(b"this is not a zip file", 100, u64::MAX).unwrap_err();
    assert!(matches!(err, SeedError::Zip(_)), "{err}");
}

#[test]
fn build_03_check_mode_detects_drift() {
    let selection = select(fixture(), &wide_caps(), 0);
    let index = index_for(&selection, wide_caps());
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("corpus");

    // A directory that does not exist yet is all missing.
    let missing = check_corpus(&dir, &index, &selection, NOTICE).unwrap();
    assert_eq!(missing.len(), selection.seeds.len() + 2);
    assert!(missing.iter().all(|l| l.starts_with("missing ")));

    write_corpus(&dir, &index, &selection, NOTICE).unwrap();
    assert_eq!(
        check_corpus(&dir, &index, &selection, NOTICE).unwrap(),
        Vec::<String>::new()
    );

    let first = selection.seeds[0].sha256.clone();
    let second = selection.seeds[1].sha256.clone();
    fs::write(dir.join(&first), b"tampered").unwrap();
    fs::remove_file(dir.join(&second)).unwrap();
    fs::write(dir.join("extra.txt"), b"x").unwrap();
    fs::write(dir.join("NOTICE"), b"edited").unwrap();

    let mut expected = vec![
        format!("changed {first}"),
        format!("missing {second}"),
        "added extra.txt".to_owned(),
        "changed NOTICE".to_owned(),
    ];
    // One line per difference, ordered by file name.
    expected.sort_by(|a, b| {
        a.split_once(' ')
            .unwrap()
            .1
            .cmp(b.split_once(' ').unwrap().1)
    });
    assert_eq!(
        check_corpus(&dir, &index, &selection, NOTICE).unwrap(),
        expected
    );

    // Regenerating repairs it.
    write_corpus(&dir, &index, &selection, NOTICE).unwrap();
    assert!(
        check_corpus(&dir, &index, &selection, NOTICE)
            .unwrap()
            .is_empty()
    );

    // A changed index.json is drift as well.
    fs::write(dir.join("index.json"), b"{}\n").unwrap();
    assert_eq!(
        check_corpus(&dir, &index, &selection, NOTICE).unwrap(),
        vec!["changed index.json".to_owned()]
    );
}
