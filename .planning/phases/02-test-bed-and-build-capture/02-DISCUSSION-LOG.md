# Phase 2: Test Bed and Build Capture - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-10-12
**Phase:** 02-test-bed-and-build-capture
**Areas discussed:** Target list & licences, Configs & RTOS choice, Reproduction bar, Seed corpus & held-out, Source fetching & cache, Manifest & CLI surface, Entry functions, CI scope

---

## Target list & licences

| Option | Description | Selected |
|--------|-------------|----------|
| Start with ~16 | Up to 6 can drop and 10 still remain | ✓ |
| Start with 20 | Most headroom, more work | |
| Start with 12 | Least work; only 2 drops allowed | |

| Option | Description | Selected |
|--------|-------------|----------|
| Permissive only | MIT/BSD/Apache/Zlib/ISC; no copyleft issues when publishing | ✓ |
| Permissive + LGPL/GPL | Wider pool, copyleft obligations | |
| Any OSI licence | Widest pool, resolved at publish time | |

| Option | Description | Selected |
|--------|-------------|----------|
| Researcher proposes | Research proposes the list; founder approves before capture | ✓ |
| I'll give a list | Founder names the targets | |
| Spec examples + fill | Spec's 5 fixed; research fills the rest without review | |

| Option | Description | Selected |
|--------|-------------|----------|
| Cover all beachheads | TLV, CBOR, MQTT, BLE, OTA header, ASN.1, plus general formats | ✓ |
| Depth on CBOR/TLV | Several implementations of the same formats | |
| You decide | Research balances it | |

**User's choice:** All four recommended options.

---

## Configs & RTOS choice

| Option | Description | Selected |
|--------|-------------|----------|
| Default + 1 variant | Upstream default plus one realistic variant | ✓ |
| Default only | One configuration per target | |
| All documented options | Every documented macro set | |

| Option | Description | Selected |
|--------|-------------|----------|
| Cortex-M4F baseline | `-mcpu=cortex-m4 -mthumb -mfloat-abi=hard -mfpu=fpv4-sp-d16 -Os`, matching thumbv7em-none-eabihf | ✓ |
| M4F + M0+ variant | Adds thumbv6m for some targets | |
| Upstream defaults | Normalise later | |

| Option | Description | Selected |
|--------|-------------|----------|
| Upstream build + toolchain file | The project's own Make/CMake, captured with Bear or the CMake export | ✓ |
| Our own minimal build | A per-target source list compiled directly | |
| Mix per target | Upstream where possible, minimal otherwise | |

| Option | Description | Selected |
|--------|-------------|----------|
| Deferred | "Deferred; parsers are RTOS-independent" | ✓ |
| Zephyr | Industrial IoT | |
| FreeRTOS | Consumer IoT | |

**User's choice:** All four recommended options.

---

## Reproduction bar

| Option | Description | Selected |
|--------|-------------|----------|
| Objects, with normalisation | Byte-identical objects as the goal; fixed, documented normalisation (prefix maps, `.comment` and debug sections) | ✓ |
| Byte-identical objects only | No normalisation | |
| Objects + linked archive | Also compare the `.a` built with deterministic `ar` | |

| Option | Description | Selected |
|--------|-------------|----------|
| In the container, under Bear | Capture once, then replay in a clean tree in the same container | ✓ |
| Build twice and compare | Also an uncaptured second build | |
| You decide | Research picks | |

| Option | Description | Selected |
|--------|-------------|----------|
| Drop after one fix attempt | One documented fix, then drop with the cause logged | ✓ |
| Drop immediately | No fix attempt | |
| Replace with a backup | A swap-in backup pool | |

| Option | Description | Selected |
|--------|-------------|----------|
| Fetch online, build offline | Capture and replay with `--network none` | ✓ |
| Network allowed | Simpler | |

**User's choice:** All four recommended options.

---

## Seed corpus & held-out

