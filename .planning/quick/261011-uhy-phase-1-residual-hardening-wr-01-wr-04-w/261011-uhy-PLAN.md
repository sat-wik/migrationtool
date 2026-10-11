---
phase: 01-pinned-toolchain-container
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - Cargo.toml
  - Cargo.lock
  - crates/mt-toolchain/Cargo.toml
  - crates/mt-toolchain/src/runner.rs
  - crates/mt-toolchain/tests/tool_01_runner.rs
  - container/Dockerfile
  - .github/workflows/container.yml
  - crates/mt-toolchain/tests/container_static.rs
  - container/README.md
autonomous: true
requirements: [TOOL-01, TOOL-03]

estimate:
  tokens: 240000
  raw_tokens: 240000
  tasks: 3
  confidence: low

must_haves:
  truths:
    - "WR-01: a timed-out run whose descendant called setsid, directly or as a forking subtree, returns in under 4 s with timed_out set and both streams drained. The runner signals through rustix (the child's process group, plus a pidfd for each escaped descendant found by walking /proc ppid links), starts no process other than the requested tool, and never signals a bare pid"
    - "WR-01: Cargo.lock gains no [[package]] entry (rustix 1.1.5 is already locked through tempfile), cargo deny check passes, and every tool crate still has #![forbid(unsafe_code)]"
    - "WR-06: every Dockerfile step that consumes only pre-downloaded, verified files (rustup-init, rustup toolchain install/default, cargo install, the Arm archive extraction, the klee-uclibc and KLEE builds) runs in RUN --network=none, after a separate networked RUN that downloads and checks its inputs. Any step that cannot is labelled 'Remains networked:' with its reason in a Dockerfile comment, and also in container_static.rs if it is an install"
    - "WR-06: container_static.rs refuses an install command in a networked RUN, any RUN --network= value other than none, and a download (curl, git clone, git fetch, apt-get) inside a RUN --network=none. It accepts cargo install evidence (sha256sum -c plus tar -C) from an earlier RUN of the same stage only when the install RUN is offline. No existing refusal is loosened"
    - "WR-07: the image build fails unless each mirror file has exactly one matching record line. Each recorded archive's sha256 and URL must be an xz_hash/xz_url (or hash/url) pair of the pin-verified channel manifest that precedes it in the record. image-a's pin-binding gate re-checks this against in-image copies of both manifests whose sha256 it compares with the pins, and fails on any unclassified record line"
    - "WR-04: image-a holds only contents: read. The new publish job (needs image-a, image-b, compare; only for pushes to main or the phase branch) is the only holder of packages: write. It loads the exact docker-save archive image-a gated: the sha256 comes from image-a's job output and is checked before load, and the image ID must match after load. It never rebuilds. It skips, with a warning, a sha-<commit> tag that already exists in GHCR, and fails closed when the tag lookup errors. It announces verified-image with a digest-only reference after the push"
    - "CI: the final container push run on claude/gsd-discuss-phase-1-mzdw5v has jobs mt, image-a, image-b, compare and publish all success. manifest-a and manifest-b are byte-identical with sha256 40e55cc69fa93f238960bad268cfa4ec5775d51f98dd10efcf92741801d7aed3. The ci workflow is green for that commit, and container/README.md `## Current image` names that run's verified-image digest"
  artifacts:
    - path: "crates/mt-toolchain/src/runner.rs"
      provides: "kill_tree: group STOP, /proc descendant walk with pidfd plus start-time check, KILL; parse_stat helper; documented residual limits"
      contains: "pidfd_send_signal"
    - path: "crates/mt-toolchain/tests/tool_01_runner.rs"
      provides: "setsid, forking-setsid, orphaned-escapee and no-kill-program TOOL-01 tests"
      contains: "tool_01_runner_timeout_kills_a_setsid_descendant"
    - path: "container/Dockerfile"
      provides: "networked download RUNs plus RUN --network=none install/build RUNs; kept channel-manifest copies; in-build archive-vs-manifest check"
      contains: "RUN --network=none"
    - path: ".github/workflows/container.yml"
      provides: "image-a gate on archive lines; image-a saves the gated image; publish job with packages: write"
      contains: "needs: [image-a, image-b, compare]"
    - path: "crates/mt-toolchain/tests/container_static.rs"
      provides: "offline-RUN rules, cross-RUN cargo binding, archive-check and publish-structure TOOL-03 tests"
      contains: "tool_03_dockerfile_installs_only_in_network_none_runs"
    - path: "container/README.md"
      provides: "Current image record of the image published by the publish job"
      contains: "## Current image"
  key_links:
    - from: "crates/mt-toolchain/src/runner.rs"
      to: "rustix::process"
      via: "kill_process_group, pidfd_open and pidfd_send_signal replace the external kill program"
      pattern: "kill_process_group"
    - from: "container/Dockerfile"
      to: ".github/workflows/container.yml"
      via: "/opt/cargo-tools/share/mt/channel-rust-<version>.toml copies, read by the Pin binding evidence step and checked against the pins"
      pattern: "share/mt/channel-rust-"
    - from: ".github/workflows/container.yml image-a"
      to: ".github/workflows/container.yml publish"
      via: "needs.image-a.outputs.archive_sha256 and image_id bind the downloaded image-a-image archive to the gated image"
      pattern: "needs.image-a.outputs.archive_sha256"
    - from: ".github/workflows/container.yml compare"
      to: ".github/workflows/container.yml publish"
      via: "needs.compare.outputs.manifest_sha256 goes into the verified-image notice after the push"
      pattern: "needs.compare.outputs.manifest_sha256"
---

<objective>
Close the four Phase 1 review findings that 01-REVIEW-FIX.md left partly done:
- WR-01: the runner's timeout kill depends on an external kill program and misses descendants that call setsid.
- WR-04: GHCR publishing happens inside image-a, before compare.
- WR-06: install steps run with network access.
- WR-07: the pin-binding gate does not tie archive record lines to their manifests.

