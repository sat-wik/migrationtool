---
phase: 03-equivalence-contract-and-test-vector-schema
fixed_at: 2026-11-01T00:00:00Z
review_path: .planning/phases/03-equivalence-contract-and-test-vector-schema/03-REVIEW.md
iteration: 1
findings_in_scope: 9
fixed: 8
skipped: 1
status: partial
---

# Phase 3: Code Review Fix Report

**Fixed at:** 2026-11-01
**Source review:** .planning/phases/03-equivalence-contract-and-test-vector-schema/03-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 9 (CR-01, WR-01 to WR-08; Info findings out of scope)
- Fixed: 8 (WR-01 and WR-05 are partly fixed, see below)
- Skipped: 1 (WR-03)

**Verification ran in:** an isolated git worktree (`.claude/worktrees/rf-03-*`, sharing the main checkout's cargo target directory). Before each commit: `cargo fmt --check`, `cargo clippy -p <crate> --all-targets -- -D warnings` and `cargo test -p <crate>`. After the last commit: `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace`, all green. `cargo deny check` and the container version check were not run. The two workflow changes (WR-04, WR-05) are exercised only by the static workflow tests locally; **CI verification of `.github/workflows/testbed.yml` is pending.** The tests marked `ignored` that need `MT_LOCAL_CAPTURES` and the pinned toolchain image did not run, so WR-01 (live replay path) and CR-01 (live build path) are covered by offline unit and structure tests only.

## Fixed Issues

### CR-01: A return value that is a typedef of a 64-bit integer is silently truncated to 32 bits

**Files modified:** `crates/mt-harness/src/run.rs`, `crates/mt-harness/tests/eqv_04_return_width.rs` (new)
**Commit:** 6c58482
**Applied fix:** Factored the DWARF-to-`ReturnKind` mapping into `pub fn resolved_return_kind`. An integer or enum return larger than 4 bytes is now refused with "a 32-bit rc cannot hold it" instead of being rendered as `(int32_t)(call)`. The other mappings (struct, union, pointer, float and array refusal) are unchanged. Tests cover `intmax_t`, `uintmax_t`, `int_fast64_t`, `off64_t`, a wide enum, narrow integers and the unchanged pointer and float mapping. The alternative (record wide returns in `lib_state_out.ret`) was not implemented; the refusal matches what the code does for spelled-out wide types today.

### WR-01: `replay_corpus` is weaker than its documentation and trusts unvalidated index fields

**Files modified:** `crates/mt-harness/src/replay.rs`, `crates/mt-harness/tests/eqv_04_replay_shape.rs` (new)
**Commit:** 9f3aca3
**Applied fix (partial):** New `pub fn check_corpus_shape`, called before any vector is run. It requires the index `config` to be a configuration of the index `target` in the manifest, requires every index key to be a lowercase 64-hex sha256 (closes the `../../x` path join), and refuses any non-regular file (symlink) or file that is not `index.json` or an index key's `<sha>.json`. `replay_corpus` also calls `manifest.validate()` and now counts a vector as differing when `sha256(stored) != index key`. Tests run on the committed sample (all 16 corpora pass) plus a stray file, a traversing key, an unknown configuration and a symlink.
**Not done:** the provenance comparisons for `policy_sha256`, `image`, `executor_version`, `contract_version`, `budgets`, `seed` and `scalars`. They need the policy file path and run context plumbed into replay, and a wrong comparison would fail the CI sample replay, which cannot be run locally. This remains open for a follow-up. Status for this finding: logic change, requires human verification.

### WR-02: `mt vectors build` and `mt vectors run` report `result: passed (0 rows)` and exit 0 on an empty selection

**Files modified:** `crates/mt-cli/src/main.rs`
**Commit:** f1de950
**Applied fix:** Added `listing_failed(rows, any_failure)`, which is true for an empty listing. `build_command`, `run_command` and `adapters_command` use it, so an empty selection prints `result: failed` and exits 1. The "only `no_roles` rows" refinement from the review was not added. Test: `eqv_04_empty_listing_is_a_failure` (unit test in `main.rs`; an end-to-end empty-manifest case was not practical offline).

### WR-04: The vectors job unpacks a tar produced by the untrusted-build job straight onto the runner

**Files modified:** `.github/workflows/testbed.yml`, `crates/mt-harness/tests/eqv_04_workflow_static.rs`
**Commit:** 8bdbd61
**Applied fix:** The restore step now unpacks through a `safe_untar` function. It lists the archive (`tar -tvf`), fails on any member that is not a regular file, directory or symlink (hard links, devices and so on), fails on any symlink whose target is absolute or contains `..`, and extracts with `--no-same-owner --no-same-permissions`. Symlinks that stay inside the tree are still allowed because `capture.rs` documents that run directories may legitimately hold them. The function was exercised locally against crafted tars (clean tar accepted; `..` and absolute symlinks refused). Producer-side filtering and in-container extraction were not done. Test: `eqv_04_workflow_static_restore_vets_the_archives_before_unpacking`. **CI verification pending.**

### WR-05: The "Replay the committed sample" step passes silently when the sample directory is missing

**Files modified:** `.github/workflows/testbed.yml`, `crates/mt-harness/tests/eqv_04_workflow_static.rs`
**Commit:** cf3d3c3
**Applied fix:** The missing-directory branch now writes an error message, calls `finish_vectors sample 1 vectors-sample || true` (publishes the error notice) and `exit 1`. The header comment was updated. The explicit-input opt-out was not added. The existing `eqv_04_workflow_static_sample_steps` test was extended to assert the branch fails. **CI verification pending.**

### WR-06: `is_prefix_line` does not enforce "a single directive" and lets line-splicing and comment openers into the C source

**Files modified:** `crates/mt-harness/src/driver.rs`, `crates/mt-harness/tests/eqv_03_driver_golden.rs`
**Commit:** 6c06e69
**Applied fix (partial):** Lines ending in a backslash, containing `/*`, or containing `//` outside a string or character literal are now refused. The escape state inside literals is tracked so `"a\\" // x` is still caught. The absolute-path `#include` rejection from the review was deliberately **not** applied: the entry header is itself included by absolute path (`#include "/fixed/src/include/core_json.h"` in every golden driver), so it would break all builds. The "line count equals fragment count" assertion was not added because control characters (newlines) are already refused. Tests extend the existing injection test in `eqv_03_driver_golden.rs`, including an accepted `//` inside a string literal.

### WR-07: Policy `error_code` values are not range-checked against the function's return type

**Files modified:** `crates/mt-contract/src/policy.rs`, `crates/mt-contract/tests/eqv_01_policy.rs`
**Commit:** 43af8c5
**Applied fix:** Integer values and decimal-text values must fit `i32::MIN..=u32::MAX` (same window as `StateValue::Int`); text that overflows `i64` is refused. Boundary values (`4294967295`, `-2147483648`) are accepted and the first values outside the window are refused in `eqv_01_error_codes_are_checked`. All committed policies still validate.

### WR-08: Identifier grammars for target ids and configuration names disagree between the manifest, the policy loader and the vector schema

**Files modified:** `crates/mt-capture/src/manifest.rs`, `crates/mt-capture/tests/bed_01_manifest.rs`
**Commit:** 0f9264e
**Applied fix:** `target_issues` refuses an id starting with `_`; `is_config_name` refuses a name starting with `-`. Both now match the leading `[a-z0-9]` requirement of the vector `argv[0]` and policy module grammars. Test: `bed_01_target_ids_and_config_names_start_with_a_letter_or_digit`. The cross-crate test that ties all three grammars together was not added (the three crates' predicates are private).

## Skipped Issues

### WR-03: `struct mt_ksigaction` does not match the kernel `sigaction` layout (uninitialised padding becomes the signal mask)

**File:** `crates/mt-harness/src/runtime.c:257-262, 292-296`
**Reason:** The claim reproduces by reading the code (`unsigned long long mask` sits at offset 16 on ARM, the kernel reads the mask at offset 12). The fix was skipped because `runtime.c` is compiled into every generated driver (`RUNTIME_TEMPLATE`), so any edit changes the `driver_source` and `driver_elf` sha256 of every driver. Those hashes are recorded in the provenance of the committed vectors under `testbed/vectors-sample`, so the CI sample replay would report provenance mismatches until the sample is regenerated. Regenerating the sample is outside this fix pass. Suggested follow-up: change the struct to `unsigned long mask[2]` zeroed together with the rest of `act`, then regenerate `testbed/vectors-sample` in CI in the same change.
**Original issue:** The driver declares the signal mask as `unsigned long long`, leaving bytes 12-15 as uninitialised padding that the kernel reads as the blocked-signal set.

---

_Fixed: 2026-11-01_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
