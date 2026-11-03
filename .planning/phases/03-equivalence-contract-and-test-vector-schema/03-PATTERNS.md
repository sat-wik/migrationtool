# Phase 3 (gap closure): Pattern Map

**Scope:** safe_untar symlink guard only. All paths below are git-tracked.

## File Classification

| File | Role | Data flow | Analog | Match |
|---|---|---|---|---|
| `.github/workflows/testbed.yml` (step "Unpack the captures of the testbed job", lines 1028-1058; `safe_untar` 1039-1051) | config / CI shell | batch | same step; `ci-lib.sh` heredoc (181, 807) | exact |
| `crates/mt-harness/tests/eqv_04_workflow_static.rs` (test at ~1050-1075) | test | transform + subprocess | same file helpers; `crates/mt-harness/tests/eqv_04_globals.rs` 60-86 | exact |
| optional guard script (e.g. `.github/scripts/safe-untar.sh`) | utility | batch | none tracked | no analog |

## Findings

**Current guard (testbed.yml:1039-1051)**
```bash
safe_untar() {
  local archive="$1" dest="$2" listing
  listing="$(tar -tvf "$archive")"
  if printf '%s\n' "$listing" | grep -v -E '^[-dl]'; then ... return 1; fi
  if printf '%s\n' "$listing" | grep -E '^l.* -> (/|(.*/)?\.\.(/|$))'; then ... return 1; fi
  tar -xf "$archive" -C "$dest" --no-same-owner --no-same-permissions
}
```
The second grep rejects any `..` component. The fix must, per `l` member, split `NAME -> TARGET`, refuse if TARGET starts with `/`, else join `dirname(NAME)/TARGET`, normalise lexically (pop on `..`, drop `.` and empty), and refuse if a `..` would pop past the root. Caveat: `tar -tvf` prints `name -> target` space-separated, so names containing ` -> ` are ambiguous. Hard links print `link to`, and the first grep already refuses those because they do not start with `-`, `d` or `l`.

Test-side hazard: the existing assertions need `safe_untar()`, `tar -tvf`, `--no-same-owner`, `--no-same-permissions`, and exactly one line starting with `tar -xf` (lines 1057-1074). Keep these true after the fix. If `safe_untar` moves to a script, update those asserts and the `safe_untar ` call-line checks.

**Workflow-text helpers (eqv_04_workflow_static.rs)**
- `const WORKFLOW: &str = ".github/workflows/testbed.yml";` (l.17) and `const RESTORE_STEP` (used at ~1050, so check its definition).
- `fn repo_root() -> PathBuf` (l.33): `Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()`.
- `fn workflow() -> String` (l.39): reads the file.
- `fn function_body(text: &str, name: &str) -> String` (l.46): lines between `name() {` and a line that is exactly `}` after trim. It is the ready-made extractor for `safe_untar`. The body comes back dedented only by the YAML indent, which is fine for bash.
- `fn step_text(text: &str, name: &str) -> String` (l.61): the step from `- name: X` to the next `- ` at the same indent.
- `fn job_text(text: &str, name: &str) -> String` (l.~80); `fn vectors_job() -> String` (l.109); `fn code_lines(text: &str) -> Vec<String>` (l.119, drops `#` comment lines).
- Imports (l.11-15): `BTreeSet`, `fs`, `Path`, `PathBuf`, `regex::Regex`. The file has `#![allow(clippy::unwrap_used, clippy::expect_used)]` as the globals test does.
- Behavioural plan: `let body = function_body(&step_text(&vectors_job(), RESTORE_STEP), "safe_untar");` then build a script `safe_untar() {\n{body}\n}\n safe_untar "$1" "$2"` and run it with bash on tars built by `tar` in a tempdir. No existing test executes a workflow shell snippet or `ci-lib.sh` (grep of `crates/*/tests/*.rs` for `ci-lib`, `"bash"` found nothing), so this is the first.

