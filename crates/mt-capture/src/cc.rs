//! The canonical `compile_commands.json` form (BUILD-01, CONTEXT D-07).
//!
//! Every argv this crate ever executes comes from a compile-commands file, so
//! the form is strict: `arguments` arrays only, an absolute output derived
//! from the arguments, a compiler allow-list and a run-root containment check.
//! Nothing here runs a shell or spawns a process.
//!
//! # The gate on a recorded command (plan 02-20, threats T-02-67 and T-02-70)
//!
//! Besides the compile-only, allow-list, `-flto`/`-fprofile-*` and run-root
//! rules, an entry is refused when
//!
//! - an argument makes the GCC driver run or load a program other than the
//!   pinned compiler's own ([`HELPER_OPTION_PREFIXES`], [`find_helper_option`],
//!   [`CcError::HelperOption`]), or
//! - a file the compiler would write cannot be shown to lie under the run root
//!   ([`check_write_paths`], [`CcError::OutsideRunRoot`] and
//!   [`CcError::WriteRedirect`]); `--output` is resolved exactly like `-o`.
//!
//! [`canonicalise`] applies both at capture and [`CcPolicy::check_entry`] again
//! at replay, before anything is deleted, copied or spawned.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

/// One canonical compile command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CcEntry {
    /// Working directory of the compile.
    pub directory: String,
    /// The source file, absolute or relative to `directory`.
    pub file: String,
    /// The full argv, `arguments[0]` being the compiler.
    pub arguments: Vec<String>,
    /// Absolute path of the object the command writes.
    pub output: String,
}

/// What a compile-commands file may contain before replay will execute it.
#[derive(Debug, Clone)]
pub struct CcPolicy {
    /// The values `argv[0]` may take. Anything else is refused before any
    /// process is spawned.
    pub allowed_compilers: Vec<String>,
    /// Every directory and output must lie under this root.
    pub run_root: PathBuf,
}

/// Why a compile-commands file or entry was refused.
#[derive(Debug, thiserror::Error)]
pub enum CcError {
    /// A `command` string could not be split into words.
    #[error("the command string cannot be split into arguments")]
    Unsplittable,
    /// The entry has neither `arguments` nor `command`.
    #[error("the entry has neither arguments nor command")]
    NoCommand,
    /// `-o` is the last argument.
    #[error("-o has no argument")]
    DanglingOutput,
    /// The source file name has no usable stem.
    #[error("the source file name has no stem")]
    BadFile,
    /// The command has no `-c`, so it is not a compile-only command.
    #[error("the command is not a compile-only command (no -c)")]
    NotCompileOnly,
    /// `argv[0]` is not in the allow-list.
    #[error("compiler {argv0:?} is not in the allow-list")]
    CompilerNotAllowed {
        /// The refused program.
        argv0: String,
    },
    /// An argument makes the object depend on something replay cannot hold fixed.
    #[error("argument {flag} is not supported: the object would not be reproducible")]
    ForbiddenFlag {
        /// The refused argument.
        flag: String,
    },
    /// An argument makes the driver run or load another program.
    #[error(
        "argument {flag} makes the GCC driver run or load a program other than the pinned compiler's own"
    )]
    HelperOption {
        /// The offending argument.
        flag: String,
    },
    /// An argument makes the compiler write a file the gate cannot bound.
    #[error(
        "argument {flag} makes the compiler write a file the gate cannot keep inside the run root"
    )]
    WriteRedirect {
        /// The offending argument.
        flag: String,
    },
    /// A directory or output lies outside the run root.
    #[error("{path} is outside the run root")]
    OutsideRunRoot {
        /// The offending path.
        path: String,
    },
    /// No entry produced an object.
    #[error("no entry produced an object")]
    Empty,
    /// Two entries write the same object.
    #[error("two entries write the same output {output}")]
    DuplicateOutput {
        /// The shared output.
        output: String,
    },
    /// The JSON could not be read or written.
    #[error("compile commands JSON: {0}")]
    Json(#[from] serde_json::Error),
}

