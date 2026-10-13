//! The replay report (CONTEXT D-22).
//!
//! A report says, object by object, whether a clean-copy replay of the captured
//! compile commands rebuilt the same bytes. Everything in it is deterministic:
//! maps are `BTreeMap`, there is a `schema_version` and no timestamp, so the
//! same report serialises to the same bytes every time. The verdict words are
//! factual statements about bytes; no report makes a claim about a product or
//! a module.

use std::collections::BTreeMap;

use mt_toolchain::runner::RunRecord;
use serde::{Deserialize, Serialize};

use crate::entries::EntryResult;

/// Version of the serialised [`ReplayReport`].
pub const REPORT_SCHEMA_VERSION: u32 = 1;

/// The one normalisation that exists (CONTEXT D-09), written into every report
/// whether or not it was applied.
pub const NORMALISATION_RULE: &str = "objcopy --strip-debug --remove-section=.comment on both objects, only when every differing section is .comment, .debug_*, .rel.debug_* or .rela.debug_*";

/// The verdict for one target configuration.
///
/// Precedence when several apply: `not_proven`, `build_failed`, `differs`,
/// `identical_after_normalisation`, `identical`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// Every object is byte-equal and every compile exited zero.
    Identical,
    /// Every object is equal, some only after the fixed normalisation.
    IdenticalAfterNormalisation,
    /// At least one object differs or is missing on one side.
    Differs,
    /// A compile timed out or left an incomplete output stream.
    NotProven,
    /// A compile exited non-zero, or there was nothing to compile.
    BuildFailed,
}

/// The status of one object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectStatus {
    /// Byte-equal with no normalisation.
    Identical,
    /// Equal after the fixed normalisation of both objects.
    IdenticalAfterNormalisation,
    /// Present on both sides with different bytes.
    Differs,
    /// An original object the replay did not produce.
    MissingInReplay,
    /// A replayed object that has no original.
    MissingInOriginal,
}

/// The comparison of one object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectComparison {
    /// sha256 of the original object, when there is one.
    pub original: Option<String>,
    /// sha256 of the replayed object, when there is one.
    pub replay: Option<String>,
    /// The outcome.
    pub status: ObjectStatus,
    /// sha256 of the normalised original, when normalisation was applied.
    pub normalised_original: Option<String>,
    /// sha256 of the normalised replay, when normalisation was applied.
    pub normalised_replay: Option<String>,
    /// Sorted names of the sections that differ.
    pub differing_sections: Vec<String>,
    /// Free text: the reader error for an unparseable object, and similar.
    pub detail: Option<String>,
}

/// One subprocess of a replay, with its full run record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Step {
    /// `compile <file>`, `normalise original <key>` or `normalise replay <key>`.
    pub name: String,
    /// The run record of the step.
    pub run: RunRecord,
}

/// The report of one replay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayReport {
    /// [`REPORT_SCHEMA_VERSION`].
    pub schema_version: u32,
    /// Always `replay`.
    pub kind: String,
    /// Test-bed target id.
    pub target: String,
    /// Configuration name.
    pub config: String,
    /// The verdict.
    pub verdict: Verdict,
    /// [`NORMALISATION_RULE`], present in every report.
    pub normalisation_rule: String,
    /// True when any object was compared after normalisation.
    pub normalisation_applied: bool,
    /// Objects keyed by run-root-relative path, in sorted order.
    pub objects: BTreeMap<String, ObjectComparison>,
    /// Every subprocess, in the order it ran.
    pub steps: Vec<Step>,
    /// Why the verdict is what it is, where a word is not enough.
    pub detail: Option<String>,
}

/// The report of one fetch (CONTEXT D-17, D-22).
///
/// It names what was pinned and what was found, and carries the full run record
/// of every git, curl, tar and verification step. `reused` is the one field that
/// differs between a first fetch and a later one; everything else is a function
/// of the pin and the run records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FetchReport {
    /// [`REPORT_SCHEMA_VERSION`].
    pub schema_version: u32,
    /// Always `fetch`.
    pub kind: String,
    /// Test-bed target id.
    pub target: String,
    /// `git` or `archive`.
    pub source_kind: String,
    /// The pinned commit (the tag commit, for provenance, on an archive source).
    pub commit: String,
    /// The tree digest that was verified.
    pub tree_sha256: String,
    /// The archive digest that was verified, on an archive source.
    pub archive_sha256: Option<String>,
    /// True when the tree came from the cache and was verified again.
    pub reused: bool,
    /// Every subprocess, in the order it ran.
    pub steps: Vec<Step>,
}

impl FetchReport {
    /// Pretty JSON plus a trailing newline. Holds no timestamp.
    ///
    /// # Errors
    /// [`serde_json::Error`] if serialisation fails.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        let mut text = serde_json::to_string_pretty(self)?;
        text.push('\n');
        Ok(text)
    }
}

/// The verdict for one captured target configuration (CONTEXT D-22).
///
/// `captured` is the only word that lets replay start. A timeout is
/// `not_proven`, never a pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureVerdict {
    /// Every build step and every gate passed.
    Captured,
    /// The build ran but a gate refused its result; `causes` says which.
    CaptureFailed,
    /// A build step exited non-zero.
    BuildFailed,
    /// A step timed out or left an incomplete output stream, or a gate could
    /// not be evaluated.
    NotProven,
}

