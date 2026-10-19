//! C and C++ lexer for constant byte arrays and string literals (D-13).
//!
//! No regex and no recursion: one linear pass turns the source into a flat
//! token stream (comments and skipped preprocessor lines produce no tokens),
//! and a second linear pass over that stream picks out brace groups or runs of
//! string literals.
//!
//! What the lexer does and does not do:
//!
//! - Comments are skipped; braces inside string and character literals are
//!   not braces.
//! - Preprocessor lines are skipped, except that the body of a `#define` is
//!   lexed as code up to the end of its (continued) line. Conditionals are not
//!   evaluated, so data in an inactive `#if` branch is still found.
//! - A brace group is a seed only when it is innermost and every element is
//!   an integer or character literal in `0..=255`, optionally behind a cast to
//!   `uint8_t`, `unsigned char`, `char` or `int8_t`. A group that contains a
//!   `;` is a statement block, not an initialiser, and is not counted.
//! - String literals follow the C escape rules exactly (see [`c_string_literals`]).
//! - [`c_designated_string_literals`] keeps only the strings that are the value
//!   of a designated initialiser (`{ .raw = "..." }`) with a given member name,
//!   for sources whose inputs are named members of test tables.

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

/// A token kind. Whitespace and comments are not tokens.
#[derive(Debug)]
enum Kind<'a> {
    Open,
    Close,
    Comma,
    Semi,
    /// An integer literal; `None` when it is not a valid C integer or overflows `u64`.
    Int(Option<u64>),
    /// A character constant; `None` when it is prefixed, multi-character or malformed.
    Char(Option<u8>),
    /// A string literal; `None` when it is prefixed, raw, unterminated or has a bad escape.
    Str(Option<Vec<u8>>),
    Ident(&'a [u8]),
    Punct(u8),
    /// End of a `#define` body: open groups cannot continue past it.
    Barrier,
}

#[derive(Debug)]
struct Tok<'a> {
    kind: Kind<'a>,
    line: u32,
}

struct Lexer<'a> {
    src: &'a [u8],
    pos: usize,
    line: u32,
    toks: Vec<Tok<'a>>,
}

/// Lex `src` into tokens.
fn tokenize(src: &[u8]) -> Vec<Tok<'_>> {
    let mut lexer = Lexer {
        src,
        pos: 0,
        line: 1,
        toks: Vec::new(),
    };
    lexer.run();
    lexer.toks
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || b == b'$' || b >= 0x80
}

fn is_ident_continue(b: u8) -> bool {
    is_ident_start(b) || b.is_ascii_digit()
}

impl<'a> Lexer<'a> {
    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<u8> {
        self.src.get(self.pos + offset).copied()
    }

    fn push(&mut self, kind: Kind<'a>, line: u32) {
        self.toks.push(Tok { kind, line });
    }

    fn newline(&mut self) {
        self.line = self.line.saturating_add(1);
    }

    /// Length of a backslash-newline splice at the current position (0 if none).
    fn splice_len(&self) -> usize {
        match (self.peek(), self.peek_at(1), self.peek_at(2)) {
            (Some(b'\\'), Some(b'\n'), _) => 2,
            (Some(b'\\'), Some(b'\r'), Some(b'\n')) => 3,
            _ => 0,
        }
    }

    fn run(&mut self) {
        // True while only whitespace and comments have been seen on this logical line.
        let mut line_start = true;
        let mut in_define = false;
        while let Some(c) = self.peek() {
            match c {
                b'\n' => {
                    self.pos += 1;
                    self.newline();
                    line_start = true;
                    if in_define {
                        in_define = false;
                        let line = self.line;
                        self.push(Kind::Barrier, line);
                    }
                }
                b' ' | b'\t' | b'\r' | 0x0b | 0x0c => self.pos += 1,
                b'\\' if self.splice_len() > 0 => {
                    self.pos += self.splice_len();
                    self.newline();
                }
                b'/' if self.peek_at(1) == Some(b'/') => self.skip_line_comment(),
                b'/' if self.peek_at(1) == Some(b'*') => self.skip_block_comment(),
                b'#' if line_start && !in_define => {
                    in_define = self.directive();
                    line_start = false;
                }
                _ => {
                    line_start = false;
                    self.token(c);
                }
            }
        }
        if in_define {
            let line = self.line;
            self.push(Kind::Barrier, line);
        }
    }

