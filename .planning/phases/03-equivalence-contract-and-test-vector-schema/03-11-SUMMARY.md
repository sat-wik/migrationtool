---
phase: 03-equivalence-contract-and-test-vector-schema
plan: 11
subsystem: ci-handoff-guard
tags: [safe_untar, symlink-guard, tar, github-actions, vectors-job, gap-closure, behavioural-test]

requires:
  - phase: 03-equivalence-contract-and-test-vector-schema
    provides: plan 03-08 vectors job (D-32), the sample replay and the WR-04 safe_untar guard of review fix 8bdbd61
provides:
  - "testbed.yml restore step: safe_untar refuses a symlink only when its target is absolute or, walked from the link's own directory, leaves the tree (or pops a component that is itself a symlink of the archive); refuses absolute or `..` member names, non file/directory/symlink members and empty archives"
  - "eqv_04_workflow_static_safe_untar_accepts_in_tree_links_and_refuses_escaping_ones: runs the workflow's own safe_untar, extracted from testbed.yml, on 19 real tars built with GNU tar"
  - "testbed run 41735869792 (head 752badf): vectors job green with build, layouts, twice-run vectors, determinism, sample fixture and the replay of the committed sample all run in the pinned image"
affects: [phase 3 verification (gap closed pending /gsd-verify-work 3), any later change to the vectors job restore step]

requirements-completed: [EQV-04]

status: complete
commits: 5
plan_head_before: 98e063502301e998dcea8aa7a37b7126dfbd4c52
plan_head_after: f90c27bb1193d9e1fea0f6fea6206db76e3e9082
actuals:
  tokens: 20600
  tasks: 3
  commits: 5

tech-stack:
  added: []
  patterns:
    - "a CI shell guard that decides on workflow text is tested by extracting the function from the workflow with function_body and running it under bash on real archives through the mt-toolchain runner, with no skip path and no copy of the guard"
    - "a symlink target is vetted by walking dirname(member)/target as a component stack; a `..` that would pop a component that is an archive symlink fails closed"

key-files:
  created:
    - .planning/phases/03-equivalence-contract-and-test-vector-schema/03-11-CI-NOTICES.md
    - .planning/phases/03-equivalence-contract-and-test-vector-schema/03-11-VECTORS-NOTICES.md
  modified:
    - .github/workflows/testbed.yml
    - crates/mt-harness/tests/eqv_04_workflow_static.rs

key-decisions:
  - "A `..` that would pop a component that is itself a symlink of the same archive is refused even where the link would land inside the tree (fail closed): the guard works on names from the listing and cannot tell where such a path lands, and the kernel resolves d/up/.. to the parent of d/up's target, so a lexical walk is wrong exactly there. Neither nanopb's nor nimble's links have that shape (10 local links all accepted)."
  - "The member name comes from `tar -tf`, never from splitting the `tar -tvf` line; the target is what follows ` NAME -> ` in the verbose line, so a member name containing ` -> ` cannot shift the target (rows accept_arrow_in_name, refuse_arrow_in_name)."
  - "The behavioural test has no skip path: it asserts /bin/bash exists and panics without GNU tar, because a vacuous pass is how the WR-04 regression got through."

patterns-established:
  - "Notices files are generated from gh api annotations by a script that first reproduces the previous plan's committed files byte for byte"

