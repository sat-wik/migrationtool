//! Hex-text fixture decoding.

use crate::SeedError;

/// Decode a hex-text fixture to bytes.
pub fn decode_hex_text(text: &str) -> Result<Vec<u8>, SeedError> {
    let _ = text;
    Ok(Vec::new())
}
