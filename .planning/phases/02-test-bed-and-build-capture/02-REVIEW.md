---
phase: 02-test-bed-and-build-capture
reviewed: 2026-10-20T00:00:00Z
depth: standard
files_reviewed: 34
files_reviewed_list:
  - .github/workflows/testbed.yml
  - Cargo.toml
  - crates/mt-capture/src/capture.rs
  - crates/mt-capture/src/compare.rs
  - crates/mt-capture/src/droplog.rs
  - crates/mt-capture/src/entries.rs
  - crates/mt-capture/src/fetch.rs
  - crates/mt-capture/src/lib.rs
  - crates/mt-capture/src/manifest.rs
  - crates/mt-capture/src/report.rs
  - crates/mt-capture/src/seeding.rs
  - crates/mt-capture/src/testbed.rs
  - crates/mt-capture/tests/bed_01_entry_include_prefix.rs
  - crates/mt-capture/tests/build_01_capture_drivers.rs
  - crates/mt-capture/tests/build_01_capture_stages.rs
  - crates/mt-capture/tests/build_02_compare.rs
  - crates/mt-capture/tests/build_02_configure_evidence.rs
  - crates/mt-capture/tests/build_02_drop_log.rs
  - crates/mt-capture/tests/build_02_replay_pin.rs
  - crates/mt-capture/tests/build_02_workflow_static.rs
  - crates/mt-capture/tests/build_03_seed_manifest.rs
  - crates/mt-capture/tests/build_03_seed_narrowing.rs
  - crates/mt-cli/src/main.rs
  - crates/mt-cli/tests/build_01_capture_stages_cli.rs
  - crates/mt-cli/tests/build_02_compare_cli.rs
  - crates/mt-cli/tests/build_03_seeds_cli.rs
  - crates/mt-seeds/Cargo.toml
  - crates/mt-seeds/src/lexer.rs
  - crates/mt-seeds/src/zipsrc.rs
  - crates/mt-seeds/tests/build_03_seed_designators.rs
  - crates/mt-seeds/tests/build_03_seed_selection.rs
  - crates/mt-seeds/tests/build_03_zip_backend.rs
  - testbed/dropped.toml
  - testbed/manifest.toml
findings:
  critical: 0
  warning: 7
  info: 11
  total: 18
status: issues_found
---

# Phase 2: Code Review Report (re-review after gap closure)

**Reviewed:** 2026-10-20
**Depth:** standard
**Files Reviewed:** 34
**Status:** issues_found

## Summary

This is an incremental re-review of the Phase 2 gap-closure plans 02-11 to 02-19. It re-checks every finding of the 2026-10-13 review (WR-01..WR-10, IN-01..IN-08) against the current code, and reviews the diff since `88638d57` in the 34 listed files.

What I ran. `cargo test --offline -p mt-capture -p mt-seeds -p mt-cli` passes. `cargo clippy --offline --all-targets -p mt-capture -p mt-seeds -p mt-cli -- -D warnings` and `cargo fmt --check` are clean. There is no `unwrap`/`expect` in the source of these crates, no `std::process::Command` outside the runner, and the words "certified" and "compliant" do not appear in the diff. I did not run the workflow itself. For its behaviour I relied on reading it and on the 02-19 CI notices (run 39335185551, head `fb048678`), which show 46 capture rows, 31 replay rows all `identical`, and `compared 74 files / result: identical` for the determinism step.

Outcome of the re-check:
- Fixed: WR-02, WR-04, WR-05, WR-08, WR-10, IN-07.
- Partially fixed: WR-07. The decompressed-bytes budget is in. The member-count cap is deferred by the founder.
- Still open: WR-01, WR-03, WR-06, WR-09, and IN-01..IN-06 and IN-08. None of those files or code paths changed in the gap-closure plans, apart from a wider reach for WR-01 and IN-01 (see their entries).

