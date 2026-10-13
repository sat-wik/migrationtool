# Phase 2: Test Bed and Build Capture - Context

**Gathered:** 2026-10-12
**Status:** Ready for planning

<domain>
## Phase Boundary

This phase delivers a pinned test bed of about 16 permissive-licence open-source C parser and protocol libraries, declared in `testbed/manifest.toml`. For every target and each of its declared configurations, the tool:

- fetches the sources by commit inside the pinned container;
- runs the upstream arm-none-eabi GCC build under capture to write `compile_commands.json`;
- replays the captured commands in a clean tree with no network;
- shows that every object matches the original, byte for byte after a documented normalisation;
- extracts a per-target seed corpus.

Targets that cannot be reproduced are dropped with the cause logged, and at least 10 must remain. Requirements: BED-01, BED-02, BUILD-01, BUILD-02, BUILD-03. Success criteria 1-4 are in `.planning/ROADMAP.md` Phase 2.

Not in this phase:
- the equivalence contract and test-vector schema (Phase 3);
- sanitizer and UB builds (Phase 4);
- the ILP32 differential harness and emulator replay (Phase 5);
- the held-out corpus split and its isolation (GATE-05, Phase 7);
- IAR and Keil project parsing, and Zephyr `west` builds (deferred, see the spec non-goals);
- sandboxing of customer builds (Phases 5 and 8).

</domain>

<decisions>
## Implementation Decisions

Decision IDs are phase-local (D-01..). Phase 1 decisions are cited as "P1 D-xx" and project decisions as "PROJECT D-xx".

### Target list and licences
- **D-01:** The initial manifest holds about **16 targets**. Up to 6 can then drop and BUILD-02 still keeps at least 10.
- **D-02:** **Permissive licences only**: MIT, BSD-2/3-Clause, Apache-2.0, Zlib, ISC and equivalents. Each target records its SPDX identifier and the path of its licence file at the pinned commit. GPL, LGPL and other copyleft targets are excluded, so that later Rust translations and benchmark results can be published. — **Reversibility:** costly — adding copyleft targets later means a licence review of every published translation and benchmark artifact derived from them.
- **D-03:** **The researcher proposes the concrete list, and the founder approves it before any capture work.** The plan must include a `checkpoint:decision` that presents the proposed targets: name, repo, commit, licence, category, entry functions and why each was chosen. Candidates include the spec's examples (zcbor, tinycbor, nanopb, lwIP protocol parsers, Mbed TLS ASN.1). Backup candidates may be listed, but are not captured unless approved.
- **D-04:** **Cover every beachhead category** at least once: TLV, CBOR, MQTT, BLE (for example the GATT/ATT or advertising-data parsers), OTA image headers (for example MCUboot image header/TLV parsing), and ASN.1/X.509. Fill the rest with general formats such as JSON and protobuf.

### Configurations and RTOS
- **D-05:** A "shipped configuration" is the **upstream default plus at most one realistic variant** per target, for example a feature macro or size option that firmware commonly sets. Configurations are declared in the manifest as macro and flag sets (BED-02). The variant should change compiled code, not only names.
- **D-06:** All configurations target a **Cortex-M4F baseline**: `-mcpu=cortex-m4 -mthumb -mfloat-abi=hard -mfpu=fpv4-sp-d16 -Os`. This matches `thumbv7em-none-eabihf`, the Rust target pinned in Phase 1. The compiler is the container's Arm GNU 14.3.rel1 `arm-none-eabi-gcc`. Per-configuration macros and flags are added on top.
- **D-07:** The "original" build is driven by **each project's own build system**: Make or CMake with an arm-none-eabi toolchain file or a `CC`/`CFLAGS` override. Make builds are captured with Bear and CMake builds through `CMAKE_EXPORT_COMPILE_COMMANDS` (BUILD-01). Only the parser library and its translation units are built; no RTOS, application or link step for a full firmware image.
- **D-08:** **The first RTOS ecosystem is recorded as "deferred; parsers are RTOS-independent"** (BED-02). It will be revisited from the design-partner interviews (weeks 4-10). The record names Zephyr (industrial IoT) and FreeRTOS (consumer IoT) as the candidates. — **Reversibility:** reversible — a later decision entry in PROJECT.md settles it, and no Phase 2 code depends on the choice.

### Reproduction bar
- **D-09:** BUILD-02 compares **every object file (`.o`)** of the replay against the original build. The goal is byte-identical. Only a **fixed, documented normalisation** is allowed:
  - path remapping (`-ffile-prefix-map`/`-fdebug-prefix-map` added identically to both builds, or applied by the capture);
  - stripping `.comment` and debug sections, and only when they are the sole differing sections.
  The normalisation is applied the same way to every target and recorded in each report. Any other difference fails the configuration.
