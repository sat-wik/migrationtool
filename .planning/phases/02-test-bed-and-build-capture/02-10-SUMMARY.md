---
phase: 02-test-bed-and-build-capture
plan: 10
subsystem: testing
tags: [fixtures, ci-captures, compile-commands, readelf, elf, clang-ast, offline-tests]

requires:
  - phase: 02-test-bed-and-build-capture
    provides: "plan 02-06 green testbed run 38380382970 (job 115239376216) and its fixture notices; plans 02-04 and 02-05 parsers, report types and entry check"
provides:
  - "crates/mt-capture/tests/fixtures/ci/: eight fixtures copied verbatim from the pinned image's CI run, plus SOURCE.md provenance"
  - "tests/build_01_ci_fixtures.rs: six offline tests that parse the fixtures with cc, report, elf and entries"
affects: [02-08, 02-09, ci workflow, any later replacement of the fixtures from a newer testbed run]

requirements-completed: [BUILD-01, BUILD-02, BED-01]

actuals:
  tokens: 25000
  tasks: 1
  commits: 1
plan_head_before: 2423826fab4ba1435109e7e34bcc92d022f98c0c
plan_head_after: a59570246ec3ab9994536f1d5377d072adfcccc1

tech-stack:
  added: []
  patterns:
    - "CI-captured fixtures sit next to their provenance (SOURCE.md) and are replaced only by copying the notices of a newer run"
    - "A notice drops the final newline of its file, so round-trip tests compare the serialised form with the fixture plus one newline"

key-files:
  created:
    - crates/mt-capture/tests/fixtures/ci/SOURCE.md
    - crates/mt-capture/tests/fixtures/ci/qcbor-default.compile_commands.json
    - crates/mt-capture/tests/fixtures/ci/corejson-default.compile_commands.json
    - crates/mt-capture/tests/fixtures/ci/corejson-default.capture.json
    - crates/mt-capture/tests/fixtures/ci/corejson-default.replay.json
    - crates/mt-capture/tests/fixtures/ci/arm-object.readelf-A.txt
    - crates/mt-capture/tests/fixtures/ci/arm-small.o
    - crates/mt-capture/tests/fixtures/ci/arm-small-comment.o
    - crates/mt-capture/tests/fixtures/ci/qcbor-entry.ast.json
    - crates/mt-capture/tests/build_01_ci_fixtures.rs
  modified: []

key-decisions:
  - "Round-trip tests compare to_json output with the fixture plus one newline, because the notices drop each file's final newline and the fixtures are not hand-edited"
  - "A sixth test (build_01_ci_source_md_names_every_fixture) checks that SOURCE.md names the run id and every fixture file, so a fixture added without provenance fails"

patterns-established:
  - "Fixture checks tie a CI capture to committed data: the replay's original object hashes equal the capture's, and the AST signature equals the entry in testbed/manifest.toml"

