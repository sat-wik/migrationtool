---
phase: 02-test-bed-and-build-capture
plan: 12
subsystem: testing
tags: [cmake, makefile2, normalisation, configure-evidence, capture-report, replay, g-02-2, wr-10]

requires:
  - phase: 02-test-bed-and-build-capture
    provides: "plan 02-06 CMake capture with configure_files; plan 02-09 measured Makefile2 pointer-order nondeterminism and the committed corpora"
provides:
  - "CMakeFiles/Makefile2 is back in the configure-time evidence as a normalised hash (normalise_make_driver, MAKE_DRIVER_RULE), recorded in CaptureReport.normalised_configure_files with its rule"
  - "configure_evidence_changed(run_root, report): the single replay-time re-check of raw and normalised configure-time files (replaces testbed.rs changed_configure_file)"
  - "is_cmake_product matches exactly the documented CMake probe paths; excluded_from_configure_hashes is the pure skip decision that Session::configure_files uses for every skip"
  - "CONFIGURE_HASH_EXCLUSIONS (5 lines) and NORMALISED_CONFIGURE_FILES (1 line) published for plan 02-17's testbed.yml determinism comment"
affects: [02-17 testbed.yml determinism comment and image proof, phase-2 verification]

actuals:
  tokens: 11350
  tasks: 2
  commits: 4
plan_head_before: a62d9b933dc1eb65208770d457555866d6a06675
plan_head_after: 1889fbcbbb9b2f0a1950b0c99c529cdab81ddb2a

tech-stack:
  added: []
  patterns: ["a pure, tool-free decision function (excluded_from_configure_hashes) carries the exclusion rule so the tool-free ci workflow guards it; the capture-level test stays as the end-to-end proof", "normalisation recorded as rule text next to its hash in the report", "REPORT additions use serde(default, skip_serializing_if) so Make reports and committed CI fixtures stay byte-identical"]

key-files:
  created:
    - "crates/mt-capture/tests/build_02_configure_evidence.rs"
    - "crates/mt-capture/tests/fixtures/makefile2/zcbor-pet-a.txt"
    - "crates/mt-capture/tests/fixtures/makefile2/zcbor-pet-b.txt"
  modified:
    - "crates/mt-capture/src/capture.rs"
    - "crates/mt-capture/src/report.rs"
    - "crates/mt-capture/src/testbed.rs"
    - "crates/mt-capture/tests/build_01_capture_drivers.rs"

key-decisions:
  - "G-02-2: Makefile2 is hashed after normalising each run of consecutive prerequisite lines of one target (bytewise sort); the normalisation touches that one configure-time file only, never an object or the D-09 rule"
  - "An .a or .so is left out of the configure-time hashes only when the cmake build step created it (absent from the listing taken after cmake configure); a configure-time look-alike such as gen/table.a is hashed"
  - "Objects are left out by declaration only (keys of report.objects, run-root keys); the suffix rule is gone"

patterns-established:
  - "Exclusion sets are code constants (CONFIGURE_HASH_EXCLUSIONS, NORMALISED_CONFIGURE_FILES) that a static test in another plan can hold documentation to"

requirements-completed: [BUILD-01, BUILD-02]

