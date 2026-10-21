---
phase: 02-test-bed-and-build-capture
plan: 22
subsystem: testing
tags: [mt-capture, cmake-toolchain, read-only-mount, record-stage, testbed, wr-12, wr-14]

requires:
  - phase: 02-test-bed-and-build-capture
    provides: "02-21 allow-list compile-command gate, its gate file:line table and 02-21-CI-NOTICES.md (equal to 02-20's from line 5)"
provides:
  - "write_trusted_toolchain, capture_build_with_toolchain, capture_record_read_only, cmake_configure_argv_with_toolchain and RunLayout::trusted_toolchain_path: a CMake pair is built against a toolchain file the build stage checks against the pins and never writes, and recorded only when the trusted copy is still the pinned text and the build left no toolchain file of its own"
  - "toolchain_file_finding and the toolchain_unreadable cause: check_toolchain_file and check_configured_toolchain have no error path (WR-14)"
  - "configured_toolchain_findings: the CMakeCache.txt and contiguous-flag consistency checks, named as such"
  - "toolchain_file_read_only in the capture report (omitted when false)"
  - "mt testbed toolchains, capture --toolchain-dir, capture --read-only-toolchain; run_toolchains and run_capture_record_read_only"
  - "testbed.yml: run_offline toolchains before every build container, the read-only /opt/mt/toolchain mount, --toolchain-dir and --read-only-toolchain on both record lines"
  - "container.yml image-a: the toolchain-ro-control step"
  - "one green full-bed testbed run on the final code commit with notices equal to 02-20's from line 5 on (02-22-CI-NOTICES.md)"
  - "the T-02-67/T-02-70 evidence and the G-02-8 closure table across plans 02-21 and 02-22, for /gsd-secure-phase 2"
affects: [phase-2-verify-work, phase-2-code-review, phase-2-secure-phase]

actuals:
  tokens: 26277
  tasks: 3
  commits: 6
plan_head_before: ae3df6c779103425d0917301c9e3d060ace33f27
plan_head_after: 7c55dbe5f7b2b563a9024bd2c9330e41abd4ca60
commits: 6

tech-stack:
  added: []
  patterns:
    - "prove a property by making the untrusted step unable to break it (a read-only mount written by a step that runs no upstream code), and keep end-state data checks as named consistency checks"
    - "a check of a file the build owns has no error path: every read outcome maps to a cause on the pair"
    - "a live negative control in CI is paired with a writable positive control, so the failures can only come from the property under test"

key-files:
  created:
    - crates/mt-capture/tests/build_01_toolchain_evidence.rs
    - .planning/phases/02-test-bed-and-build-capture/02-22-CI-NOTICES.md
  modified:
    - crates/mt-capture/src/capture.rs
    - crates/mt-capture/src/report.rs
    - crates/mt-capture/src/testbed.rs
    - crates/mt-cli/src/main.rs
    - crates/mt-capture/tests/build_01_capture_stages.rs
    - crates/mt-capture/tests/build_02_workflow_static.rs
    - crates/mt-cli/tests/build_01_capture_stages_cli.rs
    - .github/workflows/testbed.yml
    - .github/workflows/container.yml

key-decisions:
  - "Causes as planned: toolchain_unreadable (new) for a read of either toolchain file or of build/CMakeCache.txt that fails with anything but NotFound; toolchain_changed for a CMake pair without the trusted copy in read-only mode, a build-written stage/toolchain.cmake beside it, a copy that differs from the pins, and every consistency finding; run_dir_symlink for a file that is not regular; the 02-21 gate causes unchanged"
  - "toolchain_file_read_only is true whenever the record stage found the trusted copy and so compared it instead of the end state; a failed comparison still reads capture_failed through its cause"
  - "The build stage checks the trusted file before it verifies the cache or prepares the run directory, so a refused file leaves no run directory, build record or build tree"
  - "configure_files skips an unreadable build/CMakeCache.txt instead of aborting, because check_configured_toolchain runs first and has raised the cause (WR-14 covers the cache)"
  - "The container control makes the mounted file and directory writable by the container user and repeats every write on a writable mount, which must succeed, so only the :ro mount can make the read-only writes fail"

patterns-established:
  - "Read-only evidence inputs: a trusted step writes them under <work>/<kind>/<id>/<cfg>, the untrusted container mounts that directory read-only under /opt/mt, and the record stage requires it by flag"

requirements-completed: []  # BED-01, BED-02, BUILD-01, BUILD-02, BUILD-03 are this plan's requirements; verify-work reconciles them for phase 2

