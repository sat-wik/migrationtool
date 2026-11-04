---
phase: 03-equivalence-contract-and-test-vector-schema
plan: 12
subsystem: ci-handoff-guard
tags: [safe_untar, symlink-guard, tar, numeric-owner, github-actions, vectors-job, gap-closure, behavioural-test, fake-tar]

requires:
  - phase: 03-equivalence-contract-and-test-vector-schema
    provides: plan 03-11 safe_untar walk and its 19-row behavioural table (head 752badf); 03-VERIFICATION.md re-verification gap (CR-01) and 03-REVIEW.md CR-01, WR-01, WR-02, IN-01
provides:
  - "testbed.yml restore step: safe_untar never steps into or out of an archive symlink (a link target whose walk meets a stack that names an archive symlink is refused with 'passes through a symlink of the archive'), over a numeric-owner listing whose owner column is checked"
  - "eqv_04_workflow_static_safe_untar_accepts_in_tree_links_and_refuses_escaping_ones: 27 rows (24 on real GNU tar archives, 3 on a fake tar), per-row message fragments, an outside-destination snapshot around every run, and a host tool check with no skip path"
  - "testbed run 41881917020 (head adfd2bd): vectors job green with the corrected guard, the seven evidence steps run in the pinned image, notices equal to 02-22 and 03-11"
affects: [plan 03-13 (records: disposition, security register, validation map), phase 3 re-verification (/gsd-verify-work 3), any later change to the vectors job restore step]

requirements-completed: [EQV-04]

status: complete
commits: 5
plan_head_before: 498c11ac4538b51f3bb4bc0cf641b49222e78436
plan_head_after: 58b018539e0339567b65ed9e8a94e7de13f47c65
actuals:
  tokens: 8100
  tasks: 3
  commits: 5

tech-stack:
  added: []
  patterns:
    - "a symlink target is vetted by a component walk that refuses to push or pop while the stack so far names a symlink of the same archive, so the walked path is the path the kernel resolves (fail closed where a link is stepped through)"
    - "a tar listing is parsed only from a numeric-owner listing whose owner column is checked (digits/digits), because no column before the name can then hold '>'"
    - "listing branches GNU tar cannot reach are driven by a fake tar on the PATH of that row's bash child only, with an exit-99 extraction so it can never extract"

key-files:
  created:
    - .planning/phases/03-equivalence-contract-and-test-vector-schema/03-12-CI-NOTICES.md
    - .planning/phases/03-equivalence-contract-and-test-vector-schema/03-12-VECTORS-NOTICES.md
  modified:
    - .github/workflows/testbed.yml
    - crates/mt-harness/tests/eqv_04_workflow_static.rs

key-decisions:
  - "A target that steps into or out of an archive symlink is refused even where it lands inside the tree (X -> m/n with c -> X/y): the lexical walk is exact only if it never crosses a link, and a link whose last component names another link (a -> b, b -> c/..) stays accepted because that link is vetted on its own line."
  - "The listing is numeric-owner (tar --numeric-owner -tvf) and a symlink line whose owner column is not digits/digits is refused: the text before the name is then the mode, digits, size, date and time, none of which can hold '>', so the first ' NAME -> ' is the real one."

patterns-established:
  - "Notices files are generated from gh api annotations by a script in the session scratchpad and compared with the previous plan's committed files from line 5 on"

coverage:
  - id: D1
    description: "safe_untar refuses CR-01's alias archive in both member orders and the descend-through-link archive with the message 'holds a symlink that passes through a symlink of the archive', and still accepts a link to a link, every plan 03-11 accept row and the local capture and run trees (10 of 10 symlinks extracted)"
    requirement: EQV-04
    verification:
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_04_workflow_static.rs#eqv_04_workflow_static_safe_untar_accepts_in_tree_links_and_refuses_escaping_ones"
        status: pass
      - kind: other
        ref: "the workflow's own function on a tar of .mt-work/capture and .mt-work/run: exit 0, 10 symlinks extracted of 10"
        status: pass
    human_judgment: false
  - id: D2
    description: "The numeric-owner listing and the owner-column check defeat a forged uname on a real GNU tar archive (refuse_forged_owner) and on a fake listing (refuse_forged_owner_listing); every refusal branch has a row that asserts only its own message, every row shows nothing outside dest changed, and the test fails on a host without GNU tar or bash 4.3"
    requirement: EQV-04
    verification:
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_04_workflow_static.rs#eqv_04_workflow_static_restore_vets_the_archives_before_unpacking"
        status: pass
      - kind: unit
        ref: "cargo test --workspace --locked: 557 passed, 0 failed, 23 ignored"
        status: pass
    human_judgment: false
  - id: D3
    description: "The vectors job is green at the plan's last trigger-path commit with the restore, build, layouts, twice-run vectors, determinism, sample fixture and committed-sample replay steps run in the pinned image, and the notices equal 02-22-CI-NOTICES.md and 03-11-VECTORS-NOTICES.md from line 5 on"
    requirement: EQV-04
    verification:
      - kind: other
        ref: "gh api repos/sat-wik/migrationtool/actions/runs/41881917020: .github/workflows/testbed.yml adfd2bdbbe229b63df6d682efc48199447776599 success; vectors job 126760859985 success; the seven named steps success; ci 41881917214 success"
        status: pass
    human_judgment: false

