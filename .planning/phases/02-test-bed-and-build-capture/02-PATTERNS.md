# Phase 2: Test Bed and Build Capture - Pattern Map

**Mapped:** 2026-10-12
**Files analyzed:** 27 new or modified files (some are groups)
**Analogs found:** 24 / 27 (3 are workflow or data files with partial analogs only, see "No Analog Found")

All analog paths below were confirmed git-tracked with `git ls-files` (crates, `.github`, `container`). No gitignored mirror paths are used. `testbed/` and `.mt-work/` do not exist yet.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/mt-capture/Cargo.toml` | config | n/a | `crates/mt-toolchain/Cargo.toml` | exact |
| `Cargo.toml` (workspace `members`, new deps `shlex`, `object`, optional `zip`) | config | n/a | `Cargo.toml:1-28` | exact |
| `crates/mt-capture/src/lib.rs` | config (crate root) | n/a | `crates/mt-toolchain/src/lib.rs` | exact |
| `crates/mt-capture/src/manifest.rs` | model + validator | transform (TOML to typed, validate) | `crates/mt-toolchain/src/pins.rs` | exact |
| `crates/mt-capture/src/treehash.rs` | utility | file-I/O, transform | `crates/mt-toolchain/src/runner.rs` `hex()` plus `fetch.rs` hashing style | partial |
| `crates/mt-capture/src/fetch.rs` | service | request-response via runner (git/curl) | `crates/mt-toolchain/src/fetch.rs` | role-match |
| `crates/mt-capture/src/cc.rs` | utility | transform (JSON parse, canonicalise) | `crates/mt-toolchain/src/manifest.rs` (serde_json, BTreeMap) | partial |
| `crates/mt-capture/src/capture.rs` (Make/CMake drivers, gates) | service | batch via runner | `crates/mt-toolchain/src/check.rs` (`RunnerConfig` + `RunRequest` + `run_one`) | role-match |
| `crates/mt-capture/src/elf.rs` | utility | transform (section list, readelf text) | `crates/mt-toolchain/src/version.rs` (regex over tool output) | role-match |
| `crates/mt-capture/src/replay.rs` | service | batch via runner, file-I/O | `crates/mt-toolchain/src/check.rs` `run_one` | role-match |
| `crates/mt-capture/src/entries.rs` | service | request-response via runner (clang) | `crates/mt-toolchain/src/check.rs` `run_one` | role-match |
| `crates/mt-capture/src/seeds/` | service | file-I/O, transform | none in repo | no analog |
| `crates/mt-capture/src/report.rs` | model | transform (deterministic JSON) | `crates/mt-toolchain/src/manifest.rs` | exact |
| `crates/mt-capture/tests/*.rs` (`bed_*`, `build_*`) | test | n/a | `tool_01_runner.rs`, `pins_and_versions.rs`, `check_and_manifest.rs` | exact |
| `crates/mt-capture/tests/fixtures/*` | fixture | n/a | `crates/mt-toolchain/tests/fixtures/{pins-full,tool-outputs}.toml` | exact |
| `crates/mt-cli/src/main.rs` (`Group::Testbed`) | controller (CLI) | request-response | `crates/mt-cli/src/main.rs` `Toolchain` group | exact |
| `crates/mt-cli/Cargo.toml` (add `mt-capture` dep) | config | n/a | `crates/mt-cli/Cargo.toml` | exact |
| `crates/mt-toolchain/src/runner.rs` (`Deserialize` derives) | model | n/a | same file, lines 138, 149, 169, 191 | exact |
| `crates/mt-toolchain/tests/repo_checks.rs` `ROOT_SKIPS` | test config | file-I/O | same file line 16 | exact |
| `.gitignore` (add `/.mt-work/`) | config | n/a | `.gitignore` | exact |
| `testbed/manifest.toml` | config (data) | n/a | `container/pins.toml` | role-match |
| `testbed/dropped.toml`, `testbed/DROPPED.md`, `testbed/README.md`, `testbed/seeds/**`, `testbed/targets/**` | data/docs | n/a | `container/README.md` for prose; no data analog | partial |
| `.github/workflows/testbed.yml` | CI workflow | event-driven | `.github/workflows/container.yml` | role-match |
| Image extension: `container/Dockerfile` | config | n/a | same file, apt-base stage lines 95-118 | exact |
| Image extension: `container/pins.toml` | config | n/a | same file `[apt]` lines 112-121, `[tool.bear]` lines 133-137 | exact |
| Image extension: `crates/mt-toolchain/src/pins.rs` `REQUIRED_TOOLS` | config | n/a | same file lines 23-35 | exact |
| Image extension: fixtures `pins-full.toml`, `tool-outputs.toml` | fixture | n/a | same files (`[tool.bear]` row, `[bear]` table) | exact |
| Image extension: `tests/container_static.rs`, `check_and_manifest.rs`, `pins_and_versions.rs` | test | n/a | same files (see below) | exact |
| Image extension: `.github/workflows/pin-discovery.yml`, `container/README.md` | CI/docs | n/a | same files | exact |

## Pattern Assignments

### `crates/mt-capture/src/lib.rs` and `Cargo.toml` (crate root, config)

**Analog:** `crates/mt-toolchain/src/lib.rs:1-18`, `crates/mt-toolchain/Cargo.toml`

```rust
#![forbid(unsafe_code)]
//! Pinned-toolchain support for migrationtool.
...
pub mod build_args;
pub mod check;
```

```toml
[package]
name = "mt-toolchain"
description = "Pinned subprocess runner, pins schema and toolchain version check"
edition.workspace = true
version.workspace = true
publish.workspace = true

[lints]
workspace = true

[dependencies]
regex.workspace = true
serde.workspace = true
serde_json.workspace = true
sha2.workspace = true
thiserror.workspace = true
toml.workspace = true

[dev-dependencies]
tempfile.workspace = true
```

Copy: `[lints] workspace = true` (this enforces `unwrap_used`/`expect_used = deny`, root `Cargo.toml:30-33`), `.workspace = true` for shared deps, `mt-toolchain = { path = "../mt-toolchain" }` as in `crates/mt-cli/Cargo.toml`. Add `"crates/mt-capture"` to `[workspace] members` (root `Cargo.toml:3`). New deps (`shlex`, `object`, `zip`) go in `[workspace.dependencies]` with exact versions like the existing entries (`toml = "=1.1.7"` style for pinned ones), and each needs a `cargo deny check` pass. No `rustix` dep: `mt-capture` must not signal processes itself.

---

### `crates/mt-capture/src/manifest.rs` (model + validator, TOML transform)

**Analog:** `crates/mt-toolchain/src/pins.rs`

**Imports and header** (lines 13-21):
```rust
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
pub const PINS_SCHEMA_VERSION: u32 = 1;
```

**Struct and `deny_unknown_fields` pattern** (lines 43-65, 159-197 for the tagged enum used by `source.kind`):
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pins {
    pub schema_version: u32,
    #[serde(default)]
    pub source: BTreeMap<String, SourcePin>,
    pub tool: BTreeMap<String, ToolSpec>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub enum SourcePin { Git { url: String, tag: String, commit: String }, Archive { version: String, url: String, sha256: String }, ... }
```
Use the same internally tagged enum for `[target.<id>.source] kind = "git" | "archive"`. `LlvmStatus` (lines 88-95, `rename_all = "lowercase"`) is the model for `build = "make" | "cmake"` and `form = "direct" | "range" | "wrapped"` enums.

**Error type** (lines 276-305) — copy the four-variant shape exactly (Read, Parse via `#[from] toml::de::Error`, SchemaVersion, Invalid with sorted `issues: Vec<String>`):
```rust
#[derive(Debug, thiserror::Error)]
pub enum PinsError {
    #[error("cannot read pins file {}: {source}", path.display())]
    Read { path: PathBuf, #[source] source: std::io::Error },
    #[error("invalid pins file: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("unsupported pins schema_version {found}; expected {expected}")]
    SchemaVersion { found: u32, expected: u32 },
    #[error("pins validation failed with {} issue(s): {}", issues.len(), issues.join("; "))]
    Invalid { issues: Vec<String> },
}
```

**parse / load / validate** (lines 307-344): `toml::from_str(text)?` then check `schema_version`; `load` maps `read_to_string` into `Read`; `validate()` returns `finish(self.collect_issues())`.

**Issue collection helpers** (lines 586-604) — reuse the idea verbatim (private in `pins.rs`, so copy, do not import; or make them `pub(crate)` only if the planner prefers one small change):
```rust
fn finish(mut issues: Vec<String>) -> Result<(), PinsError> {
    issues.sort(); issues.dedup();
    if issues.is_empty() { Ok(()) } else { Err(PinsError::Invalid { issues }) }
}
fn is_lower_hex(text: &str, len: usize) -> bool {
    text.len() == len && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}
fn check_hex(issues: &mut Vec<String>, field: &str, value: &str, len: usize) { ... "{field}: must be {len} lowercase hex digits" }
```
Issue strings name the offending field (`"tool.{key}: missing; ..."`, lines 357-365). Use `target.<id>.source.commit: ...` the same way. Use `if let Some(..) = ... && cond` let-chains (edition 2024, line 378-381).

Target validation rules are in RESEARCH Pattern 1 (D-20, D-27 `elected` operand-of-`spdx` rule, D-28 `accepted_wrapped`). Note D-20 mentions the `elected` allow-list `MIT, BSD-2-Clause, BSD-3-Clause, Apache-2.0, Zlib, ISC`.

---

### `crates/mt-capture/src/report.rs` (model, deterministic JSON)

**Analog:** `crates/mt-toolchain/src/manifest.rs:1-60` and `RunRecord::to_json` at `runner.rs:211-221`

```rust
pub const MANIFEST_SCHEMA_VERSION: u32 = 1;
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Manifest {
    pub schema_version: u32,
    pub observed: BTreeMap<String, ObservedRow>,
}
```
```rust
pub fn to_json(&self) -> Result<String, RunnerError> {
    let mut text = serde_json::to_string_pretty(self)?;
    text.push('\n');
    Ok(text)
}
```
Copy: `BTreeMap` for every map (objects keyed by relative path), a `schema_version` const, pretty JSON plus a trailing newline, no timestamps. Differences: report types derive `Serialize, Deserialize` (D-26 fixtures round-trip), and embed `mt_toolchain::runner::RunRecord` as-is, so `RunRecord`, `ToolStamp`, `ExitRecord`, `StreamRecord` need `Deserialize` added (below).

---

### `crates/mt-toolchain/src/runner.rs` — add `Deserialize` (modified)

**Lines to change:** 65, 138, 149, 169, 191.
```rust
use serde::Serialize;                                   // line 65 -> use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]      // lines 138, 149, 169, 191 -> add Deserialize
```
No `deny_unknown_fields` there (backwards compatible with schema v2, `RUN_RECORD_SCHEMA_VERSION = 2` at line 69). Do not touch `RunnerConfig`/`RunRequest` (they are inputs, not records). `serde` is already a dependency of `mt-toolchain`.

---

### Runner usage (applies to `fetch.rs`, `capture.rs`, `replay.rs`, `entries.rs`, `elf.rs` objcopy/readelf calls)

**Primary analog:** `crates/mt-toolchain/src/fetch.rs:83-115` (one request, check timeout, exit code, completeness, return stdout hash) and `crates/mt-toolchain/src/check.rs:345-358` (config and request builders).

```rust
let cfg = RunnerConfig {
    timeout,
    ..RunnerConfig::new(pins.image.tool_path.clone(), pins.image.source_date_epoch.clone())
};
let request = |key: &str, argv: Vec<OsString>, env: BTreeMap<String, String>| RunRequest {
    tool_name: key.to_owned(), argv, cwd: cwd.to_path_buf(), extra_env: env, tool_version: None,
};
```
```rust
let output = runner::run(&cfg, &request)?;
let exit = &output.record.exit;
if exit.timed_out { return Err(FetchError::TimedOut(timeout)); }
if exit.code != Some(0) { return Err(FetchError::Failed { exit_code: exit.code }); }
if !output.record.stdout.complete { return Err(FetchError::Incomplete); }
```
Rules to carry over:
- `argv[0]` absolute path from `pins.tool[..].bin` (bear, arm_gcc, clang). Missing `git`/`cmake`/`make` rows arrive with the image extension.
- `extra_env` may NOT contain `LANG`, `PATH`, `SOURCE_DATE_EPOCH`, `TZ` (`BASE_ENV_KEYS`, `runner.rs:75`; `run` returns `RunnerError::EnvConflict`, lines 648-657). Put `HOME`, `TMPDIR`, `GIT_*` in `extra_env`.
- Always check `exit.timed_out` (report "not proven"), `exit.code`, `stdout.complete`/`stderr.complete` (WR-13). Output cap default 16 MiB (`DEFAULT_OUTPUT_CAP`, line 72); set `output_cap: 0` when only the hash matters (fetch.rs:89-93); raise timeouts explicitly (default 120 s, `RunnerConfig::new` line 116).
- Error enums: one `thiserror::Error` enum per module with a `#[error(transparent)] Runner(#[from] RunnerError)` variant plus `TimedOut`, `Incomplete`, `Failed { exit_code }` (fetch.rs:11-35). Mirror this for `mt-capture` modules (or one `CaptureError`), `thiserror` only, no `anyhow`.
- Never `std::process::Command` (clippy `disallowed-types`, `clippy.toml:1-3`; plus a repo test in `repo_checks.rs`). This includes tests building the D-26 host-`cc` project: use `runner::run` with `RunnerConfig::new("/usr/bin:/bin", "1791158400")` as `tool_01_runner.rs:27-39` does.
- `hex()` in `runner.rs:277` is private; sha2 0.11 digests lack `LowerHex`. Copy the 8-line helper into `mt-capture` (or the tests' `sha256_hex`, `tool_01_runner.rs:20-25`).

**Fetch-specific:** `curl_argv` (fetch.rs:50-67) is `pub`; reuse it for the Mbed TLS archive and add `-o <file>` after it. Note it hardcodes `argv[0] = "curl"` and relies on `PATH`.

---

### `crates/mt-capture/src/fetch.rs` (service, git/curl through runner)

**Analog:** `crates/mt-toolchain/src/fetch.rs` (above). Command sequence and env: RESEARCH Pattern 3 (git init, fetch --depth 1, checkout --detach, rev-parse HEAD, then tree hash). Offline tests use a local `file://` repo built through the runner (`git` is on dev hosts and CI), with `GIT_ALLOW_PROTOCOL=file` added only in tests.

---

### `crates/mt-capture/src/treehash.rs` (utility, file-I/O)

**Analog (partial):** none for tree walking in `src/`. Use the walk idiom from `crates/mt-toolchain/tests/repo_checks.rs:33-60` and `62-80` (`file_type()` first, never follow symlinks, `stack` + `read_dir`, sort results):
```rust
let file_type = entry.file_type().unwrap();
if file_type.is_symlink() { continue; }
...
found.sort();
```
Production code replaces `unwrap` with `?` into a typed error (the `unwrap_used` lint is deny outside tests). Algorithm is in RESEARCH Pattern 2 and the appendix gives reference vectors for `bed_01_tree_hash_matches_reference_vectors`.

---

### `crates/mt-capture/src/cc.rs`, `capture.rs`, `elf.rs`, `replay.rs`, `entries.rs`

No in-repo analog parses `compile_commands.json` or ELF; use RESEARCH "Code Examples" (canonicalise sketch, readelf regexes, clang probe). Repo conventions to copy:
- Regex over tool text: `crates/mt-toolchain/src/version.rs` (`version::extract(re, group, text)` used at `pins_and_versions.rs:29-32`); `regex` is a workspace dep.
- Parse errors as typed variants, not `unwrap`; compile `Regex` once and return an error if construction fails (or `LazyLock` with an invariant comment if an allow is justified, see the lint note in root `Cargo.toml:30-33`).
- Failure of one target-configuration is data in the report, not an `Err` (exit 1 convention); invalid inputs are `Err` (exit 2).

---

### `crates/mt-cli/src/main.rs` — `Group::Testbed` (controller, request-response)

**Analog:** same file. Argument parsing only (header comment line 2).

**Imports** (lines 4-13) and **group enum** (lines 22-29):
```rust
use clap::{Parser, Subcommand, ValueEnum};
#[derive(Debug, Subcommand)]
enum Group {
    /// Inspect and verify the pinned toolchain.
    Toolchain { #[command(subcommand)] action: ToolchainAction },
}
```
**Subcommand shape** (lines 49-76): doc comment states the exit-code contract ("Exit 0 ..., 1 ..., 2 ..."), `#[arg(long)] pins: PathBuf`, `#[arg(long, value_enum, default_value_t = ...)]`, `#[arg(long, requires = "capture_outputs")]`. For `--all` vs id use `#[arg(conflicts_with = "all")]`/`required_unless_present`.

**Command function and error wiring** (lines 123-159): `anyhow::Context`, `with_context(|| format!(..))`, validation issues printed one per line as `eprintln!("error: invalid pins: {issue}")`, `Ok(bool)` for pass/fail.

**GOTCHA — dispatch in `main`** (lines 228-260): `let Group::Toolchain { action } = cli.group;` is an irrefutable pattern and stops compiling when a second variant exists. Replace it with `match cli.group { Group::Toolchain { action } => ..., Group::Testbed { action } => ... }` and keep the closing block (lines 253-259):
```rust
match result {
    Ok(code) => ExitCode::from(code),
    Err(err) => { eprintln!("error: {err:#}"); ExitCode::from(2) }
}
```
Result convention `.map(|passed| u8::from(!passed))` (line 237). Add `mt-capture = { path = "../mt-capture" }` to `crates/mt-cli/Cargo.toml` (both `[dependencies]`; tests dir uses `[dev-dependencies]`).

**CLI end-to-end test analog:** `crates/mt-cli/tests/tool_01_check_end_to_end.rs` — builds a hermetic fake container tree with `/bin/sh` scripts, serialises exec with `static EXEC_LOCK: Mutex<()>` (lines 22-26) to avoid "text file busy", and launches the built binary through `runner::run`. Reuse the lock idiom for any test writing then executing scripts.

---

### Test files in `crates/mt-capture/tests/` (naming, layout)

**Analogs:** `tool_01_runner.rs`, `pins_and_versions.rs`, `check_and_manifest.rs`.

**File header** (every test file, e.g. `tool_01_runner.rs:1-6`):
```rust
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! TOOL-01 runner contract: ... Tests drive only `mt_toolchain::runner::run` ...
```
**Fixtures via `include_str!`** (`pins_and_versions.rs:13-14`): `const FULL: &str = include_str!("fixtures/pins-full.toml");` and the recorded-output table `#[derive(Deserialize)] struct Recorded { source, stdout, stderr }` (lines 17-22) loaded with `toml::from_str(OUTPUTS).expect(..)`.
**Mutate-and-assert helpers** (lines 34-76): `mutate(from, to)` asserts the needle exists then `replace`; `assert_fails_naming(text, needle)` checks the validation message names the field. Copy these for `bed_01_manifest_rejects_*` tests over a `fixtures/manifest-small.toml`.
**Test names** carry the requirement ID in snake case: `tool_01_runner_records_argv_env_exit_and_output_hashes` (`tool_01_runner.rs:46`), `tool_03_pins_digests_and_commits_are_well_formed`. Phase 2 names from RESEARCH Validation map: `bed_01_*`, `bed_02_*`, `build_01_*`, `build_02_*`, `build_03_*` (e.g. `build_02_replay_matches_original_objects`).
**Temp trees:** `tempfile::tempdir()` (`tool_01_runner.rs:107`); `tempfile` is already a workspace dev-dep.
**Repo-root helper** (`repo_checks.rs:21-27`) `Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize()` — copy for tests reading `testbed/manifest.toml`, `container/README.md` (`build_02_image_digest_matches_readme`) and `.github/workflows/testbed.yml` (`build_02_workflow_runs_capture_and_replay_without_network`). Static YAML/Dockerfile check style: `container_static.rs` `read_repo_file(..)` + regex (`tool_03_discovery_package_list_matches_apt_pins`, lines 1948-1966).

---

### `crates/mt-toolchain/tests/repo_checks.rs` — `ROOT_SKIPS` (modified)

Line 16:
```rust
const ROOT_SKIPS: &[&str] = &[".git", "target", "research"];
```
Add `".mt-work"` (fetched upstream trees contain `.py`; the walk at lines 45-48 only skips these names directly under the root). A planted-file test already exists (`tool_01_python_walk_flags_a_planted_file`, lines 94-105); add `.mt-work/x.py` to a similar case. Also add `/.mt-work/` to `.gitignore` (currently only `/target/`, `/container/smoke/target/`, `/.claude/worktrees/`, `/.planning/milestone.lock`; `.mt-work` is NOT yet ignored).

---

### `testbed/manifest.toml` (config data)

**Analog:** `container/pins.toml` (schema_version, `[apt]`, `[tool.<key>]` tables; per-row comments citing decision IDs). Schema sketch and example target are in RESEARCH Pattern 1 and the Appendix (40-hex commits and tree hashes for 16 targets). Treat the D-03 founder checkpoint as a gate before committing the final list. `[image] digest` must equal the digest under `## Current image` in `container/README.md` (test `build_02_image_digest_matches_readme`).

---

### `.github/workflows/testbed.yml` (CI workflow, event-driven)

**Analog:** `.github/workflows/container.yml` (and `pin-discovery.yml` for the smaller shape).

**Header and triggers** (container.yml:1-54): leading comment block listing annotation titles, `permissions: contents: read`, `on: push: paths:` + `workflow_dispatch:` + `schedule: - cron: "17 4 * * 1"`, a `concurrency` group per event type. For testbed: paths `testbed/**`, `crates/mt-capture/**`, `.github/workflows/testbed.yml`; weekly cron; `workflow_dispatch`.

**Pinned actions** (copy these exact SHAs, comments included):
```yaml
- uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
  with:
    persist-credentials: false
- uses: actions/upload-artifact@cf430e030ddbb5b0abf93d22962f4752f3646cd9 # v7.0.2
- uses: actions/download-artifact@9000827ccba6bdab643e8b6fd33ac0654aef8333 # v8.0.2
```
**`mt` static build and bundle** (container.yml:59-93): `rustup toolchain install`, `cargo build --release --locked -p mt-cli --target x86_64-unknown-linux-musl`, upload `mt-bundle`; later jobs `download-artifact` then `chmod +x "$RUNNER_TEMP/bundle/mt"` (lines 119-125). The static musl `mt` is mounted into the image, never baked in.

**Annotation helper** (container.yml:131-169, repeated per job because jobs do not share files): `annotate LEVEL TITLE FILE` splits at 3800 bytes, escapes `%`, `\r`, `\n`, prints `::level title=...::text`. Copy verbatim into a helper heredoc `cat > "$RUNNER_TEMP/ci-lib.sh" <<'LIB'` and `source` it per step. Pass/fail steps follow this shape (lines 443-467): `set +e; run ...; rc=$?; set -e`, write `*-notice.txt`, `annotate error` + `exit 1` on failure else `annotate notice`. Steps use `if: ${{ !cancelled() && steps.build.outcome == 'success' }}` so evidence still publishes.

**`docker run` flags** (lines 159-168, adapt per RESEARCH "Three container steps"): `--rm --network none --user 65534:65534 --read-only --tmpfs /tmp:rw,exec,nosuid,size=2g -v "$RUNNER_TEMP/bundle/mt:/usr/local/bin/mt:ro" -v "$pins:/opt/mt/pins.toml:ro" -v "$RUNNER_TEMP/out:/out"`, plus a writable `-v "$RUNNER_TEMP/work:/work"` and `chmod 0777` on writable dirs (line 171-172). `fetch` omits `--network none`; `capture` and `replay` keep it (D-12; a static test must assert this). The image is referenced by the digest from `container/README.md` (not `mt-toolchain:ci`).

**Differences from container.yml:** no publish job, no `packages: write`, no buildx; add `timeout-minutes` on every job (container.yml uses 15-120); results as annotations plus `upload-artifact` of `capture`/`replay` reports with `retention-days: 7` (lines 635-643 style).

---

### Image extension (D-30): `container/Dockerfile`, `container/pins.toml`, `REQUIRED_TOOLS`, fixtures, tests, workflow, README

All files change together; each analog is the sibling entry for `bear`.

1. **`container/Dockerfile:104-119`** (apt-base `apt-get install` list). Add `cmake`, `git`, `curl`, `bzip2` (and keep `make`) inside the existing backslash-continued list, before `xz-utils`; keep alphabetical-ish style as is. `tool_03_dockerfile_installs_every_apt_pin` (container_static.rs:2054) requires every `[apt]` key to appear in the apt-base install command.
2. **`container/pins.toml`**: `[apt]` (lines 112-121) keys are quoted and sorted, e.g. `"bear" = "3.1.1-1"`. Add `"bzip2"`, `"cmake"`, `"curl"`, `"git"`, `"make"` with versions printed by `pin-discovery`, do not hand-type. Add tool rows modelled on lines 133-137:
   ```toml
   [tool.bear]
   bin = "/usr/bin/bear"
   version_args = ["--version"]
   version_regex = 'bear (?P<version>\d+\.\d+\.\d+)'
   expect = "3.1.1"
   ```
   New: `[tool.cmake]` (`cmake version (?P<version>...)`), `[tool.git]` (`git version ...`), `[tool.make]` (`GNU Make ...`); optional `curl`, `bzip2`. Note `tool.make` comment: make is installed today but not pinned.
3. **`crates/mt-toolchain/src/pins.rs:24-35`**: `REQUIRED_TOOLS: [&str; 10]` is a sorted list; add the new keys in sorted position and update the array length (`[&str; N]`). `validate_complete` (lines 353-391) then demands each row, so every fixture/test pins file must gain them.
4. **Fixtures**: `crates/mt-toolchain/tests/fixtures/pins-full.toml` (`[tool.bear]` at line 103, `[apt]` at line 82) and `tool-outputs.toml` (table `[bear]` at line 21 with `source = "captured in CI image-a, run <id>"`, `stdout = """..."""`). New tool-output tables must be recorded from a real CI run (`mt toolchain check --capture-outputs`), as the Phase 1 comment says; until then mark `source` honestly. Also `dpkg-query` rows (e.g. line 86 `bear\t3.1.1-1\tinstalled`) for new apt packages.
5. **Tests to update** (these list the tool set literally):
   - `crates/mt-toolchain/tests/check_and_manifest.rs:45` and `:329` (name lists for `good_observations`/rows) and `:597`;
   - `crates/mt-toolchain/tests/pins_and_versions.rs:546` area and any fixed counts;
   - `crates/mt-toolchain/tests/container_static.rs:207-220` (apt-derived `expect` equality for `("bear","bear")`; add `("git","git")`-style lines using `upstream_version(apt(..))`), `tool_03_discovery_package_list_matches_apt_pins` (:1948) and `tool_03_dockerfile_installs_every_apt_pin` (:2054, whose negative cases at :2078-2086 use `bear` and need no change).
   - `crates/mt-cli/tests/tool_01_check_end_to_end.rs` builds a fake container from `pins-full.toml`: new tools need fake scripts there or the test fails.
6. **`.github/workflows/pin-discovery.yml:90`**: the `packages="..."` line must equal the `[apt]` keys with `${LLVM_MAJOR}` expanded (checked by `tool_03_discovery_package_list_matches_apt_pins`). Append `cmake git curl bzip2 make` (match the pins).
7. **`container/README.md`**: contents list and the `## Current image` section (line 200 onwards; run id, merge commit, archive sha, image id, digest). Replace after the new image publishes from `main` (WR-09: only pushes to `main` publish). The founder-facing `docker run` examples at lines 70-93 stay the template for `testbed/README.md`.

---

## Shared Patterns

### Error handling
**Source:** `crates/mt-toolchain/src/fetch.rs:11-35`, `crates/mt-toolchain/src/pins.rs:276-305`, `runner.rs:234-261`
**Apply to:** every `mt-capture` module. `thiserror::Error` enums with doc comments per variant, `#[from]`/`#[error(transparent)]` for wrapped lower-level errors, `#[source]` on io errors with context fields. `anyhow` only in `mt-cli` (`main.rs:8`, `.with_context`).

### Exit-code contract
**Source:** `crates/mt-cli/src/main.rs:51-56, 228-259`. 0 pass; 1 something failed or was "not proven"; 2 invalid or unreadable inputs. Timeout is "not proven", never success (runner header, `runner.rs:11-13`; `fetch.rs:103-105`).

### Determinism
**Source:** `manifest.rs:1-31`, `runner.rs:211-221`. `BTreeMap` everywhere, `schema_version` const, `to_string_pretty` + `"\n"`, no timestamps outside an optional header block. Add a test like `build_02_report_is_deterministic` (serialise twice, compare bytes; compare to `tool_01_runner.rs` "byte-stable records").

### Validation messages
**Source:** `pins.rs:586-604, 357-365`. Collect all issues, sort, dedup, return one `Invalid { issues }`; each issue starts with the dotted field path.

### Process launching
**Source:** `runner.rs:1-5, 648-690`; `clippy.toml:1-3`. Only `runner.rs` may name `std::process::Command` (the file carries `#![allow(clippy::disallowed_types)]`). The repo check in `repo_checks.rs` (from line 110) scans all crates including new ones and tests, so `mt-capture` code and tests must use `runner::run`.

### CI evidence as annotations
**Source:** `container.yml:131-169, 443-467`. `annotate` helper, titled notices, `error` on failure, `!cancelled()` guards, SHA-pinned actions, `permissions: contents: read`, `persist-credentials: false`.

### Tool-source licensing
**Source:** `docs/guidelines/building-the-tool.md` §4 (via CONTEXT D-21). Bear and QEMU run only as subprocesses in the container; never link or vendor them. New crates (`shlex`, `object`, `zip`) are MIT/Apache; confirm with `cargo deny check`.

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| `crates/mt-capture/src/seeds/` (lexer for C byte arrays, selection, index writer, optional zip) | service | file-I/O, transform | No file-corpus or lexer code exists. Use RESEARCH "Seeds" section and the Validation map tests `build_03_*`; the index uses the same `BTreeMap` + `schema_version` JSON style as `report.rs`. |
| `crates/mt-capture/src/cc.rs`, `elf.rs` core logic (shlex split, `-o` default, `object` section list) | utility | transform | No parser for build artefacts exists; use RESEARCH Pattern 6, 8 and the "Code Examples" sketches. Only the error, regex and test conventions above transfer. |
| `testbed/seeds/**`, `testbed/targets/**`, `testbed/dropped.toml` | data | n/a | No committed corpora or per-target data exist; follow the `container/pins.toml` comment style and RESEARCH layout (D-15 sha256 file names plus index). |

## Metadata

**Analog search scope:** `crates/mt-toolchain/src`, `crates/mt-toolchain/tests`, `crates/mt-cli/src`, `crates/mt-cli/tests`, `.github/workflows`, `container/`, root `Cargo.toml`, `clippy.toml`, `.gitignore`.
**Files scanned:** about 20 read in full or in part (`runner.rs`, `pins.rs`, `fetch.rs`, `lib.rs`, `manifest.rs`, `check.rs` excerpts, `main.rs`, test files, `container.yml`, `pins.toml`, `Dockerfile`, README, `pin-discovery.yml`).
**Pattern extraction date:** 2026-10-12
**Caveats for the planner:**
- `main.rs:230` irrefutable `let Group::Toolchain { action } = cli.group;` must become a `match` when `Testbed` is added.
- `.mt-work/` is not git-ignored yet; `ROOT_SKIPS` and `.gitignore` both need the entry.
- `make` is installed in the image but has no `[apt]` pin or `[tool.make]` row today.
- `RunnerConfig::new(path, sde)` takes the `PATH` string for the child; `pins.image.tool_path` is `/usr/bin:/bin` plus the Arm and LLVM dirs in the real pins, so absolute binaries from `pins.tool[..].bin` are still the safer choice.
- `curl_argv` in `fetch.rs` uses a bare `"curl"` as `argv[0]`; the Phase 2 archive fetch should replace `argv[0]` with the pinned absolute path.
