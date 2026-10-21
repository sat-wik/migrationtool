# Phase 3: Equivalence Contract and Test-Vector Schema - Research

**Researched:** 2026-10-21
**Domain:** Rust harness crates (`mt-contract`, `mt-vectors`, executor + adapters), TRACTOR vector format, qemu-arm user-mode execution of captured Cortex-M objects, DWARF-derived ILP32 state layout
**Confidence:** MEDIUM-HIGH (design measured end to end on this host with qemu-arm 8.2.2 and arm-none-eabi-gcc 13.2.1; the pinned image has QEMU 7.2 and GCC 14.3, so the CI smoke step in Plan 03-03 must repeat the key measurements)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

Carried forward, not re-asked:

- **Refinement:** equivalence is refinement, with "return error" as the default UB action (PROJECT D-11).
- **Schema fields:** exactly the nine TRACTOR fields, including two lib_state fields (PROJECT constraint).
- **Licences:** the TRACTOR corpus licence is unchecked, so nothing from it is committed or redistributed (PROJECT legal constraint).
- **Process rules:** every subprocess goes through the `mt-toolchain` runner (D-16); deterministic outputs; tests never use the network.
- **Crate plan:** `docs/guidelines/building-the-tool.md` already names `mt-vectors` (schema) and `mt-contract` (contract, UB menu, policy).

#### Reference build and executor

- **D-01: The reference result comes from the real Cortex-M code.** The executor links the exact objects Phase 2 captured and replayed for a target/config into a small generated driver, built with the pinned arm-none-eabi toolchain and that config's flags. It runs the driver under the pinned `qemu-arm` user mode. "What the C does" means what the shipped firmware code does on the target. Phase 5 may fuzz on a faster host build, but this executor stays the judge. — **Reversibility:** costly — Phase 5 replay, the evidence bundle and every stored vector assume this reference.
- **D-02: Fixed sandbox and budgets.** Each run goes through the `mt-toolchain` runner with:
  - no network;
  - an empty, fixed environment;
  - a fixed working directory;
  - a per-vector time budget and memory budget.

  A timeout or crash is recorded as the result, never retried, and never counted as a pass. Every vector is executed twice, and the two results must be byte-identical or the run fails.
- **D-03: A crash or fault in the C is recorded as the outcome, with `has_ub = true`.** The signal or fault goes into `rc`/`stderr` as the outcome class. A crashing input is out of contract, so the vector is excluded from output comparison. Phase 4's UB tools confirm or refine the label later.
- **D-04: Guard pages around buffers.** The input buffer is placed so its last byte sits immediately before an unmapped guard page, with another guard before its first byte. The output buffer gets the same treatment beyond its capacity. An out-of-bounds read or write then faults under QEMU and becomes a D-03 outcome, instead of reading leftover memory, which would break determinism.

#### Mapping a library call onto TRACTOR's fields

- **D-05: Fixed field convention, written once in the contract.**

  | Field | Contents |
  |---|---|
  | `argv[0]` | Names the target, config and entry |
  | `argv[1..]` | The scalar arguments (lengths, flags, capacities) as decimal text, in the order the adapter declares |
  | `stdin` | The input buffer bytes |
  | `stdout` | The output buffer bytes after the call (see D-07) |
  | `rc` | The C return value |
  | `stderr` | Only the executor's own diagnostics (outcome class: ok / fault / timeout, plus detail) |
  | `env` | Always empty |

  Vectors stay readable by TRACTOR's own tools. — **Reversibility:** costly — every stored vector and the comparison rule depend on it.
- **D-06: lib_state is field by field.** `lib_state_in` and `lib_state_out` are JSON objects of named fields, each carrying its C type and an exact value:
  - integers as decimal;
  - byte arrays as hex;
  - pointers as `{buffer, offset}` into a named buffer (input, output, or a state struct), never as raw addresses.

  This stays stable across builds and lets later phases compare the Rust through the abstraction functions the spec calls for. — **Reversibility:** costly — the schema and every comparator depend on it.
- **D-07: The whole output buffer is compared, after a sentinel prefill.** Before the call, the executor fills the output buffer with a fixed sentinel byte. It then records the entire buffer, so bytes the C left untouched and partial writes on error paths are both part of the result. This matches the emitted-Rust rule on partial writes.
- **D-08: The fields compared are `rc`, `stdout` and `lib_state_out`, plus the stderr outcome class.** These are the observation point at the C interface. The stderr text is not compared, only its class (ok / fault / timeout). A vector with `has_ub = true` is excluded from output comparison and checked only for the policy's safe action. The contract states this rule.

#### Library state contents

- **D-09: State structs are recorded with all their fields.** A struct passed by pointer (for example tinycbor's `CborParser` and `CborValue`) is recorded with every field its header defines, using the layout of the compiled C for ILP32 (from DWARF or clang's AST). The caller owns that memory, so all of it crosses the boundary. A field can be marked unobserved only with a recorded reason, and the contract counts such marks.
- **D-10: Every writable global is found automatically.** Every symbol in the target's own `.data` and `.bss`, taken from the captured objects' symbol tables, is snapshotted before and after the call. A global changing unexpectedly is caught without anyone declaring it.

#### Adapters

- **D-11: Adapters are declared in the manifest, and the driver is generated.** The existing `[[target.X.entry]]` blocks in `testbed/manifest.toml` are extended with argument roles: input buffer, its length, output buffer and its capacity, flags/scalars, and state struct. `mt` generates the C driver from those roles. The manifest validator rejects bad role declarations, and no hand-written per-target C enters the repo. — **Reversibility:** costly — the manifest schema is shared with Phase 2's validator and later phases.
- **D-12: Entries that need prior state use a declared setup sequence.** Some entries need state before they can run: the six `form = "wrapped"` entries (struct-wrapped buffers per Phase 2 D-28, such as QCBOR `UsefulBufC`, nanopb `pb_istream_t` and zlib `z_stream`), and entries that need an earlier call (tinycbor `cbor_value_validate` needs `cbor_parser_init`). For these, the adapter declares the setup steps that run first, with their arguments drawn from the same vector. The state after setup is `lib_state_in`; the state after the entry is `lib_state_out`. A failed setup is recorded as its own outcome. The existing `accepted_wrapped` flag stays in force.
- **D-13: Adapters cover every target that fits, with a floor of 10 enforced by a test.** All 16 retained targets are attempted. A target whose entry the roles cannot express is logged with the reason, the same way Phase 2 logs drops.
- **D-14: Scalar arguments come from adapter defaults plus an optional short sweep.** Each scalar role has a declared default and an optional short list of values to sweep, for example parser flags 0 and the strict-mode flag. Each seed runs once per declared combination, with a capped total. Fuzzing the scalars is Phase 5's job.

#### Contract and UB policy

- **D-15: Contract v0 is a markdown document plus a TOML policy file.**
  - The document is `docs/specs/equivalence-contract.md`, with a semver version and a changelog.
  - The policy file is a per-module TOML file loaded by the new `mt-contract` crate. It carries `schema_version` and the contract version it implements, and unknown keys are rejected.
  - TOML matches the project's other machine files (manifest, pins).
- **D-16: UB policy is a module default plus per-site overrides.** Each module has a default action, `return error`. Individual UB sites can override it. A site is identified the way Phase 4's UB log identifies it (function, kind, location). An override naming no logged site is rejected. In this phase the override table is defined and validated but empty.
- **D-17: The three UB actions are defined as follows.**
  - **return error:** the function returns a declared error code and makes no further writes.
  - **panic:** abort. Emitted Rust uses `panic = "abort"`, so this is a trap, and the vector records the trap.
  - **preserve:** reproduce what the compiled C did on the target, with that behavior recorded per site as evidence.

  Under every action, the Rust never reads or writes out of bounds and never corrupts memory.
- **D-18: Every override is a full record.** Each entry names the site, the new action, the reason, the approver and the date. The policy loader refuses an override missing any of them. Reports list every override, so an evidence bundle shows where the default was changed.

#### Vectors, storage and the TRACTOR corpus

- **D-19: Provenance lives in a sidecar index, so vectors stay exactly nine fields.** Each vector's sha256 maps, in the corpus `index.json`, to a provenance record:
  - target, config and entry;
  - hashes of the build objects;
  - the image digest;
  - the executor version and the budgets used;
  - the source of `has_ub`: observed crash, unchecked, or Phase 4.
- **D-20: `has_ub` before Phase 4 is `false`, with its source recorded as unchecked.** A run that did not crash gets `has_ub = false`, and its provenance says UB detection has not run. Reports call such vectors "UB unchecked", never "no UB". Phase 4 re-labels them. Only crash-observed vectors are `true` in this phase.
- **D-21: Vectors are built in the work directory, with a small sample committed.** Vectors are build outputs in `.mt-work`, named by sha256 with an index, like the seed corpora. A small committed sample backs the offline tests: one vector per drivable target, made from the project's own seeds, so no licence issue arises. CI regenerates the full set and checks determinism.
- **D-22: Phase 3 builds vectors from seeds only.** It runs the existing Phase 2 seed corpora through the executor. Fuzzing-generated and mutated inputs belong to Phase 5.
- **D-23: The TRACTOR corpus is fetched pinned and tested locally.**
  - A pinned commit of `DARPA-TRACTOR-Program/PUBLIC-Test-Corpus` is fetched into the work directory and never committed.
  - An ignored local test, plus a CI step, loads at least 3 of its vectors unchanged and round-trips them.
  - The repo records only the commit, the file paths and their sha256 hashes.

#### Sizing

- **D-24: The planner holds the phase to its size by splitting plans and putting the floor first.** The plans cover:
  - the contract and schema (including TRACTOR loading);
  - then the executor and adapters, with 10 drivable targets as the success line.

  The remaining targets are attempted within the same budget. Any not done are logged with their reason rather than growing the phase. No new roadmap phase is added.

### Claude's Discretion

