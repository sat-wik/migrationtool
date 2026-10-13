//! Deterministic seed selection under the D-16 caps.

use std::cmp::Ordering;
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{Method, sha256_hex};

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

/// Total order on candidates, built only from their content so the input
/// order can never matter: method rank, source path bytes, line, member name,
/// then the seed bytes, entries and flag as the last tie-breaks.
fn selection_order(a: &Candidate, b: &Candidate) -> Ordering {
    a.method
        .cmp(&b.method)
        .then_with(|| a.source.path.as_bytes().cmp(b.source.path.as_bytes()))
        .then_with(|| a.source.line.cmp(&b.source.line))
        .then_with(|| a.source.member.cmp(&b.source.member))
        .then_with(|| a.bytes.cmp(&b.bytes))
        .then_with(|| a.entries.cmp(&b.entries))
        .then_with(|| a.proposed.cmp(&b.proposed))
}

/// Choose seeds from `candidates` under `caps` (D-16).
///
/// 1. Candidates are put in a total order: method rank (`hand_made`,
///    `file_copy`, `hex_text`, `c_array`, `c_string`, `zip_member`), then source
///    path bytes, then position (line, then zip member name).
/// 2. They are grouped by (method, source path) and walked round-robin, one
///    candidate from each group per round, so one large file cannot fill the
///    caps.
/// 3. In walk order a candidate larger than `max_seed_bytes` is `too_large`,
///    one whose bytes were already kept is a `duplicate`, and the first
///    remaining candidate that would push the seed count above `max_seeds` or
///    the total above `max_total_bytes` stops the walk: it and every candidate
///    after it are `over_cap`.
///
/// Every limit is inclusive. A zero-length candidate is an ordinary seed (it
/// costs no bytes and, by the duplicate rule, appears at most once).
/// `unsupported` is the number of candidates the extractors could not decode;
/// it is copied into the result.
#[must_use]
pub fn select(candidates: Vec<Candidate>, caps: &Caps, unsupported: u64) -> Selection {
    let mut ordered = candidates;
    ordered.sort_by(selection_order);

    // Consecutive candidates with the same (method, path) form one group.
    let mut groups: Vec<Vec<Candidate>> = Vec::new();
    for candidate in ordered {
        let same_group = groups.last().and_then(|g| g.last()).is_some_and(|last| {
            last.method == candidate.method && last.source.path == candidate.source.path
        });
        match groups.last_mut() {
            Some(group) if same_group => group.push(candidate),
            _ => groups.push(vec![candidate]),
        }
    }

    let mut cursors: Vec<std::vec::IntoIter<Candidate>> =
        groups.into_iter().map(Vec::into_iter).collect();
    let mut walk: Vec<Candidate> = Vec::new();
    loop {
        let before = walk.len();
        walk.extend(cursors.iter_mut().filter_map(Iterator::next));
        if walk.len() == before {
            break;
        }
    }

    let mut skipped = Skipped {
        unsupported,
        ..Skipped::default()
    };
    let mut seeds: Vec<SelectedSeed> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut total_bytes: u64 = 0;
    let walk_len = walk.len();
    for (position, candidate) in walk.into_iter().enumerate() {
        let len = u64::try_from(candidate.bytes.len()).unwrap_or(u64::MAX);
        if len > caps.max_seed_bytes {
            skipped.too_large = skipped.too_large.saturating_add(1);
            continue;
        }
        let sha256 = sha256_hex(&candidate.bytes);
        if seen.contains(&sha256) {
            skipped.duplicate = skipped.duplicate.saturating_add(1);
            continue;
        }
        let seed_count = u64::try_from(seeds.len()).unwrap_or(u64::MAX);
        if seed_count >= caps.max_seeds || total_bytes.saturating_add(len) > caps.max_total_bytes {
            let remaining = u64::try_from(walk_len - position).unwrap_or(u64::MAX);
            skipped.over_cap = skipped.over_cap.saturating_add(remaining);
            break;
        }
        total_bytes += len;
        seen.insert(sha256.clone());
        seeds.push(SelectedSeed { sha256, candidate });
    }
    Selection { seeds, skipped }
}
