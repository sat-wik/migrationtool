#![forbid(unsafe_code)]
//! The `mt` command line. Argument parsing only; the logic lives in the library crates.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Context;
use clap::{Parser, Subcommand};
use mt_toolchain::check::run_check;
use mt_toolchain::pins::Pins;

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

#[derive(Debug, Subcommand)]
enum ToolchainAction {
    /// Compare every live tool with its pin. Exit 0 when all match, 1 on drift,
    /// 2 when the pins file cannot be read or is invalid.
    Check {
        /// Path to the pins file.
        #[arg(long)]
        pins: PathBuf,
    },
}

fn check(pins_path: &Path) -> anyhow::Result<bool> {
    let pins = Pins::load(pins_path)
        .with_context(|| format!("loading pins from {}", pins_path.display()))?;
    let cwd = std::env::current_dir().context("reading the current directory")?;
    let report = run_check(&pins, &cwd).context("running the toolchain check")?;
    print!("{}", report.render_table());
    Ok(report.passed())
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.group {
        Group::Toolchain {
            action: ToolchainAction::Check { pins },
        } => match check(&pins) {
            Ok(true) => ExitCode::SUCCESS,
            Ok(false) => ExitCode::from(1),
            Err(err) => {
                eprintln!("error: {err:#}");
                ExitCode::from(2)
            }
        },
    }
}