- **Executor internals:** how the driver is generated and linked (startup code, newlib with semihosting or a minimal stub). The constraints are the pinned toolchain, the shipped `.specs` names (02-21 `SHIPPED_SPECS`) and determinism.
- **Fixed values:** the exact budget values, the sentinel byte value and the guard-page mechanism under qemu-arm user mode.
- **CLI shape:** for example `mt vectors validate|run|check`.
- **Layout source:** how struct layout is obtained (DWARF from a `-g` build of the same objects versus clang's AST), provided it is exact for the target's ILP32 layout and matches the captured objects.
- **Crate split:** how `mt-vectors`, `mt-contract` and the executor split across crates, following `docs/guidelines/building-the-tool.md`.

### Deferred Ideas (OUT OF SCOPE)

- **Mutation and fuzz generation** of vector inputs and scalar arguments: Phase 5.
- **A host ILP32 build** as a fast differential back end, compared against this phase's QEMU reference: Phase 5.
- **Per-site UB overrides with real entries,** which need the Phase 4 UB log.
- **The TRACTOR corpus licence check,** before any redistribution: Phase 12.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| EQV-01 | Contract v0 as versioned document + machine-readable policy (refinement, observation point, UB menu, never-crash rule) | Section "Q7 Contract document and policy file"; policy TOML schema with validation rules; `mt-contract` crate |
| EQV-02 | Vector schema with exactly nine fields, round-trip, rejects merged `lib_state`, `has_ub` excluded from comparison | Section "Q1 TRACTOR vector format" (measured shapes of 2610 public vectors, cando source), D-05..D-08 mapping, `mt-vectors` crate |
| EQV-03 | Per-target adapter with argument roles; at least 10 targets drivable | Section "Q5 Adapter roles" (27-entry inventory with tiers), role vocabulary, manifest extension |
| EQV-04 | C reference executor records expected result in a schema-valid vector; identical on re-run | Sections "Q2/Q3/Q6" (measured qemu-arm user-mode pipeline, link recipe, classification, run-twice) |
</phase_requirements>

## Summary

The whole executor pipeline works as D-01 describes, with no deviation from D-01. On this host I linked captured Cortex-M4F objects (corejson, zlib) with a small generated-style driver using the pinned compiler's flags and ran them under `qemu-arm -cpu cortex-m4` (user mode, QEMU 8.2.2 here; the image has 7.2, whose source also lists `cortex-m4` for user mode). The driver is a Linux-ABI static ELF with its own `_start` and raw `svc` syscalls, not a firmware image, which is why Phase 1's note that "user mode cannot run bare-metal Cortex-M images" does not contradict it. Guard pages (mmap + mprotect from the driver), exact one-byte-overrun faults, an in-guest fault handler (exit status 128+signal, no core files), an own stack, fixed addresses with `-R`, 100 byte-identical re-runs, and about 5 ms per run were all measured. `qemu-system-arm` on `mps2-an386` also works (about 36 ms per run) but cannot give byte-exact guard regions (v7-M MPU needs power-of-two regions of at least 32 bytes), so user mode is the better judge for D-04.

The TRACTOR vector shape differs from what the CONTEXT assumes in four ways that change the schema design. `stdin`, `stdout.pattern` and `stderr.pattern` are JSON strings, not bytes (`stdout` and `stderr` are objects `{pattern, is_regex?}`); `has_ub` is a string or integer label whose mere presence means UB (cando treats even `"has_ub": false` as UB), so our `has_ub = false` must be an absent key; `lib_state_in/out` are free-form JSON objects; and a tenth optional key `note` exists. These are verified from the pinned corpus commit and the cando source. The ILP32 layout must come from the pinned GCC's DWARF, not clang: arm-none-eabi-gcc defaults to 1-byte enums (`-fshort-enums`, `Tag_ABI_enum_size: small` in the captured objects) while clang for `thumbv7em-none-eabihf` used 4-byte enums in my probe.

No new crates are needed (`serde`, `serde_json`, `toml`, `sha2`, `thiserror`, `object` are already locked; DWARF is read from `llvm-dwarfdump` text through the runner). Of the 27 manifest entries, ten targets have a tier A entry (roles only, no setup) with seeds, nine of which link locally today and one (tinycbor) only in CI, and three more targets are tier B, so the floor of 10 has margin. The riskiest unknowns are the QEMU 7.2 and GCC 14.3 behavior in the image (CI-only measurement), the byte encoding of binary data inside TRACTOR's string fields (a founder confirmation), and the DWARF-to-JSON state decoder.

**Primary recommendation:** Build `mt-contract` and `mt-vectors` first (pure Rust, offline), then run a CI smoke of the qemu-arm + driver pipeline on `corejson/JSON_Validate` before building the generic adapter generator; link each pair's objects as a deterministic `ar` archive into a driver with a shared runtime template (own `_start`, raw syscalls, guard pages, own stack), and take struct layouts from `-gdwarf-5` DWARF of the pinned GCC.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Contract document and policy semantics | `mt-contract` (library) | `docs/specs` | Pure data validation; no process execution |
| Vector schema, validation, canonical JSON, corpus index | `mt-vectors` (library) | `mt-cli` | Depends on nothing internal (building-the-tool.md section 3) |
| Adapter role declarations and validation | `mt-capture::manifest` (extend) | `testbed/manifest.toml` | D-11: the manifest is the single source; the validator already lives there |
| Driver generation, link, run, classify | New executor module (recommended: `mt-harness` crate, see Q8) | `mt-toolchain::runner` | Needs mt-capture (objects, flags) and mt-vectors; must be callable by Phase 5 |
| Guest-side behavior (guards, syscalls, fault handler) | Generated C runtime inside the driver, run under qemu-arm | — | Only code that runs on the emulated target |
| Struct layout and state decoding | Executor (host side) reading `llvm-dwarfdump` text and ELF symbols | `object` crate | Layout must come from the pinned GCC to match the captured objects |
| TRACTOR fetch and load | `mt-vectors` types + a small fetch module through the runner | CI network step | Only networked step (D-23) |
| Determinism proof (run twice, compare) | Executor + CI (`testbed.yml`) | — | Image-dependent evidence is read from CI |

## Standard Stack

### Core

| Library / tool | Version | Purpose | Why Standard |
|----------------|---------|---------|--------------|
| serde 1.0.229, serde_json 1.0.151, toml =1.1.7, sha2 0.11.0, thiserror 2.0.21 | locked workspace deps | Vector and policy types, canonical JSON, strict TOML, sha256 | Already used by mt-capture/mt-seeds; no new crates [VERIFIED: Cargo.toml workspace.dependencies, read this session] |
| object =0.40.0 (`read_core`, `elf`, `std`) | locked | ELF symbol table of captured objects and the linked driver (D-10 globals, function-pointer names) | Already in the workspace for section digests [VERIFIED: Cargo.toml] |
| arm-none-eabi-gcc 14.3.Rel1 (pinned), `arm-none-eabi-ar`, `-nm`, `-size` | image pin | Compile the driver, build the object archive, link | Same compiler and flags as the capture (D-01) [CITED: container/pins.toml `[tool.arm_gcc]`, `[source.arm_gcc]`] |
| qemu-user 1:7.2+dfsg-7+deb12u18+b3 (`qemu-arm`) | image pin | Run the driver | D-01 [CITED: container/pins.toml:126-127, 248-252] |
| llvm-dwarfdump (llvm-16 package, `/usr/lib/llvm-16/bin`) | image pin | Text DWARF of a `-gdwarf-5` probe object for struct layout | `llvm-16` is in the apt pin list; tool path includes `/usr/lib/llvm-16/bin` [CITED: container/pins.toml:121, `tool_path`]. That the binary is present is [ASSUMED] (A3) until the CI smoke step checks it |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| clap 4.6.7 | locked | `mt vectors ...` subcommands | In `mt-cli` only |
| tempfile 3.27.0 | locked (dev) | Test work dirs | Tests |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| DWARF from the pinned GCC | `clang -Xclang -fdump-record-layouts-complete` | Rejected: clang laid out `enum` as 4 bytes where GCC lays out 1 byte (measured, Pitfall 2); output also names typedef'd anonymous structs `struct (unnamed at file:line)` |
| `gimli` crate for DWARF | `llvm-dwarfdump` text via the runner | `gimli` is a new crate (legitimacy audit + founder approval); text parse is about 300 lines and uses the pinned tool |
| `proptest` for round-trips | Table-driven tests over a fixed list | `proptest` is not in `Cargo.lock` (new crate); building-the-tool.md mentions it, so ask the founder only if wanted |
| qemu-system-arm `mps2-an386` | qemu-arm user mode | System mode is more faithful for faults and MPU but is about 7x slower and cannot make byte-exact guards (Q2 table). Keep user mode (D-01) |
| semihosting (rdimon.specs) | raw Linux syscalls from a custom `_start` | Both work; the raw-syscall stub needs no host file semantics beyond open/read/write, and is the only way to also get mmap/mprotect for guards (Q2) |

**Installation:** none (no new packages). Local development setup used for this research: `apt-get install qemu-user qemu-system-arm` (Ubuntu 24.04, QEMU 8.2.2); not needed in CI, the image has both.

**Version verification:** `qemu-arm --version` on this host reports 8.2.2 (Debian 1:8.2.2+ds-0ubuntu1.18) [VERIFIED: command run]. The image pins 7.2 [CITED: container/pins.toml:126-127]; not runnable here.

## Package Legitimacy Audit

No new external packages are recommended for this phase. Every dependency is already in the workspace `Cargo.lock`; the slopcheck gate was therefore not run. The two crates a reader might expect (`gimli`, `proptest`) are explicitly not used.

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| (none new) | — | — | — | — | — | — |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```
                 testbed/manifest.toml  (entries + NEW argument roles, setup, unobserved marks)
                          |  validate (mt-capture::manifest, extended)
                          v
 testbed/seeds/<t>/{sha256 files,index.json} --> seed x scalar-sweep combos (cap)         TRACTOR corpus (pinned commit,
                          |                                                                local only, never committed)
 .mt-work/capture/<t>/<c>/{capture.json, objects/**}                                          |
        | verify each object sha256 == capture.json                                           v
        v                                                                          mt-vectors loader: strict nine fields,
 [BUILD, once per (target, config, entry), offline]                                 "load unchanged" round-trip test
   1 layout probe TU: entry header (+TU preamble) + one variable per state type
     gcc -gdwarf-5 (TU's -I/-D/-m/-std flags) -> llvm-dwarfdump text -> LayoutModel
   2 ar rcsD pair.a <captured objects>                         (members bytes unchanged)
   3 driver.c = runtime template + generated call code; gcc -> link with pair.a  -> driver.elf
   4 object crate: symbols of driver.elf -> globals table (.data/.bss of captured objects), function names
        |
        v
 [RUN, once per vector x2]                       mt-toolchain runner (empty env, fixed cwd, timeout)
   write cwd/req.bin (input bytes, scalars, scratch plan)  --> qemu-arm -cpu cortex-m4 -R <va> driver.elf req.bin
        guest: own stack + guard pages -> prefill out buffer -> setup -> ENTRY -> dump state/globals -> write frame on fd 1
        fault: in-guest handler -> exit(128+sig)       timeout: runner kills (timed_out)
        |
        v
 classify (ok | fault | timeout | abort | executor_error)  -> decode frame with LayoutModel
        -> Vector {argv, stdin, env{}, stdout{pattern}, stderr{pattern}, rc, lib_state_in, lib_state_out, [has_ub]}
        -> run again; canonical JSON bytes must be identical, else FAIL
        -> corpus: .mt-work/vectors/<t>/<c>/<entry>/<sha256>.json + index.json (provenance sidecar, D-19)
```

### Recommended Project Structure

```
crates/
├── mt-contract/        # NEW lib: Policy (TOML), Action menu, Override record, load+validate, contract version const
├── mt-vectors/         # NEW lib: Vector types (TRACTOR-compatible), StrictVector validation, LibStateValue model,
│   └── src/{vector.rs,state.rs,bytes.rs,corpus.rs,tractor.rs}   # byte<->string codec, sha256 corpus + index.json, pinned TRACTOR list
├── mt-harness/         # NEW lib (name from building-the-tool.md): adapter model, driver gen, layout (DWARF), link, executor
│   └── src/{adapter.rs,runtime.c.in (include_str!),layout.rs,globals.rs,link.rs,exec.rs,classify.rs}
├── mt-capture/         # EXTEND manifest.rs: Entry gains optional args/setup/unobserved (additive, #[serde(default)])
└── mt-cli/             # ADD `mt vectors ...` group (argument parsing only)
docs/specs/equivalence-contract.md          # contract v0 (semver 0.1.0 + changelog)
testbed/policy/<target>.toml                # per-module policy files (default only; override table empty)
testbed/vectors-sample/<target>/...         # committed sample, one vector per drivable target (D-21)
testbed/tractor-pin.toml                    # commit + paths + sha256 (D-23)
```

Dependencies: `mt-vectors` and `mt-contract` depend on nothing internal; `mt-harness` depends on `mt-capture`, `mt-toolchain`, `mt-vectors`, `mt-seeds`; `mt-cli` depends on all (matches building-the-tool.md section 3).

### Q1. TRACTOR vector format (measured, answers D-05..D-08 mapping)

**Pin:** `DARPA-TRACTOR-Program/PUBLIC-Test-Corpus` commit `37960ee08a7c644ee66e29855eca33903f702e67` (`main`, 2026-09-14, "Second evaluation (#225)"; tags v1 and v2 also exist) [VERIFIED: `git ls-remote` and `git log -1` on a blobless clone, this session]. A sparse fetch by commit works with `git init; git fetch --depth 1 --no-tags --filter=blob:none <url> <sha>; git sparse-checkout set --no-cone <paths>; git checkout FETCH_HEAD` and fetched exactly the requested files [VERIFIED: command run]. Verify each file by sha256 (the whole tree has 26,492 files, so the Phase 2 whole-tree hash is the wrong tool here).

**Layout:** `Public-Tests/<bundle>/<case>/test_vectors/<name>.json`, or a directory `<name>/cando_vector.json` with optional `setup`, `resources/`, `file_changes.tar.gz`. Bundles: `B01_synthetic`, `B01_organic`, `B02_synthetic`, `B02_organic`, `P00_perlin_noise`, `P01_sphincs_plus`, `P02_libgit2`, `Examples`. A case folder ending `_lib` is a library case. cando's `load_all` reads only `*.json` directly inside `test_vectors/` (`.json.bak` is ignored) [CITED: raw README at the pinned commit; tools/cando2/src/test_vector.rs `load_all`].

**Rust type in cando (source of truth)**, `tools/cando2/src/test_vector.rs` at the pin [VERIFIED: file read this session]:
- `#[serde(crate = "self::serde", deny_unknown_fields)] pub struct TestVector<S, E>` with: `stdin: Option<String>`, `stdout: Option<Output>`, `stderr: Option<Output>`, `env: Option<HashMap<String, String>>`, `has_ub: bool` via `#[serde(default, deserialize_with = "deserialize_has_ub")]`, `note: Option<String>`, `argv: Option<Vec<String>>`, `rc: Option<i32>`, `lib_state_in: Option<S>`, `lib_state_out: Option<E>`.
- `Output { pattern: String, is_regex: Option<bool> }`, also `deny_unknown_fields`.
- The has_ub deserializer, verbatim: `Ok(serde_json::Value::deserialize(deserializer).is_ok())`, documented "deserialize it to a `bool` if it's present and ignore any value".

**Field shapes** (survey of the 2,610 vector JSON files under `Public-Tests/B0*/**/test_vectors`, all parsed, none malformed) [VERIFIED: python3 -I survey over a sparse checkout, this session]:

| Field | JSON type seen | Notes |
|---|---|---|
| `argv` | array of strings (1,219) | In TRACTOR it is "appended after the driver path", so `argv[0]` is the first argument, not the program name |
| `stdin` | string (1,140) | Unicode text; NUL as `\u0000`; control bytes as `\u00XX`; real non-ASCII appears as UTF-8 in 4 files. No hex/base64 convention exists |
| `env` | object | Never used in the 2,610 files; optional, default empty |
| `stdout`, `stderr` | object `{pattern}` (2,342) or `{pattern, is_regex}` (69), `is_regex: true` in 49 | Exact string match by default |
| `rc` | integer (969) | Negative values are used for signals, e.g. -11 with a segfault label, -8 for divide by zero |
| `lib_state_in`, `lib_state_out` | free-form JSON object (1,394 / 1,378) | Shape is defined per test case by a Rust `harness!` state struct: ints, floats, strings, arrays, null, nested objects. The function return value is a field of this state (e.g. `returns`) |
| `has_ub` | string (101) or integer (57), never boolean | Presence means "skip this vector" |
| `note` | string (3) | A tenth key, free text, "unused by test runner" |

Key-set frequencies: `lib_state_in+lib_state_out` alone 1,066; `argv+rc+stderr+stdin+stdout` 879; `lib_state_in+lib_state_out+stdout` 243; many fields are absent.

**Consequences for the schema (these refine, not reopen, D-05..D-08):**
1. **`has_ub = false` is the absent key.** In cando any present `has_ub` value, including `false` and `null`, deserializes to true, so a serialized `"has_ub": false` would make TRACTOR tooling skip our vector. The Rust type is `has_ub: Option<HasUb>` (or a bool that is skipped when false); when true, emit a string label (suggested: `"crash:SIGSEGV"` for an observed crash). D-20 "`has_ub = false`" is then "key omitted", with the unchecked source in the sidecar.
2. **`stdout`/`stderr` serialize as `{"pattern": "<string>"}`**, never as bare strings; `is_regex` is omitted (exact). Reject unknown keys inside them.
3. **Binary bytes in `stdin` and `stdout.pattern` need one codec, written once in the contract.** TRACTOR strings are Unicode text. Recommended codec: each byte `b` is the scalar `U+00bb` (Latin-1 identity), serialized as a JSON string with `\u00XX` escapes for controls, `"` and `\`. It is lossless for all 256 values and readable by any JSON tool, but a tool that UTF-8-encodes the string sees bytes >= 0x80 doubled. This is the one point to confirm with the founder (Open Question 1). Alternative: hex text in the string, flagged by a leading marker in `argv[0]`.
4. **`rc` holds the C return value as an `i32`** (two's complement reinterpretation of any <= 32-bit scalar; void gives 0; a pointer return gives 0 or 1 for null or not). On a fault, use TRACTOR's own convention `rc = -signal` (e.g. -11) together with `has_ub` and an outcome class in `stderr`. Note that in TRACTOR's lib runner `rc` is the harness exit code, so repurposing it for the return value is only parse-compatible, as D-05 intends.
5. **`argv[0]`** (`<target>/<config>/<entry>` per D-05) is a first argument in TRACTOR's convention; harmless because our vectors are not run by cando.
6. **`lib_state_*` are free-form**, so D-06's typed fields fit inside them. Recommended field shape: `"name": {"type": "<C type>", "value": <v>}` with `value` a JSON integer when it fits 32 bits, a decimal string for 64-bit, hex string for byte arrays, `{"buffer":"in","offset":4}` for pointers, `null` for NULL, `{"symbol":"name"}` for function pointers, and for `float`/`double` the IEEE bit pattern as a hex string (serde_json cannot hold NaN or infinity, and decimal text is not an exact value).
7. **Top-level `note` is accepted on load and preserved, never emitted** by our executor, and does not count as one of the nine (3 of 2,610 public vectors carry it; pick vectors without it for the 3-vector test anyway). Two validation levels are needed: *TRACTOR-compatible* (every field optional, `deny_unknown_fields` except `note`) and *executor-complete* (all eight non-`has_ub` fields present, `env` equal to `{}`, `argv[0]` well-formed). "Rejects a single merged `lib_state`" is `deny_unknown_fields` (a `lib_state` key is unknown).
8. **"Loads unchanged" = parses with no preprocessing and re-serializes to a JSON-equal value** (`serde_json::Value` equality). serde_json's default map is sorted, so key order is not preserved (preserve_order needs `indexmap` as a direct feature, which is not enabled); do not promise byte equality of the file. Our own output uses a fixed field order and sorted maps.

**Good candidates** (paths under the pin; sha256 of the file bytes; none has `note`) [VERIFIED: sha256sum, this session]:

| Path | sha256 | Why |
|---|---|---|
| `Public-Tests/B01_synthetic/001_helloworld_lib/test_vectors/test1.json` | `bb2fb76152cf022ea508ceb16d77a710223dd73dab8ef0d58f6ed52b9fb42447` | `_lib`, `lib_state_in/out` + `stdout` |
| `Public-Tests/B01_synthetic/007_errno_pow_lib/test_vectors/test11.json` | `64afaa619c399ec9220f562bd44efe92a66236fe43c0ca0ea712a1040182a331` | `_lib`, floats in lib_state, `stderr` |
| `Public-Tests/B01_synthetic/030_mutable_buffer_overlap_extrahard_lib/test_vectors/test04.json` | `9329bb5aeb230a860f2ddd96a249eeb1c7a2acbbb024fd42455eb3055d5a6531` | `_lib`, string `has_ub`, array in lib_state |
| `Public-Tests/B01_synthetic/009_stack_buffer_overflow_lib/test_vectors/test09_overflow_int2.json` | `c86028ad8ef42b5df95d10aae7965ffd28ebcdcb04c98d8f162f9f7735e5e701` | `rc` -11, `argv: []`, `has_ub` string |
| `Public-Tests/B01_synthetic/002_stdin_echo/test_vectors/nul.json` | `b8fbd77226a3b179f58a20b2ef7766393a0c708980dbeb4cf14c94cd51ec785c` | executable case, `stdin` with `\u0000` |
| `Public-Tests/B02_synthetic/strcpy/test_vectors/41.json` | `65b1e40ba3503b17536a9c2ff2faa2e830de8b937f07c86ca8e171afcd53fb21` | integer `has_ub` |

Record only these paths, the commit and the hashes in `testbed/tractor-pin.toml`; do not copy vector content into the repo. **Licence status as found (not legal advice, Phase 12 decides):** the repo root has an MIT `LICENSE` ("Copyright (c) 2026 Massachusetts Institute of Technology"), the README carries a "DISTRIBUTION STATEMENT A. Approved for public release" and DFARS rights text, and the `*_organic` and `P0x` bundles contain third-party code (for example libgit2) under their own licences that were not checked. So the PROJECT "unchecked" status stands; keep the local-only rule.

### Q2. Running the captured Cortex-M objects (D-01, D-04): what works

**Verdict: workable and faithful enough; no change to D-01.** Evidence (QEMU 8.2.2 on this host, GCC 13.2.1; the pinned image differs, see the CI smoke list below):

| Question | Finding | Evidence |
|---|---|---|
| Can a thumbv7em hard-float binary run in `qemu-arm`? | Yes, with `-cpu cortex-m4` (also cortex-m7, cortex-a15, cortex-a7, max, any). A static ELF with a Thumb entry, own `_start` and raw `svc #0` Linux syscalls printed output and exited with the expected status; VFP (FPv4-SP) code gave the right result | [VERIFIED: ran `t.elf` under 6 CPU models; `qemu-arm -cpu help` lists cortex-m3/m4/m7/m33/m55] |
| Is `cortex-m4` available in user mode in 7.2? | Yes per source: the table `arm_tcg_cpus` lists `{ .name = "cortex-m4", .initfn = cortex_m4_initfn, .class_init = arm_v7m_class_init }` outside any `CONFIG_USER_ONLY` exclusion | [CITED: raw.githubusercontent.com/qemu/qemu/v7.2.0/target/arm/cpu_tcg.c lines 1153-1157]. Must be re-proved in the image (CI smoke) |
| Why does Phase 1 say user mode cannot run Cortex-M? | It is true for unmodified firmware (vector table, reset handler, MSP). The driver is not firmware: it is a hosted-style ELF that reuses the same object code | [CITED: container/pins.toml:244-247 comment "user mode cannot run bare-metal Cortex-M images (RESEARCH correction 1, CONTEXT D-11)"]. Plan should fix that comment's wording |
| Semihosting in user mode | Works: rdimon.specs gave `argc` and `argv` (from qemu's argv), `open`/`read` of a host file, `write` to fd 1, and the exit status. QEMU 7.2 `arm-compat-semi.c` handles OPEN, CLOSE, WRITE, READ, EXIT, EXIT_EXTENDED, HEAPINFO, GET_CMDLINE and also CLOCK, TIME, ELAPSED (non-deterministic; never use). `semihosting_enabled()` returns true unconditionally in user-only builds. No flag exists in user mode | [VERIFIED: ran `m.elf` built with `--specs=rdimon.specs`; CITED: qemu v7.2.0 semihosting/arm-compat-semi.c TARGET_SYS_* cases, include/semihosting/semihost.h:29-33] |
| `--specs=linux.specs` | Built but segfaulted at startup under `-cpu cortex-m4` on first try. Do not use | [VERIFIED: ran `ml.elf`] |
| I/O choice | Recommended: no semihosting. Own `_start`, own stack, raw syscalls `open(5) read(3) write(4) mmap2(192) mprotect(125) rt_sigaction(174) sigaltstack(186) exit_group(248)` through `svc #0` with `r7` = number. One implementation gives input, result, guards and fault handling | [VERIFIED: corejson and zlib drivers ran this way] |
| A-profile CPU instead? | Not needed. Unaligned `LDR` works, unaligned `LDRD`, `LDM` and `VLDR` raise SIGBUS, `UDIV` by zero returns 0, FPSCR starts at 0, identically on cortex-m4, cortex-a15 and max | [VERIFIED: `ua.elf` test] |
| Entry/startup | `__libc_init_array` then the entry. Provide `_init` and `_fini` stubs and newlib syscall stubs `_sbrk` (static arena), `_write`, `_read`, `_close`, `_lseek`, `_fstat`, `_isatty`, `_getpid`, `_kill`, `_exit`. Link `-nostartfiles --specs=nosys.specs` (name is in SHIPPED_SPECS) | [VERIFIED: linked and ran] |

SHIPPED_SPECS, verbatim: `pub const SHIPPED_SPECS: [&str; 13] = ["aprofile-validation-v2m.specs", "aprofile-validation.specs", "aprofile-ve-v2m.specs", "aprofile-ve.specs", "iq80310.specs", "linux.specs", "nano.specs", "nosys.specs", "pid.specs", "rdimon-v2m.specs", "rdimon.specs", "rdpmon.specs", "redboot.specs",];` [VERIFIED: crates/mt-capture/src/cc.rs:365-379]. These bound which `-specs` a *captured compile* may carry; the driver link may use `nosys.specs`.

**Guard pages (D-04), measured:** the driver `mmap2`s `(pages + 2)` pages at a fixed address with `MAP_FIXED`, `mprotect`s the first and last page `PROT_NONE`, and places the buffer right-aligned so its last byte is the last byte before the trailing guard. A normal call returned the right result; passing `len + 1` faulted (SIGSEGV) [VERIFIED]. Limits to write into the contract: page granularity is 4 KiB, so the end guard is byte-exact and the start guard is exact only when `len` is a multiple of 4096; bytes before a right-aligned buffer inside its first page are slack (fill with a pattern and re-check after the call for writes). For `len == 0`, pass the address of the guard page so any dereference faults. Put the stack on its own mapping with a guard page below it to catch overflow.

**Fault surfaces:** an uncaught guest SIGSEGV makes qemu-arm print `qemu: uncaught target signal 11 (Segmentation fault) - core dumped`, die with signal 11, and, if `RLIMIT_CORE` is nonzero, write `qemu_<exe>_<timestamp>_<pid>.core` into the cwd (observed with `ulimit -c unlimited`), which is non-deterministic. Fix: install in-guest handlers (`rt_sigaction` for SIGSEGV, SIGBUS, SIGILL, SIGFPE, SIGABRT with `SA_ONSTACK` plus `sigaltstack`) that call `exit_group(128 + sig)`. Verified: exit status 139 and no core file. The runner then sees `exit.code = Some(139)`, a normal exit, and the executor maps `code >= 129 && code <= 128 + 31` from the handler to a fault with `rc = -(code - 128)`. If the runner reports `exit.signal` instead, QEMU itself died: classify as `executor_error`, never as a vector outcome.

**Determinism knobs:** `-R <bytes>` (reserved guest VA) gives a fixed guest layout independent of host ASLR and doubles as the memory budget (a driver that needs more fails at `mmap`; with `-R 0x04000000` a mapping at 0x08000000 failed and the driver exited 98) [VERIFIED]. QEMU passes the host environment to the guest, so the guest stack pointer depends on environment size (observed 0x407feb9c without `-R`, 0x0b2a9b9c with `-R 0x20000000`); relocate `sp` to the driver's own mapping so nothing depends on it. Add `-seed 0` and `-cpu cortex-m4`; no timers, no `getrandom`. 100 consecutive runs gave one result hash [VERIFIED]. Speed: about 5 ms per run in user mode (100 runs in 0.44 s).

**qemu-system-arm comparison (`mps2-an386`, Cortex-M4, `-semihosting-config enable=on,target=native`):**

| Aspect | qemu-arm user mode (recommended) | qemu-system-arm mps2-an386 |
|---|---|---|
| Runs the captured code | Yes, via hosted-style driver | Yes, needs vector table, linker script, FPU enable (CPACR), startup code |
| Speed | about 5 ms per run | about 36 ms per run (10 runs in 0.36 s) [VERIFIED] |
| Guard regions | 4 KiB pages, end-exact via mprotect | v7-M MPU regions are power of two, minimum 32 bytes, 8 regions; an access beyond end of RAM gives HardFault [VERIFIED: read of 0x60000000 gave HardFault]; byte-exact is not possible with the MPU |
| Fault surface | In-guest signal handler, exit 128+sig | HardFault handler must be written and must call semihosting EXIT |
| Fidelity differences | No NVIC/MPU/SysTick; NULL (page 0) faults, whereas on hardware address 0 is readable flash | Closest to hardware; NULL read does not fault |
| Determinism | Fixed with `-R` and own stack | Deterministic when no peripherals or timers are touched; `-icount` not needed |
| I/O | Raw syscalls (or semihosting) | Semihosting `target=native` gives the guest unrestricted host file access |

Recommendation: keep D-01 as written. Record the two fidelity differences (NULL page, no MPU or fault CSRs) in the contract as "out-of-contract input" cases consistent with D-03. Phase 5's DIFF-06 can add the system-mode replay.

**CI smoke list (cannot be measured here):** in the pinned image run `qemu-arm -version`, `qemu-arm -cpu help | grep cortex-m4`, the tiny `_start` ELF, a guard-page overrun with the in-guest handler (assert exit 139, no core file), `-R` rejection, and 100 repeated runs with one hash; also `command -v llvm-dwarfdump arm-none-eabi-ar arm-none-eabi-nm`; also that `--cap-drop ALL --security-opt no-new-privileges --read-only` containers allow qemu-arm (mmap FIXED, `/proc/self/maps`).

### Q3. Linking the driver against captured objects

Facts [VERIFIED: ran on this host with local objects compiled by GCC 13.2.1, `.comment` shows `GCC: (15:13.2.rel1-2) 13.2.1`]:
- **Archive vs object list.** `arm-none-eabi-ar rcsD pair.a <objects sorted by path>` gives a deterministic archive whose members are the unchanged object bytes; ld pulls only members the entry needs. Linking all 113 mbedtls objects directly also worked (0 undefined), but the unresolved-vs-objects survey shows libc and libgcc symbols only for most pairs, and nimble needs five symbols no object defines (`ble_sm_alg_c1`, `ble_sm_alg_s1`, `ble_transport_ll_init`, `ble_transport_to_ll_acl_impl`, `ble_transport_to_ll_cmd_impl`). With the archive, `ble_uuid_init_from_buf` links (36 KB) and `ble_hs_adv_parse_fields` does not. Use the archive and treat an unresolved symbol as a logged non-drivable reason.
- **Which libs:** link with the same `-mcpu/-mthumb/-mfloat-abi/-mfpu` flags so gcc selects the `thumb/v7e-m+fp/hard` multilib. Add `-lm` (qcbor needs `pow`, `exp2`, `round`, `llround`, `feclearexcept`). newlib `libc.a` supplies `memcpy`, `malloc`, `strtod`, `sprintf`, `__assert_func`; libgcc supplies `__aeabi_*`.
- **Symbols the driver must define itself:** `_sbrk` over a static arena (1 MiB worked for zlib), `_kill`/`_getpid` so `abort()` and a failed `assert` reach the fault exit (SIGABRT, 134), `_init`/`_fini`, and the newlib I/O stubs. `main` is never defined, so no conflict. newlib already supplies `__stack_chk_guard` and `__stack_chk_fail` (cjson links without driver definitions); its constructor, run by `__libc_init_array`, sets a fixed terminator canary, measured as bytes `00 00 0a ff` on three runs, so stack-protector builds are deterministic; a smashed stack reaches `abort` and so the SIGABRT path [VERIFIED: ran; the 14.3 newlib is [ASSUMED] to match, A2].
- **Entry check flags:** do not use the driver's own idea of flags. Reuse `mt_capture::entries::filter_flags` and `include_prefix`/`unit_include_prefix` on the defining TU's recorded argv to get `-I/-D/-std/-m*` plus the TU's `#define` preamble (the Phase 2 `entry_probe.c` mechanism already reproduces it). Run with the compile command's `directory` as cwd (libfdt and httpparser use relative `-I`). Drop `-O*`, `-g`, `-W*`, `-fPIC`; keep `-std=`, `-fshort-enums`/`-funsigned-char`/`-fpack-struct` style ABI flags if a config carries them. Across all 481 locally recorded compile commands the only `-f`/`-m`/`-std` options are the four `-m` baseline flags, `-fPIC` (32), `-std=c89` (2), `-fstack-protector-strong` (2, cjson) and `-fvisibility=hidden` (2); none changes layout [VERIFIED: python3 -I over `.mt-work/capture/*/*/compile_commands.json`]. The validator should still fail closed on a layout-affecting flag it does not handle.
- **Exact-objects property (D-19):** before linking, recompute each object's sha256 and require equality with `capture.json` `objects.<path>.sha256` (fail closed); record those hashes, `pair.a` sha256, `driver.c` sha256 and `driver.elf` sha256 in the provenance sidecar. `ar` copies member bytes verbatim, so the check on inputs is sufficient; optionally extract and re-hash members in a test.
- **GC:** `-Wl,--gc-sections` is unnecessary and not safe for the "exact code" argument; do not use it. Request a map (`-Wl,-Map=<work file>`) only if the provenance wants the list of pulled members.
- **Objects with `-fPIC`** (httpparser sets `-fPIC` and `-O3`) link statically without problems (a `.got` is created) [ASSUMED for the 14.3 image; not run, A5].
- **Local coverage:** this host has objects for 14 targets. `tinycbor` (both configs, `build_failed` locally), `isotp` (`capture_failed` locally) and `paho/server` (dropped) have none locally; they are CI-only.

### Q4. State layout (D-09) and globals (D-10)

**Layout source: DWARF from the pinned GCC.** Probe TU = the entry's include preamble + one global variable per state type from the entry signature, compiled with the TU's flags plus `-g -gdwarf-5 -O0` to a scratch object. `llvm-dwarfdump --debug-info` text then gives `DW_TAG_structure_type` with `DW_AT_byte_size`, members with `DW_AT_name`, `DW_AT_type` and `DW_AT_data_member_location`, array `DW_TAG_subrange_type` with `DW_AT_upper_bound`, typedef/const chains, and bitfields as `DW_AT_bit_size` + `DW_AT_data_bit_offset` (checked on `http_parser`, 21 members) [VERIFIED: ran with llvm-dwarfdump 18 here; the image has llvm-16, same text format for these tags is [ASSUMED], A3]. A tab-separated `DW_AT_name\t("x")` line format is easy to parse with an indentation-based reader.

**Why not clang:** GCC arm-none-eabi defaults to short enums: `_Static_assert(sizeof(enum E) == 4)` fails under `arm-none-eabi-gcc -mcpu=cortex-m4 ...`, `__ARM_SIZEOF_MINIMAL_ENUM` is 1, and the captured object says `Tag_ABI_enum_size: small`; the same assert passes under `clang --target=thumbv7em-none-eabihf -mcpu=cortex-m4` [VERIFIED: ran both]. Any enum-typed field (CborError, JSONStatus_t, QCBORDecodeMode) would be laid out wrongly by clang's record-layout dump. Clang's dump also prints typedef'd anonymous structs as `struct (unnamed at file:line)`.

**Decoder (host side):** the driver dumps raw bytes of each state object before and after the call; the host decodes with the LayoutModel: base types by DW_ATE and size (signed/unsigned/char/bool/float bits), enums as integers with underlying size, arrays (byte arrays as hex, others as lists), nested structs as objects keyed by field name, unions as hex plus an `unobserved` reason or per-member (planner choice; `ble_uuid_any_t` is a union), bitfields from `bit_size`/`data_bit_offset`, padding bytes never recorded (zero-fill state before the call, compare fields only), anonymous members named `_anon<offset>`. Pointers: classify each 32-bit value against the driver's known regions: null, inside `in`/`out`/state objects gives `{buffer, offset}`, equal to a function symbol gives `{symbol}`, inside the heap arena gives `{heap: offset}` (deterministic malloc) or an unobserved mark, else `unresolved`. The driver's fixed region bases are constants of the template, so the same classification is reproducible.

**Globals (D-10):** `object` crate over each captured object: symbols defined in sections whose name starts `.data` or `.bss` (including local statics: `nm -S` shows `b version.0`, `b global_error`, `d global_hooks` for cjson). In the *linked* driver ELF, find the same names by walking `STT_FILE` entries so a local symbol is joined to its object, and take final addresses and sizes from there. The driver reads a small dump plan from `req.bin` (address, size pairs; the host writes it after the single link, so no second link or patching) and dumps those ranges before and after. Sizes seen: cjson 3, libfdt 1 (`fdt_errtable`), microcoap 2 (`light`, `rsp` 1500 bytes), httpparser 3, mbedtls 33 (AES tables), zlib/lz4/zcbor 0 [VERIFIED: `arm-none-eabi-nm -S --defined-only`]. Constant tables that contain pointers sit in `.data` too (`method_strings`, `fdt_errtable`), so record only **changed** globals in `lib_state_out.globals` (name, size, hex of new bytes) and keep "unchanged" implicit; raw words that hold addresses are deterministic for a fixed driver but not portable, so for Tier 1 mark pointer-bearing changed globals `unobserved` with reason `link-address-dependent`, and type them in a later step by re-compiling their TU with `-g` (same flags). cjson's `global_error {json, position}` is file-private (not in a header), so only the `-g` re-compile can type it.

**Pointers inside state (D-06):** expressed as `{buffer, offset}` after classification above, e.g. microcoap `coap_packet_t` option and payload pointers into `in`, paho `unsigned char **payload` into `in`, zcbor `Pet` strings into `in`, `QCBORDecodeContext` input pointer into `in`.

### Q5. Adapter roles (D-11..D-14): vocabulary and inventory

**Where it lives.** Extend `Entry` in `crates/mt-capture/src/manifest.rs` (currently `name, header, source, form, signature, accepted_wrapped`, `#[serde(deny_unknown_fields)]`, lines 291-308 [VERIFIED]) with three optional, additive fields, each `#[serde(default)]`: `arg` (one table per C parameter, in order), `setup` (steps) and `unobserved` (marks). Additive keys need no `MANIFEST_SCHEMA_VERSION` bump (it is `1` at manifest.rs:20 and checked at :418); bumping it is possible but touches Phase 2 tests, so recommend not bumping. A global `[exec]` table holds `timeout_ms`, `va_bytes`, `sentinel`, `max_combos`.

**Role vocabulary** (signature types come from the existing `signature` string; the validator splits its top-level parameter list and checks count and pointer shape):

| Role | Meaning | Recorded as |
|---|---|---|
| `in_buf` | pointer to the input bytes (`mutable = true` records the bytes after the call; `align` default 1) | `stdin` |
| `in_len` | length of the input in the parameter's C type (`max` for narrow types, e.g. 255); derived, not in `argv` | — |
| `out_buf` + `out_cap` | output buffer of `cap` bytes, prefilled with the sentinel (D-07); `out_cap` is a scalar with default and sweep | `stdout` (whole buffer) |
| `scalar` | value with `default` and optional `sweep` (decimal or an uppercase constant name matching `^[A-Z][A-Z0-9_]*$`) | `argv[1..]` decimal text |
| `str` | NUL-terminated string from a declared list (corejson query) | `argv[n]` text |
| `state` | pointer to a struct (type from the signature), zero-filled; optional `count` for arrays | `lib_state_in` / `lib_state_out` |
| `out_scalar` | pointer to a scalar, zero-filled | `lib_state_out` |
| `out_ptr` | pointer to a pointer, recorded as `{buffer, offset}` | `lib_state_out` |
| `inout_len` | pointer to a size (zlib `destLen` = capacity in, produced out; lz4f `srcSizePtr` = len in, consumed out) | `lib_state_*` |
| `cursor` + `end` | pointer-to-pointer into the input plus end pointer (`mbedtls_asn1_get_tag`, form `range`) | `lib_state_*` |
| `zero_const` | pointer to a const zeroed struct (http_parser settings with no callbacks) | — |
| `wrap` | struct by value built from the input (`ptr_field`, `len_field`), e.g. `UsefulBufC` | — |
| return | implicit from the signature: integer or enum gives `rc`; void gives 0; pointer gives 0 or 1; struct by value goes to `lib_state_out.ret` | `rc` |

**Setup steps** (D-12): `[[target.X.entry.setup]] call = "<C identifier>" args = ["$in", "$len", "0", "&parser"]`. The argument grammar is closed: `$in`, `$len`, `$cap`, a decimal literal, an uppercase constant, or `&<name of a state arg>`. Field assignments for wrapped packet-info structs (coremqtt) use a second closed form `set = {field = "$in[0]" | "$in+1" | "$len-1"}`. A setup step that returns nonzero or faults is the outcome `setup_failed` (own class). The validator rejects any text outside these grammars, so manifest strings can never inject C into the generated driver (Security Domain).

**Inventory of the 27 entries** (seeds from the committed `index.json` files [VERIFIED]; tiers: A flat roles only, B needs `str`/`state` array/`wrap`/one setup step, C needs setup plus field assignments or opaque/heap state, N not expressible):

| Target | Entry | Seeds | Roles | Tier |
|---|---|---|---|---|
| corejson | `JSON_Validate` | 108 | in_buf, in_len | A (tracer) |
| corejson | `JSON_SearchT` | 108 | in_buf(mutable), in_len, str query + len, out_ptr, out_scalar x2 | B |
| libfdt | `fdt_check_full` | 16 | in_buf, in_len | A |
| lz4 | `LZ4_decompress_safe` | 20 | in_buf, out_buf, in_len(int), out_cap(int) | A |
| lz4 | `LZ4F_getFrameInfo` | 20 | setup createDecompressionContext (heap, opaque), state, inout_len | C |
| zlib | `uncompress` | 18 | out_buf, inout_len, in_buf, in_len | A |
| zlib | `inflate` (wrapped) | 18 | setup inflateInit, assign next_in/avail_in/next_out/avail_out, flush sweep, opaque `state` | C |
| microcoap | `coap_parse` | 16 | state coap_packet_t, in_buf, in_len; globals `light`, `rsp` | A |
| paho | `MQTTDeserialize_connack` | 17 | out_scalar x2, in_buf, in_len | A |
| paho | `MQTTDeserialize_publish` | 17 | out_scalar x4, state MQTTString, out_ptr, out_scalar, in_buf, in_len | A |
| nanopb | `pb_istream_from_buffer` | 200 | in_buf, in_len, struct return | A |
| nanopb | `pb_decode_ex` (wrapped) | 200 | needs a generated `pb_msgdesc_t`, not in the captured objects (protoc is stubbed to `/usr/bin/true`) | N |
| cjson | `cJSON_ParseWithLength` | 25 | in_buf, in_len; return pointer gives 0/1; heap tree unobserved; global `global_error` via D-10 | A (partial) |
| nimble | `ble_uuid_init_from_buf` | 200 | state union `ble_uuid_any_t`, in_buf, in_len | A |
| nimble | `ble_hs_adv_parse_fields` | 200 | in_len is `uint8_t` (max 255); link fails on 5 unresolved symbols | N (logged) |
| tinycbor | `cbor_parser_init` | 200 | in_buf, in_len, scalar flags, state CborParser, state CborValue | A (CI-only objects) |
| tinycbor | `cbor_value_validate` (wrapped) | 200 | setup `cbor_parser_init`, state, scalar flags | B |
| zcbor | `zcbor_new_decode_state` | 137 | state array `zcbor_state_t`, scalars, in_buf, scratch buffer role | B |
| zcbor | `cbor_decode_Pet` | 0 | in_buf, in_len, state Pet, out_scalar | A, no seeds |
| qcbor | `QCBORDecode_Init` (wrapped) | 200 | state QCBORDecodeContext, wrap `UsefulBufC{ptr,len}`, scalar mode sweep | B |
| coremqtt | `MQTT_DeserializePublish` (wrapped) | 15 | assign packet-info fields from input, 3 states, 2 scalars | C |
| coremqtt | `MQTT_DeserializeAck` (wrapped) | 15 | as above plus connection-properties state | C |
| httpparser | `http_parser_execute` | 76 | setup `http_parser_init`, zero_const settings, in_buf, in_len | B |
| httpparser | `http_parser_parse_url` | 0 | in_buf, in_len, scalar is_connect {0,1}, state `http_parser_url` | A, no seeds |
| mbedtls | `mbedtls_x509_crt_parse_der` | 200 | setup `mbedtls_x509_crt_init`, heap pointers (`next`, `raw.p`) unobserved | C |
| mbedtls | `mbedtls_asn1_get_tag` (range) | 0 | cursor, end, out_scalar, scalar tag sweep | A, no seeds |
| isotp | `isotp_on_can_message` | 19 | setup `isotp_init_link` with two scratch buffers; objects CI-only locally | C |

**Floor first (10 targets, all with seeds, in suggested build order):** corejson, zlib, lz4, libfdt, microcoap, paho, nanopb (`pb_istream_from_buffer`), nimble (`ble_uuid_init_from_buf`), cjson, tinycbor (CI-only) — then zcbor, qcbor, httpparser from tier B. mbedtls, coremqtt, isotp are tier C and can be logged with their reason. Entries with zero seeds (`cbor_decode_Pet`, `http_parser_parse_url`, `mbedtls_asn1_get_tag`) cannot be driven under D-22; log them `no_seeds` (Open Question 5). Entries whose output is not a buffer are the validators `JSON_Validate`, `fdt_check_full` and `cbor_value_validate`, plus parsers that fill only a state struct (`coap_parse`, `cbor_parser_init`, the paho and nimble entries): they produce `rc` and `lib_state_out`, and `stdout` is the empty string.

**Drop log (D-13):** reuse `DropLog`-style TOML (`testbed/dropped.toml` pattern: `target`, `entry`, `cause`, `detail`) in a sibling file `testbed/adapter-dropped.toml`; the floor test counts targets with at least one drivable entry using the same arithmetic as `check_floor` (`pub fn check_floor(manifest, drops)`, `MIN_RETAINED: usize = 10` [VERIFIED: crates/mt-capture/src/testbed.rs:293, droplog.rs:27]). Enforce `drivable_targets >= 10` as a test named `eqv_03_adapter_floor_is_ten`.

### Q6. Executor determinism, budgets and classification

Runner facts [VERIFIED: crates/mt-toolchain/src/runner.rs read this session]:
- `pub const BASE_ENV_KEYS: [&str; 4] = ["LANG", "PATH", "SOURCE_DATE_EPOCH", "TZ"];` (line 75); extras may not override them; the child starts from `env_clear()` plus those four plus `extra_env`.
- `.stdin(Stdio::null())` (line 673): **the runner cannot feed stdin.** Feed input through `req.bin` in the fixed cwd (recommended; no runner change), or add `stdin: Option<Vec<u8>>` to `RunRequest` with a writer thread (touches audited code; only if the planner wants D-05's `stdin` to be literal).
- `RunnerConfig.timeout` is config-wide (line 101), `RunRequest.cwd` per request (line 130). Create one `RunnerConfig` per budget. There is **no memory limit and no rlimit** (grep for `rlimit` finds nothing): use `qemu-arm -R <bytes>` as the memory budget (guest VA cap) and treat the 1 MiB arena plus mappings as the guest's whole memory.
- `DEFAULT_OUTPUT_CAP = 16 MiB` (line 72); require `stdout.complete && !stdout.truncated` or fail closed.
- Timeout: `exit.timed_out == true` is recorded as the outcome, never retried. The record has no clock fields, so two identical runs serialize identically.

Suggested fixed values (Claude's discretion): timeout 10 s, `-R 0x10000000` (256 MiB; driver mappings stay below it), sentinel `0xA5`, 1 MiB heap arena, input cap 65,536 bytes (the seed cap `max_seed_bytes = 65536` [VERIFIED: manifest.toml `[seed_caps]`]), `max_combos` 16.

Classification (executor, never from guest text):

| Runner result | Outcome class | Vector effect |
|---|---|---|
| `exit.code == 0` and a well-formed result frame (magic, version, lengths, trailing sha) on stdout | `ok` | `has_ub` omitted, source `unchecked` (D-20) |
| `exit.code` in 129..=159 (driver handler, 128 + signo) | `fault` | `rc = -signo`, `has_ub = "crash:SIG<name>"` (D-03), source `observed_crash` |
| `exit.code` 134 from `abort`/`assert` | `abort` (a fault subclass) | same; Open Question 4 |
| `exit.timed_out` | `timeout` | `stderr` class `timeout`, excluded from comparison, never a pass |
| driver setup exit codes 96..=99 (open/read/mmap failed), `exit.signal.is_some()`, `tree_kill_sent`, incomplete or truncated stream, bad frame | `executor_error` | fail the run, store nothing |
| setup step returned nonzero | `setup_failed` | own class (D-12) |

`stderr.pattern` carries only executor text: `outcome=<class>` plus fixed detail (signal name), never guest output or addresses. Anything the C code prints through libc `printf`/`puts` (microcoap `DEBUG`, zcbor, mbedtls) goes to the driver's `_write` shim, which must **discard** it and count bytes, so it can never mix into the result frame on fd 1; the byte count goes to provenance. The result frame is written with a raw `write(1, ...)` after the call.

Run-twice check: execute the same request twice in the same slot; require equal `RunRecord` (minus nothing), equal stdout sha256, and equal canonical vector bytes; on mismatch fail and keep both for inspection. Parallel workers need one cwd per slot.

### Q7. Contract document and policy file

**Policy TOML** (`testbed/policy/<module>.toml`, loaded by `mt_contract::Policy::load`, `#[serde(deny_unknown_fields)]` everywhere, same strict-TOML style as the manifest):

```toml
schema_version = 1
contract_version = "0.1.0"        # semver of docs/specs/equivalence-contract.md this file implements
module = "corejson"               # manifest target id (or customer module name later)

[ub]
default_action = "return_error"   # return_error | panic | preserve

# Required only when default_action != "return_error" (roadmap criterion: a recorded setting to change the default)
# [ub.default_change]
# reason = "..."
# approver = "..."
# date = "2026-10-21"

# Optional declared error code per function, used by return_error (D-17); syntactic check only in Phase 3
# [[ub.error_code]]
# function = "JSON_Validate"
# value = "JSONIllegalDocument"

# Phase 3 ships with no overrides. The table exists and is validated.
# [[ub.override]]
# function = "JSON_Validate"
# kind = "signed_overflow"        # free identifier now; Phase 4 constrains the kind set
# location = "source/core_json.c:123"
# action = "preserve"
# reason = "..."
# approver = "..."
# date = "2026-10-21"
```

Validation rules (each gets a named test): unknown key anywhere fails; `schema_version == 1`; `contract_version` is semver and not greater than the crate's `CONTRACT_VERSION`; `default_action` is one of the three; `default_action != return_error` requires the full `default_change` record; every override has all seven keys non-empty (`function`, `kind`, `location`, `action`, `reason`, `approver`, `date`) and `date` matches `YYYY-MM-DD` with a real calendar date (no clock read); `action` is in the menu; no two overrides share `(function, kind, location)`; an override whose site is not in the `known_sites` slice passed to `validate(&policy, &known_sites)` is rejected, and Phase 3 passes an empty slice, so any non-empty override list fails today (D-16). Reports list overrides (D-18) via a `Policy::override_report()`.

**`docs/specs/equivalence-contract.md` outline** (semver 0.1.0, status, changelog at the end; wording to align with `docs/specs/emitted-rust-rules.md` lines 45 (no panic, map errors to the C function's own return codes), 50 (observation point: return values, every byte written to output buffers including partial writes on error paths, boundary-visible global state), 71 (UB decisions log actions: return the policy error, panic, preserve) and `docs/specs/product-spec.md` lines 157 (refinement, not equality) and 174 (fix the observation point at the C interface)):
1. Purpose and non-claims (say that the contract makes no certification or conformance claim, without using the two banned words, so the doc check can be a plain word search)
2. Definitions: module, entry, vector, outcome class, defined-behavior input
3. Refinement: for every input where the C has defined behavior, Rust and C agree on the compared fields; on inputs where it does not, Rust takes the policy action and never reads or writes out of bounds or corrupts memory
4. Observation point at the C interface (D-08): `rc`, whole output buffer after sentinel prefill (D-07), `lib_state_out` including writable globals (D-10), stderr outcome class only
5. Field convention (D-05) with the byte codec (Q1 point 3), `rc` conventions, `has_ub` as an absent key when none (Q1 point 1)
6. `lib_state` typing (D-06) and the unobserved-marks rule with count (D-09)
7. Comparison rule (D-08) and the `has_ub` exclusion; "UB unchecked" wording (D-20)
8. UB action menu (D-17) and policy file reference (D-15, D-16, D-18), including the default-change record
9. Crash, fault, timeout and abort outcomes (D-02, D-03); sandbox and determinism summary
10. Reference executor definition (D-01): objects, qemu-arm user mode, CPU model, guard pages (D-04) with their stated limits (Q2), and the two known differences from hardware
11. Out of scope (Phase 4 UB detection, Phase 5 fuzzing, Phase 12 TRACTOR licence)
12. Changelog

### Q8. Crate layout and CLI

| Crate | Contents | Depends on |
|---|---|---|
| `mt-contract` (new) | `Policy`, `Action`, `Override`, `CONTRACT_VERSION`, `validate(&Policy, &[Site])`, `override_report` | serde, toml, thiserror only |
| `mt-vectors` (new) | `Vector` (TRACTOR-compatible, all optional), `CompleteVector` (executor-complete validation), `Output{pattern}`, `LibStateValue`, byte codec, canonical JSON writer, `Corpus` (sha256-named files + `index.json` + provenance sidecar, modelled on `mt-seeds::index`), `TractorPin` list | serde, serde_json, sha2, thiserror; nothing internal |
| `mt-harness` (new; the name is in building-the-tool.md, which also reserves it for the Phase 5 differential runner and emulator replay, so the executor is reusable there) | adapter model, driver generator, runtime template, layout model, link/run/classify | mt-capture, mt-toolchain, mt-vectors, mt-seeds, object |
| `mt-capture` (extend) | manifest `Entry` gains `arg`, `setup`, `unobserved`; `[exec]`; role validator | unchanged |
| `mt-cli` (extend) | `mt vectors` group | all |

Do not put the executor in `mt-vectors`: that crate must stay free of process execution so Phase 5 and Phase 11 can use it as a pure schema crate. Add the three new crates to `[workspace] members`; lint config `[workspace.lints]` (unwrap/expect denied) applies via `[lints] workspace = true` like mt-capture.

CLI (argument parsing only in `mt-cli`, logic in libraries; exit status 0 only on success, same convention as `mt testbed`):
- `mt vectors validate <path>...` — schema-check files or a corpus dir (`--tractor` for the permissive level)
- `mt vectors policy-check <policy.toml>...` — load and validate policies
- `mt vectors adapters --manifest ... [--check-floor]` — validate roles, print drivable/dropped, enforce the floor
- `mt vectors build --manifest ... --pins ... --work-dir ... [<target>...]` — layout probe, archive, driver, link (offline)
- `mt vectors run --work-dir ... --seeds-dir ... [<target>...]` — seeds x sweep through the executor, run twice, write corpus
- `mt vectors check --work-dir ... --first A --second B` — byte-compare two corpora (CI determinism)
- `mt vectors tractor fetch|check --pin testbed/tractor-pin.toml --work-dir ...` — the only networked step; `check` loads and round-trips

### Q9. CI shape

`testbed.yml` already has `run_offline` (network none, read-only, cap-drop ALL, `/work` and `/out` writable), a `testbed` job after capture, record, replay and the determinism compare (lines about 354-540). New steps, in order, all through `run_offline` unless marked:
1. **QEMU/driver smoke** (first, small, Plan 03-03): the checklist at the end of Q2 against the pinned image, ending with a `corejson/JSON_Validate` driver built and run twice.
2. **Validate adapters and policies** (offline): `mt vectors adapters --check-floor`, `mt vectors policy-check`.
3. **Build drivers** (offline, `--network none`): `mt vectors build` for every retained pair; publish a notice with the number of drivers, the unresolved-symbol log and the sha256 of each `driver.elf`.
4. **Run vectors twice** (offline): `mt vectors run` for all seeds; the command itself fails on any twice-run mismatch.
5. **Regenerate and compare** (offline): run steps 3 and 4 again into a second output dir and `mt vectors check --first --second` (byte compare), following the existing `capture-rerun` pattern; also compare the committed sample (`testbed/vectors-sample`) with the regenerated vectors for the same seeds.
6. **TRACTOR fetch and load** (the only networked step, `run_fetch`-style container with network): `mt vectors tractor fetch` then `check` of at least 3 files and their sha256 against `testbed/tractor-pin.toml`; `continue-on-error` is not allowed (fail closed), but failure of github.com reachability should be reported as a distinct notice.

Provable only locally (this host): schema, policy, role validation, fixture-based tests, and a full executor run with host GCC 13.2.1 and QEMU 8.2.2 using local objects (development evidence, not image evidence). Provable only in CI: QEMU 7.2 behavior, GCC 14.3 objects, tinycbor and isotp pairs, image tool presence, container sandbox compatibility, TRACTOR fetch (github.com is reachable from the proxy here via raw.githubusercontent.com and git, but the CI step is the authoritative one). Read CI evidence with `gh api repos/sat-wik/migrationtool/actions/...` (log hosts blocked); publish counts as workflow annotations/notices like the existing steps.

### Plan split recommendation (D-24; each plan 1-2 weeks part-time at most, the phase total is the sum, so keep plans small)

| Plan | Scope | Requirements | Retires |
|---|---|---|---|
| 03-01 | `mt-contract` + policy TOML + `docs/specs/equivalence-contract.md` v0.1.0 + `testbed/policy/*.toml` for the 16 targets + tests | EQV-01 | wording alignment |
| 03-02 | `mt-vectors`: types, two validation levels, byte codec, canonical writer, corpus + sidecar index, TRACTOR pin file, fetch/check, ignored local test + CI step | EQV-02 | TRACTOR shape risk |
| 03-03 | **Tracer**: manifest roles for `in_buf`/`in_len`, runtime template, link recipe, `corejson/JSON_Validate` end to end, classification, run-twice, CI QEMU smoke | EQV-03 (start), EQV-04 | QEMU 7.2 / GCC 14.3 risk, runner stdin/cwd/budget design |
| 03-04 | Layout model (DWARF) + state decode + globals (`object`) + `out_scalar`/`state`/`out_ptr`/`inout_len`/`out_buf` roles; targets zlib, lz4, microcoap, paho, nimble, nanopb | EQV-03, EQV-04 | DWARF and decoder risk |
| 03-05 | Remaining floor targets (libfdt, cjson, tinycbor) and tier B (zcbor, qcbor, httpparser), adapter drop log, `check_floor` test, committed sample, CI determinism steps, `/gsd-verify` evidence | EQV-03, EQV-04 | scale and CI |

Plans 03-01 and 03-02 can be written in parallel with 03-03's CI smoke step because they share no files; the single builder should do the CI smoke commit first so its feedback arrives while 03-01 and 03-02 are written.

### Validation Architecture (Nyquist)

#### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness (`cargo test`), `tempfile` for work dirs; no new dev-dependencies |
| Config file | workspace `Cargo.toml`, `clippy.toml` (bans `std::process::Command`) |
| Quick run command | `cargo test -p mt-contract -p mt-vectors` |
| Full suite command | `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && cargo deny check` |

Local-only ignored tests follow the existing convention `MT_LOCAL_CAPTURES=$PWD/.mt-work/capture cargo test -p <crate> --test <file> -- --ignored --exact <name> --nocapture`; use `MT_LOCAL_TRACTOR=<dir>` for the TRACTOR test. Tests never use the network or a live model.

#### Phase Requirements to Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| EQV-01 | policy loads; unknown key rejected; default is `return_error`; changing default needs `default_change`; override missing any of 7 keys rejected; unlogged-site override rejected; empty override table accepted | unit | `cargo test -p mt-contract eqv_01` | Wave 0 |
| EQV-01 | contract doc has semver version, changelog, and states refinement, observation point, three actions; never contains the words "certified" or "compliant" | unit (doc check) | `cargo test -p mt-contract eqv_01_contract_document` | Wave 0 |
| EQV-02 | exactly nine fields in the strict level; unknown key and a merged `lib_state` rejected; `has_ub` false serializes with no key; `stdout` serializes as `{pattern}` | unit | `cargo test -p mt-vectors eqv_02` | Wave 0 |
| EQV-02 | byte codec round-trips all 256 byte values and NUL in `stdin`/`stdout` | unit (exhaustive table) | `cargo test -p mt-vectors eqv_02_byte_codec` | Wave 0 |
| EQV-02 | 3 or more TRACTOR vectors load unchanged and re-serialize JSON-equal | integration, ignored locally + CI | `MT_LOCAL_TRACTOR=... cargo test -p mt-vectors --test eqv_02_tractor -- --ignored --exact eqv_02_tractor_vectors_load_unchanged` | Wave 0 |
| EQV-02 | committed TRACTOR pin lists only paths+hashes; no TRACTOR content in repo (search for a known public-vector hash) | unit | `cargo test -p mt-vectors eqv_02_tractor_pin_has_no_content` | Wave 0 |
| EQV-03 | manifest roles: bad role, wrong parameter count, forbidden token in `setup.args`, missing `max` for a `uint8_t` length are rejected | unit | `cargo test -p mt-capture eqv_03_roles` | Wave 0 |
| EQV-03 | drivable targets >= 10 (floor) from the committed manifest + adapter drop log | unit (committed files) | `cargo test -p mt-harness eqv_03_adapter_floor_is_ten` | Wave 0 |
| EQV-03 | generated driver source is stable (golden) for corejson, zlib, microcoap | golden | `cargo test -p mt-harness eqv_03_driver_golden` | Wave 0 |
| EQV-04 | link recipe refuses an object whose sha256 differs from `capture.json` | unit | `cargo test -p mt-harness eqv_04_refuses_modified_object` | Wave 0 |
| EQV-04 | classification table maps each runner result to one outcome class (table-driven over synthetic `RunRecord`s) | unit | `cargo test -p mt-harness eqv_04_classify` | Wave 0 |
| EQV-04 | layout probe parser on a committed llvm-dwarfdump text fixture (struct with bitfield, array, enum, union, typedef) | unit | `cargo test -p mt-harness eqv_04_layout_parser` | Wave 0 |
| EQV-04 | run twice gives identical vector bytes (corejson, zlib) with local host tools | integration, ignored locally + CI | `cargo test -p mt-harness --test eqv_04_exec -- --ignored --exact eqv_04_run_twice_is_byte_identical` | Wave 0 |
| EQV-04 | guard-page overrun yields `fault`, `rc = -11`, `has_ub` label, no core file | integration, ignored locally + CI | `... eqv_04_overrun_is_a_fault` | Wave 0 |
| EQV-04 | timeout yields `timeout`, never `ok` | integration (infinite-loop fixture) | `... eqv_04_timeout_is_recorded` | Wave 0 |
| D-05 / D-07 / D-08 | output buffer sentinel survives untouched bytes; whole buffer recorded | integration | `... eqv_04_sentinel_prefill_recorded` | Wave 0 |
| D-19 / D-21 | provenance sidecar hashes match; committed sample regenerates identically | integration in CI | `mt vectors check` (CI step 5) | Wave 0 |
| workflow | `testbed.yml` contains the new steps, `--network none` for all but TRACTOR fetch | static workflow test | `cargo test -p mt-capture eqv_04_workflow_static` (pattern of `build_02_workflow_static`) | Wave 0 |

#### Sampling Rate

- **Per task commit:** `cargo test -p mt-contract -p mt-vectors` (seconds), plus the crate under change.
- **Per wave merge:** full suite command above, plus the ignored local executor tests on this host.
- **Phase gate:** full suite green and a green `testbed` run on the final code commit with the new notices (driver count, vectors, twice-run mismatches = 0, TRACTOR loaded >= 3), before `/gsd-verify-work`.

#### Wave 0 Gaps

- [ ] `crates/mt-contract` and `crates/mt-vectors` crates, `crates/mt-harness`, tests named above
- [ ] fixtures: llvm-dwarfdump text fixture, synthetic `RunRecord`s, a tiny infinite-loop and an overrun fixture built from the runtime template (generated at test time, not committed as C)
- [ ] CI QEMU smoke step (Q2 list) so the 7.2 assumptions are measured before 03-04
- Framework install: none.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Struct layout for ILP32 | A C declaration parser or offset tables in Rust | DWARF from the pinned GCC via `llvm-dwarfdump` text | The compiler already computed it, including short enums and bitfields; a parser would miss them (Pitfall 2) |
| Process execution, env, timeouts, output hashing | `std::process::Command` | `mt_toolchain::runner::run` | Clippy bans `Command`; the runner records evidence and kills process trees |
| ELF symbol reading | Own ELF parser | `object` (locked) | Bounds-checked, already used in `mt-capture::elf` |
| Flag selection from a captured compile | New allow-list or regex | `mt_capture::entries::filter_flags`, `include_prefix`, `cc` walker | Phase 2 measured and tested these against GCC quirks |
| sha256 corpus + index | New storage format | The `mt-seeds::index` pattern (sha256 file names, `index.json`, drift check, NOTICE) | D-19/D-21 say "like the seed corpora" |
| Tree/commit pinning of fetched content | New hash scheme | Commit SHA + per-file sha256 (TRACTOR sparse set); `fetch.rs` flow for the git calls | Same trust model as Phase 2 |
| Deterministic archive | Own archiver | `arm-none-eabi-ar rcsD` | `D` zeroes timestamps and owners |
| JSON for vectors | Hand string building | serde with a fixed-order struct and `BTreeMap` | Canonical, testable, no escape bugs |
| Drop log and floor | New logging scheme | `DropLog` TOML pattern and `check_floor` arithmetic | Phase 2 convention (D-13) |

**Key insight:** every hard part (layout, flags, process control, hashing, storage) already has a trusted source; the new code is glue plus the closed role grammar and the decoder.

## Common Pitfalls

### Pitfall 1: `"has_ub": false` marks a vector as UB in TRACTOR tooling
**What goes wrong:** cando's `deserialize_has_ub` returns true whenever the key is present, whatever its value, so a serializer that writes `false` makes every vector "skipped".
**Why it happens:** D-20 is worded as `has_ub = false`; a naive `bool` field serializes it.
**How to avoid:** `Option` or skip-if-false serialization; a test that the serialized JSON of a non-UB vector has no `has_ub` key; an unmarshal test that `false` and `null` in a TRACTOR file are read as present.
**Warning signs:** `"has_ub"` appears in more than the crash-observed vectors.

### Pitfall 2: Enum width differs between GCC and clang for this target
**What goes wrong:** layout from clang records 4-byte enums where the captured objects use 1-byte enums, so every enum-typed state field and any struct after it is decoded wrongly.
**How to avoid:** take layout from the pinned GCC's DWARF only; add a test that `sizeof` of an enum in a fixture is 1 (the `Tag_ABI_enum_size: small` attribute is also readable from the captured object with `readelf -A`).

### Pitfall 3: QEMU writes core files and passes the host environment
**What goes wrong:** an uncaught guest SIGSEGV makes QEMU write `qemu_*_<time>_<pid>.core` to the cwd (if RLIMIT_CORE allows) and die with a signal; guest stack addresses move with environment size.
**How to avoid:** in-guest handlers that exit 128+sig, own stack on a fixed mapping, `-R`, `-seed 0`; treat any `exit.signal` as `executor_error`.

### Pitfall 4: libc output in the result channel
**What goes wrong:** `printf` inside the target (microcoap DEBUG, zcbor, mbedtls) writes to fd 1 and corrupts the frame.
**How to avoid:** newlib `_write` shim discards and counts; result goes out through a raw syscall in a framed, length-checked, hashed record; the executor rejects anything else.

### Pitfall 5: Binary data in TRACTOR's string fields
**What goes wrong:** bytes >= 0x80 are not valid UTF-8 alone; naive `String::from_utf8_lossy` destroys them and breaks byte-identical re-runs.
**How to avoid:** one codec in `mt-vectors` (Latin-1 identity, pending Open Question 1), used by every reader and writer; exhaustive 256-value test.

### Pitfall 6: `-R` and fixed addresses must agree
**What goes wrong:** a mapping above the reserved VA fails and the driver exits 98; this looks like a fault.
**How to avoid:** keep all driver mappings below 0x10000000 and reserve exactly that; exit codes 96..=99 are `executor_error`, not outcomes.

### Pitfall 7: Seeds are tied to entries
**What goes wrong:** three entries have no seeds (`cbor_decode_Pet`, `http_parser_parse_url`, `mbedtls_asn1_get_tag`), and `pb_decode_ex` and `LZ4F_getFrameInfo` seeds are really stream data for a sibling entry.
**How to avoid:** the runner selects seeds by `entries` in `index.json`; zero seeds is the logged reason `no_seeds`, never silent borrowing.

### Pitfall 8: Local objects are not CI objects
**What goes wrong:** local `.mt-work` objects come from host GCC 13.2.1 (`.comment` shows it), tinycbor and isotp have none locally, and `capture.json` says `"image": "local"`. A vector generated locally is development evidence only.
**How to avoid:** provenance records the image digest or `local`; the committed sample is generated in CI (or marked `image: local` and regenerated by the first green CI run); CI is the judge.

### Pitfall 9: Stack protector, assert and abort
**What goes wrong:** cjson is built with `-fstack-protector-strong` and http_parser, lz4 and nimble call `__assert_func`; both end in newlib's `abort`, which calls `raise(SIGABRT)` and then `_kill`/`_getpid`, which the bare-metal stubs fail or loop on.
**How to avoid:** the runtime template maps `_kill` to the fault exit (128+6); newlib's canary is a fixed constant after `__libc_init_array` (measured), so no canary definition is needed; classification per Open Question 4.

### Pitfall 10: Manifest strings become C
**What goes wrong:** role fields, setup `call` and constants are pasted into generated C; a malformed manifest could inject code into something that is compiled and run.
**How to avoid:** closed grammars (Q5), identifier regexes, no free text; golden-test the generated source.

### Pitfall 11: Guard pages are page-granular
**What goes wrong:** claims of "guard before the first byte" are only true when `len` is a multiple of 4096; a small underflow reads slack bytes.
**How to avoid:** state the limit in the contract; fill slack with a pattern and re-check for writes; end guard is byte-exact.

## Code Examples

Own illustrations (not TRACTOR content). The runtime template (shipped as an `include_str!` constant in `mt-harness`, shared by all targets, so no per-target hand-written C), the parts that were run in the experiments:

```c
/* svc wrapper: Linux ARM EABI, r7 = number [VERIFIED: ran under qemu-arm -cpu cortex-m4] */
static long sc(long n, long a, long b, long c, long d, long e, long f) {
    register long r7 __asm__("r7") = n;  register long r0 __asm__("r0") = a;
    register long r1 __asm__("r1") = b;  register long r2 __asm__("r2") = c;
    register long r3 __asm__("r3") = d;  register long r4 __asm__("r4") = e;
    register long r5 __asm__("r5") = f;
    __asm__ volatile("svc #0" : "+r"(r0) : "r"(r7),"r"(r1),"r"(r2),"r"(r3),"r"(r4),"r"(r5) : "memory");
    return r0;
}
/* guarded buffer, right-aligned: guard page before and after, last byte right before the end guard */
static unsigned char *guarded(uint32_t base, size_t len) {
    size_t pages = len ? (len + 4095) / 4096 : 1;
    if (sc(192 /*mmap2*/, base, (pages + 2) * 4096, 3, 0x32 /*PRIVATE|ANON|FIXED*/, -1, 0) != (long)base) _exit(98);
    sc(125 /*mprotect*/, base, 4096, 0, 0, 0, 0);
    sc(125, base + (pages + 1) * 4096, 4096, 0, 0, 0, 0);
    return (unsigned char *)(base + 4096) + pages * 4096 - len;   /* len == 0: points at the end guard */
}
/* fault handler: exit 128 + signo, no core file */
struct ksa { void (*h)(int); unsigned long flags; void (*rest)(void); unsigned long long mask; };
static void on_fault(int sig) { sc(248 /*exit_group*/, 128 + sig, 0, 0, 0, 0, 0); }
/* register for 11, 7, 4, 8, 6 with: sc(174 /*rt_sigaction*/, sig, (long)&sa, 0, 8 /*sigsetsize*/, 0, 0); flags = SA_ONSTACK (0x08000000)
   after sigaltstack (186). The sigsetsize argument (8) is required: omitting it makes the call fail silently. */
/* _start: align sp, call C. Thumb-2 cannot use `bic sp, sp, #7`; go through a low register. */
```

Build line (the same flags as the capture, from `[baseline]` `cflags = ["-mcpu=cortex-m4", "-mthumb", "-mfloat-abi=hard", "-mfpu=fpv4-sp-d16", "-Os"]` [VERIFIED: testbed/manifest.toml:43]):
`arm-none-eabi-gcc <baseline -m flags> <TU -I/-D/-std> -c driver.c -o driver.o` then `arm-none-eabi-gcc <baseline -m flags> -nostartfiles --specs=nosys.specs -o driver.elf driver.o pair.a -lm`; run: `qemu-arm -cpu cortex-m4 -R 0x10000000 -seed 0 driver.elf req.bin`.

Vector shape produced by the executor (field-shape illustration):
```json
{
  "argv": ["corejson/default/JSON_Validate"],
  "stdin": "{\"a\":[1,2]}",
  "env": {},
  "stdout": {"pattern": ""},
  "stderr": {"pattern": "outcome=ok"},
  "rc": 1,
  "lib_state_in": {},
  "lib_state_out": {}
}
```
(a crash adds `"has_ub": "crash:SIGSEGV"` and `"rc": -11`; no `has_ub` key otherwise).

Policy loader sketch (mirrors `Manifest`'s strict parsing): `#[derive(Deserialize)] #[serde(deny_unknown_fields)] struct Policy { schema_version: u32, contract_version: String, module: String, ub: Ub }`, `toml::from_str`, then rule checks returning `thiserror` variants.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Bare-metal QEMU system mode for firmware replay | Hosted-style driver of the same object code under qemu-arm user mode | this phase | 7x faster, byte-exact end guard, simple fault surface |
| Clang record layouts for target structs | Pinned GCC DWARF 5 | measured here | Correct short enums |
| TRACTOR `has_ub` as a free label | Absent key when none | cando source at pin | Avoids accidental skipping |

**Deprecated/outdated:** `--specs=linux.specs` for these objects (crashed); semihosting for I/O (works but cannot do mmap guards); the pins.toml comment "user mode cannot run bare-metal Cortex-M images" needs rewording to "unmodified firmware images".

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | qemu-arm 7.2 in the image runs `-cpu cortex-m4` drivers like 8.2.2 (availability is cited from 7.2 source; behavior was measured on 8.2.2) | Q2 | Fall back to `-cpu cortex-a15` (identical results on my tests) and record it; flag to founder |
| A2 | GCC 14.3 produces the same layout and DWARF 5 member forms as 13.2.1 | Q4 | Layout tests on CI fixtures would fail early |
| A3 | The image's llvm-16 package includes `llvm-dwarfdump` with the same text for these tags | Q4 | Use `arm-none-eabi-readelf --debug-dump=info` instead |
| A4 | `arm-none-eabi-ar/nm/size` exist in the Arm 14.3 tarball `bin` | Q3 | Then use `llvm-ar` from llvm-16 (also `rcsD`) |
| A5 | `-fPIC` captured objects (httpparser) link statically with newlib 14.3 | Q3 | Link failure logged per pair |
| A6 | Container flags (`--cap-drop ALL`, `no-new-privileges`, default seccomp, read-only) allow qemu-arm user mode | Q9 | Step 1 smoke fails; adjust mounts, not design |
| A7 | Latin-1 identity byte codec is acceptable | Q1 | Re-key every stored vector; costly, so confirm before 03-02 |
| A8 | Accepting and never emitting TRACTOR's `note` key is consistent with "exactly nine fields" | Q1 | Reject `note` and choose vectors without it |
| A9 | Treating `assert`/`abort` as a fault with `has_ub` label is the right D-03 reading | Q6 | Phase 4 relabels; classification code is one table |
| A10 | Budget values (10 s, 256 MiB VA, 1 MiB arena, sentinel 0xA5, 16 combos) are adequate for all seeds | Q6 | Tune; they are provenance fields (D-19) |
| A11 | Adding optional manifest keys without a schema bump is acceptable | Q5 | Bump to 2 and update Phase 2 tests |
| A12 | libfdt and the other floor targets do not require an aligned input buffer | Q5 | Add `align` to the role; default 1 already allowed |

## Open Questions (RESOLVED)

All eight questions were answered by the founder decisions of 2026-10-21 (03-CONTEXT.md D-25..D-30) or are documented in the plans; each item below names its resolution.

1. **Byte codec for binary data inside TRACTOR strings (needs founder confirmation).** RESOLVED by D-25: Latin-1 identity in `stdin` and `stdout.pattern`, `is_regex: false`, `rc = -signal` on faults.
   - Known: `stdin` and `stdout.pattern` are JSON strings with no binary convention in 2,610 public vectors.
   - Unclear: whether TRACTOR tools must be able to feed our `stdin` to a process unchanged.
   - Recommendation: Latin-1 identity, written in the contract; revisit only if a TRACTOR runner must execute our vectors.
2. **`note` key.** RESOLVED by D-26: accepted on load, never emitted, not counted in the nine fields. Recommendation: accept on load, preserve, never emit, not counted in the nine.
3. **argv[0] and rc meaning differ from TRACTOR's executables** RESOLVED (documented, no decision needed): D-05 and D-25 fix the convention, and contract v0 states it (plan 03-02, section "Field convention"). (first argument; process exit code). No action; document in the contract.
4. **Assert/abort classification.** RESOLVED by D-27: outcome class `abort` with a `has_ub` crash label, excluded from comparison; Phase 4 may relabel. An `assert` in a library built without `NDEBUG` is defined library behavior, not C UB. Recommendation: outcome class `abort`, vector `has_ub` label `crash:SIGABRT` with source `observed_crash` so it is out of comparison; Phase 4 may relabel. Founder may prefer a separate "library-abort" label.
5. **Entries with no seeds** RESOLVED by D-28: logged `no_seeds`, never borrowed from a sibling entry. (`cbor_decode_Pet`, `http_parser_parse_url`, `mbedtls_asn1_get_tag`). Recommendation: log `no_seeds`; floor does not depend on them. Alternative: allow a manifest key `seed_from = ["other_entry"]` for explicit borrowing.
6. **Input transport.** RESOLVED by D-30: the input goes through a file in the fixed working directory; the runner is not changed. Recommendation: `req.bin` in a fixed cwd. Alternative: add stdin to the runner.
7. **Shared runtime template vs D-11 "no hand-written per-target C".** RESOLVED by D-29: one shared, reviewed template is not per-target C. The runtime is one shared template inside `mt-harness`, not per-target. Confirm this reading.
8. **`proptest`.** RESOLVED by D-29: no new crates; table-driven tests. building-the-tool.md suggests it for schema round-trips; it is not locked. Table-driven tests are proposed instead; ask only if wanted.

## Environment Availability

| Dependency | Required By | Available (this host) | Version | Fallback |
|------------|------------|-----------|---------|----------|
| arm-none-eabi-gcc | driver, probe | yes | 13.2.1 (image pins 14.3.Rel1) | CI only for evidence |
| arm-none-eabi-ar/nm/readelf | archive, symbols | yes | binutils with 13.2.1 | `llvm-ar` |
| clang | none needed (layout from GCC) | yes | 18.1.3 (image 16.0.6) | — |
| llvm-dwarfdump | layout | yes | 18.1.3 (image llvm-16) | `arm-none-eabi-readelf --debug-dump=info` |
| qemu-arm | executor | **installed during this research** | 8.2.2 (image 7.2) | — (was absent before) |
| qemu-system-arm | comparison only | installed during this research | 8.2.2 | not needed |
| docker | local image runs | no | — | CI `testbed` job |
| cmake, bear, make | Phase 2 capture only | cmake, bear present | — | — |
| git + network to github.com | TRACTOR fetch | yes (git and raw.githubusercontent.com work; `gh api` and github.com web are blocked for that repo) | — | CI fetch step |
| python3 | research scripts only | yes | — | not part of the tool (ADR-0001) |
| Local objects | executor development | 14 targets, 27 of 31 pair directories (tinycbor x2 `build_failed`, isotp and paho/server `capture_failed` locally) | host GCC 13.2.1 | CI objects |

**Missing dependencies with no fallback:** none for planning. CI-only proofs are listed in Q9.
**Disk:** about 5.5 GB free after clean-up; scratch directories created for this research were removed.

## Security Domain

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | no | container sandbox, read-only mounts (existing) |
| V5 Input Validation | yes | strict serde (`deny_unknown_fields`), closed manifest grammars, bounded reads, vector size caps, path-component checks for corpus names (sha256 only) |
| V6 Cryptography | no (sha256 only, `sha2`) | never hand-roll |

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Manifest or vector text injected into generated C or a shell | Tampering / Elevation | no shell (argv vectors), closed grammars, identifier regexes, golden tests of generated source |
| Untrusted TRACTOR JSON (sizes, deep nesting, NaN) | Denial of service | file size cap, serde_json depth limit (default 128), per-file timeout in tests |
| Fetched corpus tampered | Tampering | commit pin + per-file sha256 check, never executed |
| Guest code escaping the emulator or reaching the network | Elevation | `--network none` container, read-only fs, empty env, QEMU user mode only runs captured open-source objects; no semihosting host file access |
| Core files or temp files outside work dir | Information disclosure | in-guest fault handler, fixed cwd inside the work dir, nothing written elsewhere |
| Held-out corpus leakage | Information disclosure | not touched in this phase |

## Project Constraints (from CLAUDE.md)

- `#![forbid(unsafe_code)]` in every tool crate; `thiserror` in libraries, `anyhow` only in `mt-cli`; no `unwrap`/`expect` outside tests without an invariant comment (workspace lints deny both).
- Every external tool runs through the `mt-toolchain` runner; never `std::process::Command` directly (clippy.toml enforces).
- Deterministic outputs: explicit seeds and budgets, `BTreeMap`, `schema_version` in every machine file, timestamps only in manifest headers.
- Two pinned Rust toolchains; one LLVM major 16-19; the executor uses the arm-none-eabi GCC pin for the driver and llvm-16 only for `llvm-dwarfdump`.
- Each requirement ID has a test whose name contains it in snake case (`eqv_01_...`); tests never use the network or a live model.
- Pre-merge: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo deny check`, container version check.
- Python only under `research/`; analysers only as subprocesses; held-out corpus never read.
- Reports: timeouts are "not proven", missing layers "unavailable", never "certified" or "compliant"; vectors without Phase 4 labels are called "UB unchecked".
- Do not copy TRACTOR content into the repo before Phase 12; do not depend on or redistribute Hayroll (not used here).
- Phase loop: discuss, plan, execute, verify, code-review, ship; no direct repo edits outside a GSD command. This research made no repo edits other than this file.

## Sources

### Primary (HIGH confidence)
- Repo files read this session: `crates/mt-toolchain/src/runner.rs`, `crates/mt-capture/src/{cc.rs,manifest.rs,entries.rs,droplog.rs,testbed.rs,fetch.rs}`, `testbed/manifest.toml`, `testbed/seeds/*/index.json`, `container/pins.toml`, `.mt-work/capture/*/*/{capture.json,compile_commands.json}`, `docs/specs/emitted-rust-rules.md`, `docs/specs/product-spec.md`, `docs/guidelines/building-the-tool.md`, `.github/workflows/testbed.yml`
- TRACTOR `PUBLIC-Test-Corpus` at commit `37960ee08a7c644ee66e29855eca33903f702e67`: `README.md`, `LICENSE`, `tools/cando2/src/test_vector.rs`, `tools/cando2/src/state.rs`, 2,610 vector files under `Public-Tests/B0*`
- Measurements on this host: qemu-arm 8.2.2, qemu-system-arm 8.2.2, arm-none-eabi-gcc 13.2.1, llvm-dwarfdump 18.1.3, clang 18.1.3

### Secondary (MEDIUM confidence)
- QEMU v7.2.0 source from `raw.githubusercontent.com/qemu/qemu/v7.2.0/`: `target/arm/cpu_tcg.c` (cortex-m4 in the table), `semihosting/arm-compat-semi.c`, `include/semihosting/semihost.h`, `linux-user/arm/cpu_loop.c`

### Tertiary (LOW confidence)
- Nothing in this document rests on web search alone.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - all crates already locked; tools are pins.
- Architecture: MEDIUM-HIGH - pipeline measured end to end for corejson and zlib on 8.2.2/13.2.1; image versions differ.
- Pitfalls: HIGH - each pitfall was observed or read from source.
- Adapter inventory: MEDIUM - roles proposed from signatures, headers and link/symbol surveys; two targets (corejson, zlib) were run end to end and nine pairs were link-tested.

**Research date:** 2026-10-21
**Valid until:** 2026-11-20 (the TRACTOR repo moves; the pin makes it stable; re-check only if the commit changes)
