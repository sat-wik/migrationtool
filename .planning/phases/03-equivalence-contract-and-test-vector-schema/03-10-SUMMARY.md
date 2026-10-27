---
phase: 03-equivalence-contract-and-test-vector-schema
plan: 10
subsystem: infra
tags: [qemu, container, ghcr, pull-request, founder-merge, testbed, notices-baseline]

requires:
  - phase: 03-equivalence-contract-and-test-vector-schema
    provides: plan 03-09, the QEMU bookworm-backports re-pin, green on the session branch
provides:
  - "PR #6 merged by the founder; main published the QEMU 10.0.2 image sha256:9de8bee1..., recorded as the only pullable digest under Current image in container/README.md"
  - "03-10-CI-NOTICES.md: the eight testbed-* notices of the first testbed run (40451139101) on the new image"
  - "02-22-CI-NOTICES.md re-baselined on the founder's reply, so every Phase 3 plan compares its testbed notices with the D-31 image baseline"
affects: [03-01 Task 3, 03-02]

founder_merge: merged 970755559b91110faa84126efe2a6c4c2ac5c1bb
rebaseline: approved
current_image: ghcr.io/sat-wik/migrationtool-toolchain@sha256:9de8bee18683cc8f5db862e6e657541135f1080b6859ccdeba0fcffa5bd5b565

status: complete
commits: 6
plan_head_before: 76dcebd61c6fc442c8d21d25ad48746a71a24b75
plan_head_after: cf7366d6af8df69a562086d3442d855dc06d7f06
actuals:
  tokens: 13337
  tasks: 3
  commits: 6
---

# Phase 03 Plan 10: QEMU image PR to main Summary

**PR #6 was merged by the founder (merge commit 970755559b91110faa84126efe2a6c4c2ac5c1bb), main published `sha256:9de8bee1...` with QEMU 10.0.2 (proven by the in-image `qemu-mprofile` probe, `exit code: 42`), origin/main is merged back, `## Current image` records the new digest as its only pullable reference, testbed ran green once on it, and the Phase 2 notice baseline was re-baselined on the founder's reply `Rebaseline`. 03-01 Task 3 can resume.**

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

## Task 2 result

The founder replied "Merged" (orchestrator read: PR #6 closed and merged, merge commit `970755559b91110faa84126efe2a6c4c2ac5c1bb` = origin/main). No agent merged, approved or enabled auto-merge.

## Task 3 result (up to the re-baseline gate)

### Publish confirmed (main push run 40449127238)

Head sha equals `git rev-parse origin/main` (`970755559b91110faa84126efe2a6c4c2ac5c1bb`), conclusion success; jobs `mt` 122007422049, `image-a` 122007666992, `image-b` 122007666902, `compare` 122010750379, `publish` 122010802759 all success. Task 3's first automated verify: exit 0.

Notices copied verbatim from that run:

- publish `verified-image`:
```
ghcr.io/sat-wik/migrationtool-toolchain@sha256:9de8bee18683cc8f5db862e6e657541135f1080b6859ccdeba0fcffa5bd5b565
manifest sha256 9f382c31c09adfc0a19ebb247b8fb7d51ea013f44255c048f99036cb82257b68
```
- publish `image-digest`:
```
ghcr.io/sat-wik/migrationtool-toolchain@sha256:9de8bee18683cc8f5db862e6e657541135f1080b6859ccdeba0fcffa5bd5b565
pushed as ghcr.io/sat-wik/migrationtool-toolchain:sha-970755559b91110faa84126efe2a6c4c2ac5c1bb by the publish job, after image-a, image-b and compare succeeded
```
- publish `image-archive`:
```
archive sha256 73bba2278bcfea4a141549eb97d907ab2cd0a327d7fdce8243f3825e2a9bcffd checked before docker load
loaded image id sha256:24ebba9e468a17e3503722b6085121a0802886372e75eef1b51ef268e675a5a6 equals the id image-a gated
```
- image-a `image-archive` (first lines): `image id sha256:24ebba9e468a17e3503722b6085121a0802886372e75eef1b51ef268e675a5a6`, `archive sha256 73bba2278bcfea4a141549eb97d907ab2cd0a327d7fdce8243f3825e2a9bcffd`, saved after every gate of image-a.
- image-a `qemu-mprofile`:
```
qemu-arm version 10.0.2 (Debian 1:10.0.2+ds-2+deb13u1~bpo12+1)
cpu list: cortex-m4 listed
exit code: 42
```
- compare `manifest-compare`: `manifest-a.json and manifest-b.json are byte-identical; sha256 9f382c31c09adfc0a19ebb247b8fb7d51ea013f44255c048f99036cb82257b68`

### Probe gate (fail closed)

Task 3's second automated verify (notice-level `qemu-mprofile` on main's image-a, last line `exit code: 42`, no warning- or failure-level `qemu-mprofile` annotation): exit 0, run before the merge-back and before the README was touched.

