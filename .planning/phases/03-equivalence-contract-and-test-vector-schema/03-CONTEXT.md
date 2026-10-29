# Phase 3: Equivalence Contract and Test-Vector Schema - Context

**Gathered:** 2026-10-21
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 3 makes "equivalent" written down and machine-checkable, and makes the original C runnable against any test vector to produce a deterministic reference result (EQV-01..EQV-04). It delivers four things:

- **Contract:** contract v0, as a versioned document plus a policy file the harness loads.
- **Schema:** the TRACTOR-shaped vector schema, with exactly `argv, stdin, env, stdout, stderr, rc, lib_state_in, lib_state_out, has_ub`, in Rust types.
- **Adapters:** per-target adapters that make at least 10 test-bed targets drivable.
- **Executor:** a C reference executor that turns a vector into an expected result, byte-identical on a re-run.

Not in this phase:

- UB detection itself (Phase 4).
- Differential fuzzing and generated inputs (Phase 5).
- Any Rust translation (Phase 8 onward).
- Redistributing any TRACTOR corpus content (licence check is Phase 12).

</domain>

<decisions>
## Implementation Decisions

Carried forward, not re-asked:

- **Refinement:** equivalence is refinement, with "return error" as the default UB action (PROJECT D-11).
- **Schema fields:** exactly the nine TRACTOR fields, including two lib_state fields (PROJECT constraint).
- **Licences:** the TRACTOR corpus licence is unchecked, so nothing from it is committed or redistributed (PROJECT legal constraint).
- **Process rules:** every subprocess goes through the `mt-toolchain` runner (D-16); deterministic outputs; tests never use the network.
- **Crate plan:** `docs/guidelines/building-the-tool.md` already names `mt-vectors` (schema) and `mt-contract` (contract, UB menu, policy).

### Reference build and executor

- **D-01: The reference result comes from the real Cortex-M code.** The executor links the exact objects Phase 2 captured and replayed for a target/config into a small generated driver, built with the pinned arm-none-eabi toolchain and that config's flags. It runs the driver under the pinned `qemu-arm` user mode. "What the C does" means what the shipped firmware code does on the target. Phase 5 may fuzz on a faster host build, but this executor stays the judge. — **Reversibility:** costly — Phase 5 replay, the evidence bundle and every stored vector assume this reference.
- **D-02: Fixed sandbox and budgets.** Each run goes through the `mt-toolchain` runner with:
  - no network;
  - an empty, fixed environment;
  - a fixed working directory;
  - a per-vector time budget and memory budget.

  A timeout or crash is recorded as the result, never retried, and never counted as a pass. Every vector is executed twice, and the two results must be byte-identical or the run fails.
- **D-03: A crash or fault in the C is recorded as the outcome, with `has_ub = true`.** The signal or fault goes into `rc`/`stderr` as the outcome class. A crashing input is out of contract, so the vector is excluded from output comparison. Phase 4's UB tools confirm or refine the label later.
- **D-04: Guard pages around buffers.** The input buffer is placed so its last byte sits immediately before an unmapped guard page, with another guard before its first byte. The output buffer gets the same treatment beyond its capacity. An out-of-bounds read or write then faults under QEMU and becomes a D-03 outcome, instead of reading leftover memory, which would break determinism.

### Mapping a library call onto TRACTOR's fields

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

### Library state contents

- **D-09: State structs are recorded with all their fields.** A struct passed by pointer (for example tinycbor's `CborParser` and `CborValue`) is recorded with every field its header defines, using the layout of the compiled C for ILP32 (from DWARF or clang's AST). The caller owns that memory, so all of it crosses the boundary. A field can be marked unobserved only with a recorded reason, and the contract counts such marks.
- **D-10: Every writable global is found automatically.** Every symbol in the target's own `.data` and `.bss`, taken from the captured objects' symbol tables, is snapshotted before and after the call. A global changing unexpectedly is caught without anyone declaring it.

### Adapters

