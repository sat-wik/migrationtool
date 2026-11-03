---
phase: 03-equivalence-contract-and-test-vector-schema
verified: 2026-11-03T23:42:13Z
status: gaps_found
score: 12/13 must-haves verified
covered_files:
  - ".github/workflows/testbed.yml"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-01-PLAN.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-01-SUMMARY.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-02-PLAN.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-02-SUMMARY.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-03-PLAN.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-03-SUMMARY.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-04-PLAN.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-04-SUMMARY.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-05-PLAN.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-05-SUMMARY.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-06-PLAN.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-06-SUMMARY.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-07-PLAN.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-07-SUMMARY.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-08-PLAN.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-08-SUMMARY.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-09-PLAN.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-09-SUMMARY.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-10-PLAN.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-10-SUMMARY.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-11-PLAN.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-11-SUMMARY.md"
  - "crates/mt-harness/tests/eqv_04_workflow_static.rs"
covered_digest: "v3:sha256:d650f6b09e3a17900516f5cc0396126f67c9d484ca5f758c344244a1e37e7cf6"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 3/4
  gaps_closed:
    - "Gap item 1, over-refusal half: safe_untar no longer refuses every link target with a `..` component; the CI-observed nanopb and nimble in-tree links are accepted (workflow's own function accepted a tar of the local capture and run trees: exit 0, 10 of 10 links extracted)"
    - "Gap item 2: a behavioural test now runs the workflow's own safe_untar (extracted with function_body, no copy) on 19 real GNU-tar archives; passes at HEAD (14 of 14 tests in the file, 557 in the workspace)"
    - "Gap item 3: testbed run 41735869792 on 752badf, the last trigger-path commit, is success; the vectors job and all seven named evidence steps are success, none skipped (read with gh api)"
  gaps_remaining:
    - "Gap item 1, soundness half (plan 03-11 must-have truth 1): the guard does not refuse a link that reaches a `..` through a second link aliasing a directory that holds an archive link (CR-01)"
  regressions: []
gaps:
  - truth: "Plan 03-11 must-have truth 1 / EQV-04 gap item 1: safe_untar refuses a symlink when its target, joined to the link's own directory and walked component by component, pops past the top of the tree or pops a component that is itself a symlink of the same archive"
    status: partial
    reason: "The in-tree half is fixed and shown in the image. The refusing half has a bypass that I reproduced with the workflow's own function (extracted from .github/workflows/testbed.yml at HEAD, bash 5.2, GNU tar 1.35). Archive members in order: m/ m/n/ m/n/d/ (dirs), m/n/d/p -> ../../.. (resolves to the top of the tree, accepted like accept_up_to_top), X -> m/n (all pushes), e -> X/d/p/.. . The function exits 0 and extracts. readlink -f dest/e is the parent of the destination, which in the workflow is $RUNNER_TEMP (holds ci-lib.sh, sourced by every later step). Cause: the first pass records links by exact normalised name (islink[m/n/d/p]), the walk pops-checks only the exact joined stack, and X/d/p is not a key, so the pop of p (which is a symlink of the same archive) is taken for the pop of a real directory. The truth's literal wording ('pops a component that is itself a symlink of the same archive') is false for this input. Exploitability today is bounded by GNU tar's own refusal: a follow-up member e/pwn.txt made GNU tar 1.35 fail with 'Cannot open: Not a directory' (reproduced), so no write outside the destination was observed. That is tar's behaviour, not the guard's, and the escaping link stays in $RUNNER_TEMP/work. The register treats it as a failed mitigation of T-03-42 (high, threats_open 1, block_on high). The 19-row table has no row for this shape, so the suite and CI stay green on a guard that accepts it (review WR-02)."
    artifacts:
      - path: ".github/workflows/testbed.yml"
        issue: "safe_untar second pass (lines 1121-1139): a non-`..` component is pushed onto the stack without checking whether the joined stack is an archive link, so a path that descends through an aliasing link (X/d/p) hides the link p from the later pop-of-link test. The comment at lines 1034-1048 claims the walk fails closed where it cannot tell where a path lands; it does not for this shape."
      - path: "crates/mt-harness/tests/eqv_04_workflow_static.rs"
        issue: "The table (about lines 1423-1593) has no alias row; refused link rows all assert the fragment 'leaves the tree', which the member-name message also contains; the test checks that dest is empty but not that nothing outside dest changed; the branches 'cannot read the target of the symlink' and 'tar -tf and tar -tvf list different members' have no row."
    missing:
      - "In safe_untar's second pass, before a non-`..` component is pushed onto a non-empty stack, join the stack and look it up in `islink`; if it is an archive link, set `left=1` (refuse). This makes the logical path equal the physical path, so the existing pop-of-link test is exact. The link's own dirname components pass through the same branch, so they are covered. I applied exactly this to a scratch copy of the extracted function: the CR-01 archive then exits 1 with '::error::... holds a symlink that is absolute or leaves the tree: e -> X/d/p/..', and the local capture and run trees still exit 0 with 10 of 10 links extracted. Rewrite the comment above the function to state the real rule: a link target may not descend into or pop an archive symlink."
      - "Add test rows, built as real tars like the existing ones: `refuse_alias_up_through_link` with the six CR-01 members in that order (exit 1, empty dest), and one row where a target descends through an archive link without any `..` after it (for example `X -> m/n`, `c -> X/y`), to pin the fail-closed decision. Give the link-walk refusal a message fragment that the member-name refusal does not share (assert 'holds a symlink' on link rows). Put a sentinel file beside `dest` in check_untar_case and assert the directory outside `dest` is unchanged after every row."
      - "Close review WR-01 in the same edit (same function, same register entry T-03-43): list with `tar --numeric-owner -tvf` so the owner column is digits and a slash and cannot contain ` -> `; add a row with a forged uname (Python tarfile `uname='u a -> p/q/r/s/t'` on `a -> ../../../../etc`, directories p/q/r/s/t present). I reproduced the forgery: the current guard exits 0 on that archive."
      - "Push, obtain a green testbed run for the new last trigger-path head (gh api: run success; vectors steps 'Unpack the captures of the testbed job', 'Build the reference drivers', 'Publish the state layouts', 'Run the reference vectors twice', 'Run the vectors again and compare (vectors determinism)', 'Fixture vectors sample', 'Replay the committed sample' each success), write 03-12 notices, then set CR-01, WR-01 and WR-02 to fixed in 03-REVIEW-DISPOSITION.md, close T-03-42 (and re-evidence T-03-43) in 03-SECURITY.md, and update the 03-11 rows of 03-VALIDATION.md, whose claim of a '`..` through an archive link' refusal holds only for the exact-name case."
