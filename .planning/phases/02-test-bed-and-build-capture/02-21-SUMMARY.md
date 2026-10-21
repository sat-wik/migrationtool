---
phase: 02-test-bed-and-build-capture
plan: 21
subsystem: testing
tags: [mt-capture, compile-commands-gate, allow-list, gcc-driver-options, response-files, spec-files, testbed]

requires:
  - phase: 02-test-bed-and-build-capture
    provides: "02-20 deny-list gate, record-stage checks and 02-20-CI-NOTICES.md (the baseline the confirming run must equal)"
provides:
  - "ALLOWED_OPTIONS (56 spellings), SHIPPED_SPECS (13 names measured in the pinned image), walk_arguments and CommandWalk, CcPolicy::check_command and CcError::OptionNotAllowed: one operand-aware allow-list walk is the only reader of compile-command options, at capture and at replay"
  - "the @ rule over every argument after argv[0], operands included (T-02-72), and compile-only decided by the walk at both ends (CR-03, IN-13)"
  - "the arm-specs measurement step in container.yml image-a"
  - "pure and live RED/GREEN reproductions of CR-01, CR-02 and CR-03"
  - "one green full-bed testbed run on the final code commit with notices equal to 02-20's from line 5 on (02-21-CI-NOTICES.md)"
  - "the gate's file:line and tests for T-02-67 and T-02-70, for plan 02-22 to collect"
affects: [plan-02-22, phase-2-verify-work, phase-2-secure-phase]

actuals:
  tokens: 25800
  tasks: 3
  commits: 11
plan_head_before: 00e3415024fa34c4a109fd6b4d43fd733eefa582
plan_head_after: 79fe1925dc8473ccbb5dd3626cbaa371cadbd957
commits: 11

tech-stack:
  added: []
  patterns:
    - "one walk, one table: walk_arguments over ALLOWED_OPTIONS serves the -c test, derive_output and check_write_paths; CcPolicy::check_command is the whole gate and both canonical_entry and check_entry call it"
    - "refuse by default: an argument that is not a measured spelling ends the walk with its cause; the walk never guesses what follows an unknown option"
    - "constants that widen a gate carry their measurement (sources, counts, run ids) and change only test first"

key-files:
  created:
    - crates/mt-capture/tests/build_01_gate_bypasses.rs
    - crates/mt-capture/tests/build_02_replay_bypasses.rs
    - .planning/phases/02-test-bed-and-build-capture/02-21-CI-NOTICES.md
  modified:
    - crates/mt-capture/src/cc.rs
    - crates/mt-capture/src/capture.rs
    - crates/mt-capture/tests/build_01_compile_commands.rs
    - crates/mt-capture/tests/build_02_replay.rs
    - crates/mt-capture/tests/build_01_capture_stages.rs
    - crates/mt-capture/tests/build_01_capture_drivers.rs
    - .github/workflows/container.yml

key-decisions:
  - "Cause taxonomy as planned: option_not_allowed (new) for any unlisted argument and a trailing -I; helper_option kept for @ arguments (operands included) and unshipped or malformed -specs; write_redirect kept for a trailing -MF, -MT or -MQ; forbidden_flag, dangling_output, outside_run_root, not_compile_only and compiler_not_allowed unchanged"
  - "Operand model: AllowedOption { spelling, operand }, Operand::{None, JoinedOrSeparate(kind), JoinedOnly(kind)}, OperandKind::{OutputPath, DepfilePath, TargetName, IncludeDirectory, Macro, SpecName}; exact spellings are matched before operand prefixes"
  - "-Wmissing-variable-declarations added as the 56th spelling under the pre-decided measurement-gap handling (testbed run 39453293771, cjson/default and cjson/no-locale)"
  - "SHIPPED_SPECS = the 13 names of the arm-specs notice of container run 39453293834; all 13 read the toolchain's file under /opt/arm-gnu-toolchain and the planted control read the planted file"
  - "The response-file positive control uses a file whose first word is a harmless operand (-I.), because in operand position the driver takes the file's first word as the operand"

patterns-established:
  - "Live bypass table: each refused spelling first runs unguarded through the runner and must write its file outside the working directory, so a refusal test can never pass vacuously"

requirements-completed: []  # BED-01, BED-02, BUILD-01, BUILD-02, BUILD-03 are this plan's requirements; not marked complete here, verify-work reconciles them after 02-22

