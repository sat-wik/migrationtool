---
phase: 01-pinned-toolchain-container
reviewed: 2026-10-12T05:00:00Z
update_diff_base: 53faae5d5d4deb0e0f2fd3cf76f22a1f59ea4092
depth: standard
files_reviewed: 12
files_reviewed_list:
  - .github/workflows/ci.yml
  - .github/workflows/container.yml
  - .github/workflows/pin-discovery.yml
  - Cargo.toml
  - container/Dockerfile
  - container/README.md
  - container/pins.toml
  - crates/mt-toolchain/src/check.rs
  - crates/mt-toolchain/src/fetch.rs
  - crates/mt-toolchain/src/runner.rs
  - crates/mt-toolchain/tests/container_static.rs
  - crates/mt-toolchain/tests/tool_01_runner.rs
findings:
  critical: 0
  warning: 1
  info: 21
  total: 22
status: issues_found
---

# Phase 1: Code Review Report

**Reviewed:** 2026-10-12
**Depth:** standard
**Files Reviewed:** 12 (the files changed since `53faae5d5d4deb0e0f2fd3cf76f22a1f59ea4092`: workflows, Cargo.toml, Dockerfile, README, pins, runner, check, fetch and two test files)
**Status:** issues_found

## Summary

This pass re-reviews the fixes for WR-08..WR-12: the pinned klee-uclibc locale tarball with an offline `make`, publishing from `main` only, the cfg-gated runner for macOS, the `complete` flag with partial output (schema version 2), the `waitid(WNOWAIT)` descendant kill after a normal exit, and `FetchError::Incomplete`.

I traced the runner's supervise, kill and collect paths by hand. I ran `cargo test -p mt-toolchain --test tool_01_runner` (17 passed). I installed `aarch64-apple-darwin` temporarily and ran `cargo check` and `cargo clippy -D warnings` for it on the whole workspace (both clean), then removed the target. I did not rebuild the image or run CI.

All five fixes work as described, and I found no critical issue. The runner's timeout path is sound:
- The child leads its own group and is never reaped before the kill.
- `waitid(NOHANG | NOWAIT)` keeps an exited child as a zombie, so its pid and group id cannot be reused before the group kill.
- Escaped descendants are pinned by pidfd plus a start-time check, so pid reuse cannot redirect a signal.

The one defect worth fixing is in the new post-exit path (WR-13). After a normal exit the `/proc` parent walk cannot find any escaped descendant, because the kernel has already reparented them. The record still says `descendants_killed: true`. `check.rs` and `fetch.rs` treat incomplete streams correctly: neither parses nor hashes a partial stream.

No structural (fallow) findings were supplied for this review.

## Prior findings

