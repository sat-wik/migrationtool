#![forbid(unsafe_code)]
//! The C reference executor (Phase 3, EQV-04).
//!
//! The executor produces the reference result of the original C by running real
//! Cortex-M code under the pinned `qemu-arm` user mode (CONTEXT D-01). It is
//! not a firmware runner: the unit under test is linked into a small
//! hosted-style Linux-ABI static ELF whose `_start`, stack, guarded buffers and
//! fault handlers come from one shared, reviewed C template
//! ([`runtime::RUNTIME_TEMPLATE`], D-29, D-30).
//!
//! - [`budget`] holds the fixed run budgets,
//! - [`tools`] the pinned tool paths,
//! - [`runtime`] renders the template,
//! - [`driver`] compiles and links a rendered driver through the runner,
//! - [`exec`] runs a driver under `qemu-arm` through the runner,
//! - [`frame`] parses the result frame the driver writes,
//! - [`classify`] maps a finished run to exactly one outcome,
//! - [`selftest`] runs the executor's own cases in the pinned image.
//!
//! Every external program runs only through `mt_toolchain::runner`.

pub mod budget;
pub mod classify;
pub mod driver;
pub mod exec;
pub mod frame;
pub mod runtime;
pub mod selftest;
pub mod tools;

use std::path::PathBuf;

use mt_toolchain::runner::RunnerError;

/// Version of the executor, recorded in every provenance record (D-19).
pub const EXECUTOR_VERSION: &str = "0.1.0";

/// Why the executor could not do what was asked.
#[derive(Debug, thiserror::Error)]
pub enum HarnessError {
    /// A tool the executor needs is not an installed pin.
    #[error("tool {key} is not an installed pin in the pins file")]
    ToolNotPinned {
        /// The pins key.
        key: String,
    },
    /// Tools the executor needs are not installed at their pinned paths.
    #[error("missing tools: {}", describe_missing(.tools))]
    ToolsMissing {
        /// Each missing tool: its name and the path that was looked at.
        tools: Vec<(String, PathBuf)>,
    },
    /// The runtime template could not be rendered.
    #[error("cannot render the runtime template: {0}")]
    Render(String),
    /// A file or directory could not be read or written.
    #[error("{action} {}: {source}", path.display())]
    Io {
        /// What was being done.
        action: &'static str,
        /// The path.
        path: PathBuf,
        /// The underlying error.
        #[source]
        source: std::io::Error,
    },
    /// The runner could not run a program.
    #[error(transparent)]
    Runner(#[from] RunnerError),
    /// A build step ended without producing its output.
    #[error("{step} failed: {detail}")]
    Build {
        /// The step.
        step: String,
        /// Exit status and a stderr excerpt.
        detail: String,
    },
    /// A result frame is malformed.
    #[error(transparent)]
    Frame(#[from] frame::FrameError),
}

fn describe_missing(tools: &[(String, PathBuf)]) -> String {
    tools
        .iter()
        .map(|(name, path)| format!("{name} at {}", path.display()))
        .collect::<Vec<_>>()
        .join("; ")
}

/// Lowercase hex of `bytes`.
pub(crate) fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        // Writing into a String cannot fail.
        let _ = write!(text, "{byte:02x}");
    }
    text
}

/// Lowercase hex sha256 of `bytes`.
pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    hex(&Sha256::digest(bytes))
}
