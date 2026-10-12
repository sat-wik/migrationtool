# Requirements: migrationtool

**Defined:** 2026-10-07
**Core Value:** Every migrated module ships with evidence, produced by a harness proven to catch planted bugs, that the Rust matches the C on every input where the C has defined behavior. Metric: share of C modules that reach full evidence with zero divergences.

> Naming note. "v1 Requirements" below means the first GSD milestone, v1.0 MVP (spec months 0-3). The spec's own "v1" stage (months 4-9) is called "spec-v1" and lives under "v2 Requirements (Deferred)" as milestone v2.0. Spec-v2 (months 10-18) is milestone v3.0. See PROJECT.md "Milestones".
>
> Thresholds marked "(proposed)" come from the spec and are to be confirmed with design partners. Spike pass/fail criteria are written into the phase success criteria in ROADMAP.md and are roadmapper proposals.

## v1 Requirements

Milestone v1.0 MVP. Each maps to exactly one roadmap phase.

### Toolchain and Container (TOOL)

- [x] **TOOL-01**: The tool is a Rust Cargo workspace that runs every external analyser (clang/LLVM, KLEE, c2rust, Hayroll, Kani, Bear, QEMU) as a pinned subprocess inside the container and never links one as a library; a repository check fails if any Python source exists outside `research/`.
- [x] **TOOL-02**: One LLVM major version in 16-19 is pinned for all C-side analysis (clang, KLEE, c2rust, Hayroll), and a version check fails if any of them uses a different major or if the rustc used for bitcode reports an LLVM above 19 in `rustc -vV`.
- [x] **TOOL-03**: The container image is reproducible: base image, tool sources and Rust toolchain are pinned by digest, commit or checksum, and building twice yields an identical tool-version manifest.
- [x] **TOOL-04**: The container provides arm-none-eabi GCC and the `thumbv7em-none-eabihf` Rust target, and a trivial `no_std` crate builds for that target inside it.

### Test Bed and Build Capture (BED, BUILD)

- [ ] **BED-01**: A test bed of 10-20 open-source C parser or protocol targets (for example zcbor, tinycbor, nanopb, lwIP protocol parsers, Mbed TLS ASN.1) is pinned by commit, with licence recorded and at least one buffer-in entry function identified per target.
- [ ] **BED-02**: Each test-bed target declares its shipped configurations (macro and flag sets) declaratively, and the first RTOS ecosystem (Zephyr or FreeRTOS) is recorded as a decision, even if recorded as "deferred; parsers are RTOS-independent".
- [ ] **BUILD-01**: For each target and configuration, the tool captures every compile command (Bear for Make, exported compile commands for CMake) into a `compile_commands.json`.
- [ ] **BUILD-02**: Replaying the captured commands in the container reproduces the target's original build output (byte-identical objects, or identical after a documented normalisation) for every retained target and configuration; targets that cannot be reproduced are dropped from the bed with the cause logged, keeping at least 10.
- [ ] **BUILD-03**: Each target's existing tests, fixtures and sample inputs are extracted into a per-target seed corpus; targets with none get at least 10 hand-made seeds (proposed).

### Equivalence Contract and Test Vectors (EQV)

- [ ] **EQV-01**: Equivalence contract v0 exists as a versioned document and a machine-readable policy: refinement, observation point at the C interface (return values, output buffers, global state across the boundary), UB action menu {return error (default), panic, preserve}, and a rule that the Rust never crashes or corrupts memory.
- [ ] **EQV-02**: A test-vector schema with exactly the fields `argv, stdin, env, stdout, stderr, rc, lib_state_in, lib_state_out, has_ub` validates vectors, round-trips in Rust types, rejects a single merged `lib_state`, and treats `has_ub` vectors as excluded from output comparison.
- [ ] **EQV-03**: A per-target adapter declares each parser entry point and its argument roles so that buffer-in, buffer-out functions can be driven by vectors; at least 10 test-bed targets are drivable.
- [ ] **EQV-04**: A C reference executor runs any vector against the target's original C build and records the expected result in a schema-valid vector; running the same vector twice gives identical results.

### UB Detection and Decision Log (UB)

