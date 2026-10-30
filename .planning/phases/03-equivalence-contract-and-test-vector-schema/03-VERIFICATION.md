---
phase: 03-equivalence-contract-and-test-vector-schema
verified: 2026-10-30T00:00:00Z
status: passed
score: 4/4 must-haves verified
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
  - "crates/mt-contract/src/policy.rs"
  - "crates/mt-harness/src/run.rs"
  - "crates/mt-vectors/src/vector.rs"
  - "docs/specs/equivalence-contract.md"
  - "testbed/adapter-dropped.toml"
  - "testbed/manifest.toml"
covered_digest: "v3:sha256:46e81dfac7b40331ddbb344fce5c6f716beed61bdd97f57f01e1b8b76feb86ff"
behavior_unverified: 0
overrides_applied: 0
human_verification:
  - test: "Read docs/specs/equivalence-contract.md (0.1.0) against 03-CONTEXT.md D-05..D-30"
    expected: "Every sentence of the contract matches the decision it restates; none adds or drops a rule"
    why_human: "03-VALIDATION.md lists this as the one manual-only check (judgment over wording beyond the keyword checks). The verifier read it and found no mismatch, but that does not replace the founder's reading."
  - test: "Decide whether LZ4_decompress_safe's overshoot is acceptable under D-07 (whole output buffer recorded after a 0xA5 sentinel prefill)"
    expected: "A recorded founder decision: keep D-07 as is, or amend it (the question has been open since 03-04 and is repeated untouched in 03-05..03-08)"
    why_human: "A design decision about what the reference records, not something a test can settle."
  - test: "Decide how function-local statics named name.N (for example version.0) are keyed in lib_state.globals"
    expected: "Either relax the C-identifier rule on the symbol part of a globals key in mt-vectors, or accept that a changed function-local static fails the run. No committed target triggers it today."
    why_human: "03-05 left it open on purpose. It changes the schema's key grammar, which D-06 marks as costly to reverse."
  - test: "Review 03-05's edit of its own Task 3 verify command (added `| grep -v ' no_roles'`, commit 7878588)"
    expected: "Agreement that the original command could not pass (it also matched nimble's no_roles rows) and that the corrected command still checks the layout cross-check"
    why_human: "An executor changed its own acceptance test. The summary documents it as a plan defect, but it is an edit to the check itself."
  - test: "Decide on isotp/isotp_on_can_message, logged as link_unresolved instead of declared"
    expected: "Either accept the log entry, or approve a reviewed C shim for the three user hooks (isotp_user_debug, isotp_user_get_ms, isotp_user_send_can). D-11 forbids per-target hand-written C, so a shim needs a decision."
    why_human: "Needs a policy exception to D-11. The floor does not depend on it (15 targets are drivable, floor 10)."
  - test: "Judge the committed sample rule (smallest seed per target)"
    expected: "7 of the 15 sample inputs are 1 byte and tinycbor's is empty. Decide whether that is an adequate offline reference or the rule should pick a richer seed."
    why_human: "The tests prove the rule and the bytes (03-08 says so itself); whether the sample is good enough is a judgment."
  - test: "Confirm the has_ub label `timeout` (rc -9) for the timeout outcome"
    expected: "Agreement that a timeout is labelled has_ub `timeout` so it is excluded from comparison and never counted as a pass, or a different label."
    why_human: "03-01 flagged it as a choice the founder may revisit. The contract and the selftest agree with each other."
  - test: "Accept QEMU 10.0.2 (bookworm-backports, 1:10.0.2+ds-2+deb13u1~bpo12+1) in place of D-31's 8.2.x"
    expected: "Explicit acceptance that D-31's version number was not followed. The cause D-31 names (commpage on M-profile) is gone: `qemu-arm -cpu cortex-m4` starts and every vectors job run is green."
    why_human: "03-09 recorded the deviation for the founder; 03-10 records the founder's merge of PR #6 and the re-baseline, but the 8.2.x versus 10.0.2 difference is not stated as accepted in the 03-10 summary."
---

