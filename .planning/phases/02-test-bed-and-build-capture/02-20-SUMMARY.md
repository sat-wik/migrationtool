---
phase: 02-test-bed-and-build-capture
plan: 20
subsystem: testing
tags: [mt-capture, compile-commands-gate, gcc-driver-options, record-stage, security-register, testbed]

requires:
  - phase: 02-test-bed-and-build-capture
    provides: "02-18 build/record stage split and read_run_file; 02-19 pinned-image run_build and the 02-19 CI notices"
provides:
  - "HELPER_OPTION_PREFIXES, find_helper_option and CcError::HelperOption: driver options that run or load another program are refused at capture and at replay before any spawn"
  - "check_write_paths and CcError::WriteRedirect, with --output resolved like -o: every write path is bounded to the run root or refused, at capture and at replay"
  - "settle_verdict, build_record_inconsistent and the stage/toolchain.cmake comparison: a forged build record can no longer make its pair read captured"
  - "capture.rs module doc and AR-02-03 stating exactly what a forged record can and cannot do; T-02-67..T-02-70 registered"
  - "one green full-bed testbed run on the final code commit with notices equal to 02-19's"
affects: [phase-2-verify-work, phase-2-secure-phase, phase-3]

actuals:
  tokens: 17900
  tasks: 3
  commits: 6
plan_head_before: 39209ef1a6f132f3140414615fd017ac715acfb5
plan_head_after: 6b8480a8caf29d35da0fb981461ddb75cc5ddcad
commits: 6

tech-stack:
  added: []
  patterns:
    - "gate-at-both-ends: the same CcPolicy check runs when the export is canonicalised and again in replay's validate"
    - "a gate that was not evaluated reads as failed, and that now decides the capture verdict"

key-files:
  created:
    - crates/mt-capture/tests/build_01_capture_verdict.rs
    - .planning/phases/02-test-bed-and-build-capture/02-20-CI-NOTICES.md
  modified:
    - crates/mt-capture/src/cc.rs
    - crates/mt-capture/src/capture.rs
    - crates/mt-capture/tests/build_01_compile_commands.rs
    - crates/mt-capture/tests/build_02_replay.rs
    - crates/mt-capture/tests/build_01_capture_stages.rs
    - .planning/phases/02-test-bed-and-build-capture/02-SECURITY.md

key-decisions:
  - "Cause tokens helper_option, write_redirect, build_record_inconsistent, toolchain_changed and gates_not_passed; an inconsistent record is not_proven, like no_build_record and build_record_mismatch"
  - "Prefix matching for the nine helper prefixes; --sysroot and the other header-lookup options are not refused (measured); --prefix, -fcompare-debug=, @file and the -Wp, carrier added to the founder's examples"
  - "A trailing -o or --output is left to CcError::DanglingOutput (existing test), every other operand-less path or name option is write_redirect"
  - "T-02-68 and T-02-69 are this plan's WR-11 and WR-12 mitigations; T-02-67 and T-02-70 are the founder's two medium threats"

patterns-established:
  - "Carrier rule: a write option inside -Wp,, -Xpreprocessor, -Wa,, -Xassembler or --for-assembler is refused outright, even with a relative path, because the driver joins an operand from a separate carrier argument"

requirements-completed: []  # BED-01, BED-02, BUILD-01, BUILD-02, BUILD-03 are this plan's requirements; not marked complete here, verify-work reconciles them

