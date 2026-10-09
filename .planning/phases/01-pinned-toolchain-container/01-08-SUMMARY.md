---
phase: 01-pinned-toolchain-container
plan: 08
subsystem: infra
tags: [ghcr, github-actions, docker-login-action, image-digest, container-readme, manifest-identical, licence-notices]

requires:
  - phase: 01-pinned-toolchain-container
    provides: "01-02 ci_push_mode executor-pushes; 01-07 container workflow (jobs mt, image-a, image-b, compare), check/off-pin/manifest/smoke proofs and chunked annotations"
provides:
  - "ghcr.io/sat-wik/migrationtool-toolchain@sha256:b8032124716db8f2bfdfea5b693665bb57f37972895f038c2948affb5956e46a: the verified toolchain image, the container reference for later phases"
  - ".github/workflows/container.yml: image-a pushes the already-verified image after its check, off-pin, manifest and smoke steps (GITHUB_TOKEN, SHA-pinned docker/login-action, never on pull_request) and publishes image-digest; compare republishes verified-image only after manifest-diff succeeds"
  - "container/README.md: pull by digest and emulated run on Apple Silicon, local build fallback, pin-bump workflow (D-20, D-21), notices, honest manifest-identical guarantee, Current image record"
  - "Tests: README wording, README sections, digest-only references, least-privilege publishing"
affects: [phase-02, phase-03, phase-06-r1-verdict]

actuals:
  tokens: 6100
  tasks: 2
  commits: 2
plan_head_before: cf444f1a279b5596c89aeaf2fe5ef701e18b9f2b
plan_head_after: 111f3835bda22fac15e984cffc58b4c392f84b42

tech-stack:
  added: []
  patterns:
    - "Only the publishing job holds write access to packages; the publish steps carry no status function so they run only when every proof step passed, and are skipped for pull_request"
    - "Publishing announces twice: image-digest when pushed, verified-image only after the manifest diff succeeded; consumers use the digest, never a tag"
    - "Workflow policy is guarded by static tests (single grant of package write access, only GITHUB_TOKEN, push after login, no rebuild after push), mutation-checked"

key-files:
  created:
    - container/README.md
  modified:
    - .github/workflows/container.yml
    - crates/mt-toolchain/tests/container_static.rs

key-decisions:
  - "The pushed image is the exact image that passed the checks (re-tagged and pushed from the loaded mt-toolchain:ci), and the digest is read back from RepoDigests and validated as a full sha256 before it is published"
  - "Added three tests beyond the one the plan names (README sections, digest-only references, least-privilege publishing) because the plan's mitigations T-01-26 and T-01-27 are only durable if a test enforces them"
  - "Licence names in the README come only from files read in this session (Bear COPYING at tag 3.1.1, QEMU LICENSE at tag v7.2.0); the Arm GNU Toolchain licence files could not be read from here and the README says so instead of naming them"

patterns-established:
  - "Registry presence is checked from a cloud session through the anonymous registry token: the served manifest bytes hash to the recorded digest"

requirements-completed: [TOOL-01, TOOL-03]

