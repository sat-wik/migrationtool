# migrationtool

> Working name; product name TBD.

## What This Is

A Rust tool that migrates a device maker's riskiest embedded C modules into safe, `no_std`, allocation-free Rust behind the existing C interface, and ships each module with a re-runnable evidence bundle showing the Rust behaves like the C. Translation (c2rust + Hayroll + LLM agents) is treated as a commodity; the validation harness and the evidence it produces are the product. The first beachhead is hardware-independent parsers and protocol handlers (TLV, CBOR, MQTT, BLE, OTA image headers) on Arm Cortex-M built with arm-none-eabi GCC. Builder: one founder working nights and weekends with Claude Code and GSD.

## Core Value

Every migrated module ships with evidence, produced by a harness that is itself proven to catch planted bugs, that the Rust matches the C on every input where the C has defined behavior. Developer-facing success metric: the share of C modules that reach full evidence with zero divergences on defined-behavior inputs.

## Business Context

- **Customer**: Firmware leads, product-security (PSIRT) owners and compliance managers at connected-device makers, first those selling into the EU where the Cyber Resilience Act applies.
- **Revenue model**: Services-led paid pilots first (hypotheses: scoping assessment about $15k-$30k, paid pilot about $25k-$50k), later per-module migration and a continuous-equivalence subscription. All figures are hypotheses to test in discovery.
- **Success metric**: Day-90 gate: the harness catches 100% of planted bugs; at least 70% of test-bed parsers are fully evidenced (zero divergences); at least 2 paid pilots or letters of intent. The last item is a business gate, not a code deliverable.
- **Strategy notes**: `docs/specs/product-spec.md` (SPEC), `docs/research/VERIFICATION.md` (fact-check and risks R1-R6), `docs/research/literature-map.md` (background).

## Requirements

Full list, IDs and traceability: `.planning/REQUIREMENTS.md`. Roadmap: `.planning/ROADMAP.md`.

### Validated

(None yet — ship to validate)

### Active

Milestone v1.0 MVP (spec months 0-3), 63 requirements across 12 phases:

- [ ] Pinned, reproducible container with one LLVM (16-19) for all C-side analysis; analysers run as subprocesses (TOOL)
- [ ] Test bed of 10-20 open-source C parsers whose real builds are captured and replayed exactly, per shipped configuration (BED, BUILD)
- [ ] Equivalence contract v0 (refinement) and a TRACTOR-compatible test-vector schema, with a C reference executor (EQV)
- [ ] UB detection: separate ASan+UBSan and MSan builds, -O0 vs real-flags differential, C-side symbolic scan, UB decisions log with error default (UB)
- [ ] Differential fuzzing harness on an ILP32, unsigned-char configuration that matches the Cortex-M target, with C-side branch coverage (DIFF)
- [ ] Go/no-go spikes for Rust-to-KLEE (R1) and c2rust-output-as-Kani-reference (R2) (SYM)
- [ ] Harness proven by 20 planted bugs and mutation testing; deterministic anti-gaming gates; held-out corpus (SELF, GATE)
- [ ] Translator front end behind adapters (c2rust, Hayroll after licence check), dependency graph, function map (FRONT)
- [ ] Model-agnostic agentic translation, function by function in dependency order, faithful then idiomatic (PROV, XLATE)
- [ ] C-ABI boundary: header, Cargo crate, Make/CMake hook, existing tests pass with Rust swapped in (BOUND)
- [ ] Evidence bundle v0 and scoping report v0 (EVID, ASSESS)
- [ ] Public benchmark (TRACTOR public tests plus test bed) and the technical day-90 gate report (BENCH)

### Out of Scope

Explicit boundaries. Reasons included so they are not re-added by accident.

