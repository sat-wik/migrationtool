---
phase: 02-test-bed-and-build-capture
reviewed: 2026-10-13T00:00:00Z
depth: standard
files_reviewed: 36
files_reviewed_list:
  - .github/workflows/testbed.yml
  - .github/workflows/pin-discovery.yml
  - .gitignore
  - Cargo.toml
  - container/Dockerfile
  - container/pins.toml
  - crates/mt-capture/src/capture.rs
  - crates/mt-capture/src/cc.rs
  - crates/mt-capture/src/droplog.rs
  - crates/mt-capture/src/elf.rs
  - crates/mt-capture/src/entries.rs
  - crates/mt-capture/src/fetch.rs
  - crates/mt-capture/src/lib.rs
  - crates/mt-capture/src/manifest.rs
  - crates/mt-capture/src/replay.rs
  - crates/mt-capture/src/report.rs
  - crates/mt-capture/src/seeding.rs
  - crates/mt-capture/src/testbed.rs
  - crates/mt-capture/src/treehash.rs
  - crates/mt-capture/tests/build_02_workflow_static.rs
  - crates/mt-capture/tests/build_02_replay.rs
  - crates/mt-capture/tests/bed_01_tree_hash.rs
  - crates/mt-capture/tests/support/mod.rs
  - crates/mt-cli/src/main.rs
  - crates/mt-seeds/src/hex.rs
  - crates/mt-seeds/src/index.rs
  - crates/mt-seeds/src/lexer.rs
  - crates/mt-seeds/src/lib.rs
  - crates/mt-seeds/src/select.rs
  - crates/mt-seeds/src/zipsrc.rs
  - crates/mt-toolchain/src/pins.rs
  - crates/mt-toolchain/src/runner.rs
  - testbed/dropped.toml
  - testbed/targets/mbedtls/user_config.h
  - crates/mt-capture/Cargo.toml
  - crates/mt-seeds/Cargo.toml
findings:
  critical: 0
  warning: 10
  info: 8
  total: 18
status: issues_found
---

# Phase 2: Code Review Report

**Reviewed:** 2026-10-13
**Depth:** standard
**Files Reviewed:** 36
**Status:** issues_found

## Summary

Scope was the Phase 2 tool code (`mt-capture`, `mt-seeds`, `mt-cli`), the `testbed` workflow, the drop log and the Phase 1 files touched by Phase 2. The `src/` files were read in full. The test files were sampled: the workflow static tests, the replay tests, the tree-hash tests and the shared support module. The remaining test files and `testbed/manifest.toml` were not read line by line. The rest of the review was driven from the sources of the evidence path.

Project rules hold. Every crate forbids unsafe code, there is no `unwrap` or `expect` in the source, and no `std::process::Command` outside the runner. Reports use `BTreeMap` and carry no timestamps, and the words "certified" and "compliant" do not appear.

I found no defect that makes the harness report a pass on a real divergence in normal operation. The replay comparison fails closed in every branch I traced:
- The object-level comparison is a whole-file sha256 over every object.
- Normalisation is only attempted when every differing section is `.comment` or a debug section.
- When it is attempted, the final decision is again a whole-file hash after `objcopy`.
- A header-only difference therefore lands in `differs`.

The weaknesses are about trust boundaries and robustness around that core.
- Replay trusts files in the writable work directory that the untrusted build can reach. It does not re-verify them against the manifest.
- Several comparisons and limits are weaker than their documentation says.
- Errors that come from untrusted upstream content abort a whole `--all` run instead of becoming a row.

There are no BLOCKER findings. WR-01 to WR-05 are the ones I would fix before relying on the evidence for a customer-facing claim.

## Warnings

### WR-01: Capture aborts the whole step on errors that come from untrusted upstream content

