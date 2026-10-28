//! The adapter an entry's manifest roles turn into (CONTEXT D-05, D-11, D-14).
//!
//! The manifest declares one role per C parameter. The adapter keeps each
//! parameter's C type text next to its role and renders the call the generated
//! driver makes, the `argv` of a vector, and the scalar combinations to run.
//! Nothing here reads free text from the manifest into C: roles are an enum,
//! names are C identifiers, values are integers, and the type texts are checked
//! against a closed character set before they are used in a cast.

use std::collections::BTreeMap;

use mt_capture::manifest::{
    BASELINE_CFLAGS, Entry, Expect, MAX_SCALAR_COMBOS, Role, SetupArg, UnobservedMark,
    is_function_name, pointer_pointee_type, split_parameters, state_pointee_type,
    value_pointee_type,
};
use serde::{Deserialize, Serialize};

use crate::budget::Budgets;
use crate::frame::SETUP_STATUS_BASE;
use crate::runtime::NO_STATES;

/// Why an entry cannot be driven, or a compile flag cannot be handled.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AdapterError {
    /// The entry declares no argument roles.
    #[error("the entry declares no argument roles")]
    NoRoles,
    /// The signature or the roles are not usable.
    #[error("{0}")]
    Invalid(String),
    /// The return type cannot be recorded as a 32-bit `rc`.
    #[error("the return type {0:?} cannot be recorded as a 32-bit rc")]
    UnsupportedReturn(String),
    /// A compile flag that may change the ABI or layout and is not handled.
    #[error("compile flag {0} is not handled")]
    UnhandledFlag(String),
}

/// How the call's return value becomes the vector's `rc` (contract: Field
/// convention).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReturnKind {
    /// An integer or enum, cast to 32 bits.
    Integer,
    /// `void`: `rc` is 0.
    Void,
    /// A pointer: `rc` is 0 for NULL and 1 otherwise, and the pointer is recorded
    /// as `lib_state_out.ret`, classified like any pointer.
    Pointer,
    /// A struct or union by value: `rc` is 0 and the value is recorded as
    /// `lib_state_out.ret`.
    Struct,
}

/// One C parameter with its role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    /// The parameter's C type text, as clang prints it.
    pub c_type: String,
    /// What the parameter is.
    pub role: Role,
    /// The name an `out_cap` or `scalar` value, or a `state`, is recorded under.
    pub name: Option<String>,
    /// For a `state` or an `inout_len`, the type its pointer points at (a plain
    /// type name, checked by [`state_pointee_type`] and [`value_pointee_type`]).
    pub state_type: Option<String>,
    /// The first value of an `out_cap` or `scalar`.
    pub default: Option<i64>,
    /// The values after the default.
    pub sweep: Vec<i64>,
    /// The largest input an `in_len` accepts.
    pub max: Option<u64>,
    /// What an `inout_len` is the length of (`out`), or the name of the `str` a
    /// `str_len` is the length of.
    pub of: Option<String>,
    /// For an `in_buf`: the input's bytes after the call are recorded.
    pub mutable: bool,
    /// The declared strings of a `str`.
    pub values: Vec<String>,
}

/// The values of the dimensions of one run, in parameter order (one per
/// `out_cap`, `scalar`, `inout_len` or `str` parameter; a `str` is the index of
/// its value).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Combination {
    /// The values.
    pub values: Vec<i64>,
}

/// The name the returned value of a pointer or struct return is recorded under.
pub const RET_NAME: &str = "ret";

/// A state of an entry: the object the library fills in, or the value a pointer
/// parameter leaves behind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateParam {
    /// The name its fields are recorded under in `lib_state_in` and `lib_state_out`.
    pub name: String,
    /// The type text of the object (the pointee of the parameter).
    pub type_text: String,
    /// The request word that holds its value before the call (an `inout_len`),
    /// or `None` for an object that starts zero-filled.
    pub init_word: Option<usize>,
}

