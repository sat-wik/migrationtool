---
phase: 02-test-bed-and-build-capture
plan: 06
subsystem: testing
tags: [testbed, github-actions, docker, bear, cmake, replay, determinism, annotations, fixtures]

requires:
  - phase: 02-test-bed-and-build-capture
    provides: "plan 02-02 main-published image digest; plans 02-03 to 02-05 manifest, fetch, capture, replay, entries; plan 02-07 (no code dependency)"
provides:
  - "mt_capture::testbed: Selection, TestbedOptions, StepSummary, TestbedError, run_validate, run_fetch, run_capture, run_replay"
  - "mt testbed validate|fetch|capture|replay (Group::Testbed in mt-cli), exit 0, 1, 2"
  - ".github/workflows/testbed.yml: jobs mt and testbed, helpers run_networked, run_offline, image_tool, steps with annotations testbed-* and fixture ..."
  - "testbed/manifest.toml [image] digest (ImageRef) tied to container/README.md by a test"
  - "Green CI evidence: qcbor (Make plus Bear) and corejson (CMake), default and variant, fetched, captured, gated, entry-checked and replayed identical in the real image"
affects: [02-08, 02-09, 02-10, testbed seeds, full-bed run]

requirements-completed: [BED-01, BED-02, BUILD-01, BUILD-02]

actuals:
  tokens: 21000
  tasks: 2
  commits: 4
plan_head_before: 66e972de3ea9c8d516c629ccb98cb3507f187336
plan_head_after: 4195a6dd54ea17e9c4c0bba1ee4017e52725ddcb

tech-stack:
  added: []
  patterns:
    - "A pair that fails is a row of the step summary (a verdict word), never an error; Err is for unusable inputs and tools that cannot start (exit 2)"
    - "Every mt testbed step except fetch goes through run_offline (--network none); a static test fails if any other step uses run_networked"
    - "Results are published as annotations: the summary table plus evidence lines as notice testbed-<step>, or an error with stderr and the failing reports"
    - "A multi-part notice is published by several steps when it has more than 9 parts (GitHub keeps 10 annotations per level per step)"

key-files:
  created:
    - crates/mt-capture/src/testbed.rs
    - .github/workflows/testbed.yml
    - testbed/README.md
    - crates/mt-capture/tests/build_02_workflow_static.rs
    - crates/mt-cli/tests/bed_01_testbed_cli.rs
  modified:
    - crates/mt-capture/src/lib.rs
    - crates/mt-capture/src/capture.rs
    - crates/mt-capture/src/manifest.rs
    - crates/mt-capture/tests/bed_01_manifest.rs
    - crates/mt-capture/tests/build_01_capture_drivers.rs
    - crates/mt-capture/tests/fixtures/manifest-small.toml
    - crates/mt-cli/Cargo.toml
    - crates/mt-cli/src/main.rs
    - testbed/manifest.toml
    - Cargo.lock

key-decisions:
  - "The determinism comparison excludes two fields, both documented in the workflow and the notice: the stdout digest and byte counts of the step `cmake configure` (CMake prints its elapsed time) and of the steps `clang ast <fn>` (clang's JSON dump holds node addresses). Everything else, and every compile_commands file, is compared byte for byte"
  - "The determinism step runs after replay, so replay verifies the first capture; the second capture then overwrites the work directory"
  - "CMake's compiler-ABI probe output (CMakeFiles/<version>/CMakeDetermineCompilerABI_*.bin) is not a configure-time input and is left out of configure_files"
  - "Capture failures now carry the tail of the failing step's standard error in the cause text, because run records hold only digests and CI logs are unreachable"
  - "run_validate takes the manifest path only; --pins is not read by validate"

patterns-established:
  - "annotate LEVEL TITLE FILE [FROM [TO]] with a part range, for evidence longer than 9 parts"
  - "evidence and failing_details jq helpers print the gate, entry, variant and object results of the reports into the step notice"

