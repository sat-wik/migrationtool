---
phase: 02-test-bed-and-build-capture
plan: 16
subsystem: testing
tags: [replay, capture-report, manifest-pin, drop-log, floor, wr-02, wr-08]

requires:
  - phase: 02-test-bed-and-build-capture
    provides: "plan 02-12 configure_evidence_changed and the normalised Makefile2 in capture.rs/testbed.rs; plan 02-09 drop log and dropped-pair rows; plan 02-15 designated seed selector (unchanged here)"
provides:
  - "fetch::pinned_source / PinnedSource: the validated manifest commit and tree sha256 of a target"
  - "capture::reported_tools: the one tool-version map capture records and replay compares"
  - "run_replay refuses a capture report of another pair, source pin or tool set (not_proven, field named) before verify_cached or any tool; pristine_src is built from the pinned commit only"
  - "testbed::check_floor and TestbedError::BelowFloor: every Selection::All step refuses a bed retaining fewer than MIN_RETAINED (10) targets, with no override"
  - "DropLog::validate refuses a differs drop with no differing_objects or no differing_sections"
affects: [02-17 testbed workflow pinned to --all, 02-19 work-directory separation, phase-2 verification]

actuals:
  tokens: 8430
  tasks: 3
  commits: 6
plan_head_before: 4b973a011b61234fbacd3d8498d4d03761aa21d3
plan_head_after: da512c49ee1568fc9b89a2c760445beb1a7970f7

tech-stack:
  added: []
  patterns: ["compare an untrusted work-directory report with the committed pin before any path is built from it", "the floor is enforced in load_inputs, the one function every step calls, so no step can skip it"]

key-files:
  created:
    - "crates/mt-capture/tests/build_02_replay_pin.rs"
  modified:
    - "crates/mt-capture/src/fetch.rs"
    - "crates/mt-capture/src/capture.rs"
    - "crates/mt-capture/src/testbed.rs"
    - "crates/mt-capture/src/droplog.rs"
    - "crates/mt-capture/tests/build_02_drop_log.rs"
    - "crates/mt-capture/tests/build_03_seed_manifest.rs"
    - "crates/mt-cli/tests/build_03_seeds_cli.rs"

key-decisions:
  - "WR-02: the order of checks in run_replay is verdict, then pin_of validity, then pair (target, config), source.commit, source.tree_sha256, tool set, then verify_cached; a mismatch is `not_proven: the capture report was made from a different <pair|source pin|tool set>: <field>`, the tool-set field listing the differing tool keys"
  - "WR-02: reported_tools(tools, build) is the single definition of the recorded tool map; prepare in capture.rs now uses it, so capture and replay cannot drift"
  - "WR-08: the floor applies only to Selection::All; named ids are partial runs (plan 02-17 pins the workflow to --all); no flag, option or environment variable lowers it"
  - "WR-08: a differs entry reports one issue per empty field (differing_objects, differing_sections) so the message names exactly what is missing"

patterns-established:
  - "Fixture beds that must not be floored name their targets explicitly; named ids skip fully dropped targets exactly as --all does"

requirements-completed: [BUILD-02]

coverage:
  - id: D1
    description: "Replay accepts a capture report only when target, configuration, source commit, tree sha256 and tool versions equal the manifest pin and pins file; otherwise not_proven naming the field, before any cache or tool is touched (T-02-53)"
    requirement: BUILD-02
    verification:
      - kind: integration
        ref: "crates/mt-capture/tests/build_02_replay_pin.rs#build_02_replay_refuses_a_capture_from_another_pin"
        status: pass
      - kind: integration
        ref: "crates/mt-capture/tests/build_02_replay_pin.rs#build_02_replay_refuses_a_capture_made_with_other_tools"
        status: pass
      - kind: integration
        ref: "crates/mt-capture/tests/build_02_replay_pin.rs#build_02_replay_refuses_a_report_for_another_pair"
        status: pass
      - kind: integration
        ref: "crates/mt-capture/tests/build_02_replay_pin.rs#build_02_replay_checks_the_pin_before_the_cache"
        status: pass
    human_judgment: false
  - id: D2
    description: "The pristine source of a replay is the cache of the manifest's pinned commit; no value of capture.json becomes a path, and a traversal commit is a pin mismatch (T-02-54)"
    requirement: BUILD-02
    verification:
      - kind: integration
        ref: "crates/mt-capture/tests/build_02_replay_pin.rs#build_02_replay_refuses_a_capture_from_another_pin"
        status: pass
    human_judgment: false
  - id: D3
    description: "An --all run (fetch, capture, replay, seeds) over fewer than 10 retained targets, including zero, is refused with the retained count and floor, writes no summary, and has no override (T-02-55)"
    requirement: BUILD-02
    verification:
      - kind: integration
        ref: "crates/mt-capture/tests/build_02_drop_log.rs#build_02_an_all_run_below_the_floor_is_refused"
        status: pass
      - kind: unit
        ref: "crates/mt-capture/tests/build_02_drop_log.rs#build_02_floor_counts_retained_targets"
        status: pass
      - kind: integration
        ref: "crates/mt-cli/tests/build_03_seeds_cli.rs#build_02_cli_all_below_the_floor_exits_two"
        status: pass
    human_judgment: false
  - id: D4
    description: "A differs drop must name at least one differing object and one differing section (a missing object records its status word as its section); the committed drop log and manifest still validate (T-02-56)"
    requirement: BUILD-02
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/build_02_drop_log.rs#build_02_drop_log_requires_evidence_for_differs"
        status: pass
      - kind: unit
        ref: "crates/mt-capture/tests/build_02_drop_log.rs#build_02_drop_log_keeps_at_least_ten_targets"
        status: pass
    human_judgment: false