/// Remove `.` and resolve `..` textually. A `..` that would climb above the
/// root of an absolute path stays at the root.
pub(crate) fn normalise_lexical(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => match out.components().next_back() {
                Some(Component::Normal(_)) => {
                    out.pop();
                }
                Some(Component::RootDir | Component::Prefix(_)) => {}
                _ => out.push(".."),
            },
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// True when `path`, once `.` and `..` are resolved textually, lies under `root`.
pub(crate) fn lexically_under(root: &Path, path: &Path) -> bool {
    normalise_lexical(path).starts_with(normalise_lexical(root))
}

/// Options whose operand is the next argument and is not an output: it must
/// not be read as an option itself (`-MT -oinside.o` names a target).
const SEPARATE_OPERAND_OPTIONS: [&str; 13] = [
    "-MF",
    "-MT",
    "-MQ",
    "-dumpdir",
    "--dumpdir",
    "-dumpbase",
    "--dumpbase",
    "-dumpbase-ext",
    "--dumpbase-ext",
    "-aux-info",
    "-Xpreprocessor",
    "-Xassembler",
    "--for-assembler",
];

/// The object a command writes: the last `-o <path>`, `-o<path>`,
/// `--output <path>` or `--output=<path>` argument (the last output wins, as it
/// does in GCC), else `<source stem>.o`, resolved against `directory`.
pub(crate) fn derive_output(
    directory: &str,
    file: &str,
    arguments: &[String],
) -> Result<String, CcError> {
    let mut output: Option<String> = None;
    let mut index = 1;
    while index < arguments.len() {
        let argument = &arguments[index];
        if argument == "-o" || argument == "--output" {
            let value = arguments.get(index + 1).ok_or(CcError::DanglingOutput)?;
            output = Some(value.clone());
            index += 2;
            continue;
        }
        if SEPARATE_OPERAND_OPTIONS.contains(&argument.as_str()) {
            index += 2;
            continue;
        }
        if let Some(joined) = argument
            .strip_prefix("--output=")
            .or_else(|| argument.strip_prefix("-o"))
        {
            output = Some(joined.to_owned());
        }
        index += 1;
    }
    let output = match output {
        Some(path) => path,
        None => {
            let stem = Path::new(file)
                .file_stem()
                .and_then(|stem| stem.to_str())
                .ok_or(CcError::BadFile)?;
            format!("{stem}.o")
        }
    };
    let resolved = normalise_lexical(&Path::new(directory).join(output));
    resolved.to_str().map(str::to_owned).ok_or(CcError::BadFile)
}

/// Driver options that make `gcc` run or load a program other than the pinned
/// compiler's own `cc1` and `as` (T-02-67). An argument is refused when it
/// starts with one of these, in any position after `argv[0]` (so an operand is
/// checked too), or when it is a `-Wp,` argument with a comma-separated piece
/// that starts with one (cc1 expands `@file` and loads plugins itself, and the
/// driver parses `-fcompare-debug=` as driver options).
///
/// - `-wrapper`: every subcommand (`cc1`, `as`) runs under the named program.
/// - `-B`: the driver looks for `cc1` and `as` in that directory first.
/// - `--prefix`: the same, through the install prefix.
/// - `-specs`: reads a spec file, and spec strings define the commands the
///   driver runs.
/// - `--specs`: the long spelling of `-specs`.
/// - `-fplugin`: loads a shared object into `cc1`; this covers
///   `-fplugin-arg-<name>-<key>`.
/// - `-iplugindir`: sets the directory plugins load from.
/// - `-fcompare-debug=`: the driver parses the payload as driver options, so it
///   can carry `-B` or `-wrapper`.
/// - `@`: a response file holds more options, any of the above included.
///
/// Considered and not refused, each measured with `arm-none-eabi-gcc -###`
/// (which runs nothing): `--sysroot`, `-isysroot`, `-iprefix`,
/// `--include-prefix`, `-imultilib` and `-no-canonical-prefixes` change header
/// lookup only and leave `cc1` and `as` the pinned programs; `-Wa,`,
/// `-Xassembler` and `--for-assembler` reach only GNU `as` (their write options
/// are refused by [`check_write_paths`]); `-Wl,` and `--for-linker` run nothing
/// for a `-c` command, and the linker never runs for one; `-fuse-ld` is
/// unrecognised; `-foffload` is refused by this toolchain; `-no-integrated-cpp`,
/// `-save-temps`, `-time` and plain `-fcompare-debug` re-run only the pinned
/// `cc1` and `as`. GCC rejects abbreviated and doubled spellings (`--pref=`,
/// `--spec=`, `--wrapper`, `-wrapper=`, `--fplugin=`), so there is no hidden
/// spelling.
///
/// The set was measured with the GCC 13.2 driver against the pinned Arm GNU
/// Toolchain 14.3.Rel1. The GCC 14 documentation pages (Overall, Directory,
/// Preprocessor and Developer Options) could not be fetched when this was
/// written; the set is therefore a measured one, and a prefix is never removed.
pub const HELPER_OPTION_PREFIXES: [&str; 9] = [
    "-wrapper",
    "-B",
    "--prefix",
    "-specs",
    "--specs",
    "-fplugin",
    "-iplugindir",
    "-fcompare-debug=",
    "@",
];

fn is_helper_option(text: &str) -> bool {
    HELPER_OPTION_PREFIXES
        .iter()
        .any(|prefix| text.starts_with(prefix))
}

/// The first argument after `argv[0]` that starts with a prefix of
/// [`HELPER_OPTION_PREFIXES`], or the whole `-Wp,` argument that carries one.
#[must_use]
pub fn find_helper_option(arguments: &[String]) -> Option<&String> {
    arguments.iter().skip(1).find(|argument| {
        is_helper_option(argument)
            || argument
                .strip_prefix("-Wp,")
                .is_some_and(|rest| rest.split(',').any(is_helper_option))
    })
}

/// Options a preprocessor carrier (`-Wp,`, `-Xpreprocessor`) may pass to `cc1`
/// that name a file it writes.
const PREPROCESSOR_WRITE_PREFIXES: [&str; 8] = [
    "-M",
    "-o",
    "-dumpdir",
    "-dumpbase",
    "-aux-info",
    "-fdump-",
    "-fopt-info",
    "-fdeps-file",
];

/// Options an assembler carrier (`-Wa,`, `-Xassembler`, `--for-assembler`) may
/// pass to GNU `as` that name a file it writes (a listing, a dependency file,
/// the output) or that read more options from a file.
const ASSEMBLER_WRITE_PREFIXES: [&str; 4] = ["-a", "--MD", "-o", "@"];

fn writes_through_preprocessor(piece: &str) -> bool {
    PREPROCESSOR_WRITE_PREFIXES
        .iter()
        .any(|prefix| piece.starts_with(prefix))
}

fn writes_through_assembler(piece: &str) -> bool {
    ASSEMBLER_WRITE_PREFIXES
        .iter()
        .any(|prefix| piece.starts_with(prefix))
}

/// Refuse a command that writes a file the gate cannot keep under `run_root`
/// (threat T-02-70). `directory` is the entry's working directory; relative
/// operands are resolved against it, lexically.
///
/// The arguments after `argv[0]` are walked left to right, and an option's
/// separate operand is consumed with it, so a target name is never read as an
/// option. Every occurrence is checked, not only the last one.
///
/// - Path options. The operand must lie under `run_root`
///   ([`CcError::OutsideRunRoot`], carrying the resolved path): `-o` and
///   `-MF` (separate or joined), `--output` (separate or `=`), `-dumpdir`,
///   `--dumpdir`, `-dumpbase` and `--dumpbase` (separate only), `-aux-info`
///   (separate or `=`), `-fdump-*=<path>` and `-fopt-info*=<path>` (the values
///   `stdout` and `stderr` are streams) and `-fdeps-file=`. A trailing `-o` or
///   `--output` is left to [`CcError::DanglingOutput`].
/// - Name options. `-MT` and `-MQ` (separate or joined), `-dumpbase-ext` and
///   `--dumpbase-ext` name a target or a suffix, never a file, and their
///   operand is consumed unchecked.
/// - [`CcError::WriteRedirect`]: a path or name option without an operand; any
///   other argument that starts with `-dumpdir`, `--dumpdir`, `-dumpbase`,
///   `--dumpbase` or `-aux-info` (the `=` form of `-dumpdir` is read by `cc1` as
///   debugging letters); and a write option carried by `-Wp,`, `-Xpreprocessor`,
///   `-Wa,`, `-Xassembler` or `--for-assembler`, even with a relative path,
///   because the driver joins an operand from a separate carrier argument.
///
/// Accepted unchanged, because what they write is derived from the output, the
/// directory, the dump directory or the dump base, all of which are checked: `-MD`,
/// `-MMD`, `-M`, `-MM`, `-MG`, `-MP` and their long aliases; `-save-temps`,
/// `-save-temps=cwd`, `-save-temps=obj`, `-fstack-usage`, `-fcallgraph-info`,
/// `-gsplit-dwarf`, `-fsave-optimization-record`,
/// `-fdiagnostics-format=sarif-file`; and `-fdump-*` without a value. The linker
/// never runs for a `-c` command.
///
/// # Errors
/// [`CcError::OutsideRunRoot`] or [`CcError::WriteRedirect`].
pub fn check_write_paths(
    directory: &str,
    arguments: &[String],
    run_root: &Path,
) -> Result<(), CcError> {
    const PATH_SEPARATE: [&str; 6] = [
        "-MF",
        "-dumpdir",
        "--dumpdir",
        "-dumpbase",
        "--dumpbase",
        "-aux-info",
    ];
    const NAME_SEPARATE: [&str; 4] = ["-MT", "-MQ", "-dumpbase-ext", "--dumpbase-ext"];
    const BOUNDED_PREFIXES: [&str; 5] = [
        "-dumpdir",
        "--dumpdir",
        "-dumpbase",
        "--dumpbase",
        "-aux-info",
    ];
    let inside = |operand: &str| -> Result<(), CcError> {
        let resolved = normalise_lexical(&Path::new(directory).join(operand));
        if lexically_under(run_root, &resolved) {
            Ok(())
        } else {
            Err(CcError::OutsideRunRoot {
                path: resolved.to_string_lossy().into_owned(),
            })
        }
    };
    let redirect = |flag: &str| CcError::WriteRedirect {
        flag: flag.to_owned(),
    };
    let mut index = 1;
    while index < arguments.len() {
        let argument = arguments[index].as_str();
        let operand = arguments.get(index + 1).map(String::as_str);
        index += 1;
        if argument == "-o" || argument == "--output" {
            if let Some(path) = operand {
                inside(path)?;
                index += 1;
            }
            continue;
        }
        if PATH_SEPARATE.contains(&argument) {
            inside(operand.ok_or_else(|| redirect(argument))?)?;
            index += 1;
            continue;
        }
        if NAME_SEPARATE.contains(&argument) {
            operand.ok_or_else(|| redirect(argument))?;
            index += 1;
            continue;
        }
        if argument == "-Xpreprocessor"
            || argument == "-Xassembler"
            || argument == "--for-assembler"
        {
            let piece = operand.ok_or_else(|| redirect(argument))?;
            let writes = if argument == "-Xpreprocessor" {
                writes_through_preprocessor(piece)
            } else {
                piece.split(',').any(writes_through_assembler)
            };
            if writes {
                return Err(redirect(piece));
            }
            index += 1;
            continue;
        }
        if let Some(path) = argument.strip_prefix("--output=") {
            inside(path)?;
        } else if let Some(rest) = argument.strip_prefix("-Wp,") {
            if rest.split(',').any(writes_through_preprocessor) {
                return Err(redirect(argument));
            }
        } else if let Some(rest) = argument
            .strip_prefix("-Wa,")
            .or_else(|| argument.strip_prefix("--for-assembler="))
        {
            if rest.split(',').any(writes_through_assembler) {
                return Err(redirect(argument));
            }
        } else if let Some(path) = argument
            .strip_prefix("-aux-info=")
            .or_else(|| argument.strip_prefix("-fdeps-file="))
        {
            inside(path)?;
        } else if argument.starts_with("-fdump-") || argument.starts_with("-fopt-info") {
            if let Some((_, value)) = argument.split_once('=')
                && value != "stdout"
                && value != "stderr"
            {
                inside(value)?;
            }
        } else if BOUNDED_PREFIXES
            .iter()
            .any(|prefix| argument.starts_with(prefix))
        {
            return Err(redirect(argument));
        } else if argument.starts_with("-MT") || argument.starts_with("-MQ") {
            // A joined target name.
        } else if let Some(path) = argument
            .strip_prefix("-MF")
            .or_else(|| argument.strip_prefix("-o"))
        {
            inside(path)?;
        }
    }
    Ok(())
}

/// The output a canonical entry's arguments write, resolved against its
/// directory. The entry's own `output` field is never consulted: CMake writes
/// it relative to the top-level build directory even for sub-directory targets,
/// and Bear omits it when the command has no `-o` (RESEARCH Pitfall 4, 8).
///
/// # Errors
/// [`CcError::DanglingOutput`] for a trailing `-o`; [`CcError::BadFile`] when
/// there is no `-o` and the file name has no stem.
pub fn canonical_output(entry: &CcEntry) -> Result<String, CcError> {
    derive_output(&entry.directory, &entry.file, &entry.arguments)
}

impl CcPolicy {
    /// Refuse an entry whose compiler is not allow-listed, that carries a
    /// driver option that runs or loads another program ([`find_helper_option`]),
    /// that writes a file outside the run root or one the gate cannot bound
    /// ([`check_write_paths`]), or whose directory or output lies outside the
    /// run root. Called before anything is deleted, copied or spawned.
    ///
    /// # Errors
    /// [`CcError::CompilerNotAllowed`], [`CcError::HelperOption`],
    /// [`CcError::WriteRedirect`] or [`CcError::OutsideRunRoot`].
    pub fn check_entry(&self, entry: &CcEntry) -> Result<(), CcError> {
        let argv0 = entry.arguments.first().map_or("", String::as_str);
        if !self
            .allowed_compilers
            .iter()
            .any(|allowed| allowed == argv0)
        {
            return Err(CcError::CompilerNotAllowed {
                argv0: argv0.to_owned(),
            });
        }
        if let Some(flag) = find_helper_option(&entry.arguments) {
            return Err(CcError::HelperOption { flag: flag.clone() });
        }
        check_write_paths(&entry.directory, &entry.arguments, &self.run_root)?;
        for path in [&entry.directory, &entry.output] {
            if !lexically_under(&self.run_root, Path::new(path)) {
                return Err(CcError::OutsideRunRoot { path: path.clone() });
            }
        }
        Ok(())
    }
}

/// Pretty JSON plus a trailing newline.
///
/// # Errors
/// [`CcError::Json`] if serialisation fails.
pub fn to_json(entries: &[CcEntry]) -> Result<String, CcError> {
    let mut text = serde_json::to_string_pretty(entries)?;
    text.push('\n');
    Ok(text)
}

/// One entry of an exported compile-commands file, as Bear or CMake wrote it.
///
/// The reader is tolerant on purpose: fields this crate does not know are
/// ignored, because the two exporters add and drop fields between versions.
/// Only the fields below ever influence what is executed, and `output` is
/// carried but never trusted.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RawEntry {
    /// Working directory of the compile.
    pub directory: String,
    /// The source file.
    pub file: String,
    /// Bear writes an argv array.
    pub arguments: Option<Vec<String>>,
    /// CMake writes a shell-quoted command string.
    pub command: Option<String>,
    /// The exported output, relative to an unspecified directory. Not trusted.
    pub output: Option<String>,
}

