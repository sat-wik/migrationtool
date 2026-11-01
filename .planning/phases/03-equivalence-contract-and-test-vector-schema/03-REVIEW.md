---
phase: 03-equivalence-contract-and-test-vector-schema
reviewed: 2026-11-01T00:00:00Z
depth: standard
files_reviewed: 21
files_reviewed_list:
  - .github/workflows/testbed.yml
  - container/Dockerfile
  - crates/mt-capture/src/manifest.rs
  - crates/mt-capture/tests/bed_01_manifest.rs
  - crates/mt-cli/src/main.rs
  - crates/mt-contract/src/policy.rs
  - crates/mt-contract/tests/eqv_01_policy.rs
  - crates/mt-harness/src/adapter.rs
  - crates/mt-harness/src/driver.rs
  - crates/mt-harness/src/link.rs
  - crates/mt-harness/src/probe.rs
  - crates/mt-harness/src/replay.rs
  - crates/mt-harness/src/run.rs
  - crates/mt-harness/src/runtime.c
  - crates/mt-harness/src/sample.rs
  - crates/mt-harness/tests/eqv_03_driver_golden.rs
  - crates/mt-harness/tests/eqv_04_replay_shape.rs
  - crates/mt-harness/tests/eqv_04_return_width.rs
  - crates/mt-harness/tests/eqv_04_workflow_static.rs
  - crates/mt-vectors/src/state.rs
  - docs/specs/equivalence-contract.md
findings:
  critical: 0
  warning: 3
  info: 9
  total: 12
status: issues_found
---

# Phase 3: Code Review Report (re-review after fix pass)

**Reviewed:** 2026-11-01
**Depth:** standard
**Files Reviewed:** 21
**Status:** issues_found

## Summary

This is an incremental re-review of the fix commits 6c58482..0f9264e (`git diff 4c4f884 HEAD`) on the 13 listed files, plus a re-check of the carried-forward findings against current source.

The critical finding and four of the warnings are resolved. Three warnings remain, two of them narrowed to the part that is still a real defect. Two new Info items come from the fix diffs. No regression was found in the fix code itself.

Not run: the workflow changes (WR-04, WR-05 in the previous numbering) are unexercised in CI. The fix report says CI verification is pending, and this review could not run it. One risk to watch on the first CI run is `safe_untar`. It rejects any symlink whose target contains a `..` component. An upstream build tree that legitimately holds such a link (for example `include -> ../include`) would fail the whole vectors job.

## Resolved since last review

- **CR-01** (64-bit typedef return truncated to `rc`): fixed in `run.rs:532-559`. `resolved_return_kind` refuses any `Kind::Int` or `Kind::Enum` larger than 4 bytes, whatever its spelling. `layout_for_entry` always inserts the `ret` layout when `resolve_return` is true, so the `None => Integer` arm cannot be reached for a probed return. Struct, union, pointer, float and array mappings are unchanged. `eqv_04_return_width.rs` covers `intmax_t`, `uintmax_t`, `int_fast64_t`, `off64_t`, a wide enum, the narrow sizes and the unchanged pointer and float mapping. The contract-document mismatch that remains is tracked in IN-02.
- **WR-04** (tar unpacked on the runner): `safe_untar` (`testbed.yml:1038-1053`) now lists each archive, rejects members that are not a regular file, directory or symlink (this includes hard links), rejects symlinks whose target is absolute or contains `..`, and extracts with `--no-same-owner --no-same-permissions`. Every remaining symlink target is relative and has no `..`, so a later member cannot be written outside the destination through a link, and GNU tar refuses `..` in member names by itself. The `grep -v` and `grep -E` conditions behave correctly under `set -euo pipefail`. A test checks that the single `tar -xf` sits inside `safe_untar`. CI verification is still pending.
- **WR-05** (missing sample directory passes silently): fixed. The branch writes an error, calls `finish_vectors sample 1 ... || true` and exits 1. The test checks that the branch contains `exit 1` and no `exit 0`.
- **WR-07** (`error_code` range): fixed with `fits_return_window` for both the integer and the decimal-text forms. Text that overflows `i64` is refused. Boundary tests are added. A cosmetic regression in the same hunk is IN-08.
- **WR-08** (identifier grammars): fixed. The manifest target id and configuration name now start with `[a-z0-9]`, matching `argv0_is_well_formed` (`vector.rs:210-231`) and `is_module_name` (`policy.rs`). The manifest's remaining restrictions (no `-` in target ids) are stricter than the other two grammars, which is safe.
- **WR-01, partly resolved**: `check_corpus_shape` and the `sha256(stored) == key` check close the traversal, stray-file, symlink, unknown-configuration, renamed-file and unvalidated-manifest parts. Re-verified against `replay.rs:273-340` and `372-440`. The remaining part is the new WR-01.
- **WR-02, partly resolved**: `listing_failed` makes an empty listing a failure for `build`, `run` and `adapters`. The remaining part is the new WR-02.
- **WR-06, mostly resolved**: `joins_or_comments` refuses a trailing backslash, `/*`, and `//` outside a quote. Refusing an absolute `#include` was rightly skipped. What remains is only IN-09.