**File:** `crates/mt-capture/src/testbed.rs:436-442`, `crates/mt-capture/src/capture.rs:981`, `crates/mt-capture/src/capture.rs:752`, `crates/mt-capture/src/capture.rs:1150`
**Issue:** The module contract (`testbed.rs:6-9`) is "a failing build, gate or comparison is a row, never an error". `run_capture` only turns `CaptureError::Fetch` into a row. Every other error returns from `run_capture`, so the step exits 2, writes no summary, and skips every later target. Several of those errors are driven by upstream data:
- `copy_tree(&cache.tree, &layout.src)` returns `EscapingSymlink`, `SpecialFile` or `Read` for a pinned tree. A tree with an absolute or escaping symlink is accepted by `fetch` (the digest hashes link text) and then kills the capture of every target.
- `timestamp_entries` does `fs::read` of each entry's source. A source that the build deleted or generated and removed raises `CaptureError::Io`.
- `gates` does `fs::read(&output)` per entry, and `list_files` fails on non-UTF-8 names the build creates.

`run_replay` handles the equivalent `ReplayError::Tree` as a `not_proven` row, so the two steps are inconsistent. The CI step would be red either way, but a single bad target hides the verdicts of the other 15 and leaves no `summary-capture.json` for the annotations.
**Fix:** Treat tree and I/O failures from a specific target as that target's failure:
```rust
Err(error @ (CaptureError::Tree(_) | CaptureError::Io { .. } | CaptureError::Cc(_))) => {
    summary.add(&row, &format!("capture_failed: {error}"), false);
    continue;
}
Err(error) => return Err(error.into()),   // InvalidInput, Runner, Json stay fatal
```
Alternatively, record the failure as a cause inside `capture()` so a `capture.json` with verdict `capture_failed` is still written.

### WR-02: Replay does not tie the capture report to the manifest pin; `source.commit` is used unvalidated as a path component

**File:** `crates/mt-capture/src/testbed.rs:537-592` (line 572), `crates/mt-capture/src/testbed.rs:655`
**Issue:** `run_replay` calls `verify_cached(.., id, target)`, which verifies the cache of the commit in the current manifest. `plan_of` then builds `pristine_src` from `cache_paths(&work_dir, id, &captured.source.commit).tree`. `captured.source.commit` comes from `capture.json` in the work directory and is never compared with the manifest's commit or tree digest. Two consequences:
- If the manifest pin was bumped after the capture, replay compares against a stale capture and an older cache tree that was not re-verified in this run. The old tree may still exist, so replay reports `identical` for evidence that no longer corresponds to the committed manifest.
- `commit` is joined into a filesystem path with no `is_hex(.., 40)` check. A modified `capture.json` can make `pristine_src` point anywhere (`../..`). Only `copy_tree` reads from it, but the verified-cache guarantee is lost.
**Fix:** In `plan_of`, require the report to match the pin and the tools before building the plan:
```rust
let (pin_commit, pin_tree) = pin_of(target);          // expose from fetch.rs
if captured.source.commit != pin_commit || captured.source.tree_sha256 != pin_tree {
    return Err("the capture report was made from a different source pin".into());
}
```
Also compare `captured.target`, `captured.config` and `captured.tools` against the current pins.

### WR-03: Normalised comparison reads the original object from the work directory without checking it against the reported hash

**File:** `crates/mt-capture/src/replay.rs:426-476`, `crates/mt-capture/src/replay.rs:495-506`
**Issue:** The first comparison uses `plan.original_objects` (sha256 values from `capture.json`). When the hashes differ, `compare_differing` reads `original_objects_dir/<key>` and uses those bytes both for `differing_sections` and for the `objcopy` run. It never checks that `sha256(original_bytes) == original`. `IdenticalAfterNormalisation` can therefore be produced from a file that is not the object whose hash the report states, and `normalised_original` is the hash of an unverified file. The same holds for a stale or substituted copy in `capture/<id>/<config>/objects`.
**Fix:** Before using the bytes:
```rust
if sha256_hex(&original_bytes) != original {
    return Ok(unchanged(differs(original, replay_hash, Vec::new(),
        Some("the stored original object does not match its reported sha256".to_owned()))));
}
```

### WR-04: Untrusted upstream build code runs with write access to the evidence directories and with no resource limits

**File:** `.github/workflows/testbed.yml:152-160` (run_offline), `:140-148` (run_networked), `:164-171` (image_tool)
**Issue:** `capture` executes upstream `Makefile`s and `CMakeLists.txt` as children of `mt`, in the same container, as the same uid, with `-v work:/work` and `-v out:/out` writable. The build can therefore rewrite, for targets captured earlier in the same `--all` run:
- `/work/capture/<id>/<cfg>/capture.json` and `.../objects/*`.
- `/work/capture/<id>/<cfg>/compile_commands.json`.
- The `/out` reports.

