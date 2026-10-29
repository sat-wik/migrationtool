---
phase: 03-equivalence-contract-and-test-vector-schema
plan: 08
subsystem: testing
tags: [adapters, bounded-attempts, drop-log, committed-sample, replay-tree, github-actions, second-job]

requires:
  - phase: 03-equivalence-contract-and-test-vector-schema
    provides: plan 03-04..03-07 roles, setup sequences, locals, wrap, state arrays, the adapter drop log and the floor of ten; plan 03-03 corpus and provenance; plan 03-02 policies; plan 03-01 and 03-10 reference executor in the D-31 image
provides:
  - "mt_capture::manifest: Role::{Null, Local}, ArgRole.local, inout_len of = \"in\", SetupArg::SizeOf, SetValue::InByte; manifest schema_version stays 1"
  - "mt_harness: sample::{select_sample, write_sample, load_inputs, SampleChoice}, replay::{corpus_dirs_below, replay_tree}; Adapter::scalar_words takes the input length"
  - "CLI: mt vectors sample --corpus-dir --out [--manifest] [IDS]; mt vectors replay --tree"
  - "testbed.yml: a second job, vectors (D-32), with the self-test, TRACTOR, build, layouts, twice-run vectors, rerun comparison, sample fixture and committed-sample replay; the testbed job keeps its eight Phase 2 steps byte-identical"
  - "drivable entries: mbedtls mbedtls_x509_crt_parse_der, coreMQTT MQTT_DeserializePublish and MQTT_DeserializeAck, lz4 LZ4F_getFrameInfo; 15 drivable targets; every one of the 27 entries drivable or logged"
  - "testbed/vectors-sample: 15 one-vector corpora made by CI in the pinned image, replayed identical in the final run"
affects: [phase 3 verification, phase 4 relabelling, phase 5 comparators, phase 11 evidence bundle]

requirements-completed: [EQV-03, EQV-04]

status: complete
commits: 12
plan_head_before: ef0bdbdbd0cc8916de82ee0f554f83a79322a6d5
plan_head_after: ffa622acbb2204d45d1353de84a628e0ae0e6b0a
actuals:
  tokens: 43400
  tasks: 3
  commits: 12

tech-stack:
  added: []
  patterns:
    - "a long CI evidence set that exceeds a job's 50 annotations moves to a second job in the same pinned image; the captures travel as one artifact of two tars made by an image tool, and the second job reads only that"
    - "a committed sample crosses from the pinned image to the repository only as a deterministic tar in a fixture notice whose last line is the tar's sha256, checked before unpacking; the next run replays it byte for byte"
    - "an attempted entry ends in identical vectors twice or in one drop row with cause, detail and the attempt made; a link that cannot succeed is logged, not declared, because a declared entry that does not link fails the whole driver-build step"

key-files:
  created:
    - crates/mt-harness/src/sample.rs
    - crates/mt-harness/tests/eqv_04_committed_sample.rs
    - testbed/vectors-sample/NOTICE
    - "testbed/vectors-sample/<target>/default/<entry>/{<sha256>.json,index.json} (15 corpora)"
    - .planning/phases/03-equivalence-contract-and-test-vector-schema/03-08-CI-NOTICES.md
    - .planning/phases/03-equivalence-contract-and-test-vector-schema/03-08-VECTORS-NOTICES.md
  modified:
    - .github/workflows/testbed.yml
    - crates/mt-capture/src/manifest.rs
    - crates/mt-capture/tests/eqv_03_roles.rs
    - crates/mt-capture/tests/build_02_workflow_static.rs
    - crates/mt-harness/src/adapter.rs
    - crates/mt-harness/src/replay.rs
    - crates/mt-harness/src/run.rs
    - crates/mt-harness/src/lib.rs
    - crates/mt-harness/tests/eqv_03_adapter.rs
    - crates/mt-harness/tests/eqv_03_adapter_floor.rs
    - crates/mt-harness/tests/eqv_04_exec.rs
    - crates/mt-harness/tests/eqv_04_workflow_static.rs
    - crates/mt-cli/src/main.rs
    - crates/mt-cli/tests/eqv_04_vectors_cli.rs
    - testbed/manifest.toml
    - testbed/adapter-dropped.toml
    - testbed/README.md
    - .planning/phases/03-equivalence-contract-and-test-vector-schema/03-CONTEXT.md

