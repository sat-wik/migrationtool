---
phase: 01-pinned-toolchain-container
plan: 03
subsystem: infra
tags: [rust, pins-toml, serde, toml, regex, dockerfile-build-args, llvm-pin, version-parsers, fixtures]

requires:
  - phase: 01-pinned-toolchain-container
    provides: "plan 01-01: mt-toolchain runner, tracer pins subset, check, mt toolchain check"
provides:
  - "Full pins.toml schema v1 (image, llvm, rust.tool/bitcode, source.* git/crate/archive/transitive, apt, ci, tool.* with not_installed and LLVM rules), every struct deny_unknown_fields"
  - "Pins::validate (every issue collected, sorted, field-named) and Pins::validate_complete (every container tool, hayroll/kani not_installed, LLVM rules, non-empty apt)"
  - "build_args::{Scope, render, render_lines} and `mt toolchain build-args --pins <PATH> [--scope image|ci]`"
  - "version.rs: compile_pattern/extract, LlvmVersion, parse_rustc_vv, llvm_major_from_version, DpkgPackage/parse_dpkg_query, llvm_family_majors, upstream_version"
  - "Fixtures: pins-full.toml (complete valid pins) and tool-outputs.toml (provenance-tagged tool outputs; rustc_tool and rustc_bitcode really captured)"
affects: [01-04, 01-05, 01-06, 01-07, phase-11-evidence-bundles]

actuals:
  tokens: 23600
  tasks: 2
  commits: 4
plan_head_before: 53c7da24be698be8877c3bececb55af597d60ad0
plan_head_after: a7ba4f46b524a566cbf484d24088765b24924939

tech-stack:
  added: []
  patterns:
    - "Validation walks the serialized pins (serde_json::Value) for the placeholder rule, so a field added later is covered automatically; regex fields are exempt by key name"
    - "Placeholder detection is whole-word (todo, tbd, fixme, placeholder, discover) plus the angle-quote characters, so words like 'mastodon' are not flagged"
    - "Build args carry no defaults: sorted NAME=value from pins only, with empty, whitespace/control-character values and name collisions rejected"
    - "Every regex from pins or tool output compiles through version::compile_pattern (1 MiB size limit, required named group)"

key-files:
  created:
    - crates/mt-toolchain/src/build_args.rs
    - crates/mt-toolchain/src/version.rs
    - crates/mt-toolchain/tests/pins_and_versions.rs
    - crates/mt-toolchain/tests/fixtures/pins-full.toml
    - crates/mt-toolchain/tests/fixtures/tool-outputs.toml
  modified:
    - crates/mt-toolchain/src/pins.rs
    - crates/mt-toolchain/src/check.rs
    - crates/mt-toolchain/src/lib.rs
    - crates/mt-cli/src/main.rs
    - crates/mt-cli/tests/tool_01_check_end_to_end.rs

key-decisions:
  - "build-args runs Pins::validate, not validate_complete: plan 01-06 renders build args while [apt] is still being filled by pin discovery, so completeness is enforced by the check, not the renderer"
  - "ToolSpec keeps plain String/Vec fields (empty when absent) rather than Options so plan 01-05 can read spec.bin and friends directly; not_installed tools must leave them empty, which validate enforces"
  - "parse_rustc_vv returns Option: release and commit-hash are required, a missing LLVM line gives llvm None, an unparseable LLVM line rejects the whole text (never guess)"
  - "The c2rust LLVM probe regex is `lib(?:clang-cpp\\.so\\.|LLVM-)(?P<major>\\d+)` because ldd prints libclang-cpp.so.16, which the research sketch's `lib(?:clang-cpp|LLVM)[-.]` form cannot match"
  - "tool_03 build-arg set: sources render as <SOURCE_KEY>_<FIELD> (kind omitted); rustup-init is a [source.rustup] archive, so RUSTUP_VERSION/URL/SHA256 sit beside RUSTUP_HOME from [rust]"

