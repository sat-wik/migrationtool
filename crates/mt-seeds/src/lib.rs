#![forbid(unsafe_code)]
//! Seed corpus extraction for migrationtool (BUILD-03).
//!
//! Test vectors are buried in the target's own test sources, fixtures and fuzz
//! corpora (D-13). This crate pulls candidate seeds out of them: [`lexer`]
//! reads constant byte arrays and string literals from C and C++ sources,
//! [`hex`] decodes hex-text fixtures and [`zipsrc`] reads zip members in
//! memory. [`select`] turns any set of candidates into a capped, deduplicated,
//! deterministic selection (D-16), and [`index`] stores that selection as one
//! file per seed named by its sha256 plus an `index.json` (D-15), or checks a
//! stored corpus against a fresh selection.
//!
//! The crate never starts a process and never reads the network; it works on
//! bytes handed to it by the caller and writes only inside the corpus
//! directory it is given.

pub mod hex;
pub mod index;
pub mod lexer;
pub mod select;
pub mod zipsrc;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// How a seed was obtained.
///
/// The declaration order is the selection rank: earlier methods are preferred
/// when the corpus is capped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    /// Written by hand (or by a committed script) because the target ships no usable data.
    HandMade,
    /// A fixture file copied verbatim.
    FileCopy,
    /// A hex-text fixture decoded to bytes.
    HexText,
    /// A constant byte array in a C or C++ source.
    CArray,
    /// A string literal in a C or C++ source.
    CString,
    /// A member of a zip archive (for example a fuzz corpus).
    ZipMember,
}

/// Everything that can go wrong while extracting or storing seeds.
#[derive(Debug, thiserror::Error)]
pub enum SeedError {
    /// A hex-text file contained something other than hex digits, whitespace or comments.
    #[error("invalid hex text: {0}")]
    HexText(String),
    /// The bytes are not a readable zip archive.
    #[error("zip archive error: {0}")]
    Zip(#[from] zip::result::ZipError),
    /// A file or directory operation failed.
    #[error("{operation} {path}: {source}")]
    Io {
        /// What was being done.
        operation: &'static str,
        /// The path involved.
        path: PathBuf,
        /// The underlying error.
        #[source]
        source: std::io::Error,
    },
    /// The index could not be serialised.
    #[error("index serialisation failed: {0}")]
    Json(#[from] serde_json::Error),
    /// The index and the selection passed together do not describe the same seeds.
    #[error("index and selection disagree: {0}")]
    Inconsistent(String),
    /// The corpus directory holds a subdirectory, which regeneration will not delete.
    #[error("corpus directory contains a subdirectory: {0}")]
    UnexpectedDirectory(PathBuf),
}

/// Lower-case hex sha256 of `bytes`.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let digest = Sha256::digest(bytes);
    let mut text = String::with_capacity(digest.len() * 2);
    for byte in digest.iter() {
        // Writing into a String cannot fail.
        let _ = write!(text, "{byte:02x}");
    }
    text
}
