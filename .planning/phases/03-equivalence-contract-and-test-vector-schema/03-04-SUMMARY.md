---
phase: 03-equivalence-contract-and-test-vector-schema
plan: 04
subsystem: testing
tags: [adapters, manifest-roles, generated-driver, captured-objects, qemu-user, test-vectors, globals, corpus, github-actions]

requires:
  - phase: 03-equivalence-contract-and-test-vector-schema
    provides: plan 03-01's executor core (runtime template, link recipe, run_driver, classify, Budgets), plan 03-02's policy loader and contract text, plan 03-03's Vector, Provenance and write_corpus/read_corpus
  - phase: 02-test-bed-and-build-capture
    provides: capture.json, canonical compile_commands.json, replay reports, the committed seeds and the probe builder (unit_include_prefix, probe_source_with_prefix)
provides:
  - "mt_capture::manifest: ArgRole, Role (in_buf, in_len, out_buf, out_cap, scalar), Entry.arg, split_parameters, MAX_SCALAR_COMBOS, MAX_OUT_CAP, RESERVED_ARG_NAMES and the role validator; manifest schema_version stays 1"
  - "crates/mt-harness: adapter (Adapter, combinations, driver_flags), render_entry_driver, link (verify_objects, archive, link_driver, unresolved_symbols), globals (writable_globals, locate_globals, DumpRange), run (Scope, build_entries, run_entries, run_vector_twice, list_adapters), replay (check_corpora, replay_corpus)"
  - "mt vectors adapters, build, run, replay and check"
  - "reference vectors for corejson JSON_Validate, lz4 LZ4_decompress_safe and libfdt fdt_check_full in both configurations of each, run twice and rerun in a second container, byte-identical (testbed run 40541809803)"
  - "testbed.yml: run_guest and the steps Build the reference drivers, Run the reference vectors twice, Run the vectors again and compare; notices vectors-build, vectors-run, vectors-determinism"
affects: [03-05, 03-06, 03-07, 03-08, phase 4 relabelling, phase 5 comparators, phase 11 evidence bundle]

requirements-completed: [EQV-03, EQV-04]

status: complete
commits: 9
plan_head_before: 98afa0cabcb3a1fb408cffdd87abc43a128d376d
plan_head_after: 62038441f9d7ef9ba27c672d710b182ec959896e
actuals:
  tokens: 64715
  tasks: 3
  commits: 9

tech-stack:
  added: []
  patterns:
    - "manifest roles are a closed grammar (enum, C identifiers, decimal integers); the only manifest text in generated C is the entry name, the closed-charset parameter types and Phase 2's validated include lines, pinned by golden drivers"
    - "the exact-objects property: every object's sha256 is recomputed against capture.json, the capture verdict must be captured, symlinks and escaping paths are refused, and the archive is `ar qcsD` over the sorted members"
    - "a run is two runs: every (seed, scalar combination) goes through the driver twice and the two vectors must be byte-identical or the entry stores nothing"
    - "captured object code runs only in run_guest: /work read-only, one writable output directory, /tmp, no network"

key-files:
  created:
    - crates/mt-harness/src/adapter.rs
    - crates/mt-harness/src/link.rs
    - crates/mt-harness/src/run.rs
    - crates/mt-harness/src/globals.rs
    - crates/mt-harness/src/replay.rs
    - crates/mt-capture/tests/eqv_03_roles.rs
    - crates/mt-harness/tests/eqv_03_adapter.rs
    - crates/mt-harness/tests/eqv_03_driver_golden.rs
    - crates/mt-harness/tests/eqv_04_exec.rs
    - crates/mt-harness/tests/eqv_04_link.rs
    - crates/mt-harness/tests/eqv_04_globals.rs
    - crates/mt-harness/tests/eqv_04_ties.rs
    - crates/mt-harness/tests/fixtures/golden/corejson-JSON_Validate.driver.c
    - crates/mt-harness/tests/fixtures/golden/lz4-LZ4_decompress_safe.driver.c
    - crates/mt-harness/tests/fixtures/obj/globals.c
    - crates/mt-harness/tests/fixtures/obj/globals.o
    - crates/mt-cli/tests/eqv_04_vectors_cli.rs
    - .planning/phases/03-equivalence-contract-and-test-vector-schema/03-04-CI-NOTICES.md
    - .planning/phases/03-equivalence-contract-and-test-vector-schema/03-04-VECTORS-NOTICES.md
  modified:
    - crates/mt-capture/src/manifest.rs
    - testbed/manifest.toml
    - crates/mt-harness/Cargo.toml
    - crates/mt-harness/src/lib.rs
    - crates/mt-harness/src/runtime.c
    - crates/mt-harness/src/runtime.rs
    - crates/mt-harness/src/driver.rs
    - crates/mt-harness/src/exec.rs
    - crates/mt-harness/tests/eqv_04_workflow_static.rs
    - crates/mt-capture/tests/build_02_workflow_static.rs
    - crates/mt-capture/tests/bed_01_entries.rs
    - crates/mt-capture/tests/bed_01_entry_include_prefix.rs
    - crates/mt-cli/src/main.rs
    - .github/workflows/testbed.yml
    - Cargo.lock

