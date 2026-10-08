# Phase 1: Pinned Toolchain Container - Pattern Map

**Mapped:** 2026-10-08
**Files analyzed:** 27 (new; no existing files modified except `.planning/PROJECT.md`)
**Analogs found:** 0 in-repo code analogs / 27. The repository is greenfield: `git ls-files` shows only `.claude/`, `.planning/`, `docs/` and `README.md`. No Rust, Dockerfile, TOML or workflow exists. No `.github/` directory. Per instructions, nothing is invented; every file below points to the closest NORMATIVE source (doc section or a verified prototype recorded in `01-RESEARCH.md`).

Source key used below:
- **GUIDE** = `/home/user/migrationtool/docs/guidelines/building-the-tool.md`
- **EMIT** = `/home/user/migrationtool/docs/specs/emitted-rust-rules.md`
- **CONV** = `/home/user/migrationtool/.planning/codebase/CONVENTIONS.md`
- **RES** = `/home/user/migrationtool/.planning/phases/01-pinned-toolchain-container/01-RESEARCH.md`
- All paths from `.planning/` and `docs/` above are git-tracked (checked via `git ls-files`). No gitignored mirror paths are referenced.

## File Classification

| New/Modified File | Role | Data Flow | Closest Source (normative, not code) | Match Quality |
|---|---|---|---|---|
| `Cargo.toml` (workspace root) | config | n/a | GUIDE s3 layout; RES Architecture "Recommended Project Structure" | spec-only |
| `rust-toolchain.toml` | config | n/a | GUIDE s4 Toolchains; RES Example 6 `[rust.tool]` | spec-only |
| `Cargo.lock` | config | n/a | RES Standard Stack (committed, `--locked`) | spec-only |
| `deny.toml` | config | n/a | RES Example 5 (verified vs cargo-deny 0.20.2) | prototype |
| `clippy.toml` | config | n/a | RES Example 5, Pitfall 12 (verified) | prototype |
| `.gitignore` | config | n/a | RES Wave 0 (`target/`) | spec-only |
| `research/.gitkeep` | config | n/a | GUIDE s3 (`research/` only place for Python) | spec-only |
| `crates/mt-toolchain/Cargo.toml` | config | n/a | GUIDE s4 (thiserror, forbid unsafe); RES Supporting crates | spec-only |
| `crates/mt-toolchain/src/lib.rs` | utility | n/a | CONV Tool code; GUIDE s4 Safety | spec-only |
| `crates/mt-toolchain/src/runner.rs` | service | request-response + streaming (pipe drain) | RES Example 1 (verified prototype, 4 tests pass) | prototype |
| `crates/mt-toolchain/src/pins.rs` | model | file-I/O (TOML parse) | RES Example 6 `pins.toml` sketch; Security V5 (`deny_unknown_fields`) | prototype shape |
| `crates/mt-toolchain/src/version.rs` | utility | transform (regex parse) | RES "Version commands and parse targets" table + Pitfall 5, 8 | spec-only |
| `crates/mt-toolchain/src/check.rs` | service | request-response (runner -> parse -> compare) | RES Pattern 3 | spec-only |
| `crates/mt-toolchain/src/manifest.rs` | service/model | transform (serialize) | GUIDE s4 Determinism; RES Pattern 1 Record | spec-only |
| `crates/mt-toolchain/tests/fixtures/<tool>.txt` | test fixture | n/a | RES version table "Policy for [ASSUMED] rows" | spec-only |
| `crates/mt-toolchain/tests/*.rs` (tool_01..tool_04 tests) | test | file-I/O / unit | RES Validation Architecture test map; Example 2, 3 | prototype |
| `crates/mt-cli/Cargo.toml`, `src/main.rs` | controller (CLI) | request-response | GUIDE s3 (`mt-cli` args only, anyhow only here); RES Architecture | spec-only |
| `container/pins.toml` | config | n/a | RES Example 6 | prototype |
| `container/Dockerfile` | config | batch (image build) | RES Dockerfile skeleton (ASSUMED, unbuilt) | prototype (unverified) |
| `container/.dockerignore`, `container/README.md` | config/doc | n/a | RES Sizing Plan 04 | spec-only |
| `container/smoke/Cargo.toml`, `src/lib.rs`, `src/ffi.rs` | library (no_std) | transform | RES Example 4 (verified); EMIT s1/s3/s4 | prototype + EMIT |
| `container/smoke/smoke.c` | C stub | n/a | RES Example 4 note (arm-none-eabi-gcc flags) | spec-only |
| `.github/workflows/ci.yml` | config (CI) | batch | RES CI Design; CONV pre-merge list | spec-only |
| `.github/workflows/container.yml` | config (CI) | batch | RES CI Design + pinned action SHAs table | spec-only |
| `.planning/PROJECT.md` (modified: add D-15) | doc | n/a | existing decision lines 136-151 (format); RES Example 3 | exact (format) |

