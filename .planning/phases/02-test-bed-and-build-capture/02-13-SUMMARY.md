---
phase: 02-test-bed-and-build-capture
plan: 13
subsystem: testing
tags: [zip, flate2, miniz_oxide, deflate, seeds, decompression-budget, cargo-deny, supply-chain]

requires:
  - phase: 02-test-bed-and-build-capture
    provides: "plan 02-08 zip seed reader (mt-seeds zipsrc.rs) and plan 02-09 committed corpora"
provides:
  - "zip seed archives inflated by flate2 rust_backend (miniz_oxide, forbids unsafe code); zlib-rs and every C zlib backend absent from Cargo.lock; cargo deny clean"
  - "zip_members(bytes, max_member_bytes, max_total_bytes): a per-archive decompressed-size budget spent in archive order (WR-07)"
  - "MAX_ZIP_DECOMPRESSED_BYTES = 8 MiB passed by mt-capture seeding; committed corpora unchanged"
affects: [02-17 image proof of seeds --check with the new backend, phase-2 verification, any plan that adds crates to Cargo.lock]

actuals:
  tokens: 5700
  tasks: 3
  commits: 4
plan_head_before: 2ebab53190bfb6d67a295bd9161e39a7d7dc3dc0
plan_head_after: f16fca6a057de944976f75ab8e96009547631a4a

tech-stack:
  added: ["miniz_oxide 0.9.1", "adler2 2.0.1", "simd-adler32 0.3.10", "flate2 named in the workspace and mt-seeds (already locked, 1.1.10)"]
  patterns: ["lockfile and manifest asserted as text in a test so the test adds no dependency edge", "budget refusal proven by poisoning the refused members' bytes: a read would show as unsupported, a refusal as too_large"]

key-files:
  created: ["crates/mt-seeds/tests/build_03_zip_backend.rs"]
  modified: ["Cargo.toml", "Cargo.lock", "crates/mt-seeds/Cargo.toml", "crates/mt-seeds/src/zipsrc.rs", "crates/mt-seeds/tests/build_03_seed_selection.rs", "crates/mt-capture/src/seeding.rs"]

key-decisions:
  - "G-02-4: zip features are exactly deflate-flate2 and flate2 =1.1.10 with rust_backend is named in the workspace and in mt-seeds only to select the miniz_oxide inflater"
  - "The founder approved three added crates, not two: simd-adler32 0.3.10 enters because flate2 1.1.10 enables miniz_oxide's simd feature"
  - "Safe-Rust wording (founder-approved): the inflater (miniz_oxide) and adler2 forbid unsafe code; simd-adler32, zip's header parsing and flate2's buffer code contain unsafe code; no C code. The zip stack is never called entirely safe Rust"
  - "The Cargo.lock and Cargo.toml checks in the new test read the files as text; the toml crate is not added to mt-seeds, which would have changed Cargo.lock beyond the approved diff"
  - "WR-07 second fix (a cap on the member count) stays deferred to a founder decision; MAX_ZIP_ARCHIVE_BYTES (2 MiB) still bounds the central directory and so the member count"

patterns-established:
  - "Package legitimacy gate: lock change computed in a scratch copy, founder verifies each crate on its registry page (blocking-human), then the executor refuses any lock diff other than the approved one"

requirements-completed: [BUILD-03]