key-decisions:
  - "Manifest shape as planned: [[target.X.entry.arg]] with role, name, default, sweep, max (deny_unknown_fields); entries without arg tables validate exactly as before and serialize unchanged. The out_cap is recorded under the name out_cap unless it is named, so a scalar called out_cap is a duplicate. A sweep may not repeat the default or itself. out_cap values are limited to 0..=65536 (MAX_OUT_CAP, a constant of mt-capture tied to Budgets::DEFAULT.max_input by a test in mt-harness)"
  - "D-05 as the locked text says it: argv[0] is <target>/<config>/<entry> and argv[1..] are every non-pointer parameter in parameter order (the input length included, equal to the stdin byte count), so lz4 vectors read [entry, in_len, out_cap]"
  - "Request: fields 0 selector (unused by entries), 1 input, 2 output capacity, 3 scalar block (8 bytes each, two's complement), 4 dump plan (address and size pairs); a request has 3 fields (self-test, unchanged) or 5. Result frame regions: 1 the whole output buffer, 0x1000+i dump range i before, 0x2000+i after; the driver streams the frame in pieces from its buffers"
  - "Closed placeholder set grows from 6 to 8 (MT_INCLUDES, MT_CALL); a fragment that contains an @MT_ token is refused. render_entry_driver returns Result and accepts only single #include, #define or #undef lines with an operand (tighter than the Phase 2 probe rule, which it reuses to produce them)"
  - "Return conversion: integer or enum cast to int32_t, void is 0, pointer is 0 or 1; float, double, 64-bit integers and struct returns are UnsupportedReturn, never truncated"
  - "driver_flags fails closed: any option that is neither kept (-I, -isystem, -iquote, -include, -D, -U with operands; -fshort-enums and its negation, -funsigned-char, -fsigned-char, -fshort-wchar, -fpack-struct*) nor dropped (the plan's list, plus every -M* option and the manifest baseline flags the driver re-adds) is unhandled_flag naming it, including -f, -m and any other dashed option"
  - "D-10 rule: a writable global counts as changed when its bytes before or after the call differ from the image's initial bytes; without a typed layout that is executor_error untyped_global_changed naming the global and the seed. An unchanged global is snapshotted and not recorded, so lib_state_in and lib_state_out are {} for all three entries"
  - "Local symbols join the linked image through the STT_FILE name before them; a (file, name) claimed by more than one object, or found twice in the image, is an ambiguous join and an error"
  - "build.json (BuildRecord) holds object, archive, driver source and ELF hashes, the image, the replay state, the include lines, the driver flags, the run record of ar/compile/link and the dump plan; run recomputes the driver ELF hash against it before any run, and replay compares each provenance record's hashes with it"
  - "lz4 capacity sweep kept as planned: default 256, then 16 and 4096 (3 combinations per seed, 60 vectors per lz4 pair)"

patterns-established:
  - "Scope (manifest, work dir, pairs, entry filter, build root) is the one selection every build, run and replay takes; the CLI makes it from the same pairs `mt testbed pairs` prints"
  - "Golden drivers: MT_BLESS_GOLDEN=1 rewrites them; the diff is the review of a template or role change"
  - "Notices are rebuilt from the job annotations by one script and compared with the previous plan's file, so the eight Phase 2 sections are known to be byte-equal"

