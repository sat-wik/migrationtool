#![allow(clippy::unwrap_used, clippy::expect_used)]
//! BUILD-01 toolchain evidence (review warnings WR-12 and WR-14, plan 02-22).
//!
//! What proves the toolchain file a CMake configure read is that the build
//! cannot write it: in the testbed workflow it is written by a step that runs no
//! upstream code and mounted read-only (see build_01_capture_stages.rs and
//! build_02_workflow_static.rs). The checks here are of two other kinds:
//!
//! - the mapping of a toolchain-file read to a cause (WR-14: an unreadable file
//!   is a cause on its pair, never an aborted step);
//! - the consistency checks over `build/CMakeCache.txt` and the compile
//!   commands. They are consistency checks only: the build can write the cache,
//!   and the flag check cannot see flags that were added.
//!
//! The pure tests run everywhere. The ignored test reads the captures of an
//! earlier local run and must be run explicitly.

use std::io;
use std::path::{Path, PathBuf};

use mt_capture::capture::{RunReadError, configured_toolchain_findings, toolchain_file_finding};
use mt_capture::cc::{CcEntry, CcPolicy, canonicalise, parse_raw};
use mt_capture::treehash::sha256_hex;

fn flags() -> Vec<String> {
    [
        "-mcpu=cortex-m4",
        "-mthumb",
        "-mfloat-abi=hard",
        "-mfpu=fpv4-sp-d16",
        "-Os",
        "-DX=1",
    ]
    .iter()
    .map(|flag| (*flag).to_owned())
    .collect()
}

fn cache_with(value: &str) -> String {
    format!(
        "# This is the CMakeCache file.\n\
         CMAKE_BUILD_TYPE:STRING=MinSizeRel\n\
         //Flags used by the C compiler during all build types.\n\
         CMAKE_C_FLAGS:STRING={value}\n\
         CMAKE_C_FLAGS_MINSIZEREL:STRING=-Os -DNDEBUG\n"
    )
}

fn entry(file: &str, middle: &[String]) -> CcEntry {
    let mut arguments = vec![
        "/opt/arm-gnu-toolchain/bin/arm-none-eabi-gcc".to_owned(),
        "-I/work/run/t/default/src/include".to_owned(),
    ];
    arguments.extend(middle.iter().cloned());
    arguments.extend(
        [
            "-Os",
            "-DNDEBUG",
            "-o",
            &format!("CMakeFiles/t.dir/{file}.o"),
            "-c",
            &format!("/work/run/t/default/src/{file}"),
        ]
        .iter()
        .map(|argument| (*argument).to_owned()),
    );
    CcEntry {
        directory: "/work/run/t/default/build".to_owned(),
        file: format!("/work/run/t/default/src/{file}"),
        arguments,
        output: format!("/work/run/t/default/build/CMakeFiles/t.dir/{file}.o"),
    }
}

/// The fixture: the cache line and two entries that carry the flags as one run.
fn fixture() -> (String, Vec<CcEntry>) {
    let flags = flags();
    (
        cache_with(&flags.join(" ")),
        vec![entry("a.c", &flags), entry("b.c", &flags)],
    )
}

