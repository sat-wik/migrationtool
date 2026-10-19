---
phase: 02-test-bed-and-build-capture
plan: 17
subsystem: testing
tags: [github-actions, ci, determinism, docker-limits, testbed, gap-closure, mt-testbed-compare]
requires:
  - phase: 02-test-bed-and-build-capture
    provides: "02-11 isotp entry include prefix, 02-12 Makefile2 normalisation and CONFIGURE_HASH_EXCLUSIONS, 02-13 miniz_oxide zip backend, 02-14 mt testbed compare and DETERMINISM_EXCLUSIONS, 02-15 narrowed seed corpora, 02-16 replay pin and 10-target floor"
provides:
  - "testbed.yml determinism step decided by the exit codes of the rerun capture and of run_offline compare (fail closed); jq normaliser removed (WR-05)"
  - "--pids-limit 1024 --memory 6g --memory-swap 6g --cpus 2 on run_networked, run_offline and image_tool (WR-04 first half)"
  - "complete exclusion comment above the determinism step, tied to the three code constants by a static test (G-02-2)"
  - "testbed-capture notice prints a 'normalised <key> rule=... sha256=...' line per CMake pair"
  - "one green full-bed testbed run in the pinned image on the session branch, first round trip (G-02-1, G-02-2, G-02-3, G-02-4)"
affects: [02-18 capture restructure into separate containers, 02-19 second full-bed run, 02-UAT gap re-check]
actuals:
  tokens: 4745
  tasks: 2
  commits: 2
plan_head_before: ce655e8b16da3cdfd1df5313f342f334c78342f9
plan_head_after: 687faf290d7014e229657ffe89745673899d6e95
tech-stack:
  added: []
  patterns:
    - "A workflow comment that states a list of exclusions is held to the code constants by a static test that joins the comment lines and searches for each constant string"
    - "A CI verdict is the exit code of a tested Rust comparator, never a shell pipeline whose failure can read as equality"
key-files:
  created:
    - .planning/phases/02-test-bed-and-build-capture/02-17-CI-NOTICES.md
  modified:
    - .github/workflows/testbed.yml
    - crates/mt-capture/tests/build_02_workflow_static.rs
key-decisions:
  - "The determinism step runs the comparison even when the rerun capture failed, and publishes both exit codes; it fails when either is non-zero"
  - "Container limits are 1024 pids, 6 GiB memory and swap, 2 CPUs on every helper; no limit killed a build in the evidence run, so none was raised"
requirements-completed: [BED-01, BUILD-01, BUILD-02, BUILD-03]
coverage:
  - id: D1
    description: "The determinism verdict comes from mt testbed compare through run_offline and fails when the rerun or the comparison exits non-zero; the jq normaliser is gone"
    requirement: BUILD-02
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/build_02_workflow_static.rs#build_02_workflow_determinism_step_uses_the_fail_closed_comparison"
        status: pass
      - kind: integration
        ref: "testbed run 39331139047, step 'Capture again and compare the reports (determinism)': compared 74 files, result: identical, compare exit code: 0"
        status: pass
    human_judgment: false
  - id: D2
    description: "The comment above the determinism step lists every DETERMINISM_EXCLUSIONS, CONFIGURE_HASH_EXCLUSIONS and NORMALISED_CONFIGURE_FILES string"
    requirement: BUILD-02
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/build_02_workflow_static.rs#build_02_determinism_comment_lists_every_exclusion"
        status: pass
    human_judgment: false
  - id: D3
    description: "All three container helpers carry --pids-limit, --memory, --memory-swap and --cpus; compare is a recognised offline subcommand"
    requirement: BUILD-01
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/build_02_workflow_static.rs#build_02_workflow_runs_capture_and_replay_without_network"
        status: pass
      - kind: integration
        ref: "testbed run 39331139047: every step ran under the limits and none was killed"
        status: pass
    human_judgment: false
  - id: D4
    description: "TESTBED_SELECTION is exactly --all, so the 10-target floor applies to the evidence run"
    requirement: BED-01
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/build_02_workflow_static.rs#build_02_workflow_runs_the_whole_bed"
        status: pass
    human_judgment: false
  - id: D5
    description: "Full bed green in the pinned image: isotp captured with its entry found and replayed identical (G-02-1); Makefile2 normalised for all 16 CMake pairs and determinism identical (G-02-2); seeds --check passed with the narrowed corpora (G-02-3) and the miniz_oxide backend (G-02-4); all 16 fetch tree hashes equal the manifest; 16 targets retained"
    requirement: BED-01
    verification:
      - kind: integration
        ref: "testbed run 39331139047 (success) and ci run 39331138952 (success) on head 687faf2; notices in 02-17-CI-NOTICES.md"
        status: pass
    human_judgment: false
