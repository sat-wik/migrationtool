---
phase: "02"
slug: "test-bed-and-build-capture"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: "2026-10-13"
updated: "2026-10-20"
---

# Phase 02 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.
> Register authored at plan time in the 02-01..02-19-PLAN.md `<threat_model>` blocks; verified by gsd-security-auditor on 2026-10-13 (T-02-01..T-02-40, T-02-SC) and again on 2026-10-20 after the gap closure (all 67 re-checked; L1, block_on: high) against the code at 8f8f267, `cargo deny --locked check`, `cargo test --workspace --locked --offline` (318 passed) and GitHub evidence (PR #4, main publish run 38342425433, final testbed run 39335185551 and ci 39335185566 on fb04867).

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
| upstream TU source -> probe file | the defining TU's text decides what the probe includes before the header | see description |
| entry check -> capture verdict | a wrong pass would claim an entry exists with a signature the build does not see | see description |
| CMake build directory -> configure-time evidence | generated files decide what replay may keep and what determinism compares | see description |
| crates.io -> workspace | new crates enter the build through the backend switch | see description |
| pinned upstream archives -> seed reader | fuzz corpora are untrusted bytes inflated in memory | see description |
| capture outputs -> determinism verdict | files written by the capture steps decide whether the bed is called deterministic | see description |
| upstream test sources -> seed corpora | what the harness later treats as real inputs comes from declared upstream text | see description |
| manifest -> extraction | the manifest decides which bytes become seeds | see description |
| work-directory reports -> replay | capture.json is read from a directory upstream builds can reach until plan 02-19 separates it | see description |
| GitHub Actions runner -> containers | upstream builds run inside the image with host directories mounted | see description |
| upstream build -> work directory | untrusted Makefiles and CMake scripts run as the container user | see description |
| build stage -> record stage | the build record and the build's outputs cross from untrusted to trusted code | see description |
| upstream build container -> host directories | untrusted build code runs with the mounts run_build gives it | see description |
| record, replay and comparison containers -> evidence | trusted mt code writes and reads the evidence | see description |

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
| T-02-41 | Spoofing | include_prefix guessing a context the TU does not have, so a declaration passes that the TU never sees | medium | mitigate | the prefix is only #include, #define and #undef lines reproduced exactly; any conditional, other directive or code falls back to the header-alone probe; name, signature and -MM checks unchanged (bed_01_include_prefix_refuses_what_it_cannot_reproduce, bed_01_entry_missing_function_still_fails_with_a_prefix) (02-11) | closed |
| T-02-42 | Tampering | include names from untrusted source written into the probe file | medium | mitigate | quoted names are emitted only as resolved absolute lexical paths passing probe_source's quote, backslash and control-character refusal; unresolved names are emitted in angle form; TU read capped at 8 MiB (02-11) | closed |
| T-02-43 | Repudiation | a dropped target reopened without evidence | medium | mitigate | the reopening and its reason are in DROPPED.md; isotp counts as retained only while the drop log says so, and plan 02-17's full-bed run must show it captured and identical or re-drop it with cause and run id (02-11) | closed |
| T-02-44 | Tampering | a configure-time input hidden from the evidence by a broad exclusion rule (WR-10) | medium | mitigate | exact product paths only; archives excluded only when the build step created them; build_01_cmake_products_are_exactly_the_documented_paths and build_01_configure_files_skip_only_build_created_archives (02-12) | closed |
| T-02-45 | Repudiation | a normalisation that hides a real change in Makefile2 | low | mitigate | only runs of one target's prerequisite lines are sorted; any changed prerequisite, moved line or reordered run changes the hash (build_02_make_driver_normalisation_sorts_runs_of_one_target); the rule text is stored with the hash; objects are never normalised by it (02-12) | closed |
| T-02-46 | Tampering | crates added by plan 02-13 (expected miniz_oxide, adler2), not covered by the RESEARCH audit | high | mitigate | lock change computed in a scratch copy first; blocking founder verification on crates.io of name, version, checksum, licence and repository before any build (Task 1, never auto-approved); exact pins; Task 2 refuses a lock diff other than the approved one; cargo deny check (02-13) | closed |
| T-02-47 | Denial of service | decompression bomb in a pinned zip archive (WR-07) | medium | mitigate | per-archive budget MAX_ZIP_DECOMPRESSED_BYTES on bytes read, per-member read bound kept, archives above 2 MiB still unopened; build_03_zip_archive_has_a_total_decompressed_budget (02-13) | closed |
| T-02-48 | Tampering | memory-safety defect in the inflater reading untrusted archives | medium | mitigate | inflater is miniz_oxide (forbids unsafe code); zlib-rs and C backends absent (build_03_zip_inflates_with_miniz_oxide_only); residual unsafe in zip header parsing and flate2 buffers recorded, not claimed away (02-13) | closed |
| T-02-49 | Repudiation | determinism reported identical for truncated or invalid reports (WR-05) | medium | mitigate | parse failures, empty files, one-sided files and zero compared files are differences (build_02_compare_fails_closed_on_unparseable_reports, build_02_compare_flags_empty_and_one_sided_files, build_02_compare_with_no_files_does_not_pass); plan 02-17 makes the workflow step use it (02-14) | closed |
| T-02-50 | Tampering | exclusions widened so a real difference is ignored | low | mitigate | two exclusions in one constant, applied to capture reports only and to three named fields of two step kinds; build_02_compare_excludes_only_the_documented_step_outputs (02-14) | closed |
| T-02-51 | Repudiation | seeds presented as upstream inputs that are messages or labels | low | mitigate | narrowed sources documented in the manifest with reasons; committed-data test on methods and paths; samples quoted in the SUMMARY; hand-made seeds stay marked proposed (02-15) | closed |
| T-02-52 | Tampering | a selector that silently changes other targets' corpora | low | mitigate | c_string_literals unchanged (test), seeds --all --check before and after, regeneration limited to the three ids and checked by diff (02-15) | closed |
| T-02-53 | Spoofing | a stale capture report or one made with other tools replayed as current evidence (WR-02) | high | mitigate | target, config, commit, tree sha256 and tools compared with the manifest and pins before replay; build_02_replay_refuses_a_capture_from_another_pin, build_02_replay_refuses_a_capture_made_with_other_tools (02-16) | closed |
| T-02-54 | Tampering | path traversal through `source.commit` in capture.json (WR-02) | medium | mitigate | the pristine path is built from the manifest's validated commit only; a traversal value is refused as a pin mismatch before any path is built (02-16) | closed |
| T-02-55 | Repudiation | an --all run below 10 retained targets, or with every target dropped, printing a pass (WR-08) | high | mitigate | BelowFloor in load_inputs for Selection::All, no override; build_02_an_all_run_below_the_floor_is_refused; the workflow is pinned to --all in plan 02-17 (02-16) | closed |
| T-02-56 | Repudiation | a `differs` drop recorded without the differing objects and sections (WR-08) | low | mitigate | DropLog::validate refuses it; build_02_drop_log_requires_evidence_for_differs (02-16) | closed |
| T-02-57 | Denial of service | a runaway upstream build or fork loop exhausting the runner (WR-04) | medium | mitigate | --pids-limit, --memory, --memory-swap and --cpus on every helper, pinned by the network static test; the 45-minute job timeout stays (02-17) | closed |
| T-02-58 | Repudiation | the workflow's determinism step passing on unparseable reports (WR-05) | medium | mitigate | the step's verdict is the exit code of mt testbed compare (fail closed, plan 02-14); build_02_workflow_determinism_step_uses_the_fail_closed_comparison (02-17) | closed |
| T-02-59 | Repudiation | the determinism comment claiming fewer exclusions than the code applies (G-02-2) | low | mitigate | build_02_determinism_comment_lists_every_exclusion reads the code constants (02-17) | closed |
| T-02-60 | Repudiation | an evidence run on a subset of the bed sidestepping the floor | medium | mitigate | build_02_workflow_runs_the_whole_bed pins TESTBED_SELECTION to --all (02-17) | closed |
| T-02-61 | Tampering | an upstream build rewriting capture.json, compile commands or object copies that replay trusts (WR-04) | high | mitigate | build stage writes only the pair's run directory and never the capture directory (build_01_build_stage_writes_only_the_run_directory); the record stage, which runs no upstream code, writes the evidence and reads the run directory without following symlinks, so a build cannot point it at another pair's files (build_01_run_dir_reads_refuse_symlinks, build_01_record_stage_does_not_follow_symlinks); plan 02-19 enforces it with per-pair container mounts (02-18) | closed |
| T-02-62 | Tampering | the build's copy of the committed inputs altered so the build and replay agree on a different configuration | medium | mitigate | the record stage hashes the committed inputs from testbed/ and refuses a differing run copy (build_01_record_stage_refuses_changed_inputs) (02-18) | closed |
| T-02-63 | Spoofing | a forged build record (steps, flags, before listing) | low | accept | written by the same pair's build, which could equally have built other objects; it cannot change source verification, tools, inputs, other pairs or the replay comparison; a missing or mismatched record is not_proven (build_01_record_stage_refuses_a_record_for_another_pair); residual recorded in the SUMMARY (02-18) | closed |
| T-02-64 | Tampering | an upstream build rewriting earlier pairs' capture artifacts, reports or the first capture before the determinism comparison (WR-04) | high | mitigate | one run_build container per pair; writable mounts only a discarded scratch /work and the pair's run directory; cache read-only; no /out; rerun in its own work directory; build_02_untrusted_builds_cannot_write_the_evidence (02-19) | closed |
| T-02-65 | Elevation of privilege | an upstream build escaping its container's limits or privileges | high | mitigate | --network none, --read-only, user 65534, --cap-drop ALL, no-new-privileges, tmpfs /tmp, no Docker socket, no secrets, pinned by the network static test (02-19) | closed |
| T-02-66 | Denial of service | a runaway build or fork loop in any container (WR-04) | medium | mitigate | --pids-limit, --memory, --memory-swap and --cpus on all four helpers; job timeout 45 minutes (02-19) | closed |
| T-02-SC | Tampering | npm/pip/cargo installs, phase-wide (one row, identical in every Phase 2 plan) | high | mitigate | Crates enter the workspace only in plan 02-04 (shlex 2.0.1; object 0.40.0 with read_core, elf and std) and plan 02-07 (zip 9.0.2, deflate backend only); each has RESEARCH Package Legitimacy Audit verdict OK, is pinned to an exact version with minimal features, and is gated by `cargo deny check` (licences and advisories) in the task that adds it and in the verification of every code plan that follows it; mt-capture and mt-seeds are path crates, not installs, and the workspace crates they reuse (serde, serde_json, sha2, thiserror, toml, regex, tempfile) were already locked and deny-checked in Phase 1; no pip or npm install anywhere (the plan 02-08 research helpers use the Python standard library only); Debian packages enter only from the digest-pinned snapshot archive at the versions pin discovery reports (plan 02-01, T-02-01) (02-01) | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

Per-row evidence (file:line and test names) is in the auditor verdicts summarised in the audit trail below.

### Notes from verification

None of these is an open threat.

- **Gap closure re-check (2026-10-20).** The 26 gap-closure threats are T-02-41..T-02-66. After the gap closure rewrote the code and workflow that the earlier mitigations depend on, these were re-checked and still hold:
  - T-02-22, T-02-24, T-02-25 and T-02-39: the four docker helpers, including the per-pair `run_build`. They are now tighter.
  - T-02-14, T-02-15 and T-02-18: replay and capture, after the pin binding and the stage split.
  - T-02-30 and T-02-31: zip, after the miniz_oxide switch and the 8 MiB budget. These are strengthened.
  - T-02-37 and T-02-38: the drop log, after the isotp reopening and the 10-target floor.
- **Determinism exclusions (02-06, 02-12, 02-14, 02-17).** These are now code constants: `DETERMINISM_EXCLUSIONS`, `CONFIGURE_HASH_EXCLUSIONS` and `NORMALISED_CONFIGURE_FILES`.
  - The comment on the determinism step lists every entry, and a static test (`build_02_determinism_comment_lists_every_exclusion`) holds the comment to the constants.
  - `CMakeFiles/Makefile2` is no longer excluded. It is hashed after its runs of prerequisite lines are sorted. The rule text is stored next to the hash.
  - The comparison is `mt testbed compare`, which fails closed. It replaces the jq pipeline.
- **zip stack (T-02-46, T-02-48).** The founder approved three added crates by name, version and checksum, with the exact lock diff (02-13): miniz_oxide 0.9.1, adler2 2.0.1 and simd-adler32 0.3.10. zlib-rs is removed.
  - Approved wording: the inflater (miniz_oxide) and adler2 forbid unsafe code; simd-adler32, zip's header parsing and flate2's buffer code contain unsafe code; there is no C code. The zip reading stack is not entirely safe Rust.
  - The member-count cap from WR-07 is deferred by founder decision. The 2 MiB archive cap bounds the central directory, and so bounds the member count indirectly.
- **Unregistered: recorded compile flags at replay (relates to T-02-14 and T-02-64).** This predates the gap closure and is not demonstrated as exploitable.
  - The compile-command gate checks argv[0], `-c`, run-root paths and `-flto*`/`-fprofile-*` only.
  - Replay passes the rest of the recorded argv to the pinned gcc unchanged. Driver options that execute helpers, such as `-wrapper`, `-B<dir>`, `-specs` or plugins, would therefore run inside the offline replay container, where `/work` and `/out` are writable.
  - Exploiting this needs a pinned, hash-verified malicious upstream.
  - Suggested for registration (medium) or explicit acceptance in a later phase.
- **Unregistered residual: replay follows symlinks (02-19).** Replay reads the run directory with plain `fs::read`, so it follows symlinks there. This is bounded by the offline replay container and by the hashes the record stage wrote.
- **Documentation lag.** `testbed/DROPPED.md` and `testbed/README.md` still say the isotp reopening awaits "the next testbed run". Runs 39331139047 and 39335185551 have since proven it.

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-02-01 | T-02-28 | Low severity. CI annotations publish only versions, hashes, reports and fixture data. The testbed workflow has `contents: read`, no `secrets.` and a fixed run environment (02-06-PLAN register). | plan-time disposition, verified by gsd-security-auditor | 2026-10-13 |
| AR-02-02 | T-02-36 | Low severity. No seed is hidden in Phase 2; the held-out split is deferred to Phase 7 (GATE-05) by founder decision D-14 (02-08-PLAN register, testbed/README.md). | founder (D-14) | 2026-10-13 |
| AR-02-03 | T-02-63 | Low severity. A pair's own build writes its build record. It can falsify only that pair's steps, flags and `before` listing. It cannot change source verification, tools, inputs, other pairs or the replay comparison. The record stage binds the record to target, config and verified source (`build_record_mismatch`). Recorded in 02-18-SUMMARY "Residual trust" and in the capture.rs module doc. | plan-time disposition (02-19), verified by gsd-security-auditor | 2026-10-20 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-13 | 41 | 41 | 0 | gsd-security-auditor (L1, block_on high) |
| 2026-10-20 | 67 | 67 | 0 | gsd-security-auditor (L1, block_on high; gap closure 02-11..02-19, earlier threats re-checked) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-10-13; re-verified 2026-10-20 after gap closure
