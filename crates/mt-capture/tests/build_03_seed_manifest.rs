#![allow(clippy::unwrap_used, clippy::expect_used)]
//! BUILD-03 manifest and extraction: the seed sources and caps a manifest may
//! declare, the extraction of seeds with provenance from a fixture tree, the rule
//! that only declared paths are read, and the drift check. Plain `cargo test`: the
//! trees are built in temporary directories, nothing touches the network and no
//! external process is started.

use std::fs;
use std::path::{Path, PathBuf};

use mt_capture::manifest::{Manifest, ManifestError, Target};
use mt_capture::seeding::{SeedingError, run_seeds, run_seeds_detailed, seed_target};
use mt_capture::testbed::{Selection, TestbedOptions};
use mt_capture::treehash::{sha256_hex, tree_sha256};
use mt_seeds::Method;
use mt_seeds::select::Caps;
use tempfile::TempDir;

const SMALL: &str = include_str!("fixtures/manifest-small.toml");
const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

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
fn build_03_seed_sources_validate() {
    let good = "[[target.demo_make.seed_source]]\n\
        path = \"tests/vectors\"\n\
        methods = [\"c_array\", \"c_string\"]\n\
        entries = [\"demo_decode\"]\n\
        \n\
        [[target.demo_make.seed_source]]\n\
        path = \"tests/inputs\"\n\
        methods = [\"file_copy\"]\n\
        extensions = [\"\", \"bin\"]\n";
    assert_eq!(issues_of(&with_extra(good)), Vec::<String>::new());

    let bad = |body: &str| {
        issues_of(&with_extra(&format!(
            "[[target.demo_make.seed_source]]\n{body}"
        )))
    };
    let issues = bad("path = \"t\"\nmethods = [\"hex\"]\n");
    assert!(
        any_issue(&issues, "target.demo_make.seed_source[0].methods"),
        "{issues:?}"
    );
    let issues = bad("path = \"t\"\nmethods = []\n");
    assert!(
        any_issue(&issues, "target.demo_make.seed_source[0].methods"),
        "{issues:?}"
    );
    let issues = bad("path = \"t\"\nmethods = [\"hand_made\"]\n");
    assert!(
        any_issue(&issues, "target.demo_make.seed_source[0].methods"),
        "hand_made is not an extraction method: {issues:?}"
    );
    for path in ["../gpl", "a/../../b", "/etc", ""] {
        let issues = bad(&format!("path = {path:?}\nmethods = [\"file_copy\"]\n"));
        assert!(
            any_issue(&issues, "target.demo_make.seed_source[0].path"),
            "{path:?}: {issues:?}"
        );
    }
    for extension in [".bin", "a/b", "b\\\\c"] {
        let issues = bad(&format!(
            "path = \"t\"\nmethods = [\"file_copy\"]\nextensions = [{extension:?}]\n"
        ));
        assert!(
            any_issue(&issues, "target.demo_make.seed_source[0].extensions"),
            "{extension:?}: {issues:?}"
        );
    }
    let issues = bad("path = \"t\"\nmethods = [\"file_copy\"]\nentries = [\"no_such_entry\"]\n");
    assert!(
        any_issue(&issues, "target.demo_make.seed_source[0].entries"),
        "{issues:?}"
    );

    // The caps are required and above zero, and one seed must fit the total.
    for (from, to, field) in [
        ("max_seeds = 200", "max_seeds = 0", "seed_caps.max_seeds"),
        (
            "max_total_bytes = 1048576",
            "max_total_bytes = 0",
            "seed_caps.max_total_bytes",
        ),
        (
            "max_seed_bytes = 65536",
            "max_seed_bytes = 0",
            "seed_caps.max_seed_bytes",
        ),
        (
            "max_seed_bytes = 65536",
            "max_seed_bytes = 2097152",
            "seed_caps.max_seed_bytes",
        ),
    ] {
        let issues = issues_of(&SMALL.replace(from, to));
        assert!(any_issue(&issues, field), "{to}: {issues:?}");
    }
    let without = SMALL.replace("[seed_caps]", "[not_seed_caps]");
    assert!(Manifest::parse(&without).is_err(), "seed_caps is required");
}

