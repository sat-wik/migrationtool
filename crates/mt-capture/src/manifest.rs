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
