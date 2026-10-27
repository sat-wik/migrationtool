---
phase: 03-equivalence-contract-and-test-vector-schema
plan: 09
subsystem: infra
tags: [qemu, bookworm-backports, apt-preferences, container, pin-discovery, m-profile, github-actions]

requires:
  - phase: 02-test-bed-and-build-capture
    provides: the pinned image, the container workflow (image-a, image-b, compare, publish) and the pin-discovery workflow this plan extends (plan 02-01 pattern)
provides:
  - "QEMU (qemu-user, qemu-system-arm and the qemu-source packages) from bookworm-backports at the pinned snapshot timestamp, selected by an apt preferences file with every other backports package at priority -1"
  - "image-a probe step QEMU M-profile user-mode start (D-31) with notice qemu-mprofile: a -cpu cortex-m4 guest starts and its raw exit code 42 reaches the host, before the image can be saved or published"
  - "pin-discovery qemu-backports notice listing every qemu-user ~bpo12 version the snapshot archive has seen, with first_seen"
  - "new QEMU pins 1:10.0.2+ds-2+deb13u1~bpo12+1 in pins.toml, pins-full.toml and the CI-captured tool-outputs fixture"
  - "PROJECT.md decision D-21 and the 03-01 hand-back edits (expected version string, D-31 pins-comment line)"
  - "green container, pin-discovery and ci runs on the session branch for head 922c116, publish skipped"
affects: [03-10, 03-01 Task 3, 03-02, phase 03 executor and self-test, phase 5 emulator replay (R6)]

actuals:
  tokens: 11300
  tasks: 3
  commits: 6
plan_head_before: f79b8361c884605ddb2bbf671bfead5521ab1ef2
plan_head_after: 922c1169218eda72158427e7cb53b8e2bd6f44ff

tech-stack:
  added: []
  patterns:
    - "apt preferences in the snapshot-sources stage so pin-discovery and the image see the same candidates"
    - "in-image start probe that gates the save and publish steps by omitting a status function from the save step"

key-files:
  created:
    - container/smoke/qemu-mprofile.S
  modified:
    - container/Dockerfile
    - .github/workflows/container.yml
    - .github/workflows/pin-discovery.yml
    - crates/mt-toolchain/tests/container_static.rs
    - container/pins.toml
    - crates/mt-toolchain/tests/fixtures/pins-full.toml
    - crates/mt-toolchain/tests/fixtures/tool-outputs.toml
    - crates/mt-toolchain/tests/check_and_manifest.rs
    - crates/mt-cli/tests/tool_01_check_end_to_end.rs
    - container/README.md
    - .planning/PROJECT.md
    - .planning/phases/03-equivalence-contract-and-test-vector-schema/03-01-PLAN.md

key-decisions:
  - "Accepted QEMU 1:10.0.2+ds-2+deb13u1~bpo12+1 (upstream 10.0.2) for qemu-user and qemu-system-arm: ~bpo12, same version for both, major 10 at least 8. This is NOT 8.2.x as D-31 reads; recorded as a deviation for the founder to see in plan 03-10 before the costly merge."
  - "Source is bookworm-backports at the existing snapshot_timestamp (no new [image] key, no mt-toolchain code change); qemu-system-arm moves with qemu-user."
  - "Priority 990 for exactly qemu-user qemu-system-arm qemu-system-common qemu-system-data and -1 for every other backports package, so a dependency outside the set fails the build."
  - "The qemu-mprofile probe, not the 03-01 self-test, is the pre-publish proof on the branch (testbed pulls only the digest under Current image)."

patterns-established:
  - "Probe scripts must not put a command that exits non-zero by design (qemu-arm -cpu help exits 1) in a pipeline under pipefail"

requirements-completed: [EQV-04]

