---
phase: 03-equivalence-contract-and-test-vector-schema
plan: 07
subsystem: testing
tags: [setup-sequences, local-objects, setup-failed, wrap, state-arrays, set-steps, closed-grammars, github-actions]

requires:
  - phase: 03-equivalence-contract-and-test-vector-schema
    provides: plan 03-04's roles and generated drivers, plan 03-05's DWARF layouts, state role and pointer regions, plan 03-06's output roles, heap walk, adapter drop log and floor of ten
provides:
  - "mt_capture::manifest: SetupStep, SetupArg, SetValue, Expect, LocalObject, parse_set_path, is_closed_type_text, is_function_name, TYPE_TEXT_GRAMMAR, MAX_SETUP_STEPS, MAX_OBJECT_COUNT, Role::Wrap, ArgRole.{count, ptr_field, len_field}, Entry.{setup, local}; manifest schema_version stays 1"
  - "mt_harness: generated setup code (calls and set steps) in adapter.rs, Adapter::setup_fragment, MAX_STATES, frame::SETUP_STATUS_BASE, Outcome::SetupFailed built by classify, probe::render_probe_counted"
  - "runtime template: the MT_SETUP placeholder, setup_status and setup_ret, status word 0x100 + k"
  - "drivable entries: httpparser http_parser_execute, qcbor QCBORDecode_Init, zcbor zcbor_new_decode_state, tinycbor cbor_value_validate; 13 targets produce vectors in the pinned image (testbed run 40905688514)"
affects: [03-08, phase 4 relabelling, phase 5 comparators and abstraction functions, phase 11 evidence bundle]

requirements-completed: [EQV-03, EQV-04]

status: complete
commits: 5
plan_head_before: 9df7b2a43a3a48883be30f66e6d86c5e27c4ce7a
plan_head_after: fdf8ad975c702ff79c855fd65a7e41cd5cedae18
actuals:
  tokens: 50360
  tasks: 3
  commits: 5

tech-stack:
  added: []
  patterns:
    - "every string of a setup step is parsed into a closed form (identifier, decimal, runtime accessor, reference to a declared object) and only the parsed form is rendered; no manifest text is pasted into C"
    - "a local object is a state: one guarded zero-filled mapping, a DWARF layout from a probe variable, recorded before and after the call; a counted state or local is an array in one mapping recorded as items"
    - "a failed setup step is a frame status word (0x100 + k) and the step's return value, classified as setup_failed, compared, never labelled"
    - "a wrap stands for the in_buf and in_len together and builds the struct by value as a compound literal over validated member names"

key-files:
  created:
    - crates/mt-harness/tests/fixtures/golden/httpparser-http_parser_execute.driver.c
    - crates/mt-harness/tests/fixtures/golden/coremqtt-set-step.driver.c
    - .planning/phases/03-equivalence-contract-and-test-vector-schema/03-07-CI-NOTICES.md
    - .planning/phases/03-equivalence-contract-and-test-vector-schema/03-07-VECTORS-NOTICES.md
  modified:
    - crates/mt-capture/src/manifest.rs
    - crates/mt-capture/tests/eqv_03_roles.rs
    - crates/mt-capture/tests/bed_01_entries.rs
    - crates/mt-capture/tests/bed_01_entry_include_prefix.rs
    - crates/mt-cli/tests/eqv_04_vectors_cli.rs
    - crates/mt-harness/src/adapter.rs
    - crates/mt-harness/src/classify.rs
    - crates/mt-harness/src/driver.rs
    - crates/mt-harness/src/frame.rs
    - crates/mt-harness/src/probe.rs
    - crates/mt-harness/src/replay.rs
    - crates/mt-harness/src/run.rs
    - crates/mt-harness/src/runtime.c
    - crates/mt-harness/src/runtime.rs
    - crates/mt-harness/tests/eqv_03_adapter.rs
    - crates/mt-harness/tests/eqv_03_adapter_floor.rs
    - crates/mt-harness/tests/eqv_03_driver_golden.rs
    - crates/mt-harness/tests/eqv_04_classify.rs
    - crates/mt-harness/tests/eqv_04_decode.rs
    - crates/mt-harness/tests/eqv_04_exec.rs
    - crates/mt-harness/tests/eqv_04_globals.rs
    - crates/mt-harness/tests/fixtures/golden/corejson-JSON_Validate.driver.c
    - crates/mt-harness/tests/fixtures/golden/lz4-LZ4_decompress_safe.driver.c
    - crates/mt-harness/tests/fixtures/golden/microcoap-coap_parse.driver.c
    - crates/mt-harness/tests/fixtures/golden/zlib-uncompress.driver.c
    - testbed/manifest.toml