coverage:
  - id: D1
    description: "zip seed archives are inflated by miniz_oxide; zlib-rs, libz-rs-sys, libz-sys, libz-ng-sys and cloudflare-zlib-sys are absent from Cargo.lock; manifest features exact; cargo deny clean"
    requirement: BUILD-03
    verification:
      - kind: unit
        ref: "crates/mt-seeds/tests/build_03_zip_backend.rs#build_03_zip_inflates_with_miniz_oxide_only"
        status: pass
      - kind: unit
        ref: "crates/mt-seeds/tests/build_03_zip_backend.rs#build_03_zip_deflated_members_read_with_the_rust_backend"
        status: pass
      - kind: other
        ref: "cargo deny check"
        status: pass
    human_judgment: false
  - id: D2
    description: "each archive has a total decompressed-size budget spent in archive order, exact fit allowed, zero budget reads nothing, committed corpora unchanged"
    requirement: BUILD-03
    verification:
      - kind: unit
        ref: "crates/mt-seeds/tests/build_03_zip_backend.rs#build_03_zip_archive_has_a_total_decompressed_budget"
        status: pass
      - kind: unit
        ref: "crates/mt-seeds/tests/build_03_zip_backend.rs#build_03_zip_budget_boundary_is_exact"
        status: pass
      - kind: unit
        ref: "crates/mt-seeds/tests/build_03_zip_backend.rs#build_03_zip_budget_of_zero_reads_nothing"
        status: pass
      - kind: other
        ref: "cargo run --locked -q -p mt-cli -- testbed seeds --manifest testbed/manifest.toml --pins container/pins.toml --all --check (result: passed, 16 rows)"
        status: pass
    human_judgment: false
  - id: D3
    description: "the safe-Rust statement in code comments and this SUMMARY says exactly which crates forbid unsafe code and which contain it"
    verification: []
    human_judgment: true
    rationale: "Wording accuracy is a judgment the founder approved at the Task 1 checkpoint; no test can assert it. The counts behind it were measured with grep -w unsafe, comments included."

duration: 4min
completed: 2026-10-19
status: complete
---

# Phase 2 Plan 13: Zip Backend Switch and Decompressed Budget Summary

**zip seed archives are inflated by flate2's rust_backend (miniz_oxide, which forbids unsafe code) with zlib-rs gone from Cargo.lock, and each archive has an 8 MiB total decompressed-size budget (WR-07) that binds on no pinned archive.**

## Performance

- **Duration:** about 4 min of executor clock from the continuation (the founder checkpoint wait is not included)
- **Started:** 2026-10-19T21:11:55Z
- **Completed:** 2026-10-19T21:16Z
- **Tasks:** 3 (Task 1 founder checkpoint, Task 2 tracer, Task 3 budget)
- **Files modified:** 7 (6 modified, 1 created), plus planning files

## Accomplishments

- Cargo.lock now carries miniz_oxide 0.9.1, adler2 2.0.1 and simd-adler32 0.3.10 and no longer carries zlib-rs 0.6.8; `cargo tree -p mt-seeds -e normal` shows flate2 over crc32fast and miniz_oxide (adler2, simd-adler32); `cargo deny check` reports advisories, bans, licenses and sources ok.
- `zip_members` takes a total budget, spends it in archive order, allows an exact fit, refuses over-budget members unread (counted `too_large`) and bounds every read one byte past the smaller of the member limit and what is left.
- `mt testbed seeds --all --check` passes on all 16 rows with the new backend and the 8 MiB budget: no committed corpus moved. The image proof of the same check is plan 02-17's run.

## Founder package legitimacy gate (Task 1)

The founder typed "Approved" at the `checkpoint:human-verify` (`gate="blocking-human"`) for all three added crates by name, version and checksum, and for the amended safe-Rust wording. The lock change Task 2 applied equals the approved list exactly (diff checked line by line before building):

