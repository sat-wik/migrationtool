//! Zip members as seed candidates, read in memory only (T-02-30, T-02-31).
//!
//! Member names are taken solely from `enclosed_name()`; a name that is
//! absolute, escapes the archive root or holds a NUL is not enclosed and the
//! member is counted unsupported. No path is ever built from a member name and
//! nothing is written to disk: the caller gets names and bytes.
//!
//! Reads are bounded twice (T-02-32, T-02-47): no member is read past the
//! caller's per-member limit, and no archive is read past the caller's total
//! budget, so a small archive cannot expand to gigabytes in memory (WR-07).
//! Members are inflated by flate2's `rust_backend`, miniz_oxide, which forbids
//! unsafe code. The reading stack as a whole is not safe Rust: zip's header
//! parsing and flate2's buffer code contain unsafe code, as does simd-adler32
//! (its SIMD intrinsics; not checked whether zip's raw-deflate path calls it).

use std::io::{Cursor, Read};
use std::path::Component;

use zip::read::ZipFile;

use crate::SeedError;

/// One regular member of a zip archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZipMember {
    /// The member name, from `enclosed_name()`, with `/` separators.
    pub name: String,
    /// The decompressed bytes (at most the caller's limit).
    pub bytes: Vec<u8>,
}

/// What reading an archive produced.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ZipResult {
    /// Kept members in archive order.
    pub members: Vec<ZipMember>,
    /// Members skipped because their name is not enclosed or they cannot be read.
    pub unsupported: u64,
    /// Members skipped because they are larger than the per-member limit, and
    /// members refused because reading them would pass the archive's total
    /// budget.
    pub too_large: u64,
}

/// The enclosed name of `file` as `/`-separated UTF-8, if it has one.
fn member_name(file: &ZipFile<'_, Cursor<&[u8]>>) -> Option<String> {
    let path = file.enclosed_name()?;
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_str()?),
            Component::CurDir => {}
            _ => return None,
        }
    }
    if parts.is_empty() {
        return None;
    }
    let name = parts.join("/");
    // `enclosed_name()` is lenient: it makes a leading `/` relative and folds
    // `a/../b` into `b`. A seed's recorded provenance must be the member's
    // real name, and two members must never collapse onto one name, so any
    // member whose enclosed form differs from its stored name is unsupported.
    let stored = file.name().ok()?;
    if stored != name {
        return None;
    }
    Some(name)
}

/// Read the regular members of the zip archive in `bytes`.
///
/// Members come back in archive order. Directories are skipped. A member
/// larger than `max_member_bytes` (by its declared size, and again by a read
/// bounded at `max_member_bytes + 1` so a lying header cannot force a large
/// allocation) is counted in `too_large`; a member that cannot be opened or
/// read (encrypted, unsupported compression, corrupt) or whose name is not
/// enclosed is counted in `unsupported`. Only an unreadable archive is an error.
///
/// `max_total_bytes` is the budget of decompressed bytes read from this
/// archive, spent in archive order. A member whose declared size is more than
/// what is left is counted in `too_large` without being read; a member that
/// fits exactly is kept. Each read is bounded at one byte past the smaller of
/// the member limit and what is left, so a header that understates its size
/// is caught too: that member is counted in `too_large` and the bytes already
/// read stay spent. Every byte read counts, so the bytes read from one archive
/// never pass the budget by more than that one byte.
pub fn zip_members(
    bytes: &[u8],
    max_member_bytes: u64,
    max_total_bytes: u64,
) -> Result<ZipResult, SeedError> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))?;
    let mut result = ZipResult::default();
    let mut total_read: u64 = 0;
    for index in 0..archive.len() {
        let Ok(mut file) = archive.by_index(index) else {
            result.unsupported = result.unsupported.saturating_add(1);
            continue;
        };
        if file.is_dir() {
            continue;
        }
        let Some(name) = member_name(&file) else {
            result.unsupported = result.unsupported.saturating_add(1);
            continue;
        };
        let remaining = max_total_bytes.saturating_sub(total_read);
        if file.size() > max_member_bytes || file.size() > remaining {
            result.too_large = result.too_large.saturating_add(1);
            continue;
        }
        let allowed = max_member_bytes.min(remaining);
        let mut content = Vec::new();
        let read = (&mut file)
            .take(allowed.saturating_add(1))
            .read_to_end(&mut content);
        let read_len = u64::try_from(content.len()).unwrap_or(u64::MAX);
        total_read = total_read.saturating_add(read_len);
        if read.is_err() {
            result.unsupported = result.unsupported.saturating_add(1);
            continue;
        }
        if read_len > allowed {
            result.too_large = result.too_large.saturating_add(1);
            continue;
        }
        result.members.push(ZipMember {
            name,
            bytes: content,
        });
    }
    Ok(result)
}
