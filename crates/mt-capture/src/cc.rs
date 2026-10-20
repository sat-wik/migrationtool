//! The canonical `compile_commands.json` form (BUILD-01, CONTEXT D-07).
//!
//! Every argv this crate ever executes comes from a compile-commands file, so
//! the form is strict: `arguments` arrays only, an absolute output derived
//! from the arguments, a compiler allow-list and a run-root containment check.
//! Nothing here runs a shell or spawns a process.
//!
//! # The gate on a recorded command (plans 02-20 and 02-21; T-02-67, T-02-70, T-02-72, T-02-73, T-02-74)
//!
//! The gate is an allow-list, not a list of known-bad options (gap G-02-8,
//! founder decisions 1-3 of 2026-10-20). One operand-aware walk,
//! [`walk_arguments`] over [`ALLOWED_OPTIONS`], is the only code that reads the
//! options of a recorded command. It knows which options take a separate
//! operand, so an operand is never read as an option, and it serves the
//! compile-only test, the derived output ([`canonical_output`]) and the
//! write-path check ([`check_write_paths`]). [`CcPolicy::check_command`] runs
//! the whole gate; [`canonicalise`] calls it at capture (the record stage) and
//! [`CcPolicy::check_entry`] calls it again at replay, before anything is
//! deleted, copied or spawned. Nothing outside the list reaches the driver, the
//! assembler or the linker.
//!
//! - The response-file rule comes first: any argument after `argv[0]` that
//!   starts with `@`, operands included, is refused, because the driver
//!   expands response files before it parses options (T-02-72).
//! - Only the spellings of [`ALLOWED_OPTIONS`] pass: those measured from the
//!   30 retained pairs' recorded commands, the depfile flags `-MD`, `-MMD`,
//!   `-MP`, `-MF`, `-MT` and `-MQ` with every `-o` and `-MF` path under the run
//!   root, and `-specs=<name>` for the names of [`SHIPPED_SPECS`] only.
//! - `-c` counts only as a top-level option of the walk (CR-03, T-02-73).
//!
//! Causes (refusal, [`CcError`] variant, cause token of the capture report):
//!
//! | Refusal | Variant | Cause |
//! |---|---|---|
//! | empty argv | `NotCompileOnly` | `not_compile_only` |
//! | `argv[0]` not allow-listed | `CompilerNotAllowed` | `compiler_not_allowed` |
//! | an argument after `argv[0]` that starts with `@` | `HelperOption` | `helper_option` |
//! | `-specs=<name>` with a name not in [`SHIPPED_SPECS`], empty or with `/`; any other argument that starts with `-specs` | `HelperOption` | `helper_option` |
//! | `-flto*`, `-fprofile-*` | `ForbiddenFlag` | `forbidden_flag` |
//! | any other argument that starts with `-` and is not on the list (every `--<name>`, every `-Wa,`, `-Xassembler`, `--for-assembler`, `-Wl,`, `-Xlinker`, `--for-linker`, `-Wp,` and `-Xpreprocessor` carrier, `-wrapper`, `-B`, `-fplugin*`, `-include`, `-x`, `-dumpdir`, ...) | `OptionNotAllowed` | `option_not_allowed` |
//! | trailing `-o` | `DanglingOutput` | `dangling_output` |
//! | trailing `-MF`, `-MT` or `-MQ` | `WriteRedirect` | `write_redirect` |
//! | trailing `-I` | `OptionNotAllowed` | `option_not_allowed` |
//! | an `-o` or `-MF` path, the directory or the derived output outside the run root (lexically) | `OutsideRunRoot` | `outside_run_root` |
//! | no top-level `-c` after a complete walk | `NotCompileOnly` | `not_compile_only` |
//!
//! Measured basis (plan 02-21, re-run at execution on 2026-10-20). The driver
//! splices a response file's words in place of the `@` argument before it
//! parses options, so in operand position the first word becomes the operand
//! and the rest are options: with `x.rsp` holding `-I. -wrapper <script>`,
//! each of `-I @x.rsp`, `-MT @x.rsp`, `-MF @x.rsp`, `-o @x.rsp` and `@x.rsp`
//! ran every subcommand under the script, with the host `arm-none-eabi-gcc`
//! 13.2.1 and `-###` (which runs nothing) and with a real compile by the host
//! `/usr/bin/cc` 13.3.0. `-MD -MT -c t.c -o t.o` runs `collect2` under `-###`.
//! With the host `/usr/bin/cc` 13.3.0 and GNU as 2.42, real compiles wrote
//! outside the working directory for `--dump-final-insns=`, `--opt-info-all=`
//! (CR-01), `-Wa,-MD,`, `-Wa,-MD=`, `-Wa,--M,`, `-Xassembler -MD -Xassembler
//! <path>` (CR-02) and the CR-03 vector with `-Wl,-Map=`. The pinned
//! toolchain is Arm GNU Toolchain 14.3.Rel1; its shipped spec names are
//! measured in CI (WR-13).

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
    /// An argument makes the driver read options or commands from a file: a
    /// response file (`@file`, operands included) or a spec file the pinned
    /// toolchain does not ship.
    #[error(
        "argument {flag} makes the GCC driver read options or commands from a file (a response file, or a spec file the pinned toolchain does not ship)"
    )]
    HelperOption {
        /// The offending argument.
        flag: String,
    },
    /// A depfile option (`-MF`, `-MT`, `-MQ`) has no operand, so the gate
    /// cannot bound what it writes.
    #[error(
        "argument {flag} has no operand, so the gate cannot bound the dependency file it writes"
    )]
    WriteRedirect {
        /// The offending argument.
        flag: String,
    },
    /// An argument is not on [`ALLOWED_OPTIONS`].
    #[error(
        "argument {flag} is not on the compile-command allow-list (measured from the retained pairs' recorded commands)"
    )]
    OptionNotAllowed {
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

/// What the operand of an allow-listed option names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperandKind {
    /// The object the command writes (`-o`); it must lie under the run root.
    OutputPath,
    /// The dependency file the command writes (`-MF`); it must lie under the
    /// run root.
    DepfilePath,
    /// A make target name (`-MT`, `-MQ`), never a file.
    TargetName,
    /// A header search directory (`-I`), only read.
    IncludeDirectory,
    /// A macro definition (`-D`).
    Macro,
    /// A spec file name (`-specs=`); only the names of [`SHIPPED_SPECS`].
    SpecName,
}