Purpose: the TOOL-01 timeout guarantee ("killed and recorded as timed_out") must hold for tools that spawn children. TOOL-03's verified image must be the image that passed every gate, published only after the manifests compare equal (CONTEXT D-24). The install steps must be unable to reach the network, so the pins in container/pins.toml are the only source of the installed bytes (CONTEXT D-04).

Output:
- crates/mt-toolchain/src/runner.rs with an escape-aware tree kill.
- container/Dockerfile with download RUNs split from RUN --network=none install RUNs, and an in-build archive check.
- .github/workflows/container.yml with the archive gate and a separate publish job.
- Updated static tests.
- Two CI round trips on claude/gsd-discuss-phase-1-mzdw5v.
- An updated container/README.md `## Current image`.

Decisions fixed by this plan (each task applies them; do not revisit):
- WR-01 uses rustix 1.1.5 with features std and process. It is already in Cargo.lock at a locked checksum through the tempfile dev-dependency, and it uses the linux_raw backend (no libc) on both deny.toml targets, so it adds no crate.
  - Rejected: fixed candidate paths for the system kill program. They still depend on a binary, and reaching escaped pids would mean signalling bare pids that can be reused.
  - Rejected: PR_SET_CHILD_SUBREAPER. It is process-wide, and it would make the mt process reap arbitrary orphans, racing std's Child::wait in concurrent runs.
- WR-06: the built-in Dockerfile frontend of the pinned BuildKit (pins.toml [ci] buildkit_image, v0.34.0, used through docker buildx) supports RUN --network=none. It rejects unknown RUN flags, so a green build shows the flag was accepted. The Dockerfile keeps no syntax directive.
- WR-06 cargo installs: the networked RUN runs `cargo fetch --locked` against the Cargo.lock packaged in each verified .crate, then the offline RUN runs `cargo install --offline`. c2rust's exporter clones tinycbor through a CMake ExternalProject at build time. The networked RUN pre-clones tinycbor at the pinned commit, and the offline RUN redirects the clone to that local copy with a RUN-scoped git insteadOf.
- WR-07 is checked at build time while the mirror exists. The CI gate then re-checks the same thing against in-image copies of the verified manifests, which it compares with the pins on its own.
- WR-04 carries the gated image as a gzip-compressed docker-save artifact. The sha256 travels through the image-a job output, not the artifact. The image ID is compared after load. Publish jobs for one commit are serialized by a concurrency group.
- CI round trips: two. Round trip 1 after task 2 carries tasks 1 and 2, because the Dockerfile has the highest risk and should be diagnosed on its own. Round trip 2 after task 3 also re-validates the Dockerfile. Each round trip allows at most 3 pushes. If the run is still failing after that, stop and report the failing annotation text verbatim.
- Push rules: work in the main checkout on branch claude/gsd-discuss-phase-1-mzdw5v and push only to origin claude/gsd-discuss-phase-1-mzdw5v. Never force-push, never push to main.
- Manifest invariant: manifest-a and manifest-b must stay byte-identical with sha256 40e55cc69fa93f238960bad268cfa4ec5775d51f98dd10efcf92741801d7aed3. If the sha changes, stop: do not continue to task 3 or the README update, and report the manifest-diff annotation.
</objective>

<execution_context>
@.claude/gsd-core/workflows/execute-plan.md
@.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@.claude/CLAUDE.md
@.planning/phases/01-pinned-toolchain-container/01-REVIEW.md
@.planning/phases/01-pinned-toolchain-container/01-REVIEW-FIX.md
@crates/mt-toolchain/src/runner.rs
@container/Dockerfile
@.github/workflows/container.yml
@deny.toml

Facts observed at planning time (container run 38156036821, image-a job 114519265232):
- Image size 4.14 GB.
- Record order: channel-rust-1.99.0.toml, then its 7 archives (rustc, cargo, rust-std x86_64-gnu, clippy, rustfmt, rust-std thumbv7em-none-eabihf, rust-std x86_64-unknown-linux-musl). Then channel-rust-1.72.1.toml and its 3 archives (rustc, cargo, rust-std). Then the c2rust and cargo-mutants .crate lines.
- Archive lines look like `<sha256>  /tmp/rust-dist/dist/2026-10-01/rustc-1.99.0-x86_64-unknown-linux-gnu.tar.xz`.
- c2rust-ast-exporter 0.22.1's src/CMakeLists.txt sets TINYCBOR_REPO to https://github.com/intel/tinycbor.git and TINYCBOR_TAG to d393c16f3eb30d0c47e6f9d92db62272f0ec4dc7 (= pins.toml [source.tinycbor] commit). Both are CMake cache strings, cloned by ExternalProject_Add.
- The repo is public, so artifact storage is not quota-bound.
- gh api can read actions/runs, runs/<id>/jobs and check-runs/<job-id>/annotations through the proxy. Some actions paths are refused (HTTP 403).
- Locally there is no Docker daemon. dash, mawk, yq and setsid exist.

