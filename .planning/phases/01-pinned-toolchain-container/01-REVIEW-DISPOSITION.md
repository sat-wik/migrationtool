---
phase: 01
review: 01-REVIEW.md
titles: json
findings:
  - id: WR-13
    severity: warning
    disposition: open
    title: "After a normal exit the descendant sweep cannot find escaped processes, yet the record says `descendants_killed: true`"
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
    title: "A missing `cwd` or interpreter is reported as \"program not found\""
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
    title: "`pin-discovery` never compares the pinned Arm checksum, and the README says it does"
  - id: IN-10
    severity: info
    disposition: open
    title: "GHCR owner is hard-coded and the annotation helper is copied four times"
  - id: IN-11
    severity: info
    disposition: open
    title: "The smoke step falls back to root, and the manifest comparison proves little about installed content"
  - id: IN-12
    severity: info
    disposition: open
    title: "The workflow tests are string-presence checks, and the gate's comparison logic has no executable test"
  - id: IN-13
    severity: info
    disposition: open
    title: "Manifest URL hardening is partial and the Dockerfile curl calls lack the usual options"
  - id: IN-14
    severity: info
    disposition: open
    title: "Dependency crates and tinycbor are bound indirectly"
  - id: IN-15
    severity: info
    disposition: open
    title: "`mt toolchain hash` discards curl's error message"
  - id: IN-16
    severity: info
    disposition: open
    title: "Dead `ProcStat` fields, and a source-grep test stands in for a behavioural one"
  - id: IN-17
    severity: info
    disposition: open
    title: "The tag lookup regex is broad, and a push that fails after succeeding can never be announced or repaired"
  - id: IN-18
    severity: info
    disposition: open
    title: "After a timeout the wait loop has no upper bound"
  - id: IN-19
    severity: info
    disposition: open
    title: "`NETWORKED_INSTALLS` and `NETWORKED_STEPS` are permanently empty allowlists"
  - id: IN-20
    severity: info
    disposition: open
    title: "A reader thread of an incomplete stream keeps hashing and storing after the snapshot"
  - id: IN-21
    severity: info
    disposition: open
    title: "`abort` can signal a reaped child's group if the OS reaps behind the runner"
  - id: WR-08
    severity: warning
    disposition: fixed
    title: "One build step still downloads an unchecksummed tarball, contradicting the Dockerfile's own header, and nothing in the manifest or gate would notice a change"
  - id: WR-09
    severity: warning
    disposition: fixed
    title: "Publishing with `packages: write` is open to a hard-coded feature branch"
  - id: WR-10
    severity: warning
    disposition: fixed
    title: "The runner now compiles only on Linux, but the README tells the founder to run `mt` natively on a Mac"
  - id: WR-11
    severity: warning
    disposition: fixed
    title: "A normal exit with a lingering descendant discards the result, leaves the process running and leaks a reader thread"
  - id: WR-12
    severity: warning
    disposition: fixed
    title: "The \"unobserved\" stream record states false facts about what was captured and drops partial output"
  - id: CR-01
    severity: critical
    disposition: fixed
    title: "Pinned checksums for the Rust channel manifests and crates are verified on a separate download that the installer never uses"
  - id: WR-01
    severity: warning
    disposition: fixed
    title: "A timeout kills only the direct child, and the timeout is then reported as a drain error"
  - id: WR-02
    severity: warning
    disposition: fixed
    title: "`mt toolchain hash` lets curl read user config and expand URL globs"
  - id: WR-03
    severity: warning
    disposition: fixed
    title: "The single-LLVM-major probe inspects only the first library in `ldd` output"
  - id: WR-04
    severity: warning
    disposition: fixed
    title: "Any branch push (and the weekly schedule) publishes to GHCR; the cancel group covers the publish"
  - id: WR-05
    severity: warning
    disposition: fixed
    title: "The \"process launching only in runner.rs\" guard misses the idiom `runner.rs` itself uses"
  - id: WR-06
    severity: warning
    disposition: fixed
    title: "The new static rules are whole-RUN and spelling-bound, so several ways back to an unbound install pass them (new, plan 01-09)"
  - id: WR-07
    severity: warning
    disposition: fixed
    title: "The `Pin binding evidence` gate checks the Dockerfile's own bookkeeping, and its update-hash witness is tautological (new, plan 01-09)"
open: 22
total: 35
recorded: 2026-10-12T03:47:15.841Z
---

# Phase 01: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| WR-13 | warning | open | - |
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
| IN-12 | info | open | - |
| IN-13 | info | open | - |
| IN-14 | info | open | - |
| IN-15 | info | open | - |
| IN-16 | info | open | - |
| IN-17 | info | open | - |
| IN-18 | info | open | - |
| IN-19 | info | open | - |
| IN-20 | info | open | - |
| IN-21 | info | open | - |
| WR-08 | warning | fixed | 01-REVIEW-FIX.md (not in the current review) |
| WR-09 | warning | fixed | 01-REVIEW-FIX.md (not in the current review) |
| WR-10 | warning | fixed | 01-REVIEW-FIX.md (not in the current review) |
| WR-11 | warning | fixed | 01-REVIEW-FIX.md (not in the current review) |
| WR-12 | warning | fixed | 01-REVIEW-FIX.md (not in the current review) |
| CR-01 | critical | fixed | 01-09: 686b56b, container run 38019633821 (not in the current review) |
| WR-01 | warning | fixed | 01-REVIEW-FIX.md (not in the current review) |
| WR-02 | warning | fixed | 01-REVIEW-FIX.md (not in the current review) |
| WR-03 | warning | fixed | 01-REVIEW-FIX.md (not in the current review) |
| WR-04 | warning | fixed | 01-REVIEW-FIX.md (not in the current review) |
| WR-05 | warning | fixed | 01-REVIEW-FIX.md (not in the current review) |
| WR-06 | warning | fixed | 01-REVIEW-FIX.md (not in the current review) |
| WR-07 | warning | fixed | 01-REVIEW-FIX.md (not in the current review) |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
