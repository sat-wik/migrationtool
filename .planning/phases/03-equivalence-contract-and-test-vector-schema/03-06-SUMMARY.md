---
phase: 03-equivalence-contract-and-test-vector-schema
plan: 06
subsystem: testing
tags: [adapters, manifest-roles, inout-len, out-scalar, out-ptr, str-args, struct-return, heap-walk, drop-log, floor, github-actions]

requires:
  - phase: 03-equivalence-contract-and-test-vector-schema
    provides: plan 03-03's StateValue, LibState and corpus writer, plan 03-04's roles, generated drivers, build.json and the twice-run executor, plan 03-05's DWARF layouts, state role, pointer regions and typed globals
  - phase: 02-test-bed-and-build-capture
    provides: capture.json, the replayed objects, the committed seeds and Phase 2's drop log
provides:
  - "mt_capture::manifest: Role::{InoutLen, OutScalar, OutPtr, Str, StrLen}, ArgRole.{of, mutable, values}, value_pointee_type, pointer_pointee_type, MAX_STR_VALUES, MAX_STR_BYTES; manifest schema_version stays 1"
  - "mt_harness::adapter: ReturnKind::Struct, Adapter::{with_return_kind, records_return, strings, mutable_input, input_element_type, return_text}, StateParam.init_word, RET_NAME"
  - "mt_harness::heap: walk_heap, HEAP_WALK_CAP, HEAP_CAP_REASON; LayoutModel::pointee_types; probe::EntryLayouts"
  - "mt_harness::adapter_drops: AdapterDropLog, AdapterDrop, AdapterDropCause, ADAPTER_FLOOR, drivable_targets, check_adapter_floor, testbed_drivable_targets"
  - "runtime template: inout_len initial values, strings in guarded mappings (request field 5), frame regions 3 (input after the call) and 4 (used heap bytes)"
  - "mt vectors adapters --check-floor, dropped rows, and compact build and run listings (no_roles: <n>)"
  - "reference vectors for zlib uncompress, paho MQTTDeserialize_connack and MQTTDeserialize_publish, nanopb pb_istream_from_buffer, cjson cJSON_ParseWithLength and corejson JSON_SearchT; with plans 03-04 and 03-05 ten targets produce vectors in the pinned image (testbed run 40761178752)"
affects: [03-07, 03-08, phase 4 relabelling, phase 5 comparators and abstraction functions, phase 11 evidence bundle]

requirements-completed: [EQV-03, EQV-04]

status: complete
commits: 9
plan_head_before: 6272aaf1d9e39d00eab5d822b437eb598e059b25
plan_head_after: 7dd826fcd616f5e233431b39f576b41024f07c2a
actuals:
  tokens: 48533
  tasks: 3
  commits: 9

tech-stack:
  added: []
  patterns:
    - "every pointer-valued parameter that the C code fills in (state, inout_len, out_scalar, out_ptr) is a state: its own guarded mapping, a DWARF layout from a probe variable of its pointee type, recorded before and after the call; a pointer or struct return is one more state, called ret, after the declared ones"
    - "a typedef name cannot say whether it is an integer, an enum, a struct or a pointer, so the build resolves the return kind from the DWARF of a probe variable of the return type and records it in build.json"
    - "strings are data: they travel in the request file into guarded NUL-terminated mappings, never into generated C; argv carries the text and the derived length"
    - "the heap walk reads a table of pointee types (the text of a pointer minus its trailing *) because a CType keeps only the text of what a pointer points at"
    - "the floor of drivable targets is a count over committed files (manifest, seed indexes, both drop logs), so no tool and no network is needed to prove it"

