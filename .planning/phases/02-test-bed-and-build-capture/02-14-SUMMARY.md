---
phase: 02-test-bed-and-build-capture
plan: 14
subsystem: testing
tags: [determinism, capture, serde_json, json-pointer, fail-closed, wr-05, tdd]

requires:
  - phase: 02-test-bed-and-build-capture
    provides: "capture report and compile_commands layout (02-06, 02-09); the two determinism exclusions measured in 02-06"
provides:
  - "mt_capture::compare: compare_capture_dirs, DirComparison, CompareError, DETERMINISM_EXCLUSIONS"
  - "mt testbed compare --first <dir> --second <dir> (exit 0 identical, 1 differences, 2 unreadable tree)"
  - "A determinism verdict that cannot pass on unparseable, empty, one-sided, non-regular or zero files (WR-05)"
affects: [02-17 workflow switch to mt testbed compare, testbed determinism evidence]

actuals:
  tokens: 7700
  tasks: 2
  commits: 4
plan_head_before: e9e34ec02b57c1c7a3e832d13f61473d5fe19400
plan_head_after: 2ef8c6d5cc4ccaa54c78d05e7a12b3c8a17d4a1b

tech-stack:
  added: []
  patterns:
    - "Fail-closed comparison: every file the verdict depends on is read, parsed and non-empty, or it is a named problem"
    - "JSON pointer (RFC 6901) of the first difference reported per file, object keys as a sorted set, arrays by position"

key-files:
  created:
    - crates/mt-capture/src/compare.rs
    - crates/mt-capture/tests/build_02_compare.rs
    - crates/mt-cli/tests/build_02_compare_cli.rs
  modified:
    - crates/mt-capture/src/lib.rs
    - crates/mt-cli/src/main.rs

key-decisions:
  - "The comparator is a pure file reader in mt-capture (no std::process, no runner, no write); mt-cli only parses arguments and maps the verdict to exit 0/1/2"
  - "Exclusions live in one constant (DETERMINISM_EXCLUSIONS, words) applied by one function (strip_exclusions) to reports whose own kind is capture; nothing else is excluded"
  - "A non-regular entry (symlink, socket, device) is a problem on whichever side it appears and is never followed"

patterns-established:
  - "Problem lines name the side and path: cannot parse <first|second>/<path>, empty file: <side>/<path>, only in <side>: <path>, not a regular file: <side>/<path>, differs: <path> at <pointer>"

requirements-completed: [BUILD-02]

coverage:
  - id: D1
    description: "The determinism comparison fails closed: unparseable reports (one or both sides), empty files, one-sided files, non-regular entries and zero compared files are differences"
    requirement: BUILD-02
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/build_02_compare.rs#build_02_compare_fails_closed_on_unparseable_reports"
        status: pass
      - kind: unit
        ref: "crates/mt-capture/tests/build_02_compare.rs#build_02_compare_flags_empty_and_one_sided_files"
        status: pass
      - kind: unit
        ref: "crates/mt-capture/tests/build_02_compare.rs#build_02_compare_flags_non_regular_files"
        status: pass
      - kind: unit
        ref: "crates/mt-capture/tests/build_02_compare.rs#build_02_compare_with_no_files_does_not_pass"
        status: pass
    human_judgment: false
  - id: D2
    description: "Only the stdout sha256, bytes_total and bytes_stored of `cmake configure` and `clang ast <fn>` in capture reports are excluded; every other field, report kind and compile_commands file is compared"
    requirement: BUILD-02
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/build_02_compare.rs#build_02_compare_excludes_only_the_documented_step_outputs"
        status: pass
      - kind: unit
        ref: "crates/mt-capture/tests/build_02_compare.rs#build_02_compare_exclusions_apply_to_capture_reports_only"
        status: pass
      - kind: unit
        ref: "crates/mt-capture/tests/build_02_compare.rs#build_02_compare_compile_commands_byte_for_byte"
        status: pass
      - kind: unit
        ref: "crates/mt-capture/tests/build_02_compare.rs#build_02_compare_ignores_key_order_but_not_values"
        status: pass
    human_judgment: false
  - id: D3
    description: "mt testbed compare maps the verdict to exit 0 identical, 1 differences, 2 unreadable tree and prints the compared count, the exclusions and the result"
    requirement: BUILD-02
    verification:
      - kind: integration
        ref: "cargo test -p mt-cli --test build_02_compare_cli"
        status: pass
    human_judgment: false
  - id: D4
    description: "The CI workflow's determinism step uses the new command (plan 02-17); not part of this plan"
    verification: []
    human_judgment: true
    rationale: "Wiring and the in-container proof belong to plan 02-17; this plan proves the comparator offline only"

