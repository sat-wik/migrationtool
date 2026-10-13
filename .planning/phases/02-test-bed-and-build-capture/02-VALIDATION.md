---
phase: "2"
slug: "test-bed-and-build-capture"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: validated
nyquist_compliant: true
wave_0_complete: true
created: "2026-10-12"
---

# Phase 2 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution. Seeded from `02-RESEARCH.md` § Validation Architecture; task IDs are filled in once plans exist.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test harness (`cargo test`), `tempfile` 3.27.0 for temporary trees |
| **Config file** | none beyond the workspace `Cargo.toml` (`crates/mt-capture` added to `members`) |
| **Quick run command** | `cargo test -p mt-capture` |
| **Full suite command** | `cargo test --workspace` plus `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo deny check` |
| **Container-only checks** | the `testbed` workflow (needs Docker and the extended image, D-30) |
| **Estimated runtime** | ~30 seconds for the quick command; full suite ~90 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p mt-capture`
- **After every plan wave:** Run the full suite command
- **Before `/gsd-verify-work`:** Full suite green and a green `testbed` workflow run (replay identical for every retained target, drop log committed, at least 10 retained)
- **Max feedback latency:** 30 seconds (offline); CI round trips are the long pole and are not part of per-task sampling

---

## Per-Task Verification Map

"Offline" runs under plain `cargo test` in a cloud session (no Docker, no network). "CI" needs the image and is proven by `testbed` workflow annotations. Task IDs are assigned by the planner.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| — | 02-03 | — | BED-01 | — | Every target pinned by 40-hex commit (or archive sha256) and 64-hex tree hash | unit, offline | `cargo test -p mt-capture bed_01_manifest_pins_every_target_by_commit` | ✅ | ✅ green |
| — | 02-03 | — | BED-01 | — | 10 to 20 targets in the committed manifest | unit, offline | `cargo test -p mt-capture bed_01_manifest_has_between_10_and_20_targets` | ✅ | ✅ green |
| — | 02-03 | — | BED-01 | — | Copyleft-only licence rejected; OR expression accepted only with a permissive `elected` operand (D-27) | unit, offline | `cargo test -p mt-capture bed_01_manifest_rejects_copyleft_licence` and `bed_01_every_target_records_permissive_licence_and_file` | ✅ | ✅ green |
| — | 02-03 | — | BED-01 | — | 1-3 buffer-in entries per target; wrapped form needs `accepted_wrapped = true` (D-28) | unit, offline | `cargo test -p mt-capture bed_01_every_target_has_a_buffer_in_entry` | ✅ | ✅ green |
| — | 02-03 | — | BED-01 | — | Unknown keys, duplicate ids, missing entries rejected | unit, offline | `cargo test -p mt-capture bed_01_manifest_rejects_unknown_keys` (and `_duplicate_ids`, `_missing_entry`) | ✅ | ✅ green |
| — | 02-03 | — | BED-01 | — | Every beachhead category covered, stand-ins per D-29 | unit, offline | `cargo test -p mt-capture bed_01_every_beachhead_category_is_covered` | ✅ | ✅ green |
| — | 02-03 | — | BED-01 | — | Tree hash equals the reference vectors (files, exec bit, symlink, `.git` excluded) | unit, offline | `cargo test -p mt-capture bed_01_tree_hash_matches_reference_vectors` | ✅ | ✅ green |
| — | 02-05 | — | BED-01 | — | Fetch pins commit and tree hash; mismatch rejected; reuse verified | integration, offline (local `file://` repo) | `cargo test -p mt-capture bed_01_fetch_pins_commit_and_tree_hash` and `bed_01_fetch_rejects_tree_hash_mismatch` | ✅ | ✅ green |
| — | 02-05 | — | BED-01 | — | Entry signature found and equal in the captured build (D-23) | CI + offline fixture parse | testbed `capture` step; `cargo test -p mt-capture bed_01_entry_ast_stream_parses_concatenated_documents` | ✅ | ✅ green |
| — | 02-03 | — | BED-02 | — | `default` configuration plus at most one variant | unit, offline | `cargo test -p mt-capture bed_02_every_target_declares_default_and_at_most_one_variant` | ✅ | ✅ green |
| — | 02-03 | — | BED-02 | — | First-RTOS decision recorded (manifest and PROJECT.md) | unit, offline | `cargo test -p mt-capture bed_02_first_rtos_decision_recorded` | ✅ | ✅ green |
| — | 02-06/02-09 | — | BED-02 | — | Variant changes compiled code | CI | testbed `capture` report | n/a | ✅ green (testbed run 38430137113) |
| — | 02-04 | — | BUILD-01 | — | Make and CMake commands canonicalise to `arguments`, sorted, built-only, correct output | unit, offline | `cargo test -p mt-capture build_01_cmake_command_strings_split_into_argv`, `build_01_default_output_is_source_stem_object`, `build_01_cmake_entries_for_unbuilt_targets_are_dropped`, `build_01_entries_sorted_canonically` | ✅ | ✅ green |
| — | 02-04 | — | BUILD-01 | — | Only allow-listed compilers; every object is Cortex-M4 hard-float (silent-wrong-build gate) | unit, offline | `cargo test -p mt-capture build_01_rejects_non_allowlisted_compiler`, `build_01_captured_objects_are_cortex_m4_hard_float` | ✅ | ✅ green |
| — | 02-05 | — | BUILD-01 | — | Generated toolchain file carries baseline and config defines | unit, offline | `cargo test -p mt-capture build_01_toolchain_file_carries_baseline_flags` | ✅ | ✅ green |
| — | 02-06/02-09 | — | BUILD-01 | — | `compile_commands.json` covers every TU of every retained target | CI | testbed `capture` step | n/a | ✅ green (testbed run 38430137113) |
| — | 02-04 | — | BUILD-02 | — | Tiny C project replay matches; a changed byte is reported | integration, offline | `cargo test -p mt-capture build_02_replay_matches_original_objects` and `build_02_replay_detects_changed_object` | ✅ | ✅ green |
| — | 02-04 | — | BUILD-02 | — | Normalisation forgives `.comment`/debug only; `.text` differences fail | unit, offline | `cargo test -p mt-capture build_02_normalisation_only_forgives_comment_and_debug_sections` and `build_02_normalisation_rejects_text_section_difference` | ✅ | ✅ green |
| — | 02-04 | — | BUILD-02 | — | Clean-copy replay removes stale outputs first | integration, offline | `cargo test -p mt-capture build_02_clean_copy_replay_removes_stale_outputs` | ✅ | ✅ green |
| — | 02-04 | — | BUILD-02 | — | Reports deterministic (`BTreeMap`, `schema_version`) | unit, offline | `cargo test -p mt-capture build_02_report_is_deterministic` | ✅ | ✅ green |
| — | 02-09 | — | BUILD-02 | — | Drop log has causes and ≥10 retained; capture/replay run `--network none`, fetch does not | unit, offline (static YAML check) | `cargo test -p mt-capture build_02_drop_log_keeps_at_least_ten_targets` and `build_02_workflow_runs_capture_and_replay_without_network` | ✅ | ✅ green |
| — | 02-06/02-09 | — | BUILD-02 | — | Every retained target and configuration replays identically in the image | CI | testbed `replay` step | n/a | ✅ green (testbed run 38430137113) |
| — | 02-07 | — | BUILD-03 | — | Seed files named by sha256; index deterministic and versioned | unit, offline | `cargo test -p mt-capture build_03_seed_names_are_sha256_of_contents` and `build_03_seed_index_is_deterministic_and_versioned` | ✅ | ✅ green |
| — | 02-07 | — | BUILD-03 | — | C byte-array extraction follows C escape rules | unit, offline | `cargo test -p mt-capture build_03_c_byte_array_extraction_follows_c_escape_rules` | ✅ | ✅ green |
| — | 02-07 | — | BUILD-03 | — | Seed caps and selection deterministic | unit, offline | `cargo test -p mt-capture build_03_seed_caps_are_enforced_deterministically` | ✅ | ✅ green |
| — | 02-08 | — | BUILD-03 | — | Provenance recorded; hand-made sets `proposed` and ≥10 | unit, offline | `cargo test -p mt-capture build_03_hand_made_seeds_marked_proposed_and_at_least_ten` and `build_03_seed_provenance_records_source_path_commit_and_method` | ✅ | ✅ green |
| — | 02-06/02-09 | — | BUILD-03 | — | Committed seeds equal regenerated seeds | CI | testbed `seeds --check` | n/a | ✅ green (testbed run 38430137113) |
| — | 02-05 | — | (guard) | — | `.mt-work` does not trip the Python check; manifest image digest equals README | unit, offline | `cargo test -p mt-toolchain tool_01_no_python_outside_research` and `cargo test -p mt-capture build_02_image_digest_matches_readme` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] `crates/mt-capture/` crate, workspace member, `tests/` and `tests/fixtures/` (recorded Bear and CMake JSON for two small targets, `readelf -A` text, two Arm `.o` fixtures, a tiny C project)
- [x] `testbed/manifest.toml` (after the D-03 checkpoint), `testbed/dropped.toml`
- [x] `.github/workflows/testbed.yml`
- [x] Image extension and its static tests (D-30, shipped to `main` first)
- [x] `Deserialize` derives on `RunRecord` and friends; `ROOT_SKIPS` update for `.mt-work`
- [x] Framework install: none (cargo only)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Founder approves the concrete target list | BED-01 (D-03) | A decision, not a behavior | `checkpoint:decision` in the manifest plan presents the research tables; record the approval in the plan summary |

*All other phase behaviors have automated verification (offline or CI).*

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 30s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** validated 2026-10-13 (founder approval of the target list recorded in 02-03-SUMMARY.md)

## Validation Audit 2026-10-13

| Metric | Count |
|---|---|
| Gaps found | 0 |
| Resolved | 0 |
| Escalated | 0 |
