#![allow(clippy::unwrap_used, clippy::expect_used)]
#![cfg(unix)]
//! BED-01 entry-function check (CONTEXT D-23, D-24).
//!
//! `tests/fixtures/ast/probe-dump.json` is a real clang dump of a probe file.
//! It was generated once on the development host, with Ubuntu clang 18.1.3,
//! from a directory holding `entry_probe.c` (`#include "tb.h"`) and `tb.h`
//! (`typedef unsigned long tb_size;`, `int tb_parse(const unsigned char *buf,
//! tb_size len);`, `int tb_parse_reader(void *r);`) with
//!
//! ```text
//! clang --target=arm-none-eabi --sysroot=/usr/lib/arm-none-eabi -fsyntax-only \
//!   -Xclang -ast-dump=json -Xclang -ast-dump-filter=tb_parse -I. \
//!   -mcpu=cortex-m4 -mthumb -mfloat-abi=hard -mfpu=fpv4-sp-d16 entry_probe.c
//! ```
//!
//! The filter `tb_parse` also dumped `tb_parse_reader`, so the file holds two
//! JSON documents back to back. The pinned image's clang 16 dump is published
//! by the plan 02-06 workflow and committed by plan 02-10.
//!
//! The parsing, flag filtering and command lines are proved as data. The checks
//! that run clang use the host's clang and Arm sysroot and skip themselves,
//! visibly, when this host lacks them (`support::local_arm_tools`).

mod support;

use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use mt_capture::capture::{RunLayout, ToolPaths, capture};
use mt_capture::cc::CcEntry;
use mt_capture::entries::{
    EntryError, check_entry, clang_ast_argv, clang_deps_argv, filter_flags, normalise_signature,
    parse_ast_stream, parse_make_deps, probe_source,
};
use mt_capture::fetch::fetch;
use mt_capture::manifest::{BuildArgs, BuildSystem, Config, Entry, EntryForm};
use mt_capture::report::CaptureVerdict;
use support::{
    capture_context, fetch_context, git_target, local_arm_tools, make_upstream, runner_config,
};

const DUMP: &[u8] = include_bytes!("fixtures/ast/probe-dump.json");

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

fn image_tools() -> ToolPaths {
    ToolPaths {
        arm_gcc: PathBuf::from("/opt/arm-gnu-toolchain/bin/arm-none-eabi-gcc"),
        bear: PathBuf::from("/usr/bin/bear"),
        cmake: PathBuf::from("/usr/bin/cmake"),
        make: PathBuf::from("/usr/bin/make"),
        clang: PathBuf::from("/usr/lib/llvm-16/bin/clang"),
        readelf: PathBuf::from("/opt/arm-gnu-toolchain/bin/arm-none-eabi-readelf"),
        objcopy: PathBuf::from("/opt/arm-gnu-toolchain/bin/arm-none-eabi-objcopy"),
        sysroot: PathBuf::from("/opt/arm-gnu-toolchain/arm-none-eabi"),
        git: PathBuf::from("/usr/bin/git"),
        versions: BTreeMap::new(),
    }
}

#[test]
fn bed_01_entry_ast_stream_parses_concatenated_documents() {
    assert!(DUMP.windows(2).filter(|w| w == b"}\n").count() >= 2);

    let declarations = parse_ast_stream(DUMP).unwrap();

    let summary: Vec<(&str, &str, Option<&str>)> = declarations
        .iter()
        .map(|d| (d.name.as_str(), d.qual_type.as_str(), d.file.as_deref()))
        .collect();
    assert_eq!(
        summary,
        [
            (
                "tb_parse",
                "int (const unsigned char *, tb_size)",
                Some("./tb.h")
            ),
            ("tb_parse_reader", "int (void *)", Some("./tb.h")),
        ]
    );

    // clang prints loc.file only when it changes: the second document carries
    // the file of the first, and a later file replaces it.
    let stream = br#"
{"kind":"FunctionDecl","name":"a","loc":{"file":"a.h","line":1},"type":{"qualType":"int (void)"}}
{"kind":"FunctionDecl","name":"b","loc":{"line":2},"type":{"qualType":"int (void)"}}
{"kind":"TranslationUnitDecl","inner":[
  {"kind":"VarDecl","name":"v","loc":{"file":"b.h"},"type":{"qualType":"int"}},
  {"kind":"FunctionDecl","name":"c","loc":{"line":9},"type":{"qualType":"void (void)"},"inner":[
    {"kind":"ParmVarDecl","name":"a","type":{"qualType":"int"}}]}]}"#;
    let carried = parse_ast_stream(stream).unwrap();
    let files: Vec<(&str, Option<&str>)> = carried
        .iter()
        .map(|d| (d.name.as_str(), d.file.as_deref()))
        .collect();
    assert_eq!(
        files,
        [("a", Some("a.h")), ("b", Some("a.h")), ("c", Some("b.h"))]
    );

    // Nothing printed, or no declaration in it, is no declaration.
    assert!(parse_ast_stream(b"").unwrap().is_empty());
    assert!(parse_ast_stream(b"  \n").unwrap().is_empty());
    // Text that is not JSON is an error, never an empty success.
    assert!(matches!(
        parse_ast_stream(b"{\"kind\": \"FunctionDecl\", \"na"),
        Err(EntryError::Ast(_))
    ));
    assert!(matches!(
        parse_ast_stream(b"error: unknown type name"),
        Err(EntryError::Ast(_))
    ));
}

