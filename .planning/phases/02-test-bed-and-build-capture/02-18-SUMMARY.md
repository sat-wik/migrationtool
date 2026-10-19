---
phase: 02-test-bed-and-build-capture
plan: 18
subsystem: testing
tags: [capture, wr-04, build-stage, record-stage, symlink-safe-reads, mt-testbed-cli, gap-closure]
requires:
  - phase: 02-test-bed-and-build-capture
    provides: "02-16 reported_tools and the 10-target floor, 02-17 resource limits and the first green full-bed run, 02-11/02-12 entry include prefix and Makefile2 normalisation"
provides:
  - "capture_build: the untrusted upstream build, writing only <work>/run/<id>/<config> (raw export, toolchain file and build record under stage/)"
  - "capture_record: the evidence stage, running no upstream code, alone writing <work>/capture/<id>/<config>"
  - "read_run_file and RunReadError: a run-directory read that never follows a symlink"
  - "BuildRecord (schema_version 1, deny_unknown_fields) and RunLayout::stage_dir, build_record_path, raw_copy_path, toolchain_copy_path"
  - "mt testbed capture --stage build|record, --config, and mt testbed pairs, with documented exit contracts"
affects: [02-19 per-pair containers and second full-bed run, 02-UAT gap re-check]
actuals:
  tokens: 17000
  tasks: 2
  commits: 6
plan_head_before: 9aab9db3e1d975349ad8a4a11d0effcee5040829
plan_head_after: ca478ae9a1478b4c9092751c4eb2ffa39239a665
tech-stack:
  added: []
  patterns:
    - "Untrusted build output is read only through a function that lstat-checks every component from the run root down and opens regular files only"
    - "The evidence stage recomputes everything it can from the pins, the committed testbed/ files and the verified cache, and takes from the untrusted side only the build's own outputs and a record of the build steps"
    - "capture() is the two stages in one process, so the single-process path and the containerised path cannot diverge"
key-files:
  created:
    - crates/mt-capture/tests/build_01_capture_stages.rs
    - crates/mt-cli/tests/build_01_capture_stages_cli.rs
  modified:
    - crates/mt-capture/src/capture.rs
    - crates/mt-capture/src/testbed.rs
    - crates/mt-cli/src/main.rs
    - crates/mt-capture/tests/build_01_capture_drivers.rs
key-decisions:
  - "The record stage writes toolchain.cmake into the capture directory from the text it recomputes from the pins and the configuration (not the build's file), so a staged capture keeps the same capture artifacts as before"
  - "A symlink where a declared object should be counts as present for the existence test, so the record stage refuses it loudly (run_dir_symlink) instead of dropping the entry as unbuilt"
  - "Report steps are now the build record's steps, then the record stage's cache-verification steps, then readelf and entry steps (the plan's order); the build stage's own verification steps are not recorded"
  - "read_run_file splits the relative path on '/' itself because Path::components normalises interior '.' away"
