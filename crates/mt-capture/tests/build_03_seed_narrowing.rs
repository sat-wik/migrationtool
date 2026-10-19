#![allow(clippy::unwrap_used, clippy::expect_used)]
//! BUILD-03 seed narrowing (G-02-3): a seed source may name `designators`, so
//! that its string seeds are only the values of those designated members
//! (httpparser's `.raw` messages), and the seed sources of httpparser, nanopb and
//! qcbor hold only the arrays and fixture files that carry parser inputs.
//! Plain `cargo test`: temporary trees and committed files only; no network and
//! no external process. Targets are always named explicitly, never `--all`.

use std::fs;
use std::path::{Path, PathBuf};

use mt_capture::manifest::{Manifest, ManifestError, Target};
use mt_capture::seeding::seed_target;
use mt_seeds::Method;
use mt_seeds::index::SeedIndex;
use mt_seeds::select::Caps;
use tempfile::TempDir;

const SMALL: &str = include_str!("fixtures/manifest-small.toml");
const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

fn with_extra(extra: &str) -> String {
    format!("{SMALL}\n{extra}\n")
}

fn issues_of(text: &str) -> Vec<String> {
    let manifest = Manifest::parse(text).expect("the text parses");
    match manifest.validate() {
        Ok(()) => Vec::new(),
        Err(ManifestError::Invalid { issues }) => issues,
        Err(other) => panic!("unexpected error: {other}"),
    }
}

fn target_with(extra: &str) -> Target {
    let manifest = Manifest::parse(&with_extra(extra)).expect("the text parses");
    manifest.validate().expect("the manifest is valid");
    manifest.target["demo_make"].clone()
}