coverage:
  - id: D1
    description: "G-02-8 missing 1: one operand-aware walk (walk_arguments over ALLOWED_OPTIONS) is the only reader of options, used by the -c test, derive_output and check_write_paths, at capture and at replay before any spawn"
    requirement: "BUILD-01"
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/build_01_compile_commands.rs#build_01_one_walk_serves_compile_only_output_and_write_paths"
        status: pass
      - kind: unit
        ref: "crates/mt-capture/tests/build_01_gate_bypasses.rs#build_01_gate_decides_compile_only_from_the_walk"
        status: pass
      - kind: integration
        ref: "crates/mt-capture/tests/build_01_capture_stages.rs#build_01_record_stage_refuses_an_operand_compile_flag_in_the_export"
        status: pass
    human_judgment: false
  - id: D2
    description: "G-02-8 missing 2: measured allow-list plus depfile flags and shipped -specs names; everything else refused with a named cause; CR-01..CR-03 RED first, refused pure and live"
    requirement: "BUILD-02"
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/build_01_compile_commands.rs#build_01_allow_list_is_the_measured_set"
        status: pass
      - kind: unit
        ref: "crates/mt-capture/tests/build_01_compile_commands.rs#build_01_allow_list_refuses_everything_else"
        status: pass
      - kind: unit
        ref: "crates/mt-capture/tests/build_01_gate_bypasses.rs (four tests)"
        status: pass
      - kind: integration
        ref: "crates/mt-capture/tests/build_02_replay_bypasses.rs#build_02_replay_refuses_the_measured_bypasses"
        status: pass
    human_judgment: false
  - id: D3
    description: "WR-13: -specs=<name> only for the 13 names measured in the pinned image; non-empty; unshipped, path and empty names refused"
    requirement: "BUILD-01"
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/build_01_compile_commands.rs#build_01_shipped_specs_are_measured"
        status: pass
      - kind: unit
        ref: "crates/mt-capture/tests/build_01_compile_commands.rs#build_01_specs_allowed_only_for_shipped_names"
        status: pass
      - kind: integration
        ref: "crates/mt-capture/tests/build_01_capture_stages.rs#build_01_shipped_spec_names_are_read_from_the_toolchain"
        status: pass
      - kind: e2e
        ref: "container run 39453293834, job image-a 118734041778, step 'List the spec files the pinned Arm toolchain ships (WR-13)' success, arm-specs notice"
        status: pass
    human_judgment: false
  - id: D4
    description: "G-02-8 missing 4 (this plan's part): no real pair refused: 29 local captures canonical and raw, 2 tinycbor run exports, both CI fixtures, and a confirming full-bed run"
    requirement: "BED-01"
    verification:
      - kind: other
        ref: "MT_LOCAL_CAPTURES=$PWD/.mt-work/capture cargo test -p mt-capture --test build_01_compile_commands -- --ignored --exact build_01_local_captures_pass_the_compile_command_gate --nocapture (files=29 raw_entries=531 canonical_entries=481 run_exports=2 run_export_entries=20 refused=0)"
        status: pass
      - kind: e2e
        ref: "testbed run 39455562978 success on 44a3dd6; diff of 02-21-CI-NOTICES.md against 02-20-CI-NOTICES.md from line 5 is empty; ci run 39455563004 success on 44a3dd6"
        status: pass
    human_judgment: false

duration: 45min
completed: 2026-10-20
status: complete
---

# Phase 2 Plan 21: Allow-list compile-command gate (gap G-02-8, decisions 1-3) Summary

**The 02-20 deny-list gate is replaced by one operand-aware allow-list walk (`walk_arguments` over 56 measured spellings, plus 13 shipped `-specs=` names measured in the pinned image). The walk refuses every `@` argument (operands included), every long spelling and every carrier. It decides compile-only, the output and the write paths at capture and at replay. CR-01, CR-02 and CR-03 are refused, both pure and live, and a full-bed run on the final code captures and replays all 30 retained pairs with notices identical to 02-20's.**

## Performance

- **Duration:** 45 min (2026-10-20T15:38:56Z to 16:23:57Z)
- **Tasks:** 3 of 3
- **Files modified:** 10 (7 modified, 3 created)
- **CI round trips:** 2 of the cap of 3

## Accomplishments

- `cc.rs`: `ALLOWED_OPTIONS`, `SHIPPED_SPECS`, `walk_arguments`, `CommandWalk`, `CcPolicy::check_command`, `CcError::OptionNotAllowed`. `derive_output` and `check_write_paths` are rebuilt on the walk. The deny-list code is gone: `HELPER_OPTION_PREFIXES`, `is_helper_option`, `find_helper_option`, `PREPROCESSOR_WRITE_PREFIXES`, `ASSEMBLER_WRITE_PREFIXES`, their two predicates, `SEPARATE_OPERAND_OPTIONS` and the old `check_write_paths` body.
- `capture.rs`: the `option_not_allowed` cause. The module doc names the walk and its causes, and its trust-model bullet cites the record-stage tests.
- `container.yml`: new image-a step "List the spec files the pinned Arm toolchain ships (WR-13)", which publishes the `arm-specs` notice.
- Pure reproductions (`build_01_gate_bypasses.rs`) and live reproductions (`build_02_replay_bypasses.rs`) of CR-01, CR-02 and CR-03, and live response-file controls (IN-15).
- Confirming full-bed run: testbed run 39455562978 and ci run 39455563004 are green for 44a3dd6, and 02-21-CI-NOTICES.md equals 02-20-CI-NOTICES.md from line 5 on (0 differing lines out of 336).

## Task Commits

| # | Commit | Type | Subject |
|---|--------|------|---------|
| Tracer RED | `68e0bc7` | test | reproduce CR-01..CR-03 and specify the allow-list gate (RED) |
| Tracer GREEN | `8da9874` | feat | allow-list compile-command gate on one operand-aware walk (G-02-8) |
| Task 2 test | `148715f` | test | prove the allow-list gate live at capture, the drivers and replay (G-02-8) |
| Task 2 docs | `6725d44` | docs | name the allow-list causes in the capture trust model |
| Deviation docs | `b82f315` | docs | state the measured response-file expansion in the gate doc |
| Task 3 CI step | `4818d97` | ci | list the spec files the pinned Arm toolchain ships (WR-13) |
| RT1 gap RED | `1270139` | test | require the CI-only warning cjson passes under GCC 14 (RED) |
| RT1 gap GREEN | `2940852` | feat | allow -Wmissing-variable-declarations, measured in CI for cjson |
| WR-13 RED | `78a12ea` | test | require the spec names measured from the pinned image (WR-13 RED) |
| WR-13 GREEN | `44a3dd6` | feat | allow -specs= for the 13 names the pinned toolchain ships (WR-13) |
| Task 3 notices | `79fe192` | docs | record the confirming full-bed run's testbed notices |