# Phase 3: Equivalence Contract and Test-Vector Schema Verification Report

**Phase Goal:** "Equivalent" is written down and machine-checkable, and the original C can be run against any test vector to produce a deterministic reference result.
**Verified:** 2026-10-30
**Status:** human_needed
**Re-verification:** No, initial verification

All four roadmap success criteria hold in the code and in CI. The status is `human_needed` only because of the eight judgment items above, which the executors and the orchestrator flagged for the founder. No gap blocks the goal.

## Goal Achievement

### Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Contract v0 exists as a versioned document and a policy file the harness loads: refinement, observation point at the C interface, UB menu {return error, panic, preserve} with "return error" as default and a recorded setting to change it | VERIFIED | `docs/specs/equivalence-contract.md` has Version 0.1.0, a changelog, and sections "Refinement", "Observation point", "UB action menu and policy". `crates/mt-contract/src/policy.rs` defines `Action {ReturnError, Panic, Preserve}`, `deny_unknown_fields` at every level, `DefaultChange` and a full-record `Override`. 16 `testbed/policy/*.toml` files, one per target. `mt vectors policy-check --manifest testbed/manifest.toml` ran locally and printed `valid default=return_error overrides=0` for every module. Named tests exist and pass: `eqv_01_rejects_unknown_key`, `eqv_01_default_change_required_to_change_default`, `eqv_01_override_needs_every_field`, `eqv_01_override_naming_no_logged_site_is_rejected`, `eqv_01_empty_override_table_is_accepted`, `eqv_01_override_report_lists_every_override`. |
| 2 | A schema with exactly the nine fields validates and round-trips in Rust types; a merged `lib_state` is rejected; at least 3 TRACTOR public-corpus vectors load unchanged | VERIFIED | `crates/mt-vectors/src/vector.rs` is a struct of the nine fields in order, with unknown keys refused. I took a committed sample vector and renamed `lib_state_in` to `lib_state`: `mt vectors validate` exited 2 with "unknown field `lib_state`, expected one of `argv, stdin, env, stdout, stderr, rc, lib_state_in, lib_state_out, has_ub`". Adding `"has_ub": false` was also refused (exit 2), so D-26 holds. CI `vectors-tractor` annotation (job 123623035539): 6 pinned files fetched, `sha256=ok`, `round_trip=json-equal` on each, "loaded: 6 of 6 (at least 3 required)". `testbed/tractor-pin.toml` holds only commit, paths and hashes. Tests: `eqv_02_rejects_merged_lib_state`, `eqv_02_tractor_pin_has_no_content`, `eqv_02_has_ub_vectors_are_excluded_from_comparison`. The ignored local TRACTOR test needs `MT_LOCAL_TRACTOR` and was not run here; CI is the evidence. |
| 3 | At least 10 test-bed targets have an entry-point adapter declaring the function and its argument roles | VERIFIED | `mt vectors adapters --manifest testbed/manifest.toml --check-floor` ran locally: "drivable targets: 15 (floor 10)", "result: passed (27 rows)". Fifteen of 16 targets are drivable; 7 entries are logged in `testbed/adapter-dropped.toml` with cause and attempt (3 `no_seeds`, 2 `link_unresolved`, 2 `not_expressible`). Test `eqv_03_adapter_floor_is_ten` (crates/mt-harness/tests/eqv_03_adapter_floor.rs) counts over the committed files and passes. The CI vectors job builds a driver per entry and runs it: 51 `vectors-run` rows across both annotation pages. |
| 4 | The C reference executor runs any vector against the target's original C build, stores the result in a schema-valid vector, and running it twice gives byte-identical results | VERIFIED | `crates/mt-harness` links the exact captured objects (`eqv_04_refuses_modified_object` passes) into a generated driver and runs it under `qemu-arm -cpu cortex-m4` through the runner. CI run 40922236736 on 5e0d4a8 (jobs `testbed` 123619075984 and `vectors` 123623035539, plus `mt` 123618271049) all succeeded with no failing step, and ci run 40922236801 on the same head succeeded. The only commits after 5e0d4a8 touch `.planning` files. Every `vectors-run` row says `twice=identical`; `vectors-determinism` says "rerun rows are identical to the first run's rows" and "identical 5768 files". `vectors-selftest` shows the outcome cases (ok, overrun-input and overrun-output fault rc=-11, slack-write, abort rc=-6, loop timeout rc=-9, beyond-reserve is `executor_error`, stack-overflow fault), "no core file after overrun", and "determinism 100 runs 1 hash". The 15 committed one-vector corpora in `testbed/vectors-sample` all pass `mt vectors validate` locally ("valid corpus 1 vectors"), and the CI `vectors-sample` step compared them to a fresh run: "identical". |

