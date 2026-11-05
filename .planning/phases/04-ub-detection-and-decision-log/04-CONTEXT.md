# Phase 4: UB Detection and Decision Log - Context

**Gathered:** 2026-11-05
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 4 finds every input and code site where the original C has undefined or implementation-defined behavior. It labels each one in the vector corpus and records a signed decision for it (UB-01..UB-05). It delivers:

- **Sanitizer variants:** separate ASan+UBSan and MSan builds of each retained pair, plus a trap-mode UBSan build of the real target. The stored corpus is replayed against each, and `has_ub` is set from what they report.
- **Optimiser differential:** every vector is replayed against the target C at `-O0` and at the shipped flags. A disagreement is labelled suspected UB.
- **Symbolic scan:** a C-side KLEE scan of the parser entry functions, covering at least 3 test-bed targets. It finds UB sites the corpus never reached, with per-function completion or timeout.
- **Implementation-defined facts:** char signedness, integer and pointer sizes, and the layout of every struct crossing the interface. These are extracted from the target configuration and signed off.
- **UB decisions log and gate:** each site or input class gets a decision, a rationale, an approver and a date. `mt ub check` refuses UB-complete while anything is unsigned.

Not in this phase:

- Differential fuzzing, generated or mutated inputs, and the host ILP32 fast back end for Rust comparison (Phase 5). Phase 5 re-uses this phase's UB pass on its larger corpus.
- The Rust-to-KLEE spike R1 and the Kani spike R2 (Phase 6). This phase runs KLEE on C bitcode only.
- Planted-bug and mutation proof of the harness (Phase 7). This phase's known-UB fixtures are its own self-test, not the 20 planted bugs.
- A UB manager or review UI (milestone v2.0).

</domain>

<decisions>
## Implementation Decisions

Carried forward, not re-asked:

- **Equivalence and default:** equivalence is refinement, and the default UB action is `return_error` (PROJECT D-11). A site is identified by `function`, `kind` and `location`, matched exactly. Every override is a full record of site, action, reason, approver and date (03 D-16, D-18).
- **`has_ub` semantics:** the key is absent when no UB was observed and is never `false`. When present it is a label naming its source. Provenance lives in the sidecar `index.json` (03 D-19, D-20, D-26).
- **Reference:** the QEMU `-cpu cortex-m4` executor on the captured objects stays the judge of what the C does (03 D-01, D-30). Its sandbox and budgets apply to every target-side run (03 D-02).
- **Separate builds:** ASan+UBSan and MSan are built as separate variants (PROJECT D-07, R6).
- **Process rules:** every subprocess goes through the `mt-toolchain` runner (D-16). Outputs are deterministic. Tests never use the network. Test names carry the requirement IDs (`ub_01_...`). A timeout is "not proven", never a pass. Reports never use the words "certified" or "compliant".
- **Approver:** on the test bed, the founder signs as proxy approver (PROJECT "MVP Definition of Full Evidence").

### Sanitizer variants (UB-01)

- **D-01: ASan+UBSan runs on the host as i386 ILP32.** It is built with host `clang-16 -m32 -funsigned-char`, the shipped config's defines and include paths, and the captured sources. This gives the target's int, long and pointer sizes and its char signedness (R5). The image gains the sanitizer runtime and the i386 libc pins it needs (see D-27). — **Reversibility:** costly — every `asan:`/`ubsan:` label and its provenance assume this build.
- **D-02: UBSan check set.** `-fsanitize=undefined` without `unsigned-integer-overflow`, plus `-fno-sanitize-recover=all`, so the first finding ends the run. Unsigned wrap is defined C and never sets `has_ub`.
- **D-03: MSan uses an LP64 fallback if i386 is unsupported.** The researcher confirms or refutes A-08. If MSan has no i386 support, it runs x86-64 with `-funsigned-char`. The contract documents the fallback, and each MSan finding records that it came from an LP64 host build. This settles A-08 either way.
- **D-04: Trap-mode UBSan also runs on the real target.** A third variant is the exact arm-none-eabi GCC and shipped flags plus `-fsanitize=undefined -fsanitize-undefined-trap-on-error`, with no runtime. It runs through the existing QEMU executor. A trap carries no kind, so its label is `ubsan-trap`, and the host build supplies the kind where it fires too. This variant is beyond UB-01 and is first on the cut list (D-26).
- **D-05: Host driver.** The host variants use a driver generated from the same manifest roles (03 D-11), with a second shared, reviewed host runtime template (libc, `mmap` guard pages, the same vector file input). This runtime is not per-target hand-written C (03 D-29). Sanitizer reports on stderr are parsed into labels and sites.
- **D-06: A pair that does not build for the host is logged and the run continues.** Examples are arm intrinsics or target-only headers. The pair gets `sanitizer: unavailable` with a cause token, in the same drop-log pattern as Phases 2 and 3. It still gets trap UBSan on target (D-04). It cannot be UB-complete until a founder decision about the missing variant is recorded in its log.
- **D-07: Every stored vector of every retained pair is replayed.** The corpus is small today (seed-derived, 03 D-22). Phase 5 re-uses the same pass with its own budget.
- **D-08: Known-UB samples are small in-repo C fixtures.** They cover signed overflow, out-of-bounds read, uninitialised read, oversized shift, and one optimiser-sensitive case. Each has a triggering input and a clean input, and each is driven through the same manifest-role adapter path. They are the harness's own test data, so 03 D-11 does not apply to them. They are a permanent CI self-test, like the executor self-test: the `ub` job fails if any triggering input is not flagged with its expected `source:kind`, or if any clean input is flagged.

