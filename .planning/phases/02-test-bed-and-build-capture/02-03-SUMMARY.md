---
phase: 02-test-bed-and-build-capture
plan: 03
subsystem: testing
tags: [manifest, serde, toml, tree-hash, sha256, licence, test-bed, decisions]

requires:
  - phase: 02-test-bed-and-build-capture
    provides: "plan 02-01 image extension and PROJECT.md D-17"
provides:
  - "crates/mt-capture: Manifest schema, loader, validator, canonical_json, and the tree hash"
  - "testbed/manifest.toml: the founder-approved 16-target test bed, pinned by commit and tree sha256"
  - "PROJECT.md decisions D-18 (first RTOS deferred), D-19 (dual-licence reading), D-20 (archive source kind)"
affects: [02-04, 02-05, 02-06, 02-07, 02-08, 02-09, 02-10, testbed fetch, testbed capture, seeds]

target_list_approval: approve-as-proposed
drop_granularity: variant

actuals:
  tokens: 27000
  tasks: 3
  commits: 4
plan_head_before: 04fcb5885588b64f391c133b25b80ecee5273845
plan_head_after: 3686cc057648f6eb7b6cc69e6206770e83bd6ac3

tech-stack:
  added: []
  patterns:
    - "Manifest structs mirror pins.rs: deny_unknown_fields, BTreeMap tables, issues collected with dotted paths, sorted and deduplicated"
    - "Founder approval is committed to the SUMMARY before the data file it approves exists"
    - "Committed-data tests read the repository file via CARGO_MANIFEST_DIR/../.."

key-files:
  created:
    - crates/mt-capture/Cargo.toml
    - crates/mt-capture/src/lib.rs
    - crates/mt-capture/src/manifest.rs
    - crates/mt-capture/src/treehash.rs
    - crates/mt-capture/tests/bed_01_manifest.rs
    - crates/mt-capture/tests/bed_01_tree_hash.rs
    - crates/mt-capture/tests/bed_01_committed_manifest.rs
    - crates/mt-capture/tests/fixtures/manifest-small.toml
    - testbed/manifest.toml
  modified:
    - Cargo.toml
    - Cargo.lock
    - .planning/PROJECT.md

key-decisions:
  - "D-18 (PROJECT.md): first RTOS ecosystem deferred; parsers are RTOS-independent; candidates Zephyr and FreeRTOS; revisit after the design-partner interviews"
  - "D-19 (PROJECT.md): an OR licence expression is admitted only with a permissive elected operand; mbedtls Apache-2.0, paho BSD-3-Clause, libfdt BSD-2-Clause; copyleft-only stays excluded"
  - "D-20 (PROJECT.md): Mbed TLS is an archive source pinned by archive sha256 plus tree sha256, with the tag commit for provenance"
  - "Drop granularity is per variant: a variant that fails after its one fix attempt drops only that variant; a failing default drops the target (founder, D-11 open item)"

patterns-established:
  - "Entries are named by hand in the manifest and checked against clang output after capture (D-23)"
  - "Only tracer targets carry build_args until the tracer is green; validate_complete reports the rest as target.<id>.build_args: missing"

requirements-completed: [BED-01, BED-02]

coverage:
  - id: D1
    description: "Manifest schema and validator reject unknown keys, duplicate ids, malformed pins, unsafe paths, unknown placeholders, copyleft-only or unelected licences, bad entry forms and bad configuration sets"
    requirement: BED-01
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/bed_01_manifest.rs (19 tests, bed_01_* and bed_02_*)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Tree hash over a checked-out tree is pinned by an independent vector, skips only a top-level .git and never follows symlinks"
    requirement: BED-01
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/bed_01_tree_hash.rs (4 tests)"
        status: pass
    human_judgment: false
  - id: D3
    description: "testbed/manifest.toml lists the 16 approved targets, each pinned by commit and tree sha256, with permissive elected licences, 27 entries, the beachhead categories and 31 configurations"
    requirement: BED-01
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/bed_01_committed_manifest.rs#bed_01_committed_manifest_validates"
        status: pass
      - kind: unit
        ref: "crates/mt-capture/tests/bed_01_committed_manifest.rs#bed_01_manifest_pins_every_target_by_commit"
        status: pass
      - kind: unit
        ref: "crates/mt-capture/tests/bed_01_committed_manifest.rs#bed_02_every_target_declares_default_and_at_most_one_variant"
        status: pass
    human_judgment: false
  - id: D4
    description: "The first RTOS ecosystem is recorded as deferred in the manifest and in PROJECT.md D-18 and the Key Decisions row"
    requirement: BED-02
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/bed_01_committed_manifest.rs#bed_02_first_rtos_decision_recorded"
        status: pass
    human_judgment: false
  - id: D5
    description: "The pinned commits and tree hashes in the manifest match the real upstream repositories"
    requirement: BED-01
    verification: []
    human_judgment: true
    rationale: "This plan is offline by design; the check is the CI fetch of plans 02-06 and 02-09, which recomputes each tree hash. Until then the values are the RESEARCH appendix values."

