//! ELF object inspection for the capture and replay gates (BUILD-01, BUILD-02).
//!
//! Objects produced by untrusted upstream code are parsed here with the
//! read-only, bounds-checked `object` crate; a malformed file becomes an
//! [`ElfError`], never a panic. Nothing in this module spawns a process.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use object::elf::SHF_EXECINSTR;
use object::read::elf::{ElfFile, ElfFile32, ElfFile64, FileHeader};
use object::{Endianness, FileKind, Object, ObjectSection, SectionFlags};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::treehash::sha256_hex;

/// Why an object could not be inspected, or why it failed the architecture gate.
#[derive(Debug, thiserror::Error)]
pub enum ElfError {
    /// Too short for an ELF header, or the magic number is wrong.
    #[error("not an ELF file")]
    NotElf,
    /// The file is not 32-bit.
    #[error("not ELF32 (class byte {class})")]
    NotElf32 {
        /// The `EI_CLASS` byte.
        class: u8,
    },
    /// The file is not little-endian.
    #[error("not little-endian (data byte {data})")]
    NotLittleEndian {
        /// The `EI_DATA` byte.
        data: u8,
    },
    /// The file is not a relocatable object.
    #[error("not a relocatable object (e_type {e_type})")]
    NotRelocatable {
        /// The `e_type` field.
        e_type: u16,
    },
    /// The machine is not ARM.
    #[error("not an ARM object (e_machine {machine})")]
    NotArm {
        /// The `e_machine` field.
        machine: u16,
    },
    /// The `object` crate refused the file.
    #[error("cannot read the object: {0}")]
    Parse(String),
}

/// One section of an object: its name, size and the sha256 of its data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SectionDigest {
    /// Section name.
    pub name: String,
    /// Section size in bytes.
    pub size: u64,
    /// Lowercase hex sha256 of the section data (empty data for `NOBITS`).
    pub sha256: String,
}

/// The ARM build attributes the effective-flags gate reads from `readelf -A`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArmAttributes {
    /// `Tag_CPU_arch`, for example `v7E-M`.
    pub cpu_arch: Option<String>,
    /// `Tag_THUMB_ISA_use`, for example `Thumb-2`.
    pub thumb_isa_use: Option<String>,
    /// `Tag_FP_arch`, for example `VFPv4-D16`.
    pub fp_arch: Option<String>,
    /// `Tag_ABI_VFP_args`, for example `VFP registers`.
    pub abi_vfp_args: Option<String>,
    /// `Tag_ABI_optimization_goals`: recorded, never enforced.
    pub optimization_goals: Option<String>,
}

impl ArmAttributes {
    /// True when the four gate tags read Cortex-M4 with hard float: `v7E-M`,
    /// `Thumb-2`, `VFPv4-D16` and `VFP registers`. The optimisation goal is
    /// deliberately not part of the gate.
    #[must_use]
    pub fn is_cortex_m4_hard_float(&self) -> bool {
        self.cpu_arch.as_deref() == Some("v7E-M")
            && self.thumb_isa_use.as_deref() == Some("Thumb-2")
            && self.fp_arch.as_deref() == Some("VFPv4-D16")
            && self.abi_vfp_args.as_deref() == Some("VFP registers")
    }
}

/// A section as read from the object, borrowing its data.
struct RawSection<'data> {
    name: String,
    size: u64,
    data: &'data [u8],
    executable: bool,
}

fn parse_error(error: &object::Error) -> ElfError {
    ElfError::Parse(error.to_string())
}

fn collect<'data, Elf>(
    file: &ElfFile<'data, Elf, &'data [u8]>,
) -> Result<Vec<RawSection<'data>>, ElfError>
where
    Elf: FileHeader<Endian = Endianness>,
{
    let mut raw = Vec::new();
    for section in file.sections() {
        let name = section.name().map_err(|e| parse_error(&e))?.to_owned();
        let data = section.data().map_err(|e| parse_error(&e))?;
        let executable = match section.flags() {
            SectionFlags::Elf { sh_flags, .. } => sh_flags.0 & SHF_EXECINSTR.0 != 0,
            _ => false,
        };
        raw.push(RawSection {
            name,
            size: section.size(),
            data,
            executable,
        });
    }
    Ok(raw)
}

/// Read every section of an ELF32 or ELF64 object.
fn read_sections(bytes: &[u8]) -> Result<Vec<RawSection<'_>>, ElfError> {
    match FileKind::parse(bytes).map_err(|e| parse_error(&e))? {
        FileKind::Elf32 => {
            collect(&ElfFile32::<Endianness>::parse(bytes).map_err(|e| parse_error(&e))?)
        }
        FileKind::Elf64 => {
            collect(&ElfFile64::<Endianness>::parse(bytes).map_err(|e| parse_error(&e))?)
        }
        other => Err(ElfError::Parse(format!("not an ELF file ({other:?})"))),
    }
}

/// Every section of the object, in section-table order, with the sha256 of its
/// data. `NOBITS` sections such as `.bss` have empty data and carry their size.
///
/// # Errors
/// [`ElfError::Parse`] if the object cannot be read.
pub fn sections(bytes: &[u8]) -> Result<Vec<SectionDigest>, ElfError> {
    Ok(read_sections(bytes)?
        .into_iter()
        .map(|section| SectionDigest {
            sha256: sha256_hex(section.data),
            name: section.name,
            size: section.size,
        })
        .collect())
}

