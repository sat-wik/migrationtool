---
phase: 01-pinned-toolchain-container
plan: 09
subsystem: infra
tags: [dockerfile, rustup, cargo-install, sha256, supply-chain, github-actions, ghcr, gap-closure]

requires:
  - phase: 01-pinned-toolchain-container
    provides: "Dockerfile, pins.toml, container workflow with annotation helpers, GHCR publish (plans 01-02, 01-07, 01-08)"
provides:
  - "rustup stage that installs both Rust toolchains only from a file:// mirror of sha256-verified channel manifests and the archives they list, with --no-self-update"
  - "cargo-tools stage that unpacks each sha256-verified .crate and installs it with cargo install --locked --path"
  - "image build record /opt/cargo-tools/share/mt/pin-binding.sha256 (sha256 of every file an installer consumed)"
  - "image-a Pin binding evidence gate that fails the job before the GHCR push when the record and the pins disagree"
  - "static rules and three tool_03 tests that refuse the CR-01 verify-then-refetch patterns"
  - "verified image ghcr.io/sat-wik/migrationtool-toolchain@sha256:0f3b392427d2588e6ba0e3da2067bc503f40e6749b48bc6553832c27e96e518d"
affects: [phase-02, container, TOOL-03, 01-VERIFICATION gap CR-01]

plan_head_before: ea06c6455746fc34bf66c869e1f9b31d34a1b8bb
plan_head_after: 134b396231644e3bc7a5af9597a9192726256059

actuals:
  tokens: 8200
  tasks: 3
  commits: 4

tech-stack:
  added: []
  patterns:
    - "Verify-then-consume: a pinned download is checked and then handed to the installer as a local mirror or path, never fetched twice"
    - "Build record plus CI gate: the image carries sha256 lines of consumed files and image-a matches whole lines against the pins before publishing"

key-files:
  created: []
  modified:
    - container/Dockerfile
    - crates/mt-toolchain/tests/container_static.rs
    - .github/workflows/container.yml
    - container/README.md
    - .planning/phases/01-pinned-toolchain-container/01-VALIDATION.md
    - .planning/phases/01-pinned-toolchain-container/01-REVIEW-DISPOSITION.md

key-decisions:
  - "Bind the channel-manifest pins through a file:// RUSTUP_DIST_SERVER mirror, not by hashing rustup's stored multirust-channel-manifest.toml (rustup re-serializes it, so it never equals the pin)"
  - "Every rustup toolchain install runs with --no-self-update so the sha256-pinned rustup 1.29.1 stays in the image"
  - "Install c2rust and cargo-mutants from the unpacked, sha256-verified .crate with --locked --path; the packaged Cargo.lock fixes dependency checksums"
  - "The image-a gate matches whole record lines and treats rustup's 20-hex update-hash only as a prefix witness"

patterns-established:
  - "Static Dockerfile rules are applied per RUN and per command segment (split on ; and &&)"
  - "Gate evidence is published as a pin-binding notice (error plus exit 1 on mismatch) placed before the publish steps"

requirements-completed: [TOOL-03]