`replay` later trusts these. Its only cross-check is `canonical_sha256` in `capture.json` against `compile_commands.json`, and both files are in the same writable place. Only the fetch cache is re-verified by tree hash. `--network none`, `--read-only`, `--cap-drop ALL` and `no-new-privileges` are good. There is no `--pids-limit`, `--memory`, `--cpus` or `--ulimit`, so a runaway upstream build or a fork loop can exhaust the runner. The 45-minute job timeout is the only bound.
**Fix:**
- Keep the capture artifacts the build can reach out of reach of later evidence. Hash `capture.json` and the object copies into a file under `/out` after each configuration, and have `replay` and the determinism step compare against it.
- Alternatively, run each build step in a separate `docker run` that mounts only `/work/run/<id>/<cfg>` writable.
- Add `--pids-limit`, `--memory` and `--cpus` to the three helpers, and add the matching `needle`s to `build_02_workflow_runs_capture_and_replay_without_network`.

### WR-05: The determinism comparison fails open when a report cannot be normalised

**File:** `.github/workflows/testbed.yml:379-403`
**Issue:** `norm()` is `jq -S '...' "$1"`, used as `cmp -s <(norm "$a/$rel") <(norm "$b/$rel")`. If `jq` fails on both files (invalid JSON, truncated file), both process substitutions are empty and `cmp -s` reports equal. `set +e` is active, so the step never sees the failure. The job then prints "result: identical". The `compared -eq 0` guard only catches an empty file list. The same step also lists files with `find ... | sort` on both sides but never checks that each file is non-empty.
**Fix:**
```bash
norm() { jq -S '...' "$1" || echo "JQ_FAILED $1"; }
# or compare sizes and fail explicitly:
na=$(norm "$a/$rel") || { echo "cannot parse: $a/$rel" >> "$problems"; continue; }
nb=$(norm "$b/$rel") || { echo "cannot parse: $b/$rel" >> "$problems"; continue; }
[ "$na" = "$nb" ] || echo "differs: $rel" >> "$problems"
```

### WR-06: The tree digest is ambiguous for file names that contain a newline

**File:** `crates/mt-capture/src/treehash.rs:83-107`, `:118-120`
**Issue:** The digest is `sha256(rows.concat())` with rows `"<mode> <sha256> <relpath>\n"`. `relpath` may contain `\n` (valid UTF-8, accepted by `relative_path`). A tree with files `p` and `q` and a tree with one file named `"p\n100644 <hash(q)> q"` and the content of `p` produce the same concatenated bytes, so the same digest. Git commit pins and archive digests bound the practical exposure, but this digest is the layer meant to catch substitution, and `bed_01_tree_hash.rs` has no case for it.
**Fix:** Reject control characters (at least `\n`) in `relative_path`:
```rust
if relpath.chars().any(char::is_control) {
    return Err(TreeHashError::NonUtf8 { path: path.to_path_buf() }); // or a dedicated variant
}
```
Add a `bed_01_tree_hash_rejects_newline_in_name` test.

### WR-07: The documented memory bound for zip seed sources does not hold

**File:** `crates/mt-capture/src/seeding.rs:40-43`, `:359-380`, `crates/mt-seeds/src/zipsrc.rs:69-103`
**Issue:** `MAX_ZIP_ARCHIVE_BYTES` (2 MiB) is justified as "the archive size is what bounds the memory a pinned archive can take". It bounds the number of members, not the total decompressed size. Each kept member can be up to `max_seed_bytes` (64 KiB default) and all members are held in `ZipResult.members`, then cloned into `Candidate`s before `select` runs. A 2 MiB deflate archive with thousands of highly compressible 64 KiB members (or overlapping local headers) decompresses to gigabytes. T-02-32 is therefore not mitigated for a hostile archive in the pinned tree.
**Fix:** Pass a running budget into `zip_members`, for example `max_total_bytes` (or a few multiples of it), and stop reading further members, counting them as `too_large`, once it is exceeded. Also cap `archive.len()` (for example 10 000 members).

