//! The committed sample of reference vectors (CONTEXT D-21, T-03-29).
//!
//! CI regenerates the full set of vectors; a small sample of it is committed under
//! `testbed/vectors-sample` so that offline tests and the next CI run have a fixed
//! reference. [`select_sample`] chooses, for each target that has a drivable entry,
//! one vector of the CI corpus by a fixed rule, and [`write_sample`] writes the
//! chosen vectors as one-vector corpora whose provenance is the CI corpus's own.
//! The choice depends only on the manifest, the adapter drop log and the corpus, so
//! the same inputs always give the same sample.

use std::collections::BTreeSet;
use std::path::Path;

use mt_capture::manifest::Manifest;
use mt_vectors::bytes::decode_latin1;
use mt_vectors::corpus::{Provenance, read_corpus, write_corpus};
use mt_vectors::vector::Vector;

use crate::HarnessError;
use crate::adapter::{Adapter, combinations};
use crate::adapter_drops::AdapterDropLog;

/// The configuration the sample is taken from.
pub const SAMPLE_CONFIG: &str = "default";

/// One chosen vector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SampleChoice {
    /// The target id.
    pub target: String,
    /// The configuration name, always [`SAMPLE_CONFIG`].
    pub config: String,
    /// The entry function.
    pub entry: String,
    /// The sha256 of the chosen vector file.
    pub vector_sha256: String,
    /// The sha256 of the seed it was made from.
    pub seed_sha256: String,
    /// The size of that seed in bytes.
    pub input_len: usize,
}

impl SampleChoice {
    /// The listing row: `<t>/<c>/<e> <vector sha256> seed=<seed sha256> bytes=<n>`.
    #[must_use]
    pub fn render(&self) -> String {
        format!(
            "{}/{}/{} {} seed={} bytes={}",
            self.target,
            self.config,
            self.entry,
            self.vector_sha256,
            self.seed_sha256,
            self.input_len
        )
    }
}

fn input(what: impl Into<String>, detail: impl Into<String>) -> HarnessError {
    HarnessError::Input {
        what: what.into(),
        detail: detail.into(),
    }
}

/// The manifest, validated, and the adapter drop log next to it, validated.
///
/// # Errors
/// [`HarnessError::Input`] when either cannot be read or is invalid.
pub fn load_inputs(manifest_path: &Path) -> Result<(Manifest, AdapterDropLog), HarnessError> {
    let manifest = Manifest::load(manifest_path)
        .map_err(|e| input(manifest_path.display().to_string(), e.to_string()))?;
    manifest
        .validate()
        .map_err(|e| input(manifest_path.display().to_string(), e.to_string()))?;
    let testbed = match manifest_path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => Path::new(".").to_path_buf(),
    };
    let log_path = testbed.join("adapter-dropped.toml");
    let drops = AdapterDropLog::load_from_testbed_dir(&testbed)
        .map_err(|e| input(log_path.display().to_string(), e.to_string()))?;
    drops
        .validate(&manifest)
        .map_err(|e| input(log_path.display().to_string(), e.to_string()))?;
    Ok((manifest, drops))
}