/// One argument of a setup call, resolved against the entry's roles (CONTEXT D-12).
#[derive(Debug, Clone, PartialEq, Eq)]
enum ArgExpr {
    /// `$in`.
    In,
    /// `$len`.
    Len,
    /// `$cap`.
    Cap,
    /// `$arg:<name>`: request word `n`.
    Scalar(usize),
    /// A decimal integer.
    Int(i64),
    /// `&<name>`: the address of state `n`.
    Address(usize),
}

impl ArgExpr {
    fn render(&self) -> String {
        match self {
            Self::In => "mt_in".to_owned(),
            Self::Len => "mt_in_len".to_owned(),
            Self::Cap => "mt_out_cap".to_owned(),
            Self::Scalar(word) => format!("mt_scalar({word})"),
            Self::Int(value) => value.to_string(),
            Self::Address(index) => format!("mt_state({index})"),
        }
    }
}

/// A declared setup call: only validated tokens, never manifest text.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SetupCall {
    name: String,
    args: Vec<ArgExpr>,
    expect: Expect,
}

/// An entry ready to be driven.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Adapter {
    target: String,
    config: String,
    entry: String,
    ret: ReturnKind,
    params: Vec<Param>,
    marks: Vec<UnobservedMark>,
    ret_text: String,
    setup: Vec<SetupCall>,
}

/// `[A-Za-z_][A-Za-z0-9_]*`.
fn is_c_identifier(text: &str) -> bool {
    let mut bytes = text.bytes();
    bytes
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// A type text that may be written into a cast: letters, digits, `_`, spaces
/// and `*` only.
fn is_plain_type(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b' ' | b'*'))
}

/// Return types the 32-bit `rc` cannot hold and that are not structs: floating
/// point and 64-bit integers.
fn is_wide_return(text: &str) -> bool {
    let words: Vec<&str> = text.split_whitespace().collect();
    words
        .iter()
        .any(|word| matches!(*word, "float" | "double" | "uint64_t" | "int64_t"))
        || words.iter().filter(|word| **word == "long").count() >= 2
}

/// True for the roles whose value is one dimension of the combinations.
const fn is_dimension(role: Role) -> bool {
    matches!(
        role,
        Role::OutCap | Role::Scalar | Role::InoutLen | Role::Str
    )
}

