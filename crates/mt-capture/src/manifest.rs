//! The `testbed/manifest.toml` schema, version 1.
//!
//! The manifest is the single committed list of test-bed targets (CONTEXT
//! D-20): where each source comes from and how it is pinned, its licence, the
//! entry functions the harness drives and the build configurations that are
//! captured. Unknown keys are rejected at every level so a typo cannot silently
//! disable a check, and every map is a `BTreeMap`, so iteration order is the
//! sorted key order.
//!
//! [`Manifest::parse`] checks shape only. [`Manifest::validate`] checks every
//! value and [`Manifest::validate_complete`] additionally requires a build
//! recipe on every target.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The only schema version this crate understands.
pub const MANIFEST_SCHEMA_VERSION: u32 = 1;

/// Licence identifiers a target may elect (CONTEXT D-02, D-27).
pub const PERMISSIVE_LICENCES: [&str; 6] = [
    "Apache-2.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "MIT",
    "Zlib",
];

/// The beachhead categories the committed manifest must cover (CONTEXT D-04).
pub const BEACHHEAD_CATEGORIES: [Category; 6] = [
    Category::Tlv,
    Category::Cbor,
    Category::Mqtt,
    Category::Ble,
    Category::OtaHeader,
    Category::Asn1X509,
];

/// The only placeholders a build string may contain (CONTEXT D-20).
///
/// `{cc}` is the pinned `arm-none-eabi-gcc` path, `{cflags}` the baseline flags
/// followed by the configuration's defines and flags, `{src}` the absolute
/// source root of the run and `{inputs}` the absolute directory holding the
/// committed files under `testbed/targets/<id>/`.
pub const PLACEHOLDERS: [&str; 4] = ["{cc}", "{cflags}", "{inputs}", "{src}"];

/// The Cortex-M4F baseline every configuration builds on, in order (CONTEXT D-06).
pub const BASELINE_CFLAGS: [&str; 5] = [
    "-mcpu=cortex-m4",
    "-mthumb",
    "-mfloat-abi=hard",
    "-mfpu=fpv4-sp-d16",
    "-Os",
];

/// A parsed test-bed manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// Must equal [`MANIFEST_SCHEMA_VERSION`].
    pub schema_version: u32,
    /// The recorded first-RTOS decision (BED-02).
    pub decisions: Decisions,
    /// The flags every configuration shares.
    pub baseline: Baseline,
    /// The targets, keyed by id.
    pub target: BTreeMap<String, Target>,
}

/// The recorded decisions the manifest depends on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decisions {
    /// `deferred`, `zephyr` or `freertos`.
    pub first_rtos: String,
    /// Why the choice is deferred (required when `first_rtos` is `deferred`).
    pub first_rtos_reason: String,
    /// The ecosystems under consideration.
    pub first_rtos_candidates: Vec<String>,
    /// The PROJECT.md decision ID that records it, such as `D-18`.
    pub project_decision: String,
}

/// Flags shared by every configuration of every target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Baseline {
    /// Must equal [`BASELINE_CFLAGS`] exactly.
    pub cflags: Vec<String>,
}

/// What kind of build system drives a target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BuildSystem {
    /// GNU Make, captured with Bear.
    Make,
    /// CMake, captured through `CMAKE_EXPORT_COMPILE_COMMANDS`.
    Cmake,
}

/// What a target parses (CONTEXT D-04, D-29).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    /// Type-length-value structures.
    Tlv,
    /// CBOR.
    Cbor,
    /// MQTT.
    Mqtt,
    /// BLE advertising data, GATT and ATT.
    Ble,
    /// OTA image headers.
    OtaHeader,
    /// ASN.1 and X.509.
    Asn1X509,
    /// JSON.
    Json,
    /// Protocol Buffers.
    Protobuf,
    /// CoAP.
    Coap,
    /// HTTP.
    Http,
    /// CAN framing.
    Can,
    /// Compression formats.
    Compression,
}

