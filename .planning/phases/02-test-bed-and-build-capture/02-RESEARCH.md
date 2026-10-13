# Phase 2: Test Bed and Build Capture - Research

**Researched:** 2026-10-12
**Domain:** pinned open-source C parser test bed, Bear/CMake build capture, byte-exact replay for Cortex-M GCC builds
**Confidence:** MEDIUM. Every build result below comes from a proxy toolchain (xPack GCC 14.2.1, Bear 3.1.3, CMake 3.28.3, clang 18), not from the pinned image (Arm GNU 14.3.rel1, Bear 3.1.1, clang 16). Mechanics are HIGH confidence; "this target compiles in the image" is MEDIUM until the first CI run.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

Decision IDs are phase-local (D-01..). Phase 1 decisions are cited as "P1 D-xx" and project decisions as "PROJECT D-xx".

#### Target list and licences
- **D-01:** The initial manifest holds about **16 targets**. Up to 6 can then drop and BUILD-02 still keeps at least 10.
- **D-02:** **Permissive licences only**: MIT, BSD-2/3-Clause, Apache-2.0, Zlib, ISC and equivalents. Each target records its SPDX identifier and the path of its licence file at the pinned commit. GPL, LGPL and other copyleft targets are excluded, so that later Rust translations and benchmark results can be published. — **Reversibility:** costly — adding copyleft targets later means a licence review of every published translation and benchmark artifact derived from them.
- **D-03:** **The researcher proposes the concrete list, and the founder approves it before any capture work.** The plan must include a `checkpoint:decision` that presents the proposed targets: name, repo, commit, licence, category, entry functions and why each was chosen. Candidates include the spec's examples (zcbor, tinycbor, nanopb, lwIP protocol parsers, Mbed TLS ASN.1). Backup candidates may be listed, but are not captured unless approved.
- **D-04:** **Cover every beachhead category** at least once: TLV, CBOR, MQTT, BLE (for example the GATT/ATT or advertising-data parsers), OTA image headers (for example MCUboot image header/TLV parsing), and ASN.1/X.509. Fill the rest with general formats such as JSON and protobuf.

#### Configurations and RTOS
- **D-05:** A "shipped configuration" is the **upstream default plus at most one realistic variant** per target, for example a feature macro or size option that firmware commonly sets. Configurations are declared in the manifest as macro and flag sets (BED-02). The variant should change compiled code, not only names.
- **D-06:** All configurations target a **Cortex-M4F baseline**: `-mcpu=cortex-m4 -mthumb -mfloat-abi=hard -mfpu=fpv4-sp-d16 -Os`. This matches `thumbv7em-none-eabihf`, the Rust target pinned in Phase 1. The compiler is the container's Arm GNU 14.3.rel1 `arm-none-eabi-gcc`. Per-configuration macros and flags are added on top.
- **D-07:** The "original" build is driven by **each project's own build system**: Make or CMake with an arm-none-eabi toolchain file or a `CC`/`CFLAGS` override. Make builds are captured with Bear and CMake builds through `CMAKE_EXPORT_COMPILE_COMMANDS` (BUILD-01). Only the parser library and its translation units are built; no RTOS, application or link step for a full firmware image.
- **D-08:** **The first RTOS ecosystem is recorded as "deferred; parsers are RTOS-independent"** (BED-02). It will be revisited from the design-partner interviews (weeks 4-10). The record names Zephyr (industrial IoT) and FreeRTOS (consumer IoT) as the candidates. — **Reversibility:** reversible — a later decision entry in PROJECT.md settles it, and no Phase 2 code depends on the choice.

#### Reproduction bar
- **D-09:** BUILD-02 compares **every object file (`.o`)** of the replay against the original build. The goal is byte-identical. Only a **fixed, documented normalisation** is allowed:
  - path remapping (`-ffile-prefix-map`/`-fdebug-prefix-map` added identically to both builds, or applied by the capture);
  - stripping `.comment` and debug sections, and only when they are the sole differing sections.
  The normalisation is applied the same way to every target and recorded in each report. Any other difference fails the configuration.
- **D-10:** The original build runs **inside the pinned container, under capture, once**. The captured `compile_commands.json` is then **replayed in a clean copy of the source tree in the same container**. Nothing is built on the host toolchain. The container is referenced by digest (P1 D-24); the current verified image is `ghcr.io/sat-wik/migrationtool-toolchain@sha256:42c7fb0e00e90e946b5a21e598517fb7189a0de5492fa1d4d6f18e2e9c39e13d`, published from `main` by container run 38202499776 (manifest sha256 `d70029186639d0030fc6cf22b284b3b849ce0680656fecfe06623f6ffdc44142`, the same as the branch-built `sha256:0f5a5d2594b0405931ad7e85d5c6d2f5e3f2d5099d769059870817ee50e152ee` on which the Phase 1 founder check passed), or a later one recorded under `## Current image` in `container/README.md`.
- **D-11:** **Drop policy:** one documented fix attempt per failing target or configuration, such as adding a prefix map or a missing define. If it still differs, the target is dropped. The cause, the differing objects and sections, and the fix tried go into a committed drop log, for example `testbed/DROPPED.md` plus a machine-readable entry. The phase fails if fewer than 10 targets remain.
- **D-12:** **Sources are fetched online, and builds run offline.** The fetch step is the only one with network access. Capture and replay run with `docker run --network none`, which shows the build has no hidden downloads. Phase 1 used the same principle for its offline installs, and it is consistent with P1 D-19, which deferred isolation for customer builds to later phases.

#### Seed corpus
- **D-13:** Seeds come from each target's **tests, fixture files and upstream fuzz corpora**, for example OSS-Fuzz seed corpora committed in the upstream repo. Each seed records its source path, the commit, and how it was extracted, for example a file copy or a byte array lifted from test source. Only a target with no usable inputs gets **at least 10 hand-made seeds**, marked "proposed" (BUILD-03).
- **D-14:** **The GATE-05 held-out split is deferred to Phase 7.** Phase 2 extracts the full seed corpus. The held-out store, the split and the sandbox negative test are designed once, in GATE-05. Nothing in Phase 2 may assume the seeds are hidden from agents.
- **D-15:** Seed format: **one raw input file per seed, named by its sha256**, plus a **per-target index** (TOML or JSON, deterministic, with `schema_version`). The index records the entry function each seed targets, its source and its provenance. Phase 3's TRACTOR vectors can wrap these later; Phase 2 does not adopt the TRACTOR schema.
- **D-16:** Seeds are **committed under `testbed/`**, with each target's licence noted. Per-target count and size caps are applied (the researcher proposes values, for example at most 200 seeds and 1 MiB per target). The cap and the selection rule must be deterministic.

#### Source fetching and cache
- **D-17:** Targets are pinned by **git commit SHA plus a sha256 of the checked-out tree**, excluding `.git`. The fetch is a shallow fetch of that commit, run through the `mt-toolchain` runner. On reuse the tool checks both the commit and the tree hash. This is the same pinning style as Phase 1's KLEE and tinycbor sources.
- **D-18:** Fetched sources and build trees live in a **git-ignored work directory**: `.mt-work/` by default, overridable with `$MT_WORK_DIR`. Only the manifest, drop log, seeds and small reports are committed. Fetches are idempotent and verified on reuse.
- **D-19:** **The fetch runs inside the pinned container** using its git, in a networked `docker run`. Capture and replay then run in the same image with `--network none`.

#### Manifest and CLI
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

#### Entry functions
- **D-23:** Entry functions are **named by hand in the manifest and checked by the tool**. After capture, the tool confirms each named function exists, with its declared signature, in a header the captured build includes. It does this by parsing with clang through the runner, for example `clang -Xclang -ast-dump=json` with the captured flags. A missing or mismatched entry fails validation.
- **D-24:** A **buffer-in** entry takes a pointer and a length (`const uint8_t *`/`const char *`/`const void *` or an equivalent typedef, plus `size_t` or an integer length). It may also take a context, state or output struct pointer. This covers zcbor, tinycbor, nanopb and TLV-style APIs. Each target records 1-3 entry functions.

#### CI scope and offline tests
- **D-25:** A **new `testbed` GitHub Actions workflow** pulls the verified image by digest and runs the full fetch, capture and replay for every target. It runs **weekly on a schedule and on pushes that change `testbed/` or `crates/mt-capture/`**, plus `workflow_dispatch`. Results are published as job annotations, since cloud sessions have no Docker. Actions are SHA-pinned with `contents: read`, as in Phase 1.
- **D-26:** Ordinary pushes run **`cargo test` on recorded fixtures plus a tiny local C project**:
  - committed small `compile_commands.json` and report fixtures captured from CI;
  - a 2-file C project the host's own `cc` builds, so replay and comparison logic is tested anywhere, with no Docker or network.
  This follows the Phase 1 version-fixture pattern. Requirement-ID test names apply, for example `build_02_replay_matches_original_objects`.

### Claude's Discretion
- The exact target list, commits, entry functions and the one variant per target. The researcher proposes them and the founder approves (D-03).
- Seed caps (D-16), the index format (TOML or JSON), and the drop-log layout (D-11).
- Crate split between `mt-capture` and a testbed module (D-21), and the CMake toolchain-file details (D-07).
- Exact normalisation commands (`arm-none-eabi-objcopy --remove-section`, prefix-map flags), within the bounds of D-09.

### Deferred Ideas (OUT OF SCOPE)
- **Held-out corpus split and isolation:** GATE-05 in Phase 7 (D-14).
- **First RTOS final choice:** after design-partner interviews (D-08).
- **Copyleft (LGPL/GPL) targets:** excluded for now (D-02). Revisit only with a recorded decision and a publishing plan.
- **Additional Cortex-M profiles (M0+/thumbv6m, M33):** a later phase or a customer need.
- **Heuristic entry-function discovery** for customer code, which has no hand-named entries.
- **IAR and Keil project parsing, and Zephyr `west` builds:** spec v1 / milestone v2.0.
- **A backup-target swap-in pool** that keeps the bed near 16 after drops. Not chosen; drops simply reduce the count, down to a floor of 10.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| BED-01 | A test bed of 10-20 open-source C parser or protocol targets is pinned by commit, with licence recorded and at least one buffer-in entry function identified per target. | 16 proposed targets with verified commits, licence files and entry signatures (Proposed Test-Bed Targets); manifest schema; `git fetch <sha>` verified to work against GitHub; entry check via clang AST. |
| BED-02 | Each target declares its shipped configurations (macro and flag sets) declaratively, and the first RTOS ecosystem is recorded as a decision (deferred is acceptable). | One variant per target verified to change object code (Configurations); manifest `[[target.config]]` design; RTOS decision text for PROJECT.md. |
| BUILD-01 | For each target and configuration, the tool captures every compile command (Bear for Make, exported compile commands for CMake) into a `compile_commands.json`. | Bear and CMake mechanics, with every pitfall found (Build Capture Mechanics); image gaps (git, cmake, bzip2 absent). |
| BUILD-02 | Replaying the captured commands in the container reproduces the original build output (byte-identical objects or documented normalisation); non-reproducible targets dropped, keeping at least 10. | 31 of 31 target-configuration pairs replayed byte-identical at a fixed path; cross-path results; normalisation commands (Reproduction). |
| BUILD-03 | Each target's tests, fixtures and sample inputs are extracted into a per-target seed corpus; targets with none get at least 10 hand-made seeds (proposed). | Seed sources per target, extraction approach in Rust, caps and selection rule (Seeds). |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

Treated as locked, with the same authority as the decisions above.

- Tool code is Rust in a Cargo workspace (ADR-0001). `#![forbid(unsafe_code)]` in every tool crate; `thiserror` in libraries, `anyhow` only in `mt-cli`; no `unwrap`/`expect` outside tests without an invariant comment.
- Every external tool runs through the `mt-toolchain` runner. Never call `std::process::Command` directly (a repository test and a clippy ban enforce it). That includes tests: the D-26 host-`cc` project must be built through `runner::run` too.
- Deterministic outputs: explicit seeds and budgets, `BTreeMap`, `schema_version`, timestamps only in manifest headers.
- Each requirement ID has a test whose name contains it in snake case (`bed_01_*`, `build_02_*`). Tests never use the network or a live model.
- Pre-merge: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo deny check`, container version check.
- Python only under `research/`. Analysers only as subprocesses. Held-out corpus is not read in this phase (GATE-05 is Phase 7).
- Reports: timeouts are "not proven", missing layers "unavailable", and the two words the project bans for product claims (listed in CLAUDE.md) never appear in reports or docs; this file avoids them too.
- Constraints that touch this phase: one LLVM major (16) for clang; `thumbv7em-none-eabihf` baseline; C only, GCC and Cortex-M first, parsers and protocol logic only; legal: Hayroll and TRACTOR licences are out of scope here; GPL Bear and QEMU run only as container subprocesses.
- Phase loop: discuss, plan, execute, verify, code-review, ship. No direct repo edits outside a GSD command. Never run with `--dangerously-skip-permissions`.

## Summary

Phase 2 needs a manifest of about 16 permissively licensed C parsers, a way to fetch each by commit inside the pinned container, a capture step that records every compile command of the project's own Make or CMake build, a replay step that re-runs those commands in a clean tree, and a seed corpus per target. I built and replayed the whole proposed bed in this research session, using a proxy toolchain (xPack `arm-none-eabi-gcc` 14.2.1, Bear 3.1.3, CMake 3.28.3, clang 18) because the Arm download host and the GHCR blob host are blocked from this session. The result that matters for planning: **all 31 target-configuration pairs of the 16 proposed targets (500 translation units) built under capture and replayed byte-identical** when the replay runs at the same absolute path from a clean copy of the sources, with no normalisation at all. At a different tree path 15 of 16 targets are also identical without any flag, and all 16 are identical with `-ffile-prefix-map=<root>=/mt` added to both builds (Paho forces `-g`, which embeds the path).

Most of the work is therefore not "making replay reproducible" but avoiding silent wrong builds and image gaps. The mechanics research found these planning-relevant facts:

1. **The pinned image cannot run this phase yet.** Its final stage has `make`, Bear, clang 16 and `arm-none-eabi-gcc`, but no `git`, no `cmake`, no `curl`, no `bzip2` (they live only in the `build-base` stage). D-19 (fetch with the container's git) and D-07 (CMake builds) both need an image change, and the container workflow publishes to GHCR from `main` only. That change must be merged to `main` before the `testbed` workflow can pull a digest, so it is the first item on the critical path.
2. **Silent wrong builds are the real hazard.** Passing `-DCMAKE_C_FLAGS=...` replaces the toolchain file's `CMAKE_C_FLAGS_INIT`, so the objects were built for the default Arm target and still "succeeded"; a Makefile that is not given `CC=` builds with the host `cc`; CMake's default Release type appends `-O3` after `-Os`; and `compile_commands.json` from CMake lists every target's sources, including ones never built (38 entries exported, 7 built, for Paho). Each is caught by one gate: read `arm-none-eabi-readelf -A` of every captured object and require `Tag_CPU_arch: v7E-M`, `Tag_THUMB_ISA_use: Thumb-2`, `Tag_FP_arch: VFPv4-D16`, `Tag_ABI_VFP_args: VFP registers`, and record `Tag_ABI_optimization_goals` (`Aggressive Size` means the last `-O` was `-Os`).
3. **Mbed TLS, the ASN.1/X.509 example, cannot be built from a git checkout in this image.** Its CMake path needs the `framework` submodule and runs `scripts/config.py` (Python); its Make path stops with `No rule to make target '../framework/scripts/generate_ssl_debug_helpers.py', needed by 'ssl_debug_helpers_generated.c'` even though the generated files are committed, because they depend on scripts in the submodule. The release tarball (`mbedtls-3.6.7.tar.bz2`) contains the generated files and cross-compiles with `make -C library` in 8 s at `-j4` and 28 s serial (113 translation units). This needs an archive source kind in the manifest (D-17 says commit plus tree hash; the tarball is pinned by archive sha256 plus tree sha256, with the tag's commit recorded) and `curl` and `bzip2` in the image.
4. **Two spec examples do not fit.** lwIP has no upstream cross build (its top-level CMake supports only Windows, Linux, macOS and FreeBSD hosts) and its protocol parsers take `struct pbuf *`, not pointer plus length. MCUboot's parsers read through `flash_area_read` and its library needs a port layer (config, flash map and OS headers) that only Zephyr, Mynewt, TF-M and similar ports provide, so no upstream Make or CMake builds it standalone. The founder decides at the checkpoint how to cover the "TLV" and "OTA image header" categories (see Open Questions).
5. **Entry-function checking works with clang through the runner, but not the naive way.** `-ast-dump=json` of a full translation unit is up to 32 MB (`http_parser.c`), above the runner's 16 MiB storage cap. A probe file that only `#include`s the declaring header gives 4 KB, the dump is a concatenation of JSON documents, `-ast-dump-filter` matches by substring, and `loc.file` is delta-encoded. The pitfalls section has the exact recipe.

