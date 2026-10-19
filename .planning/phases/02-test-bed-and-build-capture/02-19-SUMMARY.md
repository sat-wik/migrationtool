---
phase: 02-test-bed-and-build-capture
plan: 19
subsystem: testing
tags: [github-actions, ci, wr-04, per-pair-containers, record-stage, determinism-isolation, testbed, gap-closure]
requires:
  - phase: 02-test-bed-and-build-capture
    provides: "02-18 capture_build/capture_record, mt testbed capture --stage build|record and mt testbed pairs; 02-17 container limits, mt testbed compare in the determinism step and the first green full-bed run; 02-11 to 02-16 gap and review fixes"
provides:
  - "run_build: one no-network container per retained pair whose only writable mounts are a discarded scratch /work and that pair's run directory (cache read-only, no /out)"
  - "testbed.yml steps: pairs from `mt testbed pairs`, 'Build each pair in its own container' (testbed-build), 'Record the captures' (capture --stage record, testbed-capture)"
  - "determinism rerun isolated: run_build into work-rerun, record into /out/capture-rerun, mt testbed compare against /out/capture"
  - "WORK_HOST, OUT_HOST and CACHE_HOST in the container helpers"
  - "build_02_untrusted_builds_cannot_write_the_evidence and an extended network/limits test (static)"
  - "testbed/README.md section 'Container kinds in CI'"
  - "final full-bed testbed run green for the head commit on the first round trip (run 39335185551), notices byte-identical to plan 02-17's"
affects: [02-UAT gap re-check, /gsd-verify-work 2, phase 3 (uses the retained bed)]
actuals:
  tokens: 7188
  tasks: 2
  commits: 2
plan_head_before: 746f8b4ede891064e0733679c6e55e2e49b67cbc
plan_head_after: fb048678e1a6b73cf006cda40f94013703541c49
tech-stack:
  added: []
  patterns:
    - "Untrusted build code runs only in a helper whose writable mounts are enumerated and pinned by a static test (exactly two); everything that writes evidence runs in containers that execute no upstream code"
    - "A repeated run of the pipeline for a determinism check gets its own work directory through an environment variable the helpers honour, so a rerun cannot overwrite the first run's evidence"
    - "A trust-split paragraph in the workflow header, held to the code by a static test"
key-files:
  created:
    - .planning/phases/02-test-bed-and-build-capture/02-19-CI-NOTICES.md
  modified:
    - .github/workflows/testbed.yml
    - crates/mt-capture/tests/build_02_workflow_static.rs
    - testbed/README.md
key-decisions:
  - "The determinism step exports WORK_HOST and CACHE_HOST once for the whole step instead of prefixing each call, so no rerun invocation can forget the redirect; the static test requires the export before the first run_build and before the record"
  - "Only the leaf run directory and the scratch /work are chmod 0777; the parents run/ and run/<id> stay 0755 because no container writes them"
  - "A pair whose build finishes build_failed or not_proven still exits the build container with 0 (its build record was written); the record step reports it, as the plan and 02-18 specify"
