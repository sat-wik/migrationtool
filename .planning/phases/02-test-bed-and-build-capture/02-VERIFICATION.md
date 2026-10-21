---
phase: 02-test-bed-and-build-capture
verified: 2026-10-21T02:34:52Z
status: passed
score: 12/12 must-haves verified
covered_files:
  - .github/workflows/container.yml
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
  - .planning/phases/02-test-bed-and-build-capture/02-11-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-11-SUMMARY.md
  - .planning/phases/02-test-bed-and-build-capture/02-12-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-12-SUMMARY.md
  - .planning/phases/02-test-bed-and-build-capture/02-13-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-13-SUMMARY.md
  - .planning/phases/02-test-bed-and-build-capture/02-14-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-14-SUMMARY.md
  - .planning/phases/02-test-bed-and-build-capture/02-15-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-15-SUMMARY.md
  - .planning/phases/02-test-bed-and-build-capture/02-16-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-16-SUMMARY.md
  - .planning/phases/02-test-bed-and-build-capture/02-17-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-17-SUMMARY.md
  - .planning/phases/02-test-bed-and-build-capture/02-18-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-18-SUMMARY.md
  - .planning/phases/02-test-bed-and-build-capture/02-19-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-19-SUMMARY.md
  - .planning/phases/02-test-bed-and-build-capture/02-20-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-20-SUMMARY.md
  - .planning/phases/02-test-bed-and-build-capture/02-21-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-21-SUMMARY.md
  - .planning/phases/02-test-bed-and-build-capture/02-22-PLAN.md
  - .planning/phases/02-test-bed-and-build-capture/02-22-SUMMARY.md
  - Cargo.lock
  - Cargo.toml
  - container/Dockerfile
  - container/pins.toml
  - crates/mt-capture/src/capture.rs
  - crates/mt-capture/src/cc.rs
  - crates/mt-capture/src/compare.rs
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
  - crates/mt-capture/tests/build_01_capture_drivers.rs
  - crates/mt-capture/tests/build_01_capture_stages.rs
  - crates/mt-capture/tests/build_01_capture_verdict.rs
  - crates/mt-capture/tests/build_01_compile_commands.rs
  - crates/mt-capture/tests/build_01_gate_bypasses.rs
  - crates/mt-capture/tests/build_01_toolchain_evidence.rs
  - crates/mt-capture/tests/build_02_replay.rs
  - crates/mt-capture/tests/build_02_replay_bypasses.rs
  - crates/mt-capture/tests/build_02_workflow_static.rs
  - crates/mt-cli/src/main.rs
  - crates/mt-cli/tests/build_01_capture_stages_cli.rs
  - crates/mt-seeds/Cargo.toml
  - crates/mt-seeds/src/hex.rs
  - crates/mt-seeds/src/index.rs
  - crates/mt-seeds/src/lexer.rs
  - crates/mt-seeds/src/lib.rs
  - crates/mt-seeds/src/select.rs
  - crates/mt-seeds/src/zipsrc.rs
  - testbed/DROPPED.md
  - testbed/README.md
  - testbed/dropped.toml
  - testbed/manifest.toml
covered_digest: "v3:sha256:467a1c78753950b183f3d0825ee08df4beb65d5ee1edc58c40124a8859fd2cb0"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: human_needed
  previous_score: 8/8
  gaps_closed:
    - "G-02-5: record stage refuses a built:false record that names no failure (build_record_inconsistent), captured needs passed gates and a non-empty object set (gates_not_passed), toolchain file compared with the pins (02-20, strengthened by 02-22)"
    - "G-02-6: compile-command gate refuses helper-executing GCC driver options at capture and at replay before any spawn; T-02-67 registered (02-20, replaced by the allow-list of 02-21)"
    - "G-02-7: AR-02-03 and the capture.rs module doc state what a forged build record can and cannot do, citing tests (02-20, updated by 02-21 and 02-22)"
    - "G-02-8: one operand-aware allow-list walk at capture and replay (02-21); measured ALLOWED_OPTIONS and SHIPPED_SPECS (02-21); read-only toolchain file and toolchain_unreadable cause (02-22); no retained pair refused; T-02-67 and T-02-70 closed"
    - "Baseline human items 1-3 (WR-11/WR-12 triage, driver-flag registration, AR-02-03 confirmation): decided by the founder in 02-UAT.md tests 6-8 and delivered as G-02-5..G-02-7"
  gaps_remaining: []
  regressions: []