duration: 14min
completed: 2026-10-19
status: complete
---

# Phase 2 Plan 17: Determinism through mt testbed compare, container limits and the first green full-bed run Summary

**The testbed workflow's determinism verdict is now the exit code of the fail-closed `mt testbed compare` (jq normaliser removed), every container helper is resource-bounded, the exclusion comment is held to the code constants by a test, and one full-bed run in the pinned image was green on the first round trip with isotp retained, Makefile2 normalised for all 16 CMake pairs and 74 files identical across the capture rerun.**

## Performance

- **Duration:** 14 min (CI wait included: testbed job 6 min 24 s)
- **Started:** 2026-10-19T21:52:15Z
- **Completed:** 2026-10-19T22:06:00Z
- **Tasks:** 2 (tracer, then CI task)
- **Files modified:** 2 (plus 1 evidence file created)
- **CI round trips used: 1 of 3** (no extension asked or granted)

## Accomplishments

- WR-05 live in CI: the determinism step reruns the capture, then runs `run_offline compare --first /out/capture --second /out/capture-rerun/capture`, and fails with `annotate error testbed-determinism` and `exit 1` when either exit code is non-zero. Everything from the `norm()` definition to the comparison loop is gone, with no `jq`, `cmp -s` or process substitution in the step.
- WR-04 first half: `--pids-limit 1024 --memory 6g --memory-swap 6g --cpus 2` on `run_networked`, `run_offline` and `image_tool`, with a helper comment saying what they bound; `compare` is now a recognised offline subcommand in the network static test.
- G-02-2: the comment above the determinism step names every string of `DETERMINISM_EXCLUSIONS`, `CONFIGURE_HASH_EXCLUSIONS` and `NORMALISED_CONFIGURE_FILES`; `build_02_determinism_comment_lists_every_exclusion` reads the constants and fails if one is missing. The `evidence()` function now prints `<target>/<config> normalised <key> rule=<rule> sha256=<sha256>` for every capture report that has `normalised_configure_files`.
- Housekeeping from the plan: header title list gained testbed-seeds and testbed-determinism; the fixture-pick comments (workflow and static test) now describe the SIGPIPE race under pipefail instead of "outgrows a pipe buffer".
- Local end-to-end check of the new step (no Docker): two copies of the CI fixtures compare identical (exit 0, 9 files); both copies truncated gives `cannot parse` and exit 1. The ci-lib.sh heredoc extracts (149 lines) and passes `bash -n`; the determinism step's run block passes `bash -n`; the YAML parses; the new `evidence()` jq filter was exercised on a hand-made report.

## Task Commits

1. **Tracer RED: static tests** - `458f367` (test). Test file only, run against the unchanged workflow.
2. **Tracer GREEN: workflow** - `687faf2` (feat). Workflow plus a comment-only wording fix in the test file.
3. **Task 2: full-bed CI round trip** - no code commit needed: the first round trip was green, so no fix, drop or data change was made.

**Plan metadata:** committed after this file (docs: complete plan).

## TDD Gate Compliance

RED commit `458f367` changed only `crates/mt-capture/tests/build_02_workflow_static.rs`; GREEN commit `687faf2` follows it. Evidence (`cargo test -p mt-capture --test build_02_workflow_static` against the unchanged workflow, `5 passed; 4 failed`):

| Failing test | Assertion that failed (the planned behavior) |
|---|---|
| `build_02_workflow_runs_capture_and_replay_without_network` | "the workflow never runs compare" (the helper-limit loop sits after it) |
| `build_02_workflow_determinism_step_uses_the_fail_closed_comparison` | "the determinism step does not compare through run_offline compare" and printed the old jq step |
| `build_02_determinism_comment_lists_every_exclusion` | "the comment above the determinism step omits ..." the first missing constant string |
| `build_02_capture_notice_shows_the_normalised_configure_evidence` | "evidence() never reads normalised_configure_files" |

