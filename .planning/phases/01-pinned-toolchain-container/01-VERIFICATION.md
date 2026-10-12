---
phase: 01-pinned-toolchain-container
verified: 2026-10-12T04:10:00Z
status: passed
score: 4/4 must-haves verified
covered_files:
  - .github/workflows/ci.yml
  - .github/workflows/container.yml
  - .github/workflows/pin-discovery.yml
  - .planning/phases/01-pinned-toolchain-container/01-01-PLAN.md
  - .planning/phases/01-pinned-toolchain-container/01-01-SUMMARY.md
  - .planning/phases/01-pinned-toolchain-container/01-02-PLAN.md
  - .planning/phases/01-pinned-toolchain-container/01-02-SUMMARY.md
  - .planning/phases/01-pinned-toolchain-container/01-03-PLAN.md
  - .planning/phases/01-pinned-toolchain-container/01-03-SUMMARY.md
  - .planning/phases/01-pinned-toolchain-container/01-04-PLAN.md
  - .planning/phases/01-pinned-toolchain-container/01-04-SUMMARY.md
  - .planning/phases/01-pinned-toolchain-container/01-05-PLAN.md
  - .planning/phases/01-pinned-toolchain-container/01-05-SUMMARY.md
  - .planning/phases/01-pinned-toolchain-container/01-06-PLAN.md
  - .planning/phases/01-pinned-toolchain-container/01-06-SUMMARY.md
  - .planning/phases/01-pinned-toolchain-container/01-07-PLAN.md
  - .planning/phases/01-pinned-toolchain-container/01-07-SUMMARY.md
  - .planning/phases/01-pinned-toolchain-container/01-08-PLAN.md
  - .planning/phases/01-pinned-toolchain-container/01-08-SUMMARY.md
  - .planning/phases/01-pinned-toolchain-container/01-09-PLAN.md
  - .planning/phases/01-pinned-toolchain-container/01-09-SUMMARY.md
  - container/Dockerfile
  - container/README.md
  - container/pins.toml
  - crates/mt-toolchain/src/check.rs
  - crates/mt-toolchain/src/runner.rs
  - crates/mt-toolchain/tests/container_static.rs
  - crates/mt-toolchain/tests/tool_01_runner.rs
covered_digest: "v3:sha256:cfc8ee922e8b9498e8057ff25f3e9eca6e28fb7b2a804cd29597f429c9a3fa95"
behavior_unverified: 0
overrides_applied: 0
gaps: []
deferred: []
behavior_unverified_items: []
coincidental_reliance_items: []
advisory:
  - finding: "WR-13 residual: after a normal exit, a tool descendant that left the process group (setsid) has already been reparented, so the runner's /proc walk cannot find it. The record shows that stream as complete=false and the field exit.tree_kill_sent says only that SIGKILL was sent, not that every pipe holder died."
    category: other
    reason: "Fixed by renaming and documenting (commit 2916af8), not by closing the gap. No Phase 1 tool is a daemonizing one. Close it (find pipe holders by pipe inode under /proc/*/fd) before KLEE or another forking analyser is run through the runner in Phase 2 or later."
    evidence_status: "01-REVIEW-FIX.md; test tool_01_runner_normal_exit_with_an_orphaned_escapee_is_incomplete_not_an_error passes locally"
  - finding: "IN-01..IN-21 from 01-REVIEW.md remain at disposition 'open' (21 open of 35). Most relevant: IN-11 (the manifest holds versions and pins, not installed content, so manifest identity proves version-level reproducibility only), IN-14 (dependency crates bound through the packaged Cargo.lock), IN-08 (final image runs as root), IN-09 (pin-discovery does not compare the Arm checksum), IN-18 (no upper bound on the post-kill wait)."
    category: other
    reason: "Hardening and documentation items; none contradicts a ROADMAP success criterion. IN-11 matches what criterion 3 and TOOL-03 actually say (identical tool-version manifests)."
    evidence_status: "01-REVIEW.md and 01-REVIEW-DISPOSITION.md read"
  - finding: "Latest head b974caa: ci and container runs (38201196211, 38201196303) were still in_progress when this report was written. Only the mt job had finished (success)."
    category: other
    reason: "b974caa and its parents 1c4017b, 88b4bc4, 58d7ae9 change only planning files; the last source change after the last fully green container run (3fa71a1, run 38199978785) is 2916af8, which touches only runner.rs and tool_01_runner.rs (a field rename). Local fmt, clippy -D warnings and the whole test suite pass on that source. The Dockerfile, pins.toml, smoke crate and check.rs are byte-identical to what run 38198934488 built and published (git diff defbe68..HEAD lists no Dockerfile, pins or check.rs change). Re-read the run results before merging."
    evidence_status: "gh api actions/runs; git diff --stat defbe68 HEAD"
