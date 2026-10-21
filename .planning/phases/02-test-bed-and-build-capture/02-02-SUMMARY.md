---
phase: 02-test-bed-and-build-capture
plan: 02
subsystem: infra
tags: [container, ghcr, pull-request, publish, merge-back, github-actions]

requires:
  - phase: 02-test-bed-and-build-capture
    provides: "plan 02-01: extended image definition (git, cmake, curl, bzip2, make pin) proven green on the session branch"
provides:
  - "Pull request #4 (session branch to main) merged by the founder with a merge commit, 69b5b7f4"
  - "Extended image published from main by container run 38342425433 (publish job success)"
  - "container/README.md `## Current image` naming the new digest as the only pullable reference (D-10)"
  - "origin/main merged back into the session branch by fast-forward"
affects: [02-06, 02-08, 02-09, testbed workflow, testbed CMake builds]

actuals:
  tokens: 1400
  tasks: 3
  commits: 2
plan_head_before: 04fcb5885588b64f391c133b25b80ecee5273845
plan_head_after: 7b31ebe036c321d5ec86f6f2ea62e1bb9e7702ee

tech-stack:
  added: []
  patterns:
    - "Image digest is recorded only from the verified-image notice of the publish job of a push-to-main run whose head_sha equals origin/main"
    - "Merge-back of main into the session branch is a merge (here a fast-forward), never a rebase"

key-files:
  created: []
  modified:
    - container/README.md

key-decisions:
  - "Current image is sha256:e83006c0a7e37f788afcbd1c95554e8c4dfc61c3b68da3ff964caf47fb3851b9, published by container run 38342425433 (push to main, merge commit 69b5b7f4, PR #4); the previous digest sha256:42c7fb0e... (run 38202499776) is mentioned as superseded without the image name"
  - "02-CONTEXT.md D-10 is unchanged: it already points at `## Current image` for a later image"

requirements-completed: [BUILD-01, BUILD-02]

coverage:
  - id: D1
    description: "Pull request #4 from the session branch to main carried the image extension; its container and ci checks were green and publish was skipped on the PR run"
    requirement: BUILD-01
    verification:
      - kind: integration
        ref: "gh api repos/sat-wik/migrationtool/actions/runs 38339937976 (container, pull_request, success) and 38339937977 (ci, success)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The merge of PR #4 to main produced a container run whose publish job succeeded and announced the verified digest"
    requirement: BUILD-02
    verification:
      - kind: integration
        ref: "gh api repos/sat-wik/migrationtool/actions/workflows/container.yml/runs?branch=main&event=push&per_page=1 (38342425433, head_sha equals origin/main, success; publish job 115118867102 success)"
        status: pass
    human_judgment: false
  - id: D3
    description: "container/README.md `## Current image` names the new digest, run id, merge commit, PR number, archive sha256, loaded image id and manifest sha256, and is the only pullable digest reference in the README"
    requirement: BUILD-02
    verification:
      - kind: unit
        ref: "cargo test -p mt-toolchain --test container_static (30 passed, including tool_03_container_readme_references_the_image_by_digest_only)"
        status: pass
      - kind: other
        ref: "test \"$(grep -oE 'ghcr.io/sat-wik/migrationtool-toolchain@sha256:[0-9a-f]{64}' container/README.md | sort -u | wc -l)\" -eq 1"
        status: pass
    human_judgment: false
  - id: D4
    description: "The merge commit the founder created and the PR contents match the commits listed in the PR body"
    verification: []
    human_judgment: true
    rationale: "The founder reviewed and merged the PR by hand (T-02-04); the SUMMARY records only that the remote head did not move between PR creation and merge"

duration: 10min (Task 3 only; Task 1 and Task 2 spanned the PR check wait and the founder merge)
completed: 2026-10-13
status: complete
---

# Phase 2 Plan 02: Ship the extended image to main and record its digest Summary

