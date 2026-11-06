# Phase 4: UB Detection and Decision Log - Research

**Researched:** 2026-11-05 to 2026-11-06 (resumed after a usage-limit pause)
**Domain:** C undefined-behavior detection (compiler-rt sanitizers, KLEE), optimiser differential under QEMU, implementation-defined facts from GCC, a signed TOML decisions log, and the runner's WR-13 residual
**Confidence:** MEDIUM overall. Compiler-rt, KLEE and in-repo facts are HIGH, read from pinned-tag sources or measured. The Debian bookworm package names are MEDIUM: the evidence is an Ubuntu noble proxy. Runtime behaviour inside the pinned image has not been measured, because no image was available in this sandbox.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

Carried forward, not re-asked:

- **Equivalence and default:** equivalence is refinement, and the default UB action is `return_error` (PROJECT D-11). A site is identified by `function`, `kind` and `location`, matched exactly. Every override is a full record of site, action, reason, approver and date (03 D-16, D-18).
- **`has_ub` semantics:** the key is absent when no UB was observed and is never `false`. When present it is a label naming its source. Provenance lives in the sidecar `index.json` (03 D-19, D-20, D-26).
- **Reference:** the QEMU `-cpu cortex-m4` executor on the captured objects stays the judge of what the C does (03 D-01, D-30). Its sandbox and budgets apply to every target-side run (03 D-02).
- **Separate builds:** ASan+UBSan and MSan are built as separate variants (PROJECT D-07, R6).
- **Process rules:** every subprocess goes through the `mt-toolchain` runner (D-16). Outputs are deterministic. Tests never use the network. Test names carry the requirement IDs (`ub_01_...`). A timeout is "not proven", never a pass. Reports never use the words "certified" or "compliant".
- **Approver:** on the test bed, the founder signs as proxy approver (PROJECT "MVP Definition of Full Evidence").

#### Sanitizer variants (UB-01)

- **D-01: ASan+UBSan runs on the host as i386 ILP32.** It is built with host `clang-16 -m32 -funsigned-char`, the shipped config's defines and include paths, and the captured sources. This gives the target's int, long and pointer sizes and its char signedness (R5). The image gains the sanitizer runtime and the i386 libc pins it needs (see D-27). — **Reversibility:** costly — every `asan:`/`ubsan:` label and its provenance assume this build.
- **D-02: UBSan check set.** `-fsanitize=undefined` without `unsigned-integer-overflow`, plus `-fno-sanitize-recover=all`, so the first finding ends the run. Unsigned wrap is defined C and never sets `has_ub`.
- **D-03: MSan uses an LP64 fallback if i386 is unsupported.** The researcher confirms or refutes A-08. If MSan has no i386 support, it runs x86-64 with `-funsigned-char`. The contract documents the fallback, and each MSan finding records that it came from an LP64 host build. This settles A-08 either way.
- **D-04: Trap-mode UBSan also runs on the real target.** A third variant is the exact arm-none-eabi GCC and shipped flags plus `-fsanitize=undefined -fsanitize-undefined-trap-on-error`, with no runtime. It runs through the existing QEMU executor. A trap carries no kind, so its label is `ubsan-trap`, and the host build supplies the kind where it fires too. This variant is beyond UB-01 and is first on the cut list (D-26).
- **D-05: Host driver.** The host variants use a driver generated from the same manifest roles (03 D-11), with a second shared, reviewed host runtime template (libc, `mmap` guard pages, the same vector file input). This runtime is not per-target hand-written C (03 D-29). Sanitizer reports on stderr are parsed into labels and sites.
- **D-06: A pair that does not build for the host is logged and the run continues.** Examples are arm intrinsics or target-only headers. The pair gets `sanitizer: unavailable` with a cause token, in the same drop-log pattern as Phases 2 and 3. It still gets trap UBSan on target (D-04). It cannot be UB-complete until a founder decision about the missing variant is recorded in its log.
- **D-07: Every stored vector of every retained pair is replayed.** The corpus is small today (seed-derived, 03 D-22). Phase 5 re-uses the same pass with its own budget.
- **D-08: Known-UB samples are small in-repo C fixtures.** They cover signed overflow, out-of-bounds read, uninitialised read, oversized shift, and one optimiser-sensitive case. Each has a triggering input and a clean input, and each is driven through the same manifest-role adapter path. They are the harness's own test data, so 03 D-11 does not apply to them. They are a permanent CI self-test, like the executor self-test: the `ub` job fails if any triggering input is not flagged with its expected `source:kind`, or if any clean input is flagged.

#### has_ub labels and the optimiser differential (UB-02)

- **D-09: Findings are written into the vectors, and the vectors are regenerated.** Vectors are build outputs (03 D-21). The UB pass writes the label into `has_ub`, re-hashes the vector, and updates `index.json`. The provenance gains the variants that ran, those that were unavailable, and each result. The committed sample is regenerated through the existing fail-closed `mt vectors sample` path. TRACTOR tools then see UB vectors as UB. — **Reversibility:** costly — vector sha256 names and the committed sample change.
- **D-10: The label grammar is `source:kind`.** Examples: `asan:heap-buffer-overflow`, `ubsan:signed-integer-overflow`, `msan:use-of-uninitialized-value`, `ubsan-trap`, `optdiff`, `klee:<kind>`, and the existing `crash` and `timeout`. When several sources fire, the vector carries the first in a fixed, documented source order, and the sidecar lists all of them. Function, kind and location go in the UB log, not the label. The contract (`docs/specs/equivalence-contract.md`) gains the grammar, with a version bump and a changelog line. — **Reversibility:** costly — stored vectors and Phase 5 comparators read it.
- **D-11: A Phase 3 `crash` is refined when a sanitizer explains it.** It becomes, for example, `asan:stack-buffer-overflow`, and the sidecar keeps the original QEMU outcome. A crash no sanitizer explains keeps `crash` and gets a log entry.
- **D-12: The optimiser differential runs on the target under QEMU.** The captured pair is rebuilt with the same arm-none-eabi GCC and shipped flags, changing only the `-O` level to `-O0`, and run through the Phase 3 executor. The fields compared are those of 03 D-08: `rc`, `stdout`, `lib_state_out` and the outcome class.
- **D-13: A disagreement sets `has_ub = "optdiff"` and needs a decision.** The vector is excluded from comparison because the C gives no single answer. A UB-log entry is created. Its decision may be "not UB, compare against shipped flags", which re-includes the vector and records why.
- **D-14: A clean vector has no `has_ub` key.** 03 D-26 is unchanged. `index.json` moves from "unchecked" to `ub_checked` and lists the variants that ran and those that were unavailable. Reports say "no UB observed by <variants>", never "no UB".
- **D-15: Host-only findings count.** A finding from host `-m32` or LP64 MSan that is not seen on target is still UB in the C source, so it sets `has_ub`. The label and the log record the variant, and MSan findings carry the LP64 note. A finding can be dismissed as a host artifact only through a signed log decision with a reason.
- **D-16: Timeouts get log entries.** Kind `timeout`, and each needs a decision. The label stays `timeout` (rc -9, 03 UAT), and a timeout is never a pass.

#### Symbolic scan (UB-03)

- **D-17: The scan uses KLEE only.** KLEE 3.2 on LLVM 16 is already pinned and checked. Frama-C is not added. Signed-overflow checks come from `-fsanitize=signed-integer-overflow` instrumentation in the bitcode, alongside KLEE's built-in checks.
- **D-18: KLEE's bitcode ABI is measured first.** The researcher checks whether KLEE 3.2 with klee-uclibc runs i386 bitcode with `-funsigned-char`. If it does, use it. If not, use x86-64 with `-funsigned-char` and record the ABI in each finding, as with D-03.
- **D-19: Symbolic inputs.** The input buffer is symbolic, with a symbolic length up to a fixed bound N. Scalars take their adapter-declared sweep values (03 D-14), not free values. Setup steps (03 D-12) run concretely. N and the budget are recorded with every result.
- **D-20: The KLEE budget is deterministic.** The primary budget is `--max-instructions` per function, with a fixed search strategy and a fixed seed. A generous wall-clock cap is only a safety net, and hitting it is recorded as `timeout` ("not proven"). KLEE results are run twice and compared, like the vectors.
- **D-21: Every drivable entry is attempted, with a floor of 3 targets.** Entries KLEE cannot handle are logged with a cause, in the drop pattern of 03 D-13. A test enforces at least 3 targets with completed or timed-out per-function results.
- **D-22: KLEE output.** Each error path becomes a UB-log entry with provenance `klee`, the function, kind and location from KLEE's report, and the per-function completion or timeout. KLEE's concrete test case is also run through the QEMU executor and the variants, then stored as a labelled witness vector.
- **D-23: An unconfirmed KLEE finding stays in the log.** If replay on target and in the variants shows no UB, the vector is stored with what the replays observed. The log entry stays with status `klee-unconfirmed` and still needs a signed decision. It can be dismissed only with a reason.
- **D-24: WR-13 is closed first.** The first plan of Phase 4 closes runner residual WR-13: after a normal exit, find the processes holding the output pipes via `/proc/*/fd` and kill them with a pidfd. It comes with a test, before any KLEE run goes through the runner. This clears the STATE.md blocker.

#### CI

- **D-25: A new `ub` job in `testbed.yml`.** It runs in the same pinned image by digest, with the same hardening (no network for anything running captured or upstream code). The vectors arrive as an artifact through the already-pinned upload and download actions. It runs the fixtures self-test, the sanitizer variants, the `-O0` diff, KLEE, the relabelled corpus and `mt ub check`, with its own 50-annotation budget. The `testbed` and `vectors` jobs stay byte-identical, following the 03 D-32 pattern.

