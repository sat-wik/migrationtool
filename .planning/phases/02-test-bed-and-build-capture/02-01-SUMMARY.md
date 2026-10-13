---
phase: 02-test-bed-and-build-capture
plan: 01
subsystem: infra
tags: [container, pins, debian-snapshot, git, cmake, curl, bzip2, make, github-actions]

requires:
  - phase: 01-pinned-toolchain-container
    provides: pins.toml/Dockerfile/mt toolchain check pipeline, container and pin-discovery workflows, ci_push_mode executor-pushes
provides:
  - "[apt] pins and [tool.*] rows for bzip2, cmake, curl, git and make at snapshot 20261005T000000Z"
  - "apt-base install list with bzip2 cmake curl git next to make; REQUIRED_TOOLS with 15 keys"
  - "CI-captured tool outputs for every installed tool (container run 38337265570)"
  - "PROJECT.md decision D-17"
  - "Extended image definition proven green on the session branch, ready for the PR in plan 02-02"
affects: [02-02, 02-03, 02-06, 02-09, testbed fetch, testbed CMake builds, Mbed TLS archive source]

actuals:
  tokens: 7700
  tasks: 3
  commits: 4
plan_head_before: ac51f20434196c66a7306da0cd18403f2e275d05
plan_head_after: 8bc2b24f4bc3360e100d1f90a386eba0a0c4d488

tech-stack:
  added: []
  patterns:
    - "New image tool = pin-discovery list + [apt] pin + [tool.*] row + REQUIRED_TOOLS key + Dockerfile line + fixtures, all in one change"
    - "Fixtures marked with an honest temporary source label until a CI capture replaces them"

key-files:
  created: []
  modified:
    - .github/workflows/pin-discovery.yml
    - container/pins.toml
    - container/Dockerfile
    - container/README.md
    - crates/mt-toolchain/src/pins.rs
    - crates/mt-toolchain/tests/container_static.rs
    - crates/mt-toolchain/tests/check_and_manifest.rs
    - crates/mt-toolchain/tests/pins_and_versions.rs
    - crates/mt-toolchain/tests/fixtures/pins-full.toml
    - crates/mt-toolchain/tests/fixtures/tool-outputs.toml
    - crates/mt-cli/tests/tool_01_check_end_to_end.rs
    - .planning/PROJECT.md

key-decisions:
  - "D-17 (PROJECT.md): final image stage gains git, cmake, curl, bzip2; make gains an [apt] pin; each has a [tool.*] row; only a push to main publishes"
  - "bzip2 is probed with --help (banner on stderr); the other four with --version; expect equals upstream_version of the apt pin"

patterns-established:
  - "Pin discovery first on its own commit (RT1), then pins/Dockerfile/fixtures together (RT2), then CI-captured fixtures (RT3)"

requirements-completed: [BUILD-01, BED-01]

coverage:
  - id: D1
    description: "bzip2, cmake, curl, git and make are discovered at the pinned snapshot and pinned in [apt] with values copied from the pin-discovery annotation"
    requirement: BUILD-01
    verification:
      - kind: integration
        ref: "pin-discovery run 38337265558 (annotation pin-discovery) and tool_03_discovery_package_list_matches_apt_pins"
        status: pass
    human_judgment: false
  - id: D2
    description: "The extended image builds, mt toolchain check shows OK for the five tools and their dpkg rows in image-a, the off-pin check still exits 1, and both builds are manifest-identical"
    requirement: BED-01
    verification:
      - kind: integration
        ref: "container run 38338543797 (jobs mt, image-a, image-b, compare success; publish skipped)"
        status: pass
    human_judgment: false
  - id: D3
    description: "REQUIRED_TOOLS lists 15 sorted keys and every fixture, name list and fixed count agrees; tool-outputs.toml holds CI captures only"
    requirement: BUILD-01
    verification:
      - kind: unit
        ref: "cargo test --workspace --locked (tool_02_container_pins_regexes_parse_captured_outputs, tool_03_dockerfile_installs_every_apt_pin, tool_01_check_end_to_end_reports_ok_and_exits_zero)"
        status: pass
      - kind: integration
        ref: "ci run 38338543830 (job checks success)"
        status: pass
    human_judgment: false
  - id: D4
    description: "README contents row and Notices (GPL-2.0-only git, GPL-3.0-or-later make, BSD-3-Clause cmake, curl licence, bzip2-1.0.6) name the Debian copyright file as the authority"
    verification: []
    human_judgment: true
    rationale: "Licence names are the ones the Debian packages declare and were not read from the image in this session; a human should confirm them against /usr/share/doc/<package>/copyright before any redistribution"

