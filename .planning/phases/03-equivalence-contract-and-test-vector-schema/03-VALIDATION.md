---
phase: "03"
slug: "equivalence-contract-and-test-vector-schema"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: validated
nyquist_compliant: true
wave_0_complete: true
created: "2026-10-21"
updated: "2026-11-03"
---

# Phase 03 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution. Seeded from `03-RESEARCH.md` § Validation Architecture (Nyquist).

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | The Rust built-in test harness (`cargo test`), with `tempfile` for work directories. No new crates (D-29); `tempfile` is the workspace's existing dev-dependency. |
| **Config file** | Workspace `Cargo.toml`, and `clippy.toml`, which bans `std::process::Command` outside the runner. |
| **Quick run command** | `cargo test -p mt-contract -p mt-vectors` (plus the crate under change) |
| **Full suite command** | `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace --locked && cargo deny check` |
| **Estimated runtime** | Quick: about 20 seconds (measured 6 seconds on a warm build, 2026-11-03). Full: about 300 seconds (`cargo test --workspace --locked` measured 39 seconds on a warm build, 2026-11-03). |

Local-only ignored tests follow the Phase 2 convention: `MT_LOCAL_CAPTURES=$PWD/.mt-work/capture cargo test -p <crate> --test <file> -- --ignored --exact <name> --nocapture`. The TRACTOR test uses `MT_LOCAL_TRACTOR=<dir>`. Tests never use the network or a live model.

---

## Sampling Rate

- **After every task commit:** run the quick command plus the crate under change.
- **After every plan wave:** run the full suite command, plus the ignored local executor tests on a host that has `qemu-arm`.
- **Before `/gsd-verify-work`:** the full suite must be green, and the `testbed` run on the final code commit must be green with the new notices: drivers built, vectors produced, twice-run mismatches = 0, and at least 3 TRACTOR vectors loaded.
- **Max feedback latency:** 300 seconds.

---

## Per-Task Verification Map

