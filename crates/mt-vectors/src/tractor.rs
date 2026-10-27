//! The pinned TRACTOR sample (D-23): the pin file and the check that loads it.
//!
//! The pin records only the repository, one commit, a list of file paths and
//! the sha256 of each file. The files themselves are fetched into the work
//! directory and never enter the repository. [`check_tree`] verifies every
//! file's sha256 against the pin, then loads it at the TRACTOR-compatible level
//! and requires the re-serialized value to equal the parsed one.
//!
//! This module reads files and nothing else: the fetch is a process and lives in
//! `mt-capture`.

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::vector::{MAX_VECTOR_BYTES, TractorVector};

/// The only pin file `schema_version` this crate understands.
pub const TRACTOR_PIN_SCHEMA_VERSION: u32 = 1;

/// At least this many pinned files must load unchanged.
pub const MIN_TRACTOR_LOADED: usize = 3;

/// Why a pin cannot be used or a tree cannot be checked.
#[derive(Debug, thiserror::Error)]
pub enum TractorError {
    /// The pin file is not valid TOML of the pin's shape.
    #[error("not a valid TRACTOR pin: {0}")]
    Parse(String),
    /// The pin breaks a rule; every issue is listed.
    #[error("invalid TRACTOR pin: {}", .issues.join("; "))]
    Invalid {
        /// Every rule that was broken.
        issues: Vec<String>,
    },
    /// A file or directory could not be read.
    #[error("cannot read {}: {source}", path.display())]
    Io {
        /// The path.
        path: PathBuf,
        /// The cause.
        #[source]
        source: io::Error,
    },
    /// The tree directory does not exist.
    #[error("{} does not exist; run `mt vectors tractor fetch` first", .0.display())]
    NoTree(PathBuf),
}

/// One pinned file.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PinnedFile {
    /// The path inside the TRACTOR repository.
    pub path: String,
    /// The sha256 of the file's bytes, lowercase hex.
    pub sha256: String,
}

/// `testbed/tractor-pin.toml`: where the sample comes from, and nothing of it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TractorPin {
    /// Always [`TRACTOR_PIN_SCHEMA_VERSION`].
    pub schema_version: u32,
    /// The repository URL (https).
    pub repo: String,
    /// The pinned commit, 40 lowercase hex digits.
    pub commit: String,
    /// The licence status as found; the sample is local use only.
    pub licence_status: String,
    /// The pinned files, in the order they are checked.
    pub file: Vec<PinnedFile>,
}

fn is_lower_hex(text: &str, length: usize) -> bool {
    text.len() == length && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// A path component made of letters, digits, `.`, `_` and `-`, not starting
/// with `.` or `-`.
fn is_safe_component(component: &str) -> bool {
    !component.is_empty()
        && !component.starts_with(['.', '-'])
        && component
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

impl TractorPin {
    /// Parse and validate pin text.
    pub fn parse(text: &str) -> Result<Self, TractorError> {
        let pin: Self = toml::from_str(text).map_err(|e| TractorError::Parse(e.to_string()))?;
        pin.validate()?;
        Ok(pin)
    }

    /// Read, parse and validate a pin file.
    pub fn load(path: &Path) -> Result<Self, TractorError> {
        let text = fs::read_to_string(path).map_err(|source| TractorError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse(&text)
    }

    /// Check every rule, collecting all issues.
    pub fn validate(&self) -> Result<(), TractorError> {
        let mut issues = Vec::new();
        if self.schema_version != TRACTOR_PIN_SCHEMA_VERSION {
            issues.push(format!(
                "schema_version {} is not {TRACTOR_PIN_SCHEMA_VERSION}",
                self.schema_version
            ));
        }
        if !self.repo.starts_with("https://")
            || self
                .repo
                .chars()
                .any(|c| c.is_whitespace() || c.is_control())
        {
            issues.push("repo must be an https:// URL without whitespace".to_owned());
        }
        if !is_lower_hex(&self.commit, 40) {
            issues.push("commit must be 40 lowercase hex digits".to_owned());
        }
        if self.licence_status.trim().is_empty() {
            issues.push("licence_status must not be empty".to_owned());
        }
        if self.file.len() < MIN_TRACTOR_LOADED {
            issues.push(format!(
                "at least {MIN_TRACTOR_LOADED} files must be pinned, found {}",
                self.file.len()
            ));
        }
        let mut seen = BTreeSet::new();
        for file in &self.file {
            let well_formed = file.path.strip_prefix("Public-Tests/").is_some_and(|rest| {
                Path::new(&file.path)
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("json"))
                    && rest.split('/').all(is_safe_component)
            });
            if !well_formed {
                issues.push(format!(
                    "path {:?} must be Public-Tests/<safe components>.json",
                    file.path
                ));
            }
            if !is_lower_hex(&file.sha256, 64) {
                issues.push(format!(
                    "sha256 of {:?} must be 64 lowercase hex digits",
                    file.path
                ));
            }
            if !seen.insert(file.path.as_str()) {
                issues.push(format!("path {:?} is pinned twice", file.path));
            }
        }
        if issues.is_empty() {
            Ok(())
        } else {
            Err(TractorError::Invalid { issues })
        }
    }

    /// The directory the fetch writes the pinned files into:
    /// `<work>/tractor/<commit>/tree`.
    #[must_use]
    pub fn tree_dir(&self, work_dir: &Path) -> PathBuf {
        work_dir.join("tractor").join(&self.commit).join("tree")
    }

    /// The pinned paths, in order.
    #[must_use]
    pub fn paths(&self) -> Vec<String> {
        self.file.iter().map(|f| f.path.clone()).collect()
    }
}

/// How one pinned file fared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRow {
    /// The pinned path.
    pub path: String,
    /// The row text after the path when the file loaded, or why it did not.
    pub outcome: Result<String, String>,
}

/// The result of checking a tree against a pin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TractorReport {
    /// One row per pinned file, in pin order.
    pub rows: Vec<FileRow>,
    /// How many files verified and loaded unchanged.
    pub loaded: usize,
    /// How many files are pinned.
    pub total: usize,
}