coverage:
  - id: D1
    description: "Dockerfile snapshot-sources takes only the four qemu-source packages from bookworm-backports at the pinned timestamp, every other backports package at priority -1, no insecure source option, no other stage touches apt sources"
    requirement: EQV-04
    verification:
      - kind: unit
        ref: "crates/mt-toolchain/tests/container_static.rs#eqv_04_dockerfile_takes_only_qemu_from_backports"
        status: pass
      - kind: integration
        ref: "container workflow run 40446062121, job image-a: Build the image (target final) success; check-a apt:qemu-user and apt:qemu-system-arm OK"
        status: pass
    human_judgment: false
  - id: D2
    description: "image-a probe: qemu-arm -cpu cortex-m4 -R 0x10000000 starts a Thumb guest in the built image and exits 42 through a raw svc, before Save the gated image"
    requirement: EQV-04
    verification:
      - kind: unit
        ref: "crates/mt-toolchain/tests/container_static.rs#eqv_04_container_workflow_probes_cortex_m4_user_mode_start"
        status: pass
      - kind: integration
        ref: "container workflow run 40446062121, job image-a, notice qemu-mprofile ends 'exit code: 42'"
        status: pass
    human_judgment: false
  - id: D3
    description: "pin-discovery publishes the informational qemu-backports notice and still reports the ~bpo12 candidates for qemu-user and qemu-system-arm; permissions stay contents: read"
    requirement: EQV-04
    verification:
      - kind: unit
        ref: "crates/mt-toolchain/tests/container_static.rs#eqv_04_pin_discovery_lists_backports_qemu_versions"
        status: pass
      - kind: integration
        ref: "pin-discovery run 40446062155, job discover success, notices qemu-backports and pin-discovery"
        status: pass
    human_judgment: false
  - id: D4
    description: "The QEMU apt pins, tool expects, pins-full.toml, tool-outputs.toml and the CLI/check tests all carry the one discovered version, copied from CI output"
    requirement: EQV-04
    verification:
      - kind: unit
        ref: "cargo test --workspace --locked (tool_01_check_end_to_end_reports_ok_and_exits_zero, tool_03_check_apt_rows_flag_missing_and_wrong_versions, tool_02_container_pins_regexes_parse_captured_outputs)"
        status: pass
      - kind: integration
        ref: "run 40446062121 image-a check-a: apt:qemu-user, apt:qemu-system-arm, qemu_arm, qemu_system_arm OK; llvm_packages 16 16 16 OK"
        status: pass
    human_judgment: false
  - id: D5
    description: "README rows, bumping-pins step and QEMU notice, and PROJECT.md decision D-21 describe the backports QEMU accurately (licence text of 10.0.2 not read, image copyright file named as authority)"
    requirement: EQV-04
    verification:
      - kind: unit
        ref: "crates/mt-toolchain/tests/container_static.rs README tests (tool_03_container_readme_*)"
        status: pass
    human_judgment: true
    rationale: "Tests only refuse forbidden claim words and check required sections; whether the prose is accurate and complete is a reading judgment"
  - id: D6
    description: "03-01-PLAN.md changed only in its expected qemu version string (three places plus the D-30 truth) and its pins-comment line"
    requirement: EQV-04
    verification:
      - kind: other
        ref: "git diff 90fb133 --numstat -- 03-01-PLAN.md reports 6 added lines; no '7.2+dfsg' left; V appears 3 times; 'reworded by plan 03-09 under D-31' present"
        status: pass
    human_judgment: false

duration: 35min
completed: 2026-10-27
status: complete
---

# Phase 03 Plan 09: QEMU from bookworm-backports Summary

**QEMU 10.0.2 (qemu-user and qemu-system-arm) from bookworm-backports at the pinned snapshot through a QEMU-only apt preferences file, proven on the session branch by an in-image probe where `qemu-arm -cpu cortex-m4` starts a Thumb guest and returns exit code 42; container, pin-discovery and ci green for head 922c116, publish skipped.**

## Performance

- **Duration:** about 35 min (start taken from the orchestrator tracking commit f79b836 at 03:22:29Z; the start time was not recorded in a variable)
- **Started:** 2026-10-27T03:22:30Z (approximate)
- **Completed:** 2026-10-27T03:58:00Z (approximate)
- **Tasks:** 3 (Tracer, Task 2, Task 3), CI round trips used: 2 of 3
- **Files modified:** 13 (12 listed in key-files plus Cargo files untouched; Cargo.lock and crates/mt-harness unchanged)

## Accomplishments