key-decisions:
  - "Setup grammar as the plan states it: a call of a C identifier (not a keyword) with args $in, $len, $cap, $arg:<scalar>, a decimal integer, &<state or local> or <pointer local>, and expect void or a 32-bit decimal; a set of <object>.<member>(.<member>)* to $in, $in+n, $len, $len-n or a decimal. At most 16 steps. $len-n is 32-bit unsigned arithmetic (it wraps below zero, as the C would see it)"
  - "setup_failed: status word SETUP_STATUS_BASE (0x100) + k, rc is the step's return value, stderr is 'outcome=setup_failed step=<k>', no has_ub label, lib_state_in equals lib_state_out (the state at the stop), the entry's ret state is not recorded"
  - "a wrap stands for the in_buf and in_len together: a wrap beside either, or two wraps, is refused; argv carries the input length where the in_len would; a wrap needs accepted_wrapped = true"
  - "the input must be reached: by an in_buf, a wrap, or $in in a setup call or a set value ($in, $in+n); $len alone does not reach the bytes"
  - "a counted state or local is an array of that many elements in one guarded mapping (count present means an array, even 1), recorded as items; a set path on an array names its first element"
  - "at most MAX_STATES = 8 states per driver (parameter states, locals and the returned value), tied to MT_MAX_STATES in runtime.c by a test"
  - "the closed type grammar applies to state pointees, locals and wrap types; out_scalar, out_ptr and returns keep the plan 03-06 character-set check because their scalar names have several words (unsigned char, short unsigned int)"

patterns-established:
  - "A new input form is a new way to name the same objects (locals, counted states, wrap), not a new recorder"
  - "Notices are rebuilt from the job annotations by one script and the eight Phase 2 sections are compared with 02-22 from line 5 on"

coverage:
  - id: D1
    description: "Declared setup sequences: calls and field assignments from a closed grammar, validated by the manifest and again by the adapter, rendered before the snapshot so lib_state_in is the state after setup; httpparser http_parser_execute runs after http_parser_init with its bitfield parser recorded in and out"
    requirement: EQV-03
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/eqv_03_roles.rs#eqv_03_roles_setup_grammar_is_closed"
        status: pass
      - kind: unit
        ref: "crates/mt-capture/tests/eqv_03_roles.rs#eqv_03_roles_types_reaching_c_are_closed"
        status: pass
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_03_driver_golden.rs#eqv_03_driver_golden"
        status: pass
      - kind: other
        ref: "MT_LOCAL_CAPTURES=.mt-work/capture cargo test -p mt-harness --test eqv_04_exec -- --ignored eqv_04_setup_sequence_runs_before_the_entry (httpparser default and strict 76 vectors each)"
        status: pass
    human_judgment: false
  - id: D2
    description: "A failed setup is its own compared outcome: setup_failed with the step in stderr and the step's return value in rc, no label, never excluded"
    requirement: EQV-04
    verification:
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_04_classify.rs#eqv_04_classify_setup_failure_is_its_own_class"
        status: pass
      - kind: other
        ref: "eqv_04_setup_failure_is_a_compared_outcome (ignored, local captures: corejson JSON_Validate with a validating setup step, 101 setup_failed and 7 ok, identical twice)"
        status: pass
      - kind: other
        ref: "testbed run 40905688514: tinycbor cbor_value_validate vectors=400 ok=340 setup_failed=60 twice=identical in both configurations"
        status: pass
    human_judgment: false
  - id: D3
    description: "wrap, counted states, locals and set steps: qcbor QCBORDecode_Init (UsefulBufC by value, three modes) and zcbor zcbor_new_decode_state (three states in one mapping, two out capacities) identical twice in both configurations; a coreMQTT-shaped test-only declaration pins the set-step rendering"
    requirement: EQV-03
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/eqv_03_roles.rs#eqv_03_roles_wrap_needs_fields_and_accepted_wrapped"
        status: pass
      - kind: unit
        ref: "crates/mt-capture/tests/eqv_03_roles.rs#eqv_03_roles_state_count_is_bounded"
        status: pass
      - kind: unit
        ref: "crates/mt-capture/tests/eqv_03_roles.rs#eqv_03_roles_input_must_be_reached"
        status: pass
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_03_driver_golden.rs#eqv_03_driver_golden_set_step_is_rendered_and_closed"
        status: pass
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_03_driver_golden.rs#eqv_03_driver_states_are_bounded_and_tied_to_the_runtime"
        status: pass
      - kind: other
        ref: "eqv_04_wrapped_struct_argument (qcbor 600 vectors per configuration) and eqv_04_state_array (zcbor 274 per configuration), ignored, local captures"
        status: pass
    human_judgment: false
  - id: D4
    description: "In the pinned image 13 distinct targets produce vectors in every retained configuration, each run twice identically, and a second container reproduces the corpus byte for byte (5170 files); the four entries of this plan are proven there"
    requirement: EQV-03
    verification:
      - kind: other
        ref: "testbed run 40905688514 (8f2c365): vectors-build 30 built rows, vectors-run 30 rows twice=identical, vectors-determinism identical 5170 files; 03-07-VECTORS-NOTICES.md; ci run 40907652198 green on fdf8ad9"
        status: pass
    human_judgment: false
  - id: D5
    description: "What setup_failed and the set-step arithmetic mean for later comparison: a failed setup is compared as defined behavior of the setup call, and $len-n wraps in 32 bits"
    requirement: EQV-04
    verification: []
    human_judgment: true
    rationale: "The tests prove the executor records what the declared steps do in the contract's shapes; whether a comparator treats a setup_failed pair as agreement, and whether a Rust port must reproduce the wrapped length, is a phase 4 and 5 reading"

