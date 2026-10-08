---
phase: "1"
slug: "pinned-toolchain-container"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-10-08"
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Source: `01-RESEARCH.md` § Validation Architecture.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test harness (`cargo test`); optional `proptest` for version-parser properties |
| **Config file** | none yet — Wave 0 creates `Cargo.toml`, `rust-toolchain.toml`, `deny.toml`, `clippy.toml` |
| **Quick run command** | `cargo test -p mt-toolchain` |
| **Full suite command** | `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace --locked && cargo deny check` |
| **Estimated runtime** | ~60 seconds after first build (cargo only); container workflow 20-60 min in CI |

"Cargo" rows run in a Docker-less session. "CI/container" rows are only provable by the GitHub Actions container workflow (D-23).

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p mt-toolchain` (or the crate the task touches)
- **After every plan wave:** Run the full suite command
- **Before `/gsd-verify-work`:** Full suite green plus container workflow green (double-build manifest diff empty, `mt toolchain check` exit 0, off-pin run exit non-zero, smoke build)
- **Max feedback latency:** 120 seconds for cargo checks; CI loops are the documented exception

---

## Per-Task Verification Map

Requirement-level map; the planner binds task IDs (`1-PP-TT`) to these rows in each PLAN.md `<verify>` block.

| Requirement | Behavior | Test Type | Automated Command / Test | Where | Status |
|-------------|----------|-----------|--------------------------|-------|--------|
| TOOL-01 | No Python outside `research/` | repo scan | `tool_01_no_python_outside_research` | Cargo | ⬜ pending |
| TOOL-01 | Runner records argv, cwd, env, exit, sha256 of stdout/stderr, schema_version | unit | `tool_01_runner_records_argv_env_exit_and_output_hashes` | Cargo | ⬜ pending |
| TOOL-01 | Env cleared, fixed base + named extras | unit | `tool_01_runner_clears_environment_and_applies_fixed_base` | Cargo | ⬜ pending |
| TOOL-01 | Output cap + truncated flag, no deadlock | unit | `tool_01_runner_caps_output_without_deadlock` | Cargo | ⬜ pending |
| TOOL-01 | Timeout reported, never a pass | unit | `tool_01_runner_timeout_is_reported_not_passed` | Cargo | ⬜ pending |
| TOOL-01 | Run record JSON byte-stable | golden | `tool_01_run_record_json_is_deterministic` | Cargo | ⬜ pending |
| TOOL-01 | `std::process::Command` only in runner | lint/scan | clippy `disallowed-types` + `tool_01_command_only_in_runner_module` | Cargo | ⬜ pending |
| TOOL-01 | Every tool crate `#![forbid(unsafe_code)]` | scan | `tool_01_every_tool_crate_forbids_unsafe` | Cargo | ⬜ pending |
| TOOL-01 | Analyser launched through runner in container with version stamp | integration | `mt toolchain check` exit 0 in image | CI/container | ⬜ pending |
| TOOL-02 | Version/LLVM-major parsing from fixture outputs | unit | `tool_02_parses_llvm_major_from_tool_outputs` | Cargo | ⬜ pending |
| TOOL-02 | Any differing LLVM major fails the check | unit | `tool_02_check_fails_when_any_tool_llvm_major_differs` | Cargo | ⬜ pending |
| TOOL-02 | Bitcode rustc LLVM > 19 fails, 16.0.5 passes | unit | `tool_02_bitcode_rustc_llvm_above_19_fails` / `tool_02_bitcode_rustc_llvm_16_passes` | Cargo | ⬜ pending |
| TOOL-02 | `pins.toml` major in 16..=19 and provisional | unit | `tool_02_pins_llvm_major_in_supported_range_and_provisional` | Cargo | ⬜ pending |
| TOOL-02 | Pins carry a matching PROJECT.md decision (D-21) | unit | `tool_02_llvm_and_bitcode_pins_have_matching_project_decision` | Cargo | ⬜ pending |
| TOOL-02 | Live versions match pins; off-pin run fails | integration | `mt toolchain check` exit 0; edited pins (`major = 17`) exit non-zero | CI/container | ⬜ pending |
| TOOL-03 | sha256/commit/digest well-formed, no placeholders | unit | `tool_03_pins_digests_and_commits_are_well_formed` | Cargo | ⬜ pending |
| TOOL-03 | Dockerfile has no floating references | static lint | `tool_03_dockerfile_has_no_floating_references` | Cargo | ⬜ pending |
| TOOL-03 | Dockerfile ARGs match `mt toolchain build-args` keys | unit | `tool_03_dockerfile_args_match_pins` | Cargo | ⬜ pending |
| TOOL-03 | Manifest byte-stable; diff detects change | unit | `tool_03_manifest_is_byte_stable_and_diff_detects_change` | Cargo | ⬜ pending |
| TOOL-03 | Two builds give identical manifests | integration | CI build A + build B (`--no-cache`), `diff -u` manifests | CI/container | ⬜ pending |
| TOOL-04 | Smoke crate shape (`no_std`, staticlib+rlib, `panic = "abort"`, no deps) | unit | `tool_04_smoke_crate_has_emitted_rust_shape` | Cargo | ⬜ pending |
| TOOL-04 | Smoke crate excluded from root workspace | unit | `tool_04_smoke_crate_is_outside_the_workspace` | Cargo | ⬜ pending |
| TOOL-04 | Smoke crate builds for `thumbv7em-none-eabihf`; arm-none-eabi-gcc present at pin | integration | CI `cargo build --release --target thumbv7em-none-eabihf --manifest-path container/smoke/Cargo.toml` | CI/container | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] Root `Cargo.toml`, `rust-toolchain.toml`, `deny.toml`, `clippy.toml`, `.gitignore` (`target/`), `research/.gitkeep`
- [ ] `crates/mt-toolchain` and `crates/mt-cli` skeletons with the tests named above
- [ ] `crates/mt-toolchain/tests/fixtures/` tool-output fixtures
- [ ] `container/pins.toml`, `container/Dockerfile`, `container/smoke/`
- [ ] `.github/workflows/ci.yml`, `.github/workflows/container.yml`
- [ ] PROJECT.md decision for the LLVM 16 / bitcode rustc 1.72.1 pin (decision-ID gate)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Container workflow results reviewed | TOOL-01..04 | Docker unavailable in cloud sessions; proven only by GitHub Actions | Open the container workflow run for the phase head; confirm compare, check, off-pin and smoke jobs are green |
| c2rust package legitimacy | TOOL-01 | Low-download crate flagged SUS by legitimacy check | Confirm crate owners (Immunant) and repo link before pinning |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s (cargo checks)
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