requirements-completed: [BED-01, BED-02, BUILD-01, BUILD-02, BUILD-03]
coverage:
  - id: D1
    description: "WR-04: every upstream build runs in its own run_build container with exactly two writable mounts (a fresh scratch /work and that pair's run directory), the cache read-only, no /out and no capture artifacts; the --stage build text occurs only in run_build"
    requirement: BUILD-01
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/build_02_workflow_static.rs#build_02_untrusted_builds_cannot_write_the_evidence"
        status: pass
      - kind: integration
        ref: "testbed run 39335185551, step 'Build each pair in its own container': 30 of 30 retained pairs built"
        status: pass
    human_judgment: false
  - id: D2
    description: "The capture artifacts replay and the determinism step read are written only by the record stage (capture --stage record in run_offline); the determinism rerun builds into work-rerun and records into /out/capture-rerun"
    requirement: BUILD-02
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/build_02_workflow_static.rs#build_02_untrusted_builds_cannot_write_the_evidence"
        status: pass
      - kind: integration
        ref: "testbed run 39335185551, steps 'Record the captures' and 'Capture again and compare the reports (determinism)': compared 74 files, result: identical, compare exit code: 0"
        status: pass
    human_judgment: false
  - id: D3
    description: "All four container helpers (run_networked, run_offline, image_tool, run_build) have --pids-limit, --memory, --memory-swap and --cpus, --network none except fetch, a read-only root, user 65534, no capabilities and no-new-privileges"
    requirement: BUILD-01
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/build_02_workflow_static.rs#build_02_workflow_runs_capture_and_replay_without_network"
        status: pass
    human_judgment: false
  - id: D4
    description: "Final full-bed run green for the head commit: 16 targets and 30 pairs retained (isotp included), every pair built, captured and replayed identical, seeds --check passed for 16 targets, determinism identical through mt testbed compare, 16 of 16 fetch tree hashes equal the manifest; ci green for the same commit"
    requirement: BED-01
    verification:
      - kind: integration
        ref: "testbed run 39335185551 (success) and ci run 39335185566 (success) on head fb04867; notices in 02-19-CI-NOTICES.md"
        status: pass
    human_judgment: false
  - id: D5
    description: "testbed/README.md describes the container kinds, what each may write and the limits, without a certification claim"
    requirement: BED-02
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/build_02_workflow_static.rs#build_02_readme_names_the_container_kinds"
        status: pass
      - kind: unit
        ref: "crates/mt-capture/tests/build_02_workflow_static.rs#build_02_testbed_docs_make_no_certification_claim"
        status: pass
    human_judgment: false
duration: 17min
completed: 2026-10-19
status: complete
---

# Phase 2 Plan 19: Per-pair build containers, a trusted record step and the final green full-bed run Summary

**WR-04 is closed in CI: every retained pair's upstream build runs in its own no-network container that can write only a discarded scratch `/work` and that pair's run directory, the capture artifacts are written only by a record step that runs no upstream code, the determinism rerun is isolated in its own work directory, a static test pins the writable mounts, and one full-bed run on the first round trip is green with notices byte-identical to plan 02-17's (16 targets, 30 pairs identical, 74 files identical across the rerun).**

## Performance

- **Duration:** 17 min (CI wait included: testbed job 6 min 40 s)
- **Started:** 2026-10-19T22:24:59Z
- **Completed:** 2026-10-19T22:42:00Z
- **Tasks:** 2 (tracer, then CI task)
- **Files modified:** 3 (plus 1 evidence file created)
- **CI round trips used: 1 of 3** (no extension asked or granted)

## Accomplishments

- `run_build ID CONFIG` (workflow helper): `--network none`, `--pids-limit 1024 --memory 6g --memory-swap 6g --cpus 2`, `--user 65534:65534`, `--read-only`, `--cap-drop ALL`, `no-new-privileges`, tmpfs `/tmp`. Writable mounts are exactly `"$scratch/work:/work"` (scratch under `$RUNNER_TEMP/scratch/<work dir name>/<id>/<cfg>`, never read again) and `"$work/run/$id/$cfg:/work/run/$id/$cfg"`; mt, pins and testbed are `:ro`, the source cache is `:ro` at `/work/cache`; nothing is mounted at `/out`. A run directory that already exists is refused with exit 2, never deleted.
- Workflow steps: `run_offline pairs` lists the retained pairs; "Build each pair in its own container" (id `build`, notice `testbed-build`) runs `run_build` per line with stdin from `/dev/null` and goes on after a failure so one run reports every pair; "Record the captures" runs `run_offline capture --stage record` (notice `testbed-capture`, unchanged id `capture`, so replay and the later steps keep their `needs`).
- Determinism step: exports `WORK_HOST=$RUNNER_TEMP/work-rerun` and `CACHE_HOST=$RUNNER_TEMP/work/cache`, rebuilds every pair with `run_build`, records into `/out/capture-rerun`, then `run_offline compare --first /out/capture --second /out/capture-rerun/capture`; fails on any non-zero build, record or compare. The exclusion comment still lists every constant (the 02-17 test passes) and now says the rerun is isolated.
- `run_networked`, `run_offline` and `image_tool` take `WORK_HOST` and `OUT_HOST`; `run_offline` adds a read-only `CACHE_HOST` mount when set. Header comment: `testbed-build` in the titles and a "Trust split" paragraph.
- `testbed/README.md`: a "Container kinds in CI" section with a table of the four container kinds (network, what runs, what each may write), the limits, and the record/rerun flow; "Current bed" untouched.
- Local checks that need no Docker: the ci-lib.sh heredoc (189 lines) and all 24 `run:` blocks pass `bash -n`; with a stub `docker`, `run_build` created `rt/work/run/qcbor/default` and the scratch directory at mode 0777, refused the second call with exit 2 and printed exactly the planned mounts, and the build and determinism step scripts ran end to end (build notice, rerun into `work-rerun`, record, compare).