Semantic assessment: each target test ran, was discovered (9 tests in the file), and failed on the planned assertion about the workflow text, not on a compile, load or fixture fault. `build_02_workflow_runs_the_whole_bed` passed already in RED because the env pin `TESTBED_SELECTION: "--all"` was in place since plan 02-09; it stays as a regression pin (WR-08) and was not weakened. `gsd_run check tdd-red-evidence` does not parse cargo test output (no supported report format), so the evidence above is the unmodified assertion text, assessed by hand. GREEN: `9 passed; 0 failed`; full workspace `301 passed, 0 failed` (297 before plus 4 new tests), `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo deny check` clean. No REFACTOR commit.

## CI round trip 1 (the only one)

- Pushed `git push origin HEAD` on `claude/gsd-discuss-phase-1-mzdw5v`, head `687faf290d7014e229657ffe89745673899d6e95` (carries the RED and GREEN commits; plans 02-11 to 02-16 were already on origin, see Deviations).
- `testbed` run **39331139047**: success, job `testbed` 118330280765 (21:57:09Z to 22:03:33Z), job `mt` 118329846947 success. All 24 steps success.
- `ci` run **39331138952**: success for 687faf2.
- `container` run 39331138908: still in progress when last read (not awaited, not part of this plan's verdict).
- Context only (not this plan's evidence): testbed run 39329552496 on 4b973a0 success; testbed run 39330674599 on ce655e8 (pre-02-17 workflow) was cancelled by the concurrency group when this plan's push landed.

### What the notices show (full text in `02-17-CI-NOTICES.md`)

- **testbed-image:** `ghcr.io/sat-wik/migrationtool-toolchain@sha256:e83006c0a7e37f788afcbd1c95554e8c4dfc61c3b68da3ff964caf47fb3851b9`, linux/amd64.
- **testbed-validate:** `manifest valid (16 targets, 31 configurations)`, `result: passed (1 rows)`.
- **testbed-fetch:** 16 rows `fetched`, `result: passed (16 rows)`, `exit code: 0`; the 16 `tree_sha256` values in the evidence lines were compared programmatically with `testbed/manifest.toml`: 16 of 16 equal.
- **testbed-seeds:** `result: passed (16 rows)`; httpparser 76, isotp 19, nanopb 200, qcbor 200 (equal to the committed `index.json` counts; the narrowed counts of plan 02-15), cjson 25, corejson 108, coremqtt 15, libfdt 16, lz4 20, mbedtls 200, microcoap 16, nimble 200, paho 17, tinycbor 200, zcbor 137, zlib 18.
- **testbed-capture** (5 annotation parts, 46 rows): `result: passed (46 rows)`; 30 retained pairs `captured`, 14 variants `changes_code true`, `paho/server` and `paho/variant` `dropped (entry_mismatch)`. isotp/default: `isotp_on_can_message=found:true,signature:true,header:true`, `objects=1`. All 30 retained pairs show `compiler_allow_listed=true arch_ok=true undeclared=0 timestamp_macro=0`.
- **Makefile2 evidence (G-02-2):** 16 `normalised build/CMakeFiles/Makefile2 rule=each run of consecutive prerequisite lines of one target (<target>: <prerequisite>) sorted bytewise; every other line in place sha256=...` lines, one for each retained CMake pair: cjson/default, cjson/no-locale, corejson/default, corejson/depth-8, coremqtt/default, coremqtt/sub-vectors-2, isotp/default, nanopb/buffer-only, nanopb/default, paho/default, tinycbor/default, tinycbor/no-float, zcbor/default, zcbor/stop-on-error, zlib/default, zlib/no-gzip. No retained CMake pair lacks one.
- **testbed-replay:** `result: passed (31 rows)`; all 30 retained pairs `identical` (isotp/default `identical`, objects=1, normalisation_applied=false), `paho/server dropped (entry_mismatch)`.
- **testbed-determinism:**

```text
rerun capture exit code: 0
compared 74 files
excluded: stdout sha256, bytes_total and bytes_stored of the step "cmake configure"
excluded: stdout sha256, bytes_total and bytes_stored of the steps "clang ast <function>"
result: identical
compare exit code: 0
```

- **Retained count:** 16 targets (paho keeps its default pair), 30 pairs, above the floor of 10.
- **isotp outcome (G-02-1):** captured, entry found with matching signature and header, replay identical; no D-11 drop needed, `testbed/seeds/isotp/` stays at 19 seeds.
- **Limits:** no step reported exit 137 or a pids error; no limit was raised.

## Files Created/Modified

- `.github/workflows/testbed.yml` - determinism step on `run_offline compare`, helper resource limits, complete exclusion comment, Makefile2 evidence lines, header and fixture-pick comment fixes
- `crates/mt-capture/tests/build_02_workflow_static.rs` - four new tests plus the extended helper test (compare subcommand, limits); fixture-pick comment reworded
- `.planning/phases/02-test-bed-and-build-capture/02-17-CI-NOTICES.md` - the seven testbed notices of run 39331139047, in order, with parts joined

## Decisions Made

- The determinism step runs `compare` even when the rerun capture failed, so one run reports both causes; the verdict still fails on either exit code.
- Limits were chosen as the plan states (1024 pids, 6g, 2 CPUs) and not changed: the evidence run needed no raise.

## Deviations from Plan

### Handling the plan's assumption "the first push carries all of them"

**1. [Process - orchestrator state] Plans 02-11 to 02-16 were already pushed before this plan ran**
- **Found during:** Task 2 setup
- **Issue:** the plan expected one push carrying plans 02-11 to 02-16 and the tracer; origin was already at ce655e8 (the orchestrator pushes after each plan), and those pushes ran the pre-02-17 workflow.
- **Fix:** the first round trip is the first push carrying this plan's commits (687faf2); the earlier runs are cited as context only. The push cancelled the in-progress testbed run 39330674599 on ce655e8 through the workflow's concurrency group; that run was not evidence for this plan.
- **Verification:** run 39331139047 and ci run 39331138952 are for 687faf2.

### Auto-fixed Issues

**2. [Rule 2 - Missing critical] Extra static test for the Makefile2 notice line**
- **Found during:** Tracer RED
- **Issue:** the plan asks for the new `evidence()` line but lists no test for it; without one the G-02-2 evidence line could vanish unnoticed.
- **Fix:** added `build_02_capture_notice_shows_the_normalised_configure_evidence` (RED, then green with the workflow change).
- **Files modified:** crates/mt-capture/tests/build_02_workflow_static.rs
- **Committed in:** 458f367 (RED), 687faf2 (GREEN)

**3. [Note] `build_02_workflow_runs_the_whole_bed` passed in RED**
- The pin already held; the test is kept as a regression guard and not weakened. Not a failure of the TDD rule: the other four tests of the file are the RED evidence.

**4. [Note] The comment-only rewording of the fixture-pick comment in the static test sits in the GREEN commit**
- It is a comment change next to the workflow comment it mirrors; no assertion changed.

**5. [Note] Notices stored in a separate evidence file**
- The capture notice is about 20 KB, so the five required notices are in `02-17-CI-NOTICES.md` and this SUMMARY quotes the determinism notice in full and the other notices' result lines and counts.

---

**Total deviations:** 1 process deviation, 1 auto-added test, 3 notes. 
**Impact on plan:** none on the gap-closure outcome; the CI budget was not touched beyond the one planned round trip.

## Issues Encountered

None. No gate, comparison, normalisation rule, exclusion or floor was edited; no pre-decided handling case triggered.

## Authentication Gates

None.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None.

## Next Phase Readiness

- G-02-1, G-02-2, G-02-3 and G-02-4 hold in the pinned image on the whole bed, WR-05 and the resource-limit half of WR-04 are live, and WR-08's floor ran on the real bed. Round trips: 1 of 3 used for this plan.
- Plan 02-18 may restructure the capture into separate containers; a failure there is now separable from these fixes. Plan 02-19 plans its own full-bed round trip.
- The `container` workflow run 39331138908 was not awaited here.
- `ci` is green for 687faf2; the plan metadata commit that follows touches only `.planning/`, so the orchestrator's next push needs no new testbed proof (trigger paths unchanged).

## Self-Check: PASSED

- FOUND: `.github/workflows/testbed.yml`, `crates/mt-capture/tests/build_02_workflow_static.rs`, `.planning/phases/02-test-bed-and-build-capture/02-17-CI-NOTICES.md`
- FOUND commits `458f367` and `687faf2` (ancestors of HEAD)
- Task 2 verifies: testbed latest run on the branch `success 687faf2...` equals the last commit touching the trigger paths; ci latest run `success` for the pre-SUMMARY HEAD; `cargo test -p mt-capture --test build_02_drop_log --test build_03_committed_seeds` 10 and 7 passed; `cargo test --workspace --locked` 301 passed

---
*Phase: 02-test-bed-and-build-capture*
*Completed: 2026-10-19*