key-files:
  created:
    - crates/mt-harness/src/heap.rs
    - crates/mt-harness/src/adapter_drops.rs
    - crates/mt-harness/tests/eqv_03_adapter_floor.rs
    - crates/mt-harness/tests/eqv_04_listing.rs
    - crates/mt-harness/tests/fixtures/golden/zlib-uncompress.driver.c
    - testbed/adapter-dropped.toml
    - .planning/phases/03-equivalence-contract-and-test-vector-schema/03-06-CI-NOTICES.md
    - .planning/phases/03-equivalence-contract-and-test-vector-schema/03-06-VECTORS-NOTICES.md
  modified:
    - crates/mt-capture/src/manifest.rs
    - crates/mt-capture/tests/eqv_03_roles.rs
    - testbed/manifest.toml
    - crates/mt-harness/src/adapter.rs
    - crates/mt-harness/src/layout.rs
    - crates/mt-harness/src/probe.rs
    - crates/mt-harness/src/run.rs
    - crates/mt-harness/src/replay.rs
    - crates/mt-harness/src/exec.rs
    - crates/mt-harness/src/runtime.c
    - crates/mt-harness/src/runtime.rs
    - crates/mt-harness/src/lib.rs
    - crates/mt-harness/Cargo.toml
    - crates/mt-harness/tests/eqv_03_adapter.rs
    - crates/mt-harness/tests/eqv_03_driver_golden.rs
    - crates/mt-harness/tests/eqv_04_decode.rs
    - crates/mt-harness/tests/eqv_04_exec.rs
    - crates/mt-harness/tests/eqv_04_globals.rs
    - crates/mt-harness/tests/fixtures/golden/corejson-JSON_Validate.driver.c
    - crates/mt-harness/tests/fixtures/golden/lz4-LZ4_decompress_safe.driver.c
    - crates/mt-harness/tests/fixtures/golden/microcoap-coap_parse.driver.c
    - crates/mt-cli/src/main.rs
    - crates/mt-cli/tests/eqv_04_vectors_cli.rs
    - Cargo.lock

key-decisions:
  - "inout_len is a state whose initial value is a request word written into its mapping before the snapshot (so lib_state_in shows the declared value), with of = \"out\" it is the capacity of the output buffer and the entry has no out_cap; its values are a combination dimension and are capacities, so they lie in 0..=65536. argv carries the initial value in parameter order"
  - "out_scalar is a pointer to a plain scalar type (one to three identifiers, or enum X, never const), out_ptr a pointer to a pointer; both are states with a probe variable of the pointee type (unsigned char, short unsigned int, JSONTypes_t, unsigned char *). The recorded type text is the DWARF's: GCC names unsigned short short unsigned int"
  - "str values are 1 to 8 strings of printable ASCII, at most 64 bytes each, none repeated; each is a combination dimension (the index is the provenance scalar) and the derived str_len is argv text. The strings sit in guarded mappings at 0x06800000 + i * 1 MiB with a NUL after them; the template refuses a request with more than 8 or longer than 64"
  - "in_buf mutable records the input after the call as lib_state_out.in (hex, type <element>[n]) and is refused on a const pointee or on any other role"
  - "return kinds: void is 0, a pointer is 0 or 1 plus ret, a struct or union by value is 0 plus ret, anything else is an integer cast to 32 bits; the signature text decides pointer, void, struct X and union X, and the DWARF of a probe variable decides a typedef name (build.json return_kind); float, double and 64-bit returns stay refused"
  - "heap walk: the arena's used bytes are a frame region; roots are all recorded pointers classified into heap; objects are decoded with the pointee type found in the table, keyed by decimal offset, FIFO order, once each; a char pointee is the NUL-terminated run including the NUL (type char[n]); at 4096 objects the next pointer becomes unobserved with the reason heap walk cap reached; a pointer to an unknown type, a void pointee or bytes outside the used arena stays a classified pointer"
  - "ADAPTER_FLOOR is 10 and drivable_targets counts a retained target with at least one entry that has roles, a seed whose index lists that entry and no adapter drop row for it. AdapterDropLog::validate checks the rows against the manifest and the fixed texts; check_consistency (an entry is logged or has roles, never both) is separate and is not run by the CLI, so a log that drops a floor entry lowers the count instead of failing as invalid"
  - "mt vectors build and run print no_roles: <n> instead of one row per entry without roles, because GitHub keeps 50 annotations per job and the vectors notices need 49 of them"