patterns-established:
  - "Integration tests mutate the pins-full.toml text (assert the target substring exists) rather than building pins by hand, so each negative case differs from a known-valid file by one edit"
  - "Fixture tables carry a `source` key stating captured vs representative, and a test fails if any table lacks it"

requirements-completed: [TOOL-02, TOOL-03]

coverage:
  - id: D1
    description: "Pins schema v1 parses the full fixture and rejects unknown keys in every table; validate rejects malformed sha256/commit/digest, BuildKit image without digest, non-https URLs, non-amd64 platform, bad snapshot timestamp, and empty or placeholder values"
    requirement: TOOL-03
    verification:
      - kind: unit
        ref: "crates/mt-toolchain/tests/pins_and_versions.rs#tool_03_pins_digests_and_commits_are_well_formed"
        status: pass
      - kind: unit
        ref: "crates/mt-toolchain/tests/pins_and_versions.rs#tool_03_pins_reject_placeholders_and_unknown_fields"
        status: pass
    human_judgment: false
  - id: D2
    description: "LLVM major in 16..=19, max_major 19, provisional/final status, and llvm-N/clang-N/clang-cppN token consistency across tool_path, tool bins, probe argv and apt keys"
    requirement: TOOL-02
    verification:
      - kind: unit
        ref: "crates/mt-toolchain/tests/pins_and_versions.rs#tool_02_pins_llvm_major_in_supported_range_and_provisional"
        status: pass
      - kind: unit
        ref: "crates/mt-toolchain/tests/pins_and_versions.rs#tool_02_pins_llvm_major_tokens_are_consistent"
        status: pass
    human_judgment: false
  - id: D3
    description: "validate_complete requires every container tool pinned, hayroll and kani as not_installed, LLVM rules on clang/klee/c2rust/llvm_config, and a non-empty [apt]"
    requirement: TOOL-03
    verification:
      - kind: unit
        ref: "crates/mt-toolchain/tests/pins_and_versions.rs#tool_03_pins_completeness_requires_every_container_tool"
        status: pass
    human_judgment: false
  - id: D4
    description: "`mt toolchain build-args` renders sorted NAME=value lines per scope from pins only and rejects unsafe values with exit 2"
    requirement: TOOL-03
    verification:
      - kind: unit
        ref: "crates/mt-toolchain/tests/pins_and_versions.rs#tool_03_build_args_render_sorted_lines_per_scope"
        status: pass
      - kind: unit
        ref: "crates/mt-toolchain/tests/pins_and_versions.rs#tool_03_build_args_reject_values_with_whitespace"
        status: pass
      - kind: integration
        ref: "crates/mt-cli/tests/tool_01_check_end_to_end.rs#tool_03_build_args_end_to_end_prints_sorted_lines_and_rejects_bad_pins"
        status: pass
    human_judgment: false
  - id: D5
    description: "Version, rustc -vV, LLVM-major and dpkg parsers extract values from the clang, llvm-config, klee, c2rust ldd, bear, arm-none-eabi-gcc, qemu, cargo-mutants, rustup and both rustc outputs"
    requirement: TOOL-02
    verification:
      - kind: unit
        ref: "crates/mt-toolchain/tests/pins_and_versions.rs#tool_02_parses_llvm_major_from_tool_outputs"
        status: pass
      - kind: unit
        ref: "crates/mt-toolchain/tests/pins_and_versions.rs#tool_02_parses_rustc_vv_release_commit_and_llvm"
        status: pass
      - kind: unit
        ref: "crates/mt-toolchain/tests/pins_and_versions.rs#tool_02_parses_dpkg_query_and_llvm_family_majors"
        status: pass
      - kind: unit
        ref: "crates/mt-toolchain/tests/pins_and_versions.rs#tool_02_fixture_pins_regexes_parse_fixture_outputs"
        status: pass
    human_judgment: false
  - id: D6
    description: "Most fixture outputs (every tool except the two rustc toolchains) are representative strings from the research table, not real captures, so the parsers are proven against assumed formats only until plan 01-07 swaps in CI captures"
    requirement: TOOL-02
    verification: []
    human_judgment: true
    rationale: "Representative output formats (RESEARCH A5) cannot be confirmed without running the real tools in the container; the first CI capture decides whether any regex needs adjusting"