duration: 5min
completed: 2026-10-19
status: complete
---

# Phase 2 Plan 14: Fail-closed capture tree comparison (WR-05) Summary

**`mt testbed compare` compares two capture output trees in Rust, with the two measured exclusions in one constant and every unparseable, empty, one-sided, non-regular or absent file a named difference instead of the `jq | cmp -s` "identical".**

## Performance

- **Duration:** 5 min
- **Started:** 2026-10-19T21:27:26Z
- **Completed:** 2026-10-19T21:32:00Z
- **Tasks:** 2 (tracer + comparison rules), 4 commits (RED, GREEN per task)
- **Files modified:** 5 (3 created, 2 modified)

## Accomplishments

- `mt_capture::compare` with `compare_capture_dirs`, `DirComparison` (`passed()` needs at least one compared file and no problem), `CompareError` and `DETERMINISM_EXCLUSIONS`. It reads files only.
- Rules held to the jq semantics they replace: `*.compile_commands.json` byte for byte; other `*.json` as parsed JSON (key order ignored, array order compared) after removing the three stdout fields of `cmake configure` and `clang ast <fn>` from reports whose `kind` is `capture`; any other file byte for byte.
- Fail-closed paths the old step lacked: a report that cannot be parsed on either side (`cannot parse first|second/<path>`), a zero-byte file, a file on one side only, a symlink or other non-regular entry, and zero compared files.
- Each differing JSON file is reported with the RFC 6901 pointer of its first difference (for example `/steps/2/run/stdout/sha256`).
- `mt testbed compare --first --second` prints `compared <n> files`, one `excluded:` line per exclusion, `result: identical` or `result: differences found` and then each problem.

## Task Commits

1. **Task 1 (tracer): `mt testbed compare` from the CLI to a fail-closed verdict**
   - RED `d935a5d` (test): CLI tests only
   - GREEN `386d372` (feat): minimal comparator plus the subcommand
2. **Task 2: complete comparison rules**
   - RED `51c7249` (test): comparison tests only
   - GREEN `2ef8c6d` (feat): full rules in compare.rs

**Plan metadata:** recorded in the docs(02-14) commit that follows this summary.

## TDD Evidence

Tracer gate (end-of-phase mode, automated-only verify): the tracer's verify was re-run after its GREEN commit and passed (3 of 3), logged "Tracer verified end-to-end, expanding", then Task 2 started.

### RED, Task 1 (commit d935a5d, test code only, unchanged implementation)

`cargo test -p mt-cli --test build_02_compare_cli`: the built binary refuses the subcommand, so all three tests fail:

```
assertion `left == right` failed: stderr: error: unrecognized subcommand 'compare'
  tip: a similar subcommand exists: 'capture'
  left: Some(2)  right: Some(0)     (build_02_compare_cli_identical_trees_exit_zero)
  left: Some(2)  right: Some(1)     (build_02_compare_cli_fails_closed_on_unparseable_reports)
stderr: error: unrecognized subcommand 'compare'   (build_02_compare_cli_missing_tree_exits_two:
                                                    asserts the stderr names the absent tree, not the parser)
test result: FAILED. 0 passed; 3 failed
```

The missing-tree test also asserts that stderr names `absent-tree` and does not say `unrecognized subcommand`, because exit 2 alone would already have passed against the unknown subcommand.

Semantic assessment: all three failed on the planned assertions for the intended reason (the subcommand does not exist yet); no compile, load or fixture fault.

### RED, Task 2 (commit 51c7249, test code only, against the tracer's minimal comparator)

`cargo test -p mt-capture --test build_02_compare`: 6 failed on assertions, 3 passed (identical trees, unparseable reports, empty trees: the tracer already covered them).

```
excludes_only_the_documented_step_outputs  cmake configure.stdout.sha256: ["differs: corejson/corejson-default.json"]
exclusions_apply_to_capture_reports_only   replay: ["differs: corejson/corejson-default.json"]   (no JSON pointer yet)
flags_empty_and_one_sided_files            first: ["cannot parse first/corejson/corejson-default.json: EOF while parsing a value at line 1 column 0"]
compile_commands_byte_for_byte             assertion failed: !comparison.passed()
flags_non_regular_files                    assertion failed: !comparison.passed()
ignores_key_order_but_not_values           ["differs: x/r.json"]   (no JSON pointer yet)
test result: FAILED. 3 passed; 6 failed
```

