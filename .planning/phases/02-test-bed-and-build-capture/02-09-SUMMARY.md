---
phase: 02-test-bed-and-build-capture
plan: 09
subsystem: testing
tags: [testbed, drop-log, cmake, determinism, github-actions, replay, full-bed]

requires:
  - phase: 02-test-bed-and-build-capture
    provides: "02-03 manifest and drop_granularity: variant; 02-05 capture drivers; 02-06 workflow, replay and determinism step; 02-08 seeds and seeds --check; 02-10 fixtures"
provides:
  - "mt_capture::droplog (DROP_LOG_SCHEMA_VERSION, MIN_RETAINED = 10, DropLog, DropPolicy, Dropped, DropCause, retained_targets, is_dropped) with validation against the manifest"
  - "testbed/dropped.toml and testbed/DROPPED.md: two entries (isotp, paho/server), each with cause, fix tried and the CI run"
  - "build_args for all 16 approved targets, testbed/targets/mbedtls/user_config.h, and a workflow that runs --all"
  - "A green full-bed run: 15 targets retained, 29 target-configuration pairs, every replay verdict identical, determinism identical, seeds without drift"
  - "CMakeFiles/Makefile2 classified as a CMake product (not a configure input), with its measured cause"
affects: [02-10, phase 2 verification, Phase 3 (the bed the harness runs on)]

plan_head_before: c53b48977f6bde643d5ffce3df18d53bebca5ccc
plan_head_after: f0a36b82895537aa730c8c354d921fa647875a3b

actuals:
  tokens: 14900   # chars/4 over the added lines of the realized diff; .planning, Cargo.lock and testbed/seeds excluded
  tasks: 3
  commits: 6      # measured: git rev-list --count c53b489..f0a36b8 (the SUMMARY and state commits come after)

tech-stack:
  added: []
  patterns:
    - "A dropped pair is a row value `dropped (<cause>)`, excluded from `passed`, and kept in the manifest and the drop log"
    - "A generated file is left out of the configure-time hashes only when no compile command reads it and its instability is measured, one exact path at a time"
    - "Shell pipelines in the workflow must not close a pipe early (`| head`) under pipefail"

key-files:
  created:
    - crates/mt-capture/src/droplog.rs
    - crates/mt-capture/tests/build_02_drop_log.rs
    - testbed/dropped.toml
    - testbed/DROPPED.md
    - testbed/targets/mbedtls/user_config.h
  modified:
    - crates/mt-capture/src/capture.rs
    - crates/mt-capture/src/testbed.rs
    - crates/mt-capture/src/seeding.rs
    - crates/mt-capture/src/lib.rs
    - crates/mt-capture/tests/build_01_capture_drivers.rs
    - crates/mt-capture/tests/build_02_workflow_static.rs
    - crates/mt-capture/tests/build_03_committed_seeds.rs
    - crates/mt-capture/tests/bed_01_committed_manifest.rs
    - testbed/manifest.toml
    - testbed/README.md
    - .github/workflows/testbed.yml

key-decisions:
  - "Drop granularity is variant (02-03): a failing variant drops only that variant, a failing default drops the target"
  - "CMakeFiles/Makefile2 is left out of configure_files by exact path, not by pattern: CMake writes the prerequisite lines of equal-rank targets in pointer order, so two configures of the same tree can differ in the order of two lines. flags.make, build.make, depend.make and every other generated file stay hashed"
  - "The determinism comparison in the workflow is unchanged: no exclusion was added there"
  - "The fixture step reads the whole object listing with sed -n 1p instead of closing the pipe with head -n 1"

patterns-established:
  - "Measure a suspected nondeterminism with plain repeated cmake configures over every CMake pair before classifying it (20 runs per pair; only zcbor varied)"

requirements-addressed: [BED-01, BED-02, BUILD-01, BUILD-02, BUILD-03]
requirements-completed: []  # left empty on purpose: the dispatch said not to mark any requirement complete; phase verification decides