/// Whether an allow-listed option takes an operand, and how.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operand {
    /// The argument is the exact spelling, with nothing joined.
    None,
    /// The operand is joined (`-Iinc`) or, when the argument equals the
    /// spelling, the next argument (`-I inc`).
    JoinedOrSeparate(OperandKind),
    /// The operand is joined only and must not be empty (`-DNAME`,
    /// `-specs=nano.specs`).
    JoinedOnly(OperandKind),
}

/// One spelling of the compile-command allow-list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllowedOption {
    /// The exact spelling, or the prefix an operand is joined to.
    pub spelling: &'static str,
    /// How the operand is given, if there is one.
    pub operand: Operand,
}

const fn exact(spelling: &'static str) -> AllowedOption {
    AllowedOption {
        spelling,
        operand: Operand::None,
    }
}

const fn joined_or_separate(spelling: &'static str, kind: OperandKind) -> AllowedOption {
    AllowedOption {
        spelling,
        operand: Operand::JoinedOrSeparate(kind),
    }
}

const fn joined_only(spelling: &'static str, kind: OperandKind) -> AllowedOption {
    AllowedOption {
        spelling,
        operand: Operand::JoinedOnly(kind),
    }
}

/// The compile-command allow-list (gap G-02-8, founder decisions 2 and 3).
///
/// How it was measured. On 2026-10-20 (plan 02-21 planning at HEAD 295c4a5,
/// re-run at execution at HEAD 00e3415) every option spelling was listed from
/// the recorded compile commands of all 30 retained pairs plus the dropped
/// paho/server: the 29 local canonical files
/// `.mt-work/capture/<id>/<cfg>/compile_commands.json` (481 entries), their 29
/// raw exports `compile_commands.raw.json` (531 entries), the 2 tinycbor run
/// exports `.mt-work/run/tinycbor/{default,no-float}/build/compile_commands.json`
/// (20 entries; tinycbor's local build failed on the host newlib, so it has no
/// capture export) and the 2 CI fixtures
/// `tests/fixtures/ci/{qcbor,corejson}-default.compile_commands.json` (6
/// entries), 1038 entries in all, with
/// `jq -r '.[].arguments[1:][]' <files> | grep '^-' | sed -E 's/^(-D|-I).+$/\1<joined>/' | sort | uniq -c`
/// (the CMake `command` strings of the raw files and run exports are covered by
/// `build_01_local_captures_pass_the_compile_command_gate`). The result is 45
/// of the exact spellings below (all but the CI-only addition named after the
/// founder additions), `-D<macro>` (joined only), `-I<dir>` and
/// `-I <dir>`, and `-o <path>`. No `--` spelling, `@` argument, carrier,
/// `-specs`, depfile flag, `-x`, `-include` or `-isystem` occurs.
///
/// Founder additions: `-MD`, `-MMD` and `-MP` (exact), `-MF`, `-MT` and `-MQ`
/// (joined or separate; every `-MF` path under the run root), and `-specs=`
/// (joined, names of [`SHIPPED_SPECS`] only). Joined `-o<path>` is the same
/// GCC option as `-o <path>`.
///
/// CI-only addition (plan 02-21 round trip 1): `-Wmissing-variable-declarations`.
/// Testbed run 39453293771 (job 118733983780, head 4818d97) refused
/// cjson/default and cjson/no-locale with `option_not_allowed` on it, at the
/// first refused entry of their CMake export. cjson's `CMakeLists.txt` lists it
/// among its custom compiler flags and adds it only when
/// `check_c_compiler_flag` accepts it: the pinned GCC 14 does, the host GCC
/// 13.2.1 of the local measurement does not (checked offline with GCC 14.2.1;
/// it is the only flag of that list whose support differs).
///
/// A spelling is added only test first (`build_01_allow_list_is_the_measured_set`),
/// with the evidence of the recorded command that needs it.
pub const ALLOWED_OPTIONS: [AllowedOption; 56] = [
    exact("-c"),
    exact("-g"),
    exact("-O3"),
    exact("-Os"),
    exact("-fPIC"),
    exact("-fstack-protector-strong"),
    exact("-fvisibility=hidden"),
    exact("-mcpu=cortex-m4"),
    exact("-mfloat-abi=hard"),
    exact("-mfpu=fpv4-sp-d16"),
    exact("-mthumb"),
    exact("-pedantic"),
    exact("-std=c89"),
    exact("-std=c99"),
    exact("-Wall"),
    exact("-Wc++-compat"),
    exact("-Wcast-align"),
    exact("-Wcast-qual"),
    exact("-Wconversion"),
    exact("-Wdouble-promotion"),
    exact("-Werror"),
    exact("-Werror=format-security"),
    exact("-Werror=implicit-function-declaration"),
    exact("-Werror=incompatible-pointer-types"),
    exact("-Werror=int-conversion"),
    exact("-Wextra"),
    exact("-Wformat-overflow"),
    exact("-Wformat=2"),
    exact("-Winit-self"),
    exact("-Wmissing-prototypes"),
    exact("-Wmissing-variable-declarations"),
    exact("-Wnested-externs"),
    exact("-Wno-format-nonliteral"),
    exact("-Wparentheses"),
    exact("-Wpointer-arith"),
    exact("-Wredundant-decls"),
    exact("-Wshadow"),
    exact("-Wsign-compare"),
    exact("-Wstrict-overflow=2"),
    exact("-Wstrict-prototypes"),
    exact("-Wsuggest-attribute=format"),
    exact("-Wswitch-default"),
    exact("-Wswitch-enum"),
    exact("-Wundef"),
    exact("-Wunused-macros"),
    exact("-Wwrite-strings"),
    exact("-MD"),
    exact("-MMD"),
    exact("-MP"),
    joined_or_separate("-o", OperandKind::OutputPath),
    joined_or_separate("-I", OperandKind::IncludeDirectory),
    joined_only("-D", OperandKind::Macro),
    joined_or_separate("-MF", OperandKind::DepfilePath),
    joined_or_separate("-MT", OperandKind::TargetName),
    joined_or_separate("-MQ", OperandKind::TargetName),
    joined_only("-specs=", OperandKind::SpecName),
];

