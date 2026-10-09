---
phase: 01-pinned-toolchain-container
plan: 04
subsystem: infra
tags: [rust, cargo-deny, repo-guards, no_std, thumbv7em-none-eabihf, toml]

requires:
  - phase: 01-pinned-toolchain-container
    provides: "Cargo workspace, runner.rs as the single process user, clippy.toml, root exclude for container/smoke (plan 01-01)"
provides:
  - "crates/mt-toolchain/tests/repo_checks.rs: seven repository tests (five TOOL-01 guards, two TOOL-04 shape tests) runnable with plain cargo test"
  - "deny.toml: licence allowlist, crates.io-only sources, analyser-binding ban list; `cargo deny check` passes"
  - "research/.gitkeep: the only directory where Python may live"
  - "container/smoke: mt-smoke no_std crate (own workspace) with C ABI export mt_smoke_add and C caller smoke.c"
affects: [01-05, 01-06, 01-07, phase-02-gates, phase-04-boundary]

actuals:
  tokens: 3900
  tasks: 2
  commits: 4
plan_head_before: 53c7da24be698be8877c3bececb55af597d60ad0
plan_head_after: b0debef2b2ac8a8c652d950e8c2d2d0fc7bae5ea

tech-stack:
  added: ["cargo-deny 0.20.2 (tool install only, not a project dependency)"]
  patterns:
    - "Repository guards are plain cargo tests in mt-toolchain/tests, with a root-parameterised walk so a planted-file test can reuse it"
    - "Emitted-Rust lint table is asserted verbatim against the smoke manifest, so drift from docs/specs/emitted-rust-rules.md section 6 fails cargo test"
    - "Needle strings that a guard searches for are assembled from pieces so the guard file does not trip itself"

key-files:
  created:
    - crates/mt-toolchain/tests/repo_checks.rs
    - deny.toml
    - research/.gitkeep
    - container/smoke/Cargo.toml
    - container/smoke/Cargo.lock
    - container/smoke/src/lib.rs
    - container/smoke/src/ffi.rs
    - container/smoke/smoke.c
  modified: []

key-decisions:
  - "Python walk returns root-relative sorted paths, skipping .git, target and research only directly under the walk root, and never following symlinks"
  - "The smoke crate lints are asserted by deserialising the section 6 table from a literal in the test and comparing it to the manifest, rather than spot-checking keys"
  - "lib.rs shape test accepts line comments and docs before `#![no_std]` by checking the first code line, so the inner crate docs can follow the attribute"

patterns-established:
  - "RED for a guard whose repo already conforms: stub only the helper under test, so the planted-input test fails on its assertion while the conforming-repo tests pass"

requirements-completed: []

coverage:
  - id: D1
    description: "Plain cargo test fails when Python source exists outside research/ (walk skips only root .git, target, research; never follows symlinks)"
    requirement: TOOL-01
    verification:
      - kind: unit
        ref: "crates/mt-toolchain/tests/repo_checks.rs#tool_01_no_python_outside_research"
        status: pass
      - kind: unit
        ref: "crates/mt-toolchain/tests/repo_checks.rs#tool_01_python_walk_flags_a_planted_file"
        status: pass
    human_judgment: false
  - id: D2
    description: "std process Command confined to runner.rs, forbid(unsafe_code) in every tool crate root, no analyser binding crate in Cargo.lock"
    requirement: TOOL-01
    verification:
      - kind: unit
        ref: "crates/mt-toolchain/tests/repo_checks.rs#tool_01_command_only_in_runner_module"
        status: pass
      - kind: unit
        ref: "crates/mt-toolchain/tests/repo_checks.rs#tool_01_every_tool_crate_forbids_unsafe"
        status: pass
      - kind: unit
        ref: "crates/mt-toolchain/tests/repo_checks.rs#tool_01_never_links_an_analyser_library"
        status: pass
    human_judgment: false
  - id: D3
    description: "cargo-deny policy passes on the current dependency set with the approved licence list, crates.io-only sources and the analyser ban list"
    requirement: TOOL-01
    verification:
      - kind: other
        ref: "cargo deny check (advisories ok, bans ok, licenses ok, sources ok)"
        status: pass
    human_judgment: false
  - id: D4
    description: "container/smoke is a no_std, dependency-free staticlib+rlib crate with the emitted-Rust lint table, unsafe only in ffi.rs, outside the tool workspace, that builds and passes clippy for thumbv7em-none-eabihf"
    requirement: TOOL-04
    verification:
      - kind: unit
        ref: "crates/mt-toolchain/tests/repo_checks.rs#tool_04_smoke_crate_has_emitted_rust_shape"
        status: pass
      - kind: unit
        ref: "crates/mt-toolchain/tests/repo_checks.rs#tool_04_smoke_crate_is_outside_the_workspace"
        status: pass
      - kind: other
        ref: "cargo build --release --target thumbv7em-none-eabihf --manifest-path container/smoke/Cargo.toml"
        status: pass
      - kind: other
        ref: "cargo clippy --manifest-path container/smoke/Cargo.toml --target thumbv7em-none-eabihf -- -D warnings"
        status: pass
    human_judgment: false
  - id: D5
    description: "smoke.c compiles with arm-none-eabi-gcc and links against libmt_smoke.a inside the pinned container"
    requirement: TOOL-04
    verification: []
    human_judgment: true
    rationale: "arm-none-eabi-gcc is not installed in this session and no Docker daemon exists; the in-container C build is proven by plan 01-07, so the C half of TOOL-04 is unverified here"

