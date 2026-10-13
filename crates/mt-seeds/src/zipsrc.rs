//! Zip members as seed candidates, read in memory only (T-02-30, T-02-31).

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

/// Read the regular members of the zip archive in `bytes`.
pub fn zip_members(bytes: &[u8], max_member_bytes: u64) -> Result<ZipResult, SeedError> {
    let _ = (bytes, max_member_bytes);
    Ok(ZipResult::default())
}