Task IDs are filled in by the planner and plan checker. The rows below are the requirement-level contract each plan's tasks must cover.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 03-02-T1, 03-02-T2 | 03-02 | 3 | EQV-01 | T-03-05, T-03-06, T-03-07 | The policy loads. An unknown key is rejected. The default is `return_error`. Changing the default needs a `default_change` record. An override missing any required key, or naming a site that was not logged, is rejected. An empty override table is accepted. | unit | `cargo test -p mt-contract eqv_01` | ✅ | ✅ green |
| 03-02-T3 | 03-02 | 3 | EQV-01 | T-03-08 | The contract document has a semver version and a changelog. It states refinement, the observation point, the comparison rule and the three actions. It never contains "certified" or "compliant". | unit (doc check) | `cargo test -p mt-contract eqv_01_contract_document` | ✅ | ✅ green |
| 03-03-T1, 03-03-T2 | 03-03 | 4 | EQV-02 | T-03-13 | A vector has at most the nine fields. An unknown key is rejected, and so is a merged `lib_state`. `has_ub` absent means no observed UB, and `has_ub: false` is never emitted (D-26). `stdout` serializes as `{pattern, is_regex: false}`. | unit | `cargo test -p mt-vectors eqv_02` | ✅ | ✅ green |
| 03-03-T1 | 03-03 | 4 | EQV-02 | — | The Latin-1 codec (D-25) round-trips all 256 byte values, NUL included, in `stdin` and `stdout`. | unit (exhaustive) | `cargo test -p mt-vectors eqv_02_byte_codec` | ✅ | ✅ green |
| 03-03-T3 | 03-03 | 4 | EQV-02 | T-03-09, T-03-10 | At least 3 TRACTOR vectors load unchanged and re-serialize JSON-equal. | integration (ignored locally, plus CI) | `MT_LOCAL_TRACTOR=... cargo test -p mt-vectors --test eqv_02_tractor -- --ignored --exact eqv_02_tractor_vectors_load_unchanged` | ✅ | ✅ green |
| 03-03-T3 | 03-03 | 4 | EQV-02 | T-03-11 | The committed TRACTOR pin lists only the commit, paths and sha256 hashes. No TRACTOR content is in the repo. | unit | `cargo test -p mt-vectors eqv_02_tractor_pin_has_no_content` | ✅ | ✅ green |
| 03-04-T1, 03-04-T2 (extended in 03-05..03-08) | 03-04 | 5 | EQV-03 | T-03-14 | Manifest roles reject a bad role, a wrong parameter count, a forbidden token in setup args, and a missing bound on a length. | unit | `cargo test -p mt-capture eqv_03_roles` | ✅ | ✅ green |
| 03-06-T3 (completeness 03-08-T2) | 03-06 | 7 | EQV-03 | T-03-23, T-03-28 | At least 10 targets are drivable, counted from the committed manifest and the adapter drop log (`no_seeds` and others logged with a reason, D-28). | unit (committed files) | `cargo test eqv_03_adapter_floor_is_ten` | ✅ | ✅ green |
| 03-04-T2 (extended in 03-05..03-07) | 03-04 | 5 | EQV-03 | T-03-14 | The generated driver source is stable (golden) for the tracer targets. | golden | `cargo test eqv_03_driver_golden` | ✅ | ✅ green |
| 03-04-T2 | 03-04 | 5 | EQV-04 | T-03-16 | The link recipe refuses an object whose sha256 differs from the capture record. | unit | `cargo test eqv_04_refuses_modified_object` | ✅ | ✅ green |
| 03-01-T2 (setup_failed in 03-07-T1) | 03-01 | 1 | EQV-04 | T-03-01, T-03-04 | The outcome classification maps each runner result to exactly one class: ok, fault, abort or timeout. Fault and abort give `rc = -signal` and a `has_ub` crash label (D-25, D-27). | unit (table-driven) | `cargo test eqv_04_classify` | ✅ | ✅ green |
| 03-05-T1 | 03-05 | 6 | EQV-04 | T-03-19 | The layout parser reads a committed `llvm-dwarfdump` text fixture covering a struct with a bitfield, an array, a 1-byte enum, a union and a typedef. | unit | `cargo test eqv_04_layout_parser` | ✅ | ✅ green |
| 03-04-T1, 03-04-T3 | 03-04 | 5 | EQV-04 | T-03-15, T-03-17 | Running the same vector twice gives identical vector bytes for the tracer targets. | integration (ignored locally, plus CI) | `cargo test --test eqv_04_exec -- --ignored --exact eqv_04_run_twice_is_byte_identical` | ✅ | ✅ green |
| 03-01-T2, 03-01-T3 | 03-01 | 1 | EQV-04 | T-03-01 | An overrun into the guard page gives outcome `fault`, `rc = -11`, a `has_ub` label, and no core file. | integration (runs when the host has `qemu-arm` and `arm-none-eabi-gcc`, otherwise prints `skipping:`; plus CI) | `cargo test -p mt-harness --test eqv_04_selftest eqv_04_overrun_is_a_fault` | ✅ | ✅ green |
| 03-01-T2 | 03-01 | 1 | EQV-04 | T-03-02 | A timeout gives outcome `timeout` and is never recorded as `ok`. | integration (as above) | `cargo test -p mt-harness --test eqv_04_selftest eqv_04_timeout_is_recorded` | ✅ | ✅ green |
| 03-04-T2 | 03-04 | 5 | EQV-04 (D-07) | — | The output buffer's sentinel prefill is recorded, and the whole buffer is captured. | integration (ignored locally) | `MT_LOCAL_CAPTURES=... cargo test -p mt-harness --test eqv_04_exec -- --ignored --exact eqv_04_sentinel_prefill_recorded` | ✅ | ✅ green |
| 03-04-T3, 03-08-T3 | 03-08 | 9 | EQV-04 (D-19, D-21) | T-03-29, T-03-31 | The provenance sidecar hashes match, and the committed sample regenerates identically. | CI, plus offline replay checks of the committed sample | `mt vectors check` (testbed step); `cargo test -p mt-harness --test eqv_04_committed_sample --test eqv_04_replay_shape --test eqv_04_replay_provenance` | ✅ | ✅ green |
| 03-01-T3 (extended in 03-03, 03-04, 03-05, 03-08) | 03-01 | 1 | workflow | T-03-03, T-03-12, T-03-15 | `testbed.yml` contains the new steps, and every one except the TRACTOR fetch runs with `--network none`. | static workflow test | `cargo test eqv_04_workflow_static` | ✅ | ✅ green |
| 03-09-Tracer | 03-09 | 1 | EQV-04 | T-03-32, T-03-33 | The Dockerfile takes only the four qemu-source packages from bookworm-backports at the pinned snapshot (priority 990), every other backports package has priority -1, and no unsigned-source option exists. | static | `cargo test -p mt-toolchain --test container_static eqv_04_dockerfile_takes_only_qemu_from_backports` | ✅ | ✅ green |
| 03-09-Tracer | 03-09 | 1 | EQV-04 | T-03-34, T-03-36 | container.yml image-a runs the cortex-m4 start probe offline, read-only and unprivileged, before `Save the gated image`; pin-discovery lists backports QEMU versions without failing the run and keeps `contents: read`. | static workflow test | `cargo test -p mt-toolchain --test container_static -- eqv_04_container_workflow_probes_cortex_m4_user_mode_start eqv_04_pin_discovery_lists_backports_qemu_versions` | ✅ | ✅ green |
| 03-09-Tracer, 03-09-T3 | 03-09 | 1 | EQV-04 | T-03-32 | In the branch-built image, `qemu-arm -cpu cortex-m4 -R 0x10000000` starts the probe guest and returns its exit code 42; check-a shows the four QEMU rows OK at the discovered version. | CI | container workflow, image-a notices `qemu-mprofile` and `check-a` | ✅ | ✅ green |
| 03-10-T3 | 03-10 | 2 | EQV-04 | T-03-38, T-03-39 | Main's image-a `qemu-mprofile` notice ends `exit code: 42` (fail-closed gate before anything is recorded); Current image names only the digest main's publish job verified; the first testbed run on it has its eight Phase 2 steps green; 02-22-CI-NOTICES.md changes from line 5 on only after the founder's `rebaseline` reply. | CI plus founder gate | testbed workflow; `diff <(sed '1,4d' 02-22-CI-NOTICES.md) <(sed '1,4d' 03-10-CI-NOTICES.md)` | ✅ | ✅ green |
| 03-11-T1, 03-11-T2 | 03-11 | 10 | EQV-04 (gap closure) | T-03-42, T-03-43, T-03-44 | The vectors job's `safe_untar`, taken from `testbed.yml` itself (no copy), runs on 19 GNU-tar archives built by the test. It accepts the in-tree nanopb and nimble `..` links. It refuses an absolute target, a target that leaves the tree, a `..` that pops an archive link named by its own path, an absolute or `..` member name, a hard link and an empty archive, and each refusal leaves the destination empty. This row proves the exact-name case only: a path that reached a link through a second, aliasing link stayed accepted until plan 03-12 (code review CR-01); the plan 03-12 rows below carry that case. | behavioural (host bash + GNU tar through the mt-toolchain runner) | `cargo test -p mt-harness --test eqv_04_workflow_static` | ✅ | ✅ green |
| 03-11-T3 | 03-11 | 10 | EQV-04 (D-01, D-02, D-19, D-21, D-32) | T-03-45 | The vectors job is green at the last trigger-path head, and its seven evidence steps all have conclusion success. All 15 sample corpora replay `identical 1`. The testbed-* notices equal 02-22 from line 5 on. | CI (read with gh api) | testbed run 41735869792 on `752badf` (`03-11-VECTORS-NOTICES.md`, `03-11-CI-NOTICES.md`) | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

