# Phase 4: UB Detection and Decision Log - Pattern Map

**Mapped:** 2026-11-06
**Files analyzed:** 22 new/modified
**Analogs found:** 20 / 22 (two are partial). All analog paths were confirmed tracked with `git ls-files`.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match |
|---|---|---|---|---|
| `crates/mt-toolchain/src/runner.rs` (WR-13 pipe-holder kill, `RUN_RECORD_SCHEMA_VERSION` 2 to 3) | utility (process supervisor) | event-driven / batch | same file: `kill_tree` (520-558), `run` (695-722) | exact (in place) |
| `crates/mt-toolchain/tests/tool_01_runner.rs` (rewrite escapee test, add `tool_01_runner_kills_a_pipe_holder_after_a_normal_exit`) | test | request-response | same file, lines 309-331 and 377-397 | exact |
| `crates/mt-contract/src/ublog.rs` (strict-TOML log, facts, merge, `decided()`) | model | CRUD / transform | `crates/mt-contract/src/policy.rs` | exact |
| `crates/mt-contract/src/lib.rs` (export, `UB_LOG_SCHEMA_VERSION`, `CONTRACT_VERSION` bump) | config | n/a | same file | exact |
| `crates/mt-contract/tests/ub_0N_*.rs` | test | request-response | `crates/mt-contract/tests/eqv_01_policy.rs`, `eqv_01_committed_policies.rs` | exact |
| `testbed/ub-log/<target>.toml` (16 files) | config | file-I/O | `testbed/policy/<module>.toml` | role-match |
| `testbed/policy/*.toml` (`[[ub.override]]` / `[[ub.error_code]]` cross-check) | config | n/a | same files; `Policy::validate(known_sites)` | exact |
| `crates/mt-ub/src/{lib,facts}.rs` | service | transform / file-I/O | `crates/mt-harness/src/probe.rs`, `layout.rs` | role-match |
| `crates/mt-ub/src/hostflags.rs` | utility | transform | `probe.rs::recompile_arguments`; `mt-capture/src/cc.rs` ALLOWED_OPTIONS (290-294) | role-match |
| `crates/mt-ub/src/host_runtime.c` | template | file-I/O | `crates/mt-harness/src/runtime.c` + `runtime.rs` (`render`, `Placeholders`) | role-match (cannot reuse svc code) |
| `crates/mt-ub/src/report.rs` | utility (parser) | transform | `crates/mt-harness/src/classify.rs`, `frame.rs` (`parse_frame`) | role-match |
| `crates/mt-ub/src/optdiff.rs` | service | batch | `crates/mt-harness/src/replay.rs`, `probe.rs::recompile_arguments`, `mt-vectors/src/compare.rs` | role-match |
| `crates/mt-ub/src/klee.rs` | service | batch / subprocess | `crates/mt-harness/src/driver.rs` (generated harness), `selftest.rs` (runner use) | partial |
| `crates/mt-ub/src/relabel.rs` | service | CRUD | `crates/mt-vectors/src/corpus.rs` (`write_corpus`, `HasUbSource::Phase4`) | exact |
| `crates/mt-ub/fixtures/*.c` and `tests/ub_0N_*.rs` (D-08 self-test) | test fixture | request-response | `crates/mt-harness/src/selftest.rs`, `tests/eqv_04_selftest.rs` | exact |
| `crates/mt-vectors/src/corpus.rs` (`Provenance.ub_checked`, index schema bump) | model | CRUD | same file, 23-67 | exact |
| `crates/mt-cli/src/main.rs` (`Group::Ub { check, sign, run, facts }`) | controller (CLI) | request-response | `Group::Vectors` / `VectorsAction` (lines 47-64) | exact |
| `crates/mt-cli/tests/ub_0N_*_cli.rs` | test | request-response | `crates/mt-cli/tests/eqv_01_policy_check_cli.rs`, `eqv_04_vectors_cli.rs` | exact |
| `.github/workflows/testbed.yml` (`ub` job) | config (CI) | pipeline | `vectors:` job (line 787 on) and its `ci-lib.sh` helpers | exact |
| `container/Dockerfile`, `container/pins.toml`, `container.yml` smoke step | config | n/a | 02-01/02-02 and 03-10 image plans (`apt-base` list, `[apt]` pins) | exact |
| `docs/specs/equivalence-contract.md` (label grammar, source order, ABI notes, version bump) | doc | n/a | same file | exact |
| `crates/mt-harness/src/probe.rs` (optional `optlevel_arguments` sibling) | utility | transform | `recompile_arguments` (258-286) | exact |

