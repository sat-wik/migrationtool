---
phase: 01-pinned-toolchain-container
plan: 06
subsystem: infra
tags: [pins-toml, llvm-16-provisional, decision-gate, dockerfile, snapshot-debian, github-actions, pin-discovery, cargo-deny]

requires:
  - phase: 01-pinned-toolchain-container
    provides: "01-02 ci_push_mode executor-pushes; 01-03 pins schema v1, validate/validate_complete, build-args; 01-04 smoke crate and repo guards; 01-05 check, manifest and hash commands"
provides:
  - "container/pins.toml: the complete single source of every pin (validate and validate_complete pass), LLVM 16 and bitcode rustc 1.72.1 marked provisional with decision D-15"
  - "PROJECT.md decisions D-15 (llvm=16 bitcode-rustc=1.72.1) and D-16 (runner is the only std::process user), plus a Key Decisions row; cargo test fails if either pin moves without a matching decision line (CONTEXT D-21)"
  - "container/Dockerfile: 38 default-free global ARGs equal to the image build-args keys, digest-pinned base, snapshot-sources stage on snapshot.debian.org; container/.dockerignore"
  - ".github/workflows/ci.yml (fmt, clippy, test --locked, smoke crate for thumbv7em, cargo-deny at the pinned version) and pin-discovery.yml (apt candidates and Arm checksum published as an annotation)"
  - "crates/mt-toolchain/tests/container_static.rs: 10 static TOOL-02/TOOL-03 tests over the real pins, Dockerfile, workflows and PROJECT.md"
  - "A green ci run and a green pin-discovery run at the pushed commits, with the discovered apt versions and the confirmed Arm sha256 now pinned"
affects: [01-07, 01-08, phase-06-r1-verdict]

actuals:
  tokens: 10200
  tasks: 3
  commits: 4
plan_head_before: 452fe58a5ff5426cbd61dcb4b1938bdfab69a8a0
plan_head_after: 23fc266a1528458c7044fcc0df9daf773b8e0826

tech-stack:
  added: []
  patterns:
    - "Static lints are pure functions over text (dockerfile_issues, dockerfile_args, workflow_use_issues, decision_gate_issues) and each test proves the function refuses a synthetic bad input, so a lint cannot pass vacuously"
    - "The decision gate compares whole whitespace-separated tokens (llvm=16, not a prefix of llvm=161) on the PROJECT.md line named by the pin's decision ID, and fails rather than skips when PROJECT.md is absent"
    - "Workflows publish data through job annotations (title pin-discovery) because Actions log and artifact hosts are not reachable from cloud sessions; values from rendered build-args are read line by line and split at the first = (never evaluated)"
    - "CI-supplied values reach shell steps through env: indirection, not by interpolating step outputs into the script text"

key-files:
  created:
    - container/pins.toml
    - container/Dockerfile
    - container/.dockerignore
    - .github/workflows/ci.yml
    - .github/workflows/pin-discovery.yml
    - crates/mt-toolchain/tests/container_static.rs
  modified:
    - .planning/PROJECT.md

key-decisions:
  - "D-15 added to PROJECT.md: provisional llvm=16 and bitcode-rustc=1.72.1, revisited at the Phase 6 R1 verdict; D-16 added: runner.rs is the only std::process module"
  - "qemu_arm and qemu_system_arm expect the full apt candidate version (1:7.2+dfsg-7+deb12u18+b3) as the plan states; whether the banner prints the +b3 binNMU suffix is unconfirmed and is settled by the first container capture in plan 01-07"
  - "Arm 14.3.rel1 sha256 kept unchanged: the official .sha256asc, a fresh download hashed through mt toolchain hash, and the pinned value are identical"

patterns-established:
  - "Pin discovery by CI annotation: a workflow resolves values only the runners can reach, prints them as a TOML block, and a test ties the workflow's package list to the pinned [apt] key set"

requirements-completed: [TOOL-02, TOOL-03]