coverage:
  - id: D1
    description: "mt testbed validate|fetch|capture|replay take --manifest, --pins, --work-dir, --out, --image and --all or ids, exit 0 when every selected pair passes, 1 when one failed or was not proven, 2 on invalid input; mt-cli only parses arguments"
    requirement: BED-01
    verification:
      - kind: integration
        ref: "crates/mt-cli/tests/bed_01_testbed_cli.rs (bed_01_testbed_validate_accepts_the_committed_manifest, bed_01_testbed_validate_rejects_an_invalid_manifest_with_exit_two, build_01_testbed_capture_of_a_target_without_build_args_exits_two)"
        status: pass
    human_judgment: false
  - id: D2
    description: "In the real image, qcbor (Make plus Bear) and corejson (CMake) are fetched by commit with tree hashes equal to the pins, captured under --network none with every gate and entry check passing, and replayed identical for all four target-configuration pairs"
    requirement: BUILD-02
    verification:
      - kind: integration
        ref: "testbed run 38380382970, job 115239376216 (annotations testbed-fetch, testbed-capture, testbed-replay)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Only fetch is networked; privileges, mounts, permissions, checkout credentials, timeouts, triggers, image digest and documentation wording are pinned by static tests"
    requirement: BED-01
    verification:
      - kind: unit
        ref: "crates/mt-capture/tests/build_02_workflow_static.rs (build_02_workflow_runs_capture_and_replay_without_network, build_02_workflow_is_least_privilege_and_scheduled, build_02_image_digest_matches_readme, build_02_testbed_docs_make_no_certification_claim)"
        status: pass
    human_judgment: false
  - id: D4
    description: "A second capture in the same run gives identical reports and compile commands apart from the two documented digest fields"
    requirement: BUILD-02
    verification:
      - kind: integration
        ref: "testbed run 38380382970, job 115239376216 (annotation testbed-determinism: compared 10 files, result identical)"
        status: pass
    human_judgment: false
  - id: D5
    description: "The D-26 fixture evidence is published as notices (compile_commands of qcbor/default and corejson/default, the corejson/default capture and replay reports, readelf -A, two Arm objects differing only in .comment, the QCBORDecode_Init AST dump), complete with all their parts"
    requirement: BED-01
    verification:
      - kind: integration
        ref: "testbed run 38380382970, job 115239376216 (fixture ... annotations; capture 5/5, entry ast 18/18)"
        status: pass
    human_judgment: false
  - id: D6
    description: "Whether the fixture notices are what plan 02-10 needs, and whether the two documented determinism exclusions are acceptable evidence of reproducibility"
    verification: []
    human_judgment: true
    rationale: "Plan 02-10 commits and reads the fixtures; the founder decides whether excluding the stdout digest of two step kinds weakens the determinism claim"

duration: 25min
completed: 2026-10-13
status: complete
---

# Phase 2 Plan 06: The tracer, `mt testbed` and the testbed workflow Summary

**`mt testbed validate|fetch|capture|replay` wired into mt-cli and the `testbed` workflow that, in the real extended image, fetches qcbor (Make plus Bear) and corejson (CMake) by commit, captures them under `--network none`, and replays all four target-configuration pairs byte-identical, with the network policy, image digest and report determinism pinned by tests and CI and the D-26 fixture evidence published as annotations**

## Performance

- **Duration:** about 25 min wall clock (start estimated from the base commit, 09:35Z; the start time was not captured in a persisted variable)
- **Started:** 2026-10-13T09:35Z (estimate)
- **Completed:** 2026-10-13T10:05Z
- **Tasks:** 2 (the tracer, type `tracer`; Task 2, type `auto`)
- **Files modified:** 15 (5 created, 10 modified, excluding this summary)

## Accomplishments

