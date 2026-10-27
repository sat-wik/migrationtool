---
phase: 03-equivalence-contract-and-test-vector-schema
plan: 01
subsystem: testing
tags: [qemu-user, cortex-m4, c-reference-executor, guard-pages, outcome-classification, dwarf, github-actions]

requires:
  - phase: 02-test-bed-and-build-capture
    provides: the mt-toolchain runner, the pins, mt_capture::manifest::BASELINE_CFLAGS and the testbed workflow helpers this plan extends
  - phase: 03-equivalence-contract-and-test-vector-schema
    provides: plans 03-09 and 03-10, the D-31 image (QEMU 10.0.2 from bookworm-backports) that this plan's self-test finally ran on
provides:
  - "crates/mt-harness: the C reference executor core (shared runtime template, link recipe, qemu-arm -cpu cortex-m4 run through the runner, strict result frame, outcome classification) and the executor self-test"
  - "mt vectors selftest (exit 0 every row passed, 1 a row failed, 2 invalid input)"
  - "run_vectors / finish_vectors workflow helpers and the step 'Run the executor self-test in the pinned image', measured green in the pinned image (17 rows)"
  - "crates/mt-harness/tests/fixtures/dwarf/selftest-probe-gcc14.txt: GCC 14.3 llvm-dwarfdump text of the probe, 1-byte enum, for plan 03-05's layout parser"
  - "03-01-CI-NOTICES.md and 03-01-VECTORS-NOTICES.md of round trip 3"
affects: [03-02, 03-03, 03-04, 03-05, 03-07, phase 5 emulator replay]

requirements-completed: [EQV-04]

status: complete
commits: 29
plan_head_before: 7835375bdb9967c7fa7a948c2b2b857aa8d82652
plan_head_after: 32a8c5e926a5a69f60ac9f33c56f4a98b6761451
actuals:
  tokens: 40651
  tasks: 3
  commits: 29

tech-stack:
  added: []
  patterns:
    - "one shared, reviewed C runtime template with a closed @MT_<NAME>@ placeholder set; the renderer fails on an unknown or leftover token"
    - "Linux-ABI static ELF with its own _start, raw svc syscalls and guarded mappings, run under qemu-arm user mode; the in-guest fault handler exits 128+signal"
    - "classification decided only from the runner's record and a strictly parsed result frame, never from guest text"

key-files:
  created:
    - crates/mt-harness/Cargo.toml
    - crates/mt-harness/src/lib.rs
    - crates/mt-harness/src/runtime.c
    - crates/mt-harness/src/runtime.rs
    - crates/mt-harness/src/budget.rs
    - crates/mt-harness/src/tools.rs
    - crates/mt-harness/src/driver.rs
    - crates/mt-harness/src/exec.rs
    - crates/mt-harness/src/frame.rs
    - crates/mt-harness/src/classify.rs
    - crates/mt-harness/src/selftest.rs
    - crates/mt-harness/tests/eqv_04_classify.rs
    - crates/mt-harness/tests/eqv_04_selftest.rs
    - crates/mt-harness/tests/eqv_04_workflow_static.rs
    - crates/mt-harness/tests/fixtures/dwarf/selftest-probe-gcc14.txt
    - crates/mt-cli/tests/eqv_04_selftest_cli.rs
    - .planning/phases/03-equivalence-contract-and-test-vector-schema/03-01-CI-NOTICES.md
    - .planning/phases/03-equivalence-contract-and-test-vector-schema/03-01-VECTORS-NOTICES.md
  modified:
    - Cargo.toml
    - Cargo.lock
    - crates/mt-cli/Cargo.toml
    - crates/mt-cli/src/main.rs
    - crates/mt-capture/tests/build_02_workflow_static.rs
    - .github/workflows/testbed.yml