coverage:
  - id: D1
    description: "container/pins.toml carries every research-verified pin, passes validate and validate_complete, with LLVM 16 provisional (D-15)"
    requirement: TOOL-03
    verification:
      - kind: unit
        ref: "crates/mt-toolchain/tests/container_static.rs#tool_03_container_pins_are_well_formed"
        status: pass
      - kind: unit
        ref: "crates/mt-toolchain/tests/container_static.rs#tool_03_container_pins_are_complete"
        status: pass
      - kind: unit
        ref: "crates/mt-toolchain/tests/container_static.rs#tool_02_container_pins_llvm_major_in_range_and_provisional"
        status: pass
    human_judgment: false
  - id: D2
    description: "Changing [llvm].major or [rust.bitcode].version without a PROJECT.md decision line carrying matching tokens fails cargo test"
    requirement: TOOL-02
    verification:
      - kind: unit
        ref: "crates/mt-toolchain/tests/container_static.rs#tool_02_llvm_and_bitcode_pins_have_matching_project_decision"
        status: pass
    human_judgment: false
  - id: D3
    description: "rust-toolchain.toml and every tool's expected version agree with the pinned toolchain, sources and apt packages"
    requirement: TOOL-03
    verification:
      - kind: unit
        ref: "crates/mt-toolchain/tests/container_static.rs#tool_03_rust_toolchain_file_matches_pins"
        status: pass
      - kind: unit
        ref: "crates/mt-toolchain/tests/container_static.rs#tool_03_tool_expectations_agree_with_sources"
        status: pass
    human_judgment: false
  - id: D4
    description: "Dockerfile has no floating reference, no ARG default, and declares exactly the build-args keys; both workflows pin every action by 40-hex SHA"
    requirement: TOOL-03
    verification:
      - kind: unit
        ref: "crates/mt-toolchain/tests/container_static.rs#tool_03_dockerfile_has_no_floating_references"
        status: pass
      - kind: unit
        ref: "crates/mt-toolchain/tests/container_static.rs#tool_03_dockerfile_args_match_pins"
        status: pass
      - kind: unit
        ref: "crates/mt-toolchain/tests/container_static.rs#tool_03_workflows_pin_actions_by_commit_sha"
        status: pass
    human_judgment: false
  - id: D5
    description: "Pin discovery ran green at the pushed commits; its annotation supplied the exact Debian versions now in [apt] and the official Arm sha256, which equals the pinned value; ci ran green at the same commits"
    requirement: TOOL-03
    verification:
      - kind: integration
        ref: "gh api actions/workflows/pin-discovery.yml/runs (run 37966083220 success at 23fc266); actions/workflows/ci.yml/runs (run 37966083107 success at 23fc266)"
        status: pass
      - kind: unit
        ref: "crates/mt-toolchain/tests/container_static.rs#tool_03_discovery_package_list_matches_apt_pins"
        status: pass
    human_judgment: false
  - id: D6
    description: "The apt-sourced tools' version regexes (clang, llvm-config, bear, both QEMU binaries) and the qemu expected strings are taken from the plan, not from real container output; the first container capture decides whether any regex or the +b3 suffix needs adjusting"
    requirement: TOOL-02
    verification: []
    human_judgment: true
    rationale: "Real output formats can only be seen by running the tools inside the built image (RESEARCH A5); plan 01-07 captures them"

duration: 12min
completed: 2026-10-09
status: complete
---

# Phase 1 Plan 06: Container pins, decision gate, Dockerfile base and CI workflows Summary

**Complete `container/pins.toml` (LLVM 16 and bitcode rustc 1.72.1 provisional under PROJECT decision D-15, apt versions and the Arm sha256 filled from a green pin-discovery run), a default-free Dockerfile ARG block with a snapshot.debian.org base stage, SHA-pinned ci and pin-discovery workflows, and ten static tests that make every pin, ARG and action reference checkable in plain `cargo test`.**

## Performance

- **Duration:** 12 min
- **Started:** 2026-10-09T17:15:50Z
- **Completed:** 2026-10-09T17:28:00Z (before the SUMMARY commit)
- **Tasks:** 3 (Task 1 `tdd="true"`, two code commits; Tasks 2 and 3 one commit each)
- **Files modified:** 6 created, 1 modified

## Accomplishments