#[test]
fn bed_01_entry_filter_selects_exact_name_not_substring() {
    let declarations = parse_ast_stream(DUMP).unwrap();

    // The filter `tb_parse` dumped both functions.
    assert_eq!(declarations.len(), 2);
    let exact: Vec<&str> = declarations
        .iter()
        .filter(|d| d.name == "tb_parse")
        .map(|d| d.qual_type.as_str())
        .collect();
    assert_eq!(exact, ["int (const unsigned char *, tb_size)"]);

    let tools = image_tools();
    let argv = clang_ast_argv(
        &tools,
        "tb_parse",
        &strings(&["-I.", "-mthumb"]),
        Path::new("/cap/entry_probe.c"),
    )
    .unwrap();
    assert_eq!(
        argv,
        [
            "/usr/lib/llvm-16/bin/clang",
            "--target=arm-none-eabi",
            "--sysroot=/opt/arm-gnu-toolchain/arm-none-eabi",
            "-fsyntax-only",
            "-Xclang",
            "-ast-dump=json",
            "-Xclang",
            "-ast-dump-filter=tb_parse",
            "-I.",
            "-mthumb",
            "/cap/entry_probe.c",
        ]
    );
    // The name reaches clang's command line, so only an identifier is allowed.
    for bad in ["", "a b", "a;b", "-x", "1a", "a/b"] {
        assert!(
            matches!(
                clang_ast_argv(&tools, bad, &[], Path::new("/cap/p.c")),
                Err(EntryError::InvalidInput(_))
            ),
            "{bad:?}"
        );
    }
    let deps = clang_deps_argv(&tools, &strings(&["-DX"]), "src/t.c").unwrap();
    assert_eq!(
        deps,
        [
            "/usr/lib/llvm-16/bin/clang",
            "--target=arm-none-eabi",
            "--sysroot=/opt/arm-gnu-toolchain/arm-none-eabi",
            "-MM",
            "-DX",
            "src/t.c",
        ]
    );
    assert_eq!(
        probe_source(Path::new("/work/run/t/default/src/inc/t.h")).unwrap(),
        "#include \"/work/run/t/default/src/inc/t.h\"\n"
    );
    for bad in ["inc/t.h", "/a\"b.h", "/a\\b.h", "/a\nb.h"] {
        assert!(probe_source(Path::new(bad)).is_err(), "{bad:?}");
    }
}

#[test]
fn bed_01_entry_signature_compare_collapses_whitespace() {
    assert_eq!(
        normalise_signature("int (const unsigned char *,  unsigned long)"),
        normalise_signature("int (const unsigned char *, unsigned long)")
    );
    assert_eq!(
        normalise_signature("  int\t(const unsigned char *,\n unsigned long) "),
        "int (const unsigned char *, unsigned long)"
    );
    // Only whitespace runs collapse: different types stay different.
    assert_ne!(
        normalise_signature("int (const unsigned char *, unsigned long)"),
        normalise_signature("int (const unsigned char *, tb_size)")
    );
    assert_ne!(
        normalise_signature("int (unsigned char *)"),
        normalise_signature("int (unsigned char*)")
    );
}