| ID | Verdict | Evidence |
|----|---------|----------|
| CR-01 | resolved | Unchanged by this diff. The Dockerfile change touches only the `klee-build` stage; the mirror, the offline install and the gate are as before (`container/Dockerfile:200-403`, `.github/workflows/container.yml:252-438`). |
| WR-01 | resolved | Timeout path intact: `process_group(0)` at `runner.rs:667`; `kill_tree` (STOP group, pidfd walk with start-time check, KILL held pidfds, KILL group) at `:511-550`; `abort` on early returns at `:565-569`, `:677`, `:688`, `:709`. The child is unreaped when `kill_tree` runs on every path (`:605-620`, `:697-705`), so the group id cannot be reused. One race I traced: the child exits between the `waitid` poll and the STOP, its children are reparented, and an escapee is missed on a timeout. The group kill still works, and WR-13 covers the same limit. |
| WR-02 | resolved | Unchanged: `curl_argv` at `fetch.rs:50-67`. |
| WR-03 | resolved | Unchanged. |
| WR-04 | resolved | Previously partial because of the hard-coded branch. The guard is now `github.event_name == 'push' && github.ref == 'refs/heads/main'` at `container.yml:598`, `:627` and `:845`, and `publish` holds the only `packages: write` (`:850`). `container_static.rs:2407-2411` and the new assertions at `:2477-2492` fix the guard and the three-occurrence count. |
| WR-05 | resolved | Unchanged. |
| WR-06 | resolved | Every install and build step is `RUN --network=none`; the only networked RUNs download or clone and check against a pin in the same RUN. The klee-uclibc `make` moved offline (`Dockerfile:449-451`), `NETWORKED_STEPS` is empty (`container_static.rs:620`), and the new test forbids a `# Remains networked:` label. |
| WR-07 | resolved | Unchanged. |
| WR-08 | resolved | `[source.klee_uclibc_locale]` at `container/pins.toml:84-94`. The clone RUN downloads the tarball into `extra/locale/`, asserts the Makefile names that file, and runs `sha256sum -c` against the pin (`Dockerfile:436-445`). `make` is `RUN --network=none` (`:449-451`). `pin-discovery.yml:115-123` recomputes the checksum. The header comment is updated (`Dockerfile:17-22`). Residuals are in IN-09 and IN-11. |
| WR-09 | resolved | `container.yml:598`, `:627` and `:845` name only `refs/heads/main`. The test requires exactly three `refs/heads/` occurrences and no `claude/` ref (`container_static.rs:2477-2492`). |
| WR-10 | resolved | The pidfd and `/proc` code is `#[cfg(target_os = "linux")]` (`runner.rs:46-61`, `:409-550`). A portable `kill_process_group` fallback is at `:558-561`, and a non-unix host hits `compile_error!` (`:39-43`). The README states the macOS limits (`README.md:110-116`). `ci.yml:35-40` type-checks `aarch64-apple-darwin`. I reproduced a clean `cargo check` and `cargo clippy -D warnings` for that target. |
| WR-11 | partially | The mechanism is sound. `waitid(EXITED \| NOHANG \| NOWAIT)` leaves the zombie, which holds the pid (`runner.rs:575-587`). After `drain_grace` the group is killed and the record is returned with the real exit code (`:693-712`). A group member holding a pipe is killed, as the new test shows (`tool_01_runner.rs`). A descendant that left the group through `setsid` is never found after a normal exit, so the process is left running and the old failure persists for it. See WR-13. |
| WR-12 | resolved | `StreamRecord::complete` with partial bytes and the digest of exactly those bytes: `runner.rs:279-292`, `:361-389`. `truncated` again means only "cap exceeded" (`:170-172`). `RUN_RECORD_SCHEMA_VERSION` is 2 (`:66`). The tests assert the key set and the partial digest. |

## Warnings

### WR-13: After a normal exit the descendant sweep cannot find escaped processes, yet the record says `descendants_killed: true`

**File:** `crates/mt-toolchain/src/runner.rs:464-481`, `:511-550`, `:693-705`, `:149-157`; `crates/mt-toolchain/tests/tool_01_runner.rs` (`..._normal_exit_with_an_orphaned_escapee_is_incomplete_not_an_error`)
**Issue:** `kill_tree` finds escapees by walking `ppid` links down from the child (`descendants(&table, root)`). That works on the timeout path because the child is still alive. On the new post-exit path the child is a zombie, and the kernel reparented its living children to init (or a subreaper) at the moment it exited. So `descendants(.., root)` returns the empty set for every escapee, not only for the double-fork case the fix report lists. The scan makes one `/proc` pass and stops.
- Only processes still in the child's own group are killed, by the group `SIGKILL`.
- Failure path: a tool runs `setsid daemon &` (or a tool that daemonizes) and exits 0. The daemon keeps stdout open. After `drain_grace` the runner calls `kill_tree`, kills nothing, and sets `descendants_killed = true`. It then waits `drain_grace` again, returns `stdout.complete: false`, and the daemon keeps running. WR-11 asked for exactly this case to be cleaned up.
- The test above passes only because it asserts the opposite of the success path: it checks `descendants_killed == true` and `!stdout.complete` together, so it records that the flag is true while the escapee is alive.
- `descendants_killed` is documented as "the runner killed the child's process tree" (`:154-156`) and the module header says escapees are found by the `/proc` walk (`:13-24`). Phase 11 will read this flag as evidence that no tool process was left behind. It records only that the kill was attempted.
- `REVIEW-FIX.md` lists the residual as a "double fork whose middle process already exited". That understates it: a direct `setsid` child of the tool is also missed after a normal exit.

