//! The byte codec of the field convention (D-25): Latin-1 identity.
//!
//! Test vectors keep binary data in the text fields `stdin` and
//! `stdout.pattern`. Byte `b` becomes the character U+00`b`. The mapping is
//! lossless for all 256 byte values (NUL included), and the same function pair
//! is used everywhere in the tool. JSON escaping of control characters, `"` and
//! `\` is the JSON writer's job and does not change the characters.

/// Why a text field is not a byte string.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CodecError {
    /// A character above U+00FF cannot stand for a byte.
    #[error(
        "character U+{:04X} at position {index} is above U+00FF and is not a byte",
        u32::from(*.ch)
    )]
    WideChar {
        /// Index of the character in the text, counted in characters.
        index: usize,
        /// The offending character.
        ch: char,
    },
}

/// The text for `bytes`: each byte becomes the character with the same code.
#[must_use]
pub fn encode_latin1(bytes: &[u8]) -> String {
    bytes.iter().copied().map(char::from).collect()
}

/// The bytes for `text`: each character is a byte, or an error naming the
/// position of the first character above U+00FF.
pub fn decode_latin1(text: &str) -> Result<Vec<u8>, CodecError> {
    let mut bytes = Vec::with_capacity(text.len());
    for (index, ch) in text.chars().enumerate() {
        match u8::try_from(u32::from(ch)) {
            Ok(byte) => bytes.push(byte),
            Err(_) => return Err(CodecError::WideChar { index, ch }),
        }
    }
    Ok(bytes)
}
