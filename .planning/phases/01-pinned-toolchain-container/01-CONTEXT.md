# Phase 1: Pinned Toolchain Container - Context

**Gathered:** 2026-10-08
**Status:** Ready for planning

<domain>
## Phase Boundary

One reproducible linux/amd64 container image in which every compiler and analyser the harness needs (clang/LLVM, KLEE, c2rust, Bear, arm-none-eabi GCC, QEMU, cargo-mutants, the tool's Rust toolchain and a separate KLEE bitcode Rust toolchain) is pinned, on a single LLVM major. Plus the first workspace code: the `mt-toolchain` subprocess runner, an `mt toolchain check` command that verifies every pin, a trivial `no_std` crate built for `thumbv7em-none-eabihf`, and a repository check that rejects Python outside `research/`. Requirements TOOL-01..TOOL-04; success criteria 1-4 in `.planning/ROADMAP.md` Phase 1.

Not in this phase: running Rust bitcode through KLEE (Phase 6 R1 spike), Hayroll and Kani (Phase 8 licence gate / milestone v2.0), network sandboxing of customer builds (Phases 5/8), any test-bed capture (Phase 2).

</domain>

<decisions>
## Implementation Decisions

Decision IDs below are phase-local (D-01..). Project-level decisions are cited as "PROJECT D-xx".

### LLVM pin
- **D-01:** Provisional pin is **LLVM 16** for clang, KLEE and c2rust (KLEE 3.2's only fully supported LLVM; best odds for R1). Recorded as provisional until the Phase 6 R1 verdict (PROJECT D-08). — **Reversibility:** costly — every C-side tool, the bitcode rustc and the image are rebuilt; later phases' recorded outputs would carry a different LLVM.
- **D-02:** If research finds a required tool cannot run on LLVM 16, move the pin to the nearest major in 16-19 that all tools support and record it as a decision. Never split LLVM versions across tools.
- **D-03:** The bitcode rustc is checked by version only in Phase 1: `rustc -vV` must report `LLVM version: 16.x` (matching the pin, and in any case ≤ 19). Compiling Rust to bitcode and loading it into KLEE is out of scope — that is the R1 spike. The researcher must confirm which exact rustc release ships LLVM 16 (believed to be around 1.70-1.72; verify against release notes).

### Reproducibility method
- **D-04:** The image is a multi-stage **Dockerfile**: base image pinned by `@sha256` digest; apt installs from a fixed `snapshot.debian.org` date (no floating archives); every downloaded tool pinned by version/commit plus sha256. No `latest` tags, no unpinned `apt-get`, no `curl | sh` without a checksum.
- **D-05:** **Docker only** is supported in Phase 1. The output is a standard OCI image, so other runtimes may work but are not tested.
- **D-06:** The reproducibility bar for TOOL-03 is **manifest-identical**: building the image twice from the same inputs yields a byte-identical tool-version manifest (names, versions, source refs, source hashes). Binary-hash identity of installed tools is not required.

### Tool sourcing
- **D-07:** **KLEE 3.2** is built from its pinned git tag in a dedicated build stage against the image's LLVM 16 packages (with its solver and klee-uclibc pinned the same way); only the install tree is copied into the final image. Do not base on the `klee/klee` image.
- **D-08:** **arm-none-eabi GCC** comes from an official **Arm GNU Toolchain** release tarball (a specific `x.y.Rel1`), sha256-verified. The researcher picks the release; prefer the most recent one that firmware teams commonly ship.
- **D-09:** **c2rust** is installed with `cargo install --locked` at a pinned release version, built against LLVM 16 in a build stage.
- **D-10:** **Hayroll and Kani are not installed.** The manifest lists them with status `not_installed`; the check does not fail on their absence.
- **D-11:** Bear, QEMU (user-mode Arm is enough for later replay; researcher confirms which QEMU binaries) and cargo-mutants are pinned the same way (snapshot apt or `cargo install --locked` with exact version).

### Rust toolchains
- **D-12:** Both Rust toolchains are installed via **rustup** (installer verified by sha256) as exact versions: the tool toolchain from `rust-toolchain.toml`, and the bitcode toolchain as a named exact version (e.g. `1.7x.0`). The bitcode toolchain is only invoked as `cargo +<ver>` / `rustc +<ver>` through the `mt-toolchain` runner, never as the default.
- **D-13:** The bitcode toolchain has the host target only (`x86_64-unknown-linux-gnu`). `thumbv7em-none-eabihf` is added to the tool toolchain, which builds the TOOL-04 `no_std` crate.

### Pins, manifest and check command
- **D-14:** A committed **`container/pins.toml`** is the single source of truth for every pin (versions, digests, sha256s, snapshot date). The Dockerfile consumes it (via build args or a generated file); nothing pins a version anywhere else.
- **D-15:** Inside the image a **JSON** tool-version manifest is generated (`schema_version`, sorted keys, no timestamps except an optional header field). `mt toolchain check` compares the live tools against `pins.toml`, prints every tool's exact version (including the bitcode rustc's LLVM from `rustc -vV`), and exits non-zero if any tool is missing or off its pin.
- **D-16:** The command is **`mt toolchain check`**, in the `mt-cli` binary, with the logic in `mt-toolchain`. Leave room for sibling subcommands (e.g. `mt toolchain manifest`).

### Subprocess runner
- **D-17:** The runner starts every subprocess with an **empty environment plus a fixed base** (`PATH`, `LANG=C.UTF-8`, `TZ=UTC`, `SOURCE_DATE_EPOCH`). Callers add named variables explicitly, and the effective env is recorded with argv, cwd, tool version and exit status.
- **D-18:** The runner records the sha256 of stdout and stderr **and stores the full output up to a size cap** (proposed 16 MiB per stream; beyond that it truncates and sets a `truncated` flag).
- **D-19:** No network isolation in Phase 1. Network-off for customer builds (`docker run --network=none` per job) belongs to Phases 5/8 and their secure-phase reviews.

### Pin bumps
- **D-20:** Pins are bumped by **hand-edited PRs to `pins.toml`**, with a helper that recomputes sha256s (prefer an `mt toolchain` subcommand; a script under `research/` is acceptable). CI rebuilds and diffs the manifest. No Renovate/Dependabot.
- **D-21:** Changing the **LLVM major or the bitcode rustc version** is a project decision: `pins.toml` carries the decision ID that justifies the current LLVM/bitcode pin, and CI fails if either changes without a matching decision entry in `.planning/PROJECT.md`. — **Reversibility:** reversible — the gate is a CI check and can be loosened by a recorded decision.

### Build location, CI and workspace scope
- **D-22:** The founder works on an **Apple Silicon Mac**. The image stays **linux/amd64 only** (no arm64 variant), per the runtime constraint.
- **D-23:** **GitHub Actions in Phase 1** builds the image on native amd64 runners, builds it a second time and diffs the two manifests (TOOL-03), runs `mt toolchain check` inside it, and runs `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` and `cargo deny check`.
- **D-24:** CI pushes the image to **GHCR, referenced by digest**. The founder pulls that digest and runs it under emulation on the Mac; a local emulated build remains a documented, slow fallback.
- **D-25:** Scaffold **only `mt-toolchain` and `mt-cli`** (plus the trivial `no_std` fixture crate for TOOL-04, kept out of the tool's own build or in a separate path). Later phases add their own crates from the layout in `docs/guidelines/building-the-tool.md` §3.
- **D-26:** The Python check is a Rust test named `tool_01_no_python_outside_research` that walks the repository (excluding `.git/` and `research/`) and fails on any `.py` file, so it runs with `cargo test` everywhere, including sessions without Docker.

### Claude's Discretion
- Exact Debian base release and snapshot date, Arm GNU Toolchain release, c2rust/Bear/QEMU/cargo-mutants versions — researcher picks, consistent with D-01 and D-04.
- `pins.toml` schema and JSON manifest field names, as long as D-14/D-15 hold.
- Whether the TOOL-04 `no_std` crate lives under `fixtures/` or `container/smoke/`.
- Runner output cap value (D-18) if 16 MiB proves impractical.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase scope and requirements
- `.planning/ROADMAP.md` — Phase 1 goal and success criteria 1-4
- `.planning/REQUIREMENTS.md` — TOOL-01..TOOL-04 wording
- `.planning/PROJECT.md` — Constraints, decisions D-01..D-14 (project-level), risk table R1-R6

### Locked architecture and toolchain constraints
- `docs/adr/0001-tool-implementation-language.md` — Rust workspace, analysers as pinned subprocesses (LOCKED)
- `docs/research/VERIFICATION.md` — R1 (LLVM version lock: KLEE 3.2 / Hayroll / c2rust / rustc limits)

### Coding rules for the tool
- `docs/guidelines/building-the-tool.md` — §3 workspace layout, §4 runner/determinism/toolchain/testing/licence rules, §6 commits
- `.planning/codebase/CONVENTIONS.md` — summary of the above
- `docs/specs/emitted-rust-rules.md` — rules the TOOL-04 `no_std` smoke crate should already respect (`no_std`, `panic = "abort"`, target)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- None. The repository has no Rust code yet; this phase creates the Cargo workspace.

### Established Patterns
- Conventions are documented but not yet coded: `#![forbid(unsafe_code)]`, `thiserror` in libraries / `anyhow` only in `mt-cli`, `BTreeMap` and `schema_version` in serialized output, requirement-ID test names.
- `.planning/config.json` already expects `cargo build --workspace` / `cargo test --workspace` as build/test commands.

### Integration Points
- `mt-toolchain`'s runner is the single path to every external tool for all later phases; its record format (argv, env, cwd, version, exit status, output hashes) becomes part of the evidence bundles in Phase 11, so design it as a stable, versioned schema.
- GHCR image digest becomes the container reference later phases and CI use.

</code_context>

<specifics>
## Specific Ideas

- Cloud Claude Code sessions (like the one that ran this discussion) have no Docker. Anything that must be verifiable there has to run as plain `cargo test` (hence D-26); container-level checks are proven by CI (D-23).
- The founder's Mac runs the amd64 image emulated; expect KLEE and builds to be slow locally. Do not design workflows that require local image builds.

</specifics>

<deferred>
## Deferred Ideas

- arm64 image variant for fast local runs on Apple Silicon — rejected for now (constraint says x86-64); revisit only with a recorded decision.
- Podman support — later, when customer on-prem installs are in scope (v2.0).
- Rust-bitcode-into-KLEE smoke test — Phase 6 R1 spike.
- Network isolation for subprocesses — Phases 5/8.
- Renovate/Dependabot for pin bumps — not wanted.

</deferred>

---

*Phase: 01-pinned-toolchain-container*
*Context gathered: 2026-10-08*
