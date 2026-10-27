---
phase: 03-equivalence-contract-and-test-vector-schema
plan: 03
subsystem: testing
tags: [test-vectors, tractor, latin-1-codec, lib-state, corpus, sparse-fetch, serde, mt-vectors]

requires:
  - phase: 03-equivalence-contract-and-test-vector-schema
    provides: plan 03-01's outcome class set and testbed workflow helpers (run_vectors, finish_vectors), plan 03-02's contract text (field convention, library-state shapes, comparison rule) and the mt vectors group
  - phase: 02-test-bed-and-build-capture
    provides: mt-capture's fetch module (Stepper, FetchContext, FetchError) and the mt-seeds index pattern
provides:
  - "crates/mt-vectors: Vector (nine fields, canonical JSON, sha256), HasUbLabel, OutcomeClass/OUTCOME_CLASSES, TractorVector, encode_latin1/decode_latin1, StateValue/LibState/Pointer/RESERVED_NAMES, compare/Comparison, write_corpus/read_corpus/CorpusIndex/Provenance, TractorPin/check_tree/TractorReport, sha256_hex"
  - "mt vectors validate [--tractor] <PATH>... (reference file, TRACTOR-level file, or corpus directory)"
  - "mt vectors tractor fetch|check, testbed/tractor-pin.toml (repo, commit, six paths, sha256; no content)"
  - "mt_capture::fetch::fetch_sparse and verify_head (additive to Phase 2's fetch)"
  - "testbed.yml: fetch_tractor and the step 'Fetch the TRACTOR sample and load it unchanged (local use only)', notice vectors-tractor"
affects: [03-04, 03-05, 03-06, 03-07, 03-08, phase 4 relabelling, phase 5 comparators, phase 11 evidence bundle, phase 12 TRACTOR licence check]

requirements-completed: [EQV-02]

status: complete
commits: 7
plan_head_before: 6f1c7e32f6afe23694fb5a8614a4d46fe5849dd5
plan_head_after: 4b834757143a0fb366718f4f2f251191f7b82054
actuals:
  tokens: 41019
  tasks: 3
  commits: 7

tech-stack:
  added: []
  patterns:
    - "two validation levels: strict reference Vector (deny_unknown_fields, all eight fields required) and permissive TractorVector (all optional, note accepted, any present has_ub is UB), 'loads unchanged' meaning JSON-equal re-serialization"
    - "has_ub is absent or a label, enforced by the type (Option<HasUbLabel> with a deserialize_with that keeps a present null as an error); no boolean path exists"
    - "state values parsed from serde_json::Value by hand so every error names the path; one shape key per value; raw addresses unrepresentable"
    - "corpus written only into an empty directory, then re-listed; read refuses symlinks, non-regular files, foreign names, non-canonical bytes and index/file-set drift"
    - "TRACTOR content never in the repository: pin records commit, paths and sha256; the check hashes every file before loading it; a test hashes every repository file against the pin"

key-files:
  created:
    - crates/mt-vectors/Cargo.toml
    - crates/mt-vectors/src/lib.rs
    - crates/mt-vectors/src/vector.rs
    - crates/mt-vectors/src/bytes.rs
    - crates/mt-vectors/src/state.rs
    - crates/mt-vectors/src/compare.rs
    - crates/mt-vectors/src/corpus.rs
    - crates/mt-vectors/src/tractor.rs
    - crates/mt-vectors/tests/eqv_02_schema.rs
    - crates/mt-vectors/tests/eqv_02_byte_codec.rs
    - crates/mt-vectors/tests/eqv_02_state_and_compare.rs
    - crates/mt-vectors/tests/eqv_02_corpus.rs
    - crates/mt-vectors/tests/eqv_02_tractor.rs
    - crates/mt-vectors/tests/fixtures/own/reference.json
    - crates/mt-capture/tests/eqv_02_sparse_fetch.rs
    - crates/mt-cli/tests/eqv_02_vectors_cli.rs
    - testbed/tractor-pin.toml
    - .planning/phases/03-equivalence-contract-and-test-vector-schema/03-03-CI-NOTICES.md
    - .planning/phases/03-equivalence-contract-and-test-vector-schema/03-03-VECTORS-NOTICES.md
  modified:
    - Cargo.toml
    - Cargo.lock
    - crates/mt-cli/Cargo.toml
    - crates/mt-cli/src/main.rs
    - crates/mt-capture/src/fetch.rs
    - crates/mt-capture/tests/build_02_workflow_static.rs
    - crates/mt-harness/tests/eqv_04_workflow_static.rs
    - .github/workflows/testbed.yml