What I found in the new code. The two-stage capture, the symlink-refusing reads, the replay pin binding, the 10-target floor, the fail-closed `compare`, the normalised `Makefile2` hash, the include-prefix scanner, the designator selector and the zip budget all behave as documented on the paths I traced. The new trust boundary has two gaps in its own claims, both in the record stage:
- WR-11: a build record that says `built: false` but names no cause yields a `captured` report with no gates evaluated. This contradicts the module doc, which says a forged record can only make its own pair read as built, failed or not proven.
- WR-12: the recorded toolchain-file hash is recomputed and never compared with the file the build actually used.

There are no BLOCKER findings.

## Previous Findings

| ID | Title | Status | Evidence |
|----|-------|--------|----------|
| WR-01 | Capture aborts the whole step on errors that come from untrusted upstream content | still open | `testbed.rs:601-608`: only `CaptureError::Fetch` becomes a row. Every other error returns from `capture_pairs`. The record stage reads untrusted run directories, which makes the other error classes more reachable (see below). |
| WR-02 | Replay does not tie the capture report to the manifest pin; `source.commit` is used unvalidated as a path component | fixed | `report_mismatch` (`testbed.rs:751-788`) requires target, config, `source.commit`, `source.tree_sha256` and the tool set to equal the manifest pin and the pins file. `plan_of` takes `pristine_src` from `pinned_source(..).commit` (`testbed.rs:725`), which `pin_of` validates as 40 lowercase hex digits (`fetch.rs:266-305`), never from the report. Test: `build_02_replay_pin.rs`. |
| WR-03 | Normalised comparison reads the original object from the work directory without checking it against the reported hash | still open | `replay.rs` is unchanged. `compare_differing` (`replay.rs:418-476`) still reads `original_objects_dir/<key>` and never checks `sha256(bytes) == original`. Exposure is lower than before, because that directory is now written only by the record stage, but the missing check is unchanged. |
| WR-04 | Untrusted upstream build code runs with write access to the evidence directories and with no resource limits | fixed | `capture_build` runs in its own container per pair (`run_build`, `testbed.yml:207-228`). It has no network and mounts only a scratch `/work`, the cache read-only and that pair's run directory. `/out` and `/work/capture` are not mounted. `capture_record` runs no upstream code and reads the run directory only through `read_run_file` (no symlinks, regular files only). `--pids-limit`, `--memory`, `--memory-swap` and `--cpus` are on all four helpers (`testbed.yml:158,176,191,219`). The determinism rerun builds into `work-rerun` and records to `/out/capture-rerun`. Residual gaps in the new boundary are WR-11 and WR-12. |
| WR-05 | The determinism comparison fails open when a report cannot be normalised | fixed | The step now runs `mt testbed compare` (`testbed.yml:501`). `compare_capture_dirs` (`compare.rs:83-174`) reports an unparseable file, an empty file, a file on one side only, a non-regular file and a comparison of zero files as differences. `passed()` needs `compared > 0`. CI notice: `compared 74 files`, `result: identical`. |
| WR-06 | The tree digest is ambiguous for file names that contain a newline | still open | `treehash.rs` is unchanged. `relative_path` still accepts a `\n` in a name. |
| WR-07 | The documented memory bound for zip seed sources does not hold | partially fixed | `zip_members(.., max_total_bytes)` (`zipsrc.rs:88-134`) bounds the decompressed bytes of one archive at `MAX_ZIP_DECOMPRESSED_BYTES` (8 MiB). A member whose declared size exceeds what is left is skipped unread, and every read is bounded at one byte past the allowed size. Tests: `build_03_zip_backend.rs`. The cap on the number of members is deferred by the founder and still absent. |
| WR-08 | The drop-log floor and the evidence fields are not enforced where the evidence is produced | fixed | `check_floor` runs in `load_inputs` for every `--all` step (`testbed.rs:285-309`) and returns `TestbedError::BelowFloor`. `DropLog::validate` now requires `differing_objects` and `differing_sections` for cause `differs` (`droplog.rs:278-297`). `ci_run` is still any non-zero number. That was a minor part of the original finding and cannot be checked offline. |
| WR-09 | Archive extraction relies only on the archive digest; the extracted tree is not checked for escaping links | still open | `fetch.rs:628-631` is unchanged: `tar -xjf ... --no-same-owner`, with no post-extraction link check. |
| WR-10 | `is_cmake_product` is broader than its documentation and masks a known non-determinism | fixed | `is_cmake_product` (`capture.rs:236-261`) matches exactly the documented paths: the three logs, `CMakeFiles/<digits.dots>/CMakeDetermineCompilerABI_{C,CXX}.bin` and `CMakeFiles/<version>/CompilerId{C,CXX}/CMake{C,CXX}CompilerId.o`. `.a`/`.so` are excluded only when the build step created them (provenance via `before`). `CMakeFiles/Makefile2` is hashed after `normalise_make_driver` and recorded with its rule in `normalised_configure_files`. The workflow comment (`testbed.yml:462-476`) lists every exclusion and the normalised file, and is held to `CONFIGURE_HASH_EXCLUSIONS` by a test. |
| IN-01 | Capture and entry tests pass without running when the Arm tools are missing | still open | `support::local_arm_tools()` (`tests/support/mod.rs:158-175`) still returns `None` and the test returns early. The new stage tests (`build_01_capture_stages.rs:100`) and include-prefix tests (`bed_01_entry_include_prefix.rs:279`) use it too, so the trust-boundary behaviour tests also pass vacuously on a host without the tools. |
| IN-02 | The manifest allows a configuration name that capture then rejects | still open | `is_config_name` (`manifest.rs:912-917`) still accepts a leading `-`. `pair_args` (`capture.rs:1880`) rejects it through `is_safe_component`. |
| IN-03 | `mt testbed validate` does not validate the drop log | still open | `run_validate` (`testbed.rs:366-380`) is unchanged. |
| IN-04 | The truncated flag of captured streams is ignored | still open | `entries.rs:793` and `capture.rs:1263` test `complete` only. |
| IN-05 | Stale version in a code comment | still open | `capture.rs:1004` still says "not honoured by CMake 3.28". The pin is 3.25.1. |
| IN-06 | Archive download has no size bound and is read fully into memory | still open | `fetch.rs:604` still does `fs::read(&download)`. There is no `--max-filesize`. |
| IN-07 | `isotp` is dropped as a whole target because of the probe design, not a defect of the target | fixed | `include_prefix` (`entries.rs`) reproduces the defining unit's include/define/undef prefix before the header, and falls back to the header alone when it cannot. The `isotp` entry is gone from `dropped.toml`. CI notice: `isotp/default captured`, then `identical` on replay, 16 of 16 targets retained. |
| IN-08 | `fetch` accepts `file://` URLs regardless of the allowed protocols | still open | `check_url` (`fetch.rs:220-228`) still accepts `file://`. |

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

