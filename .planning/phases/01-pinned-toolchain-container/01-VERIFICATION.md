---
phase: 01-pinned-toolchain-container
verified: 2026-10-10T06:00:00Z
status: human_needed
score: 4/4 must-haves verified
covered_files:
  - .github/workflows/ci.yml
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
  - .planning/phases/01-pinned-toolchain-container/01-09-PLAN.md
  - .planning/phases/01-pinned-toolchain-container/01-09-SUMMARY.md
  - container/Dockerfile
  - container/README.md
  - container/pins.toml
  - crates/mt-toolchain/src/check.rs
  - crates/mt-toolchain/src/runner.rs
  - crates/mt-toolchain/tests/container_static.rs
covered_digest: "v3:sha256:fc69c94739f645592bd30997551f820c017450156e5014025ebcea3316e01a14"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 3/4
  gaps_closed:
    - "ROADMAP SC3 / TOOL-03 / CR-01: the pinned sha256 of both Rust channel manifests, c2rust and cargo-mutants now governs the installed bytes (plan 01-09)"
  gaps_remaining: []
  regressions: []
gaps: []
deferred: []
human_verification:
  - test: "On the founder's Apple Silicon Mac, `docker pull --platform linux/amd64 ghcr.io/sat-wik/migrationtool-toolchain@sha256:0f3b392427d2588e6ba0e3da2067bc503f40e6749b48bc6553832c27e96e518d`, then run `mt toolchain check --pins /opt/mt/pins.toml` with mt and pins.toml mounted read-only, --network none, as an unprivileged user (container/README.md steps)"
    expected: "Pull succeeds anonymously (or after docker login ghcr.io), the check prints every row OK, hayroll and kani as not_installed, and exits 0; the image is the digest recorded under Current image in container/README.md (this replaces the earlier b8032124... digest)"
    why_human: "No Docker daemon in this cloud session, and the Mac pull under emulation is a founder-environment fact. CI already proved the same digest on x86-64 runners (container run 38019633821)."
coincidental_reliance_items: []
behavior_unverified_items: []
advisory:
  - finding: "WR-07: the image-a Pin binding evidence gate compares the Dockerfile's own build record (sha256 lines it writes after each sha256sum -c) and a tautological update-hash prefix, not an independent re-measurement of the installed bytes"
    category: other
    reason: "The primary control is the in-RUN `sha256sum -c` on the file the installer consumes (traced in the Dockerfile; rustup has only the file:// mirror, cargo installs --path the unpacked checked crate). The gate is regression evidence layered on top. Hardening: read .crates2.json for path+file sources, assert update-hashes in the Dockerfile, RUN --network=none for the install."
    evidence_status: "reviewed; traced in the Dockerfile; CI run 38019633821 pin-binding notice matches all four pins"
  - finding: "WR-06: the new run_issues static rules are whole-RUN and spelling-bound (other rustup fetch verbs, `cargo  install`, `cargo +tc install`, per-install file:// prefix, rustup-init without --default-toolchain none)"
    category: other
    reason: "Affects regression protection only. The current Dockerfile has none of those spellings (grep: two rustup toolchain install lines, both with the file:// prefix and --no-self-update; two cargo install --path lines; one rustup-init with --default-toolchain none). Hardening for a later pin bump."
    evidence_status: "reviewed; grep of the real Dockerfile"
  - finding: "WR-03: regex_major in check.rs takes the first LLVM library match in the c2rust-transpile ldd output"
    category: other
    reason: "Not currently exploitable: image installs only LLVM 16 packages (llvm_packages row OK in CI)."
    evidence_status: "reviewed; no failing test"
  - finding: "WR-01: runner timeout kills only the direct child; descendants can surface as OutputNotDrained instead of timed_out=true"
    category: other
    reason: "Tested single-process timeout path works; fix before Phase 2 relies on tools that spawn children (KLEE, cargo)."
    evidence_status: "reviewed; no test covers the path"
  - finding: "WR-02, WR-04, WR-05, IN-01..IN-14 from 01-REVIEW.md remain open (dispositions 'open')"
    category: other
    reason: "Hardening items; none contradicts a ROADMAP criterion. IN-11 is relevant context for SC3: the manifest contains versions, not installed content, so manifest identity proves version-level reproducibility only, which is exactly what the criterion and TOOL-03 say."
    evidence_status: "reviewed"
