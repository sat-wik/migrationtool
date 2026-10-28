---
phase: 03-equivalence-contract-and-test-vector-schema
plan: 05
subsystem: testing
tags: [library-state, dwarf, struct-layout, pointer-classification, unobserved-marks, typed-globals, unions, bitfields, github-actions]

requires:
  - phase: 03-equivalence-contract-and-test-vector-schema
    provides: plan 03-01's executor core and the GCC 14.3 DWARF fixture, plan 03-03's StateValue and LibState shapes, plan 03-04's manifest roles, generated drivers, dump plan, build.json and the reference corpus writer
  - phase: 02-test-bed-and-build-capture
    provides: capture.json, canonical compile commands, the replayed objects, the committed seeds
provides:
  - "mt_capture::manifest: Role::State, state_pointee_type, UnobservedMark, Entry.unobserved, parse_mark_path and the mark rules; manifest schema_version stays 1"
  - "mt_harness::layout: CType, Kind, Member, BitField, LayoutError, parse_dwarfdump, LayoutModel::type_of_variable and type_of_global, layout_summary"
  - "mt_harness::probe: render_probe, probe_flags, layout_for_entry, layout_for_globals, recompile_arguments, GlobalType"
  - "mt_harness::decode: Regions, classify_pointer, decode_value (ints, enums, floats, arrays, structs, unions, bitfields), apply_marks, check_marks, UNRESOLVED_POINTER_REASON"
  - "mt_harness::globals: function_symbols, record_globals; build.json gains layouts and global_types"
  - "runtime.c: each state in its own guarded zero-filled mapping, an address region and state regions in the result frame"
  - "mt vectors layouts, the step Publish the state layouts and the notice vectors-layout"
  - "reference vectors with lib_state_in and lib_state_out for microcoap coap_parse, nimble ble_uuid_init_from_buf and tinycbor cbor_parser_init, both configurations each, run twice and rerun in a second container, byte-identical (testbed run 40550615693)"
affects: [03-06, 03-07, 03-08, phase 4 relabelling, phase 5 comparators and abstraction functions, phase 11 evidence bundle]

requirements-completed: [EQV-03, EQV-04]

status: complete
commits: 7
plan_head_before: 0e7ccd01bb2e8f259b19baab326b7b68a5124f97
plan_head_after: 7878588a49fc916c21173f436e2a4505fda4e514
actuals:
  tokens: 44468
  tasks: 3
  commits: 7

tech-stack:
  added: []
  patterns:
    - "a layout is read, never computed: llvm-dwarfdump text of a probe built by the pinned GCC with the pair's own flags plus -g -gdwarf-5 -O0 is parsed into an owned CType tree; an unknown DWARF tag reached through a recorded type is Unsupported, a by-value cycle is Cycle, a truncated dump is Truncated"
    - "a pointer in recorded state is null, {buffer, offset} into a named region (in, out, a state, heap, global:<key>), {symbol} for a function of the linked driver, or an unobserved mark with the fixed reason; the region addresses come from the driver's own result frame, so none is assumed"
    - "a changed writable global is recorded typed under globals in both lib_state_in and lib_state_out; a changed one without a type stays executor_error untyped_global_changed"
    - "the size the driver reports for a state (sizeof in the C it was compiled with) must equal the size in the DWARF layout, or the run is an executor error"

