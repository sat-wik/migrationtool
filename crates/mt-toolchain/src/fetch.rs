//! The pin-bump helper: the sha256 of a URL's content, fetched through the runner.

use std::time::Duration;

use crate::runner::RunnerError;

/// Why a URL could not be hashed.
#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    /// Only `https://` and `file://` URLs are accepted.
    #[error("unsupported URL {url:?}: only https:// and file:// are accepted")]
    UnsupportedScheme {
        /// The rejected URL.
        url: String,
    },
    /// The runner could not start or supervise curl.
    #[error(transparent)]
    Runner(#[from] RunnerError),
    /// curl exceeded the timeout; no digest is reported.
    #[error("curl timed out after {} s; no digest is reported", .0.as_secs())]
    TimedOut(Duration),
    /// curl exited non-zero; no digest is reported.
    #[error("curl failed (exit code {exit_code:?}); no digest is reported")]
    Failed {
        /// curl's exit code, `None` when it was killed by a signal.
        exit_code: Option<i32>,
    },
}

/// The lowercase hex sha256 of every byte a URL serves.
///
/// # Errors
/// [`FetchError`] for an unsupported scheme, a runner failure, a timeout or a
/// non-zero curl exit.
pub fn sha256_of_url(_url: &str, _timeout: Duration) -> Result<String, FetchError> {
    // Placeholder for the RED commit.
    Ok(String::new())
}