Large test file: crates/mt-toolchain/tests/container_static.rs (about 1976 lines). Read it by range.
- Rule helpers: lines 250-750 (dockerfile_instructions, run_commands, parse_command, tar_extraction, checked_words, rustup_issues, rustup_init_issues, cargo_install_issues, run_issues, dockerfile_issues).
- Fixture tests: 790-1230.
- Workflow tests: 1747-1976 (workflow_job_text, tool_03_workflow_publishes_by_digest_with_least_privilege, tool_03_dockerfile_asserts_the_rustup_update_hash_comes_from_the_pins, tool_03_workflow_gates_on_pin_binding_evidence).
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: WR-01, kill the child's group and every escaped descendant through rustix, with no external kill program</name>
  <files>Cargo.toml, crates/mt-toolchain/Cargo.toml, Cargo.lock, crates/mt-toolchain/src/runner.rs, crates/mt-toolchain/tests/tool_01_runner.rs</files>
  <read_first>
    - crates/mt-toolchain/src/runner.rs (whole file: kill_group, abort, supervise, run, the unobserved and collect_after_run path)
    - crates/mt-toolchain/tests/tool_01_runner.rs lines 1-45 and 195-225 (helpers config/request/run_ok; tool_01_runner_timeout_kills_descendants_and_reports_timed_out)
    - The rustix 1.1.5 sources in the local cargo registry (src/process/kill.rs, src/process/pidfd.rs, src/pid.rs, src/signal.rs), to confirm the paths of Pid::from_child, Pid::from_raw, kill_process_group, pidfd_open, PidfdFlags, pidfd_send_signal, Signal::STOP and Signal::KILL
  </read_first>
  <behavior>
    - tool_01_runner_timeout_kills_a_setsid_descendant:
      - argv: /bin/sh -c 'setsid sleep 30 & echo $!; sleep 30', timeout 300 ms.
      - Expected: run returns in under 4 s, timed_out true, exit code None, stdout and stderr not truncated.
      - stdout holds the escaped pid. After the run, /proc/<pid>/stat is absent or shows state Z.
    - tool_01_runner_timeout_kills_a_forking_setsid_subtree:
      - argv: /bin/sh -c 'setsid /bin/sh -c "sleep 30 & sleep 30" & sleep 30', timeout 300 ms.
      - Expected: under 4 s, timed_out true, both streams not truncated.
    - tool_01_runner_reports_an_orphaned_escapee_as_unobserved (pins the documented limit):
      - argv: /bin/sh -c '(setsid sleep 12 2>/dev/null &); sleep 30', timeout 300 ms. The escapee is orphaned before the timeout, so no ppid chain reaches it.
      - Expected: run returns Ok in under 9 s with timed_out true, stdout truncated with bytes_total 0 (the unobserved record), stderr not truncated.
    - tool_01_runner_signals_without_spawning_a_kill_program: read crates/mt-toolchain/src/runner.rs through CARGO_MANIFEST_DIR. With full-line comments removed, exactly one `Command::new(` occurrence remains (the requested tool in run).
    - tool_01_proc_stat_parse_reads_fields_after_the_last_paren (unit test inside runner.rs):
      - A stat line whose comm is `(a) b (c)` yields the right ppid, pgrp, session and start_time (field 22).
      - Empty, truncated and non-numeric lines yield None.
    - Existing tests keep passing unchanged, including tool_01_runner_timeout_kills_descendants_and_reports_timed_out and tool_01_run_record_json_is_deterministic. RUN_RECORD_SCHEMA_VERSION and the RunRecord fields stay as they are.
  </behavior>
  <action>
WR-01 under TOOL-01, using the rustix decision fixed in the objective.

1. Dependency.
   - Cargo.toml [workspace.dependencies]: add `rustix = { version = "1.1.5", default-features = false, features = ["std", "process"] }` in alphabetical order. Add a one-line comment: already locked through tempfile, linux_raw backend (no libc), used only by runner.rs to signal the child's group and escaped descendants.
   - crates/mt-toolchain/Cargo.toml [dependencies]: add `rustix.workspace = true`.
   - Do not run cargo add or cargo update. The next build must change Cargo.lock only by adding "rustix" to mt-toolchain's dependency list.

2. runner.rs. Replace the helper that launches the system kill program with `fn kill_tree(child: &Child)`, which starts no process. After the change, runner.rs spawns exactly one process: the requested tool in run.
   kill_tree runs while the child is unreaped, as the current helper already requires. That keeps the child's pid, and so its group id, from being reused.
   a. Freeze the group. Take the group pid from Pid::from_child (pgid == pid because of process_group(0)) and send Signal::STOP to it with kill_process_group, ignoring errors, so group members stop forking.
   b. Find escaped descendants, for up to `ESCAPE_SCAN_ROUNDS` (const, 16) rounds.
      - List the numeric entries of /proc and read each /proc/<pid>/stat.
      - Parse each line with a pure `parse_stat(&str) -> Option<ProcStat>` that reads the fields after the LAST `)`: ppid, pgrp, session, start_time (field 22).
      - Build the ppid-to-children map and collect every descendant of the child pid.
      - For each descendant not yet held:
        - Call pidfd_open(pid, PidfdFlags::empty()).
        - On success, re-read its stat and keep the fd only if start_time equals the scanned value. The fd then names the scanned process, not a reused pid.
        - Send Signal::STOP through pidfd_send_signal and keep the (pid, fd) pair.
      - End early when a round adds nothing. Stopped processes cannot fork, so the set converges.
   c. Send Signal::KILL through every held pidfd, then call kill_process_group(group, Signal::KILL).
   d. Never signal a bare pid. Skip a descendant when pidfd_open fails (ENOSYS, EPERM, ESRCH) or its start time differs. Skip unreadable /proc entries. Ignore all errors (best effort, as today).
   Change supervise and abort to call kill_tree where they called the old helper. Keep their structure and the child.kill() and reap calls as they are.

3. Comments.
   - Rewrite the module doc sentence about the group kill to describe the group-plus-escapee kill.
   - Remove the old note that forbid(unsafe_code) forces an external program.
   - kill_tree's doc lists the residual limits:
     - (i) A descendant whose ancestry was cut before the timeout is not found. Example: a double fork whose middle process already exited, so the descendant was reparented to init or a subreaper. If it holds a pipe, the run is still returned as timed_out with that stream marked truncated, through the existing unobserved path.
     - (ii) Without /proc or pidfd support (kernel below 5.3, or a seccomp filter), only the child's group is signalled.
     - (iii) PR_SET_CHILD_SUBREAPER was rejected, for the reason given in the objective.
     - (iv) Processes of another uid are not signalled.

4. TDD order.
   - Write the behavior tests first.
   - Confirm that the setsid and forking-setsid tests fail against the unchanged runner (each takes about 5 s and reports a truncated stream).
   - Implement, and rerun.
   - Record the observed failure-then-pass in the summary.

Before committing, run cargo fmt --check, cargo clippy --all-targets -- -D warnings, cargo test and cargo deny check, all four green. Then commit with message `fix(01): WR-01 kill escaped descendants on timeout without an external kill program`. Do not push yet; round trip 1 in task 2 carries this commit.
  </action>
  <verify>
    <automated>cargo test -p mt-toolchain --test tool_01_runner && cargo test -p mt-toolchain --lib tool_01_proc_stat && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && cargo deny check && lockdiff=$(git diff Cargo.lock) && test "$(printf '%s\n' "$lockdiff" | grep -c '^+name = ')" -eq 0</automated>
  </verify>
  <done>