`44a3dd6` is the plan's final code commit. The run the notices cite was made on it, and no later commit touches a path in testbed.yml's push trigger (that verify passed).

## RED evidence (TDD rule)

Each RED commit holds test code only and was run against the unchanged implementation.

**Tracer RED (`68e0bc7`, against the 02-20 gate):**

- `build_01_gate_bypasses`: 1 passed, 3 failed.
  - `build_01_gate_refuses_long_option_spellings` failed. All 7 CR-01 spellings were "accepted at capture" and "accepted at replay" (`--plugin=/work/run/t/default/evil.so`, `--compare-debug=-wrapper,...`, `--opt-info-all=/out/leak.txt`, `--dump-tree-all=/out/leak`, `--dump-final-insns=/out/f`, `--profile-use=/out/p`, `--profile-dir=/out/pd`). Assessment: the deny-list compared `-f` spellings only, and the driver rewrites `--<name>` into `-f<name>`, exactly as CR-01 says.
  - `build_01_gate_refuses_assembler_carrier_spellings` failed. All 6 CR-02 spellings were accepted at both ends (`-Wa,-MD,`, `-Wa,-MD=`, `-Wa,--M,`, `-Wa,-M,`, `-Xassembler -MD -Xassembler /out/e.d`, `--for-assembler=-MD,`). Assessment: the assembler write-prefix list missed single-dash and abbreviated GNU `as` options (CR-02).
  - `build_01_gate_decides_compile_only_from_the_walk` failed. The review vector, the minimal `-MD -MT -c a.c -o out/a.o` and `-Wl,-plugin,...` were accepted at both ends. A replay entry without `-c` gave "expected NotCompileOnly, got Ok(())". Assessment: the `-c` test accepted an operand `-c` (CR-03), and `check_entry` had no `-c` test at all (IN-13).
  - `build_01_gate_refuses_response_files_in_operand_position` passed. Assessment: as planned, this is a guard, because the 02-20 finder already scanned operands for `@`.
- `build_01_compile_commands` did not compile:
  - `error[E0432]: unresolved imports mt_capture::cc::ALLOWED_OPTIONS, CommandWalk, Operand, OperandKind, SHIPPED_SPECS, walk_arguments`
  - 2 x `error[E0599]: no method named check_command found for struct CcPolicy`
  - 2 x `error[E0599]: no variant named OptionNotAllowed found for enum CcError`

  Assessment: every new API item is named.