patterns-established:
  - "A recorded state is one mechanism for every pointer the C code fills in; a new output form is a new way to name its pointee type, not a new recorder"
  - "Notices are rebuilt from the job annotations by one script, with a per-title part check, and the eight Phase 2 sections are compared with 02-22 from line 5 on"

coverage:
  - id: D1
    description: "inout_len: a pointer-carried length whose declared value goes in and whose produced value comes out; it sizes the output buffer (of = out); zlib uncompress records the initial capacity in argv and lib_state_in, the produced length in lib_state_out, the whole output buffer with the sentinel tail, and a Z_BUF_ERROR partial write at capacity 16, identically twice in both configurations"
    requirement: EQV-03
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/eqv_03_roles.rs#eqv_03_roles_inout_len_may_size_the_output"
        status: pass
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_03_adapter.rs#eqv_03_adapter_inout_len_argv_and_capacity"
        status: pass
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_03_driver_golden.rs#eqv_03_driver_golden"
        status: pass
      - kind: other
        ref: "MT_LOCAL_CAPTURES=.mt-work/capture cargo test -p mt-harness --test eqv_04_exec -- --ignored eqv_04_inout_len_is_recorded_in_and_out (zlib default and no-gzip 54 vectors each)"
        status: pass
    human_judgment: false
  - id: D2
    description: "out_scalar, out_ptr, str with str_len and a mutable in_buf: roles validated (pointer shapes, names, printable bounded values, no const), recorded as states, strings as request data; paho connack and publish, and corejson JSON_SearchT with its query values, produce identical vectors"
    requirement: EQV-03
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/eqv_03_roles.rs#eqv_03_roles_str_values_are_printable_and_bounded"
        status: pass
      - kind: unit
        ref: "crates/mt-capture/tests/eqv_03_roles.rs#eqv_03_roles_mutable_input_needs_a_non_const_pointer"
        status: pass
      - kind: unit
        ref: "crates/mt-capture/tests/eqv_03_roles.rs#eqv_03_roles_out_roles_need_pointers_and_names"
        status: pass
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_03_adapter.rs#eqv_03_adapter_str_and_out_roles"
        status: pass
      - kind: other
        ref: "eqv_04_out_scalars_and_pointers_are_recorded and eqv_04_string_argument_and_mutable_input (ignored, local captures)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Struct and pointer returns are lib_state_out.ret with rc 0 or 0/1, and the typed heap walk records the objects a returned pointer reaches by decimal offset with char runs in hex, decoding a cycle once and stopping at 4096 objects with an unobserved mark; nanopb records pb_istream_t and cjson records its parse trees and its error global"
    requirement: EQV-04
    verification:
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_04_decode.rs#eqv_04_heap_walk_follows_typed_edges"
        status: pass
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_04_decode.rs#eqv_04_heap_walk_cap_is_an_unobserved_mark"
        status: pass
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_04_decode.rs#eqv_04_struct_return_is_ret"
        status: pass
      - kind: other
        ref: "eqv_04_struct_return_is_recorded and eqv_04_heap_tree_is_recorded (ignored, local captures; cjson 10 parsed, 15 failed, 332 string objects per configuration)"
        status: pass
    human_judgment: false
  - id: D4
    description: "The adapter drop log with the three no_seeds rows, the floor of 10 as a test over the committed files, and mt vectors adapters --check-floor"
    requirement: EQV-03
    verification:
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_03_adapter_floor.rs#eqv_03_adapter_floor_is_ten"
        status: pass
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_03_adapter_floor.rs#eqv_03_no_seeds_entries_are_logged"
        status: pass
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_03_adapter_floor.rs#eqv_03_adapter_drop_log_is_consistent"
        status: pass
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_03_adapter_floor.rs#eqv_03_floor_counts_only_seeded_undropped_entries"
        status: pass
      - kind: integration
        ref: "crates/mt-cli/tests/eqv_04_vectors_cli.rs#eqv_04_adapters_cli_checks_the_floor"
        status: pass
    human_judgment: false
  - id: D5
    description: "In the pinned image ten targets (cjson, corejson, libfdt, lz4, microcoap, nanopb, nimble, paho, tinycbor, zlib) produce vectors in every retained configuration, each run twice identically, and a second container reproduces the corpus byte for byte (2462 files)"
    requirement: EQV-03
    verification:
      - kind: other
        ref: "testbed run 40761178752 (f67dd1d): vectors-build 22 built rows, vectors-run 22 rows twice=identical, vectors-determinism identical 2462 files; 03-06-VECTORS-NOTICES.md"
        status: pass
    human_judgment: false
  - id: D6
    description: "What the new records mean for later comparison: the empty lib_state of a crashed run, a pointer beyond the input recorded as an unobserved mark although the C reports success (paho publish), the ret struct with aliasing or function-pointer members, and the heap objects keyed by offset"
    requirement: EQV-03
    verification: []
    human_judgment: true
    rationale: "The tests prove the executor records what the C does in the contract's shapes; whether a comparator treats these records member by member, through an abstraction function or as undefined behavior is a phase 4 and 5 reading"