/// The spec file names `-specs=<name>` may take (review WR-13).
///
/// Empty until plan 02-21 Task 3 adds the names measured from the pinned image:
/// those the pinned Arm GNU Toolchain 14.3.Rel1 ships and its driver reads from
/// inside the toolchain even when a same-named file is planted in the working
/// directory. Until then every `-specs=` is refused, the fail-safe direction.
pub const SHIPPED_SPECS: [&str; 0] = [];

/// What one walk of a recorded command found ([`walk_arguments`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommandWalk {
    /// A top-level `-c` was seen (never an operand that reads `-c`).
    pub compile_only: bool,
    /// Every `-o` operand, as written, in order.
    pub outputs: Vec<String>,
    /// Every `-MF` operand, as written, in order.
    pub dep_files: Vec<String>,
}

/// The one walk, plus every write operand (`-o` and `-MF`) in order of
/// appearance for [`check_write_paths`].
fn walk(arguments: &[String]) -> Result<(CommandWalk, Vec<String>), CcError> {
    let rest = arguments.get(1..).unwrap_or_default();
    // The driver expands every `@` argument before it parses options, so the
    // rule covers operands too (T-02-72).
    if let Some(flag) = rest.iter().find(|argument| argument.starts_with('@')) {
        return Err(CcError::HelperOption { flag: flag.clone() });
    }
    let mut found = CommandWalk::default();
    let mut writes: Vec<String> = Vec::new();
    let mut index = 0;
    while index < rest.len() {
        let argument = rest[index].as_str();
        index += 1;
        if !argument.starts_with('-') {
            // An input.
            continue;
        }
        if ALLOWED_OPTIONS
            .iter()
            .any(|option| option.operand == Operand::None && option.spelling == argument)
        {
            if argument == "-c" {
                found.compile_only = true;
            }
            continue;
        }
        if let Some((spelling, kind, operand)) = joined_or_separate_operand(argument) {
            let operand = match operand {
                Some(joined) => joined.to_owned(),
                None => {
                    let separate = rest.get(index).ok_or_else(|| missing(spelling, kind))?;
                    index += 1;
                    separate.clone()
                }
            };
            match kind {
                OperandKind::OutputPath => found.outputs.push(operand.clone()),
                OperandKind::DepfilePath => found.dep_files.push(operand.clone()),
                _ => continue,
            }
            writes.push(operand);
            continue;
        }
        if argument.starts_with("-specs") {
            let shipped = argument.strip_prefix("-specs=").is_some_and(|name| {
                !name.is_empty() && !name.contains('/') && SHIPPED_SPECS.contains(&name)
            });
            if shipped {
                continue;
            }
            return Err(CcError::HelperOption {
                flag: argument.to_owned(),
            });
        }
        if ALLOWED_OPTIONS.iter().any(|option| {
            matches!(option.operand, Operand::JoinedOnly(kind) if kind != OperandKind::SpecName)
                && argument.len() > option.spelling.len()
                && argument.starts_with(option.spelling)
        }) {
            continue;
        }
        if argument.starts_with("-flto") || argument.starts_with("-fprofile-") {
            return Err(CcError::ForbiddenFlag {
                flag: argument.to_owned(),
            });
        }
        // The walk never continues past an argument it cannot classify, so it
        // never guesses what follows an unknown option.
        return Err(CcError::OptionNotAllowed {
            flag: argument.to_owned(),
        });
    }
    Ok((found, writes))
}