key-decisions:
  - "D-32 (founder reply \"Second job (Recommended)\"): the Phase 3 vectors steps run in job vectors of testbed.yml with its own 50-annotation budget; the testbed job and its eight Phase 2 steps are byte-identical (a test pins the sha256 of each step's text); vectors-* notices are read from the vectors job's id"
  - "the captures reach the vectors job as the artifact vectors-input (work.tar of capture and run, replay.tar of the replay reports, both made by image_tool so the container user's files are read by their owner); only the unpacked trees are chmod'ed, because the runner cannot chmod the earlier containers' files"
  - "an attempted entry that cannot link is logged link_unresolved with the unresolved names instead of declared: nimble ble_hs_adv_parse_fields (five names, measured locally) and isotp isotp_on_can_message (three user hooks, derived locally from nm -u of isotp.c)"
  - "the sample rule is fixed: default configuration, first drivable entry by name, vector of the first scalar combination whose seed is smallest (ties by seed sha256, then vector sha256); select_sample fails closed when a drivable target has no corpus and write_sample writes nothing then"
  - "the committed-sample test requires the stdin of every sample vector to equal its committed seed file byte for byte and to be the smallest seed that lists the entry"

patterns-established:
  - "A Phase 2 step is protected by the sha256 of its text in a static test, so a move of the neighbouring steps proves it changed nothing"
  - "A form added to the closed grammar has a validator rule in the manifest, a second check in the adapter and a rendering test; the adapter refuses what the validator refuses"

coverage:
  - id: D1
    description: "Job vectors: the Phase 3 steps moved out of the testbed job (D-32) with the same image, the same hardening and an artifact hand-off; the eight Phase 2 notices equal 02-22-CI-NOTICES.md from line 5 on"
    requirement: EQV-04
    verification:
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_04_workflow_static.rs#eqv_04_workflow_static_phase_2_steps_are_unchanged"
        status: pass
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_04_workflow_static.rs#eqv_04_workflow_static_vectors_job_is_separate_and_equally_hardened"
        status: pass
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_04_workflow_static.rs#eqv_04_workflow_static_captures_travel_by_artifact"
        status: pass
      - kind: other
        ref: "testbed run 40922236736 (5e0d4a8): testbed job 123619075984 and vectors job 123623035539 success; 03-08-CI-NOTICES.md diffed against 02-22-CI-NOTICES.md from line 5: empty"
        status: pass
    human_judgment: false
  - id: D2
    description: "The additive forms null, local, inout_len of in, sizeof of a local and $in[n], validated in the manifest and again in the adapter"
    requirement: EQV-03
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/eqv_03_roles.rs#eqv_03_roles_tier_c_forms"
        status: pass
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_03_adapter.rs#eqv_03_adapter_tier_c_forms_render"
        status: pass
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_03_adapter.rs#eqv_03_adapter_tier_c_forms_are_refused_when_they_do_not_fit"
        status: pass
    human_judgment: false
  - id: D3
    description: "The eight remaining entries each attempted once: mbedtls x509, coreMQTT publish and ack and lz4 LZ4F_getFrameInfo drivable and identical twice in the image; isotp and nimble adv logged link_unresolved, zlib inflate and nanopb pb_decode_ex logged not_expressible"
    requirement: EQV-03
    verification:
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_03_adapter_floor.rs#eqv_03_every_entry_is_drivable_or_logged"
        status: pass
      - kind: other
        ref: "MT_LOCAL_CAPTURES=.mt-work/capture cargo test -p mt-harness --test eqv_04_exec -- --ignored eqv_04_tier_c_attempts (mbedtls 200 and 200, coreMQTT publish 60 and 60, ack 15 and 15, lz4F 20 and 20 per configuration)"
        status: pass
      - kind: other
        ref: "testbed run 40922236736, vectors-run: mbedtls x509 200 per configuration, coreMQTT 60 and 15 per configuration, LZ4F_getFrameInfo 20 per configuration, all twice=identical"
        status: pass
    human_judgment: false
  - id: D4
    description: "The committed sample: 15 CI-produced one-vector corpora, valid offline, each input the smallest committed seed of its entry, replayed identical in the final run; the final run regenerates the same tar"
    requirement: EQV-04
    verification:
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_04_committed_sample.rs#eqv_04_committed_sample_is_valid"
        status: pass
      - kind: unit
        ref: "crates/mt-cli/tests/eqv_04_vectors_cli.rs#sample::eqv_04_sample_cli_selects_one_vector_per_target"
        status: pass
      - kind: other
        ref: "testbed run 40922236736: vectors-sample prints 15 corpora identical 1; its fixture vectors-sample tar sha256 ca5f0796913858d4b84dc53b16e5ea764a872ebcdb4ce0da570c381811b11dd1 equals run 40919765252's, and the rebuilt files equal testbed/vectors-sample"
        status: pass
    human_judgment: false
  - id: D5
    description: "What the sample, the drop rows and the unobserved opaque context mean for later phases (the sample's first-seed rule picks very short seeds; the LZ4F context is a heap object recorded as a pointer only)"
    requirement: EQV-04
    verification: []
    human_judgment: true
    rationale: "The tests prove the rule and the bytes; whether the smallest-seed sample is a good enough offline reference, and whether an opaque heap object may stay unrecorded, are Phase 4 and 5 readings"

