---
phase: 02-test-bed-and-build-capture
plan: 04
subsystem: testing
tags: [replay, compile-commands, shlex, object, elf, readelf, objcopy, runner, determinism]

requires:
  - phase: 02-test-bed-and-build-capture
    provides: "plan 02-03 mt-capture crate (manifest, treehash::sha256_hex) and the mt-toolchain runner"
provides:
  - "mt_capture::cc: canonical compile_commands form (CcEntry, RawEntry, CcPolicy, Canonical, CcError, parse_raw, canonicalise, canonical_output, to_json)"
  - "mt_capture::replay: clean-copy replay of captured compiles through the runner with per-object comparison and the fixed D-09 normalisation"
  - "mt_capture::report: deterministic ReplayReport, Verdict, ObjectStatus, NORMALISATION_RULE"
  - "mt_capture::elf: section digests, differing_sections, normalisable, ELF32/ARM header check, readelf -A gate, code_digest"
  - "treehash::copy_tree: symlink-safe tree copy"
  - "Deserialize on RunRecord, ToolStamp, ExitRecord, StreamRecord"
affects: [02-05, 02-06, 02-09, 02-10, testbed capture, testbed replay]

actuals:
  tokens: 24000
  tasks: 3
  commits: 7
plan_head_before: 62844a820195cb934b96829daedc2044c91bc519
plan_head_after: af9ad7f8ea9bdd431efc5b6db1bcf44d2bb7e342

tech-stack:
  added:
    - "shlex 2.0.1 (exact pin, MIT OR Apache-2.0)"
    - "object 0.40.0 (exact pin, features read_core, elf, std; Apache-2.0 OR MIT)"
  patterns:
    - "Replay is data, not control flow: compile failures, timeouts, missing objects are report fields; Err is only for invalid plans, refused compilers or paths, and a child that cannot start"
    - "Every argv passes the compiler allow-list and run-root containment before anything is deleted, copied or spawned"
    - "Reports hold BTreeMap keys, a schema_version and no timestamps, so the same report serialises to the same bytes"
    - "Gates fail closed: an unreadable tag, an unparseable object or a timeout never reads as identical"

key-files:
  created:
    - crates/mt-capture/src/cc.rs
    - crates/mt-capture/src/replay.rs
    - crates/mt-capture/src/report.rs
    - crates/mt-capture/src/elf.rs
    - crates/mt-capture/tests/build_02_replay.rs
    - crates/mt-capture/tests/build_01_compile_commands.rs
    - crates/mt-capture/tests/build_02_normalisation.rs
    - crates/mt-capture/tests/fixtures/cc/bear-raw.json
    - crates/mt-capture/tests/fixtures/cc/cmake-raw.json
  modified:
    - crates/mt-toolchain/src/runner.rs
    - crates/mt-capture/src/lib.rs
    - crates/mt-capture/src/treehash.rs
    - crates/mt-capture/Cargo.toml
    - Cargo.toml
    - Cargo.lock

key-decisions:
  - "replay() refuses a plan whose entry output differs from canonical_output(entry), whose policy run root differs from the plan run root, or in which two entries write one output: the output field decides what is deleted and hashed, so it must agree with the arguments"
  - "An object whose original file cannot be read, or that the ELF reader refuses, is differs with the reason in detail, never an error and never normalised"
  - "copy_tree checks link text lexically and then resolves every copied link physically, because a chain of links can leave the tree where the text of each does not"
  - "canonicalise gates every exported entry, built or not, before dropping unbuilt ones; a tampered unbuilt entry is still rejected"
  - "parse_readelf_a is infallible and fail-closed: one regex compiled once in a LazyLock<Option<Regex>>, a missing tag is None and fails the gate"

patterns-established:
  - "Offline test for the real-target logic: build with host cc through the runner, wipe, restore, replay, compare"
  - "Verdict precedence not_proven, build_failed, differs, identical_after_normalisation, identical; zero objects is build_failed with detail empty_capture"

requirements-completed: [BUILD-01, BUILD-02]