advisory:
  - finding: "entries.rs filter_flags is a second reader of recorded compile options, separate from cc::walk. It keeps -I/-isystem/-iquote/-include with a separate operand, so an argument the walk read as the operand of -MT/-MQ/-MF/-o (for example -MT -include x.h) would reach the clang entry probe as -include x.h"
    category: security
    reason: "Found by reading the code only. filter_flags keeps only parse options (-D, -U, -I, -isystem, -iquote, -include, -std=, -m*, three char/enum flags), so the most it can do is change how clang parses upstream headers in the entry check. It cannot run a helper or write a file. entries.rs was last changed in 02-11, before the baseline. G-02-8 names the -c test, derive_output and the write-path check, not the entry probe. To resolve: derive the probe flags from walk_arguments, or add a test that filter_flags and the walk agree on operands"
    evidence_status: "none provided (no reproduction run)"
  - finding: "The local end-state toolchain mode (capture_record without --read-only-toolchain, and the single-process capture()) still cannot see a toolchain file that was changed and then restored (review IN-16). Replay does not refuse a report with toolchain_file_read_only false"
    category: security
    reason: "Production CI always uses read-only mode (testbed.yml:454 and :517, pinned by build_02_record_steps_require_the_read_only_toolchain). The weaker mode is labelled in every report and in the module doc. Resolve if local reports ever count as evidence"
    evidence_status: "none provided"
  - finding: "container.yml toolchain-ro-control and 02-SECURITY.md T-02-76 say the control uses run_build's docker flags. It uses only the flags that bear on writability (review IN-17)"
    category: other
    reason: "Wording only: the read-only result depends on --read-only and the :ro mount, which the control does use"
    evidence_status: "none provided"
  - finding: ".planning/ROADMAP.md still shows '- [ ] Phase 2' (line 30), and its Phase 2 block says '20/20 plans executed' and lists plans only up to 02-20 (lines 92, 139). Plans 02-21 and 02-22 are not listed"
    category: other
    reason: "Orchestrator state, updated at phase completion"
    evidence_status: "n/a (documentation)"
  - finding: "Open review items carried unchanged from before this gap closure: WR-01, WR-03, WR-06, WR-09, WR-07 (deferred), IN-01..IN-06, IN-08..IN-12, IN-16, IN-17 (0 critical)"
    category: other
    reason: "Tracked in 02-REVIEW-DISPOSITION.md. None contradicts a success criterion or a UAT gap"
    evidence_status: "none provided"
human_verification:
  - test: "Resolve 02-21 prohibition 1: no unmeasured spelling in ALLOWED_OPTIONS or SHIPPED_SPECS (exceptions: founder depfile flags, one CI-only flag of the named families with CI evidence)"
    result: "confirmed by the founder on 2026-10-21"
    expected: "Founder marks it resolved, or names a spelling that should not be on the list"
    why_human: "A judgment-tier prohibition with status active in 02-21-PLAN.md. Advisory verifier verdict (not authoritative): holds. cc.rs:290-347 has 56 spellings: 45 measured exact spellings, -Wmissing-variable-declarations (CI-only, testbed run 39453293771 failed on cjson with option_not_allowed, added test first in 1270139/2940852), -MD/-MMD/-MP/-MF/-MT/-MQ (founder), and -o/-I/-D (measured) plus -specs= (founder). SHIPPED_SPECS (cc.rs:365-379) equals the 13 names of the arm-specs notice of container run 39453293834, which I counted. No carrier, long spelling, @ or other file-naming option is on the list"
  - test: "Resolve 02-21 prohibition 2: no existing gate, comparison, limit, verdict rule or floor was relaxed"
    result: "confirmed by the founder on 2026-10-21"
    expected: "Founder marks it resolved"
    why_human: "Judgment-tier, status active. Advisory verdict: holds. The deny-list became an allow-list, which is stricter. Every changed 02-20 expectation in 02-21-SUMMARY moves an accepted flag to refused. The one cause change (soft-float in build_01_capture_drivers_fail_closed, arch_gate to option_not_allowed) leaves the arch gate's own end-to-end test (build_01_record_stage_refuses_an_object_built_for_another_abi) passing. The notices of the final run equal 02-20's line for line (same 30 captured, 31 replay rows, drop log and floor)"
  - test: "Resolve 02-21 prohibition 3: push only to claude/gsd-discuss-phase-1-mzdw5v, no force-push, never red, at most 3 round trips"
    result: "confirmed by the founder on 2026-10-21"
    expected: "Founder marks it resolved"
    why_human: "Judgment-tier, status active. Advisory verdict: holds. Every Actions run since 2026-10-20T15:00Z is on that branch. 02-21 used 2 testbed round trips (4818d97 failure, 44a3dd6 success). Every pushed head is an ancestor of HEAD. ci passed on every pushed head (4818d97, 44a3dd6, 79fe192, ae3df6c). No run exists for a RED-only commit. Force-push cannot be fully ruled out from run history alone"
  - test: "Resolve 02-22 prohibition 1: the toolchain directory is never mounted writable, inside the run directory or under a writable mount; --toolchain-dir and --read-only-toolchain are never dropped; CI never falls back to the end-state comparison"
    result: "confirmed by the founder on 2026-10-21"
    expected: "Founder marks it resolved"
    why_human: "Judgment-tier, status active. Advisory verdict: holds. testbed.yml:243 mounts $work/toolchain/$id/$cfg at /opt/mt/toolchain:ro. The writable mounts are $scratch/work and the run directory (lines 240, 242). Line 244 passes --toolchain-dir and lines 454 and 517 pass --read-only-toolchain. build_02_untrusted_builds_cannot_write_the_evidence and build_02_record_steps_require_the_read_only_toolchain pass. The corejson report of run 39528941371 reads toolchain_file_read_only true"
  - test: "Resolve 02-22 prohibition 2: no relaxation of gates, comparison, limits, the 02-21 allow-list, the verdict rule or the floor"
    result: "confirmed by the founder on 2026-10-21"
    expected: "Founder marks it resolved"
    why_human: "Judgment-tier, status active. Advisory verdict: holds. cc.rs and replay.rs are unchanged by 02-22 (git log), settle_verdict and the build-record check are unchanged, and the notices equal 02-20's. 02-22 adds causes (toolchain_unreadable, more toolchain_changed cases) and removes none"
  - test: "Resolve 02-22 prohibition 3: push only to the session branch, no force-push, never red, at most 3 round trips"
    result: "confirmed by the founder on 2026-10-21"
    expected: "Founder marks it resolved"
    why_human: "Judgment-tier, status active. Advisory verdict: holds. 02-22 used 1 testbed round trip (9273ef8). The ci, testbed and container runs on 9273ef8 all passed. Later pushes (7c55dbe..bd0df6a) touch only .planning/ and their ci runs passed"