key-files:
  created:
    - crates/mt-harness/src/layout.rs
    - crates/mt-harness/src/probe.rs
    - crates/mt-harness/src/decode.rs
    - crates/mt-harness/tests/eqv_04_layout.rs
    - crates/mt-harness/tests/eqv_04_decode.rs
    - crates/mt-harness/tests/fixtures/golden/microcoap-coap_parse.driver.c
    - .planning/phases/03-equivalence-contract-and-test-vector-schema/03-05-CI-NOTICES.md
    - .planning/phases/03-equivalence-contract-and-test-vector-schema/03-05-VECTORS-NOTICES.md
  modified:
    - crates/mt-capture/src/manifest.rs
    - crates/mt-capture/tests/eqv_03_roles.rs
    - crates/mt-capture/tests/bed_01_entries.rs
    - crates/mt-capture/tests/bed_01_entry_include_prefix.rs
    - testbed/manifest.toml
    - crates/mt-harness/src/adapter.rs
    - crates/mt-harness/src/driver.rs
    - crates/mt-harness/src/globals.rs
    - crates/mt-harness/src/lib.rs
    - crates/mt-harness/src/link.rs
    - crates/mt-harness/src/replay.rs
    - crates/mt-harness/src/run.rs
    - crates/mt-harness/src/runtime.c
    - crates/mt-harness/src/runtime.rs
    - crates/mt-harness/tests/eqv_03_driver_golden.rs
    - crates/mt-harness/tests/eqv_04_exec.rs
    - crates/mt-harness/tests/eqv_04_globals.rs
    - crates/mt-harness/tests/eqv_04_workflow_static.rs
    - crates/mt-harness/tests/fixtures/golden/corejson-JSON_Validate.driver.c
    - crates/mt-harness/tests/fixtures/golden/lz4-LZ4_decompress_safe.driver.c
    - crates/mt-cli/src/main.rs
    - crates/mt-cli/tests/eqv_04_vectors_cli.rs
    - .github/workflows/testbed.yml
    - .planning/phases/03-equivalence-contract-and-test-vector-schema/03-05-PLAN.md

key-decisions:
  - "Role::State is a named pointer parameter whose pointee, the parameter text with its one top-level * and any const removed, matches ^(struct |union |enum )?[A-Za-z_][A-Za-z0-9_]*$ and is not void; the validator, the adapter and the probe all use state_pointee_type, so that text is the only manifest-derived type that reaches C. A state name shares the namespace of the out_cap and scalar names (reserved and duplicate names are refused) and a state takes no default, sweep or max"
  - "[[target.X.entry.unobserved]] takes field and reason (deny_unknown_fields). field is <state name>(.<member>|[<index>])+ in C identifiers and decimal indices without sign or leading zero; the reason is not blank; the state must be declared by a state role of the entry; a field is marked once. The members are checked against the layout when the driver is built (an unknown member or index, or one element of a byte array, is a refused build row naming the mark)"
  - "A layout is a self-contained CType tree (text, size, kind, members with offset and optional bit position), so decode_value takes no model; typedefs resolve to their target and keep their own name as the type text, struct and union texts are struct X and union X (<anonymous> without a name), enums are enum X with the signedness of their underlying type, arrays are T[N] (T[N][M] for several dimensions), pointers are T * with only the pointee text kept. A flexible array member has count 0"
  - "Anonymous members are named _anon<byte offset>; a second one at the same offset is _anon<offset>_<n>, so names stay unique and stable"
  - "probe_flags is the manifest baseline, then the driver's flags, then -std=gnu11 (the driver compile's own), then -g -gdwarf-5 -O0 -c; the layout therefore comes from the flags the driver is built with, and the size the driver reports for each state is compared with the DWARF size on every run"
  - "Each state lives at 0x06000000 + i * 1 MiB in its own guarded zero-filled mapping (at most 8 states of at most 64 KiB), right-aligned like the buffers, with its slack checked after the call; a changed slack byte is the existing oob-write:guard-slack fault. The frame gains region 2 (addresses of the input and output buffers, the heap arena in use, each state) and regions 0x3000+i and 0x4000+i (state i before and after); the closed placeholder set grows from 8 to 9 (MT_STATES) and the three golden drivers are re-blessed"
  - "Pointer classification order: null, then a named region (strictly inside before one past the end), then a function of the linked driver (Thumb bit cleared), then an unobserved mark with the reason pointer outside known regions. Function names come from the driver ELF's defined Text symbols whose name is a C identifier; two names at one address keep the smaller. The heap region is the arena in use"
  - "A changed writable global is typed from the DWARF of its defining unit, found as the canonical compile command whose output is the global's object, recompiled with that command's own arguments minus -o, -M*, -O* and -g*, plus -g -gdwarf-5 -O0, into a scratch object under the entry's build directory. A function-local static name.N joins the variable name inside a function only when exactly one matches. A global with no type, no unit or an unsupported type is recorded as untyped with the reason in build.json"
  - "A vector whose run ends in a fault by signal, an abort or a timeout has no result frame, so its lib_state_in and lib_state_out are empty (as its stdout already is); a slack-write fault has a frame and records its states"
  - "build.json is additive (layouts, global_types, both #[serde(default)]); BUILD_SCHEMA_VERSION stays 1. mt vectors layouts reads only build.json, prints one line per state in target, configuration, entry and state order, and exits 1 with result: failed when there is no line, so a missing layer is never green"
  - "tinycbor cbor_parser_init: the flags scalar has the single value 0. The only documented parser flag, CborParserFlag_ExternalSource (0x01), makes the parser call through source.ops, which a buffer entry sets to a data pointer, so sweeping it would be undefined behavior and not a second configuration of the call"

