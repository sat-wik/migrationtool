//! The `testbed/manifest.toml` schema, version 1.
//!
//! The manifest is the single committed list of test-bed targets (CONTEXT
//! D-20): where each source comes from and how it is pinned, its licence, the
//! entry functions the harness drives and the build configurations that are
//! captured. Unknown keys are rejected at every level so a typo cannot silently
//! disable a check, and every map is a `BTreeMap`, so iteration order is the
//! sorted key order.
//!
//! [`Manifest::parse`] checks shape only. [`Manifest::validate`] checks every
//! value and [`Manifest::validate_complete`] additionally requires a build
//! recipe on every target.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The only schema version this crate understands.
pub const MANIFEST_SCHEMA_VERSION: u32 = 1;

/// Licence identifiers a target may elect (CONTEXT D-02, D-27).
pub const PERMISSIVE_LICENCES: [&str; 6] = [
    "Apache-2.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "MIT",
    "Zlib",
];

/// The beachhead categories the committed manifest must cover (CONTEXT D-04).
pub const BEACHHEAD_CATEGORIES: [Category; 6] = [
    Category::Tlv,
    Category::Cbor,
    Category::Mqtt,
    Category::Ble,
    Category::OtaHeader,
    Category::Asn1X509,
];

/// The only placeholders a build string may contain (CONTEXT D-20).
///
/// `{cc}` is the pinned `arm-none-eabi-gcc` path, `{cflags}` the baseline flags
/// followed by the configuration's defines and flags, `{src}` the absolute
/// source root of the run and `{inputs}` the absolute directory holding the
/// committed files under `testbed/targets/<id>/`.
pub const PLACEHOLDERS: [&str; 4] = ["{cc}", "{cflags}", "{inputs}", "{src}"];

/// The Cortex-M4F baseline every configuration builds on, in order (CONTEXT D-06).
pub const BASELINE_CFLAGS: [&str; 5] = [
    "-mcpu=cortex-m4",
    "-mthumb",
    "-mfloat-abi=hard",
    "-mfpu=fpv4-sp-d16",
    "-Os",
];

/// A parsed test-bed manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// Must equal [`MANIFEST_SCHEMA_VERSION`].
    pub schema_version: u32,
    /// The recorded first-RTOS decision (BED-02).
    pub decisions: Decisions,
    /// The toolchain image every capture and replay runs in (CONTEXT D-10).
    pub image: ImageRef,
    /// The flags every configuration shares.
    pub baseline: Baseline,
    /// The seed corpus caps every target is selected under (CONTEXT D-16).
    pub seed_caps: SeedCaps,
    /// The targets, keyed by id.
    pub target: BTreeMap<String, Target>,
}

/// The recorded decisions the manifest depends on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decisions {
    /// `deferred`, `zephyr` or `freertos`.
    pub first_rtos: String,
    /// Why the choice is deferred (required when `first_rtos` is `deferred`).
    pub first_rtos_reason: String,
    /// The ecosystems under consideration.
    pub first_rtos_candidates: Vec<String>,
    /// The PROJECT.md decision ID that records it, such as `D-18`.
    pub project_decision: String,
}

/// The toolchain image the test bed runs in.
///
/// The digest is kept here and under `## Current image` in
/// `container/README.md`; a test ties the two together (RESEARCH Open Question
/// 10).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageRef {
    /// `sha256:` followed by 64 lowercase hex digits.
    pub digest: String,
}

/// Flags shared by every configuration of every target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Baseline {
    /// Must equal [`BASELINE_CFLAGS`] exactly.
    pub cflags: Vec<String>,
}

/// The caps a seed corpus is selected under (CONTEXT D-16): the most seeds, the
/// most bytes in total and the largest single seed. Every cap is inclusive and
/// must be above zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeedCaps {
    /// Most seeds kept per target.
    pub max_seeds: u64,
    /// Most bytes kept per target.
    pub max_total_bytes: u64,
    /// Largest single seed.
    pub max_seed_bytes: u64,
}

impl From<SeedCaps> for mt_seeds::select::Caps {
    fn from(caps: SeedCaps) -> Self {
        Self {
            max_seeds: caps.max_seeds,
            max_total_bytes: caps.max_total_bytes,
            max_seed_bytes: caps.max_seed_bytes,
        }
    }
}

/// The extraction methods a [`SeedSource`] may list (CONTEXT D-13). Hand-made
/// seeds are not a source method: they come from `testbed/targets/<id>/hand-made/`.
pub const SEED_METHODS: [&str; 5] = ["c_array", "c_string", "file_copy", "hex_text", "zip_member"];

/// A place inside a target's pinned tree its seeds may be taken from (CONTEXT
/// D-13, D-16, RESEARCH Pitfall 18). Nothing outside the declared paths is read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeedSource {
    /// A file or directory of the tree, relative to the tree root. It must lie in
    /// a permissively licensed part of the repository.
    pub path: String,
    /// Which of [`SEED_METHODS`] to apply to the files found.
    pub methods: Vec<String>,
    /// File extensions to take, without the dot; an empty string matches files
    /// without an extension. Absent: every file.
    #[serde(default)]
    pub extensions: Option<Vec<String>>,
    /// The entry functions the inputs exercise; empty means every entry of the target.
    #[serde(default)]
    pub entries: Vec<String>,
    /// Designated-initialiser member names (without the dot), for a source whose
    /// methods are exactly `["c_string"]`: only string literals that are the value
    /// of `.name = "..."` for one of these names are seeds (G-02-3, httpparser's
    /// `.raw` messages). Absent: every string literal. An explicit empty list is
    /// rejected as meaningless, which is why this is an `Option` like `extensions`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub designators: Option<Vec<String>>,
}

/// What kind of build system drives a target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BuildSystem {
    /// GNU Make, captured with Bear.
    Make,
    /// CMake, captured through `CMAKE_EXPORT_COMPILE_COMMANDS`.
    Cmake,
}

/// What a target parses (CONTEXT D-04, D-29).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    /// Type-length-value structures.
    Tlv,
    /// CBOR.
    Cbor,
    /// MQTT.
    Mqtt,
    /// BLE advertising data, GATT and ATT.
    Ble,
    /// OTA image headers.
    OtaHeader,
    /// ASN.1 and X.509.
    Asn1X509,
    /// JSON.
    Json,
    /// Protocol Buffers.
    Protobuf,
    /// CoAP.
    Coap,
    /// HTTP.
    Http,
    /// CAN framing.
    Can,
    /// Compression formats.
    Compression,
}

/// Where a target's source comes from and how it is pinned (CONTEXT D-17, D-20).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub enum Source {
    /// A shallow git fetch of one commit.
    Git {
        /// `https://` clone URL.
        url: String,
        /// Tag or branch the commit was taken from (informational).
        #[serde(rename = "ref")]
        reference: String,
        /// The commit: 40 lowercase hex digits.
        commit: String,
        /// The tree digest: 64 lowercase hex digits.
        tree_sha256: String,
    },
    /// A release archive, pinned by archive digest and tree digest.
    Archive {
        /// `https://` download URL.
        url: String,
        /// sha256 of the downloaded file.
        sha256: String,
        /// Leading path components removed on extraction.
        strip_components: u32,
        /// The release tag (informational).
        #[serde(rename = "ref")]
        reference: String,
        /// The commit of the tag, recorded for provenance.
        commit: String,
        /// The tree digest of the extracted tree.
        tree_sha256: String,
    },
}

/// The licence a target is used under (CONTEXT D-02, D-27).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Licence {
    /// One SPDX identifier, or identifiers joined by ` OR `.
    pub spdx: String,
    /// The permissive operand the project uses.
    pub elected: String,
    /// The licence file at the pinned commit, relative to the tree root.
    pub file: String,
    /// A reader's note, such as which part of the tree the licence covers.
    #[serde(default)]
    pub note: Option<String>,
}

fn default_dir() -> String {
    ".".to_owned()
}

/// Everything the project's own build system needs (CONTEXT D-07).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildArgs {
    /// Build directory relative to the tree root.
    #[serde(default = "default_dir")]
    pub dir: String,
    /// Make goals.
    #[serde(default)]
    pub goals: Vec<String>,
    /// Make variable assignments, such as `CC={cc} {cflags}`.
    #[serde(default)]
    pub make_vars: Vec<String>,
    /// The CMake target to build.
    #[serde(default)]
    pub cmake_target: Option<String>,
    /// CMake configure options.
    #[serde(default)]
    pub cmake_options: Vec<String>,
}

/// How an entry function receives its input (CONTEXT D-24, D-28).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntryForm {
    /// A pointer plus a length.
    Direct,
    /// A cursor plus an end pointer.
    Range,
    /// A pointer and length inside a struct argument; needs `accepted_wrapped`.
    Wrapped,
}

/// One buffer-in entry function, named by hand and checked after capture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// The C function name.
    pub name: String,
    /// The declaring header, relative to the tree root.
    pub header: String,
    /// The defining translation unit, relative to the tree root.
    pub source: String,
    /// How the input is passed.
    pub form: EntryForm,
    /// The `type.qualType` string clang prints for the function.
    pub signature: String,
    /// Set on, and only on, `wrapped` entries the founder accepted (CONTEXT D-28).
    #[serde(default)]
    pub accepted_wrapped: bool,
    /// The argument roles, one table per C parameter in order (Phase 3, CONTEXT
    /// D-11). Empty for an entry the harness cannot drive yet, and then absent
    /// from the serialised form, so such an entry reads as it did in Phase 2.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub arg: Vec<ArgRole>,
    /// Fields of the entry's states the harness does not compare, each with the
    /// reason (Phase 3, CONTEXT D-09). Absent when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unobserved: Vec<UnobservedMark>,
    /// Objects the harness creates besides the parameters, recorded like states
    /// (Phase 3, CONTEXT D-12). Absent when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub local: Vec<LocalObject>,
    /// The steps that run before the entry, in order (Phase 3, CONTEXT D-12).
    /// Absent when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub setup: Vec<SetupStep>,
}

/// The most setup steps one entry may declare.
pub const MAX_SETUP_STEPS: usize = 16;