coverage:
  - id: D1
    description: "A real two-file host-cc build, wiped, restored from its pristine tree and replayed through the runner reports identical with every object hash equal; one changed source byte reports differs and names the changed object; a stale planted object and an invalid source can never read identical"
    requirement: BUILD-02
    verification:
      - kind: integration
        ref: "crates/mt-capture/tests/build_02_replay.rs#build_02_replay_matches_original_objects"
        status: pass
      - kind: integration
        ref: "crates/mt-capture/tests/build_02_replay.rs#build_02_replay_detects_changed_object"
        status: pass
      - kind: integration
        ref: "crates/mt-capture/tests/build_02_replay.rs#build_02_clean_copy_replay_removes_stale_outputs"
        status: pass
    human_judgment: false
  - id: D2
    description: "Missing objects on either side are differs, an empty plan is build_failed with empty_capture, a non-allow-listed compiler is refused before any spawn, and the report serialises to identical bytes twice"
    requirement: BUILD-02
    verification:
      - kind: integration
        ref: "crates/mt-capture/tests/build_02_replay.rs (build_02_missing_object_is_reported_as_differs, build_02_object_without_original_is_reported_as_differs, build_02_zero_object_configuration_never_identical, build_02_replay_refuses_non_allowlisted_compiler, build_02_report_is_deterministic)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Run trees are copied with copy_tree: relative in-tree symlinks stay symlinks, the execute bit survives, only the top-level .git is skipped, and absolute or escaping links are refused; run records round-trip through JSON"
    requirement: BUILD-02
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/build_02_replay.rs#bed_01_tree_copy_preserves_symlinks_and_refuses_escapes"
        status: pass
      - kind: unit
        ref: "crates/mt-capture/tests/build_02_replay.rs#build_02_run_record_round_trips_through_json"
        status: pass
    human_judgment: false
  - id: D4
    description: "Bear and CMake exports become one canonical, sorted, gated compile_commands form: shlex split without a shell, output from -o or <stem>.o and never the exported field, unbuilt entries dropped and counted, and each gate (compiler, -flto/-fprofile-*, -c, run root, empty, duplicate output) rejects with its own error"
    requirement: BUILD-01
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/build_01_compile_commands.rs (12 tests, build_01_*)"
        status: pass
    human_judgment: false
  - id: D5
    description: "Differing sections are listed per object, only .comment and debug differences are forgiven by objcopy on both objects, a .text difference stays differs with no objcopy step, the rule is in every report, and unparseable objects are differs with the reader error"
    requirement: BUILD-02
    verification:
      - kind: integration
        ref: "crates/mt-capture/tests/build_02_normalisation.rs (build_02_normalisation_only_forgives_comment_and_debug_sections, build_02_normalisation_rejects_text_section_difference, build_02_equal_objects_are_identical_without_normalisation, build_02_unparseable_object_is_reported_as_differs, build_02_code_digest_ignores_non_code_sections)"
        status: pass
    human_judgment: false
  - id: D6
    description: "The Cortex-M4 hard-float architecture gate: ELF32 little-endian ARM header and the four readelf -A tags, with the optimisation goal recorded and not enforced"
    requirement: BUILD-01
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/build_02_normalisation.rs#build_01_architecture_gate_parses_readelf_attributes"
        status: pass
      - kind: integration
        ref: "crates/mt-capture/tests/build_02_normalisation.rs#host_objects::build_01_captured_objects_are_cortex_m4_hard_float"
        status: pass
    human_judgment: false
  - id: D7
    description: "The gate and the normalisation work on real Arm objects built by arm-none-eabi-gcc 14.3.rel1"
    requirement: BUILD-02
    verification: []
    human_judgment: true
    rationale: "This session has no Arm cross compiler. The logic is proved on host ELF64 objects and a hand-built ARM header; real Arm objects come from the CI workflow of plan 02-06 and are committed as fixtures in plan 02-10."

duration: 14min
completed: 2026-10-13
status: complete
---

