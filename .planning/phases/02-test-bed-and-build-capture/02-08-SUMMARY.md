---
phase: 02-test-bed-and-build-capture
plan: 08
subsystem: testing
tags: [seeds, corpus, hand-made, manifest, github-actions, determinism, licensing]

requires:
  - phase: 02-test-bed-and-build-capture
    provides: "plan 02-07 mt-seeds (lexer, hex, zip, select, index); plan 02-06 mt testbed CLI, StepSummary and the testbed workflow; plan 02-03 manifest API; plan 02-05 fetch and verify_cached"
provides:
  - "testbed/manifest.toml [seed_caps] and per-target [[target.<id>.seed_source]] tables, validated (methods, safe paths, extensions, entries)"
  - "mt_capture::seeding: seed_target, run_seeds, run_seeds_detailed, notice_text; mt testbed seeds [--check] [--seeds-dir]"
  - "testbed/seeds/<id>/ for all 16 targets (1623 files: seeds named by sha256, index.json, NOTICE), identical when regenerated"
  - "Hand-made .hex seed sets for paho, microcoap, libfdt, isotp, lz4, zlib (16 to 20 each) and coremqtt (10), with research/testbed-seeds helpers"
  - "testbed workflow step 'seeds' (run_offline seeds --check) with notice testbed-seeds"
affects: [02-09 full-bed run (set TESTBED_SELECTION to --all, which also checks the other 14 corpora in the image), 02-10, Phase 7 held-out split GATE-05]

plan_head_before: 2423826fab4ba1435109e7e34bcc92d022f98c0c
plan_head_after: 7f16a10c0327d38f82daf3221dd4b7d30fe281cc

actuals:
  tokens: 31473   # chars/4 over the added lines of the realized diff; Cargo.lock and the generated testbed/seeds corpora excluded
  tasks: 3
  commits: 5      # measured: git rev-list --count 2423826..7f16a10 (SUMMARY and state commits not included)

tech-stack:
  added: []
  patterns:
    - "Seeds come only from manifest-declared paths of the verified tree; a missing or symlinked declared path is an error, never silence"
    - "Check mode regenerates in memory and compares file by file; drift is a row plus difference lines on standard error, exit 1"
    - "Hand-made inputs are reviewable hex text with a specification comment; computed bytes come from research helpers run once, never by the tool"

key-files:
  created:
    - crates/mt-capture/src/seeding.rs
    - crates/mt-capture/tests/build_03_seed_manifest.rs
    - crates/mt-capture/tests/build_03_committed_seeds.rs
    - crates/mt-cli/tests/build_03_seeds_cli.rs
    - research/testbed-seeds/README.md
    - research/testbed-seeds/libfdt.py
    - research/testbed-seeds/lz4.py
    - research/testbed-seeds/zlib.py
    - testbed/targets/ (116 hand-made .hex files under <id>/hand-made/)
    - testbed/seeds/ (16 corpora)
  modified:
    - crates/mt-capture/src/manifest.rs
    - crates/mt-capture/src/testbed.rs
    - crates/mt-capture/src/lib.rs
    - crates/mt-capture/Cargo.toml
    - crates/mt-capture/tests/fixtures/manifest-small.toml
    - crates/mt-capture/tests/bed_01_fetch.rs
    - crates/mt-capture/tests/support/mod.rs
    - crates/mt-cli/src/main.rs
    - testbed/manifest.toml
    - testbed/README.md
    - .github/workflows/testbed.yml
    - Cargo.lock

key-decisions:
  - "zlib has no seed_source: test/example.c holds error messages and test/infcover.c holds its inflate streams as hex text inside C strings, so string extraction yields ASCII of those, not streams; the 18 seeds are hand-made"
  - "A zip archive above 2 MiB is not opened (counted unsupported), a source file above 8 MiB is not read: the member count of an archive is unbounded, so the archive size is the bound"
  - "seed_source paths may not be the whole tree or contain .git; a declared path that is missing or goes through a symlink fails the target"
  - "The CLI makes the work directory absolute (the default .mt-work is relative and git ran with the cache directory as its working directory)"
  - "Selection order is unchanged from 02-07, so hand-made seeds rank first when a target has both extracted and hand-made seeds (coremqtt)"

