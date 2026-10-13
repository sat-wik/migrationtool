//! Manifest-driven seed extraction and the committed corpus check (CONTEXT D-13,
//! D-15, D-16, D-21; BUILD-03).
//!
//! [`seed_target`] reads only the paths a target's `seed_source` tables declare
//! (RESEARCH Pitfall 18: never a copyleft directory of a mixed-licence
//! repository), applies the listed methods, adds the target's committed
//! hand-made inputs and selects under the manifest caps with
//! [`mt_seeds::select::select`]. [`run_seeds`] does that for the selected
//! targets from their verified cached trees and then writes
//! `<seeds_dir>/<id>/` or, in check mode, compares it with what would be
//! written and reports any difference as drift.
//!
//! Bounds (T-02-32): a source file above 8 MiB is counted unsupported and never
//! read; a zip archive above 2 MiB is counted unsupported and never
//! opened, because the number of members of an archive is not bounded by the
//! per-member limit. Symlinks are never followed.
//!
//! Nothing here starts a process; the cache is verified through
//! [`crate::fetch::verify_cached`], which uses the runner.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use mt_seeds::index::{IndexLicence, SeedIndex, check_corpus, write_corpus};
use mt_seeds::select::{Candidate, Caps, Selection as SeedSelection, SourceRef, select};
use mt_seeds::{Method, SeedError, hex, lexer, zipsrc};

use crate::fetch::{FetchContext, FetchError, TAR_PATH, cache_paths, pinned_tool, verify_cached};
use crate::manifest::{Source, Target};
use crate::testbed::{
    FETCH_TIMEOUT, StepSummary, TestbedError, TestbedOptions, load_inputs, runner_config,
    target_of, write_summary,
};

/// The largest source file that is read; a bigger one is counted unsupported.
pub const MAX_SOURCE_FILE_BYTES: u64 = 8 * 1024 * 1024;

/// The largest zip archive that is opened; a bigger one is counted unsupported.
/// The members of an archive are bounded one by one but not in number, so the
/// archive size is what bounds the memory a pinned archive can take.
pub const MAX_ZIP_ARCHIVE_BYTES: u64 = 2 * 1024 * 1024;

/// Extensions of C and C++ sources the lexer runs on.
const C_EXTENSIONS: [&str; 5] = ["c", "cc", "cpp", "h", "hpp"];

/// Why the seeds of a target could not be extracted.
#[derive(Debug, thiserror::Error)]
pub enum SeedingError {
    /// A declared seed path is not in the tree.
    #[error("seed source {path:?} is not in the tree")]
    SourceMissing {
        /// The declared path.
        path: String,
    },
    /// A declared seed path goes through a symlink.
    #[error("seed source {path:?} goes through a symlink; symlinks are never followed")]
    SourceSymlink {
        /// The declared path.
        path: String,
    },
    /// A seed source names something that is not an extraction method.
    #[error("seed source {path:?}: {detail}")]
    InvalidSource {
        /// The declared path.
        path: String,
        /// What is wrong.
        detail: String,
    },
    /// A hand-made input cannot be used.
    #[error("hand-made input {}: {detail}", path.display())]
    HandMade {
        /// The file.
        path: PathBuf,
        /// What is wrong.
        detail: String,
    },
    /// A file operation failed.
    #[error("cannot {action} {}: {source}", path.display())]
    Io {
        /// What was attempted.
        action: &'static str,
        /// The path involved.
        path: PathBuf,
        /// The underlying error.
        #[source]
        source: io::Error,
    },
    /// The seed library refused something.
    #[error(transparent)]
    Seed(#[from] SeedError),
}

fn io_error(action: &'static str, path: &Path, source: io::Error) -> SeedingError {
    SeedingError::Io {
        action,
        path: path.to_path_buf(),
        source,
    }
}

/// What a seeds run found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedsRun {
    /// One row per target: `seeds <count>`, `drift` or `failed: <cause>`.
    pub summary: StepSummary,
    /// Per target, the difference lines of a check that found drift (`added`,
    /// `missing` or `changed` and a file name).
    pub drift: BTreeMap<String, Vec<String>>,
}