- [ ] **UB-01**: The tool builds separate ASan+UBSan and MSan variants of the C, replays the corpus against each, and sets `has_ub` on any input that either variant flags.
- [ ] **UB-02**: The tool replays every input against the C built at `-O0` and at the shipped-configuration flags, and tags any input where the two disagree as suspected UB.
- [ ] **UB-03**: A KLEE scan (C side, pinned LLVM) or Frama-C scan of parser entry functions lists UB sites the corpus never reached, with per-function completion or timeout recorded, and merges them into the UB log with provenance.
- [ ] **UB-04**: Implementation-defined behavior (plain `char` signedness, integer and pointer sizes, layout of every struct crossing the interface) is extracted from the target configuration rather than assumed, and logged for sign-off.
- [ ] **UB-05**: A UB decisions log records, per site or input class, the decision, rationale, approver and date; every detected entry defaults to "return error", and the tool refuses to mark a module UB-complete while any entry lacks a decision.

### Differential Harness (DIFF)

- [ ] **DIFF-01**: A single command links the original C and a candidate Rust module behind the same C interface into a libFuzzer or cargo-fuzz target and compares outputs and state on every input.
- [ ] **DIFF-02**: The harness runs on an ILP32 configuration matching the target (default i686 `-m32`; Arm Linux under QEMU user mode as alternative) with `-funsigned-char` and the shipped-configuration flags, and a probe pair shows that a default LP64 signed-`char` run would disagree where the target-matched run does not.
- [ ] **DIFF-03**: For inputs with `has_ub`, the harness checks the Rust against the UB policy (default: returns the error code) instead of the C output, and reports a crash or memory corruption in the Rust as a divergence.
- [ ] **DIFF-04**: The harness measures branch coverage of the original C (not the Rust) per function and module, reports it against the threshold of at least 90% (proposed), and lists uncovered branches.
- [ ] **DIFF-05**: Fuzzing is seeded from the test-bed seed corpus plus per-target dictionaries, with protocol-aware mutators for at least CBOR and TLV-style formats.
- [ ] **DIFF-06**: The final corpus replays on an emulated Cortex-M target (Renode preferred; QEMU as fallback with the gap recorded as a limitation) and produces the same verdicts as the host ILP32 run, or the mismatches become findings.
- [ ] **DIFF-07**: Every divergence is minimized and saved as a schema-valid vector containing the counterexample input.

### Symbolic Layer and Spikes (SYM)

- [ ] **SYM-01**: The R1 spike (Rust-to-KLEE under the pinned LLVM) ends in a recorded GO or NO-GO decision against the pass/fail criteria in Phase 6.
- [ ] **SYM-02**: If R1 is GO, per-function differential symbolic checking runs as a harness layer and reports per-function verdicts in the same result schema as fuzzing; if NO-GO, the schema reports "symbolic: unavailable" and the full-evidence definition drops the KLEE term.
- [ ] **SYM-03**: The R2 spike (c2rust output as Kani reference, with libc stubs) ends in a recorded GO, NO-GO or "deferred to v2.0" decision within its time box, and Kani's known blind spots (aliasing, unaligned dereference, concurrency, inline assembly) are written into the evidence caveat text regardless of the verdict.

### Harness Self-Validation and Gates (SELF, GATE)

- [ ] **SELF-01**: 20 known bugs planted in hand-written Rust, spanning at least 6 bug classes, are all caught by the harness, and the report names which layer caught each.
- [ ] **SELF-02**: cargo-mutants runs over the hand-written Rust and the kill rate is published with surviving mutants classified as equivalent mutant or harness gap.
- [ ] **SELF-03**: The planted-bug suite and the gate-violation suite run from one command and fail the build if any planted bug or violation is missed.
- [ ] **GATE-01**: A deterministic gate rejects candidate Rust that does not compile for `thumbv7em-none-eabihf`, is not `#![no_std]`, or uses the `alloc` crate or heap APIs.
- [ ] **GATE-02**: A deterministic gate rejects stubs: `todo!()`, `unimplemented!()`, bodies consisting only of `panic!` or `unreachable!`, and empty bodies of non-unit functions.
- [ ] **GATE-03**: A deterministic gate rejects `unsafe` outside designated C-boundary modules and any `unsafe` block lacking a written SAFETY justification, and emits the remaining blocks for the unsafe ledger.
- [ ] **GATE-05**: The held-out test corpus is stored outside the agent-visible workspace, is used only for final verdicts, and a negative test shows a sandboxed agent process cannot read it.
- [ ] **GATE-06**: A deterministic gate enforces output conventions: rustfmt clean, Clippy clean with warnings denied, documentation on public items, a pinned Rust version file, and a dependency allowlist.

