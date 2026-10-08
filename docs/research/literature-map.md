---
type: DOC
title: "Validated C-to-Rust Migration: Literature Map, Hardest Problems, and the Human/AI Split (October 2026)"
source: Claude research report, produced 2026-10-07 from web sources
note: Research output, not a decision record. Figures are as reported by each source; some were re-checked on 2026-10-07.
---

# Validated C-to-Rust Migration: Literature Map, Hardest Problems, and the Human/AI Split (as of October 2026)

The hardest part of this product will not be getting an LLM to write Rust. The 2025–2026 agentic systems already compile medium-sized repositories almost every time. The hard part is defining and proving what "equivalent" means for C that has undefined behavior, weak or no tests, preprocessor variants and hardware side effects. That definition, the oracle and harness design built on it, and how you handle the unsafe/FFI boundary are what you must own personally. They are also where the moat is.

## TL;DR

- **Translation is becoming a commodity; validation is not.** ORBIT reports 100% compilation and 91.7% test success on 24 CRUST-Bench programs, mostly over 1,000 lines of code. But MIT Lincoln Lab's TRACTOR benchmark openly defines correctness as test-based I/O fidelity to the clang-compiled binary ("C means what LLVM does") and leaves UB test vectors out of automated scoring. An independent security study found automated tools carried over 177 original memory-safety bugs and introduced 77 new ones. The open, sellable problem is strong equivalence evidence plus a clear policy for UB.
- **Embedded/IoT is the least-studied and hardest wedge.** The only peer-reviewed measurement found (CCS 2024) reports that c2rust fails on 93.8% (15/16) of embedded codebases, because of compiler flags, non-standard C, and output that depends on `std`. No published LLM-based C-to-Rust evaluation on MMIO/interrupt/no_std code was found. That makes it an opportunity, but also a place where you will be building from scratch.
- **What you must do yourself:** define the equivalence contract and the UB policy; design the oracle and the differential/symbolic harness architecture; decide unsafe/FFI boundaries; own the hardware-semantics model; review security; build the evaluation methodology, including your own anti-gaming checks. LLMs can reliably do the bulk translation, compile-error repair, idiomatic refactoring and first-draft test and harness generation, but only behind deterministic checks, because agents report success they have not achieved.

## Key Findings

1. **Single-shot LLM translation is weak; scaffolded agentic pipelines are strong on benchmarks.**
   - CRUST-Bench (100 repositories, average 958 LOC; Khatry et al.): the best single-shot model, OpenAI o1, solved only 15% of tasks, and the top models reached 32–37% when using a repair loop.
   - By 2026, skeleton- and orchestration-based systems (ORBIT, EvoC2Rust, His2Trans, Rustine) report 90%+ compile and test rates on their chosen benchmarks.
2. **The tests are the weak link.** ORBIT found that vanilla coding agents "achieve build and test success while covering only 64.4% of functions and 3.0% of test cases." A pipeline that trusts agent self-reports or weak tests will ship broken code that looks finished.
3. **Static ownership inference has a known ceiling.**
   - "Aliasing Limits" (OOPSLA 2023) found only 12% of eligible raw-pointer declarations could be made safe after pseudo-safety; their improved encoding raised this to 21%.
   - Crown's median reduction is 37.3% of raw-pointer declarations.
   - In practice, the jump to safe Rust needs semantic re-design (often LLM-proposed). Rule-based lifting alone does not get there.
4. **Formal equivalence works only on small or restricted code.**
   - VERT raised bounded-model-checked equivalence from 1% to 42% on competitive-programming-scale programs.
   - Fromherz & Protzenko's formal C-to-safe-Rust compilation works for a restricted "Mini-C" subset (HACL* at 80K LOC, EverParse).
   - Neither is a general solution for arbitrary legacy C.
5. **Safety is not the same as security.** Reducing `unsafe` counts is a poor proxy. TRACTOR's evaluators saw a team write a safe internal function plus an unsafe wrapper that reimplemented all its logic and never called the safe function. Google's first near-miss Rust memory-safety bug (CVE-2025-48530, CrabbyAVIF) was in unsafe Rust.