duration: "1 h 30 min of wall clock (2026-10-29T16:16Z to 17:50Z), three CI round trips of about 8 to 15 min each"
completed: 2026-10-29
---

# Phase 3 Plan 08: Bounded attempts, the committed sample and the final run Summary

**Every one of the 27 manifest entries is now drivable or logged with its reason (15 drivable targets); a second CI job, `vectors`, carries the Phase 3 evidence under its own annotation budget; and 15 CI-produced reference vectors are committed and replay byte for byte, in the third and last allowed round trip.**

## Performance

- **Duration:** about 1 h 30 min of wall clock, from the root-pin guard to this summary.
- **Tasks:** 3 of 3 (Tracer, Task 2, Task 3), plus the founder-decided job split (D-32) done first, each as RED then GREEN where it carries code.
- **Files:** 22 changed outside `.planning` and the sample against `ef0bdbd` (3094 insertions, 178 deletions), 31 sample files, the two notices files and this summary.
- **Round trips: 3 of 3 used, no slack.** Round trip 1 failed on a bug of mine in the new job (below); round trips 2 and 3 are the two the plan expected.

## Accomplishments

- **Second job (D-32).** The self-test, the TRACTOR sample, the driver build, the layouts, the twice-run vectors, the rerun comparison and the two sample steps run in job `vectors`, in the same image by digest (it pulls it itself and publishes `vectors-image`), with the same helpers and the same hardening; `fetch_tractor` is still the only networked vectors helper. The `testbed` job lost only the vectors steps and the helpers they used; its eight Phase 2 steps are byte-identical (their text is pinned by sha256 in `eqv_04_workflow_static_phase_2_steps_are_unchanged`) and its notices equal `02-22-CI-NOTICES.md` from line 5 on in all three round trips. The testbed job now has 41 annotations, the vectors job 27, each of 50.
- **Five additive forms (D-13, D-24).** `null` (a pointer parameter passed as NULL), `local` with its key (the value of a declared pointer local), `inout_len` with `of = "in"` (initialised from the input length, no default or sweep, not a combination dimension, still listed in argv, the in_buf's length), the setup argument `sizeof(<local>)`, and the set value `$in[<n>]` (the input byte at n, 0 when shorter). Each has a manifest rule, a second check in the adapter and a rendering test; nothing else was added.
- **The eight attempts.**

  | Entry | Forms used | Outcome |
  |-------|-----------|---------|
  | mbedtls `mbedtls_x509_crt_parse_der` (Tracer) | state, in_buf, in_len, setup call `mbedtls_x509_crt_init(&crt)` (existing forms, data only) | 200 vectors per configuration, identical twice; parsed chain, heap parts through the heap walk (default 21 parse, 574 heap objects; suite-b 7 parse) |
  | coreMQTT `MQTT_DeserializePublish` | `null` property builder, set steps `$in[0]`, `$in+2`, `$len-2`, two swept scalars | 60 vectors per configuration (4 combinations of 15 seeds), identical twice |
  | coreMQTT `MQTT_DeserializeAck` | `null`, set steps, connection properties set to maxPacketSize 4096 and requestProblemInfo 1 | 15 vectors per configuration, 9 accepted, identical twice |
  | lz4 `LZ4F_getFrameInfo` | `local` pointer local from `LZ4F_createDecompressionContext(&dctx, 100)`, inout_len `of = "in"` | 20 vectors per configuration, 3 frame headers accepted, identical twice |
  | isotp `isotp_on_can_message` | designed, not declared | `link_unresolved`: `isotp_user_debug`, `isotp_user_get_ms`, `isotp_user_send_can` |
  | zlib `inflate` | none fits | `not_expressible`: string-literal setup argument |
  | nanopb `pb_decode_ex` | none fits | `not_expressible`: message descriptor absent from the captured objects |
  | nimble `ble_hs_adv_parse_fields` | state, in_buf, in_len (max 255), built once | `link_unresolved`: `ble_sm_alg_c1`, `ble_sm_alg_s1`, `ble_transport_ll_init`, `ble_transport_to_ll_acl_impl`, `ble_transport_to_ll_cmd_impl` (research Q3 again) |

- **Final adapter drop log: 7 rows.** The three `no_seeds` rows from plan 03-06 (zcbor `cbor_decode_Pet`, httpparser `http_parser_parse_url`, mbedtls `mbedtls_asn1_get_tag`) and the four above. **15 drivable targets**: cjson, corejson, coremqtt, httpparser, libfdt, lz4, mbedtls, microcoap, nanopb, nimble, paho, qcbor, tinycbor, zcbor, zlib; isotp is the only retained target left with every entry logged. `eqv_03_every_entry_is_drivable_or_logged` holds for all 27 entries.
- **Completeness and the sample.** `mt vectors sample` chooses and writes the sample, `mt vectors replay --tree` replays every corpus below a directory, and `testbed/vectors-sample` holds 15 one-vector corpora (default configuration, first drivable entry by name, smallest seed, first combination) with the CI provenance (`replay identical`, the pinned image digest, `has_ub` unchecked), plus a NOTICE. Their stdin is the committed seed file byte for byte.
- **Final run.** In round trip 3 (head `5e0d4a8`, the first run with a committed sample) the step "Replay the committed sample" printed all 15 corpora `identical 1`, and the same run's step "Fixture vectors sample" regenerated a tar with the same sha256 as round trip 2's, so the whole sample regenerates identically across CI runs.

## Task Commits

1. **D-32, the job split** (`tdd`): `58f2931` test (RED), `c6419e1` feat (GREEN)
2. **Tracer** (`tdd`): `073443e` test (RED), `28ab24c` feat (GREEN, data only)
3. **Task 2** (`tdd`): `5fb18da` test (RED), `de95de8` feat (GREEN)
4. **Task 3** (`tdd`): `088d6a9` test (RED), `feebc49` feat (GREEN, pushed: round trip 1), `654eb2c` test (RED of the fix), `c41d877` fix (pushed: round trip 2), `5e0d4a8` feat (the sample, pushed: round trip 3), `ffa622a` docs (round trip 3 notices, docs-only push)

`commits: 12` is the measured `git rev-list --count ef0bdbd..HEAD` at `ffa622a`; the summary, STATE, ROADMAP and requirements commits follow it. No REFACTOR commit was needed.

## CI round trips (3 of 3 used; the plan expected 2)

| Round trip | Head | testbed run (testbed job) | vectors job | ci | Conclusion |
|------------|------|---------------------------|-------------|----|------------|
| 1 | `feebc49` | 40917551324 (job 123602227074) success, 41 annotations | 123605905065 failure | 40917551264 success | not green: step "Unpack the captures of the testbed job" exited 1 before any vectors step ran; the self-test and TRACTOR steps of the new job passed and the eight Phase 2 notices already equalled 02-22 |
| 2 | `c41d877` | 40919765252 (job 123610036188) success | 123613902496 success, 27 annotations | 40919765255 success | green; the sample published (10 parts), `vectors-sample` said `no committed sample`; container run 40919765379 success |
| 3 (final) | `5e0d4a8` | 40922236736 (job 123619075984) success | 123623035539 success | 40922236801 success | green; sample replayed 15 identical; the docs-only push `ffa622a` started only ci |

- **Round trip 1 cause.** The unpack step ended with `chmod -R a+rX` over the whole work and out directories. The self-test and TRACTOR containers had already made files there (owned by the container user), which the runner may not chmod, so chmod exited 1. Fix `c41d877`: chmod only the unpacked `capture`, `run` and `replay` trees; `654eb2c` made a static test fail first. Log hosts are blocked, so the cause was read from the step order and the exit code, then confirmed by round trip 2.
- **Round trip 2 and 3 results (vectors job).** `vectors-build` 51 rows `result: passed`, `vectors-run` 38 target/configuration rows all `twice=identical` (51 rows with the `no_roles: 13` line), `vectors-determinism` `identical 5768 files`, `vectors-selftest` 17 rows, `vectors-tractor` 6 of 6 loaded, `vectors-layout` 6 parts, `vectors-sample` 15 identical (round trip 3). Every title is present; 15 distinct targets have `twice=identical` rows.
- **Sample transport.** The notice `fixture vectors-sample` came in 10 parts. I rebuilt the tar from the parts with a script, checked its `tar sha256` line (`ca5f0796913858d4b84dc53b16e5ea764a872ebcdb4ce0da570c381811b11dd1`), unpacked it into `testbed/vectors-sample`, and ran `eqv_04_committed_sample_is_valid` and `mt vectors validate` on it before committing. A local rehearsal of the same shell commands round-tripped a 14-target local sample first.

## TDD evidence (RED output, with a one-line assessment per failure)

**D-32 RED (`58f2931`)**
```
eqv_04_workflow_static: 9 of 11 FAILED  .github/workflows/testbed.yml has no job "vectors" under jobs:
build_02_workflow_is_least_privilege_and_scheduled: FAILED  expected the mt, testbed and vectors jobs
```
Assessment: right reason; the vectors-scoped checks cannot find the job. The two tests that pass unchanged are the Phase 2 pin test and the guest-call test, which read the whole file.

**Tracer RED (`073443e`)**
```
eqv_04_tier_c_attempts: FAILED  the driver must build: [BuildRow { target: "mbedtls", config: "default", entry: "mbedtls_x509_crt_parse_der", state: NoRoles }]
```
Assessment: right reason; the entry has neither roles nor a drop row. GREEN was data only (the roles), as the plan allowed.

**Task 2 RED (`5fb18da`)**
```
eqv_03_roles:   21 errors  E0599 no variant Null (7) and Local (3) of Role, E0560/E0609 no field local on ArgRole (7), no variant SizeOf of SetupArg (2), no variant InByte of SetValue (2)
eqv_03_adapter: 11 errors  the same items, and E0061 scalar_words takes 1 argument but 2 were supplied (4)
eqv_03_every_entry_is_drivable_or_logged: FAILED  coremqtt x2, isotp, lz4 LZ4F_getFrameInfo, nanopb pb_decode_ex, nimble adv, zlib inflate: neither drivable nor logged
eqv_04_tier_c_attempts: FAILED  the driver must build: ... coremqtt ... NoRoles
```
Assessment: right reasons; each names the missing item or entry.

**Task 3 RED (`088d6a9`)**
```
eqv_04_committed_sample: E0432 unresolved import mt_harness::replay::corpus_dirs_below
eqv_04_vectors_cli: sample::eqv_04_sample_cli_selects_one_vector_per_target FAILED  error: unrecognized subcommand 'sample'
eqv_04_workflow_static: eqv_04_workflow_static_sample_steps FAILED (no step "Fixture vectors sample"); eqv_04_workflow_static_captured_code_runs_only_in_run_guest FAILED (2 calls, expected 3)
```
Assessment: right reasons. The plan names `select_sample`, `write_sample` and `replay_tree` as the missing items; the committed-sample test imports the tree walk `corpus_dirs_below` instead, and the CLI test reaches the other two through the command.

**Fix RED (`654eb2c`)**
```
eqv_04_workflow_static_captures_travel_by_artifact: FAILED  chmod misses "$RUNNER_TEMP/work/capture":  chmod -R a+rX "$RUNNER_TEMP/work" "$RUNNER_TEMP/out"
```
Assessment: right reason; it pins the lesson of round trip 1.

GREEN for each: the named tests pass, the four pre-merge gates exit 0 (79 result lines, 532 tests, no `skipping:` line), and the 21 ignored local proofs of `eqv_04_exec` pass with `MT_LOCAL_CAPTURES` (the new ones are `eqv_04_tier_c_attempts` and `eqv_04_sample_is_chosen_and_replays_from_a_tree`).

## Tracer feedback gate

Auto mode was off and `human_verify_mode` was the default `end-of-phase`; the Tracer's `<verify>` is automated-only, so it was re-run after the Tracer GREEN commit (all ignored local proofs, `eqv_04_tier_c_attempts ... ok`, the `mbedtls/default/mbedtls_x509_crt_parse_der vectors=200 identical` row, no `skipping:` line, then the four gates) and logged `Tracer verified end-to-end - expanding`. No checkpoint was synthesized.

## Phase evidence

### ROADMAP Phase 3 success criteria

| # | Criterion | Evidence |
|---|-----------|----------|
| 1 | Contract v0 as a versioned document and a policy file the harness loads; refinement, observation point, UB menu with `return error` default and a recorded setting to change it | `docs/specs/equivalence-contract.md` 0.1.0 and 16 policy files (plan 03-02: `eqv_01_policy`, `eqv_01_committed_policies`, `eqv_01_contract_document`, `mt vectors policy-check`); the executor loads the policy before every run and records its sha256 in each provenance record (`policy_sha256`, plan 03-04); the manual read of the prose is a founder item below |
| 2 | Vector schema with exactly the nine fields, round trips, merged `lib_state` rejected, at least 3 TRACTOR vectors load unchanged | plan 03-03: `eqv_02_schema`, `eqv_02_state_and_compare`, `eqv_02_corpus`, `eqv_02_byte_codec`, `eqv_02_tractor_pin_has_no_content`; CI notice `vectors-tractor` in this plan's runs: `loaded: 6 of 6 (at least 3 required)`, every file `sha256=ok round_trip=json-equal` |
| 3 | At least 10 targets have an entry-point adapter | `eqv_03_adapter_floor_is_ten` and `eqv_03_every_entry_is_drivable_or_logged` over the committed files; `mt vectors adapters --check-floor` prints `drivable targets: 15 (floor 10)`; `vectors-run` has twice-identical rows for 15 distinct targets in the pinned image |
| 4 | The executor runs any vector against the original C build and stores a schema-valid vector; twice gives byte-identical results | `vectors-run`: 38 target/configuration rows, every one `skipped=0 twice=identical`; `vectors-determinism`: a second full run in a second container, rows identical, `identical 5768 files`; `vectors-sample`: the 15 committed vectors replayed twice each and reproduced byte for byte, and the same run regenerated the same sample tar |

### 03-VALIDATION.md rows

| Plan / task | Requirement | Test or CI notice | Status |
|-------------|-------------|-------------------|--------|
| 03-02 T1, T2 | EQV-01 | `cargo test -p mt-contract eqv_01` (`eqv_01_policy`, `eqv_01_committed_policies`) | pass (gates of every plan) |
| 03-02 T3 | EQV-01 | `eqv_01_contract_document` | pass |
| 03-03 T1, T2 | EQV-02 | `cargo test -p mt-vectors eqv_02` | pass |
| 03-03 T1 | EQV-02 | `eqv_02_byte_codec` | pass |
| 03-03 T3 | EQV-02 | `eqv_02_tractor_vectors_load_unchanged` (ignored, local) and CI `vectors-tractor` | pass (6 of 6 in runs 40919765252 and 40922236736) |
| 03-03 T3 | EQV-02 | `eqv_02_tractor_pin_has_no_content` | pass |
| 03-04..03-08 | EQV-03 | `cargo test -p mt-capture eqv_03_roles` (this plan adds `eqv_03_roles_tier_c_forms`) | pass |
| 03-06 T3, 03-08 T2 | EQV-03 | `eqv_03_adapter_floor_is_ten`, `eqv_03_every_entry_is_drivable_or_logged` | pass |
| 03-04..03-07 | EQV-03 | `eqv_03_driver_golden` | pass |
| 03-04 T2 | EQV-04 | `eqv_04_refuses_modified_object` (`eqv_04_link`) | pass |
| 03-01 T2 (03-07 T1) | EQV-04 | `eqv_04_classify` | pass |
| 03-05 T1 | EQV-04 | `eqv_04_layout_parser` (`eqv_04_layout`) | pass |
| 03-04 T1, T3 | EQV-04 | `eqv_04_run_twice_is_byte_identical` (ignored, local) and CI `vectors-run` `twice=identical` | pass |
| 03-01 T2, T3 | EQV-04 | `eqv_04_overrun_is_a_fault` (self-test rows `overrun-input`, `overrun-output` fault -11) | pass |
| 03-01 T2 | EQV-04 | `eqv_04_timeout_is_recorded` (self-test row `loop` timeout -9) | pass |
| 03-04 T2 | EQV-04 | `eqv_04_sentinel_prefill_recorded` (ignored, local) | pass |
| 03-04 T3, 03-08 T3 | EQV-04 | `mt vectors check` in `vectors-determinism` and the sample replay in `vectors-sample` | pass |
| 03-01 .. 03-08 | workflow | `cargo test eqv_04_workflow_static` (12 tests, this plan rewrites them job by job) | pass |
| 03-09 Tracer | EQV-04 | `container_static` tests `eqv_04_dockerfile_takes_only_qemu_from_backports`, `eqv_04_container_workflow_probes_cortex_m4_user_mode_start`, `eqv_04_pin_discovery_lists_backports_qemu_versions` | pass |
| 03-09 Tracer, T3 | EQV-04 | container workflow notices `qemu-mprofile` and `check-a` (container runs 40917551251 and 40919765379 also success in this plan) | pass |
| 03-10 T3 | EQV-04 | the eight Phase 2 sections equal `02-22-CI-NOTICES.md` from line 5 (rounds 1, 2 and 3 of this plan) | pass |
| manual | EQV-01 | read `docs/specs/equivalence-contract.md` against 03-CONTEXT.md D-05..D-32 | founder item |

`/gsd-validate-phase` can set `nyquist_compliant` once the founder item is read; every automated row is green.

## Deviations from Plan

**1. [Founder decision D-32] The vectors steps run in a second job.** The plan assumed every vectors notice stays in the `testbed` job and counted annotations there; the founder's reply "Second job (Recommended)" moved them. Exact changes against the plan: the self-test, TRACTOR, build, layouts, run and determinism steps and the sample steps are in job `vectors` (`needs: [mt, testbed]`, `if: !cancelled() && needs.mt.result == 'success'`, same runner image, own timeout); the `testbed` job gained "Archive the captures for the vectors job" and one upload step and lost the vectors helpers (`run_vectors`, `fetch_tractor`, `run_guest`, `finish_vectors`) its steps no longer call; the vectors job gained "Pull the image by digest" (title `vectors-image`), "Download the captures of the testbed job" and "Unpack the captures of the testbed job". The static tests read job-scoped text and were updated test-first (`58f2931` then `c6419e1`); `build_02_workflow_is_least_privilege_and_scheduled` now expects the three jobs. The plan's notice reading changes with it: `vectors-*` notices come from the vectors job id, `testbed-*` from the testbed job id (both ids are in every round trip row above and in the head of each notices file). The plan's Task 3 verify commands were not changed; its `03-08-VECTORS-NOTICES.md` holds the vectors job's sections and its `03-08-CI-NOTICES.md` the testbed job's.

**2. [Rule 1 - Bug] The unpack step failed in round trip 1.** Found in the first push: `chmod -R a+rX` over the whole work and out directories exited 1 on files the earlier containers had made. Fix: chmod only the unpacked trees (`c41d877`, RED `654eb2c`). Cost: one of the three round trips; the plan had two planned, so the third was the last allowed and the final run.

**3. [Interpretation] isotp is logged, not declared.** The plan says to declare isotp's roles and prove them only in the image. Its single captured object leaves three user hooks undefined (`isotp_user_debug`, `isotp_user_get_ms`, `isotp_user_send_can`; `arm-none-eabi-nm -u` of `isotp.c` compiled with the pinned flags), the driver may not define them (no per-target C, D-11), and a declared entry that does not link makes `vectors-build` print `result: failed`, which skips every later vectors step. A declared isotp would have cost a round trip to learn what the object file already says, in a plan with two spare round trips at most. The drop row says exactly that, including that the pair itself was not linked.

**4. [Interpretation] coreMQTT's packet information is a `state`, not a `local`.** The plan says "a local packet-info struct filled by set steps". The parameter is a pointer to the struct, so a `state` named `packet` with set steps is the same object, recorded in `lib_state_in` after the set steps and in `lib_state_out`; it avoids a role to take the address of a local. `local` is used where the value of a pointer local is needed (lz4F).

**5. [Interpretation] The NOTICE arrives with the sample, not with GREEN.** The plan puts `testbed/vectors-sample/NOTICE` in GREEN, but `eqv_04_committed_sample_is_valid` allows an absent sample only while the NOTICE is absent too, and the replay step treats the directory's existence as "a sample is committed". Both arrive together in `5e0d4a8`.

**6. [Interpretation] API details the plan left open.** `select_sample` takes a fourth argument `only` (target ids; the CLI's optional positional ids), `mt vectors sample` fails closed when a drivable target has no corpus and writes nothing, `Adapter::scalar_words` takes the input length, `replay::corpus_dirs_below` is public, the CI replay step also passes `--manifest` and `--pins` (the plan's command was abbreviated), and the sample fixture is published by three steps (parts 1 to 9, 10 to 18, 19 on).

**7. [Test maintenance] Existing tests changed because the committed state changed.** `eqv_03_floor_counts_only_seeded_undropped_entries` and `eqv_04_adapters_cli_checks_the_floor` now log every entry with roles of the six floor-test targets (lz4, nimble and others gained entries); `eqv_04_adapters_cli_lists_entries` expects `nanopb/pb_decode_ex dropped (not_expressible)` and no `no_roles` row, and the `no_roles` row is asserted in the floor test's temporary copy; `eqv_03_roles_setup_and_local_parse_from_toml_and_stay_absent_otherwise` checks the absence of the `"local": [` table because the `local` role and key now legitimately contain the word. Three details of my own D-32 RED tests were wrong and corrected in GREEN without weakening any assertion about the product (the image tool has no mt mount, the testbed titles come also from `finish_step`, and a job's text excludes the comment above the next job).