coverage:
  - id: D1
    description: "After its check, off-pin check, manifest and smoke steps pass, image-a pushes the verified image to GHCR and publishes its sha256 digest; compare republishes it as verified-image only after the two manifests are identical"
    requirement: TOOL-03
    verification:
      - kind: integration
        ref: "gh api actions/runs/37983108094 (container, head 537ef91): jobs mt, image-a, image-b, compare all success; image-a steps 'Log in to GHCR' and 'Push the verified image and read its digest' success; annotations image-digest (job 113998553882) and verified-image (job 114002156699)"
        status: pass
      - kind: unit
        ref: "crates/mt-toolchain/tests/container_static.rs#tool_03_workflow_publishes_by_digest_with_least_privilege"
        status: pass
    human_judgment: false
  - id: D2
    description: "Only image-a holds write access to packages; GITHUB_TOKEN is the only secret; login and push are skipped for pull_request; the login action is pinned by commit SHA"
    requirement: TOOL-03
    verification:
      - kind: unit
        ref: "crates/mt-toolchain/tests/container_static.rs#tool_03_workflow_publishes_by_digest_with_least_privilege"
        status: pass
      - kind: unit
        ref: "crates/mt-toolchain/tests/container_static.rs#tool_03_workflows_pin_actions_by_commit_sha"
        status: pass
    human_judgment: false
  - id: D3
    description: "container/README.md documents pulling by digest and the emulated run, the local fallback, pin bumps, notices, why Hayroll and Kani are absent, and states manifest-identical builds with no forbidden claim"
    requirement: TOOL-03
    verification:
      - kind: unit
        ref: "crates/mt-toolchain/tests/container_static.rs#tool_03_container_readme_states_manifest_identical_and_makes_no_certification_claim"
        status: pass
      - kind: unit
        ref: "crates/mt-toolchain/tests/container_static.rs#tool_03_container_readme_documents_the_required_sections"
        status: pass
      - kind: unit
        ref: "crates/mt-toolchain/tests/container_static.rs#tool_03_container_readme_references_the_image_by_digest_only"
        status: pass
    human_judgment: false
  - id: D4
    description: "The published digest exists in GHCR: the registry served a manifest whose sha256 equals the digest recorded in the README and this SUMMARY"
    requirement: TOOL-03
    verification:
      - kind: other
        ref: "anonymous GET https://ghcr.io/v2/sat-wik/migrationtool-toolchain/manifests/sha256:b8032124... returned 200; sha256sum of the body is b8032124716db8f2bfdfea5b693665bb57f37972895f038c2948affb5956e46a"
        status: pass
    human_judgment: false
  - id: D5
    description: "The founder's emulated pull and run of the digest on Apple Silicon (docker pull --platform linux/amd64, then mt toolchain check inside it) works as the README describes"
    requirement: TOOL-03
    verification: []
    human_judgment: true
    rationale: "D-24 makes the founder's emulated run the consumer path; it needs the founder's machine, which no cloud session has. The README's in-image build route for a static mt has also not been exercised by CI."

duration: 13min
completed: 2026-10-09
status: complete
---

# Phase 1 Plan 08: Publish the verified image Summary

**The toolchain image that passed the live check, the off-pin check, the manifest and the thumbv7em smoke build is pushed to GHCR by `image-a` alone with the workflow's own token, announced as verified only after the two independent builds' manifests matched, and recorded by digest in a new `container/README.md` that also documents the Apple Silicon pull, the local fallback, pin bumps and licence notices.**

## Container reference

```
ghcr.io/sat-wik/migrationtool-toolchain@sha256:b8032124716db8f2bfdfea5b693665bb57f37972895f038c2948affb5956e46a
```

- Container run: **37983108094**, head `537ef911a8adffac79aa1faf4cec73f5794df829` (the last commit touching any container trigger path)
- Manifest sha256 (a and b identical): `40e55cc69fa93f238960bad268cfa4ec5775d51f98dd10efcf92741801d7aed3`
- Registry check from this session: an anonymous registry token was granted and `GET /v2/sat-wik/migrationtool-toolchain/manifests/sha256:b8032124...` returned 200 with a Docker schema 2 manifest (9 layers, about 1.2 GB compressed); `sha256sum` of the served bytes equals the digest above. The package was therefore readable without login when checked.

## Performance

- **Duration:** 13 min (2026-10-09T19:49:42Z to 2026-10-09T20:03:05Z), of which the container run itself was the main wait
- **Tasks:** 2
- **Files modified:** 3 (1 created: `container/README.md`; 487 insertions, 2 deletions)

## Accomplishments

- **Task 1.** `image-a` now has job-level `contents: read` and `packages: write`, an `image_ref` output, a `Log in to GHCR` step (docker/login-action v4.6.0 pinned to `dbcb8138...`, user `github.actor`, password `secrets.GITHUB_TOKEN`) and a `Push the verified image and read its digest` step. Both skip for `pull_request` and, having no status function, run only after every earlier step succeeded. The image is re-tagged `ghcr.io/sat-wik/migrationtool-toolchain:sha-<commit>` and pushed as it is, the digest is read with `docker inspect --format '{{index .RepoDigests 0}}'`, validated as a full sha256 of that repository, published as notice `image-digest` and exposed as a job output. `compare` publishes notice `verified-image` (reference plus manifest sha256) after `manifest-diff` succeeded.
- **README.** Sections for what the image contains (each tool with its `pins.toml` source; LLVM 16 and bitcode rustc 1.72.1 provisional per D-15), the manifest-identical guarantee, pulling and running on Apple Silicon, the local build fallback, bumping pins (D-20, D-21, no Renovate or Dependabot), notices, and Current image. Hayroll and Kani are explained as not installed (licence unconfirmed R3; deferred to v2.0 with the R2 spike).
- **Task 2.** One CI round trip (bound 3) was green; the digest, run id, commit and manifest sha256 are recorded in the README's Current image section and above.

