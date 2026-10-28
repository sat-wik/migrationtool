//! Comparing stored corpora (CONTEXT D-21).
//!
//! [`check_corpora`] compares two corpus trees file by file and fails closed: a
//! file on one side only, an unreadable file, a file that is not a regular file
//! and a differing byte are all problems, and so is a comparison of no file at
//! all.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use mt_capture::manifest::{Manifest, Role};
use mt_toolchain::runner::RunnerConfig;
use mt_vectors::bytes::decode_latin1;
use mt_vectors::corpus::CorpusIndex;
use mt_vectors::vector::Vector;

use crate::HarnessError;
use crate::adapter::{Adapter, Combination};
use crate::budget::Budgets;
use crate::run::{Exec, Scope, load_driver, run_vector_twice};
use crate::tools::ExecTools;

/// The result of comparing two corpus trees.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusComparison {
    /// The number of files that were present on both sides and identical.
    pub files: usize,
    /// Every problem found, sorted by path; empty when the trees are identical.
    pub problems: Vec<String>,
}

impl CorpusComparison {
    /// True when at least one file was compared and nothing differed.
    #[must_use]
    pub fn identical(&self) -> bool {
        self.problems.is_empty() && self.files > 0
    }

    /// The first problem, or the reason nothing was compared.
    #[must_use]
    pub fn first_difference(&self) -> Option<String> {
        self.problems
            .first()
            .cloned()
            .or_else(|| (self.files == 0).then(|| "no file was compared".to_owned()))
    }
}

fn io(action: &'static str, path: &Path, source: std::io::Error) -> HarnessError {
    HarnessError::Io {
        action,
        path: path.to_path_buf(),
        source,
    }
}

/// Every regular file under `root`, by relative path, with its bytes; anything
/// else (a symbolic link, a socket) is recorded in `problems`.
fn read_tree(
    root: &Path,
    side: &str,
    problems: &mut Vec<String>,
) -> Result<BTreeMap<String, Vec<u8>>, HarnessError> {
    let mut files = BTreeMap::new();
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let listing = std::fs::read_dir(&dir).map_err(|e| io("cannot list", &dir, e))?;
        for entry in listing {
            let entry = entry.map_err(|e| io("cannot list", &dir, e))?;
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .map_or_else(|_| path.display().to_string(), |r| r.display().to_string());
            let kind = entry.file_type().map_err(|e| io("cannot stat", &path, e))?;
            if kind.is_dir() {
                stack.push(path);
            } else if kind.is_file() {
                match std::fs::read(&path) {
                    Ok(bytes) => {
                        files.insert(relative, bytes);
                    }
                    Err(error) => {
                        problems.push(format!("{relative}: unreadable in {side}: {error}"))
                    }
                }
            } else {
                problems.push(format!("{relative}: not a regular file in {side}"));
            }
        }
    }
    Ok(files)
}

/// Compare the corpus trees `first` and `second`.
///
/// # Errors
/// [`HarnessError::Io`] when a tree's root cannot be listed.
pub fn check_corpora(first: &Path, second: &Path) -> Result<CorpusComparison, HarnessError> {
    let mut problems = Vec::new();
    let left = read_tree(first, "the first tree", &mut problems)?;
    let right = read_tree(second, "the second tree", &mut problems)?;
    let mut files = 0usize;
    for (path, bytes) in &left {
        match right.get(path) {
            None => problems.push(format!("{path}: missing from the second tree")),
            Some(other) if other == bytes => files += 1,
            Some(_) => problems.push(format!("{path}: differs")),
        }
    }
    for path in right.keys() {
        if !left.contains_key(path) {
            problems.push(format!("{path}: missing from the first tree"));
        }
    }
    problems.sort();
    Ok(CorpusComparison { files, problems })
}

// ---------------------------------------------------------------------------
// Replaying a stored corpus
// ---------------------------------------------------------------------------

/// What a replay needs.
#[derive(Debug, Clone)]
pub struct ReplayOptions {
    /// The manifest and the build root: the driver of the corpus's entry is
    /// loaded from `<build root>/<target>/<config>/<entry>`.
    pub scope: Scope,
    /// The pinned tools.
    pub tools: ExecTools,
    /// The runner configuration.
    pub cfg: RunnerConfig,
    /// Where the replay's scratch slots go.
    pub scratch_dir: PathBuf,
}

/// The result of replaying one entry's corpus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayReport {
    /// The corpus directory.
    pub dir: PathBuf,
    /// The vectors the index lists.
    pub vectors: usize,
    /// Vectors that were re-run and reproduced byte for byte.
    pub identical: usize,
    /// sha256 of each vector that did not reproduce (or could not be read or
    /// re-run), in index order.
    pub differing: Vec<String>,
    /// What differs between a provenance record and the current build, as
    /// `<sha256> <field>`.
    pub provenance_mismatch: Vec<String>,
}

impl ReplayReport {
    /// True when every vector reproduced and every provenance record names the
    /// current build.
    #[must_use]
    pub fn passed(&self) -> bool {
        self.vectors > 0 && self.differing.is_empty() && self.provenance_mismatch.is_empty()
    }

    /// The row: `<dir> identical <n>`, `<dir> differs <sha256>` for the first
    /// vector that did not reproduce, or `<dir> provenance_mismatch (<sha256>
    /// <field>)`.
    #[must_use]
    pub fn render(&self) -> String {
        let dir = self.dir.display();
        if let Some(first) = self.differing.first() {
            format!("{dir} differs {first}")
        } else if let Some(first) = self.provenance_mismatch.first() {
            format!("{dir} provenance_mismatch ({first})")
        } else if self.vectors == 0 {
            format!("{dir} empty")
        } else {
            format!("{dir} identical {}", self.identical)
        }
    }
}

