//! The entry-function check (BED-01, CONTEXT D-23, D-24).
//!
//! Each manifest entry names a function by hand. After a capture, every one is
//! confirmed against the build that was actually captured, with clang run
//! through the `mt-toolchain` runner on a probe file:
//!
//! 1. The probe file holds one line, `#include "<absolute header path>"`, so the
//!    AST dump covers one header and not a whole translation unit (a dump of
//!    `http_parser.c` is 32 MB and would overrun the runner's stream cap;
//!    RESEARCH Pitfall 12).
//! 2. clang runs with the defining translation unit's flags, reduced to the ones
//!    clang accepts ([`filter_flags`]: it rejects GCC-only `-f` and `-W` options).
//! 3. `-ast-dump-filter=<name>` matches substrings, so the function is selected
//!    by exact name from the concatenated JSON documents on stdout
//!    ([`parse_ast_stream`]).
//! 4. The declared type must equal the manifest signature after whitespace
//!    collapse ([`normalise_signature`]).
//! 5. `clang -MM` on the defining translation unit must list the header, so the
//!    header really is one the captured build includes ([`parse_make_deps`]).
//!
//! A miss at any step is a failed entry and the caller fails the capture. A
//! timeout or an incomplete output stream is `not_proven`, never a pass.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use mt_toolchain::runner::{RunRequest, RunnerConfig, RunnerError, run};
use serde::{Deserialize, Serialize};

use crate::capture::{RunLayout, ToolPaths};
use crate::cc::CcEntry;
use crate::manifest::Entry;
use crate::report::Step;

/// The probe file name inside the capture directory.
pub const PROBE_FILE: &str = "entry_probe.c";

/// How much of clang's standard error a failed entry quotes.
const STDERR_EXCERPT: usize = 300;

/// A function declaration found in clang's JSON AST.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionDecl {
    /// The function name.
    pub name: String,
    /// `type.qualType`, exactly as clang printed it.
    pub qual_type: String,
    /// The file of the declaration. clang prints `loc.file` only when it changes
    /// from the previous location it printed, so it is carried across nodes and
    /// documents; `None` when no earlier location named a file.
    pub file: Option<String>,
}

/// The outcome of checking one manifest entry (CONTEXT D-22, D-23).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntryResult {
    /// A `FunctionDecl` with exactly this name was dumped.
    pub found: bool,
    /// Every dumped declaration has the manifest signature, whitespace
    /// collapsed.
    pub signature_matches: bool,
    /// The header is among the dependencies of the defining translation unit.
    pub header_included: bool,
    /// The `qualType` clang printed: the first one that differs from the
    /// manifest, else the first.
    pub found_signature: Option<String>,
    /// Why the entry failed, where the booleans are not enough. `not_proven:`
    /// leads the text when a timeout or an incomplete stream left it open.
    pub detail: Option<String>,
}

impl EntryResult {
    /// True when every part of the check passed.
    #[must_use]
    pub fn passed(&self) -> bool {
        self.found && self.signature_matches && self.header_included
    }
}

/// The result of one entry check with the clang runs behind it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryCheck {
    /// The outcome.
    pub result: EntryResult,
    /// Every clang run, in order, with its full run record.
    pub steps: Vec<Step>,
    /// True when a timeout or an incomplete stream left the check open.
    pub not_proven: bool,
}