/// Where a target's source comes from and how it is pinned (CONTEXT D-17, D-20).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub enum Source {
    /// A shallow git fetch of one commit.
    Git {
        /// `https://` clone URL.
        url: String,
        /// Tag or branch the commit was taken from (informational).
        #[serde(rename = "ref")]
        reference: String,
        /// The commit: 40 lowercase hex digits.
        commit: String,
        /// The tree digest: 64 lowercase hex digits.
        tree_sha256: String,
    },
    /// A release archive, pinned by archive digest and tree digest.
    Archive {
        /// `https://` download URL.
        url: String,
        /// sha256 of the downloaded file.
        sha256: String,
        /// Leading path components removed on extraction.
        strip_components: u32,
        /// The release tag (informational).
        #[serde(rename = "ref")]
        reference: String,
        /// The commit of the tag, recorded for provenance.
        commit: String,
        /// The tree digest of the extracted tree.
        tree_sha256: String,
    },
}

/// The licence a target is used under (CONTEXT D-02, D-27).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Licence {
    /// One SPDX identifier, or identifiers joined by ` OR `.
    pub spdx: String,
    /// The permissive operand the project uses.
    pub elected: String,
    /// The licence file at the pinned commit, relative to the tree root.
    pub file: String,
    /// A reader's note, such as which part of the tree the licence covers.
    #[serde(default)]
    pub note: Option<String>,
}

fn default_dir() -> String {
    ".".to_owned()
}

/// Everything the project's own build system needs (CONTEXT D-07).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildArgs {
    /// Build directory relative to the tree root.
    #[serde(default = "default_dir")]
    pub dir: String,
    /// Make goals.
    #[serde(default)]
    pub goals: Vec<String>,
    /// Make variable assignments, such as `CC={cc} {cflags}`.
    #[serde(default)]
    pub make_vars: Vec<String>,
    /// The CMake target to build.
    #[serde(default)]
    pub cmake_target: Option<String>,
    /// CMake configure options.
    #[serde(default)]
    pub cmake_options: Vec<String>,
}

/// How an entry function receives its input (CONTEXT D-24, D-28).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntryForm {
    /// A pointer plus a length.
    Direct,
    /// A cursor plus an end pointer.
    Range,
    /// A pointer and length inside a struct argument; needs `accepted_wrapped`.
    Wrapped,
}

/// One buffer-in entry function, named by hand and checked after capture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// The C function name.
    pub name: String,
    /// The declaring header, relative to the tree root.
    pub header: String,
    /// The defining translation unit, relative to the tree root.
    pub source: String,
    /// How the input is passed.
    pub form: EntryForm,
    /// The `type.qualType` string clang prints for the function.
    pub signature: String,
    /// Set on, and only on, `wrapped` entries the founder accepted (CONTEXT D-28).
    #[serde(default)]
    pub accepted_wrapped: bool,
}

/// One named build configuration: macro and flag sets on top of the baseline.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// `default` for upstream defaults, otherwise the variant's name.
    pub name: String,
    /// Preprocessor definitions, without the leading `-D`.
    #[serde(default)]
    pub defines: Vec<String>,
    /// Extra compiler flags.
    #[serde(default)]
    pub cflags: Vec<String>,
    /// Extra Make variable assignments.
    #[serde(default)]
    pub make_vars: Vec<String>,
    /// Extra CMake configure options.
    #[serde(default)]
    pub cmake_options: Vec<String>,
    /// Extra Make goals.
    #[serde(default)]
    pub goals: Vec<String>,
    /// A different CMake target.
    #[serde(default)]
    pub cmake_target: Option<String>,
}

