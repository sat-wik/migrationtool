---
phase: 02
review: 02-REVIEW.md
titles: json
findings:
  - id: WR-01
    severity: warning
    disposition: open
    title: "Capture aborts the whole step on errors that come from untrusted upstream content"
  - id: WR-02
    severity: warning
    disposition: open
    title: "Replay does not tie the capture report to the manifest pin; `source.commit` is used unvalidated as a path component"
  - id: WR-03
    severity: warning
    disposition: open
    title: "Normalised comparison reads the original object from the work directory without checking it against the reported hash"
  - id: WR-04
    severity: warning
    disposition: open
    title: "Untrusted upstream build code runs with write access to the evidence directories and with no resource limits"
  - id: WR-05
    severity: warning
    disposition: open
    title: "The determinism comparison fails open when a report cannot be normalised"
  - id: WR-06
    severity: warning
    disposition: open
    title: "The tree digest is ambiguous for file names that contain a newline"
  - id: WR-07
    severity: warning
    disposition: open
    title: "The documented memory bound for zip seed sources does not hold"
  - id: WR-08
    severity: warning
    disposition: open
    title: "The drop-log floor and the evidence fields are not enforced where the evidence is produced"
  - id: WR-09
    severity: warning
    disposition: open
    title: "Archive extraction relies only on the archive digest; the extracted tree is not checked for escaping links"
  - id: WR-10
    severity: warning
    disposition: open
    title: "`is_cmake_product` is broader than its documentation and masks a known non-determinism"
  - id: IN-01
    severity: info
    disposition: open
    title: "Capture and entry tests pass without running when the Arm tools are missing"
  - id: IN-02
    severity: info
    disposition: open
    title: "The manifest allows a configuration name that capture then rejects"
  - id: IN-03
    severity: info
    disposition: open
    title: "`mt testbed validate` does not validate the drop log"
  - id: IN-04
    severity: info
    disposition: open
    title: "The truncated flag of captured streams is ignored"
  - id: IN-05
    severity: info
    disposition: open
    title: "Stale version in a code comment"
  - id: IN-06
    severity: info
    disposition: open
    title: "Archive download has no size bound and is read fully into memory"
  - id: IN-07
    severity: info
    disposition: open
    title: "`isotp` is dropped as a whole target because of the probe design, not a defect of the target"
  - id: IN-08
    severity: info
    disposition: open
    title: "`fetch` accepts `file://` URLs regardless of the allowed protocols"
open: 18
total: 18
recorded: 2026-10-13T17:04:41.281Z
---

# Phase 02: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| WR-01 | warning | open | - |
| WR-02 | warning | open | - |
| WR-03 | warning | open | - |
| WR-04 | warning | open | - |
| WR-05 | warning | open | - |
| WR-06 | warning | open | - |
| WR-07 | warning | open | - |
| WR-08 | warning | open | - |
| WR-09 | warning | open | - |
| WR-10 | warning | open | - |
| IN-01 | info | open | - |
| IN-02 | info | open | - |
| IN-03 | info | open | - |
| IN-04 | info | open | - |
| IN-05 | info | open | - |
| IN-06 | info | open | - |
| IN-07 | info | open | - |
| IN-08 | info | open | - |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