# Phase 2 Plan 4: Offline capture/replay core of mt-capture Summary

**Clean-copy replay of captured compile commands through the runner with per-object comparison, the canonical Bear/CMake compile_commands form with its gates, and ELF section comparison with the Cortex-M4 architecture gate and the fixed D-09 normalisation, all proved offline on a real host-cc project**

## Performance

- **Duration:** 14 min
- **Started:** 2026-10-13T04:35:27Z
- **Completed:** 2026-10-13T04:50:00Z
- **Tasks:** 3 (all `type="auto"`, `tdd="true"`)
- **Files modified:** 15 (9 created, 6 modified, excluding this summary)

## Accomplishments

- `replay()` deletes the source directory, restores the pristine tree with `copy_tree`, removes every declared output and runs each entry as an argv vector through `runner::run` with `cwd = directory`. It hashes each rebuilt object against the original and returns a report; a timeout or incomplete stream reads `not_proven`, a non-zero exit `build_failed`, a missing or differing object `differs`. Zero objects is `build_failed` with detail `empty_capture`.
- The D-26 tiny project (`a.c`, `b.c`, `b.h`, built by `/usr/bin/cc`) proves it end to end: identical on a clean replay, `differs` naming `src/out/a.o` after a one-byte source change while `b.o` stays identical, and `build_failed` (never identical) when a good object is planted at the output path and its source no longer compiles.
- `cc::canonicalise` turns Bear `arguments` and CMake `command` strings (split with `shlex`) into one sorted form. The output comes from `-o`/`-o<path>` or `<stem>.o`, never from the exported field. Unbuilt entries are dropped and counted; `NotCompileOnly`, `CompilerNotAllowed`, `ForbiddenFlag` (`-flto*`, `-fprofile-*`), `OutsideRunRoot`, `Empty` and `DuplicateOutput` are separate errors.
- `elf` compares sections by name and occurrence index, forgives only `.comment`, `.debug_*`, `.rel.debug_*` and `.rela.debug_*` (objcopy `--strip-debug --remove-section=.comment` on both objects through the runner), and carries the architecture gate (ELF32 LE ARM relocatable plus the four `readelf -A` tags; optimisation goal recorded, not enforced) and `code_digest`.
- `RunRecord`, `ToolStamp`, `ExitRecord` and `StreamRecord` now deserialize, so replay reports that embed run records round-trip.

## Task Commits

1. **Task 1 (groundwork): Deserialize on the run record types** - `26167b3` (feat)
2. **Task 1 (RED): failing replay tests over a two-file host-cc project** - `b7b35a0` (test)
3. **Task 1 (GREEN): replay, report and copy_tree** - `ecdaefb` (feat)
4. **Task 2 (RED): failing tests for the canonical compile_commands form** - `6696dbd` (test)
5. **Task 2 (GREEN): canonicalise with the capture gates** - `20436f9` (feat)
6. **Task 3 (RED): failing tests for section comparison and normalisation** - `9601447` (test)
7. **Task 3 (GREEN): elf module and replay normalisation** - `af9ad7f` (feat)

There is no REFACTOR commit for any task. **Plan metadata:** the commit that adds this SUMMARY (docs).

## TDD Gate Compliance

RED then GREEN for each task, in order. Each RED commit adds the types, a stub that compiles, and the tests, so the tests fail on their planned assertions rather than on a compile error.

| Task | RED commit | RED result | GREEN commit |
|------|-----------|------------|--------------|
| 1 | `b7b35a0` | 8 of the 9 planned tests failed on the planned assertions (plus 1 extra); `build_02_run_record_round_trips_through_json` passed because the Deserialize commit `26167b3` precedes it | `ecdaefb` |
| 2 | `6696dbd` | 11 of 12 failed; `build_01_empty_capture_is_a_failure` passed vacuously because the stub returns `Empty` | `20436f9` |
| 3 | `9601447` | 7 of 8 failed; `build_02_equal_objects_are_identical_without_normalisation` passed because Task 1 already implements byte equality | `af9ad7f` |

