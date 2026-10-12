---
phase: 01-pinned-toolchain-container
plan: 01
subsystem: infra
tags: [rust, cargo-workspace, subprocess-runner, sha2, clap, toml, regex, clippy-disallowed-types]

requires: []
provides:
  - "Cargo workspace with mt-toolchain (lib) and mt-cli (bin `mt`), toolchain pinned to Rust 1.99.0"
  - "mt_toolchain::runner::run: single subprocess runner with env policy, hashed capped capture, timeout and versioned run record"
  - "mt_toolchain::pins (tracer subset of pins.toml) and mt_toolchain::check (version check producing OK/MISMATCH/MISSING rows)"
  - "`mt toolchain check --pins <PATH>` with exit codes 0 / 1 / 2"
  - "clippy.toml disallowed-types ban on std::process::Command outside runner.rs"
affects: [01-02, 01-03, 01-04, 01-05, 01-06, phase-11-evidence-bundles]

actuals:
  tokens: 9900
  tasks: 2
  commits: 3
plan_head_before: 7418bedf86921a17372ff3dc711829e8f64edcc4
plan_head_after: 6cca9d7d6ce5f58ea473430606dbb448b5367c0b

tech-stack:
  added: [serde 1.0.229, serde_json 1.0.151, sha2 0.11.0, thiserror 2.0.21, toml =1.1.7, regex 1.13.1, clap 4.6.7, anyhow 1.0.104, tempfile 3.27.0 (dev)]
  patterns:
    - "Runner is the only module allowed to use std::process (module-level allow of clippy::disallowed_types)"
    - "Records use BTreeMap, schema_version and no wall-clock fields so identical runs serialize identically"
    - "Thin CLI: mt-cli parses arguments and maps results to exit codes, logic lives in mt-toolchain"

key-files:
  created:
    - Cargo.toml
    - Cargo.lock
    - rust-toolchain.toml
    - .gitignore
    - clippy.toml
    - crates/mt-toolchain/Cargo.toml
    - crates/mt-toolchain/src/lib.rs
    - crates/mt-toolchain/src/runner.rs
    - crates/mt-toolchain/src/pins.rs
    - crates/mt-toolchain/src/check.rs
    - crates/mt-toolchain/tests/tool_01_runner.rs
    - crates/mt-cli/Cargo.toml
    - crates/mt-cli/src/main.rs
    - crates/mt-cli/tests/tool_01_check_end_to_end.rs
  modified: []

key-decisions:
  - "Workspace dependencies use plain caret versions with Cargo.lock committed, except toml which is pinned `=1.1.7` because cargo resolved the unaudited 1.1.8"
  - "Every error from the pins/check path (unreadable file, invalid TOML, bad regex, runner failure other than program-not-found) exits 2; only version drift or a missing tool exits 1"
  - "Check table has no header row: one line per tool (key, expected, actual or `-`, status)"
  - "RunnerConfig::new(path, source_date_epoch) added as a convenience constructor (default cap 16 MiB, 120 s timeout)"

patterns-established:
  - "Integration tests start with #![allow(clippy::unwrap_used, clippy::expect_used)]; non-test code never unwraps"
  - "Tests that need a subprocess launch it through runner::run, never the process API"

requirements-completed: [TOOL-01]

coverage:
  - id: D1
    description: "`mt toolchain check --pins <file>` drives a pins file through the runner to a real child process and prints OK, MISMATCH or MISSING per tool with exit 0 / 1 / 2"
    requirement: TOOL-01
    verification:
      - kind: integration
        ref: "crates/mt-cli/tests/tool_01_check_end_to_end.rs#tool_01_check_end_to_end_reports_ok_and_exits_zero"
        status: pass
      - kind: integration
        ref: "crates/mt-cli/tests/tool_01_check_end_to_end.rs#tool_01_check_end_to_end_reports_mismatch_and_exits_one"
        status: pass
      - kind: integration
        ref: "crates/mt-cli/tests/tool_01_check_end_to_end.rs#tool_01_check_end_to_end_reports_missing_tool_and_exits_one"
        status: pass
      - kind: integration
        ref: "crates/mt-cli/tests/tool_01_check_end_to_end.rs#tool_01_check_end_to_end_rejects_invalid_pins_with_exit_two"
        status: pass
    human_judgment: false
  - id: D2
    description: "Runner contract: cleared env plus fixed base, base-key collision rejected before spawn, output cap boundary, empty streams, timeout reported (never passed), typed spawn errors, byte-stable run record"
    requirement: TOOL-01
    verification:
      - kind: unit
        ref: "crates/mt-toolchain/tests/tool_01_runner.rs (9 tests, cargo test -p mt-toolchain --test tool_01_runner)"
        status: pass
    human_judgment: false
  - id: D3
    description: "std::process::Command is confined to runner.rs by clippy disallowed-types; workspace is fmt- and clippy-clean"
    requirement: TOOL-01
    verification:
      - kind: other
        ref: "cargo clippy --all-targets -- -D warnings && cargo fmt --check"
        status: pass
    human_judgment: true
    rationale: "Clean clippy shows no violation exists today, but no test yet proves the lint fires on a violation; the planned repository check (tool_01_command_only_in_runner_module, later plan) will assert it"