## CI evidence

Round trip 1 for `537ef91` (only one needed):

| Run | Id | Conclusion |
|-----|----|-----------|
| container | 37983108094 | success |
| ci | 37983108143 | success (job `checks` 113998281375) |

Jobs of the container run: `mt` 113998281639 success, `image-a` 113998553882 success, `image-b` 113998553937 success, `compare` 114002156699 success. The GHCR steps in `image-a` (steps 15 and 16, plus the post-step logout) all concluded success. pin-discovery did not run: none of its trigger paths changed.

Annotations quoted from the run:

```
[image-digest, image-a]
ghcr.io/sat-wik/migrationtool-toolchain@sha256:b8032124716db8f2bfdfea5b693665bb57f37972895f038c2948affb5956e46a
pushed as ghcr.io/sat-wik/migrationtool-toolchain:sha-537ef911a8adffac79aa1faf4cec73f5794df829; announced as verified only after the compare job

[verified-image, compare]
ghcr.io/sat-wik/migrationtool-toolchain@sha256:b8032124716db8f2bfdfea5b693665bb57f37972895f038c2948affb5956e46a
manifest sha256 40e55cc69fa93f238960bad268cfa4ec5775d51f98dd10efcf92741801d7aed3

[manifest-compare, compare]
manifest-a.json and manifest-b.json are byte-identical; sha256 40e55cc69fa93f238960bad268cfa4ec5775d51f98dd10efcf92741801d7aed3
```

The same run's `check-a` annotation ends `result: OK` (all tool rows OK, `hayroll` and `kani` `not_installed`, `rustc_bitcode` `1.72.1 d5c2e9c342b3 LLVM 16.0.5`, `llvm pin: 16 (provisional, decision D-15)`), `negative-pin` shows exit code 1 with MISMATCH on `klee` and `rustc_bitcode` for the LLVM 17 copy, and `smoke` shows exit code 0 as user 65534.

The Task 2 verify command (latest container run on the branch is `success <last commit touching the trigger paths>`) passed with `537ef911a8adffac79aa1faf4cec73f5794df829`. The README commit `111f383` touches no trigger path, so it did not start a container run.

## Task Commits

1. **Task 1:** `537ef91` (feat) - GHCR push and verified-reference steps, `container/README.md`, four README and workflow tests
2. **Task 2:** `111f383` (docs) - the verified digest, run id, commit and manifest sha256 recorded under Current image

**Plan metadata:** committed with this SUMMARY. `commits: 2` is measured with `git rev-list --count cf444f1..HEAD` at SUMMARY write.

## Files Created/Modified

- `.github/workflows/container.yml` - image-a permissions, outputs, login and push steps; compare's verified-image notice; header lists the new annotation titles
- `container/README.md` - the founder guide and the Current image record
- `crates/mt-toolchain/tests/container_static.rs` - `tool_03_container_readme_states_manifest_identical_and_makes_no_certification_claim`, `tool_03_container_readme_documents_the_required_sections`, `tool_03_container_readme_references_the_image_by_digest_only`, `tool_03_workflow_publishes_by_digest_with_least_privilege`

## Decisions Made

See `key-decisions`. No founder decision was needed: the push scope stayed within `ci_push_mode: executor-pushes` (current branch, one plain push, no force), the GHCR push was accepted with the workflow token alone, and no secret was introduced.

## Deviations from Plan

### Auto-added

**1. [Rule 2 - Missing Critical] Static tests for the least-privilege and digest-only mitigations**
- **Found during:** Task 1 (threat register T-01-26, T-01-27)
- **Issue:** The plan names one new test (README wording). The mitigations "`packages: write` only on image-a, GITHUB_TOKEN only, login only off pull_request" and "digest references only" would otherwise be unprotected against later edits.
- **Fix:** Added `tool_03_workflow_publishes_by_digest_with_least_privilege` (single grant, only `secrets.GITHUB_TOKEN`, login SHA-pinned and skipped for pull_request, push after login and itself skipped for pull_request, no rebuild after push, verified-image after the manifest diff) and `tool_03_container_readme_references_the_image_by_digest_only`, plus `tool_03_container_readme_documents_the_required_sections` for the acceptance headings. Mutation check: removing the push step's pull_request guard made the first test fail; the workflow was restored.
- **Files modified:** `crates/mt-toolchain/tests/container_static.rs`
- **Committed in:** `537ef91`