**Task 2 RED (`148715f`'s four test files run in a detached worktree at `68e0bc7`):**

- The combined `cargo test ... --test build_02_replay_bypasses --test build_02_replay --test build_01_capture_stages --test build_01_capture_drivers -- --nocapture` stops at build_02_replay: `error[E0599]: no variant named OptionNotAllowed found for enum CcError` (2x). Assessment: expected, the decided variant did not exist. The other three binaries were then run one by one, because cargo builds nothing else after that error.
- `build_02_replay_bypasses` FAILED with these lines, each written by the 02-20 gate's replay. Assessment: every measured bypass ran and wrote outside the run root at replay.
  ```text
  CR-01 --dump-final-insns=: replay returned Ok(..) and wrote ["f"]
  CR-01 --opt-info-all=: replay returned Ok(..) and wrote ["o"]
  CR-02 -Wa,-MD,: replay returned Ok(..) and wrote ["e1.d"]
  CR-02 -Wa,-MD=: replay returned Ok(..) and wrote ["e2.d"]
  CR-02 -Wa,--M,: replay returned Ok(..) and wrote ["e3.d"]
  CR-02 -Xassembler -MD -Xassembler: replay returned Ok(..) and wrote ["x1.d"]
  CR-03 -MD -MT -c ... -Wl,-Map=: replay returned Ok(..) and wrote ["m.map"]
  minimal CR-03: replay returned Ok(..)
  minimal CR-03: the original object was deleted
  ```
- `build_01_capture_stages`: 12 passed, 2 failed.
  - `build_01_record_stage_refuses_a_helper_option_in_the_export`: left `["helper_option"]`, right `["option_not_allowed"]`. Assessment: the old cause.
  - `build_01_record_stage_refuses_an_operand_compile_flag_in_the_export`: left `Captured`, right `CaptureFailed`. Assessment: CR-03 got through the 02-20 record stage.
  - `build_01_record_stage_refuses_an_object_built_for_another_abi` passed. Assessment: a guard, as planned.
- `build_01_capture_drivers`: `build_01_capture_drivers_fail_closed` failed with left `["arch_gate"]`, right `["option_not_allowed"]`. Assessment: `-mfloat-abi=soft` passed the deny-list.
- None of the runs printed a `skipping:` line. The worktree was removed with `git worktree remove --force`, and `git worktree list` shows one worktree.

**Round trip 1 measurement gap RED (`1270139`):** `build_01_allow_list_is_the_measured_set` failed. The left list lacks `-Wmissing-variable-declarations` between `-Wmissing-prototypes` and `-Wnested-externs`. Assessment: the spelling was not on the list yet.

**WR-13 RED (`78a12ea`, against the empty `SHIPPED_SPECS`):**

- `build_01_allow_list_is_the_measured_set`: left `[]`, right the 13 names.
- `build_01_shipped_specs_are_measured`: "SHIPPED_SPECS is empty: the names measured from the pinned image are missing".
- `build_01_specs_allowed_only_for_shipped_names`: "-specs=aprofile-validation-v2m.specs is refused at capture: HelperOption { flag: "-specs=aprofile-validation-v2m.specs" }".
- `build_01_shipped_spec_names_are_read_from_the_toolchain`: "no name of SHIPPED_SPECS (0) is shipped by the host toolchain".

Assessment: all four fail because the set is empty, which is the fail-safe direction.

**GREEN:**

- Tracer verify: build_01_gate_bypasses 4/4, build_01_compile_commands 18 passed and 1 ignored, build_01_ci_fixtures 6/6.
- End-to-end guards: `build_02_replay_matches_original_objects` and `build_02_replay_refuses_non_allowlisted_compiler` 2 passed; `build_01_capture_make_with_bear_end_to_end` and `build_01_capture_cmake_end_to_end` 2 passed, with no skip line.
- Task 2 on HEAD: build_02_replay_bypasses 1/1, build_02_replay 12/12, build_01_capture_drivers 19/19, build_01_capture_stages 14/14, with no skip line.
- Task 3 on HEAD: build_01_compile_commands 20 passed and 1 ignored, build_01_capture_stages 15/15, with no skip line.
- `cargo test --workspace --locked` is green (44 test-result lines, none failed).

## Changed 02-20 expectations (each from founder decisions 1-3 unless noted)

| Test | Old expectation | New expectation |
|------|-----------------|-----------------|
| build_01_rejects_helper_executing_driver_options | `HELPER_OPTION_PREFIXES` equals 9 prefixes; all 17 cases `HelperOption`; CMake `-wrapper` `HelperOption` | constant equality and finder calls removed; `HelperOption` for `-specs=x.specs`, `-specs` + `x.specs` (flag `-specs`) and `@opts.rsp`; `OptionNotAllowed` for the other 14 (`-Xpreprocessor` case names `-Xpreprocessor`, `-Wp,` cases name the whole argument); CMake `-wrapper` `OptionNotAllowed { "-wrapper" }` |
| build_01_driver_option_gate_keeps_ordinary_flags | kept `--sysroot=`, `-isysroot`, `-Wa,-mthumb`, `-Xassembler -mthumb`, `-fcompare-debug`, `-include`, `-iquote`, `--write-dependencies`, `-dumpdir`, `-dumpbase`, `-dumpbase-ext`, `-save-temps=obj`, `-fstack-usage`, `-fdump-tree-all`, `-fopt-info-vec=stderr`, `-aux-info=`, `--output=` | kept set is every exact measured spelling, `-DAT=@`, `-DNAME=1`, `-Iinc`, `-I inc`, the CMake depfile shape, `-MMD`, `-MP`, `-MQ /anything`, `-MTtgt`, `-MFout/a.d`, `-o out/a.o`; the 17 flags above moved to build_01_allow_list_refuses_everything_else as `OptionNotAllowed` |
| build_01_handles_write_path_options | `--output` resolved like `-o`; 18 `OutsideRunRoot` cases including `--output`, `-dumpdir`, `-dumpbase`, `-aux-info`, `-fdump-*=`, `-fopt-info*=`, `-fdeps-file=`; 13 `WriteRedirect` cases including carriers | `-o a.o -o b.o` gives `b.o`; 7 `OutsideRunRoot` cases (`-o`, `-MF` joined and separate, climbing `-MF`, the decoy, first-of-two `-MF`); `WriteRedirect` for a trailing `-MF`, `-MT`, `-MQ`; `OptionNotAllowed` for `--output=`, `--output`, `-dumpdir`, `-aux-info`, `-fdump-tree-all=`, `-fdeps-file=`, `-Wp,-MD,`, `-Xpreprocessor -MF ...` |
| build_01_local_captures_pass_the_compile_command_gate | canonical and raw files; prints `files raw_entries canonical_entries refused` | also the run export of every capture directory without a raw file; prints `run_exports run_export_entries` as well; fails on an unset or empty variable |
| build_02_replay_refuses_helper_executing_driver_options | 13 cases all `HelperOption`; `-Xpreprocessor` case names its operand | `HelperOption` for `-specs=x.specs`, `@<rsp>` and `-I @<rsp>`; `OptionNotAllowed` for the rest (`--specs` names `--specs`, `-Wp,@<rsp>` the whole argument, `-Xpreprocessor` the carrier); new live controls `[@<rsp>]` and `["-I", "@<rsp>"]` |
| build_02_replay_refuses_writes_outside_the_run_root | `--output=<o>/x.o` `OutsideRunRoot`; `-Wp,-MD,<o>/wp.d` `WriteRedirect` | `--output=<o>/x.o` `OptionNotAllowed` with that argument; `-MD -MF <o>/x.d` stays `OutsideRunRoot`; `-Wp,-MD,<o>/wp.d` `OptionNotAllowed` with that argument |
| build_01_record_stage_refuses_a_helper_option_in_the_export | causes `["helper_option"]` | causes `["option_not_allowed"]`, detail names `-wrapper` |
| build_01_capture_drivers_fail_closed (soft-float case) | causes `["arch_gate"]`, `gates.arch_ok == false` | causes `["option_not_allowed"]`, detail names `-mfloat-abi=soft`; the arch gate keeps its end-to-end test in build_01_record_stage_refuses_an_object_built_for_another_abi |
| probes() response file (build_02_replay.rs) | holds `-wrapper <wrapper>` | holds `-I. -wrapper <wrapper>` (deviation 1; the refusal cases are unchanged) |

The regression guards named in the plan pass unchanged.

## Planner choices (recorded)

- **Cause taxonomy:** as in the plan's table. It is implemented in `walk` (cc.rs:394), `missing` (the trailing-operand causes) and `check_command` (cc.rs:614), and documented in the cc.rs module-doc table.
- **Names:** `ALLOWED_OPTIONS` (a table of `AllowedOption { spelling, operand }`), `Operand`, `OperandKind`, `SHIPPED_SPECS`, `CommandWalk { compile_only, outputs, dep_files }`, `walk_arguments`, `CcPolicy::check_command`. A private `walk` also returns the `-o` and `-MF` operands in order of appearance, so `check_write_paths` reports the first outside path as written.
- **WR-13:** measured in container.yml image-a. testbed.yml is not touched.
- **Task split:** the Tracer holds the pure gate. Task 2 holds the live and stage tests, whose RED ran in the detached worktree. Nothing was pushed before Task 3's pre-merge gates passed.

## Local proofs (fail closed)

- `MT_LOCAL_CAPTURES=$PWD/.mt-work/capture cargo test -p mt-capture --test build_01_compile_commands -- --ignored --exact build_01_local_captures_pass_the_compile_command_gate --nocapture` prints `files=29 raw_entries=531 canonical_entries=481 run_exports=2 run_export_entries=20 refused=0`. The output is the same after the Tracer, after the CI-only addition and after WR-13.
- The jq corroboration over the 29 canonical files and both CI fixtures (11243 arguments) finds no long spelling, `@`, carrier, `-specs`, `-wrapper`, `-B`, `-fplugin`, `-include` or `-x`.
- Both CI fixtures pass `check_entry`, and `canonical_output` equals their `output` (build_01_driver_option_gate_keeps_ordinary_flags).

## Driver probes (re-run at execution)

- With `-###` on the host arm-none-eabi-gcc 13.2.1, `-MD -MT -c t.c -o t.o` gives 1 collect2 line, and `-c t.c -I -c -o t.o` gives 0.
- Response files (corrected measurement, deviation 1). With `o2.rsp` holding `-I. -wrapper <w.sh>`, each of `-I @o2.rsp`, `-MT @o2.rsp`, `-MF @o2.rsp`, `-o @o2.rsp` and `@o2.rsp` produced 2 wrapper lines under arm `-###`, and its real host cc 13.3.0 compile left the wrapper's marker. With the planning-time `x.rsp` (`-wrapper /bin/echo,viarsp`), `-I @x.rsp` gives 0 lines that start with `/bin/echo`: the driver warns `/bin/echo,viarsp: linker input file unused`, because `-wrapper` became the `-I` operand.
- The host compilers are cc 13.3.0 (Ubuntu 13.3.0-6ubuntu2~24.04.1) and GNU as 2.42. The live table in build_02_replay_bypasses re-measures each CR write on every run.

## CI round trips (cap 3, used 2)

| RT | Head | Runs (id: conclusion) | Result |
|----|------|------------------------|--------|
| 1 | `4818d97` | testbed 39453293771: failure (step "Record the captures"); ci 39453293792: success; container 39453293834: image-a 118734041778 success, spec-files step success | cjson/default and cjson/no-locale `capture_failed (option_not_allowed)` on `-Wmissing-variable-declarations`; the other 28 pairs captured; the image/validate/fetch/seeds/build notices equal 02-20's |
| 2 | `44a3dd6` | testbed 39455562978: success (job 118741910540); ci 39455563004: success; container 39455563052: not awaited | confirming run: notices equal 02-20's from line 5 on |

Round trip 2 green criteria, all met:

- testbed-fetch has 16 rows, and the tree hashes equal `testbed/manifest.toml`, compared programmatically (16 of 16).
- testbed-build: 30 pairs built.
- testbed-seeds: `passed (16 rows)` with 02-20's counts.
- testbed-capture: `passed (46 rows)` with 30 pairs captured, every gate row `compiler_allow_listed=true arch_ok=true undeclared=0 timestamp_macro=0`, 16 Makefile2 lines, 14 variants `changes_code true`, and paho/server `dropped (entry_mismatch)`.
- testbed-replay: `passed (31 rows)`, 30 of them identical.
- testbed-determinism: `identical`, compare exit code 0, 74 files.

The notices diff (`sed '1,4d'` of both files) is empty. The docs push of the notices, SUMMARY and state commits triggers only ci and is not a round trip.

### arm-specs notice (container run 39453293834, job 118734041778, head 4818d97)

```text
arm-none-eabi-gcc (Arm GNU Toolchain 14.3.Rel1 (Build arm-14.174)) 14.3.1 20250623
--- shipped
aprofile-validation-v2m.specs /opt/arm-gnu-toolchain/bin/../lib/gcc/arm-none-eabi/14.3.1/../../../../arm-none-eabi/lib/aprofile-validation-v2m.specs
aprofile-validation.specs /opt/arm-gnu-toolchain/bin/../lib/gcc/arm-none-eabi/14.3.1/../../../../arm-none-eabi/lib/aprofile-validation.specs
aprofile-ve-v2m.specs /opt/arm-gnu-toolchain/bin/../lib/gcc/arm-none-eabi/14.3.1/../../../../arm-none-eabi/lib/aprofile-ve-v2m.specs
aprofile-ve.specs /opt/arm-gnu-toolchain/bin/../lib/gcc/arm-none-eabi/14.3.1/../../../../arm-none-eabi/lib/aprofile-ve.specs
iq80310.specs /opt/arm-gnu-toolchain/bin/../lib/gcc/arm-none-eabi/14.3.1/../../../../arm-none-eabi/lib/iq80310.specs
linux.specs /opt/arm-gnu-toolchain/bin/../lib/gcc/arm-none-eabi/14.3.1/../../../../arm-none-eabi/lib/linux.specs
nano.specs /opt/arm-gnu-toolchain/bin/../lib/gcc/arm-none-eabi/14.3.1/../../../../arm-none-eabi/lib/nano.specs
nosys.specs /opt/arm-gnu-toolchain/bin/../lib/gcc/arm-none-eabi/14.3.1/../../../../arm-none-eabi/lib/nosys.specs
pid.specs /opt/arm-gnu-toolchain/bin/../lib/gcc/arm-none-eabi/14.3.1/../../../../arm-none-eabi/lib/pid.specs
rdimon-v2m.specs /opt/arm-gnu-toolchain/bin/../lib/gcc/arm-none-eabi/14.3.1/../../../../arm-none-eabi/lib/rdimon-v2m.specs
rdimon.specs /opt/arm-gnu-toolchain/bin/../lib/gcc/arm-none-eabi/14.3.1/../../../../arm-none-eabi/lib/rdimon.specs
rdpmon.specs /opt/arm-gnu-toolchain/bin/../lib/gcc/arm-none-eabi/14.3.1/../../../../arm-none-eabi/lib/rdpmon.specs
redboot.specs /opt/arm-gnu-toolchain/bin/../lib/gcc/arm-none-eabi/14.3.1/../../../../arm-none-eabi/lib/redboot.specs
--- control (planted)
only-planted.specs only-planted.specs
exit code: 0
```

Measured set: all 13 names, each read from under `/opt/arm-gnu-toolchain/`. The control read the planted file, so the measurement is live. The names are committed in byte order in `SHIPPED_SPECS` (`44a3dd6`).

## Closure table

| Item | Commits | Tests | CI evidence |
|------|---------|-------|-------------|
| G-02-8 missing 1 (one walk) | 68e0bc7, 8da9874, 148715f | build_01_one_walk_serves_compile_only_output_and_write_paths, build_01_gate_decides_compile_only_from_the_walk, build_01_record_stage_refuses_an_operand_compile_flag_in_the_export | testbed 39455562978 (30 pairs captured and replayed through the walk) |
| G-02-8 missing 2 (measured allow-list, named causes) | 68e0bc7, 8da9874, 148715f, 1270139, 2940852, 78a12ea, 44a3dd6 | build_01_allow_list_is_the_measured_set, build_01_allow_list_refuses_everything_else, build_01_rejects_helper_executing_driver_options, build_01_handles_write_path_options, build_02_replay_refuses_the_measured_bypasses | ci 39455563004 (the live bypass table runs in ci) |
| G-02-8 missing 4, local | 8da9874 | build_01_local_captures_pass_the_compile_command_gate (refused=0) | n/a |
| G-02-8 missing 4, fixtures | 8da9874 | build_01_driver_option_gate_keeps_ordinary_flags, build_01_ci_fixtures | ci 39455563004 |
| G-02-8 missing 4, CI | 44a3dd6, 79fe192 | build_02_drop_log, build_03_committed_seeds, build_02_workflow_static | testbed 39455562978, notices equal to 02-20's |
| CR-01 | 68e0bc7, 8da9874, 148715f | build_01_gate_refuses_long_option_spellings, build_02_replay_refuses_the_measured_bypasses | ci 39455563004 |
| CR-02 | 68e0bc7, 8da9874, 148715f | build_01_gate_refuses_assembler_carrier_spellings, build_02_replay_refuses_the_measured_bypasses | ci 39455563004 |
| CR-03 | 68e0bc7, 8da9874, 148715f | build_01_gate_decides_compile_only_from_the_walk, build_02_replay_refuses_the_measured_bypasses, build_01_record_stage_refuses_an_operand_compile_flag_in_the_export | ci 39455563004 |
| WR-13 (part 1) | 4818d97, 78a12ea, 44a3dd6 | build_01_shipped_specs_are_measured, build_01_specs_allowed_only_for_shipped_names, build_01_shipped_spec_names_are_read_from_the_toolchain | container 39453293834 arm-specs |
| IN-13 | 8da9874 | build_01_gate_decides_compile_only_from_the_walk (replay entry without -c) | - |
| IN-14 | 8da9874 | build_01_one_walk_serves_compile_only_output_and_write_paths | - |
| IN-15 | 148715f | build_02_replay_refuses_helper_executing_driver_options (live `@rsp` and `-I @rsp` controls), build_02_replay_bypasses (live table) | ci 39455563004 |

G-02-8 missing 3 (WR-12, WR-14) and missing 5 (the T-02-67/T-02-70 re-verification) belong to plan 02-22. This plan does not edit 02-SECURITY.md, 02-UAT.md, 02-REVIEW-DISPOSITION.md or ROADMAP.md. The GSD roadmap progress update was reverted for that reason, and the requirement checkboxes are left to verify-work after 02-22.

## Threat evidence (this plan's register)

- **T-02-72 (response files in operand position):**
  - Code: crates/mt-capture/src/cc.rs:398, the `@` rule in `walk`, before the option walk.
  - Tests: build_01_gate_refuses_response_files_in_operand_position; build_02_replay_refuses_helper_executing_driver_options (live `-I @<rsp>` control, then the `HelperOption { "@<rsp>" }` refusal with no marker); build_01_rejects_helper_executing_driver_options.
- **T-02-73 (linker for an operand `-c`):**
  - Code: crates/mt-capture/src/cc.rs:634, `compile_only` from the walk required in `check_command`. `-Wl,`, `-Xlinker` and `--for-linker` reach the `OptionNotAllowed` fall-through at cc.rs:462.
  - Tests: build_01_gate_decides_compile_only_from_the_walk; build_02_replay_refuses_the_measured_bypasses (live `m.map` write refused); build_01_record_stage_refuses_an_operand_compile_flag_in_the_export; build_01_one_walk_serves_compile_only_output_and_write_paths.
- **T-02-74 (unshipped `-specs=`):**
  - Code: crates/mt-capture/src/cc.rs:365 (`SHIPPED_SPECS`) and cc.rs:437 (the `-specs` rule).
  - Tests: build_01_shipped_specs_are_measured, build_01_specs_allowed_only_for_shipped_names, build_01_shipped_spec_names_are_read_from_the_toolchain.
- **T-02-75 (over-refusal of a real pair):**
  - Code: crates/mt-capture/src/cc.rs:290 (`ALLOWED_OPTIONS`, measured).
  - Evidence: local refused=0; testbed run 39455562978 captures every pair. The one CI-only flag, `-Wmissing-variable-declarations`, was added test first with its CI evidence. It is of the named warning family.
- **T-02-78 (spec names crossing from an annotation into the constant):**
  - Names come only from the arm-specs notice of container run 39453293834 for head 4818d97. Its control read the planted file, and each name is a bare `<name>.specs` whose path lies under `/opt/arm-gnu-toolchain`.
  - The constant's comment cites the run, job and head. The equality test makes any change deliberate. The verify checks that the cited run is a container run whose spec-files step succeeded.
- **T-02-SC:** no package installed, no crate added.

## Gate file:line for T-02-67 and T-02-70 (for plan 02-22)

| What | Where | Proven by |
|------|-------|-----------|
| `walk_arguments`, the one walk (private `walk` holds the body) | crates/mt-capture/src/cc.rs:519 (`walk` at :394) | build_01_one_walk_serves_compile_only_output_and_write_paths, build_01_allow_list_refuses_everything_else |
| the `@` rule | crates/mt-capture/src/cc.rs:398 | build_01_gate_refuses_response_files_in_operand_position, build_02_replay_refuses_helper_executing_driver_options |
| unlisted argument refused (`OptionNotAllowed`) | crates/mt-capture/src/cc.rs:462 | build_01_gate_refuses_long_option_spellings, build_01_gate_refuses_assembler_carrier_spellings, build_02_replay_refuses_the_measured_bypasses |
| `CcPolicy::check_command`, the whole gate | crates/mt-capture/src/cc.rs:614 | build_01_one_walk_serves_compile_only_output_and_write_paths, build_01_gate_decides_compile_only_from_the_walk |
| `check_write_paths` on the walk's `-o` and `-MF` operands (`writes_inside` :552) | crates/mt-capture/src/cc.rs:576 | build_01_handles_write_path_options, build_02_replay_refuses_writes_outside_the_run_root, build_01_record_stage_refuses_a_write_outside_the_run_root_in_the_export |
| capture: `canonical_entry` calls `check_command` | crates/mt-capture/src/cc.rs:733 | build_01_record_stage_refuses_a_helper_option_in_the_export, build_01_record_stage_refuses_an_operand_compile_flag_in_the_export |
| replay: `check_entry` calls `check_command` | crates/mt-capture/src/cc.rs:656-657 | build_02_replay_refuses_the_measured_bypasses |
| replay.rs `validate`, before any delete, copy or spawn | crates/mt-capture/src/replay.rs:149 `plan.policy.check_entry(entry)?;` (unchanged) | build_02_replay_refuses_the_measured_bypasses (original object present, outside directory empty) |
| cause mapping | crates/mt-capture/src/capture.rs:1219 `CcError::OptionNotAllowed { .. } => "option_not_allowed"` | build_01_record_stage_refuses_a_helper_option_in_the_export |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] The planning-time response-file measurement was misread, so the planned positive control was dead**
- **Found during:** Task 2 (the GREEN run of build_02_replay)
- **Issue:** with the shared probe file holding `-wrapper <wrapper>`, `assert_probe_is_live(["-I", "@<rsp>"])` left no marker. The driver splices the file's words in place of the `@` argument, so `-wrapper` became the `-I` operand. The planning-time `-###` count matched the input-file name `/bin/echo,viarsp`, not a wrapper run. T-02-72 is still real: when the file starts with a word the option consumes, every operand position runs the wrapper (measured, see Driver probes).
- **Fix:** the shared probe file now holds `-I. -wrapper <wrapper>`, which runs the probe both alone and as an operand. The comments in build_01_gate_bypasses.rs (in 148715f) and the cc.rs module doc (b82f315, a separate docs commit) state the corrected measurement. No refusal case or assertion was weakened.
- **Files modified:** crates/mt-capture/tests/build_02_replay.rs, crates/mt-capture/tests/build_01_gate_bypasses.rs, crates/mt-capture/src/cc.rs (doc only)
- **Commits:** 148715f, b82f315