Semantic assessment: each failure is the planned assertion (exclusion not applied, byte-for-byte rule missing, empty and one-sided files not flagged, no pointer reported). No fixture or syntax fault. The first draft of the test file failed clippy (needless lifetimes, type complexity), fixed before the RED commit; a first pointer into the fixture's `entries`/`objects`/`configure_files` assumed arrays, but they are maps, so the test picks the first key instead.

### GREEN and verification

- `cargo test -p mt-capture --test build_02_compare`: 9 passed
- `cargo test -p mt-cli --test build_02_compare_cli`: 3 passed
- `cargo fmt --check`: clean; `cargo clippy --all-targets -- -D warnings`: clean
- `cargo test --workspace --locked`: all suites ok, 0 failed (includes repo_checks' process-confinement test)
- `cargo deny check`: advisories ok, bans ok, licenses ok, sources ok
- `compare.rs` contains no `std::process`, `Command` or write call

### TDD Gate Compliance

For both tasks a `test(02-14)` commit precedes its `feat(02-14)` commit (`d935a5d` then `386d372`; `51c7249` then `2ef8c6d`). No REFACTOR commit was needed.

## Files Created/Modified

- `crates/mt-capture/src/compare.rs` - the comparator, exclusions constant, JSON pointer of the first difference
- `crates/mt-capture/src/lib.rs` - declares `compare` and mentions it in the crate doc
- `crates/mt-cli/src/main.rs` - `TestbedAction::Compare` and `compare_command` (arguments and exit mapping only)
- `crates/mt-capture/tests/build_02_compare.rs` - nine comparison tests on the committed CI capture report
- `crates/mt-cli/tests/build_02_compare_cli.rs` - three tests of the built binary

## Decisions Made

- The comparator reads files only and lives in mt-capture; mt-cli stays argument parsing plus verdict mapping (CONTEXT D-21).
- `strip_exclusions` decides by each side's own `kind`; a `kind` mismatch between the two sides shows up as a difference at `/kind`.
- A non-regular entry is reported on whichever side has it; if the other side lacks the path the one-sided problem is reported as well.
- The CLI prints `no file was compared` after the problems list when the count is zero, so the zero-file verdict is explained.

## Deviations from Plan

Minor test-layout choice, no behaviour change:

- The plan's `build_02_compare_flags_empty_and_one_sided_files` also lists the symlink case. The symlink case is its own test, `build_02_compare_flags_non_regular_files` (unix only), and `build_02_compare_identical_trees_pass` was added as the positive control. The plan said "seven comparison tests"; there are nine. All named tests exist.

**Total deviations:** 0 auto-fixed. **Impact on plan:** none.

## Issues Encountered

- The first RED draft of the missing-tree CLI test would have passed against the unknown subcommand (clap also exits 2); it now also asserts the refusal names the tree. Recorded above.
- Sibling plans 02-11, 02-12 and 02-13 had already committed; no transient failure from them occurred. `seeds --all --check` was not part of this plan's verification and was not run.

## Known Stubs

None.

## Threat Flags

None. The comparator adds no network, process or write surface; it reads two directory trees given on the command line and does not follow symlinks.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plan 02-17 can replace the workflow's `jq`/`cmp` block with `mt testbed compare --first "$out/capture" --second "$out/capture-rerun/capture"`; stdout carries the same `compared`, `excluded:` and `result:` lines the step already writes into its notice.
- The old step also required `rerun capture exit code` to be 0; the new command does not know about the rerun, so the workflow must keep that check.

## Self-Check: PASSED

- FOUND: crates/mt-capture/src/compare.rs, crates/mt-capture/tests/build_02_compare.rs, crates/mt-cli/tests/build_02_compare_cli.rs
- FOUND commits: d935a5d, 386d372, 51c7249, 2ef8c6d (all ancestors of HEAD)
- Acceptance: `TestbedAction::Compare` in main.rs; `DETERMINISM_EXCLUSIONS` and `compare_capture_dirs` in compare.rs

---
*Phase: 02-test-bed-and-build-capture*
*Completed: 2026-10-19*