impl Adapter {
    /// The adapter of `entry` of `target` in `config`.
    ///
    /// The return kind is read from the signature's text: `void`, a pointer, a
    /// `struct` or `union` by value, or otherwise an integer. A typedef name
    /// cannot say whether it is an integer, an enum or a struct, so the build
    /// resolves it from the DWARF of the entry's header ([`Adapter::with_return_kind`]).
    ///
    /// # Errors
    /// [`AdapterError::NoRoles`] for an entry without `arg` tables,
    /// [`AdapterError::UnsupportedReturn`] for a return the `rc` cannot hold and
    /// [`AdapterError::Invalid`] for a signature or role that does not fit.
    pub fn from_entry(target: &str, config: &str, entry: &Entry) -> Result<Self, AdapterError> {
        if entry.arg.is_empty() {
            return Err(AdapterError::NoRoles);
        }
        if !is_c_identifier(&entry.name) {
            return Err(AdapterError::Invalid(format!(
                "entry name {:?} is not a C identifier",
                entry.name
            )));
        }
        let (ret_text, param_texts) = split_parameters(&entry.signature)
            .map_err(|error| AdapterError::Invalid(error.to_string()))?;
        if param_texts.len() != entry.arg.len() {
            return Err(AdapterError::Invalid(format!(
                "{} arg tables but the signature has {} parameters",
                entry.arg.len(),
                param_texts.len()
            )));
        }
        let ret = if ret_text == "void" {
            ReturnKind::Void
        } else if ret_text.contains('*') {
            ReturnKind::Pointer
        } else if is_wide_return(&ret_text) {
            return Err(AdapterError::UnsupportedReturn(ret_text));
        } else if ret_text.starts_with("struct ") || ret_text.starts_with("union ") {
            ReturnKind::Struct
        } else {
            ReturnKind::Integer
        };
        if !is_plain_type(&ret_text) {
            return Err(AdapterError::Invalid(format!(
                "return type {ret_text:?} holds a character outside letters, digits, `_`, space and `*`"
            )));
        }
        let mut params = Vec::new();
        for (text, arg) in param_texts.iter().zip(&entry.arg) {
            if !is_plain_type(text) {
                return Err(AdapterError::Invalid(format!(
                    "parameter type {text:?} holds a character outside letters, digits, `_`, space and `*`"
                )));
            }
            if let Some(name) = &arg.name
                && !is_c_identifier(name)
            {
                return Err(AdapterError::Invalid(format!(
                    "argument name {name:?} is not a C identifier"
                )));
            }
            let name = match arg.role {
                Role::OutCap => Some(arg.name.clone().unwrap_or_else(|| "out_cap".to_owned())),
                Role::Scalar
                | Role::State
                | Role::InoutLen
                | Role::OutScalar
                | Role::OutPtr
                | Role::Str => arg.name.clone(),
                _ => None,
            };
            let needs_name = matches!(
                arg.role,
                Role::State | Role::InoutLen | Role::OutScalar | Role::OutPtr | Role::Str
            );
            if needs_name && name.is_none() {
                return Err(AdapterError::Invalid(format!(
                    "a {} needs a name",
                    arg.role.word()
                )));
            }
            let bad_pointee = |what: &str| {
                AdapterError::Invalid(format!(
                    "{} parameter {text:?} does not point at {what}",
                    arg.role.word()
                ))
            };
            let state_type = match arg.role {
                Role::State => {
                    Some(state_pointee_type(text).ok_or_else(|| bad_pointee("a plain type name"))?)
                }
                Role::InoutLen | Role::OutScalar => Some(
                    value_pointee_type(text).ok_or_else(|| bad_pointee("a plain scalar type"))?,
                ),
                Role::OutPtr => {
                    Some(pointer_pointee_type(text).ok_or_else(|| bad_pointee("a pointer"))?)
                }
                _ => None,
            };
            if arg.role == Role::Str && !arg.values.iter().all(|v| is_printable(v)) {
                return Err(AdapterError::Invalid(
                    "a str value holds a character outside printable ASCII".to_owned(),
                ));
            }
            params.push(Param {
                c_type: text.clone(),
                role: arg.role,
                name,
                state_type,
                default: arg.default,
                sweep: arg.sweep.clone(),
                max: arg.max,
                of: arg.of.clone(),
                mutable: arg.mutable,
                values: arg.values.clone(),
            });
        }
        let setup = build_setup(entry, &params)?;
        Ok(Self {
            target: target.to_owned(),
            config: config.to_owned(),
            entry: entry.name.clone(),
            ret,
            params,
            marks: entry.unobserved.clone(),
            ret_text,
            setup,
        })
    }

    /// `<target>/<config>/<entry>`: `argv[0]` of every vector (D-05).
    #[must_use]
    pub fn argv0(&self) -> String {
        format!("{}/{}/{}", self.target, self.config, self.entry)
    }

    /// The entry's C function name.
    #[must_use]
    pub fn entry_name(&self) -> &str {
        &self.entry
    }

    /// The parameters, in order.
    #[must_use]
    pub fn params(&self) -> &[Param] {
        &self.params
    }

    /// How the return value is recorded.
    #[must_use]
    pub const fn return_kind(&self) -> ReturnKind {
        self.ret
    }

    /// The return type as the signature spells it.
    #[must_use]
    pub fn return_text(&self) -> &str {
        &self.ret_text
    }

    /// This adapter with its return kind resolved from the DWARF of the return
    /// type: a typedef name read as an integer by the signature's text may be a
    /// struct, an enum or a pointer.
    #[must_use]
    pub fn with_return_kind(mut self, kind: ReturnKind) -> Self {
        self.ret = kind;
        self
    }

    /// True when the returned value is stored in a state called [`RET_NAME`].
    #[must_use]
    pub const fn records_return(&self) -> bool {
        matches!(self.ret, ReturnKind::Pointer | ReturnKind::Struct)
    }

    /// The value of each dimension of `combination` by parameter index.
    fn dimension_values(&self, combination: &Combination) -> BTreeMap<usize, i64> {
        let mut found = BTreeMap::new();
        let mut dimension = 0usize;
        for (index, param) in self.params.iter().enumerate() {
            if is_dimension(param.role) {
                let value = combination.values.get(dimension).copied().unwrap_or(0);
                found.insert(index, value);
                dimension += 1;
            }
        }
        found
    }

