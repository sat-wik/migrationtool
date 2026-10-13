#![allow(clippy::unwrap_used, clippy::expect_used)]
#![cfg(unix)]
//! BUILD-01 capture drivers (CONTEXT D-05, D-06, D-07, D-22).
//!
//! The command lines and the toolchain file are proved as data, against the
//! committed manifest's qcbor (Make) and corejson (CMake) recipes. The silent
//! wrong-build gates and the whole `capture()` pipeline are proved end to end
//! on a local upstream when this host has the Arm compiler, Bear and CMake
//! (see `support::local_arm_tools`); otherwise those parts skip themselves and
//! plan 02-06 proves them in the pinned image.

mod support;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use mt_capture::capture::{
    CaptureContext, CaptureError, Expansion, RunLayout, ToolPaths, capture, cmake_build_argv,
    cmake_configure_argv, effective_cflags, expand, is_cmake_product, list_files, make_argv,
    timestamp_macro_sources, toolchain_file, undeclared_objects, variant_code_diff,
};
use mt_capture::cc::CcEntry;
use mt_capture::elf::code_digest;
use mt_capture::fetch::fetch;
use mt_capture::manifest::{BASELINE_CFLAGS, BuildArgs, BuildSystem, Config, Manifest, Target};
use mt_capture::report::{CaptureReport, CaptureVerdict, CapturedObject};
use mt_toolchain::pins::Pins;
use support::{
    capture_context, fetch_context, git_target, local_arm_tools, make_upstream, runner_config, tool,
};

const ARM_GCC: &str = "/opt/arm-gnu-toolchain/bin/arm-none-eabi-gcc";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn committed() -> Manifest {
    Manifest::load(&repo_root().join("testbed/manifest.toml")).unwrap()
}

fn config_of<'a>(target: &'a Target, name: &str) -> &'a Config {
    target.config.iter().find(|c| c.name == name).unwrap()
}

fn baseline_vec() -> Vec<String> {
    BASELINE_CFLAGS.iter().map(|f| (*f).to_owned()).collect()
}

