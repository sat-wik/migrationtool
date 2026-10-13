---
phase: 02-test-bed-and-build-capture
plan: 05
subsystem: testing
tags: [fetch, capture, bear, cmake, make, clang, ast-dump, runner, determinism, git-cache]

requires:
  - phase: 02-test-bed-and-build-capture
    provides: "plan 02-03 manifest and treehash; plan 02-04 cc, elf, replay and report modules; the mt-toolchain runner and curl_argv"
provides:
  - "mt_capture::fetch: fetch by pinned commit or pinned archive into the git-ignored work directory, verify_cached on every reuse"
  - "mt_capture::capture: ToolPaths, RunLayout, CaptureContext, the Make driver (Bear) and CMake driver (per-configuration toolchain file), the silent-wrong-build gates, variant_code_diff"
  - "mt_capture::entries: probe-file clang check of every manifest entry (exact name, signature, header dependency)"
  - "report::CaptureReport, CaptureVerdict, Gates, CapturedObject with an entries map; report::FetchReport"
  - "/.mt-work/ ignored by git and skipped by the Python walk"
affects: [02-06, 02-09, 02-10, testbed capture, testbed replay]

actuals:
  tokens: 46000
  tasks: 3
  commits: 6
plan_head_before: b9430d0e7a2fc40f86756297d1477ee1ac66a14b
plan_head_after: 2113a64e7cf6eb32d51c750b60e3b44191a02bd0

tech-stack:
  added: []
  patterns:
    - "A target-configuration failure is data in the report (verdict plus named causes), Err is only for invalid inputs, a cache that no longer matches its pin, or a tool that cannot start"
    - "Every gate fails closed and accumulates: one failed gate does not hide the others, and a timeout or incomplete stream is not_proven, never a pass"
    - "End-to-end tests run the real Arm compiler, Bear, CMake and clang when the host has them (support::local_arm_tools) and skip themselves, visibly, when it does not"

key-files:
  created:
    - crates/mt-capture/src/fetch.rs
    - crates/mt-capture/src/capture.rs
    - crates/mt-capture/src/entries.rs
    - crates/mt-capture/tests/bed_01_fetch.rs
    - crates/mt-capture/tests/build_01_capture_drivers.rs
    - crates/mt-capture/tests/bed_01_entries.rs
    - crates/mt-capture/tests/support/mod.rs
    - crates/mt-capture/tests/fixtures/ast/probe-dump.json
  modified:
    - crates/mt-capture/src/lib.rs
    - crates/mt-capture/src/report.rs
    - crates/mt-capture/src/replay.rs
    - crates/mt-toolchain/tests/repo_checks.rs
    - .gitignore

key-decisions:
  - "CMake is given the make program as -DCMAKE_MAKE_PROGRAM=<pinned make> on the configure line; set in the toolchain file it was ignored by CMake 3.28 and the compiler probe ran an empty command"
  - "CMake's own diagnostic logs (CMakeConfigureLog.yaml, CMakeOutput.log, CMakeError.log) are not hashed into configure_files: they hold scratch-directory names that change on every run and are inputs to no compile"
  - "The timestamp gate records run-root-relative source keys (src/t.c), not bare file names, so two directories holding a t.c cannot be confused"
  - "An entry passes only if every FunctionDecl dumped under its exact name has the manifest signature; a header-only probe plus exact-name selection keeps the dump small and the match precise"
  - "A clang timeout or incomplete stream during an entry check makes the whole capture not_proven (not entry_mismatch)"

patterns-established:
  - "Probe-file entry check: one #include line, clang-safe flags only, exact-name selection from concatenated JSON documents, -MM for the header dependency"
  - "RED stub commits carry the full public shape so tests fail on assertions, not on missing symbols"

requirements-completed: [BED-01, BED-02, BUILD-01]

