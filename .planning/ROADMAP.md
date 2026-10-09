# Roadmap: migrationtool

## Milestones

- 🚧 **v1.0 MVP** - Phases 1-12 (in progress; spec months 0-3, ends at the Day-90 gate)
- 📋 **v2.0 Customer Platform** - Not yet phased (spec-v1, months 4-9); starts only after the Day-90 gate passes
- 📋 **v3.0 Hardware and Scale** - Not yet phased (spec-v2, months 10-18); starts only after the spec-v1 gate (3 paying customers, first renewal)

## Overview

The MVP builds the trust machinery first and the translator second. Phases 1-2 give a pinned container and a test bed of real Cortex-M parsers whose builds replay exactly. Phases 3-5 define "equivalent" (a refinement contract and a TRACTOR-compatible vector schema), label undefined behavior mechanically, and build a differential fuzzing harness that runs on an ILP32, unsigned-char configuration matching the target. Phase 6 settles by spike whether KLEE and Kani can join the evidence layer. Phase 7 proves the harness catches 20 planted bugs and cannot be gamed. Only then do Phases 8-9 bring in c2rust, Hayroll (after a licence check) and a model-agnostic agent loop that translates function by function behind deterministic gates. Phases 10-12 integrate the Rust behind the original C interface, package evidence and a scoping report, and run the public benchmark that produces the technical half of the Day-90 gate.

Deterministic anti-gaming gates (stubs, `unsafe` boundary, held-out corpus) are placed in Phase 7, before the translator exists, because they are harness-class checks. The agent loop in Phase 9 consumes them.

Phase sizing: each phase is meant to be finishable in 1-2 weeks of part-time work. Twelve phases is above the "standard" band (4-6) because that sizing rule dominates; see PROJECT.md assumptions A-09 and A-10.

**Day-90 gate (technical half, computed in Phase 12):** harness catches 100% of planted bugs; at least 70% of test-bed parsers fully evidenced (definition in PROJECT.md, assumption A-01); and at least 2 paid pilots or LOIs, which is a business gate entered by hand, not a code deliverable. Customer interviews and pilots run as a founder-run parallel track described in PROJECT.md Context.

**Cut line if the schedule slips.** None of these feeds a Day-90 gate criterion; defer them to v2.0 through a roadmap edit, in this order: (1) stage 2 idiomatic refinement, XLATE-06, the last plans of Phase 9; (2) the R2 Kani spike, SYM-03, in Phase 6; (3) the scoping report, ASSESS-01, in Phase 11. Phases 1-8, XLATE-05, EVID-02 and BENCH-04 are not cuttable.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [ ] **Phase 1: Pinned Toolchain Container** - Reproducible container, one LLVM pin in 16-19, subprocess runner, Rust workspace skeleton
- [ ] **Phase 2: Test Bed and Build Capture** - 10-20 pinned C parsers whose real builds are captured and replayed exactly per configuration
- [ ] **Phase 3: Equivalence Contract and Test-Vector Schema** - Refinement contract, TRACTOR-compatible vectors, deterministic C reference executor
- [ ] **Phase 4: UB Detection and Decision Log** - Separate sanitizer builds, -O0 vs real-flags differential, symbolic C scan, UB decisions log
- [ ] **Phase 5: Target-Matched Differential Harness** - C vs Rust differential fuzzing on ILP32 unsigned-char, C-side coverage, emulated-target replay
- [ ] **Phase 6: Symbolic Layer and Analysis Spikes** - R1 Rust-to-KLEE go/no-go and integration, R2 Kani-with-c2rust-reference time-boxed spike
- [ ] **Phase 7: Harness Self-Validation and Anti-Gaming Gates** - 20 planted bugs, mutation kill rate, stub and unsafe gates, held-out corpus
- [ ] **Phase 8: Translator Front End** - Hayroll licence gate, c2rust and Hayroll adapters, dependency order, function map
- [ ] **Phase 9: Agentic Two-Stage Translation** - Model-agnostic agent loop, dependency-ordered gated translation, faithful then idiomatic
- [ ] **Phase 10: C-ABI Boundary and Integration** - Header and shim, Cargo crate, Make/CMake hook, existing tests pass with Rust swapped in
- [ ] **Phase 11: Evidence Bundle and Scoping Report v0** - Versioned re-runnable evidence bundle, full-evidence predicate, scoping report
- [ ] **Phase 12: Public Benchmark and Day-90 Gate** - TRACTOR public tests, test-bed run, published report, Day-90 gate report