---

# Phase 1: Pinned Toolchain Container Verification Report

**Phase Goal:** Every compiler and analyser the harness and translator need runs as a pinned subprocess inside one reproducible container, on a single pinned LLVM.
**Verified:** 2026-10-12
**Status:** passed
**Re-verification:** No new gaps to close. This report replaces the stale 2026-10-10 report, which was written before the review fixes WR-08..WR-13, the publish-from-main change and the image rebuild. Every criterion was checked again from scratch against the current tree and the current CI record.

## Evidence base

- Source: branch `claude/gsd-discuss-phase-1-mzdw5v` at HEAD `b974caa6a084e577d1830107fe225e426d7a880b`.
- Local: `cargo test --workspace --locked` passes (101 tests: 8 + 2 + 23 + 30 + 12 + 9 + 17, no failures; 32 `tool_01_*`, 21 `tool_02_*`, 44 `tool_03_*`, 2 `tool_04_*` test names listed). `cargo fmt --check` and `cargo clippy --all-targets --locked -- -D warnings` exit 0.
- CI, read directly with `gh api` (no Docker daemon in this session):
  - container push run 38198934488 at defbe68: jobs `mt`, `image-a`, `image-b`, `compare`, `publish` all `success`, run_attempt 1. It published `ghcr.io/sat-wik/migrationtool-toolchain@sha256:0f5a5d2594b0405931ad7e85d5c6d2f5e3f2d5099d769059870817ee50e152ee` (annotations `verified-image`, `image-digest`, `image-archive` on the publish and image-a jobs; archive sha256 c5ce0042..., loaded image id equals the id image-a gated).
  - container push run 38199978785 at 3fa71a1: `mt`, `image-a`, `image-b`, `compare` `success`; `publish` `skipped` (main-only guard).
  - Between defbe68 and HEAD no change touched `container/Dockerfile`, `container/pins.toml`, `container/smoke/` or `check.rs`; the diff is the workflow (publish guard), README, `runner.rs` and two test files. Run 3fa71a1 rebuilt the image from the same Dockerfile and pins and produced the same manifest sha256, so the published digest still corresponds to the tree.
- UAT: `01-UAT.md` records the founder running `mt toolchain check --pins` on Apple Silicon against exactly that digest: every row OK, hayroll and kani not_installed, rustc_bitcode 1.72.1 LLVM 16.0.5, `result: OK`, exit 0. Status complete, 1 passed, 0 issues. Not re-raised as a human item.

## Goal Achievement

### Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | One command in the container prints the exact version of every pinned tool and exits non-zero if any is missing or off its pin | VERIFIED | Run 38199978785 image-a annotation `check-a` (run in the image, no network, unprivileged): rows for clang 16.0.6, llvm_config 16.0.6, klee 3.2, c2rust 0.22.1, bear 3.1.1, arm_gcc 14.3.Rel1, qemu_arm and qemu_system_arm 7.2, cargo_mutants 27.1.0, rustup 1.29.1, rustc_tool 1.99.0 (LLVM 23), rustc_bitcode 1.72.1 LLVM 16.0.5, the apt pins and both Rust target sets, all `OK`; `result: OK`; `exit code: 0`. Annotation `negative-pin` (pins copy with LLVM 17) exits 1 with `MISSING` on clang, llvm_config and the 17-named apt packages, `MISMATCH` on c2rust, klee, rustc_bitcode and llvm_packages. Local: `tool_01_check_end_to_end` (8 tests) and `check_and_manifest` (23) pass, including `tool_01_check_unobserved_failed_and_missing_runs_never_pass`. `mt-cli` `toolchain check` calls `run_check_observed`, which drives every probe through `runner::run`. Hayroll and Kani print `not_installed` with a reason (decision D-10; Hayroll licence R3 gates Phase 8, Kani is v2.0). They are not in the tool list this criterion names. |
| 2 | One LLVM major in 16-19 recorded as the pin for clang, KLEE, c2rust; check fails on a different major; pinned rustc reports LLVM at or below 19, provisional until Phase 6 R1 | VERIFIED | `container/pins.toml` `[llvm]`: `major = 16`, `max_major = 19`, `status = "provisional"`, `decision = "D-15"`. The `check-a` output ends with `llvm pin: 16 (provisional, decision D-15); bitcode rustc must report LLVM 16.x and at most 19; provisional until the Phase 6 R1 verdict`. LLVM column: clang 16, klee 16, c2rust 16, llvm_config 16, rustc_bitcode 16 (16.0.5), llvm_packages 16. `negative-pin` shows `MISMATCH  LLVM major 16 differs from the pinned 17` on klee, c2rust and rustc_bitcode. Local tests `tool_02_pins_llvm_major_in_supported_range_and_provisional`, `tool_02_pins_llvm_major_tokens_are_consistent`, `tool_02_parses_rustc_vv_release_commit_and_llvm` and the other 18 `tool_02_*` tests pass. The tool rustc (LLVM 23) is a separate row, correctly outside the bitcode bound. |
| 3 | Building the image twice yields identical tool-version manifests, and every base image and tool source is pinned by digest, commit or checksum with no floating tags | VERIFIED | **Identical manifests:** annotation `manifest-compare` (run 38199978785, job `compare`): `manifest-a.json and manifest-b.json are byte-identical; sha256 d70029186639d0030fc6cf22b284b3b849ce0680656fecfe06623f6ffdc44142`; the same sha256 is in `manifest-a` and `manifest-b`, and in the earlier run 38198934488. Build B is a separate job. **Pins:** every `FROM` in `container/Dockerfile` (lines 70, 97, 126, 171, 318, 353, 422, 491) is `${BASE_REF}@${BASE_DIGEST}` or a prior stage; `base_digest` is a sha256 in pins.toml; apt by snapshot timestamp plus exact versions; Arm archive, rustup-init, both Rust channel manifests, c2rust and cargo-mutants `.crate` files and the klee-uclibc locale tarball by `sha256sum -c`; KLEE and klee-uclibc by commit with `rev-parse HEAD`. Floating-tag and unchecked-download rules are enforced by the 30 `container_static` tests. **CR-01 stays closed:** the `pin-binding` annotation of image-a shows the build record lines equal to the pins (`ce6dddc8...b6a2` channel-rust-1.99.0, `77113b96...a229` channel-rust-1.72.1, `e331f411...3f3b` c2rust crate, `07072e7b...1cf9` cargo-mutants crate), 11 archive lines `OK`, cargo's ledger `path+file:///tmp/crates/...`, rustup update-hashes equal to the pin prefixes; the install RUNs are `RUN --network=none` (13 such RUN lines in the Dockerfile). **WR-08 closed:** the locale tarball is a pin (`[source.klee_uclibc_locale]`) checked with `sha256sum -c` and the klee-uclibc `make` runs offline; its sha is not part of the pin-binding record or the manifest (IN-11, advisory). **Limit that is true and stated in the README:** the guarantee is manifest identity (versions and pins), not bit-for-bit image identity. |
| 4 | A trivial `no_std` crate builds for `thumbv7em-none-eabihf` in the container; the workspace launches an analyser through the subprocess runner with stdout, stderr, exit status and version stamp captured; a repository check fails on Python outside `research/` | VERIFIED | **Build:** annotation `smoke` (run 38199978785): ran as user 65534, `Finished release`, `arm-none-eabi-gcc ... 14.3.1`, `nm libmt_smoke.a` shows `T mt_smoke_add`, `nm smoke.o` shows `U mt_smoke_add`, `exit code: 0`; local `tool_04_smoke_crate_has_emitted_rust_shape` and `tool_04_smoke_crate_is_outside_the_workspace` pass. **Runner:** `runner::run` is the only `Command::new(` in the workspace (`runner.rs:668`); the 17 `tool_01_runner_*` tests pass, covering argv, cleared environment, exit code and signal, stdout/stderr hashes, output caps, timeouts that kill descendants and report `timed_out`, partial output on incomplete streams, and the run record JSON schema (key set and determinism). Tool stamps and per-tool stdout/stderr are in the `tool-outputs` annotations of image-a (klee, llvm_config, qemu, rustc, rustup). **Python guard:** `tool_01_no_python_outside_research` and `tool_01_python_walk_flags_a_planted_file` pass; `find` for `*.py` outside `research/`, `.claude`, `target` and `.git` returns nothing. Also passing: `tool_01_command_only_in_runner_module`, `tool_01_command_matcher_flags_grouped_imports_aliases_and_globs` (WR-05), `tool_01_never_links_an_analyser_library`, `tool_01_every_tool_crate_forbids_unsafe`. |

**Score:** 4/4 truths verified, 0 present-but-behavior-unverified.