- **D-10:** The original build runs **inside the pinned container, under capture, once**. The captured `compile_commands.json` is then **replayed in a clean copy of the source tree in the same container**. Nothing is built on the host toolchain. The container is referenced by digest (P1 D-24); the current verified image is `ghcr.io/sat-wik/migrationtool-toolchain@sha256:42c7fb0e00e90e946b5a21e598517fb7189a0de5492fa1d4d6f18e2e9c39e13d`, published from `main` by container run 38202499776 (manifest sha256 `d70029186639d0030fc6cf22b284b3b849ce0680656fecfe06623f6ffdc44142`, the same as the branch-built `sha256:0f5a5d2594b0405931ad7e85d5c6d2f5e3f2d5099d769059870817ee50e152ee` on which the Phase 1 founder check passed), or a later one recorded under `## Current image` in `container/README.md`.
- **D-11:** **Drop policy:** one documented fix attempt per failing target or configuration, such as adding a prefix map or a missing define. If it still differs, the target is dropped. The cause, the differing objects and sections, and the fix tried go into a committed drop log, for example `testbed/DROPPED.md` plus a machine-readable entry. The phase fails if fewer than 10 targets remain.
- **D-12:** **Sources are fetched online, and builds run offline.** The fetch step is the only one with network access. Capture and replay run with `docker run --network none`, which shows the build has no hidden downloads. Phase 1 used the same principle for its offline installs, and it is consistent with P1 D-19, which deferred isolation for customer builds to later phases.

### Seed corpus
- **D-13:** Seeds come from each target's **tests, fixture files and upstream fuzz corpora**, for example OSS-Fuzz seed corpora committed in the upstream repo. Each seed records its source path, the commit, and how it was extracted, for example a file copy or a byte array lifted from test source. Only a target with no usable inputs gets **at least 10 hand-made seeds**, marked "proposed" (BUILD-03).
- **D-14:** **The GATE-05 held-out split is deferred to Phase 7.** Phase 2 extracts the full seed corpus. The held-out store, the split and the sandbox negative test are designed once, in GATE-05. Nothing in Phase 2 may assume the seeds are hidden from agents.
- **D-15:** Seed format: **one raw input file per seed, named by its sha256**, plus a **per-target index** (TOML or JSON, deterministic, with `schema_version`). The index records the entry function each seed targets, its source and its provenance. Phase 3's TRACTOR vectors can wrap these later; Phase 2 does not adopt the TRACTOR schema.
- **D-16:** Seeds are **committed under `testbed/`**, with each target's licence noted. Per-target count and size caps are applied (the researcher proposes values, for example at most 200 seeds and 1 MiB per target). The cap and the selection rule must be deterministic.

### Source fetching and cache
- **D-17:** Targets are pinned by **git commit SHA plus a sha256 of the checked-out tree**, excluding `.git`. The fetch is a shallow fetch of that commit, run through the `mt-toolchain` runner. On reuse the tool checks both the commit and the tree hash. This is the same pinning style as Phase 1's KLEE and tinycbor sources.
- **D-18:** Fetched sources and build trees live in a **git-ignored work directory**: `.mt-work/` by default, overridable with `$MT_WORK_DIR`. Only the manifest, drop log, seeds and small reports are committed. Fetches are idempotent and verified on reuse.
- **D-19:** **The fetch runs inside the pinned container** using its git, in a networked `docker run`. Capture and replay then run in the same image with `--network none`.

### Manifest and CLI
- **D-20:** There is **one `testbed/manifest.toml`**, validated like `pins.toml`. It has `schema_version` and `deny_unknown_fields`, with one table per target holding:
  - id, repo URL and commit;
  - tree sha256, licence SPDX ID and licence file;
  - category and build system (`make` or `cmake`);
  - entry functions;
  - configurations, as named macro and flag sets.
  Validation tests reject unknown keys, non-permissive licences, missing entry functions and duplicate ids.
- **D-21:** The CLI is **`mt testbed fetch|capture|replay|seeds`**. Each subcommand takes a target id or `--all` and writes deterministic JSON reports. Capture and replay logic lives in the planned **`mt-capture`** crate (`docs/guidelines/building-the-tool.md` §3). Manifest and seed handling sit beside it or in a small testbed module, at the planner's choice. `mt-cli` only parses arguments. Every external tool (git, make, cmake, bear, arm-none-eabi-gcc, objcopy/readelf for normalisation) runs through the `mt-toolchain` runner (P1 D-16, D-17).
- **D-22:** Each capture and replay run writes a **report per target and configuration** containing:
  - the runner's run records (argv, effective env, tool version, exit status, output hashes);
  - the sha256 of `compile_commands.json`;
  - the number of translation units;
  - a sha256 for every original and replayed object, before and after normalisation;
  - the differing objects and sections.
  Reports are deterministic (`BTreeMap`, `schema_version`, timestamps only in a header) and feed the Phase 11 evidence bundle.