duration: 10min (continuation from the D-03 checkpoint; Task 1 was done by the previous executor)
completed: 2026-10-13
status: complete
---

# Phase 2 Plan 3: Test-bed manifest, validator, tree hash and the approved target list Summary

**mt-capture crate with a strict manifest validator and a symlink-safe tree hash, plus the founder-approved testbed/manifest.toml of 16 commit- and tree-pinned parsers, and PROJECT decisions D-18 to D-20**

## Performance

- **Duration:** about 10 min for this continuation; Task 1 and its checkpoint were earlier
- **Completed:** 2026-10-13T03:30:00Z
- **Tasks:** 3 (Task 1 auto/TDD, Task 2 checkpoint:decision, Task 3 auto)
- **Files modified:** 12 (9 created, 3 modified, excluding this summary)

## Founder approval (Task 2, checkpoint:decision, gate blocking-human)

The orchestrator collected the answer directly from the founder at the D-03 checkpoint. It was committed in this SUMMARY (714d604) before `testbed/manifest.toml` existed (3686cc0).

```
target_list_approval: approve-as-proposed
drop_granularity: variant
```

- **Edits:** no edits. The list, entries and variants are exactly those of RESEARCH Tables 1-3.
- **Final target ids (16):** tinycbor, zcbor, qcbor, nanopb, cjson, corejson, coremqtt, paho, microcoap, mbedtls, nimble, libfdt, httpparser, isotp, lz4, zlib.
- **Entries:** 27 (the 28 of Table 2 minus libfdt `fdt_check_header`, omitted as proposed because it takes no length and is not a D-24 buffer-in entry; `fdt_check_full` stays). The six wrapped entries carry `accepted_wrapped = true` (tinycbor `cbor_value_validate`, qcbor `QCBORDecode_Init`, nanopb `pb_decode_ex`, coremqtt `MQTT_DeserializePublish` and `MQTT_DeserializeAck`, zlib `inflate`); mbedtls `mbedtls_asn1_get_tag` is form `range`.
- **Variants:** those of Table 3 (one per target, none for isotp), giving 31 target-configuration pairs.
- **D-27 elections:** mbedtls Apache-2.0, paho BSD-3-Clause, libfdt BSD-2-Clause.
- **Backups:** libcbor and yajl are NOT added.
- **Drop granularity (research Open Question 4):** `drop_granularity: variant`. A variant that still fails after its one fix attempt (D-11) drops only that variant, which is logged. A default configuration that fails drops the target. Plan 02-09 applies this.
- **Recorded, not decided again:** http-parser appends `-O3` after the baseline and the gate records the optimisation goal without failing it (Open Question 7); Mbed TLS is fetched as the release tarball (Open Question 5, now PROJECT D-20); signatures were printed by clang 18 and the tracer checks them with the image's clang 16 (A4).

## Accomplishments

- `mt-capture` is a workspace crate with `#![forbid(unsafe_code)]`, the manifest schema (`Manifest`, `Target`, `Source`, `Licence`, `BuildArgs`, `Entry`, `Config` and friends), `parse`, `load`, `validate`, `validate_complete` and `canonical_json`. The validator enforces D-20, D-24, D-27, D-28 and the three BED-02 edges (empty variant, variant equal to the default, reordered tables).
- `treehash` implements the D-17 algorithm: sorted rows `<mode> <sha256> <relpath>`, symlinks hashed by their target text and never followed, only a top-level `.git` skipped, special files an error. The digest is pinned by an independently computed vector.
- `testbed/manifest.toml` declares the 16 approved targets with the RESEARCH appendix pins (Mbed TLS as an archive source), permissive elected licences with the licence file path, 27 entries, 31 configurations, and recipes for the two tracer targets only (qcbor Make, corejson CMake).
- PROJECT.md records D-18 (first RTOS deferred), D-19 (dual-licence reading) and D-20 (archive source kind); the Key Decisions row reads `Deferred (D-18)`.
- Ten tests over the committed file (`bed_01_committed_manifest.rs`): nine named in the plan plus `bed_01_project_records_dual_licence_and_archive_decisions`.

## Task Commits

1. **Task 1 (RED): failing tests for the manifest validator and tree hash** - `bf2929b` (test)
2. **Task 1 (GREEN): manifest validator, canonical_json, tree hash** - `6f9ca2d` (feat)
3. **Task 2: founder approval recorded in this SUMMARY** - `714d604` (docs)
4. **Task 3: approved manifest, committed-manifest tests, PROJECT.md D-18 to D-20** - `3686cc0` (feat)

There is no REFACTOR commit for Task 1. **Plan metadata:** the commit that completes this SUMMARY (docs).

## Files Created/Modified

