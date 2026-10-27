---
phase: 03-equivalence-contract-and-test-vector-schema
plan: 02
subsystem: testing
tags: [equivalence-contract, ub-policy, toml, serde, mt-contract, refinement]

requires:
  - phase: 03-equivalence-contract-and-test-vector-schema
    provides: plan 03-01's executor core, the outcome class set (OUTCOME_CLASSES), the has_ub labels, Budgets::DEFAULT and the mt vectors group that policy-check joins
  - phase: 02-test-bed-and-build-capture
    provides: testbed/manifest.toml (the 16 target ids) and the strict-TOML style of mt_capture::manifest
provides:
  - "crates/mt-contract: CONTRACT_VERSION 0.1.0, POLICY_SCHEMA_VERSION 1, Policy/Ub/Action/DefaultChange/ErrorCode/ErrorValue/Override/Site/PolicyError, Policy::parse/load/validate/override_report/summary_row, policy::load_dir"
  - "testbed/policy/<id>.toml for the 16 manifest targets, each default return_error with empty override, default-change and error-code tables"
  - "mt vectors policy-check [--manifest <PATH>] [<FILE>...]"
  - "docs/specs/equivalence-contract.md 0.1.0: refinement, observation point, field convention, library-state typing, comparison rule, UB menu and policy, outcomes, reference executor"
affects: [03-03, 03-04, 03-05, 03-06, 03-07, phase 4 UB log, phase 5 comparators, phase 11 evidence bundle]

requirements-completed: [EQV-01]

status: complete
commits: 6
plan_head_before: dcb9462da27cbf2014548885abbbbebe397db25f
plan_head_after: a8a1e8f2b3c6f51c3a326d018e21190a202c8a8c
actuals:
  tokens: 20045
  tasks: 3
  commits: 6

tech-stack:
  added: []
  patterns:
    - "strict TOML policy file: deny_unknown_fields on every struct, shape checked by parse, values by validate, every issue collected, sorted and reported at once"
    - "contract document held to code by a test: the Version line equals CONTRACT_VERSION and the 16 policies carry the same version"
    - "the UB log is a parameter: validate takes a known-site list, Phase 3 passes an empty one, so any override fails until Phase 4"

key-files:
  created:
    - crates/mt-contract/Cargo.toml
    - crates/mt-contract/src/lib.rs
    - crates/mt-contract/src/policy.rs
    - crates/mt-contract/tests/eqv_01_policy.rs
    - crates/mt-contract/tests/eqv_01_committed_policies.rs
    - crates/mt-contract/tests/eqv_01_contract_document.rs
    - crates/mt-cli/tests/eqv_01_policy_check_cli.rs
    - docs/specs/equivalence-contract.md
    - testbed/policy/cjson.toml
    - testbed/policy/corejson.toml
    - testbed/policy/coremqtt.toml
    - testbed/policy/httpparser.toml
    - testbed/policy/isotp.toml
    - testbed/policy/libfdt.toml
    - testbed/policy/lz4.toml
    - testbed/policy/mbedtls.toml
    - testbed/policy/microcoap.toml
    - testbed/policy/nanopb.toml
    - testbed/policy/nimble.toml
    - testbed/policy/paho.toml
    - testbed/policy/qcbor.toml
    - testbed/policy/tinycbor.toml
    - testbed/policy/zcbor.toml
    - testbed/policy/zlib.toml
  modified:
    - Cargo.toml
    - Cargo.lock
    - crates/mt-cli/Cargo.toml
    - crates/mt-cli/src/main.rs

