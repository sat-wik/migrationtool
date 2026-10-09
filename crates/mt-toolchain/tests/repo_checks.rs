//! Repository-level guards for TOOL-01 and TOOL-04.
//!
//! These run under plain `cargo test`, with no container and no network
//! (CONTEXT D-26): no Python outside `research/`, process launching confined
//! to the runner module, `forbid(unsafe_code)` in every tool crate, and no
//! analyser linked as a library.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};

/// Extensions that mark Python source (ADR-0001: Python lives under `research/` only).
const PYTHON_EXTENSIONS: &[&str] = &["py", "pyi", "pyw", "pyx"];

/// Directories skipped, and only when they sit directly under the walk root.
const ROOT_SKIPS: &[&str] = &[".git", "target", "research"];

/// Crates whose presence in `Cargo.lock` would mean an analyser is linked as a library.
const ANALYSER_BINDINGS: &[&str] = &["clang-sys", "llvm-sys", "inkwell", "z3", "z3-sys"];

/// The repository root: two levels above this crate's manifest directory.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// Walk `root` and return the root-relative paths of Python source files.
///
/// Symlinks are never followed. Only `.git`, `target` and `research` directly
/// under `root` are skipped.
fn find_python_files(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let entry = entry.unwrap();
            let file_type = entry.file_type().unwrap();
            if file_type.is_symlink() {
                continue;
            }
            let path = entry.path();
            if file_type.is_dir() {
                let name = entry.file_name();
                let skipped =
                    dir == root && ROOT_SKIPS.contains(&name.to_str().unwrap_or_default());
                if !skipped {
                    stack.push(path);
                }
            } else if path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| PYTHON_EXTENSIONS.contains(&e))
            {
                found.push(path.strip_prefix(root).unwrap().to_path_buf());
            }
        }
    }
    found.sort();
    found
}

/// Every regular file under `dir` with the given extension, recursively, never
/// following symlinks.
fn files_with_extension(dir: &Path, extension: &str) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        for entry in fs::read_dir(&current).unwrap() {
            let entry = entry.unwrap();
            let file_type = entry.file_type().unwrap();
            if file_type.is_symlink() {
                continue;
            }
            let path = entry.path();
            if file_type.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|e| e.to_str()) == Some(extension) {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

#[test]
fn tool_01_no_python_outside_research() {
    let root = repo_root();
    let bad = find_python_files(&root);
    assert!(bad.is_empty(), "Python source outside research/: {bad:?}");
}

#[test]
fn tool_01_python_walk_flags_a_planted_file() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("a")).unwrap();
    fs::create_dir_all(dir.path().join("research")).unwrap();
    fs::write(dir.path().join("a/b.py"), "print('x')\n").unwrap();
    fs::write(dir.path().join("research/ok.py"), "print('ok')\n").unwrap();

    let found = find_python_files(dir.path());

    assert_eq!(found, vec![PathBuf::from("a/b.py")]);
}

#[test]
fn tool_01_command_only_in_runner_module() {
    let root = repo_root();
    let crates = root.join("crates");
    // The needle is assembled from two pieces so this file does not contain it.
    let needle = ["process", "::Command"].concat();
    let runner = crates.join("mt-toolchain/src/runner.rs");

    let offenders: Vec<PathBuf> = files_with_extension(&crates, "rs")
        .into_iter()
        .filter(|path| path != &runner)
        .filter(|path| fs::read_to_string(path).unwrap().contains(&needle))
        .collect();

    assert!(
        offenders.is_empty(),
        "{needle} used outside crates/mt-toolchain/src/runner.rs: {offenders:?}"
    );
}

#[test]
fn tool_01_every_tool_crate_forbids_unsafe() {
    let crates = repo_root().join("crates");
    let mut roots = Vec::new();
    for entry in fs::read_dir(&crates).unwrap() {
        let src = entry.unwrap().path().join("src");
        for name in ["lib.rs", "main.rs"] {
            let candidate = src.join(name);
            if candidate.is_file() {
                roots.push(candidate);
            }
        }
    }
    assert!(!roots.is_empty(), "no crate roots found under crates/");

    let missing: Vec<&PathBuf> = roots
        .iter()
        .filter(|path| {
            !fs::read_to_string(path)
                .unwrap()
                .contains("#![forbid(unsafe_code)]")
        })
        .collect();

    assert!(
        missing.is_empty(),
        "crate roots without #![forbid(unsafe_code)]: {missing:?}"
    );
}

#[test]
fn tool_01_never_links_an_analyser_library() {
    let lock = fs::read_to_string(repo_root().join("Cargo.lock")).unwrap();
    let lock: toml::Table = lock.parse().unwrap();
    let packages = lock["package"].as_array().unwrap();
    assert!(!packages.is_empty(), "Cargo.lock lists no packages");

    let linked: Vec<&str> = packages
        .iter()
        .filter_map(|package| package.get("name").and_then(|name| name.as_str()))
        .filter(|name| ANALYSER_BINDINGS.contains(name))
        .collect();

    assert!(
        linked.is_empty(),
        "analyser binding crates in Cargo.lock (analysers run as subprocesses only): {linked:?}"
    );
}
