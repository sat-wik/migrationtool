#![allow(clippy::unwrap_used, clippy::expect_used)]
#![cfg(unix)]
//! BUILD-02, plan 02-21 (gap G-02-8; review CR-01, CR-02, CR-03 and IN-15;
//! threats T-02-70 and T-02-73): the review's bypass spellings, live.
//!
//! Each case is first run unguarded with the host `/usr/bin/cc` through the
//! runner (the positive control: it does write its file outside the working
//! directory), then handed to `replay` as a one-entry plan. Replay must refuse
//! it before anything is deleted, copied or spawned, so nothing appears in the
//! outside directory and the original object stays where it is. The host
//! compiler is used so that this runs in the ci workflow too.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use mt_capture::cc::{CcEntry, CcError, CcPolicy};
use mt_capture::replay::{ReplayError, ReplayPlan, replay};
use mt_capture::treehash::sha256_hex;
use mt_toolchain::runner::{RunRequest, RunnerConfig, run};
use tempfile::TempDir;

const CC: &str = "/usr/bin/cc";
const OBJECT: &str = "src/out/a.o";
const SOURCE: &str = "int a_value(void) { return 42; }\n";

fn config() -> RunnerConfig {
    RunnerConfig::new("/usr/bin:/bin", "1791158400")
}

fn request(argv: &[String], cwd: &Path) -> RunRequest {
    RunRequest {
        tool_name: "cc".to_owned(),
        argv: argv.iter().map(OsString::from).collect(),
        cwd: cwd.to_path_buf(),
        extra_env: BTreeMap::new(),
        tool_version: None,
    }
}

fn write_source(dir: &Path) {
    fs::create_dir_all(dir.join("out")).unwrap();
    fs::write(dir.join("a.c"), SOURCE).unwrap();
}

/// `run/src/a.c` with its original object built once, a pristine copy, the
/// saved original, and a control copy of the source directory for the
/// unguarded runs.
struct Fixture {
    _tmp: TempDir,
    pristine: PathBuf,
    run_root: PathBuf,
    originals: PathBuf,
    control: PathBuf,
    hashes: BTreeMap<String, String>,
}

impl Fixture {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        let pristine = root.join("pristine");
        let run_root = root.join("run");
        let originals = root.join("originals");
        let control = root.join("control/src");
        fs::create_dir_all(&pristine).unwrap();
        fs::write(pristine.join("a.c"), SOURCE).unwrap();
        let src = run_root.join("src");
        write_source(&src);
        write_source(&control);
        let output = run(
            &config(),
            &request(&strings(&[CC, "-c", "-o", "out/a.o", "a.c"]), &src),
        )
        .unwrap();
        assert_eq!(output.record.exit.code, Some(0), "original build");
        let bytes = fs::read(run_root.join(OBJECT)).unwrap();
        let saved = originals.join(OBJECT);
        fs::create_dir_all(saved.parent().unwrap()).unwrap();
        fs::write(&saved, &bytes).unwrap();
        let mut hashes = BTreeMap::new();
        hashes.insert(OBJECT.to_owned(), sha256_hex(&bytes));
        Self {
            _tmp: tmp,
            pristine,
            run_root,
            originals,
            control,
            hashes,
        }
    }

    /// A one-entry plan whose output is the object the arguments write.
    fn plan(&self, arguments: Vec<String>) -> ReplayPlan {
        let directory = self.run_root.join("src");
        ReplayPlan {
            target: "bypass".to_owned(),
            config: "default".to_owned(),
            run_root: self.run_root.clone(),
            pristine_src: self.pristine.clone(),
            src_dir_name: "src".to_owned(),
            keep_dirs: Vec::new(),
            entries: vec![CcEntry {
                directory: directory.to_str().unwrap().to_owned(),
                file: "a.c".to_owned(),
                arguments,
                output: directory.join("out/a.o").to_str().unwrap().to_owned(),
            }],
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

    fn original_is_present(&self) -> bool {
        self.run_root.join(OBJECT).is_file()
    }
}

fn strings(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| (*part).to_owned()).collect()
}

