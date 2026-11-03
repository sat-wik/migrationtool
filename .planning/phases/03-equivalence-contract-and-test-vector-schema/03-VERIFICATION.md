---
phase: 03-equivalence-contract-and-test-vector-schema
verified: 2026-11-03T06:00:00Z
status: gaps_found
score: 3/4 must-haves verified
covered_files:
  - ".github/workflows/testbed.yml"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-01-PLAN.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-01-SUMMARY.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-02-PLAN.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-02-SUMMARY.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-03-PLAN.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-03-SUMMARY.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-04-PLAN.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-04-SUMMARY.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-05-PLAN.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-05-SUMMARY.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-06-PLAN.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-06-SUMMARY.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-07-PLAN.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-07-SUMMARY.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-08-PLAN.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-08-SUMMARY.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-09-PLAN.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-09-SUMMARY.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-10-PLAN.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-10-SUMMARY.md"
  - "container/Dockerfile"
  - "crates/mt-capture/src/manifest.rs"
  - "crates/mt-cli/src/main.rs"
  - "crates/mt-contract/src/policy.rs"
  - "crates/mt-harness/src/adapter.rs"
  - "crates/mt-harness/src/driver.rs"
  - "crates/mt-harness/src/replay.rs"
  - "crates/mt-harness/src/run.rs"
  - "crates/mt-harness/src/sample.rs"
  - "crates/mt-vectors/src/state.rs"
  - "crates/mt-vectors/src/vector.rs"
  - "docs/specs/equivalence-contract.md"
  - "testbed/adapter-dropped.toml"
  - "testbed/manifest.toml"
covered_digest: "v3:sha256:f5b54dc55d0c4e9dcd99464e9c10f905ad678c2932a3f0eb760616338ea1be40"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: passed
  previous_score: 4/4
  gaps_closed: []
  gaps_remaining: []
  regressions:
    - "Truth 4 CI evidence: the vectors job, green on 5e0d4a8, is red at HEAD since the WR-04 review fix (8bdbd61). Its build, run, determinism and committed-sample replay steps have not run since."
gaps:
  - truth: "The C reference executor runs any vector against the target's original C build, stores the result in a schema-valid vector, and running it twice gives byte-identical results (demonstrated in the pinned image by the testbed workflow)"
    status: partial
    reason: "The executor itself works at HEAD (verified locally under qemu-arm 8.2.2 with arm-none-eabi-gcc 13.2.1: 556 default tests and 23 ignored tests pass). But the CI gate that proves it in the pinned image is broken. Fix commit 8bdbd61 (WR-04) added a safe_untar check to the vectors job whose symlink regex rejects every relative symlink containing a `..` component, including in-tree ones such as nimble's `npl_sycfg.h -> ../syscfg/syscfg.h` and nanopb's `pb.h -> ../../pb.h`. The step 'Unpack the captures of the testbed job' fails on GitHub run 41275770544 (head a016909, workflow file identical to HEAD) and on the earlier run 41265174150. Steps 'Build the reference drivers', 'Publish the state layouts', 'Run the reference vectors twice', 'vectors determinism', the sample steps and 'Replay the committed sample' are all skipped, so no in-image run exists for the code at HEAD. This includes the provenance comparison that WR-01 (90767fd) added to the replay: it has run only in offline unit tests, never against the committed sample in the image. Neither REVIEW-FIX nor the re-review mentions the red run (both say CI-only paths were not run)."
    artifacts:
      - path: ".github/workflows/testbed.yml"
        issue: "safe_untar (lines ~1039-1050): the symlink guard `grep -E '^l.* -> (/|(.*/)?\\.\\.(/|$))'` matches any target with a `..` component, so legitimate in-tree relative links in the capture tar are refused. I replayed the regex on two listing lines copied from the CI log and it matches both."
      - path: "crates/mt-harness/tests/eqv_04_workflow_static.rs"
        issue: "The test only checks that the text `safe_untar()` is present and used for both tars. It never feeds the guard a listing with a legitimate relative symlink, so it passed while the job failed."
    missing:
      - "Make safe_untar refuse only a symlink that is absolute or whose target, resolved against the member's own directory, leaves the tree (for example normalise `dirname(member)/target` and test the result for a leading `..`), instead of any `..` component"
      - "Add a behavioural test of the guard: an in-tree link such as `a/b/c.h -> ../x.h` and `a/b/c.h -> ../../x.h` accepted, `a -> /etc/passwd` and `a/b -> ../../../x` refused"
      - "Push and obtain a green `vectors` job at the new head, so that build, run, determinism and the replay of testbed/vectors-sample (with the WR-01 checks) run in the pinned image again"
