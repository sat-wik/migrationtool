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
//! attributes.
//!
//! [`entries`] confirms each hand-named entry function against the captured
//! build with clang on a probe file.
//!
//! [`fetch`] brings a pinned source into the git-ignored work directory and
//! verifies it on every reuse. Every external tool runs only through the
//! `mt-toolchain` runner.
//!
//! [`testbed`] wires fetch, capture and replay over the selected targets and
//! returns the per-step summaries `mt testbed` prints.

pub mod capture;
pub mod cc;
pub mod elf;
pub mod entries;
pub mod fetch;
pub mod manifest;
pub mod replay;
pub mod report;
pub mod seeding;
pub mod testbed;
pub mod treehash;