- `mt_capture::testbed` runs the three container steps over the selected targets and returns a `StepSummary` per step (rows are verdict words, `passed` only when every row passes), writing `fetch/<id>.json`, `capture/<id>/<config>.json` and `.compile_commands.json`, `capture/<id>/variant.json`, `replay/<id>/<config>.json` and `summary-<step>.json`. `mt-cli` only parses arguments and maps the summary to exit 0 or 1; invalid inputs, an unknown id or a target with no recipe are exit 2.
- The `testbed` workflow went green on its first run in the real image for the tracer (qcbor and corejson, default and variant): both source trees fetched with the pinned tree hashes, every gate passed, all entry functions found with matching signatures and headers, both variants change code, and all four replays are `identical`.
- Static tests now enforce that no step but fetch can be networked, that containers run unprivileged and read-only with `:ro` mounts for mt, pins and testbed, that permissions are `contents: read`, and that the manifest `[image] digest` equals the README's single Current image digest.
- A second capture in the same run found real nondeterminism (CMake's compiler-ABI probe output in `configure_files`); fixed and confirmed identical in CI.

## CI evidence

### Round trips used (cap 3 each)

| Task | Round trips | Detail |
|------|-------------|--------|
| Tracer (Task 1) | **1** | RT1 green on the first run |
| Task 2 | **3** | RT1 failed at the determinism step (real nondeterminism, fixed); RT2 green but the 18-part AST fixture was cut to 10 parts (fixed); RT3 green and complete |

No budget extension, no escalation, no tracer target switch (no pre-decided fallback was needed).

### Runs

| Round trip | Head SHA | testbed | ci | container (not awaited) |
|---|---|---|---|---|
| Tracer RT1 | 7cdba42d58a931b3e5320b67fa90ef81632aedb9 | 38378619035 success (job mt 115233163534, testbed 115233521750) | 38378618767 success (checks 115233161706) | 38378618760 success |
| Task 2 RT1 | 8d047393b2fa1016f79f51175822ec4c63e9a6a1 | 38379206515 failure (job testbed 115235466014, only step `Capture again and compare the reports (determinism)`) | 38379206757 success | 38379206939 cancelled (superseded by the next push) |
| Task 2 RT2 | de927856a1ef9f5b65c89894a8c0478ce15adf46 | 38379662768 success (job testbed 115237011794) | 38379662721 success (checks 115236640544) | 38379662785 not finished when this was written |
| **Task 2 RT3 (final, green)** | **4195a6dd54ea17e9c4c0bba1ee4017e52725ddcb** | **38380382970 success (job mt 115239029386, testbed 115239376216)** | **38380382925 success (checks 115239029341)** | not awaited; not finished |

The plan's verify command (`success <sha of the last commit touching the trigger paths>`) exited 0 after the tracer round trip (7cdba42) and after RT3 (4195a6d).

### The four summary tables (final green run 38380382970, job 115239376216)

`testbed-validate`:
```
manifest  valid (16 targets, 31 configurations)
result: passed (1 rows)
```

`testbed-fetch` (both `fetched`, so neither was reused):
```
corejson  fetched
qcbor     fetched
result: passed (2 rows)
corejson git commit cffa492da18c890181d64462f8af63992a69d3b0 tree_sha256 9ca1058329f6ce0c31f9c13f861ec20cf75f35230b9aae0122ab81480a6df9a5 reused=false
qcbor git commit 930708bb86481e88879eb1d87fd4d664f1d69503 tree_sha256 ed42fa8d2ea1619e71613e2925c197ae55a6c0dee151d61983334ef6968f8ee0 reused=false
```
Both tree hashes equal the RESEARCH appendix values and `testbed/manifest.toml` (a mismatch would have failed the fetch). The container's own git therefore confirms the manifest pins for these two targets.

`testbed-capture`:
```
corejson/default   captured
corejson/depth-8   captured
corejson/variant   changes_code true
qcbor/default      captured
qcbor/no-hw-float  captured
qcbor/variant      changes_code true
result: passed (6 rows)
corejson/default compiler_allow_listed=true arch_ok=true undeclared=0 timestamp_macro=0 objects=1 opt_goals=Aggressive Size entries=JSON_SearchT=found:true,signature:true,header:true;JSON_Validate=found:true,signature:true,header:true
corejson/depth-8 (same values, objects=1)
qcbor/default compiler_allow_listed=true arch_ok=true undeclared=0 timestamp_macro=0 objects=5 opt_goals=Aggressive Size entries=QCBORDecode_Init=found:true,signature:true,header:true
qcbor/no-hw-float (same values, objects=5)
corejson variant depth-8: changes_code=true code_differs=build/CMakeFiles/core_json.dir/source/core_json.c.obj
qcbor variant no-hw-float: changes_code=true code_differs=src/src/ieee754.o,src/src/qcbor_decode.o,src/src/qcbor_encode.o
```
Assumption A4 (clang 16 prints the same signature strings as clang 18) held: all three signatures matched with no manifest edit.

`testbed-replay`:
```
corejson/default   identical
corejson/depth-8   identical
qcbor/default      identical
qcbor/no-hw-float  identical
result: passed (4 rows)
corejson/default verdict=identical objects=1 normalisation_applied=false
corejson/depth-8 verdict=identical objects=1 normalisation_applied=false
qcbor/default verdict=identical objects=5 normalisation_applied=false
qcbor/no-hw-float verdict=identical objects=5 normalisation_applied=false
```
No object needed the `.comment` normalisation (it stayed unused, as RESEARCH measured), so `identical` is byte equality. Also published: `testbed-image` (the image reference, id `sha256:1267a6230e171d711a7d056cd7a129fbdf0753339c92af22dac69afc806eac44`, `linux/amd64`), and `testbed-determinism`:
```
rerun capture exit code: 0
compared 10 files of capture/ against the rerun
excluded: stdout sha256 and byte counts of the steps "cmake configure" and "clang ast <function>"
result: identical
```

### Notice titles for plan 02-10 (all in job 115239376216)

- `fixture qcbor-default compile_commands`
- `fixture corejson-default compile_commands`
- `fixture corejson-default capture (1/5)` to `(5/5)`
- `fixture corejson-default replay`
- `fixture readelf-A` (readelf -A of `build/CMakeFiles/core_json.dir/source/core_json.c.obj`: v7E-M, Thumb-2, VFPv4-D16, VFP registers, Aggressive Size)
- `fixture arm-small.o` (792 bytes: `/capture/qcbor/no-hw-float/objects/src/src/ieee754.o`; lines `source:`, `sha256:`, `bytes:`, `base64:` and the base64 wrapped at 76 columns)
- `fixture arm-small-comment.o` (740 bytes: the same object with `.comment` replaced by `arm-none-eabi-objcopy --update-section .comment=<19 bytes>`; the step also checked that `objcopy --remove-section=.comment` of both gives identical files)
- `fixture qcbor entry ast (1/18)` to `(18/18)` (the AST dump of the QCBORDecode_Init probe, run in the image with the argv and cwd recorded in the capture report)

Reading multi-part notices: each part is a run of whole lines, and the final newline of each part is dropped by the annotation helper. Join the parts of one title in order with a newline; the whole file also loses its final newline. The job's annotations list newest first.

## Task Commits

1. **Tracer: mt testbed fetch, capture and replay with the testbed workflow** - `7cdba42` (feat)
2. **Task 2: workflow safety tests, image digest, determinism and fixture steps** - `8d04739` (test)
3. **Task 2: leave CMake's compiler-ABI probe output out of the configure-time hashes** - `de92785` (fix)
4. **Task 2: publish the entry-check AST fixture in three steps** - `4195a6d` (ci)

**Plan metadata:** the commit that adds this SUMMARY (docs).

## Files Created/Modified

- `crates/mt-capture/src/testbed.rs` - the pipeline, `StepSummary` and `TestbedError`
- `crates/mt-cli/src/main.rs`, `crates/mt-cli/Cargo.toml` - `Group::Testbed`, `TestbedAction`, `match cli.group`
- `.github/workflows/testbed.yml` - jobs `mt` and `testbed`; helpers `run_networked`, `run_offline`, `image_tool`, `evidence`, `failing_details`, `finish_step`; env `TESTBED_SELECTION: "qcbor corejson"`
- `testbed/README.md` - how to run the three container steps
- `crates/mt-capture/src/manifest.rs`, `testbed/manifest.toml`, `crates/mt-capture/tests/fixtures/manifest-small.toml` - required `[image] digest`
- `crates/mt-capture/src/capture.rs` - stderr tail in build failure causes; `is_cmake_product`
- `crates/mt-capture/tests/build_02_workflow_static.rs`, `crates/mt-cli/tests/bed_01_testbed_cli.rs`, additions to `bed_01_manifest.rs` and `build_01_capture_drivers.rs` - the tests
- `crates/mt-capture/src/lib.rs`, `Cargo.lock` - module and the mt-cli dependency edge

## Decisions Made

See key-decisions. For plan 02-09: set `TESTBED_SELECTION` to `--all`; the capture and replay summaries are the `testbed-capture` and `testbed-replay` notices, and a variant with `changes_code false` shows as a row `<id>/variant` that does not fail the step (plan 02-09 drops it as `variant_no_code_change`). A `capture` or `replay` run on a target with no `build_args` is exit 2 before anything runs, so `--all` needs every recipe written first.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical] Capture failure causes carry the failing step's standard error**
- **Found during:** Tracer (before the first push)
- **Issue:** run records hold only stream digests, and Actions logs are unreachable from this session, so a failing CMake or Make step would have been undiagnosable from annotations within the 3-round-trip cap.
- **Fix:** `Outcome::Failed` carries a one-line tail (last 1500 bytes) of standard error, appended to the `build_failed` cause detail.
- **Files modified:** crates/mt-capture/src/capture.rs
- **Committed in:** 7cdba42

