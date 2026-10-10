---
phase: 01-pinned-toolchain-container
reviewed: 2026-10-09T00:00:00Z
updated: 2026-10-10T00:00:00Z
update_diff_base: b2b3fd6928d8aab0f0b913148bbdad303a643ace
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
  critical: 0
  warning: 7
  info: 14
  total: 21
status: issues_found
---

# Phase 1: Code Review Report

**Reviewed:** 2026-10-09 (full review); updated 2026-10-10 (incremental, plan 01-09)
**Depth:** standard
**Files Reviewed:** 33 (the update re-read 4 of them: `container/Dockerfile`, `crates/mt-toolchain/tests/container_static.rs`, `.github/workflows/container.yml`, `container/README.md`)
**Status:** issues_found

## Summary

The runner itself is largely sound. Environment clearing, concurrent pipe draining, the exact output-cap boundary, hash-over-all-bytes, and the rule that a timeout or unobserved item is never `OK` all hold when I traced them. The check logic degrades fail-closed in nearly every path I tried. Actions are SHA-pinned, `persist-credentials: false` is set, permissions are least-privilege, and no `${{ }}` expression reaches a shell with untrusted data.

The defects concentrate where the project's own claims go further than the code:

1. The Dockerfile's "checked against its pinned sha256" controls verify a throw-away download, not the artefact that is installed (CR-01). **Resolved by plan 01-09; see the update below.**
2. A timeout kills only the direct child (WR-01).
3. `mt toolchain hash`, the pin-bump checksum source, still lets curl read user config and expand URL globs (WR-02).
4. The single-LLVM-major probe inspects only the first library it sees (WR-03).
5. Any branch push publishes to GHCR (WR-04).
6. The test that enforces "process launching only in runner.rs" misses the idiom `runner.rs` itself uses (WR-05).

No structural (fallow) findings were supplied for this review.

### Update 2026-10-10: incremental review of plan 01-09 (a8702d8..a03afa5)

Scope: the four files changed since `b2b3fd6`. Question asked: does the pin now bind the installed bytes, and did the change introduce a defect.

**CR-01 is resolved.** Evidence I checked myself, not taken from the SUMMARY:

- **Mirror installs only verified bytes.** The rustup stage downloads each channel manifest, runs `sha256sum -c` against the pin, then downloads every archive that manifest lists for the wanted packages and checks it against the manifest's own `xz_hash` (`Dockerfile:191-218`). `rustup toolchain install` then runs with `RUSTUP_DIST_SERVER=file:///tmp/rust-dist` and `--no-self-update` (`:226-227`). A `file://` server has no network fallback, so a component that is not in the mirror fails the build rather than being fetched unverified. Every archive rustup does read is re-checked by rustup against the same verified manifest.
- **The pins are the real manifest hashes.** I downloaded both manifests and hashed them: `channel-rust-1.72.1.toml` is `77113b96...ea229` and `channel-rust-1.99.0.toml` is `ce6dddc8...b6a2`. Both equal `container/pins.toml:41` and `:51`.
- **The awk helper `sv` works under dash and mawk 1.3.4 on both real manifests.** It returned the expected `xz_url` and `xz_hash` for rustc, cargo and rust-std of the host, `clippy-preview` and `rustfmt-preview` through `[renames]`, and an empty string for an absent section (which the RUN turns into a build failure). Neither manifest has a `zst_url`, so rustup's preferred archive format is the one the mirror holds. No `local`, no bashisms, no gawk extensions.
- **No URL or path injection from the manifest that survives the pin.** A URL must start with `https://static.rust-lang.org/dist/` and must not contain `..` (`:209-213`). The manifest is itself sha256-pinned, so an attacker would need to control the pinned bytes. The remaining laxness is hardening only (IN-13).
- **No TOCTOU.** Each file is hashed by `sha256sum -c` and then read by rustup or `tar` in the same RUN of a single-tenant build container. The record line is taken from the same file after the check. The tool-toolchain and the bitcode-toolchain installs both go through the mirror.
- **The crates install from the verified file.** `tar -xzf` unpacks the file that was checked, `Cargo.toml` and `Cargo.lock` are asserted present, and `cargo install --locked --path` builds that directory (`:283-296`). Dependency checksums come from the packaged `Cargo.lock` under `--locked`.
- **The gate sits before the push and gates the pushed image.** `Pin binding evidence` (`container.yml:229`) runs before `docker/login-action` (`:485`) and `id: publish` (`:492`). It reads the record from `mt-toolchain:ci`, and the publish step tags and pushes that same local image, never a rebuild. Publish steps have no status function, so a failed gate skips them.