/// These are consistency checks, not a proof: the build can write the cache,
/// and the flag check cannot see added flags (case f).
#[test]
fn build_01_configured_toolchain_consistency_findings() {
    let flags = flags();
    let joined = flags.join(" ");

    // (a) The fixture is consistent.
    let (cache, entries) = fixture();
    assert_eq!(
        configured_toolchain_findings(&cache, &entries, &flags),
        Vec::<String>::new()
    );

    // (b) A flag appended to the cache value.
    let planted = format!("{joined} -DPLANTED");
    let findings = configured_toolchain_findings(&cache_with(&planted), &entries, &flags);
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert!(findings[0].contains("build/CMakeCache.txt"), "{findings:?}");
    assert!(
        findings[0].contains(&planted) && findings[0].contains(&format!("{joined:?}")),
        "the finding names both values: {findings:?}"
    );

    // (c) No CMAKE_C_FLAGS:STRING= line.
    let no_line = cache.replace("CMAKE_C_FLAGS:STRING=", "CMAKE_C_FLAGS_OTHER:STRING=");
    let findings = configured_toolchain_findings(&no_line, &entries, &flags);
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert!(findings[0].contains("build/CMakeCache.txt"), "{findings:?}");

    // (d) One entry lacks -mfpu=fpv4-sp-d16.
    let without: Vec<String> = flags
        .iter()
        .filter(|flag| *flag != "-mfpu=fpv4-sp-d16")
        .cloned()
        .collect();
    let mut changed = entries.clone();
    changed[1] = entry("b.c", &without);
    let findings = configured_toolchain_findings(&cache, &changed, &flags);
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert!(
        findings[0].contains("/work/run/t/default/src/b.c"),
        "{findings:?}"
    );
    assert!(findings[0].contains("1 of 2"), "{findings:?}");

    // (e) The flags split by another argument.
    let mut split = flags.clone();
    split.insert(3, "-DSPLIT".to_owned());
    let mut changed = entries.clone();
    changed[0] = entry("a.c", &split);
    let findings = configured_toolchain_findings(&cache, &changed, &flags);
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert!(
        findings[0].contains("/work/run/t/default/src/a.c"),
        "{findings:?}"
    );

    // (f) A flag added before the run in every entry is not seen: the check
    // cannot see additions, which is why it is only a consistency check.
    let mut added = vec!["-DPLANTED".to_owned()];
    added.extend(flags.iter().cloned());
    let changed = vec![entry("a.c", &added), entry("b.c", &added)];
    assert_eq!(
        configured_toolchain_findings(&cache, &changed, &flags),
        Vec::<String>::new()
    );
}

/// WR-14. Tests run as uid 0 here, where file modes do not stop a read, so a
/// mode-000 file is still readable; the mapping from a read to a cause is
/// therefore tested purely.
#[test]
fn build_01_unreadable_toolchain_file_is_a_named_cause() {
    let expected = "set(CMAKE_C_FLAGS_INIT \"-Os\")\n";
    let key = "stage/toolchain.cmake";
    let path = PathBuf::from("/work/run/t/default/stage/toolchain.cmake");

    // Equal bytes: no finding.
    assert_eq!(
        toolchain_file_finding(key, &Ok(expected.as_bytes().to_vec()), expected),
        None
    );

    // Other bytes: toolchain_changed, naming the key and both sha256 values.
    let other = b"set(CMAKE_C_FLAGS_INIT \"-O0\")\n".to_vec();
    let (token, detail) = toolchain_file_finding(key, &Ok(other.clone()), expected).unwrap();
    assert_eq!(token, "toolchain_changed");
    assert!(detail.contains(key), "{detail}");
    assert!(detail.contains(&sha256_hex(&other)), "{detail}");
    assert!(
        detail.contains(&sha256_hex(expected.as_bytes())),
        "{detail}"
    );

    // Missing: toolchain_changed.
    let missing = Err(RunReadError::Io {
        path: path.clone(),
        source: io::Error::from(io::ErrorKind::NotFound),
    });
    let (token, detail) = toolchain_file_finding(key, &missing, expected).unwrap();
    assert_eq!(token, "toolchain_changed");
    assert!(
        detail.contains(key) && detail.contains("is missing"),
        "{detail}"
    );

    // A symlink or a file that is not regular: run_dir_symlink.
    let refused = Err(RunReadError::NotRegular { path: path.clone() });
    let (token, detail) = toolchain_file_finding(key, &refused, expected).unwrap();
    assert_eq!(token, "run_dir_symlink");
    assert!(detail.contains(key), "{detail}");

    // Any other I/O error: toolchain_unreadable, naming the key and the error.
    let denied = Err(RunReadError::Io {
        path,
        source: io::Error::new(io::ErrorKind::PermissionDenied, "planted permission error"),
    });
    let (token, detail) = toolchain_file_finding(key, &denied, expected).unwrap();
    assert_eq!(token, "toolchain_unreadable");
    assert!(detail.contains(key), "{detail}");
    assert!(detail.contains("planted permission error"), "{detail}");
}