deferred: []
advisory: []
behavior_unverified_items: []
human_verification:
  - test: "Re-read the three passages of docs/specs/equivalence-contract.md that changed after the founder's UAT item 1 (commit dd1deeb, 2026-11-01): the `rc` row, the `ret` bullet under Library state, and the Byte codec paragraph"
    expected: "The new wording matches the decisions it restates (D-05 'rc: the C return value', D-06, D-25 Latin-1 identity). Specifically: (a) a struct or union return goes to lib_state_out.ret with rc 0; (b) an integer or enum return wider than 32 bits is now refused when the adapter is built (before this it was promised to go to `ret`, but the code never did that and truncated it before CR-01); (c) the codec paragraph now names serde_json escapes (short escapes for \\b \\t \\n \\f \\r, \\u00XX for other control characters, other characters as UTF-8) and says the vector file is serde_json::to_vec_pretty output with sorted maps and one trailing newline"
    why_human: "UAT item 1 (pass, 2026-10-30) judged the earlier text. The document changed afterwards, still under version 0.1.0 with no changelog line (review IN-11). The change narrows a promise rather than adding a rule, and no committed vector or policy depends on the withdrawn promise (checked: testbed/policy and testbed/vectors-sample are unchanged since the UAT). Whether a correction without a version bump is acceptable is a founder call."
---

# Phase 3: Equivalence Contract and Test-Vector Schema Verification Report

**Phase Goal:** "Equivalent" is written down and machine-checkable, and the original C can be run against any test vector to produce a deterministic reference result.
**Verified:** 2026-11-03T23:42:13Z (HEAD 70e28ee)
**Status:** gaps_found
**Re-verification:** Yes, after gap-closure plan 03-11. Previous report: gaps_found, 3/4.

Plan 03-11 did what it said for the CI failure. The vectors job is green again in the pinned image with every evidence step run, and the executor, contract, schema and adapters are unchanged and still pass. But the guard that plan 03-11 rewrote does not meet its own central must-have: it accepts a link that resolves outside the destination (CR-01, reproduced here). The gap is therefore narrowed, not closed.