## Phase Details

### Phase 1: Pinned Toolchain Container

**Goal**: Every compiler and analyser the harness and translator need runs as a pinned subprocess inside one reproducible container, on a single pinned LLVM.
**Depends on**: Nothing (first phase)
**Requirements**: TOOL-01, TOOL-02, TOOL-03, TOOL-04
**Success Criteria** (what must be TRUE):
  1. One command run inside the container prints the exact version of every pinned tool (clang/LLVM, KLEE, c2rust, Bear, arm-none-eabi GCC, QEMU, cargo-mutants, and the pinned rustc with the LLVM version from `rustc -vV`) and exits non-zero if any tool is missing or off its pin.
  2. One LLVM major in 16-19 is recorded as the pin for clang, KLEE and c2rust, the version check fails if any of them reports a different major, and the pinned rustc reports an LLVM at or below 19 (recorded as provisional until the Phase 6 R1 spike).
  3. Building the container image twice from the same inputs produces identical tool-version manifests, and every base image and tool source is pinned by digest, commit or checksum with no floating tags.
  4. Inside the container a trivial `no_std` crate builds for `thumbv7em-none-eabihf`, the workspace launches an analyser through the subprocess runner with stdout, stderr, exit status and version stamp captured, and a repository check fails on any Python source outside `research/`.

**Plans:** 8/9 plans executed (01-09 is gap closure)

Plans:
**Wave 1**
- [x] 01-01-PLAN.md — Tracer: `mt toolchain check` end to end through the subprocess runner; runner contract hardening (wave 1)
- [x] 01-02-PLAN.md — Founder gates: c2rust package legitimacy and CI push mode (wave 1, checkpoints)

**Wave 2** *(blocked on Wave 1 completion)*
- [x] 01-03-PLAN.md — pins.toml schema v1, validation, version/LLVM parsers, `mt toolchain build-args` (wave 2)
- [x] 01-04-PLAN.md — Repository guards (no Python, process confinement, forbid unsafe, no analyser bindings), cargo-deny, no_std smoke crate (wave 2)

**Wave 3** *(blocked on Wave 2 completion)*
- [x] 01-05-PLAN.md — Full check rules (single LLVM major, bitcode rustc bound), manifest and manifest-diff, `mt toolchain hash` (wave 3)

**Wave 4** *(blocked on Wave 3 completion)*
- [x] 01-06-PLAN.md — Real container/pins.toml, PROJECT D-15/D-16 and decision gate, Dockerfile base stage, ci and pin-discovery workflows, discovery round trip (wave 4)

**Wave 5** *(blocked on Wave 4 completion)*
- [x] 01-07-PLAN.md — Full Dockerfile, container workflow (live check, off-pin check, double build and manifest diff, smoke), CI to green (wave 5)

**Wave 6** *(blocked on Wave 5 completion)*
- [x] 01-08-PLAN.md — GHCR publish by digest, container README, recorded container reference (wave 6)

**Wave 7** *(gap closure; blocked on Wave 6 completion)*
- [ ] 01-09-PLAN.md — CR-01 / TOOL-03: Rust toolchains installed by rustup only from a file:// mirror of the sha256-verified channel manifests, c2rust and cargo-mutants installed from the verified .crate with --path, static rules plus an image-a pin-binding gate, CI to green, README digest refreshed (wave 7)

### Phase 2: Test Bed and Build Capture