#[test]
fn bed_01_entry_missing_function_is_not_found() {
    // Empty stdout with exit 0 is what clang prints when nothing matched.
    let declarations = parse_ast_stream(b"").unwrap();
    assert!(!declarations.iter().any(|d| d.name == "tb_missing"));

    // A name that only contains the filter is not the function.
    let declarations = parse_ast_stream(DUMP).unwrap();
    assert!(!declarations.iter().any(|d| d.name == "tb_pars"));
    assert!(!declarations.iter().any(|d| d.name == "reader"));

    let Some(world) = World::new(&[]) else { return };
    let missing = world.check("t_missing", "inc/t.h", "t.c", "int (void)");
    assert!(!missing.result.found, "{:?}", missing.result);
    assert!(!missing.result.passed());
    assert!(
        missing
            .result
            .detail
            .as_deref()
            .unwrap()
            .contains("no function named t_missing")
    );

    // The defining translation unit is not in the capture.
    let absent = world.check("t_parse", "inc/t.h", "nowhere.c", "int (void)");
    assert!(!absent.result.found);
    assert_eq!(
        absent.result.detail.as_deref(),
        Some("defining TU not captured")
    );
    assert!(absent.steps.is_empty());
}

#[test]
fn bed_01_entry_flags_keep_only_clang_safe_options() {
    let arguments = strings(&[
        "/opt/arm-gnu-toolchain/bin/arm-none-eabi-gcc",
        "-DQ=1",
        "-D",
        "R",
        "-UX",
        "-I",
        "inc",
        "-Isrc",
        "-isystem",
        "/sys",
        "-iquote",
        "q",
        "-include",
        "cfg.h",
        "-std=gnu11",
        "-ffreestanding",
        "-fsigned-char",
        "-funsigned-char",
        "-fshort-enums",
        "-mcpu=cortex-m4",
        "-mthumb",
        "-mfloat-abi=hard",
        "-mfpu=fpv4-sp-d16",
        "-march=armv7e-m",
        "-Wall",
        "-Werror",
        "-Os",
        "-O3",
        "-g",
        "-fno-common",
        "-fstack-usage",
        "-ffile-prefix-map=/a=/b",
        "-MMD",
        "-MF",
        "t.d",
        "-c",
        "t.c",
        "-o",
        "t.o",
    ]);

    let kept = filter_flags(&arguments);

    assert_eq!(
        kept,
        strings(&[
            "-DQ=1",
            "-D",
            "R",
            "-UX",
            "-I",
            "inc",
            "-Isrc",
            "-isystem",
            "/sys",
            "-iquote",
            "q",
            "-include",
            "cfg.h",
            "-std=gnu11",
            "-ffreestanding",
            "-fsigned-char",
            "-funsigned-char",
            "-fshort-enums",
            "-mcpu=cortex-m4",
            "-mthumb",
            "-mfloat-abi=hard",
            "-mfpu=fpv4-sp-d16",
            "-march=armv7e-m",
        ])
    );
    // The attached forms of the separate-operand options are kept too.
    assert_eq!(
        filter_flags(&strings(&[
            "-isystem/sys",
            "-iquotex",
            "-includecfg.h",
            "-D"
        ])),
        strings(&["-isystem/sys", "-iquotex", "-includecfg.h"])
    );
    assert!(filter_flags(&[]).is_empty());
}