**2. [Rule 1 - Bug] CMake's compiler-ABI probe output was hashed into `configure_files`**
- **Found during:** Task 2, round trip 1 (the determinism step)
- **Issue:** `build/CMakeFiles/3.25.1/CMakeDetermineCompilerABI_C.bin`, the output of a test compile, differed between two captures of corejson/default and corejson/depth-8, so the capture report was not reproducible (and replay's re-check of configure files would be on a moving target). It is not an input of any compile command.
- **Fix:** `capture::is_cmake_product` names it together with objects, archives, shared libraries and CMake's logs; unit test `build_01_cmake_probe_outputs_are_not_configure_files`.
- **Files modified:** crates/mt-capture/src/capture.rs, crates/mt-capture/tests/build_01_capture_drivers.rs
- **Verification:** testbed run 38379662768 and the final run 38380382970, `testbed-determinism` result identical
- **Committed in:** de92785

**3. [Rule 3 - Blocking] The 18-part AST notice was cut to 10 parts**
- **Found during:** Task 2, round trip 2 (annotation listing of the green run)
- **Issue:** GitHub keeps 10 annotations of a level per step. `fixture qcbor entry ast` was published as parts 1 to 10 of 18 (the plan warns a missing title means the step must be split; here the title was present but the content was incomplete).
- **Fix:** `annotate` takes an optional part range and three steps publish parts 1 to 9, 10 to 18 and 19 on.
- **Files modified:** .github/workflows/testbed.yml
- **Verification:** run 38380382970 lists parts (1/18) to (18/18) and (1/5) to (5/5) for the capture report
- **Committed in:** 4195a6d

### Plan deviations

**4. [Needs your attention] A second field is excluded from the determinism comparison**
- The orchestrator's note named one field (the `cmake configure` stdout digest). A local rerun showed the stdout digest of `clang ast <function>` changes between runs too (clang's JSON AST dump contains node addresses). The step cannot compare it byte for byte, and the runner records only digests. The step excludes, and states in every notice, exactly two things: the stdout `sha256`, `bytes_total` and `bytes_stored` of steps named `cmake configure` and of steps whose name starts with `clang ast `. The entry results derived from the dump (`found`, `signature_matches`, `header_included`, `found_signature`) stay in the compared part, as do all other fields and every compile_commands file byte for byte. This needed no design change. Say if you want the exclusion removed or handled differently.