coverage:
  - id: D1
    description: "Sources arrive only by pinned commit (git init, shallow fetch, checkout, rev-parse equality, tree sha256) or pinned archive (sha256 before extraction), land in a cache keyed by id and commit, and every reuse is verified again; a tampered cache, wrong tree hash or unknown commit fails naming the cause"
    requirement: BED-01
    verification:
      - kind: integration
        ref: "crates/mt-capture/tests/bed_01_fetch.rs (bed_01_fetch_pins_commit_and_tree_hash, bed_01_fetch_rejects_tree_hash_mismatch, bed_01_fetch_rejects_unknown_commit, bed_01_fetch_reuse_verifies_cache, bed_01_archive_fetch_checks_archive_sha256)"
        status: pass
    human_judgment: false
  - id: D2
    description: ".mt-work/ is git-ignored and skipped by the Python-outside-research walk while a planted a/b.py is still flagged, so fetched trees cannot enter the repository"
    requirement: BED-01
    verification:
      - kind: unit
        ref: "crates/mt-toolchain/tests/repo_checks.rs#tool_01_python_walk_skips_the_work_directory"
        status: pass
    human_judgment: false
  - id: D3
    description: "The Make driver injects the baseline through CC and runs bear, make -j1; the CMake driver writes a toolchain file carrying CMAKE_C_FLAGS_INIT, configures with the export switch and MinSizeRel, never sets CMAKE_C_FLAGS, and rejects manifest options that set what the driver owns; placeholders expand and anything else is rejected"
    requirement: BUILD-01
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/build_01_capture_drivers.rs (build_01_toolchain_file_carries_baseline_flags, build_01_cmake_configure_never_sets_cmake_c_flags, build_01_cmake_build_type_is_minsizerel, build_01_make_argv_injects_flags_through_cc_and_runs_j1, build_01_placeholders_expand_and_unknown_are_rejected)"
        status: pass
    human_judgment: false
  - id: D4
    description: "A real Make capture under Bear and a real CMake capture with the Arm compiler produce the captured verdict with objects, hashes, architecture result, optimisation goal, code digest and configure-time file hashes; a silent recipe is empty_capture, a failing build is build_failed, the host compiler is compiler_not_allowed, soft float is arch_gate, an object Bear never saw is created_undeclared, and __TIMESTAMP__ is timestamp_macro"
    requirement: BUILD-01
    verification:
      - kind: integration
        ref: "crates/mt-capture/tests/build_01_capture_drivers.rs (build_01_capture_make_with_bear_end_to_end, build_01_capture_cmake_end_to_end, build_01_capture_drivers_fail_closed, build_01_created_undeclared_objects_fail_capture, build_01_timestamp_macro_in_sources_fails_capture)"
        status: pass
    human_judgment: false
  - id: D5
    description: "variant_code_diff reports changes_code true for different code or object sets (and for an unreadable digest) and false for an identical build"
    requirement: BED-02
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/build_01_capture_drivers.rs#bed_02_variant_code_diff_detects_changed_code"
        status: pass
    human_judgment: false
  - id: D6
    description: "Each manifest entry is checked with clang on a header-only probe: exact name from concatenated JSON documents, signature equal after whitespace collapse, header among the -MM dependencies of the defining TU; a miss fails the capture with entry_mismatch, a failing clang never passes and a stuck clang is not_proven"
    requirement: BED-01
    verification:
      - kind: integration
        ref: "crates/mt-capture/tests/bed_01_entries.rs (8 tests, bed_01_entry_*)"
        status: pass
    human_judgment: false
  - id: D7
    description: "The drivers, gates and entry check behave the same with the pinned image's Arm GCC 14.3.Rel1, Bear 3.1.1, CMake and clang 16 (this host has Arm GCC 13.2.1, Bear 3.1.x, CMake 3.28.3 and clang 18.1.3)"
    verification: []
    human_judgment: true
    rationale: "No Docker here. Plan 02-06 runs capture in the pinned image under --network none and publishes the real clang 16 AST dump; plan 02-10 commits it."

duration: 85min
completed: 2026-10-13
status: complete
---

# Phase 2 Plan 5: Fetch, capture drivers and entry check Summary

**Pinned-commit and pinned-archive fetch with a re-verified cache, Make-under-Bear and CMake-toolchain-file capture drivers whose every silent-wrong-build failure is a named cause in the report, and a clang probe-file check that confirms each hand-named entry function by exact name, signature and header dependency, all proved offline with the host's Arm compiler, Bear, CMake and clang**

## Performance

- **Duration:** 85 min across two executors (the first was cut off by an API usage limit after Task 2 RED; the second resumed from its commits and its uncommitted work)
- **Completed:** 2026-10-13
- **Tasks:** 3 (all `type="auto"`, `tdd="true"`)
- **Files modified:** 13 (8 created, 5 modified, excluding this summary)

## Accomplishments

