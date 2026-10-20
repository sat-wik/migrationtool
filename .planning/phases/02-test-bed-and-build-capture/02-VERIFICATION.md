---
phase: 02-test-bed-and-build-capture
verified: 2026-10-20T02:40:00Z
status: human_needed
score: 8/8 must-haves verified
covered_files:
  - .github/workflows/testbed.yml
  - .planning/phases/02-test-bed-and-build-capture/02-01-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-01-SUMMARY.md
  - .planning/phases/02-test-bed-and-build-capture/02-02-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-02-SUMMARY.md
  - .planning/phases/02-test-bed-and-build-capture/02-03-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-03-SUMMARY.md
  - .planning/phases/02-test-bed-and-build-capture/02-04-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-04-SUMMARY.md
  - .planning/phases/02-test-bed-and-build-capture/02-05-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-05-SUMMARY.md
  - .planning/phases/02-test-bed-and-build-capture/02-06-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-06-SUMMARY.md
  - .planning/phases/02-test-bed-and-build-capture/02-07-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-07-SUMMARY.md
  - .planning/phases/02-test-bed-and-build-capture/02-08-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-08-SUMMARY.md
  - .planning/phases/02-test-bed-and-build-capture/02-09-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-09-SUMMARY.md
  - .planning/phases/02-test-bed-and-build-capture/02-10-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-10-SUMMARY.md
  - .planning/phases/02-test-bed-and-build-capture/02-11-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-11-SUMMARY.md
  - .planning/phases/02-test-bed-and-build-capture/02-12-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-12-SUMMARY.md
  - .planning/phases/02-test-bed-and-build-capture/02-13-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-13-SUMMARY.md
  - .planning/phases/02-test-bed-and-build-capture/02-14-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-14-SUMMARY.md
  - .planning/phases/02-test-bed-and-build-capture/02-15-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-15-SUMMARY.md
  - .planning/phases/02-test-bed-and-build-capture/02-16-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-16-SUMMARY.md
  - .planning/phases/02-test-bed-and-build-capture/02-17-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-17-SUMMARY.md
  - .planning/phases/02-test-bed-and-build-capture/02-18-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-18-SUMMARY.md
  - .planning/phases/02-test-bed-and-build-capture/02-19-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-19-SUMMARY.md
  - Cargo.lock
  - Cargo.toml
  - container/Dockerfile
  - container/pins.toml
  - crates/mt-capture/src/capture.rs
  - crates/mt-capture/src/cc.rs
  - crates/mt-capture/src/compare.rs
  - crates/mt-capture/src/droplog.rs
  - crates/mt-capture/src/elf.rs
  - crates/mt-capture/src/entries.rs
  - crates/mt-capture/src/fetch.rs
  - crates/mt-capture/src/lib.rs
  - crates/mt-capture/src/manifest.rs
  - crates/mt-capture/src/replay.rs
  - crates/mt-capture/src/report.rs
  - crates/mt-capture/src/seeding.rs
  - crates/mt-capture/src/testbed.rs
  - crates/mt-capture/src/treehash.rs
  - crates/mt-cli/src/main.rs
  - crates/mt-seeds/Cargo.toml
  - crates/mt-seeds/src/hex.rs
  - crates/mt-seeds/src/index.rs
  - crates/mt-seeds/src/lexer.rs
  - crates/mt-seeds/src/lib.rs
  - crates/mt-seeds/src/select.rs
  - crates/mt-seeds/src/zipsrc.rs
  - testbed/DROPPED.md
  - testbed/README.md
  - testbed/dropped.toml
  - testbed/manifest.toml