coverage:
  - id: D1
    description: "Both Rust toolchains install only from a file:// mirror of the sha256-pinned channel manifests and the archives they list, with --no-self-update"
    requirement: TOOL-03
    verification:
      - kind: unit
        ref: "crates/mt-toolchain/tests/container_static.rs#tool_03_dockerfile_installs_rust_toolchains_from_verified_manifests"
        status: pass
      - kind: integration
        ref: "container run 38019633821 image-a pin-binding notice: whole record lines for channel-rust-1.99.0.toml and channel-rust-1.72.1.toml equal their pins"
        status: pass
    human_judgment: false
  - id: D2
    description: "c2rust and cargo-mutants install from the sha256-verified, unpacked .crate with cargo install --locked --path"
    requirement: TOOL-03
    verification:
      - kind: unit
        ref: "crates/mt-toolchain/tests/container_static.rs#tool_03_dockerfile_installs_crates_from_verified_files"
        status: pass
      - kind: integration
        ref: "container run 38019633821 image-a pin-binding notice: whole record lines for c2rust-0.22.1.crate and cargo-mutants-27.1.0.crate equal their pins"
        status: pass
    human_judgment: false
  - id: D3
    description: "image-a refuses to publish when the build record and the pins disagree"
    requirement: TOOL-03
    verification:
      - kind: unit
        ref: "crates/mt-toolchain/tests/container_static.rs#tool_03_workflow_gates_on_pin_binding_evidence"
        status: pass
      - kind: integration
        ref: "local dry run of the step with a fake docker: OK on a matching record, exit 1 and MISMATCH when a record line is changed or removed"
        status: pass
    human_judgment: false
  - id: D4
    description: "Container run green end to end with unchanged manifests; ci green for the final pushed commit"
    requirement: TOOL-03
    verification:
      - kind: integration
        ref: "container run 38019633821 (686b56b): mt, image-a, image-b, compare success; check-a exit 0; negative-pin exit 1; manifest sha256 40e55cc6...7aed3; smoke exit 0"
        status: pass
      - kind: integration
        ref: "ci run 38020337166 for 134b396 success"
        status: pass
    human_judgment: false
  - id: D5
    description: "README Current image, VALIDATION rows and the CR-01 disposition record the closure"
    requirement: TOOL-03
    verification:
      - kind: unit
        ref: "crates/mt-toolchain/tests/container_static.rs#tool_03_container_readme_references_the_image_by_digest_only"
        status: pass
    human_judgment: false
  - id: D6
    description: "The new image digest pulls and passes mt toolchain check on the founder's Apple Silicon machine"
    verification: []
    human_judgment: true
    rationale: "No cloud session has Docker or the founder's Apple Silicon machine; this replaces the open human item in 01-VERIFICATION.md for the earlier digest"

duration: 35min
completed: 2026-10-10
status: complete
---

# Phase 1 Plan 09: Pins bind the installed bytes (CR-01 gap closure) Summary