requirements-completed: [BUILD-01, BUILD-02]
coverage:
  - id: D1
    description: "WR-04: the build stage writes only the pair's run directory and never creates or reads the capture directory, reports or out directory; an existing run root is emptied in place"
    requirement: BUILD-01
    verification:
      - kind: integration
        ref: "crates/mt-capture/tests/build_01_capture_stages.rs#build_01_build_stage_writes_only_the_run_directory"
        status: pass
      - kind: integration
        ref: "crates/mt-capture/tests/build_01_capture_stages.rs#build_01_build_stage_keeps_an_existing_run_root"
        status: pass
    human_judgment: false
  - id: D2
    description: "The record stage reads the run directory without following symlinks (build record, raw export, objects, configure-time files, inputs copy); refused build record is not_proven no_build_record, any other refusal is capture_failed run_dir_symlink"
    requirement: BUILD-01
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/build_01_capture_stages.rs#build_01_run_dir_reads_refuse_symlinks"
        status: pass
      - kind: integration
        ref: "crates/mt-capture/tests/build_01_capture_stages.rs#build_01_record_stage_does_not_follow_symlinks"
        status: pass
    human_judgment: false
  - id: D3
    description: "The record stage trusts nothing the build could change beyond its own outputs: missing, unreadable or mismatched build record is not_proven; a differing inputs copy is capture_failed inputs_changed; home and tmp are created when missing"
    requirement: BUILD-02
    verification:
      - kind: integration
        ref: "crates/mt-capture/tests/build_01_capture_stages.rs#build_01_record_stage_without_a_build_record_is_not_proven"
        status: pass
      - kind: integration
        ref: "crates/mt-capture/tests/build_01_capture_stages.rs#build_01_record_stage_refuses_a_record_for_another_pair"
        status: pass
      - kind: integration
        ref: "crates/mt-capture/tests/build_01_capture_stages.rs#build_01_record_stage_refuses_changed_inputs"
        status: pass
    human_judgment: false
  - id: D4
    description: "capture() is capture_build then capture_record, and a staged capture gives capture artifacts identical to a single-process one for a Make (Bear) and a CMake target"
    requirement: BUILD-01
    verification:
      - kind: integration
        ref: "crates/mt-capture/tests/build_01_capture_stages.rs#build_01_staged_capture_equals_the_single_process_capture"
        status: pass
      - kind: unit
        ref: "cargo test -p mt-capture (all existing capture, entry, replay, drop-log and fixture tests)"
        status: pass
    human_judgment: false
  - id: D5
    description: "mt testbed capture --stage build|record, --config and mt testbed pairs, with exit contracts documented in the help and tested"
    requirement: BUILD-01
    verification:
      - kind: integration
        ref: "crates/mt-cli/tests/build_01_capture_stages_cli.rs (7 tests)"
        status: pass
    human_judgment: false
  - id: D6
    description: "Per-pair containers that mount only the run directory writable, proving the separation on the whole bed (plan 02-19)"
    verification: []
    human_judgment: true
    rationale: "This plan makes the separation possible and tests it offline; the container enforcement and the full-bed proof belong to plan 02-19"
duration: 12min
completed: 2026-10-19
status: complete
---

# Phase 2 Plan 18: Capture in two stages (WR-04) Summary

**Capture is split into `capture_build`, which runs the untrusted upstream build and can write only its pair's run directory, and `capture_record`, which runs no upstream code, reads the run directory through a never-follow-symlinks reader, re-derives everything it can (cache check, tools, inputs, toolchain file) and alone writes the evidence replay trusts; `mt testbed capture --stage build|record`, `--config` and `mt testbed pairs` expose the stages for plan 02-19's per-pair containers.**

## Performance

- **Duration:** 12 min
- **Started:** 2026-10-19T22:08:24Z
- **Completed:** 2026-10-19T22:21:00Z
- **Tasks:** 2 (tracer, then CLI task)
- **Files modified:** 6 (2 created)
- **Sizing note:** the plan's escape hatch (split Task 2 into its own plan) was not needed; both tasks ran in this plan.

## Accomplishments