#[test]
fn bed_01_entry_header_must_be_a_dependency() {
    assert_eq!(parse_make_deps("t.o: t.c inc/t.h\n"), ["t.c", "inc/t.h"]);
    assert_eq!(
        parse_make_deps("t.o: t.c \\\n  inc/t.h \\\n  /abs/x.h\n"),
        ["t.c", "inc/t.h", "/abs/x.h"]
    );
    assert_eq!(
        parse_make_deps("t.o: dir\\ with\\ space/t.h t.c"),
        ["dir with space/t.h", "t.c"]
    );
    assert!(parse_make_deps("t.o:\n").is_empty());
    assert!(parse_make_deps("").is_empty());
    assert!(parse_make_deps("no rule here").is_empty());

    let Some(world) = World::new(&[]) else { return };

    // Declared, defined and included: everything passes, whitespace aside.
    let good = world.check(
        "t_parse",
        "inc/t.h",
        "t.c",
        "int (const unsigned char *,   t_size)",
    );
    assert!(good.result.passed(), "{:?}", good.result);
    assert_eq!(
        good.result.found_signature.as_deref(),
        Some("int (const unsigned char *, t_size)")
    );
    assert!(good.result.detail.is_none());
    assert!(!good.not_proven);
    let names: Vec<&str> = good.steps.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["clang ast t_parse", "clang deps t_parse"]);
    // t_parse_reader is also in the header: the exact-name filter ignores it.
    let reader = world.check("t_parse_reader", "inc/t.h", "t.c", "int (void *)");
    assert!(reader.result.passed(), "{:?}", reader.result);

    // A different type is found but does not match.
    let wrong = world.check(
        "t_parse",
        "inc/t.h",
        "t.c",
        "int (const unsigned char *, unsigned long)",
    );
    assert!(wrong.result.found && wrong.result.header_included);
    assert!(!wrong.result.signature_matches);
    assert!(!wrong.result.passed());
    assert!(
        wrong
            .result
            .detail
            .as_deref()
            .unwrap()
            .contains("signature mismatch")
    );

    // The function exists in a header the defining TU never includes.
    let stray = world.check("t_other", "inc/other.h", "t.c", "int (void)");
    assert!(stray.result.found && stray.result.signature_matches);
    assert!(!stray.result.header_included);
    assert!(!stray.result.passed());
    assert!(
        stray
            .result
            .detail
            .as_deref()
            .unwrap()
            .contains("not a dependency")
    );
}

#[test]
fn bed_01_entry_a_failing_or_stuck_clang_never_passes() {
    let Some(mut world) = World::new(&[]) else {
        return;
    };

    // A clang that exits non-zero.
    world.tools.clang = PathBuf::from("/usr/bin/false");
    let failed = world.check("t_parse", "inc/t.h", "t.c", "int (void)");
    assert!(!failed.result.found && !failed.result.passed());
    assert!(!failed.not_proven);
    assert!(
        failed
            .result
            .detail
            .as_deref()
            .unwrap()
            .contains("exited with Some(1)")
    );

    // A clang that outlives the limit is not proven, not failed.
    let script = world.dir.path().join("slow-clang");
    fs::write(&script, "#!/bin/sh\nexec sleep 30\n").unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    world.tools.clang = script;
    world.timeout = Duration::from_millis(300);
    let stuck = world.check("t_parse", "inc/t.h", "t.c", "int (void)");
    assert!(stuck.not_proven);
    assert!(!stuck.result.passed());
    assert!(
        stuck
            .result
            .detail
            .as_deref()
            .unwrap()
            .starts_with("not_proven: ")
    );
    assert!(stuck.steps.iter().all(|s| s.run.exit.timed_out));
}

#[test]
fn bed_01_entry_mismatch_fails_the_capture() {
    let Some(tools) = local_arm_tools() else {
        return;
    };
    let upstream = make_upstream(&[
        ("t.h", "int t_add(int a, int b);\n"),
        (
            "t.c",
            "#include \"t.h\"\nint t_add(int a, int b) { return a + b; }\n",
        ),
        (
            "Makefile",
            "all: t.o\nt.o: t.c t.h\n\t$(CC) -c t.c -o t.o\n",
        ),
    ]);
    let work = tempfile::tempdir().unwrap();
    let testbed = tempfile::tempdir().unwrap();
    let ctx = capture_context(work.path(), testbed.path(), tools);
    let config = Config {
        name: "default".to_owned(),
        ..Config::default()
    };
    let entry = |name: &str, signature: &str| Entry {
        name: name.to_owned(),
        header: "t.h".to_owned(),
        source: "t.c".to_owned(),
        form: EntryForm::Direct,
        signature: signature.to_owned(),
        accepted_wrapped: false,
        arg: Vec::new(),
    };
    let mut target = git_target(&upstream, BuildSystem::Make);
    target.build_args = Some(BuildArgs {
        dir: ".".to_owned(),
        goals: vec!["all".to_owned()],
        make_vars: vec!["CC={cc} {cflags}".to_owned()],
        cmake_target: None,
        cmake_options: Vec::new(),
    });
    fetch(
        &runner_config(),
        &fetch_context(work.path()),
        "demo",
        &target,
    )
    .unwrap();

    // Every entry matches: the capture passes and the report names each check.
    target.entry = vec![entry("t_add", "int (int,  int)")];
    let report = capture(&runner_config(), &ctx, "demo", &target, &config).unwrap();
    assert_eq!(
        report.verdict,
        CaptureVerdict::Captured,
        "{:?}",
        report.causes
    );
    assert!(report.entries["t_add"].passed());
    assert!(report.steps.iter().any(|s| s.name == "clang ast t_add"));
    assert!(report.steps.iter().any(|s| s.name == "clang deps t_add"));

    // A wrong signature, a missing function and an uncaptured TU each fail it.
    let mut wrong = entry("t_add", "long (int, int)");
    wrong.form = EntryForm::Direct;
    let mut missing = entry("t_sub", "int (int, int)");
    missing.form = EntryForm::Direct;
    let mut uncaptured = entry("t_other", "int (void)");
    uncaptured.source = "other.c".to_owned();
    for (bad, detail) in [
        (wrong, "signature mismatch"),
        (missing, "no function named t_sub"),
        (uncaptured, "defining TU not captured"),
    ] {
        let name = bad.name.clone();
        target.entry = vec![entry("t_add", "int (int, int)"), bad];
        let report = capture(&runner_config(), &ctx, "demo", &target, &config).unwrap();
        assert_eq!(report.verdict, CaptureVerdict::CaptureFailed, "{name}");
        assert_eq!(report.causes, ["entry_mismatch"], "{name}");
        assert!(
            report.cause_details[0].contains(detail),
            "{:?}",
            report.cause_details
        );
        assert!(!report.entries[&name].passed(), "{name}");
        assert!(report.entries["t_add"].passed() || name == "t_add");
    }
}

