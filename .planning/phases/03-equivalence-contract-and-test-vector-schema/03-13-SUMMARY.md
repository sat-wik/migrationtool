---
phase: 03-equivalence-contract-and-test-vector-schema
plan: 13
subsystem: testing
tags: [records, threat-register, review-disposition, validation-map, gap-closure, safe_untar]

requires:
  - phase: 03-equivalence-contract-and-test-vector-schema
    provides: "plan 03-12 evidence: 27-row safe_untar table, testbed run 41881917020 (head adfd2bd) green, Records for plan 03-13"
provides:
  - "CR-01, WR-01, WR-02 and IN-01 recorded fixed in 03-REVIEW-DISPOSITION.md (open: 3, total: 21)"
  - "T-03-42 closed, T-03-43..T-03-46 re-evidenced, T-03-47..T-03-50 registered and closed in 03-SECURITY.md (51 rows, threats_open: 0, status: verified)"
  - "03-VALIDATION.md 03-11 row limited to the exact-name case, rows 03-12-T1, 03-12-T2 and 03-12-T3 added"
affects: [03-verify-work, ship-gate, phase-3-reverification]

actuals:
  tokens: 5500
  tasks: 2
  commits: 2
plan_head_before: de2184abde56f2f5dd254add4a17b52d0c2c1b06
plan_head_after: 9f78ac43a374e1a98991d5b73a03fcc435a00196

tech-stack:
  added: []
  patterns: ["a closure or fix is recorded only with a test that passes at HEAD and a run read with gh api"]

key-files:
  created: []
  modified:
    - .planning/phases/03-equivalence-contract-and-test-vector-schema/03-REVIEW-DISPOSITION.md
    - .planning/phases/03-equivalence-contract-and-test-vector-schema/03-SECURITY.md
    - .planning/phases/03-equivalence-contract-and-test-vector-schema/03-VALIDATION.md

key-decisions:
  - "T-03-42 is closed on plan 03-12's evidence (through-link rule, three alias and descend rows, the 27-row test at HEAD, testbed run 41881917020 read with gh api); the 2026-11-03 re-opening note stays in the cell as history"
  - "The 03-11 validation row states the exact-name case only and says the alias shape stayed accepted until plan 03-12 (CR-01)"

requirements-completed: [EQV-04]

coverage:
  - id: D1
    description: "CR-01, WR-01, WR-02 and IN-01 recorded fixed in 03-REVIEW-DISPOSITION.md with Source cells naming 03-12-SUMMARY.md and testbed run 41881917020; open: 3, total: 21; no other row changed"
    requirement: EQV-04
    verification:
      - kind: other
        ref: "Task 2 verify 1: frontmatter and table checks, plus the row-preservation diff against 00a706e (exit 0, empty diff)"
        status: pass
    human_judgment: false
  - id: D2
    description: "T-03-42 closed with its evidence and history, T-03-43..T-03-46 re-evidenced, T-03-47..T-03-50 registered (51 rows), threats_open 0, status verified, sign-off checked, audit row 51/51/0"
    requirement: EQV-04
    verification:
      - kind: unit
        ref: "cargo test -p mt-harness --test eqv_04_workflow_static#eqv_04_workflow_static_safe_untar_accepts_in_tree_links_and_refuses_escaping_ones"
        status: pass
      - kind: other
        ref: "Tracer verify 1 (gh api read of testbed run 41881917020) and Task 2 verify 2 (row count, new-row shape, row-preservation diff)"
        status: pass
    human_judgment: false
  - id: D3
    description: "03-VALIDATION.md 03-11 row corrected, rows 03-12-T1, 03-12-T2 and 03-12-T3 added with commands, threat refs and status, evidence bullet, updated date and validation audit section"
    requirement: EQV-04
    verification:
      - kind: other
        ref: "Task 2 verify 3: row presence, run id in 03-12-T3, row-preservation diff, no path outside .planning changed after 03-12's SUMMARY commit (exit 0)"
        status: pass
    human_judgment: false
  - id: D4
    description: "Docs-only push with ci success for the pushed head"
    verification:
      - kind: other
        ref: "gh api ci run 41886479200 reads success 9f78ac43a374e1a98991d5b73a03fcc435a00196 (Task 2 verify 4)"
        status: pass
    human_judgment: false

duration: 7min
completed: 2026-11-04
status: complete
---

# Phase 3 Plan 13: Records for the safe_untar gap closure Summary

**T-03-42 closed and `threats_open: 0` in 03-SECURITY.md, CR-01/WR-01/WR-02/IN-01 recorded fixed (`open: 3`), and the validation map corrected, all on plan 03-12's 27-row test passing at HEAD and testbed run 41881917020 read with gh api.**

## Performance

- **Duration:** 7 min
- **Started:** 2026-11-04T18:19:33Z
- **Completed:** 2026-11-04T18:26:00Z
- **Tasks:** 2 (Tracer, Task 2)
- **Files modified:** 3 (docs only)

## Accomplishments