## Pattern Assignments

### `crates/mt-toolchain/src/runner.rs` (WR-13) (utility, event-driven)

**Analog:** the same file, `kill_tree` and `run`.

**Hook point** (lines 703-715). Today the drain-grace expiry calls `kill_tree(&child)`, which only walks descendants by ppid. The pipe-holder scan goes beside it:
```rust
let mut tree_kill_sent = false;
if !timed_out && HOLDS_EXITED_CHILD {
    let until = Instant::now() + cfg.drain_grace;
    let stdout_done = stdout_reader.wait(until);
    let stderr_done = stderr_reader.wait(until);
    if !(stdout_done && stderr_done) {
        kill_tree(&child);
        tree_kill_sent = true;
    }
}
```
Record the pipe identities before `spawn_reader` (line 692) from `/proc/self/fd/N`, then scan `/proc/*/fd`.

**pidfd + start-time pattern to copy** (lines 534-546):
```rust
let Ok(fd) = pidfd_open(target, PidfdFlags::empty()) else { continue };
// The fd names the scanned process only if the start time is unchanged now that the fd is held.
if read_stat(pid).map(|now| now.start_time) != Some(scanned.start_time) { continue; }
let _ = pidfd_send_signal(&fd, Signal::STOP);
held.insert(pid, fd);
```
Bounded rounds use `ESCAPE_SCAN_ROUNDS`, with STOP first and KILL last (554-557). Reuse `scan_proc()` and `read_stat`. Add the safe-std prototype from RESEARCH "Code Examples" (`read_link("/proc/self/fd/N")` gives `pipe:[ino]`). Bump `RUN_RECORD_SCHEMA_VERSION` (line 69) and add `exit.pipe_holders_killed`. Update the "Residual limits" doc comment (507-518) and the `run` doc comment (638-647). Constraints: no `unsafe`, no `Command` outside this file, and non-Linux keeps the `#[cfg(not(target_os = "linux"))]` stub.

### `crates/mt-toolchain/tests/tool_01_runner.rs` (test)

**Analog:** lines 309-331 (`tool_01_runner_marks_a_stream_held_by_an_orphaned_escapee_incomplete`) and 377-397 (`..._normal_exit_with_an_orphaned_escapee_is_incomplete_not_an_error`). Two existing tests pin the old limit, not one as RESEARCH says. Rewrite both RED-first to expect `complete: true` and a dead escapee. The second asserts `tree_kill_sent`, so keep that assertion. Add `tool_01_runner_kills_a_pipe_holder_after_a_normal_exit`. Copy the shape:
```rust
let mut cfg = config();
cfg.drain_grace = Duration::from_millis(300);
let output = run_ok(&cfg, &request(&["/bin/sh", "-c", "setsid sleep 6 2>/dev/null & exit 0"]));
assert_eq!(output.record.exit.code, Some(0));
assert!(!output.record.exit.timed_out);
assert!(output.record.exit.tree_kill_sent);
```

---

### `crates/mt-contract/src/ublog.rs` (model, CRUD/transform)

**Analog:** `crates/mt-contract/src/policy.rs`.

**Imports and schema conventions** (lines 8-13, 40-42):
```rust
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use serde::Deserialize;
use crate::{CONTRACT_VERSION, POLICY_SCHEMA_VERSION};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy { pub schema_version: u32, ... }
```
The log needs `Serialize` as well as `Deserialize`, because `Policy` and `Override` are Deserialize-only (policy.rs:41, 118). Use `BTreeMap`/sorted `Vec` for deterministic output (D-30).

**Site key** (lines 136-146): reuse `Site { function, kind, location }` (derive `PartialOrd, Ord`) as the merge key. Do not define a second key type.

**Error type** (lines 148-179):
```rust
#[derive(Debug, thiserror::Error)]
pub enum PolicyError {
    #[error("cannot read policy {}: {source}", path.display())]
    Read { path: PathBuf, #[source] source: std::io::Error },
    #[error("invalid policy: {0}")]
    Parse(toml::de::Error),   // not #[source]: text already in message
    #[error("policy validation failed with {} issue(s): {}", issues.len(), issues.join("; "))]
    Invalid { issues: Vec<String> },
}
```
Mirror it as `UbLogError { Read, Parse, Invalid { issues } }`.