coverage:
  - id: D1
    description: "safe_untar refuses a symlink only when it is absolute or leaves the tree as resolved from its own directory (fail closed for a `..` through an archive link); the CI-observed nanopb and nimble links are accepted"
    requirement: EQV-04
    verification:
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_04_workflow_static.rs#eqv_04_workflow_static_safe_untar_accepts_in_tree_links_and_refuses_escaping_ones"
        status: pass
      - kind: other
        ref: "the workflow's own function accepts a tar of .mt-work/capture and .mt-work/run (10 in-tree symlinks, all extracted)"
        status: pass
    human_judgment: false
  - id: D2
    description: "A behavioural test runs the workflow's own safe_untar on 19 real-tar cases and the static restore test pins tar -tf and the two exact call lines"
    requirement: EQV-04
    verification:
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_04_workflow_static.rs#eqv_04_workflow_static_restore_vets_the_archives_before_unpacking"
        status: pass
      - kind: unit
        ref: "cargo test --workspace --locked: 557 passed, 0 failed"
        status: pass
    human_judgment: false
  - id: D3
    description: "The vectors job is green at the plan's last trigger-path commit with every evidence step run in the pinned image, the 15 committed corpora replayed identical 1, and the vectors notices equal to 03-08 round trip 3"
    requirement: EQV-04
    verification:
      - kind: other
        ref: "gh api repos/sat-wik/migrationtool/actions/runs/41735869792: .github/workflows/testbed.yml 752badfc854dbcf4e3b27dfc3971f23798915d6d success; vectors job 126268645422 success; the seven named steps success"
        status: pass
    human_judgment: false

duration: 31min
completed: 2026-11-03
---

# Phase 3 Plan 11: safe_untar gap closure Summary

**The vectors job's `safe_untar` now refuses a capture-tar symlink only when it is absolute or walks out of the tree (fail closed through archive links), proven on 19 real-tar rows, and one green testbed run shows the vectors job's build, run, determinism and committed-sample replay running again in the pinned image.**

## Performance

- **Duration:** 31 min
- **Started:** 2026-11-03T22:55:26Z
- **Completed:** 2026-11-03T23:26:00Z
- **Tasks:** 3 (Tracer, Task 2, Task 3)
- **Files modified:** 4 (2 code, 2 generated notices)
- **Round trips used:** 1 of the cap of 3

## Accomplishments

- Gap missing item 1: the any-`..` grep is gone; the guard walks each symlink target from the link's own directory and refuses only an absolute target, a walk that pops past the top, or a `..` that pops an archive symlink. The rest of WR-04 is kept (non file/directory/symlink members, absolute or `..` member names, nothing extracted before the whole listing passed, one extraction with `--no-same-owner --no-same-permissions`, both archives through `safe_untar`).
- Gap missing item 2: `eqv_04_workflow_static_safe_untar_accepts_in_tree_links_and_refuses_escaping_ones` extracts the function with `function_body(&step_text(&vectors_job(), RESTORE_STEP), "safe_untar")` and runs it under `set -euo pipefail` with bash through the mt-toolchain runner on tars made with GNU tar.
- Gap missing item 3: testbed run 41735869792 for head 752badf is green, and the vectors job ran every evidence step.

## Task Commits

Each task was committed atomically (TDD: RED then GREEN):

1. **Tracer: the workflow's own safe_untar accepts the CI-observed in-tree links and refuses the gap's escaping ones**
   - RED `fff63b8` (test) - 6 rows, four accept rows fail
   - GREEN `24c953a` (feat) - stack walk of each symlink target; also turns the test table into an array (clippy `useless_vec`)
2. **Task 2: the full case table, links through links, member names and the empty archive**
   - RED `530802e` (test) - 13 more rows (19 in all), four fail
   - GREEN `752badf` (feat) - `islink` first pass, member-name check, empty-archive check, static test tightened. This is the last commit that touches a trigger path.
3. **Task 3: CI round trip and notices** - `f90c27b` (docs): `03-11-CI-NOTICES.md` and `03-11-VECTORS-NOTICES.md`

**Plan metadata:** the commit that adds this SUMMARY (docs: complete plan). It cannot name its own hash.

## Gap closure

### Item 1: the rule

