#![allow(clippy::unwrap_used, clippy::expect_used)]
#![cfg(unix)]
//! BUILD-02 replay logic proved on a real two-file host-cc project (CONTEXT
//! D-26): no Docker, no network, no Arm compiler.
//!
//! The project is built with `/usr/bin/cc` through the runner, wiped, restored
//! from its pristine copy and replayed from its compile commands. The same code
//! replays the real Arm builds in CI.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};

use mt_capture::cc::{CcEntry, CcError, CcPolicy};
use mt_capture::replay::{ReplayError, ReplayPlan, replay};
use mt_capture::report::{ObjectStatus, ReplayReport, Verdict};
use mt_capture::treehash::{TreeHashError, copy_tree, sha256_hex, tree_sha256};
use mt_toolchain::runner::{RunRecord, RunRequest, RunnerConfig, run};
use tempfile::TempDir;

const CC: &str = "/usr/bin/cc";
const A_OBJECT: &str = "src/out/a.o";
const B_OBJECT: &str = "src/out/b.o";

fn config() -> RunnerConfig {
    RunnerConfig::new("/usr/bin:/bin", "1791158400")
}

fn write_project(dir: &Path) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("a.c"),
        "int b_value(void);\nint a_value(void) { return b_value() + 1; }\n",
    )
    .unwrap();
    fs::write(dir.join("b.h"), "#define B_BASE 41\n").unwrap();
    fs::write(
        dir.join("b.c"),
        "#include \"b.h\"\nint b_value(void) { return B_BASE; }\n",
    )
    .unwrap();
}

fn request(argv: &[&str], cwd: &Path) -> RunRequest {
    RunRequest {
        tool_name: "cc".to_owned(),
        argv: argv.iter().map(OsString::from).collect(),
        cwd: cwd.to_path_buf(),
        extra_env: BTreeMap::new(),
        tool_version: None,
    }
}

/// A built original, its pristine tree and the saved original objects.
struct Fixture {
    _tmp: TempDir,
    pristine: PathBuf,
    run_root: PathBuf,
    originals: PathBuf,
    hashes: BTreeMap<String, String>,
}

impl Fixture {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        let pristine = root.join("pristine");
        let run_root = root.join("run");
        let originals = root.join("originals");
        write_project(&pristine);
        let src = run_root.join("src");
        write_project(&src);
        fs::create_dir_all(src.join("out")).unwrap();

        let mut hashes = BTreeMap::new();
        for name in ["a", "b"] {
            let object = format!("out/{name}.o");
            let source = format!("{name}.c");
            let output = run(
                &config(),
                &request(&[CC, "-c", "-o", &object, &source], &src),
            )
            .unwrap();
            assert_eq!(output.record.exit.code, Some(0), "original build of {name}");
            let bytes = fs::read(src.join(&object)).unwrap();
            let key = format!("src/out/{name}.o");
            let saved = originals.join(&key);
            fs::create_dir_all(saved.parent().unwrap()).unwrap();
            fs::write(&saved, &bytes).unwrap();
            hashes.insert(key, sha256_hex(&bytes));
        }
        Self {
            _tmp: tmp,
            pristine,
            run_root,
            originals,
            hashes,
        }
    }

    fn entries(&self) -> Vec<CcEntry> {
        let directory = self.run_root.join("src");
        ["a", "b"]
            .iter()
            .map(|name| CcEntry {
                directory: directory.to_str().unwrap().to_owned(),
                file: format!("{name}.c"),
                arguments: [
                    CC,
                    "-c",
                    "-o",
                    &format!("out/{name}.o"),
                    &format!("{name}.c"),
                ]
                .iter()
                .map(|s| (*s).to_owned())
                .collect(),
                output: directory
                    .join(format!("out/{name}.o"))
                    .to_str()
                    .unwrap()
                    .to_owned(),
            })
            .collect()
    }

    fn plan(&self) -> ReplayPlan {
        ReplayPlan {
            target: "tiny".to_owned(),
            config: "default".to_owned(),
            run_root: self.run_root.clone(),
            pristine_src: self.pristine.clone(),
            src_dir_name: "src".to_owned(),
            keep_dirs: Vec::new(),
            entries: self.entries(),
            original_objects: self.hashes.clone(),
            original_objects_dir: self.originals.clone(),
            policy: CcPolicy {
                allowed_compilers: vec![CC.to_owned()],
                run_root: self.run_root.clone(),
            },
            compiler_tool_name: "cc".to_owned(),
            normaliser: None,
        }
    }
}

