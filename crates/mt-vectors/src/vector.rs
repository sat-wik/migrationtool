//! The reference-level vector (EQV-02): exactly the nine TRACTOR fields.
//!
//! A [`Vector`] holds at most the nine fields `argv`, `stdin`, `env`, `stdout`,
//! `stderr`, `rc`, `lib_state_in`, `lib_state_out` and `has_ub`, in that order.
//! `has_ub` is absent when no UB was observed and is never `false` (TRACTOR's
//! tools read any present `has_ub` as UB, D-26); a present one is a
//! [`HasUbLabel`] naming its source. Unknown keys are refused, which also
//! refuses a single merged `lib_state` and TRACTOR's `note`.
//!
//! The canonical byte form is `serde_json::to_vec_pretty` of the struct in
//! field order, maps sorted, plus a trailing newline; its sha256 is the file
//! name of a stored vector.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};

use crate::bytes::decode_latin1;
use crate::state::LibState;

/// A vector file above this size is refused before it is parsed (T-03-10).
pub const MAX_VECTOR_BYTES: usize = 16 * 1024 * 1024;

/// The outcome classes of the contract, as the words written in `stderr`.
pub const OUTCOME_CLASSES: [&str; 5] = ["ok", "fault", "abort", "timeout", "setup_failed"];

/// How one execution of the C ended (equivalence contract, Outcomes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OutcomeClass {
    /// The call returned and the result was read.
    Ok,
    /// The C code faulted or wrote into the slack beside a guarded buffer.
    Fault,
    /// `abort()` or a failed `assert`.
    Abort,
    /// The time budget ran out.
    Timeout,
    /// A declared setup step returned an unexpected value.
    SetupFailed,
}

impl OutcomeClass {
    /// The word written after `outcome=`.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Fault => "fault",
            Self::Abort => "abort",
            Self::Timeout => "timeout",
            Self::SetupFailed => "setup_failed",
        }
    }

    /// The class named by `word`, if it is one of [`OUTCOME_CLASSES`].
    #[must_use]
    pub fn from_word(word: &str) -> Option<Self> {
        match word {
            "ok" => Some(Self::Ok),
            "fault" => Some(Self::Fault),
            "abort" => Some(Self::Abort),
            "timeout" => Some(Self::Timeout),
            "setup_failed" => Some(Self::SetupFailed),
            _ => None,
        }
    }
}

/// Why a vector, or a file holding one, is refused.
#[derive(Debug, thiserror::Error)]
pub enum VectorError {
    /// The file is larger than [`MAX_VECTOR_BYTES`].
    #[error("{what} is {len} bytes, above the {MAX_VECTOR_BYTES} byte limit")]
    TooLarge {
        /// What was measured (a path or "the vector").
        what: String,
        /// Its size in bytes.
        len: u64,
    },
    /// The text is not JSON of the vector's shape (a missing, unknown or
    /// mistyped key).
    #[error("not a valid vector: {0}")]
    Parse(String),
    /// The vector has the right shape but breaks a rule of the reference level.
    #[error("invalid vector: {}", .issues.join("; "))]
    Invalid {
        /// Every rule that was broken, each naming its field.
        issues: Vec<String>,
    },
    /// A file could not be read.
    #[error("cannot read {}: {source}", path.display())]
    Io {
        /// The file.
        path: PathBuf,
        /// The cause.
        #[source]
        source: io::Error,
    },
}

/// The name of the source of a `has_ub` mark, such as `crash:SIGSEGV`.
///
/// The grammar is `^[a-z][a-z-]*(:[A-Za-z0-9_.-]+)*$`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct HasUbLabel(String);

impl HasUbLabel {
    /// Check `text` against the label grammar.
    pub fn new(text: impl Into<String>) -> Result<Self, VectorError> {
        let text = text.into();
        if label_is_well_formed(&text) {
            Ok(Self(text))
        } else {
            Err(VectorError::Invalid {
                issues: vec![format!(
                    "has_ub label {text:?} does not match ^[a-z][a-z-]*(:[A-Za-z0-9_.-]+)*$"
                )],
            })
        }
    }

    /// The label text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for HasUbLabel {
    type Error = VectorError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        Self::new(text)
    }
}

impl From<HasUbLabel> for String {
    fn from(label: HasUbLabel) -> Self {
        label.0
    }
}

fn label_is_well_formed(text: &str) -> bool {
    let mut parts = text.split(':');
    let head_ok = parts.next().is_some_and(|head| {
        let mut chars = head.chars();
        chars.next().is_some_and(|c| c.is_ascii_lowercase())
            && chars.all(|c| c.is_ascii_lowercase() || c == '-')
    });
    head_ok
        && parts.all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
        })
}

/// A present `has_ub` is a label; `null`, booleans and numbers are refused.
fn present_label<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<HasUbLabel>, D::Error> {
    HasUbLabel::deserialize(deserializer).map(Some)
}

