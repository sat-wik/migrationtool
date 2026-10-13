# Dropped targets and configurations

This file explains `testbed/dropped.toml`, the drop log of the test bed. Nothing
is dropped silently: a dropped target stays in `testbed/manifest.toml`, and each
drop is listed here with its cause. A test (`build_02_dropped_md_matches_drop_log`)
requires one heading under "Entries" for every entry of `dropped.toml` and no
other.

## Policy (D-11)

- A target or configuration that fails a full-bed run gets exactly one documented
  fix attempt, made only in its manifest data or committed inputs (a define, a
  prefix map, `-Wno-error`, a documented build option). No gate, normalisation
  rule or comparison is relaxed.
- If it still fails, it is dropped and logged with the cause, the differing
  objects and sections, the fix that was tried and the CI run that showed the
  failure.
- The granularity was chosen by the founder at the D-03 checkpoint
  (`drop_granularity: variant`): a variant that fails drops only that variant, and
  a failing `default` configuration drops the whole target.
- A variant that changes no compiled code is dropped as a variant with cause
  `variant_no_code_change` (D-05), whatever the granularity.
- A dropped pair is reported as `dropped (<cause>)` in the step summaries. It is
  never counted as passing and never counted toward the retained targets.
- At least 10 targets must remain retained. If fewer would, the phase fails by
  D-11 and only the founder can reopen a backup swap-in.

## Entry format

One heading per entry under "Entries": `## <target>` for a dropped target or
`## <target>/<config>` for a dropped variant. Under it: the cause, the differing
objects and sections, the fix tried, and the CI run.

# Entries

None yet. The test bed is intact: every target and configuration of the manifest
is retained.