### has_ub labels and the optimiser differential (UB-02)

- **D-09: Findings are written into the vectors, and the vectors are regenerated.** Vectors are build outputs (03 D-21). The UB pass writes the label into `has_ub`, re-hashes the vector, and updates `index.json`. The provenance gains the variants that ran, those that were unavailable, and each result. The committed sample is regenerated through the existing fail-closed `mt vectors sample` path. TRACTOR tools then see UB vectors as UB. — **Reversibility:** costly — vector sha256 names and the committed sample change.
- **D-10: The label grammar is `source:kind`.** Examples: `asan:heap-buffer-overflow`, `ubsan:signed-integer-overflow`, `msan:use-of-uninitialized-value`, `ubsan-trap`, `optdiff`, `klee:<kind>`, and the existing `crash` and `timeout`. When several sources fire, the vector carries the first in a fixed, documented source order, and the sidecar lists all of them. Function, kind and location go in the UB log, not the label. The contract (`docs/specs/equivalence-contract.md`) gains the grammar, with a version bump and a changelog line. — **Reversibility:** costly — stored vectors and Phase 5 comparators read it.
- **D-11: A Phase 3 `crash` is refined when a sanitizer explains it.** It becomes, for example, `asan:stack-buffer-overflow`, and the sidecar keeps the original QEMU outcome. A crash no sanitizer explains keeps `crash` and gets a log entry.
- **D-12: The optimiser differential runs on the target under QEMU.** The captured pair is rebuilt with the same arm-none-eabi GCC and shipped flags, changing only the `-O` level to `-O0`, and run through the Phase 3 executor. The fields compared are those of 03 D-08: `rc`, `stdout`, `lib_state_out` and the outcome class.
- **D-13: A disagreement sets `has_ub = "optdiff"` and needs a decision.** The vector is excluded from comparison because the C gives no single answer. A UB-log entry is created. Its decision may be "not UB, compare against shipped flags", which re-includes the vector and records why.
- **D-14: A clean vector has no `has_ub` key.** 03 D-26 is unchanged. `index.json` moves from "unchecked" to `ub_checked` and lists the variants that ran and those that were unavailable. Reports say "no UB observed by <variants>", never "no UB".
- **D-15: Host-only findings count.** A finding from host `-m32` or LP64 MSan that is not seen on target is still UB in the C source, so it sets `has_ub`. The label and the log record the variant, and MSan findings carry the LP64 note. A finding can be dismissed as a host artifact only through a signed log decision with a reason.
- **D-16: Timeouts get log entries.** Kind `timeout`, and each needs a decision. The label stays `timeout` (rc -9, 03 UAT), and a timeout is never a pass.

### Symbolic scan (UB-03)