Semantic assessment of the RED evidence: in each case the failures were `assert_eq!` or `unwrap` failures on the behaviour under test (verdict `build_failed` with detail `not implemented` where `identical` was planned, `Err(Empty)` where entries were planned, empty attribute fields where `v7E-M` was planned). No failure was a syntax, import, fixture or zero-test fault. The `gsd check tdd-red-evidence` classifier was not run: cargo's libtest console output is not one of its supported report formats (TAP, JUnit XML, swift-testing, unittest), and no record was fabricated. The three tests that passed at RED are listed above and are covered by the other tests of their task.

## Files Created/Modified

- `crates/mt-capture/src/cc.rs` - canonical entry types, `canonicalise`, `canonical_output`, `CcPolicy::check_entry`, lexical path helpers
- `crates/mt-capture/src/replay.rs` - `ReplayPlan`, `Normaliser`, `ReplayError`, `replay()`, `compare_differing`, verdict precedence
- `crates/mt-capture/src/report.rs` - `ReplayReport`, `Verdict`, `ObjectStatus`, `ObjectComparison`, `Step`, `NORMALISATION_RULE`
- `crates/mt-capture/src/elf.rs` - section digests, header check, readelf attribute gate, `code_digest`
- `crates/mt-capture/src/treehash.rs` - `copy_tree`, `TreeHashError::EscapingSymlink`
- `crates/mt-capture/tests/build_02_replay.rs`, `build_01_compile_commands.rs`, `build_02_normalisation.rs` and `fixtures/cc/{bear,cmake}-raw.json` - the tests and recorded exporter shapes
- `crates/mt-toolchain/src/runner.rs` - `Deserialize` on four record types
- `Cargo.toml`, `Cargo.lock`, `crates/mt-capture/Cargo.toml` - shlex and object pinned with `=`, regex wired for the readelf parser

## Decisions Made

See key-decisions. Downstream plans (02-05, 02-06, 02-09) should know the binding behaviours: `replay` validates that every `CcEntry.output` equals `canonical_output(entry)`, that `policy.run_root` equals `plan.run_root`, that `run_root` is absolute, that `src_dir_name` is one directory name and that `keep_dirs` lie under the run root and outside the source directory; `original_objects_dir` mirrors the run-root-relative keys; a child that cannot be started is `Err(ReplayError::Runner)`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical] Replay validates the entry output against its arguments, the run roots and duplicate outputs**
- **Found during:** Task 1
- **Issue:** The plan checks the allow-list and containment, but replay deletes and hashes `entry.output`. A tampered `output` field that disagrees with `-o` would delete or hash a file the compile does not write (threat T-02-15).
- **Fix:** `validate()` also requires `output == canonical_output(entry)`, equal policy and plan run roots, unique outputs, a clean `src_dir_name`, safe `keep_dirs` and clean object keys, all before anything is touched.
- **Files modified:** `crates/mt-capture/src/replay.rs`
- **Committed in:** `ecdaefb`

**2. [Rule 2 - Missing critical] copy_tree resolves link chains physically after copying**
- **Found during:** Task 1
- **Issue:** The plan asks for a lexical check of each link target. A chain such as `d/up -> ..` then `e -> d/up/../..` passes the text check for each link but escapes the tree when followed.
- **Fix:** After the copy, every link that can be resolved is canonicalised and refused if it lands outside the copied tree. Dangling and looping links are left alone since they cannot be followed anywhere.
- **Files modified:** `crates/mt-capture/src/treehash.rs`
- **Committed in:** `ecdaefb`

**3. [Rule 2 - Missing critical] Four tests beyond the plan's list**
- **Found during:** Tasks 1 and 3
- **Issue:** The planned list did not cover the `missing_in_original` side, the readelf tag gate on its own platform-independent path, or the `normalisable` name rules.
- **Fix:** Added `build_02_object_without_original_is_reported_as_differs`, `build_01_architecture_gate_parses_readelf_attributes` and `build_02_normalisable_accepts_only_comment_and_debug_names`.
- **Committed in:** `b7b35a0`, `9601447`