/// Choose the sample of the corpora under `corpus_root` (`<root>/<target>/<config>/
/// <entry>`): for each target, in id order, the default configuration, the first
/// entry by name that has roles and is not in the drop log, the vector of the first
/// scalar combination whose seed is the smallest (ties by the seed's sha256, then the
/// vector's). A target with no such entry is logged and skipped. `only` restricts the
/// targets (empty: all of them).
///
/// # Errors
/// [`HarnessError::Input`] when `only` names a target the manifest does not have, or
/// when a target's chosen entry has no readable corpus or no vector of its first
/// combination (a drivable target without a corpus is never skipped silently).
pub fn select_sample(
    corpus_root: &Path,
    manifest: &Manifest,
    drops: &AdapterDropLog,
    only: &[String],
) -> Result<Vec<SampleChoice>, HarnessError> {
    if let Some(unknown) = only.iter().find(|id| !manifest.target.contains_key(*id)) {
        return Err(input(
            "targets",
            format!("the manifest has no target {unknown}"),
        ));
    }
    let wanted: BTreeSet<&String> = only.iter().collect();
    let mut chosen = Vec::new();
    for (id, target) in &manifest.target {
        if !wanted.is_empty() && !wanted.contains(id) {
            continue;
        }
        let Some(config) = target.default_config() else {
            continue;
        };
        let mut candidates: Vec<_> = target
            .entry
            .iter()
            .filter(|entry| !entry.arg.is_empty() && drops.row(id, &entry.name).is_none())
            .collect();
        candidates.sort_by(|a, b| a.name.cmp(&b.name));
        let Some(entry) = candidates.first() else {
            continue;
        };
        let key = format!("{id}/{}/{}", config.name, entry.name);
        let dir = corpus_root.join(id).join(&config.name).join(&entry.name);
        let corpus = read_corpus(&dir)
            .map_err(|e| input(&key, format!("no corpus of the drivable entry: {e}")))?;
        let adapter =
            Adapter::from_entry(id, &config.name, entry).map_err(|e| input(&key, e.to_string()))?;
        let combos = combinations(&adapter).map_err(|e| input(&key, e.to_string()))?;
        let first = combos
            .first()
            .ok_or_else(|| input(&key, "the entry has no scalar combination"))?;
        let scalars = adapter.scalars_by_name(first);
        let mut best: Option<(usize, String, String)> = None;
        for (sha, vector) in &corpus.vectors {
            let provenance = &corpus.index.vectors[sha];
            if provenance.scalars != scalars {
                continue;
            }
            let length = decode_latin1(&vector.stdin)
                .map_err(|e| input(&key, format!("{sha}: stdin is not Latin-1 text: {e}")))?
                .len();
            let candidate = (length, provenance.seed.clone(), sha.clone());
            if best.as_ref().is_none_or(|current| candidate < *current) {
                best = Some(candidate);
            }
        }
        let (input_len, seed_sha256, vector_sha256) = best.ok_or_else(|| {
            input(
                &key,
                format!("no vector of the first combination {scalars:?}"),
            )
        })?;
        chosen.push(SampleChoice {
            target: id.clone(),
            config: config.name.clone(),
            entry: entry.name.clone(),
            vector_sha256,
            seed_sha256,
            input_len,
        });
    }
    Ok(chosen)
}

/// Write the chosen vectors under `out` as `<target>/<config>/<entry>` corpora of one
/// vector each, with the CI corpus's provenance record. Everything is read and
/// checked first; `out` must be absent or empty, and nothing is written when a read
/// fails.
///
/// # Errors
/// [`HarnessError::Input`] when a source corpus or vector cannot be read, or `out`
/// is not empty; [`HarnessError::Io`] when `out` cannot be created.
pub fn write_sample(
    corpus_root: &Path,
    choices: &[SampleChoice],
    out: &Path,
) -> Result<(), HarnessError> {
    let mut sets: Vec<(std::path::PathBuf, Vec<(Vector, Provenance)>)> = Vec::new();
    for choice in choices {
        let key = format!("{}/{}/{}", choice.target, choice.config, choice.entry);
        let source = corpus_root
            .join(&choice.target)
            .join(&choice.config)
            .join(&choice.entry);
        let corpus = read_corpus(&source).map_err(|e| input(&key, e.to_string()))?;
        let vector = corpus
            .vectors
            .get(&choice.vector_sha256)
            .ok_or_else(|| input(&key, format!("no vector {}", choice.vector_sha256)))?;
        let provenance = corpus
            .index
            .vectors
            .get(&choice.vector_sha256)
            .ok_or_else(|| input(&key, format!("no provenance of {}", choice.vector_sha256)))?;
        sets.push((
            out.join(&choice.target)
                .join(&choice.config)
                .join(&choice.entry),
            vec![(vector.clone(), provenance.clone())],
        ));
    }
    if out.exists() {
        let empty = std::fs::read_dir(out)
            .map_err(|source| HarnessError::Io {
                action: "cannot list",
                path: out.to_path_buf(),
                source,
            })?
            .next()
            .is_none();
        if !empty {
            return Err(input(
                out.display().to_string(),
                "the sample directory is not empty; a sample is written only into an empty one",
            ));
        }
    }
    for (dir, set) in &sets {
        if let Some(parent) = dir.parent() {
            std::fs::create_dir_all(parent).map_err(|source| HarnessError::Io {
                action: "cannot create",
                path: parent.to_path_buf(),
                source,
            })?;
        }
        write_corpus(dir, set).map_err(|e| input(dir.display().to_string(), e.to_string()))?;
    }
    Ok(())
}