/// The hand-made directory of a target under the committed testbed directory.
#[must_use]
pub fn hand_made_dir(testbed_dir: &Path, id: &str) -> PathBuf {
    testbed_dir.join("targets").join(id).join("hand-made")
}

/// The commit a target's source is pinned to.
#[must_use]
pub fn source_commit(target: &Target) -> &str {
    match &target.source {
        Source::Git { commit, .. } | Source::Archive { commit, .. } => commit,
    }
}

fn source_url(target: &Target) -> &str {
    match &target.source {
        Source::Git { url, .. } | Source::Archive { url, .. } => url,
    }
}

/// The `NOTICE` text of a target's corpus: where the seeds come from and under
/// which licence (RESEARCH "Licensing of seeds"). It is a plain description, not
/// a legal opinion.
#[must_use]
pub fn notice_text(id: &str, commit: &str, target: &Target) -> String {
    let licence = &target.licence;
    format!(
        "Seed corpus of the test-bed target {id}\n\
         \n\
         Upstream: {url}\n\
         Commit: {commit}\n\
         Licence (SPDX expression): {spdx}\n\
         Elected licence: {elected}\n\
         Licence file: {file}\n\
         \n\
         Seeds whose method is not hand_made are copies of upstream test data taken\n\
         from the paths the manifest declares as seed_source, and they carry the\n\
         licence above.\n\
         Seeds whose method is hand_made were written for this project from the\n\
         format specification named in each source file under\n\
         testbed/targets/{id}/hand-made/. They are not upstream test data, and\n\
         index.json marks each of them proposed.\n",
        url = source_url(target),
        spdx = licence.spdx,
        elected = licence.elected,
        file = licence.file,
    )
}

fn parse_method(path: &str, text: &str) -> Result<Method, SeedingError> {
    match text {
        "file_copy" => Ok(Method::FileCopy),
        "hex_text" => Ok(Method::HexText),
        "c_array" => Ok(Method::CArray),
        "c_string" => Ok(Method::CString),
        "zip_member" => Ok(Method::ZipMember),
        other => Err(SeedingError::InvalidSource {
            path: path.to_owned(),
            detail: format!("{other:?} is not an extraction method"),
        }),
    }
}

/// The extension of a file name as text; `""` when it has none.
fn extension_of(path: &Path) -> Option<&str> {
    match path.extension() {
        Some(ext) => ext.to_str(),
        None => Some(""),
    }
}

/// One file found under a declared path.
struct Found {
    /// Path relative to the tree root, `/`-separated.
    rel: String,
    abs: PathBuf,
}

/// The files of one declared source: the file itself, or every regular file
/// below the directory. Symlinks are never followed (neither on the way to the
/// path nor inside it) and `.git` is never entered. `skipped` counts names that
/// are not UTF-8, which cannot be recorded as provenance.
fn files_of(tree: &Path, declared: &str, skipped: &mut u64) -> Result<Vec<Found>, SeedingError> {
    let mut abs = tree.to_path_buf();
    let mut rel = String::new();
    for part in declared.split('/').filter(|p| !p.is_empty() && *p != ".") {
        if part == ".git" {
            return Err(SeedingError::InvalidSource {
                path: declared.to_owned(),
                detail: "a seed source must not be inside .git".to_owned(),
            });
        }
        abs.push(part);
        if !rel.is_empty() {
            rel.push('/');
        }
        rel.push_str(part);
        match fs::symlink_metadata(&abs) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(SeedingError::SourceSymlink {
                    path: declared.to_owned(),
                });
            }
            Ok(_) => {}
            Err(source) if source.kind() == io::ErrorKind::NotFound => {
                return Err(SeedingError::SourceMissing {
                    path: declared.to_owned(),
                });
            }
            Err(source) => return Err(io_error("inspect", &abs, source)),
        }
    }
    let meta = fs::symlink_metadata(&abs).map_err(|source| io_error("inspect", &abs, source))?;
    let mut files = Vec::new();
    if meta.is_file() {
        files.push(Found { rel, abs });
    } else if meta.is_dir() {
        let mut stack = vec![(abs, rel)];
        while let Some((dir, dir_rel)) = stack.pop() {
            let mut names = Vec::new();
            for entry in fs::read_dir(&dir).map_err(|source| io_error("read", &dir, source))? {
                let entry = entry.map_err(|source| io_error("read", &dir, source))?;
                names.push(entry.file_name());
            }
            names.sort();
            for name in names {
                let path = dir.join(&name);
                let Some(text) = name.to_str() else {
                    *skipped += 1;
                    continue;
                };
                if text == ".git" {
                    continue;
                }
                let child_rel = format!("{dir_rel}/{text}");
                let meta = fs::symlink_metadata(&path)
                    .map_err(|source| io_error("inspect", &path, source))?;
                let kind = meta.file_type();
                if kind.is_symlink() {
                    continue;
                } else if kind.is_dir() {
                    stack.push((path, child_rel));
                } else if kind.is_file() {
                    files.push(Found {
                        rel: child_rel,
                        abs: path,
                    });
                }
            }
        }
    }
    Ok(files)
}