/// The most elements a `local` may have (and a `state` with a count).
pub const MAX_OBJECT_COUNT: u32 = 64;

/// The closed grammar of every type text that is written into generated C (a
/// local's type, a wrapped struct's type). Applied by [`is_closed_type_text`].
pub const TYPE_TEXT_GRAMMAR: &str =
    r"^(const )?(struct |union |enum )?[A-Za-z_][A-Za-z0-9_]*( \*)?$";

/// Words that match the identifier part of [`TYPE_TEXT_GRAMMAR`] but are not
/// names: they would not make a type, so a text built from them is refused.
const NOT_A_NAME: [&str; 24] = [
    "auto", "break", "case", "const", "continue", "default", "do", "else", "enum", "extern", "for",
    "goto", "if", "inline", "register", "restrict", "return", "sizeof", "static", "struct",
    "switch", "typedef", "union", "volatile",
];

/// True when `text` can be the name of a function the generated driver calls: a C
/// identifier that is not a keyword.
#[must_use]
pub fn is_function_name(text: &str) -> bool {
    is_c_identifier(text) && !NOT_A_NAME.contains(&text)
}

/// True when `text` matches [`TYPE_TEXT_GRAMMAR`] and its name is not a C keyword
/// that cannot name a type. `void` is a name only as `void *`.
#[must_use]
pub fn is_closed_type_text(text: &str) -> bool {
    let mut rest = text.strip_prefix("const ").unwrap_or(text);
    for keyword in ["struct ", "union ", "enum "] {
        if let Some(after) = rest.strip_prefix(keyword) {
            rest = after;
            break;
        }
    }
    let (name, pointer) = match rest.strip_suffix(" *") {
        Some(name) => (name, true),
        None => (rest, false),
    };
    is_c_identifier(name) && !NOT_A_NAME.contains(&name) && (pointer || name != "void")
}

/// One `[[target.X.entry.local]]` table: an object the harness creates in its own
/// guarded, zero-filled mapping and records like a state (CONTEXT D-12).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalObject {
    /// A C identifier, unique among the entry's role names and locals.
    pub name: String,
    /// A type text matching [`TYPE_TEXT_GRAMMAR`].
    #[serde(rename = "type")]
    pub type_text: String,
    /// An array of this many objects, 1 to [`MAX_OBJECT_COUNT`]; absent for one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
}

/// One `[[target.X.entry.setup]]` table (CONTEXT D-12): exactly one of a `call`
/// (with `args` and `expect`) or a `set` (with `value`). Every string is parsed
/// into a closed form by [`SetupArg::parse`], [`Expect::parse`], [`parse_set_path`]
/// and [`SetValue::parse`]; none is pasted into C.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetupStep {
    /// The C function to call: an identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub call: Option<String>,
    /// The arguments of the call, each a [`SetupArg`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    /// The expected return of the call: `void` or a decimal integer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expect: Option<String>,
    /// The field to assign: `<local or state>.<member>(.<member>)*`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set: Option<String>,
    /// The value to assign, a [`SetValue`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// The decimal integers of the setup grammar: `0` or an optional `-` and digits
/// without a leading zero, at most `u32::MAX` in magnitude.
fn parse_decimal(text: &str) -> Option<i64> {
    let (negative, digits) = match text.strip_prefix('-') {
        Some(digits) => (true, digits),
        None => (false, text),
    };
    let canonical = !digits.is_empty()
        && digits.bytes().all(|b| b.is_ascii_digit())
        && (digits == "0" || !digits.starts_with('0'));
    if !canonical || (negative && digits == "0") {
        return None;
    }
    let value: i64 = digits.parse().ok()?;
    if value > i64::from(u32::MAX) {
        return None;
    }
    Some(if negative { -value } else { value })
}

/// An unsigned decimal in the same form, for `$in+<n>` and `$len-<n>`.
fn parse_unsigned(text: &str) -> Option<u64> {
    let value = parse_decimal(text)?;
    u64::try_from(value).ok()
}

/// One argument of a setup call (CONTEXT D-12).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupArg {
    /// `$in`: the input buffer.
    In,
    /// `$len`: the input length in bytes.
    Len,
    /// `$cap`: the output capacity.
    Cap,
    /// `$arg:<name>`: the value of the scalar called `name` in this run.
    Scalar(String),
    /// A decimal integer, optionally negative.
    Int(i64),
    /// `&<name>`: the address of a state or local.
    AddressOf(String),
    /// `<name>`: the value of a pointer local.
    Local(String),
    /// `sizeof(<local>)`: the size in bytes of a declared local (of an array local,
    /// the whole array), as a 32-bit unsigned value (plan 03-08).
    SizeOf(String),
}

impl SetupArg {
    /// Parse one argument, or `None` when it is not in the closed grammar.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "$in" => return Some(Self::In),
            "$len" => return Some(Self::Len),
            "$cap" => return Some(Self::Cap),
            _ => {}
        }
        if let Some(name) = text.strip_prefix("$arg:") {
            return is_c_identifier(name).then(|| Self::Scalar(name.to_owned()));
        }
        if let Some(inner) = text
            .strip_prefix("sizeof(")
            .and_then(|rest| rest.strip_suffix(')'))
        {
            return is_c_identifier(inner).then(|| Self::SizeOf(inner.to_owned()));
        }
        if let Some(name) = text.strip_prefix('&') {
            return is_c_identifier(name).then(|| Self::AddressOf(name.to_owned()));
        }
        if let Some(value) = parse_decimal(text) {
            return Some(Self::Int(value));
        }
        is_c_identifier(text).then(|| Self::Local(text.to_owned()))
    }
}

/// The value a `set` step assigns (CONTEXT D-12).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetValue {
    /// `$in`: the address of the input buffer.
    In,
    /// `$in+<n>`: the address `n` bytes into it.
    InPlus(u64),
    /// `$len`: the input length.
    Len,
    /// `$len-<n>`: the input length less `n`, in 32-bit unsigned arithmetic.
    LenMinus(u64),
    /// `$in[<n>]`: the input byte at index `n`, 0 when the input is shorter
    /// (plan 03-08).
    InByte(u64),
    /// A decimal integer.
    Int(i64),
}

impl SetValue {
    /// Parse one value, or `None` when it is not in the closed grammar.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "$in" => return Some(Self::In),
            "$len" => return Some(Self::Len),
            _ => {}
        }
        if let Some(digits) = text.strip_prefix("$in+") {
            return parse_unsigned(digits).map(Self::InPlus);
        }
        if let Some(digits) = text.strip_prefix("$len-") {
            return parse_unsigned(digits).map(Self::LenMinus);
        }
        if let Some(digits) = text
            .strip_prefix("$in[")
            .and_then(|rest| rest.strip_suffix(']'))
        {
            return parse_unsigned(digits).map(Self::InByte);
        }
        parse_decimal(text).map(Self::Int)
    }
}

/// What a setup call must return for the sequence to go on (CONTEXT D-12).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expect {
    /// `void`: nothing is checked.
    Void,
    /// A decimal return value, as a 32-bit integer.
    Value(i32),
}

impl Expect {
    /// Parse `void` or a decimal integer that fits `i32`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        if text == "void" {
            return Some(Self::Void);
        }
        i32::try_from(parse_decimal(text)?).ok().map(Self::Value)
    }
}

/// Split a `set` path into the object name and its members, or `None` when it is
/// not `<name>.<member>(.<member>)*` in C identifiers.
#[must_use]
pub fn parse_set_path(path: &str) -> Option<(String, Vec<String>)> {
    let mut parts = path.split('.');
    let name = parts.next()?;
    if !is_c_identifier(name) {
        return None;
    }
    let members: Vec<String> = parts.map(str::to_owned).collect();
    (!members.is_empty() && members.iter().all(|member| is_c_identifier(member)))
        .then(|| (name.to_owned(), members))
}

/// One `[[target.X.entry.unobserved]]` table: a field of a state that is recorded
/// as unobserved and never compared (CONTEXT D-09). The reason is required and
/// the number of marks is counted in the provenance of the corpus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnobservedMark {
    /// `<state name>(.<member>|[<index>])+`, in C identifiers and decimal
    /// indices.
    pub field: String,
    /// Why the field is not observed; not empty.
    pub reason: String,
}

/// One step of an unobserved mark's path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkStep {
    /// `.member`
    Member(String),
    /// `[index]`
    Index(u64),
}

/// Split a mark's `field` into the state name and its steps, or `None` when it is
/// not `<state name>(.<member>|[<index>])+` in C identifiers and decimal indices
/// (no sign, no leading zero).
#[must_use]
pub fn parse_mark_path(field: &str) -> Option<(String, Vec<MarkStep>)> {
    let name_end = field.find(['.', '[']).unwrap_or(field.len());
    let name = &field[..name_end];
    if !is_c_identifier(name) {
        return None;
    }
    let mut rest = &field[name_end..];
    let mut steps = Vec::new();
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix('.') {
            let end = after.find(['.', '[']).unwrap_or(after.len());
            let member = &after[..end];
            if !is_c_identifier(member) {
                return None;
            }
            steps.push(MarkStep::Member(member.to_owned()));
            rest = &after[end..];
        } else {
            let after = rest.strip_prefix('[')?;
            let close = after.find(']')?;
            let digits = &after[..close];
            let canonical = !digits.is_empty()
                && digits.bytes().all(|b| b.is_ascii_digit())
                && (digits == "0" || !digits.starts_with('0'));
            if !canonical {
                return None;
            }
            steps.push(MarkStep::Index(digits.parse().ok()?));
            rest = &after[close + 1..];
        }
    }
    (!steps.is_empty()).then(|| (name.to_owned(), steps))
}

/// The most scalar combinations one entry may declare (CONTEXT D-14): the
/// product of `[default] + sweep` over its `out_cap` and `scalar` arguments.
pub const MAX_SCALAR_COMBOS: usize = 16;

