---
phase: 03-equivalence-contract-and-test-vector-schema
fixed_at: 2026-11-01T00:00:00Z
review_path: .planning/phases/03-equivalence-contract-and-test-vector-schema/03-REVIEW.md
iteration: 2
findings_in_scope: 12
fixed: 10
skipped: 2
status: partial
---

# Phase 3: Code Review Fix Report (iteration 2)

**Fixed at:** 2026-11-01
**Source review:** `.planning/phases/03-equivalence-contract-and-test-vector-schema/03-REVIEW.md`
**Iteration:** 2

**Summary:**
- Findings in scope: 12 (WR-01..WR-03, IN-01..IN-09)
- Fixed: 10 (IN-06 and WR-01 are fixed in part; see their entries)
- Skipped: 2 (WR-03, IN-01)

**Where verification ran:** the main checkout `/home/user/migrationtool` on branch `claude/gsd-discuss-phase-1-mzdw5v`. No worktree was used. Before each commit: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test -p <touched crate>`. After the last commit: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (no failures) and `cargo deny check` (advisories, bans, licenses and sources ok). The CI-only paths were not run here: the Docker image build, the in-image `mt vectors replay --tree` over the committed sample, and the workflow steps. `runtime.c` and `testbed/vectors-sample` are unchanged (`git diff 7c77cd6 -- testbed crates/mt-harness/src/runtime.c` is empty), and the golden driver tests and the committed-sample tests pass, so no `driver_source` or `driver_elf` hash moved.

## Fixed Issues

### WR-01: Replay still does not compare most provenance fields, so an edited provenance record or a changed policy is not detected

**Status:** fixed in part
**Commit:** 90767fd
**Files modified:** `crates/mt-harness/src/replay.rs`, `crates/mt-harness/src/run.rs`, `crates/mt-harness/tests/eqv_04_replay_provenance.rs` (new)
**Applied fix:** `replay_corpus` now calls a new public `provenance_mismatches`, which compares these fields for every vector and reports each mismatch as `<sha> <field>` in `provenance_mismatch`:
- `seed`: sha256 of the vector's own stdin.
- `scalars`: the adapter's scalars for the combination read from `argv`.
- `executor_version`, `contract_version` and `budgets` against the current constants and `Budgets::DEFAULT`.
- `policy_sha256`: the module's committed policy file, read through the existing `policy_of` (now `pub(crate)`). An unreadable policy never matches.
- `record`: `Provenance::issues(vector)`, which includes the check that `outcome` equals the vector's own outcome class.

A new `check_corpus_location` refuses a corpus whose directory does not end in `<target>/<config>/<entry>` for the names its index holds. Offline tests run `provenance_mismatches` over all 15 committed sample vectors against the committed manifest and policies (zero mismatches, so the sample still replays on these fields), and check that each edited field is flagged on its own.
**Left alone (not compared):**
- `image`. It names the image that made the corpus and changes whenever the image is rebuilt, even when the tool output is identical. Comparing it would fail the committed sample on every image publish. The driver, archive and object hashes already pin the tool output.
- `unobserved`, `discarded_output_bytes`, `has_ub_source` and `replay`. Only a full re-run observes them.

The in-image replay of the sample could not be run here. The comparisons use only values the replay already holds or reads from the mounted `testbed` directory, which `run_guest` mounts whole.

### WR-02: A listing whose rows are all `no_roles` still prints `result: passed` and exits 0

**Status:** fixed
**Commit:** 975103a
**Files modified:** `crates/mt-harness/src/run.rs`, `crates/mt-cli/src/main.rs`
**Applied fix:** `BuildRow::did_work` (state `Built`) and `RunRow::did_work` (state `Ran`) were added. `build_command` and `run_command` now pass the count of rows that did work to `listing_failed`, so a non-empty listing of `no_roles` rows alone fails with exit 1. The printed row count is still `rows.len()`. Test `eqv_04_listing_of_only_no_roles_rows_is_a_failure`.

### IN-02: The contract document describes two behaviours the code does not have

**Status:** fixed
**Commit:** dd1deeb
**Files modified:** `docs/specs/equivalence-contract.md`, `crates/mt-contract/tests/eqv_01_contract_document.rs`
**Applied fix:** The `rc` row and the `ret` bullet now say that a struct or union return goes to `lib_state_out.ret` with `rc` 0, that an integer or enum return wider than 32 bits is refused when the adapter is built (by spelling or by the DWARF size), and that float and array returns are refused. The byte codec paragraph now describes the `serde_json` escapes (short escapes for `\b \t \n \f \r`, `\u00XX` for other control characters, other characters written as UTF-8) and says a vector file is `serde_json::to_vec_pretty` output with sorted maps and one trailing newline. The contract version and changelog line are unchanged, because a version bump would move every policy file and `policy_sha256` in the sample. Test `eqv_01_contract_document_matches_return_and_codec_behavior`.

### IN-03: Canonical form depends on `serde_json`'s sorted `Map`, and a few `[]` indexings rely on earlier checks without a comment

**Status:** fixed
**Commit:** 4ab3b0c
**Files modified:** `crates/mt-vectors/src/state.rs`, `crates/mt-harness/src/sample.rs`, `crates/mt-vectors/tests/eqv_02_state_and_compare.rs`
**Applied fix:** Invariant comments on the `object[*shape]`, `map["buffer"]` and `map["offset"]` indexings. In `sample.rs` the `corpus.index.vectors[sha]` index became `.get()` with an `Input` error. Test `eqv_02_serde_json_map_keeps_keys_sorted` fails if `preserve_order` is ever enabled.

### IN-04: Duplicated helpers

**Status:** fixed
**Commit:** 31e36b3
**Files modified:** `crates/mt-harness/src/driver.rs`, `crates/mt-cli/src/main.rs`
**Applied fix:** `build_driver` uses `link::healthy` and `link::failure` (same step text and detail format; the local `STDERR_EXCERPT` is gone). `selftest_command` calls `exec_context`. Pure refactor; the generated driver source is unchanged and existing tests cover both paths.

### IN-05: The image has no default non-root user

**Status:** fixed
**Commit:** 7df5e88
**Files modified:** `container/Dockerfile`, `crates/mt-toolchain/tests/container_static.rs`
**Applied fix:** `USER 65534:65534` as the last instruction of the `final` stage. Checked first: every `docker run` of the final image in the workflows already passes `--user 65534:65534`, and the smoke fallback passes an explicit `--user 0:0`, so none depends on the image default. `pin-discovery.yml` runs a different target (`snapshot-sources`). Nothing is built `FROM final`. Test `tool_03_final_stage_runs_as_the_unprivileged_user`. The image itself was not rebuilt here.

### IN-06: Recompile and flag handling is a denylist while the driver flags are an allowlist

**Status:** fixed in part
**Commit:** c74f962
**Files modified:** `crates/mt-harness/src/adapter.rs`, `crates/mt-harness/tests/eqv_03_adapter.rs`
**Applied fix:** `driver_flags` now returns `UnhandledFlag` for any argument that starts with `@` instead of dropping it as the source file. Test `eqv_03_driver_flags_refuse_a_response_file`.
**Not done:** making `recompile_arguments` (`probe.rs`) reuse the `driver_flags` allowlist. That function intentionally keeps every option the unit had so the DWARF layout matches the object. Narrowing it could change the typed layouts and so the recorded state and the driver, and it returns a plain `Vec` with no way to refuse. Its inputs come from a verified capture. This part stays open.

### IN-07: Policy loader silently ignores `.toml` files whose stem is not valid UTF-8

**Status:** fixed
**Commit:** ad8fc28
**Files modified:** `crates/mt-contract/src/policy.rs`, `crates/mt-contract/tests/eqv_01_policy.rs`, `crates/mt-contract/Cargo.toml`, `Cargo.lock`
**Applied fix:** `load_dir` reports a `*.toml` whose file stem is not UTF-8 as an issue. `tempfile` (already a workspace dependency) was added as a dev-dependency of `mt-contract` for the test `eqv_01_load_dir_reports_a_non_utf8_policy_file`.

### IN-08: The WR-07 fix split a doc comment: `is_decimal_integer` lost its doc and `fits_return_window` carries a wrong first line

**Status:** fixed
**Commit:** 09f4a96
**Files modified:** `crates/mt-contract/src/policy.rs`
**Applied fix:** The `/// An optional leading '-' ...` line moved to sit above `is_decimal_integer`. Doc-only; no test.