/// The names in `dir`, sorted.
fn listing(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn clear(dir: &Path) {
    for name in listing(dir) {
        fs::remove_file(dir.join(name)).unwrap();
    }
}

/// One case: its label, the file it writes under the outside directory, and
/// its argv after CC.
struct Case {
    label: &'static str,
    writes: &'static str,
    arguments: Vec<String>,
}

/// `[<options>, -c, -o, out/a.o, a.c]`.
fn compile_with(options: &[String]) -> Vec<String> {
    let mut arguments = options.to_vec();
    arguments.extend(strings(&["-c", "-o", "out/a.o", "a.c"]));
    arguments
}

fn cases(outside: &str) -> Vec<Case> {
    let path = |name: &str| format!("{outside}/{name}");
    vec![
        Case {
            label: "CR-01 --dump-final-insns=",
            writes: "f",
            arguments: compile_with(&[format!("--dump-final-insns={}", path("f"))]),
        },
        Case {
            label: "CR-01 --opt-info-all=",
            writes: "o",
            arguments: compile_with(&[format!("--opt-info-all={}", path("o"))]),
        },
        Case {
            label: "CR-02 -Wa,-MD,",
            writes: "e1.d",
            arguments: compile_with(&[format!("-Wa,-MD,{}", path("e1.d"))]),
        },
        Case {
            label: "CR-02 -Wa,-MD=",
            writes: "e2.d",
            arguments: compile_with(&[format!("-Wa,-MD={}", path("e2.d"))]),
        },
        Case {
            label: "CR-02 -Wa,--M,",
            writes: "e3.d",
            arguments: compile_with(&[format!("-Wa,--M,{}", path("e3.d"))]),
        },
        Case {
            label: "CR-02 -Xassembler -MD -Xassembler",
            writes: "x1.d",
            arguments: compile_with(&[
                "-Xassembler".to_owned(),
                "-MD".to_owned(),
                "-Xassembler".to_owned(),
                path("x1.d"),
            ]),
        },
        Case {
            label: "CR-03 -MD -MT -c ... -Wl,-Map=",
            writes: "m.map",
            arguments: vec![
                "-MD".to_owned(),
                "-MT".to_owned(),
                "-c".to_owned(),
                "a.c".to_owned(),
                "-o".to_owned(),
                "out/a.o".to_owned(),
                "-nostdlib".to_owned(),
                "-nostartfiles".to_owned(),
                format!("-Wl,-Map={}", path("m.map")),
                "-Wl,-e,0".to_owned(),
            ],
        },
    ]
}

#[test]
fn build_02_replay_refuses_the_measured_bypasses() {
    let fixture = Fixture::new();
    let mut failures: Vec<String> = Vec::new();
    for index in 0..cases("/unused").len() {
        // A fresh outside directory per case.
        let outside = tempfile::tempdir().unwrap();
        let outside_dir = outside.path().to_str().unwrap().to_owned();
        let case = cases(&outside_dir).swap_remove(index);
        let mut argv = vec![CC.to_owned()];
        argv.extend(case.arguments.iter().cloned());

        // Positive control: unguarded, the driver writes the file.
        run(&config(), &request(&argv, &fixture.control)).unwrap();
        assert!(
            outside.path().join(case.writes).is_file(),
            "positive control {}: {:?} did not write {} (found {:?})",
            case.label,
            case.arguments,
            case.writes,
            listing(outside.path())
        );
        clear(outside.path());

        // Replay refuses it before anything runs.
        let result = replay(&config(), &fixture.plan(argv));
        let wrote = listing(outside.path());
        let refused = matches!(result, Err(ReplayError::Entry(_)));
        if !refused || !wrote.is_empty() {
            let returned = match &result {
                Ok(_) => "Ok(..)".to_owned(),
                Err(error) => format!("Err({error:?})"),
            };
            failures.push(format!(
                "{}: replay returned {returned} and wrote {wrote:?}",
                case.label
            ));
        }
        if !fixture.original_is_present() {
            failures.push(format!(
                "{}: the original object was deleted before the refusal",
                case.label
            ));
        }
    }

    // The minimal CR-03 vector is not a compile-only command.
    let minimal = strings(&[CC, "-MD", "-MT", "-c", "a.c", "-o", "out/a.o"]);
    match replay(&config(), &fixture.plan(minimal)) {
        Err(ReplayError::Entry(CcError::NotCompileOnly)) => {}
        Ok(_) => failures.push("minimal CR-03: replay returned Ok(..)".to_owned()),
        Err(error) => failures.push(format!(
            "minimal CR-03: expected Entry(NotCompileOnly), got {error:?}"
        )),
    }
    if !fixture.original_is_present() {
        failures.push("minimal CR-03: the original object was deleted".to_owned());
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