/// The result of canonicalising an export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Canonical {
    /// The entries whose object exists, sorted by (file, output).
    pub entries: Vec<CcEntry>,
    /// How many entries the export listed.
    pub entries_exported: usize,
    /// How many of them built an object.
    pub entries_built: usize,
    /// The `file` of each dropped entry, in export order.
    pub dropped_unbuilt: Vec<String>,
}

/// Read an exported compile-commands file (a JSON array).
///
/// # Errors
/// [`CcError::Json`] if the bytes are not a JSON array of entries.
pub fn parse_raw(bytes: &[u8]) -> Result<Vec<RawEntry>, CcError> {
    Ok(serde_json::from_slice(bytes)?)
}

/// The argv of one raw entry: Bear's `arguments` as they are, or CMake's
/// `command` split into words without a shell.
fn argv_of(raw: &RawEntry) -> Result<Vec<String>, CcError> {
    match (&raw.arguments, &raw.command) {
        (Some(arguments), _) => Ok(arguments.clone()),
        (None, Some(command)) => shlex::split(command).ok_or(CcError::Unsplittable),
        (None, None) => Err(CcError::NoCommand),
    }
}

/// Canonicalise one entry and apply every gate that does not need the
/// filesystem, in a fixed order.
fn canonical_entry(raw: &RawEntry, policy: &CcPolicy) -> Result<CcEntry, CcError> {
    let arguments = argv_of(raw)?;
    if !arguments.iter().any(|argument| argument == "-c") {
        return Err(CcError::NotCompileOnly);
    }
    let argv0 = arguments.first().map_or("", String::as_str);
    if !policy
        .allowed_compilers
        .iter()
        .any(|allowed| allowed == argv0)
    {
        return Err(CcError::CompilerNotAllowed {
            argv0: argv0.to_owned(),
        });
    }
    if let Some(flag) = find_helper_option(&arguments) {
        return Err(CcError::HelperOption { flag: flag.clone() });
    }
    check_write_paths(&raw.directory, &arguments, &policy.run_root)?;
    if let Some(flag) = arguments
        .iter()
        .find(|argument| argument.starts_with("-flto") || argument.starts_with("-fprofile-"))
    {
        return Err(CcError::ForbiddenFlag { flag: flag.clone() });
    }
    // The exported `output` is never consulted (RESEARCH Pitfall 4).
    let output = derive_output(&raw.directory, &raw.file, &arguments)?;
    let entry = CcEntry {
        directory: raw.directory.clone(),
        file: raw.file.clone(),
        arguments,
        output,
    };
    for path in [&entry.directory, &entry.output] {
        if !lexically_under(&policy.run_root, Path::new(path)) {
            return Err(CcError::OutsideRunRoot { path: path.clone() });
        }
    }
    Ok(entry)
}

