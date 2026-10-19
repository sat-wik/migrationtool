---
phase: 02-test-bed-and-build-capture
plan: 15
subsystem: testing
tags: [seeds, lexer, designated-initialisers, manifest, httpparser, qcbor, nanopb, gap-closure]

requires:
  - phase: 02-test-bed-and-build-capture
    provides: "plan 02-08 seed lexer, manifest SeedSource and seed_target; plan 02-09 committed corpora; plan 02-13 zip backend and zip_members budget"
provides:
  - "mt_seeds::lexer::c_designated_string_literals: strings that are the value of a `.name =` designated initialiser after { or , (c_string_literals unchanged, shares one run collector)"
  - "SeedSource.designators (optional list of C identifiers; valid only with methods exactly [\"c_string\"]) wired through seed_target"
  - "httpparser, qcbor and nanopb seed_source tables narrowed to input-bearing arrays and fixture files; their three corpora regenerated and committed"
affects: [02-16, 02-17 image proof of seeds --check, Phase 3 and Phase 5 harness seed starts]

actuals:
  tokens: 5300
  tasks: 2
  commits: 4
plan_head_before: 93a86acc84bb45cf5d00cb011561bf2ed23328a0
plan_head_after: 84e51655d27a9dc68a7c65ad0f121e40c7262de9

tech-stack:
  added: []
  patterns: ["a private run collector with a keep predicate, so a new selector cannot change an existing extraction", "optional list fields are Option<Vec> when an explicit empty list must be rejected (same as extensions)"]

key-files:
  created: ["crates/mt-seeds/tests/build_03_seed_designators.rs", "crates/mt-capture/tests/build_03_seed_narrowing.rs"]
  modified: ["crates/mt-seeds/src/lexer.rs", "crates/mt-capture/src/manifest.rs", "crates/mt-capture/src/seeding.rs", "testbed/manifest.toml", "testbed/seeds/httpparser/", "testbed/seeds/nanopb/", "testbed/seeds/qcbor/"]

key-decisions:
  - "SeedSource.designators is Option<Vec<String>>, not Vec<String> with serde default: the plan asks to reject an explicit `designators = []`, which a defaulted Vec cannot tell apart from an absent key"
  - "qcbor keeps test/qcbor_encode_tests.c as the plan says, although a few of its small arrays are payload blobs encoded into byte strings rather than CBOR; recorded in the manifest comment"
  - "Commit attribution trailer names Claude Sonnet 5.5 (the model that ran), as the session attribution reminder directs; the orchestrator prompt named Opus 5.5 and earlier commits carry that name"

patterns-established:
  - "Narrowed seed sources are declared per file with a comment that says what was left out and why"

requirements-completed: [BUILD-03]

coverage:
  - id: D1
    description: "A manifest seed_source can name designators; c_string extraction then keeps only the strings that are the value of those designated members, and the validator accepts designators only with methods exactly [\"c_string\"] and C identifiers"
    requirement: BUILD-03
    verification:
      - kind: unit
        ref: "crates/mt-seeds/tests/build_03_seed_designators.rs#build_03_c_strings_after_designator_only"
        status: pass
      - kind: unit
        ref: "crates/mt-seeds/tests/build_03_seed_designators.rs#build_03_c_string_literals_unchanged_without_designators"
        status: pass
      - kind: unit
        ref: "crates/mt-capture/tests/build_03_seed_narrowing.rs#build_03_seed_source_designators_validate"
        status: pass
      - kind: unit
        ref: "crates/mt-capture/tests/build_03_seed_narrowing.rs#build_03_seeding_takes_only_designated_strings"
        status: pass
    human_judgment: false
  - id: D2
    description: "httpparser, qcbor and nanopb seeds come only from the `.raw` messages, the CBOR test arrays and the fixture files and fuzz corpora; corpora regenerated; seeds --all --check passes on the host with every other corpus unchanged"
    requirement: BUILD-03
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/build_03_seed_narrowing.rs#build_03_narrowed_corpora_hold_only_input_methods"
        status: pass
      - kind: other
        ref: "cargo run --locked -q -p mt-cli -- testbed seeds --manifest testbed/manifest.toml --pins container/pins.toml --all --check (result: passed, 16 rows)"
        status: pass
      - kind: other
        ref: "git diff --name-only origin/claude/gsd-discuss-phase-1-mzdw5v HEAD -- testbed/seeds touches only httpparser, nanopb, qcbor, isotp"
        status: pass
    human_judgment: false
  - id: D3
    description: "The narrowed corpora hold parser inputs rather than labels: the quoted samples are inputs, but qcbor's encode-test arrays include a few payload blobs"
    verification: []
    human_judgment: true
    rationale: "Whether each retained array and message counts as a parser input is a judgment about upstream test content; the committed-data test asserts methods and paths, not meaning. Samples are quoted below for the founder."

