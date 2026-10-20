---
phase: 02-test-bed-and-build-capture
reviewed: 2026-10-20T00:00:00Z
depth: standard
files_reviewed: 6
files_reviewed_list:
  - crates/mt-capture/src/capture.rs
  - crates/mt-capture/src/cc.rs
  - crates/mt-capture/tests/build_01_capture_stages.rs
  - crates/mt-capture/tests/build_01_capture_verdict.rs
  - crates/mt-capture/tests/build_01_compile_commands.rs
  - crates/mt-capture/tests/build_02_replay.rs
findings:
  critical: 3
  warning: 8
  info: 14
  total: 25
status: issues_found
---

# Phase 2: Code Review Report (re-review after plan 02-20)

**Reviewed:** 2026-10-20
**Depth:** standard
**Files Reviewed:** 6 (the diff since `4733ea4`). Carried findings keep the file references of the review that raised them; those files were not re-read.
**Status:** issues_found

## Summary

This is an incremental re-review after plan 02-20 (gap closure G-02-5, G-02-6, G-02-7). It re-checks WR-11 and WR-12, carries every other earlier finding forward unchanged, and reviews the diff of the six files above.

Outcome for the two target findings:
- WR-11: fixed. `capture_record` now refuses a `built: false` record that names no failure, and `settle_verdict` makes `captured` depend on the gates and the object set.
- WR-12: partially fixed. `stage/toolchain.cmake` is now read through `read_run_file` and compared byte for byte with the recomputed text. The comparison sees only the end state of the file, so a build that changes it, lets CMake re-read it and then restores it is not detected.

What I ran, beyond reading. `arm-none-eabi-gcc` 13.2.1 and `arm-none-eabi-as` 2.42 are on this host, so I measured the claims of the new gate with `-###` (runs nothing) and, for file writes, with real compiles into the scratchpad. I also built a small scratch crate (outside the repository, path dependency on `mt-capture`) and fed the arguments below to the real `canonicalise`. I did not run the repository test suite or the workflow.

The new gate has three bypasses, each reproduced against the real gate and the real driver:
- CR-01: GCC's driver translates long spellings (`--plugin=`, `--compare-debug=`, `--opt-info*=`, `--dump-*=`, `--profile-use=`, `--profile-dir=`) into the `-f` options the gate refuses. The gate matches only the `-f` spelling. The doc comment on `HELPER_OPTION_PREFIXES` states the opposite ("there is no hidden spelling").
- CR-02: the assembler carrier gate is a deny-list of `--MD`, `-a`, `-o` and `@`. GNU `as` parses single-dash long options and unique abbreviations, so `-Wa,-MD,<path>`, `-Wa,-MD=<path>` and `-Wa,--M,<path>` write a file outside the run root and pass.
- CR-03: the compile-only test is "some argument equals `-c`". `-MD -MT -c ...` passes it, but GCC consumes that `-c` as the dependency target, so the driver compiles and then runs the linker. The new documentation assumes "the linker never runs for a `-c` command", so `-Wl,-Map=<path>` and `-Wl,-plugin,<so>` are not examined.