**Fix:** Pick one or both of these:
- Find the holders of the pipe instead of walking ancestry. Before the readers are spawned, take the inode of each pipe end (`fstat` on the `ChildStdout`/`ChildStderr` fd, or `readlink /proc/self/fd/N`). After the grace period, scan `/proc/*/fd/*` for links equal to `pipe:[inode]`. Pin each holder with `pidfd_open` plus the same start-time check and `SIGKILL` it. This works whatever the ancestry.
- Or set `descendants_killed` only when `kill_tree` signalled at least one process other than the group leader. Change the doc text to "a kill was attempted", and rename the field (`tree_kill_attempted`) so a record cannot be read as "nothing was left running".

Then correct the module header and the residual list, and add a test that an orphaned `setsid` holder is either killed or leaves `descendants_killed == false`.

## Info

### IN-01: Truncated captures are treated as complete observations

**File:** `crates/mt-toolchain/src/check.rs:466-499`
**Issue:** Still applies. `run_one` now checks `complete` but ignores `StreamRecord::truncated`, so output past the 16 MiB cap is dropped before the version regex or `dpkg` parsing sees it.
**Fix:** If either stream is `truncated`, return an `Observation` with `error: Some("output truncated")` so the row reads `not proven`.

### IN-02: `not_installed` tools are never verified absent

**File:** `crates/mt-toolchain/src/check.rs:361-364`
**Issue:** Still applies. Hayroll and Kani pass without a measurement, so installing either would keep the check green.
**Fix:** Optionally probe the pinned install paths and report `MISMATCH` if they exist.

### IN-03: LLVM-family package detection gaps and a misleading comment