    fn skip_line_comment(&mut self) {
        // The terminating newline is left for the main loop.
        while let Some(c) = self.peek() {
            match c {
                b'\n' => break,
                b'\\' if self.splice_len() > 0 => {
                    self.pos += self.splice_len();
                    self.newline();
                }
                _ => self.pos += 1,
            }
        }
    }

    fn skip_block_comment(&mut self) {
        self.pos += 2;
        while let Some(c) = self.peek() {
            if c == b'*' && self.peek_at(1) == Some(b'/') {
                self.pos += 2;
                return;
            }
            if c == b'\n' {
                self.newline();
            }
            self.pos += 1;
        }
    }

    /// Handle a preprocessor directive starting at `#`. Returns true when it is
    /// a `#define`, whose body is then lexed as code by the main loop.
    fn directive(&mut self) -> bool {
        self.pos += 1;
        while matches!(self.peek(), Some(b' ' | b'\t')) {
            self.pos += 1;
        }
        let start = self.pos;
        while self.peek().is_some_and(is_ident_continue) {
            self.pos += 1;
        }
        if &self.src[start..self.pos] == b"define" {
            return true;
        }
        self.skip_directive_rest();
        false
    }

    /// Skip to the end of the logical line, honouring continuations, block
    /// comments and double-quoted strings (so `/*` inside a string is not a comment).
    fn skip_directive_rest(&mut self) {
        while let Some(c) = self.peek() {
            match c {
                b'\n' => break,
                b'\\' if self.splice_len() > 0 => {
                    self.pos += self.splice_len();
                    self.newline();
                }
                b'/' if self.peek_at(1) == Some(b'*') => self.skip_block_comment(),
                b'/' if self.peek_at(1) == Some(b'/') => self.skip_line_comment(),
                b'"' => {
                    self.pos += 1;
                    while let Some(d) = self.peek() {
                        match d {
                            b'\n' => break,
                            b'\\' if self.splice_len() > 0 => {
                                self.pos += self.splice_len();
                                self.newline();
                            }
                            b'\\' => self.pos += 2,
                            b'"' => {
                                self.pos += 1;
                                break;
                            }
                            _ => self.pos += 1,
                        }
                    }
                }
                _ => self.pos += 1,
            }
        }
    }

    /// Lex one token whose first byte is `c` (not whitespace, not a comment).
    fn token(&mut self, c: u8) {
        let line = self.line;
        match c {
            b'{' => {
                self.pos += 1;
                self.push(Kind::Open, line);
            }
            b'}' => {
                self.pos += 1;
                self.push(Kind::Close, line);
            }
            b',' => {
                self.pos += 1;
                self.push(Kind::Comma, line);
            }
            b';' => {
                self.pos += 1;
                self.push(Kind::Semi, line);
            }
            b'"' => {
                let value = self.quoted(b'"', false);
                self.push(Kind::Str(value), line);
            }
            b'\'' => {
                let value = self.quoted(b'\'', false);
                let ch = value.and_then(|v| match v.as_slice() {
                    [one] => Some(*one),
                    _ => None,
                });
                self.push(Kind::Char(ch), line);
            }
            b'0'..=b'9' => {
                let start = self.pos;
                self.pos += 1;
                while let Some(d) = self.peek() {
                    let exponent_sign = matches!(d, b'+' | b'-')
                        && matches!(self.src[self.pos - 1], b'e' | b'E' | b'p' | b'P');
                    if d.is_ascii_alphanumeric() || d == b'_' || d == b'.' || exponent_sign {
                        self.pos += 1;
                    } else {
                        break;
                    }
                }
                let value = parse_int(&self.src[start..self.pos]);
                self.push(Kind::Int(value), line);
            }
            _ if is_ident_start(c) => self.identifier(line),
            _ => {
                self.pos += 1;
                self.push(Kind::Punct(c), line);
            }
        }
    }