---

# Phase 2: Test Bed and Build Capture Verification Report (re-verification)

**Phase Goal:** A pinned test bed of 10-20 real C parsers can be rebuilt exactly, per shipped configuration, for Cortex-M with GCC.
**Verified:** 2026-10-21T02:34:52Z
**Status:** passed (founder confirmed the six judgment-tier prohibitions of 02-21 and 02-22 on 2026-10-21; they are now `status: resolved`)
**Re-verification:** Yes, after gap-closure plans 02-20, 02-21 and 02-22. This file replaces the report of 2026-10-20T02:40:00Z.

## Goal Achievement

No truth failed. The four ROADMAP success criteria still hold. The four new UAT gaps (G-02-5 to G-02-8) have every `missing` item delivered in code, with passing tests, and CI runs that I read from GitHub myself rather than from the SUMMARYs.

The three human items of the baseline are gone: the founder decided them in 02-UAT.md (tests 6, 7, 8), and they became G-02-5, G-02-6 and G-02-7, which are now delivered.

The status is `human_needed`, not `passed`, for one reason. The six `prohibitions` in the 02-21 and 02-22 PLAN frontmatter are judgment-tier and still `status: active`. The verifier must not resolve them on its own. My advisory verdict on each is "holds", with evidence (see Human Verification Required).

Evidence base:
- CI, final code commit `9273ef82979df478d7e7bba80bd27a8345b0dff0` (push, branch `claude/gsd-discuss-phase-1-mzdw5v`):
  - testbed 39528941371: success (jobs `mt` 118990101488 and `testbed` 118990405406);
  - container 39528941352: success (image-a 118990397017; step 10 "List the spec files..." and step 11 "Prove a read-only toolchain mount cannot be written (WR-12)" both success);
  - ci 39528941315: success (Format, Clippy, Test, Dependency policy, thumbv7em smoke builds).
- Latest ci run on HEAD `bd0df6aaf618e2603161592fa9585227e86b6926`: run 39532995983, success.
- Currency: `git diff --stat 9273ef8 HEAD -- . ':!.planning'` is empty. The five commits after 9273ef8 touch only `.planning/`.
- Notices:
  - `diff <(sed '1,4d' 02-20-CI-NOTICES.md) <(sed '1,4d' 02-22-CI-NOTICES.md)` is empty, so the files are equal from line 5 on.
  - The same holds for 02-21 against 02-20, and for 02-22 against 02-19.
  - I fetched the 41 annotations of job 118990405406 with `gh api`. All are notice level, and every section (image, validate, fetch, seeds, build, capture, replay, determinism) occurs verbatim in 02-22-CI-NOTICES.md.
- Local, run by me:
  - `CARGO_INCREMENTAL=0 cargo test --workspace --locked --offline`: exit 0, 352 passed, 0 failed, 2 ignored, 45 result lines.
  - The 2 ignored tests are the local-capture proofs, which I then ran explicitly (see Behavioral Spot-Checks).
  - Arm GCC 13.2.1, Bear, CMake, clang and make are installed at the paths `local_arm_tools()` checks, and two live tests run with `--nocapture` printed no `skipping:` line.

### Observable Truths