**Goal**: A pinned test bed of 10-20 real C parsers can be rebuilt exactly, per shipped configuration, for Cortex-M with GCC.
**Depends on**: Phase 1
**Requirements**: BED-01, BED-02, BUILD-01, BUILD-02, BUILD-03
**Success Criteria** (what must be TRUE):
  1. A test-bed manifest lists 10-20 open-source C parser or protocol targets (for example zcbor, tinycbor, nanopb, lwIP parsers, Mbed TLS ASN.1), each pinned by commit with its licence recorded and at least one buffer-in entry function identified.
  2. Each target declares its shipped configurations, and the first-RTOS choice (Zephyr or FreeRTOS) is recorded as a decision, even if recorded as deferred because parsers are RTOS-independent.
  3. For every target and configuration the capture command writes a `compile_commands.json` covering every translation unit, and replaying it in the container reproduces the original build output (byte-identical objects, or identical after a documented normalisation); targets that cannot be reproduced are dropped with the cause logged, leaving at least 10.
  4. Each target has a seed corpus extracted from its own tests and fixtures, or at least 10 hand-made seeds (proposed) when it has none.

**Plans**: TBD

### Phase 3: Equivalence Contract and Test-Vector Schema

**Goal**: "Equivalent" is written down and machine-checkable, and the original C can be run against any test vector to produce a deterministic reference result.
**Depends on**: Phase 2
**Requirements**: EQV-01, EQV-02, EQV-03, EQV-04
**Success Criteria** (what must be TRUE):
  1. Contract v0 exists as a versioned document and a policy file the harness loads: refinement (Rust equals C on every defined-behavior input), the observation point at the C interface, and the UB action menu {return error, panic, preserve} with "return error" as the default and a recorded setting required to change it.
  2. A vector schema with exactly `argv, stdin, env, stdout, stderr, rc, lib_state_in, lib_state_out, has_ub` validates vectors and round-trips in Rust types; a vector with a single merged `lib_state` is rejected; at least 3 TRACTOR public-corpus vectors load unchanged (corpus used locally only, licence checked in Phase 12 before any redistribution).
  3. At least 10 test-bed targets have an entry-point adapter declaring the function and its argument roles, so bytes in produce return code, output buffer and library state out as a vector.
  4. The C reference executor runs any vector against the target's original C build and stores the result in a schema-valid vector; running the same vector twice gives byte-identical results.

**Plans**: TBD

### Phase 4: UB Detection and Decision Log

**Goal**: Every input and code site where the original C has undefined or implementation-defined behavior is detected, labelled and recorded with a decision, defaulting to "return an error".
**Depends on**: Phase 3
**Requirements**: UB-01, UB-02, UB-03, UB-04, UB-05
**Success Criteria** (what must be TRUE):
  1. The tool builds separate ASan+UBSan and MSan variants of the C and replays the corpus against each; on a set of known-UB C samples (signed overflow, out-of-bounds read, uninitialised read, oversized shift) every sample's triggering input is flagged `has_ub`. Whether MSan can run under the ILP32 configuration is confirmed or its LP64 fallback is documented (PROJECT.md assumption A-08).
  2. Every input is also replayed against the C built at `-O0` and at the shipped-configuration flags, and a known optimiser-sensitive sample is tagged as suspected UB.
  3. A KLEE scan (C side, pinned LLVM) or Frama-C scan of the parser entry functions of at least 3 test-bed targets lists UB sites the corpus never reached, with per-function completion or timeout recorded, and merges them into the UB log tagged with their provenance.
  4. Plain `char` signedness, integer and pointer sizes and the layout of every struct crossing the interface are extracted from the target configuration (not assumed) and logged for sign-off.
  5. The UB decisions log has site or input class, decision, rationale, approver and date for every detected entry, pre-filled with "return error"; the tool refuses to mark a module UB-complete while any entry lacks a decision.

**Plans**: TBD

### Phase 5: Target-Matched Differential Harness

**Goal**: The original C and a candidate Rust module, linked behind the same C interface, are compared on every fuzz and test input under an ILP32, unsigned-char configuration that matches the Cortex-M target.
**Depends on**: Phase 4
**Requirements**: DIFF-01, DIFF-02, DIFF-03, DIFF-04, DIFF-05, DIFF-06, DIFF-07
**Success Criteria** (what must be TRUE):
  1. One command runs differential fuzzing for a C and Rust pair behind the same C interface: a known-equivalent hand-written pair (at least 3 test-bed functions) runs without divergence, and a known-bad pair reports a divergence within a bounded time (proposed: 5 minutes) with the minimized counterexample saved as a schema-valid vector.
  2. The harness runs under an ILP32 configuration (default i686 `-m32`, `-funsigned-char`, shipped-configuration flags), and a probe pair whose behavior depends on `char` signedness and `long` width diverges between a default LP64 signed-`char` run and matches in the target-matched run.
  3. Inputs tagged `has_ub` are checked against the UB policy (Rust returns the error code) instead of the C output, and a Rust function that crashes or corrupts memory on such an input is reported as a divergence.
  4. Branch coverage of the original C (not the Rust) is reported per function and module against the 90% threshold (proposed), with uncovered branches listed.
  5. Fuzzing is seeded from the test-bed seeds and per-target dictionaries with protocol-aware mutators for at least CBOR and TLV-style formats, and the final corpus replays on an emulated Cortex-M target (Renode preferred; QEMU as a recorded fallback) with the same verdicts as the host run, or the mismatches are recorded as findings.

