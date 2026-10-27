#![allow(clippy::unwrap_used, clippy::expect_used)]
//! EQV-02, D-23: the pinned TRACTOR sample. The pin file records only the
//! repository, the commit, six paths and their sha256; the corpus is fetched
//! into the gitignored work directory and used locally only (licence unchecked
//! until Phase 12). Nothing from it is in the repository.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use mt_vectors::tractor::{MIN_TRACTOR_LOADED, TractorPin, check_tree};
use sha2::{Digest, Sha256};

const COMMIT: &str = "37960ee08a7c644ee66e29855eca33903f702e67";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Every regular file of at most 1 MiB under `dir`, skipping `.git`, `target`
/// and `.mt-work` and never following a symlink.
fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().into_owned();
        let kind = entry.file_type().unwrap();
        if kind.is_dir() {
            if !matches!(name.as_str(), ".git" | "target" | ".mt-work") {
                walk(&entry.path(), out);
            }
        } else if kind.is_file() && entry.metadata().unwrap().len() <= 1 << 20 {
            out.push(entry.path());
        }
    }
}

#[test]
fn eqv_02_tractor_pin_has_no_content() {
    let root = repo_root();
    let pin = TractorPin::load(&root.join("testbed/tractor-pin.toml")).unwrap();
    assert_eq!(pin.commit, COMMIT);
    assert!(pin.repo.starts_with("https://"), "{}", pin.repo);
    assert!(
        pin.file.len() >= MIN_TRACTOR_LOADED,
        "{} files",
        pin.file.len()
    );
    const { assert!(MIN_TRACTOR_LOADED >= 3) };
    for file in &pin.file {
        assert!(file.path.starts_with("Public-Tests/"), "{}", file.path);
        assert!(
            Path::new(&file.path)
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("json")),
            "{}",
            file.path
        );
        assert!(
            !file
                .path
                .split('/')
                .any(|c| c == ".." || c == "." || c.is_empty()),
            "{}",
            file.path
        );
        assert_eq!(file.sha256.len(), 64);
        assert!(
            file.sha256
                .bytes()
                .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
        );
    }
    assert!(pin.licence_status.contains("unchecked"));

    // No regular file of the repository has the content of a pinned file.
    let pinned: BTreeSet<&str> = pin.file.iter().map(|f| f.sha256.as_str()).collect();
    let mut files = Vec::new();
    walk(&root, &mut files);
    assert!(
        files.len() > 100,
        "the walk found only {} files",
        files.len()
    );
    for path in files {
        let digest = sha256_hex(&fs::read(&path).unwrap());
        assert!(
            !pinned.contains(digest.as_str()),
            "{} holds the content of a pinned TRACTOR file",
            path.display()
        );
    }
}

#[test]
#[ignore = "needs the fetched TRACTOR tree named by MT_LOCAL_TRACTOR"]
fn eqv_02_tractor_vectors_load_unchanged() {
    let tree = std::env::var("MT_LOCAL_TRACTOR")
        .expect("MT_LOCAL_TRACTOR must name the fetched TRACTOR tree (mt vectors tractor fetch)");
    let pin = TractorPin::load(&repo_root().join("testbed/tractor-pin.toml")).unwrap();
    let report = check_tree(&pin, Path::new(&tree)).unwrap();
    print!("{}", report.render());
    println!("tractor loaded={} of {}", report.loaded, report.total);
    assert!(report.loaded >= MIN_TRACTOR_LOADED, "{}", report.render());
    assert!(report.passed(), "{}", report.render());
}

/// Our own JSON at three pinned paths, and the pin that names their hashes.
fn own_tree(dir: &Path) -> TractorPin {
    let files = [
        (
            "Public-Tests/B/one/test_vectors/a.json",
            r#"{"stdout": {"pattern": "x"}}"#,
        ),
        (
            "Public-Tests/B/two/test_vectors/b.json",
            r#"{"lib_state_in": {"a": 1}, "lib_state_out": {"a": 2}}"#,
        ),
        (
            "Public-Tests/B/three/test_vectors/c.json",
            r#"{"argv": [], "rc": -11, "has_ub": "crash"}"#,
        ),
    ];
    let mut text = format!(
        "schema_version = 1\nrepo = \"https://example.invalid/corpus\"\ncommit = \"{COMMIT}\"\n\
         licence_status = \"unchecked; local use only until the Phase 12 licence check\"\n"
    );
    for (path, content) in files {
        let full = dir.join(path);
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(&full, content).unwrap();
        text.push_str(&format!(
            "\n[[file]]\npath = \"{path}\"\nsha256 = \"{}\"\n",
            sha256_hex(content.as_bytes())
        ));
    }
    TractorPin::parse(&text).unwrap()
}