- **Pins.** `container/pins.toml` holds every value research verified: base image by index digest, snapshot timestamp, LLVM 16 (provisional, D-15), tool Rust 1.99.0 and bitcode Rust 1.72.1 (LLVM 16.0.5) with channel-manifest sha256s, rustup-init, KLEE and klee-uclibc commits, c2rust and cargo-mutants crate sha256s, the Arm GNU 14.3.rel1 URL and sha256, the transitive tinycbor commit, the BuildKit image digest and the cargo-deny version. The base digest and the BuildKit digest were re-resolved against Docker Hub and the two action SHAs against `git ls-remote` before committing; all four matched.
- **Decision gate (CONTEXT D-21).** PROJECT.md gained D-15 (`llvm=16 bitcode-rustc=1.72.1`) and D-16, plus a Key Decisions row (only added lines; D-01 to D-14 untouched). The gate test fails, not skips, if PROJECT.md is missing, and is proven to fail for bitcode 1.73.0, LLVM 17, an unknown decision ID and an empty PROJECT.md.
- **Dockerfile.** Header comment, then 38 `ARG` lines with no default (exactly the keys `mt toolchain build-args` renders), then stage `snapshot-sources`: `FROM --platform=${PLATFORM} ${BASE_REF}@${BASE_DIGEST}`, every apt source removed (including the deb822 file), codename read from `/etc/os-release`, snapshot sources for the three suites over http, and apt retry/timeout settings. No syntax directive, no package installation, no version literal.
- **Workflows.** `ci.yml` runs fmt, clippy, `cargo test --workspace --locked`, the smoke crate build and clippy for `thumbv7em-none-eabihf`, and `cargo deny check` with cargo-deny installed at `[ci].cargo_deny_version`. `pin-discovery.yml` builds `mt`, renders the pins, builds `--target snapshot-sources` with BuildKit pinned through `[ci].buildkit_image`, resolves the apt candidates and the Arm checksum, and publishes the block as a `pin-discovery` annotation and in the step summary. Every `uses:` is a 40-hex SHA with its tag as a trailing comment.
- **Discovery round trip.** Both workflows were green on the first round trip, so no fix rounds were needed. The annotation supplied the nine apt versions and confirmed the Arm checksum (see below); those went into `[apt]` and the five apt-sourced tool tables. A second round trip for the completed pins was green again with an identical annotation.

## Pin-discovery annotation (verbatim)

Job `discover` of run 37965680251 at `6916bb2` (job id 113939304018), annotation title `pin-discovery`, level notice. The run at `23fc266` (run 37966083220, job 113940658318) produced a byte-identical block.

```
[apt] candidates at snapshot 20261005T000000Z
"clang-16" = "1:16.0.6-15~deb12u1"
"llvm-16" = "1:16.0.6-15~deb12u1"
"llvm-16-dev" = "1:16.0.6-15~deb12u1"
"libclang-16-dev" = "1:16.0.6-15~deb12u1"
"libclang-cpp16-dev" = "1:16.0.6-15~deb12u1"
"bear" = "3.1.1-1"
"qemu-system-arm" = "1:7.2+dfsg-7+deb12u18+b3"
"qemu-user" = "1:7.2+dfsg-7+deb12u18+b3"
"libz3-4" = "4.8.12-3.1"
arm_gnu official sha256: 8f6903f8ceb084d9227b9ef991490413014d991874a1e34074443c2a72b14dbd
arm_gnu computed sha256: 8f6903f8ceb084d9227b9ef991490413014d991874a1e34074443c2a72b14dbd
arm_gnu pinned sha256:   8f6903f8ceb084d9227b9ef991490413014d991874a1e34074443c2a72b14dbd
base digest pinned:      sha256:7c7b2c966bc9ee8cedfeef67e0e279108992c77681fa595db4a9d65c06ccc587
base digest today:       sha256:7c7b2c966bc9ee8cedfeef67e0e279108992c77681fa595db4a9d65c06ccc587
```

The official `.sha256asc`, the digest of a fresh download hashed through `mt toolchain hash`, and the value pinned from nixpkgs are identical, so `[source.arm_gnu].sha256` was not changed. The base tag still resolves to the pinned digest.

## CI evidence

| Commit | Workflow | Run id | Job (id) | Conclusion |
|--------|----------|--------|----------|------------|
| `6916bb2` | pin-discovery | 37965680251 | discover (113939304018) | success |
| `6916bb2` | ci | 37965680419 | checks (113939304402) | success |
| `23fc266` | pin-discovery | 37966083220 | discover (113940658318) | success |
| `23fc266` | ci | 37966083107 | checks (113940656899) | success |