- `crates/mt-capture/src/manifest.rs` - schema, loader, validator, canonical form
- `crates/mt-capture/src/treehash.rs` - tree rows and tree sha256
- `crates/mt-capture/src/lib.rs`, `crates/mt-capture/Cargo.toml` - crate shell, path dependency on mt-toolchain, workspace lints
- `crates/mt-capture/tests/bed_01_manifest.rs`, `bed_01_tree_hash.rs`, `fixtures/manifest-small.toml` - validator and tree-hash tests over a two-target fixture
- `crates/mt-capture/tests/bed_01_committed_manifest.rs` - ten tests over the committed manifest and PROJECT.md
- `testbed/manifest.toml` - the approved test bed
- `Cargo.toml`, `Cargo.lock` - workspace member added; lock gained only the new path crate
- `.planning/PROJECT.md` - D-18, D-19, D-20 and the Key Decisions row

## Decisions Made

- D-18, D-19 and D-20 as recorded in PROJECT.md (see key-decisions).
- Founder-chosen drop granularity `variant` (above).
- Test `bed_01_every_target_has_a_buffer_in_entry` also pins the exact wrapped set and the single `range` entry, so a later edit to the entry forms is a visible diff in a test, not only in the data.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical] Four extra validator tests**
- **Found during:** Task 1 (previous executor)
- **Issue:** The planned test list did not exercise the build-system consistency rules or the decision record or special files in the tree hash.
- **Fix:** Added `bed_01_build_args_must_match_the_build_system`, `bed_02_config_settings_must_match_the_build_system`, `bed_02_decision_record_is_validated` and `bed_01_tree_hash_rejects_special_files`.
- **Committed in:** `bf2929b` and `6f9ca2d`

**2. [Rule 3 - Blocking] Cargo.lock updated for the new workspace member**
- **Found during:** Task 1 (previous executor)
- **Issue:** The first test run rewrote Cargo.lock to add `mt-capture`.
- **Fix:** Committed the lock change. No external crate was added; every dependency was already locked and passes `cargo deny check`.
- **Committed in:** `6f9ca2d`

**3. [Rule 3 - Blocking] A header comment matched the `build_args]` acceptance grep**
- **Found during:** Task 3
- **Issue:** My first header comment in `testbed/manifest.toml` mentioned `[target.<id>.build_args]`, so `grep -c 'build_args\]'` printed 3 instead of the required 2.
- **Fix:** Reworded the comment to "a build recipe table".
- **Committed in:** `3686cc0`

### Behaviour notes (not defects)

- Unknown keys are rejected at serde parse time, as a TOML error naming the key, not as a dotted-path validation issue. `deny_unknown_fields` appears 9 times in `manifest.rs`.
- The source field `ref` is the Rust field `reference` (`#[serde(rename = "ref")]`).
- The D-03 answer was collected by the orchestrator directly from the founder, not by this executor, because the task is `gate="blocking-human"` and the executor cannot ask.
- The plan commit ledger was re-created in this worktree with the previous executor's recorded base (04fcb58), so `commits: 4` counts the four commits of this plan up to Task 3; this SUMMARY's own commit follows.

---

**Total deviations:** 3 auto-fixed (1 missing critical, 2 blocking)
**Impact on plan:** No scope creep; no external dependency, no architectural change.

## Issues Encountered

- A compound `git`-containing shell command was refused by the worktree guard; the same checks ran as separate plain commands.

## Authentication Gates

None.

## Known Stubs

None. The 14 build recipes not yet written are not stubs: `build_args` is optional by design until plan 02-09 (D-03), and `validate_complete` reports the 14 missing ones.

## Threat Flags

None. No new network endpoint, auth path or file access beyond what the plan's threat model (T-02-08 to T-02-12) covers; the committed-manifest tests only read two repository files.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plans 02-04 to 02-09 can read `testbed/manifest.toml` through `Manifest::load` and trust its shape; the tree hash is ready for the fetch step.
- The pins are the RESEARCH appendix values and are not yet checked against the real repositories: the CI fetch in plans 02-06 and 02-09 does that, and a mismatch there means the algorithm or the pin is wrong, not the plan.
- Apply `drop_granularity: variant` in plan 02-09.

## Self-Check: PASSED

- Created files exist: manifest.rs, treehash.rs, lib.rs, Cargo.toml, bed_01_manifest.rs, bed_01_tree_hash.rs, bed_01_committed_manifest.rs, manifest-small.toml, testbed/manifest.toml (all FOUND).
- Commits `bf2929b`, `6f9ca2d`, `714d604`, `3686cc0` are ancestors of HEAD (all FOUND).
- Acceptance: `grep -c 'build_args\]' testbed/manifest.toml` prints 2; PROJECT.md has lines starting `- **D-18:**`, `- **D-19:**`, `- **D-20:**` and the row reads `Deferred (D-18)`; `deny_unknown_fields` count 9 (at least 8); `lib.rs` starts with `#![forbid(unsafe_code)]`.
- Verification: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace --locked` and `cargo deny check` all exited 0 before the Task 3 commit.
- D-03 ordering: the approval commit `714d604` precedes the commit that created `testbed/manifest.toml` (`3686cc0`).

---
*Phase: 02-test-bed-and-build-capture*
*Completed: 2026-10-13*
