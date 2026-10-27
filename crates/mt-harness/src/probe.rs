//! The probes that give a state type, or a global, its layout (CONTEXT D-09,
//! D-10, D-29).
//!
//! A probe is compiled by the pinned `arm-none-eabi-gcc` with the flags of the
//! unit that defines the entry, plus `-g -gdwarf-5 -O0`, into a scratch object
//! that is never linked. `llvm-dwarfdump --debug-info` dumps it through the
//! runner and [`crate::layout`] reads the text. Optimisation does not change a
//! type's DWARF, so `-O0` only keeps every variable.
//!
//! - [`layout_for_entry`] compiles the entry's include lines and one used global
//!   per state type, and gives each state's [`CType`].
//! - [`layout_for_globals`] recompiles the defining unit of each writable global
//!   with that unit's own flags and types the global from that unit's DWARF.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use mt_capture::cc::CcEntry;
use mt_capture::manifest::BASELINE_CFLAGS;
use mt_toolchain::runner::{self, RunnerConfig};
use serde::{Deserialize, Serialize};

use crate::layout::{CType, LayoutError, parse_dwarfdump};
use crate::link::{failure, gcc_request, healthy};
use crate::tools::ExecTools;
use crate::{HarnessError, sha256_hex};

/// The largest state object the driver snapshots, in bytes. A larger type is not
/// expressible (`runtime.c` holds a fixed snapshot buffer).
pub const MAX_STATE_BYTES: u32 = 65_536;

/// The prefix of a probe variable's name.
pub const PROBE_PREFIX: &str = "mt_probe_";

fn io(action: &'static str, path: &Path, source: std::io::Error) -> HarnessError {
    HarnessError::Io {
        action,
        path: path.to_path_buf(),
        source,
    }
}

/// The source of a layout probe: the include lines, then one used global per
/// state, `<type> mt_probe_<name> __attribute__((used));`. The names are C
/// identifiers and the types are plain type names; both are checked where the
/// manifest is validated and again by the adapter before they get here.
#[must_use]
pub fn render_probe(include_lines: &[String], states: &[(String, String)]) -> String {
    let mut source = String::new();
    for line in include_lines {
        source.push_str(line);
        source.push('\n');
    }
    for (name, type_text) in states {
        source.push_str(&format!(
            "{type_text} {PROBE_PREFIX}{name} __attribute__((used));\n"
        ));
    }
    source
}

/// The flags a probe is compiled with: the baseline, the driver's flags, the
/// driver's `-std=gnu11`, then `-g -gdwarf-5 -O0 -c` (the later `-O0` wins over
/// the baseline's `-Os`).
#[must_use]
pub fn probe_flags(driver_flags: &[String]) -> Vec<String> {
    let mut flags: Vec<String> = BASELINE_CFLAGS
        .iter()
        .map(|flag| (*flag).to_owned())
        .collect();
    flags.extend(driver_flags.iter().cloned());
    flags.extend(
        ["-std=gnu11", "-g", "-gdwarf-5", "-O0", "-c"]
            .iter()
            .map(|flag| (*flag).to_owned()),
    );
    flags
}