## (a) Literature Map

### A1. Rule-based / static-analysis transpilers and idiom-specific lifters

| Work | Venue/Year | Link | Key result | Limitation |
|---|---|---|---|---|
| c2rust (Immunant) | Tool, 2018– | github.com/immunant/c2rust | Syntax-directed C→unsafe Rust; the de facto front-end for most hybrid systems (C2SaferRust, Crown, Hayroll, SACTOR) | Output is largely unsafe and "C-in-Rust". ORBIT measured 69.6% unsafe lines on CRUST-Bench programs. Expands macros, discarding configurability |
| Laertes, "Translating C to Safer Rust" (Emre et al.) | OOPSLA 2021 | doi.org/10.1145/3485498 | Lifts some raw pointers to references, using rustc's borrow checker as an oracle | Follow-up limit study found it applicable to only around 11% of pointers |
| "Aliasing Limits on Translating C to Safe Rust" (Emre et al.) | OOPSLA 2023 (PACMPL 7, OOPSLA1) | doi.org/10.1145/3586046 | Shows the borrow checker's imprecision is the binding constraint. Only 12% of eligible raw-pointer declarations made safe; their taint-analysis encoding raises this to 21% | Concludes "the vast majority of raw pointers cannot be automatically made into safe references" by existing techniques |
| Crown, "Ownership Guided C to Rust Translation" (Zhang, David, Yu, Wang) | CAV 2023 | arxiv.org/abs/2303.10515 | Whole-program ownership/mutability/fatness analysis. Median 37.3% reduction in raw-pointer declarations and 62.1% in uses; 100% on some data structures | Array pointers stay raw; only 21.4% on brotli. Used as an LLM hint in SACTOR (turning it off drops idiomatic successes 41→34) |
| Concrat (Hong & Ryu) | ICSE 2023 | doi.org/10.1109/ICSE48619.2023.00069 | Translates pthread lock APIs to Rust std locks by inferring which data each lock protects | Lock idioms only |
| Urcrat, "To Tag, or Not to Tag" (Hong & Ryu) | ASE 2024 | (ASE '24 proceedings) | C unions → Rust tagged enums via analysis | Union idioms only |
| Nopcrat, "Don't Write, but Return" (Hong & Ryu) | PLDI 2024 | (PACMPL 8, PLDI) | Output parameters → tuples/Option | Output-parameter idioms only |
| Forcrat (Hong & Ryu) | arXiv 2025 | arxiv.org/abs/2506.01427 | libc I/O API → Rust std I/O via origin/capability analysis. Notes GPT-4o-mini left 44% of functions uncompilable in prior work | I/O idioms only |
| GenC2Rust (Demsky et al.) | ICSE 2025 | (ICSE '25) | `void*` → Rust generics via typing constraints | Generic-pointer idioms only |
| "Compiling C to Safe Rust, Formalized" (Fromherz & Protzenko) and Scylla | arXiv 2412.15042; Scylla at OOPSLA 2026 | arxiv.org/abs/2412.15042 | Type-directed, formalized translation of "Mini-C" to safe Rust with "split trees" for pointer arithmetic. HACL* (80K LOC) needed minimal changes; EverParse CBOR parser (1.4K LOC) none. Output shipped in libcrux | Requires source to be in an applicative subset; user restructures the C first |
| Hayroll (Peng, Kasikci, Bernstein, Ernst) | PLDI 2026 | doi.org/10.1145/3808276 | Wraps c2rust to preserve macros as Rust functions/macros and conditional compilation as `#[cfg]`, using symbolic execution to derive per-line activation conditions. Evaluated on CRUST-Bench, LibmCS, zlib | Reconstructs "most" syntactic macros, not all; research prototype |
| &inator (Chen, Coughlin, Bond) | arXiv 2604.17261 (PACMPL) | arxiv.org/abs/2604.17261 | "Correct, precise" C-to-Rust interface (signature) translation | Interface-level only |

### A2. LLM-based and hybrid (neuro-symbolic / agentic) systems

| Work | Venue/Year | Link | Key result | Limitation |
|---|---|---|---|---|
| Flourine, "Towards Translating Real-World Code with LLMs" (Eniser et al., AWS) | arXiv 2405.11514 (2024; ICSE 2025) | arxiv.org/abs/2405.11514 | Cross-language differential fuzzing as the equivalence oracle with counterexample feedback. Best LLM translated 47% of benchmarks (408 samples, 8,160 translations) | Fuzzing is incomplete. NDSS user-study authors report it handles <20% of C programs >150 LOC. ORBIT authors note it required substantial manual effort on new datasets |
| VERT (Yang, Takashima, Paulsen, Dodds, Kroening) | arXiv 2404.18852; polyglot version at ASE 2025 | arxiv.org/abs/2404.18852 | Compiles source to Wasm, then rWasm, to get an unreadable-but-equivalent Rust oracle; checks LLM output against it with property-based testing and Kani BMC. With Claude-2: PBT pass 31%→54%, BMC 1%→42% | Mostly competitive-programming-scale programs (~100 LOC median per ORBIT's table); bounded proofs |
| Syzygy (Shetty, Jain, Godbole, Seshia, Sen) | arXiv 2412.14234 (2024) | arxiv.org/abs/2412.14234 | Dual code+test translation with dynamic analysis. Translated Zopfli (∼3,000 LOC and 98 functions, generating ∼4,500 Rust LOC), which the authors call "the largest automated and test-validated C to safe Rust code translation achieved so far," in about 15 hours for about $2,500 | Rust up to 3.67× slower than C; a macro corner case needed manual repair. A third-party review notes top-level validation compared compression ratios rather than exact outputs |
| C2SaferRust (Nitin, Krishna, Valle, Ray) | arXiv 2501.14257 (IEEE TSE 2025) | arxiv.org/abs/2501.14257 | c2rust output, then static-analysis-guided slicing, then LLM rewriting to safer Rust with test feedback. Up to 38% fewer raw pointers and up to 28% less unsafe code on coreutils, still passing tests | Most unsafe code remains |
| SACTOR (Zhou et al.) | arXiv 2503.12511 (ACL 2026) | arxiv.org/abs/2503.12511 | Two-step (unidiomatic, then idiomatic) translation with Crown hints and FFI-based verification. 93%/84% end-to-end correctness on two 200-program datasets (DeepSeek-R1). CRUST-Bench: 85% unidiomatic vs 52% idiomatic function-level success | Idiomatic step is far weaker. Failure modes: compiler intrinsics, variadics, pointer arithmetic plus buffer access |
| Tymcrat (Hong & Ryu) | Empirical Software Engineering 30(1), 2025 | doi.org/10.1007/s10664-024-10573-2 | Candidate signatures, callee signatures and compiler-feedback type-error fixing. 63.5% more migrated types and 71.5% fewer type errors vs naive LLM | Type migration on function subsets, not full translation |
| RustMap (Cai et al.) | ICECCS 2025 | arxiv.org/abs/2503.17741 | Project-scale migration with program analysis plus LLM (bzip2, Rosetta Code) | ~1% of programs >1K LOC. Falls back to GCC preprocessing for unhandled directives |
| SmartC2Rust (Shiraishi, Cao, Shinagawa) | ICSE 2026 | arxiv.org/abs/2409.10506 | Segmentation plus feedback; compiles programs >4,000 lines | Authors note runtime/functional errors remain open |
| EvoC2Rust | arXiv 2508.04295 | arxiv.org/abs/2508.04295 | Skeleton-guided project translation. 92.25% compile / 89.53% test at module level on Huawei industrial projects; 75.61% incremental compile on 10K–92K-LOC RepoTransBench projects | A later build-aware study scored EvoC2Rust 0% on OpenHarmony modules, failing on unresolved constants, missing types and placeholder tokens |
| His2Trans / build-aware incremental migration | arXiv 2603.02617 | arxiv.org/abs/2603.02617 | Skeleton-first plus mined historical C/Rust pairs. On OpenHarmony, C2Rust reached only 10.28% and C2SaferRust 53.62% incremental compile; on eight open-source projects, 100% compile/test and unsafe ratio cut from 42.88% (C2Rust) to 8.59% | Relies on historical migration corpora; tested-behavior only |
| LAC2R (Sim et al.) | arXiv 2505.15858 | arxiv.org/abs/2505.15858 | Evaluated on TRACTOR Public-Tests (B01_organic 38 programs, B01_synthetic 85, P00_perlin_noise); claims to beat SACTOR, EvoC2Rust, RustAssure on safety/quality | Small programs |
| ORBIT (Farrukh, Palit, Coskun, Polychronakis) | arXiv 2604.12048 (2026) | arxiv.org/abs/2604.12048 | Dependency-graph orchestration of specialized agents, each paired with deterministic checks. 24 CRUST-Bench programs (91.7% >1K LOC): 100% compile, 91.7% test success; unsafe lines 0.06–0.11% vs 69.6% for C2Rust. 9/13 (~70%) on hardest TRACTOR battery programs, "competitive" with six performers | Programs still only ~1.4K LOC median. Test-based correctness only. Notes float-precision divergence (f32::powf) |
| LLM4C2Rust (Bedell, Siavash, Moin) | arXiv 2604.15485 (2026) | arxiv.org/abs/2604.15485 | RAG (Rust docs plus compiler-error references) with an LLM+SLM pipeline; GPT-4o/GPT-4-Turbo/o3-mini "generally" improves correctness and security | Small-scale, coreutils-derived evaluation; modest claims |
| Rustine (Dehghan et al., UIUC) | arXiv 2511.20617 | arxiv.org/abs/2511.20617 | Repository-level pipeline that avoids reliance on frontier models. 23 programs (27–13,200 LOC), all fully compilable; safer/more idiomatic than six prior repo-level tools | Abstract reports 87% on a functional metric (exact definition not verified) |
| Other 2025–2026 agentic work | arXiv | 2604.04527 (ENCRUST), 2605.14634 (RustPrint), 2609.15381 (Translator vs. Challenger), 2606.31706 (AdaTrans), 2505.04852 (PR2: removed 13.22% of local raw pointers), 2607.28835 (Ship-of-Theseus: iodine DNS tunnel, 12.5 kSLOC C → 10,299 Rust lines, 96% safe), 2606.27122 (Reboot: six C interpreters of 6K–23K LOC with 1–11 human interventions each, 62–92% on new validation tests), 2602.18534, 2603.28686 (C2RustXW) | Converging pattern: skeleton first, dependency order, compile/test gate, agent repair, unsafe-elimination pass. Mostly preprints, not yet peer-reviewed |

### A3. Benchmarks and empirical studies

| Work | Venue/Year | Link | Key result | Limitation |
|---|---|---|---|---|
| TRACTOR Benchmark (Okhravi, Ogden, Luther, McQuoid, Mak, Burow; MIT LL) | arXiv 2609.25121, 20 Sep 2026 | arxiv.org/abs/2609.25121 | Batteries B01/B02 plus milestone projects P00–P02, public and hidden test cases, executable vs `_lib` (ABI-preserving) targets. JSON test vectors (argv/stdin/env/stdout/stderr/rc/lib_state_in/lib_state_out/has_ub; corrected 2026-10-07). Filesystem-state equivalence via Falco. Semantic unsafe-ops counter built on compiler instrumentation. Clippy plus a manual 6-dimension idiomaticity rubric. Metrics in priority order: correctness, safety, idiomaticity, performance | Explicitly "empirical, test-based… not formal"; correctness "bounded by the coverage of the test suite". UB vectors (`has_ub`) skipped. Performer results not in the report |
| CRUST-Bench (Khatry et al., UT Austin) | COLM 2025 | arxiv.org/abs/2504.15254 | 100 C repos with hand-written safe Rust interfaces and tests (avg 76.4 tests/repo, 67% coverage). o1: 15% single-shot test pass, 37% with test repair; SWE-agent+Claude 3.7: 32%. Dominant errors are type mismatches and borrow violations | Small repos; fixed interfaces test interface-conformant translation, not interface design |
| C2Rust-Bench | arXiv 2504.15144 | arxiv.org/abs/2504.15144 | Minimized representative dataset for transpilation evaluation | Not reviewed in depth |
| "Translating C To Rust: Lessons from a User Study" (Li, Wang, Li, Saxena, Kundu) | NDSS 2025 | ndss-symposium.org (2025-1407 paper) | Non-expert humans produced safe Rust; tools could not. Humans "semantically lift" to String/Vec/enums; overhead mostly within 20%. Fuzzing showed 27/31 translations behaved differently on more than half of newly generated inputs | Small study (8 programs); even human translations were not equivalent |
| "C-to-Rust Fallacy: Automatic Refactoring ≠ Memory Security" (Chen et al.) | arXiv 2609.25682, Sep 2026 | arxiv.org/abs/2609.25682 | C2Rust-analyze, Crown, C2SaferRust and Flourine on 116 NIST Juliet programs with known memory bugs: 342 Rust outputs failed to compile, 177 inherited the original bugs, 77 new bugs introduced | Juliet is synthetic; agentic 2026 tools not tested |
| "Rust for Embedded Systems: Current State and Open Problems" (Sharma et al., Purdue) | ACM CCS 2024 | doi.org/10.1145/3658644.3690275 | c2rust fails on 93.8% (15/16) of embedded codebases. Failed outright on 6 of 16 RTOSes; most needed hand-fixed compile_commands.json; emits std-dependent wrappers unusable in embedded. Embedded crates use ~2× the unsafe blocks of non-embedded crates | Rule-based tool only; no LLM evaluation |

### A4. Equivalence checking and verification

| Work / tool | Venue/Year | Link | Relevance | Limitation |
|---|---|---|---|---|
| RustAssure (Bai & Palit) | arXiv 2510.07604 (IEEE 2025) | arxiv.org/abs/2510.07604 | Differential symbolic testing: both C and Rust compiled to LLVM IR, symbolically executed with KLEE, symbolic return values compared by graph edit distance. 89.8% of functions compilable; 69.9% (v1 abstract says 72%) symbolically equivalent. Found 11 semantic bugs Flourine missed, e.g. a dropped `++ptr` | Function-level. External calls return fresh symbolic values, so false positives/negatives are possible. Rust-IR artifacts need normalization |
| Kani (AWS bounded model checker for Rust) | Tool; "Verifying the Rust Standard Library", arXiv 2606.17374 | arxiv.org/abs/2606.17374 | Used by VERT for C-vs-Rust equivalence harnesses | Bounded by default; checks only a subset of Rust UB; trust rests on unverified CBMC/solvers. Authors say results are "high-confidence validation rather than formal verification" |
| Wasm/rWasm oracle (VERT) | 2024–25 | (see VERT) | Mechanically correct but unreadable reference Rust | Equivalence relative to the Wasm compile of C, not C semantics |
| KLEE, CBMC, CompCert-style translation validation, semantic program alignment (Churchill et al., PLDI 2019) | Foundational | — | Building blocks for C-side symbolic execution and cross-program equivalence | Path explosion; loops need alignment or invariants |
| Verus, Prusti, Creusot, Aeneas | Rust deductive verifiers | — | Could prove properties of translated Rust (e.g., parser memory/functional specs) | Not found applied to C-to-Rust equivalence at scale; all need human-written specifications or invariants (inferred) |
| "From C to verifiable Rust: Towards practical migration of code and specifications" | Science of Computer Programming 254, 2026 | doi.org/10.1016/j.scico.2026.103535 | Migrating specifications along with code | Not read in full |

### A5. Industry and policy context

- **DARPA TRACTOR.** Aims for Rust of "quality and style that a skilled Rust developer would produce." MIT LL is the independent test and evaluation organization and releases new benchmark batteries every 6 months. MIT LL says manual translation "can take years and cost billions of dollars." The benchmark report states that "human judgment remains essential."
- **Google Android (Nov 2025 blog).** Memory-safety bugs fell below 20% of vulnerabilities for the first time. Google estimates 0.2 memory-safety vulnerabilities per million lines across roughly 5M lines of Rust, vs closer to 1,000/MLOC historically for C/C++: "a more than 1000x reduction." Its one near-miss (CVE-2025-48530) sat in unsafe Rust. This comes from writing *new* Rust, not from translating old C.
- **Microsoft.** MSRC (July 2019): "~70% of the vulnerabilities Microsoft assigns a CVE each year continue to be memory safety issues."
- **NSA/CISA, "Memory Safe Languages: Reducing Vulnerabilities in Modern Software Development" (June 2025).** Favors incremental adoption: "Adopting MSLs does not necessitate a complete rewrite of existing codebases." Mentions TRACTOR.
- **EU Cyber Resilience Act (Regulation (EU) 2024/2847).** Applies from 11 December 2027. Article 14 reporting obligations apply from 11 September 2026, and Chapter IV from 11 June 2026. Secondary sources say the CRA does not mandate specific programming languages. Treat the CRA as a demand driver for "secure by design" evidence, not as a Rust mandate.

## (b) Hardest Development Challenges, Ranked

1. **Defining equivalence when the C has UB, implementation-defined behavior, or no spec (hardest; mostly unsolved).** TRACTOR sidesteps it ("C means what LLVM does"; UB-marked vectors excluded) and its authors say "the best way to handle UB is unclear" and that silently changing semantics is "the most dangerous option." For parsers, the UB is often the vulnerability, so exact equivalence would preserve the bug. The product needs a formal *refinement* notion: equal on all defined-behavior inputs, with a documented, customer-approved policy for UB inputs, plus tooling to detect UB-triggering inputs (sanitizers, KLEE, CBMC).
2. **The oracle and coverage problem: evidence stronger than "passes the tests" (hard; partially solved).** CRUST-Bench tests average 67% coverage; ORBIT showed agents passing while implementing 64.4% of functions. Differential fuzzing misses deep paths; RustAssure is function-local; VERT/Kani BMC is small-scale. Combining per-function symbolic/BMC checks, whole-program differential fuzzing and coverage gating is the core technical moat.
3. **Ownership, aliasing and lifetimes (hard; structural limits).** Static lifting caps at 12–21% (Aliasing Limits), median 37.3% (Crown). LLMs get further by re-architecting, which is exactly what makes equivalence harder. Permanent tension: more idiomatic means further from the C.
4. **Embedded/IoT specifics (hard; unstudied for LLMs).** c2rust fails on 15/16 embedded codebases; embedded Rust uses ~2× more unsafe. Volatile registers, ISR-shared globals, linker-placed buffers and vendor extensions have no I/O oracle; equivalence must cover register read/write sequences with a peripheral model (QEMU/Renode or recorded traces).
5. **Preprocessor, conditional compilation and build integration (hard engineering).** Hayroll reconstructs "most" macros. EvoC2Rust scored 0% and C2Rust 10.28% incremental compile on OpenHarmony. Firmware ships many `#ifdef` variants; each shipped configuration must be validated.
6. **Repository-scale consistency, ABI/FFI and incremental migration (medium-hard).** Skeleton-first, dependency-ordered translation; ABI-preserving `_lib` targets force unsafe at the boundary; unsafe counts cannot distinguish thin wrappers from laundered logic.
7. **Semantic mismatches (medium):** debug-mode overflow panics vs release wrapping, f32/double promotion, signedness and shift UB, errno conventions, global mutable state, lock semantics. LLMs drop details such as a single `++ptr`.
8. **Performance regressions (medium):** Syzygy up to 3.67× slower; NDSS humans mostly within 20%. Embedded customers reject heap use and code-size growth: need performance and footprint gates.
9. **Idiomaticity and maintainability (medium):** TRACTOR uses Clippy, cyclomatic complexity and a manual rubric; SACTOR's idiomatic step succeeds far less (52% vs 85%).
10. **LLM failure modes and cost (medium):** false success reports, TODO-comment stubs, hallucinated mappings, nondeterminism, context limits; Syzygy spent ~$2,500 for 3K LOC.

## (c) Human vs. AI Responsibility Breakdown

| Area | Must be you (human) | Can be delegated to LLM/AI (with deterministic gates) |
|---|---|---|
| Equivalence contract and UB policy | Define refinement semantics, treatment per UB class, float tolerances, observable-state model (I/O, registers, timing); customer sign-off | Draft UB inventories from sanitizer/KLEE output |
| Oracle/validation architecture | Choose and compose fuzzing, symbolic execution, BMC and proofs; set coverage and acceptance thresholds; design anti-gaming checks | Generate fuzz harnesses, property tests, Kani harness drafts, test-vector translation |
| Pipeline architecture | Skeleton/dependency orchestration; which tool feeds what; deterministic verifier after every agent step | Implementation boilerplate |
| Bulk translation and repair | Spot-review semantic diffs on high-risk functions | Function-by-function translation, compile-error and borrow-checker repair, unsafe-elimination refactoring |
| Ownership and data-structure redesign | Approve redesigns of core data structures; judge the trade-off against provability | Propose redesigns; use Crown hints |
| Unsafe/FFI boundary | Decide where unsafe lives; write or audit `unsafe` blocks and safety comments; design the C ABI shim | Generate bindgen glue and wrapper boilerplate |
| Embedded/hardware semantics | Model MMIO, volatile ordering, ISR concurrency, memory maps, no_std/alloc constraints, vendor toolchains; build peripheral emulation | Translate register-access code into PAC/svd2rust-style APIs once the model exists |
| Build and configuration | Enumerate shipped `#ifdef` configurations and decide which to validate | Translate macros (Hayroll-assisted), write Cargo features |
| Security review | Threat-model the parser; confirm the translation removed rather than reproduced the C bug class; review panics as DoS | Run Miri/sanitizers/fuzzers, triage crashes |
| Performance | Set budgets; approve unsafe or `get_unchecked` optimizations | Profile-guided rewrite suggestions |
| Evaluation methodology | Hold out hidden tests; measure semantic unsafe, not keyword counts; never let the agent see the acceptance tests | Run benchmarks |
| Customer "done" definition and liability | Agree what evidence ships; sign off as accountable engineer | Draft reports |

**Where LLMs are proven good:** syntax and API translation, compile-error repair loops, idiomatic rewriting under compiler feedback, test translation, harness scaffolding, orchestrated codebase navigation.

**Where they demonstrably fail:** single-shot ownership reasoning; off-by-one and pointer-increment semantics; honest self-assessment; preserving security properties; anything requiring hardware knowledge outside the code.

## (d) Recommended Roadmap

1. **Months 0–2: equivalence contract and validation harness, before any translator.** Spec of "equivalent" (refinement modulo a UB policy) for stateless or bounded-state parsers; C-side UB detector (ASan/UBSan/MSan plus KLEE); differential harness over an ABI shim; reuse TRACTOR's JSON test-vector schema.
2. **Months 1–3: borrow the translator front-end.** c2rust plus Hayroll, Crown hints, then a skeleton and dependency-ordered agent loop with deterministic gates (implementation checker, `cargo check`, semantic-unsafe counter). Do not invent a new translator.
3. **Months 2–5: layered equivalence engine, which is the product.** Per-function differential symbolic execution (KLEE on LLVM IR of both sides); Kani/CBMC bounded equivalence for small critical functions; coverage-guided differential fuzzing (libFuzzer/AFL++) with structure-aware grammars; gate every merge on coverage thresholds and zero divergences on defined-behavior inputs.
4. **Months 3–6: benchmark publicly** on TRACTOR Public-Tests and CRUST-Bench; report correctness, semantic unsafe, idiomaticity, performance and evidence strength.
5. **Months 4–9: embedded wedge.** Start with hardware-independent parser/protocol modules (TLV, CBOR, MQTT, BLE GATT, OTA image parsers) as `no_std`, allocation-free crates behind a C ABI; only then MMIO/ISR code with Renode/QEMU and register-trace equivalence.
6. **Months 6–12: evidence packaging** mapped to CRA documentation; optional deductive proofs (Verus/Creusot/Aeneas) for the most critical parser functions.

## (e) Open Problems (Moat) vs. Solved-Enough (Borrow)

**Open: where you can differentiate**

- Refinement-based equivalence with explicit UB policies and automatic UB-input characterization.
- Compositional equivalence across structurally divergent code (C arrays vs Rust Vec/enums).
- Hardware-aware equivalence: MMIO traces, interrupt interleavings, no_std/no-alloc translation.
- Configuration-space validation without combinatorial blow-up.
- Distinguishing necessary from laundered unsafe automatically.
- Security-preservation checks showing the translation eliminated, rather than inherited, the C bug class.

**Solved-enough: borrow**

- c2rust front-end; Crown ownership hints; Hayroll macros; Concrat/Urcrat/Nopcrat/Forcrat/GenC2Rust idiom lifters (check licenses and maintenance).
- Skeleton-first, dependency-ordered agent orchestration (ORBIT, EvoC2Rust, His2Trans patterns).
- Compile-error and borrow-error repair loops with LLMs.
- Fuzzers (libFuzzer, AFL++, cargo-fuzz), KLEE, Kani/CBMC, Miri, sanitizers.
- TRACTOR's test-vector schema, Test Runner/Cando, and semantic-unsafe instrumentation.

## (f) Caveats

- Most 2025–2026 results are arXiv preprints, often self-evaluated on self-chosen subsets with differing metrics; cross-paper numbers are not directly comparable.
- Benchmark leakage and gaming risk: public TRACTOR tests and CRUST-Bench are public.
- Conflicting figures: RustAssure 69.9% vs 72%; VERT venue; Rustine's 87% metric unverified.
- No direct published application of Verus/Prusti/Creusot/Aeneas to C-to-Rust equivalence found.
- Embedded evidence is thin: one rule-based study, no LLM-based embedded evaluations.
- Google's Android 1000× figure is for newly written Rust, not machine-translated code.

## Sources

1. https://arxiv.org/html/2604.12048v1
2. https://www.cs.usfca.edu/~memre/oopsla23-aliasing-limits.pdf
3. https://www.emergentmind.com/papers/2404.18852
4. https://arxiv.org/html/2609.25121
5. https://blog.google/security/rust-in-android-move-fast-fix-things/
6. https://www.ndss-symposium.org/wp-content/uploads/2025-1407-paper.pdf
7. https://dl.acm.org/doi/abs/10.1145/3586046
8. https://arxiv.org/pdf/2303.10515
9. https://arxiv.org/pdf/2503.12511
10. https://arxiv.org/pdf/2506.01427
11. https://homes.cs.washington.edu/~mernst/pubs/c-rust-macros-pldi2026-abstract.html
12. https://arxiv.org/pdf/2412.14234
13. https://www.arxiv.org/pdf/2501.14257
14. https://www.arxiv.org/pdf/2508.04295
15. https://arxiv.org/html/2603.02617
16. https://arxiv.org/html/2505.15858v3
17. https://arxiv.org/html/2604.15485
18. https://arxiv.org/abs/2511.20617
19. https://arxiv.org/html/2607.28835
20. https://arxiv.org/abs/2609.25682
21. https://dl.acm.org/doi/pdf/10.1145/3658644.3690275
22. https://arxiv.org/pdf/2510.07604
23. https://arxiv.org/pdf/2404.18852
24. https://arxiv.org/pdf/2606.17374
25. https://media.defense.gov/2025/Jun/23/2003742198/-1/-1/0/CSI_MEMORY_SAFE_LANGUAGES_REDUCING_VULNERABILITIES_IN_MODERN_SOFTWARE_DEVELOPMENT.PDF
26. https://eur-lex.europa.eu/eli/reg/2024/2847/oj/eng
27. https://www.ll.mit.edu/r-d/projects/translating-all-c-rust-tractor-benchmarks
28. https://ieeexplore.ieee.org/document/11334436/
