---
phase: 02-test-bed-and-build-capture
verified: 2026-10-13T18:30:00Z
status: human_needed
score: 4/4 must-haves verified
covered_files:
  - .github/workflows/testbed.yml
  - .planning/phases/02-test-bed-and-build-capture/02-01-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-01-SUMMARY.md
  - .planning/phases/02-test-bed-and-build-capture/02-02-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-02-SUMMARY.md
  - .planning/phases/02-test-bed-and-build-capture/02-03-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-03-SUMMARY.md
  - .planning/phases/02-test-bed-and-build-capture/02-04-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-04-SUMMARY.md
  - .planning/phases/02-test-bed-and-build-capture/02-05-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-05-SUMMARY.md
  - .planning/phases/02-test-bed-and-build-capture/02-06-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-06-SUMMARY.md
  - .planning/phases/02-test-bed-and-build-capture/02-07-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-07-SUMMARY.md
  - .planning/phases/02-test-bed-and-build-capture/02-08-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-08-SUMMARY.md
  - .planning/phases/02-test-bed-and-build-capture/02-09-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-09-SUMMARY.md
  - .planning/phases/02-test-bed-and-build-capture/02-10-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-10-SUMMARY.md
  - container/Dockerfile
  - container/pins.toml
  - crates/mt-capture/src/capture.rs
  - crates/mt-capture/src/cc.rs
  - crates/mt-capture/src/droplog.rs
  - crates/mt-capture/src/elf.rs
  - crates/mt-capture/src/entries.rs
  - crates/mt-capture/src/fetch.rs
  - crates/mt-capture/src/lib.rs
  - crates/mt-capture/src/manifest.rs
  - crates/mt-capture/src/replay.rs
  - crates/mt-capture/src/report.rs
  - crates/mt-capture/src/seeding.rs
  - crates/mt-capture/src/testbed.rs
  - crates/mt-capture/src/treehash.rs
  - crates/mt-seeds/src/lib.rs
  - testbed/DROPPED.md
  - testbed/dropped.toml
  - testbed/manifest.toml
covered_digest: "v3:sha256:490957a92b93de5e951e319134938fc0e222dc711602b72e2068b92f9b5bae14"
behavior_unverified: 0
overrides_applied: 0
human_verification:
  - test: "Review the two drops (isotp whole target, paho/server variant) and confirm they are acceptable under D-11"
    expected: "Founder accepts the drops as logged, or reopens them (for example by letting the entry probe use the defining TU's include order so isotp.h need not be self-contained, review IN-07)"
    why_human: "testbed/dropped.toml records fix_attempted as host dry-run analysis plus run A, not a CI fix attempt. D-11 says one documented fix attempt; whether analysis counts, and whether losing isotp to a harness-probe limitation (not a target defect) is acceptable, is a founder call. SC3 itself is met: cause logged, 15 targets retained (>= 10)."
  - test: "Review the CMakeFiles/Makefile2 exclusion from configure-time hashes (capture.rs is_cmake_product, review WR-10)"
    expected: "Founder accepts that one CMake driver file is unobserved by the configure-hash and determinism evidence, or asks for a normalised check of its prerequisite lines"
    why_human: "The exclusion is documented in code, measured (zcbor app.dir/all line order), and replay does not read the file, but it narrows what the determinism evidence covers and the testbed.yml determinism comment does not list it. Whether that weakening is acceptable is a judgment call."
  - test: "Review the 97 committed hand-made seeds (coremqtt 10, libfdt 16, lz4 20, microcoap 16, paho 17, zlib 18) plus the 19 uncommitted isotp hand-made files"
    expected: "Hand-made seeds follow their intended wire formats; founder accepts, edits or rejects each set"
    why_human: "BUILD-03 and ROADMAP SC4 say hand-made seeds are 'proposed'. Count (>= 10), provenance and proposed=true are machine-checked. Whether the bytes are good inputs cannot be. The 116 figure in the 02-08 SUMMARY includes the 19 isotp files, which produce no corpus because isotp is dropped."
  - test: "Inspect the extracted httpparser (200 c_string), nanopb (190 c_string) and qcbor (102 c_string) seeds for strings that are not parser inputs"
    expected: "Narrow the c_string sources if error messages, identifiers or other non-input strings are present"
    why_human: "02-08 SUMMARY states string extraction from whole test files includes some non-input strings. Extraction follows declared sources and C escape rules (tested), but input usefulness is a human judgment."
  - test: "Decide whether the zip seed backend (zip 9.0.2 with deflate-flate2-zlib-rs) is acceptable for reading untrusted fuzz archives"
    expected: "Accept, or switch to an all-safe-Rust decompressor (flate2 miniz_oxide) and add the member-size budget from review WR-07"
    why_human: "mt-seeds forbids unsafe in its own code, but zlib-rs is a dependency with unsafe code. cargo deny (advisories, bans, licences, sources) is clean. The 02-07 SUMMARY itself says to reconsider if an all-safe decompressor is wanted. The decision is a risk/dependency call, not a goal failure."
