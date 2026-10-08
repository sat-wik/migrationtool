---
type: DOC
title: "Fact-check of the product spec and literature map"
checked: 2026-10-07
scope: docs/specs/product-spec.md, docs/research/literature-map.md
method: Each load-bearing claim checked against the primary source (regulation text, vendor page, paper abstract or full text, tool repository). Claims not re-checked are listed as such.
precedence_note: Where this file and another doc disagree on a fact, this file wins. It does not override decisions.
---

# Fact-check: product spec and literature map

Checked 2026-10-07. **No claim the spec depends on turned out false.** Two need tighter wording, one detail in the literature map was slightly wrong, and the check surfaced six technical risks the spec does not mention. The risks matter more than the wording fixes: R1, R2 and R5 change how the validation engine has to be built.

## Verdicts

### Regulation and market claims

- **VERIFIED: CRA dates.** Regulation (EU) 2024/2847 Art. 71(2): applies from 11 December 2027; Article 14 (reporting) from 11 September 2026; Chapter IV (Articles 35–51) from 11 June 2026. Source: OJ text via springlex.eu Art. 71.
- **VERIFIED: Medical devices and vehicles are outside the CRA.** Art. 2(2) excludes products covered by Regulations (EU) 2017/745 (medical devices), 2017/746 (IVD) and 2019/2144 (vehicle type-approval); Art. 2(3) excludes aviation products certified under 2018/1139 and marine equipment under Directive 2014/90/EU. Nuance: automotive parts that are not type-approved under 2019/2144 (aftermarket, some tier-2 components) can still be in scope.
- **CORRECTED: CRA SBOM sentence.** The spec says manufacturers must keep a machine-readable SBOM in their technical documentation for at least ten years. More precisely: Annex I Part II(1) requires an SBOM "in a commonly used and machine-readable format covering at the very least the top-level dependencies"; technical documentation and the EU declaration of conformity are kept for at least ten years after placing on the market **or for the support period, whichever is longer**; market surveillance authorities can request the SBOM (Art. 13). Product implication: our SBOM should cover all dependencies we add, not just top-level, because that exceeds the legal minimum at no extra cost.
- **VERIFIED: Google Android figures.** Blog post of 13 Nov 2025: ~0.2 memory-safety vulnerabilities per MLOC of Rust (≈5M lines, one potential vulnerability fixed pre-release) vs "closer to 1,000" per MLOC historically for C/C++, "a more than 1000x reduction"; memory-safety share of vulnerabilities fell below 20% in 2025. The post's analysis centres on new code; it does not make claims about translated code. The spec's caveat stands.
- **VERIFIED: C-to-Rust Fallacy study.** arXiv 2609.25682 (Chen, He, Lu, Zhang, Sun; 22 Sep 2026): 116 Juliet programs, 464 Rust outputs from C2Rust-analyze, CROWN, C2SaferRust and FLOURINE; 342 failed to compile, 177 inherited the original memory bugs, 77 new bugs introduced.
- **NOT RE-CHECKED: NSA/CISA June 2025 guidance.** Consistent with prior knowledge of the document; the PDF was not re-fetched.

### Toolchain and qualification claims

- **VERIFIED, WITH NUANCE: Ferrocene.** Ferrous Systems (Ferrocene 26.02.0): TÜV SÜD-qualified for ISO 26262 ASIL D, IEC 61508 SIL 3 and IEC 62304 Class C; DO-178C is not qualified, only supported toward DAL C (the spec says this correctly). Business Wire (Dec 2025, Ferrocene 25.11.0): certification applies to qualified targets "including Armv7E-M and Armv8-A". **New nuance:** only a subset of `core` is certified (IEC 61508 SIL 2 / ISO 26262 ASIL B; 5,169 functions as of 26.02.0). See risk R4.
- **VERIFIED: c2rust.** BSD-3-style licence; requires LLVM/Clang 15 or later; needs exact compile commands via `compile_commands.json`; supports cross-architecture transpilation with a different sysroot. Output is unsafe Rust mirroring the C.
- **VERIFIED: Hayroll.** PLDI 2026 (Peng, Kasikci, Bernstein, Ernst). Wraps C2Rust; translator-agnostic design. Code at github.com/UW-HARVEST/Hayroll. Requires a single matching LLVM/Clang version and explicitly not LLVM 20. **Licence not stated** in the repository view checked. See risk R3.
- **VERIFIED: TRACTOR benchmark.** arXiv 2609.25121 (Okhravi et al., 20 Sep 2026). Correctness is "behavioral fidelity to the compiled C binary" ("C means what LLVM does"); test vectors with `has_ub` are excluded from automated scoring; `_lib` targets must preserve the C library's symbols and signatures; unsafe is measured semantically by compiler instrumentation. Public tests and tools: github.com/DARPA-TRACTOR-Program/PUBLIC-Test-Corpus (Test Runner and Cando under `tools/`), evaluation infrastructure at github.com/DARPA-TRACTOR-Program/PUBLIC-aws-translate. The report itself is CC BY 4.0; the corpus licence was not checked.
- **CORRECTED (literature map): TRACTOR test-vector fields.** The schema has separate `lib_state_in` and `lib_state_out` fields, not one `lib_state`. Full field list: `argv`, `stdin`, `env`, `stdout`, `stderr`, `rc`, `lib_state_in`, `lib_state_out`, `has_ub`.
- **VERIFIED, WITH LIMITS: Kani.** Official feature table: unsafe code and external blocks supported; inline assembly unsupported (Kani aborts); concurrency out of scope (compiled as sequential with a warning); dereferencing unaligned raw pointers and breaking aliasing rules are **not** detected. See risk R2.
- **VERIFIED: KLEE version support.** KLEE 3.2 (23 Dec 2025): recommended LLVM 16, partial support for LLVM 17–19, LLVM < 13 removed. No Rust support mentioned. See risk R1.