### Merge-back

`git merge --no-edit origin/main`: a true merge (the local commit b9ad755 made fast-forward impossible), no conflicts, no rebase, no force. Merge commit `414f3bbb02d37f525ef7903d0c48ba976497d353` (parents b9ad755 and 970755559b91). Its unpushed message was amended once to add the commit trailers.

### Record under `## Current image` and push

Commit `892f03dcda2d78979e511b7815969ef57dcffc0d` (`docs(03-10): record the D-31 image under Current image`), pushed to the session branch. `container/README.md` now holds the single pullable reference `ghcr.io/sat-wik/migrationtool-toolchain@sha256:9de8bee18683cc8f5db862e6e657541135f1080b6859ccdeba0fcffa5bd5b565`, run id 40449127238, merge commit, PR #6, archive sha256, loaded image id, manifest sha256, the tag written alone, the QEMU 10.0.2 sentence, and the superseded images (`sha256:e83006c0...` run 38342425433 PR #4; `sha256:84b1f9a7...` run 39538036371 PR #5, never recorded, both QEMU 7.2) written without the image name. Third automated verify (origin/main is an ancestor of HEAD; README digest equals the publish job's `verified-image` digest; `cargo test -p mt-toolchain --test container_static`, 33 passed): exit 0. Pre-merge gates before the push: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check` all exit 0 (after the Rule 3 fix below). No forbidden claim word in the README.

### Round trip 1 of 3: first testbed run on the new image

Head `892f03dcda2d78979e511b7815969ef57dcffc0d`.