**The extended image (git, cmake, curl, bzip2, pinned make) reached main through PR #4, was published by container run 38342425433, and `container/README.md` now names `sha256:e83006c0...` as the only pullable reference, with origin/main merged back by fast-forward.**

## Performance

- **Duration:** Task 3 about 10 min; Tasks 1 and 2 are dominated by the PR check wait and the founder's merge, so no single wall-clock figure is recorded for the plan
- **Started:** Task 3 at 2026-10-13T04:10Z (approximate); Task 1 earlier in the same session
- **Completed:** 2026-10-13T04:31Z
- **Tasks:** 3 (Task 1 auto, Task 2 checkpoint:human-action, Task 3 auto)
- **Files modified:** 1 (`container/README.md`)

## Accomplishments

- PR #4 was opened from `claude/gsd-discuss-phase-1-mzdw5v` to `main` and its checks went green; the founder merged it with a merge commit.
- Main's container run for the merge commit published the image; the verified digest, archive sha256, loaded image id, manifest sha256 and tag were copied verbatim from the publish and image-a annotations into `## Current image`.
- `origin/main` was merged back into the session branch (fast-forward, no rebase entry in the reflog) and the README commit was pushed.

## CI and PR evidence

Round trips used: **0**, plus one PR-check wait (Task 1). No push occurred between PR creation and merge; the remote head stayed 04fcb5885588b64f391c133b25b80ecee5273845 throughout.

- **PR:** #4, https://github.com/sat-wik/migrationtool/pull/4, head `claude/gsd-discuss-phase-1-mzdw5v` at 04fcb5885588b64f391c133b25b80ecee5273845, base `main` at 67467607. Merged; merge commit 69b5b7f4fae92d0c3ec99d4e798609c0606b4213 equals origin/main.
- **PR checks:**
  - container run 38339937976 (pull_request) success: jobs mt 115108421245, image-a 115108618084, image-b 115108618043, compare 115110593349 success; publish 115110620565 skipped (WR-09).
  - ci run 38339937977 success (job checks 115108420715).
- **Main push run:** container run 38342425433, head_sha 69b5b7f4fae92d0c3ec99d4e798609c0606b4213, success. Jobs: mt 115116179367, image-a 115116379880, image-b 115116379894, compare 115118824034, publish 115118867102, all success.
- **Annotations, publish job 115118867102 (verbatim):**
  - `verified-image`: `ghcr.io/sat-wik/migrationtool-toolchain@sha256:e83006c0a7e37f788afcbd1c95554e8c4dfc61c3b68da3ff964caf47fb3851b9`, manifest sha256 `1a405da68aca4c97fba29acb57186403b022c6859ed22c45cb56e019bb59ae75`
  - `image-digest`: same digest, pushed as tag `sha-69b5b7f4fae92d0c3ec99d4e798609c0606b4213`, after image-a, image-b and compare succeeded
  - `image-archive`: archive sha256 `d2b5c1fac92d6771d1d49b9409be4d864431c887fed783cb80e8fd34259814f9` checked before docker load; loaded image id `sha256:1267a6230e171d711a7d056cd7a129fbdf0753339c92af22dac69afc806eac44` equals the id image-a gated
- **Annotations, image-a job 115116379880:** `image-archive` image id `sha256:1267a6230e171d711a7d056cd7a129fbdf0753339c92af22dac69afc806eac44`, archive sha256 `d2b5c1fac92d6771d1d49b9409be4d864431c887fed783cb80e8fd34259814f9`, matching the publish job.
- **Annotation, compare job 115118824034:** `manifest-compare`: manifest-a.json and manifest-b.json are byte-identical; sha256 `1a405da68aca4c97fba29acb57186403b022c6859ed22c45cb56e019bb59ae75`.
- The manifest sha256 equals the one recorded for the branch run in 02-01 (container run 38338543797), as expected for the same image definition.
- **Superseded image:** `sha256:42c7fb0e00e90e946b5a21e598517fb7189a0de5492fa1d4d6f18e2e9c39e13d` (run 38202499776), now mentioned in the README without the image name.

