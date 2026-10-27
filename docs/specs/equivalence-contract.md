---
type: SPEC
title: "Equivalence contract for migrated C modules"
date: 2026-10-21
status: v0, MVP (milestone v1.0)
enforced_by: mt-contract (policy loader), mt-vectors (vector schema), mt-harness (C reference executor, Phase 5 comparators)
---

# Equivalence contract

**Version:** 0.1.0
**Status:** v0

This document defines what "the Rust is equivalent to the C" means for every module the tool migrates. The vector schema (Phase 3), the UB log (Phase 4), the comparators (Phase 5) and the evidence bundle (Phase 11) all read this one definition. `mt_contract::CONTRACT_VERSION` equals the version line above, and a test in the `mt-contract` crate fails if the two differ. A change to the contract bumps the version, adds a line to the changelog, and moves `CONTRACT_VERSION` and every policy file under `testbed/policy` together.

## Purpose and non-claims

The contract decides four things: what counts as the same behavior (refinement), where behavior is observed (the C interface), how a test vector records it (the field convention and the library-state typing), and what the Rust does on inputs where the C has no defined behavior (the UB action menu and the per-module policy).

The contract makes no certification claim and no conformance claim, for any product or any module. It describes how the tool compares two programs. A module that passes every comparison has matched the C on the inputs that were run, under the stated conditions; reports say so in those words. A run that hit its time budget is reported as "not proven", and a layer that did not run is reported as "unavailable". A vector that has not been checked for UB is called "UB unchecked", never "no UB".

## Definitions

- **Module:** one unit of C migrated together, named by a manifest target id such as `corejson`. A module has one policy file.
- **Entry:** one C function of a module that the harness calls, as declared in the manifest.
- **Adapter:** the manifest declaration of an entry's argument roles (input buffer and length, output buffer and capacity, scalars, state structs, setup steps). The harness generates the driver from it; no per-target C is written by hand.
- **Vector:** one JSON object holding the inputs of one call of one entry and the result the C produced, in the nine fields of the field convention below.
- **Defined-behavior input:** an input on which the C code has defined behavior on the target. Every input is treated as defined until a crash, a sanitizer finding or a later UB tool says otherwise.
- **Outcome class:** how one execution ended: `ok`, `fault`, `abort`, `timeout` or `setup_failed`. `executor_error` is not an outcome of the C code (see Outcomes).
- **Observation point:** the C interface of the module, the only place behavior is compared.
- **Policy:** the per-module file that names the action taken on inputs without defined behavior.

## Refinement

Equivalence is refinement, not equality. On every input where the C has defined behavior, the Rust agrees with the C on the compared fields (see Comparison rule). On every other input, the Rust takes the action the module's policy names. Under every action the Rust never reads or writes out of bounds and never corrupts memory. The Rust may do better than the C on an input where the C is undefined; it may not do worse.

## Observation point

Behavior is observed at the C interface and nowhere else. Internals may differ freely. What crosses the interface is:

- the return value;
- the whole output buffer after the call, filled with a sentinel byte before the call. The sentinel is `0xA5`. The executor records the entire buffer, so bytes the C left untouched and partial writes on error paths are both part of the result, as in the emitted-Rust rule on partial writes;
- every state struct passed by pointer, with every field its header defines, in the layout of the compiled C for the target (ILP32, little-endian, plain `char` unsigned);
- every writable global of the target's own `.data` and `.bss`, found automatically from the symbol tables of the captured objects, snapshotted before and after the call.

A field of a state struct may be marked unobserved only with a recorded reason, and the evidence counts such marks (see Library state).

## Field convention

A vector holds at most the nine fields of the TRACTOR test-vector format: `argv`, `stdin`, `env`, `stdout`, `stderr`, `rc`, `lib_state_in`, `lib_state_out` and `has_ub`. There are two library-state fields, not one. Provenance (target, config, entry, object hashes, image digest, executor version, budgets, the source of `has_ub`) does not go into the vector; it lives in the sidecar index of the corpus, keyed by the sha256 of the vector.