patterns-established:
  - "Layout lines are comparable text: mt vectors layouts prints the same line locally and in the image, and the plan's cross-check is a sorted diff of the two"
  - "A decoder test builds its CType by hand; only the parser tests need DWARF text, and those use the committed GCC 14.3 dump"

coverage:
  - id: D1
    description: "The DWARF layout reader: the GCC 14.3 dump of the probe parses into a struct with a 1-byte enum, a 3-bit bitfield at bit 8, a 5-byte array, a union and a typedef resolved to the struct; unknown tags, cycles, a truncated dump and an unknown variable fail closed; the summary text is ordered"
    requirement: EQV-04
    verification:
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_04_layout.rs#eqv_04_layout_parser"
        status: pass
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_04_layout.rs#eqv_04_layout_rejects_unsupported_tags"
        status: pass
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_04_layout.rs#eqv_04_layout_rejects_a_cycle_without_a_pointer"
        status: pass
    human_judgment: false
  - id: D2
    description: "The state role and the unobserved marks in the manifest: pointer, name, pointee grammar, reserved and duplicate names, reason, path grammar, declared state, duplicates, TOML parsing and absence when empty; Phase 2 manifests validate as before"
    requirement: EQV-03
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/eqv_03_roles.rs#eqv_03_roles_state_needs_a_pointer_and_a_name"
        status: pass
      - kind: unit
        ref: "crates/mt-capture/tests/eqv_03_roles.rs#eqv_03_unobserved_marks_need_a_reason_and_a_declared_state"
        status: pass
      - kind: unit
        ref: "crates/mt-capture/tests/eqv_03_roles.rs#eqv_03_unobserved_marks_parse_from_toml_and_stay_absent_otherwise"
        status: pass
    human_judgment: false
  - id: D3
    description: "The decoder: every pointer region (null, in, out, one past the end, a state, heap, a global, a Thumb function) and the unresolved reason; union members decoded from the same bytes; bitfields across bytes with sign extension; signed 8-bit, 1-byte enum, 64-bit decimal strings and float bit patterns; unobserved marks applied, checked against the layout and counted"
    requirement: EQV-04
    verification:
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_04_decode.rs#eqv_04_decode_classifies_every_region"
        status: pass
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_04_decode.rs#eqv_04_decode_union_records_every_member"
        status: pass
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_04_decode.rs#eqv_04_decode_bitfields_and_signed_values"
        status: pass
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_04_decode.rs#eqv_04_unobserved_marks_replace_fields_and_are_counted"
        status: pass
    human_judgment: false
  - id: D4
    description: "Typed writable globals: a changed typed global is recorded in both states (5 before, 8 after in a synthetic driver run end to end), an untyped one stays untyped_global_changed, and microcoap's light and rsp are typed from their unit's DWARF at build time"
    requirement: EQV-04
    verification:
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_04_decode.rs#eqv_04_changed_global_is_recorded_typed"
        status: pass
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_04_globals.rs#eqv_04_changed_global_with_a_type_is_recorded_in_both_states"
        status: pass
      - kind: other
        ref: "MT_LOCAL_CAPTURES=.mt-work/capture cargo test -p mt-harness --test eqv_04_exec -- --ignored eqv_04_globals_are_typed_at_build_time"
        status: pass
    human_judgment: false
  - id: D5
    description: "microcoap coap_parse records coap_packet_t field by field with its option and payload pointers as {buffer: in, offset}, lib_state_in zero, no raw address, two runs identical; nimble ble_uuid_init_from_buf records every member of the union ble_uuid_any_t in both configurations; the generated microcoap driver is pinned by a golden"
    requirement: EQV-03
    verification:
      - kind: other
        ref: "MT_LOCAL_CAPTURES=.mt-work/capture cargo test -p mt-harness --test eqv_04_exec -- --ignored (microcoap 16 x2, nimble default 200 x2, nimble peripheral 200 x2 identical)"
        status: pass
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_03_driver_golden.rs#eqv_03_driver_golden"
        status: pass
    human_judgment: false
  - id: D6
    description: "In the pinned image six targets (corejson, lz4, libfdt, microcoap, nimble, tinycbor) produce vectors in every configuration, each run twice identically, with the states of microcoap, nimble and tinycbor recorded, and a second container reproduces the corpus byte for byte"
    requirement: EQV-03
    verification:
      - kind: other
        ref: "testbed run 40550615693 (68dbd06): vectors-build 12 built, vectors-run 12 rows twice=identical, vectors-determinism identical 1212 files; 03-05-VECTORS-NOTICES.md"
        status: pass
    human_judgment: false
  - id: D7
    description: "A2 measured: the layouts computed by GCC 14.3 in the image for microcoap and nimble equal the layouts computed locally by GCC 13.2.1, line for line (the vectors-layout notice against the local layout lines)"
    requirement: EQV-04
    verification:
      - kind: other
        ref: "the corrected Task 3 verify against 03-05-VECTORS-NOTICES.md and eqv_04_print_local_layouts: diff of the sorted local and CI lines is empty (4 lines)"
        status: pass
    human_judgment: false
  - id: D8
    description: "mt vectors layouts and the workflow step that publishes it: ordered rows from build.json, exit 1 with no row, the step after the build and before the run, no upstream code run"
    requirement: EQV-04
    verification:
      - kind: integration
        ref: "crates/mt-cli/tests/eqv_04_vectors_cli.rs#eqv_04_layouts_cli_prints_the_layouts_of_built_entries"
        status: pass
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_04_workflow_static.rs#eqv_04_workflow_static_layout_step_follows_the_build"
        status: pass
    human_judgment: false
  - id: D9
    description: "What the recorded state means for later comparison: the lib_state of union members that alias (tinycbor's source.ptr and source.token, nimble's u16 over u32), the pointer-to-state form {buffer: parser}, and the empty lib_state of a crashed run are the shapes the comparators of phase 5 will read"
    requirement: EQV-03
    verification: []
    human_judgment: true
    rationale: "The tests prove the executor records what the C does in the contract's shapes; whether aliasing union members are compared member by member or through an abstraction function is a phase 5 reading, and the empty state of a signal fault is an interpretation of the contract"

