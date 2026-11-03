---
phase: 03-equivalence-contract-and-test-vector-schema
reviewed: 2026-11-03T00:00:00Z
depth: standard
files_reviewed: 20
files_reviewed_list:
  - Cargo.lock
  - container/Dockerfile
  - crates/mt-cli/src/main.rs
  - crates/mt-contract/Cargo.toml
  - crates/mt-contract/src/policy.rs
  - crates/mt-contract/tests/eqv_01_contract_document.rs
  - crates/mt-contract/tests/eqv_01_policy.rs
  - crates/mt-harness/src/adapter.rs
  - crates/mt-harness/src/driver.rs
  - crates/mt-harness/src/replay.rs
  - crates/mt-harness/src/run.rs
  - crates/mt-harness/src/sample.rs
  - crates/mt-harness/tests/eqv_03_adapter.rs
  - crates/mt-harness/tests/eqv_03_driver_golden.rs
  - crates/mt-harness/tests/eqv_04_replay_provenance.rs
  - crates/mt-toolchain/tests/container_static.rs
  - crates/mt-vectors/src/state.rs
  - crates/mt-vectors/tests/eqv_02_state_and_compare.rs
  - docs/specs/equivalence-contract.md
  - crates/mt-harness/src/runtime.c
findings:
  critical: 0
  warning: 1
  info: 4
  total: 5
status: issues_found
---

# Phase 3: Code Review Report (re-review after fix iteration 2)

**Reviewed:** 2026-11-03
**Depth:** standard
**Files Reviewed:** 20
**Status:** issues_found

## Summary

This is an incremental re-review of fix iteration 2 (commits 09f4a96..90767fd, `git diff 7c77cd6 HEAD`) on the 19 listed files. `crates/mt-harness/src/runtime.c` was re-read for the two carried-forward findings (WR-03, IN-01).

No critical issue and no regression was found in the iteration-2 code. Every fix that was applied does what its commit message says. The remaining findings are one carried-forward Warning (WR-03), one carried-forward Info (IN-01) and three new Info items. Two of the new items come from the contract-document edit, and one is a narrowing of the old WR-01.

Checks run here: `cargo test --offline` for `mt-contract`, `mt-vectors` and `mt-cli` (all pass). For `mt-harness`, the tests `eqv_04_replay_provenance`, `eqv_03_driver_golden` and `eqv_03_adapter` pass. `container_static` `tool_03_final_stage_runs_as_the_unprivileged_user` passes. The CI-only paths were not run: the image build, the in-image `mt vectors replay --tree` of the committed sample and the workflow steps. By reading, the new replay checks use only values the replay already holds or reads from the mounted `testbed` directory. `/opt/mt/testbed/vectors-sample/<target>/default/<entry>` ends in the three names the index holds, so `check_corpus_location` accepts it. The `eqv_04_exec` replay test builds `corpus/<target>/<config>/<entry>` and so satisfies it too.

CLAUDE.md tool-code rules checked on the new code: no new `unsafe`; no new `unwrap` or `expect` outside tests (the new `.get()` in `sample.rs` replaces an index); no direct `std::process::Command`; `BTreeMap` throughout; new tests carry requirement names (`eqv_04_`, `eqv_03_`, `eqv_02_`, `eqv_01_`, `tool_03_`); no use of "certified" or "compliant" in the changed text.

### The contract versioning question (dd1deeb, IN-02)

The document says: "A change to the contract bumps the version, adds a line to the changelog, and moves `CONTRACT_VERSION` and every policy file under `testbed/policy` together." Commit dd1deeb edited the `rc` row, the `ret` bullet and the byte-codec paragraph and left the version at 0.1.0 with no changelog line.

Determination: this is a correction of wording, not a change to the contract as the rule means it, so no version bump is due. It does need a small finding (IN-11), because the rule has no exception for corrections and the edit left no trace in the document.