duration: "about 19 h 25 min of wall clock between the first RED commit (2026-10-28T20:39Z) and this summary (2026-10-29T16:10Z), which includes a container restart that killed the first executor, a usage-limit pause, and one CI round trip of about 20 min"
completed: 2026-10-29
---

# Phase 3 Plan 07: Declared setup sequences, local objects, struct arguments by value and arrays of states Summary

**Entries that need prior state are drivable through a validated setup sequence (calls and field assignments from closed grammars, a failed step recorded as its own compared `setup_failed` outcome), with locals, `wrap` struct arguments and arrays of states; httpparser, qcbor, zcbor and tinycbor `cbor_value_validate` produce identical vectors twice in the pinned image, 13 distinct targets in all, on round trip 1 of 3, and the testbed job is now at exactly 50 of 50 annotations.**

## Performance

- **Duration:** see the frontmatter. The first executor was killed by a container restart after the Tracer RED commit with an uncommitted, unverified GREEN in the working tree; this session reviewed that diff, verified it and committed it.
- **Tasks:** 3 of 3 (Tracer, Task 2, Task 3), each RED then GREEN where it carries code.
- **Files:** 28 changed outside `.planning` against `9df7b2a` (4366 insertions, 113 deletions, of which about 3600 lines are the two new and four re-blessed golden drivers), plus the two notices files and this summary.
- **Pushes and CI:** 1 round trip of the 3 allowed. Head `8f2c365`: testbed 40905688514 (job 123560448033) success, ci 40905688495 success, container 40905688290 not awaited. The docs-only notices commit `fdf8ad9` was pushed with ci 40907652198 green.

## Accomplishments

