---
phase: 01
review: 01-REVIEW.md
titles: json
findings:
  - id: CR-01
    severity: critical
    disposition: open
    title: "Pinned checksums for the Rust channel manifests and crates are verified on a separate download that the installer never uses"
  - id: WR-01
    severity: warning
    disposition: open
    title: "A timeout kills only the direct child, and the timeout is then reported as a drain error"
  - id: WR-02
    severity: warning
    disposition: open
    title: "`mt toolchain hash` lets curl read user config and expand URL globs"
  - id: WR-03
    severity: warning
    disposition: open
    title: "The single-LLVM-major probe inspects only the first library in `ldd` output"
  - id: WR-04
    severity: warning
    disposition: open
    title: "Any branch push (and the weekly schedule) publishes to GHCR; the cancel group covers the publish"
  - id: WR-05
    severity: warning
    disposition: open
    title: "The \"process launching only in runner.rs\" guard misses the idiom `runner.rs` itself uses"
  - id: IN-01
    severity: info
    disposition: open
    title: "Truncated captures are treated as complete observations"
  - id: IN-02
    severity: info
    disposition: open
    title: "`not_installed` tools are never verified absent"
  - id: IN-03
    severity: info
    disposition: open
    title: "LLVM-family package detection gaps and a misleading comment"
  - id: IN-04
    severity: info
    disposition: open
    title: "A missing `cwd` or missing interpreter is reported as \"program not found\""
  - id: IN-05
    severity: info
    disposition: open
    title: "Run record is lossy and does not identify the executable that ran"
  - id: IN-06
    severity: info
    disposition: open
    title: "Pins validation does not cover every value the Dockerfile interpolates"
  - id: IN-07
    severity: info
    disposition: open
    title: "Dead build arguments and no in-build assertion of the pinned rustc commit"
  - id: IN-08
    severity: info
    disposition: open
    title: "Final image runs as root and ships unpinned runtime packages"
  - id: IN-09
    severity: info
    disposition: open
    title: "`pin-discovery` never compares the pinned Arm checksum"
  - id: IN-10
    severity: info
    disposition: open
    title: "GHCR owner is hard-coded and the annotation helper is copied three times"
  - id: IN-11
    severity: info
    disposition: open
    title: "The smoke step falls back to root on any unprivileged failure, and the manifest comparison proves little"
open: 17
total: 17
recorded: 2026-10-09T20:15:03.832Z
---

# Phase 01: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| CR-01 | critical | open | - |
| WR-01 | warning | open | - |
| WR-02 | warning | open | - |
| WR-03 | warning | open | - |
| WR-04 | warning | open | - |
| WR-05 | warning | open | - |
| IN-01 | info | open | - |
| IN-02 | info | open | - |
| IN-03 | info | open | - |
| IN-04 | info | open | - |
| IN-05 | info | open | - |
| IN-06 | info | open | - |
| IN-07 | info | open | - |
| IN-08 | info | open | - |
| IN-09 | info | open | - |
| IN-10 | info | open | - |
| IN-11 | info | open | - |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
