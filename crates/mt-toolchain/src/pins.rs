//! The `pins.toml` schema, version 1.
//!
//! `pins.toml` is the single source of truth for every pinned version, digest,
//! checksum and path (CONTEXT D-14). Unknown keys are rejected everywhere so a
//! typo in a pin cannot silently disable a check. Every map is a `BTreeMap`, so
//! iteration order is the sorted key order.
//!
//! [`Pins::parse`] only checks shape. [`Pins::validate`] checks every value
//! (hex formats, digests, https URLs, placeholders, LLVM major rules) and
//! [`Pins::validate_complete`] additionally requires the full container tool
//! set.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use regex::RegexBuilder;
use serde::{Deserialize, Serialize};

/// The only schema version this crate understands.
pub const PINS_SCHEMA_VERSION: u32 = 1;

/// Tools the container must provide, each pinned as installed (CONTEXT D-11).
pub const REQUIRED_TOOLS: [&str; 10] = [
    "arm_gcc",
    "bear",
    "c2rust",
    "cargo_mutants",
    "clang",
    "klee",
    "llvm_config",
    "qemu_arm",
    "qemu_system_arm",
    "rustup",
];

/// Tools that are deliberately not installed and must be listed as such (CONTEXT D-10).
pub const NOT_INSTALLED_TOOLS: [&str; 2] = ["hayroll", "kani"];

/// Tools whose LLVM major must be checked, so each carries an LLVM rule (TOOL-02).
pub const LLVM_RULE_TOOLS: [&str; 4] = ["c2rust", "clang", "klee", "llvm_config"];

/// A parsed pins file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pins {
    /// Must equal [`PINS_SCHEMA_VERSION`].
    pub schema_version: u32,
    /// Settings shared by every tool run and by the image build.
    pub image: ImagePins,
    /// The single LLVM major every C-side tool must use.
    pub llvm: LlvmPins,
    /// The two Rust toolchains.
    pub rust: RustPins,
    /// Downloaded or built sources, keyed by a stable source key.
    #[serde(default)]
    pub source: BTreeMap<String, SourcePin>,
    /// Exact Debian package versions, keyed by package name (filled after pin discovery).
    #[serde(default)]
    pub apt: BTreeMap<String, String>,
    /// Pins for the CI host tools.
    pub ci: CiPins,
    /// One table per pinned tool, keyed by a stable tool key.
    pub tool: BTreeMap<String, ToolSpec>,
}

/// Image-wide settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImagePins {
    /// Image platform; only `linux/amd64` is supported (CONTEXT D-22).
    pub platform: String,
    /// Base image reference without the digest, such as `debian:bookworm-20261005-slim`.
    pub base: String,
    /// Digest of the base image index: `sha256:` plus 64 lowercase hex digits.
    pub base_digest: String,
    /// The `snapshot.debian.org` timestamp, `YYYYMMDDTHHMMSSZ`.
    pub snapshot_timestamp: String,
    /// Value of `SOURCE_DATE_EPOCH` for every child.
    pub source_date_epoch: String,
    /// Value of `PATH` for every child.
    pub tool_path: String,
    /// Absolute path of `dpkg-query`, used by the package check.
    pub dpkg_query: String,
}

/// Whether the LLVM pin is still provisional.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LlvmStatus {
    /// Revisited at the Phase 6 R1 verdict.
    Provisional,
    /// Confirmed by a recorded decision.
    Final,
}

/// The one LLVM major used by clang, KLEE, c2rust and (later) Hayroll.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LlvmPins {
    /// The pinned major; must be within 16 to 19.
    pub major: u32,
    /// Highest LLVM major any tool may report; must be 19.
    pub max_major: u32,
    /// Provisional or final.
    pub status: LlvmStatus,
    /// The PROJECT.md decision ID justifying the pin, such as `D-15`.
    pub decision: String,
}

/// The tool and bitcode Rust toolchains.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RustPins {
    /// Absolute `RUSTUP_HOME` inside the image.
    pub rustup_home: String,
    /// Absolute `CARGO_HOME` inside the image.
    pub cargo_home: String,
    /// The host triple of both toolchains.
    pub host: String,
    /// The toolchain that builds the tool and the `no_std` crates.
    pub tool: RustToolPins,
    /// The toolchain whose LLVM matches the C-side LLVM (for KLEE bitcode).
    pub bitcode: RustBitcodePins,
}