coverage:
  - id: D1
    description: "The manifest argument roles: ArgRole/Role/Entry.arg, split_parameters and the validator (unknown role, parameter count, pointer shape, in_buf/in_len and out_buf/out_cap pairing, narrow-length bound, scalar name and default, reserved and duplicate names, sweep repeats, the combination cap), with schema_version still 1 and Phase 2 manifests unchanged"
    requirement: EQV-03
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/eqv_03_roles.rs#eqv_03_roles_accept_the_committed_manifest"
        status: pass
      - kind: unit
        ref: "crates/mt-capture/tests/eqv_03_roles.rs#eqv_03_roles_reject_too_many_combinations"
        status: pass
      - kind: unit
        ref: "crates/mt-capture/tests/eqv_03_roles.rs#eqv_03_split_parameters"
        status: pass
    human_judgment: false
  - id: D2
    description: "The adapter and the generated driver: call expression, argv, scalar combinations in D-14 order, return kinds, fail-closed driver flags, goldens that pin the two generated drivers, and no manifest text reaching C unchecked"
    requirement: EQV-03
    verification:
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_03_adapter.rs#eqv_03_adapter_maps_corejson_validate"
        status: pass
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_03_adapter.rs#eqv_03_driver_flags_fail_closed_on_an_unhandled_option"
        status: pass
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_03_driver_golden.rs#eqv_03_driver_golden"
        status: pass
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_03_driver_golden.rs#eqv_03_driver_names_never_reach_rendering"
        status: pass
    human_judgment: false
  - id: D3
    description: "The exact-objects property: verify_objects refuses a changed, uncaptured, symlinked or escaping object; the archive is deterministic and keeps members with the same base name; unresolved symbols are parsed from the linker"
    requirement: EQV-04
    verification:
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_04_link.rs#eqv_04_refuses_modified_object"
        status: pass
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_04_link.rs#eqv_04_refuses_a_symlinked_object"
        status: pass
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_04_link.rs#eqv_04_archive_keeps_members_with_the_same_name"
        status: pass
    human_judgment: false
  - id: D4
    description: "Seeds of corejson JSON_Validate, lz4 LZ4_decompress_safe and libfdt fdt_check_full become schema-valid vectors from the exact captured objects in both configurations of each, every vector run twice byte-identically, with the policy loaded and full provenance; a second full run gives an identical corpus, locally and in the pinned image"
    requirement: EQV-04
    verification:
      - kind: other
        ref: "MT_LOCAL_CAPTURES=.mt-work/capture cargo test -p mt-harness --test eqv_04_exec -- --ignored (corejson 108 x2, lz4 60 x2, libfdt 16 x2 identical)"
        status: pass
      - kind: other
        ref: "testbed run 40541809803 (68852da): vectors-build 6 built, vectors-run 6 rows twice=identical, vectors-determinism identical 374 files; 03-04-VECTORS-NOTICES.md"
        status: pass
    human_judgment: false
  - id: D5
    description: "The D-10 globals snapshot: writable_globals over .data and .bss, the join with the linked image, an unchanged global snapshotted and not recorded, a changed untyped global failing the run with untyped_global_changed, an ambiguous local join refused"
    requirement: EQV-04
    verification:
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_04_link.rs#eqv_04_globals_are_found_in_data_and_bss"
        status: pass
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_04_globals.rs#eqv_04_changed_global_is_untyped_global_changed"
        status: pass
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_04_globals.rs#eqv_04_ambiguous_local_global_join_is_an_error"
        status: pass
    human_judgment: false
  - id: D6
    description: "Stored vectors can be re-run and compared: replay_corpus reproduces a corpus, reports a one-byte edit as differing and a provenance record that names another driver as provenance_mismatch; check_corpora and mt vectors check fail closed"
    requirement: EQV-04
    verification:
      - kind: other
        ref: "crates/mt-harness/tests/eqv_04_exec.rs#eqv_04_replay_reproduces_stored_vectors (ignored, local captures)"
        status: pass
      - kind: integration
        ref: "crates/mt-cli/tests/eqv_04_vectors_cli.rs#eqv_04_check_cli_fails_closed"
        status: pass
    human_judgment: false
  - id: D7
    description: "Captured object code runs only in run_guest, which cannot write the evidence, and the three CI steps run in order after the replay step"
    requirement: EQV-04
    verification:
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_04_workflow_static.rs#eqv_04_workflow_static_run_guest_cannot_write_the_evidence"
        status: pass
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_04_workflow_static.rs#eqv_04_workflow_static_captured_code_runs_only_in_run_guest"
        status: pass
    human_judgment: false
  - id: D8
    description: "What the stored vectors record at the observation point: the whole output buffer (so lz4's few bytes written past the returned length and its partial writes on error paths are part of the reference), empty stdout for a fault, abort or timeout, argv with the input length, and no lib_state for these three entries"
    requirement: EQV-03
    verification: []
    human_judgment: true
    rationale: "The tests prove the executor records what the C does; whether a Rust translation must reproduce bytes past the returned length (D-07 read literally) or the contract should compare only the produced prefix is a founder reading, found while running lz4 (see Issues Encountered)"