    /// The string a `str` parameter takes in a combination.
    fn chosen<'a>(&'a self, index: usize, values: &BTreeMap<usize, i64>) -> &'a str {
        let choice = values.get(&index).copied().unwrap_or(0);
        self.params
            .get(index)
            .and_then(|param| param.values.get(usize::try_from(choice).unwrap_or(0)))
            .map_or("", String::as_str)
    }

    /// The index of the `str` parameter called `name`.
    fn str_param(&self, name: &str) -> Option<usize> {
        self.params
            .iter()
            .position(|param| param.role == Role::Str && param.name.as_deref() == Some(name))
    }

    /// The C expression of the call, of type `int32_t`, over the runtime's
    /// `mt_in`, `mt_in_len`, `mt_out`, `mt_out_cap`, `mt_scalar(index)`,
    /// `mt_state(index)`, `mt_str(index)` and `mt_str_len(index)`.
    #[must_use]
    pub fn call_expression(&self) -> String {
        let mut scalar_index = 0usize;
        let mut state_index = 0usize;
        let mut str_index = 0usize;
        let mut str_indexes: BTreeMap<&str, usize> = BTreeMap::new();
        let mut args = Vec::new();
        for param in &self.params {
            let value = match param.role {
                Role::InBuf => "mt_in".to_owned(),
                Role::OutBuf => "mt_out".to_owned(),
                Role::InLen => "mt_in_len".to_owned(),
                Role::OutCap => "mt_out_cap".to_owned(),
                Role::Scalar => {
                    let text = format!("mt_scalar({scalar_index})");
                    scalar_index += 1;
                    text
                }
                Role::State | Role::OutScalar | Role::OutPtr => {
                    let text = format!("mt_state({state_index})");
                    state_index += 1;
                    text
                }
                Role::InoutLen => {
                    // The initial value is request word `scalar_index`; the
                    // driver writes it into the state before the call.
                    scalar_index += 1;
                    let text = format!("mt_state({state_index})");
                    state_index += 1;
                    text
                }
                Role::Str => {
                    if let Some(name) = param.name.as_deref() {
                        str_indexes.insert(name, str_index);
                    }
                    let text = format!("mt_str({str_index})");
                    str_index += 1;
                    text
                }
                Role::StrLen => {
                    let of = param
                        .of
                        .as_deref()
                        .and_then(|name| str_indexes.get(name))
                        .copied()
                        .unwrap_or(0);
                    format!("mt_str_len({of})")
                }
            };
            args.push(format!("({}){value}", param.c_type));
        }
        let call = format!("{}({})", self.entry, args.join(", "));
        match self.ret {
            ReturnKind::Integer => format!("(int32_t)({call})"),
            ReturnKind::Void => format!("((void)({call}), (int32_t)0)"),
            ReturnKind::Pointer => format!(
                "((*({} *)mt_state({state_index}) = {call}) != 0 ? (int32_t)1 : (int32_t)0)",
                self.ret_text
            ),
            ReturnKind::Struct => format!(
                "(*({} *)mt_state({state_index}) = {call}, (int32_t)0)",
                self.ret_text
            ),
        }
    }

    /// The C statements of the declared setup steps (CONTEXT D-12), in order. Each
    /// step runs only while every earlier one returned what it expected; the first
    /// mismatch records `SETUP_STATUS_BASE + k` and the step's return value in
    /// `setup_status` and `setup_ret`. Everything in it is a validated identifier, a
    /// decimal integer or a runtime accessor, never text from the manifest.
    #[must_use]
    pub fn setup_fragment(&self) -> String {
        if self.setup.is_empty() {
            return "/* no setup steps */".to_owned();
        }
        let blocks: Vec<String> = self
            .setup
            .iter()
            .enumerate()
            .map(|(step, call)| {
                let args: Vec<String> = call.args.iter().map(ArgExpr::render).collect();
                let invocation = format!("{}({})", call.name, args.join(", "));
                match call.expect {
                    Expect::Void => {
                        format!("if (setup_status == 0u) {{\n        (void)({invocation});\n    }}")
                    }
                    Expect::Value(value) => format!(
                        "if (setup_status == 0u) {{\n        int32_t mt_setup_r = (int32_t)({invocation});\n        if (mt_setup_r != (int32_t)({value})) {{\n            setup_status = {}u;\n            setup_ret = (u32)mt_setup_r;\n        }}\n    }}",
                        SETUP_STATUS_BASE as usize + step
                    ),
                }
            })
            .collect();
        blocks.join("\n    ")
    }

    /// The `argv` of a vector: `argv[0]`, then every non-pointer parameter in
    /// parameter order as decimal text, and each `str` as its text. The input
    /// length is the stdin byte count (D-05).
    #[must_use]
    pub fn argv(&self, input_len: usize, combination: &Combination) -> Vec<String> {
        let values = self.dimension_values(combination);
        let mut argv = vec![self.argv0()];
        for (index, param) in self.params.iter().enumerate() {
            match param.role {
                Role::InLen => argv.push(input_len.to_string()),
                Role::OutCap | Role::Scalar | Role::InoutLen => {
                    argv.push(values.get(&index).copied().unwrap_or(0).to_string());
                }
                Role::Str => argv.push(self.chosen(index, &values).to_owned()),
                Role::StrLen => {
                    let length = param
                        .of
                        .as_deref()
                        .and_then(|name| self.str_param(name))
                        .map_or(0, |at| self.chosen(at, &values).len());
                    argv.push(length.to_string());
                }
                Role::InBuf | Role::OutBuf | Role::State | Role::OutScalar | Role::OutPtr => {}
            }
        }
        argv
    }

    /// The unobserved marks of the entry (CONTEXT D-09).
    #[must_use]
    pub fn marks(&self) -> &[UnobservedMark] {
        &self.marks
    }

    /// The states, in parameter order, then the returned value's state for a
    /// pointer or struct return.
    #[must_use]
    pub fn states(&self) -> Vec<StateParam> {
        let mut word = 0usize;
        let mut states = Vec::new();
        for param in &self.params {
            let init_word = match param.role {
                Role::Scalar => {
                    word += 1;
                    None
                }
                Role::InoutLen => {
                    word += 1;
                    Some(word - 1)
                }
                _ => None,
            };
            if !matches!(
                param.role,
                Role::State | Role::InoutLen | Role::OutScalar | Role::OutPtr
            ) {
                continue;
            }
            if let (Some(name), Some(type_text)) = (&param.name, &param.state_type) {
                states.push(StateParam {
                    name: name.clone(),
                    type_text: type_text.clone(),
                    init_word,
                });
            }
        }
        if self.records_return() {
            states.push(StateParam {
                name: RET_NAME.to_owned(),
                type_text: self.ret_text.clone(),
                init_word: None,
            });
        }
        states
    }

    /// The C text that declares the states of the driver: their count, the
    /// `sizeof` of each type and the request word that holds each initial value,
    /// in the order of [`Adapter::states`].
    #[must_use]
    pub fn states_fragment(&self) -> String {
        let states = self.states();
        if states.is_empty() {
            return NO_STATES.to_owned();
        }
        let sizes: String = states
            .iter()
            .map(|state| format!("(u32)sizeof({}), ", state.type_text))
            .collect();
        let words: String = states
            .iter()
            .map(|state| {
                state
                    .init_word
                    .map_or_else(|| "-1, ".to_owned(), |word| format!("{word}, "))
            })
            .collect();
        format!(
            "#define MT_STATE_COUNT {}u\nstatic const u32 mt_state_sizes[MT_STATE_COUNT + 1u] = {{ {sizes}0u }};\nstatic const int mt_state_word[MT_STATE_COUNT + 1u] = {{ {words}-1 }};",
            states.len()
        )
    }

    /// The output capacity of a combination, 0 without an output buffer.
    #[must_use]
    pub fn output_capacity(&self, combination: &Combination) -> u32 {
        let values = self.dimension_values(combination);
        for (index, param) in self.params.iter().enumerate() {
            let sizes_output = param.role == Role::OutCap
                || (param.role == Role::InoutLen && param.of.as_deref() == Some("out"));
            if sizes_output {
                let value = values.get(&index).copied().unwrap_or(0);
                return u32::try_from(value).unwrap_or(0);
            }
        }
        0
    }

    /// The values of the `scalar` and `inout_len` parameters of a combination, in
    /// order: the words the request carries for `mt_scalar` and for the initial
    /// value of an `inout_len`.
    #[must_use]
    pub fn scalar_words(&self, combination: &Combination) -> Vec<i64> {
        let values = self.dimension_values(combination);
        self.params
            .iter()
            .enumerate()
            .filter(|(_, param)| matches!(param.role, Role::Scalar | Role::InoutLen))
            .map(|(index, _)| values.get(&index).copied().unwrap_or(0))
            .collect()
    }

    /// The strings the request carries for a combination, one per `str`
    /// parameter in parameter order (the bytes are never part of the C source).
    #[must_use]
    pub fn strings(&self, combination: &Combination) -> Vec<String> {
        let values = self.dimension_values(combination);
        self.params
            .iter()
            .enumerate()
            .filter(|(_, param)| param.role == Role::Str)
            .map(|(index, _)| self.chosen(index, &values).to_owned())
            .collect()
    }

    /// The recorded values of a combination by name (the provenance): a
    /// scalar's, a capacity's or a length's value, and the index of a `str`'s
    /// value.
    #[must_use]
    pub fn scalars_by_name(&self, combination: &Combination) -> BTreeMap<String, i64> {
        let values = self.dimension_values(combination);
        let mut named = BTreeMap::new();
        for (index, param) in self.params.iter().enumerate() {
            if let (Some(name), Some(value)) = (&param.name, values.get(&index)) {
                named.insert(name.clone(), *value);
            }
        }
        named
    }

    /// The element type of the input buffer, qualifiers removed: `const char *`
    /// is `char`. It names the type of a recorded `in`.
    #[must_use]
    pub fn input_element_type(&self) -> String {
        self.params
            .iter()
            .find(|param| param.role == Role::InBuf)
            .map_or_else(
                || "char".to_owned(),
                |param| {
                    param
                        .c_type
                        .replace('*', " ")
                        .split_whitespace()
                        .filter(|word| !matches!(*word, "const" | "volatile"))
                        .collect::<Vec<_>>()
                        .join(" ")
                },
            )
    }

    /// True when the input's bytes after the call are recorded.
    #[must_use]
    pub fn mutable_input(&self) -> bool {
        self.params
            .iter()
            .any(|param| param.role == Role::InBuf && param.mutable)
    }

    /// The smallest `max` among the `in_len` parameters: seeds longer than it
    /// are not run.
    #[must_use]
    pub fn max_input(&self) -> Option<u64> {
        self.params
            .iter()
            .filter(|param| param.role == Role::InLen)
            .filter_map(|param| param.max)
            .min()
    }
}