- **Byte codec.** The old text (`\u00XX` for control characters) never described what the code writes. The committed sample's bytes and file names are unchanged. Only the description moved to match them.
- **`rc` and `ret`.** The old text promised that a return wider than 32 bits goes to `lib_state_out.ret`. No version of the code ever did that. It was truncated before 6c58482 and refused after it. No committed policy, vector or provenance record depends on the promise (`grep` of `testbed/policy` finds nothing about wide returns). The version and `policy_sha256` exist to pin behaviour that vectors and policies depend on, and none of that moved.
- **Release status.** 0.1.0 has one changelog line ("contract v0 for Phase 3") and there are no git tags, so no copy of the old text has been published to a reader.
- **Why not bump.** A bump would move every policy file, every `policy_sha256` and `contract_version` in the sample, and force a regeneration. That cost is not the reason it is unneeded. The reason is that no behaviour changed.

The rule itself should say this, and the changelog should record the correction. See IN-11.

## Resolved since last review

- **WR-02** (listing of only `no_roles` rows passes): fixed in 975103a. `BuildRow::did_work` (state `Built`) and `RunRow::did_work` (state `Ran`) feed `listing_failed`'s first argument. Re-verified in `main.rs:1104-1142`: a non-empty listing with no worked row now exits 1, and the printed count stays `rows.len()`. The test covers `no_roles` only, `no_roles` plus a built row, and the run side. The `adapters` listing still passes `rows.len()`. That is acceptable, because it lists without running anything and has `--check-floor` as its gate.
- **IN-02** (document describes two behaviours the code does not have): fixed in dd1deeb. The new text matches the code. I checked `is_wide_return` (spelling), `resolved_return_kind` (DWARF size, float, array) and `Vector::render` (`to_vec_pretty`, field order, trailing newline). Two follow-ups are IN-11 and IN-12.
- **IN-03** (sorted `Map` assumption, unexplained indexings): fixed in 4ab3b0c. I checked the invariant comments against `state.rs:200-225` and `332-345`, and they are true. The `sample.rs` index became `.get()` with an `Input` error. The new test fails if `preserve_order` is enabled in the graph under test (the pre-merge `cargo test` over the workspace unifies features, so it covers the whole workspace).
- **IN-04** (duplicated helpers): fixed in 31e36b3. `build_driver` now uses `link::healthy` and `link::failure`. The old copy and the shared one test and format the same way, so there is no behaviour change. `selftest_command` calls `exec_context`, which is the same load, validate, tools and runner-config sequence.
- **IN-05** (no default non-root user): fixed in 7df5e88. `USER 65534:65534` is the last instruction of the `final` stage. Every `docker run` of the image in `testbed.yml` and `container.yml` passes `--user` (the one without it, in `pin-discovery.yml`, runs the `snapshot-sources` target). Nothing is built `FROM final`. The new test checks the last `USER` of the stage.
- **IN-06** (denylist versus allowlist): fixed in c74f962 for `driver_flags`, which now refuses any argument starting with `@`. The remaining part, that `recompile_arguments` (`probe.rs:258-286`) is a denylist, is dropped as a reasonable design choice, not kept as a finding. That function must reproduce the flags the object was built with so that the DWARF layout matches the object, so narrowing it to the driver allowlist could change recorded layouts. It works on commands from a verified capture. Those commands come from the same upstream build that already ran in the capture job, and the recompile happens in the no-network, read-only, capability-dropped container. An `@file` kept there is expanded by the compiler exactly as it was for the object, which is the faithful behaviour.
- **IN-07** (non-UTF-8 policy file name skipped): fixed in ad8fc28. `load_dir` reports it as an issue. The test is `cfg(unix)` and creates a `0xff` file name, which a Linux filesystem accepts.
- **IN-08** (split doc comment): fixed in 09f4a96. The doc lines now sit above the right functions.
- **IN-09** (trigraph splice, probe path unchecked): fixed in 4e35d59. `joins_or_comments` also refuses `??/`. `include_lines()` (`run.rs:473-492`) checks every line with `is_prefix_line` before the probe and the driver see it. Accepted lines render the same text, so no hash moves. The golden driver test passes. The false refusal of `/*` inside a string literal is kept on the safe side, as the fix report says.
- **WR-01** (replay does not compare most provenance fields): fixed in 90767fd for everything that can be checked against a trusted value. `provenance_mismatches` compares `seed`, `scalars`, `executor_version`, `contract_version`, `budgets`, `policy_sha256` and the record shape (which includes `outcome` against the vector's own class). `check_corpus_location` ties the directory to `<target>/<config>/<entry>`. Re-verified against `replay.rs:190-267, 445-453`, the sample tests, and `policy_of` (the same function the run uses to write the field, so a replay reads the same hash). The remaining parts:
  - `image` is dropped as a reasonable design choice. It names the image that made the corpus, and the Dockerfile changed in this very iteration (IN-05), so every image rebuild would fail the committed sample for no change in tool output. The driver, archive and object hashes already pin the tool output.
  - `has_ub_source`, `unobserved`, `discarded_output_bytes` and `replay`: `unobserved` and the fit of `has_ub_source` to the vector are already checked by `Provenance::issues`, and `replay` cannot be derived. The two that a replay could still check are the subject of IN-10, because the stated reason for skipping them is wrong.

## Warnings

### WR-03: `struct mt_ksigaction` does not match the kernel `sigaction` layout (uninitialised padding becomes the signal mask)

**File:** `crates/mt-harness/src/runtime.c:257-262, 292-296`
**Issue:** Carried forward unchanged and re-verified against current source (`git diff 7c77cd6 HEAD -- runtime.c` is empty). The fix was skipped because editing `runtime.c` changes every `driver_source` and `driver_elf` hash in the committed sample. The ARM kernel `struct sigaction` is `{handler, flags, restorer, sigset_t mask[2 x u32]}`, with the mask at offset 12. The driver declares `unsigned long long mask`, which AAPCS aligns to 8, so the member sits at offset 16 and bytes 12-15 are uninitialised stack padding. `act.mask = 0` initialises the member, not the padding. `act` is a plain automatic variable, so the padding is whatever was on the stack. The kernel and qemu-user read `sa_mask` from offset 12, so the set of signals blocked while the handler runs is stack garbage and can differ between runs. The handler only calls `mt_exit`, so the visible effect today is small, but the layout is wrong and the value is nondeterministic, in a runtime whose purpose is deterministic replay.
**Fix:** Declare `unsigned long mask[2];` (offset 12, 8 bytes, matching `MT_SIGSET_SIZE`), zero both words, and `memset(&act, 0, sizeof act)` before filling the struct. Regenerate `testbed/vectors-sample` in CI in the same change, since the driver hashes move.

## Info

### IN-01: `MAP_FIXED` mappings silently replace whatever is already at the address, and no assert covers the input mapping

**File:** `crates/mt-harness/src/runtime.c:108, 129, 177, 737, 787`
**Issue:** Carried forward and re-verified. `MT_MAP_RW_FIXED` is `0x32`, which is `MAP_PRIVATE|MAP_FIXED|MAP_ANONYMOUS`, so a mapping that lands on existing memory replaces it without error. The `_Static_assert` at line 108 covers only the output mapping against `MT_STATE_BASE`. Nothing checks `MT_IN_BASE` plus the input mapping against `MT_OUT_BASE`, and `mt_fields[1].len` is bounded only through `MT_REQ_MAX` (`MT_MAX_INPUT + 4096`, line 454), not by `MT_MAX_INPUT`. With `Budgets::DEFAULT` (`max_input` 65536) the input mapping is about 20 pages inside a 16 MiB gap, so the overlap is unreachable today and a static image would have to reach 64 MiB to hit `0x04000000`. This is defence in depth. `MT_MAX_INPUT` is a template placeholder, so a later change of the budget could create the overlap silently, and the failure would be wrong vectors, not a driver error.
**Fix:** Use `MAP_FIXED_NOREPLACE` (`0x100000`) so a collision returns an error and exits with `MT_EXIT_MAP`. Add `_Static_assert(MT_IN_BASE + (MT_MAX_INPUT / MT_PAGE + 3u) * MT_PAGE <= MT_OUT_BASE, ...)`. Reject `mt_fields[1].len > MT_MAX_INPUT` in `mt_main`, as `out_cap` already is at line 734. Regenerate the sample together with WR-03.

### IN-10: Replay re-runs every vector but does not compare `discarded_output_bytes` or the derived `has_ub_source`, and the comment that excuses it is wrong

**File:** `crates/mt-harness/src/replay.rs:190-203, 437-453`; `crates/mt-harness/src/run.rs:962-974`
**Issue:** (Narrowed from the old WR-01.) The doc comment of `provenance_mismatches` and the fix report say that `has_ub_source`, `discarded_output_bytes`, `unobserved` and `replay` are "not compared" because "only a full re-run observes them". But `replay_corpus` is a full re-run: `run_vector_twice` returns an `Observed` with `discarded` and `outcome` at line 437, and the provenance loop runs right after it. `unobserved` and the fit of `has_ub_source` to the vector's `has_ub` label are already checked by `Provenance::issues`. What is not checked: `discarded_output_bytes` against `observed.discarded`, and the exact `has_ub_source` variant (for example `observed_crash` against `observed_abort` against `oob_write`, all of which pass the "labelled" test) against `has_ub_source(&observed.outcome)`. Both are carried in the provenance as evidence and never verified. This is low impact: they are informational counters, and an edited vector or driver is already caught elsewhere. But the stated reason is false, and the replay is described as the tamper check for the sample.
**Fix:** Make `has_ub_source` in `run.rs` `pub(crate)`. After `run_vector_twice` returns `Ok(done)`, push `"discarded_output_bytes"` when `u64::from(done.discarded) != provenance.discarded_output_bytes`, and push `"has_ub_source"` when `provenance.has_ub_source != has_ub_source(&done.outcome)`, skipping the second when the record says `Phase4` (written later by Phase 4). Correct the doc comment to say that only `image` and `replay` are not compared, and why.

### IN-11: The contract document was edited under version 0.1.0 with no changelog line, and its versioning rule has no exception for corrections

**File:** `docs/specs/equivalence-contract.md:14, 149-151`
**Issue:** See "The contract versioning question" in the Summary for the determination. dd1deeb is a correction of wording and needs no bump. But the rule as written ("A change to the contract bumps the version, adds a line to the changelog...") has no carve-out, and the changelog says only `- 0.1.0 (2026-10-21): contract v0 for Phase 3.` The `rc` row, the `ret` bullet and the byte-codec paragraph were changed in a way that is visible only in git. A reader holding the earlier text, or a later phase that quotes it, has no way to learn that the wide-return promise was withdrawn. The test `eqv_01_contract_document_has_version_and_changelog` requires only a line starting with `- 0.1.0`, so an erratum line does not break it.
**Fix:** Add an indented erratum line under the 0.1.0 entry, for example `  - erratum (2026-11-01): the rc and ret entries and the byte codec paragraph now describe what the code does (wide integer returns are refused, never recorded; the serde_json escapes). No behavior, vector, policy or version changed.` Add one sentence to the versioning rule: a correction that changes no behavior is recorded as an erratum under the current version and moves nothing else. Any change to what a conforming vector, policy or comparator must do still bumps the version and moves the policies.

### IN-12: The byte-codec paragraph still leaves out the hex case of `\u00XX`, so an independent reader cannot reproduce the file names

**File:** `docs/specs/equivalence-contract.md:64`; `crates/mt-contract/tests/eqv_01_contract_document.rs:146-163`
**Issue:** The rewritten paragraph says "an independent reader must reproduce exactly those bytes" and writes the escape as `\u00XX`. `serde_json` writes the two hex digits in lowercase (`HEX_DIGITS = b"0123456789abcdef"`, `ser.rs`), for example `\u001f`. A reader who follows the document and writes `\u001F` computes a different sha256 and so a different file name. The new test would not catch it: it checks for the substrings `serde_json` and `` `\n` `` and the absence of one old sentence, not the escape form.
**Fix:** Write the escape as "`\u00` followed by two lowercase hex digits" in the document, and add `u001f` (lowercase) to the substring test, or better, have the test serialise a string with U+001F and compare it with the form the document states.

---

_Reviewed: 2026-11-03_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