coverage:
  - id: D1
    description: "G-02-6 helper options: the nine prefixes and the -Wp, carrier are refused at capture (helper_option) and at replay before any spawn; real flags and the CI fixtures pass"
    requirement: "BUILD-01"
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/build_01_compile_commands.rs#build_01_rejects_helper_executing_driver_options"
        status: pass
      - kind: unit
        ref: "crates/mt-capture/tests/build_01_compile_commands.rs#build_01_driver_option_gate_keeps_ordinary_flags"
        status: pass
      - kind: integration
        ref: "crates/mt-capture/tests/build_02_replay.rs#build_02_replay_refuses_helper_executing_driver_options"
        status: pass
      - kind: integration
        ref: "crates/mt-capture/tests/build_01_capture_stages.rs#build_01_record_stage_refuses_a_helper_option_in_the_export"
        status: pass
    human_judgment: false
  - id: D2
    description: "G-02-6 write paths: --output resolved like -o; every path-valued write option under the run root, carried and unboundable write options refused, at capture and replay"
    requirement: "BUILD-02"
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/build_01_compile_commands.rs#build_01_handles_write_path_options"
        status: pass
      - kind: integration
        ref: "crates/mt-capture/tests/build_02_replay.rs#build_02_replay_refuses_writes_outside_the_run_root"
        status: pass
      - kind: integration
        ref: "crates/mt-capture/tests/build_01_capture_stages.rs#build_01_record_stage_refuses_a_write_outside_the_run_root_in_the_export"
        status: pass
      - kind: other
        ref: "MT_LOCAL_CAPTURES=$PWD/.mt-work/capture cargo test -p mt-capture --test build_01_compile_commands -- --ignored --exact build_01_local_captures_pass_the_compile_command_gate --nocapture (files=29 raw_entries=531 canonical_entries=481 refused=0)"
        status: pass
    human_judgment: false
  - id: D3
    description: "G-02-5 record stage: a built-false record that names no failure is refused, captured needs passed gates and objects, stage/toolchain.cmake is compared with the pins"
    requirement: "BUILD-01"
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/build_01_capture_verdict.rs#build_01_captured_verdict_needs_passed_gates_and_objects"
        status: pass
      - kind: integration
        ref: "crates/mt-capture/tests/build_01_capture_stages.rs#build_01_record_stage_refuses_a_record_that_did_not_build"
        status: pass
      - kind: integration
        ref: "crates/mt-capture/tests/build_01_capture_stages.rs#build_01_record_stage_refuses_a_changed_toolchain_file"
        status: pass
    human_judgment: false
  - id: D4
    description: "G-02-7: the capture.rs module doc, AR-02-03 and the 02-SECURITY.md register (T-02-67..T-02-70, T-02-15 note) state what a forged record can and cannot do"
    verification: []
    human_judgment: true
    rationale: "Prose accuracy of a security rationale needs a reader; the grep checks only prove the rows and test names are present"
  - id: D5
    description: "One green full-bed testbed run on the final code commit: 16 fetched, 30 pairs captured and replayed identical, seeds passed, determinism identical, notices equal to 02-19's"
    requirement: "BED-01"
    verification:
      - kind: e2e
        ref: "gh run 39405529914 (testbed) success on e69a699; diff of 02-20-CI-NOTICES.md against 02-19-CI-NOTICES.md from line 5 is empty"
        status: pass
    human_judgment: false

duration: 20min
completed: 2026-10-20
status: complete
---

# Phase 2 Plan 20: Gap closure G-02-5, G-02-6, G-02-7 Summary

**The compile-command gate now refuses GCC driver options that run or load another program and every write path it cannot keep under the run root (at capture and again at replay), the record stage refuses a build record that names no failure and compares stage/toolchain.cmake with the pins, and a full-bed run in the pinned image on the final code is green with notices identical to 02-19's.**

## Performance

- **Duration:** about 20 min of wall clock for this session (one CI round trip of about 7 min inside it)
- **Started:** 2026-10-20T09:55:09Z
- **Completed:** 2026-10-20T10:20Z
- **Tasks:** 3 (Tracer, Task 2, Task 3)
- **Files modified:** 8 (7 tracked source/doc files plus the new notices file)

## Accomplishments