fn write(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

fn small_caps() -> Caps {
    Caps {
        max_seeds: 200,
        max_total_bytes: 1_048_576,
        max_seed_bytes: 65_536,
    }
}

fn any_issue(issues: &[String], needle: &str) -> bool {
    issues.iter().any(|issue| issue.contains(needle))
}

#[test]
fn build_03_seed_source_designators_validate() {
    let source =
        |body: &str| format!("[[target.demo_make.seed_source]]\npath = \"test.c\"\n{body}");

    // The key is optional, and valid with exactly the c_string method.
    assert_eq!(
        issues_of(&with_extra(&source("methods = [\"c_string\"]\n"))),
        Vec::<String>::new()
    );
    assert_eq!(
        issues_of(&with_extra(&source(
            "methods = [\"c_string\"]\ndesignators = [\"raw\"]\n"
        ))),
        Vec::<String>::new()
    );
    assert_eq!(
        issues_of(&with_extra(&source(
            "methods = [\"c_string\"]\ndesignators = [\"raw\", \"_body2\"]\n"
        ))),
        Vec::<String>::new()
    );

    // Designators only make sense for string extraction and only alone.
    for methods in [
        "[\"c_array\"]",
        "[\"c_array\", \"c_string\"]",
        "[\"file_copy\"]",
    ] {
        let issues = issues_of(&with_extra(&source(&format!(
            "methods = {methods}\ndesignators = [\"raw\"]\n"
        ))));
        assert!(
            any_issue(&issues, "target.demo_make.seed_source[0].designators"),
            "methods {methods}: {issues:?}"
        );
    }

    // An explicit empty list selects nothing and is meaningless.
    let issues = issues_of(&with_extra(&source(
        "methods = [\"c_string\"]\ndesignators = []\n",
    )));
    assert!(
        any_issue(&issues, "target.demo_make.seed_source[0].designators"),
        "{issues:?}"
    );

    // Every name is a C identifier.
    for name in ["", "1raw", "a-b", "a b", ".raw", "raw=", "a.b", "r\\u00e9"] {
        let issues = issues_of(&with_extra(&source(&format!(
            "methods = [\"c_string\"]\ndesignators = [{name:?}]\n"
        ))));
        assert!(
            any_issue(&issues, "target.demo_make.seed_source[0].designators"),
            "{name:?}: {issues:?}"
        );
    }
}

#[test]
fn build_03_seeding_takes_only_designated_strings() {
    let target = target_with(
        "[[target.demo_make.seed_source]]\n\
         path = \"test.c\"\n\
         methods = [\"c_string\"]\n\
         designators = [\"raw\"]\n\
         entries = [\"demo_decode\"]\n",
    );
    let tmp = TempDir::new().unwrap();
    let tree = tmp.path().join("tree");
    write(
        &tree.join("test.c"),
        b"static const struct message m[] = {\n\
          { .name= \"curl get\", .raw= \"GET / HTTP/1.1\\r\\n\" /* c */ \"Host: x\\r\\n\"\n\
          \"\\r\\n\", .body= \"\" },\n\
          { .raw = \"POST / HTTP/1.0\\r\\n\\r\\n\" }\n\
          };\n\
          void f(void) { printf(\"error %d\\n\", 1); m->raw = \"x\"; s.raw = \"y\"; }\n",
    );
    let hand = tmp.path().join("hand-made");

    let (index, selection) =
        seed_target("demo_make", &tree, COMMIT, &target, &hand, small_caps()).unwrap();

    assert_eq!(index.seeds.len(), 2, "{:#?}", index.seeds);
    assert_eq!(selection.seeds.len(), 2);
    let mut found: Vec<(Vec<u8>, u32)> = index
        .seeds
        .iter()
        .map(|seed| {
            assert_eq!(seed.method, Method::CString);
            assert_eq!(seed.source.path, "test.c");
            assert_eq!(seed.entries, vec!["demo_decode".to_owned()]);
            assert!(!seed.proposed);
            (
                selection
                    .seeds
                    .iter()
                    .find(|s| s.sha256 == seed.sha256)
                    .unwrap_or_else(|| panic!("no selected bytes for {}", seed.sha256))
                    .candidate
                    .bytes
                    .clone(),
                seed.source.line.unwrap(),
            )
        })
        .collect();
    found.sort_by_key(|(_, line)| *line);
    assert_eq!(
        found,
        vec![
            (b"GET / HTTP/1.1\r\nHost: x\r\n\r\n".to_vec(), 2),
            (b"POST / HTTP/1.0\r\n\r\n".to_vec(), 4),
        ]
    );
}

fn repo_file(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn committed_manifest() -> Manifest {
    let manifest = Manifest::load(&repo_file("testbed/manifest.toml")).expect("manifest parses");
    manifest.validate().expect("manifest is valid");
    manifest
}

fn committed_index(id: &str) -> SeedIndex {
    let path = repo_file("testbed/seeds").join(id).join("index.json");
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} cannot be read: {error}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// The paths of the declared sources that name a method.
fn source_paths_with(target: &Target, method: &str) -> Vec<String> {
    target
        .seed_source
        .iter()
        .filter(|source| source.methods.iter().any(|m| m == method))
        .map(|source| source.path.clone())
        .collect()
}

#[test]
fn build_03_narrowed_corpora_hold_only_input_methods() {
    let manifest = committed_manifest();

    // qcbor: byte arrays of the CBOR test files only; no string source, and
    // neither the buffer utility tests nor the runner (labels and messages).
    let qcbor = &manifest.target["qcbor"];
    assert_eq!(
        source_paths_with(qcbor, "c_string"),
        Vec::<String>::new(),
        "qcbor must declare no c_string source"
    );
    let mut declared: Vec<&str> = qcbor.seed_source.iter().map(|s| s.path.as_str()).collect();
    declared.sort_unstable();
    assert_eq!(
        declared,
        vec![
            "test/float_tests.c",
            "test/not_well_formed_cbor.h",
            "test/qcbor_decode_tests.c",
            "test/qcbor_encode_tests.c",
        ]
    );
    let index = committed_index("qcbor");
    assert!(!index.seeds.is_empty());
    for seed in &index.seeds {
        assert_eq!(
            seed.method,
            Method::CArray,
            "qcbor seed {} from {} is not a byte array",
            seed.sha256,
            seed.source.path
        );
        assert!(
            !seed.source.path.ends_with("UsefulBuf_Tests.c")
                && !seed.source.path.ends_with("run_tests.c"),
            "qcbor seed {} comes from {}",
            seed.sha256,
            seed.source.path
        );
    }

    // nanopb: fixture files and fuzz corpora only; the strings under tests/ are
    // names and messages.
    let nanopb = &manifest.target["nanopb"];
    assert_eq!(
        source_paths_with(nanopb, "c_string"),
        Vec::<String>::new(),
        "nanopb must declare no c_string source"
    );
    let index = committed_index("nanopb");
    assert!(!index.seeds.is_empty());
    for seed in &index.seeds {
        assert!(
            matches!(seed.method, Method::FileCopy | Method::ZipMember),
            "nanopb seed {} from {} has method {:?}",
            seed.sha256,
            seed.source.path,
            seed.method
        );
    }

    // httpparser: the `.raw` messages of test.c and nothing else of it.
    let http = &manifest.target["httpparser"];
    assert_eq!(http.seed_source.len(), 1);
    let source = &http.seed_source[0];
    assert_eq!(source.path, "test.c");
    assert_eq!(source.methods, vec!["c_string".to_owned()]);
    assert_eq!(source.designators, Some(vec!["raw".to_owned()]));
    let index = committed_index("httpparser");
    assert!(!index.seeds.is_empty());
    assert!(
        index.seeds.len() <= 77,
        "test.c holds 77 `.raw` members, the corpus holds {}",
        index.seeds.len()
    );
    for seed in &index.seeds {
        assert_eq!(seed.method, Method::CString);
        assert_eq!(seed.source.path, "test.c");
    }
}