/// Candidates and counts gathered before selection.
#[derive(Default)]
struct Gathered {
    candidates: Vec<Candidate>,
    unsupported: u64,
    too_large: u64,
}

fn entry_names(target: &Target) -> Vec<String> {
    let mut names: Vec<String> = target.entry.iter().map(|e| e.name.clone()).collect();
    names.sort();
    names.dedup();
    names
}

/// Apply the methods of one source to one file.
fn extract_file(
    found: &Found,
    methods: &[Method],
    entries: &[String],
    caps: &Caps,
    gathered: &mut Gathered,
) -> Result<(), SeedingError> {
    let meta = fs::symlink_metadata(&found.abs)
        .map_err(|source| io_error("inspect", &found.abs, source))?;
    if meta.len() > MAX_SOURCE_FILE_BYTES {
        gathered.unsupported += 1;
        return Ok(());
    }
    let extension = extension_of(Path::new(&found.rel)).unwrap_or("");
    let is_c = C_EXTENSIONS.contains(&extension);
    let is_zip = extension == "zip";
    let wants = |m: Method| methods.contains(&m);
    let needs_bytes = wants(Method::FileCopy)
        || wants(Method::HexText)
        || (is_c && (wants(Method::CArray) || wants(Method::CString)))
        || (is_zip && wants(Method::ZipMember));
    if !needs_bytes {
        return Ok(());
    }
    let bytes = fs::read(&found.abs).map_err(|source| io_error("read", &found.abs, source))?;
    let candidate =
        |bytes: Vec<u8>, method: Method, line: Option<u32>, member: Option<String>| Candidate {
            bytes,
            method,
            source: SourceRef {
                path: found.rel.clone(),
                line,
                member,
            },
            entries: entries.to_vec(),
            proposed: false,
        };
    if wants(Method::FileCopy) {
        gathered
            .candidates
            .push(candidate(bytes.clone(), Method::FileCopy, None, None));
    }
    if wants(Method::HexText) {
        match std::str::from_utf8(&bytes)
            .map_err(|_| ())
            .and_then(|text| hex::decode_hex_text(text).map_err(|_| ()))
        {
            Ok(decoded) => {
                gathered
                    .candidates
                    .push(candidate(decoded, Method::HexText, None, None))
            }
            Err(()) => gathered.unsupported += 1,
        }
    }
    if is_c {
        for (method, result) in [
            (
                Method::CArray,
                wants(Method::CArray).then(|| lexer::c_byte_arrays(&bytes)),
            ),
            (
                Method::CString,
                wants(Method::CString).then(|| lexer::c_string_literals(&bytes)),
            ),
        ] {
            if let Some(result) = result {
                gathered.unsupported += result.unsupported;
                for item in result.found {
                    gathered
                        .candidates
                        .push(candidate(item.bytes, method, Some(item.line), None));
                }
            }
        }
    }
    if is_zip && wants(Method::ZipMember) {
        if meta.len() > MAX_ZIP_ARCHIVE_BYTES {
            // The member count is not bounded by the per-member limit, so a larger
            // archive is not opened (T-02-32).
            gathered.unsupported += 1;
        } else {
            match zipsrc::zip_members(&bytes, caps.max_seed_bytes) {
                Ok(zip) => {
                    gathered.unsupported += zip.unsupported;
                    gathered.too_large += zip.too_large;
                    for member in zip.members {
                        gathered.candidates.push(candidate(
                            member.bytes,
                            Method::ZipMember,
                            None,
                            Some(member.name),
                        ));
                    }
                }
                Err(_) => gathered.unsupported += 1,
            }
        }
    }
    Ok(())
}

