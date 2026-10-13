#![forbid(unsafe_code)]
//! Seed corpus extraction for migrationtool (BUILD-03).
//!
//! Test vectors are buried in the target's own test sources, fixtures and fuzz
//! corpora (D-13). This crate pulls candidate seeds out of them: [`lexer`]
//! reads constant byte arrays and string literals from C and C++ sources,
//! [`hex`] decodes hex-text fixtures.
//!
//! The crate never starts a process and never reads the network; it works on
//! bytes handed to it by the caller.

pub mod hex;
pub mod lexer;

use serde::{Deserialize, Serialize};

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
}