**Score:** 4/4 truths verified (0 behavior-unverified; every behavior-dependent truth, the twice-identical and guard-page cases, has a CI-run behavioral result).

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `docs/specs/equivalence-contract.md` | Versioned contract, changelog | VERIFIED | 0.1.0, changelog line, 12 sections; no forbidden claim words in the document or in the three crates' sources. |
| `crates/mt-contract/src/{lib,policy}.rs` | Policy loader | VERIFIED | 586-line `policy.rs`, `#![forbid(unsafe_code)]`, imported by `mt-cli` and `mt-vectors` tests. |
| `crates/mt-vectors/src/{vector,state,bytes,compare,corpus,tractor}.rs` | Schema, Latin-1 codec, lib_state model, comparison rule, corpus index, TRACTOR level | VERIFIED | Substantive; wired through `mt vectors validate`, `check`, `tractor`. |
| `crates/mt-harness/src/*` | Adapter, driver, link, runtime template, executor, layout, globals, heap walk, sample | VERIFIED | About 8,800 lines across 20 modules; wired through `mt vectors build|run|replay|sample|adapters|selftest|layouts`. |
| `testbed/policy/*.toml` | 16 policies | VERIFIED | One per manifest target; `policy-check --manifest` enforces exactly one each. |
| `testbed/manifest.toml`, `testbed/adapter-dropped.toml` | Roles and drop log | VERIFIED | `adapters --check-floor` parses both and passes. |
| `testbed/tractor-pin.toml` | Commit, paths, hashes only | VERIFIED | Six paths and sha256 values; no corpus content. |
| `testbed/vectors-sample/` | Committed offline sample | VERIFIED | 15 corpora, valid, produced by CI in the pinned image. |
| `.github/workflows/testbed.yml` | `testbed` and `vectors` jobs | VERIFIED | Both green on 5e0d4a8. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| manifest roles | generated driver | `mt vectors build` | WIRED | `built driver_sha256=...` rows in CI for each entry with roles. |
| captured objects | driver link | sha256 recheck against capture.json | WIRED | `eqv_04_refuses_modified_object`. |
| driver | vector files + provenance | `mt vectors run` | WIRED | Corpus per entry, `twice=identical`, replay compares provenance hashes. |
| policy | run | policy loaded first in `mt vectors run` | WIRED | Per the command's help text and the `eqv_01_policy_check_cli` / `eqv_04_vectors_cli` tests. |
| pinned TRACTOR pin | local loader | `mt vectors tractor fetch|check` | WIRED | CI annotation above. |

### Data-Flow Trace (Level 4)

| Artifact | Data | Source | Produces Real Data | Status |
|----------|------|--------|--------------------|--------|
| committed sample vector (cjson) | `rc`, `stdout`, `lib_state_out.heap` | QEMU run of the cJSON object code | Yes: `rc: 1`, a typed heap walk with pointer `{buffer, offset}` edges | FLOWING |
| `mt vectors run` counts | ok/fault/setup_failed | classified runner results | Yes: for example `paho MQTTDeserialize_publish ok=16 fault=1`, `tinycbor cbor_value_validate setup_failed=60` | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Workspace tests | `cargo test --workspace --locked` | no failures in any `test result` line (orchestrator also reported 532 passed, 0 failed, 23 ignored) | PASS |
| Floor of 10 | `mt vectors adapters --manifest testbed/manifest.toml --check-floor` | 15 drivable, passed | PASS |
| Policies | `mt vectors policy-check --manifest testbed/manifest.toml` | 16 valid, default return_error | PASS |
| Sample valid | `mt vectors validate testbed/vectors-sample/*/default/*` | 15 of 15 valid | PASS |
| Merged lib_state rejected | `mt vectors validate` on a tampered sample | exit 2, unknown field `lib_state` | PASS |
| `has_ub: false` rejected | `mt vectors validate` on a tampered sample | exit 2 | PASS |