key-decisions:
  - "Claude's discretion settled as the plan listed it: two validation levels; canonical JSON is serde_json::to_vec_pretty in field order, maps as BTreeMap, trailing newline, sha256 of those bytes is the file name; MAX_VECTOR_BYTES 16 MiB checked before parsing; OutcomeClass lives in mt-vectors (plan 03-04 ties mt-harness's class_word to it)"
  - "ASSUMPTION for the founder: lib_state `globals` and `heap` are struct-shaped values ({type, fields}) whose member names are `<object>:<symbol>` and decimal offsets; `in` must be a byte-array value; `ret` is any value. The contract text names the keys but not the wrapper, so this is mt-vectors' reading; plans 03-04/03-05 write to it"
  - "ASSUMPTION for the founder: a 32-bit-or-less value is a JSON integer in i32::MIN..=u32::MAX; any decimal string `-?[0-9]+` without leading zeros within i64::MIN..=u64::MAX is the 64-bit form; the type text is not checked against the width (no C type table in this crate)"
  - "Provenance fields: seed, scalars, objects, archive, driver_source, driver_elf, image, executor_version, contract_version, policy_sha256, budgets, outcome, has_ub_source, unobserved, discarded_output_bytes, replay; target/config/entry live in the index and are read from every vector's argv[0] (a mixed set is refused, as is an empty set or a duplicate vector)"
  - "Two cross-checks beyond the plan's has_ub_source rule: provenance.outcome must equal the vector's own outcome class, and provenance.unobserved must equal the number of unobserved marks in lib_state_in plus lib_state_out. Plans 03-04/03-05 must fill both consistently; either can be relaxed if the founder reads 'unobserved count' differently"
  - "compare names a differing lib_state_out field by path: lib_state_out.<name>[.<member>|[i]]; fields are listed in the order rc, stdout.pattern, lib_state_out.*, stderr.outcome; a field present on one side only differs unless the expected side marks it unobserved"
  - "Corpus directory arguments to `mt vectors validate` are read as corpora; --tractor on a directory is an error"
  - "TRACTOR check: a bad file is a failed row, not an abort, so the report always says 'loaded: n of m (at least 3 required)'; a missing tree directory is exit 2; the closing line is `tractor: passed|failed` (not `result: passed`, so the self-test's `result: passed (n rows)` line stays the only one the notices check counts)"

patterns-established:
  - "Pure schema crate: mt-vectors depends on serde, serde_json, sha2, thiserror and toml only, no internal crate and no process execution; the one process (git) lives in mt-capture"
  - "A networked helper that can run exactly one command, pinned by a static workflow test to one call site"

