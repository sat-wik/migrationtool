---
status: testing
phase: 02-test-bed-and-build-capture
source: [02-VERIFICATION.md]
started: 2026-10-13T18:40:00Z
updated: 2026-10-13T18:40:00Z
---

## Current Test

number: 1
name: Review the two drops (isotp whole target, paho/server variant) and confirm they are acceptable under D-11
expected: |
  Founder accepts the drops as logged, or reopens them (for example by letting the entry probe use the defining TU's include order so isotp.h need not be self-contained, review IN-07)
awaiting: user response

## Tests

### 1. Review the two drops (isotp whole target, paho/server variant) and confirm they are acceptable under D-11
expected: Founder accepts the drops as logged, or reopens them (for example by letting the entry probe use the defining TU's include order so isotp.h need not be self-contained, review IN-07)
result: [pending]

### 2. Review the CMakeFiles/Makefile2 exclusion from configure-time hashes (capture.rs is_cmake_product, review WR-10)
expected: Founder accepts that one CMake driver file is unobserved by the configure-hash and determinism evidence, or asks for a normalised check of its prerequisite lines
result: [pending]

### 3. Review the 97 committed hand-made seeds (coremqtt 10, libfdt 16, lz4 20, microcoap 16, paho 17, zlib 18) plus the 19 uncommitted isotp hand-made files
expected: Hand-made seeds follow their intended wire formats; founder accepts, edits or rejects each set
result: [pending]

### 4. Inspect the extracted httpparser (200 c_string), nanopb (190 c_string) and qcbor (102 c_string) seeds for strings that are not parser inputs
expected: Narrow the c_string sources if error messages, identifiers or other non-input strings are present
result: [pending]

### 5. Decide whether the zip seed backend (zip 9.0.2 with deflate-flate2-zlib-rs) is acceptable for reading untrusted fuzz archives
expected: Accept, or switch to an all-safe-Rust decompressor (flate2 miniz_oxide) and add the member-size budget from review WR-07
result: [pending]

## Summary

total: 5
passed: 0
issues: 0
pending: 5
skipped: 0
blocked: 0

## Gaps
