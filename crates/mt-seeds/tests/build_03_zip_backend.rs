//! BUILD-03 (G-02-4, WR-07): which backend inflates zip seed archives, and the
//! per-archive decompressed budget.
//!
//! The lockfile and manifest checks read the two files as text on purpose:
//! naming the `toml` crate here would add a dependency edge to Cargo.lock that
//! the package legitimacy gate (plan 02-13, Task 1) did not approve.
//!
//! What is and is not safe Rust, stated exactly: the inflater (miniz_oxide) and
//! adler2 forbid unsafe code; simd-adler32 (SIMD Adler-32 checksum, with unsafe
//! code in its SIMD intrinsics; not checked whether zip's raw-deflate path calls
//! it), zip's header parsing and flate2's buffer code contain unsafe code; there
//! is no C code in the zip stack.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::fs;
use std::io::{Cursor, Write};
use std::path::PathBuf;

use mt_seeds::zipsrc::zip_members;

/// The workspace root, two levels above this crate.
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

/// Cargo.lock as package name -> names in its `dependencies` list.
///
/// Cargo.lock is a flat sequence of `[[package]]` tables whose `dependencies`
/// entries are `"name"` or `"name version"`; only the name is kept.
fn lock_packages() -> BTreeMap<String, Vec<String>> {
    let text = fs::read_to_string(workspace_root().join("Cargo.lock")).unwrap();
    let mut packages: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut current: Option<String> = None;
    let mut in_dependencies = false;
    for line in text.lines() {
        if line == "[[package]]" {
            current = None;
            in_dependencies = false;
        } else if let Some(name) = line
            .strip_prefix("name = \"")
            .and_then(|rest| rest.strip_suffix('"'))
        {
            packages.entry(name.to_owned()).or_default();
            current = Some(name.to_owned());
        } else if line == "dependencies = [" {
            in_dependencies = true;
        } else if line == "]" {
            in_dependencies = false;
        } else if in_dependencies {
            let entry = line
                .trim()
                .trim_end_matches(',')
                .trim_matches('"')
                .split(' ')
                .next()
                .unwrap_or("");
            if let Some(name) = &current {
                packages
                    .entry(name.clone())
                    .or_default()
                    .push(entry.to_owned());
            }
        }
    }
    packages
}

/// The `[workspace.dependencies]` line that starts with `<name> = `.
fn workspace_dependency_line(name: &str) -> Option<String> {
    let text = fs::read_to_string(workspace_root().join("Cargo.toml")).unwrap();
    let prefix = format!("{name} = ");
    text.lines()
        .find(|line| line.starts_with(&prefix))
        .map(str::to_owned)
}

/// The quoted items of `features = [ ... ]` on one dependency line.
fn features_of(line: &str) -> Vec<String> {
    let after = line.split("features = [").nth(1).unwrap_or("");
    let list = after.split(']').next().unwrap_or("");
    list.split(',')
        .map(|item| item.trim().trim_matches('"').to_owned())
        .filter(|item| !item.is_empty())
        .collect()
}

#[test]
fn build_03_zip_inflates_with_miniz_oxide_only() {
    let packages = lock_packages();

    // The packages the founder approved at the legitimacy checkpoint.
    for wanted in ["miniz_oxide", "adler2", "simd-adler32"] {
        assert!(
            packages.contains_key(wanted),
            "Cargo.lock must hold {wanted}"
        );
    }
    let flate2 = packages.get("flate2").expect("flate2 is locked");
    assert!(
        flate2.iter().any(|dep| dep == "miniz_oxide"),
        "flate2 must depend on miniz_oxide, has {flate2:?}"
    );

    // No other zlib backend: not the Rust port of zlib, not a C library.
    for banned in [
        "zlib-rs",
        "libz-rs-sys",
        "libz-sys",
        "libz-ng-sys",
        "cloudflare-zlib-sys",
    ] {
        assert!(
            !packages.contains_key(banned),
            "Cargo.lock must not hold {banned}"
        );
    }

    // The manifest asks for exactly that: zip with the backend-less deflate
    // feature, and flate2 named once to select its rust_backend.
    let zip = workspace_dependency_line("zip").expect("workspace zip dependency");
    assert_eq!(features_of(&zip), ["deflate-flate2"], "zip line: {zip}");
    assert!(zip.contains("default-features = false"), "zip line: {zip}");
    let flate2 = workspace_dependency_line("flate2").expect("workspace flate2 dependency");
    assert_eq!(
        features_of(&flate2),
        ["rust_backend"],
        "flate2 line: {flate2}"
    );
    assert!(
        flate2.contains("default-features = false"),
        "flate2 line: {flate2}"
    );
}

/// An archive with the given `(name, bytes, deflated)` members.
fn archive_of(members: &[(&str, &[u8], bool)]) -> Vec<u8> {
    use zip::write::SimpleFileOptions;
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
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

/// Bytes that compress well but are not a single repeated byte.
fn compressible(len: usize) -> Vec<u8> {
    (0..len)
        .map(|i| u8::try_from((i / 7 + i % 5) % 251).unwrap())
        .collect()
}

#[test]
fn build_03_zip_deflated_members_read_with_the_rust_backend() {
    let deflated = compressible(20_000);
    let stored = b"stored member bytes".to_vec();
    let archive = archive_of(&[
        ("deflated.bin", &deflated, true),
        ("stored.bin", &stored, false),
    ]);
    assert!(
        archive.len() < deflated.len(),
        "the Deflated member must really be compressed ({} archive bytes)",
        archive.len()
    );

    let result = zip_members(&archive, 1 << 20).unwrap();
    assert_eq!(result.unsupported, 0);
    assert_eq!(result.too_large, 0);
    let names: Vec<&str> = result.members.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, ["deflated.bin", "stored.bin"]);
    assert_eq!(result.members[0].bytes, deflated);
    assert_eq!(result.members[1].bytes, stored);
}