- D-31 on the branch: the image takes exactly `qemu-user qemu-system-arm qemu-system-common qemu-system-data` from bookworm-backports; a probe in image-a shows `qemu-arm -cpu cortex-m4 -R 0x10000000` starting a guest whose `svc` exit code 42 reaches the host. The save step carries no status function, so an image that fails the probe is never archived or published.
- Pin discovery reports the backports candidates (same snapshot timestamp) and a new informational `qemu-backports` notice lists every qemu-user `~bpo12` version the snapshot archive has seen.
- Pins, fixtures, tests, README, PROJECT.md D-21 and the two 03-01 hand-back edits all carry the one version, copied from CI output.
- Final round trip (RT2) green: container mt/image-a/image-b/compare success, publish skipped, pin-discovery success, ci success.

## Version decision (version rule of the plan context)

- Candidate in RT1 `pin-discovery` for both packages: `1:10.0.2+ds-2+deb13u1~bpo12+1`. Contains `~bpo12`, the two are equal, upstream major 10 is at least 8: accepted.
- **V = `1:10.0.2+ds-2+deb13u1~bpo12+1`. V is not 8.2.x**, so this is a recorded deviation from D-31's "8.2.x, as measured on the host". In-image evidence: the `qemu-mprofile` notice below (exit code 42 with this release). The `qemu-backports` notice shows the only 8.2.x versions the archive ever held were `8.2.0+ds-5~bpo12+1` (first seen 20240123T212534Z) and `8.2.1+ds-1~bpo12+1` (20240206T163351Z); the pinned snapshot 20261005T000000Z offers 10.0.2. Plan 03-10 must show this to the founder in the PR body and the merge checkpoint text before the costly merge; a different backports timestamp would need a new `[image]` key and mt-toolchain code (gap plan).

## Task Commits

Each task was committed atomically (TDD: RED then GREEN):

1. **Tracer RED** - `260610f` (test): three failing static tests
2. **Tracer GREEN** - `9770902` (feat): Dockerfile backports source and preferences, `qemu-mprofile.S`, image-a probe step, `qemu-backports` discovery step (pushed with RED as RT1)
3. **Task 2 RED** - `0aed190` (test): CLI fake container and off-pin edit expect the new version
4. **Task 2 GREEN** - `58d465c` (feat): pins.toml, pins-full.toml, tool-outputs.toml (CI capture of run 40444623268)
5. **Task 3 fix** - `7177ce4` (fix): cortex-m4 CPU-list line of the probe (Rule 1, see Deviations)
6. **Task 3 docs** - `922c116` (docs): README, PROJECT.md D-21, 03-01 hand-back edits (pushed with 3-5 as RT2)

**Plan metadata:** the docs(03-09) commit that adds this file, STATE.md and ROADMAP.md.

## TDD Gate Compliance

RED/GREEN sequence present for the Tracer and for Task 2; the commits are listed above (`test(03-09)` precedes `feat(03-09)` in each pair). No REFACTOR commit.

**Tracer RED evidence** (`cargo test -p mt-toolchain --test container_static eqv_04`, 3 failed, 30 filtered out), each failing on its planned first assertion with the unchanged implementation:

- `eqv_04_dockerfile_takes_only_qemu_from_backports`: `assertion 'left == right' failed: stage snapshot-sources must write exactly four snapshot apt sources: [... main, ...-updates main, debian-security ...-security main]  left: 3  right: 4`. Assessment: valid RED, the backports source does not exist yet.
- `eqv_04_container_workflow_probes_cortex_m4_user_mode_start`: `no step named 'QEMU M-profile user-mode start (D-31)'`. Assessment: valid RED, the probe step does not exist yet.
- `eqv_04_pin_discovery_lists_backports_qemu_versions`: `no step named 'List qemu-user versions seen in backports (informational, D-31)'`. Assessment: valid RED, the discovery step does not exist yet.

**Task 2 RED evidence** (`cargo test --workspace --locked --no-fail-fast`, 5 failed):