## Task Commits

1. **Tracer RED** - `63b3064` (test) - static test file only
2. **Tracer GREEN** - `fb04867` (feat) - workflow and README
3. **Task 2: full-bed CI round trip** - no code commit: the first round trip was green, so no fix, drop or data change was made

**Plan metadata:** committed after this file (docs: complete plan).

## TDD Gate Compliance

RED `63b3064` changed only `crates/mt-capture/tests/build_02_workflow_static.rs`; GREEN `fb04867` follows. `cargo test -p mt-capture --test build_02_workflow_static` against plan 02-17's unchanged workflow: `8 passed; 3 failed`:

| Failing test | Observed failure (the planned behaviour) |
|---|---|
| `build_02_workflow_runs_capture_and_replay_without_network` | `.github/workflows/testbed.yml defines no run_build() {` (assertion in `function_body`, called for the new fourth helper) |
| `build_02_untrusted_builds_cannot_write_the_evidence` | the same message: the first thing the test needs is the `run_build` helper |
| `build_02_readme_names_the_container_kinds` | `testbed/README.md does not describe "run_build"` |

Semantic assessment: the three target tests were discovered (11 tests in the file) and failed on assertions about the workflow and README text, not on a compile, load or fixture fault; the other 8 tests (including the 02-17 determinism and exclusion tests) passed before and after. The isolation test stops at the first helper lookup in RED, so its deeper assertions (exactly two writable mounts, no `/out`, `--stage` rules, step order, rerun redirect) are first exercised in GREEN. To show they bite, after GREEN I mutated the workflow four times and each mutation failed the test (then restored the file, `cmp` identical): cache mount without `:ro`, a `-v $RUNNER_TEMP/out:/out` added to `run_build`, the record step without `--stage record`, and the rerun's `WORK_HOST` pointed at the first work directory. The first of these also failed the network test. `gsd_run check tdd-red-evidence` does not parse `cargo test` output (no supported report format), so the evidence is the unmodified assertion text, assessed by hand, as in plans 02-17 and 02-18. GREEN: `11 passed; 0 failed`; workspace `318 passed, 0 failed` (316 before plus 2 new tests). No REFACTOR commit.

## CI round trip 1 (the only one)