**Plans**: TBD

### Phase 6: Symbolic Layer and Analysis Spikes

**Goal**: Decide with evidence whether per-function symbolic equivalence (KLEE on both sides) and Kani with c2rust output as reference are viable, and wire up whatever passes, without letting either block the day-90 path.
**Depends on**: Phase 5
**Requirements**: SYM-01, SYM-02, SYM-03
**Success Criteria** (what must be TRUE):
  1. R1 spike verdict (Rust-to-KLEE) is recorded as GO or NO-GO with evidence. Proposed GO criteria, all required: (a) the pinned rustc (LLVM at or below 19 per `rustc -vV`) emits bitcode for `no_std`, `panic=abort` Rust that the pinned KLEE loads without aborting, for at least 5 function pairs across at least 2 test-bed parsers; (b) on each known-equivalent pair KLEE completes within the bound (proposed: at most 32 symbolic bytes, at most 10 minutes) with zero assertion failures; (c) on at least 3 planted-bug pairs of different classes (off-by-one, wrong comparison, missing bounds check) KLEE emits a counterexample that the Phase 5 harness reproduces as a real divergence; (d) two clean container runs give identical verdicts. NO-GO if these are not met within a 1-week part-time time box (proposed); the LLVM pin is then revisited or a different Rust-side engine is logged as a v2.0 item.
  2. If R1 is GO, a per-function symbolic layer runs on at least 5 test-bed functions and reports equivalent-within-bound, divergent-with-counterexample, or timeout (shown as "not proven", never as a pass) in the same result schema as fuzzing. If NO-GO, the schema reports "symbolic: unavailable" and the full-evidence definition drops the KLEE term (PROJECT.md D-14).
  3. R2 spike verdict (c2rust output as Kani reference) is recorded as GO, NO-GO or "deferred to v2.0" within a 1-week part-time time box (proposed). Proposed GO criteria, all required: Kani loads c2rust output for at least 2 toy C functions including one that calls libc through `extern "C"` using hand-written stubs; proves equivalence to a clean Rust version for inputs of at most 16 bytes within 5 minutes; and returns a counterexample on a planted-bug pair. A missed time box defers it to v2.0 and does not block Phase 7.
  4. Kani's known blind spots (aliasing violations, unaligned dereference, concurrency, inline assembly) appear in the evidence caveat text regardless of the R2 verdict.

**Plans**: TBD

### Phase 7: Harness Self-Validation and Anti-Gaming Gates

**Goal**: Prove the harness catches what it claims and that translated output cannot pass by cheating, before any translator exists.
**Depends on**: Phase 5, Phase 6
**Requirements**: SELF-01, SELF-02, SELF-03, GATE-01, GATE-02, GATE-03, GATE-05, GATE-06
**Success Criteria** (what must be TRUE):
  1. 20 known bugs planted in hand-written Rust for at least 5 test-bed functions, spanning at least 6 bug classes (for example off-by-one, wrong comparison or sign, missing bounds check, integer width or wrap, `char` signedness, skipped error path, stale state), are all caught by the harness (20 of 20), and the report names which layer caught each. This is a Day-90 gate input.
  2. cargo-mutants runs over the hand-written Rust and the kill rate is published with each surviving mutant classified as an equivalent mutant or a harness gap, and every gap becomes a fix or a logged limitation.
  3. A gate suite rejects planted violations with specific messages: `todo!()`, `unimplemented!()`, an empty non-unit body, `unsafe` outside the boundary module, an `unsafe` block without a SAFETY justification, use of `alloc`, a crate that fails the `thumbv7em-none-eabihf` build, a Clippy warning, a missing rustfmt pass; a clean crate passes.
  4. The held-out corpus is stored outside the agent-visible workspace, is used only for final verdicts, and a negative test shows a sandboxed process cannot read it.
  5. The planted-bug suite and the gate-violation suite run from one command and fail the build if any bug or violation is missed.