duration: 6min
completed: 2026-10-19
status: complete
---

# Phase 2 Plan 16: Replay bound to the manifest pin, 10-target floor in every --all step Summary

**Replay refuses any capture report whose pair, source commit, tree sha256 or tool versions differ from the manifest and pins file and builds the pristine path from the pinned commit only; every `--all` step refuses a bed below 10 retained targets; a `differs` drop must name its objects and sections.**

## Performance

- **Duration:** about 6 min
- **Started:** 2026-10-19T21:42:53Z
- **Completed:** 2026-10-19T21:48:48Z
- **Tasks:** 3 (a tracer and two TDD tasks, each RED then GREEN)
- **Files modified:** 8 (1 created, 7 modified)

## Accomplishments

- WR-02: `run_replay` compares target, config, `source.commit`, `source.tree_sha256` and the tool map with the manifest pin (`pinned_source`) and the pins file (`reported_tools`) after the verdict check and before `verify_cached`. `plan_of` takes the `PinnedSource`, so the only pristine path line is `pristine_src: cache_paths(&opts.work_dir, id, &pin.commit).tree` (testbed.rs:599); `captured.source.commit` no longer builds any path. A `../../../../etc` commit is just a `source.commit` mismatch.
- WR-08: `check_floor` / `TestbedError::BelowFloor { retained, floor }`; `load_inputs` calls it for `Selection::All` after the drop log validates and before any directory is made or tool started. Fetch, capture, replay and seeds all go through `load_inputs`, so none can pass below the floor. The real bed still passes: `mt testbed seeds --check --all` on the committed manifest ran with 16 rows and `result: passed`.
- WR-08: `DropLog::validate` pushes `dropped[i] (target/config).differing_objects|differing_sections: cause differs needs at least one differing object|section, as the replay report lists them` for each empty field; the field docs say a section may be `missing_in_replay` or `missing_in_original`. `testbed/dropped.toml` (one `entry_mismatch` entry) validates unchanged.

## Task Commits

1. **Tracer RED:** `5d9d038` (test) - four replay-pin tests
2. **Tracer GREEN:** `fa1df60` (feat) - `pinned_source`, `reported_tools`, `report_mismatch` in `run_replay`
3. **Task 2 RED:** `7476f1c` (test) - floor tests, changed existing tests, explicit-id fixture beds
4. **Task 2 GREEN:** `5dc8714` (feat) - `BelowFloor`, `check_floor`, floor in `load_inputs`
5. **Task 3 RED:** `19d5d1d` (test) - `build_02_drop_log_requires_evidence_for_differs`
6. **Task 3 GREEN:** `da512c4` (feat) - differs evidence rule, whole_target helper fixture

Tracer gate: `<verify>` is automated-only and the mode is end-of-phase interactive, so the tracer verify (four test binaries, 40 tests) was re-run, passed, and execution expanded to Task 2. No REFACTOR commit was needed.

## RED evidence (test code only against the unchanged implementation)

Tracer RED `5d9d038`, assertion-level (the tests use existing names only, so they compile):

```
build_02_replay_refuses_a_capture_from_another_pin ... FAILED
  the row must say the report is from a different source pin and name source.commit:
  not_proven: the cache at .../work/cache/demo_make/0123456789abcdef0123456789abcdef01234567/tree
  does not match its pin: the cached tree is missing
build_02_replay_refuses_a_capture_made_with_other_tools ... FAILED   (same row, tool set / arm_gcc)
build_02_replay_refuses_a_report_for_another_pair ... FAILED         (same row, pair / target)
build_02_replay_checks_the_pin_before_the_cache ... ok
test result: FAILED. 1 passed; 3 failed
```

