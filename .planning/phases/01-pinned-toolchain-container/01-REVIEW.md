---
phase: 01-pinned-toolchain-container
reviewed: 2026-10-09T00:00:00Z
depth: standard
files_reviewed: 33
files_reviewed_list:
  - .github/workflows/ci.yml
  - .github/workflows/container.yml
  - .github/workflows/pin-discovery.yml
  - .gitignore
  - Cargo.toml
  - clippy.toml
  - container/.dockerignore
  - container/Dockerfile
  - container/README.md
  - container/pins.toml
  - container/smoke/Cargo.toml
  - container/smoke/smoke.c
  - container/smoke/src/ffi.rs
  - container/smoke/src/lib.rs
  - crates/mt-cli/Cargo.toml
  - crates/mt-cli/src/main.rs
  - crates/mt-cli/tests/tool_01_check_end_to_end.rs
  - crates/mt-toolchain/Cargo.toml
  - crates/mt-toolchain/src/build_args.rs
  - crates/mt-toolchain/src/check.rs
  - crates/mt-toolchain/src/fetch.rs
  - crates/mt-toolchain/src/lib.rs
  - crates/mt-toolchain/src/manifest.rs
  - crates/mt-toolchain/src/pins.rs
  - crates/mt-toolchain/src/runner.rs
  - crates/mt-toolchain/src/version.rs
  - crates/mt-toolchain/tests/check_and_manifest.rs
  - crates/mt-toolchain/tests/container_static.rs
  - crates/mt-toolchain/tests/pins_and_versions.rs
  - crates/mt-toolchain/tests/repo_checks.rs
  - crates/mt-toolchain/tests/tool_01_runner.rs
  - deny.toml
  - rust-toolchain.toml
findings:
  critical: 1
  warning: 5
  info: 11
  total: 17
status: issues_found
---

# Phase 1: Code Review Report

**Reviewed:** 2026-10-09
**Depth:** standard
**Files Reviewed:** 33
**Status:** issues_found

## Summary

The runner itself is largely sound. Environment clearing, concurrent pipe draining, the exact output-cap boundary, hash-over-all-bytes, and the rule that a timeout or unobserved item is never `OK` all hold when I traced them. The check logic degrades fail-closed in nearly every path I tried. Actions are SHA-pinned, `persist-credentials: false` is set, permissions are least-privilege, and no `${{ }}` expression reaches a shell with untrusted data.

The defects concentrate where the project's own claims go further than the code:

1. The Dockerfile's "checked against its pinned sha256" controls verify a throw-away download, not the artefact that is installed (CR-01).
2. A timeout kills only the direct child (WR-01).
3. `mt toolchain hash`, the pin-bump checksum source, still lets curl read user config and expand URL globs (WR-02).
4. The single-LLVM-major probe inspects only the first library it sees (WR-03).
5. Any branch push publishes to GHCR (WR-04).
6. The test that enforces "process launching only in runner.rs" misses the idiom `runner.rs` itself uses (WR-05).

No structural (fallow) findings were supplied for this review.

## Critical Issues

### CR-01: Pinned checksums for the Rust channel manifests and crates are verified on a separate download that the installer never uses

**File:** `container/Dockerfile:163-180` (rustup stage) and `container/Dockerfile:215-231` (cargo-tools stage)
**Issue:** In both places the build downloads a file with `curl`, checks it with `sha256sum -c`, and then lets another tool fetch its own copy.
- Rustup stage: `channel-rust-<ver>.toml` is fetched to `/tmp` and hashed, then `rustup toolchain install` downloads the manifest itself from `static.rust-lang.org` and installs whatever it describes. The verified `/tmp` copy is deleted unused.
- Cargo-tools stage: `/tmp/<name>-<ver>.crate` is fetched and hashed, then `cargo install --locked <name> --version =<ver>` fetches the crate again from the registry. The verified file is never installed.

The comments say rustup-init and both channel manifests are "trusted on the strength of a pinned sha256" and that each crate is "checked against its pinned sha256 before an exact-version locked install". The pins do not bind what is installed. The check only proves the server answered correctly once. A server, CDN or registry that answered differently on the second request would pass the pin check and still deliver different bytes.

The post-build `mt toolchain check` does compare the `rustc` commit hash. It does not cover clippy, rustfmt, rust-std for either target, or the contents of the two installed crates. The test `run_issues` accepts these RUN steps because it only looks for `curl` plus `sha256sum -c` anywhere in the instruction. This is the supply-chain control the phase exists to provide, so a check that is not tied to the installed bytes is a defect, not a nicety.

