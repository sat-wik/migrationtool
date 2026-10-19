//! The determinism comparison of two capture output trees (review warning WR-05).
//!
//! The workflow step "Capture again and compare the reports (determinism)" used
//! `jq -S ... | cmp -s`, which says "identical" when jq fails on both files: two
//! empty outputs compare equal, and an empty file was never flagged. This module
//! does the comparison in Rust and fails closed: a report that cannot be parsed on
//! either side, an empty file, a file present on one side only, a file that is not
//! a regular file, and a comparison that examined no file at all are differences.
//!
//! Compared as the old step compared them:
//!
//! - `*.compile_commands.json` files byte for byte, with no exclusion;
//! - every other `*.json` file as parsed JSON (key order does not matter, array
//!   order does), after the exclusions of [`DETERMINISM_EXCLUSIONS`] when the
//!   report's `kind` is `"capture"`;
//! - any other file byte for byte.
//!
//! The module reads files only: it starts no process and writes nothing.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde_json::Value;

/// The complete list of fields left out of the comparison, in words. Both entries
/// were measured in plan 02-06: CMake prints its own elapsed time, and clang's JSON
/// AST dump holds node addresses that change from run to run. The code that applies
/// them is [`strip_exclusions`]; nothing else is excluded.
pub const DETERMINISM_EXCLUSIONS: [&str; 2] = [
    "stdout sha256, bytes_total and bytes_stored of the step \"cmake configure\"",
    "stdout sha256, bytes_total and bytes_stored of the steps \"clang ast <function>\"",
];

/// The name of the step whose output carries CMake's elapsed time.
const CONFIGURE_STEP: &str = "cmake configure";
/// The name prefix of the steps whose output carries clang's AST node addresses.
const AST_STEP_PREFIX: &str = "clang ast ";
/// The three fields of a step's stdout that are left out for those steps.
const EXCLUDED_STDOUT_FIELDS: [&str; 3] = ["sha256", "bytes_total", "bytes_stored"];

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

/// What a directory entry is, symlinks not followed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Entry {
    Regular,
    /// A symlink, socket, device or other non-regular entry.
    Other,
}

/// Compare the capture output trees `first` and `second`.
///
/// # Errors
/// [`CompareError::Unreadable`] when a directory of either tree cannot be read.
pub fn compare_capture_dirs(first: &Path, second: &Path) -> Result<DirComparison, CompareError> {
    let (left, right) = (list_entries(first)?, list_entries(second)?);
    let paths: BTreeSet<&PathBuf> = left.keys().chain(right.keys()).collect();
    let mut result = DirComparison {
        compared: 0,
        problems: Vec::new(),
    };
    for rel in paths {
        let sides = [
            ("first", first, left.get(rel)),
            ("second", second, right.get(rel)),
        ];
        let shown = rel.display();
        let mut usable = true;
        for (side, _, entry) in sides {
            match entry {
                None => {
                    let other = if side == "first" { "second" } else { "first" };
                    result.problems.push(format!("only in {other}: {shown}"));
                    usable = false;
                }
                Some(Entry::Other) => {
                    result
                        .problems
                        .push(format!("not a regular file: {side}/{shown}"));
                    usable = false;
                }
                Some(Entry::Regular) => {}
            }
        }
        if usable {
            result.compared += 1;
            compare_file(first, second, rel, &mut result.problems);
        }
    }
    Ok(result)
}

/// Every entry below `top` by relative path, directories descended into and
/// symlinks never followed.
fn list_entries(top: &Path) -> Result<BTreeMap<PathBuf, Entry>, CompareError> {
    let mut entries = BTreeMap::new();
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
                entries.insert(child, Entry::Regular);
            } else {
                entries.insert(child, Entry::Other);
            }
        }
    }
    Ok(entries)
}

