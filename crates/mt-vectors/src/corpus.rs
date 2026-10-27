//! The sha256-named corpus and its provenance index (D-19, D-21).
//!
//! A corpus directory holds one target, configuration and entry: the vectors
//! as `<sha256>.json` (the canonical bytes of [`Vector`], named by their
//! sha256) and `index.json`, which maps each sha256 to a [`Provenance`] record.
//! Provenance stays out of the vectors so they keep their nine fields.
//!
//! A corpus is written once, into an empty directory, from the whole set (the
//! executor holds the vectors in memory until every run has finished). Reading
//! refuses anything that is not part of it, so a stray file never counts as
//! evidence (T-03-13).

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::vector::{MAX_VECTOR_BYTES, OutcomeClass, Vector, VectorError};

/// Version of the `index.json` schema.
pub const CORPUS_INDEX_SCHEMA_VERSION: u32 = 1;

const INDEX_FILE: &str = "index.json";
const VECTOR_SUFFIX: &str = ".json";

/// Where a vector's `has_ub` status came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HasUbSource {
    /// No label and no UB tool has looked yet: "UB unchecked".
    Unchecked,
    /// The label records an observed fault.
    ObservedCrash,
    /// The label records an observed abort.
    ObservedAbort,
    /// The label records a write into the slack beside a guarded buffer.
    OobWrite,
    /// The label records a timeout.
    Timeout,
    /// Phase 4's tools set or cleared the label.
    Phase4,
}

/// The budgets a vector was produced under.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProvenanceBudgets {
    /// Wall-clock timeout in milliseconds.
    pub timeout_ms: u64,
    /// Reserved guest address space (`-R`).
    pub reserved_va: u64,
    /// Heap arena size in bytes.
    pub heap_arena: u64,
    /// Sentinel byte filled into output buffers.
    pub sentinel: u8,
    /// Fill byte of the slack beside a guarded buffer.
    pub slack_fill: u8,
    /// Largest input in bytes.
    pub max_input: u64,
}

/// How one vector was produced: the record kept beside it in the index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    /// sha256 of the seed the input came from.
    pub seed: String,
    /// The scalar arguments by name.
    pub scalars: BTreeMap<String, i64>,
    /// sha256 of each captured object, by path.
    pub objects: BTreeMap<String, String>,
    /// sha256 of the archive the objects were linked from.
    pub archive: String,
    /// sha256 of the generated driver source.
    pub driver_source: String,
    /// sha256 of the linked driver ELF.
    pub driver_elf: String,
    /// The toolchain image reference.
    pub image: String,
    /// The executor version.
    pub executor_version: String,
    /// The contract version.
    pub contract_version: String,
    /// sha256 of the module's policy file.
    pub policy_sha256: String,
    /// The budgets the run used.
    pub budgets: ProvenanceBudgets,
    /// The outcome class word; equals the vector's own.
    pub outcome: String,
    /// Where the vector's `has_ub` status came from.
    pub has_ub_source: HasUbSource,
    /// The number of unobserved marks in the vector's states.
    pub unobserved: u64,
    /// Bytes of libc output the driver discarded.
    pub discarded_output_bytes: u64,
    /// `identical` or `not_checked`: whether a replay reproduced the vector.
    pub replay: String,
}

/// The `index.json` of a corpus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorpusIndex {
    /// Always [`CORPUS_INDEX_SCHEMA_VERSION`].
    pub schema_version: u32,
    /// Test-bed target id.
    pub target: String,
    /// Build configuration name.
    pub config: String,
    /// Entry function.
    pub entry: String,
    /// The provenance of each vector, by sha256.
    pub vectors: BTreeMap<String, Provenance>,
}

/// A corpus read back from a directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Corpus {
    /// The index.
    pub index: CorpusIndex,
    /// The vectors by sha256.
    pub vectors: BTreeMap<String, Vector>,
}

