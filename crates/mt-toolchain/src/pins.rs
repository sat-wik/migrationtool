//! The `pins.toml` schema, version 1.
//!
//! `pins.toml` is the single source of truth for every pinned version, digest,
//! checksum and path (CONTEXT D-14). Unknown keys are rejected everywhere so a
//! typo in a pin cannot silently disable a check. Every map is a `BTreeMap`, so
//! iteration order is the sorted key order.
//!
//! [`Pins::parse`] only checks shape. [`Pins::validate`] checks every value
//! (hex formats, digests, https URLs, placeholders, LLVM major rules) and
//! [`Pins::validate_complete`] additionally requires the full container tool
//! set.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The only schema version this crate understands.
pub const PINS_SCHEMA_VERSION: u32 = 1;

/// Tools the container must provide, each pinned as installed (CONTEXT D-11).
pub const REQUIRED_TOOLS: [&str; 10] = [
    "arm_gcc",
    "bear",
    "c2rust",
    "cargo_mutants",
    "clang",
    "klee",
    "llvm_config",
    "qemu_arm",
    "qemu_system_arm",
    "rustup",
];

/// Tools that are deliberately not installed and must be listed as such (CONTEXT D-10).
pub const NOT_INSTALLED_TOOLS: [&str; 2] = ["hayroll", "kani"];

/// Tools whose LLVM major must be checked, so each carries an LLVM rule (TOOL-02).
pub const LLVM_RULE_TOOLS: [&str; 4] = ["c2rust", "clang", "klee", "llvm_config"];

/// A parsed pins file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pins {
    /// Must equal [`PINS_SCHEMA_VERSION`].
    pub schema_version: u32,
    /// Settings shared by every tool run and by the image build.
    pub image: ImagePins,
    /// The single LLVM major every C-side tool must use.
    pub llvm: LlvmPins,
    /// The two Rust toolchains.
    pub rust: RustPins,
    /// Downloaded or built sources, keyed by a stable source key.
    #[serde(default)]
    pub source: BTreeMap<String, SourcePin>,
    /// Exact Debian package versions, keyed by package name (filled after pin discovery).
    #[serde(default)]
    pub apt: BTreeMap<String, String>,
    /// Pins for the CI host tools.
    pub ci: CiPins,
    /// One table per pinned tool, keyed by a stable tool key.
    pub tool: BTreeMap<String, ToolSpec>,
}

/// Image-wide settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImagePins {
    /// Image platform; only `linux/amd64` is supported (CONTEXT D-22).
    pub platform: String,
    /// Base image reference without the digest, such as `debian:bookworm-20261005-slim`.
    pub base: String,
    /// Digest of the base image index: `sha256:` plus 64 lowercase hex digits.
    pub base_digest: String,
    /// The `snapshot.debian.org` timestamp, `YYYYMMDDTHHMMSSZ`.
    pub snapshot_timestamp: String,
    /// Value of `SOURCE_DATE_EPOCH` for every child.
    pub source_date_epoch: String,
    /// Value of `PATH` for every child.
    pub tool_path: String,
    /// Absolute path of `dpkg-query`, used by the package check.
    pub dpkg_query: String,
}

/// Whether the LLVM pin is still provisional.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LlvmStatus {
    /// Revisited at the Phase 6 R1 verdict.
    Provisional,
    /// Confirmed by a recorded decision.
    Final,
}

/// The one LLVM major used by clang, KLEE, c2rust and (later) Hayroll.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LlvmPins {
    /// The pinned major; must be within 16 to 19.
    pub major: u32,
    /// Highest LLVM major any tool may report; must be 19.
    pub max_major: u32,
    /// Provisional or final.
    pub status: LlvmStatus,
    /// The PROJECT.md decision ID justifying the pin, such as `D-15`.
    pub decision: String,
}

/// The tool and bitcode Rust toolchains.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RustPins {
    /// Absolute `RUSTUP_HOME` inside the image.
    pub rustup_home: String,
    /// Absolute `CARGO_HOME` inside the image.
    pub cargo_home: String,
    /// The host triple of both toolchains.
    pub host: String,
    /// The toolchain that builds the tool and the `no_std` crates.
    pub tool: RustToolPins,
    /// The toolchain whose LLVM matches the C-side LLVM (for KLEE bitcode).
    pub bitcode: RustBitcodePins,
}