#[test]
fn build_03_seeding_extracts_with_provenance_from_a_fixture_tree() {
    let target = target_with(
        "[[target.demo_make.seed_source]]\n\
         path = \"tests/vectors\"\n\
         methods = [\"c_array\", \"c_string\"]\n\
         entries = [\"demo_decode\"]\n\
         \n\
         [[target.demo_make.seed_source]]\n\
         path = \"tests/hexes\"\n\
         methods = [\"hex_text\"]\n\
         extensions = [\"cborhex\"]\n\
         \n\
         [[target.demo_make.seed_source]]\n\
         path = \"tests/inputs\"\n\
         methods = [\"file_copy\"]\n\
         extensions = [\"bin\"]\n",
    );
    let tmp = TempDir::new().unwrap();
    let tree = tmp.path().join("tree");
    write(
        &tree.join("tests/vectors/data.c"),
        b"static const unsigned char a[] = { 0x01, 0x02, 0x03 };\nstatic const char *s = \"\\x04\\x05\";\n",
    );
    write(&tree.join("tests/hexes/one.cborhex"), b"# case one\n0a0b\n");
    write(&tree.join("tests/inputs/raw.bin"), &[0xde, 0xad]);
    write(&tree.join("tests/inputs/raw.txt"), b"not selected");
    let hand = tmp.path().join("hand-made");
    write(
        &hand.join("a.hex"),
        b"# RFC 0000 section 1, case a\n01 02\n",
    );
    write(&hand.join("b.hex"), b"# RFC 0000 section 2, case b\nff\n");
    write(&hand.join("README.md"), b"not a seed");

    let (index, selection) =
        seed_target("demo_make", &tree, COMMIT, &target, &hand, small_caps()).unwrap();

    assert_eq!(index.target, "demo_make");
    assert_eq!(index.commit, COMMIT);
    assert_eq!(index.licence.spdx, "BSD-3-Clause");
    assert_eq!(index.licence.elected.as_deref(), Some("BSD-3-Clause"));
    assert_eq!(index.licence.file, "LICENSE");
    assert_eq!(index.caps, small_caps());
    assert_eq!(index.seeds.len(), 6, "{:#?}", index.seeds);
    assert_eq!(selection.seeds.len(), 6);
    let sorted: Vec<&str> = index.seeds.iter().map(|s| s.sha256.as_str()).collect();
    let mut expected = sorted.clone();
    expected.sort_unstable();
    assert_eq!(sorted, expected, "seeds are sorted by sha256");

    let find = |bytes: &[u8]| {
        let sha = sha256_hex(bytes);
        index
            .seeds
            .iter()
            .find(|seed| seed.sha256 == sha)
            .unwrap_or_else(|| panic!("no seed for {bytes:?}"))
    };
    let both = vec!["demo_decode".to_owned(), "demo_next".to_owned()];

    let array = find(&[1, 2, 3]);
    assert_eq!(array.method, Method::CArray);
    assert_eq!(array.source.path, "tests/vectors/data.c");
    assert_eq!(array.source.line, Some(1));
    assert_eq!(array.entries, vec!["demo_decode".to_owned()]);
    assert!(!array.proposed);

    let string = find(&[4, 5]);
    assert_eq!(string.method, Method::CString);
    assert_eq!(string.source.path, "tests/vectors/data.c");
    assert_eq!(string.source.line, Some(2));
    assert!(!string.proposed);

    let hex = find(&[0x0a, 0x0b]);
    assert_eq!(hex.method, Method::HexText);
    assert_eq!(hex.source.path, "tests/hexes/one.cborhex");
    assert_eq!(hex.entries, both, "no entries listed means every entry");
    assert!(!hex.proposed);

    let copy = find(&[0xde, 0xad]);
    assert_eq!(copy.method, Method::FileCopy);
    assert_eq!(copy.source.path, "tests/inputs/raw.bin");
    assert!(!copy.proposed);

    for (bytes, name) in [(&[1u8, 2][..], "a.hex"), (&[0xff][..], "b.hex")] {
        let made = find(bytes);
        assert_eq!(made.method, Method::HandMade);
        assert!(made.proposed, "hand-made seeds are proposals");
        assert_eq!(
            made.source.path,
            format!("testbed/targets/demo_make/hand-made/{name}")
        );
        assert_eq!(made.entries, both);
    }
    assert!(
        !index.seeds.iter().any(|s| s.bytes == 12),
        "raw.txt is not selected"
    );
}

#[test]
fn build_03_seeding_reads_only_declared_sources() {
    let target = target_with(
        "[[target.demo_make.seed_source]]\n\
         path = \"lib\"\n\
         methods = [\"file_copy\"]\n",
    );
    let tmp = TempDir::new().unwrap();
    let tree = tmp.path().join("tree");
    write(&tree.join("lib/ok.bin"), b"ok");
    write(&tree.join("lib/sub/nested.bin"), b"nested");
    write(&tree.join("gpl/secret.bin"), b"secret");
    write(
        &tree.join("gpl/secret.c"),
        b"static const unsigned char x[] = {9,9};",
    );
    let big = vec![7u8; 8 * 1024 * 1024 + 1];
    write(&tree.join("lib/big.bin"), &big);
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("../gpl/secret.bin", tree.join("lib/link.bin")).unwrap();
        std::os::unix::fs::symlink("../gpl", tree.join("lib/dirlink")).unwrap();
    }
    let hand = tmp.path().join("no-hand-made");

    let (index, _) = seed_target("demo_make", &tree, COMMIT, &target, &hand, small_caps()).unwrap();
    let mut paths: Vec<&str> = index.seeds.iter().map(|s| s.source.path.as_str()).collect();
    paths.sort_unstable();
    assert_eq!(
        paths,
        vec!["lib/ok.bin", "lib/sub/nested.bin"],
        "{:#?}",
        index.seeds
    );
    assert_eq!(
        index.skipped.unsupported, 1,
        "the file above 8 MiB is counted unsupported"
    );

    // A declared path that is missing, or reached through a symlink, is an error:
    // the manifest says seeds are there, so silence would hide a pin or typo.
    let missing = target_with(
        "[[target.demo_make.seed_source]]\npath = \"nowhere\"\nmethods = [\"file_copy\"]\n",
    );
    let error = seed_target("demo_make", &tree, COMMIT, &missing, &hand, small_caps()).unwrap_err();
    assert!(
        matches!(error, SeedingError::SourceMissing { .. }),
        "{error}"
    );
    #[cfg(unix)]
    {
        let through = target_with(
            "[[target.demo_make.seed_source]]\npath = \"lib/dirlink\"\nmethods = [\"file_copy\"]\n",
        );
        let error =
            seed_target("demo_make", &tree, COMMIT, &through, &hand, small_caps()).unwrap_err();
        assert!(
            matches!(error, SeedingError::SourceSymlink { .. }),
            "{error}"
        );
    }
}