- G-02-6 helper options (T-02-67): `HELPER_OPTION_PREFIXES` (`-wrapper`, `-B`, `--prefix`, `-specs`, `--specs`, `-fplugin`, `-iplugindir`, `-fcompare-debug=`, `@`) plus the `-Wp,` carrier rule. `canonical_entry` refuses with cause `helper_option`; `CcPolicy::check_entry`, called by replay's `validate` for every entry before anything is deleted, copied or spawned, refuses with the same error.
- G-02-6 write paths (T-02-70): `derive_output` resolves `--output` like `-o` (the last output wins, as GCC does) and skips the operands of `-MT`, `-MQ`, `-MF` and the dump options; `check_write_paths` requires every occurrence of a path-valued write option to resolve under the run root (`outside_run_root`) and refuses carried, operand-less and unrecognised dump or aux spellings (`write_redirect`). CMake-shaped depfile flags under the run root stay accepted.
- G-02-5 (WR-11, WR-12; T-02-68, T-02-69): `settle_verdict` (captured needs no cause, both gates passed and a non-empty object set, else `gates_not_passed`), `build_record_inconsistent` for a built-false record without a named failure, and a byte comparison of `stage/toolchain.cmake` through `read_run_file` (`toolchain_changed`, `run_dir_symlink` for a link).
- G-02-7: capture.rs module doc and AR-02-03 now state what a forged record can still do and can no longer do, citing the tests; T-02-67..T-02-70 added to 02-SECURITY.md, T-02-15 and T-02-63 notes added, the "Unregistered: recorded compile flags" bullet replaced by the registered note. `threats_open` stays 0.
- Final run: testbed run 39405529914 and ci run 39405529939 are green for e69a699; 02-20-CI-NOTICES.md equals 02-19's from line 5 on (0 differing lines).

## Task Commits

1. **Tracer RED** - `850fd8b` (test)
2. **Tracer GREEN** - `aab868a` (feat)
3. **Task 2 RED** - `27908f2` (test)
4. **Task 2 GREEN** - `69adb69` (feat)
5. **Task 2 docs (module doc, 02-SECURITY.md)** - `e69a699` (docs)
6. **Task 3 notices** - `6b8480a` (docs)

**Plan metadata:** the SUMMARY/STATE/ROADMAP commit that follows this file.

`commits: 6` is measured with `git rev-list --count 39209ef..6b8480a`, before the SUMMARY commit.

## RED evidence (TDD rule)

Tracer RED (850fd8b, test files only, source unchanged):

```text
error[E0432]: unresolved imports `mt_capture::cc::HELPER_OPTION_PREFIXES`, `mt_capture::cc::check_write_paths`, `mt_capture::cc::find_helper_option`
  --> crates/mt-capture/tests/build_01_compile_commands.rs:15:44
error[E0599]: no variant named `HelperOption` found for enum `CcError`   (build_02_replay.rs:547, build_01_compile_commands.rs:359, :443)
error[E0599]: no variant named `WriteRedirect` found for enum `CcError`  (build_01_compile_commands.rs:379, build_02_replay.rs:638)
error: could not compile `mt-capture` (test "build_02_replay") due to 2 previous errors
error: could not compile `mt-capture` (test "build_01_compile_commands") due to 4 previous errors
```

`build_01_capture_stages` compiled and its two new tests failed on the verdict, with no `skipping:` line in the `--nocapture` output:

```text
thread 'build_01_record_stage_refuses_a_helper_option_in_the_export' panicked at build_01_capture_stages.rs:601:5:
assertion `left == right` failed: []   left: Captured   right: CaptureFailed
thread 'build_01_record_stage_refuses_a_write_outside_the_run_root_in_the_export' panicked at build_01_capture_stages.rs:642:5:
assertion `left == right` failed: []   left: Captured   right: CaptureFailed
test result: FAILED. 8 passed; 2 failed
```

Semantic assessment: both tests ran the real Make driver and Bear with the host Arm tools, the planted `-wrapper` and `-MD -MF <outside>/x.d` reached the record stage, and the unchanged gate let them through (verdict `captured`). That is the planned defect, for the intended reason.

Task 2 RED (27908f2):