/// `stdout` or `stderr`: always written as `{pattern, is_regex}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Output {
    /// The expected text (bytes through the Latin-1 codec for `stdout`).
    pub pattern: String,
    /// Always `false` in our vectors: output is compared exactly.
    pub is_regex: bool,
}

/// A reference vector: the result of one call of one entry (D-05).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Vector {
    /// `<target>/<config>/<entry>`, then the scalar arguments as decimal text.
    pub argv: Vec<String>,
    /// The input bytes through the Latin-1 codec.
    pub stdin: String,
    /// Always empty.
    pub env: BTreeMap<String, String>,
    /// The output buffer after the call.
    pub stdout: Output,
    /// The executor's own text: `outcome=<class>` and optional detail words.
    pub stderr: Output,
    /// The C return value, or `-signal` on a fault.
    pub rc: i32,
    /// The library state before the entry.
    pub lib_state_in: LibState,
    /// The library state after the entry.
    pub lib_state_out: LibState,
    /// Absent when no UB was observed; never `false`.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_label"
    )]
    pub has_ub: Option<HasUbLabel>,
}

/// `argv[0]` must be `<target>/<config>/<entry>`.
fn argv0_is_well_formed(text: &str) -> bool {
    let parts: Vec<&str> = text.split('/').collect();
    let [target, config, entry] = parts.as_slice() else {
        return false;
    };
    let lower_head = |s: &str, tail: fn(char) -> bool| {
        let mut chars = s.chars();
        chars
            .next()
            .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
            && chars.all(tail)
    };
    let mut entry_chars = entry.chars();
    lower_head(target, |c| {
        c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '-')
    }) && lower_head(config, |c| {
        c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'
    }) && entry_chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && entry_chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// The class word of a `stderr` pattern: the text after `outcome=` up to the
/// first space.
fn class_word(pattern: &str) -> Option<&str> {
    let rest = pattern.strip_prefix("outcome=")?;
    Some(rest.split(' ').next().unwrap_or(rest))
}

/// Read a regular file of at most [`MAX_VECTOR_BYTES`], checking the size
/// before reading.
pub fn read_capped(path: &Path) -> Result<Vec<u8>, VectorError> {
    let io_error = |source| VectorError::Io {
        path: path.to_path_buf(),
        source,
    };
    let len = fs::metadata(path).map_err(io_error)?.len();
    if len > u64::try_from(MAX_VECTOR_BYTES).unwrap_or(u64::MAX) {
        return Err(VectorError::TooLarge {
            what: path.display().to_string(),
            len,
        });
    }
    fs::read(path).map_err(io_error)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

impl Vector {
    /// Read a vector from JSON text: the size cap first, then a strict parse,
    /// then every reference-level rule.
    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self, VectorError> {
        if bytes.len() > MAX_VECTOR_BYTES {
            return Err(VectorError::TooLarge {
                what: "the vector".to_owned(),
                len: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
            });
        }
        let vector: Self =
            serde_json::from_slice(bytes).map_err(|e| VectorError::Parse(e.to_string()))?;
        vector.validate()?;
        Ok(vector)
    }

    /// Read a vector file, refusing one above [`MAX_VECTOR_BYTES`] before
    /// reading it.
    pub fn load(path: &Path) -> Result<Self, VectorError> {
        Self::from_json_bytes(&read_capped(path)?)
    }

    /// Check every reference-level rule, collecting all issues.
    pub fn validate(&self) -> Result<(), VectorError> {
        let mut issues = Vec::new();
        match self.argv.first() {
            Some(first) if argv0_is_well_formed(first) => {}
            Some(first) => issues.push(format!(
                "argv[0] {first:?} is not <target>/<config>/<entry>"
            )),
            None => {
                issues.push("argv is empty; argv[0] must be <target>/<config>/<entry>".to_owned())
            }
        }
        if let Err(e) = decode_latin1(&self.stdin) {
            issues.push(format!("stdin: {e}"));
        }
        for (name, state) in [
            ("lib_state_in", &self.lib_state_in),
            ("lib_state_out", &self.lib_state_out),
        ] {
            if let Err(e) = state.validate() {
                issues.push(format!("{name}: {e}"));
            }
        }
        if !self.env.is_empty() {
            issues.push(format!(
                "env must be empty, found {} key(s)",
                self.env.len()
            ));
        }
        if let Err(e) = decode_latin1(&self.stdout.pattern) {
            issues.push(format!("stdout.pattern: {e}"));
        }
        for (name, output) in [("stdout", &self.stdout), ("stderr", &self.stderr)] {
            if output.is_regex {
                issues.push(format!("{name}.is_regex must be false"));
            }
        }
        match class_word(&self.stderr.pattern) {
            Some(word) if OutcomeClass::from_word(word).is_some() => {}
            _ => issues.push(format!(
                "stderr.pattern {:?} must start with outcome=<class>, class one of {}",
                self.stderr.pattern,
                OUTCOME_CLASSES.join(", ")
            )),
        }
        if issues.is_empty() {
            Ok(())
        } else {
            Err(VectorError::Invalid { issues })
        }
    }

    /// The canonical bytes of a vector that passes [`Vector::validate`]: the
    /// bytes that are hashed and stored.
    pub fn to_canonical_json(&self) -> Result<Vec<u8>, VectorError> {
        self.validate()?;
        Ok(self.render())
    }

    fn render(&self) -> Vec<u8> {
        // Invariant: every field is a string, an i32, a `BTreeMap` with string
        // keys or a `serde_json::Value`, none of which can fail to serialize
        // into a `Vec<u8>`.
        #[allow(clippy::expect_used)]
        let mut bytes = serde_json::to_vec_pretty(self).expect("vector fields always serialize");
        bytes.push(b'\n');
        bytes
    }

    /// Lower-case hex sha256 of the canonical bytes: the file name of a stored
    /// vector.
    #[must_use]
    pub fn sha256(&self) -> String {
        hex(&Sha256::digest(self.render()))
    }

    /// True exactly when `has_ub` is present: output comparison skips the
    /// vector (D-08).
    #[must_use]
    pub const fn excluded_from_comparison(&self) -> bool {
        self.has_ub.is_some()
    }

    /// The outcome class written in `stderr`.
    pub fn outcome_class(&self) -> Result<OutcomeClass, VectorError> {
        class_word(&self.stderr.pattern)
            .and_then(OutcomeClass::from_word)
            .ok_or_else(|| VectorError::Invalid {
                issues: vec![format!(
                    "stderr.pattern {:?} does not start with outcome=<class>",
                    self.stderr.pattern
                )],
            })
    }
}

/// A present `has_ub` of any JSON value (string, integer, `false`, `null`) is
/// kept as present, as TRACTOR's checker reads it.
fn present_value<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<serde_json::Value>, D::Error> {
    serde_json::Value::deserialize(deserializer).map(Some)
}

/// `stdout` or `stderr` of a TRACTOR vector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TractorOutput {
    /// The expected text.
    pub pattern: String,
    /// Whether the pattern is a regular expression; absent means exact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_regex: Option<bool>,
}