### WR-08: The drop-log floor and the evidence fields are not enforced where the evidence is produced

**File:** `crates/mt-capture/src/droplog.rs:26-27`, `:231-309`, `crates/mt-capture/src/testbed.rs:181-186`
**Issue:**
- `MIN_RETAINED` is only checked by a unit test on the committed `dropped.toml`. `StepSummary::add_dropped` deliberately leaves `passed` untouched, so a run where every target is dropped, or the manifest is emptied, prints `result: passed` and `mt testbed` exits 0. The D-11 floor is not visible to the CI step that produces the evidence.
- `DropLog::validate` does not require `differing_objects` or `differing_sections` when `cause = differs`. The module doc and `dropped.toml` header say they record "the objects and sections that differed". A `differs` entry with both empty is accepted. The checks do not cover `ci_run` either: it can be any non-zero number, and nothing ties it to the run that produced the failure.
**Fix:**
- In `load_inputs` (or in each `run_*`) return `TestbedError::DropLog` when `drops.retained_targets(&manifest).len() < MIN_RETAINED`.
- In `validate`, add:
```rust
if entry.cause == DropCause::Differs
    && (entry.differing_objects.is_empty() || entry.differing_sections.is_empty())
{
    issues.push(format!("{at}: cause differs needs differing_objects and differing_sections"));
}
```

### WR-09: Archive extraction relies only on the archive digest; the extracted tree is not checked for escaping links

**File:** `crates/mt-capture/src/fetch.rs:592-610`
**Issue:** `tar -xjf archive --strip-components=N --no-same-owner -C partial` extracts without further constraint. `check_tree` hashes `partial`, which cannot see anything written outside it. A member that is a symlink to an absolute path or to `..`, followed by a member written through it, is not detected by the tree digest. `copy_tree` has the escaping-symlink logic but it only runs later, on the copy in the capture. Today the archive digest is the only defence, and the download runs in the networked container with `/work` writable.
**Fix:** Pass `--no-same-permissions --no-overwrite-dir` and verify the result before hashing, for example by running `copy_tree(partial, scratch)` or exposing `link_stays_in_tree` plus `check_links_physically` over `partial`, and failing the fetch with `EscapingSymlink`. Remove the archive on failure, as is done now.

### WR-10: `is_cmake_product` is broader than its documentation and masks a known non-determinism

**File:** `crates/mt-capture/src/capture.rs:86-100`
**Issue:** I checked the function against its doc comment:
- The suffix test is `relative.ends_with(".o" | ".obj" | ".a" | ".so")` on the whole path, at any depth. Any generated configure-time file whose name ends in one of those suffixes, including a generated input or a data file, is silently left out of `configure_files` and therefore out of the "has changed since the capture" check in `changed_configure_file`.
- `CMakeFiles/Makefile2` is excluded by exact match because it differed between two plain `cmake` runs. The comment says "measured". The root cause is unresolved (pointer-order output). The exclusion is safe for replay, which does not run `make`. It also removes the file from the determinism evidence, which is exactly the content the second capture was meant to compare. The step comment at `testbed.yml:354-361` lists only two exclusions and does not mention this one.
- The probe test accepts any file under `CMakeFiles/` whose name starts with `CMakeDetermineCompilerABI_` and ends with `.bin`, not only the `<version>/` directory the doc names.

None of this makes a wrong pass, but the exclusion set is wider than the stated intent.
**Fix:** Match objects and archives by declared output instead of by suffix: skip a path only if it is a key of `report.objects` (already done) or ends in `.o`/`.obj` under `CMakeFiles/`, `*.a` directly named by a CMake target, and so on. Add a test that pins the exact set, including `sub/CMakeFiles/Makefile2` being hashed. State the `Makefile2` exclusion in the workflow's determinism comment, and add a determinism check on the sorted prerequisite lines of `Makefile2` so the file is not unobserved.

## Info

### IN-01: Capture and entry tests pass without running when the Arm tools are missing

**File:** `crates/mt-capture/tests/support/mod.rs:158-175`, `tests/build_01_capture_drivers.rs:428,491,633`, `tests/bed_01_entries.rs:406,446`
**Issue:** `local_arm_tools()` returns `None` and the test returns early, printing "skipping" to stderr, which `cargo test` hides. On a host without `arm-none-eabi-gcc` or Bear these tests are reported as passed. Only the CI image proves them.
**Fix:** Mark them `#[ignore]` with a reason, or fail when `MT_REQUIRE_ARM_TOOLS=1` is set (set it in CI) so a missing tool cannot show as green.