key-decisions:
  - "Crate split: the executor lives in the new mt-harness crate (the name building-the-tool.md section 3 reserves for the differential runner and emulator replay); it depends on mt-toolchain and mt-capture (path crates) and reads BASELINE_CFLAGS from mt_capture::manifest directly, so no constant copy exists"
  - "Budgets::DEFAULT: timeout_ms 10000, reserved_va 0x1000_0000 (also the memory budget), heap_arena 1 MiB, sentinel 0xA5, slack_fill 0x5A, max_input 65536; constants, not manifest keys"
  - "has_ub labels written by the executor: crash:SIGSEGV, crash:SIGBUS, crash:SIGILL, crash:SIGFPE, crash:SIGABRT, oob-write:guard-slack, timeout. A timeout carries the label 'timeout' and rc -9 so TRACTOR tools and the comparison rule skip it (D-02 never a pass). Founder may revisit"
  - "D-02 'empty, fixed environment' is the runner's fixed four-key base (LANG, PATH, SOURCE_DATE_EPOCH, TZ) with no extra variable, because the audited runner cannot start a child with fewer; QEMU passes them to the guest and the driver moves sp to its own mapping. Interpretation for the founder"
  - "Driver exit codes 95 (C code called exit), 96 (request open or read failed), 97 (malformed request), 98 (mmap failed, including a mapping beyond -R), 99 (other setup failure) are all executor_error"
  - "Result frame: little-endian, magic MTRS, version 1, status word, raw return value, count of libc bytes discarded by the _write stub, slack check result, length-prefixed region table, end marker MTRE; request req.bin: magic MTRQ, version 1, length-prefixed fields"
  - "The slack-write case reports rc 0 (the frame's raw return value) with label oob-write:guard-slack; a fault from a changed slack byte keeps the frame's rc, a signal fault uses -signal"

patterns-established:
  - "Self-test row order is fixed and every row is a measurement in the pinned image; a missing row is a failure, not a pass"
  - "Workflow helper for a run inside the pinned image: offline, read-only, capability-free, only /work and /out writable, enforced by a static test"

coverage:
  - id: D1
    description: "Shared runtime template rendered and linked with the pinned compiler's baseline flags, run under qemu-arm -cpu cortex-m4 -R 0x10000000 -seed 0 through the runner with the four base environment keys and a fixed slot cwd; parsed result frame; twice-identical runs"
    requirement: EQV-04
    verification:
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_04_selftest.rs#eqv_04_selftest_ok_case_runs_end_to_end"
        status: pass
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_04_selftest.rs#eqv_04_selftest_runs_twice_identically"
        status: pass
    human_judgment: false
  - id: D2
    description: "Full classification table: ok, fault (rc = -signal), abort, timeout, setup_failed class set; executor_error never becomes a vector"
    requirement: EQV-04
    verification:
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_04_classify.rs#eqv_04_classify_maps_each_runner_result_to_one_class"
        status: pass
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_04_classify.rs#eqv_04_classify_executor_error_never_has_a_vector_class"
        status: pass
    human_judgment: false
  - id: D3
    description: "Fault surfaces are deterministic and contained: guard-page overruns, slack writes, abort, timeout, -R budget, stack guard, 100-run determinism, GCC DWARF 1-byte enum, template target-neutrality"
    requirement: EQV-04
    verification:
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_04_selftest.rs#eqv_04_overrun_is_a_fault"
        status: pass
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_04_selftest.rs#eqv_04_selftest_runs_are_byte_identical"
        status: pass
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_04_selftest.rs#eqv_04_runtime_template_is_target_neutral"
        status: pass
    human_judgment: false
  - id: D4
    description: "run_vectors is offline and least-privilege; the self-test step publishes vectors-selftest and the DWARF fixture; vectors steps never publish testbed titles"
    requirement: EQV-04
    verification:
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_04_workflow_static.rs#eqv_04_workflow_static_run_vectors_is_offline_and_least_privilege"
        status: pass
      - kind: unit
        ref: "crates/mt-harness/tests/eqv_04_workflow_static.rs#eqv_04_workflow_static_vectors_steps_never_publish_testbed_titles"
        status: pass
    human_judgment: false
  - id: D5
    description: "The self-test measured QEMU 10.0.2 with -cpu cortex-m4, GCC 14.3 linking, guard pages, the fault handler, -R, determinism and the analysis tools in the pinned image (testbed run 40451139101, result: passed (17 rows)); the eight Phase 2 notices equal 02-22 from line 5"
    requirement: EQV-04
    verification:
      - kind: other
        ref: "gh api repos/sat-wik/migrationtool/actions/runs/40451139101 (testbed success on 892f03d); ci run 40451139087 success; 03-01-VECTORS-NOTICES.md"
        status: pass
    human_judgment: false

duration: "spans four sessions (first commit 2026-10-21T22:21Z, summary 2026-10-27T04:59Z, with the D-31 gap plans 03-09 and 03-10 and a founder merge in between)"
completed: 2026-10-27
---

# Phase 3 Plan 01: C reference executor core and in-image self-test Summary

**The C reference executor (shared runtime template, guarded mappings, in-guest fault handler, `qemu-arm -cpu cortex-m4` through the runner, strict result frame, full outcome classification) exists in `mt-harness`, and `mt vectors selftest` passed all 17 rows in the pinned image (QEMU 10.0.2, GCC 14.3) on CI round trip 3 (testbed run 40451139101).**