| Field | Contents |
|---|---|
| `argv` | `argv[0]` is `<target>/<config>/<entry>`, a first argument in TRACTOR's convention (TRACTOR appends `argv` after the driver path; our vectors are not run by TRACTOR's runner). `argv[1..]` are the scalar arguments (lengths, flags, capacities) as decimal text, in the order the adapter declares. |
| `stdin` | The input buffer bytes, written with the byte codec below. |
| `env` | Always `{}`. |
| `stdout` | The output buffer bytes after the call, as `{"pattern": "<string>", "is_regex": false}`; the pattern uses the byte codec. A bare string is not written. |
| `stderr` | The executor's own text only, as `{"pattern": "outcome=<class>"}` with `is_regex` false, optionally followed by fixed detail words (for example `step=<k>` after `setup_failed`). It never holds guest output, an address or a time. |
| `rc` | The C return value as a 32-bit signed integer. A `void` function gives 0; a pointer gives 0 for NULL and 1 otherwise; a struct returned by value or a return wider than 32 bits goes to `lib_state_out.ret` and `rc` is 0. On a fault `rc` is `-signal` (TRACTOR's convention, for example -11). On a timeout `rc` is -9. |
| `lib_state_in` | The state after any setup steps and before the entry (see Library state). |
| `lib_state_out` | The state after the entry. |
| `has_ub` | Absent when no UB was observed, and never `false`: TRACTOR's tools treat any present `has_ub` key as UB. When present it is a label naming its source (see Outcomes). |

**Byte codec.** Bytes go into the text fields `stdin` and `stdout.pattern` by the Latin-1 identity: byte `b` becomes the character U+00`b`, written as a JSON string with `\u00XX` escapes for control characters, `"` and `\`. It is lossless for all 256 byte values. The codec is the same everywhere in the tool; decoders reject a character above U+00FF.

**`is_regex` is always false** in our vectors: output is compared exactly.

**Loading TRACTOR vectors.** A TRACTOR vector with its optional `note` key, or with an integer or string `has_ub`, loads unchanged. Our own vectors never emit `note`. A key that is not one of the nine (and not `note`) is refused, which also refuses a single merged `lib_state`.

**UB unchecked.** A vector with no `has_ub` key has not been proven free of UB; it is "UB unchecked" until Phase 4's tools have looked at it, and the sidecar index records which source applies (an observed crash, unchecked, or Phase 4).

## Library state

`lib_state_in` and `lib_state_out` are JSON objects of named fields. Each field carries its C type and an exact value; a pointer is never a raw address.

- Integer, enum, bool or char of at most 32 bits: `{"type": "<C type>", "value": <JSON integer>}`. A 64-bit integer has `"value": "<decimal string>"`.
- `float` and `double`: `{"type": "<C type>", "bits": "<IEEE bit pattern, lowercase hex>"}`. The bit pattern, not decimal text, because NaN and infinity cannot be written as JSON numbers and decimal text is not an exact value.
- Array of a 1-byte element type: `{"type": "<C type>", "hex": "<lowercase hex>"}`. Other arrays: `{"type": "<C type>", "items": [...]}`.
- Struct: `{"type": "struct X", "fields": {"<member>": <value>, ...}}`. A union records every member, each decoded from the same bytes.
- Pointer: `{"type": "T *", "pointer": null}` for NULL; `{"type": "T *", "pointer": {"buffer": "<region>", "offset": <n>}}` otherwise. This is the `{buffer, offset}` form: the region is `in`, `out`, a declared state or local name, `heap`, or `global:<object>:<symbol>`, and the offset is in bytes. A function pointer is `{"type": "...", "pointer": {"symbol": "<function>"}}`.
- A field marked unobserved: `{"type": "<C type>", "unobserved": "<reason>"}`. Unobserved fields are not compared. Each needs a reason, and the number of such marks is counted in the provenance of the corpus.

Reserved top-level names in a `lib_state` object:

- `globals`: the writable globals whose bytes differ from the image's initial bytes, keyed `<object path>:<symbol>`.
- `ret`: a struct returned by value, the full value of a return wider than 32 bits, or a returned pointer classified like any pointer.
- `heap`: the objects in the driver's arena that are reachable from recorded pointers through typed edges, keyed by decimal offset, each decoded with its pointee type. A `char` pointee is recorded as the NUL-terminated bytes in hex. The walk stops after 4096 objects; the remaining pointers are then recorded as unobserved with a reason.
- `in`: the input buffer's bytes after the call, only for an input the adapter declares mutable.

## Comparison rule

The compared fields are `rc`, `stdout.pattern`, `lib_state_out`, and the outcome class word of `stderr` (the text after `outcome=` up to the first space). Not compared: `argv`, `stdin`, `env`, `lib_state_in`, the detail words of `stderr`, and fields marked unobserved.

A vector with `has_ub` is excluded from output comparison: the Rust is checked on it only for the safe action its module's policy names, never for the C's output (Phase 5 does this). TRACTOR excludes such vectors from scoring in the same way. A vector without `has_ub` is compared in full and is called "UB unchecked" until Phase 4 labels it. A timeout is never a pass.

## UB action menu and policy

The menu has three actions.

- **`return_error`:** the function returns a declared error code and makes no further writes.
- **`panic`:** the Rust aborts. Emitted Rust builds with `panic = "abort"`, so this is a trap, and the vector records the trap as a crash outcome.
- **`preserve`:** the Rust reproduces what the compiled C did on the target, and that behavior is recorded per site as evidence.

Under every action the Rust never reads or writes out of bounds and never corrupts memory. The default for every module is `return_error`.

The policy is one TOML file per module, loaded by `mt_contract::Policy::load` (the test bed keeps them in `testbed/policy/<module>.toml`). Unknown keys are rejected at every level. The keys:

- `schema_version` (1), `contract_version` (the version of this document the file implements, as `MAJOR.MINOR.PATCH`, not above the loader's `CONTRACT_VERSION`) and `module` (lowercase letters, digits, `_` and `-`; equal to the file stem).
- `[ub] default_action`: one of the three actions.
- `[ub.default_change]`: required when the default is not `return_error`, and refused while it is. A record of `module` (the file's module), `reason`, `approver` and `date`. This is the recorded setting that changes a module's default.
- `[[ub.error_code]]`: `function` (a C identifier) and `value` (a decimal integer or a C identifier naming a constant): the code `return_error` returns for that function. At most one per function. The committed policies leave the list empty; the UB decisions log of Phase 4 fills it.
- `[[ub.override]]`: a per-site override with all seven keys `function`, `kind`, `location`, `action`, `reason`, `approver` and `date`. A site is identified the way the UB log identifies it, by exact equality of `function`, `kind` and `location`; two overrides for one site are refused. An override that names a site the UB log did not record is refused. Until Phase 4 there is no UB log, so the loader is given an empty list of known sites and every committed override table is empty.

Every `reason` and `approver` must be non-empty, and every `date` must be a real calendar day written `YYYY-MM-DD` (Gregorian leap years; no clock is read). A record missing any part, or holding an empty one, is refused. `Policy::override_report` lists the module, the default, the default change and every override with all its fields, ordered by function, kind and location, so an evidence bundle shows where a default was changed and by whom. `mt vectors policy-check` loads and validates policy files, and with `--manifest` checks that there is exactly one for each test-bed target.

## Outcomes

Every execution of the C ends in one outcome class, recorded as `outcome=<class>` in `stderr`.

| Class | When | `rc` | `has_ub` label | Compared |
|---|---|---|---|---|
| `ok` | the call returned and the result was read | the return value | none | yes |
| `fault` | the C code faulted (SIGSEGV, SIGBUS, SIGILL or SIGFPE), or wrote a changed byte into the slack beside a guarded buffer | `-signal`; for a slack write the call's return value | `crash:SIGSEGV`, `crash:SIGBUS`, `crash:SIGILL`, `crash:SIGFPE`, or `oob-write:guard-slack` | no, UB |
| `abort` | `abort()` or a failed `assert` | -6 | `crash:SIGABRT` | no, UB |
| `timeout` | the time budget ran out | -9 | `timeout` | no; never a pass, reported as "not proven" |
| `setup_failed` | a declared setup step before the entry returned an unexpected value | the step's return value | none | yes: the setup call's behavior on that input is defined behavior |

A `setup_failed` vector has `outcome=setup_failed step=<k>` in `stderr`, and `lib_state_in` and `lib_state_out` both hold the state at the stop. Phase 4's tools may refine or relabel the crash labels.

`executor_error` means the executor itself could not judge a run: the emulator died of a host signal, the driver could not start, or its result was malformed. It stops the run, is reported, and stores no vector. It is never an outcome of the C code and never a pass.

## Reference executor

The reference result is what the shipped Cortex-M code does on the target. The executor works as follows.

- **Code under test.** It links the exact objects that were captured and replayed for a target and configuration into a small generated driver, built with the pinned `arm-none-eabi-gcc` and that configuration's flags. The driver comes from one shared, reviewed runtime template (startup, guard pages, signal handlers, input and result I/O); it is not per-target C.
- **Run.** The driver is a static ELF with its own `_start` and raw `svc` system calls. It runs under `qemu-arm -cpu cortex-m4` user mode with `-R 0x10000000` and `-seed 0`, through the tool's process runner, with no network. User mode is the right choice for this driver and not for firmware: an unmodified firmware image (vector table, reset handler, stack pointer setup) cannot run in `qemu-arm` user mode; the reference driver is a hosted-style ELF that reuses the same object code.
- **Environment.** The runner's fixed four-key environment (`LANG`, `PATH`, `SOURCE_DATE_EPOCH`, `TZ`) and no other variable; a fixed working directory that holds the request file; the input is passed through that file.
- **Budgets.** Wall-clock timeout 10000 ms. Reserved guest address space `0x10000000` (256 MiB), which is also the memory budget. Heap arena 1 MiB. Sentinel byte `0xA5`. Slack fill `0x5A`. Largest input 65536 bytes. A budget change is a contract change.
- **Determinism.** Every vector is run twice, and the two results must be byte-identical or the run fails. Nothing is retried. The provenance index records the image digest, the executor version and the budgets used.
- **Guard pages.** The input buffer is placed so its last byte sits directly before an unmapped guard page, with another guard before its first page. The output buffer is placed the same way beyond its capacity. An out-of-bounds access then faults and becomes an outcome instead of reading leftover memory. The limits: pages are 4096 bytes, so the end guard is byte-exact and the start guard is exact only when the length is a multiple of 4096; the bytes before a right-aligned buffer inside its first page are slack, prefilled with `0x5A` and checked after the call, and a changed byte is reported as `oob-write:guard-slack`. A buffer of length 0 is passed as the address of a guard page, and the stack lies in its own mapping with a guard page below it.
- **Differences from hardware, out of contract.** Address 0 is unmapped here and is readable flash on the target; the emulator has no NVIC, MPU, SysTick or fault status registers. An input whose C behavior depends on either is outside the contract, and the executor makes no statement about it. Phase 5 adds a replay on an emulated or real target.
- **Inputs.** In Phase 3 vectors come from the committed seed corpora only. Generated and mutated inputs belong to Phase 5.

## Out of scope

UB detection and the error code per function (Phase 4); differential fuzzing, generated inputs and a faster host ILP32 back end (Phase 5); the licence of the TRACTOR corpus (Phase 12; nothing from it is committed or redistributed, and it is fetched pinned into a working directory only to check that its vectors load).

## Changelog

- 0.1.0 (2026-10-21): contract v0 for Phase 3.