- The four new integration tests and the parse unit test pass. The setsid tests were seen failing first, against the unchanged runner.
- runner.rs starts no process other than the requested tool, and signals only through rustix (group calls and verified pidfds).
- Cargo.lock adds no package. All four local gates are green, and the commit exists locally.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: WR-06 and WR-07, offline install RUNs, in-build archive-to-manifest check and gate re-assertion; CI round trip 1</name>
  <files>container/Dockerfile, .github/workflows/container.yml, crates/mt-toolchain/tests/container_static.rs</files>
  <precondition>`git branch --show-current` prints claude/gsd-discuss-phase-1-mzdw5v, the task 1 commit is HEAD, and `gh api "repos/sat-wik/migrationtool/actions/runs?per_page=1"` succeeds.</precondition>
  <read_first>
    - container/Dockerfile (whole file)
    - .github/workflows/container.yml lines 128-170 (ci-lib.sh helpers) and 225-355 (Pin binding evidence step)
    - crates/mt-toolchain/tests/container_static.rs lines 250-750, 790-1230 and 1860-1976
  </read_first>
  <behavior>
    - tool_03_dockerfile_installs_only_in_network_none_runs. Each refused case is refused with a message that names the offending command, not the whole RUN:
      - In a plain RUN: rustup toolchain install, rustup default, rustup target add, rustup component add, rustup-init, cargo install, cargo binstall. A same-RUN sha256sum -c plus tar -C does not excuse a networked install.
      - `RUN --network=host` and `RUN --network=default`.
      - A RUN --network=none containing curl, git clone, git fetch or apt-get.
      - A cargo install whose verified tar evidence sits in an earlier stage (a FROM in between). Message: "not unpacked from a verified .crate".
      - A networked install with evidence in an earlier RUN.
    - The same test's accepted cases:
      - Download/check/unpack in one RUN, then RUN --network=none cargo install of that path in the same stage.
      - The rustup mirror fetch in one RUN, then a RUN --network=none file:// install.
      - rustup-init checked in one RUN and run in a RUN --network=none.
      - Read-only rustup commands in any RUN.
    - The same test against the real Dockerfile:
      - dockerfile_issues is empty, and there are at least 6 `RUN --network=none` instructions.
      - Each RUN containing `rustup-init -y`, `rustup toolchain install`, `cargo install`, `tar -xJf /tmp/arm-gnu-toolchain.tar.xz`, `--make-llvm-lib` or `cmake --build /build/klee` carries --network=none, unless it is listed in NETWORKED_INSTALLS / NETWORKED_STEPS with a non-empty reason that also appears in a Dockerfile comment starting `Remains networked:`.
    - tool_03_dockerfile_fetches_crate_dependencies_before_the_offline_install:
      - The real cargo-tools stage has `cargo fetch --locked --manifest-path /tmp/crates/<name>-<version>/Cargo.toml` per crate in its networked RUN, and `cargo install --locked --offline --path` in its RUN --network=none.
      - cargo fetch without --locked is refused. So is a --manifest-path outside a directory unpacked from a checked archive.
    - tool_03_dockerfile_checks_each_recorded_archive_against_its_manifest:
      - The RUN containing `rustup toolchain install` carries --network=none. Before its first install it contains: a sha256sum -c naming both RUST_TOOL_CHANNEL_SHA256 and RUST_BITCODE_CHANNEL_SHA256, a read of pin-binding.sha256, `xz_hash`, `xz_url`, and `exit 1`.
      - The mirror RUN copies each verified manifest to /opt/cargo-tools/share/mt/channel-rust- after its sha256sum -c.
    - tool_03_workflow_gates_each_recorded_archive_on_the_pinned_manifest. image-a's Pin binding evidence step:
      - reads /opt/cargo-tools/share/mt/channel-rust- copies, compares their sha256sum with the pins, and pairs xz_hash with xz_url;
      - prints an `OK archive` line, has an archive MISMATCH and an `unclassified record line` MISMATCH;
      - keeps every assertion of tool_03_workflow_gates_on_pin_binding_evidence.
    - Every existing static test keeps its assertions. Fixtures that only tripped the new offline rule move to the split shape (download RUN plus RUN --network=none install), so each fixture still isolates the rule it was written for.
  </behavior>
  <action>
WR-06 and WR-07 under TOOL-03 (CONTEXT D-04).

