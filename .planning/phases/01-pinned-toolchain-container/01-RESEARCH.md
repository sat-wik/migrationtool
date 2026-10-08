# Phase 1: Pinned Toolchain Container - Research

**Researched:** 2026-10-08
**Domain:** Reproducible Debian-based tool container (LLVM 16, KLEE 3.2, c2rust, Arm GNU, QEMU, two rustup toolchains) plus the first Rust workspace code (subprocess runner, `mt toolchain check`, repository checks)
**Confidence:** MEDIUM overall. Every Rust-side and registry-side pin was verified by running or fetching it this session (HIGH). The Debian/apt side and the Arm tarball could not be reached from this session (network egress blocks `deb.debian.org`, `snapshot.debian.org`, `developer.arm.com`, `gitlab.arm.com`), so those pins rest on search results, nixpkgs and KLEE/c2rust sources, and are resolved by a CI "pin discovery" step defined below (MEDIUM).

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

Decision IDs below are phase-local (D-01..). Project-level decisions are cited as "PROJECT D-xx".

#### LLVM pin
- **D-01:** Provisional pin is **LLVM 16** for clang, KLEE and c2rust (KLEE 3.2's only fully supported LLVM; best odds for R1). Recorded as provisional until the Phase 6 R1 verdict (PROJECT D-08). — **Reversibility:** costly — every C-side tool, the bitcode rustc and the image are rebuilt; later phases' recorded outputs would carry a different LLVM.
- **D-02:** If research finds a required tool cannot run on LLVM 16, move the pin to the nearest major in 16-19 that all tools support and record it as a decision. Never split LLVM versions across tools.
- **D-03:** The bitcode rustc is checked by version only in Phase 1: `rustc -vV` must report `LLVM version: 16.x` (matching the pin, and in any case ≤ 19). Compiling Rust to bitcode and loading it into KLEE is out of scope — that is the R1 spike. The researcher must confirm which exact rustc release ships LLVM 16 (believed to be around 1.70-1.72; verify against release notes).

#### Reproducibility method
- **D-04:** The image is a multi-stage **Dockerfile**: base image pinned by `@sha256` digest; apt installs from a fixed `snapshot.debian.org` date (no floating archives); every downloaded tool pinned by version/commit plus sha256. No `latest` tags, no unpinned `apt-get`, no `curl | sh` without a checksum.
- **D-05:** **Docker only** is supported in Phase 1. The output is a standard OCI image, so other runtimes may work but are not tested.
- **D-06:** The reproducibility bar for TOOL-03 is **manifest-identical**: building the image twice from the same inputs yields a byte-identical tool-version manifest (names, versions, source refs, source hashes). Binary-hash identity of installed tools is not required.

#### Tool sourcing
- **D-07:** **KLEE 3.2** is built from its pinned git tag in a dedicated build stage against the image's LLVM 16 packages (with its solver and klee-uclibc pinned the same way); only the install tree is copied into the final image. Do not base on the `klee/klee` image.
- **D-08:** **arm-none-eabi GCC** comes from an official **Arm GNU Toolchain** release tarball (a specific `x.y.Rel1`), sha256-verified. The researcher picks the release; prefer the most recent one that firmware teams commonly ship.
- **D-09:** **c2rust** is installed with `cargo install --locked` at a pinned release version, built against LLVM 16 in a build stage.
- **D-10:** **Hayroll and Kani are not installed.** The manifest lists them with status `not_installed`; the check does not fail on their absence.
- **D-11:** Bear, QEMU (user-mode Arm is enough for later replay; researcher confirms which QEMU binaries) and cargo-mutants are pinned the same way (snapshot apt or `cargo install --locked` with exact version).

#### Rust toolchains
- **D-12:** Both Rust toolchains are installed via **rustup** (installer verified by sha256) as exact versions: the tool toolchain from `rust-toolchain.toml`, and the bitcode toolchain as a named exact version (e.g. `1.7x.0`). The bitcode toolchain is only invoked as `cargo +<ver>` / `rustc +<ver>` through the `mt-toolchain` runner, never as the default.
- **D-13:** The bitcode toolchain has the host target only (`x86_64-unknown-linux-gnu`). `thumbv7em-none-eabihf` is added to the tool toolchain, which builds the TOOL-04 `no_std` crate.

#### Pins, manifest and check command
- **D-14:** A committed **`container/pins.toml`** is the single source of truth for every pin (versions, digests, sha256s, snapshot date). The Dockerfile consumes it (via build args or a generated file); nothing pins a version anywhere else.
- **D-15:** Inside the image a **JSON** tool-version manifest is generated (`schema_version`, sorted keys, no timestamps except an optional header field). `mt toolchain check` compares the live tools against `pins.toml`, prints every tool's exact version (including the bitcode rustc's LLVM from `rustc -vV`), and exits non-zero if any tool is missing or off its pin.
- **D-16:** The command is **`mt toolchain check`**, in the `mt-cli` binary, with the logic in `mt-toolchain`. Leave room for sibling subcommands (e.g. `mt toolchain manifest`).

#### Subprocess runner
- **D-17:** The runner starts every subprocess with an **empty environment plus a fixed base** (`PATH`, `LANG=C.UTF-8`, `TZ=UTC`, `SOURCE_DATE_EPOCH`). Callers add named variables explicitly, and the effective env is recorded with argv, cwd, tool version and exit status.
- **D-18:** The runner records the sha256 of stdout and stderr **and stores the full output up to a size cap** (proposed 16 MiB per stream; beyond that it truncates and sets a `truncated` flag).
- **D-19:** No network isolation in Phase 1. Network-off for customer builds (`docker run --network=none` per job) belongs to Phases 5/8 and their secure-phase reviews.

#### Pin bumps
- **D-20:** Pins are bumped by **hand-edited PRs to `pins.toml`**, with a helper that recomputes sha256s (prefer an `mt toolchain` subcommand; a script under `research/` is acceptable). CI rebuilds and diffs the manifest. No Renovate/Dependabot.
- **D-21:** Changing the **LLVM major or the bitcode rustc version** is a project decision: `pins.toml` carries the decision ID that justifies the current LLVM/bitcode pin, and CI fails if either changes without a matching decision entry in `.planning/PROJECT.md`. — **Reversibility:** reversible — the gate is a CI check and can be loosened by a recorded decision.

#### Build location, CI and workspace scope
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

### Deferred Ideas (OUT OF SCOPE)
- arm64 image variant for fast local runs on Apple Silicon — rejected for now (constraint says x86-64); revisit only with a recorded decision.
- Podman support — later, when customer on-prem installs are in scope (v2.0).
- Rust-bitcode-into-KLEE smoke test — Phase 6 R1 spike.
- Network isolation for subprocesses — Phases 5/8.
- Renovate/Dependabot for pin bumps — not wanted.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| TOOL-01 | Rust Cargo workspace runs every external analyser as a pinned subprocess inside the container, never linked; a repository check fails on any Python outside `research/` | Runner design and verified prototype (Code Examples 1); `clippy.toml` `disallowed-types` enforcement; Python-walk test (Example 2); `.claude/` has 0 `.py` files today |
| TOOL-02 | One LLVM major in 16-19 pinned for clang, KLEE, c2rust, (Hayroll); check fails on a different major or if bitcode rustc reports LLVM > 19 | LLVM 16 feasibility per tool; bitcode rustc = 1.72.1 (LLVM 16.0.5, executed); version-command/parse table; decision-ID gate design |
| TOOL-03 | Image reproducible: base, tool sources and Rust toolchain pinned by digest/commit/checksum; building twice yields identical tool-version manifest | Concrete pin table; `pins.toml` schema; Dockerfile consumption via generated build args; CI double-build design; static lints runnable in plain `cargo test` |
| TOOL-04 | Container provides arm-none-eabi GCC and `thumbv7em-none-eabihf` target; a trivial `no_std` crate builds for it inside the container | Arm GNU 14.3.Rel1 pin; verified smoke crate (builds, clippy-clean, excluded from workspace); CI-only build step |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

From `/home/user/migrationtool/.claude/CLAUDE.md` (treated as locked, same authority as CONTEXT.md):

- Tech stack LOCKED: Rust Cargo workspace; analysers (clang/LLVM, KLEE, c2rust, Hayroll, Kani, Bear) as pinned subprocesses in a reproducible container (ADR-0001).
- `#![forbid(unsafe_code)]` in every tool crate; `thiserror` in libraries, `anyhow` only in `mt-cli`; no `unwrap`/`expect` outside tests without an invariant comment.
- Every external tool runs through the `mt-toolchain` runner (argv, env allowlist, version, exit status, output hashes); never call `std::process::Command` directly (the runner module is the single exception; see Pitfall 12).
- Deterministic outputs: explicit seeds and budgets, `BTreeMap`, `schema_version`, timestamps only in manifest headers.
- Two pinned Rust toolchains: the tool's own and a KLEE bitcode toolchain whose `rustc -vV` reports LLVM <= 19. One LLVM major in 16-19 for clang, KLEE, c2rust and Hayroll.
- Each requirement ID has a test whose name contains it in snake case (e.g. `tool_01_...`); tests never use the network or a live model.
- Pre-merge: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo deny check`, container version check.
- Python only under `research/`; analysers only as subprocesses.
- Reports: timeouts are "not proven", missing layers "unavailable", never the words "certified" or "compliant".
- Emitted-Rust rules (apply to the TOOL-04 smoke crate shape): `#![no_std]`, no heap, `crate-type = ["staticlib","rlib"]`, `panic = "abort"` in dev and release, `#[panic_handler]` behind a default-on `panic-handler` feature, no dependencies, `unsafe` only in `ffi.rs`.
- Never run with `--dangerously-skip-permissions`. Edits happen through GSD commands.
- Licence allowlist for the shipped binary (`docs/guidelines/building-the-tool.md` §4): MIT, Apache-2.0, BSD-2/3-Clause, ISC, Zlib, Unicode. GPL tools (Bear) run only as container subprocesses.

## Summary

Phase 1 splits cleanly into a part that can be proven with plain `cargo test` in a Docker-less cloud session (runner, `pins.toml` parsing, version parsers, manifest comparison, Python check, decision-ID gate, Dockerfile static lints, smoke-crate shape) and a part that only CI can prove (image build, double-build manifest diff, live `mt toolchain check`, smoke build on the target inside the image). The first part should be built first as a tracer. This session verified the whole Rust side by running it: a runner prototype with four passing tests, `cargo clippy -D warnings`, a `cargo-deny 0.20.2` config that passes against the intended dependency set, a static musl `mt` binary build, and the `thumbv7em-none-eabihf` smoke crate (see Code Examples).

