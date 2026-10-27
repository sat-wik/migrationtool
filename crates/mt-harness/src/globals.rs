//! The writable globals of the captured objects (CONTEXT D-06, D-10).
//!
//! Every symbol defined in a `.data` or `.bss` section of a captured object is a
//! writable global of the target. [`writable_globals`] lists them from one
//! object with the `object` crate; [`locate_globals`] joins them with the linked
//! driver's symbol table to get their final addresses and initial bytes. The
//! driver snapshots each range before and after the call. A global whose bytes
//! differ from the image's initial bytes needs a typed layout to be recorded;
//! until a type is available the executor stops with `untyped_global_changed`
//! and never stores raw bytes (D-06).

use std::collections::{BTreeMap, BTreeSet};

use object::read::elf::ElfFile32;
use object::{
    Endianness, Object, ObjectSection, ObjectSymbol, SectionKind, SymbolKind, SymbolSection,
};

use crate::HarnessError;

/// Which kind of section a global lives in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GlobalKind {
    /// A `.data` section: initialised bytes in the image.
    Data,
    /// A `.bss` section: zero at start.
    Bss,
}

/// A defined data symbol of one captured object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalSymbol {
    /// The object's path relative to the objects directory.
    pub object: String,
    /// The symbol name.
    pub name: String,
    /// `.data` or `.bss`.
    pub kind: GlobalKind,
    /// Size in bytes (never zero).
    pub size: u64,
    /// True for a file-local (`static`) symbol.
    pub local: bool,
    /// For a local symbol, the source file named by the `STT_FILE` symbol before
    /// it: the key that joins it to the linked image.
    pub file: Option<String>,
}

/// One range the driver snapshots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DumpRange {
    /// `<object path>:<symbol>`, the key under `lib_state.globals`.
    pub key: String,
    /// The final address in the linked driver.
    pub address: u32,
    /// Size in bytes.
    pub size: u32,
    /// The image's initial bytes (zero for `.bss`).
    pub initial: Vec<u8>,
}

fn input(what: &str, detail: impl Into<String>) -> HarnessError {
    HarnessError::Input {
        what: what.to_owned(),
        detail: detail.into(),
    }
}

fn section_kind(name: &str) -> Option<GlobalKind> {
    if name.starts_with(".data") {
        Some(GlobalKind::Data)
    } else if name.starts_with(".bss") {
        Some(GlobalKind::Bss)
    } else {
        None
    }
}

/// A defined data symbol of an ELF file.
struct Defined {
    name: String,
    kind: GlobalKind,
    size: u64,
    address: u64,
    local: bool,
    file: Option<String>,
}

/// Every defined, non-empty data symbol in a `.data*` or `.bss*` section (or
/// common), with the source file that precedes it in the symbol table.
fn defined_data(file: &ElfFile32<'_, Endianness>) -> Result<Vec<Defined>, HarnessError> {
    let mut found = Vec::new();
    let mut current_file: Option<String> = None;
    for symbol in file.symbols() {
        if symbol.kind() == SymbolKind::File {
            current_file = symbol.name().ok().map(str::to_owned);
            continue;
        }
        if symbol.kind() != SymbolKind::Data || symbol.size() == 0 {
            continue;
        }
        let kind = match symbol.section() {
            SymbolSection::Common => GlobalKind::Bss,
            SymbolSection::Section(index) => {
                let section = file
                    .section_by_index(index)
                    .map_err(|e| input("object", e.to_string()))?;
                let name = section.name().map_err(|e| input("object", e.to_string()))?;
                match section_kind(name) {
                    Some(kind) => kind,
                    None => continue,
                }
            }
            _ => continue,
        };
        let name = symbol
            .name()
            .map_err(|e| input("object", e.to_string()))?
            .to_owned();
        if name.is_empty() {
            continue;
        }
        let local = symbol.is_local();
        found.push(Defined {
            name,
            kind,
            size: symbol.size(),
            address: symbol.address(),
            local,
            file: local.then(|| current_file.clone()).flatten(),
        });
    }
    Ok(found)
}

fn parse(bytes: &[u8]) -> Result<ElfFile32<'_, Endianness>, HarnessError> {
    ElfFile32::parse(bytes).map_err(|e| input("ELF file", e.to_string()))
}