coverage:
  - id: D1
    description: "Drop log schema and rules: dropped pairs are skipped, reported as dropped (<cause>), never counted as passing, at least 10 targets stay retained, DROPPED.md matches dropped.toml"
    requirement: "BED-02"
    verification:
      - kind: unit
        ref: "cargo test -p mt-capture --test build_02_drop_log (build_02_drop_log_keeps_at_least_ten_targets, build_02_drop_log_entries_name_manifest_targets, build_02_drop_log_rejects_bad_entries, build_02_dropped_md_matches_drop_log, build_02_dropped_pairs_are_never_reported_as_passing)"
        status: pass
      - kind: unit
        ref: "cargo test -p mt-capture --test build_03_committed_seeds (build_03_dropped_targets_have_no_seed_corpus)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Every approved target has a build recipe, the committed manifest passes validate_complete, and the workflow runs --all"
    requirement: "BED-02"
    verification:
      - kind: unit
        ref: "cargo test -p mt-capture --test bed_01_committed_manifest (bed_01_committed_manifest_is_complete)"
        status: pass
      - kind: other
        ref: "mt testbed validate in testbed run 38430137113, step 'Validate the manifest'"
        status: pass
    human_judgment: false
  - id: D3
    description: "Full-bed run C is green for the final code commit f0a36b8: 15 retained targets, 29 pairs all replayed identical, 14 variants change compiled code, 16 fetch tree hashes equal the manifest, seeds without drift, determinism identical"
    requirement: "BUILD-01"
    verification:
      - kind: other
        ref: "gh api repos/sat-wik/migrationtool/actions/runs/38430137113 (testbed, success, head f0a36b8); annotations testbed-fetch, testbed-capture, testbed-replay, testbed-seeds, testbed-determinism"
        status: pass
    human_judgment: true
    rationale: "The evidence is CI annotations, not a repository test; whether leaving CMakeFiles/Makefile2 out of the configure hashes and dropping two pairs is acceptable evidence is a founder call"
  - id: D4
    description: "Run B's two failures are fixed: Makefile2 classified as a CMake product, and the fixture pick no longer closes a pipe early"
    verification:
      - kind: unit
        ref: "cargo test -p mt-capture --test build_01_capture_drivers (build_01_cmake_probe_outputs_are_not_configure_files); --test build_02_workflow_static (build_02_workflow_fixture_object_pick_reads_the_whole_listing)"
        status: pass
    human_judgment: false

duration: 45min (this resumed session; the plan took several sessions)
completed: 2026-10-13
status: complete
---

# Phase 2 Plan 09: Full-bed run, drop log and recipes Summary

**The whole approved bed (16 targets, 46 capture rows including the variant-versus-default comparisons) now runs under `--all` in the pinned image: 15 targets and 29 pairs are retained and replay byte-identical, isotp and paho/server are dropped and logged, and the third and last allowed round trip is green with ci green for the same commit.**

## Performance

- **Duration:** about 45 min in this resumed session. Earlier sessions did Tasks 1 and 2 and runs A and B.
- **Completed:** 2026-10-13
- **Tasks:** 3 of 3
- **Files modified:** 37 outside `.planning` (1457 insertions, 315 deletions)

## Interruption and resume

The previous executor was cut off by an API usage limit after pushing 7a88ed3 (run B) and before reading all of run B's annotations. Its four commits were pushed and origin equalled local when this session started, so nothing needed recovery. This session read the complete annotation set of run B (`gh api --paginate .../check-runs/115267828827/annotations`), root-caused both failures, fixed them test-first, and used the last round trip.

## CI round trips (Task 3): 3 of 3, no extension