- `tool_01_check_end_to_end_reports_ok_and_exits_zero`, `..._reports_mismatch_and_exits_one`, `..._reports_missing_tool_and_exits_one`, `tool_03_manifest_end_to_end_is_byte_stable_and_diff_locates_the_change`: each prints `apt:qemu-system-arm ... 1:7.2+dfsg-7+deb12u18+b3  1:10.0.2+ds-2+deb13u1~bpo12+1 ... MISMATCH version differs from the pin` (and the same for `apt:qemu-user`, `qemu_arm`, `qemu_system_arm`); the first additionally `left: Some(1) right: Some(0)`. Assessment: valid RED, the fake container now prints the new banners while pins still name 7.2.
- `tool_03_check_apt_rows_flag_missing_and_wrong_versions`: `dpkg_query stdout does not contain "qemu-user\t1:10.0.2+ds-2+deb13u1~bpo12+1"`. Assessment: valid RED, the committed capture still has the old dpkg line.

## CI round trips (2 of 3 used; RT3 not spent)

Evidence only from `gh api repos/sat-wik/migrationtool/actions/...` and check-run annotations.

### RT1 - head `97709020a2c897f7fc8c9ebde6a230e2f5df7657` (RED + GREEN of the Tracer, plus the orchestrator tracking commit f79b836)

| Workflow | Run id | Conclusion |
|----------|--------|------------|
| pin-discovery | 40444623194 | success (job discover 121992828820) |
| ci | 40444623196 | success |
| container | 40444623268 | failure, as expected: mt 121992829282 success; image-a 121993148181 failure; image-b 121993148074 failure; compare 121995966162 skipped; publish 121995967373 skipped |
| testbed (not awaited, not evidence) | 40444623235 | failure (old 7.2 image under Current image) |

image-a steps: Build the image success; Pin binding evidence success; Live check inside the image FAILURE (check-a, four expected rows); Publish the captured tool outputs success; Off-pin check success; Manifest of build A FAILURE; Smoke build success; QEMU M-profile user-mode start (D-31) success; Save the gated image skipped. image-b: Manifest of build B FAILURE. Both manifest failures are the same `result: FAILED (4 rows)` (the manifest refuses to build while check rows mismatch), so they are consequences of the four expected rows, not separate defects.

`pin-discovery` and `qemu-backports` notices of job 121992828820, verbatim:

```
== qemu-backports
qemu-user versions in backports on snapshot.debian.org (informational)
1:8.0.3+dfsg-1~bpo12+1  first_seen 20230815T213225Z
1:8.0.4+dfsg-1~bpo12+1  first_seen 20230818T031630Z
1:8.1.1+ds-2~bpo12+1  first_seen 20231018T211047Z
1:8.1.2+ds-1~bpo12+1  first_seen 20231024T213521Z
1:8.2.0+ds-5~bpo12+1  first_seen 20240123T212534Z
1:8.2.1+ds-1~bpo12+1  first_seen 20240206T163351Z
1:9.0.1+ds-1~bpo12+1  first_seen 20240713T204319Z
1:9.0.2+ds-1~bpo12+1  first_seen 20240724T203333Z
1:9.1.1+ds-2~bpo12+1  first_seen 20241102T211236Z
1:9.1.2+ds-1~bpo12+1  first_seen 20241206T084226Z
1:9.2.0+ds-3~bpo12+2  first_seen 20241227T204205Z
1:9.2.2+ds-1~bpo12+1  first_seen 20250317T205300Z
1:10.0.0+ds-2~bpo12+1  first_seen 20250520T143831Z
1:10.0.0+ds-2~bpo12+2  first_seen 20250527T203152Z
1:10.0.2+ds-2~bpo12+1  first_seen 20250808T084211Z
1:10.0.2+ds-2+deb13u1~bpo12+1  first_seen 20250906T023741Z
== pin-discovery
[apt] candidates at snapshot 20261005T000000Z
"clang-16" = "1:16.0.6-15~deb12u1"
"llvm-16" = "1:16.0.6-15~deb12u1"
"llvm-16-dev" = "1:16.0.6-15~deb12u1"
"libclang-16-dev" = "1:16.0.6-15~deb12u1"
"libclang-cpp16-dev" = "1:16.0.6-15~deb12u1"
"bear" = "3.1.1-1"
"qemu-system-arm" = "1:10.0.2+ds-2+deb13u1~bpo12+1"
"qemu-user" = "1:10.0.2+ds-2+deb13u1~bpo12+1"
"libz3-4" = "4.8.12-3.1"
"bzip2" = "1.0.8-5+b1"
"cmake" = "3.25.1-1"
"curl" = "7.88.1-10+deb12u15"
"git" = "1:2.39.5-0+deb12u3"
"make" = "4.3-4.1"
arm_gnu official sha256: 8f6903f8ceb084d9227b9ef991490413014d991874a1e34074443c2a72b14dbd
arm_gnu computed sha256: 8f6903f8ceb084d9227b9ef991490413014d991874a1e34074443c2a72b14dbd
arm_gnu pinned sha256:   8f6903f8ceb084d9227b9ef991490413014d991874a1e34074443c2a72b14dbd
klee_uclibc_locale computed sha256: c4362be318a38f18d98dccf462d22d95bab92f05548bb93f65298fe9afaebd57
klee_uclibc_locale pinned sha256:   c4362be318a38f18d98dccf462d22d95bab92f05548bb93f65298fe9afaebd57
base digest pinned:      sha256:7c7b2c966bc9ee8cedfeef67e0e279108992c77681fa595db4a9d65c06ccc587
base digest today:       sha256:7c7b2c966bc9ee8cedfeef67e0e279108992c77681fa595db4a9d65c06ccc587
```