**2. [Pre-decided handling - measurement gap] `-Wmissing-variable-declarations` added (ALLOWED_OPTIONS has 56 spellings, not 55)**
- **Found during:** Task 3, round trip 1 (testbed run 39453293771)
- **Issue:** two pairs, cjson/default and cjson/no-locale, read `capture_failed (option_not_allowed)`: "argument -Wmissing-variable-declarations is not on the compile-command allow-list". The pair's export is refused at its first unlisted argument, and the cause detail names only that argument. cjson's CMakeLists.txt adds the flag only when `check_c_compiler_flag` accepts it. GCC 14 does, and the host GCC 13.2.1 of the local measurement does not.
- **Fix:** RED in build_01_allow_list_is_the_measured_set (1270139), then GREEN with the run id, job id, head and pairs in the constant's comment (2940852). Corroborated offline: a GCC 14.2.1 (xpack) configure of cjson, for both configurations, exports no other spelling outside the list, and that flag is the only one in cjson's list whose support differs between GCC 13 and 14. The local proofs were re-run (refused=0), and round trip 2 was used within the cap.
- **Files modified:** crates/mt-capture/src/cc.rs, crates/mt-capture/tests/build_01_compile_commands.rs
- **Commits:** 1270139, 2940852

**3. [Rule 3 - Blocking] The disk filled during the Task 2 RED worktree build**
- **Issue:** the first build in the detached worktree failed with "No space left on device" (36K free).
- **Fix:** I removed the worktree's partial target directory and the regenerable `target/debug/incremental` cache of the main checkout. The RED run was then built with `CARGO_TARGET_DIR` set to the main target and `CARGO_INCREMENTAL=0`, so registry dependencies were reused. No tracked file or evidence was affected.

