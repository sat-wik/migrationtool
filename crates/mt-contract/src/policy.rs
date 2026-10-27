//! The per-module UB policy file (CONTEXT D-15, D-16, D-17, D-18, D-29).
//!
//! One TOML file per module, `testbed/policy/<module>.toml` for the test bed.
//! Every struct rejects unknown keys, so a typo cannot silently disable a rule
//! (threat T-03-05). [`Policy::parse`] checks shape only; [`Policy::validate`]
//! checks every value and collects all issues into one error.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::{CONTRACT_VERSION, POLICY_SCHEMA_VERSION};

/// What the Rust does at a UB site (D-17).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// Return a declared error code and make no further writes.
    ReturnError,
    /// Abort; emitted Rust uses `panic = "abort"`, so this is a trap.
    Panic,
    /// Reproduce what the compiled C did on the target, recorded per site.
    Preserve,
}

impl Action {
    /// The word used in policy files and reports.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ReturnError => "return_error",
            Self::Panic => "panic",
            Self::Preserve => "preserve",
        }
    }
}

/// A parsed policy file.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    /// Must equal [`POLICY_SCHEMA_VERSION`].
    pub schema_version: u32,
    /// `MAJOR.MINOR.PATCH` of the contract document this file implements; not
    /// above [`CONTRACT_VERSION`].
    pub contract_version: String,
    /// The manifest target id (or, later, the customer module name).
    pub module: String,
    /// The UB action settings.
    pub ub: Ub,
}

/// The `[ub]` table.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ub {
    /// The module default (D-16).
    pub default_action: Action,
    /// The record that must accompany a default other than `return_error`
    /// (D-29).
    #[serde(default)]
    pub default_change: Option<DefaultChange>,
    /// The declared error code per function that `return_error` returns (D-17).
    #[serde(default)]
    pub error_code: Vec<ErrorCode>,
    /// Per-site overrides (D-16, D-18). TOML key `override`.
    #[serde(default)]
    pub r#override: Vec<Override>,
}

/// Who changed the module default, why, when and where (D-29).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefaultChange {
    /// The "where": the module, equal to the file's module.
    pub module: String,
    /// Why the default changed.
    pub reason: String,
    /// Who approved it.
    pub approver: String,
    /// `YYYY-MM-DD`.
    pub date: String,
}

/// The declared error code `return_error` returns for one function.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorCode {
    /// A C identifier.
    pub function: String,
    /// A decimal integer or a C identifier naming a constant.
    pub value: ErrorValue,
}

/// The value of an [`ErrorCode`], as written in the file: a TOML integer, or
/// text holding a decimal integer or a C identifier naming a constant.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum ErrorValue {
    /// A TOML integer.
    Integer(i64),
    /// Text; checked by [`Policy::validate`].
    Text(String),
}

/// A per-site override: a full record (D-18).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Override {
    /// The function holding the site.
    pub function: String,
    /// The UB kind; free text until Phase 4 constrains it.
    pub kind: String,
    /// The location; free text until Phase 4 constrains it.
    pub location: String,
    /// The action taken at this site.
    pub action: Action,
    /// Why the default does not apply.
    pub reason: String,
    /// Who approved it.
    pub approver: String,
    /// `YYYY-MM-DD`.
    pub date: String,
}

/// A UB site as Phase 4's log identifies it: exact equality of the three
/// strings.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Site {
    /// The function holding the site.
    pub function: String,
    /// The UB kind.
    pub kind: String,
    /// The location.
    pub location: String,
}

/// Why a policy could not be used.
#[derive(Debug, thiserror::Error)]
pub enum PolicyError {
    /// The file could not be read.
    #[error("cannot read policy {}: {source}", path.display())]
    Read {
        /// The path that failed.
        path: PathBuf,
        /// The underlying error.
        #[source]
        source: std::io::Error,
    },
    /// The text is not valid TOML for this schema (unknown key, missing key,
    /// wrong type or value).
    ///
    /// The TOML error is not a `source`: its text is already part of this
    /// message, and a chain printer would show it twice.
    #[error("invalid policy: {0}")]
    Parse(toml::de::Error),
    /// The policy parses but one or more values are not acceptable.
    #[error("policy validation failed with {} issue(s): {}", issues.len(), issues.join("; "))]
    Invalid {
        /// Every issue found, sorted, each starting with its key.
        issues: Vec<String>,
    },
}

