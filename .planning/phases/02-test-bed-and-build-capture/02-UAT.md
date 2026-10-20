---
status: diagnosed
phase: 02-test-bed-and-build-capture
source: [02-VERIFICATION.md (2026-10-13 and 2026-10-20 re-verification)]
started: 2026-10-13T18:40:00Z
updated: 2026-10-20T03:00:00Z
---

## Current Test

[testing complete]

## Tests

### 1. Review the two drops (isotp whole target, paho/server variant) and confirm they are acceptable under D-11
expected: Founder accepts the drops as logged, or reopens them (for example by letting the entry probe use the defining TU's include order so isotp.h need not be self-contained, review IN-07)
result: issue
reported: "Reopen isotp by making the entry probe use the defining TU's include order, so that isotp.h need not be self-contained. Accept the paho/server drop as logged."
severity: major

### 2. Review the CMakeFiles/Makefile2 exclusion from configure-time hashes (capture.rs is_cmake_product, review WR-10)
expected: Founder accepts that one CMake driver file is unobserved by the configure-hash and determinism evidence, or asks for a normalised check of its prerequisite lines
result: issue
reported: "Replace the Makefile2 exclusion with a normalised check that hashes the file with its prerequisite lines sorted. List the exclusion in the testbed.yml determinism comment."
severity: major

### 3. Review the 97 committed hand-made seeds (coremqtt 10, libfdt 16, lz4 20, microcoap 16, paho 17, zlib 18) plus the 19 uncommitted isotp hand-made files
expected: Hand-made seeds follow their intended wire formats; founder accepts, edits or rejects each set
result: pass
note: "Accepted as proposals. To be reviewed once the Phase 3 harness shows which inputs exercise real code paths."

### 4. Inspect the extracted httpparser (200 c_string), nanopb (190 c_string) and qcbor (102 c_string) seeds for strings that are not parser inputs
expected: Narrow the c_string sources if error messages, identifiers or other non-input strings are present
result: issue
reported: "Narrow the declared seed sources for httpparser, nanopb and qcbor to the test arrays and fixture files that hold parser inputs."
severity: major

### 5. Decide whether the zip seed backend (zip 9.0.2 with deflate-flate2-zlib-rs) is acceptable for reading untrusted fuzz archives
expected: Accept, or switch to an all-safe-Rust decompressor (flate2 miniz_oxide) and add the member-size budget from review WR-07
result: issue
reported: "Switch the zip backend to flate2 with miniz_oxide (all safe Rust) and add a total decompressed-size budget (review WR-07)."
severity: major

### 6. Triage review warnings WR-11 and WR-12 (the record stage trust of the build record): fix before ship, or accept and log
expected: Either a follow-up plan makes the record stage refuse a record with built:false and no failure cause (verdict depends on the gates) and compare stage/toolchain.cmake with the recomputed text, or the founder accepts both as residual risk next to AR-02-03
result: issue
reported: "Fix both in a small gap plan: the record stage refuses a record with built:false and no failure cause and makes the verdict depend on the gates (WR-11), and it compares stage/toolchain.cmake with the recomputed text (WR-12)."
severity: major

### 7. Register or explicitly accept the unregistered SECURITY.md observation: replay passes the recorded GCC driver flags to the pinned gcc
expected: Founder registers it as a medium threat (a later plan rejects -wrapper, -B<dir>, -specs and plugin options in the compile-command gate) or accepts it, logged as an accepted risk
result: issue
reported: "Register it as a medium threat, and make the compile-command gate reject GCC driver options that execute helpers (-wrapper, -B<dir>, -specs, plugins) in the same small plan."
severity: major

### 8. Confirm accepted risk AR-02-03 (a pair own build writes its build record) with WR-11 in view
expected: Founder confirms AR-02-03 stands with its rationale amended to say what a forged record can and cannot do, or asks for the WR-11 fix first
result: issue
reported: "Keep AR-02-03, and amend its rationale once WR-11 is fixed so it states what a forged record can and cannot do."
severity: minor

## Summary

total: 8
passed: 1
issues: 7
pending: 0
skipped: 0
blocked: 0

## Gaps

- gap_id: G-02-1
  truth: "A target whose header is not self-contained in C still passes the entry check when its defining TU includes the header after its prerequisites; isotp is retained"
  status: resolved
  resolved_by: 02-11-PLAN.md
  resolved_at: 2026-10-20
  reason: "User reported: Reopen isotp by making the entry probe use the defining TU's include order, so that isotp.h need not be self-contained. Accept the paho/server drop as logged."
  severity: major
  test: 1
  root_cause: "The entry probe (probe_source in crates/mt-capture/src/entries.rs) includes the entry header on its own. isotp.h uses uint8_t but includes <stdint.h> only under __cplusplus, so the probe fails. isotp.c includes <stdint.h> first and compiles. This is a harness limitation, not a target defect (review IN-07)."
  artifacts:
    - crates/mt-capture/src/entries.rs
    - testbed/dropped.toml
    - testbed/DROPPED.md
    - testbed/manifest.toml
  missing:
    - "The probe reproduces the include prefix of the defining TU before the entry header, instead of including the header alone. A RED/GREEN test covers it, and a self-contained header still works."
    - "isotp is removed from dropped.toml and DROPPED.md, and its seed corpus is generated. The paho/server drop stays."
  debug_session: ""

- gap_id: G-02-2
  truth: "CMakeFiles/Makefile2 is covered by the configure-time evidence in a normalised form (prerequisite lines sorted), and the determinism comment lists every exclusion"
  status: resolved
  resolved_by: 02-12-PLAN.md
  resolved_at: 2026-10-20
  reason: "User reported: Replace the Makefile2 exclusion with a normalised check that hashes the file with its prerequisite lines sorted. List the exclusion in the testbed.yml determinism comment."
  severity: major
  test: 2
  root_cause: "is_cmake_product in capture.rs excludes CMakeFiles/Makefile2 completely, because CMake writes equal-rank prerequisite lines in pointer order (the zcbor app.dir/all lines). Real content therefore drops out of the evidence. The testbed.yml determinism comment does not list the exclusion. Review WR-10 also notes that the suffix match is wider than documented."
  artifacts:
    - crates/mt-capture/src/capture.rs
    - .github/workflows/testbed.yml
  missing:
    - "A normalised hash of Makefile2, with prerequisite lines sorted, replaces the exclusion. RED/GREEN tests cover it."
    - "is_cmake_product matches exactly the documented paths."
    - "The testbed.yml determinism comment lists every excluded field and file."
  debug_session: ""

- gap_id: G-02-3
  truth: "Extracted c_string seeds for httpparser, nanopb and qcbor come only from declared test arrays or fixture files that hold parser inputs"
  status: resolved
  resolved_by: 02-15-PLAN.md
  resolved_at: 2026-10-20
  reason: "User reported: Narrow the declared seed sources for httpparser, nanopb and qcbor to the test arrays and fixture files that hold parser inputs."
  severity: major
  test: 4
  root_cause: "The seed_source entries in testbed/manifest.toml point c_string extraction at whole test files. Those files also contain error messages and identifiers (02-08 SUMMARY D6)."
  artifacts:
    - testbed/manifest.toml
    - testbed/seeds/httpparser/
    - testbed/seeds/nanopb/
    - testbed/seeds/qcbor/
  missing:
    - "The seed_source declarations for the three targets are narrowed to input-bearing arrays or fixture files."
    - "The corpora are regenerated in the image and seeds --check passes."
  debug_session: ""

- gap_id: G-02-4
  truth: "Zip seed sources are decompressed with an all-safe-Rust backend under a total decompressed-size budget"
  status: resolved
  resolved_by: 02-13-PLAN.md
  resolved_at: 2026-10-20
  reason: "User reported: Switch the zip backend to flate2 with miniz_oxide (all safe Rust) and add a total decompressed-size budget (review WR-07)."
  severity: major
  test: 5
  root_cause: "zip 9.0.2 is built with deflate-flate2-zlib-rs, so a dependency with unsafe code reads untrusted archives. zipsrc.rs bounds each member but not the total, so a 2 MiB archive can still decompress to gigabytes (review WR-07)."
  artifacts:
    - Cargo.toml
    - Cargo.lock
    - crates/mt-seeds/Cargo.toml
    - crates/mt-seeds/src/zipsrc.rs
    - crates/mt-capture/src/seeding.rs
  missing:
    - "The zip feature is switched to flate2 with the miniz_oxide backend, and cargo deny is clean."
    - "Each archive has a total decompressed-size budget, covered by a test."
  debug_session: ""

- gap_id: G-02-5
  truth: "The record stage never reports a pair as captured unless its gates ran and passed, and it refuses a build record whose built:false carries no failure cause; the toolchain file the build used equals the pinned toolchain text"
  status: failed
  reason: "User reported: Fix both in a small gap plan: the record stage refuses a record with built:false and no failure cause and makes the verdict depend on the gates (WR-11), and it compares stage/toolchain.cmake with the recomputed text (WR-12)."
  severity: major
  test: 6
  root_cause: "capture_record in crates/mt-capture/src/capture.rs (about lines 2019-2060) computes the verdict only from the absence of causes. A build record with built:false, no cause and no failure flags therefore skips every gate, and the pair reads 'captured' with gates.arch_ok=false, no objects and no entries (review WR-11). The recorded toolchain_file_sha256 is recomputed from the pins and never compared with the stage/toolchain.cmake file the build used (review WR-12). The capture.rs module doc's claim about what a forged record can do is therefore wrong."
  artifacts:
    - crates/mt-capture/src/capture.rs
    - crates/mt-capture/tests/build_01_capture_stages.rs
  missing:
    - "A built:false record with no failure cause is refused (not_proven or capture_failed with a named cause). A Captured verdict requires the gates to have run and passed, and a non-empty object set. RED/GREEN tests cover both."
    - "The record stage reads stage/toolchain.cmake through read_run_file and compares it byte for byte with the text recomputed from the pins. A mismatch is a named cause. A test covers it."
    - "The capture.rs module doc states correctly what a forged build record can and cannot do."
  debug_session: ""

- gap_id: G-02-6
  truth: "The compile-command gate refuses recorded GCC driver options that execute helper programs, so replay never passes them to the pinned compiler; the threat is registered in 02-SECURITY.md"
  status: failed
  reason: "User reported: Register it as a medium threat, and make the compile-command gate reject GCC driver options that execute helpers (-wrapper, -B<dir>, -specs, plugins) in the same small plan."
  severity: major
  test: 7
  root_cause: "The gate in crates/mt-capture/src/cc.rs (canonicalise / check_entry, about lines 165-175 and 250-268) checks argv[0], -c, run-root paths and -flto*/-fprofile-* only. replay.rs (about lines 275-282) passes the rest of the recorded argv unchanged to the pinned gcc, so GCC driver options that run helper programs (-wrapper, -B<dir>, -specs=, -fplugin*, -iplugindir, and similar) would execute inside the offline replay container, where /work and /out are writable. This was observed in the 2026-10-20 security audit and has no threat ID."
  artifacts:
    - crates/mt-capture/src/cc.rs
    - crates/mt-capture/tests/build_01_compile_commands.rs
    - .planning/phases/02-test-bed-and-build-capture/02-SECURITY.md
  missing:
    - "The gate rejects the helper-executing GCC driver options (at least -wrapper, -B, -specs, -fplugin, -fplugin-arg-*, -iplugindir, --sysroot pointing outside the run root if applicable) at capture and again at replay, before any spawn. A test covers each option. No retained pair's real commands are refused (CI full-bed run green)."
    - "The threat is registered in the plan threat model (next free T-02 id, medium) and verified."
  debug_session: ""

- gap_id: G-02-7
  truth: "AR-02-03's rationale states exactly what a forged build record can and cannot do after the G-02-5 fix"
  status: failed
  reason: "User reported: Keep AR-02-03, and amend its rationale once WR-11 is fixed so it states what a forged record can and cannot do."
  severity: minor
  test: 8
  root_cause: "AR-02-03 in 02-SECURITY.md says a forged record can falsify only that pair's steps, flags and before listing. Review WR-11 showed it can also make the pair read 'captured' with no gate evaluated. The acceptance was given on the narrower statement."
  artifacts:
    - .planning/phases/02-test-bed-and-build-capture/02-SECURITY.md
    - crates/mt-capture/src/capture.rs
  missing:
    - "After G-02-5, AR-02-03's rationale (and the capture.rs module doc) states what a forged record can still do and what it can no longer do, citing the tests."
  debug_session: ""

- gap_id: G-02-8
  truth: "The compile-command gate accepts only measured-good options through one operand-aware argument walk, so no spelling, carrier or operand trick reaches the compiler, assembler or linker at capture or replay; the toolchain evidence covers what configure read"
  status: failed
  reason: "Founder decision 2026-10-20 after the post-02-20 code re-review: replace the deny-list gate with an allow-list gate (one operand-aware walk shared by the -c test, derive_output and the write-path check); fold in WR-12 (configure-time toolchain check), WR-13 (-specs allowed only for names the pinned toolchain ships) and WR-14 (an unreadable toolchain file is a named cause on the pair, not an aborted step)."
  severity: blocker
  test: review
  root_cause: "02-20's gate is a deny-list of option prefixes. Review CR-01: the GCC driver rewrites long --<name> spellings to -f<name> (--plugin=, --compare-debug=, --opt-info-all=, --dump-*=, --profile-use=, --profile-dir=), which the prefixes miss. CR-02: ASSEMBLER_WRITE_PREFIXES misses GNU as single-dash and abbreviated forms (-Wa,-MD,<path>, -Wa,--M,<path>). CR-03: the compile-only test accepts a -c that is the operand of -MT, so the linker runs and -Wl, (e.g. -Map=, -plugin) is never examined. All three were reproduced with arm-none-eabi-gcc 13.2.1. WR-12: check_toolchain_file sees only the end state of stage/toolchain.cmake. WR-13: -specs is refused for every name. WR-14: a non-NotFound I/O error on the toolchain file aborts the step."
  artifacts:
    - crates/mt-capture/src/cc.rs
    - crates/mt-capture/src/capture.rs
    - crates/mt-capture/src/replay.rs
    - crates/mt-capture/tests/build_01_compile_commands.rs
    - crates/mt-capture/tests/build_02_replay.rs
    - crates/mt-capture/tests/build_01_capture_stages.rs
    - .planning/phases/02-test-bed-and-build-capture/02-SECURITY.md
  missing:
    - "One operand-aware argument walk (an option table with operand arity) is the single source for the -c test, derive_output and the write-path check, at capture and at replay before any spawn."
    - "An allow-list of options measured from the 30 retained pairs (plus depfile flags under the run root and -specs= names shipped by the pinned toolchain); any other option, including every long --<name> spelling, any -Wa,/-Xassembler/-Wl,/-Xlinker/-Wp,/-Xpreprocessor carrier piece not allow-listed, and @files, is refused with a named cause. Tests reproduce CR-01, CR-02 and CR-03 RED first."
    - "WR-12: the record stage proves the toolchain file configure read equals the pinned text (not only the end state). WR-14: an I/O error reading it is a named cause on that pair."
    - "No retained pair's real commands are refused: the 29 local captures, both CI fixtures and one full-bed CI run whose notices equal 02-20's line for line."
    - "T-02-67 and T-02-70 are re-verified and closed in 02-SECURITY.md."
  debug_session: ""