coverage:
  - id: D1
    description: "Real compile_commands of qcbor/default and corejson/default from the pinned image parse as canonical entries, re-serialise to the same bytes, are sorted by (file, output), and name only the pinned compiler as argv[0]"
    requirement: BUILD-01
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/build_01_ci_fixtures.rs#build_01_ci_compile_commands_are_canonical"
        status: pass
    human_judgment: false
  - id: D2
    description: "The real corejson/default capture and replay reports deserialise into CaptureReport and ReplayReport with their run records and re-serialise to the same bytes; the replay verdict is identical and its objects are the captured ones"
    requirement: BUILD-02
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/build_01_ci_fixtures.rs#build_02_ci_reports_round_trip"
        status: pass
    human_judgment: false
  - id: D3
    description: "Real readelf -A text passes the Cortex-M4 hard-float architecture gate"
    requirement: BUILD-01
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/build_01_ci_fixtures.rs#build_01_ci_readelf_passes_the_arch_gate"
        status: pass
    human_judgment: false
  - id: D4
    description: "Two real Arm objects pass the ELF header check, differ only in .comment, that difference is normalisable, and their code digests are equal"
    requirement: BUILD-02
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/build_01_ci_fixtures.rs#build_02_ci_arm_objects_differ_only_in_comment"
        status: pass
      - kind: other
        ref: "cmp -s arm-small.o arm-small-comment.o exits 1"
        status: pass
    human_judgment: false
  - id: D5
    description: "The real clang 16 AST dump yields QCBORDecode_Init with the signature and header recorded in testbed/manifest.toml"
    requirement: BED-01
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/build_01_ci_fixtures.rs#bed_01_ci_ast_dump_contains_the_entry_signature"
        status: pass
    human_judgment: false
  - id: D6
    description: "SOURCE.md records the run id, head SHA, image digest and notice title of every fixture, and the rule that fixtures are replaced only from a newer run"
    requirement: BED-01
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/build_01_ci_fixtures.rs#build_01_ci_source_md_names_every_fixture"
        status: pass
    human_judgment: false

duration: 4min
completed: 2026-10-13
status: complete
---

# Phase 2 Plan 10: CI-captured fixtures and the offline tests that read them Summary

**Eight fixtures copied verbatim from the pinned image's green testbed run 38380382970 (compile_commands, a capture and a replay report, `readelf -A`, two Arm objects that differ only in `.comment`, and the clang 16 AST dump of `QCBORDecode_Init`), with SOURCE.md provenance and six offline tests that run them through the real parsers**

## Performance

- **Duration:** about 4 min
- **Started:** 2026-10-13T10:07:56Z
- **Completed:** 2026-10-13T10:12Z
- **Tasks:** 1
- **Files modified:** 10 created, none modified

## Accomplishments

- The notices of job 115239376216 (run 38380382970, head 4195a6dd54ea17e9c4c0bba1ee4017e52725ddcb, image digest `sha256:e83006c0...51b9`) were read with `gh api --paginate` (35 annotations) and written to `crates/mt-capture/tests/fixtures/ci/`. The five-part capture report and the eighteen-part AST dump were joined in part order with a newline. The two objects were decoded from base64, and the `sha256` and `bytes` lines of each notice matched the decoded bytes exactly (792 and 740 bytes).
- `SOURCE.md` names the run, job, head SHA, image digest and image id, the notice title of each file, how multi-part notices were joined, and the rule that the files are replaced only by copying a newer run's notices, never edited.
- All six tests passed on the first run with no fixture and no source file changed, so the real image output agrees with the parsers: the `QCBORDecode_Init` signature from clang 16 equals the manifest's, the two objects differ in exactly `.comment` with equal code digests, and the readelf text satisfies the four gate tags.

## Task Commits

1. **Task 1: Commit the CI-captured fixtures and the offline tests that read them** - `a595702` (test)

**Plan metadata:** the commit that adds this SUMMARY (docs). No STATE.md or ROADMAP.md change (the orchestrator owns them).

## Files Created/Modified

- `crates/mt-capture/tests/fixtures/ci/SOURCE.md` - provenance and replacement rule
- `crates/mt-capture/tests/fixtures/ci/qcbor-default.compile_commands.json`, `corejson-default.compile_commands.json` - canonical compile commands from the image
- `crates/mt-capture/tests/fixtures/ci/corejson-default.capture.json`, `corejson-default.replay.json` - real reports with run records
- `crates/mt-capture/tests/fixtures/ci/arm-object.readelf-A.txt` - `readelf -A` of the corejson object
- `crates/mt-capture/tests/fixtures/ci/arm-small.o`, `arm-small-comment.o` - the `.comment`-only pair
- `crates/mt-capture/tests/fixtures/ci/qcbor-entry.ast.json` - clang 16 AST dump of the `QCBORDecode_Init` probe
- `crates/mt-capture/tests/build_01_ci_fixtures.rs` - the six offline tests

