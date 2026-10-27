#![forbid(unsafe_code)]
//! The test vector schema (Phase 3, EQV-02): the nine-field TRACTOR-shaped
//! vector, the byte codec, the comparison rule and the sha256-named corpus.
//!
//! The rules are written in `docs/specs/equivalence-contract.md`. The crate is
//! pure schema: it starts no process and reads only the files it is asked to
//! load.
//!
//! There are two validation levels: the reference level (`Vector`) and the
//! TRACTOR-compatible level (`TractorVector`).

pub mod bytes;
pub mod compare;
pub mod corpus;
pub mod state;
pub mod vector;
