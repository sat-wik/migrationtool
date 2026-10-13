//! BUILD-03: the C lexer and the hex-text decoder (D-13).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use mt_seeds::hex::decode_hex_text;
use mt_seeds::lexer::{LexResult, c_byte_arrays, c_string_literals};

/// The decoded bytes of every found value, in order.
fn values(result: &LexResult) -> Vec<Vec<u8>> {
    result.found.iter().map(|e| e.bytes.clone()).collect()
}

/// The start line of every found value, in order.
fn lines(result: &LexResult) -> Vec<u32> {
    result.found.iter().map(|e| e.line).collect()
}

#[test]
fn build_03_c_byte_array_extraction_follows_c_escape_rules() {
    // Integer forms, character constants, a cast, suffixes, line number.
    let r = c_byte_arrays(
        r"static const uint8_t a[] = { 0x01, 2, 0303, 'A', '\n', (uint8_t)0xff, 7u };",
    );
    assert_eq!(values(&r), vec![vec![1, 2, 195, 65, 10, 255, 7]]);
    assert_eq!(lines(&r), vec![1]);
    assert_eq!(r.unsupported, 0);

    // Groups with an identifier, a negative value or a value above 255 are
    // skipped and counted.
    for src in [
        "{ x, 1 }",
        "{ -1 }",
        "{ 256 }",
        "{ 1, , 2 }",
        "{ 089 }",
        "{ 1.5 }",
    ] {
        let r = c_byte_arrays(src);
        assert!(r.found.is_empty(), "{src} must not yield a seed");
        assert_eq!(r.unsupported, 1, "{src} must be counted unsupported");
    }

    // Trailing comma, suffix combinations, the accepted casts, octal, zero, escapes.
    let r = c_byte_arrays(
        r"{1,2,} {1UL, 2llu, 0x10lu} {(unsigned char)3, (char)4, (int8_t)5} {010} {0} {'\x41', '\101', '\\', '\''}",
    );
    assert_eq!(
        values(&r),
        vec![
            vec![1, 2],
            vec![1, 2, 16],
            vec![3, 4, 5],
            vec![8],
            vec![0],
            vec![0x41, 0x41, 0x5c, 0x27],
        ]
    );
    assert_eq!(r.unsupported, 0);

    // An empty initialiser is not a seed and is not counted.
    let r = c_byte_arrays("void f(void) {}");
    assert!(r.found.is_empty());
    assert_eq!(r.unsupported, 0);

    // Only innermost groups count; the outer group is not an array.
    let r = c_byte_arrays("{ {1, 2}, {3} }");
    assert_eq!(values(&r), vec![vec![1, 2], vec![3]]);
    assert_eq!(r.unsupported, 0);

    // Preprocessor lines are ignored except for #define bodies.
    let r = c_byte_arrays("#define V { 1, 2 }\n#pragma foo { 9 }\n#include <x{y>\n");
    assert_eq!(values(&r), vec![vec![1, 2]]);
    let r = c_byte_arrays("#define W { 3, \\\n 4 }\nint x;\n");
    assert_eq!(values(&r), vec![vec![3, 4]]);
    // An unbalanced define body cannot leak into the code that follows it.
    let r = c_byte_arrays("#define OPEN {\n 1, 2 }\nuint8_t a[] = { 7 };\n");
    assert_eq!(values(&r), vec![vec![7]]);

    // Statement blocks are code, not initialisers: neither seeds nor counted.
    let r = c_byte_arrays("int f(void) { int x = 1; return x; }");
    assert!(r.found.is_empty());
    assert_eq!(r.unsupported, 0);

    // Strings.
    // A \x escape consumes every following hex digit: 0x41BC is above 255.
    let r = c_string_literals(r#""\x41BC""#);
    assert!(r.found.is_empty());
    assert_eq!(r.unsupported, 1);
    // Adjacent literals concatenate, so \x41 stops at the literal boundary.
    let r = c_string_literals(r#""\x41" "BC""#);
    assert_eq!(values(&r), vec![vec![0x41, 0x42, 0x43]]);
    // Octal takes up to three digits; the NUL is data, the terminator is not.
    let r = c_string_literals(r#""\101\0z""#);
    assert_eq!(values(&r), vec![vec![65, 0, 122]]);
    let r = c_string_literals(r#""\1012""#);
    assert_eq!(values(&r), vec![vec![65, b'2']]);
    // Octal above 255 rejects the literal.
    let r = c_string_literals(r#""\400""#);
    assert!(r.found.is_empty());
    assert_eq!(r.unsupported, 1);
    // The simple escapes.
    let r = c_string_literals(r#""\n\r\t\\\'\"\?\a\b\f\v""#);
    assert_eq!(
        values(&r),
        vec![vec![10, 13, 9, 92, 39, 34, 63, 7, 8, 12, 11]]
    );
    // An unknown escape rejects the literal.
    let r = c_string_literals(r#""\q""#);
    assert!(r.found.is_empty());
    assert_eq!(r.unsupported, 1);
    // An empty literal is a valid, empty seed.
    let r = c_string_literals(r#""""#);
    assert_eq!(values(&r), vec![Vec::<u8>::new()]);
    // Literals split across lines and comments concatenate.
    let r = c_string_literals("\"ab\" /* c\nd */\n // e\n \"cd\" ;\n\"ef\"");
    assert_eq!(values(&r), vec![b"abcd".to_vec(), b"ef".to_vec()]);
    // Non-ASCII bytes pass through unchanged.
    let r = c_string_literals(b"\"\xc3\xa9\xff\"");
    assert_eq!(values(&r), vec![vec![0xc3, 0xa9, 0xff]]);

    // Prefixed and raw literals are skipped and counted; one run, one count.
    for src in [
        r#"L"x""#,
        r#"u8"x""#,
        r#"u"x""#,
        r#"U"x""#,
        r#"R"(x)""#,
        r#"u8R"d(x)d""#,
    ] {
        let r = c_string_literals(src);
        assert!(r.found.is_empty(), "{src} must be skipped");
        assert_eq!(r.unsupported, 1, "{src} must be counted");
    }
    let r = c_string_literals(r#""a" L"b" "c""#);
    assert!(r.found.is_empty());
    assert_eq!(r.unsupported, 1);
    // A raw string's content is not lexed as code.
    let r = c_string_literals("R\"x(\" } ')x\"; \"after\"");
    assert_eq!(values(&r), vec![b"after".to_vec()]);
    assert_eq!(r.unsupported, 1);
    // extern "C" names a linkage, not data.
    let r = c_string_literals("extern \"C\" { int f(void); }");
    assert!(r.found.is_empty());
    assert_eq!(r.unsupported, 0);
}

#[test]
fn build_03_lexer_ignores_braces_in_comments_and_strings() {
    let src = r#"
/* { 1 } */ // { 2 }
const char *s = "{ 3 }";
char c = '{';
const char *t = "}";
const char *u = "a//b";
uint8_t a[] = {4};
"#;
    let arrays = c_byte_arrays(src);
    assert_eq!(values(&arrays), vec![vec![4]]);
    assert_eq!(arrays.unsupported, 0);

    let strings = c_string_literals(src);
    assert_eq!(
        values(&strings),
        vec![b"{ 3 }".to_vec(), b"}".to_vec(), b"a//b".to_vec()]
    );

    // Comment markers inside a character constant or string do not start a comment.
    let r = c_byte_arrays("char c[] = {'/', '*'}; /* x */ uint8_t b[] = {9};");
    assert_eq!(values(&r), vec![vec![0x2f, 0x2a], vec![9]]);

    // An apostrophe in a skipped preprocessor line or a comment does not open a literal.
    let r = c_byte_arrays("#error don't\n// it's\nuint8_t b[] = {9};");
    assert_eq!(values(&r), vec![vec![9]]);
}

#[test]
fn build_03_lexer_reports_line_numbers() {
    let src = "int a;\n\
/* two\n\
   lines */\n\
uint8_t x[] = { 1 };\n\
#define D \\\n\
  { 2 }\n\
const char *s = \"p\\\n\
q\";\n\
const char *t = \"r\"\n\
  \"s\";\n\
uint8_t y[] = {\n\
  3,\n\
  4 };\n";
    let arrays = c_byte_arrays(src);
    assert_eq!(values(&arrays), vec![vec![1], vec![2], vec![3, 4]]);
    assert_eq!(lines(&arrays), vec![4, 6, 11]);

    let strings = c_string_literals(src);
    assert_eq!(values(&strings), vec![b"pq".to_vec(), b"rs".to_vec()]);
    assert_eq!(lines(&strings), vec![7, 9]);

    // CRLF line endings count once.
    let r = c_byte_arrays("a\r\nb\r\n{ 5 }");
    assert_eq!(lines(&r), vec![3]);
}

#[test]
fn build_03_lexer_handles_pathological_input_without_recursion() {
    // Deep nesting, a long unterminated string and an unterminated comment.
    let deep = "{".repeat(300_000);
    let r = c_byte_arrays(&deep);
    assert!(r.found.is_empty());

    let r = c_string_literals(&format!("\"{}", "a".repeat(500_000)));
    assert!(r.found.is_empty());
    assert_eq!(r.unsupported, 1);

    let r = c_byte_arrays("/* never closed { 1 }");
    assert!(r.found.is_empty());
    assert_eq!(r.unsupported, 0);

    // Many small groups stay linear and ordered.
    let many = "{1}".repeat(50_000);
    let r = c_byte_arrays(&many);
    assert_eq!(r.found.len(), 50_000);
}

#[test]
fn build_03_hex_text_decoding_rejects_malformed_input() {
    assert_eq!(
        decode_hex_text("# comment\n0x01 02\nA0").unwrap(),
        vec![1, 2, 160]
    );
    assert_eq!(
        decode_hex_text("de ad\nbe ef # tail\n0X0A").unwrap(),
        vec![0xde, 0xad, 0xbe, 0xef, 0x0a]
    );
    assert_eq!(decode_hex_text("").unwrap(), Vec::<u8>::new());
    assert_eq!(
        decode_hex_text("# only a comment\n").unwrap(),
        Vec::<u8>::new()
    );

    for bad in [
        "0x1", "zz", "abc", "0x", "0x1 0x2", "01 g2", "01,02", "0x0x01",
    ] {
        assert!(decode_hex_text(bad).is_err(), "{bad:?} must be rejected");
    }
}