## Warnings

### WR-01: Replay still does not compare most provenance fields, so an edited provenance record or a changed policy is not detected

**File:** `crates/mt-harness/src/replay.rs:312-355`
**Issue:** The replay loop compares only `driver_elf`, `driver_source`, `archive` and `objects` with `build.json`. The `Provenance` record (`mt-vectors/src/corpus.rs:67-100`) also holds `policy_sha256`, `image`, `executor_version`, `contract_version`, `budgets`, `seed`, `scalars`, `outcome`, `has_ub_source`, `unobserved`, `discarded_output_bytes` and `replay`. None of these is re-derived or compared during replay.

- The re-run is driven by the vector's own `argv` and `stdin`, so a vector can be edited together with its `seed` and `scalars` while `driver_elf`, `archive` and `objects` stay intact, and replay still reports `identical`.
- A policy file that changed after the corpus was written is never noticed, because `policy_sha256` is never compared.
- A corpus produced by a different image or executor version replays as if it were current.
- `provenance.outcome` is not checked against the vector's own `stderr` class word. The doc comment on the field says they are equal.

The replay is described (CLI help, CI comments, T-03-17, T-03-31) as the tamper check for the sample, so these fields are evidence that is carried but never verified. `dir` is also not tied to the index fields: a corpus moved to another `<target>/<config>/<entry>` path replays without complaint.
**Fix:** Add to the field list in `replay_corpus`:
```rust
("image", provenance.image == loaded.build.image),
("executor_version", provenance.executor_version == EXECUTOR_VERSION),
("contract_version", provenance.contract_version == CONTRACT_VERSION),
("budgets", provenance.budgets == ProvenanceBudgets::from(&budgets)),
("policy_sha256", provenance.policy_sha256 == policy_sha256_of(&options.scope, &index.target)),
("outcome", Some(provenance.outcome.as_str()) == outcome_word(&vector)),
```
Also compare `provenance.seed` with `sha256_hex(&seed)` and `provenance.scalars` with `loaded.adapter.scalars_by_name(&combination)`. Check that `dir` ends in `<target>/<config>/<entry>`. The fix report notes that this needs the policy path plumbed into `ReplayOptions`, and that it must be validated against the CI sample replay.

### WR-02: A listing whose rows are all `no_roles` still prints `result: passed` and exits 0

**File:** `crates/mt-cli/src/main.rs:950-965, 1104-1135`; `crates/mt-harness/src/run.rs:275-277, 889-891`
**Issue:** `listing_failed` fixes the zero-row case, but `BuildRow::is_failure` treats `NoRoles` as not a failure. `RunRow::is_failure` does the same for `RunState::NoRoles`. If every selected entry is `no_roles` (ids that match only role-less entries, or role files removed), the rows are non-empty and none fails. The command then prints `result: passed (N rows)` while nothing was built or run, and `render_*_listing` hides the rows behind a single `no_roles: N` line. CI gates on the exit code of these commands. The `adapters --check-floor` floor is a separate gate and does not cover `build` or `run`.
**Fix:** Count the rows that did work:
```rust
let worked = rows.iter().any(|r| matches!(r.state, BuildState::Built { .. }));
listing_failed(worked as usize, rows.iter().any(BuildRow::is_failure))
```
For `run`, use `RunState::Ran { .. }`. Keep the printed row count as `rows.len()`.

### WR-03: `struct mt_ksigaction` does not match the kernel `sigaction` layout (uninitialised padding becomes the signal mask)