#### Sizing

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

#### Image

- **D-27: An early image plan, merged by the founder.** It follows the route of 02-01/02-02 and 03-10. Pin discovery covers the sanitizer runtime for clang-16 and the i386 libc and dev packages, plus whatever MSan needs. Then a container round trip, the founder's merge to `main` (only main publishes, WR-09), a new Current image digest, and a new PROJECT.md D-line. Offline work (log, fixtures, labels) proceeds meanwhile. Dependent CI work waits for the digest.

#### UB decisions log and gate (UB-04, UB-05)

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

### Deferred Ideas (OUT OF SCOPE)

- A UB manager or review UI for signing log entries (milestone v2.0, spec-v1).
- Frama-C Eva as a second static scanner (not planned; reconsider if KLEE coverage is thin).
- Recording unsigned-wrap hits as informational notes (rejected for now; may help Phase 7 or later intent checks).
- Corpus-wide re-runs of the UB pass on fuzz-generated inputs (Phase 5).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| UB-01 | Separate ASan+UBSan and MSan variants; replay the corpus against each; set `has_ub` on any flagged input | Q1 (A-08: MSan has no i386 runtime, so LP64 fallback), Q3 (packages), Q5 (report formats and option strings), the "Host ABI fidelity" pitfall (`-m32` alone does not reproduce AAPCS layouts), Patterns 2-3 |
| UB-02 | Replay at `-O0` and at shipped flags; tag disagreements as suspected UB | Q6 (`-O0` derivation from `probe.rs::recompile_arguments`; the allow-list holds only `-O3` and `-Os`, so a pair with no `-O` is already `-O0`), the optdiff pitfalls |
| UB-03 | KLEE scan of parser entries; per-function completion or timeout; merge into the UB log with provenance | Q2 (i386 is not usable with the pinned KLEE and klee-uclibc build, so x86-64 fallback), Q5 (`.err` format, `--ubsan-runtime`), Pattern 5 (deterministic option set, completion predicate, `.ktest` parsing in Rust) |
| UB-04 | char signedness, integer and pointer sizes, interface struct layouts extracted from the target config and signed off | Q7 (`-dM -E` macro set measured on arm-none-eabi GCC; `layout.rs` and `probe.rs` reuse) |
| UB-05 | Log with site or input class, decision, rationale, approver, date; default `return_error`; refuse UB-complete while anything is undecided | Pattern 6 (log schema in `mt-contract`, `Policy::validate(known_sites)` already takes the list), the D-28 to D-36 checks, the Security section on approver repudiation |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

- `#![forbid(unsafe_code)]` in every tool crate. `thiserror` in libraries, `anyhow` only in `mt-cli`. No `unwrap`/`expect` outside tests without an invariant comment (clippy `unwrap_used`/`expect_used` = deny in `Cargo.toml` `[workspace.lints.clippy]`).
- Every external tool runs through the `mt-toolchain` runner. `std::process::Command` is banned by `clippy.toml` `disallowed-types` everywhere except `runner.rs`.
- Deterministic outputs: explicit seeds and budgets, `BTreeMap`, `schema_version`, and timestamps only in manifest headers.
- One LLVM major (16) for clang, KLEE, c2rust and Hayroll. Python only under `research/`. Analysers only as subprocesses.
- Each requirement ID has a test whose name contains it in snake case (`ub_01_...` to `ub_05_...`). Tests never use the network or a live model.
- Pre-merge: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo deny check`, and the container version check.
- Reports: timeouts are "not proven", missing layers are "unavailable", and the words "certified" and "compliant" never appear.
- Never run with `--dangerously-skip-permissions`. Each phase must be finishable in 1-2 part-time weeks.
- Work goes through GSD commands. Image changes go through pin discovery, a container round trip and the founder's merge to `main`.

## Summary

The phase is mostly integration of well-understood tools into the existing harness. Four facts decide the plan.

**A-08 is confirmed.** In LLVM 16, MSan's instrumentation pass can target i386 (`Linux_I386_MemoryMapParams` exists), but compiler-rt builds no i386 MSan runtime. `ALL_MSAN_SUPPORTED_ARCH` is `${X86_64} ${MIPS64} ${ARM64} ${PPC64} ${S390X}`, and `msan.h` hits `#error "Unsupported platform"` for 32-bit Linux. The Ubuntu `libclang-rt-16-dev` package ships `libclang_rt.msan-x86_64.a` only, while ASan and UBSan come in both i386 and x86-64 forms. So `clang -m32 -fsanitize=memory` compiles but cannot link. The LP64 fallback of D-03 is what happens.

**D-18 resolves to the x86-64 fallback.** KLEE 3.2 picks a `32_` runtime suffix from the module triple. It builds those runtimes only if the build stage can run `-m32` programs (`M32_SUPPORTED`), and the pinned `klee-build` stage has no 32-bit libc. klee-uclibc is a single `klee-uclibc.bca`, configured for the host x86-64 target.

**Plain `clang -m32 -funsigned-char` does not reproduce the Cortex-M ABI.** On a probe struct, i386 SysV puts a `long long` at offset 4 and `sizeof` at 24 with 4-byte enums. arm-none-eabi GCC gives offset 8, size 32 and 1-byte enums. Adding `-malign-double -fshort-enums` matched exactly. So the host flags must be derived from the extracted target facts (UB-04), and then checked against the host object's own DWARF.

**WR-13 can be closed in safe Rust with no new dependency.** Read the pipe inode of each read end (`std::fs::read_link("/proc/self/fd/N")` gives `pipe:[ino]`). After the drain grace, scan `/proc/*/fd` for holders of that inode, then use the existing `rustix` `pidfd_open` / `pidfd_send_signal` path with its start-time check. A prototype found a `setsid` escapee (ppid 1, its own session) as the only holder after a normal exit.

**Primary recommendation:** Build a new `mt-ub` crate for the engine, and put the UB-log schema types in `mt-contract`. Run the host i386 ASan+UBSan build with flags derived from the target facts and verified by DWARF. Run MSan on x86-64 LP64. Run KLEE on x86-64 bitcode with `--libc=uclibc --ubsan-runtime --external-calls=none --search=dfs --max-instructions`. Parse only the `SUMMARY:` line and the first stack of each report into `source:kind` plus the innermost target frame. Keep raw reports out of anything hashed.

## Answers to the Researcher Questions (CONTEXT)

### Q1. D-03 / A-08: does MSan support i386 (`-m32`) with clang 16?

**No. The LP64 fallback applies.** Confidence: HIGH.

- Compiler-rt 16 runtime arch list: `set(ALL_MSAN_SUPPORTED_ARCH ${X86_64} ${MIPS64} ${ARM64} ${PPC64} ${S390X})`, while `set(ALL_ASAN_SUPPORTED_ARCH ${X86} ${X86_64} ...)` and `set(ALL_UBSAN_SUPPORTED_ARCH ${X86} ${X86_64} ...)` include i386. [CITED: llvm-project llvmorg-16.0.6 compiler-rt/cmake/Modules/AllSupportedArchDefs.cmake]
- `compiler-rt/lib/msan/msan.h` (16.0.6) defines `kMemoryLayout` only for `SANITIZER_LINUX && defined(__mips64)`, `__aarch64__`, `SANITIZER_PPC64`, `SANITIZER_S390_64` and `SANITIZER_NETBSD || (SANITIZER_LINUX && SANITIZER_WORDSIZE == 64)`, followed by `#else #error "Unsupported platform"`. [CITED: llvm-project llvmorg-16.0.6 compiler-rt/lib/msan/msan.h]
- The instrumentation pass does accept i386: `case Triple::x86: MapParams = Linux_X86_MemoryMapParams.bits32;`. [CITED: llvm-project llvmorg-16.0.6 llvm/lib/Transforms/Instrumentation/MemorySanitizer.cpp] So the failure appears at link time, not compile time. This is a trap for a "does it build" probe.
- Measured (proxy: Ubuntu noble `libclang-rt-16-dev 1:16.0.6-23ubuntu4`, `dpkg -c`): the package holds `libclang_rt.asan-i386.a`, `libclang_rt.asan_static-i386.a`, `libclang_rt.ubsan_standalone-i386.a`, and the x86-64 versions of each, but only `libclang_rt.msan-x86_64.a` for MSan. [VERIFIED: dpkg -c of the downloaded .deb, this session; Ubuntu proxy for Debian bookworm `1:16.0.6-15~deb12u1`]
- Measured (proxy: host clang 18.1.3): `clang -m32 -fsanitize=memory -c t.c` exits 0. Linking asks for `/usr/lib/llvm-18/lib/clang/18/lib/linux/libclang_rt.msan-i386.a`, which no package provides. [VERIFIED: this session, clang 18 proxy]

**Fallback to document in the contract:** MSan builds use `clang-16 -m64 -funsigned-char -fshort-enums` (the last flag only when the target facts say enums are 1 byte) plus `-fsanitize=memory -fsanitize-memory-track-origins=2 -fno-sanitize-recover=all`. Each MSan finding records `abi = "x86_64-lp64"`. `long` and pointers are 8 bytes there, so code that depends on `sizeof(long)` may behave differently. That is why D-15 lets a signed decision dismiss a host artifact with a reason.

### Q2. D-18: does KLEE 3.2 with klee-uclibc (LLVM 16) run i386 bitcode built with `-funsigned-char`?

**No, not with the pinned image as built. Use x86-64 bitcode and record the ABI per finding.** Confidence: MEDIUM-HIGH from source reading. Confirm with one probe in the image plan (below).

