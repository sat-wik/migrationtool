<!-- GSD:project-start source:PROJECT.md -->

## Project

**migrationtool**

A Rust tool that migrates a device maker's riskiest embedded C modules into safe, `no_std`, allocation-free Rust behind the existing C interface, and ships each module with a re-runnable evidence bundle showing the Rust behaves like the C. Translation (c2rust + Hayroll + LLM agents) is treated as a commodity; the validation harness and the evidence it produces are the product. The first beachhead is hardware-independent parsers and protocol handlers (TLV, CBOR, MQTT, BLE, OTA image headers) on Arm Cortex-M built with arm-none-eabi GCC. Builder: one founder working nights and weekends with Claude Code and GSD.

**Core Value:** Every migrated module ships with evidence, produced by a harness that is itself proven to catch planted bugs, that the Rust matches the C on every input where the C has defined behavior. Developer-facing success metric: the share of C modules that reach full evidence with zero divergences on defined-behavior inputs.

### Constraints

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

<!-- GSD:project-end -->

<!-- GSD:stack-start source:STACK.md -->

## Technology Stack

Technology stack not yet documented. Will populate after codebase mapping or first phase.
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->

## Conventions

## Phase loop

- Every phase runs: `/gsd-discuss-phase N` → `/gsd-plan-phase N` → `/gsd-execute-phase N` → `/gsd-verify-work N` → `/gsd-code-review N` (plus `/gsd-secure-phase N` for phases 5, 7, 8, 9, 10) → `/gsd-ship`.
- Phase 9 also runs `/gsd-ai-integration-phase 9` before planning.
- Spikes end with `.planning/spikes/<R#>-verdict.md` (GO, NO-GO or DEFERRED, with commands and output); nothing depending on a spike is planned before its verdict exists.
- Never run with `--dangerously-skip-permissions`.

## Tool code (Rust, ADR-0001)

- `#![forbid(unsafe_code)]` in every tool crate; `thiserror` in libraries, `anyhow` only in `mt-cli`; no `unwrap`/`expect` outside tests without an invariant comment.
- Every external tool runs through the `mt-toolchain` runner (argv, env allowlist, version, exit status, output hashes recorded); never call `std::process::Command` directly.
- Deterministic outputs: explicit seeds and budgets, `BTreeMap`, `schema_version`, timestamps only in manifest headers.
- Two pinned Rust toolchains: the tool's own, and a KLEE bitcode toolchain whose `rustc -vV` reports LLVM ≤ 19. One LLVM major in 16–19 for clang, KLEE, c2rust and Hayroll.
- Each requirement ID has a test whose name contains it in snake case (e.g. `gate_02_rejects_todo_macro`); tests never use the network or a live model.
- Pre-merge: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo deny check`, container version check.
- Python only under `research/`; analysers only as subprocesses; held-out corpus never read outside the final-verdict step.
- Reports: timeouts are "not proven", missing layers "unavailable", and never the words "certified" or "compliant".

## Emitted Rust (the product's output)

- `#![no_std]`, no heap, builds for `thumbv7em-none-eabihf`, `panic = "abort"`, staticlib + rlib, pinned toolchain, no dependencies by default.
- `unsafe` only in `ffi.rs`, each block preceded by `// SAFETY:`; `#[repr(C)]` structs with compile-time size and `offset_of!` assertions; `core::ffi` types (`c_char` is `u8` on this target).
- Behavior matches the C at the interface on defined-behavior inputs, including partial writes on error paths; UB sites take the action in the UB decisions log (default: return error).
- Explicit arithmetic (`wrapping_*` for unsigned, `checked_*` plus UB policy for signed and shifts); explicit endianness; exact float widths; no `core::fmt`.
- No `unwrap`, `expect`, `panic!`, `unreachable!`, `todo!`, `unimplemented!` or empty value-returning bodies.
- Stage 1 faithful, stage 2 idiomatic in small Rust-to-Rust steps; `ffi.rs` signatures and boundary-visible data never change in stage 2.
- Agent prompts never contain held-out vectors; an agent's claim of success is never evidence.

<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->

## Architecture

Architecture not yet mapped. Follow existing patterns found in the codebase.
<!-- GSD:architecture-end -->

<!-- GSD:skills-start source:skills/ -->

## Project Skills

No project skills found. Add skills to any of: `.claude/skills/`, `.agents/skills/`, `.cursor/skills/`, `.github/skills/`, or `.codex/skills/` with a `SKILL.md` index file.
<!-- GSD:skills-end -->

<!-- GSD:workflow-start source:GSD defaults -->

## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd-fast` for a trivial task inline, with no subagents and no PLAN.md
- `/gsd-quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd-debug` for investigation and bug fixing
- `/gsd-execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->

<!-- GSD:profile-start -->

## Developer Profile

> Profile not yet configured. Run `/gsd-profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->
