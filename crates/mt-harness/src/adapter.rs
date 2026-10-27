//! The adapter an entry's manifest roles turn into (CONTEXT D-05, D-11, D-14).
//!
//! The manifest declares one role per C parameter. The adapter keeps each
//! parameter's C type text next to its role and renders the call the generated
//! driver makes, the `argv` of a vector, and the scalar combinations to run.
//! Nothing here reads free text from the manifest into C: roles are an enum,
//! names are C identifiers, values are integers, and the type texts are checked
//! against a closed character set before they are used in a cast.

use std::collections::BTreeMap;

use mt_capture::manifest::{BASELINE_CFLAGS, Entry, MAX_SCALAR_COMBOS, Role, split_parameters};

use crate::budget::Budgets;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnKind {
    /// An integer or enum, cast to 32 bits.
    Integer,
    /// `void`: `rc` is 0.
    Void,
    /// A pointer: 0 for NULL, 1 otherwise.
    Pointer,
}

/// One C parameter with its role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    /// The parameter's C type text, as clang prints it.
    pub c_type: String,
    /// What the parameter is.
    pub role: Role,
    /// The name an `out_cap` or `scalar` value is recorded under.
    pub name: Option<String>,
    /// The first value of an `out_cap` or `scalar`.
    pub default: Option<i64>,
    /// The values after the default.
    pub sweep: Vec<i64>,
    /// The largest input an `in_len` accepts.
    pub max: Option<u64>,
}

/// The values of the scalar dimensions of one run, in parameter order (one per
/// `out_cap` or `scalar` parameter).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Combination {
    /// The values.
    pub values: Vec<i64>,
}

/// An entry ready to be driven.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Adapter {
    target: String,
    config: String,
    entry: String,
    ret: ReturnKind,
    params: Vec<Param>,
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

/// Return types the 32-bit `rc` cannot hold: floating point and 64-bit
/// integers (their full value belongs to `lib_state_out.ret`), and a struct or
/// union by value.
fn is_wide_return(text: &str) -> bool {
    let words: Vec<&str> = text.split_whitespace().collect();
    words.iter().any(|word| {
        matches!(
            *word,
            "float" | "double" | "struct" | "union" | "uint64_t" | "int64_t"
        )
    }) || words.iter().filter(|word| **word == "long").count() >= 2
}

impl Adapter {
    /// The adapter of `entry` of `target` in `config`.
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
                Role::Scalar => arg.name.clone(),
                _ => None,
            };
            params.push(Param {
                c_type: text.clone(),
                role: arg.role,
                name,
                default: arg.default,
                sweep: arg.sweep.clone(),
                max: arg.max,
            });
        }
        Ok(Self {
            target: target.to_owned(),
            config: config.to_owned(),
            entry: entry.name.clone(),
            ret,
            params,
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

    /// The C expression of the call, of type `int32_t`, over the runtime's
    /// `mt_in`, `mt_in_len`, `mt_out`, `mt_out_cap` and `mt_scalar(index)`.
    #[must_use]
    pub fn call_expression(&self) -> String {
        let mut scalar_index = 0usize;
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
            };
            args.push(format!("({}){value}", param.c_type));
        }
        let call = format!("{}({})", self.entry, args.join(", "));
        match self.ret {
            ReturnKind::Integer => format!("(int32_t)({call})"),
            ReturnKind::Void => format!("((void)({call}), (int32_t)0)"),
            ReturnKind::Pointer => format!("(({call}) != 0 ? (int32_t)1 : (int32_t)0)"),
        }
    }

    /// The `argv` of a vector: `argv[0]`, then every non-pointer parameter in
    /// parameter order as decimal text. The input length is the stdin byte
    /// count (D-05).
    #[must_use]
    pub fn argv(&self, input_len: usize, combination: &Combination) -> Vec<String> {
        let mut argv = vec![self.argv0()];
        let mut dimension = 0usize;
        for param in &self.params {
            match param.role {
                Role::InLen => argv.push(input_len.to_string()),
                Role::OutCap | Role::Scalar => {
                    let value = combination.values.get(dimension).copied().unwrap_or(0);
                    dimension += 1;
                    argv.push(value.to_string());
                }
                Role::InBuf | Role::OutBuf => {}
            }
        }
        argv
    }

    /// The output capacity of a combination, 0 without an output buffer.
    #[must_use]
    pub fn output_capacity(&self, combination: &Combination) -> u32 {
        let mut dimension = 0usize;
        for param in &self.params {
            if matches!(param.role, Role::OutCap | Role::Scalar) {
                let value = combination.values.get(dimension).copied().unwrap_or(0);
                if param.role == Role::OutCap {
                    return u32::try_from(value).unwrap_or(0);
                }
                dimension += 1;
            }
        }
        0
    }

    /// The values of the `scalar` parameters of a combination, in order: the
    /// words the request carries for `mt_scalar`.
    #[must_use]
    pub fn scalar_words(&self, combination: &Combination) -> Vec<i64> {
        let mut dimension = 0usize;
        let mut words = Vec::new();
        for param in &self.params {
            if matches!(param.role, Role::OutCap | Role::Scalar) {
                let value = combination.values.get(dimension).copied().unwrap_or(0);
                dimension += 1;
                if param.role == Role::Scalar {
                    words.push(value);
                }
            }
        }
        words
    }

    /// The recorded scalars of a combination by name (the provenance).
    #[must_use]
    pub fn scalars_by_name(&self, combination: &Combination) -> BTreeMap<String, i64> {
        let mut named = BTreeMap::new();
        let mut dimension = 0usize;
        for param in &self.params {
            if matches!(param.role, Role::OutCap | Role::Scalar) {
                let value = combination.values.get(dimension).copied().unwrap_or(0);
                dimension += 1;
                if let Some(name) = &param.name {
                    named.insert(name.clone(), value);
                }
            }
        }
        named
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

/// The scalar combinations of `adapter` (CONTEXT D-14): the cartesian product,
/// in parameter order with the first parameter varying slowest, of `[default] +
/// sweep` for each `out_cap` and `scalar`. An adapter with no such parameter
/// has one empty combination.
///
/// # Errors
/// [`AdapterError::Invalid`] for a missing default, an `out_cap` outside
/// `0..=max_input`, or more than [`MAX_SCALAR_COMBOS`] combinations.
pub fn combinations(adapter: &Adapter) -> Result<Vec<Combination>, AdapterError> {
    let limit = i64::from(Budgets::DEFAULT.max_input);
    let mut dimensions: Vec<Vec<i64>> = Vec::new();
    for param in &adapter.params {
        if !matches!(param.role, Role::OutCap | Role::Scalar) {
            continue;
        }
        let label = param.name.as_deref().unwrap_or("scalar");
        let Some(default) = param.default else {
            return Err(AdapterError::Invalid(format!(
                "{label} has no default value"
            )));
        };
        let mut values = vec![default];
        values.extend(param.sweep.iter().copied());
        if param.role == Role::OutCap
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