duration: 34min
completed: 2026-10-13
status: complete
---

# Phase 2 Plan 01: Extend the pinned image with git, cmake, curl, bzip2 and pin make Summary

**The pinned image now carries git, cmake, curl and bzip2 and an `[apt]` pin for make, all from the snapshot archive at versions copied from a pin-discovery run, proven green on the session branch (container, ci, pin-discovery) with CI-captured tool outputs backing the parser tests.**

## Performance

- **Duration:** 34 min (start approximated from the first commit, 02:24Z)
- **Started:** 2026-10-13T02:24:00Z
- **Completed:** 2026-10-13T02:58:00Z
- **Tasks:** 3
- **Files modified:** 12

## Accomplishments

- Pin discovery resolved the five packages at snapshot 20261005T000000Z; pins, tool rows, Dockerfile list and `REQUIRED_TOOLS: [&str; 15]` describe the extended image consistently.
- Every Phase 1 fixture, name list and fixed count agrees with the 15-tool set (observed rows 27 to 37, installed tools checked 10 to 15); the fake container in the CLI end-to-end test gained the five scripts.
- The extended image built twice manifest-identical in CI, `mt toolchain check` passed with OK rows for the five tools and their dpkg rows, the off-pin check still exits 1, and `tool-outputs.toml` now holds only CI captures.
- PROJECT.md decision D-17 records the extension; the README lists the new tools and licences.

## Discovered `[apt]` lines (pin-discovery run 38337265558, commit c9b3c74)

```
"bzip2" = "1.0.8-5+b1"
"cmake" = "3.25.1-1"
"curl" = "7.88.1-10+deb12u15"
"git" = "1:2.39.5-0+deb12u3"
"make" = "4.3-4.1"
```

## CI evidence

Round trips used (cap 3 each): **Task 1 used 1 (RT1)**, **Task 3 used 2 (RT2, RT3)**. No budget extension, no escalation.

| Round trip | Head SHA | Runs |
|---|---|---|
| RT1 | c9b3c745686b96f03708fcb60c00e2f56865cb9b | pin-discovery 38336910397 success (job discover 115098955416); ci 38336910382 failure (job checks 115098955384), the one planned failure |
| RT2 | c3c861648b627df88c5337425fff34e7adb6a7f6 | container 38337265570: mt 115100058668, image-a 115100221489, image-b 115100221515, compare 115103674454 success, publish 115103712958 skipped; pin-discovery 38337265558 success; ci 38337265551 failure (the one planned failure) |
| RT3 (final) | 8bc2b24f4bc3360e100d1f90a386eba0a0c4d488 | container 38338543797: mt 115104079216, image-a 115104271835, image-b 115104271740, compare 115106578117 success, publish 115106611244 skipped; ci 38338543830: checks 115104079037 success |

- **pin-discovery on the final commit:** the final commit touches none of its trigger paths (`container/pins.toml`, `container/Dockerfile`, the workflow); `git diff c3c8616 HEAD -- container/pins.toml container/Dockerfile .github` is empty, so the latest pin-discovery run (38337265558, head c3c8616, job discover success) covers the same pin inputs. No new pin-discovery run exists for 8bc2b24 and none was dispatched (the push scope allows no other ref and no manual dispatch was authorised).
- **check-a (RT3 image-a, result OK, exit 0), the five tools:**

```
apt:bzip2  1.0.8-5+b1  1.0.8-5+b1  -  OK
apt:cmake  3.25.1-1  3.25.1-1  -  OK
apt:curl  7.88.1-10+deb12u15  7.88.1-10+deb12u15  -  OK
apt:git  1:2.39.5-0+deb12u3  1:2.39.5-0+deb12u3  -  OK
apt:make  4.3-4.1  4.3-4.1  -  OK
bzip2  1.0.8  1.0.8  -  OK
cmake  3.25.1  3.25.1  -  OK
curl  7.88.1  7.88.1  -  OK
git  2.39.5  2.39.5  -  OK
make  4.3  4.3  -  OK
llvm_packages  16  16  16  OK
```

- **negative-pin (RT3):** `result: FAILED (11 rows)`, `exit code: 1 (required: exactly 1, with MISMATCH on klee and rustc_bitcode)`; the new tools stay OK, only the LLVM rows mismatch.
- **manifest-compare:** `manifest-a.json and manifest-b.json are byte-identical; sha256 1a405da68aca4c97fba29acb57186403b022c6859ed22c45cb56e019bb59ae75`.
- **publish:** the publish job was skipped at job level (`if:` push to `refs/heads/main`), so it ran zero steps, wrote nothing to GHCR and emitted no `publish-skipped` annotation (that annotation belongs to the tag-already-exists path inside a running publish job). The branch-built image is not recorded as the current image; README "Current image" is unchanged.
- bzip2 printed its banner on stderr for `--help` as predicted, so no regex or argument fallback (A5-style) was needed; no LLVM-family dependency appeared (`llvm_packages` stays 16 OK); every package had a candidate (no A1 stop).