duration: 11min
completed: 2026-10-09
status: complete
---

# Phase 1 Plan 03: Full pins schema, version parsers and build-args Summary

**pins.toml schema v1 with strict whole-file validation (hex/digest/https/placeholder/LLVM-major rules), regex-based version, `rustc -vV`, LLVM and dpkg parsers, and `mt toolchain build-args` rendering pins into sorted Docker build arguments, all provable in plain `cargo test`**

## Performance

- **Duration:** about 11 min
- **Started:** 2026-10-09T16:44:00Z (approximate; the start timestamp was taken after the context reads)
- **Completed:** 2026-10-09T16:55:00Z
- **Tasks:** 2 (both `tdd="true"`, four code commits)
- **Files modified:** 5 created, 5 modified

## Accomplishments

- **Schema v1.** `pins.rs` now models `[image]`, `[llvm]`, `[rust]` with `[rust.tool]` and `[rust.bitcode]`, `[source.*]` as a `kind`-tagged enum (git, crate, archive, transitive), `[apt]`, `[ci]` and `[tool.*]` including `not_installed` entries and optional per-tool `env` and LLVM rules (`version`, `output`, `probe`). Every struct rejects unknown keys, including the tagged source enum and the nested LLVM rule.
- **Validation.** `Pins::validate` gathers every problem at once, sorted, each prefixed with the field path (`source.rustup.sha256`, `image.base_digest`, `ci.buildkit_image`...). The placeholder rule walks the serialized pins, so it covers every string field, including ones added later; regex fields are exempt. `Pins::validate_complete` adds the container completeness rules. The LLVM rules (range 16..=19, `max_major` 19, provisional/final, `llvm-N`/`clang-N`/`clang-cppN` token consistency) make a half-migrated LLVM bump fail in `cargo test`.
- **Build args.** `mt toolchain build-args --pins <file> [--scope image|ci]` prints sorted `NAME=value` lines derived only from pins (38 image args, 2 CI args for the fixture). Empty values, whitespace, newlines, NUL, any control character and name collisions are refused (exit 2), so a pin cannot inject an extra Docker build argument.
- **Parsers.** `version.rs` provides size-limited pattern compilation, `rustc -vV` parsing (tool 1.99.0 reports LLVM 23.1.1, bitcode 1.72.1 reports 16.0.5, both really captured this session), LLVM-major extraction, `dpkg-query` rows, installed llvm/clang-family package majors, and Debian upstream-version extraction. `check.rs` now compiles regexes through `version::compile_pattern`.
- **Fixtures with provenance.** `tool-outputs.toml` marks each table "captured" or "representative"; a test fails if a table lacks `source` and asserts the bitcode capture says `LLVM version: 16.0.5`.

## Task Commits

1. **Task 1 RED:** `4acc148` (test) - schema types, placeholder validate/render, fixture, seven failing tests
2. **Task 1 GREEN:** `e32bbef` (feat) - validation, completeness, build_args rendering, CLI subcommand, extra end-to-end tests
3. **Task 2 RED:** `f5e737d` (test) - fixture outputs, placeholder parsers, five failing tests
4. **Task 2 GREEN:** `a7ba4f4` (feat) - parsers, check.rs and pins.rs moved onto `version::compile_pattern`

**Plan metadata:** committed with this SUMMARY (docs: complete plan)

## TDD Gate Compliance

Both tasks followed RED then GREEN with the `test(01-03)` commit before the `feat(01-03)` commit. No REFACTOR commits (the code needed no cleanup after GREEN).

**RED evidence.** The cargo test (libtest) console format is not one of the formats `gsd_run check tdd-red-evidence` accepts (TAP, JUnit, swift-testing, unittest), so the classifier was not run; the assessment below is manual and based on the real failing run.

