---
phase: 01-pinned-toolchain-container
verified: 2026-10-09T21:00:00Z
status: gaps_found
score: 3/4 must-haves verified
covered_files:
  - .github/workflows/container.yml
  - .planning/phases/01-pinned-toolchain-container/01-01-PLAN.md
  - .planning/phases/01-pinned-toolchain-container/01-01-SUMMARY.md
  - .planning/phases/01-pinned-toolchain-container/01-02-PLAN.md
  - .planning/phases/01-pinned-toolchain-container/01-02-SUMMARY.md
  - .planning/phases/01-pinned-toolchain-container/01-03-PLAN.md
  - .planning/phases/01-pinned-toolchain-container/01-03-SUMMARY.md
  - .planning/phases/01-pinned-toolchain-container/01-04-PLAN.md
  - .planning/phases/01-pinned-toolchain-container/01-04-SUMMARY.md
  - .planning/phases/01-pinned-toolchain-container/01-05-PLAN.md
  - .planning/phases/01-pinned-toolchain-container/01-05-SUMMARY.md
  - .planning/phases/01-pinned-toolchain-container/01-06-PLAN.md
  - .planning/phases/01-pinned-toolchain-container/01-06-SUMMARY.md
  - .planning/phases/01-pinned-toolchain-container/01-07-PLAN.md
  - .planning/phases/01-pinned-toolchain-container/01-07-SUMMARY.md
  - .planning/phases/01-pinned-toolchain-container/01-08-PLAN.md
  - .planning/phases/01-pinned-toolchain-container/01-08-SUMMARY.md
  - container/Dockerfile
  - container/README.md
  - container/pins.toml
  - crates/mt-toolchain/src/check.rs
  - crates/mt-toolchain/src/runner.rs
covered_digest: "v3:sha256:e9f1231506ea3e446c72dbdb99aa11bed962067e002afff4823f95d391240736"
behavior_unverified: 0
overrides_applied: 0
gaps:
  - truth: "Every base image and tool source is pinned by digest, commit or checksum (ROADMAP SC3 / TOOL-03), meaning the pin governs the bytes that end up in the image"
    status: partial
    reason: "CR-01 (01-REVIEW.md, still open) is confirmed in container/Dockerfile. The rustup stage downloads channel-rust-<ver>.toml to /tmp, checks it with sha256sum -c, deletes it, and then `rustup toolchain install` fetches its own copy of the manifest. The cargo-tools stage downloads <name>-<ver>.crate to /tmp, checks it, and then `cargo install --locked <name> --version =<ver>` fetches the crate again from the registry. So the pinned sha256 of both Rust channel manifests, c2rust and cargo-mutants never binds the installed bytes. The other sources (base digest, apt snapshot plus exact versions, rustup-init, Arm archive, KLEE and klee-uclibc commits checked against rev-parse HEAD, tinycbor commit grep) are consumed correctly. Integrity for the four affected sources currently rests on rustup/cargo's own hash checks plus the post-build rustc commit/LLVM comparison, not on the project's pins."
    artifacts:
      - path: "container/Dockerfile"
        issue: "Lines ~163-180 (rustup stage) and ~215-231 (cargo-tools stage): sha256 verified on a throw-away download that the installer does not use; the Dockerfile comments say the opposite"
      - path: "crates/mt-toolchain/tests/container_static.rs"
        issue: "run_issues accepts any RUN that has curl plus sha256sum -c, so this pattern passes the static check"
    missing:
      - "Rust: after `rustup toolchain install`, sha256sum the stored multirust-channel-manifest.toml under ${RUSTUP_HOME}/toolchains/<ver>-<host>/lib/rustlib/ against RUST_*_CHANNEL_SHA256 (confirm byte-equality with one CI run), or install from the verified manifest"
      - "Crates: `cargo install --locked --path` the unpacked, verified .crate, or assert the pinned sha against the registry-cached .crate after install"
      - "Extend run_issues in container_static.rs so a pinned download must be consumed (forbid `cargo install <name>` / `rustup toolchain install` in a RUN with no post-install hash of the installed artefact)"
      - "Re-run the container workflow and confirm the two manifests are still identical and check still exits 0"
deferred: []
human_verification:
  - test: "On the founder's Apple Silicon Mac, `docker pull --platform linux/amd64 ghcr.io/sat-wik/migrationtool-toolchain@sha256:b8032124716db8f2bfdfea5b693665bb57f37972895f038c2948affb5956e46a`, then run `mt toolchain check --pins /opt/mt/pins.toml` with mt and pins.toml mounted read-only, --network none, as an unprivileged user (README steps)"
    expected: "Pull succeeds anonymously (or after docker login ghcr.io), the check prints every row OK, hayroll and kani as not_installed, and exits 0; image is the digest recorded in container/README.md"
    why_human: "No Docker daemon in this cloud session, and the Mac pull under emulation is a founder-environment fact. CI already proved the same digest on x86-64 runners."
