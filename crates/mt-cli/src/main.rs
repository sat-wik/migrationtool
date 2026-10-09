#![forbid(unsafe_code)]
//! The `mt` command line. Argument parsing only; the logic lives in the library crates.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Context;
use clap::{Parser, Subcommand, ValueEnum};
use mt_toolchain::build_args::{self, Scope};
use mt_toolchain::check::run_check;
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
    /// Compare every live tool with its pin. Exit 0 when all match, 1 on drift,
    /// 2 when the pins file cannot be read or is invalid.
    Check {
        /// Path to the pins file.
        #[arg(long)]
        pins: PathBuf,
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
}

/// Load a pins file and validate every value, printing each issue on its own line.
fn load_validated(pins_path: &Path) -> anyhow::Result<Pins> {
    let pins = Pins::load(pins_path)
        .with_context(|| format!("loading pins from {}", pins_path.display()))?;
    if let Err(err) = pins.validate() {
        if let PinsError::Invalid { issues } = &err {
            for issue in issues {
                eprintln!("error: invalid pins: {issue}");
            }
        }
        return Err(err).with_context(|| format!("pins file {} is not valid", pins_path.display()));
    }
    Ok(pins)
}

fn check(pins_path: &Path) -> anyhow::Result<bool> {
    let pins = load_validated(pins_path)?;
    let cwd = std::env::current_dir().context("reading the current directory")?;
    let report = run_check(&pins, &cwd).context("running the toolchain check")?;
    print!("{}", report.render_table());
    Ok(report.passed())
}

fn build_args_command(pins_path: &Path, scope: Scope) -> anyhow::Result<()> {
    let pins = load_validated(pins_path)?;
    let lines = build_args::render_lines(&pins, scope).context("rendering build arguments")?;
    print!("{lines}");
    Ok(())
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let Group::Toolchain { action } = cli.group;
    let result = match action {
        ToolchainAction::Check { pins } => check(&pins).map(|passed| u8::from(!passed)),
        ToolchainAction::BuildArgs { pins, scope } => {
            build_args_command(&pins, scope.into()).map(|()| 0)
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