duration: "about 1 h 15 min of wall clock; no start time was recorded, the plan ledger was created at 16:56:45Z after the reading and the last commit is 17:37:52Z, with one CI round trip of about 12 min"
completed: 2026-10-27
---

# Phase 3 Plan 05: DWARF-laid-out library state, pointer regions, unobserved marks and typed globals Summary

**Library state is now recorded field by field from the pinned GCC's own DWARF: the state role passes a zero-filled object in a guarded mapping, a parsed layout turns its bytes into typed values (unions, bitfields, 1-byte enums, 64-bit values, floats), every pointer becomes null, a region offset, a function name or a reasoned unobserved mark, a changed global is typed from its unit's DWARF, and testbed run 40550615693 reproduced six targets (microcoap, nimble and tinycbor new) byte for byte in the pinned image on the first round trip, with GCC 14.3's layouts equal to the local GCC 13.2.1's.**

## Performance

- **Duration:** about 1 h 15 min of wall clock (see the frontmatter; no start time was recorded), one CI round trip of about 12 min.
- **Tasks:** 3 of 3 (Tracer, Task 2, Task 3), each RED then GREEN.
- **Files:** 29 changed outside `.planning` against `0e7ccd0` (4732 insertions, 95 deletions, of which 933 insertions are the three golden drivers), plus the two notices files and this summary.
- **Pushes and CI:** 1 round trip of the 3 allowed (testbed 40550615693 and ci 40550615682 green on `68dbd06`); the docs-only notices commit `7878588` was pushed with ci 40552443023 green.