- `fetch()` brings a source in by pinned commit (`git init`, shallow `fetch`, `checkout --detach`, `rev-parse HEAD` equal to the pin, tree sha256 equal to the manifest) or by pinned archive (hardened curl flags, archive sha256 checked before `tar --strip-components --no-same-owner` extracts into a fresh directory, then the tree hash). Git runs with HOME and TMPDIR under the work directory and a locked-down environment. `verify_cached()` re-checks the commit or archive and the tree hash on every reuse and names the cache path on a mismatch; it never silently refetches. `/.mt-work/` is ignored and skipped by the Python walk.
- `capture()` copies the verified tree into a fresh run directory, copies the committed inputs, builds, canonicalises the export and applies the gates: allow-listed compiler, ELF header plus Cortex-M4 hard-float tags per object, undeclared objects, `__TIMESTAMP__`, empty or duplicate captures. Each failure is a verdict (`capture_failed`, `build_failed`, `not_proven`) with machine-readable causes, and `capture.json` is written in every case. Original objects are copied to the capture directory for replay.
- The CMake driver writes a per-configuration toolchain file (`CMAKE_C_FLAGS_INIT` with the baseline and the configuration's flags, `CMAKE_TRY_COMPILE_TARGET_TYPE STATIC_LIBRARY`), configures with `CMAKE_EXPORT_COMPILE_COMMANDS=ON` and `CMAKE_BUILD_TYPE=MinSizeRel`, and rejects manifest options that set C flags, build type, toolchain file, compiler, make program or the export switch. Configure-time files (the generated header in the test) are hashed into `configure_files`.
- `entries::check_entry` runs clang on a one-line probe with the defining TU's clang-safe flags and the Arm sysroot, selects the `FunctionDecl` by exact name from the concatenated JSON documents, compares the signature after whitespace collapse and requires the header among the `-MM` dependencies. `capture()` runs it for every manifest entry and fails with `entry_mismatch` on any miss.
- `variant_code_diff` compares code digests of objects with the same path and the object sets, so a variant that changes no code is reported with `changes_code: false`.

## Task Commits

1. **Task 1 (RED): failing fetch tests, ignore and skip .mt-work** - `a45d47d` (test)
2. **Task 1 (GREEN): fetch by commit or archive with a verified cache** - `6df2954` (feat)
3. **Task 2 (RED): failing tests for the Make and CMake capture drivers** - `e2a949c` (test)
4. **Task 2 (GREEN): capture drivers with the silent-wrong-build gates** - `4fea443` (feat)
5. **Task 3 (RED): failing tests for the entry-function check** - `26440e8` (test)
6. **Task 3 (GREEN): entry check wired into capture** - `2113a64` (feat)

There is no REFACTOR commit for any task. **Plan metadata:** the commit that adds this SUMMARY (docs).

## TDD Gate Compliance

RED then GREEN for each task, in order. Each RED commit adds the public shape as a compiling stub and the tests, so the tests fail on their assertions rather than on a compile error.

| Task | RED commit | RED result | GREEN commit |
|------|-----------|------------|--------------|
| 1 | `a45d47d` | recorded by the first executor | `6df2954` |
| 2 | `e2a949c` | recorded by the first executor | `4fea443` |
| 3 | `26440e8` | all 7 tests failed on their assertions (empty parse results, empty argv, `Vec::new()` flags, default `EntryResult`); the eighth test, `bed_01_entry_mismatch_fails_the_capture`, was added in the GREEN commit because it needs the capture wiring | `2113a64` |

Semantic assessment of the Task 3 RED evidence: every failure was an `assert_eq!`, `assert!` or `unwrap` on the behaviour under test, none a syntax, import, fixture or zero-test fault. The `gsd check tdd-red-evidence` classifier was not run: cargo's libtest console output is not one of its supported report formats, and no record was fabricated. The first executor's RED output for Tasks 1 and 2 was not carried over to me; I did not re-run those stubs.

## Files Created/Modified

- `crates/mt-capture/src/fetch.rs` - `FetchContext`, `CachePaths`, `fetch`, `verify_cached`, `FetchError`, git and archive paths
- `crates/mt-capture/src/capture.rs` - `ToolPaths`, `RunLayout`, `CaptureContext`, argv builders, `toolchain_file`, gates, `capture`, `variant_code_diff`
- `crates/mt-capture/src/entries.rs` - probe source, flag filter, AST stream parser, signature normalisation, `-MM` parser, `check_entry`
- `crates/mt-capture/src/report.rs` - `FetchReport`, `CaptureReport` (with `entries`), `CaptureVerdict`, `Gates`, `CapturedObject`
- `crates/mt-capture/tests/bed_01_fetch.rs`, `build_01_capture_drivers.rs`, `bed_01_entries.rs`, `support/mod.rs`, `fixtures/ast/probe-dump.json` - tests, shared support (local git upstream, target builders, local Arm tools) and the clang 18.1.3 dump
- `crates/mt-toolchain/tests/repo_checks.rs`, `.gitignore` - `.mt-work` skipped and ignored

## Decisions Made

See key-decisions. Binding for plans 02-06 and 02-09:

- Step names in `CaptureReport.steps`: `build` (Make), `cmake configure`, `cmake build`, `readelf <key>`, `clang ast <fn>`, `clang deps <fn>`, plus the fetch verification steps. Object keys are run-root-relative (`src/t.o`, `build/CMakeFiles/tj.dir/t.c.obj`).
- With `CMAKE_SYSTEM_NAME Generic`, CMake names objects `*.obj`. The undeclared-object gate counts both `.o` and `.obj`.
- The standard output digest of the `cmake configure` step is not stable between runs: CMake prints `-- Configuring done (0.2s)`. Anything that compares two CMake capture reports byte for byte (plan 02-06's workflow, plan 02-09) must ignore that digest.
- `ToolPaths::from_pins` derives readelf, objcopy and the sysroot from the pinned Arm compiler's install directory and records the pinned `expect` versions of arm_gcc, bear, clang, cmake and make.

## Deviations from Plan

### Interruption and resume

**[Process] Executor interrupted by an API usage limit and resumed**
- **Found during:** between Task 2 RED and Task 2 GREEN
- **What happened:** the first executor committed Tasks 1 (RED, GREEN) and 2 (RED) and was cut off mid-edit while adding the shared test support module, leaving 1302 lines of uncommitted Task 2 GREEN work. This executor fast-forwarded to the first executor's branch (`a45d47d`, `6df2954`, `e2a949c` confirmed), applied the saved diff (`/tmp/02-05-wip.diff`), reviewed it against the plan and the research, fixed it where it was wrong (below) and committed it as Task 2 GREEN. The measured commit count (6) runs from `b9430d0` and includes the first executor's three commits.
- **Review result:** the diff to `fetch.rs` was one line (`pinned_tool` made `pub(crate)` so `ToolPaths::from_pins` can reuse it) and was kept. The diff to the committed Task 2 test changed one expectation, recorded below.

### Auto-fixed Issues

**1. [Rule 1 - Bug] CMake make program set in the toolchain file was ignored**
- **Found during:** Task 2 GREEN (`build_01_capture_cmake_end_to_end` failed with `build_failed`)
- **Issue:** the saved work wrote `set(CMAKE_MAKE_PROGRAM ...)` in the toolchain file. CMake 3.28.3 ignored it and the compiler probe ran `cmake -E env VERBOSE=1 "" -f Makefile`, failing configure.
- **Fix:** removed it from the toolchain file (the research text has no such line) and pass `-DCMAKE_MAKE_PROGRAM=<pinned make>` on the configure line; `CMAKE_MAKE_PROGRAM` joined the options the manifest may not set.
- **Files modified:** `crates/mt-capture/src/capture.rs`
- **Commit:** `4fea443`

**2. [Rule 1 - Bug] Timestamp gate could name the wrong file**
- **Found during:** Task 2 GREEN review
- **Issue:** the saved work de-duplicated stamped sources by bare file name and then looked the entry up by that name, so two directories holding a `t.c` would collapse into one and could report the wrong one.
- **Fix:** an internal `timestamp_entries` returns the entries themselves; the report records run-root-relative keys. `timestamp_macro_sources` keeps returning the `file` strings.
- **Files modified:** `crates/mt-capture/src/capture.rs`
- **Commit:** `4fea443`

**3. [Rule 1 - Bug] CMake's configure log made `configure_files` nondeterministic**
- **Found during:** Task 2 GREEN (`build_01_capture_cmake_end_to_end` rerun comparison)
- **Issue:** `build/CMakeFiles/CMakeConfigureLog.yaml` holds scratch-directory names that change on every run and feeds no compile.
- **Fix:** CMake's three diagnostic logs are skipped when hashing configure-time files.
- **Files modified:** `crates/mt-capture/src/capture.rs`
- **Commit:** `4fea443`

### Committed tests changed (recorded as required)

**4. [Test expectation] CMake objects are `*.obj`, not `*.o`**
- **Test:** `build_01_capture_cmake_end_to_end` (committed in RED `e2a949c`)
- **Why:** with `CMAKE_SYSTEM_NAME Generic`, CMake 3.28.3 names the object `build/CMakeFiles/tj.dir/t.c.obj`. The RED test expected `t.c.o`, a guess the plan itself did not make (the plan's gate counts both `.o` and `.obj`). The assertion now expects `.obj`, and the check that configure-time files contain no products excludes `.obj` instead of `.o`. It is a fact about the pinned build system, not a weakened check.
- **Commit:** `4fea443`

**5. [Test expectation] The timestamp gate reports run-root-relative keys**
- **Test:** `build_01_timestamp_macro_in_sources_fails_capture`
- **Why:** the saved work changed `gates.timestamp_macro` from `["t.c"]` to `["src/t.c"]`; see deviation 2. The unit-level assertion on `timestamp_macro_sources` (returns `["stamp.c"]`) is unchanged.
- **Commit:** `4fea443`

**6. [Rule 1 - Bug in test] CMake rerun comparison was flaky**
- **Test:** `build_01_capture_cmake_end_to_end`
- **Issue:** the test asserted the reports of two runs are byte-identical. They differ about one run in eight in the standard output digest of `cmake configure`, because CMake prints its own elapsed time (`-- Configuring done (0.2s)`; `--log-level` does not remove it).
- **Fix:** the comparison now removes the standard output digest and byte counts of the `cmake configure` step before comparing; every other field, including the `cmake build` step, must still match. Fifteen consecutive runs passed afterwards. The Make test's byte-for-byte comparison is unchanged.
- **Commit:** `2113a64`

### Plan deviations in scope

**7. [Plan wording] The eighth entry test**
- Task 3 lists six tests; the file has eight: the six planned (the AST-stream, exact-name, signature-whitespace, missing-function, flag-filter and header-dependency tests), `bed_01_entry_a_failing_or_stuck_clang_never_passes` (a non-zero or timed-out clang is never a pass) and `bed_01_entry_mismatch_fails_the_capture` (the capture-level `entry_mismatch` wiring). Both were added because the plan's behaviour list requires them to be provable.
- **Fixture caveat:** `probe-dump.json` was generated with the host clang 18.1.3 (command in the test file's doc comment). Its second document repeats `loc.file`, so the carry-across-documents behaviour is proved by an inline stream in the same test, not by the fixture alone. Plan 02-06's workflow publishes the real clang 16 dump.

**Total deviations:** 3 auto-fixed bugs, 3 committed tests changed, 1 interruption and resume, 1 plan-wording addition. **Impact:** none on the plan's contract; all tests the plan names pass.

## Authentication Gates

None.

## Issues Encountered

- The first executor's RED output for Tasks 1 and 2 was not available to this executor, so the TDD table above does not repeat it.
- The host differs from the pinned image (Arm GCC 13.2.1 vs 14.3.Rel1, clang 18 vs 16). Behaviour that depends on the version is listed as human-judgment item D7 and settled by plans 02-06 and 02-10.

## Known Stubs

None. No placeholder text, hardcoded empty values flowing to output, or unwired components in the files this plan created.

## Threat Flags

None beyond the plan's threat model. Capture executes upstream Makefiles and CMake scripts (T-02-22); that is mitigated by plan 02-06 running it only inside the pinned image with `--network none`, and the offline tests build only test-authored sources.

## Self-Check: PASSED

- Files: `fetch.rs`, `capture.rs`, `entries.rs`, the three test files, `support/mod.rs` and `fixtures/ast/probe-dump.json` all exist.
- Commits: `a45d47d`, `6df2954`, `e2a949c`, `4fea443`, `26440e8` and `2113a64` are ancestors of HEAD; `git rev-list --count b9430d0..HEAD` is 6 before this summary's commit.
- Acceptance: `grep -c '^/.mt-work/$' .gitignore` prints 1 and `ROOT_SKIPS` holds `.mt-work`; `capture.rs` contains `CMAKE_C_FLAGS_INIT`, `CMAKE_TRY_COMPILE_TARGET_TYPE STATIC_LIBRARY`, `-DCMAKE_BUILD_TYPE=MinSizeRel` and `-j1`; `entries.rs` contains `-ast-dump-filter=` and `--sysroot=`; the fixture holds two concatenated JSON documents.
- Plan verification: `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --workspace --locked` (no failing test result) and `cargo deny check` (advisories, bans, licenses and sources ok) each exited 0 on the last code commit.
