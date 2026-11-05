---
phase: 03-equivalence-contract-and-test-vector-schema
verified: 2026-11-04T18:42:10Z
status: passed
score: 27/27 must-haves verified
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
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-12-PLAN.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-12-SUMMARY.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-13-PLAN.md"
  - ".planning/phases/03-equivalence-contract-and-test-vector-schema/03-13-SUMMARY.md"
  - "crates/mt-harness/tests/eqv_04_workflow_static.rs"
covered_digest: "v3:sha256:0e86dc6342fd69369e1d3b6db1bae5d1c67ff8ca31d7e696b9efa30682ea9eb9"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 12/13
  gaps_closed:
    - "Plan 03-11 must-have truth 1 / CR-01: safe_untar now refuses a link target that steps into or out of an archive symlink. The CR-01 archive (m/, m/n/, m/n/d/, m/n/d/p -> ../../.., X -> m/n, e -> X/d/p/..) exits 1 in both member orders. Reproduced with the workflow's own function extracted at HEAD."
    - "Review WR-01 (owner-column forgery): the listing is tar --numeric-owner -tvf with an owner_re check. A real GNU tar archive with uname 'u a -> p/q/r/s/t' on a -> ../../../../etc exits 1 at HEAD and exits 0 on the 752badf function (both run by me)."
    - "Review WR-02 (table did not pin CR-01, shared fragments, no outside-dest check): 27 rows, distinct fragments, sentinel snapshot around every row. Mutation checks below show the table fails when the fixed code is broken."
    - "Review IN-01 (host tools): assert_host_tools runs first with no skip path."
    - "Gap item 4, CI half: testbed run 41881917020 on adfd2bd (the last trigger-path commit) is success; the vectors job and all seven named evidence steps are success, none skipped."
    - "Gap item 4, records half: 03-REVIEW-DISPOSITION.md, 03-SECURITY.md and 03-VALIDATION.md updated by plan 03-13; every cited run, label and count re-read here."
  gaps_remaining: []
  regressions: []
gaps: []
deferred: []
advisory: []
behavior_unverified_items: []
human_verification:
  - test: "Re-read the three passages of docs/specs/equivalence-contract.md that changed after the founder's UAT item 1 (commit dd1deeb, 2026-11-01): the `rc` row, the `ret` bullet under Library state, and the Byte codec paragraph"
    expected: "The new wording matches the decisions it restates (D-05 'rc: the C return value', D-06, D-25 Latin-1 identity). (a) A struct or union return goes to lib_state_out.ret with rc 0. (b) An integer or enum return wider than 32 bits is refused when the adapter is built (earlier text promised it would go to `ret`, but the code never did that). (c) The codec paragraph names serde_json escapes and says the vector file is serde_json::to_vec_pretty output with sorted maps and one trailing newline."
    why_human: "UAT item 1 (pass, 2026-10-30) judged the earlier text. The document changed afterwards under version 0.1.0 with no changelog line (review IN-11, still open in 03-REVIEW-DISPOSITION.md). No file under docs/ has changed since the previous report (git diff --name-only 70e28ee HEAD -- docs prints nothing), so this item is unresolved. Whether an unversioned correction is acceptable is a founder call."
---

# Phase 3: Equivalence Contract and Test-Vector Schema Verification Report

**Phase Goal:** "Equivalent" is written down and machine-checkable, and the original C can be run against any test vector to produce a deterministic reference result.
**Verified:** 2026-11-04T18:42:10Z (HEAD 84b1240; last code commit adfd2bd)
**Status:** human_needed
**Re-verification:** Yes, after gap-closure plans 03-12 and 03-13. Previous report: gaps_found, 12/13.

The one recorded gap is closed. The guard that plan 03-11 left unsound (CR-01) now refuses the alias shape, I reproduced the refusal with the workflow's own function, and the in-image evidence exists for the corrected code. The only open item is the same founder decision the previous report carried (unversioned contract wording), which the decision tree routes to `human_needed`. No truth failed.

## Goal Achievement

### Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Contract v0: versioned document and loaded policy file; refinement, observation point, UB menu with return error as default and a recorded way to change it | VERIFIED (regression) | `mt vectors policy-check --manifest testbed/manifest.toml` printed 16 lines `valid default=return_error overrides=0`. Only the workflow and its static test changed outside `.planning` since the previous report (`git diff --name-only 70e28ee HEAD -- . ':!.planning'`). |
| 2 | Nine-field schema validates and round-trips; merged `lib_state` rejected; at least 3 TRACTOR vectors load unchanged | VERIFIED (regression) | Same code. `cargo test --workspace --locked`: 557 passed, 0 failed, 23 ignored. |
| 3 | At least 10 test-bed targets have an entry-point adapter with argument roles | VERIFIED (regression) | `mt vectors adapters --manifest testbed/manifest.toml --check-floor`: "drivable targets: 15 (floor 10)", "result: passed (27 rows)". |
| 4 | The C reference executor runs any vector against the target's original C build, stores a schema-valid result, and a second run is byte-identical (shown in the pinned image) | VERIFIED | Executor, adapters and sample are unchanged since 752badf (`git diff --name-only 752badf HEAD -- testbed crates/*/src crates/mt-harness/src/runtime.c container` is empty). In-image: testbed run 41881917020 on adfd2bd, below. |

### Plan 03-12 must-haves (the gap closure)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | The walk refuses a target that is absolute, pops an empty stack, or meets a stack that names an archive symlink before any push or pop | VERIFIED | Code read at testbed.yml 1131-1163: `why` set to `is absolute or leaves the tree` or `passes through a symlink of the archive`; `islink` lookup runs before every push or pop on a non-empty stack. |
| 2 | CR-01's archive is refused in either member order, with the right message, empty dest, unchanged case dir | VERIFIED | My run of the extracted function on real GNU tar archives: `cr01` exit 1, `cr01_rev` exit 1, message `holds a symlink that passes through a symlink of the archive`. The test rows `refuse_alias_up_through_link` and `refuse_alias_up_through_link_reordered` pass. |
| 3 | Fail-closed price pinned and stated; link to link accepted; plan 03-11 accept rows kept; local capture and run trees accepted with every link extracted | VERIFIED | `descend` (X -> m/n, c -> X/y) exit 1; `l2l` (a -> b, b -> c/..) exit 0; comment at lines 1046-1059 states the rule and price. Local trees: exit 0, 10 links in source, 10 extracted. |
| 4 | WR-01: `--numeric-owner -tvf`; a non-digits owner column is refused; forged uname on a real tar is refused | VERIFIED | Line 1067 `listing="$(tar --numeric-owner -tvf "$archive")"` appears once; `owner_re` at 1066, used at 1122. My forged-owner archive (`--owner='u a -> p/q/r/s/t:0'`): HEAD function exit 1 with `holds a symlink that is absolute or leaves the tree: a -> ../../../../etc`; the 752badf function exit 0 on the same file. |
| 5 | WR-02: each refused row asserts a fragment only its own refusal prints; the two GNU-tar-unreachable branches are driven by a fake tar on that row's PATH only; outside-dest snapshot after every row | VERIFIED, with a wording caveat (see Anti-Patterns, review IN-01) | The table has exactly 27 rows (failure output in my mutation runs reads "of 27 rows"). The two rows `refuse_forged_owner_listing` and `refuse_unreadable_target` assert the same fragment, printed by two different branches, so "only its own refusal" is not literally true for that pair. In substance each row detects its own branch: removing the owner check fails `refuse_forged_owner` and `refuse_forged_owner_listing`; removing the no-match check fails `refuse_unreadable_target` alone (mutants 2 and 5). The plan prescribed the shared fragment in its own behavior block. |
| 6 | IN-01: GNU tar and bash 4.3 asserted before the table, no skip path | VERIFIED | `fn assert_host_tools` present; the test passes on this host (GNU tar 1.35, bash 5.2.21). |
| 7 | Kept from WR-04 and 03-11: type check, member-name check, empty archive, nothing extracted before both passes, one extraction with `--no-same-owner --no-same-permissions`, two call lines, workflow outside the restore step identical to 752badf | VERIFIED | Lines 1069-1082 and 1164; `grep -c 'safe_untar "$RUNNER_TEMP/handoff/'` prints 2; region diff against 752badf (awk excluding the restore step) is empty; no `|| true` after either call; `continue-on-error:` as a key: 0 (one comment mention). |
| 8 | Testbed run for the last trigger-path commit is success; seven named vectors steps success, none skipped | VERIFIED | gh api: run 41881917020, path `.github/workflows/testbed.yml`, head adfd2bdbbe229b63df6d682efc48199447776599, completed, success. `git log -1 -- testbed crates Cargo.toml Cargo.lock container/pins.toml container/README.md .github/workflows/testbed.yml` is adfd2bd. Vectors job 126760859985 success; every step success (list below). |
| 9 | Corpora replay `identical 1`; testbed notices equal 02-22 from line 5; vectors notices equal 03-11 from line 5 | VERIFIED | `03-12-VECTORS-NOTICES.md`: 15 `identical 1` lines, the rerun line once, `identical 5768 files`. `diff <(sed '1,4d' 03-11-VECTORS-NOTICES.md) <(sed '1,4d' 03-12-VECTORS-NOTICES.md)` and the same for 02-22 vs `03-12-CI-NOTICES.md` are empty. The notices are the files the plan wrote from annotations; I read run and step conclusions with gh api, not the annotation text. |
| 10 | Edge boundary: `a/up -> ..` accepted, `a/up2 -> ../..` refused | VERIFIED | My cases `upone` exit 0, `uptwo` exit 1. |
| 11 | Edge adjacency: link naming another link as last component accepted, stepping through it refused; names holding ` -> ` read by exact name | VERIFIED | `l2l` exit 0, `descend` exit 1; table rows `accept_arrow_in_name`, `refuse_arrow_in_name` pass. |
| 12 | Edge ordering: verdict independent of member order | VERIFIED | `cr01` and `cr01_rev` both exit 1; the first pass records every link before the second pass walks any. |
| 13 | Edge empty: an archive with no members is refused | VERIFIED | Row `refuse_empty_archive` passes; line 1069-1072. |

