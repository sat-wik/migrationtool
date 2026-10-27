//! The library-state value model (D-06).
//!
//! `lib_state_in` and `lib_state_out` are JSON objects of named fields. Each
//! field carries its C type and an exact value in one of the shapes of the
//! equivalence contract ("Library state"). A pointer is never a raw address.
//!
//! | Shape | JSON |
//! |---|---|
//! | integer of at most 32 bits | `{"type", "value": <integer>}` |
//! | 64-bit integer | `{"type", "value": "<decimal string>"}` |
//! | `float`, `double` | `{"type", "bits": "<lowercase hex>"}` |
//! | array of a 1-byte element | `{"type", "hex": "<lowercase hex>"}` |
//! | other array | `{"type", "items": [..]}` |
//! | struct or union | `{"type", "fields": {..}}` |
//! | pointer | `{"type", "pointer": null}`, `{"buffer", "offset"}` or `{"symbol"}` |
//! | unobserved | `{"type", "unobserved": "<reason>"}` |
//!
//! The reserved top-level names are [`RESERVED_NAMES`]: `in` is a byte-array
//! value, `globals` and `heap` are struct-shaped values whose member names are
//! `<object>:<symbol>` and decimal offsets, and `ret` is any value.

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};

/// Top-level names with a fixed meaning in a `lib_state` object.
pub const RESERVED_NAMES: [&str; 4] = ["globals", "heap", "in", "ret"];

/// The longest C type text accepted.
const MAX_TYPE_LEN: usize = 256;

/// The keys that select the shape of a value.
const SHAPE_KEYS: [&str; 7] = [
    "value",
    "bits",
    "hex",
    "items",
    "fields",
    "pointer",
    "unobserved",
];

/// A value that does not follow the contract's shapes.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{path}: {message}")]
pub struct StateError {
    /// Where in the state the value is (field names joined by `.`).
    pub path: String,
    /// What is wrong.
    pub message: String,
}

fn error(path: &str, message: impl Into<String>) -> StateError {
    StateError {
        path: path.to_owned(),
        message: message.into(),
    }
}

/// What a pointer field points at. Never a raw address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pointer {
    /// NULL.
    Null,
    /// A byte offset into a named region: `in`, `out`, a declared state or
    /// local name, `heap`, or `global:<object>:<symbol>`.
    Buffer {
        /// The region.
        region: String,
        /// The offset in bytes.
        offset: u64,
    },
    /// A function, by name.
    Symbol(String),
}

/// One value of the library state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateValue {
    /// An integer, enum, bool or char of at most 32 bits.
    Int {
        /// The C type.
        c_type: String,
        /// The value, within `i32::MIN..=u32::MAX`.
        value: i64,
    },
    /// A 64-bit integer, as a decimal string.
    Wide {
        /// The C type.
        c_type: String,
        /// Decimal digits, with a leading `-` for a negative value.
        value: String,
    },
    /// A `float` or `double`, as its IEEE bit pattern.
    Float {
        /// The C type.
        c_type: String,
        /// Lowercase hex, 8 or 16 digits.
        bits: String,
    },
    /// An array of a 1-byte element type, as hex.
    Bytes {
        /// The C type.
        c_type: String,
        /// Lowercase hex of even length.
        hex: String,
    },
    /// Any other array.
    Items {
        /// The C type.
        c_type: String,
        /// The elements.
        items: Vec<StateValue>,
    },
    /// A struct or union.
    Struct {
        /// The C type.
        c_type: String,
        /// The members by name.
        fields: BTreeMap<String, StateValue>,
    },
    /// A pointer.
    Pointer {
        /// The C type.
        c_type: String,
        /// What it points at.
        pointer: Pointer,
    },
    /// A field that was not observed, with the reason.
    Unobserved {
        /// The C type.
        c_type: String,
        /// Why it was not observed.
        reason: String,
    },
}

