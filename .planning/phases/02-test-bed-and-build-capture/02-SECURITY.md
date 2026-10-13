---
phase: "02"
slug: "test-bed-and-build-capture"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: "2026-10-13"
---

# Phase 02 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.
> Register authored at plan time in the 02-01..02-10-PLAN.md `<threat_model>` blocks; verified by gsd-security-auditor on 2026-10-13 (L1, block_on: high) against the code, `cargo deny --locked check`, `cargo test --workspace --locked` (250 passed) and GitHub evidence (PR #4, main publish run 38342425433, final testbed run 38430137113).

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| Debian snapshot archive -> image | five more packages and their dependencies enter the final stage | see description |
| branch push -> GitHub Actions | executor pushes trigger pin-discovery, ci and container on the session branch | see description |
| session branch -> main | the PR merge publishes the image and changes the default branch | see description |
| GHCR publish job -> README | the digest recorded here is what every later container run pulls | see description |
| testbed/manifest.toml -> mt-capture | every URL, path, flag and placeholder the later steps act on comes from this file | see description |
| upstream licence claims -> committed data | the licence recorded decides what may be published later | see description |
| compile_commands.json (Bear, CMake, fixtures) -> replay | every argv mt executes comes from this file | see description |
| object files -> ELF reader | objects produced by untrusted upstream code are parsed | see description |
| GitHub and the Mbed TLS release host -> work directory | upstream bytes enter only through fetch | see description |
| fetched tree -> build | upstream Makefiles and CMake scripts run during capture | see description |
| clang output -> entry check | parsed JSON from a compiler run on upstream headers | see description |
| GitHub Actions runner -> container | upstream builds run inside the image with host directories mounted | see description |
| internet -> fetch container | the only networked step | see description |
| workflow -> GitHub API | the job token and its scope | see description |
| upstream test files and fuzz archives -> extractor | arbitrary bytes and archive structures are parsed | see description |
| extractor -> committed corpus directory | the only place the library writes | see description |
| upstream test data -> committed repository | seed bytes and their licences enter the repository | see description |
| session-host git fetch -> seed extraction | trees fetched outside the container for this one purpose | see description |
| 14 more upstream builds -> container | more untrusted Makefiles and CMake scripts execute | see description |
| drop log -> phase verdict | what is counted as retained decides whether the phase passes | see description |
| CI annotations -> committed fixtures | text produced by upstream builds in the image enters the repository as test data | see description |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-02-01 | Tampering | new apt packages (bzip2, cmake, curl, git, make) | high | mitigate | versions only from the pin-discovery annotation at the pinned snapshot; installed versions re-checked by the dpkg rows of `mt toolchain check` in image-a; tool_03_discovery_package_list_matches_apt_pins and tool_03_dockerfile_installs_every_apt_pin keep the three lists equal (02-01) | closed |
| T-02-02 | Elevation of privilege | container and pin-discovery workflows on branch pushes | medium | mitigate | no workflow file except pin-discovery's package list changes; packages write stays in the publish job guarded to a push to main (tool_03_workflow_publishes_by_digest_with_least_privilege unchanged); pushes go to the session branch only, never forced (02-01) | closed |
| T-02-03 | Repudiation | GPL tools added to the image (git, make) | low | mitigate | README Notices name each licence and say subprocess only; tool_01_never_links_an_analyser_library still guards linking (02-01) | closed |
| T-02-04 | Tampering | PR content merged to main | high | mitigate | the PR body lists every commit (`git log origin/main..HEAD`); no push between PR creation and merge (Task 1 acceptance compares the remote head with the recorded PR head); the founder reviews and merges by hand (02-02) | closed |
| T-02-05 | Elevation of privilege | GHCR publish token | high | mitigate | no workflow change in this plan; packages write exists only in the publish job, guarded to a push to main (tool_03_workflow_publishes_by_digest_with_least_privilege) (02-02) | closed |
| T-02-06 | Repudiation | merge-back of origin/main | medium | mitigate | merge only (fast-forward allowed), never rebase or force push; conflicts stop for the founder (02-02) | closed |
| T-02-07 | Spoofing | digest recorded under Current image | high | mitigate | digest copied from the verified-image notice of the main push run whose head_sha equals origin/main; exactly one pullable reference in the README (Task 3 verify) (02-02) | closed |
| T-02-08 | Tampering | manifest typos that silently disable a check | medium | mitigate | `deny_unknown_fields` on every struct; validation collects every issue with its field path; bed_01_manifest_rejects_unknown_keys (02-03) | closed |
| T-02-09 | Repudiation | licence mislabelling of a target | high | mitigate | spdx plus elected plus licence file at the pinned commit; validator rejects copyleft-only and unelected OR expressions; founder approval at the D-03 checkpoint; PROJECT D-19 records the reading (02-03) | closed |
| T-02-10 | Tampering | path traversal through manifest paths (licence file, header, build dir) | medium | mitigate | safe-relative-path rule (no absolute path, no `..`, no NUL) checked by validate; bed_01_manifest_rejects_unsafe_paths (02-03) | closed |
| T-02-11 | Tampering | flag strings smuggling extra placeholders or shell text | medium | mitigate | only the four placeholders are allowed; strings are argv elements later, never passed to a shell by mt (plan 02-04) (02-03) | closed |
| T-02-12 | Tampering | tree hash following a symlink out of the tree | medium | mitigate | the walk never follows symlinks and hashes the link text; special files are an error (02-03) | closed |
| T-02-13 | Elevation of privilege | shell injection through `command` strings | high | mitigate | `shlex::split` into argv; runner spawns the argv directly, never `sh -c`; build_01_cmake_command_strings_split_into_argv (02-04) | closed |
| T-02-14 | Elevation of privilege | tampered compile_commands or fixture runs an arbitrary program | high | mitigate | argv[0] allow-list checked before any spawn in canonicalise and again in replay; build_01_rejects_non_allowlisted_compiler, build_02_replay_refuses_non_allowlisted_compiler (02-04) | closed |
| T-02-15 | Tampering | replay writing outside the run tree (`directory` or `-o` escaping, or a symlink in the restored tree) | medium | mitigate | directory and output must resolve under run_root (OutsideRunRoot) before replay starts; copy_tree refuses absolute or escaping links; build_01_rejects_paths_outside_run_root, bed_01_tree_copy_preserves_symlinks_and_refuses_escapes (02-04) | closed |
| T-02-16 | Denial of service | malformed or hostile object files | medium | mitigate | `object` crate read-only bounds-checked parsing; errors become differs with detail, never a panic (02-04) | closed |
| T-02-17 | Denial of service | a compile that hangs or floods output | medium | mitigate | runner timeout gives not_proven, 16 MiB output cap, process-tree kill; never reported as identical (02-04) | closed |
| T-02-18 | Tampering | upstream repository or tag moved or rewritten | high | mitigate | fetch by commit SHA, `rev-parse HEAD` equality, tree sha256 equality before acceptance and on every reuse (bed_01_fetch_rejects_tree_hash_mismatch, bed_01_fetch_reuse_verifies_cache) (02-05) | closed |
| T-02-19 | Elevation of privilege | hostile git configuration, hooks or submodules | high | mitigate | GIT_CONFIG_NOSYSTEM=1, GIT_CONFIG_GLOBAL=/dev/null, GIT_ALLOW_PROTOCOL=https, --no-recurse-submodules, fresh `git init` (no upstream .git/config or hooks), HOME under the work dir (02-05) | closed |
| T-02-20 | Tampering | archive path traversal or swapped tarball | high | mitigate | archive sha256 checked before extraction; extraction into a fresh partial directory with --no-same-owner; tree sha256 after extraction; the archive bytes are pinned (02-05) | closed |
| T-02-21 | Tampering | symlink escape while copying cached trees into run directories | medium | mitigate | capture copies only with treehash::copy_tree (plan 02-04), which recreates links without following them and refuses absolute or escaping targets (02-05) | closed |
| T-02-22 | Elevation of privilege | upstream build scripts executed by capture | high | mitigate | real targets are captured only inside the pinned image under `--network none`, read-only root, non-root user (plan 02-06 workflow and its static test); offline tests build only test-authored sources (02-05) | closed |
| T-02-23 | Denial of service | very large clang AST output | low | mitigate | the probe file limits the dump to one header; an incomplete stream is not_proven, never parsed as found (02-05) | closed |
| T-02-24 | Information disclosure | network access during capture and replay (exfiltration or hidden downloads by upstream builds) | high | mitigate | run_offline adds `--network none` for validate, capture and replay; only fetch is networked; build_02_workflow_runs_capture_and_replay_without_network (02-06) | closed |
| T-02-25 | Elevation of privilege | `docker run` mounts and privileges | high | mitigate | user 65534:65534, `--read-only`, `--cap-drop ALL`, `no-new-privileges`, tmpfs /tmp; mt, pins and testbed mounted `:ro`; only /work and /out writable; no Docker socket and no secrets (static test) (02-06) | closed |
| T-02-26 | Elevation of privilege | workflow token scope | high | mitigate | `permissions: contents: read`, no `packages: write`, `persist-credentials: false`, SHA-pinned actions (tool_03_workflows_pin_actions_by_commit_sha covers every workflow) (02-06) | closed |
| T-02-27 | Spoofing | wrong or moved image | high | mitigate | image pulled by digest from `## Current image` only; manifest `[image] digest` must equal it (build_02_image_digest_matches_readme) (02-06) | closed |
| T-02-28 | Information disclosure | annotations and artifacts | low | accept | they carry reports of public upstream builds and fixed run-record environments (PATH, LANG, TZ, SOURCE_DATE_EPOCH, HOME, TMPDIR, GIT_*); no secret exists in the job (02-06) | closed |
| T-02-29 | Tampering | committed CI fixtures | medium | mitigate | SOURCE.md records the run; fixtures are parsed and re-serialised byte-identically by tests; argv[0] of every fixture entry must be the pinned compiler (02-10) | closed |
| T-02-30 | Tampering | zip member names (zip slip, absolute names) | high | mitigate | names only via `enclosed_name()`; members are kept in memory and written solely as sha256-named files by write_corpus (build_03_zip_members_are_read_without_touching_disk) (02-07) | closed |
| T-02-31 | Denial of service | zip bombs and huge members | medium | mitigate | bounded `take(max_member_bytes + 1)` reads; per-seed, count and total caps (02-07) | closed |
| T-02-32 | Denial of service | pathological source files for the lexer | low | mitigate | single linear pass, no regex or recursion; plan 02-08 skips source files above 8 MiB as unsupported (02-07) | closed |
| T-02-33 | Repudiation | licence contamination of committed seeds | high | mitigate | seeds only from declared seed_source paths in permissive directories (build_03_seeds_come_only_from_declared_sources); NOTICE per target; index licence equals manifest licence (02-08) | closed |
| T-02-34 | Tampering | host-fetched trees used for extraction | high | mitigate | `mt testbed fetch --all` verifies commit, archive sha256 and tree sha256 of every host-filled cache before use; CI's `seeds --check` regenerates from container-fetched trees and fails on any drift (02-08) | closed |
| T-02-35 | Tampering | research helper scripts | low | mitigate | Python only under research/ (tool_01_no_python_outside_research); the tool and tests never execute them; their outputs are committed as reviewable hex text (02-08) | closed |
| T-02-36 | Information disclosure | seeds treated as hidden in later phases | low | accept | D-14: Phase 2 makes no hiding claim; the held-out split is designed in GATE-05 (Phase 7) (02-08) | closed |
| T-02-37 | Repudiation | dropped or failing targets presented as passing | high | mitigate | dropped pairs reported as dropped and excluded from `passed`; drop entries validated against the manifest; DROPPED.md kept in step by test; floor of 10 is a test (02-09) | closed |
| T-02-38 | Tampering | a fix attempt that weakens a gate | high | mitigate | fix attempts only in manifest data or committed inputs; gates, normalisation and comparison code are not touched by this plan (files_modified excludes them); every attempt recorded (02-09) | closed |
| T-02-39 | Elevation of privilege | additional untrusted builds | high | mitigate | same container controls as the tracer: `--network none`, read-only root, non-root user, no capabilities, no secrets (build_02_workflow_runs_capture_and_replay_without_network) (02-09) | closed |
| T-02-40 | Tampering | Mbed TLS user config altering what is validated | low | mitigate | the file is committed, hashed into every Mbed TLS capture report as an input, and part of the declared configuration (02-09) | closed |
| T-02-SC | Tampering | npm/pip/cargo installs, phase-wide (one row, identical in every Phase 2 plan) | high | mitigate | Crates enter the workspace only in plan 02-04 (shlex 2.0.1; object 0.40.0 with read_core, elf and std) and plan 02-07 (zip 9.0.2, deflate backend only); each has RESEARCH Package Legitimacy Audit verdict OK, is pinned to an exact version with minimal features, and is gated by `cargo deny check` (licences and advisories) in the task that adds it and in the verification of every code plan that follows it; mt-capture and mt-seeds are path crates, not installs, and the workspace crates they reuse (serde, serde_json, sha2, thiserror, toml, regex, tempfile) were already locked and deny-checked in Phase 1; no pip or npm install anywhere (the plan 02-08 research helpers use the Python standard library only); Debian packages enter only from the digest-pinned snapshot archive at the versions pin discovery reports (plan 02-01, T-02-01) (02-01) | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

The auditor verdict has the evidence for each row, as file:line references and test names. The audit trail below summarises it.

### Notes from verification

None of these is an open threat.

- **Determinism exclusions (02-06, 02-09).** The testbed workflow runs capture twice and compares the results. Only the stdout sha256 and byte counts of the `cmake configure` and `clang ast <fn>` steps are left out of that comparison. Every notice states the exclusion. No static test pins the exclusion list yet.
- **CMake hash-scope classifier (T-02-38).** `is_cmake_product` in `capture.rs` excludes `CMakeFiles/Makefile2` and the `CMakeDetermineCompilerABI_*.bin` probes from the configure-time hashes (commit f0a36b8, RED test 5c6576d). This fixes nondeterminism in the harness; it is not a fix attempt for a failing target. The compile gates, normalisation and replay comparison are unchanged. T-02-38 is closed on that reading and flagged for founder review.
- **zip backend (T-02-SC).** `zip =9.0.2` is built with `deflate-flate2-zlib-rs` only, so deflate is the only format and encryption is off. zip also adds `nix` and `cfg_aliases` to Cargo.lock. Both are macOS/BSD-only target dependencies and are absent from the Linux dependency graph.
- **Archive bound (T-02-31).** `seeding.rs` does not open zip archives over 2 MiB, which bounds the member count.

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-02-01 | T-02-28 | Low severity. CI annotations publish only versions, hashes, reports and fixture data. The testbed workflow has `contents: read`, uses no `secrets.` and runs with a fixed environment (02-06-PLAN register). | plan-time disposition, verified by gsd-security-auditor | 2026-10-13 |
| AR-02-02 | T-02-36 | Low severity. Phase 2 hides no seeds. The held-out split is deferred to Phase 7 (GATE-05) under founder decision D-14 (02-08-PLAN register, testbed/README.md). | founder (D-14) | 2026-10-13 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-13 | 41 | 41 | 0 | gsd-security-auditor (L1, block_on high) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-10-13