- **D-17: The scan uses KLEE only.** KLEE 3.2 on LLVM 16 is already pinned and checked. Frama-C is not added. Signed-overflow checks come from `-fsanitize=signed-integer-overflow` instrumentation in the bitcode, alongside KLEE's built-in checks.
- **D-18: KLEE's bitcode ABI is measured first.** The researcher checks whether KLEE 3.2 with klee-uclibc runs i386 bitcode with `-funsigned-char`. If it does, use it. If not, use x86-64 with `-funsigned-char` and record the ABI in each finding, as with D-03.
- **D-19: Symbolic inputs.** The input buffer is symbolic, with a symbolic length up to a fixed bound N. Scalars take their adapter-declared sweep values (03 D-14), not free values. Setup steps (03 D-12) run concretely. N and the budget are recorded with every result.
- **D-20: The KLEE budget is deterministic.** The primary budget is `--max-instructions` per function, with a fixed search strategy and a fixed seed. A generous wall-clock cap is only a safety net, and hitting it is recorded as `timeout` ("not proven"). KLEE results are run twice and compared, like the vectors.
- **D-21: Every drivable entry is attempted, with a floor of 3 targets.** Entries KLEE cannot handle are logged with a cause, in the drop pattern of 03 D-13. A test enforces at least 3 targets with completed or timed-out per-function results.
- **D-22: KLEE output.** Each error path becomes a UB-log entry with provenance `klee`, the function, kind and location from KLEE's report, and the per-function completion or timeout. KLEE's concrete test case is also run through the QEMU executor and the variants, then stored as a labelled witness vector.
- **D-23: An unconfirmed KLEE finding stays in the log.** If replay on target and in the variants shows no UB, the vector is stored with what the replays observed. The log entry stays with status `klee-unconfirmed` and still needs a signed decision. It can be dismissed only with a reason.
- **D-24: WR-13 is closed first.** The first plan of Phase 4 closes runner residual WR-13: after a normal exit, find the processes holding the output pipes via `/proc/*/fd` and kill them with a pidfd. It comes with a test, before any KLEE run goes through the runner. This clears the STATE.md blocker.

### CI

- **D-25: A new `ub` job in `testbed.yml`.** It runs in the same pinned image by digest, with the same hardening (no network for anything running captured or upstream code). The vectors arrive as an artifact through the already-pinned upload and download actions. It runs the fixtures self-test, the sanitizer variants, the `-O0` diff, KLEE, the relabelled corpus and `mt ub check`, with its own 50-annotation budget. The `testbed` and `vectors` jobs stay byte-identical, following the 03 D-32 pattern.

### Sizing

- **D-26: Floor first, with a cut list, and no new phase.** The planner orders plans as follows:
  1. WR-13 (D-24) and the image plan (D-27).
  2. The log, the facts, `mt ub check` and the known-UB fixtures.
  3. Host ASan+UBSan.
  4. MSan.
  5. The `-O0` differential.
  6. KLEE, up to its floor of 3.

  If the phase runs long, these are cut first, in order:
  1. Trap UBSan on target (D-04).
  2. Attempting KLEE on every drivable entry beyond the floor of 3 (D-21).

  Cuts are logged with a reason in the phase summary.

### Image

- **D-27: An early image plan, merged by the founder.** It follows the route of 02-01/02-02 and 03-10. Pin discovery covers the sanitizer runtime for clang-16 and the i386 libc and dev packages, plus whatever MSan needs. Then a container round trip, the founder's merge to `main` (only main publishes, WR-09), a new Current image digest, and a new PROJECT.md D-line. Offline work (log, fixtures, labels) proceeds meanwhile. Dependent CI work waits for the digest.

### UB decisions log and gate (UB-04, UB-05)