Evidence [CITED: github.com/klee/klee tag v3.2, files named]:
- `tools/klee/main.cpp:1356-1364`: `std::string opt_suffix = "64"; // Fall back to 64bit`, which becomes `"32"` when the module triple contains `i686`/`i586`/`i486`/`i386`, then gets `"_" + RuntimeBuild`. Every runtime library (`libkleeRuntimeFreestanding`, `libkleeRuntimePOSIX`, `libkleeUBSan`, ...) is loaded as `<name> + opt_suffix + ".bca"`.
- `runtime/CMakeLists.txt:14-19`: `set(bc_architectures 64)`, plus `"32"` only `if (M32_SUPPORTED)`. Top-level `CMakeLists.txt:451-457` sets `M32_SUPPORTED` with `check_c_source_runs` under `-m32`. That needs a 32-bit libc and crt in the build stage.
- `container/Dockerfile` `klee-build` stage is `FROM build-base`. Its packages are `build-essential cmake curl git libsqlite3-dev libz3-dev ninja-build patch pkg-config python3 zlib1g-dev`, with no `libc6-dev-i386` or `gcc-multilib`. [VERIFIED: read this session, Dockerfile lines 145-163, 441-508] So `M32_SUPPORTED` is expected to be 0 [ASSUMED until the image build log line `32bit platform supported:` is read].
- `linkWithUclibc` (main.cpp:1209-1230) loads a single `KLEE_UCLIBC_BCA_NAME` = `"klee-uclibc.bca"`, with no architecture suffix (CMakeLists.txt:483). The Dockerfile configures klee-uclibc with `./configure --make-llvm-lib --with-cc .../clang --with-llvm-config ...` and no target flag, so it is the host's x86-64 build.
- `main.cpp:1350-1353`: a module triple different from the host triple only produces `klee_warning("Module and host target triples do not match...This may cause unexpected crashes or assertion violations.")`. KLEE would not refuse, but it would link 32-bit user code with a 64-bit libc. That result is not trustworthy.

Making i386 work would need multilib in `klee-build`, a second i386 klee-uclibc build, and a KLEE patch (one uclibc name). None of that fits a 1-2 week phase.

**Fallback:** compile bitcode with `clang-16 --target=x86_64-linux-gnu -funsigned-char` (plus `-fshort-enums` when the target facts require it), and record `abi = "x86_64-lp64"` in each KLEE finding.

**Probe for the image plan** (one runner call, a fact rather than a gate): `klee --libc=uclibc --output-dir=<tmp> probe32.bc`, where `probe32.bc` is built with `clang-16 -m32 -emit-llvm -c`. Record whether it errors on `libkleeRuntime*32_Release.bca` or warns on the triple. Also record `ls /opt/klee/lib/klee/runtime/` in the image notice.

### Q3. D-27: packages for the clang-16 sanitizer runtimes and the i386 libc

Base: `debian:bookworm-20261005-slim` at snapshot `20261005T000000Z`. clang is `clang-16 = "1:16.0.6-15~deb12u1"`. [VERIFIED: container/pins.toml read this session]

Debian mirrors (`packages.debian.org`, `sources.debian.org`, `deb.debian.org`, `snapshot.debian.org`) were not reachable from this session, so the list below is measured on the Ubuntu noble proxy. The pin-discovery workflow must confirm each name and version at the snapshot.

| Package (bookworm name) | Provides | Evidence |
|---|---|---|
| `libclang-rt-16-dev` | `/usr/lib/llvm-16/lib/clang/16/lib/linux/libclang_rt.{asan,asan_static,ubsan_standalone}-{i386,x86_64}.a`, `libclang_rt.msan-x86_64.a`, `clang_rt.crtbegin-i386.o` | Noble proxy `dpkg -c`. Its `Depends: lib32stdc++6 (>= 4.1.1), libc6 (>= 2.38), libc6-i386 (>= 2.38), libstdc++6`. [VERIFIED: proxy] Debian split this package out of `libclang-common-16-dev` in the LLVM 15+ packaging. [ASSUMED] |
| `libc6-dev-i386` | `/usr/lib32/crt1.o`, `Scrt1.o`, `crti.o`, `libc.so` and the i386 headers that `clang -m32` links against | Noble: `clang -m32` link fails with `cannot find Scrt1.o`/`crti.o` without it. [VERIFIED: proxy measurement] Bookworm name [ASSUMED] (it is the same in Debian). |
| `lib32gcc-12-dev` | `crtbeginS.o` and `libgcc.a` for 32-bit under `/usr/lib/gcc/x86_64-linux-gnu/12/32/`; clang uses GCC's 32-bit crt objects | Noble needs `lib32gcc-13-dev` (`cannot find crtbeginS.o` without it). [VERIFIED: proxy] Bookworm's default GCC is 12, so the name `lib32gcc-12-dev` is [ASSUMED]. |
| `libc6-i386`, `lib32stdc++6`, `lib32gcc-s1` | i386 runtime libraries, pulled in as dependencies | Proxy dependency list. These must appear in the `[apt]` pins only if `mt toolchain check` requires every installed package to be pinned. Follow the existing 02-01 pattern. |
| `llvm-16` (already pinned) | `/usr/lib/llvm-16/bin/llvm-symbolizer` for symbolizing reports | [ASSUMED] path. Confirm with `ls` in the image plan. |

**Placement:** the runtimes must be in the `final` image. Add them to the `apt-base` install list. The Dockerfile comment at lines 113-118 already reserves this ("Sanitizer and libFuzzer runtimes are left out: they serve later phases and arrive through the pin-bump flow then"). Also add new `[tool.*]` rows or a smoke step, for example a 3-line C file built and run as i386 ASan, i386 UBSan and x86-64 MSan in `container.yml` image-a. That proves the runtimes link and run inside the image before any harness code depends on them.

### Q4. D-24: WR-13 under `#![forbid(unsafe_code)]`

**Approach (safe Rust, no new crate or feature):**

1. Before `spawn_reader` takes the pipes, record each read end's identity with `std::fs::read_link(format!("/proc/self/fd/{}", pipe.as_raw_fd()))`, which gives a string like `pipe:[7455]`. `AsRawFd::as_raw_fd` is a safe call. `rustix::fs::fstat` would also work, but it needs rustix's `fs` feature. The workspace enables only `["std", "process"]`. [VERIFIED: Cargo.toml read this session]
2. Only when the drain grace expires (`!(stdout_done && stderr_done)`, runner.rs:707-714) and on the timeout path: scan `std::fs::read_dir("/proc")`. For each numeric pid other than `std::process::id()`, `read_dir("/proc/<pid>/fd")` and `read_link` each entry. A pid is a holder if any link equals one of the recorded ids.
3. For each holder, reuse the existing pattern in `kill_tree` (runner.rs:520-558): `pidfd_open`. Then confirm the pidfd names the scanned process by re-checking the start time. Better still, re-scan that pid's fd directory to see that it still holds the pipe. Then `pidfd_send_signal(&fd, Signal::STOP)`, repeat until no new holder appears (bounded rounds, like `ESCAPE_SCAN_ROUNDS`), then `Signal::KILL`.
4. Record this in the run record. For example, a new `exit.pipe_holders_killed: u32`, which needs `RUN_RECORD_SCHEMA_VERSION` bumped from 2 to 3. [VERIFIED: runner.rs:69 `pub const RUN_RECORD_SCHEMA_VERSION: u32 = 2;`] The alternative is to fold it into `tree_kill_sent`. Prefer the explicit field, because the evidence bundle should say what happened.

**Prototype evidence** (scratch program with `#![forbid(unsafe_code)]`, std only, this session): the child was `sh -c "(setsid sleep 7 2>/dev/null &); exit 0"`. After the child exited, stdout was `pipe:[7455]` with holders `[2759]`, and `/proc/2759/stat` was `2759 (sleep) S 1 2759 2759` (ppid 1, its own session and group). stderr had no holders because of `2>/dev/null`. [VERIFIED: run this session]

**Why this is safe:** a pipe inode is unique to one run's pipe, so concurrent runs never kill each other's processes. Reading `/proc/<pid>/fd` needs ptrace-read access (same uid in the CI containers, which run as `--user 65534:65534`). Processes of another uid are invisible and are left alone. The existing "residual limits" doc comment must be updated.

**Test impact:** `tool_01_runner_marks_a_stream_held_by_an_orphaned_escapee_incomplete` (`crates/mt-toolchain/tests/tool_01_runner.rs`) currently pins the old limit (`assert!(!output.record.stdout.complete)`). It must be rewritten RED-first to expect `complete: true` and a dead escapee. Add a new normal-exit test (`sh -c "(setsid sleep 30 &); exit 0"`) that asserts the escapee is gone and the streams are complete. A suitable name is `tool_01_runner_kills_a_pipe_holder_after_a_normal_exit` (`tool_01` is the requirement ID the runner tests already carry).

### Q5. D-32 / D-05: report formats and symbolization

**Run each sanitized binary with explicit options.** Never rely on defaults. Pass them through the runner's `extra_env`:

| Variant | Env var | Value (one string, `:`-separated) |
|---|---|---|
| ASan+UBSan (i386) | `ASAN_OPTIONS` | `detect_leaks=0:exitcode=86:abort_on_error=0:symbolize=1:external_symbolizer_path=/usr/lib/llvm-16/bin/llvm-symbolizer:strip_path_prefix=<pair src root>/:color=never:print_summary=1:log_path=<scratch>/asan` |
| same binary | `UBSAN_OPTIONS` | `halt_on_error=1:print_stacktrace=1:report_error_type=1:print_summary=1:exitcode=87:symbolize=1:external_symbolizer_path=...:strip_path_prefix=...:color=never:log_path=<scratch>/ubsan` |
| MSan (x86-64) | `MSAN_OPTIONS` | `exitcode=88:print_summary=1:symbolize=1:external_symbolizer_path=...:strip_path_prefix=...:color=never:log_path=<scratch>/msan` |