/// Resolve the entry's `setup` steps against its roles (CONTEXT D-12, T-03-25): a
/// call name must be a function name, an argument one of the closed forms that names
/// a state or a scalar of the entry, and the expectation `void` or a 32-bit decimal.
/// The manifest validator checks the same; this is the second check, at the point the
/// steps become C.
fn build_setup(entry: &Entry, params: &[Param]) -> Result<Vec<SetupCall>, AdapterError> {
    let bad = |step: usize, why: String| AdapterError::Invalid(format!("setup[{step}]: {why}"));
    if !entry.local.is_empty() {
        return Err(AdapterError::Invalid(
            "local objects are not driven yet".to_owned(),
        ));
    }
    let mut states: BTreeMap<&str, usize> = BTreeMap::new();
    let mut scalars: BTreeMap<&str, usize> = BTreeMap::new();
    let (mut state_index, mut word) = (0usize, 0usize);
    for param in params {
        match param.role {
            Role::State => {
                if let Some(name) = param.name.as_deref() {
                    states.insert(name, state_index);
                }
                state_index += 1;
            }
            Role::InoutLen => {
                state_index += 1;
                word += 1;
            }
            Role::OutScalar | Role::OutPtr => state_index += 1,
            Role::Scalar => {
                if let Some(name) = param.name.as_deref() {
                    scalars.insert(name, word);
                }
                word += 1;
            }
            _ => {}
        }
    }
    let mut setup = Vec::new();
    for (step, declared) in entry.setup.iter().enumerate() {
        let Some(name) = declared.call.as_deref() else {
            return Err(bad(step, "set steps are not driven yet".to_owned()));
        };
        if !is_function_name(name) {
            return Err(bad(step, format!("call {name:?} is not a C identifier")));
        }
        let Some(expect) = declared.expect.as_deref().and_then(Expect::parse) else {
            return Err(bad(
                step,
                "a call needs expect void or a decimal".to_owned(),
            ));
        };
        let mut args = Vec::new();
        for text in &declared.args {
            let arg = match SetupArg::parse(text) {
                Some(SetupArg::In) => ArgExpr::In,
                Some(SetupArg::Len) => ArgExpr::Len,
                Some(SetupArg::Cap) => ArgExpr::Cap,
                Some(SetupArg::Int(value)) => ArgExpr::Int(value),
                Some(SetupArg::Scalar(name)) => scalars
                    .get(name.as_str())
                    .map(|index| ArgExpr::Scalar(*index))
                    .ok_or_else(|| bad(step, format!("argument {text:?} names no scalar")))?,
                Some(SetupArg::AddressOf(name)) => states
                    .get(name.as_str())
                    .map(|index| ArgExpr::Address(*index))
                    .ok_or_else(|| bad(step, format!("argument {text:?} names no state")))?,
                Some(SetupArg::Local(_)) | None => {
                    return Err(bad(
                        step,
                        format!("argument {text:?} is not in the closed grammar"),
                    ));
                }
            };
            args.push(arg);
        }
        setup.push(SetupCall {
            name: name.to_owned(),
            args,
            expect,
        });
    }
    if setup.len() > mt_capture::manifest::MAX_SETUP_STEPS {
        return Err(AdapterError::Invalid(format!(
            "{} setup steps; at most {}",
            setup.len(),
            mt_capture::manifest::MAX_SETUP_STEPS
        )));
    }
    Ok(setup)
}

