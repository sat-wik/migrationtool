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
    /// A symlink is absolute or resolves outside the tree being copied.
    #[error("symlink {} points to {}, which is absolute or leaves the tree", path.display(), target.display())]
    EscapingSymlink {
        /// The link.
        path: PathBuf,
        /// Its target text.
        target: PathBuf,
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
pub fn tree_rows(root: &Path) -> Result<Vec<String>, TreeHashError> {
    let mut rows: Vec<(String, String)> = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let listing = std::fs::read_dir(&dir).map_err(|source| read_error(&dir, source))?;
        for entry in listing {
            let entry = entry.map_err(|source| read_error(&dir, source))?;
            let path = entry.path();
            // Only a `.git` directly under the root is repository metadata.
            if dir == root && entry.file_name() == ".git" {
                continue;
            }
            // `DirEntry::file_type` does not follow symlinks.
            let file_type = entry
                .file_type()
                .map_err(|source| read_error(&path, source))?;
            let relpath = relative_path(root, &path)?;
            if file_type.is_symlink() {
                let target =
                    std::fs::read_link(&path).map_err(|source| read_error(&path, source))?;
                // The link target text is hashed; the link is never followed.
                let text = target.to_str().ok_or_else(|| TreeHashError::NonUtf8 {
                    path: target.clone(),
                })?;
                rows.push((
                    relpath.clone(),
                    row("120000", &sha256_hex(text.as_bytes()), &relpath),
                ));
            } else if file_type.is_dir() {
                stack.push(path);
            } else if file_type.is_file() {
                let content = std::fs::read(&path).map_err(|source| read_error(&path, source))?;
                let metadata = entry
                    .metadata()
                    .map_err(|source| read_error(&path, source))?;
                let mode = if is_executable(&metadata) {
                    "100755"
                } else {
                    "100644"
                };
                rows.push((relpath.clone(), row(mode, &sha256_hex(&content), &relpath)));
            } else {
                return Err(TreeHashError::SpecialFile { path });
            }
        }
    }
    // Row order is the byte order of the relative path, not of the whole row.
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(rows.into_iter().map(|(_, row)| row).collect())
}

fn row(mode: &str, digest: &str, relpath: &str) -> String {
    format!("{mode} {digest} {relpath}\n")
}

fn read_error(path: &Path, source: std::io::Error) -> TreeHashError {
    TreeHashError::Read {
        path: path.to_path_buf(),
        source,
    }
}

/// The path below `root` with `/` separators.
fn relative_path(root: &Path, path: &Path) -> Result<String, TreeHashError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| TreeHashError::NonUtf8 {
            path: path.to_path_buf(),
        })?;
    let mut parts = Vec::new();
    for component in relative.components() {
        let part = component
            .as_os_str()
            .to_str()
            .ok_or_else(|| TreeHashError::NonUtf8 {
                path: path.to_path_buf(),
            })?;
        parts.push(part);
    }
    Ok(parts.join("/"))
}

#[cfg(unix)]
fn is_executable(metadata: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(_metadata: &std::fs::Metadata) -> bool {
    false
}

/// The tree digest: the sha256 of the concatenated [`tree_rows`], as lowercase hex.
///
/// # Errors
/// As [`tree_rows`].
pub fn tree_sha256(root: &Path) -> Result<String, TreeHashError> {
    let rows = tree_rows(root)?;
    Ok(sha256_hex(rows.concat().as_bytes()))
}

/// Copy the tree under `src` to `dst`.
///
/// Not implemented yet.
///
/// # Errors
/// [`TreeHashError`].
pub fn copy_tree(_src: &Path, _dst: &Path) -> Result<(), TreeHashError> {
    Ok(())
}
