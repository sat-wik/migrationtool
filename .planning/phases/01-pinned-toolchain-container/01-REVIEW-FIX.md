---
phase: 01-pinned-toolchain-container
fixed_at: 2026-10-11T00:00:00Z
review_path: .planning/phases/01-pinned-toolchain-container/01-REVIEW.md
iteration: 1
findings_in_scope: 7
fixed: 7
skipped: 0
status: all_fixed
---

# Phase 1: Code Review Fix Report

**Fixed at:** 2026-10-11
**Source review:** .planning/phases/01-pinned-toolchain-container/01-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 7 (WR-01 to WR-07; CR-01 was already resolved, Info findings are out of scope)
- Fixed: 7
- Skipped: 0

Before each commit, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` were run in the main checkout (no worktree, as instructed). `cargo deny check` was run at the end: advisories, bans, licenses and sources are all ok. Local verification ran on the cloud session, not in the container. Items marked "CI validation pending" below have not run in the container build.

One process slip, corrected: the WR-06 commit was first made with two clippy errors in the new test code. They were fixed and folded into the same commit (amend, unpushed), and clippy was re-run at the WR-04 and WR-05 commits to confirm they were clean. Every commit now passes all three gates.

## Fixed Issues

### WR-01: A timeout kills only the direct child, and the timeout is then reported as a drain error

**Files modified:** `crates/mt-toolchain/src/runner.rs`, `crates/mt-toolchain/tests/tool_01_runner.rs`
**Commit:** 2bbd431
**Status:** fixed: requires human verification (supervision logic)
**Applied fix:**
- The child now leads its own process group (`CommandExt::process_group(0)`, safe std API).
- On timeout the runner kills the whole group by running `/bin/kill -s KILL -- -<pgid>` from inside `runner.rs`, and also calls `child.kill()`. The group is signalled while the child is still unreaped, so its pid cannot be reused.
- The wait loop is split into `supervise`. Any supervision error (`try_wait`, `kill`) and the "pipes not created" path now go through `abort`: group kill, `child.kill()`, `child.wait()`. No early return leaves a running or zombie child.
- If a timeout happens and a pipe is still held by a process that left the group (for example via `setsid`), the run returns a record with `timed_out: true` and that stream marked `truncated` (empty digest, zero bytes) instead of `OutputNotDrained`. A non-timeout undrained pipe still returns `OutputNotDrained`.
- New test `tool_01_runner_timeout_kills_descendants_and_reports_timed_out` runs `sh -c 'sleep 30 & sleep 30; wait'` with a 300 ms timeout and asserts it ends in under 4 s (below the 5 s drain grace) with `timed_out`. I confirmed that it fails (after about 10 s) with the group kill disabled.
- Limits: the group kill depends on `/bin/kill` existing (it does on ubuntu-24.04 and in the image's base). If it is missing, only the direct child is killed, as before. A descendant that calls `setsid` escapes the group; the unobserved-stream record covers that case.

### WR-02: `mt toolchain hash` lets curl read user config and expand URL globs

**Files modified:** `crates/mt-toolchain/src/fetch.rs`, `crates/mt-toolchain/tests/check_and_manifest.rs`
**Commit:** 3507c27
**Applied fix:** The curl argv is now built by a public `curl_argv(url)`: `curl -q --globoff --fail --silent --show-error --location --max-redirs 5 --proto =https,file --proto-redir =https <url>`. New tests assert `-q` is the first option and `--globoff` and `--max-redirs 5` are present, and a behavioural test hashes a file literally named `a[1-2].txt` through a `file://` URL (this fails if curl expands the glob). The optional small stderr cap for `FetchError::Failed` was not added (the output cap of 0 is unchanged).

### WR-03: The single-LLVM-major probe inspects only the first library in `ldd` output