**Parse / load / validate split** (lines 181-200, 234): `parse(&str)` checks shape only. `load(&Path)` checks that the file stem equals the target. `validate()` collects every issue into one sorted error. The `decided()` predicate (D-28: action, rationale, approver and date all non-empty) and the facts-signed check go in `validate`.

**Policy cross-check** (policy.rs 227-234, 422-431): the `Policy::validate(&self, known_sites: &[Site])` hook already exists, and override validation rejects a site not in `known_sites`. The log supplies `known_sites`. Add the reverse check, that a decided non-`return_error` entry has an `[[ub.override]]`, in `mt-contract` or `mt ub check`:
```rust
let logged = known_sites.iter().any(|k| {
    (&k.function, &k.kind, &k.location) == (&o.function, &o.kind, &o.location)
});
```
Policy `Override.kind` and `location` are "free text until Phase 4 constrains it" (policy.rs 122-124). Update those doc comments.

### `crates/mt-contract/tests/ub_0N_*.rs` (test)

**Analog:** `crates/mt-contract/tests/eqv_01_policy.rs`.
```rust
#![allow(clippy::unwrap_used, clippy::expect_used)]
fn repo_root() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap() }
fn verdict(text: &str) -> Result<Policy, String> {
    let policy = Policy::parse(text).map_err(|e| e.to_string())?;
    policy.validate(&[]).map_err(|e| e.to_string())?;
    Ok(policy)
}
```
Copy the BASE-string-plus-variant approach for unknown-key rejection, undecided-entry and merge tests. Mirror `eqv_01_committed_policies.rs` for "every committed `testbed/ub-log/*.toml` loads". Names: `ub_05_...`, `ub_04_...`.

### `testbed/ub-log/<target>.toml` (config)

**Analog:** `testbed/policy/cjson.toml` header:
```toml
# Rules: docs/specs/equivalence-contract.md. Loaded by mt_contract::Policy::load; unknown keys are rejected.
schema_version = 1
```
Entry field names follow RESEARCH Pattern 6 (`function`, `kind`, `location`, `status`, `abi`, `configs`, `witnesses`, `action`, `error_code_proposal`, `rationale`, `approver`, `date`). The policy list currently holds 16 files, and policy set equals manifest ids (`load_all`-style check, policy.rs ~437).

---

### `crates/mt-ub/src/facts.rs` and `hostflags.rs` (service/utility, transform)

**Analog:** `crates/mt-harness/src/probe.rs`.

**Recompile-argument stripping to copy** (lines 258-286):
```rust
pub fn recompile_arguments(compiler: &Path, arguments: &[String], scratch: &Path) -> Vec<OsString> {
    let mut argv: Vec<OsString> = vec![compiler.as_os_str().to_owned()];
    let mut remaining = arguments.iter().skip(1);
    while let Some(argument) = remaining.next() {
        let text = argument.as_str();
        if matches!(text, "-o" | "-MF" | "-MT" | "-MQ") { remaining.next(); }
        else if text.starts_with("-o") || text.starts_with("-M") || text.starts_with("-O") || text.starts_with("-g") {}
        else { has_compile_only |= text == "-c"; argv.push(OsString::from(argument)); }
    }
    for flag in ["-g", "-gdwarf-5", "-O0"] { argv.push(OsString::from(flag)); }
    ...
}
```
- `facts.rs`: swap `-c` for `-dM -E -x c <empty file>` and parse `#define` lines into a `BTreeMap`. Layouts come from `probe::layout_for_entry` and `layout::layout_summary` (layout.rs:221-227, 843), not new code.
- `optdiff.rs`: add a sibling `optlevel_arguments(compiler, arguments, scratch, level)` that drops only `-O*`, `-o` and depfile options, keeps `-g`, and returns whether a shipped `-O` was present (Q6).
- `hostflags.rs`: an exhaustive `match` over the `ALLOWED_OPTIONS` spellings in `crates/mt-capture/src/cc.rs` (about 290-294, `exact("-O3"), exact("-Os")`), so a new spelling fails to compile until classified.
- Runner use pattern: `selftest.rs` imports `mt_toolchain::runner::{self, RunOutput, RunRequest, RunnerConfig}` and `crate::tools::ExecTools`. All subprocesses go through them.

