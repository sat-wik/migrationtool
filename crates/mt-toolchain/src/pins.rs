//! The `pins.toml` schema (tracer subset; plan 01-03 extends it).
//!
//! Unknown keys are rejected everywhere so a typo in a pin cannot silently
//! disable a check. Tool tables live in a `BTreeMap`, so iteration order is the
//! sorted key order.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// The only schema version this crate understands.
pub const PINS_SCHEMA_VERSION: u32 = 1;

/// A parsed pins file.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pins {
    /// Must equal [`PINS_SCHEMA_VERSION`].
    pub schema_version: u32,
    /// Settings shared by every tool run.
    pub image: ImagePins,
    /// One table per pinned tool, keyed by a stable tool key.
    pub tool: BTreeMap<String, ToolSpec>,
}

/// Image-wide settings the runner needs.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImagePins {
    /// Value of `SOURCE_DATE_EPOCH` for every child.
    pub source_date_epoch: String,
    /// Value of `PATH` for every child.
    pub tool_path: String,
}

/// How to find and identify one tool.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolSpec {
    /// Program to run (absolute path or a name found on `tool_path`).
    pub bin: String,
    /// Arguments that make the tool print its version.
    pub version_args: Vec<String>,
    /// Regex with a named group `version`.
    pub version_regex: String,
    /// The pinned version the extracted `version` must equal.
    pub expect: String,
}

/// Why a pins file could not be used.
#[derive(Debug, thiserror::Error)]
pub enum PinsError {
    /// The file could not be read.
    #[error("cannot read pins file {}: {source}", path.display())]
    Read {
        /// The path that failed.
        path: PathBuf,
        /// The underlying error.
        #[source]
        source: std::io::Error,
    },
    /// The text is not valid TOML for this schema.
    #[error("invalid pins file: {0}")]
    Parse(#[from] toml::de::Error),
    /// The file declares a schema this crate does not support.
    #[error("unsupported pins schema_version {found}; expected {expected}")]
    SchemaVersion {
        /// Version found in the file.
        found: u32,
        /// Version this crate supports.
        expected: u32,
    },
}

impl Pins {
    /// Parse pins from TOML text.
    ///
    /// # Errors
    /// [`PinsError::Parse`] on malformed TOML or unknown keys;
    /// [`PinsError::SchemaVersion`] when `schema_version` is not 1.
    pub fn parse(text: &str) -> Result<Self, PinsError> {
        let pins: Self = toml::from_str(text)?;
        if pins.schema_version != PINS_SCHEMA_VERSION {
            return Err(PinsError::SchemaVersion {
                found: pins.schema_version,
                expected: PINS_SCHEMA_VERSION,
            });
        }
        Ok(pins)
    }

    /// Read and parse a pins file.
    ///
    /// # Errors
    /// [`PinsError::Read`] if the file cannot be read, otherwise as [`Pins::parse`].
    pub fn load(path: &Path) -> Result<Self, PinsError> {
        let text = std::fs::read_to_string(path).map_err(|source| PinsError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse(&text)
    }
}