LLVM 16 is feasible for every tool. Debian **bookworm** ships `clang-16`, `llvm-16-dev` and `libclang-16-dev` at `1:16.0.6-15~deb12u1` [CITED: packages.debian.org bookworm clang-16, via search results], so no apt.llvm.org, no trixie/sid snapshot and no LLVM source build is needed: use `debian:bookworm-20261005-slim` pinned by digest plus a `snapshot.debian.org` timestamp. KLEE 3.2 (tag `v3.2` = commit `92ee8201...`) recommends LLVM 16 and its CI builds LLVM 16 with klee-uclibc `klee_uclibc_v1.4`; c2rust 0.22.1 builds on **stable** rustc (verified: `cargo install --locked c2rust --version =0.22.1` under 1.99.0 compiled all Rust dependencies and reached the C++ exporter's cmake step, with no nightly installed) and takes `LLVM_CONFIG_PATH`. The bitcode rustc is **1.72.1**, which `rustc -vV` reports as `LLVM version: 16.0.5` (executed this session).

Three corrections to CONTEXT.md premises that the planner must honour: (1) QEMU user mode (`qemu-arm`) cannot run bare-metal Cortex-M firmware; later phases (DIFF-06 QEMU fallback) need `qemu-system-arm` (machines `mps2-an386` Cortex-M4, `mps2-an500` Cortex-M7), so install **both** `qemu-user` and `qemu-system-arm` (D-11). (2) Bookworm's Bear is **3.1.1-1**, not 3.0.x. (3) With the runner's empty environment (D-17), rustup shims need `RUSTUP_HOME` (and a writable `CARGO_HOME`) passed explicitly, otherwise `cargo +1.72.1` fails; calling toolchain binaries by absolute path avoids this.

**Primary recommendation:** Build the Docker-free tracer first (workspace, runner, `pins.toml` schema, `mt toolchain check|manifest|build-args`, repo checks, smoke crate, `ci.yml` cargo job), then write the Dockerfile stage by stage with a CI `workflow_dispatch` that builds one `--target` at a time, then add the double-build/diff/GHCR workflow. The biggest schedule risk is not any single tool but the iteration latency of debugging KLEE/klee-uclibc/c2rust builds through CI with no local Docker.

## Architectural Responsibility Map

Adapted tiers (this phase has no browser/API/DB). "Tier" = where the capability lives.

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Subprocess launching, env control, output capture, hashing | `mt-toolchain` crate (host/in-container Rust) | — | Single choke point for all tools (ADR-0001); pure Rust, unit-testable without Docker |
| Pin storage and consumption | `container/pins.toml` (repo data) | `mt toolchain build-args` renders Docker build args | One source of truth (D-14); Dockerfile must not carry its own version literals |
| Tool installation and LLVM single-major guarantee | Container image (Dockerfile) | CI (builds it) | Reproducibility lives in the image, not in the tool |
| Version verification (`check`) and manifest | `mt-toolchain` run inside the container | `mt-cli` (argument parsing only) | Logic in library, CLI thin (building-the-tool.md §3) |
| Static pin lints (digest/format/ARG parity, decision-ID gate, Python ban, Command confinement) | Plain `cargo test` | CI | Must run in Docker-less sessions (D-26) |
| Image build, double-build diff, live check, target build of smoke crate | GitHub Actions (amd64 runners) | GHCR | Only CI has Docker (CONTEXT specifics) |
| `thumbv7em-none-eabihf` smoke build | Container (tool toolchain with target) | `container/smoke/` crate outside the host workspace | Proves TOOL-04 without disturbing `cargo build --workspace` |

## Standard Stack

### Core (container contents, all pins resolved or with a defined resolution step)

| Tool | Pin | Source / pin mechanism | Verification this session |
|------|-----|------------------------|---------------------------|
| Base image | `debian:bookworm-20261005-slim@sha256:7c7b2c966bc9ee8cedfeef67e0e279108992c77681fa595db4a9d65c06ccc587` (OCI index; amd64 manifest `sha256:a4672c0cb26fbdde88e38fa2dfb6c681942306680e41e4378b28770b6e79ee91`) | Docker Hub registry | [VERIFIED: registry-1.docker.io manifest HEAD and hub.docker.com tags API, 2026-10-08] |
| apt snapshot | `http://snapshot.debian.org/archive/debian/20261005T000000Z/` (suites `bookworm`, `bookworm-updates`) and `.../archive/debian-security/20261005T000000Z/` (`bookworm-security`) | snapshot.debian.org, `Acquire::Check-Valid-Until "false"` | [CITED: snapshot.debian.org usage per debuerreotype/Docker `debian/snapshot` description]; reachability [ASSUMED] (host blocked here) |
| clang/LLVM 16 | `clang-16`, `llvm-16`, `llvm-16-dev`, `libclang-16-dev`, `libclang-cpp16-dev` at `1:16.0.6-15~deb12u1` (expected) | apt from snapshot | [CITED: packages.debian.org/bookworm/clang-16 via search]; exact version in snapshot resolved by discovery step |
| KLEE | `v3.2`, commit `92ee8201a050184aacaaeef645ade491f5c93c41` | `git clone --depth 1 --branch v3.2` then assert `git rev-parse HEAD` | [VERIFIED: git ls-remote and clone, 2026-10-08] |
| klee-uclibc | tag `klee_uclibc_v1.4`, commit `955d502cc1f0688e82348304b053ad787056c754` (KLEE 3.2 CI uses v1.4; its Dockerfile env still says v1.3 = `7862d6f0d600f2fc47e1d66f093ce76512fb77ee`) | git | [VERIFIED: git ls-remote; KLEE `.github/workflows/build.yaml` line `UCLIBC_VERSION: klee_uclibc_v1.4`] |
| SMT solver | Z3 from Debian (`libz3-dev`/`libz3-4`, bookworm 4.8.12 expected); fallback: Z3 tag `z3-4.8.15` = `f1806d32d6f21fd4df7a08719abbc1f6493d9dc5` built from source (the version KLEE CI uses) | apt / git | tag [VERIFIED: git ls-remote]; Debian z3 compiling against KLEE 3.2 [ASSUMED] |
| c2rust | `0.22.1` (published 2026-02-11), `.crate` sha256 `e331f411600da65f3ed67033048cf1fcc203d7655974cf5bd6410da8bc653f3b` | `cargo install --locked c2rust --version =0.22.1` | [VERIFIED: index.crates.io cksum equals sha256 of downloaded `static.crates.io` file] |
| c2rust transitive native dep | tinycbor commit `d393c16f3eb30d0c47e6f9d92db62272f0ec4dc7` (v0.6.3), cloned by the exporter's cmake at build time | pinned inside c2rust 0.22.1 | [VERIFIED: c2rust-ast-exporter `src/CMakeLists.txt` at tag v0.22.1] |
| cargo-mutants | `27.1.0` (2026-06-02, MSRV 1.88), `.crate` sha256 `07072e7bcdeb425d5e5fdbfd9f15a2c749e23cb2edf5ef40aee5876760ae1cf9` | `cargo install --locked cargo-mutants --version =27.1.0` | [VERIFIED: installed here in ~1 min; `cargo mutants --version` prints `cargo-mutants 27.1.0`] |
| Bear | bookworm `3.1.1-1` (expected; version printed `bear 3.1.1`) | apt from snapshot | [CITED: packages.debian.org bear page via search]; exact snapshot version via discovery |
| QEMU | `qemu-system-arm` and `qemu-user`, bookworm `1:7.2+dfsg-7+deb12uNN` (latest seen: `deb12u18`) | apt from snapshot | [CITED: packages.debian.org/bookworm/qemu-system-arm via search] |
| Arm GNU Toolchain | **14.3.Rel1**, `arm-gnu-toolchain-14.3.rel1-x86_64-arm-none-eabi.tar.xz`, sha256 `8f6903f8ceb084d9227b9ef991490413014d991874a1e34074443c2a72b14dbd` | `https://developer.arm.com/-/media/Files/downloads/gnu/14.3.rel1/binrel/<file>`; official hash at `<file>.sha256asc` | hash from nixpkgs master `gcc-arm-embedded-14` (third-party copy of Arm's `.sha256asc`) [CITED]; official `.sha256asc` could not be fetched (host blocked) so the pin step must re-confirm it |
| rustup | `1.29.1`, `rustup-init` sha256 `dda7234360b7f578ca8b0ddcb80145646fa61a67c1720a5abc7051b35c9fcb71` (x86_64-unknown-linux-gnu) | `https://static.rust-lang.org/rustup/archive/1.29.1/x86_64-unknown-linux-gnu/rustup-init` | [VERIFIED: downloaded; sha256 matches `.sha256`; `--version` = `rustup-init 1.29.1`] |
| Tool Rust toolchain | **1.99.0** (stable 2026-10-01), commit `b940084d7eb6a299eb4bfeb8e34901bc051e7ac4`, channel manifest sha256 `ce6dddc886364f8d786514771212cebe9b731ba82d6b859951c6b0ccc516b6a2`; components rustfmt, clippy; targets `thumbv7em-none-eabihf` (+ `x86_64-unknown-linux-musl` for the static `mt`) | rustup | [VERIFIED: installed and executed; `rustc -vV` reports `LLVM version: 23.1.1`, so the tool rustc can never be the bitcode rustc] |
| Bitcode Rust toolchain | **1.72.1**, commit `d5c2e9c342b358556da91d61ed4133f6f50fc0c3`, channel manifest sha256 `77113b9660855a5ab49e5ff029c998be93fdcea41b062b40420675cf1f4ea229`, host target only | rustup `--profile minimal` | [VERIFIED: installed and executed; see table below] |

**Rust releases and their LLVM (answer to research question 1)** [VERIFIED: each toolchain installed with rustup 1.28.2 this session and `rustc -vV` executed]:

| rustc | `LLVM version:` | commit-hash |
|-------|-----------------|-------------|
| 1.69.0 | 15.0.7 | `84c898d65adf2f39a5a98507f1fe0ce10a2b8dbc` |
| 1.70.0 | 16.0.2 | `90c541806f23a127002de5b4038be731ba1458ca` |
| 1.71.1 | 16.0.5 | `eb26296b556cef10fb713a38f3d16b9886080f26` |
| **1.72.1** | **16.0.5** | `d5c2e9c342b358556da91d61ed4133f6f50fc0c3` |
| 1.73.0 | 17.0.2 | `cc66ad468955717ab92600c770da8c1601a4ff33` |

Recommendation: pin **1.72.1** (last Rust release on LLVM 16, highest 16.0.5 patch level, point release with fixes). Bitcode from LLVM 16.0.5 is read by Debian's LLVM 16.0.6 (same major). Tool rustc 1.99.0 is on LLVM 23, which is why two toolchains are required.

### Supporting (Rust crates for `mt-toolchain` / `mt-cli`)

| Library | Version | Purpose | Notes |
|---------|---------|---------|-------|
| serde (derive) | 1.0.229 | (de)serialisation | [VERIFIED: crates.io] |
| serde_json | 1.0.151 | manifest and run records | `BTreeMap` for deterministic key order |
| sha2 | 0.11.0 | sha256 of streams | **No `LowerHex` on the digest output** (verified compile error); use a 6-line hex helper, not a new crate |
| thiserror | 2.0.21 | typed errors in libraries | |
| toml | 1.1.7 | parse `pins.toml` | crate version string `1.1.7+spec-1.1.0` |
| regex | 1.13.1 | tool version-output parsing | avoid ad-hoc `split` parsing |
| clap (derive) | 4.6.7 | `mt` arguments (mt-cli only) | |
| anyhow | 1.0.104 | `mt-cli` only | |
| tempfile | 3.27.0 | dev-dependency for runner tests | |
| proptest | 1.11.0 | optional: parser property tests | guidelines mention it for tool-output parsers |
| cargo-deny | 0.20.2 | CI licence/advisory check | installed and run this session |

**Installation (tool side):**
```bash
# repo root rust-toolchain.toml pins 1.99.0 + thumbv7em target; rustup auto-installs it (verified ~20 s in a fresh RUSTUP_HOME).
cargo build --workspace --locked
cargo install --locked cargo-deny --version 0.20.2   # CI only
```

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Debian bookworm LLVM 16 packages | apt.llvm.org `llvm-toolchain-bookworm-16`; LLVM source build; official LLVM binary tarballs | apt.llvm.org is not snapshot-able (floating); source build costs 1-2 h per CI run; bookworm already has 16.0.6, so none is needed |
| Debian Z3 4.8.12 for KLEE | Z3 4.8.15 from tag (KLEE CI config), STP 2.3.4 | Source Z3 adds ~20-40 min to a cold build; keep as documented fallback if KLEE's cmake rejects the apt Z3 |
| Arm GNU 14.3.Rel1 | 15.3.Rel1 (exists, July 2026; x86_64 sha256 `563bebb2b97d53382b956d6ee1fe61e2cae26699901417234a37df505ef9b5fa` per nixpkgs; new download host `gitlab.arm.com/api/v4/projects/tooling%2Fgnu-toolchains-for-arm/packages/generic/gnu-toolchain/15.3.rel1/...`) | 14.3.Rel1 is the version TF-M CI moved to and is widely shipped; 15.3 is newest. Single-line pin change either way |
| `mt` built on the CI host (glibc 2.39) then mounted | Build `mt` as a static `x86_64-unknown-linux-musl` binary | **Use musl static** (verified: builds without musl-tools, `statically linked`, 1.1 MB); a glibc binary from ubuntu-24.04 will not run on bookworm (glibc 2.36). Also matches ADR-0001's "one static binary" |
| Python parse of TOML for Docker args | `mt toolchain build-args` (Rust) | Python is banned outside `research/`; shell TOML parsing is brittle |

## Package Legitimacy Audit

Run via `gsd-tools query package-legitimacy check --ecosystem crates ...` on 2026-10-08.

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| serde | crates.io | 11 yrs | 26.9M/wk | github.com/serde-rs/serde | OK | Approved |
| serde_json | crates.io | 11 yrs | 26.9M/wk | github.com/serde-rs/json | OK | Approved |
| sha2 | crates.io | 10 yrs | 22.4M/wk | github.com/RustCrypto/hashes | OK | Approved |
| thiserror | crates.io | 7 yrs | 31.6M/wk | github.com/dtolnay/thiserror | OK | Approved |
| toml | crates.io | 11 yrs | 19.1M/wk | github.com/toml-rs/toml | OK | Approved |
| clap | crates.io | 11 yrs | 19.3M/wk | github.com/clap-rs/clap | OK | Approved |
| anyhow | crates.io | 7 yrs | 18.1M/wk | github.com/dtolnay/anyhow | OK | Approved |
| regex | crates.io | 11 yrs | 21.0M/wk | github.com/rust-lang/regex | OK | Approved |
| tempfile | crates.io | 11 yrs | 15.6M/wk | github.com/Stebalien/tempfile | OK | Approved |
| proptest | crates.io | 8 yrs | 4.2M/wk | github.com/proptest-rs/proptest | OK | Approved |
| cargo-deny | crates.io | 7 yrs | 142k/wk | github.com/EmbarkStudios/cargo-deny | OK | Approved (CI tool only) |
| cargo-mutants | crates.io | 5 yrs | 23.6k/wk | github.com/sourcefrog/cargo-mutants | OK | Approved (container tool) |
| c2rust `[WARNING: flagged as suspicious — verify before using.]` | crates.io | 8 yrs | 369/wk | github.com/immunant/c2rust | **SUS** (reason: `low-downloads`) | Flagged - planner must add a `checkpoint:human-verify` before the first install |

**Packages removed due to SLOP verdict:** none.
**Packages flagged SUS:** c2rust. Mitigating evidence gathered (for the human-verify checkpoint, not a substitute for it): crates.io `repository` = `https://github.com/immunant/c2rust/`, 68.7k total downloads since 2018-10-15, owners `ahomescu, thedataking, rinon, kkysen, fw-immunant` (Immunant staff), the project README instructs `cargo install --locked c2rust`, and the published `.crate` sha256 equals the index `cksum`. The seam rates it SUS only on weekly downloads. The crate is a project requirement (D-09, FRONT-02), so it stays, gated by a human checkpoint that compares owners/repository once.

*Non-crate artifacts (apt packages, Arm tarball, git sources, rustup-init) are outside the seam; each is pinned by digest/commit/checksum per the table above, and any value not verified this session is tagged in the Assumptions Log.*

## Architecture Patterns

### System Architecture Diagram

```
 CI (GitHub Actions, ubuntu-24.04, amd64)                      Founder (Mac, emulated amd64)
 ───────────────────────────────────────                       ─────────────────────────────
 container/pins.toml ──► mt toolchain build-args ──► --build-arg NAME=VALUE ...
   (single source)            (Rust, host-built)               docker pull ghcr.io/...@sha256:<digest>
        │                                                                │
        ▼                                                                ▼
 docker buildx build (context = container/) ────────────────►  IMAGE  (linux/amd64)
   stage apt-base:  debian@sha256 + snapshot sources + LLVM16 + runtime pkgs
   stage rustup:    rustup-init(sha256) → toolchain 1.99.0(+thumbv7em,+musl) and 1.72.1(host only)
   stage cargo-tools: LLVM_CONFIG_PATH=llvm-config-16  cargo install --locked c2rust, cargo-mutants
   stage klee:      klee-uclibc@commit, KLEE@commit, cmake -DLLVM_DIR=/usr/lib/llvm-16/...
   stage arm:       Arm tarball + sha256 -c
   final:           apt-base + COPY /opt/klee /opt/cargo-tools /opt/arm-gnu-toolchain /opt/rustup /opt/cargo
        │
        ▼
 CI job "check": cargo build -p mt-cli --target x86_64-unknown-linux-musl (static mt)
   docker run --network none -v mt:/usr/local/bin/mt:ro -v pins.toml:/opt/mt/pins.toml:ro IMAGE
        mt toolchain check    ──► runner (env_clear + fixed base env) ──► each tool --version
                              ──► parse (regex) ──► compare to pins ──► table on stdout, exit≠0 on any drift
        mt toolchain manifest ──► manifest.json (sorted keys, no timestamps)
 (second, --no-cache build in parallel job) ──► manifest-b.json ──► diff -u manifest-a.json manifest-b.json
        │
        ▼
 smoke job: in IMAGE: cargo build --release --target thumbv7em-none-eabihf --manifest-path container/smoke/Cargo.toml
            arm-none-eabi-gcc -mcpu=cortex-m4 ... -c container/smoke/smoke.c
 push job (main only): docker push → digest recorded as the container reference for later phases
```

Docker-free path (cloud sessions): `cargo test --workspace` exercises everything left of "docker buildx" except the live tools, using fixture outputs.

### Recommended Project Structure

```
Cargo.toml                    # [workspace] members = ["crates/*"]; exclude = ["container/smoke"]
Cargo.lock                    # committed; CI uses --locked
rust-toolchain.toml           # channel = "1.99.0", components rustfmt+clippy, targets thumbv7em-none-eabihf, profile minimal
deny.toml  clippy.toml        # see Code Examples
.github/workflows/ci.yml      # cargo fmt/clippy/test/deny on ubuntu-24.04
.github/workflows/container.yml
crates/
  mt-toolchain/               # runner.rs (ONLY file using std::process), pins.rs, version.rs, check.rs, manifest.rs
  mt-cli/                     # bin `mt`: `mt toolchain {check,manifest,build-args,hash}`; argument parsing only
container/
  Dockerfile  pins.toml  README.md  .dockerignore
  smoke/                      # no_std crate (own [workspace]); smoke.c
research/                     # empty placeholder (.gitkeep): the only place Python may live
```

### Pattern 1: Runner (verified prototype)
**What:** `run(cfg, argv, cwd, extra_env) -> RunOutput { record, stdout, stderr }`. `env_clear()` then base env (`PATH`, `LANG=C.UTF-8`, `TZ=UTC`, `SOURCE_DATE_EPOCH`) then named extras; stdin null; one reader thread per pipe that hashes every byte and stores only the first `cap` bytes; main thread polls `try_wait`; timeout kills the child; `signal()` from `ExitStatusExt` (safe). Hash covers the **whole** stream, so `truncated` never changes the sha256 and `mt toolchain hash <url>` can be `curl -fsSL` run through the runner.
**When to use:** every external process, including `rustc +1.72.1 -vV` and `cargo +1.72.1`.
**Record:** `{schema_version, argv, cwd, env (BTreeMap), tool{name,path,version?}, exit{code,signal,timed_out}, stdout{sha256,bytes_total,bytes_stored,truncated}, stderr{...}}`. Output bytes are returned beside the record, not inside the JSON (Phase 11 decides storage).

### Pattern 2: Generated build args, no duplicated pins
`mt toolchain build-args --pins container/pins.toml` prints sorted `NAME=value` lines. The Dockerfile declares `ARG NAME` with **no default** (an unset ARG then fails the build via `${NAME:?}`), so a version literal can never live in the Dockerfile. Plain-cargo tests assert parity: every Dockerfile `ARG` has a pin, every rendered arg is declared.

### Pattern 3: Check = data-driven table
`pins.toml` `[tool.<name>]` tables carry `bin`, `version_args`, `version_regex` (named group `version`), `expect`, optional `llvm_major_regex`. `check` runs each through the runner, extracts, compares, and prints one line per tool (`name  expected  actual  OK|MISMATCH|MISSING`). One shared assertion enforces TOOL-02: every extracted LLVM major equals `[llvm].major`, and the bitcode `rustc -vV` `LLVM version:` major is equal to the pin and `<= llvm.max_major (19)`.

### Anti-Patterns to Avoid
- **Unversioned LLVM packages** (`clang`, `llvm-dev`, `libclang-dev`, `lld`): on bookworm these are LLVM **14**. One accidental dependency installs a second LLVM and c2rust's build picks the *highest* `llvm-config-N` on PATH (observed: it chose `/usr/lib/llvm-18`). Install only `*-16` packages, set `LLVM_CONFIG_PATH` explicitly, and add a check that `dpkg-query -W 'llvm-[0-9]*' 'clang-[0-9]*'` shows only major 16.
- **Hashing raw `--version` output** into the manifest: `klee --version` appends `Host CPU:` (varies by runner CPU). Store parsed fields only.
- **Putting the image's tool code in the image digest:** keep `mt` out of the image (mounted at run time) so the image digest depends only on `Dockerfile` + `pins.toml`.
- **`https://` apt snapshot sources in the slim image:** `bookworm-slim` has no `ca-certificates`; use `http://` (apt verifies signed Release files) or install the package first.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| TOML/JSON parsing and stable output | custom parser/printer | `toml`, `serde_json` with `BTreeMap` | key order and escaping are where reproducibility bugs hide |
| sha256 | own implementation | `sha2` (+ 6-line hex helper) | verified API quirk: no `LowerHex` in 0.11 |
| Tool version extraction | `split(' ')` | `regex` with named groups + fixtures | formats vary by distro build strings |
| Reproducible apt | mirror/pin scripts | `snapshot.debian.org` timestamp sources | Debian runs the archive; `Check-Valid-Until=false` handles expired Release files |
| Rust toolchain install | tarball juggling | `rustup-init` (sha256-checked) + `rustup toolchain install <exact>` + channel-manifest sha256 | rustup verifies component hashes |
| LLVM | source build | Debian `*-16` packages | 1-2 h per build vs minutes |
| Image digest resolution | scraping | `docker buildx imagetools inspect` / registry HEAD | |
| Static-binary `mt` | glibc build on host | `x86_64-unknown-linux-musl` target | verified; avoids glibc mismatch |
| Python-ban walker | `walkdir` dep | `std::fs::read_dir` recursion (do not follow symlinks) | no new dependency |
| Banning `std::process::Command` | grep script | `clippy.toml` `disallowed-types` (+ module-level allow in `runner.rs`) | verified to fire under `cargo clippy -D warnings` |

**Key insight:** every "small script" in this phase (arg rendering, digest checks, python walk, decision gate) is a Rust test or `mt` subcommand, because Python is banned and shell parsing of TOML/JSON is the classic source of silent pin drift.

## Common Pitfalls

### Pitfall 1: Debian default LLVM is 14
**What goes wrong:** `apt-get install clang cmake libclang-dev` pulls LLVM 14; KLEE/c2rust build against the wrong major; manifest passes if only `clang-16` is checked.
**How to avoid:** list only `-16` packages; `LLVM_CONFIG_PATH=/usr/lib/llvm-16/bin/llvm-config`; put `/usr/lib/llvm-16/bin` first in PATH; the check asserts no other LLVM major is installed.
**Warning signs:** `ls /usr/lib | grep llvm-` shows more than one entry; `ldd $(which c2rust)` shows a `.so.14` or `.so.18`.

### Pitfall 2: c2rust crate ships `rust-toolchain.toml` pinning `nightly-2022-08-08`
**What goes wrong:** Looks like a nightly requirement. **Verified false for install:** with 1.99.0 active, `cargo install --locked c2rust --version =0.22.1` ignored it (rustup toolchain list stayed `1.99.0` only) and compiled the Rust dependency graph; only the nightly-pinned extras (`c2rust-refactor`) need it, and they are not published. README: "`c2rust` and `c2rust-transpile` ... can be built on `stable` rustc."
**How to avoid:** no third toolchain; set `RUSTUP_TOOLCHAIN=1.99.0` explicitly in that Dockerfile `RUN` as a belt-and-braces guard.

### Pitfall 3: c2rust build needs network at image-build time
`c2rust-ast-exporter`'s cmake `ExternalProject` clones `https://github.com/intel/tinycbor.git` at a pinned commit (`d393c16f...`) during the build. Needs `git` and network in the build stage; the pin is embedded in the crate so no extra pin is needed, but record it in `pins.toml` as a transitive source for the manifest. Override with `TINYCBOR_DIR` only if mirroring is ever wanted.

### Pitfall 4: KLEE built with assertions against LLVM without assertions
Debian LLVM is built without assertions; KLEE's default `ENABLE_KLEE_ASSERTS=ON` prints a "might lead to unexpected behaviour" warning. Pass `-DENABLE_KLEE_ASSERTS=OFF` (KLEE's own CI "Asserts disabled" configuration). Also pass `-DENABLE_UNIT_TESTS=OFF -DENABLE_SYSTEM_TESTS=OFF -DENABLE_DOCS=OFF` (gtest/lit not needed; CMakeLists states this combination) [VERIFIED: KLEE CMakeLists.txt at v3.2].

### Pitfall 5: `klee --version` is not deterministic as raw text
Output (from `lib/Support/PrintVersion.cpp` + `llvm::cl::PrintVersionMessage`): `KLEE 3.2 (https://klee-se.org/)`, `Build mode: ...`, `Build revision: unknown|<rev>`, blank, `LLVM (http://llvm.org/):`, `  LLVM version 16.0.6`, ..., `Host CPU: <cpu>`. Parse `KLEE (\d+\.\d+)` and `LLVM version (\d+)\.(\d+)\.(\d+)`; ignore the rest.

### Pitfall 6: sha2 0.11 digest has no `LowerHex`
`format!("{:x}", hasher.finalize())` fails to compile [VERIFIED]. Use a hex helper writing `{b:02x}` per byte.

### Pitfall 7: cargo-deny on a workspace with path deps and private crates
Defaults fail: `wildcards = "deny"` rejects `path = "../mt-toolchain"` (needs `allow-wildcard-paths = true`), and workspace crates without a `license` field are "unlicensed" (need `publish = false` plus `[licenses.private] ignore = true`) [VERIFIED with cargo-deny 0.20.2]. Dependency licences encountered for the intended crate set were all within MIT/Apache-2.0/Unicode-3.0.

### Pitfall 8: `cargo mutants --version` vs running the binary directly
`cargo mutants --version` prints `cargo-mutants 27.1.0` [VERIFIED]. Invoking `cargo-mutants --version` directly errors (`unexpected argument '--version'`); `cargo mutants -V` tries to open a workspace. Use argv `["cargo","mutants","--version"]`.

### Pitfall 9: env_clear breaks rustup shims
With the runner's empty environment, `rustc +1.72.1 -vV` through the rustup proxy needs `RUSTUP_HOME` (and `HOME` or `CARGO_HOME`) passed explicitly. Prefer absolute paths to toolchain binaries (`$RUSTUP_HOME/toolchains/1.72.1-x86_64-unknown-linux-gnu/bin/rustc -vV`) for the check, and pass `RUSTUP_HOME`/`CARGO_HOME` as named extras when `cargo +1.72.1` is needed (D-12). `CARGO_HOME` must be writable at run time if cargo downloads crates; set it to a writable dir for non-root runs.

### Pitfall 10: glibc mismatch for host-built `mt`
A binary built on ubuntu-24.04 (glibc 2.39) does not run on bookworm (2.36). Build `mt` with `--target x86_64-unknown-linux-musl` [VERIFIED: static-pie, 1.1 MB, no musl-tools needed for pure-Rust dependencies].

### Pitfall 11: `snapshot.debian.org` is slow and rate-limited
Large installs (LLVM 16 dev, QEMU) can time out. Set `Acquire::Retries "5"`, `Acquire::http::Timeout "120"`, `Acquire::Check-Valid-Until "false"`; keep one `apt-get update && apt-get install` per stage; install runtime and build packages in as few layers as possible. Mirror host fallbacks exist but were not verified [ASSUMED].

### Pitfall 12: "never call `Command` directly" vs the runner itself
Consistent with `docs/guidelines/building-the-tool.md` §4 only if the runner module is the one exception. Make it explicit: `clippy.toml` `disallowed-types = [{ path = "std::process::Command", ... }]` with `#![allow(clippy::disallowed_types)]` in `runner.rs` only [VERIFIED to error on any other use under `-D warnings`]. Also record the clarification as a PROJECT decision when D-15 is added (guidelines say rule changes need one).

### Pitfall 13: GitHub runner resources
Standard `ubuntu-24.04` runner: 4 vCPU/16 GB/14 GB SSD for **public** repos, 2 vCPU/8 GB/14 GB for **private** [CITED: docs.github.com hosted-runner reference]. A multi-GB image plus a `--no-cache` second build can exhaust 14 GB; free space first (`sudo rm -rf /usr/share/dotnet /opt/ghc /usr/local/lib/android`), build the pair in **separate parallel jobs**, and do not `--load` into the daemon more than one image per job. Repo visibility (github.com/sat-wik/migrationtool) is unknown to this research: if private, 2000 free min/month makes two 40-60 min builds per CI run expensive; trigger the container workflow only on `container/**`, `pins.toml`, workflow edits, `workflow_dispatch`, and a weekly schedule.

### Pitfall 14: Apple Silicon default platform
`docker build` on an M-series Mac defaults to arm64. Every `FROM` must carry `--platform=linux/amd64` and the build command `--platform linux/amd64`; the digest pin is the multi-arch **index** digest so the platform flag selects amd64 deterministically.

### Pitfall 15: pipe grandchildren can hang reader threads
If a tool forks a daemon that inherits stdout, EOF never arrives after `kill`. The prototype receives results over a channel with `recv_timeout(grace)` after the child exits; it reports empty capture rather than blocking. Phase 1 tools do not daemonise; document the limitation, do not add `unsafe` process-group code (`forbid(unsafe_code)`).

### Pitfall 16: Rust edition 2024 makes `std::env::set_var` unsafe
The env-cleared runner test should not depend on `set_var` (forbidden under `forbid(unsafe_code)` in edition 2024). Assert that variables cargo sets for tests (e.g. `CARGO_MANIFEST_DIR`) are absent from the child's `env` output instead.

## Code Examples

### Example 1: Runner core [VERIFIED: compiled, 4 tests pass, `cargo clippy --all-targets -- -D warnings` clean on 1.99.0]

```rust
// crates/mt-toolchain/src/runner.rs  (the ONLY file allowed to use std::process)
#![allow(clippy::disallowed_types)]
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};
use std::sync::mpsc; use std::thread; use std::time::{Duration, Instant};
use sha2::{Digest, Sha256};

fn hex(bytes: &[u8]) -> String {                       // sha2 0.11 digests lack LowerHex
    use std::fmt::Write;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes { let _ = write!(s, "{b:02x}"); }
    s
}

/// Drain a pipe to EOF: hash EVERY byte, store only the first `cap` bytes.
fn drain<R: std::io::Read>(mut r: R, cap: usize) -> std::io::Result<Captured> {
    let (mut h, mut stored, mut total, mut buf) = (Sha256::new(), Vec::new(), 0u64, [0u8; 64 * 1024]);
    loop {
        let n = match r.read(&mut buf) {
            Ok(0) => break, Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        h.update(&buf[..n]); total += n as u64;
        let room = cap.saturating_sub(stored.len());
        stored.extend_from_slice(&buf[..n.min(room)]);
    }
    Ok(Captured { rec: StreamRecord { sha256: hex(&h.finalize()), bytes_total: total,
        bytes_stored: stored.len() as u64, truncated: total > stored.len() as u64 }, stored })
}

// in run(): cmd.env_clear().envs(&env).current_dir(&cwd).stdin(Stdio::null())
//            .stdout(Stdio::piped()).stderr(Stdio::piped());
// spawn one thread per pipe -> mpsc; loop { try_wait(); if timeout { child.kill(); } sleep(5ms) };
// then recv_timeout(5s) for each stream; record exit.code(), exit.signal(), timed_out.
```
Tests that passed (rename to the project convention): captures stdout/stderr/exit and sha256 (`sh -c 'echo out; echo err 1>&2; exit 3'`); env cleared; 5 MB on both streams with cap 1024 does not deadlock and sets `truncated`; `sleep 30` with 200 ms timeout yields `timed_out` and `signal == Some(9)`.

### Example 2: `tool_01_no_python_outside_research` [ASSUMED shape; walk logic is standard std]

```rust
#[test]
fn tool_01_no_python_outside_research() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    const SKIP: &[&str] = &[".git", "target", "research"];          // do NOT skip .claude: it has 0 .py today
    const EXT: &[&str] = &["py", "pyi", "pyw", "pyx"];
    let mut bad = Vec::new();
    let mut stack = vec![root.clone()];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).unwrap() {
            let e = e.unwrap(); let p = e.path(); let ft = e.file_type().unwrap();
            if ft.is_symlink() { continue; }                         // never follow symlinks
            if ft.is_dir() { if !(dir == root && SKIP.contains(&e.file_name().to_str().unwrap_or(""))) { stack.push(p); } }
            else if p.extension().and_then(|x| x.to_str()).is_some_and(|x| EXT.contains(&x)) { bad.push(p); }
        }
    }
    assert!(bad.is_empty(), "Python source outside research/: {bad:?}");
}
```
Current state [VERIFIED: `find .claude -name '*.py'` = 0; `git ls-files | grep -c '\.py$'` = 0; `.claude/hooks` are `.js`/`.sh`]. Add a second assertion (shebang scan of tracked files under `container/`, `crates/`) only if the founder wants it; extension check satisfies TOOL-01 wording. Tests are allowed `unwrap` per conventions.

### Example 3: decision-ID gate (D-21) [ASSUMED shape]

Add one PROJECT decision (next free ID is **D-15**) whose line carries machine-readable tokens, e.g. `- **D-15:** Provisional pin for Phase 1: llvm=16 bitcode-rustc=1.72.1 (revisited at the Phase 6 R1 verdict; supersedes nothing in D-08).` `pins.toml` has `[llvm] major = 16, decision = "D-15"` and `[rust.bitcode] version = "1.72.1", decision = "D-15"`. The test (`tool_02_llvm_and_bitcode_pins_have_matching_project_decision`) parses `pins.toml`, reads `.planning/PROJECT.md`, finds the line starting `- **D-15:**`, and asserts it contains `llvm=16` and `bitcode-rustc=1.72.1` computed from the pins. Changing either pin without editing PROJECT.md fails `cargo test`. The test must **fail, not skip,** if `.planning/PROJECT.md` is missing (`/gsd-pr-branch` strips `.planning/` only for external review branches). Reuse the same decision for recording the "runner is the one `Command` user" clarification.

### Example 4: TOOL-04 smoke crate [VERIFIED: builds for thumbv7em-none-eabihf in dev and release; clippy `-D warnings` clean; invisible to `cargo build --workspace`]

```toml
# container/smoke/Cargo.toml
[package]  name = "mt-smoke"  version = "0.0.0"  edition = "2021"  publish = false
[lib]      crate-type = ["staticlib", "rlib"]
[features] default = ["panic-handler"]  panic-handler = []
[dependencies]
[profile.dev]     panic = "abort"
[profile.release] panic = "abort"  opt-level = "s"  lto = true  codegen-units = 1
[workspace]                                   # makes it its own workspace root
[lints.rust]   unsafe_code = "deny"  missing_docs = "deny"
[lints.clippy] all = { level = "deny", priority = -1 }
```
```rust
// src/lib.rs
#![no_std]
mod ffi;
/// Adds two values with explicit wrapping arithmetic.
pub fn add(a: u32, b: u32) -> u32 { a.wrapping_add(b) }
#[cfg(feature = "panic-handler")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! { loop {} }
// src/ffi.rs
#![allow(unsafe_code)]
/// C ABI wrapper for [`crate::add`].
#[no_mangle]
pub extern "C" fn mt_smoke_add(a: u32, b: u32) -> u32 { crate::add(a, b) }
```
Root `Cargo.toml` uses `exclude = ["container/smoke"]`. Located at `container/smoke/` (not `fixtures/`, which the layout reserves for planted-bug suites). Build command: `cargo build --release --target thumbv7em-none-eabihf --manifest-path container/smoke/Cargo.toml`. The root `rust-toolchain.toml` applies (rustup searches parents) [VERIFIED]. Release `libmt_smoke.a` was 8.4 MB (it bundles `core`/`compiler_builtins`), `.rlib` 8 KB.
Optional C-side companion (TOOL-04's "arm-none-eabi GCC"): `container/smoke/smoke.c` declaring `uint32_t mt_smoke_add(uint32_t, uint32_t)` compiled with `arm-none-eabi-gcc -mcpu=cortex-m4 -mthumb -mfloat-abi=hard -mfpu=fpv4-sp-d16 -Os -c`; a stronger `-r -nostdlib smoke.o libmt_smoke.a` partial link is [ASSUMED] to resolve the Rust symbol and should be tried in CI, not relied on.

### Example 5: `deny.toml` and `clippy.toml` [VERIFIED against cargo-deny 0.20.2: "advisories ok, bans ok, licenses ok, sources ok" for serde, serde_json, sha2, thiserror, toml, regex, clap, anyhow, tempfile]

```toml
# deny.toml
[graph]
targets = ["x86_64-unknown-linux-gnu"]
[advisories]
version = 2
yanked = "deny"
[licenses]
version = 2
allow = ["MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Zlib", "Unicode-3.0"]
confidence-threshold = 0.93
unused-allowed-license = "allow"
[licenses.private]
ignore = true                 # workspace crates are publish = false
[bans]
multiple-versions = "warn"
wildcards = "deny"
allow-wildcard-paths = true   # path deps between workspace crates
[sources]
unknown-registry = "deny"
unknown-git = "deny"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
allow-git = []
```
```toml
# clippy.toml
disallowed-types = [
    { path = "std::process::Command", reason = "run external tools only through mt-toolchain's runner" },
]
```
The advisory DB fetch worked from this session (`advisories ok`), so `cargo deny check` also runs in cloud sessions with network. In CI the DB is fetched live; that is the one network use and it is CI-level, not a test.

### Example 6: `pins.toml` sketch (schema v1; values marked ✔ are verified above, `‹discover›` fields are produced by the pin-discovery step before the file is committed — the committed file contains no placeholders)

```toml
schema_version = 1

[image]
platform           = "linux/amd64"
base               = "debian:bookworm-20261005-slim"
base_digest        = "sha256:7c7b2c966bc9ee8cedfeef67e0e279108992c77681fa595db4a9d65c06ccc587"   # ✔
snapshot_timestamp = "20261005T000000Z"
source_date_epoch  = "1791158400"                                                                # ✔ = 2026-10-05T00:00:00Z
tool_path          = "/usr/lib/llvm-16/bin:/opt/klee/bin:/opt/cargo-tools/bin:/opt/arm-gnu-toolchain/bin:/opt/cargo/bin:/usr/local/bin:/usr/bin:/bin"

[llvm]
major     = 16
max_major = 19
status    = "provisional"      # until Phase 6 R1 verdict (PROJECT D-08)
decision  = "D-15"

[rustup]
version = "1.29.1"
sha256  = "dda7234360b7f578ca8b0ddcb80145646fa61a67c1720a5abc7051b35c9fcb71"                      # ✔

[rust.tool]                                                                                       # ✔ all
version = "1.99.0"  commit = "b940084d7eb6a299eb4bfeb8e34901bc051e7ac4"
channel_manifest_sha256 = "ce6dddc886364f8d786514771212cebe9b731ba82d6b859951c6b0ccc516b6a2"
targets = ["thumbv7em-none-eabihf", "x86_64-unknown-linux-musl"]

[rust.bitcode]                                                                                    # ✔ all
version = "1.72.1"  commit = "d5c2e9c342b358556da91d61ed4133f6f50fc0c3"  llvm = "16.0.5"
channel_manifest_sha256 = "77113b9660855a5ab49e5ff029c998be93fdcea41b062b40420675cf1f4ea229"
decision = "D-15"

[source.klee]        git = "https://github.com/klee/klee"        tag = "v3.2"            commit = "92ee8201a050184aacaaeef645ade491f5c93c41"   # ✔
[source.klee_uclibc] git = "https://github.com/klee/klee-uclibc" tag = "klee_uclibc_v1.4" commit = "955d502cc1f0688e82348304b053ad787056c754"  # ✔
[source.c2rust]      crate = "c2rust"        version = "0.22.1" crate_sha256 = "e331f411...3f3b"   # ✔ (full hash above)
[source.cargo_mutants] crate = "cargo-mutants" version = "27.1.0" crate_sha256 = "07072e7b...1cf9" # ✔
[source.arm_gnu]     version = "14.3.rel1" url = "https://developer.arm.com/-/media/Files/downloads/gnu/14.3.rel1/binrel/arm-gnu-toolchain-14.3.rel1-x86_64-arm-none-eabi.tar.xz"
                     sha256  = "8f6903f8ceb084d9227b9ef991490413014d991874a1e34074443c2a72b14dbd"   # ✔ nixpkgs; re-confirm vs .sha256asc
[source.tinycbor]    note = "transitive, pinned by c2rust 0.22.1" commit = "d393c16f3eb30d0c47e6f9d92db62272f0ec4dc7"   # ✔

[apt]   # ‹discover›: exact versions printed by apt-cache policy against the snapshot
"clang-16" = "‹discover›"  "llvm-16-dev" = "‹discover›"  "libclang-16-dev" = "‹discover›"
"bear" = "‹discover›"  "qemu-system-arm" = "‹discover›"  "qemu-user" = "‹discover›"  "libz3-dev" = "‹discover›"

[tool.clang]  bin = "/usr/lib/llvm-16/bin/clang"  version_args = ["--version"]
              version_regex = 'clang version (?P<version>\d+\.\d+\.\d+)'   expect = "16.0.6"   llvm_major = true
[tool.llvm_config] bin = "/usr/lib/llvm-16/bin/llvm-config" version_args = ["--version"] version_regex = '^(?P<version>\d+\.\d+\.\d+)' expect = "16.0.6" llvm_major = true
[tool.klee]   bin = "/opt/klee/bin/klee" version_args = ["--version"] version_regex = '(?m)^KLEE (?P<version>\d+\.\d+)' expect = "3.2"
              llvm_regex = '(?m)LLVM version (?P<llvm>\d+\.\d+\.\d+)'
[tool.c2rust] bin = "/opt/cargo-tools/bin/c2rust" version_args = ["--version"] version_regex = '(?P<version>\d+\.\d+\.\d+)' expect = "0.22.1"
              ldd_llvm_regex = 'lib(?:clang-cpp|LLVM)[-.](?P<major>\d+)'            # LLVM major via `ldd`
[tool.bear]   ...  [tool.arm_gcc] ...  [tool.qemu_system_arm] ...  [tool.qemu_arm] ...  [tool.cargo_mutants] ...
[tool.hayroll] status = "not_installed"   [tool.kani] status = "not_installed"
```
(Fields beyond what D-14/D-15 require are the planner's call; the shape above is a recommendation.)

### Version commands and parse targets for `mt toolchain check`

| Tool | argv | Expected first lines | Parse | Evidence |
|------|------|----------------------|-------|----------|
| rustc (tool) | `rustc -vV` | `release: 1.99.0`, `commit-hash: b940084d7e...`, `LLVM version: 23.1.1` | `^release: (\S+)`, `^commit-hash: (\w{40})`, `^LLVM version: (\d+)\.(\d+)\.(\d+)` | [VERIFIED: executed] |
| rustc (bitcode) | `<RUSTUP_HOME>/toolchains/1.72.1-x86_64-unknown-linux-gnu/bin/rustc -vV` (or `rustc +1.72.1 -vV` with `RUSTUP_HOME`) | `release: 1.72.1`, `commit-hash: d5c2e9c342b358556da91d61ed4133f6f50fc0c3`, `LLVM version: 16.0.5` | same | [VERIFIED: executed] |
| cargo-mutants | `cargo mutants --version` | `cargo-mutants 27.1.0` | `cargo-mutants (\S+)` | [VERIFIED: executed] |
| klee | `klee --version` | `KLEE 3.2 (https://klee-se.org/)` ... `LLVM (http://llvm.org/):` `  LLVM version 16.0.x` | see Pitfall 5 | [CITED: klee v3.2 `PrintVersion.cpp`, `CMakeLists.txt` lines 28-29] |
| llvm-config | `llvm-config --version` | `16.0.6` | whole-line semver | [ASSUMED] standard |
| clang | `clang --version` | `Debian clang version 16.0.6 (15~deb12u1)` | `clang version (\d+\.\d+\.\d+)` | [ASSUMED] Debian format |
| c2rust | `c2rust --version` | `c2rust 0.22.1` | `(\d+\.\d+\.\d+)` | [ASSUMED] clap derive/cargo |
| bear | `bear --version` | `bear 3.1.1` | `(\d+\.\d+\.\d+)` | [ASSUMED] |
| arm-none-eabi-gcc | `arm-none-eabi-gcc --version` | `arm-none-eabi-gcc (Arm GNU Toolchain 14.3.Rel1 (Build arm-14.xxx)) 14.3.1 2025xxxx` | `Arm GNU Toolchain (\d+\.\d+\.Rel\d+)` and `\) (\d+\.\d+\.\d+) \d{8}` | [ASSUMED] format; build number unknown |
| qemu-system-arm | `qemu-system-arm --version` | `QEMU emulator version 7.2.xx (Debian 1:7.2+dfsg-7+deb12uNN)` | `QEMU emulator version (\S+) \(Debian ([^)]+)\)` | [ASSUMED] |
| qemu-arm | `qemu-arm --version` | `qemu-arm version 7.2.xx (Debian ...)` | `version (\S+) \(Debian ([^)]+)\)` | [ASSUMED] |

Policy for [ASSUMED] rows: the first container run in CI prints the raw output; commit those outputs as fixtures (`crates/mt-toolchain/tests/fixtures/<tool>.txt`) and make the parse tests run against them. Until then the parser tests use the table's representative strings.

### Dockerfile skeleton [ASSUMED: assembled from verified commands; not built]

```dockerfile
# syntax=docker/dockerfile:1.7   (pin the frontend by digest in the committed file; resolve with `docker buildx imagetools inspect docker/dockerfile:1.7`)
ARG BASE_REF
ARG BASE_DIGEST
FROM --platform=linux/amd64 ${BASE_REF}@${BASE_DIGEST} AS apt-base
ARG SNAPSHOT_TIMESTAMP
ARG SOURCE_DATE_EPOCH
RUN set -eux; rm -f /etc/apt/sources.list /etc/apt/sources.list.d/*; \
    printf '%s\n' \
      "deb [check-valid-until=no] http://snapshot.debian.org/archive/debian/${SNAPSHOT_TIMESTAMP}/ bookworm main" \
      "deb [check-valid-until=no] http://snapshot.debian.org/archive/debian/${SNAPSHOT_TIMESTAMP}/ bookworm-updates main" \
      "deb [check-valid-until=no] http://snapshot.debian.org/archive/debian-security/${SNAPSHOT_TIMESTAMP}/ bookworm-security main" \
      > /etc/apt/sources.list; \
    printf 'Acquire::Check-Valid-Until "false";\nAcquire::Retries "5";\nAcquire::http::Timeout "120";\nAPT::Install-Recommends "false";\n' \
      > /etc/apt/apt.conf.d/99snapshot
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y \
      ca-certificates clang-16 llvm-16 llvm-16-dev libclang-16-dev libclang-cpp16-dev \
      bear qemu-system-arm qemu-user libz3-4 libsqlite3-0 xz-utils make ... && rm -rf /var/lib/apt/lists/*

FROM apt-base AS klee-build           # extra: cmake ninja-build git build-essential libz3-dev libsqlite3-dev zlib1g-dev python3
ARG KLEE_COMMIT  ARG KLEE_UCLIBC_COMMIT
RUN git clone --depth 1 --branch klee_uclibc_v1.4 https://github.com/klee/klee-uclibc /src/uclibc \
 && test "$(git -C /src/uclibc rev-parse HEAD)" = "${KLEE_UCLIBC_COMMIT}" \
 && cd /src/uclibc && ./configure --make-llvm-lib --with-cc /usr/lib/llvm-16/bin/clang \
        --with-llvm-config /usr/lib/llvm-16/bin/llvm-config && make -j"$(nproc)"      # recipe from KLEE scripts/build/p-uclibc.inc [VERIFIED text]
RUN git clone --depth 1 --branch v3.2 https://github.com/klee/klee /src/klee && test "$(git -C /src/klee rev-parse HEAD)" = "${KLEE_COMMIT}" \
 && cmake -S /src/klee -B /build/klee -G Ninja -DCMAKE_BUILD_TYPE=RelWithDebInfo -DCMAKE_INSTALL_PREFIX=/opt/klee \
      -DLLVM_DIR=/usr/lib/llvm-16/lib/cmake/llvm -DLLVMCC=/usr/lib/llvm-16/bin/clang -DLLVMCXX=/usr/lib/llvm-16/bin/clang++ \
      -DENABLE_SOLVER_Z3=ON -DENABLE_SOLVER_STP=OFF -DENABLE_KLEE_ASSERTS=OFF \
      -DENABLE_POSIX_RUNTIME=ON -DKLEE_UCLIBC_PATH=/src/uclibc -DENABLE_KLEE_LIBCXX=OFF -DENABLE_TCMALLOC=OFF \
      -DENABLE_UNIT_TESTS=OFF -DENABLE_SYSTEM_TESTS=OFF -DENABLE_DOCS=OFF \
 && cmake --build /build/klee && cmake --install /build/klee
# cargo-tools stage: RUSTUP_TOOLCHAIN=1.99.0 LLVM_CONFIG_PATH=/usr/lib/llvm-16/bin/llvm-config CLANG_PATH=/usr/lib/llvm-16/bin/clang \
#   cargo install --locked --root /opt/cargo-tools c2rust --version =0.22.1   (after: curl static.crates.io .crate | sha256sum -c)
# arm stage: curl -fsSLO <url> && echo "<sha256>  <file>" | sha256sum -c - && tar -xJf ... --strip-components=1 -C /opt/arm-gnu-toolchain
# rustup stage: curl rustup-init && sha256sum -c; rustup-init -y --default-toolchain none --profile minimal --no-modify-path;
#   curl channel-rust-<v>.toml | sha256sum -c; rustup toolchain install 1.99.0 -c rustfmt -c clippy -t thumbv7em-none-eabihf -t x86_64-unknown-linux-musl;
#   rustup toolchain install 1.72.1 --profile minimal; rustup default 1.99.0
```
KLEE decisions embedded above: `libc++` is **optional** (`ENABLE_KLEE_LIBCXX` default OFF, only needed for C++ symbolic targets; this project is C-only), tcmalloc off, STP off (Z3 only satisfies "at least one solver"), gtest/lit tests skipped, `ENABLE_POSIX_RUNTIME=ON` only because klee-uclibc is pinned per D-07. `build.yaml` for LLVM 16 uses solvers `STP:Z3`, STP 2.3.3 (Dockerfile: 2.3.4 = commit `d70085462f07c8a5a2f1225f727cda3ef505b141`), Z3 4.8.15; our reduced Z3-only configuration corresponds to KLEE CI's own "Z3 only" matrix entry.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| c2rust "needs pinned nightly" | `c2rust`/`c2rust-transpile` build on stable; only `c2rust-refactor` etc. need nightly-2022-08-08 and are unpublished | c2rust README at v0.22.1 | no third toolchain in the image |
| rustup auto-install semantics | rustup 1.28.2 (this session) auto-installed 1.99.0 + components + target from `rust-toolchain.toml` in ~20 s | observed | cloud sessions need no manual toolchain step (needs network to static.rust-lang.org) |
| Arm tarballs only on developer.arm.com | 15.x moved to `gitlab.arm.com/.../gnu-toolchains-for-arm` generic packages; 14.3 and older stay on developer.arm.com | nixpkgs master | pick one host per pin; both hash via `.sha256asc` |
| Debian ships LLVM 16 only in sid/testing | bookworm main carries `llvm-toolchain-16` (`1:16.0.6-15~deb12u1`) | by 2025 listing | no cross-release snapshot hack |
| `debian/snapshot` Docker images | stale (last tag 2023-12-18) | — | do not use as base; use official `debian:bookworm-YYYYMMDD-slim` + own snapshot sources |
| Official Debian tags | `bookworm` is now oldstable (trixie stable) | 2025 | LLVM 16 availability is the reason to stay on bookworm; support continues via LTS |

**Deprecated/outdated:** `cargo-deny` 0.1x config keys (`vulnerability`, `unmaintained` as lint levels) — use `[advisories] version = 2`, `[licenses] version = 2` as in Example 5.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The `snapshot.debian.org` archive at `20261005T000000Z` contains `clang-16 1:16.0.6-15~deb12u1` in `bookworm` or `bookworm-updates` and is reachable from GitHub runners | Standard Stack | Entire apt layer fails; fallback: earlier timestamp or `bookworm-security`; discovery step catches it first |
| A2 | Debian bookworm `libz3-dev` (4.8.12) compiles with KLEE 3.2 | Dockerfile skeleton | KLEE stage fails; fallback Z3 `z3-4.8.15` from tag `f1806d32...` (adds 20-40 min) |
| A3 | Debian's RTTI-enabled, assertion-less LLVM 16 packages are accepted by KLEE 3.2 cmake | KLEE | Build fails or warns; fallback documented in KLEE `p-llvm-linux-*.inc` flows; escalate as a D-02 decision only if LLVM 16 itself is unusable |
| A4 | c2rust 0.22.1 builds and runs against Debian LLVM 16 (its CI tests clang 15 and 18; verified only that the Rust graph compiles and cmake is reached) | Standard Stack | C++ exporter compile error; D-02 would move the whole pin, so test in the first CI run of that stage |
| A5 | Version-output formats for clang, llvm-config, c2rust, bear, arm-none-eabi-gcc, qemu-* match the table | Version table | Parse tests need fixture updates; low impact |
| A6 | Arm 14.3.Rel1 x86_64 sha256 `8f6903f8...` (from nixpkgs) equals Arm's official `.sha256asc`, and the `developer.arm.com/-/media/...14.3.rel1/binrel/` URL still resolves | Standard Stack | `sha256sum -c` fails in CI (safe failure); then take the value from the official `.sha256asc` |
| A7 | klee-uclibc `klee_uclibc_v1.4` builds with clang-16 on bookworm with only `make`/gcc host tools | KLEE | uclibc stage fails; descope is a recorded decision (build KLEE without POSIX runtime), not a silent skip |
| A8 | Build times on GH runners: Z3-only KLEE ~10-20 min, klee-uclibc ~5 min, c2rust ~8-12 min, cargo-mutants ~1-3 min (measured ~1 min on 4 cores), apt layer ~5-15 min; image ~6-9 GB | Pitfall 13 | Timeouts/disk; mitigate with parallel stage jobs and disk cleanup |
| A9 | Bookworm `bear` is 3.1.1-1 and prints `bear 3.1.1` | Standard Stack | Parse regex still generic; pin value comes from discovery |
| A10 | Package names `libclang-cpp16-dev`, `libclang-rt-16-dev`, `libfuzzer-16-dev`, `libz3-4`, `qemu-user` exist in bookworm | Dockerfile | Discovery step (`apt-cache policy`) catches it |
| A11 | `ldd c2rust` shows `libclang-cpp.so.16` / `libLLVM-16.so.1` (Debian has `libclang-cpp.so` in the llvm-16 libdir so build.rs links shared) | Check design | Fall back to checking `LLVM_CONFIG_PATH` used at build time recorded in the manifest |
| A12 | `arm-none-eabi-gcc -r -nostdlib smoke.o libmt_smoke.a` resolves the Rust symbol | Example 4 | Drop the optional partial link; compile-only still proves TOOL-04 |
| A13 | Repo `sat-wik/migrationtool` visibility (public vs private) | Pitfall 13 | Private: 2 vCPU/8 GB runners and metered minutes; plan for fewer builds or a public repo |
| A14 | The `debian-security` snapshot path works for `bookworm-security` at the same timestamp | Dockerfile | apt-get update errors; fallback: drop security suite (snapshot already contains updates up to its date for `bookworm-updates` only) and record the gap |
| A15 | `libclang-rt-16-dev`/`libfuzzer-16-dev` (sanitizer and libFuzzer runtimes for Phases 4-5) are worth adding now | Dockerfile | Only a later pin-bump PR if omitted; harmless if included |

## Open Questions

1. **Exact apt versions at the chosen snapshot** — What we know: expected versions above. Unclear: the precise strings (`clang-16`, `llvm-16-dev`, `bear`, `qemu-*`, `libz3-dev`) at `20261005T000000Z`. Recommendation: first plan task is a **pin-discovery workflow** (`workflow_dispatch`) that runs the `apt-base` stage prefix with `apt-cache policy <pkgs>` and prints the lines, plus `curl -fsSL <arm .sha256asc>` and `docker buildx imagetools inspect`; its output is pasted into `pins.toml` in a PR before the Dockerfile stages that consume it are merged. No placeholder reaches `main`.
2. **Repo visibility and CI minutes** (A13) — founder decision; affects the build-twice cadence.
3. **Z3 4.8.12 vs 4.8.15** (A2) — decide at first KLEE CI run.
4. **Arm 14.3.Rel1 vs 15.3.Rel1** — recommendation 14.3.Rel1; the planner may take 15.3 (hash and host recorded above) if the founder wants newest.
5. **Add sanitizer/libFuzzer runtimes and `-m32` multilib now?** (A15) — outside TOOL-01..04; cheap to include `libclang-rt-16-dev libfuzzer-16-dev`; multilib (`gcc-multilib`/`libc6-dev-i386`) belongs to Phase 5 (R5) and is not recommended now.
6. **Renode** (preferred for DIFF-06) is not in scope here; QEMU is installed as the stated fallback. Flag for Phase 5 research.

## Environment Availability

Probed 2026-10-08 in the current cloud session.

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| cargo / rustc / rustup | all plain-cargo verification | ✓ | cargo 1.97.0, rustc 1.97.0 (`LLVM version: 22.1.6`), rustup 1.28.2; a `rust-toolchain.toml` for 1.99.0 auto-installs in ~20 s (static.rust-lang.org reachable) | — |
| docker CLI / buildx | image work | CLI ✓ 29.8.2, buildx 0.37.1 | **no daemon** (`/var/run/docker.sock` missing); `dockerd`/`containerd`/`runc` binaries exist but an attempt to start `dockerd` failed on a too-long socket path, and apt mirrors are blocked anyway | CI only (as CONTEXT states) |
| Network egress | research and cargo | reachable: static.rust-lang.org, index.crates.io, static.crates.io, crates.io API (with User-Agent), raw.githubusercontent.com, `git ls-remote/clone` of github.com repos, registry-1.docker.io + auth.docker.io + hub.docker.com API, api.github.com (limited), pypi, npm, RustSec DB (cargo-deny ran) | blocked: deb.debian.org, snapshot.debian.org, apt.llvm.org, sources.debian.org, packages.debian.org, developer.arm.com, gitlab.arm.com, github.com release/codeload downloads (403), download.qemu.org, llvm.org | pin discovery for apt/Arm runs in CI |
| clang / llvm-config | — | ✓ clang 18.1.3 (Ubuntu) but **no** `libclang-dev` / cmake config files | cannot build c2rust or KLEE here | CI |
| cmake 3.28.3, gcc 13.3, make, ninja, git, python3 | — | ✓ (host) | irrelevant to the image (bookworm versions apply) | — |
| cargo-deny 0.20.2, cargo-mutants 27.1.0 | CI / container | built here into the scratchpad (2 min and 1 min) | not installed system-wide | CI installs |
| Machine | — | 4 vCPU, 15 GB RAM, 252 GB disk | — | — |

**Missing with no fallback:** none for plain-cargo work. **Missing with fallback:** every container-level proof (CI).

## Validation Architecture

Nyquist validation is enabled (`.planning/config.json` `workflow.nyquist_validation: true`). Test names embed the requirement ID in snake case; tests never use the network or a model. "Cargo" = runs under plain `cargo test --workspace` in a Docker-less session. "CI/container" = needs the built image.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness (`cargo test`); optional `proptest` 1.11.0 for version-parser properties |
| Config file | none beyond `Cargo.toml`, `rust-toolchain.toml`, `deny.toml`, `clippy.toml` — Wave 0 creates them |
| Quick run command | `cargo test -p mt-toolchain` |
| Full suite command | `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace --locked && cargo deny check` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command / Test name | Where it runs |
|--------|----------|-----------|-------------------------------|---------------|
| TOOL-01 | No Python outside `research/` | unit/repo | `tool_01_no_python_outside_research` | Cargo |
| TOOL-01 | Runner captures stdout, stderr, exit status, sha256, argv, cwd, effective env, schema_version | unit | `tool_01_runner_records_argv_env_exit_and_output_hashes` | Cargo |
| TOOL-01 | Env is cleared then fixed base (+ named extras) | unit | `tool_01_runner_clears_environment_and_applies_fixed_base` | Cargo |
| TOOL-01 | Output cap, truncation flag, no deadlock with both streams large | unit | `tool_01_runner_caps_output_without_deadlock` | Cargo |
| TOOL-01 | Timeout kills and reports `timed_out` (maps to "not proven") | unit | `tool_01_runner_timeout_is_reported_not_passed` | Cargo |
| TOOL-01 | Run record JSON is byte-stable (sorted env, schema_version) | golden | `tool_01_run_record_json_is_deterministic` | Cargo |
| TOOL-01 | `std::process::Command` used only in `runner.rs` | lint/scan | `cargo clippy --all-targets -- -D warnings` (disallowed-types) plus `tool_01_command_only_in_runner_module` | Cargo |
| TOOL-01 | Every crate declares `#![forbid(unsafe_code)]` | scan | `tool_01_every_tool_crate_forbids_unsafe` | Cargo |
| TOOL-01 | Analyser launched through the runner inside the container with version stamp | integration | `mt toolchain check` exit 0 in image; prints tool/version table | CI/container |
| TOOL-02 | Parsers extract versions/LLVM major from fixture outputs for clang, llvm-config, klee, rustc -vV, c2rust(ldd) | unit | `tool_02_parses_llvm_major_from_tool_outputs` | Cargo |
| TOOL-02 | Different major in any C-side tool fails the check | unit | `tool_02_check_fails_when_any_tool_llvm_major_differs` | Cargo |
| TOOL-02 | Bitcode rustc with LLVM > 19 (e.g. the 1.99.0 output `23.1.1`) fails; 16.0.5 passes | unit | `tool_02_bitcode_rustc_llvm_above_19_fails` / `..._16_passes` | Cargo |
| TOOL-02 | `pins.toml` major is in 16..=19 and status is `provisional` | unit | `tool_02_pins_llvm_major_in_supported_range_and_provisional` | Cargo |
| TOOL-02 | pins decision ID exists in PROJECT.md with matching `llvm=`/`bitcode-rustc=` tokens (D-21) | unit | `tool_02_llvm_and_bitcode_pins_have_matching_project_decision` | Cargo |
| TOOL-02 | Only LLVM 16 packages installed; live versions match pins; off-pin run fails | integration | `mt toolchain check` exit 0; same command with a copy of `pins.toml` edited to `major = 17` exits non-zero | CI/container |
| TOOL-03 | Every sha256 is 64 hex, every commit is 40 hex, every digest `sha256:`+64 hex, no empty/placeholder values | unit | `tool_03_pins_digests_and_commits_are_well_formed` | Cargo |
| TOOL-03 | Dockerfile `FROM` lines pinned by `@sha256:`, no `:latest`, no `curl|sh`, no `apt` source other than snapshot | static lint | `tool_03_dockerfile_has_no_floating_references` | Cargo |
| TOOL-03 | Dockerfile `ARG`s and `mt toolchain build-args` keys match exactly | unit | `tool_03_dockerfile_args_match_pins` | Cargo |
| TOOL-03 | Manifest serialisation deterministic; comparer reports the first differing key | unit | `tool_03_manifest_is_byte_stable_and_diff_detects_change` | Cargo |
| TOOL-03 | Two builds from the same inputs give identical manifests | integration | CI: build A and build B (`--no-cache`), `mt toolchain manifest` in each, `diff -u` | CI/container |
| TOOL-04 | Smoke crate: `#![no_std]`, `staticlib`+`rlib`, `panic = "abort"` in dev and release, no dependencies, excluded from the root workspace | unit | `tool_04_smoke_crate_has_emitted_rust_shape` and `tool_04_smoke_crate_is_outside_the_workspace` | Cargo |
| TOOL-04 | `arm-none-eabi-gcc` present at the pinned release; smoke crate builds for `thumbv7em-none-eabihf`; C stub compiles with `arm-none-eabi-gcc` | integration | CI: `cargo build --release --target thumbv7em-none-eabihf --manifest-path container/smoke/Cargo.toml` and `arm-none-eabi-gcc ... -c smoke.c` inside the image | CI/container |

Note on the TOOL-04 build: it is deliberately **not** a `cargo test` (it would fetch a toolchain/target over the network on first use, violating "tests never use the network", and `#[ignore]` is forbidden by the guidelines). The Cargo-level tests prove its shape; the CI step proves it builds.

### Sampling Rate
- **Per task commit:** `cargo test -p mt-toolchain` (sub-second to a few seconds once built)
- **Per wave merge:** the full suite command above
- **Phase gate:** full suite green plus the container workflow green (double-build manifest diff empty, `mt toolchain check` exit 0 and negative-pin exit non-zero, smoke build) before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] Root `Cargo.toml`, `rust-toolchain.toml`, `deny.toml`, `clippy.toml`, `.gitignore` (`target/`), `research/.gitkeep`
- [ ] `crates/mt-toolchain` (runner, pins, version, check, manifest) and `crates/mt-cli` skeleton with the tests named above
- [ ] `crates/mt-toolchain/tests/fixtures/` tool-output fixtures (seeded from the version table; replaced by real CI captures)
- [ ] `container/pins.toml` (after pin discovery), `container/Dockerfile`, `container/smoke/`
- [ ] `.github/workflows/ci.yml`, `.github/workflows/container.yml`
- [ ] PROJECT.md decision D-15 (LLVM 16 / bitcode rustc 1.72.1) for the decision-ID gate

## Security Domain

`security_enforcement` is not set in `.planning/config.json` (absent = enabled).

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no (no user auth in this phase) | — |
| V3 Session Management | no | — |
| V4 Access Control | partly | GitHub Actions `permissions:` least privilege (`contents: read` by default; `packages: write` only on the push job); container runs non-root for check/smoke jobs where bind-mounts allow |
| V5 Input Validation | yes | `pins.toml` and tool output are untrusted-ish input: `serde` + `toml` with `deny_unknown_fields`, regex with named groups, bounded output capture (16 MiB cap), argv vectors only (no shell strings) |
| V6 Cryptography | yes (integrity only) | `sha2` for hashes; never hand-roll; sha256 verification of every downloaded artifact (rustup-init, Arm tarball, `.crate`, channel manifests); git commit assertion after clone |
| V10/V14 Supply chain & configuration | yes | digest-pinned base, snapshot apt, commit-pinned sources, `cargo install --locked`, action refs pinned by commit SHA, `cargo deny` (licences, advisories, sources), no `curl | sh` |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Tampered download (tarball, rustup-init, crate) | Tampering | sha256 in `pins.toml`, `sha256sum -c` before use; crate `cksum` check; git `rev-parse HEAD` equals pinned commit |
| Typosquatted/slopsquatted crate | Spoofing | Package legitimacy gate (done); exact `--version =x.y.z` plus `--locked`; c2rust human-verify checkpoint |
| Floating tag/latest drifting under the image | Tampering | `@sha256` base, snapshot timestamp, tests `tool_03_*` fail on floating refs |
| Command/argument injection into spawned tools | Elevation | runner takes `Vec<OsString>` argv, never a shell string; tool paths absolute from pins |
| Environment leakage into tool runs (tokens, locale, TZ) | Information disclosure | `env_clear()` + fixed base; effective env recorded |
| Output flooding / memory exhaustion | DoS | per-stream cap with streaming hash; timeouts |
| Path traversal / symlink loops in repo walk | Tampering/DoS | do not follow symlinks; skip list fixed |
| Secrets in image layers or build args | Information disclosure | no secrets as `ARG`/`ENV`; GHCR login only in push job via `GITHUB_TOKEN` |
| Third-party GitHub Actions compromise | Tampering | pin by commit SHA (see CI section); minimal action set |
| GPL tool contamination of the shipped binary | Legal | Bear/QEMU/GCC run only as container subprocesses; `cargo deny` licence allowlist covers only Rust crates in `mt` |

## CI Design (research questions 10 and 11)

**Workflows**
- `ci.yml` (every push/PR, `ubuntu-24.04`, `permissions: contents: read`): checkout → rustup auto-install from `rust-toolchain.toml` → `cargo fmt --check` → `cargo clippy --all-targets -- -D warnings` → `cargo test --workspace --locked` → `cargo deny check` (install with `cargo install --locked cargo-deny --version 0.20.2`, or the pinned action below).
- `container.yml` (paths `container/**`, `crates/mt-toolchain/**`, workflow file, plus `workflow_dispatch` with a `target` input and a weekly `schedule` to catch URL/snapshot rot):
  1. `mt` job: `cargo build --release --locked -p mt-cli --target x86_64-unknown-linux-musl`, upload artifact.
  2. `build-a` and `build-b` run **in parallel** on separate runners: `docker buildx build --platform linux/amd64 --load` with `mt toolchain build-args` output; `build-b` adds `--no-cache`. Each then runs `docker run --rm --network none -v mt:/usr/local/bin/mt:ro -v container/pins.toml:/opt/mt/pins.toml:ro IMAGE mt toolchain manifest` and uploads `manifest-{a,b}.json`.
  3. `compare` job: download both, `diff -u manifest-a.json manifest-b.json` (job fails on any difference) — TOOL-03.
  4. `check` (in build-a's runner, same image): `mt toolchain check` must exit 0; a second invocation against a pins copy with `major = 17` must exit non-zero — TOOL-02, success criterion 1.
  5. `smoke`: smoke crate build for `thumbv7em-none-eabihf` and `arm-none-eabi-gcc -c` — TOOL-04.
  6. `push` (needs 2-5, only on `main`/`workflow_dispatch`, `permissions: packages: write`): `docker/build-push-action` with `push: true`, `provenance: false`, `sbom: false`, image `ghcr.io/sat-wik/migrationtool-toolchain` (lowercase), record `steps.<id>.outputs.digest`; consumers reference `...@sha256:<digest>` (D-24).
- Free runner disk before image jobs (Pitfall 13). Use `--target` per stage in `workflow_dispatch` runs so a failing KLEE stage is debugged without rebuilding apt/Rust stages (BuildKit layer cache on a single runner within one job).

**Pinned action references** (commit SHAs resolved via `git ls-remote` on 2026-10-08; re-run `git ls-remote https://github.com/<repo> refs/tags/<tag> refs/tags/<tag>^{}` and take the `^{}` line when present):
| Action | Tag | Commit |
|--------|-----|--------|
| actions/checkout | v7.0.1 | `3d3c42e5aac5ba805825da76410c181273ba90b1` |
| docker/setup-buildx-action | v4.4.1 | `f87e5991a6d7451dcb8d9637bfbc97413f497069` |
| docker/build-push-action | v7.4.0 | `c3c9e263c25d99ce0380d002d59b67737d91b0dc` |
| docker/login-action | v4.6.0 | `dbcb813823bdd20940b903addbd779551569679f` |
| EmbarkStudios/cargo-deny-action | v2.1.1 (annotated, peeled) | `3c6349835b2b7b196a839186cb8b78e02f7b5f25` |
| actions/upload-artifact | v7.0.2 | `cf430e030ddbb5b0abf93d22962f4752f3646cd9` |
| actions/download-artifact | v8.0.2 | `9000827ccba6bdab643e8b6fd33ac0654aef8333` |
| Swatinem/rust-cache (optional) | v2.9.2 (annotated, peeled) | `6323deb102c322ba6fcbdcafc7e3dddab59af2b6` |

**Caching:** do not share cache between `build-a` and `build-b` (that is the point of the pair). The GHA cache backend has a per-repo size limit that multi-GB layers will evict; if iteration speed matters, push a build cache to `ghcr.io/...:buildcache` (`type=registry`) used only by dev `workflow_dispatch` runs.

**Timeouts [ASSUMED, A8]:** set job `timeout-minutes: 90` for build jobs, 20 for `ci.yml`; expect a cold image build of 40-60 min on a 4-vCPU public runner, longer on private 2-vCPU runners.

## Sizing and Plan Split (research question 15)

Threats to the 1-2 week budget: (1) CI-only iteration for KLEE, klee-uclibc and the c2rust C++ exporter with no local Docker, each loop 20-60 min; (2) apt/snapshot flakiness; (3) pin discovery requiring a CI round-trip before `pins.toml` can be completed; (4) the double-build doubling cost. Mitigations: stage-by-stage `--target` builds, the discovery workflow first, KLEE Z3-only/no-libc++/no-tests, pre-decided fallbacks (A2, A7) so a stuck stage becomes a recorded decision rather than an open-ended debug.

Suggested split (4 plans, 3 waves; sizes are relative, not time estimates):
1. **Plan 01 — Workspace, runner, repo checks (TOOL-01).** Cargo workspace, `rust-toolchain.toml`, `deny.toml`, `clippy.toml`, `ci.yml` (cargo job), `mt-toolchain` runner + tests, Python-ban test, Command-confinement and forbid-unsafe tests, `research/.gitkeep`. Fully verifiable in a cloud session.
2. **Plan 02 — Pins, check/manifest, decision gate, smoke crate (TOOL-02, TOOL-03 static, TOOL-04 shape).** `pins.toml` schema + parser, version parsers + fixtures, `mt toolchain check|manifest|build-args|hash`, PROJECT.md D-15, Dockerfile lints (written against a stub Dockerfile), smoke crate. Still Docker-free. Depends on Plan 01.
3. **Plan 03 — Image (TOOL-02/03/04 in container).** Pin-discovery workflow and PR to fill apt/Arm values, then Dockerfile stages in order apt-base → rustup → arm → cargo-tools → klee, each proven by `container.yml` `workflow_dispatch --target`. Highest risk; may be split into 03a (apt/rust/arm/cargo-tools) and 03b (KLEE + uclibc) if time runs short. Depends on Plan 02 (`build-args`, `check`).
4. **Plan 04 — Reproducibility, check and publish.** Double-build jobs, manifest diff, negative-pin check, smoke job, GHCR push by digest, `container/README.md` (how to pull the digest and run under emulation on the Mac, how to bump pins with `mt toolchain hash`, D-20/D-21 workflow). Depends on Plan 03.

## Sources

### Primary (HIGH confidence — executed or fetched this session)
- Rust toolchains: `rustup toolchain install` + `rustc -vV` for 1.69.0, 1.70.0, 1.71.1, 1.72.1, 1.73.0, 1.99.0; `static.rust-lang.org/dist/channel-rust-{1.72.1,1.99.0}.toml` (+ `.sha256`, `channel-rust-stable.toml` dated 2026-10-01); `static.rust-lang.org/rustup/archive/1.29.1/x86_64-unknown-linux-gnu/rustup-init(.sha256)`; `static.rust-lang.org/rustup/release-stable.toml`
- crates.io API/index: `crates.io/api/v1/crates/{c2rust,cargo-mutants,cargo-deny,...}`, `index.crates.io/...` cksums, `static.crates.io/crates/{c2rust,cargo-mutants}/*.crate`; `cargo install` of cargo-mutants 27.1.0 and cargo-deny 0.20.2; `cargo install --locked c2rust --version =0.22.1` run to the cmake step
- Docker Hub: `registry-1.docker.io/v2/library/debian/manifests/bookworm-20261005-slim`, `hub.docker.com/v2/repositories/library/debian/tags`
- KLEE v3.2 source (cloned, commit `92ee8201...`): `CMakeLists.txt`, `README-CMake.md`, `NEWS`, `Dockerfile`, `.github/workflows/build.yaml`, `scripts/build/p-{uclibc,stp,z3,klee,llvm-linux-ubuntu}.inc`, `lib/Support/PrintVersion.cpp`
- `git ls-remote` for klee/klee, klee/klee-uclibc, Z3Prover/z3, stp/stp and the GitHub Actions repos in the CI table
- immunant/c2rust v0.22.1 via raw.githubusercontent: `README.md`, `Cargo.toml`, `c2rust/Cargo.toml`, `c2rust-ast-exporter/build.rs`, `c2rust-ast-exporter/src/CMakeLists.txt`, `c2rust-build-paths/src/lib.rs`, `.github/workflows/internal-testsuite.yml`
- cargo-deny 0.20.2 `deny.template.toml` (raw.githubusercontent) and an actual `cargo deny check` run
- Project files: CONTEXT.md, REQUIREMENTS.md, STATE.md, PROJECT.md decisions, `docs/guidelines/building-the-tool.md`, `docs/specs/emitted-rust-rules.md`, `docs/research/VERIFICATION.md`, `.claude/CLAUDE.md`

### Secondary (MEDIUM confidence — search results quoting official pages)
- packages.debian.org / CSAIL mirror pages: clang-16 `1:16.0.6-15~deb12u1` in bookworm; bear 3.1.1-1 in bookworm; qemu-system-arm `1:7.2+dfsg-7+deb12u18` in bookworm
- Debian docker `debian/snapshot` description (Check-Valid-Until false, sources pointing at snapshot) and stale tag list (via Docker Hub API)
- docs.github.com hosted-runner hardware table (4 vCPU/16 GB public, 2 vCPU/8 GB private, 14 GB SSD)
- QEMU docs: `system/arm/mps2.html` (mps2-an386 Cortex-M4, mps2-an500 Cortex-M7)
- nixpkgs master `pkgs/by-name/gc/gcc-arm-embedded-15/package.nix` (15.3.rel1, gitlab.arm.com URL, hashes from `.sha256asc`) and the gcc-arm-embedded-14 / -13 packages (14.3.rel1 hash `8f6903f8...`, 13.3.rel1 hash `95c011ce...`)
- Arch AUR / Debian tracker entries confirming Arm 15.2.Rel1 (Dec 2025) and 15.3.Rel1 (Jul 2026)

### Tertiary (LOW confidence — not independently verified)
- Build-time and image-size estimates (A8); version-output strings for tools not runnable here (A5); package names (A10)

## Metadata

**Confidence breakdown:**
- Standard stack (Rust, registry, git, rustup, base digest): HIGH — executed or fetched.
- Standard stack (apt versions, Arm hash, Z3/uclibc compile): MEDIUM — cited/third-party; resolved by the discovery step and first CI runs.
- Architecture (runner, check, pins flow, CI shape): HIGH for the Rust prototype parts, MEDIUM for Dockerfile and workflow YAML (unbuilt).
- Pitfalls: HIGH where marked VERIFIED; MEDIUM otherwise.

**Research date:** 2026-10-08
**Valid until:** 2026-11-07 for the Rust/registry pins (stable ecosystem); re-check the Debian snapshot reachability and Arm release listing at plan execution time.
