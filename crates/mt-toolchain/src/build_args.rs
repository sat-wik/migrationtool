//! Rendering of pins into Docker build arguments (placeholder until the tests drive it in).

use std::collections::BTreeMap;

use crate::pins::Pins;

/// Which set of arguments to render.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Arguments the image build consumes.
    Image,
    /// Pins for CI host tools.
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
}

/// Render the arguments of `scope` as a sorted map.
///
/// # Errors
/// [`BuildArgsError::BadValue`] for a value with whitespace, a newline or NUL.
pub fn render(_pins: &Pins, _scope: Scope) -> Result<BTreeMap<String, String>, BuildArgsError> {
    Ok(BTreeMap::new())
}

/// Render the arguments of `scope` as sorted `NAME=value` lines.
///
/// # Errors
/// As [`render`].
pub fn render_lines(pins: &Pins, scope: Scope) -> Result<String, BuildArgsError> {
    let mut out = String::new();
    for (key, value) in render(pins, scope)? {
        out.push_str(&format!("{key}={value}\n"));
    }
    Ok(out)
}