---

# Phase 1: Pinned Toolchain Container Verification Report

**Phase Goal:** Every compiler and analyser the harness and translator need runs as a pinned subprocess inside one reproducible container, on a single pinned LLVM.
**Verified:** 2026-10-10
**Status:** human_needed
**Re-verification:** Yes, after gap closure (plan 01-09, CR-01)

Evidence base: local source at HEAD 0eed2ef; `cargo test --workspace --locked` (all suites pass, 77 tests: 8 + 20 + 21 + 12 + 7 + 9); and the GitHub Actions record read directly with `gh api`: container run 38019633821 at 686b56b (jobs mt, image-a, image-b, compare all `success`, run_attempt 1), ci run 38019633791 and pin-discovery run 38019633805 at 686b56b (both `success`). Since 686b56b only planning files and `container/README.md` changed (`git diff 686b56b..HEAD --name-only`); README is not in the container workflow's trigger paths and the Dockerfile, pins, workflow and Rust sources are byte-identical to what CI built, so that run proves the shipped image. Annotations were read directly, not taken from SUMMARY.

## Goal Achievement

### Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | One command in the container prints the exact version of every pinned tool and exits non-zero if any is missing or off its pin | VERIFIED | image-a annotation `check-a` (run 38019633821, `--network none`, unprivileged): rows OK for apt pins, arm_gcc 14.3.Rel1, bear 3.1.1, c2rust 0.22.1 (LLVM 16), cargo_mutants 27.1.0, clang 16.0.6, klee 3.2, llvm_config 16.0.6, qemu_arm and qemu_system_arm 7.2, rustup 1.29.1, rustc_tool 1.99.0 (LLVM 23), rustc_bitcode 1.72.1 LLVM 16.0.5, rust_default, both target sets; `result: OK`, exit 0. Annotation `negative-pin` on a pins copy with LLVM 17: exit 1 with MISSING/MISMATCH on clang, llvm_config, c2rust, klee, rustc_bitcode, llvm_packages. Hayroll and Kani `not_installed` with reasons (CONTEXT D-10; not in the roadmap list of tools for this criterion). |
| 2 | One LLVM major in 16-19 pinned for clang, KLEE, c2rust; check fails on a different major; pinned rustc LLVM at or below 19, recorded provisional until Phase 6 R1 | VERIFIED | `container/pins.toml` `[llvm]` major 16, max_major 19, status provisional, decision D-15; check output line "llvm pin: 16 (provisional, decision D-15); bitcode rustc must report LLVM 16.x and at most 19". Rows show LLVM 16 for clang, klee, c2rust, llvm_config, rustc_bitcode (16.0.5). `negative-pin` proves MISMATCH when the pinned major differs (klee, c2rust, rustc_bitcode, llvm_packages). Advisory WR-03 (first-match ldd probe) is not exploitable in this image. |
| 3 | Building the image twice yields identical tool-version manifests, and every base image and tool source is pinned by digest, commit or checksum with no floating tags | VERIFIED | **Identical manifests:** compare annotation `manifest-compare`: "manifest-a.json and manifest-b.json are byte-identical; sha256 40e55cc69fa93f238960bad268cfa4ec5775d51f98dd10efcf92741801d7aed3" (build B with `--no-cache`); same hash as the pre-fix run, as expected since the manifest holds versions. **Pins:** base by `sha256:` digest, BuildKit by digest, apt snapshot timestamp plus exact versions, KLEE and klee-uclibc commits asserted with `rev-parse HEAD`, tinycbor commit asserted in the checksum-verified c2rust crate, rustup-init and Arm archive by `sha256sum -c`, no floating FROM or syntax directive. **CR-01 closed, traced in the Dockerfile myself:** (a) rustup stage downloads each channel manifest, `sha256sum -c` against `RUST_*_CHANNEL_SHA256`, then every archive that verified manifest lists against its manifest hash, and `rustup toolchain install` runs only with `RUSTUP_DIST_SERVER="file:///tmp/rust-dist"` and `--no-self-update` (lines 226-227; the only two `rustup toolchain install` lines in the file; no ENV for the server), so rustup has no network source and the installed bytes are the checked bytes; (b) cargo-tools stage `sha256sum -c`, `tar -xzf`, then `cargo install --locked --root /opt/cargo-tools --path /tmp/crates/<name>-<version>` of that unpacked directory (lines 295-296), with dependency crates bound by the packaged Cargo.lock under `--locked`. **Evidence in CI:** image-a `pin-binding` notice shows record lines `ce6dddc8...b6a2  .../channel-rust-1.99.0.toml`, `77113b96...a229  .../channel-rust-1.72.1.toml`, `e331f411...3f3b  /tmp/crates/c2rust-0.22.1.crate`, `07072e7b...1cf9  /tmp/crates/cargo-mutants-27.1.0.crate`, each equal to the corresponding value in `container/pins.toml` (lines 41, 51, 64, 70); the record also lists 11 component archives; rustup's stored multirust-channel-manifest.toml hashes (28e24aa2..., 3bd93d33...) differ from the pins and are printed as "not the binding", matching the plan's finding that rustup re-serialises. The step ran before "Log in to GHCR" and "Push the verified image" (steps 10 vs 16, 17) and all succeeded. |
| 4 | A trivial `no_std` crate builds for `thumbv7em-none-eabihf` in the container; the workspace launches an analyser through the subprocess runner with stdout, stderr, exit status and version stamp captured; a repository check fails on Python outside `research/` | VERIFIED | `smoke` annotation (user 65534, no network): `Finished release`, arm-none-eabi-gcc 14.3.1, `nm libmt_smoke.a` shows `T mt_smoke_add`, `nm smoke.o` shows `U mt_smoke_add`, exit 0. `runner::run` returns a `RunRecord` with argv, cwd, sorted env, ToolStamp, exit code/signal/timed_out and stdout/stderr hashes (9 `tool_01_runner_*` tests pass); `mt toolchain check` launches every analyser through it in the image (tool-outputs annotations show captured stdout/stderr for klee, llvm_config, qemu, rustc, rustup). `tool_01_no_python_outside_research` passes; `find` for `*.py*` outside research, `.claude`, target and .git returns nothing. Persisted run records for real analyser-on-input runs begin with Phase 2 (hand-off, not a Phase 1 gap, as in the first report). |

