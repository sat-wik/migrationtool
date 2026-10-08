# Conventions

Full rules: `docs/guidelines/building-the-tool.md` (the tool's own code) and `docs/specs/emitted-rust-rules.md` (the Rust the tool generates). Changing either needs a recorded decision in `.planning/PROJECT.md`.

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