```text
error[E0432]: unresolved import `mt_capture::capture::settle_verdict`   (build_01_capture_verdict.rs:7:5)
thread 'build_01_record_stage_refuses_a_record_that_did_not_build' panicked at build_01_capture_stages.rs:714:9:
assertion `left == right` failed: built false with no flag and no cause   left: Captured   right: NotProven
thread 'build_01_record_stage_refuses_a_changed_toolchain_file' panicked at build_01_capture_stages.rs:760:5:
assertion `left == right` failed: []   left: Captured   right: CaptureFailed
test result: FAILED. 10 passed; 2 failed
```

Semantic assessment: forgery (1) read `captured` and a toolchain file with an appended `-DPLANTED` read `captured`, exactly WR-11 and WR-12. The first failing assertion stops each test, so forgeries (2) and (3) and cases (2) to (4) of the toolchain test were first exercised at GREEN.

GREEN: all of these pass (`build_01_capture_stages` 12 passed with no `skipping:` line; `build_01_compile_commands` 15 passed and 1 ignored; `build_02_replay` 12; `build_01_capture_verdict` 1; `build_01_capture_drivers` 19; `build_01_ci_fixtures` 6).

## Local captures (fail-closed explicit run)

```text
files=29 raw_entries=531 canonical_entries=481 refused=0
```

Over `.mt-work/capture` with `MT_LOCAL_CAPTURES`, exactly the planning-time counts. The `jq`/`grep` check over the 29 canonical files found no refused prefix. The two committed CI fixtures (qcbor, corejson) pass the whole gate inside `build_01_driver_option_gate_keeps_ordinary_flags`.

Refused option set as finally implemented:

- Helper prefixes (any argument after argv[0], operands included): `-wrapper`, `-B`, `--prefix`, `-specs`, `--specs`, `-fplugin` (covers `-fplugin-arg-*`), `-iplugindir`, `-fcompare-debug=`, `@`; plus any `-Wp,` argument with a comma-separated piece starting with one (the whole `-Wp,` argument is the flag).
- Path options bounded to the run root (every occurrence): `-o`, `-MF` (separate or joined), `--output` (separate or `=`), `-dumpdir`, `--dumpdir`, `-dumpbase`, `--dumpbase` (separate only), `-aux-info` (separate or `=`), `-fdump-*=`, `-fopt-info*=` (`stdout` and `stderr` accepted as streams), `-fdeps-file=`.
- Name options, operand consumed unchecked: `-MT`, `-MQ` (separate or joined), `-dumpbase-ext`, `--dumpbase-ext`.
- `write_redirect`: a path or name option with no operand (a trailing `-o`/`--output` stays `dangling_output`); any other argument starting with `-dumpdir`, `--dumpdir`, `-dumpbase`, `--dumpbase`, `-aux-info`; a preprocessor carrier piece (`-Wp,` pieces, the operand of `-Xpreprocessor`) starting with `-M`, `-o`, `-dumpdir`, `-dumpbase`, `-aux-info`, `-fdump-`, `-fopt-info`, `-fdeps-file`; an assembler carrier piece (`-Wa,` pieces, the operand of `-Xassembler`, the value or operand of `--for-assembler`) starting with `-a`, `--MD`, `-o`, `@`.
- Accepted unchanged: `-MD`, `-MMD`, `-M`, `-MM`, `-MG`, `-MP` and long aliases, `-save-temps[=cwd|obj]`, `-fstack-usage`, `--sysroot`, `-isysroot`, `-Wa,-mthumb`, `-Xassembler -mthumb`, plain `-fcompare-debug`.

## GCC 14 documentation and driver probes

- Probes with the host `arm-none-eabi-gcc` 13.2.1 and `-###` (runs nothing), re-run before writing the doc comment:
  - `-wrapper /bin/echo,hi`: `cc1` and `as` are launched as ` /bin/echo hi /usr/lib/gcc/arm-none-eabi/13.2.1/cc1 ...` and ` /bin/echo hi .../bin/as ...`.
  - `-specs=nano.specs`: `Reading specs from /usr/lib/gcc/arm-none-eabi/13.2.1/../../../arm-none-eabi/lib/nano.specs`, then `rename spec link to nano_link`.
  - `-fplugin=/nonexistent.so`: reaches cc1 as `-fplugin=/nonexistent.so`.