## Decision on CR-01

Question put to this verification: does CR-01 leave Truth 4, or only plan 03-11's must-haves, partial?

- **ROADMAP criterion 4 (Truth 4) is VERIFIED.** It says the executor runs a vector against the original C build, stores a schema-valid result, and gives byte-identical results on a second run, shown in the pinned image. All of that holds at HEAD, and the in-image demonstration that was missing is back (testbed run 41735869792, below). CR-01 sits in a hand-off defence around the tar of the build output. It does not stop the executor, the sample replay or the determinism check from running, and it did not stop them in the run. The previous report put the broken step under Truth 4 only because a red restore step skipped the demonstration. That reason is gone.
- **Plan 03-11 must-have truth 1 is FAILED (partial).** It promises a refusal when a link "pops a component that is itself a symlink of the same archive". In CR-01, `e -> X/d/p/..` pops `p`, which is `m/n/d/p`, a symlink of the same archive, and the guard exits 0. The sentence is false for a reproduced input. The truth's own gap item, "refuse only a symlink that is absolute or whose target ... leaves the tree", has the same defect on its refusing side.
- **Consequence:** the verdict is `gaps_found`, not `passed`. T-03-42 is a high-severity threat re-opened with a reproduced bypass, and its `block_on` gate counts it (`threats_open: 1`). I count it against the must-haves and not as an advisory, because the behaviour that is wrong is the one the plan wrote down as a truth and the register lists as the mitigation. I do not claim an exploit: GNU tar 1.35 refuses to write through the escaping link (reproduced), so what remains is a guard whose stated contract is wrong, one tar-version dependency away from a write outside the destination.
- Counted separately: review WR-01 (the owner column can forge the parsed target) is a second, lower-severity hole in the same function. I reproduced it too (forged uname, guard exits 0 on `a -> ../../../../etc`). It does not touch a plan truth word for word, but it makes T-03-43's evidence ("target is what follows that exact name") overstated. It is listed in the same gap as a recommended fix in the same edit.

## Goal Achievement

### Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Contract v0: versioned document and loaded policy file; refinement, observation point, UB menu with return error as default and a recorded way to change it | VERIFIED (regression check) | `mt vectors policy-check --manifest testbed/manifest.toml`: 16 lines `valid default=return_error overrides=0`. Only `.github/workflows/testbed.yml` and the workflow test changed outside `.planning` since the previous report (`git diff --name-only 286baca HEAD -- . ':!.planning'`), so the evidence of the previous report stands. |
| 2 | Nine-field schema validates and round-trips; merged `lib_state` rejected; at least 3 TRACTOR vectors load unchanged | VERIFIED (regression check) | Same unchanged code. Workspace suite 557 passed, 0 failed (includes the `eqv_02_*` group). Previous tampered-copy checks stand. |
| 3 | At least 10 test-bed targets have an entry-point adapter with argument roles | VERIFIED (regression check) | `mt vectors adapters --manifest testbed/manifest.toml --check-floor`: "drivable targets: 15 (floor 10)", "result: passed (27 rows)". |
| 4 | The C reference executor runs any vector against the target's original C build, stores the result in a schema-valid vector, and running it twice gives byte-identical results (shown in the pinned image by the testbed workflow) | VERIFIED | Executor and local evidence unchanged and green (557 default tests). In-image: testbed run 41735869792 (head 752badf), see CI evidence. Restore, build, layouts, twice-run vectors, determinism rerun, sample fixture and committed-sample replay all ran and succeeded. The guard defect (gap below) does not bear on this criterion. |