patterns-established:
  - "Pre-fill a verified cache of an archive-kind target in a temp directory to test run_seeds with no git, network or container"
  - "Build mt-cli before generating committed data from it: cargo run or cargo build -p mt-cli, never a binary from before the last source edit"

requirements-completed: [BUILD-03]

coverage:
  - id: D1
    description: "The manifest declares [seed_caps] (max_seeds 200, max_total_bytes 1048576, max_seed_bytes 65536) and per-target seed_source tables, rejecting unknown methods, unsafe paths, bad extensions, unknown entries and zero caps"
    requirement: BUILD-03
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/build_03_seed_manifest.rs#build_03_seed_sources_validate"
        status: pass
    human_judgment: false
  - id: D2
    description: "seed_target extracts seeds with commit, licence, method, source path, line or member and the proposed flag from a fixture tree, and reads only declared sources (symlinks and files above 8 MiB handled)"
    requirement: BUILD-03
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/build_03_seed_manifest.rs#build_03_seeding_extracts_with_provenance_from_a_fixture_tree"
        status: pass
      - kind: unit
        ref: "crates/mt-capture/tests/build_03_seed_manifest.rs#build_03_seeding_reads_only_declared_sources"
        status: pass
    human_judgment: false
  - id: D3
    description: "mt testbed seeds writes the corpus and --check reports drift without changing anything, exit 0, 1 (drift or failure) or 2 (invalid input)"
    requirement: BUILD-03
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/build_03_seed_manifest.rs#build_03_seed_check_reports_drift"
        status: pass
      - kind: integration
        ref: "crates/mt-cli/tests/build_03_seeds_cli.rs (build_03_seeds_help_lists_check_and_seeds_dir, build_03_seeds_cli_exits_one_on_drift_and_zero_when_equal, build_03_seeds_cli_rejects_an_unknown_target_with_exit_two, build_03_seeds_cli_accepts_a_relative_work_dir_for_a_git_source)"
        status: pass
    human_judgment: false
  - id: D4
    description: "Every target has a committed corpus with provenance, within the caps, licence equal to the manifest, only from declared sources; targets with no extracted seed have at least 10 hand-made seeds, all method hand_made and proposed true"
    requirement: BUILD-03
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/build_03_committed_seeds.rs (six tests: build_03_every_retained_target_has_a_seed_corpus, build_03_hand_made_seeds_marked_proposed_and_at_least_ten, build_03_seed_provenance_records_source_path_commit_and_method, build_03_committed_seeds_respect_the_caps, build_03_seed_licence_matches_manifest, build_03_seeds_come_only_from_declared_sources)"
        status: pass
    human_judgment: false
  - id: D5
    description: "In the pinned image, offline, seeds --check regenerates the corpora of qcbor and corejson from the container-fetched trees and finds no drift"
    requirement: BUILD-03
    verification:
      - kind: integration
        ref: "testbed run 38383502703, job 115249862727 (annotation testbed-seeds: corejson seeds 108, qcbor seeds 200, result passed)"
        status: pass
    human_judgment: false
  - id: D6
    description: "Whether the extracted seeds are useful fuzzing inputs (string extraction from whole test files includes some non-input strings) and whether the hand-made seeds follow their specifications as intended"
    verification: []
    human_judgment: true
    rationale: "Seeds are proposals by design (BUILD-03); a person reviews the hand-made files and decides whether noisy extracted sources should be narrowed before a harness uses them"

duration: about 26 min (start estimated from the base commit time 10:05Z; the start time was not captured)
completed: 2026-10-13
status: complete
---

# Phase 2 Plan 08: Seed corpora and the seeds check Summary