duration: 7min
completed: 2026-10-19
status: complete
---

# Phase 2 Plan 15: Designated-String Seed Selector and Narrowed Corpora Summary

**Seed sources can now select the strings of one designated initialiser member (`designators = ["raw"]`), and httpparser's corpus is its 76 HTTP messages (was 200 mixed strings) while qcbor holds only CBOR test arrays and nanopb only its fixture files and fuzz-corpus members.**

## Performance

- **Duration:** 7 min
- **Started:** 2026-10-19T21:34:53Z
- **Completed:** 2026-10-19T21:42Z
- **Tasks:** 2 (Task 1 tracer, Task 2 manifest and corpora)
- **Files modified:** 7 source, test and manifest files, plus the three corpora (338 seed files added, 462 removed, 3 index.json changed)

## Accomplishments

- `c_designated_string_literals(src, designators)` keeps a string run only when the four tokens before it are `{` or `,`, `.`, a listed identifier and `=`. `c_string_literals` now calls the same private collector with an accept-all predicate, and the existing corpora did not move before any manifest edit (`seeds --all --check` passed, 16 rows, after the code commit).
- `SeedSource.designators` validates (methods exactly `["c_string"]`, C identifiers, not empty) and `seed_target` routes a designated source through the new function; nothing else in extraction changed.
- Manifest tables narrowed and the three corpora regenerated from the verified host caches with a freshly built mt-cli; `testbed validate` and `seeds --all --check` pass on the host (the image run is plan 02-17).

## Corpus counts

| Target | Seeds before | Seeds after | Skipped before (dup / too_large / over_cap / unsupported) | Skipped after |
|--------|-------------|-------------|------------------------------------------------------------|---------------|
| httpparser | 200 (all c_string, test.c) | 76 (all c_string, test.c, `.raw` only) | 242 / 0 / 670 / 0 | 0 / 0 / 0 / 0 |
| qcbor | 200 (98 c_array, 102 c_string) | 200 (all c_array) | 42 / 0 / 995 / 343 | 32 / 0 / 88 / 331 |
| nanopb | 200 (190 c_string, 2 file_copy, 8 zip_member) | 200 (2 file_copy, 198 zip_member) | 127 / 189 / 4056 / 0 | 0 / 189 / 3291 / 0 |

After, by source: qcbor not_well_formed_cbor.h 100, qcbor_decode_tests.c 69, qcbor_encode_tests.c 29, float_tests.c 2; nanopb sample1.pb and sample2.pb 1 each, corpus.zip 191 members, regressions.zip 7 members. The 77 `.raw` members of httpparser's test.c give 76 seeds (two messages are byte-identical).

## Sample seeds (first bytes, source and line)

httpparser (test.c):
- `GET /test HTTP/1.1\r\nUser-Age...` (154 B) test.c:97
- `\r\nGET /test HTTP/1.1\r\n\r\n` (24 B) test.c:473
- `POST / HTTP/1.1\r\nHost: www.e...` (135 B) test.c:841
- `POST / HTTP/1.1\r\nTransfer-En...` (99 B) test.c:1225
- `HTTP/1.0 204 No content\r\nCon...` (51 B) test.c:1655