**Files modified:** `crates/mt-toolchain/src/check.rs`, `crates/mt-toolchain/src/version.rs`, `crates/mt-toolchain/tests/check_and_manifest.rs`
**Commit:** 3145918
**Status:** fixed: requires human verification (check logic)
**Applied fix:** `regex_major` now collects every match of the `major` group across stdout and stderr (new `version::extract_all`). One distinct major is returned; several distinct majors give `Mismatch` ("more than one LLVM major in the output: 16, 17") with no `llvm_major`; no match is still `Mismatch`. New test `tool_02_check_fails_when_the_ldd_probe_lists_mixed_llvm_majors` covers two mixed fixtures and a repeated-same-major pass.
The existing c2rust test `tool_02_check_fails_when_any_tool_llvm_major_differs` had the bug baked in: the recorded `ldd` output lists both `libclang-cpp.so.16` and `libLLVM-16.so.1`, and the test changed only the first. It passed because the first match decided. It now changes both.

### WR-04: Any branch push (and the weekly schedule) publishes to GHCR; the cancel group covers the publish

**Files modified:** `.github/workflows/container.yml`, `crates/mt-toolchain/tests/container_static.rs`
**Commit:** 336ac81
**Applied fix:**
- The login and push steps are now gated on `github.event_name == 'push' && (github.ref == 'refs/heads/main' || github.ref == 'refs/heads/claude/gsd-discuss-phase-1-mzdw5v')`. Pull requests, the weekly schedule, `workflow_dispatch` and other branches no longer publish.
- Concurrency is now `container-${{ github.event_name }}-${{ github.ref }}` with `cancel-in-progress: ${{ github.event_name == 'pull_request' }}`, so a push run is never cancelled between `docker push` and the compare job, and a scheduled run does not share a group with a push run.
- Gating order is unchanged: the pin-binding gate and all other checks still come before login and push, and `verified-image` is still announced by the compare job only after the manifest diff. Actions remain SHA-pinned. `packages: write` is still in `image-a` only.
- The static test now requires the exact guard on exactly two steps, rejects `!= 'pull_request'`, and requires the new `cancel-in-progress` expression.
- Not done (kept to the instructed scope): moving publishing to a separate job that needs `compare`, and refusing to re-push an existing `sha-<commit>` tag. A re-run of the same commit on a gated branch can still push a new, differently built image to the same tag.
- CI validation pending: the workflow is valid YAML and the static tests pass, but the new conditions have not been exercised by a GitHub run.

### WR-05: The "process launching only in runner.rs" guard misses the idiom `runner.rs` itself uses

**Files modified:** `crates/mt-toolchain/tests/repo_checks.rs`
**Commit:** 8776a4c
**Applied fix:** New matcher `process_launch_violations` ignores full-line comments and flags: a `use` statement that names the process module together with `Command`, a glob import, or the module itself (`use std::process;`, `... as p`, `process::{self}`); a qualified `process::Command` path; and any mention of the `disallowed_types` lint (which only the runner may allow). `use std::process::ExitCode` and `std::os::unix::process::CommandExt` are allowed. `tool_01_command_only_in_runner_module` uses it. New tests: `tool_01_command_matcher_flags_grouped_imports_aliases_and_globs` (13 positive and 6 negative fixtures, including multi-line braces and a nested `std::{..., process::Command}`) and `tool_01_command_walk_flags_a_planted_file`. The fixtures are assembled from pieces so this file does not trip its own check.

### WR-06: The new static rules are whole-RUN and spelling-bound, so several ways back to an unbound install pass them (new, plan 01-09)