duration: 27min
completed: 2026-11-04
---

# Phase 3 Plan 12: safe_untar CR-01 gap closure Summary

**The vectors job's `safe_untar` now never steps into or out of an archive symlink and reads a numeric-owner listing with a checked owner column, proven on 27 real-tar and fake-tar rows, and one green testbed run shows the vectors job's seven evidence steps running again in the pinned image on the corrected guard.**

## Performance

- **Duration:** 27 min of executor time (the CI wait included)
- **Started:** 2026-11-04T17:45:04Z
- **Completed:** 2026-11-04T18:12:00Z
- **Tasks:** 3 (Tracer, Task 2, Task 3)
- **Files modified:** 4 (2 code, 2 generated notices)
- **Round trips used:** 1 of the cap of 3

## Accomplishments

- Gap item 1 (CR-01, soundness): before a component is pushed onto or popped from a non-empty stack, the joined stack is looked up in `islink`; an archive symlink there refuses the link. The comment above the function states the real rule and the fail-closed price.
- Gap item 2 (WR-02): the table grew from 19 to 27 rows; each refused row asserts a fragment only its own refusal prints, the two GNU-tar-unreachable branches are driven by a fake tar, and every row snapshots the case directory outside `dest` around the run.
- Gap item 3 (WR-01): `tar --numeric-owner -tvf` plus an owner-column check; a real GNU tar archive whose header carries the forged uname `u a -> p/q/r/s/t` is refused.
- IN-01: `assert_host_tools()` runs first and fails (no skip path) unless the host tar is GNU tar and `/bin/bash` is 4.3 or later.
- Gap item 4, CI half: testbed run 41881917020 for head adfd2bd is green with the seven named vectors steps `success`; notices equal 02-22's and 03-11's from line 5 on.

## Task Commits

Each task was committed atomically (TDD: RED then GREEN):

1. **Tracer: the workflow's own safe_untar refuses CR-01's alias shape on real tars, in either member order, and still accepts every in-tree link**
   - RED `cca62a9` (test) - 4 new rows plus refuse_up_through_link's fragment; three exit-Some(0) rows and the message mismatch
   - GREEN `1201217` (feat) - stack check before every push or pop, `why` replaces `left`, comment rewritten
2. **Task 2: numeric-owner listing and forged-owner rows, fake-listing rows, distinct fragments, outside-destination snapshot, host tool checks**
   - RED `0728608` (test) - 4 more rows (27 in all); refuse_forged_owner (exit 0) and refuse_forged_owner_listing (exit 99) fail
   - GREEN `adfd2bd` (feat) - `--numeric-owner` listing, `owner_re` check, `in the listing line:` suffix, static restore test tightened. This is the last commit that touches a trigger path.
3. **Task 3: CI round trip and notices** - `58b0185` (docs): `03-12-CI-NOTICES.md` and `03-12-VECTORS-NOTICES.md`

**Plan metadata:** the commit that adds this SUMMARY (docs: complete plan). It cannot name its own hash.

## Gap closure

### Item 1: the rule and the CR-01 rows

`safe_untar` in the restore step 'Unpack the captures of the testbed job' (vectors job). Per symlink member the walk starts at the link's own directory and takes the target's components in order. An absolute target, or a `..` on an empty stack, sets `why` to `is absolute or leaves the tree`. For every other component, while `why` is empty and the stack is non-empty, the joined stack is looked up in `islink`; an archive symlink there sets `why` to `passes through a symlink of the archive` and ends the walk. Only then is the component pushed, or popped on `..`. The refusal line is `::error::ARCHIVE holds a symlink that $why: NAME -> TARGET`. The first pass records every symlink before the second pass walks any, so the verdict does not depend on member order.