### IN-09: Include-prefix hardening is incomplete: trigraph splice, and the probe path is not covered

**Status:** fixed
**Commit:** 4e35d59
**Files modified:** `crates/mt-harness/src/driver.rs`, `crates/mt-harness/src/run.rs`, `crates/mt-harness/tests/eqv_03_driver_golden.rs`
**Applied fix:** `joins_or_comments` also refuses a line containing `??/`. `is_prefix_line` is now `pub(crate)` and `include_lines()` in `run.rs` checks every line once, so the probe sink and the driver sink see only checked lines. Both changes only refuse more; accepted lines render the same text, and the golden driver test is unchanged and passing. Test `eqv_03_driver_include_line_with_trigraph_splice_is_refused`. The false refusal of `/*` inside a string literal is kept (safe side).

## Skipped Issues

### WR-03: `struct mt_ksigaction` does not match the kernel `sigaction` layout (uninitialised padding becomes the signal mask)

**Status:** skipped
**Reason:** The fix edits `crates/mt-harness/src/runtime.c`, which feeds every `driver_source` and `driver_elf` hash recorded in the committed `testbed/vectors-sample`. The task forbids changing `runtime.c` and regenerating the sample. Still open: it should be done together with a CI regeneration of the sample.

### IN-01: `MAP_FIXED` mappings silently replace whatever is already at the address

**Status:** skipped
**Reason:** Same as WR-03. The fix (`MAP_FIXED_NOREPLACE`, the extra `_Static_assert`, bounding `mt_fields[1].len`) is in `runtime.c`, so it changes the driver hashes of the committed sample. Still open.

---

_Fixed: 2026-11-01_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 2_