**File:** `crates/mt-harness/src/runtime.c:257-262, 292-296`
**Issue:** Unchanged and still open. The fix was deliberately skipped, because editing `runtime.c` changes every `driver_source` and `driver_elf` hash recorded in the committed sample. The ARM kernel `struct sigaction` is `{handler, flags, restorer, sigset_t mask[2 x u32]}`, with the mask at offset 12. The driver declares `unsigned long long mask`, which AAPCS aligns to 8, so the member sits at offset 16 and bytes 12-15 are uninitialised stack padding. `act.mask = 0` initialises the member, not the padding. The kernel (and qemu-user) read `sa_mask.sig[0]` from that padding, so the set of signals blocked while the handler runs is stack garbage. The handler only calls `exit_group`, so the visible effect is small, but the layout is wrong and the value is nondeterministic.
**Fix:** Declare `unsigned long mask[2];` (offset 12, 8 bytes) and zero both words, or `memset(&act, 0, sizeof act)` before filling the struct. Regenerate `testbed/vectors-sample` in CI in the same change.

## Info

### IN-01: `MAP_FIXED` mappings silently replace whatever is already at the address

**File:** `crates/mt-harness/src/runtime.c:129, 177, 787`
**Issue:** `MT_MAP_RW_FIXED` is `MAP_PRIVATE|MAP_FIXED|MAP_ANONYMOUS`. If a target's static image (`.data`/`.bss`, the 1 MiB heap arena, the snapshot buffers) reaches the fixed bases from `0x04000000`, the mapping overwrites the driver's own memory without error, which gives wrong vectors instead of a driver failure (exit 98). The existing `_Static_assert` at line 108 covers only the output mapping against `MT_STATE_BASE`. There is none for the input mapping (`MT_IN_BASE` plus its pages) against `MT_OUT_BASE`, and `mt_fields[1].len` is bounded only through `MT_REQ_MAX` (`MT_MAX_INPUT + 4096`), not by `MT_MAX_INPUT`.
**Fix:** Use `MAP_FIXED_NOREPLACE` (`0x100000`) so a collision returns an error and exits 98. Add `_Static_assert(MT_IN_BASE + (MT_MAX_INPUT / MT_PAGE + 3u) * MT_PAGE <= MT_OUT_BASE, ...)`. Bound `mt_fields[1].len` by `MT_MAX_INPUT` in `mt_main`, as `out_cap` already is.

### IN-02: The contract document describes two behaviours the code does not have

**File:** `docs/specs/equivalence-contract.md:59, 64`
**Issue:** (a) Line 59 says a return wider than 32 bits goes to `lib_state_out.ret` with `rc` 0. The code now refuses such entries in both paths: by spelling (`UnsupportedReturn`) and, since 6c58482, by DWARF size (`resolved_return_kind`). No wide return is ever recorded. (b) Line 64 says control characters are written as `\u00XX`. `serde_json` writes `\n`, `\t`, `\r`, `\b` and `\f` as short escapes. The sha256 of a vector is the hash of those exact bytes, so an independent reader who follows the document computes different file names.
**Fix:** State that integer and enum returns wider than 32 bits are refused (not recorded) until `ret` recording exists, and state the actual serialiser rules (or refer to `serde_json::to_vec_pretty` plus the trailing newline).

### IN-03: Canonical form depends on `serde_json`'s sorted `Map`, and a few `[]` indexings rely on earlier checks without a comment

**File:** `crates/mt-vectors/src/state.rs:222, 330-343`; `crates/mt-harness/src/sample.rs:143`
**Issue:** `StateValue::to_json` output order, the pointer key match `["buffer", "offset"]` and the sha256 file names all assume `serde_json::Map` is a `BTreeMap`. Enabling the `preserve_order` feature anywhere in the dependency graph would change every file name and reject valid pointer objects. The indexings `object[*shape]`, `map["buffer"]`, `map["offset"]` and `corpus.index.vectors[sha]` cannot panic today, but the project rule asks for an invariant comment where a panic is possible.
**Fix:** Add a test that fails if `serde_json::Map` is not sorted (insert `b` then `a` and compare the order). Add one-line invariant comments, or use `.get()` with an error.

### IN-04: Duplicated helpers

