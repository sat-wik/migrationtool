#![forbid(unsafe_code)]
//! Pinned-toolchain support for migrationtool.
//!
//! Every external analyser the tool uses is launched through [`runner::run`],
//! which controls the environment, captures and hashes output, enforces a
//! timeout and returns a versioned run record. [`pins`] describes the pinned
//! tools and [`check`] verifies the live tools against those pins.

pub mod check;
pub mod pins;
pub mod runner;