The flag names were checked in LLVM 16 sources: `UBSAN_FLAG(bool, halt_on_error, ...)`, `print_stacktrace`, `report_error_type`, and the common flags `symbolize`, `external_symbolizer_path`, `strip_path_prefix`, `log_path`, `detect_leaks`, `print_summary`, `color`, `exitcode` (`COMMON_FLAG(int, exitcode, 1, ...)`). [CITED: compiler-rt/lib/ubsan/ubsan_flags.inc and sanitizer_common/sanitizer_flags.inc at llvmorg-16.0.6] Distinct exit codes (86, 87, 88; [ASSUMED], at the planner's discretion) separate a sanitizer stop from the C's own return code. `detect_leaks=0` is required: a leak is not UB, and LeakSanitizer needs ptrace, which `--cap-drop ALL` removes. `log_path` keeps the report out of the program's stderr, which is a vector field. The file is named `<log_path>.<pid>`, so read the single file in the scratch directory.

**Formats to parse** (proxy run of Ubuntu GCC 13 libsanitizer, which shares the compiler-rt report code; clang-16 with `llvm-symbolizer` adds `:col` to stack frames):

```
# UBSan (-fno-sanitize-recover=all, report_error_type=1, print_stacktrace=1)
ub.c:2:27: runtime error: signed integer overflow: 5 + 2147483647 cannot be represented in type 'int'
    #0 0x564536d2a33d in sovf ub.c:2
    #1 0x564536d2a5c4 in main ub.c:6

SUMMARY: UndefinedBehaviorSanitizer: signed-integer-overflow ub.c:2:27 in
```
```
# ASan
==2314==ERROR: AddressSanitizer: heap-buffer-overflow on address 0x502000000015 at pc ... bp ... sp ...
READ of size 1 at 0x502000000015 thread T0
    #0 0x55f28e6de457 in oob_heap ub.c:4
    #1 0x55f28e6de5e2 in main ub.c:6
...
SUMMARY: AddressSanitizer: heap-buffer-overflow ub.c:4 in oob_heap
```
[VERIFIED: run this session with `strip_path_prefix` and `exitcode=87` honoured, GCC libsanitizer proxy]

- **Kind token:** the word after `SUMMARY: <Tool>: `. For UBSan, `report_error_type=1` is what turns the generic `undefined-behavior` into the specific SummaryKind. The UBSan SummaryKind list (16.0.6 `ubsan_checks.inc`) includes `signed-integer-overflow`, `invalid-shift-exponent`, `invalid-shift-base`, `out-of-bounds-index`, `integer-divide-by-zero`, `null-pointer-use`, `misaligned-pointer-use`, `pointer-overflow`, `invalid-bool-load`, `invalid-enum-load`, `float-cast-overflow`, `unreachable-call`, `missing-return`, `non-positive-vla-index`, `nullptr-with-offset`, `nullptr-with-nonzero-offset`, `nullptr-after-nonzero-offset`, `alignment-assumption`, `insufficient-object-size` and `invalid-builtin-use`. [CITED: compiler-rt/lib/ubsan/ubsan_checks.inc llvmorg-16.0.6] Observed ASan kinds include `heap-buffer-overflow`, `stack-buffer-overflow`, `global-buffer-overflow`, `heap-use-after-free`, `double-free` and `SEGV`. MSan's kind is `use-of-uninitialized-value`. All fit the existing label grammar `^[a-z][a-z-]*(:[A-Za-z0-9_.-]+)*$` (`SEGV` included). [VERIFIED: crates/mt-vectors/src/vector.rs:104-125]
- **Site:** parse the first stack block only (the lines `    #N 0x... in <func> <file>:<line>[:<col>]` up to the first blank line). Take the innermost frame whose file is under the pair's source tree and is not the generated driver. Do not use the `SUMMARY` location: for `double-free` it named `../../../../src/libsanitizer/asan/asan_malloc_linux.cpp:52 in free` (an interceptor), as measured. For UBSan, the first line's `file:line:col` is the check site itself and is always in instrumented user code.
- **Determinism:** a report contains pids (`==2314==`), addresses and build IDs. Never hash or commit raw report text. Store only the parsed `(source, kind, function, file:line:col, frame list without addresses)`, and the report's sha256 if needed in a non-compared field.
- **Symbolization:** compile with `-g -fno-omit-frame-pointer`. Point `external_symbolizer_path` at the pinned `llvm-symbolizer` rather than relying on PATH lookup. [CITED: clang/docs/UndefinedBehaviorSanitizer.rst llvmorg-16.0.6: "Compile with -g and -fno-omit-frame-pointer", "Make sure llvm-symbolizer binary is in PATH"]

**KLEE `.err` format** [CITED: klee v3.2 lib/Core/Executor.cpp:3915-3927, include/klee/Core/TerminationTypes.h]:
```
Error: <message>
File: <file>
Line: <line>
assembly.ll line: <n>
State: <id>
Stack:
	#<n> in <func>(...) at <file>:<line>
Info:
...
```
- The file suffix gives the kind: `ptr.err`, `free.err`, `overflow.err`, `div.err`, `overshift.err`, `abort.err`, `assert.err`, `read_only.err`, `report_error.err`, `invalid_load.err`, `exec.err`, `external.err`, `model.err`, `user.err`, `solver.err`, and others. Emit `klee:<suffix stem>` (for example `klee:ptr`, `klee:overflow`). The message text adds detail (`memory error: out of bound pointer`, `memory error: null page access`, `memory error: use after free`, `memory error: double free`, `memory error: invalid pointer: <name>`).
- With `--ubsan-runtime`, KLEE links `libkleeUBSan<suffix>.bca`, and the error message is the UBSan SummaryKind (`report_error(ConvertTypeToString(ET), get_suffix(ET))`), for example `signed-integer-overflow` in `overflow.err`. The exception is shifts, whose literal message is `"shift out of bounds"`. [CITED: klee v3.2 runtime/Sanitizer/ubsan/ubsan_handlers.cpp:35-112, 204-268] D-17 needs `--ubsan-runtime`. Without it, the `__ubsan_handle_*` calls in the bitcode are unresolved externals.
- KLEE reports **file and line, never a column**. See Pitfall 6 for what this means for the D-32 site key.
- `.err` files are not written for `exec.err`/`external.err`/`solver.err` failures of the harness itself. Map those to drop causes (`klee_exec_error`, `klee_external_call`, `klee_solver_error`), not to UB entries.

### Q6. D-12: deriving the `-O0` rebuild

- The captured compile commands are canonical and have already passed `walk_arguments` over `ALLOWED_OPTIONS`. The only optimisation spellings on the allow-list are `exact("-O3")` and `exact("-Os")`. [VERIFIED: crates/mt-capture/src/cc.rs:290-294, quoted `exact("-O3"), exact("-Os"),`]
- An existing, reviewed helper already rebuilds a captured unit with changed flags: `probe::recompile_arguments` drops `-o`/`-MF`/`-MT`/`-MQ` with their operands, plus every argument starting `-o`, `-M`, `-O`, `-g`, then appends `-g -gdwarf-5 -O0`, `-c` and `-o <scratch>`. [VERIFIED: crates/mt-harness/src/probe.rs:251-288] Tool-built commands like this are not re-walked against the allow-list, which is captured-command evidence only.
- **Recommendation:** add a sibling `optlevel_arguments(compiler, arguments, scratch, level)`. It removes only `-O*` and the output and depfile options, keeps `-g` as captured, and appends the requested `-O0`. It returns whether a shipped `-O` was present. Rebuild every translation unit of the pair (not only the entry's), then link with the unchanged `link.rs` and driver. Run the Phase 3 executor with an inflated budget (see Pitfall 8).
- If no `-O` was present, the shipped build is already `-O0` (GCC's default). Record `optdiff: not_applicable` with cause `shipped_is_O0` in the provenance. Do not create an entry.

### Q7. D-33: facts from the pinned arm-none-eabi GCC

Measured with `arm-none-eabi-gcc 13.2.1` (Ubuntu package, a proxy for the pinned `14.3.Rel1`) and the shipped Cortex-M4 flags from the allow-list:
`echo | arm-none-eabi-gcc -mcpu=cortex-m4 -mthumb -mfloat-abi=hard -mfpu=fpv4-sp-d16 -Os -dM -E -x c -` gives `__CHAR_UNSIGNED__ 1`, `__CHAR_BIT__ 8`, `__SIZEOF_SHORT__ 2`, `__SIZEOF_INT__ 4`, `__SIZEOF_LONG__ 4`, `__SIZEOF_LONG_LONG__ 8`, `__SIZEOF_POINTER__ 4`, `__SIZEOF_SIZE_T__ 4`, `__SIZEOF_PTRDIFF_T__ 4`, `__SIZEOF_WCHAR_T__ 4`, `__SIZEOF_FLOAT__ 4`, `__SIZEOF_DOUBLE__ 8`, `__SIZEOF_LONG_DOUBLE__ 8`, `__SIZE_TYPE__ unsigned int`, `__ARM_SIZEOF_MINIMAL_ENUM 1`, `__ARM_SIZEOF_WCHAR_T 4`, `__BYTE_ORDER__ __ORDER_LITTLE_ENDIAN__`, `__BIGGEST_ALIGNMENT__ 8`. With `-fsigned-char`, `__CHAR_UNSIGNED__` disappears. [VERIFIED: run this session, GCC 13 proxy]

- **Command:** the unit's canonical arguments, through the same stripping as `recompile_arguments` but with `-c` replaced by `-dM -E -x c /dev/null` (or an empty scratch file). Run through the runner. Parse `#define NAME VALUE` lines into a `BTreeMap`. Keep the whole sorted macro set's sha256 in the facts block so that any change unsigns it (D-33), and show the named subset for sign-off.
- **Struct layouts:** reuse `probe::layout_for_entry` and `layout::layout_summary`. They already lower `llvm-dwarfdump --debug-info` text into `CType` with sizes, offsets and bitfields. [VERIFIED: crates/mt-harness/src/layout.rs:1-14, 221-227, 843; probe.rs:1-13] The "structs crossing the interface" are the entry's `Role::State` pointee types, the `ret` state for struct returns, and typed globals (03 decisions in STATE.md).
- Add `__ARM_SIZEOF_MINIMAL_ENUM` (enum width), `__BIGGEST_ALIGNMENT__` and `__SIZEOF_LONG_DOUBLE__` to the signed facts, because these drive the host ABI flags (Pitfall 1).

## Architectural Responsibility Map

This is a single-binary CLI tool. Its "tiers" are crates and subprocess boundaries.

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Process supervision, WR-13 pipe-holder kill | `mt-toolchain::runner` | — | Only module allowed to spawn (clippy ban); D-24 |
| UB-log and facts schema (strict TOML types, validation, merge, D-28 decided predicate) | `mt-contract` (new `ublog` module) | `mt-cli` | Shared with Phases 5 and 11 and the policy loader; `Policy::validate(known_sites)` already lives there |
| Sanitizer, `-O0`, KLEE orchestration; report parsers; relabelling | new `mt-ub` crate | `mt-harness` (executor, link, probe, layout), `mt-vectors` (corpus, re-hash) | `docs/guidelines/building-the-tool.md` §3 lists `mt-ub` for exactly this |
| Host runtime template (C) | `mt-ub` (rendered like `runtime.c`) | `mt-harness::driver` role rendering | D-05: a reviewed shared template; the target `runtime.c` uses raw ARM `svc` calls and cannot be reused |
| Implementation-defined facts extraction | `mt-ub::facts` calling `mt-harness::probe`/`layout` | `mt-contract::ublog` (storage) | Reuses the DWARF reader |
| Gate and sign CLI (`mt ub run/facts/check/sign`) | `mt-cli` | — | `anyhow` only here |
| CI `ub` job | `.github/workflows/testbed.yml` | `container.yml` (runtime smoke) | D-25 |
| Pins for runtimes and i386 libc | `container/pins.toml`, `container/Dockerfile` | `pin-discovery.yml` | D-27 |

## Standard Stack

### Core (all already pinned or in the workspace; no new crates)

| Component | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| clang-16 + compiler-rt (`libclang-rt-16-dev`) | 1:16.0.6-15~deb12u1 (to pin) | i386 ASan+UBSan, x86-64 MSan | The only LLVM major allowed (PROJECT D-15) |
| KLEE | 3.2 (commit 92ee8201…) | Symbolic scan with `--ubsan-runtime` | Already pinned and checked |
| klee-uclibc | klee_uclibc_v1.4 | libc for KLEE (x86-64) | Already pinned |
| arm-none-eabi GCC | 14.3.Rel1 | `-O0` rebuild, `-dM -E` facts, DWARF | The judge toolchain |
| QEMU user | 1:10.0.2+ds-2+deb13u1~bpo12+1 | Target replays (`-O0`, trap UBSan) | Phase 3 executor |
| `llvm-symbolizer` / `llvm-dwarfdump` / `llvm-link` (llvm-16) | 16.0.6 | Symbolize reports; host DWARF layout check; link bitcode | Already in the image |
| Rust crates | existing: `serde`, `toml =1.1.7`, `serde_json`, `sha2`, `regex`, `thiserror`, `rustix 1.1.5` (`std`,`process`) | Log, parsers, runner | No new dependency is needed |

**Installation (image, via the pin flow):** `libclang-rt-16-dev libc6-dev-i386 lib32gcc-12-dev` added to the `apt-base` install list with `[apt]` pins from pin discovery. Exact versions come from the pin-discovery workflow and are [ASSUMED] until then.

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| KLEE x86-64 bitcode | KLEE i386 (rebuild uclibc for i386, multilib, patch KLEE) | Matches ILP32, but costs an image redesign and a KLEE patch; not finishable in the phase |
| Parsing `.ktest` in Rust | `ktest-tool` | `ktest-tool` is Python, and the final image has no `python3` (only `build-base` installs it; Dockerfile line 162) |
| `rustix::fs::fstat` for pipe inodes | `std::fs::read_link("/proc/self/fd/N")` | The std route needs no new rustix feature; both are safe |

## Package Legitimacy Audit

No new Rust crates and no npm or PyPI packages are installed. The seam was not run, because there is nothing to check in a language registry. The new artefacts are Debian apt packages from the pinned snapshot, governed by the existing pin-discovery and `[apt]` check flow:

| Package | Registry | Source | Verdict | Disposition |
|---------|----------|--------|---------|-------------|
| libclang-rt-16-dev | Debian snapshot (bookworm) | llvm-toolchain-16 source package | Name measured on the Ubuntu proxy [VERIFIED: proxy] | Approved, subject to pin discovery |
| libc6-dev-i386 | Debian snapshot | glibc | [ASSUMED] bookworm name | Pin discovery confirms |
| lib32gcc-12-dev | Debian snapshot | gcc-12 | [ASSUMED] version suffix 12 | Pin discovery confirms; the founder merges |

**Packages removed:** none. **Packages flagged suspicious:** none.

## Architecture Patterns

### System Architecture Diagram

```
 retained pairs (capture tree) + corpus (sha256 vectors + index.json) + manifest roles + testbed/policy
        |                                   |
        v                                   |
  [facts] arm-none-eabi-gcc -dM -E ; DWARF probe ──> target_facts (per config) ──> host ABI flags
        |                                                                            |
        +──> [host build i386]  clang-16 -m32 <derived flags> -fsanitize=address,undefined ──┐
        |       check host DWARF layout == target layout? ── no ──> sanitizer: unavailable   |
        +──> [host build LP64]  clang-16 -m64 ... -fsanitize=memory ──────────────────────────┤
        +──> [target -O0]       arm-none-eabi-gcc <shipped minus -O> -O0 ──> QEMU executor ───┤
        +──> [target trap]      ... -fsanitize=undefined -fsanitize-undefined-trap-on-error ──┤ (cut list)
        +──> [KLEE]             clang-16 x86_64 bitcode + harness main ──> klee ──> .err/.ktest
        |                                                     |                               |
        |                                   .ktest -> witness vector -> replay (QEMU + variants)
        v                                                                                     v
  replay every vector (runner, budgets) ──> parse report (SUMMARY + first stack) ──> Finding{source,kind,site,abi}
        |
        v
  merge: label = first source in fixed order ──> rewrite vectors, re-hash, index.json (ub_checked, variants)
        |
        v
  UB log testbed/ub-log/<target>.toml  (merge by site key; stale; witnesses; proposals)
        |
        v
  mt ub check ── policy cross-check (Policy::validate(known_sites)) ── facts re-extract == signed? ──> ub-complete | list
```

### Recommended Project Structure

```
crates/mt-contract/src/ublog.rs      # strict TOML schema: Log, Entry, Decision, TargetFacts, Status; merge; decided()
crates/mt-ub/src/lib.rs              # #![forbid(unsafe_code)], thiserror
crates/mt-ub/src/facts.rs            # -dM -E parse + DWARF layouts via mt-harness::probe
crates/mt-ub/src/hostflags.rs        # exhaustive map of ALLOWED_OPTIONS spellings -> keep/drop for host; fact-derived flags
crates/mt-ub/src/host_runtime.c      # second reviewed template (libc, malloc'd exact-size buffers, vector file input)
crates/mt-ub/src/report.rs           # ASan/UBSan/MSan report parser -> Finding
crates/mt-ub/src/klee.rs             # bitcode build, harness main, option set, .err/.ktest parse, completion
crates/mt-ub/src/optdiff.rs          # -O0 rebuild + compare via mt-vectors::compare
crates/mt-ub/src/relabel.rs          # source order, has_ub rewrite, provenance
crates/mt-ub/fixtures/*.c            # D-08 known-UB samples (+ manifest-role sidecar)
crates/mt-ub/tests/ub_0N_*.rs        # requirement-named tests; report-text fixtures captured from CI
testbed/ub-log/<target>.toml         # D-29
```

### Pattern 1: Host ABI flags derived from facts, then verified

Map each `ALLOWED_OPTIONS` spelling with an exhaustive `match` (a new allow-list spelling then fails to compile until someone classifies it):
- **drop:** `-mcpu=`, `-mthumb`, `-mfloat-abi=`, `-mfpu=`, `-specs=`, `-O*`, `-o`, `-M*`, `-Werror*`, `-c` (re-added)
- **keep:** `-D`, `-I`, `-std=`, `-W*` warnings, `-fvisibility=hidden`, `-fPIC`, `-fstack-protector-strong`, `-pedantic`

Then add facts-derived flags:
- `-funsigned-char` iff `__CHAR_UNSIGNED__`
- `-fshort-enums` iff `__ARM_SIZEOF_MINIMAL_ENUM == 1`
- for i386 only: `-malign-double` when the target aligns 64-bit types to 8, and `-mlong-double-64` when `__SIZEOF_LONG_DOUBLE__ == 8`

Finally, check the host object's DWARF layout of every interface struct against the target facts with the same `layout.rs`. On a mismatch, record `sanitizer: unavailable`, cause `host_layout_mismatch`.

Measured on the probe struct `{char; long long; double; enum}`: arm `off_ll=8 sz=32 enum=1`; `clang -m32 -funsigned-char` `off_ll=4 sz=24 enum=4`; with `-malign-double -fshort-enums` it gives `off_ll=8 sz=32 enum=1`. `clang -m32 -mlong-double-64` gives `__SIZEOF_LONG_DOUBLE__ 8`. [VERIFIED: this session, clang 18 and GCC 13 proxies]

### Pattern 2: Sanitized host build options

- i386: `clang-16 -m32 -g -fno-omit-frame-pointer -O1 -fsanitize=address,undefined -fno-sanitize-recover=all <kept flags> <fact flags>`
- LP64 MSan: `clang-16 -m64 -g -fno-omit-frame-pointer -O1 -fsanitize=memory -fsanitize-memory-track-origins=2 -fno-sanitize-recover=all ...`

Notes:
- `-fsanitize=undefined` already excludes `unsigned-integer-overflow`, `float-divide-by-zero`, `implicit-conversion`, `local-bounds` and `nullability-*`. [CITED: clang/docs/UndefinedBehaviorSanitizer.rst llvmorg-16.0.6] D-02 needs no `-fno-sanitize=`, but adding `-fno-sanitize=unsigned-integer-overflow` explicitly documents the intent at no cost.
- `-O1` is [ASSUMED] as a balance between speed and detection. The shipped `-Os`/`-O3` can fold away some UB before ASan sees it. The `-O0` target differential covers the optimiser angle separately.

### Pattern 3: Host runtime template

Use the same request-file format as `runtime.c` (`"MTRQ"`, fields 0-5) so that vectors replay unchanged. Under ASan, copy the input into a `malloc(len)` buffer of exactly the input length, so a one-byte over-read hits a redzone. A page guard only catches reads past the end of the page. Leave output and state buffers uninitialised except for what `lib_state_in` specifies, so MSan can see reads of uninitialised memory. Do not use an arena allocator: the target runtime has `mt_heap_arena` (runtime.c:305), and a custom allocator hides heap errors from ASan and MSan. The host run needs only "report or no report" plus rc. It does not rebuild `lib_state`. [VERIFIED: runtime.c header and lines 118-150, 305 read this session]

### Pattern 4: Merging findings into vectors

Fixed source order (discretion; document it in the contract): `asan` > `ubsan` > `msan` > `ubsan-trap` > `klee` > `optdiff` > `crash` > `timeout`. The ILP32 kinded sources come first, then LP64, then the kindless target trap, then derived classes. `HasUbSource::Phase4` already exists and `issues()` accepts it whether or not a label is present. [VERIFIED: crates/mt-vectors/src/corpus.rs:28-46, 274-282] Extend `Provenance` with `ub_checked { variants_run, variants_unavailable, results }` as a `schema_version` bump of the index. The re-hash renames the file, and `write_corpus` already re-checks provenance against each vector.

### Pattern 5: KLEE harness and deterministic option set

- **Harness main (generated):** `klee_make_symbolic(buf, N, "in")`, a symbolic `len` with `klee_assume(len <= N)`, each scalar made symbolic with `klee_assume(x == v1 | x == v2 ...)` over its sweep values (one run per entry; KLEE forks per value), setup steps called concretely, then the entry.
- **Bitcode:** `clang-16 --target=x86_64-linux-gnu -emit-llvm -c -g -O0 -Xclang -disable-O0-optnone -funsigned-char [-fshort-enums] -fsanitize=signed-integer-overflow,shift,integer-divide-by-zero,bounds,null,pointer-overflow` per TU, then `llvm-link`. The exact `-fsanitize` subset is [ASSUMED]. Keep to checks that KLEE's UBSan runtime handles (see `get_suffix` in ubsan_handlers.cpp).
- **Run:**

  ```
  klee --output-dir=<fresh> --libc=uclibc --ubsan-runtime --external-calls=none \
       --search=dfs --max-instructions=<B> --max-time=<cap>s --max-memory=<large> \
       --max-solver-time=0 --emit-all-errors=false --write-no-tests=false --entry-point=<harness> h.bc
  ```

  - `--search=dfs` needs no RNG. `--rng-initial-seed` defaults to `5489U` and can be set explicitly anyway. [CITED: klee v3.2 lib/Support/RNG.cpp:16-17]
  - `--max-instructions` is a global stop ("Stop execution after this many instructions"), not per path. [CITED: Executor.cpp:343-347]
  - `--external-calls=none` stops KLEE calling host functions natively. KLEE notes that printf and puts are still allowed. [CITED: Executor.cpp:194-206]
  - `--max-memory` (default 2000 MB) refuses forks above the cap. That makes exploration depend on actual memory use, so set it well above expected use and treat any "skipping fork (memory cap exceeded)" warning as "not proven". [CITED: Executor.cpp:361-373, 883-884]
- **Completion predicate:** parse `KLEE: done: completed paths = N` and `KLEE: done: partially completed paths = M` from the `info` file or stderr. [CITED: tools/klee/main.cpp:1752-1758] `M == 0`, no `HaltTimer invoked` (Executor.cpp:494), and no memory-cap warning together mean `completed`. Anything else is `timeout` ("not proven").
- **`.ktest` format** (big-endian u32): `"KTEST"`, version (3), `numArgs` + strings (u32 length + bytes), `symArgvs`, `symArgvLen`, `numObjects`, then per object a name string, `numBytes` and the bytes. [CITED: klee v3.2 lib/Basic/KTest.cpp kTest_toFile] Parse it in Rust with a size cap, and turn `in`/`len`/scalars into a request file.

### Pattern 6: UB-log schema (sketch; field names are discretion)

```toml
schema_version = 1
target = "cjson"

[[target_facts]]              # one per config (D-33)
config = "default"
char_signed = false
sizeof = { short = 2, int = 4, long = 4, long_long = 8, pointer = 4, size_t = 4 }
enum_min_size = 1
macro_set_sha256 = "..."
structs = { "cJSON" = "<layout_summary text>" }
approver = ""                 # empty = unsigned
date = ""

[[entry]]
function = "cJSON_ParseWithLength"
kind = "asan:heap-buffer-overflow"     # source:kind
location = "cJSON.c:312:17"            # or input-class:<sha256> (D-31)
status = "observed"                    # observed | stale | klee-unconfirmed
abi = "i386-ilp32"                     # i386-ilp32 | x86_64-lp64 | arm-target
configs = ["default"]
witnesses = ["<sha256>", "..."]        # sorted
stale_run = ""
action = "return_error"                # proposed (D-28)
error_code_proposal = "-1"             # D-34
rationale = ""
approver = ""
date = ""
```

Cross-check `Policy` against the log. `Policy` and `Override` derive `Deserialize` only (policy.rs:41, 118), so the log needs its own `Serialize` types. `Site { function, kind, location }` is the existing key type (policy.rs:136-147). [VERIFIED: crates/mt-contract/src/policy.rs read this session]

### Anti-Patterns to Avoid

- **Treating a clean compile of `-fsanitize=memory -m32` as MSan support.** It links nowhere (Q1).
- **Taking the site from `SUMMARY:`.** It can name an interceptor frame.
- **Committing raw sanitizer or KLEE output.** Pids, addresses and timing break determinism.
- **Reusing the target `runtime.c` on the host.** It uses raw ARM `svc` calls and its own heap arena.
- **Using `ktest-tool`.** It is Python, and the final image has no python3.
- **Letting `-Werror` through to host clang.** Different warnings would turn into spurious `sanitizer: unavailable` drops.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Struct layouts | A sizeof/offset calculator | `mt-harness::layout` over `llvm-dwarfdump` | The compiler already decided bitfields and enum widths |
| Type sizes and char sign | Guesses per target | `-dM -E` predefined macros | They follow the actual flags |
| Detecting UB | Custom bounds checks in the driver | ASan, UBSan and MSan runtimes, KLEE | Coverage and precision |
| Symbolization | addr2line parsing | `llvm-symbolizer` via `external_symbolizer_path` | Gives columns and inline frames |
| Killing escapees | A new process-tree walker | Extend `kill_tree`'s pidfd and start-time pattern | Already reviewed against pid reuse |
| Vector re-hash and index | Ad-hoc JSON writes | `mt_vectors::corpus::write_corpus` | It validates provenance against each vector |

## Common Pitfalls

### Pitfall 1: The host ILP32 ABI is not the AAPCS ABI
**What goes wrong:** host `-m32` places `long long`/`double` at 4-byte alignment and uses 4-byte enums, so states and globals are laid out differently from the target. Findings then come from a different memory picture.
**How to avoid:** derive `-malign-double -fshort-enums -mlong-double-64` from the facts. Verify the host DWARF layout against the target for every interface struct. On a mismatch, record `unavailable` with a cause. Remaining known differences: `wchar_t` signedness and bitfield container rules.
**Warning signs:** a host-only finding near struct fields; a `layout_summary` that differs between host and target.

### Pitfall 2: Flags that change the host ABI against glibc
`-fshort-enums` and `-malign-double` change the ABI for any libc structure containing enums or doubles. Parsers rarely pass such structures to libc, but a pair that does should fail the DWARF check or show as a host artifact. Record the flags in provenance.

### Pitfall 3: High-entropy ASLR breaks LLVM 16 sanitizers on newer kernels
GitHub's ubuntu-24.04 runner kernels have shipped `vm.mmap_rnd_bits=32`, which makes ASan and TSan abort at startup with older LLVM. MSan has a separate personality-related failure. The common fix is `sysctl vm.mmap_rnd_bits=28` on the host. [CITED: github.com/ms609/TreeTools/pull/270; mirror.b10c.me/bitcoin-bitcoin/30674; inbox.dpdk.org patch 20240312145326.1377818-1-aconole@redhat.com] Inside `docker run --cap-drop ALL` the container cannot change it, and default seccomp likely blocks `personality(ADDR_NO_RANDOMIZE)`. [ASSUMED] **How to avoid:** add a host step before `docker run` in the `ub` job that sets 28 and records `/proc/sys/vm/mmap_rnd_bits` and `mmap_rnd_compat_bits` in a notice. Make the fixtures self-test the canary. This sandbox reads 28 and 8. [VERIFIED: this session]

### Pitfall 4: LeakSanitizer in a capability-dropped container
LSan needs ptrace, and CI drops all capabilities. Always set `detect_leaks=0`; a leak is not UB anyway.

### Pitfall 5: A forged sanitizer report from untrusted C
The C under test shares the process and can print text that looks like `SUMMARY: AddressSanitizer: ...`. **How to avoid:** use `log_path` files and require the exit code to equal the variant's configured `exitcode`. Fail closed: a report without the exit code, or the exit code without a parseable report, becomes `kind = report-malformed` and gets an entry that needs a decision. Forging can only add a label, which excludes the vector and forces a decision, so it is the safe direction.

### Pitfall 6: KLEE sites have no column
D-32's key is `file:line:col` for dynamic findings, but KLEE's `.err` has `File:` and `Line:` only. A KLEE entry (`klee:ptr`, `cJSON.c:312`) never equals the ASan entry for the same bug (`asan:heap-buffer-overflow`, `cJSON.c:312:17`). That is acceptable because the kinds differ anyway. **Recommendation:** KLEE entries use `file:line`. The witness replay (D-22) produces the sanitizer entry. The KLEE entry gets `status = "confirmed"` with a `confirmed_by` reference to the sanitizer site key, or `klee-unconfirmed` (D-23). The planner should make this an explicit schema rule.

### Pitfall 7: Paths in the site key
Absolute build paths differ between local and CI work directories. Use `strip_path_prefix=<pair source root>/` and the equivalent normalisation for KLEE `File:`, so locations are relative to the target's source tree. This keeps the log deterministic across runs.

### Pitfall 8: Spurious optdiffs
- `-O0` code is slower, so a vector that finishes at `-Os` may time out at `-O0`. Run `-O0` with an inflated budget (for example 4x, discretion). A disagreement whose only difference is a timeout is labelled `timeout`, not `optdiff`.
- Raw addresses differ between builds. The compare must use the classified forms (`{buffer, offset}`, `{symbol}`, unobserved), which `mt-vectors::compare` already uses.
- A pair with no `-O` in its shipped flags gets `optdiff: not_applicable`.

### Pitfall 9: KLEE nondeterminism sources
Memory-cap fork skipping, solver timeouts (`--max-solver-time`) and wall-clock halts all make two runs differ. Use a large `--max-memory`, `--max-solver-time=0`, and wall time only as a safety cap recorded as `timeout`. Then the D-20 double-run comparison is meaningful. Without `--emit-all-errors`, KLEE reports each (instruction, message) once, which fits site-keyed logging.

### Pitfall 10: An existing test pins the WR-13 limit
`tool_01_runner_marks_a_stream_held_by_an_orphaned_escapee_incomplete` asserts the old behaviour. Change it RED-first in the WR-13 plan. Do not delete it.

### Pitfall 11: The image's KLEE has no 32-bit runtime
Any plan step that builds `-m32` bitcode for KLEE will fail or silently mis-link (Q2).

## Code Examples

### Pipe-holder lookup (safe std; the prototype that ran this session, trimmed)
```rust
// Source: scratch prototype, this session (#![forbid(unsafe_code)], std only)
fn pipe_id(fd: i32) -> std::io::Result<String> {
    Ok(std::fs::read_link(format!("/proc/self/fd/{fd}"))?.to_string_lossy().into_owned()) // "pipe:[7455]"
}
fn holders(id: &str) -> Vec<i32> {
    let me = std::process::id() as i32;
    let mut out = Vec::new();
    let Ok(procs) = std::fs::read_dir("/proc") else { return out };
    for e in procs.flatten() {
        let Some(pid) = e.file_name().to_str().and_then(|n| n.parse::<i32>().ok()) else { continue };
        if pid == me { continue }
        let Ok(fds) = std::fs::read_dir(format!("/proc/{pid}/fd")) else { continue };
        if fds.flatten().any(|fd| std::fs::read_link(fd.path()).is_ok_and(|l| l.to_string_lossy() == id)) {
            out.push(pid);
        }
    }
    out
}
// then: rustix::process::pidfd_open + re-check that the pid still holds `id` + pidfd_send_signal(STOP/KILL)
```

### Report parsing (shape)
```rust
// SUMMARY line: "SUMMARY: AddressSanitizer: heap-buffer-overflow cJSON.c:312:17 in parse_string"
// kind = token after "SUMMARY: <Tool>: "; tool -> source: AddressSanitizer=asan,
// UndefinedBehaviorSanitizer=ubsan, MemorySanitizer=msan
// frames: lines matching r"^\s+#(\d+) 0x[0-9a-f]+ in (\S+) (\S+?):(\d+)(?::(\d+))?$" in the FIRST block
// site = first frame whose file is under the pair's source tree and is not the driver file
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `-fsanitize=undefined-trap` | `-fsanitize-trap=undefined` (`undefined-trap` is a deprecated alias of `-fsanitize=undefined`) | clang docs 16 | For GCC (D-04), `-fsanitize-undefined-trap-on-error` is still the GCC spelling [ASSUMED for GCC 14]; confirm in the pinned image |
| KLEE `.err` file types fixed per check | KLEE 3.x `TerminationTypes.h` with the UBSan runtime mapping to `overflow`/`ptr`/`div` | KLEE 3.0+ | The kind tokens are the file suffixes |
| Debian `libclang-common-N-dev` held the runtimes | `libclang-rt-N-dev` | Debian LLVM 15+ packaging [ASSUMED] | Pin the new package name |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Bookworm package names `libclang-rt-16-dev`, `libc6-dev-i386`, `lib32gcc-12-dev`, and their file layout, match the Ubuntu noble proxy | Q3 | The image plan needs another discovery round; low cost, caught by pin discovery |
| A2 | `M32_SUPPORTED` is 0 in the current `klee-build` stage | Q2 | If 1, KLEE i386 runtimes exist, but uclibc is still 64-bit, so the fallback stands |
| A3 | `llvm-symbolizer` is at `/usr/lib/llvm-16/bin/llvm-symbolizer` in the image | Q5 | Unsymbolized frames; D-32 sites cannot be keyed. Check with `ls` in the image plan |
| A4 | Docker default seccomp blocks `personality(ADDR_NO_RANDOMIZE)`, so the host sysctl is needed | Pitfall 3 | Low; the host step is harmless either way |
| A5 | Exit codes 86, 87, 88; `-O1` for sanitized builds; a 4x `-O0` budget; the KLEE `-fsanitize` subset | Q5, Patterns | Tuning only (discretion) |
| A6 | GCC 14 accepts `-fsanitize=undefined -fsanitize-undefined-trap-on-error` for arm-none-eabi with no runtime | D-04 | The trap variant cannot build; it is first on the cut list |
| A7 | clang 16 accepts `-mlong-double-64` on i386 (measured on clang 18 only) | Pattern 1 | Drop that flag and record `long double` as a known host difference |
| A8 | The GCC-libsanitizer report shapes equal those of clang-16 compiler-rt (plus columns) | Q5 | Parser tweaks; capture real report fixtures from CI first (Wave 0) |
| A9 | The `/proc/<pid>/fd` readlink works for same-uid processes under `--cap-drop ALL --security-opt no-new-privileges` | Q4 | If it fails, WR-13 holders are not found; the test catches this in CI |

## Open Questions

1. **The KLEE site key and confirmation link (Pitfall 6).**
   - What we know: KLEE gives `file:line` only, and D-32 specifies columns for dynamic findings.
   - What's unclear: whether the founder wants KLEE entries merged into sanitizer entries on confirmation, or kept as separate entries.
   - Recommendation: keep them separate, and add a `confirmed_by` link as a schema field. Surface this at plan check.
2. **Pairs whose host DWARF layout cannot match the target.**
   - Recommendation: `sanitizer: unavailable`, cause `host_layout_mismatch`, as in the D-06 pattern. The founder decides per pair in the log.
3. **Whether `ub_checked` provenance needs an index `schema_version` bump** (Phase 5 reads it).
   - Recommendation: yes. Bump it, with a contract changelog line, together with the D-10 grammar change.
4. **Real clang-16 report fixtures.**
   - Recommendation: the image plan's smoke step publishes the reports of the known-UB fixtures as a CI notice or artifact. Parser tests then use those bytes, the way the 03 `vectors-selftest-dwarf` notice was used.

## Environment Availability

| Dependency | Required By | Available here | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Pinned image (Docker) | All runtime checks | ✗ (daemon not responding) | — | CI container round trip (D-27) |
| clang-16 + libclang-rt-16-dev | UB-01, KLEE bitcode | ✗ (host clang 18.1.3, no runtimes) | — | Proxy measurements only; real runs in CI |
| i386 libc dev | UB-01 | ✗ (`dpkg` install failed: disk full, 25 MB free) | — | CI image |
| KLEE 3.2 | UB-03 | ✗ | — | CI image |
| arm-none-eabi-gcc | UB-02, UB-04 | ✓ (13.2.1, proxy for 14.3.Rel1) | 13.2.1 | — |
| qemu-arm | UB-02 | ✓ (host build) | — | — |
| cargo | All tool code | ✓ | 1.99 | — |
| Debian package sites | Q3 | ✗ (not allowed in this session's network settings) | — | Ubuntu archive proxy; pin-discovery workflow |

**Missing dependencies with no fallback:** none for planning. Every runtime proof moves to CI, as in Phases 2-3.
**Note for the founder:** this sandbox's root filesystem is full (25 MB free). An attempted install of local i386 runtimes left `dpkg` partly unpacked; `dpkg --configure -a` was run and `dpkg --audit` is now clean. `packages.debian.org`, `sources.debian.org` and `snapshot.debian.org` would have to be allowed in the session's network settings to check bookworm package contents directly.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | cargo test (Rust 1.99.0, edition 2024), integration tests under `crates/*/tests/`, `#[ignore]` for tests that need the pinned tools (the `eqv_04_exec.rs` pattern with `MT_LOCAL_CAPTURES`) |
| Config file | `Cargo.toml` workspace; `clippy.toml`; `.github/workflows/testbed.yml` (new `ub` job) |
| Quick run command | `cargo test -p mt-ub -p mt-contract -p mt-toolchain` |
| Full suite command | `cargo test --workspace` (plus `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo deny check`) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| (D-24) | Pipe holder killed after a normal exit; orphan-escapee test now complete | unit/integration | `cargo test -p mt-toolchain tool_01_runner_kills_a_pipe_holder` | ❌ Wave 0 (rewrite of existing test) |
| UB-01 | Report parser: kind and innermost target frame from ASan/UBSan/MSan text | unit | `cargo test -p mt-ub ub_01_parse` | ❌ Wave 0 (needs CI-captured fixtures) |
| UB-01 | Host flag map is exhaustive over `ALLOWED_OPTIONS`; fact-derived flags | unit | `cargo test -p mt-ub ub_01_host_flags` | ❌ |
| UB-01 | Known-UB fixtures flagged with the expected `source:kind`; clean inputs not flagged | CI self-test (`#[ignore]` locally) | `mt ub selftest` in the `ub` job; `cargo test -p mt-ub ub_01_fixtures -- --ignored` | ❌ |
| UB-01 | MSan LP64 note recorded on every MSan finding | unit | `cargo test -p mt-ub ub_01_msan_lp64` | ❌ |
| UB-02 | `-O0` arguments derived from the canonical command; `shipped_is_O0` path | unit | `cargo test -p mt-ub ub_02_optlevel_arguments` | ❌ |
| UB-02 | Optimiser-sensitive fixture labelled `optdiff`; a timeout-only difference is not optdiff | CI | `ub` job notice | ❌ |
| UB-03 | `.err`/`.ktest` parsing; completion predicate; floor ≥ 3 targets | unit + CI | `cargo test -p mt-ub ub_03_` | ❌ |
| UB-04 | `-dM -E` parse; facts block compared to fresh extraction; a change unsigns it | unit | `cargo test -p mt-ub ub_04_facts` | ❌ |
| UB-05 | Strict TOML (unknown key refused); decided = all four fields; merge never deletes; stale marking; bulk sign never touches facts or non-default actions; policy/log cross-check; `mt ub check` exit codes | unit + CLI | `cargo test -p mt-contract ub_05_ && cargo test -p mt-cli ub_05_` | ❌ |
| (D-10) | Label grammar and source order documented; contract version bumped | doc test | `cargo test -p mt-contract eqv_01_contract_document` (extend) | ✅ (extend) |

### Sampling Rate
- **Per task commit:** the quick run command
- **Per wave merge:** `cargo test --workspace` plus clippy, fmt and deny
- **Phase gate:** full suite green, `ub` job green on the new image digest, and the `mt ub check` notice reviewed before `/gsd-verify-work 4`

### Wave 0 Gaps
- [ ] `crates/mt-ub/` crate skeleton (`#![forbid(unsafe_code)]`, lints workspace) added to `members`
- [ ] `crates/mt-ub/fixtures/` D-08 C samples, each with triggering and clean inputs and manifest-role sidecars
- [ ] Captured clang-16 report texts (ASan i386, UBSan i386, MSan x86-64) and one KLEE `.err`/`.ktest` from the image smoke step, committed as test fixtures
- [ ] Rewrite of `tool_01_runner_marks_a_stream_held_by_an_orphaned_escapee_incomplete`

## Security Domain

`security_enforcement` is not set to false in `.planning/config.json`, so this section applies.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no (single founder; approver is free text by D-35 design) | Git history as the audit trail |
| V3 Session Management | no | — |
| V4 Access Control | partly | CI hardening: `--network none --user 65534 --cap-drop ALL --read-only no-new-privileges` for every run of captured code, KLEE included |
| V5 Input Validation | yes | Strict TOML (`deny_unknown_fields`), bounded report and `.ktest` parsing (runner output cap, explicit size caps), closed label grammar (`HasUbLabel::new`) |
| V6 Cryptography | yes (integrity only) | sha256 via the existing `sha2` crate for vector names and the facts macro-set digest |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Untrusted C forges or suppresses sanitizer reports | Spoofing / Tampering | `log_path` files plus a required exitcode match; fail closed to `report-malformed` entries (Pitfall 5) |
| KLEE calls host functions natively on behalf of untrusted code | Elevation | `--external-calls=none`; no-network container |
| The WR-13 kill hits an unrelated process | Tampering / DoS | Match by this run's pipe inode only; pidfd plus re-check before the signal; same uid only |
| Bulk signing hides non-default actions or facts | Repudiation | D-35: bulk only for `return_error` entries, one record per entry; facts never bulk-signed; a test pins this |
| Path traversal through report `file` fields written into the log | Tampering | Treat locations as text; strip the prefix; never open a path read from a report |
| A changed toolchain silently changes facts | Tampering | Macro-set sha256 in the facts block; a fresh extraction must equal the signed one (D-33) |

## Sources

### Primary (HIGH confidence)
- llvm-project `llvmorg-16.0.6`: `compiler-rt/cmake/Modules/AllSupportedArchDefs.cmake`, `compiler-rt/lib/msan/msan.h`, `compiler-rt/lib/msan/msan.cpp`, `compiler-rt/lib/ubsan/ubsan_flags.inc`, `compiler-rt/lib/ubsan/ubsan_checks.inc`, `compiler-rt/lib/sanitizer_common/sanitizer_flags.inc`, `llvm/lib/Transforms/Instrumentation/MemorySanitizer.cpp`, `clang/lib/Driver/ToolChains/Linux.cpp`, `clang/docs/MemorySanitizer.rst`, `clang/docs/UndefinedBehaviorSanitizer.rst`
- klee `v3.2`: `tools/klee/main.cpp`, `runtime/CMakeLists.txt`, `CMakeLists.txt`, `include/klee/Core/TerminationTypes.h`, `lib/Core/Executor.cpp`, `lib/Core/UserSearcher.cpp`, `lib/Support/RNG.cpp`, `lib/Basic/KTest.cpp`, `runtime/Sanitizer/ubsan/ubsan_handlers.cpp`, `runtime/Intrinsic/klee_div_zero_check.c`, `runtime/Intrinsic/klee_overshift_check.c`
- In-repo, read this session: `container/pins.toml`, `container/Dockerfile`, `crates/mt-toolchain/src/runner.rs`, `crates/mt-toolchain/tests/tool_01_runner.rs`, `crates/mt-capture/src/cc.rs`, `crates/mt-harness/src/{probe.rs,layout.rs,runtime.c,run.rs}`, `crates/mt-contract/src/policy.rs`, `crates/mt-vectors/src/{vector.rs,corpus.rs}`, `Cargo.toml`, `clippy.toml`, `.github/workflows/testbed.yml`, `docs/guidelines/building-the-tool.md`
- Measurements this session: Ubuntu noble `libclang-rt-16-dev 1:16.0.6-23ubuntu4` `dpkg -c`; clang 18 `-m32` MSan and ASan link behaviour; GCC 13 libsanitizer report formats; arm-none-eabi-gcc 13.2.1 `-dM -E`; layout probe across arm, i386 and x86-64; WR-13 prototype

### Secondary (MEDIUM confidence)
- [TreeTools PR #270: Lower ASLR entropy so ASan can map its shadow region](https://github.com/ms609/TreeTools/pull/270), [TreeTools PR #271](https://github.com/ms609/TreeTools/pull/271)
- [Bitcoin Core #30674: TSAN/MSAN fails with vm.mmap_rnd_bits=32](https://mirror.b10c.me/bitcoin-bitcoin/30674/)
- [DPDK patch on sanitizer ASLR](https://inbox.dpdk.org/dev/20240312145326.1377818-1-aconole@redhat.com)
- [Ubuntu kernel bug 2056762 comment](https://bugs.launchpad.net/ubuntu/+source/linux/+bug/2056762/comments/5)

### Tertiary (LOW confidence)
- Debian bookworm package names (not reachable this session; Ubuntu proxy only)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH for tools and versions (pinned); MEDIUM for the apt package names (proxy)
- Architecture: HIGH (in-repo seams read; crate plan from the guidelines)
- A-08 and D-18 answers: HIGH (A-08) and MEDIUM-HIGH (D-18), both from pinned-tag source; D-18 needs one image probe
- Report formats: MEDIUM-HIGH (source plus a GCC libsanitizer proxy run; capture clang-16 fixtures in Wave 0)
- Pitfalls: MEDIUM-HIGH (ABI layout measured; the ASLR issue cited from community sources)

**Research date:** 2026-11-06
**Valid until:** 2026-12-06 (pinned toolchain; changes only with a pin bump)