deferred: []
human_verification:
  - test: "Re-read the three passages of docs/specs/equivalence-contract.md that changed after the founder's UAT item 1 (commit dd1deeb, 2026-11-01): the `rc` row, the `ret` bullet under Library state, and the Byte codec paragraph"
    expected: "The new wording matches the decisions it restates (D-05 'rc: the C return value', D-06, D-25 Latin-1 identity). Specifically: (a) a struct or union return goes to lib_state_out.ret with rc 0; (b) an integer or enum return wider than 32 bits is now refused when the adapter is built (before this it was promised to go to `ret`, but the code never did that and truncated it before CR-01); (c) the codec paragraph now names serde_json escapes (short escapes for \\b \\t \\n \\f \\r, \\u00XX for other control characters, other characters as UTF-8) and says the vector file is serde_json::to_vec_pretty output with sorted maps and one trailing newline"
    why_human: "UAT item 1 (pass, 2026-10-30) judged the earlier text. The document changed afterwards, still under version 0.1.0 with no changelog line (review IN-11). The change narrows a promise rather than adding a rule, and no committed vector or policy depends on the withdrawn promise (checked: testbed/policy and testbed/vectors-sample are unchanged since the UAT). Whether a correction without a version bump is acceptable is a founder call."
behavior_unverified_items: []
---

# Phase 3: Equivalence Contract and Test-Vector Schema Verification Report

**Phase Goal:** "Equivalent" is written down and machine-checkable, and the original C can be run against any test vector to produce a deterministic reference result.
**Verified:** 2026-11-03
**Status:** gaps_found
**Re-verification:** Yes. The report of 2026-10-30 (status passed) is stale: commits 6c58482..90767fd changed 25 non-planning files after the UAT of 2026-10-30. Everything below was checked against HEAD (286baca).

The contract, the schema, the adapters and the executor are all present and work at HEAD. One regression stops the phase from passing: a review fix broke the CI job that proves the executor and the committed sample in the pinned image, and nobody noticed because both fix reports say the CI paths were not run.

## What changed since the last verification

`git diff e23098d..HEAD` (the commit that closed the UAT) touches 25 files outside `.planning`. Unchanged since the UAT: `crates/mt-harness/src/runtime.c`, `classify.rs`, `globals.rs`, everything under `testbed/` (policies, manifest, adapter log, the 15-corpus sample, tractor pin) and the container pins. Changed: the CI workflow (WR-04, WR-05), `Dockerfile` (IN-05, `USER 65534:65534` as last instruction), `mt-capture` manifest id/config grammar (WR-08), `policy.rs` (WR-07 error_code range, IN-07, IN-08), `adapter.rs` (IN-06), `driver.rs` (WR-06, IN-04, IN-09), `run.rs` and `main.rs` (CR-01, WR-02), `replay.rs` (WR-01), `sample.rs` and `state.rs` (IN-03), and `docs/specs/equivalence-contract.md` (IN-02).

## Goal Achievement

### Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Contract v0 exists as a versioned document and a policy file the harness loads: refinement, observation point at the C interface, UB menu {return error, panic, preserve} with return error as default and a recorded setting to change it | VERIFIED | `docs/specs/equivalence-contract.md` at 0.1.0 with changelog. `mt vectors policy-check --manifest testbed/manifest.toml` exit 0, 16 of 16 lines read `valid default=return_error overrides=0`. Tests `eqv_01_rejects_unknown_key`, `eqv_01_default_change_required_to_change_default` and the rest of the `eqv_01_*` group pass in my run. The post-UAT policy changes (WR-07 range check to `i32::MIN..=u32::MAX`, IN-07 non-UTF-8 file name reported) only refuse more; new test `eqv_01_load_dir_reports_a_non_utf8_policy_file` passes. |
| 2 | A schema with exactly the nine fields validates and round-trips in Rust types; a merged `lib_state` is rejected; at least 3 TRACTOR public-corpus vectors load unchanged | VERIFIED | Tampered copy of a committed sample vector with `lib_state_in` renamed `lib_state`: `mt vectors validate` exit 2, "unknown field `lib_state`, expected one of `argv`, `stdin`, `env`, `stdout`, `stderr`, `rc`, `lib_state_in`, `lib_state_out`, `has_ub`". Adding `"has_ub": false`: exit 2 ("invalid type: boolean `false`, expected a string"). All 15 committed corpora pass `mt vectors validate <dir>` ("valid corpus 1 vectors"). Ignored test `eqv_02_tractor_vectors_load_unchanged` passed against the local TRACTOR tree at the pinned commit (this closes the gap the old report left, where only CI had run it). `eqv_02_rejects_merged_lib_state`, `eqv_02_tractor_pin_has_no_content`, `eqv_02_has_ub_vectors_are_excluded_from_comparison`, and the new `eqv_02_serde_json_map_keeps_keys_sorted` pass. |
| 3 | At least 10 test-bed targets have an entry-point adapter declaring the function and its argument roles | VERIFIED | `mt vectors adapters --manifest testbed/manifest.toml --check-floor`: exit 0, "drivable targets: 15 (floor 10)", "result: passed (27 rows)". `eqv_03_adapter_floor_is_ten` passes. The manifest grammar tightening (WR-08: ids and config names may not start with `_` or `-`) did not drop any target. `testbed/` is unchanged since the UAT. |
| 4 | The C reference executor runs any vector against the target's original C build, stores the result in a schema-valid vector, and running it twice gives byte-identical results | PARTIAL (gap) | Code and local behaviour at HEAD: VERIFIED. `cargo test --workspace --locked`: 556 passed, 0 failed, 23 ignored. With `MT_LOCAL_CAPTURES` and `MT_LOCAL_TRACTOR` set, `cargo test --workspace --locked -- --ignored`: 23 passed, 0 failed (includes `eqv_04_run_twice_is_byte_identical`, `eqv_04_replay_reproduces_stored_vectors`, `eqv_04_sample_is_chosen_and_replays_from_a_tree`, `eqv_04_state_is_recorded_field_by_field`, `eqv_04_setup_failure_is_a_compared_outcome`, `eqv_04_sentinel_prefill_recorded`). The qemu-arm selftest cases (`eqv_04_overrun_is_a_fault`, `eqv_04_slack_write_is_detected`, abort, timeout) ran for real on this host in the default run. In-image evidence: NOT reproduced at HEAD. See the gap below and the CI table. |

**Score:** 3/4 truths verified (0 behavior-unverified).

### CI evidence at HEAD (GitHub, read with `gh`)

The last `testbed` workflow run on this branch is 41275770544 (head a016909, 2026-11-01). `git diff a016909 HEAD -- . ':!.planning'` is empty, so it is the run for the code at HEAD. Job results:

| Job | Result | Notes |
|-----|--------|-------|
| `mt` | success | static build |
| `testbed` | success | capture and replay of the 16 pairs |
| `vectors` | FAILURE | steps up to 'Download the captures of the testbed job' succeed, including 'Run the executor self-test in the pinned image' and the TRACTOR load; 'Unpack the captures of the testbed job' fails; the next eight steps are skipped |

Log line of the failure: `##[error]/home/runner/work/_temp/handoff/work.tar holds a symlink that is absolute or leaves the tree`, printed after a listing of `nobody/nogroup` symlinks such as `run/nanopb/default/src/spm_headers/nanopb/pb.h -> ../../pb.h`. The earlier run 41265174150 (after the first fix pass) failed the same way. Run 40922236736 (green on 5e0d4a8) is the last in-image run of build, run, determinism and the sample replay, and it predates every fix that followed the UAT.