### Plan 03-13 must-haves (records)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| R1 | CR-01, WR-01, WR-02, IN-01 `fixed` in frontmatter and table with a Source naming 03-12-SUMMARY.md and the run; `open: 3`; `total: 21`; no other row changed | VERIFIED | Table rows read `fixed | 03-12-SUMMARY.md (plan 03-12; testbed run 41881917020)`; 3 entries still at `disposition: open` (IN-10, IN-11, IN-12) and `open: 3`; the row-preservation diff against 00a706e is empty. |
| R2 | T-03-42 `closed` with rows, test, run id and head; the 2026-11-03 re-opening note kept | VERIFIED | The T-03-42 row ends `closed`, contains `Re-opened 2026-11-03`, `Closed 2026-11-04 (plan 03-12)`, the three row labels and 41881917020. |
| R3 | `threats_open: 0`, `status: verified`, sign-off box checked, no open row at or above high | VERIFIED | Frontmatter reads both; line 149 is `- [x]`; zero `open` rows among critical or high rows. |
| R4 | T-03-43 re-evidenced; T-03-44..46 cite the run; T-03-47..50 registered; audit row 51 | VERIFIED | 51 `| T-03-` rows; T-03-47..50 each have severity, disposition and `closed`; T-03-43 names `--numeric-owner` and `refuse_forged_owner_listing`; the diff of all rows outside T-03-42..T-03-50 against 00a706e is empty. |
| R5 | 03-11 validation row limited to the exact-name case; 03-12-T1/T2 and 03-12-T3 added; `updated:` dated | VERIFIED | The 03-11 row says "This row proves the exact-name case only ... stayed accepted until plan 03-12 (code review CR-01)"; the flagged clause is gone (0 matches); the two new rows exist (27 archives: 24 GNU tar, 3 fake listings, which matches the test file); `updated: "2026-11-04"`; other validation rows identical to 00a706e. |
| R6 | Every citation resolves at HEAD | VERIFIED | Run, jobs and conclusions re-read with gh api; all eight cited labels in the test file; the test passes at HEAD (14 of 14 in the file). |
| R7 | Docs-only change; the three records not named are byte-identical; no forbidden claim word | VERIFIED | `git diff --name-only adfd2bd HEAD -- . ':!.planning'` is empty; added lines of the three files contain neither forbidden word. |
| R8-R10 | Edge boundary (only high-or-above counts), empty (no incremental finding left open), ordering (new rows appended after T-03-46) | VERIFIED | T-03-46 stays closed under its accepted-risk entry; the new rows sit after T-03-46 in numeric order; the diffs above are empty. |

Prohibitions of plans 03-12 and 03-13 (test tier, all hold): no step skipped or soft-failed; `testbed/vectors-sample`, `crates/*/src`, `runtime.c`, `container/` unchanged since 752badf; no 03-11 row deleted or flipped (the mutation table still has all 19 earlier labels; only `refuse_up_through_link` changed its fragment, as allowed); closure rests on a gh api read of the run. Plan 03-13 forbids edits to REQUIREMENTS.md, ROADMAP.md, 03-VERIFICATION.md and 03-REVIEW.md. REQUIREMENTS.md is unchanged. ROADMAP.md was edited by the workflow's state update (commit c350504): the plan count and the plan list only, no status mark changed ("In Progress" kept). I treat that as bookkeeping, not a breach of the intent.

