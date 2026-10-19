#![allow(clippy::unwrap_used, clippy::expect_used)]
#![cfg(unix)]
//! BUILD-01 capture in two stages (review warning WR-04, plan 02-18).
//!
//! The build stage runs the untrusted upstream build and may write only its own
//! run directory (`<work>/run/<id>/<config>`). The record stage runs no upstream
//! code and alone writes the capture artifacts that replay trusts. Most tests
//! run the real Make and CMake drivers and so need the host's Arm tools (see
//! `support::local_arm_tools`); `build_01_run_dir_reads_refuse_symlinks` is pure
//! and runs everywhere.

mod support;

use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::{MetadataExt, symlink};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};

use mt_capture::capture::{
    BuildRecord, CaptureContext, RunLayout, RunReadError, capture, capture_build, capture_record,
    list_files, read_run_file,
};
use mt_capture::compare::compare_capture_dirs;
use mt_capture::fetch::fetch;
use mt_capture::manifest::{BuildArgs, BuildSystem, Config, Target};
use mt_capture::report::{CaptureReport, CaptureVerdict};
use mt_capture::treehash::copy_tree;
use support::{
    capture_context, fetch_context, git_target, local_arm_tools, make_upstream, runner_config,
};

const MAKE_PROJECT: [(&str, &str); 3] = [
    ("t.h", "int t_add(int a, int b);\n"),
    (
        "t.c",
        "#include \"t.h\"\nint t_add(int a, int b) { return a + b; }\n",
    ),
    (
        "Makefile",
        "all: libt.a\nlibt.a: t.o\n\tar rcs $@ t.o\nt.o: t.c t.h\n\t$(CC) -c t.c -o t.o\n",
    ),
];

const CMAKE_PROJECT: [(&str, &str); 5] = [
    (
        "CMakeLists.txt",
        "cmake_minimum_required(VERSION 3.10)\nproject(demo C)\n\
         configure_file(gen.h.in gen.h)\n\
         add_library(tj STATIC t.c)\n\
         target_include_directories(tj PRIVATE ${CMAKE_CURRENT_BINARY_DIR})\n\
         add_library(unused STATIC EXCLUDE_FROM_ALL u.c)\n",
    ),
    ("gen.h.in", "#define GEN 7\n"),
    ("t.c", "#include \"gen.h\"\nint t(void) { return GEN; }\n"),
    ("u.c", "int u(void) { return 2; }\n"),
    ("README", "x\n"),
];

fn make_target(upstream: &support::Upstream) -> Target {
    let mut target = git_target(upstream, BuildSystem::Make);
    target.build_args = Some(BuildArgs {
        dir: ".".to_owned(),
        goals: vec!["libt.a".to_owned()],
        make_vars: vec!["CC={cc} {cflags}".to_owned()],
        cmake_target: None,
        cmake_options: Vec::new(),
    });
    target
}

fn cmake_target(upstream: &support::Upstream) -> Target {
    let mut target = git_target(upstream, BuildSystem::Cmake);
    target.build_args = Some(BuildArgs {
        dir: ".".to_owned(),
        goals: Vec::new(),
        make_vars: Vec::new(),
        cmake_target: Some("tj".to_owned()),
        cmake_options: Vec::new(),
    });
    target
}

fn default_config() -> Config {
    Config {
        name: "default".to_owned(),
        ..Config::default()
    }
}

/// A fetched local upstream and the context to capture it with.
struct Bed {
    _work: tempfile::TempDir,
    _testbed: tempfile::TempDir,
    ctx: CaptureContext,
    upstream: support::Upstream,
}

fn bed(files: &[(&str, &str)]) -> Option<Bed> {
    let tools = local_arm_tools()?;
    let upstream = make_upstream(files);
    let work = tempfile::tempdir().unwrap();
    let testbed = tempfile::tempdir().unwrap();
    let ctx = capture_context(work.path(), testbed.path(), tools);
    Some(Bed {
        _work: work,
        _testbed: testbed,
        ctx,
        upstream,
    })
}

fn fetched(bed: &Bed, target: &Target) {
    fetch(
        &runner_config(),
        &fetch_context(&bed.ctx.work_dir),
        "demo",
        target,
    )
    .unwrap();
}

fn build_stage(bed: &Bed, target: &Target) -> BuildRecord {
    capture_build(
        &runner_config(),
        &bed.ctx,
        "demo",
        target,
        &default_config(),
    )
    .unwrap()
}

fn record_stage(bed: &Bed, target: &Target) -> CaptureReport {
    capture_record(
        &runner_config(),
        &bed.ctx,
        "demo",
        target,
        &default_config(),
    )
    .unwrap()
}

