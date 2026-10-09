//! Parsers for tool version output.
//!
//! Version text is untrusted input (threat T-01-10), so patterns are compiled
//! with a size limit and run by the linear-time `regex` engine, and every
//! parser returns `None` rather than guessing. Parsing works on the fields a
//! tool reports, never on a raw dump: `klee --version`, for example, ends with
//! a `Host CPU:` line that varies by machine and is ignored.

use std::collections::BTreeMap;
use std::fmt;

use regex::{Regex, RegexBuilder};

/// Compiled-regex size limit: 1 MiB.
const REGEX_SIZE_LIMIT: usize = 1024 * 1024;

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
/// [`VersionError::Regex`] for an invalid pattern or one whose compiled form
/// exceeds 1 MiB; [`VersionError::MissingGroup`] when the group is absent.
pub fn compile_pattern(pattern: &str, required_group: &str) -> Result<Regex, VersionError> {
    let regex = RegexBuilder::new(pattern)
        .size_limit(REGEX_SIZE_LIMIT)
        .build()
        .map_err(|source| VersionError::Regex {
            pattern: pattern.to_owned(),
            source,
        })?;
    if regex
        .capture_names()
        .flatten()
        .any(|name| name == required_group)
    {
        Ok(regex)
    } else {
        Err(VersionError::MissingGroup {
            pattern: pattern.to_owned(),
            group: required_group.to_owned(),
        })
    }
}

/// The text of named group `group` in the first match of `re` in `text`.
#[must_use]
pub fn extract(re: &Regex, group: &str, text: &str) -> Option<String> {
    re.captures(text)
        .and_then(|caps| caps.name(group))
        .map(|m| m.as_str().to_owned())
}

/// Parse an `X.Y.Z` triple such as `16.0.5`; anything else (a suffix, fewer or
/// more components) is `None`.
#[must_use]
pub fn parse_llvm_version(text: &str) -> Option<LlvmVersion> {
    let mut parts = text.trim().split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some(LlvmVersion {
        major,
        minor,
        patch,
    })
}

/// Parse the output of `rustc -vV`.
///
/// `release:` and `commit-hash:` are required. A missing `LLVM version:` line
/// gives `llvm: None`; a present but unparseable one rejects the whole text.
#[must_use]
pub fn parse_rustc_vv(text: &str) -> Option<RustcVersion> {
    let mut release = None;
    let mut commit_hash = None;
    let mut llvm = None;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("release:") {
            release = Some(value.trim().to_owned());
        } else if let Some(value) = line.strip_prefix("commit-hash:") {
            commit_hash = Some(value.trim().to_owned());
        } else if let Some(value) = line.strip_prefix("LLVM version:") {
            llvm = Some(parse_llvm_version(value)?);
        }
    }
    Some(RustcVersion {
        release: release.filter(|r| !r.is_empty())?,
        commit_hash: commit_hash.filter(|c| !c.is_empty())?,
        llvm,
    })
}

/// The first dot component of a version, such as 16 for `16.0.6`.
#[must_use]
pub fn llvm_major_from_version(version: &str) -> Option<u32> {
    version.trim().split('.').next()?.parse().ok()
}

/// Parse tab-separated `Package`, `Version`, `db:Status-Status` lines. Lines
/// with fewer than three fields are skipped.
#[must_use]
pub fn parse_dpkg_query(text: &str) -> Vec<DpkgPackage> {
    text.lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let name = fields.next()?.trim();
            let version = fields.next()?.trim();
            let status = fields.next()?.trim();
            if name.is_empty() {
                return None;
            }
            Some(DpkgPackage {
                name: name.to_owned(),
                version: version.to_owned(),
                status: status.to_owned(),
            })
        })
        .collect()
}

/// The LLVM major of every installed llvm/clang-family package.
///
/// A package belongs to the family when its name starts with `llvm`, `clang`,
/// `libllvm` or `libclang`; its major is the last run of two or more digits in
/// the name (`libclang1-16` is 16, `libclang-cpp16` is 16). Packages that are
/// not installed, and family names without such a run (`llvm-runtime`), are
/// skipped. Names with a trailing digit run after the major, such as the
/// `libllvm16t64` of later Debian releases, are not recognised.
#[must_use]
pub fn llvm_family_majors(packages: &[DpkgPackage]) -> BTreeMap<String, u32> {
    const FAMILY_PREFIXES: [&str; 4] = ["llvm", "clang", "libllvm", "libclang"];
    packages
        .iter()
        .filter(|p| p.status == "installed")
        .filter(|p| {
            FAMILY_PREFIXES
                .iter()
                .any(|prefix| p.name.starts_with(prefix))
        })
        .filter_map(|p| last_digit_run(&p.name).map(|major| (p.name.clone(), major)))
        .collect()
}

/// The last run of two or more ASCII digits in `name`, as a number.
fn last_digit_run(name: &str) -> Option<u32> {
    name.split(|c: char| !c.is_ascii_digit())
        .rfind(|run| run.len() >= 2)
        .and_then(|run| run.parse().ok())
}

/// A Debian version without its epoch (`N:`) and Debian revision (after the last `-`).
#[must_use]
pub fn upstream_version(version: &str) -> &str {
    let without_epoch = match version.split_once(':') {
        Some((epoch, rest)) if !epoch.is_empty() && epoch.bytes().all(|b| b.is_ascii_digit()) => {
            rest
        }
        _ => version,
    };
    without_epoch
        .rsplit_once('-')
        .map_or(without_epoch, |(upstream, _revision)| upstream)
}
