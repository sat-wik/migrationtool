---
status: testing
phase: 01-pinned-toolchain-container
source: [01-VERIFICATION.md]
started: 2026-10-10T03:40:31Z
updated: 2026-10-10T03:40:31Z
---

## Current Test

number: 1
name: Pull the verified image on the founder's Apple Silicon Mac and run the toolchain check inside it
expected: |
  `docker pull --platform linux/amd64 ghcr.io/sat-wik/migrationtool-toolchain@sha256:0f3b392427d2588e6ba0e3da2067bc503f40e6749b48bc6553832c27e96e518d`
  succeeds (anonymously, or after `docker login ghcr.io`). Running `mt toolchain check --pins /opt/mt/pins.toml`
  inside it as container/README.md describes (mt and pins.toml mounted read-only, --network none, unprivileged user)
  prints every row OK, hayroll and kani as not_installed, and exits 0. The digest equals the one under
  "Current image" in container/README.md.
awaiting: user response

## Tests

### 1. Pull the verified image on the founder's Apple Silicon Mac and run the toolchain check inside it
expected: Pull succeeds; `mt toolchain check --pins /opt/mt/pins.toml` prints every row OK (hayroll and kani not_installed) and exits 0; digest matches container/README.md "Current image" (sha256:0f3b392427d2588e6ba0e3da2067bc503f40e6749b48bc6553832c27e96e518d).
result: [pending]

## Summary

total: 1
passed: 0
issues: 0
pending: 1
skipped: 0
blocked: 0

## Gaps
