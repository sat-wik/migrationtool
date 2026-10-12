---
gsd_state_version: "1.0"
milestone: v1.0
milestone_name: MVP
current_phase: 2
current_phase_name: Test Bed and Build Capture
status: planning
stopped_at: Phase 2 context gathered
last_updated: "2026-10-12T04:39:18.720Z"
last_activity: 2026-10-12
last_activity_desc: Phase 01 complete, transitioned to Phase 2
state_head: 99647103ab078bc735dd4f52c220527fbb1e7657
progress:
  total_phases: 12
  completed_phases: 1
  total_plans: 9
  completed_plans: 9
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-10-12)

**Core value:** Every migrated module ships with evidence, from a harness proven to catch planted bugs, that the Rust matches the C on every defined-behavior input; metric is the share of C modules reaching full evidence with zero divergences.
**Current focus:** Phase 2 — Test Bed and Build Capture

## Current Position

Phase: 2 — Test Bed and Build Capture
Plan: Not started
Status: Ready to plan
Last activity: 2026-10-12 — Phase 01 complete, transitioned to Phase 2

Progress: [█░░░░░░░░░] 8%

## Performance Metrics

**Velocity:**
- Total plans completed: 9
- Average duration: - min
- Total execution time: 0.0 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 01 | 9 | - | - |

**Recent Trend:**
- Last 5 plans: none yet
- Trend: n/a

*Updated after each plan completion*
**Per-Plan Metrics:**

| Plan | Duration | Tasks | Files |
|------|----------|-------|-------|
| Phase 01 P06 | 12 min | 3 tasks | 7 files |
| Phase 01 P07 | 130 min | 3 tasks | 9 files |
| Phase 01 P08 | 13 min | 2 tasks | 3 files |
| Phase 01 P09 | 35 min | 3 tasks | 6 files |

## Accumulated Context

### Decisions

Full log in PROJECT.md (`<decisions>` D-01 to D-14, Key Decisions table). Recent:

- [Init]: ADR-0001 LOCKED: Rust Cargo workspace, analysers as pinned subprocesses, Rust agent loop behind a provider trait (D-01 to D-04)
- [Init]: Harness before translator; Phase 7 proves the harness and gates before Phase 8 starts (D-05)
- [Init]: Differential harness runs ILP32 with `-funsigned-char`, separate ASan+UBSan and MSan builds (D-06, D-07)
- [Init]: R1 is a required spike; R2 is time-boxed and may slip to v2.0 (D-10)
- [Init]: MVP "full evidence" excludes Kani proofs (D-14, assumption A-01, awaiting founder confirmation)
- [Phase 01]: D-15: provisional pin llvm=16 bitcode-rustc=1.72.1 recorded in PROJECT.md; D-16: runner.rs is the only std::process module — CONTEXT D-01, D-03, D-21: one LLVM major for all C-side tools, gate fails cargo test if either value moves without a matching decision line
- [Phase 01]: qemu_arm and qemu_system_arm expect the full apt candidate version; confirm against real banner in 01-07 — Plan 01-06 wording; the +b3 binNMU suffix may not appear in the banner, a risk settled by the first container capture
- [Phase 01]: 01-07: c2rust LLVM probe retargeted to c2rust-transpile (the c2rust dispatcher links no LLVM); version format is 'C2Rust 0.22.1'
- [Phase 01]: 01-07: QEMU pins use the Debian version in the banner parentheses incl. +b3; fixtures are real CI captures from container run 37969688167
- [Phase 01]: Plan 01-08: the pushed GHCR image is the exact image that passed the check, off-pin, manifest and smoke steps; container reference ghcr.io/sat-wik/migrationtool-toolchain@sha256:b8032124716db8f2bfdfea5b693665bb57f37972895f038c2948affb5956e46a (container run 37983108094) — Only image-a holds write access to packages via GITHUB_TOKEN; verified-image is announced only after manifest-diff succeeds; consumers pin the digest, never a tag (T-01-26, T-01-27)
- [Phase 01]: 01-09: channel-manifest pins bind installed bytes through a file:// RUSTUP_DIST_SERVER mirror with --no-self-update; rustup's stored multirust-channel-manifest.toml is a re-serialization and is never used as evidence
- [Phase 01]: 01-09: c2rust and cargo-mutants install with cargo install --locked --path from the sha256-verified, unpacked .crate; image-a pin-binding gate matches whole build-record lines to the pins before the GHCR push; verified image ghcr.io/sat-wik/migrationtool-toolchain@sha256:0f3b392427d2588e6ba0e3da2067bc503f40e6749b48bc6553832c27e96e518d (container run 38019633821)
- [Phase 01]: Final verified image ghcr.io/sat-wik/migrationtool-toolchain@sha256:0f5a5d2594b0405931ad7e85d5c6d2f5e3f2d5099d769059870817ee50e152ee (container run 38198934488, manifest sha256 d7002918…); GHCR publishes from main only from now on (WR-09); founder UAT passed on this digest