fn is_c_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn is_lower_hex(text: &str) -> bool {
    text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// `-?[0-9]+` without leading zeros, within `i64::MIN..=u64::MAX`.
fn is_wide_decimal(text: &str) -> bool {
    let digits = text.strip_prefix('-').unwrap_or(text);
    let plain = !digits.is_empty()
        && digits.bytes().all(|b| b.is_ascii_digit())
        && (digits == "0" || !digits.starts_with('0'))
        && text != "-0";
    plain
        && text
            .parse::<i128>()
            .is_ok_and(|n| n >= i128::from(i64::MIN) && n <= i128::from(u64::MAX))
}

fn region_is_well_formed(region: &str) -> bool {
    match region.strip_prefix("global:") {
        Some(rest) => rest
            .rsplit_once(':')
            .is_some_and(|(object, symbol)| !object.is_empty() && is_c_identifier(symbol)),
        None => is_c_identifier(region),
    }
}

fn text_of<'a>(path: &str, what: &str, value: &'a Value) -> Result<&'a str, StateError> {
    value
        .as_str()
        .ok_or_else(|| error(path, format!("{what} must be a string")))
}

impl StateValue {
    /// Read a value from JSON, naming `path` in every error.
    pub fn from_json(value: &Value, path: &str) -> Result<Self, StateError> {
        let object = value
            .as_object()
            .ok_or_else(|| error(path, "a state value must be an object"))?;
        let c_type = match object.get("type") {
            Some(Value::String(text))
                if !text.is_empty()
                    && text.len() <= MAX_TYPE_LEN
                    && !text.chars().any(char::is_control) =>
            {
                text.clone()
            }
            Some(_) => {
                return Err(error(
                    path,
                    "type must be a non-empty string without control characters",
                ));
            }
            None => return Err(error(path, "missing key `type`")),
        };
        if let Some(key) = object
            .keys()
            .find(|k| k.as_str() != "type" && !SHAPE_KEYS.contains(&k.as_str()))
        {
            return Err(error(path, format!("unknown key `{key}`")));
        }
        let present: Vec<&str> = SHAPE_KEYS
            .iter()
            .copied()
            .filter(|k| object.contains_key(*k))
            .collect();
        let [shape] = present.as_slice() else {
            return Err(error(
                path,
                format!(
                    "exactly one of {} is required, found {}",
                    SHAPE_KEYS.join(", "),
                    present.len()
                ),
            ));
        };
        let body = &object[*shape];
        match *shape {
            "value" => Self::integer(path, c_type, body),
            "bits" => {
                let bits = text_of(path, "bits", body)?;
                if matches!(bits.len(), 8 | 16) && is_lower_hex(bits) {
                    Ok(Self::Float {
                        c_type,
                        bits: bits.to_owned(),
                    })
                } else {
                    Err(error(
                        path,
                        "bits must be 8 or 16 lowercase hex digits (an IEEE bit pattern)",
                    ))
                }
            }
            "hex" => {
                let hex = text_of(path, "hex", body)?;
                if hex.len() % 2 == 0 && is_lower_hex(hex) {
                    Ok(Self::Bytes {
                        c_type,
                        hex: hex.to_owned(),
                    })
                } else {
                    Err(error(path, "hex must be lowercase hex of even length"))
                }
            }
            "items" => {
                let array = body
                    .as_array()
                    .ok_or_else(|| error(path, "items must be an array"))?;
                let items = array
                    .iter()
                    .enumerate()
                    .map(|(i, item)| Self::from_json(item, &format!("{path}[{i}]")))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Self::Items { c_type, items })
            }
            "fields" => {
                let members = body
                    .as_object()
                    .ok_or_else(|| error(path, "fields must be an object"))?;
                let mut fields = BTreeMap::new();
                for (name, member) in members {
                    if name.is_empty() {
                        return Err(error(path, "a member name must not be empty"));
                    }
                    fields.insert(
                        name.clone(),
                        Self::from_json(member, &format!("{path}.{name}"))?,
                    );
                }
                Ok(Self::Struct { c_type, fields })
            }
            "pointer" => Ok(Self::Pointer {
                c_type,
                pointer: Self::pointer(path, body)?,
            }),
            _ => {
                let reason = text_of(path, "unobserved", body)?;
                if reason.trim().is_empty() {
                    Err(error(path, "an unobserved mark needs a reason"))
                } else {
                    Ok(Self::Unobserved {
                        c_type,
                        reason: reason.to_owned(),
                    })
                }
            }
        }
    }

    fn integer(path: &str, c_type: String, body: &Value) -> Result<Self, StateError> {
        match body {
            Value::Number(number) => {
                let within = number
                    .as_i64()
                    .filter(|n| (i64::from(i32::MIN)..=i64::from(u32::MAX)).contains(n));
                within.map_or_else(
                    || {
                        Err(error(
                            path,
                            format!(
                                "value {number} is not an integer of at most 32 bits; \
                                 a 64-bit value is a decimal string"
                            ),
                        ))
                    },
                    |value| Ok(Self::Int { c_type, value }),
                )
            }
            Value::String(text) if is_wide_decimal(text) => Ok(Self::Wide {
                c_type,
                value: text.clone(),
            }),
            Value::String(text) => Err(error(
                path,
                format!("value {text:?} is not a decimal integer string within 64 bits"),
            )),
            _ => Err(error(path, "value must be an integer or a decimal string")),
        }
    }

    fn pointer(path: &str, body: &Value) -> Result<Pointer, StateError> {
        match body {
            Value::Null => Ok(Pointer::Null),
            Value::Object(map) => {
                let keys: Vec<&str> = map.keys().map(String::as_str).collect();
                match keys.as_slice() {
                    ["buffer", "offset"] => {
                        let region = text_of(path, "buffer", &map["buffer"])?;
                        if !region_is_well_formed(region) {
                            return Err(error(
                                path,
                                format!(
                                    "buffer {region:?} is not in, out, a name, heap or \
                                     global:<object>:<symbol>"
                                ),
                            ));
                        }
                        let offset = map["offset"]
                            .as_u64()
                            .ok_or_else(|| error(path, "offset must be a non-negative integer"))?;
                        Ok(Pointer::Buffer {
                            region: region.to_owned(),
                            offset,
                        })
                    }
                    ["symbol"] => {
                        let symbol = text_of(path, "symbol", &map["symbol"])?;
                        if is_c_identifier(symbol) {
                            Ok(Pointer::Symbol(symbol.to_owned()))
                        } else {
                            Err(error(path, "symbol must be a C identifier"))
                        }
                    }
                    _ => Err(error(
                        path,
                        "a pointer is null, {buffer, offset} or {symbol}; a raw address \
                         is not recorded",
                    )),
                }
            }
            _ => Err(error(
                path,
                "a pointer is null, {buffer, offset} or {symbol}; a raw address is not recorded",
            )),
        }
    }

    /// The JSON form of the value.
    #[must_use]
    pub fn to_json(&self) -> Value {
        let mut map = Map::new();
        let (c_type, key, body) = match self {
            Self::Int { c_type, value } => (c_type, "value", Value::from(*value)),
            Self::Wide { c_type, value } => (c_type, "value", Value::from(value.clone())),
            Self::Float { c_type, bits } => (c_type, "bits", Value::from(bits.clone())),
            Self::Bytes { c_type, hex } => (c_type, "hex", Value::from(hex.clone())),
            Self::Items { c_type, items } => (
                c_type,
                "items",
                Value::Array(items.iter().map(Self::to_json).collect()),
            ),
            Self::Struct { c_type, fields } => (
                c_type,
                "fields",
                Value::Object(
                    fields
                        .iter()
                        .map(|(name, member)| (name.clone(), member.to_json()))
                        .collect(),
                ),
            ),
            Self::Pointer { c_type, pointer } => (
                c_type,
                "pointer",
                match pointer {
                    Pointer::Null => Value::Null,
                    Pointer::Buffer { region, offset } => {
                        let mut inner = Map::new();
                        inner.insert("buffer".to_owned(), Value::from(region.clone()));
                        inner.insert("offset".to_owned(), Value::from(*offset));
                        Value::Object(inner)
                    }
                    Pointer::Symbol(symbol) => {
                        let mut inner = Map::new();
                        inner.insert("symbol".to_owned(), Value::from(symbol.clone()));
                        Value::Object(inner)
                    }
                },
            ),
            Self::Unobserved { c_type, reason } => {
                (c_type, "unobserved", Value::from(reason.clone()))
            }
        };
        map.insert("type".to_owned(), Value::from(c_type.clone()));
        map.insert(key.to_owned(), body);
        Value::Object(map)
    }

    /// The number of unobserved marks in this value and below it.
    #[must_use]
    pub fn unobserved_count(&self) -> usize {
        match self {
            Self::Unobserved { .. } => 1,
            Self::Items { items, .. } => items.iter().map(Self::unobserved_count).sum(),
            Self::Struct { fields, .. } => fields.values().map(Self::unobserved_count).sum(),
            _ => 0,
        }
    }

    /// True for a byte-array value.
    #[must_use]
    pub const fn is_bytes(&self) -> bool {
        matches!(self, Self::Bytes { .. })
    }
}