#[test]
fn build_02_replay_matches_original_objects() {
    let fixture = Fixture::new();
    let report = replay(&config(), &fixture.plan()).unwrap();

    assert_eq!(report.verdict, Verdict::Identical, "{report:?}");
    assert_eq!(report.kind, "replay");
    assert_eq!(report.target, "tiny");
    assert_eq!(report.config, "default");
    assert_eq!(
        report
            .objects
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        [A_OBJECT, B_OBJECT]
    );
    for (key, object) in &report.objects {
        assert_eq!(object.status, ObjectStatus::Identical, "{key}");
        assert_eq!(object.original, object.replay, "{key}");
        assert_eq!(object.original.as_ref(), fixture.hashes.get(key), "{key}");
        assert!(object.differing_sections.is_empty(), "{key}");
    }
    assert!(!report.normalisation_applied);
    assert_eq!(report.steps.len(), 2);
    assert_eq!(report.steps[0].name, "compile a.c");
    assert_eq!(report.steps[1].name, "compile b.c");
    for step in &report.steps {
        assert_eq!(step.run.exit.code, Some(0));
        assert_eq!(step.run.tool.name, "cc");
    }
    assert!(report.to_json().unwrap().contains("normalisation_rule"));
}

#[test]
fn build_02_replay_detects_changed_object() {
    let fixture = Fixture::new();
    // One byte of the pristine source changes: the constant 1 becomes 2.
    fs::write(
        fixture.pristine.join("a.c"),
        "int b_value(void);\nint a_value(void) { return b_value() + 2; }\n",
    )
    .unwrap();
    let report = replay(&config(), &fixture.plan()).unwrap();

    assert_eq!(report.verdict, Verdict::Differs, "{report:?}");
    assert_eq!(report.objects[A_OBJECT].status, ObjectStatus::Differs);
    assert_ne!(
        report.objects[A_OBJECT].original,
        report.objects[A_OBJECT].replay
    );
    assert_eq!(report.objects[B_OBJECT].status, ObjectStatus::Identical);
}

#[test]
fn build_02_clean_copy_replay_removes_stale_outputs() {
    let fixture = Fixture::new();
    // A good object sits at the output path inside the pristine tree, and the
    // source that should produce it no longer compiles.
    fs::create_dir_all(fixture.pristine.join("out")).unwrap();
    fs::copy(
        fixture.originals.join(A_OBJECT),
        fixture.pristine.join("out/a.o"),
    )
    .unwrap();
    fs::write(fixture.pristine.join("a.c"), "this is not C\n").unwrap();
    let report = replay(&config(), &fixture.plan()).unwrap();

    assert_eq!(report.verdict, Verdict::BuildFailed, "{report:?}");
    assert_ne!(report.verdict, Verdict::Identical);
    assert_eq!(
        report.objects[A_OBJECT].status,
        ObjectStatus::MissingInReplay
    );
    assert!(!fixture.run_root.join("src/out/a.o").exists());
    // The other entry still ran, so the report lists every failure.
    assert_eq!(report.steps.len(), 2);
    assert_ne!(report.steps[0].run.exit.code, Some(0));
    assert_eq!(report.steps[1].run.exit.code, Some(0));
    assert_eq!(report.objects[B_OBJECT].status, ObjectStatus::Identical);
}

#[test]
fn build_02_missing_object_is_reported_as_differs() {
    let fixture = Fixture::new();
    let mut plan = fixture.plan();
    // An original object that no entry produces.
    plan.original_objects
        .insert("src/out/extra.o".to_owned(), sha256_hex(b"extra"));
    let report = replay(&config(), &plan).unwrap();

    assert_eq!(report.verdict, Verdict::Differs, "{report:?}");
    let extra = &report.objects["src/out/extra.o"];
    assert_eq!(extra.status, ObjectStatus::MissingInReplay);
    assert_eq!(extra.replay, None);
    assert_eq!(report.objects[A_OBJECT].status, ObjectStatus::Identical);
}