**8. [Process] `gsd_run` is not on the shell path**; the pre-commit protected-branch assertion used the five-name fallback (the branch is `claude/gsd-discuss-phase-1-mzdw5v`), and `node .claude/gsd-core/bin/gsd-tools.cjs` was used for the state commands. Disk was 1.7 GB free at the end.

**Total deviations:** 8 (1 founder decision, 1 bug, 5 interpretations or test maintenance, 1 process). **Impact:** the plan's goal is met; the round-trip cap is fully used.

## Decisions Made

See `key-decisions`. For the founder to read at `/gsd-verify-work`:

- **The first-seed rule makes short samples.** The smallest seed of most entries is one byte (the sample vectors of 7 of 15 targets are 1-byte inputs and tinycbor's is empty). It is deterministic and cheap and exercises the refusal paths, not the accepting ones; the full set in CI covers the rest.
- **`$len-2` wraps in 32 bits for coreMQTT.** A seed shorter than two bytes (`41`, `00`) gives a huge remaining length, which both functions refuse before reading (no fault); all 15 seeds have a one-byte remaining length, so one fixed offset fits every seed.
- **Opaque heap object of LZ4F_getFrameInfo.** `LZ4F_dctx` is incomplete in the header, so the context is recorded as a pointer into the heap and its internals are not recorded; the observable result is the frame information and the consumed size.

## Known Stubs

None.

## Threat Flags

| Flag | File | Description |
|------|------|-------------|
| threat_flag: artifact-extraction | .github/workflows/testbed.yml | The vectors job extracts a tar made from trees that upstream build code produced (in no-network containers of the testbed job) into its runner temp directory. Mitigations in place: the tar is made by an image tool with relative member names, extracted with `tar -xf` and no absolute-name option (a test refuses `-P` and `--absolute-names`), the runner is ephemeral with `contents: read` and no secret, and only the unpacked trees are chmod'ed. The trust is that of the existing `testbed-reports` upload. |

The plan's register is otherwise unchanged: T-03-28 by `eqv_03_every_entry_is_drivable_or_logged` and `eqv_03_adapter_drop_log_is_consistent`; T-03-29 by the tar sha256 check before unpacking, the pinned-digest and `replay identical` assertions of `eqv_04_committed_sample_is_valid`, the byte-for-byte replay in `vectors-sample` and the NOTICE; T-03-30 by the step durations (each round trip finished well inside 45 minutes, no timeout raised); T-03-31 by the replay comparing canonical bytes and the driver, archive and object hashes of the current build; T-03-SC: no external crate added (`Cargo.lock` gained only path crates against `949214e`; `cargo deny check` passes).

## Issues Encountered

- **Open founder questions untouched:** lz4's overshoot against D-07, and function-local statics named `name.N` in `globals` (none of the new entries changes one; mbedtls records 33 globals as before).
- **The round-trip cap is spent.** Phase 3 verification and any further push to a testbed trigger path need a fresh budget; this plan's docs-only pushes started only ci.
- **Local versus image toolchains.** Local proofs ran on GCC 13.2.1, QEMU 8.2.2 and llvm-dwarfdump 18.1.3, the image on GCC 14.3, QEMU 10.0.2 and llvm-dwarfdump 16.0.6; every row count that exists in both agrees, and all 14 sample vectors I rehearsed locally have the same sha256 as the image's (for those targets the vector bytes do not depend on the toolchain version). The generated drivers of the four new entries also compile clean under GCC 13 with the GCC 14 errors switched on (`-Werror=incompatible-pointer-types`, `int-conversion`, `implicit-function-declaration`, `implicit-int`).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Phase 3 has no unexecuted plan: 03-01 to 03-10 are done. Next: `/gsd-verify-work 3`, the founder items below, then `/gsd-code-review 3` and `/gsd-ship`.
- **Founder items for verify-work:** (1) the manual contract read of `docs/specs/equivalence-contract.md` against D-05..D-32; (2) the timeout label (`has_ub` label `timeout` with rc -9, plan 03-01); (3) the error-code table of `[[ub.error_code]]`, which stays empty until Phase 4's UB decisions log; (4) the two open questions above; (5) whether `isotp` should get its user hooks from the harness (a reviewed shared C shim like the runtime template) in a later phase, which is the only way to drive it.

## Self-Check: PASSED

- Created files found on disk: `crates/mt-harness/src/sample.rs`, `crates/mt-harness/tests/eqv_04_committed_sample.rs`, `testbed/vectors-sample/NOTICE`, 15 `index.json` files, `03-08-CI-NOTICES.md`, `03-08-VECTORS-NOTICES.md`.
- Commits found as ancestors of HEAD: 58f2931, c6419e1, 073443e, 28ab24c, 5fb18da, de95de8, 088d6a9, feebc49, 654eb2c, c41d877, 5e0d4a8, ffa622a.
- Plan-level verification: the pre-merge gates exit 0 on the final code commit `5e0d4a8`; `Cargo.lock` gained no external package against `949214e`; no forbidden claim word in `crates/mt-harness/src`, `testbed/vectors-sample`, `testbed/adapter-dropped.toml`, `docs/specs/equivalence-contract.md`; no `Public-Tests` path in `git ls-files`; Task 3's second verify exits 0 (run 40922236736 and its ci run green for `5e0d4a8`, the eight Phase 2 sections equal 02-22's, 15 targets twice identical, 15 sample corpora identical, no later commit touches a testbed trigger path).