/// The tool toolchain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RustToolPins {
    /// Release, such as `1.99.0`.
    pub version: String,
    /// `commit-hash` from `rustc -vV`: 40 lowercase hex digits.
    pub commit: String,
    /// sha256 of the channel manifest.
    pub channel_manifest_sha256: String,
    /// Installed rustup components.
    pub components: Vec<String>,
    /// Installed compilation targets besides the host.
    pub targets: Vec<String>,
}

/// The bitcode toolchain (host target only).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RustBitcodePins {
    /// Release, such as `1.72.1`.
    pub version: String,
    /// `commit-hash` from `rustc -vV`: 40 lowercase hex digits.
    pub commit: String,
    /// The `LLVM version:` its `rustc -vV` reports, such as `16.0.5`.
    pub llvm: String,
    /// sha256 of the channel manifest.
    pub channel_manifest_sha256: String,
    /// The PROJECT.md decision ID justifying the pin.
    pub decision: String,
}

/// One pinned source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub enum SourcePin {
    /// A git checkout asserted by commit.
    Git {
        /// `https://` clone URL.
        url: String,
        /// Tag that is cloned.
        tag: String,
        /// The commit the tag must resolve to: 40 lowercase hex digits.
        commit: String,
    },
    /// A crates.io crate installed with `cargo install --locked`.
    Crate {
        /// Crate name.
        name: String,
        /// Exact version.
        version: String,
        /// sha256 of the `.crate` file.
        sha256: String,
    },
    /// A downloaded file verified by sha256.
    Archive {
        /// Release label.
        version: String,
        /// `https://` download URL.
        url: String,
        /// sha256 of the downloaded file.
        sha256: String,
    },
    /// A source fetched by another pinned source's build.
    Transitive {
        /// Key of the source that pulls this one in.
        parent: String,
        /// The commit the parent fetches: 40 lowercase hex digits.
        commit: String,
    },
}

/// Pins for tools that run on the CI host rather than in the image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CiPins {
    /// Version of `cargo-deny` installed in CI.
    pub cargo_deny_version: String,
    /// BuildKit image reference with an `@sha256:` digest.
    pub buildkit_image: String,
}

/// Whether a tool is installed in the image.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolPinStatus {
    /// The tool is installed and checked.
    #[default]
    Installed,
    /// The tool is deliberately absent; listed for the manifest only.
    NotInstalled,
}

/// Where the LLVM major of a tool comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LlvmSource {
    /// The first component of the extracted version.
    Version,
    /// A regex with named group `major` over the version command's output.
    Output,
    /// A separate command whose output a regex with group `major` is applied to.
    Probe,
}

/// How to obtain a tool's LLVM major.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LlvmRule {
    /// Where the major comes from.
    pub source: LlvmSource,
    /// Regex with named group `major` (sources `output` and `probe`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regex: Option<String>,
    /// Command to run (source `probe`); the first element is an absolute path.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub argv: Vec<String>,
}

/// How to find and identify one tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolSpec {
    /// Installed (default) or not installed.
    #[serde(default)]
    pub status: ToolPinStatus,
    /// Why the tool is not installed (required for, and only for, `not_installed`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Program to run: an absolute path.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub bin: String,
    /// Arguments that make the tool print its version.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub version_args: Vec<String>,
    /// Regex with a named group `version`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub version_regex: String,
    /// The pinned version the extracted `version` must equal.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub expect: String,
    /// Named environment variables passed to the runner for this tool (CONTEXT D-17).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    /// How to obtain the tool's LLVM major, if it links LLVM.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llvm: Option<LlvmRule>,
}

