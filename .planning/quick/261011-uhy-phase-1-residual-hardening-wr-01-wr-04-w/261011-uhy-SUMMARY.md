---
phase: 01-pinned-toolchain-container
plan: 01
subsystem: toolchain-container
tags: [runner, rustix, pidfd, dockerfile, buildkit, ghcr, ci]
status: complete
requirements: [TOOL-01, TOOL-03]
quick_id: 261011-uhy
completed: 2026-10-11
plan_head_before: 7a04a8bfd4c0c5aa447909450a34ae9fcd07a9aa
plan_head_after: bcf56e3e73255349461b99fdd2939edf5b864a50
commits: 6
actuals:
  tokens: 23173
  tasks: 3
  commits: 6
verified_image: ghcr.io/sat-wik/migrationtool-toolchain@sha256:acdfe312b21c6af19bc61750cf454f3914f482b3bfb9a2ca42d3e6b5f7758b5c
manifest_sha256: 40e55cc69fa93f238960bad268cfa4ec5775d51f98dd10efcf92741801d7aed3
key-files:
  modified:
    - Cargo.toml
    - Cargo.lock
    - crates/mt-toolchain/Cargo.toml
    - crates/mt-toolchain/src/runner.rs
    - crates/mt-toolchain/tests/tool_01_runner.rs
    - crates/mt-toolchain/tests/container_static.rs
    - container/Dockerfile
    - container/README.md
    - .github/workflows/container.yml
---

# Phase 1 Quick 261011-uhy: Residual hardening WR-01, WR-04, WR-06, WR-07 Summary

The runner's timeout now kills setsid escapees through verified pidfds with no external kill program. The image build installs in `RUN --network=none` and binds every recorded archive to its pin-verified manifest. GHCR publishing moved to a separate `publish` job that pushes the exact gated image after `compare`. All four local gates were green at the last commit, both CI round trips are green, and the manifest sha256 stayed `40e55cc69fa93f238960bad268cfa4ec5775d51f98dd10efcf92741801d7aed3`.

## Fixes applied

**WR-01 (commit b59f890).** `kill_group` (which ran `/bin/kill`) became `kill_tree` in `crates/mt-toolchain/src/runner.rs`. It does these in order:
- SIGSTOP to the child's group through `rustix::process::kill_process_group`.
- Up to 16 `/proc` scans that follow ppid links, open a pidfd per descendant, and keep it only if the start time (field 22) matches the scanned value. Each kept process is SIGSTOPped through the pidfd.
- SIGKILL through every held pidfd, then to the group.

It never signals a bare pid. `parse_stat` reads the fields after the last `)`. `runner.rs` now has exactly one `Command::new(`, the requested tool. rustix 1.1.5 (features `std`, `process`, linux_raw backend) was already locked through tempfile. `Cargo.lock` gained only the `"rustix"` entry in mt-toolchain's dependency list and no `[[package]]`. `cargo deny check` passes.

**WR-06 (commits 821f904, f2f8759).** Every `Dockerfile` step that consumes only verified files now runs in `RUN --network=none`, after a separate networked RUN that downloads and checks its inputs:
- rustup stage: R1 downloads and checks rustup-init. R2 (offline) re-checks it and runs it. R3 (networked) builds the mirror and keeps verified manifest copies. R4 (offline) does the checks and installs.
- arm stage: download and check, then offline re-check and `tar -xJf`.
- cargo-tools stage: the networked RUN downloads the crates, runs `cargo fetch --locked --manifest-path` for both, and pre-clones tinycbor at `TINYCBOR_COMMIT` with a `rev-parse HEAD` check and a `mt-pinned` branch. The offline RUN runs `cargo install --locked --offline` with a RUN-scoped `GIT_CONFIG_*` insteadOf redirect of the tinycbor URL to the local copy, and greps CMakeLists for the commit and the URL.
- klee-build stage: the KLEE clone is networked and its cmake configure, build and install are offline. The klee-uclibc clone is networked and its `configure` is offline.

The static rules in `container_static.rs` now refuse:
- an install command in a networked RUN, with a message naming the command;
- any `--network=` value other than `none`;
- curl, wget, apt-get, apt, `git clone|fetch|pull|ls-remote|submodule` and `cargo fetch|update` in an offline RUN;
- a `cargo fetch` without `--locked`, or whose `--manifest-path` is not under a verified unpack directory.

Tar and checksum evidence from earlier RUNs of the same stage counts only for an offline install. It resets at each `FROM`. No existing refusal was loosened.