---

**Total deviations:** 1 auto-added (Rule 2). **Impact on plan:** no scope change; tests only.

## Issues Encountered

- **The README's second route to a static `mt` has not been run.** Building `mt` inside the image (`cargo build ... --target x86_64-unknown-linux-musl`) is described in the README as unexercised by CI; the `mt-bundle` artifact route is the one CI produces (retained 7 days).
- **The Arm GNU Toolchain licence files could not be read** from this session (the archive host returns 403 through the proxy), so the README names no per-component licence for it and tells the reader where to look in the image. Bear's licence (GNU GPL version 3, `COPYING` at tag 3.1.1) and QEMU's (GNU GPL version 2 for the emulator as a whole, `LICENSE` at tag v7.2.0) were read from upstream. Debian's own copyright files for the packaged versions were not reachable and are not quoted.
- **Roadmap criterion 4, second half still has no owner.** The 01-07 SUMMARY (coverage D6) expected a runner-driven demonstration of clang, KLEE and c2rust on a C input to land in 01-08, but this plan's tasks do not include it. It remains a verification-time question for `/gsd-verify-work 1`, not something this plan delivered.
- The package visibility is whatever GitHub assigned on first push; it was anonymously readable when checked. If the founder wants it private, that is a package setting; the README tells readers to `docker login ghcr.io` if a pull is refused.

## Authentication Gates

None. The GHCR push used the workflow's `GITHUB_TOKEN` and succeeded on the first attempt, so the permissions checkpoint the plan reserved for a refused push was not needed.

## Known Stubs

None. `container/README.md` and the workflow contain no placeholder text; the Current image section holds the real digest.

## Threat Flags

None beyond the plan's register. T-01-26 (write access only on `image-a`, `GITHUB_TOKEN` only, login only off pull_request, SHA-pinned login action), T-01-27 (digest-only references, verified announcement after the manifest diff) and T-01-28 (manifest-identical wording, tested) are mitigated and covered by tests. T-01-SC: no installs were added.

## User Setup Required

None required to proceed. Optional human check recorded as coverage D5: on the Mac, run the README's `docker pull --platform linux/amd64 ghcr.io/sat-wik/migrationtool-toolchain@sha256:b803...e46a` and the `mt toolchain check` command; expected exit 0 with every row OK or `not_installed`.

## Next Phase Readiness

- Later phases can reference the image by the digest above; a rebuild (any change to the Dockerfile, `pins.toml`, smoke crate, Rust sources or workflow) publishes a new digest that the README then needs to record.
- The LLVM 16 and bitcode rustc 1.72.1 pins remain provisional until the Phase 6 R1 verdict.
- Open items carried over from 01-07: the unpinned build-time apt packages in the image inventory, and the unowned second half of ROADMAP criterion 4 (see Issues Encountered).

## Verification Results

Run on `111f383` before the SUMMARY commit:

- `cargo fmt --check`: pass
- `cargo clippy --all-targets -- -D warnings`: pass
- `cargo test --workspace --locked`: pass (container_static 18 of 18)
- `cargo deny check`: advisories ok, bans ok, licenses ok, sources ok
- Task 2 `gh api` verify: latest container run on the branch is `success 537ef911a8adffac79aa1faf4cec73f5794df829`

## Self-Check: PASSED

`container/README.md` exists with the headings Pulling and running on Apple Silicon, Local build fallback, Bumping pins, Notices and Current image and the strings `manifest-identical`, `mt toolchain hash` and `--platform linux/amd64`; `.github/workflows/container.yml` has `packages: write` once (in `image-a`), `docker/login-action` pinned by SHA, `ghcr.io/sat-wik/migrationtool-toolchain` and the titles `image-digest` and `verified-image`; the README holds `ghcr.io/sat-wik/migrationtool-toolchain@sha256:` followed by 64 hex characters, run id 37983108094 and commit 537ef911a8adffac79aa1faf4cec73f5794df829; commits `537ef91` and `111f383` are ancestors of HEAD.

---
*Phase: 01-pinned-toolchain-container*
*Completed: 2026-10-09*