key-decisions:
  - "Policy TOML shape as the plan settled it: schema_version, contract_version, module, [ub] default_action, [ub.default_change] (module, reason, approver, date), [[ub.error_code]] (function, value), [[ub.override]] (function, kind, location, action, reason, approver, date); unknown keys rejected at every level"
  - "ASSUMPTION for the founder: [[ub.error_code]] is defined and syntax-checked now (function a C identifier, value a decimal integer or a C identifier, at most one entry per function) but every committed policy leaves it empty; Phase 4's UB decisions log fills it. The value may be written as a TOML integer or as text"
  - "kind and location of an override are free non-empty text until Phase 4 constrains them; site identity is exact equality of (function, kind, location)"
  - "contract_version is MAJOR.MINOR.PATCH of decimal digits (leading zeros are not rejected) and not above CONTRACT_VERSION; module matches ^[a-z0-9][a-z0-9_-]*$ and, through Policy::load, equals the file stem"
  - "Dates are YYYY-MM-DD, year 0001 to 9999, Gregorian leap years, checked with no clock and no date crate; a blank (whitespace only) reason, approver or other text counts as empty"
  - "The check logic lives in mt-contract (summary_row, load_dir), not in mt-cli, so the CLI stays argument handling; mt-contract still depends on serde, toml and thiserror only"
  - "override_report writes values with backslash, line breaks, tabs and other control characters escaped, so every field stays on one line"
  - "The contract names the user-mode distinction (an unmodified firmware image cannot run in qemu-arm user mode; the reference driver is a hosted-style ELF reusing the same object code) and does not edit container/pins.toml"

patterns-established:
  - "A contract change bumps the version, adds a changelog line and moves CONTRACT_VERSION and the 16 policies together; eqv_01_committed_policies_cover_every_target and the document test enforce it"

coverage:
  - id: D1
    description: "Strict per-module policy loader with the full rules: schema and contract version, module shape and file stem, default_change record, error codes, overrides with seven non-empty fields, real calendar dates, unique sites that the UB log lists, and a deterministic override report"
    requirement: EQV-01
    verification:
      - kind: unit
        ref: "crates/mt-contract/tests/eqv_01_policy.rs#eqv_01_rejects_unknown_key"
        status: pass
      - kind: unit
        ref: "crates/mt-contract/tests/eqv_01_policy.rs#eqv_01_default_change_required_to_change_default"
        status: pass
      - kind: unit
        ref: "crates/mt-contract/tests/eqv_01_policy.rs#eqv_01_override_needs_every_field"
        status: pass
      - kind: unit
        ref: "crates/mt-contract/tests/eqv_01_policy.rs#eqv_01_override_naming_no_logged_site_is_rejected"
        status: pass
      - kind: unit
        ref: "crates/mt-contract/tests/eqv_01_policy.rs#eqv_01_override_report_lists_every_override"
        status: pass
    human_judgment: false
  - id: D2
    description: "16 committed policies, one per manifest target, all return_error with empty tables, and mt vectors policy-check (file form and --manifest form)"
    requirement: EQV-01
    verification:
      - kind: unit
        ref: "crates/mt-contract/tests/eqv_01_committed_policies.rs#eqv_01_committed_policies_cover_every_target"
        status: pass
      - kind: integration
        ref: "crates/mt-cli/tests/eqv_01_policy_check_cli.rs#eqv_01_policy_check_cli_checks_every_target"
        status: pass
      - kind: other
        ref: "cargo run -q -p mt-cli -- vectors policy-check --manifest testbed/manifest.toml (16 rows, exit 0)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Contract v0 (docs/specs/equivalence-contract.md 0.1.0) states refinement, the observation point, the field convention, library-state typing, the comparison rule, the UB menu, outcomes and the reference executor, with a version equal to CONTRACT_VERSION and no forbidden claim word"
    requirement: EQV-01
    verification:
      - kind: unit
        ref: "crates/mt-contract/tests/eqv_01_contract_document.rs#eqv_01_contract_document_has_version_and_changelog"
        status: pass
      - kind: unit
        ref: "crates/mt-contract/tests/eqv_01_contract_document.rs#eqv_01_contract_document_states_the_rules"
        status: pass
      - kind: unit
        ref: "crates/mt-contract/tests/eqv_01_contract_document.rs#eqv_01_contract_document_makes_no_claim"
        status: pass
    human_judgment: true
    rationale: "Tests prove the document is versioned, names every rule and avoids the forbidden words; whether its wording says what D-05 to D-30 decided is a reading task for the founder at /gsd-verify-work (03-VALIDATION.md manual-only row)"