| Run | Commit | testbed run | Result | What it showed |
|-----|--------|-------------|--------|----------------|
| A | 564f256 | 38387515333 | failed | Fetch verified all 16 tree hashes; seeds without drift; 29 of 31 pairs captured; isotp and paho/server refused at the entry check (`entry_mismatch`) |
| B | 7a88ed3 | 38388888221 (job 115267828827) | failed | All 29 retained pairs replayed identical; failed only at the determinism step and the fixture step (below). ci run 38388888203 green |
| C | f0a36b8 | 38430137113 (testbed job 115410309401) | **green** | Everything green; ci run 38430137092 green for the same commit |

The docs commit that follows this SUMMARY changes no path that triggers testbed. It is a push to read ci for HEAD, not a Task 3 round trip.

## Run B failures, root-caused

**1. Determinism step: `./zcbor/default.json`, `build/CMakeFiles/Makefile2` had two different sha256 values in the two captures.** The annotation named exactly one differing file (72 compared); nothing else differed.
- Measured locally: 20 plain `cmake` configures of the same zcbor tree gave two variants of `Makefile2`, differing only in the order of two lines (`CMakeFiles/app.dir/all: CMakeFiles/pet1_yaml.dir/all` and the other prerequisite of `app.dir/all`). CMake writes the prerequisites of equal-rank targets in pointer order. A loop over all 17 captured CMake pairs (20 configures each, every file hashed except the already excluded logs and ABI probe) found `CMakeFiles/Makefile2` of zcbor/default and zcbor/stop-on-error to be the only varying file. Eight further full `mt testbed capture` runs of the CMake targets produced one varying hash between processes (so the failure is intermittent, which is why run A and the other pairs passed).
- Fix, in mt-capture only: `is_cmake_product("CMakeFiles/Makefile2")` is now true, the exact path, with the reasoning in the doc comment. No compile command reads Makefile2 (replay runs the captured argv; flags and defines live in `flags.make`, still hashed), and the unit test pins both the new product and look-alike inputs that must stay hashed (`flags.make`, `build.make`, `depend.make`, `Makefile.cmake`, `Makefile`, `Makefile2` at other paths). RED commit 5c6576d, GREEN commit f0a36b8.
- The determinism comparison in the workflow was not touched: no new exclusion, the report is still compared byte for byte apart from the two stdout fields documented in 02-06. Run C: `compared 72 files ... result: identical`.