- **D-28: The proposed action is not a decision.** The tool writes each new entry with `action = "return_error"` proposed, and with rationale, approver and date empty. An entry counts as decided only when all four are filled. — **Reversibility:** costly — every signed log and the UB-complete predicate depend on it.
- **D-29: The log is a separate file, and the policy is checked against it.** The log is `testbed/ub-log/<target>.toml`: strict TOML, unknown keys rejected, with `schema_version`. It has one file per target covering all configs. It holds every entry and decision. A check enforces two things. First, the policy file (`testbed/policy`, 03 D-15/D-16) has a `[[ub.override]]` exactly for each decided entry whose action is not `return_error`. Second, it has an `[[ub.error_code]]` for each function that needs one. The known-site list passed to the policy loader comes from this log. — **Reversibility:** costly — the file format is shared with Phases 5, 11 and the evidence bundle.
- **D-30: Re-runs merge by site key and never delete entries.** Decided entries keep their decision. New sites are appended undecided. A site that is no longer observed is marked `stale` with the run id and keeps its decision; only a human removes it. Witness lists are updated, and the output order is sorted and deterministic.
- **D-31: Input classes use the site grammar.** Entries with no code site (`optdiff`, unexplained `crash`, `timeout`) are keyed as function = the entry name, kind = the label, and location = `input-class:<n>`, where n is the smallest witness vector sha256. The policy can override them like any site, and later witnesses with the same entry and label join that entry.
- **D-32: The site key for a dynamic finding is the innermost frame in the target's own code.** The location is the source `file:line:col` of the innermost stack frame in the target's own sources (not libc, not the driver), together with the sanitizer kind. Every vector hitting that site is listed as a witness.
- **D-33: Implementation-defined facts are a section of the same log.** There is a `[target_facts]` block per config with `char` signedness, the sizes of `short`/`int`/`long`/`long long`/pointer/`size_t`, and the layout of every struct crossing the interface. The values are read from the pinned arm-none-eabi GCC for the shipped flags (predefined macros such as `__CHAR_UNSIGNED__` and `__SIZEOF_POINTER__`, plus DWARF layouts as in 03 D-29). The block carries the same approver and date sign-off. UB-complete requires the facts to be signed and to equal a fresh extraction; a changed fact unsigns the block.
- **D-34: Error codes are proposed by the tool and signed by a human.** For each function that needs a `return_error` code, the tool proposes one from the C's own observed error returns in the corpus (the declared error enum where there is one, otherwise the most common non-success `rc`). The proposal is written beside the log entry, and it becomes binding, filling `[[ub.error_code]]`, only when the entry is signed. Nothing is invented silently.
- **D-35: Bulk acceptance of the default is allowed, with a record per entry.** `mt ub sign --accept-default --approver <name> [<target>]` writes approver, date and a fixed rationale ("default return_error accepted") into each undecided entry separately. Fact blocks and non-default actions are never bulk-signed.
- **D-36: The gate is `mt ub check [<target>]`.** It exits non-zero and lists each undecided entry, unsigned or changed fact block, and policy/log mismatch. A pass prints `ub-complete` with the variants that ran and those that were unavailable. In CI it runs as a notice, not a failure, because test-bed entries stay undecided until the founder signs them.

### Claude's Discretion

- Exact values: the symbolic bound N, the KLEE instruction budget and wall cap, and per-variant time and memory budgets.
- The fixed source order for choosing a vector's single `has_ub` label (D-10). It must be documented in the contract.
- The exact kind tokens parsed from ASan, MSan, UBSan and KLEE reports, and the cause tokens for unavailable variants and KLEE drops.
- The crate split, for example a new `mt-ub` crate or extending `mt-harness` and `mt-contract`, following `docs/guidelines/building-the-tool.md`.
- CLI shape beyond `mt ub check` and `mt ub sign`, for example `mt ub run` and `mt ub facts`.
- How the host driver template shares code with the Phase 3 runtime template.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project rules and decisions
- `.claude/CLAUDE.md` — tool-code rules (forbid unsafe, runner only, thiserror/anyhow, determinism, requirement-ID test names), report wording.
- `.planning/PROJECT.md` — D-07 (separate sanitizer builds), D-11 (refinement, return-error default), D-13, D-15 (LLVM 16 pin), D-16 (runner only), A-08 (MSan i386 assumption to settle), the "MVP Definition of Full Evidence" term 4.
- `.planning/REQUIREMENTS.md` — UB-01..UB-05 wording.
- `.planning/ROADMAP.md` § Phase 4 — the five success criteria.
- `.planning/STATE.md` — the WR-13 runner blocker (D-24) and the A-08 concern.

### Contract, vectors and policy (Phase 3 outputs this phase extends)
- `docs/specs/equivalence-contract.md` — `has_ub` rules, the outcome table, the policy file keys (`[ub] default_action`, `[[ub.error_code]]`, `[[ub.override]]`); gains the label grammar (D-10) and the MSan/KLEE ABI notes (D-03, D-18).
- `.planning/phases/03-equivalence-contract-and-test-vector-schema/03-CONTEXT.md` — D-01/D-02 executor, D-11/D-12/D-14 adapter roles and setup, D-16..D-18 UB policy, D-19..D-21 sidecar index and sample, D-26/D-27 labels, D-29 runtime template and DWARF layouts, D-32 job split.
- `testbed/policy/` — the 16 per-module policies that D-29 checks against the log.
- `testbed/manifest.toml` — entry roles that the host driver and KLEE harness re-use.