coverage:
  - id: D1
    description: "The reference vector has exactly the nine fields in order, round-trips to byte-identical canonical JSON, refuses unknown keys (a merged lib_state and note included) and never carries a boolean has_ub; bytes go through the Latin-1 codec losslessly for all 256 values"
    requirement: EQV-02
    verification:
      - kind: unit
        ref: "crates/mt-vectors/tests/eqv_02_schema.rs#eqv_02_reference_vector_round_trips_byte_for_byte"
        status: pass
      - kind: unit
        ref: "crates/mt-vectors/tests/eqv_02_schema.rs#eqv_02_rejects_merged_lib_state"
        status: pass
      - kind: unit
        ref: "crates/mt-vectors/tests/eqv_02_schema.rs#eqv_02_has_ub_absent_means_no_observed_ub"
        status: pass
      - kind: unit
        ref: "crates/mt-vectors/tests/eqv_02_byte_codec.rs#eqv_02_byte_codec_round_trips_every_byte"
        status: pass
      - kind: integration
        ref: "crates/mt-cli/tests/eqv_02_vectors_cli.rs#eqv_02_validate_cli_rejects_a_merged_lib_state"
        status: pass
    human_judgment: false
  - id: D2
    description: "The TRACTOR level reads every optional shape and any present has_ub as UB, and re-serializes JSON-equal; the lib_state value model follows the contract's shapes and refuses raw addresses; compare implements D-08 with the has_ub exclusion"
    requirement: EQV-02
    verification:
      - kind: unit
        ref: "crates/mt-vectors/tests/eqv_02_schema.rs#eqv_02_tractor_level_accepts_every_optional_shape"
        status: pass
      - kind: unit
        ref: "crates/mt-vectors/tests/eqv_02_state_and_compare.rs#eqv_02_lib_state_rejects_bad_shapes"
        status: pass
      - kind: unit
        ref: "crates/mt-vectors/tests/eqv_02_state_and_compare.rs#eqv_02_compares_only_the_observation_point"
        status: pass
      - kind: unit
        ref: "crates/mt-vectors/tests/eqv_02_state_and_compare.rs#eqv_02_has_ub_vectors_are_excluded_from_comparison"
        status: pass
    human_judgment: true
    rationale: "Tests prove the shapes and the rule as this plan read the contract; whether the reserved-name wrapper for globals/heap and the 32/64-bit value split are what the founder meant is a reading task at /gsd-verify-work (see key-decisions)"
  - id: D3
    description: "The sha256-named corpus with its provenance index: written only into an empty directory, foreign or inconsistent entries refused, the has_ub_source rule enforced"
    requirement: EQV-02
    verification:
      - kind: unit
        ref: "crates/mt-vectors/tests/eqv_02_corpus.rs#eqv_02_corpus_round_trips_with_provenance"
        status: pass
      - kind: unit
        ref: "crates/mt-vectors/tests/eqv_02_corpus.rs#eqv_02_corpus_detects_a_foreign_entry"
        status: pass
      - kind: unit
        ref: "crates/mt-vectors/tests/eqv_02_corpus.rs#eqv_02_provenance_has_ub_source_unchecked_for_unlabelled_vectors"
        status: pass
    human_judgment: false
  - id: D4
    description: "Six pinned TRACTOR public vectors fetched by commit (sparse) load unchanged and re-serialize JSON-equal, locally and in the pinned image; no TRACTOR file is in the repository; the fetch is the only networked step added"
    requirement: EQV-02
    verification:
      - kind: unit
        ref: "crates/mt-vectors/tests/eqv_02_tractor.rs#eqv_02_tractor_pin_has_no_content"
        status: pass
      - kind: other
        ref: "MT_LOCAL_TRACTOR=.mt-work/tractor/37960ee0.../tree cargo test -p mt-vectors --test eqv_02_tractor -- --ignored eqv_02_tractor_vectors_load_unchanged (tractor loaded=6 of 6)"
        status: pass
      - kind: other
        ref: "testbed run 40456178214 (be0f323), notice vectors-tractor: loaded: 6 of 6 (at least 3 required); 03-03-VECTORS-NOTICES.md"
        status: pass
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_04_workflow_static.rs#eqv_02_workflow_static_tractor_fetch_is_the_only_new_networked_step"
        status: pass
      - kind: unit
        ref: "crates/mt-capture/tests/eqv_02_sparse_fetch.rs#eqv_02_sparse_fetch_checks_commit_and_paths"
        status: pass
    human_judgment: false

duration: "about 10.5 h wall clock (2026-10-27T05:16:50Z to 15:42Z), of which a usage-limit pause interrupted the CI wait; execution itself was roughly three working sessions"
completed: 2026-10-27
---

# Phase 3 Plan 03: Vector schema, comparison rule, corpus and the pinned TRACTOR sample Summary

**The new pure `mt-vectors` crate makes the nine-field TRACTOR-shaped vector machine-checkable (strict reference level with a byte-identical canonical form and a Latin-1 byte codec, a permissive TRACTOR level that treats any present `has_ub` as UB, a typed library-state value model, the D-08 comparison rule and a sha256-named corpus with a provenance index), and six pinned public TRACTOR vectors fetched sparsely by commit load unchanged both on this host and in the pinned image (testbed run 40456178214: `loaded: 6 of 6`), with no TRACTOR content in the repository.**

## Performance

- **Duration:** about 10.5 h wall clock from 2026-10-27T05:16:50Z to the notices commit at 15:42Z; the gap is dominated by a usage-limit pause around the CI wait, not by work.
- **Tasks:** 3 of 3 (Tracer, Task 2, Task 3), each RED then GREEN.
- **Files:** 25 changed in crates, testbed, workflow and manifests against `6f1c7e3` (4510 insertions, 8 deletions), plus the two notices files and this summary.