/// A one-target testbed whose verified cache is pre-filled: an archive source
/// needs no tool to be verified again, so no git or network is involved.
struct Bed {
    _tmp: TempDir,
    opts: TestbedOptions,
    seeds_dir: PathBuf,
}

fn make_bed() -> Bed {
    let tmp = TempDir::new().unwrap();
    let work = tmp.path().join("work");
    let commit = "89abcdef0123456789abcdef0123456789abcdef";
    let cache = work.join("cache/demo").join(commit);
    let tree = cache.join("tree");
    write(&tree.join("tests/a.bin"), &[1, 2, 3]);
    write(&tree.join("tests/b.bin"), &[4, 5]);
    let archive = b"pretend archive";
    write(&cache.join("archive.tar.bz2"), archive);
    let tree_hash = tree_sha256(&tree).unwrap();
    let testbed = tmp.path().join("testbed");
    write(
        &testbed.join("targets/demo/hand-made/one.hex"),
        b"# RFC 0000, case one\n10 11\n",
    );
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
         methods = [\"file_copy\"]\n\
         extensions = [\"bin\"]\n",
        archive_sha = sha256_hex(archive),
    );
    let manifest_path = testbed.join("manifest.toml");
    write(&manifest_path, manifest.as_bytes());
    let opts = TestbedOptions {
        manifest_path,
        pins_path: repo_root().join("container/pins.toml"),
        testbed_dir: testbed.clone(),
        work_dir: work,
        out_dir: tmp.path().join("out"),
        // Named, not `--all`: a one-target bed is a partial run (WR-08 floors
        // only the whole bed).
        selection: Selection::Ids(vec!["demo".to_owned()]),
        image: None,
    };
    Bed {
        seeds_dir: testbed.join("seeds"),
        opts,
        _tmp: tmp,
    }
}

#[test]
fn build_03_seed_check_reports_drift() {
    let bed = make_bed();

    // Nothing is committed yet: check mode reports every file missing.
    let run = run_seeds_detailed(&bed.opts, &bed.seeds_dir, true).unwrap();
    assert!(!run.summary.passed);
    assert_eq!(run.summary.rows["demo"], "drift");
    assert!(
        run.drift["demo"]
            .iter()
            .any(|line| line == "missing index.json"),
        "{:?}",
        run.drift
    );

    // Writing makes the corpus: three seeds (two copied, one hand-made).
    let written = run_seeds(&bed.opts, &bed.seeds_dir, false).unwrap();
    assert!(written.passed, "{written:?}");
    assert_eq!(written.rows["demo"], "seeds 3");
    let checked = run_seeds_detailed(&bed.opts, &bed.seeds_dir, true).unwrap();
    assert!(checked.summary.passed, "{:?}", checked.summary);
    assert!(checked.drift.is_empty());

    // One byte of one seed changes: drift, naming the file.
    let victim = bed.seeds_dir.join("demo").join(sha256_hex(&[1, 2, 3]));
    fs::write(&victim, [1, 2, 4]).unwrap();
    let drifted = run_seeds_detailed(&bed.opts, &bed.seeds_dir, true).unwrap();
    assert!(!drifted.summary.passed);
    assert_eq!(drifted.summary.rows["demo"], "drift");
    assert_eq!(
        drifted.drift["demo"],
        vec![format!("changed {}", sha256_hex(&[1, 2, 3]))]
    );
    // Check mode never repairs.
    assert_eq!(fs::read(&victim).unwrap(), [1, 2, 4]);

    // A cache that no longer matches its pin is a failed row, not seeds.
    let tree_file = bed
        .opts
        .work_dir
        .join("cache/demo/89abcdef0123456789abcdef0123456789abcdef/tree/tests/a.bin");
    fs::write(tree_file, [9]).unwrap();
    let failed = run_seeds_detailed(&bed.opts, &bed.seeds_dir, true).unwrap();
    assert!(!failed.summary.passed);
    assert!(
        failed.summary.rows["demo"].starts_with("failed: "),
        "{:?}",
        failed.summary.rows
    );
}
