#![allow(clippy::unwrap_used, clippy::expect_used)]
#![cfg(unix)]
//! EQV-02, D-23, T-03-09: the sparse fetch of the pinned TRACTOR sample.
//!
//! The upstream is a local repository made through the runner and served over
//! `file://`, so nothing here touches the network. The protocol allow-list is
//! widened to `file` for these tests only.

mod support;

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use mt_capture::fetch::{FetchError, fetch_sparse, verify_head};
use support::{GIT, fetch_context, make_upstream, runner_config, tool};

const UNKNOWN_COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

/// The files under `dir` outside `.git`, as relative paths.
fn tree_files(dir: &Path) -> BTreeSet<String> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeSet<String>) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if entry.file_name() == ".git" {
                continue;
            }
            if entry.file_type().unwrap().is_dir() {
                walk(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
    }
    let mut out = BTreeSet::new();
    walk(dir, dir, &mut out);
    out
}

fn upstream() -> support::Upstream {
    let up = make_upstream(&[
        ("keep/a.json", "{\"a\": 1}\n"),
        ("keep/b.json", "{\"b\": 2}\n"),
        ("skip/c.json", "{\"c\": 3}\n"),
        ("skip/d.json", "{\"d\": 4}\n"),
    ]);
    let repo = up.url.strip_prefix("file://").unwrap().to_owned();
    // A blobless fetch needs the server to allow filters.
    tool(
        &[GIT, "-C", &repo, "config", "uploadpack.allowFilter", "true"],
        Path::new("/"),
        &[
            ("GIT_CONFIG_GLOBAL", "/dev/null"),
            ("GIT_CONFIG_NOSYSTEM", "1"),
        ],
    );
    up
}

#[test]
fn eqv_02_sparse_fetch_checks_commit_and_paths() {
    let up = upstream();
    let work = tempfile::tempdir().unwrap();
    let ctx = fetch_context(work.path());
    let dest = work.path().join("tractor").join(&up.commit).join("tree");
    let paths = vec!["keep/a.json".to_owned(), "keep/b.json".to_owned()];

    let steps = fetch_sparse(&runner_config(), &ctx, &up.url, &up.commit, &paths, &dest).unwrap();
    let names: Vec<&str> = steps.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "git init",
            "git fetch",
            "git sparse-checkout",
            "git checkout",
            "git rev-parse"
        ]
    );

    // Exactly the two requested files, at the pinned commit.
    assert_eq!(
        tree_files(&dest),
        BTreeSet::from(["keep/a.json".to_owned(), "keep/b.json".to_owned()])
    );
    assert_eq!(
        fs::read_to_string(dest.join("keep/a.json")).unwrap(),
        "{\"a\": 1}\n"
    );
    let head = tool(
        &[GIT, "-C", dest.to_str().unwrap(), "rev-parse", "HEAD"],
        work.path(),
        &[
            ("GIT_CONFIG_GLOBAL", "/dev/null"),
            ("GIT_CONFIG_NOSYSTEM", "1"),
        ],
    );
    assert_eq!(String::from_utf8(head.stdout).unwrap().trim(), up.commit);
    let partial = dest.with_file_name("tree.partial");
    assert!(!partial.exists(), "no partial tree is left behind");

    // The destination is created fresh: an existing tree is left alone.
    let again = fetch_sparse(&runner_config(), &ctx, &up.url, &up.commit, &paths, &dest);
    assert!(
        matches!(again, Err(FetchError::InvalidInput(_))),
        "{again:?}"
    );
    assert_eq!(tree_files(&dest).len(), 2);

    // A commit the upstream does not have fails and leaves nothing behind.
    let other = work.path().join("other/tree");
    let missing = fetch_sparse(
        &runner_config(),
        &ctx,
        &up.url,
        UNKNOWN_COMMIT,
        &paths,
        &other,
    );
    assert!(
        matches!(missing, Err(FetchError::Failed { .. })),
        "{missing:?}"
    );
    assert!(!other.exists());
    assert!(!other.with_file_name("tree.partial").exists());

    // The head must be the commit that was asked for.
    assert!(verify_head(&up.commit, format!("{}\n", up.commit).as_bytes()).is_ok());
    match verify_head(&up.commit, format!("{UNKNOWN_COMMIT}\n").as_bytes()) {
        Err(FetchError::CommitMismatch { expected, found }) => {
            assert_eq!(expected, up.commit);
            assert_eq!(found, UNKNOWN_COMMIT);
        }
        other => panic!("expected CommitMismatch, got {other:?}"),
    }
}

#[test]
fn eqv_02_sparse_fetch_rejects_unsafe_input() {
    let up = upstream();
    let work = tempfile::tempdir().unwrap();
    let ctx = fetch_context(work.path());
    let good = vec!["keep/a.json".to_owned()];
    let dest = |name: &str| work.path().join(name).join("tree");
    let run = |url: &str, commit: &str, paths: &[String], name: &str| {
        fetch_sparse(&runner_config(), &ctx, url, commit, paths, &dest(name))
    };

    for (what, paths) in [
        ("no paths", vec![]),
        ("parent component", vec!["keep/../skip/c.json".to_owned()]),
        ("absolute path", vec!["/keep/a.json".to_owned()]),
        ("leading dash", vec!["-keep/a.json".to_owned()]),
        ("glob", vec!["keep/*.json".to_owned()]),
        ("negation", vec!["!keep/a.json".to_owned()]),
        ("empty component", vec!["keep//a.json".to_owned()]),
        ("hidden component", vec![".git/config".to_owned()]),
        ("space", vec!["keep/a b.json".to_owned()]),
    ] {
        let result = run(&up.url, &up.commit, &paths, "bad-path");
        assert!(
            matches!(result, Err(FetchError::InvalidInput(_))),
            "{what}: {result:?}"
        );
    }
    for (what, commit) in [
        ("short", "37960ee"),
        ("upper-case", "0123456789ABCDEF0123456789abcdef01234567"),
        ("a ref name", "main"),
    ] {
        let result = run(&up.url, commit, &good, "bad-commit");
        assert!(
            matches!(result, Err(FetchError::InvalidInput(_))),
            "{what}: {result:?}"
        );
    }
    for url in [
        "http://example.invalid/x",
        "ftp://example.invalid/x",
        "/local/path",
        "https://a b",
    ] {
        let result = run(url, &up.commit, &good, "bad-url");
        assert!(
            matches!(result, Err(FetchError::InvalidInput(_))),
            "{url}: {result:?}"
        );
    }
    assert!(
        !dest("bad-path").exists() && !dest("bad-commit").exists() && !dest("bad-url").exists()
    );
}