**Primary recommendation:** run `mt` inside the container (as `mt toolchain check` does), orchestrate three `docker run` steps from CI (fetch with network; capture and replay with `--network none`), implement everything in a new `mt-capture` crate (modules: manifest, fetch, capture, replay, entries, seeds, report), start with an image PR merged to `main`, then a tracer slice of two targets (QCBOR for Make plus Bear, coreJSON for CMake) end to end in CI before touching the other 14.

## Architectural Responsibility Map

This is a developer-tool phase, so the "tiers" are the places work runs, not web tiers.

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Manifest parsing, validation, tree hashing, report types | `mt-capture` library (runs anywhere, offline) | `mt-cli` (argument parsing only) | Must be testable under plain `cargo test` in cloud sessions with no Docker (P1 D-26, D-26 here). |
| Fetch by commit, build capture, replay, entry check | `mt` running inside the pinned container | CI workflow supplies `docker run` flags | D-10: nothing is built on the host; the runner's `PATH` and tool versions are those of the image. Network policy is a `docker run` flag (D-12), not something the runner can enforce. |
| Orchestration of the three container steps and the network policy | GitHub Actions `testbed` workflow | founder's documented `docker run` lines | Cloud sessions have no Docker (P1 D-23); CI is the only place it runs. |
| Compile, assemble, archive (the "original" build and the replay) | upstream Make/CMake and `arm-none-eabi-gcc` 14.3.rel1 in the image | Bear 3.1.1 (preload) intercepting Make builds | D-07: the project's own build system drives; Bear and CMake export only observe. |
| Entry-function signature check | clang 16 in the image, driven by the runner | captured flags from `compile_commands.json` | D-23: parse with the captured flags, compare the declared signature. |
| Seed extraction (files, C byte arrays, zip corpora) | `mt testbed seeds` (host or container, offline) | hand-made seeds committed from `research/` scripts | Reads fetched sources only; no compiler needed. |
| Committed artefacts (manifest, seeds, drop log, small fixtures) | git repository under `testbed/` | `.mt-work/` (git-ignored) for sources and build trees | D-16, D-18. |

## Proposed Test-Bed Targets (for the D-03 checkpoint)

Everything in this section was produced in this session by cloning each repository, fetching the pinned commit by SHA (`git fetch --depth 1 <url> <sha>` succeeded against GitHub for all 16, which is the D-17 mechanism), reading licence files, building under capture and replaying. Tags were resolved to the commit with `^{}`: `git ls-remote --tags --refs` returns the annotated tag object, not the commit (for Mbed TLS it printed `627361b9...` while the commit is `068ff080...`). [VERIFIED: git ls-remote and git fetch at the pinned SHAs, 2026-10-12]

Repository URLs are `https://github.com/<owner>/<repo>` for every target. Each of the 16 builds the parser library alone, with no RTOS, no application and no firmware link step (NimBLE's Makefile has a `dummy` link rule that is not run; only object goals are built).

**Proxy toolchain caveat.** Build and replay results used xPack `arm-none-eabi-gcc` 14.2.1 (a community GCC 14 build for bare-metal Arm with newlib, downloaded from its GitHub release), Bear 3.1.3, CMake 3.28.3 and clang 18. The image has Arm GNU 14.3.rel1, Bear 3.1.1, CMake 3.25.1 (once added) and clang 16. Treat "builds" as strong evidence for the recipe and MEDIUM evidence for the image; the first CI run is the real test (see the tracer slice).

### Table 1: identity, licence, build (D-02, D-04, D-20)

`TUs` is translation units captured for default / variant. "Cross" notes what the upstream build needed beyond the toolchain file. Licence files were opened at the pinned commit.

| # | id | Repo @ ref | Commit (date) | SPDX / licence file | Category | Build | TUs | Cross-build notes |
|---|----|-----------|---------------|---------------------|----------|-------|-----|-------------------|
| 1 | tinycbor | intel/tinycbor @ v7.0 | 6442e749 (2026-02-18) | MIT / `LICENSE` | cbor | cmake | 11 / 8 | CMake only (no Makefile at v7.0). Target `tinycbor`; `tools/cbordump` is exported but not built. |
| 2 | zcbor | zephyrproject-rtos/zcbor @ main | 9164bd18 (2026-01-14) | Apache-2.0 / `LICENSE` | cbor | cmake | 6 / 6 | No top-level build. `samples/pet/CMakeLists.txt` is plain CMake; build target `pet` (library, no Python needed at build). Latest release tag 0.9.1 is from 2024-10-17. |
| 3 | qcbor | laurencelundblade/QCBOR @ v1.6.1 | 930708bb (2026-03-20) | BSD-3-Clause / `LICENSE` ("essentially" 3-clause; files from Qualcomm CAF carry a modified clause) | cbor | make | 5 / 5 | `make libqcbor.a`; Makefile hard-codes host `ar`. |
| 4 | nanopb | nanopb/nanopb @ nanopb-0.4.9.2 | 160d4f09 (2026-08-24) | Zlib / `LICENSE.txt` | protobuf | cmake | 3 / 3 | CMake demands `protoc` at configure even with the generator off: pass `-Dnanopb_PROTOC_PATH=/usr/bin/true -Dnanopb_BUILD_GENERATOR=OFF`. HEAD is 1.0-dev; use the tag. |
| 5 | cjson | DaveGamble/cJSON @ v1.7.19 | c859b25d (2025-09-09) | MIT / `LICENSE` | json | cmake | 1 / 1 | `-DBUILD_SHARED_LIBS=OFF -DENABLE_CJSON_TEST=OFF`, target `cjson`. Allocates. |
| 6 | corejson | FreeRTOS/coreJSON @ v3.3.1 | cffa492d (2026-01-27) | MIT / `LICENSE` | json | cmake | 1 / 1 | Library target `core_json`; no allocation. |
| 7 | coremqtt | FreeRTOS/coreMQTT @ v5.0.2 | 04845c6a (2026-04-28) | MIT / `LICENSE` | mqtt | cmake | 6 / 6 | Needs `-DSTANDALONE_TEST_BUILD_UNIX=ON` (it only defines `MQTT_DO_NOT_USE_CUSTOM_CONFIG`; otherwise a user `core_mqtt_config.h` is required). Target `core_mqtt`. |
| 8 | paho | eclipse-paho/paho.mqtt.embedded-c @ master | 6035ea2d (2026-07-03) | EPL-2.0 OR BSD-3-Clause (EDL-1.0) / `LICENSE`, `epl-v20`, `edl-v10` | mqtt | cmake | 7 / 7 | Target `MQTTPacketClient` (variant: `MQTTPacketServer`). Top-level CMake forces `CMAKE_BUILD_TYPE Debug`, so objects carry `-g` and absolute paths. Libraries are `SHARED` but fall back to static on `Generic`. |
| 9 | microcoap | 1248/microcoap @ master | ef272895 (2016-02-05) | MIT / `LICENSE.txt` | coap, tlv | make | 2 / 2 | Goals `coap.o endpoints.o` (skip `main-posix.c`). Unmaintained but compiles under GCC 14. |
| 10 | mbedtls | Mbed-TLS/mbedtls @ mbedtls-3.6.7 | 068ff080 (2026-07-01) + archive sha256 `a7e8bcbe...` | Apache-2.0 OR GPL-2.0-or-later / `LICENSE` | asn1_x509, tlv | make | 113 / 113 | **Release tarball only** (git needs the `framework` submodule and Python). `make -C library static`; needs a user config header that removes OS modules. |
| 11 | nimble | apache/mynewt-nimble @ nimble_1_10_0_tag | a7a156f2 (2026-07-10) | Apache-2.0 / `LICENSE` | ble, tlv | make | 65 / 65 | Uses `porting/examples/dummy/Makefile` with overrides: `CC=`, `NIMBLE_CFG_MBEDTLS=`, `NIMBLE_CFLAGS=<baseline>`, `NIMBLE_LDFLAGS=` and explicit `.o` goals (skip `ble_sm_alg.c`, the socket transport, the timer ports). No link step. |
| 12 | libfdt | dgibson/dtc @ v1.8.1 | 8f48565e (2026-05-28) | GPL-2.0-or-later OR BSD-2-Clause (libfdt only) / `BSD-2-Clause`, `README.license` | ota_header (FIT/DTB header), tlv | make | 10 / 10 | Goal `libfdt/libfdt.a` (the `libfdt` goal also links a shared library and fails). `NO_PYTHON=1 NO_YAML=1`. dtc itself and `tests/` are GPL. |
| 13 | httpparser | nodejs/http-parser @ v2.9.4 | 2343fd6b (2020-03-24) | MIT / `LICENSE-MIT` | http | make | 1 / 1 | Goal `libhttp_parser.o`. Makefile hard-codes `-O3` after the caller's flags (see pitfall 5). |
| 14 | isotp | lishen2/isotp-c @ master | 5593428d (2022-01-30) | MIT / `LICENSE` | can | cmake | 1 / - | `add_library(isotp SHARED ...)` becomes static on `Generic`. Configured through `isotp_config.h` only, so no macro variant. |
| 15 | lz4 | lz4/lz4 @ v1.10.0 | ebb370ca (2024-07-21) | BSD-2-Clause (`lib/` only; the rest of the repo is GPL-2.0-or-later) / `lib/LICENSE` | compression | make | 5 / 5 | `make -C lib liblz4.a`; Makefile has `CFLAGS ?= -O3`, so pass `CFLAGS=-Os`. Compiles with no `-o` (see pitfall 8). |
| 16 | zlib | madler/zlib @ v1.3.2 | da607da7 (2026-02-17) | Zlib / `LICENSE` | compression | cmake | 15 / 15 | `-DZLIB_BUILD_SHARED=OFF -DZLIB_BUILD_TESTING=OFF`, target `zlibstatic`. |

Full 40-character commits, archive digest and the reference tree hashes are in the appendix at the end of this section.

**Category coverage (D-04).** CBOR: 1-3. Protobuf: 4. JSON: 5-6. MQTT: 7-8. ASN.1/X.509: 10. BLE: 11. General formats: 13-16. **TLV and OTA image header are not covered by a dedicated library** (open question 2): the tags above mark libraries whose parsed structure is a TLV (CoAP options, DER, BLE advertising data, the devicetree structure block), and libfdt stands in for the OTA category (a FIT image is a devicetree blob; `fdt_check_full` validates its header and structure).

**Dual-licensed targets (D-02 interpretation, open question 1).** Mbed TLS (Apache-2.0 OR GPL-2.0-or-later), Paho (EPL-2.0 OR EDL-1.0, which Eclipse documents as BSD-3-Clause) and libfdt (GPL-2.0-or-later OR BSD-2-Clause) each offer a permissive option. The manifest should record both the full `spdx` expression and an `elected` permissive identifier, and the validator should accept an `OR` expression only when `elected` is on the permissive allow-list. The founder confirms this reading at the checkpoint; if it is rejected, drop 8, 10, 12 and the bed falls to 13 targets (still above 10) but loses ASN.1/X.509 and OTA.

### Table 2: entry functions (D-23, D-24)

Signatures are the `type.qualType` strings clang printed from the captured flags (clang 18 here), verbatim, so they are directly usable as the manifest's `signature`. "Form": **direct** = pointer plus length parameters (D-24 literal); **range** = cursor plus end pointer; **wrapped** = pointer and length inside a struct argument (needs the founder's reading of "equivalent typedef", open question 3). [VERIFIED: `-ast-dump=json -ast-dump-filter=<name>` run on the pinned trees]

| id | Entry | Header (under tree root) | Defining TU | Form | clang `qualType` |
|----|-------|--------------------------|-------------|------|------------------|
| tinycbor | `cbor_parser_init` | `src/cbor.h` | `src/cborparser.c` | direct | `CborError (const uint8_t *, size_t, uint32_t, CborParser *, CborValue *)` |
| tinycbor | `cbor_value_validate` | `src/cbor.h` | `src/cborvalidation.c` | wrapped | `CborError (const CborValue *, uint32_t)` |
| zcbor | `cbor_decode_Pet` | `samples/pet/include/pet_decode.h` | `samples/pet/src/pet_decode.c` | direct | `int (const uint8_t *, size_t, struct Pet *, size_t *)` |
| zcbor | `zcbor_new_decode_state` | `include/zcbor_decode.h` | `src/zcbor_decode.c` | direct | `void (zcbor_state_t *, size_t, const uint8_t *, size_t, size_t, uint8_t *, size_t)` |
| qcbor | `QCBORDecode_Init` | `inc/qcbor/qcbor_decode.h` | `src/qcbor_decode.c` | wrapped (`UsefulBufC` by value) | `void (QCBORDecodeContext *, UsefulBufC, QCBORDecodeMode)` |
| nanopb | `pb_istream_from_buffer` | `pb_decode.h` | `pb_decode.c` | direct | `pb_istream_t (const pb_byte_t *, size_t)` |
| nanopb | `pb_decode_ex` | `pb_decode.h` | `pb_decode.c` | wrapped (stream plus message descriptor) | `bool (pb_istream_t *, const pb_msgdesc_t *, void *, unsigned int)` |
| cjson | `cJSON_ParseWithLength` | `cJSON.h` | `cJSON.c` | direct | `cJSON *(const char *, size_t)` |
| corejson | `JSON_Validate` | `source/include/core_json.h` | `source/core_json.c` | direct | `JSONStatus_t (const char *, size_t)` |
| corejson | `JSON_SearchT` | `source/include/core_json.h` | `source/core_json.c` | direct | `JSONStatus_t (char *, size_t, const char *, size_t, char **, size_t *, JSONTypes_t *)` |
| coremqtt | `MQTT_DeserializePublish` | `source/include/core_mqtt_serializer.h` | `source/core_mqtt_serializer.c` | wrapped (`MQTTPacketInfo_t`) | `MQTTStatus_t (const MQTTPacketInfo_t *, uint16_t *, MQTTPublishInfo_t *, MQTTPropBuilder_t *, uint32_t, uint16_t)` |
| coremqtt | `MQTT_DeserializeAck` | same | same | wrapped | `MQTTStatus_t (const MQTTPacketInfo_t *, uint16_t *, MQTTReasonCodeInfo_t *, MQTTPropBuilder_t *, const MQTTConnectionProperties_t *)` |
| paho | `MQTTDeserialize_publish` | `MQTTPacket/src/MQTTPublish.h` | `MQTTPacket/src/MQTTDeserializePublish.c` | direct | `int (unsigned char *, int *, unsigned char *, unsigned short *, MQTTString *, unsigned char **, int *, unsigned char *, int)` |
| paho | `MQTTDeserialize_connack` | `MQTTPacket/src/MQTTConnect.h` | `MQTTPacket/src/MQTTConnectClient.c` | direct | `int (unsigned char *, unsigned char *, unsigned char *, int)` |
| microcoap | `coap_parse` | `coap.h` | `coap.c` | direct | `int (coap_packet_t *, const uint8_t *, size_t)` |
| mbedtls | `mbedtls_x509_crt_parse_der` | `include/mbedtls/x509_crt.h` | `library/x509_crt.c` | direct (allocates) | `int (mbedtls_x509_crt *, const unsigned char *, size_t)` |
| mbedtls | `mbedtls_asn1_get_tag` | `include/mbedtls/asn1.h` | `library/asn1parse.c` | range | `int (unsigned char **, const unsigned char *, size_t *, int)` |
| nimble | `ble_hs_adv_parse_fields` | `nimble/host/include/host/ble_hs_adv.h` | `nimble/host/src/ble_hs_adv.c` | direct (8-bit length) | `int (struct ble_hs_adv_fields *, const uint8_t *, uint8_t)` |
| nimble | `ble_uuid_init_from_buf` | `nimble/host/include/host/ble_uuid.h` | `nimble/host/src/ble_uuid.c` | direct | `int (ble_uuid_any_t *, const void *, size_t)` |
| libfdt | `fdt_check_full` | `libfdt/libfdt.h` | `libfdt/fdt_check.c` | direct | `int (const void *, size_t)` |
| libfdt | `fdt_check_header` | `libfdt/libfdt.h` | `libfdt/fdt.c` | direct (length from the header) | `int (const void *)` |
| httpparser | `http_parser_execute` | `http_parser.h` | `http_parser.c` | direct | `size_t (http_parser *, const http_parser_settings *, const char *, size_t)` |
| httpparser | `http_parser_parse_url` | `http_parser.h` | `http_parser.c` | direct | `int (const char *, size_t, int, struct http_parser_url *)` |
| isotp | `isotp_on_can_message` | `isotp.h` | `isotp.c` | direct (non-const, 8-bit length) | `void (IsoTpLink *, uint8_t *, uint8_t)` |
| lz4 | `LZ4_decompress_safe` | `lib/lz4.h` | `lib/lz4.c` | direct | `int (const char *, char *, int, int)` |
| lz4 | `LZ4F_getFrameInfo` | `lib/lz4frame.h` | `lib/lz4frame.c` | direct (in/out size) | `size_t (LZ4F_dctx *, LZ4F_frameInfo_t *, const void *, size_t *)` |
| zlib | `uncompress` | `zlib.h` | `uncompr.c` | direct | `int (Bytef *, uLongf *, const Bytef *, uLong)` |
| zlib | `inflate` | `zlib.h` | `inflate.c` | wrapped (`z_stream`) | `int (z_streamp, int)` |