coverage:
  - id: D1
    description: "WR-12: the build cannot write the toolchain file configure reads (trusted toolchains step, read-only mount outside every writable mount, build-stage check, record-stage requirement)"
    requirement: "BUILD-01"
    verification:
      - kind: integration
        ref: "crates/mt-capture/tests/build_01_capture_stages.rs#build_01_record_stage_uses_the_read_only_toolchain_file"
        status: pass
      - kind: integration
        ref: "crates/mt-capture/tests/build_01_capture_stages.rs#build_01_record_stage_refuses_a_build_that_rewrote_the_read_only_toolchain_file"
        status: pass
      - kind: integration
        ref: "crates/mt-capture/tests/build_01_capture_stages.rs#build_01_record_stage_refuses_its_own_toolchain_file_beside_a_read_only_one"
        status: pass
      - kind: integration
        ref: "crates/mt-capture/tests/build_01_capture_stages.rs#build_01_build_stage_refuses_a_read_only_toolchain_that_differs_from_the_pins"
        status: pass
      - kind: integration
        ref: "crates/mt-capture/tests/build_01_capture_stages.rs#build_01_record_stage_requires_the_read_only_toolchain_when_asked"
        status: pass
      - kind: unit
        ref: "crates/mt-capture/tests/build_02_workflow_static.rs#build_02_untrusted_builds_cannot_write_the_evidence"
        status: pass
      - kind: unit
        ref: "crates/mt-capture/tests/build_02_workflow_static.rs#build_02_record_steps_require_the_read_only_toolchain"
        status: pass
      - kind: integration
        ref: "crates/mt-cli/tests/build_01_capture_stages_cli.rs#build_01_toolchains_writes_the_pinned_text_for_cmake_pairs"
        status: pass
      - kind: e2e
        ref: "container run 39528941352, job image-a 118990397017, step 'Prove a read-only toolchain mount cannot be written (WR-12)' success, toolchain-ro-control notice"
        status: pass
      - kind: e2e
        ref: "testbed run 39528941371 (job 118990405406) on 9273ef8: fixture corejson-default capture shows toolchain_file_read_only true and -DCMAKE_TOOLCHAIN_FILE=/opt/mt/toolchain/toolchain.cmake"
        status: pass
    human_judgment: false
  - id: D2
    description: "WR-12 consistency checks: the CMakeCache.txt line and the contiguous flags, named as consistency checks"
    requirement: "BUILD-01"
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/build_01_toolchain_evidence.rs#build_01_configured_toolchain_consistency_findings"
        status: pass
      - kind: integration
        ref: "crates/mt-capture/tests/build_01_capture_stages.rs#build_01_record_stage_checks_the_toolchain_consistency"
        status: pass
      - kind: other
        ref: "MT_LOCAL_CAPTURES=$PWD/.mt-work/capture cargo test -p mt-capture --test build_01_toolchain_evidence -- --ignored --exact build_01_local_captures_pass_the_toolchain_consistency_checks --nocapture (cmake_pairs=17 entries=99 refused=0)"
        status: pass
    human_judgment: false
  - id: D3
    description: "WR-14: an unreadable toolchain file or cache is the named cause toolchain_unreadable, never an aborted step"
    requirement: "BUILD-01"
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/build_01_toolchain_evidence.rs#build_01_unreadable_toolchain_file_is_a_named_cause"
        status: pass
    human_judgment: false
  - id: D4
    description: "G-02-8 missing 4, final part: one full-bed run on the final code commit with notices equal to 02-20's"
    requirement: "BED-01"
    verification:
      - kind: e2e
        ref: "testbed run 39528941371 success on 9273ef8; diff of 02-22-CI-NOTICES.md against 02-20-CI-NOTICES.md from line 5 is empty; ci 39528941315 success on 9273ef8"
        status: pass
    human_judgment: false

duration: 30min
completed: 2026-10-21
status: complete
---

# Phase 2 Plan 22: Read-only toolchain file and named unreadable cause (gap G-02-8, decisions 4-5) Summary

**In the testbed workflow every CMake configure now reads a toolchain file that a container running no upstream code wrote and that run_build mounts read-only at `/opt/mt/toolchain`. The build stage refuses the file unless it equals the pins, and every record step refuses a CMake pair that was not built against it. The CI control shows that every write to such a mount fails, CMake's `file(APPEND)` included. An unreadable toolchain file or cache is the cause `toolchain_unreadable` on its pair. The final full-bed run on the final code commit has notices identical to 02-20's from line 5 on, and its corejson report reads `"toolchain_file_read_only": true`.**

## Performance

- **Duration:** about 30 min (2026-10-21T01:26:44Z to about 01:57Z)
- **Tasks:** 3 of 3
- **Files modified:** 11 (9 modified, 2 created)
- **CI round trips:** 1 of the cap of 3

## Accomplishments

- `capture.rs`:
  - `RunLayout::trusted_toolchain_dir` and `trusted_toolchain_path()`, which give `<work>/toolchain/<id>/<config>/toolchain.cmake`, outside the run directory.
  - `write_trusted_toolchain` and `cmake_configure_argv_with_toolchain`. The old `cmake_configure_argv` calls the new one and keeps its signature.
  - `capture_build_with_toolchain`, which checks the file before anything runs, and `capture_record_read_only`. Each shares one implementation with its 02-20 sibling.
  - `toolchain_file_finding`, `configured_toolchain_findings`, `Session::check_configured_toolchain`, and `check_toolchain_file` with no error path.
  - The module doc now has a section "The toolchain file a CMake configure read", and the trust-model bullet was rewritten.