**Plans**: TBD

### Phase 8: Translator Front End

**Goal**: Given a captured build, produce a dependency-ordered translation work plan: unsafe-Rust skeletons where c2rust works, macros through Hayroll only if licensed, a function map, and a gate that every C function is mapped and exercised.
**Depends on**: Phase 2, Phase 5, Phase 7
**Requirements**: FRONT-01, FRONT-02, FRONT-03, FRONT-04, FRONT-05, GATE-04
**Success Criteria** (what must be TRUE):
  1. R3 Hayroll licence is resolved before any code depends on it. Proposed PASS: a licence file at a recorded commit that permits our use, or written permission from the authors, saved under `docs/licences/` with the date. Proposed FAIL: neither exists, in which case the Hayroll adapter stays disabled, shipped configurations rely on preprocessed c2rust input, and the decision is recorded.
  2. A front-end trait has swappable implementations selected by configuration (c2rust, Hayroll when PASS, and a no-skeleton path where agents translate from the C alone), all passing one shared contract-test suite; swapping needs no change in callers.
  3. Running the c2rust adapter over every retained test-bed target produces a per-translation-unit success and failure table from the Phase 2 `compile_commands.json`; a failing unit is reported and the module proceeds on the no-skeleton path instead of aborting.
  4. For each module the tool emits a machine-readable translation order from the clang call graph (callees before callers, cycles grouped), and a function map from C names to Rust symbols that is checked for completeness against clang's function list.
  5. The mapped-and-exercised gate rejects a candidate that omits a mapped counterpart or leaves any C function unexercised by the corpus (verified with a planted omission), using C-side coverage from Phase 5.

**Plans**: TBD

### Phase 9: Agentic Two-Stage Translation

**Goal**: LLM agents translate function by function in dependency order with every step gated, first to faithful Rust and then to idiomatic Rust, and correctness never depends on which model did the work.
**Depends on**: Phase 7, Phase 8
**Requirements**: PROV-01, XLATE-01, XLATE-02, XLATE-03, XLATE-04, XLATE-05, XLATE-06
**Success Criteria** (what must be TRUE):
  1. A hosted model and a locally hosted open-weight model (OpenAI-compatible endpoint) both drive the same loop by changing configuration only, and a scripted replay provider makes loop runs deterministic in tests.
  2. An adversarial provider that returns stubs or subtly wrong code cannot produce an accepted module: the gates and harness reject it, and swapping in a weaker model changes only retry count and cost.
  3. Stage 1: for at least 5 test-bed targets the loop translates in dependency order, runs the compile check, all gates and the differential harness after every function, and produces faithful Rust that passes everything; functions that exhaust their retry budget are reported as stay-in-C candidates and never stubbed; the translation success rate is reported as measured.
  4. Agent runs execute in a sandbox with no access to the held-out corpus, and every prompt, response and gate verdict is logged for audit.
  5. Stage 2 (cut-line item): for at least 3 stage-1 modules the idiomatic refactor runs in small Rust-to-Rust steps, each checked against the previous step and the C at the C interface, failed steps are reverted, and each module is labelled faithful or idiomatic.

**Plans**: TBD

### Phase 10: C-ABI Boundary and Integration

**Goal**: A translated module drops into the original C build behind the same C interface, and the existing build and tests pass with the Rust swapped in.
**Depends on**: Phase 9
**Requirements**: BOUND-01, BOUND-02, BOUND-03, BOUND-04
**Success Criteria** (what must be TRUE):
  1. For a translated module the tool generates a C header and interface shim whose exported symbols and signatures match the original header, verified by compiling a C unit against the original header and linking it with the Rust static library with an empty symbol and signature diff.
  2. The tool generates a Cargo crate (static library, `no_std`, `panic=abort`, pinned toolchain file) and Make and CMake hooks that swap one module for the Rust library while the other modules remain C.
  3. For at least 5 test-bed targets, the original full build and existing test suite pass with the Rust module swapped in, for every shipped configuration of that target.
  4. A defined panic handler is in place and a test shows a `has_ub` input returns the policy error through the C interface without relying on unwinding.