## Task Commits

1. **Task 1: Pin discovery round trip, then the apt pins, tool rows, Dockerfile list and required-tool set** - `c9b3c74` (ci, discovery list, RT1) and `a524503` (feat, pins, Dockerfile, REQUIRED_TOOLS, D-17)
2. **Task 2: Fixtures and fixture-driven tests know the five new tools** - `c3c8616` (test)
3. **Task 3: Container round trips, captured outputs, README** - `8bc2b24` (test)

**Plan metadata:** recorded in the docs commit that follows this file.

## Files Created/Modified

- `.github/workflows/pin-discovery.yml` - packages line gains bzip2 cmake curl git make
- `container/pins.toml` - five `[apt]` pins and five `[tool.*]` rows, comments cite phase 02 D-30
- `container/Dockerfile` - apt-base list gains bzip2 cmake curl git
- `container/README.md` - contents row and Notices for the five tools
- `crates/mt-toolchain/src/pins.rs` - `REQUIRED_TOOLS` has 15 sorted keys
- `crates/mt-toolchain/tests/container_static.rs` - apt-derived expects for the five tools
- `crates/mt-toolchain/tests/check_and_manifest.rs`, `pins_and_versions.rs` - tool-name lists and counts (rows 27 to 37, tools 10 to 15)
- `crates/mt-toolchain/tests/fixtures/pins-full.toml`, `tool-outputs.toml` - five tools; outputs are CI captures from run 38337265570
- `crates/mt-cli/tests/tool_01_check_end_to_end.rs` - fake scripts and dpkg rows for the five tools
- `.planning/PROJECT.md` - decision D-17

## Decisions Made

- D-17 added as planned (phase 02 CONTEXT D-30); supersedes nothing.
- Kept `expect` equal to `upstream_version` of each apt pin (bzip2 1.0.8, cmake 3.25.1, curl 7.88.1, git 2.39.5, make 4.3), asserted in container_static.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Header comment of tool-outputs.toml reworded**
- **Found during:** Task 3 (acceptance criterion `grep -c 'representative' ... prints 0`)
- **Issue:** the Phase 1 header comment contains the quoted word "representative" (describing what the test refuses), so the criterion would print 1 although no source label says it.
- **Fix:** header now says the test "refuses any source that is not a CI capture"; the test itself is unchanged and nothing parses the header.
- **Files modified:** crates/mt-toolchain/tests/fixtures/tool-outputs.toml
- **Verification:** `grep -c representative` prints 0; full `cargo test --workspace --locked` passes.
- **Committed in:** 8bc2b24

### Process notes (not deviations)

- Pushes used `git push -u origin claude/gsd-discuss-phase-1-mzdw5v` instead of the plan's `git push origin HEAD`, per the dispatch instruction; no force, no other ref.
- The ci failure annotation only says "exit code 101". That RT1 and RT2 failed on exactly the named test was established by reproducing the same commit state locally with `cargo test --workspace --locked --no-fail-fast` (one failure each time, the named test), not from the CI log, which is unreachable from cloud sessions.
- Requirements BUILD-01 and BED-01 are shared with later plans of this phase and were not marked complete here (the orchestrator owns requirement status).

---

**Total deviations:** 1 auto-fixed (1 bug-class wording fix to satisfy a stated acceptance criterion)
**Impact on plan:** None on behavior; no check, pin, test, `--locked` or workflow step was weakened and `.github/workflows/container.yml` was not edited.

## Issues Encountered

None. Local gates before every push: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace --locked` (named skip where the plan says so; full run before RT3), `cargo deny check` all exited 0.

## Known Stubs

None.

## Threat Flags

None. T-02-01 (versions only from the pin-discovery annotation, dpkg rows re-checked in image-a), T-02-02 (no workflow other than pin-discovery's package list changed; pushes to the session branch only) and T-02-03 (README Notices) are mitigated as planned.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Ready for plan 02-02: ship this change to `main` through a PR (only a push to main publishes, WR-09), then update README "Current image".
- The licence names for git, make, cmake, curl and bzip2 in the README are the Debian-declared ones and should be confirmed against the image's copyright files before any redistribution.

---
*Phase: 02-test-bed-and-build-capture*
*Completed: 2026-10-13*