/// `llvm-dwarfdump --debug-info <object>` through the runner, with the default
/// output cap. An incomplete or truncated dump is an error.
fn dump_object(
    tools: &ExecTools,
    cfg: &RunnerConfig,
    dir: &Path,
    object: &Path,
) -> Result<String, HarnessError> {
    let request = runner::RunRequest {
        tool_name: "llvm_dwarfdump".to_owned(),
        argv: vec![
            tools.llvm_dwarfdump.clone().into_os_string(),
            OsString::from("--debug-info"),
            object.as_os_str().to_owned(),
        ],
        cwd: dir.to_path_buf(),
        extra_env: BTreeMap::new(),
        tool_version: None,
    };
    let output = runner::run(cfg, &request)?;
    if !healthy(&output) {
        return Err(failure("llvm-dwarfdump of a layout probe", &output));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn layout_error(what: &str, source: LayoutError) -> HarnessError {
    HarnessError::Layout {
        what: what.to_owned(),
        source,
    }
}

/// What a state layout needs.
#[derive(Debug, Clone)]
pub struct LayoutRequest<'a> {
    /// The entry's build directory; the probe source, object and dump go here.
    pub dir: &'a Path,
    /// The defining unit's directory: the working directory of the compile.
    pub compile_cwd: &'a Path,
    /// The include lines the driver starts with.
    pub include_lines: &'a [String],
    /// The driver's flags ([`crate::adapter::driver_flags`]).
    pub flags: &'a [String],
    /// Each state's name and the type text of its pointee.
    pub states: &'a [(String, String)],
}

/// Compile the probe of an entry's states and give each state's layout.
///
/// # Errors
/// [`HarnessError::Build`] when the compile or the dump fails,
/// [`HarnessError::Layout`] when a type cannot be read (an unsupported DWARF
/// construct is named) or is larger than [`MAX_STATE_BYTES`].
pub fn layout_for_entry(
    tools: &ExecTools,
    cfg: &RunnerConfig,
    request: &LayoutRequest<'_>,
) -> Result<BTreeMap<String, CType>, HarnessError> {
    if request.states.is_empty() {
        return Ok(BTreeMap::new());
    }
    std::fs::create_dir_all(request.dir).map_err(|e| io("cannot create", request.dir, e))?;
    let source_path = request.dir.join("probe.c");
    let object_path = request.dir.join("probe.o");
    let dump_path = request.dir.join("probe.dwarf.txt");
    std::fs::write(
        &source_path,
        render_probe(request.include_lines, request.states),
    )
    .map_err(|e| io("cannot write", &source_path, e))?;
    if object_path.exists() {
        std::fs::remove_file(&object_path).map_err(|e| io("cannot remove", &object_path, e))?;
    }
    let mut argv: Vec<OsString> = vec![tools.arm_gcc.clone().into_os_string()];
    argv.extend(probe_flags(request.flags).iter().map(OsString::from));
    argv.push(source_path.as_os_str().to_owned());
    argv.push(OsString::from("-o"));
    argv.push(object_path.as_os_str().to_owned());
    let compiled = runner::run(cfg, &gcc_request(tools, argv, request.compile_cwd))?;
    if !healthy(&compiled) {
        return Err(failure(
            "arm-none-eabi-gcc compile of the layout probe",
            &compiled,
        ));
    }
    let text = dump_object(tools, cfg, request.dir, &object_path)?;
    std::fs::write(&dump_path, &text).map_err(|e| io("cannot write", &dump_path, e))?;
    let model = parse_dwarfdump(&text).map_err(|e| layout_error("the layout probe", e))?;
    let mut layouts = BTreeMap::new();
    for (name, type_text) in request.states {
        let variable = format!("{PROBE_PREFIX}{name}");
        let ty = model
            .type_of_variable(&variable)
            .map_err(|e| layout_error(&format!("state {name} ({type_text})"), e))?;
        if ty.size == 0 || ty.size > MAX_STATE_BYTES {
            return Err(layout_error(
                &format!("state {name} ({type_text})"),
                LayoutError::Unsupported(format!(
                    "a state of {} bytes (the limit is 1..={MAX_STATE_BYTES})",
                    ty.size
                )),
            ));
        }
        layouts.insert(name.clone(), ty);
    }
    Ok(layouts)
}

/// The type of one writable global, or why it has none (CONTEXT D-10).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum GlobalType {
    /// Typed from the DWARF of its defining unit.
    Typed {
        /// The type.
        ty: CType,
    },
    /// Not typed; a change in its bytes stops the run (`untyped_global_changed`).
    Untyped {
        /// Why.
        reason: String,
    },
}

/// One global to type: the key it is recorded under, its symbol name and the
/// compile command whose output is its object.
#[derive(Debug, Clone)]
pub struct GlobalTarget {
    /// `<object path>:<symbol>`.
    pub key: String,
    /// The symbol name in the object.
    pub symbol: String,
    /// The compile command of the defining unit.
    pub unit: CcEntry,
}