### Plan 03-11 must-haves

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| P1 | safe_untar refuses a symlink only when absolute, or when the walked target pops past the top or pops a component that is itself a symlink of the same archive; CI-observed nanopb and nimble links accepted | FAILED (partial) | In-tree half verified: rows accept_nanopb_ci and accept_nimble_ci pass, and the function accepts the local capture and run trees (10 of 10 links). Refusing half false for the alias shape of CR-01 (reproduced; see gap). |
| P2 | Rest of WR-04 kept: non file/dir/symlink refused; absolute or `..` member names refused; nothing extracted before the listing passes; one extraction with `--no-same-owner --no-same-permissions`; both archives through safe_untar | VERIFIED | Code read at lines 1049-1146: `grep -v -E '^[-dl]'` check, `case "/$name/"` member-name check, single `tar -xf ... --no-same-owner --no-same-permissions` after both passes, two call lines (`grep -c 'safe_untar "$RUNNER_TEMP/handoff/'` prints 2). |
| P3 | Behavioural test runs the workflow's own function on real tars, with the listed accept and refuse rows | VERIFIED (as written) | `eqv_04_workflow_static_safe_untar_accepts_in_tree_links_and_refuses_escaping_ones ... ok`; all 19 labels are present; the test extracts the function with `function_body` and has no copy of the guard. It does not pin CR-01 (part of the gap). |
| P4 | The testbed run for the last trigger-path commit concludes success and the seven vectors steps are success, none skipped | VERIFIED | gh api: run 41735869792 path `.github/workflows/testbed.yml`, head 752badfc854dbcf4e3b27dfc3971f23798915d6d, completed, success. `git log -1 -- testbed crates Cargo.toml Cargo.lock container/pins.toml container/README.md .github/workflows/testbed.yml` is 752badf. |
| P5 | All committed corpora replay `identical 1` in the pinned image; rerun identical; at least 10 targets twice=identical; testbed notices equal 02-22 from line 5 | VERIFIED | `03-11-VECTORS-NOTICES.md`: 15 lines ending `identical 1` (15 `index.json` under testbed/vectors-sample), `rerun rows are identical to the first run's rows`, `identical 5768 files`. `diff <(sed '1,4d' 02-22-CI-NOTICES.md) <(sed '1,4d' 03-11-CI-NOTICES.md)` is empty. These are the notice texts the CI run produced; I read the run and step conclusions with gh api, not the annotation text. |
| P6 | Edge boundary: `a/up -> ..` accepted; `a/up2 -> ../..`, `a -> ../x`, `a -> b/../../x` refused | VERIFIED | Rows accept_up_to_top, refuse_one_past_top, refuse_top_level_up, refuse_down_then_up pass. |
| P7 | Edge adjacency: dot-only components and `.` handled; names containing ` -> ` read by exact `tar -tf` name | VERIFIED | Rows accept_dot_components, accept_arrow_in_name, refuse_arrow_in_name pass. (The owner-column forgery, WR-01, is a different route into the same parse and is in the gap.) |
| P8 | Edge ordering: verdict independent of member order; `d/up -> ..` then `e -> d/up/..` refused; one extraction after every member passed | VERIFIED for the shape stated | refuse_up_through_link passes; the first pass records all links before the second pass walks. The alias variant is the gap (P1). |
| P9 | Edge empty: an archive with no members refused with 'no members' | VERIFIED | Row refuse_empty_archive passes. |

Prohibitions of plan 03-11 (all three, test tier, all hold): no `continue-on-error:` key in the workflow (grep empty), the region outside the restore step is byte-identical to 83352a1 (diff empty), the two call lines are exact (asserted by the static test), `git diff --name-only 83352a1 HEAD -- . ':!.planning'` prints only the workflow and the workflow test, `testbed/vectors-sample` is unchanged, and the closure claim rests on a gh api read of the run, not on local runs.

**Score:** 12/13 (4 ROADMAP criteria + 9 plan 03-11 truths; the one failed is P1). 0 behavior-unverified.

### CI evidence (GitHub, read with `gh api` only)

Run 41735869792, `.github/workflows/testbed.yml`, head 752badfc854dbcf4e3b27dfc3971f23798915d6d, completed, success. Jobs: `mt` 126264790431 success, `testbed` 126265405983 success, `vectors` 126268645422 success. Same head: `ci` 41735869789 success, `container` 41735869785 success (listed by `actions/runs?head_sha=`).

Vectors job steps, from `.jobs[] | select(.name=="vectors") | .steps[]`, all `success`: Set up job, checkout, download-artifact, Make mt executable, Write the annotation and docker helpers, Pull the image by digest, Run the executor self-test in the pinned image, Fetch the TRACTOR sample and load it unchanged, Download the captures of the testbed job, **Unpack the captures of the testbed job**, **Build the reference drivers**, **Publish the state layouts**, **Run the reference vectors twice**, **Run the vectors again and compare (vectors determinism)**, **Fixture vectors sample** (and parts 10 to 18, 19 on), **Replay the committed sample**, Post checkout, Complete job. None skipped.