    fn identifier(&mut self, line: u32) {
        let start = self.pos;
        while self.peek().is_some_and(is_ident_continue) {
            self.pos += 1;
        }
        let src = self.src;
        let text = &src[start..self.pos];
        match (text, self.peek()) {
            (b"L" | b"u" | b"U" | b"u8", Some(q @ (b'"' | b'\''))) => {
                // Prefixed literal: lexed (so its content is not mistaken for
                // code) and reported as undecodable.
                let _ = self.quoted(q, true);
                if q == b'"' {
                    self.push(Kind::Str(None), line);
                } else {
                    self.push(Kind::Char(None), line);
                }
            }
            (b"R" | b"LR" | b"uR" | b"UR" | b"u8R", Some(b'"')) if self.raw_string() => {
                self.push(Kind::Str(None), line);
            }
            _ => self.push(Kind::Ident(text), line),
        }
    }

    /// Try to consume a raw string literal whose opening quote is at the
    /// current position. Returns false (consuming nothing) when the delimiter
    /// is malformed, so the quote is lexed as an ordinary string.
    fn raw_string(&mut self) -> bool {
        let delim_start = self.pos + 1;
        let mut i = delim_start;
        while let Some(&b) = self.src.get(i) {
            if b == b'(' {
                break;
            }
            let invalid = matches!(b, b' ' | b')' | b'\\' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c);
            if invalid || i - delim_start >= 16 {
                return false;
            }
            i += 1;
        }
        if self.src.get(i) != Some(&b'(') {
            return false;
        }
        let delim = &self.src[delim_start..i];
        let mut j = i + 1;
        // Scan for `)` delim `"`.
        loop {
            match self.src.get(j) {
                None => {
                    j = self.src.len();
                    break;
                }
                Some(&b')') => {
                    let tail = &self.src[j + 1..];
                    if tail.starts_with(delim) && tail.get(delim.len()) == Some(&b'"') {
                        j += 1 + delim.len() + 1;
                        break;
                    }
                    j += 1;
                }
                Some(_) => j += 1,
            }
        }
        for &b in &self.src[self.pos..j] {
            if b == b'\n' {
                self.line = self.line.saturating_add(1);
            }
        }
        self.pos = j;
        true
    }

    /// Lex a quoted literal starting at the opening `quote`. Returns the decoded
    /// bytes, or `None` when the literal is prefixed, unterminated or has an
    /// escape that C does not define. The closing quote is always consumed when
    /// present, so a rejected literal never leaks its content into the code.
    fn quoted(&mut self, quote: u8, prefixed: bool) -> Option<Vec<u8>> {
        self.pos += 1;
        let mut out = Vec::new();
        let mut ok = !prefixed;
        loop {
            match self.peek() {
                None => return None,
                Some(b'\n') => return None,
                Some(b'\r') if self.peek_at(1) == Some(b'\n') => return None,
                Some(c) if c == quote => {
                    self.pos += 1;
                    break;
                }
                Some(b'\\') => {
                    let splice = self.splice_len();
                    if splice > 0 {
                        self.pos += splice;
                        self.newline();
                    } else {
                        match self.escape() {
                            Some(value) => out.push(value),
                            None => ok = false,
                        }
                    }
                }
                Some(c) => {
                    out.push(c);
                    self.pos += 1;
                }
            }
        }
        ok.then_some(out)
    }

    /// Decode the escape sequence at the current backslash. Always advances
    /// past it; `None` when the sequence is unknown or its value exceeds 255.
    fn escape(&mut self) -> Option<u8> {
        self.pos += 1;
        let c = self.peek()?;
        match c {
            b'x' => {
                self.pos += 1;
                let mut digits = 0usize;
                let mut value: u32 = 0;
                while let Some(d) = self.peek().and_then(|b| char::from(b).to_digit(16)) {
                    value = value.saturating_mul(16).saturating_add(d);
                    digits += 1;
                    self.pos += 1;
                }
                if digits == 0 {
                    return None;
                }
                u8::try_from(value).ok()
            }
            b'0'..=b'7' => {
                let mut value: u32 = 0;
                let mut digits = 0;
                while digits < 3 {
                    match self.peek() {
                        Some(d @ b'0'..=b'7') => {
                            value = value * 8 + u32::from(d - b'0');
                            digits += 1;
                            self.pos += 1;
                        }
                        _ => break,
                    }
                }
                u8::try_from(value).ok()
            }
            b'n' | b'r' | b't' | b'\\' | b'\'' | b'"' | b'?' | b'a' | b'b' | b'f' | b'v' => {
                self.pos += 1;
                Some(match c {
                    b'n' => 10,
                    b'r' => 13,
                    b't' => 9,
                    b'a' => 7,
                    b'b' => 8,
                    b'f' => 12,
                    b'v' => 11,
                    other => other,
                })
            }
            _ => {
                // Unknown escape. A newline cannot reach here (splices are
                // handled by the caller) but is never swallowed anyway.
                if c != b'\n' {
                    self.pos += 1;
                }
                None
            }
        }
    }
}