### Background and risks
- `docs/research/VERIFICATION.md` — R1 (LLVM lock, KLEE 3.2 on LLVM 16), R5 (ILP32, char signedness), R6 (MSan cannot share a build with ASan).
- `docs/specs/product-spec.md` — Technical approach item 1 (undefined behavior) and the UB policy manager requirement (the UI part is deferred).
- `docs/specs/emitted-rust-rules.md` — the UB decisions log as the emitted Rust consumes it.
- `docs/guidelines/building-the-tool.md` — crate plan and the runner rule.

### Toolchain and image
- `container/pins.toml` — clang-16, KLEE 3.2 (`[source.klee]`, `[tool.klee]`), arm-none-eabi GCC, QEMU 10.0.2; no sanitizer runtime or i386 pins yet (D-27).
- `crates/mt-toolchain/src/runner.rs` — the runner WR-13 is fixed in.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`crates/mt-harness`:** the QEMU executor (`exec.rs`, `run.rs`), driver generation (`driver.rs`, `runtime.c`, `link.rs`), the outcome classifier (`classify.rs`), DWARF layouts (`layout.rs`), globals and heap capture, and the self-test (`selftest.rs`). The target trap-UBSan and `-O0` variants re-use the executor unchanged. The fixtures self-test follows `selftest.rs`. The facts extraction re-uses `layout.rs`.
- **`crates/mt-contract/src/policy.rs`:** the strict policy loader, which already takes a known-site list (empty until now). D-29 feeds it from the log.
- **`crates/mt-vectors`:** the vector types, `corpus.rs` index and sidecar, and `compare.rs`. D-09 relabelling and re-hashing go through these.
- **`crates/mt-capture`:** captured objects and compile commands per pair (the source of the shipped flags for host and `-O0` rebuilds), the drop log and floor checks (D-06, D-21), and the compile-command allow-list gate (02-21).
- **`crates/mt-toolchain/src/runner.rs`:** the only process runner. WR-13 is fixed here first.

### Established Patterns
- Strict TOML with `deny_unknown_fields` and `schema_version`; `BTreeMap` ordering; sha256-named corpora with an `index.json`.
- Drop logs with cause tokens and a floor enforced by a test (Phase 2 `check_floor`, 03 D-13 `ADAPTER_FLOOR`).
- Image changes go through pin discovery, a container round trip, the founder's merge to main, and a new digest (02-01/02, 03-10).
- CI notices are compared line for line across runs. Each job has a 50-annotation budget, so new work gets its own job (03 D-32).

### Integration Points
- A new `mt ub ...` subcommand family in `crates/mt-cli`.
- A new `ub` job in `.github/workflows/testbed.yml`, fed by the vectors artifact.
- The Phase 5 differential harness consumes the labels (D-10, D-13) and `ub_checked` provenance (D-14).
- The Phase 11 evidence bundle and the full-evidence predicate term 4 consume the log and `mt ub check`.

</code_context>

<specifics>
## Specific Ideas

- The founder chose the recommended option on every question. The rationale recorded in the options is the intent: fidelity to the target ABI first, every finding traceable to a signed human decision, and nothing dismissed silently.
- UB-05's "pre-filled with return error" means the proposed action is written for the human. It does not mean the entry is decided (D-28).
- Report wording: "no UB observed by <variants>", "UB unchecked" and "not proven". Never "no UB", "certified" or "compliant".

</specifics>

<deferred>
## Deferred Ideas

- A UB manager or review UI for signing log entries (milestone v2.0, spec-v1).
- Frama-C Eva as a second static scanner (not planned; reconsider if KLEE coverage is thin).
- Recording unsigned-wrap hits as informational notes (rejected for now; may help Phase 7 or later intent checks).
- Corpus-wide re-runs of the UB pass on fuzz-generated inputs (Phase 5).

</deferred>

---

*Phase: 04-ub-detection-and-decision-log*
*Context gathered: 2026-11-05*