`safe_untar` in the restore step 'Unpack the captures of the testbed job' (the vectors job). Per symlink member: name from `tar -tf`, target from the `tar -tvf` line (everything after the first ` NAME -> `); an unlocatable target is refused. An absolute target is refused. Otherwise the stack is built from `dirname(name)` followed by the target's components (empty and `.` skipped, `..` pops, everything else pushes); a `..` on an empty stack, or one that would pop while the stack names an archive symlink, is refused with `::error::ARCHIVE holds a symlink that is absolute or leaves the tree: NAME -> TARGET`. Rows of the Task 2 table: accept_up_to_top, refuse_one_past_top, refuse_top_level_up, refuse_down_then_up, accept_dot_components, accept_arrow_in_name, refuse_arrow_in_name, refuse_up_through_link, refuse_dotdot_member_name, refuse_absolute_member_name, refuse_hard_link, refuse_empty_archive, accept_plain_members.

Local check of the workflow's own function on `.mt-work/capture` and `.mt-work/run` (15,541 entries): accepted, 10 symlinks extracted of 10 in the source tree, about 2 s.

### Item 2: the test

`eqv_04_workflow_static_safe_untar_accepts_in_tree_links_and_refuses_escaping_ones ... ok` (19 rows: 7 accepted and extracted, 12 refused with exit 1, an `::error::` line carrying the row's fragment, and an empty destination). `eqv_04_workflow_static_restore_vets_the_archives_before_unpacking ... ok` now also asserts `tar -tf` and the two call lines exactly. `eqv_04_workflow_static_phase_2_steps_are_unchanged` still passes. Added beyond the artifact list (private helpers): `fn run_tool`, `fn check_untar_case`, `const`-free table built as an array of `UntarCase`.

### Item 3: the in-image evidence (read with gh api)

Testbed run **41735869792** (`.github/workflows/testbed.yml`), head **752badfc854dbcf4e3b27dfc3971f23798915d6d**, conclusion success. Jobs: mt 126264790431 success, testbed **126265405983** success, vectors **126268645422** success. Also green for the same head: ci 41735869789 and container 41735869785. `git log -1 -- testbed crates Cargo.toml Cargo.lock container/pins.toml container/README.md .github/workflows/testbed.yml` is 752badf, so the run is for the plan's last trigger-path commit.

Vectors job steps (all `success`, none skipped):

| Step | Conclusion | Duration |
|------|-----------|----------|
| Set up job, checkout, download artifact, Make mt executable, Write the annotation and docker helpers, Pull the image by digest | success | |
| Run the executor self-test in the pinned image | success | |
| Fetch the TRACTOR sample and load it unchanged (local use only) | success | |
| Download the captures of the testbed job | success | |
| **Unpack the captures of the testbed job** | success | 2 s |
| **Build the reference drivers** | success | 12 s |
| **Publish the state layouts** | success | 0 s |
| **Run the reference vectors twice** | success | 96 s |
| **Run the vectors again and compare (vectors determinism)** | success | 102 s |
| **Fixture vectors sample** (and its parts 10 to 18, 19 on) | success | |
| **Replay the committed sample** | success | under 1 s |
| Post checkout, Complete job | success | |

The vectors job ran 4 min 31 s of its 45; no timeout change. The notices confirm: the rerun reports `rerun rows are identical to the first run's rows` and `identical 5768 files`; 38 `twice=identical` rows over 15 targets; every one of the 15 committed corpora under `testbed/vectors-sample` replays `identical 1` with `exit code: 0` (so the WR-01 provenance comparison of 90767fd has now run against the committed sample in the image); the eight testbed-* notices equal `02-22-CI-NOTICES.md` from line 5 on.

## Round trips

| # | Run | Head | Outcome |
|---|-----|------|---------|
| 1 | testbed 41735869792 (ci 41735869789, container 41735869785) | 752badfc854dbcf4e3b27dfc3971f23798915d6d | green: testbed success, vectors success with all seven named steps success, ci success |
| notices (not a round trip) | ci 41737919294 | f90c27bb1193d9e1fea0f6fea6206db76e3e9082 (docs only, starts only ci) | success |

One of the planned one, cap three. No second finding, no `fix-here`, no `extend`.

The fixture vectors-sample `tar sha256` of this run is `ca5f0796913858d4b84dc53b16e5ea764a872ebcdb4ce0da570c381811b11dd1`, **equal** to 03-08 round trip 3's `ca5f0796913858d4b84dc53b16e5ea764a872ebcdb4ce0da570c381811b11dd1`. No open item.

## Vectors notices against 03-08 round trip 3

`diff <(sed '1,4d' 03-08-VECTORS-NOTICES.md) <(sed '1,4d' 03-11-VECTORS-NOTICES.md)` prints nothing: all eight sections (vectors-image, vectors-selftest, vectors-tractor, vectors-build, vectors-layout, vectors-run, vectors-determinism, vectors-sample) are identical to those of run 40922236736. So none of the 22 commits of `git log --oneline 5e0d4a8..752badf -- crates .github testbed container` (CR-01, WR-01, WR-02, WR-06, IN-04, IN-09 among them) changed any notice text, and there are no unexplained differences and no open items from this comparison.

## Decisions Made

- **A `..` that pops an archive symlink is refused even where it would land inside the tree (fail closed).** The guard works on names from the listing and cannot tell where such a path lands. With `d/up -> ..` and `e -> d/up/..`, a purely lexical walk stays inside, but the kernel resolves `d/up/..` to the parent of the top. The price is a few refused in-tree shapes (for example `c -> a/../z` where `a` is a link). Neither nanopb's nor nimble's links have that shape: the 10 local links are all accepted. The comment above the function states this.
- Member name from `tar -tf`, target from the verbose line after the exact name (T-03-43); the two listings must have equal length.
- No skip path in the behavioural test (T-03-44, T-03-45).

## RED outputs (TDD gate)

Tracer RED (`fff63b8`, against the guard of 83352a1; `cargo test -p mt-harness --test eqv_04_workflow_static`): the target test failed on the planned assertion, the two refuse rows passed.

```text
4 of 6 rows failed:
accept_one_up: accepted (exit 0), but the guard did not accept it
  exit: Some(1)
  stdout: lrwxrwxrwx root/root         0 2026-11-03 22:56 a/b/c.h -> ../x.h
::error::/tmp/.tmpOkk38P/case.tar holds a symlink that is absolute or leaves the tree
accept_two_up: accepted (exit 0), but the guard did not accept it
  exit: Some(1)
  stdout: lrwxrwxrwx root/root         0 2026-11-03 22:56 a/b/c.h -> ../../x.h
::error::/tmp/.tmpxS0RwN/case.tar holds a symlink that is absolute or leaves the tree
accept_nanopb_ci: accepted (exit 0), but the guard did not accept it
  exit: Some(1)
  stdout: lrwxrwxrwx root/root         0 2026-11-03 22:56 run/nanopb/default/src/spm_headers/nanopb/pb.h -> ../../pb.h
::error::/tmp/.tmpXqVOd2/case.tar holds a symlink that is absolute or leaves the tree
accept_nimble_ci: accepted (exit 0), but the guard did not accept it
  exit: Some(1)
  stdout: lrwxrwxrwx root/root         0 2026-11-03 22:56 run/nimble/default/src/porting/npl/riot/include/npl_syscfg/npl_sycfg.h -> ../syscfg/syscfg.h
::error::/tmp/.tmpxbX1rV/case.tar holds a symlink that is absolute or leaves the tree
test result: FAILED. 13 passed; 1 failed
```

This is the CI failure of runs 41265174150 and 41275770544, reproduced offline. Semantic assessment: the target test executed, built real tars, and failed on the planned assertion (the in-tree shapes are accepted) for the intended reason (the any-`..` grep); no setup, load or unrelated fault.

Task 2 RED (`530802e`, against the Tracer's guard): exactly the four rows marked RED failed, the other 15 passed.

```text
4 of 19 rows failed:
refuse_up_through_link: refused (exit 1), but the exit status differs
  exit: Some(0)
  stdout:
refuse_dotdot_member_name: refused (exit 1), but the exit status differs
  exit: Some(2)
  stderr: tar: Removing leading `../' from member names
tar: ../outside: Member name contains '..'
tar: Exiting with failure status due to previous errors
refuse_absolute_member_name: refused (exit 1), but the exit status differs
  exit: Some(0)
  stderr: tar: Removing leading `/' from member names
refuse_empty_archive: refused with an ::error:: line saying "no members", but the message differs
  exit: Some(1)
  stdout:
::error::/tmp/.tmpjmuIzd/case.tar holds a member that is not a file, directory or symlink
```

## TDD Gate Compliance

RED `test(03-11)` commits precede their GREEN `feat(03-11)` commits for both the Tracer (fff63b8, 24c953a) and Task 2 (530802e, 752badf). No REFACTOR commit.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] clippy `useless_vec` on the case table**
- **Found during:** Tracer GREEN (pre-merge clippy)
- **Issue:** `let cases = vec![...]` fails `cargo clippy --all-targets -- -D warnings`.
- **Fix:** the table is an array. This one-line test change is in the Tracer GREEN commit `24c953a`, so that commit touches the test file as well as the workflow (the RED commit held only the test file, as required).
- **Verification:** clippy and the test file pass.

**2. [Minor, implementation shape] `left` flag instead of `break` in the walk**
- The walk sets a `left` flag and skips the rest of the components instead of `break`ing, which keeps the loop body to plain `if`/`continue`. Behaviour is the one the plan specifies.

**Total deviations:** 1 auto-fixed (Rule 3), 1 shape note. **Impact on plan:** none on scope; no file outside the workflow and the static test changed, `testbed/vectors-sample` unchanged, no Cargo change.

Added to the test file beyond the named artifacts (private): `run_tool`, `check_untar_case`, and the `HardLink` member variant used by refuse_hard_link.

## Issues Encountered

- A first local-check command used `rm -rf "$S"` on a mktemp variable and the harness safety check refused it. The check was re-run in a fixed scratch directory and cleaned up with a literal path. No effect on results.
- Python (`python3 -I`) was used only for scratch editing and for the notices generator under the session scratchpad; none of it is committed.

## Stub tracking

None: no stub, placeholder or hardcoded empty value was added.

## Threat Flags

None: no new endpoint, auth path or trust boundary beyond the plan's threat model (T-03-42 to T-03-46 are all covered: refused rows for T-03-42, arrow rows for T-03-43, the 10 CI-shaped links and the step conclusions for T-03-44, gh api step reads for T-03-45, about 2 s measured for T-03-46).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- The gap of 03-VERIFICATION.md (Truth 4) has its three missing items evidenced; the status change is for `/gsd-verify-work 3` to make, together with the one human-verification item already listed there (the equivalence-contract wording changes after UAT item 1).
- `git diff --name-only 83352a1 HEAD -- . ':!.planning'` prints exactly `.github/workflows/testbed.yml` and `crates/mt-harness/tests/eqv_04_workflow_static.rs`; `git diff --quiet 83352a1 HEAD -- testbed/vectors-sample` is clean.

## Self-Check: PASSED

Files found: .github/workflows/testbed.yml, crates/mt-harness/tests/eqv_04_workflow_static.rs, 03-11-CI-NOTICES.md, 03-11-VECTORS-NOTICES.md. Commits found on HEAD: fff63b8, 24c953a, 530802e, 752badf, f90c27b. Task 3 verify commands 1 to 3 printed VERIFY1_OK, VERIFY2_OK and VERIFY3_OK; ci for f90c27b (run 41737919294) is success.

---
*Phase: 03-equivalence-contract-and-test-vector-schema*
*Completed: 2026-11-03*