duration: "about 66 min of wall clock (2026-10-28T19:22:08Z to 20:28Z), including two CI round trips of about 10 min each"
completed: 2026-10-28
---

# Phase 3 Plan 06: Output roles, struct and pointer returns, the typed heap walk, the adapter drop log and the floor of ten Summary

**Six more entries are drivable (zlib uncompress, paho connack and publish, nanopb pb_istream_from_buffer, cjson cJSON_ParseWithLength, corejson JSON_SearchT) through five new roles, struct and pointer returns recorded as `ret`, a typed heap walk and strings that travel as request data, the floor of 10 drivable targets is a test over the committed files with the three no_seeds entries logged, and testbed run 40761178752 reproduced ten targets in the pinned image twice-identical with a byte-identical rerun corpus (2462 files) on round trip 2 of 3.**

## Performance

- **Duration:** about 66 min of wall clock (start recorded 19:22:08Z, last commit 20:23:46Z), of which two CI round trips took about 10 min each.
- **Tasks:** 3 of 3 (Tracer, Task 2, Task 3), each RED then GREEN, plus a RED/GREEN pair for the listing fix.
- **Files:** 30 changed outside `.planning` against `6272aaf` (4757 insertions, 223 deletions, of which about 42k characters are the four golden drivers), plus the two notices files and this summary.
- **Pushes and CI:** 2 round trips of the 3 allowed. Round trip 1 (testbed 40758919770, ci 40758919865) passed every step but is not counted green: the job may have 50 annotations and the 51st, `vectors-determinism`, was cut. Round trip 2 (testbed 40761178752, ci 40761178760) is green with 49 annotations. The docs-only notices commit `7dd826f` was pushed with ci 40762900849 green. The container runs were not awaited.

## Accomplishments

- **Roles (EQV-03, D-05, D-11, D-14):** `inout_len` (with `of = "out"` it sizes the output buffer), `out_scalar`, `out_ptr`, `str` with the derived `str_len`, and `in_buf` `mutable`, all validated in the manifest (pointer shapes, names, bounded printable string lists, no const output, combination cap 16) and all additive to schema 1. zlib, paho, nanopb, cjson and corejson JSON_SearchT carry roles with D-nn comments.
- **One recorder (D-06):** an inout_len, an out_scalar, an out_ptr and a returned struct or pointer are states like Phase 3's struct states: a guarded mapping, a DWARF layout from a probe variable of the pointee type, recorded before and after the call (a returned value only after). The driver template gained the initial-value table, the strings and two frame regions; four goldens pin the generated drivers.
- **Typed heap walk (D-06, D-09):** `walk_heap` follows pointers classified into `heap` through a table of pointee types, records objects by decimal offset, reads a char pointee as its NUL-terminated bytes, decodes a cycle once and turns the pointers beyond 4096 objects into unobserved marks. cjson's parse trees (10 parsing seeds, 332 string objects per configuration) and its failed-parse `global_error` (15 failing seeds, `json` a region offset into `in`) are recorded.
- **Adapter drop log and floor (D-13, D-22, D-24, D-28):** `testbed/adapter-dropped.toml` holds the three no_seeds rows; `drivable_targets` and `check_adapter_floor` count a retained target with a seeded, unlogged entry with roles; `mt vectors adapters --check-floor` prints `drivable targets: 10 (floor 10)` and exits 1 below the floor.
- **Measured in the pinned image:** `vectors-build` 22 `built` rows, `vectors-run` 22 rows `twice=identical` (cjson 25 per configuration, corejson JSON_SearchT 324, nanopb 200, paho 17 and 17, zlib 54, plus the earlier targets), `vectors-determinism` `identical 2462 files`, ten distinct targets.