/// Why an entry could not be checked at all. A function that is merely absent
/// or mistyped is an [`EntryResult`], not an error.
#[derive(Debug, thiserror::Error)]
pub enum EntryError {
    /// The inputs cannot be used safely.
    #[error("invalid entry input: {0}")]
    InvalidInput(String),
    /// clang's output is not a stream of JSON documents.
    #[error("clang's AST output is not a stream of JSON documents: {0}")]
    Ast(#[from] serde_json::Error),
    /// A runner failure: clang cannot be started or supervised.
    #[error(transparent)]
    Runner(#[from] RunnerError),
    /// The probe file could not be written.
    #[error("cannot write {}: {source}", path.display())]
    Io {
        /// The path involved.
        path: PathBuf,
        /// The underlying error.
        #[source]
        source: io::Error,
    },
}

/// The probe file text: one line that includes the declaring header.
///
/// # Errors
/// [`EntryError::InvalidInput`] when the path is not absolute or holds a quote,
/// backslash or control character that would change what the line includes.
pub fn probe_source(header_abs: &Path) -> Result<String, EntryError> {
    let text = header_abs.to_str().ok_or_else(|| {
        EntryError::InvalidInput(format!("{} is not UTF-8", header_abs.display()))
    })?;
    if !header_abs.is_absolute()
        || text
            .chars()
            .any(|c| c.is_control() || c == '"' || c == '\\')
    {
        return Err(EntryError::InvalidInput(format!(
            "{text:?} is not an absolute header path that can be written in an #include"
        )));
    }
    Ok(format!("#include \"{text}\"\n"))
}

/// Options that carry their operand as the next argument.
const SEPARATE_OPERAND: [&str; 6] = ["-D", "-U", "-I", "-isystem", "-iquote", "-include"];

/// Options kept as spelled.
const KEPT_EXACT: [&str; 5] = [
    "-mthumb",
    "-ffreestanding",
    "-fsigned-char",
    "-funsigned-char",
    "-fshort-enums",
];

/// Options kept when they begin with these (the operand is attached).
const KEPT_PREFIX: [&str; 8] = [
    "-D",
    "-U",
    "-I",
    "-std=",
    "-mcpu=",
    "-mfloat-abi=",
    "-mfpu=",
    "-march=",
];

/// Keep the options of a captured argv that matter to clang's parse and drop
/// the rest: the compiler itself, the source, `-o` and its operand, warnings,
/// optimisation and debug options, and every GCC-only `-f` option clang
/// rejects. `-D`, `-U`, `-I`, `-isystem`, `-iquote` and `-include` keep their
/// operand in either spelling.
#[must_use]
pub fn filter_flags(arguments: &[String]) -> Vec<String> {
    let mut kept = Vec::new();
    let mut index = 0;
    while index < arguments.len() {
        let argument = arguments[index].as_str();
        index += 1;
        if SEPARATE_OPERAND.contains(&argument) {
            if let Some(operand) = arguments.get(index) {
                kept.push(argument.to_owned());
                kept.push(operand.clone());
                index += 1;
            }
        } else if KEPT_EXACT.contains(&argument)
            || KEPT_PREFIX
                .iter()
                .any(|prefix| argument.starts_with(prefix))
            || ["-isystem", "-iquote", "-include"]
                .iter()
                .any(|prefix| argument.starts_with(prefix) && argument.len() > prefix.len())
        {
            kept.push(argument.to_owned());
        }
    }
    kept
}

fn base_argv(tools: &ToolPaths) -> Result<Vec<String>, EntryError> {
    let text = |path: &Path| {
        path.to_str()
            .map(str::to_owned)
            .ok_or_else(|| EntryError::InvalidInput(format!("{} is not UTF-8", path.display())))
    };
    Ok(vec![
        text(&tools.clang)?,
        "--target=arm-none-eabi".to_owned(),
        format!("--sysroot={}", text(&tools.sysroot)?),
    ])
}

/// The clang command that dumps the declarations whose name contains `name`.
///
/// # Errors
/// [`EntryError::InvalidInput`] for a name that is not a C identifier or a path
/// that is not UTF-8.
pub fn clang_ast_argv(
    tools: &ToolPaths,
    name: &str,
    flags: &[String],
    probe: &Path,
) -> Result<Vec<String>, EntryError> {
    if !is_c_identifier(name) {
        return Err(EntryError::InvalidInput(format!(
            "{name:?} is not a C identifier"
        )));
    }
    let mut argv = base_argv(tools)?;
    argv.extend(
        [
            "-fsyntax-only",
            "-Xclang",
            "-ast-dump=json",
            "-Xclang",
            &format!("-ast-dump-filter={name}"),
        ]
        .map(str::to_owned),
    );
    argv.extend(flags.iter().cloned());
    argv.push(
        probe
            .to_str()
            .ok_or_else(|| EntryError::InvalidInput(format!("{} is not UTF-8", probe.display())))?
            .to_owned(),
    );
    Ok(argv)
}

/// The clang command that lists the user-header dependencies of `source`.
///
/// # Errors
/// [`EntryError::InvalidInput`] for a path that is not UTF-8.
pub fn clang_deps_argv(
    tools: &ToolPaths,
    flags: &[String],
    source: &str,
) -> Result<Vec<String>, EntryError> {
    let mut argv = base_argv(tools)?;
    argv.push("-MM".to_owned());
    argv.extend(flags.iter().cloned());
    argv.push(source.to_owned());
    Ok(argv)
}

fn is_c_identifier(text: &str) -> bool {
    let mut bytes = text.bytes();
    bytes
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// Collapse every run of whitespace to one space and trim the ends, so
/// `int (const unsigned char *,  unsigned long)` equals the same text with one
/// space.
#[must_use]
pub fn normalise_signature(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Every `FunctionDecl` with a name in a stream of concatenated JSON documents,
/// in order of appearance.
///
/// clang prints one document per matching declaration, back to back and not as
/// an array, and prints `loc.file` only when it differs from the previous
/// location, so the file is carried from node to node and from document to
/// document. An empty stream has no declarations.
///
/// # Errors
/// [`EntryError::Ast`] when the bytes are not JSON documents.
pub fn parse_ast_stream(bytes: &[u8]) -> Result<Vec<FunctionDecl>, EntryError> {
    let mut found = Vec::new();
    let mut file: Option<String> = None;
    for document in serde_json::Deserializer::from_slice(bytes).into_iter::<serde_json::Value>() {
        walk(&document?, &mut file, &mut found);
    }
    Ok(found)
}

fn location_file(node: &serde_json::Value) -> Option<&str> {
    let loc = node.get("loc")?;
    loc.get("file")
        .or_else(|| loc.get("spellingLoc")?.get("file"))
        .and_then(serde_json::Value::as_str)
}

fn walk(node: &serde_json::Value, file: &mut Option<String>, found: &mut Vec<FunctionDecl>) {
    if let Some(name) = location_file(node) {
        *file = Some(name.to_owned());
    }
    if node.get("kind").and_then(serde_json::Value::as_str) == Some("FunctionDecl")
        && let Some(name) = node.get("name").and_then(serde_json::Value::as_str)
        && let Some(qual_type) = node
            .get("type")
            .and_then(|ty| ty.get("qualType"))
            .and_then(serde_json::Value::as_str)
    {
        found.push(FunctionDecl {
            name: name.to_owned(),
            qual_type: qual_type.to_owned(),
            file: file.clone(),
        });
    }
    if let Some(children) = node.get("inner").and_then(serde_json::Value::as_array) {
        for child in children {
            walk(child, file, found);
        }
    }
}

/// The prerequisites of a Make-format dependency rule such as
/// `x.o: x.c x.h \` followed by continuation lines. A backslash before a
/// newline joins lines and a backslash before a space is a space in a name.
#[must_use]
pub fn parse_make_deps(text: &str) -> Vec<String> {
    let joined = text.replace("\\\r\n", " ").replace("\\\n", " ");
    let rest = match joined.split_once(':') {
        Some((_, rest)) => rest,
        None => return Vec::new(),
    };
    let mut deps = Vec::new();
    let mut current = String::new();
    let mut chars = rest.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&' ') => {
                current.push(' ');
                chars.next();
            }
            c if c.is_whitespace() => {
                if !current.is_empty() {
                    deps.push(std::mem::take(&mut current));
                }
            }
            c => current.push(c),
        }
    }
    if !current.is_empty() {
        deps.push(current);
    }
    deps
}

/// Resolve `.` and `..` without touching the file system.
fn lexical(path: &Path) -> PathBuf {
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

/// A path as the compile's working directory would resolve it.
fn resolved(directory: &str, file: &str) -> PathBuf {
    lexical(&Path::new(directory).join(file))
}

/// What one clang run came to.
enum Run {
    /// Exit 0 with complete streams; the captured standard output.
    Done(Vec<u8>),
    /// A timeout or an incomplete stream.
    NotProven,
    /// A non-zero exit, with the start of its standard error.
    Failed(Option<i32>, String),
}

fn run_clang(
    cfg: &RunnerConfig,
    tools: &ToolPaths,
    env: &BTreeMap<String, String>,
    name: &str,
    argv: &[String],
    cwd: &Path,
    steps: &mut Vec<Step>,
) -> Result<Run, EntryError> {
    let request = RunRequest {
        tool_name: "clang".to_owned(),
        argv: argv.iter().map(OsString::from).collect(),
        cwd: cwd.to_path_buf(),
        extra_env: env.clone(),
        tool_version: tools.versions.get("clang").cloned(),
    };
    let output = run(cfg, &request)?;
    let exit = output.record.exit.clone();
    let complete = output.record.stdout.complete && output.record.stderr.complete;
    steps.push(Step {
        name: name.to_owned(),
        run: output.record.clone(),
    });
    Ok(if exit.timed_out || !complete {
        Run::NotProven
    } else if exit.code == Some(0) {
        Run::Done(output.stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Run::Failed(exit.code, stderr.chars().take(STDERR_EXCERPT).collect())
    })
}

/// Check one manifest entry against the captured build (D-23).
///
/// `canonical` is the canonical compile commands of the capture; the entry's
/// `source` must be one of its files. `env` is the environment the capture's
/// own steps used (`HOME` and `TMPDIR` under the work directory), and the
/// timeout is `cfg`'s. The probe is written to the capture directory.
///
/// # Errors
/// [`EntryError`] for a name or path that cannot be used, a probe that cannot be
/// written, unparseable AST output, or a clang that cannot be started. A
/// function that is absent, mistyped or not declared by an included header is
/// an [`EntryResult`] with the matching boolean false.
pub fn check_entry(
    cfg: &RunnerConfig,
    tools: &ToolPaths,
    layout: &RunLayout,
    env: &BTreeMap<String, String>,
    canonical: &[CcEntry],
    entry: &Entry,
) -> Result<EntryCheck, EntryError> {
    let mut check = EntryCheck {
        result: EntryResult::default(),
        steps: Vec::new(),
        not_proven: false,
    };
    let defining = lexical(&layout.src.join(&entry.source));
    let Some(unit) = canonical
        .iter()
        .find(|candidate| resolved(&candidate.directory, &candidate.file) == defining)
    else {
        check.result.detail = Some("defining TU not captured".to_owned());
        return Ok(check);
    };
    let header = lexical(&layout.src.join(&entry.header));
    let probe = layout.capture_dir.join(PROBE_FILE);
    fs::write(&probe, probe_source(&header)?).map_err(|source| EntryError::Io {
        path: probe.clone(),
        source,
    })?;
    let flags = filter_flags(unit.arguments.get(1..).unwrap_or_default());
    let cwd = Path::new(&unit.directory);
    let mut notes: Vec<String> = Vec::new();

    let ast = clang_ast_argv(tools, &entry.name, &flags, &probe)?;
    match run_clang(
        cfg,
        tools,
        env,
        &format!("clang ast {}", entry.name),
        &ast,
        cwd,
        &mut check.steps,
    )? {
        Run::Done(stdout) => match parse_ast_stream(&stdout) {
            Ok(declarations) => {
                let matching: Vec<&FunctionDecl> = declarations
                    .iter()
                    .filter(|declaration| declaration.name == entry.name)
                    .collect();
                let expected = normalise_signature(&entry.signature);
                check.result.found = !matching.is_empty();
                check.result.signature_matches = check.result.found
                    && matching
                        .iter()
                        .all(|declaration| normalise_signature(&declaration.qual_type) == expected);
                let shown = matching
                    .iter()
                    .find(|declaration| normalise_signature(&declaration.qual_type) != expected)
                    .or_else(|| matching.first());
                check.result.found_signature =
                    shown.map(|declaration| declaration.qual_type.clone());
                if !check.result.found {
                    notes.push(format!(
                        "clang found no function named {} in {}",
                        entry.name, entry.header
                    ));
                } else if !check.result.signature_matches {
                    notes.push(format!(
                        "signature mismatch: the manifest says {expected:?}, clang printed {:?}",
                        check.result.found_signature.as_deref().unwrap_or_default()
                    ));
                }
            }
            Err(error) => notes.push(error.to_string()),
        },
        Run::NotProven => {
            check.not_proven = true;
            notes.push("clang ast dump timed out or left an incomplete output stream".to_owned());
        }
        Run::Failed(code, stderr) => {
            notes.push(format!("clang ast dump exited with {code:?}: {stderr}"));
        }
    }

    let deps = clang_deps_argv(tools, &flags, &unit.file)?;
    match run_clang(
        cfg,
        tools,
        env,
        &format!("clang deps {}", entry.name),
        &deps,
        cwd,
        &mut check.steps,
    )? {
        Run::Done(stdout) => {
            let listed = parse_make_deps(&String::from_utf8_lossy(&stdout));
            check.result.header_included = listed
                .iter()
                .any(|dependency| resolved(&unit.directory, dependency) == header);
            if !check.result.header_included {
                notes.push(format!(
                    "{} is not a dependency of {}",
                    entry.header, entry.source
                ));
            }
        }
        Run::NotProven => {
            check.not_proven = true;
            notes.push(
                "clang dependency listing timed out or left an incomplete output stream".to_owned(),
            );
        }
        Run::Failed(code, stderr) => {
            notes.push(format!(
                "clang dependency listing exited with {code:?}: {stderr}"
            ));
        }
    }

    if !notes.is_empty() {
        let prefix = if check.not_proven { "not_proven: " } else { "" };
        check.result.detail = Some(format!("{prefix}{}", notes.join("; ")));
    }
    Ok(check)
}