- GCC docs reachability: unreachable. One `curl` of `https://gcc.gnu.org/onlinedocs/gcc-14.3.0/gcc/Overall-Options.html` through the session proxy returned `CONNECT tunnel failed, response 403` (HTTP 000); not retried. The session has no WebFetch tool, so the four pages were not read. No further option was added from GCC 14 documentation, no prefix was removed, and the doc comment on `HELPER_OPTION_PREFIXES` says the set is a measured one against 13.2 and the pinned 14.3.Rel1.
- `replay.rs` needed no change; `validate` already calls the policy for every entry before anything is deleted, copied or spawned: `plan.policy.check_entry(entry)?;` (replay.rs line 149).

## CI round trips

- **Round trips used: 1 of 3** (no extension asked). Push of `claude/gsd-discuss-phase-1-mzdw5v` at `e69a699`, which carried the Tracer and Task 2 commits.
- Final testbed run: **39405529914**, head **e69a6995b0ad09e9f84bb957b90f431994231ff3**, job 118570704581 (`testbed`), conclusion success.
- Final ci run: **39405529939**, same head, jobs `mt` and `checks` success.
- `container` run 39405529905 (triggered by crates/**, not awaited): in progress when this was written.
- A docs-only push of the notices, SUMMARY, STATE and ROADMAP commits follows; it triggers ci again, and its result is reported in the hand-back, not counted as a round trip.
- Retained count: 16 targets fetched with tree hashes equal to the manifest (16 of 16 compared programmatically), 30 pairs built, captured and replayed identical (31 replay rows), paho/server dropped as logged (`entry_mismatch`), seeds `passed (16 rows)`, determinism `identical` with `compare exit code: 0` over 74 files.
- Diff against 02-19: `diff <(sed '1,4d' 02-19-CI-NOTICES.md) <(sed '1,4d' 02-20-CI-NOTICES.md)` is empty, so the notices equal 02-19's line for line from line 5 on (image digest `sha256:e83006c0...`, object hashes, replay verdicts, seed counts, Makefile2 hashes and the determinism file count all unchanged). No real pair was refused with `helper_option`, `write_redirect`, a new `outside_run_root`, `build_record_inconsistent`, `gates_not_passed` or `toolchain_changed`.
- The notices file was assembled from the job annotations with a small script (parts of one title joined with a newline, which reproduces 02-19's layout exactly; the first attempt joined with nothing and the diff showed it).

## Closure table

| Item | Commits | Tests | CI and register evidence |
|------|---------|-------|--------------------------|
| G-02-6 helper options, T-02-67 | 850fd8b, aab868a | build_01_rejects_helper_executing_driver_options, build_01_driver_option_gate_keeps_ordinary_flags, build_02_replay_refuses_helper_executing_driver_options, build_01_record_stage_refuses_a_helper_option_in_the_export | testbed 39405529914: no retained pair refused; T-02-67 row in 02-SECURITY.md (e69a699) |
| G-02-6 write paths, T-02-70 | 850fd8b, aab868a | build_01_handles_write_path_options, build_02_replay_refuses_writes_outside_the_run_root, build_01_record_stage_refuses_a_write_outside_the_run_root_in_the_export, build_01_local_captures_pass_the_compile_command_gate (refused=0) | same run; T-02-70 row, T-02-15 note |
| G-02-5 / WR-11, T-02-68 | 27908f2, 69adb69 | build_01_captured_verdict_needs_passed_gates_and_objects, build_01_record_stage_refuses_a_record_that_did_not_build | same run (30 pairs captured, so the stricter verdict rule accepts every genuine record) |
| G-02-5 / WR-12, T-02-69 | 27908f2, 69adb69 | build_01_record_stage_refuses_a_changed_toolchain_file | same run (every CMake pair's toolchain file equals the recomputed text) |
| G-02-7 | e69a699 | the can/cannot statements cite the tests above | capture.rs module doc and AR-02-03 rewritten to match |
| Evidence current | 6b8480a | build_02_workflow_static, build_02_drop_log, build_03_committed_seeds (workspace run) | 02-20-CI-NOTICES.md equals 02-19's from line 5 |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical] derive_output skips the operands of -MT, -MQ, -MF and the dump options**
- **Found during:** Tracer (design of the decoy case `-o /out/x.o -MT -oinside.o`)
- **Issue:** The plan adds `--output` to `derive_output` but leaves its naive scan, which would read the target name `-oinside.o` as the output. The captured object would then be a file the compiler never writes.
- **Fix:** `derive_output` skips the separate operand of the options in `SEPARATE_OPERAND_OPTIONS`, matching how `check_write_paths` walks. Capture and replay stay consistent because both use `derive_output`.
- **Files modified:** crates/mt-capture/src/cc.rs
- **Verification:** build_01_handles_write_path_options (decoy case), the 29 local captures (outputs equal)
- **Committed in:** aab868a

**2. [Plan detail] a trailing -o or --output is left to DanglingOutput**
- **Found during:** Tracer GREEN design
- **Issue:** The plan says a path option without an operand is `write_redirect`, but `build_01_rejects_non_compile_commands` (a regression guard that must pass unchanged) expects `CcError::DanglingOutput` for a trailing `-o`.
- **Fix:** `check_write_paths` skips a trailing `-o`/`--output`; `derive_output` reports it as before. Every other operand-less path or name option (`-MF`, `-dumpbase`, ...) is `write_redirect`.
- **Files modified:** crates/mt-capture/src/cc.rs
- **Committed in:** aab868a

**3. [Test addition] extra cases and a tolerant raw check**
- A trailing `-dumpbase` joins the trailing `-MF` case in `build_01_handles_write_path_options`, and a first-occurrence `-MF /out/first.d -MF ok.d` case proves that every occurrence is checked.
- The ignored local-captures test treats a `DuplicateOutput` result for a raw file as "no entry refused by a gate", because that check runs after every per-entry gate; any other error counts as refused. The run printed `refused=0`.

**4. [Environment] GCC 14 pages not read**
- WebFetch is not a tool in this session and `curl` through the proxy got a 403, so the plan's GCC 14 documentation confirmation could not be done; recorded above. The plan allowed this outcome.

---

**Total deviations:** 4 (1 Rule 2 addition, 1 plan-detail reconciliation with a regression guard, 1 test addition, 1 environment limit)
**Impact on plan:** No gate was narrowed or weakened, no prefix removed, no exception added, no pair dropped.

## Known Stubs

None.

## Threat Flags

None beyond the plan's register. Adding the gates introduced no new network endpoint, auth path or file-access surface.

## Issues Encountered

- The first notices assembly joined the annotation parts without a separator and the diff against 02-19 flagged four lines; joining with a newline reproduced 02-19 exactly. No CI round trip was spent on it.
- The `container` workflow was still running when this was written.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- 02-UAT.md was not edited; `/gsd-verify-work` reconciles gaps G-02-5, G-02-6 and G-02-7 and the BED/BUILD requirements.
- `/gsd-secure-phase` should verify and close T-02-67..T-02-70 (open, below the high threshold).
- Out of scope and still open: WR-01, WR-03, WR-06, WR-09 and IN-01..IN-12 in 02-REVIEW-DISPOSITION.md.

---
*Phase: 02-test-bed-and-build-capture*
*Completed: 2026-10-20*

## Self-Check: PASSED

Created files exist (build_01_capture_verdict.rs, 02-20-CI-NOTICES.md, cc.rs changes), and commits 850fd8b, aab868a, 27908f2, 69adb69, e69a699 and 6b8480a are ancestors of HEAD.