A. container/Dockerfile. Split RUNs per stage as below. Every RUN restates the `: "${NAME:?}"` guards for the values it reads. Every existing check, message, record write, flag and URL guard is kept verbatim.
   - rustup stage, four RUNs.
     - R1 (networked): download rustup-init and run sha256sum -c against RUSTUP_SHA256.
     - R2 (RUN --network=none): run the same sha256sum -c again, so this RUN is bound to the checked file. Then chmod, rustup-init with today's flags (including `--default-toolchain none`), and rm.
     - R3 (networked): today's sv/mirror functions and both mirror calls. Right after each manifest's sha256sum -c, copy it to /opt/cargo-tools/share/mt/channel-rust-${version}.toml: these exact verified bytes stay in the image for the CI gate (WR-07). R3 contains no rustup command.
     - R4 (RUN --network=none). Before any install:
       - For each version:pin pair, run sha256sum -c of the mirror manifest against the pin, and cmp it with the kept copy.
       - WR-07 (i): every regular file under /tmp/rust-dist except *.sha256 sidecars has exactly one record line, equal to its current sha256sum line.
       - WR-07 (ii): walk the record in order, reading it by input redirection (not through a pipe, so `exit 1` ends the RUN).
         - A line naming /tmp/rust-dist/dist/channel-rust-<v>.toml becomes the current manifest.
         - Every other /tmp/rust-dist line needs a current manifest, and its `<hash> https://static.rust-lang.org<path after /tmp/rust-dist>` must be one of the pairs a POSIX, mawk-compatible awk helper prints per [pkg.*] section (xz_hash with xz_url, hash with url) of that manifest.
         - Any other line fails, and each manifest needs at least one archive line.
       - Each failure names the line and the reason on stderr, then runs `exit 1`.
       - After the checks, today's two installs, rustup default, toolchain list, the update-hash loop, rm -rf /tmp/rust-dist and chmod, all unchanged.
   - cargo-tools stage, two RUNs.
     - C1 (networked):
       - Today's download, check, record and unpack loop.
       - Then, from / with RUSTUP_TOOLCHAIN exported (Pitfall 2), `cargo fetch --locked --manifest-path "/tmp/crates/<name>-<version>/Cargo.toml"` for both crates.
       - Then tinycbor: `git clone --no-checkout https://github.com/intel/tinycbor.git /tmp/tinycbor`, `git -C /tmp/tinycbor checkout --detach "${TINYCBOR_COMMIT}"`, `test "$(git -C /tmp/tinycbor rev-parse HEAD)" = "${TINYCBOR_COMMIT}"`, and `git -C /tmp/tinycbor branch mt-pinned "${TINYCBOR_COMMIT}"`.
     - C2 (RUN --network=none):
       - Today's exports, plus GIT_CONFIG_COUNT=1, GIT_CONFIG_KEY_0=url.file:///tmp/tinycbor.insteadOf and GIT_CONFIG_VALUE_0=https://github.com/intel/tinycbor.git. These are scoped to this RUN, like the rustup mirror.
       - Both cargo install commands with --offline added and every other flag unchanged.
       - The existing TINYCBOR_COMMIT grep, plus a grep -qF that the same CMakeLists.txt names https://github.com/intel/tinycbor.git.
       - The llvm-config file, rm -rf /tmp/crates /tmp/tinycbor, chmod.
       - Rewrite the stage comment: downloads happen only in C1, and the build is offline.
   - arm stage:
     - Networked RUN: curl and sha256sum -c.
     - RUN --network=none: the same sha256sum -c again, then mkdir, tar -xJf (flags unchanged), rm, chmod.
   - klee-build stage:
     - Networked RUN: klee-uclibc clone and rev-parse check.
     - RUN --network=none: configure, make, test -f.
     - Networked RUN: KLEE clone and rev-parse check.
     - RUN --network=none: cmake configure (-D flags unchanged), build, install, find check, chmod.
   - Header comment: note the frontend fact from the objective, and keep "no syntax directive".
   - Fallback rule, applied only after CI proves a RUN --network=none step needs the network:
     - Make one fix attempt. For c2rust the allowed second form is GIT_CONFIG_GLOBAL, pointing to a gitconfig written in C1 with the same insteadOf.
     - If it still fails, move only that step back into the networked RUN before it. Put a comment line starting `Remains networked:` with the concrete error and reason directly above that RUN.
     - If the step is an install, add the same reason to NETWORKED_INSTALLS. Otherwise add it to NETWORKED_STEPS.
     - Never drop a check while doing this. cargo-mutants stays offline in every case.

B. .github/workflows/container.yml, Pin binding evidence step (image-a). Keep every existing line of logic and wording: check_toolchain, check_crate, the ledger, the witness wording, `set -eu`, notice versus error, and exit 1.
   - Extract both kept copies into "$out", each with `docker run --rm --network none --user 65534:65534 --read-only mt-toolchain:ci cat /opt/cargo-tools/share/mt/channel-rust-<v>.toml`. A failure is a MISMATCH.
   - Add `check_archives VERSION PIN`:
     - sha256sum of the extracted copy must equal the pin read from image-args.txt.
     - Build the same per-section pairs with the same POSIX awk rule.
     - Walk binding-record.txt in order. Write `OK archive <path> sha256 equals the xz_hash channel-rust-<v>.toml (sha256 equals its pin) gives for <url>`, or a MISMATCH line.
     - A manifest with zero archive lines is a MISMATCH.
   - Add a classification pass: every record line must be a pinned manifest line, an accepted archive line or a pinned crate line. Anything else gives `MISMATCH unclassified record line: <line>`.
   - Update the step comment to say that archive lines are now gated against pin-verified manifest copies.

C. crates/mt-toolchain/tests/container_static.rs (TDD: write the behavior tests first and see them fail on the unchanged Dockerfile and workflow).
   - Add `run_flags(rest)`, which splits the leading `--` tokens of a RUN from its body. Pass the body to run_issues together with an `offline` flag and the stage's accumulated evidence.
   - In dockerfile_issues, keep per-stage tar/checked accumulators, reset at each FROM. Evidence from earlier RUNs counts only for an offline install RUN; a networked RUN keeps today's same-RUN rule.
   - Add the rules from the behavior block:
     - an install command in a networked RUN;
     - a --network= value other than none;
     - a download inside an offline RUN;
     - cargo fetch, which needs --locked and a bound --manifest-path.
   - Add `const NETWORKED_INSTALLS: &[(&str, &str)]` and `const NETWORKED_STEPS: &[(&str, &str)]`, as (marker, reason) pairs, both empty unless the fallback was used.
   - Move accepted fixtures, and refusal fixtures whose `mentions`/`!mentions` assertion would now also match the new rule's message, to the split shape. Use new helpers, an offline fixture and a split two-RUN fixture. Keep every refusal and every message string.
   - Add the four tests from the behavior block.

D. Local evidence. Work in the session scratchpad, never in the repo.
   - Gate dry-run:
     - Extract the Pin binding evidence run script and the ci-lib.sh heredoc with yq.
     - Run the script with bash, with a fake `docker` first on PATH that serves synthetic record, ledger, update-hashes, stored and manifest-copy output, an image-args.txt whose pins equal the synthetic manifests' sha256, and RUNNER_TEMP set to the scratch dir.
     - Cases: everything matching (exit 0, OK archive lines); one record archive hash altered (MISMATCH, exit 1); a manifest copy whose sha256 is not the pin (MISMATCH); an extra unclassified line (MISMATCH).
   - R4 dry-run:
     - Copy R4's WR-07 check portion into a scratch script, with /tmp/rust-dist and the record path substituted by sed.
     - Run it with dash against a synthetic mirror. Cases: pass; a mirror file altered (exit 1); an extra unrecorded file (exit 1).
   - Record every command and its exit code in the summary.