### Translator Front End (FRONT, GATE-04)

- [ ] **FRONT-01**: The Hayroll licence is confirmed (licence file at a recorded commit, or written permission from the authors) and recorded under `docs/licences/` before any code depends on it; if unresolved, the Hayroll adapter stays disabled and the decision is recorded.
- [ ] **FRONT-02**: A c2rust adapter consumes `compile_commands.json` and produces unsafe-Rust skeletons per translation unit, reports per-unit failures, and never aborts the pipeline on a failure.
- [ ] **FRONT-03**: A front-end trait has swappable implementations (c2rust, Hayroll when licensed, and a no-skeleton path where agents translate from the C alone), selected by configuration, with contract tests shared across implementations.
- [ ] **FRONT-04**: A clang-derived call graph yields a machine-readable translation order (callees before callers, cycles grouped as strongly connected components) for each module.
- [ ] **FRONT-05**: A function map (C function name to intended Rust symbol) is generated for each module and checked for completeness against clang's function list.
- [ ] **GATE-04**: A deterministic gate rejects any module that has a C function without a mapped Rust counterpart or without being exercised by the corpus (using C-side coverage from the differential harness).

### Agentic Translation (PROV, XLATE)

- [ ] **PROV-01**: A model-agnostic provider trait has a hosted-API implementation, a locally hosted open-weight implementation (OpenAI-compatible endpoint), and a scripted replay implementation for deterministic tests; switching providers needs configuration only.
- [ ] **XLATE-01**: The agent loop translates function by function in dependency order and runs the full gate stack and the differential harness after every step before accepting the function.
- [ ] **XLATE-02**: On a gate failure or divergence the loop feeds the counterexample back and retries within a per-function budget; functions that exhaust the budget are reported as "stay in C" candidates and never stubbed.
- [ ] **XLATE-03**: Agent runs execute in a sandbox with no access to the held-out corpus, and every prompt, response and gate verdict is logged for audit.
- [ ] **XLATE-04**: An adversarial provider that returns stubs or subtly wrong code cannot produce an accepted module; a weaker model changes only retry count and cost, never what ships.
- [ ] **XLATE-05**: Stage 1 produces faithful, C-like Rust accepted by every gate and the harness for at least 5 test-bed targets, with the translation success rate reported honestly.
- [ ] **XLATE-06**: Stage 2 refactors toward idiomatic Rust in small Rust-to-Rust steps, each checked against the previous step and the C at the C interface; failed steps are reverted and each module is labelled with its tier (faithful or idiomatic).

### Boundary and Integration (BOUND)

- [ ] **BOUND-01**: The tool generates a C header and interface shim (cbindgen-style) whose exported symbols and signatures match the original header, verified by compiling a C unit against the original header and linking it with the Rust static library.
- [ ] **BOUND-02**: The tool generates the Cargo crate (static library, `no_std`, `panic=abort`, pinned toolchain file) and Make and CMake hooks that swap one C module for the Rust library in a mixed C and Rust build, leaving other modules as C.
- [ ] **BOUND-03**: For at least 5 test-bed targets, the original full build and existing test suite pass with the Rust module swapped in, for every shipped configuration of that target.
- [ ] **BOUND-04**: A defined panic handler and `panic=abort` strategy are in place, and a test shows that a UB-tagged input returns the policy error through the C interface without relying on unwinding.

### Evidence and Scoping Report (EVID, ASSESS)