### Code-review fixes (2026-11-01, after the plans)

The two fix passes of `03-REVIEW-FIX.md` added behavior after the plans were executed. Each change carries a requirement-named test:

| Finding | Requirement | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|-------------|-----------------|-----------|-------------------|-------------|--------|
| CR-01 | EQV-04 | An integer or enum return wider than 32 bits, by spelling or by a typedef's DWARF size, is refused instead of being cast to `int32_t`. | unit | `cargo test -p mt-harness --test eqv_04_return_width` | ✅ | ✅ green |
| WR-01 | EQV-04 | Replay refuses a stray or non-regular file, a traversing or non-sha256 index key, an unknown configuration, a moved corpus, a vector whose sha256 differs from its key, and an edited provenance field (`seed`, `scalars`, `executor_version`, `contract_version`, `budgets`, `policy_sha256`); every committed sample corpus passes. | unit (committed files) | `cargo test -p mt-harness --test eqv_04_replay_shape --test eqv_04_replay_provenance` | ✅ | ✅ green |
| WR-02 | EQV-04 | `vectors build`, `run` and `adapters` fail on an empty listing, and `build` and `run` fail on a listing whose rows are all `no_roles`. | unit | `cargo test -p mt-cli -- eqv_04_empty_listing_is_a_failure eqv_04_listing_of_only_no_roles_rows_is_a_failure` | ✅ | ✅ green |
| WR-04, WR-05 | workflow | The vectors job vets both hand-off tars before unpacking, and a missing `testbed/vectors-sample` fails the sample replay. | static workflow test | `cargo test -p mt-harness --test eqv_04_workflow_static` | ✅ | ✅ green |
| WR-06, IN-09 | EQV-03 | An include-prefix line that splices (trailing backslash, `??/`) or opens a comment is refused, and the check runs before the probe. | golden, unit | `cargo test -p mt-harness --test eqv_03_driver_golden` | ✅ | ✅ green |
| WR-07 | EQV-01 | A policy `error_code` outside `i32::MIN..=u32::MAX` is rejected. | unit | `cargo test -p mt-contract eqv_01_error_codes_are_checked` | ✅ | ✅ green |
| WR-08 | BED-01, EQV-02 | Target ids and configuration names start with a letter or digit, matching the vector and policy grammars. | unit | `cargo test -p mt-capture bed_01_target_ids_and_config_names_start_with_a_letter_or_digit` | ✅ | ✅ green |
| IN-02 | EQV-01 | The contract text matches the wide-return refusal and the `serde_json` escapes. | unit (doc check) | `cargo test -p mt-contract eqv_01_contract_document_matches_return_and_codec_behavior` | ✅ | ✅ green |
| IN-03 | EQV-02 | Canonical vector bytes rely on `serde_json::Map` keeping keys sorted; a test pins it. | unit | `cargo test -p mt-vectors eqv_02_serde_json_map_keeps_keys_sorted` | ✅ | ✅ green |
| IN-05 | TOOL-03 | The final image stage runs as `65534:65534` by default. | static | `cargo test -p mt-toolchain --test container_static tool_03_final_stage_runs_as_the_unprivileged_user` | ✅ | ✅ green |
| IN-06 | EQV-03 | A `@file` response-file argument in the driver flags is refused, not dropped. | unit | `cargo test -p mt-harness eqv_03_driver_flags_refuse_a_response_file` | ✅ | ✅ green |
| IN-07 | EQV-01 | A policy file whose name is not UTF-8 is reported, not skipped. | unit | `cargo test -p mt-contract eqv_01_load_dir_reports_a_non_utf8_policy_file` | ✅ | ✅ green |