## Pattern Assignments

### `crates/mt-toolchain/src/runner.rs` (service, request-response + pipe streaming)

**Analog:** none in repo. Copy from RES Example 1 (lines 356-395 of RES), verified: compiled, 4 tests pass, clippy `-D warnings` clean on Rust 1.99.0.

**Rules to apply (GUIDE s4 lines 68-74, CONV):** `#![forbid(unsafe_code)]` at crate level; `thiserror` error type (no `anyhow`); argv as `Vec<OsString>`, never a shell string; record argv, env, cwd, tool version, exit status, sha256 of both streams.

**Single-exception pattern (RES Pitfall 12):** only this module may use `std::process::Command`:
```rust
// crates/mt-toolchain/src/runner.rs  (the ONLY file allowed to use std::process)
#![allow(clippy::disallowed_types)]
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};
use std::sync::mpsc; use std::thread; use std::time::{Duration, Instant};
use sha2::{Digest, Sha256};
```
Caveat: `#![allow(...)]` as an inner attribute in a non-root module file works only at the top of the module file; keep it first. `forbid(unsafe_code)` stays on the crate root (`lib.rs`).

**Hex helper (sha2 0.11 digests lack `LowerHex`, RES Pitfall 6):**
```rust
fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes { let _ = write!(s, "{b:02x}"); }
    s
}
```
**Stream drain (hash every byte, store first `cap` bytes; D-18 cap 16 MiB):** copy `drain` from RES lines 374-388 verbatim.

**Spawn recipe (RES lines 390-393):** `cmd.env_clear().envs(&env).current_dir(&cwd).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped())`; one reader thread per pipe via `mpsc`; poll `try_wait`; on timeout `child.kill()`; `recv_timeout(5s)` per stream afterwards (Pitfall 15); record `exit.code()`, `exit.signal()`, `timed_out`.

**Env policy (D-17):** base = `PATH`, `LANG=C.UTF-8`, `TZ=UTC`, `SOURCE_DATE_EPOCH` (value from `pins.toml [image].source_date_epoch`), then named extras. Env recorded as `BTreeMap<String,String>`. For rustup shims pass `RUSTUP_HOME`/`CARGO_HOME` as named extras, or use absolute toolchain paths (Pitfall 9).

**Record schema (RES Pattern 1):** `{schema_version, argv, cwd, env (BTreeMap), tool{name,path,version?}, exit{code,signal,timed_out}, stdout{sha256,bytes_total,bytes_stored,truncated}, stderr{...}}`. Output bytes returned beside, not inside, the JSON.

**Tests to port (rename per convention):** the four prototype tests listed at RES line 395 become `tool_01_runner_records_argv_env_exit_and_output_hashes`, `tool_01_runner_clears_environment_and_applies_fixed_base`, `tool_01_runner_caps_output_without_deadlock`, `tool_01_runner_timeout_is_reported_not_passed`. Env test must NOT use `set_var` (edition 2024, Pitfall 16): assert cargo-set vars such as `CARGO_MANIFEST_DIR` are absent from child `env` output. Tests may `unwrap` (CONV).

---

### `crates/mt-toolchain/src/pins.rs` (model, file-I/O TOML)

**Analog:** none. Shape from RES Example 6 (lines 492-544). Parse with `toml` + `serde` and `#[serde(deny_unknown_fields)]` (RES Security V5). Use `BTreeMap` for the `[tool.*]` and `[apt]` tables (GUIDE s4 Determinism).

Key sections to model: `schema_version`, `[image]` (base, base_digest, snapshot_timestamp, source_date_epoch, tool_path), `[llvm]` (major, max_major, status, decision), `[rustup]`, `[rust.tool]`, `[rust.bitcode]` (version, commit, llvm, channel_manifest_sha256, decision), `[source.*]`, `[apt]`, `[tool.<name>]` (bin, version_args, version_regex, expect, llvm_major / llvm_regex / ldd_llvm_regex, or `status = "not_installed"` for hayroll and kani per D-10).