## Accomplishments

- **State role and marks (EQV-03, D-09, D-11):** `Role::State` with `state_pointee_type`, `UnobservedMark`, `Entry.unobserved`, `parse_mark_path` and the validator rules. microcoap `coap_parse` (state `pkt`), nimble `ble_uuid_init_from_buf` (state `uuid`) and tinycbor `cbor_parser_init` (`flags` scalar, states `parser` and `it`) carry roles with D-09/D-11 comments.
- **DWARF layout (D-29):** `parse_dwarfdump` reads the `llvm-dwarfdump --debug-info` text by indentation into DIEs and lowers a variable's type into a `CType`; `layout_for_entry` compiles the probe with the pair's own flags through the runner, with the 16 MiB dump cap and an incomplete dump as an error; states larger than 64 KiB are refused.
- **Decoder (D-06, D-09):** `decode_value`, `classify_pointer`, `Regions`; unions, bitfields across bytes with sign extension, enums as integers of their size, 64-bit values as decimal strings, floats as bit patterns, padding never read.
- **Runtime:** each state in its own guarded zero-filled mapping, snapshotted before the call, an address region so the host builds its pointer regions from the driver's own frame, and the state sizes compared with the DWARF sizes on every run.
- **Typed globals (D-10):** `layout_for_globals` recompiles the defining unit of each writable global with its own flags plus `-g -gdwarf-5 -O0`; `record_globals` records a changed typed global in both states and keeps `untyped_global_changed` for the rest.
- **CLI and CI:** `mt vectors layouts`; the step "Publish the state layouts" and the notice `vectors-layout`.
- **Measured in the pinned image:** `vectors-build` 12 `built` rows (corejson, libfdt, lz4, microcoap, nimble, tinycbor, both configurations each), `vectors-run` 12 rows `twice=identical` with microcoap 16 vectors per configuration and nimble and tinycbor 200, `vectors-determinism` `identical 1212 files`, and four layout lines for microcoap and nimble equal to the local ones.

## Task Commits

1. **Tracer** (`tdd`): `48cbe28` test (RED), `5a90a44` feat (GREEN)
2. **Task 2** (`tdd`): `ea78e18` test (RED), `58a8f4c` feat (GREEN)
3. **Task 3** (`tdd`): `667f80d` test (RED), `68dbd06` feat (GREEN, pushed; round trip 1)
4. **Notices:** `7878588` docs (round trip 1 notices and the Task 3 verify correction, pushed)

No REFACTOR commits were needed. `commits: 7` is the measured `git rev-list --count 0e7ccd0..HEAD` when this summary was written; the summary, STATE and ROADMAP commits follow it.

## CI round trips (1 of 3 used)

| Round trip | Head | testbed | ci | Conclusion |
|------------|------|---------|----|------------|
| 1 | `68dbd06` | 40550615693 (job 122353562262) success | 40550615682 success | green; container run 40550615790 was not awaited |