/// The tool toolchain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RustToolPins {
    /// Release, such as `1.99.0`.
    pub version: String,
    /// `commit-hash` from `rustc -vV`: 40 lowercase hex digits.
    pub commit: String,
    /// sha256 of the channel manifest.
    pub channel_manifest_sha256: String,
    /// Installed rustup components.
    pub components: Vec<String>,
    /// Installed compilation targets besides the host.
    pub targets: Vec<String>,
}

/// The bitcode toolchain (host target only).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RustBitcodePins {
    /// Release, such as `1.72.1`.
    pub version: String,
    /// `commit-hash` from `rustc -vV`: 40 lowercase hex digits.
    pub commit: String,
    /// The `LLVM version:` its `rustc -vV` reports, such as `16.0.5`.
    pub llvm: String,
    /// sha256 of the channel manifest.
    pub channel_manifest_sha256: String,
    /// The PROJECT.md decision ID justifying the pin.
    pub decision: String,
}

/// One pinned source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub enum SourcePin {
    /// A git checkout asserted by commit.
    Git {
        /// `https://` clone URL.
        url: String,
        /// Tag that is cloned.
        tag: String,
        /// The commit the tag must resolve to: 40 lowercase hex digits.
        commit: String,
    },
    /// A crates.io crate installed with `cargo install --locked`.
    Crate {
        /// Crate name.
        name: String,
        /// Exact version.
        version: String,
        /// sha256 of the `.crate` file.
        sha256: String,
    },
    /// A downloaded file verified by sha256.
    Archive {
        /// Release label.
        version: String,
        /// `https://` download URL.
        url: String,
        /// sha256 of the downloaded file.
        sha256: String,
    },
    /// A source fetched by another pinned source's build.
    Transitive {
        /// Key of the source that pulls this one in.
        parent: String,
        /// The commit the parent fetches: 40 lowercase hex digits.
        commit: String,
    },
}

/// Pins for tools that run on the CI host rather than in the image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CiPins {
    /// Version of `cargo-deny` installed in CI.
    pub cargo_deny_version: String,
    /// BuildKit image reference with an `@sha256:` digest.
    pub buildkit_image: String,
}

/// Whether a tool is installed in the image.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolPinStatus {
    /// The tool is installed and checked.
    #[default]
    Installed,
    /// The tool is deliberately absent; listed for the manifest only.
    NotInstalled,
}

/// Where the LLVM major of a tool comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LlvmSource {
    /// The first component of the extracted version.
    Version,
    /// A regex with named group `major` over the version command's output.
    Output,
    /// A separate command whose output a regex with group `major` is applied to.
    Probe,
}

/// How to obtain a tool's LLVM major.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LlvmRule {
    /// Where the major comes from.
    pub source: LlvmSource,
    /// Regex with named group `major` (sources `output` and `probe`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regex: Option<String>,
    /// Command to run (source `probe`); the first element is an absolute path.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub argv: Vec<String>,
}

/// How to find and identify one tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolSpec {
    /// Installed (default) or not installed.
    #[serde(default)]
    pub status: ToolPinStatus,
    /// Why the tool is not installed (required for, and only for, `not_installed`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Program to run: an absolute path.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub bin: String,
    /// Arguments that make the tool print its version.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub version_args: Vec<String>,
    /// Regex with a named group `version`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub version_regex: String,
    /// The pinned version the extracted `version` must equal.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub expect: String,
    /// Named environment variables passed to the runner for this tool (CONTEXT D-17).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    /// How to obtain the tool's LLVM major, if it links LLVM.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llvm: Option<LlvmRule>,
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
    /// The pins parse but one or more values are not acceptable.
    #[error("pins validation failed with {} issue(s): {}", issues.len(), issues.join("; "))]
    Invalid {
        /// Every issue found, sorted, each naming the offending field.
        issues: Vec<String>,
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

    /// Check every value. Placeholder: accepts everything until the tests drive the rules in.
    ///
    /// # Errors
    /// [`PinsError::Invalid`] listing every issue.
    pub fn validate(&self) -> Result<(), PinsError> {
        Ok(())
    }

    /// [`Pins::validate`] plus the completeness rules. Placeholder.
    ///
    /// # Errors
    /// [`PinsError::Invalid`] listing every issue.
    pub fn validate_complete(&self) -> Result<(), PinsError> {
        self.validate()
    }
}