ROADMAP success criteria first (the contract), then the eight UAT gap truths. Rows 1-8 are regression-checked. Rows 9-12 are new and fully verified.

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SC1: manifest lists 10-20 C targets pinned by commit, licence recorded, at least one buffer-in entry | VERIFIED (regression) | `testbed/` unchanged since fb04867. `target/debug/mt testbed validate --manifest testbed/manifest.toml`: "valid (16 targets, 31 configurations)". CI fetch section is equal to 02-19's (16 rows, tree hashes). |
| 2 | SC2: declared configurations; first-RTOS decision recorded | VERIFIED (regression) | Manifest unchanged. CI capture shows 14 variants `changes_code true` (equal to 02-20). |
| 3 | SC3: capture writes compile_commands.json per pair and replay reproduces it; unreproducible pairs dropped with cause; at least 10 kept | VERIFIED | Annotations of testbed 39528941371, read by me: capture `result: passed (46 rows)` with 30 `captured`; `paho/server dropped (entry_mismatch)`. Replay `result: passed (31 rows)` with 30 `identical` and 1 dropped. Determinism: `rerun capture exit code: 0`, `compared 74 files`, `result: identical`, `compare exit code: 0`. All 30 pairs now pass the stricter 02-21 allow-list and the 02-22 read-only toolchain check. |
| 4 | SC4: per-target seed corpus from tests and fixtures, or at least 10 hand-made seeds | VERIFIED (regression) | I recomputed sha256 over `testbed/seeds/*/index.json`: 1467 rows, 0 bad, 16 corpora, smallest 15. CI seeds `result: passed (16 rows)`. |
| 5 | G-02-1: entry probe reproduces the defining TU's include prefix; isotp retained | VERIFIED (regression) | `entries.rs` unchanged since 02-11. CI: `isotp/default identical`. The `bed_01_entry_*` tests pass in the workspace run. |
| 6 | G-02-2: Makefile2 in normalised form; exact `is_cmake_product`; determinism comment lists every exclusion | VERIFIED (regression) | CI notices (Makefile2 hashes) equal 02-19's. `build_02_determinism_comment_lists_every_exclusion` and `build_02_configure_evidence` tests pass. |
| 7 | G-02-3: c_string seeds for httpparser, nanopb, qcbor only from input-bearing arrays and fixtures | VERIFIED (regression) | `testbed/manifest.toml` and the seeds are unchanged since baseline. The `build_03_seed_*` tests pass. |
| 8 | G-02-4: zip seeds through a safe-Rust inflater under a total budget | VERIFIED (regression) | `Cargo.toml`, `Cargo.lock` and `crates/mt-seeds` are unchanged since fb04867. `build_03_zip_*` pass. The ci Dependency policy step passed on 9273ef8. |
| 9 | G-02-5: the record stage never reports captured unless gates ran and passed; refuses a built:false record that names no failure; the toolchain file the build used equals the pinned text | VERIFIED | **Missing 1:**<br>- `capture.rs:2565-2585`: `names_its_failure` sets `build_record_inconsistent` and `not_proven`.<br>- `settle_verdict` (`capture.rs:1485-1507`) gives `captured` only with no cause, `compiler_allow_listed`, `arch_ok` and non-empty objects, else `gates_not_passed`.<br>- Tests: `build_01_captured_verdict_needs_passed_gates_and_objects` (build_01_capture_verdict.rs:29), `build_01_record_stage_refuses_a_record_that_did_not_build` (build_01_capture_stages.rs:817). Both pass, and the RED commit 27908f2 is test-only.<br>**Missing 2:**<br>- `check_toolchain_file` (`capture.rs:2106-2146`) reads through `read_run_file` and maps the result through `toolchain_file_finding` (`capture.rs:1337-1366`): a byte mismatch is `toolchain_changed`, a link is `run_dir_symlink`.<br>- Tests: `build_01_record_stage_refuses_a_changed_toolchain_file` (:891), plus the 02-22 read-only tests in row 12.<br>**Missing 3:**<br>- module doc `capture.rs:107-148`, checked against the code paths above. |
| 10 | G-02-6: the gate refuses helper-executing GCC driver options at capture and at replay before any spawn; threat registered | VERIFIED | **Missing 1:**<br>- Since 02-21, anything not on `ALLOWED_OPTIONS` is `OptionNotAllowed` (`cc.rs:462`). `@` anywhere after argv[0] is `HelperOption` (`cc.rs:398`). `-specs` is allowed only for `SHIPPED_SPECS` names (`cc.rs:437-447`).<br>- Capture: `canonical_entry` calls `check_command` (`cc.rs:733`). Replay: `check_entry` (`cc.rs:656-657`), called from `validate` (`replay.rs:149`), which is the first call in `replay()` (`replay.rs:234`), before any delete, copy or spawn.<br>- Per-option tests at capture, in `build_01_rejects_helper_executing_driver_options` (build_01_compile_commands.rs:748): `-wrapper`, `-B`, `--prefix`, `-specs=`, `-specs`, `--specs`, `-fplugin=`, `-fplugin-arg-*`, `-iplugindir=`, `-fcompare-debug=`, `@file`, `-Wp,` and `-Xpreprocessor`. `--sysroot=` and `-isysroot` are refused in `build_01_allow_list_refuses_everything_else` (:562).<br>- At replay, `build_02_replay_refuses_helper_executing_driver_options` (build_02_replay.rs:503) covers the same set.<br>- Record stage: `build_01_record_stage_refuses_a_helper_option_in_the_export` (build_01_capture_stages.rs:583).<br>- No retained pair is refused: testbed 39528941371 shows 30 of 30 captured.<br>**Missing 2:**<br>- 02-SECURITY.md T-02-67: medium, `closed`, verified by the 2026-10-21 audit (79/79, `threats_open: 0`). |
| 11 | G-02-7: AR-02-03's rationale states exactly what a forged build record can and cannot do after the G-02-5 fix | VERIFIED | 02-SECURITY.md AR-02-03 and the module doc at `capture.rs:111-148` agree. I checked each "can no longer" claim against code and a passing test:<br>- `build_record_inconsistent`, `gates_not_passed`;<br>- `option_not_allowed`, `helper_option`, `write_redirect`, `outside_run_root`, `not_compile_only` from `check_command`;<br>- `toolchain_changed` in both toolchain modes;<br>- `build_record_mismatch` (`build_01_record_stage_refuses_a_record_for_another_pair` passes).<br>Each "can still" claim (verdict build_failed or not_proven, falsified steps and causes, the `before` listing) matches what `record_stage` copies from the record (`capture.rs:2575-2580`). |
| 12 | G-02-8: only measured options pass, through one operand-aware walk, so no spelling, carrier or operand trick reaches the compiler, assembler or linker at capture or replay; the toolchain evidence covers what configure read | VERIFIED | Item by item in the reconciliation table below:<br>- `walk` (`cc.rs:394-467`) is the only reader for compile-only (`check_command` :634), `derive_output` (:531) and `check_write_paths` (:581), at both ends.<br>- The allow-list is `cc.rs:290-347`.<br>- CR-01, CR-02 and CR-03 were reproduced RED first in test-only commit 68e0bc7, then live in 148715f.<br>- WR-12: read-only mount `testbed.yml:243`, build-stage check `capture.rs:2410-2433`, record-stage requirement `capture.rs:2131-2138`. The live control in container 39528941352 failed every write to the `:ro` mount, CMake `file(APPEND)` included, with the hash unchanged. The writable control accepted the writes. The corejson report of testbed 39528941371 (annotation parts joined by me) has `verdict captured`, `causes []`, `toolchain_file_read_only true` and `-DCMAKE_TOOLCHAIN_FILE=/opt/mt/toolchain/toolchain.cmake`.<br>- WR-14: `toolchain_unreadable` at `capture.rs:1363` and `:2171`. `check_toolchain_file` and `check_configured_toolchain` return `()`. |