Behavior-dependent truths: the runner kill/cleanup invariants inside criterion 4 are exercised by named tests that pass locally (`tool_01_runner_timeout_kills_a_setsid_descendant`, `..._kills_a_forking_setsid_subtree`, `..._normal_exit_with_a_lingering_descendant_returns_the_record`), so they are not left at PRESENT_BEHAVIOR_UNVERIFIED. The one invariant they do not establish, killing an escapee after a normal exit, is not claimed by the criterion; see the WR-13 advisory.

### Deferred Items

None.

### Advisory (carried, not blocking)

| # | Finding | Category | Why Advisory |
|---|---------|----------|--------------|
| 1 | WR-13 residual: a `setsid` escapee of a tool that already exited normally is not killed; `exit.tree_kill_sent` only records that a kill was sent | other | Documented in the runner header and field docs; not a ROADMAP criterion; close before forking analysers run through the runner |
| 2 | IN-01..IN-21 open | other | Hardening and docs, no criterion contradicted |
| 3 | CI of head b974caa was in progress at verification time | other | Source after the last green container run differs only by a runner field rename and planning files; local gates pass |

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `container/Dockerfile` | Pinned multi-stage image, installs bound to checked files | VERIFIED | 509 lines, built twice in CI at defbe68 and 3fa71a1; no TBD/FIXME/XXX; installs in `RUN --network=none` after checked downloads |
| `container/pins.toml` | Single source of pins | VERIFIED | LLVM 16 / max 19 / provisional D-15; base digest; Rust channel and crate checksums equal the CI build record |
| `container/README.md` | Current image digest and founder guide | VERIFIED | "Current image" names run 38198934488, commit defbe68, digest `0f5a5d25...e152ee` and manifest sha256 `d7002918...4142`, all equal to the CI `verified-image` annotation; the main-only publish rule is stated; no forbidden claim words (grep) |
| `.github/workflows/container.yml` | Double build, gates, compare, main-only publish | VERIFIED | Runs 38198934488 and 3fa71a1 behaved as designed, publish skipped off main |
| `.github/workflows/ci.yml`, `pin-discovery.yml` | CI proofs, pin recomputation | VERIFIED | ci green on 3efc5fd and earlier head commits; pin-discovery green on defbe68 |
| `crates/mt-toolchain/src/{runner,check,pins,version,build_args,manifest,fetch}.rs` | Runner, check, pins | VERIFIED | Wired from `mt-cli`; all tests pass; `#![forbid(unsafe_code)]` in both crates |
| `crates/mt-toolchain/tests/{container_static,repo_checks,tool_01_runner,...}.rs` | Requirement-ID tests | VERIFIED | 101 tests pass |
| `container/smoke/*`, `deny.toml`, `research/.gitkeep` | no_std smoke crate, dependency policy, Python home | VERIFIED | Smoke annotation green; repo checks pass |

### Key Link Verification

| From | To | Via | Status |
|------|----|-----|--------|
| `mt-cli` `toolchain check` | `check::run_check_observed` | main.rs line 150 | WIRED |
| `check.rs` probes | `runner::run` | `run_one` | WIRED |
| `mt-cli` | `runner::run` | main.rs line 468 | WIRED |
| `container.yml` | Dockerfile | `--target final`, args from `mt toolchain build-args` | WIRED (CI green) |
| Dockerfile ARG block | `pins.toml` | `tool_03_dockerfile_args_match_pins` | WIRED |
| Dockerfile installers | `pins.toml` checksums | `sha256sum -c` then file:// mirror or `--path` install | WIRED (pin-binding annotation equals pins) |
| image-a gates | publish | archive sha256 and loaded image id checked, publish guarded by `refs/heads/main` | WIRED |
| `pins.toml` `[llvm]`, `[rust.bitcode]` | PROJECT.md D-15 | decision-token test | WIRED |

### Data-Flow Trace (Level 4)

Not applicable: nothing renders dynamic data. The equivalent trace for pins was done: `pins.toml` to `mt toolchain build-args` to Dockerfile ARG to `sha256sum -c` to installer input to check row to manifest.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Workspace tests | `cargo test --workspace --locked` | 101 passed, 0 failed | PASS |
| Formatting | `cargo fmt --check` | exit 0 | PASS |
| Lints | `cargo clippy --all-targets --locked -- -D warnings` | exit 0 | PASS |
| Python outside research | `find . -name '*.py'` excluding research, .claude, target, .git | no output | PASS |
| Single process launcher | `grep -n "Command::new" -r crates/*/src` | only `runner.rs:668` | PASS |
| Forbidden claim words | grep for the two words in README, pins, crates src, workflows | no match | PASS |