- `vectors-build`: 12 `built` rows; every other entry `no_roles` (51 rows); no `unhandled_flag`, `link_unresolved` or `refused`. nimble's two configurations and tinycbor's two each share one driver hash (the configurations compile the entry's code identically).
- `vectors-run`: corejson 108, libfdt 16, lz4 60 (as in 03-04), microcoap 16, nimble 200, tinycbor 200 vectors per configuration, every one `ok=<n> fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical`.
- `vectors-layout`: eight rows, microcoap `coap_packet_t` size 220, nimble `ble_uuid_any_t` size 20, tinycbor `CborValue` size 16 and `CborParser` size 8. The CI corpus itself is not readable from here (artifact hosts are blocked); locally the provenance `unobserved` total is 0 for microcoap and nimble in both configurations and for the fabricated tinycbor run (a throwaway measurement), so no pointer fell outside the known regions there.
- `vectors-determinism`: rerun exit 0, rows identical, `identical 1212 files`, `check exit code: 0`. `vectors-selftest` (`result: passed (17 rows)`, llvm-dwarfdump 16.0.6) and `vectors-tractor` still pass; the eight Phase 2 sections of `03-05-CI-NOTICES.md` equal `02-22-CI-NOTICES.md` from line 5 on.
- The pre-decided handling for A2 (layout differs), pointers outside known regions and a tinycbor failure was not triggered.

## TDD evidence (RED output, with a one-line assessment per failure)

**Tracer RED (`48cbe28`)**

```
error[E0432]: unresolved import `mt_harness::layout` / `mt_harness::probe`            (eqv_04_layout)
error[E0432]: unresolved import `mt_capture::manifest::state_pointee_type`            (eqv_03_roles)
error[E0599]: no variant, associated function, or constant named `State` found for enum `Role`
test eqv_03_driver_golden ... FAILED   called `Result::unwrap()` on an `Err` value: NoRoles
test eqv_04_state_is_recorded_field_by_field ... FAILED   the driver must build: [BuildRow { target: "microcoap", ..., state: NoRoles }]
```

Assessment: right reason for each; the layout and role tests name the missing items, the golden test and the local proof fail on the absent microcoap roles. The golden file itself is generated output and arrives with GREEN (deviation 2).

**Task 2 RED (`ea78e18`)**

```
error[E0432]: unresolved import `mt_capture::manifest::UnobservedMark`                (eqv_04_decode, eqv_03_roles)
error[E0432]: unresolved imports `mt_harness::decode::apply_marks`, `mt_harness::decode::check_marks`
error[E0432]: unresolved import `mt_harness::globals::record_globals` / `mt_harness::probe::GlobalType`
error[E0599]: no method named `with_function` found for struct `Regions`
error[E0609]: no field `unobserved` on type `&mut Entry`; no field `global_types` on type `BuildRecord`   (eqv_04_exec)
test eqv_04_layouts_cli_prints_the_layouts_of_built_entries ... FAILED   stderr: error: unrecognized subcommand 'layouts' (left Some(2), right Some(0))
test eqv_04_layouts_cli_fails_closed_without_a_build ... FAILED          (left Some(2), right Some(1))
```

Assessment: right reason; the Tracer's decoder already classified `in`, `out` and states and refused unions, bitfields and floats, so the Task 2 tests name the items Task 2 adds. The CLI tests assert exit codes 0 and 1, so clap's exit code 2 cannot satisfy them by accident.

**Task 3 RED (`667f80d`)**: `eqv_04_workflow_static_layout_step_follows_the_build` FAILED (`.github/workflows/testbed.yml has no step "Publish the state layouts"`). Assessment: right reason, an assertion on the absent step; the allowed-subcommand list of the existing test gained `layouts` in the same commit and passes on its own.

GREEN: every named test passes; the four pre-merge gates exit 0 on the pushed head (`cargo test --workspace --locked`: 492 passed, 0 failed, 12 ignored, no `skipping:` line), and the ignored local proofs pass with `MT_LOCAL_CAPTURES` (9 tests).

## Tracer feedback gate

Auto mode was off and `human_verify_mode` was the default `end-of-phase`; the tracer's `<verify>` is automated-only, so it was re-run after the Tracer GREEN commit (the layout, golden and role tests, the ignored microcoap proof printing `microcoap/default/coap_parse vectors=16 identical` with the earlier 03-04 proofs still identical, the four gates) and logged `Tracer verified end-to-end - expanding`. No checkpoint was synthesized.

## Deviations from Plan

**1. [Rule 3 - Blocking] New public fields broke struct literals in existing tests.** `Entry.unobserved` needed `unobserved: Vec::new()` in `bed_01_entries.rs` (two), `bed_01_entry_include_prefix.rs` and `eqv_04_globals.rs`; `PreparedDriver` gained fields and derives `Default`, and `eqv_04_globals.rs` builds it with `..PreparedDriver::default()`. No assertion changed. A clippy lint in the Tracer's RED test (`needless_borrow` in `eqv_04_exec.rs`) was fixed in the GREEN commit. Commits `5a90a44`, `58a8f4c`.

**2. [Interpretation] The goldens arrive with GREEN.** The plan says the RED commit holds the tests and the golden, but a golden is the generated output of the implementation: the microcoap golden cannot exist before the roles and the template change, and the corejson and lz4 goldens change because the template grew (the MT_STATES fragment, the address and state regions, 115 insertions and 11 deletions each). The RED test fails on the absent roles (`NoRoles`), and the three goldens are re-blessed in `5a90a44` with `MT_BLESS_GOLDEN=1`; their diff is the review of the template change.

**3. [Interpretation] `decode_value` takes no `LayoutModel`.** The plan listed `decode_value(model, type, bytes, &Regions)`. The parsed layout is lowered into an owned `CType` tree (pointers keep only their pointee's text, so the tree is finite), which makes the model unnecessary at decode time and lets the decoder tests build types by hand. `LayoutModel::type_of_variable` returns the `CType`.

**4. [Plan defect] The Task 3 verify as written cannot pass.** Its CI side of the layout cross-check is `grep -E '^(microcoap|nimble)/' | grep -v ' vectors=' | grep -v ' built '`, which also matches the `no_roles` rows of nimble's other entry `ble_hs_adv_parse_fields` in the `vectors-build` and `vectors-run` sections (four lines). With `grep -v ' no_roles'` added the sorted local and CI lines are equal; `03-05-PLAN.md` carries that one-line correction (commit `7878588`), and every other check of the verify passes as written.

**5. [Minor] Tracer decoder staged.** The Tracer's decoder handles integers, enums, arrays, structs and pointers into named regions and refuses unions, bitfields and floats; Task 2 adds them, with functions, heap and globals as regions, so that Task 2's RED is a real RED against the Tracer implementation.

**6. [Minor] Public API beyond the artifact list.** `record_globals` and `function_symbols` (globals.rs), `layout_lines` (run.rs), `recompile_arguments`, `GlobalTarget` and `GlobalType` (probe.rs), `apply_marks` and `check_marks` (decode.rs), `parse_mark_path` and `MarkStep` (manifest.rs). The first two are needed by the tests the plan asked for; `layout_lines` is what both the CLI and `eqv_04_print_local_layouts` print.

**7. [Minor] Extra tests.** `eqv_04_changed_global_with_a_type_is_recorded_in_both_states` (a synthetic driver whose global changes from 5 to 8, run end to end through the executor) was added with the Task 2 GREEN commit and passed on first run, not RED first; the CLI test `eqv_04_layouts_cli_fails_closed_without_a_build`, the probe source and flags tests and `eqv_03_state_pointee_type` were added in the RED commits.

**8. [Process] Development evidence for tinycbor.** tinycbor has no local objects (its capture is `build_failed` on this host because GCC 13's newlib lacks `PRIu64` in `cborpretty.c`), so before the first push a throwaway capture was fabricated in the session scratch directory from four locally compiled objects (`cborparser`, `cborerrorstrings`, `cborencoder`, `cborencoder_close_container_checked`, plus two more) and the real `testbed/manifest.toml` roles were run against it: the driver linked, 200 vectors ran twice identically, and `it` and `parser` were recorded with `{buffer: parser}` and `{buffer: in}` pointers. This is development evidence only (GCC 13.2.1, hand-compiled), not a capture; the proof is the CI row. The scratch test file was deleted and nothing of it is committed.

**9. [Process] No start time was recorded** at the first step, so the duration in the frontmatter is estimated from the plan ledger and the commit times.

**Total deviations:** 9 (1 Rule 3, 1 plan defect, 3 interpretations or staging choices, 4 minor or process). **Impact:** none on the plan's goal; one of three round trips used.

## Decisions Made

See `key-decisions` in the frontmatter. For the founder to read at `/gsd-verify-work`: the empty `lib_state` of a signal fault, abort or timeout (no frame exists to read), the state-name namespace shared with scalar names, the tinycbor `flags` choice, and the shared `UnobservedMark` path grammar.

## Known Stubs

None. `Provenance.unobserved` is 0 for every locally measured vector of microcoap and nimble (and of the fabricated tinycbor run), and corejson, lz4 and libfdt have no state; the three new entries have no `unobserved` tables because none was needed.

## Threat Flags

None beyond the plan's register. Mitigations exercised: T-03-19 by `probe_flags` (the driver's fail-closed flags, kept ABI flags), `eqv_04_layout_parser` on the GCC 14.3 fixture, the per-run check that the driver's `sizeof` equals the DWARF size, and the cross-check of the CI layouts against the local ones (equal); T-03-20 by `eqv_04_decode_classifies_every_region` and `eqv_04_state_is_recorded_field_by_field` (only null, `{buffer, offset}` and `{symbol}` appear; a changed global is recorded only typed); T-03-21 by the runner's 16 MiB cap with incomplete output as an error, the cycle check in the lowering and `LayoutError::Unsupported` for unknown tags; T-03-SC: no external crate added (`Cargo.lock` gained only the three path crates against `949214e`).

## Issues Encountered

- **Function-local statics cannot be keys of `globals`.** A symbol such as `version.0` is typed correctly (the join rule is implemented and unit-tested), but `LibState` requires the symbol part of a `globals` key to be a C identifier, so a changed function-local static would fail the run with the validation message instead of being recorded. None of the six targets' changed globals hit this (no global changed at all in these corpora), so the rule was left as plan 03-03 wrote it; a target that needs it (cJSON in a later plan) must either relax the key rule in `mt-vectors` or accept the failure.
- **lz4's overshoot (open question from 03-04) is untouched:** D-07 still records the whole output buffer, as decided.
- **Signal faults record no state.** A run that ends in SIGSEGV, SIGBUS, SIGILL, SIGFPE, abort or timeout has no frame, so `lib_state_in` and `lib_state_out` are `{}` for it, while a slack-write fault (frame present) records both. None of the 16, 400 and 400 new vectors is such a case.
- The ignored local proofs need `MT_LOCAL_CAPTURES` and ran on this host with QEMU 8.2.2, GCC 13.2.1 and llvm-dwarfdump 18.1.3; in CI they ran on QEMU 10.0.2, GCC 14.3 and llvm-dwarfdump 16.0.6 with the same row counts.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plans 03-06 to 03-08 only add roles and setup steps: `Role` is a closed enum, `state` gives them the object to fill, and a drop for an entry that cannot be laid out would be named for plan 03-06's adapter drop log (none was needed).
- With plan 03-04's three targets this makes 6 of the 10-target floor drivable in the pinned image (corejson, lz4, libfdt, microcoap, nimble, tinycbor).
- Not pushed: this summary, STATE.md and ROADMAP.md commits follow the pushed notices commit and ride with the next plan's first push.

## Self-Check: PASSED

- Created files found on disk: `layout.rs`, `probe.rs`, `decode.rs`, `eqv_04_layout.rs`, `eqv_04_decode.rs`, the microcoap golden driver, both notices files.
- Commits found as ancestors of HEAD: 48cbe28, 5a90a44, ea78e18, 58a8f4c, 667f80d, 68dbd06, 7878588 (HEAD equals origin at the time of writing).
- Plan-level verification: pre-merge gates exit 0 on the pushed head; `Cargo.lock` gained only the three path crates against `949214e`; no forbidden claim word in `crates/mt-harness/src` or the vectors notices; Phase 2 tests pass unchanged; the cited testbed run 40550615693 and ci runs 40550615682 and 40552443023 are green for their heads; the Task 3 verify passes with the one-line correction of deviation 4.