| Task | RED run | Semantic assessment |
|------|---------|---------------------|
| 1 | `cargo test -p mt-toolchain --test pins_and_versions`: 7 tests ran, 0 passed, 7 failed | The fixture and every test compiled and executed (target tests discovered). Each failed on its planned assertion: `validate` accepted malformed/placeholder/inconsistent pins ("expected validation to fail naming source.rustup.sha256", `expect_err` on files that must be rejected) and the stub `render` returned an empty map (key-set and error assertions). Not a syntax, import or fixture fault. |
| 2 | same command: 12 ran, 7 passed (Task 1), 5 failed | Failures came from stub parsers: `compile_pattern` returned `Err(MissingGroup)`, `parse_dpkg_query` returned zero rows (`left: 0, right: 17`). The fixtures parsed (a fixture-load fault would have failed the Task 1 tests too). |

The RED commits include compile-time scaffolding (schema types and stub functions) because Rust tests cannot compile against missing items; the stubs are placeholders that always return the empty/permissive answer, so the failures are behavioral.

## Files Created/Modified

- `crates/mt-toolchain/src/pins.rs` - schema v1 types, `validate`, `validate_complete`, `REQUIRED_TOOLS`/`NOT_INSTALLED_TOOLS`/`LLVM_RULE_TOOLS`, `PinsError::Invalid`
- `crates/mt-toolchain/src/build_args.rs` - `Scope`, `render`, `render_lines`, `BuildArgsError`
- `crates/mt-toolchain/src/version.rs` - the parsers listed above and `VersionError`
- `crates/mt-toolchain/src/check.rs` - `ToolStatus::NotInstalled` (never fails a report), per-tool `env`, regexes via `version::`
- `crates/mt-toolchain/src/lib.rs` - declares `build_args` and `version`
- `crates/mt-cli/src/main.rs` - `check` validates first and prints each issue; new `build-args` subcommand
- `crates/mt-cli/tests/tool_01_check_end_to_end.rs` - pins helper carries every v1 section; two added end-to-end tests
- `crates/mt-toolchain/tests/pins_and_versions.rs` - 12 tests (7 Task 1, 4 Task 2, 1 extra)
- `crates/mt-toolchain/tests/fixtures/pins-full.toml`, `tool-outputs.toml` - test-only fixtures

## Decisions Made

See `key-decisions` above. The two worth repeating for later plans: `build-args` validates but does not demand completeness (01-06 needs it while `[apt]` is empty), and `ToolSpec` fields stay plain `String`/`Vec` so 01-05 can use them directly.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] build_args also rejects empty values and duplicate names**
- **Found during:** Task 1 (implementing `Args::put`)
- **Issue:** The plan lists whitespace, newline and NUL only. An empty value would render `NAME=`, which the Dockerfile's `${NAME:?}` guard treats as unset and fails late; two pins mapping to one name (for example a source key `rust_tool`) would silently overwrite each other.
- **Fix:** `BuildArgsError::BadValue` for empty values and for any Unicode control character; `BuildArgsError::DuplicateKey` for collisions.
- **Files modified:** `crates/mt-toolchain/src/build_args.rs`
- **Verification:** `tool_03_build_args_reject_values_with_whitespace` (tab, newline, NUL, trailing space) passes; no test needs the duplicate path yet.
- **Committed in:** `e32bbef`

**2. [Rule 2 - Missing Critical] Extra validation rules beyond the plan text**
- **Found during:** Task 1
- **Issue:** Values that become build args or shell inputs needed checks the plan did not list: `[source.*]` keys must be lower-case snake case (they become `NAME` prefixes); `transitive.parent` must name another `[source]` entry; `tool_path` entries and `dpkg_query`, `rustup_home`, `cargo_home` must be absolute; probe argv[0] must be absolute; version triples (`rust.*.version`, `rust.bitcode.llvm`, `ci.cargo_deny_version`) must be `X.Y.Z`.
- **Fix:** added to `Pins::validate`.
- **Files modified:** `crates/mt-toolchain/src/pins.rs`
- **Verification:** the full fixture passes; the mutation tests still isolate their single edits.
- **Committed in:** `e32bbef`