coincidental_reliance_items: []
behavior_unverified_items: []
advisory:
  - finding: "WR-03: regex_major in check.rs takes the first LLVM library match in the c2rust-transpile ldd output, so a later stray LLVM-17 line would pass as 16"
    category: other
    reason: "Not currently exploitable: the image installs only LLVM 16 packages (llvm_packages row OK in CI) and the ldd capture shows 16. Fix: require every match to equal the pinned major, add a two-major fixture."
    evidence_status: "reviewed; no failing test"
  - finding: "WR-01: runner timeout kills only the direct child; descendants keep pipes open and the timeout can surface as OutputNotDrained instead of timed_out=true"
    category: other
    reason: "The tested single-process timeout path works; later phases that run tools spawning children (KLEE, cargo) will hit it. Fix before Phase 2 relies on it."
    evidence_status: "reviewed; no test covers the path"
  - finding: "WR-02, WR-04, WR-05 and IN-01..IN-11 from 01-REVIEW.md remain open"
    category: other
    reason: "Hardening items (curl -q/--globoff in hash helper, publish only from main after compare, brace-import blind spot in the process::Command guard); none contradicts a ROADMAP criterion"
    evidence_status: "reviewed; dispositions all 'open'"
---

# Phase 1: Pinned Toolchain Container Verification Report

**Phase Goal:** Every compiler and analyser the harness and translator need runs as a pinned subprocess inside one reproducible container, on a single pinned LLVM.
**Verified:** 2026-10-09
**Status:** gaps_found
**Re-verification:** No, initial verification

Evidence base: local source and `cargo test --workspace --locked` (all suites pass, 74 tests), plus the GitHub Actions record for container run 37983108094 at 537ef91 (jobs mt, image-a, image-b, compare all `success`; annotations read directly). Between 537ef91 and HEAD only `container/README.md` and planning files changed, so the CI proof applies to the shipped Dockerfile, pins and code. CI run 37985742991 at HEAD was still in progress when checked; fd87d41 (previous push) was green.

## Goal Achievement

### Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | One command in the container prints the exact version of every pinned tool and exits non-zero if any is missing or off its pin | VERIFIED | `mt toolchain check` in CI image-a ("Live check inside the image", `--network none`, unprivileged): annotation `check-a` has rows for apt pins, arm_gcc 14.3.Rel1, bear 3.1.1, c2rust 0.22.1, cargo_mutants 27.1.0, clang 16.0.6, klee 3.2, llvm_config 16.0.6, qemu_arm and qemu_system_arm 7.2+dfsg, rustup 1.29.1, rustc_tool 1.99.0 (LLVM 23), rustc_bitcode 1.72.1 LLVM 16.0.5, all OK, result OK, exit 0. Negative-pin step on a pins copy with LLVM 17 exits 1 with MISSING/MISMATCH rows (c2rust, klee, rustc_bitcode, llvm_packages, clang, llvm_config). Hayroll and Kani show `not_installed` with reasons, matching CONTEXT D-10 and not named in the roadmap list. |
| 2 | One LLVM major in 16-19 pinned for clang, KLEE, c2rust; check fails on a different major; pinned rustc LLVM at or below 19, recorded provisional until Phase 6 R1 | VERIFIED | `container/pins.toml` `[llvm] major=16 max_major=19 status="provisional" decision="D-15"`; PROJECT.md D-15 carries tokens `llvm=16 bitcode-rustc=1.72.1`; static test ties pins to the decision. CI check rows show LLVM 16 for clang, klee, c2rust, llvm_config, rustc_bitcode (16.0.5) and the check output prints "provisional, decision D-15". Off-pin run proves MISMATCH when the major differs. Caveat WR-03 (advisory): the c2rust probe uses the first `ldd` match only. |
| 3 | Building the image twice yields identical tool-version manifests, and every base image and tool source is pinned by digest, commit or checksum with no floating tags | FAILED (partial) | Identical manifests: VERIFIED. Jobs image-a and image-b (B built with `--no-cache`) produced manifests, and compare annotation reads "byte-identical; sha256 40e55cc6...aed3". Pins: base by `sha256:` digest, BuildKit image by digest, apt snapshot plus exact versions, KLEE/klee-uclibc by commit asserted with `rev-parse HEAD`, no floating FROM or syntax directive. FAILED part: CR-01 confirmed in `container/Dockerfile` (see gaps). The pinned sha256 of the two Rust channel manifests, c2rust and cargo-mutants is verified on a download the installer never uses, so those pins do not bind installed bytes. |
| 4 | A trivial `no_std` crate builds for `thumbv7em-none-eabihf` in the container; the workspace launches an analyser through the subprocess runner with stdout, stderr, exit status and version stamp captured; a repository check fails on Python outside `research/` | VERIFIED | Smoke annotation (user 65534, no network): `Finished release`, arm-none-eabi-gcc 14.3.1, `nm libmt_smoke.a` shows `T mt_smoke_add`, `nm smoke.o` shows `U mt_smoke_add`, exit 0. `runner::run` returns a `RunRecord` with argv, cwd, sorted env, `ToolStamp{name,path,version}`, exit code/signal/timed_out, and stdout/stderr sha256, byte counts and truncation flags (runner.rs lines 128-145, 365-377); 9 `tool_01_runner_*` tests pass. `mt toolchain check` launches every analyser (clang, klee, c2rust, bear, qemu, arm gcc, cargo-mutants, rustc) through `runner::run` inside the image (CI proof above); `process::Command` is confined to runner.rs by a test. `tool_01_no_python_outside_research` exists in repo_checks.rs and passes; `research/.gitkeep` is tracked and no `.py*` file exists elsewhere. Note: the check keeps only exit code, timed_out and output text from each run; no run record from a real analyser is persisted (see Assessment of open items). |