- [ ] **EVID-01**: One command produces a versioned per-module bundle with a manifest containing: equivalence report, UB decisions log, unsafe ledger, security delta report, test and fuzz assets (regression tests, corpus, harness runnable in CI), traceability matrix (C function to Rust counterpart, reviewer field), reproducibility bundle, and informational code-size and timing numbers; the manifest states that the SBOM is absent by design in v1.0.
- [ ] **EVID-02**: A machine-checked `full_evidence` predicate implements the five-term MVP definition in PROJECT.md and returns true or false with a reason per term.
- [ ] **EVID-03**: A clean checkout in a fresh container re-runs validation from the reproducibility bundle with one command and reproduces the same verdicts for at least 3 modules.
- [ ] **EVID-04**: The equivalence report states per method what was and was not done (fuzzing hours, C-side coverage, symbolic results or "unavailable", Kani "not included", divergences and resolutions, tool versions) and a test shows the generator emits no claim that a product is certified or compliant.
- [ ] **EVID-05**: The unsafe ledger and security delta report are generated from code and run data: each remaining `unsafe` block with justification and invariants, bug classes removed, crashes found in the original C during fuzzing, and sanitizer and Miri results.
- [ ] **ASSESS-01**: A scoping report v0 for a C code base outputs the dependency graph, a risk map (external-input entry points, function size and complexity, UB pre-scan counts), feasibility indicators (c2rust success, functions using inline assembly, `volatile`, hardware access or compiler extensions flagged as stay-in-C candidates), and no price or effort figure.

### Public Benchmark and Day-90 Gate (BENCH)

- [ ] **BENCH-01**: TRACTOR's public tests and tools (Test Runner, Cando) run against the tool's output for the subset it can build, `has_ub` vectors are excluded from scoring as TRACTOR does, and the corpus licence is checked before any artifact is redistributed.
- [ ] **BENCH-02**: The full pipeline runs across the test bed and records per target: fully-evidenced yes or no with the failing term, divergences, remaining `unsafe` count, code size C vs Rust, informational timing, C-side coverage and the share of functions with symbolic results.
- [ ] **BENCH-03**: A benchmark report publishes correctness, remaining unsafe, performance, code size and evidence strength with method and honest caveats, and a third party can reproduce it from the reproducibility bundle.
- [ ] **BENCH-04**: A day-90 gate report computes the planted-bug catch rate (must be 100%) and the share of test-bed parsers fully evidenced (must be at least 70%, proposed), records the pilots-or-LOIs count (at least 2) as a manually entered business input, and states continue or narrow-the-niche.

## v2 Requirements (Deferred)

Acknowledged and tracked, not in the current roadmap. Moving any of these into v1.0 needs a roadmap update. Nothing here starts until the gate noted for its milestone passes.

### Milestone v2.0 Customer Platform (spec-v1, months 4-9; gate: day-90 passes)

- **PROOF-01**: Bounded Kani proofs for critical functions with stated bounds in the equivalence report (outcome of SYM-03 decides the approach).
- **UBMGR-01**: UB policy manager with customer sign-off workflow beyond the error default.
- **REVIEW-01**: Review workspace: side-by-side C and Rust, mapped functions, divergence explorer with counterexamples, unsafe ledger and recorded named-reviewer approvals; delivery as reviewable pull requests.
- **CI-01**: Continuous equivalence in customer CI (GitHub Actions, GitLab, Jenkins) that blocks merges on divergence, under 30 minutes per module (proposed).
- **RES-01**: On-target resource gates for speed, code size, stack and RAM on STM32 and nRF52 boards or an emulator, default no more than 10% regression (proposed).
- **LOCAL-01**: On-prem deployment with locally hosted open-weight models, including a translation quality and cost evaluation.
- **DATA-01**: Data-commitment tooling: no training on customer code, retention and deletion confirmed in writing, encryption in transit and at rest, secrets scanned and stripped.
- **ADMIN-01**: SSO (SAML or OIDC), role-based access, and an audit log of every action by people and agents.
- **SUPPLY-01**: Signed releases of the tool and its own SBOM.
- **SBOM-01**: SBOM (SPDX or CycloneDX) for every Rust crate and toolchain component added, CRA-ready evidence formatting, and licence review of added components.
- **FERRO-01**: Ferrocene builds, an output profile restricted to the certified `core` subset, and a pinned MSRV matching the customer's Ferrocene `rustc`.
- **IAR-01**: IAR and Keil project-file parsing and linking Rust into those builds.
- **ASSESS-02**: Scoping effort and price estimates within plus or minus 30% (proposed) of actual effort, calibrated on pilot actuals.
- **ABSTR-01**: Abstraction functions for changed data structures, drafted by AI and reviewed by a human.
- **RULES-01**: Compiler-extension rule library (packed structs, section placement, attributes) mapping each to Rust or keeping it in C.