- Pushed `git push origin HEAD` on `claude/gsd-discuss-phase-1-mzdw5v` (branch name checked first), head `fb048678e1a6b73cf006cda40f94013703541c49`, carrying the RED and GREEN commits. Plan 02-18's commits were already on origin (746f8b4); their runs used the 02-17 workflow and are not evidence here.
- `testbed` run **39335185551**: success, job `testbed` 118343432265 (22:30:01Z to 22:36:41Z), job `mt` 118343109529 success. All 25 steps of the job success (steps 10 `Build each pair in its own container` and 11 `Record the captures` are the new ones).
- `ci` run **39335185566**: success for fb04867.
- `container` run 39335185555: still in progress when last read (not awaited, not part of this plan's verdict).
- Verify commands run: the testbed run on the branch is `success fb048678...` which equals the last commit touching the trigger paths; the latest `ci` run is `success` for the pre-SUMMARY HEAD; `cargo test -p mt-capture --test build_02_drop_log --test build_03_committed_seeds --test build_02_workflow_static` 10 + 7 + 11 passed.

### What the notices show (full text in `02-19-CI-NOTICES.md`)

- **testbed-image:** `ghcr.io/sat-wik/migrationtool-toolchain@sha256:e83006c0a7e37f788afcbd1c95554e8c4dfc61c3b68da3ff964caf47fb3851b9`, linux/amd64.
- **testbed-validate:** `manifest valid (16 targets, 31 configurations)`, `result: passed (1 rows)`.
- **testbed-fetch:** 16 rows `fetched`, `result: passed (16 rows)`; the 16 `tree_sha256` values were compared programmatically with `testbed/manifest.toml`: **16 of 16 equal**.
- **testbed-build** (new): 30 lines, every retained pair `built`, `exit code: 0`; `paho/server` is absent because `mt testbed pairs` skips dropped pairs. Excerpt: `isotp/default isotp/default built`, `qcbor/no-hw-float qcbor/no-hw-float built`, `zlib/no-gzip zlib/no-gzip built` (the line is the pair name, then `mt`'s own `<id>/<config> built`).
- **testbed-seeds:** `result: passed (16 rows)`; counts cjson 25, corejson 108, coremqtt 15, httpparser 76, isotp 19, libfdt 16, lz4 20, mbedtls 200, microcoap 16, nanopb 200, nimble 200, paho 17, qcbor 200, tinycbor 200, zcbor 137, zlib 18.
- **testbed-capture** (5 annotation parts, 46 rows): `result: passed (46 rows)`; 30 retained pairs `captured`, 14 variants `changes_code=true`, `paho/server` and `paho/variant` `dropped (entry_mismatch)`. All 30 gate rows show `compiler_allow_listed=true arch_ok=true undeclared=0 timestamp_macro=0`, every entry `found:true,signature:true,header:true`, 16 `normalised build/CMakeFiles/Makefile2 rule=... sha256=...` lines (cjson/default, cjson/no-locale, corejson/default, corejson/depth-8, coremqtt/default, coremqtt/sub-vectors-2, isotp/default, nanopb/buffer-only, nanopb/default, paho/default, tinycbor/default, tinycbor/no-float, zcbor/default, zcbor/stop-on-error, zlib/default, zlib/no-gzip; 16 distinct hashes).
- **testbed-replay:** `result: passed (31 rows)`; all 30 retained pairs `identical` (objects=1 for isotp/default, `normalisation_applied=false` everywhere), `paho/server dropped (entry_mismatch)`.
- **testbed-determinism:**

```text
rerun capture exit code: 0
compared 74 files
excluded: stdout sha256, bytes_total and bytes_stored of the step "cmake configure"
excluded: stdout sha256, bytes_total and bytes_stored of the steps "clang ast <function>"
result: identical
compare exit code: 0
```

- **Comparison with plan 02-17's final run (run 39331139047):** `testbed-validate`, `-fetch`, `-seeds`, `-capture`, `-replay` and `-determinism` were diffed line by line against `02-17-CI-NOTICES.md` (the notice bodies, parts joined): **0 differing lines in each** (Makefile2 hashes, per-pair object counts, seed counts and the 74-file comparison included). The only change in the evidence is the new `testbed-build` notice, i.e. the split itself.
- **Retained count:** 16 targets, 30 pairs, above the floor of 10. **isotp (G-02-1):** retained, built, captured with entry found, replayed `identical`; no D-11 drop. **Drop log:** unchanged (`paho/server`).
- **Limits:** no step reported exit 137 or a pids error; no limit was raised.

## Closure table (gap ids and review items to their proof)

| Item | Closed by | Tests (offline) | CI evidence, run 39335185551 (and ci 39335185566) |
|---|---|---|---|
| G-02-1 isotp retained | 02-11 (entry probe include prefix), 02-17 | `bed_01_entry_include_prefix` tests, e.g. `bed_01_entry_probe_uses_the_defining_tu_include_prefix` | isotp/default built, captured with `isotp_on_can_message` found, signature and header true, replay identical, 19 seeds equal to the committed corpus |
| G-02-2 Makefile2 normalised, exclusions listed | 02-12, 02-17 | `build_02_cmake_capture_hashes_makefile2_normalised`, `build_02_real_makefile2_variants_normalise_equal`, `build_02_determinism_comment_lists_every_exclusion`, `build_02_capture_notice_shows_the_normalised_configure_evidence` | 16 normalised Makefile2 lines (one per retained CMake pair); determinism identical over 74 files |
| G-02-3 narrowed seed corpora | 02-15, 02-17 | `build_03_narrowed_corpora_hold_only_input_methods`, `build_03_seeding_takes_only_designated_strings` | `seeds --check` passed (16 rows); httpparser 76, nanopb 200, qcbor 200, equal to the committed `index.json` counts |
| G-02-4 all-safe-Rust zip backend with a budget | 02-13, 02-17 | `build_03_zip_inflates_with_miniz_oxide_only`, `build_03_zip_archive_has_a_total_decompressed_budget`; `cargo deny check` | `seeds --check` passed in the image (the archive-derived seeds regenerate equal) |
| WR-02 replay bound to the manifest pin | 02-16 | `build_02_replay_refuses_a_capture_from_another_pin`, `build_02_replay_refuses_a_report_for_another_pair` | replay passed, 30 pairs identical |
| WR-04 isolation and limits | 02-17 (limits), 02-18 (two stages), 02-19 (containers, trust split) | `build_02_untrusted_builds_cannot_write_the_evidence`, `build_02_workflow_runs_capture_and_replay_without_network`, `build_01_capture_stages` (8 tests), `build_01_capture_stages_cli` (7 tests) | testbed-build: 30 of 30 pairs built in their own containers; record, replay, determinism green; no limit kill |
| WR-05 fail-closed determinism comparison | 02-14, 02-17 | `build_02_compare_fails_closed_on_unparseable_reports`, `build_02_workflow_determinism_step_uses_the_fail_closed_comparison` | `compare exit code: 0`, `result: identical`, 74 files, now over the isolated rerun |
| WR-07 zip decompressed-size budget | 02-13 | `build_03_zip_budget_boundary_is_exact`, `build_03_zip_budget_of_zero_reads_nothing` | `seeds --check` passed (no archive hit the 8 MiB budget) |
| WR-08 floor and evidence fields where evidence is produced | 02-16 | `build_02_an_all_run_below_the_floor_is_refused`, `build_02_workflow_runs_the_whole_bed` | every `--all` step ran with 16 retained targets (pairs, record, replay included) |
| WR-10 exact CMake product set | 02-12 | `build_01_cmake_products_are_exactly_the_documented_paths`, `build_01_configure_hash_skip_decision_matches_the_documented_set` | 16 CMake pairs captured with their configure-time evidence; determinism identical |
| IN-07 isotp dropped as a whole target | 02-11, 02-17 | as G-02-1 | as G-02-1 |

## Files Created/Modified

- `.github/workflows/testbed.yml` - `run_build`, `WORK_HOST`/`OUT_HOST`/`CACHE_HOST` in the helpers, the build step, the record step, the isolated determinism rerun, header trust-split paragraph and titles
- `crates/mt-capture/tests/build_02_workflow_static.rs` - extended network/limits test; `build_02_untrusted_builds_cannot_write_the_evidence`; `build_02_readme_names_the_container_kinds`; helpers `code_lines`, `volume_mounts`, `first_line_with`
- `testbed/README.md` - "Container kinds in CI" section and the capture item's two-stage note
- `.planning/phases/02-test-bed-and-build-capture/02-19-CI-NOTICES.md` - the eight testbed notices of run 39335185551, parts joined in order

## Decisions Made

- WORK_HOST and CACHE_HOST are exported once at the top of the determinism step, not set per call. The test requires the export (with `work-rerun`) before the first `run_build` and the record.
- The parents `run/` and `run/<id>` stay mode 0755; only the leaf run directory and the scratch `/work` are 0777, because those are the only directories a container writes. The replay step (which rewrites a run directory as before) writes inside the leaf, whose files the build container created as uid 65534.
- The build step's exit code is non-zero only when `pairs` or a `run_build` exits non-zero; a build that finished `build_failed` or `not_proven` exits 0 and is reported by the record step, as plan 02-18's contract says.

## Deviations from Plan

### Handling the plan's assumption about the first push

**1. [Process - orchestrator state] Plan 02-18's commits were already on origin**
- **Found during:** Task 2 setup (the orchestrator's brief)
- **Issue:** origin was at 746f8b4 and that push had run the 02-17 workflow.
- **Fix:** the first round trip is the first push carrying this plan's commits (fb04867); earlier runs are not cited as evidence.

### Auto-fixed Issues

**2. [Rule 2 - Missing critical] Extra static test for the README text**
- **Found during:** Tracer RED
- **Issue:** the plan's `testbed/README.md` artifact must contain `run_build`, but the only README test checks forbidden words and `--network none`.
- **Fix:** added `build_02_readme_names_the_container_kinds` (RED, then green with the README change); it looks for `run_build`, `--stage build`, `--stage record` and `--pids-limit`.
- **Files modified:** crates/mt-capture/tests/build_02_workflow_static.rs
- **Committed in:** 63b3064 (RED), fb04867 (GREEN)

**3. [Note] Rerun exit code is a flag**
- The determinism step now ORs the rerun's builds and record into `rerun_rc` (`|| rerun_rc=1`), so "rerun capture exit code" prints 0 or 1 rather than a docker exit code; the rerun's stderr is still appended and its tail is published on failure. The existing static test (verdict from `$rerun_rc` and `$compare_rc`) passes unchanged.

**4. [Note] The isolation test's deeper assertions were not part of its RED output**
- See TDD Gate Compliance: the test panics at the first helper lookup in RED; mutation checks after GREEN show the other assertions fail when their property is broken.

---

**Total deviations:** 1 process deviation, 1 auto-added test, 2 notes.
**Impact on plan:** none on the outcome; the CI budget was not touched beyond the one planned round trip. No gate, comparison, normalisation, exclusion, limit or floor was edited, and no pre-decided handling case triggered.

## Issues Encountered

None. The first full-bed run with per-pair containers was green; no mount, ownership, "dubious ownership" or permission problem appeared, so none of the plan's plumbing fixes was needed.

## Authentication Gates

None.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None.

## Residual Trust and Open Items

- **Replay still reads the run directory with ordinary reads.** `configure_evidence_changed` and the replay copy in `testbed.rs` use plain `fs::read` (noted in the 02-18 SUMMARY). This plan's file list covers `capture.rs` only under the pre-decided handling, so it was not changed. The risk is bounded: the replay container is `run_offline` (no network, no upstream code, `/work` and `/out` the only writable mounts) and compares against hashes the record stage wrote, so a symlink planted in a run directory can at most make a replay read a file inside that container. Left to a follow-up if wanted.
- **Replay rewrites the run directory** (deletes `src`, restores the pristine copy, rebuilds the declared objects) as before; it runs after the record stage, so it cannot change the capture artifacts, only the run directory the record stage has already read.
- **Scratch directories** under `$RUNNER_TEMP/scratch` hold the build containers' HOME and temporary files and are not deleted by the step; `RUNNER_TEMP` is fresh per job. They are files owned by uid 65534, which the runner user cannot remove, so cleanup is left to the job's end.
- **The `container` workflow run 39335185555** was not awaited here.
- `ci` is green for fb04867; the plan metadata commit that follows touches only `.planning/`, so it needs no new testbed proof.

## Next Phase Readiness

- Phase 2 gap closure is complete in CI: G-02-1 to G-02-4 and WR-02, WR-04, WR-05, WR-07, WR-08, WR-10 and IN-07 each map to committed tests and the green run above. Round trips: 1 of 3 used for this plan (the cap was 3; no extension).
- The bed is 16 targets and 30 retained pairs, `paho/server` dropped as logged; `/gsd-verify-work 2` can re-check the UAT gaps against run 39335185551.

## Self-Check: PASSED

- FOUND: `.github/workflows/testbed.yml`, `crates/mt-capture/tests/build_02_workflow_static.rs`, `testbed/README.md`, `.planning/phases/02-test-bed-and-build-capture/02-19-CI-NOTICES.md`
- FOUND commits `63b3064` and `fb04867` (ancestors of HEAD); `git rev-list --count 746f8b4..HEAD` = 2 at SUMMARY write; no deleted files in either commit
- Acceptance: `build_02_untrusted_builds_cannot_write_the_evidence` and the extended network test pass with RED quoted above; `testbed.yml` contains `run_build`, `capture --stage record` (twice) and `pairs`, and no capture invocation without `--stage`; the helper library passes `bash -n`; the latest testbed run on the branch is `success` for the last commit touching its trigger paths (fb04867) and the latest ci run is `success` for HEAD before this SUMMARY
- Verification: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace --locked` (318 passed, 0 failed) and `cargo deny check` (advisories, bans, licenses, sources ok) on the tracer tree

---
*Phase: 02-test-bed-and-build-capture*
*Completed: 2026-10-19*