coverage:
  - id: D1
    description: "CMakeFiles/Makefile2 is hashed normalised, recorded with MAKE_DRIVER_RULE in normalised_configure_files, and absent from the raw configure_files"
    requirement: BUILD-01
    verification:
      - kind: integration
        ref: "crates/mt-capture/tests/build_01_capture_drivers.rs#build_02_cmake_capture_hashes_makefile2_normalised"
        status: pass
      - kind: unit
        ref: "crates/mt-capture/tests/build_02_configure_evidence.rs#build_02_make_driver_normalisation_sorts_runs_of_one_target"
        status: pass
      - kind: unit
        ref: "crates/mt-capture/tests/build_02_configure_evidence.rs#build_02_real_makefile2_variants_normalise_equal"
        status: pass
    human_judgment: false
  - id: D2
    description: "Replay re-checks raw and normalised configure-time files through configure_evidence_changed: a line swap in one target's run is no change, a changed prerequisite or any changed or missing raw file is named"
    requirement: BUILD-02
    verification:
      - kind: integration
        ref: "crates/mt-capture/tests/build_01_capture_drivers.rs#build_02_replay_rechecks_the_normalised_makefile2"
        status: pass
    human_judgment: false
  - id: D3
    description: "is_cmake_product matches exactly the documented paths and the skip decision is the pure function excluded_from_configure_hashes (WR-10)"
    requirement: BUILD-01
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/build_02_configure_evidence.rs#build_01_cmake_products_are_exactly_the_documented_paths"
        status: pass
      - kind: unit
        ref: "crates/mt-capture/tests/build_02_configure_evidence.rs#build_01_configure_hash_skip_decision_matches_the_documented_set"
        status: pass
      - kind: integration
        ref: "crates/mt-capture/tests/build_01_capture_drivers.rs#build_01_configure_files_skip_only_build_created_archives"
        status: pass
    human_judgment: false
  - id: D4
    description: "The exclusion set and the normalised file are published as constants for plan 02-17"
    requirement: BUILD-01
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/build_02_configure_evidence.rs#build_01_configure_hash_exclusions_are_published"
        status: pass
    human_judgment: false
  - id: D5
    description: "Image proof that determinism is identical with Makefile2 hashed (plan 02-17's testbed run)"
    verification: []
    human_judgment: true
    rationale: "The determinism comparison across two configures in the pinned image is plan 02-17's step; this plan proves the rule on 20 local configures and in tests only"

duration: 10min
completed: 2026-10-19
status: complete
---

# Phase 2 Plan 12: Normalised Makefile2 hash and exact CMake product set Summary

**CMakeFiles/Makefile2 returns to the configure-time evidence as a hash of its prerequisite-line-sorted form, recorded with its rule and re-checked before replay, and is_cmake_product is an exact path list with archives excluded only when the build step created them.**

## Performance

