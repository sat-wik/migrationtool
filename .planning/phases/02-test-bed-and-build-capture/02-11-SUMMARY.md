---
phase: 02-test-bed-and-build-capture
plan: 11
subsystem: testing
tags: [rust, clang, entry-check, include-prefix, testbed, isotp, gap-closure]

requires:
  - phase: 02-test-bed-and-build-capture
    provides: "02-05 entry check (check_entry, probe_source, EntryResult), 02-09 drop log and the run A failure of isotp"
provides:
  - "include_prefix / IncludePrefix / unit_include_prefix: a linear scan of the defining TU for the include, define and undef lines before the header's include"
  - "probe_source_with_prefix and EntryResult.probe_prefix (skipped when empty)"
  - "isotp reopened: out of dropped.toml and DROPPED.md, seed corpus committed again (19 hand-made proposed seeds), README at 16 targets / 30 pairs"
affects: [02-17 testbed run (image proof of G-02-1), 02-12, 02-14]

actuals:
  tokens: 11266
  tasks: 2
  commits: 3
plan_head_before: 79145cd7d592d5cee5fffbf3afc5d2c193668d0d
plan_head_after: 23af072d49f89ca2508c3655029a81e54a988dfc

tech-stack:
  added: []
  patterns:
    - "Probe context is reproduced exactly or not at all: an unreproducible prefix falls back to the header alone and the failure detail says why"
    - "Report fields added with serde(default, skip_serializing_if) so existing reports and committed CI fixtures serialise and parse unchanged (D-22)"

key-files:
  created:
    - crates/mt-capture/tests/bed_01_entry_include_prefix.rs
    - testbed/seeds/isotp/ (index.json, NOTICE, 19 seed files)
  modified:
    - crates/mt-capture/src/entries.rs
    - testbed/dropped.toml
    - testbed/DROPPED.md
    - testbed/README.md

key-decisions:
  - "Added unit_include_prefix (reads the TU, 8 MiB cap, regular file only) as a public wrapper so the cap is unit-testable; include_prefix itself takes the text as the plan specified"
  - "The scan stops at the first include that resolves to the entry header; anything after it is never read"
  - "probe_source_with_prefix validates every prefix line (single #include/#define/#undef line, no control characters) so a prefix can never smuggle a second line into the probe"

patterns-established:
  - "RED commit is test code only; pure-function tests are RED by compile error naming the missing symbols, the capture-level tracer is RED by assertion in a scratch copy"

requirements-completed: [BED-01, BUILD-02]

coverage:
  - id: D1
    description: "The entry check's clang probe reproduces the defining TU's include prefix and then includes the header, so a header that relies on its TU's earlier includes passes through capture (G-02-1)"
    requirement: "BED-01"
    verification:
      - kind: integration
        ref: "cargo test -p mt-capture --test bed_01_entry_include_prefix#bed_01_entry_probe_uses_the_defining_tu_include_prefix"
        status: pass
      - kind: unit
        ref: "cargo test -p mt-capture --test bed_01_entry_include_prefix#bed_01_include_prefix_reproduces_includes_and_defines"
        status: pass
    human_judgment: false
  - id: D2
    description: "A prefix that cannot be reproduced exactly (conditional, other directive, code, unusable path, oversize or missing unit) is refused and the probe falls back to the header alone; a self-contained header still passes"
    requirement: "BED-01"
    verification:
      - kind: unit
        ref: "cargo test -p mt-capture --test bed_01_entry_include_prefix#bed_01_include_prefix_refuses_what_it_cannot_reproduce"
        status: pass
      - kind: integration
        ref: "cargo test -p mt-capture --test bed_01_entry_include_prefix#bed_01_entry_self_contained_header_still_passes"
        status: pass
      - kind: unit
        ref: "cargo test -p mt-capture --test bed_01_entry_include_prefix#bed_01_include_prefix_is_empty_when_the_header_comes_first"
        status: pass
    human_judgment: false
  - id: D3
    description: "The comparison is not relaxed: a function the header does not declare and a wrong signature still fail with entry_mismatch when a prefix was reproduced"
    requirement: "BED-01"
    verification:
      - kind: integration
        ref: "cargo test -p mt-capture --test bed_01_entry_include_prefix#bed_01_entry_missing_function_still_fails_with_a_prefix"
        status: pass
    human_judgment: false
  - id: D4
    description: "The report records the probe prefix only when used; header-alone results serialise as before and old reports parse"
    requirement: "BED-01"
    verification:
      - kind: unit
        ref: "cargo test -p mt-capture --test bed_01_entry_include_prefix#bed_01_entry_probe_prefix_is_recorded_only_when_used"
        status: pass
      - kind: integration
        ref: "cargo test -p mt-capture --test build_01_ci_fixtures"
        status: pass
    human_judgment: false
  - id: D5
    description: "isotp is retained: drop entry and DROPPED.md section removed, seed corpus committed (19 seeds, identical to the corpus removed at the drop), paho/server drop kept, README lists 16 targets and 30 pairs"
    requirement: "BUILD-02"
    verification:
      - kind: integration
        ref: "cargo test -p mt-capture --test build_02_drop_log --test build_03_committed_seeds --test build_02_workflow_static"
        status: pass
      - kind: other
        ref: "mt testbed seeds --manifest testbed/manifest.toml --pins container/pins.toml --all --check (exit 0, isotp seeds 19)"
        status: pass
    human_judgment: false
  - id: D6
    description: "isotp is captured, its entries found and its replay identical in the pinned image"
    requirement: "BUILD-02"
    verification: []
    human_judgment: true
    rationale: "The image proof needs the pinned container and the testbed workflow; it is the success criterion of plan 02-17, not of this offline plan"