/// Why a pins file could not be used.
#[derive(Debug, thiserror::Error)]
pub enum PinsError {
    /// The file could not be read.
    #[error("cannot read pins file {}: {source}", path.display())]
    Read {
        /// The path that failed.
        path: PathBuf,
        /// The underlying error.
        #[source]
        source: std::io::Error,
    },
    /// The text is not valid TOML for this schema.
    #[error("invalid pins file: {0}")]
    Parse(#[from] toml::de::Error),
    /// The file declares a schema this crate does not support.
    #[error("unsupported pins schema_version {found}; expected {expected}")]
    SchemaVersion {
        /// Version found in the file.
        found: u32,
        /// Version this crate supports.
        expected: u32,
    },
    /// The pins parse but one or more values are not acceptable.
    #[error("pins validation failed with {} issue(s): {}", issues.len(), issues.join("; "))]
    Invalid {
        /// Every issue found, sorted, each naming the offending field.
        issues: Vec<String>,
    },
}

impl Pins {
    /// Parse pins from TOML text.
    ///
    /// # Errors
    /// [`PinsError::Parse`] on malformed TOML or unknown keys;
    /// [`PinsError::SchemaVersion`] when `schema_version` is not 1.
    pub fn parse(text: &str) -> Result<Self, PinsError> {
        let pins: Self = toml::from_str(text)?;
        if pins.schema_version != PINS_SCHEMA_VERSION {
            return Err(PinsError::SchemaVersion {
                found: pins.schema_version,
                expected: PINS_SCHEMA_VERSION,
            });
        }
        Ok(pins)
    }

