#![forbid(unsafe_code)]
//! Test-bed support for migrationtool (Phase 2).
//!
//! [`manifest`] defines and validates `testbed/manifest.toml`, the committed
//! list of open-source C parsers the harness is exercised on, and [`treehash`]
//! computes the tree digest that pins each fetched source (CONTEXT D-17, D-20)
//! and copies a tree symlink-safely.
//!
//! The offline capture and replay core: [`cc`] is the canonical
//! `compile_commands.json` form and its gates, [`replay`] re-runs captured
//! compiles in a clean copy and compares objects, [`report`] is the
//! deterministic replay report, and [`elf`] reads sections and Arm build
//! attributes. Fetch and capture drivers are added by later plans of the phase.
//! Every external tool runs only through the `mt-toolchain` runner.

pub mod cc;
pub mod elf;
pub mod manifest;
pub mod replay;
pub mod report;
pub mod treehash;