## Accomplishments

- **Reference level (EQV-02, D-05, D-25, D-26):** `Vector` has the nine fields in order with `deny_unknown_fields`; `lib_state`, `note` and any other key are refused with the key named. `has_ub` is `Option<HasUbLabel>`: absent means no observed UB, a present `null`, boolean or number fails, and nothing in the type can write `false`. `stdout` and `stderr` are `{pattern, is_regex: false}`; `argv[0]` is `<target>/<config>/<entry>`; `stderr` starts `outcome=<class>`. Canonical JSON round-trips the hand-made fixture byte for byte.
- **Byte codec:** `encode_latin1`/`decode_latin1` are lossless for all 256 values (NUL, `"`, `\`, 0x7F, 0x80, 0xFF in the fixture); a character above U+00FF is refused with its position.
- **TRACTOR level (D-26):** `TractorVector` has every field optional plus `note`, `deny_unknown_fields`, `has_ub` kept as a present value of any JSON type; `load_unchanged` parses twice and succeeds only when the re-serialized value equals the file's `Value`.
- **Library state (D-06):** `StateValue`/`LibState` cover the contract's shapes (32-bit integers, 64-bit decimal strings, IEEE bit patterns, byte arrays as hex, items, fields, pointers as null, `{buffer, offset}` or `{symbol}`, unobserved with a reason), the reserved names and the unobserved count.
- **Comparison (D-08):** `compare` is `Excluded{label}` when the expected vector has `has_ub`, otherwise compares exactly `rc`, `stdout.pattern`, `lib_state_out` (expected-unobserved skipped) and the outcome class word; stderr detail text alone is `Equal`.
- **Corpus (D-19, D-21):** `write_corpus` (empty directory only, re-listed afterwards) and `read_corpus` (regular files named `<sha256>.json` plus `index.json`, canonical bytes, index equal to the file set, provenance checked per vector, including the `has_ub_source` rule).
- **TRACTOR pin (D-23):** `testbed/tractor-pin.toml` records the repository, commit `37960ee08a7c644ee66e29855eca33903f702e67` and six paths with sha256; `mt vectors tractor fetch` uses the new `fetch_sparse` (blobless, `--no-cone` sparse checkout, `rev-parse` check) and `mt vectors tractor check` hashes then loads each file.
- **CI step:** `fetch_tractor` (the same hardening and mounts as `run_networked`, ending `mt vectors tractor fetch "$@"`, called from one step) then `run_vectors tractor check`; a fetch failure is published with `fetch failed with exit code N` first.

## Task Commits

1. **Tracer** (`tdd`): `3821a37` test (RED), `c2c161f` feat (GREEN)
2. **Task 2** (`tdd`): `76ced04` test (RED), `9aee0a2` feat (GREEN)
3. **Task 3** (`tdd`): `ddbd887` test (RED), `be0f323` feat (GREEN, pushed; round trip 1)
4. **Notices:** `4b83475` docs (round trip 1 notices)

No REFACTOR commits were needed. `commits: 7` is the measured `git rev-list --count 6f1c7e3..HEAD` at the time this summary was written (the six code commits and the notices commit); the summary, STATE and ROADMAP commits follow it.

## CI round trips (1 of 3 used)

| Round trip | Head | testbed | ci | Conclusion |
|------------|------|---------|----|------------|
| 1 | `be0f323` | 40456178214 (job 122030154846) success | 40456178149 success | green; the container run 40456178159 was not awaited (also success) |

- Annotations of job 122030154846: 45, all notices. Steps: all eight Phase 2 notices, the fixtures, `vectors-selftest` (`result: passed (17 rows)`) and `vectors-tractor`.
- `vectors-tractor`: five git steps `ok`, `fetched 6 pinned files into /work/tractor/37960ee0.../tree`, six rows each `sha256=ok ... round_trip=json-equal` (three with `has_ub=present`, three absent), `loaded: 6 of 6 (at least 3 required)`, `tractor: passed`, `exit code: 0`. This is the first proof that `fetch_sparse` works against github.com from the pinned image.
- The eight Phase 2 notices in `03-03-CI-NOTICES.md` equal `02-22-CI-NOTICES.md` from line 5 on (`diff` empty); the third Task 3 automated verify exits 0. The fourth (latest `ci` run on the branch is success for HEAD) is checked after the docs-only push; see the hand-off report for its result.

## TDD evidence (RED output, with a one-line assessment per failure)

**Tracer RED (`3821a37`: tests, our fixture and the allowed skeleton)**

```
error[E0432]: unresolved import `mt_vectors::bytes`   (eqv_02_schema, eqv_02_byte_codec)
error[E0432]: unresolved import `mt_vectors::vector`
test eqv_02_validate_cli_accepts_the_reference_fixture ... FAILED
test eqv_02_validate_cli_rejects_a_merged_lib_state ... FAILED
  the refusal must name the key: error: unrecognized subcommand 'validate'
```

Assessment: right reason; the skeleton holds neither module, and clap rejects `validate` (the "rejects" test's exit code 2 would have passed by accident, so it asserts that the key is named and the message is not "unrecognized").

**Task 2 RED (`76ced04`: tests only)**

```
error[E0432]: unresolved import `mt_vectors::corpus` / `mt_vectors::compare` / `mt_vectors::state`
error[E0432]: unresolved import `mt_vectors::vector::TractorVector`
error: could not compile `mt-cli` (test "eqv_02_vectors_cli") due to 1 previous error
```

Assessment: right reason for every test file, but a compile error hides the assertion failures. The `mt-cli` file fails to compile because one case imports `mt_vectors::corpus`, so its behaviour was shown instead by running the Tracer binary: `mt vectors validate --tractor <file>` gave clap's `unexpected argument '--tractor' found` (exit 2) and, without the flag, the strict level refused the same file (`invalid type: integer 7, expected a string`).

**Task 3 RED (`ddbd887`: tests only; the pin file arrives in GREEN)**

```
error[E0432]: unresolved import `mt_vectors::tractor`          (eqv_02_tractor)
error[E0432]: unresolved imports `mt_capture::fetch::fetch_sparse`, `mt_capture::fetch::verify_head`
error[E0425]: cannot find function `sha256_hex` in crate `mt_vectors`   (eqv_02_vectors_cli)
test eqv_02_workflow_static_tractor_fetch_is_the_only_new_networked_step ... FAILED
  .github/workflows/testbed.yml defines no fetch_tractor() {
```

Assessment: right reason; the static test is the one RED that is an assertion, not a compile error. The `build_02_workflow_static` helper-list change is intended and passes on its own (12 tests green), since the removed name does not exist yet.

GREEN: every named test passes (`cargo test --workspace --locked`: 432 passed, 0 failed, 3 ignored, no `skipping:` line), and the four pre-merge gates exit 0 at each GREEN commit and on HEAD before the push.

## Deviations from Plan

**1. [Orchestrator] The first push carried only this plan's own commits.** At the founder's request the 03-01 fixture and notices commits and all of 03-02 were already pushed (head `6f1c7e3`), so the plan's statement that the first push carries every unpushed commit since 03-01 meant six commits here. That earlier push started a testbed run that is not evidence for any plan.

**2. [Minor] The pre-decided shell fetch was not needed.** `mt vectors tractor fetch` reached github.com from this host through the runner, fetched all six files, and every pinned sha256 matched on the first attempt, so the local proof is the real `fetch_sparse` and no hash was touched. Local output: `tractor loaded=6 of 6`. A deliberately corrupted file was re-fetched and re-verified, which exercises the reuse path too.

**3. [Minor] `verify_head` added to mt-capture.** No upstream can make git print a head other than the commit it was asked for, so `CommitMismatch` is tested through a small public helper that `fetch_sparse` uses, not through the full fetch. Phase 2's `fetch`, `fetch_git` and `verify_cached` are untouched; the ten `bed_01_fetch` tests pass.

**4. [Minor] `--tractor` arrives in Task 2, not the Tracer.** The plan's Tracer lists `Validate { tractor, paths }` but also says the TRACTOR level arrives in Task 2; adding a flag that did nothing would be a stub, so the field was added with its behaviour.

**5. [Minor] Invariant-commented `expect` in `Vector::render`.** `sha256()` returns a `String` (as the plan's tests read it), so serialization of a struct of strings, integers and sorted maps is unwrapped with an `#[allow(clippy::expect_used)]` and a comment stating why it cannot fail. `to_canonical_json` returns `Result` because it validates first.

**6. [Rule 1 - Lint] clippy `assertions_on_constants` in a Task 3 test.** `assert!(MIN_TRACTOR_LOADED >= 3)` became `const { assert!(...) }` in the GREEN commit; the assertion is unchanged.

**7. [Minor] Public additions the plan did not list:** `mt_vectors::sha256_hex`, `vector::read_capped`, `Vector::load`, `Vector::validate`, `TractorPin::parse/tree_dir/paths`, `TractorReport::render/passed`, `LibState::new/get/fields/is_empty/validate`, `StateValue::from_json/to_json`. They keep file reading and hashing in one place and keep `mt-cli` to argument handling.

**8. [Interpretation] Several rules go beyond the plan's text** (listed under key-decisions as assumptions): struct-shaped `globals`/`heap`, the outcome and unobserved-count cross-checks in provenance, a mixed-entry/duplicate/empty set refused by `write_corpus`, and the report's `tractor: passed|failed` closing line.

**9. [Process] The run was interrupted by a usage limit** after the push and resumed by the orchestrator at the CI evaluation; nothing was repeated and no CI round trip was lost.

**Total deviations:** 9 (1 orchestrator, 1 Rule 1 lint, 7 minor or interpretive). **Impact:** none on the plan's goal; one of three round trips used.

## Tracer feedback gate

Auto mode was off and `human_verify_mode` was the default `end-of-phase`; the tracer's `<verify>` is automated-only, so it was re-run after the Tracer GREEN commit (the eight named tests, the two CLI tests, the four gates) and logged `Tracer verified end-to-end - expanding`. No checkpoint was synthesized.

## Plan-level verification

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check`: all exit 0 on `be0f323`.
- `git diff 949214e -- Cargo.lock`: `+name =` only for `mt-contract`, `mt-harness` and `mt-vectors` (path crates); `cargo tree -p mt-vectors --edges normal` lists no internal crate.
- `git ls-files | grep -c Public-Tests`: 0; `eqv_02_tractor_pin_has_no_content` passes; `git status --porcelain -- .mt-work` is empty.
- `grep -rniE 'certif[i]ed|complia[n]t'` over `crates/mt-vectors`, `testbed/tractor-pin.toml` and both notices files: no match.
- No `unwrap`/`expect` outside tests in `crates/mt-vectors/src` except the one invariant-commented `expect`; `#![forbid(unsafe_code)]` in `mt-vectors`.

## Known Stubs

None.

## Threat Flags

None beyond the plan's register. Mitigations exercised: T-03-09 by `eqv_02_sparse_fetch_checks_commit_and_paths` and `eqv_02_tractor_check_refuses_a_changed_file` (and the CI row `sha256=ok` for all six); T-03-10 by `eqv_02_rejects_a_file_above_the_size_cap` and the size check in `read_corpus`/`read_pinned`; T-03-11 by `eqv_02_tractor_pin_has_no_content` plus `git ls-files`; T-03-12 by `eqv_02_workflow_static_tractor_fetch_is_the_only_new_networked_step`; T-03-13 by `eqv_02_corpus_detects_a_foreign_entry` and `eqv_02_corpus_refuses_a_non_empty_directory`; T-03-SC: no external crate added.

## Issues Encountered

None that changed the plan. The fetch worked from the host, so the pre-decided handling was not triggered.

## Next Phase Readiness

- Plan 03-04 can write vectors through `Vector`/`write_corpus` and compare through `compare`; it must set `Provenance.outcome` and `Provenance.unobserved` consistently with the vector and tie `mt-harness`'s `Outcome::class_word` to `OutcomeClass` with a test.
- Plans 03-04/03-05 should read the assumptions under key-decisions (struct-shaped `globals`/`heap`, argv[0] as the entry key) before emitting state; the founder reads them at `/gsd-verify-work`.
- The TRACTOR sample stays local-use only until Phase 12 decides the licence; nothing from it is committed.

## Self-Check: PASSED

- Created files found on disk: all eight `mt-vectors` sources and tests, the fixture, `eqv_02_sparse_fetch.rs`, `eqv_02_vectors_cli.rs`, `testbed/tractor-pin.toml`, both notices files.
- Commits found as ancestors of HEAD: 3821a37, c2c161f, 76ced04, 9aee0a2, ddbd887, be0f323, 4b83475.
- Task acceptance criteria and the plan-level verification re-run after the CI evaluation: pass.