/// (name, occurrence index) mapped to (size, sha256).
type KeyedSections = BTreeMap<(String, usize), (u64, String)>;

/// Sections keyed by name and occurrence index, so a name that repeats compares
/// its first occurrence with the first, its second with the second.
fn keyed(bytes: &[u8]) -> Result<KeyedSections, ElfError> {
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    let mut keyed = BTreeMap::new();
    for section in sections(bytes)? {
        let occurrence = seen.entry(section.name.clone()).or_insert(0);
        keyed.insert((section.name, *occurrence), (section.size, section.sha256));
        *occurrence += 1;
    }
    Ok(keyed)
}

/// Sorted names of the sections present on one side only or with different
/// size or data. A name that repeats appears once.
///
/// # Errors
/// [`ElfError::Parse`] if either object cannot be read.
pub fn differing_sections(a: &[u8], b: &[u8]) -> Result<Vec<String>, ElfError> {
    let left = keyed(a)?;
    let right = keyed(b)?;
    let mut names: BTreeSet<String> = BTreeSet::new();
    for key in left.keys().chain(right.keys()) {
        if left.get(key) != right.get(key) {
            names.insert(key.0.clone());
        }
    }
    Ok(names.into_iter().collect())
}

/// True only for a non-empty set whose every name is `.comment`, `.debug_*`,
/// `.rel.debug_*` or `.rela.debug_*` (CONTEXT D-09).
#[must_use]
pub fn normalisable(names: &[String]) -> bool {
    !names.is_empty()
        && names.iter().all(|name| {
            name == ".comment"
                || name.starts_with(".debug_")
                || name.starts_with(".rel.debug_")
                || name.starts_with(".rela.debug_")
        })
}

/// Require ELF32, little-endian, ARM, relocatable. A host x86-64 object fails
/// at the class byte; this is the machine check behind the architecture gate
/// (RESEARCH Pitfall 2).
///
/// # Errors
/// The [`ElfError`] naming the first property that fails.
pub fn check_elf_header(bytes: &[u8]) -> Result<(), ElfError> {
    const EI_CLASS: usize = 4;
    const EI_DATA: usize = 5;
    const ELFCLASS32: u8 = 1;
    const ELFDATA2LSB: u8 = 1;
    const ET_REL: u16 = 1;
    const EM_ARM: u16 = 40;
    if bytes.len() < 20 || !bytes.starts_with(b"\x7fELF") {
        return Err(ElfError::NotElf);
    }
    if bytes[EI_CLASS] != ELFCLASS32 {
        return Err(ElfError::NotElf32 {
            class: bytes[EI_CLASS],
        });
    }
    if bytes[EI_DATA] != ELFDATA2LSB {
        return Err(ElfError::NotLittleEndian {
            data: bytes[EI_DATA],
        });
    }
    let e_type = u16::from_le_bytes([bytes[16], bytes[17]]);
    if e_type != ET_REL {
        return Err(ElfError::NotRelocatable { e_type });
    }
    let machine = u16::from_le_bytes([bytes[18], bytes[19]]);
    if machine != EM_ARM {
        return Err(ElfError::NotArm { machine });
    }
    Ok(())
}

/// One `Tag_...: value` line of `readelf -A`, compiled once. If the constant
/// pattern ever failed to compile the gate would read nothing and fail closed.
static TAG_LINE: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(?m)^[ \t]*(Tag_[A-Za-z0-9_]+):[ \t]*(.*?)[ \t]*$").ok());

/// Read the build attributes out of `readelf -A` text. A tag that is absent
/// reads as `None`, which fails the gate, and the first occurrence of a tag wins.
#[must_use]
pub fn parse_readelf_a(text: &str) -> ArmAttributes {
    let mut tags: BTreeMap<&str, &str> = BTreeMap::new();
    if let Some(pattern) = TAG_LINE.as_ref() {
        for captures in pattern.captures_iter(text) {
            if let (Some(tag), Some(value)) = (captures.get(1), captures.get(2))
                && !value.as_str().is_empty()
            {
                tags.entry(tag.as_str()).or_insert(value.as_str());
            }
        }
    }
    let tag = |name: &str| tags.get(name).map(|value| (*value).to_owned());
    ArmAttributes {
        cpu_arch: tag("Tag_CPU_arch"),
        thumb_isa_use: tag("Tag_THUMB_ISA_use"),
        fp_arch: tag("Tag_FP_arch"),
        abi_vfp_args: tag("Tag_ABI_VFP_args"),
        optimization_goals: tag("Tag_ABI_optimization_goals"),
    }
}

/// sha256 over the name and data of the executable (`SHF_EXECINSTR`) sections,
/// in section order. It ignores `.comment`, debug and data sections, so it says
/// whether the code is the same where the whole object is not.
///
/// # Errors
/// [`ElfError::Parse`] if the object cannot be read.
pub fn code_digest(bytes: &[u8]) -> Result<String, ElfError> {
    let mut material: Vec<u8> = Vec::new();
    for section in read_sections(bytes)?.iter().filter(|s| s.executable) {
        material.extend_from_slice(section.name.as_bytes());
        material.push(0);
        material.extend_from_slice(
            &u64::try_from(section.data.len())
                .unwrap_or(u64::MAX)
                .to_le_bytes(),
        );
        material.extend_from_slice(section.data);
    }
    Ok(sha256_hex(&material))
}