WR-03 and IN-01 (both in `runtime.c`) were skipped by the fix passes because editing the runtime changes the driver hashes recorded in the committed sample; they are open code-review findings, not validation gaps (`03-REVIEW-DISPOSITION.md`).

### Evidence (audit 2026-11-03, HEAD `332f7d0`)

- Every non-ignored command above passes: `cargo test --workspace --locked` (39 seconds warm). The `eqv_04_selftest` cases ran for real on this host (`qemu-arm` 8.2.2, `arm-none-eabi-gcc` 13.2.1; no `skipping:` line), so the overrun and timeout rows are proven locally, not only in CI.
- All 23 ignored local proofs pass: `MT_LOCAL_CAPTURES=$PWD/.mt-work/capture MT_LOCAL_TRACTOR=$PWD/.mt-work/tractor/37960ee08a7c644ee66e29855eca33903f702e67/tree cargo test --workspace --locked -- --ignored` (23 passed, 0 failed), including `eqv_02_tractor_vectors_load_unchanged`, `eqv_04_run_twice_is_byte_identical`, `eqv_04_sentinel_prefill_recorded`, `eqv_04_replay_reproduces_stored_vectors` and `eqv_04_sample_is_chosen_and_replays_from_a_tree`. These ran with the host toolchain, not the pinned image.
- In the pinned image (QEMU 10.0.2, GCC 14.3), the CI rows were green in testbed run 40922236736 on `5e0d4a8` (`03-08-VECTORS-NOTICES.md`: selftest 17 rows, TRACTOR 6 of 6, every target `twice=identical`, the sample step), in the container runs recorded in `03-09-SUMMARY.md` and `03-10-SUMMARY.md` (`qemu-mprofile` `exit code: 42`), and through the founder's `rebaseline` reply (`03-10-SUMMARY.md`).
- Not yet run in CI: the code-review fix commits `6c58482..90767fd` (including the `testbed.yml` changes for WR-04 and WR-05 and the Dockerfile `USER` line) postdate the last testbed and container runs. Their static and offline tests pass; the next testbed and container runs are their first in the pinned image.