/// What one C parameter of an entry is for (CONTEXT D-11). The set is closed;
/// later plans add variants. Written in lower snake case in the manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// A pointer to the input bytes (the vector's `stdin`).
    InBuf,
    /// The length of the input, in the parameter's C type.
    InLen,
    /// A pointer to the output buffer (the vector's `stdout`).
    OutBuf,
    /// The capacity of the output buffer, in the parameter's C type.
    OutCap,
    /// A value with a default and an optional sweep.
    Scalar,
    /// A pointer to a struct or union the library fills in (Phase 3, CONTEXT
    /// D-09). The driver passes a zero-filled object in its own guarded mapping
    /// and records every field it has before and after the call.
    State,
    /// A pointer to a length: the declared value goes in and the value the C code
    /// leaves comes out (Phase 3, CONTEXT D-11). With `of = "out"` it is the
    /// capacity of the entry's `out_buf`, so the entry has no `out_cap`.
    InoutLen,
    /// A pointer to a scalar or enum the C code writes; it starts zero-filled in
    /// its own guarded mapping and is recorded before and after the call.
    OutScalar,
    /// A pointer to a pointer the C code sets; it starts zero-filled and is
    /// recorded as a classified pointer after the call.
    OutPtr,
    /// A pointer to a NUL-terminated string taken from the declared `values`.
    /// The bytes travel as data in the request file, never as C text.
    Str,
    /// The byte length of the `str` named by `of`, without the NUL; derived.
    StrLen,
    /// A struct passed by value that wraps the input: its `ptr_field` receives the
    /// input pointer and its `len_field` the input length (Phase 3, CONTEXT D-11,
    /// D-12). It stands for the entry's `in_buf` and `in_len` together and needs
    /// `accepted_wrapped = true`.
    Wrap,
    /// A pointer parameter passed as NULL; nothing is recorded (plan 03-08).
    Null,
    /// A pointer parameter that receives the value of a declared pointer `local`,
    /// named by the key `local` (plan 03-08).
    Local,
}

impl Role {
    /// The word written in the manifest.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::InBuf => "in_buf",
            Self::InLen => "in_len",
            Self::OutBuf => "out_buf",
            Self::OutCap => "out_cap",
            Self::Scalar => "scalar",
            Self::State => "state",
            Self::InoutLen => "inout_len",
            Self::OutScalar => "out_scalar",
            Self::OutPtr => "out_ptr",
            Self::Str => "str",
            Self::StrLen => "str_len",
            Self::Wrap => "wrap",
            Self::Null => "null",
            Self::Local => "local",
        }
    }

    /// True for the roles whose parameter is a pointer.
    #[must_use]
    pub const fn is_pointer_role(self) -> bool {
        matches!(
            self,
            Self::InBuf
                | Self::OutBuf
                | Self::State
                | Self::InoutLen
                | Self::OutScalar
                | Self::OutPtr
                | Self::Str
                | Self::Null
                | Self::Local
        )
    }
}

/// One `[[target.X.entry.arg]]` table: the role of the C parameter at its
/// position (CONTEXT D-11). Values are decimal integers only, so nothing but
/// digits from the manifest ever reaches `argv` or the generated C.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArgRole {
    /// What the parameter is.
    pub role: Role,
    /// The name a `scalar` or a `state` is recorded under (a C identifier).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The value used first; required by `out_cap` and `scalar`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<i64>,
    /// The further values to try after the default (CONTEXT D-14).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sweep: Vec<i64>,
    /// The largest input length an `in_len` accepts; required when its C type
    /// is narrower than 32 bits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<u64>,
    /// What an `inout_len` is the length of (`out`, the entry's output buffer),
    /// or the name of the `str` a `str_len` is the length of.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub of: Option<String>,
    /// For an `in_buf`: record the input's bytes after the call as
    /// `lib_state_out.in`. Only a pointer to non-const data may be mutable.
    #[serde(default, skip_serializing_if = "is_false")]
    pub mutable: bool,
    /// The declared strings of a `str` (1 to [`MAX_STR_VALUES`] values of at
    /// most [`MAX_STR_BYTES`] printable ASCII bytes each).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub values: Vec<String>,
    /// For a `state`: an array of this many objects of the pointee type, 1 to
    /// [`MAX_OBJECT_COUNT`] (Phase 3, CONTEXT D-12).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
    /// For a `wrap`: the struct member that receives the input pointer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ptr_field: Option<String>,
    /// For a `wrap`: the struct member that receives the input length.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub len_field: Option<String>,
    /// For a `local`: the name of the pointer local whose value the parameter
    /// receives (plan 03-08).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local: Option<String>,
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// The most values a `str` may declare.
pub const MAX_STR_VALUES: usize = 8;

/// The longest a declared string may be, in bytes, without its NUL.
pub const MAX_STR_BYTES: usize = 64;

fn invalid_signature(message: String) -> ManifestError {
    ManifestError::Invalid {
        issues: vec![message],
    }
}

/// Split the `type.qualType` text clang prints for a function into its return
/// type text and the text of each top-level parameter, splitting on commas
/// outside parentheses. Runs of white space collapse to one space; `(void)` and
/// `()` are zero parameters.
///
/// # Errors
/// [`ManifestError::Invalid`] when the text is not `<return type> (<parameters>)`
/// with balanced parentheses and no empty parameter.
pub fn split_parameters(signature: &str) -> Result<(String, Vec<String>), ManifestError> {
    let text = signature.split_whitespace().collect::<Vec<_>>().join(" ");
    let fail = |why: &str| invalid_signature(format!("signature {text:?}: {why}"));
    if !text.ends_with(')') {
        return Err(fail("does not end with a parameter list"));
    }
    let mut depth = 0usize;
    let mut open = None;
    for (index, byte) in text.bytes().enumerate().rev() {
        match byte {
            b')' => depth += 1,
            b'(' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    open = Some(index);
                    break;
                }
            }
            _ => {}
        }
    }
    let Some(open) = open else {
        return Err(fail("has unbalanced parentheses"));
    };
    let ret = text[..open].trim();
    if ret.is_empty() {
        return Err(fail("has no return type"));
    }
    let inner = text[open + 1..text.len() - 1].trim();
    if inner.is_empty() || inner == "void" {
        return Ok((ret.to_owned(), Vec::new()));
    }
    let mut params = Vec::new();
    let mut nesting = 0usize;
    let mut start = 0usize;
    for (index, byte) in inner.bytes().enumerate() {
        match byte {
            b'(' => nesting += 1,
            b')' => nesting = nesting.saturating_sub(1),
            b',' if nesting == 0 => {
                params.push(inner[start..index].trim().to_owned());
                start = index + 1;
            }
            _ => {}
        }
    }
    params.push(inner[start..].trim().to_owned());
    if params.iter().any(String::is_empty) {
        return Err(fail("has an empty parameter"));
    }
    Ok((ret.to_owned(), params))
}

/// True when a parameter's text names a pointer type.
fn is_pointer_parameter(text: &str) -> bool {
    text.contains('*')
}

/// The type a `state` parameter points at: the parameter text with its one
/// top-level `*` and any `const` removed. It is the only manifest-derived text
/// that becomes a type in the generated driver and in the layout probe, so it must
/// be `^(struct |union |enum )?[A-Za-z_][A-Za-z0-9_]*$` and not `void` (CONTEXT
/// D-09, D-11). `None` for anything else: the entry is then not expressible.
#[must_use]
pub fn state_pointee_type(parameter: &str) -> Option<String> {
    let spaced = parameter.replace('*', " * ");
    let mut tokens: Vec<&str> = spaced
        .split_whitespace()
        .filter(|token| *token != "const")
        .collect();
    if tokens.iter().filter(|token| **token == "*").count() != 1 || tokens.last() != Some(&"*") {
        return None;
    }
    tokens.pop();
    let plain =
        |name: &str| is_c_identifier(name) && !matches!(name, "void" | "struct" | "union" | "enum");
    let acceptable = match tokens.as_slice() {
        [name] => plain(name),
        [keyword, name] => matches!(*keyword, "struct" | "union" | "enum") && plain(name),
        _ => false,
    };
    acceptable.then(|| tokens.join(" "))
}

/// The type a pointer parameter to a plain value points at: `uLongf *` is
/// `uLongf`, `unsigned char *` is `unsigned char`, `enum mode *` is `enum mode`.
/// It becomes a type in the generated driver and in the layout probe, so it is one
/// to three C identifiers (the first may be `enum`), never `void`, `struct`,
/// `union` or `const`, with the one `*` last. `None` for anything else (CONTEXT
/// D-09, D-11).
#[must_use]
pub fn value_pointee_type(parameter: &str) -> Option<String> {
    let spaced = parameter.replace('*', " * ");
    let mut tokens: Vec<&str> = spaced.split_whitespace().collect();
    if tokens.iter().filter(|token| **token == "*").count() != 1 || tokens.last() != Some(&"*") {
        return None;
    }
    tokens.pop();
    let word = |token: &str| {
        is_c_identifier(token)
            && !matches!(
                token,
                "void" | "struct" | "union" | "const" | "volatile" | "restrict"
            )
    };
    let acceptable = match tokens.as_slice() {
        [first, name] if *first == "enum" => word(name),
        rest => (1..=3).contains(&rest.len()) && rest.iter().all(|t| word(t) && *t != "enum"),
    };
    acceptable.then(|| tokens.join(" "))
}

/// The type an `out_ptr` parameter points at: the parameter text with its last
/// `*` removed, for a pointer to a pointer. `unsigned char **` is `unsigned char *`
/// and `const char **` is `const char *`. The text becomes a type in the generated
/// driver and in the layout probe, so it is one to four C identifiers followed by
/// `*`. `None` when the parameter is not a pointer to a pointer of that form
/// (CONTEXT D-09, D-11).
#[must_use]
pub fn pointer_pointee_type(parameter: &str) -> Option<String> {
    let spaced = parameter.replace('*', " * ");
    let mut tokens: Vec<&str> = spaced.split_whitespace().collect();
    if tokens.iter().filter(|token| **token == "*").count() != 2
        || tokens.last() != Some(&"*")
        || tokens.iter().rev().nth(1) != Some(&"*")
    {
        return None;
    }
    tokens.pop();
    tokens.pop();
    let acceptable = (1..=4).contains(&tokens.len()) && tokens.iter().all(|t| is_c_identifier(t));
    acceptable.then(|| format!("{} *", tokens.join(" ")))
}

