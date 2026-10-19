---
gsd_state_version: "1.0"
milestone: v1.0
milestone_name: MVP
current_phase: 02
current_phase_name: Test Bed and Build Capture
status: executing
stopped_at: Completed 02-14-PLAN.md
last_updated: "2026-10-19T21:32:34.361Z"
last_activity: 2026-10-19
last_activity_desc: Phase 02 execution started
state_head: 2ef8c6d5cc4ccaa54c78d05e7a12b3c8a17d4a1b
progress:
  total_phases: 12
  completed_phases: 1
  total_plans: 28
  completed_plans: 23
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-10-12)

**Core value:** Every migrated module ships with evidence, from a harness proven to catch planted bugs, that the Rust matches the C on every defined-behavior input; metric is the share of C modules reaching full evidence with zero divergences.
**Current focus:** Phase 02 — Test Bed and Build Capture

## Current Position

Phase: 02 (Test Bed and Build Capture) — EXECUTING
Plan: 5 of 19
Status: Ready to execute
Last activity: 2026-10-19 — Phase 02 execution started

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
| Phase 02 P01 | 34 min | 3 tasks | 12 files |
| Phase 02 P02 | 10min | 3 tasks | 1 files |
| Phase 02 P06 | 25min | 2 tasks | 15 files |
| Phase 02 P08 | 26 min | 3 tasks | 1759 files |
| Phase 02 P09 | 45min (resumed session) | 3 tasks | 37 files |
| Phase 02 P11 | 13 min | 2 tasks | 26 files |
| Phase 02 P13 | 4 min | 3 tasks | 7 files |
| Phase 02 P12 | 10 min | 2 tasks | 7 files |
| Phase 02 P14 | 5 min | 2 tasks | 5 files |

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
- [Phase 02]: 02-01: D-17 image extension: git, cmake, curl, bzip2 added to the final stage and make gets an [apt] pin (bzip2 1.0.8-5+b1, cmake 3.25.1-1, curl 7.88.1-10+deb12u15, git 1:2.39.5-0+deb12u3, make 4.3-4.1 at snapshot 20261005T000000Z) — Needed by D-07 CMake builds, D-19 fetch by commit and the Mbed TLS archive source; proven green on the session branch (container run 38338543797); only a push to main publishes (WR-09), so plan 02-02 ships it through a PR
- [Phase 02]: 02-02: Current image is sha256:e83006c0a7e37f788afcbd1c95554e8c4dfc61c3b68da3ff964caf47fb3851b9, published by container run 38342425433 (push to main, merge commit 69b5b7f4, PR #4); previous sha256:42c7fb0e... superseded
- [Phase 02]: 02-06: the testbed determinism comparison excludes only the stdout digest and byte counts of the steps 'cmake configure' and 'clang ast <fn>' (CMake prints its elapsed time; clang's JSON AST holds node addresses); every other report field and every compile_commands file is compared byte for byte — Both fields change between identical runs and the runner records only digests; the entry results derived from the dump stay compared. Stated in the workflow and every testbed-determinism notice. Tracer green: run 38380382970, head 4195a6d, 4 pairs identical in the pinned image
- [Phase 02]: 02-06: CMake's compiler-ABI probe output (CMakeFiles/<ver>/CMakeDetermineCompilerABI_*.bin) is not a configure-time input and is excluded from configure_files; notices longer than 9 annotation parts are published by several steps (GitHub keeps 10 annotations per level per step) — Found by the determinism step (Task 2 RT1) and the annotation listing (RT2); both fixed within the 3-round-trip cap
- [Phase 02]: 02-08: zlib has no extracted seeds; test/example.c and test/infcover.c yield message text and hex text inside C strings, not zlib streams, so its 18 seeds are hand-made — A string extraction there would commit misleading seeds
- [Phase 02]: 02-08: a zip archive above 2 MiB is not opened and a source file above 8 MiB is not read; seed_source paths may not be . or .git — The member count of an archive is unbounded; the archive size bounds the memory
- [Phase 02]: 02-08: the mt testbed work directory is made absolute in mt-cli — git runs with the cache directory as cwd, so the relative default .mt-work broke verification of reused git sources
- [Phase 02]: 02-09: CMakeFiles/Makefile2 is left out of the configure-time hashes by exact path (CMake pointer-order nondeterminism, measured; flags.make and every other generated file stay hashed)
- [Phase 02]: 02-09: bed retained at 15 of 16 targets (29 pairs); isotp and paho/server dropped with cause entry_mismatch (testbed run 38387515333); final full-bed run 38430137113 green on f0a36b8, 3 of 3 round trips
- [Phase 02]: 02-11: entry probe reproduces the defining TU's include/define/undef prefix exactly or falls back to the header alone; comparison untouched; EntryResult.probe_prefix skipped when empty — G-02-1: isotp.h needs the stdint.h its TU includes first; guessing a context would let a declaration pass that the TU never sees (T-02-41)
- [Phase 02]: 02-11: isotp reopened (drop entry and DROPPED.md section removed, 19-seed corpus committed, README at 16 targets/30 pairs); paho/server drop kept — Founder decision G-02-1; proof is the plan 02-17 testbed run, and isotp is dropped again with cause and run id if it fails
- [Phase 02]: 02-13: zip seed archives are inflated by flate2 rust_backend (miniz_oxide) with zlib-rs gone from Cargo.lock; each archive has an 8 MiB decompressed budget — G-02-4 and WR-07. Founder approved miniz_oxide 0.9.1, adler2 2.0.1 and simd-adler32 0.3.10 at the legitimacy gate. Safe-Rust wording: the inflater and adler2 forbid unsafe code; simd-adler32, zip header parsing and flate2 buffer code contain unsafe code; no C code. Member-count cap stays deferred to a founder decision. Image proof is plan 02-17.
- [Phase 02]: G-02-2: CMakeFiles/Makefile2 hashed normalised (prerequisite lines of one target sorted within a run), rule recorded in normalised_configure_files; .a/.so excluded from configure hashes only when the cmake build step created them (WR-10)
- [Phase 02]: 02-14: mt testbed compare compares two capture trees in Rust and fails closed: unparseable, empty, one-sided, non-regular or zero files are differences; only DETERMINISM_EXCLUSIONS (stdout sha256, bytes_total, bytes_stored of cmake configure and clang ast steps in capture reports) are excluded — WR-05: jq | cmp -s reported identical when jq failed on both files; plan 02-17 switches the workflow step to the command

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
| 261012-6i0 | Point container/README.md Current image at the first main-published image | 2026-10-12 | 8bb40a3 | [261012-6i0-point-container-readme-md-current-image-](./quick/261012-6i0-point-container-readme-md-current-image-/) |

### Parallel Business Track (not phased)

- Interview about 20 buyers (spec weeks 4-10), land 2-3 design partners, run one paid pilot (weeks 8-13), collect at least 2 pilots or LOIs; enter the count by hand in the Phase 12 gate report. Details in PROJECT.md Context.

## Deferred Items

Items acknowledged and deferred at milestone close, most recent first:

| Category | Item | Status | Deferred At | Milestone |
|----------|------|--------|-------------|-----------|
| *(none)* | | | | |

## Session Continuity

Last session: 2026-10-19T21:32:34.236Z
Stopped at: Completed 02-14-PLAN.md
Resume file: None