#[test]
fn build_02_object_without_original_is_reported_as_differs() {
    let fixture = Fixture::new();
    let mut plan = fixture.plan();
    plan.original_objects.remove(B_OBJECT);
    let report = replay(&config(), &plan).unwrap();

    assert_eq!(report.verdict, Verdict::Differs, "{report:?}");
    let object = &report.objects[B_OBJECT];
    assert_eq!(object.status, ObjectStatus::MissingInOriginal);
    assert_eq!(object.original, None);
    assert!(object.replay.is_some());
}

#[test]
fn build_02_zero_object_configuration_never_identical() {
    let fixture = Fixture::new();
    let mut plan = fixture.plan();
    plan.entries.clear();
    let report = replay(&config(), &plan).unwrap();

    assert_eq!(report.verdict, Verdict::BuildFailed);
    assert_eq!(report.detail.as_deref(), Some("empty_capture"));
    assert!(report.steps.is_empty());
}

#[test]
fn build_02_replay_refuses_non_allowlisted_compiler() {
    let fixture = Fixture::new();
    let marker = fixture.run_root.join("marker");
    let directory = fixture.run_root.join("src");
    let mut plan = fixture.plan();
    plan.entries = vec![CcEntry {
        directory: directory.to_str().unwrap().to_owned(),
        file: "a.c".to_owned(),
        arguments: vec![
            "/bin/sh".to_owned(),
            "-c".to_owned(),
            format!("touch {}", marker.display()),
        ],
        output: directory.join("a.o").to_str().unwrap().to_owned(),
    }];
    let error = replay(&config(), &plan).unwrap_err();

    assert!(
        matches!(
            &error,
            ReplayError::Entry(CcError::CompilerNotAllowed { argv0 }) if argv0 == "/bin/sh"
        ),
        "{error:?}"
    );
    assert!(
        !marker.exists(),
        "nothing may be spawned before the refusal"
    );
}

#[test]
fn build_02_report_is_deterministic() {
    let fixture = Fixture::new();
    let first = replay(&config(), &fixture.plan()).unwrap();
    let second = replay(&config(), &fixture.plan()).unwrap();

    assert_eq!(first.to_json().unwrap(), first.to_json().unwrap());
    assert_eq!(first.to_json().unwrap(), second.to_json().unwrap());
    let text = first.to_json().unwrap();
    assert!(text.ends_with("}\n"));
    // Objects are keyed in sorted order.
    assert!(text.find(A_OBJECT).unwrap() < text.find(B_OBJECT).unwrap());
    let parsed: ReplayReport = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed, first);
}

#[test]
fn build_02_run_record_round_trips_through_json() {
    let output = run(
        &config(),
        &request(
            &["/bin/sh", "-c", "printf out; printf err 1>&2"],
            Path::new("/"),
        ),
    )
    .unwrap();
    let text = serde_json::to_string(&output.record).unwrap();
    let parsed: RunRecord = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed, output.record);
    let pretty: RunRecord = serde_json::from_str(&output.record.to_json().unwrap()).unwrap();
    assert_eq!(pretty, output.record);
}

fn write_tree(root: &Path) {
    fs::create_dir_all(root.join("sub/.git")).unwrap();
    fs::create_dir_all(root.join(".git")).unwrap();
    fs::write(root.join(".git/config"), "[core]\n").unwrap();
    fs::write(root.join("sub/.git/keep"), "nested\n").unwrap();
    fs::write(root.join("data.txt"), "data\n").unwrap();
    fs::write(root.join("tool.sh"), "#!/bin/sh\n").unwrap();
    fs::write(root.join("sub/real.txt"), "real\n").unwrap();
    fs::set_permissions(root.join("tool.sh"), fs::Permissions::from_mode(0o755)).unwrap();
    fs::set_permissions(root.join("data.txt"), fs::Permissions::from_mode(0o644)).unwrap();
    symlink("sub/real.txt", root.join("link")).unwrap();
    symlink("../data.txt", root.join("sub/up")).unwrap();
}