qcbor (c_array):
- `8b f9 00 00 fb 40 09 1e b8 51 eb 85 1f ...` (137 B) test/float_tests.c:625
- `b8` (1 B, a truncated map head) test/not_well_formed_cbor.h:137
- `f8 18` (2 B, a malformed simple value) test/not_well_formed_cbor.h:231
- `87 a6 08 00 61 79 6a 32 30 38 35 ...` (158 B, array of 7 with dates) test/qcbor_decode_tests.c:3583
- `a6 04 c1 fb 7e 37 e4 3c ...` (41 B) test/qcbor_decode_tests.c:6820

nanopb (zip_member, members of tests/fuzztest/corpus.zip):
- `9a 7f 01 00 ba 02 00 ba 02 00 ...` (3616 B) member 000bbe6b...
- `e5 01 10 30 18 01 20 2b 28 0c ...` (232 B) member 03b35537...
- `10 ff ff 7f` (4 B) member 0625ceb3...
- `01 7e` (2 B) member 07cbe4af...
- `f9 01 aa 01 08 09 07 08 0c 04 ...` (1635 B) member 0abe53aa...

## TDD evidence

**Tracer (Task 1).**
- RED `10490bc` (two new test files only, no production file). `cargo test --locked -p mt-seeds --test build_03_seed_designators`: `error[E0432]: unresolved import mt_seeds::lexer::c_designated_string_literals ... no c_designated_string_literals in lexer`, `could not compile mt-seeds (test "build_03_seed_designators") due to 1 previous error`. `cargo test --locked -p mt-capture --test build_03_seed_narrowing`: both tests FAILED with `the text parses: Parse(Error { message: "unknown field designators, expected one of path, methods, extensions, entries" ...` (test result: FAILED. 0 passed; 2 failed). Semantic assessment: both targets ran and failed for the intended reason, the function and the manifest key do not exist yet; the first valid-without-key assertions of the validate test passed before the failing line, as designed.
- GREEN `3d0a6a3`: all four new tests pass; `cargo test -p mt-seeds` and `--test build_03_seed_narrowing --test build_03_seed_manifest --test bed_01_manifest` pass unchanged (20 + 4 + 2 mt-capture tests); `seeds --all --check` passed on 16 rows before any manifest edit.

**Task 2.**
- RED `1214dad` (test file only; manifest and indexes untouched): `build_03_narrowed_corpora_hold_only_input_methods ... FAILED`, panic at tests/build_03_seed_narrowing.rs:211 `assertion left == right failed: qcbor must declare no c_string source  left: ["test"]  right: []`; `test result: FAILED. 2 passed; 1 failed`. Semantic assessment: the target test ran and failed on the planned first assertion (qcbor still declares the `test` c_string source). The nanopb and httpparser assertions were not reached at RED; they pass at GREEN and the GREEN corpora satisfy all three targets.
- GREEN `84e5165`: manifest edits and regenerated corpora; the test passes with `build_03_committed_seeds` and `bed_01_committed_manifest` (11 + 7 + 3 tests).

## Task Commits

1. **Task 1 (tracer): designated-initialiser selector, manifest to seed index** - `10490bc` (test), `3d0a6a3` (feat)
2. **Task 2: narrow httpparser, qcbor, nanopb; regenerate corpora** - `1214dad` (test), `84e5165` (feat)

**Plan metadata:** the docs commit that carries this SUMMARY (docs: complete plan)

## Files Created/Modified

- `crates/mt-seeds/src/lexer.rs` - `c_designated_string_literals`, shared private `string_runs`, module doc line
- `crates/mt-seeds/tests/build_03_seed_designators.rs` - selector and no-regression tests
- `crates/mt-capture/src/manifest.rs` - `SeedSource.designators` and its validation (reuses the existing `is_c_identifier`)
- `crates/mt-capture/src/seeding.rs` - `extract_file` takes the designators and picks the lexer function
- `crates/mt-capture/tests/build_03_seed_narrowing.rs` - validation, seeding and committed-data tests
- `testbed/manifest.toml` - httpparser designators; qcbor four c_array sources; nanopb string source removed; comments say what was left out and why
- `testbed/seeds/{httpparser,nanopb,qcbor}/` - regenerated corpora

