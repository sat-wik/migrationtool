//! Turning the bytes of a state object into the contract's typed values
//! (CONTEXT D-06, D-09; the equivalence contract, "Library state").
//!
//! [`decode_value`] reads the bytes of an object of a [`CType`] and gives a
//! [`StateValue`]. Padding is never read: a struct is its members, in
//! declaration order. A pointer is never stored as an address: it is classified
//! against the driver's known [`Regions`] and becomes null, an offset into a
//! named region, or an unobserved mark with a reason ([`classify_pointer`]).

use mt_vectors::state::{Pointer, StateValue};

use crate::hex;
use crate::layout::{CType, Kind};

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
    /// The type has a form the decoder does not read yet.
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

/// The regions a pointer can be classified against.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Regions {
    named: Vec<Region>,
}

impl Regions {
    /// No region.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a region called `name` (`in`, `out`, a state name, ...) at `base`.
    #[must_use]
    pub fn with_region(mut self, name: &str, base: u32, len: u32) -> Self {
        self.named.push(Region {
            name: name.to_owned(),
            base,
            len,
        });
        self
    }
}

/// Classify a 32-bit pointer value: 0 is null; an address inside a named region
/// (strictly inside first, then one past its end) is a byte offset into it.
/// `None` for anything else; the caller records an unobserved mark. No raw
/// address is ever returned.
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
    })?;
    Some(Pointer::Buffer {
        region: found.name.clone(),
        offset: u64::from(value - found.base),
    })
}

/// The little-endian unsigned value of up to 8 bytes.
fn le_unsigned(bytes: &[u8]) -> u64 {
    bytes
        .iter()
        .rev()
        .fold(0u64, |value, byte| (value << 8) | u64::from(*byte))
}

/// `value` sign-extended from `bytes` bytes.
fn sign_extend(value: u64, bytes: u32) -> i64 {
    let shift = 64 - bytes * 8;
    // The shifts move the sign bit to the top and back; the cast reinterprets
    // the bits and does not change them.
    #[allow(clippy::cast_possible_wrap)]
    let signed = (value << shift) as i64;
    signed >> shift
}

fn integer(ty: &CType, bytes: &[u8], signed: bool) -> StateValue {
    let raw = le_unsigned(bytes);
    if ty.size == 8 {
        let value = if signed {
            sign_extend(raw, 8).to_string()
        } else {
            raw.to_string()
        };
        return StateValue::Wide {
            c_type: ty.text.clone(),
            value,
        };
    }
    let value = if signed {
        sign_extend(raw, ty.size)
    } else {
        i64::try_from(raw).unwrap_or(i64::MAX)
    };
    StateValue::Int {
        c_type: ty.text.clone(),
        value,
    }
}

/// Decode `bytes`, the object of type `ty`, into a typed value.
///
/// # Errors
/// [`DecodeError::Size`] when `bytes` is not `ty.size` long;
/// [`DecodeError::Unsupported`] for a form not read yet.
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
        Kind::Int { signed } | Kind::Enum { signed } => Ok(integer(ty, bytes, *signed)),
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
        Kind::Struct { members } => {
            let mut fields = std::collections::BTreeMap::new();
            for member in members {
                if member.bits.is_some() {
                    return Err(DecodeError::Unsupported(format!(
                        "the bitfield {} of {}",
                        member.name, ty.text
                    )));
                }
                let start = member.offset as usize;
                let end = start + member.ty.size as usize;
                let slice = bytes.get(start..end).ok_or_else(|| {
                    DecodeError::Value(format!("{} lies outside {}", member.name, ty.text))
                })?;
                let value = decode_value(&member.ty, slice, regions)?;
                fields.insert(member.name.clone(), value);
            }
            Ok(StateValue::Struct {
                c_type: ty.text.clone(),
                fields,
            })
        }
        Kind::Union { .. } => Err(DecodeError::Unsupported(format!("the union {}", ty.text))),
        Kind::Float => Err(DecodeError::Unsupported(format!(
            "the floating-point type {}",
            ty.text
        ))),
    }
}