/// Turn an export into the canonical, gated entries (BUILD-01).
///
/// Every entry is converted to an `arguments` array (CMake `command` strings are
/// split with `shlex`, never a shell) and gated: it must be a compile-only
/// command (`-c`), its `argv[0]` must be allow-listed, no argument may make the
/// driver run or load another program ([`HELPER_OPTION_PREFIXES`]), every file it
/// writes must lie under the run root or be refused ([`check_write_paths`]), it
/// may not use `-flto` or `-fprofile-*` (the object would not be reproducible),
/// and its directory and output must lie under the run root. The output is
/// derived from the arguments (`--output` like `-o`), never read from the export. Entries whose object does not exist
/// are then dropped (CMake exports every target, built or not) and counted.
/// `output_exists` decides existence; tests pass a set, the pipeline passes the
/// filesystem.
///
/// # Errors
/// The [`CcError`] of the first entry that fails a gate, [`CcError::Empty`]
/// when no entry produced an object (a silent Bear failure looks like this),
/// and [`CcError::DuplicateOutput`] when two kept entries write one object.
pub fn canonicalise(
    raw: &[RawEntry],
    policy: &CcPolicy,
    output_exists: &dyn Fn(&Path) -> bool,
) -> Result<Canonical, CcError> {
    let mut entries: Vec<CcEntry> = Vec::new();
    let mut dropped_unbuilt: Vec<String> = Vec::new();
    for entry in raw {
        let canonical = canonical_entry(entry, policy)?;
        if output_exists(Path::new(&canonical.output)) {
            entries.push(canonical);
        } else {
            dropped_unbuilt.push(canonical.file);
        }
    }
    if entries.is_empty() {
        return Err(CcError::Empty);
    }
    entries.sort_by(|a, b| {
        (a.file.as_bytes(), a.output.as_bytes()).cmp(&(b.file.as_bytes(), b.output.as_bytes()))
    });
    // Sorted by file first, so equal outputs are not necessarily adjacent.
    let mut outputs: BTreeSet<&str> = BTreeSet::new();
    for entry in &entries {
        if !outputs.insert(entry.output.as_str()) {
            return Err(CcError::DuplicateOutput {
                output: entry.output.clone(),
            });
        }
    }
    Ok(Canonical {
        entries_exported: raw.len(),
        entries_built: entries.len(),
        entries,
        dropped_unbuilt,
    })
}