Validation tests (`tool_03_pins_digests_and_commits_are_well_formed`): sha256 = 64 hex, commit = 40 hex, digest = `sha256:` + 64 hex, no empty or placeholder values. `tool_02_pins_llvm_major_in_supported_range_and_provisional`: major in 16..=19, status `provisional`.

---

### `crates/mt-toolchain/src/version.rs` and `check.rs` (utility/service, regex parse then compare)

**Analog:** none. Follow RES Pattern 3 and the "Version commands and parse targets" table (RES lines 549-561).

- Use `regex` with named groups (RES "Don't Hand-Roll"); never `split(' ')`.
- Do not hash or store raw `klee --version` output (it contains `Host CPU:`, Pitfall 5); store parsed fields only.
- Bitcode rustc: parse `^LLVM version: (\d+)\.(\d+)\.(\d+)`; fail if major != `[llvm].major` or major > `max_major` (19). Tests: `tool_02_bitcode_rustc_llvm_above_19_fails` (use the 1.99.0 string `23.1.1`) and `..._16_passes` (`16.0.5`).
- cargo-mutants argv is `["cargo","mutants","--version"]` (Pitfall 8).
- Output line per tool: `name  expected  actual  OK|MISMATCH|MISSING`; exit non-zero on any drift; `not_installed` tools never fail (D-10).
- Report words: timeouts are "not proven", missing layers "unavailable" (CONV).
- Fixture policy: tests for [ASSUMED] rows use representative strings first, then real CI captures committed under `crates/mt-toolchain/tests/fixtures/`.

---

### `crates/mt-toolchain/src/manifest.rs` (service, transform to JSON)

**Analog:** none. Follow GUIDE s4 lines 72-76: `BTreeMap`, `schema_version`, stable field order, timestamps only in an optional header field. D-06: manifest-identical means names, versions, source refs and source hashes only. Test: `tool_03_manifest_is_byte_stable_and_diff_detects_change` (comparer reports first differing key).

---

### `crates/mt-cli/src/main.rs` (controller, CLI request-response)

**Analog:** none. GUIDE s3 line 41: "argument parsing only, no logic". `clap` derive with a `toolchain` subcommand group: `check`, `manifest`, `build-args`, `hash <url>` (RES Architecture; D-16 leaves room for siblings). `anyhow` allowed here only (GUIDE s4 line 69). `hash <url>` must run `curl -fsSL` through the runner, not `Command`.

---

### Tests: `tool_01_no_python_outside_research`, command-confinement, forbid-unsafe (test, file-I/O)

**Source:** RES Example 2 (lines 399-416, shape ASSUMED but std-only). Walk repo root via `CARGO_MANIFEST_DIR/../..`, skip `.git`, `target`, `research` at the root only, never follow symlinks, extensions `py pyi pyw pyx`. Additional scans: `tool_01_command_only_in_runner_module` (grep `std::process::Command` in `crates/**/src`), `tool_01_every_tool_crate_forbids_unsafe` (each crate root contains `#![forbid(unsafe_code)]`). No new `walkdir` dependency.

**Decision-gate test (RES Example 3):** `tool_02_llvm_and_bitcode_pins_have_matching_project_decision` reads `.planning/PROJECT.md`, finds the line starting `- **D-15:**`, asserts it contains `llvm=16` and `bitcode-rustc=1.72.1` derived from pins. It must fail (not skip) when PROJECT.md is missing.

**Smoke-shape tests:** `tool_04_smoke_crate_has_emitted_rust_shape` and `tool_04_smoke_crate_is_outside_the_workspace`. The actual target build is CI-only (RES note at line 705; `#[ignore]` forbidden by GUIDE s5).

---

### `.planning/PROJECT.md` (modification: add decision D-15)

**Analog (exact, format):** existing decisions at `/home/user/migrationtool/.planning/PROJECT.md` lines 136-151, e.g.
```
- **D-08:** One LLVM major version in 16-19 is pinned for all C-side analysis, and Hayroll is never run on LLVM 20 (VERIFICATION R1).
```
Append `- **D-15:** Provisional pin for Phase 1: llvm=16 bitcode-rustc=1.72.1 (revisited at the Phase 6 R1 verdict; supersedes nothing in D-08).` D-15 is the next free ID (D-14 is the last). The same entry should record that the runner module is the one permitted `std::process::Command` user (GUIDE says rule changes need a recorded decision; RES Pitfall 12).