**Fix:**
- Rust: after `rustup toolchain install`, compare the manifest rustup stored with the pin. Confirm byte-equality with a one-off CI run before relying on it.
```sh
for pair in "${RUST_TOOL_VERSION}:${RUST_TOOL_CHANNEL_SHA256}" "${RUST_BITCODE_VERSION}:${RUST_BITCODE_CHANNEL_SHA256}"; do
  v="${pair%%:*}"; sha="${pair#*:}"
  echo "${sha}  ${RUSTUP_HOME}/toolchains/${v}-${RUST_HOST}/lib/rustlib/multirust-channel-manifest.toml" | sha256sum -c -
done
```
  Remove the separate `/tmp/channel-rust-*.toml` download, or keep it only as an early fail.
- Crates: install the file that was hashed. Unpack the verified `.crate` and run `cargo install --locked --path <unpacked dir> ...`. Alternatively, keep the registry install and assert the pinned sha against the registry-cached `${CARGO_HOME}/registry/cache/*/<name>-<ver>.crate` after the install.
- Extend `run_issues` in `container_static.rs` so a pinned download must be consumed. For example, forbid `cargo install <name>` or `rustup toolchain install` in a RUN that has no post-install hash of the installed artefact.

## Warnings

### WR-01: A timeout kills only the direct child, and the timeout is then reported as a drain error

**File:** `crates/mt-toolchain/src/runner.rs:342-354`
**Issue:**
- `child.kill()` signals only the leader. Any descendant (a `cargo` that spawned `rustc`, a shell wrapper, a KLEE worker) keeps running after the "timeout", and it keeps the stdout/stderr pipes open.
- `collect()` then waits `DRAIN_GRACE` for each stream and `run` returns `Err(OutputNotDrained)`. The caller gets a supervision error, not a `RunRecord` with `timed_out: true`. The reader threads and the orphan process leak.
- Every later phase uses this runner for tools that spawn children, so the timeout guarantee ("killed and recorded as `timed_out`") does not hold for them.
- The `?` on `child.kill()` and `child.try_wait()` also returns early without reaping or killing the child if either call errors.

**Fix:** Start the child in its own process group with `CommandExt::process_group(0)`, which is safe and stable. Kill the whole group on timeout. `forbid(unsafe_code)` blocks `libc::kill`, so either invoke `/bin/kill -KILL -- -<pgid>` through the same `Command` machinery inside `runner.rs`, or relax `forbid` to `deny` for one audited module. After the kill, if a stream still does not drain, return the record with `timed_out: true` and the truncated capture instead of discarding it. On a `kill`/`try_wait` error, make a best-effort `child.kill(); child.wait()` before returning. Add a test with `sh -c 'sleep 30 & wait'` and a short timeout. No existing test covers the `OutputNotDrained` path.

### WR-02: `mt toolchain hash` lets curl read user config and expand URL globs

**File:** `crates/mt-toolchain/src/fetch.rs:61-81`
**Issue:**
- The runner clears the environment, but curl falls back to the passwd entry's home directory when `HOME` is unset. It can therefore still read `~/.curlrc`, where `insecure`, `proxy`, `cacert` and similar options silently weaken the TLS guarantee of the one tool used to recompute pin checksums.
- Without `--globoff`, a URL containing `[1-3]` or `{a,b}` is expanded and the concatenated bodies are hashed, so the digest is for something other than the URL.
- Output cap 0 also discards stderr, so a failure reports only the exit code, not curl's `--show-error` text.

**Fix:** Pass `-q` (or `--disable`) as the first curl argument. Add `--globoff` and `--max-redirs 5`.
```rust
argv: ["curl", "-q", "--globoff", "--fail", "--silent", "--show-error",
       "--location", "--max-redirs", "5",
       "--proto", "=https,file", "--proto-redir", "=https", url]
```
Consider a small stderr cap (for example 4 KiB) so `FetchError::Failed` can carry curl's message.

### WR-03: The single-LLVM-major probe inspects only the first library in `ldd` output

**File:** `crates/mt-toolchain/src/check.rs:730-745` (`regex_major`) and `container/pins.toml:140`
**Issue:** `regex_major` takes the first regex match in stdout, then stderr (`extract_either` goes through `Regex::captures`). The c2rust probe runs `ldd` on `c2rust-transpile`, whose output can list several LLVM/clang libraries. If the first match is `libclang-cpp.so.16` and a later line is `libLLVM-17.so.1`, the row passes as major 16. The Phase 1 requirement (TOOL-02) is exactly that all C-side tools use one major, and the `llvm_packages` row catches this only when a second set of packages is installed, not when a binary links a stray library. The unit tests use a single-library probe, so the gap is untested.

**Fix:** Collect every match of group `major` across stdout and stderr and require all to equal the pinned major. Report `Mismatch` naming the differing majors, and `Mismatch` if there are none. Add a fixture with two different majors in one `ldd` output.

### WR-04: Any branch push (and the weekly schedule) publishes to GHCR; the cancel group covers the publish