**4. [Test shape] Small additions and orderings that do not change what the plan's tests prove**
- **Positive controls:** build_02_replay_bypasses runs them in a control copy of the source directory, so the unguarded runs never touch the replay fixture's own `run/src`.
- **build_01_allow_list_refuses_everything_else:** also covers `-wrapper /bin/echo` (18 unmeasured cases).
- **build_01_driver_option_gate_keeps_ordinary_flags:** also inserts `BASELINE_CFLAGS`.
- **build_01_specs_allowed_only_for_shipped_names:** asserts the list equality last, so its RED shows the refusal of a measured name.
- **Task 2's RED:** each of the three compilable binaries ran on its own, because the combined command stops at build_02_replay's compile error.
- **Commit type:** the workflow commit uses the type `ci`.

---

**Total deviations:** 4 (1 bug in a planned control and its measurement, 1 pre-decided measurement gap, 1 environment blocker, 1 group of test-shape details).
**Impact on plan:** none on scope. Every gate, comparison, limit and the verdict rule are unchanged or stricter, and the list was widened only under the pre-decided handling.

## Known Stubs

None.

## Threat Flags

None. The new CI step is the T-02-78 surface of the plan's threat model. It runs offline, read-only, with no capabilities and as user 65534.

## Issues Encountered

- `cargo doc -p mt-capture` shows two pre-existing warnings about links to private items (`REPORTED_TOOLS` in capture.rs and `strip_exclusions` in compare.rs). They are not from this plan and are logged in deferred-items.md.
- The container run of round trip 2 (39455563052) was not awaited. The plan awaits container only in round trip 1, until its spec-files step has run.

## User Setup Required

None.

## Next Phase Readiness

Plan 02-22 (WR-12, WR-14 and the final full-bed run) can start from `79fe192` and later. The gate's file:line table above is what its SUMMARY collects for T-02-67 and T-02-70. Then /gsd-secure-phase 2 closes those rows.

## Self-Check: PASSED

- Files: build_01_gate_bypasses.rs, build_02_replay_bypasses.rs, 02-21-CI-NOTICES.md, cc.rs, container.yml all present.
- Commits: 68e0bc7, 8da9874, 148715f, 6725d44, b82f315, 4818d97, 1270139, 2940852, 78a12ea, 44a3dd6, 79fe192 are all ancestors of HEAD.
- Measured: `git rev-list --count 00e3415..79fe192` = 11.
