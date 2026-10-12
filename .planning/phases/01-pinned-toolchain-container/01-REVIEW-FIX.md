---
phase: 01-pinned-toolchain-container
fixed_at: 2026-10-12T03:40:00Z
review_path: .planning/phases/01-pinned-toolchain-container/01-REVIEW.md
iteration: 1
findings_in_scope: 5
fixed: 5
skipped: 0
status: all_fixed
---

# Phase 1: Code Review Fix Report

**Fixed at:** 2026-10-12T03:40:00Z
**Source review:** .planning/phases/01-pinned-toolchain-container/01-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 5 (WR-08 to WR-12; CR-01 and WR-01 to WR-07 were already resolved)
- Fixed: 5
- Skipped: 0

Branch `claude/gsd-discuss-phase-1-mzdw5v`, no worktree, no new branch. Before
every commit `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo test` and `cargo deny check` each exited 0 (run in the main checkout,
exit status checked per command). Container evidence comes from GitHub Actions,
not from the local checkout.

## Fixed Issues

### WR-10: The runner now compiles only on Linux

**Files modified:** `crates/mt-toolchain/src/runner.rs`, `crates/mt-toolchain/tests/tool_01_runner.rs`, `Cargo.toml` (comment), `.github/workflows/ci.yml`, `container/README.md`
**Commit:** d532ff6
**Applied fix:** The pidfd and `/proc` descendant sweep (and its unit tests) are `#[cfg(target_os = "linux")]`. On other unix, `kill_tree` signals the child's group through rustix's portable `kill_process_group`. A non-unix host hits a `compile_error!` with a plain message. The `setsid`-based runner tests are Linux-only. The README no longer says the runner behaves the same on a Mac: it says `mt` builds on macOS, is only run and tested on Linux, and that a Mac cannot find a descendant that left the group.
**Verification:** `rustup target add aarch64-apple-darwin`, then `cargo check -p mt-toolchain -p mt-cli --target aarch64-apple-darwin` failed before the change with the reported unresolved pidfd imports and passed after it. `cargo check --workspace --all-targets --locked --target aarch64-apple-darwin` and `cargo clippy ... --target aarch64-apple-darwin -- -D warnings` also pass (no linker needed for a check). The target was removed afterwards with `rustup target remove`.
**CI:** a new `ci.yml` step adds the target and runs `cargo check --workspace --all-targets --locked --target aarch64-apple-darwin` on ubuntu (no new `uses:`, so no new action to pin). It passed in `ci` runs 38198934560 and 38199978794.

### WR-12: The "unobserved" stream record states false facts and drops partial output

**Files modified:** `crates/mt-toolchain/src/runner.rs`, `crates/mt-toolchain/tests/tool_01_runner.rs`
**Commit:** 9f988ad
**Applied fix:** `StreamRecord` has a new `complete: bool`: true when the pipe reached end-of-file, false when a process the runner could not find still held it. Reader threads accumulate into shared state (`Arc<Mutex<..>>`: running digest, stored bytes, byte count), so a stream that never closes is reported with the bytes read so far and the digest of exactly those bytes, not the empty-input digest. `truncated` again means only "the cap was exceeded"; its doc and the `sha256`/`bytes_total` docs say what `complete: false` means. `RUN_RECORD_SCHEMA_VERSION` is 2. Tests: an orphaned `setsid` escapee gives `complete: false` with zero bytes and `truncated: false`; an escapee that prints `partial` keeps `partial\n` with its exact sha256; the JSON key set and schema version are asserted; empty streams still report `complete: true`.

### WR-11: A normal exit with a lingering descendant discards the result, leaves the process running and leaks a reader thread

**Files modified:** `crates/mt-toolchain/src/runner.rs`, `crates/mt-toolchain/src/check.rs`, `crates/mt-toolchain/src/fetch.rs`, `crates/mt-toolchain/tests/tool_01_runner.rs`
**Commit:** 1daa730
**Applied fix:** On Linux the runner detects exit with `rustix::process::waitid(WaitId::Pid, EXITED | NOHANG | NOWAIT)`, so an exited child stays an unreaped zombie and its pid and group id cannot be reused. After a normal exit the pipes get `RunnerConfig::drain_grace` (new field, default 5 s) to close; if they do not, `kill_tree` runs while the pid is still held, then the child is reaped. The record is returned with its real exit code and a new `exit.descendants_killed: true`. A stream that still does not close is returned `complete: false` with its partial bytes (WR-12). `RunnerError::OutputNotDrained` is removed. Non-Linux unix keeps `try_wait` (which reaps), does not kill after a normal exit, and returns the stream incomplete. `check.rs` records an incomplete stream as an `error` observation (not proven) and `fetch.rs` returns a new `FetchError::Incomplete` instead of a digest, so a partial stream is never parsed or hashed as whole. Tests: `sh -c 'sleep 30 & echo $!; exit 0'` returns in under 4 s with code 0, `descendants_killed`, complete streams, the sleeper gone (or a zombie), and the digest equal to the printed pid line; a `setsid`-orphaned sleeper after a normal exit returns the record with stdout incomplete.
**Status note:** logic change in process supervision; the behaviour is covered by the new tests (stable over three consecutive full gate runs) but deserves a human read.

### WR-08: One build step still downloads an unchecksummed tarball