### Entry functions
- **D-23:** Entry functions are **named by hand in the manifest and checked by the tool**. After capture, the tool confirms each named function exists, with its declared signature, in a header the captured build includes. It does this by parsing with clang through the runner, for example `clang -Xclang -ast-dump=json` with the captured flags. A missing or mismatched entry fails validation.
- **D-24:** A **buffer-in** entry takes a pointer and a length (`const uint8_t *`/`const char *`/`const void *` or an equivalent typedef, plus `size_t` or an integer length). It may also take a context, state or output struct pointer. This covers zcbor, tinycbor, nanopb and TLV-style APIs. Each target records 1-3 entry functions.

### CI scope and offline tests
- **D-25:** A **new `testbed` GitHub Actions workflow** pulls the verified image by digest and runs the full fetch, capture and replay for every target. It runs **weekly on a schedule and on pushes that change `testbed/` or `crates/mt-capture/`**, plus `workflow_dispatch`. Results are published as job annotations, since cloud sessions have no Docker. Actions are SHA-pinned with `contents: read`, as in Phase 1.
- **D-26:** Ordinary pushes run **`cargo test` on recorded fixtures plus a tiny local C project**:
  - committed small `compile_commands.json` and report fixtures captured from CI;
  - a 2-file C project the host's own `cc` builds, so replay and comparison logic is tested anywhere, with no Docker or network.
  This follows the Phase 1 version-fixture pattern. Requirement-ID test names apply, for example `build_02_replay_matches_original_objects`.

### Decisions after research (founder, 2026-10-12)
These answer the research's open questions 1, 2, 3 and 6 (`02-RESEARCH.md` § Open Questions). They refine D-02, D-04, D-24 and the sequencing of D-10/D-25. The D-03 approval checkpoint over the concrete target list still applies.
- **D-27:** **Dual-licensed targets are accepted when a permissive option is elected.** The manifest records the full SPDX expression (`spdx`) and an `elected` identifier. The validator accepts an `OR` expression only when `elected` is one of its operands and is on the permissive allow-list (D-02); a single copyleft licence is still rejected. This keeps Mbed TLS (elected Apache-2.0), Paho Embedded (elected BSD-3-Clause / EDL-1.0) and libfdt (elected BSD-2-Clause). Record the reading as a PROJECT.md decision. — **Reversibility:** costly — reversing it later drops three targets and their ASN.1/X.509 and OTA coverage, and every published artifact derived from them would need review.
- **D-28:** **Struct-wrapped buffer entries are accepted, flagged explicitly** (refines D-24). An entry whose buffer and length arrive inside a struct argument (QCBOR `UsefulBufC`, coreMQTT `MQTTPacketInfo_t`, nanopb `pb_istream_t` plus descriptor, zlib `z_stream`) is allowed only with `accepted_wrapped = true` on that entry; the validator rejects a wrapped form without the flag. Later harnesses construct the struct from the input buffer.
- **D-29:** **TLV and OTA image header are covered by the research's stand-ins** (refines D-04). libfdt stands in for the OTA image header category (a FIT image is a devicetree blob; `fdt_check_full` validates header and structure). TLV is covered structurally by Mbed TLS DER, NimBLE advertising data and microcoap options, tagged `tlv` in the manifest's categories. MCUboot is not added in this phase (it would need a testbed-authored port, departing from D-07); it is a deferred idea.
- **D-30:** **The image extension ships first, as its own PR to `main`** (sequencing for D-10/D-19/D-25). The pinned image gains `git`, `cmake`, `curl` and `bzip2`, pinned the Phase 1 way (snapshot apt, `[apt]`/`[tool.*]` pins, `REQUIRED_TOOLS`, `mt toolchain check` rows, static tests, README `## Current image`). Because only pushes to `main` publish (WR-09, which stays in force), that change is merged to `main` as a separate PR before any plan that needs the new digest runs in CI. Offline work (manifest, tree hash, capture/replay core, fixture tests) proceeds in parallel; the CI tracer and the full-bed run wait for the new main-published digest.

### Claude's Discretion
- The exact target list, commits, entry functions and the one variant per target. The researcher proposes them and the founder approves (D-03).
- Seed caps (D-16), the index format (TOML or JSON), and the drop-log layout (D-11).
- Crate split between `mt-capture` and a testbed module (D-21), and the CMake toolchain-file details (D-07).
- Exact normalisation commands (`arm-none-eabi-objcopy --remove-section`, prefix-map flags), within the bounds of D-09.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase scope and requirements
- `.planning/ROADMAP.md`: Phase 2 goal and success criteria 1-4.
- `.planning/REQUIREMENTS.md`: the wording of BED-01, BED-02, BUILD-01, BUILD-02 and BUILD-03, and the spec mapping (`REQ-ingest-build-capture`, `REQ-embedded-build-handling`).
- `.planning/PROJECT.md`: Constraints (Scope: C only, GCC and Cortex-M first, parsers only), the open spec question on the first RTOS, and the day-90 bar (70% of test-bed parsers reach full evidence).