| Option | Description | Selected |
|--------|-------------|----------|
| Tests + fixtures + fuzz corpora | Provenance recorded per seed; hand-made seeds only when a target has none | ✓ |
| Fixture files only | Standalone files only | |
| Hand-made for all | Written by hand for every target | |

| Option | Description | Selected |
|--------|-------------|----------|
| Defer to Phase 7 | GATE-05 designs the held-out split once | ✓ |
| Split now, deterministically | A seeded ~20% split stored outside the workspace | |
| Separate held-out source | Built later from different inputs | |

| Option | Description | Selected |
|--------|-------------|----------|
| Raw files + index | Files named by sha256 plus a per-target index | ✓ |
| TRACTOR vectors now | Adopt the Phase 3 schema early | |

| Option | Description | Selected |
|--------|-------------|----------|
| Committed under testbed/ | Licence noted, capped by size and count | ✓ |
| Generated on demand | Only the recipe is committed | |

**User's choice:** All four recommended options.

---

## Source fetching & cache

| Option | Description | Selected |
|--------|-------------|----------|
| Git commit + tree hash | Shallow fetch of the commit, then verify the commit SHA and the sha256 of the tree | ✓ |
| Release tarball + sha256 | Archive and checksum | |
| Either, per target | Git by default, tarball when a release exists | |

| Option | Description | Selected |
|--------|-------------|----------|
| Git-ignored work dir | `.mt-work/` or `$MT_WORK_DIR` | ✓ |
| System cache dir | `~/.cache/migrationtool` | |

| Option | Description | Selected |
|--------|-------------|----------|
| Inside the container | The pinned image's git, in a networked run | ✓ |
| On the host | Host git | |

**User's choice:** All three recommended options.

---

## Manifest & CLI surface

| Option | Description | Selected |
|--------|-------------|----------|
| One testbed/manifest.toml | A single validated TOML, in pins.toml style | ✓ |
| One file per target | `testbed/<target>/target.toml` | |

| Option | Description | Selected |
|--------|-------------|----------|
| mt testbed fetch\|capture\|replay\|seeds | Logic in mt-capture plus a testbed module | ✓ |
| mt capture … / mt testbed … | Split by concern | |

| Option | Description | Selected |
|--------|-------------|----------|
| Runner records + per-object hashes | A full JSON report per target and configuration | ✓ |
| Pass/fail summary only | Smaller | |

**User's choice:** All three recommended options.

---

## Entry functions

| Option | Description | Selected |
|--------|-------------|----------|
| Manual + checked | Named in the manifest; existence and signature checked through a clang AST dump | ✓ |
| Manual only | No check | |
| Heuristic discovery | Header scan proposes candidates | |

| Option | Description | Selected |
|--------|-------------|----------|
| Pointer + length, plus context | Buffer and length, optionally with a context or output struct; 1-3 per target | ✓ |
| Strict ptr+len only | Buffer and length only | |

**User's choice:** Both recommended options.

---

## CI scope

| Option | Description | Selected |
|--------|-------------|----------|
| Full bed weekly + on change | New testbed workflow, on a schedule and when testbed/ or mt-capture change | ✓ |
| Full bed on every push | Slow | |
| Manual dispatch only | Cheapest | |

| Option | Description | Selected |
|--------|-------------|----------|
| Recorded fixtures + tiny local C | Offline `cargo test` with fixtures and a 2-file C project | ✓ |
| Fixtures only | No live compile | |

**User's choice:** Both recommended options.

---

## Claude's Discretion

- The concrete target list, commits, entry functions and variants (founder approval checkpoint).
- Seed caps, the index format and the drop-log layout.
- Crate split, CMake toolchain-file details and the exact normalisation commands.

## Deferred Ideas

- Held-out split (Phase 7, GATE-05); the final first-RTOS choice; copyleft targets; M0+/M33 profiles; heuristic entry discovery; IAR, Keil and Zephyr west; a backup-target pool.