**Manifest-declared seed sources and caps, `mt testbed seeds [--check]`, 16 committed deterministic corpora (extracted from each target's own permissively licensed tests, or 10 to 20 hand-made proposed seeds where there are none), and a testbed workflow step that regenerates the seeds offline in the pinned image and found no drift.**

## Performance

- **Duration:** about 26 min wall clock (start estimated from the base commit; not captured in a variable)
- **Started:** 2026-10-13T10:05Z (estimate)
- **Completed:** 2026-10-13T10:32Z
- **Tasks:** 3 (Task 1 TDD; Tasks 2 and 3 plain)
- **Files modified:** 1759 in the plan range (1623 under testbed/seeds, 116 hand-made .hex files, 20 other)

## Accomplishments

- `[seed_caps]` and `[[target.<id>.seed_source]]` are part of the manifest schema with validation (known methods, relative paths with no `..`, `.git` or whole-tree path, extension syntax, entries that exist, caps above zero and `max_seed_bytes <= max_total_bytes`).
- `mt_capture::seeding` extracts from the verified cached tree: it reads only declared paths, never follows a symlink (a declared path through one is an error), counts files above 8 MiB and zip archives above 2 MiB unsupported, applies `file_copy`, `hex_text`, `c_array`, `c_string` and `zip_member`, adds each `*.hex` under `testbed/targets/<id>/hand-made/` as a `hand_made`, `proposed: true` seed, and selects under the manifest caps. `run_seeds` writes `testbed/seeds/<id>/` or, with `--check`, compares in memory.
- All 16 targets have a corpus. Regenerating into an empty directory gives a byte-identical tree (`diff -r`), and `mt testbed seeds --all --check` exits 0 on the host.
- Hand-made sets with specification comments in every file: paho 17 (MQTT 3.1.1), microcoap 16 (RFC 7252), libfdt 16 (Devicetree Specification FDT v17), isotp 19 (ISO 15765-2), lz4 20 (block and frame format), zlib 18 (RFC 1950 and 1951), and 10 extra for coremqtt (MQTT 3.1.1 and 5). Malformed and truncated cases are included on purpose.
- CI: the `seeds` step runs `run_offline seeds --check` after `fetch`; run 38383502703 passed it for qcbor and corejson.

### Seeds per target (committed corpora; bytes are the seed bytes in total)

| Target | Extracted | Hand-made | Total seeds | Total bytes | Skipped (counts in index.json) |
|--------|-----------|-----------|-------------|-------------|--------------------------------|
| cjson | 25 | 0 | 25 | 16175 | dup 0, too_large 0, over_cap 0, unsupported 0 |
| corejson | 108 | 0 | 108 | 1210 | dup 119, too_large 0, over_cap 0, unsupported 0 |
| coremqtt | 5 | 10 | 15 | 133 | dup 988, too_large 0, over_cap 0, unsupported 32 |
| httpparser | 200 | 0 | 200 | 6779 | dup 242, too_large 0, over_cap 670, unsupported 0 |
| isotp | 0 | 19 | 19 | 93 | none |
| libfdt | 0 | 16 | 16 | 1690 | none |
| lz4 | 0 | 20 | 20 | 213 | none |
| mbedtls | 200 | 0 | 200 | 194050 | dup 1, too_large 0, over_cap 25, unsupported 0 |
| microcoap | 0 | 16 | 16 | 146 | none |
| nanopb | 200 | 0 | 200 | 47380 | dup 127, too_large 189, over_cap 4056, unsupported 0 |
| nimble | 200 | 0 | 200 | 2592 | dup 424, too_large 0, over_cap 873, unsupported 471 |
| paho | 0 | 17 | 17 | 230 | none |
| qcbor | 200 | 0 | 200 | 7881 | dup 42, too_large 0, over_cap 995, unsupported 343 |
| tinycbor | 200 | 0 | 200 | 1121 | dup 36, too_large 0, over_cap 2197, unsupported 0 |
| zcbor | 137 | 0 | 137 | 21972 | dup 80, too_large 0, over_cap 0, unsupported 354 |
| zlib | 0 | 18 | 18 | 239 | none |

Extracted counts before any hand-made file existed (first run): paho, microcoap, libfdt, isotp and lz4 were 0 as RESEARCH expected; coremqtt was 5; zlib was 170 with the sources tried first and 0 once they were dropped (see Decisions). nanopb's 8 zip-member seeds come from `corpus.zip` and `regressions.zip`.

## CI evidence

### Round trips used (cap 3)

| Task | Round trips | Detail |
|------|-------------|--------|
| Task 1 | 0 | offline, no push |
| Task 2 | 0 | offline, no push |
| Task 3 | **1** | RT1 green on the first run; no budget extension, no escalation |

### Runs (head SHA 7f16a10c0327d38f82daf3221dd4b7d30fe281cc)

| Run | Id | Result |
|-----|----|--------|
| testbed | **38383502703** | success (job mt 115249476083, job testbed 115249862727) |
| ci | **38383502791** | success (job checks 115249476570) |
| container | 38383502752 | not awaited, still in progress when this was written |

The plan's verify command (latest testbed run is `success <sha of the last commit touching its trigger paths>`) exited 0: both are 7f16a10.

Steps of job 115249862727: `Check the committed seeds` (step 9) succeeded, then capture, replay, determinism and the fixture steps all succeeded as before.

`testbed-seeds` notice (job 115249862727):
```
step: seeds
corejson  seeds 108
qcbor     seeds 200
result: passed (2 rows)
exit code: 0
--- committed seed corpora (compared offline with the container's regeneration)
qcbor commit 930708bb86481e88879eb1d87fd4d664f1d69503 seeds=200 hand_made=0 bytes=7881 skipped={"duplicate":42,"too_large":0,"over_cap":995,"unsupported":343}
corejson commit cffa492da18c890181d64462f8af63992a69d3b0 seeds=108 hand_made=0 bytes=1210 skipped={"duplicate":119,"too_large":0,"over_cap":0,"unsupported":0}
```
No drift. The same job's `testbed-fetch` notice shows both trees fetched by the container's git with tree hashes equal to the pins (corejson 9ca10583..., qcbor ed42fa8d...), which is the D-19 proof for those two targets; `testbed-determinism` was identical again.

**Scope of the CI proof:** `TESTBED_SELECTION` is still `qcbor corejson`, so the image regenerated and compared only those two corpora. The other 14 were extracted from trees that the host fetched with plain git and that `mt testbed fetch --all` verified (commit, archive sha256 and tree sha256, all 16 `reused`), but they have not yet been regenerated from container-fetched trees. Plan 02-09 sets the selection to `--all`, and its `seeds` step then covers all 16.

## Task Commits

1. **Task 1: seed sources, seeding module, `mt testbed seeds` (TDD)**
   - RED `0d5550c` (test)
   - GREEN `9c55552` (feat)
   - fix found while preparing Task 2: `fa9dfae` (fix, absolute work directory)
2. **Task 2: seed sources per target, hand-made sets, corpora** - `3d6437c` (feat)
3. **Task 3: CI seeds check** - `7f16a10` (ci)

**Plan metadata:** the commit that adds this SUMMARY (docs).

## TDD Gate Compliance

Task 1 carried `tdd="true"` (the plan is `type: execute`). The RED commit `0d5550c` compiles against a stub `seeding.rs` (an empty selection and a `run_seeds_detailed` that always passes) and the schema without validation. All four target tests failed on their planned assertions, none on a compile, load or fixture error: `build_03_seed_sources_validate` (no issue reported for an unknown method), `build_03_seeding_extracts_with_provenance_from_a_fixture_tree` (0 seeds, expected 6), `build_03_seeding_reads_only_declared_sources` (no paths, expected `lib/ok.bin` and `lib/sub/nested.bin`), `build_03_seed_check_reports_drift` (`!run.summary.passed` failed). The GREEN commit `9c55552` makes all four pass. The `gsd_run check tdd-red-evidence` classifier was not run: it does not support cargo's report format and the plan is not `type: tdd`; the evidence is the unmodified `cargo test` output seen before GREEN. The relative work directory fix (`fa9dfae`) also went RED first: the new CLI test failed with `fatal: not a git repository: 'work/cache/demo/.../tree/.git'` before the one-line fix.

## Files Created/Modified

See key-files. The hand-made files and the corpora are data: `testbed/targets/<id>/hand-made/*.hex` (116 files) and `testbed/seeds/<id>/` (1623 files, 854109 bytes in total including `index.json` and `NOTICE`).

## Decisions Made

- **zlib has no extracted seeds.** The first version of the manifest used `test/example.c` and `test/infcover.c`; the 170 seeds that gave were the ASCII of messages and of hex text such as `"63 18 5"`, not zlib streams. They were removed and the manifest says why. The same reasoning is why lz4 and libfdt (GPL outside `lib/` and `libfdt/`) have no seed_source at all.
- **Bounds.** 8 MiB per source file and 2 MiB per zip archive. 2 MiB is above nanopb's `corpus.zip` (1.07 MiB, 3672 members, 189 members over 64 KiB) and keeps memory bounded by the pinned content. Residual risk: a pinned archive of that size can still expand to tens of megabytes in memory; no pinned archive does.
- **Entries.** Each source lists the entries its inputs exercise (zcbor's SUIT cases and decode tests only `zcbor_new_decode_state`; the others both of their entries). Hand-made seeds list every entry of the target.
- **`run_seeds_detailed`** returns the per-target difference lines next to the summary, because `StepSummary` rows are one line; `run_seeds` is the plan's signature and calls it.
- **coremqtt** got 10 hand-made seeds (the plan allows it for 1 to 9 extracted): it had only 5 extracted arrays, and MQTT 5 properties are not in them.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] A relative work directory broke the reuse of a git source**
- **Found during:** Task 2 (`mt testbed fetch --all` on the host-filled cache)
- **Issue:** the default work directory `.mt-work` is relative, but `fetch` and `verify_cached` run git with the cache directory as their working directory and pass paths built from the work directory, so every cached git source failed with `fatal: not a git repository`. CI uses the absolute `/work`, which is why it never showed.
- **Fix:** `testbed_options` in mt-cli makes the work directory absolute (`std::path::absolute`). New CLI test with a git source and a relative `--work-dir`, red before the fix.
- **Files modified:** crates/mt-cli/src/main.rs, crates/mt-cli/tests/build_03_seeds_cli.rs
- **Verification:** `mt testbed fetch --all` reports all 16 `reused`; full workspace tests green
- **Committed in:** fa9dfae

**2. [Rule 2 - Missing critical] Bounds and path rules the plan only implied**
- **Found during:** Task 1 (the 02-07 SUMMARY asked the caller to bound archive sizes)
- **Issue:** a zip archive's member count is unbounded, `.` as a seed_source would have made the whole tree (including `.git` and copyleft parts) a source, and symlinks inside nanopb and NimBLE could leave the tree.
- **Fix:** zip archives above 2 MiB are unsupported; seed_source paths may not be `.` or contain `.git` (manifest validation and a runtime check); a declared path that is missing or goes through a symlink is an error; `.git` and symlinks are skipped inside a walk.
- **Files modified:** crates/mt-capture/src/seeding.rs, crates/mt-capture/src/manifest.rs
- **Verification:** build_03_seed_sources_validate, build_03_seeding_reads_only_declared_sources
- **Committed in:** 9c55552 and 3d6437c (the 2 MiB constant replaced a first version tied to max_total_bytes)

**3. [Rule 3 - Blocking] New `Target.seed_source` field broke two struct literals**
- **Found during:** Task 1 (RED commit)
- **Fix:** `seed_source: Vec::new()` in `crates/mt-capture/tests/bed_01_fetch.rs` and `crates/mt-capture/tests/support/mod.rs`.
- **Committed in:** 0d5550c

### Plan deviations

**4. `seed_target` takes the target id as its first argument.** The plan lists `seed_target(tree, commit, target, hand_made_dir, caps)`; the id is needed for the index `target` field and the hand-made source path `testbed/targets/<id>/hand-made/<name>`.

**5. Extra CLI test file.** `crates/mt-cli/tests/build_03_seeds_cli.rs` is not in the plan's file list; it holds the exit-code, `--help` and relative-work-directory tests (the behaviour list asks for exit 1 through the CLI).

**6. The seeds step passes `--out /out`.** The plan's command has no `--out`; without it the summary file would go to `/work/reports`. Same result, consistent with the other steps. The step also has an evidence case in `evidence()` and `testbed/README.md` has a Seeds section.

**7. corejson's source is `test/unit-test` with extension `c`** (plan: `test/unit-test` c_string); the `.clusterfuzzlite` directory holds no corpus files at this commit, so nothing is declared for it.

**8. Commit trailers name Sonnet 5.5**, from the session's own attribution reminder, as the dispatch said to prefer; no model name appears elsewhere.

## Authentication Gates

None. `gh api` worked throughout; the host git and curl fetched through the session proxy.

## Issues Encountered

- **Stale binary.** After editing `seeding.rs` I rebuilt only `mt-capture`, then generated the corpora with the older `mt` binary. nanopb's committed seeds therefore lacked `corpus.zip` (the old rule skipped it) and `--check` through a fresh `cargo run` reported drift. Caught before the Task 2 commit by the pre-push gate (`cargo run --locked ... seeds --all --check`); regenerated with a fresh build and committed only that. Nothing wrong reached the branch, and the CI step would have caught it too.
- Quality limit, not fixed: `c_string` over whole test files includes some strings that are not parser inputs (error messages, names), for example in httpparser, nanopb and qcbor. The seeds are deterministic and marked as upstream test data, but nothing filters them semantically (D6 above).
- `xxd` is not on this host; plausibility checks of the hand-made files used small throwaway C and Python programs in the scratchpad, none of them in the repository.

## Known Stubs

None. The RED stubs of `seeding.rs` were replaced in `9c55552`.

## Threat Flags

None beyond the plan's register. The module reads files only below the tree root of a verified cache and writes only below the given seeds directory (T-02-33, T-02-34 mitigations are tested and, for two targets, proven in CI). The Python helpers are never run by the tool or the tests (T-02-35).

## User Setup Required

None.

## Next Phase Readiness

- Plan 02-09: set `TESTBED_SELECTION` to `--all`; the `seeds` step then regenerates and compares all 16 corpora in the image. If it reports drift for a target other than qcbor and corejson, a host-fetched tree differed from the container's: report it, do not regenerate by hand.
- Plan 02-10 is unaffected (no shared file).
- The founder may want to review the hand-made files (each is proposed) and decide whether noisy `c_string` sources should be narrowed.

---
*Phase: 02-test-bed-and-build-capture*
*Completed: 2026-10-13*

## Self-Check: PASSED

- Files: `crates/mt-capture/src/seeding.rs`, `crates/mt-capture/tests/build_03_seed_manifest.rs`, `crates/mt-capture/tests/build_03_committed_seeds.rs`, `crates/mt-cli/tests/build_03_seeds_cli.rs`, `research/testbed-seeds/{README.md,libfdt.py,lz4.py,zlib.py}`, `testbed/seeds/<id>/index.json` for all 16 ids, `.github/workflows/testbed.yml` exist (checked below).
- Commits: 0d5550c, 9c55552, fa9dfae, 3d6437c, 7f16a10 are ancestors of HEAD; `git rev-list --count 2423826..7f16a10` is 5.
- Acceptance: all four Task 1 tests, the six committed-seed tests and the four workflow static tests pass; `mt testbed seeds --help` lists `--check` and `--seeds-dir`; every zero-extracted target has at least 10 hand-made `.hex` files, each starting with a specification comment; `cargo test -p mt-toolchain --test repo_checks` passes; the latest testbed run is success for 7f16a10, the last commit touching its trigger paths.
- Verification before the push: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check` and `mt testbed seeds --all --check` each exited 0.