- `report.rs`: `toolchain_file_read_only`, with serde default and omitted when false. The committed corejson fixture still round-trips byte for byte.
- `testbed.rs`: `run_toolchains`, `run_capture_build(.., toolchain_dir)` and `run_capture_record_read_only`.
- `mt-cli`: `mt testbed toolchains`, `capture --toolchain-dir` (only with `--stage build`) and `capture --read-only-toolchain` (only with `--stage record`).
- `testbed.yml`:
  - run_build runs `run_offline toolchains` and sends its word to stderr. It then mounts `-v "$work/toolchain/$id/$cfg:/opt/mt/toolchain:ro"` and appends `--toolchain-dir /opt/mt/toolchain`.
  - Both record lines carry `--read-only-toolchain`.
  - The header and run_build comments explain the mount.
- `container.yml` image-a: the step "Prove a read-only toolchain mount cannot be written (WR-12)", which publishes the `toolchain-ro-control` notice.
- Final run: testbed 39528941371, ci 39528941315 and container 39528941352 are all green on 9273ef8. The notices diff against 02-20 is empty.

## Task Commits

| # | Commit | Type | Subject |
|---|--------|------|---------|
| Tracer RED | `bc02716` | test | specify the read-only toolchain file and the named unreadable cause (WR-12, WR-14 RED) |
| Tracer GREEN | `e85800f` | feat | build CMake pairs against a read-only toolchain file and name an unreadable one (WR-12, WR-14) |
| Task 2 RED | `4b689d4` | test | require the read-only toolchain mount and the record-stage flag in the workflow and CLI (WR-12 RED) |
| Task 2 GREEN | `b312054` | feat | mount the pinned toolchain file read-only into every build and require it at record (WR-12) |
| Task 3 CI step | `9273ef8` | ci | prove live that a read-only toolchain mount cannot be written (WR-12) |
| Task 3 notices | `7c55dbe` | docs | record the final full-bed run's testbed notices |

`9273ef8` is the plan's final code commit. The run the notices cite was made on it, and no later commit touches a path in testbed.yml's push trigger (Task 3 verify 2 passed).

## RED evidence (TDD rule)

Each RED commit holds test code only, and each was run against the unchanged implementation.

**Tracer RED (`bc02716`).** `cargo test -p mt-capture --test build_01_toolchain_evidence --test build_01_capture_stages`: neither file compiles.

- `error[E0432]: unresolved imports mt_capture::capture::configured_toolchain_findings, mt_capture::capture::toolchain_file_finding` (build_01_toolchain_evidence). Assessment: both pure functions are named, and neither exists.
- `error[E0432]: unresolved imports mt_capture::capture::capture_build_with_toolchain, mt_capture::capture::capture_record_read_only, mt_capture::capture::write_trusted_toolchain` (build_01_capture_stages). Assessment: the three stage functions are named, and none exists.
- `error[E0599]: no method named trusted_toolchain_path found for struct RunLayout`. Assessment: the trusted path did not exist.
- 2 x `error[E0609]: no field toolchain_file_read_only on type CaptureReport`. Assessment: the report field did not exist.

**Task 2 RED (`4b689d4`).**

- `build_02_workflow_static`: 10 passed, 2 failed.
  - `build_02_untrusted_builds_cannot_write_the_evidence`: "run_build must mount the pair's trusted toolchain directory read-only at /opt/mt/toolchain", with `left: []`. Assessment: run_build had no toolchain mount.
  - `build_02_record_steps_require_the_read_only_toolchain`: "a record line does not require the read-only toolchain: run_offline capture --stage record --manifest ...". Assessment: the flag was absent.
- `build_01_capture_stages_cli`: 7 passed, 2 failed.
  - `build_01_toolchains_writes_the_pinned_text_for_cmake_pairs` and `build_01_toolchain_dir_needs_stage_build`: "error: unrecognized subcommand 'toolchains'" (exit 2 where 0 is expected). Assessment: the subcommand did not exist.
  - The capture-flag half of the second test already got exit 2, because clap names an unknown `--toolchain-dir` or `--read-only-toolchain` argument in its error. That half is a guard. The test is RED through its `toolchains` half.

None of the runs printed a `skipping:` line.

**GREEN:**