/// One named build configuration: macro and flag sets on top of the baseline.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// `default` for upstream defaults, otherwise the variant's name.
    pub name: String,
    /// Preprocessor definitions, without the leading `-D`.
    #[serde(default)]
    pub defines: Vec<String>,
    /// Extra compiler flags.
    #[serde(default)]
    pub cflags: Vec<String>,
    /// Extra Make variable assignments.
    #[serde(default)]
    pub make_vars: Vec<String>,
    /// Extra CMake configure options.
    #[serde(default)]
    pub cmake_options: Vec<String>,
    /// Extra Make goals.
    #[serde(default)]
    pub goals: Vec<String>,
    /// A different CMake target.
    #[serde(default)]
    pub cmake_target: Option<String>,
}

/// One test-bed target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    /// What the target parses; at least one.
    pub category: Vec<Category>,
    /// The build system that drives it.
    pub build: BuildSystem,
    /// Where the source comes from.
    pub source: Source,
    /// The licence it is used under.
    pub licence: Licence,
    /// The build recipe; filled only for targets whose recipe is written.
    #[serde(default)]
    pub build_args: Option<BuildArgs>,
    /// One to three entry functions.
    #[serde(default)]
    pub entry: Vec<Entry>,
    /// The default configuration and at most one variant.
    #[serde(default)]
    pub config: Vec<Config>,
    /// Where the target's seeds may come from (CONTEXT D-13); may be empty.
    #[serde(default)]
    pub seed_source: Vec<SeedSource>,
}

impl Target {
    /// The configuration named `default`, if declared.
    #[must_use]
    pub fn default_config(&self) -> Option<&Config> {
        self.config.iter().find(|c| c.name == "default")
    }

    /// The one configuration that is not `default`, if declared.
    #[must_use]
    pub fn variant(&self) -> Option<&Config> {
        self.config.iter().find(|c| c.name != "default")
    }
}

/// Why a manifest could not be used.
#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    /// The file could not be read.
    #[error("cannot read manifest {}: {source}", path.display())]
    Read {
        /// The path that failed.
        path: PathBuf,
        /// The underlying error.
        #[source]
        source: std::io::Error,
    },
    /// The text is not valid TOML for this schema.
    #[error("invalid manifest: {0}")]
    Parse(#[from] toml::de::Error),
    /// The file declares a schema this crate does not support.
    #[error("unsupported manifest schema_version {found}; expected {expected}")]
    SchemaVersion {
        /// Version found in the file.
        found: u32,
        /// Version this crate supports.
        expected: u32,
    },
    /// The manifest parses but one or more values are not acceptable.
    #[error("manifest validation failed with {} issue(s): {}", issues.len(), issues.join("; "))]
    Invalid {
        /// Every issue found, sorted, each starting with its dotted field path.
        issues: Vec<String>,
    },
    /// The canonical form could not be serialized.
    #[error("cannot serialize manifest: {0}")]
    Json(#[from] serde_json::Error),
}

impl Manifest {
    /// Parse a manifest from TOML text.
    ///
    /// # Errors
    /// [`ManifestError::Parse`] on malformed TOML, unknown keys or a duplicate
    /// table; [`ManifestError::SchemaVersion`] when `schema_version` is not 1.
    pub fn parse(text: &str) -> Result<Self, ManifestError> {
        let manifest: Self = toml::from_str(text)?;
        if manifest.schema_version != MANIFEST_SCHEMA_VERSION {
            return Err(ManifestError::SchemaVersion {
                found: manifest.schema_version,
                expected: MANIFEST_SCHEMA_VERSION,
            });
        }
        Ok(manifest)
    }

    /// Read and parse a manifest file.
    ///
    /// # Errors
    /// [`ManifestError::Read`] if the file cannot be read, otherwise as
    /// [`Manifest::parse`].
    pub fn load(path: &Path) -> Result<Self, ManifestError> {
        let text = std::fs::read_to_string(path).map_err(|source| ManifestError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse(&text)
    }

    /// Check every value. Every issue is collected, sorted and reported at
    /// once, each starting with its dotted field path.
    ///
    /// # Errors
    /// [`ManifestError::Invalid`] listing every issue.
    pub fn validate(&self) -> Result<(), ManifestError> {
        finish(self.collect_issues())
    }

    /// [`Manifest::validate`] plus the completeness rule: every target carries
    /// a `build_args` recipe.
    ///
    /// # Errors
    /// [`ManifestError::Invalid`] listing every issue.
    pub fn validate_complete(&self) -> Result<(), ManifestError> {
        let mut issues = self.collect_issues();
        for (id, target) in &self.target {
            if target.build_args.is_none() {
                issues.push(format!(
                    "target.{id}.build_args: missing; every target needs a build recipe"
                ));
            }
        }
        finish(issues)
    }

    /// The canonical JSON form: configurations ordered `default` first and then
    /// by name, entries ordered by name, pretty-printed with a trailing newline.
    ///
    /// # Errors
    /// [`ManifestError::Json`] if serialization fails.
    pub fn canonical_json(&self) -> Result<String, ManifestError> {
        let mut canonical = self.clone();
        for target in canonical.target.values_mut() {
            target.config.sort_by(|a, b| {
                (a.name != "default", &a.name).cmp(&(b.name != "default", &b.name))
            });
            target.entry.sort_by(|a, b| a.name.cmp(&b.name));
        }
        let mut text = serde_json::to_string_pretty(&canonical)?;
        text.push('\n');
        Ok(text)
    }

    fn collect_issues(&self) -> Vec<String> {
        let mut issues = Vec::new();
        if self.schema_version != MANIFEST_SCHEMA_VERSION {
            issues.push(format!(
                "schema_version: must be {MANIFEST_SCHEMA_VERSION}, found {}",
                self.schema_version
            ));
        }
        decisions_issues(&self.decisions, &mut issues);
        match self.image.digest.strip_prefix("sha256:") {
            Some(hex) if is_lower_hex(hex, 64) => {}
            _ => issues.push(
                "image.digest: must be sha256: followed by 64 lowercase hex digits".to_owned(),
            ),
        }
        if self.baseline.cflags != BASELINE_CFLAGS {
            issues.push(format!(
                "baseline.cflags: must be exactly {BASELINE_CFLAGS:?} in this order (CONTEXT D-06), found {:?}",
                self.baseline.cflags
            ));
        }
        seed_caps_issues(&self.seed_caps, &mut issues);
        for (id, target) in &self.target {
            target_issues(id, target, &mut issues);
        }
        issues
    }
}

/// The ecosystems `decisions.first_rtos` may name.
const RTOS_CHOICES: [&str; 3] = ["deferred", "freertos", "zephyr"];

/// Sort and deduplicate the issues; an empty list is success.
fn finish(mut issues: Vec<String>) -> Result<(), ManifestError> {
    issues.sort();
    issues.dedup();
    if issues.is_empty() {
        Ok(())
    } else {
        Err(ManifestError::Invalid { issues })
    }
}

fn is_lower_hex(text: &str, len: usize) -> bool {
    text.len() == len && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

fn check_hex(issues: &mut Vec<String>, field: &str, value: &str, len: usize) {
    if !is_lower_hex(value, len) {
        issues.push(format!("{field}: must be {len} lowercase hex digits"));
    }
}

fn check_https(issues: &mut Vec<String>, field: &str, url: &str) {
    let host = url.strip_prefix("https://").unwrap_or("");
    if host.is_empty() || url.chars().any(char::is_whitespace) {
        issues.push(format!(
            "{field}: must be an https:// URL without whitespace"
        ));
    }
}

/// `D-` followed by one or more digits.
fn is_decision_id(text: &str) -> bool {
    text.strip_prefix("D-")
        .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
}

/// A relative path with no `..` component and no NUL (CONTEXT D-20, T-02-10).
fn is_safe_relative_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !Path::new(path).is_absolute()
        && !path.contains('\0')
        && !path.split('/').any(|part| part == "..")
}

fn check_path(issues: &mut Vec<String>, field: &str, path: &str) {
    if !is_safe_relative_path(path) {
        issues.push(format!(
            "{field}: must be a non-empty relative path with no `..` component and no NUL, found {path:?}"
        ));
    }
}

/// `[A-Za-z_][A-Za-z0-9_]*`.
fn is_c_identifier(text: &str) -> bool {
    let mut bytes = text.bytes();
    bytes
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// One SPDX identifier: letters, digits, `-`, `.` and `+`.
fn is_spdx_identifier(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'+'))
}

/// True when `text` contains a `{` that does not start one of [`PLACEHOLDERS`].
fn has_unknown_brace(text: &str) -> bool {
    let mut rest = text;
    while let Some(at) = rest.find('{') {
        let tail = &rest[at..];
        match PLACEHOLDERS.iter().find(|p| tail.starts_with(**p)) {
            Some(placeholder) => rest = &tail[placeholder.len()..],
            None => return true,
        }
    }
    false
}

fn check_placeholders(issues: &mut Vec<String>, field: &str, values: &[String]) {
    for (i, value) in values.iter().enumerate() {
        if has_unknown_brace(value) {
            issues.push(format!(
                "{field}[{i}]: `{{` may only start one of {}, found {value:?}",
                PLACEHOLDERS.join(", ")
            ));
        }
    }
}

fn decisions_issues(decisions: &Decisions, issues: &mut Vec<String>) {
    if !RTOS_CHOICES.contains(&decisions.first_rtos.as_str()) {
        issues.push(format!(
            "decisions.first_rtos: must be one of {RTOS_CHOICES:?}, found {:?}",
            decisions.first_rtos
        ));
    }
    if decisions.first_rtos == "deferred" && decisions.first_rtos_reason.trim().is_empty() {
        issues.push("decisions.first_rtos_reason: required when first_rtos is deferred".to_owned());
    }
    if decisions.first_rtos_candidates.is_empty()
        || decisions
            .first_rtos_candidates
            .iter()
            .any(|c| c.trim().is_empty())
    {
        issues.push("decisions.first_rtos_candidates: must name at least one candidate".to_owned());
    }
    if !is_decision_id(&decisions.project_decision) {
        issues.push("decisions.project_decision: must look like D-18".to_owned());
    }
}

fn target_issues(id: &str, target: &Target, issues: &mut Vec<String>) {
    let f = format!("target.{id}");
    if id.is_empty()
        || !id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
    {
        issues.push(format!(
            "{f}: the id must be lower-case letters, digits and underscores"
        ));
    }
    if target.category.is_empty() {
        issues.push(format!("{f}.category: must name at least one category"));
    }
    for (i, category) in target.category.iter().enumerate() {
        if target.category[..i].contains(category) {
            issues.push(format!("{f}.category: {category:?} is listed twice"));
        }
    }
    source_issues(&f, &target.source, issues);
    licence_issues(&f, &target.licence, issues);
    if let Some(args) = &target.build_args {
        build_args_issues(&f, target.build, args, issues);
    }
    entry_issues(&f, &target.entry, issues);
    config_issues(&f, target, issues);
    seed_source_issues(&f, target, issues);
}

fn seed_caps_issues(caps: &SeedCaps, issues: &mut Vec<String>) {
    for (name, value) in [
        ("max_seeds", caps.max_seeds),
        ("max_total_bytes", caps.max_total_bytes),
        ("max_seed_bytes", caps.max_seed_bytes),
    ] {
        if value == 0 {
            issues.push(format!("seed_caps.{name}: must be above zero"));
        }
    }
    if caps.max_seed_bytes > caps.max_total_bytes {
        issues.push(format!(
            "seed_caps.max_seed_bytes: {} is above max_total_bytes {}, so no seed of that size could fit",
            caps.max_seed_bytes, caps.max_total_bytes
        ));
    }
}

/// A file extension a seed source may name: letters, digits, `_`, `-` and `+`,
/// or the empty string for files without an extension.
fn is_extension(text: &str) -> bool {
    text.bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'+'))
}

