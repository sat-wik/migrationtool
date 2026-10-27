//! Turning the bytes of a state object into the contract's typed values
//! (CONTEXT D-06, D-09; the equivalence contract, "Library state").
//!
//! [`decode_value`] reads the bytes of an object of a [`CType`] and gives a
//! [`StateValue`]. Padding is never read: a struct is its members, in
//! declaration order, and a union records every member decoded from the same
//! bytes. A pointer is never stored as an address: it is classified against the
//! driver's known [`Regions`] and becomes null, an offset into a named region, a
//! function's name, or an unobserved mark with a reason ([`classify_pointer`]).
//! [`apply_marks`] and [`check_marks`] apply the manifest's unobserved marks.

use std::collections::BTreeMap;

use mt_capture::manifest::{MarkStep, UnobservedMark, parse_mark_path};
use mt_vectors::state::{Pointer, StateValue};

use crate::hex;
use crate::layout::{BitField, CType, Kind};

/// The reason recorded for a pointer that points at no known region.
pub const UNRESOLVED_POINTER_REASON: &str = "pointer outside known regions";

/// Why bytes cannot be decoded as a type.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DecodeError {
    /// The bytes are not the size of the type.
    #[error("{what}: {actual} bytes for a type of {expected}")]
    Size {
        /// The type text.
        what: String,
        /// The type's size.
        expected: u32,
        /// The bytes given.
        actual: usize,
    },
    /// The type has a form the decoder does not read.
    #[error("cannot decode {0}")]
    Unsupported(String),
    /// A decoded value broke the contract's shapes.
    #[error("{0}")]
    Value(String),
}

/// A named span of the driver's address space: `[base, base + len]` is a valid
/// pointer target, one past the end included.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Region {
    name: String,
    base: u32,
    len: u32,
}

/// The regions a pointer can be classified against: the named buffers, state
/// objects, heap arena and globals, and the functions of the linked driver.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Regions {
    named: Vec<Region>,
    /// Function addresses with the Thumb bit cleared.
    functions: BTreeMap<u32, String>,
}

impl Regions {
    /// No region.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a region called `name` (`in`, `out`, a state name, `heap`,
    /// `global:<object>:<symbol>`) at `base`.
    #[must_use]
    pub fn with_region(mut self, name: &str, base: u32, len: u32) -> Self {
        self.named.push(Region {
            name: name.to_owned(),
            base,
            len,
        });
        self
    }

    /// Add a function at `address` (its Thumb bit, if set, is ignored). When two
    /// names share an address the first added is kept.
    #[must_use]
    pub fn with_function(mut self, address: u32, name: &str) -> Self {
        self.functions
            .entry(address & !1)
            .or_insert_with(|| name.to_owned());
        self
    }

    /// Add every function of `functions` (address to name).
    #[must_use]
    pub fn with_functions(mut self, functions: &BTreeMap<u32, String>) -> Self {
        for (address, name) in functions {
            self = self.with_function(*address, name);
        }
        self
    }
}

/// Classify a 32-bit pointer value: 0 is null; an address inside a named region
/// (strictly inside first, then one past its end) is a byte offset into it; an
/// address equal to a function of the driver, with the Thumb bit cleared, is that
/// function's name. `None` for anything else; the caller records an unobserved
/// mark. No raw address is ever returned.
#[must_use]
pub fn classify_pointer(value: u32, regions: &Regions) -> Option<Pointer> {
    if value == 0 {
        return Some(Pointer::Null);
    }
    let inside = regions.named.iter().find(|region| {
        value >= region.base && u64::from(value) < u64::from(region.base) + u64::from(region.len)
    });
    let found = inside.or_else(|| {
        regions
            .named
            .iter()
            .find(|region| u64::from(value) == u64::from(region.base) + u64::from(region.len))
    });
    if let Some(region) = found {
        return Some(Pointer::Buffer {
            region: region.name.clone(),
            offset: u64::from(value - region.base),
        });
    }
    regions
        .functions
        .get(&(value & !1))
        .map(|name| Pointer::Symbol(name.clone()))
}

/// The little-endian unsigned value of up to 8 bytes.
fn le_unsigned(bytes: &[u8]) -> u64 {
    bytes
        .iter()
        .rev()
        .fold(0u64, |value, byte| (value << 8) | u64::from(*byte))
}

/// `value` sign-extended from its low `bits` bits (1..=64).
fn sign_extend(value: u64, bits: u32) -> i64 {
    let shift = 64 - bits;
    // The shifts move the sign bit to the top and back; the cast reinterprets
    // the bits and does not change them.
    #[allow(clippy::cast_possible_wrap)]
    let signed = (value << shift) as i64;
    signed >> shift
}