**2. Step "Make the two Arm fixture objects" failed.** The job log is not readable from here (the log host answers 403), and the annotation only said "Process completed with exit code N". Reading both "exit code" annotations (2 and 1), the determinism step ends with an explicit `exit 1`, so the fixture step is the one with exit code 2.
- Likely cause, reproduced but not confirmed from the log: under `bash -eo pipefail`, `find | sort | head -n 1` is a race. `head` exits after the first write, and if the runner ignores SIGPIPE, `sort` fails its remaining writes with `write failed: Broken pipe` and exits 2. A listing of one write (the 4 to 10 objects of 02-06 and 02-08) cannot race; the `--all` listing of about 490 paths (about 48 KB with the runner's path prefix) can. Measured with the real local listing under a shell that ignores SIGPIPE: the `head` form exited non-zero in 3 of 40 tries and the `sed -n 1p` form in 0 of 40. A synthetic 80 KB listing failed every time with exit status 2.
- Fix (workflow only): `sed -n 1p` reads the whole listing, with a static test that rejects a `| head` in that step (RED 5c6576d, GREEN f0a36b8). Run C passed that step and the three fixture steps after it, which is consistent with the diagnosis but does not prove it for a race.
- **Imprecise wording left in place:** the workflow comment and the test comment say the listing "outgrows a pipe buffer". The mechanism is the race above, not the 64 KiB buffer. Correcting the comments touches `.github/workflows/testbed.yml` and `crates/**`, which retriggers the testbed workflow; a 4th round trip is not allowed without the founder's reply, so the wording stays until the next change that has a testbed run anyway.

Neither failure was a failing pair, so neither consumed a D-11 fix attempt of a target.

## Final replay table (run C, testbed-replay: 31 rows, passed, exit code 0)

All 29 retained pairs are `identical` with `normalisation_applied=false`; the two dropped pairs show `dropped (entry_mismatch)`.

| Target | default | variant | objects per configuration |
|--------|---------|---------|---------------------------|
| cjson | identical | no-locale identical | 1 |
| corejson | identical | depth-8 identical | 1 |
| coremqtt | identical | sub-vectors-2 identical | 6 |
| httpparser | identical | strict identical | 1 |
| libfdt | identical | assume-valid identical | 10 |
| lz4 | identical | heap-small identical | 5 |
| mbedtls | identical | suite-b identical | 113 |
| microcoap | identical | release identical | 2 |
| nanopb | identical | buffer-only identical | 3 |
| nimble | identical | peripheral identical | 65 |
| paho | identical | server: dropped (entry_mismatch) | 7 |
| qcbor | identical | no-hw-float identical | 5 |
| tinycbor | identical | no-float identical | 11 and 8 |
| zcbor | identical | stop-on-error identical | 6 |
| zlib | identical | no-gzip identical | 15 |
| isotp | dropped (entry_mismatch) | none | none |

Capture (`testbed-capture`, 46 rows, passed): every retained variant reports `changes_code=true` against its default (14 of 14), with the differing objects named. Mbed TLS ran with `testbed/targets/mbedtls/user_config.h`.

## Drop log (testbed/dropped.toml, testbed/DROPPED.md)

| Entry | Cause | Fix attempt | CI run |
|-------|-------|-------------|--------|
| isotp (whole target) | `entry_mismatch`: the entry check's clang probe includes `isotp.h` alone, which includes `<stdint.h>` only under `__cplusplus`, so `uint8_t` is unknown | No data-only fix exists: forcing `-include stdint.h` is refused by the manifest validator (the default configuration sets nothing, D-06) and CMake C flags belong to the capture driver. Entry check, validator and driver left unchanged | 38387515333 |
| paho/server (variant only) | `entry_mismatch`: CMake target `MQTTPacketServer` does not compile `MQTTConnectClient.c`, where the approved entry `MQTTDeserialize_connack` is defined | Entry header of paho corrected to `MQTTPacket.h` before run A (that fixed paho/default); for the variant the entry list is the founder-approved one, so changing it is not a recipe change | 38387515333 |

The policy is `variant_failure = "drop_variant"`. The seed corpus of isotp was removed. No drop was approved under the founder's budget `drop` reply (none was needed).

## Retained count and seeds

- **Retained: 15 of 16 targets**, 29 pairs (floor of 10; `build_02_drop_log_keeps_at_least_ten_targets` passes). The 15 are cjson, corejson, coremqtt, httpparser, libfdt, lz4, mbedtls, microcoap, nanopb, nimble, paho, qcbor, tinycbor, zcbor, zlib.
- **`seeds --check`, all targets (run C, `testbed-seeds`):** `result: passed (16 rows)`, exit 0: 15 corpora regenerated in the image equal the committed ones (no drift), isotp reported `dropped (entry_mismatch)`.
- **Fetch (`testbed-fetch`):** 15 trees fetched in the image and each tree_sha256 equal to the manifest (including the Mbed TLS archive, `source_kind archive`); isotp skipped as dropped. Run A also verified isotp's tree before it was dropped.

## Task Commits

1. **Task 1 RED:** `6af55b1` (test) failing tests for the drop log and for dropped pairs
2. **Task 1 GREEN:** `bbc1ade` (feat) drop log validation; dropped pairs skipped and reported, never passing
3. **Task 2:** `564f256` (feat) build recipes for the 14 remaining targets, Mbed TLS user config, workflow runs `--all`
4. **Task 3 run A drops:** `7a88ed3` (feat) drop isotp and paho/server, record the current bed
5. **Task 3 run B/C fix RED:** `5c6576d` (test) Makefile2 and the fixture pick
6. **Task 3 run C fix GREEN:** `f0a36b8` (fix) Makefile2 as a CMake product; the fixture pick reads the whole listing

**Plan metadata:** the docs commit that follows this file.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] CMake's `CMakeFiles/Makefile2` made `configure_files` nondeterministic**
- **Found during:** Task 3, run B (determinism step)
- **Fix:** exact-path classification in `is_cmake_product`, documented and unit tested (see above)
- **Files modified:** `crates/mt-capture/src/capture.rs`, `crates/mt-capture/tests/build_01_capture_drivers.rs`
- **Commits:** 5c6576d (test), f0a36b8 (fix)