### `crates/mt-ub/src/host_runtime.c` (template, file-I/O)

**Analog:** `crates/mt-harness/src/runtime.c` with `runtime.rs` (`Placeholders`, `render`). Reuse the request-file format (`"MTRQ"`, fields 0-5). Do not reuse its raw ARM `svc` calls or its `mt_heap_arena` (runtime.c:305). Use libc `malloc(len)` exact-size input buffers. Golden driver tests: `tests/eqv_03_driver_golden.rs` with `tests/fixtures/golden/*.driver.c`. Add a host driver golden test the same way.

### `crates/mt-ub/src/report.rs` (utility, transform)

**Analog:** `crates/mt-harness/src/classify.rs` (outcome classification as pure function of run output) and `frame.rs::parse_frame`. Use the regex shape from RESEARCH "Report parsing". Test with captured report-text fixtures in `tests/fixtures`, following `crates/mt-harness/tests/eqv_04_classify.rs`. Never hash raw report text.

### `crates/mt-ub/src/relabel.rs` and `mt-vectors` Provenance (service/model, CRUD)

**Analog:** `crates/mt-vectors/src/corpus.rs`.
```rust
pub const CORPUS_INDEX_SCHEMA_VERSION: u32 = 1;   // bump with ub_checked
pub enum HasUbSource { Unchecked, ObservedCrash, ObservedAbort, OobWrite, Timeout, Phase4 }  // Phase4 already exists
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance { pub seed: String, ... }   // add ub_checked { variants_run, variants_unavailable, results }
```
Write through `write_corpus`, which re-checks provenance against each vector. Label grammar regex `^[a-z][a-z-]*(:[A-Za-z0-9_.-]+)*$` is enforced in `crates/mt-vectors/src/vector.rs:104-125`. Tests: `tests/eqv_02_corpus.rs`.

### `crates/mt-ub/fixtures` and the D-08 self-test (test fixture)

**Analog:** `crates/mt-harness/src/selftest.rs` (header lines 1-10: "executor fixtures, rendered from constants ... not per-target C"; imports 12-26) and `tests/eqv_04_selftest.rs`, `crates/mt-cli/tests/eqv_04_selftest_cli.rs`. Follow its `SelftestRow`/`SelftestReport` shape: one row per fixture, triggering and clean input, expected `source:kind`.

### `crates/mt-cli/src/main.rs` (controller/CLI)

**Analog:** the `Group` enum (lines 47-64):
```rust
#[derive(Debug, Subcommand)]
enum Group {
    Toolchain { #[command(subcommand)] action: ToolchainAction },
    Testbed   { #[command(subcommand)] action: TestbedAction },
    Vectors   { #[command(subcommand)] action: VectorsAction },
}
```
Add `Ub { #[command(subcommand)] action: UbAction }` with `Check { target: Option<String> }`, `Sign { accept_default, approver, target }`, plus `Run` and `Facts`. `anyhow` is used only here. Exit-code doc style follows `ToolchainAction::Check` (0 pass, 1 findings, 2 unusable input). Test with `eqv_01_policy_check_cli.rs` style.

### `.github/workflows/testbed.yml` (`ub` job)

**Analog:** the `vectors:` job (line 787 on):
```yaml
  vectors:
    needs: [mt, testbed]
    if: ${{ !cancelled() && needs.mt.result == 'success' }}
    runs-on: ubuntu-24.04
    timeout-minutes: 45
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with: { persist-credentials: false }
      - uses: actions/download-artifact@9000827ccba6bdab643e8b6fd33ac0654aef8333 # v8.0.2
```
Copy its `ci-lib.sh` helpers (`annotate`, `run_offline`, `image_tool`; hardening flags: `--network none --user 65534:65534 --read-only --cap-drop ALL --pids-limit 1024 --memory 6g --cpus 2 --security-opt no-new-privileges`, lines 241-248). Add a host step setting `vm.mmap_rnd_bits=28` before `docker run` (Pitfall 3). `crates/mt-harness/tests/eqv_04_workflow_static.rs` and `mt-capture/tests/build_02_workflow_static.rs` are the static workflow tests to extend, including "testbed/vectors jobs byte-identical".