## Task Commits

1. **Tracer** (`tdd`): `f72d856` test (RED), `fea72c0` feat (GREEN)
2. **Task 2** (`tdd`): `4c4719b` test (RED), `bd00c3b` feat (GREEN)
3. **Task 3** (`tdd`): `15655a3` test (RED), `72ab194` feat (GREEN, pushed; round trip 1)
4. **Annotation fix** (`tdd`): `32df7ee` test (RED), `f67dd1d` feat (GREEN, pushed; round trip 2)
5. **Notices:** `7dd826f` docs (round trip 2 notices, pushed)

No REFACTOR commits were needed. `commits: 9` is the measured `git rev-list --count 6272aaf..HEAD` when this summary was written; the summary, STATE and ROADMAP commits follow it.

## CI round trips (2 of 3 used)

| Round trip | Head | testbed | ci | Conclusion |
|------------|------|---------|----|------------|
| 1 | `72ab194` | 40758919770 (job 123065891562) success | 40758919865 success | all steps passed; 50 annotations read, `vectors-determinism` missing, so not green |
| 2 | `f67dd1d` | 40761178752 (job 123073879393) success | 40761178760 success | green; container run 40761178786 was not awaited |

- `vectors-build`: 22 `built` rows and `no_roles: 29`; no `unhandled_flag`, `link_unresolved` or `refused`. nimble's and tinycbor's two configurations still share one driver hash each.
- `vectors-run`: cjson 25 and 25, corejson JSON_SearchT 324 and 324 (and JSON_Validate 108 and 108), libfdt 16 and 16, lz4 60 and 60, microcoap 16 and 16, nanopb 200 and 200, nimble 200 and 200, paho connack 17 and publish 17 (one fault), tinycbor 200 and 200, zlib 54 and 54; every row `skipped=0 twice=identical`.
- `vectors-determinism`: rerun exit 0, rows identical, `identical 2462 files`, `check exit code: 0`. `vectors-selftest` and `vectors-tractor` still pass; the eight Phase 2 sections of `03-06-CI-NOTICES.md` equal `02-22-CI-NOTICES.md` from line 5 on (`diff` empty). The three Task 3 automated verifies exit 0, the third on `7dd826f` after ci 40762900849.
- The CI and local counts agree (paho publish `ok=16 fault=1` in both), so the local proofs on GCC 13.2.1 and QEMU 8.2.2 and the CI rows on GCC 14.3 and QEMU 10.0.2 show the same behavior.

## TDD evidence (RED output, with a one-line assessment per failure)

**Tracer RED (`f72d856`)**

```
error[E0560]: struct `ArgRole` has no field named `of`        (eqv_03_roles, eqv_03_adapter, eqv_04_globals)
error[E0599]: no variant ... named `InoutLen` found for enum `Role`
test eqv_03_driver_golden ... FAILED   called `Result::unwrap()` on an `Err` value: NoRoles
test eqv_04_inout_len_is_recorded_in_and_out ... FAILED   the driver must build: [BuildRow { target: "zlib", config: "default", entry: "uncompress", state: NoRoles }]
```

Assessment: right reason; the role and adapter tests name the missing field and variant, the golden and the local proof fail on the absent zlib roles. The golden file is generated output and arrives with GREEN (deviation 2).

**Task 2 RED (`4c4719b`)**