/// Parse a C integer literal (decimal, `0x` hex or leading-zero octal, with
/// any combination of `u`, `U`, `l`, `L` suffix characters).
fn parse_int(text: &[u8]) -> Option<u64> {
    let suffix = text
        .iter()
        .rev()
        .take_while(|b| matches!(b, b'u' | b'U' | b'l' | b'L'))
        .count();
    if suffix > 3 {
        return None;
    }
    let body = &text[..text.len() - suffix];
    let (digits, radix) = match body {
        [b'0', b'x' | b'X', rest @ ..] => (rest, 16),
        [b'0', rest @ ..] if !rest.is_empty() => (rest, 8),
        _ => (body, 10),
    };
    if digits.is_empty() || !digits.iter().all(|&b| char::from(b).is_digit(radix)) {
        return None;
    }
    // The digits are ASCII (checked above), so this conversion cannot fail.
    let text = std::str::from_utf8(digits).ok()?;
    u64::from_str_radix(text, radix).ok()
}

/// An open brace group while scanning for byte arrays.
struct Group<'t, 'a> {
    line: u32,
    kinds: Vec<&'t Kind<'a>>,
    has_child: bool,
    has_statement: bool,
}

enum Evaluated {
    Empty,
    Bytes(Vec<u8>),
    Unsupported,
}