### Image plan: `container/Dockerfile`, `container/pins.toml`, `container.yml`

**Analog:** the 02-01/02-02 and 03-10 plans. Add `libclang-rt-16-dev libc6-dev-i386 lib32gcc-12-dev` to the `apt-base` list (Dockerfile comment at lines ~113-118 reserves this). Add `[apt]` pins from `pin-discovery.yml`. Add an in-image i386 ASan/UBSan and x86-64 MSan smoke step in `container.yml`. Static test: `crates/mt-toolchain/tests/container_static.rs`.

## Shared Patterns

### Strict TOML plus `schema_version`
**Source:** `crates/mt-contract/src/policy.rs:40-42`. **Apply to:** `ublog.rs`, `testbed/ub-log/*.toml`, any new config. `#[serde(deny_unknown_fields)]` on every struct, `schema_version: u32` first.

### Errors
**Source:** `policy.rs:148-179` (`thiserror`, `Invalid { issues: Vec<String> }`, sorted, each issue starting with its key). **Apply to:** all `mt-ub` and `mt-contract` library code. `anyhow` only in `mt-cli`.

### Subprocesses through the runner
**Source:** `crates/mt-harness/src/selftest.rs:17` (`mt_toolchain::runner::{self, RunOutput, RunRequest, RunnerConfig}`); `run_driver` in `exec.rs:168-175` for QEMU runs. **Apply to:** clang, KLEE, arm-none-eabi-gcc, llvm-symbolizer. Sanitizer options go in `RunRequest.extra_env` (the runner rejects keys that collide with `BASE_ENV_KEYS`, runner.rs 650-656). `std::process::Command` is clippy-banned outside `runner.rs`.

### Test conventions
**Source:** `eqv_01_policy.rs:1` (`#![allow(clippy::unwrap_used, clippy::expect_used)]`). **Apply to:** all test files. Test names contain the requirement ID in snake case (`ub_01_`..`ub_05_`, `tool_01_`). No network and no live model.

### Drop logs with cause tokens and a floor
**Source:** `crates/mt-capture/src/droplog.rs`, `crates/mt-harness/src/adapter_drops.rs`, `tests/eqv_03_adapter_floor.rs`. **Apply to:** `sanitizer: unavailable` causes (D-06, e.g. `host_layout_mismatch`), KLEE drop causes and the floor of 3 (D-21).

### Determinism
`BTreeMap` and sorted vectors; timestamps only in manifest headers; raw sanitizer/KLEE text is never committed or hashed (pids, addresses). Parsed tuples only.

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| `crates/mt-ub/src/klee.rs` (`.ktest` parser, `.err` parser, completion predicate) | service | batch | No symbolic-execution code exists. Reuse only runner usage and driver-style C generation (`driver.rs`); use RESEARCH Pattern 5 and Q5 (big-endian `.ktest`, no Python `ktest-tool`). |
| `crates/mt-ub/src/report.rs` sanitizer-report grammar | utility | transform | Only structural analogs (`classify.rs`, `frame.rs`). Use RESEARCH Q5 formats and regex. |

## Notes for the Planner

- The `mt-vectors` crate has `compare.rs` for the optdiff comparison, so no new comparator is needed.
- `crates/mt-harness/Cargo.toml` already depends on `mt-capture`, `mt-contract`, `mt-seeds`, `mt-toolchain` and `mt-vectors`. A new `mt-ub` crate should depend on `mt-harness` and `mt-contract`, and the workspace members list must gain it (check root `Cargo.toml`).
- RESEARCH Q4 says one existing runner test pins the WR-13 limit. Two do (see above).
- Tracked-source gate: all analogs above are under `crates/`, `testbed/`, `.github/` and `container/`. Nothing is read from a gitignored mirror.

## Metadata

**Analog search scope:** `crates/*` (mt-contract, mt-toolchain, mt-harness, mt-vectors, mt-cli, mt-capture), `testbed/policy`, `.github/workflows`, `container/`
**Files read:** policy.rs (1-200, 410-440), runner.rs (500-730), probe.rs (240-290), corpus.rs (20-70), main.rs (44-104), selftest.rs (1-26), tool_01_runner.rs (excerpts), eqv_01_policy.rs (1-40), testbed.yml (excerpts)
**Pattern extraction date:** 2026-11-06
