//! The shared runtime template and its renderer (CONTEXT D-29, D-30).
//!
//! `runtime.c` is one reviewed file for every target. The harness turns it into
//! a driver by replacing a closed set of placeholders written
//! `@MT_<NAME>@`. The renderer replaces each exactly once and fails on any other
//! `@MT_` token or any token left over, so a generated fragment can never smuggle
//! a placeholder in.

use crate::HarnessError;
use crate::budget::Budgets;

/// The template source, compiled into the binary.
pub const RUNTIME_TEMPLATE: &str = include_str!("runtime.c");

/// The names of the closed placeholder set, in the order they are replaced.
pub const PLACEHOLDER_NAMES: [&str; 9] = [
    "MT_RESERVED_VA",
    "MT_HEAP_ARENA",
    "MT_SENTINEL",
    "MT_SLACK_FILL",
    "MT_MAX_INPUT",
    "MT_INCLUDES",
    "MT_STATES",
    "MT_CASES",
    "MT_CALL",
];

/// The call expression of the self-test, which dispatches on the request's
/// selector word.
pub const SELFTEST_CALL: &str = "mt_call(selector, mt_in, mt_in_len, mt_out, mt_out_cap)";

/// The states fragment of a driver with no state.
pub const NO_STATES: &str = "#define MT_STATE_COUNT 0u\nstatic const u32 mt_state_sizes[MT_STATE_COUNT + 1u] = { 0u };\nstatic const int mt_state_word[MT_STATE_COUNT + 1u] = { -1 };";

/// The values written into the template.
#[derive(Debug, Clone)]
pub struct Placeholders {
    /// Reserved guest address space; mappings must lie below it.
    pub reserved_va: u32,
    /// Bytes of the `_sbrk` arena.
    pub heap_arena: u32,
    /// Output prefill byte.
    pub sentinel: u8,
    /// Slack pattern byte.
    pub slack_fill: u8,
    /// Largest request input in bytes.
    pub max_input: u32,
    /// The `#include`, `#define` and `#undef` lines an entry driver starts
    /// with, one per line; empty for the self-test.
    pub includes: String,
    /// The entry's states: `#define MT_STATE_COUNT <n>u`, the array
    /// `mt_state_sizes` (`sizeof` of each state type followed by `0u`) and the
    /// array `mt_state_word` (the request word that holds each state's value
    /// before the call, or -1).
    pub states: String,
    /// C source of `static u32 mt_call(u32, u8 *, u32, u8 *, u32)`, the cases.
    pub cases: String,
    /// The C expression of the call, of a type convertible to `u32`. It may
    /// read `selector`, `mt_in`, `mt_in_len`, `mt_out`, `mt_out_cap` and
    /// `mt_scalar(index)`.
    pub call: String,
}

impl Placeholders {
    /// The placeholders for `budgets` and a generated cases fragment: the
    /// self-test's, with no entry includes and the selector-dispatch call.
    #[must_use]
    pub fn from_budgets(budgets: &Budgets, cases: String) -> Self {
        Self {
            reserved_va: budgets.reserved_va,
            heap_arena: budgets.heap_arena,
            sentinel: budgets.sentinel,
            slack_fill: budgets.slack_fill,
            max_input: budgets.max_input,
            includes: String::new(),
            states: NO_STATES.to_owned(),
            cases,
            call: SELFTEST_CALL.to_owned(),
        }
    }

    /// The placeholders of an entry driver: the entry's include lines, states
    /// and call expression, and no cases.
    #[must_use]
    pub fn for_entry(budgets: &Budgets, includes: String, states: String, call: String) -> Self {
        Self {
            includes,
            states,
            call,
            ..Self::from_budgets(budgets, String::new())
        }
    }

    fn value(&self, name: &str) -> String {
        match name {
            "MT_RESERVED_VA" => format!("{:#x}", self.reserved_va),
            "MT_HEAP_ARENA" => self.heap_arena.to_string(),
            "MT_SENTINEL" => self.sentinel.to_string(),
            "MT_SLACK_FILL" => self.slack_fill.to_string(),
            "MT_MAX_INPUT" => self.max_input.to_string(),
            "MT_INCLUDES" => self.includes.clone(),
            "MT_STATES" => self.states.clone(),
            "MT_CALL" => self.call.clone(),
            _ => self.cases.clone(),
        }
    }
}

/// Every `@MT_...@` token in `text`, as the part between the at signs.
fn tokens(text: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("@MT_") {
        let after = &rest[start + 1..];
        match after.find('@') {
            Some(end) => {
                found.push(&after[..end]);
                rest = &after[end + 1..];
            }
            None => {
                found.push(after);
                break;
            }
        }
    }
    found
}

/// Render `template` with `values`: each placeholder of the closed set must
/// appear exactly once and is replaced; any other `@MT_` token, before or after
/// the replacement, is an error.
///
/// # Errors
/// [`HarnessError::Render`] naming the offending token.
pub fn render_template(template: &str, values: &Placeholders) -> Result<String, HarnessError> {
    // A generated fragment never carries a token of its own.
    for (what, text) in [
        ("includes", &values.includes),
        ("states", &values.states),
        ("cases", &values.cases),
        ("call", &values.call),
    ] {
        if text.contains("@MT_") {
            return Err(HarnessError::Render(format!(
                "the {what} fragment contains a placeholder token"
            )));
        }
    }
    for token in tokens(template) {
        if !PLACEHOLDER_NAMES.contains(&token) {
            return Err(HarnessError::Render(format!(
                "unknown placeholder @{token}@ in the template"
            )));
        }
    }
    let mut text = template.to_owned();
    for name in PLACEHOLDER_NAMES {
        let marker = format!("@{name}@");
        let count = text.matches(&marker).count();
        if count != 1 {
            return Err(HarnessError::Render(format!(
                "placeholder {marker} appears {count} times in the template; it must appear exactly once"
            )));
        }
        text = text.replacen(&marker, &values.value(name), 1);
    }
    if let Some(left) = tokens(&text).first() {
        return Err(HarnessError::Render(format!(
            "token @{left}@ is left after rendering"
        )));
    }
    Ok(text)
}

/// Render the shared template with `values`.
///
/// # Errors
/// As [`render_template`].
pub fn render(values: &Placeholders) -> Result<String, HarnessError> {
    render_template(RUNTIME_TEMPLATE, values)
}