## Task Commits

1. **Task 1: Open the image PR and wait for its checks** - no commit (PR #4 on GitHub)
2. **Task 2: Founder merges the PR** - no commit (checkpoint:human-action; the founder replied "merged", the orchestrator confirmed the merge and the main run through gh api)
3. **Task 3: Confirm the publish, merge origin/main back, record the new digest** - `69b5b7f4` (merge of PR #4, brought in by fast-forward) and `7b31ebe` (docs: `docs(02-02): record the extended main-published image under Current image`)

**Plan metadata:** recorded in the docs commit that follows this file.

## Files Created/Modified

- `container/README.md` - `## Current image` rewritten for run 38342425433: new digest in the fenced block, run id, merge commit, PR #4, archive sha256, loaded image id, manifest sha256, tag written alone, one sentence on what the image adds, the superseded digest in bare form, an updated founder-check paragraph and WR-09 paragraph.

## Decisions Made

- The README founder-check paragraph keeps the Phase 1 UAT tied to the branch-built `sha256:0f5a5d25...` image and says it has not been repeated on the current image. Because this image's manifest differs from the one in Phase 1, the paragraph says that image has the manifest of the superseded image, not of the current one.
- `02-CONTEXT.md` D-10 needed no change (it ends with "or a later one recorded under `## Current image`").

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The orchestrator made the `create_pull_request` call**
- **Found during:** Task 1 (Open the image PR)
- **Issue:** Executor agents have no GitHub MCP tools, so the plan's `create_pull_request` call could not be made from the executor.
- **Fix:** The orchestrator made the call. It first checked that no PR template exists and that no open PR had this head, used the plan's title, and wrote a body that lists the change, the reasons, the green branch run ids, the 20-commit `git log`, the "Create a merge commit" request and the PR attribution lines. The founder's merge answer was also collected by the orchestrator.
- **Files modified:** none
- **Verification:** `gh api` shows PR #4 merged with merge commit 69b5b7f4; remote head unchanged between creation and merge.
- **Committed in:** n/a (no repository change)

---

**Total deviations:** 1 auto-fixed (1 blocking, tooling access)
**Impact on plan:** None on content. The PR was created with the specified title, base, head and body contents; only the actor differed.

## Issues Encountered

None. Local gates before the commit all exited 0: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test -p mt-toolchain --test container_static` (30 passed), `cargo test --workspace --locked`, `cargo deny check`. `git reflog -n 30` has no `rebase` entry.

## Authentication Gates

None. The founder merge in Task 2 was the planned checkpoint:human-action, not an auth gate.

## Known Stubs

None.

## Threat Flags

None. T-02-04 (remote head unchanged from PR creation to merge, founder merged by hand), T-02-05 (no workflow change), T-02-06 (fast-forward merge-back, no rebase, no force push) and T-02-07 (digest copied from the publish job of the run whose head_sha equals origin/main; one pullable digest in the README) are mitigated as planned.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plans 02-06, 02-08 and 02-09 can pull the image by `ghcr.io/sat-wik/migrationtool-toolchain@sha256:e83006c0a7e37f788afcbd1c95554e8c4dfc61c3b68da3ff964caf47fb3851b9` (D-10).
- The founder's Phase 1 check was not repeated on this image; the README says so.
- REQUIREMENTS.md and requirement status were left to the orchestrator; BUILD-01 and BUILD-02 are shared with later plans of this phase.
- Plan 02-03 worktrees are untouched; the orchestrator merges them into the session branch.

---
*Phase: 02-test-bed-and-build-capture*
*Completed: 2026-10-13*

## Self-Check: PASSED

`container/README.md` and this SUMMARY exist; commits 69b5b7f4 and 7b31ebe are ancestors of HEAD; `git merge-base --is-ancestor origin/main HEAD` exits 0; the README holds exactly one pullable digest; main's latest push run of container.yml is success for origin/main.
