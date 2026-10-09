//! The deterministic tool-version manifest (CONTEXT D-06, D-15).

use std::collections::BTreeMap;

use serde::Serialize;

use crate::check::CheckReport;
use crate::pins::{LlvmStatus, Pins, SourcePin};

/// Version of the manifest layout.
pub const MANIFEST_SCHEMA_VERSION: u32 = 1;

/// The manifest: pins plus every row's parsed observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Manifest {
    /// [`MANIFEST_SCHEMA_VERSION`].
    pub schema_version: u32,
    /// The image the tools live in.
    pub image: ManifestImage,
    /// The LLVM pin and its status.
    pub llvm: ManifestLlvm,
    /// The two Rust toolchains.
    pub rust: ManifestRust,
    /// Every pinned source.
    pub sources: BTreeMap<String, SourcePin>,
    /// One entry per checked item.
    pub observed: BTreeMap<String, ObservedRow>,
}

/// Image identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManifestImage {
    /// Platform, `linux/amd64`.
    pub platform: String,
    /// Base image reference.
    pub base: String,
    /// Base image index digest.
    pub base_digest: String,
    /// The `snapshot.debian.org` timestamp input.
    pub snapshot_timestamp: String,
}

/// The LLVM pin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManifestLlvm {
    /// Pinned major.
    pub major: u32,
    /// Highest permitted major.
    pub max_major: u32,
    /// Provisional or final.
    pub status: LlvmStatus,
    /// The PROJECT.md decision ID.
    pub decision: String,
}

/// The Rust toolchains.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManifestRust {
    /// The tool toolchain.
    pub tool: ManifestToolchain,
    /// The bitcode toolchain.
    pub bitcode: ManifestBitcode,
}

/// The tool toolchain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManifestToolchain {
    /// Release.
    pub version: String,
    /// Commit hash.
    pub commit: String,
}

/// The bitcode toolchain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManifestBitcode {
    /// Release.
    pub version: String,
    /// Commit hash.
    pub commit: String,
    /// The LLVM version its `rustc -vV` reports.
    pub llvm: String,
    /// The PROJECT.md decision ID.
    pub decision: String,
}

/// One checked item, as parsed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ObservedRow {
    /// The status word.
    pub status: String,
    /// The pinned value.
    pub expected: String,
    /// The parsed live value.
    pub actual: Option<String>,
    /// The LLVM major, where one applies.
    pub llvm_major: Option<u32>,
    /// The reason for the status, when there is one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Why a manifest could not be produced or compared.
#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    /// Serialization failed.
    #[error("cannot serialize the manifest: {0}")]
    Serialize(#[from] serde_json::Error),
    /// One of the compared texts is not JSON.
    #[error("{which} manifest is not valid JSON: {source}")]
    Parse {
        /// `first` or `second`.
        which: &'static str,
        /// The parse error.
        #[source]
        source: serde_json::Error,
    },
}

/// Build the manifest from the pins and the report produced from the same observations.
#[must_use]
pub fn build(pins: &Pins, _report: &CheckReport) -> Manifest {
    // Placeholder for the RED commit: nothing observed yet.
    Manifest {
        schema_version: MANIFEST_SCHEMA_VERSION,
        image: ManifestImage {
            platform: pins.image.platform.clone(),
            base: pins.image.base.clone(),
            base_digest: pins.image.base_digest.clone(),
            snapshot_timestamp: pins.image.snapshot_timestamp.clone(),
        },
        llvm: ManifestLlvm {
            major: pins.llvm.major,
            max_major: pins.llvm.max_major,
            status: pins.llvm.status,
            decision: pins.llvm.decision.clone(),
        },
        rust: ManifestRust {
            tool: ManifestToolchain {
                version: pins.rust.tool.version.clone(),
                commit: pins.rust.tool.commit.clone(),
            },
            bitcode: ManifestBitcode {
                version: pins.rust.bitcode.version.clone(),
                commit: pins.rust.bitcode.commit.clone(),
                llvm: pins.rust.bitcode.llvm.clone(),
                decision: pins.rust.bitcode.decision.clone(),
            },
        },
        sources: pins.source.clone(),
        observed: BTreeMap::new(),
    }
}

/// Pretty JSON with sorted keys and a trailing newline.
///
/// # Errors
/// [`ManifestError::Serialize`] if serialization fails.
pub fn to_json(_manifest: &Manifest) -> Result<String, ManifestError> {
    // Placeholder for the RED commit.
    Ok("{}\n".to_owned())
}

/// The dotted path of the first key (in sorted order) at which two manifests differ.
///
/// # Errors
/// [`ManifestError::Parse`] when either text is not JSON.
pub fn first_difference(_a: &str, _b: &str) -> Result<Option<String>, ManifestError> {
    // Placeholder for the RED commit.
    Ok(None)
}