duration: 12 min
completed: 2026-10-27
---

# Phase 3 Plan 02: Equivalence contract v0 and the per-module UB policy Summary

**"Equivalent" is now written down once: `docs/specs/equivalence-contract.md` 0.1.0 (refinement, the C-interface observation point, the TRACTOR field convention with a Latin-1 codec, library-state typing, the UB action menu), and the new `mt-contract` crate loads a strict per-module policy file that defaults to `return_error`, demands a full who/why/when record to change the default or add a per-site override, and refuses any override that names a site the UB log did not record; 16 policies are committed and `mt vectors policy-check --manifest testbed/manifest.toml` passes them.**

## Performance

- **Duration:** 12 min
- **Started:** 2026-10-27T05:02:54Z
- **Completed:** 2026-10-27T05:15Z
- **Tasks:** 3 of 3 (Tracer, Task 2, Task 3)
- **Files:** 28 files changed against dcb9462 (1922 insertions, 2 deletions), plus this summary

## Accomplishments

- **Policy loader (EQV-01):** `Policy::parse` and `Policy::load` read TOML with `deny_unknown_fields` on every struct (T-03-05); `Policy::validate(&[Site])` collects every issue, each naming its key and the decision (D-16, D-17, D-18, D-29). A changed default needs `[ub.default_change]` and the record is refused while the default is `return_error`; overrides need seven non-empty fields, a real calendar date, a unique site, and a site in the known-site list (T-03-06, T-03-07).
- **Override report:** `Policy::override_report` writes `module`, `default_action`, the default change or `default_change=none`, `overrides=<n>`, then each override with all seven fields ordered by (function, kind, location); one `key=value` line per field, escaped, deterministic across calls.
- **16 policies:** `testbed/policy/<id>.toml` for cjson, corejson, coremqtt, httpparser, isotp, libfdt, lz4, mbedtls, microcoap, nanopb, nimble, paho, qcbor, tinycbor, zcbor, zlib; all `return_error`, no override, default change or error code. `eqv_01_committed_policies_cover_every_target` ties the file set to the manifest's targets.
- **CLI:** `mt vectors policy-check [--manifest <PATH>] [<FILE>...]` prints `<module> valid default=<action> overrides=<n> error_codes=<n>` per policy; with `--manifest`, the `policy` directory next to the manifest must hold exactly one file per target (a missing or stray file names itself); exit 0, or 2 with the error.
- **Contract v0:** twelve sections in the planned order, version 0.1.0, changelog `- 0.1.0 (2026-10-21)`. It carries the budgets of plan 03-01 (`Budgets::DEFAULT`: 10000 ms, `0x10000000`, 1 MiB heap, sentinels `0xA5` and `0x5A`, 65536 input), the guard-page limits, the two out-of-contract hardware differences and the user-mode distinction.

## Task Commits

1. **Tracer** (`tdd`): `4308c8a` test (RED), `e56ca97` feat (GREEN)
2. **Task 2** (`tdd`): `3cded6a` test (RED), `8adb31a` feat (GREEN)
3. **Task 3** (`tdd`): `1e169b0` test (RED), `a8a1e8f` docs (GREEN)

No REFACTOR commits were needed. **Plan metadata:** the docs commits that carry this file, STATE.md, ROADMAP.md and REQUIREMENTS.md. Nothing was pushed (plan rule): the six commits above, the two 03-01 commits and these metadata commits are local and ride with plan 03-03's first push.

## TDD evidence (RED output, with a one-line assessment per failure)

**Tracer RED (`4308c8a`: tests plus the crate skeleton only)**

```
error[E0432]: unresolved imports `mt_contract::Action`, `mt_contract::CONTRACT_VERSION`, `mt_contract::Policy`
 --> crates/mt-contract/tests/eqv_01_policy.rs:7:19
  no `Policy` in the root / no `CONTRACT_VERSION` in the root / no `Action` in the root
error: could not compile `mt-contract` (test "eqv_01_policy") due to 1 previous error
```

