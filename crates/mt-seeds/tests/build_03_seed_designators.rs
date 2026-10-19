//! BUILD-03: string literals selected by the designated initialiser they are the
//! value of (G-02-3). httpparser's test inputs are the `.raw` members of its
//! message arrays; the rest of its test file is names, header fields and messages.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use mt_seeds::lexer::{LexResult, c_designated_string_literals, c_string_literals};

/// The decoded bytes of every found value, in order.
fn values(result: &LexResult) -> Vec<Vec<u8>> {
    result.found.iter().map(|e| e.bytes.clone()).collect()
}

/// The start line of every found value, in order.
fn lines(result: &LexResult) -> Vec<u32> {
    result.found.iter().map(|e| e.line).collect()
}

/// Seven lines: two designated `raw` runs (the first spans a comment and two
/// continuation literals), a `name`, a `body`, a printf message, two member
/// assignments, a longer member name and a `NULL` value.
const MESSAGES: &str = r#"static const struct message m[] = {
  { .name= "curl get", .raw= "GET / HTTP/1.1\r\n" /* c */ "Host: x\r\n"
    "\r\n", .body= "" },
  { .raw = "POST / HTTP/1.0\r\n\r\n" }
};
void f(void) { printf("error %d\n", 1); m->raw = "x"; s.raw = "y"; }
struct t u = { .rawx = "z", .raw = NULL };
"#;

#[test]
fn build_03_c_strings_after_designator_only() {
    let result = c_designated_string_literals(MESSAGES, &["raw"]);
    assert_eq!(
        values(&result),
        vec![
            b"GET / HTTP/1.1\r\nHost: x\r\n\r\n".to_vec(),
            b"POST / HTTP/1.0\r\n\r\n".to_vec(),
        ],
        "only the values of `.raw =` directly after `{{` or `,`: not `.name`, \
         `.body`, `m->raw`, `s.raw`, `.rawx`, nor the printf message"
    );
    assert_eq!(lines(&result), vec![2, 4]);
    assert_eq!(
        result.unsupported, 0,
        "runs that are not designated are not counted"
    );

    // Another designator selects another member, an empty one included.
    let body = c_designated_string_literals(MESSAGES, &["body"]);
    assert_eq!(values(&body), vec![Vec::<u8>::new()]);
    assert_eq!(lines(&body), vec![3]);

    // Several designators select the union, in source order.
    let both = c_designated_string_literals(MESSAGES, &["name", "raw"]);
    assert_eq!(
        values(&both),
        vec![
            b"curl get".to_vec(),
            b"GET / HTTP/1.1\r\nHost: x\r\n\r\n".to_vec(),
            b"POST / HTTP/1.0\r\n\r\n".to_vec(),
        ]
    );

    // No designator selects nothing.
    let none = c_designated_string_literals(MESSAGES, &[]);
    assert!(none.found.is_empty());
    assert_eq!(none.unsupported, 0);

    // A designated run that cannot be decoded counts once; an undecodable run
    // that is not designated is not counted at all.
    let result = c_designated_string_literals(
        r#"struct m a[] = { { .raw = "\q" }, { .name = "\q" }, { .raw = L"w" }, { .raw = "ok" } };"#,
        &["raw"],
    );
    assert_eq!(values(&result), vec![b"ok".to_vec()]);
    assert_eq!(result.unsupported, 2);

    // A designator inside a nested initialiser after a comma still counts.
    let result = c_designated_string_literals(
        r#"struct m a = { .x = 1, .raw = "a" "b", .y = { .raw = "c" } };"#,
        &["raw"],
    );
    assert_eq!(values(&result), vec![b"ab".to_vec(), b"c".to_vec()]);

    // The linkage string of `extern "C"` is never a value.
    let result = c_designated_string_literals(r#"extern "C" { int x; }"#, &["raw"]);
    assert!(result.found.is_empty());
    assert_eq!(result.unsupported, 0);
}

#[test]
fn build_03_c_string_literals_unchanged_without_designators() {
    let result = c_string_literals(MESSAGES);
    let expected: Vec<Vec<u8>> = vec![
        b"curl get".to_vec(),
        b"GET / HTTP/1.1\r\nHost: x\r\n\r\n".to_vec(),
        b"".to_vec(),
        b"POST / HTTP/1.0\r\n\r\n".to_vec(),
        b"error %d\n".to_vec(),
        b"x".to_vec(),
        b"y".to_vec(),
        b"z".to_vec(),
    ];
    assert_eq!(values(&result), expected, "every literal run is a string");
    assert_eq!(lines(&result), vec![2, 2, 3, 4, 6, 6, 6, 7]);
    assert_eq!(result.unsupported, 0);

    let result = c_string_literals(r#"extern "C" { const char *a = "\q"; const char *b = "ok"; }"#);
    assert_eq!(values(&result), vec![b"ok".to_vec()]);
    assert_eq!(result.unsupported, 1);
}