fn layout_of(bed: &Bed) -> RunLayout {
    RunLayout::new(&bed.ctx.work_dir, "demo", "default")
}

fn read_record(layout: &RunLayout) -> BuildRecord {
    serde_json::from_slice(&fs::read(layout.build_record_path()).unwrap()).unwrap()
}

fn write_record(layout: &RunLayout, record: &BuildRecord) {
    fs::write(
        layout.build_record_path(),
        serde_json::to_vec_pretty(record).unwrap(),
    )
    .unwrap();
}

#[test]
fn build_01_staged_capture_equals_the_single_process_capture() {
    for (files, cmake) in [
        (MAKE_PROJECT.as_slice(), false),
        (CMAKE_PROJECT.as_slice(), true),
    ] {
        let Some(bed) = bed(files) else { return };
        let target = if cmake {
            cmake_target(&bed.upstream)
        } else {
            make_target(&bed.upstream)
        };
        fetched(&bed, &target);
        let layout = layout_of(&bed);

        let single = capture(
            &runner_config(),
            &bed.ctx,
            "demo",
            &target,
            &default_config(),
        )
        .unwrap();
        assert_eq!(
            single.verdict,
            CaptureVerdict::Captured,
            "{:#?}",
            single.causes
        );
        let hold = tempfile::tempdir().unwrap();
        let snapshot = hold.path().join("single");
        copy_tree(&layout.capture_dir, &snapshot).unwrap();

        let record = build_stage(&bed, &target);
        assert!(record.built, "{record:#?}");
        let staged = record_stage(&bed, &target);

        assert_eq!(
            staged.verdict,
            CaptureVerdict::Captured,
            "{:#?}",
            staged.causes
        );
        let comparison = compare_capture_dirs(&snapshot, &layout.capture_dir).unwrap();
        assert!(
            comparison.passed(),
            "cmake={cmake}: {:#?} (compared {})",
            comparison.problems,
            comparison.compared
        );
    }
}

#[test]
fn build_01_build_stage_writes_only_the_run_directory() {
    for (files, cmake) in [
        (MAKE_PROJECT.as_slice(), false),
        (CMAKE_PROJECT.as_slice(), true),
    ] {
        let Some(bed) = bed(files) else { return };
        let target = if cmake {
            cmake_target(&bed.upstream)
        } else {
            make_target(&bed.upstream)
        };
        fetched(&bed, &target);
        let work = bed.ctx.work_dir.clone();
        let before = list_files(&work).unwrap();

        let record = build_stage(&bed, &target);

        assert!(record.built, "{record:#?}");
        assert!(!work.join("capture").exists(), "cmake={cmake}");
        assert!(!work.join("reports").exists(), "cmake={cmake}");
        let after = list_files(&work).unwrap();
        let allowed = ["run/demo/default/", "home/", "tmp/"];
        for key in after.difference(&before) {
            assert!(
                allowed.iter().any(|prefix| key.starts_with(prefix)),
                "cmake={cmake}: the build stage wrote {key}"
            );
        }
        let layout = layout_of(&bed);
        assert_eq!(layout.stage_dir(), layout.run_root.join("stage"));
        let mut staged = vec![layout.build_record_path(), layout.raw_path()];
        if cmake {
            staged.push(layout.toolchain_path());
        }
        for path in staged {
            assert!(path.is_file(), "cmake={cmake}: missing {}", path.display());
            assert!(
                path.starts_with(layout.stage_dir()),
                "{} is not in the stage directory",
                path.display()
            );
        }
        assert_eq!(read_record(&layout), record);
    }
}

#[test]
fn build_01_build_stage_keeps_an_existing_run_root() {
    let Some(bed) = bed(&MAKE_PROJECT) else {
        return;
    };
    let target = make_target(&bed.upstream);
    fetched(&bed, &target);
    let layout = layout_of(&bed);
    fs::create_dir_all(layout.run_root.join("old/deep")).unwrap();
    fs::write(layout.run_root.join("stale.txt"), "stale").unwrap();
    fs::write(layout.run_root.join("old/deep/x"), "stale").unwrap();
    let inode = fs::metadata(&layout.run_root).unwrap().ino();

    let record = build_stage(&bed, &target);

    assert!(record.built, "{record:#?}");
    assert_eq!(fs::metadata(&layout.run_root).unwrap().ino(), inode);
    assert!(!layout.run_root.join("stale.txt").exists());
    assert!(!layout.run_root.join("old").exists());
    assert!(layout.src.join("t.c").is_file());
    assert!(layout.build_record_path().is_file());
}