**Running external programs through the runner (eqv_04_globals.rs)**
```rust
use mt_toolchain::runner::{self, RunRequest, RunnerConfig};
fn cfg() -> RunnerConfig { RunnerConfig::new("/usr/bin:/bin", "1791158400") }   // l.50-52
let output = runner::run(&cfg(), &RunRequest {                                   // l.70-80
    tool_name: "arm_gcc".to_owned(),
    argv,                                  // Vec<OsString>, program first, never a shell string
    cwd: dir.to_path_buf(),
    extra_env: BTreeMap::new(),            // BTreeMap<String,String>
    tool_version: None,
}).unwrap();
assert_eq!(output.record.exit.code, Some(0), "...{}", String::from_utf8_lossy(&output.stderr));
```
- `RunOutput { record, stdout: Vec<u8>, stderr: Vec<u8> }` (mt-toolchain/src/runner.rs:225). `RunnerConfig` has pub `timeout` (default 120 s) and `output_cap`; the env is a fixed base plus `extra_env`, with PATH taken from the config.
- Run bash as `argv = ["/bin/bash", "-c", script, "bash", archive, dest]` or `["/bin/bash", "<script file>", ...]` with `cwd` the tempdir. Make the tars with `tar -cf` via the runner too. Symlink-bearing tars with `..` or absolute targets need `tar -cf x.tar -C dir members` with symlinks created by `std::os::unix::fs::symlink` (std fs, not Command) or `ln -s` through the runner. A member like `a/b -> ../../../x` can be made as a symlink in the tempdir.
- Skip-if-missing analog: `host_tools()` (eqv_04_globals.rs:26-48) checks `Path::exists`, then `eprintln!("skipping: ... not installed on this host"); return None;`. Use the same for `/bin/bash` and `/usr/bin/tar` (or `/bin/tar`). Alternative: `#[ignore = "..."]` as in eqv_04_exec.rs:118, used only for local-capture tests, so prefer the early-return skip. Other `mt-toolchain/tests/` files (tool_01_runner.rs) are the runner's own tests; read it for request shapes with `/bin/sh` if wanted (not read here).

**Shared shell functions**
- `ci-lib.sh` is written by `cat > "$RUNNER_TEMP/ci-lib.sh" <<'LIB'` at testbed.yml:181 (testbed job) and again at :807 (vectors job), then `source "$RUNNER_TEMP/ci-lib.sh"` in later steps (e.g. 395, 1070). It is a per-job generated file, not a tracked file.
- No tracked script is sourced by any workflow: `git ls-files | grep '\.sh$'` shows only `.claude/**` hooks. `.github` holds only `workflows/{ci,container,pin-discovery,testbed}.yml`; no `scripts/` or `ci/` directory exists. A new guard file would have no analog, so the lowest-risk fix keeps `safe_untar` inline and has the test extract it with `function_body`. If a file is created, the vectors job must check out the repo (verify it does) and the workflow-static asserts above change.
- The heredoc option is awkward: the quoted `'LIB'` delimiter at 807 means `safe_untar` could live in the vectors job's `ci-lib.sh`, but the unpack step at 1028 runs before/independent of sourcing it (first `source` in the vectors job is at 939, so it is already available). Keep inline for minimal churn.

**Naming and temp dirs**
- Test names: `eqv_04_workflow_static_<what>` in snake case, for example `eqv_04_workflow_static_restore_vets_the_archives_before_unpacking`. New: `eqv_04_workflow_static_safe_untar_accepts_in_tree_links_and_refuses_escaping_ones`.
- Temp dirs: `tempfile::tempdir().unwrap()` (mt-capture/tests/bed_01_entries.rs:459; `use tempfile::TempDir;` in bed_01_fetch.rs:23).

**Dependencies**
- `crates/mt-harness/Cargo.toml:24-26`: `[dev-dependencies] regex.workspace = true`, `tempfile.workspace = true`. The runner comes from `mt-toolchain` (already imported in eqv_04_globals.rs, so it is a normal dependency). Root `Cargo.toml:30`: `tempfile = "3.27.0"`. No Cargo.toml or deny.toml change is needed for a bash/tar/tempfile test.

## Test cases to cover
Accepted: `a/b/c.h -> ../x.h`; `a/b/c.h -> ../../x.h`; `run/nanopb/default/src/spm_headers/nanopb/pb.h -> ../../pb.h`; a plain file and a directory.
Refused: `a -> /etc/passwd`; `a/b -> ../../../x`; `a -> ../x` (top-level link escaping); `a -> b/../../x`; a hard link or FIFO member (first grep).
Assert via `output.record.exit.code` (`Some(0)` vs `Some(1)`) and, on refusal, that the destination holds no extracted files.