fn input(what: impl Into<String>, detail: impl Into<String>) -> HarnessError {
    HarnessError::Input {
        what: what.into(),
        detail: detail.into(),
    }
}

/// The combination a stored vector was made with: `argv[1..]` are the
/// non-pointer parameters in order, the input length first where there is an
/// `in_len`.
fn combination_of(
    adapter: &Adapter,
    vector: &Vector,
    seed_len: usize,
) -> Result<Combination, String> {
    let mut texts = vector.argv.iter().skip(1);
    let mut values = Vec::new();
    for param in adapter.params() {
        match param.role {
            Role::InLen | Role::Wrap => {
                let text = texts.next().ok_or("argv is shorter than the parameters")?;
                if text != &seed_len.to_string() {
                    return Err(format!(
                        "argv input length {text} is not the stdin length {seed_len}"
                    ));
                }
            }
            Role::OutCap | Role::Scalar | Role::InoutLen => {
                let text = texts.next().ok_or("argv is shorter than the parameters")?;
                values.push(
                    text.parse::<i64>()
                        .map_err(|_| format!("argv value {text:?} is not a decimal integer"))?,
                );
            }
            Role::Str => {
                let text = texts.next().ok_or("argv is shorter than the parameters")?;
                let position = param
                    .values
                    .iter()
                    .position(|value| value == text)
                    .ok_or_else(|| format!("argv string {text:?} is not a declared value"))?;
                values.push(i64::try_from(position).map_err(|_| "too many values".to_owned())?);
            }
            Role::StrLen => {
                let text = texts.next().ok_or("argv is shorter than the parameters")?;
                text.parse::<usize>()
                    .map_err(|_| format!("argv length {text:?} is not a decimal integer"))?;
            }
            Role::InBuf | Role::OutBuf | Role::State | Role::OutScalar | Role::OutPtr => {}
        }
    }
    if texts.next().is_some() {
        return Err("argv is longer than the parameters".to_owned());
    }
    Ok(Combination { values })
}

/// Re-run every vector of the corpus in `dir` (`<corpus>/<target>/<config>/
/// <entry>`) twice through the current build and compare the canonical bytes
/// with the stored file, then compare each provenance record's driver, archive
/// and object hashes with `build.json` (CONTEXT D-21; threat T-03-17). A stored
/// file that was edited, cannot be read, or does not parse counts as differing;
/// the corpus is read file by file so one bad file does not hide the rest.
///
/// # Errors
/// [`HarnessError`] when the index cannot be read, the entry's driver is not
/// built, or a tool cannot run.
pub fn replay_corpus(options: &ReplayOptions, dir: &Path) -> Result<ReplayReport, HarnessError> {
    let index_path = dir.join("index.json");
    let index: CorpusIndex = serde_json::from_slice(
        &std::fs::read(&index_path).map_err(|e| io("cannot read", &index_path, e))?,
    )
    .map_err(|e| input(index_path.display().to_string(), e.to_string()))?;
    let manifest = Manifest::load(&options.scope.manifest_path).map_err(|e| {
        input(
            options.scope.manifest_path.display().to_string(),
            e.to_string(),
        )
    })?;
    let entry = manifest
        .target
        .get(&index.target)
        .and_then(|target| target.entry.iter().find(|entry| entry.name == index.entry))
        .ok_or_else(|| {
            input(
                index_path.display().to_string(),
                format!("the manifest has no entry {}/{}", index.target, index.entry),
            )
        })?;
    let loaded =
        load_driver(&options.scope, &index.target, &index.config, entry).map_err(|state| {
            input(
                format!("{}/{}/{}", index.target, index.config, index.entry),
                format!("the driver cannot be loaded: {state:?}"),
            )
        })?;
    let budgets = Budgets::DEFAULT;
    let exec = Exec {
        tools: &options.tools,
        cfg: &options.cfg,
        budgets: &budgets,
    };
    let slot_dir = options.scratch_dir.join("replay");
    let mut report = ReplayReport {
        dir: dir.to_path_buf(),
        vectors: index.vectors.len(),
        identical: 0,
        differing: Vec::new(),
        provenance_mismatch: Vec::new(),
    };
    for (sha, provenance) in &index.vectors {
        for (field, same) in [
            (
                "driver_elf",
                provenance.driver_elf == loaded.build.driver_elf_sha256,
            ),
            (
                "driver_source",
                provenance.driver_source == loaded.build.driver_source_sha256,
            ),
            ("archive", provenance.archive == loaded.build.archive_sha256),
            ("objects", provenance.objects == loaded.build.objects),
        ] {
            if !same {
                report.provenance_mismatch.push(format!("{sha} {field}"));
            }
        }
        let path = dir.join(format!("{sha}.json"));
        let Ok(stored) = std::fs::read(&path) else {
            report.differing.push(sha.clone());
            continue;
        };
        let Ok(vector) = Vector::from_json_bytes(&stored) else {
            report.differing.push(sha.clone());
            continue;
        };
        let Ok(seed) = decode_latin1(&vector.stdin) else {
            report.differing.push(sha.clone());
            continue;
        };
        let Ok(combination) = combination_of(&loaded.adapter, &vector, seed.len()) else {
            report.differing.push(sha.clone());
            continue;
        };
        let observed = run_vector_twice(
            &exec,
            &loaded.adapter,
            &loaded.driver,
            &slot_dir,
            &seed,
            &combination,
        )?;
        let reproduced = vector.argv.first() == Some(&loaded.adapter.argv0())
            && observed.is_ok_and(|done| {
                done.vector
                    .to_canonical_json()
                    .is_ok_and(|bytes| bytes == stored)
            });
        if reproduced {
            report.identical += 1;
        } else {
            report.differing.push(sha.clone());
        }
    }
    Ok(report)
}
