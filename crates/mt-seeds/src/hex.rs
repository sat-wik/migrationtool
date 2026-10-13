//! Hex-text fixture decoding (zcbor `.cborhex` files, hand-made `.hex` seeds).

use crate::SeedError;

/// Decode a hex-text fixture to bytes.
///
/// `#` starts a comment that runs to the end of the line. The rest is split on
/// ASCII whitespace into tokens; a token may start with `0x` or `0X`. Every
/// other character must be a hex digit. The digits of all tokens are joined
/// and read in pairs, so the total digit count must be even; a token with a
/// `0x` prefix stands for one value and must itself hold an even count, so
/// `0x1 0x2` is rejected rather than read as `0x12`.
///
/// An empty file (or one with only comments) decodes to no bytes.
pub fn decode_hex_text(text: &str) -> Result<Vec<u8>, SeedError> {
    let mut digits: Vec<u8> = Vec::new();
    for line in text.split('\n') {
        let content = line.split('#').next().unwrap_or_default();
        for token in content.split_ascii_whitespace() {
            let (body, prefixed) = match token
                .strip_prefix("0x")
                .or_else(|| token.strip_prefix("0X"))
            {
                Some(rest) => (rest, true),
                None => (token, false),
            };
            if prefixed && body.is_empty() {
                return Err(SeedError::HexText(format!("`{token}` has no digits")));
            }
            if prefixed && !body.len().is_multiple_of(2) {
                return Err(SeedError::HexText(format!(
                    "`{token}` has an odd number of digits"
                )));
            }
            for ch in body.chars() {
                match ch.to_digit(16) {
                    // to_digit(16) is below 16, so the narrowing cannot truncate.
                    Some(value) => digits.push(u8::try_from(value).unwrap_or_default()),
                    None => {
                        return Err(SeedError::HexText(format!(
                            "unexpected character {ch:?} in `{token}`"
                        )));
                    }
                }
            }
        }
    }
    if !digits.len().is_multiple_of(2) {
        return Err(SeedError::HexText(format!(
            "odd number of hex digits ({})",
            digits.len()
        )));
    }
    // The count is even (checked above), so there is no remainder.
    let (pairs, _) = digits.as_chunks::<2>();
    Ok(pairs.iter().map(|[high, low]| (high << 4) | low).collect())
}
