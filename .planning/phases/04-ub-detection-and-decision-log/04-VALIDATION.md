---
phase: "4"
slug: "ub-detection-and-decision-log"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-11-06"
---

# Phase 4 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust 1.99.0, edition 2024); integration tests under `crates/*/tests/`; `#[ignore]` for tests that need the pinned tools (the `eqv_04_exec.rs` pattern with `MT_LOCAL_CAPTURES`) |
| **Config file** | `Cargo.toml` workspace; `clippy.toml`; `.github/workflows/testbed.yml` (new `ub` job, D-25) |
| **Quick run command** | `cargo test -p mt-ub -p mt-contract -p mt-toolchain` |
| **Full suite command** | `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace --locked && cargo deny check` |
| **Estimated runtime** | ~300 seconds (full suite, warm cache) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p mt-ub -p mt-contract -p mt-toolchain`
- **After every plan wave:** Run the full suite command
- **Before `/gsd-verify-work 4`:** Full suite green, the `ub` job green on the new image digest, and the `mt ub check` notice reviewed
- **Max feedback latency:** 300 seconds locally; CI round trips for the pinned-tool steps (sanitizer runs, KLEE) are capped per plan

---

## Per-Task Verification Map

Task IDs from the plans (04-01..04-14); rows below map each requirement to its test command (from 04-RESEARCH.md § Validation Architecture).

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 4-01-01 | 01 | 1 | (D-24 WR-13) | — | Pipe holders killed via pidfd after a normal exit; no orphan keeps a stream open | integration | `cargo test -p mt-toolchain tool_01_runner_kills_a_pipe_holder` | ❌ W0 | ⬜ pending |
| 4-06-01, 4-06-02 | 06 | 3 | UB-01 | — | Report parser takes kind from SUMMARY and site from the innermost target frame | unit | `cargo test -p mt-ub ub_01_parse` | ❌ W0 | ⬜ pending |
| 4-06-02 | 06 | 3 | UB-01 | — | Host flag map exhaustive over `ALLOWED_OPTIONS`; flags derived from facts; layout mismatch → unavailable | unit | `cargo test -p mt-ub ub_01_host_flags` | ❌ W0 | ⬜ pending |
| 4-07-01, 4-07-02, 4-07-03 | 07 | 4 | UB-01 | — | Known-UB fixtures flagged with expected `source:kind`; clean inputs unflagged | CI self-test | `cargo test -p mt-ub ub_01_fixtures -- --ignored` (and `mt ub selftest` in the `ub` job) | ❌ W0 | ⬜ pending |
| 4-08-01 | 08 | 5 | UB-01 | — | Every MSan finding records the LP64 host note (A-08 settled) | unit | `cargo test -p mt-ub ub_01_msan_lp64` | ❌ W0 | ⬜ pending |
| 4-11-01 | 11 | 7 | UB-02 | — | `-O0` arguments derived from the canonical command; shipped `-O0` recorded as not applicable | unit | `cargo test -p mt-ub ub_02_optlevel_arguments` | ❌ W0 | ⬜ pending |
| 4-11-03 | 11 | 7 | UB-02 | — | Optimiser-sensitive fixture labelled `optdiff`; timeout-only difference is not optdiff | CI | `ub` job notice | ❌ W0 | ⬜ pending |
| 4-12-01..03, 4-13-01..02 | 12, 13 | 8, 9 | UB-03 | — | `.err`/`.ktest` parsing; completion predicate; floor ≥ 3 targets; timeout = not proven | unit + CI | `cargo test -p mt-ub ub_03_` | ❌ W0 | ⬜ pending |
| 4-05-01, 4-05-03 | 05 | 2 | UB-04 | — | Facts from `-dM -E` and DWARF; changed fact unsigns the block | unit | `cargo test -p mt-ub ub_04_facts` | ❌ W0 | ⬜ pending |
| 4-04-02, 4-04-03, 4-05-02, 4-10-03 | 04, 05, 10 | 1, 2, 6 | UB-05 | — | Strict TOML log; decided = all four fields; merge never deletes; bulk sign excludes facts and non-default actions; policy/log cross-check; `mt ub check` exit codes | unit + CLI | `cargo test -p mt-contract ub_05_ && cargo test -p mt-cli ub_05_` | ❌ W0 | ⬜ pending |
| 4-10-02 | 10 | 6 | (D-10) | — | Label grammar and source order in the contract; version bumped; no forbidden claim words | doc test | `cargo test -p mt-contract eqv_01_contract_document_states_ub_01_labels_order_and_abis` | ✅ (extend) | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] (4-05-01) `crates/mt-ub/` crate skeleton (`#![forbid(unsafe_code)]`, workspace lints) added to workspace `members`
- [ ] (4-07) `crates/mt-ub/fixtures/` D-08 C samples (signed overflow, OOB read, uninitialised read, oversized shift, optimiser-sensitive), each with triggering and clean inputs and manifest-role sidecars
- [ ] (04-02 notices → 4-06-01, 4-12-02) Captured clang-16 report texts (ASan i386, UBSan i386, MSan x86-64) and one KLEE `.err`/`.ktest` from the image smoke step, committed as parser fixtures
- [ ] (4-01-01) Test-first rewrite of `tool_01_runner_marks_a_stream_held_by_an_orphaned_escapee_incomplete` and `tool_01_runner_normal_exit_with_an_orphaned_escapee_is_incomplete_not_an_error` for WR-13

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Image PR merged to `main` and new Current image digest recorded | UB-01, UB-03 (D-27) | Only the founder merges to main; only main publishes (WR-09) | Founder merges the image PR; confirm the container run on main publishes and the digest is recorded in `container/README.md` and PROJECT.md |
| Founder signs the facts blocks and log entries on the test bed | UB-04, UB-05 | Signing is a human decision (D-28, D-35) | Run `mt ub sign --accept-default --approver <name>` and sign facts; `mt ub check` then prints `ub-complete` |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 300s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