| Change | Package | Version | Checksum | Licence |
|--------|---------|---------|----------|---------|
| added | miniz_oxide | 0.9.1 (plan text said 0.8.x) | b63fbc4a50860e98e7b2aa7804ded1db5cbc3aff9193adaff57a6931bf7c4b4c | MIT OR Zlib OR Apache-2.0 |
| added | adler2 | 2.0.1 | 320119579fcad9c21884f5c4861d16174d0e06250625266f50fe6898340abefa | 0BSD OR MIT OR Apache-2.0 |
| added | simd-adler32 | 0.3.10 (not named in the plan; flate2 1.1.10 enables miniz_oxide's `simd` feature) | 3a219298ac11a56ea9a6d2120044824d6f01aeb034955e7af7bc16858527deea | MIT |
| removed | zlib-rs | 0.6.8 | b268e58e7c693d7c271f93ffc4ba3b380412554231c85bf61ca7af91042a4112 | - |

Non-package lines: flate2's dependency list goes from `zlib-rs` to `crc32fast` and `miniz_oxide` (crc32fast 1.5.2 already locked); mt-seeds' dependency list gains `flate2`. Nothing else in Cargo.lock changed. Task 1's own automated check (manifests and lock clean before approval) passed at the start of this continuation.

## What is and is not safe Rust

Approved wording, to be used instead of any narrower or wider claim: the inflater (miniz_oxide) and adler2 forbid unsafe code; simd-adler32 (SIMD Adler-32 checksum, about 36 unsafe occurrences in its SIMD intrinsics; not checked whether zip's raw-deflate path calls it), zip's header parsing and flate2's buffer code contain unsafe code; there is no C code. The zip reading stack is not entirely safe Rust.

Measured in Task 1 (`grep -w unsafe` over `src/`, counts include comments): miniz_oxide 0.9.1 has `#![forbid(unsafe_code)]` at src/lib.rs:25; adler2 2.0.1 at src/lib.rs:16; simd-adler32 0.3.10 has 36 occurrences (src/imp/*); flate2 1.1.10 has 37 (mem.rs 10, lib.rs 1, ffi/mod.rs 1, ffi/c.rs 25; the ffi modules are not compiled with rust_backend); zip 9.0.2 has 27; crc32fast 1.5.2 has 15.

## TDD evidence

**Tracer (Task 2).**
- RED `ef4fc35`: `cargo test --locked -p mt-seeds --test build_03_zip_backend` against the unchanged lock: `build_03_zip_inflates_with_miniz_oxide_only ... FAILED`, panic at tests/build_03_zip_backend.rs:96 `Cargo.lock must hold miniz_oxide`; `test result: FAILED. 1 passed; 1 failed`. Semantic assessment: the target test ran and failed on its planned first assertion for the intended reason (the lock still holds zlib-rs and not miniz_oxide). `build_03_zip_deflated_members_read_with_the_rust_backend` passed at RED: it is a guard that the deflate round trip works before and after the switch, not a red test. No manifest or src file was in the RED commit.
- GREEN `25033b9`: both tests pass; `cargo test -p mt-seeds --test build_03_zip_backend --test build_03_seed_selection` ok (8 + 2 tests); `cargo deny check` ok.

**Budget (Task 3).**
- RED `35af076`: the three new tests call `zip_members(bytes, max_member_bytes, max_total_bytes)`; against the unchanged function, `error[E0061]: this function takes 2 arguments but 3 arguments were supplied` at tests/build_03_zip_backend.rs lines 229, 243, 252 and 265 (`unexpected argument #3`), `could not compile mt-seeds (test "build_03_zip_backend") due to 4 previous errors`. The RED commit holds test code only.
- GREEN `f16fca6`: all three pass. Refusal is proven, not assumed: in the 10 x 1000-byte test the six over-budget members' stored bytes are corrupted in the archive, so a read of one would fail its CRC-32 check and count `unsupported`; the test asserts `unsupported == 0` and `too_large == 6`.

## Task Commits

1. **Task 1: founder verifies every added crate** - no commit (changes no repository file); APPROVED by the founder
2. **Task 2 (tracer): zip members inflated through miniz_oxide** - `ef4fc35` (test), `25033b9` (feat)
3. **Task 3: total decompressed-size budget** - `35af076` (test), `f16fca6` (feat)

**Plan metadata:** the docs commit that carries this SUMMARY (docs: complete plan)

## Files Created/Modified

- `crates/mt-seeds/tests/build_03_zip_backend.rs` - lockfile and manifest backend check (text-parsed), deflate round trip, three budget tests
- `Cargo.toml` - zip features `["deflate-flate2"]`, flate2 `=1.1.10` with `rust_backend`, comments with the approved wording
- `Cargo.lock` - the approved diff only
- `crates/mt-seeds/Cargo.toml` - `flate2.workspace = true`
- `crates/mt-seeds/src/zipsrc.rs` - `max_total_bytes`, running total, docs for `too_large` and the budget
- `crates/mt-seeds/tests/build_03_seed_selection.rs` - five `zip_members` calls pass `u64::MAX`
- `crates/mt-capture/src/seeding.rs` - `MAX_ZIP_DECOMPRESSED_BYTES`, module and `MAX_ZIP_ARCHIVE_BYTES` docs corrected

## Decisions Made

See key-decisions. In short: the backend is selected by naming flate2 once; the lock/manifest test reads text so no unapproved dependency edge appears; the budget is 8 MiB per archive (nanopb's corpus.zip reads about 2.86 MB within the member limit).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Lock/manifest test parses text instead of using the toml crate**
- **Found during:** Task 2 (RED)
- **Issue:** the plan says Cargo.lock is "parsed with the toml crate". mt-seeds has no toml dependency; adding it as a dev-dependency would add a `toml` edge to mt-seeds in Cargo.lock, a change the founder did not approve ("if the regenerated Cargo.lock adds or removes anything else, STOP").
- **Fix:** `lock_packages()` and `workspace_dependency_line()` read the flat `[[package]]` and `[workspace.dependencies]` text; the assertions are unchanged.
- **Files modified:** crates/mt-seeds/tests/build_03_zip_backend.rs
- **Verification:** RED fails on the planned assertion; GREEN passes; Cargo.lock diff equals the approved list.
- **Committed in:** ef4fc35

**2. [Rule 1 - Plan detail] Five existing call sites, not four, passed `u64::MAX`**
- **Found during:** Task 3 (GREEN)
- **Issue:** the plan counts four `zip_members` calls in build_03_seed_selection.rs; there are five (the "not a zip file" error call at line 439 is the fifth).
- **Fix:** all five pass `u64::MAX`; their assertions are unchanged.
- **Committed in:** f16fca6

---

**Total deviations:** 2 auto-fixed (1 blocking, 1 plan-detail). **Impact on plan:** none on scope or on the approved lock.

## Known behaviours worth recording

- A zero-byte member fits a zero budget (declared size 0 is not over 0), so "budget 0 keeps no member" holds for non-empty members only; the test uses non-empty members. This follows from "an exact fit is allowed".
- A member whose header understates its size is caught by the bounded read (one byte past the smaller of the member limit and what is left); the bytes already read stay spent, so the bytes read from one archive never pass the budget by more than that one byte. Only the declared-size refusal path is covered by a test; the lying-header path is not exercised by a dedicated test.

## Deferred

- WR-07's second fix, a cap on the member count (`archive.len()`, for example 10 000), is not in this plan: the founder's G-02-4 decision asked for the total budget only. MAX_ZIP_ARCHIVE_BYTES (2 MiB) still bounds the central directory and so the member count. Left to a founder decision.
- Image proof: `seeds --all --check` inside the pinned image with the new backend is plan 02-17's run. This plan checked it on the host only.
- Cargo.lock and the pre-merge `container version check` were not part of this plan's verification list and were not run.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Plan 02-17 can run the image proof of `seeds --all --check` against this backend. No blockers. Threat T-02-46 (added crates), T-02-47 (decompression bomb) and T-02-48 (inflater memory safety, residual unsafe recorded not claimed away) are mitigated as planned; no new threat surface beyond the plan's register.

## Self-Check: PASSED

- Files found: build_03_zip_backend.rs, zipsrc.rs, seeding.rs, Cargo.toml, Cargo.lock, mt-seeds/Cargo.toml
- Commits found as ancestors of HEAD: ef4fc35, 25033b9, 35af076, f16fca6
- `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check` and `mt testbed seeds --all --check` all exit 0 at f16fca6
- Stub scan of created/modified files: none

---
*Phase: 02-test-bed-and-build-capture*
*Completed: 2026-10-19*