/// Every `*.hex` file directly under the hand-made directory becomes a proposed
/// `hand_made` candidate.
fn hand_made_candidates(
    id: &str,
    dir: &Path,
    entries: &[String],
    gathered: &mut Gathered,
) -> Result<(), SeedingError> {
    let listing = match fs::read_dir(dir) {
        Ok(listing) => listing,
        Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(source) => return Err(io_error("read", dir, source)),
    };
    let mut names = Vec::new();
    for entry in listing {
        let entry = entry.map_err(|source| io_error("read", dir, source))?;
        let kind = entry
            .file_type()
            .map_err(|source| io_error("inspect", &entry.path(), source))?;
        if !kind.is_file() {
            continue;
        }
        let name = entry.file_name();
        let Some(text) = name.to_str() else {
            return Err(SeedingError::HandMade {
                path: entry.path(),
                detail: "the file name is not UTF-8".to_owned(),
            });
        };
        if Path::new(text).extension().and_then(|e| e.to_str()) == Some("hex") {
            names.push(text.to_owned());
        }
    }
    names.sort();
    for name in names {
        let path = dir.join(&name);
        let size = fs::metadata(&path)
            .map_err(|source| io_error("inspect", &path, source))?
            .len();
        if size > MAX_SOURCE_FILE_BYTES {
            return Err(SeedingError::HandMade {
                path,
                detail: "the file is larger than 8 MiB".to_owned(),
            });
        }
        let text = fs::read_to_string(&path).map_err(|source| io_error("read", &path, source))?;
        let bytes = hex::decode_hex_text(&text).map_err(|error| SeedingError::HandMade {
            path: path.clone(),
            detail: error.to_string(),
        })?;
        gathered.candidates.push(Candidate {
            bytes,
            method: Method::HandMade,
            source: SourceRef {
                path: format!("testbed/targets/{id}/hand-made/{name}"),
                line: None,
                member: None,
            },
            entries: entries.to_vec(),
            proposed: true,
        });
    }
    Ok(())
}

/// Extract and select the seeds of one target (CONTEXT D-13, D-15, D-16).
///
/// `tree` is the verified source tree, `hand_made_dir` the target's committed
/// hand-made directory (it may be absent) and `commit` the pin recorded in the
/// index. Only the paths listed in `target.seed_source` are read from `tree`.
///
/// # Errors
/// [`SeedingError`] when a declared path is missing or reached through a
/// symlink, a source names an unknown method, a hand-made input is not valid
/// hex text, or a file cannot be read.
pub fn seed_target(
    id: &str,
    tree: &Path,
    commit: &str,
    target: &Target,
    hand_made_dir: &Path,
    caps: Caps,
) -> Result<(SeedIndex, SeedSelection), SeedingError> {
    let all_entries = entry_names(target);
    let mut gathered = Gathered::default();
    for source in &target.seed_source {
        let methods = source
            .methods
            .iter()
            .map(|m| parse_method(&source.path, m))
            .collect::<Result<Vec<_>, _>>()?;
        let entries = if source.entries.is_empty() {
            all_entries.clone()
        } else {
            let mut named = source.entries.clone();
            named.sort();
            named.dedup();
            named
        };
        let mut unreadable_names = 0;
        let files = files_of(tree, &source.path, &mut unreadable_names)?;
        gathered.unsupported += unreadable_names;
        for found in &files {
            let wanted = match &source.extensions {
                None => true,
                Some(list) => extension_of(Path::new(&found.rel))
                    .is_some_and(|ext| list.iter().any(|l| l == ext)),
            };
            if wanted {
                extract_file(found, &methods, &entries, &caps, &mut gathered)?;
            }
        }
    }
    hand_made_candidates(id, hand_made_dir, &all_entries, &mut gathered)?;
    let mut selection = select(gathered.candidates, &caps, gathered.unsupported);
    selection.skipped.too_large = selection
        .skipped
        .too_large
        .saturating_add(gathered.too_large);
    let licence = IndexLicence {
        spdx: target.licence.spdx.clone(),
        elected: Some(target.licence.elected.clone()),
        file: target.licence.file.clone(),
    };
    let index = SeedIndex::from_selection(id, commit, licence, caps, &selection);
    Ok((index, selection))
}