### Milestone v3.0 Hardware and Scale (spec-v2, months 10-18; gate: 3 paying customers and first subscription renewal)

- **DRV-01**: Hardware-facing code via device models: register read and write traces in Renode, svd2rust register crates, explicit volatile access.
- **CONC-01**: Interrupts and concurrency.
- **ARCH-01**: RISC-V and further target architectures.
- **AIRGAP-01**: Air-gapped hardening: offline licensing and signed update bundles.
- **QUAL-01**: Tool-qualification kit.
- **PLAT-01**: Platform license packaging for teams running their own migrations.
- **CERT-01**: SOC 2 Type II and ISO 27001.

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| C++ migration | Spec non-goal; C only |
| Whole-firmware rewrites | Product migrates one module at a time behind the existing C interface |
| Interrupt handlers and drivers (MVP) | Need device models; deferred to v3.0 |
| Worst-case timing guarantees | Only if contracted separately; real-time paths are flagged |
| Certifying products or claiming compliance | Customer's assessor decides; EVID-04 tests the tool never claims it |
| General-purpose coding-assistant features | Positioning is evidence, not assistance |
| Python in the shipped tool | ADR-0001 (LOCKED); enforced by TOOL-01 |
| Kani bounded proofs as MVP evidence | Spec-v1 scope and R2 unproven; deferred to v2.0 |
| Customer interviews, pilots, LOIs, pricing | Business activities tracked in PROJECT.md Context; only the pilots-or-LOIs count enters BENCH-04 as a manual input |
| Effort and price figures in the scoping report | Accuracy target needs pilot actuals; ASSESS-02 deferred |

## Traceability

Which phases cover which requirements.

| Requirement | Phase | Status |
|-------------|-------|--------|
| TOOL-01 | Phase 1 | Complete |
| TOOL-02 | Phase 1 | Complete |
| TOOL-03 | Phase 1 | Complete |
| TOOL-04 | Phase 1 | Complete |
| BED-01 | Phase 2 | Pending |
| BED-02 | Phase 2 | Pending |
| BUILD-01 | Phase 2 | Pending |
| BUILD-02 | Phase 2 | Pending |
| BUILD-03 | Phase 2 | Pending |
| EQV-01 | Phase 3 | Pending |
| EQV-02 | Phase 3 | Pending |
| EQV-03 | Phase 3 | Pending |
| EQV-04 | Phase 3 | Pending |
| UB-01 | Phase 4 | Pending |
| UB-02 | Phase 4 | Pending |
| UB-03 | Phase 4 | Pending |
| UB-04 | Phase 4 | Pending |
| UB-05 | Phase 4 | Pending |
| DIFF-01 | Phase 5 | Pending |
| DIFF-02 | Phase 5 | Pending |
| DIFF-03 | Phase 5 | Pending |
| DIFF-04 | Phase 5 | Pending |
| DIFF-05 | Phase 5 | Pending |
| DIFF-06 | Phase 5 | Pending |
| DIFF-07 | Phase 5 | Pending |
| SYM-01 | Phase 6 | Pending |
| SYM-02 | Phase 6 | Pending |
| SYM-03 | Phase 6 | Pending |
| SELF-01 | Phase 7 | Pending |
| SELF-02 | Phase 7 | Pending |
| SELF-03 | Phase 7 | Pending |
| GATE-01 | Phase 7 | Pending |
| GATE-02 | Phase 7 | Pending |
| GATE-03 | Phase 7 | Pending |
| GATE-05 | Phase 7 | Pending |
| GATE-06 | Phase 7 | Pending |
| FRONT-01 | Phase 8 | Pending |
| FRONT-02 | Phase 8 | Pending |
| FRONT-03 | Phase 8 | Pending |
| FRONT-04 | Phase 8 | Pending |
| FRONT-05 | Phase 8 | Pending |
| GATE-04 | Phase 8 | Pending |
| PROV-01 | Phase 9 | Pending |
| XLATE-01 | Phase 9 | Pending |
| XLATE-02 | Phase 9 | Pending |
| XLATE-03 | Phase 9 | Pending |
| XLATE-04 | Phase 9 | Pending |
| XLATE-05 | Phase 9 | Pending |
| XLATE-06 | Phase 9 | Pending |
| BOUND-01 | Phase 10 | Pending |
| BOUND-02 | Phase 10 | Pending |
| BOUND-03 | Phase 10 | Pending |
| BOUND-04 | Phase 10 | Pending |
| EVID-01 | Phase 11 | Pending |
| EVID-02 | Phase 11 | Pending |
| EVID-03 | Phase 11 | Pending |
| EVID-04 | Phase 11 | Pending |
| EVID-05 | Phase 11 | Pending |
| ASSESS-01 | Phase 11 | Pending |
| BENCH-01 | Phase 12 | Pending |
| BENCH-02 | Phase 12 | Pending |
| BENCH-03 | Phase 12 | Pending |
| BENCH-04 | Phase 12 | Pending |