#[test]
fn build_01_record_stage_without_a_build_record_is_not_proven() {
    let Some(bed) = bed(&MAKE_PROJECT) else {
        return;
    };
    let target = make_target(&bed.upstream);
    fetched(&bed, &target);
    let work = bed.ctx.work_dir.clone();
    fs::remove_dir_all(work.join("home")).unwrap();
    fs::remove_dir_all(work.join("tmp")).unwrap();

    let report = record_stage(&bed, &target);

    assert_eq!(report.verdict, CaptureVerdict::NotProven);
    assert!(
        report.causes.iter().any(|cause| cause == "no_build_record"),
        "{:?}",
        report.causes
    );
    let saved: CaptureReport =
        serde_json::from_slice(&fs::read(layout_of(&bed).report_path()).unwrap()).unwrap();
    assert_eq!(saved, report);
    assert!(work.join("home").is_dir() && work.join("tmp").is_dir());
}

#[test]
fn build_01_record_stage_refuses_a_record_for_another_pair() {
    let Some(bed) = bed(&MAKE_PROJECT) else {
        return;
    };
    let target = make_target(&bed.upstream);
    fetched(&bed, &target);
    let layout = layout_of(&bed);
    build_stage(&bed, &target);
    let genuine = read_record(&layout);

    for (what, forge) in [
        (
            "target",
            Box::new(|r: &mut BuildRecord| r.target = "other".to_owned())
                as Box<dyn Fn(&mut BuildRecord)>,
        ),
        (
            "config",
            Box::new(|r: &mut BuildRecord| r.config = "variant".to_owned()),
        ),
        (
            "source commit",
            Box::new(|r: &mut BuildRecord| r.source.commit = "0".repeat(40)),
        ),
        (
            "source tree",
            Box::new(|r: &mut BuildRecord| r.source.tree_sha256 = "0".repeat(64)),
        ),
    ] {
        let mut record = genuine.clone();
        forge(&mut record);
        write_record(&layout, &record);

        let report = record_stage(&bed, &target);

        assert_eq!(report.verdict, CaptureVerdict::NotProven, "{what}");
        assert!(
            report
                .causes
                .iter()
                .any(|cause| cause == "build_record_mismatch"),
            "{what}: {:?}",
            report.causes
        );
        assert!(
            !report.causes.iter().any(|cause| cause == "no_build_record"),
            "{what}"
        );
    }
}

#[test]
fn build_01_record_stage_refuses_changed_inputs() {
    let Some(bed) = bed(&MAKE_PROJECT) else {
        return;
    };
    let committed = bed.ctx.testbed_dir.join("targets/demo");
    fs::create_dir_all(&committed).unwrap();
    fs::write(committed.join("extra.h"), "#define EXTRA 1\n").unwrap();
    let target = make_target(&bed.upstream);
    fetched(&bed, &target);
    let layout = layout_of(&bed);

    build_stage(&bed, &target);
    let untouched = record_stage(&bed, &target);
    assert_eq!(
        untouched.verdict,
        CaptureVerdict::Captured,
        "{:#?}",
        untouched.causes
    );
    assert!(untouched.inputs.contains_key("extra.h"));

    // A changed committed input.
    build_stage(&bed, &target);
    fs::write(layout.inputs.join("extra.h"), "#define EXTRA 2\n").unwrap();
    let changed = record_stage(&bed, &target);
    assert_eq!(
        changed.verdict,
        CaptureVerdict::CaptureFailed,
        "{:#?}",
        changed.causes
    );
    assert!(changed.causes.iter().any(|cause| cause == "inputs_changed"));
    assert!(
        changed
            .cause_details
            .iter()
            .any(|detail| detail.contains("extra.h")),
        "{:?}",
        changed.cause_details
    );

    // A file the build added to the inputs copy.
    build_stage(&bed, &target);
    fs::write(layout.inputs.join("planted.h"), "#define PLANTED 1\n").unwrap();
    let planted = record_stage(&bed, &target);
    assert_eq!(
        planted.verdict,
        CaptureVerdict::CaptureFailed,
        "{:#?}",
        planted.causes
    );
    assert!(planted.causes.iter().any(|cause| cause == "inputs_changed"));
    assert!(
        planted
            .cause_details
            .iter()
            .any(|detail| detail.contains("planted.h"))
    );
}

