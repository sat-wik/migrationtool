#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]
//! Helpers shared by the capture and entry tests: a local git upstream served
//! over `file://`, target builders and the locally installed Arm tools.
//!
//! The real Arm toolchain, Bear and CMake of the pinned image are not required
//! by `cargo test`. A test that needs them calls [`local_arm_tools`] and returns
//! early when this host lacks them, so the same test runs end to end on a
//! machine that has the tools and is skipped, visibly, where it does not.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use mt_capture::capture::{CaptureContext, ToolPaths};
use mt_capture::fetch::FetchContext;
use mt_capture::manifest::{BASELINE_CFLAGS, BuildSystem, Category, Licence, Source, Target};
use mt_toolchain::runner::{RunOutput, RunRequest, RunnerConfig, run};
use tempfile::TempDir;

pub const GIT: &str = "/usr/bin/git";

pub fn runner_config() -> RunnerConfig {
    RunnerConfig::new("/usr/bin:/bin", "1791158400")
}

/// Run a tool through the runner and require a zero exit.
pub fn tool(argv: &[&str], cwd: &Path, env: &[(&str, &str)]) -> RunOutput {
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

/// A local git repository with one commit, reachable by SHA over `file://`.
pub struct Upstream {
    pub _tmp: TempDir,
    pub url: String,
    pub commit: String,
    pub tree_sha256: String,
}

pub fn make_upstream(files: &[(&str, &str)]) -> Upstream {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("upstream");
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    for (name, text) in files {
        let path = repo.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    let repo_text = repo.to_str().unwrap();
    let env = [
        ("HOME", home.to_str().unwrap()),
        ("GIT_CONFIG_GLOBAL", "/dev/null"),
        ("GIT_CONFIG_NOSYSTEM", "1"),
        ("GIT_AUTHOR_NAME", "Fixture"),
        ("GIT_AUTHOR_EMAIL", "fixture@example.invalid"),
        ("GIT_COMMITTER_NAME", "Fixture"),
        ("GIT_COMMITTER_EMAIL", "fixture@example.invalid"),
        ("GIT_AUTHOR_DATE", "2026-01-01T00:00:00Z"),
        ("GIT_COMMITTER_DATE", "2026-01-01T00:00:00Z"),
    ];
    tool(&[GIT, "init", "--quiet", repo_text], tmp.path(), &env);
    tool(
        &[
            GIT,
            "-C",
            repo_text,
            "config",
            "uploadpack.allowAnySHA1InWant",
            "true",
        ],
        tmp.path(),
        &env,
    );
    tool(&[GIT, "-C", repo_text, "add", "-A"], tmp.path(), &env);
    tool(
        &[GIT, "-C", repo_text, "commit", "--quiet", "-m", "fixture"],
        tmp.path(),
        &env,
    );
    let head = tool(
        &[GIT, "-C", repo_text, "rev-parse", "HEAD"],
        tmp.path(),
        &env,
    );
    let commit = String::from_utf8(head.stdout).unwrap().trim().to_owned();
    let tree_sha256 = mt_capture::treehash::tree_sha256(&repo).unwrap();
    Upstream {
        url: format!("file://{}", repo.display()),
        _tmp: tmp,
        commit,
        tree_sha256,
    }
}

/// A target whose source is the local upstream; the caller fills the recipe.
pub fn git_target(upstream: &Upstream, build: BuildSystem) -> Target {
    Target {
        category: vec![Category::Tlv],
        build,
        source: Source::Git {
            url: upstream.url.clone(),
            reference: "main".to_owned(),
            commit: upstream.commit.clone(),
            tree_sha256: upstream.tree_sha256.clone(),
        },
        licence: Licence {
            spdx: "MIT".to_owned(),
            elected: "MIT".to_owned(),
            file: "LICENSE".to_owned(),
            note: None,
        },
        build_args: None,
        entry: Vec::new(),
        config: Vec::new(),
        seed_source: Vec::new(),
    }
}

pub fn fetch_context(work: &Path) -> FetchContext {
    FetchContext {
        work_dir: work.to_path_buf(),
        git: PathBuf::from(GIT),
        curl: PathBuf::from("/usr/bin/curl"),
        tar: PathBuf::from("/usr/bin/tar"),
        allowed_protocols: "file".to_owned(),
        timeout: Duration::from_secs(120),
    }
}

pub fn baseline() -> Vec<String> {
    BASELINE_CFLAGS
        .iter()
        .map(|flag| (*flag).to_owned())
        .collect()
}

/// The Arm toolchain, Bear, CMake and clang installed on this host, or `None`
/// (the test then skips itself).
pub fn local_arm_tools() -> Option<ToolPaths> {
    let paths = [
        "/usr/bin/arm-none-eabi-gcc",
        "/usr/bin/arm-none-eabi-ar",
        "/usr/bin/arm-none-eabi-readelf",
        "/usr/bin/arm-none-eabi-objcopy",
        "/usr/bin/bear",
        "/usr/bin/make",
        "/usr/bin/cmake",
        "/usr/bin/clang",
        "/usr/bin/ar",
        GIT,
        "/usr/lib/arm-none-eabi/include",
    ];
    if let Some(missing) = paths.iter().find(|path| !Path::new(path).exists()) {
        eprintln!("skipping: {missing} is not installed on this host");
        return None;
    }
    Some(ToolPaths {
        arm_gcc: PathBuf::from("/usr/bin/arm-none-eabi-gcc"),
        bear: PathBuf::from("/usr/bin/bear"),
        cmake: PathBuf::from("/usr/bin/cmake"),
        make: PathBuf::from("/usr/bin/make"),
        clang: PathBuf::from("/usr/bin/clang"),
        readelf: PathBuf::from("/usr/bin/arm-none-eabi-readelf"),
        objcopy: PathBuf::from("/usr/bin/arm-none-eabi-objcopy"),
        sysroot: PathBuf::from("/usr/lib/arm-none-eabi"),
        git: PathBuf::from(GIT),
        versions: BTreeMap::from([("arm_gcc".to_owned(), "local".to_owned())]),
    })
}

pub fn capture_context(work: &Path, testbed: &Path, tools: ToolPaths) -> CaptureContext {
    CaptureContext {
        work_dir: work.to_path_buf(),
        testbed_dir: testbed.to_path_buf(),
        tools,
        baseline: baseline(),
        timeout: Duration::from_secs(300),
    }
}