`qemu-mprofile` notice of image-a job 121993148181 (RT1), verbatim. The second line is wrong because of a probe bug fixed afterwards (see Deviations, item 1); the exit code line is the gate:

```
qemu-arm version 10.0.2 (Debian 1:10.0.2+ds-2+deb13u1~bpo12+1)
cpu list: cortex-m4 NOT listed
exit code: 42
```

`check-a` notice of job 121993148181 (RT1), verbatim: exactly the four expected MISMATCH rows, every other row OK, `llvm_packages  16  16  16  OK`:

```
KEY  EXPECTED  ACTUAL  LLVM  STATUS
apt:bear  3.1.1-1  3.1.1-1  -  OK
apt:bzip2  1.0.8-5+b1  1.0.8-5+b1  -  OK
apt:clang-16  1:16.0.6-15~deb12u1  1:16.0.6-15~deb12u1  -  OK
apt:cmake  3.25.1-1  3.25.1-1  -  OK
apt:curl  7.88.1-10+deb12u15  7.88.1-10+deb12u15  -  OK
apt:git  1:2.39.5-0+deb12u3  1:2.39.5-0+deb12u3  -  OK
apt:libclang-16-dev  1:16.0.6-15~deb12u1  1:16.0.6-15~deb12u1  -  OK
apt:libclang-cpp16-dev  1:16.0.6-15~deb12u1  1:16.0.6-15~deb12u1  -  OK
apt:libz3-4  4.8.12-3.1  4.8.12-3.1  -  OK
apt:llvm-16  1:16.0.6-15~deb12u1  1:16.0.6-15~deb12u1  -  OK
apt:llvm-16-dev  1:16.0.6-15~deb12u1  1:16.0.6-15~deb12u1  -  OK
apt:make  4.3-4.1  4.3-4.1  -  OK
apt:qemu-system-arm  1:7.2+dfsg-7+deb12u18+b3  1:10.0.2+ds-2+deb13u1~bpo12+1  -  MISMATCH  version differs from the pin
apt:qemu-user  1:7.2+dfsg-7+deb12u18+b3  1:10.0.2+ds-2+deb13u1~bpo12+1  -  MISMATCH  version differs from the pin
arm_gcc  14.3.Rel1  14.3.Rel1  -  OK
bear  3.1.1  3.1.1  -  OK
bzip2  1.0.8  1.0.8  -  OK
c2rust  0.22.1  0.22.1  16  OK
cargo_mutants  27.1.0  27.1.0  -  OK
clang  16.0.6  16.0.6  16  OK
cmake  3.25.1  3.25.1  -  OK
curl  7.88.1  7.88.1  -  OK
git  2.39.5  2.39.5  -  OK
hayroll  -  -  -  not_installed  licence unconfirmed (risk R3); adapter gated in Phase 8
kani  -  -  -  not_installed  Kani evidence deferred to milestone v2.0; R2 spike in Phase 6
klee  3.2  3.2  16  OK
llvm_config  16.0.6  16.0.6  16  OK
llvm_packages  16  16  16  OK
make  4.3  4.3  -  OK
qemu_arm  1:7.2+dfsg-7+deb12u18+b3  1:10.0.2+ds-2+deb13u1~bpo12+1  -  MISMATCH  version differs from the pin
qemu_system_arm  1:7.2+dfsg-7+deb12u18+b3  1:10.0.2+ds-2+deb13u1~bpo12+1  -  MISMATCH  version differs from the pin
rust_default  1.99.0-x86_64-unknown-linux-gnu  1.99.0-x86_64-unknown-linux-gnu  -  OK
rust_targets_bitcode  x86_64-unknown-linux-gnu  x86_64-unknown-linux-gnu  -  OK
rust_targets_tool  thumbv7em-none-eabihf,x86_64-unknown-linux-gnu,x86_64-unknown-linux-musl  thumbv7em-none-eabihf,x86_64-unknown-linux-gnu,x86_64-unknown-linux-musl  -  OK
rustc_bitcode  1.72.1 d5c2e9c342b3 LLVM 16.0.5  1.72.1 d5c2e9c342b3 LLVM 16.0.5  16  OK
rustc_tool  1.99.0 b940084d7eb6  1.99.0 b940084d7eb6  23  OK
rustup  1.29.1  1.29.1  -  OK
llvm pin: 16 (provisional, decision D-15); bitcode rustc must report LLVM 16.x and at most 19; provisional until the Phase 6 R1 verdict
result: FAILED (4 rows)
exit code: 1
```

