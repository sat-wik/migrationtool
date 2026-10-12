---
phase: 01-pinned-toolchain-container
reviewed: 2026-10-12T00:00:00Z
update_diff_base: 214730673de4ccc65252bdde9f24b7c280dd19aa
depth: standard
files_reviewed: 13
files_reviewed_list:
  - .github/workflows/container.yml
  - Cargo.toml
  - container/Dockerfile
  - container/README.md
  - crates/mt-toolchain/Cargo.toml
  - crates/mt-toolchain/src/check.rs
  - crates/mt-toolchain/src/fetch.rs
  - crates/mt-toolchain/src/runner.rs
  - crates/mt-toolchain/src/version.rs
  - crates/mt-toolchain/tests/check_and_manifest.rs
  - crates/mt-toolchain/tests/container_static.rs
  - crates/mt-toolchain/tests/repo_checks.rs
  - crates/mt-toolchain/tests/tool_01_runner.rs
findings:
  critical: 0
  warning: 5
  info: 18
  total: 23
status: issues_found
---

# Phase 1: Code Review Report

**Reviewed:** 2026-10-12
**Depth:** standard
**Files Reviewed:** 13 (the files changed since `214730673de4ccc65252bdde9f24b7c280dd19aa` that are source, workflow, Dockerfile, README or tests)
**Status:** issues_found

## Summary

This pass re-reviews the fixes for CR-01 and WR-01..WR-07 and the residual hardening (pidfd and process-group kill in `runner.rs`, `RUN --network=none` in the Dockerfile, per-archive manifest binding, and the separate `publish` job). I traced the runner, the Dockerfile RUNs and the workflow gate by hand against the source. I also checked the vendored rustix 1.1.5 source for the API gating. I did not rebuild the image or run CI.

All eight prior blocking and warning findings are resolved or substantively resolved, and I found no new critical issue. The runner's escape handling is sound: the group is stopped first, descendants are walked by ppid, each is pinned with a pidfd plus a start-time check, and nothing signals a bare pid. The Dockerfile now installs only from files whose hashes were checked in a previous networked RUN, and the offline RUN re-proves record, mirror and manifests before the first install. The tests use no network. The publish job holds the only `packages: write`, loads an archive whose hash and image id it checks against the producing job, and refuses to move an existing tag.

What remains is narrower:

1. One build step still downloads an unchecksummed tarball, and the Dockerfile's own header says that never happens (WR-08).
2. Publishing is open to a hard-coded feature branch with `packages: write` (WR-09).
3. The runner now compiles only on Linux, while the README tells the founder to run `mt` natively on a Mac (WR-10).
4. Two runner edge cases still lose or leak (WR-11, WR-12).

No structural (fallow) findings were supplied for this review.

## Prior findings

