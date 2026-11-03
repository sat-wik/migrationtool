---
phase: 03-equivalence-contract-and-test-vector-schema
reviewed: 2026-11-03T00:00:00Z
depth: standard
files_reviewed: 2
files_reviewed_list:
  - .github/workflows/testbed.yml
  - crates/mt-harness/tests/eqv_04_workflow_static.rs
findings:
  critical: 1
  warning: 2
  info: 1
  total: 4
status: issues_found
---

# Phase 3: Code Review Report (incremental, plan 03-11)

**Reviewed:** 2026-11-03
**Depth:** standard
**Scope:** incremental since a03811d (`git diff a03811d HEAD`), covering plan 03-11 only: the body of `safe_untar` and its comment in the vectors job step "Unpack the captures of the testbed job", and the behavioural test added to `eqv_04_workflow_static.rs`. This replaces the earlier phase-3 review, which is already dispositioned in 03-REVIEW-DISPOSITION.md.
**Files Reviewed:** 2
**Status:** issues_found

## Summary

The change does what the plan asks for the shapes it lists. I ran the workflow's own function (extracted with awk, under `set -euo pipefail`, GNU tar 1.35, bash 5.2.21) and `cargo test -p mt-harness --test eqv_04_workflow_static`: 14 of 14 pass. The nanopb and nimble in-tree links are accepted, and absolute, past-top, top-level `..`, `..` member names, hard links and the empty archive are refused. Names holding ` -> `, `@`, `*`, `]`, `$(...)` and an embedded newline cause no injection and no mis-pairing of the two listings. A 30,000-symlink archive takes about 6 s, so cost is linear.

The guard is not a sound implementation of the rule it states, though. The walk tracks links by their exact archive name, so a second link that aliases a directory hides the link from the walk, and a link that resolves outside the destination is accepted (CR-01). The test table has no row for this shape, so the suite passes anyway.

I did not find a way to write outside the destination. GNU tar 1.35 refuses to extract a member through the escaping link (`Cannot open: Not a directory`). The guard's own claim is broken, and what remains is one layer that it does not control.

## Critical Issues

### CR-01: `safe_untar` accepts a symlink that resolves outside the destination when a second link aliases a directory holding a link

**File:** `.github/workflows/testbed.yml:1049-1144` (the second-pass walk, especially the `islink` lookup at line 1133); the stated rule is in the comment at lines 1034-1048.
**Issue:** The walk is purely lexical and recognises a link only by the exact normalised name of an archive member. A `..` is refused only when the stack, joined, equals a recorded link name. A link `X -> m/n` makes `X/d/p` an alias of `m/n/d/p`. The key `X/d/p` is never in `islink`, so a `..` after it is treated as popping a real directory. The kernel does follow `p`.

I built this archive with Python `tarfile` and ran the extracted function on it:

```
m/            dir
m/n/          dir
m/n/d/        dir
m/n/d/p -> ../../..      (resolves to the top of the tree: accepted, as accept_up_to_top is)
X -> m/n                 (all pushes: accepted)
e -> X/d/p/..            (guard: stack [X,d,p], one pop, ends at [X,d], accepted)
```

The function exited 0 and extracted `e`. `readlink -f dest/e` is the parent of the destination, and `ls dest/e` listed the runner's `ci-lib.sh` next to `work/`. In the workflow that parent is `$RUNNER_TEMP`, which holds `ci-lib.sh`, sourced by every later step.

Consequences, as observed:

- The guard's documented contract ("any symlink whose target, walked from the link's own directory, leaves the tree" is refused) is false for this input.
- A follow-up member `e/ci-lib.sh` (file, symlink or directory) did not write outside the destination. GNU tar 1.35 itself refused each one with `Not a directory`. That is behaviour of the tar on the runner, not of the guard, and it is the only thing stopping the write. Threat T-03-42 names it as a "second layer", but here the first layer does not hold.
- The escaping link stays in `$RUNNER_TEMP/work`. Any later host-side step that follows it (a `cp -L`, `find -L`, `realpath`, or a tool that opens `work/e/...`) reaches `$RUNNER_TEMP`.

The comment claims the walk "fails closed where it cannot tell from the listing where a path lands". It does not for this shape, because the walk cannot tell that `X/d/p` is `m/n/d/p`.

**Fix:** Never let the walk descend through a link, so that the logical path always equals the physical one and the existing pop-of-link test is exact. Before pushing a component onto a non-empty stack, look up the joined stack in `islink` and refuse if it is a link. The same rule applies to the name's own directory parts, because they are the initial stack. For example, in the second pass:

