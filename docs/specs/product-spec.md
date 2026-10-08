---
type: SPEC
title: "Validated C-to-Rust Migration: Getting Started and Product Spec"
source: https://claude.ai/code/artifact/da6026b8-04a5-4ef2-bfd2-2545eefcb9fb
source_revision: 14
exported: 2026-10-07
author: Satwik
---

# Validated C-to-Rust Migration: Getting Started and Product Spec

Oct 7, 2026 · Satwik

> Exported from the Claude Docs original (rev 14). The two diagrams are transcribed as text below. Load-bearing claims were checked against primary sources on 2026-10-07; corrections are applied inline and flagged with **[verified-fix]**.

## Where to start: the first 90 days

Build the validation harness before the translator, and land one paying design partner by day 90. Everything else waits.

1. **Weeks 1–2: Pick the beachhead and a test bed.**
   - Beachhead: parsers and protocol handlers inside Arm Cortex-M firmware (TLV, CBOR, MQTT, BLE, OTA image headers). They are bug-dense, mostly hardware-independent, and testable through inputs and outputs.
   - Start with one RTOS ecosystem (Zephyr or FreeRTOS) and one toolchain (arm-none-eabi GCC).
   - Collect 10–20 open-source C targets as a test bed, for example zcbor, tinycbor, nanopb, lwIP's protocol parsers and Mbed TLS's ASN.1 parser.
   - If you have a day job, check your employment agreement's invention-assignment clause and keep this work on your own time and hardware.
2. **Weeks 1–4: Write the equivalence contract and build the harness.**
   - Contract v0: the Rust must match the C on every input where the C has defined behavior. On inputs that trigger undefined behavior (UB) in the C, the Rust returns an error instead. That default is customer-adjustable.
   - UB detector: AddressSanitizer and UndefinedBehaviorSanitizer runs, plus KLEE, to label which inputs trigger UB in the original C.
   - Differential runner: link the original C and the candidate Rust into one fuzz harness (libFuzzer or cargo-fuzz) behind the same C interface, and compare outputs and state on every input.
   - Reuse the TRACTOR benchmark's JSON test-vector format so your numbers compare directly with the DARPA program's.
   - Prove the harness works: plant 20 known bugs in hand-written Rust and confirm it catches every one.
3. **Weeks 3–6: Assemble the translator from existing parts.** Use c2rust as the front end and Hayroll for macros and `#ifdef`s. Then run an LLM agent loop that translates function by function in dependency order, with a deterministic check after every step: it compiles, contains no stubs, and passes the harness. Target `no_std`, allocation-free Rust from day one.
4. **Weeks 4–10, in parallel: Talk to 20 potential buyers.** Interview firmware leads, product-security (PSIRT) owners and compliance managers at device makers in regulated markets. Learn which C modules worry them most, what evidence their auditors and customers accept, whether code may leave their network, who signs off, and what a migration is worth. Goal: two or three design partners.
5. **Weeks 6–10: Benchmark publicly.** Run TRACTOR's public tests and your test bed. Publish correctness, remaining unsafe code, performance, code size and evidence strength: fuzz coverage and the share of functions with symbolic or bounded proofs.
6. **Weeks 8–13: Run a paid pilot.** Migrate one module from a design partner's codebase, drop it into their build behind the existing C interface, and deliver the full evidence package described in the evidence section.

**Day-90 go/no-go (proposed thresholds):** continue if the harness catches every planted bug, at least 70% of test-bed parsers reach full evidence with zero divergences, and at least two companies have paid for a pilot or signed a letter of intent.

## Product overview

The product turns a device maker's riskiest C modules into drop-in safe Rust, and ships each module with evidence that it behaves the same as the original. The translation is a commodity; the evidence is what customers buy.

**The problem**