/// The joined-or-separate option `argument` spells: its spelling, its operand
/// kind, and the joined operand (`None` when the operand is the next argument).
fn joined_or_separate_operand(argument: &str) -> Option<(&'static str, OperandKind, Option<&str>)> {
    ALLOWED_OPTIONS
        .iter()
        .find_map(|option| match option.operand {
            Operand::JoinedOrSeparate(kind) => {
                let joined = argument.strip_prefix(option.spelling)?;
                Some((
                    option.spelling,
                    kind,
                    (!joined.is_empty()).then_some(joined),
                ))
            }
            _ => None,
        })
}

/// The refusal for a joined-or-separate option that ends the command.
fn missing(spelling: &str, kind: OperandKind) -> CcError {
    match kind {
        OperandKind::OutputPath => CcError::DanglingOutput,
        OperandKind::DepfilePath | OperandKind::TargetName => CcError::WriteRedirect {
            flag: spelling.to_owned(),
        },
        OperandKind::IncludeDirectory | OperandKind::Macro | OperandKind::SpecName => {
            CcError::OptionNotAllowed {
                flag: spelling.to_owned(),
            }
        }
    }
}

/// Walk `arguments[1..]` once, left to right, over [`ALLOWED_OPTIONS`] (gap
/// G-02-8, founder decision 1). This is the only code that reads the options of
/// a recorded command: the compile-only test, [`canonical_output`] and
/// [`check_write_paths`] all use it, at capture and at replay.
///
/// First, any argument that starts with `@` is refused, operands included.
/// Then each argument is an exact spelling (`-c` sets `compile_only`), a
/// joined-or-separate option that takes the rest of the argument or else the
/// next argument as its operand (so an operand is never read as an option), a
/// joined-only option with a non-empty value, a `-specs=` name of
/// [`SHIPPED_SPECS`], or an input (it does not start with `-`). Anything else
/// ends the walk with its refusal; see the cause table in the module doc.
///
/// # Errors
/// [`CcError::HelperOption`], [`CcError::ForbiddenFlag`],
/// [`CcError::OptionNotAllowed`], [`CcError::DanglingOutput`] or
/// [`CcError::WriteRedirect`].
pub fn walk_arguments(arguments: &[String]) -> Result<CommandWalk, CcError> {
    walk(arguments).map(|(found, _)| found)
}