Assessment: right reason; the three policy tests cannot compile because the skeleton holds neither the types nor the constant.

```
test eqv_01_policy_check_cli_accepts_the_corejson_policy ... FAILED
  assertion `left == right` failed: stderr: error: unrecognized subcommand 'policy-check'
test eqv_01_policy_check_cli_rejects_a_bad_policy ... FAILED
  the refusal must name the key: error: unrecognized subcommand 'policy-check'
```

Assessment: right reason; clap's usage error exits 2, so the exit-code half of the "rejects" test passes by accident, and its assertion that the refusal names the key (and is not "unrecognized") is what fails. The bad-policy test embeds its text so it does not fail on the missing corejson file.

**Task 2 RED (`3cded6a`: test files only)**

```
error[E0599]: no method named `override_report` found for struct `Policy` in the current scope   (x3)
error: could not compile `mt-contract` (test "eqv_01_policy") due to 3 previous errors
```

Assessment: right reason, but the compile error hides the assertion failures. To see them without touching the implementation I ran a scratch copy of the test file without the report test (not committed, deleted), against the unchanged Tracer code:

```
test eqv_01_default_change_only_with_a_changed_default ... FAILED   (unwrap_err on an Ok value)
test eqv_01_default_change_required_to_change_default ... FAILED
test eqv_01_error_codes_are_checked ... FAILED                     (value "" accepted)
test eqv_01_override_naming_no_logged_site_is_rejected ... FAILED  (override accepted with an empty site list)
test eqv_01_override_needs_every_field ... FAILED                  (function = "" accepted)
test eqv_01_rejects_bad_dates ... FAILED                           (2026-02-29 accepted)
test eqv_01_rejects_duplicate_sites ... FAILED                     (two identical sites accepted)
test result: FAILED. 5 passed; 7 failed
```

Assessment: each of the seven fails because the Tracer's validate accepts what the rule must refuse. The five that passed are the three Tracer tests plus `eqv_01_empty_override_table_is_accepted` and `eqv_01_override_of_a_logged_site_is_accepted`, which are acceptance cases that hold trivially before the rules exist; they guard against the GREEN rules over-rejecting.

```
test eqv_01_committed_policies_cover_every_target ... FAILED   (stems of testbed/policy differ from the manifest's 16 ids: only corejson exists)
test eqv_01_policy_check_cli_checks_every_target ... FAILED    (stderr: error: unexpected argument '--manifest' found)
```

Assessment: both right reasons (15 files missing; flag not implemented).

**Task 3 RED (`1e169b0`)**

```
test eqv_01_contract_document_has_version_and_changelog ... FAILED
test eqv_01_contract_document_makes_no_claim ... FAILED
test eqv_01_contract_document_states_the_rules ... FAILED
  cannot read /home/user/migrationtool/docs/specs/equivalence-contract.md: No such file or directory (os error 2)
```

Assessment: right reason; the document does not exist.

GREEN: all three tasks' named tests pass (`cargo test -p mt-contract`: 3 + 1 + 13 tests; `cargo test -p mt-cli --test eqv_01_policy_check_cli`: 3 tests), and the four pre-merge gates exit 0 at each GREEN commit.

## Files Created/Modified

See `key-files`. `Cargo.lock` gained only the path crate `mt-contract` (10 added lines, one `+name =`); no external crate was added.

## Decisions Made

See `key-decisions`. Items the founder may want to revisit: the `[[ub.error_code]]` table (defined, empty everywhere, filled by Phase 4), free-text `kind` and `location` until Phase 4, and that the contract states the user-mode distinction in its own text rather than editing `container/pins.toml`.

## Deviations from Plan

**1. [Minor] `ErrorValue` is an enum, not a string.** The plan lists `ErrorValue` among the types without a shape. It is `Integer(i64)` or `Text(String)` (untagged), so `value = -1` and `value = "-1"` both load; text must be a decimal integer or a C identifier. Found during: Tracer. Files: `crates/mt-contract/src/policy.rs`. Committed in `e56ca97`.

