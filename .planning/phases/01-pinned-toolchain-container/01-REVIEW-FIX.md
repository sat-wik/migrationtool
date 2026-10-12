---
phase: 01-pinned-toolchain-container
fixed_at: 2026-10-12T04:00:00Z
review_path: .planning/phases/01-pinned-toolchain-container/01-REVIEW.md
iteration: 2
findings_in_scope: 1
fixed: 1
skipped: 0
status: all_fixed
---

# Phase 1: Code Review Fix Report (iteration 2)

**Source review:** 01-REVIEW.md (diff base 53faae5; 0 critical, 1 warning, 21 info).
Iteration 1 (WR-08..WR-12, commits d532ff6, 9f988ad, 1daa730, 736256f, defbe68, d909893, 3fa71a1; CI runs 38198934488 and 38199978785) is recorded in git history at commit e733ce5 and confirmed resolved by this review's "Prior findings" section.

## Fixed Issues

### WR-13: After a normal exit the descendant sweep cannot find escaped processes, yet the record says `descendants_killed: true`

**Files modified:** `crates/mt-toolchain/src/runner.rs`, `crates/mt-toolchain/tests/tool_01_runner.rs`
**Commit:** 2916af8
**Applied fix:** The review's second option. The record field is renamed `exit.tree_kill_sent`. It records that SIGKILL was sent to the child's group and to every descendant the `/proc` walk still found, not that every pipe holder died. The field doc, `run` doc and module doc now say this: after the child exits, a descendant that left the group has already been reparented away, may survive, and shows as `complete: false` on the stream it holds. The schema stays at `RUN_RECORD_SCHEMA_VERSION` 2. Version 2 was introduced on this branch and has never shipped on `main`, so renaming its field does not need a new version. The tests assert the renamed key.
**Not done:** The review's first option, finding pipe holders by pipe inode through `/proc/*/fd` and killing them by pidfd. That would close the gap rather than describe it, and remains a hardening item for the phase that first runs forking analysers (KLEE) through the runner.

Local gates: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo deny check` all exit 0.

## Skipped Issues

None.