covered_digest: "v3:sha256:a79d771059fcbe39823145d1a0531acfb818553df3bb0041fa73fe345b203568"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: human_needed
  previous_score: 4/4
  gaps_closed:
    - "G-02-1: isotp reopened (entry probe reproduces the defining TU's include prefix)"
    - "G-02-2: CMakeFiles/Makefile2 hashed in a normalised form instead of excluded; exact is_cmake_product; exclusions listed in the determinism comment"
    - "G-02-3: httpparser, nanopb and qcbor seed sources narrowed to input-bearing arrays and fixtures"
    - "G-02-4: zip on flate2 rust_backend (miniz_oxide) with a per-archive decompressed-size budget"
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "Triage review warnings WR-11 and WR-12 (the record stage's trust of the build record): fix before ship, or accept and log"
    expected: "Either a follow-up plan makes the record stage (a) refuse a record with built:false and no failure cause and make the verdict depend on the gates, and (b) compare stage/toolchain.cmake with the recomputed text; or the founder accepts both as residual risk, registered in 02-SECURITY.md next to AR-02-03"
    why_human: "Both are confirmed in the code (capture.rs capture_record, lines 2019-2060) and both are still open in 02-REVIEW-DISPOSITION.md. Neither contradicts a ROADMAP success criterion or a UAT gap: a forged record needs a malicious pinned upstream, replay refuses the WR-11 case (no canonical compile commands) and the compile-command gates still check the real commands. But the capture.rs module doc says a forged record can make its pair read only as built, failed or not proven. WR-11 shows it can read as captured. Whether to fix now or defer is a founder call."
  - test: "Register or explicitly accept the unregistered SECURITY.md observation: replay passes the recorded GCC driver flags to the pinned gcc"
    expected: "Founder registers it as a medium threat (and a later plan rejects -wrapper, -B<dir>, -specs and plugin options in the compile-command gate) or accepts it, logged as an accepted risk"
    why_human: "Confirmed: cc.rs rejects only a non-allow-listed argv[0], a missing -c, paths outside the run root, and -flto*/-fprofile-*. Driver options that execute helpers would run in the offline replay container, where /work and /out are writable. It needs a hash-pinned malicious upstream. 02-SECURITY.md suggests registration or acceptance and does neither."
  - test: "Confirm accepted risk AR-02-03 (a pair's own build writes its build record) with WR-11 in view"
    expected: "Founder confirms AR-02-03 stands, with its rationale amended to say what a forged record can and cannot do, or asks for the WR-11 fix first"
    why_human: "AR-02-03 says the record can falsify only that pair's steps, flags and before listing. WR-11 shows the forged record can also make the pair's verdict captured with no gate evaluated. The acceptance was given on the narrower statement."
---

# Phase 2: Test Bed and Build Capture Verification Report (re-verification)

**Phase Goal:** A pinned test bed of 10-20 real C parsers can be rebuilt exactly, per shipped configuration, for Cortex-M with GCC.
**Verified:** 2026-10-20T02:40:00Z
**Status:** human_needed
**Re-verification:** Yes, after gap closure (plans 02-11 to 02-19). This file replaces the earlier report of 2026-10-13.

## Goal Achievement

No truth failed. The four ROADMAP success criteria and the four UAT gaps (G-02-1 to G-02-4) are backed by committed data, passing tests and a green full-bed CI run that I read from GitHub, not from the SUMMARYs. Status is `human_needed`, not `passed`, because three founder-judgment items remain open (listed under Human Verification Required). None of them contradicts a success criterion.

Evidence base:
- CI: testbed run 39335185551 on `fb048678e1a6b73cf006cda40f94013703541c49`, event push, conclusion success. Jobs `mt` (118343109529) and `testbed` (118343432265): every step success. `ci` 39335185566 and `container` 39335185555 on the same SHA: success. I read the `testbed` job's check-run annotations with `gh api`.
- Currency: `git diff --stat fb04867 HEAD -- . ':!.planning'` prints nothing. Commits after fb04867 touch only `.planning/`, so the run covers the shipped code. HEAD is f85d319.
- Local gates, run by me: `cargo test --workspace --locked` gives 318 passed, 0 failed (Arm tools, Bear, CMake and clang are installed here, so the capture and entry tests ran; no "skipping" line appears). `cargo fmt --check` clean, `cargo clippy --all-targets --locked -- -D warnings` clean, `cargo deny --locked check` gives advisories, bans, licenses and sources ok.

### Observable Truths