#[test]
fn build_01_run_dir_reads_refuse_symlinks() {
    let run = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let root = run.path();
    fs::create_dir_all(root.join("stage")).unwrap();
    fs::create_dir_all(root.join("dir")).unwrap();
    fs::write(root.join("stage/record.json"), b"{}\n").unwrap();
    fs::write(root.join("plain.o"), b"object bytes").unwrap();
    fs::write(outside.path().join("secret"), b"outside bytes").unwrap();
    fs::create_dir_all(outside.path().join("elsewhere")).unwrap();
    fs::write(outside.path().join("elsewhere/b.json"), b"{}\n").unwrap();

    // A regular file, directly and below a real directory.
    assert_eq!(read_run_file(root, "plain.o").unwrap(), b"object bytes");
    assert_eq!(read_run_file(root, "stage/record.json").unwrap(), b"{}\n");

    let refused = |relative: &str| -> PathBuf {
        match read_run_file(root, relative) {
            Err(RunReadError::NotRegular { path }) => {
                let text = RunReadError::NotRegular { path: path.clone() }.to_string();
                assert!(text.contains(&path.display().to_string()), "{text}");
                path
            }
            other => panic!("{relative}: expected NotRegular, got {other:?}"),
        }
    };

    // A symlink to a regular file inside the run root.
    symlink(root.join("plain.o"), root.join("link-inside.o")).unwrap();
    assert_eq!(refused("link-inside.o"), root.join("link-inside.o"));
    // A symlink to a file outside it.
    symlink(outside.path().join("secret"), root.join("link-outside.o")).unwrap();
    assert_eq!(refused("link-outside.o"), root.join("link-outside.o"));
    // A regular file reached through a symlinked directory component.
    symlink(outside.path().join("elsewhere"), root.join("linked")).unwrap();
    assert_eq!(refused("linked/b.json"), root.join("linked"));
    fs::remove_dir_all(root.join("stage")).unwrap();
    symlink(outside.path().join("elsewhere"), root.join("stage")).unwrap();
    assert_eq!(refused("stage/b.json"), root.join("stage"));
    // A directory.
    assert_eq!(refused("dir"), root.join("dir"));
    // A Unix socket.
    let socket = root.join("sock");
    let _listener = UnixListener::bind(&socket).unwrap();
    assert_eq!(refused("sock"), socket);
    // A relative path with a parent or current-directory component.
    refused("dir/../plain.o");
    refused("../secret");
    refused("./plain.o");
    // An absolute path, and an empty one.
    refused(outside.path().join("secret").to_str().unwrap());
    refused("");
    // A file that is not there is an I/O error, not a refusal.
    match read_run_file(root, "absent.o") {
        Err(RunReadError::Io { path, source }) => {
            assert_eq!(path, root.join("absent.o"));
            assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
        }
        other => panic!("expected Io, got {other:?}"),
    }
}

fn files_under(root: &Path) -> BTreeSet<PathBuf> {
    list_files(root)
        .unwrap()
        .into_iter()
        .map(|key| root.join(key))
        .collect()
}

#[test]
fn build_01_record_stage_does_not_follow_symlinks() {
    let Some(bed) = bed(&MAKE_PROJECT) else {
        return;
    };
    let target = make_target(&bed.upstream);
    fetched(&bed, &target);
    let layout = layout_of(&bed);
    let outside = tempfile::tempdir().unwrap();
    let marker = b"MARKER-BYTES-OUTSIDE-THE-WORK-DIRECTORY";
    let secret = outside.path().join("secret.o");
    fs::write(&secret, marker).unwrap();

    // A declared object replaced by a symlink to a file outside the work directory.
    build_stage(&bed, &target);
    let object = layout.src.join("t.o");
    fs::remove_file(&object).unwrap();
    symlink(&secret, &object).unwrap();

    let report = record_stage(&bed, &target);

    assert_eq!(
        report.verdict,
        CaptureVerdict::CaptureFailed,
        "{:#?}",
        report.causes
    );
    assert_eq!(report.causes, ["run_dir_symlink"]);
    assert!(
        report
            .cause_details
            .iter()
            .any(|detail| detail.contains("src/t.o")),
        "{:?}",
        report.cause_details
    );
    for path in files_under(&layout.capture_dir) {
        let bytes = fs::read(&path).unwrap();
        assert!(
            !bytes.windows(marker.len()).any(|window| window == marker),
            "{} holds bytes from outside the run directory",
            path.display()
        );
    }

    // The build record replaced by a symlink to a valid copy outside the run root.
    build_stage(&bed, &target);
    let copy = outside.path().join("build-record.json");
    fs::copy(layout.build_record_path(), &copy).unwrap();
    fs::remove_file(layout.build_record_path()).unwrap();
    symlink(&copy, layout.build_record_path()).unwrap();

    let report = record_stage(&bed, &target);

    assert_eq!(
        report.verdict,
        CaptureVerdict::NotProven,
        "{:#?}",
        report.causes
    );
    assert!(
        report.causes.iter().any(|cause| cause == "no_build_record"),
        "{:?}",
        report.causes
    );
}