## Performance

- **Duration:** spans four sessions; the plan's first commit is 2026-10-21T22:21Z, this summary 2026-10-27T04:59Z. Wall time is dominated by the D-31 gap (plans 03-09 and 03-10) and a founder merge, not by work on this plan.
- **Tasks:** 3 of 3 (Tracer, Task 2, Task 3)
- **Files:** 24 files changed against 949214e in the plan's directories (3738 insertions, 10 deletions), plus the three planning artifacts below.
- **Plan commits:** nine (see Task Commits). `commits: 29` in the frontmatter is the measured `git rev-list --count 7835375..HEAD`; it also counts the interleaved D-31 planning commits and plans 03-09 and 03-10, because this plan was paused for them. See deviation 6.

## Accomplishments

- **Executor core (EQV-04):** `crates/mt-harness/src/runtime.c` is the one shared template: Thumb `_start`, own stack mapping with an unmapped guard page, raw `svc #0` syscalls (no semihosting, no breakpoint instruction), guarded right-aligned buffers with prefilled slack, handlers for SIGSEGV, SIGBUS, SIGILL, SIGFPE and SIGABRT on an alternate stack exiting `128 + signal`, newlib stubs that discard and count libc output, request reading and a single raw-write result frame. `driver.rs` links with `BASELINE_CFLAGS -nostartfiles --specs=nosys.specs ... -lm`; `exec.rs` runs `qemu-arm -cpu cortex-m4 -R 0x10000000 -seed 0 driver.elf req.bin` through `mt_toolchain::runner::run` with the four base environment keys only and a fixed slot cwd.
- **Classification (D-03, D-25, D-27):** `classify.rs` maps each runner result to exactly one class from the runner's record and the strictly parsed frame; `OUTCOME_CLASSES` is `["ok", "fault", "abort", "timeout", "setup_failed"]`; `executor_error` is never a vector outcome.
- **In-image measurement (D-30):** testbed run 40451139101, the first on the D-31 image, passed the self-test with 17 rows. Overruns of input and output fault with rc -11 and leave no core file; a slack write is `fault` with `oob-write:guard-slack`; `abort` gives rc -6; the infinite loop gives `timeout` rc -9; a mapping beyond `-R` is `executor_error` exit 98; unbounded recursion faults through the stack guard; 100 runs give one stdout hash; `arm-none-eabi-ar`, `-nm` and `llvm-dwarfdump` run; GCC 14.3's DWARF for the probe shows a 1-byte enum.
- **Workflow:** `run_vectors` (offline, read-only, `--cap-drop ALL`, `no-new-privileges`, user 65534, four resource limits, only `/work` and `/out` writable) and `finish_vectors` publish `vectors-selftest` and `fixture vectors-selftest-dwarf`; the eight Phase 2 `testbed-*` titles are unchanged.
- **Fixture:** `selftest-probe-gcc14.txt` (146 lines, rebuilt from the two annotation parts) is committed for plan 03-05's layout parser.

## Task Commits

1. **Tracer** (`tdd`): `935321b` test (RED), `5634044` feat (GREEN)
2. **Task 2** (`tdd`): `66b82e9` test (RED), `8ec8714` feat (GREEN)
3. **Task 3** (`tdd`): `5c33255` test (RED), `66043e4` feat (GREEN, round trip 1); `8c5ce41` test (RED), `89f9add` feat (GREEN, diagnostics, round trip 2); `32a8c5e` test (GCC 14.3 DWARF fixture and round trip 3 notices)

No REFACTOR commits were needed. **Plan metadata:** the docs commit that carries this file, STATE.md and ROADMAP.md.

## CI round trips (3 of 3)

| Round trip | Head | testbed | ci | Conclusion |
|------------|------|---------|----|------------|
| 1 | `66043e4` | 39687629155 (failure) | 39687629025 success | failed at the self-test step: the ok case ended with qemu-arm dying of host signal 6 and the report carried no qemu stderr |
| 2 | `89f9add` | 39688983966 (failure) | 39688983977 success | failed at the same step, same cause; the new diagnostics (stderr excerpt and qemu-arm option variants, informational only) showed QEMU 7.2's startup abort `page_set_flags: Assertion 'end - 1 <= GUEST_ADDR_MAX'` |
| 3 | `892f03d` | 40451139101 (job 122014198632) | 40451139087 success | success: all 8 Phase 2 steps, all fixture steps and the self-test step; `result: passed (17 rows)` |

