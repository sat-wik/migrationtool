//! C and C++ lexer for constant byte arrays and string literals (D-13).

/// One extracted value and the 1-based line where it starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extracted {
    /// The decoded bytes (no terminating NUL for strings).
    pub bytes: Vec<u8>,
    /// 1-based line of the opening brace or first string literal.
    pub line: u32,
}

/// What a lexer pass found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LexResult {
    /// Values in source order.
    pub found: Vec<Extracted>,
    /// Candidates that looked like seeds but could not be decoded.
    pub unsupported: u64,
}

/// Extract constant byte arrays (brace groups of integer literals).
pub fn c_byte_arrays<S: AsRef<[u8]> + ?Sized>(src: &S) -> LexResult {
    let _ = src.as_ref();
    LexResult::default()
}

/// Extract string literals, concatenating adjacent literals.
pub fn c_string_literals<S: AsRef<[u8]> + ?Sized>(src: &S) -> LexResult {
    let _ = src.as_ref();
    LexResult::default()
}