| Workflow | Run | Conclusion |
|----------|-----|------------|
| testbed | 40451139101 | success (job testbed 122014198632 success; job mt 122013889667 success) |
| ci | 40451139087 | success |
| container | 40451139089 | in progress when read (branch push, publish is skipped off main; not part of this plan's green criteria) |

The eight Phase 2 steps of job testbed, each success: Pull the image by digest, Validate the manifest, Fetch the sources, Check the committed seeds, Build each pair in its own container, Record the captures, Replay the captured builds, Capture again and compare the reports (determinism). The fixture steps and "Run the executor self-test in the pinned image" also concluded success (recorded, not judged here). The `testbed-image` notice names the new digest and the loaded image id `sha256:24ebba9e...` of the publish annotations. Task 3's fourth automated verify (notices file cites the run for its head; eight steps success; ci success; head is an ancestor of HEAD; `testbed-image` names the README digest): exit 0. The eight sections are in `03-10-CI-NOTICES.md`.

### vectors-* notices of that run, verbatim (recorded, not judged; 03-01 Task 3 evaluates them as its round trip 3)

`vectors-selftest` (notice):

```
tool qemu-arm qemu-arm version 10.0.2 (Debian 1:10.0.2+ds-2+deb13u1~bpo12+1)
tool arm-none-eabi-gcc arm-none-eabi-gcc (Arm GNU Toolchain 14.3.Rel1 (Build arm-14.174)) 14.3.1 20250623
tool arm-none-eabi-ar GNU ar (Arm GNU Toolchain 14.3.Rel1 (Build arm-14.174)) 2.44.0.20250616
tool arm-none-eabi-nm GNU nm (Arm GNU Toolchain 14.3.Rel1 (Build arm-14.174)) 2.44.0.20250616
tool llvm-dwarfdump Debian LLVM version 16.0.6
cpu cortex-m4 listed
case ok outcome=ok input_len=5
case overrun-input outcome=fault rc=-11 label=crash:SIGSEGV
case overrun-output outcome=fault rc=-11 label=crash:SIGSEGV
case slack-write outcome=fault rc=0 label=oob-write:guard-slack
case abort outcome=abort rc=-6 label=crash:SIGABRT
case loop outcome=timeout rc=-9 label=timeout
case beyond-reserve outcome=executor_error exit=98
case stack-overflow outcome=fault rc=-11 label=crash:SIGSEGV
no core file after overrun
determinism 100 runs 1 hash
dwarf enum byte_size 1
result: passed (17 rows)
exit code: 0
```

`fixture vectors-selftest-dwarf` (2 parts joined in order, notice):

```
probe.o:	file format elf32-littlearm

.debug_info contents:
0x00000000: Compile Unit: length = 0x000000e9, format = DWARF32, version = 0x0005, unit_type = DW_UT_compile, abbr_offset = 0x0000, addr_size = 0x04 (next unit at 0x000000ed)

0x0000000c: DW_TAG_compile_unit
              DW_AT_producer	("GNU C17 14.3.1 20250623 -mcpu=cortex-m4 -mthumb -mfloat-abi=hard -mfpu=fpv4-sp-d16 -march=armv7e-m+fp -g -gdwarf-5 -Os -O0")
              DW_AT_language	(DW_LANG_C11)
              DW_AT_name	("probe.c")
              DW_AT_comp_dir	("/work/vectors-selftest/dwarf")
              DW_AT_stmt_list	(0x00000000)

0x0000001e:   DW_TAG_enumeration_type
                DW_AT_name	("probe_kind")
                DW_AT_encoding	(DW_ATE_unsigned)
                DW_AT_byte_size	(0x01)
                DW_AT_type	(0x0000003d "unsigned char")
                DW_AT_decl_file	("/work/vectors-selftest/dwarf/probe.c")
                DW_AT_decl_line	(1)
                DW_AT_decl_column	(0x06)
                DW_AT_sibling	(0x0000003d)

0x00000030:     DW_TAG_enumerator
                  DW_AT_name	("PROBE_A")
                  DW_AT_const_value	(0x00)

0x00000036:     DW_TAG_enumerator
                  DW_AT_name	("PROBE_B")
                  DW_AT_const_value	(0x01)

0x0000003c:     NULL

0x0000003d:   DW_TAG_base_type
                DW_AT_byte_size	(0x01)
                DW_AT_encoding	(DW_ATE_unsigned_char)
                DW_AT_name	("unsigned char")

0x00000044:   DW_TAG_union_type
                DW_AT_name	("probe_value")
                DW_AT_byte_size	(0x04)
                DW_AT_decl_file	("/work/vectors-selftest/dwarf/probe.c")
                DW_AT_decl_line	(2)
                DW_AT_decl_column	(0x07)
                DW_AT_sibling	(0x00000066)

0x00000051:     DW_TAG_member
                  DW_AT_name	("word")
                  DW_AT_decl_file	("/work/vectors-selftest/dwarf/probe.c")
                  DW_AT_decl_line	(2)
                  DW_AT_decl_column	(0x22)
                  DW_AT_type	(0x00000066 "unsigned int")

0x0000005b:     DW_TAG_member
                  DW_AT_name	("bytes")
                  DW_AT_decl_file	("/work/vectors-selftest/dwarf/probe.c")
                  DW_AT_decl_line	(2)
                  DW_AT_decl_column	(0x36)
                  DW_AT_type	(0x0000006d "unsigned char[4]")

0x00000065:     NULL

0x00000066:   DW_TAG_base_type
                DW_AT_byte_size	(0x04)
                DW_AT_encoding	(DW_ATE_unsigned)
                DW_AT_name	("unsigned int")

0x0000006d:   DW_TAG_array_type
                DW_AT_type	(0x0000003d "unsigned char")
                DW_AT_sibling	(0x0000007d)

0x00000076:     DW_TAG_subrange_type
                  DW_AT_type	(0x00000066 "unsigned int")
                  DW_AT_upper_bound	(0x03)

0x0000007c:     NULL

0x0000007d:   DW_TAG_structure_type
                DW_AT_name	("probe_record")
                DW_AT_byte_size	(0x0c)
                DW_AT_decl_file	("/work/vectors-selftest/dwarf/probe.c")
                DW_AT_decl_line	(3)
                DW_AT_decl_column	(0x10)
                DW_AT_sibling	(0x000000be)

0x0000008a:     DW_TAG_member
                  DW_AT_name	("kind")
                  DW_AT_decl_file	("/work/vectors-selftest/dwarf/probe.c")
                  DW_AT_decl_line	(4)
                  DW_AT_decl_column	(0x15)
                  DW_AT_type	(0x0000001e "probe_kind")
                  DW_AT_data_member_location	(0x00)

0x00000096:     DW_TAG_member
                  DW_AT_name	("flags")
                  DW_AT_decl_file	("/work/vectors-selftest/dwarf/probe.c")
                  DW_AT_decl_line	(5)
                  DW_AT_decl_column	(0x12)
                  DW_AT_type	(0x00000066 "unsigned int")
                  DW_AT_bit_size	(0x03)
                  DW_AT_data_bit_offset	(0x08)

0x000000a4:     DW_TAG_member
                  DW_AT_name	("tag")
                  DW_AT_decl_file	("/work/vectors-selftest/dwarf/probe.c")
                  DW_AT_decl_line	(6)
                  DW_AT_decl_column	(0x13)
                  DW_AT_type	(0x000000be "unsigned char[5]")
                  DW_AT_data_member_location	(0x02)

0x000000b1:     DW_TAG_member
                  DW_AT_name	("value")
                  DW_AT_decl_file	("/work/vectors-selftest/dwarf/probe.c")
                  DW_AT_decl_line	(7)
                  DW_AT_decl_column	(0x17)
                  DW_AT_type	(0x00000044 "probe_value")
                  DW_AT_data_member_location	(0x08)

0x000000bd:     NULL

0x000000be:   DW_TAG_array_type
                DW_AT_type	(0x0000003d "unsigned char")
                DW_AT_sibling	(0x000000ce)

0x000000c7:     DW_TAG_subrange_type
                  DW_AT_type	(0x00000066 "unsigned int")
                  DW_AT_upper_bound	(0x04)

0x000000cd:     NULL

0x000000ce:   DW_TAG_typedef
                DW_AT_name	("probe_record_t")
                DW_AT_decl_file	("/work/vectors-selftest/dwarf/probe.c")
                DW_AT_decl_line	(8)
                DW_AT_decl_column	(0x03)
                DW_AT_type	(0x0000007d "probe_record")

0x000000da:   DW_TAG_variable
                DW_AT_name	("probe_global")
                DW_AT_decl_file	("/work/vectors-selftest/dwarf/probe.c")
                DW_AT_decl_line	(9)
                DW_AT_decl_column	(0x10)
                DW_AT_type	(0x000000ce "probe_record_t")
                DW_AT_external	(true)
                DW_AT_location	(DW_OP_addr 0x0)

0x000000ec:   NULL
```

### Re-baseline gate: diff of 02-22-CI-NOTICES.md vs 03-10-CI-NOTICES.md from line 5 on

`diff <(sed '1,4d' 02-22-CI-NOTICES.md) <(sed '1,4d' 03-10-CI-NOTICES.md)` shows exactly one changed section, `## testbed-image (1 annotation part(s))`, two lines, both expected (they carry the digest and loaded image id):

```
4,5c4,5
< image: ghcr.io/sat-wik/migrationtool-toolchain@sha256:e83006c0a7e37f788afcbd1c95554e8c4dfc61c3b68da3ff964caf47fb3851b9
< id: sha256:1267a6230e171d711a7d056cd7a129fbdf0753339c92af22dac69afc806eac44
---
> image: ghcr.io/sat-wik/migrationtool-toolchain@sha256:9de8bee18683cc8f5db862e6e657541135f1080b6859ccdeba0fcffa5bd5b565
> id: sha256:24ebba9e468a17e3503722b6085121a0802886372e75eef1b51ef268e675a5a6
```

The other seven sections (`testbed-validate`, `-fetch`, `-seeds`, `-build`, `-capture`, `-replay`, `-determinism`) are byte-identical. The `architecture: linux/amd64` line is unchanged. Status at the gate: the founder was shown this list and the single expected change, and asked to reply `rebaseline` or `stop`.

### Re-baseline applied

Founder's reply at the gate, verbatim: `Rebaseline`

02-22-CI-NOTICES.md now keeps four header lines (line 1 `# Plan 02-22 round trip 1: testbed notices, re-baselined by plan 03-10 (D-31)`; line 3 the 03-10 run line followed by: Re-baselined on the founder's reply `rebaseline` after the D-31 image change; the sections of run 39528941371 are in git history at commit 7c55dbe) and takes 03-10-CI-NOTICES.md from line 5 on. All changed sections moved together (only `testbed-image`). Fifth automated verify (header lines present; `diff` of the two files from line 5 on empty): exit 0. Commit `cf7366d6af8df69a562086d3442d855dc06d7f06` (`docs(03-10): re-baseline the Phase 2 testbed notices for the D-31 image`), local only, not pushed (planning files; the orchestrator pushes).

## Deviations from Plan

**1. [Pre-decided handling] No GitHub MCP tool for the executing agent.** The executor had no `create_pull_request` tool; the drafted title and body were handed to the orchestrator, which created PR #6 with them verbatim (the 02-02 precedent). No agent merged, approved or enabled auto-merge on the PR.

**2. [Rule 3 - Blocking] `testbed/manifest.toml` `[image] digest` and one hardcoded digest in a CLI test.** Found during: Task 3 (workspace tests before the push). The test `build_02_image_digest_matches_readme` ties `testbed/manifest.toml [image] digest` to the README's single pullable digest, so recording the new digest under `## Current image` required the same digest in the manifest (the plan's file list named neither). Also `bed_01_testbed_validate_rejects_an_invalid_manifest_with_exit_two` hardcoded the old digest and failed after the manifest changed. Fix: `testbed/manifest.toml` takes the new digest; the test now reads the digest from the committed manifest instead of naming it (its assertions are unchanged in strength). Files modified: `testbed/manifest.toml`, `crates/mt-cli/tests/bed_01_testbed_cli.rs`. Verification: full pre-merge gates exit 0. Commit: 892f03d.

**3. [Minor] Merge commit message amended before the push** to carry the commit trailers; parents unchanged (414f3bb, parents b9ad755 and 970755559b91).

**4. [Minor] Commit count and plan head.** No per-plan commit ledger file was created for 03-10 because the first plan commit (b9ad755) was made by the orchestrator. `plan_head_before` is the parent of b9ad755 (76dcebd, the PR head), `plan_head_after` is the head when this SUMMARY was written (cf7366d), and `commits: 6` is `git rev-list --count 76dcebd..cf7366d`. It counts four plan commits (b9ad755, 892f03d, 7dd3c84, cf7366d) plus the founder's merge of PR #6 (9707555) and the merge-back (414f3bb). The final metadata commit (this SUMMARY, STATE, ROADMAP) is not in the count. `actuals.tokens` is chars/4 over the added and removed lines of the four non-merge commits.

**Total deviations:** 4 (1 pre-decided, 1 Rule 3, 2 minor). One CI round trip of 3 used (testbed and ci on 892f03d, both green).

## Known Stubs

None. This plan writes no code stubs.

## Next Phase Readiness

- **Hand back to 03-01 Task 3.** Its precondition command (in 03-01-PLAN.md Task 3 `<precondition>`: `founder_merge: merged <40hex>`, `rebaseline: approved`, `current_image` equal to the single README digest) was run against this SUMMARY and container/README.md and exits 0.
- Testbed run 40451139101 (head 892f03d, job 122014198632) is the first run on the new image and is the run 03-01 Task 3 evaluates as its round trip 3; 03-01 has no commit of its own to push for it. The self-test step concluded success and its `vectors-selftest` notices are recorded verbatim above; judging them is 03-01's. A fix, if 03-01 finds one needed, is 03-01's under its own escalation.
- Every Phase 3 plan now compares its testbed notices with 02-22-CI-NOTICES.md from line 5 on (`sed '1,4d'`); that file equals 03-10-CI-NOTICES.md from line 5 on.
- 03-02 (wave 3, `depends_on: ["03-01", "03-10"]`) follows 03-01, then 03-03 .. 03-08.
- Testbed on main still pulls the QEMU 7.2 digest in main's README until the Current-image record (commit 892f03d) reaches main with a later PR.
- The commits cf7366d and the final metadata commit are unpushed; the orchestrator pushes.

## Self-Check: PASSED

- Files found: container/README.md, testbed/manifest.toml, crates/mt-cli/tests/bed_01_testbed_cli.rs, 03-10-CI-NOTICES.md, 02-22-CI-NOTICES.md.
- Commits found as ancestors of HEAD: b9ad755, 414f3bb, 892f03d, 7dd3c84, cf7366d, merge commit 970755559b91110faa84126efe2a6c4c2ac5c1bb.
- `git reflog -n 40` has no rebase entry. No forbidden claim word in the README or the notices files. Commit 7c55dbe exists.