---

# Phase 2: Test Bed and Build Capture Verification Report

**Phase Goal:** A pinned test bed of 10-20 real C parsers can be rebuilt exactly, per shipped configuration, for Cortex-M with GCC.
**Verified:** 2026-10-13T18:30:00Z
**Status:** human_needed
**Re-verification:** No, initial verification

## Goal Achievement

No truth failed. All four ROADMAP success criteria are backed by committed data, passing tests and CI evidence that I read from GitHub, not from the SUMMARYs. Status is `human_needed` only because five founder-judgment items remain open (listed under Human Verification Required). None of them contradicts a success criterion.

### Observable Truths

| # | Truth (ROADMAP SC) | Status | Evidence |
|---|--------------------|--------|----------|
| 1 | Manifest lists 10-20 open-source C parser/protocol targets, each pinned by commit with licence recorded and at least one buffer-in entry function | VERIFIED | `testbed/manifest.toml` has 16 `[target.*]` tables. `mt testbed validate --manifest testbed/manifest.toml` run locally: "valid (16 targets, 31 configurations)". 15 targets pin a 40-hex commit plus 64-hex tree hash. Mbed TLS is an archive source with sha256 plus tree hash and the tag commit for provenance. Every target has `spdx`, `elected` and `file` licence rows (all elected operands permissive). Every target declares 1 to 3 entries. CI fetch (run 38430137113, annotation `testbed-fetch`) verified all 15 retained tree hashes: I compared each reported commit and tree_sha256 to the manifest and all 15 match. CI capture reports `found:true,signature:true,header:true` for every entry of every retained pair. |
| 2 | Each target declares its shipped configurations; first-RTOS choice recorded as a decision | VERIFIED | Each target has `default` plus at most one variant (31 configurations over 16 targets). `[decisions] first_rtos = "deferred"` with reason and candidates in the manifest, mirrored as PROJECT.md D-18. Tests `bed_02_every_target_declares_default_and_at_most_one_variant` and `bed_02_first_rtos_decision_recorded` pass. CI `testbed-capture` shows `changes_code=true` for every one of the 15 retained variants, so the variants are real configurations and not duplicates. |
| 3 | For every target and configuration the capture writes `compile_commands.json` covering every TU, and replay reproduces the original output; unreproducible targets dropped with cause logged, leaving at least 10 | VERIFIED | Run 38430137113 on f0a36b8 (workflow `testbed`, conclusion success; jobs `mt` and `testbed` both success; `ci` 38430137092 and `container` 38430137007 on the same SHA also success). I read the job annotations. Capture: 29 pairs `captured`, 2 pairs `dropped (entry_mismatch)`. Replay: "result: passed (31 rows)", 29 `identical`, 2 `dropped`, every object row `normalisation_applied=false` (byte-identical, no normalisation was needed). Capture gates per pair: `compiler_allow_listed=true arch_ok=true undeclared=0 timestamp_macro=0`; `undeclared=0` is the gate that fails when an object is built but no compile command declares it (`undeclared_objects`, capture.rs:718). Determinism: re-ran capture and compared 72 files, "result: identical". Drop log: `testbed/dropped.toml` has 2 entries with cause, detail and ci_run; `testbed/DROPPED.md` has matching headings; `build_02_drop_log_keeps_at_least_ten_targets` passes. Retained: 15 of 16. Commits after f0a36b8 touch only `.planning/` files (diff checked), so the run is current for the shipped code. |
| 4 | Each target has a seed corpus from its own tests/fixtures, or at least 10 hand-made (proposed) seeds | VERIFIED | `testbed/seeds/<id>/index.json` exists for all 15 retained targets. I recomputed sha256 of every seed file against its index row: 0 mismatches. Every index commit equals the manifest commit. Extracted corpora: cjson 25, corejson 108, httpparser 200, mbedtls 200, nanopb 200, nimble 200, qcbor 200, tinycbor 200, zcbor 137, coremqtt 5 extracted plus 10 hand-made. Hand-made only: libfdt 16, lz4 20, microcoap 16, paho 17, zlib 18, all `proposed: true`, all at least 10. CI `seeds --check` (annotation `testbed-seeds`, step exit 0) regenerated the corpora in the image and found no drift. isotp has no corpus because it is dropped. |