/// The capture directories under `captures` (`<captures>/<id>/<config>`), sorted.
fn local_capture_dirs(captures: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for id in std::fs::read_dir(captures).unwrap() {
        let id = id.unwrap().path();
        if !id.is_dir() {
            continue;
        }
        for cfg in std::fs::read_dir(&id).unwrap() {
            let cfg = cfg.unwrap().path();
            if cfg.is_dir() {
                found.push(cfg);
            }
        }
    }
    found.sort();
    found
}

/// The words of the `CMAKE_C_FLAGS_INIT` value of a toolchain file.
fn init_flags(toolchain: &str) -> Vec<String> {
    let prefix = "set(CMAKE_C_FLAGS_INIT \"";
    let line = toolchain
        .lines()
        .find(|line| line.starts_with(prefix))
        .expect("the toolchain file sets CMAKE_C_FLAGS_INIT");
    line[prefix.len()..]
        .trim_end_matches("\")")
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}

/// Explicit run only: `MT_LOCAL_CAPTURES=<dir> cargo test ... -- --ignored`.
/// It fails, never passes vacuously, when the variable is unset or no CMake
/// capture is found.
#[test]
#[ignore = "reads the captures of an earlier local run; set MT_LOCAL_CAPTURES"]
fn build_01_local_captures_pass_the_toolchain_consistency_checks() {
    let captures = std::env::var("MT_LOCAL_CAPTURES")
        .expect("MT_LOCAL_CAPTURES must name the directory of the local captures");
    assert!(!captures.is_empty(), "MT_LOCAL_CAPTURES is empty");
    let captures = PathBuf::from(captures);
    let runs = captures.parent().unwrap().join("run");
    let (mut pairs, mut entries_checked, mut refused) = (0_usize, 0_usize, 0_usize);
    for cfg_dir in local_capture_dirs(&captures) {
        let toolchain = cfg_dir.join("toolchain.cmake");
        if !toolchain.is_file() {
            continue;
        }
        pairs += 1;
        let id = cfg_dir.parent().unwrap().file_name().unwrap();
        let cfg = cfg_dir.file_name().unwrap();
        let run_root = runs.join(id).join(cfg);
        let flags = init_flags(&std::fs::read_to_string(&toolchain).unwrap());
        let cache = std::fs::read_to_string(run_root.join("build/CMakeCache.txt")).unwrap();
        let canonical = cfg_dir.join("compile_commands.json");
        let entries: Vec<CcEntry> = if canonical.is_file() {
            serde_json::from_slice(&std::fs::read(&canonical).unwrap()).unwrap()
        } else {
            // No canonical file (tinycbor's local build failed on the host
            // newlib): the run export, every output treated as existing.
            let export = run_root.join("build/compile_commands.json");
            let raw = parse_raw(&std::fs::read(&export).unwrap()).unwrap();
            let compilers: std::collections::BTreeSet<String> = raw
                .iter()
                .map(|raw| match (&raw.arguments, &raw.command) {
                    (Some(arguments), _) => arguments[0].clone(),
                    (None, Some(command)) => shlex::split(command).unwrap()[0].clone(),
                    (None, None) => String::new(),
                })
                .collect();
            let policy = CcPolicy {
                allowed_compilers: compilers.into_iter().collect(),
                run_root: run_root.clone(),
            };
            canonicalise(&raw, &policy, &|_: &Path| true)
                .unwrap()
                .entries
        };
        entries_checked += entries.len();
        let findings = configured_toolchain_findings(&cache, &entries, &flags);
        if !findings.is_empty() {
            refused += 1;
            eprintln!("REFUSED {}: {findings:?}", cfg_dir.display());
        }
    }
    assert!(
        pairs > 0,
        "no CMake capture (toolchain.cmake) under {}",
        captures.display()
    );
    println!("cmake_pairs={pairs} entries={entries_checked} refused={refused}");
    assert_eq!(refused, 0);
}
