---
status: complete
phase: 03-equivalence-contract-and-test-vector-schema
source: [03-VERIFICATION.md]
started: 2026-10-30T02:00:00Z
updated: 2026-10-30T14:22:42.114Z
---

## Current Test

[testing complete]

## Tests

### 1. Read docs/specs/equivalence-contract.md (0.1.0) against 03-CONTEXT.md D-05..D-30
expected: Every sentence of the contract matches the decision it restates; none adds or drops a rule
result: pass

### 2. Decide whether LZ4_decompress_safe's overshoot is acceptable under D-07 (whole output buffer recorded after a 0xA5 sentinel prefill)
expected: A recorded founder decision: keep D-07 as is, or amend it (the question has been open since 03-04 and is repeated untouched in 03-05..03-08)
result: pass
note: "Founder decision: keep D-07 as is. The whole output buffer is recorded after the 0xA5 sentinel prefill, so LZ4_decompress_safe's overshoot is captured as boundary-visible behaviour the Rust must match."

### 3. Decide how function-local statics named name.N (for example version.0) are keyed in lib_state.globals
expected: Either relax the C-identifier rule on the symbol part of a globals key in mt-vectors, or accept that a changed function-local static fails the run. No committed target triggers it today.
result: pass
note: "Founder decision: keep the C-identifier rule on the globals key for now; a changed function-local static (name.N) fails the run loudly. No committed target triggers it; revisit the key grammar (D-06, costly to reverse) only when one does."

### 4. Review 03-05's edit of its own Task 3 verify command (added `| grep -v ' no_roles'`, commit 7878588)
expected: Agreement that the original command could not pass (it also matched nimble's no_roles rows) and that the corrected command still checks the layout cross-check
result: pass
note: "Agreed: the original command also matched nimble's no_roles rows and could not pass; the corrected command still checks the layout cross-check."

### 5. Decide on isotp/isotp_on_can_message, logged as link_unresolved instead of declared
expected: Either accept the log entry, or approve a reviewed C shim for the three user hooks (isotp_user_debug, isotp_user_get_ms, isotp_user_send_can). D-11 forbids per-target hand-written C, so a shim needs a decision.
result: pass
note: "Founder decision: accept the link_unresolved log entry for isotp/isotp_on_can_message; no per-target C shim (D-11 stands). Floor is met without it (15 drivable, floor 10)."

### 6. Judge the committed sample rule (smallest seed per target)
expected: 7 of the 15 sample inputs are 1 byte and tinycbor's is empty. Decide whether that is an adequate offline reference or the rule should pick a richer seed.
result: pass
note: "Founder decision: smallest-seed sample is adequate as a deterministic offline reference; the full corpus in CI covers the accepting paths."

### 7. Confirm the has_ub label `timeout` (rc -9) for the timeout outcome
expected: Agreement that a timeout is labelled has_ub `timeout` so it is excluded from comparison and never counted as a pass, or a different label.
result: pass
note: "Confirmed: timeout is labelled has_ub `timeout` with rc -9, excluded from comparison and never counted as a pass."

### 8. Accept QEMU 10.0.2 (bookworm-backports, 1:10.0.2+ds-2+deb13u1~bpo12+1) in place of D-31's 8.2.x
expected: Explicit acceptance that D-31's version number was not followed. The cause D-31 names (commpage on M-profile) is gone: `qemu-arm -cpu cortex-m4` starts and every vectors job run is green.
result: pass
note: "Founder accepts QEMU 1:10.0.2+ds-2+deb13u1~bpo12+1 in place of D-31's 8.2.x: the M-profile commpage cause D-31 names is gone and every vectors job is green."

## Summary

total: 8
passed: 8
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps

[none]