The Task 3 verify commands passed at `23fc266`: the latest pin-discovery run on the branch is `success` for the last commit that touched `container/pins.toml`, the Dockerfile or the workflow (`23fc266`), and the latest ci run is `success` for HEAD at that time.

## Task Commits

1. **Task 1 RED:** `c26dfc7` (test) - five failing static tests (container/pins.toml and the D-15 line did not exist)
2. **Task 1 GREEN:** `7f335b9` (feat) - container/pins.toml, PROJECT.md D-15/D-16
3. **Task 2:** `6916bb2` (feat) - Dockerfile, .dockerignore, ci and pin-discovery workflows, three lint tests
4. **Task 3:** `23fc266` (feat) - `[apt]` and apt-sourced tools from the discovery annotation, two completeness tests

**Plan metadata:** committed with this SUMMARY (docs: complete plan). `commits: 4` and `plan_head_after` are measured at the last code commit, before the SUMMARY commit.

## TDD Gate Compliance

Task 1 (`tdd="true"`): `test(01-06)` commit `c26dfc7` precedes `feat(01-06)` commit `7f335b9`; no REFACTOR commit. The cargo test (libtest) console format is not a report format `gsd_run check tdd-red-evidence` accepts, so the classifier was not run; the assessment is manual, from the real failing run.

| Task | RED run | Semantic assessment |
|------|---------|---------------------|
| 1 | `cargo test -p mt-toolchain --test container_static`: 5 ran, 0 passed, 5 failed | Every test compiled and executed. Each failed at its first read of `container/pins.toml` ("No such file or directory"), the planned missing artifact; the D-15 assertions were never reached, so the failure proves the tests need the artifact but does not by itself exercise the assertion logic. The assertion logic is exercised in GREEN, including the negative cases in `tool_02_llvm_and_bitcode_pins_have_matching_project_decision` (bitcode 1.73.0, LLVM 17, unknown ID, empty PROJECT.md all rejected). Not a syntax, import or fixture-parse fault. |

Tasks 2 and 3 are `type="auto"` without `tdd="true"`; their tests were added with the code and each lint test proves refusal of synthetic bad inputs.

## Files Created/Modified

- `container/pins.toml` - complete pins, schema v1, no placeholder
- `container/Dockerfile` - 38 global ARGs and the `snapshot-sources` stage
- `container/.dockerignore` - ignores everything except the Dockerfile
- `.github/workflows/ci.yml` - pre-merge cargo checks and cargo-deny (CONTEXT D-23)
- `.github/workflows/pin-discovery.yml` - apt candidates and Arm checksum resolver
- `crates/mt-toolchain/tests/container_static.rs` - ten static tests
- `.planning/PROJECT.md` - D-15, D-16 and one Key Decisions row (three added lines)

## Decisions Made

See `key-decisions`. In short: D-15 and D-16 recorded; the Arm sha256 stays as pinned because three sources agree; the QEMU expected strings follow the plan's "full Debian version" wording and are to be confirmed against real output in plan 01-07.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Hardened workflow inputs**
- **Found during:** Task 2 (writing ci.yml and pin-discovery.yml)
- **Issue:** The plan's cargo-deny step interpolates a step output into the command text, and checkout would persist the job token in the clone (threat T-01-19, T-01-SC are "mitigate").
- **Fix:** `CARGO_DENY_VERSION` is passed through `env:` and expanded by the shell; both checkouts use `persist-credentials: false`.
- **Files modified:** `.github/workflows/ci.yml`, `.github/workflows/pin-discovery.yml`
- **Verification:** both workflows ran green; `tool_03_workflows_pin_actions_by_commit_sha` passes.
- **Committed in:** `6916bb2`

**2. [Rule 2 - Missing Critical] Failure annotation and extra consistency assertions**
- **Found during:** Tasks 2 and 3
- **Issue:** With the Actions log unreachable from cloud sessions, a failing discovery step would have been undiagnosable; and the plan's expectation test did not tie the apt-sourced tools' expected versions to the pinned apt packages.
- **Fix:** pin-discovery.yml has an ERR trap that publishes a `pin-discovery-failure` annotation with the log tail; `tool_03_tool_expectations_agree_with_sources` also asserts clang, llvm-config and bear expect the upstream part of their apt pin (via `version::upstream_version`) and both QEMU tools expect the full package version.
- **Files modified:** `.github/workflows/pin-discovery.yml`, `crates/mt-toolchain/tests/container_static.rs`
- **Verification:** `container_static` passes 10 of 10; the failure path was not exercised because discovery succeeded.
- **Committed in:** `6916bb2`, `23fc266`