### Research claims used in the spec's reasoning

- **VERIFIED: c2rust on embedded code.** Sharma et al., ACM CCS 2024 (arXiv 2311.05063): "C to Rust tools fail on most, i.e., 93.8% (15/16), embedded codebases." Breakdown: failed outright on 6 of 16 RTOSes; of the 10 that ran, 9 produced incorrect or syntactically invalid Rust. All but two needed hand-fixed `compile_commands.json`. Embedded crates use ~2× the unsafe blocks of non-embedded crates.
- **VERIFIED: ORBIT headline numbers.** arXiv 2604.12048 (13 Apr 2026): 100% compilation and 91.7% test success. **Not re-checked:** the "64.4% of functions / 3.0% of test cases" figure for vanilla agents is not in the abstract; it is attributed to the paper body.
- **VERIFIED: RustAssure.** arXiv 2510.07604 (Bai, Palit; 8 Oct 2025): 89.8% of C functions translated to compilable Rust; 69.9% of those produced equivalent symbolic return values.
- **NOT RE-CHECKED:** CRUST-Bench, Aliasing Limits, Crown, Syzygy, SACTOR, EvoC2Rust, His2Trans and Rustine figures in the literature map. They inform background reasoning only; no requirement depends on an exact value.

## Technical risks the spec does not mention

These were found while checking the claims. Each is a requirement or a spike for the roadmap.

- **R1. LLVM version lock across the analysis toolchain.** KLEE supports LLVM 16 fully and 17–19 partially; Hayroll needs one matching LLVM/Clang and rejects LLVM 20; c2rust needs LLVM 15 or later. Current stable `rustc` ships a newer LLVM than KLEE supports, so "KLEE on both sides" needs either a pinned older `rustc` whose LLVM is 19 or lower for bitcode emission (confirm with `rustc -vV`), or a different Rust-side symbolic engine. **Action:** pin one LLVM (16–19) for all C-side analysis; spike the Rust-to-KLEE path in the harness phase before promising per-function symbolic equivalence.
- **R2. Kani's limits shape the "c2rust output as reference" idea.** That idea is plausible but unproven: VERT used an rWasm reference, not c2rust output. c2rust output calls libc through `extern "C"`, which Kani cannot execute without stubs or models; Kani also misses aliasing violations and unaligned dereferences, and does not reason about concurrency. **Action:** treat the technique as a spike with a go/no-go, write libc stubs, and keep fuzzing plus KLEE as the primary evidence until the spike passes.
- **R3. Hayroll licence is unconfirmed.** **Action:** confirm the licence (or get permission) before depending on it; isolate it behind an adapter so it can be swapped.
- **R4. Ferrocene certifies only part of `core`.** Safety customers may require emitted Rust to call only certified `core` functions, and our output's minimum Rust version must match the `rustc` inside the customer's Ferrocene release. **Action:** add an output profile that restricts emitted code to the certified `core` subset and to a pinned MSRV.
- **R5. Host-versus-target ABI mismatch in the harness.** Sanitizers and fuzzers run on a host, but Cortex-M is ILP32 with unsigned plain `char`, while an x86-64 host is LP64 with signed `char`. A differential run on a default 64-bit host can disagree with the target or, worse, agree with each other and with neither. **Action:** run the differential harness on an ILP32 configuration that matches the target (for example `-m32`/i686, or Arm Linux under QEMU user mode) with `-funsigned-char` and the customer's flags; replay the final corpus on the real target or Renode.
- **R6. Sanitizer constraints.** MemorySanitizer needs Clang, every linked object instrumented, and cannot share a build with AddressSanitizer. **Action:** build separate ASan+UBSan and MSan variants of the C and replay the corpus against each.

## Sources

- [EU Cyber Resilience Act, Art. 2, Art. 13, Art. 71, Annex I (OJ text mirror)](https://www.springlex.eu/en/packages/cra/cra-regulation/article-71/)
- [EUR-Lex, Regulation (EU) 2024/2847](https://eur-lex.europa.eu/eli/reg/2024/2847/oj/eng)
- [CRA technical documentation and SBOM guide (sota.io)](https://sota.io/blog/cra-article-22-technical-documentation-requirements-annex-v-vi-conformity-assessment-developer-guide-2026)
- [Google: Rust in Android, move fast and fix things](https://blog.google/security/rust-in-android-move-fast-fix-things/)
- [C-to-Rust Fallacy, arXiv 2609.25682](https://arxiv.org/abs/2609.25682)
- [Ferrocene 26.02.0 release (Ferrous Systems)](https://ferrous-systems.com/blog/ferrocene-26-02-0/)
- [Ferrocene certification press release (Business Wire)](https://www.businesswire.com/news/home/20251203818102/en)
- [c2rust repository](https://github.com/immunant/c2rust)
- [Hayroll, PLDI 2026](https://homes.cs.washington.edu/~mernst/pubs/c-rust-macros-pldi2026-abstract.html) and [repository](https://github.com/UW-HARVEST/Hayroll)
- [TRACTOR Benchmark, arXiv 2609.25121](https://arxiv.org/html/2609.25121)
- [Kani Rust feature support](https://model-checking.github.io/kani/rust-feature-support.html)
- [KLEE 3.2 release notes](https://github.com/klee/klee/releases/tag/v3.2)
- [Rust for Embedded Systems, CCS 2024 (arXiv 2311.05063)](https://arxiv.org/html/2311.05063v2)
- [ORBIT, arXiv 2604.12048](https://arxiv.org/abs/2604.12048)
- [RustAssure, arXiv 2510.07604](https://arxiv.org/abs/2510.07604)