The `tool-outputs` notice (5 parts, joined in order) was saved to `.mt-work/tmp/03-09-rt1-tool-outputs.toml` (not committed) and became the committed fixture.

### RT2 - head `922c1169218eda72158427e7cb53b8e2bd6f44ff` (Task 2 and Task 3 commits)

| Workflow | Run id | Conclusion |
|----------|--------|------------|
| container | 40446062121 | success: mt 121997520612 success; image-a 121997755389 success; image-b 121997755396 success; compare 122000265472 success; publish 122000304950 skipped |
| pin-discovery | 40446062155 | success (job discover 121997520636) |
| ci | 40446062085 | success (job checks 121997520299) |
| testbed (not awaited, not evidence) | 40446062142 | failure at "Run the executor self-test in the pinned image" (still pulls the 7.2 digest under Current image) |

- check-a (image-a job 121997755389), the four QEMU rows, each `OK`: `apt:qemu-system-arm  1:10.0.2+ds-2+deb13u1~bpo12+1  1:10.0.2+ds-2+deb13u1~bpo12+1  -  OK`, `apt:qemu-user  <same>  <same>  -  OK`, `qemu_arm  <same>  <same>  -  OK`, `qemu_system_arm  <same>  <same>  -  OK`; `llvm_packages  16  16  16  OK`; `result: OK`, `exit code: 0`.
- `qemu-mprofile` notice (RT2), verbatim:

```
qemu-arm version 10.0.2 (Debian 1:10.0.2+ds-2+deb13u1~bpo12+1)
cpu list: cortex-m4 listed
exit code: 42
```

- `manifest-compare` notice, verbatim: `manifest-a.json and manifest-b.json are byte-identical; sha256 9f382c31c09adfc0a19ebb247b8fb7d51ea013f44255c048f99036cb82257b68`.
- Negative pin (`negative-pin` notice, level notice): LLVM major 16 -> 17; `exit code: 1 (required: exactly 1, with MISMATCH on klee and rustc_bitcode)`; `klee  3.2  3.2  16  MISMATCH  LLVM major 16 differs from the pinned 17`, `rustc_bitcode ... 16  MISMATCH ...`, `llvm_packages  17  16  16  MISMATCH`, `result: FAILED (11 rows)` (the MISSING rows are the pre-existing off-pin behavior of renaming the clang and llvm packages; the QEMU rows stayed OK).
- `tool-outputs` of RT2 (saved as `.mt-work/tmp/03-09-rt2-tool-outputs.toml`, not committed) equals the committed fixture apart from the run id in the `source` labels and the ASLR load addresses in `c2rust_ldd` (see Deviations 4).
- pin-discovery RT2 reports `"qemu-system-arm" = "1:10.0.2+ds-2+deb13u1~bpo12+1"` and `"qemu-user" = "1:10.0.2+ds-2+deb13u1~bpo12+1"` and the `qemu-backports` notice.

