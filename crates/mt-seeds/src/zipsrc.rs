//! Zip members as seed candidates, read in memory only (T-02-30, T-02-31).
//!
//! Member names are taken solely from `enclosed_name()`; a name that is
//! absolute, escapes the archive root or holds a NUL is not enclosed and the
//! member is counted unsupported. No path is ever built from a member name and
//! nothing is written to disk: the caller gets names and bytes.

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
    /// Members skipped because they are larger than the limit.
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
pub fn zip_members(bytes: &[u8], max_member_bytes: u64) -> Result<ZipResult, SeedError> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))?;
    let mut result = ZipResult::default();
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
        if file.size() > max_member_bytes {
            result.too_large = result.too_large.saturating_add(1);
            continue;
        }
        let mut content = Vec::new();
        let limit = max_member_bytes.saturating_add(1);
        if (&mut file).take(limit).read_to_end(&mut content).is_err() {
            result.unsupported = result.unsupported.saturating_add(1);
            continue;
        }
        if u64::try_from(content.len()).is_ok_and(|len| len > max_member_bytes) {
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