- C++ — spec non-goal; C only.
- Whole-firmware rewrites — the product migrates one module at a time behind the existing C interface.
- Interrupt handlers, drivers and hardware-facing code — need device models (Renode, svd2rust); deferred to milestone v3.0.
- Worst-case timing guarantees — only if contracted separately; real-time paths are flagged, not proven.
- Certifying a product or claiming compliance on a customer's behalf — we supply evidence; the customer's assessor decides. The tool must never emit such a claim.
- General-purpose coding-assistant features — positioning is evidence, not assistance.
- Python in the shipped tool — ADR-0001 (LOCKED); Python only in throwaway scripts under `research/`.
- Bounded Kani proofs as MVP evidence — Kani is spec-v1 scope and its use with c2rust output is unproven (R2); deferred to milestone v2.0.
- Review UI, CI integration, on-target resource gates, SBOM, IAR/Keil builds, SSO/RBAC, data-retention tooling — spec-v1 scope; see Milestones below.
- Customer interviews, design-partner pilots, LOIs and pricing work — business activities, tracked under Context, not code phases.

## Milestones

Naming: the spec labels its stages MVP, v1 and v2. GSD's active milestone is resolved from `STATE.md` (`milestone: v1.0`), so the spec labels are mapped to GSD versions as below. In this repository "spec-v1" always means the spec's months 4-9 stage.

| GSD milestone | Spec stage | Window | Status | Gate to start |
|---------------|-----------|--------|--------|---------------|
| v1.0 MVP | MVP | months 0-3 | Active (this roadmap, 12 phases) | None |
| v2.0 Customer Platform | spec-v1 | months 4-9 | Deferred, not yet phased | Day-90 gate passes |
| v3.0 Hardware and Scale | spec-v2 | months 10-18 | Deferred, not yet phased | spec-v1 gate: 3 paying customers and first subscription renewal |

If the day-90 gate fails, the spec says to narrow the niche or the module class rather than add features. Deferred items are listed in `REQUIREMENTS.md` under "v2 Requirements (Deferred)".

## MVP Definition of "Full Evidence"

