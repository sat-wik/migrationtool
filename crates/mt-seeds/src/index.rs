//! The per-target seed index and the committed corpus directory (D-15).
//!
//! A corpus directory holds exactly the seed files (named by the sha256 of
//! their bytes, no extension), `index.json` and `NOTICE`. Nothing in it
//! depends on the clock.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::select::{Caps, Selection, Skipped, SourceRef};
use crate::{Method, SeedError, sha256_hex};

/// Version of the `index.json` schema.
pub const SEED_INDEX_SCHEMA_VERSION: u32 = 1;

const INDEX_FILE: &str = "index.json";
const NOTICE_FILE: &str = "NOTICE";

/// The licence the seeds carry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexLicence {
    /// SPDX expression of the target's licence.
    pub spdx: String,
    /// The alternative elected from a dual licence, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elected: Option<String>,
    /// Upstream licence file the seeds fall under.
    pub file: String,
}

/// One seed in the index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeedRecord {
    /// Lower-case hex sha256 of the seed, which is also its file name.
    pub sha256: String,
    /// Length of the seed in bytes.
    pub bytes: u64,
    /// Entry points the seed is meant to exercise.
    pub entries: Vec<String>,
    /// How the seed was obtained.
    pub method: Method,
    /// Where it came from.
    pub source: SourceRef,
    /// True when the seed is a proposal awaiting review.
    pub proposed: bool,
}

/// The per-target index of a seed corpus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeedIndex {
    /// Always [`SEED_INDEX_SCHEMA_VERSION`].
    pub schema_version: u32,
    /// Test-bed target id.
    pub target: String,
    /// Upstream commit the seeds were taken from.
    pub commit: String,
    /// Licence of the seeds.
    pub licence: IndexLicence,
    /// Caps the selection ran under.
    pub caps: Caps,
    /// Seeds sorted by sha256.
    pub seeds: Vec<SeedRecord>,
    /// Counts of what was left out.
    pub skipped: Skipped,
}

impl SeedIndex {
    /// Build the index for `selection`, with seeds sorted by sha256.
    #[must_use]
    pub fn from_selection(
        target: &str,
        commit: &str,
        licence: IndexLicence,
        caps: Caps,
        selection: &Selection,
    ) -> Self {
        let mut seeds: Vec<SeedRecord> = selection
            .seeds
            .iter()
            .map(|seed| SeedRecord {
                sha256: seed.sha256.clone(),
                bytes: u64::try_from(seed.candidate.bytes.len()).unwrap_or(u64::MAX),
                entries: seed.candidate.entries.clone(),
                method: seed.candidate.method,
                source: seed.candidate.source.clone(),
                proposed: seed.candidate.proposed,
            })
            .collect();
        seeds.sort_by(|a, b| a.sha256.cmp(&b.sha256));
        Self {
            schema_version: SEED_INDEX_SCHEMA_VERSION,
            target: target.to_owned(),
            commit: commit.to_owned(),
            licence,
            caps,
            seeds,
            skipped: selection.skipped,
        }
    }

    /// Pretty JSON with a trailing newline and no clock-derived field.
    pub fn to_json(&self) -> Result<String, SeedError> {
        let mut text = serde_json::to_string_pretty(self)?;
        text.push('\n');
        Ok(text)
    }
}

fn io_error(operation: &'static str, path: &Path, source: io::Error) -> SeedError {
    SeedError::Io {
        operation,
        path: path.to_path_buf(),
        source,
    }
}

/// The files `write_corpus` would produce, by name, after checking that the
/// index and the selection describe the same seeds.
fn expected_files(
    index: &SeedIndex,
    selection: &Selection,
    notice: &str,
) -> Result<BTreeMap<String, Vec<u8>>, SeedError> {
    let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    for seed in &selection.seeds {
        if sha256_hex(&seed.candidate.bytes) != seed.sha256 {
            return Err(SeedError::Inconsistent(format!(
                "seed {} does not hash to its recorded sha256",
                seed.sha256
            )));
        }
        files.insert(seed.sha256.clone(), seed.candidate.bytes.clone());
    }
    let mut recorded = BTreeMap::new();
    for record in &index.seeds {
        recorded.insert(record.sha256.as_str(), record.bytes);
    }
    let same_seeds = recorded.len() == index.seeds.len()
        && recorded.len() == files.len()
        && files.iter().all(|(sha, bytes)| {
            recorded
                .get(sha.as_str())
                .is_some_and(|len| u64::try_from(bytes.len()).is_ok_and(|n| n == *len))
        });
    if !same_seeds {
        return Err(SeedError::Inconsistent(
            "the index lists different seeds than the selection".to_owned(),
        ));
    }
    files.insert(INDEX_FILE.to_owned(), index.to_json()?.into_bytes());
    files.insert(NOTICE_FILE.to_owned(), notice.as_bytes().to_vec());
    Ok(files)
}