**3. [Rule 2 - Missing Critical] Two extra end-to-end CLI tests and one extra parser test**
- **Found during:** Tasks 1 and 2
- **Issue:** The plan's `<behavior>` covers the library but not the CLI contract for `build-args` and placeholder pins, nor regex-limit/edge cases (threat T-01-10 is "mitigate").
- **Fix:** added `tool_03_check_end_to_end_rejects_placeholder_pins_with_exit_two`, `tool_03_build_args_end_to_end_prints_sorted_lines_and_rejects_bad_pins` and `tool_02_compile_pattern_and_extract_edge_cases` (oversized regex refused). The four original end-to-end assertions are unchanged.
- **Files modified:** `crates/mt-cli/tests/tool_01_check_end_to_end.rs`, `crates/mt-toolchain/tests/pins_and_versions.rs`
- **Committed in:** `e32bbef`, `f5e737d`

---

**Total deviations:** 3 auto-fixed (all Rule 2). **Impact on plan:** no scope change; all files are inside `files_modified`, no new crates or `Cargo.lock` changes.

## Issues Encountered

- `rustup toolchain install 1.72.1 --profile minimal` worked through the sandbox proxy (a rustup self-update notice followed); that toolchain exists only in this session's rustup home, not in the repository.
- Sandbox git wrappers refuse compound or heredoc shell commands, so test additions were made with the editor tools and git commands were issued one at a time.

## Authentication Gates

None.

## Known Stubs

None. The placeholder bodies that existed in the RED commits (`validate`, `render`, the version parsers) were all replaced in the matching GREEN commits.

## Threat Flags

None. `build-args` is a new read-only CLI path over the same pins-file trust boundary already modelled (T-01-08 to T-01-10 are mitigated by the tests above; T-01-SC: no new crates).

## User Setup Required

None - no external service configuration required.

## Verification Results

Run on `a7ba4f4` before this commit:

- `cargo fmt --check`: pass
- `cargo clippy --all-targets -- -D warnings`: pass
- `cargo test --workspace --locked`: pass (6 end-to-end, 12 pins/version, 9 runner tests)
- `cargo run -q -p mt-cli -- toolchain build-args --pins crates/mt-toolchain/tests/fixtures/pins-full.toml`: exit 0, first line `ARM_GNU_SHA256=8f6903f8...`
- same with `--scope ci`: exactly `BUILDKIT_IMAGE=moby/buildkit:v0.34.0@sha256:...` and `CARGO_DENY_VERSION=0.20.2`

## Next Phase Readiness

- **01-05** can build `evaluate` on `ToolSpec.{status,reason,bin,version_args,version_regex,expect,env,llvm}`, `Pins::validate_complete`, `version::{compile_pattern, extract, parse_rustc_vv, parse_dpkg_query, llvm_family_majors}` and the rustup/default/targets fixtures it still has to add. `ToolRow` has no `llvm_major` or `detail` yet (01-05 adds them). `run_check` currently calls only the tracer path; per the plan it will call `validate_complete`.
- **01-06** supplies `container/pins.toml`; `mt toolchain build-args` already renders it with an empty `[apt]`.
- **01-07** should replace the representative `tool-outputs.toml` tables (everything except `rustc_tool` and `rustc_bitcode`) with CI captures.
- Open: the LLVM-bump decision gate against PROJECT.md (`tool_02_llvm_and_bitcode_pins_have_matching_project_decision`) is not part of this plan; it belongs to a later plan in the phase.

## Self-Check: PASSED

All 6 created source/test/fixture files exist; commits `4acc148`, `e32bbef`, `f5e737d` and `a7ba4f4` are ancestors of HEAD; `pins.rs` contains `deny_unknown_fields`, `pub fn validate(` and `pub fn validate_complete(`; `tool-outputs.toml` has a `[rustc_bitcode]` table whose stdout contains `LLVM version: 16.0.5` and whose source says "captured". Task acceptance criteria re-run: 12/12 tests in `pins_and_versions`, 6/6 end-to-end tests, CLI commands as above, `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` clean.

---
*Phase: 01-pinned-toolchain-container*
*Completed: 2026-10-09*