impl TractorReport {
    /// True only when every pinned file verified and loaded and at least
    /// [`MIN_TRACTOR_LOADED`] did.
    #[must_use]
    pub const fn passed(&self) -> bool {
        self.loaded == self.total && self.loaded >= MIN_TRACTOR_LOADED
    }

    /// The report text: a row per file, the `loaded:` line, then `tractor:
    /// passed` or `tractor: failed`.
    #[must_use]
    pub fn render(&self) -> String {
        let mut text = String::new();
        for row in &self.rows {
            match &row.outcome {
                Ok(detail) => text.push_str(&format!("{} {detail}\n", row.path)),
                Err(reason) => text.push_str(&format!("{} FAILED {reason}\n", row.path)),
            }
        }
        text.push_str(&format!(
            "loaded: {} of {} (at least {MIN_TRACTOR_LOADED} required)\n",
            self.loaded, self.total
        ));
        text.push_str(if self.passed() {
            "tractor: passed\n"
        } else {
            "tractor: failed\n"
        });
        text
    }
}

/// Read `relative` below `tree`, refusing a symlink at any component and any
/// file that is not regular or is above the size cap.
fn read_pinned(tree: &Path, relative: &str) -> Result<Vec<u8>, String> {
    let mut current = tree.to_path_buf();
    let parts: Vec<&str> = relative.split('/').collect();
    for (i, part) in parts.iter().enumerate() {
        current.push(part);
        let meta = fs::symlink_metadata(&current).map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                "is missing from the tree".to_owned()
            } else {
                format!("cannot be inspected: {e}")
            }
        })?;
        if meta.file_type().is_symlink() {
            return Err(format!(
                "is reached through a symlink at {}",
                current.display()
            ));
        }
        let last = i + 1 == parts.len();
        if last && !meta.is_file() {
            return Err("is not a regular file".to_owned());
        }
        if !last && !meta.is_dir() {
            return Err(format!(
                "is below {}, which is not a directory",
                current.display()
            ));
        }
        if last && meta.len() > u64::try_from(MAX_VECTOR_BYTES).unwrap_or(u64::MAX) {
            return Err(format!("is {} bytes, above the size limit", meta.len()));
        }
    }
    fs::read(&current).map_err(|e| format!("cannot be read: {e}"))
}

/// Check every pinned file of `tree`: its sha256 against the pin, then its
/// TRACTOR-level load and JSON-equal re-serialization.
///
/// A file that fails is a failed row, not an error; the other files are still
/// checked. An error means the tree itself is missing.
pub fn check_tree(pin: &TractorPin, tree: &Path) -> Result<TractorReport, TractorError> {
    match fs::metadata(tree) {
        Ok(meta) if meta.is_dir() => {}
        Ok(_) => return Err(TractorError::NoTree(tree.to_path_buf())),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Err(TractorError::NoTree(tree.to_path_buf()));
        }
        Err(source) => {
            return Err(TractorError::Io {
                path: tree.to_path_buf(),
                source,
            });
        }
    }
    let mut rows = Vec::new();
    let mut loaded = 0;
    for file in &pin.file {
        let outcome = read_pinned(tree, &file.path).and_then(|bytes| {
            let found = crate::sha256_hex(&bytes);
            if found != file.sha256 {
                return Err(format!(
                    "sha256 mismatch: the pin has {}, the file has {found}",
                    file.sha256
                ));
            }
            let (_vector, parsed) = TractorVector::load_unchanged(&bytes)
                .map_err(|e| format!("does not load at the TRACTOR level: {e}"))?;
            let keys: Vec<&str> = parsed
                .as_object()
                .map(|object| object.keys().map(String::as_str).collect())
                .unwrap_or_default();
            let has_ub = if keys.contains(&"has_ub") {
                "present"
            } else {
                "absent"
            };
            Ok(format!(
                "sha256=ok keys={} round_trip=json-equal has_ub={has_ub}",
                keys.join(",")
            ))
        });
        if outcome.is_ok() {
            loaded += 1;
        }
        rows.push(FileRow {
            path: file.path.clone(),
            outcome,
        });
    }
    Ok(TractorReport {
        total: rows.len(),
        rows,
        loaded,
    })
}