fn seed_source_issues(f: &str, target: &Target, issues: &mut Vec<String>) {
    for (i, source) in target.seed_source.iter().enumerate() {
        let g = format!("{f}.seed_source[{i}]");
        check_path(issues, &format!("{g}.path"), &source.path);
        // The whole tree is not an allow-list, and `.git` is not source (Pitfall 18).
        if source.path.split('/').any(|part| part == ".git")
            || source
                .path
                .split('/')
                .all(|part| part.is_empty() || part == ".")
        {
            issues.push(format!(
                "{g}.path: must name a part of the tree, not the whole tree or .git, found {:?}",
                source.path
            ));
        }
        if source.methods.is_empty() {
            issues.push(format!("{g}.methods: must name at least one method"));
        }
        for method in &source.methods {
            if !SEED_METHODS.contains(&method.as_str()) {
                issues.push(format!(
                    "{g}.methods: {method:?} is not one of {SEED_METHODS:?}"
                ));
            }
        }
        if let Some(extensions) = &source.extensions {
            if extensions.is_empty() {
                issues.push(format!(
                    "{g}.extensions: leave it out to take every file, or list at least one"
                ));
            }
            for extension in extensions {
                if !is_extension(extension) {
                    issues.push(format!(
                        "{g}.extensions: {extension:?} must be letters, digits, `_`, `-` or `+` only (no dot, no slash); use \"\" for files without an extension"
                    ));
                }
            }
        }
        for entry in &source.entries {
            if !target.entry.iter().any(|e| &e.name == entry) {
                issues.push(format!(
                    "{g}.entries: {entry:?} is not an entry function of the target"
                ));
            }
        }
        if let Some(designators) = &source.designators {
            if designators.is_empty() {
                issues.push(format!(
                    "{g}.designators: leave it out to take every string, or list at least one member name"
                ));
            }
            if source.methods != ["c_string"] {
                issues.push(format!(
                    "{g}.designators: only valid with methods exactly [\"c_string\"], found {:?}",
                    source.methods
                ));
            }
            for name in designators {
                if !is_c_identifier(name) {
                    issues.push(format!(
                        "{g}.designators: {name:?} is not a C identifier (letters, digits and `_`, not starting with a digit)"
                    ));
                }
            }
        }
    }
}

fn source_issues(f: &str, source: &Source, issues: &mut Vec<String>) {
    let f = format!("{f}.source");
    let (url, reference, commit, tree_sha256) = match source {
        Source::Git {
            url,
            reference,
            commit,
            tree_sha256,
        } => (url, reference, commit, tree_sha256),
        Source::Archive {
            url,
            sha256,
            strip_components,
            reference,
            commit,
            tree_sha256,
        } => {
            check_hex(issues, &format!("{f}.sha256"), sha256, 64);
            if *strip_components > MAX_STRIP_COMPONENTS {
                issues.push(format!(
                    "{f}.strip_components: must be at most {MAX_STRIP_COMPONENTS}, found {strip_components}"
                ));
            }
            (url, reference, commit, tree_sha256)
        }
    };
    check_https(issues, &format!("{f}.url"), url);
    if reference.trim().is_empty() {
        issues.push(format!("{f}.ref: must not be empty"));
    }
    check_hex(issues, &format!("{f}.commit"), commit, 40);
    check_hex(issues, &format!("{f}.tree_sha256"), tree_sha256, 64);
}

/// The most leading path components an archive source may strip.
const MAX_STRIP_COMPONENTS: u32 = 4;

fn licence_issues(f: &str, licence: &Licence, issues: &mut Vec<String>) {
    let f = format!("{f}.licence");
    let operands: Vec<&str> = licence.spdx.split(" OR ").collect();
    if !operands.iter().all(|operand| is_spdx_identifier(operand)) {
        issues.push(format!(
            "{f}.spdx: must be one SPDX identifier or identifiers joined by ' OR ' (no AND, WITH or parentheses), found {:?}",
            licence.spdx
        ));
    }
    if !operands.contains(&licence.elected.as_str()) {
        issues.push(format!(
            "{f}.elected: {:?} is not one of the operands of spdx {:?}",
            licence.elected, licence.spdx
        ));
    }
    if !PERMISSIVE_LICENCES.contains(&licence.elected.as_str()) {
        issues.push(format!(
            "{f}.elected: {:?} is not on the permissive allow-list {PERMISSIVE_LICENCES:?} (CONTEXT D-02, D-27)",
            licence.elected
        ));
    }
    check_path(issues, &format!("{f}.file"), &licence.file);
}

fn build_args_issues(f: &str, build: BuildSystem, args: &BuildArgs, issues: &mut Vec<String>) {
    let f = format!("{f}.build_args");
    check_path(issues, &format!("{f}.dir"), &args.dir);
    match build {
        BuildSystem::Make => {
            if args.goals.is_empty() {
                issues.push(format!("{f}.goals: a make target needs at least one goal"));
            }
            if args.cmake_target.is_some() {
                issues.push(format!("{f}.cmake_target: not allowed on a make target"));
            }
        }
        BuildSystem::Cmake => {
            if args.cmake_target.is_none() {
                issues.push(format!("{f}.cmake_target: a cmake target needs one"));
            }
            if !args.goals.is_empty() {
                issues.push(format!("{f}.goals: not allowed on a cmake target"));
            }
        }
    }
    check_placeholders(issues, &format!("{f}.goals"), &args.goals);
    check_placeholders(issues, &format!("{f}.make_vars"), &args.make_vars);
    check_placeholders(issues, &format!("{f}.cmake_options"), &args.cmake_options);
    if let Some(target) = &args.cmake_target {
        check_placeholders(
            issues,
            &format!("{f}.cmake_target"),
            std::slice::from_ref(target),
        );
    }
}

fn entry_issues(f: &str, entries: &[Entry], issues: &mut Vec<String>) {
    let f = format!("{f}.entry");
    if !(1..=3).contains(&entries.len()) {
        issues.push(format!(
            "{f}: needs 1 to 3 entry functions (CONTEXT D-24), found {}",
            entries.len()
        ));
    }
    for (i, entry) in entries.iter().enumerate() {
        let g = format!("{f}.{}", entry.name);
        if !is_c_identifier(&entry.name) {
            issues.push(format!("{g}.name: must be a C identifier"));
        }
        if entries[..i].iter().any(|e| e.name == entry.name) {
            issues.push(format!("{f}: name {:?} is used twice", entry.name));
        }
        check_path(issues, &format!("{g}.header"), &entry.header);
        check_path(issues, &format!("{g}.source"), &entry.source);
        if entry.signature.trim().is_empty() {
            issues.push(format!("{g}.signature: must not be empty"));
        }
        match (entry.form, entry.accepted_wrapped) {
            (EntryForm::Wrapped, false) => issues.push(format!(
                "{g}.accepted_wrapped: a wrapped entry needs accepted_wrapped = true (CONTEXT D-28)"
            )),
            (EntryForm::Direct | EntryForm::Range, true) => issues.push(format!(
                "{g}.accepted_wrapped: only allowed on a wrapped entry"
            )),
            _ => {}
        }
        role_issues(&g, entry, issues);
        local_issues(&g, entry, issues);
        setup_issues(&g, entry, issues);
        mark_issues(&g, entry, issues);
    }
}

/// The largest output capacity an `out_cap` may take: the executor's largest
/// input (a test in `mt-harness` ties the two), so every combination can run.
pub const MAX_OUT_CAP: u64 = 65_536;

/// Names a `scalar` (or a named `out_cap`) may not take: the top-level names of
/// a vector's library state and the regions of a pointer.
pub const RESERVED_ARG_NAMES: [&str; 5] = ["in", "out", "globals", "heap", "ret"];

/// Parameter types narrower than 32 bits: an `in_len` of one of these needs a
/// `max`, because a longer seed would wrap in the call.
const NARROW_LENGTH_TYPES: [&str; 9] = [
    "uint8_t",
    "int8_t",
    "unsigned char",
    "char",
    "signed char",
    "uint16_t",
    "int16_t",
    "unsigned short",
    "short",
];