### Behaviour notes (not defects)

- A separate commit `26167b3` adds the Deserialize derives ahead of the RED commit so the round-trip test compiles; the plan lists that change inside Task 1.
- `shlex` and `object` are pinned with `=` (`=2.0.1`, `=0.40.0`) because the plan says exactly those versions. `cargo deny check` passed after each was added and `Cargo.lock` gained only `shlex`, `object` and their edges (`memchr` was already locked).
- `parse_readelf_a` uses one regex compiled once and has no `Result`: the plan's API gives it a plain `ArmAttributes` return, so a pattern that failed to compile would read nothing and fail the gate closed.
- `canonicalise` applies the gates to all exported entries, then drops unbuilt ones. A tampered entry for an unbuilt target is therefore still rejected.
- The ELF64 host objects exercise the same code paths as Arm ELF32 objects (`sections` handles both classes), but no Arm object was read in this session.

---

**Total deviations:** 3 auto-fixed (3 missing critical)
**Impact on plan:** All three tighten the plan's own mitigations (T-02-15, T-02-16); no scope creep and no architectural change.

## Issues Encountered

- The worktree guard refused compound shell commands that contained a `git` call and a Python heredoc; each was rerun as plain separate commands and file edits went through the Edit tool.

## Authentication Gates

None.

## Known Stubs

None. The stubs used for the RED commits were replaced in the GREEN commit of the same task.

## Threat Flags

None. The new surface (deleting `run_root/<src>` and declared outputs, spawning allow-listed compilers and objcopy, parsing untrusted objects) is the one in the plan's threat model, T-02-13 to T-02-17, with mitigations covered by `build_01_cmake_command_strings_split_into_argv`, `build_01_rejects_non_allowlisted_compiler`, `build_02_replay_refuses_non_allowlisted_compiler`, `build_01_rejects_paths_outside_run_root`, `bed_01_tree_copy_preserves_symlinks_and_refuses_escapes` and `build_02_unparseable_object_is_reported_as_differs`. T-02-17 (a hung compile) is the runner's timeout; replay turns it into `not_proven`, but no test here forces a timeout, since the runner suite covers the kill path.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plans 02-05, 02-06 and 02-09 can call `canonicalise`, `replay`, `elf::check_elf_header`, `elf::parse_readelf_a` and `copy_tree` unchanged. Pass `compiler_tool_name = "arm_gcc"` and the image's `arm-none-eabi-objcopy` as the `Normaliser`.
- Not yet proven on real data: the Cortex-M4 gate and the normalisation on Arm objects built by 14.3.rel1. Plan 02-06's workflow publishes them and plan 02-10 commits them as fixtures. Research measured 31 of 31 pairs identical without normalisation, so the objcopy path may stay unused in CI.
- The runner's open residual (a `complete: false` stream) is handled as `not_proven`; replay does not retry.

## Self-Check: PASSED

- Created files exist: cc.rs, replay.rs, report.rs, elf.rs, build_02_replay.rs, build_01_compile_commands.rs, build_02_normalisation.rs, bear-raw.json, cmake-raw.json (all FOUND).
- Commits `26167b3`, `b7b35a0`, `ecdaefb`, `6696dbd`, `20436f9`, `9601447`, `af9ad7f` are ancestors of HEAD (all FOUND).
- Acceptance: `grep -c Deserialize crates/mt-toolchain/src/runner.rs` prints 5; `shlex::split` is in `cc.rs` and nothing in `mt-capture/src` uses `std::process` or `Command::new`; `repo_checks` passes (9 tests); `report` JSON of `build_02_replay_matches_original_objects` contains `normalisation_rule`.
- Verification: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace --locked` and `cargo deny check` all exited 0 before the last task commit. Test counts: `build_02_replay` 10, `build_01_compile_commands` 12, `build_02_normalisation` 8, `tool_01_runner` 17, all passing.

---
*Phase: 02-test-bed-and-build-capture*
*Completed: 2026-10-13*