**File:** `crates/mt-harness/src/driver.rs:90-105` vs `crates/mt-harness/src/link.rs:174-202`; `crates/mt-cli/src/main.rs:1000-1009` vs `1272-1281`
**Issue:** `build_driver` repeats the health test and the stderr excerpt that `link::healthy` and `link::failure` implement. `selftest_command` repeats `exec_context` (pins load, validate, tools, runner config). Two copies of a "did the tool succeed" test can drift apart (the `driver.rs` copy already omits `signal`).
**Fix:** Use `link::healthy` and `link::failure` in `driver.rs`, and call `exec_context` from `selftest_command`.

### IN-05: The image has no default non-root user

**File:** `container/Dockerfile:510-529`
**Issue:** The final stage (`FROM apt-base AS final`) sets no `USER`. Every CI invocation passes `--user 65534:65534`, but anyone running the image directly gets root, so the workflow's hardening is a property of the callers.
**Fix:** Add `USER 65534:65534` to the final stage (the tools are world-readable via `chmod -R a+rX`), or document why not.

### IN-06: Recompile and flag handling is a denylist while the driver flags are an allowlist

**File:** `crates/mt-harness/src/probe.rs:258-286`; `crates/mt-harness/src/adapter.rs:1273-1277`
**Issue:** `driver_flags` fails closed on unknown `-` options. `recompile_arguments` keeps every option except `-o`, `-M*`, `-O*` and `-g*`, so `-fplugin=`, `-specs=`, `-wrapper` or `@file` from an upstream compile command reach the pinned compiler. In `driver_flags`, an `@file` argument does not start with `-` and is dropped as "the source file", so its flags are lost without an error. Both paths work on canonical commands from a verified capture, so the risk is low, but they should agree.
**Fix:** Reuse `driver_flags` (or the same allowlist) in `recompile_arguments`, and make `driver_flags` return `UnhandledFlag` for any argument that starts with `@`.

### IN-07: Policy loader silently ignores `.toml` files whose stem is not valid UTF-8

**File:** `crates/mt-contract/src/policy.rs:450-458`
**Issue:** `load_dir` skips a `*.toml` whose `file_stem().to_str()` is `None` (the `let` chain at line 454), so a stray non-UTF-8 policy file is neither accepted nor reported, although the stated rule is "the set of `*.toml` files must equal the ids".
**Fix:** Push an issue for a `.toml` entry whose stem is not UTF-8.

### IN-08: The WR-07 fix split a doc comment: `is_decimal_integer` lost its doc and `fits_return_window` carries a wrong first line

**File:** `crates/mt-contract/src/policy.rs:543-553`
**Issue:** The new helper was inserted between the existing doc line `/// An optional leading '-' and at least one decimal digit.` and the function it described. `fits_return_window` now has a two-part doc whose first line describes the other function, and `is_decimal_integer` has no doc. `cargo doc` and readers get the wrong description. It is cosmetic and not caught by clippy.
**Fix:** Move the `/// An optional leading '-' ...` line down to sit directly above `fn is_decimal_integer`, and leave only the "Whether `n` fits the 32-bit return convention" doc on `fits_return_window`.

### IN-09: Include-prefix hardening is incomplete: trigraph splice, and the probe path is not covered

**File:** `crates/mt-harness/src/driver.rs:121-160`; `crates/mt-harness/src/probe.rs:66-69`; `crates/mt-harness/src/run.rs:598-635`
**Issue:** Two small gaps remain after the WR-06 fix.

- (a) `joins_or_comments` detects a trailing `\` but not the trigraph `??/` at the end of a line. The customer's flags are passed through, so with a strict `-std=c99` (trigraphs on) `??/` splices the next line exactly as a backslash does. A `/*` inside a string literal (for example `#define GLOB "/usr/*"`) is also refused, which is safe but turns a valid entry into a refusal.
- (b) `is_prefix_line` runs only in `render_entry_driver`. In `build_entry`, `layout_for_entry` writes the same `include_lines` into `probe.c` (`render_probe_counted`, no validation) and compiles it before the driver check runs. The probe is compile-only, so the exposure is small, but the two sinks do not share one check, although the doc comment says the lines are checked "the same way".

**Fix:** Also reject lines that contain `??/`. Validate the lines once, in `include_lines()` in `run.rs`, before the probe, so both sinks see only checked lines.

---

_Reviewed: 2026-11-01_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