duration: 13min
completed: 2026-10-19
status: complete
---

# Phase 2 Plan 11: Entry probe include prefix and isotp reopened Summary

**Entry-check probe now reproduces the defining TU's #include/#define/#undef prefix (exact or header-alone fallback, comparison untouched), and isotp is back in the bed with its 19-seed corpus; the image proof is plan 02-17.**

## Performance

- **Duration:** 13 min
- **Started:** 2026-10-19T20:59:12Z
- **Completed:** 2026-10-19T21:12:00Z
- **Tasks:** 2 (tracer with a RED/GREEN pair, and the isotp reopening)
- **Files modified:** 26 (3 source/test, 3 testbed text files, 20 isotp seed files)

## Accomplishments

- `include_prefix` scans the defining translation unit linearly (no recursion, no regex): joins backslash newlines, strips comments outside literals, and reproduces `#include`, `#define` and `#undef` lines up to the header's include. Quoted includes resolve in compiler order (TU directory, `-iquote`, `-I`); a name found nowhere (isotp's `"assert.h"`) is written as `#include <name>`. Conditionals, any other directive, `#include_next`, computed includes, code before the header, unwritable paths and units over 8 MiB give `NotReproducible` naming the 1-based line, and the probe is then the header alone.
- `check_entry` builds the probe with the prefix, records the lines in `EntryResult.probe_prefix` (absent from the JSON when empty), and appends "the defining TU's include prefix could not be reproduced: <reason>" to a failing entry's detail when it fell back. The name, signature and `-MM` dependency checks are unchanged.
- isotp reopened on founder decision G-02-1: drop entry and DROPPED.md section removed (a "Reopened" paragraph states why and that the proof is the next testbed run), corpus regenerated with a freshly built `mt` and byte-identical to the corpus removed by the drop, README at 16 retained targets and 30 pairs, paho/server drop kept byte for byte.
- A manual check against the real cached isotp tree: the probe the scanner would build for `isotp.c` (`<stdint.h>`, `<assert.h>`, then `isotp.h`) compiles under the host clang with the Arm sysroot.

## Task Commits

1. **Tracer RED: failing tests for the TU include prefix** - `790ff91` (test)
2. **Tracer GREEN: probe the entry header with its defining TU's include prefix** - `8d92dbb` (feat)
3. **Task 2: reopen isotp in the drop log, DROPPED.md, seeds and README** - `23af072` (feat)

**Plan metadata:** committed with this SUMMARY (docs: complete plan)

## TDD Evidence

**RED (790ff91, test code only, run against the unchanged implementation).**
Pure-function tests: the evidence is the compile error naming the missing symbols.

```
error[E0432]: unresolved imports `mt_capture::entries::IncludePrefix`, `mt_capture::entries::include_prefix`,
`mt_capture::entries::probe_source_with_prefix`, `mt_capture::entries::unit_include_prefix`
error[E0609]: no field `probe_prefix` on type `&EntryResult`   (x4)
error[E0560]: struct `EntryResult` has no field named `probe_prefix`
error: could not compile `mt-capture` (test "bed_01_entry_include_prefix") due to 6 previous errors
```