---

## Wave 0 Requirements

- [x] Crates `crates/mt-contract`, `crates/mt-vectors` and the executor crate (the research names `mt-harness`), with the tests named above.
- [x] A fixture of `llvm-dwarfdump` text output, and synthetic runner records for the classification table.
- [x] The infinite-loop fixture and the overrun fixture, generated at test time from the shared runtime template, not committed as per-target C.
- [x] A CI QEMU smoke step that measures QEMU 7.2 and GCC 14.3 behavior in the pinned image (D-30) before the layout and adapter work depends on it. Measured: QEMU 7.2 cannot start a `-cpu cortex-m4` user-mode guest, so D-31 re-pins QEMU (plans 03-09 and 03-10) and the step measures the re-pinned QEMU with GCC 14.3.
- No framework install is needed.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| The contract's prose is accurate to the decisions | EQV-01 | Judgment over the wording, beyond the automated keyword checks | Read `docs/specs/equivalence-contract.md` against 03-CONTEXT.md D-05..D-30 |

Status: UAT test 1 passed on 2026-10-30 against the 0.1.0 text (`03-UAT.md`). Commit `dd1deeb` (code-review IN-02, 2026-11-01) then changed the `rc` row, the byte-codec paragraph and the `ret` entry: an integer return wider than 32 bits is now refused instead of going to `lib_state_out.ret`. The document still says 0.1.0 and has no changelog line for the change, although its own versioning paragraph says a change to the contract bumps the version, adds a changelog line and moves `CONTRACT_VERSION` and every policy file together. The changed paragraphs need a re-read, and the version question needs a decision.

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies (29 of 30 tasks have an `<automated>` block; the 30th, 03-10 Task 2, is the founder's merge, a `checkpoint:human-action`)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify (longest run without: 1)
- [x] Wave 0 covers all MISSING references (none missing)
- [x] No watch-mode flags
- [x] Feedback latency < 300s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** validated 2026-11-03 at HEAD `332f7d0`: every requirement row has an automated command that passes (workspace suite green; 23 of 23 ignored local proofs green on the host toolchain; CI rows green in testbed run 40922236736 and the 03-09/03-10 container runs). One manual-only item (contract prose) needs a re-read after `dd1deeb`.

## Validation Audit 2026-11-03

| Metric | Count |
|---|---|
| Gaps found | 0 |
| Resolved | 0 |
| Escalated | 0 |