/// Why a corpus cannot be written or is refused on reading.
#[derive(Debug, thiserror::Error)]
pub enum CorpusError {
    /// A file or directory could not be read or written.
    #[error("{action} {}: {source}", path.display())]
    Io {
        /// What was being done.
        action: &'static str,
        /// The path.
        path: PathBuf,
        /// The cause.
        #[source]
        source: io::Error,
    },
    /// The target directory already holds something.
    #[error("{} is not empty; a corpus is written only into an empty directory", .0.display())]
    NotEmpty(PathBuf),
    /// There is nothing to write.
    #[error("a corpus needs at least one vector")]
    Empty,
    /// The vectors do not all belong to one target, configuration and entry.
    #[error("vectors of different entries in one corpus: {first:?} and {other:?}")]
    MixedEntries {
        /// `argv[0]` of the first vector.
        first: String,
        /// A different `argv[0]`.
        other: String,
    },
    /// Two vectors have the same bytes.
    #[error("two vectors have the same sha256 {0}")]
    Duplicate(String),
    /// A vector is invalid.
    #[error("vector {name}: {source}")]
    Vector {
        /// The sha256 or file name.
        name: String,
        /// The cause.
        #[source]
        source: VectorError,
    },
    /// A provenance record breaks a rule.
    #[error("provenance of {name}: {}", .issues.join("; "))]
    Provenance {
        /// The sha256.
        name: String,
        /// Every rule that was broken.
        issues: Vec<String>,
    },
    /// An entry of the directory is not part of a corpus.
    #[error("{name} is not part of the corpus: {reason}")]
    Foreign {
        /// The entry name.
        name: String,
        /// Why it is refused.
        reason: String,
    },
    /// A file's sha256 differs from its name.
    #[error("{name}: its sha256 is {actual}, not the name")]
    ShaMismatch {
        /// The file name.
        name: String,
        /// The sha256 of its bytes.
        actual: String,
    },
    /// The index or the file set is inconsistent.
    #[error("index: {0}")]
    Index(String),
    /// The directory changed while it was written.
    #[error("{name} appeared in the corpus directory that this write did not create")]
    Tampered {
        /// The unexpected entry.
        name: String,
    },
}

fn io_error(action: &'static str, path: &Path, source: io::Error) -> CorpusError {
    CorpusError::Io {
        action,
        path: path.to_path_buf(),
        source,
    }
}

