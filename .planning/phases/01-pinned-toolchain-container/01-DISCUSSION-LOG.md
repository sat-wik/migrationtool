# Phase 1: Pinned Toolchain Container - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-10-08
**Phase:** 01-pinned-toolchain-container
**Areas discussed:** LLVM pin choice, Reproducibility method, Tool sourcing, Where it builds and runs, Manifest & check CLI, Runner env & sandbox, Version-bump policy, Bitcode rustc packaging

---

## LLVM pin choice

| Option | Description | Selected |
|--------|-------------|----------|
| LLVM 16 | KLEE 3.2 full support; best R1 odds; old bitcode rustc | ✓ |
| LLVM 18 | Middle ground; KLEE partial | |
| LLVM 19 | Newest allowed; weakest KLEE support | |
| You decide | Researcher picks | |

| Option | Description | Selected |
|--------|-------------|----------|
| Move the pin | Nearest 16-19 major all tools support, recorded as decision | ✓ |
| Stop and ask me | Halt the phase | |
| Allow a second LLVM | Breaks PROJECT D-08 | |

| Option | Description | Selected |
|--------|-------------|----------|
| Version check only | `rustc -vV` LLVM check; KLEE load is the R1 spike | ✓ |
| Add a smoke test | Compile Rust to bitcode and load in KLEE now | |

**Notes:** Claude tightened "≤ 19" to "matches the LLVM 16 pin" in CONTEXT D-03, since KLEE on LLVM 16 cannot read newer bitcode; the planner should confirm.

---

## Reproducibility method

| Option | Description | Selected |
|--------|-------------|----------|
| Dockerfile + snapshots | Digest base, snapshot.debian.org apt, sha256 tools | ✓ |
| Nix flake | Strongest reproducibility, steep curve | |
| Dockerfile, all from source | Max control, hours-long builds | |

| Option | Description | Selected |
|--------|-------------|----------|
| Docker only | Simplest | ✓ |
| Docker + Podman | Doubles test matrix | |

| Option | Description | Selected |
|--------|-------------|----------|
| Manifest-identical | Byte-identical tool-version manifest (TOOL-03) | ✓ |
| Also binary hashes | Stricter; timestamp risk | |

---

## Tool sourcing

| Option | Description | Selected |
|--------|-------------|----------|
| Build in a stage | KLEE 3.2 from pinned tag against LLVM 16 | ✓ |
| Base on klee/klee image | Fast; fixes base OS/LLVM | |
| You decide | | |

| Option | Description | Selected |
|--------|-------------|----------|
| Arm GNU Toolchain release | Official tarball, sha256 | ✓ |
| Debian package | Lags, distro-patched | |

| Option | Description | Selected |
|--------|-------------|----------|
| cargo install, locked | Pinned release | ✓ |
| Pinned git commit | More churn | |

| Option | Description | Selected |
|--------|-------------|----------|
| Leave them out | Hayroll/Kani listed as not installed | ✓ |
| Reserve manifest slots | Disabled entries | |

---

## Where it builds and runs

| Option | Description | Selected |
|--------|-------------|----------|
| x86-64 Linux | Native | |
| Apple Silicon Mac | amd64 emulated | ✓ |
| x86-64 Windows/WSL2 | Docker Desktop | |

| Option | Description | Selected |
|--------|-------------|----------|
| GitHub Actions now | Build twice, diff manifests, check, cargo gates | ✓ |
| Local only for now | Defer CI | |

| Option | Description | Selected |
|--------|-------------|----------|
| Only what's needed | mt-toolchain + mt-cli | ✓ |
| All as empty stubs | 14 crates | |

| Option | Description | Selected |
|--------|-------------|----------|
| Rust test in workspace | `tool_01_no_python_outside_research` | ✓ |
| CI script step | | |

Follow-up (Apple Silicon):

| Option | Description | Selected |
|--------|-------------|----------|
| CI builds, push to GHCR | Native amd64 runners; pull by digest | ✓ |
| Also an arm64 variant | Unsupported platform | |
| Local emulated builds | Slow | |

---

## Manifest & check CLI

| Option | Description | Selected |
|--------|-------------|----------|
| JSON | schema_version, sorted keys | ✓ |
| TOML | | |

| Option | Description | Selected |
|--------|-------------|----------|
| Committed pin file | `container/pins.toml` single source; diffable | ✓ |
| Manifest only in image | Weaker drift detection | |

| Option | Description | Selected |
|--------|-------------|----------|
| mt toolchain check | Namespaced | ✓ |
| mt doctor | | |

---

## Runner env & sandbox

| Option | Description | Selected |
|--------|-------------|----------|
| Empty + explicit | Cleared env + fixed base, recorded | ✓ |
| Inherit minus denylist | | |

| Option | Description | Selected |
|--------|-------------|----------|
| Defer to Phase 5/8 | Network isolation later | ✓ |
| Network off by default now | | |

| Option | Description | Selected |
|--------|-------------|----------|
| Hash + store, bounded | sha256 + full output up to cap | ✓ |
| Hashes only | | |

---

## Version-bump policy

| Option | Description | Selected |
|--------|-------------|----------|
| Manual PR + helper | Helper recomputes sha256s; CI diffs | ✓ |
| Renovate proposes | | |
| Manual only | | |

| Option | Description | Selected |
|--------|-------------|----------|
| No, decision required | LLVM major / bitcode rustc change needs a PROJECT.md decision; CI-enforced | ✓ |
| Treat like any bump | | |

---

## Bitcode rustc packaging

| Option | Description | Selected |
|--------|-------------|----------|
| rustup, both pinned | Exact versions; bitcode only via `+<ver>` through runner | ✓ |
| Standalone tarballs | | |

| Option | Description | Selected |
|--------|-------------|----------|
| Host target only | thumbv7em on the tool toolchain | ✓ |
| Both targets | | |

---

## Claude's Discretion

- Exact base release, snapshot date and tool versions; pins.toml/manifest field names; location of the no_std smoke crate; runner output cap.

## Deferred Ideas

- arm64 image variant; Podman support; KLEE bitcode smoke test (Phase 6); network isolation (Phases 5/8); Renovate/Dependabot.
