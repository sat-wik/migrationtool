//! The determinism comparison of two capture output trees (review warning WR-05).
//!
//! The workflow step "Capture again and compare the reports (determinism)" used
//! `jq -S ... | cmp -s`, which says "identical" when jq fails on both files: two
//! empty outputs compare equal. This module does the comparison in Rust and fails
//! closed. It reads files only: it starts no process and writes nothing.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The complete list of fields left out of the comparison, in words. The reason
/// for both entries was measured in plan 02-06: CMake prints its own elapsed time,
/// and clang's JSON AST dump holds node addresses that change from run to run.
pub const DETERMINISM_EXCLUSIONS: [&str; 2] = [
    "stdout sha256, bytes_total and bytes_stored of the step \"cmake configure\"",
    "stdout sha256, bytes_total and bytes_stored of the steps \"clang ast <function>\"",
];

/// The verdict of comparing two capture output trees.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirComparison {
    /// How many regular files present in both trees were examined.
    pub compared: usize,
    /// Every way the two trees differ, one line each.
    pub problems: Vec<String>,
}

impl DirComparison {
    /// True only when at least one file was compared and nothing differs.
    #[must_use]
    pub fn passed(&self) -> bool {
        self.compared > 0 && self.problems.is_empty()
    }
}

/// A tree that cannot be compared at all (as opposed to a difference found).
#[derive(Debug, thiserror::Error)]
pub enum CompareError {
    /// A top directory, or a directory below it, cannot be read.
    #[error("cannot read the capture tree {}: {source}", path.display())]
    Unreadable {
        /// The directory that failed.
        path: PathBuf,
        /// The operating system error.
        source: std::io::Error,
    },
}

/// Compare the capture output trees `first` and `second`.
///
/// # Errors
/// [`CompareError::Unreadable`] when a directory of either tree cannot be read.
pub fn compare_capture_dirs(first: &Path, second: &Path) -> Result<DirComparison, CompareError> {
    let (left, right) = (list_files(first)?, list_files(second)?);
    let mut result = DirComparison {
        compared: 0,
        problems: Vec::new(),
    };
    for rel in left.intersection(&right) {
        result.compared += 1;
        compare_file(first, second, rel, &mut result.problems);
    }
    Ok(result)
}

/// The relative paths of the regular files below `top`, symlinks not followed.
fn list_files(top: &Path) -> Result<BTreeSet<PathBuf>, CompareError> {
    let mut files = BTreeSet::new();
    let mut pending = vec![PathBuf::new()];
    while let Some(rel) = pending.pop() {
        let dir = top.join(&rel);
        let unreadable = |source| CompareError::Unreadable {
            path: dir.clone(),
            source,
        };
        for entry in std::fs::read_dir(&dir).map_err(unreadable)? {
            let entry = entry.map_err(unreadable)?;
            let kind = entry.file_type().map_err(unreadable)?;
            let child = rel.join(entry.file_name());
            if kind.is_dir() {
                pending.push(child);
            } else if kind.is_file() {
                files.insert(child);
            }
        }
    }
    Ok(files)
}

fn compare_file(first: &Path, second: &Path, rel: &Path, problems: &mut Vec<String>) {
    let shown = rel.display();
    let (Ok(left), Ok(right)) = (
        std::fs::read(first.join(rel)),
        std::fs::read(second.join(rel)),
    ) else {
        problems.push(format!("cannot read {shown}"));
        return;
    };
    if rel.extension().is_some_and(|ext| ext == "json") {
        let parse = |side: &str, bytes: &[u8]| {
            serde_json::from_slice::<serde_json::Value>(bytes)
                .map_err(|error| problems_line(side, rel, &error))
        };
        match (parse("first", &left), parse("second", &right)) {
            (Ok(a), Ok(b)) => {
                if a != b {
                    problems.push(format!("differs: {shown}"));
                }
            }
            (a, b) => {
                for failure in [a.err(), b.err()].into_iter().flatten() {
                    problems.push(failure);
                }
            }
        }
    } else if left != right {
        problems.push(format!("differs byte for byte: {shown}"));
    }
}

fn problems_line(side: &str, rel: &Path, error: &serde_json::Error) -> String {
    format!("cannot parse {side}/{}: {error}", rel.display())
}