- **Memory-safety bugs dominate.** Google reports roughly 1,000x fewer memory-safety vulnerabilities per line in new Android Rust than in its historical C/C++ ([Google](https://blog.google/security/rust-in-android-move-fast-fix-things/)). That figure is for newly written code, not translated code.
- **Pressure is rising.** The EU Cyber Resilience Act applies in full from December 11, 2027, with vulnerability reporting obligations already in force since September 11, 2026 ([EUR-Lex](https://eur-lex.europa.eu/eli/reg/2024/2847/oj/eng)). NSA and CISA guidance favors incremental adoption of memory-safe languages over full rewrites ([NSA/CISA](https://media.defense.gov/2025/Jun/23/2003742198/-1/-1/0/CSI_MEMORY_SAFE_LANGUAGES_REDUCING_VULNERABILITIES_IN_MODERN_SOFTWARE_DEVELOPMENT.PDF)).
- **Current tools can't be trusted on their own.** One study found automated C-to-Rust tools carried 177 existing memory bugs into the Rust and introduced 77 new ones ([arXiv](https://arxiv.org/abs/2609.25682)). Code that compiles and passes tests can still behave differently.

**Positioning:** for firmware teams who must cut memory-safety risk without breaking shipped behavior, we migrate their riskiest C modules to safe Rust and prove each one matches the original. Unlike AI coding assistants, every module ships with an equivalence report an auditor can read.

**First customers:** makers of connected industrial and consumer devices sold in the EU, where the CRA applies. Medical and automotive suppliers follow their own safety and cybersecurity rules but need the same evidence. Later: defense primes, network-equipment and OS vendors.

| Persona | Cares about | Will ask for |
| --- | --- | --- |
| Firmware lead or VP Engineering (champion, often the budget owner) | Schedule, team capacity, not breaking the product | Effort estimate, incremental path, performance and code-size guarantees |
| Product security / PSIRT lead | Fewer vulnerabilities, faster CVE response | Proof that bug classes were removed, fuzzing results |
| Compliance or regulatory manager | Passing CRA and industry audits | Evidence mapped to requirements, a full audit trail |
| Firmware engineers (daily users) | Readable, maintainable Rust that fits their build | Clean code, CI integration, a good review workflow |
| Procurement, legal and IT security | IP, data handling, liability | On-prem option, no training on their code, IP ownership, indemnity |

## Customer requirements

A buyer needs nine things before they'll ship migrated code; missing any one of them stalls the deal.

**1. Inputs they will hand over (and we must accept)**

- Source access: a git repo, or an on-prem snapshot if code can't leave their network.
- Their build as-is: Make, CMake, Zephyr's west, or IDE projects (IAR, Keil), plus `compile_commands.json` where available.
- Every shipped build configuration (product variants behind `#ifdef`s), with compiler versions and flags.
- Existing tests, fixtures and sample inputs such as packet captures or firmware images.
- Constraints: flash, RAM and stack budgets, heap policy, coding standards, real-time paths.

**2. Scoping before they commit money**

- A risk map of which modules matter most: external-input parsers, CVE history, complexity.
- Per-module feasibility, effort and price, plus a list of what should stay in C.

**3. The Rust they receive**

- Safe Rust, with `unsafe` only at the C boundary and every unsafe block justified in writing.
- `no_std` and allocation-free unless they allow otherwise; builds for their targets (for example `thumbv7em-none-eabihf`).
- A drop-in replacement: the same C interface, so the rest of the firmware doesn't change.
- Readable and maintainable: formatted, Clippy-clean, documented, with a pinned Rust version and minimal, vetted dependencies.

**4. Proof it behaves the same**

- A per-module equivalence report, every undefined-behavior decision, and fuzz coverage numbers.
- The regression tests and fuzz corpus, handed over so they can keep checking after we leave.

**5. Guarantees on resources**

- Speed, code size, stack and RAM within agreed budgets, measured on their target or a cycle-accurate model.
- Real-time paths flagged; no worst-case-timing guarantees in early versions unless contracted separately.

**6. Fit with how they work**

- Incremental delivery, one module at a time, inside a mixed C and Rust build.
- Delivery as reviewable pull requests, plus a review view that shows behavior-relevant differences.
- CI integration (GitHub Actions, GitLab, Jenkins) that re-checks equivalence whenever the C or the Rust changes.

**7. Security and deployment**

- On-prem or air-gapped deployment with locally hosted models; no retention of, or training on, their code.
- SSO, role-based access and audit logs for every action on their code.

**8. Compliance support**

- A software bill of materials (SBOM) covering every Rust dependency we introduce.
- Evidence formatted for CRA technical documentation, and support for qualified Rust toolchains in safety-certified products.
- License review of all third-party components we add.

**9. Commercial terms**

- Clear pricing, full IP ownership of the output, and no lock-in: the code and tests stay usable without our tools.
- A warranty on certified modules, support with response-time commitments, and training so their engineers can maintain the Rust.

## Product spec: modules and acceptance criteria

The product is eleven modules, ten of them forming one pipeline; each ships only when it meets the criteria below. Thresholds marked \* are proposed starting points to confirm with design partners.

**Pipeline diagram (transcribed): "Every module passes validation and two human sign-offs before it ships."** Human decisions are steps 5 and 8; every other step is automated.

1. Ingest: repo, build and every shipped configuration
2. Assess: risk map, UB pre-scan, fixed quote
3. Translate: c2rust + Hayroll, then agents per function
4. Validate: fuzzing, symbolic checks and proofs vs the C (loop: divergence → retranslate, back to step 3)
5. UB decisions (human): customer approves each UB case
6. Integrate: drop-in behind the C interface
7. Resource gates: speed, size, stack, RAM within budget
8. Review and sign-off (human): named reviewer approves the module
9. Evidence bundle: nine artifacts, re-runnable by a third party
10. Continuous equivalence: re-checks in the customer's CI (loop: every code change → re-validate, back to step 4)

Validation sends any divergence back to translation, and the CI check re-runs validation whenever the C or the Rust changes.

| Module | What it does | Done when |
| --- | --- | --- |
| 1. Ingest and build capture | Imports the repo and build; records every compile command for every shipped configuration | We rebuild the customer's firmware for each configuration and the output matches theirs |
| 2. Assess | Produces the scoping report: risk map, dependency graph, UB pre-scan, feasibility, effort and price per module | Pilot estimates land within ±30%\* of actual effort |
| 3. Translate | c2rust and Hayroll front end, then agentic function-by-function translation in dependency order; lifts C idioms to safe Rust; targets `no_std` | Compiles for the target, contains no stubs or TODOs, `unsafe` only at the C boundary |
| 4. Validate | Layered equivalence engine: module-level differential fuzzing against the original C, per-function differential symbolic execution (KLEE), bounded model checking (Kani) for critical functions | Zero divergences on defined-behavior inputs; branch coverage of the C at least 90%\*; every critical function has a bounded proof |
| 5. UB policy manager | Catalogs every input and code site where the C has undefined behavior; records a decision for each (return an error, panic, or preserve) | Every detected UB site has a decision signed off by the customer |
| 6. Boundary and integration | Generates the C header and interface shim, the Cargo crate, and the hooks into Make, CMake or west | The customer's full build and existing test suite pass with the Rust module swapped in |
| 7. Resource gates | Measures speed, code size, stack and RAM against the C on the target or an emulator | Within the agreed budget; default no more than 10%\* regression on any measure |
| 8. Review workspace | Side-by-side C and Rust with mapped functions, a divergence explorer with counterexample inputs, the unsafe ledger, and approvals | A named reviewer approves each module and the sign-off is recorded |
| 9. Continuous equivalence | Re-runs the harness in the customer's CI whenever the C or the Rust changes; blocks merges on divergence | Runs inside their CI in under 30 minutes\* per module |
| 10. Evidence packager | Bundles the reports and artifacts listed in the next section | Every shipped module has a complete, versioned evidence bundle |
| 11. Admin and security | SSO, role-based access, audit log, and the deployment modes in the deployment section | Passes the customer's security review |

Two steps stay human by design: UB decisions (module 5) and final sign-off (module 8). The tooling prepares both, but a named person owns them.

## Technical approach: the four hardest problems

Two of the four problems are hard engineering with known techniques; undefined behavior and the idiomatic-versus-provable tension have open edges, and those edges are the moat.

**1. Undefined behavior in the original C**

- **Refinement, not equality.** The Rust must match the C on every input where the C is well-defined. On inputs where it isn't, the Rust takes a safe action from an approved menu, such as returning an error, and never crashes or corrupts memory.
- **Label UB inputs mechanically.** Replay every fuzz and test input against a C build with AddressSanitizer, UndefinedBehaviorSanitizer and MemorySanitizer. Tagged inputs are checked against the UB policy instead of the C's output.
- **Use optimization levels as a detector.** Build the C at `-O0` and at the customer's real flags; if the two disagree on an input, that input almost certainly hits undefined behavior.
- **Find what fuzzing misses.** Run KLEE or an abstract interpreter such as Frama-C over parser functions to list problem sites the fuzzer never reached.
- **Pin implementation-defined behavior to the target.** Signedness of plain `char` (unsigned on ARM), integer sizes and struct layout come from the customer's real flags and ABI. Every decision goes in the UB log for sign-off.

**2. Evidence strong enough to trust**

- **Layer methods that catch different bugs:** coverage-guided differential fuzzing with protocol-aware mutators seeded from real packet captures, per-function differential symbolic execution with KLEE on both sides, and bounded proofs for critical functions.
- **Use c2rust's output as the reference.** Its unsafe but faithful Rust lets Kani prove the clean Rust matches on bounded inputs, a same-language comparison. The VERT paper used the same idea with a different mechanical reference.
- **Measure coverage on the C side,** so 100% means the original's behavior was exercised, not just the new code.
- **Prove the harness works with mutation testing.** Deliberately break the Rust (cargo-mutants) and confirm the harness catches each change; publish the kill rate.
- **Stop agents gaming the checks.** Agents never see the held-out corpus. Deterministic checks reject stubs, `todo!()` and empty bodies, and confirm every C function is mapped and exercised.

**3. Idiomatic versus provable**

- **Translate in two stages.** First produce faithful, C-like Rust that is easy to prove equivalent to the C. Then refactor toward idiomatic Rust in small Rust-to-Rust steps, each checked against the step before.
- **Fix the observation point at the C interface.** Internals may change freely; return values, output buffers and global state visible across the boundary must match.
- **Write abstraction functions for changed data structures,** for example mapping a Rust `Vec` back to the C pointer and length, and compare through them. AI drafts them; a human reviews them.
- **Offer two tiers per module:** faithful (strongest proof) or idiomatic (easier to maintain, slightly weaker evidence).

**4. Real embedded codebases**

- **Capture the real build** with Bear for Make, CMake's exported compile commands, and parsers for IAR and Keil project files. Rebuild the exact firmware before touching anything.
- **Validate only shipped configurations,** not every `#ifdef` combination. Carry macros over with Hayroll and reuse proofs for configuration-independent code.
- **Grow a rule library for compiler extensions** such as packed structs, section placement and attributes: map each to its Rust equivalent or keep it in C behind the interface. The library compounds with every customer.
- **Start where there is no hardware.** Parsers are pure functions of input buffers, which is why they are the first wedge.
- **For hardware-facing code later,** record register read and write traces on real boards or in the Renode emulator; equivalence means the same trace from the same inputs. Use vendor register crates (svd2rust) and keep volatile access explicit.
- **Measure on real targets.** A few inexpensive STM32 and nRF52 boards cover code size, stack and cycle counts per module.

## Evidence and compliance deliverables

Every migrated module ships with nine artifacts that a third party can re-check without us; this bundle is the product customers pay for.

| Artifact | What it contains | Who uses it |
| --- | --- | --- |
| Equivalence report | The equivalence contract, methods used (fuzzing hours, coverage, symbolic results, bounded proofs and their bounds), divergences found and how each was resolved, tool versions | Security lead, auditors |
| UB decisions log | Each undefined-behavior site or input class in the original C, the decision taken, rationale, approver and date | Firmware lead, auditors |
| Unsafe ledger | Every remaining `unsafe` block, why it is needed, the invariants it relies on, and who reviewed it | Security reviewers |
| Security delta report | Bug classes removed, crashes found in the original C during fuzzing, sanitizer and Miri results | PSIRT, compliance |
| Resource report | Speed, code size, stack and RAM versus the C, measured on target or emulator | Firmware lead |
| Test and fuzz assets | Regression tests, fuzz corpus and harnesses, runnable in the customer's CI | Firmware engineers |
| SBOM | Machine-readable list (SPDX or CycloneDX) of every Rust crate and toolchain component we add | Compliance |
| Traceability matrix | Each C function mapped to its Rust counterpart, with review sign-offs | Auditors, safety assessors |
| Reproducibility bundle | Pinned toolchain, container and scripts so anyone can re-run the validation | Auditors, future maintainers |

**EU Cyber Resilience Act.** **[verified-fix]** Manufacturers must draw up a machine-readable SBOM covering at least the product's top-level dependencies (Annex I Part II(1)), and keep the technical documentation for at least ten years after the product goes on the market or for the support period, whichever is longer; market surveillance authorities can request the SBOM ([itemis](https://www.itemis.com/en/glossary/sbom/), [OpenSSF](https://openssf.org/?p=9096)). Our SBOM covers every dependency we add, which exceeds the top-level minimum. Our SBOM and evidence feed directly into that file. Medical devices are excluded from the CRA because they follow their own sector rules ([Johner Institute](https://blog.johner-institute.com/iec-62304-medical-software/sbom-software-bill-of-materials/)), so for those buyers the evidence supports their medical-device cybersecurity file instead.

**Functional safety (automotive, industrial, medical).** Safety-certified products need a qualified Rust compiler. Ferrocene is qualified by TÜV SÜD for ISO 26262 (ASIL D), IEC 61508 (SIL 3) and IEC 62304 (Class C), and supports work toward DO-178C (DAL C) ([Ferrous Systems](https://ferrous-systems.com/blog/ferrocene-26-02-0/)). Its qualified targets include Armv7E-M, which covers common Cortex-M parts ([Business Wire](https://www.businesswire.com/news/home/20251203818102/en)). Our output must build with Ferrocene, and safety customers will ask how far they can rely on our own tool, so plan a tool-qualification kit for v2. **[verified-fix]** Only a subset of `core` is certified (IEC 61508 SIL 2 / ISO 26262 ASIL B), so offer an output profile that restricts generated code to that subset and to the `rustc` version inside the customer's Ferrocene release.

**What we never claim:** that a product is certified or compliant. We supply evidence; the customer's assessor or notified body makes the call.

## Deployment, security and data handling

Offer three deployment modes, and expect most embedded and defense buyers to choose on-prem. That makes a model-agnostic pipeline a requirement, not a nice-to-have.

| Mode | Where code and models run | Who wants it |
| --- | --- | --- |
| Managed cloud | Our isolated per-customer environment; hosted models only under zero-data-retention terms | Smaller companies, open-source projects, early pilots |
| Customer cloud (VPC) | Inside the customer's own cloud account | Mid-size device makers |
| On-prem or air-gapped | Customer hardware with locally hosted open-weight models, offline licensing and signed update bundles | Defense, critical infrastructure, IP-sensitive firms |

**Design rule:** correctness must never depend on which model translated the code. The validation engine decides what ships, so a weaker local model only costs time, never safety.

**Data commitments (put these in the contract):**

- The customer owns all inputs and outputs, with IP explicitly assigned to them.
- No training on customer code, and no retention after the engagement unless agreed; deletion confirmed in writing.
- Code and evidence encrypted in transit and at rest; secrets scanned and stripped before any processing.

**Access and audit:** SSO (SAML or OIDC), role-based access, and an audit log of every action taken on customer code, by people and by agents.

**Our own supply chain:** signed releases of our tools, our own SBOM, and pinned, reproducible toolchains.

**Certifications to plan for:** SOC 2 Type II before selling cloud deployments to US enterprises, and ISO 27001 for European buyers. Defense contractors will ask about handling controlled information and export rules; air-gapped deployment, where we never hold their code, answers most of that.

## Questions customers will ask

These seventeen questions will come up in almost every sales cycle; have the answers ready before the first call.

| Question | Our answer |
| --- | --- |
| How do you know the Rust behaves the same? | Layered evidence: differential fuzzing against your original C, symbolic checks per function, and bounded proofs for critical functions. Zero divergences on valid inputs, and anyone can re-run it from the bundle. It is strong evidence, not a mathematical proof of the whole module, and the report says exactly which parts are proven. |
| What about places where our C has undefined behavior or bugs? | We find them, list them, and you decide how each is handled. The default is a safe error return. We never change behavior silently. |
| Will it be slower or bigger? | We measure speed, size, stack and RAM per module and agree a budget in the contract before work starts. |
| Does it work with our toolchain and chip? | The first release supports GCC and Arm Cortex-M, delivering the Rust as a library behind your existing C interface. IAR and Keil builds, and more architectures, follow; we confirm fit during scoping. |
| Can our engineers maintain Rust? | The output is readable and documented, with every C function mapped to its Rust counterpart. Training for your team is included. |
| Do we have to migrate everything? | No. Start with your riskiest modules; C and Rust live side by side for as long as you like. |
| Does our code leave our network? | Not with on-prem or air-gapped deployment, which uses locally hosted models. |
| Is our code used to train AI? | No, and that is in the contract. |
| Who owns the output? | You do: code, tests and evidence. |
| What if a bug shows up later? | Certified modules carry a warranty period in which we fix divergences at no cost, and the CI check catches regressions as your code evolves. |
| Will this make us CRA compliant or certified? | No tool can. It removes a class of vulnerabilities and supplies evidence for your technical documentation; your assessor makes the call. |
| Can we use a qualified Rust compiler? | Yes. The output builds with Ferrocene, the qualified Rust toolchain. |
| How long does it take and what does it cost? | A scoping report gives a fixed price and schedule per module before you commit. |
| What if you go out of business? | No lock-in: the Rust, tests, harnesses and evidence run without our tools, and source escrow is available for the tools themselves. |
| How is this different from an AI coding assistant? | Assistants translate. We prove equivalence, manage the undefined-behavior decisions with you, support embedded targets, and stand behind the result. |
| What about interrupts, concurrency and hardware registers? | The first release covers parsers and protocol logic. Hardware-facing code comes later, once we can model your peripherals. |
| Why trust a startup with this? | Public benchmark results, evidence anyone can re-run, and a paid pilot on a module you pick. |

## Pricing and packaging hypotheses

Charge for verified outcomes, not tokens or seats, and anchor every price to what the customer would spend rewriting the module by hand. All figures below are hypotheses to test in discovery interviews.

| Package | What's included | Pricing hypothesis | Available |
| --- | --- | --- | --- |
| Scoping assessment | Risk map, UB pre-scan, per-module feasibility, fixed quotes | Fixed fee, roughly $15k–$30k, credited toward a migration | MVP |
| Paid pilot | One module migrated, integrated and fully evidenced | Fixed fee, roughly $25k–$50k | MVP |
| Module migration | Translation, validation, integration, evidence bundle, warranty period | Per module by complexity tier, targeting 30–50% of the customer's own manual-rewrite estimate | v1 |
| Continuous equivalence | CI checks, evidence kept current as code changes, ongoing warranty | Annual subscription per repository | v1 |
| Platform license | On-prem tooling for teams running their own migrations, plus support | Annual license per site or team | v2 |
| Add-ons | Tool-qualification kit, support for a new chip family, Rust training | Fixed fee each | v2 |

**Pricing rules:**

- Price by complexity tier, never per line of code alone; per-line pricing rewards bloated output.
- Keep model and compute costs out of customer-facing prices; they belong in your margin math.
- Ask every interviewee for their own estimate of manual rewrite cost per module. That number sets your ceiling.

## Success metrics, non-goals, risks and open questions

The one metric that matters most is the share of modules that reach full evidence with zero divergences; every other number supports it.

**Product metrics**

- Share of modules reaching full evidence with zero divergences on valid inputs.
- Evidence strength: branch coverage of the original C, and the share of functions with symbolic or bounded proofs.
- Remaining `unsafe` code, and resource regression versus the C.
- Human hours per 1,000 lines migrated, and days from intake to signed-off evidence.
- Scoping accuracy: quoted versus actual effort.

**Business metrics:** pilot-to-contract conversion, average contract value, gross margin per module, and the share of migrated modules that move onto a continuous-equivalence subscription.

**Non-goals for v1:** C++ (C only), whole-firmware rewrites, interrupt handlers and drivers, worst-case timing guarantees, certifying products on a customer's behalf, and general-purpose coding-assistant features.

| Risk | Mitigation |
| --- | --- |
| AI labs and coding agents make translation nearly free | Compete on evidence, embedded depth and on-prem deployment; keep the pipeline model-agnostic |
| Established tool vendors add this as a feature | Move first on the evidence standard; treat them as partners or likely acquirers |
| Evidence turns out weaker than claimed | Conservative claims, coverage reported honestly, mutation-testing the harness itself, an external audit of the method |
| Legacy C is so full of undefined behavior that "equivalent" gets murky | Explicit UB policy with customer sign-off on every decision |
| Long sales cycles in regulated industries | Services-led pilots; start with smaller device makers before large ones |
| A missed divergence causes a field failure | Liability caps, a clearly scoped warranty, insurance, staged rollout behind the existing C interface |
| One founder with a day job runs out of bandwidth | Keep scope narrow; look for a cofounder with embedded or security depth |

**Open questions to resolve in discovery**

- [ ] Which first vertical and RTOS: industrial IoT on Zephyr, or consumer IoT on FreeRTOS?
- [ ] What evidence do assessors and notified bodies actually accept?
- [ ] Can locally hosted models translate well enough, at acceptable cost, for on-prem customers?
- [ ] Where do prices land relative to customers' own rewrite estimates?
- [ ] Is "return an error" an acceptable default for undefined-behavior inputs?
- [ ] How hard is linking Rust libraries into IAR and Keil builds?

## Roadmap: MVP, v1, v2

Ship a services-backed MVP for parsers in 90 days, self-serve tooling by month 9, and hardware-facing support by month 18, each phase gated on paying customers.

**Roadmap diagram (transcribed): "Each phase starts only after its gate passes."** Months from start; the MVP is the current focus.

- **MVP (months 0–3):** parsers on Cortex-M, GCC; translate with c2rust + agents; differential fuzzing + KLEE; UB detection with error default; evidence report v0; services-led pilots.
- **Gate: Day-90 go/no-go:** all planted bugs caught; 70% of parsers fully evidenced; 2 paid pilots or LOIs.
- **v1 (months 4–9):** Kani bounded proofs; UB manager + review UI; CI equivalence checks; on-target resource gates; on-prem with local models; SBOM + CRA-ready evidence; Ferrocene, IAR, Keil builds.
- **Gate: v1 gate:** 3 paying customers; first subscription renewal.
- **v2 (months 10–18):** drivers via device models; interrupts and concurrency; RISC-V and more targets; air-gapped hardening; tool-qualification kit; platform license; SOC 2, ISO 27001.

Nothing in v1 starts until the day-90 gate passes; if it fails, narrow the niche or the module class rather than adding features.