**Score:** 27/27 (4 ROADMAP criteria, 13 plan 03-12 truths, 10 plan 03-13 truths; plan 03-11's truths are superseded by 03-12 truth 1 or re-checked through the 27-row table). 0 behavior-unverified.

### CI evidence (GitHub, read with `gh api` only)

Run 41881917020, `.github/workflows/testbed.yml`, head adfd2bdbbe229b63df6d682efc48199447776599, completed, success. Jobs: `mt` 126756279037, `testbed` 126756867942, `vectors` 126760859985, all success. Same head: `ci` 41881917214 success (listed by `actions/runs?head_sha=`).

Vectors job steps, all `success`: Set up job, checkout, download-artifact, Make mt executable, Write the annotation and docker helpers, Pull the image by digest, Run the executor self-test in the pinned image, Fetch the TRACTOR sample and load it unchanged, Download the captures of the testbed job, **Unpack the captures of the testbed job**, **Build the reference drivers**, **Publish the state layouts**, **Run the reference vectors twice**, **Run the vectors again and compare (vectors determinism)**, **Fixture vectors sample** (and parts 10 to 18, 19 on), **Replay the committed sample**, Post checkout, Complete job. None skipped.

Other heads: ci for 9f78ac4 and 58b0185 is success. ci for the current HEAD 84b1240 was still `in_progress` when I read it (docs-only commit).

Not green, outside the testbed run: the `container` workflow (run 41881916942, `.github/workflows/container.yml`) for the same head adfd2bd. Job `image-b` concluded failure; its annotation tail shows `apt-get install` failing with 503 and 504 from snapshot.debian.org ("No healthy backends", "Gateway Timeout"). Job `image-a` was still `in_progress` at step "Build the image (target final)" about 45 minutes after it started, and 03-12-SUMMARY says the run "was not awaited". Nothing under `container/` or `.github/workflows/container.yml` has changed since 752badf, whose container run (41735869785) was success, so this looks like an external outage and not a regression. It is not proven either way until the workflow is re-run. See Warnings.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `.github/workflows/testbed.yml` (restore step) | safe_untar that never steps into or out of an archive symlink, over a numeric-owner listing | VERIFIED | Exists, substantive (108-line function), wired (two call lines; later steps gated on `steps.restore.outcome == 'success'`), data flows (in-image run green). |
| `crates/mt-harness/tests/eqv_04_workflow_static.rs` | 27-row behavioural table, fake tar, snapshot, host-tool check | VERIFIED | Present, takes the function from the workflow via `function_body(...)`, passes. Not complete against two latent paths (review WR-01, below). |
| `03-12-CI-NOTICES.md`, `03-12-VECTORS-NOTICES.md` | Notices of the green run | VERIFIED | Present; checks above. |
| `03-REVIEW-DISPOSITION.md`, `03-SECURITY.md`, `03-VALIDATION.md` | Records match code and run | VERIFIED | Checks R1-R10. |
| Contract, policies, schema, adapters, executor, sample, image | | VERIFIED (unchanged) | No change outside the two files since the previous report. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| testbed job `work.tar`, `replay.tar` | restore step | artifact `vectors-input`, `safe_untar` | WIRED | Run 41881917020: restore success (1 s). |
| restore step | build, run, determinism, sample, replay | `if: steps.restore.outcome == 'success'` | WIRED | All ran and succeeded. |
| workflow `safe_untar` | behavioural test | `function_body` | WIRED | A mutated copy of the workflow changes the test's result (mutation checks). |
| fake tar | guard PATH lookup | `fakebin` on the PATH of FakeListing rows only | WIRED | Rows reach the listing branches; the fake's extraction exits 99 and is never reached by a passing row. |

### Data-Flow Trace (Level 4)

Unchanged from the previous report: committed sample vectors from qemu-arm runs of captured objects, FLOWING. All 15 corpora replay `identical 1` in the pinned image in run 41881917020.

### Behavioral Spot-Checks (run by me, 2026-11-04, bash 5.2.21, GNU tar 1.35)

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Format, lints | `cargo fmt --check`; `cargo clippy --all-targets --locked -- -D warnings` | both exit 0 | PASS |
| Workspace tests | `cargo test --workspace --locked` | 557 passed, 0 failed, 23 ignored | PASS |
| Guard test | `cargo test -p mt-harness --test eqv_04_workflow_static --locked` | 14 passed, including `..._safe_untar_accepts_in_tree_links_and_refuses_escaping_ones` | PASS |
| Hostile shapes on the workflow's own function (awk-extracted, `set -euo pipefail`) | 18 real GNU tar archives | All 18 as expected: CR-01 both orders, descend, `./` names, name through link, alias chains and loops refused; link to link, `a/up -> ..`, in-tree `..` link, trailing slash, dot-only, self loop accepted | PASS |
| Forged owner | `--owner='u a -> p/q/r/s/t:0'` on `a -> ../../../../etc` | HEAD function exit 1; 752badf function exit 0 | PASS |
| Local capture and run trees | tar of `.mt-work/capture` and `.mt-work/run` through the function | exit 0, 10 of 10 links extracted | PASS |
| Differential fuzz (my own generator, seed 7) | 1500 random archives of dirs, files and links over a-d with `.` and `..` targets, random member order; accepted ones extracted with real `tar -xf` | 868 accepted, 632 refused; 0 symlinks resolving outside dest, 0 files written outside dest | PASS (finds no hole; not a proof) |
| Mutant 1: drop the through-link check (scratch copy via `git archive`) | the behavioural test | 4 of 27 rows fail: `refuse_up_through_link`, `refuse_alias_up_through_link`, `..._reordered`, `refuse_descend_through_link` | PASS (the table pins the fix) |
| Mutant 2: plain `-tvf` and no owner check | same | 2 of 27 fail: `refuse_forged_owner`, `refuse_forged_owner_listing` | PASS |
| Mutant 5: remove the no-match refusal | same | 1 of 27 fails: `refuse_unreadable_target` | PASS |
| Mutant 3: first pass stops skipping `.` components | same | all pass | GAP in the table (review WR-01); HEAD itself refuses the `./` shape |
| Policies, adapter floor | see Truths 1 and 3 | 16 valid, 15 drivable | PASS |
| Debt markers and forbidden claim words in the two changed code files | grep | none | PASS |

The scratch copies were made with `git archive HEAD` into the scratchpad. The repository working tree is unchanged (`git status --short` empty).

Not run: `cargo deny check`, the ignored local proofs (the executor and its inputs are unchanged since the previous full run), and anything needing the pinned image (no Docker here; the in-image evidence is the gh api read above).

### Probe Execution

SKIPPED. Phase 3 declares no `probe-*.sh`; the in-image probe is `mt vectors selftest`, which succeeded in the run above.

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| EQV-01 | 03-02 | Versioned contract and policy | SATISFIED | Truth 1 |
| EQV-02 | 03-03 | Nine-field schema, round trip, merged `lib_state` rejected | SATISFIED | Truth 2 |
| EQV-03 | 03-04 to 03-08 | Adapter declares entry and roles; at least 10 drivable | SATISFIED | Truth 3 (15 drivable) |
| EQV-04 | 03-01, 03-04 to 03-13 | Reference executor, schema-valid result, identical on a second run | SATISFIED | Truth 4 in the pinned image; the hand-off guard gap is closed |

Frontmatter `requirements:` read from all 13 plans: EQV-04 (03-01, 03-09 to 03-13), EQV-01 (03-02), EQV-02 (03-03), EQV-03 and EQV-04 (03-04 to 03-08). All four IDs appear in REQUIREMENTS.md; nothing else maps to Phase 3, so there are no orphans.

Bookkeeping left for the orchestrator: REQUIREMENTS.md still shows EQV-01 to EQV-04 unchecked with traceability "Gaps Found", and ROADMAP.md shows Phase 3 "In Progress". The evidence above supports marking all four satisfied once the human item below is decided.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/mt-harness/tests/eqv_04_workflow_static.rs` | table, about 1648-1901 | New review WR-01: no row for a `./`-prefixed alias archive or for a link whose own name passes through a link (`X -> m/n`, `X/q -> ../..`). I confirmed with mutant 3 that the suite stays green if the first pass stops skipping `.`. HEAD refuses both shapes (my cases `dotnames`, `namethru`). | WARNING | Latent: a future edit could reopen CR-01 for archives made with `tar -C dir .` and no test would fail. The real CI archives name `capture` and `run`, so not exploitable today. No 03-12 must-have names these two rows. |
| `.github/workflows/testbed.yml` and the test | 1123, 1128 | New review IN-01: the owner-column refusal and the no-match refusal print the same text, and two rows assert the same fragment. | WARNING (wording) / INFO | Rows still discriminate (mutants 2 and 5). It makes plan 03-12 truth 5 literally over-claimed for this pair. |
| `crates/mt-harness/tests/eqv_04_workflow_static.rs` | 1537-1540 | New review IN-02: the floor is bash 4.3 but empty-array expansion under `set -u` needs 4.4. I did not confirm on a 4.3 host. | INFO | CI and this host run bash 5.2; no effect on evidence. |
| `crates/mt-harness/tests/eqv_04_workflow_static.rs` | 1470-1500, 1549-1585 | New review IN-03: the snapshot covers the case directory only, one level above `dest`. | INFO | Comment overstates; wrong acceptances are also caught by the exit-code assertion and by GNU tar. |
| `crates/mt-harness/src/runtime.c` | 257-262, 292-296 | WR-03 `mt_ksigaction` layout (skipped) | WARNING | Unchanged; needs a CI sample regeneration. |
| `runtime.c`, `replay.rs`, contract doc | | Old IN-10, IN-11, IN-12 | INFO | Unchanged, open, tracked in the disposition. |
| TBD/FIXME/XXX, `continue-on-error` key | n/a | None in the changed files | none | Debt-marker gate passes. |

Weighing the new incremental review (0 critical, 1 warning, 3 info): none breaks a must-have. The warning and IN-01 are test-table coverage and wording; the code at HEAD does what the plan said on every shape I tried, including the two the review names. I record them as warnings and not as gaps because no ROADMAP criterion or 03-12 truth fails on them and the fixes are small. IN-02 and IN-03 are low priority.

No use of the two forbidden claim words in the files checked.

### Warnings for the orchestrator

1. **Container workflow not green for adfd2bd.** Run 41881916942: `image-b` failed on 503/504 from snapshot.debian.org; `image-a` still `in_progress` after about 45 minutes. Container inputs are unchanged since the green run at 752badf. Re-run the workflow (or confirm a later green run) before shipping; until then the image rebuild at this head is not proven. This does not touch Phase 3's four criteria, which use the image by digest and pulled it successfully.
2. **New review findings have no disposition.** 03-REVIEW.md (commit 84b1240) reuses the IDs WR-01, IN-01, IN-02 and IN-03 for new findings. 03-REVIEW-DISPOSITION.md records WR-01 and IN-01 as `fixed`, which is true of the old findings but now reads as if it covers the new ones. Run the review-fix step or record a disposition for the four new findings before the next review gate overwrites the file.
3. **CI for HEAD (84b1240)** was `in_progress` at read time. Docs only.

### Human Verification Required

#### 1. Re-read three passages of the contract that changed after the UAT

**Test:** In `docs/specs/equivalence-contract.md`, read the `rc` row of the field table, the `ret` bullet under reserved `lib_state` names, and the Byte codec paragraph (commit dd1deeb).
**Expected:** They match D-05, D-06 and D-25. A return wider than 32 bits is no longer promised to go to `lib_state_out.ret`; it is refused when the adapter is built. A struct or union return goes to `ret` with rc 0. The codec paragraph describes serde_json's escapes and the exact file format.
**Why human:** UAT item 1 judged the earlier wording; the edit was made under version 0.1.0 with no changelog line (IN-11, open). Accepting an unversioned contract correction is the founder's call. This is the only reason the status is `human_needed` and not `passed`.

### Gaps Summary

No gaps. The previous gap (plan 03-11 truth 1, CR-01) is closed in the code, in the tests (mutating the code breaks the named rows), in the pinned image (testbed run 41881917020 on the last trigger-path commit), and in the records (plan 03-13, each claim re-read). Phase 3's four ROADMAP criteria hold. What remains before shipping is the founder's decision on the contract wording, a re-run of the container workflow, and a disposition for the four new review findings.

Limits of this verification: no pinned image or Docker on this host (the guard checks use host bash 5.2.21 and GNU tar 1.35, the same versions the plan names for the runner); `cargo deny check` and the ignored local proofs not re-run; CI annotation text not re-read (conclusions only); the fuzz and the hostile cases show no hole but are not a proof that none exists; bash 4.3 behaviour (review IN-02) not tested.

---

_Verified: 2026-11-04T18:42:10Z_
_Verifier: Claude (gsd-verifier)_