Every target has at least one **direct** entry except QCBOR (wrapped only) and coreMQTT (wrapped only); both have no pointer-plus-length parsing function in their public API. Keep them only if the founder accepts struct-wrapped buffers, otherwise they drop and the bed is 14.

### Table 3: shipped configurations (D-05, D-06)

Default = upstream's own defaults (plus the options the project needs to build at all, shown in Table 1). Variant = one macro or flag set a firmware team plausibly sets. "Code differs" counts objects whose disassembly differs between default and variant (same objects compared by name), measured with `arm-none-eabi-objdump -d`. [VERIFIED: local build, both configurations, pinned commits]

| id | Variant name | Variant (passed how) | Code differs |
|----|--------------|----------------------|--------------|
| tinycbor | `no-float` | CMake `-DWITH_FLOATING_POINT=OFF` (defines `CBOR_NO_FLOATING_POINT`, 8 of 11 TUs remain) | 2 of 8 common |
| zcbor | `stop-on-error` | `-DZCBOR_STOP_ON_ERROR` in `CMAKE_C_FLAGS_INIT` | 5 of 6 |
| qcbor | `no-hw-float` | `-DQCBOR_DISABLE_FLOAT_HW_USE -DQCBOR_DISABLE_PREFERRED_FLOAT` in `CC` | 3 of 5 |
| nanopb | `buffer-only` | `-DPB_BUFFER_ONLY=1 -DPB_NO_ERRMSG=1` | 2 of 3 |
| cjson | `no-locale` | `-DENABLE_LOCALES=OFF` | 1 of 1 |
| corejson | `depth-8` | `-DJSON_MAX_DEPTH=8` | 1 of 1 |
| coremqtt | `sub-vectors-2` | `-DMQTT_SUB_UNSUB_MAX_VECTORS=2U` | 1 of 6 |
| paho | `server` | CMake target `MQTTPacketServer` (compiles with `MQTT_SERVER`; 7 TUs, replayed identical) | 1 of 4 common objects; different TU set |
| microcoap | `release` | `CFLAGS=-Wall` (drops the Makefile's `-DDEBUG`) | 1 of 2 |
| mbedtls | `suite-b` | `-DMBEDTLS_CONFIG_FILE='"<tree>/configs/config-suite-b.h"'` on top of the testbed user config | 87 of 113 |
| nimble | `peripheral` | `-DMYNEWT_VAL_BLE_ROLE_OBSERVER=0 -DMYNEWT_VAL_BLE_ROLE_CENTRAL=0` | 3 of 65 |
| libfdt | `assume-valid` | make `ASSUME_MASK=0xff` | 5 of 10 |
| httpparser | `strict` | make `CPPFLAGS_FAST="-I. -DHTTP_PARSER_STRICT=1"` (appending a second `-D` is a redefinition error under the Makefile's `-Werror`) | 1 of 1 |
| isotp | none | single configuration (no macro knob outside `isotp_config.h`) | - |
| lz4 | `heap-small` | `-DLZ4_HEAPMODE=1 -DLZ4_MEMORY_USAGE=10` | 2 of 5 |
| zlib | `no-gzip` | `-DNO_GZIP` | 3 of 15 |

Rejected variants (evidence for the drop log format, D-11): tinycbor `-DWITH_FREESTANDING=ON` fails to compile `cborpretty.c` (`PRIu64` undefined with `-ffreestanding` and newlib's `<inttypes.h>`); lz4 `-DLZ4_FREESTANDING=1` fails (`memset` undeclared). Both with xPack GCC 14.2.1; the image's newlib may differ.

For the Mbed TLS testbed user config, `-DMBEDTLS_USER_CONFIG_FILE` points at a small committed header that removes `MBEDTLS_NET_C`, `MBEDTLS_TIMING_C`, `MBEDTLS_FS_IO`, `MBEDTLS_HAVE_TIME_DATE`, `MBEDTLS_HAVE_TIME`, `MBEDTLS_PSA_ITS_FILE_C`, `MBEDTLS_PSA_CRYPTO_STORAGE_C`, `MBEDTLS_THREADING_C`, `MBEDTLS_THREADING_PTHREAD` and defines `MBEDTLS_NO_PLATFORM_ENTROPY` (this is the file I used; it built all 113 TUs). It belongs in `testbed/targets/mbedtls/user_config.h` and is part of the shipped configuration (hash it in the report).

### Excluded or conditional candidates (checked, with the evidence)

| Candidate | Licence | Verdict | Evidence |
|-----------|---------|---------|----------|
| lwIP (spec example) | BSD-3-Clause (`COPYING`) | Exclude | `CMakeLists.txt` handles only Windows, Linux, Darwin, FreeBSD hosts (`message(WARNING "Host ... is not supported to build example_app")`). Parsers take `struct pbuf *` (`dhcp_parse_reply(struct pbuf *p, struct dhcp *dhcp)`, `mqtt_parse_incoming(mqtt_client_t *client, struct pbuf *p)`), not pointer plus length. Buffer-in helpers (`inet_chksum`, `ip4addr_aton`) are not protocol parsers. |
| MCUboot (spec example for OTA/TLV) | Apache-2.0 | Conditional (adapter) | `boot/bootutil/CMakeLists.txt` defines `bootutil` but needs `mcuboot_config/mcuboot_config.h`, `flash_map_backend/flash_map_backend.h`, `sysflash/sysflash.h` from a port; TLV parsing reads via `flash_area_read`. Only the Cargo-driven `sim/mcuboot-sys/csupport` provides a standalone port. Capturing it means a testbed-authored port directory, which departs from D-07. If wanted: pin v2.4.0 (latest non-rc tag). |
| MQTT-C | MIT | Exclude | `include/mqtt_pal.h` has branches for unix-like and MSVC only; compiling `mqttc` for arm-none-eabi fails with `unknown type name 'ssize_t'`. |
| Wakaama (LwM2M TLV) | EPL-2.0 OR BSD-3-Clause | Exclude | Top-level `wakaama.cmake` includes `transport/tinydtls/tinydtls.cmake`, which fatals unless the tinydtls submodule is present, and that needs autotools to generate `tinydtls.h`. |
| microtlv | MIT | Exclude | `add_subdirectory(tests)` fails without the `Unity` submodule ("No SOURCES given to target: unity"); the library target exists only via a `build_tlv()` function called from tests and examples; sets `CMAKE_C_FLAGS` as a normal variable (`-O0 --coverage`). |
| jsmn | MIT | Exclude | Header-only; the Makefile compiles only `test/tests.c`; no library translation unit. |
| picohttpparser | MIT or Perl | Exclude | Makefile's only target builds a sanitizer-instrumented test binary; no library build. |
| protobuf-c | BSD-2-Clause | Exclude | `build-cmake/CMakeLists.txt` calls `find_package(Protobuf REQUIRED)` at configure (host protobuf). |
| coreSNTP | MIT | Exclude | `add_library(core_sntp INTERFACE)`; no compiled target. |
| libcbor (backup, verified) | MIT | Backup | v0.14.0, commit `6730c20ab487c0b4dc5fb3fea918937085355bac` (2026-04-07). `cmake -DWITH_TESTS=OFF -DBUILD_SHARED_LIBS=OFF -DWITH_EXAMPLES=OFF` builds target `cbor`. Entry `cbor_load(cbor_data source, size_t source_size, struct cbor_load_result *result)`. Builds; replay not run. |
| yajl (backup, verified) | ISC | Backup | 2.1.0 (2011), commit `66cb08ca2ad8581080b626a75dfca266a890afb2`; target `yajl_s` builds; stale; replay not run. |
| BearSSL | MIT | Unverifiable here | Hosted at `bearssl.org`, blocked from this session; may be checked in CI if an ASN.1 alternative to Mbed TLS is wanted. |

### Appendix: manifest-ready pins and reference tree hashes

`tree_sha256` below uses the algorithm defined in "Tree hash" under Architecture Patterns, computed over the checked-out tree with `.git` excluded (for mbedtls, over the extracted tarball with the top directory stripped). They double as test vectors for the Rust implementation: a mismatch means the algorithm, not the repository, differs. Verify against the CI fetch before committing.

```toml
# id        commit (40 hex)                           tree_sha256
tinycbor  = { ref = "v7.0",              commit = "6442e749ca811e24afad1551338a45c75e09808f", tree = "9acf217a3877140b998586cc8c4172e28bfe90630938344e3cc6067a35b86189" }
zcbor     = { ref = "main",              commit = "9164bd18dcd88ff9d9ef98279501fc1093571017", tree = "131d840f381cd3ac9e5d118f7c59717f04deda4c92e546eeab12663528509f1f" }
qcbor     = { ref = "v1.6.1",            commit = "930708bb86481e88879eb1d87fd4d664f1d69503", tree = "ed42fa8d2ea1619e71613e2925c197ae55a6c0dee151d61983334ef6968f8ee0" }
nanopb    = { ref = "nanopb-0.4.9.2",    commit = "160d4f09e5fabb2b66aa2dea32d4f38ace2c4b3f", tree = "b86f61ac9e874160f9f405412d131d3522c2e9b536a582b8581158ee9520b6ce" }
cjson     = { ref = "v1.7.19",           commit = "c859b25da02955fef659d658b8f324b5cde87be3", tree = "cb98da4236b68041e4be9a325bf3b41ce8a639bd0080faa0abe4a117afde0618" }
corejson  = { ref = "v3.3.1",            commit = "cffa492da18c890181d64462f8af63992a69d3b0", tree = "9ca1058329f6ce0c31f9c13f861ec20cf75f35230b9aae0122ab81480a6df9a5" }
coremqtt  = { ref = "v5.0.2",            commit = "04845c6a8e5f9cf2d232f1c6e80baeb81302e690", tree = "4ab12658af26706fb40200ff240e86b240041bcb002e5467ff1c2b01e7c0739e" }
paho      = { ref = "master",            commit = "6035ea2d4922bb7558b444fb2a051743f3f1974b", tree = "a69bce85a45e561bdf490f80b35ddae7d13eb04a67c9b59fb070a2593286320b" }
microcoap = { ref = "master",            commit = "ef272895925f0d4c563725fe0102966f544a0fdc", tree = "8d331b8362cb16f499647c044f5d4b2033e5fc8ad5a3a33809d86582526cb415" }
nimble    = { ref = "nimble_1_10_0_tag", commit = "a7a156f28954819e158b62dd613008f22f9cf73b", tree = "726eb68c8d5179d0c57aade2ecf44f4a653a11870e0585f8b340ee154e317700" }
libfdt    = { ref = "v1.8.1",            commit = "8f48565e5cfedc74d3f7512f1e0188e9d85dc1de", tree = "f844d15ee31ac3a97fdecce950e50472a4f7a889c2e4e1b678bf9f26dc21b313" }
httpparser= { ref = "v2.9.4",            commit = "2343fd6b5214b2ded2cdcf76de2bf60903bb90cd", tree = "03f91004dd3d60a8b7ada8106d16b3ec6976006f208b216a3f630434436a68a6" }
isotp     = { ref = "master",            commit = "5593428d95af10dde1e565cebcda16089fc74857", tree = "6829a25ee549e3ea0e152124ddd6ebb1945ac548a2b46ce4c3f06e73da73e4c0" }
lz4       = { ref = "v1.10.0",           commit = "ebb370ca83af193212df4dcbadcc5d87bc0de2f0", tree = "8def842ac8ebd5d80ddd31f00ddb3bb156edf5d7980dbcde5c734ac7a2f57ca7" }
zlib      = { ref = "v1.3.2",            commit = "da607da739fa6047df13e66a2af6b8bec7c2a498", tree = "76b51f6bd05c3ad238b1a05c7c163f83f082b64d9d1d3f5df83912f2bd3b26f3" }
# mbedtls is an archive source: commit of the tag is recorded for provenance only.
mbedtls   = { ref = "mbedtls-3.6.7",     commit = "068ff080b369adfac81509f9b57b2afabaf82dc5", tree = "00a636ffc99e447237abd6dee684a9ba0fe3727c1ea6d266cbe68989bb408248",
              archive_url = "https://github.com/Mbed-TLS/mbedtls/releases/download/mbedtls-3.6.7/mbedtls-3.6.7.tar.bz2",
              archive_sha256 = "a7e8bcbec0e6f761b4af24f25677626b35f762f68eef79c08677a363212d11f6" }
```

## Standard Stack

### Core

Rust crates already in the workspace (reuse; no new versions) [VERIFIED: `/home/user/migrationtool/Cargo.toml:15-28`]:

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `serde` / `serde_json` | 1.0.229 / 1.0.151 | Manifest and report types; stream-parse clang's concatenated JSON with `Deserializer::from_slice(..).into_iter()` | Already the report format of Phase 1 |
| `toml` | `=1.1.7` | `testbed/manifest.toml` with `deny_unknown_fields` | Same loader style as `pins.rs` |
| `sha2` | 0.11.0 | Object, seed and tree hashes (sha2 0.11 digests lack `LowerHex`, write a small `hex()` as `runner.rs` does) | Already used by the runner |
| `thiserror` | 2.0.21 | Typed errors in `mt-capture` | Project rule |
| `regex` | 1.13.1 | Parse `arm-none-eabi-readelf -A` tags | Already a dependency of `mt-toolchain` |
| `clap` / `anyhow` | 4.6.7 / 1.0.104 | `mt testbed` argument parsing, `mt-cli` only | Project rule |
| `tempfile` (dev) | 3.27.0 | Temporary trees in offline tests | Already a dev dependency |

New crates (each needs a one-line reason in the PR, `cargo deny check` must still pass):

| Library | Version | Purpose | When to use |
|---------|---------|---------|-------------|
| `shlex` | 2.0.1 (MIT OR Apache-2.0) | Split CMake's `"command"` strings into argv without a shell | Required: CMake 3.28 (and 3.25) export only `command` strings |
| `object` | 0.40.0 (Apache-2.0 OR MIT), `default-features = false`, features `read_core`, `elf`, `std` | List ELF section names and bytes to report differing sections (D-22) and to apply the "sole differing sections" rule (D-09) | Recommended; the alternative is a ~60-line ELF32 section-header reader or one `objcopy --dump-section` run per section |
| `zip` | 9.0.2 (MIT), `default-features = false`, one deflate feature | Read nanopb's `tests/fuzztest/corpus.zip` (3672 entries, 28.5 MB) and `regressions.zip` | Optional: skip it and nanopb still has 176 `\x` literals plus two `.pb` files |

One-line change to an existing crate: derive `Deserialize` on `RunRecord`, `ToolStamp`, `ExitRecord`, `StreamRecord` (they derive only `Serialize` today, `runner.rs`), so that reports containing run records round-trip in fixture tests. It is backwards compatible with schema version 2.

### Container tools (what `mt testbed` runs through the runner)

| Tool | In image today | Needed for | Action |
|------|----------------|------------|--------|
| `arm-none-eabi-gcc`, `-ar`, `-objcopy`, `-readelf`, `-objdump` (14.3.rel1) | yes (`/opt/arm-gnu-toolchain/bin`) | compile, normalise, inspect | none |
| Bear 3.1.1-1 | yes (`apt-base`, pinned) | Make capture | none |
| clang 16.0.6 | yes | entry check | none |
| GNU make | yes (installed, **not** in `[apt]` pins) | Make builds | add `[apt]` pin and `[tool.make]` row |
| git | **no** (only in `build-base`) | D-19 fetch | add |
| cmake | **no** (only in `build-base`) | CMake builds, D-07 | add |
| curl | **no** (only in `build-base`) | Mbed TLS archive fetch | add (or use another source for Mbed TLS) |
| bzip2 | **no** | extract `mbedtls-3.6.7.tar.bz2` | add |
| python3, perl (full), protoc, ninja, pkg-config | no | nothing in the proposed bed | do not add |

[VERIFIED: `/home/user/migrationtool/container/Dockerfile:104-119`, final-stage apt list, quoted:
`ca-certificates clang-${LLVM_MAJOR} llvm-${LLVM_MAJOR} llvm-${LLVM_MAJOR}-dev libclang-${LLVM_MAJOR}-dev libclang-cpp${LLVM_MAJOR}-dev bear qemu-system-arm qemu-user libz3-4 libsqlite3-0 zlib1g libc6-dev make xz-utils`;
`Dockerfile:131-145`, `build-base` stage only: `build-essential cmake curl git libsqlite3-dev libz3-dev ninja-build patch pkg-config python3 zlib1g-dev`;
`Dockerfile:491-506`: `FROM apt-base AS final` with `COPY --from=` for rustup, cargo, cargo-tools, klee, arm only.]

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `mt` inside the container | `mt` on the host driving `docker run` | Host `PATH` differs from the image's `tool_path`; tool versions in run records would describe the host; macOS founder machine; needs Docker in `mt` tests. Rejected. |
| Bear around CMake builds | CMake `CMAKE_EXPORT_COMPILE_COMMANDS` | Bear would record only what ran, with `arguments` arrays and no unbuilt targets, but D-07/BUILD-01 name the CMake export. Keep the export and filter by output existence. |
| `readelf -A` text for arch and optimisation checks | parse `.ARM.attributes` ULEB128 tags in Rust | Text is stable and short; four regexes. Parsing the tags is more code for no extra evidence. |
| Mbed TLS release tarball | git checkout plus `framework` submodule plus Python 3 in the image | Tarball adds `curl` and `bzip2` (small); the git route adds a second repository (the submodule) to the pin and, because Make compares timestamps of the generated files against the generator scripts after a fresh checkout, risks regeneration that needs Python (not tested). |
| Per-config generated CMake toolchain file | `-DCMAKE_C_FLAGS=...` | The cache entry silently replaces the toolchain file's initial flags (measured), so objects lose `-mcpu`. |

**Installation (image, via the Phase 1 pin flow):** add `cmake git curl bzip2` to the `apt-base` `apt-get install` list in `container/Dockerfile`; add `cmake`, `git`, `make`, `curl`, `bzip2` keys to `[apt]` in `container/pins.toml` with the versions printed by the `pin-discovery` workflow for snapshot `20261005T000000Z`; add `[tool.cmake]`, `[tool.git]`, `[tool.make]` (and `[tool.curl]` if wanted) rows; extend `REQUIRED_TOOLS` and the tests listed under Environment Availability.

```bash
# Rust side (only if the optional pieces are accepted)
cargo add shlex@2.0.1 -p mt-capture
cargo add object@0.40.0 --no-default-features --features read_core,elf,std -p mt-capture
cargo add zip@9.0.2 --no-default-features --features deflate-flate2 -p mt-capture   # optional, nanopb corpus only
```

**Version verification.** `cargo info shlex` printed `version: 2.0.1`, `license: MIT OR Apache-2.0`; `cargo info object` printed `version: 0.40.0`, `license: Apache-2.0 OR MIT`, features `read_core`, `elf`, `std` present; `cargo info zip` printed `version: 9.0.2`, `license: MIT`. [VERIFIED: cargo info, 2026-10-12] Debian bookworm package versions are not verifiable from this session (`snapshot.debian.org` and `deb.debian.org` are blocked): cmake `3.25.1-1` is listed for bookworm by [packages.debian.org mirror aptpeek.dev](https://aptpeek.dev/debian/bookworm/main/amd64/cmake/3.25.1-1) [CITED]; git was `1:2.39.5-0+deb12u3` on 2025-10-07 and may be newer at the snapshot [CITED: tracker.debian.org changelog]; `make` (bookworm 4.3-4.1) and `bzip2`, `curl` versions are [ASSUMED]. The `pin-discovery` workflow prints the real candidates; do not hand-type them.

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| shlex | crates.io | since 2015-06 | 21.7M/week | github.com/comex/rust-shlex | OK | Approved |
| object | crates.io | since 2016-08 | 9.4M/week | github.com/gimli-rs/object | OK | Approved (optional use) |
| zip | crates.io | long-lived (zip-rs/zip2) | 6.4M/week | github.com/zip-rs/zip2 | OK | Approved (optional use) |

[VERIFIED: `gsd-tools query package-legitimacy check --ecosystem crates shlex object zip`, all `OK`, no postinstall.] The three names come from the Rust ecosystem's well-known crates and were confirmed with `cargo info`; no crate was taken from a web search result alone.

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none
Debian packages (`cmake`, `git`, `curl`, `bzip2`) come from the snapshot archive pinned by Phase 1, not from a registry lookup.

## Architecture Patterns

### System Architecture Diagram

```
 testbed/manifest.toml ──► mt testbed validate (offline, cargo test + CI)
        │ ids, commits, tree hashes, entries, configs
        ▼
 [docker run  NETWORK ON ]  mt testbed fetch --all
        │  git init + fetch --depth 1 <sha>  (or curl + sha256 + tar for mbedtls)
        │  check HEAD == commit ; tree_sha256 == manifest  ──► .mt-work/cache/<id>/<commit>/tree
        ▼
 [docker run  --network none ]  mt testbed capture --all
        │  per <id>,<config>:  copy cache tree ──► /work/run/<id>/<cfg>/src
        │     Make : bear --output raw.json -- make -j1 <vars> <goals>
        │     CMake: cmake -S -B -DCMAKE_TOOLCHAIN_FILE=<generated> ; cmake --build --target T -j1
        │  canonicalise  (command→arguments, output from -o, drop unbuilt, sort)  ──► compile_commands.json
        │  gates: argv[0] allow-list ; ELF arch/opt of every object ; objects sha256
        │  entry check: probe TU + captured flags ──► clang -ast-dump=json ──► signature equals manifest?
        ▼  capture-report.json (run records, hashes, TU count, gates, entries)
 [docker run  --network none ]  mt testbed replay --all
        │  rm -rf run dir ; restore pristine tree (hash-checked) ; keep CMake configure files ;
        │  delete declared outputs ; run each argv (cwd = directory) via runner ; compare
        │  identical ─► ok ; else section diff ─► normalise (.comment, .debug_*) ─► ok/fail
        ▼  replay-report.json ──► drop log (D-11) if a configuration fails
 mt testbed seeds --all [--check]  (offline) ──► testbed/seeds/<id>/<sha256> + index.json
        ▼
 GitHub Actions `testbed`: annotations + artefacts ──► fixtures for offline cargo tests (D-26)
```

### Recommended Project Structure

```
crates/mt-capture/
├── src/lib.rs          # forbid(unsafe_code); re-exports
├── src/manifest.rs     # schema, loader, validator (mirror pins.rs)
├── src/treehash.rs     # tree sha256, symlink-safe copy
├── src/fetch.rs        # git/curl fetch through the runner, cache verify
├── src/cc.rs           # compile_commands: parse, shlex, canonicalise, filters
├── src/capture.rs      # toolchain file, Make/CMake drivers, gates
├── src/elf.rs          # sections (object crate), readelf -A tags
├── src/replay.rs       # clean restore, replay, compare, normalise
├── src/entries.rs      # probe TU, clang AST JSON stream, signature compare
├── src/seeds/          # lexer for C byte arrays, selection, index writer
├── src/report.rs       # report types (Serialize + Deserialize)
└── tests/              # bed_01_*, bed_02_*, build_01_*, build_02_*, build_03_* ; fixtures/
crates/mt-cli/src/main.rs   # adds `Group::Testbed` (argument parsing only)
testbed/
├── manifest.toml
├── README.md           # the three docker run lines
├── DROPPED.md  dropped.toml
├── targets/<id>/       # committed config inputs, e.g. mbedtls/user_config.h
└── seeds/<id>/         # <sha256> files + index.json + NOTICE
.github/workflows/testbed.yml
```

One crate, not two: the manifest and seed code is small, and `docs/guidelines/building-the-tool.md` §3 already names `mt-capture` and `testbed/`. Add `crates/mt-capture` to `[workspace] members`.

### Pattern 1: Manifest schema (mirror `pins.rs`)

**What:** one TOML file, `schema_version = 1`, `#[serde(deny_unknown_fields)]` on every struct, `BTreeMap` for targets, a `validate()` that returns every issue sorted (like `PinsError::Invalid`). [VERIFIED: `/home/user/migrationtool/crates/mt-toolchain/src/pins.rs:43-46`, `#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]`, `#[serde(deny_unknown_fields)]`, `pub struct Pins`, with `BTreeMap` fields]

```toml
schema_version = 1

[decisions]                     # BED-02
first_rtos = "deferred"
first_rtos_reason = "parsers are RTOS-independent; revisit after design-partner interviews (weeks 4-10)"
first_rtos_candidates = ["zephyr", "freertos"]
project_decision = "D-17"       # the PROJECT.md line, checked by a test

[image]                         # must equal container/README.md "Current image"
digest = "sha256:..."

[baseline]                      # D-06
cflags = ["-mcpu=cortex-m4", "-mthumb", "-mfloat-abi=hard", "-mfpu=fpv4-sp-d16", "-Os"]
work_dir = "/work"              # path inside the container; fixed so recorded paths are stable

[seed_caps]                     # D-16
max_seeds = 200
max_total_bytes = 1048576
max_seed_bytes = 65536

[target.qcbor]
category = ["cbor"]
build = "make"
[target.qcbor.source]
kind = "git"                    # or "archive" (mbedtls): url, sha256, strip_components
url = "https://github.com/laurencelundblade/QCBOR"
ref = "v1.6.1"                  # informational
commit = "930708bb86481e88879eb1d87fd4d664f1d69503"
tree_sha256 = "ed42fa8d..."
[target.qcbor.licence]
spdx = "BSD-3-Clause"
elected = "BSD-3-Clause"        # required, and must be on the allow-list
file = "LICENSE"
[target.qcbor.build_args]       # everything the project's own build system needs
dir = "."
goals = ["libqcbor.a"]
make_vars = ["CC=arm-none-eabi-gcc {baseline}"]   # {baseline} expands to baseline.cflags
[[target.qcbor.entry]]
name = "QCBORDecode_Init"
header = "inc/qcbor/qcbor_decode.h"
source = "src/qcbor_decode.c"
form = "wrapped"                # direct | range | wrapped
signature = "void (QCBORDecodeContext *, UsefulBufC, QCBORDecodeMode)"
[[target.qcbor.config]]
name = "default"
[[target.qcbor.config]]
name = "no-hw-float"
defines = ["QCBOR_DISABLE_FLOAT_HW_USE", "QCBOR_DISABLE_PREFERRED_FLOAT"]
```

**Validation rules (tests named per D-20):** unknown keys rejected; `commit` 40 lowercase hex; `tree_sha256` 64 hex; archive sources require `sha256`; `licence.elected` on the allow-list `MIT, BSD-2-Clause, BSD-3-Clause, Apache-2.0, Zlib, ISC` (D-02) and contained in `spdx`; licence `file` relative with no `..`; at least one entry with a `direct` or `range` form unless flagged `accepted_wrapped = true`; 1-3 entries (D-24); exactly one config named `default`, at most one other (D-05); config and entry names unique; ids match `[a-z0-9_]+` and are unique (TOML keys already are); every path relative and `..`-free; `{baseline}` is the only placeholder; target count between 10 and 20 for the committed file (BED-01), checked in a separate test so unit tests can use small manifests.

### Pattern 2: Tree hash (D-17)

**What:** a pure-Rust function over a directory, identical for a fresh checkout, the cache, and the replay copy.
Algorithm (the reference values in the appendix use it): walk the tree without following symlinks, skipping a top-level `.git`; for each regular file or symlink (including symlinks to directories) build the row `"<mode> <sha256-hex> <relpath>\n"` where mode is `100644`, `100755` (any execute bit) or `120000` (symlink; the hashed content is the link target text, never followed); sort rows by the relpath bytes; the tree hash is the sha256 of the concatenated rows. Directories are not listed. nanopb and NimBLE contain symlinks (`spm_headers/nanopb/*.h`, `porting/npl/riot/include/npl_syscfg/npl_sycfg.h`), so the copy to the run directory must recreate symlinks as symlinks and refuse targets that escape the tree. [VERIFIED: `find -type l` on the pinned trees]

### Pattern 3: Fetch (D-17, D-19) through the runner

Environment added by the caller (the runner starts from an empty environment plus `PATH`, `LANG`, `TZ`, `SOURCE_DATE_EPOCH`): `HOME=<work>/home`, `GIT_TERMINAL_PROMPT=0`, `GIT_CONFIG_NOSYSTEM=1`, `GIT_CONFIG_GLOBAL=/dev/null`, `GIT_ALLOW_PROTOCOL=https` (tests with a local `file://` repository set `file` as well). Commands, each a separate `RunRequest`, absolute `git` path from `pins.tool`:

```
git init --quiet <dir>
git -C <dir> fetch --quiet --depth 1 --no-tags --no-recurse-submodules <url> <sha>
git -C <dir> checkout --quiet --detach FETCH_HEAD
git -C <dir> rev-parse HEAD            # must equal the pin
```

then compute the tree hash (excluding `.git`), compare, and move the tree to `.mt-work/cache/<id>/<commit>/tree`. On reuse, verify both again. Archive sources: `curl` with the hardened flags of `fetch::curl_argv` plus `-o <file>`, verify the archive sha256, `tar -xjf <file> --strip-components=1 --no-same-owner -C <dir>`, then the tree hash. [VERIFIED: fetching by SHA from GitHub worked for all 16 commits; `curl_argv` flags read from `/home/user/migrationtool/crates/mt-toolchain/src/fetch.rs:49-71`]

### Pattern 4: Work layout and fixed paths

Inside the container the work directory is always `/work` (host: `$MT_WORK_DIR`, default `.mt-work`, bind-mounted). Recorded paths in `compile_commands.json` and reports are therefore the same on a Mac, in CI and in a cloud fixture. Layout: `/work/cache/<id>/<commit>/tree`, `/work/run/<id>/<config>/{src,build}`, `/work/capture/<id>/<config>/{compile_commands.json,report.json}`, `/work/home`. Original build and replay both use `/work/run/<id>/<config>/`.

### Pattern 5: Capture drivers

**Make:** `bear --output <raw.json> -- make -j1 <make_vars...> <goals...>` with `cwd = src/<dir>`. Use `-j1`: with `-j4` Bear's output order changed from run to run (three runs, three file hashes; sorted contents equal). [VERIFIED: Bear 3.1.3, NimBLE subset, `-j4`] Inject the baseline through `CC="arm-none-eabi-gcc {baseline}"` so the Makefile keeps its own `CFLAGS` (microcoap's `CFLAGS += -Wall -DDEBUG`, libfdt's `-g -Os -Werror ...`, http-parser's `-Wall -Wextra -Werror`); where the Makefile defaults an optimisation level with `?=` (Mbed TLS `CFLAGS ?= -O2`, lz4 `CFLAGS ?= -O3`) also pass `CFLAGS=-Os` (Mbed TLS: `CFLAGS=-Os -DMBEDTLS_USER_CONFIG_FILE=...`).

**CMake:** write a deterministic toolchain file per target-configuration into the capture directory and hash it into the report:

```cmake
set(CMAKE_SYSTEM_NAME Generic)
set(CMAKE_SYSTEM_PROCESSOR arm)
set(CMAKE_C_COMPILER   /opt/arm-gnu-toolchain/bin/arm-none-eabi-gcc)
set(CMAKE_AR           /opt/arm-gnu-toolchain/bin/arm-none-eabi-ar)
set(CMAKE_RANLIB       /opt/arm-gnu-toolchain/bin/arm-none-eabi-ranlib)
set(CMAKE_TRY_COMPILE_TARGET_TYPE STATIC_LIBRARY)   # without it configure dies: "undefined reference to `_exit'"
set(CMAKE_C_FLAGS_INIT "-mcpu=cortex-m4 -mthumb -mfloat-abi=hard -mfpu=fpv4-sp-d16 -Os <config defines>")
set(CMAKE_FIND_ROOT_PATH_MODE_PROGRAM NEVER)
set(CMAKE_FIND_ROOT_PATH_MODE_LIBRARY ONLY)
set(CMAKE_FIND_ROOT_PATH_MODE_INCLUDE ONLY)
```

then `cmake -S src/<dir> -B build -G "Unix Makefiles" -DCMAKE_TOOLCHAIN_FILE=<file> -DCMAKE_EXPORT_COMPILE_COMMANDS=ON -DCMAKE_BUILD_TYPE=MinSizeRel <target options>` and `cmake --build build --target <T> -j1`. `MinSizeRel` gives `-Os -DNDEBUG`; the default (empty) type adds nothing, `Release` appends `-O3` after the baseline's `-Os` (last wins). Projects that set their own type (Paho forces `Debug`) keep it; the ELF gate records the effective result. Config defines go into `CMAKE_C_FLAGS_INIT`, never `-DCMAKE_C_FLAGS=`. [VERIFIED: local, CMake 3.28.3; the `_exit` failure reproduced by removing the line]

### Pattern 6: Canonical `compile_commands.json` (BUILD-01)

Input is Bear's file (`arguments` arrays) or CMake's (`command` strings). Canonicalise:
1. `command` to `arguments` with `shlex::split` (no shell), keeping Bear's `arguments` as is.
2. Output path: the `-o` argument (or `-ofile`) resolved against `directory`; **if there is no `-o`, GCC writes `<file stem>.o` in `directory`** (lz4's Makefile does this; Bear then emits no `output` field). Do not trust the `output` field: CMake writes it relative to the *top-level* build directory even for sub-directory targets (Paho: `directory` is `build/MQTTPacket/src`, `output` is `MQTTPacket/src/CMakeFiles/...`). [VERIFIED]
3. Drop entries whose output does not exist after the build (CMake exports every target: Paho 38 entries, 7 built; tinycbor 12, 11 built; zcbor lists `samples/pet/src/main.c`).
4. Sort by (`file`, resolved output) and write pretty JSON with a trailing newline; hash both the raw and the canonical file.
5. Allow-list `argv[0]`: it must equal the pinned `arm-none-eabi-gcc` path (Bear records the absolute resolved path). D-26 tests pass `cc` through a configurable allow-list. A hostile or corrupted fixture must not make replay run arbitrary programs.

### Pattern 7: Replay and compare (BUILD-02)

1. Remove `/work/run/<id>/<config>` entirely; restore the pristine tree from the cache (hash-checked) to `.../src`.
2. For CMake keep `.../build` (configure-time files such as `tinycbor-export.h` are inputs of the compile commands, not outputs; hash them in the capture report and compare again before replay) and delete every declared output. For Make builds the whole run directory was rebuilt from the pristine tree, so any file the original build generated and the compile commands need would be missing; in all 16 targets none was (replay succeeded), and the capture report should still list created-but-undeclared files so a future target cannot hide such a dependency.
3. For each canonical entry in order: `RunRequest { argv = arguments, cwd = directory, tool_name = "arm-none-eabi-gcc" }` through the runner (no shell). A non-zero exit or `timed_out` fails the configuration ("not proven" on timeout).
4. Hash every output. Equal means `identical`. Otherwise list differing sections (see Pattern 8) and apply the fixed normalisation.

Replay at the same absolute path from a clean copy is the BUILD-02 evidence (31 of 31 identical, no normalisation). An optional `--relocate` check (second root, `-ffile-prefix-map=<root>=/mt` in both builds) shows path independence; it is not needed for the bar and is the first thing to cut under time pressure. [VERIFIED: Reproduction section]

### Pattern 8: Normalisation (D-09), fixed and documented

Only two normalisations exist, applied identically to every target and recorded in every report:
1. Path remapping: `-ffile-prefix-map=<run root>=/mt` added to both builds (only on a D-11 fix attempt or the `--relocate` check).
2. Strip `.comment` and debug sections, **only if every differing section is `.comment`, `.debug_*`, `.rel.debug_*` or `.rela.debug_*`**.

Implementation: list sections of both objects (`object` crate: name, size, sha256 of data). If the differing set is empty the objects are identical; if every differing section is in the allowed set, run `arm-none-eabi-objcopy --strip-debug --remove-section=.comment <in> <out>` on both (runner), hash the outputs, and require equality; anything else is `differs` with the section names recorded. [ASSUMED: this exact objcopy invocation; in this session objects were identical so the path was not exercised on real data. Commit two tiny Arm object fixtures that differ only in `.comment` to test it offline.]

### Pattern 9: Entry check (D-23)

Per entry: write `entry_probe.c` containing `#include "<header>"` into the capture directory; run
`clang --target=arm-none-eabi --sysroot=/opt/arm-gnu-toolchain/arm-none-eabi -fsyntax-only -Xclang -ast-dump=json -Xclang -ast-dump-filter=<name> <allow-listed captured flags> entry_probe.c`
with `cwd` = the captured `directory`. Parse stdout as a stream of JSON documents; select `kind == "FunctionDecl"` and `name == <name>` exactly; compare `type.qualType` with the manifest `signature` after collapsing whitespace; also run `clang -MM <flags> <defining TU>` and require the header to be in the dependency list ("a header the captured build includes"). The flag allow-list: `-D`, `-U`, `-I`, `-isystem`, `-iquote`, `-include`, `-std=`, `-ffreestanding`, `-fsigned-char`, `-funsigned-char`, `-fshort-enums`, `-mcpu`, `-mthumb`, `-mfloat-abi`, `-mfpu`, `-march` (clang rejects GCC-only `-f` options). A missing function gives empty output with exit 0, which must fail. [VERIFIED: clang 18 with the xPack sysroot, all 28 entries found; `zcbor_new_decode_state` reported missing when I named the wrong TU]

### Pattern 10: Seeds (D-13 to D-16)

See the Seeds section.

### Pattern 11: CLI, runner use and where `mt` runs

`mt testbed validate | fetch | capture | replay | seeds` with `--manifest`, `--pins`, `--work-dir`, `--all` or an id, and optional `--config`; exit 0 success, 1 a target-configuration failed or was not proven, 2 invalid inputs (the convention of `mt toolchain`). `capture` includes the entry check and writes its results into the capture report. `mt` is mounted read-only into the image like `mt toolchain check` does, with `pins.toml` and the manifest; the work directory and an output directory are the only writable mounts.

Runner use: `RunnerConfig::new(pins.image.tool_path, pins.image.source_date_epoch)` with an explicit `timeout` per step (the default is 120 s; Mbed TLS takes about 30 s serial, plan 900 s for builds and replays, 600 s for fetch) and the default 16 MiB `output_cap`. Always check `exit.timed_out`, `exit.code` and `stdout.complete`/`stderr.complete` (the open runner residual WR-13 means a `complete: false` stream must be reported, not trusted). Pass absolute tool paths from `pins.tool[...].bin`; add `HOME`, `TMPDIR` and the git variables above through `extra_env`. [VERIFIED: runner semantics read in `/home/user/migrationtool/crates/mt-toolchain/src/runner.rs:93-135` and `:648-690`; constants quoted from lines 69 and 72: `pub const RUN_RECORD_SCHEMA_VERSION: u32 = 2;` and `pub const DEFAULT_OUTPUT_CAP: usize = 16 * 1024 * 1024;`; `RunRecord`, `ToolStamp`, `ExitRecord`, `StreamRecord` derive `Debug, Clone, PartialEq, Eq, Serialize` only (lines 138, 149, 169, 191)]

### Pattern 12: Reports (D-22)

All deterministic: `BTreeMap`, `schema_version`, no timestamps except an optional `header.generated_by` block.

```json
{ "schema_version": 1, "kind": "capture", "target": "qcbor", "config": "no-hw-float",
  "image": "sha256:...", "source": { "commit": "...", "tree_sha256": "..." },
  "tools": { "arm_gcc": "14.3.Rel1", "bear": "3.1.1", "clang": "16.0.6", "make": "4.3" },
  "steps": [ { "name": "build", "run": { "<RunRecord, schema_version 2>": "..." } } ],
  "toolchain_file_sha256": null,
  "compile_commands": { "raw_sha256": "...", "canonical_sha256": "...", "entries_exported": 5, "entries_built": 5 },
  "gates": { "compiler_allow_listed": true, "arch_ok": true, "opt_goal_size": true, "created_undeclared": [] },
  "objects": { "src/qcbor_decode.o": { "sha256": "...", "arch_ok": true, "opt_goal": "Aggressive Size" } },
  "entries": { "QCBORDecode_Init": { "found": true, "signature_matches": true, "header_included": true } } }
```
```json
{ "schema_version": 1, "kind": "replay", "target": "qcbor", "config": "no-hw-float", "verdict": "identical",
  "objects": { "src/qcbor_decode.o": { "original": "...", "replay": "...", "status": "identical",
      "normalised_original": null, "normalised_replay": null, "differing_sections": [] } },
  "normalisation": "none", "steps": [ ] }
```
Verdicts: `identical`, `identical_after_normalisation`, `differs`, `not_proven` (timeout or incomplete stream), `build_failed`. Verdict words stay factual; no product or module claim is made.

### Pattern 13: Drop log (D-11)

`testbed/dropped.toml` (machine) plus `testbed/DROPPED.md` (generated or hand-kept in sync by a test): `[[dropped]]` with `target`, `config` (optional: variant-only drop), `cause` (`build_failed`, `differs`, `not_proven`, `entry_missing`, `licence`), `detail`, `differing_objects`, `differing_sections`, `fix_attempted`, `ci_run`. A test requires `manifest targets - fully dropped targets >= 10` and that each dropped id is still named in the manifest or the log, never silently removed.

### Pattern 14: CI workflow (D-25)

New `.github/workflows/testbed.yml`, structured like `container.yml` (SHA-pinned actions, `permissions: contents: read`, `persist-credentials: false`, per-event concurrency group). Triggers: `push` on `testbed/**`, `crates/mt-capture/**`, `crates/mt-toolchain/**`, `crates/mt-cli/**`, the workflow file; `schedule` weekly (offset from the container workflow's `17 4 * * 1`); `workflow_dispatch`. Jobs: `mt` (build static musl `mt` as `container.yml` does; artifacts cannot be shared across workflows) then `testbed`:
1. Read the image: `grep -oE 'ghcr.io/sat-wik/migrationtool-toolchain@sha256:[0-9a-f]{64}' container/README.md | head -1` (exactly one match today; a test pins this) and require it equals `manifest.toml [image] digest`.
2. `docker pull` by digest, create `$RUNNER_TEMP/work` and `out` with `chmod 0777` (the container user is `65534:65534`).
3. `fetch`: `docker run --rm --user 65534:65534 --read-only --tmpfs /tmp:rw,exec,nosuid,size=2g -v mt:/usr/local/bin/mt:ro -v pins:/opt/mt/pins.toml:ro -v testbed:/opt/mt/testbed:ro -v work:/work image mt testbed fetch --all ...` (network on).
4. `capture` then `replay` with the same flags plus `--network none`.
5. `seeds --all --check` (committed seeds equal regenerated ones).
6. Annotations in 3800-byte chunks using the `annotate` helper pattern of `container.yml`: one summary table per step, drop-log candidates, and compact fixtures (two small targets' canonical `compile_commands.json` and reports) for D-26.
7. `actions/upload-artifact` of reports (7 days).
Estimated wall time: serial builds 132 s plus replays 63 s for all 31 pairs measured here (4 cores, 500 TUs), plus fetch of 16 small trees (largest 44 MB for Mbed TLS), plus image pull; expect 10-15 minutes; set `timeout-minutes: 45`. [VERIFIED for the build and replay sums; fetch and pull are estimates]

## Build Capture Mechanics (BUILD-01)

**Bear with an arm-none-eabi cross build.** Preload mode (the default; `--force-wrapper` also exists) intercepted `arm-none-eabi-gcc` correctly with no extra flags: `bear --output cc.json -- make CC=arm-none-eabi-gcc CFLAGS=... libqcbor.a` produced one entry per compile with an `arguments` array whose first element is the absolute path the compiler resolved to, plus absolute `file`, `directory` (the make working directory) and `output` when the command had `-o`. Relative arguments (`-I inc`, `src/UsefulBuf.c`) are kept relative to `directory`. The compiler's internal `cc1`/`as` executions are ignored, and dependency-generation commands (`gcc -MM`, run by microcoap's Makefile) are not recorded; `ar` is not a compile and is not recorded. [VERIFIED: Bear 3.1.3, xPack GCC 14.2.1, QCBOR, microcoap, NimBLE, Mbed TLS, libfdt, lz4, http-parser]

**Bear needs the loopback interface.** In preload mode the intercepted processes report to Bear over gRPC on `127.0.0.1`. In a bare network namespace with `lo` down the build still succeeds but Bear prints `wrapper: failed with: gRPC call failed ... 127.0.0.1:40153: Network is unreachable` and records nothing; with `lo` up it recorded all 5 QCBOR entries. `docker run --network none` creates only the loopback device, with 127.0.0.1/8 configured [CITED: https://docs.docker.com/engine/network/drivers/none/], so D-12 is compatible with Bear. Because a silent capture failure is possible, add a gate: every object file created during the build must appear as a declared output, and the entry count must be at least 1. [VERIFIED: `unshare -n` with and without bringing `lo` up; Bear 3.1.1 in the image is [ASSUMED] to use the same mechanism]

**CMake.** `CMAKE_EXPORT_COMPILE_COMMANDS=ON` writes `compile_commands.json` into the build directory at generate time, as `command` strings (never `arguments`) with `directory` = the sub-directory of the owning target, `file` absolute, and `output` relative to the top-level build directory. It lists the sources of **every** target in the project, built or not. Details and the filter are in Patterns 5 and 6. CMake is not in the image (see the Standard Stack table); `3.25.1-1` is the bookworm version, and every proposed target's `cmake_minimum_required` is at most 3.15 (tinycbor 3.10, zcbor/samples 3.13.3, nanopb 3.14.0, cJSON 3.0, coreJSON 3.15, coreMQTT 3.15, Paho 3.5, isotp-c 3.10, zlib `3.12...3.31`). [VERIFIED: grep of each `CMakeLists.txt` at the pinned commits]

**Toolchain-file pattern** (`CMAKE_SYSTEM_NAME Generic`, `CMAKE_TRY_COMPILE_TARGET_TYPE STATIC_LIBRARY`): in Pattern 5. Without the `STATIC_LIBRARY` line, configure fails with `CMakeTestCCompiler.cmake ... is not able to compile a simple test program ... undefined reference to _exit` (newlib stubs). `--specs=nosys.specs` also works but makes CMake link and run feature probes as executables; prefer the static-library mode. [VERIFIED: both variants run]

**Can a networked `docker run` fetch?** In principle yes: the default bridge network gives a container outbound access, the image has `ca-certificates`, and anonymous GHCR pulls worked in this session (token and manifest fetched without credentials; the blob host `pkg-containers.githubusercontent.com` is blocked from this session only). This session has no Docker daemon, so the fetch container itself was not run here [ASSUMED from standard Docker behaviour]; the first CI run proves it. The container needs `git` (absent, see above), a writable `HOME`, and `--read-only` with a tmpfs `/tmp`.

## Reproduction (BUILD-02)

**Result matrix.** Each row: target built under capture at a fixed root (`/tmp/mtw`, standing in for `/work`) with `-j1`, then the run directory deleted, the pristine tree copied back, declared outputs removed and every captured argv re-run via a shell-free subprocess; objects compared by sha256. Pinned commits throughout. [VERIFIED: local harness, xPack GCC 14.2.1]

| Experiment | Targets | Result |
|------------|---------|--------|
| Same path, clean tree copy, default and variant configurations | 16 targets, 31 pairs, 500 translation units | **31 of 31 pairs, every object byte-identical**, no normalisation |
| Make targets rerun with the upstream-flag-preserving recipes (baseline injected through `CC`) | qcbor, microcoap, libfdt, httpparser, lz4 (10 pairs) | 10 of 10 identical |
| Same configuration built under a different root (`/tmp/mtx`), **no** prefix map | 16 targets, default configuration | 15 of 16 identical; Paho 7 of 7 differ (its CMake forces `Debug`, so DWARF carries absolute paths) |
| Different root, `-ffile-prefix-map=<root>=/mt` added to **both** builds | 16 targets, default configuration | **16 of 16 identical**, including Paho; adding the map to a non-`-g` build changes nothing (0 objects differ) |
| Minimal case (`x.c` with `__FILE__`, built in two directories) | 1 file | different path: differs (the string); with `-g`: differs (DWARF); with `-ffile-prefix-map=<dir>=.` in both: identical with and without `-g` |

**Sources of non-determinism in GCC 14 object files**, with what was observed:

| Source | Observed | Handling |
|--------|----------|----------|
| Absolute source or build path in `__FILE__`, `assert` messages, DWARF (`DW_AT_name`, `DW_AT_comp_dir`, line tables) | Only effect found; objects differ across tree roots exactly when `-g` or `__FILE__` is used | Same-path replay; or `-ffile-prefix-map=<root>=/mt` in both builds (alone it was sufficient, no `-fdebug-prefix-map` needed) |
| `.comment` section | Contains the compiler identification, identical for the same compiler across runs and trees | Allowed in normalisation (D-09), expected unused |
| `__DATE__`, `__TIME__`, `__TIMESTAMP__` | None used in library sources (grep of the pinned trees excluding tests). GCC 7 and later take `__DATE__` and `__TIME__` from `SOURCE_DATE_EPOCH`, which the runner sets; `__TIMESTAMP__` (file mtime) is not covered by it [CITED: https://wiki.debian.org/ReproducibleBuilds/TimestampsFromCPPMacros] | Keep the runner's fixed `SOURCE_DATE_EPOCH`; a manifest test or capture gate can grep the captured sources for `__TIMESTAMP__` |
| Random seeds (`-frandom-seed`) | Matter for LTO, C++ anonymous namespaces and profile builds; none used; independent compiles were identical [ASSUMED beyond the 31 pairs] | Reject `-flto` and `-fprofile-*` in captured argv as an unverified configuration |
| `STT_FILE` symbol | Basename only (`x.c`), not the path | none |
| Parallel make | Per-object bytes unaffected; log text and Bear entry order are | `-j1`, plus canonical sort |
| Environment (`TMPDIR`, locale, time zone) | No effect on content | Runner fixes `LANG`, `TZ` |

**How to replay deterministically** (Pattern 7): same path from a clean copy is the evidence; run `arguments` (or the `shlex`-split `command`) with `cwd = directory`, never through a shell; create `directory` and the output's parent if missing (lz4's Makefile creates `cachedObjs/<hash>/` itself and then compiles with no `-o`); delete declared outputs first so a stale object cannot pass; fail on any non-zero exit.

**Recommended strategy.** (1) BUILD-02 evidence = same-path clean-copy replay with zero normalisation; (2) implement the fixed normalisation of Pattern 8 anyway, because D-09 requires it to be defined and the image's compiler (14.3.rel1) was not available to test; (3) `--relocate` (second root with the prefix map) is an optional extra and the cheapest feature to drop. A D-11 "one fix attempt" then means one of: add the prefix map, add a missing define, add `-Wno-error` for a `-Werror` Makefile that trips a GCC 14.3 warning, or pass a documented build option. Record the attempt in the drop log whether or not it works.

**Effective flags gate (new, because the measured failure modes are silent).** For every captured object run `arm-none-eabi-readelf -A` and require `Tag_CPU_arch: v7E-M`, `Tag_THUMB_ISA_use: Thumb-2`, `Tag_FP_arch: VFPv4-D16`, `Tag_ABI_VFP_args: VFP registers`; also record `Tag_ABI_optimization_goals`. `Aggressive Size` appears when the last `-O` is `-Os`. A host `cc` object fails the machine check (`ELF64`, x86-64). http-parser's Makefile appends `-O3` after the caller's flags, so its object lacks `Aggressive Size`: record it, do not fail it (D-05 says upstream defaults apply; D-06's `-Os` is a baseline, not an override), and let the manifest carry an optional override such as `CFLAGS_FAST=$(CFLAGS) -Os` when the founder wants uniform `-Os`. [VERIFIED: readelf output on zlib objects built with and without the arch flags; OPT warning fired for http-parser only]

## Seeds (BUILD-03)

**Sources per target** (all paths at the pinned commits; counts are greps, not extractions, so treat as estimates). [VERIFIED: file listings and greps; counts are estimates]

| id | Real inputs available | Estimated candidates | Hand-made needed |
|----|-----------------------|----------------------|------------------|
| tinycbor | `tests/parser/tst_parser.cpp`: `raw("\x..")` rows | about 490 string literals | no |
| zcbor | `tests/cases/*.cborhex` (18 hex-text files), byte arrays in `tests/*/*/src/*.c` | 18 files + about 107 arrays | no |
| qcbor | `test/not_well_formed_cbor.h` (about 122 entries), arrays and `\x` strings in `test/*.c` | about 120 + 120 + 280 | no |
| nanopb | `tests/fuzztest/sample_data/*.pb` (2), `corpus.zip` (3672 members, 28.5 MB), `regressions.zip` (7), `\x` literals in `tests/` | 9 plus 176 literals, thousands with zip | no (zip optional) |
| cjson | `tests/inputs/test1..11` (+ `.expected`), `fuzzing/inputs/*` (14) | 21 + 14 files | no |
| corejson | `test/unit-test/*.c` string literals, `.clusterfuzzlite/` (5 files) | about 19 | no |
| coremqtt | byte arrays in `test/unit-test/*.c` (CMock/Unity submodules absent) | about 6 arrays | yes, to reach 10 |
| paho | `MQTTPacket/test/test1.c` round-trips only | 0 | **yes** |
| microcoap | none | 0 | **yes** |
| mbedtls | `framework/data_files/` in the tarball: 623 files, 475 `.der`/`.crt`, 4.8 MB | hundreds (DER certificates) | no |
| nimble | byte arrays in `nimble/host/test/src/*.c` | about 39 arrays | no |
| libfdt | `dtc/tests/*.dts` need the dtc binary to become blobs; none committed | 0 | **yes** (hand-assembled blobs) |
| httpparser | `test.c` `requests[]`/`responses[]` with `.raw=` | about 77 | no |
| isotp | none | 0 | **yes** |
| lz4 | test data lives in GPL-2.0-or-later directories | 0 permissible | **yes** |
| zlib | `test/infcover.c`, `test/example.c` | about 6 | **yes** |

Six or seven targets need hand-made seeds (at least 10 each, marked `proposed` per BUILD-03). Produce them with committed scripts under `research/testbed-seeds/` (Python is allowed there only); commit the resulting files and an index with `method = "hand_made"`, `proposed = true` and the script path; source ideas: RFC 7252 section 4 message examples for microcoap, MQTT 3.1.1 packet examples for Paho and coreMQTT, ISO 15765-2 single/first/consecutive/flow-control frames for isotp-c, hand-assembled FDT headers (magic `0xd00dfeed`, version, offsets) for libfdt, the LZ4 frame specification examples for lz4, and streams produced with Python's `zlib` for zlib.

**Licensing of seeds (D-16).** Seeds are copies of upstream test data and carry the target's licence. Only take them from permissively licensed directories: lz4's repository is GPL-2.0-or-later outside `lib/`, and dtc's `tests/` are GPL; both are therefore hand-made. Write `testbed/seeds/<id>/NOTICE` with the SPDX identifier and the upstream licence file path.

**Extracting byte arrays from C test sources in Rust (no Python in the tool).** A small lexer in `crates/mt-capture/src/seeds/lexer.rs`, about 200 lines, is justified because no crate gives "constant byte arrays and string literals from C and C++ test files" without a full parser:
1. Skip comments, preprocessor lines (but keep `#define NAME {..}` bodies), and string/char literals when scanning for braces.
2. Brace initialisers: take each innermost `{ ... }` whose elements are all integer literals (decimal, hex, octal, char constants such as `'A'`), each in 0..=255; one seed per group; optionally accept `(uint8_t)` casts; skip groups containing identifiers or negative values.
3. String literals: concatenate adjacent literals; apply C escapes exactly: `\x` consumes **all** following hex digits (a value above 255 rejects the literal), octal up to three digits, `\n \r \t \\ \' \" \0 \a \b \f \v`; skip wide, `u8`, `u`, `U` prefixes; accept C++ raw strings only if needed.
4. Hex-text files (`*.cborhex`): ignore whitespace and an optional `0x` prefix, decode pairs.
5. Zip members: read with the `zip` crate using `enclosed_name()` semantics; never write members to disk by name.
Every seed records `source.path`, `source.line` or member name, the target commit and `method` in `{file_copy, c_array, c_string, hex_text, zip_member, hand_made}`.

**Caps and selection (D-16, proposed values).** `max_seeds = 200`, `max_total_bytes = 1 MiB` per target, `max_seed_bytes = 64 KiB`; zero-length input allowed once. Deterministic rule: collect candidates; drop duplicates by sha256 keeping the first in order; order sources by (`file_copy`, `hex_text`, `c_array`, `c_string`, `zip_member`) then path bytes; walk the source files round-robin (one candidate from each file in path order per round, candidates within a file in offset order) so one large test file cannot fill the cap; stop at the first cap reached; record skipped counts by reason (`duplicate`, `too_large`, `over_cap`, `unsupported`) in the index. Seeds go to `testbed/seeds/<id>/<sha256>` with no extension, plus `index.json`:

```json
{ "schema_version": 1, "target": "qcbor", "commit": "930708bb...", "licence": { "spdx": "BSD-3-Clause", "file": "LICENSE" },
  "caps": { "max_seeds": 200, "max_total_bytes": 1048576, "max_seed_bytes": 65536 },
  "seeds": [ { "sha256": "...", "bytes": 7, "entries": ["QCBORDecode_Init"], "method": "c_array",
               "source": { "path": "test/qcbor_decode_tests.c", "line": 214 }, "proposed": false } ],
  "skipped": { "duplicate": 5, "too_large": 0, "over_cap": 40, "unsupported": 3 } }
```

`seeds` is sorted by sha256; nothing in the index depends on the clock. Total repository growth is bounded by 16 MiB and in practice a few hundred KiB.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Splitting CMake `command` strings | a quote-aware splitter | `shlex::split` | CMake escapes with backslashes and quotes; edge cases (embedded quotes in `-D` values) are easy to get wrong |
| Fetching sources | an HTTP/git client | `git` (and `curl` for the Mbed TLS archive) through the runner | Pinned by SHA; hardened flags already exist in `fetch.rs` |
| Extracting `tar.bz2` | a bzip2 decoder | the image's `tar -xjf` through the runner (after adding `bzip2`) | Fewer dependencies; extraction flags `--no-same-owner --strip-components=1` |
| ELF section listing | a custom parser | `object` (read-only features) or `readelf -S` | Bounds-checking untrusted objects |
| Object normalisation | rewriting ELF files | `arm-none-eabi-objcopy --strip-debug --remove-section=.comment` | The authoritative tool, named in D-21 |
| Arch and optimisation checks | decoding `.ARM.attributes` | `arm-none-eabi-readelf -A` text with four regexes | Stable text, one place to change |
| Parsing clang's AST | a C declaration parser | `clang -Xclang -ast-dump=json` with `serde_json` streaming | D-23 names it |
| Compile-command discovery | scanning Makefiles | Bear (Make) and `CMAKE_EXPORT_COMPILE_COMMANDS` (CMake) | D-07, BUILD-01 |
| C test-vector extraction | regexes over `.c` files | the small lexer above | Regexes break on comments, macros and `\x` greediness; the lexer is the one justified custom piece |
| Hashing | any digest code | `sha2` | Already a dependency |

**Key insight:** the capture and replay logic is small. The cost and risk are in the build recipes (one per target) and in silent wrong builds, so spend effort on the manifest, the gates and the fixtures, not on clever machinery.

## Common Pitfalls

### Pitfall 1: `-DCMAKE_C_FLAGS=...` silently drops the toolchain flags
**What goes wrong:** zlib's "no-gzip" variant built with `-DCMAKE_C_FLAGS=-DNO_GZIP` had `C_FLAGS = -DNO_GZIP -Os -DNDEBUG` in `flags.make`: no `-mcpu`, no `-mthumb`. The build succeeded and produced objects for the default Arm target (`Tag_CPU_name: "7E-M"` appeared only after the fix).
**Why:** a cache entry set on the command line replaces `CMAKE_C_FLAGS_INIT`, which is only the initial value.
**How to avoid:** generate a per-configuration toolchain file and put baseline plus config defines in `CMAKE_C_FLAGS_INIT`; run the effective-flags gate (Reproduction section) on every object.
**Warning signs:** `flags.make` lacks `-mcpu=cortex-m4`; `readelf -A` shows no `Tag_FP_arch: VFPv4-D16`.

### Pitfall 2: The host compiler builds the "ARM" target
**What goes wrong:** `make libqcbor.a` without `CC=` ran the host `cc` (`unrecognized command-line option '-mthumb'` here; on a host that accepts the flags it would silently produce x86-64 objects). A CMake project built without the toolchain file does the same.
**How to avoid:** allow-list `argv[0]` to the pinned `arm-none-eabi-gcc`; check `ELF32`/`Machine: ARM` on every object. [VERIFIED: failure observed with the first harness]

### Pitfall 3: CMake's build type appends `-O3` after `-Os`
**What goes wrong:** GNU's `CMAKE_C_FLAGS_RELEASE_INIT` is `-O3 -DNDEBUG` and is appended after the toolchain file's value; the last `-O` wins. Setting `CMAKE_C_FLAGS_RELEASE_INIT` in the toolchain file does not replace it. **How to avoid:** pass `-DCMAKE_BUILD_TYPE=MinSizeRel` (flags `-Os -DNDEBUG`, observed as `-Os -Os -DNDEBUG`), or leave the type empty; record `Tag_ABI_optimization_goals`.

### Pitfall 4: CMake's `compile_commands.json` lists targets that were never built, and `output` is not relative to `directory`
**What goes wrong:** Paho exports 38 entries for 7 built objects; tinycbor exports `tools/cbordump/cbordump.c`; zcbor exports the sample's `main.c` (which needs a generated header). A replay of all entries fails or compiles code that was never part of the build. The `output` field of a sub-directory target is relative to the top-level build directory.
**How to avoid:** take the output from `-o`, keep entries whose output exists after the build, and record the exported-versus-built counts.

### Pitfall 5: A Makefile appends its own optimisation level after yours
**What goes wrong:** http-parser's Makefile has `CFLAGS_FAST = $(CFLAGS) -O3 $(CFLAGS_FAST_EXTRA)`; the object lacks `Aggressive Size` even with `-Os` first. Mbed TLS has `CFLAGS ?= -O2` and lz4 `CFLAGS ?= -O3`: an `-Os` placed only in `CC` loses to those defaults. **How to avoid:** inject the baseline through `CC`, pass `CFLAGS=-Os` where the default is `?=`, record the effective level, and let the manifest carry a per-target make variable when uniform `-Os` is wanted.

### Pitfall 6: Command-line `CFLAGS=` discards the Makefile's own flags
**What goes wrong:** `make CFLAGS="..."` overrides `CFLAGS += -Wall -DDEBUG` in microcoap's Makefile, so the "upstream default" configuration silently lacks `-DDEBUG`. **How to avoid:** put the baseline in `CC="arm-none-eabi-gcc <baseline>"` and leave `CFLAGS` alone, except where the Makefile uses `?=`.

### Pitfall 7: Bear can fail silently
**What goes wrong:** with the loopback interface down Bear writes no entries while `make` succeeds (Build Capture Mechanics). **How to avoid:** require at least one entry and that every object created during the build is a declared output; run capture under `docker run --network none` (loopback up) in CI, never under a bare namespace.

### Pitfall 8: Entries without `-o`, entry order under `-j`
**What goes wrong:** lz4's Makefile runs `arm-none-eabi-gcc ... -c lz4.c` in `cachedObjs/<hash>/`, so the output is `lz4.o` in `directory` and Bear emits no `output`; code that requires `-o` crashes (it did in the harness). Bear's entry order changes between parallel runs. **How to avoid:** default output rule (`<stem>.o` in `directory`); `-j1`; sort the canonical file.

### Pitfall 9: The image cannot run this phase, and only `main` publishes
**What goes wrong:** the plan assumes `git` and `cmake` exist; they do not (Dockerfile lists above). The `container` workflow's `publish` job skips every run that is not a push to `main` (`container/README.md`, "Only a push to `main` publishes (WR-09)"), so a branch-built image cannot be pulled by digest.
**How to avoid:** make the image change a separate, early change merged to `main` (a quick task or the first plan of the phase with an early `/gsd-ship`), then record the new digest under `## Current image` in `container/README.md`; the `testbed` workflow and `manifest.toml [image]` read it from there. The container workflow builds the image twice and takes the longest of any CI job in the repository, so start it first.

### Pitfall 10: Mbed TLS from git does not build
**What goes wrong:** on the git tag, `make -C library static` stops with `No rule to make target '../framework/scripts/generate_ssl_debug_helpers.py', needed by 'ssl_debug_helpers_generated.c'` (the generated files such as `library/error.c` are committed, but their generator scripts live in the `framework` submodule), and `cmake` stops because `framework/CMakeLists.txt` is missing, after running `scripts/config.py` (Python) and failing with `No module named 'mbedtls_framework'`. [VERIFIED: both run on the pinned tag] **How to avoid:** use the release tarball, pinned by archive sha256 and tree sha256 (Table 1), which contains `framework/` and the generated `library/error.c`; add `curl` and `bzip2` to the image.

### Pitfall 11: Per-target recipe traps (all measured)
- nanopb: `find_program(protoc)` is unconditional; pass `-Dnanopb_PROTOC_PATH=/usr/bin/true -Dnanopb_BUILD_GENERATOR=OFF` (`/usr/bin/true` is coreutils).
- coreMQTT: add `-DSTANDALONE_TEST_BUILD_UNIX=ON` or supply a `core_mqtt_config.h`.
- NimBLE: the `dummy` example Makefile hard-codes `CC := ccache ...gcc` and `-m32` and includes hosted pieces. Override `CC`, `NIMBLE_CFG_MBEDTLS=` (empty), `NIMBLE_CFLAGS`, `NIMBLE_LDFLAGS=` and name the `.o` goals; skip `ble_sm_alg.c` (needs Mbed TLS or tinycrypt headers) and `nimble/transport/socket`.
- libfdt: goal `libfdt/libfdt.a`, not `libfdt` (the latter links a shared library and fails with a VFP-ABI mismatch).
- Paho: top-level `set(CMAKE_BUILD_TYPE Debug)` and `SHARED` libraries; target `MQTTPacketClient`.
- zcbor: the library has no top-level build; use `samples/pet` target `pet`, not `app` (the app needs the Python `zcbor` tool for `pet1.h`).
- tinycbor: `WITH_FREESTANDING=ON` fails (`PRIu64`).

### Pitfall 12: The entry check is easy to get wrong
- A whole-translation-unit `-ast-dump=json` is 11.5 MB for `cborparser.c` and 32 MB for `http_parser.c`; the runner stores at most 16 MiB per stream and sets `truncated`, which makes the JSON unparseable. Use the probe translation unit (Pattern 9; 3.9 KB for `http_parser_execute`).
- `-ast-dump-filter=name` matches **substrings** (`cbor_parser_init` also dumped `cbor_parser_init_reader`), so select by exact `name`.
- The output is several JSON documents back to back, not one array; stream-parse.
- In a document, `loc.file` appears only when it changes from the previous location printed; track the last file when reading later documents.
- Clang rejects GCC-only `-f` and `-W` options; filter flags (Pattern 9).
- No match means empty output and exit 0; treat as missing.
- The sysroot flag is required (`--sysroot=/opt/arm-gnu-toolchain/arm-none-eabi`) so newlib headers resolve; clang 16 is [ASSUMED] to behave like clang 18 here.

### Pitfall 13: `.mt-work/` inside the repository breaks an existing test
`tool_01_no_python_outside_research` walks the repository and skips only `.git`, `target` and `research` directly under the root (`crates/mt-toolchain/tests/repo_checks.rs`, constant `ROOT_SKIPS`). A fetched zcbor tree contains `zcbor/zcbor.py`, so running `cargo test` after a local fetch (or in a CI job that kept the work directory in the checkout) fails the test. **How to avoid:** add `.mt-work` to `ROOT_SKIPS` with a comment (it is git-ignored third-party content, not tool code), or keep the work directory outside the checkout in CI. [VERIFIED: `repo_checks.rs:20-22` quoted: `const ROOT_SKIPS: &[&str] = &[".git", "target", "research"];`]

### Pitfall 14: Report determinism
Run records carry the sha256 of stdout and stderr. `make -jN` interleaves output; CMake 3.28.3 prints elapsed times in "Configuring done (0.2s)"; whether the image's 3.25.1 does is [ASSUMED] no. Use `-j1`, and add a CI step that runs capture twice and diffs the reports so a volatile stream is found at once rather than in Phase 11. `RunRecord` has no `Deserialize` (Standard Stack).

### Pitfall 15: Container user, ownership and git
`--user 65534:65534 --read-only` needs a world-writable work mount (`chmod 0777`, as Phase 1 does for `/out`); git refuses repositories owned by another uid ("dubious ownership") if a later step runs as a different user, so run every step as the same uid; set `HOME` (the runner provides none).

### Pitfall 16: Symlinks, modes and attributes in the tree hash
nanopb and NimBLE contain symlinks; a copy that follows them changes the hash and can escape the tree. A repository with `.gitattributes` `eol` rules could check out differently on another platform; the reference tree hashes were computed on Linux and CI must reproduce them.

### Pitfall 17: Tag object versus commit
`git ls-remote --tags --refs` returns the annotated tag object. Always resolve with `^{}`, then verify with `git rev-parse HEAD` after the fetch.

### Pitfall 18: Mixed-licence repositories
lz4 (BSD-2-Clause in `lib/`, GPL-2.0-or-later elsewhere) and dtc (libfdt dual-licensed, the rest GPL) are fetched in full into `.mt-work` (not redistributed), but seeds and any committed fixture may only come from the permissively licensed directories. Record `licence.file` as the library directory's file and add a path allow-list per target for seeds.

### Pitfall 19: Compiler differences between the proxy and the image
GCC 14.3 may add warnings that `-Werror` Makefiles (libfdt, http-parser) turn into errors; Arm's newlib may differ from the xPack one in headers (`PRIu64` under `-ffreestanding` failed here). One fix attempt then drop (D-11) is the designed path; the first CI run will show which targets need it.

### Pitfall 20: Scope against the sizing rule
The phase has an image change, a manifest checkpoint, a fetch, capture, replay, an entry check, seeds with hand-made sets for 6-7 targets, a CI workflow and fixtures. That is at the top of "1-2 weeks of part-time work". Cut order if needed: `--relocate`, nanopb zip support, `seeds --check`, the second configuration of low-value targets, then the founder-approved target count down toward 12.

## Code Examples

Illustrative; the shell and clang commands were run, the Rust sketches were not compiled.

### Canonicalise one entry (Rust sketch, no `unwrap`)
```rust
// Source: pattern verified with the Python harness in this session; shlex = 2.0.1 (cargo info).
fn canonical(e: &RawEntry) -> Result<CcEntry, CcError> {
    let argv: Vec<String> = match (&e.arguments, &e.command) {
        (Some(a), _) => a.clone(),
        (None, Some(c)) => shlex::split(c).ok_or(CcError::Unsplittable)?,
        (None, None) => return Err(CcError::NoCommand),
    };
    let out = match argv.iter().position(|a| a == "-o") {
        Some(i) => argv.get(i + 1).ok_or(CcError::DanglingO)?.clone(),
        None => format!("{}.o", Path::new(&e.file).file_stem().and_then(|s| s.to_str()).ok_or(CcError::BadFile)?),
    };
    let abs = if Path::new(&out).is_absolute() { PathBuf::from(&out) } else { Path::new(&e.directory).join(&out) };
    Ok(CcEntry { directory: e.directory.clone(), file: e.file.clone(), arguments: argv, output: abs })
}
```

### Effective-flags gate (regexes over `arm-none-eabi-readelf -A`)
```text
Tag_CPU_arch: v7E-M          Tag_THUMB_ISA_use: Thumb-2
Tag_FP_arch: VFPv4-D16       Tag_ABI_VFP_args: VFP registers
Tag_ABI_optimization_goals: Aggressive Size     (present when the last -O was -Os)
```
Output observed for a correct object (verbatim lines): `Tag_CPU_arch: v7E-M`, `Tag_THUMB_ISA_use: Thumb-2`, `Tag_FP_arch: VFPv4-D16`, `Tag_ABI_VFP_args: VFP registers`, `Tag_ABI_optimization_goals: Aggressive Size`. [VERIFIED: readelf of a zlib object]

### Entry probe and AST filter (clang through the runner)
```bash
# entry_probe.c contains only:  #include "http_parser.h"
clang --target=arm-none-eabi --sysroot=/opt/arm-gnu-toolchain/arm-none-eabi -fsyntax-only \
  -Xclang -ast-dump=json -Xclang -ast-dump-filter=http_parser_execute \
  -I. -DHTTP_PARSER_STRICT=0 -mcpu=cortex-m4 -mthumb -mfloat-abi=hard -mfpu=fpv4-sp-d16 entry_probe.c
# -> one FunctionDecl, type.qualType = "size_t (http_parser *, const http_parser_settings *, const char *, size_t)", 3.9 KB
clang ... -MM http_parser.c        # -> "http_parser.o: http_parser.c http_parser.h"  (header is a dependency)
```
[VERIFIED: clang 18, xPack sysroot]

### Three container steps (CI)
```bash
run() {  # $1 = network flag(s), rest = mt args
  docker run --rm "$@" --user 65534:65534 --read-only --tmpfs /tmp:rw,exec,nosuid,size=2g \
    -v "$RUNNER_TEMP/bundle/mt:/usr/local/bin/mt:ro" -v "$PWD/container/pins.toml:/opt/mt/pins.toml:ro" \
    -v "$PWD/testbed:/opt/mt/testbed:ro" -v "$RUNNER_TEMP/work:/work" -v "$RUNNER_TEMP/out:/out" "$IMAGE" \
    mt testbed ...   # fetch (default network) | capture | replay (add: --network none)
}
```

### The D-26 tiny project (no Docker, no network)
Two C files and a generated `compile_commands.json` pointing at a `tempfile` directory, with `cc` allowed by a test-only allow-list; build it through `runner::run`, capture, wipe, replay, and assert equality; then flip one source byte and assert the replay reports `differs`. For the normalisation tests, make a host object pair that differs only in `.comment` with the host `objcopy --update-section`/`--add-section`, and commit two tiny Arm `.o` fixtures taken from CI.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `-fdebug-prefix-map` for `.debug_*` only | `-ffile-prefix-map` also remaps `__FILE__` and debug paths | GCC 8 [ASSUMED] | One flag was enough in every experiment here [VERIFIED] |
| `__DATE__`/`__TIME__` break reproducibility | `SOURCE_DATE_EPOCH` honoured | GCC 7 | The runner already sets it; `__TIMESTAMP__` is the gap |
| tinycbor built by a Makefile | tinycbor v7.0 builds with CMake (only `Makefile.configure`, `Makefile.nmake` remain) | v7.0, 2026-02 | CMake path for tinycbor |
| nanopb 0.4.x | `master` is 1.0.0-dev | 2026 | Pin the 0.4.9.2 tag |
| Mbed TLS 3.x single repository | 4.x splits crypto into TF-PSA-Crypto; 3.6 is the long-term branch | 4.0 [ASSUMED] | Pin 3.6.7 (tags `mbedtls-4.0.0` to `mbedtls-4.2.0` exist, [VERIFIED: git ls-remote]) |
| GCC 13 | GCC 14 makes implicit function declarations, `int` conversions and incompatible pointer types errors by default [ASSUMED from the GCC 14 porting notes] | GCC 14 | Old code (microcoap 2016, http-parser 2020, isotp-c 2022) still compiled with xPack 14.2.1 |

**Unmaintained but kept:** `nodejs/http-parser` (last commit 2020-03-24; its README does not mention a successor), microcoap (2016) and isotp-c (2022) are kept because they are widely copied into firmware and still compile with GCC 14; the founder may prefer actively maintained replacements (libcbor and yajl are the verified backups, neither an HTTP or CoAP parser).

## Assumptions Log

Everything not listed here was verified in this session (tool run or file read) or cited. These need the founder's confirmation or a CI run before they become locked.

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Debian bookworm versions at snapshot `20261005T000000Z`: cmake 3.25.1-1, git 1:2.39.5-0+deb12u3 or newer, make 4.3-4.1, plus bzip2 and curl versions | Standard Stack | Pin edits only; the `pin-discovery` workflow prints the real values |
| A2 | Bear 3.1.1 (image) behaves like 3.1.3 (here): arguments format, `-MM` filtering, `output` only with `-o`, gRPC over loopback | Build Capture Mechanics | Capture recipe or gates need adjusting at the first CI run |
| A3 | Arm GNU 14.3.rel1 and its newlib compile the 16 targets like xPack 14.2.1 | Proposed Targets | Some targets or variants drop (designed path, D-11); bed stays above 10 only if at most 6 fail |
| A4 | clang 16's bare-metal driver accepts `--target=arm-none-eabi --sysroot=<arm>/arm-none-eabi` and prints the same `qualType` strings as clang 18 | Pattern 9 | Manifest signatures may need per-version normalisation |
| A5 | Debian's `clang-16` depends on `binutils`, so a host `ar` exists (QCBOR's Makefile hard-codes `ar -r`); Ubuntu's `clang-18` does | Pattern 5 | QCBOR needs `AR`-independent handling or `binutils` in the image |
| A6 | A networked `docker run` can reach github.com from a hosted runner and from the image (CA bundle present) | Build Capture Mechanics | Fetch step fails; add a smoke step |
| A7 | Runner speed is similar to this sandbox (4 cores): 10-15 minutes end to end | Pattern 14 | Raise `timeout-minutes` |
| A8 | Upstream tags and commits stay fetchable by SHA (no force-pushes or deletions) | Proposed Targets | A target drops with cause `fetch_failed`; commit pins protect integrity, not availability |
| A9 | The exact `objcopy --strip-debug --remove-section=.comment` normalisation produces equal outputs for two objects that differ only in those sections; not exercised on real data | Pattern 8 | Adjust the command during the offline fixture test |
| A10 | `-frandom-seed`-class nondeterminism does not occur for C objects without LTO/profile flags | Reproduction | A target with such flags fails replay and is dropped |
| A11 | CMake 3.25.1 writes `compile_commands.json` in the same `command` form as 3.28.3 | Build Capture Mechanics | Canonicaliser handles both forms anyway |
| A12 | GCC 14 defaults (implicit declarations etc. are errors) and `-ffile-prefix-map` since GCC 8 | State of the Art | Informational |

## Open Questions

1. **Dual-licensed targets (D-02).** Mbed TLS, Paho and libfdt offer a permissive option next to a copyleft one. *Known:* each has an unambiguous permissive grant (Apache-2.0, EDL-1.0 which is BSD-3-Clause text, BSD-2-Clause). *Unclear:* whether "permissive licences only" admits an `OR` expression with an elected permissive option. *Recommendation:* admit, record `spdx` and `elected`, and decide at the D-03 checkpoint; record the reading as a PROJECT.md decision.
2. **TLV and OTA-image-header categories (D-04).** No standalone permissive library with an upstream Make/CMake build was found for either as a dedicated target. *Known:* the structural stand-ins in Table 1; MCUboot needs a testbed-authored port. *Recommendation:* approve the stand-ins for Phase 2 and add MCUboot (v2.4.0, with a small committed port directory) as a documented, separately approved extra only if the founder wants a literal OTA/TLV target, accepting a departure from D-07 for that one target.
3. **Struct-wrapped buffers as "equivalent typedef" (D-24).** QCBOR (`UsefulBufC`), coreMQTT (`MQTTPacketInfo_t`), nanopb (`pb_istream_t` plus descriptor) and zlib (`z_stream`) have no direct pointer-plus-length parser besides helpers. *Recommendation:* accept `wrapped` entries for those four with an explicit `accepted_wrapped = true` in the manifest; otherwise QCBOR and coreMQTT drop and the bed is 14.
4. **Drop granularity (D-11).** The decision says a failing target or configuration is dropped. *Recommendation:* a failing variant drops only the variant (logged); a failing default configuration drops the target.
5. **Mbed TLS as an archive source (D-17).** *Recommendation:* add `kind = "archive"` to the manifest source (url, archive sha256, tree sha256, tag commit for provenance), record it as a decision, and add `curl` and `bzip2` to the image. The alternative (git plus submodule plus Python in the image) is heavier.
6. **Image change sequencing.** Only a push to `main` publishes. *Recommendation:* carve the image change out as the first change, ship it to `main` before the rest of the phase relies on a digest, and let planning proceed in parallel on the offline pieces. The phase branch strategy (one branch per phase) needs the founder's choice here.
7. **Uniform `-Os`.** http-parser forces `-O3`. *Recommendation:* record, do not fail; add a manifest override only on request.
8. **Optional extras:** nanopb `corpus.zip` support (adds the `zip` crate), the `--relocate` check, `seeds --check` in CI.
9. **Unverified proxy results.** *Recommendation:* the tracer slice runs the two smallest targets in the real image first; its report decides whether recipes need changing before the other 14.
10. **Where the digest lives.** `container/README.md` already holds it; the manifest `[image] digest` duplicates it. *Recommendation:* keep both and add `build_02_image_digest_matches_readme` (offline).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Docker daemon | `testbed` workflow, founder runs | CI yes; cloud sessions no (`failed to connect to the docker API`) | 29.8.2 client only here | none for container steps; offline `cargo test` covers logic (D-26) |
| Pinned image `ghcr.io/sat-wik/migrationtool-toolchain@sha256:42c7fb0e...` | all container steps | yes (manifest fetched anonymously) but **lacks git, cmake, curl, bzip2** | Arm 14.3.rel1, Bear 3.1.1-1, clang 16.0.6, make | image change (Pitfall 9) |
| `git` in the image | D-19 fetch | no | - | image change |
| `cmake` in the image | D-07 CMake builds (9 of 16 targets) | no | - | image change |
| `curl`, `bzip2` in the image | Mbed TLS archive | no | - | image change, or drop Mbed TLS |
| `make`, `bear`, clang, `arm-none-eabi-*` in the image | Make builds, entry check, gates | yes | see above | none |
| `git` and `cc` on the dev host | offline tests (local repo for fetch tests, host-`cc` project) | yes here (git 2.43.0, gcc 13.3.0); also on Linux CI and the Mac | - | tests skip with a clear message only if absent, never silently pass |
| Network from a cloud session | pin research, not tests | partial: `github.com`, `codeload`, `static.crates.io` reachable; Arm download host, `pkg-containers.githubusercontent.com`, Debian archives, `sourceware.org`, `bearssl.org` blocked | - | CI computes whatever cloud cannot |

**Missing dependencies with no fallback:** none beyond the image change itself.
**Missing dependencies with fallback:** all four missing image tools are covered by the single image change.

Files that change with the image (all in Phase 1 code, discovered by reading the tests): `container/Dockerfile` (apt-base list), `container/pins.toml` (`[apt]`, `[tool.cmake]`, `[tool.git]`, `[tool.make]`), `crates/mt-toolchain/src/pins.rs` (`REQUIRED_TOOLS: [&str; 10]`, `pins.rs:24-35`), `crates/mt-toolchain/tests/fixtures/pins-full.toml` and `tool-outputs.toml`, `crates/mt-toolchain/tests/container_static.rs` (the apt-tool list near line 212 and `tool_03_discovery_package_list_matches_apt_pins`, `tool_03_dockerfile_installs_every_apt_pin`), `.github/workflows/pin-discovery.yml` (the `packages="..."` line), and `container/README.md` (what the image contains, Current image). [VERIFIED: files read; `REQUIRED_TOOLS` quoted from `pins.rs:24`: `pub const REQUIRED_TOOLS: [&str; 10] = [`] Version output formats seen locally and suitable for the `version_regex` rows: `cmake version 3.28.3`, `git version 2.43.0`, `GNU Make 4.3`. [VERIFIED: local]

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness (`cargo test`), `tempfile` 3.27.0 for temporary trees |
| Config file | none beyond the workspace `Cargo.toml` (add `crates/mt-capture` to `members`) |
| Quick run command | `cargo test -p mt-capture` |
| Full suite command | `cargo test --workspace` (plus `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo deny check`) |
| Container-only checks | the `testbed` workflow (needs Docker and the new image) |

### Phase Requirements to Test Map

"Offline" = runs under plain `cargo test` in a cloud session (no Docker, no network). "CI" = needs the image; proven by the `testbed` workflow annotations.

| Req ID | Behavior (success criterion) | Test Type | Automated Command | File Exists? |
|--------|------------------------------|-----------|-------------------|-------------|
| BED-01 (SC1) | Manifest pins every target by 40-hex commit (or archive sha256) and 64-hex tree hash | unit, offline | `cargo test -p mt-capture bed_01_manifest_pins_every_target_by_commit` | Wave 0 |
| BED-01 | 10 to 20 targets in the committed manifest | unit, offline | `cargo test -p mt-capture bed_01_manifest_has_between_10_and_20_targets` | Wave 0 |
| BED-01 | Licence SPDX, elected permissive id, licence file path recorded; copyleft-only rejected | unit, offline | `cargo test -p mt-capture bed_01_manifest_rejects_copyleft_licence` and `bed_01_every_target_records_permissive_licence_and_file` | Wave 0 |
| BED-01 | At least one buffer-in entry per target; 1-3 entries | unit, offline | `cargo test -p mt-capture bed_01_every_target_has_a_buffer_in_entry` | Wave 0 |
| BED-01 | Unknown keys, duplicate ids, missing entries rejected | unit, offline | `cargo test -p mt-capture bed_01_manifest_rejects_unknown_keys` (and `_duplicate_ids`, `_missing_entry`) | Wave 0 |
| BED-01 | Every beachhead category covered | unit, offline | `cargo test -p mt-capture bed_01_every_beachhead_category_is_covered` | Wave 0 |
| BED-01 | Tree hash equals the reference vectors (files, exec bit, symlink, `.git` excluded) | unit, offline | `cargo test -p mt-capture bed_01_tree_hash_matches_reference_vectors` | Wave 0 |
| BED-01 | Fetch pins commit and tree hash; mismatch rejected; reuse verified (local `file://` repo) | integration, offline | `cargo test -p mt-capture bed_01_fetch_pins_commit_and_tree_hash` and `bed_01_fetch_rejects_tree_hash_mismatch` | Wave 0 |
| BED-01 (D-23) | Entry signature found and equal in the captured build | CI (and fixture parse offline) | testbed `capture` step; offline: `cargo test -p mt-capture bed_01_entry_ast_stream_parses_concatenated_documents` | Wave 0 |
| BED-02 (SC2) | Each target has a `default` configuration and at most one variant | unit, offline | `cargo test -p mt-capture bed_02_every_target_declares_default_and_at_most_one_variant` | Wave 0 |
| BED-02 | First-RTOS decision recorded (manifest and PROJECT.md line) | unit, offline | `cargo test -p mt-capture bed_02_first_rtos_decision_recorded` | Wave 0 |
| BED-02 | Variant changes compiled code | CI | testbed `capture` report: at least one object's code differs between configurations | n/a |
| BUILD-01 (SC3) | Make and CMake `compile_commands` canonicalise to `arguments`, sorted, built-only, correct output | unit, offline | `cargo test -p mt-capture build_01_cmake_command_strings_split_into_argv`, `build_01_default_output_is_source_stem_object`, `build_01_cmake_entries_for_unbuilt_targets_are_dropped`, `build_01_entries_sorted_canonically` | Wave 0 (fixtures from CI) |
| BUILD-01 | Compiler allow-list and arch/optimisation gates | unit, offline | `cargo test -p mt-capture build_01_rejects_non_allowlisted_compiler`, `build_01_captured_objects_are_cortex_m4_hard_float` (fixture `readelf -A` text) | Wave 0 |
| BUILD-01 | Generated toolchain file carries baseline and config defines | unit, offline | `cargo test -p mt-capture build_01_toolchain_file_carries_baseline_flags` | Wave 0 |
| BUILD-01 | Capture writes `compile_commands.json` covering every translation unit of every retained target | CI | testbed `capture` step | n/a |
| BUILD-02 (SC3) | Replay of a recorded tiny C project matches the original; a changed byte is reported | integration, offline | `cargo test -p mt-capture build_02_replay_matches_original_objects` and `build_02_replay_detects_changed_object` | Wave 0 |
| BUILD-02 | Normalisation forgives `.comment`/debug only; a `.text` difference fails | unit, offline | `cargo test -p mt-capture build_02_normalisation_only_forgives_comment_and_debug_sections` and `build_02_normalisation_rejects_text_section_difference` | Wave 0 |
| BUILD-02 | Replay restores a pristine tree and removes declared outputs first | integration, offline | `cargo test -p mt-capture build_02_clean_copy_replay_removes_stale_outputs` | Wave 0 |
| BUILD-02 | Reports are deterministic and use `BTreeMap` and `schema_version` | unit, offline | `cargo test -p mt-capture build_02_report_is_deterministic` | Wave 0 |
| BUILD-02 | Drop log has causes and at least 10 targets remain; capture/replay run with `--network none`, fetch does not | unit, offline (static check of YAML) | `cargo test -p mt-capture build_02_drop_log_keeps_at_least_ten_targets` and `build_02_workflow_runs_capture_and_replay_without_network` | Wave 0 |
| BUILD-02 | Every retained target and configuration replays identically in the image | CI | testbed `replay` step; annotation lists identical counts | n/a |
| BUILD-03 (SC4) | Seed files named by sha256; index deterministic and versioned | unit, offline | `cargo test -p mt-capture build_03_seed_names_are_sha256_of_contents` and `build_03_seed_index_is_deterministic_and_versioned` | Wave 0 |
| BUILD-03 | Lexer extracts arrays and `\x` literals per the C rules (greedy `\x`, concatenation) | unit, offline | `cargo test -p mt-capture build_03_c_byte_array_extraction_follows_c_escape_rules` | Wave 0 |
| BUILD-03 | Caps and selection are deterministic | unit, offline | `cargo test -p mt-capture build_03_seed_caps_are_enforced_deterministically` | Wave 0 |
| BUILD-03 | Provenance recorded; hand-made sets are `proposed` and at least 10 | unit, offline (over committed index files) | `cargo test -p mt-capture build_03_hand_made_seeds_marked_proposed_and_at_least_ten` and `build_03_seed_provenance_records_source_path_commit_and_method` | Wave 0 |
| BUILD-03 | Committed seeds equal regenerated seeds | CI | testbed `seeds --check` | n/a |
| (guard) | `.mt-work` does not trip the Python check; image digest in manifest equals README | unit, offline | `cargo test -p mt-toolchain tool_01_no_python_outside_research` and `cargo test -p mt-capture build_02_image_digest_matches_readme` | exists / Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p mt-capture` (target under 30 s; no test builds anything heavy)
- **Per wave merge:** `cargo test --workspace`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo deny check`
- **Phase gate:** full suite green and a green `testbed` workflow run on the branch (replay identical for every retained target, drop log committed, at least 10 retained) before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `crates/mt-capture/` crate, workspace member, `tests/` and `tests/fixtures/` (recorded Bear and CMake JSON for two small targets, `readelf -A` text, two Arm `.o` fixtures, a tiny C project)
- [ ] `testbed/manifest.toml` (after the checkpoint), `testbed/dropped.toml`
- [ ] `.github/workflows/testbed.yml`
- [ ] Image change and its tests (previous section)
- [ ] `Deserialize` derives on `RunRecord` and friends; `ROOT_SKIPS` update for `.mt-work`
- [ ] Framework install: none (cargo only)

## Security Domain

`security_enforcement` is absent from `.planning/config.json`, so this section applies. Phase 2 fetches and builds third-party code, so the build step is the sensitive part. (Sandboxing of customer builds is Phases 5 and 8; these controls are the minimum for public test-bed sources.)

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | no accounts or secrets; GHCR pulls are anonymous |
| V3 Session Management | no | - |
| V4 Access Control | yes (CI) | `permissions: contents: read`, `persist-credentials: false`, SHA-pinned actions, no secrets mounted into containers |
| V5 Input Validation | yes | `deny_unknown_fields` manifest; strict hex and path validation; clang and Bear output parsed with `serde_json`; compile argv never passed to a shell; compiler allow-list |
| V6 Cryptography | yes (integrity only) | SHA-256 via `sha2` for commits-to-tree, archives, objects and seeds; never hand-rolled |
| V12 Files and Resources | yes | archive and zip extraction without path traversal; symlinks recreated, never followed; bounded reads and output caps |
| V14 Configuration | yes | image by digest; `--network none` for capture and replay; `--read-only`, tmpfs, non-root user |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Upstream repository or tag moved or tampered | Tampering | commit SHA plus tree sha256 verified on fetch and on every reuse; archive sha256 for Mbed TLS |
| Malicious Makefile or CMake script exfiltrates data or phones home during the build | Information disclosure | build runs with `--network none`, read-only root, non-root uid, no host secrets or credentials mounted, only the work and output directories writable |
| Tar or zip path traversal ("zip slip"), absolute names, symlink escape | Tampering | `tar --no-same-owner` into a fresh directory then tree hash; zip members via `enclosed_name()` only; copy recreates symlinks and rejects targets outside the tree |
| Tampered `compile_commands.json` fixture makes replay run an arbitrary program | Elevation of privilege | allow-list `argv[0]`; argv vector, no shell; replay confined to the run directory |
| Build bomb or hang (infinite loop in a target's build) | Denial of service | runner timeouts with `timed_out` recorded as "not proven"; output cap; CI `timeout-minutes` |
| Git operations pick up hostile config or hooks from fetched trees | Elevation of privilege | `GIT_CONFIG_NOSYSTEM=1`, `GIT_CONFIG_GLOBAL=/dev/null`, no `git` run inside fetched trees after the tree hash is taken, `--no-recurse-submodules`, `GIT_ALLOW_PROTOCOL=https` |
| Licence contamination of committed seeds or fixtures | (legal) | per-directory licence allow-list; seeds only from permissive directories; `NOTICE` per target |
| Seeds assumed hidden from agents | (design) | D-14: nothing in Phase 2 may assume they are hidden; do not place seeds anywhere the later held-out store will use |

## Recommended Plan Breakdown

Sized against "1-2 weeks of part-time work": six plans, three waves, tracer first. The long poles are CI round trips (image build and publish, then the first testbed run), not code. Start Plan 1 before anything else.

| Wave | Plan | Closes | Contents | Notes |
|------|------|--------|----------|-------|
| 0 | **1. Image extension** (ship to `main` early; may be a `/gsd-quick` task like 261012-6i0) | prerequisite of BUILD-01 | Run `pin-discovery`; add `cmake git curl bzip2` to `apt-base`; `[apt]` pins for `cmake git make curl bzip2`; `[tool.cmake]`, `[tool.git]`, `[tool.make]` rows; `REQUIRED_TOOLS`; fixtures and `container_static.rs` updates; container workflow green; update `## Current image` in `container/README.md` | Only `main` publishes (WR-09). Everything that needs a digest waits for this. |
| 1 | **2. Manifest, tree hash, checkpoint** | BED-01, BED-02 | `mt-capture` skeleton and workspace member; `manifest.rs` with validation tests; `treehash.rs` with the reference vectors from the appendix; `testbed/manifest.toml` from this research (without capture results); PROJECT.md decision line for the RTOS (see below) and the key-decisions row; **`checkpoint:decision`**: present Tables 1-3, the exclusions, and open questions 1-5; apply the founder's edits | No capture work before approval (D-03). Offline only. |
| 1 | **3. Capture/replay core (offline)** | BUILD-01, BUILD-02 | `cc.rs` (shlex, canonical form, default output, filters, allow-list), `elf.rs` (sections, `readelf -A` gate), `replay.rs` and normalisation, `report.rs`, `Deserialize` on run records, `.mt-work` in `ROOT_SKIPS`, D-26 tiny-project and fixture tests | Runs in cloud sessions; can proceed while Plan 1 builds. |
| 2 | **4. Tracer in the real image** | BUILD-01, BUILD-02, BED-01 (entries) | `mt testbed fetch/capture/replay` wired in `mt-cli`; `testbed.yml` with the three container steps; targets **qcbor (Make plus Bear)** and **corejson (CMake)** only; entry check; record annotations and commit CI-captured fixtures | Decides whether the proxy results hold in the image (A2, A3, A4). Do not start Plan 5 until this is green. |
| 3 | **5. Remaining targets and drop log** | BED-01, BED-02, BUILD-01, BUILD-02 | Recipes for the other 14 targets and variants; run the full matrix; one fix attempt per failure; `dropped.toml` and `DROPPED.md`; confirm at least 10 retained; double-run determinism step | The founder-approved list may already be shorter. |
| 3 | **6. Seeds** | BUILD-03 | Lexer, selection, index, `seeds --check`; commit extracted seeds; `research/testbed-seeds/` scripts and committed hand-made sets for the 6-7 targets; `NOTICE` files; optional zip support | Can run in parallel with Plan 5 once the manifest is fixed. |

**RTOS decision text (BED-02), for `.planning/PROJECT.md`** as `- **D-17:**` and the "First RTOS ecosystem" row of Key Decisions changed from "Pending" to "Deferred (D-17)":
"First RTOS ecosystem: deferred; parsers are RTOS-independent. Candidates are Zephyr (industrial IoT) and FreeRTOS (consumer IoT). Revisit after the design-partner interviews (weeks 4-10). No Phase 2 code depends on the choice." Mirror it in `testbed/manifest.toml [decisions]`. Two more decision lines are worth recording at the checkpoint: the dual-licence reading (open question 1) and the image extension (new apt tools: `cmake`, `git`, `curl`, `bzip2`).

**Tracer slice definition.** One command sequence in CI, on two targets, proving: fetch by SHA with tree-hash check; Bear capture under `--network none`; CMake export plus filter; gates (arch, optimisation, allow-list); byte-identical clean-copy replay; entry signature check; reports and fixtures. Everything else in the phase is the same code applied to more recipes.

## Sources

### Primary (HIGH confidence)
- Local experiments on pinned commits (this session): builds, Bear and CMake captures, replays, cross-root comparisons, clang AST dumps, readelf output, `git fetch` by SHA, `git ls-remote` tag peeling, `cargo info`, `gsd-tools query package-legitimacy check`.
- Repository files read: `container/Dockerfile` (lines 104-119, 131-145, 491-506), `container/pins.toml`, `container/README.md`, `crates/mt-toolchain/src/{runner,pins,fetch,lib}.rs`, `crates/mt-cli/src/main.rs`, `crates/mt-toolchain/tests/{container_static,repo_checks}.rs`, `.github/workflows/{container,pin-discovery}.yml`, `Cargo.toml`, `deny.toml`, `clippy.toml`, `rust-toolchain.toml`, `docs/guidelines/building-the-tool.md`, `.planning/{PROJECT,REQUIREMENTS,STATE}.md`, Phase 1 and Phase 2 `CONTEXT.md`.
- Upstream licence files, headers and build files of the 16 targets at the pinned commits (listed in Table 1).
- Docker none network driver: https://docs.docker.com/engine/network/drivers/none/ (loopback only, 127.0.0.1/8)

### Secondary (MEDIUM confidence)
- cmake 3.25.1-1 in bookworm: https://aptpeek.dev/debian/bookworm/main/amd64/cmake/3.25.1-1 (third-party mirror of packages.debian.org)
- git bookworm security versions (`1:2.39.5-0+deb12u2`, `u3` dated 2025-10-07): https://tracker.debian.org/media/packages/g/git/changelog-12.39.5-0deb12u3
- GCC `SOURCE_DATE_EPOCH` and `__TIMESTAMP__` gap: https://wiki.debian.org/ReproducibleBuilds/TimestampsFromCPPMacros

### Tertiary (LOW confidence)
- Behaviour of Arm GNU 14.3.rel1, its newlib, Bear 3.1.1, CMake 3.25.1 and clang 16 (not reachable from this session; see Assumptions Log A2 to A5).

## Metadata

**Confidence breakdown:**
- Standard stack: MEDIUM. Rust side HIGH (versions from `cargo info`, workspace read); image side MEDIUM (what is missing is read from the Dockerfile; exact Debian versions are assumed until `pin-discovery` runs).
- Architecture: HIGH for mechanics (each pattern was exercised), MEDIUM for the image-specific behaviours (A2, A3, A4, A6).
- Target list: HIGH that the commits, licences and signatures are right and that the proxy builds replay identically; MEDIUM that all survive the image's compiler.
- Pitfalls: HIGH (every item was observed, except the ones tagged [ASSUMED]).

**Research date:** 2026-10-12
**Valid until:** 2026-11-11 for mechanics; re-check upstream tags and the image digest if planning slips (the pins are exact, so only availability and the image contents can change).