**File:** `crates/mt-toolchain/src/version.rs:193-221` (unchanged since the diff base; outside this review's file list)
**Issue:** Still applies. `libllvm16t64` is recognised with the wrong major (64). `lld`, `lldb`, `libc++`, `libomp` and `libpolly` packages are never inspected.
**Fix:** Correct the doc and extend `FAMILY_PREFIXES`. Consider flagging installed unversioned family packages.

### IN-04: A missing `cwd` or interpreter is reported as "program not found"

**File:** `crates/mt-toolchain/src/runner.rs:658-672`, `crates/mt-toolchain/src/check.rs:491`
**Issue:** Still applies. A nonexistent working directory and a missing shebang interpreter both surface as `ErrorKind::NotFound`, so `check` prints `MISSING` for every tool when the real fault is the cwd.
**Fix:** Check `cwd.is_dir()` before spawning and return a distinct `RunnerError`.

### IN-05: Run record is lossy and does not identify the executable that ran

**File:** `crates/mt-toolchain/src/runner.rs:720-733`
**Issue:** Still applies. argv and cwd go through `to_string_lossy`, `tool.path` is `argv[0]` as given, and the version is whatever the caller supplied.
**Fix:** Record non-UTF-8 arguments explicitly and record the resolved executable path or its sha256.

### IN-06: Pins validation does not cover every value the Dockerfile interpolates

**File:** `crates/mt-toolchain/src/pins.rs:342-547` (unchanged; outside this review's file list and not re-read)
**Issue:** Still applies. Crate names and versions, archive versions, git tags, `image.base` and `rust.host` get no format check. The new `klee_uclibc_locale.version` is one more value interpolated into a file name; the Dockerfile asserts the URL tail equals it (`Dockerfile:440`), which bounds the risk but is not a pins check.
**Fix:** Validate names (`[a-z0-9_-]+`), versions, tags and host against strict patterns, and add the static bitcode-LLVM cross-check to `llvm_issues`.

### IN-07: Dead build arguments and no in-build assertion of the pinned rustc commit

**File:** `container/Dockerfile:25`, `:46`, `:48`, `:52`, `:59`
**Issue:** Still applies. `ARM_GNU_VERSION`, `RUSTUP_VERSION`, `RUST_BITCODE_COMMIT`, `RUST_TOOL_COMMIT` and `TINYCBOR_PARENT` are global ARGs used in no stage.
**Fix:** Add `rustc -vV | grep -q "commit-hash: ${RUST_TOOL_COMMIT}"` (and the bitcode equivalent) in the offline install RUN, or stop rendering the unused keys.

### IN-08: Final image runs as root and ships unpinned runtime packages

**File:** `container/Dockerfile:474-492`, `:96-115`
**Issue:** Still applies. The final stage has no `USER`. `ca-certificates`, `libsqlite3-0`, `zlib1g`, `libc6-dev`, `make` and `xz-utils` have no `[apt]` pin and no check row.
**Fix:** Add `USER 65534:65534` or document root as intentional. Add the remaining packages to `[apt]` and the `pin-discovery` list.

### IN-09: `pin-discovery` never compares the pinned Arm checksum, and the README says it does

**File:** `.github/workflows/pin-discovery.yml:108-113`; `container/README.md:149-155`
**Issue:** Still applies, and the new locale check makes it visible. The locale block fails when the pinned value differs from the computed one (`:121`). The Arm block fails only when the official `.sha256asc` differs from the computed digest; `ARM_GNU_SHA256` is printed but never compared. The README now says the workflow "fails when a pinned checksum differs from the computed one", which is false for Arm.
**Fix:** Also fail when `$ARM_GNU_SHA256 != $computed`.

### IN-10: GHCR owner is hard-coded and the annotation helper is copied four times

**File:** `.github/workflows/container.yml:134-156`, `:676-698`, `:793-813`, `:864-886`; owner literal at `:937`, `:967`, `:1006`
**Issue:** Still applies. `annotate()` is pasted into four jobs and `ghcr.io/sat-wik/...` is a literal in three places (and in the README and `container_static.rs`).
**Fix:** Keep one helper under `.github/scripts/` and `source` it after checkout. Derive the image name from `github.repository_owner`, lower-cased in a step.

### IN-11: The smoke step falls back to root, and the manifest comparison proves little about installed content

**File:** `.github/workflows/container.yml:566-578`; `crates/mt-toolchain/src/manifest.rs:121-172`; `container/README.md:225-233`
**Issue:** Still applies. A build failure that only happens unprivileged is retried as root and passes with a warning. The manifest holds pins verbatim plus row status words, so two builds could differ in installed content and compare equal. The locale tarball is a trust-on-first-use checksum from one host, it is not a line of the `pin-binding` record, and `klee-uclibc.bca` is not hashed into the manifest.
**Fix:** Restrict the root fallback to a recognised permission-denied message. Add content digests (sorted `dpkg-query` hash, sha256 of each installed tool binary, `klee-uclibc.bca`) to the manifest.

### IN-12: The workflow tests are string-presence checks, and the gate's comparison logic has no executable test

**File:** `crates/mt-toolchain/tests/container_static.rs:2343-2543`, `:2585-2702`; `.github/workflows/container.yml:313-438`
**Issue:** Still applies, and the new tests follow the same pattern. The new publish-guard assertions (`:2477-2492`) count substrings, so a commented-out guard line or `continue-on-error: true` still passes. Nothing runs the gate's `grep -qxF` / prefix / `check_archives` logic. The archive-pair `awk` is duplicated in `Dockerfile:257` and `container.yml:370` with no test that they agree.
**Fix:** Parse the YAML and assert on step structure (name, `if`, no `continue-on-error`). Move the gate logic to a script under `.github/scripts/` and test it with a matching record, a changed line and a removed line.

### IN-13: Manifest URL hardening is partial and the Dockerfile curl calls lack the usual options

**File:** `container/Dockerfile:185`, `:210`, `:229-236`, `:372`, `:443`
**Issue:** Still applies. The new locale download at `:443` is also `curl -fsSL` with no `--globoff`, `--proto '=https'` or `--proto-redir '=https'`. It is hash-checked in the same RUN, so this is hardening only.
**Fix:** Add `--globoff --proto '=https' --proto-redir '=https'` to the Dockerfile curl calls. Accept only `[A-Za-z0-9._/-]` after a manifest URL prefix.

### IN-14: Dependency crates and tinycbor are bound indirectly

**File:** `container/Dockerfile:330-347`, `:399-400`; `container/README.md:139-144`
**Issue:** Still applies. The README does not mention dependency crates. The post-install `grep -q "${TINYCBOR_COMMIT}"` on `CMakeLists.txt` proves only that the string appears.
**Fix:** Say in the README that dependency crates are bound through the packaged `Cargo.lock`. Tighten the check to `grep -Eq "GIT_TAG[[:space:]]+${TINYCBOR_COMMIT}"`.

### IN-15: `mt toolchain hash` discards curl's error message

**File:** `crates/mt-toolchain/src/fetch.rs:89-93`, `:101-110`
**Issue:** Still applies. `output_cap` is 0, so stderr is neither stored nor returned, and `FetchError::Failed` carries only the exit code. For the locale pin this matters: the host is unreachable from cloud sessions, and a failure prints only `exit code Some(6)`.
**Fix:** Use a 4 KiB cap and include the cleaned stderr text in `FetchError::Failed`.

### IN-16: Dead `ProcStat` fields, and a source-grep test stands in for a behavioural one

**File:** `crates/mt-toolchain/src/runner.rs:409-422`; `crates/mt-toolchain/tests/tool_01_runner.rs` (`tool_01_runner_signals_without_spawning_a_kill_program`)
**Issue:** Still applies. `pgrp` and `session` are parsed and compared in the unit test but never used by `kill_tree`. The test greps `runner.rs` for one `Command::new(` and no `/bin/kill`, which breaks on a rename and proves nothing about processes started.
**Fix:** Drop the two fields, or use them. Replace the grep with a behavioural check.

### IN-17: The tag lookup regex is broad, and a push that fails after succeeding can never be announced or repaired

**File:** `.github/workflows/container.yml:933-960`, `:962-996`
**Issue:** Still applies. `not found` in the lookup's stderr is treated as "absent". If `docker push` succeeds but the later `docker inspect` or digest validation fails, every rerun hits `exists=true`, skips the push and never announces `verified-image`. The `main` publish path has also not run since the guard changed (`REVIEW-FIX.md`, residual limits), so this path is unproven.
**Fix:** Match `MANIFEST_UNKNOWN` and `no such manifest` only. On the skip path, compare the existing digest with the loaded image id so a rerun can still announce a tag this workflow pushed.

### IN-18: After a timeout the wait loop has no upper bound

**File:** `crates/mt-toolchain/src/runner.rs:605-620`
**Issue:** Still applies. After `kill_tree`, `supervise` polls `child_exited` until the child is gone. A process in uninterruptible sleep ignores SIGKILL, so `run` never returns.
**Fix:** Bound the post-kill wait (for example by `drain_grace`) and return `Io` or an explicit "child did not die" error.

### IN-19: `NETWORKED_INSTALLS` and `NETWORKED_STEPS` are permanently empty allowlists

**File:** `crates/mt-toolchain/tests/container_static.rs:614-620`
**Issue:** After WR-08 both constants are `&[]` and a test asserts they stay empty. The lint code that reads them is dead scaffolding, and anyone who needs an exception must edit the constant and the assertion in two places.
**Fix:** Either remove the constants and the exception path from the lint, or keep them and drop the "must stay empty" assertion in favour of a review note.

### IN-20: A reader thread of an incomplete stream keeps hashing and storing after the snapshot

**File:** `crates/mt-toolchain/src/runner.rs:322-333`, `:378-388`
**Issue:** `into_captured` empties `stored` with `mem::take` and returns. The detached reader thread keeps reading, hashing and (because `stored.len()` is now 0) storing up to another `output_cap` bytes in memory, for as long as the escapee holds the pipe. An escapee that logs to an inherited stdout makes this a long-lived thread and a buffer of up to 16 MiB per stream in a process that outlives the run.
**Fix:** Add a `closed: AtomicBool` (or a flag inside `Partial`) set by `into_captured` and checked by `drain` after each read, so the thread drops the pipe and returns when the supervisor has stopped waiting.

### IN-21: `abort` can signal a reaped child's group if the OS reaps behind the runner

**File:** `crates/mt-toolchain/src/runner.rs:565-569`, `:575-587`
**Issue:** If the host runs `mt` with `SIGCHLD` set to `SIG_IGN` (the disposition is inherited across `exec`), the kernel reaps the child itself. `waitid` then fails with `ECHILD`, `supervise` returns `Err`, and `abort` calls `kill_tree` on a pid that is already gone and may be reused. This is not new (the earlier `try_wait` failed the same way) and needs an unusual parent, but the new code relies on "unreaped" without checking it.
**Fix:** On `ECHILD` from `child_exited`, return the error without calling `kill_tree`, or reset `SIGCHLD` to the default in the runner before the first spawn.

---

_Reviewed: 2026-10-12_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