/// Compare one regular file present in both trees, adding a line to `problems`
/// for every way it differs or cannot be compared.
fn compare_file(first: &Path, second: &Path, rel: &Path, problems: &mut Vec<String>) {
    let shown = rel.display();
    let mut contents = Vec::with_capacity(2);
    for (side, root) in [("first", first), ("second", second)] {
        match std::fs::read(root.join(rel)) {
            Ok(bytes) if bytes.is_empty() => {
                problems.push(format!("empty file: {side}/{shown}"));
            }
            Ok(bytes) => contents.push((side, bytes)),
            Err(error) => problems.push(format!("cannot read {side}/{shown}: {error}")),
        }
    }
    let [(_, left), (_, right)] = contents.as_slice() else {
        return;
    };
    let name = rel
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    if name.ends_with(".json") && !name.ends_with(".compile_commands.json") {
        compare_json(rel, &contents, problems);
    } else if left != right {
        problems.push(format!("differs byte for byte: {shown}"));
    }
}

/// Parse both sides, apply the exclusions to capture reports and compare.
fn compare_json(rel: &Path, contents: &[(&str, Vec<u8>)], problems: &mut Vec<String>) {
    let mut parsed = Vec::with_capacity(2);
    for (side, bytes) in contents {
        match serde_json::from_slice::<Value>(bytes) {
            Ok(mut value) => {
                strip_exclusions(&mut value);
                parsed.push(value);
            }
            Err(error) => problems.push(format!("cannot parse {side}/{}: {error}", rel.display())),
        }
    }
    let [left, right] = parsed.as_slice() else {
        return;
    };
    if let Some(pointer) = first_difference(left, right, &mut String::new()) {
        let place = if pointer.is_empty() {
            "the document root".to_owned()
        } else {
            pointer
        };
        problems.push(format!("differs: {} at {place}", rel.display()));
    }
}

/// Remove the fields of [`DETERMINISM_EXCLUSIONS`] from a capture report. Any
/// other document, and every other field, is left as it is.
fn strip_exclusions(report: &mut Value) {
    if report.get("kind").and_then(Value::as_str) != Some("capture") {
        return;
    }
    let Some(steps) = report.get_mut("steps").and_then(Value::as_array_mut) else {
        return;
    };
    for step in steps {
        let excluded = step
            .get("name")
            .and_then(Value::as_str)
            .is_some_and(|name| name == CONFIGURE_STEP || name.starts_with(AST_STEP_PREFIX));
        if !excluded {
            continue;
        }
        if let Some(stdout) = step
            .get_mut("run")
            .and_then(|run| run.get_mut("stdout"))
            .and_then(Value::as_object_mut)
        {
            for field in EXCLUDED_STDOUT_FIELDS {
                stdout.remove(field);
            }
        }
    }
}

/// The RFC 6901 JSON pointer of the first location where the documents differ:
/// object keys are visited as a sorted set, arrays by position, and a key or
/// element present on one side only is the difference.
fn first_difference(left: &Value, right: &Value, pointer: &mut String) -> Option<String> {
    match (left, right) {
        (Value::Object(a), Value::Object(b)) => {
            let keys: BTreeSet<&String> = a.keys().chain(b.keys()).collect();
            for key in keys {
                let mark = pointer.len();
                pointer.push('/');
                pointer.push_str(&key.replace('~', "~0").replace('/', "~1"));
                match (a.get(key), b.get(key)) {
                    (Some(x), Some(y)) => {
                        if let Some(found) = first_difference(x, y, pointer) {
                            return Some(found);
                        }
                    }
                    _ => return Some(pointer.clone()),
                }
                pointer.truncate(mark);
            }
            None
        }
        (Value::Array(a), Value::Array(b)) => {
            for index in 0..a.len().max(b.len()) {
                let mark = pointer.len();
                pointer.push_str(&format!("/{index}"));
                match (a.get(index), b.get(index)) {
                    (Some(x), Some(y)) => {
                        if let Some(found) = first_difference(x, y, pointer) {
                            return Some(found);
                        }
                    }
                    _ => return Some(pointer.clone()),
                }
                pointer.truncate(mark);
            }
            None
        }
        _ => (left != right).then(|| pointer.clone()),
    }
}