**Files modified:** `container/pins.toml`, `container/Dockerfile`, `container/README.md`, `.github/workflows/pin-discovery.yml`, `crates/mt-toolchain/tests/container_static.rs`
**Commits:** 736256f (pin-discovery computes the checksum), defbe68 (pin, Dockerfile, tests), d909893 (README records the published image)
**Applied fix:**
- `[source.klee_uclibc_locale]` (kind `archive`, version `030818`, URL `https://www.uclibc.org/downloads/uClibc-locale-030818.tgz`, sha256 `c4362be318a38f18d98dccf462d22d95bab92f05548bb93f65298fe9afaebd57`). The URL is the one the klee-uclibc Makefile (`extra/locale/Makefile.in` at the pinned commit 955d502) uses. `www.uclibc.org` is blocked for cloud sessions, so the checksum was computed with `mt toolchain hash` on a GitHub runner (pin-discovery run 38198676698, annotation `locale-tarball-sha256`). `pin-discovery.yml` now recomputes it on every run and fails if it differs from the pin.
- The networked RUN that clones klee-uclibc now also downloads the tarball into `extra/locale/`, checks it with `sha256sum -c` against the pin, and asserts the Makefile names that exact file (`LOCALE_DATA_FILENAME := ...`). The klee-uclibc `make` runs in `RUN --network=none`; the Makefile's rule for a file with no prerequisites does not fetch again.
- `Remains networked:` label, the Dockerfile header exception and the `NETWORKED_STEPS` entry are removed. New test `tool_03_dockerfile_pins_the_klee_uclibc_locale_tarball` requires the pin, the checked download before an offline make, and that no exception comes back; it fails against the old Dockerfile (checked). Build-arg and Dockerfile-ARG tests pass with the three new `KLEE_UCLIBC_LOCALE_*` args.
**Manifest:** the manifest records pin sources, so its sha256 changed from `40e55cc6...` to `d70029186639d0030fc6cf22b284b3b849ce0680656fecfe06623f6ffdc44142`; the only pin change is the new source. Both builds were byte-identical.
**CI (commit defbe68):** `container` push run 38198934488: `mt`, `image-a`, `image-b`, `compare`, `publish` all `success`; `manifest-compare` reports manifest-a and manifest-b byte-identical, sha256 `d70029186639...44142`. `container` pull_request run 38198937060: success, `publish` skipped. `ci` runs 38198934560 and 38198937044, and `pin-discovery` run 38198934501: success.
**Published image from that run (recorded in `container/README.md` "Current image"):** `ghcr.io/sat-wik/migrationtool-toolchain@sha256:0f5a5d2594b0405931ad7e85d5c6d2f5e3f2d5099d769059870817ee50e152ee` (archive sha256 `c5ce0042f017664ba95a752177916fecfee513b8be091eb3dc80a8a13dfd5b93`, image id `sha256:5f1a410fa23e7874901c6f5ea70cd6fe609a46c0085b474e446bba23ac025e27`, commit defbe68309b4e1fc3a517ca49a11fed0087a2fd8).

### WR-09: Publishing with `packages: write` is open to a hard-coded feature branch

**Files modified:** `.github/workflows/container.yml`, `crates/mt-toolchain/tests/container_static.rs`, `container/README.md`
**Commit:** 3fa71a1
**Applied fix:** The publish job, image-a's save step and its archive upload are guarded by `github.event_name == 'push' && github.ref == 'refs/heads/main'` and nothing else. `PUBLISH_GUARD` and its count assertion (3) are updated, and a new assertion requires that the workflow names no other ref (exactly three `refs/heads/` occurrences, none under `claude/`, no `||` alternative). The README states that only a push to `main` publishes, that other runs build, gate and compare with `publish` skipped, and that the recorded image was published from the phase branch before the restriction.
**CI (commit 3fa71a1):** `container` push run 38199978785: `mt`, `image-a`, `image-b`, `compare` `success`, `publish` `skipped`; `manifest-compare` byte-identical, sha256 `d70029186639...44142` (equal to the previous run). `container` pull_request run 38199981938: success. `ci` runs 38199978794 and 38199981904: success.

## Skipped Issues

None.

## Verified image

Final verified image digest (published by run 38198934488 at defbe68, the last run that was allowed to publish from this branch; the 3fa71a1 run built and gated an identical-manifest image and skipped publish by design):

```
ghcr.io/sat-wik/migrationtool-toolchain@sha256:0f5a5d2594b0405931ad7e85d5c6d2f5e3f2d5099d769059870817ee50e152ee
```

## Residual limits

- The locale tarball checksum is trust-on-first-use: it is the hash that a runner computed from `www.uclibc.org` (pin-discovery run 38198676698), confirmed by a second independent download in the image build (`sha256sum -c` passed) and by every later pin-discovery run, but no second source vouches for the bytes. The tarball is not a line of the `pin-binding` record (that record covers the Rust toolchains and crates), and the manifest holds only its pin, not an installed-content digest (IN-11).
- The `klee-uclibc.bca` content is still not hashed into the manifest (the first bullet of WR-08's optional fix); a divergence in that input shows up only if a pin changes (IN-11).
- Linux only for post-exit tree kill: on macOS (type-checked in CI, never run there) a descendant that outlives the child is reported through `complete: false`, and an escapee that left the group is not found. On Linux a descendant whose parent chain was cut before the kill (double fork whose middle process already exited, then `setsid`) is still not found; its pipe is reported `complete: false` after `drain_grace`.
- A child that exits cleanly leaving a descendant that does NOT hold an output pipe is not killed (only a pipe that stays open triggers the kill).
- IN-18 (unbounded wait for a process in uninterruptible sleep after SIGKILL) is unchanged.
- `check.rs`'s handling of an incomplete stream has no dedicated test (it goes through the private `run_one`); the runner side is tested.
- The `main` publish path itself was not exercised by this branch's CI after WR-09; the guard is checked statically and by the skipped `publish` job on this branch.

---

_Fixed: 2026-10-12T03:40:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
