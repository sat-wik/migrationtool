## Conflict Detection Report

### BLOCKERS (0)

### WARNINGS (0)

### INFO (10)

[INFO] Cross-reference graph is acyclic (previous cycle blockers cleared)
  Note: Classification cross_refs now contain only two edges: /home/claude/migrationtool/docs/research/VERIFICATION.md -> docs/specs/product-spec.md and VERIFICATION.md -> docs/research/literature-map.md. product-spec.md, literature-map.md and ADR-0001 declare no cross_refs. Three-color DFS finds no cycle; maximum depth 1 (cap is 50). The two earlier 2-cycle blockers (VERIFICATION <-> product-spec, VERIFICATION <-> literature-map) no longer apply and were not carried forward. A text search of /home/claude/migrationtool/docs confirms neither source document references VERIFICATION.md.

[INFO] Auto-resolved: VERIFICATION (precedence 1) over product-spec (precedence 2) on "KLEE on both sides"
  Note: /home/claude/migrationtool/docs/specs/product-spec.md (Technical approach problem 2; module 4) assumes per-function differential symbolic execution with KLEE on the C and Rust sides. /home/claude/migrationtool/docs/research/VERIFICATION.md (R1) states KLEE 3.2 supports LLVM 16 fully and 17-19 partially, and current stable rustc ships a newer LLVM. The fact from VERIFICATION wins; the spec's plan is retained in requirements.md but conditioned by REQ-llvm-version-pin (pin LLVM 16-19, spike Rust-to-KLEE before promising per-function symbolic equivalence). No decision is overridden.

[INFO] Auto-resolved: VERIFICATION over product-spec on "c2rust output as the reference" for Kani
  Note: /home/claude/migrationtool/docs/specs/product-spec.md (Technical approach problem 2) presents c2rust output as a Kani reference. /home/claude/migrationtool/docs/research/VERIFICATION.md (R2) calls it plausible but unproven and requires a go/no-go spike with libc stubs, keeping fuzzing plus KLEE as primary evidence until it passes. Carried as REQ-spike-kani-c2rust-reference. Spike pass criteria are absent from the sources.

[INFO] Auto-resolved: VERIFICATION over product-spec on sanitizer replay and harness host
  Note: /home/claude/migrationtool/docs/specs/product-spec.md (Technical approach problem 1) replays inputs against "a C build with AddressSanitizer, UndefinedBehaviorSanitizer and MemorySanitizer" and does not state the harness host ABI. /home/claude/migrationtool/docs/research/VERIFICATION.md R6 says MSan cannot share a build with ASan, and R5 says the differential harness must run on an ILP32 target-matched configuration. Carried as REQ-sanitizer-variants and REQ-target-matched-harness. R3 (Hayroll licence adapter) is additive because the spec is silent on it; R4 (Ferrocene core profile) is already noted inline in the spec.

[INFO] Corrections already applied in source documents
  Note: CRA SBOM wording and the Ferrocene certified-core subset are marked [verified-fix] in /home/claude/migrationtool/docs/specs/product-spec.md; TRACTOR lib_state_in/lib_state_out is marked corrected in /home/claude/migrationtool/docs/research/literature-map.md. Synthesized intel uses the VERIFICATION.md versions (/home/claude/migrationtool/docs/research/VERIFICATION.md). NSA/CISA guidance and several literature-map figures were not re-checked by VERIFICATION and are recorded as such in context.md.

[INFO] Auto-resolved: product-spec (precedence 2) over literature-map (precedence 3) on roadmap timing
  Note: /home/claude/migrationtool/docs/research/literature-map.md section (d) places the layered equivalence engine (KLEE, Kani/CBMC) in months 2-5, public benchmarking in months 3-6, and the embedded parser wedge in months 4-9. /home/claude/migrationtool/docs/specs/product-spec.md starts the Cortex-M parser beachhead in week 1, benchmarks in weeks 6-10, and puts differential fuzzing + KLEE in the MVP (months 0-3) and Kani in v1 (months 4-9). The spec timeline is used in requirements.md and constraints.md; the literature-map roadmap is kept in context.md for reference only.

[INFO] Phasing note: bounded-proof acceptance criterion versus MVP scope in product-spec
  Note: /home/claude/migrationtool/docs/specs/product-spec.md module 4 "Done when" includes "every critical function has a bounded proof", while its Roadmap lists only "differential fuzzing + KLEE" in the MVP and "Kani bounded proofs" in v1. The same spec resolves the ordering through its phase list, so REQ-validate is tagged MVP (fuzzing + KLEE) and v1 (Kani proofs). VERIFICATION R2 further makes the Kani path a go/no-go spike. The spec's own definition of "full evidence" for the day-90 gate is not stated; roadmapper should confirm whether MVP "full evidence" excludes bounded proofs.

[INFO] Numbering note: pipeline steps versus module numbers in product-spec
  Note: /home/claude/migrationtool/docs/specs/product-spec.md lists eleven modules, ten forming one pipeline. Pipeline step 9 (Evidence bundle) corresponds to module 10 (Evidence packager) and pipeline step 10 (Continuous equivalence) corresponds to module 9; module 11 (Admin and security) is outside the pipeline. Human steps are pipeline steps 5 and 8 (modules 5 and 8). Requirements are keyed by module name, not number, to avoid ambiguity.

[INFO] No ADR-versus-SPEC contradiction detected
  Note: /home/claude/migrationtool/docs/adr/0001-tool-implementation-language.md (LOCKED) is consistent with /home/claude/migrationtool/docs/specs/product-spec.md on on-prem/air-gapped deployment, model-agnostic pipeline, reproducibility bundle and signed releases. Its subprocess-with-pinned-toolchain decision also matches the adapter isolation recommended for Hayroll (R3) and the LLVM pin (R1). Only one ADR exists, so no LOCKED-vs-LOCKED check applies. No UNKNOWN or low-confidence classifications; no competing PRD variants.

[INFO] Typing note: no PRD in ingest set
  Note: Product requirements were extracted from the SPEC (explicit "Done when" criteria) and from VERIFICATION risks R1-R6 into requirements.md, with SPEC constraints in constraints.md. Roadmapper should treat requirements.md Part A and B as spec-derived.