```
eqv_03_roles:   20 errors: ArgRole has no field `mutable` / `values`; no variant OutScalar, OutPtr, Str, StrLen for Role
eqv_03_adapter: 12 errors: ... no method `strings`, `mutable_input`, `with_return_kind`; no variant ReturnKind::Struct
eqv_04_decode:   6 errors: unresolved import `mt_harness::heap`; ReturnKind::Struct; Adapter::with_return_kind
eqv_04_globals:  2 errors: ArgRole has no field `mutable` / `values`
eqv_04_out_scalars_and_pointers_are_recorded, _struct_return_is_recorded, _heap_tree_is_recorded, _string_argument_and_mutable_input: FAILED
  the driver must build: [BuildRow { ... state: NoRoles }]   (paho connack, nanopb, cjson, corejson JSON_SearchT)
```

Assessment: right reason for each: the pure tests name the missing roles, fields, methods and module, and the four local proofs fail because the entries have no roles.

**Task 3 RED (`15655a3`)**

```
error[E0432]: unresolved import `mt_harness::adapter_drops`                      (eqv_03_adapter_floor)
test eqv_04_adapters_cli_checks_the_floor ... FAILED   stderr: error: unexpected argument '--check-floor' found  (left Some(2), right Some(0))
```

Assessment: right reason; the module is missing and clap refuses the argument. The test asserts exit codes 0 and 1, so clap's exit code 2 cannot satisfy it by accident.

**Annotation fix RED (`32df7ee`)**: `error[E0432]: unresolved imports mt_harness::run::render_build_listing, mt_harness::run::render_run_listing` (eqv_04_listing). Assessment: right reason, the functions are missing.

GREEN: every named test passes; the four pre-merge gates exit 0 on the pushed head (`cargo test --workspace --locked`: 511 passed, 0 failed, 17 ignored, no `skipping:` line), and the 14 ignored local proofs of `eqv_04_exec` pass with `MT_LOCAL_CAPTURES`.

## Tracer feedback gate

Auto mode was off and `human_verify_mode` was the default `end-of-phase`; the tracer's `<verify>` is automated-only, so it was re-run after the Tracer GREEN commit (the role and golden tests, the ignored zlib proofs printing `zlib/default/uncompress vectors=54 identical` and the `no-gzip` row, with every earlier target still identical, the four gates) and logged `Tracer verified end-to-end - expanding`. No checkpoint was synthesized.

## Deviations from Plan

**1. [Rule 3 - Blocking] GitHub keeps 50 annotations per job.** Found during Task 3, round trip 1: every step passed but the job had 51 annotations (12 `testbed-`, 31 `fixture`, 8 `vectors-`), and the 51st, `vectors-determinism`, was cut. By the plan's own rule a missing title is never green. Fix: `mt vectors build` and `run` print the rows of entries with roles and one `no_roles: <n>` line (the result line still counts every entry), which brings `vectors-build` and `vectors-run` back to one annotation each (49 of 50). RED `32df7ee`, GREEN `f67dd1d`; round trip 2 green. **This is a standing limit for plans 03-07 and 03-08:** each new drivable entry adds about 180 bytes to `vectors-run` (one part holds 3800), the job has one annotation left, and the fixture notices (31) are the only large block to trim. I did not touch the workflow or the fixture steps (outside this plan's files and Phase 2's notices).

**2. [Interpretation] The goldens arrive with GREEN.** As in plan 03-05, a golden is generated output: the zlib golden cannot exist before the roles and the template change, and the corejson, lz4 and microcoap goldens change with the template (`mt_state_word`, then strings and the two regions). They were re-blessed with `MT_BLESS_GOLDEN=1` in `fea72c0` and `bd00c3b`; the diff is the review.

**3. [Minor] `ReturnKind::Integer` keeps its name.** The plan's artifact list writes `ReturnKind { Int, Void, Pointer, Struct }`; three committed tests use `Integer`, so only `Struct` was added.

