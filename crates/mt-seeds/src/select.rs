//! Deterministic seed selection under the D-16 caps.

use serde::{Deserialize, Serialize};

use crate::Method;

/// Where a candidate came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRef {
    /// Path of the source file, relative to the target tree (or the script for hand-made seeds).
    pub path: String,
    /// 1-based line, for seeds taken from a C or C++ source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    /// Member name, for seeds taken from a zip archive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member: Option<String>,
}

/// A possible seed before selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// The seed bytes.
    pub bytes: Vec<u8>,
    /// How the bytes were obtained.
    pub method: Method,
    /// Where they came from.
    pub source: SourceRef,
    /// Entry points the seed is meant to exercise.
    pub entries: Vec<String>,
    /// True when the seed is a proposal awaiting review (hand-made seeds).
    pub proposed: bool,
}

/// The D-16 caps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Caps {
    /// Most seeds kept.
    pub max_seeds: u64,
    /// Most bytes kept in total.
    pub max_total_bytes: u64,
    /// Largest single seed.
    pub max_seed_bytes: u64,
}

impl Default for Caps {
    /// The proposed D-16 values: 200 seeds, 1 MiB in total, 64 KiB each.
    fn default() -> Self {
        Self {
            max_seeds: 200,
            max_total_bytes: 1024 * 1024,
            max_seed_bytes: 64 * 1024,
        }
    }
}

/// Candidates not kept, by reason.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Skipped {
    /// Same bytes as an earlier seed.
    pub duplicate: u64,
    /// Larger than `max_seed_bytes`.
    pub too_large: u64,
    /// Not reached before a cap stopped the selection.
    pub over_cap: u64,
    /// Could not be decoded by an extractor.
    pub unsupported: u64,
}

/// A kept seed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedSeed {
    /// Lower-case hex sha256 of the bytes.
    pub sha256: String,
    /// The candidate that won.
    pub candidate: Candidate,
}

/// The outcome of [`select`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Selection {
    /// Kept seeds in selection order.
    pub seeds: Vec<SelectedSeed>,
    /// Counts of what was left out.
    pub skipped: Skipped,
}

/// Choose seeds from `candidates` under `caps`.
pub fn select(candidates: Vec<Candidate>, caps: &Caps, unsupported: u64) -> Selection {
    let _ = (candidates, caps);
    Selection {
        seeds: Vec::new(),
        skipped: Skipped {
            unsupported,
            ..Skipped::default()
        },
    }
}
