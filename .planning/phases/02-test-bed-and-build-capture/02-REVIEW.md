---
phase: 02-test-bed-and-build-capture
reviewed: 2026-10-21T00:00:00Z
depth: deep
files_reviewed: 16
files_reviewed_list:
  - .github/workflows/container.yml
  - .github/workflows/testbed.yml
  - crates/mt-capture/src/capture.rs
  - crates/mt-capture/src/cc.rs
  - crates/mt-capture/src/report.rs
  - crates/mt-capture/src/testbed.rs
  - crates/mt-capture/tests/build_01_capture_drivers.rs
  - crates/mt-capture/tests/build_01_capture_stages.rs
  - crates/mt-capture/tests/build_01_compile_commands.rs
  - crates/mt-capture/tests/build_01_gate_bypasses.rs
  - crates/mt-capture/tests/build_01_toolchain_evidence.rs
  - crates/mt-capture/tests/build_02_replay.rs
  - crates/mt-capture/tests/build_02_replay_bypasses.rs
  - crates/mt-capture/tests/build_02_workflow_static.rs
  - crates/mt-cli/src/main.rs
  - crates/mt-cli/tests/build_01_capture_stages_cli.rs
findings:
  critical: 0
  warning: 5
  info: 13
  total: 18
status: issues_found
---

# Phase 2: Code Review Report (deep re-review after gap-closure plans 02-21 and 02-22)

**Reviewed:** 2026-10-21
**Depth:** deep
**Files Reviewed:** 16 (the diff `00e3415..1abac14`). Carried findings whose files are not in this diff keep the file references of the review that raised them; those files were not re-read.
**Status:** issues_found

## Summary

This is an incremental deep re-review after the two gap-closure plans that targeted the blockers of the previous review:

- 02-21 replaced the deny-list compile-command gate with an allow-list gate built on one operand-aware argument walk (`cc::walk_arguments` over `cc::ALLOWED_OPTIONS`, through `CcPolicy::check_command`), shared by the compile-only test, `derive_output` and `check_write_paths`, at capture (`canonical_entry`) and at replay (`check_entry`). It also added the WR-13 `-specs=` allow-list (`SHIPPED_SPECS`, measured in the pinned image).
- 02-22 added the read-only toolchain mount (WR-12) and the `toolchain_unreadable` cause (WR-14).

**Adversarial result on the three previous blockers: all three are genuinely fixed, at both ends.** I traced every bypass class the previous review and this task named against the current `walk`:

- Long `--<name>` spellings (CR-01): `--plugin=`, `--opt-info-all=`, `--dump-*=`, `--profile-*=` etc. do not match any exact spelling, any joined-or-separate prefix (`-o`/`-I`/`-MF`/`-MT`/`-MQ` — none is a prefix of `--…`), `-specs`, `-D`, or `-flto`/`-fprofile-`, so they reach the fall-through `OptionNotAllowed`. Refused.
- Assembler/linker/preprocessor carriers (CR-02): `-Wa,…`, `-Xassembler`, `--for-assembler=…`, `-Wl,…`, `-Xlinker`, `-Wp,…`, `-Xpreprocessor` all hit `OptionNotAllowed`. Refused.
- The `-c`-as-operand trick (CR-03): `-MT`/`-MQ`/`-MF`/`-o`/`-I` consume the next argument as their operand in the same walk GCC uses, so `-MD -MT -c …` reads `-c` as the `-MT` target and `compile_only` stays false → `NotCompileOnly`. Refused. `check_entry` now runs the same `check_command`, so the compile-only invariant holds at replay too (IN-13 closed).
- Response files in operand position (`@file`): the walk refuses any argument after `argv[0]` that starts with `@`, operands included, before option parsing. Refused.
- `-specs=` and `--specs=`: only bare `SHIPPED_SPECS` names without `/` pass joined `-specs=`; `--specs=…` (second char differs), `-specs <name>` (separate), `-specsX`, any unshipped or slash-bearing name → `helper_option`/`option_not_allowed`. Refused.
- Case/`=` variants and joined vs separate spellings: GCC-matching for the five separate-operand options, and fail-closed for everything else; the walk never continues past an argument it cannot classify, so it never guesses what follows an unknown option. I found no operand/option desync against GCC's own argument model.