**Score:** 12/12 truths verified (0 present, behavior-unverified).

The behavior-dependent truths (9, 10 and 12: verdict transitions, refusals before spawn, the read-only mount) each have a named behavioral test that passes. The live ones (record-stage tests with real Make, CMake and Bear, and the live replay bypass table) ran here without skipping. The read-only property was observed directly in the CI control.

Plan-level truths of 02-21 and 02-22 that the table above does not already cover:
- The deny-list is gone from `crates/`: grep finds no `HELPER_OPTION_PREFIXES`, `ASSEMBLER_WRITE`, `SEPARATE_OPERAND_OPTIONS` or `find_helper_option`. `option_not_allowed` is mapped at `capture.rs:1317` and documented in the cc.rs module table.
- No push between the RED and Task-3 commits: Actions has runs only for pushed heads 4818d97, 44a3dd6, 79fe192, ae3df6c, 9273ef8, 7c55dbe and later, and none for a RED commit.
- `mt testbed toolchains` exists (`testbed.rs:596 run_toolchains`, mt-cli), and so does `run_capture_record_read_only` (`testbed.rs:648`).
- `toolchain_file_read_only` is in the report (`report.rs:268`) and omitted when false.
- Read-only coverage of every CMake pair: both record lines carry the flag, read-only mode refuses a CMake pair without the trusted copy (`build_01_record_stage_requires_the_read_only_toolchain_when_asked`), and the final run captured all 30 pairs.

### UAT Gap Reconciliation (02-UAT.md not edited; for /gsd-verify-work)

| Gap | UAT test | `missing` items delivered | Resolved by | Proof |
|-----|----------|---------------------------|-------------|-------|
| G-02-5 | 6 (WR-11, WR-12) | 3 of 3 | 02-20-PLAN.md (WR-12 strengthened by 02-22-PLAN.md) | Row 9; tests `build_01_captured_verdict_needs_passed_gates_and_objects`, `build_01_record_stage_refuses_a_record_that_did_not_build`, `build_01_record_stage_refuses_a_changed_toolchain_file`; CI 39528941371 |
| G-02-6 | 7 (driver flags) | 2 of 2 | 02-20-PLAN.md (gate replaced by 02-21-PLAN.md) | Row 10; T-02-67 closed in 02-SECURITY.md |
| G-02-7 | 8 (AR-02-03) | 1 of 1 | 02-20-PLAN.md (text kept current by 02-21 and 02-22) | Row 11; AR-02-03 row and capture.rs:107-148 |
| G-02-8 | review | 5 of 5 | 02-21-PLAN.md (items 1, 2, 4) and 02-22-PLAN.md (items 3, 4 final run); item 5 by /gsd-secure-phase 2 on 2026-10-21 | See below |

G-02-8, item by item:

1. **One operand-aware walk at capture and at replay, before any spawn.**
   - Code: `cc::walk` (`cc.rs:394`) with `walk_arguments` (`:519`), used by `derive_output` (`:531`), `check_write_paths` (`:581`) and `check_command` (`:632-636`). `canonical_entry` calls it at `:733` and `check_entry` at `:657`; `replay.rs:149` and `:234` run it before anything else.
   - Tests: `build_01_one_walk_serves_compile_only_output_and_write_paths` (build_01_compile_commands.rs:661), `build_01_gate_decides_compile_only_from_the_walk` (build_01_gate_bypasses.rs:121), `build_01_record_stage_refuses_an_operand_compile_flag_in_the_export` (build_01_capture_stages.rs:648).
2. **Measured allow-list; everything else refused with a named cause; CR-01..CR-03 RED first.**
   - Code: `ALLOWED_OPTIONS` (`cc.rs:290`, 56 spellings, with its measurement comment) and `SHIPPED_SPECS` (`cc.rs:365`, 13 names from container run 39453293834, which I counted from its annotation).
   - Tests: `build_01_allow_list_is_the_measured_set` (:412), `build_01_allow_list_refuses_everything_else` (:562), `build_01_shipped_specs_are_measured` (:505), `build_01_specs_allowed_only_for_shipped_names` (:529), `build_01_shipped_spec_names_are_read_from_the_toolchain`, `build_01_gate_refuses_long_option_spellings` (CR-01, build_01_gate_bypasses.rs:82), `build_01_gate_refuses_assembler_carrier_spellings` (CR-02, :102), `build_01_gate_refuses_response_files_in_operand_position` (:171), `build_02_replay_refuses_the_measured_bypasses` (live, build_02_replay_bypasses.rs:219).
   - RED commits 68e0bc7, 148715f, 1270139 and 78a12ea touch only test files.