**Score:** 4/4 roadmap truths verified (0 present, behavior-unverified)

### Deferred Items

None. Later phases (3 and up) do not take over any Phase 2 gap, and I found no gap to defer.

### Advisory (New Scope, Unevidenced)

Not applicable: initial verification, not re-verification.

### Required Artifacts

| Artifact | Expected | Status | Details |
| -------- | -------- | ------ | ------- |
| `testbed/manifest.toml` | 10-20 pinned targets, configs, RTOS decision | VERIFIED | 16 targets, 31 configs, validates |
| `testbed/dropped.toml`, `testbed/DROPPED.md` | Drop log with causes | VERIFIED | 2 entries, headings match, test-checked |
| `testbed/seeds/*` | Seed corpora, deterministic | VERIFIED | 15 corpora, hashes recomputed |
| `testbed/targets/*/hand-made/*.hex` | Hand-made proposed seeds | VERIFIED | 6 retained targets plus isotp (unused) |
| `crates/mt-capture/src/{manifest,fetch,treehash,capture,cc,elf,entries,replay,report,droplog,seeding,testbed}.rs` | Schema, fetch, capture, replay, drop log, seeds | VERIFIED | Substantive; no `todo!`, `unimplemented!`, `unwrap` or `expect` in `src/`; no TBD/FIXME/XXX markers |
| `crates/mt-seeds/src/*` | Seed extraction | VERIFIED | Wired into `mt-capture::seeding` |
| `.github/workflows/testbed.yml` | fetch, capture, replay, seeds, determinism steps | VERIFIED | Ran green, steps visible as annotations |
| `container/Dockerfile`, `container/pins.toml` | git, cmake, curl, bzip2, make pinned in the image | VERIFIED | `container` workflow green on f0a36b8; testbed image digest equals manifest `[image]` digest (test-checked) |

### Key Link Verification

| From | To | Via | Status | Details |
| ---- | -- | --- | ------ | ------- |
| manifest pins | fetched trees | tree hash check in `fetch` | WIRED | 15 of 15 CI tree hashes equal the manifest |
| capture report | replay | `capture.json` plus `compile_commands.json` `canonical_sha256` | WIRED | Replay produced 29 identical rows. See WR-02 and WR-04 below for the trust-boundary weaknesses |
| drop log | step summaries | `add_dropped` | WIRED | Dropped pairs print `dropped (<cause>)`, never count as passing |
| manifest seed sources | committed corpora | `mt testbed seeds --check` | WIRED | CI check passed with no drift |
| manifest image digest | `container/README.md` | `build_02_image_digest_matches_readme` | WIRED | Test passes |

### Data-Flow Trace (Level 4)

Not applicable: no rendered dynamic data. The data path is manifest to fetch to capture to replay to report, and the CI annotations show real values at each stage (real commits, 113 objects for Mbed TLS, real object hashes).

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
| -------- | ------- | ------ | ------ |
| Workspace tests | `cargo test --workspace --locked` | 250 passed, 0 failed | PASS |
| Format | `cargo fmt --check` | clean | PASS |
| Lints | `cargo clippy --all-targets --locked -- -D warnings` | clean | PASS |
| Dependencies | `cargo deny --locked check` | advisories, bans, licenses, sources ok | PASS |
| Committed manifest validates | `mt testbed validate --manifest testbed/manifest.toml` | valid (16 targets, 31 configurations) | PASS |
| Seed integrity | Python recomputation of sha256 for every seed file and commit match | 0 bad files, 15 of 15 commits equal | PASS |
| Fetch pins | compare CI `testbed-fetch` hashes to manifest | 15 of 15 equal | PASS |
| Real Arm build and replay | GitHub run 38430137113 annotations | conclusion success, 29 identical | PASS (CI only; no Docker in this session) |

### Probe Execution

No `scripts/*/tests/probe-*.sh` exist and none are declared by the plans: SKIPPED.

### Requirements Coverage

Every requirement ID in the PLAN frontmatter is in REQUIREMENTS.md and mapped to Phase 2. No orphaned requirements.

