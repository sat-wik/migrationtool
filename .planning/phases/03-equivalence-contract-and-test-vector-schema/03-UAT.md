---
status: testing
phase: 03-equivalence-contract-and-test-vector-schema
source: [03-VERIFICATION.md]
started: 2026-10-30T02:00:00Z
updated: 2026-10-30T02:00:00Z
---

## Current Test

number: 1
name: Read docs/specs/equivalence-contract.md (0.1.0) against 03-CONTEXT.md D-05..D-30
expected: |
  Every sentence of the contract matches the decision it restates; none adds or drops a rule
awaiting: user response

## Tests

### 1. Read docs/specs/equivalence-contract.md (0.1.0) against 03-CONTEXT.md D-05..D-30
expected: Every sentence of the contract matches the decision it restates; none adds or drops a rule
result: [pending]

### 2. Decide whether LZ4_decompress_safe's overshoot is acceptable under D-07 (whole output buffer recorded after a 0xA5 sentinel prefill)
expected: A recorded founder decision: keep D-07 as is, or amend it (the question has been open since 03-04 and is repeated untouched in 03-05..03-08)
result: [pending]

### 3. Decide how function-local statics named name.N (for example version.0) are keyed in lib_state.globals
expected: Either relax the C-identifier rule on the symbol part of a globals key in mt-vectors, or accept that a changed function-local static fails the run. No committed target triggers it today.
result: [pending]

### 4. Review 03-05's edit of its own Task 3 verify command (added `| grep -v ' no_roles'`, commit 7878588)
expected: Agreement that the original command could not pass (it also matched nimble's no_roles rows) and that the corrected command still checks the layout cross-check
result: [pending]

### 5. Decide on isotp/isotp_on_can_message, logged as link_unresolved instead of declared
expected: Either accept the log entry, or approve a reviewed C shim for the three user hooks (isotp_user_debug, isotp_user_get_ms, isotp_user_send_can). D-11 forbids per-target hand-written C, so a shim needs a decision.
result: [pending]

### 6. Judge the committed sample rule (smallest seed per target)
expected: 7 of the 15 sample inputs are 1 byte and tinycbor's is empty. Decide whether that is an adequate offline reference or the rule should pick a richer seed.
result: [pending]

### 7. Confirm the has_ub label `timeout` (rc -9) for the timeout outcome
expected: Agreement that a timeout is labelled has_ub `timeout` so it is excluded from comparison and never counted as a pass, or a different label.
result: [pending]

### 8. Accept QEMU 10.0.2 (bookworm-backports, 1:10.0.2+ds-2+deb13u1~bpo12+1) in place of D-31's 8.2.x
expected: Explicit acceptance that D-31's version number was not followed. The cause D-31 names (commpage on M-profile) is gone: `qemu-arm -cpu cortex-m4` starts and every vectors job run is green.
result: [pending]

## Summary

total: 8
passed: 0
issues: 0
pending: 8
skipped: 0
blocked: 0

## Gaps