---

**Total deviations:** 2 auto-fixed (both Rule 2). **Impact on plan:** no scope change; every touched file is in `files_modified`. No CI-fix round trips were needed (the plan allowed 3).

## Issues Encountered

- The first push carried the earlier unpushed plan commits (`452fe58..6916bb2`) as well as this plan's, since the branch had not been pushed since before wave 1. Scope stayed within the permitted branch (no force, no other ref).
- The Task 3 verify `ci` check requires the ci run to belong to the current HEAD; the SUMMARY commit that follows creates a new HEAD whose ci run is separate. Its conclusion is reported in the final handback, not here.
- The local environment has no Docker daemon, so the Dockerfile stage was exercised only by the pin-discovery runs (it built `snapshot-sources` successfully in both).

## Authentication Gates

None.

## Known Stubs

None. `grep -rn "TODO\|FIXME\|placeholder" container .github` finds no stub; `container/pins.toml` passes `validate_complete`.

## Threat Flags

None. The two workflows and the Dockerfile are the trust-boundary surface already modelled (T-01-17 to T-01-21, T-01-SC); pin-discovery downloads the Arm archive and queries the Debian snapshot, with the Arm hash required to match both the official `.sha256asc` and a fresh download.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- **01-07** can add the remaining Dockerfile stages (apt install from `snapshot-sources`, rustup, arm, cargo-tools, klee) against `container/pins.toml`, which is complete. It should run `mt toolchain check --capture-outputs` in the first image build and, from that capture, confirm or fix: the `clang`, `llvm-config`, `bear`, `qemu-arm` and `qemu-system-arm` version regexes and expected strings (the banner may omit the `+b3` binNMU suffix the apt candidate carries), the c2rust `ldd` LLVM probe, and the representative fixtures in `tool-outputs.toml`. Build-time apt packages (cmake, ninja, git, compilers, libz3-dev) are not in `[apt]`; add them to both `pins.toml` and the discovery package list together if they should be pinned.
- The LLVM 16 and bitcode rustc 1.72.1 pins stay provisional (D-15) until the Phase 6 R1 verdict; nothing here is evidence that Rust-to-KLEE works.
- No blockers.

## Verification Results

Run on `23fc266` before this commit:

- `cargo fmt --check`: pass
- `cargo clippy --all-targets -- -D warnings`: pass
- `cargo test --workspace --locked`: pass (8 end-to-end, 20 check/manifest/hash, 10 container_static, 12 pins/version, 7 repo guards, 9 runner tests)
- `cargo deny check`: advisories ok, bans ok, licenses ok, sources ok
- `cargo run -q -p mt-cli -- toolchain build-args --pins container/pins.toml --scope ci`: prints `BUILDKIT_IMAGE=moby/buildkit:v0.34.0@sha256:b059...` and `CARGO_DENY_VERSION=0.20.2`
- `mt toolchain check --pins container/pins.toml` locally: exit 1 (tools are not installed in the cloud session), not 2, so the pins are valid and complete.

## Self-Check: PASSED

Created files exist (`container/pins.toml`, `container/Dockerfile`, `container/.dockerignore`, both workflows, `container_static.rs`); commits `c26dfc7`, `7f335b9`, `6916bb2` and `23fc266` are ancestors of HEAD; `pins.toml` contains `decision = "D-15"`, `status = "provisional"` and the pinned base digest; the Dockerfile contains `AS snapshot-sources` and `snapshot.debian.org`; `ci.yml` contains `cargo deny check`, `cargo test --workspace --locked` and `contents: read`; `pin-discovery.yml` contains `title=pin-discovery` and `--target snapshot-sources`; PROJECT.md contains D-15 with `llvm=16 bitcode-rustc=1.72.1` and D-16, and `git diff` of PROJECT.md showed only added lines. Acceptance criteria re-run: 10 of 10 `container_static` tests pass, and the Task 3 `gh api` verifies passed at `23fc266`.

---
*Phase: 01-pinned-toolchain-container*
*Completed: 2026-10-09*