**WR-07 (commit 821f904).** R4 fails the build before the first install unless:
- both mirror manifests equal their pins and the kept copies;
- every mirror file other than a `*.sha256` sidecar has exactly one record line, equal to its current `sha256sum` line;
- walking the record in order, every line is one of the two manifests, or an archive that exists and is an `xz_hash`/`xz_url` (or `hash`/`url`) pair of the manifest preceding it, with at least one archive per manifest. Any other line fails.

The image-a "Pin binding evidence" step reads both kept manifest copies out of the image, compares their sha256 with the pins from `image-args.txt`, matches each archive line to a pair, prints `OK archive ...` or `MISMATCH archive ...`, and fails on any `unclassified record line`.

**WR-04 (commits 6221b39, 2f77537).**
- image-a is `contents: read` only. It drops the GHCR login and push.
- A new guarded `Save the gated image` step (`id: save`) runs after every gate. It writes a `docker save | gzip -1` archive and exports `image_id` and `archive_sha256`. It posts an `image-archive` notice and uploads the artifact `image-a-image` (retention 1 day).
- `compare` exports `manifest_sha256` and no longer announces.
- The new `publish` job needs `image-a`, `image-b` and `compare` and carries the publish guard. It holds the only `packages: write`, runs one per commit (concurrency group with `github.sha`, no cancel), and has no checkout or build. It checks the archive `sha256sum -c` against the image-a output, then `docker load`, then compares the loaded `{{.Id}}` with the image-a output, then logs in. It looks up `sha-<commit>` with `docker manifest inspect`: an existing tag gives a `publish-skipped` warning and no push, and any lookup error other than not-found fails closed. It then pushes, reads the `RepoDigests` entry, validates it, and announces `verified-image` with the compare job's manifest sha256.

## TDD failure-then-pass observations

- **Task 1.** Against the unchanged runner, `tool_01_runner_timeout_kills_a_setsid_descendant` failed with "a setsid descendant outlived the timeout: 10.31s". `..._kills_a_forking_setsid_subtree` and `..._signals_without_spawning_a_kill_program` also failed (3 failed, 11 passed). After the change all 14 pass in about 5.3 s, plus 2 `tool_01_proc_stat_parse_*` unit tests. `tool_01_runner_reports_an_orphaned_escapee_as_unobserved` passed on both versions because it pins the documented limit.
- **Task 2.** After adding the new rule, 6 existing static tests failed. The real Dockerfile produced six "RUN installs in a networked RUN ..." messages (rustup-init, two `rustup toolchain install`, `rustup default`, two `cargo install`). The new tests failed against the old Dockerfile and workflow: "the RUN with `rustup toolchain install` must be RUN --network=none", "one RUN --network=none builds them" (0 found), "the pin-binding gate lacks /opt/cargo-tools/share/mt/channel-rust-". After the Dockerfile and workflow changes all 29 container_static tests pass.
- **Task 3.** The rewritten workflow tests failed on "no job publish" and "the save step must have the id save". They passed after the workflow change.

## Local dry-run evidence

Work was done in the session scratchpad, not in the repo.

| What | Command shape | Result |
| --- | --- | --- |
| Gate script (extracted with yq, fake `docker`, synthetic manifests, `bash -eo pipefail`), all matching | `run-gate.sh` good | exit 0, OK archive lines |
| One record archive hash altered | `run-gate.sh` altered-archive | exit 1, `MISMATCH archive line ...` |
| In-image manifest copy not equal to the pin | `run-gate.sh` bad-copy | exit 1, `MISMATCH the in-image copy ... not its pin` |
| Extra archive line | `run-gate.sh` unclassified | exit 1, `MISMATCH archive line` and `unclassified record line` |
| Stray non-archive line | `run-gate.sh` stray-line | exit 1, `unclassified record line: ... /etc/passwd` |
| R4 WR-07 portion (extracted from the Dockerfile, paths substituted, `dash` + mawk), synthetic mirror: pass | `dash run.sh` | exit 0 |
| R4: mirror archive altered, extra unrecorded file, record hash not the manifest's, mirror manifest differs from kept copy, extra non-rust-dist record line, duplicate record line, record line with no file | `dash run.sh` | exit 1 each, with a named reason |
| Publish scripts (fake `docker`): archive sha mismatch | check-and-load script | exit 1, `docker load` not called |
| Image id mismatch | check-and-load script | exit 1 |
| Good archive and id | check-and-load script | exit 0 |
| Tag found | lookup script | `exists=true`, `publish-skipped` warning |
| `no such manifest` | lookup script | `exists=false` |
| Other lookup error (`denied`) | lookup script | exit 1, `tag-lookup` error |
| Announce script: digest-only ref / `:latest` ref | announce script | exit 0 / exit 1 |