### WR-11: A build record with `built: false` and no cause yields a `captured` report in which no gate ran

**File:** `crates/mt-capture/src/capture.rs:2019-2060` (module doc claim at `:57-63`)
**Issue:** In the `Accepted` branch of `capture_record` the record supplies `causes`, `not_proven`, `build_failed` and `built`. If `built` is false, the whole canonicalise/gates/entries block is skipped (`if record.built && ...`). The verdict is then computed from the absence of causes only:
```rust
} else if report.causes.is_empty() { CaptureVerdict::Captured }
```
A record `{built:false, causes:[], not_proven:false, build_failed:false}` therefore produces a `captured` report with `gates.compiler_allow_listed = false`, `gates.arch_ok = false`, no objects and no entries. `Gates` documents that "a gate that was not evaluated reads as failed", but the verdict never looks at the gates. `run_capture_record` prints `captured` and the step passes. Replay later catches it ("cannot read the canonical compile commands"), so no false replay pass results. The module doc nevertheless states that a forged record "can therefore make its own pair read as built, failed or not proven, and nothing else". A forged or corrupt record can make its pair read as captured, which is the one outcome the record stage exists to prevent. A genuine `capture_build` cannot produce this record, because `built` is false only after `halt`, which sets a flag.
**Fix:** Validate the record as untrusted data and make the verdict depend on the gates:
```rust
if !record.built && !record.build_failed && !record.not_proven {
    session.not_proven = true;
    session.cause("build_record_mismatch", "the build record says the build did not finish but names no failure".to_owned());
}
...
} else if report.causes.is_empty()
    && report.gates.compiler_allow_listed
    && report.gates.arch_ok
    && !report.objects.is_empty()
{
    CaptureVerdict::Captured
} else {
    CaptureVerdict::CaptureFailed
}
```
Add a `build_01_record_stage_refuses_a_record_that_did_not_build` test.

