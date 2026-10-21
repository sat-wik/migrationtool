# Plan 02-22 round trip 1: testbed notices

Run 39528941371 (testbed), head 9273ef82979df478d7e7bba80bd27a8345b0dff0, job 118990405406, read from the job annotations with `gh api` (parts joined in order, final newline dropped as the workflow's annotate helper specifies).

## testbed-image (1 annotation part(s))

```text
image: ghcr.io/sat-wik/migrationtool-toolchain@sha256:e83006c0a7e37f788afcbd1c95554e8c4dfc61c3b68da3ff964caf47fb3851b9
id: sha256:1267a6230e171d711a7d056cd7a129fbdf0753339c92af22dac69afc806eac44
architecture: linux/amd64
```

## testbed-validate (1 annotation part(s))

```text
step: validate
manifest  valid (16 targets, 31 configurations)
result: passed (1 rows)
exit code: 0
```

## testbed-fetch (1 annotation part(s))

```text
step: fetch
cjson       fetched
corejson    fetched
coremqtt    fetched
httpparser  fetched
isotp       fetched
libfdt      fetched
lz4         fetched
mbedtls     fetched
microcoap   fetched
nanopb      fetched
nimble      fetched
paho        fetched
qcbor       fetched
tinycbor    fetched
zcbor       fetched
zlib        fetched
result: passed (16 rows)
exit code: 0
--- fetched sources
cjson git commit c859b25da02955fef659d658b8f324b5cde87be3 tree_sha256 cb98da4236b68041e4be9a325bf3b41ce8a639bd0080faa0abe4a117afde0618 reused=false
corejson git commit cffa492da18c890181d64462f8af63992a69d3b0 tree_sha256 9ca1058329f6ce0c31f9c13f861ec20cf75f35230b9aae0122ab81480a6df9a5 reused=false
coremqtt git commit 04845c6a8e5f9cf2d232f1c6e80baeb81302e690 tree_sha256 4ab12658af26706fb40200ff240e86b240041bcb002e5467ff1c2b01e7c0739e reused=false
httpparser git commit 2343fd6b5214b2ded2cdcf76de2bf60903bb90cd tree_sha256 03f91004dd3d60a8b7ada8106d16b3ec6976006f208b216a3f630434436a68a6 reused=false
isotp git commit 5593428d95af10dde1e565cebcda16089fc74857 tree_sha256 6829a25ee549e3ea0e152124ddd6ebb1945ac548a2b46ce4c3f06e73da73e4c0 reused=false
libfdt git commit 8f48565e5cfedc74d3f7512f1e0188e9d85dc1de tree_sha256 f844d15ee31ac3a97fdecce950e50472a4f7a889c2e4e1b678bf9f26dc21b313 reused=false
lz4 git commit ebb370ca83af193212df4dcbadcc5d87bc0de2f0 tree_sha256 8def842ac8ebd5d80ddd31f00ddb3bb156edf5d7980dbcde5c734ac7a2f57ca7 reused=false
mbedtls archive commit 068ff080b369adfac81509f9b57b2afabaf82dc5 tree_sha256 00a636ffc99e447237abd6dee684a9ba0fe3727c1ea6d266cbe68989bb408248 reused=false
microcoap git commit ef272895925f0d4c563725fe0102966f544a0fdc tree_sha256 8d331b8362cb16f499647c044f5d4b2033e5fc8ad5a3a33809d86582526cb415 reused=false
nanopb git commit 160d4f09e5fabb2b66aa2dea32d4f38ace2c4b3f tree_sha256 b86f61ac9e874160f9f405412d131d3522c2e9b536a582b8581158ee9520b6ce reused=false
nimble git commit a7a156f28954819e158b62dd613008f22f9cf73b tree_sha256 726eb68c8d5179d0c57aade2ecf44f4a653a11870e0585f8b340ee154e317700 reused=false
paho git commit 6035ea2d4922bb7558b444fb2a051743f3f1974b tree_sha256 a69bce85a45e561bdf490f80b35ddae7d13eb04a67c9b59fb070a2593286320b reused=false
qcbor git commit 930708bb86481e88879eb1d87fd4d664f1d69503 tree_sha256 ed42fa8d2ea1619e71613e2925c197ae55a6c0dee151d61983334ef6968f8ee0 reused=false
tinycbor git commit 6442e749ca811e24afad1551338a45c75e09808f tree_sha256 9acf217a3877140b998586cc8c4172e28bfe90630938344e3cc6067a35b86189 reused=false
zcbor git commit 9164bd18dcd88ff9d9ef98279501fc1093571017 tree_sha256 131d840f381cd3ac9e5d118f7c59717f04deda4c92e546eeab12663528509f1f reused=false
zlib git commit da607da739fa6047df13e66a2af6b8bec7c2a498 tree_sha256 76b51f6bd05c3ad238b1a05c7c163f83f082b64d9d1d3f5df83912f2bd3b26f3 reused=false
```

## testbed-seeds (1 annotation part(s))

```text
step: seeds
cjson       seeds 25
corejson    seeds 108
coremqtt    seeds 15
httpparser  seeds 76
isotp       seeds 19
libfdt      seeds 16
lz4         seeds 20
mbedtls     seeds 200
microcoap   seeds 16
nanopb      seeds 200
nimble      seeds 200
paho        seeds 17
qcbor       seeds 200
tinycbor    seeds 200
zcbor       seeds 137
zlib        seeds 18
result: passed (16 rows)
exit code: 0
--- committed seed corpora (compared offline with the container's regeneration)
cjson commit c859b25da02955fef659d658b8f324b5cde87be3 seeds=25 hand_made=0 bytes=16175 skipped={"duplicate":0,"too_large":0,"over_cap":0,"unsupported":0}
corejson commit cffa492da18c890181d64462f8af63992a69d3b0 seeds=108 hand_made=0 bytes=1210 skipped={"duplicate":119,"too_large":0,"over_cap":0,"unsupported":0}
coremqtt commit 04845c6a8e5f9cf2d232f1c6e80baeb81302e690 seeds=15 hand_made=10 bytes=133 skipped={"duplicate":988,"too_large":0,"over_cap":0,"unsupported":32}
httpparser commit 2343fd6b5214b2ded2cdcf76de2bf60903bb90cd seeds=76 hand_made=0 bytes=9004 skipped={"duplicate":0,"too_large":0,"over_cap":0,"unsupported":0}
isotp commit 5593428d95af10dde1e565cebcda16089fc74857 seeds=19 hand_made=19 bytes=93 skipped={"duplicate":0,"too_large":0,"over_cap":0,"unsupported":0}
libfdt commit 8f48565e5cfedc74d3f7512f1e0188e9d85dc1de seeds=16 hand_made=16 bytes=1690 skipped={"duplicate":0,"too_large":0,"over_cap":0,"unsupported":0}
lz4 commit ebb370ca83af193212df4dcbadcc5d87bc0de2f0 seeds=20 hand_made=20 bytes=213 skipped={"duplicate":0,"too_large":0,"over_cap":0,"unsupported":0}
mbedtls commit 068ff080b369adfac81509f9b57b2afabaf82dc5 seeds=200 hand_made=0 bytes=194050 skipped={"duplicate":1,"too_large":0,"over_cap":25,"unsupported":0}
microcoap commit ef272895925f0d4c563725fe0102966f544a0fdc seeds=16 hand_made=16 bytes=146 skipped={"duplicate":0,"too_large":0,"over_cap":0,"unsupported":0}
nanopb commit 160d4f09e5fabb2b66aa2dea32d4f38ace2c4b3f seeds=200 hand_made=0 bytes=84459 skipped={"duplicate":0,"too_large":189,"over_cap":3291,"unsupported":0}
nimble commit a7a156f28954819e158b62dd613008f22f9cf73b seeds=200 hand_made=0 bytes=2592 skipped={"duplicate":424,"too_large":0,"over_cap":873,"unsupported":471}
paho commit 6035ea2d4922bb7558b444fb2a051743f3f1974b seeds=17 hand_made=17 bytes=230 skipped={"duplicate":0,"too_large":0,"over_cap":0,"unsupported":0}
qcbor commit 930708bb86481e88879eb1d87fd4d664f1d69503 seeds=200 hand_made=0 bytes=6427 skipped={"duplicate":32,"too_large":0,"over_cap":88,"unsupported":331}
tinycbor commit 6442e749ca811e24afad1551338a45c75e09808f seeds=200 hand_made=0 bytes=1121 skipped={"duplicate":36,"too_large":0,"over_cap":2197,"unsupported":0}
zcbor commit 9164bd18dcd88ff9d9ef98279501fc1093571017 seeds=137 hand_made=0 bytes=21972 skipped={"duplicate":80,"too_large":0,"over_cap":0,"unsupported":354}
zlib commit da607da739fa6047df13e66a2af6b8bec7c2a498 seeds=18 hand_made=18 bytes=239 skipped={"duplicate":0,"too_large":0,"over_cap":0,"unsupported":0}
```

## testbed-build (1 annotation part(s))

```text
cjson/default cjson/default built
cjson/no-locale cjson/no-locale built
corejson/default corejson/default built
corejson/depth-8 corejson/depth-8 built
coremqtt/default coremqtt/default built
coremqtt/sub-vectors-2 coremqtt/sub-vectors-2 built
httpparser/default httpparser/default built
httpparser/strict httpparser/strict built
isotp/default isotp/default built
libfdt/default libfdt/default built
libfdt/assume-valid libfdt/assume-valid built
lz4/default lz4/default built
lz4/heap-small lz4/heap-small built
mbedtls/default mbedtls/default built
mbedtls/suite-b mbedtls/suite-b built
microcoap/default microcoap/default built
microcoap/release microcoap/release built
nanopb/default nanopb/default built
nanopb/buffer-only nanopb/buffer-only built
nimble/default nimble/default built
nimble/peripheral nimble/peripheral built
paho/default paho/default built
qcbor/default qcbor/default built
qcbor/no-hw-float qcbor/no-hw-float built
tinycbor/default tinycbor/default built
tinycbor/no-float tinycbor/no-float built
zcbor/default zcbor/default built
zcbor/stop-on-error zcbor/stop-on-error built
zlib/default zlib/default built
zlib/no-gzip zlib/no-gzip built
exit code: 0
```

## testbed-capture (5 annotation part(s))

```text
step: capture
cjson/default           captured
cjson/no-locale         captured
cjson/variant           changes_code true
corejson/default        captured
corejson/depth-8        captured
corejson/variant        changes_code true
coremqtt/default        captured
coremqtt/sub-vectors-2  captured
coremqtt/variant        changes_code true
httpparser/default      captured
httpparser/strict       captured
httpparser/variant      changes_code true
isotp/default           captured
libfdt/assume-valid     captured
libfdt/default          captured
libfdt/variant          changes_code true
lz4/default             captured
lz4/heap-small          captured
lz4/variant             changes_code true
mbedtls/default         captured
mbedtls/suite-b         captured
mbedtls/variant         changes_code true
microcoap/default       captured
microcoap/release       captured
microcoap/variant       changes_code true
nanopb/buffer-only      captured
nanopb/default          captured
nanopb/variant          changes_code true
nimble/default          captured
nimble/peripheral       captured
nimble/variant          changes_code true
paho/default            captured
paho/server             dropped (entry_mismatch)
paho/variant            dropped (entry_mismatch)
qcbor/default           captured
qcbor/no-hw-float       captured
qcbor/variant           changes_code true
tinycbor/default        captured
tinycbor/no-float       captured
tinycbor/variant        changes_code true
zcbor/default           captured
zcbor/stop-on-error     captured
zcbor/variant           changes_code true
zlib/default            captured
zlib/no-gzip            captured
zlib/variant            changes_code true
result: passed (46 rows)
exit code: 0
--- gates and entries
cjson/default compiler_allow_listed=true arch_ok=true undeclared=0 timestamp_macro=0 objects=1 opt_goals=Aggressive Size entries=cJSON_ParseWithLength=found:true,signature:true,header:true
cjson/no-locale compiler_allow_listed=true arch_ok=true undeclared=0 timestamp_macro=0 objects=1 opt_goals=Aggressive Size entries=cJSON_ParseWithLength=found:true,signature:true,header:true
corejson/default compiler_allow_listed=true arch_ok=true undeclared=0 timestamp_macro=0 objects=1 opt_goals=Aggressive Size entries=JSON_SearchT=found:true,signature:true,header:true;JSON_Validate=found:true,signature:true,header:true
corejson/depth-8 compiler_allow_listed=true arch_ok=true undeclared=0 timestamp_macro=0 objects=1 opt_goals=Aggressive Size entries=JSON_SearchT=found:true,signature:true,header:true;JSON_Validate=found:true,signature:true,header:true
coremqtt/default compiler_allow_listed=true arch_ok=true undeclared=0 timestamp_macro=0 objects=6 opt_goals=Aggressive Size entries=MQTT_DeserializeAck=found:true,signature:true,header:true;MQTT_DeserializePublish=found:true,signature:true,header:true
coremqtt/sub-vectors-2 compiler_allow_listed=true arch_ok=true undeclared=0 timestamp_macro=0 objects=6 opt_goals=Aggressive Size entries=MQTT_DeserializeAck=found:true,signature:true,header:true;MQTT_DeserializePublish=found:true,signature:true,header:true
httpparser/default compiler_allow_listed=true arch_ok=true undeclared=0 timestamp_macro=0 objects=1 opt_goals=Aggressive Speed entries=http_parser_execute=found:true,signature:true,header:true;http_parser_parse_url=found:true,signature:true,header:true
httpparser/strict compiler_allow_listed=true arch_ok=true undeclared=0 timestamp_macro=0 objects=1 opt_goals=Aggressive Speed entries=http_parser_execute=found:true,signature:true,header:true;http_parser_parse_url=found:true,signature:true,header:true
isotp/default compiler_allow_listed=true arch_ok=true undeclared=0 timestamp_macro=0 objects=1 opt_goals=Aggressive Size entries=isotp_on_can_message=found:true,signature:true,header:true
libfdt/assume-valid compiler_allow_listed=true arch_ok=true undeclared=0 timestamp_macro=0 objects=10 opt_goals=Aggressive Size entries=fdt_check_full=found:true,signature:true,header:true
libfdt/default compiler_allow_listed=true arch_ok=true undeclared=0 timestamp_macro=0 objects=10 opt_goals=Aggressive Size entries=fdt_check_full=found:true,signature:true,header:true
lz4/default compiler_allow_listed=true arch_ok=true undeclared=0 timestamp_macro=0 objects=5 opt_goals=Aggressive Size entries=LZ4F_getFrameInfo=found:true,signature:true,header:true;LZ4_decompress_safe=found:true,signature:true,header:true
lz4/heap-small compiler_allow_listed=true arch_ok=true undeclared=0 timestamp_macro=0 objects=5 opt_goals=Aggressive Size entries=LZ4F_getFrameInfo=found:true,signature:true,header:true;LZ4_decompress_safe=found:true,signature:true,header:true
mbedtls/default compiler_allow_listed=true arch_ok=true undeclared=0 timestamp_macro=0 objects=113 opt_goals=Aggressive Size entries=mbedtls_asn1_get_tag=found:true,signature:true,header:true;mbedtls_x509_crt_parse_der=found:true,signature:true,header:true
mbedtls/suite-b compiler_allow_listed=true arch_ok=true undeclared=0 timestamp_macro=0 objects=113 opt_goals=Aggressive Size entries=mbedtls_asn1_get_tag=found:true,signature:true,header:true;mbedtls_x509_crt_parse_der=found:true,signature:true,header:true
microcoap/default compiler_allow_listed=true arch_ok=true undeclared=0 timestamp_macro=0 objects=2 opt_goals=Aggressive Size entries=coap_parse=found:true,signature:true,header:true
microcoap/release compiler_allow_listed=true arch_ok=true undeclared=0 timestamp_macro=0 objects=2 opt_goals=Aggressive Size entries=coap_parse=found:true,signature:true,header:true
nanopb/buffer-only compiler_allow_listed=true arch_ok=true undeclared=0 timestamp_macro=0 objects=3 opt_goals=Aggressive Size entries=pb_decode_ex=found:true,signature:true,header:true;pb_istream_from_buffer=found:true,signature:true,header:true
nanopb/default compiler_allow_listed=true arch_ok=true undeclared=0 timestamp_macro=0 objects=3 opt_goals=Aggressive Size entries=pb_decode_ex=found:true,signature:true,header:true;pb_istream_from_buffer=found:true,signature:true,header:true
nimble/default compiler_allow_listed=true arch_ok=true undeclared=0 timestamp_macro=0 objects=65 opt_goals=Aggressive Size entries=ble_hs_adv_parse_fields=found:true,signature:true,header:true;ble_uuid_init_from_buf=found:true,signature:true,header:true
nimble/peripheral compiler_allow_listed=true arch_ok=true undeclared=0 timestamp_macro=0 objects=65 opt_goals=Aggressive Size entries=ble_hs_adv_parse_fields=found:true,signature:true,header:true;ble_uuid_init_from_buf=found:true,signature:true,header:true
paho/default compiler_allow_listed=true arch_ok=true undeclared=0 timestamp_macro=0 objects=7 opt_goals=Aggressive Size entries=MQTTDeserialize_connack=found:true,signature:true,header:true;MQTTDeserialize_publish=found:true,signature:true,header:true
qcbor/default compiler_allow_listed=true arch_ok=true undeclared=0 timestamp_macro=0 objects=5 opt_goals=Aggressive Size entries=QCBORDecode_Init=found:true,signature:true,header:true
qcbor/no-hw-float compiler_allow_listed=true arch_ok=true undeclared=0 timestamp_macro=0 objects=5 opt_goals=Aggressive Size entries=QCBORDecode_Init=found:true,signature:true,header:true
tinycbor/default compiler_allow_listed=true arch_ok=true undeclared=0 timestamp_macro=0 objects=11 opt_goals=Aggressive Size entries=cbor_parser_init=found:true,signature:true,header:true;cbor_value_validate=found:true,signature:true,header:true
tinycbor/no-float compiler_allow_listed=true arch_ok=true undeclared=0 timestamp_macro=0 objects=8 opt_goals=Aggressive Size entries=cbor_parser_init=found:true,signature:true,header:true;cbor_value_validate=found:true,signature:true,header:true
zcbor/default compiler_allow_listed=true arch_ok=true undeclared=0 timestamp_macro=0 objects=6 opt_goals=Aggressive Size entries=cbor_decode_Pet=found:true,signature:true,header:true;zcbor_new_decode_state=found:true,signature:true,header:true
zcbor/stop-on-error compiler_allow_listed=true arch_ok=true undeclared=0 timestamp_macro=0 objects=6 opt_goals=Aggressive Size entries=cbor_decode_Pet=found:true,signature:true,header:true;zcbor_new_decode_state=found:true,signature:true,header:true
zlib/default compiler_allow_listed=true arch_ok=true undeclared=0 timestamp_macro=0 objects=15 opt_goals=Aggressive Size entries=inflate=found:true,signature:true,header:true;uncompress=found:true,signature:true,header:true
zlib/no-gzip compiler_allow_listed=true arch_ok=true undeclared=0 timestamp_macro=0 objects=15 opt_goals=Aggressive Size entries=inflate=found:true,signature:true,header:true;uncompress=found:true,signature:true,header:true
--- normalised configure files (G-02-2)
cjson/default normalised build/CMakeFiles/Makefile2 rule=each run of consecutive prerequisite lines of one target (<target>: <prerequisite>) sorted bytewise; every other line in place sha256=8bff7c99e9708a0cb0fdbc9658735b929b6856ddd5e51a89f7c6e6870b6894a6
cjson/no-locale normalised build/CMakeFiles/Makefile2 rule=each run of consecutive prerequisite lines of one target (<target>: <prerequisite>) sorted bytewise; every other line in place sha256=ecad9de4cb8d3746e3802a864b3ceee984602bf15442297fed7c38c1a2fe7b1e
corejson/default normalised build/CMakeFiles/Makefile2 rule=each run of consecutive prerequisite lines of one target (<target>: <prerequisite>) sorted bytewise; every other line in place sha256=4545f560c11020ca6418a1ec5dfdce432231cbc3e2d94f649dfdcd7e926a6c68
corejson/depth-8 normalised build/CMakeFiles/Makefile2 rule=each run of consecutive prerequisite lines of one target (<target>: <prerequisite>) sorted bytewise; every other line in place sha256=9cc29979dd253b22589d3624082e27674b58ac2564f8aefa1ee7d767bb3c8c3e
coremqtt/default normalised build/CMakeFiles/Makefile2 rule=each run of consecutive prerequisite lines of one target (<target>: <prerequisite>) sorted bytewise; every other line in place sha256=76c2668cbd7bea89d409fcb889b61217fb1a4d25a333bc8cfcdf10beea7d7b98
coremqtt/sub-vectors-2 normalised build/CMakeFiles/Makefile2 rule=each run of consecutive prerequisite lines of one target (<target>: <prerequisite>) sorted bytewise; every other line in place sha256=7d3c07d1344241933f1b706d57a40ea7d9db71da92987f5db1bc2ace74eb1587
isotp/default normalised build/CMakeFiles/Makefile2 rule=each run of consecutive prerequisite lines of one target (<target>: <prerequisite>) sorted bytewise; every other line in place sha256=efaeba0a31ee518f7e44b522c190f71c1f3f6dfa76ed540a429b58a27afff389
nanopb/buffer-only normalised build/CMakeFiles/Makefile2 rule=each run of consecutive prerequisite lines of one target (<target>: <prerequisite>) sorted bytewise; every other line in place sha256=256c554ec411a912e3cfd884aa957f08ecd908b59b193b3cac713c0b60f65426
nanopb/default normalised build/CMakeFiles/Makefile2 rule=each run of consecutive prerequisite lines of one target (<target>: <prerequisite>) sorted bytewise; every other line in place sha256=7f1afaa778f3bc14cedccfed6bed2b1d06b9701cbd3b6587e3ac616447a6d534
paho/default normalised build/CMakeFiles/Makefile2 rule=each run of consecutive prerequisite lines of one target (<target>: <prerequisite>) sorted bytewise; every other line in place sha256=c9cdc2d27209c14d6516904976a35b5a2bf9c545b52c652ea084add3bcc0518c
tinycbor/default normalised build/CMakeFiles/Makefile2 rule=each run of consecutive prerequisite lines of one target (<target>: <prerequisite>) sorted bytewise; every other line in place sha256=303d258ad26435776ac4c5f8696daf430d0e8fbbb021f80856c2d7daaeb2b93e
tinycbor/no-float normalised build/CMakeFiles/Makefile2 rule=each run of consecutive prerequisite lines of one target (<target>: <prerequisite>) sorted bytewise; every other line in place sha256=d169350f749023b05254c523e49831f0af3b24d2f92f4576f806f54e801bbc8b
zcbor/default normalised build/CMakeFiles/Makefile2 rule=each run of consecutive prerequisite lines of one target (<target>: <prerequisite>) sorted bytewise; every other line in place sha256=c02027e2ef6a205568e2a6ece744442fd59ea95c929ae7bce469ec4664d02479
zcbor/stop-on-error normalised build/CMakeFiles/Makefile2 rule=each run of consecutive prerequisite lines of one target (<target>: <prerequisite>) sorted bytewise; every other line in place sha256=7fdddacc78c2cea52f6d37479e3775f96a06f11d2c45641ae30bd7706ad50827
zlib/default normalised build/CMakeFiles/Makefile2 rule=each run of consecutive prerequisite lines of one target (<target>: <prerequisite>) sorted bytewise; every other line in place sha256=722963173a56e34b7d61849f21c9f07f53eb9d038b413c35250488e30ee2077d
zlib/no-gzip normalised build/CMakeFiles/Makefile2 rule=each run of consecutive prerequisite lines of one target (<target>: <prerequisite>) sorted bytewise; every other line in place sha256=283619370e1cefc659a8f101d44843aa3a1f26174b3c91109a662d56e2d40810
--- variants
cjson variant no-locale: changes_code=true code_differs=build/CMakeFiles/cjson.dir/cJSON.c.obj
corejson variant depth-8: changes_code=true code_differs=build/CMakeFiles/core_json.dir/source/core_json.c.obj
coremqtt variant sub-vectors-2: changes_code=true code_differs=build/CMakeFiles/core_mqtt.dir/source/core_mqtt.c.obj
httpparser variant strict: changes_code=true code_differs=src/libhttp_parser.o
libfdt variant assume-valid: changes_code=true code_differs=src/libfdt/fdt.o,src/libfdt/fdt_overlay.o,src/libfdt/fdt_ro.o,src/libfdt/fdt_rw.o,src/libfdt/fdt_sw.o
lz4 variant heap-small: changes_code=true code_differs=src/lib/lz4.o,src/lib/lz4frame.o
mbedtls variant suite-b: changes_code=true code_differs=src/library/aes.o,src/library/aria.o,src/library/base64.o,src/library/bignum.o,src/library/camellia.o,src/library/ccm.o,src/library/chacha20.o,src/library/chachapoly.o,src/library/cipher.o,src/library/cipher_wrap.o,src/library/cmac.o,src/library/constant_time.o,src/library/ctr_drbg.o,src/library/debug.o,src/library/des.o,src/library/dhm.o,src/library/ecdsa.o,src/library/ecjpake.o,src/library/ecp.o,src/library/ecp_curves.o,src/library/entropy.o,src/library/error.o,src/library/gcm.o,src/library/hkdf.o,src/library/hmac_drbg.o,src/library/lmots.o,src/library/lms.o,src/library/md.o,src/library/md5.o,src/library/mps_reader.o,src/library/nist_kw.o,src/library/oid.o,src/library/pem.o,src/library/pk.o,src/library/pk_wrap.o,src/library/pkcs12.o,src/library/pkcs5.o,src/library/pkcs7.o,src/library/pkparse.o,src/library/pkwrite.o,src/library/platform.o,src/library/poly1305.o,src/library/psa_crypto.o,src/library/psa_crypto_aead.o,src/library/psa_crypto_cipher.o,src/library/psa_crypto_client.o,src/library/psa_crypto_driver_wrappers_no_static.o,src/library/psa_crypto_ecp.o,src/library/psa_crypto_ffdh.o,src/library/psa_crypto_hash.o,src/library/psa_crypto_mac.o,src/library/psa_crypto_pake.o,src/library/psa_crypto_random.o,src/library/psa_crypto_rsa.o,src/library/psa_crypto_slot_management.o,src/library/psa_util.o,src/library/ripemd160.o,src/library/rsa.o,src/library/rsa_alt_helpers.o,src/library/sha1.o,src/library/sha256.o,src/library/sha3.o,src/library/sha512.o,src/library/ssl_cache.o,src/library/ssl_ciphersuites.o,src/library/ssl_client.o,src/library/ssl_cookie.o,src/library/ssl_debug_helpers_generated.o,src/library/ssl_msg.o,src/library/ssl_ticket.o,src/library/ssl_tls.o,src/library/ssl_tls12_client.o,src/library/ssl_tls12_server.o,src/library/ssl_tls13_client.o,src/library/ssl_tls13_generic.o,src/library/ssl_tls13_keys.o,src/library/ssl_tls13_server.o,src/library/version.o,src/library/version_features.o,src/library/x509.o,src/library/x509_create.o,src/library/x509_crl.o,src/library/x509_crt.o,src/library/x509_csr.o,src/library/x509write.o,src/library/x509write_crt.o,src/library/x509write_csr.o
microcoap variant release: changes_code=true code_differs=src/coap.o
nanopb variant buffer-only: changes_code=true code_differs=build/CMakeFiles/protobuf-nanopb-static.dir/pb_decode.c.obj,build/CMakeFiles/protobuf-nanopb-static.dir/pb_encode.c.obj
nimble variant peripheral: changes_code=true code_differs=src/nimble/host/src/ble_att_clt.o,src/nimble/host/src/ble_gap.o,src/nimble/host/src/ble_gattc.o
qcbor variant no-hw-float: changes_code=true code_differs=src/src/ieee754.o,src/src/qcbor_decode.o,src/src/qcbor_encode.o
tinycbor variant no-float: changes_code=true code_differs=build/CMakeFiles/tinycbor.dir/src/cborpretty.c.obj,build/CMakeFiles/tinycbor.dir/src/cborvalidation.c.obj
zcbor variant stop-on-error: changes_code=true code_differs=build/CMakeFiles/pet.dir/src/pet_decode.c.obj,build/CMakeFiles/pet.dir/src/pet_encode.c.obj
zlib variant no-gzip: changes_code=true code_differs=build/CMakeFiles/zlibstatic.dir/deflate.c.obj,build/CMakeFiles/zlibstatic.dir/inflate.c.obj,build/CMakeFiles/zlibstatic.dir/zutil.c.obj
```

## testbed-replay (1 annotation part(s))

```text
step: replay
cjson/default           identical
cjson/no-locale         identical
corejson/default        identical
corejson/depth-8        identical
coremqtt/default        identical
coremqtt/sub-vectors-2  identical
httpparser/default      identical
httpparser/strict       identical
isotp/default           identical
libfdt/assume-valid     identical
libfdt/default          identical
lz4/default             identical
lz4/heap-small          identical
mbedtls/default         identical
mbedtls/suite-b         identical
microcoap/default       identical
microcoap/release       identical
nanopb/buffer-only      identical
nanopb/default          identical
nimble/default          identical
nimble/peripheral       identical
paho/default            identical
paho/server             dropped (entry_mismatch)
qcbor/default           identical
qcbor/no-hw-float       identical
tinycbor/default        identical
tinycbor/no-float       identical
zcbor/default           identical
zcbor/stop-on-error     identical
zlib/default            identical
zlib/no-gzip            identical
result: passed (31 rows)
exit code: 0
--- objects
cjson/default verdict=identical objects=1 normalisation_applied=false
cjson/no-locale verdict=identical objects=1 normalisation_applied=false
corejson/default verdict=identical objects=1 normalisation_applied=false
corejson/depth-8 verdict=identical objects=1 normalisation_applied=false
coremqtt/default verdict=identical objects=6 normalisation_applied=false
coremqtt/sub-vectors-2 verdict=identical objects=6 normalisation_applied=false
httpparser/default verdict=identical objects=1 normalisation_applied=false
httpparser/strict verdict=identical objects=1 normalisation_applied=false
isotp/default verdict=identical objects=1 normalisation_applied=false
libfdt/assume-valid verdict=identical objects=10 normalisation_applied=false
libfdt/default verdict=identical objects=10 normalisation_applied=false
lz4/default verdict=identical objects=5 normalisation_applied=false
lz4/heap-small verdict=identical objects=5 normalisation_applied=false
mbedtls/default verdict=identical objects=113 normalisation_applied=false
mbedtls/suite-b verdict=identical objects=113 normalisation_applied=false
microcoap/default verdict=identical objects=2 normalisation_applied=false
microcoap/release verdict=identical objects=2 normalisation_applied=false
nanopb/buffer-only verdict=identical objects=3 normalisation_applied=false
nanopb/default verdict=identical objects=3 normalisation_applied=false
nimble/default verdict=identical objects=65 normalisation_applied=false
nimble/peripheral verdict=identical objects=65 normalisation_applied=false
paho/default verdict=identical objects=7 normalisation_applied=false
qcbor/default verdict=identical objects=5 normalisation_applied=false
qcbor/no-hw-float verdict=identical objects=5 normalisation_applied=false
tinycbor/default verdict=identical objects=11 normalisation_applied=false
tinycbor/no-float verdict=identical objects=8 normalisation_applied=false
zcbor/default verdict=identical objects=6 normalisation_applied=false
zcbor/stop-on-error verdict=identical objects=6 normalisation_applied=false
zlib/default verdict=identical objects=15 normalisation_applied=false
zlib/no-gzip verdict=identical objects=15 normalisation_applied=false
```

## testbed-determinism (1 annotation part(s))

```text
rerun capture exit code: 0
compared 74 files
excluded: stdout sha256, bytes_total and bytes_stored of the step "cmake configure"
excluded: stdout sha256, bytes_total and bytes_stored of the steps "clang ast <function>"
result: identical
compare exit code: 0
```