`build_02_replay_checks_the_pin_before_the_cache` passes at RED by design: it is the non-regression guard that a genuine report still reaches the cache check (the plan's behaviour list says "RED today" only for the three refusal tests).

Task 2 RED `7476f1c`: the new floor tests call `TestbedError::BelowFloor` and `check_floor`, which do not exist, so the file fails to compile (the evidence the plan allows):

```
error[E0432]: unresolved import `mt_capture::testbed::check_floor`
  --> crates/mt-capture/tests/build_02_drop_log.rs:20:59
error[E0599]: no variant named `BelowFloor` found for enum `TestbedError`   (x4: lines 444, 524, 529, 583)
```

The CLI test uses existing code only, so its RED is an assertion failure:

```
test build_02_cli_all_below_the_floor_exits_two ... FAILED
  assertion `left == right` failed: stdout: step: seeds   (the run exited 0, not 2)
test result: FAILED. 4 passed; 1 failed
```

Task 3 RED `19d5d1d`, assertion-level:

```
build_02_drop_log_requires_evidence_for_differs ... FAILED
  a differs entry with no differing object is refused: DropLog { ... cause: Differs,
  differing_objects: [], differing_sections: [".text"], ... }   (the empty entry was accepted)
```

## Existing tests edited (named per the TDD rule)

All edits were made in RED commit `7476f1c` except the last, which is in GREEN `da512c4`. No check was weakened and no validation code was removed.

- `build_02_drop_log.rs::make_bed`: `Selection::All` became `Selection::Ids(["demo_cmake", "demo_make"])`. Named ids skip dropped targets exactly as `--all` does (02-09), so what every test using it checks is unchanged.
- `build_02_drop_log.rs::build_02_dropped_pairs_are_never_reported_as_passing`, "Every target dropped" block: its dropped-row assertions now run on the named-id bed (same assertions), and the block adds the WR-08 refusal: the same bed as `Selection::All` returns `BelowFloor { retained: 0, floor: 10 }` from fetch, capture, replay and seeds. This replaces the old expectation `summary.passed` for an all-dropped `--all` run, which was exactly the WR-08 fail-open (founder-requested fix WR-08).
- `build_02_drop_log.rs::whole_target` (GREEN `da512c4`): a `differs` entry now carries `differing_objects = ["src/a.o"]` and `differing_sections = [".text"]`. Reason: the new WR-08 rule refuses a `differs` entry without them, and `build_02_dropped_pairs_are_never_reported_as_passing`, `build_03_seeds_check_skips_fully_dropped_targets` and the new floor test build such entries. Those tests failed after the rule landed (observed before the fix) and check unrelated behaviour. This was found when the rule went in, so it sits in the GREEN commit.
- `build_03_seed_manifest.rs::make_bed`: `Selection::All` became `Selection::Ids(["demo"])`.
- `build_03_seeds_cli.rs`: `seeds_args` and the relative-work-dir test pass `demo` instead of `--all`; the new `build_02_cli_all_below_the_floor_exits_two` replaces the last argument with `--all`.

## Files Created/Modified

- `crates/mt-capture/src/fetch.rs` - `PinnedSource`, `pinned_source`
- `crates/mt-capture/src/capture.rs` - `reported_tools`; `prepare` uses it
- `crates/mt-capture/src/testbed.rs` - `report_mismatch`, pin check in `run_replay`, `plan_of(.., pin)`, `BelowFloor`, `check_floor`, floor in `load_inputs`
- `crates/mt-capture/src/droplog.rs` - `differs` evidence rule and field docs
- `crates/mt-capture/tests/build_02_replay_pin.rs` - four WR-02 tests (new)
- `crates/mt-capture/tests/build_02_drop_log.rs` - three WR-08 tests, edits above
- `crates/mt-capture/tests/build_03_seed_manifest.rs`, `crates/mt-cli/tests/build_03_seeds_cli.rs` - explicit ids, CLI floor test

## Decisions Made

See key-decisions above. The traversal case is covered at the row level (a `source.commit` pin mismatch before any path is built, and no `<work>/cache` directory touched); the plan's `pristine_src` line is quoted in Accomplishments. No new crate was added (T-02-SC unaffected; `cargo deny check` ok).

## Deviations from Plan

None - plan executed exactly as written. The `whole_target` helper edit is a consequence of the WR-08 rule the plan introduces, listed above under existing tests.

## Issues Encountered

None. Plan 02-15's mid-edit state never interfered: it was already merged at HEAD `4b973a0`, and `build_03_committed_seeds` and `build_03_seed_narrowing` pass untouched.

## Known Stubs

None.

## Threat Flags

None - no new endpoint, auth path or trust boundary; the changes close T-02-53 to T-02-56.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plan 02-17 can pin the testbed workflow to `--all` (named ids are partial runs and are not floored) and will see the floor pass on the real bed (16 targets, 15 retained; checked with `seeds --check --all`).
- Full gate green: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace --locked` (297 passed, 0 failed; 289 before this plan plus 8 new), `cargo deny check` (advisories, bans, licenses, sources ok).

## Self-Check: PASSED

- Created file exists: `crates/mt-capture/tests/build_02_replay_pin.rs` (FOUND).
- Commits 5d9d038, fa1df60, 7476f1c, 5dc8714, 19d5d1d, da512c4 are ancestors of HEAD (FOUND).
- `pub fn pinned_source` in fetch.rs, `pub fn reported_tools` in capture.rs, `pub fn check_floor` and `BelowFloor` in testbed.rs (FOUND); `pristine_src` is built from `pin.commit` only.

---
*Phase: 02-test-bed-and-build-capture*
*Completed: 2026-10-19*