**4. [Plan expectation] Existing tests that used corejson `JSON_SearchT` as the entry without roles now use nanopb `pb_decode_ex`.** `eqv_03_roles_accept_the_committed_manifest`, `eqv_03_adapter_refuses_an_entry_without_roles`, `eqv_04_build_refuses_a_pair_without_roles` and `eqv_04_adapters_cli_lists_entries` asserted that JSON_SearchT has no roles; the plan gives it roles. Their intent (an entry without arg tables stays without roles and prints `no_roles`) is unchanged. `eqv_03_adapter_return_kinds` expected the old call text for a pointer return, `((JSON_Validate(...)) != 0 ? ...)`; a pointer return is now stored in a `ret` state, so the expected text is `((*(cJSON * *)mt_state(0) = JSON_Validate(...)) != 0 ? ...)`. All in `bd00c3b` (RED expectations in `4c4719b`).

**5. [Interpretation] The return kind of a typedef name is resolved at build time.** The plan says return kinds are derived from the signature; the signature text decides `void`, a pointer, `struct X` and `union X`, but `pb_istream_t` and `JSONStatus_t` are both typedef names. The build compiles one probe variable of the return type, reads its kind from the DWARF and stores `return_kind` in `build.json` (additive). The `ret` probe is dropped for integers, so no `ret` layout line appears for them.

**6. [Interpretation] Every new pointer role is a state.** `inout_len`, `out_scalar`, `out_ptr` and the returned value reuse the state machinery (mapping, probe, before and after record, marks) instead of new recorders; the only additions are the initial-value word for `inout_len` and the `ret` rule (recorded after the call only). `BuildRecord` also gained `pointees`, the table of pointee types for the heap walk.

**7. [Measured, test expectation wrong] Three expectations of the local proofs were corrected.** nanopb's `callback` is a function only in the default configuration; under `PB_BUFFER_ONLY` the member is an unused `int *` and records as a null pointer of type `int *`. GCC names `unsigned short` `short unsigned int` in its DWARF, and the recorded type text is the DWARF's. The paho publish test skips the one vector that ends in a signal (no frame, so no state) and accepts a `payload` recorded as unobserved (see Issues Encountered).

**8. [Minor] The adapter drop log has two checks.** `validate` (rows against the manifest, fixed texts, no duplicates) and `check_consistency` (an entry is logged or has roles, never both). The CLI runs only `validate`, so the CLI test can drop four floor entries that have roles and see the count fall to 6 and the exit code 1; the committed log is held to both by `eqv_03_adapter_drop_log_is_consistent`.

**9. [Minor] Public API beyond the artifact list:** `LayoutModel::pointee_types`, `probe::EntryLayouts`, `Request::with_strings`, `Adapter::{strings, mutable_input, input_element_type, return_text, with_return_kind, records_return}`, `adapter_drops::{testbed_drivable_targets, AdapterFloorError, NOT_ATTEMPTED_*, NO_SEEDS_ATTEMPT}`, `run::{render_build_listing, render_run_listing}`, `AdapterState::Dropped`. `mt-harness` depends on `toml` (already in the lock file for other crates; no new package).

**10. [Process] The disk filled during Task 3** (the 12 GB debug `target` plus 3.6 GB of incremental data). `target/debug/incremental` was removed, then `cargo clean`, and every later cargo command ran with `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0`; no tracked file was affected and `.mt-work/capture` was never touched.

**Total deviations:** 10 (1 Rule 3, 3 interpretations, 1 plan expectation, 1 measured, 4 minor or process). **Impact:** none on the plan's goal; two of three round trips used, the first lost to a limit the CI protocol did not anticipate.

## Decisions Made

See `key-decisions` in the frontmatter. For the founder to read at `/gsd-verify-work`: the `ret` record for pointer and struct returns, the `in` record of a mutable input, the heap walk's reading of a char pointee (bytes including the NUL, type `char[n]`), the str query values chosen for JSON_SearchT (below), the floor counting rule (entry-level drop rows) and the compact listing.