Rows: refuse_alias_up_through_link (CR-01's six members in the plan's order), refuse_alias_up_through_link_reordered (the same six reversed), refuse_descend_through_link (the pinned fail-closed price), accept_link_to_link, and refuse_up_through_link with its new fragment. Boundary edges kept: accept_up_to_top and refuse_one_past_top.

Local check of the workflow's own function (awk-extracted) on a tar of `.mt-work/capture` and `.mt-work/run`: exit 0, 10 symlinks extracted of 10 in the source tree, 3.6 s wall time including the tar creation and extraction.

### Item 2: the 27 rows, the fragments and the snapshot

27 labelled rows: 24 whose archive the host GNU tar makes (the 19 rows of plan 03-11, the four Tracer rows and refuse_forged_owner) and 3 FakeListing rows (refuse_forged_owner_listing, refuse_unreadable_target, refuse_listing_mismatch). Fragments: the refused link rows assert `holds a symlink that is absolute or leaves the tree` or `holds a symlink that passes through a symlink of the archive`; the member-name rows assert `holds a member name that is absolute or leaves the tree`; `not a file, directory or symlink`, `has no members`, `cannot read the target of the symlink` and `tar -tf and tar -tvf list different members` each belong to one refusal. No row asserts the bare `leaves the tree`. Every row writes `<case dir>/sentinel`, takes `snapshot_outside_dest` before and after the bash run (paths, kinds, file bytes in hex, link targets; `dest` skipped, links never followed) and fails with the differing paths when they differ. `run_safe_untar` was split into `write_safe_untar_script` and `run_safe_untar` so the script exists before the first snapshot; `run_tool` and `run_safe_untar` take the child's PATH (`HOST_PATH` = `/usr/bin:/bin` for every real-tar call, `<case dir>/fakebin:/usr/bin:/bin` for FakeListing rows only).

### Item 3: the numeric owner and the two forged-owner rows

The listing is `tar --numeric-owner -tvf "$archive"` (exactly one such line in the restore step). In the second pass, before the target is cut, `[[ $line =~ $owner_re ]]` with `owner_re='^l[^ ]* [0-9]+/[0-9]+ '` must match, or the link is refused with `cannot read the target of the symlink NAME in the listing line: LINE`; the absent-pattern refusal carries the same suffix. refuse_forged_owner is a real GNU tar archive made with `--owner='u a -> p/q/r/s/t:0' --group='g:0'` (no Python, no hand-written header); refuse_forged_owner_listing is the same forgery as a fake listing line.

### Item 4 (CI half): the in-image evidence (read with gh api)

Testbed run **41881917020** (`.github/workflows/testbed.yml`), head **adfd2bdbbe229b63df6d682efc48199447776599**, conclusion success. Jobs: mt 126756279037 success, testbed **126756867942** success, vectors **126760859985** success. Also for the same head: ci 41881917214 success. `git log -1 -- testbed crates Cargo.toml Cargo.lock container/pins.toml container/README.md .github/workflows/testbed.yml` is adfd2bd, so the run is for the plan's last trigger-path commit.

Vectors job steps (all `success`, none skipped; durations from the jobs API):

| Step | Conclusion | Duration |
|------|-----------|----------|
| Set up job, checkout, download artifact, Make mt executable, Write the annotation and docker helpers | success | |
| Pull the image by digest | success | 48 s |
| Run the executor self-test in the pinned image | success | 3 s |
| Fetch the TRACTOR sample and load it unchanged (local use only) | success | 1 s |
| Download the captures of the testbed job | success | 2 s |
| **Unpack the captures of the testbed job** | success | 1 s |
| **Build the reference drivers** | success | 11 s |
| **Publish the state layouts** | success | 0 s |
| **Run the reference vectors twice** | success | 78 s |
| **Run the vectors again and compare (vectors determinism)** | success | 78 s |
| **Fixture vectors sample** (and its parts 10 to 18, 19 on) | success | 7 s |
| **Replay the committed sample** | success | 1 s |
| Post checkout, Complete job | success | |

The vectors job ran 3 min 58 s of its 45 (18:03:38Z to 18:07:36Z); no timeout change. The notices confirm `rerun rows are identical to the first run's rows`, `identical 5768 files`, and 15 of 15 committed corpora under `testbed/vectors-sample` replayed `identical 1`.