**Score:** 4/4 truths verified, 0 present-but-behavior-unverified.

### Judgement on the new review warnings (WR-06, WR-07) against criterion 3

Neither leaves criterion 3 or TOOL-03 unmet.

- The criterion is "every base image and tool source is pinned by digest, commit or checksum". The control that delivers it is the `sha256sum -c` on the file the installer then consumes, in the same RUN, with the installer unable to reach any other source (file:// mirror with no fallback; `--path` of the unpacked checked crate). I read those RUNs line by line; the check and the consumption are the same bytes. That was the CR-01 defect and it is gone.
- WR-07 is about the CI gate, which compares the record the Dockerfile writes, so it cannot detect a future edit that keeps the record right but points the installer elsewhere. It is regression evidence layered over a correct control, not the control. Its real value today is that CI shows, for the published image, the pins and record agree and rustup's update-hashes are consistent. The "witness" wording in the gate is honest that the update-hash is derived from the Dockerfile-written sidecar. Treat as hardening.
- WR-06 says the static rules would miss alternate spellings. I grep-checked the real Dockerfile: the only `rustup toolchain install` lines (226, 227) carry the file:// prefix and `--no-self-update`; the only `cargo install` lines (295, 296) use `--locked --path`; the only `rustup-init` use passes `--default-toolchain none`; `rustup default` (228) acts on an already-installed toolchain. So the current file has none of the evading spellings. It is a weaker guard, not a present hole. Treat as hardening to land before the next pin bump or Dockerfile edit.
- Residual limits that remain true and are not criterion-3 failures: apt packages are pinned by snapshot timestamp plus exact version rather than checksum (accepted at planning, CONTEXT D-04; apt verifies signed Release files); dependency crates are bound via the packaged Cargo.lock (IN-14); the tinycbor check greps for the commit string in the verified crate (IN-14). The stated guarantee remains manifest-identical builds, not bit-for-bit, and the README says so.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `container/Dockerfile` | Pinned multi-stage image, installs bound to checked files | VERIFIED | Built twice in CI; mirror install and `--path` crate install present; build record written; no TBD/FIXME/XXX |
| `container/pins.toml` | Single source of pins | VERIFIED | Channel and crate pins match the CI record lines |
| `container/README.md` | Current image digest and founder guide | VERIFIED | Current image `0f3b3924...e518d`, run 38019633821, commit 686b56b, manifest sha256 40e55cc6...aed3 equal the CI `image-digest` and `verified-image` annotations; no certified/compliant claims (grep) |
| `.github/workflows/container.yml` | image-a Pin binding evidence gate before push | VERIFIED | Step 10 of image-a precedes login (16) and push (17); fails with exit 1 on mismatch; WR-07 and IN-12 hardening open |
| `crates/mt-toolchain/tests/container_static.rs` | Rules refusing CR-01 patterns, three new tool_03 tests | VERIFIED | Tests `tool_03_dockerfile_installs_rust_toolchains_from_verified_manifests`, `..._crates_from_verified_files`, `tool_03_workflow_gates_on_pin_binding_evidence` exist and pass; WR-06 limits noted |
| `crates/mt-toolchain/src/{runner,check,pins,version,build_args,manifest,fetch}.rs` | Runner, check, pins | VERIFIED | Unchanged since first verification; wired from `mt-cli`; tests pass |
| `.github/workflows/{ci,pin-discovery}.yml`, `container/smoke/*`, `deny.toml`, `research/.gitkeep`, repo_checks.rs | CI proofs and repository guards | VERIFIED | ci and pin-discovery green at 686b56b |

### Key Link Verification

| From | To | Via | Status |
|------|----|-----|--------|
| mt-cli main.rs | check::run_check | toolchain check subcommand | WIRED |
| check.rs | runner::run | every version probe | WIRED |
| container.yml | Dockerfile | `--target final`, args from `mt toolchain build-args` | WIRED (CI green) |
| Dockerfile ARG block | pins.toml | `tool_03_dockerfile_args_match_pins` | WIRED |
| Dockerfile rustup stage | pins.toml `channel_manifest_sha256` | sha256sum -c on the mirror manifest that rustup reads via RUSTUP_DIST_SERVER=file:// | WIRED (was NOT_WIRED; CI record lines equal pins) |
| Dockerfile cargo-tools stage | pins.toml `sha256` of c2rust, cargo-mutants | sha256sum -c then `cargo install --locked --path` of the unpacked crate | WIRED (was NOT_WIRED) |
| container.yml image-a | image build record | `Pin binding evidence` reads pin-binding.sha256 from mt-toolchain:ci, fails before push | WIRED (self-reported record, see WR-07) |
| pins.toml `[llvm]`, `[rust.bitcode]` | PROJECT.md D-15 | decision-token test | WIRED |

### Data-Flow Trace (Level 4)

Not applicable: no component renders dynamic data. The equivalent trace was done for the pins: pins.toml -> build-args -> Dockerfile ARG -> `sha256sum -c` -> installer input -> check row, all connected.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Workspace tests | `cargo test --workspace --locked` | 77 tests, all suites ok | PASS |
| Python guard | `find . -name '*.py*'` outside research/.claude/target/.git | none | PASS |
| Install lines in Dockerfile | `grep -n "RUSTUP_DIST_SERVER\|rustup toolchain install\|cargo install\|rustup-init" container/Dockerfile` | only the file://-prefixed installs and `--path` installs | PASS |

Container behaviours were not re-run (no Docker daemon); they were read from CI annotations of run 38019633821.

### Probe Execution

Step 7c: SKIPPED. No `probe-*.sh` is declared by any plan or present under `scripts/`.

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| TOOL-01 | 01-01, 01-02, 01-04, 01-05, 01-07, 01-08 | Analysers as pinned subprocesses, never linked; Python outside research/ fails a check | SATISFIED | Runner, `process::Command` confinement test, Python walk test, CI check run. Hayroll and Kani `not_installed` by D-10 |
| TOOL-02 | 01-03, 01-05, 01-06, 01-07 | One LLVM major 16-19; check fails on different major or rustc LLVM above 19 | SATISFIED | Pins, check rules, negative-pin step in CI |
| TOOL-03 | 01-02, 01-03, 01-05, 01-06, 01-07, 01-08, 01-09 | Image reproducible: base, tool sources, Rust toolchain pinned by digest, commit or checksum; two builds give identical manifest | SATISFIED | CR-01 closed (traced, plus CI pin-binding record equal to pins); manifests byte-identical in run 38019633821. Hardening items WR-06 and WR-07 open |
| TOOL-04 | 01-04, 01-07 | arm-none-eabi GCC, thumbv7em target, trivial no_std crate builds | SATISFIED | Smoke annotation in CI |

All four IDs appear in at least one PLAN's `requirements:` and are the only IDs REQUIREMENTS.md maps to Phase 1. No orphans. Bookkeeping: REQUIREMENTS.md traceability rows (lines 168-171) and checkboxes (16-19) still read "Gaps Found" / unchecked after the first verification; they should be moved to Complete by the orchestrator only after the human item below is accepted, or immediately if the founder treats it as non-blocking.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| crates/mt-toolchain/src/runner.rs | 342-354 | Timeout kills only the direct child (WR-01) | Warning | Descendants can outlive a timeout |
| crates/mt-toolchain/src/check.rs | 730-745 | First-match LLVM major (WR-03) | Warning | Stray second major in ldd output passes |
| .github/workflows/container.yml | 16-47, 372-414 | Publish on any branch push; cancel group (WR-04) | Warning | A tag can move before compare; unverified image can be pushed |
| crates/mt-toolchain/tests/container_static.rs | 297-370 | Whole-RUN, spelling-bound rules (WR-06) | Warning | Guard weaker than its name; current Dockerfile passes correctly |
| .github/workflows/container.yml | 229-330 | Gate compares self-reported record (WR-07) | Warning | Regression evidence, not independent measurement |
| crates/mt-toolchain/tests/repo_checks.rs | 109-126 | Needle misses brace imports (WR-05) | Warning | Guard weaker than stated |

No TBD/FIXME/XXX markers in the Dockerfile, README, workflows or crate sources (the one FIXME string is a test fixture in `pins_and_versions.rs`). The CR-01 blocker from the first report is resolved and no new blocker was found.

CI note on a03afa5: ci run 38020608044 attempt 1 failed in the step "Install the pinned Rust toolchain" (a runner-side toolchain download), and attempt 2 passed. That commit changed only planning files, so the failure is infrastructure flake in the CI toolchain-install step, not the phase deliverables. It does not affect any criterion; if it recurs, it points at the same network-fetch fragility already tracked as a hardening topic. The ci run for HEAD (0eed2ef, docs only) was still in progress when checked.

### Human Verification Required

1. **Apple Silicon pull and check (new digest).** On the founder's Mac, `docker pull --platform linux/amd64 ghcr.io/sat-wik/migrationtool-toolchain@sha256:0f3b392427d2588e6ba0e3da2067bc503f40e6749b48bc6553832c27e96e518d`, then run `mt toolchain check --pins /opt/mt/pins.toml` with `mt` and `pins.toml` mounted read-only, `--network none`, unprivileged (README steps). Expect all rows OK, hayroll and kani `not_installed`, exit 0. Why human: no Docker here and the emulated-Mac pull path (and anonymous GHCR access) cannot be seen from CI. This is the only item keeping the status from `passed`.

### Gaps Summary

No gaps. The one gap from the first report (CR-01 against criterion 3 / TOOL-03) is closed: the Rust channel-manifest pins, c2rust and cargo-mutants are now consumed by the installers in the same RUN as their `sha256sum -c`, rustup can read only a local mirror of verified files, and CI run 38019633821 shows the build record equal to the pins, byte-identical manifests, check exit 0, negative-pin exit 1, and a green smoke build, all before the image was pushed. The new review warnings WR-06 and WR-07 are regression-protection hardening that do not make any pinned source unbound today; they, plus WR-01..WR-05 and IN-01..IN-14, should be scheduled (WR-01 and WR-06 before Phase 2 and before the next Dockerfile edit respectively). The Apple Silicon pull-and-check remains with the founder.

---

_Verified: 2026-10-10_
_Verifier: Claude (gsd-verifier)_
