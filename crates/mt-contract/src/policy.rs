//! The per-module UB policy file (CONTEXT D-15, D-16, D-17, D-18, D-29).
//!
//! One TOML file per module, `testbed/policy/<module>.toml` for the test bed.
//! Every struct rejects unknown keys, so a typo cannot silently disable a rule
//! (threat T-03-05). [`Policy::parse`] checks shape only; [`Policy::validate`]
//! checks every value and collects all issues into one error.

use std::collections::BTreeSet;
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

impl std::fmt::Display for ErrorValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Integer(n) => write!(f, "{n}"),
            Self::Text(text) => write!(f, "{text:?}"),
        }
    }
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
        self.ub
            .collect_issues(&self.module, known_sites, &mut issues);
        finish(issues)
    }

    /// The report of the module's UB settings that goes into evidence (D-18):
    /// the module, the default, the default change if any, then every override
    /// with all seven fields, ordered by (function, kind, location). One
    /// `key=value` field per line; values hold no raw line break. The text
    /// holds no clock reading and no path, so two calls give the same text.
    #[must_use]
    pub fn override_report(&self) -> String {
        let mut lines = vec![
            format!("module={}", escape(&self.module)),
            format!("default_action={}", self.ub.default_action.as_str()),
        ];
        match &self.ub.default_change {
            None => lines.push("default_change=none".to_owned()),
            Some(change) => {
                lines.push(format!("default_change.module={}", escape(&change.module)));
                lines.push(format!("default_change.reason={}", escape(&change.reason)));
                lines.push(format!(
                    "default_change.approver={}",
                    escape(&change.approver)
                ));
                lines.push(format!("default_change.date={}", escape(&change.date)));
            }
        }
        let mut overrides: Vec<&Override> = self.ub.r#override.iter().collect();
        overrides.sort_by(|a, b| {
            let key = |o: &Override| {
                (
                    o.function.clone(),
                    o.kind.clone(),
                    o.location.clone(),
                    o.action.as_str(),
                    o.reason.clone(),
                    o.approver.clone(),
                    o.date.clone(),
                )
            };
            key(a).cmp(&key(b))
        });
        lines.push(format!("overrides={}", overrides.len()));
        for (i, o) in overrides.iter().enumerate() {
            lines.push(format!("override[{i}].function={}", escape(&o.function)));
            lines.push(format!("override[{i}].kind={}", escape(&o.kind)));
            lines.push(format!("override[{i}].location={}", escape(&o.location)));
            lines.push(format!("override[{i}].action={}", o.action.as_str()));
            lines.push(format!("override[{i}].reason={}", escape(&o.reason)));
            lines.push(format!("override[{i}].approver={}", escape(&o.approver)));
            lines.push(format!("override[{i}].date={}", escape(&o.date)));
        }
        let mut text = lines.join("\n");
        text.push('\n');
        text
    }
}

impl Ub {
    /// The D-16, D-18 and D-29 rules; `module` is the file's module.
    fn collect_issues(&self, module: &str, known_sites: &[Site], issues: &mut Vec<String>) {
        match (&self.default_action, &self.default_change) {
            (Action::ReturnError, None) => {}
            (Action::ReturnError, Some(_)) => issues.push(
                "ub.default_change: refused while default_action is return_error; a default \
                 change is recorded only with a changed default (D-29)"
                    .to_owned(),
            ),
            (other, None) => issues.push(format!(
                "ub.default_action: {} needs a [ub.default_change] record naming module, \
                 reason, approver and date (D-16, D-29)",
                other.as_str()
            )),
            (_, Some(_)) => {}
        }
        if let Some(change) = &self.default_change {
            if change.module != module {
                issues.push(format!(
                    "ub.default_change.module: {:?} must equal the file's module {module:?} (D-29)",
                    change.module
                ));
            }
            for (key, value) in [("reason", &change.reason), ("approver", &change.approver)] {
                if is_blank(value) {
                    issues.push(format!("ub.default_change.{key}: must not be empty (D-29)"));
                }
            }
            if !is_calendar_date(&change.date) {
                issues.push(format!(
                    "ub.default_change.date: {:?} is not a calendar date YYYY-MM-DD (D-29)",
                    change.date
                ));
            }
        }

        let mut coded = BTreeSet::new();
        for (i, code) in self.error_code.iter().enumerate() {
            if !is_c_identifier(&code.function) {
                issues.push(format!(
                    "ub.error_code[{i}].function: {:?} is not a C identifier (D-17)",
                    code.function
                ));
            } else if !coded.insert(code.function.as_str()) {
                issues.push(format!(
                    "ub.error_code[{i}].function: duplicate entry for {:?}; one declared code \
                     per function (D-17)",
                    code.function
                ));
            }
            let value_ok = match &code.value {
                ErrorValue::Integer(_) => true,
                ErrorValue::Text(text) => is_decimal_integer(text) || is_c_identifier(text),
            };
            if !value_ok {
                issues.push(format!(
                    "ub.error_code[{i}].value: {} is neither a decimal integer nor a C \
                     identifier (D-17)",
                    code.value
                ));
            }
        }

        let mut seen: Vec<(&Override, usize)> = Vec::new();
        for (i, o) in self.r#override.iter().enumerate() {
            for (key, value) in [
                ("function", &o.function),
                ("kind", &o.kind),
                ("location", &o.location),
                ("reason", &o.reason),
                ("approver", &o.approver),
            ] {
                if is_blank(value) {
                    issues.push(format!("ub.override[{i}].{key}: must not be empty (D-18)"));
                }
            }
            if !is_calendar_date(&o.date) {
                issues.push(format!(
                    "ub.override[{i}].date: {:?} is not a calendar date YYYY-MM-DD (D-18)",
                    o.date
                ));
            }
            let sited = |other: &Override| {
                (&other.function, &other.kind, &other.location)
                    == (&o.function, &o.kind, &o.location)
            };
            if let Some((_, first)) = seen.iter().find(|(other, _)| sited(other)) {
                issues.push(format!(
                    "ub.override[{i}]: duplicate site (function {:?}, kind {:?}, location {:?}) \
                     of ub.override[{first}]; one override per site (D-16)",
                    o.function, o.kind, o.location
                ));
            } else {
                seen.push((o, i));
            }
            let logged = known_sites.iter().any(|k| {
                (&k.function, &k.kind, &k.location) == (&o.function, &o.kind, &o.location)
            });
            if !logged {
                issues.push(format!(
                    "ub.override[{i}]: site (function {:?}, kind {:?}, location {:?}) is not in \
                     the UB log's known sites; an override must name a logged site (D-16)",
                    o.function, o.kind, o.location
                ));
            }
        }
    }
}