**Plans**: TBD

### Phase 11: Evidence Bundle and Scoping Report v0

**Goal**: Each module leaves with a versioned, re-runnable evidence bundle that states exactly what was and was not shown, and a scoping report can be produced for a C code base.
**Depends on**: Phase 4, Phase 5, Phase 6, Phase 7, Phase 9, Phase 10
**Requirements**: EVID-01, EVID-02, EVID-03, EVID-04, EVID-05, ASSESS-01
**Success Criteria** (what must be TRUE):
  1. One command produces a versioned per-module bundle whose manifest lists the equivalence report, UB decisions log, unsafe ledger, security delta report, test and fuzz assets, traceability matrix, reproducibility bundle and informational code-size and timing numbers, and states that the SBOM is absent by design in v1.0.
  2. The `full_evidence` predicate implements the five-term MVP definition and returns true or false with a reason per term for each module (PROJECT.md, assumption A-01).
  3. On a clean checkout in a fresh container, one command re-runs validation from the reproducibility bundle and reproduces the same verdicts for at least 3 modules.
  4. The equivalence report states per method what was and was not done (fuzzing hours, C-side coverage, symbolic results or "unavailable", Kani "not included", divergences and resolutions, tool versions), and a test shows the generator never emits a claim that a product is certified or compliant.
  5. Cut-line item: a scoping report v0 for a C code base outputs the dependency graph, a risk map (external-input entry points, size and complexity, UB pre-scan counts) and feasibility indicators with inline-assembly, `volatile`, hardware-access and compiler-extension users flagged as stay-in-C candidates, and contains no price or effort figure.

**Plans**: TBD

### Phase 12: Public Benchmark and Day-90 Gate

**Goal**: Publish reproducible numbers on the test bed and TRACTOR's public tests, and compute the technical half of the Day-90 gate.
**Depends on**: Phase 11
**Requirements**: BENCH-01, BENCH-02, BENCH-03, BENCH-04
**Success Criteria** (what must be TRUE):
  1. TRACTOR's public Test Runner and Cando run against the tool's output for the subset it can build, `has_ub` vectors are excluded from scoring as TRACTOR does, and the corpus licence is checked and noted before any artifact is redistributed.
  2. A full-pipeline run across the test bed yields a per-target table: fully evidenced yes or no with the failing term, divergences, remaining `unsafe` count, code size C vs Rust, informational timing, C-side coverage, and share of functions with symbolic results.
  3. A benchmark report is published with method and honest caveats (self-evaluated, test-bed limits), and a third party can reproduce it from the reproducibility bundle.
  4. The Day-90 gate report computes the planted-bug catch rate (must be 100%) and the share of test-bed parsers fully evidenced (must be at least 70%, proposed), takes the pilots-or-LOIs count (at least 2) as a manual business input, and states continue or narrow the niche; v2.0 work does not start unless the verdict is continue.

**Plans**: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9 → 10 → 11 → 12

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Pinned Toolchain Container | 8/9 | In Progress | - |
| 2. Test Bed and Build Capture | 0/0 | Not started | - |
| 3. Equivalence Contract and Test-Vector Schema | 0/0 | Not started | - |
| 4. UB Detection and Decision Log | 0/0 | Not started | - |
| 5. Target-Matched Differential Harness | 0/0 | Not started | - |
| 6. Symbolic Layer and Analysis Spikes | 0/0 | Not started | - |
| 7. Harness Self-Validation and Anti-Gaming Gates | 0/0 | Not started | - |
| 8. Translator Front End | 0/0 | Not started | - |
| 9. Agentic Two-Stage Translation | 0/0 | Not started | - |
| 10. C-ABI Boundary and Integration | 0/0 | Not started | - |
| 11. Evidence Bundle and Scoping Report v0 | 0/0 | Not started | - |
| 12. Public Benchmark and Day-90 Gate | 0/0 | Not started | - |
