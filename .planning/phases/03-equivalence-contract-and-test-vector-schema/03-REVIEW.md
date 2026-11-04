---
phase: 03-equivalence-contract-and-test-vector-schema
reviewed: 2026-11-04T00:00:00Z
depth: standard
files_reviewed: 2
files_reviewed_list:
  - .github/workflows/testbed.yml
  - crates/mt-harness/tests/eqv_04_workflow_static.rs
findings:
  critical: 0
  warning: 1
  info: 3
  total: 4
status: issues_found
---

# Phase 3: Code Review Report (incremental, plan 03-12)

**Reviewed:** 2026-11-04
**Depth:** standard
**Scope:** incremental since 17ce931 (`git diff 17ce931 HEAD`), covering plan 03-12 only: the `safe_untar` function of the vectors job step "Unpack the captures of the testbed job" in `.github/workflows/testbed.yml`, and its behavioural table in `crates/mt-harness/tests/eqv_04_workflow_static.rs`. This replaces the review of plan 03-11 (CR-01, WR-01, WR-02, IN-01), which plan 03-12 set out to fix.
**Files Reviewed:** 2
**Status:** issues_found

## Summary

The fixes for the previous CR-01, WR-01, WR-02 and IN-01 hold. I found no blocker.

How I checked, with GNU tar 1.35 and bash 5.2.21:

- `cargo test -p mt-harness --test eqv_04_workflow_static --offline`: 14 of 14 pass.
- I extracted `safe_untar` from the workflow with awk and ran it under `set -euo pipefail`.
- **CR-01 (symlink walk).** The exact archive from the previous review (`m/n/d/p -> ../../..`, `X -> m/n`, `e -> X/d/p/..`) is now refused with "passes through a symlink of the archive". It is refused whichever way the members are ordered. The same holds when the names carry a `./` prefix, when the target carries `./` components, and when the link's own directory part passes through a link (`X/q -> ../..`).
- I also fuzzed the function: about 8,000 random archives of dirs, files and links over the names a/b/c with `..` targets, in random member order. 2,051 were accepted, and each was extracted with the real `tar -xf`. In none did any extracted symlink resolve (`realpath`) outside the destination, and nothing appeared outside the destination. The rule "never push onto or pop from a stack that names an archive symlink" keeps the logical path equal to the physical one, so the walk is exact.
- Keys and names that could break the `islink` lookup caused no escape and no mis-pairing: `@`, `*`, `]`, spaces, quotes, tab, backslash, a newline, a non-ASCII byte, a leading `-`, `~`, `:`, a control byte. GNU tar escapes these identically in `-tf` and `-tvf`.
- **WR-01 (forged owner column).** `--numeric-owner` makes the text before the name the mode, `N/N`, size, date and time, so the first ` NAME -> ` is the real one. `owner_re` additionally refuses any line whose owner column is not digits. The forged `uname="u a -> p/q/r/s/t"` archive of the previous review is now read correctly and refused with "leaves the tree" (the row `refuse_forged_owner`, on a real tar).
- **WR-02 (test table).** Every refused row whose archive can trip two checks asserts a fragment that only its own check prints. The member-name refusal says "holds a member name", the link refusals say "holds a symlink that is absolute or leaves the tree" or "holds a symlink that passes through a symlink of the archive", and the listing-shape refusals say "cannot read the target" or "list different members". The new `snapshot_outside_dest` comparison runs around every row. The one exception, two branches that print the same text, is IN-01 below.
- **IN-01 (host tools).** `assert_host_tools` now fails, with no skip path, unless `tar --version` says GNU tar and `/bin/bash` is 4.3 or later. The bash floor is slightly wrong (IN-02 below).

What remains is a coverage gap in the table (WR-01) and three low-priority items.

## Warnings

### WR-01: the table has no row for the two guard inputs that are not covered by any other row