- Build stage (`capture_build`): empties an existing run root in place (same inode, so a bind mount survives), makes `stage/`, copies the verified source and committed inputs, runs the build exactly as before with the raw export and CMake toolchain file in `stage/`, and writes `stage/build-record.json`. Nothing is created, read or written under `<work>/capture`, reports or the out directory.
- Record stage (`capture_record`): creates `<work>/home` and `<work>/tmp`, verifies the cache again, then replaces the capture directory; reads the build record (no_build_record / build_record_mismatch are `not_proven`), compares the run copy of the inputs with `testbed/targets/<id>` (`inputs_changed`), recomputes the toolchain file hash from the pins, then runs canonicalise, gates, entry checks and configure-file hashing as before.
- `read_run_file(run_root, relative)` refuses an absolute path, an empty path or any `.`/`..`/empty component, checks the run root and each component with `symlink_metadata` (directories real, last component a regular file), so symlinks, directories, sockets and FIFOs are refused with the offending path.
- `capture()` is `capture_build` then `capture_record`; the staged and single-process capture directories compare identical (`compare_capture_dirs`) for a Make and a CMake toy target.
- CLI: `--stage build --config <NAME> <ID>` (one pair; prints `<id>/<config> built|build_failed|not_proven` or `dropped (<cause>)`), `--stage record` (the record stage over the selection with today's summary, reports and variant comparison), `mt testbed pairs` (retained pairs, default first). `run_capture` and `run_capture_record` share one loop.

## Task Commits

1. **Tracer RED** - `b2cf8ba` (test) - `build_01_capture_stages.rs` only
2. **Tracer GREEN** - `c4d75af` (feat) - capture.rs plus the two argv-path expectations of the drivers test
3. **Tracer follow-up** - `a453028` (fix) - verify the cache before replacing the capture directory (see Deviations)
4. **Task 2 RED** - `017dd89` (test) - `build_01_capture_stages_cli.rs` only
5. **Task 2 GREEN** - `1755dd1` (feat) - testbed.rs and main.rs
6. **Task 2 coverage** - `ca478ae` (test) - two exit-contract tests added after GREEN

**Plan metadata:** committed after this file (docs: complete plan).

## TDD Gate Compliance

`gsd_run check tdd-red-evidence` does not parse `cargo test` output (no supported report format), so the evidence below is the unmodified compiler or binary output, assessed by hand, as in plan 02-17.

**Tracer RED `b2cf8ba`** (test file only, against the unchanged `capture.rs`): `cargo test -p mt-capture --test build_01_capture_stages` failed to compile with 10 errors:

```text
error[E0432]: unresolved imports `mt_capture::capture::BuildRecord`, `mt_capture::capture::RunReadError`,
  `mt_capture::capture::capture_build`, `mt_capture::capture::capture_record`, `mt_capture::capture::read_run_file`
error[E0599]: no method named `build_record_path` found for reference `&RunLayout` in the current scope   (x7)
error[E0599]: no method named `stage_dir` found for struct `RunLayout` in the current scope               (x2)
```

Semantic assessment: the errors name exactly the new API (capture_build, capture_record, BuildRecord, read_run_file, RunReadError, stage_dir, build_record_path); no other fault (no syntax, fixture or unrelated error). GREEN `c4d75af`: `8 passed; 0 failed`, run with `--nocapture`, no "skipping" line (the host Arm tools were found); `cargo test -p mt-capture` all suites ok.

**Task 2 RED `017dd89`** (CLI test file only, against the unchanged binary): `cargo test -p mt-cli --test build_01_capture_stages_cli` `0 passed; 5 failed`, each on the planned assertion:

| Test | Observed (unchanged binary) |
|---|---|
| `build_01_capture_stage_build_needs_one_pair` | `error: unexpected argument '--stage' found` (tip: `--image`), exit 2 without the rule text |
| `build_01_capture_config_needs_stage_build` | `error: unexpected argument '--config' found`, exit 2 without `--config is only valid with --stage build` |
| `build_01_pairs_lists_retained_pairs_default_first` | `error: unrecognized subcommand 'pairs'`, exit 2 (left `Some(2)`, right `Some(0)`) |
| `build_01_capture_stage_build_of_a_dropped_pair_does_nothing` | `unexpected argument '--stage'`, exit 2 (left `Some(2)`, right `Some(0)`) |
| `build_01_capture_stage_record_without_build_records_fails` | `unexpected argument '--stage'`, exit 2 (left `Some(2)`, right `Some(1)`) |

Semantic assessment: the binary ran and refused the new arguments and subcommand; the first two tests also reached their message assertions. GREEN `1755dd1`: `5 passed`. The two tests of `ca478ae` were added after GREEN as coverage of documented exit codes (cache missing exits 1 and starts no build; unknown configuration or target exits 2); they passed on first run and the exit-1 test was mutation-checked (changing the Fetch arm's `Ok(false)` to `Ok(true)` failed it with left `Some(0)`, right `Some(1)`; the change was reverted). No REFACTOR commit.

## Existing tests changed

Only `crates/mt-capture/tests/build_01_capture_drivers.rs`, two path expectations moved to the staged layout (GREEN commit `c4d75af`):

- `build_01_cmake_configure_never_sets_cmake_c_flags`: `-DCMAKE_TOOLCHAIN_FILE=/work/capture/corejson/depth-8/toolchain.cmake` -> `/work/run/corejson/depth-8/stage/toolchain.cmake`
- `build_01_make_argv_injects_flags_through_cc_and_runs_j1`: Bear `--output` `/work/capture/qcbor/no-hw-float/compile_commands.raw.json` -> `/work/run/qcbor/no-hw-float/stage/compile_commands.raw.json`

No assertion about a gate, a verdict, a hash, a step list or a cause was changed or loosened. `build_01_capture_cmake_end_to_end` still reads `capture/<...>/toolchain.cmake` and passes because the record stage writes the recomputed toolchain text there.

## Files Created/Modified

- `crates/mt-capture/src/capture.rs` - module-doc trust model; stage directory layout; `BuildRecord`; `RunReadError` and `read_run_file`; `capture_build`, `capture_record`, `capture`; session split into verify / prepare_run / prepare_record / check_inputs / load_build_record / copy_raw_export; symlink-safe reads in gates and configure_files
- `crates/mt-capture/src/testbed.rs` - `run_pairs`, `run_capture_build`, `run_capture_record`, the shared `capture_pairs` loop, `TestbedError::UnknownConfig`
- `crates/mt-cli/src/main.rs` - `--stage`, `--config`, `pairs`, exit contracts in the doc comments
- `crates/mt-capture/tests/build_01_capture_stages.rs` - eight stage tests
- `crates/mt-cli/tests/build_01_capture_stages_cli.rs` - seven CLI tests
- `crates/mt-capture/tests/build_01_capture_drivers.rs` - two path expectations

## Decisions Made

- Evidence copy of the toolchain file: the record stage writes `toolchain.cmake` into the capture directory from the text it recomputes (pins plus configuration), not from the build's file. This keeps the capture directory byte-identical to a single-process capture and means the hash cannot come from an altered file.
- Existence test: a symlink at a declared object path now counts as present (see Deviation 1).
- Step order in `capture.json`: build record steps, then the record stage's cache verification steps, then readelf and entry steps, as the plan says; the build stage's own cache verification is a fail-fast and is not recorded.
- `read_run_file` splits on `/` itself: `Path::components()` drops an interior `.`, which the plan says to refuse.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical] A symlinked object was silently dropped as unbuilt instead of refused**
- **Found during:** Tracer GREEN, while checking `build_01_record_stage_does_not_follow_symlinks` against `cc::canonicalise`
- **Issue:** `Session::canonicalise` decides whether an entry's object exists with `symlink_metadata(..).is_file()`, which is false for a symlink. A build that replaced `src/t.o` with a symlink got its entry dropped (`dropped_unbuilt`, or `empty_capture` for a one-object target) and the new refusal was never reached, contradicting the plan's expected verdict `capture_failed` / `run_dir_symlink`.
- **Fix:** the existence closure is now `is_file() || is_symlink()`, so the entry is kept and `gates` refuses the object loudly. A path that does not exist at all (CMake targets that built nothing) is still dropped as before.
- **Files modified:** crates/mt-capture/src/capture.rs
- **Verification:** `build_01_record_stage_does_not_follow_symlinks` (`causes == ["run_dir_symlink"]`, no marker bytes anywhere under the capture directory); all existing capture tests unchanged and green.
- **Committed in:** c4d75af

**2. [Rule 1 - Bug] Record stage removed the capture directory before verifying the cache**
- **Found during:** starting Task 2 (reading what `capture_record` does on a missing cache)
- **Issue:** my tracer GREEN ran `prepare_record` (remove and recreate the capture directory) before `verify_cached`, so an unverifiable cache destroyed the previous capture. `capture()` always verified first.
- **Fix:** create home and tmp, verify the cache, then replace the capture directory.
- **Files modified:** crates/mt-capture/src/capture.rs
- **Verification:** `build_01_capture_stages` 8 passed after the change; Task 2 `build_01_capture_stage_record_without_build_records_fails` and `build_01_capture_stage_build_without_a_cache_exits_one` pass.
- **Committed in:** a453028

**3. [Note] Two tests added after GREEN**
- `build_01_capture_stage_build_without_a_cache_exits_one` and `build_01_capture_stage_build_of_an_unknown_pair_exits_two` cover the "exit contracts documented and tested" truth, which the plan's five CLI tests do not (they never reach the exit-1 case). Not RED-first; see TDD Gate Compliance.

**4. [Note] RED evidence is compiler output, assessed by hand** (same as plan 02-17; no supported report format).

---

**Total deviations:** 2 auto-fixed (1 Rule 2, 1 Rule 1), 2 notes.
**Impact on plan:** deviation 1 tightens the record stage (a planted symlink is a loud failure rather than a silent drop) and was needed for the plan's own test; deviation 2 repairs a regression I introduced in the tracer. No scope creep.

## Symlink re-check (plan instruction)

`find .mt-work/run -mindepth 3 -maxdepth 3 -type d \( -name build -o -name inputs \) -exec find {} -type l \;` found **0** symlinks in the **48** `build` and `inputs` directories under `.mt-work/run`, so the never-follow rule changes no current capture. No checkpoint:decision was needed. Committed `testbed/targets/*` holds no symlinks either.

## Residual trust, recorded as the plan asks

- **Forged build record (T-02-63, accepted).** The build stage's own code runs before the record is written and can rewrite it. The record stage takes from it only the build steps' run records, the `not_proven` / `build_failed` flags, the cause lines and the `before` listing. A forged record can make its own pair read as built, failed or not proven (the build could equally have built different objects) and cannot change the source verification, the tools, the inputs check, any other pair or the replay comparison. A record for another pair or source is `not_proven` / `build_record_mismatch`.
- **Reads that still follow links, by design.** The compiler's own reads during the entry check; the defining-TU read of `include_prefix` (plan 02-11); the `__TIMESTAMP__` scan of the sources (it reads, never copies, and the copied source tree may legitimately hold symlinks, see `copy_tree`); `readelf` on an object path the record stage has already opened as a regular file. All four are stated in the `capture.rs` module doc.
- **Check-then-open window.** `read_run_file` checks with `symlink_metadata` and then opens (it re-checks `is_file` on the opened handle). It does not use `O_NOFOLLOW` (needs `unsafe` or a libc dependency); this is closed in practice because the record stage runs after the build has exited, and plan 02-19 runs it in a container where the run directory is not writable.

## Issues Encountered

None blocking. The staged equivalence test needed no change to the compare exclusions.

## Authentication Gates

None.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None.

## Next Phase Readiness

- Plan 02-19 can run `mt testbed pairs` to list the pairs, one `mt testbed capture --stage build --config <c> <id>` per pair in a container that mounts only that pair's run directory writable, then `--stage record` with the work directory mounted for the evidence, and the existing replay and compare steps.
- Things 02-19 should watch in the first full-bed round trip: (a) `inputs_changed` would flag any recipe whose build writes into `{inputs}`; (b) `run_dir_symlink` would flag a CMake build directory or object path that is a symlink (none exist locally); (c) the record stage creates `<work>/home` and `<work>/tmp` itself, so a record container needs a writable work directory; (d) the build stage's `before` listing now includes `stage/` files, which never match the object-name gate.
- Not done here and left to 02-19: `run_replay`'s `configure_evidence_changed` and the replay copy still read the run directory with ordinary file reads; the capture artifacts replay trusts are now written only by the record stage.
- Round trips: none used in this plan (02-19 owns the CI proof).

## Self-Check: PASSED

- FOUND: `crates/mt-capture/src/capture.rs`, `crates/mt-capture/src/testbed.rs`, `crates/mt-cli/src/main.rs`, `crates/mt-capture/tests/build_01_capture_stages.rs`, `crates/mt-cli/tests/build_01_capture_stages_cli.rs`
- FOUND commits `b2cf8ba`, `c4d75af`, `a453028`, `017dd89`, `1755dd1`, `ca478ae` (ancestors of HEAD); `git rev-list --count 9aab9db..HEAD` = 6
- Acceptance: eight stage tests pass with no "skipping" line; `capture.rs` contains `pub fn capture_build`, `pub fn capture_record`, `pub fn read_run_file` and `BUILD_RECORD_SCHEMA_VERSION`, and `capture()` calls both stages; `mt testbed capture --help` lists `--stage` and `--config`, `mt testbed pairs --help` exists; `cargo test -p mt-cli --test build_01_capture_stages_cli --test bed_01_testbed_cli --test build_03_seeds_cli` 7 + 3 + 5 passed
- Verification: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace --locked` (316 passed, 0 failed; 301 before plus 8 and 7 new), `cargo deny check` (advisories, bans, licenses, sources ok); the repository process-confinement test passes (no `std::process` outside `runner.rs` besides `ExitCode`)

---
*Phase: 02-test-bed-and-build-capture*
*Completed: 2026-10-19*