- **Setup grammar and validator (D-11, D-12, T-03-25):** `[[target.X.entry.setup]]` steps are a `call` (identifier, `args`, `expect`) or a `set` (path, `value`); `[[target.X.entry.local]]` objects have a name, a type of the closed grammar and an optional count. The manifest validator names the failing step (`setup[1]`, `local[0]`), and the adapter checks every token again where it becomes C. Nothing from the manifest is pasted into the driver.
- **setup_failed (D-12, T-03-26):** the runtime stops at the first mismatching step and still dumps every region; `classify` builds `Outcome::SetupFailed { step, rc }` from status word `0x100 + k`; the vector has `outcome=setup_failed step=<k>`, rc the step's return value, no label. A fault, abort or timeout during setup stays that fault, abort or timeout, and a changed guard-slack byte is a fault even after a failed step.
- **Forms for the hard entries (D-11, D-12, D-14):** `wrap` (a `UsefulBufC` built by value from the input), `count` on a state (an array in one guarded mapping recorded as items), locals (the iterator's parser for tinycbor), and the `set` step. A coreMQTT-shaped test-only declaration with a counted local and five set steps is pinned by `coremqtt-set-step.driver.c`.
- **Four more drivable entries:** httpparser `http_parser_execute` (76 vectors per configuration, parser with 21 bitfield members recorded in and out), qcbor `QCBORDecode_Init` (600: 200 seeds times three modes), zcbor `zcbor_new_decode_state` (274: 137 seeds times two out capacities), tinycbor `cbor_value_validate` (400 per configuration: 200 seeds times two flag values, 340 ok and 60 `setup_failed` in the image).
- **Measured in the pinned image:** `vectors-build` 30 `built` rows, `vectors-run` 30 rows `twice=identical`, `vectors-determinism` `identical 5170 files`, 13 distinct targets; `mt vectors adapters --check-floor` prints `drivable targets: 13 (floor 10)`.

## Task Commits

1. **Tracer** (`tdd`): `c4e0358` test (RED), `a4ed22d` feat (GREEN)
2. **Task 2** (`tdd`): `2c523b6` test (RED), `8f2c365` feat (GREEN, pushed; round trip 1)
3. **Task 3:** `fdf8ad9` docs (round trip 1 notices, pushed)

No REFACTOR commits were needed. `commits: 5` is the measured `git rev-list --count 9df7b2a..HEAD` at `fdf8ad9`; the summary, STATE and ROADMAP commits follow it.

## CI round trips (1 of 3 used)

| Round trip | Head | testbed | ci | Conclusion |
|------------|------|---------|----|------------|
| 1 | `8f2c365` | 40905688514 (job 123560448033) success | 40905688495 success | green; 50 annotations read, every title present; container run 40905688290 not awaited |

- `vectors-build`: 30 `built` rows, `no_roles: 21`, `result: passed (51 rows)`. `vectors-run`: every row `skipped=0 twice=identical`; httpparser 76 and 76, qcbor 600 and 600, zcbor 274 and 274, tinycbor `cbor_value_validate` 400 and 400 (`setup_failed=60` each). `vectors-determinism`: rerun exit 0, rows identical, `identical 5170 files`. `vectors-selftest` and `vectors-tractor` pass; the eight Phase 2 sections of `03-07-CI-NOTICES.md` equal `02-22-CI-NOTICES.md` from line 5 on (`diff` empty).
- The local proofs (GCC 13.2.1, QEMU 8.2.2) and the image rows (GCC 14.3, QEMU 10.0.2) agree on every row count that exists in both. tinycbor has no local capture (its local capture verdict is `build_failed`), so `cbor_value_validate` was proven only in the image; before the push its generated driver passed `arm-none-eabi-gcc -fsyntax-only -Wall -Wextra` against the real headers, and it built and ran on the first round trip.
- The three Task 3 automated verifies exit 0 on `fdf8ad9`.

## Annotation cap (standing blocker for plan 03-08)

The job has exactly **50 annotations** (12 `testbed-`, 31 fixture, 7 `vectors-` titles with `vectors-layout` and the dwarf fixture in two parts each). Before the push I counted the effect of this plan by simulating GitHub's `split -C 3800` on the layout, run and build listings: layout grows to two parts (the one annotation of headroom), run and build stay in one part. The run confirmed it. Nothing was trimmed, merged or moved. Measured part sizes: `vectors-layout (2/2)` 3737 bytes, `vectors-run` 3638, `vectors-build` 3634, each against 3800. Plan 03-08 cannot add a row to the run or build listing, nor a layout line, without a third layout part or a second run part, which would make a 51st annotation and cut `vectors-determinism`. Options for the founder (not decided here): a second job for the vectors steps, merging the 31 Phase 2 fixture notices, or one multi-part listing notice that carries layout, build and run together.

## TDD evidence (RED output, with a one-line assessment per failure)

**Tracer RED (`c4e0358`, written by the interrupted executor; its output was not re-run in this session)**

The commit adds role tests importing `SetupStep`, `SetupArg`, `SetValue`, `LocalObject`, `TYPE_TEXT_GRAMMAR`, `is_closed_type_text` and the fields `ArgRole.{count, ptr_field, len_field}`, `Entry.{setup, local}`, a classify test importing `frame::SETUP_STATUS_BASE`, a driver golden case for httpparser with no golden file, and a local proof for httpparser. At that parent none of these items exist. Assessment: right reason by construction (compile errors naming the missing items), but the quoted compiler output is not available.

**Task 2 RED (`2c523b6`)**

```
eqv_03_roles:   3 errors E0599: no variant, associated function, or constant named `Wrap` found for enum `Role`
eqv_03_driver_golden_set_step_is_rendered_and_closed: FAILED  called `Result::unwrap()` on an `Err` value: Invalid("local objects are not driven yet")
eqv_04_wrapped_struct_argument, eqv_04_state_array: FAILED  the driver must build: [BuildRow { target: "qcbor" / "zcbor", ..., state: NoRoles }]
```

Assessment: right reason for each; the role tests name the missing variant, the golden test fails on the unimplemented local, and the two local proofs fail because qcbor and zcbor have no roles yet.

GREEN: every named test passes; the four pre-merge gates exit 0 (`cargo test --workspace --locked` with no `skipping:` line), and the 18 ignored local proofs of `eqv_04_exec` pass with `MT_LOCAL_CAPTURES`.

## Tracer feedback gate

Auto mode was off and `human_verify_mode` was the default `end-of-phase`; the tracer's `<verify>` is automated-only, so it was re-run after the Tracer GREEN commit (role, classify and golden tests; the ignored proofs printing `httpparser/default/http_parser_execute vectors=76 identical` and the `strict` row, with every earlier target still identical; the four gates) and logged `Tracer verified end-to-end - expanding`. No checkpoint was synthesized.

## Deviations from Plan

**1. [Process] Resumed after a container restart.** The first executor died after the Tracer RED commit with about 23 files of uncommitted GREEN. I reviewed the whole diff against the plan, rebuilt, ran every test and ignored proof, and committed it unchanged as `a4ed22d`. The RED tests of `c4e0358` were not touched. It includes one extra local proof the first executor wrote, `eqv_04_setup_failure_is_a_compared_outcome` (a validating setup step added to corejson `JSON_Validate` in a temporary manifest copy).

**2. [Interpretation] Goldens arrive with GREEN.** As in plans 03-05 and 03-06, a golden is generated output: `httpparser-http_parser_execute.driver.c` and `coremqtt-set-step.driver.c` cannot exist before the template and the forms; the four older goldens changed with the template (setup placeholder, status word). Blessed with `MT_BLESS_GOLDEN=1`; the diff is the review.

**3. [Plan expectation] The floor tests no longer hard-code 4 dropped targets.** A drivable httpparser made the count 11, so dropping four no longer fell below 10. `eqv_03_floor_counts_only_seeded_undropped_entries` and `eqv_04_adapters_cli_checks_the_floor` now drop six and expect `count - 6`. The floor rule itself is unchanged (`ADAPTER_FLOOR` 10).

**4. [Interpretation] The closed type grammar covers state pointees, locals and wrap types, not out roles or returns.** The plan lists the out-role pointee and the struct return too, but `out_scalar` takes multi-word scalar names (`unsigned char`, `short unsigned int`) that the grammar cannot express; those keep the plan 03-06 check (letters, digits, `_`, spaces and `*`, plus `value_pointee_type`).

**5. [Interpretation] Details the plan left open.** A wrap stands for the in_buf and in_len together (a wrap beside either, or two wraps, is refused; argv carries the input length at its position); the input-must-be-reached rule also accepts a `set` value of `$in` or `$in+n`; `count` present means an array (even 1); a `set` path on an array names its first element; a set on a pointer-typed local is refused; at most 8 states per driver (`MAX_STATES`, tied by `eqv_03_driver_states_are_bounded_and_tied_to_the_runtime`); `Expect` and `SetValue` literals above `i32::MAX` get a `u` suffix and negative ones are parenthesised.

**6. [Minor] tinycbor declares a local.** The plan's Task 2 action says "no local objects" for tinycbor, but the Roles section requires `local parser` of type `CborParser` for `cbor_parser_init(..., &parser, &it)`; the Roles section was followed.

**7. [Minor] Public API beyond the artifact list:** `Expect`, `parse_set_path`, `is_function_name`, `MAX_SETUP_STEPS`, `MAX_OBJECT_COUNT`, `adapter::MAX_STATES`, `probe::render_probe_counted` (the old `render_probe` delegates to it), `LayoutRequest.states` now carries an optional count.

**8. [Process] `gsd_run` is not on the shell path here**, so the pre-commit protected-branch assertion used its five-name fallback; the branch is `claude/gsd-discuss-phase-1-mzdw5v`. The disk was at 2.2 GB at the end; `target/debug/incremental` was not yet removed.

**Total deviations:** 8 (3 interpretations, 1 plan expectation, 2 minor, 2 process). **Impact:** none on the plan's goal; one of three round trips used, the job left exactly at the annotation cap.

## Decisions Made

See `key-decisions` in the frontmatter. For the founder to read at `/gsd-verify-work`: `$len-n` wraps in 32 bits (a length shorter than `n` becomes a huge length and the C reads out of bounds, which is the C's behavior and is recorded as a fault), the choice that a setup call returning anything other than its `expect` makes the vector `setup_failed` rather than calling the entry anyway (tinycbor: 60 of 400 vectors per configuration), and the sweep value 1048320 (`CborValidateStrictMode`, 0xfff00 in `src/cbor.h`) for tinycbor's `flags`.

## Known Stubs

None.

## Threat Flags

None beyond the plan's register. Mitigations exercised: T-03-25 by `eqv_03_roles_setup_grammar_is_closed`, `eqv_03_roles_types_reaching_c_are_closed` and `eqv_03_driver_golden_set_step_is_rendered_and_closed` (`$in*2`, `$len-x`, `$in+`, `1+1`, `$cap` refused as set values); T-03-26 by `eqv_04_classify_setup_failure_is_its_own_class` and the image rows (`setup_failed=60`, compared, no label); T-03-27 by `eqv_03_roles_state_count_is_bounded` (0, 65 and 1000 refused), `eqv_03_driver_states_are_bounded_and_tied_to_the_runtime` and one guarded mapping per object; T-03-SC: no external crate added (`Cargo.lock` gained only path crates against `949214e`; `cargo deny check` passes).

## Issues Encountered

- **Open founder questions untouched:** lz4's overshoot against D-07, and function-local statics named `name.N` in `globals` (none of the four new entries changes one).
- **tinycbor locally unbuilt:** its local capture verdict is `build_failed`, so its only proof is the image run (first try, green).
- **Annotation cap:** see the section above; plan 03-08 starts at 50 of 50.
- The ignored local proofs ran on this host with QEMU 8.2.2, GCC 13.2.1 and llvm-dwarfdump 18.1.3; in CI on QEMU 10.0.2, GCC 14.3 and llvm-dwarfdump 16.0.6 with the same row counts.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- 13 targets are drivable and reproduced, three above the floor; no floor restoration was needed.
- Plan 03-08 can use call and set steps, locals, wrap and counted states for the hardest entries, but must resolve the annotation cap before its first push (it cannot add listing rows).
- Not pushed: this summary, STATE.md and ROADMAP.md commits follow the pushed notices commit and ride with the next plan's first push.

## Self-Check: PASSED

- Created files found on disk: both new golden drivers, `03-07-CI-NOTICES.md`, `03-07-VECTORS-NOTICES.md`.
- Commits found as ancestors of HEAD: c4e0358, a4ed22d, 2c523b6, 8f2c365, fdf8ad9 (HEAD equals origin at the time of writing).
- Plan-level verification: pre-merge gates exit 0 on the pushed code head; `Cargo.lock` gained no external package against `949214e`; no forbidden claim word in `crates/mt-harness/src`, `crates/mt-capture/src/manifest.rs` or the vectors notices; `mt vectors adapters --check-floor` prints `drivable targets: 13 (floor 10)` and exits 0; testbed run 40905688514 and ci runs 40905688495 and 40907652198 are green for their heads; the eight Phase 2 sections equal 02-22's from line 5 on; 13 distinct targets have `twice=identical` rows.