/// Extract or check the seeds of the selected targets, each from its verified
/// cached tree (no network), and keep the difference lines of any drift.
///
/// Without `check` the corpus `<seeds_dir>/<id>/` is rewritten. With `check`
/// nothing on disk changes: the corpus is compared with what would be written,
/// and any difference is the row `drift`. A cache that fails verification or a
/// target whose seeds cannot be extracted is the row `failed: <cause>`.
///
/// # Errors
/// [`TestbedError`] for unusable inputs or a git that cannot start.
pub fn run_seeds_detailed(
    opts: &TestbedOptions,
    seeds_dir: &Path,
    check: bool,
) -> Result<SeedsRun, TestbedError> {
    let inputs = load_inputs(opts, false)?;
    let cfg = runner_config(&inputs.pins, FETCH_TIMEOUT);
    let verify_ctx = FetchContext {
        work_dir: opts.work_dir.clone(),
        git: pinned_tool(&inputs.pins, "git")?,
        curl: PathBuf::new(),
        tar: PathBuf::from(TAR_PATH),
        allowed_protocols: String::new(),
        timeout: FETCH_TIMEOUT,
    };
    let caps: Caps = inputs.manifest.seed_caps.into();
    let mut summary = StepSummary::new("seeds");
    let mut drift: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for id in &inputs.ids {
        if let Some(dropped) = inputs.drops.target_dropped(id) {
            summary.add_dropped(id, dropped);
            continue;
        }
        let target = target_of(&inputs.manifest, id)?;
        match verify_cached(&cfg, &verify_ctx, id, target) {
            Ok(_) => {}
            Err(error @ (FetchError::InvalidInput(_) | FetchError::ToolMissing(_))) => {
                return Err(error.into());
            }
            Err(error) => {
                summary.add(id, &format!("failed: {error}"), false);
                continue;
            }
        }
        let commit = source_commit(target);
        let tree = cache_paths(&opts.work_dir, id, commit).tree;
        let hand = hand_made_dir(&opts.testbed_dir, id);
        let (index, selection) = match seed_target(id, &tree, commit, target, &hand, caps) {
            Ok(pair) => pair,
            Err(error) => {
                summary.add(id, &format!("failed: {error}"), false);
                continue;
            }
        };
        let notice = notice_text(id, commit, target);
        let dir = seeds_dir.join(id);
        let count = index.seeds.len();
        if check {
            match check_corpus(&dir, &index, &selection, &notice) {
                Ok(lines) if lines.is_empty() => summary.add(id, &format!("seeds {count}"), true),
                Ok(lines) => {
                    drift.insert(id.clone(), lines);
                    summary.add(id, "drift", false);
                }
                Err(error) => summary.add(id, &format!("failed: {error}"), false),
            }
        } else {
            match write_corpus(&dir, &index, &selection, &notice) {
                Ok(()) => summary.add(id, &format!("seeds {count}"), true),
                Err(error) => summary.add(id, &format!("failed: {error}"), false),
            }
        }
    }
    write_summary(opts, &summary)?;
    Ok(SeedsRun { summary, drift })
}

/// [`run_seeds_detailed`] without the difference lines.
///
/// # Errors
/// [`TestbedError`] for unusable inputs or a git that cannot start.
pub fn run_seeds(
    opts: &TestbedOptions,
    seeds_dir: &Path,
    check: bool,
) -> Result<StepSummary, TestbedError> {
    run_seeds_detailed(opts, seeds_dir, check).map(|run| run.summary)
}