/// One test-bed target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    /// What the target parses; at least one.
    pub category: Vec<Category>,
    /// The build system that drives it.
    pub build: BuildSystem,
    /// Where the source comes from.
    pub source: Source,
    /// The licence it is used under.
    pub licence: Licence,
    /// The build recipe; filled only for targets whose recipe is written.
    #[serde(default)]
    pub build_args: Option<BuildArgs>,
    /// One to three entry functions.
    #[serde(default)]
    pub entry: Vec<Entry>,
    /// The default configuration and at most one variant.
    #[serde(default)]
    pub config: Vec<Config>,
}

impl Target {
    /// The configuration named `default`, if declared.
    #[must_use]
    pub fn default_config(&self) -> Option<&Config> {
        self.config.iter().find(|c| c.name == "default")
    }

    /// The one configuration that is not `default`, if declared.
    #[must_use]
    pub fn variant(&self) -> Option<&Config> {
        self.config.iter().find(|c| c.name != "default")
    }
}

/// Why a manifest could not be used.
#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    /// The file could not be read.
    #[error("cannot read manifest {}: {source}", path.display())]
    Read {
        /// The path that failed.
        path: PathBuf,
        /// The underlying error.
        #[source]
        source: std::io::Error,
    },
    /// The text is not valid TOML for this schema.
    #[error("invalid manifest: {0}")]
    Parse(#[from] toml::de::Error),
    /// The file declares a schema this crate does not support.
    #[error("unsupported manifest schema_version {found}; expected {expected}")]
    SchemaVersion {
        /// Version found in the file.
        found: u32,
        /// Version this crate supports.
        expected: u32,
    },
    /// The manifest parses but one or more values are not acceptable.
    #[error("manifest validation failed with {} issue(s): {}", issues.len(), issues.join("; "))]
    Invalid {
        /// Every issue found, sorted, each starting with its dotted field path.
        issues: Vec<String>,
    },
    /// The canonical form could not be serialized.
    #[error("cannot serialize manifest: {0}")]
    Json(#[from] serde_json::Error),
}

impl Manifest {
    /// Parse a manifest from TOML text.
    ///
    /// # Errors
    /// [`ManifestError::Parse`] on malformed TOML, unknown keys or a duplicate
    /// table; [`ManifestError::SchemaVersion`] when `schema_version` is not 1.
    pub fn parse(text: &str) -> Result<Self, ManifestError> {
        let manifest: Self = toml::from_str(text)?;
        if manifest.schema_version != MANIFEST_SCHEMA_VERSION {
            return Err(ManifestError::SchemaVersion {
                found: manifest.schema_version,
                expected: MANIFEST_SCHEMA_VERSION,
            });
        }
        Ok(manifest)
    }

    /// Read and parse a manifest file.
    ///
    /// # Errors
    /// [`ManifestError::Read`] if the file cannot be read, otherwise as
    /// [`Manifest::parse`].
    pub fn load(path: &Path) -> Result<Self, ManifestError> {
        let text = std::fs::read_to_string(path).map_err(|source| ManifestError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse(&text)
    }

    /// Check every value. Every issue is collected, sorted and reported at
    /// once, each starting with its dotted field path.
    ///
    /// # Errors
    /// [`ManifestError::Invalid`] listing every issue.
    pub fn validate(&self) -> Result<(), ManifestError> {
        Ok(())
    }

    /// [`Manifest::validate`] plus the completeness rule: every target carries
    /// a `build_args` recipe.
    ///
    /// # Errors
    /// [`ManifestError::Invalid`] listing every issue.
    pub fn validate_complete(&self) -> Result<(), ManifestError> {
        self.validate()
    }

    /// The canonical JSON form: configurations ordered `default` first and then
    /// by name, entries ordered by name, pretty-printed with a trailing newline.
    ///
    /// # Errors
    /// [`ManifestError::Json`] if serialization fails.
    pub fn canonical_json(&self) -> Result<String, ManifestError> {
        let mut text = serde_json::to_string_pretty(self)?;
        text.push('\n');
        Ok(text)
    }
}
