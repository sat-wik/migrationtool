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

## Reopened

`isotp` was dropped by testbed run 38387515333 (`entry_mismatch`: the entry check's
probe included `isotp.h` alone, and `isotp.h` includes `<stdint.h>` only under
`__cplusplus`). Plan 02-11 reopened it on the founder's decision G-02-1, after the
entry probe began reproducing the defining translation unit's include prefix (the
`#include`, `#define` and `#undef` lines `isotp.c` has before it includes
`isotp.h`), so it is no longer dropped and its seed corpus is committed again. The
entry comparison did not change. Its proof is the plan 02-17 testbed run: until that
run shows it captured and replayed identical, the reopening is unproven, and if the
run fails it is dropped again with the cause and the run id.

# Entries

The remaining entry comes from the first full-bed run (testbed run 38387515333,
head 564f256), where fetch verified all 16 tree hashes, the seeds check found no
drift, and capture refused exactly two pairs at the entry check. It cannot be
fixed in the recipe data, so it was dropped without a further CI attempt.

## paho/server

- Cause: `entry_mismatch`. `MQTTDeserialize_connack` is defined in
  `MQTTConnectClient.c`, which the variant's CMake target `MQTTPacketServer` does not
  compile, so the defining translation unit is not captured.
- Differing objects and sections: none.
- Fix tried: the entry header of paho was changed to `MQTTPacket.h` before run A
  (the declaring headers do not compile alone), which fixed `paho/default`. No
  data-only fix exists for the variant: the entry list is the founder-approved one,
  and changing it is not a recipe change.
- CI run: 38387515333. Only the variant is dropped (`drop_granularity: variant`);
  `paho/default` stays retained.