/// The rules for an entry's `arg` tables (CONTEXT D-11, D-14): one table per
/// parameter, a pointer role only on a pointer parameter and a value role only
/// on a value parameter, every input buffer paired with its length and every
/// output buffer with its capacity, the keys each role takes, names that are C
/// identifiers, not reserved and not repeated, and at most
/// [`MAX_SCALAR_COMBOS`] combinations. An entry without tables is not checked,
/// so Phase 2's manifests validate as before.
fn role_issues(g: &str, entry: &Entry, issues: &mut Vec<String>) {
    if entry.arg.is_empty() {
        return;
    }
    let f = format!("{g}.arg");
    let params = match split_parameters(&entry.signature) {
        Ok((_, params)) => params,
        Err(error) => {
            issues.push(format!("{f}: {error} (D-11)"));
            return;
        }
    };
    if params.len() != entry.arg.len() {
        issues.push(format!(
            "{f}: {} arg tables but the signature has {} parameters (D-11)",
            entry.arg.len(),
            params.len()
        ));
        return;
    }
    for (index, (arg, param)) in entry.arg.iter().zip(&params).enumerate() {
        let h = format!("{f}[{index}]");
        let word = arg.role.word();
        let pointer = is_pointer_parameter(param);
        if arg.role.is_pointer_role() && !pointer {
            issues.push(format!(
                "{h}: role {word} needs a pointer parameter, found {param:?} (D-11)"
            ));
        } else if !arg.role.is_pointer_role() && pointer {
            issues.push(format!(
                "{h}: role {word} needs a value parameter, found the pointer {param:?} (D-11)"
            ));
        }
        if arg.role == Role::State && pointer && state_pointee_type(param).is_none() {
            issues.push(format!(
                "{h}: the pointee of a state parameter must be a plain type name, optionally after struct, union or enum, and not void; found {param:?} (D-09)"
            ));
        }
        if matches!(arg.role, Role::InoutLen | Role::OutScalar)
            && pointer
            && value_pointee_type(param).is_none()
        {
            issues.push(format!(
                "{h}: the pointee of an {word} parameter must be a plain scalar type name and not const; found {param:?} (D-11)"
            ));
        }
        if arg.role == Role::Wrap && !pointer && !is_closed_type_text(param) {
            issues.push(format!(
                "{h}: role wrap: type {param:?} does not match {TYPE_TEXT_GRAMMAR} (D-11)"
            ));
        }
        if arg.role == Role::OutPtr && pointer && pointer_pointee_type(param).is_none() {
            issues.push(format!(
                "{h}: role out_ptr needs a pointer to a pointer, found {param:?} (D-11)"
            ));
        }
        arg_key_issues(&h, arg, param, issues);
        local_key_issues(&h, arg, entry, issues);
    }
    let count = |role: Role| entry.arg.iter().filter(|arg| arg.role == role).count();
    let str_names: Vec<&str> = entry
        .arg
        .iter()
        .filter(|arg| arg.role == Role::Str)
        .filter_map(|arg| arg.name.as_deref())
        .collect();
    for (index, arg) in entry.arg.iter().enumerate() {
        if arg.role == Role::StrLen
            && let Some(of) = arg.of.as_deref()
            && !str_names.contains(&of)
        {
            issues.push(format!(
                "{f}[{index}]: str_len of {of:?} names no str of the entry (D-11)"
            ));
        }
    }
    // An inout_len with `of = "out"` is the capacity of the output buffer.
    let sizing = entry
        .arg
        .iter()
        .filter(|arg| arg.role == Role::InoutLen && arg.of.as_deref() == Some("out"))
        .count();
    if sizing > 1 {
        issues.push(format!(
            "{f}: {sizing} inout_len tables with of = \"out\"; the output buffer has one capacity (D-11)"
        ));
    }
    if sizing > 0 && count(Role::OutCap) > 0 {
        issues.push(format!(
            "{f}: an inout_len with of = \"out\" is the capacity of the output buffer, so the entry has no out_cap as well (D-11)"
        ));
    }
    if sizing > 0 && count(Role::OutBuf) == 0 {
        issues.push(format!(
            "{f}: an inout_len with of = \"out\" needs an out_buf to size (D-11)"
        ));
    }
    // An inout_len with `of = "in"` is the length of the input: it is the in_buf's
    // length, so the entry has no in_len or wrap as well (plan 03-08).
    let counting = entry
        .arg
        .iter()
        .filter(|arg| arg.role == Role::InoutLen && arg.of.as_deref() == Some("in"))
        .count();
    if counting > 1 {
        issues.push(format!(
            "{f}: {counting} inout_len tables with of = \"in\"; the input has one length (D-11)"
        ));
    }
    if counting > 0 && (count(Role::InLen) > 0 || count(Role::Wrap) > 0) {
        issues.push(format!(
            "{f}: an inout_len with of = \"in\" is the length of the input, so the entry has no in_len or wrap as well (D-11)"
        ));
    }
    if counting > 0 && count(Role::InBuf) == 0 {
        issues.push(format!(
            "{f}: an inout_len with of = \"in\" needs an in_buf to measure (D-11)"
        ));
    }
    wrap_and_input_issues(g, &f, entry, issues);
    for (buf, len, buf_word, len_word) in [
        (Role::InBuf, Role::InLen, "in_buf", "in_len"),
        (Role::OutBuf, Role::OutCap, "out_buf", "out_cap"),
    ] {
        let (bufs, lens) = (
            count(buf),
            count(len)
                + if buf == Role::OutBuf {
                    sizing
                } else {
                    counting
                },
        );
        if bufs > 1 {
            issues.push(format!("{f}: {bufs} {buf_word} tables; at most one (D-11)"));
        }
        if bufs > lens {
            issues.push(format!(
                "{f}: an {buf_word} needs its {len_word}, found {lens} {len_word} for {bufs} {buf_word} (D-11)"
            ));
        } else if lens > bufs {
            issues.push(format!(
                "{f}: an {len_word} needs an {buf_word}, found {lens} {len_word} for {bufs} {buf_word} (D-11)"
            ));
        }
    }
    name_issues(&f, entry, issues);
    let combinations: usize = entry
        .arg
        .iter()
        .filter(|arg| {
            matches!(
                arg.role,
                Role::OutCap | Role::Scalar | Role::InoutLen | Role::Str
            ) && !(arg.role == Role::InoutLen && arg.of.as_deref() == Some("in"))
        })
        .map(|arg| {
            if arg.role == Role::Str {
                arg.values.len().max(1)
            } else {
                1 + arg.sweep.len()
            }
        })
        .try_fold(1usize, usize::checked_mul)
        .unwrap_or(usize::MAX);
    if combinations > MAX_SCALAR_COMBOS {
        issues.push(format!(
            "{f}: {combinations} scalar combinations exceed the limit of {MAX_SCALAR_COMBOS} (D-14)"
        ));
    }
}

/// The rules that join the input to the entry (CONTEXT D-11, D-12): at most one
/// `wrap`, none beside an `in_buf` or an `in_len` (it stands for both), a wrap only
/// on an entry with `accepted_wrapped = true`, and an input that is reached: by an
/// `in_buf`, a `wrap`, or `$in` in a setup call or a `set` value.
fn wrap_and_input_issues(g: &str, f: &str, entry: &Entry, issues: &mut Vec<String>) {
    let count = |role: Role| entry.arg.iter().filter(|arg| arg.role == role).count();
    let wraps = count(Role::Wrap);
    if wraps > 1 {
        issues.push(format!("{f}: {wraps} wrap tables; at most one (D-12)"));
    }
    if wraps > 0 && (count(Role::InBuf) > 0 || count(Role::InLen) > 0) {
        issues.push(format!(
            "{f}: a wrap stands for the in_buf and the in_len, so the entry has neither beside it (D-12)"
        ));
    }
    if wraps > 0 && !entry.accepted_wrapped {
        issues.push(format!(
            "{g}.accepted_wrapped: a wrap needs accepted_wrapped = true (D-12, D-28)"
        ));
    }
    let named_by_setup = entry.setup.iter().any(|step| {
        step.args
            .iter()
            .any(|text| SetupArg::parse(text) == Some(SetupArg::In))
            || step
                .value
                .as_deref()
                .and_then(SetValue::parse)
                .is_some_and(|value| {
                    matches!(
                        value,
                        SetValue::In | SetValue::InPlus(_) | SetValue::InByte(_)
                    )
                })
    });
    if count(Role::InBuf) == 0 && wraps == 0 && !named_by_setup {
        issues.push(format!(
            "{f}: the input is never reached; declare an in_buf with its in_len, a wrap, or name $in in a setup step (D-12)"
        ));
    }
}

/// The names the roles of an entry are recorded under: scalars, capacities,
/// states, lengths and the pointer outputs. An unnamed `out_cap` is `out_cap`.
fn role_names(entry: &Entry) -> Vec<String> {
    entry
        .arg
        .iter()
        .filter_map(|arg| match (arg.role, &arg.name) {
            (Role::OutCap, None) => Some("out_cap".to_owned()),
            (
                Role::OutCap
                | Role::Scalar
                | Role::State
                | Role::InoutLen
                | Role::OutScalar
                | Role::OutPtr
                | Role::Str,
                Some(name),
            ) => Some(name.clone()),
            _ => None,
        })
        .collect()
}

/// The rules for an entry's `local` tables (CONTEXT D-12): a C identifier that is
/// not reserved and not repeated among the role names and the other locals, a type
/// text of the closed grammar and a count of 1 to [`MAX_OBJECT_COUNT`]. Locals and
/// setup steps belong to an entry that has roles.
fn local_issues(g: &str, entry: &Entry, issues: &mut Vec<String>) {
    if entry.arg.is_empty() {
        if !entry.local.is_empty() || !entry.setup.is_empty() {
            issues.push(format!(
                "{g}: local and setup tables need the entry's arg roles (D-12)"
            ));
        }
        return;
    }
    let mut seen = role_names(entry);
    for (index, local) in entry.local.iter().enumerate() {
        let h = format!("{g}.local[{index}]");
        if !is_c_identifier(&local.name) {
            issues.push(format!(
                "{h}: name {:?} is not a C identifier (D-12)",
                local.name
            ));
        }
        if RESERVED_ARG_NAMES.contains(&local.name.as_str()) {
            issues.push(format!(
                "{h}: name {:?} is reserved ({}) (D-12)",
                local.name,
                RESERVED_ARG_NAMES.join(", ")
            ));
        }
        if seen.contains(&local.name) {
            issues.push(format!("{h}: duplicate name {:?} (D-12)", local.name));
        }
        seen.push(local.name.clone());
        if !is_closed_type_text(&local.type_text) {
            issues.push(format!(
                "{h}: type {:?} does not match {TYPE_TEXT_GRAMMAR} (D-11, D-12)",
                local.type_text
            ));
        }
        if let Some(count) = local.count
            && !(1..=MAX_OBJECT_COUNT).contains(&count)
        {
            issues.push(format!(
                "{h}: count {count} is outside 1..={MAX_OBJECT_COUNT} (D-12)"
            ));
        }
    }
}