duration: 6min
completed: 2026-10-09
status: complete
---

# Phase 1 Plan 01: Pinned toolchain runner and `mt toolchain check` tracer Summary

**Cargo workspace whose `mt toolchain check --pins` drives a pins file through a hardened subprocess runner (cleared env, sha256-hashed capped capture, timeout kill, versioned byte-stable run record) to a real child and back to an OK/MISMATCH/MISSING table and exit code**

## Performance

- **Duration:** about 6 min (start time was not captured at launch; estimated from worktree creation at 16:35Z)
- **Started:** 2026-10-09T16:35:00Z (approximate)
- **Completed:** 2026-10-09T16:41:00Z (approximate)
- **Tasks:** 2
- **Files modified:** 14 created, 0 modified

## Accomplishments

- The tracer path is real end to end: pins file, `mt-cli`, `mt_toolchain::check::run_check`, `runner::run`, child process, regex version extraction, comparison, table and exit code. Four end-to-end tests launch the built `mt` binary through the runner.
- The runner is the single choke point for external tools: empty environment plus `PATH`, `LANG=C.UTF-8`, `TZ=UTC`, `SOURCE_DATE_EPOCH` plus named extras; a named extra colliding with a base key is rejected before anything spawns; every byte is hashed while only the first cap bytes (default 16 MiB) are stored; a timeout kills the child and is recorded as `timed_out` with the kill signal.
- Nine runner contract tests pin the edge cases (cap and cap+1, 5 MB streams with a 1 KiB cap, empty streams with the well-known empty sha256, empty argv, missing program, deterministic JSON with exact top-level keys). Mutating the runner (truncation comparison, removing `env_clear`, dropping `timed_out`) makes the matching tests fail.
- `clippy.toml` bans `std::process::Command` outside the runner module, and the workspace denies `unwrap_used` and `expect_used`.

## Task Commits

1. **Task 1: End-to-end `mt toolchain check` through the runner (tracer)** - `48461ab` (feat)
2. **Task 2: Harden the runner contract** - `cfdb718` (test; no GREEN commit needed, see TDD note)
3. **Dependency pin correction** - `6cca9d7` (fix, see Deviations)

**Plan metadata:** committed with this SUMMARY (docs: complete plan)

## Tracer gate

`cargo test -p mt-cli --test tool_01_check_end_to_end` passed (4 tests) and clippy and fmt were clean before Task 2 started. Tracer verified end-to-end, expansion proceeded.

## TDD Gate Compliance

Task 2 is `tdd="true"` inside an `execute` plan. RED evidence: the nine tests were written first and run before touching `runner.rs`; all nine passed on first run, because the Task 1 runner, specified in full by the plan and copied from the verified prototype, already implements the whole contract. This is the "unexpected GREEN in RED phase" case, so the RED step is **not valid RED evidence** (no failing target test) and `gsd_run check tdd-red-evidence` was not run (there is no failing report to classify). Investigation: the tests are not vacuous. Three single-line mutations of `runner.rs`, each reverted with `git checkout -- <file>` afterwards, each failed the expected tests:

| Mutation | Result |
|----------|--------|
| `truncated: total >= bytes_stored` | `tool_01_runner_cap_boundary_is_exact` and `tool_01_runner_empty_output_and_empty_argv` FAILED |
| `.env_clear()` removed | `tool_01_runner_clears_environment_and_applies_fixed_base` FAILED |
| `timed_out` recorded as `false` | `tool_01_runner_timeout_is_reported_not_passed` FAILED |

Gate commits: `test(01-01)` = `cfdb718`; `feat(01-01)` = `48461ab` (Task 1, precedes the test commit). The test commit therefore does not precede the implementation it covers; this is a TDD discipline deviation inherent to the plan's ordering (tracer implements first, hardening tests second). No REFACTOR commit.