/// The tool paths of the pinned image, as plain data (nothing is run).
fn image_tools() -> ToolPaths {
    ToolPaths {
        arm_gcc: PathBuf::from(ARM_GCC),
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
fn build_01_tool_paths_and_layout_follow_the_pins() {
    let pins = Pins::load(&repo_root().join("container/pins.toml")).unwrap();

    let tools = ToolPaths::from_pins(&pins).unwrap();

    assert_eq!(tools.arm_gcc, Path::new(ARM_GCC));
    assert_eq!(tools.readelf, image_tools().readelf);
    assert_eq!(tools.objcopy, image_tools().objcopy);
    assert_eq!(tools.sysroot, image_tools().sysroot);
    assert_eq!(tools.bear, Path::new("/usr/bin/bear"));
    assert_eq!(tools.make, Path::new("/usr/bin/make"));
    assert_eq!(tools.cmake, Path::new("/usr/bin/cmake"));
    assert_eq!(tools.clang, Path::new("/usr/lib/llvm-16/bin/clang"));
    assert_eq!(tools.git, Path::new("/usr/bin/git"));
    assert_eq!(tools.versions["arm_gcc"], "14.3.Rel1");
    assert_eq!(tools.versions["bear"], "3.1.1");
    assert_eq!(tools.versions["make"], "4.3");

    let layout = RunLayout::new(Path::new("/work"), "qcbor", "no-hw-float");
    assert_eq!(layout.run_root, Path::new("/work/run/qcbor/no-hw-float"));
    assert_eq!(layout.src, Path::new("/work/run/qcbor/no-hw-float/src"));
    assert_eq!(layout.build, Path::new("/work/run/qcbor/no-hw-float/build"));
    assert_eq!(
        layout.inputs,
        Path::new("/work/run/qcbor/no-hw-float/inputs")
    );
    assert_eq!(
        layout.capture_dir,
        Path::new("/work/capture/qcbor/no-hw-float")
    );
    assert_eq!(
        layout.objects_dir,
        Path::new("/work/capture/qcbor/no-hw-float/objects")
    );
}

#[test]
fn build_01_toolchain_file_carries_baseline_flags() {
    let manifest = committed();
    let corejson = &manifest.target["corejson"];
    let variant = config_of(corejson, "depth-8");
    let cflags = effective_cflags(&baseline_vec(), variant);

    let text = toolchain_file(&image_tools(), &cflags).unwrap();

    assert!(text.contains("set(CMAKE_SYSTEM_NAME Generic)"), "{text}");
    assert!(
        text.contains("set(CMAKE_TRY_COMPILE_TARGET_TYPE STATIC_LIBRARY)"),
        "{text}"
    );
    for (variable, program) in [
        ("CMAKE_C_COMPILER", "arm-none-eabi-gcc"),
        ("CMAKE_AR", "arm-none-eabi-ar"),
        ("CMAKE_RANLIB", "arm-none-eabi-ranlib"),
    ] {
        let line = format!("set({variable} \"/opt/arm-gnu-toolchain/bin/{program}\")");
        assert!(text.contains(&line), "missing {line} in {text}");
    }
    let flags = "set(CMAKE_C_FLAGS_INIT \"-mcpu=cortex-m4 -mthumb -mfloat-abi=hard \
                 -mfpu=fpv4-sp-d16 -Os -DJSON_MAX_DEPTH=8\")";
    assert!(text.contains(flags), "missing {flags} in {text}");
    // The default configuration has no define.
    let default = effective_cflags(&baseline_vec(), config_of(corejson, "default"));
    let default_text = toolchain_file(&image_tools(), &default).unwrap();
    assert!(
        default_text.contains("-mfpu=fpv4-sp-d16 -Os\")"),
        "{default_text}"
    );
    assert!(!default_text.contains("JSON_MAX_DEPTH"));
    // Deterministic, with no timestamp or host path.
    assert_eq!(text, toolchain_file(&image_tools(), &cflags).unwrap());
    assert!(!text.contains("/home/"));
}

#[test]
fn build_01_toolchain_file_refuses_flags_it_cannot_quote() {
    for flag in ["-DA;B", "-DA\nB", "-D\"A\"", "-DA$B", "-DA\\B"] {
        let error = toolchain_file(&image_tools(), &[flag.to_owned()]).unwrap_err();
        assert!(
            matches!(error, CaptureError::InvalidInput(_)),
            "{flag:?}: {error:?}"
        );
    }
}

fn cmake_recipe() -> (Target, BuildArgs) {
    let manifest = committed();
    let target = manifest.target["corejson"].clone();
    let args = target.build_args.clone().unwrap();
    (target, args)
}

#[test]
fn build_01_cmake_configure_never_sets_cmake_c_flags() {
    let (target, args) = cmake_recipe();
    let layout = RunLayout::new(Path::new("/work"), "corejson", "depth-8");
    let variant = config_of(&target, "depth-8");
    let cflags = effective_cflags(&baseline_vec(), variant);

    let argv = cmake_configure_argv(&image_tools(), &layout, &cflags, &args, variant).unwrap();

    assert_eq!(argv[0], "/usr/bin/cmake");
    assert_eq!(
        argv[1..8],
        [
            "-S",
            "/work/run/corejson/depth-8/src",
            "-B",
            "/work/run/corejson/depth-8/build",
            "-G",
            "Unix Makefiles",
            "-DCMAKE_TOOLCHAIN_FILE=/work/capture/corejson/depth-8/toolchain.cmake",
        ]
    );
    assert!(argv.contains(&"-DCMAKE_EXPORT_COMPILE_COMMANDS=ON".to_owned()));
    assert!(
        !argv.iter().any(|arg| arg.starts_with("-DCMAKE_C_FLAGS")),
        "{argv:?}"
    );
    // The configuration's define travels in the toolchain file, not in the argv.
    assert!(
        !argv.iter().any(|arg| arg.contains("JSON_MAX_DEPTH")),
        "{argv:?}"
    );

    // A manifest option may not set what the driver owns.
    for option in [
        "-DCMAKE_C_FLAGS=-DX",
        "-DCMAKE_C_FLAGS:STRING=-DX",
        "-DCMAKE_C_FLAGS_RELEASE=-O3",
        "-DCMAKE_BUILD_TYPE=Release",
        "-DCMAKE_TOOLCHAIN_FILE=/x.cmake",
        "-DCMAKE_C_COMPILER=/usr/bin/cc",
        "-D",
    ] {
        let mut bad = args.clone();
        bad.cmake_options.push(option.to_owned());
        let error = cmake_configure_argv(&image_tools(), &layout, &cflags, &bad, variant);
        assert!(
            matches!(error, Err(CaptureError::InvalidInput(_))),
            "{option} was accepted: {error:?}"
        );
        let mut bad_config = variant.clone();
        bad_config.cmake_options.push(option.to_owned());
        assert!(
            cmake_configure_argv(&image_tools(), &layout, &cflags, &args, &bad_config).is_err(),
            "{option} was accepted from a configuration"
        );
    }
}

#[test]
fn build_01_cmake_build_type_is_minsizerel() {
    let (target, args) = cmake_recipe();
    let layout = RunLayout::new(Path::new("/work"), "corejson", "default");
    let config = config_of(&target, "default");
    let cflags = effective_cflags(&baseline_vec(), config);

    let configure = cmake_configure_argv(&image_tools(), &layout, &cflags, &args, config).unwrap();
    let build = cmake_build_argv(&image_tools(), &layout, &args, config).unwrap();

    assert_eq!(
        configure
            .iter()
            .filter(|arg| arg.starts_with("-DCMAKE_BUILD_TYPE"))
            .collect::<Vec<_>>(),
        ["-DCMAKE_BUILD_TYPE=MinSizeRel"]
    );
    assert_eq!(
        build,
        [
            "/usr/bin/cmake",
            "--build",
            "/work/run/corejson/default/build",
            "--target",
            "core_json",
            "-j1",
        ]
    );
    // A configuration may name another target.
    let mut other = config.clone();
    other.cmake_target = Some("other".to_owned());
    let argv = cmake_build_argv(&image_tools(), &layout, &args, &other).unwrap();
    assert_eq!(argv[4], "other");
    // No target at all is an invalid input.
    let mut none = args.clone();
    none.cmake_target = None;
    assert!(cmake_build_argv(&image_tools(), &layout, &none, config).is_err());
}

#[test]
fn build_01_make_argv_injects_flags_through_cc_and_runs_j1() {
    let manifest = committed();
    let qcbor = &manifest.target["qcbor"];
    let args = qcbor.build_args.clone().unwrap();
    let variant = config_of(qcbor, "no-hw-float");
    let layout = RunLayout::new(Path::new("/work"), "qcbor", "no-hw-float");
    let cflags = effective_cflags(&baseline_vec(), variant);

    let argv = make_argv(&image_tools(), &layout, &cflags, &args, variant).unwrap();

    assert_eq!(
        argv,
        [
            "/usr/bin/bear",
            "--output",
            "/work/capture/qcbor/no-hw-float/compile_commands.raw.json",
            "--",
            "/usr/bin/make",
            "-j1",
            &format!(
                "CC={ARM_GCC} -mcpu=cortex-m4 -mthumb -mfloat-abi=hard -mfpu=fpv4-sp-d16 -Os \
                 -DQCBOR_DISABLE_FLOAT_HW_USE -DQCBOR_DISABLE_PREFERRED_FLOAT"
            ),
            "libqcbor.a",
        ]
    );
    // The Makefile keeps its own CFLAGS: none is set unless the manifest does.
    assert!(!argv.iter().any(|arg| arg.starts_with("CFLAGS=")));
    // Configuration make variables and goals follow the target's.
    let mut extra = variant.clone();
    extra.make_vars.push("CFLAGS=-Os".to_owned());
    extra.goals.push("extra".to_owned());
    let argv = make_argv(&image_tools(), &layout, &cflags, &args, &extra).unwrap();
    assert_eq!(
        argv[argv.len() - 3..],
        ["CFLAGS=-Os", "libqcbor.a", "extra"].map(String::from)
    );
}

#[test]
fn build_01_effective_cflags_are_baseline_defines_then_flags() {
    let config = Config {
        name: "x".to_owned(),
        defines: vec!["A=1".to_owned(), "B".to_owned()],
        cflags: vec!["-fshort-enums".to_owned()],
        ..Config::default()
    };

    let flags = effective_cflags(&baseline_vec(), &config);

    assert_eq!(
        flags,
        [
            "-mcpu=cortex-m4",
            "-mthumb",
            "-mfloat-abi=hard",
            "-mfpu=fpv4-sp-d16",
            "-Os",
            "-DA=1",
            "-DB",
            "-fshort-enums",
        ]
    );
}

#[test]
fn build_01_placeholders_expand_and_unknown_are_rejected() {
    let values = Expansion {
        cc: "/usr/bin/gcc",
        cflags: "-Os -DX",
        src: "/work/run/t/default/src",
        inputs: "/work/run/t/default/inputs",
    };

    assert_eq!(
        expand("CC={cc} {cflags}", &values).unwrap(),
        "CC=/usr/bin/gcc -Os -DX"
    );
    assert_eq!(
        expand("-I{src}/inc -I{inputs} {src}", &values).unwrap(),
        "-I/work/run/t/default/src/inc -I/work/run/t/default/inputs /work/run/t/default/src"
    );
    assert_eq!(expand("plain", &values).unwrap(), "plain");
    for bad in ["{foo}", "{cc", "}", "{{cc}}", "${CC}", "{ cc }", "{CC}"] {
        let error = expand(bad, &values).unwrap_err();
        assert!(
            matches!(error, CaptureError::InvalidInput(_)),
            "{bad}: {error:?}"
        );
    }
}

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

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

fn make_target(upstream: &support::Upstream, goals: &[&str], make_vars: &[&str]) -> Target {
    let mut target = git_target(upstream, BuildSystem::Make);
    target.build_args = Some(BuildArgs {
        dir: ".".to_owned(),
        goals: goals.iter().map(|g| (*g).to_owned()).collect(),
        make_vars: make_vars.iter().map(|v| (*v).to_owned()).collect(),
        cmake_target: None,
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

/// A fetched local upstream and the context to capture it with, or `None` when
/// this host lacks the Arm tools.
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

fn run_capture(bed: &Bed, target: &Target, config: &Config) -> CaptureReport {
    fetch(
        &runner_config(),
        &fetch_context(&bed.ctx.work_dir),
        "demo",
        target,
    )
    .unwrap();
    capture(&runner_config(), &bed.ctx, "demo", target, config).unwrap()
}

#[test]
fn build_01_capture_make_with_bear_end_to_end() {
    let Some(bed) = bed(&MAKE_PROJECT) else {
        return;
    };
    // Committed inputs of the target are copied beside the source and hashed.
    write(
        &bed.ctx.testbed_dir.join("targets/demo/extra.h"),
        "#define EXTRA 1\n",
    );
    let target = make_target(&bed.upstream, &["libt.a"], &["CC={cc} {cflags}"]);
    let mut config = default_config();
    config.defines = vec!["T_FLAG=1".to_owned()];

    let report = run_capture(&bed, &target, &config);

    assert_eq!(
        report.verdict,
        CaptureVerdict::Captured,
        "{:#?}",
        report.causes
    );
    assert!(report.causes.is_empty());
    assert_eq!(report.kind, "capture");
    assert_eq!(report.source.commit, bed.upstream.commit);
    assert_eq!(report.source.tree_sha256, bed.upstream.tree_sha256);
    assert_eq!(report.compile_commands.entries_exported, 1);
    assert_eq!(report.compile_commands.entries_built, 1);
    assert!(report.compile_commands.raw_sha256.is_some());
    assert!(report.compile_commands.canonical_sha256.is_some());
    assert!(report.gates.compiler_allow_listed && report.gates.arch_ok);
    assert!(report.gates.created_undeclared.is_empty() && report.gates.timestamp_macro.is_empty());
    let object = &report.objects["src/t.o"];
    assert!(object.arch_ok);
    assert_eq!(object.opt_goal.as_deref(), Some("Aggressive Size"));
    assert!(object.code_digest.is_some());
    assert_eq!(report.inputs.len(), 1);
    assert!(report.inputs.contains_key("extra.h"));
    assert!(report.toolchain_file_sha256.is_none());
    assert!(report.configure_files.is_empty());
    assert!(report.steps.iter().any(|s| s.name == "build"));
    assert!(report.steps.iter().any(|s| s.name == "readelf src/t.o"));

    let layout = RunLayout::new(&bed.ctx.work_dir, "demo", "default");
    for file in [
        "compile_commands.raw.json",
        "compile_commands.json",
        "capture.json",
        "objects/src/t.o",
    ] {
        assert!(layout.capture_dir.join(file).is_file(), "missing {file}");
    }
    assert!(layout.inputs.join("extra.h").is_file());
    let saved: CaptureReport =
        serde_json::from_str(&fs::read_to_string(layout.capture_dir.join("capture.json")).unwrap())
            .unwrap();
    assert_eq!(saved, report);

    // Run again from scratch: the same report, byte for byte.
    let again = capture(&runner_config(), &bed.ctx, "demo", &target, &config).unwrap();
    assert_eq!(again.to_json().unwrap(), report.to_json().unwrap());
}

#[test]
fn build_01_capture_drivers_fail_closed() {
    let Some(bed) = bed(&MAKE_PROJECT) else {
        return;
    };
    let default = default_config();

    // A recipe that never compiles captures nothing.
    let silent = make_upstream(&[("Makefile", "all:\n\t@true\n")]);
    let target = make_target(&silent, &["all"], &["CC={cc} {cflags}"]);
    let report = run_capture(&bed, &target, &default);
    assert_eq!(report.verdict, CaptureVerdict::CaptureFailed);
    assert_eq!(report.causes, ["empty_capture"]);

    // A build that exits non-zero is build_failed.
    let broken = make_upstream(&[("Makefile", "all:\n\t@false\n")]);
    let target = make_target(&broken, &["all"], &[]);
    let report = run_capture(&bed, &target, &default);
    assert_eq!(report.verdict, CaptureVerdict::BuildFailed);

    // The host compiler is not the pinned one.
    let target = make_target(&bed.upstream, &["libt.a"], &["CC=/usr/bin/cc"]);
    let report = run_capture(&bed, &target, &default);
    assert_eq!(report.verdict, CaptureVerdict::CaptureFailed);
    assert_eq!(report.causes, ["compiler_not_allowed"]);

    // An object built for another float ABI fails the architecture gate.
    let target = make_target(&bed.upstream, &["libt.a"], &["CC={cc} {cflags}"]);
    let mut soft = default_config();
    soft.cflags = vec!["-mfloat-abi=soft".to_owned()];
    let report = run_capture(&bed, &target, &soft);
    assert_eq!(report.verdict, CaptureVerdict::CaptureFailed);
    assert_eq!(report.causes, ["arch_gate"]);
    assert!(!report.gates.arch_ok);
    assert!(!report.objects["src/t.o"].arch_ok);
}

#[test]
fn build_01_created_undeclared_objects_fail_capture() {
    let before = set(&["src/t.c", "src/Makefile"]);
    let after = set(&[
        "src/t.c",
        "src/Makefile",
        "src/t.o",
        "src/extra.o",
        "src/lib/x.obj",
        "src/libt.a",
        "src/notes.txt",
    ]);
    let declared = set(&["src/t.o"]);

    assert_eq!(
        undeclared_objects(&before, &after, &declared),
        ["src/extra.o", "src/lib/x.obj"]
    );
    // An object that already existed, or is declared, is not reported.
    let before_with_object = set(&["src/extra.o", "src/lib/x.obj"]);
    assert!(undeclared_objects(&before_with_object, &after, &declared).is_empty());

    // End to end: a rule that creates an object Bear never saw.
    let files = [
        ("t.c", "int t(void) { return 1; }\n"),
        (
            "Makefile",
            "all: t.o extra.o\nt.o: t.c\n\t$(CC) -c t.c -o t.o\nextra.o:\n\ttouch extra.o\n",
        ),
    ];
    let Some(bed) = bed(&files) else { return };
    let target = make_target(&bed.upstream, &["all"], &["CC={cc} {cflags}"]);

    let report = run_capture(&bed, &target, &default_config());

    assert_eq!(report.verdict, CaptureVerdict::CaptureFailed);
    assert_eq!(report.causes, ["created_undeclared"]);
    assert_eq!(report.gates.created_undeclared, ["src/extra.o"]);
}

#[test]
fn build_01_timestamp_macro_in_sources_fails_capture() {
    let dir = tempfile::tempdir().unwrap();
    write(
        &dir.path().join("stamp.c"),
        "const char *s(void) { return __TIMESTAMP__; }\n",
    );
    write(
        &dir.path().join("plain.c"),
        "const char *s(void) { return \"__DATE__\"; }\n",
    );
    let entry = |file: &str| CcEntry {
        directory: dir.path().to_str().unwrap().to_owned(),
        file: file.to_owned(),
        arguments: vec!["gcc".to_owned(), "-c".to_owned(), file.to_owned()],
        output: dir.path().join("x.o").to_str().unwrap().to_owned(),
    };

    let found = timestamp_macro_sources(&[entry("stamp.c"), entry("plain.c")]).unwrap();

    assert_eq!(found, ["stamp.c"]);
    // An unreadable source is an error, not a pass.
    assert!(timestamp_macro_sources(&[entry("missing.c")]).is_err());

    // End to end.
    let files = [
        ("t.c", "const char *when(void) { return __TIMESTAMP__; }\n"),
        ("Makefile", "all: t.o\nt.o: t.c\n\t$(CC) -c t.c -o t.o\n"),
    ];
    let Some(bed) = bed(&files) else { return };
    let target = make_target(&bed.upstream, &["all"], &["CC={cc} {cflags}"]);

    let report = run_capture(&bed, &target, &default_config());

    assert_eq!(report.verdict, CaptureVerdict::CaptureFailed);
    assert_eq!(report.causes, ["timestamp_macro"]);
    assert_eq!(report.gates.timestamp_macro, ["src/t.c"]);
}

#[test]
fn build_01_list_files_is_sorted_and_relative() {
    let dir = tempfile::tempdir().unwrap();
    write(&dir.path().join("b/c.o"), "x");
    write(&dir.path().join("a.c"), "x");

    let files = list_files(dir.path()).unwrap();

    assert_eq!(files.into_iter().collect::<Vec<_>>(), ["a.c", "b/c.o"]);
}

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

#[test]
fn build_01_capture_cmake_end_to_end() {
    let Some(bed) = bed(&CMAKE_PROJECT) else {
        return;
    };
    let mut target = git_target(&bed.upstream, BuildSystem::Cmake);
    target.build_args = Some(BuildArgs {
        dir: ".".to_owned(),
        goals: Vec::new(),
        make_vars: Vec::new(),
        cmake_target: Some("tj".to_owned()),
        cmake_options: Vec::new(),
    });
    let mut config = default_config();
    config.defines = vec!["T_EXTRA=1".to_owned()];

    let report = run_capture(&bed, &target, &config);

    assert_eq!(
        report.verdict,
        CaptureVerdict::Captured,
        "{:#?}",
        report.causes
    );
    assert!(report.toolchain_file_sha256.is_some());
    assert_eq!(report.compile_commands.entries_exported, 2);
    assert_eq!(report.compile_commands.entries_built, 1);
    assert_eq!(report.compile_commands.dropped_unbuilt.len(), 1);
    assert!(report.compile_commands.dropped_unbuilt[0].ends_with("u.c"));
    let keys: Vec<&String> = report.objects.keys().collect();
    assert_eq!(keys, ["build/CMakeFiles/tj.dir/t.c.obj"]);
    assert!(report.objects["build/CMakeFiles/tj.dir/t.c.obj"].arch_ok);
    // The configure-time header is an input of the compile, hashed in the report.
    assert!(
        report.configure_files.contains_key("build/gen.h"),
        "{:?}",
        report.configure_files
    );
    assert!(
        report
            .configure_files
            .keys()
            .all(|key| !key.ends_with(".obj") && !key.ends_with(".a")),
        "{:?}",
        report.configure_files
    );
    let names: Vec<&str> = report.steps.iter().map(|s| s.name.as_str()).collect();
    assert!(
        names.contains(&"cmake configure") && names.contains(&"cmake build"),
        "{names:?}"
    );
    let layout = RunLayout::new(&bed.ctx.work_dir, "demo", "default");
    let toolchain = fs::read_to_string(layout.capture_dir.join("toolchain.cmake")).unwrap();
    assert!(toolchain.contains("-Os -DT_EXTRA=1"), "{toolchain}");
    // The compile command carries the baseline and the define, exactly once each.
    let canonical = fs::read_to_string(layout.capture_dir.join("compile_commands.json")).unwrap();
    assert!(canonical.contains("-mfloat-abi=hard") && canonical.contains("-DT_EXTRA=1"));

    // Run again from scratch: the same report, byte for byte, except for the
    // output digest of `cmake configure`, whose text includes its own elapsed
    // time ("-- Configuring done (0.2s)").
    let again = capture(&runner_config(), &bed.ctx, "demo", &target, &config).unwrap();
    assert_eq!(
        without_configure_output(&again),
        without_configure_output(&report)
    );
}

/// The report as JSON, with the standard output digest of every `cmake
/// configure` step removed.
fn without_configure_output(report: &CaptureReport) -> serde_json::Value {
    let mut value = serde_json::to_value(report).unwrap();
    for step in value["steps"].as_array_mut().unwrap() {
        if step["name"] == "cmake configure" {
            let stdout = step["run"]["stdout"].as_object_mut().unwrap();
            stdout.remove("sha256");
            stdout.remove("bytes_total");
            stdout.remove("bytes_stored");
        }
    }
    value
}

fn host_object(source_dir: &Path, define: &str, out_dir: &str) -> Vec<u8> {
    let out = source_dir.join(out_dir);
    fs::create_dir_all(&out).unwrap();
    tool(
        &[
            "/usr/bin/cc",
            define,
            "-c",
            "v.c",
            "-o",
            out.join("v.o").to_str().unwrap(),
        ],
        source_dir,
        &[],
    );
    fs::read(out.join("v.o")).unwrap()
}

fn report_with(objects: &[(&str, &[u8])]) -> CaptureReport {
    let mut report = CaptureReport::new("demo", "x");
    for (key, bytes) in objects {
        report.objects.insert(
            (*key).to_owned(),
            CapturedObject {
                sha256: mt_capture::treehash::sha256_hex(bytes),
                arch_ok: true,
                opt_goal: None,
                code_digest: code_digest(bytes).ok(),
            },
        );
    }
    report
}

#[test]
fn bed_02_variant_code_diff_detects_changed_code() {
    let dir = tempfile::tempdir().unwrap();
    write(&dir.path().join("v.c"), "int f(void) { return VALUE; }\n");
    let one = host_object(dir.path(), "-DVALUE=1", "one");
    let one_again = host_object(dir.path(), "-DVALUE=1", "one-again");
    let two = host_object(dir.path(), "-DVALUE=2", "two");
    assert!(code_digest(&one).is_ok());

    let default = report_with(&[("src/v.o", &one)]);

    // Different -D values give different code.
    let changed = variant_code_diff(&default, &report_with(&[("src/v.o", &two)]));
    assert!(changed.changes_code);
    assert_eq!(changed.common_objects, ["src/v.o"]);
    assert_eq!(changed.code_differs, ["src/v.o"]);

    // An identical build changes nothing.
    let same = variant_code_diff(&default, &report_with(&[("src/v.o", &one_again)]));
    assert!(!same.changes_code);
    assert!(same.code_differs.is_empty());
    assert!(same.only_default.is_empty() && same.only_variant.is_empty());

    // A different set of translation units is a code change too.
    let more = variant_code_diff(
        &default,
        &report_with(&[("src/v.o", &one), ("src/w.o", &one)]),
    );
    assert!(more.changes_code);
    assert_eq!(more.only_variant, ["src/w.o"]);
    let fewer = variant_code_diff(
        &report_with(&[("src/v.o", &one), ("src/w.o", &one)]),
        &default,
    );
    assert!(fewer.changes_code);
    assert_eq!(fewer.only_default, ["src/w.o"]);

    // An unreadable object never reads as unchanged.
    let mut unreadable = report_with(&[("src/v.o", &one)]);
    unreadable.objects.get_mut("src/v.o").unwrap().code_digest = None;
    assert!(variant_code_diff(&default, &unreadable).changes_code);
}

#[test]
fn build_01_cmake_probe_outputs_are_not_configure_files() {
    // Outputs of the build and of CMake's own probes: left out of the hashes.
    for product in [
        "CMakeFiles/tj.dir/t.c.obj",
        "CMakeFiles/tj.dir/t.c.o",
        "libtj.a",
        "libtj.so",
        "CMakeFiles/CMakeConfigureLog.yaml",
        "CMakeFiles/CMakeOutput.log",
        "CMakeFiles/CMakeError.log",
        "CMakeFiles/3.25.1/CMakeDetermineCompilerABI_C.bin",
        "CMakeFiles/3.28.3/CMakeDetermineCompilerABI_C.bin",
    ] {
        assert!(is_cmake_product(product), "{product} is an output");
    }
    // Inputs of a compile command, and files that merely look similar.
    for input in [
        "gen.h",
        "config/version.h",
        "CMakeFiles/3.25.1/CMakeCCompiler.cmake",
        "CMakeFiles/3.25.1/CMakeSystem.cmake",
        "CMakeFiles/tj.dir/t.c.obj.d",
        "tables/CMakeDetermineCompilerABI_C.bin",
        "CMakeFiles/3.25.1/data.bin",
        "CMakeCache.txt",
    ] {
        assert!(!is_cmake_product(input), "{input} is a configure input");
    }
}