**Score:** 3/4 truths verified, 0 present-but-behavior-unverified.

### Assessment of the open items raised by the orchestrator

1. **Criterion 4, "launches an analyser through the runner with stdout, stderr, exit status and version stamp captured".** Satisfied by two pieces taken together: the runner contract is implemented and tested (record fields above), and `mt toolchain check` runs each pinned analyser through that runner inside the image, proven in CI. What does not exist is a persisted run record from a real analyser run on real input (check.rs `run_one` reads `output.record.exit` and discards the rest; `observe` passes `tool_version: None` for the version probes). The criterion does not require an analyser run on input, so I count it VERIFIED. The first real analyser-on-input run (build capture, Phase 2; sanitizers and KLEE, Phases 4 and 6) is where run records first have a consumer. COVERAGE.md D6 is therefore a Phase 2+ hand-off, not a Phase 1 gap, but it should be named as an owned item in Phase 2 planning.
2. **CR-01 and TOOL-03.** Confirmed by reading the Dockerfile, not just the review. It is a real defect against the controlled claim "pinned by checksum", and the phase's purpose is that control, so it is recorded as the one gap. It is small to close (see `missing`). The practical exposure today is limited by rustup's and cargo's own hashing and by the post-build rustc commit check, which is why I do not call it a security incident, but the pins in pins.toml are not what is enforcing integrity for those four sources.
3. **Apple Silicon pull-and-check** is recorded as a human verification item.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/mt-toolchain/src/runner.rs` | Single subprocess runner | VERIFIED | Exists, substantive, used by check.rs and fetch.rs; 9 runner tests pass |
| `crates/mt-toolchain/src/check.rs` | Pins-driven check and evaluation | VERIFIED | Wired from `mt toolchain check`; CI exercised |
| `crates/mt-toolchain/src/{pins,version,build_args,manifest,fetch}.rs` | Pins schema, parsers, build args, manifest, hash helper | VERIFIED | Present and wired through `mt-cli`; tests pass |
| `container/pins.toml` | Single source of pins | VERIFIED | Validates; used by build-args and check in CI |
| `container/Dockerfile` | Pinned multi-stage image | VERIFIED with gap | Built twice in CI; CR-01 pattern in rustup and cargo-tools stages |
| `.github/workflows/{ci,container,pin-discovery}.yml` | CI proofs | VERIFIED | Runs green at 537ef91 / fd87d41; actions SHA-pinned |
| `container/smoke/*` | no_std smoke crate | VERIFIED | Built for thumbv7em-none-eabihf in CI |
| `crates/mt-toolchain/tests/repo_checks.rs`, `deny.toml`, `research/.gitkeep` | Repository guards | VERIFIED | Tests pass locally (WR-05 hardening open) |
| `container/README.md` | Digest and founder guide | VERIFIED | Digest b8032124...e46a matches the CI `image-digest` and `verified-image` annotations; no "certified/compliant" claims found |

### Key Link Verification

| From | To | Via | Status |
|------|----|-----|--------|
| mt-cli main.rs | check::run_check | toolchain check subcommand | WIRED |
| check.rs | runner::run | every version probe | WIRED |
| container.yml | Dockerfile | `--target final` with args from `mt toolchain build-args` | WIRED (CI green) |
| Dockerfile ARG block | pins.toml | `tool_03_dockerfile_args_match_pins` | WIRED |
| pins.toml `[llvm]` / `[rust.bitcode]` | PROJECT.md D-15 | decision-token test | WIRED |
| Dockerfile pinned downloads | installed artefacts | sha256 of rust manifests and crates | NOT_WIRED (CR-01) |

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| TOOL-01 | 01-01, 01-02, 01-04, 01-05, 01-07, 01-08 | Workspace runs analysers as pinned subprocesses, never linked; Python outside research/ fails a check | SATISFIED | Runner, `process::Command` confinement test, deny/Cargo.lock ban on analyser-binding crates, Python walk test, CI check run. Hayroll and Kani are `not_installed` by decision D-10 (adapters Phase 8 / v2.0) |
| TOOL-02 | 01-03, 01-05, 01-06, 01-07 | One LLVM major 16-19 for C-side tools; check fails on different major or rustc LLVM above 19 | SATISFIED | Pins, check rules, negative-pin CI step. Advisory WR-03 |
| TOOL-03 | 01-02, 01-03, 01-05, 01-06, 01-07, 01-08 | Image reproducible: base, tool sources, Rust toolchain pinned by digest, commit or checksum; two builds give identical manifest | PARTIAL | Identical manifests proven; pins not all bound to installed bytes (CR-01). REQUIREMENTS.md marks this Complete; it should read Pending until the gap closes |
| TOOL-04 | 01-04, 01-07 | arm-none-eabi GCC and thumbv7em target; trivial no_std crate builds | SATISFIED | Smoke annotation in CI |

No orphaned requirements: REQUIREMENTS.md maps only TOOL-01..04 to Phase 1 and every ID appears in at least one plan's `requirements:`.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Workspace tests | `cargo test --workspace --locked` | all suites ok (8, 20, 18, 12, 7, 9 tests, others 0) | PASS |
| Python guard | `find . -name '*.py*'` outside research/.claude/target/.git | none | PASS |
| Unsafe in tool crates | grep for `unsafe` outside `forbid` lines | none | PASS |

Step 7b container behaviors were not re-run (no Docker daemon); they were read from CI annotations instead.

### Probe Execution

Step 7c: SKIPPED. No `probe-*.sh` is declared by any plan or present under `scripts/`.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| container/Dockerfile | rustup and cargo-tools RUN steps | Verify-then-refetch (CR-01) | Blocker | Pins do not bind installed bytes for Rust manifests, c2rust, cargo-mutants |
| crates/mt-toolchain/src/runner.rs | 342-354 | Timeout kills only the direct child (WR-01) | Warning | Descendants can outlive a timeout |
| crates/mt-toolchain/src/check.rs | 730-745 | First-match LLVM major (WR-03) | Warning | Stray second major in ldd output passes |
| .github/workflows/container.yml | 16-47, 372-414 | Publish on any branch push; cancel group (WR-04) | Warning | A tag can move; unverified image can be pushed |
| crates/mt-toolchain/tests/repo_checks.rs | 109-126 | Needle misses brace imports (WR-05) | Warning | Guard weaker than stated |

No TBD/FIXME/XXX markers were checked beyond the reviewed files; the review's file list covers all phase source.

### Human Verification Required

1. **Apple Silicon pull and check.** Pull the recorded digest with `--platform linux/amd64`, run `mt toolchain check` as in README (mounted mt and pins.toml, no network, unprivileged). Expect all rows OK and exit 0. Why human: no Docker here and the Mac path (emulation, GHCR anonymous pull) cannot be seen from CI.

### Gaps Summary

One gap, and it is on the controlled claim the phase exists to deliver. The container builds, checks, and reproduces its manifest in CI, one LLVM major is enforced, the no_std smoke crate builds for Cortex-M, and the runner and repository guards are real and tested. What falls short is ROADMAP criterion 3 and TOOL-03: for the Rust channel manifests, c2rust and cargo-mutants the project verifies a pinned sha256 on a separate download that is then discarded, so the pin does not govern what is installed. Fix the Dockerfile (hash the installed or the actually-used artefact), tighten the static test that let it through, and re-run the container workflow; then close CR-01 in the review disposition. The Apple Silicon pull stays as a human item after that. Warnings WR-01 and WR-03 are worth fixing before Phase 2 depends on the runner and the LLVM probe, but they do not contradict a roadmap criterion today.

---

_Verified: 2026-10-09_
_Verifier: Claude (gsd-verifier)_
