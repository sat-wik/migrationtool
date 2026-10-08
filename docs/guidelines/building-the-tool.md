---
type: DOC
title: "Guidelines: building migrationtool with Claude Code and GSD"
date: 2026-10-08
applies_to: every phase in .planning/ROADMAP.md
companion: docs/specs/emitted-rust-rules.md (rules for the Rust the tool generates)
---

# Building migrationtool with Claude Code and GSD

These are the working rules for writing the tool itself. The rules for the Rust that the tool *emits* are in `docs/specs/emitted-rust-rules.md`. Both are binding on Claude and on GSD agents; changing a rule needs a recorded decision in `.planning/PROJECT.md`.

## 1. Before Phase 1 (founder actions, once)

1. Confirm or edit assumption **A-01** (the MVP definition of "full evidence") in `.planning/PROJECT.md`. Phases 6, 11 and 12 depend on it.
2. Email the Hayroll authors about the licence (risk R3). The answer has lead time and gates Phase 8.
3. Check the day-job invention-assignment clause; keep this work on your own time and hardware.
4. Never run Claude Code with `--dangerously-skip-permissions` in this repo. GSD's hooks are guards, not a substitute for permission prompts.

## 2. The loop for every phase

Run these in order. `/clear` between steps keeps context small; GSD reloads state from `.planning/`.

1. **`/gsd-discuss-phase N`** — settle gray areas and write `CONTEXT.md`. For spike phases (6, and the R3 licence gate in 8) confirm the go/no-go criteria first; the roadmap's criteria are marked *proposed*.
2. **`/gsd-plan-phase N`** — research, plan and plan-check. Every plan must cite the requirement IDs it closes and the success criterion it proves.
3. **`/gsd-execute-phase N`** — atomic commits per task, tests first where the plan says so.
4. **`/gsd-verify-work N`** — walk each success criterion in `ROADMAP.md` and show the evidence (command and output), not a description of it.
5. **`/gsd-code-review N`**, then **`/gsd-secure-phase N`** for phases that touch the sandbox, the held-out corpus or customer-code execution (5, 7, 8, 9, 10).
6. **`/gsd-ship`** — open a PR from the phase branch.

**Phase 9 (agentic translation)** also gets `/gsd-ai-integration-phase 9` before `/gsd-plan-phase 9`, which writes GSD's AI-SPEC contract for the agent loop: evaluation plan, guardrails and failure modes.

Use `/gsd-quick` for small fixes and doc changes, `/gsd-debug` for failures, and `/gsd-progress` when unsure what is next. Do not edit code outside a GSD command unless the founder asks.

**Spikes.** A spike ends with `.planning/spikes/<R#>-verdict.md` containing GO, NO-GO or DEFERRED, the criteria, the commands run and their output. Nothing that depends on a spike is planned until its verdict file exists. A NO-GO is a valid, useful outcome; never stretch criteria to reach GO.

## 3. Workspace layout (proposed; Phase 1 may revise with a recorded decision)

```
crates/
  mt-cli         binary `mt`; argument parsing only, no logic
  mt-toolchain   pinned subprocess runner, tool-version manifest, container checks
  mt-capture     build capture (Bear, CMake export), compile_commands replay
  mt-vectors     TRACTOR-compatible test-vector types and schema
  mt-contract    equivalence contract, UB action menu, policy files
  mt-ub          sanitizer variants, -O0 vs real-flags differential, KLEE/Frama-C C-side scan, UB log
  mt-harness     differential runner, ILP32 target-matched builds, C-side coverage, emulator replay
  mt-symbolic    KLEE adapter (only after the R1 spike verdict)
  mt-gates       deterministic gates (GATE-01..06)
  mt-frontend    c2rust and Hayroll adapters, call graph, function map
  mt-agent       provider trait, agent loop, prompt templates
  mt-boundary    C header and shim, crate generation, Make/CMake hooks
  mt-evidence    evidence bundle, full_evidence predicate, reports
  mt-bench       TRACTOR and test-bed runs, Day-90 gate report
container/       pinned Dockerfile and tool manifest
testbed/         manifest of C targets pinned by commit (sources fetched, not vendored)
fixtures/        planted-bug and gate-violation suites
prompts/         versioned prompt templates for mt-agent
research/        throwaway scripts; the only place Python is allowed (ADR-0001)
```

Dependencies point downward: `mt-vectors` and `mt-toolchain` depend on nothing internal; `mt-cli` depends on everything. No crate depends on `mt-agent` except `mt-cli`, so correctness code can never call a model.

