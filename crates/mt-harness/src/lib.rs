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
//! - [`driver`] compiles and links a rendered driver through the runner and
//!   renders an entry's driver,
//! - [`adapter`] turns an entry's manifest roles into the call, the `argv` and
//!   the scalar combinations,
//! - [`link`] checks the captured objects and links the driver against them,
//! - [`globals`] finds the writable globals of the objects and their addresses,
//! - [`layout`] reads a type's layout from the pinned GCC's DWARF text,
//! - [`probe`] compiles the probes that DWARF comes from,
//! - [`decode`] turns the bytes of a state object into typed values,
//! - [`exec`] runs a driver under `qemu-arm` through the runner,
//! - [`frame`] parses the result frame the driver writes,
//! - [`classify`] maps a finished run to exactly one outcome,
//! - [`run`] builds the drivers of a selection and runs the seeds into corpora,
//! - [`replay`] compares corpora,
//! - [`selftest`] runs the executor's own cases in the pinned image.
//!
//! Every external program runs only through `mt_toolchain::runner`.

pub mod adapter;
pub mod budget;
pub mod classify;
pub mod decode;
pub mod driver;
pub mod exec;
pub mod frame;
pub mod globals;
pub mod layout;
pub mod link;
pub mod probe;
pub mod replay;
pub mod run;
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
    /// A capture directory cannot be used as the code under test.
    #[error("capture {}: {detail}", path.display())]
    Capture {
        /// The capture directory or file.
        path: PathBuf,
        /// What is wrong.
        detail: String,
    },
    /// An object differs from the hash its capture recorded (CONTEXT D-19).
    #[error(
        "object {object} of {}: capture.json records sha256 {expected}, the file has {actual}",
        capture.display()
    )]
    ObjectMismatch {
        /// The capture directory.
        capture: PathBuf,
        /// The object's path relative to the objects directory.
        object: String,
        /// The recorded hash.
        expected: String,
        /// The recomputed hash.
        actual: String,
    },
    /// A type's layout could not be read from the compiler's DWARF.
    #[error("layout of {what}: {source}")]
    Layout {
        /// What was being laid out.
        what: String,
        /// Why it cannot be.
        #[source]
        source: layout::LayoutError,
    },
    /// A manifest, policy, seed index or build record could not be used.
    #[error("{what}: {detail}")]
    Input {
        /// What was being read.
        what: String,
        /// Why it cannot be used.
        detail: String,
    },
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
