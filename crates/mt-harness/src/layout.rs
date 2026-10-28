//! The layout of a state type, read from the pinned GCC's DWARF (CONTEXT D-09,
//! D-29; RESEARCH Q4, Pitfall 2).
//!
//! The compiler that built the captured objects already decided every size,
//! offset, bitfield position and enum width (GCC's `arm-none-eabi` enums are one
//! byte where clang's are four), so nothing here computes a layout: the text of
//! `llvm-dwarfdump --debug-info` for a probe object is parsed into a tree of
//! DIEs by indentation, and a variable's type is lowered into a [`CType`]. A tag
//! the lowering does not know is [`LayoutError::Unsupported`], never skipped.
//!
//! Typedefs are resolved to the type they name and keep their own name as the
//! type text; pointers keep only the text of what they point at, so the lowered
//! tree is finite. A type that contains itself by value is
//! [`LayoutError::Cycle`].

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use serde::{Deserialize, Serialize};

/// The deepest chain of type references followed before a cycle is assumed.
const MAX_DEPTH: usize = 64;

/// Why a dump cannot be turned into a layout.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LayoutError {
    /// The text ends before the DWARF does, or holds none.
    #[error("the DWARF dump is incomplete: {0}")]
    Truncated(String),
    /// The text is not the `llvm-dwarfdump` format this reader knows.
    #[error("the DWARF dump is malformed: {0}")]
    Malformed(String),
    /// A DWARF construct reached through a recorded type is not read; the
    /// payload names it.
    #[error("unsupported DWARF construct {0}")]
    Unsupported(String),
    /// A type contains itself without a pointer.
    #[error("type reference cycle: {0}")]
    Cycle(String),
    /// No variable with this name is in the dump.
    #[error("no variable {0} in the DWARF dump")]
    NoSuchVariable(String),
}

/// A bitfield: where its bits are, from the start of the enclosing struct.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BitField {
    /// `DW_AT_data_bit_offset`: the bit offset of the least significant bit.
    pub offset: u32,
    /// `DW_AT_bit_size`.
    pub size: u32,
}

/// One member of a struct or union.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Member {
    /// The name; an anonymous member is `_anon<offset>`.
    pub name: String,
    /// The member's type.
    pub ty: CType,
    /// The byte offset (for a bitfield, of the byte its first bit is in).
    pub offset: u32,
    /// The bitfield position, for a bitfield.
    pub bits: Option<BitField>,
}

/// What a type is made of.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Kind {
    /// An integer, bool or char of 1, 2, 4 or 8 bytes.
    Int {
        /// Whether values are sign-extended.
        signed: bool,
    },
    /// A `float` (4 bytes) or `double` (8 bytes).
    Float,
    /// An enumeration, as an integer of its own size.
    Enum {
        /// Whether values are sign-extended.
        signed: bool,
    },
    /// A pointer (4 bytes on this target).
    Pointer {
        /// True when it points at a function.
        function: bool,
    },
    /// An array.
    Array {
        /// The element type.
        elem: Box<CType>,
        /// The element count.
        count: u64,
    },
    /// A struct; members in declaration order.
    Struct {
        /// The members.
        members: Vec<Member>,
    },
    /// A union; every member starts at offset 0.
    Union {
        /// The members.
        members: Vec<Member>,
    },
}

/// A type as the compiler laid it out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CType {
    /// The C type text recorded with a value: a typedef's own name, `struct X`,
    /// a base type's name, `T *`, `T[N]`.
    pub text: String,
    /// `sizeof` in bytes.
    pub size: u32,
    /// The structure.
    pub kind: Kind,
}

/// One parsed DIE.
#[derive(Debug, Clone)]
struct Die {
    tag: String,
    attrs: BTreeMap<String, String>,
    children: Vec<u64>,
    parent: Option<u64>,
}

impl Die {
    fn attr(&self, name: &str) -> Option<&str> {
        self.attrs.get(name).map(String::as_str)
    }

    fn name(&self) -> Option<String> {
        self.attr("DW_AT_name").and_then(unquote)
    }

    /// An integer attribute written `0x..` or in decimal.
    fn number(&self, name: &str) -> Option<u64> {
        self.attr(name).and_then(parse_number)
    }

    /// A type reference: `0x<offset>` optionally followed by the type's name.
    fn reference(&self, name: &str) -> Option<u64> {
        self.number(name)
    }
}

