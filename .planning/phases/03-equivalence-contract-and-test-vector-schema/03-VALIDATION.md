---
phase: "03"
slug: "equivalence-contract-and-test-vector-schema"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-10-21"
---

# Phase 03 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution. Seeded from `03-RESEARCH.md` § Validation Architecture (Nyquist).

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | The Rust built-in test harness (`cargo test`), with `tempfile` for work directories. No new dev-dependencies (D-29). |
| **Config file** | Workspace `Cargo.toml`, and `clippy.toml`, which bans `std::process::Command` outside the runner. |
| **Quick run command** | `cargo test -p mt-contract -p mt-vectors` (plus the crate under change) |
| **Full suite command** | `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace --locked && cargo deny check` |
| **Estimated runtime** | Quick: about 20 seconds. Full: about 300 seconds. |

Local-only ignored tests follow the Phase 2 convention: `MT_LOCAL_CAPTURES=$PWD/.mt-work/capture cargo test -p <crate> --test <file> -- --ignored --exact <name> --nocapture`. The TRACTOR test uses `MT_LOCAL_TRACTOR=<dir>`. Tests never use the network or a live model.

---

## Sampling Rate

- **After every task commit:** run the quick command plus the crate under change.
- **After every plan wave:** run the full suite command, plus the ignored local executor tests on a host that has `qemu-arm`.
- **Before `/gsd-verify-work`:** the full suite must be green, and the `testbed` run on the final code commit must be green with the new notices: drivers built, vectors produced, twice-run mismatches = 0, and at least 3 TRACTOR vectors loaded.
- **Max feedback latency:** 300 seconds.

---

## Per-Task Verification Map