**Files modified:** `crates/mt-toolchain/tests/container_static.rs`, `container/Dockerfile`
**Commit:** 2fc0a1a
**Applied fix:** `run_issues` now splits each RUN into simple commands and parses each one (leading `NAME=value` assignments, wrappers such as `sudo`, `env` and `command`, `sh -c '...'`, `sh script`, absolute program paths, extra blanks, `cargo +toolchain`). The rules apply per command:
- **rustup:** every verb that can download (install, update, `toolchain add|install|update`, `target add`, `component add`, `default`, `show`, `self ...`) needs a `file://` `RUSTUP_DIST_SERVER` in that same command. Only `--version`, `help`, `which` and `toolchain|target|component list` are exempt. Installs and updates also need `--no-self-update`.
- **rustup-init:** needs `--default-toolchain none` (also `=none`).
- **cargo install / cargo binstall / cargo-binstall:** each needs `--locked`, an exact `--version =`, and `--path`. The path must lie under the `-C`/`--directory` of a `tar` extraction (short, clustered and long forms) of an archive that a `sha256sum -c` line in the RUN names, with no `..`.
- The RUN-wide checks that still make sense per RUN (pipe to shell, curl needs a checksum, git clone needs `rev-parse HEAD`, non-file dist server) are kept.
- Three new negative-test functions cover the cases above, including the review's examples (second unprefixed install, `rustup install`/`add`/`update`/`target add`/`component add`/`default`, `cargo  install`, `cargo +1.99.0 install`, `cargo binstall`, an unrelated checked file plus an unrelated unpacked archive plus an out-of-tree `--path`). All earlier fixtures and messages still hold.
- To satisfy the stricter rule the Dockerfile's `rustup default` now carries the `RUSTUP_DIST_SERVER="file:///tmp/rust-dist"` prefix. The toolchain already exists at that point, so behaviour is unchanged, and a missing toolchain would now fail instead of being fetched. CI validation of this Dockerfile edit is pending.
- Not done: the structural alternative `RUN --network=none` for the install step. Not done: variable indirection and `xargs`/`find -exec` spellings are not parsed; the check is a lint, not a sandbox.

### WR-07: The `Pin binding evidence` gate checks the Dockerfile's own bookkeeping, and its update-hash witness is tautological (new, plan 01-09)

**Files modified:** `.github/workflows/container.yml`, `container/Dockerfile`, `crates/mt-toolchain/tests/container_static.rs`
**Commit:** a2db70b
**Applied fix:**
- The container-side script now runs with `set -eu` (a failing read of the record or ledger now fails the gate instead of passing), and also prints `/opt/cargo-tools/.crates2.json`.
- `check_crate` additionally requires the ledger key `"<name> <version> (path+file:///tmp/crates/<name>-<version>)"` for each pinned crate. A registry source or a missing entry is a `MISMATCH` and fails the job before login and push. I confirmed the key format with a local `cargo install --path` on cargo 1.99.0, the same version as the image toolchain.
- The Dockerfile's rustup stage now fails the build if the update-hash rustup stored for either pinned toolchain is shorter than 16 characters or is not the start of that toolchain's pin (checked with dash). This asserts the update-hash against the pins at build time, while the mirror still exists.
- The gate's update-hash lines no longer say "is a prefix of the pin" as if corroborating. They say the value equals the leading characters of the pin from pins.toml and is a witness, because the Dockerfile wrote the sidecar from that pin. The notice header states the same.
- Tests: `tool_03_workflow_gates_on_pin_binding_evidence` now also requires the ledger path, the `path+file:///tmp/crates/${name}-${version}` source, `set -eu`, and the witness wording, and rejects the old wording. New `tool_03_dockerfile_asserts_the_rustup_update_hash_comes_from_the_pins`.
- I dry-ran the extracted gate step locally with a fake `docker`: a matching ledger gives exit 0 with a notice, and a registry-sourced c2rust gives a `MISMATCH` line and exit 1.
- **CI validation pending.** Not exercised in the container: the real `.crates2.json` contents under `--root /opt/cargo-tools`, the `set -eu` container script against the real image, and the new Dockerfile update-hash assertion (it assumes `${RUSTUP_HOME}/update-hashes/<version>-<host>` holds at least 16 characters, which matches the file names and values the existing gate already reads).
- Not done: gating each recorded archive line against its manifest's `xz_hash`, and the `RUN --network=none` install (see WR-06).

## Skipped Issues

None.

## CI validation needed

- WR-04: the new publish conditions and concurrency expression.
- WR-06: the Dockerfile `rustup default` mirror prefix (build must still pass).
- WR-07: the pin-binding gate against the real image (`.crates2.json` ledger check, `set -eu` container script) and the new in-build update-hash assertion.

Source-only fixes (WR-01, WR-02, WR-03, WR-05) are verified by the local test suite. WR-01 and WR-03 are marked as logic changes for a human to read.

---

_Fixed: 2026-10-11_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
