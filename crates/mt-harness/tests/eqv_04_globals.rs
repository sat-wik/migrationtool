#![allow(clippy::unwrap_used, clippy::expect_used)]
//! EQV-04: the writable globals of the captured objects are found in the linked
//! driver, snapshotted before and after the call, and a changed one that the
//! executor cannot type fails the run instead of being stored as raw bytes
//! (CONTEXT D-06, D-10). The objects here are compiled from small C files in a
//! temporary directory; the real targets' globals are covered by the local
//! proofs in `eqv_04_exec`.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use mt_capture::manifest::{ArgRole, BASELINE_CFLAGS, Entry, EntryForm, Role};
use mt_harness::adapter::{Adapter, combinations};
use mt_harness::budget::Budgets;
use mt_harness::driver::render_entry_driver;
use mt_harness::globals::{locate_globals, writable_globals};
use mt_harness::layout::{CType, Kind};
use mt_harness::link::{LinkOutcome, LinkRequest, archive, link_driver};
use mt_harness::probe::GlobalType;
use mt_harness::run::{Exec, PreparedDriver, run_vector_twice};
use mt_harness::tools::ExecTools;
use mt_toolchain::runner::{self, RunRequest, RunnerConfig};
use mt_vectors::state::StateValue;

fn host_tools() -> Option<ExecTools> {
    let needed = [
        "/usr/bin/qemu-arm",
        "/usr/bin/arm-none-eabi-gcc",
        "/usr/bin/arm-none-eabi-ar",
        "/usr/lib/arm-none-eabi/include",
    ];
    if let Some(missing) = needed.iter().find(|path| !Path::new(path).exists()) {
        eprintln!("skipping: {missing} is not installed on this host");
        return None;
    }
    Some(ExecTools::at(
        Path::new("/usr/bin"),
        Path::new("/usr/bin/qemu-arm"),
        Path::new("/usr/bin/llvm-dwarfdump"),
    ))
}

fn cfg() -> RunnerConfig {
    RunnerConfig::new("/usr/bin:/bin", "1791158400")
}

fn arm_role(role: Role) -> ArgRole {
    ArgRole {
        role,
        name: None,
        default: None,
        sweep: Vec::new(),
        max: None,
        of: None,
        mutable: false,
        values: Vec::new(),
        count: None,
        ptr_field: None,
        len_field: None,
        local: None,
    }
}