- **D-11: Adapters are declared in the manifest, and the driver is generated.** The existing `[[target.X.entry]]` blocks in `testbed/manifest.toml` are extended with argument roles: input buffer, its length, output buffer and its capacity, flags/scalars, and state struct. `mt` generates the C driver from those roles. The manifest validator rejects bad role declarations, and no hand-written per-target C enters the repo. — **Reversibility:** costly — the manifest schema is shared with Phase 2's validator and later phases.
- **D-12: Entries that need prior state use a declared setup sequence.** Some entries need state before they can run: the six `form = "wrapped"` entries (struct-wrapped buffers per Phase 2 D-28, such as QCBOR `UsefulBufC`, nanopb `pb_istream_t` and zlib `z_stream`), and entries that need an earlier call (tinycbor `cbor_value_validate` needs `cbor_parser_init`). For these, the adapter declares the setup steps that run first, with their arguments drawn from the same vector. The state after setup is `lib_state_in`; the state after the entry is `lib_state_out`. A failed setup is recorded as its own outcome. The existing `accepted_wrapped` flag stays in force.
- **D-13: Adapters cover every target that fits, with a floor of 10 enforced by a test.** All 16 retained targets are attempted. A target whose entry the roles cannot express is logged with the reason, the same way Phase 2 logs drops.
- **D-14: Scalar arguments come from adapter defaults plus an optional short sweep.** Each scalar role has a declared default and an optional short list of values to sweep, for example parser flags 0 and the strict-mode flag. Each seed runs once per declared combination, with a capped total. Fuzzing the scalars is Phase 5's job.

### Contract and UB policy

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

### Vectors, storage and the TRACTOR corpus

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

### Sizing

- **D-24: The planner holds the phase to its size by splitting plans and putting the floor first.** The plans cover:
  - the contract and schema (including TRACTOR loading);
  - then the executor and adapters, with 10 drivable targets as the success line.

  The remaining targets are attempted within the same budget. Any not done are logged with their reason rather than growing the phase. No new roadmap phase is added.

### Founder decisions after research (2026-10-21)

The TRACTOR format details measured in 03-RESEARCH.md (pinned corpus commit `37960ee08a7c644ee66e29855eca33903f702e67`) refine D-05, D-03, D-13 and D-20.

- **D-25: Bytes go into TRACTOR's text fields as Latin-1 identity** (refines D-05).
  - The rule: byte b becomes character U+00b. It applies to `stdin`, a JSON string, and to `stdout.pattern`, where `stdout` is TRACTOR's `{pattern, is_regex}` object and we always write `is_regex: false`.
  - The mapping is written once in the contract.
  - Faults follow TRACTOR's convention `rc = -signal`. The outcome class (ok / fault / abort / timeout) is still recorded in `stderr` per D-05 and D-08.
  - **Reversibility:** costly. Every stored vector and the Phase 5 comparators decode with it.
- **D-26: "No UB observed" means the `has_ub` key is absent** (refines D-20). TRACTOR's checker treats any present `has_ub` key, even `false`, as UB. So a vector with no observed UB omits the key, and a test enforces that `has_ub: false` is never written. "UB unchecked" provenance stays in the sidecar index (D-19). Vectors carry at most the nine fields, with `has_ub` optional as in TRACTOR.
  - On load, TRACTOR's optional `note` key is accepted, and TRACTOR's string or integer `has_ub` labels are accepted unchanged.
  - Our own vectors never emit `note`.
  - When we do emit `has_ub`, its value is a label naming the source, for example `crash`.
- **D-27: `assert()` / `abort()` in the C is outcome class `abort`, with a `has_ub` crash label** (extends D-03). It is handled the same way as a fault: excluded from output comparison, and Phase 4 may relabel it.
- **D-28: An entry with no seeds is logged `no_seeds`** (refines D-13). Seeds are never borrowed from a sibling entry. This affects zcbor `cbor_decode_Pet`, `http_parser_parse_url` and `mbedtls_asn1_get_tag`. Phase 5's fuzzing supplies their inputs.
- **D-29: Smaller confirmations accepted as the researcher recommended:**
  - The policy file may carry an optional `default_change` record (same who, why, when and where fields as D-18), so that changing a module's default action is itself recorded (roadmap success criterion 1).
  - A shared, reviewed C runtime template (startup, guard pages, signal handler, I/O) used by every generated driver is not "per-target hand-written C" under D-11.
  - New optional manifest keys for roles do not bump the manifest `schema_version`.
  - No new crates (no `proptest`). Struct layout comes from the pinned GCC's DWARF read by `llvm-dwarfdump` through the runner (GCC's 1-byte enums differ from clang's record-layout dump), and ELF symbols are read with the already-locked `object` crate.
- **D-30: D-01 is confirmed as measured.** The driver is a Linux-ABI static ELF with its own `_start` and raw `svc` syscalls, run under `qemu-arm -cpu cortex-m4`.
  - Guard pages come from `mmap`/`mprotect`.
  - An in-guest signal handler exits with 128+signal.
  - `-R` fixes the guest layout and doubles as the memory budget.
  - Input is passed through a file in the fixed working directory, because `RunRequest` has no stdin.
  - QEMU 7.2 and GCC 14.3 in the pinned image must be re-measured in CI by an early smoke step before dependent work.

### Founder decision after plan 03-01's round trips (2026-10-26)