What the in-image selftest at HEAD does show: qemu-arm in the pinned image still starts `cortex-m4`, and the outcome cases (including `beyond-reserve` as `executor_error`) pass. UAT item 8 (QEMU 10.0.2 accepted) is therefore not disturbed.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `docs/specs/equivalence-contract.md` | Versioned contract, changelog | VERIFIED (see human item) | 0.1.0, one changelog line; edited after the UAT without an erratum (review IN-11, open). No forbidden claim words. |
| `crates/mt-contract/src/{lib,policy}.rs` | Policy loader | VERIFIED | `#![forbid(unsafe_code)]`; no `process::Command`. |
| `crates/mt-vectors/src/*` | Schema, codec, state, compare, corpus, TRACTOR | VERIFIED | Only comment and test changes since the UAT. |
| `crates/mt-harness/src/*` | Adapter, driver, executor, replay, sample | VERIFIED locally | `resolved_return_kind` (CR-01) refuses an integer or enum return over 4 bytes by DWARF size; `eqv_04_return_width` passes. `provenance_mismatches` and `check_corpus_location` (WR-01) exist, are used by `replay_corpus`, and `eqv_04_replay_provenance_accepts_every_committed_vector` passes over the 15 committed vectors. |
| `testbed/policy/*.toml`, `testbed/manifest.toml`, `testbed/adapter-dropped.toml`, `testbed/tractor-pin.toml`, `testbed/vectors-sample/` | Policies, roles, drop log, pin, sample | VERIFIED | Unchanged since the UAT; 16 policies valid; 15 corpora valid. |
| `.github/workflows/testbed.yml` | `testbed` and `vectors` jobs both green | FAILED | `vectors` job red at HEAD (gap above). |
| `container/Dockerfile` | Final stage unprivileged | VERIFIED by test only | `USER 65534:65534` is the last instruction; `tool_03_final_stage_runs_as_the_unprivileged_user` passes. The image was not rebuilt after the change; the `container` workflow succeeded on run 41275770661, the same head. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| manifest roles | generated driver | `mt vectors build` | WIRED (local) | `eqv_04_exec` tests build and run drivers. Not exercised in the image at HEAD (job skipped). |
| captured objects | driver link | sha256 recheck | WIRED | `eqv_04_refuses_modified_object` passes. |
| driver | vector files + provenance | `mt vectors run` / `replay` | WIRED (local) | Replay now also compares seed, scalars, executor and contract versions, budgets, policy hash and the record shape, and checks the corpus location. In-image replay of the sample not run at HEAD. |
| policy | run | policy loaded first | WIRED | `policy_of` is `pub(crate)` and shared by run and replay. |
| TRACTOR pin | local loader | `mt vectors tractor` | WIRED | Local ignored test and CI step 'Fetch the TRACTOR sample' both succeed at HEAD. |
| testbed job output | vectors job | `safe_untar` of work.tar / replay.tar | NOT_WIRED | The guard rejects the tar, so the hand-off fails. |

### Data-Flow Trace (Level 4)

| Artifact | Data | Source | Produces Real Data | Status |
|----------|------|--------|--------------------|--------|
| committed sample vectors | `rc`, `stdout`, `lib_state_out` | qemu-arm run of captured object code | Yes (unchanged since the UAT; locally re-run by `eqv_04_sample_is_chosen_and_replays_from_a_tree` on the host toolchain) | FLOWING |
| `mt vectors build|run` row counts | did-work count | classified rows | Yes; WR-02 makes an all-`no_roles` listing exit 1, `eqv_04_listing_of_only_no_roles_rows_is_a_failure` passes | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Workspace tests | `cargo test --workspace --locked` | 556 passed, 0 failed, 23 ignored | PASS |
| Ignored proofs with local captures, TRACTOR, qemu-arm | `MT_LOCAL_CAPTURES=... MT_LOCAL_TRACTOR=... cargo test --workspace --locked -- --ignored` | 23 passed, 0 failed (host toolchain, not the pinned image) | PASS |
| Policies | `mt vectors policy-check --manifest testbed/manifest.toml` | exit 0, 16 valid | PASS |
| Floor | `mt vectors adapters --manifest testbed/manifest.toml --check-floor` | exit 0, 15 drivable | PASS |
| Sample valid | `mt vectors validate testbed/vectors-sample/*/default/*` | 15 of 15 valid corpus | PASS |
| Merged `lib_state` | `mt vectors validate` on a tampered copy | exit 2 | PASS |
| `has_ub: false` | `mt vectors validate` on a tampered copy | exit 2 | PASS |
| WR-04 guard on legitimate symlinks | the guard's grep on two listing lines from the CI log | both match (would be refused) | FAIL |