I did not run live compiler reproductions this round (the previous review and both plans' SUMMARYs already recorded live CI and host reproductions, including a non-vacuous live bypass table and a live read-only-mount control). The gate is now an allow-list with no reachable escape I could construct.

WR-12 is fixed for the production path: in the testbed workflow the toolchain file is written by a step that runs no upstream code, mounted read-only outside every writable mount, checked against the pins before configure, and required at record (`--read-only-toolchain` on both record lines, pinned by `build_02_record_steps_require_the_read_only_toolchain`). WR-14 is fixed: `check_toolchain_file` has no error path and an unreadable toolchain file or cache is the `toolchain_unreadable` cause. WR-13 is fixed with the CI-measured `SHIPPED_SPECS`.

**CLAUDE.md compliance (changed files):** no `unsafe`, no `std::process::Command`, no `unwrap`/`expect`/`panic!`/`todo!`/`unreachable!` in the changed `src/`; `#![forbid(unsafe_code)]` present; `thiserror` used in the library and `anyhow` only in `mt-cli`; the new report field is deterministic (`serde(default, skip_serializing_if)`), tables are const arrays, and the new cause/finding helpers are pure. No violation found in this diff.

No new critical issue. The findings below are the previous findings re-checked, plus two low-severity observations on the new code.

## Previous Findings (still open)

| ID | Title | Status | Evidence |
|----|-------|--------|----------|
| WR-01 | Capture aborts the whole step on errors that come from untrusted upstream content | still open | `testbed.rs` `capture_pairs` still has `Err(error) => return Err(error.into())` for any non-`Fetch` `CaptureError` (one pair's `Io`/`InvalidInput` ends the record step and hides later pairs). 02-22 avoided *adding* to this class: `toolchain_unreadable` is a cause, and `configure_files` now skips an unreadable `CMakeCache.txt` instead of aborting. But `read_or_refuse` still turns `RunReadError::Io` into a fatal error for `check_inputs` and the other configure files. |
| WR-03 | Normalised comparison reads the original object without checking its reported hash | still open | `replay.rs` not in this diff. |
| WR-06 | The tree digest is ambiguous for file names containing a newline | still open | `treehash.rs` not in this diff. |
| WR-07 | The documented memory bound for zip seed sources does not hold | partially fixed (deferred) | Member-count cap deferred by the founder (02-13-SUMMARY); unchanged. |
| WR-09 | Archive extraction does not check the extracted tree for escaping links | still open | `fetch.rs` not in this diff. |
| IN-01 | Capture/entry tests pass without running when the Arm tools are missing | still open | The new `build_01_capture_stages.rs` WR-12 tests use `local_arm_tools()` the same way; they are skipped silently off-CI. |
| IN-02 | The manifest allows a configuration name that capture then rejects | still open | `is_config_name` still accepts a leading `-`. |
| IN-03 | `mt testbed validate` does not validate the drop log | still open | |
| IN-04 | The truncated flag of captured streams is ignored | still open | |
| IN-05 | Stale version in a code comment | still open | |
| IN-06 | Archive download has no size bound and is read fully into memory | still open | |
| IN-08 | `fetch` accepts `file://` URLs regardless of the allowed protocols | still open | |
| IN-09 | `mt testbed compare` accepts the same tree as both sides | still open | |
| IN-10 | Replay is bound to the pin and tools but not to the configuration's effective flags | still open | `report_mismatch` still compares no flag fingerprint. The new `toolchain_file_sha256`/`toolchain_file_read_only` are recorded but `report_mismatch` is unchanged in this diff. |
| IN-11 | `read_run_file` checks then opens, and reads without a size bound | still open (wider reach) | `read_run_file` is now also used for the trusted toolchain copy and `build/CMakeCache.txt` (both build-writable in the end-state mode). The unbounded whole-file read into memory applies there too; a build that leaves a multi-gigabyte `CMakeCache.txt` is read whole by the record container. |
| IN-12 | The build notice repeats the pair name on every row | still open | |

## Warnings

### WR-01: Capture aborts the whole step on errors that come from untrusted upstream content

**File:** `crates/mt-capture/src/testbed.rs:672-679` (`capture_pairs`), `crates/mt-capture/src/capture.rs:1665-1673` (`read_or_refuse`), `:2036` (`check_inputs`)
**Status:** still open (original finding, carried; not reached by 02-22's WR-14 fix)
**Issue:** `capture_pairs` maps only `CaptureError::Fetch` to a per-pair row; any other `CaptureError` returns from the loop (`Err(error) => return Err(error.into())`). The record stage reads an untrusted run directory, and `read_or_refuse` still returns `Err(io_error(..))` for an `Io` failure (a missing or unreadable configure file, a disappeared inputs-copy file). One such pair ends the record step with a non-zero exit, writes no later `capture.json`, and hides the verdicts of every later pair. 02-22 fixed the toolchain-file instance (WR-14) and the cache instance (it now `continue`s on an unreadable cache), but the general class is unchanged.
**Fix:** As the previous review proposed — in `capture_pairs`, treat errors tied to one pair's tree (`Tree`, `Io`, `InvalidInput`, `Cc`) as that pair's `capture_failed` row and `continue`; keep `Runner`/`Json` fatal. Better, record the error as a cause inside the record stage so a `capture.json` with verdict `capture_failed` is still written.

### WR-03: Normalised comparison reads the original object without checking its reported hash

**File:** `crates/mt-capture/src/replay.rs:418-476`
**Status:** still open (carried; file not in this diff)
**Issue/Fix:** Unchanged from the previous review. `compare_differing` reads the stored original object and uses its bytes without checking `sha256(bytes) == original`. One-line guard: return a `differs` verdict with a named reason when the stored original does not match its reported sha256.

### WR-06: The tree digest is ambiguous for file names that contain a newline

**File:** `crates/mt-capture/src/treehash.rs:83-120`
**Status:** still open (carried; file not in this diff)
**Issue/Fix:** Unchanged. Reject `char::is_control` (at least `\n`) in `relative_path` and add a test.

### WR-07: The documented memory bound for zip seed sources does not hold

**File:** `crates/mt-seeds/src/zipsrc.rs:88-134`, `crates/mt-capture/src/seeding.rs:40-57`
**Status:** partially fixed, deferred (carried; files not in this diff)
**Issue/Fix:** Unchanged. The decompressed-bytes budget is correct; the member-count cap is deferred by the founder. Carried for the ledger.

### WR-09: Archive extraction does not check the extracted tree for escaping links

**File:** `crates/mt-capture/src/fetch.rs:624-640`
**Status:** still open (carried; file not in this diff)
**Issue/Fix:** Unchanged. Walk `partial` with the escaping-link rules before hashing and fail the fetch with `EscapingSymlink`.

## Info

### IN-01 through IN-12

Carried from the previous review, all still open. See the "Previous Findings (still open)" table above for the per-item status. The only change worth calling out is IN-11, whose reach widened: the record stage's `read_run_file` is now also the reader of the trusted toolchain copy and of `build/CMakeCache.txt`, so the unbounded whole-file read applies to two more build-influenced files. The fix is unchanged (open with `O_NOFOLLOW` where available and cap the read).

### IN-16: The non-`--read-only-toolchain` record path and the single-process `capture()` keep the old end-state-only WR-12 weakness

**File:** `crates/mt-capture/src/capture.rs:2106-2146` (`check_toolchain_file`, the `else` end-state branch), `:2634-2642` (`capture`), `crates/mt-capture/src/testbed.rs:634-636` (`run_capture_record`)
**Issue:** WR-12 is closed for the testbed workflow because both record lines pass `--read-only-toolchain` and the mount is read-only. But `capture_record` without the flag (and the in-process `capture()` used by `run_capture`, i.e. `mt testbed capture` with no `--stage`) still falls back to the end-state comparison of `stage/toolchain.cmake` when no trusted copy exists. That comparison sees only the file's final bytes, so a build that changes the toolchain file, lets CMake read it, and restores it is not detected — the exact residual the previous WR-12 described. The resulting report carries `toolchain_file_read_only: false`, so a consumer *can* tell the weaker guarantee apart, and the module doc states the limitation. This is an evidence-integrity residual on a non-production path, not a new escape.
**Fix:** None required for the shipped pipeline; if the single-process path is ever used for evidence, either require the read-only mode there too, or make `report_mismatch`/downstream consumers refuse a `toolchain_file_read_only: false` report as not-proven for the toolchain fingerprint.

### IN-17: The read-only-mount control step claims "run_build's docker flags" but does not replicate them all

**File:** `.github/workflows/container.yml` (step "Prove a read-only toolchain mount cannot be written (WR-12)")
**Issue:** The control runs `docker run` with `--read-only`, `--cap-drop ALL`, `--security-opt no-new-privileges`, a small `--tmpfs /tmp`, and the `:ro` toolchain mount, and its comment says it uses "run_build's docker flags". It omits `--platform linux/amd64`, `--pids-limit`, `--memory`/`--memory-swap`, `--cpus`, and the `exec`/`size=2g` on `/tmp`. The read-only conclusion depends only on `--read-only` plus the `:ro` mount, so the proof is valid, but the "run_build's docker flags" wording overstates the fidelity of the control to the real build container.
**Fix:** Either reproduce run_build's full flag set in the control, or soften the comment to say it uses the mount flags that bear on writability (`--read-only` and `:ro`), not the complete run_build invocation.

## Resolved since last review

| ID | Title | Resolution |
|----|-------|------------|
| CR-01 | GCC long-option spellings bypass the gate | Fixed (02-21). Allow-list `walk`: a `--<name>` matches no exact spelling, no joined-or-separate prefix, no `-specs`/`-D`/`-flto`/`-fprofile-`, so it reaches `OptionNotAllowed` at both ends (`build_01_gate_refuses_long_option_spellings`, `build_02_replay_refuses_the_measured_bypasses`). |
| CR-02 | Assembler carrier deny-list misses GNU `as` spellings | Fixed (02-21). All carriers (`-Wa,`, `-Xassembler`, `--for-assembler=`, and the linker/preprocessor carriers) are `OptionNotAllowed` (`build_01_gate_refuses_assembler_carrier_spellings`). |
| CR-03 | Compile-only check accepts a `-c` read as an operand | Fixed (02-21). Compile-only is decided by the one operand-aware walk, so `-MT -c` reads `-c` as the target and gives `NotCompileOnly`; `check_entry` runs the same `check_command` (`build_01_gate_decides_compile_only_from_the_walk`). |
| WR-12 | Recorded toolchain-file hash recomputed, never compared with the file the build used | Fixed for the production path (02-22). Read-only mount written by a no-upstream-code step, checked against the pins before configure, required at record; the record stage compares the trusted copy and sets `toolchain_file_read_only: true`. Live control `toolchain-ro-control` and the corejson report confirm it. End-state fallback residual tracked as IN-16. |
| WR-13 | The gate refuses `-specs=` names normal Cortex-M builds use | Fixed (02-21). `SHIPPED_SPECS` (13 names measured in the pinned image) allowed for joined `-specs=<name>` without `/`; everything else refused (`build_01_specs_allowed_only_for_shipped_names`, `build_01_shipped_spec_names_are_read_from_the_toolchain`). The over-refusal of unbuilt-target options (WR-13 part 2) was left open by founder decision; refusing is the safe direction and all local entries pass. |
| WR-14 | `check_toolchain_file` turns an unreadable file into a fatal error | Fixed (02-22). `check_toolchain_file` and `check_configured_toolchain` have no error path; an unreadable toolchain file or cache is the `toolchain_unreadable` cause, `configure_files` skips an unreadable cache (`build_01_unreadable_toolchain_file_is_a_named_cause`). |
| WR-11 | `built: false` with no cause yields a `captured` report | Fixed (02-20); still holds — the record stage's inconsistency check and `settle_verdict` are unchanged in this diff. |
| WR-02, WR-04, WR-05, WR-08, WR-10 | (see prior review) | Fixed earlier; unchanged by this diff. |
| IN-13 | Replay's `check_entry` repeated only part of the capture gate | Fixed (02-21). `check_entry` now calls the same `check_command` (compile-only, forbidden-flag and write-path checks included). |
| IN-14 | Three hand-kept option tables must agree | Fixed (02-21). One table (`ALLOWED_OPTIONS`) and one walk (`walk`/`walk_arguments`) serve the compile-only test, `derive_output` and `check_write_paths`. |
| IN-15 | Some refusal evidence was not live or not in CI | Addressed (02-21). Live bypass table (`build_02_replay_bypasses`) with positive controls runs in CI; measured bypass spellings are pure tests. |
| IN-07 | `isotp` dropped by probe design | Fixed earlier; unchanged. |

---

_Reviewed: 2026-10-21_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: deep_
