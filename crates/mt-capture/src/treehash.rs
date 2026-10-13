//! The tree digest that pins a fetched source (CONTEXT D-17).
//!
//! The digest is the sha256 of the sorted rows `<mode> <sha256> <relpath>\n`.
//! Modes are `100644`, `100755` and `120000`; a symlink is hashed by its target
//! text and never followed; only a `.git` entry directly under the root is
//! skipped.

use std::path::{Component, Path, PathBuf};

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

/// Copy the tree under `src` to `dst`, for a replay's clean copy.
///
/// Directories are created, regular files are copied with their mode (so the
/// execute bit survives), and symlinks are recreated as symlinks with the same
/// link text, never followed. Only a `.git` entry directly under `src` is
/// skipped, the same rule as [`tree_rows`]. A link whose target is absolute or
/// whose lexical resolution leaves the tree is refused (RESEARCH Pitfall 16):
/// after the copy every link that can be resolved is checked physically as
/// well, because a chain of links can leave the tree where the text alone does
/// not. `dst` should not exist or should be empty; an existing file is
/// overwritten and an existing link is an error.
///
/// # Errors
/// [`TreeHashError::Read`] for an i/o failure, [`TreeHashError::SpecialFile`]
/// for a FIFO, socket or device, [`TreeHashError::EscapingSymlink`] for a link
/// that leaves the tree.
pub fn copy_tree(src: &Path, dst: &Path) -> Result<(), TreeHashError> {
    std::fs::create_dir_all(dst).map_err(|source| read_error(dst, source))?;
    let mut links: Vec<PathBuf> = Vec::new();
    let mut stack = vec![PathBuf::new()];
    while let Some(relative) = stack.pop() {
        let listing = std::fs::read_dir(src.join(&relative))
            .map_err(|source| read_error(&src.join(&relative), source))?;
        for entry in listing {
            let entry = entry.map_err(|source| read_error(&src.join(&relative), source))?;
            let name = entry.file_name();
            if relative.as_os_str().is_empty() && name == ".git" {
                continue;
            }
            let child = relative.join(&name);
            let from = src.join(&child);
            let to = dst.join(&child);
            let file_type = entry
                .file_type()
                .map_err(|source| read_error(&from, source))?;
            if file_type.is_symlink() {
                let target =
                    std::fs::read_link(&from).map_err(|source| read_error(&from, source))?;
                if !link_stays_in_tree(&relative, &target) {
                    return Err(TreeHashError::EscapingSymlink { path: from, target });
                }
                make_symlink(&target, &to).map_err(|source| read_error(&to, source))?;
                links.push(to);
            } else if file_type.is_dir() {
                std::fs::create_dir(&to).map_err(|source| read_error(&to, source))?;
                stack.push(child);
            } else if file_type.is_file() {
                // `fs::copy` carries the permission bits across.
                std::fs::copy(&from, &to).map_err(|source| read_error(&from, source))?;
            } else {
                return Err(TreeHashError::SpecialFile { path: from });
            }
        }
    }
    check_links_physically(dst, &links)
}

/// True when following `target` from a link inside the directory `parent`
/// (relative to the tree root) never climbs above the root, by text alone.
fn link_stays_in_tree(parent: &Path, target: &Path) -> bool {
    let mut depth = parent.components().count();
    for component in target.components() {
        match component {
            Component::Normal(_) => depth += 1,
            Component::CurDir => {}
            Component::ParentDir => match depth.checked_sub(1) {
                Some(shallower) => depth = shallower,
                None => return false,
            },
            Component::RootDir | Component::Prefix(_) => return false,
        }
    }
    true
}

/// Refuse a copied link that physically resolves outside the copied tree. A
/// dangling or looping link cannot be followed anywhere, so it is left alone.
fn check_links_physically(dst: &Path, links: &[PathBuf]) -> Result<(), TreeHashError> {
    let root = std::fs::canonicalize(dst).map_err(|source| read_error(dst, source))?;
    for link in links {
        if let Ok(resolved) = std::fs::canonicalize(link)
            && !resolved.starts_with(&root)
        {
            let target = std::fs::read_link(link).unwrap_or_default();
            return Err(TreeHashError::EscapingSymlink {
                path: link.clone(),
                target,
            });
        }
    }
    Ok(())
}

#[cfg(unix)]
fn make_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(not(unix))]
fn make_symlink(_target: &Path, _link: &Path) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "symbolic links are only supported on unix hosts",
    ))
}
