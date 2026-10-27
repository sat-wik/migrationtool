---
phase: 03-equivalence-contract-and-test-vector-schema
plan: 10
subsystem: infra
tags: [qemu, container, ghcr, pull-request, founder-merge, testbed, notices-baseline]

requires:
  - phase: 03-equivalence-contract-and-test-vector-schema
    provides: plan 03-09, the QEMU bookworm-backports re-pin, green on the session branch
provides:
  - "PR #6 from claude/gsd-discuss-phase-1-mzdw5v to main carrying plan 03-09's green re-pin, with green container and ci checks (Task 1 done)"
affects: [03-01 Task 3, 03-02]

status: in progress, awaiting founder merge (Task 2)
---

# Phase 03 Plan 10: QEMU image PR to main Summary (partial)

**Task 1 is done: PR #6 is open with plan 03-09's re-pin and its container and ci checks are green. The plan now waits at Task 2 for the founder to merge it by hand. Tasks 2 and 3 are not done; the `founder_merge`, `rebaseline` and `current_image` frontmatter keys that 03-01 Task 3 reads are deliberately not written yet.**

## Task 1 result

- **PR:** #6, https://github.com/sat-wik/migrationtool/pull/6, head `claude/gsd-discuss-phase-1-mzdw5v` at `76dcebd61c6fc442c8d21d25ad48746a71a24b75`, base `main` (origin/main `841ac040`, merge of PR #5). State when read: open, not merged.
- **Precondition checks:** no PR template exists; no open PR had this head before creation; `HEAD` equals `origin/claude/gsd-discuss-phase-1-mzdw5v`, which still equals the PR head (nothing was pushed since creation).
- **Title:** `Pinned image: QEMU from bookworm-backports for M-profile user mode (phase 03, D-31)`
- **PR checks** (event `pull_request`, head 76dcebd):

| Workflow | Run | Conclusion |
|----------|-----|------------|
| container | 40447523176 | success: mt 122002226119 success; image-a 122002501447 success; image-b 122002501400 success; compare 122005165117 success; publish 122005222709 skipped (only main publishes, WR-09) |
| ci | 40447523213 | success (job checks 122002225983 success) |

pin-discovery and testbed do not run on `pull_request`. No check is red.

- **PR container run's `qemu-mprofile` notice** (image-a job 122002501447, level notice), verbatim:

```
qemu-arm version 10.0.2 (Debian 1:10.0.2+ds-2+deb13u1~bpo12+1)
cpu list: cortex-m4 listed
exit code: 42
```

- image-a `check-a` ends `result: OK` and `exit code: 0`; compare's `manifest-compare`: manifest-a.json and manifest-b.json byte-identical, sha256 `9f382c31c09adfc0a19ebb247b8fb7d51ea013f44255c048f99036cb82257b68`.
- Task 1 automated verify: exit 0 (one open PR whose head equals the pushed upstream head; container and ci `pull_request` runs both completed).

## PR body (essentials, as submitted)

**What changes**
- `container/Dockerfile`: a fourth snapshot apt source `bookworm-backports main` at the existing pinned snapshot timestamp, plus `/etc/apt/preferences.d/50-qemu-backports` giving priority 990 to exactly `qemu-user`, `qemu-system-arm`, `qemu-system-common`, `qemu-system-data` and priority -1 to every other backports package; a dependency outside that set fails the build. No insecure source option.
- `container/pins.toml` and fixtures: the `[apt]` values for `qemu-user` and `qemu-system-arm` and the tool expects `qemu_arm`, `qemu_system_arm` carry V; the tool-outputs fixture is a CI capture (container run 40444623268).
- `container.yml`: image-a probe `QEMU M-profile user-mode start (D-31)` (notice `qemu-mprofile`) runs `qemu-arm -cpu cortex-m4 -R 0x10000000` on a small Thumb guest (`container/smoke/qemu-mprofile.S`, `exit_group(42)` by raw `svc #0`, no semihosting) with `--network none --read-only` and an unprivileged user; the save step has no status function, so an image failing the probe is never archived or published.
- `pin-discovery.yml`: informational `qemu-backports` notice listing every `qemu-user` `~bpo12` version the snapshot archive has seen.
- README rows, bumping-pins step and notices entry; PROJECT.md decision D-21; two hand-back edits to the 03-01 plan.
- Packages changed in the image: `qemu-user`, `qemu-system-arm`, `qemu-system-common`, `qemu-system-data` change to V. Added: none. Removed (dependencies only QEMU 7.2 needed): `iso-codes`, `libcap2-bin`, `libdrm-common`, `libdrm2`, `libdw1`, `libepoxy0`, `libgbm1`, `libgstreamer-plugins-base1.0-0`, `libgstreamer1.0-0`, `libopus0`, `liborc-0.4-0`, `libsndio7.0`, `libspice-server1`, `libunwind8`, `libwayland-server0`. No non-QEMU package changed version.