duration: "about 55 min of wall clock by the system clock (2026-10-27T15:57:15Z to 16:50Z), including one CI round trip of about 14 min"
completed: 2026-10-27
---

# Phase 3 Plan 04: Manifest roles, generated drivers over the exact captured objects, and the first reference vectors Summary

**The manifest now declares argument roles per entry (`[[target.X.entry.arg]]`, validated, schema version still 1), `mt vectors build|run|replay|check|adapters` generate each driver from the shared template, link it with exactly the objects Phase 2 captured and replayed (sha256 recomputed), run the committed seeds through qemu-arm twice and store sha256-named vectors with provenance, and testbed run 40541809803 reproduced corejson, lz4 and libfdt (both configurations each, 368 vectors) byte for byte in the pinned image on the first round trip.**

## Performance

- **Duration:** about 55 min of wall clock by the system clock (15:57:15Z to 16:50Z), of which one CI round trip took about 14 min.
- **Tasks:** 3 of 3 (Tracer, Task 2, Task 3), each RED then GREEN, and a second Tracer RED.
- **Files:** 32 changed outside `.planning` against `98afa0c` (7228 insertions, 47 deletions), plus the two notices files and this summary.
- **Pushes and CI:** 1 round trip of the 3 allowed (testbed 40541809803 and ci 40541810199 green on `68852da`); the docs-only notices commit `6203844` was pushed with ci 40543597788 green.

## Accomplishments

