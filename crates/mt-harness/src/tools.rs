//! The pinned tools the executor runs.
//!
//! `arm-none-eabi-ar` and `arm-none-eabi-nm` are derived from the directory of
//! the pinned compiler and `llvm-dwarfdump` from the directory of the pinned
//! clang, so `container/pins.toml` is not extended (CONTEXT D-29).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use mt_toolchain::pins::{Pins, ToolPinStatus};

use crate::HarnessError;

/// The absolute paths of the tools the executor runs.
#[derive(Debug, Clone)]
pub struct ExecTools {
    /// The pinned `arm-none-eabi-gcc`.
    pub arm_gcc: PathBuf,
    /// `arm-none-eabi-ar`, next to the compiler.
    pub ar: PathBuf,
    /// `arm-none-eabi-nm`, next to the compiler.
    pub nm: PathBuf,
    /// The pinned `qemu-arm`.
    pub qemu_arm: PathBuf,
    /// `llvm-dwarfdump`, next to the pinned clang.
    pub llvm_dwarfdump: PathBuf,
    /// The pinned (`expect`) version of each tool, keyed by its pins key.
    pub versions: BTreeMap<String, String>,
}

fn pinned_tool(pins: &Pins, key: &str) -> Result<PathBuf, HarnessError> {
    match pins.tool.get(key) {
        Some(spec) if spec.status == ToolPinStatus::Installed && !spec.bin.is_empty() => {
            Ok(PathBuf::from(&spec.bin))
        }
        _ => Err(HarnessError::ToolNotPinned {
            key: key.to_owned(),
        }),
    }
}

impl ExecTools {
    /// The tools of the pins. Fails when one is not an installed pin, or when a
    /// tool's file is missing; the error then names every missing tool.
    ///
    /// # Errors
    /// [`HarnessError::ToolNotPinned`] or [`HarnessError::ToolsMissing`].
    pub fn from_pins(pins: &Pins) -> Result<Self, HarnessError> {
        let arm_gcc = pinned_tool(pins, "arm_gcc")?;
        let qemu_arm = pinned_tool(pins, "qemu_arm")?;
        let clang = pinned_tool(pins, "clang")?;
        let mut versions = BTreeMap::new();
        for key in ["arm_gcc", "qemu_arm", "clang"] {
            if let Some(spec) = pins.tool.get(key)
                && !spec.expect.is_empty()
            {
                versions.insert(key.to_owned(), spec.expect.clone());
            }
        }
        let tools = Self {
            ar: sibling(&arm_gcc, "arm-none-eabi-ar"),
            nm: sibling(&arm_gcc, "arm-none-eabi-nm"),
            llvm_dwarfdump: sibling(&clang, "llvm-dwarfdump"),
            arm_gcc,
            qemu_arm,
            versions,
        };
        tools.check_present()?;
        Ok(tools)
    }

    /// Tools at known places, for tests: the Arm tools in `arm_bin_dir`.
    /// Presence is not checked.
    #[must_use]
    pub fn at(arm_bin_dir: &Path, qemu_arm: &Path, llvm_dwarfdump: &Path) -> Self {
        Self {
            arm_gcc: arm_bin_dir.join("arm-none-eabi-gcc"),
            ar: arm_bin_dir.join("arm-none-eabi-ar"),
            nm: arm_bin_dir.join("arm-none-eabi-nm"),
            qemu_arm: qemu_arm.to_path_buf(),
            llvm_dwarfdump: llvm_dwarfdump.to_path_buf(),
            versions: BTreeMap::new(),
        }
    }

    /// Every tool, as `(name, path)` in a fixed order.
    #[must_use]
    pub fn entries(&self) -> [(&'static str, &Path); 5] {
        [
            ("qemu_arm", &self.qemu_arm),
            ("arm_gcc", &self.arm_gcc),
            ("arm_ar", &self.ar),
            ("arm_nm", &self.nm),
            ("llvm_dwarfdump", &self.llvm_dwarfdump),
        ]
    }

    /// Fails when a tool's file does not exist.
    ///
    /// # Errors
    /// [`HarnessError::ToolsMissing`] naming every missing tool.
    pub fn check_present(&self) -> Result<(), HarnessError> {
        let missing: Vec<(String, PathBuf)> = self
            .entries()
            .into_iter()
            .filter(|(_, path)| !path.is_file())
            .map(|(name, path)| (name.to_owned(), path.to_path_buf()))
            .collect();
        if missing.is_empty() {
            Ok(())
        } else {
            Err(HarnessError::ToolsMissing { tools: missing })
        }
    }
}

/// `name` in the directory of `tool`.
fn sibling(tool: &Path, name: &str) -> PathBuf {
    tool.parent()
        .map_or_else(|| PathBuf::from(name), |dir| dir.join(name))
}
