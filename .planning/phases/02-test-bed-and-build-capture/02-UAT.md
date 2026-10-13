---
status: diagnosed
phase: 02-test-bed-and-build-capture
source: [02-VERIFICATION.md]
started: 2026-10-13T18:40:00Z
updated: 2026-10-13T19:10:00Z
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

## Summary

total: 5
passed: 1
issues: 4
pending: 0
skipped: 0
blocked: 0

## Gaps

- gap_id: G-02-1
  truth: "A target whose header is not self-contained in C still passes the entry check when its defining TU includes the header after its prerequisites; isotp is retained"
  status: failed
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
  status: failed
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
  status: failed
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
  status: failed
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