/// `"text"` to `text`.
fn unquote(raw: &str) -> Option<String> {
    let inner = raw.strip_prefix('"')?;
    let end = inner.rfind('"')?;
    Some(inner[..end].to_owned())
}

/// The leading integer of `raw`: `0x1f`, `0x1f "name"` or `31`.
fn parse_number(raw: &str) -> Option<u64> {
    let first = raw.split_whitespace().next()?;
    if let Some(hex) = first.strip_prefix("0x") {
        u64::from_str_radix(hex, 16).ok()
    } else {
        first.parse::<u64>().ok()
    }
}

/// The parsed dump: every DIE by offset.
#[derive(Debug, Clone)]
pub struct LayoutModel {
    dies: BTreeMap<u64, Die>,
}

/// What a header line of the dump is.
enum Line<'a> {
    Unit,
    Null {
        depth: usize,
    },
    Die {
        offset: u64,
        depth: usize,
        tag: &'a str,
    },
}

/// `0x<offset>:<spaces><text>`: the DIE headers. One space after the colon is
/// depth 0, then two more per level.
fn header_line(line: &str) -> Option<Line<'_>> {
    let rest = line.strip_prefix("0x")?;
    let colon = rest.find(':')?;
    let offset = u64::from_str_radix(&rest[..colon], 16).ok()?;
    let after = &rest[colon + 1..];
    let text = after.trim_start_matches(' ');
    let spaces = after.len() - text.len();
    if spaces == 0 {
        return None;
    }
    let depth = (spaces - 1) / 2;
    if text.starts_with("Compile Unit:") {
        Some(Line::Unit)
    } else if text == "NULL" {
        Some(Line::Null { depth })
    } else if text.starts_with("DW_TAG_") {
        let tag = text.split_whitespace().next()?;
        Some(Line::Die { offset, depth, tag })
    } else {
        None
    }
}

/// `DW_AT_name<TAB>("x")` to the name and the text between the parentheses.
fn attribute_line(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with("DW_AT_") {
        return None;
    }
    let (name, value) = trimmed.split_once('\t')?;
    let value = value.trim();
    let inner = value.strip_prefix('(')?.strip_suffix(')')?;
    Some((name.trim(), inner))
}

/// Parse the text of `llvm-dwarfdump --debug-info` of one object.
///
/// # Errors
/// [`LayoutError::Truncated`] when the text holds no `.debug_info` section, no
/// compile unit, or a compile unit that is not closed by its final `NULL`;
/// [`LayoutError::Malformed`] for a DIE offset that repeats.
pub fn parse_dwarfdump(text: &str) -> Result<LayoutModel, LayoutError> {
    let mut in_info = false;
    let mut dies: BTreeMap<u64, Die> = BTreeMap::new();
    // The open DIEs, innermost last, with their depth.
    let mut stack: Vec<(usize, u64)> = Vec::new();
    let mut current: Option<u64> = None;
    let mut unit_closed = true;
    let mut unit_started = false;
    for line in text.lines() {
        if line.starts_with(".debug_info contents:") {
            in_info = true;
            continue;
        }
        if !in_info {
            continue;
        }
        match header_line(line) {
            Some(Line::Unit) => {
                if unit_started && !unit_closed {
                    return Err(LayoutError::Truncated(
                        "a compile unit is not closed before the next one".to_owned(),
                    ));
                }
                unit_started = true;
                unit_closed = false;
                stack.clear();
                current = None;
            }
            Some(Line::Null { depth }) => {
                if depth == 1 {
                    unit_closed = true;
                }
                current = None;
            }
            Some(Line::Die { offset, depth, tag }) => {
                if !unit_started {
                    return Err(LayoutError::Malformed(
                        "a DIE before any compile unit".to_owned(),
                    ));
                }
                unit_closed = false;
                while stack.last().is_some_and(|(open, _)| *open >= depth) {
                    stack.pop();
                }
                let parent = stack.last().map(|(_, offset)| *offset);
                if dies.contains_key(&offset) {
                    return Err(LayoutError::Malformed(format!(
                        "the DIE offset 0x{offset:x} appears twice"
                    )));
                }
                if let Some(parent) = parent.and_then(|p| dies.get_mut(&p)) {
                    parent.children.push(offset);
                }
                dies.insert(
                    offset,
                    Die {
                        tag: tag.to_owned(),
                        attrs: BTreeMap::new(),
                        children: Vec::new(),
                        parent,
                    },
                );
                stack.push((depth, offset));
                current = Some(offset);
            }
            None => {
                if let (Some((name, value)), Some(open)) =
                    (attribute_line(line), current.and_then(|o| dies.get_mut(&o)))
                {
                    open.attrs.insert(name.to_owned(), value.to_owned());
                }
            }
        }
    }
    if !in_info {
        return Err(LayoutError::Truncated(
            "no .debug_info section in the text".to_owned(),
        ));
    }
    if !unit_started {
        return Err(LayoutError::Truncated("no compile unit".to_owned()));
    }
    if !unit_closed {
        return Err(LayoutError::Truncated(
            "the last compile unit does not end with its closing NULL".to_owned(),
        ));
    }
    Ok(LayoutModel { dies })
}

