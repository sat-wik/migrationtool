//! Rendering of pins into Docker build arguments.
//!
//! `container/pins.toml` is the only place a version lives (CONTEXT D-14). The
//! Dockerfile declares every `ARG` without a default and receives the values
//! from `mt toolchain build-args`, so a version literal can never drift into
//! it (RESEARCH Pattern 2). Output is sorted by name and a value that could
//! split into two arguments (whitespace, newline, NUL or any control
//! character) is rejected (threat T-01-09).

use std::collections::BTreeMap;

use crate::pins::{Pins, SourcePin};

/// Which set of arguments to render.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Arguments the image build consumes.
    Image,
    /// Pins for CI host tools only (`BUILDKIT_IMAGE`, `CARGO_DENY_VERSION`).
    Ci,
}

/// Why pins could not be rendered.
#[derive(Debug, thiserror::Error)]
pub enum BuildArgsError {
    /// A value cannot be passed safely as `NAME=value`.
    #[error("build arg {key}: {reason}")]
    BadValue {
        /// The build argument name.
        key: String,
        /// What is wrong with the value.
        reason: String,
    },
    /// Two pins render to the same build argument name.
    #[error("build arg {key} is produced by more than one pin")]
    DuplicateKey {
        /// The colliding build argument name.
        key: String,
    },
}

/// Collects arguments, rejecting duplicate names and unsafe values.
#[derive(Default)]
struct Args(BTreeMap<String, String>);

impl Args {
    fn put(&mut self, key: &str, value: &str) -> Result<(), BuildArgsError> {
        if value.is_empty() {
            return Err(BuildArgsError::BadValue {
                key: key.to_owned(),
                reason: "value is empty".to_owned(),
            });
        }
        if value.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err(BuildArgsError::BadValue {
                key: key.to_owned(),
                reason: "value contains whitespace, a newline, a NUL or another control character"
                    .to_owned(),
            });
        }
        if self.0.insert(key.to_owned(), value.to_owned()).is_some() {
            return Err(BuildArgsError::DuplicateKey {
                key: key.to_owned(),
            });
        }
        Ok(())
    }
}

/// Upper snake case: ASCII letters and digits upper-cased, everything else `_`.
fn env_name(text: &str) -> String {
    text.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect()
}

fn image_args(pins: &Pins, args: &mut Args) -> Result<(), BuildArgsError> {
    let image = &pins.image;
    args.put("PLATFORM", &image.platform)?;
    args.put("BASE_REF", &image.base)?;
    args.put("BASE_DIGEST", &image.base_digest)?;
    args.put("SNAPSHOT_TIMESTAMP", &image.snapshot_timestamp)?;
    args.put("SOURCE_DATE_EPOCH", &image.source_date_epoch)?;
    args.put("TOOL_PATH", &image.tool_path)?;
    args.put("LLVM_MAJOR", &pins.llvm.major.to_string())?;

    let rust = &pins.rust;
    args.put("RUSTUP_HOME", &rust.rustup_home)?;
    args.put("CARGO_HOME", &rust.cargo_home)?;
    args.put("RUST_HOST", &rust.host)?;
    args.put("RUST_TOOL_VERSION", &rust.tool.version)?;
    args.put("RUST_TOOL_COMMIT", &rust.tool.commit)?;
    args.put(
        "RUST_TOOL_CHANNEL_SHA256",
        &rust.tool.channel_manifest_sha256,
    )?;
    args.put("RUST_TOOL_COMPONENTS", &rust.tool.components.join(","))?;
    args.put("RUST_TOOL_TARGETS", &rust.tool.targets.join(","))?;
    args.put("RUST_BITCODE_VERSION", &rust.bitcode.version)?;
    args.put("RUST_BITCODE_COMMIT", &rust.bitcode.commit)?;
    args.put(
        "RUST_BITCODE_CHANNEL_SHA256",
        &rust.bitcode.channel_manifest_sha256,
    )?;

    for (key, source) in &pins.source {
        let prefix = env_name(key);
        let fields: Vec<(&str, &str)> = match source {
            SourcePin::Git { url, tag, commit } => {
                vec![("URL", url), ("TAG", tag), ("COMMIT", commit)]
            }
            SourcePin::Crate {
                name,
                version,
                sha256,
            } => vec![("NAME", name), ("VERSION", version), ("SHA256", sha256)],
            SourcePin::Archive {
                version,
                url,
                sha256,
            } => vec![("VERSION", version), ("URL", url), ("SHA256", sha256)],
            SourcePin::Transitive { parent, commit } => {
                vec![("PARENT", parent), ("COMMIT", commit)]
            }
        };
        for (field, value) in fields {
            args.put(&format!("{prefix}_{field}"), value)?;
        }
    }
    Ok(())
}

fn ci_args(pins: &Pins, args: &mut Args) -> Result<(), BuildArgsError> {
    args.put("BUILDKIT_IMAGE", &pins.ci.buildkit_image)?;
    args.put("CARGO_DENY_VERSION", &pins.ci.cargo_deny_version)
}

/// Render the arguments of `scope` as a sorted map.
///
/// # Errors
/// [`BuildArgsError::BadValue`] for an empty value or one with whitespace, a
/// newline, a NUL or another control character; [`BuildArgsError::DuplicateKey`]
/// when two pins render to the same name.
pub fn render(pins: &Pins, scope: Scope) -> Result<BTreeMap<String, String>, BuildArgsError> {
    let mut args = Args::default();
    match scope {
        Scope::Image => image_args(pins, &mut args)?,
        Scope::Ci => ci_args(pins, &mut args)?,
    }
    Ok(args.0)
}

/// Render the arguments of `scope` as sorted `NAME=value` lines, one per argument.
///
/// # Errors
/// As [`render`].
pub fn render_lines(pins: &Pins, scope: Scope) -> Result<String, BuildArgsError> {
    let mut out = String::new();
    for (key, value) in render(pins, scope)? {
        out.push_str(&key);
        out.push('=');
        out.push_str(&value);
        out.push('\n');
    }
    Ok(out)
}
