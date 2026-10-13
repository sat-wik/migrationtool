#![forbid(unsafe_code)]
//! Test-bed support for migrationtool (Phase 2).
//!
//! [`manifest`] defines and validates `testbed/manifest.toml`, the committed
//! list of open-source C parsers the harness is exercised on, and [`treehash`]
//! computes the tree digest that pins each fetched source (CONTEXT D-17, D-20).
//! Fetch, capture and replay are added by later plans of the phase and run
//! their external tools only through the `mt-toolchain` runner.

pub mod cc;
pub mod elf;
pub mod manifest;
pub mod replay;
pub mod report;
pub mod treehash;