/// Compile `dir/<file>` to `dir/<object>` with the baseline flags.
fn compile(tools: &ExecTools, dir: &Path, file: &str, object: &str) {
    let mut argv: Vec<OsString> = vec![tools.arm_gcc.clone().into_os_string()];
    argv.extend(BASELINE_CFLAGS.iter().map(OsString::from));
    argv.extend(["-c", file, "-o", object].map(OsString::from));
    let output = runner::run(
        &cfg(),
        &RunRequest {
            tool_name: "arm_gcc".to_owned(),
            argv,
            cwd: dir.to_path_buf(),
            extra_env: BTreeMap::new(),
            tool_version: None,
        },
    )
    .unwrap();
    assert_eq!(
        output.record.exit.code,
        Some(0),
        "compile of {file}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A driver for `int peek(const char *, size_t)` over the objects compiled from
/// `sources` (file name, C text) in a temporary directory.
struct Built {
    _dir: tempfile::TempDir,
    root: PathBuf,
    adapter: Adapter,
    driver: PreparedDriver,
}

fn build(tools: &ExecTools, sources: &[(&str, &str)]) -> Result<Built, String> {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    std::fs::write(
        root.join("x.h"),
        "#include <stddef.h>\nint peek(const char *p, size_t n);\n",
    )
    .unwrap();
    let mut members = Vec::new();
    let mut symbols = Vec::new();
    for (path, text) in sources {
        let file = Path::new(path);
        let sub = root.join(file.parent().unwrap());
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(root.join(path), text).unwrap();
        let object = path.replace(".c", ".o");
        compile(
            tools,
            &sub,
            file.file_name().unwrap().to_str().unwrap(),
            &object_name(&object),
        );
        let relative = object.clone();
        symbols.extend(
            writable_globals(&relative, &std::fs::read(root.join(&relative)).unwrap()).unwrap(),
        );
        members.push(relative);
    }
    let pair = archive(tools, &cfg(), &root, &members, &root.join("pair.a")).unwrap();
    let entry = Entry {
        name: "peek".to_owned(),
        header: "x.h".to_owned(),
        source: "x.c".to_owned(),
        form: EntryForm::Direct,
        signature: "int (const char *, size_t)".to_owned(),
        accepted_wrapped: false,
        arg: vec![arm_role(Role::InBuf), arm_role(Role::InLen)],
        unobserved: Vec::new(),
        local: Vec::new(),
        setup: Vec::new(),
    };
    let adapter = Adapter::from_entry("synthetic", "default", &entry).unwrap();
    let include = format!("#include \"{}\"", root.join("x.h").display());
    let source = render_entry_driver(&adapter, &[include]).unwrap();
    let build_dir = root.join("build");
    let request = LinkRequest {
        dir: &build_dir,
        source: &source,
        flags: &[],
        compile_cwd: &root,
        archive: &pair.path,
    };
    let LinkOutcome::Linked(linked) = link_driver(tools, &cfg(), &request).unwrap() else {
        return Err("the synthetic driver did not link".to_owned());
    };
    let elf = std::fs::read(&linked.elf_path).unwrap();
    let plan = locate_globals(&symbols, &elf).map_err(|error| error.to_string())?;
    Ok(Built {
        _dir: dir,
        root,
        adapter,
        driver: PreparedDriver {
            elf_path: linked.elf_path.clone(),
            plan,
            ..PreparedDriver::default()
        },
    })
}

fn object_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned()
}

fn run(tools: &ExecTools, built: &Built, seed: &[u8]) -> Result<mt_harness::run::Observed, String> {
    let combos = combinations(&built.adapter).unwrap();
    run_vector_twice(
        &Exec {
            tools,
            cfg: &cfg(),
            budgets: &Budgets::DEFAULT,
        },
        &built.adapter,
        &built.driver,
        &built.root.join("slots"),
        seed,
        &combos[0],
    )
    .unwrap()
}

const READ_ONLY_USE: &str = "#include <stddef.h>\n\
    int counter = 5;\n\
    static volatile int secret = 7;\n\
    int peek(const char *p, size_t n) { (void)p; return counter + secret + (int)n; }\n";

const WRITES_GLOBAL: &str = "#include <stddef.h>\n\
    int counter = 5;\n\
    int peek(const char *p, size_t n) { (void)p; counter += (int)n; return counter; }\n";

#[test]
fn eqv_04_globals_are_snapshotted_and_an_unchanged_one_is_not_recorded() {
    let Some(tools) = host_tools() else {
        return;
    };
    let built = build(&tools, &[("x.c", READ_ONLY_USE)]).unwrap();
    // Both the global and the local static are in the linked image.
    let keys: Vec<(&str, u32, Vec<u8>)> = built
        .driver
        .plan
        .iter()
        .map(|range| (range.key.as_str(), range.size, range.initial.clone()))
        .collect();
    assert_eq!(
        keys,
        [
            ("x.o:counter", 4, vec![5, 0, 0, 0]),
            ("x.o:secret", 4, vec![7, 0, 0, 0]),
        ]
    );
    let observed = run(&tools, &built, b"abc").unwrap();
    assert_eq!(observed.vector.rc, 5 + 7 + 3);
    assert!(
        observed.vector.lib_state_out.is_empty(),
        "an unchanged global is not recorded"
    );
    assert!(observed.vector.lib_state_in.is_empty());
}

#[test]
fn eqv_04_changed_global_is_untyped_global_changed() {
    let Some(tools) = host_tools() else {
        return;
    };
    let built = build(&tools, &[("x.c", WRITES_GLOBAL)]).unwrap();
    assert_eq!(built.driver.plan.len(), 1);
    // The input length 0 leaves the global as it was.
    let quiet = run(&tools, &built, b"").unwrap();
    assert_eq!(quiet.vector.rc, 5);
    // Any other length changes it, and without a typed layout that is an
    // executor error that names the global and stores nothing.
    let reason = run(&tools, &built, b"abc").unwrap_err();
    assert!(reason.contains("untyped_global_changed"), "{reason}");
    assert!(reason.contains("x.o:counter"), "{reason}");
}

#[test]
fn eqv_04_changed_global_with_a_type_is_recorded_in_both_states() {
    let Some(tools) = host_tools() else {
        return;
    };
    let mut built = build(&tools, &[("x.c", WRITES_GLOBAL)]).unwrap();
    built.driver.global_types.insert(
        "x.o:counter".to_owned(),
        GlobalType::Typed {
            ty: CType {
                text: "int".to_owned(),
                size: 4,
                kind: Kind::Int { signed: true },
            },
        },
    );
    // An unchanged global leaves lib_state empty even though it has a type.
    let quiet = run(&tools, &built, b"").unwrap();
    assert!(quiet.vector.lib_state_in.is_empty() && quiet.vector.lib_state_out.is_empty());
    // The call adds the input length (3) to the counter: 5 before, 8 after.
    let observed = run(&tools, &built, b"abc").unwrap();
    assert_eq!(observed.vector.rc, 8);
    let value = |state: &mt_vectors::state::LibState| match state.get("globals") {
        Some(StateValue::Struct { fields, .. }) => match fields.get("x.o:counter") {
            Some(StateValue::Int { c_type, value }) => (c_type.clone(), *value),
            other => panic!("{other:?}"),
        },
        other => panic!("{other:?}"),
    };
    assert_eq!(value(&observed.vector.lib_state_in), ("int".to_owned(), 5));
    assert_eq!(value(&observed.vector.lib_state_out), ("int".to_owned(), 8));
}

#[test]
fn eqv_04_ambiguous_local_global_join_is_an_error() {
    let Some(tools) = host_tools() else {
        return;
    };
    // Two objects from two files that are both called x.c, each with a static
    // called secret: the linked image cannot say which is which.
    let first = "#include <stddef.h>\n\
        int other(void);\n\
        static volatile int secret = 1;\n\
        int peek(const char *p, size_t n) { (void)p; return secret + other() + (int)n; }\n";
    let second = "static volatile int secret = 2;\nint other(void) { return secret; }\n";
    let error = build(&tools, &[("a/x.c", first), ("b/x.c", second)])
        .err()
        .expect("the join is ambiguous");
    assert!(error.contains("ambiguous"), "{error}");
    assert!(error.contains("secret"), "{error}");
}