## dpkg capture: packages changed, added or removed (RT1 capture against the previous fixture)

- **Changed version** (all qemu-source packages, as the plan allows): `qemu-system-arm`, `qemu-system-common`, `qemu-system-data` (from 7.2 versions) and `qemu-user` to `1:10.0.2+ds-2+deb13u1~bpo12+1`.
- **Added:** none.
- **Removed** (installed with QEMU 7.2 only, no longer needed by 10.0.2): `iso-codes`, `libcap2-bin`, `libdrm-common`, `libdrm2`, `libdw1`, `libepoxy0`, `libgbm1`, `libgstreamer-plugins-base1.0-0`, `libgstreamer1.0-0`, `libopus0`, `liborc-0.4-0`, `libsndio7.0`, `libspice-server1`, `libunwind8`, `libwayland-server0`.
- No non-QEMU package changed version; every other table of the capture (apart from `c2rust_ldd` addresses) is identical to the previous fixture.

## Files Created/Modified

- `container/Dockerfile` - fourth snapshot source `${VERSION_CODENAME}-backports main`, `/etc/apt/preferences.d/50-qemu-backports`, stage comment
- `container/smoke/qemu-mprofile.S` - Thumb guest, `exit_group(42)` through raw `svc #0`, no semihosting
- `.github/workflows/container.yml` - header titles, image-a step `QEMU M-profile user-mode start (D-31)` (id `qemu-mprofile`)
- `.github/workflows/pin-discovery.yml` - header, step `List qemu-user versions seen in backports (informational, D-31)`
- `crates/mt-toolchain/tests/container_static.rs` - the three `eqv_04_*` tests and small helpers (`stage_names`, `quoted_words`, `workflow_step_text`); no existing annotation-title enumeration needed a change
- `container/pins.toml`, `crates/mt-toolchain/tests/fixtures/pins-full.toml` - the four QEMU values; pins.toml comments (provenance, QEMU rationale)
- `crates/mt-toolchain/tests/fixtures/tool-outputs.toml` - whole body replaced by the RT1 CI capture (header run id updated)
- `crates/mt-toolchain/tests/check_and_manifest.rs`, `crates/mt-cli/tests/tool_01_check_end_to_end.rs` - tests expect the new version
- `container/README.md` - own QEMU row, bumping-pins step 2, Notices entry; `.planning/PROJECT.md` - D-21
- `.planning/phases/03-equivalence-contract-and-test-vector-schema/03-01-PLAN.md` - the two hand-back edits (6 added lines since 90fb133, the sixth being the planning-time Task 3 precondition)

## Decisions Made

See key-decisions. All discretion items of the plan context were followed as written (source, selection by preferences, qemu-system-arm moves too, pinned set of four, proof before publish by the probe, version rule, fixtures from RT1).

## Deviations from Plan

### Version is not 8.2.x (version rule, recorded deviation from D-31)

V = `1:10.0.2+ds-2+deb13u1~bpo12+1`, see "Version decision". Surfaced for the founder via plan 03-10.

### Auto-fixed Issues

**1. [Rule 1 - Bug] Probe reported the cortex-m4 CPU list wrongly**
- **Found during:** Task 3, reading RT1's `qemu-mprofile` notice (`cpu list: cortex-m4 NOT listed` although the guest ran)
- **Issue:** `qemu-arm -cpu help` exits 1 (also on the host QEMU 8.2.2), so `qemu-arm -cpu help | grep -qw cortex-m4` under `set -o pipefail` was false. The informational line was wrong; the exit-42 gate was unaffected.
- **Fix:** capture the list first (`cpus=$(qemu-arm -cpu help 2>&1 || true)`) and grep the variable.
- **Files modified:** `.github/workflows/container.yml`
- **Verification:** RT2 notice reads `cpu list: cortex-m4 listed`, `exit code: 42`.
- **Committed in:** `7177ce4`