**2. [Rule 1 - Bug] The fixture step's `sort | head -n 1` can fail under pipefail with `--all` (race, diagnosis not confirmed from the log)**
- **Found during:** Task 3, run B (step "Make the two Arm fixture objects")
- **Fix:** `sed -n 1p`; static test rejects `| head` in that step
- **Files modified:** `.github/workflows/testbed.yml`, `crates/mt-capture/tests/build_02_workflow_static.rs`
- **Commits:** 5c6576d (test), f0a36b8 (fix)

### Plan deviations and notes

- **The two drops were decided from run A without a CI fix attempt.** The plan's run B was to carry one fix attempt per failing pair. For isotp and paho/server the previous executor found, before and in run A, that no data-only fix exists, so it dropped them after run A and saved a round trip; their fix-attempt text says so. This is the reading of D-11 ("a documented fix attempt in manifest data") that the founder should confirm; the alternative would be a CI attempt that could only fail.
- **Run B's pair results were not used to drop anything:** all 29 retained pairs replayed identical in run B and run C.
- **Local pre-checks differ from the image.** The host has Arm GCC 13 and CMake 3.28, the image GCC 14.3 and CMake 3.25.1, and tinycbor does not build on the host (newlib `PRIu64`), so the host evidence for the Makefile2 cause is about CMake behaviour, not about the image's output. The image run C confirmed the fix.
- **`requirements-completed` is empty** because the dispatch said not to mark requirements or the phase complete.

**Total deviations:** 2 auto-fixed bugs, 4 notes. **Impact:** the Makefile2 classification is the only change that touches an evidence claim: one generated make-driver file is no longer in the configure-time hashes. It is not an input of any compile command, but it is a smaller set of hashed files than before.

## Known Stubs

None.

## Threat Flags

None. No new network endpoint, auth path or trust boundary; the container controls (`--network none`, read-only root, non-root user, no capabilities) are unchanged and still pinned by `build_02_workflow_runs_capture_and_replay_without_network`.

## Issues Encountered

The `container` workflow for f0a36b8 was still running when this was written (run 38430137007); it is not a Task 3 gate.

## Next Phase Readiness

Plan 02-10 already has its fixtures. Phase 2 verification can read run 38430137113 for the full-bed evidence. A reviewer should look at the two documented judgment points: the Makefile2 classification and the two entry_mismatch drops (isotp could be reopened by a change to the entry check's probe, which only the founder can approve).

## Self-Check

## Self-Check: PASSED

- Created files exist: droplog.rs, build_02_drop_log.rs, dropped.toml, DROPPED.md, mbedtls user_config.h (nine `#undef` lines and `MBEDTLS_NO_PLATFORM_ENTROPY`).
- All six task commits are ancestors of HEAD.
- `TESTBED_SELECTION: "--all"`; dropped.toml `schema_version = 1` and `variant_failure = "drop_variant"`.
- Task 3 verify commands: the latest testbed run on the branch is `success` for f0a36b8, the last commit touching its trigger paths; the latest ci run is `success` for HEAD at the time of the check (f0a36b8).
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace --locked` (0 failures, 250 tests ok) and `cargo deny check` all exit 0.
