---
status: testing
phase: 01-pinned-toolchain-container
source: [01-VERIFICATION.md, 01-REVIEW-FIX.md, ../../quick/261011-uhy-phase-1-residual-hardening-wr-01-wr-04-w/261011-uhy-SUMMARY.md]
started: 2026-10-12T00:00:00Z
updated: 2026-10-12T03:41:32.503Z
---

## Current Test
<!-- OVERWRITE each test - shows where we are -->

number: 1
name: Pull the new verified image on the founder's Apple Silicon Mac and run the toolchain check inside it
expected: |
  Pull of ghcr.io/sat-wik/migrationtool-toolchain@sha256:0f5a5d2594b0405931ad7e85d5c6d2f5e3f2d5099d769059870817ee50e152ee succeeds; with mt from the mt-bundle of container run 38199978785 and container/pins.toml at sha256 642a1ed1…, `mt toolchain check --pins /opt/mt/pins.toml` prints every row OK (hayroll and kani not_installed, rustc_bitcode 1.72.1 LLVM 16.0.5), ends with "result: OK" and exits 0; digest matches container/README.md "Current image". (Restarted again: WR-08..WR-12 fixes published a new image; the pass on sha256:acdfe312… is superseded.)
awaiting: user response

## Tests

### 1. Pull the new verified image on the founder's Apple Silicon Mac and run the toolchain check inside it
expected: Pull of ghcr.io/sat-wik/migrationtool-toolchain@sha256:0f5a5d2594b0405931ad7e85d5c6d2f5e3f2d5099d769059870817ee50e152ee succeeds; with mt from the mt-bundle of container run 38199978785 and container/pins.toml at sha256 642a1ed1…, `mt toolchain check --pins /opt/mt/pins.toml` prints every row OK (hayroll and kani not_installed, rustc_bitcode 1.72.1 LLVM 16.0.5), ends with "result: OK" and exits 0; digest matches container/README.md "Current image". (Restarted again: WR-08..WR-12 fixes published a new image; the pass on sha256:acdfe312… is superseded.)
result: [pending]
previous_evidence: "\"Founder ran it on Apple Silicon (Docker Desktop, linux/amd64 emulation) against sha256:acdfe312…: mt from mt-bundle of run 38182234483, pins.toml fetched from the branch (sha256 5b17f39b…); every row OK, hayroll and kani not_installed, rustc_bitcode 1.72.1 LLVM 16.0.5, D-15 provisional line, result: OK, exit code 0\""

## Summary

total: 1
passed: 0
issues: 0
pending: 1
skipped: 0
blocked: 0

## Gaps

[none yet]
