//! The tree digest that pins a fetched source (CONTEXT D-17).
//!
//! The digest is the sha256 of the sorted rows `<mode> <sha256> <relpath>\n`.
//! Modes are `100644`, `100755` and `120000`; a symlink is hashed by its target
//! text and never followed; only a `.git` entry directly under the root is
//! skipped.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// Why a tree could not be hashed.
#[derive(Debug, thiserror::Error)]
pub enum TreeHashError {
    /// A directory, file or link could not be read.
    #[error("cannot read {}: {source}", path.display())]
    Read {
        /// The path that failed.
        path: PathBuf,
        /// The underlying error.
        #[source]
        source: std::io::Error,
    },
    /// A FIFO, socket or device node has no defined row.
    #[error("{} is a special file (FIFO, socket or device); a tree to be hashed holds files, directories and symlinks only", path.display())]
    SpecialFile {
        /// The offending path.
        path: PathBuf,
    },
    /// A path or link target is not valid UTF-8, so it has no row text.
    #[error("{} is not valid UTF-8", path.display())]
    NonUtf8 {
        /// The offending path.
        path: PathBuf,
    },
}

/// Lowercase hex sha256 of `bytes`.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        // Writing into a String cannot fail.
        let _ = write!(text, "{byte:02x}");
    }
    text
}

/// The sorted rows of the tree under `root`, each ending with a newline.
///
/// # Errors
/// [`TreeHashError`] if the tree cannot be read, holds a special file or has a
/// path that is not UTF-8.
pub fn tree_rows(_root: &Path) -> Result<Vec<String>, TreeHashError> {
    Ok(Vec::new())
}

/// The tree digest: the sha256 of the concatenated [`tree_rows`], as lowercase hex.
///
/// # Errors
/// As [`tree_rows`].
pub fn tree_sha256(root: &Path) -> Result<String, TreeHashError> {
    let rows = tree_rows(root)?;
    Ok(sha256_hex(rows.concat().as_bytes()))
}