**Coverage:**
- v1 requirements: 63 total
- Mapped to phases: 63
- Unmapped: 0

## Source Map

How the 27 synthesized intel requirements (`.planning/intel/requirements.md`) land in this file. Items marked "deferred" are listed under v2 Requirements (Deferred).

| Intel requirement | Landed as |
|-------------------|-----------|
| `REQ-ingest-build-capture` | BED-01, BED-02, BUILD-01, BUILD-02, BUILD-03 (GCC only; IAR and Keil deferred) |
| `REQ-assess` | ASSESS-01 (price and effort accuracy deferred) |
| `REQ-translate` | FRONT-01 to FRONT-05, XLATE-01 to XLATE-06, GATE-01 to GATE-04, GATE-06 |
| `REQ-validate` | DIFF-01 to DIFF-07, SYM-01, SYM-02 (Kani proofs deferred) |
| `REQ-ub-policy-manager` | UB-01 to UB-05 (manager UI and sign-off workflow deferred) |
| `REQ-boundary-integration` | BOUND-01 to BOUND-04 |
| `REQ-resource-gates` | Deferred; informational numbers only in EVID-01 and BENCH-02 |
| `REQ-review-workspace` | Deferred |
| `REQ-continuous-equivalence` | Deferred |
| `REQ-evidence-packager` | EVID-01 to EVID-05 (SBOM deferred) |
| `REQ-admin-security` | Deferred |
| `REQ-equivalence-contract` | EQV-01, EQV-02, UB-01 to UB-05, DIFF-03 |
| `REQ-harness-self-validation` | SELF-01 to SELF-03 |
| `REQ-anti-gaming` | GATE-02, GATE-04, GATE-05, XLATE-03 |
| `REQ-two-stage-translation` | XLATE-05, XLATE-06 (abstraction functions deferred) |
| `REQ-embedded-build-handling` | BED-02, BUILD-01, BUILD-02 (rule library, IAR/Keil, drivers deferred) |
| `REQ-model-agnostic-provider` | PROV-01, XLATE-04 (local-model evaluation deferred) |
| `REQ-data-commitments` | Deferred |
| `REQ-compliance-support` | Deferred |
| `REQ-public-benchmark` | BENCH-01 to BENCH-03 |
| `REQ-day90-gate` | BENCH-04 |
| `REQ-llvm-version-pin` | TOOL-02, SYM-01, SYM-02 |
| `REQ-spike-kani-c2rust-reference` | SYM-03 |
| `REQ-hayroll-licence-adapter` | FRONT-01, FRONT-03 |
| `REQ-ferrocene-core-profile` | Deferred (Ferrocene output profile) |
| `REQ-target-matched-harness` | DIFF-02, DIFF-06 |
| `REQ-sanitizer-variants` | UB-01 |

---
*Requirements defined: 2026-10-07*
*Last updated: 2026-10-07 after initial definition*