What this resolution does not mean: the gate and the static rules are weaker than their names suggest (WR-06, WR-07, IN-12). Those are new findings below. They are about regression protection, not about the current Dockerfile, which I traced and found correct.

## Critical Issues

### CR-01: Pinned checksums for the Rust channel manifests and crates are verified on a separate download that the installer never uses

**Status: RESOLVED (2026-10-10, plan 01-09, commits a8702d8 and 686b56b).** See the update in the Summary for the evidence I verified. Original finding kept below as recorded.

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

**Resolution note:** the first fix suggested above (comparing rustup's stored manifest) was not used and would not have worked: rustup re-serialises the manifest, so the stored copy never equals the pin (the SUMMARY and the image-a notice show `28e24aa2...` stored against pin `77113b96...`). The file:// mirror is the correct design. The crate fix is the `--path` variant suggested here.

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

### WR-06: The new static rules are whole-RUN and spelling-bound, so several ways back to an unbound install pass them (new, plan 01-09)

**File:** `crates/mt-toolchain/tests/container_static.rs:297-370` (`run_issues`); real-file loops at `:664` and `:735-749`
**Issue:** The test is the regression guard for CR-01. Reading the rules, these installs are accepted although they re-open the hole:
- **Mirror prefix is per RUN, not per install.** `servers.iter().any(|v| v.starts_with("file://"))` (`:356`) is satisfied by one prefixed install anywhere in the RUN. A second `rustup toolchain install X --no-self-update` with no `RUSTUP_DIST_SERVER=` prefix, in the same RUN, passes. Only `--no-self-update` is checked per segment (`:360-368`). The accepted-vs-refused fixtures only test the "no prefix anywhere" case, and the real-Dockerfile assertion at `:664` counts installs but never checks each has the prefix.
- **Other rustup fetch commands are not matched.** The rule keys on the literal `rustup toolchain install`. `rustup install`, `rustup toolchain add`, `rustup update`, `rustup target add`, `rustup component add`, and `rustup default <toolchain not yet installed>` all download from the network and are ignored. The Dockerfile itself uses `rustup default` (`:228`), which is safe only because the toolchain exists.
- **`rustup-init` is not constrained.** Nothing requires `--default-toolchain none` (`Dockerfile:174`). If it were dropped, rustup-init would install `stable` from `static.rust-lang.org` with no pinned checksum, and nothing in the check row would notice, since only the default toolchain's `rustc` is probed.
- **`cargo install` is matched by the literal `cargo install`.** `cargo  install` (two spaces), `cargo +1.99.0 install`, or `cargo binstall` is not matched. `--path` is satisfied by any directory: `sha256sum -c` of an unrelated file plus `tar -x` of an unrelated archive plus `--path /elsewhere` is accepted (`:324-342`). `--locked` and `--version =` are also RUN-wide checks.

The current Dockerfile passes all of this correctly. The finding is that the guard would not fail when it should.

**Fix:**
- Evaluate every rule per command segment. For each segment containing a rustup `install|add|update|default|target|component` verb, require a `file://` `RUSTUP_DIST_SERVER` in that same segment. Match whitespace with `\s+` and cover `cargo +<tc> install`.
- Require `--default-toolchain none` on any `rustup-init` invocation.
- Tie `--path` to a directory under the `-C` target of a `tar -x` of a file that appears in a `sha256sum -c` line in the same RUN.
- Structural alternative that removes the need for most of this: split the install into its own RUN with `RUN --network=none` (BuildKit's default frontend supports it) after the mirror RUN has populated `/tmp/rust-dist`. Then a network fetch cannot happen at install time regardless of how the command is spelled. Keep `/tmp/rust-dist` by writing the mirror into a stage and `COPY --from`, or by doing both in one stage with two RUNs and no cleanup between them.

### WR-07: The `Pin binding evidence` gate checks the Dockerfile's own bookkeeping, and its update-hash witness is tautological (new, plan 01-09)

**File:** `.github/workflows/container.yml:229-330` (record check `:285-311`; update-hash check `:294-300`), `container/Dockerfile:193-194`, `:285`
**Issue:**
- **The record is self-reported.** Each line is produced by `sha256sum "${file}" >> "${record}"` in the same RUN that downloaded and checked the file. The gate matches those lines to the pins. It never relates the record to what was installed. If a later edit pointed `cargo install --path` at a different directory, or populated the mirror differently, the record would still equal the pins and the gate would pass. This is a check that the Dockerfile wrote down the pin it was given, not that the installed bytes match it.
- **The update-hash "witness" adds no independent evidence.** The sidecar `channel-rust-<v>.toml.sha256` that rustup reads is written by the Dockerfile itself from the pin (`Dockerfile:193`). Rustup stores the first 20 hex characters of that sidecar as its update hash. So "update-hash is a prefix of the pin" is true whenever rustup read the sidecar the Dockerfile wrote, whatever else happened. The notice text calls it a witness, which is fair, but the check lines read `OK ... is a prefix of the pin` as though it were corroboration.
- **Archive lines are not gated.** The record also lists every component archive, but the gate matches only the two manifests and two crates. Archives are bound only through the build's own `sha256sum -c`, which is correct but is not seen by the gate.
- **`rc` is the exit code of the last command in `binding.sh` only** (`sha256sum`), so a failing `cat` of the record does not set it. The MISMATCH lines catch this, so the result is still fail-closed.

**Fix:** Add measurements that come from the installed result rather than from the build's notes.
- For crates, read `/opt/cargo-tools/.crates2.json` (or `.crates.toml`) in the same `docker run` and require the source of each pinned crate to be `path+file:///tmp/crates/<name>-<version>`, which ties the install to the unpacked directory.
- For the toolchains, assert in the Dockerfile that `${RUSTUP_HOME}/update-hashes/<toolchain>` equals the first 20 characters of the pin (cheap, fails the build), and use `RUN --network=none` for the install (WR-06) so that the only possible source is the mirror.
- In the gate, also require that every archive line in the record appears in the manifest it came from: grep the `xz_hash` of each recorded archive from a copy of the verified manifest. At minimum, say in the notice that the update-hash is derived from the Dockerfile-written sidecar.

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

**Update 2026-10-10:** plan 01-09 adds more uses of these values as path components (`/tmp/crates/${name}-${version}`, `/tmp/rust-dist/dist/channel-rust-${version}.toml`, and the gate's `want` lines). `RUST_HOST` now also forms manifest section names. The same validation closes these.

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

(Line numbers refer to the Dockerfile as reviewed on 2026-10-09; plan 01-09 shifted them. The final stage now starts at `:356` and the apt package list is at `:89-108`.)

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

**Update 2026-10-10:** the SUMMARY cites the unchanged manifest sha256 (`40e55cc6...7aed3`) as evidence that nothing changed. That is consistent with IN-11: the manifest does not contain installed content, so an identical hash would also be produced by a build that installed different bytes. It is not evidence for or against CR-01, and I did not rely on it.

(Line numbers in the workflow refer to the file as reviewed on 2026-10-09; the `Pin binding evidence` step shifted later lines.)

### IN-12: The gate test is a string-presence check, and the gate's comparison logic has no executable test (new, plan 01-09)

**File:** `crates/mt-toolchain/tests/container_static.rs:1366-1416` (`tool_03_workflow_gates_on_pin_binding_evidence`)
**Issue:** The test asserts that the job text contains `pin-binding.sha256`, `annotate notice pin-binding`, `annotate error pin-binding` followed by `exit 1` within 300 characters, `/tmp/crates/`, and that the first `pin-binding` occurs before `id: publish` and the login action. All of these are satisfied by comments, so a step that is commented out, set to `continue-on-error: true`, or given `if: false` still passes. The first-occurrence ordering check is also satisfied by the step's leading comment. Nothing exercises the `grep -qxF` / prefix logic: the SUMMARY records a local dry run with a fake docker, but that script is not in the repository, so a regression in `check_toolchain` or `check_crate` is not caught until a build runs.
**Fix:** Parse the workflow with a YAML crate (already a dev-dependency candidate) and assert on the step structure: the step named `Pin binding evidence` exists in `image-a`, has no `continue-on-error`, precedes the `docker/login-action` step and the `publish` step by index. Move the compare logic into a script under `.github/scripts/` that takes a record file and the pins as arguments, and add a test that runs it with a matching record, a record with one changed line, and a record with a line removed (the dry run the SUMMARY describes).

### IN-13: Manifest URL hardening is partial and the mirror downloads lack the usual curl options (new, plan 01-09)

**File:** `container/Dockerfile:191`, `:209-216`
**Issue:** The URL filter is a prefix match plus a `..` ban. It accepts spaces, `?`, `#`, `%`, `[`, `{` and a path that names the manifest or its sidecar (`.../dist/channel-rust-1.99.0.toml`, which would make `curl -o` overwrite the verified manifest inside the mirror). The two `curl` calls use `-fsSL` without `--globoff`, so `[`/`{` in a URL is globbed, and without `--proto =https --proto-redir =https`, so `-L` may follow a downgrade to http. Hash checks still apply to each download, and the manifest is pinned, so none of this is exploitable without already controlling the pinned bytes. It is inconsistent with the `..` hardening the plan added and with WR-02.
**Fix:** Add `--globoff --proto '=https' --proto-redir '=https'` to the two mirror `curl` calls. Accept only `[A-Za-z0-9._/-]` after the prefix, for example `case "${url#https://static.rust-lang.org/dist/}" in *[!A-Za-z0-9._/-]*) fail ;; esac`, and refuse a target path that already exists (`[ ! -e "${archive}" ]`).

### IN-14: "Never fetched a second time" is true of the top-level crate only; dependencies and tinycbor are bound indirectly (new, plan 01-09)

**File:** `container/Dockerfile:248-258`, `:297`; `container/README.md:139-144`
**Issue:** The comment, README and SUMMARY say the verified `.crate` is the one installed. That holds for `c2rust` and `cargo-mutants`, but the install still downloads every dependency crate from the registry. They are bound by the checksums in the packaged `Cargo.lock` under `--locked`, which cargo enforces, so this is sound, but those files are not in the build record and are not mentioned in the README. The tinycbor pin is bound by `grep -q "${TINYCBOR_COMMIT}" .../c2rust-ast-exporter-<ver>/src/CMakeLists.txt`. That proves the string appears in the file (a comment would do), not that the cloned tree is at that commit. `--version =X` together with `--path` is redundant, though harmless as an assertion that the unpacked directory is that version.
**Fix:** Say in the README and the Dockerfile comment that dependency crates are bound through the packaged `Cargo.lock`. Tighten the tinycbor check to the exact `GIT_TAG <commit>` line (`grep -Eq "GIT_TAG[[:space:]]+${TINYCBOR_COMMIT}"`), or after the build assert `git -C <build dir> rev-parse HEAD` equals the pin.

---

_Reviewed: 2026-10-09; updated 2026-10-10_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