| ID | Verdict | Evidence |
|----|---------|----------|
| CR-01 | resolved | Mirror built from hash-checked manifests and archives: `container/Dockerfile:200-242`. Offline install proves record, mirror and manifest equality before the first install and checks the stored update-hash against the pins: `:252-308`. Crates are installed with `--offline --path` from the checked, unpacked `.crate`: `:360-403`. Gate: `.github/workflows/container.yml:252-438`. |
| WR-01 | resolved | `process_group(0)` at `runner.rs:529`. `kill_tree` (STOP group, pidfd walk with start-time check, KILL held pidfds, KILL group) at `:423-461`. `abort` on every early return at `:465-469`, `:539`, `:550`. A timeout with an undrained pipe now returns the record with the stream marked unobserved: `:315-324`. No external `kill` program (single `Command::new` at `:520`). Residuals are WR-11, WR-12 and IN-18. |
| WR-02 | resolved | `curl_argv` has `-q` first, `--globoff`, `--max-redirs 5`: `fetch.rs:46-63`, used at `:92`. Glob and argv tests at `check_and_manifest.rs:764-785`. Residual is IN-15 (curl's message is still discarded). |
| WR-03 | resolved | `regex_major` collects every match in both streams and fails on more than one distinct major: `check.rs:730-775`, `version.rs:113-120`. Mixed-major and repeated-major tests at `check_and_manifest.rs:152-187`. |
| WR-04 | partially | Publishing moved to a separate `publish` job that needs `image-a`, `image-b` and `compare` and holds the only `packages: write`: `container.yml:844-1019`. Archive hash and image id are checked before login: `:897-921`. An existing `sha-<commit>` tag is never moved: `:933-960`. Cancel group is per event type and only cancels pull_request runs: `:52-54`. Not fully resolved: the guard still admits a hard-coded feature branch (WR-09). |
| WR-05 | resolved | `process_launch_violations` handles grouped, aliased, glob and module imports and the lint mention: `repo_checks.rs:120-162`. Positive, negative and planted-file tests at `:190-250`. |
| WR-06 | resolved | Installs run in `RUN --network=none` (`Dockerfile:187`, `:252`, `:322`, `:386`, `:426`, `:445`). Per-command lint with parsed wrappers, tar `-C` binding and a stage-scoped evidence reset: `container_static.rs:296-788`. Residual: one build step stays networked (WR-08), and the lint is explicitly not a sandbox. |
| WR-07 | resolved | Offline RUN checks each archive line against its manifest's `xz_hash`/`xz_url` pair: `Dockerfile:252-292`. The gate reads the kept manifests from the image and checks the archive lines, the cargo ledger source and the update-hash: `container.yml:313-438`. Witness wording fixed at `:324` and `:426`. Residual is IN-12 (the gate logic has no executable test). |

## Warnings

### WR-08: One build step still downloads an unchecksummed tarball, contradicting the Dockerfile's own header, and nothing in the manifest or gate would notice a change

**File:** `container/Dockerfile:9-10`, `:431-440`; `crates/mt-toolchain/tests/container_static.rs:618-621`, `:1596-1636`
**Issue:**
- `cd /src/klee-uclibc && make` runs in a networked RUN. Its Makefile fetches `uClibc-locale-030818.tgz` from `www.uclibc.org` with no checksum, and the result is compiled into `libc.a` and then into `klee-uclibc.bca`, which KLEE ships in the image.
- The file's header (lines 9-10) says the Dockerfile has "no unpinned download". The README discloses the exception (`README.md:221-223`) but the Dockerfile header and the `verified-image` announcement do not.
- The manifest holds only pins and row status words (see IN-11). `image-a` and `image-b` could therefore differ in this input and still compare equal, and the "manifest-identical" claim does not cover it.
- The whole `make` runs with network access, so it can fetch anything else the klee-uclibc Makefile decides to. `NETWORKED_STEPS` is a one-entry allowlist that makes the static test pass as long as a comment repeats the reason string. It does not require a plan to remove the exception.

**Fix:**
- Add a `[source.klee_uclibc_locale]` archive pin (URL and sha256) to `pins.toml`. Download and check it in the networked RUN that clones klee-uclibc, and place it where the Makefile looks for it. Verify the Makefile skips the download when the file is already present.
- Then run `make` in `RUN --network=none` and empty `NETWORKED_STEPS`.
- Until then, amend the header comment at lines 9-10 to name the exception.
- Include `sha256sum` of `klee-uclibc.bca` in the manifest so a divergence in this input shows up in `manifest-diff`.

### WR-09: Publishing with `packages: write` is open to a hard-coded feature branch

**File:** `.github/workflows/container.yml:599`, `:628`, `:846`; `crates/mt-toolchain/tests/container_static.rs:2340`
**Issue:** The publish guard is `push` to `refs/heads/main` or to `refs/heads/claude/gsd-discuss-phase-1-mzdw5v`.
- Anyone with write access can push to that branch name (or delete and recreate it) and get an image published into the package that the README calls "the verified image". Branch protection on `main` does not cover it.
- The name stays valid after the phase is merged, so a retired working branch keeps publish rights.
- The branch name is repeated in three workflow places and in the `PUBLISH_GUARD` constant, which the test compares by exact string. Removing the branch means editing four places in lockstep.
- WR-04's fix said to gate on `main`. The branch exception was added to keep evidence images from the phase branch.

**Fix:** Gate on `refs/heads/main` only. If phase-branch images are still wanted, give `publish` a GitHub `environment:` with required reviewers or a deployment-branch rule, and keep the branch list in one repository variable. Update `PUBLISH_GUARD` and its count assertion (`container_static.rs:2407-2411`) together.

### WR-10: The runner now compiles only on Linux, but the README tells the founder to run `mt` natively on a Mac

**File:** `crates/mt-toolchain/src/runner.rs:34-35`, `:423-461`; `container/README.md:108-122`; `Cargo.toml:19-21`
**Issue:** `runner.rs` imports `rustix::process::{pidfd_open, pidfd_send_signal, ...}` unconditionally and scans `/proc`. In rustix 1.1.5 `pidfd` is declared `#[cfg(target_os = "linux")]` (`src/process/mod.rs:29-30`), so these items do not exist on macOS and `mt-toolchain` fails to compile there.
- The README's "Local build fallback" tells the reader to run `cargo run -q -p mt-cli -- toolchain build-args` and says this "runs natively on the Mac".
- CLAUDE.md says the tool runs on Linux x86-64 hosts, so a Linux-only runner can be the intended design. The README was not updated to match, and the fallback is the documented path when the registry image is unavailable.
- `mt toolchain hash` also depends on the runner.

**Fix:** Pick one of two options:
- Make the README match the constraint: render build args inside the pinned container or on a Linux host, and say `mt` is Linux-only.
- Or gate the pidfd and `/proc` code with `#[cfg(target_os = "linux")]` and provide a fallback `kill_tree` that signals the group only.

Either way, add a `compile_error!` or a clear note so the failure is intentional and not an unresolved-import error.

### WR-11: A normal exit with a lingering descendant discards the result, leaves the process running and leaks a reader thread

**File:** `crates/mt-toolchain/src/runner.rs:283-294`, `:547-556`
**Issue:** WR-01 fixed the timeout path. When the child exits on its own but a descendant keeps stdout or stderr open (for example `sh -c 'sleep 30 & exit 0'`, or a tool that daemonizes), `collect` waits 5 s per stream and `run` returns `Err(OutputNotDrained)`.
- The record, exit status and everything already captured are dropped.
- Nothing kills the descendant, so it keeps running for as long as it likes.
- The detached reader thread stays blocked on the pipe until that process exits.
- `kill_tree` is deliberately not called here, since the child was already reaped by `try_wait` and its pid could be reused.

**Fix:** Detect child exit without reaping (`rustix::process::waitid` with `WNOWAIT`, or poll `/proc/<pid>/stat` for `Z`), run `kill_tree` while the pid is still held, and then reap. If a stream still does not drain after that, return the record with the stream marked unobserved, as on the timeout path. Add a test with `sh -c 'sleep 30 & exit 0'` that checks the sleeper is gone.

### WR-12: The "unobserved" stream record states false facts about what was captured and drops partial output

**File:** `crates/mt-toolchain/src/runner.rs:132-133`, `:296-311`
**Issue:** `unobserved()` returns `sha256` of the empty input, `bytes_total: 0`, `bytes_stored: 0` and `truncated: true`. The record is meant to become Phase 11 evidence.
- A reader of the JSON sees an empty stream whose digest equals the empty-input digest, flagged `truncated`. `StreamRecord::truncated` is documented as "`bytes_total` exceeded the cap", which is false here.
- Output the reader thread had already read is discarded with the thread. A timed-out tool's partial output is exactly what a diagnosis needs.
- Consumers rely on `exit.timed_out` to read this as "not proven". The stream record itself carries no way to tell "empty" from "never observed".

**Fix:** Add an explicit `observed: bool` (or make `sha256` `null`) and bump `RUN_RECORD_SCHEMA_VERSION`. Have the reader thread accumulate into shared state (`Arc<Mutex<..>>`) so a timeout can snapshot the bytes and the running digest. Report those bytes as partial, and update the `truncated` doc.

## Info

### IN-01: Truncated captures are treated as complete observations

**File:** `crates/mt-toolchain/src/check.rs:466-489`
**Issue:** Still applies. `run_one` ignores `StreamRecord::truncated`, so output past the 16 MiB cap is dropped before the version regex or `dpkg` parsing sees it. A timeout is now also reported through `truncated`, but those runs are caught by `timed_out`.
**Fix:** If either stream is `truncated`, return an `Observation` with `error: Some("output truncated")` so the row reads `not proven`.

### IN-02: `not_installed` tools are never verified absent

**File:** `crates/mt-toolchain/src/check.rs:361-364`, `:657-667`
**Issue:** Still applies. Hayroll and Kani pass without a measurement, so installing either would keep the check green.
**Fix:** Optionally probe the pinned install paths and report `MISMATCH` if they exist.

### IN-03: LLVM-family package detection gaps and a misleading comment

**File:** `crates/mt-toolchain/src/version.rs:193-221`
**Issue:** Still applies. `libllvm16t64` is recognised with the wrong major (64), not "not recognised" as the doc says. `lld`, `lldb`, `libc++`, `libomp` and `libpolly` packages are never inspected. Unversioned family packages are skipped.
**Fix:** Correct the doc and extend `FAMILY_PREFIXES`. Consider flagging installed unversioned family packages.

### IN-04: A missing `cwd` or interpreter is reported as "program not found"

**File:** `crates/mt-toolchain/src/runner.rs:520-534`, `crates/mt-toolchain/src/check.rs:477`
**Issue:** Still applies. A nonexistent working directory and a missing shebang interpreter both surface as `ErrorKind::NotFound`, so `check` prints `MISSING` for every tool when the real fault is the cwd.
**Fix:** Check `cwd.is_dir()` before spawning and return a distinct `RunnerError`.

### IN-05: Run record is lossy and does not identify the executable that ran

**File:** `crates/mt-toolchain/src/runner.rs:558-579`
**Issue:** Still applies. argv and cwd go through `to_string_lossy`, `tool.path` is `argv[0]` as given, and the version is whatever the caller supplied.
**Fix:** Record non-UTF-8 arguments explicitly and record the resolved executable path or its sha256.

### IN-06: Pins validation does not cover every value the Dockerfile interpolates

**File:** `crates/mt-toolchain/src/pins.rs:342-547` (unchanged since the diff base; outside this review's file list and not re-read)
**Issue:** Still applies. Crate names and versions, archive versions, git tags, `image.base` and `rust.host` get no format check. The Dockerfile joins several with `:` and uses them as path components (`/tmp/crates/${name}-${version}`, `/tmp/rust-dist/dist/channel-rust-${version}.toml`).
**Fix:** Validate names (`[a-z0-9_-]+`), versions, tags and host against strict patterns, and add the static bitcode-LLVM cross-check to `llvm_issues`.

### IN-07: Dead build arguments and no in-build assertion of the pinned rustc commit

**File:** `container/Dockerfile:25`, `:46`, `:48`, `:52`, `:59`
**Issue:** Still applies. `ARM_GNU_VERSION`, `RUSTUP_VERSION`, `RUST_BITCODE_COMMIT`, `RUST_TOOL_COMMIT` and `TINYCBOR_PARENT` are global ARGs used in no stage. The `rustc` commit is verified only after the build by `mt toolchain check`.
**Fix:** Add `rustc -vV | grep -q "commit-hash: ${RUST_TOOL_COMMIT}"` (and the bitcode equivalent) in the offline install RUN, or stop rendering the unused keys.

### IN-08: Final image runs as root and ships unpinned runtime packages

**File:** `container/Dockerfile:474-492`, `:96-115`
**Issue:** Still applies. The final stage has no `USER`. `ca-certificates`, `libsqlite3-0`, `zlib1g`, `libc6-dev`, `make` and `xz-utils` have no `[apt]` pin and no check row.
**Fix:** Add `USER 65534:65534` or document root as intentional. Add the remaining packages to `[apt]` and the `pin-discovery` list.

### IN-09: `pin-discovery` never compares the pinned Arm checksum

**File:** `.github/workflows/pin-discovery.yml:110-112` (unchanged since the diff base; outside this review's file list)
**Issue:** Still applies. The workflow fails only when the official `.sha256asc` differs from the computed digest. `ARM_GNU_SHA256` is printed (`:123`) but not compared.
**Fix:** Also fail when `$ARM_GNU_SHA256 != $computed`.

### IN-10: GHCR owner is hard-coded and the annotation helper is copied four times

**File:** `.github/workflows/container.yml:937`, `:967`, `:1006`; helper copies at `:134-156`, `:676-698`, `:793-813`, `:864-886`
**Issue:** Still applies and is worse. `annotate()` was pasted into `image-a`, `image-b` and `compare`, and the new `publish` job added a fourth copy. `ghcr.io/sat-wik/...` is a literal in three more places (and in the README and the test at `container_static.rs:2513-2516`).
**Fix:** Keep one helper under `.github/scripts/` and `source` it after checkout, or upload it once in the `mt` job's bundle. Derive the image name from `github.repository_owner`, lower-cased in a step.

### IN-11: The smoke step falls back to root on any unprivileged failure, and the manifest comparison proves little

**File:** `.github/workflows/container.yml:566-578`; `crates/mt-toolchain/src/manifest.rs:121-172`
**Issue:** Still applies. A build failure that only happens unprivileged is retried as root and passes with a warning. The manifest holds pins verbatim plus row status words, so it does not cover installed content (WR-08 is one example).
**Fix:** Restrict the root fallback to a recognised permission-denied message. Add content digests (sorted `dpkg-query` hash, sha256 of each installed tool binary, `klee-uclibc.bca`) to the manifest.

### IN-12: The workflow tests are string-presence checks, and the gate's comparison logic has no executable test

**File:** `crates/mt-toolchain/tests/container_static.rs:2343-2543`, `:2585-2660`, `:2663-2702`; `.github/workflows/container.yml:313-438`
**Issue:** Still applies and grew with the new tests. They assert that substrings occur in the job text, so a commented-out step, `continue-on-error: true` or `if: false` still passes. The first-occurrence order checks are satisfied by comments. Nothing runs the gate's `grep -qxF` / prefix / `check_archives` logic. The archive-pair `awk` is duplicated byte for byte in `Dockerfile:257` and `container.yml:370`, with no test that they agree.
**Fix:** Parse the YAML and assert on step structure (name, `if`, index, no `continue-on-error`). Move the gate logic to a script under `.github/scripts/` that takes the record, the pins and the manifests as files, and test it with a matching record, a changed line and a removed line. Source the same `pairs` awk from both places.

### IN-13: Manifest URL hardening is partial and the mirror downloads lack the usual curl options

**File:** `container/Dockerfile:185`, `:210`, `:229-236`, `:372`
**Issue:** Still applies. The URL filter is a prefix match plus a `..` ban. It accepts spaces, `?`, `#`, `%`, `[`, `{`, and a target path that equals the manifest or its sidecar. The `curl -fsSL` calls have no `--globoff`, `--proto =https` or `--proto-redir =https`. Every download is still hash-checked and the manifests are pinned, so this is hardening only.
**Fix:** Add `--globoff --proto '=https' --proto-redir '=https'` to the Dockerfile curl calls. Accept only `[A-Za-z0-9._/-]` after the prefix and refuse a target that already exists.

### IN-14: Dependency crates and tinycbor are bound indirectly

**File:** `container/Dockerfile:330-347`, `:399-400`; `container/README.md:139-144`
**Issue:** Partly improved. The Dockerfile comment now says the packaged `Cargo.lock` fixes every dependency under `--locked`, and tinycbor is cloned and checked with `rev-parse HEAD` in the networked RUN and redirected by `insteadOf` in the offline one. The README still does not mention dependency crates. The post-install `grep -q "${TINYCBOR_COMMIT}"` on `CMakeLists.txt` proves only that the string appears in the file, and the local clone holds the full history, so a different commit named in that file would still check out.
**Fix:** Say in the README that dependency crates are bound through the packaged `Cargo.lock`. Tighten the check to `grep -Eq "GIT_TAG[[:space:]]+${TINYCBOR_COMMIT}"`, or clone with `--depth 1` at the pin so no other commit exists locally.

### IN-15: `mt toolchain hash` discards curl's error message

**File:** `crates/mt-toolchain/src/fetch.rs:85-89`, `:102-106`
**Issue:** `output_cap` is 0, so stderr is neither stored nor returned. `--show-error` has no visible effect and `FetchError::Failed` carries only the exit code. WR-02 listed a small stderr cap as optional and it was not added.
**Fix:** Use a 4 KiB cap and include the (cleaned) stderr text in `FetchError::Failed`. The stdout digest still covers every byte.

### IN-16: Dead `ProcStat` fields, and a source-grep test stands in for a behavioural one

**File:** `crates/mt-toolchain/src/runner.rs:328-338`; `crates/mt-toolchain/tests/tool_01_runner.rs:318-332`
**Issue:** `pgrp` and `session` are parsed and compared in the unit test but never used by `kill_tree`. `tool_01_runner_signals_without_spawning_a_kill_program` greps `runner.rs` for one `Command::new(` and no `/bin/kill`, so it breaks on a rename and does not prove no extra process is started.
**Fix:** Drop the two fields, or use them (for example to skip processes already in the child's group). Replace the grep with a behavioural check, such as counting children created during a timeout run.

### IN-17: The tag lookup regex is broad, and a push that fails after succeeding can never be announced or repaired

**File:** `.github/workflows/container.yml:933-960`, `:962-996`
**Issue:** `not found` in the lookup's stderr is treated as "absent", which also matches unrelated messages. If `docker push` succeeds but the later `docker inspect` or digest validation fails, the tag exists, so every rerun hits `exists=true`, skips the push and never announces `verified-image` for that commit.
**Fix:** Match `MANIFEST_UNKNOWN` and `no such manifest` only. On the skip path, read the existing digest and compare it with the loaded image id, so a rerun can still announce a tag this workflow pushed.

### IN-18: After a timeout the wait loop has no upper bound

**File:** `crates/mt-toolchain/src/runner.rs:473-488`
**Issue:** After `kill_tree`, `supervise` keeps polling `try_wait` until the child is reaped. A process stuck in uninterruptible sleep (a hung network or FUSE filesystem) ignores SIGKILL, so `run` never returns and the timeout guarantee ends there.
**Fix:** Bound the post-kill wait (for example by `DRAIN_GRACE`) and return `Io` or an explicit "child did not die" error after logging the pid.

---

_Reviewed: 2026-10-12_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
