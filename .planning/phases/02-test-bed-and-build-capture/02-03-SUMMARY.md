---
phase: 02-test-bed-and-build-capture
plan: 03
status: in-progress
---

# Phase 2 Plan 3: Founder approval of the test-bed target list (D-03)

This section is written before `testbed/manifest.toml` exists (D-03 ordering). The rest of the summary is completed at the end of the plan.

## Founder approval (Task 2, checkpoint:decision, gate blocking-human)

The orchestrator collected the answer directly from the founder at the D-03 checkpoint.

```
target_list_approval: approve-as-proposed
drop_granularity: variant
```

- **Edits:** no edits. The list, entries and variants are exactly those of RESEARCH Tables 1-3.
- **Final target ids (16):** tinycbor, zcbor, qcbor, nanopb, cjson, corejson, coremqtt, paho, microcoap, mbedtls, nimble, libfdt, httpparser, isotp, lz4, zlib.
- **Entries:** 27 (the 28 of Table 2 minus libfdt `fdt_check_header`, which is omitted as proposed because it takes no length and is not a D-24 buffer-in entry; `fdt_check_full` stays). The six wrapped entries carry `accepted_wrapped = true` (tinycbor `cbor_value_validate`, qcbor `QCBORDecode_Init`, nanopb `pb_decode_ex`, coremqtt `MQTT_DeserializePublish` and `MQTT_DeserializeAck`, zlib `inflate`); mbedtls `mbedtls_asn1_get_tag` is form `range`.
- **Variants:** those of Table 3 (one per target, none for isotp), giving 31 target-configuration pairs.
- **D-27 elections:** mbedtls Apache-2.0, paho BSD-3-Clause, libfdt BSD-2-Clause.
- **Backups:** libcbor and yajl are NOT added.
- **Drop granularity (research Open Question 4):** `drop_granularity: variant`. A variant that still fails after its one fix attempt (D-11) drops only that variant, which is logged. A default configuration that fails drops the target.
- **Recorded, not decided again:** http-parser appends `-O3` after the baseline and the gate records the optimisation goal without failing it (Open Question 7); Mbed TLS is fetched as the release tarball (Open Question 5, becomes PROJECT D-20); signatures were printed by clang 18 and the tracer checks them with the image's clang 16 (A4).