Capture-level tracer, run alone from a scratch copy (deleted, never committed) so it compiled against the old code:

```
assertion `left == right` failed: ["entry_mismatch"] ["entry_mismatch: needs_parse: clang ast dump exited with Some(1):
 .../src/lib/needs.h:2:23: error: unknown type name 'uint8_t'  2 | int needs_parse(const uint8_t *buf, size_t len);"]
  left: CaptureFailed
 right: Captured
test bed_01_entry_probe_uses_the_defining_tu_include_prefix ... FAILED
```

Semantic assessment: the target test executed, captured a real Make + Bear build, and failed on the planned assertion (verdict Captured) for the intended reason, the header-alone probe not knowing `uint8_t`. It was not a setup, fixture or load fault. `gsd check tdd-red-evidence` supports TAP, JUnit, swift-testing and unittest reports and has no adapter for `cargo test` output, so the classifier was not run; per the plan's founder constraint the compile error and the assertion output above are the recorded RED evidence, and no check was weakened or removed to produce them.

**GREEN (8d92dbb):** all 7 tests pass with `-- --nocapture` and no "skipping" line; existing `bed_01_entries` (8), `build_01_capture_drivers` (16) and `build_01_ci_fixtures` (6) pass without edits. **REFACTOR:** none needed (`cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` clean on the first GREEN).

## TDD Gate Compliance

RED (`test(02-11)`) precedes GREEN (`feat(02-11)`). No gate violation.

## Files Created/Modified

- `crates/mt-capture/src/entries.rs` - `IncludePrefix`, `include_prefix`, `unit_include_prefix`, `probe_source_with_prefix`, the `LogicalLines` scanner, `EntryResult.probe_prefix`, `check_entry` wiring, module doc step 1
- `crates/mt-capture/tests/bed_01_entry_include_prefix.rs` - seven tests (scanner, refusal table, header-first, tracer, self-contained, still-fails, report serialisation)
- `testbed/dropped.toml` - isotp table removed; paho/server and `[policy]` unchanged
- `testbed/DROPPED.md` - isotp section removed, "Reopened" paragraph before `# Entries`
- `testbed/README.md` - "Current bed" at 16 targets and 30 pairs, isotp row, dropped list reduced to paho/server
- `testbed/seeds/isotp/` - index.json, NOTICE and 19 seed files

## Decisions Made

- `unit_include_prefix` is a small addition to the planned API: it holds the 8 MiB read cap so that cap is covered by a test (sparse file); `include_prefix` still takes the text.
- Directives are matched on the identifier after `#`, so `#include_next` is its own refused directive, and `#define`/`#undef` without a name are refused.
- `probe_source_with_prefix` validates its input lines instead of trusting the scanner, so the probe file can never gain a second line from a prefix.

## Deviations from Plan

None - plan executed exactly as written. The only addition beyond the plan text is the public `unit_include_prefix` wrapper noted under Decisions, which serves the plan's own 8 MiB rule.

**Total deviations:** 0.

## Issues Encountered

None. Residual limitation to carry into review, not a defect of this plan: with a prefix, an entry name that is also declared by an earlier-included prefix header (not the entry header) is found in the AST dump. The TU does see that declaration, and the header was already allowed to satisfy the check through its own nested includes, but the check does not prove the entry header itself declares the name. Tightening it would change the comparison, which this plan must not do.

## Known Stubs

None.

## Threat Flags

None. The only new surface is the read of the defining TU, capped at 8 MiB and limited to regular files (T-02-42), and quoted names are only ever written as resolved absolute paths that pass the quote/backslash/control-character refusal.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plan 02-17's full-bed testbed run must show isotp captured, `isotp_on_can_message` found, and replay identical in the pinned image; if it fails, isotp is dropped again with the cause and run id (T-02-43).
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check` and `mt testbed seeds --all --check` (16 rows, isotp 19) all pass on the host after this plan.

---
*Phase: 02-test-bed-and-build-capture*
*Completed: 2026-10-19*

## Self-Check: PASSED

- Created and modified files exist on disk (entries.rs, bed_01_entry_include_prefix.rs, dropped.toml, DROPPED.md, README.md, seeds/isotp/index.json).
- Commits 790ff91, 8d92dbb and 23af072 are ancestors of HEAD; `git rev-list --count 79145cd..HEAD` was 3 before this SUMMARY commit.
- `entries.rs` contains `pub fn include_prefix` and `probe_prefix`; `testbed/dropped.toml` still holds the paho/server entry.