### Probe Execution

SKIPPED. Phase 3 declares no `probe-*.sh` scripts. Its in-image probe is the executor's `mt vectors selftest`, whose CI result is above.

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| EQV-01 | 03-02 | Versioned contract and policy: refinement, observation point, UB menu, never crash | SATISFIED | Truth 1. |
| EQV-02 | 03-03 | Nine-field schema, round trip, merged `lib_state` rejected, `has_ub` excluded from comparison | SATISFIED | Truth 2. |
| EQV-03 | 03-04, 03-05, 03-06, 03-07, 03-08 | Adapter declares entry and roles; at least 10 targets drivable | SATISFIED | Truth 3 (15). |
| EQV-04 | 03-01, 03-04..03-08, 03-09, 03-10 | C reference executor, schema-valid result, identical on a second run | SATISFIED | Truth 4. 03-09 and 03-10 carry the QEMU re-pin that made the executor start. |

Every ID found in the plan frontmatter (EQV-01..EQV-04) is accounted for in REQUIREMENTS.md, which marks all four Complete. No ORPHANED requirement: REQUIREMENTS.md maps no other ID to Phase 3.

### Context decisions D-01..D-32

Honored, as far as the code and CI show: D-01/D-30 (qemu-arm cortex-m4, static ELF), D-02 (budgets, run twice, never retried), D-03/D-27 (fault and abort recorded with `has_ub` labels), D-04 (guard pages, slack check), D-05/D-25 (field convention, Latin-1), D-06 (typed lib_state), D-07 (whole-buffer record, but see human item 2), D-10 (globals), D-11 (no hand-written per-target C; a single shared runtime template), D-13/D-28 (floor and drop log), D-15..D-18/D-29 (TOML policy, overrides, default_change), D-19..D-23 (sidecar index, absent `has_ub`, sample, seeds only, pinned TRACTOR), D-26, D-32 (second job). Two departures need the founder: D-31 names QEMU 8.2.x and the image has 10.0.2 (human item 8), and D-02's "empty environment" is implemented as the runner's four fixed keys (disclosed in 03-01).

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| (none) | | No TBD/FIXME/XXX/TODO/`todo!`/`unimplemented!` in the phase's crates, workflow or contract; no `std::process::Command` in `mt-contract`, `mt-vectors` or `mt-harness` | none | |
| `crates/mt-vectors/src/vector.rs` | 343 | `.expect("vector fields always serialize")` outside tests | Info | States its invariant in the message; serializing plain structs with string-keyed maps cannot fail. |
| `.planning/phases/03-.../03-VALIDATION.md` | frontmatter | `status: draft`, the nyquist flag false, sign-off boxes unchecked | Info | The bookkeeping file was never advanced. It does not change the goal result; `/gsd-validate-phase 3` would close it. |

### Gaps Summary

No gaps. Truths, artifacts, links and requirements all check out against the code and the final CI run. Seven entries are not drivable (three have no seeds, two cannot link, two cannot be expressed), all logged with causes, and the floor of ten is exceeded by five targets.

Limits of this verification: I did not rebuild the image or run QEMU locally (the pinned image is not on this host), so the executor behavior (guard-page faults, twice-identical runs, selftest) rests on the recorded CI run 40922236736, read through the GitHub API, and not on a local re-run. The ignored `MT_LOCAL_TRACTOR` and `MT_LOCAL_CAPTURES` tests were not run for the same reason.

---

_Verified: 2026-10-30_
_Verifier: Claude (gsd-verifier)_