The gate does run at both ends before any spawn (`canonical_entry` at capture, `CcPolicy::check_entry` from replay's `validate`), and the helper-option scan does visit every argument after `argv[0]` (operands included). The problems are in what the gate recognises, not in where it runs.

## Previous Findings

| ID | Title | Status | Evidence |
|----|-------|--------|----------|
| WR-01 | Capture aborts the whole step on errors that come from untrusted upstream content | still open | `testbed.rs:601-608` unchanged. WR-14 is one more instance of the same class in the new code. |
| WR-02 | Replay does not tie the capture report to the manifest pin; `source.commit` is used unvalidated as a path component | fixed | Confirmed by the 2026-10-20 re-review (`report_mismatch`, `testbed.rs:751-788`). |
| WR-03 | Normalised comparison reads the original object from the work directory without checking it against the reported hash | still open | `replay.rs` is not in the diff. |
| WR-04 | Untrusted upstream build code runs with write access to the evidence directories and with no resource limits | fixed | Confirmed by the 2026-10-20 re-review. Residual gaps in the new boundary: WR-12, CR-01..CR-03. |
| WR-05 | The determinism comparison fails open when a report cannot be normalised | fixed | Confirmed by the 2026-10-20 re-review. |
| WR-06 | The tree digest is ambiguous for file names that contain a newline | still open | `treehash.rs` is not in the diff. |
| WR-07 | The documented memory bound for zip seed sources does not hold | partially fixed (deferred) | Member-count cap deferred by the founder (02-13-SUMMARY). |
| WR-08 | The drop-log floor and the evidence fields are not enforced where the evidence is produced | fixed | Confirmed by the 2026-10-20 re-review. |
| WR-09 | Archive extraction relies only on the archive digest; the extracted tree is not checked for escaping links | still open | `fetch.rs` is not in the diff. |
| WR-10 | `is_cmake_product` is broader than its documentation and masks a known non-determinism | fixed | Confirmed by the 2026-10-20 re-review. |
| WR-11 | A build record with `built: false` and no cause yields a `captured` report in which no gate ran | fixed | `capture.rs:2123-2143` sets `not_proven` and the cause `build_record_inconsistent` for a record with `built: false` that does not carry the matching flag and token (`build_failed` with cause `build_failed`, or `not_proven` with cause `not_proven`; this is exactly what `build_step` writes at `:1357-1370`, and `build` has no other `Flow::Halt` path). `settle_verdict` (`:1281-1301`) returns `captured` only for no cause, `compiler_allow_listed`, `arch_ok` and a non-empty object set, otherwise `gates_not_passed` and `capture_failed`; `capture()` goes through the same function. Tests: `build_01_captured_verdict_needs_passed_gates_and_objects` (pure), `build_01_record_stage_refuses_a_record_that_did_not_build` (three forgeries and the genuine record). No fail-open path found: every `Accepted` record either runs the gates or is `not_proven`/`build_failed`. |
| WR-12 | The recorded toolchain-file hash is recomputed, never compared with the file the build used | partially fixed | `check_toolchain_file` (`capture.rs:1863-1885`) reads `stage/toolchain.cmake` with `read_run_file` and compares bytes with the recomputed text; a different or missing file is `toolchain_changed`, a link or non-regular file is `run_dir_symlink`. Tests cover append, delete, symlink and restore. The comparison is end-state only (see WR-12 below). |
| IN-01 | Capture and entry tests pass without running when the Arm tools are missing | still open | The new stage tests use `local_arm_tools()` the same way. |
| IN-02 | The manifest allows a configuration name that capture then rejects | still open | |
| IN-03 | `mt testbed validate` does not validate the drop log | still open | |
| IN-04 | The truncated flag of captured streams is ignored | still open | |
| IN-05 | Stale version in a code comment | still open | |
| IN-06 | Archive download has no size bound and is read fully into memory | still open | |
| IN-07 | `isotp` is dropped as a whole target because of the probe design, not a defect of the target | fixed | Confirmed by the 2026-10-20 re-review. |
| IN-08 | `fetch` accepts `file://` URLs regardless of the allowed protocols | still open | |
| IN-09 | `mt testbed compare` accepts the same tree as both sides | still open | |
| IN-10 | Replay is bound to the pin and the tools but not to the configuration's effective flags | still open | |
| IN-11 | `read_run_file` checks and then opens, and reads without a size bound | still open | `read_run_file` is now also used for the toolchain comparison; the unbounded read applies there too. |
| IN-12 | The build notice repeats the pair name on every row | still open | |

## Critical Issues

### CR-01: GCC long-option spellings bypass the helper-option, write-path and `-fprofile-` gates

**File:** `crates/mt-capture/src/cc.rs:233-251` (claim and prefix set), `:262-270` (`find_helper_option`), `:336-460` (`check_write_paths`), `:575-583` (`canonical_entry`)
**Issue:** The gate compares the recorded arguments with `-f...` spellings. The GCC driver rewrites a `--<name>` argument into `-f<name>` before the compiler sees it (the generic `--` row of its option map). I measured this with `arm-none-eabi-gcc -###` on 13.2.1:

| Argument the gate sees | What `cc1` receives | Gate result |
|---|---|---|
| `--plugin=/work/run/t/default/evil.so` | `-fplugin=...` | accepted |
| `--compare-debug=-wrapper,/work/run/t/default/w.sh` | `-fcompare-debug=-wrapper,...` | accepted |
| `--opt-info-all=/out/leak.txt` | `-fopt-info-all=/out/leak.txt` | accepted |
| `--dump-tree-all=/out/leak` | `-fdump-tree-all=/out/leak` | accepted |
| `--dump-final-insns=<path>` | `-fdump-final-insns=<path>` | accepted |
| `--profile-use=<path>`, `--profile-dir=<path>` | `-fprofile-use=`, `-fprofile-dir=` | accepted (the `-fprofile-` forbidden-flag check is bypassed) |

The "accepted" column is from running the real `canonicalise` with each argument in a one-entry export. Real compiles confirm the write half: `arm-none-eabi-gcc -c x.c --dump-final-insns=<outside>/f` and `--opt-info-all=<outside>/o` created both files outside the working directory. `--plugin=<so>` loads a shared object into `cc1`; the build can create a `.so` inside the run root, so replay (which runs every recorded command) would load attacker code. That is exactly threat T-02-67, which 02-20 reports as closed. The doc comment at `cc.rs:233` ("GCC rejects abbreviated and doubled spellings ... so there is no hidden spelling") is therefore wrong, and the same claim sits in the 02-20 summary and probably in the 02-SECURITY.md rows for T-02-67 and T-02-70, which need to be re-opened.

Driver-level spellings (`--wrapper`, `--pref=`, `--spec=`, `--iplugindir=`, `--dumpbase=`) are rejected by GCC, as the author measured. The hole is the `-f` family only.
**Fix:** Normalise the spelling before any decision, and apply every check to the normalised form.
```rust
/// What the driver will see: `--plugin=x` is `-fplugin=x`.
fn driver_spelling(argument: &str) -> std::borrow::Cow<'_, str> {
    match argument.strip_prefix("--") {
        Some(rest) if !LONG_OPTIONS_WITH_OWN_MEANING.iter().any(|l| rest.starts_with(l)) => {
            format!("-f{rest}").into()
        }
        _ => argument.into(),
    }
}
```
where `LONG_OPTIONS_WITH_OWN_MEANING` holds the entries GCC maps differently (`prefix`, `specs`, `output`, `dumpdir`, `dumpbase`, `for-assembler`, `sysroot`, `write-dependencies`, `write-user-dependencies`, `dependencies`, `user-dependencies`). The simpler and safer rule is to refuse every argument that starts with `--` and is not on a short allow-list (`--sysroot=`, `--output`, `--output=`, `--write-dependencies`, `--write-user-dependencies`, `--dumpdir`, `--dumpbase`, `--dumpbase-ext`), and to run `find_helper_option` and `check_write_paths` on the allow-listed rest. Add tests that pair each `-f` refusal case with its `--` spelling, and replace the "no hidden spelling" sentence with the measured fact.

### CR-02: The assembler carrier deny-list misses single-dash and abbreviated GNU `as` options

**File:** `crates/mt-capture/src/cc.rs:282-298` (`ASSEMBLER_WRITE_PREFIXES`, `writes_through_assembler`), `:375-424` (use)
**Issue:** `ASSEMBLER_WRITE_PREFIXES` is `["-a", "--MD", "-o", "@"]`. GNU `as` reads its options with `getopt_long_only`, so a long option may be written with one dash and may be abbreviated to a unique prefix. I measured with `arm-none-eabi-gcc -c x.c <arg> -o x.o` on 13.2.1 (binutils 2.42), which created the file in each case:
- `-Wa,-MD,<outside>/e.d`: wrote a dependency file (accepted by the gate)
- `-Wa,-MD=<outside>/e.d`: wrote it (accepted)
- `-Wa,--M,<outside>/e.d`: wrote it through the abbreviation (accepted)

`-Wa,--MD,<path>` and `-Wa,-alh=<path>` (the spellings the tests use) are refused, which is why the tests pass. The first three go through `canonicalise` without error, so the write-path claim "every write path is bounded to the run root or refused" is false for the assembler, at capture and at replay. The same holds for `-Xassembler -MD -Xassembler <path>` and `--for-assembler=-MD,<path>`.
**Fix:** Do not enumerate what an assembler may write; enumerate what is allowed through. A short allow-list (`-mthumb`, `-mcpu=`, `-march=`, `-mfpu=`, `-mfloat-abi=`, `-mimplicit-it=`, `--defsym`, `-gdwarf`, `--gdwarf-<n>`, `--noexecstack`) covers what real builds pass, and anything else is `write_redirect`:
```rust
fn assembler_piece_allowed(piece: &str) -> bool {
    ASSEMBLER_ALLOWED_PREFIXES.iter().any(|prefix| piece.starts_with(prefix))
}
// -Wa,<piece>[,<piece>...], -Xassembler <piece>, --for-assembler[=]<pieces>: every piece must pass.
```
Add the three measured spellings (and `-Wa,-M,<path>`) as redirect cases in `build_01_handles_write_path_options` and `build_02_replay_refuses_writes_outside_the_run_root`.

### CR-03: The compile-only check accepts a `-c` that GCC reads as an operand, so the linker runs and `-Wl,` is unexamined

**File:** `crates/mt-capture/src/cc.rs:559-563` (compile-only test), `:229-230` (doc claim "the linker never runs for one"), `:144-158` (the operand table that the `-c` test does not use)
**Issue:** `canonical_entry` accepts an entry when any argument equals `-c`. `check_write_paths` and `derive_output` now know that `-MT`, `-MQ`, `-MF`, `-dumpdir`, `-dumpbase`, `-aux-info` and so on consume the next argument, but the `-c` test does not use that knowledge. With `-MD -MT -c a.c -o out/a.o`, GCC takes `-c` as the dependency target, performs no compile-only step, and runs `collect2`/`ld`. I verified with `-###` (the `collect2` line carries `-Map=...`) and with a real run:
```
arm-none-eabi-gcc -MD -MT -c x.c -o x.o -nostdlib -nostartfiles -Wl,-Map=<outside>/m.map -Wl,-e,0
```
wrote `<outside>/m.map` and produced an ELF executable at `x.o`. The real `canonicalise` accepted the same argument vector (`-Wl,-Map=...` and `-Wl,-plugin,<so>` are not examined; `-Wl,-plugin` loads a shared object into `ld`). The doc comment for `HELPER_OPTION_PREFIXES` lists `-Wl,` and `--for-linker` as "considered and not refused" because they "run nothing for a `-c` command"; that premise is the broken one. A hostile export entry therefore writes outside the run root and loads code, at capture only if the object passes the later gates, and at replay unconditionally. Replay's `check_entry` does not repeat the `-c` test at all (IN-13).
**Fix:** Decide "compile-only" from the same operand-aware walk the write gate uses, so that an operand is never read as an option, and refuse the linker's carriers outright:
```rust
fn options(arguments: &[String]) -> impl Iterator<Item = (&str, Option<&str>)> { /* one walk, one table */ }
// compile-only: some top-level option equals "-c"
// refuse: -Wl,*, -Xlinker, --for-linker, -T*, -nostdlib, -static, -shared, -l*, -L* when not compile-only
```
Run the same check from `CcPolicy::check_entry` so that replay does not trust the capture stage for it. Add a test with `-MD -MT -c` that must be `NotCompileOnly`, at both ends.

## Warnings

### WR-01: Capture aborts the whole step on errors that come from untrusted upstream content

**File:** `crates/mt-capture/src/testbed.rs:601-608`, `crates/mt-capture/src/capture.rs:1051-1082`, `:1123-1136`, `:1392-1401`
**Status:** still open (original finding, wider reach)
**Issue:** `capture_pairs` turns only `CaptureError::Fetch` into a row; any other error returns from the step. The module contract (`testbed.rs:6-9`) says a failing build, gate or comparison is a row, never an error. The record stage is now documented to treat the run directory as untrusted data, and it hits these paths:
- `list_files` returns `InvalidInput` for a non-UTF-8 name anywhere under the run root (`gates` lists the whole run root) or the build tree, or `Io` for an unreadable directory.
- `timestamp_entries` does `fs::read` of each entry's source and fails with `Io` if the source is gone.
- `read_or_refuse` returns `Io` for a missing or unreadable file, which aborts instead of becoming a cause.

One such pair ends the step with exit 2, writes no `summary-capture.json`, and hides the verdicts of every later pair. The failure is closed (the step is red), but the evidence for the other pairs is lost.
**Fix:** In `capture_pairs`, treat errors tied to one pair's tree as that pair's failure:
```rust
Err(error @ (CaptureError::Tree(_) | CaptureError::Io { .. } | CaptureError::InvalidInput(_)
    | CaptureError::Cc(_))) => {
    summary.add(&row, &format!("capture_failed: {error}"), false);
    continue;
}
Err(error) => return Err(error.into()), // Runner, Json stay fatal
```
Better, record the error as a cause inside `capture_record` so that a `capture.json` with verdict `capture_failed` is still written.

### WR-03: Normalised comparison reads the original object from the work directory without checking it against the reported hash

**File:** `crates/mt-capture/src/replay.rs:418-476`
**Status:** still open
**Issue:** `compare_differing` reads `original_objects_dir/<key>` and uses those bytes for `differing_sections` and for the `objcopy` run. It never checks that `sha256(original_bytes) == original`. `IdenticalAfterNormalisation` can therefore be produced from a file that is not the object the report hashes. The record stage is now the only writer of that directory, so a tamper path no longer runs through the untrusted build, but the check is still missing and costs one line.
**Fix:**
```rust
if sha256_hex(&original_bytes) != original {
    return Ok(unchanged(differs(original, replay_hash, Vec::new(),
        Some("the stored original object does not match its reported sha256".to_owned()))));
}
```

### WR-06: The tree digest is ambiguous for file names that contain a newline

**File:** `crates/mt-capture/src/treehash.rs:83-120`
**Status:** still open
**Issue:** Rows are `"<mode> <sha256> <relpath>\n"` concatenated. A file whose name contains `\n` plus a forged row can produce the same bytes as two files, so two different trees can share one digest. `relative_path` accepts any UTF-8 name, and no test covers this.
**Fix:** Reject `char::is_control` (at least `\n`) in `relative_path` and add `bed_01_tree_hash_rejects_newline_in_name`.

### WR-07: The documented memory bound for zip seed sources does not hold

**File:** `crates/mt-seeds/src/zipsrc.rs:88-134`, `crates/mt-capture/src/seeding.rs:40-57`
**Status:** partially fixed
**Issue:** The decompressed-bytes budget is in place and correct on every path I traced:
- Zero budget reads nothing.
- A member that fits exactly is kept.
- A lying header is caught by the bounded read.
- The `too_large` count includes refused members.

What remains is the second half of the original finding. The number of members is bounded only by the 2 MiB archive size (tens of thousands of central-directory entries), and each is opened with `by_index` and `enclosed_name`. The founder has deferred this. It is carried forward for the ledger, not as a new defect.
**Fix:** When it is taken up, return `SeedError` or count the rest as `unsupported` once `archive.len()` exceeds a constant (for example 10 000).

### WR-09: Archive extraction relies only on the archive digest; the extracted tree is not checked for escaping links

**File:** `crates/mt-capture/src/fetch.rs:624-640`
**Status:** still open
**Issue:** `tar -xjf archive --strip-components=N --no-same-owner -C partial` extracts with no check that no member is a link pointing outside `partial` or that nothing was written through such a link. `check_tree` hashes only `partial`. This step runs in the networked container with `/work` writable.
**Fix:** Pass `--no-same-permissions --no-overwrite-dir` and walk `partial` with the escaping-link rules of `copy_tree` before hashing, failing the fetch with `EscapingSymlink` and removing the archive.

### WR-12: The recorded toolchain-file hash is recomputed, never compared with the file the build used

**File:** `crates/mt-capture/src/capture.rs:1863-1885`, `:2145-2156`; module doc at `:44-58` and `:86`
**Status:** partially fixed
**Issue:** The original defect (the record stage never read `stage/toolchain.cmake`) is closed: `check_toolchain_file` reads the file through `read_run_file` and raises `toolchain_changed` when its bytes differ from the text recomputed from the pins, or when it is missing, and `run_dir_symlink` for a link. A file left modified is caught, which `build_01_record_stage_refuses_a_changed_toolchain_file` proves for four cases.

What is still open. The comparison sees the final state of the file only. The module doc says a forged record or build "cannot change the toolchain file unnoticed". The build is arbitrary upstream code in the same directory. Sketch of a path that is not noticed: `CMakeLists.txt` writes `toolchain.cmake` with extra flags during the first configure (this run has already read the file); `cmake --build` sees the newer toolchain file, regenerates, reads the changed flags, and the second configure writes the original text back. The compile commands carry the changed flags, and `toolchain_file_sha256` still names the intended file. Test case (4) of the new test (restore the genuine bytes, expect `captured`) shows the check cannot tell this from an untouched file. The compile commands are still gated, so this is an evidence-integrity gap, not an escape: the report states a flag fingerprint that the objects were not built with.
**Fix:** Either weaken the doc claim to "a toolchain file left changed is noticed", or make the check independent of the file's final state: require that each canonical entry's arguments contain the expanded baseline flags in order (the entry flags are what replay runs), and raise `toolchain_changed` otherwise. Record the file's mtime ordering only as a secondary signal.

### WR-13: The helper-option gate refuses flags that normal Cortex-M builds use, and applies to commands that were never built

**File:** `crates/mt-capture/src/cc.rs:241-251`, `:253-259`; `crates/mt-capture/src/cc.rs:618-640` (`canonicalise` gates before the existence filter)
**Issue:** Two over-refusals.
1. `-specs`/`--specs` are refused for every name. `--specs=nano.specs` and `-specs=nosys.specs` are common in compile flags of Cortex-M projects, the product's first market, and the founder's own test uses `--specs=nano.specs` as the refused example. A bare name is looked up in the toolchain's library directory first: I measured with `-###` that a planted `./nano.specs` does not shadow the toolchain's `nano.specs` (GCC printed `Reading specs from .../arm-none-eabi/lib/nano.specs`), while `-specs=evil.specs`, which the toolchain does not have, is read from the current directory (`Reading specs from evil.specs`). So a fixed list of names the pinned toolchain ships (for example `nano.specs`, `nosys.specs`, `rdimon.specs`, each checked to exist in the container's lib directory) can be allowed safely; any other name or a name with `/` stays refused. Such a module is refused today with `helper_option` and no way forward except an edit to the customer's flags.
2. `canonicalise` applies the gates to every raw entry and only then drops entries whose object does not exist. CMake exports every target, including ones `cmake --build` never builds. One unbuilt example or test target with a refused option (`-Wp,-MMD,...`, `-specs=`, `@file`) makes the whole pair `capture_failed`, although that command is never replayed. Refusing is the safe direction, but the cause names a command that has no effect on the evidence.

Also refused though harmless: `-MF /dev/null`, `-o /dev/null` (outside the run root), and `-Wp,-MD`-style kbuild depfile flags, which the doc acknowledges.
**Fix:** (1) Add an allow-list of spec names verified to exist in the pinned toolchain, tested against a planted same-named file. (2) Run the existence filter first and gate only the kept entries; keep the gate for entries whose object exists. Add both cases to `build_01_driver_option_gate_keeps_ordinary_flags`.

### WR-14: `check_toolchain_file` turns an unreadable toolchain file into a fatal error instead of a cause

**File:** `crates/mt-capture/src/capture.rs:1881`
**Issue:** The `Err(RunReadError::Io { path, source })` arm other than `NotFound` returns `Err(io_error(..))`. The build and record stages both run as uid 65534 (`testbed.yml:158-218`), so a build that does `chmod 000 stage/toolchain.cmake` makes the record stage's `File::open` fail with `PermissionDenied`. That aborts `capture_record` before `capture.json` is written and, through `capture_pairs`, ends the whole capture step, which is the WR-01 class again in code added by 02-20. The sibling `copy_raw_export` has the same shape (`:1547`).
**Fix:** Treat any read failure of a file the build owns as a cause of that pair:
```rust
Err(RunReadError::Io { path, source }) => self.cause(
    "toolchain_changed",
    format!("{key} cannot be read: {path:?}: {source}"),
),
```
and do the same in `copy_raw_export`, with a test that sets the file mode to 0o000.

## Info

### IN-01: Capture and entry tests pass without running when the Arm tools are missing

**File:** `crates/mt-capture/tests/support/mod.rs:158-175`, `build_01_capture_stages.rs:100`, `bed_01_entry_include_prefix.rs:279`, `bed_01_entries.rs:445`
**Status:** still open
**Issue:** `local_arm_tools()` returns `None`, the test returns early and `cargo test` hides the "skipping" line. This now covers the behaviour tests of the new trust boundary (stage separation, `no_build_record`, `inputs_changed`) and of the include prefix. Only `build_01_run_dir_reads_refuse_symlinks` is pure.
**Fix:** Fail instead of skipping when `MT_REQUIRE_ARM_TOOLS=1` is set, and set it in CI.

### IN-02: The manifest allows a configuration name that capture then rejects

**File:** `crates/mt-capture/src/manifest.rs:912-917`, `crates/mt-capture/src/capture.rs:1880`
**Status:** still open
**Issue:** `is_config_name` accepts a leading `-`. `pair_args` requires `is_safe_component`. `mt testbed validate` passes and `capture` aborts with `InvalidInput`.
**Fix:** Reject a leading `-` in `is_config_name`.

### IN-03: `mt testbed validate` does not validate the drop log

**File:** `crates/mt-capture/src/testbed.rs:366-380`
**Status:** still open
**Issue:** `run_validate` loads and validates only the manifest. A malformed `dropped.toml` first fails in the fetch step. The validate step reports "valid" for a state the next step refuses.
**Fix:** Load the drop log in `run_validate`, call `validate(&manifest)` and `check_floor`, and add a `drop log` row.

### IN-04: The truncated flag of captured streams is ignored

**File:** `crates/mt-capture/src/entries.rs:793`, `crates/mt-capture/src/capture.rs:1263`
**Status:** still open
**Issue:** Callers test `complete` but not `truncated`. A clang AST dump or `readelf` output cut between two documents is parsed as stored.
**Fix:** Treat `stdout.truncated` as `NotProven`.

### IN-05: Stale version in a code comment

**File:** `crates/mt-capture/src/capture.rs:1004`
**Status:** still open
**Issue:** The comment says "not honoured by CMake 3.28". The pinned CMake is 3.25.1.
**Fix:** Correct the version or drop it.

### IN-06: Archive download has no size bound and is read fully into memory

**File:** `crates/mt-capture/src/fetch.rs:604`, `crates/mt-toolchain/src/fetch.rs:50-67`
**Status:** still open
**Issue:** There is no `--max-filesize`, and the digest check reads the whole file into memory.
**Fix:** Add `--max-filesize <N>` and hash in chunks.

### IN-08: `fetch` accepts `file://` URLs regardless of the allowed protocols

**File:** `crates/mt-capture/src/fetch.rs:220-228`, `crates/mt-toolchain/src/fetch.rs:50-67`
**Status:** still open
**Issue:** `check_url` allows `file://`, and `curl_argv` always passes `--proto =https,file`, independent of `FetchContext.allowed_protocols`. `Manifest::validate` rejects it, so the CLI path is safe. The API path is not.
**Fix:** Reject `file://` unless `allowed_protocols` contains `file`, and derive `--proto` from the same setting.

### IN-09: `mt testbed compare` accepts the same tree as both sides

**File:** `crates/mt-capture/src/compare.rs:83`, `crates/mt-cli/src/main.rs:389-407`
**Status:** still open
**Issue:** `compare_capture_dirs(first, second)` does not check that the two arguments are different directories. A comparison of a tree with itself (a typo or a mis-set variable in the workflow) passes with `compared > 0`, which is the vacuous pass the fail-closed design is meant to prevent. The workflow passes distinct paths today.
**Fix:** In `compare_command`, canonicalise both paths and exit 2 when they are equal. Add a CLI test.

### IN-10: Replay is bound to the pin and the tools but not to the configuration's effective flags

**File:** `crates/mt-capture/src/testbed.rs:751-788`
**Status:** still open
**Issue:** `report_mismatch` checks target, config name, source pin and tool set. A change to `baseline.cflags`, a config's `defines`/`cflags`/`make_vars` or `cmake_options` in the committed manifest after the capture does not make the existing `capture.json` stale. Replay re-runs the stored compile commands and reports `identical` for flags the manifest no longer names. CI always captures fresh, so this affects only a reused work directory. The CMake toolchain hash is the only recorded flag fingerprint, and it is not compared either.
**Fix:** Record a hash of the effective flags and build arguments in the capture report, and compare it in `report_mismatch`.

### IN-11: `read_run_file` checks and then opens, and reads without a size bound

**File:** `crates/mt-capture/src/capture.rs:454-498`
**Status:** still open
**Issue:** The symlink checks use `symlink_metadata` and the read then uses `File::open`, which follows a symlink if a component is swapped in between. That is safe today because the build container has exited before the record stage runs. The in-process `capture()` and any future overlap would not be. The read is also unbounded, so a build-written multi-gigabyte file in `build/` is read whole.
**Fix:** Open with `O_NOFOLLOW` where available (check `fstat` after open for the final component) and cap the read, for example at 256 MiB, with a `NotRegular`-style refusal.

### IN-12: The build notice repeats the pair name on every row

**File:** `.github/workflows/testbed.yml:415`, `crates/mt-cli/src/main.rs:456`
**Status:** still open
**Issue:** `mt testbed capture --stage build` prints `<id>/<config> <word>`, and the workflow prefixes `$id/$cfg` again, so the notice reads `cjson/default cjson/default built`. Cosmetic, but it is the published evidence.
**Fix:** Drop the prefix in the workflow (`cat "$out/build-one.txt"`) or print only the word.

### IN-13: Replay's `check_entry` repeats only part of the capture gate

**File:** `crates/mt-capture/src/cc.rs:472-495` against `:559-583`
**Issue:** The 02-20 summary describes "gate-at-both-ends: the same CcPolicy check runs when the export is canonicalised and again in replay's validate". It is not the same check. `canonical_entry` also requires `-c` and rejects `-flto` and `-fprofile-*`; `check_entry` does neither, so replay trusts the capture stage for the compile-only invariant that CR-03 shows is weak. Replay's input is the record stage's own output, so this is defence in depth, not a new hole.
**Fix:** Move the `-c`, `-flto` and `-fprofile-` tests into one function that both `canonical_entry` and `check_entry` call, and test replay with an entry lacking `-c`.

### IN-14: Three hand-kept option tables must agree for the gate to be sound

**File:** `crates/mt-capture/src/cc.rs:144-158` (`SEPARATE_OPERAND_OPTIONS`), `:336-360` (`PATH_SEPARATE`, `NAME_SEPARATE`, `BOUNDED_PREFIXES`), `:273-298` (carrier lists)
**Issue:** `derive_output` and `check_write_paths` each walk the arguments with their own list of operand-consuming options. If they disagree, the gate checks one output while replay writes another, and CR-03 shows what an operand read as an option costs. They agree today (13 entries against 6 + 4 + 3), and nothing but review keeps them in step; `-c` detection is a third walk that does not use either.
**Fix:** One table (`name`, `operand kind`) and one iterator `options(arguments)` used by `derive_output`, `check_write_paths` and the compile-only test, plus a unit test that every table entry is consumed identically by all three.

### IN-15: Some of the new refusal evidence is not live or not run in CI

**File:** `crates/mt-capture/tests/build_02_replay.rs:455-470`, `:496-560`; `crates/mt-capture/tests/build_01_compile_commands.rs:635-640`
**Issue:** `assert_probe_is_live` proves the positive control for `-wrapper` and `-B` only; the `@response`, `--prefix`, `-specs` and `-fplugin` cases assert the refusal without showing that the unguarded driver would have run something. The local-captures test (`refused=0` over 29 captures, quoted as evidence in the 02-20 summary) is `#[ignore]` and needs a local directory, so CI never repeats it. None of the tests exercises a spelling the author had not already listed, which is why CR-01 to CR-03 passed.
**Fix:** Add a positive control for the response-file case, and add a table of the measured bypass spellings from CR-01 to CR-03 (each marked with the output of `-###` that shows it reaches the compiler) as a pure test.

---

_Reviewed: 2026-10-20_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