E. Run cargo fmt --check, cargo clippy --all-targets -- -D warnings, cargo test and cargo deny check, all green. Commit with message `fix(01): WR-06 and WR-07 install in RUN --network=none and bind each recorded archive to its pinned manifest`.

F. CI round trip 1. `git push origin claude/gsd-discuss-phase-1-mzdw5v` (no force). Find the container run with event push for the new HEAD sha. Poll its status with a bounded background loop until it completes.
   It passes only if all of the following hold:
   - Conclusion success, and jobs mt, image-a, image-b and compare are success. image-a's old in-job push still runs in this round trip; that is expected.
   - image-a annotations:
     - pin-binding: both manifest-copy pin checks OK; exactly 10 `OK archive` lines (7 for 1.99.0, 3 for 1.72.1, at the current pins); every pre-existing OK line present; no MISMATCH.
     - check-a: exit code 0.
     - negative-pin: exit code 1, with klee and rustc_bitcode MISMATCH.
     - smoke: exit code 0.
     - manifest-a: sha256 40e55cc69fa93f238960bad268cfa4ec5775d51f98dd10efcf92741801d7aed3.
   - image-b manifest-b: the same sha.
   - compare: manifest-compare says byte-identical.
   - The ci workflow push run for the same sha is success.
   On failure, read the "image-a build tail" or "image-b build tail" annotation, apply the fallback rule or fix, then commit and push again (at most 3 pushes in this round trip). Record run ids and the observed lines in the summary.
  </action>
  <verify>
    <automated>cargo test -p mt-toolchain --test container_static && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && cargo deny check && sha=$(git rev-parse HEAD) && gh api "repos/sat-wik/migrationtool/actions/runs?head_sha=$sha&event=push" --jq '[.workflow_runs[] | select(.name=="container" or .name=="ci") | .conclusion] | (length == 2 and all(. == "success"))' | grep -qx true</automated>
  </verify>
  <done>
- Every install and build step that needs no network runs in RUN --network=none, or is documented as remaining networked with its reason.
- The build fails on any archive record line that is not a pair of its pin-verified manifest.
- The gate re-asserts this against the pins.
- The static tests refuse the new escape routes, and every old assertion still holds.
- Round trip 1 is green, with the 10 OK archive lines and manifest sha 40e55cc69fa93f238960bad268cfa4ec5775d51f98dd10efcf92741801d7aed3.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: WR-04, publish from a separate job after compare, using the exact gated image archive; CI round trip 2; README Current image</name>
  <files>.github/workflows/container.yml, crates/mt-toolchain/tests/container_static.rs, container/README.md</files>
  <precondition>Task 2's round trip 1 is green with the manifest sha unchanged, and the branch is still claude/gsd-discuss-phase-1-mzdw5v.</precondition>
  <read_first>
    - .github/workflows/container.yml (whole file)
    - crates/mt-toolchain/tests/container_static.rs lines 1747-1860 and 1899-1976
    - container/README.md lines 190-222
  </read_first>
  <behavior>
    - tool_03_workflow_publishes_by_digest_with_least_privilege (rewritten; keeps every old guarantee):
      - `packages: write` appears exactly once, inside the publish job. image-a's permissions are only `contents: read`, and no other job mentions `packages:`.
      - secrets used = {GITHUB_TOKEN}. docker/login-action appears exactly once, in publish.
      - No `!= 'pull_request'` anywhere. The workflow-level `cancel-in-progress: ${{ github.event_name == 'pull_request' }}` is kept, and `cancel-in-progress: true` is absent.
      - publish has `needs: [image-a, image-b, compare]`, a job-level `if:` equal to the publish guard, and a concurrency group containing `${{ github.sha }}` with `cancel-in-progress: false`. It contains no `buildx build`, `docker build` or `actions/checkout`.
      - The guard string appears exactly 3 times: publish's job-level if, image-a's `id: save` step, and image-a's archive upload step.
      - In image-a, the save step comes after the pin-binding, check-a, negative-pin, manifest-a and smoke steps. image-a contains `docker save` and no `docker push` or `docker/login-action`. image-a's outputs are `image_id` and `archive_sha256` from steps.save.
      - The artifact name image-a-image is in image-a's upload and publish's download, both using the action SHAs already in the workflow.
      - Order inside publish: sha256sum -c of needs.image-a.outputs.archive_sha256 < docker load < the `{{.Id}}` comparison with needs.image-a.outputs.image_id < docker/login-action@ < docker manifest inspect < docker push < annotate notice verified-image.
      - The push and announce steps are gated on `steps.existing.outputs.exists == 'false'`. The existence step reaches `exit 1` for lookup errors other than not-found.
      - publish contains `{{index .RepoDigests 0}}` and the `^ghcr\.io/sat-wik/migrationtool-toolchain@sha256:[0-9a-f]{64}$` check.
      - `annotate notice verified-image` appears exactly once, in publish, and uses needs.compare.outputs.manifest_sha256.
      - compare keeps manifest-diff followed by exit 1, exports manifest_sha256, and no longer mentions IMAGE_REF.
    - tool_03_workflow_gates_on_pin_binding_evidence: the old "gate before login and push in image-a" tail becomes "the gate comes before `id: save` in image-a, and publish needs image-a". All other assertions are unchanged.
    - Local dry-run with a fake docker of publish's check-and-load and tag-existence scripts:
      - Archive sha mismatch: exit 1 before load.
      - Image ID mismatch: exit 1.
      - Tag found: exists=true plus a publish-skipped warning, no push.
      - `manifest unknown`: exists=false.
      - Any other lookup error: exit 1.
  </behavior>
  <action>
WR-04 under TOOL-03 (CONTEXT D-24). Use the publish guard exactly as it is today: `if: ${{ github.event_name == 'push' && (github.ref == 'refs/heads/main' || github.ref == 'refs/heads/claude/gsd-discuss-phase-1-mzdw5v') }}`. Reuse only action SHAs already in the file: upload-artifact cf430e03…, download-artifact 9000827c…, docker/login-action dbcb8138….