- Gap item 4, records half. 03-REVIEW-DISPOSITION.md: CR-01, WR-01, WR-02 and IN-01 are `fixed` in the frontmatter and the table, each with Source `03-12-SUMMARY.md (plan 03-12; testbed run 41881917020)`; `open: 3` (IN-10, IN-11, IN-12) and `total: 21` hold.
- 03-SECURITY.md: T-03-42 is `closed`. Its cell keeps the 2026-11-03 re-opening note and adds a dated closure naming the through-link rule, refuse_alias_up_through_link, refuse_alias_up_through_link_reordered and refuse_descend_through_link, the 27-row test, the GNU-tar second layer and the restore step's success in run 41881917020 (head `adfd2bd`). `threats_open: 0`, `status: verified`, the sign-off box is checked, and no row at or above high is open.
- T-03-43 now cites `--numeric-owner`, the owner-column check and the four rows refuse_forged_owner, refuse_forged_owner_listing, refuse_unreadable_target and refuse_listing_mismatch. T-03-44, T-03-45 and T-03-46 cite the plan 03-12 run (10 of 10 local links accepted, the seven vectors steps `success`, restore step 1 s). T-03-47..T-03-49 (plan 03-12) and T-03-50 (this plan) are appended after T-03-46 with evidence and status `closed`. A trust-boundary row and an audit row (51 / 51 / 0) were added.
- 03-VALIDATION.md: the 03-11-T1/T2 row says it proved the exact-name case only and that the alias shape stayed accepted until plan 03-12 (CR-01). Rows `03-12-T1, 03-12-T2` (27 archives, 24 GNU tar and 3 fake listings) and `03-12-T3` (the CI run) are added. The frontmatter `updated:`, an Evidence bullet and a `## Validation Audit 2026-11-04` section carry the date.
- The pushed head `9f78ac4` is docs only; ci run 41886479200 is `success` for it.

## Task Commits

1. **Tracer: CR-01 closed in all three records** - `68e3d7a` (docs)
2. **Task 2: remaining records, then the docs push** - `9f78ac4` (docs); pushed to `claude/gsd-discuss-phase-1-mzdw5v`, ci 41886479200 success

**Plan metadata:** the commit that adds this SUMMARY (docs: complete plan). It cannot name its own hash.

## Evidence read for this plan

| Check | Result |
|-------|--------|
| `gh api repos/sat-wik/migrationtool/actions/runs/41881917020` | `.github/workflows/testbed.yml adfd2bdbbe229b63df6d682efc48199447776599 success` |
| Jobs of that run | mt 126756279037, testbed 126756867942, vectors 126760859985 all success; the seven vectors evidence steps success; restore step 18:04:37Z to 18:04:38Z |
| `cargo test -p mt-harness --test eqv_04_workflow_static` | 14 passed, 0 failed; `eqv_04_workflow_static_safe_untar_accepts_in_tree_links_and_refuses_escaping_ones ... ok` |
| Cited labels in `crates/mt-harness/tests/eqv_04_workflow_static.rs` | all eight found (refuse_alias_up_through_link, refuse_alias_up_through_link_reordered, refuse_descend_through_link, accept_link_to_link, refuse_forged_owner, refuse_forged_owner_listing, refuse_unreadable_target, refuse_listing_mismatch) |
| Table split in 03-12-SUMMARY.md | 27 rows, 24 GNU tar and 3 fake listings, as planned |
| Row-preservation diffs against 00a706e | empty for the other findings, the other threat rows and the other validation rows |
| `git diff --name-only <03-12 SUMMARY commit> HEAD -- . ':!.planning'` | empty |
| ci for the pushed head | run 41886479200, `success 9f78ac43a374e1a98991d5b73a03fcc435a00196` |

## Decisions Made

- T-03-42 is closed on the evidence listed above and nothing else; the 2026-11-03 re-opening history is kept.
- The Evidence bullet in 03-VALIDATION.md names `68e3d7a` as the audited HEAD (the head at which the test was run; the second commit changed no code).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Forbidden-word grep in Task 2 verify 2 is too broad**
- **Found during:** Task 1 (Tracer), first run of the grep
- **Issue:** `! grep -rniE 'certif[i]ed|complia[n]t'` over the three whole files matches text that existed at 00a706e and that this plan must not change: the frontmatter key `nyquist_compliant` (03-VALIDATION.md lines 7 and 132, and the comment on line 5) and the 03-02-T3 row, which quotes the two words to say the contract document never contains them.
- **Fix:** Ran the check over the lines this plan adds (`git diff 00a706e -- <the three files>`, added lines only): no match. The plan's "rows not named are byte-identical" rule outranks the whole-file grep. Did not edit the pre-existing lines.
- **Files modified:** none
- **Verification:** `git diff 00a706e -- ... | grep '^+' | grep -niE 'certif[i]ed|complia[n]t'` prints nothing (exit 1).

---

**Total deviations:** 1 (Rule 3, verification scope). **Impact on plan:** none on the records; the plan's verify command should scan added lines only or exclude `nyquist_compliant` and the 03-02-T3 row.

Not deviations: the edits were made with short Python scripts that assert each anchor appears exactly once, instead of Edit calls; the audit row and the Approval line carry the date 2026-11-04.

## Issues Encountered

- The first `gh api` listing for the pushed head showed only the ci run (no testbed or container run), as expected for a docs-only push.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None.

## Threat Flags

None - docs only; no endpoint, auth path or trust-boundary file access changed.

## Next Phase Readiness

`/gsd-verify-work 3` can re-read the gap: item 1 (code and image) is closed by plan 03-12, and the records half of item 4 is closed here. Not done here by design: REQUIREMENTS.md, ROADMAP.md status marks for EQV-01..EQV-04 and 03-VERIFICATION.md belong to the re-verification; IN-10, IN-11, IN-12 stay `open` and WR-03 stays `skipped`. The human-verification item (IN-11, the contract wording) remains a founder decision.

## Self-Check: PASSED

- Records files exist: 03-REVIEW-DISPOSITION.md, 03-SECURITY.md, 03-VALIDATION.md (found).
- Commits `68e3d7a` and `9f78ac4` are ancestors of HEAD; ci for `9f78ac4` is success.
- Acceptance criteria re-run: Tracer verify 1 and 2, Task 2 verify 1, 2 (with the added-lines forbidden-word scope), 3 and 4 all exit 0.

---
*Phase: 03-equivalence-contract-and-test-vector-schema*
*Completed: 2026-11-04*
