//! The pin-bump helper: the sha256 of a URL's content, fetched through the runner.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use crate::runner::{self, RunRequest, RunnerConfig, RunnerError};

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

/// `PATH` for the curl child.
const CURL_PATH: &str = "/usr/local/bin:/usr/bin:/bin";

/// The lowercase hex sha256 of every byte a URL serves.
///
/// Only `https://` and `file://` URLs are accepted, checked here before
/// anything is spawned; curl is additionally limited to those protocols and to
/// `https` for redirects. curl runs through [`runner::run`] with an output cap
/// of 0, so the runner hashes every byte of the stream and stores none of it.
/// The runner clears the environment, so proxy variables are deliberately not
/// passed through (CONTEXT D-17); a host that needs a proxy computes the digest
/// itself.
///
/// # Errors
/// [`FetchError`] for an unsupported scheme, a runner failure, a timeout or a
/// non-zero curl exit. A digest is never returned for a fetch that did not
/// complete.
pub fn sha256_of_url(url: &str, timeout: Duration) -> Result<String, FetchError> {
    if !(url.starts_with("https://") || url.starts_with("file://")) {
        return Err(FetchError::UnsupportedScheme {
            url: url.to_owned(),
        });
    }
    let cfg = RunnerConfig {
        output_cap: 0,
        timeout,
        ..RunnerConfig::new(CURL_PATH, "0")
    };
    let request = RunRequest {
        tool_name: "curl".to_owned(),
        argv: [
            "curl",
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--proto",
            "=https,file",
            "--proto-redir",
            "=https",
            url,
        ]
        .iter()
        .map(OsString::from)
        .collect(),
        cwd: PathBuf::from("/"),
        extra_env: BTreeMap::new(),
        tool_version: None,
    };
    let output = runner::run(&cfg, &request)?;
    let exit = &output.record.exit;
    if exit.timed_out {
        return Err(FetchError::TimedOut(timeout));
    }
    if exit.code != Some(0) {
        return Err(FetchError::Failed {
            exit_code: exit.code,
        });
    }
    Ok(output.record.stdout.sha256)
}