/// Replace the contents of `dir` with the seed files, `index.json` and `NOTICE`.
///
/// `dir` is created if it does not exist. Existing files (and symlinks, which
/// are removed rather than followed) are deleted first; a subdirectory is an
/// error and nothing is deleted recursively. The index and the selection are
/// checked against each other before anything is touched.
pub fn write_corpus(
    dir: &Path,
    index: &SeedIndex,
    selection: &Selection,
    notice: &str,
) -> Result<(), SeedError> {
    let files = expected_files(index, selection, notice)?;
    match fs::read_dir(dir) {
        Ok(entries) => {
            let mut stale = Vec::new();
            for entry in entries {
                let entry = entry.map_err(|e| io_error("reading directory", dir, e))?;
                let path = entry.path();
                let file_type = entry
                    .file_type()
                    .map_err(|e| io_error("inspecting", &path, e))?;
                if file_type.is_dir() {
                    return Err(SeedError::UnexpectedDirectory(path));
                }
                stale.push(path);
            }
            for path in stale {
                fs::remove_file(&path).map_err(|e| io_error("removing", &path, e))?;
            }
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            fs::create_dir_all(dir).map_err(|e| io_error("creating", dir, e))?;
        }
        Err(e) => return Err(io_error("reading directory", dir, e)),
    }
    for (name, bytes) in &files {
        let path = dir.join(name);
        fs::write(&path, bytes).map_err(|e| io_error("writing", &path, e))?;
    }
    Ok(())
}

/// What `check_corpus` found on disk under one name.
enum OnDisk {
    File(Vec<u8>),
    /// A directory, symlink or other non-regular entry.
    Other,
}

/// Compare `dir` with what [`write_corpus`] would write.
///
/// Returns one line per difference, ordered by file name: `added <name>` for a
/// file that should not be there, `missing <name>` for one that is absent and
/// `changed <name>` for one with different bytes (or that is not a regular
/// file). The result is empty when the directory matches. A directory that
/// does not exist has every file missing.
pub fn check_corpus(
    dir: &Path,
    index: &SeedIndex,
    selection: &Selection,
    notice: &str,
) -> Result<Vec<String>, SeedError> {
    let expected = expected_files(index, selection, notice)?;
    let mut actual: BTreeMap<String, OnDisk> = BTreeMap::new();
    match fs::read_dir(dir) {
        Ok(entries) => {
            for entry in entries {
                let entry = entry.map_err(|e| io_error("reading directory", dir, e))?;
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().into_owned();
                let file_type = entry
                    .file_type()
                    .map_err(|e| io_error("inspecting", &path, e))?;
                if file_type.is_file() {
                    let bytes = fs::read(&path).map_err(|e| io_error("reading", &path, e))?;
                    actual.insert(name, OnDisk::File(bytes));
                } else {
                    actual.insert(name, OnDisk::Other);
                }
            }
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(io_error("reading directory", dir, e)),
    }

    let mut names: Vec<&String> = expected.keys().chain(actual.keys()).collect();
    names.sort_unstable();
    names.dedup();
    let mut differences = Vec::new();
    for name in names {
        match (expected.get(name), actual.get(name)) {
            (Some(want), Some(OnDisk::File(have))) if want == have => {}
            (Some(_), Some(_)) => differences.push(format!("changed {name}")),
            (Some(_), None) => differences.push(format!("missing {name}")),
            (None, Some(_)) => differences.push(format!("added {name}")),
            (None, None) => {}
        }
    }
    Ok(differences)
}