**2. [Minor] `PolicyError::Parse` is not a `#[from]` source.** With `#[from]`, `anyhow`'s chain printer showed the TOML error twice in `mt`'s output; it now has a manual `From` and no `source`. Found during: Tracer. Committed in `e56ca97`.

**3. [Rule 1 - Lint] clippy `useless_format` in a RED test.** `eqv_01_rejects_other_schema_version` used `format!` on a literal; fixed in the Tracer GREEN commit `e56ca97` (the test file changes only in form, not in what it asserts).

**4. [Minor] Two small public additions the plan did not name:** `Policy::summary_row` and `policy::load_dir`. They keep the row format and the "one policy per target" check in the library crate so `mt-cli` stays argument handling. `load_dir` takes the ids as a slice, so `mt-contract` does not depend on `mt-capture`.

**5. [Minor] Tracer CLI bad-policy test is self-contained.** It writes its own text instead of editing `testbed/policy/corejson.toml`, so its RED failure is about the missing subcommand and not about a missing file.

**Total deviations:** 5 (1 Rule 1 lint, 4 minor). **Impact:** none on the plan's goal.

## Tracer feedback gate

Auto mode was off and `human_verify_mode` was the default `end-of-phase`; the tracer's `<verify>` is automated-only, so it was re-run after the Tracer GREEN commit (both `eqv_01_policy` and `eqv_01_policy_check_cli` named tests green plus the four gates) and logged `Tracer verified end-to-end - expanding`. No checkpoint was synthesized.

## Plan-level verification (re-run after Task 3)

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check`: all exit 0.
- `git diff dcb9462 -- Cargo.lock`: one `+name =` line, `"mt-contract"`.
- `grep -rniE 'certif[i]ed|complia[n]t' docs/specs/equivalence-contract.md testbed/policy crates/mt-contract`: no match.
- `mt vectors policy-check --manifest testbed/manifest.toml`: 16 rows, each `<id> valid default=return_error overrides=0 error_codes=0`, exit 0.
- No `unwrap`/`expect` outside tests in `crates/mt-contract/src` (two `unwrap_or` calls only); `#![forbid(unsafe_code)]` in `mt-contract`.

## Known Stubs

None.

## Threat Flags

None beyond the plan's register. Mitigations exercised: T-03-05 by `eqv_01_rejects_unknown_key` (five levels) and `eqv_01_rejects_other_schema_version`; T-03-06 by `eqv_01_default_change_required_to_change_default`, `eqv_01_override_needs_every_field`, `eqv_01_rejects_bad_dates` and `eqv_01_override_report_lists_every_override`; T-03-07 by `eqv_01_override_naming_no_logged_site_is_rejected` and `eqv_01_committed_policies_cover_every_target`; T-03-08 by `eqv_01_contract_document_makes_no_claim`; T-03-SC: no external crate added.

## Issues Encountered

None. Not pushed by design: this plan changes nothing CI must prove in the image.

## Next Phase Readiness

- Plan 03-03 (the first push) can start: `mt-contract` exists as a path crate, `Policy::load` of `<testbed>/policy/<target>.toml` plus `validate(&[])` is what plan 03-04's executor loads, and the contract text for the field convention and library-state shapes is fixed for `mt-vectors`.
- The founder reads the contract against D-05 to D-30 at `/gsd-verify-work` (manual-only row of 03-VALIDATION.md).

## Self-Check: PASSED

- Created files found on disk: the `mt-contract` sources and three test files, `eqv_01_policy_check_cli.rs`, `docs/specs/equivalence-contract.md`, and all 16 files under `testbed/policy` (16 `.toml` files counted).
- Commits found as ancestors of HEAD: 4308c8a, e56ca97, 3cded6a, 8adb31a, 1e169b0, a8a1e8f.
- Plan-level verification and every task's acceptance criteria re-run after Task 3: pass.