/// An integer value of `ty` (up to 32 bits is a JSON integer, 8 bytes a decimal
/// string).
fn integer(ty: &CType, value: i128) -> Result<StateValue, DecodeError> {
    if ty.size == 8 {
        return Ok(StateValue::Wide {
            c_type: ty.text.clone(),
            value: value.to_string(),
        });
    }
    let value = i64::try_from(value)
        .map_err(|_| DecodeError::Value(format!("a value of {} out of range", ty.text)))?;
    Ok(StateValue::Int {
        c_type: ty.text.clone(),
        value,
    })
}

/// The `size` bits starting at bit `offset` of `bytes`, least significant first.
fn extract_bits(bytes: &[u8], bit: &BitField) -> Option<u64> {
    if bit.size == 0 || bit.size > 64 {
        return None;
    }
    let mut value = 0u64;
    for index in 0..bit.size {
        let position = bit.offset.checked_add(index)?;
        let byte = *bytes.get(usize::try_from(position / 8).ok()?)?;
        if (byte >> (position % 8)) & 1 == 1 {
            value |= 1u64 << index;
        }
    }
    Some(value)
}

fn is_signed(ty: &CType) -> bool {
    matches!(
        ty.kind,
        Kind::Int { signed: true } | Kind::Enum { signed: true }
    )
}

fn decode_bitfield(
    ty: &CType,
    bytes: &[u8],
    bits: &BitField,
    owner: &str,
) -> Result<StateValue, DecodeError> {
    if !matches!(ty.kind, Kind::Int { .. } | Kind::Enum { .. }) {
        return Err(DecodeError::Unsupported(format!(
            "a bitfield of the type {} in {owner}",
            ty.text
        )));
    }
    let raw = extract_bits(bytes, bits)
        .ok_or_else(|| DecodeError::Value(format!("a bitfield lies outside {owner}")))?;
    let value = if is_signed(ty) {
        i128::from(sign_extend(raw, bits.size))
    } else {
        i128::from(raw)
    };
    integer(ty, value)
}

/// Decode `bytes`, the object of type `ty`, into a typed value.
///
/// # Errors
/// [`DecodeError::Size`] when `bytes` is not `ty.size` long;
/// [`DecodeError::Unsupported`] for a bitfield of a type that is not an integer;
/// [`DecodeError::Value`] for a value the contract's shapes cannot hold.
pub fn decode_value(
    ty: &CType,
    bytes: &[u8],
    regions: &Regions,
) -> Result<StateValue, DecodeError> {
    if bytes.len() != ty.size as usize {
        return Err(DecodeError::Size {
            what: ty.text.clone(),
            expected: ty.size,
            actual: bytes.len(),
        });
    }
    match &ty.kind {
        Kind::Int { signed } | Kind::Enum { signed } => {
            let raw = le_unsigned(bytes);
            let value = if *signed {
                i128::from(sign_extend(raw, ty.size * 8))
            } else {
                i128::from(raw)
            };
            integer(ty, value)
        }
        Kind::Float => {
            let raw = le_unsigned(bytes);
            let bits = match ty.size {
                4 => format!("{raw:08x}"),
                8 => format!("{raw:016x}"),
                other => {
                    return Err(DecodeError::Unsupported(format!(
                        "a floating-point type of {other} bytes"
                    )));
                }
            };
            Ok(StateValue::Float {
                c_type: ty.text.clone(),
                bits,
            })
        }
        Kind::Pointer { .. } => {
            let value = u32::try_from(le_unsigned(bytes))
                .map_err(|_| DecodeError::Value("a pointer wider than 32 bits".to_owned()))?;
            Ok(match classify_pointer(value, regions) {
                Some(pointer) => StateValue::Pointer {
                    c_type: ty.text.clone(),
                    pointer,
                },
                None => StateValue::Unobserved {
                    c_type: ty.text.clone(),
                    reason: UNRESOLVED_POINTER_REASON.to_owned(),
                },
            })
        }
        Kind::Array { elem, count } => {
            let one_byte =
                elem.size == 1 && matches!(elem.kind, Kind::Int { .. } | Kind::Enum { .. });
            if one_byte {
                return Ok(StateValue::Bytes {
                    c_type: ty.text.clone(),
                    hex: hex(bytes),
                });
            }
            let stride = elem.size as usize;
            let mut items = Vec::new();
            for index in 0..usize::try_from(*count).unwrap_or(0) {
                let start = index * stride;
                let chunk = bytes.get(start..start + stride).ok_or_else(|| {
                    DecodeError::Value(format!("an element of {} lies outside it", ty.text))
                })?;
                items.push(decode_value(elem, chunk, regions)?);
            }
            Ok(StateValue::Items {
                c_type: ty.text.clone(),
                items,
            })
        }
        Kind::Struct { members } | Kind::Union { members } => {
            let mut fields = BTreeMap::new();
            for member in members {
                let value = if let Some(bits) = &member.bits {
                    decode_bitfield(&member.ty, bytes, bits, &ty.text)?
                } else {
                    let start = member.offset as usize;
                    let end = start + member.ty.size as usize;
                    let slice = bytes.get(start..end).ok_or_else(|| {
                        DecodeError::Value(format!("{} lies outside {}", member.name, ty.text))
                    })?;
                    decode_value(&member.ty, slice, regions)?
                };
                fields.insert(member.name.clone(), value);
            }
            Ok(StateValue::Struct {
                c_type: ty.text.clone(),
                fields,
            })
        }
    }
}

