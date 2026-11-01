---
phase: 03
review: 03-REVIEW.md
titles: json
findings:
  - id: CR-01
    severity: critical
    disposition: fixed
    title: "A return value that is a typedef of a 64-bit integer is silently truncated to 32 bits"
  - id: WR-01
    severity: warning
    disposition: fixed
    title: "`replay_corpus` is weaker than its documentation and trusts unvalidated index fields"
  - id: WR-02
    severity: warning
    disposition: fixed
    title: "`mt vectors build` and `mt vectors run` report `result: passed (0 rows)` and exit 0 on an empty selection"
  - id: WR-03
    severity: warning
    disposition: skipped
    title: "`struct mt_ksigaction` does not match the kernel `sigaction` layout (uninitialised padding becomes the signal mask)"
  - id: WR-04
    severity: warning
    disposition: fixed
    title: "The vectors job unpacks a tar produced by the untrusted-build job straight onto the runner"
  - id: WR-05
    severity: warning
    disposition: fixed
    title: "The \"Replay the committed sample\" step passes silently when the sample directory is missing"
  - id: WR-06
    severity: warning
    disposition: fixed
    title: "`is_prefix_line` does not enforce \"a single directive\" and lets line-splicing and comment openers into the C source"
  - id: WR-07
    severity: warning
    disposition: fixed
    title: "Policy `error_code` values are not range-checked against the function's return type"
  - id: WR-08
    severity: warning
    disposition: fixed
    title: "Identifier grammars for target ids and configuration names disagree between the manifest, the policy loader and the vector schema"
  - id: IN-01
    severity: info
    disposition: open
    title: "`MAP_FIXED` mappings silently replace whatever is already at the address"
  - id: IN-02
    severity: info
    disposition: open
    title: "The contract document describes two behaviours the code does not have"
  - id: IN-03
    severity: info
    disposition: open
    title: "Canonical form depends on `serde_json`'s sorted `Map`, and a few `[]` indexings rely on earlier checks without a comment"
  - id: IN-04
    severity: info
    disposition: open
    title: "Duplicated helpers"
  - id: IN-05
    severity: info
    disposition: open
    title: "The image has no default non-root user"
  - id: IN-06
    severity: info
    disposition: open
    title: "Recompile and flag handling is a denylist while the driver flags are an allowlist"
  - id: IN-07
    severity: info
    disposition: open
    title: "Policy loader silently ignores `.toml` files whose stem is not valid UTF-8"
open: 7
total: 16
recorded: 2026-11-01T02:25:51.607Z
---

# Phase 03: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| CR-01 | critical | fixed | 03-REVIEW-FIX.md |
| WR-01 | warning | fixed | 03-REVIEW-FIX.md |
| WR-02 | warning | fixed | 03-REVIEW-FIX.md |
| WR-03 | warning | skipped | 03-REVIEW-FIX.md |
| WR-04 | warning | fixed | 03-REVIEW-FIX.md |
| WR-05 | warning | fixed | 03-REVIEW-FIX.md |
| WR-06 | warning | fixed | 03-REVIEW-FIX.md |
| WR-07 | warning | fixed | 03-REVIEW-FIX.md |
| WR-08 | warning | fixed | 03-REVIEW-FIX.md |
| IN-01 | info | open | - |
| IN-02 | info | open | - |
| IN-03 | info | open | - |
| IN-04 | info | open | - |
| IN-05 | info | open | - |
| IN-06 | info | open | - |
| IN-07 | info | open | - |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