## Decisions Made

See key-decisions. Retained scope notes for the founder:
- qcbor's 200-seed cap now truncates 88 array candidates in the fixed selection order (method rank, then source path), so the tail of qcbor_encode_tests.c is mostly cut (29 of its arrays survive) while not_well_formed_cbor.h keeps 100. That is the existing D-16 policy, not changed here.
- nanopb's corpus is now almost entirely `corpus.zip` members (191 of 198); the 7 `regressions.zip` members and the two `.pb` samples are the rest.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Plan inconsistency] `designators` is `Option<Vec<String>>`**
- **Found during:** Task 1 (GREEN, manifest.rs)
- **Issue:** the plan specifies `Vec<String>` with `#[serde(default, skip_serializing_if = "Vec::is_empty")]` and also asks to reject an explicitly empty `designators = []`; with a defaulted Vec the two cannot be told apart, so the empty-list rejection (and its test) would be impossible.
- **Fix:** `Option<Vec<String>>` with `#[serde(default, skip_serializing_if = "Option::is_none")]`, the pattern `extensions` already uses. The tests and acceptance criteria are unchanged.
- **Files modified:** crates/mt-capture/src/manifest.rs
- **Verification:** `build_03_seed_source_designators_validate` rejects the empty list; no `SeedSource` struct literal exists elsewhere.
- **Committed in:** 3d0a6a3

**2. [Rule 3 - Blocking] Did not add a second `is_c_identifier`**
- **Found during:** Task 1 (GREEN)
- **Issue:** manifest.rs already has an identical `is_c_identifier`; adding the plan's implied helper failed to compile (E0428).
- **Fix:** reused the existing function.
- **Committed in:** 3d0a6a3

---

**Total deviations:** 2 auto-fixed (1 plan inconsistency, 1 blocking). **Impact on plan:** none on scope or behavior.

## Issues Encountered

- The orchestrator prompt named `Co-Authored-By: Claude Opus 5.5`; the session attribution instruction names Claude Sonnet 5.5, the model that executed this plan, so the four commits carry the Sonnet 5.5 trailer. Earlier commits on the branch carry Opus 5.5. Amend if the founder wants them uniform.
- `container version check` (pre-merge list) was not run; it is outside this plan's verification list. `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --workspace --locked` (289 passed, 0 failed) and `cargo deny check` (advisories, bans, licenses, sources ok) all pass at `84e5165`.

## Known Imprecision (T-02-51)

- qcbor_encode_tests.c: most arrays are expected CBOR encodings, but a few small ones (lines 561 to 570, 689, 1712, 2908, 3047) are payload byte blobs that the tests encode into byte strings, not CBOR. They are still byte inputs, are noted in the manifest comment and are not claimed to be encodings.
- The httpparser `.raw` selector depends on the C designator form `.raw =`; a message written another way in a future upstream pin would be missed silently. The pin is fixed and the committed-data test bounds the count at 77.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Plan 02-16 can run on this tree; it makes an `--all` run below the 10-target floor an error and this plan's tests use explicit ids on small manifests. Plan 02-17 runs the image proof of `seeds --all --check` against the three regenerated corpora. No blockers.

## Self-Check: PASSED

- Files found: build_03_seed_designators.rs, build_03_seed_narrowing.rs, lexer.rs, manifest.rs, seeding.rs, testbed/manifest.toml, testbed/seeds/{httpparser,nanopb,qcbor}/index.json
- Commits found as ancestors of HEAD: 10490bc, 3d0a6a3, 1214dad, 84e5165
- `commits: 4` measured with `git rev-list --count 93a86ac..HEAD`
- Stub scan of created and modified files: none

---
*Phase: 02-test-bed-and-build-capture*
*Completed: 2026-10-19*
