//! Comparing stored corpora (CONTEXT D-21).
//!
//! [`check_corpora`] compares two corpus trees file by file and fails closed: a
//! file on one side only, an unreadable file, a file that is not a regular file
//! and a differing byte are all problems, and so is a comparison of no file at
//! all.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::HarnessError;

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