## Files Created/Modified

- `Cargo.toml`, `Cargo.lock` - workspace (resolver 3, edition 2024, members mt-toolchain and mt-cli, `exclude = ["container/smoke"]`), dependency versions, clippy lints
- `rust-toolchain.toml` - channel 1.99.0, minimal profile, rustfmt and clippy, targets thumbv7em-none-eabihf and x86_64-unknown-linux-musl
- `.gitignore`, `clippy.toml` - ignore build output; disallow `std::process::Command`, allow unwrap/expect in tests
- `crates/mt-toolchain/src/runner.rs` - the subprocess runner and run record
- `crates/mt-toolchain/src/pins.rs` - tracer-subset pins schema with `deny_unknown_fields`
- `crates/mt-toolchain/src/check.rs` - version check, `CheckReport`, table rendering
- `crates/mt-toolchain/src/lib.rs` - `forbid(unsafe_code)` and module declarations
- `crates/mt-cli/src/main.rs` - `mt toolchain check --pins` argument parsing and exit-code mapping
- `crates/mt-toolchain/tests/tool_01_runner.rs`, `crates/mt-cli/tests/tool_01_check_end_to_end.rs` - TOOL-01 tests (9 and 4)

## Decisions Made

- Caret versions plus a committed `Cargo.lock` for workspace dependencies; `toml` alone is exact (`=1.1.7`) because it was the one crate where resolution drifted from the audited version.
- All pins/check errors exit 2 (invalid input) rather than only unreadable or unparsable files, so an unusable regex or a runner failure other than "program not found" is distinguishable from version drift (exit 1).
- No header row in the check table, to match the "one row per tool" wording and keep the output machine-friendly.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Pinned toml to the audited 1.1.7**
- **Found during:** Task 2 wrap-up (Cargo.lock review)
- **Issue:** `toml = "1.1.7"` resolved to 1.1.8, which is not the version approved in the 01-RESEARCH package legitimacy audit; threat T-01-SC requires audited versions only.
- **Fix:** `toml = "=1.1.7"` in `[workspace.dependencies]` and `cargo update -p toml --precise 1.1.7`.
- **Files modified:** `Cargo.toml`, `Cargo.lock`
- **Verification:** fmt, clippy `-D warnings`, `cargo test --workspace --locked` all pass.
- **Committed in:** `6cca9d7`

---

**Total deviations:** 1 auto-fixed (1 missing critical), plus the TDD ordering note above.
**Impact on plan:** None on scope; files stay within `files_modified` (`Cargo.toml`, `Cargo.lock`). Transitive dependencies were not audited by the plan and resolve freely in `Cargo.lock`; `cargo deny check` arrives in a later plan.

## Issues Encountered

- The first `cargo` invocation installed the pinned 1.99.0 toolchain, thumbv7em-none-eabihf and musl targets through rustup without trouble.
- A missing working directory surfaces from the OS as `NotFound`, which `RunnerError::is_not_found` would report like a missing program (so `check` would show MISSING). `mt` passes the current directory, which always exists, so this does not occur today; worth a distinct error if `run` gains other callers.

## Known Stubs

None.

## Threat Flags

None. `mt` reads a pins file and launches tools named in it, which is the pins-file to mt trust boundary already in the plan's threat model (T-01-01 to T-01-05, T-01-SC).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plan 01-03 can extend `pins::Pins` (the tracer subset rejects unknown keys, so new sections must be added to the structs) and reuse `check::run_check`.
- Plan 01-02 (CI) can run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace --locked`; none needs Docker or network beyond crates.io and the Rust toolchain download.
- The PROJECT.md decision recording the runner as the one permitted `std::process` user is still owed by plan 01-06 (the runner source comment says so).

## Self-Check: PASSED

All 14 created files exist; commits `48461ab`, `cfdb718` and `6cca9d7` are ancestors of HEAD. Plan-level verification re-run: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test --workspace --locked` exit 0 (4 end-to-end tests and 9 runner tests pass). Acceptance criteria re-checked: `ls crates` lists exactly `mt-cli` and `mt-toolchain`; `forbid(unsafe_code)` present in `lib.rs` and `main.rs`; `runner.rs` contains `env_clear()`, `pub fn run(` and `#![allow(clippy::disallowed_types)]`; `rust-toolchain.toml` has `channel = "1.99.0"` and `thumbv7em-none-eabihf`; root `Cargo.toml` has `exclude = ["container/smoke"]`.

---
*Phase: 01-pinned-toolchain-container*
*Completed: 2026-10-09*