**File:** `.github/workflows/container.yml:16-47`, `:372-414`
**Issue:**
- The publish steps are gated only on `github.event_name != 'pull_request'`. `on: push` has no branch filter, so a push to any branch by anyone with write access builds and pushes `ghcr.io/sat-wik/migrationtool-toolchain:sha-<commit>` with a job holding `packages: write`.
- `workflow_dispatch` from any branch and the Monday cron do the same. The cron re-pushes the same `sha-<HEAD>` tag from a different, non-reproducible build (the README says two builds do not share a digest), so a tag silently moves.
- `concurrency: cancel-in-progress: true` on `container-${{ github.ref }}` lets a scheduled run cancel a push run that is between `docker push` and the compare job. That leaves a pushed but never "verified" image.

**Fix:** Move publishing to a separate job that `needs: [image-a, image-b, compare]` and holds `packages: write`. Gate it on `github.ref == 'refs/heads/main'` and `github.event_name != 'pull_request'`. This also means the image is pushed only after the manifest comparison, which is what D-24 describes. Use `cancel-in-progress: ${{ github.event_name == 'pull_request' }}`, or give the schedule its own concurrency group. Do not re-push a `sha-<commit>` tag that already exists.

### WR-05: The "process launching only in runner.rs" guard misses the idiom `runner.rs` itself uses

**File:** `crates/mt-toolchain/tests/repo_checks.rs:109-126`
**Issue:** The needle is the string `process::Command`. A brace import such as `use std::process::{Command, Stdio};` (line 24 of `runner.rs`) does not contain that substring. Neither does `use std::process as p;` or a glob import, so a violating file passes this test. The remaining defence is the `clippy.toml` `disallowed-types` entry. That is a lint outside `cargo test`, and a module-level `#![allow(clippy::disallowed_types)]` (as `runner.rs` has) switches it off. The test's stated purpose, giving TOOL-01 a failing test, is therefore not met.

**Fix:** Scan for the identifier and the module, not the path:
```rust
let command = regex::Regex::new(r"\bprocess\b[^;]*\bCommand\b|\bCommand::new\b").unwrap();
```
Alternatively, fail on any `std::process` mention outside `runner.rs` other than `ExitCode`, and fail on any `allow(clippy::disallowed_types)` outside `runner.rs`. Add a planted-file test like `tool_01_python_walk_flags_a_planted_file`.

## Info

### IN-01: Truncated captures are treated as complete observations

**File:** `crates/mt-toolchain/src/check.rs:466-489`
**Issue:** `run_one` ignores `StreamRecord::truncated`. Output beyond the 16 MiB cap is dropped before the version regex or `dpkg` parsing sees it. This is not reachable for version commands today. It is the same "partially observed but reported as observed" class as a timeout.
**Fix:** If either stream is `truncated`, produce an `Observation` with `error: Some("output truncated")` so the row reads `not proven`.

### IN-02: `not_installed` tools are never verified absent

**File:** `crates/mt-toolchain/src/check.rs:361-364`, `:657-667`
**Issue:** Hayroll (licence unconfirmed, R3) and Kani pass as `not_installed` without any measurement. If either is installed into the image, the check stays green. This is by design (D-10), but nothing enforces the decision it records.
**Fix:** Optionally probe that the pinned install paths do not exist, and report `MISMATCH` if they do.

### IN-03: LLVM-family package detection gaps and a misleading comment

**File:** `crates/mt-toolchain/src/version.rs:184-212`
**Issue:**
- The doc says names like `libllvm16t64` are "not recognised". They are recognised with the wrong major: the last two-digit run is `64`, so the row fails as "LLVM 64". That fails closed but with a confusing message.
- Packages outside the four prefixes (`lld-N`, `lldb-N`, `libc++-N-dev`, `libomp-N-dev`) are never inspected, so a second major arriving through them is invisible.
- Unversioned family packages (`clang`, `libclang-dev`) are skipped.

**Fix:** Correct the doc, and extend the prefix list with `lld`, `lldb`, `libc++`, `libomp`, `libpolly`. Consider flagging unversioned installed family packages, which the Dockerfile comment says "must never be installed".

### IN-04: A missing `cwd` or missing interpreter is reported as "program not found"

**File:** `crates/mt-toolchain/src/runner.rs:205-211`, `crates/mt-toolchain/src/check.rs:477`
**Issue:** `current_dir(nonexistent)` and a script whose shebang interpreter is absent both surface as `ErrorKind::NotFound`. `is_not_found()` is true for both, so `check` prints `MISSING` for every tool when the real fault is the working directory. The outcome is still a failure, but the diagnostic is wrong.
**Fix:** Check `cwd.is_dir()` before spawning and return a distinct `RunnerError`.

### IN-05: Run record is lossy and does not identify the executable that ran

