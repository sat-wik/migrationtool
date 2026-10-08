---
type: SPEC
title: "Rules for the Rust that migrationtool emits"
date: 2026-10-08
status: v0, MVP (milestone v1.0)
enforced_by: mt-gates (GATE-01..06), mt-harness (DIFF), mt-boundary (BOUND), mt-agent (XLATE)
---

# Rules for the Rust that migrationtool emits

This is the code-generation contract for every crate the tool produces. The agent prompt includes it, the deterministic gates enforce it, and the evidence bundle records which rules were checked. A module ships only if every **MUST** holds. Section 9 maps each rule to the gate or requirement that enforces it.

The target for v0 is Arm Cortex-M, `thumbv7em-none-eabihf`: ILP32, little-endian, plain `char` unsigned, built with arm-none-eabi GCC on the C side.

## 1. Crate shape

- MUST be `#![no_std]` and never use `alloc`, `Box`, `Vec`, `String` or any heap API (GATE-01).
- MUST build for `thumbv7em-none-eabihf` with the crate's pinned `rust-toolchain.toml` (GATE-01, GATE-06).
- MUST be `crate-type = ["staticlib", "rlib"]` with `panic = "abort"` in both dev and release profiles (BOUND-02, BOUND-04).
- MUST provide a `#[panic_handler]` behind a default-on `panic-handler` feature. A firmware image can hold only one, so when several migrated modules link into one image they are combined into one staticlib with the feature on once.
- MUST use edition 2021 unless the customer's pinned toolchain is 1.85 or newer. Ferrocene releases trail upstream, so the edition and minimum Rust version follow the customer's compiler, not ours.
- MUST have no dependencies by default. Any dependency needs founder approval, an allowlist entry and an SBOM line (GATE-06).
- SHOULD build with `opt-level = "s"` (or match the C's `-Os`/`-O2`), `lto = true` and `codegen-units = 1`; code size is reported against the C.

## 2. Module layout

```
src/
  lib.rs        #![no_std], lint config, re-exports nothing public except ffi
  ffi.rs        the C boundary: the only module allowed to contain unsafe
  <unit>.rs     one Rust module per C translation unit, safe code only
  error.rs      internal error enum and its mapping to the C return codes
```

- Each C translation unit maps to one Rust module of the same stem, and each C function to one Rust function. The function map (FRONT, GATE-04) is the source of truth.
- Every Rust function carries a doc comment naming its origin: `/// C: src/tlv.c:142 tlv_parse_header`. The traceability matrix is generated from these.

## 3. The C boundary (`ffi.rs`)

- MUST export exactly the symbols and signatures of the original header, verified by compiling a C unit against the original header and linking it against the staticlib (BOUND-01).
- MUST use `core::ffi` types (`c_char`, `c_int`, `c_long`, `c_uint`, ...) for every C type. Never hard-code `i8` for `char`: on this target `c_char` is `u8`. `size_t` maps to `usize`.
- MUST mark every struct that crosses the boundary `#[repr(C)]` and assert its layout at compile time against the layout extracted in Phase 4: `const _: () = assert!(core::mem::size_of::<Hdr>() == 12);` plus an `offset_of!` assertion per field.
- MUST check pointer arguments exactly as the C contract requires (null checks the C performs, or the UB policy for null the C does not check) and turn each `(ptr, len)` pair into a slice once, at the boundary. A null pointer with length 0 becomes an empty slice, never a call to `from_raw_parts` with null.
- MUST precede every `unsafe` block with a `// SAFETY:` comment naming the invariant and where it is guaranteed. The gate extracts these into the unsafe ledger (GATE-03).
- MUST NOT panic. Boundary functions call safe internal code that returns `Result`, and map errors to the C function's own return codes. With `panic = "abort"` a panic is a crash, which the harness counts as a divergence.
- C globals that are visible across the boundary stay in `ffi.rs` as `#[no_mangle]` statics with documented access rules. Internal state is passed explicitly, never through new globals.

## 4. Preserving behavior (stage 1: faithful)

The observation point is the C interface: return values, every byte written to output buffers (including partial writes on error paths), and boundary-visible global state must match the C on every defined-behavior input. Internals may differ.

**Integers**

- MUST make every arithmetic operation explicit. Unsigned C arithmetic wraps, so use `wrapping_add`, `wrapping_mul` and similar. Signed overflow is undefined behavior in C, so use `checked_*` and apply the UB policy on `None`. The emitted crate denies `clippy::arithmetic_side_effects` so a bare `+` cannot slip through, and never depends on debug versus release overflow checks.
- MUST reproduce C's integer promotions and usual arithmetic conversions explicitly. A value-changing `as` cast needs a comment naming the C conversion it mirrors (`// C: (uint8_t) truncation`); value-preserving conversions use `From` or `TryFrom`.
- Shifts by the bit width or more, and left shifts of negative signed values, are undefined behavior in C: use `checked_shl` and `checked_shr` and apply the UB policy.

**Bytes, structs and endianness**

- MUST decode multi-byte fields with explicit `from_le_bytes` or `from_be_bytes` matching what the C does. Never `transmute` a byte buffer into a struct.
- Bounds checks: prefer `get`, `split_at_checked`, `chunks_exact` and slice patterns. Direct indexing is allowed in stage 1 only where a check in the same function proves the bound; the idiomatic stage denies `clippy::indexing_slicing`.

**Floating point**

- MUST keep the C's float widths exactly, including `float` to `double` promotion in expressions and library calls. ORBIT saw `f32::powf` diverge.
- MUST record the C build's `-ffp-contract` setting. Rust never fuses multiply-add, while GCC may with an FPU; if the C build fuses, the contract records the tolerance or the function stays in C. The harness compares floats bit for bit unless the contract says otherwise.

**Errors and undefined behavior**

- MUST return the same error codes, in the same situations, as the C, and write output parameters on error paths exactly as the C does.
- On an input or code site listed in the UB decisions log, MUST take the recorded action: return the policy error (default), panic (only if the customer chose it) or preserve. With no recorded decision the module cannot ship (UB-05).
- MUST NOT call `unreachable!`, `unwrap`, `expect`, `panic!`, `todo!` or `unimplemented!`, and MUST NOT leave an empty body in a function that returns a value (GATE-02).

## 5. Stage 2: idiomatic refactoring

- Only Rust-to-Rust transformations, one small step at a time; each step is checked against the previous step and against the C at the boundary, and a failing step is reverted (XLATE-06).
- Allowed: iterators instead of index loops, slices instead of pointer arithmetic internally, internal error enums mapped to C codes at the boundary, newtypes, `match` instead of `if` chains, removing code the C-side coverage shows is unreachable.
- Not allowed: changing anything in `ffi.rs` signatures, changing observable error codes or write order, adding `alloc` or a dependency, changing arithmetic semantics, or changing the representation of data that crosses the boundary or lives in global state (abstraction functions for that are deferred, assumption A-06).
- Each module is labelled `tier = "faithful"` or `tier = "idiomatic"` in its manifest; the evidence report states the tier.

## 6. Style, size and lint configuration

- MUST be rustfmt-clean and Clippy-clean with warnings denied, with docs on every public item (GATE-06).
- MUST NOT use `core::fmt` machinery (`write!`, `format_args!`, `Debug` printing) in non-test code; it costs flash on Cortex-M and the C had no equivalent.
- `#[inline(always)]` only where the C function was `static inline`.

Every emitted `Cargo.toml` carries this lint table (Cargo `[lints]`, Rust 1.74+):

```toml
[lints.rust]
unsafe_code = "deny"              # ffi.rs alone carries #[allow(unsafe_code)]; GATE-03 checks no other allow exists
unsafe_op_in_unsafe_fn = "deny"
missing_docs = "deny"

[lints.clippy]
all = { level = "deny", priority = -1 }
undocumented_unsafe_blocks = "deny"
arithmetic_side_effects = "deny"
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
todo = "deny"
unimplemented = "deny"
unreachable = "deny"
std_instead_of_core = "deny"
alloc_instead_of_core = "deny"
indexing_slicing = "warn"         # "deny" for tier = "idiomatic"
cast_possible_truncation = "warn" # each allowed cast carries a "// C:" comment
cast_sign_loss = "warn"
cast_possible_wrap = "warn"
```

## 7. Safety-certified customers (output profile, milestone v2.0)

For customers on Ferrocene (risk R4): the crate's toolchain file pins the `rustc` inside their Ferrocene release, and a `ferrocene-core` profile restricts generated code to the certified subset of `core`. The gate checks every `core` path used against that release's certified-function list. Until v2.0 this profile is off and reports say so.

## 8. What the agent prompt contains

The translation prompt for one C function MUST include:

- the C function and the declarations it uses, marked as untrusted data;
- the Rust signatures of its already-translated callees and its function-map entry;
- the UB-log entries and implementation-defined facts for this function (target ILP32, unsigned `char`, struct layouts, `-ffp-contract`);
- the equivalence contract and a compressed copy of sections 1–6 of this document;
- on retry, the failing gate message or the counterexample vector from the development corpus.

It MUST NOT include held-out vectors, other customers' code, or any hint about which inputs the final verdict uses (GATE-05, XLATE-03). An agent's claim that its code works is never evidence; only gates and the harness decide (XLATE-04).

## 9. Enforcement map

| Rule | Enforced by | Requirement |
| --- | --- | --- |
| `no_std`, no heap, builds for the target | mt-gates build and symbol check | GATE-01 |
| No stubs, `todo!`, empty bodies, panicking macros | mt-gates AST scan plus Clippy | GATE-02 |
| `unsafe` only in `ffi.rs`, every block justified | mt-gates scan, unsafe ledger | GATE-03 |
| Every C function mapped and exercised | mt-gates with C-side coverage | GATE-04 |
| Held-out corpus never visible to agents | sandbox mounts, negative test | GATE-05, XLATE-03 |
| rustfmt, Clippy, docs, pinned toolchain, dependency allowlist | mt-gates | GATE-06 |
| Symbols and signatures match the original header | mt-boundary link test | BOUND-01 |
| `panic = "abort"`, panic handler, UB input returns policy error | mt-boundary test | BOUND-02, BOUND-04 |
| Behavior at the C interface matches on defined-behavior inputs | mt-harness differential run (ILP32, `-funsigned-char`) | DIFF-01..07 |
| UB sites take the recorded action | mt-harness checking `has_ub` vectors against the policy | DIFF, UB-05 |
| Faithful-then-idiomatic, steps reverted on failure | mt-agent loop | XLATE-05, XLATE-06 |
| Layout assertions, explicit arithmetic, float width, endianness | Clippy lints above plus harness | GATE-06, DIFF |