ASSUMPTION A-01 — confirm with the founder. The sources never define "full evidence" for the day-90 gate (the spec's own module 4 acceptance includes bounded proofs, but the MVP omits Kani). For the MVP, a module is **fully evidenced** when all of the following hold:

1. Zero divergences under differential fuzzing on the ILP32 target-matched harness (`-funsigned-char`, shipped-configuration flags), over a recorded fuzzing budget. Proposed budget: at least 1 CPU-hour per module and C-side coverage plateaued for 30 minutes, whichever is later.
2. C-side branch coverage threshold met (spec proposes at least 90%); uncovered branches are listed.
3. KLEE per-function results are included where the R1 spike passed. Per-function verdicts are equivalent-within-bound, divergent, or timeout; a timeout is reported as "not proven" and never as a pass. If R1 is NO-GO, this term is dropped and the report says "symbolic: unavailable".
4. Every UB site and input class found is logged in the UB decisions log with a decision (default: return an error).
5. The harness mutation kill rate is reported in the bundle. It is reported, not thresholded.

Excluded until milestone v2.0: bounded Kani proofs, on-target resource gates, SBOM, named-reviewer sign-off UI.

Precondition added by the roadmapper (strike it if you disagree): the module's Rust passed the deterministic translation gates (compiles for `thumbv7em-none-eabihf`, no stubs, `unsafe` only at the C boundary, every C function mapped and exercised). Without it a stubbed module could satisfy terms 1-5 vacuously.

For the open-source test bed the UB-decision approver is recorded as the founder acting as proxy; a customer approver appears only in pilots.

## Context

**Thesis.** The hard part is not getting an LLM to write Rust; it is defining and proving "equivalent" for C with undefined behavior, weak tests, preprocessor variants and hardware side effects. Known evidence: c2rust fails on 15 of 16 embedded codebases (CCS 2024); automated tools carried 177 inherited memory bugs and added 77 new ones in one study (arXiv 2609.25682); vanilla agents can pass while exercising 64.4% of functions (not re-checked by VERIFICATION). Only one peer-reviewed embedded measurement exists, so this is both opportunity and from-scratch risk.

**Technical risks from VERIFICATION (R1-R6) and where the roadmap handles them.**

| Risk | Handling | Phase |
|------|----------|-------|
| R1 LLVM version lock; KLEE vs current rustc | Single pin in 16-19; Rust-to-KLEE spike with go/no-go | 1 (pin), 6 (spike) |
| R2 c2rust output as Kani reference unproven | Time-boxed spike; may slip to v2.0; fuzzing plus KLEE stay primary | 6 |
| R3 Hayroll licence unconfirmed | Licence gate, then adapter so it can be swapped | 8 |
| R4 Ferrocene certifies only part of `core` | Deferred output profile and pinned MSRV | v2.0 |
| R5 Host/target ABI mismatch (LP64 vs ILP32, char signedness) | ILP32 `-funsigned-char` harness, final corpus replay on emulated target | 5 |
| R6 MSan cannot share a build with ASan | Separate ASan+UBSan and MSan variants | 4 |

**Parallel business track (founder-run, not phased).** These feed the business half of the day-90 gate and may revise evidence requirements; they are not code deliverables.

- Weeks 4-10 of the spec timeline: interview about 20 firmware leads, PSIRT owners and compliance managers; goal is 2-3 design partners. Questions to settle: which modules worry them, what evidence auditors accept, whether code may leave the network, who signs off, what a migration is worth (ask each for their manual-rewrite estimate per module).
- Weeks 8-13: one paid pilot on a design partner's module with the full evidence package.
- Collect at least 2 paid pilots or signed LOIs before the day-90 review. Record the count as a manual input to the Phase 12 gate report.
- Before starting: check the founder's day-job invention-assignment clause and keep this work on own time and hardware (spec caveat).

**Open spec questions** (resolve in discovery, they can change scope): first vertical and RTOS (Zephyr industrial IoT vs FreeRTOS consumer IoT); what evidence assessors and notified bodies accept; local-model translation quality and cost; price vs customers' rewrite estimates; whether "return an error" is an acceptable UB default; linking Rust into IAR and Keil builds. Parsers are RTOS-independent, so Phase 2 only needs the first-RTOS choice recorded, not made final.

**Code-generation guidelines** (binding on every phase): `docs/guidelines/building-the-tool.md` for the tool's own code and the phase loop; `docs/specs/emitted-rust-rules.md` for the Rust the tool emits, mapped to GATE, BOUND, DIFF and XLATE requirements. Summary in `.planning/codebase/CONVENTIONS.md`, projected into `.claude/CLAUDE.md`.

**Source documents** (precedence ADR-0001 > VERIFICATION > product-spec > literature-map): `docs/adr/0001-tool-implementation-language.md`, `docs/research/VERIFICATION.md`, `docs/specs/product-spec.md`, `docs/research/literature-map.md`. Synthesized intel: `.planning/intel/`. Ingest conflict report (0 blockers): `.planning/INGEST-CONFLICTS.md`.

## Constraints

- **Tech stack (LOCKED)**: Rust, Cargo workspace; analysers (clang/LLVM, KLEE, c2rust, Hayroll, Kani, Bear) as pinned subprocesses in a reproducible container; Rust agent loop behind a provider trait — ADR-0001.
- **Runtime**: The tool runs on Linux x86-64 hosts inside a pinned container. It emits `no_std`, allocation-free Rust for Arm Cortex-M (`thumbv7em-none-eabihf` first) behind the existing C ABI — spec customer requirement 3; one static binary per platform is the ADR consequence.
- **Toolchain**: One LLVM major in 16-19 for all C-side analysis. KLEE 3.2 recommends 16 and partially supports 17-19; Hayroll needs one matching LLVM and rejects 20; c2rust needs 15 or later; rustc used for bitcode must report an LLVM at or below 19 — VERIFICATION R1.
- **Harness fidelity**: Differential runs must use an ILP32 configuration with `-funsigned-char` and the customer's flags; ASan+UBSan and MSan are separate builds; replay the final corpus on an emulated or real target — R5, R6.
- **Equivalence**: Refinement, not equality. The Rust matches the C on defined-behavior inputs and takes an approved safe action (default: return an error) on UB inputs; never crash or corrupt memory — spec problem 1.
- **Schema**: Test vectors use the TRACTOR fields `argv, stdin, env, stdout, stderr, rc, lib_state_in, lib_state_out, has_ub` (two lib_state fields, not one) — VERIFICATION correction.
- **Scope**: C only; GCC and Cortex-M first; parsers and protocol logic only; one RTOS ecosystem and arm-none-eabi GCC — spec non-goals and first release.
- **Claims**: Never state that a product or module is certified or compliant — spec.
- **Legal**: Hayroll licence unconfirmed (R3) and TRACTOR corpus licence unchecked; resolve before depending on or redistributing either. Check the day-job invention-assignment clause before starting.
- **Builder capacity**: One founder, nights and weekends. Each phase must be finishable in 1-2 weeks of part-time work. No time estimates are written into plans beyond that sizing rule.
- **Ordering**: Validation harness before translator; spikes with go/no-go before dependent work; nothing in v2.0 starts until the day-90 gate passes — spec.

## Decisions

<decisions>
## Implementation Decisions

### Locked by ADR-0001 (changing any of these needs a superseding ADR)
- **D-01:** The migration tool is written in Rust as a Cargo workspace (ADR-0001, LOCKED).
- **D-02:** External analysers (clang/LLVM, KLEE, c2rust, Hayroll, Kani, Bear) are invoked as pinned subprocesses inside a reproducible container and are never linked as libraries (ADR-0001, LOCKED).
- **D-03:** The LLM agent loop is written in Rust behind a model-agnostic provider trait so hosted and locally hosted open-weight models are interchangeable (ADR-0001, LOCKED).
- **D-04:** Python may appear only in throwaway research scripts under `research/`, never in the shipped tool (ADR-0001, LOCKED).

### Roadmap decisions (from the user brief, SPEC and VERIFICATION; revisable through the Key Decisions table)
- **D-05:** The validation harness is built and proven before any translator code exists (SPEC: "Build the validation harness before the translator").
- **D-06:** The differential harness runs on an ILP32 target-matched configuration with `-funsigned-char` and the shipped-configuration flags, not on a default LP64 host (VERIFICATION R5).
- **D-07:** ASan+UBSan and MSan are built as separate C variants and the corpus is replayed against each (VERIFICATION R6).
- **D-08:** One LLVM major version in 16-19 is pinned for all C-side analysis, and Hayroll is never run on LLVM 20 (VERIFICATION R1).
- **D-09:** Hayroll sits behind an adapter and is not depended on until its licence is confirmed or written permission is obtained (VERIFICATION R3).
- **D-10:** R1 (Rust-to-KLEE) is a required spike with go/no-go; R2 (c2rust output as Kani reference) is a time-boxed spike that may slip to milestone v2.0; fuzzing plus KLEE remain the primary evidence until R2 passes (VERIFICATION R1, R2).
- **D-11:** Equivalence is refinement; the UB default action is "return an error" and is customer-adjustable (SPEC problem 1).
- **D-12:** Correctness never depends on which model translated the code; the validation engine decides what ships (SPEC deployment design rule, ADR-0001).
- **D-13:** The tool never claims a product or module is certified or compliant (SPEC).
- **D-14:** For the MVP, "full evidence" means the five-term definition in the "MVP Definition of Full Evidence" section; bounded Kani proofs are excluded until milestone v2.0 (roadmapper proposal, assumption A-01 pending founder confirmation).
</decisions>

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Rust Cargo workspace; analysers as pinned subprocesses (D-01 to D-04) | Static binary for air-gapped installs; tool and output share a toolchain; analysers evolve independently | Locked (ADR-0001) |
| Harness before translator (D-05) | Translation is a commodity; unvalidated translation cannot be trusted or sold | Pending — validated when Phase 7 passes |
| ILP32 target-matched differential harness (D-06) | A default x86-64 host can agree with the Rust and disagree with the target | Pending — validated in Phase 5 |
| Separate ASan+UBSan and MSan builds (D-07) | MSan cannot share a build with ASan | Pending — Phase 4 |
| Single LLVM pin in 16-19 (D-08) | KLEE, Hayroll and c2rust version limits | Pending — provisional in Phase 1, final after R1 in Phase 6 |
| Hayroll behind an adapter, licence-gated (D-09) | Licence unconfirmed; keep it swappable | Pending — Phase 8 |
| R1 required spike, R2 time-boxed (D-10) | Both are unproven; each decides how the evidence layer is built | Pending — Phase 6 |
| MVP "full evidence" excludes Kani (D-14) | Kani is spec-v1 and R2 is unproven | Assumed — confirm (A-01) |
| 12 phases, sized 1-2 weeks of part-time work each | Builder constraint outranks the default "standard" granularity (4-6 phases) | Assumed — see A-09 |
| First RTOS ecosystem (Zephyr vs FreeRTOS) | Spec open question; parsers are RTOS-independent so it is not blocking | Pending — record in Phase 2 |

## Assumptions to Confirm

These are roadmapper proposals or inferences. None is stated by a source document as a decision.

- **A-01**: The MVP definition of "full evidence" above, including the fuzz budget (1 CPU-hour and a 30-minute coverage plateau) and the added gates precondition.
- **A-02**: Numeric thresholds marked proposed in the spec (90% branch coverage, 70% fully evidenced, 20 planted bugs, 10% resource regression, 30-minute CI, plus or minus 30% scoping accuracy) stand until design partners say otherwise.
- **A-03**: The go/no-go pass/fail criteria and time boxes for R1, R2 and R3 in `ROADMAP.md` are proposed by the roadmapper because the sources lack them.
- **A-04**: Deferred scope is split as: spec-v1 items (Kani proofs, UB manager and review UI, CI checks, resource gates, on-prem local models, SBOM and CRA evidence, Ferrocene/IAR/Keil, SSO/RBAC/audit log, data-commitment tooling) become milestone v2.0; spec-v2 items become v3.0.
- **A-05**: The scoping report in the MVP carries no price or effort figure. The plus or minus 30% accuracy target needs pilot actuals, so effort and price estimation is deferred.
- **A-06**: The idiomatic second translation stage stays in the MVP at modest scope and is the first thing to cut if the schedule slips (see the cut line in `ROADMAP.md`). Abstraction functions for changed data structures are deferred.
- **A-07**: Evidence bundle v0 includes seven of the nine spec artifacts plus informational code-size and timing numbers; the SBOM and gated resource report wait for v2.0.
- **A-08**: MemorySanitizer is believed not to support 32-bit x86, so the MSan variant would run LP64 with `-funsigned-char` while ASan, UBSan and the differential run stay ILP32. This is the roadmapper's own recollection, not a sourced fact; verify in Phase 4.
- **A-09**: No `config.json` existed. Phase IDs are sequential. The phase count (12) follows the 1-2 week sizing rule; the "standard" default of 4-6 phases would force phases of 4 or more weeks each.
- **A-10**: Schedule. Twelve phases at 1-2 part-time weeks each is 12-24 weeks, against the spec's 13-week day-90 window written for fuller-time effort. The day-90 date is at risk unless scope is cut or the date moves; see the cut line in `ROADMAP.md`.

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-10-07 after initialization (ingest of ADR-0001, product spec, VERIFICATION, literature map)*