### Product spec and tool rules
- `docs/specs/product-spec.md`: the weeks 1-2 test-bed plan, with example targets; ingest and build capture ("rebuild the exact firmware before touching anything"); and "validate only shipped configurations".
- `docs/guidelines/building-the-tool.md`:
  - §3 workspace layout (`mt-capture`, `testbed/`: sources fetched, not vendored);
  - §4 runner, determinism, licence and testing rules;
  - Bear is GPL and runs only as a subprocess.
- `docs/adr/0001-tool-implementation-language.md`: analysers run as pinned subprocesses, and only research code may use Python (LOCKED).
- `.planning/codebase/CONVENTIONS.md`: a summary of the coding rules.

### Phase 1 outputs this phase builds on
- `.planning/phases/01-pinned-toolchain-container/01-CONTEXT.md`: runner environment (D-17), output capture (D-18), network isolation deferred (D-19), CI and GHCR by digest (D-23, D-24).
- `container/README.md`: the current verified image digest and how to run it.
- `container/pins.toml`: the TOML schema and validation style to mirror in `testbed/manifest.toml`.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/mt-toolchain/src/runner.rs`: the only process launcher. It provides the cleared environment plus a fixed base, run records (schema v2) with sha256 of stdout and stderr, a 16 MiB capture cap, the timeout leading to NotProven, the process-tree kill, and `complete: false` on streams that did not drain. Use it for git, make, cmake, bear, gcc, clang, objcopy and readelf.
- `crates/mt-toolchain/src/pins.rs`: TOML schema loading with `deny_unknown_fields` and hex/digest/https validation. It is the model for the manifest loader and validator.
- `crates/mt-toolchain/src/fetch.rs` (`mt toolchain hash`): a hardened curl fetch with a sha256 hash, reusable for any tarball or seed-file hashing.
- `crates/mt-toolchain/src/manifest.rs`: deterministic JSON output pattern (`BTreeMap`, `schema_version`).
- The container already ships Bear 3.1.1, clang 16, arm-none-eabi-gcc 14.3.rel1, cmake and make (verify in research), and git.

### Established Patterns
- Requirement-ID test names (`tool_01_*`) and fixture-based offline tests, such as the recorded CI captures under `crates/mt-toolchain/tests/fixtures/`.
- CI evidence goes into job annotations, since cloud sessions have no Docker. Workflows pin actions by SHA and use `contents: read` with `persist-credentials: false`.
- No `std::process::Command` outside `runner.rs`, enforced by a repository test and a clippy ban.

### Integration Points
- `crates/mt-cli` gains a `testbed` subcommand group.
- New crate `crates/mt-capture`, depending on `mt-toolchain`.
- `testbed/` holds the manifest, seeds, drop log and small fixtures.
- New workflow `.github/workflows/testbed.yml`, using the image digest from `container/README.md`.

</code_context>

<specifics>
## Specific Ideas

- The spec's example targets are zcbor, tinycbor, nanopb, lwIP protocol parsers and the Mbed TLS ASN.1 parser. Beachhead categories still to cover: MQTT (for example Paho embedded MQTTPacket), BLE (for example parsers from NimBLE or Zephyr's Bluetooth host, if permissive and buildable standalone), OTA image headers (for example MCUboot's image TLV parsing), and TLV. Candidates must be confirmed by research, since licences and standalone buildability vary.
- The founder works on an Apple Silicon Mac with Docker Desktop (amd64 emulation is slow). Do not design workflows that require building the test bed locally; CI is the primary place it runs.

</specifics>

<deferred>
## Deferred Ideas

- **Held-out corpus split and isolation:** GATE-05 in Phase 7 (D-14).
- **First RTOS final choice:** after design-partner interviews (D-08).
- **Copyleft (LGPL/GPL) targets:** excluded for now (D-02). Revisit only with a recorded decision and a publishing plan.
- **Additional Cortex-M profiles (M0+/thumbv6m, M33):** a later phase or a customer need.
- **Heuristic entry-function discovery** for customer code, which has no hand-named entries.
- **IAR and Keil project parsing, and Zephyr `west` builds:** spec v1 / milestone v2.0.
- **MCUboot as a literal OTA/TLV target** (D-29): needs a testbed-authored port layer, a departure from D-07; revisit if a design partner needs it.
- **A backup-target swap-in pool** that keeps the bed near 16 after drops. Not chosen; drops simply reduce the count, down to a floor of 10.

</deferred>

---

*Phase: 02-test-bed-and-build-capture*
*Context gathered: 2026-10-12*