/// The object a command writes: the last `-o` operand of the walk (the last
/// output wins, as it does in GCC), else `<source stem>.o`, resolved against
/// `directory`.
pub(crate) fn derive_output(
    directory: &str,
    file: &str,
    arguments: &[String],
) -> Result<String, CcError> {
    let found = walk_arguments(arguments)?;
    output_of(directory, file, &found)
}

fn output_of(directory: &str, file: &str, found: &CommandWalk) -> Result<String, CcError> {
    let output = match found.outputs.last() {
        Some(path) => path.clone(),
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

/// Require every path in `writes`, resolved lexically against `directory`, to
/// lie under `run_root`.
fn writes_inside(directory: &str, writes: &[String], run_root: &Path) -> Result<(), CcError> {
    for operand in writes {
        let resolved = normalise_lexical(&Path::new(directory).join(operand));
        if !lexically_under(run_root, &resolved) {
            return Err(CcError::OutsideRunRoot {
                path: resolved.to_string_lossy().into_owned(),
            });
        }
    }
    Ok(())
}

/// Refuse a command that writes a file outside `run_root` (threat T-02-70).
/// `directory` is the entry's working directory; relative operands are resolved
/// against it, lexically.
///
/// The arguments are walked once ([`walk_arguments`]), so only a top-level `-o`
/// or `-MF` names a written file, and every one of them, in order of
/// appearance, must resolve under the run root. Nothing else on the allow-list
/// names a file the compiler writes: `-MD` and `-MMD` derive their depfile from
/// the output, and `-MT` and `-MQ` name a target.
///
/// # Errors
/// [`CcError::OutsideRunRoot`] with the resolved path, or the walk's refusal.
pub fn check_write_paths(
    directory: &str,
    arguments: &[String],
    run_root: &Path,
) -> Result<(), CcError> {
    let (_, writes) = walk(arguments)?;
    writes_inside(directory, &writes, run_root)
}

/// The output a canonical entry's arguments write, resolved against its
/// directory. The entry's own `output` field is never consulted: CMake writes
/// it relative to the top-level build directory even for sub-directory targets,
/// and Bear omits it when the command has no `-o` (RESEARCH Pitfall 4, 8).
///
/// # Errors
/// The walk's refusal ([`walk_arguments`]), including
/// [`CcError::DanglingOutput`] for a trailing `-o`; [`CcError::BadFile`] when
/// there is no `-o` and the file name has no stem.
pub fn canonical_output(entry: &CcEntry) -> Result<String, CcError> {
    derive_output(&entry.directory, &entry.file, &entry.arguments)
}

impl CcPolicy {
    /// The whole gate on one recorded command, and the output it writes. Called
    /// at capture ([`canonicalise`]) and at replay ([`CcPolicy::check_entry`]),
    /// before anything is deleted, copied or spawned. In order:
    ///
    /// 1. an empty argv is [`CcError::NotCompileOnly`];
    /// 2. `argv[0]` must be allow-listed;
    /// 3. [`walk_arguments`] must accept every argument;
    /// 4. every `-o` and `-MF` path must lie under the run root;
    /// 5. the walk must have seen a top-level `-c`;
    /// 6. the directory must lie under the run root;
    /// 7. the derived output must lie under the run root.
    ///
    /// # Errors
    /// [`CcError::NotCompileOnly`], [`CcError::CompilerNotAllowed`], the walk's
    /// refusal, [`CcError::OutsideRunRoot`] or [`CcError::BadFile`].
    pub fn check_command(
        &self,
        directory: &str,
        file: &str,
        arguments: &[String],
    ) -> Result<String, CcError> {
        let Some(argv0) = arguments.first() else {
            return Err(CcError::NotCompileOnly);
        };
        if !self
            .allowed_compilers
            .iter()
            .any(|allowed| allowed == argv0)
        {
            return Err(CcError::CompilerNotAllowed {
                argv0: argv0.clone(),
            });
        }
        let (found, writes) = walk(arguments)?;
        writes_inside(directory, &writes, &self.run_root)?;
        if !found.compile_only {
            return Err(CcError::NotCompileOnly);
        }
        if !lexically_under(&self.run_root, Path::new(directory)) {
            return Err(CcError::OutsideRunRoot {
                path: directory.to_owned(),
            });
        }
        let output = output_of(directory, file, &found)?;
        if !lexically_under(&self.run_root, Path::new(&output)) {
            return Err(CcError::OutsideRunRoot { path: output });
        }
        Ok(output)
    }

    /// Replay's gate on a canonical entry: [`CcPolicy::check_command`], the
    /// same gate as at capture (compile-only test included, review IN-13), and
    /// the entry's recorded `output` under the run root.
    ///
    /// # Errors
    /// Every error of [`CcPolicy::check_command`], or
    /// [`CcError::OutsideRunRoot`] for the recorded output.
    pub fn check_entry(&self, entry: &CcEntry) -> Result<(), CcError> {
        self.check_command(&entry.directory, &entry.file, &entry.arguments)?;
        if !lexically_under(&self.run_root, Path::new(&entry.output)) {
            return Err(CcError::OutsideRunRoot {
                path: entry.output.clone(),
            });
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

/// Canonicalise one entry and apply the whole gate ([`CcPolicy::check_command`]),
/// which needs no filesystem.
fn canonical_entry(raw: &RawEntry, policy: &CcPolicy) -> Result<CcEntry, CcError> {
    let arguments = argv_of(raw)?;
    // The exported `output` is never consulted (RESEARCH Pitfall 4).
    let output = policy.check_command(&raw.directory, &raw.file, &arguments)?;
    Ok(CcEntry {
        directory: raw.directory.clone(),
        file: raw.file.clone(),
        arguments,
        output,
    })
}

/// Turn an export into the canonical, gated entries (BUILD-01).
///
/// Every entry is converted to an `arguments` array (CMake `command` strings are
/// split with `shlex`, never a shell) and gated by [`CcPolicy::check_command`]:
/// its `argv[0]` must be allow-listed, every argument must be a spelling of
/// [`ALLOWED_OPTIONS`] (no `@` argument at all), it must be a compile-only
/// command (a top-level `-c` of [`walk_arguments`]), and every `-o` and `-MF`
/// path, its directory and its output must lie under the run root. The output
/// is derived from the arguments, never read from the export. Entries whose
/// object does not exist are then dropped (CMake exports every target, built
/// or not) and counted.
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