- Tracer verify 1: build_01_toolchain_evidence 2 passed and 1 ignored, build_01_capture_verdict 1/1, build_01_capture_drivers 19/19, build_01_ci_fixtures 6/6. No `running 0 tests`, and no skip line.
- Tracer verify 2: build_01_capture_stages 21/21, with all six new tests and build_01_record_stage_refuses_a_changed_toolchain_file. No skip line.
- Tracer verify 3: `cmake_pairs=17 entries=99 refused=0`.
- Tracer verify 4 (the API, causes, field and module-doc greps) passed.
- Task 2 verify 1: build_02_workflow_static 12/12 and build_01_capture_stages_cli 9/9.
- Task 2 verify 2 (the testbed.yml greps) passed.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace --locked` (45 test binaries, 352 passed, 0 failed) and `cargo deny check` are clean on 9273ef8. They ran before the push.

## Changed expectation (intended, from the WR-12 decision)

| Test | Old expectation | New expectation |
|------|-----------------|-----------------|
| build_02_untrusted_builds_cannot_write_the_evidence | exactly two writable run_build mounts and read-only mt, pins, testbed and cache mounts | unchanged, plus: exactly one mount ending `:/opt/mt/toolchain:ro`, with source `$work/toolchain/$id/$cfg`; the `--stage build` line carries `--toolchain-dir /opt/mt/toolchain`; `run_offline toolchains` comes before run_build's `docker run`; no `-v` of the workflow that names a toolchain path lacks `:ro` |
| build_02_workflow_static `SUBCOMMANDS` | 7 subcommands | 8: `toolchains` added, so the network rule (every subcommand but fetch through run_offline) covers it |

Every other test of 02-20 and 02-21, and every regression guard the plan names, passes unchanged. The guards are build_01_record_stage_refuses_a_changed_toolchain_file (all four end-state cases), build_01_record_stage_refuses_a_record_that_did_not_build, build_01_staged_capture_equals_the_single_process_capture, build_01_capture_cmake_end_to_end, build_01_capture_drivers_fail_closed, build_01_cmake_configure_never_sets_cmake_c_flags and build_02_ci_reports_round_trip.

## Planner choices (recorded)

- **Causes:**
  - `toolchain_unreadable` (new) is raised by `toolchain_file_finding` (capture.rs:1337) and by `check_configured_toolchain` (capture.rs:2154).
  - `toolchain_changed` is raised for a missing trusted copy in read-only mode (capture.rs:2136), a build-written stage file (capture.rs:2128), a differing or missing file, and every consistency finding.
  - The 02-21 gate causes are unchanged. cc.rs and replay.rs are untouched by this plan (`git diff --stat ae3df6c HEAD` on them is empty).
- **Names:** as planned:
  - capture.rs: `toolchain_file_finding`, `configured_toolchain_findings`, `write_trusted_toolchain`, `capture_build_with_toolchain`, `capture_record_read_only`, `cmake_configure_argv_with_toolchain` and `RunLayout::trusted_toolchain_path`. `RunLayout` also gains the field `trusted_toolchain_dir`; no code builds it as a struct literal.
  - The report field `toolchain_file_read_only`.
  - testbed.rs: `run_toolchains` and `run_capture_record_read_only`.
  - CLI: `mt testbed toolchains`, `--toolchain-dir` and `--read-only-toolchain`.
  - The mount point `/opt/mt/toolchain`.
- **New functions, not new fields** on `CaptureContext` and `TestbedOptions`, which tests build as struct literals. The one exception is `run_capture_build`, which gained a parameter; mt-cli is its only caller.
- **Mount placement:**
  - The mount is at `/opt/mt/toolchain`, in the read-only root file system, beside `/opt/mt/pins.toml` and `/opt/mt/testbed`.
  - It is not under `/work` and not in the run directory. `prepare_run` empties the run directory in place. A mount point inside a writable parent can be moved away with that parent.
- **Local fallback:**
  - A build without a trusted directory writes `stage/toolchain.cmake`. `capture_record` then compares its end state, with `toolchain_file_read_only: false`.
  - When a trusted copy exists, `capture_record` uses it as well; only `capture_record_read_only` requires it.
- **WR-14 test:** the pure mapping test. The host runs tests as uid 0, so a mode-000 file is still readable.

## Read-only coverage (every CMake pair of the final run)

Read-only mode covers every CMake pair of the final run:

- Every `capture --stage record` line in testbed.yml carries `--read-only-toolchain`. There are two, the record step and the determinism rerun (testbed.yml:454 and :517). build_02_record_steps_require_the_read_only_toolchain pins this.
- In that mode, a CMake pair without the trusted copy is `toolchain_changed`, and so is one with a build-written toolchain file (build_01_record_stage_requires_the_read_only_toolchain_when_asked, build_01_record_stage_refuses_its_own_toolchain_file_beside_a_read_only_one).
- The final run (testbed 39528941371) captured every retained pair: 30 of 30 `captured`, the 16 CMake pairs among them. paho/server is `dropped (entry_mismatch)` by the drop log, as in 02-20.

So every CMake pair was built against the read-only file and recorded in read-only mode. The corejson report shows this directly (below).

## Local proofs (fail closed)

- `MT_LOCAL_CAPTURES=$PWD/.mt-work/capture cargo test -p mt-capture --test build_01_toolchain_evidence -- --ignored --exact build_01_local_captures_pass_the_toolchain_consistency_checks --nocapture` prints `cmake_pairs=17 entries=99 refused=0`.
  - The 17 CMake pairs are the 16 retained ones plus paho/server.
  - The 99 entries are the canonical entries of 15 pairs plus the run-export entries of the two tinycbor pairs, which have no canonical file because the local build failed on the host newlib.
  - The planning-time figure of 248 also counted the raw and run-export entries of pairs that do have a canonical file. The test checks the entries the plan names: the canonical file, or else the run export.
- The CMake-toy rewrite test also checks that the trusted copy really changed in process (it contains `-DPLANTED`), so its refusal cannot pass vacuously.

## CI round trips (cap 3, used 1)

| RT | Head | Runs (id: conclusion) | Result |
|----|------|------------------------|--------|
| 1 | `9273ef8` | testbed 39528941371: success (job 118990405406); ci 39528941315: success; container 39528941352: success (image-a 118990397017, step 11 "Prove a read-only toolchain mount cannot be written (WR-12)" success) | final run: notices equal to 02-20's from line 5 on |

The docs push of `7c55dbe` (the notices) triggered only ci (39530109453). It is not a round trip.

Green criteria of round trip 1. All were checked programmatically from 02-22-CI-NOTICES.md and the annotations, and all are met:

- testbed-fetch: 16 rows, and the tree hashes equal `testbed/manifest.toml` (16 of 16).
- testbed-build: 30 pairs `built`.
- testbed-seeds: `result: passed (16 rows)` with 02-20's counts (the section is identical).
- testbed-capture: `passed (46 rows)`.
  - 30 pairs `captured`.
  - 30 of 30 gate rows read `compiler_allow_listed=true arch_ok=true undeclared=0 timestamp_macro=0`.
  - 16 Makefile2 lines.
  - 14 variants `changes_code=true`.
  - paho/server `dropped (entry_mismatch)`.
- testbed-replay: `passed (31 rows)`, 30 of them `identical`.
- testbed-determinism: `rerun capture exit code: 0`, `compared 74 files`, `result: identical`, `compare exit code: 0`.
- All 41 testbed annotations are at notice level.

### Diff against 02-20

`diff <(sed '1,4d' 02-20-CI-NOTICES.md) <(sed '1,4d' 02-22-CI-NOTICES.md)` is empty: 336 lines each, 0 differing. The same holds against 02-21-CI-NOTICES.md.

Three things could have changed the notices, and none did:

- The configure argv and the configure-time files CMakeCache.txt, Makefile.cmake and CMakeSystem.cmake now name `/opt/mt/toolchain/toolchain.cmake`, but none of them is in the notices.
- The Makefile2 hashes did not change, which confirms the planning measurement that Makefile2 does not name the toolchain path.
- The toolchains step's word goes to stderr, so the build rows are unchanged.

### corejson fixture evidence (testbed run 39528941371, annotation "fixture corejson-default capture", 5 parts)

```json
{
  "verdict": "captured",
  "causes": [],
  "toolchain_file_sha256": "10b734c60872999b948af8fc674537bb0b7e2eef25214a4077120fd3faa13357",
  "toolchain_file_read_only": true,
  "configure": [
    "-DCMAKE_TOOLCHAIN_FILE=/opt/mt/toolchain/toolchain.cmake"
  ]
}
```

(`jq '{verdict, causes, toolchain_file_sha256, toolchain_file_read_only, configure: [.steps[] | select(.name=="cmake configure") | .run.argv[] | select(startswith("-DCMAKE_TOOLCHAIN_FILE"))]}'` over the joined parts.) The `toolchain_file_sha256` equals the one in the committed fixture `tests/fixtures/ci/corejson-default.capture.json`, so the pinned text itself did not change. The committed fixture is not replaced; it still round-trips.

### toolchain-ro-control notice (container run 39528941352, job image-a 118990397017, head 9273ef8)

```text
--- read-only mount (/home/runner/work/_temp/toolchain-ro/ro:/opt/mt/toolchain:ro, run_build's docker flags)
append: exit 2: sh: 1: cannot create /opt/mt/toolchain/toolchain.cmake: Read-only file system 
truncate: exit 1: truncate: cannot open '/opt/mt/toolchain/toolchain.cmake' for writing: Read-only file system 
create: exit 2: sh: 1: cannot create /opt/mt/toolchain/new.cmake: Read-only file system 
cmake-file-append: exit 1: CMake Error at CMakeLists.txt:3 (file):   file failed to open for writing (Read-only file system):      /opt/mt/toolchain/toolchain.cmake   -- Configuring incomplete, errors occurred! See also "/tmp/r
remove: exit 1: rm: cannot remove '/opt/mt/toolchain/toolchain.cmake': Read-only file system 
rename: exit 1: mv: cannot move '/opt/mt/toolchain' to '/opt/mt/toolchain.moved': Read-only file system 
container exit code: 0
sha256 before: 718ee34cd47fb265bdcc20c3cc0ca2e46076d586061c9d0c5486289e8ea54b77
sha256 after: 718ee34cd47fb265bdcc20c3cc0ca2e46076d586061c9d0c5486289e8ea54b77
--- control (the same writes on a writable mount)
append: exit 0: 
truncate: exit 0: 
create: exit 0: 
cmake-file-append: exit 0: -- Configuring done -- Generating done -- Build files have been written to: /tmp/ro-cmake/build 
remove: exit 0: 
rename: exit 1: mv: cannot move '/opt/mt/toolchain' to '/opt/mt/toolchain.moved': Read-only file system 
container exit code: 0
result: every write to the read-only mount failed, the file is unchanged, and the writable control accepted the writes
```

Every write to the read-only mount failed with "Read-only file system", the CMake 3.25.1 `file(APPEND)` on `${CMAKE_TOOLCHAIN_FILE}` included. The hash is unchanged. On a writable mount, with the same flags and the same file modes, every write but the rename succeeded. The rename of the mount point fails even there, because `/opt/mt` is in the read-only root. So the read-only claim holds on this runner.

## G-02-8 closure table (plans 02-21 and 02-22)

| Item | Commits | Tests | CI evidence |
|------|---------|-------|-------------|
| Missing 1: one operand-aware walk | 02-21: 68e0bc7, 8da9874, 148715f | build_01_one_walk_serves_compile_only_output_and_write_paths, build_01_gate_decides_compile_only_from_the_walk, build_01_record_stage_refuses_an_operand_compile_flag_in_the_export | testbed 39455562978 (02-21); testbed 39528941371 (30 pairs captured and replayed through the walk) |
| Missing 2: measured allow-list, named causes | 02-21: 68e0bc7, 8da9874, 148715f, 1270139, 2940852, 78a12ea, 44a3dd6 | build_01_allow_list_is_the_measured_set, build_01_allow_list_refuses_everything_else, build_01_rejects_helper_executing_driver_options, build_01_handles_write_path_options, build_02_replay_refuses_the_measured_bypasses | ci 39455563004 (02-21); ci 39528941315 |
| Missing 3: WR-12 configure-time proof, WR-14 named cause | 02-22: bc02716, e85800f, 4b689d4, b312054, 9273ef8 | the WR-12 and WR-14 rows below | container 39528941352 toolchain-ro-control; testbed 39528941371 corejson `toolchain_file_read_only: true` |
| Missing 4: no real pair refused, notices equal to 02-20's | 02-21: 8da9874, 44a3dd6, 79fe192; 02-22: 9273ef8, 7c55dbe | build_01_local_captures_pass_the_compile_command_gate (refused=0), build_01_local_captures_pass_the_toolchain_consistency_checks (refused=0), build_01_ci_fixtures, build_02_drop_log, build_03_committed_seeds, build_02_workflow_static | testbed 39455562978 (02-21) and testbed 39528941371 (02-22, final code), both with notices equal to 02-20's from line 5 on |
| Missing 5 | - | - | T-02-67 and T-02-70 re-verified with the evidence below; the 02-SECURITY.md close is done by /gsd-secure-phase 2 immediately after this plan (orchestrator step) |
| CR-01 | 02-21: 68e0bc7, 8da9874, 148715f | build_01_gate_refuses_long_option_spellings, build_02_replay_refuses_the_measured_bypasses | ci 39455563004, ci 39528941315 |
| CR-02 | 02-21: 68e0bc7, 8da9874, 148715f | build_01_gate_refuses_assembler_carrier_spellings, build_02_replay_refuses_the_measured_bypasses | ci 39455563004, ci 39528941315 |
| CR-03 | 02-21: 68e0bc7, 8da9874, 148715f | build_01_gate_decides_compile_only_from_the_walk, build_02_replay_refuses_the_measured_bypasses, build_01_record_stage_refuses_an_operand_compile_flag_in_the_export | ci 39455563004, ci 39528941315 |
| WR-12 | 02-22: bc02716, e85800f, 4b689d4, b312054, 9273ef8 | build_01_record_stage_uses_the_read_only_toolchain_file, build_01_record_stage_refuses_a_build_that_rewrote_the_read_only_toolchain_file, build_01_record_stage_refuses_its_own_toolchain_file_beside_a_read_only_one, build_01_build_stage_refuses_a_read_only_toolchain_that_differs_from_the_pins, build_01_record_stage_requires_the_read_only_toolchain_when_asked, build_01_record_stage_checks_the_toolchain_consistency, build_01_configured_toolchain_consistency_findings, build_02_untrusted_builds_cannot_write_the_evidence, build_02_record_steps_require_the_read_only_toolchain, build_01_toolchains_writes_the_pinned_text_for_cmake_pairs, build_01_toolchain_dir_needs_stage_build | container 39528941352 toolchain-ro-control; testbed 39528941371 |
| WR-13 | 02-21: 4818d97, 78a12ea, 44a3dd6 | build_01_shipped_specs_are_measured, build_01_specs_allowed_only_for_shipped_names, build_01_shipped_spec_names_are_read_from_the_toolchain | container 39453293834 arm-specs (02-21) |
| WR-14 | 02-22: bc02716, e85800f | build_01_unreadable_toolchain_file_is_a_named_cause (and `check_toolchain_file` / `check_configured_toolchain` have no `Result`, Tracer verify 4) | ci 39528941315 |
| IN-13 | 02-21: 8da9874 | build_01_gate_decides_compile_only_from_the_walk (replay entry without -c) | - |
| IN-14 | 02-21: 8da9874 | build_01_one_walk_serves_compile_only_output_and_write_paths | - |
| IN-15 | 02-21: 148715f | build_02_replay_refuses_helper_executing_driver_options (live `@rsp` and `-I @rsp` controls), build_02_replay_bypasses (live table) | ci 39455563004, ci 39528941315 |

This plan does not edit 02-SECURITY.md, 02-UAT.md, 02-REVIEW-DISPOSITION.md or ROADMAP.md.

## Threat evidence

### T-02-67 (recorded driver options running a program the build wrote) and T-02-70 (recorded options writing outside the run root): re-verified

These rows were re-mitigated by plan 02-21's allow-list gate. cc.rs and replay.rs are unchanged since 02-21, so the 02-21 lines still hold at 9273ef8. They were re-read at execution time.

| What | Where (at 9273ef8) | Threat | Proven by |
|------|--------------------|--------|-----------|
| `walk_arguments`, the one walk (private `walk` holds the body) | crates/mt-capture/src/cc.rs:519 (`walk` at :394) | both | build_01_one_walk_serves_compile_only_output_and_write_paths, build_01_allow_list_refuses_everything_else, build_01_allow_list_is_the_measured_set |
| the `@` rule over every argument after argv[0] | crates/mt-capture/src/cc.rs:398 | T-02-67 | build_01_gate_refuses_response_files_in_operand_position, build_02_replay_refuses_helper_executing_driver_options, build_01_rejects_helper_executing_driver_options |
| an unlisted argument refused (`OptionNotAllowed`), including long spellings, `-wrapper`, `-B`, plugins and carriers | crates/mt-capture/src/cc.rs:462 | both | build_01_gate_refuses_long_option_spellings, build_01_gate_refuses_assembler_carrier_spellings, build_02_replay_refuses_the_measured_bypasses, build_01_record_stage_refuses_a_helper_option_in_the_export |
| `-specs=` only for the 13 shipped names | crates/mt-capture/src/cc.rs:365 (`SHIPPED_SPECS`), :437 | T-02-67 | build_01_specs_allowed_only_for_shipped_names, build_01_shipped_spec_names_are_read_from_the_toolchain |
| `CcPolicy::check_command`, the whole gate, with compile-only from the walk | crates/mt-capture/src/cc.rs:614 | both | build_01_gate_decides_compile_only_from_the_walk, build_01_record_stage_refuses_an_operand_compile_flag_in_the_export |
| `check_write_paths` on the walk's `-o` and `-MF` operands (`writes_inside` :552) | crates/mt-capture/src/cc.rs:576 | T-02-70 | build_01_handles_write_path_options, build_02_replay_refuses_writes_outside_the_run_root, build_01_record_stage_refuses_a_write_outside_the_run_root_in_the_export |
| capture: `canonical_entry` calls `check_command` | crates/mt-capture/src/cc.rs:733 | both | build_01_record_stage_refuses_a_helper_option_in_the_export, build_01_record_stage_refuses_a_write_outside_the_run_root_in_the_export |
| replay: `check_entry` calls `check_command` | crates/mt-capture/src/cc.rs:656-657 | both | build_02_replay_refuses_the_measured_bypasses |
| replay `validate`, before any delete, copy or spawn | crates/mt-capture/src/replay.rs:149 `plan.policy.check_entry(entry)?;` | both | build_02_replay_refuses_the_measured_bypasses (original object present, outside directory empty) |
| cause mapping | crates/mt-capture/src/capture.rs:1317 `CcError::OptionNotAllowed { .. } => "option_not_allowed"` (was :1219 at 02-21; moved by this plan's additions only) | both | build_01_record_stage_refuses_a_helper_option_in_the_export |

Final run evidence:

- testbed 39528941371 on 9273ef8 captured all 30 retained pairs through `canonical_entry`, and every gate row passed. It replayed all 30 through `check_entry`, and they are identical.
- The live bypass table (build_02_replay_bypasses) and the probes of build_02_replay passed in ci 39528941315 on the same head.
- Locally, `cargo test --workspace --locked` passed: 352 tests, 0 failed.

### This plan's register

- **T-02-76 (the build changing the toolchain text configure reads, then restoring it):**
  - Code:
    - capture.rs:2375 `write_trusted_toolchain`, capture.rs:2352 `capture_build_with_toolchain` and capture.rs:2410 `check_trusted_toolchain`, which runs before verify and prepare_run;
    - capture.rs:2106 `check_toolchain_file`, capture.rs:2523 `capture_record_read_only` and capture.rs:1383 `configured_toolchain_findings` (consistency only);
    - testbed.yml:233 (toolchains step), :243 (read-only mount), and :454 and :517 (record lines).
  - Tests: the WR-12 row of the closure table.
  - CI: toolchain-ro-control (above) and the corejson report.
  - Residual, stated in the module doc: upstream CMake code can load other CMake files or add flags of its own. Those flags show in the compile commands, which the allow-list gates and replay re-runs.
- **T-02-77 (an unreadable toolchain file or cache aborting the step):**
  - Code:
    - capture.rs:1337 `toolchain_file_finding` (`toolchain_unreadable` at :1363) and capture.rs:2154 `check_configured_toolchain` (:2171). Neither function nor `check_toolchain_file` returns a `Result`.
    - `configure_files` skips an unreadable cache, whose cause is already raised.
  - Test: build_01_unreadable_toolchain_file_is_a_named_cause.
- **T-02-79 (the mount bypassed by a mount point inside a writable directory, or by a later workflow edit):**
  - The mount is at `/opt/mt/toolchain` in the read-only root, never under `/work` or the run directory (testbed.yml:243).
  - build_02_untrusted_builds_cannot_write_the_evidence requires the exact read-only mount and the `--toolchain-dir` flag, the toolchains step before `docker run`, and `:ro` on every toolchain mount of the workflow. build_02_record_steps_require_the_read_only_toolchain requires the flag on both record lines.
  - Task 2 verify 2 passed.
  - The final corejson report shows `toolchain_file_read_only: true`.
- **T-02-72..T-02-75 and T-02-78:** see 02-21-SUMMARY.md, "Threat evidence".
- **T-02-SC:** no package installed, no crate added.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Lint] A RED-commit assertion tripped clippy's `manual_contains`**
- **Found during:** Tracer GREEN, pre-merge gates
- **Issue:** `configure.run.argv.iter().any(|argument| *argument == wanted)` in build_01_record_stage_uses_the_read_only_toolchain_file fails `clippy -D warnings`.
- **Fix:** `configure.run.argv.contains(&wanted)`, which is the same check. It lives in the GREEN commit, because the RED commit is already history and it was not pushed red.
- **Files modified:** crates/mt-capture/tests/build_01_capture_stages.rs
- **Commit:** e85800f

**2. [Rule 2 - Missing critical functionality] The live control needed a positive control to be non-vacuous**
- **Found during:** Task 3
- **Issue:** a file created by the runner user would refuse writes from uid 65534 through its file mode alone, so a "failed" write could not show that the `:ro` mount was the reason.
- **Fix:**
  - The control makes the directory 0777 and the file 0666, which is what run_build's mount source effectively is, since the toolchains container writes it as the container user.
  - It repeats the same writes on a writable mount, and those must succeed. In CI they did: every write but the rename of the mount point.
  - The remove runs before the rename, so the control does not depend on the rename failing.
  - The planned failure set and hash check are unchanged.
- **Files modified:** .github/workflows/container.yml
- **Commit:** 9273ef8

**3. [Rule 2 - WR-14 completeness] The configure-time hash walk could still abort on an unreadable cache**
- **Found during:** Tracer GREEN
- **Issue:** `configure_files` reads `build/CMakeCache.txt` through `read_or_refuse`, which turns an I/O error into an aborted step. The must-have requires an unreadable cache to be a cause, never an abort.
- **Fix:** `configure_files` skips that one key when its read fails. `check_configured_toolchain` runs first in the same block and has already raised `toolchain_unreadable`, `toolchain_changed` (missing) or `run_dir_symlink`.
- **Files modified:** crates/mt-capture/src/capture.rs
- **Commit:** e85800f

**4. [Rule 3 - Blocking] The disk was 98% full at start (895M free)**
- **Fix:** I removed 152 stale duplicate test binaries from target/debug/deps (6.7G), keeping the newest of each, and target/debug/incremental. Builds ran with `CARGO_INCREMENTAL=0`. No tracked file or evidence was affected.

**5. [Test shape] Small details that do not change what the plan's tests prove**
- build_01_record_stage_refuses_a_build_that_rewrote_the_read_only_toolchain_file also asserts that the trusted copy changed in process.
- build_01_toolchains_writes_the_pinned_text_for_cmake_pairs checks both CMake configurations of the fixture (`default`, `depth-8`), and an empty directory for the Make pair.
- The local consistency test counts 99 entries, not the planning figure of 248 (see Local proofs).
- The capture-flag half of build_01_toolchain_dir_needs_stage_build is a guard against the unchanged CLI (see RED evidence).

---

**Total deviations:** 5 (1 lint fix of a RED assertion, 2 critical-correctness additions, 1 environment blocker, 1 group of test-shape details).
**Impact on plan:** none on scope. No gate, comparison, limit, allow-list entry, verdict rule or floor was relaxed, and nothing was dropped or skipped to get green.

## Known Stubs

None.

## Threat Flags

None beyond the plan's register. The toolchains step runs in run_offline: no network, no upstream code, read-only root, user 65534. The control step runs offline, with no capabilities and as user 65534.

## Issues Encountered

- The tool runner blocks foreground `sleep` before a command, so CI was polled by a background loop. It polled every 90 s with a 70-minute bound, and each result was then read with `gh api`.
- The container run of round trip 1 was awaited to the end. Every image-a step succeeded; "Save the gated image" was skipped as on a non-main push, and publish was skipped.

## User Setup Required

None.

## Next Phase Readiness

Gap G-02-8 is fully evidenced across plans 02-21 and 02-22. /gsd-secure-phase 2 can close T-02-67 and T-02-70 in 02-SECURITY.md with the table above, and register T-02-76, T-02-77 and T-02-79.

## Self-Check: PASSED

- Files: build_01_toolchain_evidence.rs, 02-22-CI-NOTICES.md, capture.rs, report.rs, testbed.rs, main.rs, testbed.yml and container.yml are all present.
- Commits: bc02716, e85800f, 4b689d4, b312054, 9273ef8 and 7c55dbe are all ancestors of HEAD.
- Measured: `git rev-list --count ae3df6c..7c55dbe` = 6.