/// True for text of printable ASCII only (0x20 to 0x7E).
fn is_printable(text: &str) -> bool {
    text.bytes().all(|byte| (0x20..=0x7e).contains(&byte))
}

/// The combinations of `adapter` (CONTEXT D-14): the cartesian product, in
/// parameter order with the first parameter varying slowest, of `[default] +
/// sweep` for each `out_cap`, `scalar` and `inout_len`, and of the value indexes of
/// each `str`. An adapter with no such parameter has one empty combination.
///
/// # Errors
/// [`AdapterError::Invalid`] for a missing default, a capacity outside
/// `0..=max_input`, or more than [`MAX_SCALAR_COMBOS`] combinations.
pub fn combinations(adapter: &Adapter) -> Result<Vec<Combination>, AdapterError> {
    let limit = i64::from(Budgets::DEFAULT.max_input);
    let mut dimensions: Vec<Vec<i64>> = Vec::new();
    for param in &adapter.params {
        if !is_dimension(param.role) {
            continue;
        }
        let label = param.name.as_deref().unwrap_or("scalar");
        if param.role == Role::Str {
            if param.values.is_empty() {
                return Err(AdapterError::Invalid(format!("{label} has no values")));
            }
            dimensions.push((0..i64::try_from(param.values.len()).unwrap_or(0)).collect());
            continue;
        }
        let Some(default) = param.default else {
            return Err(AdapterError::Invalid(format!(
                "{label} has no default value"
            )));
        };
        let mut values = vec![default];
        values.extend(param.sweep.iter().copied());
        if matches!(param.role, Role::OutCap | Role::InoutLen)
            && let Some(bad) = values.iter().find(|value| !(0..=limit).contains(value))
        {
            return Err(AdapterError::Invalid(format!(
                "{label} value {bad} is outside 0..={limit}"
            )));
        }
        dimensions.push(values);
    }
    let mut product: Vec<Vec<i64>> = vec![Vec::new()];
    for values in &dimensions {
        let mut next = Vec::with_capacity(product.len() * values.len());
        for prefix in &product {
            for value in values {
                let mut row = prefix.clone();
                row.push(*value);
                next.push(row);
            }
        }
        product = next;
        if product.len() > MAX_SCALAR_COMBOS {
            return Err(AdapterError::Invalid(format!(
                "more than {MAX_SCALAR_COMBOS} scalar combinations"
            )));
        }
    }
    Ok(product
        .into_iter()
        .map(|values| Combination { values })
        .collect())
}

