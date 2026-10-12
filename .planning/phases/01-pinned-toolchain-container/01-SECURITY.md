---
phase: "01"
slug: "pinned-toolchain-container"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: "2026-10-11"
---

# Phase 01 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.
> Register authored at plan time in 01-01..01-09-PLAN.md `<threat_model>` blocks; verified by gsd-security-auditor on 2026-10-11 (L1, block_on: high) against the code and GitHub Actions evidence (container run 38019633821 at 686b56b, pin-discovery 38019633805, ci 38021398693).

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| mt process → external tool subprocess | Every analyser runs through `mt-toolchain`'s runner (D-16) | argv vector, cleared + fixed environment, captured stdout/stderr |
| Upstream sources → image build | Base image, Debian snapshot, rustup, Rust dist, crates.io, Arm, KLEE/klee-uclibc/tinycbor git | Downloaded bytes, bound by digest / snapshot+version / sha256 / commit in `container/pins.toml` |
| pins.toml → Dockerfile build args | `mt toolchain build-args` renders pins into `--build-arg` lines | Validated scalar values (no whitespace/control chars) |
| Repository → GitHub Actions | ci, pin-discovery and container workflows | SHA-pinned actions, `contents: read`, `persist-credentials: false` |
| image-a job → GHCR | Publish of the verified image | `GITHUB_TOKEN` with `packages: write` in image-a only; not on pull_request |
| Executor → origin | CI round trips (ci_push_mode: executor-pushes) | Plain pushes to `claude/gsd-discuss-phase-1-mzdw5v` only; no force, never main |
| CI → annotations | Evidence published for Docker-less sessions | Versions, manifests, inventory, build-record hashes — no secrets |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-01-01 | Information disclosure | runner environment | high | mitigate | `env_clear()` + fixed base + named extras, base overrides rejected (runner.rs:296-317); test asserts HOME etc. absent | closed |
| T-01-02 | Elevation of privilege | runner argv | high | mitigate | argv as `Vec<OsString>`, no shell; `Command` only in runner.rs (clippy ban, D-16, repo_checks); absolute `bin` paths | closed |
| T-01-03 | Denial of service | output capture | medium | mitigate | per-pipe reader threads, hash every byte, 16 MiB cap; cap/deadlock tests | closed |
| T-01-04 | Denial of service | child timeout | medium | mitigate | mandatory timeout, kill, bounded drain, timeout → NotProven (residual WR-01) | closed |
| T-01-05 | Denial of service | regex compile | low | mitigate | `RegexBuilder.size_limit(1 MiB)`, single compile site | closed |
| T-01-06 | Spoofing | c2rust crate identity | high | mitigate | founder legitimacy approval (01-02) before install; sha256 pinned and `sha256sum -c` in build; CI record matches | closed |
| T-01-07 | Elevation of privilege | executor pushes | medium | mitigate | founder-chosen executor-pushes, branch-scoped; 32/32 pushes to phase branch, 0 force pushes | closed |
| T-01-08 | Tampering | pins.toml schema | high | mitigate | `deny_unknown_fields`, hex/digest/https/placeholder validation + tests | closed |
| T-01-09 | Tampering | build-args rendering | medium | mitigate | reject whitespace/control chars and duplicates; sorted output; tests | closed |
| T-01-10 | Denial of service | version parsing regex | low | mitigate | same 1 MiB-limited linear-time regex compile | closed |
| T-01-11 | Denial of service | repo walk | low | mitigate | symlinks skipped; root-only skip list | closed |
| T-01-12 | Tampering | dependency sources/licences | medium | mitigate | deny.toml licence allowlist, crates.io only; CI dependency policy green | closed |
| T-01-13 | Tampering | analyser library linking | medium | mitigate | deny.toml bans clang-sys/llvm-sys/inkwell/z3; Cargo.lock test | closed |
| T-01-14 | Repudiation | check verdicts | high | mitigate | missing/timeout/error/non-zero/signal never OK; exit 1 unless pass; tests | closed |
| T-01-15 | Tampering | hash helper fetch | medium | mitigate | https/file only, `--proto =https,file --proto-redir =https`, hash every byte (residual WR-02) | closed |
| T-01-16 | Repudiation | tool-version manifest | medium | mitigate | BTreeMap, sorted JSON, no timestamps/raw output; CI manifests byte-identical | closed |
| T-01-17 | Tampering | base image + apt | high | mitigate | base by digest; apt only from snapshot.debian.org at pinned timestamp; live apt rows OK | closed |
| T-01-18 | Tampering | workflow actions | high | mitigate | all 17 `uses:` pinned by 40-hex SHA; test enforces | closed |
| T-01-19 | Information disclosure | CI credentials | medium | mitigate | `contents: read`, `persist-credentials: false`, only GITHUB_TOKEN used; `.dockerignore` whitelist | closed |
| T-01-20 | Tampering | apt/Arm pins drift | medium | mitigate | live apt re-check; pin-discovery official = computed = pinned Arm sha; `sha256sum -c` in build | closed |
| T-01-21 | Repudiation | LLVM/bitcode pin change | medium | mitigate | test ties `llvm=16`/`bitcode-rustc=1.72.1` to PROJECT.md D-15 | closed |
| T-01-22 | Tampering | image tool sources | high | mitigate | `sha256sum -c` on rustup-init, manifests, archives, crates, Arm; `rev-parse HEAD` for git sources; lint rules | closed |
| T-01-23 | Tampering | c2rust exporter tinycbor | medium | mitigate | verified crate + tinycbor commit check (residual IN-14) | closed |
| T-01-24 | Elevation of privilege | in-image checks | medium | mitigate | `--network none --user 65534:65534 --read-only`, read-only mounts; verified run unprivileged (residual IN-08/IN-11) | closed |
| T-01-25 | Information disclosure | CI annotations | low | accept | see Accepted Risks Log | closed |
| T-01-26 | Elevation of privilege | GHCR publish | high | mitigate | `packages: write` only in image-a, GITHUB_TOKEN only, not on pull_request, SHA-pinned login-action; test (residual WR-04) | closed |
| T-01-27 | Tampering | published image identity | high | mitigate | push the exact gated image, digest-only references, `verified-image` only after manifest-diff | closed |
| T-01-28 | Repudiation | reproducibility claim | medium | mitigate | README claims manifest-identical only, no certified/compliant wording; test | closed |
| T-01-29 | Tampering | Rust toolchain install | high | mitigate | install only from `file://` mirror of pin-verified manifest + manifest-verified archives; test; CI record = pins | closed |
| T-01-30 | Tampering | rustup self-update | medium | mitigate | `--no-self-update` on every install; lint rule; rustup 1.29.1 row OK | closed |
| T-01-31 | Tampering | crate install source | high | mitigate | `sha256sum -c` → unpack → `cargo install --locked --path`; test; CI record = pins | closed |
| T-01-32 | Repudiation | pin-binding evidence | medium | mitigate | build record + whole-line gate before GHCR login/push, fails job on mismatch (residual WR-07/IN-12) | closed |
| T-01-33 | Spoofing | mirror URLs | low | mitigate | refuse `..` and non-`static.rust-lang.org/dist/` URLs; manifest pinned | closed |
| T-01-34 | Tampering | crate archive extraction | low | accept | see Accepted Risks Log | closed |
| T-01-35 | Information disclosure | pin-binding notice | low | accept | see Accepted Risks Log | closed |
| T-01-SC | Tampering | package installs (all plans) | high | mitigate | approved crates, exact pins + committed Cargo.lock + `--locked`; c2rust legitimacy gate; cargo-deny pinned; crates from verified files | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-01-01 | T-01-25 | CI annotations are public on a public repo; they carry only tool versions, manifests, inventory and build tails, the build runs before GHCR login, and no secret reaches any annotation (premise re-verified in container.yml) | Plan-time disposition (01-07-PLAN.md); verified by gsd-security-auditor | 2026-10-11 |
| AR-01-02 | T-01-34 | Crate archives are extracted only after `sha256sum -c` against the pin, with `tar --no-same-owner`, in a throwaway stage whose `/tmp/crates` is removed; the final stage copies only `/opt/cargo-tools` | Plan-time disposition (01-09-PLAN.md); verified by gsd-security-auditor | 2026-10-11 |
| AR-01-03 | T-01-35 | The pin-binding notice publishes only hashes from the build record, update-hashes and stored-manifest hashes; the binding container receives only `RUSTUP_HOME`, never GITHUB_TOKEN | Plan-time disposition (01-09-PLAN.md); verified by gsd-security-auditor | 2026-10-11 |

*Accepted risks do not resurface in future audit runs.*

**Auditor-observed, non-blocking (not in the register):** the `mt-bundle` artifact is built in job `mt` and downloaded/executed by image-a (inside `--network none`), image-b and compare (on a `contents: read` runner) without a digest check between upload and download. Same run and commit, SHA-pinned actions, 7-day retention. Add a threat row or accepted-risk entry in a later phase.

**Residual hardening items** (tracked open in 01-REVIEW-DISPOSITION.md; none removes a declared mitigation): WR-01 (timeout kills only the direct child — fix before forking analysers such as KLEE run through the runner), WR-02, WR-04, WR-05, WR-06, WR-07, IN-08, IN-11, IN-12, IN-14.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-11 | 36 | 36 | 0 | gsd-security-auditor (ASVS L1, block_on high) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-10-11