/// The writable globals of one captured object, sorted by name.
///
/// # Errors
/// [`HarnessError::Input`] when `bytes` is not a 32-bit ELF file.
pub fn writable_globals(object: &str, bytes: &[u8]) -> Result<Vec<GlobalSymbol>, HarnessError> {
    let file = parse(bytes).map_err(|e| input(object, e.to_string()))?;
    let mut symbols: Vec<GlobalSymbol> = defined_data(&file)?
        .into_iter()
        .map(|defined| GlobalSymbol {
            object: object.to_owned(),
            name: defined.name,
            kind: defined.kind,
            size: defined.size,
            local: defined.local,
            file: defined.file,
        })
        .collect();
    symbols.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(symbols)
}

/// The initial bytes of `[address, address + size)` in the linked image: zeros
/// in a `NOBITS` section, the section's data otherwise.
fn initial_bytes(
    file: &ElfFile32<'_, Endianness>,
    key: &str,
    address: u64,
    size: u64,
) -> Result<Vec<u8>, HarnessError> {
    for section in file.sections() {
        let start = section.address();
        let end = start.saturating_add(section.size());
        if address < start || address.saturating_add(size) > end {
            continue;
        }
        let length = usize::try_from(size).map_err(|_| input(key, "size does not fit"))?;
        if section.kind() == SectionKind::UninitializedData {
            return Ok(vec![0; length]);
        }
        return match section
            .data_range(address, size)
            .map_err(|e| input(key, e.to_string()))?
        {
            Some(data) => Ok(data.to_vec()),
            None => Err(input(key, "the image holds no bytes for it")),
        };
    }
    Err(input(key, "no section of the image holds it"))
}

/// Join the globals of the captured objects with the linked driver's symbol
/// table and read their initial bytes. Only a global present in the linked image
/// is returned. A global symbol is joined by name; a local one through the
/// source file named before it, and a local name that more than one object (or
/// one object and the image twice) could mean is an error.
///
/// # Errors
/// [`HarnessError::Input`] for an ELF file that cannot be read, an ambiguous join
/// or a symbol the image cannot give bytes for.
pub fn locate_globals(
    symbols: &[GlobalSymbol],
    elf: &[u8],
) -> Result<Vec<DumpRange>, HarnessError> {
    let image = parse(elf).map_err(|e| input("driver.elf", e.to_string()))?;
    let mut by_name: BTreeMap<String, Vec<(u64, u64)>> = BTreeMap::new();
    let mut by_file: BTreeMap<(String, String), Vec<(u64, u64)>> = BTreeMap::new();
    for defined in defined_data(&image)? {
        match (defined.local, defined.file) {
            (true, Some(file)) => by_file
                .entry((file, defined.name))
                .or_default()
                .push((defined.address, defined.size)),
            (true, None) => {}
            (false, _) => by_name
                .entry(defined.name)
                .or_default()
                .push((defined.address, defined.size)),
        }
    }
    // Which objects claim each local (file, name): two claims cannot be told
    // apart in the image.
    let mut claims: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
    for symbol in symbols {
        if let (true, Some(file)) = (symbol.local, &symbol.file) {
            claims
                .entry((file.clone(), symbol.name.clone()))
                .or_default()
                .insert(symbol.object.clone());
        }
    }
    let mut ranges = Vec::new();
    for symbol in symbols {
        let key = format!("{}:{}", symbol.object, symbol.name);
        let found = if symbol.local {
            let Some(file) = &symbol.file else {
                return Err(input(
                    &key,
                    "a local symbol with no source file cannot be joined to the image",
                ));
            };
            let pair = (file.clone(), symbol.name.clone());
            let objects = claims.get(&pair).map_or(0, BTreeSet::len);
            let present = by_file.get(&pair).map_or(0, Vec::len);
            if objects > 1 || present > 1 {
                return Err(input(
                    &key,
                    format!(
                        "ambiguous join: the local {} of {file} is claimed by {objects} object(s) and found {present} time(s) in the image",
                        symbol.name
                    ),
                ));
            }
            by_file.get(&pair).and_then(|list| list.first().copied())
        } else {
            let list = by_name.get(&symbol.name);
            if list.is_some_and(|list| list.len() > 1) {
                return Err(input(
                    &key,
                    "ambiguous join: defined more than once in the image",
                ));
            }
            list.and_then(|list| list.first().copied())
        };
        let Some((address, size)) = found else {
            continue;
        };
        let narrow = |value: u64, what: &str| {
            u32::try_from(value).map_err(|_| input(&key, format!("{what} does not fit 32 bits")))
        };
        ranges.push(DumpRange {
            initial: initial_bytes(&image, &key, address, size)?,
            address: narrow(address, "the address")?,
            size: narrow(size, "the size")?,
            key,
        });
    }
    ranges.sort_by(|a, b| a.key.cmp(&b.key));
    Ok(ranges)
}