## CI round trips

All runs are on `claude/gsd-discuss-phase-1-mzdw5v`. Every push used the plain branch push; nothing was force-pushed and nothing went to main.

**Round trip 1 (2 pushes, within the 3 allowed).**
- 821f904, container push run **38180944033**: failure. `RUN --network=none` was accepted by BuildKit. The failing step was klee-uclibc `make`: `curl: (6) Could not resolve host: www.uclibc.org` while fetching `uClibc-locale-030818.tgz`. See "Steps that remain networked".
- f2f8759, container push run **38181385744**: success. Jobs `mt` 114594499075, `image-b` 114594607026, `image-a` 114594607071, `compare` 114596269162 are all success.
- image-a annotations: pin-binding has 10 `OK archive` lines and no MISMATCH, check-a exit code 0, negative-pin exit code 1 with MISMATCH on klee and rustc_bitcode, smoke exit code 0. manifest-a: `sha256 40e55cc69fa93f238960bad268cfa4ec5775d51f98dd10efcf92741801d7aed3`.
- image-b manifest-b: the same sha256. compare: `manifest-a.json and manifest-b.json are byte-identical; sha256 40e55cc69fa93f238960bad268cfa4ec5775d51f98dd10efcf92741801d7aed3`.
- ci push run 38181385716 is success.
- The old in-job push still ran in this round trip, as the plan expected. It published `ghcr.io/sat-wik/migrationtool-toolchain@sha256:b9835258fdf69e31ff0f534f9c0e1a0cb32940ed8a3216431b1d8bfd9063622e` (tag `sha-f2f8759...`).

**Round trip 2 (2 pushes).**
- 6221b39, container push run **38182203931**: success, including `publish` (job 114598682826). It announced `verified-image` as `ghcr.io/sat-wik/migrationtool-toolchain@sha256:608a8caba61e87a12e5494c7405e5a23cdc9e67c4afae093e86eddd259137b4e` with manifest sha256 `40e55cc6...aed3`. Its ci push run 38182203960 and ci pull_request run 38182208271 **failed**: see the deviation about clippy below.
- 2f77537 (clippy fix), container push run **38182234483**: success. Jobs `mt` 114599381161, `image-a` 114599497014, `image-b` 114599497016, `compare` 114601222050, `publish` 114601245190 are all success.
  - image-a has an `image-archive` notice (image id `sha256:5f7373243d8f187f5b4bd6ccec84f6005a787b5002ab6260077564b8bf576a60`) and no `image-digest` annotation.
  - image-a keeps every round-trip-1 annotation: 10 `OK archive` lines, 0 MISMATCH in pin-binding, check-a exit 0, negative-pin exit 1, smoke exit 0, manifest-a sha256 `40e55cc6...aed3`.
  - publish annotations: `image-archive`: `archive sha256 bd1f2bd0e993294e069dec3ebcbdbbfbf69c101008b53e1b1bfdb9c290c64c69 checked before docker load` and `loaded image id sha256:5f737324...6a60 equals the id image-a gated`.
  - publish annotations: `image-digest` and `verified-image`: **`ghcr.io/sat-wik/migrationtool-toolchain@sha256:acdfe312b21c6af19bc61750cf454f3914f482b3bfb9a2ca42d3e6b5f7758b5c`**, `manifest sha256 40e55cc69fa93f238960bad268cfa4ec5775d51f98dd10efcf92741801d7aed3`.
- ci push run 38182234522 and ci pull_request run 38182236837 are success. The pull_request container run 38182236839 shows `publish` skipped (job 114598460271).
- **Refusal path exercised in CI.** `gh run rerun 38182234483 --job 114601245190` produced attempt 2. The publish job (114602041929) succeeded with a `publish-skipped` warning ("`...:sha-2f77537...` already exists in GHCR; this build was not pushed and the existing tag is left as it is"). The push and announce steps were skipped, and there was no new `image-digest` or `verified-image` notice.
- README commit bcf56e3: ci push run 38184126235 is success. `## Current image` now names run 38182234483, commit 2f77537febe4e529eff05b9ba0beca6def02ecc8 and the digest above.

## Steps that remain networked