**Both Rust channel-manifest pins and the c2rust and cargo-mutants crate pins now govern the installed bytes (file:// rustup mirror with --no-self-update, `cargo install --locked --path` of the verified .crate), refused by static tests and gated by an image-a pin-binding check before the GHCR push, proven in container run 38019633821.**

## Performance

- **Duration:** 35 min of executor time (most of it waiting for two CI runs)
- **Started:** 2026-10-10T02:49:47Z
- **Completed:** 2026-10-10T03:25:33Z
- **Tasks:** 3 (1 tracer, 1 TDD, 1 records)
- **Files modified:** 6
- **Container round trips used:** 2 of the bound of 3

## Accomplishments

- Rustup stage: a POSIX sh RUN downloads each channel manifest, checks it with `sha256sum -c` against `[rust.tool]` / `[rust.bitcode]` pins, writes the `.sha256` sidecar, then downloads and checks every archive the verified manifest lists for the wanted packages (rustc, cargo, rust-std for the host, clippy and rustfmt through `[renames]`, rust-std for the two extra targets). A missing section fails the build naming it; a URL outside `https://static.rust-lang.org/dist/` (or containing `..`) fails the build. rustup then installs with `RUSTUP_DIST_SERVER="file:///tmp/rust-dist"` and `--no-self-update`.
- Cargo-tools stage: each `.crate` is checked, recorded, unpacked with `tar -xzf --no-same-owner`, `Cargo.toml` and `Cargo.lock` asserted present, and installed with `cargo install --locked --root /opt/cargo-tools --path ... --version =<v> <name>`. The tinycbor assertion, the LLVM build record and the `RUSTUP_TOOLCHAIN` exports are unchanged.
- Build record `/opt/cargo-tools/share/mt/pin-binding.sha256` is written in the image and gated by `Pin binding evidence` in image-a (whole-line match against the four pins plus rustup update-hash prefix check) ahead of the GHCR login and push.
- Static rules in `run_issues` / `dockerfile_issues` and three new tests refuse the CR-01 patterns (both pre-fix RUNs are held verbatim as refused fixtures).
- Container run 38019633821 is green end to end and the manifests are byte-identical with the same sha256 as before the fix.

## Task Commits

1. **Task 1 (tracer): channel-manifest pins bind the installed toolchains** - `a8702d8` (fix)
2. **Task 2 (TDD): crates installed from the verified .crate files** - RED `7fdec8c` (test), GREEN `686b56b` (feat)
3. **Task 3: record the closure (README, VALIDATION, REVIEW-DISPOSITION)** - `134b396` (docs)

**Plan metadata:** the SUMMARY commit that follows this file (docs: complete plan), then STATE and ROADMAP.

## Planning-time evidence the plan relied on (re-confirmed in the local pre-flight)

- rustup re-serializes the manifest, so the stored `multirust-channel-manifest.toml` never equals the pin. The pre-flight and CI show it again: 1.99.0 stored 3bd93d33... against pin ce6dddc8...; 1.72.1 stored 28e24aa2... against pin 77113b96.... The `update-hashes` files hold only the first 20 hex characters of the pin (ce6dddc886364f8d7865, 77113b9660855a5ab49e): a witness, not the binding.
- A file:// mirror binds the pin: rustup read only the mirror (the local `sh -eux` run of the new RUN under dash and mawk exited 0 and `rustup toolchain list` showed `1.72.1-x86_64-unknown-linux-gnu` and `1.99.0-x86_64-unknown-linux-gnu (active, default)`; 7 components for 1.99.0 and 3 for 1.72.1). `/tmp/rust-dist` and `/opt/cargo-tools` were removed from the host afterwards.
- Crates: `cargo install --locked --root R --path <unpacked cargo-mutants-27.1.0> --version =27.1.0 cargo-mutants` ran to completion locally; `cargo metadata --locked` on the unpacked c2rust-0.22.1 exited 0 (packaged Cargo.lock needs no change).

## TDD evidence

**Task 1 (tracer), red before the Dockerfile change:** `cargo test -p mt-toolchain --test container_static` reported 17 passed, 3 failed. `tool_03_dockerfile_installs_rust_toolchains_from_verified_manifests` and `tool_03_dockerfile_has_no_floating_references` failed on the real Dockerfile: `RUN installs a toolchain without RUSTUP_DIST_SERVER=file://..., so rustup fetches its own copy of what was checked: set -eux; ...` plus two `rustup toolchain install without --no-self-update may replace the pinned rustup` lines (the CR-01 rustup RUN). `tool_03_workflow_gates_on_pin_binding_evidence` failed with `image-a must read the build record pin-binding.sha256`. After the Dockerfile and workflow change: 20 passed, 0 failed (21 after Task 2).

**Task 2 (TDD):**

- RED (`7fdec8c`): `tool_03_dockerfile_installs_crates_from_verified_files` failed at the real-Dockerfile assertion with `RUN cargo install lacks --path, so it installs from the registry instead of the verified .crate: set -eux; : "${LLVM_MAJOR:?}" ...`. All refused and accepted fixtures before that assertion passed, so the failure is the planned one (the Task 1 Dockerfile still installs from the registry), not a syntax, discovery or fixture fault. `tool_03_dockerfile_installs_rust_toolchains_from_verified_manifests`, `tool_03_dockerfile_has_no_floating_references` and `tool_03_workflow_gates_on_pin_binding_evidence` also went red for the same reason or for the missing `/tmp/crates/` lines. `cargo test` output is not one of the report formats `gsd_run check tdd-red-evidence` reads and this plan is `type: execute`, so the semantic assessment above stands in for a machine record.
- GREEN (`686b56b`): all 21 `container_static` tests pass.
- REFACTOR: none needed.

## Container round trips

| Trip | Pushed commit | Runs | Result |
|------|---------------|------|--------|
| 1 (tracer) | `a8702d8` | container 38018745349, ci 38018745345, pin-discovery 38018745392 | all success; jobs mt 114114816360, image-b 114114949693, image-a 114114949761, compare 114116749894 all success; pin-binding notice held both manifest lines |
| 2 (crates) | `686b56b` | container 38019633821, ci 38019633791, pin-discovery 38019633805 | all success; jobs mt 114117533120, image-a 114117648054, image-b 114117648111, compare 114119219977 all success |
| docs | `134b396` | ci 38020337166 | success (README and planning files are not container trigger paths, so only ci ran) |

Tracer gate: after round trip 1 the tracer `<verify>` commands were re-run (container run success for the last trigger-path commit; both manifest record lines present) and passed, then Task 2 started.

## Final evidence (container run 38019633821, last trigger-path commit 686b56b)

`pin-binding` notice (image-a):

```
Each record line is the sha256 of a file an installer consumed, as written by the image build.
--- build record /opt/cargo-tools/share/mt/pin-binding.sha256
ce6dddc886364f8d786514771212cebe9b731ba82d6b859951c6b0ccc516b6a2  /tmp/rust-dist/dist/channel-rust-1.99.0.toml
... (rustc, cargo, rust-std host, clippy, rustfmt, rust-std thumbv7em-none-eabihf, rust-std x86_64-unknown-linux-musl archives of 1.99.0)
77113b9660855a5ab49e5ff029c998be93fdcea41b062b40420675cf1f4ea229  /tmp/rust-dist/dist/channel-rust-1.72.1.toml
... (rustc, cargo, rust-std host archives of 1.72.1)
e331f411600da65f3ed67033048cf1fcc203d7655974cf5bd6410da8bc653f3b  /tmp/crates/c2rust-0.22.1.crate
07072e7bcdeb425d5e5fdbfd9f15a2c749e23cb2edf5ef40aee5876760ae1cf9  /tmp/crates/cargo-mutants-27.1.0.crate
--- checks (container exit code 0)
OK record line for channel-rust-1.99.0.toml equals its pin
OK rustup update-hash ce6dddc886364f8d7865 for 1.99.0-x86_64-unknown-linux-gnu is a prefix of the pin
OK record line for channel-rust-1.72.1.toml equals its pin
OK rustup update-hash 77113b9660855a5ab49e for 1.72.1-x86_64-unknown-linux-gnu is a prefix of the pin
OK record line for c2rust-0.22.1.crate equals its pin
OK record line for cargo-mutants-27.1.0.crate equals its pin
--- rustup's stored multirust-channel-manifest.toml (rustup's re-serialized copy, not the binding)
28e24aa2f6333ce95cc8f1e4fead4a3475bbc2d4eedfa562c1c5f2f5c7e0b523  /opt/rustup/toolchains/1.72.1-x86_64-unknown-linux-gnu/lib/rustlib/multirust-channel-manifest.toml
3bd93d33248db8d373f1b7d53d344ee12ae2815c11653449ce5f227a0ec9d3cb  /opt/rustup/toolchains/1.99.0-x86_64-unknown-linux-gnu/lib/rustlib/multirust-channel-manifest.toml
```

- check-a: `rustup  1.29.1  1.29.1  -  OK` (the pinned rustup survived the toolchain installs), `c2rust  0.22.1  0.22.1  16  OK`, `cargo_mutants  27.1.0  27.1.0  -  OK`, `result: OK`, `exit code: 0`.
- negative-pin: `exit code: 1 (required: exactly 1, with MISMATCH on klee and rustc_bitcode)`.
- manifest-compare: `manifest-a.json and manifest-b.json are byte-identical; sha256 40e55cc69fa93f238960bad268cfa4ec5775d51f98dd10efcf92741801d7aed3` (same as the 01-08 value, so no tool version or source hash changed).
- smoke: `ran as: unprivileged user 65534`, `exit code: 0`.
- verified-image: `ghcr.io/sat-wik/migrationtool-toolchain@sha256:0f3b392427d2588e6ba0e3da2067bc503f40e6749b48bc6553832c27e96e518d` (recorded in container/README.md `## Current image`; it replaces the earlier digest b8032124...e46a, which was built with the unbound pins).

## Files Created/Modified

- `container/Dockerfile` - rustup stage mirror install without self-update, cargo-tools stage `--path` installs, build record
- `crates/mt-toolchain/tests/container_static.rs` - `run_issues`/`dockerfile_issues` rules, `CR01_RUSTUP_RUN`, `CR01_CARGO_RUN`, three new tests, accepted fixture changed to the `--path` form
- `.github/workflows/container.yml` - image-a `Pin binding evidence` step, `pin-binding` in the header title list
- `container/README.md` - Current image digest and run, two Bumping pins sentences
- `.planning/phases/01-pinned-toolchain-container/01-VALIDATION.md` - four TOOL-03 rows and a dated audit entry
- `.planning/phases/01-pinned-toolchain-container/01-REVIEW-DISPOSITION.md` - CR-01 fixed, open 16

## Decisions Made

See `key-decisions`. No refused fixture was removed or relaxed: the only removed test lines are the combined `cargo install` rule (split into two issues, both still required) and the registry-form accepted fixture, which the plan told me to replace.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Path traversal guard on manifest URLs**
- **Found during:** Task 1 (rustup stage RUN)
- **Issue:** The plan checks that a component URL starts with `https://static.rust-lang.org/dist/`; a URL such as `https://static.rust-lang.org/dist/../../x` passes that prefix test and would be written outside the mirror.
- **Fix:** The RUN also refuses any URL containing `..`. The manifest itself is sha256-pinned, so this is hardening of T-01-33, not a reaction to an observed URL.
- **Files modified:** container/Dockerfile
- **Verification:** local `sh -eux` pre-flight exit 0; container run 38018745349 green
- **Committed in:** a8702d8

**2. [Rule 2 - Missing Critical] Distinct issue messages for the cargo install rule**
- **Found during:** Task 2 (static rule)
- **Issue:** The existing rule reported "lacks `--locked` or an exact `--version =`" as one message, so the "missing --locked" and "missing exact version" refused fixtures could not be told apart and one could pass for the other.
- **Fix:** Split into two issues (both still required); each refused fixture asserts its own message. No requirement was relaxed.
- **Files modified:** crates/mt-toolchain/tests/container_static.rs
- **Verification:** `tool_03_dockerfile_installs_crates_from_verified_files` asserts both; 21 tests pass
- **Committed in:** 7fdec8c

---

**Total deviations:** 2 auto-fixed (2 missing critical hardening)
**Impact on plan:** Both strengthen checks the plan asked for; no scope creep. None of the pre-decided responses (R-a, R-b, R-c) and none of the founder-decision stops was triggered.

## Issues Encountered

- I used one-off `python3` and `perl` heredocs to rewrite a Dockerfile stage and edit the workflow in place. Nothing of that went into the repository (CLAUDE.md allows Python only under `research/`); later edits used the editor or shell.
- A first attempt at a plain `sleep` wait was refused by the sandbox; polling used bounded `timeout ... until` loops instead. Both container runs finished in about 15 minutes.

## Known Stubs

None.

## Threat Flags

None. The new surface (the image-a gate reading the build record from the built image, the mirror in the build stage) is covered by T-01-29 to T-01-35 in the plan's threat model.

## User Setup Required

None - no external service configuration required.

## Open human item

Apple Silicon pull of the new digest is still open (it replaces the earlier open item for digest b8032124...). On the Mac: `docker pull --platform linux/amd64 ghcr.io/sat-wik/migrationtool-toolchain@sha256:0f3b392427d2588e6ba0e3da2067bc503f40e6749b48bc6553832c27e96e518d`, then run `mt toolchain check --pins /opt/mt/pins.toml` inside it as `container/README.md` describes (mt and pins mounted read-only, `--network none`, unprivileged). Expected: the pull succeeds by digest, every row is OK or not_installed, rustup shows 1.29.1, the bitcode rustc shows LLVM 16.0.5, exit 0.

## Next Phase Readiness

- CR-01 (the single gap in 01-VERIFICATION.md) is closed in code and proven in CI; phase-level re-verification and the TOOL-03 / phase completion marks belong to the orchestrator.
- WR-01..WR-05 and IN-01..IN-11 remain open and advisory. IN-07 (no in-build rustc commit assertion) and WR-04 (every branch push publishes a `sha-<commit>` tag; this plan published two) are untouched.

## Self-Check: PASSED

- Files: container/Dockerfile, crates/mt-toolchain/tests/container_static.rs, .github/workflows/container.yml, container/README.md, 01-VALIDATION.md, 01-REVIEW-DISPOSITION.md all FOUND.
- Commits a8702d8, 7fdec8c, 686b56b, 134b396 are ancestors of HEAD and 134b396 is on origin.
- Full pre-merge suite (fmt, clippy -D warnings, `cargo test --workspace --locked`, `cargo deny check`) green before each push.

---
*Phase: 01-pinned-toolchain-container*
*Completed: 2026-10-10*