fn cast_is_byte_type(idents: &[&Kind<'_>]) -> bool {
    match idents {
        [Kind::Ident(a)] => matches!(*a, b"uint8_t" | b"char" | b"int8_t"),
        [Kind::Ident(a), Kind::Ident(b)] => *a == b"unsigned" && *b == b"char",
        _ => false,
    }
}

/// The byte value of one initialiser element, if it is a plain literal in 0..=255.
fn element_value(element: &[&Kind<'_>]) -> Option<u8> {
    let literal = match element {
        [literal] => literal,
        [Kind::Punct(b'('), cast @ .., Kind::Punct(b')'), literal] if cast_is_byte_type(cast) => {
            literal
        }
        _ => return None,
    };
    match literal {
        Kind::Int(Some(value)) => u8::try_from(*value).ok(),
        Kind::Char(Some(value)) => Some(*value),
        _ => None,
    }
}

fn evaluate(kinds: &[&Kind<'_>]) -> Evaluated {
    if kinds.is_empty() {
        return Evaluated::Empty;
    }
    let mut elements: Vec<&[&Kind<'_>]> = kinds.split(|kind| matches!(kind, Kind::Comma)).collect();
    // One trailing comma is allowed.
    if elements.len() > 1 && elements.last().is_some_and(|last| last.is_empty()) {
        elements.pop();
    }
    let mut bytes = Vec::with_capacity(elements.len());
    for element in elements {
        match element_value(element) {
            Some(value) => bytes.push(value),
            None => return Evaluated::Unsupported,
        }
    }
    Evaluated::Bytes(bytes)
}

/// Extract constant byte arrays: innermost brace groups whose elements are all
/// integer or character literals in `0..=255` (decimal, hex, octal, integer
/// suffixes `u` `U` `l` `L`, C escapes in character constants, an optional cast
/// to `uint8_t`, `unsigned char`, `char` or `int8_t`, one trailing comma).
///
/// A group with an identifier, a negative or too-large value, a string or any
/// other element is skipped and counted in `unsupported`. Empty groups and
/// statement blocks (groups containing `;`) are neither.
pub fn c_byte_arrays<S: AsRef<[u8]> + ?Sized>(src: &S) -> LexResult {
    let toks = tokenize(src.as_ref());
    let mut result = LexResult::default();
    let mut stack: Vec<Group<'_, '_>> = Vec::new();
    for tok in &toks {
        match &tok.kind {
            Kind::Open => {
                if let Some(parent) = stack.last_mut() {
                    parent.has_child = true;
                }
                stack.push(Group {
                    line: tok.line,
                    kinds: Vec::new(),
                    has_child: false,
                    has_statement: false,
                });
            }
            Kind::Close => {
                if let Some(group) = stack.pop()
                    && !group.has_child
                    && !group.has_statement
                {
                    match evaluate(&group.kinds) {
                        Evaluated::Empty => {}
                        Evaluated::Bytes(bytes) => result.found.push(Extracted {
                            bytes,
                            line: group.line,
                        }),
                        Evaluated::Unsupported => {
                            result.unsupported = result.unsupported.saturating_add(1);
                        }
                    }
                }
            }
            Kind::Barrier => stack.clear(),
            Kind::Semi => {
                if let Some(group) = stack.last_mut() {
                    group.has_statement = true;
                }
            }
            other => {
                if let Some(group) = stack.last_mut() {
                    group.kinds.push(other);
                }
            }
        }
    }
    result
}

/// Extract string literals, concatenating adjacent ordinary literals
/// (separated only by whitespace and comments) and decoding escapes as C does:
/// `\x` consumes every following hex digit and a value above 255 rejects the
/// literal, octal escapes take up to three digits, `\n \r \t \\ \' \" \? \a \b
/// \f \v` map to their bytes, and any other escape rejects the literal. The
/// terminating NUL is not part of the value.
///
/// Prefixed literals (`L`, `u`, `U`, `u8`) and raw strings are skipped, and a
/// run containing one is counted once in `unsupported`, as is a run with a
/// rejected literal. `extern "C"` linkage strings are ignored.
pub fn c_string_literals<S: AsRef<[u8]> + ?Sized>(src: &S) -> LexResult {
    string_runs(&tokenize(src.as_ref()), |_, _| true)
}

/// Extract the string literals that are the value of a designated initialiser
/// `.name = "..."` with `name` one of `designators`, and only those.
///
/// A run qualifies when the four tokens before it are `{` or `,`, then `.`,
/// then an identifier in `designators`, then `=`. So `{ .raw = "a" "b", .n = 1 }`
/// yields `ab` for `raw`, while `m->raw = "x"`, `s.raw = "y"`, `.rawx = "z"`,
/// a printf message and a run behind any other member are not seeds. Runs are
/// concatenated and decoded exactly as in [`c_string_literals`]; a qualifying
/// run that cannot be decoded is counted once in `unsupported`, and a run that
/// does not qualify is not counted at all. An empty `designators` yields nothing.
pub fn c_designated_string_literals<S: AsRef<[u8]> + ?Sized>(
    src: &S,
    designators: &[&str],
) -> LexResult {
    string_runs(&tokenize(src.as_ref()), |toks, start| {
        matches!(
            &toks[..start],
            [
                ..,
                Tok {
                    kind: Kind::Open | Kind::Comma,
                    ..
                },
                Tok {
                    kind: Kind::Punct(b'.'),
                    ..
                },
                Tok {
                    kind: Kind::Ident(name),
                    ..
                },
                Tok {
                    kind: Kind::Punct(b'='),
                    ..
                },
            ] if designators.iter().any(|d| d.as_bytes() == *name)
        )
    })
}

/// Collect the runs of adjacent string literals of `toks` that `keep` accepts
/// (it gets the token stream and the index of the run's first token). Linkage
/// strings of `extern "C"` are never kept.
fn string_runs(toks: &[Tok<'_>], keep: impl Fn(&[Tok<'_>], usize) -> bool) -> LexResult {
    let mut result = LexResult::default();
    let mut i = 0;
    while i < toks.len() {
        if !matches!(toks[i].kind, Kind::Str(_)) {
            i += 1;
            continue;
        }
        let start = i;
        while i < toks.len() && matches!(toks[i].kind, Kind::Str(_)) {
            i += 1;
        }
        let linkage = start > 0 && matches!(toks[start - 1].kind, Kind::Ident(b"extern"));
        if linkage || !keep(toks, start) {
            continue;
        }
        let mut bytes = Vec::new();
        let mut decodable = true;
        for tok in &toks[start..i] {
            match &tok.kind {
                Kind::Str(Some(part)) => bytes.extend_from_slice(part),
                _ => decodable = false,
            }
        }
        if decodable {
            result.found.push(Extracted {
                bytes,
                line: toks[start].line,
            });
        } else {
            result.unsupported = result.unsupported.saturating_add(1);
        }
    }
    result
}