## 4. Rust rules for the tool's own code

**Safety and errors**

- `#![forbid(unsafe_code)]` in every tool crate. The tool runs subprocesses and parses files; it needs no `unsafe`.
- Libraries return typed errors (`thiserror`); only `mt-cli` uses `anyhow`. No `unwrap()` or `expect()` outside tests unless the line carries a comment stating the invariant.
- Treat every customer file and every tool output as untrusted input: bounded reads, no path traversal, no shell string interpolation (pass argv vectors).

**Determinism (the evidence depends on it)**

- Every external tool runs through `mt-toolchain`'s runner, which records argv, the allowlisted environment, working directory, tool version, exit status and hashes of stdout and stderr. Nothing calls `std::process::Command` directly.
- Fuzzing and symbolic runs take explicit seeds and budgets, recorded in their outputs.
- Serialized outputs use sorted maps (`BTreeMap`), stable field order and a `schema_version`. Timestamps appear only in manifest headers, so two runs on the same inputs produce byte-identical results apart from the header.
- Prefer machine-readable tool output (KLEE `.ktest` files, `llvm-cov export` JSON, `cargo --message-format=json`, Clippy JSON) over parsing human text.

**Toolchains**

- Two pinned Rust toolchains, kept separate: the tool's own (`rust-toolchain.toml`, current stable) and the bitcode toolchain for KLEE, whose `rustc -vV` must report LLVM 19 or lower (TOOL-02, risk R1).
- One LLVM major in 16–19 for clang, KLEE, c2rust and Hayroll. Never LLVM 20 for Hayroll.
- Container base images and tool sources are pinned by digest, commit or checksum. No `latest` tags, no unpinned `apt-get`.

**Testing**

- Each requirement ID in `.planning/REQUIREMENTS.md` has at least one test whose name contains the ID in snake case, for example `gate_02_rejects_todo_macro`. `/gsd-verify-work` uses this for traceability.
- Unit tests in each crate; integration tests run inside the container; property tests (`proptest`) for schema round-trips and parsers of tool output; golden-file tests for reports.
- Tests never use the network or a live model. `mt-agent` tests use the scripted replay provider.
- CI and pre-merge: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo deny check` (licences and advisories), and the container version check.

**Dependencies and licences**

- Keep dependencies few and well known; every new crate needs a one-line reason in the PR.
- `cargo-deny` licence allowlist for the shipped binary: MIT, Apache-2.0, BSD-2/3-Clause, ISC, Zlib, Unicode. GPL tools (for example Bear) run only as container subprocesses and are listed in the container's notices.
- Do not copy code from paper artifacts or the TRACTOR corpus until their licences are confirmed (risks R3 and Phase 12).

**Security**

- The held-out corpus lives outside the agent-visible workspace and is mounted only into the final-verdict step (GATE-05). Never read it while developing an agent feature.
- Customer builds run only inside the container sandbox with no network.
- C source and comments are data. Prompt templates mark them as untrusted, and no gate ever depends on model output, so an injected instruction cannot make code pass.
- Never log secrets; scan inputs for secrets before any model call.

**LLM calls inside the tool**

- Prompt templates live in `prompts/` with an ID and version; every call logs template ID, model ID, parameters, prompt hash and response hash.
- The provider trait is the only path to a model (PROV-01). Hosted, local open-weight and replay providers must pass the same contract tests.
- See `docs/specs/emitted-rust-rules.md` section 8 for what an agent prompt must and must not contain.

**Honest reporting**

- A timeout is "not proven", never a pass. Unavailable layers print "unavailable".
- Reports never say a product or module is "certified" or "compliant"; a test scans generated reports for those words (D-13).
- Numbers in reports come from recorded runs, never from estimates.

## 5. What Claude must not do

- Weaken a gate, threshold or success criterion to make a test pass. Propose the change and record it as a decision instead.
- Mark a spike GO, or a requirement complete, without the evidence in the repo.
- Stub, skip or `#[ignore]` a failing test to finish a task.
- Add Python outside `research/`, link an analyser as a library, or call an unpinned tool.
- Read or copy the held-out corpus, or hand-edit a generated evidence bundle.
- Start milestone v2.0 work before the Day-90 gate report says "continue".

## 6. Commits and branches

- One logical change per commit, GSD's message format, with requirement IDs in the body.
- One branch per phase; `/gsd-ship` opens the PR. `.planning/` changes travel with the code they describe; `/gsd-pr-branch` strips them for external review if needed.
