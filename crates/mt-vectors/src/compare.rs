//! The comparison rule (D-08).
//!
//! The compared fields are `rc`, `stdout.pattern`, `lib_state_out` and the
//! outcome class word of `stderr`. Not compared: `argv`, `stdin`, `env`,
//! `lib_state_in`, the detail words of `stderr`, and fields the expected
//! vector marks unobserved. A vector with `has_ub` is excluded from output
//! comparison.

use std::collections::BTreeSet;

use crate::state::{LibState, StateValue};
use crate::vector::{HasUbLabel, Vector};

/// The result of comparing an expected vector with an actual one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Comparison {
    /// The expected vector carries a `has_ub` label, so its output is not
    /// compared (the safe action is checked elsewhere).
    Excluded {
        /// The expected vector's label.
        label: HasUbLabel,
    },
    /// Every compared field is equal.
    Equal,
    /// The named fields differ, in the order `rc`, `stdout.pattern`,
    /// `lib_state_out.<path>`, `stderr.outcome`.
    Differs {
        /// The differing fields.
        fields: Vec<String>,
    },
}

/// Compare `actual` with `expected` under the rule above.
#[must_use]
pub fn compare(expected: &Vector, actual: &Vector) -> Comparison {
    if let Some(label) = &expected.has_ub {
        return Comparison::Excluded {
            label: label.clone(),
        };
    }
    let mut fields = Vec::new();
    if expected.rc != actual.rc {
        fields.push("rc".to_owned());
    }
    if expected.stdout.pattern != actual.stdout.pattern {
        fields.push("stdout.pattern".to_owned());
    }
    diff_state(&expected.lib_state_out, &actual.lib_state_out, &mut fields);
    match (expected.outcome_class(), actual.outcome_class()) {
        (Ok(want), Ok(got)) if want == got => {}
        _ => fields.push("stderr.outcome".to_owned()),
    }
    if fields.is_empty() {
        Comparison::Equal
    } else {
        Comparison::Differs { fields }
    }
}

fn is_unobserved(value: &StateValue) -> bool {
    matches!(value, StateValue::Unobserved { .. })
}

fn diff_state(want: &LibState, got: &LibState, out: &mut Vec<String>) {
    let names: BTreeSet<&String> = want.fields().keys().chain(got.fields().keys()).collect();
    for name in names {
        diff_member(
            &format!("lib_state_out.{name}"),
            want.get(name),
            got.get(name),
            out,
        );
    }
}

fn diff_member(
    path: &str,
    want: Option<&StateValue>,
    got: Option<&StateValue>,
    out: &mut Vec<String>,
) {
    match (want, got) {
        (Some(want), _) if is_unobserved(want) => {}
        (Some(want), Some(got)) => diff_value(path, want, got, out),
        (None, None) => {}
        _ => out.push(path.to_owned()),
    }
}

fn diff_value(path: &str, want: &StateValue, got: &StateValue, out: &mut Vec<String>) {
    match (want, got) {
        (
            StateValue::Struct {
                c_type: want_type,
                fields: want_fields,
            },
            StateValue::Struct {
                c_type: got_type,
                fields: got_fields,
            },
        ) if want_type == got_type => {
            let names: BTreeSet<&String> = want_fields.keys().chain(got_fields.keys()).collect();
            for name in names {
                diff_member(
                    &format!("{path}.{name}"),
                    want_fields.get(name),
                    got_fields.get(name),
                    out,
                );
            }
        }
        (
            StateValue::Items {
                c_type: want_type,
                items: want_items,
            },
            StateValue::Items {
                c_type: got_type,
                items: got_items,
            },
        ) if want_type == got_type && want_items.len() == got_items.len() => {
            for (i, (a, b)) in want_items.iter().zip(got_items).enumerate() {
                diff_member(&format!("{path}[{i}]"), Some(a), Some(b), out);
            }
        }
        _ => {
            if want != got {
                out.push(path.to_owned());
            }
        }
    }
}
