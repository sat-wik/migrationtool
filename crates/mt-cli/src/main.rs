#![forbid(unsafe_code)]
//! The `mt` command line. Argument parsing only; the logic lives in the library crates.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use anyhow::Context;
use clap::{Args, Parser, Subcommand, ValueEnum};
use mt_capture::capture::CaptureError;
use mt_capture::compare::{DETERMINISM_EXCLUSIONS, compare_capture_dirs};
use mt_capture::manifest::ManifestError;
use mt_capture::seeding::run_seeds_detailed;
use mt_capture::testbed::{
    Selection, StepSummary, TestbedError, TestbedOptions, run_capture, run_capture_build,
    run_capture_record, run_fetch, run_pairs, run_replay, run_validate,
};
use mt_toolchain::build_args::{self, Scope};
use mt_toolchain::check::{capture_toml, run_check, run_check_observed};
use mt_toolchain::pins::{Pins, PinsError};
use mt_toolchain::{fetch, manifest};

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
    /// Fetch, capture and replay the open-source test bed.
    Testbed {
        #[command(subcommand)]
        action: TestbedAction,
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
    /// Print the sha256 of an https:// or file:// URL (a pin-bump helper): the
    /// digest and the URL, separated by two spaces. Exit 0 on success, 1 when
    /// --expect differs, 2 for a refused scheme, a malformed --expect or a
    /// failed fetch.
    Hash {
        /// The URL to hash.
        url: String,
        /// The digest it must have (64 hex digits).
        #[arg(long)]
        expect: Option<String>,
        /// Give up after this many seconds.
        #[arg(long, default_value_t = 1800, value_parser = clap::value_parser!(u64).range(1..))]
        timeout_secs: u64,
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

/// Which stage of a capture to run (WR-04).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum StageArg {
    /// The untrusted upstream build of one pair, writing only its run directory.
    Build,
    /// The evidence from what the builds left, running no upstream code.
    Record,
}

/// The arguments the fetch, capture and replay steps share.
#[derive(Debug, Args)]
struct TestbedArgs {
    /// Path to the test-bed manifest (`testbed/manifest.toml`). The directory
    /// holding it is the committed `testbed/` directory.
    #[arg(long)]
    manifest: PathBuf,
    /// Path to the pins file.
    #[arg(long)]
    pins: PathBuf,
    /// The work directory: fetched sources, run trees and capture artifacts.
    /// Defaults to $MT_WORK_DIR, else `.mt-work`.
    #[arg(long)]
    work_dir: Option<PathBuf>,
    /// Where reports and summaries are written. Defaults to `<work-dir>/reports`.
    #[arg(long)]
    out: Option<PathBuf>,
    /// Run on every target of the manifest.
    #[arg(long, conflicts_with = "ids", required_unless_present = "ids")]
    all: bool,
    /// Run on these target ids.
    ids: Vec<String>,
}

#[derive(Debug, Subcommand)]
enum TestbedAction {
    /// Check the manifest. Exit 0 when it is valid, 2 when it cannot be read or
    /// is invalid.
    Validate {
        /// Path to the test-bed manifest.
        #[arg(long)]
        manifest: PathBuf,
    },
    /// Fetch each source by its pinned commit (or archive) into the work
    /// directory and verify its tree hash. The only step that needs the network.
    /// Exit 0 when every source is fetched or reused, 1 when one failed, 2 for
    /// invalid input.
    Fetch(TestbedArgs),
    /// Build each configuration of each target under capture and check the
    /// compiler, the object architecture and every entry function. Exit 0 when
    /// every configuration is captured, 1 when one failed or was not proven, 2
    /// for invalid input (including a target with no build recipe).
    ///
    /// With --stage the capture runs in two parts, so the untrusted upstream
    /// build can run where it may write only its own run directory. --stage
    /// build --config <NAME> <ID> runs the build stage of that one pair and
    /// prints `<id>/<config> built`, `build_failed` or `not_proven`, or `dropped
    /// (<cause>)` for a pair of the drop log (nothing is run or written for it):
    /// exit 0 when a build record was written whatever the build's outcome, 1
    /// when the source cache is missing or does not verify, 2 for invalid input
    /// (it needs exactly one id, --config, and not --all). --stage record runs
    /// no upstream code and writes the capture artifacts, rows and reports over
    /// the selection as the capture does, with the same exit codes; a pair
    /// without a usable build record is a not_proven row. Without --stage both
    /// parts run in one process.
    Capture {
        #[command(flatten)]
        args: TestbedArgs,
        /// The toolchain image reference to record in each capture report.
        #[arg(long)]
        image: Option<String>,
        /// Run only the build stage or only the record stage of the capture.
        #[arg(long, value_enum)]
        stage: Option<StageArg>,
        /// The configuration to build; only with --stage build.
        #[arg(long)]
        config: Option<String>,
    },
    /// List the retained pairs of the selected targets, one `<id> <config>` per
    /// line: targets by id, each target's default first, then its other
    /// configurations by name, without the pairs of the drop log. Nothing is
    /// written. Exit 0, or 2 for invalid input.
    Pairs(TestbedArgs),
    /// Replay each captured build in a clean copy of its source and compare the
    /// objects. Exit 0 when every configuration replays identically (or
    /// identically after the fixed normalisation), 1 when one differs, failed or
    /// was not proven, 2 for invalid input.
    Replay(TestbedArgs),
    /// Extract each target's seed corpus from its verified cached source (and its
    /// committed hand-made inputs) into the seeds directory, or with --check
    /// regenerate it in memory and compare. Needs no network. Exit 0 when every
    /// corpus was written (or matches), 1 when one drifted or failed, 2 for invalid
    /// input.
    Seeds {
        #[command(flatten)]
        args: TestbedArgs,
        /// Where the corpora are (default: `seeds` next to the manifest).
        #[arg(long)]
        seeds_dir: Option<PathBuf>,
        /// Change nothing: report drift between the committed corpus and the
        /// regenerated one.
        #[arg(long)]
        check: bool,
    },
    /// Compare two capture output trees, as the determinism step does. Exit 0
    /// when identical, 1 when any file differs, is missing on one side, is empty
    /// or cannot be parsed, or no file was compared; 2 when a tree cannot be read.
    Compare {
        /// The first capture output tree.
        #[arg(long)]
        first: PathBuf,
        /// The second capture output tree.
        #[arg(long)]
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

/// True when no `--expect` was given or the digest equals it.
fn hash_command(url: &str, expect: Option<&str>, timeout_secs: u64) -> anyhow::Result<bool> {
    let expected = match expect {
        None => None,
        Some(text) => {
            let lower = text.to_ascii_lowercase();
            anyhow::ensure!(
                lower.len() == 64 && lower.bytes().all(|b| b.is_ascii_hexdigit()),
                "--expect must be 64 hexadecimal digits, found {text:?}"
            );
            Some(lower)
        }
    };
    let digest =
        fetch::sha256_of_url(url, Duration::from_secs(timeout_secs)).context("hashing the URL")?;
    println!("{digest}  {url}");
    match expected {
        Some(want) if want != digest => {
            eprintln!("error: expected {want}, got {digest}");
            Ok(false)
        }
        _ => Ok(true),
    }
}

fn build_args_command(pins_path: &Path, scope: Scope) -> anyhow::Result<()> {
    let pins = load_validated(pins_path, Completeness::Valid)?;
    let lines = build_args::render_lines(&pins, scope).context("rendering build arguments")?;
    print!("{lines}");
    Ok(())
}

/// The options of a testbed step: the work directory is `--work-dir`, else
/// `$MT_WORK_DIR`, else `.mt-work`.
fn testbed_options(args: TestbedArgs, image: Option<String>) -> TestbedOptions {
    let chosen = args
        .work_dir
        .or_else(|| std::env::var_os("MT_WORK_DIR").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from(".mt-work"));
    // Tools run with the cache directory as their working directory, so a
    // relative path would be read from the wrong place. Keep a relative one
    // as given only if the current directory cannot be read (the step then
    // fails with a clear path in its message).
    let work_dir = std::path::absolute(&chosen).unwrap_or(chosen);
    let testbed_dir = match args.manifest.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => PathBuf::from("."),
    };
    TestbedOptions {
        out_dir: args.out.unwrap_or_else(|| work_dir.join("reports")),
        manifest_path: args.manifest,
        pins_path: args.pins,
        testbed_dir,
        work_dir,
        selection: if args.all {
            Selection::All
        } else {
            Selection::Ids(args.ids)
        },
        image,
    }
}

/// True when the two capture output trees are identical (WR-05).
fn compare_command(first: &Path, second: &Path) -> anyhow::Result<bool> {
    let comparison = compare_capture_dirs(first, second).context("comparing the capture trees")?;
    println!("compared {} files", comparison.compared);
    for exclusion in DETERMINISM_EXCLUSIONS {
        println!("excluded: {exclusion}");
    }
    if comparison.passed() {
        println!("result: identical");
    } else {
        println!("result: differences found");
        for problem in &comparison.problems {
            println!("{problem}");
        }
        if comparison.compared == 0 {
            println!("no file was compared");
        }
    }
    Ok(comparison.passed())
}

/// Print each validation issue on its own line.
fn print_issues(error: &TestbedError) {
    match error {
        TestbedError::Manifest(ManifestError::Invalid { issues }) => {
            for issue in issues {
                eprintln!("error: invalid manifest: {issue}");
            }
        }
        TestbedError::Pins(PinsError::Invalid { issues }) => {
            for issue in issues {
                eprintln!("error: invalid pins: {issue}");
            }
        }
        _ => {}
    }
}

/// Print the table of a finished step; true when every row passed.
fn finish_testbed(result: Result<StepSummary, TestbedError>) -> anyhow::Result<bool> {
    match result {
        Ok(summary) => {
            print!("{}", summary.render_table());
            Ok(summary.passed)
        }
        Err(error) => {
            print_issues(&error);
            Err(error.into())
        }
    }
}

/// The capture command: both stages in one process, or one stage (WR-04). Only
/// the shape of the arguments is checked here.
fn capture_command(
    args: TestbedArgs,
    image: Option<String>,
    stage: Option<StageArg>,
    config: Option<String>,
) -> anyhow::Result<bool> {
    match (stage, config) {
        (Some(StageArg::Build), config) => {
            let (Some(config), false, [id]) = (config, args.all, args.ids.as_slice()) else {
                anyhow::bail!(
                    "--stage build runs one pair: name exactly one target id and --config <NAME>, and do not use --all"
                );
            };
            let id = id.clone();
            match run_capture_build(&testbed_options(args, image), &id, &config) {
                Ok(word) => {
                    println!("{id}/{config} {word}");
                    Ok(true)
                }
                // The cache does not verify: exit 1 with the reason, not 2.
                Err(TestbedError::Capture(CaptureError::Fetch(error))) => {
                    eprintln!("error: {error}");
                    Ok(false)
                }
                Err(error) => {
                    print_issues(&error);
                    Err(error.into())
                }
            }
        }
        (_, Some(_)) => anyhow::bail!("--config is only valid with --stage build"),
        (Some(StageArg::Record), None) => {
            finish_testbed(run_capture_record(&testbed_options(args, image)))
        }
        (None, None) => finish_testbed(run_capture(&testbed_options(args, image))),
    }
}

/// Print the retained pairs, one per line.
fn pairs_command(args: TestbedArgs) -> anyhow::Result<bool> {
    match run_pairs(&testbed_options(args, None)) {
        Ok(pairs) => {
            for (id, config) in pairs {
                println!("{id} {config}");
            }
            Ok(true)
        }
        Err(error) => {
            print_issues(&error);
            Err(error.into())
        }
    }
}

fn testbed_command(action: TestbedAction) -> anyhow::Result<bool> {
    match action {
        TestbedAction::Validate { manifest } => finish_testbed(run_validate(&manifest)),
        TestbedAction::Fetch(args) => finish_testbed(run_fetch(&testbed_options(args, None))),
        TestbedAction::Capture {
            args,
            image,
            stage,
            config,
        } => capture_command(args, image, stage, config),
        TestbedAction::Pairs(args) => pairs_command(args),
        TestbedAction::Replay(args) => finish_testbed(run_replay(&testbed_options(args, None))),
        TestbedAction::Compare { first, second } => compare_command(&first, &second),
        TestbedAction::Seeds {
            args,
            seeds_dir,
            check,
        } => {
            let opts = testbed_options(args, None);
            let seeds_dir = seeds_dir.unwrap_or_else(|| opts.testbed_dir.join("seeds"));
            match run_seeds_detailed(&opts, &seeds_dir, check) {
                Ok(run) => {
                    print!("{}", run.summary.render_table());
                    for (id, lines) in &run.drift {
                        for line in lines {
                            eprintln!("drift: {id}: {line}");
                        }
                    }
                    Ok(run.summary.passed)
                }
                Err(error) => {
                    print_issues(&error);
                    Err(error.into())
                }
            }
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.group {
        Group::Toolchain { action } => toolchain_command(action),
        Group::Testbed { action } => testbed_command(action).map(|passed| u8::from(!passed)),
    };
    match result {
        Ok(code) => ExitCode::from(code),
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::from(2)
        }
    }
}

fn toolchain_command(action: ToolchainAction) -> anyhow::Result<u8> {
    match action {
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
        ToolchainAction::Hash {
            url,
            expect,
            timeout_secs,
        } => hash_command(&url, expect.as_deref(), timeout_secs).map(|ok| u8::from(!ok)),
        ToolchainAction::ManifestDiff { first, second } => {
            manifest_diff_command(&first, &second).map(|identical| u8::from(!identical))
        }
    }
}