## Round trips

| # | Run | Head | Outcome |
|---|-----|------|---------|
| 1 | testbed 41881917020 (ci 41881917214) | adfd2bdbbe229b63df6d682efc48199447776599 | green: testbed success, vectors success with all seven named steps success, ci success |
| notices (not a round trip) | ci 41884287313 | 58b018539e0339567b65ed9e8a94e7de13f47c65 (docs only, starts only ci) | success |

One of the planned one, cap three. No second finding, no `fix-here`, no `extend`.

The fixture vectors-sample `tar sha256` of this run is `ca5f0796913858d4b84dc53b16e5ea764a872ebcdb4ce0da570c381811b11dd1`, **equal** to plan 03-11's `ca5f0796913858d4b84dc53b16e5ea764a872ebcdb4ce0da570c381811b11dd1`.

## Vectors notices against 03-11 round trip 1

`diff <(sed '1,4d' 03-11-VECTORS-NOTICES.md) <(sed '1,4d' 03-12-VECTORS-NOTICES.md)` prints nothing: all eight sections (vectors-image, vectors-selftest, vectors-tractor, vectors-build, vectors-layout, vectors-run, vectors-determinism, vectors-sample) are identical to those of run 41735869792. The eight testbed-* notices equal `02-22-CI-NOTICES.md` from line 5 on (empty diff). So this plan, which changes nothing the executor records, changed no notice text, and there is no unexplained difference and no open item from this comparison. The notices were generated from the `gh api` annotations (`--paginate --jq '.[]'`, slurped with `jq -s`) by a script in the session scratchpad, not retyped.

## Decisions Made

- **A target that steps into or out of an archive symlink is refused even where it lands inside the tree.** The walk on names is exact only if it never crosses a link; CR-01 showed that popping a name the walk had pushed through an aliasing link was the hole. The price is pinned: `c -> X/y` with `X -> m/n` is refused with the same message although it lands inside the tree; a link whose last target component names another link (`a -> b`, `b -> c/..`) is accepted because that link is vetted on its own line. Neither nanopb's nor nimble's links have either shape (the 10 local links are all accepted, and CI accepted the real captures).
- **The listing is numeric-owner and the owner column is checked.** With `--numeric-owner` the text before the name is the mode, `uid/gid` digits, the size, the date and the time, none of which can hold `>`; any ` NAME -> ` that started earlier would have to sit in that text or repeat the prefix's tail, which holds no `>`, so the first occurrence is the real one. A tar that ignored the option is caught by the owner-column check (a line that does not match is refused).

## Deviations from Plan

None - plan executed exactly as written, with these notes that are not deviations:

- `run_safe_untar` was split into `write_safe_untar_script` and `run_safe_untar` (the plan allowed this and asked for it to be named). Private helpers added: `make_member` (shared tree builder for the Tree, Ordered and Owned archives), `write_safe_untar_script`, and `HOST_PATH` (const).
- The plan edits to `testbed.yml` stayed inside the restore step; the region diff against 752badf is empty.
- Because plan 03-12's `<automated>` of the Tracer checks `grep -c '^ *}$'` as 1 over the awk-extracted function, the body keeps the single closing brace line; it prints 1.
- The REQUIREMENTS.md status mark for EQV-04 was not set by `requirements.mark-complete`: the plan's "Not planned here" list says those marks are set when re-verification passes, not by an executor (commit 00a706e reverted a premature mark).
- `workflow.tdd_mode` is false and `cargo test` output (libtest text) is not a report format of `gsd_run check tdd-red-evidence`; the RED evidence below is the unmodified test output with a semantic assessment, not a classifier verdict.

**Total deviations:** 0 auto-fixed. **Impact on plan:** none.

## RED outputs (TDD gate)

Tracer RED (`cca62a9`, against the guard of 752badf; `cargo test -p mt-harness --test eqv_04_workflow_static safe_untar`). The target test failed on the planned assertions: the three CR-01 and descend rows accepted what must be refused, and refuse_up_through_link refused with the old message. accept_link_to_link already passed, as the plan expected.

```text
4 of 23 rows failed:
refuse_up_through_link: refused with an ::error:: line saying "holds a symlink that passes through a symlink of the archive", but the message differs
  exit: Some(1)
  stdout: ::error::/tmp/.tmprYeF3g/case.tar holds a symlink that is absolute or leaves the tree: e -> d/up/..

  stderr: 
refuse_alias_up_through_link: refused (exit 1), but the exit status differs
  exit: Some(0)
  stdout: 
  stderr: 
refuse_alias_up_through_link_reordered: refused (exit 1), but the exit status differs
  exit: Some(0)
  stdout: 
  stderr: 
refuse_descend_through_link: refused (exit 1), but the exit status differs
  exit: Some(0)
  stdout: 
  stderr: 
```