fn tag_is_qualifier(tag: &str) -> bool {
    matches!(
        tag,
        "DW_TAG_const_type" | "DW_TAG_volatile_type" | "DW_TAG_restrict_type"
    )
}

impl LayoutModel {
    fn die(&self, offset: u64) -> Result<&Die, LayoutError> {
        self.dies.get(&offset).ok_or_else(|| {
            LayoutError::Malformed(format!("a reference to the missing DIE 0x{offset:x}"))
        })
    }

    /// The name and type reference of a variable DIE, following a
    /// `DW_AT_specification` for a definition of a declared variable.
    fn variable_identity(&self, die: &Die) -> (Option<String>, Option<u64>) {
        let specification = die
            .reference("DW_AT_specification")
            .and_then(|offset| self.dies.get(&offset));
        let name = die.name().or_else(|| specification.and_then(Die::name));
        let ty = die
            .reference("DW_AT_type")
            .or_else(|| specification.and_then(|s| s.reference("DW_AT_type")));
        (name, ty)
    }

    /// The type of the variable called `name` declared at file scope.
    ///
    /// # Errors
    /// [`LayoutError::NoSuchVariable`] when there is none, and the errors of
    /// lowering the type ([`LayoutError::Unsupported`], [`LayoutError::Cycle`],
    /// [`LayoutError::Malformed`]).
    pub fn type_of_variable(&self, name: &str) -> Result<CType, LayoutError> {
        let ty = self.variable_type_offset(name)?;
        self.lower(ty, &mut Vec::new())
    }

    /// The type reference of the file-scope variable called `name`.
    fn variable_type_offset(&self, name: &str) -> Result<u64, LayoutError> {
        for die in self.dies.values() {
            if die.tag != "DW_TAG_variable" || !self.is_file_scope(die) {
                continue;
            }
            let (found, ty) = self.variable_identity(die);
            if found.as_deref() == Some(name) {
                return ty
                    .ok_or_else(|| LayoutError::Malformed(format!("variable {name} has no type")));
            }
        }
        Err(LayoutError::NoSuchVariable(name.to_owned()))
    }

    /// The types that the pointers reachable from the variable `name` point at,
    /// keyed by the pointee's text (`cJSON`, `struct cJSON`, `char`, `const char`)
    /// as it appears in the pointer's own text with the trailing ` *` removed. The
    /// heap walk (CONTEXT D-09) reads a pointer's pointee from this table, because
    /// a [`CType`] keeps only the text of what a pointer points at. A pointee that
    /// cannot be lowered (a function, an unsupported construct) has no entry: a
    /// pointer to it is recorded as a classified pointer with no object.
    ///
    /// # Errors
    /// [`LayoutError::NoSuchVariable`] and the errors of finding the variable's type.
    pub fn pointee_types(&self, name: &str) -> Result<BTreeMap<String, CType>, LayoutError> {
        let mut found = BTreeMap::new();
        let mut seen = BTreeSet::new();
        let mut todo = vec![self.variable_type_offset(name)?];
        while let Some(offset) = todo.pop() {
            if !seen.insert(offset) {
                continue;
            }
            let die = self.die(offset)?;
            let tag = die.tag.as_str();
            match tag {
                "DW_TAG_typedef" | "DW_TAG_array_type" => todo.extend(die.reference("DW_AT_type")),
                tag if tag_is_qualifier(tag) => todo.extend(die.reference("DW_AT_type")),
                "DW_TAG_pointer_type" => {
                    let Some(target) = die.reference("DW_AT_type") else {
                        continue;
                    };
                    if let Ok((text, false)) = self.pointer_text(target, 0)
                        && let Some(pointee) = text.strip_suffix(" *")
                        && let Ok(lowered) = self.lower(target, &mut Vec::new())
                    {
                        found.entry(pointee.to_owned()).or_insert(lowered);
                    }
                    todo.push(target);
                }
                "DW_TAG_structure_type" | "DW_TAG_union_type" => {
                    for child in &die.children {
                        todo.extend(self.die(*child)?.reference("DW_AT_type"));
                    }
                }
                _ => {}
            }
        }
        Ok(found)
    }