---

### `deny.toml` and `clippy.toml` (config)

**Analog:** RES Example 5 (lines 458-487), verified with cargo-deny 0.20.2. Copy verbatim. Licence allowlist matches GUIDE s4 line 95 (MIT, Apache-2.0, BSD-2/3-Clause, ISC, Zlib, Unicode-3.0). Workspace crates need `publish = false` (Pitfall 7).
```toml
# clippy.toml
disallowed-types = [
    { path = "std::process::Command", reason = "run external tools only through mt-toolchain's runner" },
]
```

---

### Root `Cargo.toml` and `rust-toolchain.toml` (config)

**Analog:** none. Per RES: `[workspace] members = ["crates/*"]`, `exclude = ["container/smoke"]`, `resolver = "2"` or `"3"`; `rust-toolchain.toml` with `channel = "1.99.0"`, `profile = "minimal"`, components `rustfmt`, `clippy`, targets `thumbv7em-none-eabihf` (musl target is also needed for the static `mt`; RES Example 6 lists both). `.planning/config.json` expects `cargo build --workspace` / `cargo test --workspace` (CONTEXT). Dependency versions: see RES Supporting table (serde 1.0.229, serde_json 1.0.151, sha2 0.11.0, thiserror 2.0.21, toml 1.1.7, regex 1.13.1, clap 4.6.7, anyhow 1.0.104, tempfile 3.27.0; proptest 1.11.0 optional). Planner note: c2rust is flagged SUS in RES Package Legitimacy Audit and needs a `checkpoint:human-verify` before first install (container build, not a Cargo dependency).

---

### `container/smoke/` (library, no_std, transform)

**Analog:** RES Example 4 (lines 426-452), verified to build for `thumbv7em-none-eabihf`, clippy clean. Normative shape from EMIT line 19 (`crate-type = ["staticlib","rlib"]`, `panic = "abort"` in dev and release), line 20 (`#[panic_handler]` behind default-on `panic-handler` feature), lines 30/38-46 (`ffi.rs` is the only module with unsafe; boundary must not panic), and the emitted lint table at EMIT line 87+ (`unsafe_code = "deny"`, `ffi.rs` alone has `#[allow(unsafe_code)]`).

Core excerpts to copy:
```toml
[lib]      crate-type = ["staticlib", "rlib"]
[features] default = ["panic-handler"]  panic-handler = []
[dependencies]
[profile.dev]     panic = "abort"
[profile.release] panic = "abort"  opt-level = "s"  lto = true  codegen-units = 1
[workspace]    # own workspace root
```
```rust
#![no_std]
mod ffi;
pub fn add(a: u32, b: u32) -> u32 { a.wrapping_add(b) }   // wrapping_* for unsigned (CONV Emitted Rust)
#[cfg(feature = "panic-handler")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! { loop {} }
```
`ffi.rs`: `#![allow(unsafe_code)]`, `#[no_mangle] pub extern "C" fn mt_smoke_add(...)`. Caveat for the planner: under newer editions `#[no_mangle]` needs `#[unsafe(no_mangle)]` (edition 2024) and CONV requires a `// SAFETY:` comment before each unsafe block; RES uses edition 2021. Location is `container/smoke/` not `fixtures/` (GUIDE s3 reserves `fixtures/` for planted-bug suites). Build command: `cargo build --release --target thumbv7em-none-eabihf --manifest-path container/smoke/Cargo.toml`.

---

### `container/Dockerfile` (config, image build)

**Analog:** RES "Dockerfile skeleton" (lines 567-605). Marked ASSUMED: assembled from verified commands, not built. Treat as a starting point only.