Task 2 RED (`0728608`, against the Tracer's guard `1201217`). Exactly the two planned rows failed; the three other new rows and all older rows passed. Semantic assessment: refuse_forged_owner fails because the plain `tar -tvf` line `lrwxrwxrwx u a -> p/q/r/s/t/g 0 ... a -> ../../../../etc` is cut at the owner column, the forged target `p/q/r/s/t/g 0 ... a -> ../../../../etc` stays inside the tree and the archive is accepted (exit 0); refuse_forged_owner_listing reaches the fake tar's extraction (exit 99, stderr `fake tar: extraction reached`) for the same reason.

```text
2 of 27 rows failed:
refuse_forged_owner: refused (exit 1), but the exit status differs
  exit: Some(0)
  stdout: 
  stderr: 
refuse_forged_owner_listing: refused (exit 1), but the exit status differs
  exit: Some(99)
  stdout: 
  stderr: fake tar: extraction reached
```

## Records for plan 03-13

- New labels: refuse_alias_up_through_link, refuse_alias_up_through_link_reordered, refuse_descend_through_link, accept_link_to_link, refuse_forged_owner, refuse_forged_owner_listing, refuse_unreadable_target, refuse_listing_mismatch.
- Test names: `eqv_04_workflow_static_safe_untar_accepts_in_tree_links_and_refuses_escaping_ones` and `eqv_04_workflow_static_restore_vets_the_archives_before_unpacking`.
- Table size and split, counted from the test file at HEAD: 27 rows, 24 whose archive the host GNU tar makes (the 19 rows of plan 03-11, the four Tracer rows and refuse_forged_owner) and 3 FakeListing rows (refuse_forged_owner_listing, refuse_unreadable_target, refuse_listing_mismatch).
- Run id **41881917020**, head SHA **adfd2bdbbe229b63df6d682efc48199447776599**, testbed job 126756867942, vectors job **126760859985**, restore step duration **1 s** (18:04:37Z to 18:04:38Z); ci for the head 41881917214 success.
- Commits of this plan: `cca62a9` (RED, Tracer), `1201217` (GREEN, Tracer), `0728608` (RED, Task 2), `adfd2bd` (GREEN, Task 2), `58b0185` (notices), then the SUMMARY commit.
- Threat evidence for T-03-42..T-03-48: the 27 rows with their per-row fragments and outside-dest snapshots; the workflow's own function on the 10-link local trees (accepted, 10 of 10 extracted, T-03-44); the 1 s restore step in CI against a 45-minute job (T-03-46); the region diff against 752badf empty and the seven step conclusions read with gh api (T-03-45).

## Issues Encountered

- The first `gh api` listing of runs for the pushed head was empty for about five minutes; the runs appeared without any action and the push was not repeated.
- The `container` workflow also started on the first push (path-filtered on `.github/**`); it was not awaited because this plan changes nothing it records.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Plan 03-13 can cite the records above. The re-verification of Phase 3 (`/gsd-verify-work 3`) can read item 1 of the gap as closed in code and in the image. Not done here by design: 03-REVIEW-DISPOSITION.md, 03-SECURITY.md and 03-VALIDATION.md updates, and the REQUIREMENTS.md / ROADMAP.md status marks for EQV-01..EQV-04.

## Known Stubs

None.

## Threat Flags

None - no new network endpoint, auth path or trust-boundary file access; the fake tar lives only in a test tempdir and is on the PATH of FakeListing rows alone (T-03-49).

## Self-Check: PASSED

- Files found: 03-12-CI-NOTICES.md, 03-12-VECTORS-NOTICES.md, .github/workflows/testbed.yml, crates/mt-harness/tests/eqv_04_workflow_static.rs.
- Commits found as ancestors of HEAD: cca62a9, 1201217, 0728608, adfd2bd, 58b0185 (5 measured from plan_head_before).
- Task 3 verify commands 1 to 3 printed V1_OK, V2_OK and V3_OK; ci for 58b0185 (run 41884287313) concluded success.

---
*Phase: 03-equivalence-contract-and-test-vector-schema*
*Completed: 2026-11-04*