- **Manifest roles (EQV-03, D-11, D-14, D-29):** `Role` (in_buf, in_len, out_buf, out_cap, scalar), `ArgRole`, `Entry.arg`, `split_parameters` and the validator. It rejects an unknown role (at parse), a parameter count that differs from the signature, a pointer role on a value and the reverse, a missing or extra length or capacity, a missing `max` on a length narrower than 32 bits, a scalar without name or default, a name that is not a C identifier, reserved or repeated, a sweep that repeats a value, an out_cap outside 0..=65536 and more than 16 combinations. corejson, lz4 and libfdt carry roles with D-11/D-07/D-14 comments.
- **Adapter and driver (D-01, D-05, D-11):** `Adapter::from_entry` turns roles into the call expression, argv, the request words and the combinations (default first, then the sweep, first parameter slowest); `render_entry_driver` fills the template's `@MT_INCLUDES@` and `@MT_CALL@`; two goldens pin the generated drivers.
- **Exact objects (D-19, T-03-16):** `verify_objects`, `archive` (`ar qcsD`, sorted members), `link_driver` (compile with the baseline flags, `-std=gnu11` and the defining unit's include, define and ABI flags; link with `-nostartfiles --specs=nosys.specs ... -lm`) and an unresolved-symbol parse. `build.json` records every hash and the dump plan.
- **Vectors (D-02, D-05, D-07, D-20, D-22, D-26):** `run_entries` loads and validates the target's policy, selects the seeds whose index lists the entry, runs every (seed, combination) twice through `run_vector_twice` (executor error, `untyped_global_changed` or a difference fails the entry and stores nothing) and writes the corpus once with provenance (object, archive, driver source and ELF hashes, image, versions, policy sha256, budgets, outcome, `has_ub_source`, replay state). Local and CI results: corejson 108 vectors per configuration, lz4 60 (20 seeds x 3 capacities), libfdt 16, all `ok` and `twice=identical`.
- **Globals (D-10):** `writable_globals` and `locate_globals` find `.data`/`.bss` symbols in the objects and join them to the linked image (locals through `STT_FILE`); the driver snapshots each range before and after the call and the host applies the `untyped_global_changed` rule.
- **Replay and comparison (D-21, T-03-17):** `replay_corpus` re-runs each stored vector and compares the canonical bytes and the provenance hashes; `check_corpora` compares trees file by file and fails closed.
- **CI (D-21, T-03-15):** `run_guest` and the steps "Build the reference drivers", "Run the reference vectors twice" and "Run the vectors again and compare (vectors determinism)"; `vectors-determinism` reported `identical 374 files` across two separate containers.

## Task Commits

1. **Tracer** (`tdd`): `153b4cd` test (RED), `18216ae` test (RED 2), `5801395` feat (GREEN), `d9ba823` style (rustfmt of the RED 2 test file)
2. **Task 2** (`tdd`): `d5982b2` test (RED), `04279c6` feat (GREEN)
3. **Task 3** (`tdd`): `a1c0e08` test (RED), `68852da` feat (GREEN, pushed; round trip 1)
4. **Notices:** `6203844` docs (round trip 1 notices, pushed)

No REFACTOR commits were needed. `commits: 9` is the measured `git rev-list --count 98afa0c..HEAD` when this summary was written (the eight code and test commits and the notices commit); the summary, STATE and ROADMAP commits follow it.

## CI round trips (1 of 3 used)

| Round trip | Head | testbed | ci | Conclusion |
|------------|------|---------|----|------------|
| 1 | `68852da` | 40541809803 (job 122322581335) success | 40541810199 success | green; container run 40541810001 was not awaited |

- `vectors-build`: six rows `built driver_sha256=...` (corejson default and depth-8, libfdt default and assume-valid, lz4 default and heap-small) and 45 `no_roles` rows; no `unhandled_flag`, no `link_unresolved`, no `refused`.
- `vectors-run`: six rows, `vectors=108/108/16/16/60/60`, every one `ok=<n> fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical`.
- `vectors-determinism`: `rerun exit code: 0`, rerun rows identical to the first run's, `identical 374 files`, `check exit code: 0`.
- `vectors-selftest` (`result: passed (17 rows)`) and `vectors-tractor` (`loaded: 6 of 6`) still pass; the eight Phase 2 sections of `03-04-CI-NOTICES.md` equal `02-22-CI-NOTICES.md` from line 5 on (`diff` empty); the three Task 3 automated verifies exit 0, the third on `6203844` after ci 40543597788.
- The pre-decided handling for the CI cases (object check, unhandled flag, unresolved symbol, differing runs, untyped global) was not triggered.

## TDD evidence (RED output, with a one-line assessment per failure)

**Tracer RED (`153b4cd`)**

```
error[E0432]: unresolved imports `mt_capture::manifest::ArgRole`, `mt_capture::manifest::Role`, `mt_capture::manifest::split_parameters`
error[E0609]: no field `arg` on type `&mut mt_capture::manifest::Entry`  (x3)
error[E0432]: unresolved import `mt_harness::replay` / `mt_harness::run`          (eqv_04_exec)
test eqv_04_adapters_cli_lists_entries ... FAILED   stderr: error: unrecognized subcommand 'adapters'
test eqv_04_adapters_cli_needs_a_manifest ... FAILED   the refusal must name the missing option
```

Assessment: right reason for each; the manifest and harness tests fail to compile because the items do not exist, the CLI tests fail on the absent subcommand. The "needs a manifest" test asserts the message names `--manifest`, so clap's exit code 2 alone could not make it pass by accident.

**Tracer RED 2 (`18216ae`)**: `error[E0432]: unresolved import mt_harness::adapter` (eqv_03_adapter) and `mt_harness::link` (eqv_04_link). Assessment: right reason. These tests cover code the Tracer GREEN needed (flag classification, object verification, linker output) that the plan listed no test for.

**Task 2 RED (`d5982b2`)**

```
eqv_03_roles: 6 FAILED (accept_the_committed_manifest: lz4 and libfdt roles absent; narrow length, scalar default, reserved/duplicate names, too many combinations, out_buf without capacity: "the manifest must be invalid, got Ok")
eqv_03_driver_golden: eqv_03_driver_golden FAILED (called Result::unwrap() on Err(NoRoles))
                      eqv_03_driver_names_never_reach_rendering FAILED ("#define" must be refused)
error[E0432]: unresolved import mt_harness::globals     (eqv_04_link, eqv_04_globals)
error[E0432]: unresolved imports PreparedDriver, run_vector_twice; error[E0603]: struct Observed is private
error[E0432]: unresolved import MAX_OUT_CAP; error[E0603]: function budgets_record is private   (eqv_04_ties)
error[E0432]: unresolved imports ReplayOptions, replay_corpus                                    (eqv_04_exec)
test eqv_04_check_cli_fails_closed ... FAILED   error: unrecognized subcommand 'check'
```

Assessment: right reason; the validator and golden failures are assertions against the Tracer implementation, the others name the missing items. `check_corpora` was not among the missing items because the Tracer test needs it (see deviation 3).

**Task 3 RED (`a1c0e08`)**: `eqv_04_workflow_static_run_guest_cannot_write_the_evidence` FAILED (`testbed.yml defines no run_guest() {`), `..._vectors_steps_run_in_order` FAILED (`has no step "Build the reference drivers"`), `..._captured_code_runs_only_in_run_guest` FAILED (`run_guest is called twice` ... `left: 0`). Assessment: right reason, all assertions; the `build_02_workflow_static` helper-list change passes on its own (12 tests).

GREEN: every named test passes; the four pre-merge gates exit 0 on the pushed head (`cargo test --workspace --locked`: 472 passed, 0 failed, 8 ignored, no `skipping:` line), and the ignored local proofs pass with `MT_LOCAL_CAPTURES`.

## Tracer feedback gate

Auto mode was off and `human_verify_mode` was the default `end-of-phase`; the tracer's `<verify>` is automated-only, so it was re-run after the Tracer GREEN commit (the six role tests, the CLI test, the ignored corejson proof printing `vectors=108 identical`, the four gates) and logged `Tracer verified end-to-end - expanding`. No checkpoint was synthesized.

## Deviations from Plan

**1. [Rule 3 - Blocking] Phase 2 test literals of `Entry` gained `arg: Vec::new()`.** The new public field makes three struct literals in `bed_01_entries.rs` (two) and `bed_01_entry_include_prefix.rs` (one) fail to compile. One line each; no assertion changed; all Phase 2 tests pass unchanged otherwise. Committed in `5801395`.

**2. [Minor] A second Tracer RED (`18216ae`).** The plan listed no test for `driver_flags`, `Adapter` and `verify_objects`/`unresolved_symbols`, yet the Tracer GREEN needs them, and the tests for `verify_objects` sit in Task 2's file list. They were written before the code, so the Tracer GREEN has a RED for everything it adds.

**3. [Minor] `check_corpora` (and `replay.rs`) arrived in the Tracer.** The Tracer's own test calls `check_corpora`, so it could not wait for Task 2. Task 2's RED therefore had `mt vectors check` and `replay_corpus` as its failures, not `check_corpora`.

**4. [Minor] `Request::entry` and `with_dump_plan` constructors, not new fields.** The self-test builds `Request { fields }` directly; a constructor keeps `selftest.rs` and its tests untouched. The template accepts 3 fields (self-test) or 5 (entry); `Placeholders` gains `includes` and `call`, filled by `from_budgets` with the self-test's values and by `for_entry`.

**5. [Minor] `render_entry_driver` returns `Result`.** The plan said `String`; rendering can fail (a bad include line, a leftover token), so it returns `Result<String, HarnessError>`. Its include-line rule is tighter than Phase 2's probe check (a space and an operand are required).

**6. [Interpretation] The goldens were pinned to the Tracer template in RED and re-blessed in GREEN.** The template changed in Task 2 (dump plan, streamed frame), so the golden diff (127 lines each) is part of the GREEN commit, with the lz4 roles needed to produce the second golden applied temporarily and reverted before the RED commit.

**7. [Measured, plan expectation wrong] LZ4 writes past the length it returns.** The plan expected a short output to leave the tail at 0xA5. Measured: `LZ4_decompress_safe` writes up to 3 bytes beyond its return value on success and leaves partial output on error paths. `eqv_04_sentinel_prefill_recorded` therefore asserts the tail beyond a 16-byte margin, the prefix equality across capacities, `cut_short=4` seeds and `partial_writes=15` records. See Issues Encountered.

**8. [Minor] Test sources use `static volatile`.** GCC at `-Os` folds a read-only `static int` into a constant and drops it from `.data`, so the globals tests use `static volatile` to keep the symbol.

**9. [Minor] `run_vector_twice` takes an `Exec` struct (tools, runner config, budgets).** Eight arguments trip `clippy::too_many_arguments`; the call shape of `eqv_04_globals.rs` was adjusted in the GREEN commit, nothing it asserts changed.

**10. [Minor] `finish_vectors STEP RC [STEM]`.** The Phase 2 steps already wrote `$out/build.txt` and `$out/determinism.txt`; the new steps write `vectors-build.txt`, `vectors-run.txt` and `vectors-determinism.txt` instead of overwriting them, so `finish_vectors` takes the file stem as an optional third argument. The rerun is inside the third step, which also runs `check`.

**11. [Process] The disk filled during Task 2.** The gitignored `target/debug` (12 GB of build output) was removed and rebuilt; local cargo runs used `CARGO_PROFILE_DEV_DEBUG=0`. No tracked file was affected.

**12. [Process] The Task 2 GREEN commit was amended once before any push** to add `testbed/manifest.toml`, which the first staging missed; the tests had passed against the working tree.

**Total deviations:** 12 (1 Rule 3, 1 measured expectation, 10 minor, interpretive or process). **Impact:** none on the plan's goal; one of three round trips used.

## Decisions Made

See `key-decisions` in the frontmatter. For the founder to read at `/gsd-verify-work`: D-05 including the input length in argv (the locked text, against research's suggestion), the changed-global criterion (before or after differs from the image), the empty `stdout` of a fault, abort or timeout vector, and the fail-closed breadth of `driver_flags` (every unclassified dashed option is `unhandled_flag`).

## Known Stubs

None. `Provenance.unobserved` is 0 for all three entries because nothing is marked unobserved yet; `lib_state_in` and `lib_state_out` are `{}` because none of the three entries has state or a changed global.

## Threat Flags

None beyond the plan's register. Mitigations exercised: T-03-14 by `eqv_03_roles_*` and `eqv_03_driver_*` (names, types and include lines are closed grammars; the goldens pin the output); T-03-15 by the two `run_guest` static tests and the second-container comparison; T-03-16 by `eqv_04_refuses_*` and the `--replay-dir` requirement; T-03-17 by `eqv_04_replay_reproduces_stored_vectors`; T-03-18 by `eqv_03_roles_reject_too_many_combinations`; T-03-SC: no external crate added (`Cargo.lock` gained only path edges, `object` was already locked).

## Issues Encountered

- **Finding for the equivalence contract (D-07):** lz4's decoder copies in wide steps, so on success it can write a few bytes past the returned length (measured: at most 3, in 10-byte and 13-byte blocks), and on error paths it leaves part of the output (15 of the 60 recorded buffers hold a partial write). Because the contract compares the whole output buffer, a Rust translation would have to reproduce those bytes. The vectors record them faithfully; whether the comparison should instead cover only the produced prefix is the founder's reading (coverage item D8).
- A fault, abort or timeout vector has an empty `stdout`: the driver exits from its signal handler without a frame, so there is no "after" buffer to record. Plan 03-05 may revisit this if the contract needs the buffer at the fault.
- The ignored local proofs need `MT_LOCAL_CAPTURES`; they ran on this host with QEMU 8.2.2 and GCC 13.2.1 and in CI on QEMU 10.0.2 and GCC 14.3 with the same row counts.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plans 03-05 to 03-08 only add roles: `Role` is a closed enum a later plan extends, `lib_state` typing plugs in where `check_globals` raises `untyped_global_changed`, and `Request`/the frame already carry a dump plan and regions.
- The testbed workflow now publishes the vectors titles; the 03-04 files in `.planning/phases/03-.../` are the layout the next plan copies.
- Not pushed: this summary, STATE.md and ROADMAP.md commits follow the pushed notices commit and ride with the next plan's first push.

## Self-Check: PASSED

- Created files found on disk: `adapter.rs`, `link.rs`, `run.rs`, `globals.rs`, `replay.rs`, the seven test files, both goldens, `globals.c`/`globals.o`, both notices files.
- Commits found as ancestors of HEAD: 153b4cd, 18216ae, 5801395, d9ba823, d5982b2, 04279c6, a1c0e08, 68852da, 6203844.
- Plan-level verification: pre-merge gates exit 0; `Cargo.lock` gained no external crate; no forbidden claim word in `crates/mt-harness/src`, `manifest.rs` or the vectors notices; Phase 2 tests pass unchanged; the cited testbed run 40541809803 and ci runs 40541810199 and 40543597788 are green for their heads.