**5. The determinism step runs after replay, not between capture and replay**
- The second capture rewrites the work directory, so running it first would have made replay verify the second capture. Replay verifies the first capture; the rerun is then compared against it.

**6. Commit trailers name Sonnet 5.5**
- The dispatch text asked for `Co-Authored-By: Claude Opus 5.5`. The harness attribution reminder for this session says `Claude Sonnet 5.5`, which is the model that did the work, so every commit carries that line and the Claude-Session line. No model name appears anywhere else.

**7. Acceptance wording on `--network none`**
- "testbed.yml contains `--network none` in run_offline only": the file also names it in the header comment and in the `image_tool` helper (Task 2's readelf and objcopy fixture steps run with `--network none`, as the plan states). What holds, and is tested, is that `run_networked` has no `--network` setting, fetch is the only step through it, and every other `docker run` is offline.

**8. `run_validate` and the arguments**
- `validate` takes only `--manifest` (the plan says `--pins` is not for validate and the workflow passes the manifest only), so `run_validate(manifest_path)` has a smaller signature than the other three, which take `&TestbedOptions`.

### Behaviour notes

- The capture report's `image` field holds the full reference passed by the workflow (`ghcr.io/sat-wik/migrationtool-toolchain@sha256:...`), not the bare digest.
- Before the first push, the whole pipeline was dry-run on this host with the local Arm GCC 13.2.1, Bear, CMake 3.28 and clang 18 against the real upstream commits (tree hashes matched). That is not CI evidence and none of it is quoted above.
- `requirements.mark-complete` was not run, as instructed.

---

**Total deviations:** 3 auto-fixed (1 missing critical, 1 bug, 1 blocking) and 5 plan deviations or notes. **Impact:** the second determinism exclusion (4) is the only one that touches an evidence claim; the fixes were necessary for correctness and for complete fixtures.

## Authentication Gates

None. The image pulled anonymously and `gh api` worked throughout.

## Issues Encountered

- A script edit of the workflow with an empty slice (`s.index` found the earlier `mt` job's upload step) replaced text between every character; it was caught by the YAML parse before commit, the file was restored from git and the edit redone.
- The `container` run of 8d04739 was cancelled by GitHub when the next push queued a run in the same group; it was not awaited.

## Known Stubs

None.

## Threat Flags

None beyond the plan's threat model. The workflow adds a networked fetch container (T-02-24 policy: tested), mounts only /work and /out writable (T-02-25), holds `contents: read` (T-02-26) and pulls the image by the one README digest (T-02-27, tested).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plans 02-08 and 02-09 can expand the same code: set `TESTBED_SELECTION` to `--all` in plan 02-09 and add a seeds step through `run_offline`.
- Plan 02-10 can read the fixture notices of job 115239376216 (run 38380382970) listed above.
- Not yet proven: the other 14 targets, and the `.comment` normalisation path on real Arm objects (it stayed unused).

---
*Phase: 02-test-bed-and-build-capture*
*Completed: 2026-10-13*

## Self-Check: PASSED

- Files: `crates/mt-capture/src/testbed.rs`, `.github/workflows/testbed.yml`, `testbed/README.md`, `crates/mt-capture/tests/build_02_workflow_static.rs` and `crates/mt-cli/tests/bed_01_testbed_cli.rs` exist.
- Commits: `7cdba42`, `8d04739`, `de92785` and `4195a6d` are ancestors of HEAD; `git rev-list --count 66e972d..HEAD` is 4 before this summary's commit.
- Acceptance: `crates/mt-cli/src/main.rs` contains `Group::Testbed` and `match cli.group`; `rg 'std::process|Command::new' crates/mt-capture/src crates/mt-cli/src` finds nothing; the latest testbed run on the branch is success for the last commit touching its trigger paths (4195a6d); the annotations of that run list every title above with all their parts.
- Verification before every push: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace --locked` and `cargo deny check` each exited 0.