// ---------------------------------------------------------------------------
// Unobserved marks (CONTEXT D-09)
// ---------------------------------------------------------------------------

fn c_type_of(value: &StateValue) -> &str {
    match value {
        StateValue::Int { c_type, .. }
        | StateValue::Wide { c_type, .. }
        | StateValue::Float { c_type, .. }
        | StateValue::Bytes { c_type, .. }
        | StateValue::Items { c_type, .. }
        | StateValue::Struct { c_type, .. }
        | StateValue::Pointer { c_type, .. }
        | StateValue::Unobserved { c_type, .. } => c_type,
    }
}

fn mark_value(
    value: &mut StateValue,
    steps: &[MarkStep],
    reason: &str,
    field: &str,
) -> Result<(), DecodeError> {
    let Some((step, rest)) = steps.split_first() else {
        *value = StateValue::Unobserved {
            c_type: c_type_of(value).to_owned(),
            reason: reason.to_owned(),
        };
        return Ok(());
    };
    let next = match (step, &mut *value) {
        (MarkStep::Member(name), StateValue::Struct { fields, .. }) => fields.get_mut(name),
        (MarkStep::Index(index), StateValue::Items { items, .. }) => {
            usize::try_from(*index).ok().and_then(|i| items.get_mut(i))
        }
        _ => None,
    };
    match next {
        Some(inner) => mark_value(inner, rest, reason, field),
        None => Err(DecodeError::Value(format!(
            "the unobserved mark {field} does not name a field of the recorded state"
        ))),
    }
}

/// Replace each marked field of the state called `name` with its unobserved mark
/// (the field keeps its C type and the mark's reason). Marks of other states are
/// left to their own call.
///
/// # Errors
/// [`DecodeError::Value`] when a mark does not name a field of the value.
pub fn apply_marks(
    name: &str,
    mut value: StateValue,
    marks: &[UnobservedMark],
) -> Result<StateValue, DecodeError> {
    for mark in marks {
        let Some((state, steps)) = parse_mark_path(&mark.field) else {
            return Err(DecodeError::Value(format!(
                "the unobserved mark {:?} is not a field path",
                mark.field
            )));
        };
        if state == name {
            mark_value(&mut value, &steps, &mark.reason, &mark.field)?;
        }
    }
    Ok(value)
}

/// Check every mark against the layouts of the entry's states: the state, each
/// member and each index must exist, and one element of a byte array cannot be
/// marked (mark the array). The error names the mark.
///
/// # Errors
/// A message naming the first mark that does not fit its layout.
pub fn check_marks(
    layouts: &BTreeMap<String, CType>,
    marks: &[UnobservedMark],
) -> Result<(), String> {
    for mark in marks {
        let field = &mark.field;
        let (state, steps) = parse_mark_path(field)
            .ok_or_else(|| format!("{field}: not a field path (<state>(.<member>|[<index>])+)"))?;
        let mut current = layouts
            .get(&state)
            .ok_or_else(|| format!("{field}: the entry has no state {state}"))?;
        for step in &steps {
            current = match (step, &current.kind) {
                (MarkStep::Member(name), Kind::Struct { members } | Kind::Union { members }) => {
                    members
                        .iter()
                        .find(|member| &member.name == name)
                        .map(|member| &member.ty)
                        .ok_or_else(|| format!("{field}: {} has no member {name}", current.text))?
                }
                (MarkStep::Index(index), Kind::Array { elem, count }) => {
                    let one_byte =
                        elem.size == 1 && matches!(elem.kind, Kind::Int { .. } | Kind::Enum { .. });
                    if one_byte {
                        return Err(format!(
                            "{field}: one element of the byte array {} cannot be marked; mark the array",
                            current.text
                        ));
                    }
                    if index >= count {
                        return Err(format!(
                            "{field}: index {index} is outside {} ({count} elements)",
                            current.text
                        ));
                    }
                    elem
                }
                _ => {
                    return Err(format!(
                        "{field}: {} has no such member or index",
                        current.text
                    ));
                }
            };
        }
    }
    Ok(())
}