Task IDs are filled in by the planner and plan checker. The rows below are the requirement-level contract each plan's tasks must cover.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 03-02-T1, 03-02-T2 | 03-02 | 2 | EQV-01 | T-03-05, T-03-06, T-03-07 | The policy loads. An unknown key is rejected. The default is `return_error`. Changing the default needs a `default_change` record. An override missing any required key, or naming a site that was not logged, is rejected. An empty override table is accepted. | unit | `cargo test -p mt-contract eqv_01` | ❌ W0 | ⬜ pending |
| 03-02-T3 | 03-02 | 2 | EQV-01 | T-03-08 | The contract document has a semver version and a changelog. It states refinement, the observation point, the comparison rule and the three actions. It never contains "certified" or "compliant". | unit (doc check) | `cargo test -p mt-contract eqv_01_contract_document` | ❌ W0 | ⬜ pending |
| 03-03-T1, 03-03-T2 | 03-03 | 3 | EQV-02 | T-03-13 | A vector has at most the nine fields. An unknown key is rejected, and so is a merged `lib_state`. `has_ub` absent means no observed UB, and `has_ub: false` is never emitted (D-26). `stdout` serializes as `{pattern, is_regex: false}`. | unit | `cargo test -p mt-vectors eqv_02` | ❌ W0 | ⬜ pending |
| 03-03-T1 | 03-03 | 3 | EQV-02 | — | The Latin-1 codec (D-25) round-trips all 256 byte values, NUL included, in `stdin` and `stdout`. | unit (exhaustive) | `cargo test -p mt-vectors eqv_02_byte_codec` | ❌ W0 | ⬜ pending |
| 03-03-T3 | 03-03 | 3 | EQV-02 | T-03-09, T-03-10 | At least 3 TRACTOR vectors load unchanged and re-serialize JSON-equal. | integration (ignored locally, plus CI) | `MT_LOCAL_TRACTOR=... cargo test -p mt-vectors --test eqv_02_tractor -- --ignored --exact eqv_02_tractor_vectors_load_unchanged` | ❌ W0 | ⬜ pending |
| 03-03-T3 | 03-03 | 3 | EQV-02 | T-03-11 | The committed TRACTOR pin lists only the commit, paths and sha256 hashes. No TRACTOR content is in the repo. | unit | `cargo test -p mt-vectors eqv_02_tractor_pin_has_no_content` | ❌ W0 | ⬜ pending |
| 03-04-T1, 03-04-T2 (extended in 03-05..03-08) | 03-04 | 4 | EQV-03 | T-03-14 | Manifest roles reject a bad role, a wrong parameter count, a forbidden token in setup args, and a missing bound on a length. | unit | `cargo test -p mt-capture eqv_03_roles` | ❌ W0 | ⬜ pending |
| 03-06-T3 (completeness 03-08-T2) | 03-06 | 6 | EQV-03 | T-03-23, T-03-28 | At least 10 targets are drivable, counted from the committed manifest and the adapter drop log (`no_seeds` and others logged with a reason, D-28). | unit (committed files) | `cargo test eqv_03_adapter_floor_is_ten` | ❌ W0 | ⬜ pending |
| 03-04-T2 (extended in 03-05..03-07) | 03-04 | 4 | EQV-03 | T-03-14 | The generated driver source is stable (golden) for the tracer targets. | golden | `cargo test eqv_03_driver_golden` | ❌ W0 | ⬜ pending |
| 03-04-T2 | 03-04 | 4 | EQV-04 | T-03-16 | The link recipe refuses an object whose sha256 differs from the capture record. | unit | `cargo test eqv_04_refuses_modified_object` | ❌ W0 | ⬜ pending |
| 03-01-T2 (setup_failed in 03-07-T1) | 03-01 | 1 | EQV-04 | T-03-01, T-03-04 | The outcome classification maps each runner result to exactly one class: ok, fault, abort or timeout. Fault and abort give `rc = -signal` and a `has_ub` crash label (D-25, D-27). | unit (table-driven) | `cargo test eqv_04_classify` | ❌ W0 | ⬜ pending |
| 03-05-T1 | 03-05 | 5 | EQV-04 | T-03-19 | The layout parser reads a committed `llvm-dwarfdump` text fixture covering a struct with a bitfield, an array, a 1-byte enum, a union and a typedef. | unit | `cargo test eqv_04_layout_parser` | ❌ W0 | ⬜ pending |
| 03-04-T1, 03-04-T3 | 03-04 | 4 | EQV-04 | T-03-15, T-03-17 | Running the same vector twice gives identical vector bytes for the tracer targets. | integration (ignored locally, plus CI) | `cargo test --test eqv_04_exec -- --ignored --exact eqv_04_run_twice_is_byte_identical` | ❌ W0 | ⬜ pending |
| 03-01-T2, 03-01-T3 | 03-01 | 1 | EQV-04 | T-03-01 | An overrun into the guard page gives outcome `fault`, `rc = -11`, a `has_ub` label, and no core file. | integration (ignored locally, plus CI) | `... eqv_04_overrun_is_a_fault` | ❌ W0 | ⬜ pending |
| 03-01-T2 | 03-01 | 1 | EQV-04 | T-03-02 | A timeout gives outcome `timeout` and is never recorded as `ok`. | integration | `... eqv_04_timeout_is_recorded` | ❌ W0 | ⬜ pending |
| 03-04-T2 | 03-04 | 4 | EQV-04 (D-07) | — | The output buffer's sentinel prefill is recorded, and the whole buffer is captured. | integration | `... eqv_04_sentinel_prefill_recorded` | ❌ W0 | ⬜ pending |
| 03-04-T3, 03-08-T3 | 03-08 | 8 | EQV-04 (D-19, D-21) | T-03-29, T-03-31 | The provenance sidecar hashes match, and the committed sample regenerates identically. | CI | `mt vectors check` (testbed step) | ❌ W0 | ⬜ pending |
| 03-01-T3 (extended in 03-03, 03-04, 03-05, 03-08) | 03-01 | 1 | workflow | T-03-03, T-03-12, T-03-15 | `testbed.yml` contains the new steps, and every one except the TRACTOR fetch runs with `--network none`. | static workflow test | `cargo test eqv_04_workflow_static` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] Crates `crates/mt-contract`, `crates/mt-vectors` and the executor crate (the research names `mt-harness`), with the tests named above.
- [ ] A fixture of `llvm-dwarfdump` text output, and synthetic runner records for the classification table.
- [ ] The infinite-loop fixture and the overrun fixture, generated at test time from the shared runtime template, not committed as per-target C.
- [ ] A CI QEMU smoke step that measures QEMU 7.2 and GCC 14.3 behavior in the pinned image (D-30) before the layout and adapter work depends on it.
- No framework install is needed.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| The contract's prose is accurate to the decisions | EQV-01 | Judgment over the wording, beyond the automated keyword checks | Read `docs/specs/equivalence-contract.md` against 03-CONTEXT.md D-05..D-30 |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 300s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
