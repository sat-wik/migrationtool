---
status: complete
phase: 01-pinned-toolchain-container
source: [01-VERIFICATION.md]
started: 2026-10-10T03:40:31Z
updated: 2026-10-11T16:12:07.026Z
---

## Current Test

[testing complete]

## Tests

### 1. Pull the verified image on the founder's Apple Silicon Mac and run the toolchain check inside it
expected: Pull succeeds; `mt toolchain check --pins /opt/mt/pins.toml` prints every row OK (hayroll and kani not_installed) and exits 0; digest matches container/README.md "Current image" (sha256:0f3b392427d2588e6ba0e3da2067bc503f40e6749b48bc6553832c27e96e518d).
result: pass
evidence: "Founder ran it on Apple Silicon (Docker Desktop, linux/amd64 emulation): every row OK, hayroll and kani not_installed, rustc_bitcode 1.72.1 LLVM 16.0.5, 'result: OK', exit code 0"

## Summary

total: 1
passed: 1
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps
