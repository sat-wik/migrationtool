#![allow(clippy::unwrap_used, clippy::expect_used)]
#![cfg(unix)]
//! BED-01 fetch: a source arrives only by pinned commit or pinned archive, and
//! every reuse of the cache is verified again (CONTEXT D-17, D-18).
//!
//! The upstream is a local repository (or tarball) made through the runner and
//! served over `file://`, so nothing here touches the network. The git
//! protocol allow-list is widened to `file` for these tests only; a manifest
//! URL is always `https://`.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use mt_capture::fetch::{FetchContext, FetchError, cache_paths, fetch, verify_cached};
use mt_capture::manifest::{BuildSystem, Category, Licence, Source, Target};
use mt_capture::report::FetchReport;
use mt_capture::treehash::{sha256_hex, tree_sha256};
use mt_toolchain::runner::{RunOutput, RunRequest, RunnerConfig, run};
use tempfile::TempDir;

const GIT: &str = "/usr/bin/git";
const CURL: &str = "/usr/bin/curl";
const TAR: &str = "/usr/bin/tar";
const WRONG_HASH: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const UNKNOWN_COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

fn runner_config() -> RunnerConfig {
    RunnerConfig::new("/usr/bin:/bin", "1791158400")
}

fn tool(argv: &[&str], cwd: &Path, env: &[(&str, &str)]) -> RunOutput {
    let request = RunRequest {
        tool_name: "test".to_owned(),
        argv: argv.iter().map(OsString::from).collect(),
        cwd: cwd.to_path_buf(),
        extra_env: env
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect::<BTreeMap<_, _>>(),
        tool_version: None,
    };
    let output = run(&runner_config(), &request).unwrap();
    assert_eq!(
        output.record.exit.code,
        Some(0),
        "{argv:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn context(work: &Path) -> FetchContext {
    FetchContext {
        work_dir: work.to_path_buf(),
        git: PathBuf::from(GIT),
        curl: PathBuf::from(CURL),
        tar: PathBuf::from(TAR),
        allowed_protocols: "file".to_owned(),
        timeout: Duration::from_secs(120),
    }
}

fn target_with(source: Source) -> Target {
    Target {
        category: vec![Category::Tlv],
        build: BuildSystem::Make,
        source,
        licence: Licence {
            spdx: "MIT".to_owned(),
            elected: "MIT".to_owned(),
            file: "LICENSE".to_owned(),
            note: None,
        },
        build_args: None,
        entry: Vec::new(),
        config: Vec::new(),
    }
}

/// A local git repository with one commit, reachable by SHA.
struct Upstream {
    _tmp: TempDir,
    url: String,
    commit: String,
    tree_sha256: String,
}

fn write_sources(dir: &Path) {
    fs::create_dir_all(dir.join("include")).unwrap();
    fs::write(dir.join("lib.c"), "int lib(void) { return 1; }\n").unwrap();
    fs::write(dir.join("include/lib.h"), "int lib(void);\n").unwrap();
    fs::write(dir.join("run.sh"), "#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(dir.join("run.sh"), fs::Permissions::from_mode(0o755)).unwrap();
}

fn make_upstream() -> Upstream {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("upstream");
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    write_sources(&repo);
    let home_text = home.to_str().unwrap();
    let env = [
        ("HOME", home_text),
        ("GIT_CONFIG_GLOBAL", "/dev/null"),
        ("GIT_CONFIG_NOSYSTEM", "1"),
        ("GIT_AUTHOR_NAME", "Fixture"),
        ("GIT_AUTHOR_EMAIL", "fixture@example.invalid"),
        ("GIT_COMMITTER_NAME", "Fixture"),
        ("GIT_COMMITTER_EMAIL", "fixture@example.invalid"),
        ("GIT_AUTHOR_DATE", "2026-01-01T00:00:00Z"),
        ("GIT_COMMITTER_DATE", "2026-01-01T00:00:00Z"),
    ];
    tool(
        &[GIT, "init", "--quiet", repo.to_str().unwrap()],
        tmp.path(),
        &env,
    );
    tool(
        &[
            GIT,
            "-C",
            repo.to_str().unwrap(),
            "config",
            "uploadpack.allowAnySHA1InWant",
            "true",
        ],
        tmp.path(),
        &env,
    );
    tool(
        &[GIT, "-C", repo.to_str().unwrap(), "add", "-A"],
        tmp.path(),
        &env,
    );
    tool(
        &[
            GIT,
            "-C",
            repo.to_str().unwrap(),
            "commit",
            "--quiet",
            "-m",
            "fixture",
        ],
        tmp.path(),
        &env,
    );
    let head = tool(
        &[GIT, "-C", repo.to_str().unwrap(), "rev-parse", "HEAD"],
        tmp.path(),
        &env,
    );
    let commit = String::from_utf8(head.stdout).unwrap().trim().to_owned();
    // The tree hash skips a top-level `.git`, so it is the hash of the sources.
    let tree_sha256 = tree_sha256(&repo).unwrap();
    Upstream {
        url: format!("file://{}", repo.display()),
        _tmp: tmp,
        commit,
        tree_sha256,
    }
}

fn git_target(url: &str, commit: &str, tree: &str) -> Target {
    target_with(Source::Git {
        url: url.to_owned(),
        reference: "main".to_owned(),
        commit: commit.to_owned(),
        tree_sha256: tree.to_owned(),
    })
}

#[test]
fn bed_01_fetch_pins_commit_and_tree_hash() {
    let upstream = make_upstream();
    let work = tempfile::tempdir().unwrap();
    let ctx = context(work.path());
    let target = git_target(&upstream.url, &upstream.commit, &upstream.tree_sha256);

    let first = fetch(&runner_config(), &ctx, "demo", &target).unwrap();

    assert!(!first.reused);
    assert_eq!(first.kind, "fetch");
    assert_eq!(first.source_kind, "git");
    assert_eq!(first.commit, upstream.commit);
    assert_eq!(first.tree_sha256, upstream.tree_sha256);
    assert_eq!(first.archive_sha256, None);
    let paths = cache_paths(work.path(), "demo", &upstream.commit);
    assert_eq!(
        paths.tree,
        work.path()
            .join("cache/demo")
            .join(&upstream.commit)
            .join("tree")
    );
    assert!(paths.tree.join("lib.c").is_file());
    assert_eq!(tree_sha256(&paths.tree).unwrap(), upstream.tree_sha256);
    let saved: FetchReport =
        serde_json::from_str(&fs::read_to_string(&paths.report).unwrap()).unwrap();
    assert_eq!(saved, first);

    // Four git steps, each through the pinned absolute path with the hardened env.
    let names: Vec<&str> = first.steps.iter().map(|step| step.name.as_str()).collect();
    assert_eq!(
        names,
        ["git init", "git fetch", "git checkout", "git rev-parse"]
    );
    for step in &first.steps {
        assert_eq!(step.run.argv[0], GIT);
        assert_eq!(step.run.env["GIT_TERMINAL_PROMPT"], "0");
        assert_eq!(step.run.env["GIT_CONFIG_NOSYSTEM"], "1");
        assert_eq!(step.run.env["GIT_CONFIG_GLOBAL"], "/dev/null");
        assert_eq!(step.run.env["GIT_ALLOW_PROTOCOL"], "file");
        assert_eq!(
            step.run.env["HOME"],
            work.path().join("home").to_str().unwrap()
        );
        assert_eq!(
            step.run.env["TMPDIR"],
            work.path().join("tmp").to_str().unwrap()
        );
    }
    let fetch_step = &first.steps[1];
    for flag in ["--depth", "1", "--no-tags", "--no-recurse-submodules"] {
        assert!(
            fetch_step.run.argv.iter().any(|a| a == flag),
            "missing {flag}"
        );
    }
    assert!(fetch_step.run.argv.contains(&upstream.commit));

    let second = fetch(&runner_config(), &ctx, "demo", &target).unwrap();
    assert!(second.reused);
    assert_eq!(second.tree_sha256, first.tree_sha256);
    assert_eq!(second.commit, first.commit);
}

#[test]
fn bed_01_fetch_rejects_tree_hash_mismatch() {
    let upstream = make_upstream();
    let work = tempfile::tempdir().unwrap();
    let ctx = context(work.path());
    let target = git_target(&upstream.url, &upstream.commit, WRONG_HASH);

    let error = fetch(&runner_config(), &ctx, "demo", &target).unwrap_err();

    assert!(
        matches!(error, FetchError::TreeHashMismatch { .. }),
        "{error:?}"
    );
    let text = error.to_string();
    assert!(text.contains(WRONG_HASH), "{text}");
    assert!(text.contains(&upstream.tree_sha256), "{text}");
    let paths = cache_paths(work.path(), "demo", &upstream.commit);
    assert!(
        !paths.tree.exists(),
        "a rejected tree must not be left in the cache"
    );
    assert!(!paths.dir.join("tree.partial").exists());
    assert!(!paths.report.exists());
}

#[test]
fn bed_01_fetch_rejects_unknown_commit() {
    let upstream = make_upstream();
    let work = tempfile::tempdir().unwrap();
    let ctx = context(work.path());
    let target = git_target(&upstream.url, UNKNOWN_COMMIT, &upstream.tree_sha256);

    let error = fetch(&runner_config(), &ctx, "demo", &target).unwrap_err();

    assert!(matches!(error, FetchError::Failed { .. }), "{error:?}");
    let paths = cache_paths(work.path(), "demo", UNKNOWN_COMMIT);
    assert!(!paths.tree.exists());
}

#[test]
fn bed_01_fetch_reuse_verifies_cache() {
    let upstream = make_upstream();
    let work = tempfile::tempdir().unwrap();
    let ctx = context(work.path());
    let target = git_target(&upstream.url, &upstream.commit, &upstream.tree_sha256);
    fetch(&runner_config(), &ctx, "demo", &target).unwrap();
    let paths = cache_paths(work.path(), "demo", &upstream.commit);

    // verify_cached passes on the untouched cache and needs no network.
    let verified = verify_cached(&runner_config(), &ctx, "demo", &target).unwrap();
    assert!(verified.reused);

    // One edited byte inside the cached tree.
    fs::write(paths.tree.join("lib.c"), "int lib(void) { return 2; }\n").unwrap();
    let error = fetch(&runner_config(), &ctx, "demo", &target).unwrap_err();
    assert!(
        matches!(error, FetchError::CacheMismatch { .. }),
        "{error:?}"
    );
    assert!(
        error.to_string().contains(paths.tree.to_str().unwrap()),
        "the error must name the cache path: {error}"
    );
    // It is not silently replaced: the edit is still there.
    assert_eq!(
        fs::read_to_string(paths.tree.join("lib.c")).unwrap(),
        "int lib(void) { return 2; }\n"
    );
    assert!(verify_cached(&runner_config(), &ctx, "demo", &target).is_err());

    // A planted extra file is a mismatch as well.
    fs::write(paths.tree.join("lib.c"), "int lib(void) { return 1; }\n").unwrap();
    fetch(&runner_config(), &ctx, "demo", &target).unwrap();
    fs::write(paths.tree.join("extra.c"), "\n").unwrap();
    assert!(matches!(
        fetch(&runner_config(), &ctx, "demo", &target).unwrap_err(),
        FetchError::CacheMismatch { .. }
    ));
}

#[test]
fn bed_01_fetch_reuse_rejects_a_moved_head() {
    let upstream = make_upstream();
    let work = tempfile::tempdir().unwrap();
    let ctx = context(work.path());
    let target = git_target(&upstream.url, &upstream.commit, &upstream.tree_sha256);
    fetch(&runner_config(), &ctx, "demo", &target).unwrap();
    let paths = cache_paths(work.path(), "demo", &upstream.commit);

    // The same tree contents under another commit: HEAD no longer equals the pin.
    fs::write(paths.tree.join(".git/HEAD"), format!("{UNKNOWN_COMMIT}\n")).unwrap();
    let error = fetch(&runner_config(), &ctx, "demo", &target).unwrap_err();

    assert!(
        matches!(error, FetchError::CacheMismatch { .. }),
        "{error:?}"
    );
    assert!(error.to_string().contains(paths.tree.to_str().unwrap()));
}

fn make_archive(root: &Path) -> (PathBuf, String) {
    let package = root.join("pkg-1.0");
    fs::create_dir_all(package.join("src")).unwrap();
    fs::write(package.join("src/a.c"), "int a(void) { return 3; }\n").unwrap();
    fs::write(package.join("b.h"), "#define B 4\n").unwrap();
    let tree = tree_sha256(&package).unwrap();
    let archive = root.join("pkg-1.0.tar.bz2");
    tool(
        &[
            TAR,
            "-cjf",
            archive.to_str().unwrap(),
            "-C",
            root.to_str().unwrap(),
            "pkg-1.0",
        ],
        root,
        &[],
    );
    (archive, tree)
}

fn archive_target(archive: &Path, sha256: &str, tree: &str) -> Target {
    target_with(Source::Archive {
        url: format!("file://{}", archive.display()),
        sha256: sha256.to_owned(),
        strip_components: 1,
        reference: "v1.0".to_owned(),
        commit: UNKNOWN_COMMIT.to_owned(),
        tree_sha256: tree.to_owned(),
    })
}

#[test]
fn bed_01_archive_fetch_checks_archive_sha256() {
    let upstream = tempfile::tempdir().unwrap();
    let (archive, tree) = make_archive(upstream.path());
    let archive_sha = sha256_hex(&fs::read(&archive).unwrap());
    let work = tempfile::tempdir().unwrap();
    let ctx = context(work.path());

    // The right digest extracts, with the leading component stripped.
    let good = archive_target(&archive, &archive_sha, &tree);
    let report = fetch(&runner_config(), &ctx, "pkg", &good).unwrap();
    assert!(!report.reused);
    assert_eq!(report.source_kind, "archive");
    assert_eq!(report.archive_sha256.as_deref(), Some(archive_sha.as_str()));
    assert_eq!(report.tree_sha256, tree);
    let paths = cache_paths(work.path(), "pkg", UNKNOWN_COMMIT);
    assert!(paths.tree.join("src/a.c").is_file());
    assert!(paths.archive.is_file());
    let curl_step = &report.steps[0];
    assert_eq!(curl_step.run.argv[0], CURL);
    for flag in [
        "-q",
        "--globoff",
        "--fail",
        "--proto",
        "=https,file",
        "--proto-redir",
    ] {
        assert!(
            curl_step.run.argv.iter().any(|a| a == flag),
            "curl lacks {flag}"
        );
    }
    let tar_step = report.steps.last().unwrap();
    assert_eq!(tar_step.run.argv[0], TAR);
    for flag in ["-xjf", "--strip-components=1", "--no-same-owner"] {
        assert!(
            tar_step.run.argv.iter().any(|a| a == flag),
            "tar lacks {flag}"
        );
    }

    // Reuse verifies the archive digest and the tree again.
    assert!(fetch(&runner_config(), &ctx, "pkg", &good).unwrap().reused);
    let mut bytes = fs::read(&paths.archive).unwrap();
    bytes.push(0);
    fs::write(&paths.archive, bytes).unwrap();
    let error = fetch(&runner_config(), &ctx, "pkg", &good).unwrap_err();
    assert!(
        matches!(error, FetchError::CacheMismatch { .. }),
        "{error:?}"
    );
    assert!(error.to_string().contains(paths.archive.to_str().unwrap()));

    // A wrong archive digest fails before anything is extracted.
    let other_work = tempfile::tempdir().unwrap();
    let other_ctx = context(other_work.path());
    let bad = archive_target(&archive, WRONG_HASH, &tree);
    let error = fetch(&runner_config(), &other_ctx, "pkg", &bad).unwrap_err();
    assert!(
        matches!(error, FetchError::ArchiveMismatch { .. }),
        "{error:?}"
    );
    let text = error.to_string();
    assert!(
        text.contains(WRONG_HASH) && text.contains(&archive_sha),
        "{text}"
    );
    let bad_paths = cache_paths(other_work.path(), "pkg", UNKNOWN_COMMIT);
    assert!(!bad_paths.tree.exists());
    assert!(
        !bad_paths.dir.join("tree.partial").join("src").exists(),
        "extracted before checking"
    );
    assert!(
        !bad_paths.archive.exists(),
        "a rejected archive must not stay in the cache"
    );
}

#[test]
fn bed_01_archive_fetch_rejects_a_wrong_tree_hash() {
    let upstream = tempfile::tempdir().unwrap();
    let (archive, _) = make_archive(upstream.path());
    let archive_sha = sha256_hex(&fs::read(&archive).unwrap());
    let work = tempfile::tempdir().unwrap();
    let ctx = context(work.path());
    let target = archive_target(&archive, &archive_sha, WRONG_HASH);

    let error = fetch(&runner_config(), &ctx, "pkg", &target).unwrap_err();

    assert!(
        matches!(error, FetchError::TreeHashMismatch { .. }),
        "{error:?}"
    );
    assert!(
        !cache_paths(work.path(), "pkg", UNKNOWN_COMMIT)
            .tree
            .exists()
    );
}

#[test]
fn bed_01_fetch_refuses_identifiers_that_leave_the_cache() {
    let upstream = make_upstream();
    let work = tempfile::tempdir().unwrap();
    let ctx = context(work.path());
    let target = git_target(&upstream.url, &upstream.commit, &upstream.tree_sha256);

    for id in ["", ".", "..", "a/b", "../escape"] {
        let error = fetch(&runner_config(), &ctx, id, &target).unwrap_err();
        assert!(
            matches!(error, FetchError::InvalidInput(_)),
            "{id:?}: {error:?}"
        );
    }
    let bad_commit = git_target(&upstream.url, "../../etc", &upstream.tree_sha256);
    assert!(matches!(
        fetch(&runner_config(), &ctx, "demo", &bad_commit).unwrap_err(),
        FetchError::InvalidInput(_)
    ));
    let option_url = git_target(
        "--upload-pack=touch /tmp/x",
        &upstream.commit,
        &upstream.tree_sha256,
    );
    assert!(matches!(
        fetch(&runner_config(), &ctx, "demo", &option_url).unwrap_err(),
        FetchError::InvalidInput(_)
    ));
    assert!(
        !work.path().join("cache").exists()
            || fs::read_dir(work.path().join("cache"))
                .unwrap()
                .next()
                .is_none()
    );
}

#[test]
fn bed_01_fetch_writes_only_under_the_work_directory() {
    let upstream = make_upstream();
    let work = tempfile::tempdir().unwrap();
    let ctx = context(work.path());
    let target = git_target(&upstream.url, &upstream.commit, &upstream.tree_sha256);
    let paths = cache_paths(work.path(), "demo", &upstream.commit);

    for path in [&paths.dir, &paths.tree, &paths.archive, &paths.report] {
        assert!(
            path.starts_with(work.path()),
            "{} is outside the work directory",
            path.display()
        );
    }
    fetch(&runner_config(), &ctx, "demo", &target).unwrap();

    // The default work directory is git-ignored, so fetched trees can never be tracked.
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let ignore = fs::read_to_string(root.join(".gitignore")).unwrap();
    assert_eq!(
        ignore.lines().filter(|line| *line == "/.mt-work/").count(),
        1
    );
}