### WR-12: The recorded toolchain-file hash is recomputed, never compared with the file the build used

**File:** `crates/mt-capture/src/capture.rs:2027-2037`, `:1428-1431`
**Issue:** The build stage writes `stage/toolchain.cmake` and configures with it. The record stage recomputes the text from the pins and the configuration, stores that as `toolchain_file_sha256`, and writes it as the evidence copy. It never reads `stage/toolchain.cmake` (through `read_run_file` or otherwise). The report therefore states a hash for a file it did not verify. The file lies in the directory the build can write, and CMake re-runs its configure step during `cmake --build` when an input of the generated build system, such as the toolchain file, has changed. A build that rewrites the file would have its changed flags reach the compile commands while the report still shows the hash of the intended file. Nothing in `configure_files` covers `stage/`.
**Fix:** In the CMake branch, read `stage/toolchain.cmake` through `read_run_file` and compare it with the recomputed text. If it differs, add a cause such as `toolchain_changed`, with the detail naming the file. A refused read is `run_dir_symlink`.

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
**Issue:** `compare_capture_dirs(first, second)` does not check that the two arguments are different directories. A comparison of a tree with itself (a typo or a mis-set variable in the workflow) passes with `compared > 0`, which is the vacuous pass the fail-closed design is meant to prevent. The workflow passes distinct paths today.
**Fix:** In `compare_command`, canonicalise both paths and exit 2 when they are equal. Add a CLI test.

### IN-10: Replay is bound to the pin and the tools but not to the configuration's effective flags

**File:** `crates/mt-capture/src/testbed.rs:751-788`
**Issue:** `report_mismatch` checks target, config name, source pin and tool set. A change to `baseline.cflags`, a config's `defines`/`cflags`/`make_vars` or `cmake_options` in the committed manifest after the capture does not make the existing `capture.json` stale. Replay re-runs the stored compile commands and reports `identical` for flags the manifest no longer names. CI always captures fresh, so this affects only a reused work directory. The CMake toolchain hash is the only recorded flag fingerprint, and it is not compared either.
**Fix:** Record a hash of the effective flags and build arguments in the capture report, and compare it in `report_mismatch`.

### IN-11: `read_run_file` checks and then opens, and reads without a size bound

**File:** `crates/mt-capture/src/capture.rs:454-498`
**Issue:** The symlink checks use `symlink_metadata` and the read then uses `File::open`, which follows a symlink if a component is swapped in between. That is safe today because the build container has exited before the record stage runs. The in-process `capture()` and any future overlap would not be. The read is also unbounded, so a build-written multi-gigabyte file in `build/` is read whole.
**Fix:** Open with `O_NOFOLLOW` where available (check `fstat` after open for the final component) and cap the read, for example at 256 MiB, with a `NotRegular`-style refusal.

### IN-12: The build notice repeats the pair name on every row

**File:** `.github/workflows/testbed.yml:415`, `crates/mt-cli/src/main.rs:456`
**Issue:** `mt testbed capture --stage build` prints `<id>/<config> <word>`, and the workflow prefixes `$id/$cfg` again, so the notice reads `cjson/default cjson/default built`. Cosmetic, but it is the published evidence.
**Fix:** Drop the prefix in the workflow (`cat "$out/build-one.txt"`) or print only the word.

---

_Reviewed: 2026-10-20_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