Round trips 1 and 2 stopped the plan on the pre-decided handling "the ok case fails only in the image": the cause was QEMU 7.2 (the commpage at 0xffff0000 against the M-profile `reserved_va` cap of 0x80000000), not the CPU model. The founder decided D-31 (re-pin QEMU) instead of switching the CPU, so D-30 stayed locked. Round trip 3 is the first testbed run on the new image (`ghcr.io/sat-wik/migrationtool-toolchain@sha256:9de8bee18683cc8f5db862e6e657541135f1080b6859ccdeba0fcffa5bd5b565`); plan 03-10 pushed head `892f03d` and recorded the run, and this plan evaluated it as its round trip 3, under the founder's reply `extend 1` (as recorded in plan 03-09 and ROADMAP.md). This plan pushed nothing for it. Annotations of job 122014198632: 44, all level notice, no warning or failure; the `vectors-selftest` and `fixture vectors-selftest-dwarf (1/2, 2/2)` titles are present.

### Round trip 3 evaluation against Task 3

- testbed run 40451139101: path `.github/workflows/testbed.yml`, head `892f03dcda2d78979e511b7815969ef57dcffc0d`, conclusion success; job `testbed` 122014198632 success, 26 steps, step 25 "Run the executor self-test in the pinned image" success.
- ci run 40451139087 success for the same head. container run 40451139089 (branch push, publish skipped) was not awaited.
- `vectors-selftest` notice: `tool qemu-arm qemu-arm version 10.0.2 (Debian 1:10.0.2+ds-2+deb13u1~bpo12+1)`; `tool arm-none-eabi-gcc ... 14.3.1 20250623`; `arm-none-eabi-ar` and `-nm` 2.44.0.20250616; `llvm-dwarfdump` Debian LLVM version 16.0.6; `cpu cortex-m4 listed`; all eight cases with their expected outcome; `no core file after overrun`; `determinism 100 runs 1 hash`; `dwarf enum byte_size 1`; `result: passed (17 rows)`.
- The eight `testbed-*` sections regenerated independently from the annotations equal `03-10-CI-NOTICES.md` from line 5, which equals the re-baselined `02-22-CI-NOTICES.md` from line 5; the three Task 3 automated verifies exit 0.
- `qemu-arm -cpu help` exits 1 in the new image (03-09's heads-up); the self-test reads the CPU list from the output and not from the exit status, so the row passes.

## Files Created/Modified

- `crates/mt-harness/src/*.rs`, `runtime.c`: the executor core described above (about 2400 lines with the template).
- `crates/mt-harness/tests/eqv_04_{classify,selftest,workflow_static}.rs`, `crates/mt-cli/tests/eqv_04_selftest_cli.rs`: the requirement tests.
- `crates/mt-cli/src/main.rs`: the `mt vectors` group with `selftest`.
- `.github/workflows/testbed.yml`: `run_vectors`, `finish_vectors`, the self-test step, header comment.
- `crates/mt-capture/tests/build_02_workflow_static.rs`: the one intended Phase 2 test change, `run_vectors` added to the helper list that may hold a `docker run`; nothing else in that file changed.
- `Cargo.lock` gained only the path crate `mt-harness` (`git diff 949214e -- Cargo.lock` shows one `+name =` line).

## Decisions Made

See `key-decisions` in the frontmatter. Choices the founder may want to revisit: the `timeout` label (rc -9 plus `has_ub` label `timeout`), the environment interpretation of D-02 (the runner's four fixed keys), and the slack-write rc of 0.

## TDD evidence

RED commits hold test code and, for the new crate, the allowed skeleton only (`935321b`: `Cargo.toml`, a doc-only `lib.rs`, the workspace member line, the `mt-cli` dependency line, plus the three tests; `66b82e9`, `5c33255` and `8c5ce41` hold test files and, for `5c33255`, the one-line helper-list change and a dev-dependency). The compile-error and assertion text of each RED run was quoted in the earlier sessions' working notes, which are not preserved in this file; only the commit scopes are re-verifiable here, and the `89f9add` message records the observation that motivated `8c5ce41` (qemu-arm died of host signal 6 with no stderr in the report). GREEN is evidenced by the tests listed under `coverage` and by a full pre-merge gate run on HEAD in this session (`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check`: all exit 0). The host-tool tests ran with `--nocapture` this session with zero `skipping:` lines.

## Deviations from Plan

**1. [Pre-decided handling] QEMU 7.2 cannot start `-cpu cortex-m4` (round trips 1 and 2).** The ok case failed only in the image. Reported to the founder with the measured output; the founder chose D-31 (re-pin QEMU from bookworm-backports) rather than a CPU switch, as the handling requires for a founder decision. Plans 03-09 and 03-10 were inserted between Task 2 and Task 3; they changed no 03-01 code. The CPU model, the emulator and user mode are unchanged (D-30 holds). Image QEMU is 10.0.2 and not the 8.2.x that D-31 names (recorded in 03-09 and 03-10).

**2. [Pre-decided handling / Rule 2] Self-test failure diagnostics (`8c5ce41`, `89f9add`).** After round trip 1 the report had no qemu stderr. Added a bounded one-line stderr excerpt and informational qemu-arm option-variant rows (as-is, no -R, no -seed, cortex-a15, max, guest error log, strace tail). They decide nothing, and the production run and CPU are unchanged. They are why round trip 2 could name the QEMU 7.2 cause.

**3. [Rule 1 - Bug] Static test step extraction (`66043e4`).** The extraction of the self-test step read the next step's name (`actions/upload-artifact`) as part of the step; it now stops at any next step item.

**4. [Minor] `regex` as a dev-dependency of mt-harness (`5c33255`).** Already in `Cargo.lock`, so the lock gained no external crate; it sits in a RED commit with the static tests that need it.

**5. [Minor] `mt-capture` is a dependency of `mt-harness`** (the plan allowed adding it now "if simpler"), so `BASELINE_CFLAGS` is used directly and there is no constant copy to test.

**6. [Minor] `commits` in the frontmatter is 29, measured.** `git rev-list --count 7835375..HEAD` counts plan 03-01's nine commits plus the D-31 planning commits and plans 03-09 and 03-10 interleaved in the paused window. The plan's own commits are the nine in Task Commits.

**7. [Minor] Notice header wording for round trip 3.** Line 3 of `03-01-CI-NOTICES.md` and `03-01-VECTORS-NOTICES.md` follows the 02-22 wording and adds a sentence saying the run is the first on the D-31 image, was recorded by plan 03-10, and is evaluated here as round trip 3. The Phase 2 comparison starts at line 5, so it is unaffected.

**Total deviations:** 7 (2 pre-decided, 1 Rule 1, 4 minor). **Impact:** none on the plan's goal. The D-31 detour cost two CI round trips and two plans, not a design change.

## Known Stubs

None. The only "placeholder" text in `crates/mt-harness/src` is the template's own `@MT_<NAME>@` mechanism.

## Threat Flags

None beyond the plan's register. The register's mitigations are all exercised: T-03-01 by `eqv_04_overrun_is_a_fault` and the CI row `no core file after overrun`, T-03-02 by `eqv_04_timeout_is_recorded` and `eqv_04_mapping_beyond_reserve_is_executor_error`, T-03-03 by the `run_vectors` static test, T-03-04 by the strict frame parse (length equal to `bytes_total`). T-03-SC: no external crate added.

## Issues Encountered

- QEMU 7.2 startup abort (deviation 1). Resolved by D-31.
- Not pushed: the fixture and notices commit `32a8c5e` and this summary's commit are local, as the plan's CI budget requires (the fixture path is under `crates/**`, which would start another testbed run); the next plan that pushes carries them. Plan 03-02 depends on this plan and is next.

## Next Phase Readiness

- Plan 03-02 (wave 3, `depends_on: ["03-01", "03-10"]`) can start: the executor core, the outcome class set, the run path measured in the image, and the GCC 14.3 DWARF fixture exist.
- Plan 03-04 adds the captured-object adapters, `mt-capture`-linked drivers and the separate helper for runs of captured objects on top of exactly these parts; plan 03-05 reads the committed DWARF fixture; plan 03-07 constructs `SetupFailed` from the frame's status word.
- Testbed on `main` still pulls the QEMU 7.2 digest until the Current-image record (892f03d) reaches `main` with a later PR (recorded in 03-10); not a regression.

## Self-Check: PASSED

- Created files found on disk: runtime.c, classify.rs, selftest.rs, the four test files, the DWARF fixture, `03-01-CI-NOTICES.md`, `03-01-VECTORS-NOTICES.md`, plus `crates/mt-cli/src/main.rs` and `.github/workflows/testbed.yml`.
- Commits found as ancestors of HEAD: 935321b, 5634044, 66b82e9, 8ec8714, 5c33255, 66043e4, 8c5ce41, 89f9add, 32a8c5e.
- Plan-level verification: pre-merge gates exit 0; `Cargo.lock` gained only `mt-harness`; no forbidden claim word in `crates/mt-harness`, `testbed.yml` or the notices; testbed run 40451139101 and ci run 40451139087 are green for head 892f03d.