/// True when a local's text names a pointer.
fn local_is_pointer(local: &LocalObject) -> bool {
    local.type_text.ends_with(" *") && local.count.is_none_or(|count| count == 1)
}

/// The rules for an entry's `setup` steps (CONTEXT D-12, T-03-25): at most
/// [`MAX_SETUP_STEPS`]; each step exactly one of a call or a set; a call names a C
/// identifier, takes arguments of the closed grammar that name something the entry
/// declares and expects `void` or a 32-bit decimal; a set names a declared local or
/// state, a member path and a value of the closed grammar.
fn setup_issues(g: &str, entry: &Entry, issues: &mut Vec<String>) {
    if entry.arg.is_empty() {
        return;
    }
    if entry.setup.len() > MAX_SETUP_STEPS {
        issues.push(format!(
            "{g}.setup: {} steps; at most {MAX_SETUP_STEPS} (D-12)",
            entry.setup.len()
        ));
    }
    let objects: Vec<&str> = entry
        .arg
        .iter()
        .filter(|arg| arg.role == Role::State)
        .filter_map(|arg| arg.name.as_deref())
        .chain(entry.local.iter().map(|local| local.name.as_str()))
        .collect();
    let scalars: Vec<&str> = entry
        .arg
        .iter()
        .filter(|arg| arg.role == Role::Scalar)
        .filter_map(|arg| arg.name.as_deref())
        .collect();
    for (index, step) in entry.setup.iter().enumerate() {
        let h = format!("{g}.setup[{index}]");
        match (&step.call, &step.set) {
            (Some(call), None) => {
                call_step_issues(&h, step, call, &objects, &scalars, entry, issues);
            }
            (None, Some(path)) => set_step_issues(&h, step, path, &objects, issues),
            _ => issues.push(format!("{h}: exactly one of call or set (D-12)")),
        }
    }
}

fn call_step_issues(
    h: &str,
    step: &SetupStep,
    call: &str,
    objects: &[&str],
    scalars: &[&str],
    entry: &Entry,
    issues: &mut Vec<String>,
) {
    if !is_function_name(call) {
        issues.push(format!("{h}: call {call:?} is not a C identifier (D-12)"));
    }
    if step.value.is_some() {
        issues.push(format!("{h}: a call step takes no value (D-12)"));
    }
    match step.expect.as_deref().map(Expect::parse) {
        Some(Some(_)) => {}
        Some(None) => issues.push(format!(
            "{h}: expect {:?} must be void or a decimal integer within 32 bits (D-12)",
            step.expect.as_deref().unwrap_or_default()
        )),
        None => issues.push(format!("{h}: a call step needs expect (D-12)")),
    }
    for text in &step.args {
        let fail = |why: &str| {
            format!(
                "{h}: argument {text:?} {why}; the closed grammar is $in, $len, $cap, $arg:<scalar>, a decimal integer, &<state or local>, <pointer local> and sizeof(<local>) (D-12)"
            )
        };
        match SetupArg::parse(text) {
            None => issues.push(fail("is not in the closed grammar")),
            Some(SetupArg::AddressOf(name)) if !objects.contains(&name.as_str()) => {
                issues.push(fail("names no state or local of the entry"));
            }
            Some(SetupArg::Scalar(name)) if !scalars.contains(&name.as_str()) => {
                issues.push(fail("names no scalar of the entry"));
            }
            Some(SetupArg::Local(name))
                if !entry
                    .local
                    .iter()
                    .any(|local| local.name == name && local_is_pointer(local)) =>
            {
                issues.push(fail("names no pointer local of the entry"));
            }
            Some(SetupArg::SizeOf(name)) if !entry.local.iter().any(|local| local.name == name) => {
                issues.push(fail("names no local of the entry"));
            }
            Some(_) => {}
        }
    }
}

fn set_step_issues(
    h: &str,
    step: &SetupStep,
    path: &str,
    objects: &[&str],
    issues: &mut Vec<String>,
) {
    if !step.args.is_empty() || step.expect.is_some() {
        issues.push(format!("{h}: a set step takes no args or expect (D-12)"));
    }
    match parse_set_path(path) {
        None => issues.push(format!(
            "{h}: path {path:?} must be <name>.<member>(.<member>)* in C identifiers (D-12)"
        )),
        Some((name, _)) if !objects.contains(&name.as_str()) => issues.push(format!(
            "{h}: path {path:?} names no state or local of the entry (D-12)"
        )),
        Some(_) => {}
    }
    match step.value.as_deref().map(SetValue::parse) {
        Some(Some(_)) => {}
        Some(None) => issues.push(format!(
            "{h}: value {:?} must be $in, $in+<n>, $in[<n>], $len, $len-<n> or a decimal integer (D-12)",
            step.value.as_deref().unwrap_or_default()
        )),
        None => issues.push(format!("{h}: a set step needs a value (D-12)")),
    }
}

/// The rules for an entry's unobserved marks (CONTEXT D-09): a reason that is
/// not blank, a path of the form `<state name>(.<member>|[<index>])+`, a state
/// that a `state` role or a `local` of the same entry declares, and no field
/// marked twice. The
/// members themselves are checked against the layout when the driver is built.
fn mark_issues(g: &str, entry: &Entry, issues: &mut Vec<String>) {
    let states: Vec<&str> = entry
        .arg
        .iter()
        .filter(|arg| arg.role == Role::State)
        .filter_map(|arg| arg.name.as_deref())
        .chain(entry.local.iter().map(|local| local.name.as_str()))
        .collect();
    let mut seen: Vec<&str> = Vec::new();
    for (index, mark) in entry.unobserved.iter().enumerate() {
        let h = format!("{g}.unobserved[{index}]");
        if mark.reason.trim().is_empty() {
            issues.push(format!(
                "{h}: an unobserved mark needs a reason, found {:?} (D-09)",
                mark.reason
            ));
        }
        match parse_mark_path(&mark.field) {
            None => issues.push(format!(
                "{h}: field {:?} must be <state name>(.<member>|[<index>])+ in C identifiers and decimal indices (D-09)",
                mark.field
            )),
            Some((state, _)) if !states.contains(&state.as_str()) => issues.push(format!(
                "{h}: field {:?} names no state declared by a state role of this entry (D-09)",
                mark.field
            )),
            Some(_) => {}
        }
        if seen.contains(&mark.field.as_str()) {
            issues.push(format!(
                "{h}: duplicate mark on field {:?}; mark a field once (D-09)",
                mark.field
            ));
        }
        seen.push(&mark.field);
    }
}

/// The key `local` (plan 03-08): a `local` role needs it and it names a pointer
/// local of the entry; no other role takes it.
fn local_key_issues(h: &str, arg: &ArgRole, entry: &Entry, issues: &mut Vec<String>) {
    match (arg.role, arg.local.as_deref()) {
        (Role::Local, None) => issues.push(format!(
            "{h}: role local needs the key local, naming a pointer local of the entry (D-12)"
        )),
        (Role::Local, Some(name)) => {
            if !entry
                .local
                .iter()
                .any(|local| local.name == name && local_is_pointer(local))
            {
                issues.push(format!(
                    "{h}: local {name:?} names no pointer local of the entry (D-12)"
                ));
            }
        }
        (_, Some(_)) => issues.push(format!("{h}: local is only allowed on local (D-12)")),
        (_, None) => {}
    }
}

/// The keys one role takes and the values they may have.
fn arg_key_issues(h: &str, arg: &ArgRole, param: &str, issues: &mut Vec<String>) {
    let word = arg.role.word();
    // An inout_len initialised from the input length has no value of its own.
    let from_input = arg.role == Role::InoutLen && arg.of.as_deref() == Some("in");
    let takes_value =
        matches!(arg.role, Role::OutCap | Role::Scalar | Role::InoutLen) && !from_input;
    match arg.role {
        Role::InBuf
        | Role::OutBuf
        | Role::InLen
        | Role::StrLen
        | Role::Wrap
        | Role::Null
        | Role::Local => {
            if arg.name.is_some() {
                issues.push(format!("{h}: role {word} takes no name (D-11)"));
            }
        }
        Role::Scalar
        | Role::State
        | Role::InoutLen
        | Role::OutScalar
        | Role::OutPtr
        | Role::Str => match &arg.name {
            None => issues.push(format!("{h}: role {word} needs a name (D-11)")),
            Some(name) if !is_c_identifier(name) => {
                issues.push(format!("{h}: name {name:?} is not a C identifier (D-11)"));
            }
            Some(_) => {}
        },
        Role::OutCap => {
            if let Some(name) = &arg.name
                && !is_c_identifier(name)
            {
                issues.push(format!("{h}: name {name:?} is not a C identifier (D-11)"));
            }
        }
    }
    if takes_value {
        match arg.default {
            None => issues.push(format!("{h}: role {word} needs a default (D-14)")),
            Some(default) => {
                let mut seen = vec![default];
                for value in &arg.sweep {
                    if seen.contains(value) {
                        issues.push(format!(
                            "{h}: the sweep repeats the value {value}; every combination must differ (D-14)"
                        ));
                    }
                    seen.push(*value);
                }
            }
        }
        if matches!(arg.role, Role::OutCap | Role::InoutLen) {
            let limit = i64::try_from(MAX_OUT_CAP).unwrap_or(i64::MAX);
            for value in arg.default.iter().chain(&arg.sweep) {
                if !(0..=limit).contains(value) {
                    issues.push(format!(
                        "{h}: {word} value {value} is outside 0..={MAX_OUT_CAP} (D-14)"
                    ));
                }
            }
        }
    } else if from_input {
        if arg.default.is_some() || !arg.sweep.is_empty() {
            issues.push(format!(
                "{h}: role inout_len with of = \"in\" takes no default or sweep; its initial value is the input length (D-11)"
            ));
        }
    } else if arg.default.is_some() || !arg.sweep.is_empty() {
        issues.push(format!(
            "{h}: default and sweep belong to out_cap, scalar and inout_len, not {word} (D-14)"
        ));
    }
    match (arg.role, arg.of.as_deref()) {
        (Role::InoutLen, None | Some("out" | "in")) => {}
        (Role::InoutLen, Some(other)) => issues.push(format!(
            "{h}: of = {other:?} names neither the output buffer nor the input; the values are \"out\" and \"in\" (D-11)"
        )),
        (Role::StrLen, None) => issues.push(format!(
            "{h}: role str_len needs `of` naming its str (D-11)"
        )),
        (Role::StrLen, Some(_)) | (_, None) => {}
        (_, Some(_)) => issues.push(format!(
            "{h}: of is only allowed on inout_len and str_len (D-11)"
        )),
    }
    str_value_issues(h, arg, issues);
    object_key_issues(h, arg, issues);
    if arg.mutable {
        if arg.role != Role::InBuf {
            issues.push(format!("{h}: mutable is only allowed on in_buf (D-11)"));
        } else if pointee_is_const(param) {
            issues.push(format!(
                "{h}: a mutable in_buf needs a pointer to non-const data, found {param:?} (D-11)"
            ));
        }
    }
    match (arg.role, arg.max) {
        (Role::InLen, None) if NARROW_LENGTH_TYPES.contains(&param) => issues.push(format!(
            "{h}: role in_len on a {param:?} parameter needs a max, because a longer input would wrap (D-11)"
        )),
        (Role::InLen, Some(0)) => issues.push(format!("{h}: max must be above zero (D-11)")),
        (Role::InLen, _) | (_, None) => {}
        (_, Some(_)) => issues.push(format!("{h}: max is only allowed on in_len (D-11)")),
    }
}