1. image-a.
   - Set permissions to `contents: read` only. Delete the outputs.image_ref line, the GHCR login step and the push step.
   - After the smoke step, add a `Save the gated image` step (id: save) carrying the guard and no status function, so it runs only when every gate above succeeded. It must:
     - read `docker image inspect --format '{{.Id}}' mt-toolchain:ci` and require it to match `^sha256:[0-9a-f]{64}$`;
     - run `docker save mt-toolchain:ci | gzip -1 > "$RUNNER_TEMP/image/mt-toolchain.tar.gz"` with pipefail;
     - take its sha256;
     - write `image_id` and `archive_sha256` to GITHUB_OUTPUT;
     - annotate notice `image-archive` with both values.
   - Add an upload-artifact step with the guard: name image-a-image, that path, if-no-files-found error, retention-days 1, compression-level 0.
   - Add job outputs image_id and archive_sha256 from steps.save.outputs.
   - Rewrite the D-24 comment block: the image is carried to publish as a checked archive, never rebuilt.

2. compare.
   - Give the compare step id diff. On success, write `manifest_sha256=<hex>` to GITHUB_OUTPUT and expose it as a job output.
   - Remove the IMAGE_REF env and the verified-image block. Keep the manifest-diff failure path unchanged.

3. New job publish.
   - needs: [image-a, image-b, compare]; job-level guard if; ubuntu-24.04; timeout-minutes 60.
   - permissions contents: read and packages: write.
   - concurrency group `container-publish-${{ github.sha }}`, cancel-in-progress: false.
   - Inline annotate helper, as in compare. No checkout.
   - Steps, in order:
     a. Free runner disk space (same commands as image-a).
     b. download-artifact image-a-image into "$RUNNER_TEMP/image".
     c. `Check and load the gated image`, with ARCHIVE_SHA256 and IMAGE_ID taken from needs.image-a.outputs. It must:
        - validate both formats;
        - run `echo "${ARCHIVE_SHA256}  $RUNNER_TEMP/image/mt-toolchain.tar.gz" | sha256sum -c -`;
        - run `docker load -i` on it;
        - require `docker image inspect --format '{{.Id}}' mt-toolchain:ci` to equal IMAGE_ID.
        Any failure annotates error image-archive and runs exit 1.
     d. docker/login-action (ghcr.io, github.actor, secrets.GITHUB_TOKEN).
     e. `Refuse to overwrite an existing commit tag` (id: existing), running `docker manifest inspect "${IMAGE}:sha-${COMMIT}"`:
        - exit 0: write exists=true and annotate warning publish-skipped, saying the tag already exists and this build was not pushed.
        - stderr matching `no such manifest|manifest unknown|not found`: write exists=false.
        - anything else: annotate error and exit 1 (absence is not proven, so nothing is pushed).
     f. `Push the gated image and read its digest` (id: publish, if steps.existing.outputs.exists == 'false'): today's tag, push, RepoDigests and validation logic moved verbatim, annotating notice image-digest and setting image_ref.
     g. `Announce the verified image` (same if): validate image_ref against the existing regex and manifest_sha256 against `^[0-9a-f]{64}$`, then annotate notice verified-image with the ref and `manifest sha256 <hex>`.

4. Update the header comment's title list (add image-archive and publish-skipped, and say that verified-image now comes from publish) and the Pin binding evidence comment ("fails this job before the image is saved for publishing").

5. Tests (TDD).
   - Rewrite and adjust the two workflow tests per the behavior block first, and see them fail.
   - Implement the workflow, then make the tests pass.
   - Validate the YAML with `yq '.jobs | keys' .github/workflows/container.yml`. It must list compare, image-a, image-b, mt and publish.
   - Do the fake-docker dry-run from the behavior block in the session scratchpad.

6. Run cargo fmt --check, cargo clippy --all-targets -- -D warnings, cargo test and cargo deny check, all green. Commit with message `fix(01): WR-04 publish from a separate job after compare, from the gated image archive`.

7. CI round trip 2. Push to origin claude/gsd-discuss-phase-1-mzdw5v (no force) and poll as in task 2.
   It passes only if all of the following hold:
   - The container push run for HEAD has jobs mt, image-a, image-b, compare and publish all success.
   - image-a has an image-archive notice, every round-trip-1 annotation (pin-binding with 10 OK archive lines, check-a 0, negative-pin 1, smoke 0, manifest-a sha 40e55cc69fa93f238960bad268cfa4ec5775d51f98dd10efcf92741801d7aed3), and no image-digest annotation.
   - publish has image-digest and verified-image notices. The verified-image ref matches `^ghcr\.io/sat-wik/migrationtool-toolchain@sha256:[0-9a-f]{64}$` and its manifest sha256 equals 40e55cc69fa93f238960bad268cfa4ec5775d51f98dd10efcf92741801d7aed3.
   - Any pull_request-event container run for the same sha shows publish as skipped.
   - The ci run for the sha is success.
   At most 3 pushes.
   Then exercise the refusal path once: `gh run rerun <run-id> --job <publish-job-id>`. Expected: publish succeeds with a publish-skipped warning and no new image-digest or verified-image notice. If the proxy refuses the rerun, record "refusal path not exercised in CI; covered by static test and local dry-run" in the summary.

8. container/README.md `## Current image`:
   - Replace the digest with the round-trip-2 verified-image reference.
   - Update the run id and commit.
   - State that the image was published by the publish job, after compare, from the docker-save archive whose sha256 and image ID image-a recorded.
   - Keep the manifest sha256 line and digest-only references.
   Run cargo test (the README tests), commit with message `docs(01): record the image published by the publish job`, and push (no force). README is outside the container trigger paths, so only ci must be green for this commit.
  </action>
  <verify>
    <automated>cargo test -p mt-toolchain --test container_static && yq '.jobs | keys' .github/workflows/container.yml | grep -qx -- '- publish' && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && cargo deny check && sha=$(git rev-parse HEAD) && gh api "repos/sat-wik/migrationtool/actions/runs?head_sha=$sha&event=push" --jq '[.workflow_runs[] | select(.name=="ci") | .conclusion] | (length >= 1 and all(. == "success"))' | grep -qx true</automated>
  </verify>
  <done>