Scope of this evidence: it shows the restore step accepts the real capture tars and that every later step then ran, including the WR-01 provenance comparison of 90767fd against the committed sample. It cannot show that the guard refuses hostile archives. A green run is also green for CR-01.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `.github/workflows/testbed.yml` (restore step) | safe_untar that vets each symlink by where it resolves | PARTIAL | Exists, substantive, wired (two call lines, step `restore` gates the later steps by `steps.restore.outcome == 'success'`), in-image green. Rule unsound for aliased links (gap). |
| `crates/mt-harness/tests/eqv_04_workflow_static.rs` | Behavioural test on real tars, 19 labels | VERIFIED as written; incomplete | Present, wired to the workflow through `function_body(&step_text(&vectors_job(), RESTORE_STEP), "safe_untar")`, passes. Lacks the alias row and an outside-the-destination check. |
| `03-11-CI-NOTICES.md`, `03-11-VECTORS-NOTICES.md` | Notices of the green run | VERIFIED | Present; checks above. |
| Everything else (contract, policies, schema, adapters, executor, sample, Dockerfile) | | VERIFIED (unchanged) | No change outside the two files since the previous report. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| testbed job `work.tar`, `replay.tar` | vectors job restore step | artifact `vectors-input`, `safe_untar` | WIRED | Run 41735869792: restore success. |
| restore step | build, run, determinism, sample, replay steps | `if: steps.restore.outcome == 'success'` | WIRED | All ran and succeeded. |
| workflow `safe_untar` | behavioural test | `function_body` | WIRED | |
| manifest roles, driver, vector files, policy, TRACTOR pin | | | WIRED | Unchanged; in-image run now covers them again. |

### Data-Flow Trace (Level 4)

Unchanged from the previous report (committed sample vectors from qemu-arm runs of captured objects: FLOWING). The in-image replay of all 15 committed corpora is now evidenced.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| fmt, clippy, workspace tests | `cargo fmt --check`; `cargo clippy --all-targets -- -D warnings`; `cargo test --workspace --locked` | fmt ok, clippy finished clean, 557 passed 0 failed | PASS |
| Workflow guard test | `cargo test -p mt-harness --test eqv_04_workflow_static` | 14 passed | PASS |
| Guard on local capture and run trees | workflow function extracted with awk, `set -euo pipefail`, on `tar -cf ... -C .mt-work capture run` | exit 0, 10 links extracted | PASS |
| Guard on CR-01 archive (`m/n/d/p -> ../../..`, `X -> m/n`, `e -> X/d/p/..`) | same extracted function | exit 0; `readlink -f dest/e` is the parent of dest | FAIL (should refuse) |
| Write through the escaping link | add member `e/pwn.txt` and extract | GNU tar 1.35: `Cannot open: Not a directory`; nothing written outside | tar's own layer holds |
| Guard on forged-uname archive (WR-01) | `a -> ../../../../etc` with `uname='u a -> p/q/r/s/t'` | exit 0 | FAIL (should refuse) |
| Proposed CR-01 fix in a scratch copy of the extracted function | push-through-link check | CR-01 archive exit 1; local trees exit 0 with 10 links | PASS (fix is viable; not applied to the repository) |
| Policies, adapter floor | see Truths 1 and 3 | 16 valid, 15 drivable | PASS |
| Debt markers in the two changed files | grep for TBD/FIXME/XXX | none found | PASS |

Not run here: `cargo deny check`, the ignored local proofs (the files changed since the last full run are a workflow and its static test; the ignored group covers the executor, which is unchanged).

### Probe Execution

SKIPPED. Phase 3 declares no `probe-*.sh`; the in-image probe is `mt vectors selftest`, which passed in the run above.

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| EQV-01 | 03-02 | Versioned contract and policy | SATISFIED | Truth 1 |
| EQV-02 | 03-03 | Nine-field schema, round trip, merged `lib_state` rejected | SATISFIED | Truth 2 |
| EQV-03 | 03-04 to 03-08 | Adapter declares entry and roles; at least 10 drivable | SATISFIED | Truth 3 (15) |
| EQV-04 | 03-01, 03-04..03-07, 03-09, 03-10, 03-11 | Reference executor, schema-valid result, identical on a second run | SATISFIED for the requirement text (Truth 4 verified in the image); phase carries an open hand-off-guard gap | Truth 4; gap above |