/// A vector at the TRACTOR-compatible level: every field optional, as in
/// TRACTOR's own test-vector type, plus its free-text `note`.
///
/// Any present `has_ub` reads as UB, whatever its value
/// ([`TractorVector::has_ub_present`]). A key that is not one of the ten is
/// refused. A field written as `null` is read as absent, as TRACTOR's loader
/// does, and so does not load unchanged.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TractorVector {
    /// Arguments after the driver path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub argv: Option<Vec<String>>,
    /// Standard input text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdin: Option<String>,
    /// Environment variables.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<BTreeMap<String, String>>,
    /// Expected standard output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdout: Option<TractorOutput>,
    /// Expected standard error.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stderr: Option<TractorOutput>,
    /// Expected exit code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rc: Option<i32>,
    /// Library state before the call: free-form JSON.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lib_state_in: Option<serde_json::Value>,
    /// Library state after the call: free-form JSON.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lib_state_out: Option<serde_json::Value>,
    /// Present for a vector TRACTOR's tools treat as UB; any value.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_value"
    )]
    pub has_ub: Option<serde_json::Value>,
    /// Free text; accepted on load and never emitted by this tool.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl TractorVector {
    /// Parse `bytes` with no preprocessing, and succeed only when the parsed
    /// type re-serializes to a `serde_json::Value` equal to the file's own.
    ///
    /// The file's bytes and key order are not preserved (serde_json maps are
    /// sorted), so byte equality is never promised for these files.
    pub fn load_unchanged(bytes: &[u8]) -> Result<(Self, serde_json::Value), VectorError> {
        if bytes.len() > MAX_VECTOR_BYTES {
            return Err(VectorError::TooLarge {
                what: "the vector".to_owned(),
                len: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
            });
        }
        let parsed: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|e| VectorError::Parse(e.to_string()))?;
        let vector: Self =
            serde_json::from_slice(bytes).map_err(|e| VectorError::Parse(e.to_string()))?;
        let again = serde_json::to_value(&vector).map_err(|e| VectorError::Parse(e.to_string()))?;
        if again == parsed {
            Ok((vector, parsed))
        } else {
            Err(VectorError::Invalid {
                issues: vec!["the vector does not re-serialize JSON-equal to the file".to_owned()],
            })
        }
    }

    /// True when the file carried a `has_ub` key of any value: TRACTOR's
    /// tools treat the vector as UB.
    #[must_use]
    pub const fn has_ub_present(&self) -> bool {
        self.has_ub.is_some()
    }
}