- **Duration:** about 10 min (start about 21:16Z, end 21:26Z; the start time was not captured at spawn, so this is estimated from the previous plan's STATE timestamp)
- **Tasks:** 2 (a tracer and a TDD task, each RED then GREEN)
- **Files modified:** 7 (3 created, 4 modified)

## Accomplishments

- G-02-2 items 1 and 2: `normalise_make_driver` / `MAKE_DRIVER_RULE` and `configure_evidence_changed` in capture.rs; `NormalisedFile` and `CaptureReport.normalised_configure_files` in report.rs (omitted from the JSON when empty, so Make reports and the committed CI fixtures are unchanged: `build_01_ci_fixtures` passes untouched); testbed.rs `plan_of` calls `configure_evidence_changed` and its private `changed_configure_file` is deleted.
- WR-10: `is_cmake_product` is the three CMake logs, `CMakeFiles/<version>/CMakeDetermineCompilerABI_<C|CXX>.bin` and `CMakeFiles/<version>/CompilerId<C|CXX>/CMake<C|CXX>CompilerId.o` with an all-digits-and-dots version; no suffix test remains. `excluded_from_configure_hashes(relative, declared, before)` is pure and is the only skip decision in `Session::configure_files`.
- `CONFIGURE_HASH_EXCLUSIONS: [&str; 5]` and `NORMALISED_CONFIGURE_FILES: [&str; 1]` published with the exact strings plan 02-17 will require in the testbed.yml determinism comment.
- Normalisation applies to `CMakeFiles/Makefile2` only; no object, object comparison or D-09 rule changed (stated in the doc comment of `normalise_make_driver`).

## Measurement: 20 configures of the zcbor sample

Cached zcbor `samples/pet`, host CMake 3.28.3, the capture's own `toolchain.cmake`, a fresh build directory per run, `Makefile2` hashed after rewriting the per-run build and source prefixes to `/work/run/zcbor/default/{build,src}`:

- Raw hashes: 20 distinct (the per-run build directory name is inside the file).
- After the prefix rewrite: exactly 2 variants, 12 and 8 of 20.
  - `13cfe2a9792ecb71a17b2ee4738705ba828f3d8c2c37ea9e3baa4038fc630e12` (12 runs, committed as `zcbor-pet-a.txt`)
  - `466124be5f85de6f3f3ca6779d5286c9a880cd9c8d6c15099e0b9bc7220d47de` (8 runs, committed as `zcbor-pet-b.txt`)
- The only difference is the order of `CMakeFiles/app.dir/all: CMakeFiles/pet1_yaml.dir/all` and `CMakeFiles/app.dir/all: CMakeFiles/pet.dir/all` (`diff` shows lines 136 and 137 swapped).
- Both normalise to the same bytes (`build_02_real_makefile2_variants_normalise_equal`).

## RED evidence (compile errors and failing assertions against the unchanged implementation)

Tracer RED commit `3527a89` (test code only, no production edit):

```
error[E0432]: unresolved import `mt_capture::capture::normalise_make_driver`
  --> crates/mt-capture/tests/build_02_configure_evidence.rs:12:5
error[E0432]: unresolved imports `mt_capture::capture::MAKE_DRIVER_RULE`, `mt_capture::capture::configure_evidence_changed`, `mt_capture::capture::normalise_make_driver`
  --> crates/mt-capture/tests/build_01_capture_drivers.rs:19:46
error[E0609]: no field `normalised_configure_files` on type `CaptureReport`   (x3)
```

Task 2 RED commit `92f94c8` (test code only):

```
error[E0432]: unresolved imports `mt_capture::capture::CONFIGURE_HASH_EXCLUSIONS`,
  `mt_capture::capture::NORMALISED_CONFIGURE_FILES`, `mt_capture::capture::excluded_from_configure_hashes`
  --> crates/mt-capture/tests/build_02_configure_evidence.rs:14:5
```

Because the new pure file cannot compile until the new names exist, the assertion-level RED of the two tests that call only existing code was taken separately, against the unchanged implementation:

```
test build_01_configure_files_skip_only_build_created_archives ... FAILED
  panicked at build_01_capture_drivers.rs:861: report.configure_files.contains_key("build/gen/table.a")
  (the suffix rule hides gen/table.a)
test build_01_cmake_products_are_exactly_the_documented_paths ... FAILED   (run from a temporary, uncommitted copy of the test)
  panicked: x.o is not a CMake product
```

## Task Commits

1. **Tracer RED:** `3527a89` (test) - failing tests for the normalised Makefile2 hash, two real zcbor variants as fixtures
2. **Tracer GREEN:** `2ba0232` (feat) - normalised Makefile2 hash, NormalisedFile, configure_evidence_changed
3. **Task 2 RED:** `92f94c8` (test) - failing tests for the exact CMake product set, the pure skip decision and the published constants
4. **Task 2 GREEN:** `1889fbc` (feat) - exact is_cmake_product, excluded_from_configure_hashes, exclusion constants

Tracer gate: `<verify>` is automated-only and interactive mode is `end-of-phase`, so the tracer verify was re-run (all passed) and execution expanded to Task 2. No REFACTOR commit was needed.

## Committed tests whose expectation changed (named per the TDD rule)

- `build_01_cmake_probe_outputs_are_not_configure_files` (build_01_capture_drivers.rs), twice:
  1. `CMakeFiles/Makefile2` moved from the product list to the input list. Reason: founder decision G-02-2 makes it a hashed (normalised) file.
  2. `CMakeFiles/tj.dir/t.c.obj`, `CMakeFiles/tj.dir/t.c.o`, `libtj.a` and `libtj.so` removed from the product list. Reason: founder decision G-02-2 with review WR-10 replaces the suffix rule with declaration and build provenance. The four assertions now live in the pure test `build_01_configure_hash_skip_decision_matches_the_documented_set` (no tool, runs in the tool-free ci workflow) and end to end in `build_01_configure_files_skip_only_build_created_archives`. Every input case of the old test is kept.

No check was weakened and no validation code was removed to manufacture a failure.

## Where each path class the old suffix rule hid is handled now

Measured on the local CMake build trees under `.mt-work/run/*/*/build` (cjson, corejson, coremqtt, isotp, nanopb, paho, zcbor, zlib, tinycbor):

| Old rule hid | Handled now | Nothing new excluded |
|---|---|---|
| `CMakeFiles/<t>.dir/**.obj`, `src/CMakeFiles/<t>.dir/**.obj`, zcbor `CMakeFiles/pet.dir/<abs path>/**.obj` | declared object (key in `report.objects`) | same set as before for every captured configuration |
| `CMakeFiles/<version>/CompilerIdC/CMakeCCompilerId.o`, `CompilerIdCXX/CMakeCXXCompilerId.o` | exact CMake product | same files |
| `CMakeFiles/<version>/CMakeDetermineCompilerABI_C.bin`, `_CXX.bin` | exact CMake product | same files |
| `CMakeFiles/CMakeConfigureLog.yaml` (and the two other logs) | exact CMake product | same files |
| `libcjson.a`, `libcore_json.a`, `libz.a`, `src/libMQTTPacketClient.a`, ... | build-created archive (absent from the post-configure listing) | same files |
| `CMakeFiles/Makefile2` | now hashed, normalised | no longer excluded |
| a configure-time `*.a`/`*.so` (e.g. `gen/table.a`) | now hashed | no longer excluded |

The one oddity in the measurement: tinycbor's `.obj` files are not in its capture report, because both tinycbor captures in `.mt-work` are `build_failed`; `configure_files` is never reached for a failed build, so nothing is hashed or excluded for it either way.

## Files Created/Modified

- `crates/mt-capture/src/capture.rs` - normalise_make_driver, MAKE_DRIVER_RULE, configure_evidence_changed, exact is_cmake_product, excluded_from_configure_hashes, the two published constants, Session::configure_files rewritten around them
- `crates/mt-capture/src/report.rs` - NormalisedFile, CaptureReport.normalised_configure_files
- `crates/mt-capture/src/testbed.rs` - plan_of uses configure_evidence_changed; private changed_configure_file deleted
- `crates/mt-capture/tests/build_02_configure_evidence.rs` - pure tests (7)
- `crates/mt-capture/tests/build_01_capture_drivers.rs` - 3 new capture-level tests, adjusted probe test
- `crates/mt-capture/tests/fixtures/makefile2/zcbor-pet-{a,b}.txt` - the two measured variants

## Decisions Made

- Normalisation scope, skip rule and constants as in key-decisions above; followed the plan as specified.
- `configure_evidence_changed` compares against the current `normalise_make_driver` only; the stored rule text is not compared (the plan did not ask for it, and an unrequested check would have had no RED test).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Too-broad assertion in my own new capture-level test**
- **Found during:** Task 2 GREEN (first run of `build_01_configure_files_skip_only_build_created_archives`)
- **Issue:** the test rejected any key containing `CompilerId`, but `build/CMakeFiles/3.28.3/CompilerIdC/CMakeCCompilerId.c` is a source file that was never hidden and is correctly hashed; the assertion would have failed for a correct implementation.
- **Fix:** narrowed to `key.ends_with("CompilerId.o")` (the compiler-id object), keeping the ABI-probe, `.obj` and `libtj.a` assertions.
- **Files modified:** crates/mt-capture/tests/build_01_capture_drivers.rs
- **Verification:** the test passes with the narrowed assertion and still failed in RED on `gen/table.a`.
- **Committed in:** 1889fbc (Task 2 GREEN)

---

**Total deviations:** 1 auto-fixed (1 bug in a new test)
**Impact on plan:** none on behaviour; the check is exactly the plan's "neither the compiler-id object nor the ABI probe".

## Issues Encountered

- The scratch RED run of `build_01_cmake_products_are_exactly_the_documented_paths` against the unchanged implementation used a temporary, uncommitted test file (deleted afterwards), because the pure test file cannot compile before the new names exist.

## Known Stubs

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plan 02-17 can tie the testbed.yml determinism comment to `CONFIGURE_HASH_EXCLUSIONS` and `NORMALISED_CONFIGURE_FILES` and run the image proof that determinism is identical with Makefile2 hashed.
- Full gate green: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace --locked` (capture-level tests ran with the local Arm tools, no "skipping" line), `cargo deny check` (advisories, bans, licenses, sources ok).

## Self-Check: PASSED

- Created files exist: build_02_configure_evidence.rs, zcbor-pet-a.txt, zcbor-pet-b.txt (FOUND).
- Commits 3527a89, 2ba0232, 92f94c8, 1889fbc are ancestors of HEAD (FOUND).
- capture.rs contains `pub fn normalise_make_driver`, `pub const MAKE_DRIVER_RULE`, `pub fn configure_evidence_changed`, `pub fn excluded_from_configure_hashes`; testbed.rs calls `configure_evidence_changed`.

---
*Phase: 02-test-bed-and-build-capture*
*Completed: 2026-10-19*
