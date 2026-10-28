//! Compiling and linking a rendered driver through the runner (CONTEXT D-29, D-30).
//!
//! The link recipe is the pinned compiler with the manifest's baseline flags
//! plus `-nostartfiles --specs=nosys.specs` and `-lm`. `--specs=linux.specs` is
//! never used (it crashes at startup under qemu-arm).

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use mt_capture::manifest::BASELINE_CFLAGS;
use mt_toolchain::runner::{self, RunRecord, RunRequest, RunnerConfig};

use crate::adapter::Adapter;
use crate::budget::Budgets;
use crate::runtime::{Placeholders, render};
use crate::tools::ExecTools;
use crate::{HarnessError, sha256_hex};

/// The most stderr text kept in a build error.
const STDERR_EXCERPT: usize = 2000;

/// A built driver and the evidence of how it was built.
#[derive(Debug, Clone)]
pub struct DriverBuild {
    /// The build directory (also the compiler's working directory).
    pub dir: PathBuf,
    /// The generated `driver.c`.
    pub source_path: PathBuf,
    /// The linked `driver.elf`.
    pub elf_path: PathBuf,
    /// sha256 of `driver.c`.
    pub source_sha256: String,
    /// sha256 of `driver.elf`.
    pub elf_sha256: String,
    /// The run record of each build step, in order.
    pub records: Vec<RunRecord>,
}

fn io(action: &'static str, path: &Path, source: std::io::Error) -> HarnessError {
    HarnessError::Io {
        action,
        path: path.to_path_buf(),
        source,
    }
}

/// Write `source` as `driver.c` into `dir` and link it into `driver.elf`.
///
/// # Errors
/// [`HarnessError::Io`] when a file cannot be written or read,
/// [`HarnessError::Runner`] when the compiler cannot be started, and
/// [`HarnessError::Build`] on a non-zero exit, a timeout or an incomplete
/// stream, with an excerpt of the compiler's stderr.
pub fn build_driver(
    tools: &ExecTools,
    cfg: &RunnerConfig,
    dir: &Path,
    source: &str,
) -> Result<DriverBuild, HarnessError> {
    std::fs::create_dir_all(dir).map_err(|err| io("cannot create", dir, err))?;
    let source_path = dir.join("driver.c");
    let elf_path = dir.join("driver.elf");
    std::fs::write(&source_path, source).map_err(|err| io("cannot write", &source_path, err))?;
    // A stale binary must never be mistaken for the result of this build.
    if elf_path.exists() {
        std::fs::remove_file(&elf_path).map_err(|err| io("cannot remove", &elf_path, err))?;
    }

    let mut argv: Vec<OsString> = vec![tools.arm_gcc.clone().into_os_string()];
    argv.extend(BASELINE_CFLAGS.iter().map(OsString::from));
    for flag in [
        "-nostartfiles",
        "--specs=nosys.specs",
        "-o",
        "driver.elf",
        "driver.c",
        "-lm",
    ] {
        argv.push(OsString::from(flag));
    }
    let request = RunRequest {
        tool_name: "arm_gcc".to_owned(),
        argv,
        cwd: dir.to_path_buf(),
        extra_env: std::collections::BTreeMap::new(),
        tool_version: tools.versions.get("arm_gcc").cloned(),
    };
    let output = runner::run(cfg, &request)?;
    let record = &output.record;
    let healthy = record.exit.code == Some(0)
        && !record.exit.timed_out
        && !record.exit.tree_kill_sent
        && record.stdout.complete
        && record.stderr.complete;
    if !healthy {
        let text = String::from_utf8_lossy(&output.stderr);
        let excerpt: String = text.chars().take(STDERR_EXCERPT).collect();
        return Err(HarnessError::Build {
            step: "arm-none-eabi-gcc link of driver.elf".to_owned(),
            detail: format!(
                "exit code {:?}, signal {:?}, timed out {}; stderr: {excerpt}",
                record.exit.code, record.exit.signal, record.exit.timed_out
            ),
        });
    }
    let elf = std::fs::read(&elf_path).map_err(|err| io("cannot read", &elf_path, err))?;
    Ok(DriverBuild {
        dir: dir.to_path_buf(),
        source_path,
        elf_path,
        source_sha256: sha256_hex(source.as_bytes()),
        elf_sha256: sha256_hex(&elf),
        records: vec![output.record],
    })
}

/// True for one line of the entry driver's include prefix: a single
/// `#include`, `#define` or `#undef` directive with no control character but a
/// tab. The lines come from Phase 2's probe builder, which checks them the same
/// way; this is the second check, at the point they enter C source.
fn is_prefix_line(line: &str) -> bool {
    let Some(directive) = line.strip_prefix('#').map(str::trim_start) else {
        return false;
    };
    let known = ["include", "define", "undef"].iter().any(|name| {
        directive
            .strip_prefix(name)
            .is_some_and(|after| after.starts_with([' ', '\t']) && !after.trim().is_empty())
    });
    known && !line.chars().any(|c| c.is_control() && c != '\t')
}

/// Render the driver of one entry from the shared template: the include lines
/// of the defining unit and the entry header, and the call the adapter builds.
/// No other text from the manifest reaches the source (CONTEXT D-11, T-03-14).
///
/// # Errors
/// [`HarnessError::Render`] for an include line that is not a single directive,
/// an entry name that is not a C identifier, or a template that fails to render.
pub fn render_entry_driver(
    adapter: &Adapter,
    include_lines: &[String],
) -> Result<String, HarnessError> {
    if let Some(bad) = include_lines.iter().find(|line| !is_prefix_line(line)) {
        return Err(HarnessError::Render(format!(
            "{bad:?} is not a single #include, #define or #undef line"
        )));
    }
    let name = adapter.entry_name();
    if name.is_empty()
        || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        || name.starts_with(|c: char| c.is_ascii_digit())
    {
        return Err(HarnessError::Render(format!(
            "entry name {name:?} is not a C identifier"
        )));
    }
    render(&Placeholders::for_entry(
        &Budgets::DEFAULT,
        include_lines.join("\n"),
        adapter.states_fragment(),
        adapter.setup_fragment(),
        adapter.call_expression(),
    ))
}