Not run: `cargo clippy`, `cargo fmt --check`, `cargo deny check` (the fix report says it ran them; not repeated here).

### Probe Execution

SKIPPED. Phase 3 declares no `probe-*.sh` scripts; the in-image probe is `mt vectors selftest`, whose last CI result is above.

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| EQV-01 | 03-02 | Versioned contract and policy | SATISFIED | Truth 1 |
| EQV-02 | 03-03 | Nine-field schema, round trip, merged `lib_state` rejected, `has_ub` excluded | SATISFIED | Truth 2 |
| EQV-03 | 03-04, 03-05, 03-06, 03-07 | Adapter declares entry and roles; at least 10 targets drivable | SATISFIED | Truth 3 (15) |
| EQV-04 | 03-01, 03-04..03-07, 03-09, 03-10 | C reference executor, schema-valid result, identical on a second run | PARTIALLY SATISFIED | Satisfied by the code and by local runs at HEAD. The pinned-image demonstration is red at HEAD (gap). |

Plan frontmatter `requirements:` was read from all ten plans. IDs found: EQV-04 (03-01, 03-09, 03-10), EQV-01 (03-02), EQV-02 (03-03), EQV-03 and EQV-04 (03-04 to 03-07). 03-08 has no `requirements:` line in the first 60 lines; its work belongs to EQV-03 and EQV-04 per its summary. All four IDs appear in REQUIREMENTS.md (marked Complete) and nothing else maps to Phase 3, so there are no orphaned requirements. REQUIREMENTS.md marks EQV-04 Complete; that mark should wait for the CI fix.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `.github/workflows/testbed.yml` | ~1039-1050 | Over-broad symlink guard (`..` anywhere in the target) | BLOCKER | Breaks the vectors job (the gap). |
| `crates/mt-harness/tests/eqv_04_workflow_static.rs` | 1057-1074 | Test checks for the presence of `safe_untar` text only | WARNING | A presence check passed while the guard was wrong; the behaviour is untested. |
| `crates/mt-harness/src/runtime.c` | 257-262, 292-296 | WR-03: `struct mt_ksigaction` mask at offset 16 instead of 12, so bytes 12-15 are uninitialised stack and become part of the blocked-signal mask | WARNING | Open and skipped (disposition: skipped). It sits in the runtime whose purpose is deterministic replay. No effect has been observed (identical outputs in 100-run and twice-run checks, and the handler only exits), but the value is unspecified. Fixing it moves every driver hash in the sample and needs a CI regeneration. |
| `crates/mt-harness/src/runtime.c` | 108, 129, 177, 737, 787 | IN-01: `MAP_FIXED` without a collision check; no assert on the input mapping | INFO | Deferred. Unreachable at the default 65536-byte input budget. |
| `crates/mt-harness/src/replay.rs` | 190-203, 437-453 | IN-10: `discarded_output_bytes` and the exact `has_ub_source` variant are not compared, and the doc comment's reason is wrong | INFO | Open. |
| `docs/specs/equivalence-contract.md` | 14, 64, 149-151 | IN-11, IN-12: edit under 0.1.0 with no erratum; `\u00XX` hex case not stated (serde_json writes lowercase) | INFO | Open. IN-12 means an independent reader could compute different file names for a vector holding a control character in U+000A..U+001F other than the short escapes. |
| TBD/FIXME/XXX/TODO, `todo!`, `unimplemented!` | n/a | None in the changed Rust files; no `process::Command` in `mt-contract`, `mt-vectors`, `mt-harness`; `forbid(unsafe_code)` present in the three crates | none | Debt-marker gate passes. |

