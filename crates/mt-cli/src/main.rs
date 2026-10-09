#![forbid(unsafe_code)]
//! The `mt` command line. Argument parsing only; the logic lives in the library crates.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Context;
use clap::{Parser, Subcommand, ValueEnum};
use mt_toolchain::build_args::{self, Scope};
use mt_toolchain::check::{capture_toml, run_check, run_check_observed};
use mt_toolchain::manifest;
use mt_toolchain::pins::{Pins, PinsError};

#[derive(Debug, Parser)]
#[command(name = "mt", about = "migrationtool", version)]
struct Cli {
    #[command(subcommand)]
    group: Group,
}

#[derive(Debug, Subcommand)]
enum Group {
    /// Inspect and verify the pinned toolchain.
    Toolchain {
        #[command(subcommand)]
        action: ToolchainAction,
    },
}

/// Which build arguments `build-args` prints.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum ScopeArg {
    /// Arguments the image build consumes.
    Image,
    /// Pins of the CI host tools.
    Ci,
}

impl From<ScopeArg> for Scope {
    fn from(arg: ScopeArg) -> Self {
        match arg {
            ScopeArg::Image => Self::Image,
            ScopeArg::Ci => Self::Ci,
        }
    }
}

#[derive(Debug, Subcommand)]
enum ToolchainAction {
    /// Compare every pinned item (tools, Rust toolchains, apt packages, the LLVM
    /// major) with its live value. Exit 0 when all match, 1 on drift, a missing
    /// item or an unproven one, 2 when the pins file cannot be read, is invalid
    /// or is incomplete.
    Check {
        /// Path to the pins file.
        #[arg(long)]
        pins: PathBuf,
        /// Also write every command's output to this TOML file, in the layout
        /// of the representative test fixtures.
        #[arg(long)]
        capture_outputs: Option<PathBuf>,
        /// Text recorded as `source` in the capture file.
        #[arg(long, requires = "capture_outputs")]
        capture_label: Option<String>,
    },
    /// Print the pins as sorted NAME=value Docker build arguments. Exit 0 on
    /// success, 2 when the pins are invalid or a value is unsafe to pass.
    BuildArgs {
        /// Path to the pins file.
        #[arg(long)]
        pins: PathBuf,
        /// Which set of arguments to print.
        #[arg(long, value_enum, default_value_t = ScopeArg::Image)]
        scope: ScopeArg,
    },
    /// Run the check and write the deterministic JSON tool-version manifest
    /// (stdout when --out is absent; the check table then goes to stderr).
    /// Exit 0 when the check passes, 1 when any item failed (the manifest is
    /// still written so CI can diff it), 2 for invalid or incomplete pins.
    Manifest {
        /// Path to the pins file.
        #[arg(long)]
        pins: PathBuf,
        /// Write the manifest here instead of stdout.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Compare two manifests. Exit 0 when the files are byte-identical, 1 after
    /// printing `first difference at <dotted.path>` otherwise, 2 when they
    /// differ and cannot be read as JSON.
    ManifestDiff {
        /// The first manifest.
        first: PathBuf,
        /// The second manifest.
        second: PathBuf,
    },
}

/// How much of the pins file must be present.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Completeness {
    /// Every value valid (enough to render build arguments).
    Valid,
    /// Valid and describing the whole container (required to check one).
    Complete,
}

/// Load a pins file and validate every value, printing each issue on its own line.
fn load_validated(pins_path: &Path, completeness: Completeness) -> anyhow::Result<Pins> {
    let pins = Pins::load(pins_path)
        .with_context(|| format!("loading pins from {}", pins_path.display()))?;
    let verdict = match completeness {
        Completeness::Valid => pins.validate(),
        Completeness::Complete => pins.validate_complete(),
    };
    if let Err(err) = verdict {
        if let PinsError::Invalid { issues } = &err {
            for issue in issues {
                eprintln!("error: invalid pins: {issue}");
            }
        }
        return Err(err).with_context(|| format!("pins file {} is not valid", pins_path.display()));
    }
    Ok(pins)
}

fn check(
    pins_path: &Path,
    capture_outputs: Option<&Path>,
    capture_label: Option<&str>,
) -> anyhow::Result<bool> {
    let pins = load_validated(pins_path, Completeness::Complete)?;
    let cwd = std::env::current_dir().context("reading the current directory")?;
    let (report, observations) =
        run_check_observed(&pins, &cwd).context("running the toolchain check")?;
    if let Some(path) = capture_outputs {
        let label = capture_label.unwrap_or("captured by mt toolchain check");
        let text = capture_toml(&observations, label).context("rendering the capture file")?;
        std::fs::write(path, text)
            .with_context(|| format!("writing the capture file {}", path.display()))?;
    }
    print!("{}", report.render_table());
    Ok(report.passed())
}

fn manifest_command(pins_path: &Path, out: Option<&Path>) -> anyhow::Result<bool> {
    let pins = load_validated(pins_path, Completeness::Complete)?;
    let cwd = std::env::current_dir().context("reading the current directory")?;
    let report = run_check(&pins, &cwd).context("running the toolchain check")?;
    let text =
        manifest::to_json(&manifest::build(&pins, &report)).context("building the manifest")?;
    match out {
        Some(path) => std::fs::write(path, &text)
            .with_context(|| format!("writing the manifest {}", path.display()))?,
        None => print!("{text}"),
    }
    // The table never goes to stdout here, which may be carrying the manifest.
    eprint!("{}", report.render_table());
    Ok(report.passed())
}

/// True when the two manifests are identical.
fn manifest_diff_command(first: &Path, second: &Path) -> anyhow::Result<bool> {
    let read =
        |path: &Path| std::fs::read(path).with_context(|| format!("reading {}", path.display()));
    let (left, right) = (read(first)?, read(second)?);
    if left == right {
        return Ok(true);
    }
    let text = |bytes: Vec<u8>, path: &Path| {
        String::from_utf8(bytes).with_context(|| format!("{} is not UTF-8 text", path.display()))
    };
    let (left, right) = (text(left, first)?, text(right, second)?);
    match manifest::first_difference(&left, &right).context("comparing the manifests")? {
        Some(path) => println!("first difference at {path}"),
        None => println!("manifests differ only in formatting, not in content"),
    }
    Ok(false)
}

fn build_args_command(pins_path: &Path, scope: Scope) -> anyhow::Result<()> {
    let pins = load_validated(pins_path, Completeness::Valid)?;
    let lines = build_args::render_lines(&pins, scope).context("rendering build arguments")?;
    print!("{lines}");
    Ok(())
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let Group::Toolchain { action } = cli.group;
    let result = match action {
        ToolchainAction::Check {
            pins,
            capture_outputs,
            capture_label,
        } => check(&pins, capture_outputs.as_deref(), capture_label.as_deref())
            .map(|passed| u8::from(!passed)),
        ToolchainAction::BuildArgs { pins, scope } => {
            build_args_command(&pins, scope.into()).map(|()| 0)
        }
        ToolchainAction::Manifest { pins, out } => {
            manifest_command(&pins, out.as_deref()).map(|passed| u8::from(!passed))
        }
        ToolchainAction::ManifestDiff { first, second } => {
            manifest_diff_command(&first, &second).map(|identical| u8::from(!identical))
        }
    };
    match result {
        Ok(code) => ExitCode::from(code),
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::from(2)
        }
    }
}