#[test]
fn bed_01_tree_copy_preserves_symlinks_and_refuses_escapes() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    write_tree(&src);
    copy_tree(&src, &dst).unwrap();

    // Symlinks stay symlinks with the same link text.
    let link = fs::symlink_metadata(dst.join("link")).unwrap();
    assert!(link.file_type().is_symlink());
    assert_eq!(
        fs::read_link(dst.join("link")).unwrap(),
        Path::new("sub/real.txt")
    );
    assert_eq!(
        fs::read_link(dst.join("sub/up")).unwrap(),
        Path::new("../data.txt")
    );
    // The execute bit survives, and a plain file stays plain.
    let tool = fs::metadata(dst.join("tool.sh"))
        .unwrap()
        .permissions()
        .mode();
    assert_ne!(tool & 0o111, 0);
    let data = fs::metadata(dst.join("data.txt"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(data & 0o111, 0);
    // Only the top-level .git is skipped.
    assert!(!dst.join(".git").exists());
    assert!(dst.join("sub/.git/keep").exists());
    // The copy hashes like the original.
    assert_eq!(tree_sha256(&src).unwrap(), tree_sha256(&dst).unwrap());

    for (name, target) in [
        ("climbs", "../../etc"),
        ("absolute", "/etc/passwd"),
        ("deep", "sub/../../x"),
    ] {
        let hostile = tmp.path().join(name);
        write_tree(&hostile);
        symlink(target, hostile.join("evil")).unwrap();
        let error = copy_tree(&hostile, &tmp.path().join(format!("{name}-copy"))).unwrap_err();
        assert!(
            matches!(error, TreeHashError::EscapingSymlink { .. }),
            "{name}: {error:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Plans 02-20 and 02-21, gaps G-02-6 and G-02-8 (threats T-02-67, T-02-70 and
// T-02-72): replay refuses a recorded driver option that would run or load
// another program, and a write path outside the run root, before it deletes,
// copies or spawns anything. The allow-list decides: a response file or an
// unshipped spec file is HelperOption, anything else not on the list is
// OptionNotAllowed.
// ---------------------------------------------------------------------------

/// Programs a hostile compile command could make the driver run. Each one
/// leaves `marker` behind when it runs.
struct Probes {
    _dir: TempDir,
    marker: PathBuf,
    wrapper: PathBuf,
    /// A `-B` directory (with a trailing slash) holding a `cc1` that leaves the marker.
    prefix: String,
    /// A response file that holds `-I. -wrapper <wrapper>`.
    response: PathBuf,
}

fn make_executable(path: &Path, text: &str) {
    fs::write(path, text).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

fn probes() -> Probes {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let wrapper = root.join("wrapper.sh");
    make_executable(
        &wrapper,
        "#!/bin/sh\n: > \"$(dirname \"$0\")/marker\"\nexec \"$@\"\n",
    );
    let bin = root.join("bdir");
    fs::create_dir_all(&bin).unwrap();
    make_executable(
        &bin.join("cc1"),
        "#!/bin/sh\n: > \"$(dirname \"$0\")/../marker\"\nexit 1\n",
    );
    // The driver splices the file's words in place of the `@` argument, so in
    // operand position (`-I @opts.rsp`) the first word becomes the operand.
    // `-I.` is a harmless first word in both positions, and `-wrapper` then
    // runs in both.
    let response = root.join("opts.rsp");
    fs::write(&response, format!("-I. -wrapper {}\n", wrapper.display())).unwrap();
    Probes {
        marker: root.join("marker"),
        prefix: format!("{}/", bin.display()),
        wrapper,
        response,
        _dir: dir,
    }
}

/// Run the host compiler with `arguments` (after CC) inside the probe directory
/// and require that the marker appears; then remove it. A dead probe would make
/// every refusal test below vacuous.
fn assert_probe_is_live(probes: &Probes, fixture: &Fixture, arguments: &[&str]) {
    let source = fixture.run_root.join("src/a.c");
    let source = source.to_str().unwrap();
    let mut argv = vec![CC];
    argv.extend_from_slice(arguments);
    argv.extend(["-c", "-o", "probe.o", source]);
    let scratch = probes.marker.parent().unwrap();
    run(&config(), &request(&argv, scratch)).unwrap();
    assert!(
        probes.marker.exists(),
        "positive control: {arguments:?} did not run the probe"
    );
    fs::remove_file(&probes.marker).unwrap();
}

/// A one-entry plan: CC, `options`, then `-c -o out/a.o a.c` in the source directory.
fn plan_with_options(fixture: &Fixture, options: &[String]) -> ReplayPlan {
    let directory = fixture.run_root.join("src");
    let mut arguments = vec![CC.to_owned()];
    arguments.extend(options.iter().cloned());
    arguments.extend(["-c", "-o", "out/a.o", "a.c"].map(str::to_owned));
    let mut plan = fixture.plan();
    plan.entries = vec![CcEntry {
        directory: directory.to_str().unwrap().to_owned(),
        file: "a.c".to_owned(),
        arguments,
        output: directory.join("out/a.o").to_str().unwrap().to_owned(),
    }];
    plan
}

fn originals_are_present(fixture: &Fixture) {
    for key in [A_OBJECT, B_OBJECT] {
        assert!(
            fixture.run_root.join(key).is_file(),
            "{key} was deleted before the refusal"
        );
    }
}

#[test]
fn build_02_replay_refuses_helper_executing_driver_options() {
    let fixture = Fixture::new();
    let probes = probes();
    let wrapper = probes.wrapper.to_str().unwrap().to_owned();
    let prefix = probes.prefix.clone();
    let response = format!("@{}", probes.response.display());

    // The probes are live: the same argv, unguarded, runs the program. The
    // response file runs it too, also as the operand of -I (IN-15, T-02-72).
    assert_probe_is_live(&probes, &fixture, &["-wrapper", &wrapper]);
    assert_probe_is_live(&probes, &fixture, &[&format!("-B{prefix}")]);
    assert_probe_is_live(&probes, &fixture, &[&response]);
    assert_probe_is_live(&probes, &fixture, &["-I", &response]);

    // A response file or a spec file the toolchain does not ship:
    // (options, the argument the error must name).
    let helper: Vec<(Vec<String>, String)> = vec![
        (vec!["-specs=x.specs".into()], "-specs=x.specs".into()),
        (vec![response.clone()], response.clone()),
        (vec!["-I".into(), response.clone()], response.clone()),
    ];
    // Everything else is simply not on the allow-list.
    let not_allowed: Vec<(Vec<String>, String)> = vec![
        (vec!["-wrapper".into(), wrapper.clone()], "-wrapper".into()),
        (vec![format!("-B{prefix}")], format!("-B{prefix}")),
        (vec!["-B".into(), prefix.clone()], "-B".into()),
        (
            vec![format!("--prefix={prefix}")],
            format!("--prefix={prefix}"),
        ),
        (vec!["--specs".into(), "x.specs".into()], "--specs".into()),
        (
            vec!["-fplugin=/nonexistent/p.so".into()],
            "-fplugin=/nonexistent/p.so".into(),
        ),
        (
            vec!["-fplugin-arg-p-k=v".into()],
            "-fplugin-arg-p-k=v".into(),
        ),
        (
            vec!["-iplugindir=/nonexistent".into()],
            "-iplugindir=/nonexistent".into(),
        ),
        (
            vec![format!("-fcompare-debug=-B{prefix}")],
            format!("-fcompare-debug=-B{prefix}"),
        ),
        (vec![format!("-Wp,{response}")], format!("-Wp,{response}")),
        (
            vec!["-Xpreprocessor".into(), "-fplugin=/nonexistent/p.so".into()],
            "-Xpreprocessor".into(),
        ),
    ];
    let cases = helper
        .into_iter()
        .map(|case| (case, true))
        .chain(not_allowed.into_iter().map(|case| (case, false)));
    for ((options, expected), is_helper) in cases {
        let plan = plan_with_options(&fixture, &options);
        let error = replay(&config(), &plan).unwrap_err();
        let named = match &error {
            ReplayError::Entry(CcError::HelperOption { flag }) if is_helper => *flag == expected,
            ReplayError::Entry(CcError::OptionNotAllowed { flag }) if !is_helper => {
                *flag == expected
            }
            _ => false,
        };
        let variant = if is_helper {
            "HelperOption"
        } else {
            "OptionNotAllowed"
        };
        assert!(
            named,
            "{options:?}: expected {variant} {{ {expected} }}, got {error:?}"
        );
        assert!(
            !probes.marker.exists(),
            "{options:?}: the refused option still ran a program"
        );
        originals_are_present(&fixture);
    }
}

#[test]
fn build_02_replay_refuses_writes_outside_the_run_root() {
    let fixture = Fixture::new();
    let outside = tempfile::tempdir().unwrap();
    let outside_dir = outside.path().to_str().unwrap().to_owned();
    let probes = probes();

    // The positive control: unguarded, the driver does write the dependency file.
    let source = fixture.run_root.join("src/a.c");
    let dep = format!("{outside_dir}/control.d");
    run(
        &config(),
        &request(
            &[
                CC,
                "-MD",
                "-MF",
                &dep,
                "-c",
                "-o",
                "probe.o",
                source.to_str().unwrap(),
            ],
            probes.marker.parent().unwrap(),
        ),
    )
    .unwrap();
    assert!(Path::new(&dep).is_file(), "positive control: no depfile");
    fs::remove_file(&dep).unwrap();

    let directory = fixture.run_root.join("src");
    let directory_text = directory.to_str().unwrap().to_owned();
    let entry = |arguments: Vec<String>, output: String| CcEntry {
        directory: directory_text.clone(),
        file: "a.c".to_owned(),
        arguments,
        output,
    };
    let strings =
        |parts: &[&str]| -> Vec<String> { parts.iter().map(|s| (*s).to_owned()).collect() };

    // `--output=<outside>/x.o`: the real target is outside the run root.
    let target = format!("{outside_dir}/x.o");
    let by_output = entry(
        strings(&[CC, "-c", &format!("--output={target}"), "a.c"]),
        target.clone(),
    );
    // `-MD -MF <outside>/x.d`: a dependency file outside the run root.
    let depfile = format!("{outside_dir}/x.d");
    let by_depfile = entry(
        strings(&[CC, "-MD", "-MF", &depfile, "-c", "-o", "out/a.o", "a.c"]),
        directory.join("out/a.o").to_str().unwrap().to_owned(),
    );
    // `-Wp,-MD,<outside>/wp.d`: a carrier, never on the allow-list.
    let carried = format!("-Wp,-MD,{outside_dir}/wp.d");
    let by_carrier = entry(
        strings(&[CC, &carried, "-c", "-o", "out/a.o", "a.c"]),
        directory.join("out/a.o").to_str().unwrap().to_owned(),
    );

    // `--output=` and the `-Wp,` carrier are not on the allow-list; the
    // depfile is a listed option whose path is outside the run root.
    let mut plan = fixture.plan();
    plan.entries = vec![by_depfile];
    let error = replay(&config(), &plan).unwrap_err();
    assert!(
        matches!(
            &error,
            ReplayError::Entry(CcError::OutsideRunRoot { path }) if *path == depfile
        ),
        "depfile: {error:?}"
    );
    for (what, offending, flag) in [
        ("output", by_output, format!("--output={target}")),
        ("carrier", by_carrier, carried.clone()),
    ] {
        let mut plan = fixture.plan();
        plan.entries = vec![offending];
        let error = replay(&config(), &plan).unwrap_err();
        assert!(
            matches!(
                &error,
                ReplayError::Entry(CcError::OptionNotAllowed { flag: found }) if *found == flag
            ),
            "{what}: {error:?}"
        );
    }

    assert_eq!(
        fs::read_dir(outside.path()).unwrap().count(),
        0,
        "a refused entry still wrote under the outside directory"
    );
    originals_are_present(&fixture);
}