No use of the two forbidden claim words in the contract or the three crates' sources.

### Human Verification Required

UAT 03-UAT.md (8 of 8 pass, 2026-10-30) is not re-asked. Checked against the diff, seven of the eight judged artifacts are unchanged since the UAT:

- Items 2 and 3 (D-07 overshoot, `name.N` globals keys): `runtime.c` and the globals code are unchanged; the key grammar is unchanged.
- Item 4 (03-05 edit of its own verify command): a historical edit; nothing to re-judge.
- Item 5 (isotp `link_unresolved`) and item 6 (smallest-seed sample): `testbed/` is unchanged, 15 corpora and the drop log are byte-identical.
- Item 7 (`timeout` label, rc -9): `classify.rs` and the contract's outcome table are unchanged.
- Item 8 (QEMU 10.0.2): the image's qemu pin is unchanged; the Dockerfile gained only the final `USER` line; the in-image selftest at HEAD still passes.

Item 1 is the exception.

#### 1. Re-read three passages of the contract that changed after the UAT

**Test:** In `docs/specs/equivalence-contract.md`, read the `rc` row of the field table, the `ret` bullet under reserved `lib_state` names, and the Byte codec paragraph (commit dd1deeb).
**Expected:** They match D-05, D-06 and D-25. What changed: a return wider than 32 bits is no longer promised to go to `lib_state_out.ret`; it is refused when the adapter is built (the code never recorded it, and before CR-01 truncated it). A struct or union return goes to `ret` with rc 0. The codec paragraph now describes serde_json's escapes and the exact file format.
**Why human:** UAT item 1 judged the earlier wording. The edit was made under version 0.1.0 with no changelog line (IN-11). The reviewer's reasoning that no behaviour changed holds against the repository (no policy, vector or provenance depends on the withdrawn promise), but accepting an unversioned contract correction is the founder's call.

### Gaps Summary

One gap, one root cause. Fix 8bdbd61 (WR-04) hardened the vectors job against a hostile tar, but its symlink guard rejects every link whose target contains a `..` component. The captures contain legitimate in-tree links of that form (nimble, nanopb), so the job stops before building any driver. The `vectors` job has been red since the first fix pass (runs 41265174150 and 41275770544), and the last in-image run of the executor build, the twice-identical runs, the determinism rerun and the sample replay is 40922236736 on 5e0d4a8. The review fixes that touched those paths (CR-01, WR-01, WR-02, WR-06, IN-04, IN-09) therefore have unit-test evidence and local-host evidence but no in-image evidence.

Why this blocks the phase and is not a footnote: the phase promised a reference result that is deterministic, and the only place the plan measured that against the pinned image is the job that is now red. Locally the executor passes everything, so this is not a defect in the contract, the schema, the adapters or the executor logic. It is a regression in a phase deliverable (`.github/workflows/testbed.yml`) that the review process did not catch.

Closure (small): fix the guard to test where the link resolves, add a behavioural test for it, push, and confirm a green `vectors` job, including 'Replay the committed sample'. If that replay fails on the new WR-01 provenance checks, that is a second finding to fix; the review reasoned that it will pass (the image mounts `testbed` whole, and `check_corpus_location` accepts `/opt/mt/testbed/vectors-sample/<target>/default/<entry>`), but nothing has run it.

Not gaps, tracked in the review disposition: WR-03 (skipped) and IN-01 (deferred) need a `runtime.c` change plus a sample regeneration in CI; IN-10, IN-11, IN-12 are open info items. No later phase of the milestone covers them, so none is moved to `deferred`.

Limits of this verification: the pinned image is not on this host, so the local runs use the host arm-none-eabi-gcc 13.2.1 and qemu-arm 8.2.2, not GCC 14.3 and qemu 10.0.2; driver and ELF hashes of the committed sample therefore could not be compared here. I did not re-run `cargo clippy`, `cargo fmt --check` or `cargo deny check`, and no network or model was used.

---

_Verified: 2026-11-03_
_Verifier: Claude (gsd-verifier)_