impl Serialize for StateValue {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_json().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for StateValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        Self::from_json(&value, "value").map_err(serde::de::Error::custom)
    }
}

/// `lib_state_in` or `lib_state_out`: named fields, with the reserved names
/// of [`RESERVED_NAMES`] checked.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LibState(BTreeMap<String, StateValue>);

impl LibState {
    /// A state from its fields, with the reserved names checked.
    pub fn new(fields: BTreeMap<String, StateValue>) -> Result<Self, StateError> {
        let state = Self(fields);
        state.validate()?;
        Ok(state)
    }

    /// The fields by name.
    #[must_use]
    pub const fn fields(&self) -> &BTreeMap<String, StateValue> {
        &self.0
    }

    /// The field called `name`.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&StateValue> {
        self.0.get(name)
    }

    /// True when the state has no field.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The number of unobserved marks anywhere in the state.
    #[must_use]
    pub fn unobserved_count(&self) -> usize {
        self.0.values().map(StateValue::unobserved_count).sum()
    }

    /// Re-check every value's shape and the reserved names. Values built in
    /// code are checked the way values read from JSON are.
    pub fn validate(&self) -> Result<(), StateError> {
        for (name, value) in &self.0 {
            if name.is_empty() {
                return Err(error("lib_state", "a field name must not be empty"));
            }
            let again = StateValue::from_json(&value.to_json(), name)?;
            if &again != value {
                return Err(error(name, "the value is not in its canonical form"));
            }
            match (name.as_str(), value) {
                ("in", value) if !value.is_bytes() => {
                    return Err(error(name, "`in` must be a byte-array value (hex)"));
                }
                ("globals", StateValue::Struct { fields, .. }) => {
                    for key in fields.keys() {
                        let ok = key.rsplit_once(':').is_some_and(|(object, symbol)| {
                            !object.is_empty() && is_c_identifier(symbol)
                        });
                        if !ok {
                            return Err(error(
                                name,
                                format!("globals key {key:?} is not <object>:<symbol>"),
                            ));
                        }
                    }
                }
                ("heap", StateValue::Struct { fields, .. }) => {
                    for key in fields.keys() {
                        if key.is_empty() || !key.bytes().all(|b| b.is_ascii_digit()) {
                            return Err(error(
                                name,
                                format!("heap key {key:?} is not a decimal offset"),
                            ));
                        }
                    }
                }
                ("globals" | "heap", _) => {
                    return Err(error(
                        name,
                        "`globals` and `heap` must be struct-shaped values (fields)",
                    ));
                }
                _ => {}
            }
        }
        Ok(())
    }
}

impl Serialize for LibState {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for LibState {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = BTreeMap::<String, Value>::deserialize(deserializer)?;
        let mut fields = BTreeMap::new();
        for (name, value) in raw {
            let parsed = StateValue::from_json(&value, &name).map_err(serde::de::Error::custom)?;
            fields.insert(name, parsed);
        }
        Self::new(fields).map_err(serde::de::Error::custom)
    }
}