### Other deviations (plan text versus what happened)

**2. RT1 image-a and image-b both failed, not only image-a.** The plan expected image-a failure only. `Manifest of build A` and `Manifest of build B` fail with the same four-row `FAILED (4 rows)` because the manifest step refuses to build while check rows mismatch. Same cause as the expected failure; no action; RT2 is green.

**3. Dpkg capture shows 15 removed packages.** The plan's handling covered changed and added packages only. The removed ones are dependencies only QEMU 7.2 needed (listed above); no non-QEMU version changed, so no stop condition applied.

**4. `c2rust_ldd` of RT2 differs from the committed fixture in ASLR load addresses.** The pre-decided handling ("any difference beyond the run id: replace the fixture and spend RT3") would fire on address noise that differs in every capture. After masking `(0x...)` addresses, RT2 equals the committed fixture table for table. RT3 was not spent.

**5. Plan verify commands compare run `head_sha` with the last commit touching the trigger paths.** The pushed head of RT2 is the docs commit `922c116`, which touches none of those paths, so the container, ci and pin-discovery runs carry `head_sha` 922c116 while the commands expect 7177ce4. The equivalent check against the pushed HEAD passed (container success for 922c116, pin-discovery success 922c116, ci success 922c116); the other Task 3 CI verifications passed as written.

**6. Task 2's `git grep` gate for `7.2+dfsg` could only pass after Task 3.** `container/README.md` still named the old Debian package until the Task 3 docs commit; after it the set is exactly `check_and_manifest.rs` and `pins_and_versions.rs`.

**Total deviations:** 1 recorded version deviation, 1 auto-fixed (Rule 1), 4 plan-text mismatches handled without extra round trips.
**Impact on plan:** none on scope; D-30, D-02 and `crates/mt-harness` untouched, `Cargo.lock` unchanged.

## Issues Encountered

None beyond the deviations above. The tracking commit f79b836 (orchestrator STATE.md) was pushed with RT1 as expected. Commit trailers follow the attribution lines supplied by the harness for this session.

## Threat Surface

Threat register T-03-32..T-03-36 mitigations are in place (static test for the preferences and no insecure option; probe runs `--network none --read-only --user 65534:65534` with a capped tmpfs; discovery keeps `contents: read`; README names the image copyright file as licence authority; `timeout 60` and `curl --max-time 60`). No new threat flags.

## Known Stubs

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- **Plan 03-10** opens the image PR from head `922c116`: it must show the version deviation (10.0.2, not 8.2.x) and the `qemu-backports` list to the founder before the merge, then publish, merge back, record the new digest under `## Current image` and re-baseline testbed.
- **03-01 Task 3** resumes with round trip 3 on the new image. Heads-up for its self-test: `qemu-arm -cpu help` exits 1 in the image (so never use its exit status or a pipefail pipeline to test the CPU list), the banner now reads `qemu-arm version 10.0.2 (Debian 1:10.0.2+ds-2+deb13u1~bpo12+1)` and `Copyright (c) 2003-2025`, and 03-01-PLAN.md still has prose mentions of "QEMU 7.2" (lines 101, 103, 380, 415) that this plan was not allowed to edit.
- Testbed on the session branch keeps failing its executor self-test step until 03-10 publishes the new digest; that is the known state, not a regression.

## Self-Check: PASSED

- All created and modified files exist (13 files checked with `[ -f ]`).
- Commits `260610f`, `9770902`, `0aed190`, `58d465c`, `7177ce4`, `922c116` are ancestors of HEAD.
- `git diff 90fb133 --name-only -- crates/mt-harness Cargo.lock` is empty; no forbidden claim words in README, probe or workflows.
- Pre-merge gates (`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check`) passed before each push; RT2 container, pin-discovery and ci runs are success for head 922c116 with publish skipped.

---
*Phase: 03-equivalence-contract-and-test-vector-schema*
*Completed: 2026-10-27*