/// The source a capture was built from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRef {
    /// The pinned commit.
    pub commit: String,
    /// The verified tree digest.
    pub tree_sha256: String,
}

/// What the compile-commands file looked like before and after canonicalising.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompileCommandsInfo {
    /// sha256 of the exporter's file (Bear's or CMake's), exactly as written.
    pub raw_sha256: Option<String>,
    /// sha256 of the canonical file.
    pub canonical_sha256: Option<String>,
    /// Entries the exporter listed.
    pub entries_exported: usize,
    /// Entries whose object exists after the build.
    pub entries_built: usize,
    /// The `file` of each exported entry that built nothing, in export order.
    pub dropped_unbuilt: Vec<String>,
}

/// The silent-wrong-build gates (RESEARCH Pitfalls 2, 7 and the `__TIMESTAMP__`
/// row). A gate that was not evaluated reads as failed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gates {
    /// Every compiler in the capture is the pinned `arm-none-eabi-gcc`.
    pub compiler_allow_listed: bool,
    /// Every object is ELF32 ARM Cortex-M4 with hard float.
    pub arch_ok: bool,
    /// Objects the build created that no compile command declares.
    pub created_undeclared: Vec<String>,
    /// Captured sources that use `__TIMESTAMP__`.
    pub timestamp_macro: Vec<String>,
}

/// One object of the capture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapturedObject {
    /// sha256 of the object file.
    pub sha256: String,
    /// The ELF header and the four `readelf -A` tags pass.
    pub arch_ok: bool,
    /// `Tag_ABI_optimization_goals`: recorded, never enforced (RESEARCH Open
    /// Question 7).
    pub opt_goal: Option<String>,
    /// sha256 over the executable sections, `None` when the object cannot be read.
    pub code_digest: Option<String>,
}

/// The report of one capture (CONTEXT D-22).
///
/// Everything in it is deterministic: maps are `BTreeMap`, there is a
/// `schema_version` and no timestamp.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureReport {
    /// [`REPORT_SCHEMA_VERSION`].
    pub schema_version: u32,
    /// Always `capture`.
    pub kind: String,
    /// Test-bed target id.
    pub target: String,
    /// Configuration name.
    pub config: String,
    /// The verdict.
    pub verdict: CaptureVerdict,
    /// Why it is not `captured`: machine tokens such as `empty_capture`,
    /// `created_undeclared`, `arch_gate`, `timestamp_macro`.
    pub causes: Vec<String>,
    /// One human-readable line per cause, with the files or messages involved.
    pub cause_details: Vec<String>,
    /// The toolchain image digest, stamped by the caller.
    pub image: Option<String>,
    /// The verified source.
    pub source: SourceRef,
    /// The pinned versions of the tools the capture used.
    pub tools: BTreeMap<String, String>,
    /// sha256 of each committed input file under `testbed/targets/<id>/`.
    pub inputs: BTreeMap<String, String>,
    /// sha256 of the generated CMake toolchain file, for CMake targets.
    pub toolchain_file_sha256: Option<String>,
    /// The compile-commands counts and digests.
    pub compile_commands: CompileCommandsInfo,
    /// The gates.
    pub gates: Gates,
    /// Objects keyed by run-root-relative path.
    pub objects: BTreeMap<String, CapturedObject>,
    /// sha256 of the configure-time files of a CMake build, keyed by
    /// run-root-relative path.
    pub configure_files: BTreeMap<String, String>,
    /// The entry-function check of each manifest entry, keyed by function name
    /// (CONTEXT D-23).
    #[serde(default)]
    pub entries: BTreeMap<String, EntryResult>,
    /// Every subprocess, in the order it ran.
    pub steps: Vec<Step>,
}

impl CaptureReport {
    /// An empty report whose verdict is `not_proven` until the capture sets it.
    #[must_use]
    pub fn new(target: &str, config: &str) -> Self {
        Self {
            schema_version: REPORT_SCHEMA_VERSION,
            kind: "capture".to_owned(),
            target: target.to_owned(),
            config: config.to_owned(),
            verdict: CaptureVerdict::NotProven,
            causes: Vec::new(),
            cause_details: Vec::new(),
            image: None,
            source: SourceRef {
                commit: String::new(),
                tree_sha256: String::new(),
            },
            tools: BTreeMap::new(),
            inputs: BTreeMap::new(),
            toolchain_file_sha256: None,
            compile_commands: CompileCommandsInfo::default(),
            gates: Gates::default(),
            objects: BTreeMap::new(),
            configure_files: BTreeMap::new(),
            entries: BTreeMap::new(),
            steps: Vec::new(),
        }
    }

    /// Pretty JSON plus a trailing newline. Holds no timestamp.
    ///
    /// # Errors
    /// [`serde_json::Error`] if serialisation fails.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        let mut text = serde_json::to_string_pretty(self)?;
        text.push('\n');
        Ok(text)
    }
}

impl ReplayReport {
    /// Pretty JSON plus a trailing newline. Holds no timestamp.
    ///
    /// # Errors
    /// [`serde_json::Error`] if serialisation fails.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        let mut text = serde_json::to_string_pretty(self)?;
        text.push('\n');
        Ok(text)
    }
}