```bash
if [ "$comp" != ".." ]; then
  if [ "${#stack[@]}" -gt 0 ]; then
    printf -v joined '%s/' "${stack[@]}"
    joined="${joined%/}"
    if [ -n "${islink[$joined]+x}" ]; then
      left=1
    fi
  fi
  stack+=("$comp")
elif ...
```

I applied this patch to a scratch copy. It refuses the archive above (`e -> X/d/p/..`, exit 1). No in-tree shape of nanopb or nimble pushes through a link, so the planned accept rows keep passing. I did not re-run the Rust table against the patched copy. Add the rows from WR-02 and update the comment so it states the real rule: a link target may not pass through any archive link, and not only pop one.

## Warnings

### WR-01: the target is cut from the verbose line by the first occurrence of ` NAME -> `, so the owner field can forge it

**File:** `.github/workflows/testbed.yml:1109` (`target="${line#*" $name -> "}"`)
**Issue:** `${line#*pattern}` removes the shortest prefix, so it matches the first ` NAME -> ` anywhere in the line, including the owner/group column that `tar -tvf` prints from the archive header. The comment and T-03-43 say the name is matched "after that name", which is not anchored. I forged a header (Python `tarfile`, `uname="u a -> p/q/r/s/t"`) for the link `a -> ../../../../etc`. GNU tar prints `... u a -> p/q/r/s/t/g 0 ... a -> ../../../../etc`. The guard took `p/q/r/s/t/g 0 1970-01-01 00:00 a -> ../../../../etc` as the target, saw enough pushes to absorb the four pops, and exited 0.

In the described flow the owner names come from the image's `/etc/passwd` through the trusted `tar -cf`, so the build container cannot choose them. That is why this is a warning and not a blocker. The guard should not depend on that fact. A numeric owner column would remove it.

**Fix:** List with `tar --numeric-owner -tvf "$archive"`. The owner column is then digits and a slash (checked: `0/0`), and it can never contain ` -> `. As a second layer, require that the match begins at the expected column by anchoring the removal on the fixed prefix, for example `[[ $line =~ ^l[^\ ]+\ [0-9]+/[0-9]+\ +[0-9]+\ [0-9-]+\ [0-9:]+\  ]]` before cutting.

### WR-02: the test table does not pin the shape of CR-01, and some refuse rows can pass for the wrong reason

**File:** `crates/mt-harness/tests/eqv_04_workflow_static.rs:1423-1593` (the test and its table; `refuse_up_through_link` is at line 1551) and `check_untar_case` at line 1361.
**Issue:**
- The only "through a link" row is `refuse_up_through_link`, where the link that is popped is itself named in the stack. There is no row where a second link makes an alias path (`X -> m/n`, `m/n/d/p -> ../../..`, `e -> X/d/p/..`). Because the table lacks it, the suite passes on a guard that accepts an escaping link.
- Every refused link row asserts the fragment `leaves the tree`. The member-name message ("holds a member name that is absolute or leaves the tree") contains the same fragment, so a link row whose archive also trips the name check would pass for the wrong reason. The current rows do not, but the assertion cannot tell the two apart.
- The branches `cannot read the target of the symlink` and `tar -tf and tar -tvf list different members` have no row.
- The test asserts only an empty destination for refused rows. It does not assert that nothing outside the destination was touched, which is the property that matters.

**Fix:** Add a `refuse_alias_up_through_link` row with the six members of CR-01, in that archive order. Give the link-walk refusal a message that does not share a fragment with the name check (for example `symlink ... leaves the tree`), and assert that fragment on the link rows. Add a sentinel file beside `dest` in `check_untar_case` and assert that the directory outside `dest` is unchanged and contains no new entries after every row.

## Info

### IN-01: the test needs GNU tar and bash 4.3 or later on the host, but only checks that `/bin/bash` exists

**File:** `crates/mt-harness/tests/eqv_04_workflow_static.rs:1261` (`host_tar`) and the assertion at the top of the test.
**Issue:** `host_tar` is documented as "the GNU tar of this host", but it only checks that `/usr/bin/tar` or `/bin/tar` exists. The function under test uses `mapfile` and `local -A`, which `/bin/bash` 3.2 on macOS lacks, and the `tar -tvf` format it parses is GNU's. On such a host the test fails with an obscure shell or `tar -P` error instead of a message naming the missing tool. The plan's Tracer precondition asked for exactly this check (`tar --version` containing `GNU tar`), and the test does not make it. The project is Linux-only (CLAUDE.md), so this stays low priority.
**Fix:** Run `tar --version` and `bash -c 'echo ${BASH_VERSINFO[0]}'` through the runner at the start of the test, and assert `GNU tar` and a major version of 4 or more with a message that names the requirement.

---

_Reviewed: 2026-11-03_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