### Pending Todos

None yet.

### Blockers/Concerns

- [Phase 01]: Runner residual WR-13: after a normal exit a `setsid` escapee holding an output pipe is not killed (record shows `exit.tree_kill_sent` and `complete: false`). Close (pipe-holder lookup via /proc/*/fd + pidfd) before KLEE or any forking analyser runs through the runner. 21 Info findings stay open in 01-REVIEW-DISPOSITION.md.

- [Init]: A-01 "full evidence" definition is a roadmapper proposal; confirm before Phase 11 builds the predicate (earlier if Phase 5 or 6 planning needs the fuzz-budget number).
- [Init]: Schedule risk (A-10): 12 phases at 1-2 part-time weeks each is 12-24 weeks against a 13-week Day-90 window. Cut line is in ROADMAP.md Overview (stage 2, R2 spike, scoping report).
- [Init]: Hayroll licence unconfirmed (R3). Send the inquiry to the authors now because replies have lead time; the decision gate is Phase 8.
- [Init]: A-08, MSan is believed to lack 32-bit x86 support (unverified recollection). Verify in Phase 4; affects how R5 and R6 combine.
- [Init]: c2rust fails on 15 of 16 embedded codebases (CCS 2024). Phase 8 must treat its output as a hint, never a dependency.
- [Init]: Open spec questions: first RTOS (Zephyr or FreeRTOS), acceptability of "return error" as UB default, IAR/Keil linking. First one gets recorded in Phase 2.
- [Init]: TRACTOR corpus licence unchecked; use locally only until Phase 12 checks it.
- [Init]: Before starting, check the day-job invention-assignment clause and keep work on own time and hardware.

### Quick Tasks Completed

| # | Description | Date | Commit | Directory |
|---|-------------|------|--------|-----------|
| 261011-uhy | Phase 1 residual hardening (WR-01, WR-04, WR-06, WR-07) | 2026-10-11 | bcf56e3 | [261011-uhy-phase-1-residual-hardening-wr-01-wr-04-w](./quick/261011-uhy-phase-1-residual-hardening-wr-01-wr-04-w/) |

### Parallel Business Track (not phased)

- Interview about 20 buyers (spec weeks 4-10), land 2-3 design partners, run one paid pilot (weeks 8-13), collect at least 2 pilots or LOIs; enter the count by hand in the Phase 12 gate report. Details in PROJECT.md Context.

## Deferred Items

Items acknowledged and deferred at milestone close, most recent first:

| Category | Item | Status | Deferred At | Milestone |
|----------|------|--------|-------------|-----------|
| *(none)* | | | | |

## Session Continuity

Last session: 2026-10-12T04:39:18.317Z
Stopped at: Phase 2 context gathered
Resume file: .planning/phases/02-test-bed-and-build-capture/02-CONTEXT.md