**Why.** QEMU 7.2.22 maps the ARM commpage at 0xffff0000 for every ARM CPU before the first guest instruction, and M-profile CPUs cap `reserved_va` at 0x80000000, so `qemu-arm -cpu cortex-m4` aborts with `page_set_flags: Assertion 'end - 1 <= GUEST_ADDR_MAX'` (testbed runs 39687629155 and 39688983966).

**Version: not the 8.2.x that D-31 names.** V = `1:10.0.2+ds-2+deb13u1~bpo12+1` (upstream 10.0.2), the same for `qemu-user` and `qemu-system-arm`. D-31 names 8.2.x as measured on the host; V differs, a recorded deviation. The pinned snapshot 20261005T000000Z offers 10.0.2 as the backports candidate; the only 8.2.x versions the archive ever held in backports are `8.2.0+ds-5~bpo12+1` (first seen 20240123T212534Z) and `8.2.1+ds-1~bpo12+1` (first seen 20240206T163351Z), and using them would need a different backports snapshot timestamp, which means a new `[image]` key and mt-toolchain code (a separate gap plan). In-image evidence: the `qemu-mprofile` notice above, in 03-09's run 40446062121 and in this PR's container run 40447523176. Version rule applied: contains `~bpo12`, identical for both packages, upstream major 10 at least 8. The licence text of QEMU 10.0.2 has not been read; the README names the image's own copyright file as the authority. The full `qemu-backports` list is in 03-09-SUMMARY.md (run 40444623194, job 121992828820).

**Evidence (plan 03-09, head 922c116, push events on the session branch):** container 40446062121 success (publish skipped); pin-discovery 40446062155 success; ci 40446062085 success; check-a QEMU rows OK at V and `llvm_packages 16 16 16 OK`; negative pin (LLVM 16 -> 17) exits 1 with MISMATCH on klee and rustc_bitcode. The PR head 76dcebd adds only planning documents after 922c116.

**After the merge.** Main's container run builds twice, compares manifests and publishes by digest, announcing it in `verified-image`. Testbed on main still pulls the digest in main's README `## Current image`, the QEMU 7.2 image `sha256:e83006c0...`, so its executor self-test step fails at QEMU startup (the known state, not a regression). Testbed turns green on main only after the Current-image record reaches main with a later PR.

**Commits the PR brings to main** (`git log --oneline origin/main..HEAD`, 30 commits, 76dcebd back to b2167e1): plan 03-09's six code and doc commits (`260610f`, `9770902`, `0aed190`, `58d465c`, `7177ce4`, `922c116`) and its two docs commits (`3b1b462`, `76dcebd`); Phase 3 planning documents (D-31 record `90fb133`, plans `999a5ea`, `860b582`, `b4e5d3b`, phase start/resume `7835375`, `f79b836`, and the earlier context, research and plan commits `b2167e1` .. `5ecc9a2`); and plan 03-01's work so far (`935321b` .. `89f9add`: the `mt-harness` crate, `mt vectors selftest`, the testbed self-test step).

**Merge request.** Merge with "Create a merge commit" (not squash, not rebase), so merging `main` back stays a plain merge. Nothing is pushed to the session branch between opening and merging.

## Deviations from Plan

**1. [Pre-decided handling] No GitHub MCP tool for the executing agent.** The executor had no `create_pull_request` tool; the drafted title and body were handed to the orchestrator, which created PR #6 with them verbatim (the 02-02 precedent). No agent merged, approved or enabled auto-merge on the PR.

**Total deviations:** 1, pre-decided. No CI round trip of this plan has been used (the PR-check wait is not a round trip; no push was made).

## Next Phase Readiness

- **Task 2 (founder, blocking):** merge PR #6 with "Create a merge commit" and wait for main's `container` push run for the merge commit to finish. Reply `merged` or `stop`.
- **Task 3 (after `merged`):** confirm the publish and the `qemu-mprofile` exit code 42 on main, merge `origin/main` back, record the new digest under `## Current image`, run testbed once on it, show the changed notice sections to the founder for the `rebaseline` reply, then hand back to 03-01 Task 3 (round trip 3).
- Frontmatter keys `founder_merge`, `rebaseline`, `current_image` will be written only when those facts exist. Until they do, 03-01 Task 3 stays halted.
- Testbed on main fails its self-test step until the Current-image record reaches main in a later PR.
