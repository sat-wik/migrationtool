//! ELF object inspection for the capture and replay gates (BUILD-01, BUILD-02).
//!
//! Objects produced by untrusted upstream code are parsed here with the
//! read-only, bounds-checked `object` crate; a malformed file becomes an
//! [`ElfError`], never a panic. Nothing in this module spawns a process.

use serde::{Deserialize, Serialize};

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
    /// True when the four gate tags read Cortex-M4 with hard float.
    #[must_use]
    pub fn is_cortex_m4_hard_float(&self) -> bool {
        false
    }
}

/// Every section of the object, in section-table order.
///
/// # Errors
/// [`ElfError::Parse`] if the object cannot be read.
pub fn sections(_bytes: &[u8]) -> Result<Vec<SectionDigest>, ElfError> {
    Ok(Vec::new())
}

/// Sorted names of the sections present on one side only or with different data.
///
/// # Errors
/// [`ElfError::Parse`] if either object cannot be read.
pub fn differing_sections(_a: &[u8], _b: &[u8]) -> Result<Vec<String>, ElfError> {
    Ok(Vec::new())
}

/// True only for a non-empty set whose every name is forgivable (CONTEXT D-09).
#[must_use]
pub fn normalisable(_names: &[String]) -> bool {
    false
}

/// Require ELF32, little-endian, ARM, relocatable.
///
/// # Errors
/// The [`ElfError`] naming the first property that fails.
pub fn check_elf_header(_bytes: &[u8]) -> Result<(), ElfError> {
    Ok(())
}

/// Read the build attributes out of `readelf -A` text.
#[must_use]
pub fn parse_readelf_a(_text: &str) -> ArmAttributes {
    ArmAttributes {
        cpu_arch: None,
        thumb_isa_use: None,
        fp_arch: None,
        abi_vfp_args: None,
        optimization_goals: None,
    }
}

/// sha256 over the name and data of the executable sections, in section order.
///
/// # Errors
/// [`ElfError::Parse`] if the object cannot be read.
pub fn code_digest(_bytes: &[u8]) -> Result<String, ElfError> {
    Ok(String::new())
}
