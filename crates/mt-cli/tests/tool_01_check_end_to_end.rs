#![allow(clippy::unwrap_used, clippy::expect_used)]
//! TOOL-01 / TOOL-02: the built `mt` binary, launched through the runner,
//! drives `mt toolchain check` against a hermetic fake container tree. Every
//! pinned tool is a small `/bin/sh` script in a temporary directory that prints
//! a recorded version text, so the whole check (pins validation, runner,
//! parsing, LLVM rules, table, exit code) is proven without Docker.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use mt_toolchain::runner::{self, RunRequest, RunnerConfig};

/// The complete, valid pins used by the library tests; the fake container is
/// laid out to match it after its `/opt/` and `/usr/` prefixes are moved.
const FULL_PINS: &str = include_str!("../../mt-toolchain/tests/fixtures/pins-full.toml");

/// Tests write executable scripts and then spawn them. Holding this lock for a
/// whole test keeps another test's `fork` from briefly sharing a script's write
/// descriptor, which would make the exec fail with "text file busy".
static EXEC_LOCK: Mutex<()> = Mutex::new(());

fn serialized() -> MutexGuard<'static, ()> {
    EXEC_LOCK.lock().unwrap_or_else(PoisonError::into_inner)
}

struct Outcome {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

/// Every schema v1 section with well-formed values, so `validate` accepts the
/// file; only the one fake tool differs between tests.
fn pins_text(bin: &str, expect: &str) -> String {
    format!(
        r#"schema_version = 1

[image]
platform = "linux/amd64"
base = "debian:bookworm-20261005-slim"
base_digest = "sha256:7c7b2c966bc9ee8cedfeef67e0e279108992c77681fa595db4a9d65c06ccc587"
snapshot_timestamp = "20261005T000000Z"
source_date_epoch = "1791158400"
tool_path = "/usr/bin:/bin"
dpkg_query = "/usr/bin/dpkg-query"

[llvm]
major = 16
max_major = 19
status = "provisional"
decision = "D-15"

[rust]
rustup_home = "/opt/rustup"
cargo_home = "/opt/cargo"
host = "x86_64-unknown-linux-gnu"

[rust.tool]
version = "1.99.0"
commit = "b940084d7eb6a299eb4bfeb8e34901bc051e7ac4"
channel_manifest_sha256 = "ce6dddc886364f8d786514771212cebe9b731ba82d6b859951c6b0ccc516b6a2"
components = ["clippy", "rustfmt"]
targets = ["thumbv7em-none-eabihf"]

[rust.bitcode]
version = "1.72.1"
commit = "d5c2e9c342b358556da91d61ed4133f6f50fc0c3"
llvm = "16.0.5"
channel_manifest_sha256 = "77113b9660855a5ab49e5ff029c998be93fdcea41b062b40420675cf1f4ea229"
decision = "D-15"

[ci]
cargo_deny_version = "0.20.2"
buildkit_image = "moby/buildkit:v0.34.0@sha256:b059f8d7226d0b326bc871af489a5dddf65e36af4d20d14251b072974d9ee87c"

[tool.fake_tool]
bin = "{bin}"
version_args = ["-c", "echo fake-tool 1.2.3"]
version_regex = 'fake-tool (?P<version>\d+\.\d+\.\d+)'
expect = "{expect}"
"#
    )
}

/// Write `pins` to a temp dir and run `mt toolchain check --pins <file>` through the runner.
fn run_mt(pins: &str) -> Outcome {
    run_mt_action("check", pins)
}

/// Write `pins` to a temp dir and run `mt toolchain <action> --pins <file>`.
fn run_mt_action(action: &str, pins: &str) -> Outcome {
    let _guard = serialized();
    let dir = tempfile::tempdir().unwrap();
    let pins_path = dir.path().join("pins.toml");
    std::fs::write(&pins_path, pins).unwrap();
    run_mt_with_path(dir.path(), action, &pins_path)
}

fn run_mt_with_path(cwd: &Path, action: &str, pins_path: &Path) -> Outcome {
    run_mt_args(
        cwd,
        &[
            "toolchain".into(),
            action.into(),
            "--pins".into(),
            pins_path.as_os_str().to_owned(),
        ],
    )
}

/// Run the built `mt` with `args` (everything after the program name) through the runner.
fn run_mt_args(cwd: &Path, args: &[OsString]) -> Outcome {
    let cfg = RunnerConfig::new("/usr/bin:/bin", "1791158400");
    let mut argv: Vec<OsString> = vec![env!("CARGO_BIN_EXE_mt").into()];
    argv.extend(args.iter().cloned());
    let request = RunRequest {
        tool_name: "mt".to_owned(),
        argv,
        cwd: cwd.to_path_buf(),
        extra_env: BTreeMap::new(),
        tool_version: None,
    };
    let output = runner::run(&cfg, &request).unwrap();
    assert!(!output.record.exit.timed_out, "mt timed out");
    Outcome {
        code: output.record.exit.code,
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

const CLANG_OUT: &str = "Debian clang version 16.0.6 (15~deb12u1)\nTarget: x86_64-pc-linux-gnu\n";
const KLEE_OUT: &str = "KLEE 3.2 (https://klee-se.org)\n  Build mode: RelWithDebInfo (Asserts: OFF)\n\nLLVM (http://llvm.org/):\n  LLVM version 16.0.6\n  Host CPU: skylake-avx512\n";
const LDD_OUT: &str = "\tlinux-vdso.so.1 (0x00007ffc4a5f1000)\n\tlibclang-cpp.so.16 => /usr/lib/llvm-16/lib/libclang-cpp.so.16 (0x00007f1c2e800000)\n\tlibLLVM-16.so.1 => /usr/lib/llvm-16/lib/libLLVM-16.so.1 (0x00007f1c2a000000)\n";
const ARM_OUT: &str =
    "arm-none-eabi-gcc (Arm GNU Toolchain 14.3.Rel1 (Build arm-14.174)) 14.3.1 20250623\n";
const QEMU_USER_OUT: &str = "qemu-arm version 7.2.15 (Debian 1:7.2+dfsg-7+deb12u18)\n";
const QEMU_SYSTEM_OUT: &str = "QEMU emulator version 7.2.15 (Debian 1:7.2+dfsg-7+deb12u18)\n";
const RUSTC_TOOL_OUT: &str = "rustc 1.99.0 (b940084d7 2026-09-28)\nbinary: rustc\ncommit-hash: b940084d7eb6a299eb4bfeb8e34901bc051e7ac4\ncommit-date: 2026-09-28\nhost: x86_64-unknown-linux-gnu\nrelease: 1.99.0\nLLVM version: 23.1.1\n";
const RUSTC_BITCODE_OUT: &str = "rustc 1.72.1 (d5c2e9c34 2023-09-13)\nbinary: rustc\ncommit-hash: d5c2e9c342b358556da91d61ed4133f6f50fc0c3\ncommit-date: 2023-09-13\nhost: x86_64-unknown-linux-gnu\nrelease: 1.72.1\nLLVM version: 16.0.5\n";
const DPKG_OUT: &str = "bear\t3.1.1-1\tinstalled\nclang-16\t1:16.0.6-15~deb12u1\tinstalled\nlibclang-16-dev\t1:16.0.6-15~deb12u1\tinstalled\nlibclang-cpp16-dev\t1:16.0.6-15~deb12u1\tinstalled\nlibllvm16\t1:16.0.6-15~deb12u1\tinstalled\nlibz3-dev\t4.8.12-3.1\tinstalled\nllvm-16\t1:16.0.6-15~deb12u1\tinstalled\nllvm-16-dev\t1:16.0.6-15~deb12u1\tinstalled\nqemu-system-arm\t1:7.2+dfsg-7+deb12u18\tinstalled\nqemu-user\t1:7.2+dfsg-7+deb12u18\tinstalled\n";

/// A script that ignores its arguments and prints `text` on stdout.
fn prints(text: &str) -> String {
    format!("cat <<'MT_EOF'\n{text}MT_EOF\n")
}

/// A script that requires its arguments to be exactly `args`, then prints `text`.
fn expects_args_then_prints(args: &str, text: &str) -> String {
    format!(
        "[ \"$*\" = '{args}' ] || {{ echo \"unexpected arguments: $*\" >&2; exit 64; }}\n{}",
        prints(text)
    )
}

/// The fake `rustup`: answers `--version`, `default` and `target list` and
/// insists on the arguments and environment the check is supposed to use.
fn rustup_script() -> String {
    format!(
        "case \"$*\" in\n\
         '--version') {version} ;;\n\
         'default') [ -n \"$RUSTUP_HOME\" ] || exit 65; echo '1.99.0-x86_64-unknown-linux-gnu (default)' ;;\n\
         'target list --installed --toolchain 1.99.0') [ -n \"$RUSTUP_HOME\" ] || exit 65; printf '%s\\n' thumbv7em-none-eabihf x86_64-unknown-linux-gnu x86_64-unknown-linux-musl ;;\n\
         'target list --installed --toolchain 1.72.1') [ -n \"$RUSTUP_HOME\" ] || exit 65; printf '%s\\n' x86_64-unknown-linux-gnu ;;\n\
         *) echo \"unexpected arguments: $*\" >&2; exit 64 ;;\n\
         esac\n",
        version = "echo 'rustup 1.29.1 (b3c2e5f7a 2026-09-10)'"
    )
}

/// A fake container: every tool in `pins-full.toml` as a script under a temp dir.
struct FakeContainer {
    dir: tempfile::TempDir,
}

impl FakeContainer {
    fn new() -> Self {
        let fake = Self {
            dir: tempfile::tempdir().unwrap(),
        };
        fake.script("usr/lib/llvm-16/bin/clang", &prints(CLANG_OUT));
        fake.script("usr/lib/llvm-16/bin/llvm-config", &prints("16.0.6\n"));
        fake.script("opt/klee/bin/klee", &prints(KLEE_OUT));
        fake.script("opt/cargo-tools/bin/c2rust", &prints("c2rust 0.22.1\n"));
        fake.script("usr/bin/ldd", &prints(LDD_OUT));
        fake.script("usr/bin/bear", &prints("bear 3.1.1\n"));
        fake.script(
            "opt/arm-gnu-toolchain/bin/arm-none-eabi-gcc",
            &prints(ARM_OUT),
        );
        fake.script("usr/bin/qemu-arm", &prints(QEMU_USER_OUT));
        fake.script("usr/bin/qemu-system-arm", &prints(QEMU_SYSTEM_OUT));
        fake.script(
            "opt/cargo/bin/cargo",
            &expects_args_then_prints("mutants --version", "cargo-mutants 27.1.0\n"),
        );
        fake.script("opt/cargo/bin/rustup", &rustup_script());
        fake.script(
            "opt/rustup/toolchains/1.99.0-x86_64-unknown-linux-gnu/bin/rustc",
            &expects_args_then_prints("-vV", RUSTC_TOOL_OUT),
        );
        fake.script(
            "opt/rustup/toolchains/1.72.1-x86_64-unknown-linux-gnu/bin/rustc",
            &expects_args_then_prints("-vV", RUSTC_BITCODE_OUT),
        );
        // dpkg-query must be called as `-W -f=<format>`; its rows come from a data file.
        fake.write("usr/bin/dpkg-query.rows", DPKG_OUT);
        let rows = fake.path("usr/bin/dpkg-query.rows");
        fake.script(
            "usr/bin/dpkg-query",
            &format!(
                "[ \"$1\" = '-W' ] || exit 64\n\
                 [ \"$2\" = '-f=${{Package}}\\t${{Version}}\\t${{db:Status-Status}}\\n' ] || {{ echo \"unexpected format: $2\" >&2; exit 64; }}\n\
                 cat '{}'\n",
                rows.display()
            ),
        );
        fake
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.dir.path().join(relative)
    }

    fn write(&self, relative: &str, text: &str) {
        let path = self.path(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
    }

    /// Write an executable `/bin/sh` script.
    fn script(&self, relative: &str, body: &str) {
        self.write(relative, &format!("#!/bin/sh\n{body}"));
        std::fs::set_permissions(self.path(relative), std::fs::Permissions::from_mode(0o755))
            .unwrap();
    }

    /// `pins-full.toml` with its `/opt/` and `/usr/` prefixes moved into the fake tree.
    fn pins(&self) -> String {
        let root = self.dir.path().to_str().unwrap();
        FULL_PINS
            .replace("/opt/", "@ROOT@/opt/")
            .replace("/usr/", "@ROOT@/usr/")
            .replace("@ROOT@", root)
    }

    /// Run `mt toolchain check` on the given pins text.
    fn check(&self, pins: &str) -> Outcome {
        let pins_path = self.path("pins.toml");
        std::fs::write(&pins_path, pins).unwrap();
        run_mt_with_path(self.dir.path(), "check", &pins_path)
    }
}

#[test]
fn tool_01_check_end_to_end_reports_ok_and_exits_zero() {
    let _guard = serialized();
    let fake = FakeContainer::new();
    let out = fake.check(&fake.pins());
    assert_eq!(
        out.code,
        Some(0),
        "stdout: {} stderr: {}",
        out.stdout,
        out.stderr
    );
    assert!(out.stdout.contains("result: OK"), "stdout: {}", out.stdout);
    for needle in [
        "klee",
        "rustc_bitcode",
        "rust_default",
        "llvm_packages",
        "apt:clang-16",
        "hayroll",
        "kani",
        "not_installed",
        "llvm pin: 16 (provisional, decision D-15)",
    ] {
        assert!(out.stdout.contains(needle), "{needle}: {}", out.stdout);
    }
    assert!(!out.stdout.contains("MISMATCH"), "stdout: {}", out.stdout);
    assert!(!out.stdout.contains("Host CPU"), "stdout: {}", out.stdout);
}

#[test]
fn tool_01_check_end_to_end_reports_mismatch_and_exits_one() {
    let _guard = serialized();
    let fake = FakeContainer::new();
    // klee still says 3.2 but links LLVM 17: the single-LLVM-major rule fires.
    fake.script(
        "opt/klee/bin/klee",
        &prints(&KLEE_OUT.replace("LLVM version 16.0.6", "LLVM version 17.0.1")),
    );
    let out = fake.check(&fake.pins());
    assert_eq!(
        out.code,
        Some(1),
        "stdout: {} stderr: {}",
        out.stdout,
        out.stderr
    );
    let klee = out
        .stdout
        .lines()
        .find(|l| l.starts_with("klee"))
        .unwrap_or_else(|| panic!("no klee row in: {}", out.stdout));
    assert!(klee.contains("MISMATCH"), "{klee}");
    assert!(klee.contains("17"), "{klee}");
    assert!(
        out.stdout.contains("result: FAILED (1 rows)"),
        "stdout: {}",
        out.stdout
    );
}

#[test]
fn tool_01_check_end_to_end_reports_missing_tool_and_exits_one() {
    let _guard = serialized();
    let fake = FakeContainer::new();
    std::fs::remove_file(fake.path("usr/bin/bear")).unwrap();
    let out = fake.check(&fake.pins());
    assert_eq!(
        out.code,
        Some(1),
        "stdout: {} stderr: {}",
        out.stdout,
        out.stderr
    );
    let bear = out
        .stdout
        .lines()
        .find(|l| l.starts_with("bear"))
        .unwrap_or_else(|| panic!("no bear row in: {}", out.stdout));
    assert!(bear.contains("MISSING"), "{bear}");
    assert!(
        out.stdout.contains("result: FAILED (1 rows)"),
        "stdout: {}",
        out.stdout
    );
}

#[test]
fn tool_01_check_end_to_end_rejects_invalid_pins_with_exit_two() {
    let _guard = serialized();
    let fake = FakeContainer::new();

    // Incomplete pins: no [tool.bear] table.
    let pins = fake.pins();
    let start = pins.find("[tool.bear]").unwrap();
    let end = pins.find("[tool.c2rust]").unwrap();
    let incomplete = format!("{}{}", &pins[..start], &pins[end..]);
    let out = fake.check(&incomplete);
    assert_eq!(
        out.code,
        Some(2),
        "stdout: {} stderr: {}",
        out.stdout,
        out.stderr
    );
    assert!(out.stderr.contains("tool.bear"), "stderr: {}", out.stderr);
    assert!(out.stdout.is_empty(), "stdout: {}", out.stdout);

    // Pins that are not even TOML.
    let out = fake.check("this is = not [valid toml");
    assert_eq!(
        out.code,
        Some(2),
        "stdout: {} stderr: {}",
        out.stdout,
        out.stderr
    );
    assert!(out.stderr.contains("pins"), "stderr: {}", out.stderr);
    assert!(out.stdout.is_empty(), "stdout: {}", out.stdout);
}

#[test]
fn tool_03_check_end_to_end_rejects_placeholder_pins_with_exit_two() {
    let pins = pins_text("/bin/sh", "1.2.3").replace(
        "base = \"debian:bookworm-20261005-slim\"",
        "base = \"TODO\"",
    );
    let out = run_mt(&pins);
    assert_eq!(
        out.code,
        Some(2),
        "stdout: {} stderr: {}",
        out.stdout,
        out.stderr
    );
    assert!(out.stderr.contains("image.base"), "stderr: {}", out.stderr);
    assert!(out.stderr.contains("placeholder"), "stderr: {}", out.stderr);
    assert!(out.stdout.is_empty(), "stdout: {}", out.stdout);
}

#[test]
fn tool_03_build_args_end_to_end_prints_sorted_lines_and_rejects_bad_pins() {
    let out = run_mt_action("build-args", &pins_text("/bin/sh", "1.2.3"));
    assert_eq!(
        out.code,
        Some(0),
        "stdout: {} stderr: {}",
        out.stdout,
        out.stderr
    );
    let names: Vec<&str> = out
        .stdout
        .lines()
        .map(|line| line.split_once('=').unwrap().0)
        .collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(names, sorted, "stdout: {}", out.stdout);
    assert!(
        out.stdout.contains("LLVM_MAJOR=16\n"),
        "stdout: {}",
        out.stdout
    );

    let spaced = pins_text("/bin/sh", "1.2.3").replace(
        "tool_path = \"/usr/bin:/bin\"",
        "tool_path = \"/usr/bin /bin\"",
    );
    let out = run_mt_action("build-args", &spaced);
    assert_eq!(
        out.code,
        Some(2),
        "stdout: {} stderr: {}",
        out.stdout,
        out.stderr
    );
    assert!(out.stdout.is_empty(), "stdout: {}", out.stdout);
}

fn os(text: &str) -> OsString {
    text.into()
}

#[test]
fn tool_03_manifest_end_to_end_is_byte_stable_and_diff_locates_the_change() {
    let _guard = serialized();
    let fake = FakeContainer::new();
    let cwd = fake.dir.path();
    let pins_path = fake.path("pins.toml");
    std::fs::write(&pins_path, fake.pins()).unwrap();
    let manifest_to = |name: &str| {
        let out_path = fake.path(name);
        let out = run_mt_args(
            cwd,
            &[
                os("toolchain"),
                os("manifest"),
                os("--pins"),
                pins_path.clone().into_os_string(),
                os("--out"),
                out_path.clone().into_os_string(),
            ],
        );
        (out, out_path)
    };
    let diff = |a: &Path, b: &Path| {
        run_mt_args(
            cwd,
            &[
                os("toolchain"),
                os("manifest-diff"),
                a.as_os_str().to_owned(),
                b.as_os_str().to_owned(),
            ],
        )
    };

    // Two runs over the same container give byte-identical manifests.
    let (first, a) = manifest_to("a.json");
    assert_eq!(first.code, Some(0), "stderr: {}", first.stderr);
    let (second, b) = manifest_to("b.json");
    assert_eq!(second.code, Some(0), "stderr: {}", second.stderr);
    let (text_a, text_b) = (
        std::fs::read_to_string(&a).unwrap(),
        std::fs::read_to_string(&b).unwrap(),
    );
    assert_eq!(text_a, text_b);
    assert!(text_a.contains("\"schema_version\": 1"), "{text_a}");
    assert!(text_a.contains("\"status\": \"provisional\""), "{text_a}");
    assert!(!text_a.contains("Host CPU"), "{text_a}");
    let same = diff(&a, &b);
    assert_eq!(
        same.code,
        Some(0),
        "stdout: {} stderr: {}",
        same.stdout,
        same.stderr
    );

    // Without --out the manifest goes to stdout and the check table to stderr.
    let printed = run_mt_args(
        cwd,
        &[
            os("toolchain"),
            os("manifest"),
            os("--pins"),
            pins_path.clone().into_os_string(),
        ],
    );
    assert_eq!(printed.code, Some(0), "stderr: {}", printed.stderr);
    assert_eq!(printed.stdout, text_a);
    assert!(printed.stderr.contains("result: OK"), "{}", printed.stderr);

    // klee drifts to LLVM 17: the check fails (exit 1) but the manifest is still
    // written so CI can diff it, and the diff names the klee row.
    fake.script(
        "opt/klee/bin/klee",
        &prints(&KLEE_OUT.replace("LLVM version 16.0.6", "LLVM version 17.0.1")),
    );
    let (drifted, c) = manifest_to("c.json");
    assert_eq!(drifted.code, Some(1), "stderr: {}", drifted.stderr);
    assert!(
        c.exists(),
        "manifest must be written even when the check fails"
    );
    let found = diff(&a, &c);
    assert_eq!(found.code, Some(1), "stderr: {}", found.stderr);
    assert!(
        found
            .stdout
            .starts_with("first difference at observed.klee."),
        "stdout: {}",
        found.stdout
    );

    // Byte-identical files of any kind are "no difference"; differing non-JSON is an error.
    let same_toml = diff(&pins_path, &pins_path);
    assert_eq!(same_toml.code, Some(0), "stderr: {}", same_toml.stderr);
    fake.write("x.txt", "x");
    fake.write("y.txt", "y");
    let bad = diff(&fake.path("x.txt"), &fake.path("y.txt"));
    assert_eq!(bad.code, Some(2), "stdout: {}", bad.stdout);
    assert!(bad.stderr.contains("JSON"), "stderr: {}", bad.stderr);
}