## Decisions Made

- The text fixtures lack a final newline because each notice drops it. The plan asks for byte-identical re-serialisation; the tests therefore compare `to_json` / `CaptureReport::to_json` / `ReplayReport::to_json` output with the fixture plus one newline, rather than editing the fixtures. This is stated in the test file's doc comment and in SOURCE.md.
- `arm-object.readelf-A.txt` keeps its first line, `object: build/CMakeFiles/core_json.dir/source/core_json.c.obj`, which is the workflow's own label. `parse_readelf_a` reads only `Tag_...:` lines, so the label is harmless, and it was not removed because the fixtures are copied verbatim.

## Deviations from Plan

### Additions

**1. [Rule 2 - Missing critical] A sixth test checks SOURCE.md against the directory**
- **Found during:** Task 1
- **Issue:** Threat T-02-29 is mitigated by SOURCE.md recording the run; nothing would fail if a fixture were added or renamed without provenance.
- **Fix:** `build_01_ci_source_md_names_every_fixture` requires SOURCE.md to contain the run id and the name of each of the eight fixtures.
- **Files modified:** crates/mt-capture/tests/build_01_ci_fixtures.rs
- **Committed in:** a595702

**2. Extra assertions beyond the plan's list.** The report test also checks that the capture verdict is `captured`, the replay verdict is `identical` with no normalisation, and that the replay's `original` hash of each object equals the captured object's `sha256`. The AST test checks that the declaring file ends with the manifest's header path. The compile_commands test also rejects a repeated output. These strengthen the tests and needed no source change.

---

**Total deviations:** 1 auto-fixed (Rule 2), 1 note. **Impact:** none on scope; no code outside the plan's files changed.

## Issues Encountered

- The harness refused some compound shell commands in the worktree (a heredoc into Python, `$VAR` arguments to Python, `git` chained with other commands). The extraction script was written to the scratchpad as a file and run with literal paths, and git calls were issued as plain separate commands. No effect on results.

## Authentication Gates

None. `gh api` read the annotations without error.

## Known Stubs

None.

## Threat Flags

None. The fixtures are test data only; T-02-29 is mitigated by SOURCE.md (checked by a test), byte-for-byte round trips, and the pinned-compiler check on every fixture entry.

## Push status and CI

No `git push` was run (CI budget 0). The commit `a595702` reaches origin with the next push of plan 02-08 or 02-09, and the ci workflow then runs these tests; plan 02-09's final green run is the phase gate that covers them. Local verification before the commit, each with its exit status checked directly: `cargo fmt --check` 0, `cargo clippy --all-targets -- -D warnings` 0, `cargo test --workspace --locked` 0 (every test result ok), `cargo deny check` 0 (advisories, bans, licenses, sources ok), `cmp -s arm-small.o arm-small-comment.o` exits 1.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plan 02-09's final run covers these fixtures. When a newer testbed run is the reference, replace the notices as SOURCE.md describes and update its run id, head SHA and image digest in the same commit.
- Not covered by these fixtures: the other 14 targets, and the `.comment` normalisation on a real replay (the replays in run 38380382970 were all byte-identical; the pair here is a constructed one made with `objcopy --update-section`).

---
*Phase: 02-test-bed-and-build-capture*
*Completed: 2026-10-13*

## Self-Check: PASSED

- Files: all eight fixtures, `SOURCE.md` and `tests/build_01_ci_fixtures.rs` exist under `crates/mt-capture/tests/`.
- Commit: `a595702` is an ancestor of HEAD; `git rev-list --count 2423826..HEAD` is 1 before this summary's commit.
- Acceptance: all fixture tests pass (6 of 6); SOURCE.md names run 38380382970, equal to the run in 02-06-SUMMARY.md, and lists every fixture with its notice title; `cmp -s` of the two objects exits 1; no `git push` was run.