**File:** `crates/mt-harness/tests/eqv_04_workflow_static.rs:1648-1901` (the `cases` table). The code involved is `.github/workflows/testbed.yml:1094-1107` (first pass, name normalisation) and `1135-1136` (the name's directory parts as the initial stack).
**Issue:** two paths of the guard are guarded by no row, and each is a path on which CR-01 could silently come back:

- Normalisation of member names. The first pass builds the `islink` keys with empty and `.` components skipped, and the second pass skips them again. No row has a `./`-prefixed or `a/./b` member name. If the first pass stopped skipping `.`, the key of `./X` would be `./X` and would never match the stack `X`. CR-01 would reopen for any archive made with `tar -C dir .`. I checked by hand that the current code refuses such an archive, and no test would fail if that changed. The real CI archives name `capture` and `run`, so this is not exploitable today.
- A symlink whose own name passes through another archive symlink (`X -> m/n`, then `X/q -> ../..`). The walk starts from the name's directory parts, and the link check runs while the parts are pushed. I checked that this refuses today. No row pins it, and several rows only prove that the first component is pushed.

**Fix:** add two rows to the table, both with `accepted: false` and the message "holds a symlink that passes through a symlink of the archive":

```rust
UntarCase {
    label: "refuse_alias_up_through_link_dot_names",
    archive: Archive::Ordered(vec![
        ("./m", Member::Dir),
        ("./m/n", Member::Dir),
        ("./m/n/d", Member::Dir),
        ("./m/n/d/p", Member::Link("../../..")),
        ("./X", Member::Link("m/n")),
        ("./e", Member::Link("./X/./d/p/..")),
    ]),
    accepted: false,
    message: "holds a symlink that passes through a symlink of the archive",
},
UntarCase {
    label: "refuse_name_through_link",
    archive: Archive::Ordered(vec![
        ("m", Member::Dir),
        ("m/n", Member::Dir),
        ("X", Member::Link("m/n")),
        ("X/q", Member::Link("../..")),
    ]),
    accepted: false,
    message: "holds a symlink that passes through a symlink of the archive",
},
```

I ran equivalent archives through the function by hand: both exit 1 with that message. Check that GNU tar 1.35 archives these member orders without error before committing the rows.

## Info

### IN-01: two different refusals print the same text, so a row cannot tell which one fired

**File:** `.github/workflows/testbed.yml:1123` and `1128`; rows `refuse_forged_owner_listing` and `refuse_unreadable_target` at `crates/mt-harness/tests/eqv_04_workflow_static.rs:1874-1891`.
**Issue:** the owner-column refusal and the no-match refusal print byte-identical messages ("cannot read the target of the symlink $name in the listing line: $line"). Both rows assert the same fragment. The rows do not mask each other today: each input satisfies only one of the two checks, and removing either check makes its row exit 99 (the fake tar's extraction) instead of 1. The stated aim, a message that only its own refusal prints, does not hold for this pair.
**Fix:** give the owner-column refusal its own words, for example `::error::$archive: the owner column of the symlink $name is not numeric: $line`, and assert "owner column" in `refuse_forged_owner_listing`. The two rows then fail on the same mutation for a different reason, and the message says which check fired.

### IN-02: the bash floor of the host-tool check is 4.3, but the function needs 4.4

**File:** `crates/mt-harness/tests/eqv_04_workflow_static.rs:1537-1540` (the 4.3 check); function at `.github/workflows/testbed.yml:1060-1165`.
**Issue:** the function runs under `set -u` and expands `"${parts[@]}"` and `"${tparts[@]}"`, which are empty for a link at the top level (`parts` is empty after the `unset`) and for an empty target. Bash 4.3 and earlier treat an empty array expansion under `set -u` as an unbound variable. Bash 4.4 fixed that. The same file already works around it elsewhere with `${cache[@]+"${cache[@]}"}` (line 247). On a 4.3 host the assertion would pass and the first accept row would then die with an obscure "unbound variable". I did not have bash 4.3 to confirm, so this is from the documented 4.4 behaviour change. The runner has bash 5.2, so it does not affect CI.
**Fix:** require 4.4 in `assert_host_tools` (`major > 4 || (major == 4 && minor >= 4)`) and say so in its message. Alternatively guard the expansions with `${parts[@]+"${parts[@]}"}`.

### IN-03: the sentinel snapshot covers only the case directory, one level above `dest`

**File:** `crates/mt-harness/tests/eqv_04_workflow_static.rs:1470-1500` (`snapshot_outside_dest`) and `1549-1585`.
**Issue:** the snapshot walks `dir` (the tempdir) minus `dest`. An escape of one level lands in `dir` and is caught. Rows whose links climb further (`../../../../etc`, `../../../x`) would write into the parent of the tempdir or beyond, which the snapshot does not see. In practice every wrong acceptance is also caught by the exit-code assertion, and GNU tar refuses to write through such a link, so this is not a hole today. It does overstate the comment "no row writes anywhere but into dest".
**Fix:** nest the case one level deeper (`dir/case/{src,dest,...}` with `dir` itself as the snapshot root, plus a sentinel next to `case`), or soften the comment to "no row writes into the case directory outside dest".

---

_Reviewed: 2026-11-04_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
