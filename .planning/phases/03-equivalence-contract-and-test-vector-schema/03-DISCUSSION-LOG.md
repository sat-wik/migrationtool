# Phase 3: Equivalence Contract and Test-Vector Schema - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md; this log preserves the alternatives considered.

**Date:** 2026-10-21
**Phase:** 03-equivalence-contract-and-test-vector-schema
**Areas discussed:**
- Which C build runs
- Fitting calls into TRACTOR
- Adapters
- Contract & UB policy
- Vector storage and scope (follow-up)
- Provenance, comparison and sizing (follow-up)
- State contents and inputs (follow-up)

The selected option is shown in bold in every table below.

---

## Which C build runs

| Question | Options | Selected |
|---|---|---|
| Which build produces the expected answer | **Cortex-M under QEMU** / Host ILP32 build / Both, compared | Cortex-M under QEMU |
| Determinism and limits | **Fixed sandbox + limits (double run)** / Limits only | Fixed sandbox + limits |
| C crash on an input | **Record it, mark has_ub** / Record it, leave has_ub unset | Record it, mark has_ub |

## Fitting calls into TRACTOR

| Question | Options | Selected |
|---|---|---|
| Call-to-field mapping | **Fixed convention** / Pack everything in lib_state | Fixed convention |
| lib_state encoding | **Field by field** / Raw bytes of the struct | Field by field |
| Output buffer comparison | **Whole buffer, prefilled** / Only the reported length | Whole buffer, prefilled |
| TRACTOR "loads unchanged" proof | **Fetch pinned, test locally** / Hand-copy 3 vectors | Fetch pinned, test locally |

## Adapters

| Question | Options | Selected |
|---|---|---|
| Where adapters are declared | **Manifest + generated driver** / Hand-written C driver per target / Manifest roles + optional C shim | Manifest + generated driver |
| Entries needing prior state | **Declared setup sequence** / Only direct entries now | Declared setup sequence |
| Which targets | **Every target that fits** / Exactly the easiest 10 | Every target that fits |

**Note:** the question described the six `wrapped` entries as needing a setup call. In the manifest, `wrapped` actually means a struct-wrapped buffer (Phase 2 D-28). CONTEXT D-12 records that the declared setup sequence covers both cases.

## Contract & UB policy

| Question | Options | Selected |
|---|---|---|
| Contract form | **Markdown doc + TOML policy** / Doc + JSON policy | Markdown doc + TOML policy |
| Granularity | **Module default + per-site override** / Module-wide only | Module default + per-site override |
| Action meanings | **As proposed (return error / panic = abort trap / preserve compiled C)** / Let research propose | As proposed |
| Change record | **Who, why, when, where** / Reason only | Who, why, when, where |

## Vector storage and scope

| Question | Options | Selected |
|---|---|---|
| Vector storage | **Work dir, sample committed** / Commit all vectors | Work dir, sample committed |
| Generation scope | **Seeds only** / Seeds + simple mutations | Seeds only |
| has_ub before Phase 4 | **false + provenance** / Leave the field null | false + provenance |

## Provenance, comparison and sizing

| Question | Options | Selected |
|---|---|---|
| Provenance location | **Sidecar index entry** / Encode in argv/env | Sidecar index entry |
| Compared fields | **rc, stdout, lib_state_out (+ stderr outcome class)** / All output fields verbatim | rc, stdout, lib_state_out |
| Phase sizing | **Planner splits; 10 targets first** / Split into Phase 3 and 3.1 | Planner splits; 10 targets first |

## State contents and inputs

| Question | Options | Selected |
|---|---|---|
| State struct fields | **All fields, from the header** / Only fields the adapter lists | All fields, from the header |
| Globals | **Every writable global, auto-found** / Declared globals only | Every writable global, auto-found |
| Input buffer placement | **End-aligned against a guard page** / Plain allocation | End-aligned against a guard page |
| Scalar arguments | **Adapter default + small list** / Defaults only | Adapter default + small list |

## Claude's Discretion

- How the driver is generated and linked (startup code, semihosting or a stub).
- The budget values, the sentinel byte and the guard-page mechanism.
- The CLI shape.
- The source of struct layout: DWARF or clang's AST.
- The crate split.

## Deferred Ideas

- Mutation and fuzz input generation (Phase 5).
- A host ILP32 back end (Phase 5).
- Real per-site UB overrides (after Phase 4).
- The TRACTOR licence check (Phase 12).
