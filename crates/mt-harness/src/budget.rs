//! The fixed run budgets (CONTEXT D-02).
//!
//! They are constants, not manifest keys, and are recorded in every provenance
//! record later (D-19). A run that exceeds the time budget is the outcome
//! `timeout`, never retried and never a pass.

use std::time::Duration;

/// The budgets of one executor run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budgets {
    /// Wall-clock limit per run, in milliseconds.
    pub timeout_ms: u64,
    /// Reserved guest address space for `qemu-arm -R`; also the memory budget.
    pub reserved_va: u32,
    /// Bytes of the `_sbrk` arena the C code's `malloc` draws from.
    pub heap_arena: u32,
    /// Byte the output buffer is prefilled with (D-07).
    pub sentinel: u8,
    /// Pattern the slack before a right-aligned buffer is prefilled with (D-04).
    pub slack_fill: u8,
    /// Largest request input, in bytes (the manifest's `max_seed_bytes`).
    pub max_input: u32,
}

impl Budgets {
    /// The budgets every run uses unless a case states its own timeout.
    pub const DEFAULT: Self = Self {
        timeout_ms: 10_000,
        reserved_va: 0x1000_0000,
        heap_arena: 1024 * 1024,
        sentinel: 0xA5,
        slack_fill: 0x5A,
        max_input: 65_536,
    };

    /// The time budget as a [`Duration`].
    #[must_use]
    pub const fn timeout(&self) -> Duration {
        Duration::from_millis(self.timeout_ms)
    }
}