/// Options that carry their operand as the next argument and are kept.
const KEPT_SEPARATE: [&str; 6] = ["-I", "-isystem", "-iquote", "-include", "-D", "-U"];

/// Prefixes of kept options whose operand is attached.
const KEPT_JOINED: [&str; 6] = ["-I", "-isystem", "-iquote", "-include", "-D", "-U"];

/// Options kept because they change the ABI or the layout.
const KEPT_ABI: [&str; 5] = [
    "-fshort-enums",
    "-fno-short-enums",
    "-funsigned-char",
    "-fsigned-char",
    "-fshort-wchar",
];

/// Options that carry an operand which is dropped with them.
const DROPPED_SEPARATE: [&str; 4] = ["-o", "-MF", "-MT", "-MQ"];

/// Options dropped as they are spelled: none changes the ABI or layout.
const DROPPED_EXACT: [&str; 6] = [
    "-c",
    "-pedantic",
    "-fPIC",
    "-fpic",
    "-ffunction-sections",
    "-fdata-sections",
];

/// Prefixes of dropped options.
const DROPPED_PREFIX: [&str; 6] = [
    "-O",
    "-g",
    "-W",
    "-std=",
    "-fstack-protector",
    "-fvisibility=",
];

/// The flags the driver is compiled with, from the arguments of the defining
/// unit's compile command (`arguments[0]` is the compiler).
///
/// Kept: `-I`, `-isystem`, `-iquote`, `-include`, `-D` and `-U` with their
/// operands, and the options that change the ABI or the layout (`-fshort-enums`
/// and its negation, `-funsigned-char`, `-fsigned-char`, `-fshort-wchar`,
/// `-fpack-struct*`). Dropped: the compiler, the source, `-o`, `-c`,
/// optimisation, debug and warning options, `-std=*`, dependency-file options
/// with their operands, `-fPIC`, `-fpic`, the stack-protector, visibility and
/// section options, `-pedantic`, and the manifest's baseline flags, which the
/// driver adds itself. Any other option is an error, so a flag that might
/// change the layout is never silently ignored.
///
/// # Errors
/// [`AdapterError::UnhandledFlag`] naming the first option that is neither kept
/// nor dropped.
pub fn driver_flags(arguments: &[String]) -> Result<Vec<String>, AdapterError> {
    let mut kept = Vec::new();
    let mut remaining = arguments.iter().skip(1);
    while let Some(argument) = remaining.next() {
        let text = argument.as_str();
        if DROPPED_SEPARATE.contains(&text) {
            remaining.next();
        } else if KEPT_SEPARATE.contains(&text) {
            kept.push(argument.clone());
            if let Some(operand) = remaining.next() {
                kept.push(operand.clone());
            }
        } else if DROPPED_SEPARATE
            .iter()
            .any(|option| text.starts_with(option))
            || DROPPED_EXACT.contains(&text)
            || BASELINE_CFLAGS.contains(&text)
            || text.starts_with("-M")
            || DROPPED_PREFIX.iter().any(|prefix| text.starts_with(prefix))
        {
            // Joined `-o<path>`, `-MF<path>` and the other dropped spellings.
        } else if KEPT_JOINED
            .iter()
            .any(|prefix| text.starts_with(prefix) && text.len() > prefix.len())
            || KEPT_ABI.contains(&text)
            || text.starts_with("-fpack-struct")
        {
            kept.push(argument.clone());
        } else if text.starts_with('-') {
            return Err(AdapterError::UnhandledFlag(argument.clone()));
        }
        // An argument that does not start with `-` is the source file.
    }
    Ok(kept)
}