Patterns to preserve:
- `FROM --platform=linux/amd64 ${BASE_REF}@${BASE_DIGEST}`; all pins arrive as `ARG` with **no default** and are checked with `${NAME:?}` (RES Pattern 2). Tests `tool_03_dockerfile_has_no_floating_references` and `tool_03_dockerfile_args_match_pins` enforce this.
- Snapshot apt sources over `http://` (slim image has no ca-certificates, RES anti-patterns), `Acquire::Check-Valid-Until "false"`, retries/timeouts (Pitfall 11).
- Install only `*-16` LLVM packages; set `LLVM_CONFIG_PATH=/usr/lib/llvm-16/bin/llvm-config` (Pitfall 1).
- Clone-then-assert commit: `test "$(git -C /src/klee rev-parse HEAD)" = "${KLEE_COMMIT}"`.
- KLEE cmake flags: `-DENABLE_KLEE_ASSERTS=OFF -DENABLE_UNIT_TESTS=OFF -DENABLE_SYSTEM_TESTS=OFF -DENABLE_DOCS=OFF`, Z3 only (Pitfall 4).
- `mt` binary is mounted at run time, not baked into the image (keeps digest dependent only on Dockerfile + pins).
- Stage order: apt-base, rustup, arm, cargo-tools, klee.
- Install both `qemu-user` and `qemu-system-arm` (RES correction 1, D-11).

---

### `.github/workflows/ci.yml` and `container.yml` (config, batch)

**Analog:** none in repo (no `.github/`). Use RES "CI Design" (lines 752-777): job layout, `permissions:` least privilege, SHA-pinned actions from the table at RES lines 764-773 (re-verify SHAs with `git ls-remote` at plan time), free disk before image jobs, build-a/build-b in parallel with `--no-cache` on b, manifest `diff -u`, negative-pin check (`major = 17` must exit non-zero), push to GHCR by digest on main only. `ci.yml` runs exactly the CONV pre-merge list: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check`.

---

## Shared Patterns

### Crate hygiene (apply to every file in `crates/mt-toolchain` and `crates/mt-cli`)
**Source:** GUIDE s4 lines 68-70; CONV "Tool code".
- `#![forbid(unsafe_code)]` at each crate root.
- `thiserror` in `mt-toolchain`; `anyhow` only in `mt-cli`.
- No `unwrap`/`expect` outside tests unless the line has an invariant comment.
- Treat tool output and pins as untrusted: bounded reads, argv vectors, no shell strings.

### Deterministic serialization (runner records, manifest, build-args)
**Source:** GUIDE s4 lines 72-76.
`BTreeMap` for every map, `schema_version` field at the top level, no timestamps except an optional manifest header, sorted `build-args` output.

### Requirement-ID test naming
**Source:** GUIDE s4 line 87; RES Validation Architecture table (lines 680-703).
Every TOOL-0x requirement has tests named `tool_0x_...`. Tests never use the network (this is why the TOOL-04 target build is CI-only) and are never `#[ignore]`d.

### Single source of truth for pins
**Source:** CONTEXT D-14; RES Pattern 2.
Only `container/pins.toml` carries versions. Dockerfile ARGs, CI, and tests derive from it via `mt toolchain build-args`.

### Honest reporting
**Source:** GUIDE s4 lines 113-114.
Timeouts "not proven", missing layers "unavailable", never the words "certified" or "compliant".

## No Analog Found

All 27 files have no in-repo code analog (greenfield). Planner should use the normative sources above. Items with the weakest grounding, where RES itself marks content ASSUMED or unbuilt:

| File | Role | Data Flow | Reason |
|---|---|---|---|
| `container/Dockerfile` | config | batch | Skeleton unbuilt; apt versions, Arm hash and Z3 compatibility unverified (RES A1-A4, A6, A7) |
| `.github/workflows/*.yml` | config | batch | No workflow exists; design only, no YAML prototype |
| `crates/mt-toolchain/src/{version,check,manifest,pins}.rs` | utility/service | transform | Only schema sketches and parse tables exist; no executed prototype |
| `crates/mt-toolchain/tests/fixtures/*` | fixture | n/a | Output formats for clang, llvm-config, c2rust, bear, arm-gcc, qemu are ASSUMED (RES A5); replace with CI captures |

## Metadata

**Analog search scope:** whole repo via `git ls-files` (excluding `.claude/` agent and command files); `docs/`; `.planning/codebase/`; `.planning/PROJECT.md`.
**Files scanned:** about 30 tracked non-`.claude` files listed; 5 read in full or in the relevant ranges (CONTEXT, RESEARCH, GUIDE s3-s6, CONVENTIONS, EMIT grep).
**Pattern extraction date:** 2026-10-08