| Requirement | Source Plans | Description | Status | Evidence |
| ----------- | ------------ | ----------- | ------ | -------- |
| BED-01 | 02-01, 02-03, 02-05, 02-06, 02-09, 02-10 | 10-20 targets pinned by commit, licence recorded, buffer-in entry per target | SATISFIED | SC1 evidence; `bed_01_*` tests (56 `bed_*` tests in mt-capture) |
| BED-02 | 02-03, 02-05, 02-06, 02-09 | Declared configurations; first-RTOS decision recorded | SATISFIED | SC2 evidence; manifest `[decisions]`; PROJECT.md D-18 |
| BUILD-01 | 02-01, 02-02, 02-04, 02-05, 02-06, 02-09, 02-10 | Capture every compile command to `compile_commands.json` (Bear for Make, CMake export) | SATISFIED | CI capture 29 pairs; CI fixtures of real `compile_commands.json` parse in `build_01_ci_fixtures` |
| BUILD-02 | 02-02, 02-04, 02-06, 02-09, 02-10 | Replay reproduces original build; drops logged; at least 10 kept | SATISFIED | 29 identical, 15 retained, drop log with causes |
| BUILD-03 | 02-07, 02-08, 02-09 | Per-target seed corpus; at least 10 hand-made (proposed) where none | SATISFIED | SC4 evidence; seeds are proposals by design (human items 3 and 4) |

REQUIREMENTS.md still shows these five as "Pending" in the traceability table (lines 172-176) and unchecked in the list. That is bookkeeping for the orchestrator to flip after the verdict, not a code gap.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
| ---- | ---- | ------- | -------- | ------ |
| (none in phase source) | - | TBD/FIXME/XXX, TODO, stubs, unwrap/expect, "certified"/"compliant" | - | Scans clean |

Code review (02-REVIEW.md, 0 critical, 10 warning, 8 info, all disposition open) is carried here for completeness. None is a goal blocker; the reviewer found no case where the comparison reports a pass on a real divergence in normal operation. The ones most relevant to the evidence this phase produces, all WARNING:

- WR-02 and WR-04: replay trusts `capture.json` and object copies in the writable work directory, which the untrusted upstream build can reach; replay is not tied to the manifest pin. This weakens the claim that evidence cannot be tampered with by a hostile target, and matters before anyone relies on the evidence for a customer-facing claim.
- WR-05: the CI determinism comparison could report "identical" if `jq` failed on both files. The observed run compared 72 files and passed, and the `compared -eq 0` guard held, but the failure mode is real.
- WR-08: the 10-target floor is enforced by a unit test over the committed drop log, not by the CI step that produces the evidence.
- WR-10: see human item 2.

These are tracked in 02-REVIEW-DISPOSITION.md and should be dispositioned before the evidence is described to a customer.

### Human Verification Required

Classification of the five open founder-judgment items, per verification rules (any item the code cannot settle puts the status at `human_needed`; none is a failed truth):

| # | Item | Class | Reason |
|---|------|-------|--------|
| 1 | Two drops logged without a CI fix attempt | human_needed | SC3 is met (cause logged, 15 >= 10). Whether D-11's "one documented fix attempt" was honoured is policy interpretation; isotp is lost to a probe limitation (IN-07), which lowers the Core Value denominator |
| 2 | `CMakeFiles/Makefile2` exclusion | human_needed | Documented and measured, but narrows determinism evidence |
| 3 | 116 hand-made seeds are proposals | human_needed | Requirement text says "proposed"; quality is a human review. Correct count of committed hand-made seeds is 97; 19 more are isotp files with no corpus |
| 4 | Possible non-input strings in httpparser, nanopb, qcbor seeds | human_needed | Input usefulness not machine-checkable |
| 5 | zip backend is zlib-rs | human_needed (low) | Dependency/risk call; deny clean; not a goal criterion |

Detailed test, expected and why-human text is in the frontmatter `human_verification` list.

### Gaps Summary

No gaps. The phase goal is achieved in the codebase and by CI evidence: a pinned bed of 16 parsers (15 retained) builds for Cortex-M4F with arm-none-eabi GCC in the pinned image, captures compile commands per configuration (29 pairs), and the clean-copy replay is byte-identical for all 29 with no normalisation needed. The status is `human_needed` because of the five judgment items above and because real Arm build evidence exists only as GitHub run 38430137113 (no Docker in this verification session, so I could not re-run it; I read its conclusion and annotations through the GitHub API instead).

---

_Verified: 2026-10-13T18:30:00Z_
_Verifier: Claude (gsd-verifier)_