fn is_sha256(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// `argv[0]` split into target, configuration and entry.
fn entry_of(vector: &Vector) -> Option<(String, String, String)> {
    let first = vector.argv.first()?;
    let mut parts = first.split('/');
    let (target, config, entry) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() {
        return None;
    }
    Some((target.to_owned(), config.to_owned(), entry.to_owned()))
}

impl Provenance {
    /// Check the record on its own and against the vector it describes.
    ///
    /// The `has_ub` source must fit the label: an unlabelled vector is
    /// `unchecked` or `phase4`; a labelled one names how it was observed, or
    /// is `phase4`.
    pub fn issues(&self, vector: &Vector) -> Vec<String> {
        let mut issues = Vec::new();
        for (name, value) in [
            ("seed", &self.seed),
            ("archive", &self.archive),
            ("driver_source", &self.driver_source),
            ("driver_elf", &self.driver_elf),
            ("policy_sha256", &self.policy_sha256),
        ] {
            if !is_sha256(value) {
                issues.push(format!("{name} is not a lowercase sha256"));
            }
        }
        for (path, value) in &self.objects {
            if path.is_empty() || !is_sha256(value) {
                issues.push(format!("objects entry {path:?} is not a path and a sha256"));
            }
        }
        for (name, value) in [
            ("image", &self.image),
            ("executor_version", &self.executor_version),
            ("contract_version", &self.contract_version),
        ] {
            if value.trim().is_empty() {
                issues.push(format!("{name} is empty"));
            }
        }
        match (
            OutcomeClass::from_word(&self.outcome),
            vector.outcome_class(),
        ) {
            (Some(recorded), Ok(own)) if recorded == own => {}
            _ => issues.push(format!(
                "outcome {:?} is not the vector's own outcome class",
                self.outcome
            )),
        }
        if !matches!(self.replay.as_str(), "identical" | "not_checked") {
            issues.push(format!(
                "replay {:?} is not identical or not_checked",
                self.replay
            ));
        }
        let labelled = vector.has_ub.is_some();
        let source_fits = match self.has_ub_source {
            HasUbSource::Unchecked => !labelled,
            HasUbSource::Phase4 => true,
            HasUbSource::ObservedCrash
            | HasUbSource::ObservedAbort
            | HasUbSource::OobWrite
            | HasUbSource::Timeout => labelled,
        };
        if !source_fits {
            issues.push(format!(
                "has_ub_source {:?} does not fit a vector {} a has_ub label",
                self.has_ub_source,
                if labelled { "with" } else { "without" }
            ));
        }
        let marks =
            vector.lib_state_in.unobserved_count() + vector.lib_state_out.unobserved_count();
        if u64::try_from(marks).ok() != Some(self.unobserved) {
            issues.push(format!(
                "unobserved is {} but the vector's states hold {marks} unobserved marks",
                self.unobserved
            ));
        }
        issues
    }
}

fn index_bytes(index: &CorpusIndex) -> Result<Vec<u8>, CorpusError> {
    let mut bytes = serde_json::to_vec_pretty(index)
        .map_err(|e| CorpusError::Index(format!("cannot write the index: {e}")))?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// Write the whole set into `dir`, which must be absent or empty.
///
/// Everything is checked before the directory is touched. After writing, the
/// directory is listed, and any entry this call did not write fails with
/// [`CorpusError::Tampered`]. `index.json` is written last.
pub fn write_corpus(dir: &Path, set: &[(Vector, Provenance)]) -> Result<(), CorpusError> {
    let Some((first, _)) = set.first() else {
        return Err(CorpusError::Empty);
    };
    let Some((target, config, entry)) = entry_of(first) else {
        return Err(CorpusError::Vector {
            name: first.sha256(),
            source: VectorError::Invalid {
                issues: vec!["argv[0] is not <target>/<config>/<entry>".to_owned()],
            },
        });
    };
    let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let mut vectors = BTreeMap::new();
    for (vector, provenance) in set {
        let sha = vector.sha256();
        let bytes = vector
            .to_canonical_json()
            .map_err(|source| CorpusError::Vector {
                name: sha.clone(),
                source,
            })?;
        if vector.argv.first() != first.argv.first() {
            return Err(CorpusError::MixedEntries {
                first: first.argv.first().cloned().unwrap_or_default(),
                other: vector.argv.first().cloned().unwrap_or_default(),
            });
        }
        let issues = provenance.issues(vector);
        if !issues.is_empty() {
            return Err(CorpusError::Provenance { name: sha, issues });
        }
        if files
            .insert(format!("{sha}{VECTOR_SUFFIX}"), bytes)
            .is_some()
        {
            return Err(CorpusError::Duplicate(sha));
        }
        vectors.insert(sha, provenance.clone());
    }
    let index = CorpusIndex {
        schema_version: CORPUS_INDEX_SCHEMA_VERSION,
        target,
        config,
        entry,
        vectors,
    };
    files.insert(INDEX_FILE.to_owned(), index_bytes(&index)?);

    match fs::read_dir(dir) {
        Ok(mut entries) => {
            if entries.next().is_some() {
                return Err(CorpusError::NotEmpty(dir.to_path_buf()));
            }
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            fs::create_dir_all(dir).map_err(|e| io_error("creating", dir, e))?;
        }
        Err(e) => return Err(io_error("reading directory", dir, e)),
    }
    // The index is last so a directory without one is visibly unfinished.
    let index_name = INDEX_FILE.to_owned();
    let mut order: Vec<&String> = files.keys().filter(|n| **n != index_name).collect();
    order.push(&index_name);
    for name in order {
        let path = dir.join(name);
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| io_error("creating", &path, e))?;
        file.write_all(&files[name])
            .map_err(|e| io_error("writing", &path, e))?;
    }
    for entry in fs::read_dir(dir).map_err(|e| io_error("reading directory", dir, e))? {
        let entry = entry.map_err(|e| io_error("reading directory", dir, e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !files.contains_key(&name) {
            return Err(CorpusError::Tampered { name });
        }
    }
    Ok(())
}

fn vector_stem(name: &str) -> Option<&str> {
    name.strip_suffix(VECTOR_SUFFIX)
        .filter(|stem| is_sha256(stem))
}

/// Read a corpus directory, refusing anything that is not part of one.
///
/// Only regular files named `<64 lowercase hex>.json` and `index.json` are
/// accepted (symlinks, directories and every other name are refused), each
/// vector file must hold the canonical bytes of a valid vector whose sha256 is
/// its name, the index keys must equal the file set, and every provenance
/// record must fit its vector.
pub fn read_corpus(dir: &Path) -> Result<Corpus, CorpusError> {
    let cap = u64::try_from(MAX_VECTOR_BYTES).unwrap_or(u64::MAX);
    let mut names = BTreeSet::new();
    for entry in fs::read_dir(dir).map_err(|e| io_error("reading directory", dir, e))? {
        let entry = entry.map_err(|e| io_error("reading directory", dir, e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let foreign = |reason: &str| CorpusError::Foreign {
            name: name.clone(),
            reason: reason.to_owned(),
        };
        let kind = entry
            .file_type()
            .map_err(|e| io_error("inspecting", &entry.path(), e))?;
        if kind.is_symlink() {
            return Err(foreign("it is a symlink"));
        }
        if !kind.is_file() {
            return Err(foreign("it is not a regular file"));
        }
        if name != INDEX_FILE && vector_stem(&name).is_none() {
            return Err(foreign(
                "its name is neither index.json nor <lowercase sha256>.json",
            ));
        }
        let len = entry
            .metadata()
            .map_err(|e| io_error("inspecting", &entry.path(), e))?
            .len();
        if len > cap {
            return Err(foreign(&format!("it is {len} bytes, above the size limit")));
        }
        names.insert(name);
    }

    if !names.contains(INDEX_FILE) {
        return Err(CorpusError::Index("index.json is missing".to_owned()));
    }
    let index_path = dir.join(INDEX_FILE);
    let index: CorpusIndex = serde_json::from_slice(
        &fs::read(&index_path).map_err(|e| io_error("reading", &index_path, e))?,
    )
    .map_err(|e| CorpusError::Index(format!("not a valid index: {e}")))?;
    if index.schema_version != CORPUS_INDEX_SCHEMA_VERSION {
        return Err(CorpusError::Index(format!(
            "schema_version {} is not {CORPUS_INDEX_SCHEMA_VERSION}",
            index.schema_version
        )));
    }

    let mut vectors = BTreeMap::new();
    for name in names.iter().filter(|n| n.as_str() != INDEX_FILE) {
        let Some(stem) = vector_stem(name) else {
            continue;
        };
        let path = dir.join(name);
        let bytes = fs::read(&path).map_err(|e| io_error("reading", &path, e))?;
        let actual = crate::sha256_hex(&bytes);
        if actual != stem {
            return Err(CorpusError::ShaMismatch {
                name: name.clone(),
                actual,
            });
        }
        let vector = Vector::from_json_bytes(&bytes).map_err(|source| CorpusError::Vector {
            name: name.clone(),
            source,
        })?;
        if vector.to_canonical_json().ok().as_deref() != Some(bytes.as_slice()) {
            return Err(CorpusError::Foreign {
                name: name.clone(),
                reason: "its bytes are not the canonical form of the vector".to_owned(),
            });
        }
        vectors.insert(stem.to_owned(), vector);
    }

    for key in index.vectors.keys() {
        if !vectors.contains_key(key) {
            return Err(CorpusError::Index(format!(
                "{key} is listed but {key}{VECTOR_SUFFIX} is missing"
            )));
        }
    }
    for (sha, vector) in &vectors {
        let Some(provenance) = index.vectors.get(sha) else {
            return Err(CorpusError::Index(format!(
                "{sha}{VECTOR_SUFFIX} is not listed in the index"
            )));
        };
        let issues = provenance.issues(vector);
        if !issues.is_empty() {
            return Err(CorpusError::Provenance {
                name: sha.clone(),
                issues,
            });
        }
        let own = entry_of(vector);
        if own.as_ref()
            != Some(&(
                index.target.clone(),
                index.config.clone(),
                index.entry.clone(),
            ))
        {
            return Err(CorpusError::Index(format!(
                "{sha} is not a vector of {}/{}/{}",
                index.target, index.config, index.entry
            )));
        }
    }
    Ok(Corpus { index, vectors })
}