Container behaviours were not re-run (no Docker daemon); they were read from the annotations of run 38199978785 and run 38198934488, and from the founder's UAT on the published digest.

### Probe Execution

Step 7c: SKIPPED. No `probe-*.sh` is declared by any plan or present under `scripts/`.

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| TOOL-01 | 01-01, 01-02, 01-04, 01-05, 01-07, 01-08 | Cargo workspace; analysers run only as pinned subprocesses, never linked; check fails on Python outside `research/` | SATISFIED | `runner::run` is the only launcher; `tool_01_never_links_an_analyser_library`, `tool_01_command_only_in_runner_module`, `tool_01_no_python_outside_research` and the 17 runner tests pass; CI check run exits 0 in the image. Hayroll, Kani are `not_installed` by D-10 (Hayroll licence gate R3, Kani in v2.0). |
| TOOL-02 | 01-03, 01-05, 01-06, 01-07 | One LLVM major 16-19 for clang, KLEE, c2rust, Hayroll; check fails on another major or on bitcode rustc LLVM above 19 | SATISFIED | Pin 16, bound 19, `check-a` all 16, `negative-pin` exit 1 with MISMATCH; 21 `tool_02_*` tests pass. Hayroll is not installed, so it has no row to be on a different major (D-10); its LLVM requirement applies when Phase 8 installs it. |
| TOOL-03 | 01-02, 01-03, 01-05, 01-06, 01-07, 01-08, 01-09 | Image reproducible: base, tool sources and Rust toolchain pinned by digest, commit or checksum; two builds yield an identical manifest | SATISFIED | `manifest-compare` byte-identical (sha256 d7002918...4142); pin-binding record equals pins; 44 `tool_03_*` tests pass. Guarantee is version-level, as stated (IN-11). |
| TOOL-04 | 01-04, 01-07 | arm-none-eabi GCC and the `thumbv7em-none-eabihf` target; a trivial `no_std` crate builds for it in the image | SATISFIED | `smoke` annotation, arm_gcc 14.3.Rel1 row OK, `rust_targets_tool` row contains `thumbv7em-none-eabihf`; 2 `tool_04_*` tests pass. |

All four IDs appear in at least one PLAN's `requirements:` field (01-01 to 01-09) and are the only IDs REQUIREMENTS.md maps to Phase 1 (lines 168-171). No orphaned requirements. Bookkeeping for the orchestrator: REQUIREMENTS.md still shows TOOL-01..04 unchecked (lines 16-19) and `Gaps Found` in the traceability table (lines 168-171), and ROADMAP.md still shows Phase 1 unchecked; these should move to Complete now that this report is `passed`.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| crates/mt-toolchain/src/runner.rs | 693-748 | Post-exit escapee not found (WR-13 residual) | Warning | Flag renamed and documented; gap remains for daemonizing tools |
| crates/mt-toolchain/src/runner.rs | 605-620 | Post-kill wait unbounded (IN-18) | Info | Uninterruptible child would hang `run` |
| container/Dockerfile | 491 onward | Final image runs as root, unpinned runtime packages (IN-08) | Info | Smoke and check ran unprivileged in CI |
| .github/workflows/pin-discovery.yml | 108-113 | Arm checksum not compared (IN-09) | Info | Regression aid only |

No `TBD`, `FIXME` or `XXX` marker in any file the phase modified (the single `FIXME later` is a test fixture string in `pins_and_versions.rs`, line 143). No blocker found.

### Human Verification Required

None. The one human item (Apple Silicon pull and check) already passed on the current digest `sha256:0f5a5d25...e152ee` (01-UAT.md). No new item arose: the changes after that UAT are the WR-13 field rename in the runner (the `mt` binary is mounted, not part of the image) and planning files.

### Gaps Summary

No gaps. All four ROADMAP success criteria are met by the code and by the CI record of the current image: the check passes in the container and fails on an off-pin LLVM, one LLVM major (16, provisional) governs clang, KLEE, c2rust and the bitcode rustc, two builds give a byte-identical manifest with every source pinned and bound to the bytes the installers consume, and the no_std smoke crate builds for the Cortex-M target. The remaining items are hardening, tracked as advisory. Two things to do next outside this phase: re-read the CI result for head b974caa before merging, and close the WR-13 gap before a forking analyser runs through the runner.

---

_Verified: 2026-10-12_
_Verifier: Claude (gsd-verifier)_