ROADMAP success criteria first (the contract), then the four UAT gap truths.

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SC1: manifest lists 10-20 open-source C targets, each pinned by commit, licence recorded, at least one buffer-in entry | VERIFIED | `mt testbed validate` run by me: "valid (16 targets, 31 configurations)". I compared the CI fetch annotation (16 rows: commit and tree_sha256) with `testbed/manifest.toml`: 16 of 16 equal, 0 mismatches. CI capture reports `found:true,signature:true,header:true` for every entry of every captured pair. |
| 2 | SC2: each target declares shipped configurations; first-RTOS choice recorded as a decision | VERIFIED | 31 configurations over 16 targets (default plus at most one variant). `[decisions] first_rtos = "deferred"` is in the manifest. CI capture shows `changes_code true` for each variant row, so variants are real configurations. Tests `bed_02_*` pass. |
| 3 | SC3: for every target and configuration, capture writes `compile_commands.json` covering every TU and replay reproduces the output (byte-identical or documented normalisation); unreproducible pairs dropped with cause, at least 10 left | VERIFIED | CI capture: 46 rows `passed`, 30 pairs `captured`, 1 pair `paho/server` `dropped (entry_mismatch)`, 15 `changes_code` variant rows. Gates per pair: `compiler_allow_listed=true arch_ok=true undeclared=0 timestamp_macro=0`. Replay: "result: passed (31 rows)", 30 `identical`, 1 `dropped`, every object row `normalisation_applied=false`. Retained: 16 of 16 targets (paho keeps `default`). Drop log: one entry, `testbed/dropped.toml` plus `DROPPED.md` heading, floor of 10 enforced on `--all` (02-16). Determinism: rerun exit 0, `mt testbed compare` "compared 74 files ... result: identical", compare exit 0. |
| 4 | SC4: each target has a seed corpus from its own tests and fixtures, or at least 10 hand-made (proposed) seeds | VERIFIED | `testbed/seeds/<id>/index.json` exists for all 16 targets. I recomputed sha256 of all 1467 seed files against their index rows: 0 bad. Every index commit equals the manifest commit. Counts: cjson 25, corejson 108, coremqtt 15 (10 hand-made), httpparser 76, isotp 19 (hand-made), libfdt 16, lz4 20, mbedtls 200, microcoap 16, nanopb 200, nimble 200, paho 17, qcbor 200, tinycbor 200, zcbor 137, zlib 18; none below 10. CI "Check the committed seeds" step: success, "result: passed (16 rows)". |
| 5 | G-02-1: a target whose header is not self-contained still passes the entry check because the probe reproduces the defining TU's include prefix; isotp is retained | VERIFIED | `entries.rs`: `include_prefix`, `unit_include_prefix`, `probe_source_with_prefix`, `probe_prefix` recorded only when used; falls back to header-first when the prefix cannot be reproduced. Tests `bed_01_entry_probe_uses_the_defining_tu_include_prefix`, `bed_01_entry_self_contained_header_still_passes`, `bed_01_entry_missing_function_still_fails_with_a_prefix` plus the pure prefix tests: 7 of 7 pass here. CI: `isotp/default captured` with `isotp_on_can_message=found:true,signature:true,header:true`, replay `identical`. `dropped.toml` holds only `paho/server`; `isotp` seed corpus committed (19 hand-made). Both `missing` items delivered. |
| 6 | G-02-2: Makefile2 covered by configure-time evidence in a normalised form; exact `is_cmake_product`; determinism comment lists every exclusion | VERIFIED | `capture.rs`: `normalise_make_driver` (sorts each run of one target's prerequisite lines, everything else in place), `MAKE_DRIVER_RULE`, `normalised_configure_files` in the report, `configure_evidence_changed` re-checks it, `is_cmake_product` matches only the documented paths (no suffix test). `testbed.yml` determinism comment (lines 462-476) lists every excluded field and file and the normalised file, held to the constants by `build_02_determinism_comment_lists_every_exclusion`. Tests in `build_02_configure_evidence.rs` pass, including `build_02_real_makefile2_variants_normalise_equal` on the real zcbor fixtures. CI capture annotation "normalised configure files (G-02-2)": every CMake pair shows `normalised build/CMakeFiles/Makefile2 rule=... sha256=...`. The determinism step compared them identical. All three `missing` items delivered. |
| 7 | G-02-3: extracted c_string seeds for httpparser, nanopb and qcbor come only from declared test arrays or fixture files that hold parser inputs | VERIFIED | `manifest.toml`: httpparser `test.c` c_string restricted by `designators = ["raw"]`; qcbor four files, `c_array` only, no string source; nanopb `sample_data` file_copy plus `tests/fuzztest` zip_member, no tests/ string source. Committed index methods: httpparser 76 c_string, qcbor 200 c_array, nanopb 198 zip_member plus 2 file_copy. The corpus shrank from 200 to 76 for httpparser. Tests `build_03_seed_narrowing.rs` and `build_03_seed_designators.rs` pass. CI seeds check passed. Residual: the qcbor manifest comment records that `qcbor_encode_tests.c` also contributes "a few small payload blobs" that are not CBOR, kept by the plan. Both `missing` items delivered. |
| 8 | G-02-4: zip seed sources decompressed by an all-safe-Rust inflater under a total decompressed-size budget | VERIFIED | `Cargo.toml`: `zip = { =9.0.2, default-features = false, features = ["deflate-flate2"] }` plus `flate2 { =1.1.10, features = ["rust_backend"] }`. `Cargo.lock`: miniz_oxide 0.9.1; no zlib-rs or libz-* (test `build_03_zip_inflates_with_miniz_oxide_only` checks the ban list). `zipsrc.rs::zip_members(bytes, max_member, max_total)` with a bounded read; `seeding.rs:385` passes `MAX_ZIP_DECOMPRESSED_BYTES` (8 MiB). Tests `build_03_zip_archive_has_a_total_decompressed_budget`, `_budget_boundary_is_exact`, `_budget_of_zero_reads_nothing`: pass. `cargo deny` clean. Caveat, disclosed in `zipsrc.rs` and 02-SECURITY.md: the inflater and adler2 forbid unsafe, but simd-adler32, zip's header parsing and flate2's buffer code contain unsafe, so the whole reading stack is not safe Rust. The founder approved that wording at the 02-13 checkpoint. |

**Score:** 8/8 truths verified (0 present, behavior-unverified).

The plan-level truths of 02-18 and 02-19 (WR-04: build stage and record stage split; one container per pair; limits on every container; isolated determinism rerun) are verified as follows. `capture.rs` has `capture_build` and `capture_record`; `testbed.yml` `run_build` mounts only a scratch `/work`, the cache read-only and that pair's run directory, with `--network none`, `--pids-limit 1024 --memory 6g --memory-swap 6g --cpus 2` on all four helpers; the record step runs through `run_offline`; the determinism rerun uses `WORK_HOST=work-rerun` and `/out/capture-rerun`. The static test `build_02_untrusted_builds_cannot_write_the_evidence` passes and the run was green. The residual gaps in this boundary are WR-11 and WR-12 below.

### UAT Gap Reconciliation (02-UAT.md not edited)

| Gap | UAT test | `missing` items delivered | Plans carrying `gap_ids` | Proof |
|-----|----------|---------------------------|--------------------------|-------|
| G-02-1 | 1 (isotp) | 2 of 2 | 02-11, 02-17, 02-19 | Code, 7 tests, CI run 39335185551 (isotp captured, identical) |
| G-02-2 | 2 (Makefile2) | 3 of 3 | 02-12, 02-17, 02-19 | Code, 4 tests in `build_02_configure_evidence.rs` and the static comment test, CI normalised hashes and identical determinism |
| G-02-3 | 4 (seed sources) | 2 of 2 | 02-15, 02-17, 02-19 | Manifest diff 84e5165, 2 test files, CI seeds check passed (corpora regenerated in the image, no drift) |
| G-02-4 | 5 (zip backend) | 2 of 2 | 02-13, 02-17, 02-19 | Cargo.toml and lock, 5 tests in `build_03_zip_backend.rs`, cargo deny clean |

Test 3 (hand-made seeds) passed with the founder's note to review the seeds once the Phase 3 harness shows which inputs exercise real code paths. That is a future review, not a Phase 2 gap.

### Deferred Items

None are deferred against later phases. The founder deferred the WR-07 member-count cap (02-13-SUMMARY, ledger entry `deferred`); Phase 3 and later do not take it over, and it is not a success-criterion gap.

### Advisory (New Scope, Unevidenced)

Re-verification advisory section. No unevidenced new-scope finding was raised. Observations that are not blockers:

| # | Finding | Category | Why Advisory |
|---|---------|----------|--------------|
| 1 | `testbed/DROPPED.md` ("Reopened" section) and `testbed/README.md` line 162 still say the isotp reopening is unproven until the next testbed run. Runs 39331139047 and 39335185551 have proven it (02-SECURITY.md lists the same lag). | other | Documentation text only, no behavior |
| 2 | `.planning/ROADMAP.md` line 30 still shows "- [ ] **Phase 2**" while the plan list shows 19 of 19 done. | other | Orchestrator state, set on phase completion |
| 3 | Open review items WR-01, WR-03, WR-06, WR-09 and IN-01 to IN-06, IN-08 to IN-12 (0 critical). None changed in the gap closure and none contradicts a success criterion. | other | Tracked in 02-REVIEW-DISPOSITION.md, triage pending |

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `testbed/manifest.toml` | 10-20 pinned targets, configs, RTOS decision | VERIFIED | 16 targets, 31 configs, validates, pins equal the CI fetch hashes |
| `testbed/dropped.toml`, `testbed/DROPPED.md` | Drop log with causes | VERIFIED | 1 entry (`paho/server`), headings match |
| `testbed/seeds/*` | Seed corpora | VERIFIED | 16 corpora, 1467 hashes recomputed, 0 bad |
| `crates/mt-capture/src/{capture,compare,entries,testbed,replay,seeding,...}.rs` | Two-stage capture, normalised configure evidence, include-prefix probe, fail-closed compare | VERIFIED | Substantive, wired, exercised in CI; no `todo!`/`unwrap`/`expect` in source (re-review confirms) |
| `crates/mt-seeds/src/zipsrc.rs` | Budgeted, safe-inflater zip reads | VERIFIED | Wired from `seeding.rs:385` |
| `.github/workflows/testbed.yml` | Per-pair build, record, replay, compare, limits | VERIFIED | Green on fb04867, steps present |
| `container/Dockerfile`, `container/pins.toml` | Image with git, cmake, curl, bzip2, make | VERIFIED | Unchanged since the first verification; `container` workflow green on fb04867 |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| manifest pins | fetched trees | tree hash in `fetch` | WIRED | 16 of 16 CI hashes equal the manifest |
| `capture --stage build` (container per pair) | `capture --stage record` | `stage/build-record.json` bound to target, config and verified source | WIRED | CI build step: 30 pairs built; record step passed. See WR-11 and WR-12 for the two gaps in what the record stage re-checks. |
| capture.json plus compile commands | replay | `report_mismatch` (pin, source, tools) | WIRED | 31 replay rows, 30 identical |
| first capture | second capture | `mt testbed compare` | WIRED | 74 files identical, fail-closed |
| manifest seed sources | committed corpora | `mt testbed seeds --check` | WIRED | CI step success |
| zip reader | budget | `MAX_ZIP_DECOMPRESSED_BYTES` in `seeding.rs` | WIRED | Tested at the boundary |

### Data-Flow Trace (Level 4)

Not applicable: no rendered dynamic data. The data path is manifest to fetch to build to record to replay to compare, and the CI annotations show real values at each step (real commits, 113 objects for Mbed TLS, 65 for NimBLE, real Makefile2 hashes).

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Workspace tests | `cargo test --workspace --locked` | 318 passed, 0 failed, 0 skipped | PASS |
| Format, lints, deps | `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo deny --locked check` | all clean | PASS |
| Committed manifest validates | `mt testbed validate --manifest testbed/manifest.toml` | valid (16 targets, 31 configurations) | PASS |
| G-02-1 tests | `cargo test -p mt-capture --test bed_01_entry_include_prefix` | 7 passed | PASS |
| G-02-4 tests | `cargo test -p mt-seeds --locked` | zip backend 5 passed | PASS |
| Seed integrity | Python recomputation of all seed sha256 and commits | 1467 rows, 0 bad, 0 commit mismatch | PASS |
| Fetch pins | CI `testbed-fetch` vs manifest | 16 of 16 equal | PASS |
| Real Arm build and replay | job 118343432265 annotations | success, 30 identical, determinism identical | PASS (CI only; no Docker in this session) |

### Probe Execution

No `scripts/*/tests/probe-*.sh` exists and none is declared by any plan: SKIPPED.

### Requirements Coverage

Every requirement ID in the 19 PLAN frontmatters is in REQUIREMENTS.md, is mapped to Phase 2 in its traceability table, and is checked complete. No orphaned requirement: all five IDs of the ROADMAP Phase 2 line appear in at least one plan.

| Requirement | Source Plans | Description | Status | Evidence |
|-------------|--------------|-------------|--------|----------|
| BED-01 | 02-01, 02-03, 02-05, 02-06, 02-09, 02-10, 02-11, 02-17, 02-19 | 10-20 targets pinned by commit, licence recorded, buffer-in entry per target | SATISFIED | Truths 1 and 5; 16 targets, 16 of 16 hashes equal, entries found in every captured pair |
| BED-02 | 02-03, 02-05, 02-06, 02-09, 02-19 | Declared configurations; first-RTOS decision recorded | SATISFIED | Truth 2; 31 configurations, decision `deferred` in the manifest |
| BUILD-01 | 02-01, 02-02, 02-04, 02-05, 02-06, 02-09, 02-10, 02-12, 02-17, 02-18, 02-19 | Capture every compile command to `compile_commands.json` (Bear for Make, CMake export) | SATISFIED | Truth 3 and 6; 30 captured pairs, `undeclared=0` for each |
| BUILD-02 | 02-02, 02-04, 02-06, 02-09, 02-10, 02-11, 02-12, 02-14, 02-16, 02-17, 02-18, 02-19 | Replay reproduces the build output; unreproducible pairs dropped with cause, at least 10 kept | SATISFIED | Truth 3; 30 identical, 1 dropped with cause, 16 retained, determinism identical. Hardening caveats: WR-11, WR-12, driver-flag observation |
| BUILD-03 | 02-07, 02-08, 02-09, 02-13, 02-15, 02-17, 02-19 | Per-target seed corpus from tests and fixtures, or at least 10 hand-made proposed seeds | SATISFIED | Truths 4, 7 and 8; 16 corpora, none under 10 |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/mt-capture/src/capture.rs` | 2019-2060 | A build record with `built:false` and no cause yields verdict `captured`; no gate runs (WR-11, confirmed by reading) | Warning | Replay refuses such a pair, so no false replay pass; the capture-step verdict is wrong for a forged record |
| `crates/mt-capture/src/capture.rs` | 2027-2037 | Toolchain-file hash recomputed, `stage/toolchain.cmake` never read back (WR-12, confirmed by reading) | Warning | Report states a hash for a file it did not verify; the real compile commands are still gated |
| `crates/mt-capture/src/cc.rs` | 265, 289-300 | Compile-command gate does not reject `-wrapper`, `-B`, `-specs` or plugin options (confirmed by grep) | Warning | Needs a pinned malicious upstream; unregistered in 02-SECURITY.md |
| TBD/FIXME/XXX markers | n/a | None found in the files this phase modified | none | Debt-marker gate clear |

Counts from 02-REVIEW.md (incremental re-review): 0 critical, 7 warning open (WR-01, WR-03, WR-06, WR-07 deferred, WR-09, WR-11, WR-12), 11 info open; 6 fixed (WR-02, WR-04, WR-05, WR-08, WR-10, IN-07). 02-SECURITY.md: 67 threats, 0 open, three accepted risks (AR-02-01 to AR-02-03).

### Human Verification Required

#### 1. Triage WR-11 and WR-12 (the record stage's trust of the build record)

**Test:** Read the two findings in 02-REVIEW.md and decide fix now (a short follow-up plan; the fixes are a few lines each with named tests) or accept and register.
**Expected:** Either the record stage refuses a `built:false` record that names no failure and the verdict depends on the gates, and it compares `stage/toolchain.cmake` with the recomputed text; or both are registered as accepted residual risk.
**Why human:** Not a success-criterion failure and not exploitable without a malicious pinned upstream, but they weaken a stated invariant of the capture.rs module doc. Whether that is a ship-blocker is the founder's call. If the founder wants them fixed, they become gaps for `/gsd-plan-phase --gaps`.

#### 2. Register or accept the replay driver-flag observation

**Test:** Decide on the unregistered item in 02-SECURITY.md (replay passes recorded driver options to gcc).
**Expected:** Registered as a medium threat with a later gate, or accepted and logged.
**Why human:** Risk call. It needs a hash-pinned malicious upstream and runs in an offline container, but `/work` and `/out` are writable there.

#### 3. Confirm AR-02-03 in the light of WR-11

**Test:** Check whether AR-02-03's rationale ("can falsify only that pair's steps, flags and `before` listing") still stands.
**Expected:** Confirm with an amended rationale, or fix WR-11 first.
**Why human:** The acceptance was given on a narrower statement than the code supports.

### Gaps Summary

No gaps. All four ROADMAP success criteria hold on the current code (CI run 39335185551 on fb04867, nothing outside `.planning/` changed since), and all four UAT gaps G-02-1 to G-02-4 have their `missing` items delivered with code, tests and CI evidence. Remaining work is judgment on three trust-boundary hardening items (WR-11, WR-12, driver flags, AR-02-03), plus the stale docs and open review items noted as advisory. `/gsd-verify-work` can reconcile G-02-1 to G-02-4 through the `gap_ids` in plans 02-11, 02-12, 02-13, 02-15, 02-17 and 02-19.

---

_Verified: 2026-10-20T02:40:00Z_
_Verifier: Claude (gsd-verifier)_