- **D-31: Re-pin QEMU in the image to a release that does not map the ARM commpage for M-profile CPUs (8.2.x, as measured on the host). D-30 and D-02 stay exactly as decided.**
  - **Cause:** the QEMU 7.2.22 source (stable-7.2, 30a6877) maps the commpage at 0xffff0000 for every ARM CPU. The calls are `linux-user/elfload.c` `init_guest_commpage` and `page_set_flags`, made before any guest code runs. M-profile caps `reserved_va` at 0x80000000 (`linux-user/arm/target_cpu.h`, `linux-user/main.c`).
  - **Consequence:** QEMU 7.2.22 cannot start any `-cpu cortex-m4` linux-user guest on a 64-bit host, whatever the flags. Testbed runs 39687629155 and 39688983966 failed with `page_set_flags: Assertion 'end - 1 <= GUEST_ADDR_MAX'`.
  - **Rejected alternatives:**
    - `-cpu cortex-a15` without `-R`: it weakens Cortex-M fidelity (D-01) and the per-vector memory budget (D-02).
    - A patched 7.2 build: more upkeep.
    - Stopping.
  - **Scope:** change the `qemu-user` (and `qemu-system-arm`) apt pins and the Dockerfile source to a pinned snapshot of a newer suite or backports. Rebuild and publish the image the way plans 02-01 and 02-02 did: pin discovery, a container round trip, then the founder's merge to `main` and a new digest under Current image. Then resume 03-01's round trip 3 (the founder's `extend 1`) on the new image.
  - **Pins comment:** the `container/pins.toml` comment "user mode cannot run bare-metal Cortex-M images" may be reworded in the same change.
  - **Reversibility:** costly. Every stored vector records the image digest (D-19).

### Founder decision at plan 03-08 (2026-10-29)

- **D-32: The Phase 3 vectors steps move to a second job of `testbed.yml`, `vectors`, with its own annotation budget.** The founder was asked how plan 03-08 should stay within GitHub's 50 annotations per job (the `testbed` job was at exactly 50 of 50 after plan 03-07, so 03-08 could not add a row to a listing or a notice) and answered, quoted: "Second job (Recommended)".
  - **Meaning:** the executor self-test, the TRACTOR sample, the reference driver build, the layouts, the twice-run vectors, the rerun comparison and plan 03-08's sample steps run in the job `vectors`; the existing `testbed` job, its eight Phase 2 steps and their `testbed-` notices stay byte-identical (a static test pins the sha256 of each step's text, and the comparison with `02-22-CI-NOTICES.md` from line 5 on must still pass), and the Phase 2 fixture notices stay in that job unchanged.
  - **Constraints held:** the second job runs in the same pinned image by digest (it pulls it itself and publishes `vectors-image`), with the same hardening (no network for anything that runs captured or upstream code; `fetch_tractor` stays the one networked vectors helper) and the same helpers (`image_tool`, `run_vectors`, `run_guest`, `fetch_tractor`, `finish_vectors`). The verified captures, the builds and the replay reports reach it as the artifact `vectors-input` (two tar files made in the image, so the container user's files are read by their owner), made by the existing `actions/upload-artifact` and read with the existing `actions/download-artifact`, both already pinned by commit SHA; no new action. It fetches and rebuilds nothing.
  - **Reading notices:** from now on the `vectors-*` notices are read from the `vectors` job's id (`gh api repos/.../actions/runs/<run>/jobs`), and the `testbed-*` notices from the `testbed` job's id; every round trip records both job ids.
  - **Reversibility:** reversible. Moving the steps back is a workflow edit, but the annotation cap would return.

### Claude's Discretion

- **Executor internals:** how the driver is generated and linked (startup code, newlib with semihosting or a minimal stub). The constraints are the pinned toolchain, the shipped `.specs` names (02-21 `SHIPPED_SPECS`) and determinism.
- **Fixed values:** the exact budget values, the sentinel byte value and the guard-page mechanism under qemu-arm user mode.
- **CLI shape:** for example `mt vectors validate|run|check`.
- **Layout source:** how struct layout is obtained (DWARF from a `-g` build of the same objects versus clang's AST), provided it is exact for the target's ILP32 layout and matches the captured objects.
- **Crate split:** how `mt-vectors`, `mt-contract` and the executor split across crates, following `docs/guidelines/building-the-tool.md`.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project rules and decisions
- `.claude/CLAUDE.md` — tool-code rules (forbid unsafe, runner only, thiserror/anyhow, determinism, requirement-ID test names), emitted-Rust rules, report wording.
- `.planning/PROJECT.md` — D-11 (refinement, return-error default), D-13 (no certified/compliant claims), the schema and legal constraints.
- `.planning/REQUIREMENTS.md` — EQV-01..EQV-04 wording.
- `.planning/ROADMAP.md` § Phase 3 — the four success criteria.

### Contract and vector format sources
- `docs/specs/product-spec.md` — "Technical approach" items 1 (undefined behavior), 2 (evidence strong enough to trust) and 3 (idiomatic versus provable: the observation point at the C interface, abstraction functions); customer requirement 4 (proof it behaves the same); and the "first 90 days" plan, which reuses TRACTOR's JSON vector format.
- `docs/research/VERIFICATION.md` — TRACTOR facts: behavioral fidelity to the compiled C binary, `has_ub` excluded from scoring, the corrected two-field lib_state list, and the corpus location and unchecked licence.
- `docs/specs/emitted-rust-rules.md` — partial writes on error paths, the UB decisions log, `panic = "abort"`.
- `docs/research/literature-map.md` — the TRACTOR benchmark entry (arXiv 2609.25121).

### Tool architecture
- `docs/adr/0001-tool-implementation-language.md` — Rust workspace, analysers as pinned subprocesses.
- `docs/guidelines/building-the-tool.md` — the planned `mt-vectors` and `mt-contract` crates; no copying from the TRACTOR corpus until its licence is confirmed.

### Inputs from Phase 2
- `testbed/manifest.toml` — the `[[target.X.entry]]` blocks (name, header, source, form, signature, `accepted_wrapped`) that D-11 extends.
- `.planning/phases/02-test-bed-and-build-capture/02-CONTEXT.md` — D-24/D-28 (entry forms, struct-wrapped buffers) and the drop-log convention reused by D-13.
- `.planning/phases/02-test-bed-and-build-capture/02-22-SUMMARY.md` and `02-21-SUMMARY.md` — the current capture/replay and gate state, and `SHIPPED_SPECS`.
- `container/pins.toml` — the pinned `qemu-user` and arm-none-eabi toolchain.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`crates/mt-toolchain/src/runner.rs`:** the only process runner. It records argv, the env allowlist, versions, exit status and output hashes, so the executor runs QEMU through it.
- **`crates/mt-capture`:** the manifest validator (extend it for D-11 roles); capture and replay of per-pair objects (the executor links those objects); the fail-closed compare; the drop log and 10-target floor (`check_floor`), reusable for D-13; the `copy_tree` and never-follow-symlinks reader.
- **`crates/mt-seeds`:** sha256-named corpora with a versioned `index.json` and drift checker, the pattern for D-19/D-21 vector storage. The 16 committed seed corpora are the D-22 inputs.
- **The entry check (clang probe of entry signatures, Phase 2):** the signatures it confirms are the starting point for argument roles.

### Established Patterns
- Strict TOML with unknown keys rejected; `schema_version` in every machine file; `BTreeMap` and explicit seeds for deterministic output.
- **CI evidence:** offline work locally; image-dependent proof in the `testbed` workflow under `--network none`; notices compared line for line across runs.
- **Fetching:** commit plus tree sha256, with a re-verified cache. Reuse this for the D-23 TRACTOR corpus fetch, which is the only networked step.

### Integration Points
- A new `mt testbed`/`mt vectors` subcommand in `crates/mt-cli`.
- New `testbed.yml` steps: build drivers, run the executor twice and compare, and the TRACTOR load check.
- The Phase 4 UB log consumes the D-16 site identifiers and the D-20 unchecked labels.
- Phase 5 differential runs consume D-05..D-08.

</code_context>

<specifics>
## Specific Ideas

- **Correction recorded during discussion:** in the manifest, `form = "wrapped"` means a struct-wrapped buffer (Phase 2 D-28), not "needs a prior call". D-12's declared setup sequence covers both cases: building the wrapper struct, and calling a prior entry such as `cbor_parser_init` before `cbor_value_validate`.
- **Entry forms today:** the manifest has 20 `direct`, 6 `wrapped` and 1 `range` entry across the 16 retained targets.
- **Report wording:** reports must say "UB unchecked" for D-20 vectors, and never use the words "certified" or "compliant".

</specifics>

<deferred>
## Deferred Ideas

- **Mutation and fuzz generation** of vector inputs and scalar arguments: Phase 5.
- **A host ILP32 build** as a fast differential back end, compared against this phase's QEMU reference: Phase 5.
- **Per-site UB overrides with real entries,** which need the Phase 4 UB log.
- **The TRACTOR corpus licence check,** before any redistribution: Phase 12.

</deferred>

---

*Phase: 03-equivalence-contract-and-test-vector-schema*
*Context gathered: 2026-10-21*
