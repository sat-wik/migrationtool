//! The deterministic tool-version manifest (CONTEXT D-06, D-15).

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use serde_json::Value;

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

/// Build the manifest from the pins and the report produced from the same
/// observations. Only parsed values enter it: never raw tool output, run
/// hashes or times (`klee --version` prints a varying `Host CPU:` line, for one).
#[must_use]
pub fn build(pins: &Pins, report: &CheckReport) -> Manifest {
    let observed = report
        .rows
        .iter()
        .map(|row| {
            (
                row.key.clone(),
                ObservedRow {
                    status: row.status.as_str().to_owned(),
                    expected: row.expected.clone(),
                    actual: row.actual.clone(),
                    llvm_major: row.llvm_major,
                    detail: row.detail.clone(),
                },
            )
        })
        .collect();
    Manifest {
        schema_version: MANIFEST_SCHEMA_VERSION,
        image: ManifestImage {
            platform: pins.image.platform.clone(),
            base: pins.image.base.clone(),
            base_digest: pins.image.base_digest.clone(),
            snapshot_timestamp: pins.image.snapshot_timestamp.clone(),
        },
        // The pin the check judged against, labelled with its status and decision.
        llvm: ManifestLlvm {
            major: report.llvm.major,
            max_major: report.llvm.max_major,
            status: report.llvm.status,
            decision: report.llvm.decision.clone(),
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
        observed,
    }
}

/// Pretty JSON with sorted keys and a trailing newline.
///
/// The manifest goes through `serde_json::Value`, whose objects are ordered
/// maps, so every key at every level is sorted regardless of struct field
/// order. No header or timestamp is emitted (CONTEXT D-15 makes it optional).
///
/// # Errors
/// [`ManifestError::Serialize`] if serialization fails.
pub fn to_json(manifest: &Manifest) -> Result<String, ManifestError> {
    let value = serde_json::to_value(manifest)?;
    let mut text = serde_json::to_string_pretty(&value)?;
    text.push('\n');
    Ok(text)
}

/// The dotted path of the first key (in sorted order) at which two manifests
/// differ, `None` when their contents are equal.
///
/// A key present on one side only is reported at that key; array elements are
/// reported as `path[index]`; a difference at the root is reported as `$`.
///
/// # Errors
/// [`ManifestError::Parse`] when either text is not JSON.
pub fn first_difference(a: &str, b: &str) -> Result<Option<String>, ManifestError> {
    let parse = |text: &str, which: &'static str| {
        serde_json::from_str::<Value>(text).map_err(|source| ManifestError::Parse { which, source })
    };
    let left = parse(a, "first")?;
    let right = parse(b, "second")?;
    Ok(difference(&left, &right, ""))
}

fn difference(left: &Value, right: &Value, path: &str) -> Option<String> {
    match (left, right) {
        (Value::Object(l), Value::Object(r)) => {
            let keys: BTreeSet<&String> = l.keys().chain(r.keys()).collect();
            for key in keys {
                let child = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                match (l.get(key), r.get(key)) {
                    (Some(lv), Some(rv)) => {
                        if let Some(found) = difference(lv, rv, &child) {
                            return Some(found);
                        }
                    }
                    _ => return Some(child),
                }
            }
            None
        }
        (Value::Array(l), Value::Array(r)) => {
            for index in 0..l.len().max(r.len()) {
                let child = format!("{path}[{index}]");
                match (l.get(index), r.get(index)) {
                    (Some(lv), Some(rv)) => {
                        if let Some(found) = difference(lv, rv, &child) {
                            return Some(found);
                        }
                    }
                    _ => return Some(child),
                }
            }
            None
        }
        _ if left == right => None,
        _ if path.is_empty() => Some("$".to_owned()),
        _ => Some(path.to_owned()),
    }
}