- Nothing reaches GHCR unless image-a, image-b and compare all passed.
- Only publish holds packages: write. It pushes the byte-identical gated image (sha256 and image ID checked), refuses to re-push an existing sha-<commit> tag, and announces verified-image by digest after the push.
- Round trip 2 is green, with manifest sha 40e55cc69fa93f238960bad268cfa4ec5775d51f98dd10efcf92741801d7aed3.
- README `## Current image` names the new verified digest.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| runner → tool subprocess tree | An analyser and its descendants may fork, setsid or daemonize; the runner must end them without touching unrelated processes |
| image build → network | Install and build steps must only consume bytes already checked against pins.toml |
| build record → CI gate | The record is written by the Dockerfile; the gate must measure against independent pins |
| image-a → publish job | The gated image crosses jobs as an artifact; write credentials exist only in publish |
| workflow → GHCR tags | Re-runs and parallel runs must not move a sha-<commit> tag |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-01-36 | Denial of service | runner.rs timeout path | high | mitigate | kill_tree stops the group, walks /proc ppid links to stop setsid escapees through pidfds, then SIGKILLs all; setsid and forking-setsid tests bound the run to under 4 s |
| T-01-37 | Elevation of privilege | runner.rs kill_tree | high | mitigate | Only the unreaped child's group and pidfds whose start_time matched the scanned process are signalled; no bare-pid signal fallback, so a reused pid is never hit |
| T-01-38 | Denial of service | runner.rs orphaned escapee | low | accept | A double-forked orphan with no ppid chain is not found. The run still returns timed_out with the stream marked truncated (pinned by tool_01_runner_reports_an_orphaned_escapee_as_unobserved) |
| T-01-39 | Tampering | Dockerfile install/build RUNs | high | mitigate | RUN --network=none on every step that consumes verified files; static rules refuse a networked install, other --network values and downloads in offline RUNs |
| T-01-40 | Tampering | c2rust tinycbor ExternalProject | medium | mitigate | tinycbor pre-cloned with a `rev-parse HEAD` = pinned commit check; offline redirect via RUN-scoped insteadOf; CMakeLists grep for the commit and the redirected URL |
| T-01-41 | Tampering | pin-binding record archive lines | high | mitigate | In-build check of mirror files vs record vs the pin-verified manifest's xz_hash/xz_url pairs; the gate re-checks against in-image manifest copies whose sha256 must equal the pins, and fails on unclassified lines |
| T-01-42 | Elevation of privilege | container.yml packages: write | high | mitigate | packages: write only in publish, which runs no build and no checkout, and only after image-a, image-b and compare succeed, for pushes to main or the phase branch |
| T-01-43 | Tampering | gated image in transit to publish | high | mitigate | docker-save archive sha256 passed through the image-a job output and checked with sha256sum -c before docker load; image ID compared after load; publish never rebuilds |
| T-01-44 | Repudiation | GHCR sha-<commit> tag | medium | mitigate | docker manifest inspect before push; an existing tag is skipped with a warning; lookup errors fail closed; per-commit publish concurrency group removes the check-then-push race inside this workflow. A tag created outside the workflow blocks publishing (documented) |
| T-01-SC | Tampering | Cargo dependency (rustix) | medium | mitigate | No new package: rustix 1.1.5 is already locked by checksum through tempfile; the task fails if Cargo.lock gains any [[package]]; cargo deny check must pass; no npm/pip installs |
</threat_model>

<verification>
- Local, at each commit: cargo fmt --check, cargo clippy --all-targets -- -D warnings, cargo test and cargo deny check, all green. Cargo.lock adds no package.
- Local dry-runs (task 2 gate and R4 check, task 3 publish scripts) are recorded with their commands and exit codes.
- Round trip 1 (task 2): container push run green, with jobs mt, image-a, image-b and compare. pin-binding shows 10 OK archive lines and no MISMATCH. check-a is 0, negative-pin 1, smoke 0, and manifest-a and manifest-b sha256 are 40e55cc69fa93f238960bad268cfa4ec5775d51f98dd10efcf92741801d7aed3. ci is green.
- Round trip 2 (task 3): container push run green, including publish. verified-image is announced by publish with a digest-only reference and the same manifest sha. PR runs skip publish. ci is green. The rerun refusal path is exercised or explicitly marked as not exercised.
</verification>

<success_criteria>
- WR-01: no external kill program. Escaped setsid descendants are killed through verified pidfds, and the residual limits are written in runner.rs comments and in the summary.
- WR-06: every step that consumes only verified files runs in RUN --network=none, or is documented as remaining networked with its reason. The static rules enforce this.
- WR-07: each archive record line is bound to its pin-verified manifest at build time and again in the CI gate.
- WR-04: a separate publish job after compare, with the only packages: write, publishes the exact gated image. Tags are never overwritten.
- No existing check is weakened. The manifest sha is unchanged. The README names the new verified digest.
</success_criteria>

<output>
Create `.planning/quick/261011-uhy-phase-1-residual-hardening-wr-01-wr-04-w/261011-uhy-SUMMARY.md` with `status: complete` in its frontmatter. It must contain:
- each finding's applied fix;
- the TDD failure-then-pass observations;
- the dry-run commands and exit codes;
- both round trips' run ids, job conclusions and the quoted annotation lines (OK archive count, manifest sha, verified-image ref);
- whether the rerun refusal path was exercised;
- any step that remains networked, with its reason;
- the residual limits:
  - WR-01: orphaned double-fork escapees; no /proc or pidfd; subreaper rejected; other uids.
  - WR-04: tags created outside the workflow; artifact retention of 1 day.
  - WR-06: downloads and apt stay networked by design, and the static rules are a lint, not a sandbox.
  - WR-07: archives are deleted from the image, so the gate checks lines against manifests and relies on the build-time recheck and rustup's own verification for the bytes.

Do not commit the SUMMARY, STATE.md or the PLAN (the orchestrator handles docs commits), and do not touch ROADMAP.md.
</output>
