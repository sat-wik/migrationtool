#![forbid(unsafe_code)]
//! The equivalence contract as data (Phase 3, EQV-01).
//!
//! The rules themselves are written in `docs/specs/equivalence-contract.md`;
//! this crate holds only the parts a program must read: the contract version
//! and the per-module UB policy format with its strict loader and validation
//! rules ([`policy`]). [`CONTRACT_VERSION`] must equal the `**Version:**` line
//! of that document (a test in this crate enforces it).
//!
//! The crate does data validation only: it starts no process and reads no
//! file except the policy file it is asked to load.

pub mod policy;

pub use policy::{
    Action, DefaultChange, ErrorCode, ErrorValue, Override, Policy, PolicyError, Site, Ub,
};

/// The version of `docs/specs/equivalence-contract.md` this crate implements.
pub const CONTRACT_VERSION: &str = "0.1.0";

/// The only policy file `schema_version` this crate understands.
pub const POLICY_SCHEMA_VERSION: u32 = 1;
