---
phase: 01-pinned-toolchain-container
plan: 02
subsystem: infra
tags: [c2rust, crates-io, package-legitimacy, ci, github-actions, ghcr]

# Dependency graph
requires: []
provides:
  - "Founder verdict on c2rust 0.22.1 package legitimacy (approved), with crates.io evidence"
  - "CI push mode for the remaining Phase 1 CI round trips (executor-pushes), with its scope"
affects: [01-06, 01-07, 01-08]

# Actuals (#2632)
actuals:
  tokens: 1500
  tasks: 2
  commits: 0
plan_head_before: 7418bedf86921a17372ff3dc711829e8f64edcc4
plan_head_after: 7418bedf86921a17372ff3dc711829e8f64edcc4

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Founder-gated decisions recorded as grep-able key: value lines in a SUMMARY, read by later plans"

key-files:
  created:
    - .planning/phases/01-pinned-toolchain-container/01-02-SUMMARY.md
  modified: []

key-decisions:
  - "c2rust 0.22.1 is approved for installation: owners and repository are the Immunant project and the .crate sha256 matches the crates.io checksum"
  - "The executor may push the current phase branch (and only that branch) for CI round trips: no force-push, never main, no other branches"

requirements-completed: [TOOL-01, TOOL-03]

coverage:
  - id: D1
    description: "Recorded founder verdict on c2rust 0.22.1 legitimacy (approved) with the three read-only evidence outputs"
    requirement: "TOOL-01"
    verification: []
    human_judgment: true
    rationale: "Package legitimacy is a blocking-human decision; automation can only show the evidence, the verdict is the founder's"
  - id: D2
    description: "Recorded CI push mode (executor-pushes) and its scope for plans 01-06, 01-07 and 01-08"
    requirement: "TOOL-03"
    verification: []
    human_judgment: true
    rationale: "Consent to push to origin is a blocking-human decision made by the founder"

# Metrics
duration: 5min
completed: 2026-10-09
status: complete
---

# Phase 1 Plan 02: Founder Gates Summary

**Founder-approved c2rust 0.22.1 package legitimacy (Immunant owners, repository and .crate sha256 verified) and executor-pushes CI mode, scoped to the phase branch only.**

c2rust_legitimacy: approved
ci_push_mode: executor-pushes

## Performance

- **Duration:** 5 min (continuation after founder answers)
- **Completed:** 2026-10-09
- **Tasks:** 2 of 2 (both blocking-human checkpoints, answered by the founder)
- **Files modified:** 1 (this SUMMARY only; no code commits)

## Accomplishments

- Task 1 (package legitimacy gate): the founder answered "approved" for crate c2rust 0.22.1. Plans 01-07 and 01-08 may install c2rust.
- Task 2 (CI push mode): the founder answered "executor-pushes". Plans 01-06, 01-07 and 01-08 read `ci_push_mode` before pushing.

## Task 1 evidence: c2rust 0.22.1 legitimacy

Captured read-only by the previous executor on 2026-10-09 and recorded verbatim. The founder approved on the basis of this evidence.

Step 1: `curl -fsS -A migrationtool-phase1-check https://crates.io/api/v1/crates/c2rust`

- repository: https://github.com/immunant/c2rust/
- homepage: https://c2rust.com/
- max_version: 0.22.1
- downloads: 68855
- created_at: 2018-10-15T21:36:22.762639Z
- version 0.22.1: yanked=False, checksum=e331f411600da65f3ed67033048cf1fcc203d7655974cf5bd6410da8bc653f3b, created_at=2026-02-11T09:35:53.752839Z, published_by=kkysen

Step 2: `curl -fsS -A migrationtool-phase1-check https://crates.io/api/v1/crates/c2rust/owners`

- ahomescu (https://github.com/ahomescu)
- thedataking (Per Larsen)
- rinon (Stephen Crane)
- kkysen (Khyber Sen)
- fw-immunant (Frances Wingerter)

Step 3: `curl -fsSL https://static.crates.io/crates/c2rust/c2rust-0.22.1.crate | sha256sum`

```
e331f411600da65f3ed67033048cf1fcc203d7655974cf5bd6410da8bc653f3b  -
```

This matches both the plan's expected digest and the crates.io checksum in Step 1.

Step 4 (optional, open https://github.com/immunant/c2rust and confirm tag v0.22.1): not verified. api.github.com returned HTTP 403 from the cloud session. The founder approved without it.

Founder verdict: `approved` (no further reason given).

## Task 2: CI push mode

Founder choice: `executor-pushes`.

Scope of this permission, as recorded for this session:

- Applies only to the phase branch. In this session that branch is `claude/gsd-discuss-phase-1-mzdw5v` (the session is bound to it; no `gsd/phase-01-*` branch is used).
- No force-push (no `--force`, no `-f`, no `--force-with-lease`).
- Never `main` (or any protected/default branch).
- No other branches.
- The choice only affects who runs `git push`; it never changes what CI must show.

## Task Commits

No task commits: both tasks were founder answers to blocking-human checkpoints and changed no code. The only commit for this plan is the SUMMARY commit.

## Files Created/Modified

- `.planning/phases/01-pinned-toolchain-container/01-02-SUMMARY.md` - recorded `c2rust_legitimacy` and `ci_push_mode` with evidence

## Decisions Made

- c2rust 0.22.1 approved by the founder (mitigates T-01-06 and T-01-SC for c2rust). The sha256 is still to be pinned in `container/pins.toml` and checked with `sha256sum -c` in the image build.
- executor-pushes approved with the scope above (mitigates T-01-07: push only after founder choice, current phase branch only, never force-push, never main).

## Deviations from Plan

None - plan executed exactly as written. The only session-specific adjustment is that the phase branch name is `claude/gsd-discuss-phase-1-mzdw5v`, as recorded under Task 2.

## Authentication Gates

None. Step 4 of Task 1 hit HTTP 403 from api.github.com in the cloud session; this was an optional step and the founder approved without it.

## Issues Encountered

None. Optional verification step 4 (GitHub repository and tag v0.22.1 check) was not completed because of the 403 noted above.

## Known Stubs

None.

## Threat Flags

None - no code or files introducing new trust-boundary surface.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plan 01-06, 01-07 and 01-08 CI round-trip steps can read `ci_push_mode: executor-pushes` and push the phase branch themselves, within the scope above.
- Plan 01-07 may install c2rust (`c2rust_legitimacy: approved`); the sha256 `e331f411600da65f3ed67033048cf1fcc203d7655974cf5bd6410da8bc653f3b` should be pinned and verified in the image build.
- No blockers from this plan.

## Self-Check: PASSED

- SUMMARY file exists at the expected path.
- `c2rust_legitimacy: approved` and `ci_push_mode: executor-pushes` are present as literal lines.
- No code commits were planned or made (`commits: 0`, HEAD unchanged at 7418bedf86921a17372ff3dc711829e8f64edcc4 before the SUMMARY commit).

---
*Phase: 01-pinned-toolchain-container*
*Completed: 2026-10-09*