### IN-02: The manifest allows a configuration name that capture then rejects

**File:** `crates/mt-capture/src/manifest.rs:885-890`, `crates/mt-capture/src/capture.rs:1285`, `crates/mt-capture/src/fetch.rs:208-214`
**Issue:** `is_config_name` accepts `[a-z0-9-]+`, including a leading `-`. `capture()` requires `is_safe_component`, which rejects a leading `-` or `.`. `mt testbed validate` passes and `capture` aborts with `InvalidInput` (exit 2, see WR-01).
**Fix:** Make `is_config_name` reject a leading `-` so both agree.

### IN-03: `mt testbed validate` does not validate the drop log

**File:** `crates/mt-capture/src/testbed.rs:313-327`, `crates/mt-cli/src/main.rs:375`
**Issue:** `run_validate` checks only the manifest. A malformed or inconsistent `dropped.toml` first fails in the fetch step with exit 2. The first CI step reports "valid" for a state the next step refuses.
**Fix:** Call `DropLog::load_from_testbed_dir(manifest_path.parent())` and `validate(&manifest)` in `run_validate` and add a `drop log` row.

### IN-04: The truncated flag of captured streams is ignored

**File:** `crates/mt-capture/src/entries.rs:417-424`, `crates/mt-capture/src/capture.rs:888-899`
**Issue:** Callers test `complete` but not `truncated`. clang's AST dump or `readelf` output over the 16 MiB cap is parsed as it was stored. A cut inside a JSON document fails closed. A cut exactly between documents would silently drop declarations, and `signature_matches` is computed with `all()` over fewer declarations.
**Fix:** Treat `stdout.truncated` as `Run::NotProven` in `run_clang` and in `Session::exec`.

### IN-05: Stale version in a code comment

**File:** `crates/mt-capture/src/capture.rs:629`
**Issue:** The comment says the variable "is not honoured by CMake 3.28". The pinned CMake is 3.25.1 (`container/pins.toml`).
**Fix:** Correct the version or drop it from the comment.

### IN-06: Archive download has no size bound and is read fully into memory

**File:** `crates/mt-capture/src/fetch.rs:568-591`, `crates/mt-toolchain/src/fetch.rs:50-67`
**Issue:** curl writes to a file with `-o`. There is no `--max-filesize`, and the digest check does `fs::read(&download)` for the whole file. Only the 600 s timeout bounds disk and memory in the networked container.
**Fix:** Add `--max-filesize <N>` to the archive fetch and hash the file in streaming chunks.

### IN-07: `isotp` is dropped as a whole target because of the probe design, not a defect of the target

**File:** `testbed/dropped.toml:21-26`
**Issue:** The log records, correctly and honestly, that `isotp.h` does not compile alone and that the entry check includes only the declaring header. That is a limitation of the check (a header-only probe), recorded as a target failure. One of 16 targets, and one of 15 retained, is lost to a harness limitation. This is allowed by D-11. It is worth tracking as a harness backlog item because it lowers the denominator of the Core Value metric.
**Fix:** Consider probing with the defining translation unit's own `-include` or `#include` order for entries whose header is not self-contained, then re-run `isotp`.

### IN-08: `fetch` accepts `file://` URLs regardless of the allowed protocols

**File:** `crates/mt-capture/src/fetch.rs:220-228`, `crates/mt-toolchain/src/fetch.rs:50-67`
**Issue:** `check_url` allows `file://`, and `curl_argv` always passes `--proto =https,file`. `FetchContext.allowed_protocols` only controls `GIT_ALLOW_PROTOCOL`. For an archive source, `file://` is therefore reachable in production code through the API, even though `Manifest::validate` rejects it (`check_https`). The CLI path is safe, so this is defence in depth.
**Fix:** Reject `file://` in `check_url` unless `ctx.allowed_protocols` contains `file`, and build the curl `--proto` value from the same setting.

---

_Reviewed: 2026-10-13_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