**JSON_SearchT query choice (recorded as the plan asks).** The committed seeds hold 24 JSON documents among 108 inputs and only two keys occur in them (`foo` in 2 documents, `hello` in 1), so the declared values are `foo`, `hello` and the array-index form `[0]`. Measured hits per 108 seeds in both configurations: `foo` 10 (the search finds a key also in seeds that are not whole documents), `hello` 1, `[0]` 12. A miss is a defined path, so no value was dropped.

## Known Stubs

None. `Provenance.unobserved` is non-zero only where the C code leaves a pointer outside every region (the one paho publish seed below); no `unobserved` table was added to any entry.

## Threat Flags

None beyond the plan's register. Mitigations exercised: T-03-22 by `eqv_04_heap_walk_follows_typed_edges` and `eqv_04_heap_walk_cap_is_an_unobserved_mark` (no real seed comes near the cap: the ten parsing cjson seeds hold 332 string objects in total); T-03-23 by the four floor tests and the CLI test (a seedless or logged entry does not count); T-03-24 by `eqv_03_roles_str_values_are_printable_and_bounded` and the request-field-5 transport (the call expression pinned by `eqv_03_adapter_str_and_out_roles` names `mt_str(0)` and never a value, and the values exist only in request field 5); T-03-SC: no external crate added (`Cargo.lock` gained the path crates against `949214e` and one `toml` edge on `mt-harness`).

## Issues Encountered

- **paho `MQTTDeserialize_publish` faults on one seed** (SIGSEGV, recorded as `outcome=fault` with the observed-crash label and an empty `lib_state`, locally and in the image) and **reports success with a `payload` pointer beyond the end of the input on another** (it does not check the declared length against `buflen`); that pointer is recorded as an unobserved mark with the fixed reason, counted in the provenance. Both are facts about the C for the UB decisions of phases 4 and 5.
- **cjson's function-local static `version.0` is never changed by `cJSON_ParseWithLength`**, so the open question from plan 03-05 (a `name.N` global cannot be a key of `globals`) was not triggered; it stays open for an entry that changes one. `global_error` is a file-scope static and is recorded typed. `global_hooks` never changed, so the function-pointer classification was not exercised.
- **lz4's overshoot against D-07 is untouched** (open founder question from plan 03-04).
- **Annotation budget:** see deviation 1.
- The ignored local proofs ran on this host with QEMU 8.2.2, GCC 13.2.1 and llvm-dwarfdump 18.1.3; in CI on QEMU 10.0.2, GCC 14.3 and llvm-dwarfdump 16.0.6 with the same row counts.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Ten targets are drivable and reproduced; `eqv_03_adapter_floor_is_ten` guards the count, so plans 03-07 and 03-08 can add entries or log them (`not_expressible`, `attempt_failed`, `not_attempted`) without touching the floor logic, and the floor restoration rule did not have to be used.
- Before 03-07 pushes, decide how to stay within 50 annotations per job (deviation 1): the job is at 49.
- Not pushed: this summary, STATE.md and ROADMAP.md commits follow the pushed notices commit and ride with the next plan's first push.

## Self-Check: PASSED

- Created files found on disk: `heap.rs`, `adapter_drops.rs`, `eqv_03_adapter_floor.rs`, `eqv_04_listing.rs`, the zlib golden driver, `testbed/adapter-dropped.toml`, both notices files.
- Commits found as ancestors of HEAD: f72d856, fea72c0, 4c4719b, bd00c3b, 15655a3, 72ab194, 32df7ee, f67dd1d, 7dd826f (HEAD equals origin at the time of writing).
- Plan-level verification: pre-merge gates exit 0 on the pushed head; `Cargo.lock` gained no external package against `949214e`; no forbidden claim word in `crates/mt-harness/src`, `testbed/adapter-dropped.toml` or the vectors notices; `mt vectors adapters --check-floor` prints `drivable targets: 10 (floor 10)` and exits 0; the cited testbed run 40761178752 and ci runs 40761178760 and 40762900849 are green for their heads; the eight Phase 2 sections equal 02-22's from line 5 on; ten distinct targets have `twice=identical` rows.