    fn is_file_scope(&self, die: &Die) -> bool {
        die.parent
            .and_then(|parent| self.dies.get(&parent))
            .is_some_and(|parent| parent.tag == "DW_TAG_compile_unit")
    }

    fn is_function_scope(&self, die: &Die) -> bool {
        let mut parent = die.parent;
        while let Some(offset) = parent {
            let Some(up) = self.dies.get(&offset) else {
                return false;
            };
            if up.tag == "DW_TAG_subprogram" {
                return true;
            }
            parent = up.parent;
        }
        false
    }

    /// The type of the writable global `symbol` of the object this dump is of
    /// (CONTEXT D-10). A file-scope variable joins by name, preferring its
    /// definition; a function-local static, whose symbol is `name.N`, joins the
    /// variable `name` inside a function only when exactly one function holds
    /// one. `Ok(None)` when no unique variable matches.
    ///
    /// # Errors
    /// The errors of lowering the type.
    pub fn type_of_global(&self, symbol: &str) -> Result<Option<CType>, LayoutError> {
        let local = symbol
            .rsplit_once('.')
            .filter(|(_, number)| !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit()));
        let mut candidates: Vec<(bool, u64)> = Vec::new();
        for die in self.dies.values() {
            if die.tag != "DW_TAG_variable" {
                continue;
            }
            let (name, ty) = self.variable_identity(die);
            let (Some(name), Some(ty)) = (name, ty) else {
                continue;
            };
            let wanted = match local {
                Some((base, _)) => self.is_function_scope(die) && name == base,
                None => self.is_file_scope(die) && name == symbol,
            };
            if wanted {
                candidates.push((die.attr("DW_AT_location").is_some(), ty));
            }
        }
        // A definition (one with a location) beats a declaration.
        if candidates.iter().any(|(defined, _)| *defined) {
            candidates.retain(|(defined, _)| *defined);
        }
        let mut found: Option<CType> = None;
        for (_, ty) in &candidates {
            let lowered = self.lower(*ty, &mut Vec::new())?;
            match &found {
                None => found = Some(lowered),
                Some(other) if *other == lowered => {}
                Some(_) => return Ok(None),
            }
        }
        if local.is_some() && candidates.len() != 1 {
            return Ok(None);
        }
        Ok(found)
    }

    fn lower(&self, offset: u64, stack: &mut Vec<u64>) -> Result<CType, LayoutError> {
        if stack.contains(&offset) || stack.len() >= MAX_DEPTH {
            return Err(LayoutError::Cycle(format!("at the type 0x{offset:x}")));
        }
        stack.push(offset);
        let result = self.lower_inner(offset, stack);
        stack.pop();
        result
    }

    fn lower_inner(&self, offset: u64, stack: &mut Vec<u64>) -> Result<CType, LayoutError> {
        let die = self.die(offset)?;
        match die.tag.as_str() {
            "DW_TAG_base_type" => Self::lower_base(die),
            "DW_TAG_enumeration_type" => self.lower_enum(die, stack),
            "DW_TAG_structure_type" | "DW_TAG_union_type" => self.lower_record(die, stack),
            "DW_TAG_array_type" => self.lower_array(die, stack),
            "DW_TAG_typedef" => {
                let name = die.name().ok_or_else(|| {
                    LayoutError::Malformed(format!("the typedef at 0x{offset:x} has no name"))
                })?;
                let target = die
                    .reference("DW_AT_type")
                    .ok_or_else(|| LayoutError::Unsupported(format!("typedef {name} of void")))?;
                let inner = self.lower(target, stack)?;
                Ok(CType {
                    text: name,
                    ..inner
                })
            }
            tag if tag_is_qualifier(tag) => {
                let target = die
                    .reference("DW_AT_type")
                    .ok_or_else(|| LayoutError::Unsupported(format!("{tag} of void as a value")))?;
                let inner = self.lower(target, stack)?;
                let qualifier = match tag {
                    "DW_TAG_const_type" => "const",
                    "DW_TAG_volatile_type" => "volatile",
                    _ => "restrict",
                };
                let text = if matches!(inner.kind, Kind::Pointer { .. }) {
                    format!("{} {qualifier}", inner.text)
                } else {
                    format!("{qualifier} {}", inner.text)
                };
                Ok(CType { text, ..inner })
            }
            "DW_TAG_pointer_type" => {
                let size = u32::try_from(die.number("DW_AT_byte_size").unwrap_or(4))
                    .map_err(|_| LayoutError::Malformed("pointer size".to_owned()))?;
                if size != 4 {
                    return Err(LayoutError::Unsupported(format!(
                        "a {size}-byte pointer on a 32-bit target"
                    )));
                }
                let (text, function) = match die.reference("DW_AT_type") {
                    None => ("void *".to_owned(), false),
                    Some(target) => self.pointer_text(target, 0)?,
                };
                Ok(CType {
                    text,
                    size,
                    kind: Kind::Pointer { function },
                })
            }
            other => Err(LayoutError::Unsupported(other.to_owned())),
        }
    }

    /// The text of the pointer to the type at `target`, and whether it points at
    /// a function. Only names are read, so a pointer to itself is fine.
    fn pointer_text(&self, target: u64, depth: usize) -> Result<(String, bool), LayoutError> {
        if depth >= MAX_DEPTH {
            return Err(LayoutError::Cycle(format!("pointer chain at 0x{target:x}")));
        }
        let die = self.die(target)?;
        let named = |keyword: &str| {
            let name = die.name().unwrap_or_else(|| "<anonymous>".to_owned());
            format!("{keyword} {name} *")
        };
        Ok(match die.tag.as_str() {
            "DW_TAG_base_type" | "DW_TAG_typedef" => {
                let name = die.name().ok_or_else(|| {
                    LayoutError::Malformed(format!("the type at 0x{target:x} has no name"))
                })?;
                (format!("{name} *"), false)
            }
            "DW_TAG_structure_type" => (named("struct"), false),
            "DW_TAG_union_type" => (named("union"), false),
            "DW_TAG_enumeration_type" => (named("enum"), false),
            tag if tag_is_qualifier(tag) => {
                let qualifier = match tag {
                    "DW_TAG_const_type" => "const",
                    "DW_TAG_volatile_type" => "volatile",
                    _ => "restrict",
                };
                match die.reference("DW_AT_type") {
                    None => (format!("{qualifier} void *"), false),
                    Some(inner) => {
                        let (text, function) = self.pointer_text(inner, depth + 1)?;
                        let text = match text.strip_suffix(" *") {
                            Some(base) if tag != "DW_TAG_restrict_type" => {
                                format!("{qualifier} {base} *")
                            }
                            _ => text,
                        };
                        (text, function)
                    }
                }
            }
            "DW_TAG_pointer_type" => match die.reference("DW_AT_type") {
                None => ("void * *".to_owned(), false),
                Some(inner) => {
                    let (text, function) = self.pointer_text(inner, depth + 1)?;
                    (format!("{text} *"), function)
                }
            },
            "DW_TAG_subroutine_type" => {
                let returns = match die.reference("DW_AT_type") {
                    None => "void".to_owned(),
                    Some(inner) => {
                        let (text, _) = self.pointer_text(inner, depth + 1)?;
                        text.strip_suffix(" *").unwrap_or(&text).to_owned()
                    }
                };
                (format!("{returns} (*)(...)"), true)
            }
            other => return Err(LayoutError::Unsupported(other.to_owned())),
        })
    }

    fn lower_base(die: &Die) -> Result<CType, LayoutError> {
        let name = die
            .name()
            .ok_or_else(|| LayoutError::Malformed("a base type has no name".to_owned()))?;
        let size = die
            .number("DW_AT_byte_size")
            .and_then(|size| u32::try_from(size).ok())
            .ok_or_else(|| LayoutError::Malformed(format!("base type {name} has no size")))?;
        let encoding = die.attr("DW_AT_encoding").unwrap_or_default();
        let kind = match encoding {
            "DW_ATE_boolean" | "DW_ATE_unsigned" | "DW_ATE_unsigned_char" | "DW_ATE_UTF" => {
                Kind::Int { signed: false }
            }
            "DW_ATE_signed" | "DW_ATE_signed_char" => Kind::Int { signed: true },
            "DW_ATE_float" => Kind::Float,
            other => {
                return Err(LayoutError::Unsupported(format!(
                    "base type {name} with encoding {other}"
                )));
            }
        };
        let fits = match kind {
            Kind::Float => matches!(size, 4 | 8),
            _ => matches!(size, 1 | 2 | 4 | 8),
        };
        if !fits {
            return Err(LayoutError::Unsupported(format!(
                "base type {name} of {size} bytes"
            )));
        }
        Ok(CType {
            text: name,
            size,
            kind,
        })
    }

    fn lower_enum(&self, die: &Die, stack: &mut Vec<u64>) -> Result<CType, LayoutError> {
        let size = die
            .number("DW_AT_byte_size")
            .and_then(|size| u32::try_from(size).ok())
            .filter(|size| matches!(size, 1 | 2 | 4 | 8))
            .ok_or_else(|| {
                LayoutError::Malformed("an enumeration has no usable size".to_owned())
            })?;
        let signed = match die.reference("DW_AT_type") {
            Some(underlying) => {
                matches!(
                    self.lower(underlying, stack)?.kind,
                    Kind::Int { signed: true }
                )
            }
            None => die.attr("DW_AT_encoding").is_some_and(|encoding| {
                encoding.contains("signed") && !encoding.contains("unsigned")
            }),
        };
        let name = die.name().unwrap_or_else(|| "<anonymous>".to_owned());
        Ok(CType {
            text: format!("enum {name}"),
            size,
            kind: Kind::Enum { signed },
        })
    }

    fn lower_record(&self, die: &Die, stack: &mut Vec<u64>) -> Result<CType, LayoutError> {
        let is_union = die.tag == "DW_TAG_union_type";
        let keyword = if is_union { "union" } else { "struct" };
        let name = die.name().unwrap_or_else(|| "<anonymous>".to_owned());
        let text = format!("{keyword} {name}");
        let Some(size) = die
            .number("DW_AT_byte_size")
            .and_then(|size| u32::try_from(size).ok())
        else {
            return Err(LayoutError::Unsupported(format!("incomplete type {text}")));
        };
        let mut members: Vec<Member> = Vec::new();
        for child in &die.children {
            let member = self.die(*child)?;
            if member.tag != "DW_TAG_member" {
                return Err(LayoutError::Unsupported(member.tag.clone()));
            }
            let target = member
                .reference("DW_AT_type")
                .ok_or_else(|| LayoutError::Malformed(format!("a member of {text} has no type")))?;
            let ty = self.lower(target, stack)?;
            let bit_size = member.number("DW_AT_bit_size");
            let (offset, bits) = if let Some(bit_size) = bit_size {
                let bit_offset = member
                    .number("DW_AT_data_bit_offset")
                    .and_then(|bits| u32::try_from(bits).ok())
                    .ok_or_else(|| {
                        LayoutError::Unsupported(
                            "a bitfield without DW_AT_data_bit_offset".to_owned(),
                        )
                    })?;
                let bit_size = u32::try_from(bit_size)
                    .map_err(|_| LayoutError::Malformed("bit size".to_owned()))?;
                if u64::from(bit_size) > u64::from(ty.size) * 8
                    || u64::from(bit_offset) + u64::from(bit_size) > u64::from(size) * 8
                {
                    return Err(LayoutError::Malformed(format!(
                        "a bitfield of {text} lies outside the type"
                    )));
                }
                (
                    bit_offset / 8,
                    Some(BitField {
                        offset: bit_offset,
                        size: bit_size,
                    }),
                )
            } else {
                let offset = match member.attr("DW_AT_data_member_location") {
                    Some(raw) => parse_number(raw)
                        .and_then(|o| u32::try_from(o).ok())
                        .ok_or_else(|| {
                            LayoutError::Unsupported(
                                "DW_AT_data_member_location as an expression".to_owned(),
                            )
                        })?,
                    None if is_union => 0,
                    None => {
                        return Err(LayoutError::Malformed(format!(
                            "a member of {text} has no offset"
                        )));
                    }
                };
                if u64::from(offset) + u64::from(ty.size) > u64::from(size) {
                    return Err(LayoutError::Malformed(format!(
                        "a member of {text} lies outside the type"
                    )));
                }
                (offset, None)
            };
            let mut member_name = member.name().unwrap_or_else(|| format!("_anon{offset}"));
            let mut suffix = 1u32;
            while members.iter().any(|m| m.name == member_name) {
                member_name = format!("_anon{offset}_{suffix}");
                suffix += 1;
            }
            members.push(Member {
                name: member_name,
                ty,
                offset,
                bits,
            });
        }
        let kind = if is_union {
            Kind::Union { members }
        } else {
            Kind::Struct { members }
        };
        Ok(CType { text, size, kind })
    }

    fn lower_array(&self, die: &Die, stack: &mut Vec<u64>) -> Result<CType, LayoutError> {
        let element = die
            .reference("DW_AT_type")
            .ok_or_else(|| LayoutError::Malformed("an array has no element type".to_owned()))?;
        let base = self.lower(element, stack)?;
        let mut counts: Vec<u64> = Vec::new();
        for child in &die.children {
            let range = self.die(*child)?;
            if range.tag != "DW_TAG_subrange_type" {
                return Err(LayoutError::Unsupported(range.tag.clone()));
            }
            if range
                .number("DW_AT_lower_bound")
                .is_some_and(|lower| lower != 0)
            {
                return Err(LayoutError::Unsupported(
                    "an array with a lower bound above zero".to_owned(),
                ));
            }
            let count = if let Some(count) = range.number("DW_AT_count") {
                count
            } else if let Some(upper) = range.attr("DW_AT_upper_bound") {
                let upper = parse_number(upper).ok_or_else(|| {
                    LayoutError::Unsupported("DW_AT_upper_bound as an expression".to_owned())
                })?;
                if upper == u64::from(u32::MAX) || upper == u64::MAX {
                    0
                } else {
                    upper + 1
                }
            } else {
                // A flexible array member has no bound and no storage.
                0
            };
            counts.push(count);
        }
        if counts.is_empty() {
            return Err(LayoutError::Malformed(
                "an array has no dimension".to_owned(),
            ));
        }
        // Build from the innermost dimension out; each level's text is the base
        // text with the dimensions from its own level inwards.
        let mut current = base.clone();
        for (index, count) in counts.iter().enumerate().rev() {
            let tail: String = counts[index..].iter().fold(String::new(), |mut text, c| {
                let _ = write!(text, "[{c}]");
                text
            });
            let total = u64::from(current.size)
                .checked_mul(*count)
                .and_then(|total| u32::try_from(total).ok())
                .ok_or_else(|| LayoutError::Unsupported("an array larger than 4 GiB".to_owned()))?;
            current = CType {
                text: format!("{}{tail}", base.text),
                size: total,
                kind: Kind::Array {
                    elem: Box::new(current),
                    count: *count,
                },
            };
        }
        Ok(current)
    }
}

