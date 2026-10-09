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

/// The lint table every emitted crate carries (docs/specs/emitted-rust-rules.md section 6).
const EMITTED_LINTS: &str = r#"
[rust]
unsafe_code = "deny"
unsafe_op_in_unsafe_fn = "deny"
missing_docs = "deny"

[clippy]
all = { level = "deny", priority = -1 }
undocumented_unsafe_blocks = "deny"
arithmetic_side_effects = "deny"
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
todo = "deny"
unimplemented = "deny"
unreachable = "deny"
std_instead_of_core = "deny"
alloc_instead_of_core = "deny"
indexing_slicing = "warn"
cast_possible_truncation = "warn"
cast_sign_loss = "warn"
cast_possible_wrap = "warn"
"#;

/// Read and parse a TOML manifest, failing with a clear message when it is absent.
fn read_manifest(path: &Path) -> toml::Table {
    assert!(path.is_file(), "{} is missing", path.display());
    fs::read_to_string(path).unwrap().parse().unwrap()
}

/// The lines of `source` that are neither blank nor line comments.
fn code_lines(source: &str) -> Vec<&str> {
    source
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("//"))
        .collect()
}

#[test]
fn tool_04_smoke_crate_has_emitted_rust_shape() {
    let smoke = repo_root().join("container/smoke");
    let manifest = read_manifest(&smoke.join("Cargo.toml"));

    let crate_types: Vec<&str> = manifest["lib"]["crate-type"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect();
    assert_eq!(crate_types, ["staticlib", "rlib"]);

    assert_eq!(manifest["package"]["edition"].as_str(), Some("2021"));
    let dependencies = manifest["dependencies"].as_table().unwrap();
    assert!(dependencies.is_empty(), "smoke crate has dependencies");
    for profile in ["dev", "release"] {
        assert_eq!(
            manifest["profile"][profile]["panic"].as_str(),
            Some("abort"),
            "profile.{profile}.panic"
        );
    }
    let default_features: Vec<&str> = manifest["features"]["default"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect();
    assert_eq!(default_features, ["panic-handler"]);

    let expected_lints: toml::Table = EMITTED_LINTS.parse().unwrap();
    assert_eq!(
        manifest["lints"].as_table().unwrap(),
        &expected_lints,
        "lint table differs from docs/specs/emitted-rust-rules.md section 6"
    );

    let lib_rs = fs::read_to_string(smoke.join("src/lib.rs")).unwrap();
    assert_eq!(code_lines(&lib_rs).first().copied(), Some("#![no_std]"));

    let lines: Vec<&str> = lib_rs.lines().map(str::trim).collect();
    let handlers: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| **line == "#[panic_handler]")
        .map(|(index, _)| index)
        .collect();
    assert_eq!(handlers.len(), 1, "expected exactly one #[panic_handler]");
    assert_eq!(
        lines[handlers[0] - 1],
        "#[cfg(feature = \"panic-handler\")]",
        "#[panic_handler] must sit directly under the panic-handler feature gate"
    );

    let with_unsafe: Vec<String> = files_with_extension(&smoke.join("src"), "rs")
        .into_iter()
        .filter(|path| fs::read_to_string(path).unwrap().contains("unsafe"))
        .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(with_unsafe, ["ffi.rs"], "unsafe is allowed only in ffi.rs");
}

#[test]
fn tool_04_smoke_crate_is_outside_the_workspace() {
    let root = repo_root();
    let smoke = read_manifest(&root.join("container/smoke/Cargo.toml"));
    assert!(
        smoke.get("workspace").is_some_and(toml::Value::is_table),
        "smoke Cargo.toml needs its own [workspace] table"
    );

    let workspace = read_manifest(&root.join("Cargo.toml"));
    let strings = |key: &str| -> Vec<String> {
        workspace["workspace"][key]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap().to_owned())
            .collect()
    };
    assert!(strings("exclude").contains(&"container/smoke".to_owned()));
    assert!(!strings("members").contains(&"container/smoke".to_owned()));
}