duration: 12min
completed: 2026-10-09
status: complete
---

# Phase 1 Plan 04: Repository guards, cargo-deny policy and no_std smoke crate Summary

**Seven plain-cargo-test repository guards (no Python outside research/, Command confined to the runner, forbid(unsafe_code), no analyser bindings, smoke-crate shape), a cargo-deny 0.20.2 policy that passes, and an `mt-smoke` no_std crate that builds and lints clean for thumbv7em-none-eabihf**

## Performance

- **Duration:** about 12 min (start time was not captured at launch; estimated from the first commit and the cargo-deny build)
- **Started:** 2026-10-09T16:36:00Z (approximate)
- **Completed:** 2026-10-09T16:48:06Z
- **Tasks:** 2
- **Files modified:** 8 created, 0 modified

## Accomplishments

- A repository check now fails on any `.py`, `.pyi`, `.pyw` or `.pyx` file outside `research/`. Verified live: planting `docs/planted_check.py` made `tool_01_no_python_outside_research` fail naming the file; the file was then removed.
- `cargo deny check` prints `advisories ok, bans ok, licenses ok, sources ok`. The advisory database fetch worked through the proxy, so the policy was not weakened. The analyser ban list and `tool_01_never_links_an_analyser_library` both cover clang-sys, llvm-sys, inkwell, z3 and z3-sys.
- `container/smoke` builds in release for `thumbv7em-none-eabihf` (`libmt_smoke.a`, 8.4 MB because it bundles `core`) and passes `clippy -D warnings`. `cargo build --workspace` never compiles it (its output contains no `mt-smoke` line).
- Tool-crate hygiene is now guarded: `process::Command` text appears only in `runner.rs`, and every `crates/*/src/lib.rs` and `main.rs` carries `#![forbid(unsafe_code)]`.

## Task Commits

1. **Task 1: Repository guards and cargo-deny policy** - `f66b9ff` (test, RED), `aaa318b` (feat, GREEN)
2. **Task 2: no_std smoke crate, C caller and shape tests** - `fa429a3` (test, RED), `b0debef` (feat, GREEN)

**Plan metadata:** committed with this SUMMARY (docs: complete plan)

## TDD Gate Compliance

Both tasks are `tdd="true"` inside an `execute` plan; RED commits precede GREEN commits for each task.

- **Task 1 RED (`f66b9ff`):** the Python walk was stubbed to return an empty list. Target test `tool_01_python_walk_flags_a_planted_file` failed on its planned assertion (`left: []`, `right: ["a/b.py"]`), the other four tests passed. This is a partial RED: `tool_01_no_python_outside_research`, `tool_01_command_only_in_runner_module`, `tool_01_every_tool_crate_forbids_unsafe` and `tool_01_never_links_an_analyser_library` pass at RED because the repository already conforms, so they have no failing phase. Their non-vacuity was shown separately for the Python guard (planted file made it fail); the other three were not mutation-tested. `gsd_run check tdd-red-evidence` was not run: no persisted evidence record was produced, and the semantic assessment is by direct inspection of the cargo test output above (target executed, failed on the planned assertion, for the intended reason).
- **Task 1 GREEN (`aaa318b`):** walk implemented, `deny.toml` and `research/.gitkeep` added; 5 of 5 tests pass, `cargo deny check` passes.
- **Task 2 RED (`fa429a3`):** both `tool_04_*` tests failed on the planned assertion `container/smoke/Cargo.toml is missing` (an explicit `assert!`, not an unwrap crash). Same note: no `tdd-red-evidence` record was persisted.
- **Task 2 GREEN (`b0debef`):** smoke crate added; 7 of 7 tests pass, target build and clippy pass.
- No REFACTOR commit.

