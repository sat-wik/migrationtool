---
status: complete
phase: 01-pinned-toolchain-container
source: [01-VERIFICATION.md, 01-REVIEW-FIX.md, ../../quick/261011-uhy-phase-1-residual-hardening-wr-01-wr-04-w/261011-uhy-SUMMARY.md]
started: 2026-10-12T00:00:00Z
updated: 2026-10-12T02:49:36.435Z
---

## Current Test

[testing complete]

## Tests

### 1. Pull the new verified image on the founder's Apple Silicon Mac and run the toolchain check inside it
expected: Pull of ghcr.io/sat-wik/migrationtool-toolchain@sha256:acdfe312b21c6af19bc61750cf454f3914f482b3bfb9a2ca42d3e6b5f7758b5c succeeds; `mt toolchain check --pins /opt/mt/pins.toml` prints every row OK (hayroll and kani not_installed, rustc_bitcode 1.72.1 LLVM 16.0.5), ends with "result: OK" and exits 0; digest matches container/README.md "Current image". (Restarted after the code-review fixes and residual hardening published a new image; the previous pass on sha256:0f3b39… is superseded.)
result: pass
evidence: "Founder ran it on Apple Silicon (Docker Desktop, linux/amd64 emulation) against sha256:acdfe312…: mt from mt-bundle of run 38182234483, pins.toml fetched from the branch (sha256 5b17f39b…); every row OK, hayroll and kani not_installed, rustc_bitcode 1.72.1 LLVM 16.0.5, D-15 provisional line, result: OK, exit code 0"

## Summary

total: 1
passed: 1
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps

[none yet]
