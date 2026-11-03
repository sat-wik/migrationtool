---
phase: 03
review: 03-REVIEW.md
titles: json
findings:
  - id: WR-03
    severity: warning
    disposition: skipped
    title: "`struct mt_ksigaction` does not match the kernel `sigaction` layout (uninitialised padding becomes the signal mask)"
  - id: IN-01
    severity: info
    disposition: open
    title: "`MAP_FIXED` mappings silently replace whatever is already at the address, and no assert covers the input mapping"
  - id: IN-10
    severity: info
    disposition: open
    title: "Replay re-runs every vector but does not compare `discarded_output_bytes` or the derived `has_ub_source`, and the comment that excuses it is wrong"
  - id: IN-11
    severity: info
    disposition: open
    title: "The contract document was edited under version 0.1.0 with no changelog line, and its versioning rule has no exception for corrections"
  - id: IN-12
    severity: info
    disposition: open
    title: "The byte-codec paragraph still leaves out the hex case of `\\u00XX`, so an independent reader cannot reproduce the file names"
  - id: WR-01
    severity: warning
    disposition: fixed
    title: "Replay still does not compare most provenance fields, so an edited provenance record or a changed policy is not detected"
  - id: WR-02
    severity: warning
    disposition: fixed
    title: "A listing whose rows are all `no_roles` still prints `result: passed` and exits 0"
  - id: IN-02
    severity: info
    disposition: fixed
    title: "The contract document describes two behaviours the code does not have"
  - id: IN-03
    severity: info
    disposition: fixed
    title: "Canonical form depends on `serde_json`'s sorted `Map`, and a few `[]` indexings rely on earlier checks without a comment"
  - id: IN-04
    severity: info
    disposition: fixed
    title: "Duplicated helpers"
  - id: IN-05
    severity: info
    disposition: fixed
    title: "The image has no default non-root user"
  - id: IN-06
    severity: info
    disposition: fixed
    title: "Recompile and flag handling is a denylist while the driver flags are an allowlist"
  - id: IN-07
    severity: info
    disposition: fixed
    title: "Policy loader silently ignores `.toml` files whose stem is not valid UTF-8"
  - id: IN-08
    severity: info
    disposition: fixed
    title: "The WR-07 fix split a doc comment: `is_decimal_integer` lost its doc and `fits_return_window` carries a wrong first line"
  - id: IN-09
    severity: info
    disposition: fixed
    title: "Include-prefix hardening is incomplete: trigraph splice, and the probe path is not covered"
  - id: CR-01
    severity: critical
    disposition: fixed
    title: "A return value that is a typedef of a 64-bit integer is silently truncated to 32 bits"
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
open: 4
total: 21
recorded: 2026-11-03T05:07:00.457Z
---

# Phase 03: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| WR-03 | warning | skipped | 03-REVIEW-FIX.md |
| IN-01 | info | open | - |
| IN-10 | info | open | - |
| IN-11 | info | open | - |
| IN-12 | info | open | - |
| WR-01 | warning | fixed | 03-REVIEW-FIX.md (not in the current review) |
| WR-02 | warning | fixed | 03-REVIEW-FIX.md (not in the current review) |
| IN-02 | info | fixed | 03-REVIEW-FIX.md (not in the current review) |
| IN-03 | info | fixed | 03-REVIEW-FIX.md (not in the current review) |
| IN-04 | info | fixed | 03-REVIEW-FIX.md (not in the current review) |
| IN-05 | info | fixed | 03-REVIEW-FIX.md (not in the current review) |
| IN-06 | info | fixed | 03-REVIEW-FIX.md (not in the current review) |
| IN-07 | info | fixed | 03-REVIEW-FIX.md (not in the current review) |
| IN-08 | info | fixed | 03-REVIEW-FIX.md (not in the current review) |
| IN-09 | info | fixed | 03-REVIEW-FIX.md (not in the current review) |
| CR-01 | critical | fixed | 03-REVIEW-FIX.md (not in the current review) |
| WR-04 | warning | fixed | 03-REVIEW-FIX.md (not in the current review) |
| WR-05 | warning | fixed | 03-REVIEW-FIX.md (not in the current review) |
| WR-06 | warning | fixed | 03-REVIEW-FIX.md (not in the current review) |
| WR-07 | warning | fixed | 03-REVIEW-FIX.md (not in the current review) |
| WR-08 | warning | fixed | 03-REVIEW-FIX.md (not in the current review) |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