/// The arguments that recompile `unit` with debug information into `scratch`:
/// the unit's own arguments with `-o`, dependency-file options and `-O*`/`-g*`
/// options removed, then `-g -gdwarf-5 -O0`, `-c` if it was missing and
/// `-o <scratch>`. Every other option is kept as the unit had it, because the
/// layout must come from the flags the object was built with.
#[must_use]
pub fn recompile_arguments(compiler: &Path, arguments: &[String], scratch: &Path) -> Vec<OsString> {
    let mut argv: Vec<OsString> = vec![compiler.as_os_str().to_owned()];
    let mut remaining = arguments.iter().skip(1);
    let mut has_compile_only = false;
    while let Some(argument) = remaining.next() {
        let text = argument.as_str();
        if matches!(text, "-o" | "-MF" | "-MT" | "-MQ") {
            remaining.next();
        } else if text.starts_with("-o")
            || text.starts_with("-M")
            || text.starts_with("-O")
            || text.starts_with("-g")
        {
            // Joined operand, dependency flags, optimisation and debug options.
        } else {
            has_compile_only |= text == "-c";
            argv.push(OsString::from(argument));
        }
    }
    for flag in ["-g", "-gdwarf-5", "-O0"] {
        argv.push(OsString::from(flag));
    }
    if !has_compile_only {
        argv.push(OsString::from("-c"));
    }
    argv.push(OsString::from("-o"));
    argv.push(scratch.as_os_str().to_owned());
    argv
}

/// Type every global of `targets` from the DWARF of its defining unit, compiled
/// with that unit's own flags into a scratch object under `dir/globals/`. A
/// global that cannot be typed (no unique variable, an unsupported type, a unit
/// that does not recompile) is [`GlobalType::Untyped`] with the reason.
///
/// # Errors
/// [`HarnessError`] when a file cannot be written or a tool cannot run.
pub fn layout_for_globals(
    tools: &ExecTools,
    cfg: &RunnerConfig,
    dir: &Path,
    targets: &[GlobalTarget],
) -> Result<BTreeMap<String, GlobalType>, HarnessError> {
    let scratch_dir: PathBuf = dir.join("globals");
    let mut result = BTreeMap::new();
    let mut by_unit: BTreeMap<String, Vec<&GlobalTarget>> = BTreeMap::new();
    for target in targets {
        by_unit
            .entry(target.unit.output.clone())
            .or_default()
            .push(target);
    }
    if by_unit.is_empty() {
        return Ok(result);
    }
    std::fs::create_dir_all(&scratch_dir).map_err(|e| io("cannot create", &scratch_dir, e))?;
    for group in by_unit.values() {
        let Some(first) = group.first() else {
            continue;
        };
        let unit = &first.unit;
        let stem = &sha256_hex(unit.output.as_bytes())[..16];
        let object = scratch_dir.join(format!("{stem}.o"));
        if object.exists() {
            std::fs::remove_file(&object).map_err(|e| io("cannot remove", &object, e))?;
        }
        let argv = recompile_arguments(&tools.arm_gcc, &unit.arguments, &object);
        let compiled = runner::run(cfg, &gcc_request(tools, argv, Path::new(&unit.directory)))?;
        let model = if healthy(&compiled) {
            let text = dump_object(tools, cfg, &scratch_dir, &object)?;
            parse_dwarfdump(&text).map_err(|e| e.to_string())
        } else {
            Err("the defining unit does not recompile with debug information".to_owned())
        };
        for target in group {
            let typed = match &model {
                Err(reason) => GlobalType::Untyped {
                    reason: reason.clone(),
                },
                Ok(model) => match model.type_of_global(&target.symbol) {
                    Ok(Some(ty)) => GlobalType::Typed { ty },
                    Ok(None) => GlobalType::Untyped {
                        reason: "no unique variable of that name in the unit's DWARF".to_owned(),
                    },
                    Err(error) => GlobalType::Untyped {
                        reason: error.to_string(),
                    },
                },
            };
            result.insert(target.key.clone(), typed);
        }
    }
    Ok(result)
}