/// The keys `count`, `ptr_field` and `len_field` take (CONTEXT D-12): `count`
/// belongs to a `state` and is 1 to [`MAX_OBJECT_COUNT`]; the two members belong to
/// a `wrap`, are C identifiers and differ.
fn object_key_issues(h: &str, arg: &ArgRole, issues: &mut Vec<String>) {
    match (arg.role, arg.count) {
        (Role::State, Some(count)) if !(1..=MAX_OBJECT_COUNT).contains(&count) => {
            issues.push(format!(
                "{h}: count {count} is outside 1..={MAX_OBJECT_COUNT} (D-12, T-03-27)"
            ));
        }
        (Role::State, _) | (_, None) => {}
        (_, Some(_)) => issues.push(format!("{h}: count is only allowed on state (D-12)")),
    }
    if arg.role != Role::Wrap {
        for (key, value) in [("ptr_field", &arg.ptr_field), ("len_field", &arg.len_field)] {
            if value.is_some() {
                issues.push(format!("{h}: {key} is only allowed on wrap (D-12)"));
            }
        }
        return;
    }
    for (key, value) in [("ptr_field", &arg.ptr_field), ("len_field", &arg.len_field)] {
        match value {
            None => issues.push(format!("{h}: role wrap needs {key} (D-12)")),
            Some(member) if !is_c_identifier(member) => issues.push(format!(
                "{h}: {key} {member:?} is not a C identifier (D-12)"
            )),
            Some(_) => {}
        }
    }
    if arg.ptr_field.is_some() && arg.ptr_field == arg.len_field {
        issues.push(format!(
            "{h}: ptr_field and len_field must name different members (D-12)"
        ));
    }
}

/// True when the data a pointer parameter points at is `const`: a `const` before
/// the last `*`.
fn pointee_is_const(param: &str) -> bool {
    let spaced = param.replace('*', " * ");
    let before: Vec<&str> = spaced
        .split_whitespace()
        .take_while(|token| *token != "*")
        .collect();
    before.contains(&"const")
}

/// The rules for the declared values of a `str` (T-03-24): one to eight values of
/// at most 64 printable ASCII bytes, none repeated; `values` belongs to `str`.
fn str_value_issues(h: &str, arg: &ArgRole, issues: &mut Vec<String>) {
    if arg.role != Role::Str {
        if !arg.values.is_empty() {
            issues.push(format!("{h}: values is only allowed on str (D-11)"));
        }
        return;
    }
    if arg.values.is_empty() || arg.values.len() > MAX_STR_VALUES {
        issues.push(format!(
            "{h}: role str needs 1 to {MAX_STR_VALUES} values, found {} (D-11)",
            arg.values.len()
        ));
    }
    for (index, value) in arg.values.iter().enumerate() {
        if value.len() > MAX_STR_BYTES {
            issues.push(format!(
                "{h}: str value {index} is {} bytes; at most {MAX_STR_BYTES} (T-03-24)",
                value.len()
            ));
        }
        if !value.bytes().all(|byte| (0x20..=0x7e).contains(&byte)) {
            issues.push(format!(
                "{h}: str value {index} must be printable ASCII (0x20 to 0x7E) (T-03-24)"
            ));
        }
        if arg.values[..index].contains(value) {
            issues.push(format!(
                "{h}: the str values repeat {value:?}; every combination must differ (D-14)"
            ));
        }
    }
}

/// Names recorded for `out_cap` and `scalar` values: not reserved, not repeated.
/// An unnamed `out_cap` is recorded as `out_cap`.
fn name_issues(f: &str, entry: &Entry, issues: &mut Vec<String>) {
    let mut seen: Vec<String> = Vec::new();
    for (index, arg) in entry.arg.iter().enumerate() {
        let name = match (arg.role, &arg.name) {
            (Role::OutCap, None) => "out_cap".to_owned(),
            (
                Role::OutCap
                | Role::Scalar
                | Role::State
                | Role::InoutLen
                | Role::OutScalar
                | Role::OutPtr
                | Role::Str,
                Some(name),
            ) => name.clone(),
            _ => continue,
        };
        if RESERVED_ARG_NAMES.contains(&name.as_str()) {
            issues.push(format!(
                "{f}[{index}]: name {name:?} is reserved ({}) (D-11)",
                RESERVED_ARG_NAMES.join(", ")
            ));
        }
        if seen.contains(&name) {
            issues.push(format!("{f}[{index}]: duplicate name {name:?} (D-11)"));
        }
        seen.push(name);
    }
}

/// True when the configuration changes no setting at all.
fn sets_nothing(config: &Config) -> bool {
    config.defines.is_empty()
        && config.cflags.is_empty()
        && config.make_vars.is_empty()
        && config.cmake_options.is_empty()
        && config.goals.is_empty()
        && config.cmake_target.is_none()
}

/// True when every setting of `config` already appears in the recipe, so the
/// effective build equals the default build (edge BED-02/adjacency).
fn equals_recipe(config: &Config, args: &BuildArgs) -> bool {
    config.defines.is_empty()
        && config.cflags.is_empty()
        && config.make_vars.iter().all(|v| args.make_vars.contains(v))
        && config
            .cmake_options
            .iter()
            .all(|o| args.cmake_options.contains(o))
        && config.goals.iter().all(|g| args.goals.contains(g))
        && config
            .cmake_target
            .as_ref()
            .is_none_or(|t| args.cmake_target.as_ref() == Some(t))
}

/// `[a-z0-9-]+`.
fn is_config_name(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn config_issues(f: &str, target: &Target, issues: &mut Vec<String>) {
    let f = format!("{f}.config");
    let defaults = target.config.iter().filter(|c| c.name == "default").count();
    if defaults != 1 {
        issues.push(format!(
            "{f}: exactly one configuration named default is required, found {defaults}"
        ));
    }
    let variants = target.config.len() - defaults;
    if variants > 1 {
        issues.push(format!(
            "{f}: at most one variant besides default is allowed (CONTEXT D-05), found {variants}"
        ));
    }
    for (i, config) in target.config.iter().enumerate() {
        let g = format!("{f}.{}", config.name);
        if target.config[..i].iter().any(|c| c.name == config.name) {
            issues.push(format!("{f}: name {:?} is used twice", config.name));
        }
        if !is_config_name(&config.name) {
            issues.push(format!(
                "{g}.name: must be lower-case letters, digits and hyphens"
            ));
        }
        if config.name == "default" {
            if !sets_nothing(config) {
                issues.push(format!(
                    "{g}: must set nothing; the default configuration is the upstream defaults"
                ));
            }
        } else if sets_nothing(config) {
            issues.push(format!(
                "{g}: sets nothing; a variant must change at least one setting"
            ));
        } else if target
            .build_args
            .as_ref()
            .is_some_and(|args| equals_recipe(config, args))
        {
            issues.push(format!(
                "{g}: every setting already appears in build_args, so the build equals the default"
            ));
        }
        match target.build {
            BuildSystem::Make => {
                if !config.cmake_options.is_empty() {
                    issues.push(format!("{g}.cmake_options: not allowed on a make target"));
                }
                if config.cmake_target.is_some() {
                    issues.push(format!("{g}.cmake_target: not allowed on a make target"));
                }
            }
            BuildSystem::Cmake => {
                if !config.make_vars.is_empty() {
                    issues.push(format!("{g}.make_vars: not allowed on a cmake target"));
                }
                if !config.goals.is_empty() {
                    issues.push(format!("{g}.goals: not allowed on a cmake target"));
                }
            }
        }
        check_placeholders(issues, &format!("{g}.defines"), &config.defines);
        check_placeholders(issues, &format!("{g}.cflags"), &config.cflags);
        check_placeholders(issues, &format!("{g}.make_vars"), &config.make_vars);
        check_placeholders(issues, &format!("{g}.cmake_options"), &config.cmake_options);
        check_placeholders(issues, &format!("{g}.goals"), &config.goals);
        if let Some(cmake_target) = &config.cmake_target {
            check_placeholders(
                issues,
                &format!("{g}.cmake_target"),
                std::slice::from_ref(cmake_target),
            );
        }
    }
}
