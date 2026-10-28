//! The typed heap walk (CONTEXT D-06, D-09; the equivalence contract, "Library
//! state": `heap`).
//!
//! The driver dumps the bytes of its `_sbrk` arena that are in use, and the
//! arena sits at a fixed address, so an offset into it is stable from run to run.
//! [`walk_heap`] starts from the recorded pointers that were classified into the
//! `heap` region (the roots) and follows typed edges: a pointer whose pointee type
//! is known is decoded at its offset, and the pointers inside the decoded object
//! are followed in turn. A `char` pointee is the NUL-terminated byte run in hex.
//! Objects are keyed by decimal offset and decoded once, so a cycle ends; the walk
//! stops after [`HEAP_WALK_CAP`] objects and the pointers it does not follow
//! become unobserved marks with the reason [`HEAP_CAP_REASON`].

use std::collections::{BTreeMap, VecDeque};

use mt_vectors::state::{Pointer, StateValue};

use crate::decode::{DecodeError, Regions, decode_value};
use crate::hex;
use crate::layout::{CType, Kind};

/// The most objects one walk records (the contract's 4096).
pub const HEAP_WALK_CAP: usize = 4096;

/// The reason recorded for a pointer the walk did not follow because the cap was
/// reached.
pub const HEAP_CAP_REASON: &str = "heap walk cap reached";

/// The name of the arena's region in a pointer.
const HEAP_REGION: &str = "heap";

/// `char`, `signed char` and `unsigned char`, without qualifiers: a pointee of
/// these types is a string, recorded as its NUL-terminated bytes.
fn is_char_run(ty: &CType) -> bool {
    let base = ty
        .text
        .split_whitespace()
        .filter(|word| !matches!(*word, "const" | "volatile"))
        .collect::<Vec<_>>()
        .join(" ");
    matches!(ty.kind, Kind::Int { .. })
        && ty.size == 1
        && matches!(base.as_str(), "char" | "signed char" | "unsigned char")
}

/// The pointee text of a pointer type text: `struct node *` is `struct node`.
fn pointee_text(c_type: &str) -> Option<&str> {
    c_type.trim_end().strip_suffix('*').map(str::trim_end)
}

/// The bytes of the run that starts at `offset`, up to and including the NUL, or
/// `None` when the arena ends first.
fn run_at(arena: &[u8], offset: usize) -> Option<&[u8]> {
    let rest = arena.get(offset..)?;
    let end = rest.iter().position(|byte| *byte == 0)?;
    rest.get(..=end)
}

struct Walker<'a> {
    arena: &'a [u8],
    pointees: &'a BTreeMap<String, CType>,
    /// The offsets that have an object (queued or decoded), with the pointee text
    /// they are decoded as.
    claimed: BTreeMap<u64, String>,
    queue: VecDeque<u64>,
}

impl Walker<'_> {
    /// Visit every pointer of `value`: follow those into the heap whose object can
    /// be decoded, and replace the ones beyond the cap with an unobserved mark.
    fn follow(&mut self, value: &mut StateValue) {
        match value {
            StateValue::Items { items, .. } => {
                for item in items {
                    self.follow(item);
                }
            }
            StateValue::Struct { fields, .. } => {
                for field in fields.values_mut() {
                    self.follow(field);
                }
            }
            StateValue::Pointer {
                c_type,
                pointer: Pointer::Buffer { region, offset },
            } if region == HEAP_REGION => {
                let (c_type, offset) = (c_type.clone(), *offset);
                if self.claimed.contains_key(&offset) {
                    return;
                }
                let Some(text) = pointee_text(&c_type) else {
                    return;
                };
                let Some(ty) = self.pointees.get(text) else {
                    return;
                };
                let Ok(start) = usize::try_from(offset) else {
                    return;
                };
                let decodable = if is_char_run(ty) {
                    run_at(self.arena, start).is_some()
                } else {
                    start
                        .checked_add(ty.size as usize)
                        .is_some_and(|end| end <= self.arena.len())
                };
                if !decodable {
                    return;
                }
                if self.claimed.len() >= HEAP_WALK_CAP {
                    *value = StateValue::Unobserved {
                        c_type,
                        reason: HEAP_CAP_REASON.to_owned(),
                    };
                    return;
                }
                self.claimed.insert(offset, text.to_owned());
                self.queue.push_back(offset);
            }
            _ => {}
        }
    }
}

/// Record the objects of the arena that the `roots` reach, by decimal offset.
///
/// `arena` is the used bytes of the arena, which is also the extent of the `heap`
/// region of `regions`; `pointees` is the table of [`crate::layout::LayoutModel::pointee_types`].
/// A root keeps its classified pointer, except one that is beyond the cap. A pointer
/// to an unknown type, to bytes outside the arena or to anything but the heap is
/// left as the classified pointer it is and has no object.
///
/// # Errors
/// [`DecodeError`] when a reachable object cannot be decoded as its type.
pub fn walk_heap(
    arena: &[u8],
    pointees: &BTreeMap<String, CType>,
    regions: &Regions,
    roots: &mut [&mut StateValue],
) -> Result<BTreeMap<String, StateValue>, DecodeError> {
    let mut walker = Walker {
        arena,
        pointees,
        claimed: BTreeMap::new(),
        queue: VecDeque::new(),
    };
    for root in roots.iter_mut() {
        walker.follow(root);
    }
    let mut objects = BTreeMap::new();
    while let Some(offset) = walker.queue.pop_front() {
        let Some(text) = walker.claimed.get(&offset).cloned() else {
            continue;
        };
        let Some(ty) = pointees.get(&text) else {
            continue;
        };
        let start = usize::try_from(offset)
            .map_err(|_| DecodeError::Value("a heap offset beyond the address space".to_owned()))?;
        let mut value = if is_char_run(ty) {
            let run = run_at(arena, start)
                .ok_or_else(|| DecodeError::Value(format!("a string at {offset} is not closed")))?;
            StateValue::Bytes {
                c_type: format!("{text}[{}]", run.len()),
                hex: hex(run),
            }
        } else {
            let end = start + ty.size as usize;
            let bytes = arena.get(start..end).ok_or_else(|| {
                DecodeError::Value(format!("an object at {offset} lies outside the arena"))
            })?;
            decode_value(ty, bytes, regions)?
        };
        walker.follow(&mut value);
        objects.insert(offset.to_string(), value);
    }
    Ok(objects)
}