    /// Read and parse a pins file.
    ///
    /// # Errors
    /// [`PinsError::Read`] if the file cannot be read, otherwise as [`Pins::parse`].
    pub fn load(path: &Path) -> Result<Self, PinsError> {
        let text = std::fs::read_to_string(path).map_err(|source| PinsError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse(&text)
    }

    /// Check every value: hex formats, digests, https URLs, placeholders, the
    /// LLVM major rules and per-tool consistency. Every issue is collected,
    /// sorted and reported at once, each naming the offending field.
    ///
    /// # Errors
    /// [`PinsError::Invalid`] listing every issue.
    pub fn validate(&self) -> Result<(), PinsError> {
        finish(self.collect_issues())
    }

    /// [`Pins::validate`] plus the completeness rules: every container tool is
    /// pinned as installed, Hayroll and Kani are listed as `not_installed`
    /// (CONTEXT D-10), the LLVM-linked tools carry an LLVM rule and `[apt]` is
    /// not empty.
    ///
    /// # Errors
    /// [`PinsError::Invalid`] listing every issue.
    pub fn validate_complete(&self) -> Result<(), PinsError> {
        let mut issues = self.collect_issues();
        for key in REQUIRED_TOOLS {
            match self.tool.get(key) {
                None => issues.push(format!(
                    "tool.{key}: missing; every container tool must be pinned"
                )),
                Some(spec) if spec.status == ToolPinStatus::NotInstalled => issues.push(format!(
                    "tool.{key}: must be installed, found status not_installed"
                )),
                Some(_) => {}
            }
        }
        for key in NOT_INSTALLED_TOOLS {
            match self.tool.get(key) {
                None => issues.push(format!(
                    "tool.{key}: missing; must be listed as not_installed (CONTEXT D-10)"
                )),
                Some(spec) if spec.status == ToolPinStatus::Installed => {
                    issues.push(format!("tool.{key}: must be not_installed (CONTEXT D-10)"))
                }
                Some(_) => {}
            }
        }
        for key in LLVM_RULE_TOOLS {
            if let Some(spec) = self.tool.get(key)
                && spec.status == ToolPinStatus::Installed
                && spec.llvm.is_none()
            {
                issues.push(format!(
                    "tool.{key}.llvm: missing; this tool must carry an LLVM rule (TOOL-02)"
                ));
            }
        }
        if self.apt.is_empty() {
            issues.push("apt: must not be empty".to_owned());
        }
        finish(issues)
    }

    fn collect_issues(&self) -> Vec<String> {
        let mut issues = Vec::new();
        self.image_issues(&mut issues);
        self.llvm_issues(&mut issues);
        self.rust_issues(&mut issues);
        self.source_issues(&mut issues);
        self.apt_and_ci_issues(&mut issues);
        for (key, spec) in &self.tool {
            tool_issues(key, spec, &mut issues);
        }
        self.llvm_token_issues(&mut issues);
        placeholder_issues(self, &mut issues);
        issues
    }

    fn image_issues(&self, issues: &mut Vec<String>) {
        let image = &self.image;
        if image.platform != "linux/amd64" {
            issues.push(format!(
                "image.platform: must be linux/amd64 (CONTEXT D-22), found {:?}",
                image.platform
            ));
        }
        match image.base_digest.strip_prefix("sha256:") {
            Some(hex) if is_lower_hex(hex, 64) => {}
            _ => issues.push(
                "image.base_digest: must be sha256: followed by 64 lowercase hex digits".to_owned(),
            ),
        }
        if !is_snapshot_timestamp(&image.snapshot_timestamp) {
            issues.push("image.snapshot_timestamp: must look like YYYYMMDDTHHMMSSZ".to_owned());
        }
        if image.source_date_epoch.is_empty()
            || !image.source_date_epoch.bytes().all(|b| b.is_ascii_digit())
        {
            issues.push("image.source_date_epoch: must be digits only".to_owned());
        }
        if !image.tool_path.split(':').all(|dir| dir.starts_with('/')) {
            issues.push("image.tool_path: every entry must be an absolute path".to_owned());
        }
        if !image.dpkg_query.starts_with('/') {
            issues.push("image.dpkg_query: must be an absolute path".to_owned());
        }
    }

    fn llvm_issues(&self, issues: &mut Vec<String>) {
        let llvm = &self.llvm;
        if !(SUPPORTED_LLVM_MAJORS).contains(&llvm.major) {
            issues.push(format!(
                "llvm.major: {} is outside the supported range 16 to 19 (TOOL-02)",
                llvm.major
            ));
        }
        if llvm.max_major != MAX_LLVM_MAJOR {
            issues.push(format!(
                "llvm.max_major: must be {MAX_LLVM_MAJOR}, found {}",
                llvm.max_major
            ));
        }
        if !is_decision_id(&llvm.decision) {
            issues.push("llvm.decision: must look like D-15".to_owned());
        }
    }

    fn rust_issues(&self, issues: &mut Vec<String>) {
        let rust = &self.rust;
        if !rust.rustup_home.starts_with('/') {
            issues.push("rust.rustup_home: must be an absolute path".to_owned());
        }
        if !rust.cargo_home.starts_with('/') {
            issues.push("rust.cargo_home: must be an absolute path".to_owned());
        }
        let tool = &rust.tool;
        if !is_dotted_numeric(&tool.version, 3) {
            issues.push("rust.tool.version: must look like 1.99.0".to_owned());
        }
        check_hex(issues, "rust.tool.commit", &tool.commit, 40);
        check_hex(
            issues,
            "rust.tool.channel_manifest_sha256",
            &tool.channel_manifest_sha256,
            64,
        );
        check_name_list(issues, "rust.tool.components", &tool.components);
        check_name_list(issues, "rust.tool.targets", &tool.targets);
        let bitcode = &rust.bitcode;
        if !is_dotted_numeric(&bitcode.version, 3) {
            issues.push("rust.bitcode.version: must look like 1.72.1".to_owned());
        }
        check_hex(issues, "rust.bitcode.commit", &bitcode.commit, 40);
        if !is_dotted_numeric(&bitcode.llvm, 3) {
            issues.push("rust.bitcode.llvm: must look like 16.0.5".to_owned());
        }
        check_hex(
            issues,
            "rust.bitcode.channel_manifest_sha256",
            &bitcode.channel_manifest_sha256,
            64,
        );
        if !is_decision_id(&bitcode.decision) {
            issues.push("rust.bitcode.decision: must look like D-15".to_owned());
        }
    }

    fn source_issues(&self, issues: &mut Vec<String>) {
        for (key, source) in &self.source {
            let f = format!("source.{key}");
            if key.is_empty()
                || !key
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
            {
                issues.push(format!(
                    "{f}: the key must be lower-case letters, digits and underscores"
                ));
            }
            match source {
                SourcePin::Git { url, commit, .. } => {
                    check_https(issues, &format!("{f}.url"), url);
                    check_hex(issues, &format!("{f}.commit"), commit, 40);
                }
                SourcePin::Crate { sha256, .. } => {
                    check_hex(issues, &format!("{f}.sha256"), sha256, 64);
                }
                SourcePin::Archive { url, sha256, .. } => {
                    check_https(issues, &format!("{f}.url"), url);
                    check_hex(issues, &format!("{f}.sha256"), sha256, 64);
                }
                SourcePin::Transitive { parent, commit } => {
                    if parent == key || !self.source.contains_key(parent) {
                        issues.push(format!(
                            "{f}.parent: must name another entry in [source], found {parent:?}"
                        ));
                    }
                    check_hex(issues, &format!("{f}.commit"), commit, 40);
                }
            }
        }
    }

    fn apt_and_ci_issues(&self, issues: &mut Vec<String>) {
        if self.apt.keys().any(String::is_empty) {
            issues.push("apt: a package name must not be empty".to_owned());
        }
        if !is_dotted_numeric(&self.ci.cargo_deny_version, 3) {
            issues.push("ci.cargo_deny_version: must look like 0.20.2".to_owned());
        }
        match self.ci.buildkit_image.split_once("@sha256:") {
            Some((name, hex)) if !name.is_empty() && is_lower_hex(hex, 64) => {}
            _ => issues.push(
                "ci.buildkit_image: must be an image name followed by @sha256: and 64 lowercase hex digits"
                    .to_owned(),
            ),
        }
    }

    /// Every `llvm-N`, `clang-N` and `clang-cppN` token in a path, a tool binary,
    /// an LLVM probe command or an apt package name must carry the pinned major.
    fn llvm_token_issues(&self, issues: &mut Vec<String>) {
        let major = self.llvm.major;
        let mut check = |field: String, text: &str| {
            for (token, n) in llvm_tokens(text) {
                if n != major {
                    issues.push(format!(
                        "{field}: names {token} but [llvm].major is {major}"
                    ));
                }
            }
        };
        check("image.tool_path".to_owned(), &self.image.tool_path);
        for name in self.apt.keys() {
            check(format!("apt.{name}"), name);
        }
        for (key, spec) in &self.tool {
            check(format!("tool.{key}.bin"), &spec.bin);
            if let Some(rule) = &spec.llvm {
                for (i, arg) in rule.argv.iter().enumerate() {
                    check(format!("tool.{key}.llvm.argv[{i}]"), arg);
                }
            }
        }
    }
}

/// The LLVM majors the project accepts (PROJECT.md constraint, VERIFICATION R1).
const SUPPORTED_LLVM_MAJORS: std::ops::RangeInclusive<u32> = 16..=19;

/// The highest LLVM major any tool may report.
const MAX_LLVM_MAJOR: u32 = 19;

/// Words that mark a value nobody filled in.
const PLACEHOLDER_WORDS: [&str; 5] = ["todo", "tbd", "fixme", "placeholder", "discover"];

/// Compiled-regex size limit (threat T-01-10): 1 MiB.
const REGEX_SIZE_LIMIT: usize = 1024 * 1024;

fn finish(mut issues: Vec<String>) -> Result<(), PinsError> {
    issues.sort();
    issues.dedup();
    if issues.is_empty() {
        Ok(())
    } else {
        Err(PinsError::Invalid { issues })
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

fn check_name_list(issues: &mut Vec<String>, field: &str, names: &[String]) {
    if names.is_empty() {
        issues.push(format!("{field}: must not be empty"));
    }
    for (i, name) in names.iter().enumerate() {
        if names[..i].contains(name) {
            issues.push(format!("{field}: {name:?} is listed twice"));
        }
    }
}

/// `YYYYMMDDTHHMMSSZ`.
fn is_snapshot_timestamp(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 16
        && bytes[..8].iter().all(u8::is_ascii_digit)
        && bytes[8] == b'T'
        && bytes[9..15].iter().all(u8::is_ascii_digit)
        && bytes[15] == b'Z'
}

/// `D-` followed by one or more digits.
fn is_decision_id(text: &str) -> bool {
    text.strip_prefix("D-")
        .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
}

/// Exactly `parts` dot-separated runs of digits.
fn is_dotted_numeric(text: &str, parts: usize) -> bool {
    let mut count = 0;
    for part in text.split('.') {
        if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
            return false;
        }
        count += 1;
    }
    count == parts
}

/// Every `llvm-N`, `clang-N` and `clang-cppN` token in `text`, with its N.
fn llvm_tokens(text: &str) -> Vec<(String, u32)> {
    let mut found = Vec::new();
    for prefix in ["llvm-", "clang-", "clang-cpp"] {
        for (at, _) in text.match_indices(prefix) {
            let digits: String = text[at + prefix.len()..]
                .chars()
                .take_while(char::is_ascii_digit)
                .collect();
            if digits.is_empty() {
                continue;
            }
            let n = digits.parse::<u32>().unwrap_or(u32::MAX);
            found.push((format!("{prefix}{digits}"), n));
        }
    }
    found
}

/// True when `value` is empty, contains a marker character or a marker word.
fn is_placeholder(value: &str) -> bool {
    if value.trim().is_empty() || value.contains(['\u{2039}', '\u{203a}']) {
        return true;
    }
    value
        .split(|c: char| !c.is_alphanumeric())
        .any(|word| PLACEHOLDER_WORDS.contains(&word.to_lowercase().as_str()))
}

/// Walk every string in the pins; regex fields are exempt.
fn placeholder_issues(pins: &Pins, issues: &mut Vec<String>) {
    // Serialization of plain strings, numbers and maps cannot fail; if it ever
    // did, the structural checks above still run.
    if let Ok(value) = serde_json::to_value(pins) {
        walk_strings(&value, "", issues);
    }
}

fn walk_strings(value: &serde_json::Value, path: &str, issues: &mut Vec<String>) {
    match value {
        serde_json::Value::String(text) => {
            if is_placeholder(text) {
                issues.push(format!("{path}: empty or placeholder value"));
            }
        }
        serde_json::Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                walk_strings(item, &format!("{path}[{i}]"), issues);
            }
        }
        serde_json::Value::Object(map) => {
            for (key, item) in map {
                if key == "version_regex" || key == "regex" {
                    continue;
                }
                let child = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                walk_strings(item, &child, issues);
            }
        }
        _ => {}
    }
}

fn check_regex(issues: &mut Vec<String>, field: &str, pattern: &str, group: &str) {
    match RegexBuilder::new(pattern)
        .size_limit(REGEX_SIZE_LIMIT)
        .build()
    {
        Err(err) => issues.push(format!(
            "{field}: invalid regex: {}",
            err.to_string().replace('\n', " ")
        )),
        Ok(regex) => {
            if !regex.capture_names().flatten().any(|name| name == group) {
                issues.push(format!("{field}: regex has no `{group}` named group"));
            }
        }
    }
}

fn tool_issues(key: &str, spec: &ToolSpec, issues: &mut Vec<String>) {
    let f = format!("tool.{key}");
    match spec.status {
        ToolPinStatus::Installed => {
            if spec.reason.is_some() {
                issues.push(format!("{f}.reason: only allowed for not_installed tools"));
            }
            if !spec.bin.starts_with('/') {
                issues.push(format!("{f}.bin: must be an absolute path"));
            }
            check_regex(
                issues,
                &format!("{f}.version_regex"),
                &spec.version_regex,
                "version",
            );
            if spec.expect.is_empty() {
                issues.push(format!("{f}.expect: must not be empty"));
            }
            if let Some(rule) = &spec.llvm {
                llvm_rule_issues(&format!("{f}.llvm"), rule, issues);
            }
        }
        ToolPinStatus::NotInstalled => {
            if spec.reason.as_deref().is_none_or(|r| r.trim().is_empty()) {
                issues.push(format!(
                    "{f}.reason: required for not_installed tools (CONTEXT D-10)"
                ));
            }
            let stray = [
                ("bin", !spec.bin.is_empty()),
                ("version_args", !spec.version_args.is_empty()),
                ("version_regex", !spec.version_regex.is_empty()),
                ("expect", !spec.expect.is_empty()),
                ("env", !spec.env.is_empty()),
                ("llvm", spec.llvm.is_some()),
            ];
            for (name, set) in stray {
                if set {
                    issues.push(format!("{f}.{name}: not allowed on a not_installed tool"));
                }
            }
        }
    }
}

fn llvm_rule_issues(f: &str, rule: &LlvmRule, issues: &mut Vec<String>) {
    match rule.source {
        LlvmSource::Version => {
            if rule.regex.is_some() || !rule.argv.is_empty() {
                issues.push(format!("{f}: source \"version\" takes no regex or argv"));
            }
        }
        LlvmSource::Output => {
            if !rule.argv.is_empty() {
                issues.push(format!("{f}.argv: source \"output\" takes no argv"));
            }
            match &rule.regex {
                Some(pattern) => check_regex(issues, &format!("{f}.regex"), pattern, "major"),
                None => issues.push(format!("{f}.regex: required for source \"output\"")),
            }
        }
        LlvmSource::Probe => {
            if !rule.argv.first().is_some_and(|bin| bin.starts_with('/')) {
                issues.push(format!(
                    "{f}.argv: source \"probe\" needs a command whose first element is an absolute path"
                ));
            }
            match &rule.regex {
                Some(pattern) => check_regex(issues, &format!("{f}.regex"), pattern, "major"),
                None => issues.push(format!("{f}.regex: required for source \"probe\"")),
            }
        }
    }
}
