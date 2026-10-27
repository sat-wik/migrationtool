//! Running a built driver under `qemu-arm` through the runner (CONTEXT D-02, D-30).
//!
//! The environment is the runner's fixed four-key base with no extra variable.
//! The runner has no stdin, so the request travels in `req.bin` in a fixed
//! working directory (the slot). The memory budget is `-R`, the reserved guest
//! address space. A run is never retried.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use mt_toolchain::runner::{self, RunOutput, RunRequest, RunnerConfig};

use crate::HarnessError;
use crate::budget::Budgets;
use crate::frame::{FRAME_VERSION, REQUEST_MAGIC};
use crate::tools::ExecTools;

/// The name of the request file inside the slot.
pub const REQUEST_FILE: &str = "req.bin";

/// The CPU model every run uses (CONTEXT D-30).
pub const CPU_MODEL: &str = "cortex-m4";

/// A fixed working directory, created empty, that holds only `req.bin`.
#[derive(Debug, Clone)]
pub struct Slot {
    dir: PathBuf,
}

impl Slot {
    /// Create `dir` empty, removing what a previous run left there.
    ///
    /// # Errors
    /// [`HarnessError::Io`] when the directory cannot be recreated.
    pub fn create(dir: &Path) -> Result<Self, HarnessError> {
        let io = |action, source| HarnessError::Io {
            action,
            path: dir.to_path_buf(),
            source,
        };
        if dir.exists() {
            std::fs::remove_dir_all(dir).map_err(|err| io("cannot clear", err))?;
        }
        std::fs::create_dir_all(dir).map_err(|err| io("cannot create", err))?;
        Ok(Self {
            dir: dir.to_path_buf(),
        })
    }

    /// The directory.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }
}

/// The fields of one request file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Request {
    /// The fields, in order.
    pub fields: Vec<Vec<u8>>,
}

impl Request {
    /// The bytes of `req.bin`: magic, version, field count, then each field with
    /// its length.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&REQUEST_MAGIC);
        out.extend_from_slice(&FRAME_VERSION.to_le_bytes());
        let count = u32::try_from(self.fields.len()).unwrap_or(u32::MAX);
        out.extend_from_slice(&count.to_le_bytes());
        for field in &self.fields {
            let len = u32::try_from(field.len()).unwrap_or(u32::MAX);
            out.extend_from_slice(&len.to_le_bytes());
            out.extend_from_slice(field);
        }
        out
    }
}

/// The `qemu-arm` options of every run, before the driver path: the CPU model,
/// the reserved guest address space (the memory budget) and a fixed seed.
#[must_use]
pub fn qemu_options(budgets: &Budgets) -> Vec<OsString> {
    vec![
        OsString::from("-cpu"),
        OsString::from(CPU_MODEL),
        OsString::from("-R"),
        OsString::from(format!("{:#x}", budgets.reserved_va)),
        OsString::from("-seed"),
        OsString::from("0"),
    ]
}

/// The `qemu-arm` argument list for one driver run.
#[must_use]
pub fn qemu_args(budgets: &Budgets, elf: &Path) -> Vec<OsString> {
    let mut args = qemu_options(budgets);
    args.push(elf.as_os_str().to_owned());
    args.push(OsString::from(REQUEST_FILE));
    args
}

/// Write `request` into the slot and run the driver once.
///
/// # Errors
/// [`HarnessError::Io`] when `req.bin` cannot be written and
/// [`HarnessError::Runner`] when `qemu-arm` cannot be started. A crash, a
/// timeout or a bad frame is not an error here: it is the run's result, judged
/// by [`crate::classify::classify`].
pub fn run_driver(
    tools: &ExecTools,
    cfg: &RunnerConfig,
    budgets: &Budgets,
    elf: &Path,
    slot: &Slot,
    request: &Request,
) -> Result<RunOutput, HarnessError> {
    run_driver_with(
        tools,
        cfg,
        budgets,
        &qemu_options(budgets),
        elf,
        slot,
        request,
    )
}

/// [`run_driver`] with explicit `qemu-arm` options. Only the self-test's failure
/// diagnostics use other options than [`qemu_options`]; a vector is never made
/// from such a run.
///
/// # Errors
/// As [`run_driver`].
pub fn run_driver_with(
    tools: &ExecTools,
    cfg: &RunnerConfig,
    budgets: &Budgets,
    options: &[OsString],
    elf: &Path,
    slot: &Slot,
    request: &Request,
) -> Result<RunOutput, HarnessError> {
    let request_path = slot.dir().join(REQUEST_FILE);
    std::fs::write(&request_path, request.to_bytes()).map_err(|source| HarnessError::Io {
        action: "cannot write",
        path: request_path.clone(),
        source,
    })?;
    // One RunnerConfig per budget: the timeout is config-wide in the runner.
    let mut run_cfg = cfg.clone();
    run_cfg.timeout = budgets.timeout();
    let mut argv: Vec<OsString> = vec![tools.qemu_arm.clone().into_os_string()];
    argv.extend(options.iter().cloned());
    argv.push(elf.as_os_str().to_owned());
    argv.push(OsString::from(REQUEST_FILE));
    let run_request = RunRequest {
        tool_name: "qemu_arm".to_owned(),
        argv,
        cwd: slot.dir().to_path_buf(),
        extra_env: BTreeMap::new(),
        tool_version: tools.versions.get("qemu_arm").cloned(),
    };
    Ok(runner::run(&run_cfg, &run_request)?)
}