## Files Created/Modified

- `crates/mt-toolchain/tests/repo_checks.rs` - the seven repository tests and their walk helpers
- `deny.toml` - cargo-deny licence, advisory, source and analyser-binding ban policy
- `research/.gitkeep` - keeps the only Python-permitted directory in git
- `container/smoke/Cargo.toml`, `Cargo.lock` - own-workspace manifest with the emitted-Rust shape and lint table; lock lists only mt-smoke
- `container/smoke/src/lib.rs` - `#![no_std]`, `add` using `wrapping_add`, panic handler behind the `panic-handler` feature
- `container/smoke/src/ffi.rs` - the only `unsafe`-bearing module: `#[no_mangle] extern "C" mt_smoke_add`
- `container/smoke/smoke.c` - C caller `mt_smoke_caller` that calls `mt_smoke_add(x, 1u)`

## Decisions Made

- The walk returns root-relative sorted paths so the planted-file test can assert equality against `a/b.py`.
- Lint-table drift is detected by comparing the whole `[lints]` table with a literal copy of emitted-rust-rules.md section 6, not by checking individual keys.
- The `#![no_std]` shape check looks at the first line that is not blank or a line comment, which tolerates leading comments and keeps crate docs below the attribute.

## Deviations from Plan

None - plan executed exactly as written. All changes stay within `files_modified`.

## Issues Encountered

- `cargo-deny` was not installed; installed with `cargo install --locked cargo-deny --version =0.20.2` (the audited version in the T-01-SC threat register). Compile took about 2 minutes. The advisory DB fetch succeeded, so no deny.toml weakening was needed.
- `arm-none-eabi-gcc` is not available in this session, so `smoke.c` was not compiled here. That check belongs to plan 01-07.
- Sandbox rules refused compound git/shell commands in this worktree, so commands were run as single plain commands.

## Verification Results

Run in this worktree before the SUMMARY commit:

| Command | Result |
| --- | --- |
| `cargo fmt --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo test --workspace --locked` | pass: mt-cli e2e 4, repo_checks 7, tool_01_runner 9 |
| `cargo deny check` | advisories ok, bans ok, licenses ok, sources ok |
| `cargo build --release --target thumbv7em-none-eabihf --manifest-path container/smoke/Cargo.toml` | exit 0, `libmt_smoke.a` exists |
| `cargo clippy --manifest-path container/smoke/Cargo.toml --target thumbv7em-none-eabihf -- -D warnings` | exit 0 |
| `cargo fmt --check --manifest-path container/smoke/Cargo.toml` | exit 0 |

Acceptance criteria: `git ls-files research/.gitkeep` prints the path; `deny.toml` contains `clang-sys` and `allow-wildcard-paths = true`; `lib.rs` contains `#![no_std]` and `wrapping_add`; `ffi.rs` contains `// SAFETY:` and `mt_smoke_add`.

## Known Stubs

None.

## Threat Flags

None. The repo_checks walk reads paths only and never follows symlinks (T-01-11); licence, source and binding bans are enforced by deny.toml and the Cargo.lock test (T-01-12, T-01-13); cargo-deny was installed at the audited version (T-01-SC).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plan 01-07 can reuse `cargo build --release --target thumbv7em-none-eabihf --manifest-path container/smoke/Cargo.toml` inside the container and compile `smoke.c` with arm-none-eabi-gcc; the symbol `mt_smoke_add` is exported with a C ABI.
- Requirement bookkeeping: TOOL-04 is only half proven (host build); it should not be marked complete until plan 01-07 proves the in-container build. `requirements-completed` is left empty here and REQUIREMENTS.md was not touched; the orchestrator decides TOOL-01 and TOOL-04 status after the wave.
- Plan 01-03 runs in parallel and owns other files in `crates/mt-toolchain`; `repo_checks.rs` scans `crates/` so any file it adds is covered by the same guards (no `process::Command` text, `forbid(unsafe_code)` in new crate roots).

## Self-Check: PASSED

All eight created files exist; commits `f66b9ff`, `aaa318b`, `fa429a3` and `b0debef` are ancestors of HEAD; `git rev-list --count 53c7da2..HEAD` was 4 before the SUMMARY commit.

---
*Phase: 01-pinned-toolchain-container*
*Completed: 2026-10-09*