**File:** `crates/mt-toolchain/src/runner.rs:356-369`
**Issue:** The record converts argv and cwd with `to_string_lossy`, so two non-UTF-8 invocations can serialize identically. `tool.path` is `argv[0]` as given (for example `curl`), not the resolved path, and the version is whatever the caller supplied. Phase 11 plans to use this as evidence.
**Fix:** Record non-UTF-8 arguments explicitly, for example as an escaped form or hex with a flag. Record the resolved executable path, or its sha256, where practical.

### IN-06: Pins validation does not cover every value the Dockerfile interpolates

**File:** `crates/mt-toolchain/src/pins.rs:342-547`
**Issue:**
- `Crate.name`/`version`, `Archive.version`, Git `tag`, `image.base` and `rust.host` get no format check.
- The Dockerfile joins several of them with `:` and splits on `:` (`${C2RUST_NAME}:${C2RUST_VERSION}:${C2RUST_SHA256}`) and interpolates names into URL paths. A `:` or `/` in a pin would break parsing or traverse the URL path.
- `rust.bitcode.llvm`'s major is not checked against `[llvm].major` or `max_major` by `validate`. That happens only in the live check.

**Fix:** Validate names (`[a-z0-9_-]+`), versions (dotted numeric with an optional suffix), tags and host against strict patterns. Add the static bitcode-LLVM cross-check to `llvm_issues`.

### IN-07: Dead build arguments and no in-build assertion of the pinned rustc commit

**File:** `container/Dockerfile:15-53`
**Issue:** `RUST_TOOL_COMMIT`, `RUST_BITCODE_COMMIT`, `ARM_GNU_VERSION`, `RUSTUP_VERSION` and `TINYCBOR_PARENT` are declared as global ARGs and never used in any stage. The test only requires ARG names to match the rendered keys, so unused pins are accepted. The `rustc` commit is verified only after the build by `mt toolchain check`.
**Fix:** Add `rustc -vV | grep -q "commit-hash: ${RUST_TOOL_COMMIT}"` (and the bitcode equivalent) in the rustup stage, or drop the unused keys from the rendered args.

### IN-08: Final image runs as root and ships unpinned runtime packages

**File:** `container/Dockerfile:287-309`, `:89-108`
**Issue:**
- The final stage has no `USER`. CI always passes `--user 65534`, but a consumer who forgets runs every analyser as root.
- `ca-certificates`, `libsqlite3-0`, `zlib1g`, `libc6-dev`, `make` and `xz-utils` are installed into the runtime image but have no `[apt]` pin and no check row. Their determinism rests on the snapshot timestamp alone.

**Fix:** Add `USER 65534:65534` (the smoke step already shows the rustup home is world-readable), or document root as intentional. Add the remaining runtime packages to `[apt]` and to the `pin-discovery` package list.

### IN-09: `pin-discovery` never compares the pinned Arm checksum

**File:** `.github/workflows/pin-discovery.yml:109-113`
**Issue:** The workflow fails only if the official `.sha256asc` differs from the freshly computed digest. `ARM_GNU_SHA256` from pins is printed but not compared, so a stale or wrong pin passes discovery and fails only later in the container build.
**Fix:** Also fail when `$ARM_GNU_SHA256 != $computed`.

### IN-10: GHCR owner is hard-coded and the annotation helper is copied three times

**File:** `.github/workflows/container.yml:385`, `:617`, `:125-148`, `:455-478`, `:573-593`
**Issue:**
- `ghcr.io/sat-wik/...` is repeated as a literal in the workflow and README. A fork or rename cannot push, and GHCR needs the lower-case owner.
- `annotate()` is pasted into `image-a`, `image-b` and `compare`, with a variant `escape()` in `pin-discovery.yml`. A future fix applied to one copy will drift from the others.

**Fix:** Use `ghcr.io/${{ github.repository_owner }}/migrationtool-toolchain`, lower-cased in a step. Keep one helper under `.github/scripts/` and `source` it.

### IN-11: The smoke step falls back to root on any unprivileged failure, and the manifest comparison proves little

**File:** `.github/workflows/container.yml:346-358`, `:546-613`, `crates/mt-toolchain/src/manifest.rs:121-172`
**Issue:**
- A genuine build failure that only happens unprivileged is retried as root and can pass, with only a warning annotation.
- The manifest holds the pins verbatim plus per-row status words, so two builds of the same pins produce identical manifests whenever both pass the check. The `compare` job therefore adds little beyond "both builds passed". The README is honest about this ("names, versions, source references and source hashes"). The "independent builds" framing still suggests more reproducibility evidence than exists.

**Fix:** Restrict the root fallback to a recognised permission-denied message and fail otherwise. Consider adding a content digest to the manifest (for example the sorted `dpkg-query` output hash and the sha256 of each installed tool binary) so that a divergence in unpinned content shows up in the diff.

---

_Reviewed: 2026-10-09_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