3. **WR-12 (configure-time proof) and WR-14 (named cause).**
   - Code: `write_trusted_toolchain` (`capture.rs:2375`), `check_trusted_toolchain` (`:2410`), `capture_build_with_toolchain` (`:2352`), `capture_record_read_only` (`:2523`), `RunLayout::trusted_toolchain_path` (`:795`). Workflow: `testbed.yml:233` (toolchains step), `:243` (`:ro` mount), `:244` (`--toolchain-dir`), `:454` and `:517` (`--read-only-toolchain`). WR-14: `toolchain_file_finding` (`:1337`).
   - Tests: `build_01_record_stage_uses_the_read_only_toolchain_file` (:1040), `build_01_record_stage_refuses_a_build_that_rewrote_the_read_only_toolchain_file` (:1088), `build_01_record_stage_refuses_its_own_toolchain_file_beside_a_read_only_one` (:1140), `build_01_build_stage_refuses_a_read_only_toolchain_that_differs_from_the_pins` (:1177), `build_01_record_stage_requires_the_read_only_toolchain_when_asked` (:1214), `build_01_record_stage_checks_the_toolchain_consistency`, `build_01_configured_toolchain_consistency_findings` (build_01_toolchain_evidence.rs:87), `build_01_unreadable_toolchain_file_is_a_named_cause` (:157), `build_02_untrusted_builds_cannot_write_the_evidence` (build_02_workflow_static.rs:542), `build_02_record_steps_require_the_read_only_toolchain` (:772), `build_01_toolchains_writes_the_pinned_text_for_cmake_pairs` (build_01_capture_stages_cli.rs:344), `build_01_toolchain_dir_needs_stage_build` (:402).
   - CI: container 39528941352 step 11 and the toolchain-ro-control notice.
4. **No retained pair refused.**
   - Local: I re-ran both captures proofs. `files=29 raw_entries=531 canonical_entries=481 run_exports=2 run_export_entries=20 refused=0` and `cmake_pairs=17 entries=99 refused=0`.
   - Fixtures: both CI fixtures pass (`build_01_driver_option_gate_keeps_ordinary_flags`, `build_01_ci_fixtures`).
   - Full bed: testbed 39528941371 notices equal 02-20's from line 5 on (0 differing lines), and testbed 39455562978 (02-21) did the same.
5. **T-02-67 and T-02-70 re-verified and closed.**
   - 02-SECURITY.md rows 131 and 134 are `closed`, with the 02-21 file:line evidence, which I confirmed matches the code. The audit-trail row of 2026-10-21 reads 79/79 with `threats_open: 0`.

### Deferred Items

None are deferred against later phases. The WR-07 member-count cap stays deferred by the founder (02-13). No later phase takes it over, and it is not a success-criterion gap.

### Advisory (New Scope, Unevidenced)

None of these blocks. Each was raised in this pass and lacks a failing test or reproduction, or is documentation only.