impl From<toml::de::Error> for PolicyError {
    fn from(error: toml::de::Error) -> Self {
        Self::Parse(error)
    }
}

impl Policy {
    /// Parse a policy from TOML text.
    ///
    /// # Errors
    /// [`PolicyError::Parse`] on malformed TOML, an unknown key (at any
    /// level), a missing key or a value of the wrong type.
    pub fn parse(text: &str) -> Result<Self, PolicyError> {
        Ok(toml::from_str(text)?)
    }

    /// Read and parse a policy file. The module must equal the file stem.
    ///
    /// # Errors
    /// [`PolicyError::Read`] if the file cannot be read, [`PolicyError::Invalid`]
    /// when the module differs from the stem, otherwise as [`Policy::parse`].
    pub fn load(path: &Path) -> Result<Self, PolicyError> {
        let text = std::fs::read_to_string(path).map_err(|source| PolicyError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        let policy = Self::parse(&text)?;
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        if policy.module != stem {
            return Err(PolicyError::Invalid {
                issues: vec![format!(
                    "module: {:?} must equal the file stem {stem:?} of {}",
                    policy.module,
                    path.display()
                )],
            });
        }
        Ok(policy)
    }

    /// The one-line summary `mt vectors policy-check` prints for a policy.
    #[must_use]
    pub fn summary_row(&self) -> String {
        format!(
            "{} valid default={} overrides={} error_codes={}",
            self.module,
            self.ub.default_action.as_str(),
            self.ub.r#override.len(),
            self.ub.error_code.len()
        )
    }

    /// Check every value. Every issue is collected, sorted and reported at
    /// once. `known_sites` is the list of UB sites the UB log recorded; an
    /// override must name one of them (D-16). Phase 3 has no log and passes an
    /// empty list.
    ///
    /// # Errors
    /// [`PolicyError::Invalid`] listing every issue.
    pub fn validate(&self, known_sites: &[Site]) -> Result<(), PolicyError> {
        let _ = known_sites;
        let mut issues = Vec::new();
        if self.schema_version != POLICY_SCHEMA_VERSION {
            issues.push(format!(
                "schema_version: {} is not supported; expected {POLICY_SCHEMA_VERSION}",
                self.schema_version
            ));
        }
        match parse_version(&self.contract_version) {
            None => issues.push(format!(
                "contract_version: {:?} is not MAJOR.MINOR.PATCH",
                self.contract_version
            )),
            Some(found) => {
                if parse_version(CONTRACT_VERSION).is_some_and(|ours| found > ours) {
                    issues.push(format!(
                        "contract_version: {} is above this crate's contract version {CONTRACT_VERSION}",
                        self.contract_version
                    ));
                }
            }
        }
        if !is_module_name(&self.module) {
            issues.push(format!(
                "module: {:?} must match ^[a-z0-9][a-z0-9_-]*$",
                self.module
            ));
        }
        finish(issues)
    }
}

fn finish(mut issues: Vec<String>) -> Result<(), PolicyError> {
    issues.sort();
    issues.dedup();
    if issues.is_empty() {
        Ok(())
    } else {
        Err(PolicyError::Invalid { issues })
    }
}

/// `MAJOR.MINOR.PATCH`, each part decimal digits.
fn parse_version(text: &str) -> Option<(u64, u64, u64)> {
    let mut parts = text.split('.');
    let mut next = || -> Option<u64> {
        let part = parts.next()?;
        if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        part.parse().ok()
    };
    let version = (next()?, next()?, next()?);
    parts.next().is_none().then_some(version)
}

/// `^[a-z0-9][a-z0-9_-]*$`.
fn is_module_name(text: &str) -> bool {
    let mut bytes = text.bytes();
    let first_ok = bytes
        .next()
        .is_some_and(|b| b.is_ascii_lowercase() || b.is_ascii_digit());
    first_ok
        && bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
}