/// One line per type: `<text> size=<n>` then every member in declaration order,
/// nested members after their parent, as `<path>@<offset>:<size>` with
/// `:<bit offset>:<bit size>` appended for a bitfield. Offsets are from the start
/// of the whole type; an array of structs lists its first element's members under
/// `<path>[]`. The text is what `mt vectors layouts` prints, so two compilers'
/// layouts can be compared line for line.
#[must_use]
pub fn layout_summary(ty: &CType) -> String {
    let mut text = format!("{} size={}", ty.text, ty.size);
    summarize_members(ty, "", 0, &mut text);
    text
}

fn summarize_members(ty: &CType, prefix: &str, base: u32, out: &mut String) {
    let mut ty = ty;
    let mut prefix = prefix.to_owned();
    while let Kind::Array { elem, .. } = &ty.kind {
        ty = elem;
        prefix.push_str("[]");
    }
    if let Kind::Struct { members } | Kind::Union { members } = &ty.kind {
        let dot = if prefix.is_empty() { "" } else { "." };
        for member in members {
            let path = format!("{prefix}{dot}{}", member.name);
            let offset = base + member.offset;
            let _ = write!(out, " {path}@{offset}:{}", member.ty.size);
            if let Some(bits) = &member.bits {
                let _ = write!(out, ":{}:{}", base * 8 + bits.offset, bits.size);
            }
            summarize_members(&member.ty, &path, offset, out);
        }
    }
}