- **klee-uclibc `make`.** CI error under `RUN --network=none`: `curl: (6) Could not resolve host: www.uclibc.org`, from `make -C extra/locale locale_headers`. The klee-uclibc Makefile downloads `uClibc-locale-030818.tgz` with no checksum. That download was already unverified before this change, so nothing was weakened. Binding it to a pin needs a new `pins.toml` entry (schema and Rust changes, and a possible manifest change), which is outside this plan. Only that `make` stays networked: the clone is checked against its pinned commit in a networked RUN, and `configure` runs in a separate `RUN --network=none`. The step has a `# Remains networked:` comment above its RUN and an entry in `NETWORKED_STEPS` in `container_static.rs`. `NETWORKED_INSTALLS` stays empty. This is why the fallback fired once and the build still has 6 `RUN --network=none` instructions.
- The `curl`/`git clone`/`cargo fetch`/`apt-get` download steps are networked by design.
- No second fix attempt was needed, so the c2rust `GIT_CONFIG_GLOBAL` second form was not used. c2rust, cargo-mutants, the tinycbor redirect, rustup, the Arm unpack and the KLEE cmake build all ran offline in CI.

## Deviations from Plan

**1. [Rule 3 - Blocking] klee-uclibc make cannot run offline.**
- **Found during:** Task 2 round trip 1, push 1.
- **Fix:** Applied the plan's fallback rule: only `make` moved to a networked RUN, labelled and listed (see above). The static test marker became `cd /src/klee-uclibc && make`, and the `RUN --network=none` count assertion (at least 6) still holds.
- **Commit:** f2f8759.

**2. [Process slip] A commit with a clippy failure was pushed.**
- **Found during:** Task 3. The pre-commit gate chain piped clippy through `tail`, which hid its exit code, so 6221b39 was committed and pushed with two `needless_borrow` errors in `container_static.rs`.
- **Effect:** ci push run 38182203960 and ci pull_request run 38182208271 failed for that commit. The container run for 6221b39 still passed.
- **Fix:** 2f77537 removed the two borrows. The next gates ran with their exit codes read directly (fmt, clippy, test and deny all 0). No force-push or amend was used.
- **Side effect:** the 6221b39 run published its own gated image under `sha-6221b39...` (`ghcr.io/sat-wik/migrationtool-toolchain@sha256:608a8caba61e87a12e5494c7405e5a23cdc9e67c4afae093e86eddd259137b4e`), in addition to the final one. It is not referenced anywhere.

**3. [Plan wording] Commit scope.** The plan gave commit messages with scope `01` (`fix(01): ...`), so all six commits use it instead of `quick-261011-uhy`. Every commit ends with the required trailers.

**4. [Scope-neutral additions] Extra checks beyond the plan text.**
- R4 also requires every archive record line to name an existing mirror file with that exact `sha256sum` line.
- The static rules also refuse wget, apt, `git pull|ls-remote|submodule` and `cargo update` in offline RUNs.

## Residual limits

- **WR-01:** An orphaned double-fork escapee (ancestry cut before the timeout) is not found. The run is still returned as `timed_out` with that stream marked truncated (`bytes_total` 0), as pinned by `tool_01_runner_reports_an_orphaned_escapee_as_unobserved`. Without `/proc` or pidfd support (kernel below 5.3 or a seccomp filter) only the child's group is signalled. `PR_SET_CHILD_SUBREAPER` was rejected because it is process-wide and would race `Child::wait` in concurrent runs. Processes of another uid are not signalled.
- **WR-04:** A `sha-<commit>` tag created outside the workflow blocks publishing for that commit (documented fail-closed behaviour). The image-a-image artifact has 1-day retention, so a publish rerun after a day cannot download it. The image id comparison after `docker load` held on the CI runner's Docker; a different image store could change image id semantics.
- **WR-06:** Downloads and apt stay networked by design. klee-uclibc `make` stays networked with an unpinned locale tarball (above). The static rules are a lint over `RUN` text, not a sandbox: they cannot see what a build tool fetches on its own, as the klee-uclibc case showed, so only CI proves a step is offline.
- **WR-07:** The archives are deleted from the image, so the CI gate checks record lines against the manifests and relies on the build-time recheck (the exact files, before install) and rustup's own verification for the bytes.

## Known Stubs

None.

## Threat Flags

None. The `publish` job and the `image-a-image` artifact are the surfaces the plan's threat model (T-01-42 to T-01-44) already covers.

## Self-Check: PASSED

- Files exist: `crates/mt-toolchain/src/runner.rs`, `container/Dockerfile`, `.github/workflows/container.yml`, `crates/mt-toolchain/tests/container_static.rs`, `crates/mt-toolchain/tests/tool_01_runner.rs` and `container/README.md` were all found.
- Commits b59f890, 821f904, f2f8759, 6221b39, 2f77537 and bcf56e3 are ancestors of HEAD (`git rev-list --count 7a04a8b..HEAD` = 6). No file deletions in the range.
- `Cargo.lock` adds no `[[package]]`. Last local gates: fmt, clippy, test and deny exit 0.