/// Load every policy of `dir` for the module ids `ids`: the set of `*.toml`
/// files must equal the ids, and every policy must load and validate with no
/// logged UB site (Phase 3: the override tables are empty). Policies come back
/// in the order of `ids`.
///
/// # Errors
/// [`PolicyError::Read`] if the directory cannot be read, otherwise
/// [`PolicyError::Invalid`] listing every missing file, stray file and invalid
/// policy.
pub fn load_dir(dir: &Path, ids: &[String]) -> Result<Vec<Policy>, PolicyError> {
    let read = |source| PolicyError::Read {
        path: dir.to_path_buf(),
        source,
    };
    let mut stems = BTreeSet::new();
    for entry in std::fs::read_dir(dir).map_err(read)? {
        let path = entry.map_err(read)?.path();
        if path.extension().is_some_and(|e| e == "toml")
            && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
        {
            stems.insert(stem.to_owned());
        }
    }
    let mut issues = Vec::new();
    for id in ids {
        if !stems.contains(id) {
            issues.push(format!(
                "target {id} has no policy file {}",
                dir.join(format!("{id}.toml")).display()
            ));
        }
    }
    for stem in &stems {
        if !ids.contains(stem) {
            issues.push(format!(
                "{} names no manifest target",
                dir.join(format!("{stem}.toml")).display()
            ));
        }
    }
    if !issues.is_empty() {
        return finish(issues).map(|()| Vec::new());
    }
    let mut policies = Vec::with_capacity(ids.len());
    for id in ids {
        let path = dir.join(format!("{id}.toml"));
        match Policy::load(&path) {
            Err(error) => issues.push(format!("{id}: {error}")),
            Ok(policy) => match policy.validate(&[]) {
                Ok(()) => policies.push(policy),
                Err(PolicyError::Invalid { issues: found }) => {
                    issues.extend(found.into_iter().map(|issue| format!("{id}: {issue}")));
                }
                Err(other) => issues.push(format!("{id}: {other}")),
            },
        }
    }
    finish(issues).map(|()| policies)
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

fn is_blank(text: &str) -> bool {
    text.trim().is_empty()
}

/// `[A-Za-z_][A-Za-z0-9_]*`.
fn is_c_identifier(text: &str) -> bool {
    let mut bytes = text.bytes();
    bytes
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// An optional leading `-` and at least one decimal digit.
fn is_decimal_integer(text: &str) -> bool {
    let digits = text.strip_prefix('-').unwrap_or(text);
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}

/// `YYYY-MM-DD` naming a real day of the Gregorian calendar, year 0001 to 9999.
/// No clock is read.
fn is_calendar_date(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    let number = |range: std::ops::Range<usize>| -> Option<u32> {
        let part = &bytes[range];
        part.iter()
            .all(u8::is_ascii_digit)
            .then(|| part.iter().fold(0, |n, d| n * 10 + u32::from(d - b'0')))
    };
    let (Some(year), Some(month), Some(day)) = (number(0..4), number(5..7), number(8..10)) else {
        return false;
    };
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    year >= 1 && (1..=days).contains(&day)
}

/// One line per field: backslash, line breaks, tabs and other control
/// characters are written as escapes.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{{{:x}}}", u32::from(c))),
            c => out.push(c),
        }
    }
    out
}