Plan frontmatter `requirements:` read from all eleven plans: EQV-04 (03-01, 03-09, 03-10, 03-11), EQV-01 (03-02), EQV-02 (03-03), EQV-03 and EQV-04 (03-04 to 03-07 and 03-08). All four IDs appear in REQUIREMENTS.md and nothing else maps to Phase 3, so there are no orphans.

Bookkeeping to fix when the gap closes: REQUIREMENTS.md still has EQV-01, EQV-02 and EQV-03 unchecked with traceability "Gaps Found", while EQV-04 is checked "Complete". The first three are satisfied by this report's evidence and are stale. The EQV-04 mark was set while the guard gap was open; leave it but do not treat phase 3 as shippable until the gap closes.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `.github/workflows/testbed.yml` | 1121-1139 (comment 1034-1048) | Walk descends through an aliasing link without checking it; comment claims fail-closed | BLOCKER | The gap (CR-01). |
| `.github/workflows/testbed.yml` | 1109 | `${line#*" $name -> "}` is unanchored; the owner column can supply the match (WR-01) | WARNING | Reproduced; builds on trusted owner names today. Same function, fix with `--numeric-owner`. |
| `crates/mt-harness/tests/eqv_04_workflow_static.rs` | ~1361, ~1423-1593 | No alias row; shared message fragment `leaves the tree`; no outside-the-destination assertion; two error branches without rows (WR-02) | WARNING | The suite passes on an unsound guard. |
| `crates/mt-harness/tests/eqv_04_workflow_static.rs` | ~1261 | `host_tar` checks only existence, not GNU tar or bash 4.3 (IN-01) | INFO | Linux-only project; low priority. |
| `crates/mt-harness/src/runtime.c` | 257-262, 292-296 | WR-03 `mt_ksigaction` layout (skipped) | WARNING | Unchanged from the previous report; needs a CI sample regeneration. |
| `runtime.c`, `replay.rs`, contract doc | | IN-01 (MAP_FIXED), IN-10, IN-11, IN-12 | INFO | Unchanged, open, tracked in the disposition. |
| TBD/FIXME/XXX, `continue-on-error` | n/a | None in the changed files | none | Debt-marker gate passes. |

No use of the two forbidden claim words in the files checked.

### Human Verification Required

#### 1. Re-read three passages of the contract that changed after the UAT

**Test:** In `docs/specs/equivalence-contract.md`, read the `rc` row of the field table, the `ret` bullet under reserved `lib_state` names, and the Byte codec paragraph (commit dd1deeb).
**Expected:** They match D-05, D-06 and D-25. A return wider than 32 bits is no longer promised to go to `lib_state_out.ret`; it is refused when the adapter is built. A struct or union return goes to `ret` with rc 0. The codec paragraph describes serde_json's escapes and the exact file format.
**Why human:** UAT item 1 judged the earlier wording; the edit was made under version 0.1.0 with no changelog line (IN-11). No file under `docs/` changed since the previous report, so this item is not resolved. Accepting an unversioned contract correction is the founder's call.

### Gaps Summary

One gap, narrowed from the previous report. The three missing items of the old gap are evidenced as follows: item 2 (behavioural test) closed; item 3 (green vectors job with all evidence steps, read with gh api for the last trigger-path head) closed; item 1 closed on the over-refusal it described, open on soundness of the refusal that plan 03-11 wrote into its first must-have.

Root cause: the walk models the archive as a tree of directories plus a set of link names, and assumes a path reaches a link only by that link's own name. A second link that aliases a directory holding a link breaks the assumption. The reviewer's fix (refuse any push onto a stack that is an archive link, so the walk never descends through a link) is small, I checked it on a scratch copy, and it keeps every accepted shape (nanopb, nimble, the 10 local links, the accept rows). It needs a code and test change in the same two files, one new push, and one green testbed read with gh api. Deferred items: none; no later phase of the milestone covers this.

The phase is not shippable until T-03-42 is closed. WR-01 and WR-02 of the review should go in the same plan, since they are in the same function and test.

Limits of this verification: no pinned image on this host (not needed for the guard checks, which are host bash and GNU tar, the same versions as the ubuntu-24.04 runner per the plan); `cargo deny check` and the ignored local proofs were not re-run; annotation text of the CI run was not re-read (conclusions only); the committed notices were compared with 02-22 as files.

---

_Verified: 2026-11-03T23:42:13Z_
_Verifier: Claude (gsd-verifier)_
