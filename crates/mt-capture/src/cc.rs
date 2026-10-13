//! The canonical `compile_commands.json` form (BUILD-01, CONTEXT D-07).
//!
//! Every argv this crate ever executes comes from a compile-commands file, so
//! the form is strict: `arguments` arrays only, an absolute output derived
//! from the arguments, a compiler allow-list and a run-root containment check.
//! Nothing here runs a shell or spawns a process.

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

/// The object a command writes: the last `-o <path>` or `-o<path>` argument,
/// else `<source stem>.o`, resolved against `directory`.
pub(crate) fn derive_output(
    directory: &str,
    file: &str,
    arguments: &[String],
) -> Result<String, CcError> {
    let mut output: Option<String> = None;
    let mut index = 1;
    while index < arguments.len() {
        let argument = &arguments[index];
        if argument == "-o" {
            let value = arguments.get(index + 1).ok_or(CcError::DanglingOutput)?;
            output = Some(value.clone());
            index += 2;
            continue;
        }
        if let Some(joined) = argument.strip_prefix("-o") {
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
    /// Refuse an entry whose compiler is not allow-listed or whose directory or
    /// output lies outside the run root. Called before anything is spawned.
    ///
    /// # Errors
    /// [`CcError::CompilerNotAllowed`] or [`CcError::OutsideRunRoot`].
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

/// Turn an export into the canonical, gated entries.
///
/// Not implemented yet.
///
/// # Errors
/// [`CcError`].
pub fn canonicalise(
    _raw: &[RawEntry],
    _policy: &CcPolicy,
    _output_exists: &dyn Fn(&Path) -> bool,
) -> Result<Canonical, CcError> {
    Err(CcError::Empty)
}