| # | Finding | Category | Why Advisory |
|---|---------|----------|--------------|
| 1 | `entries.rs::filter_flags` (`entries.rs:521-574`) reads recorded options without the walk. It keeps `-I`, `-isystem`, `-iquote` and `-include` with a separate operand, so `-MT -include x.h` (which the walk reads as an `-MT` target plus an input) reaches the clang entry probe as `-include x.h`. | security | New scope, found by reading the code only; `entries.rs` is unchanged since 02-11. It is limited to header-parse options in the entry check: no helper runs and no file is written. G-02-8 names the -c test, derive_output and the write-path check. |
| 2 | IN-16: the local end-state toolchain mode cannot see a changed-and-restored file, and replay does not refuse `toolchain_file_read_only: false`. | security | CI always runs read-only mode (pinned by a static test), and the weaker mode is labelled in the report. |
| 3 | IN-17: "run_build's docker flags" in the container control and in T-02-76 overstates how closely the control matches run_build. | other | Wording only. |
| 4 | ROADMAP.md Phase 2 still unchecked, "20/20 plans executed", 02-21 and 02-22 not listed. | other | Orchestrator state. |
| 5 | Carried review items WR-01, WR-03, WR-06, WR-09, WR-07 (deferred), IN-01..IN-06, IN-08..IN-12, IN-16, IN-17. | other | 0 critical. None was changed by the gap closure except the WR-01 class, which 02-22 narrowed (an unreadable toolchain file or cache is now a cause). |

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/mt-capture/src/cc.rs` | `ALLOWED_OPTIONS`, `SHIPPED_SPECS`, `walk_arguments`, `CommandWalk`, `check_command`, `OptionNotAllowed` | VERIFIED | Substantive (796 lines). Wired from `canonicalise` and `replay::validate` |
| `crates/mt-capture/src/capture.rs` | `option_not_allowed`; `capture_build_with_toolchain`, `capture_record_read_only`, `write_trusted_toolchain`, `toolchain_file_finding`, `configured_toolchain_findings`, `settle_verdict`; module doc | VERIFIED | All present. Wired through `testbed.rs` and mt-cli |
| `crates/mt-capture/src/report.rs` | `toolchain_file_read_only` | VERIFIED | `report.rs:268`. Seen in the CI corejson report |
| `crates/mt-capture/src/testbed.rs` | `run_toolchains`, `run_capture_record_read_only` | VERIFIED | `:596`, `:648`. Called by mt-cli (`main.rs:210-216`, `:473-485`) |
| `crates/mt-capture/tests/build_01_gate_bypasses.rs`, `build_02_replay_bypasses.rs`, `build_01_toolchain_evidence.rs`, `build_01_capture_verdict.rs` | RED/GREEN reproductions | VERIFIED | All their tests pass here |
| `.github/workflows/testbed.yml` | toolchains pre-step, `/opt/mt/toolchain:ro`, `--read-only-toolchain` | VERIFIED | Lines 233, 243, 244, 454, 517. Green on 9273ef8 |
| `.github/workflows/container.yml` | arm-specs and toolchain-ro-control steps | VERIFIED | Both steps succeeded (runs 39453293834 and 39528941352) |
| `02-21-CI-NOTICES.md`, `02-22-CI-NOTICES.md` | Full-bed notices equal to 02-20's | VERIFIED | Diffs empty. 02-22's content matches the live annotations |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `canonical_entry`, `CcPolicy::check_entry` | `CcPolicy::check_command` | the same gate at both ends | WIRED | cc.rs:733, :657 |
| `derive_output`, `check_write_paths`, `check_command` | `walk` / `walk_arguments` | one walk, one table | WIRED | cc.rs:531, :581, :632 |
| `replay::validate` | `CcPolicy::check_entry` | every entry before delete, copy or spawn | WIRED | replay.rs:149, first call in `replay()` at :234 |
| `SHIPPED_SPECS` | container.yml arm-specs step | measured names, run id in comment | WIRED | 13 of 13 names equal the annotation |
| testbed.yml run_build | `<work>/toolchain/<id>/<cfg>` | `:ro` mount, `--toolchain-dir` | WIRED | lines 233, 243, 244 |
| `capture_build_with_toolchain` | trusted `toolchain.cmake` | byte check before any step | WIRED | `check_trusted_toolchain` capture.rs:2410 |
| record lines | `capture_record_read_only` | `--read-only-toolchain` | WIRED | testbed.yml:454, :517; static test |
| manifest pins, captures, compare, seeds, zip budget | (baseline links) | unchanged | WIRED | Notices equal to 02-19's; data and seeding code unchanged |

### Data-Flow Trace (Level 4)

Not applicable: nothing renders dynamic data. The evidence path runs manifest, fetch, toolchains, build (read-only toolchain), record (allow-list walk), replay (same walk), compare. I read real values at each step from the run 39528941371 annotations: 30 captured pairs, 30 identical replays, the corejson report with the read-only toolchain, and 74 compared files.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Workspace tests | `CARGO_INCREMENTAL=0 cargo test --workspace --locked --offline` | exit 0; 352 passed, 0 failed, 2 ignored | PASS |
| Local compile-command gate over real captures | `MT_LOCAL_CAPTURES=$PWD/.mt-work/capture cargo test -p mt-capture --test build_01_compile_commands -- --ignored --exact build_01_local_captures_pass_the_compile_command_gate --nocapture` | `files=29 raw_entries=531 canonical_entries=481 run_exports=2 run_export_entries=20 refused=0` | PASS |
| Local toolchain consistency over real captures | `... --test build_01_toolchain_evidence -- --ignored --exact build_01_local_captures_pass_the_toolchain_consistency_checks --nocapture` | `cmake_pairs=17 entries=99 refused=0` | PASS |
| Live WR-12 rewrite refused, not skipped | `cargo test -p mt-capture --test build_01_capture_stages -- --exact build_01_record_stage_refuses_a_build_that_rewrote_the_read_only_toolchain_file --nocapture` | 1 passed, no `skipping:` line | PASS |
| Live replay bypass table, not skipped | `cargo test -p mt-capture --test build_02_replay_bypasses -- --exact build_02_replay_refuses_the_measured_bypasses --nocapture` | 1 passed, no `skipping:` line | PASS |
| Manifest validates | `target/debug/mt testbed validate --manifest testbed/manifest.toml` | valid (16 targets, 31 configurations) | PASS |
| Seed integrity | Python sha256 recomputation over all index rows | 1467 rows, 0 bad | PASS |
| CI conclusions | `gh api repos/sat-wik/migrationtool/actions/runs/<id> --jq '.name,.head_sha,.conclusion'` | testbed 39528941371, container 39528941352, ci 39528941315: success on 9273ef8; ci 39532995983: success on HEAD bd0df6a | PASS |
| Notices equality | `diff <(sed '1,4d' 02-20-CI-NOTICES.md) <(sed '1,4d' 02-22-CI-NOTICES.md)` | empty | PASS |
| Real Arm build, capture, replay in the pinned image | job 118990405406 annotations | 30 captured, 30 identical, determinism identical | PASS (CI only; no Docker in this session) |

### Probe Execution

No `scripts/*/tests/probe-*.sh` exists, and no plan declares one: SKIPPED.

### Requirements Coverage

Every requirement ID in the 22 PLAN frontmatters is one of the five Phase 2 IDs. All five are checked complete in REQUIREMENTS.md (lines 23-27, traceability 172-176). No requirement is orphaned.

| Requirement | Source Plans | Description | Status | Evidence |
|-------------|--------------|-------------|--------|----------|
| BED-01 | 02-01..02-22 (incl. 02-20, 02-21, 02-22) | 10-20 targets pinned, licence, buffer-in entry | SATISFIED | Truths 1, 5. 16 targets, entries found for every captured pair |
| BED-02 | 02-03..02-22 | Declared configurations; RTOS decision | SATISFIED | Truth 2 |
| BUILD-01 | 02-01..02-22 | Capture every compile command | SATISFIED | Truths 3, 9, 10, 12. 30 pairs captured through the allow-list walk |
| BUILD-02 | 02-02..02-22 | Replay reproduces the build output; drops logged; at least 10 kept | SATISFIED | Truth 3. 30 identical, 1 dropped with cause, 16 targets retained, determinism identical; replay gate equals the capture gate |
| BUILD-03 | 02-07..02-22 | Seed corpora | SATISFIED | Truths 4, 7, 8 |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| 17 files changed in 02-20..02-22 (outside `.planning/`) | n/a | `TBD`/`FIXME`/`XXX` | none | Debt-marker gate clear |
| same files | n/a | `TODO`/`HACK`/`todo!`/`unimplemented!`; `unwrap`/`expect`/`panic!`/`Command::new` in `src/` | none | No match. `#![forbid(unsafe_code)]` in mt-capture and mt-cli |
| `crates/mt-capture/src/entries.rs` | 521-574 | Second option reader outside the walk | Info (advisory 1) | Parse-only options for the clang probe |
| `.github/workflows/container.yml` | toolchain-ro-control | Comment overstates fidelity to run_build (IN-17) | Info | Wording |

The baseline's three warnings (WR-11, WR-12 end-state only, unregistered driver flags) are fixed. WR-11 is fixed by settle_verdict and build_record_inconsistent. WR-12 is fixed by the read-only mount. The driver flags are covered by the allow-list and T-02-67.

02-REVIEW.md (post-02-22, deep): 0 critical, 5 warning (all carried, none in this gap's scope), 13 info. 02-SECURITY.md: 79 threats, 79 closed, `threats_open: 0`.

### Human Verification Required

The baseline's three items are resolved by founder decision (02-UAT.md tests 6-8) and delivered as G-02-5 to G-02-7. None of them still needs a human.

What remains is the six judgment-tier prohibitions of plans 02-21 and 02-22, which are still `status: active`. The verifier records an advisory verdict on each but may not resolve them.

#### 1. 02-21: only measured spellings on ALLOWED_OPTIONS and SHIPPED_SPECS

**Test:** Read `cc.rs:253-379` against the measurement comment and the arm-specs notice of container run 39453293834.
**Expected:** Resolved, or a spelling named for removal.
**Why human:** Judgment-tier prohibition. Advisory verdict: holds. The list has 56 spellings: 45 measured, 1 CI-only with run 39453293771 as evidence, plus the founder's depfile and `-specs=` additions. All 13 spec names equal the notice.

#### 2. 02-21: no relaxation of any gate, comparison, limit, verdict rule or floor

**Test:** Review the "Changed 02-20 expectations" table in 02-21-SUMMARY.md.
**Expected:** Resolved.
**Why human:** Judgment-tier. Advisory verdict: holds. Every change moves a flag to refused, and the arch gate keeps its own passing end-to-end test.

#### 3. 02-21: push discipline (branch, no force-push, never red, at most 3 round trips)

**Test:** Confirm from the Actions history.
**Expected:** Resolved.
**Why human:** Judgment-tier. Advisory verdict: holds. Every run is on the session branch, 2 testbed round trips were used, every pushed head is an ancestor of HEAD, and ci is green on every pushed head. Force-push cannot be fully excluded from run history.

#### 4. 02-22: toolchain mount never writable and flags never dropped

**Test:** Read `testbed.yml:221-244`, `:454` and `:517`.
**Expected:** Resolved.
**Why human:** Judgment-tier. Advisory verdict: holds. The mount is `:ro` at /opt/mt/toolchain, both flags are present, both are pinned by static tests, and the CI report reads `toolchain_file_read_only: true`.

#### 5. 02-22: no relaxation, including the 02-21 allow-list

**Test:** Confirm cc.rs and replay.rs are unchanged by 02-22, and the verdict rule too.
**Expected:** Resolved.
**Why human:** Judgment-tier. Advisory verdict: holds. Causes were added and none removed. The notices equal 02-20's.

#### 6. 02-22: push discipline

**Test:** Confirm from the Actions history.
**Expected:** Resolved.
**Why human:** Judgment-tier. Advisory verdict: holds. 1 round trip (9273ef8), all three workflows green, and later pushes touch only `.planning/`.

### Gaps Summary

No gaps. On the current code (identical to 9273ef8 outside `.planning/`):
- all four ROADMAP success criteria hold;
- all eight UAT gaps have their `missing` items delivered, with code, passing tests and CI evidence;
- the testbed, container and ci runs on 9273ef8, and the ci run on HEAD, all succeeded.

For `/gsd-verify-work` to reconcile:
- G-02-5, G-02-6 and G-02-7 are resolved by 02-20-PLAN.md. 02-21 replaced G-02-6's deny-list, and 02-22 strengthened G-02-5's WR-12 half.
- G-02-8 is resolved by 02-21-PLAN.md (items 1, 2 and 4) and 02-22-PLAN.md (items 3 and 4). Item 5 was closed by /gsd-secure-phase 2 on 2026-10-21.
- G-02-1 to G-02-4 stay resolved.

What remains for a human is resolving the six active judgment-tier prohibitions of 02-21 and 02-22. The advisory items above are optional follow-ups.

---

_Verified: 2026-10-21T02:34:52Z_
_Verifier: Claude (gsd-verifier)_


## Founder sign-off (2026-10-21)

The founder confirmed all six judgment-tier prohibitions of plans 02-21 and 02-22 on the verifier's advisory evidence above, and they are set to `status: resolved` in the PLAN frontmatter. The verifier's advisory finding on `entries.rs` `filter_flags` (options reaching the clang entry probe outside the gate walk) is logged for a later phase as review finding IN-18, by founder decision.