#[test]
fn eqv_02_tractor_check_refuses_a_changed_file() {
    let tmp = tempfile::tempdir().unwrap();
    let pin = own_tree(tmp.path());
    let report = check_tree(&pin, tmp.path()).unwrap();
    assert!(report.passed(), "{}", report.render());
    assert_eq!((report.loaded, report.total), (3, 3));
    let text = report.render();
    assert_eq!(text.matches("sha256=ok").count(), 3, "{text}");
    assert_eq!(text.matches("round_trip=json-equal").count(), 3, "{text}");
    assert!(text.contains("keys=stdout"), "{text}");
    assert!(
        text.contains("has_ub=present") && text.contains("has_ub=absent"),
        "{text}"
    );
    assert!(
        text.contains("loaded: 3 of 3 (at least 3 required)"),
        "{text}"
    );

    // A changed byte fails with the path and both hashes.
    let changed = pin.file[0].path.clone();
    let victim = tmp.path().join(&changed);
    let mut bytes = fs::read(&victim).unwrap();
    bytes.push(b' ');
    fs::write(&victim, &bytes).unwrap();
    let report = check_tree(&pin, tmp.path()).unwrap();
    assert!(!report.passed());
    assert_eq!(report.loaded, 2);
    let text = report.render();
    assert!(text.contains(&changed), "{text}");
    assert!(text.contains(&pin.file[0].sha256), "{text}");
    assert!(text.contains(&sha256_hex(&bytes)), "{text}");
    assert!(
        text.contains("loaded: 2 of 3 (at least 3 required)"),
        "{text}"
    );

    // A symlink at a pinned path fails.
    let tmp = tempfile::tempdir().unwrap();
    let pin = own_tree(tmp.path());
    let target = tmp.path().join(&pin.file[1].path);
    let copy = tmp.path().join("elsewhere.json");
    fs::rename(&target, &copy).unwrap();
    std::os::unix::fs::symlink(&copy, &target).unwrap();
    let report = check_tree(&pin, tmp.path()).unwrap();
    assert!(!report.passed());
    assert!(report.render().contains("symlink"), "{}", report.render());

    // A symlinked directory above a pinned file fails.
    let tmp = tempfile::tempdir().unwrap();
    let pin = own_tree(tmp.path());
    let real = tmp.path().join("Public-Tests");
    let moved = tmp.path().join("moved");
    fs::rename(&real, &moved).unwrap();
    std::os::unix::fs::symlink(&moved, &real).unwrap();
    let report = check_tree(&pin, tmp.path()).unwrap();
    assert!(!report.passed());
    assert_eq!(report.loaded, 0);

    // A missing file fails, and a file that hashes right but is not a TRACTOR
    // vector fails to load.
    let tmp = tempfile::tempdir().unwrap();
    let pin = own_tree(tmp.path());
    fs::remove_file(tmp.path().join(&pin.file[2].path)).unwrap();
    let report = check_tree(&pin, tmp.path()).unwrap();
    assert!(!report.passed());
    assert!(report.render().contains(&pin.file[2].path));

    let tmp = tempfile::tempdir().unwrap();
    let mut pin = own_tree(tmp.path());
    let odd = r#"{"unknown_key": 1}"#;
    fs::write(tmp.path().join(&pin.file[0].path), odd).unwrap();
    pin.file[0].sha256 = sha256_hex(odd.as_bytes());
    let report = check_tree(&pin, tmp.path()).unwrap();
    assert!(!report.passed());
    assert!(
        report.render().contains("unknown_key"),
        "{}",
        report.render()
    );
}

#[test]
fn eqv_02_tractor_pin_is_validated() {
    let valid = |extra: &str| {
        format!(
            "schema_version = 1\nrepo = \"https://example.invalid/c\"\ncommit = \"{COMMIT}\"\n\
             licence_status = \"unchecked\"\n{extra}"
        )
    };
    let file = |path: &str| {
        format!(
            "[[file]]\npath = \"{path}\"\nsha256 = \"{}\"\n",
            "ab".repeat(32)
        )
    };
    let three = [
        "Public-Tests/A/a/test_vectors/1.json",
        "Public-Tests/A/a/test_vectors/2.json",
        "Public-Tests/A/a/test_vectors/3.json",
    ]
    .map(file)
    .join("\n");
    assert!(TractorPin::parse(&valid(&three)).is_ok());

    for (what, text) in [
        ("two files", valid(&three.replacen("[[file]]", "[[x]]", 1))),
        ("unknown key", valid(&format!("extra = 1\n{three}"))),
        (
            "a content key",
            valid(&format!("{three}\ncontent = \"x\"\n")),
        ),
        ("path traversal", valid(&three.replace("A/a", "../a"))),
        (
            "absolute path",
            valid(&three.replace("Public-Tests/A", "/Public-Tests/A")),
        ),
        (
            "outside Public-Tests",
            valid(&three.replace("Public-Tests/A", "Other/A")),
        ),
        ("not json", valid(&three.replace("1.json", "1.txt"))),
        ("leading dash", valid(&three.replace("A/a", "-A/a"))),
        ("glob character", valid(&three.replace("A/a", "A/*"))),
        ("duplicate path", valid(&three.replace("2.json", "1.json"))),
        (
            "short hash",
            valid(&three.replacen(&"ab".repeat(32), "abcd", 1)),
        ),
        (
            "upper-case hash",
            valid(&three.replacen(&"ab".repeat(32), &"AB".repeat(32), 1)),
        ),
        ("http url", valid(&three).replace("https://", "http://")),
        ("short commit", valid(&three).replace(COMMIT, "37960ee")),
        (
            "other schema",
            valid(&three).replace("schema_version = 1", "schema_version = 2"),
        ),
    ] {
        assert!(TractorPin::parse(&text).is_err(), "{what} must be refused");
    }
}
