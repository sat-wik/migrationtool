//! Parsers for tool version output (placeholders until the tests drive them in).

use std::collections::BTreeMap;
use std::fmt;

use regex::Regex;

/// Why a version pattern could not be used.
#[derive(Debug, thiserror::Error)]
pub enum VersionError {
    /// The pattern is not a valid (or is an oversized) regex.
    #[error("invalid regex {pattern:?}: {source}")]
    Regex {
        /// The offending pattern.
        pattern: String,
        /// The regex error.
        #[source]
        source: regex::Error,
    },
    /// The pattern lacks the required named capture group.
    #[error("regex {pattern:?} has no `{group}` named group")]
    MissingGroup {
        /// The offending pattern.
        pattern: String,
        /// The group that is required.
        group: String,
    },
}

/// An LLVM version triple.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LlvmVersion {
    /// Major component.
    pub major: u32,
    /// Minor component.
    pub minor: u32,
    /// Patch component.
    pub patch: u32,
}

impl fmt::Display for LlvmVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// What `rustc -vV` reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustcVersion {
    /// The `release:` value.
    pub release: String,
    /// The `commit-hash:` value.
    pub commit_hash: String,
    /// The `LLVM version:` value, when the line is present.
    pub llvm: Option<LlvmVersion>,
}

/// One `dpkg-query` row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DpkgPackage {
    /// Package name.
    pub name: String,
    /// Debian version string.
    pub version: String,
    /// `db:Status-Status` value, such as `installed`.
    pub status: String,
}

/// Compile `pattern`, requiring the named group `required_group`.
///
/// # Errors
/// [`VersionError`] for an invalid pattern or a missing group.
pub fn compile_pattern(pattern: &str, required_group: &str) -> Result<Regex, VersionError> {
    Err(VersionError::MissingGroup {
        pattern: pattern.to_owned(),
        group: required_group.to_owned(),
    })
}

/// The text of named group `group` in the first match of `re` in `text`.
#[must_use]
pub fn extract(_re: &Regex, _group: &str, _text: &str) -> Option<String> {
    None
}

/// Parse `16.0.5`.
#[must_use]
pub fn parse_llvm_version(_text: &str) -> Option<LlvmVersion> {
    None
}

/// Parse the output of `rustc -vV`.
#[must_use]
pub fn parse_rustc_vv(_text: &str) -> Option<RustcVersion> {
    None
}

/// The first dot component of a version, such as 16 for `16.0.6`.
#[must_use]
pub fn llvm_major_from_version(_version: &str) -> Option<u32> {
    None
}

/// Parse tab-separated `Package`, `Version`, `db:Status-Status` lines.
#[must_use]
pub fn parse_dpkg_query(_text: &str) -> Vec<DpkgPackage> {
    Vec::new()
}

/// The LLVM major of every installed llvm/clang-family package.
#[must_use]
pub fn llvm_family_majors(_packages: &[DpkgPackage]) -> BTreeMap<String, u32> {
    BTreeMap::new()
}

/// A Debian version without its epoch and Debian revision.
#[must_use]
pub fn upstream_version(version: &str) -> &str {
    version
}