/// A tiny captured project under a temporary work directory.
struct World {
    dir: tempfile::TempDir,
    tools: ToolPaths,
    layout: RunLayout,
    canonical: Vec<CcEntry>,
    timeout: Duration,
}

impl World {
    fn new(extra: &[(&str, &str)]) -> Option<Self> {
        let tools = local_arm_tools()?;
        let dir = tempfile::tempdir().unwrap();
        let layout = RunLayout::new(dir.path(), "demo", "default");
        let mut files = vec![
            (
                "inc/t.h",
                "typedef unsigned long t_size;\n\
                 int t_parse(const unsigned char *buf, t_size len);\n\
                 int t_parse_reader(void *r);\n",
            ),
            ("inc/other.h", "int t_other(void);\n"),
            (
                "t.c",
                "#include \"t.h\"\n\
                 int t_parse(const unsigned char *buf, t_size len) { return buf[0] + (int)len; }\n\
                 int t_parse_reader(void *r) { return r != 0; }\n",
            ),
        ];
        files.extend_from_slice(extra);
        for (name, text) in files {
            let path = layout.src.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
        fs::create_dir_all(&layout.capture_dir).unwrap();
        let src = layout.src.to_str().unwrap().to_owned();
        let canonical = vec![CcEntry {
            directory: src.clone(),
            file: "t.c".to_owned(),
            arguments: strings(&[
                "/usr/bin/arm-none-eabi-gcc",
                "-mcpu=cortex-m4",
                "-mthumb",
                "-mfloat-abi=hard",
                "-mfpu=fpv4-sp-d16",
                "-Os",
                "-Wall",
                "-I",
                "inc",
                "-DT_FLAG=1",
                "-c",
                "t.c",
                "-o",
                &format!("{src}/t.o"),
            ]),
            output: format!("{src}/t.o"),
        }];
        Some(Self {
            dir,
            tools,
            layout,
            canonical,
            timeout: Duration::from_secs(120),
        })
    }

    fn check(
        &self,
        name: &str,
        header: &str,
        source: &str,
        signature: &str,
    ) -> mt_capture::entries::EntryCheck {
        let mut cfg = runner_config();
        cfg.timeout = self.timeout;
        let env = BTreeMap::from([
            (
                "HOME".to_owned(),
                self.dir.path().to_str().unwrap().to_owned(),
            ),
            (
                "TMPDIR".to_owned(),
                self.dir.path().to_str().unwrap().to_owned(),
            ),
        ]);
        let entry = Entry {
            name: name.to_owned(),
            header: header.to_owned(),
            source: source.to_owned(),
            form: EntryForm::Direct,
            signature: signature.to_owned(),
            accepted_wrapped: false,
            arg: Vec::new(),
        };
        check_entry(
            &cfg,
            &self.tools,
            &self.layout,
            &env,
            &self.canonical,
            &entry,
        )
        .unwrap()
    }
}
