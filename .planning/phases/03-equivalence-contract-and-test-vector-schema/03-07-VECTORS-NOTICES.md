# Plan 03-07 round trip 1: vectors notices

Run 40905688514 (testbed), head 8f2c365a1423428f90620af21f58c8c3710c0356, job 123560448033, read from the job annotations with `gh api` (parts joined in order, final newline dropped as the workflow's annotate helper specifies). The executor self-test, the TRACTOR sample step, the reference driver build, the state layouts, the twice-run reference vectors (run_guest) and the rerun comparison ran in the pinned image. The listings omit the rows of entries without roles and count them in one `no_roles: <n>` line (the job may have 50 annotations; this run has exactly 50).

## vectors-selftest (1 annotation part(s))

```text
tool qemu-arm qemu-arm version 10.0.2 (Debian 1:10.0.2+ds-2+deb13u1~bpo12+1)
tool arm-none-eabi-gcc arm-none-eabi-gcc (Arm GNU Toolchain 14.3.Rel1 (Build arm-14.174)) 14.3.1 20250623
tool arm-none-eabi-ar GNU ar (Arm GNU Toolchain 14.3.Rel1 (Build arm-14.174)) 2.44.0.20250616
tool arm-none-eabi-nm GNU nm (Arm GNU Toolchain 14.3.Rel1 (Build arm-14.174)) 2.44.0.20250616
tool llvm-dwarfdump Debian LLVM version 16.0.6
cpu cortex-m4 listed
case ok outcome=ok input_len=5
case overrun-input outcome=fault rc=-11 label=crash:SIGSEGV
case overrun-output outcome=fault rc=-11 label=crash:SIGSEGV
case slack-write outcome=fault rc=0 label=oob-write:guard-slack
case abort outcome=abort rc=-6 label=crash:SIGABRT
case loop outcome=timeout rc=-9 label=timeout
case beyond-reserve outcome=executor_error exit=98
case stack-overflow outcome=fault rc=-11 label=crash:SIGSEGV
no core file after overrun
determinism 100 runs 1 hash
dwarf enum byte_size 1
result: passed (17 rows)
exit code: 0
```

## vectors-tractor (1 annotation part(s))

```text
git init ok
git fetch ok
git sparse-checkout ok
git checkout ok
git rev-parse ok
fetched 6 pinned files into /work/tractor/37960ee08a7c644ee66e29855eca33903f702e67/tree
Public-Tests/B01_synthetic/001_helloworld_lib/test_vectors/test1.json sha256=ok keys=lib_state_in,lib_state_out,stdout round_trip=json-equal has_ub=absent
Public-Tests/B01_synthetic/007_errno_pow_lib/test_vectors/test11.json sha256=ok keys=lib_state_in,lib_state_out,stderr round_trip=json-equal has_ub=absent
Public-Tests/B01_synthetic/030_mutable_buffer_overlap_extrahard_lib/test_vectors/test04.json sha256=ok keys=has_ub,lib_state_in,lib_state_out round_trip=json-equal has_ub=present
Public-Tests/B01_synthetic/009_stack_buffer_overflow_lib/test_vectors/test09_overflow_int2.json sha256=ok keys=argv,has_ub,lib_state_in,lib_state_out,rc round_trip=json-equal has_ub=present
Public-Tests/B01_synthetic/002_stdin_echo/test_vectors/nul.json sha256=ok keys=stdin,stdout round_trip=json-equal has_ub=absent
Public-Tests/B02_synthetic/strcpy/test_vectors/41.json sha256=ok keys=argv,has_ub,rc,stderr,stdin,stdout round_trip=json-equal has_ub=present
loaded: 6 of 6 (at least 3 required)
tractor: passed
exit code: 0
```

## vectors-build (1 annotation part(s))

```text
cjson/default/cJSON_ParseWithLength built driver_sha256=a44ea8067af3cdfe6b0270a1cb99335f02fea71db19c75098192692263939638
cjson/no-locale/cJSON_ParseWithLength built driver_sha256=e54555823710ec3c7fd6c393cd561e9f937e39ca9afa710c5e9784a79f63b3f6
corejson/default/JSON_SearchT built driver_sha256=08e58b7bd5c3aa8fe6c3a9b1dc5b6e5717bf265d0a831977ad59ae78befb156e
corejson/default/JSON_Validate built driver_sha256=3cb811ffaf762a82e0e355e34412925109f1e0cc5efc7d262040fa3a1cdb6977
corejson/depth-8/JSON_SearchT built driver_sha256=86194ffccafc023fae5657f0b34217d285b4c8d515acc44ed7fb5c9d1d20cbe7
corejson/depth-8/JSON_Validate built driver_sha256=bc241d52ed7fbd0233b059baa0f4fdf2b8d434a66447606cdb2be4fba4fef02f
httpparser/default/http_parser_execute built driver_sha256=a2ca31aa78ce5d5670e71b2b53f89d7630bc4a6deb325b2bdc4dcee5349c653f
httpparser/strict/http_parser_execute built driver_sha256=c9ce7b6d6f0eaf484ab482545ef3ae90334413d6839579d219be523cf24e1814
libfdt/default/fdt_check_full built driver_sha256=40dd08091048e9fcc6f25bc70a1d7a042b656ecfe70b60bc3cbbd3bb7fc85106
libfdt/assume-valid/fdt_check_full built driver_sha256=4f3c6ac435844080339cc6aa2fdc9ccb8134dc2611e6cd9575a4d22fb53c949e
lz4/default/LZ4_decompress_safe built driver_sha256=613f2bf85ca3825a3451a21f85b800bd72e50118657ef037b1c5eafa5946a882
lz4/heap-small/LZ4_decompress_safe built driver_sha256=eb61defd48fc193a024ab6c5fbbff65dc9baf43c179e4380fedd6b28132c8b52
microcoap/default/coap_parse built driver_sha256=da96b944f0230505a3259be8bc8b9ed974707a0498e87aafd234fa0fe01f0bbd
microcoap/release/coap_parse built driver_sha256=11ff38746baf3efa10322884ab7a657b11657ee959169e4b94a5b75f80f8ce18
nanopb/default/pb_istream_from_buffer built driver_sha256=5b6563b2dc48f46351328d4d319eb05454dcb601cfbe58cd24593af17b59b0f3
nanopb/buffer-only/pb_istream_from_buffer built driver_sha256=6b2bcb56d87f93fab8b61e73aada082c166b838d317ff40030023041b520e63b
nimble/default/ble_uuid_init_from_buf built driver_sha256=81d5815a00d12815d04e5d2db1e56b63819dd45de5dbfab09f055eff46fe24db
nimble/peripheral/ble_uuid_init_from_buf built driver_sha256=81d5815a00d12815d04e5d2db1e56b63819dd45de5dbfab09f055eff46fe24db
paho/default/MQTTDeserialize_connack built driver_sha256=97070df14ffccd46129d785a301dbeb3dfd8dbea648203210244a3b8bc62a2bd
paho/default/MQTTDeserialize_publish built driver_sha256=a3cf0665cc283d78efd1a6f157e4fa13ac207e292344c21d0971da9ba9ff2c62
qcbor/default/QCBORDecode_Init built driver_sha256=4e620367282724ef95d239f482415a02144ae2b628e52adef6f36967d367bf2d
qcbor/no-hw-float/QCBORDecode_Init built driver_sha256=d8a16efb4becd6ed75bd3acb1b85871825a29d23aede515cb25f9875fda41919
tinycbor/default/cbor_parser_init built driver_sha256=19829cd87276fb1380cb3760fb61d731d7cd845a65b04a6da145bfbd9081ccad
tinycbor/default/cbor_value_validate built driver_sha256=8f2807b1d686045fc03acf82ec0a84da5d04f82548f49632e2b96d1d3cd7c661
tinycbor/no-float/cbor_parser_init built driver_sha256=19829cd87276fb1380cb3760fb61d731d7cd845a65b04a6da145bfbd9081ccad
tinycbor/no-float/cbor_value_validate built driver_sha256=148398799dee785966fc9102296f1aa33d469af9315f6edd459a01bcf61b1a06
zcbor/default/zcbor_new_decode_state built driver_sha256=860f00f70e9aab11facbd126b9696dcaa7efd137037624203d4f442f81664da9
zcbor/stop-on-error/zcbor_new_decode_state built driver_sha256=ec89b3af9a65180a4123fdc6f62dc24cf600c296ec96063c73939d42ed75a8fd
zlib/default/uncompress built driver_sha256=af93efbaf0a1fab7f4420c9e668975d1e6af5159adcb1a3066bbc9e3b85d56d4
zlib/no-gzip/uncompress built driver_sha256=1ca6bfdc078d5762eb87d3b668ac6323996eefb9eacd3e3e329fe8d545df42db
no_roles: 21
result: passed (51 rows)
exit code: 0
```

## vectors-layout (2 annotation part(s))

```text
cjson/default/cJSON_ParseWithLength ret cJSON * size=4
cjson/no-locale/cJSON_ParseWithLength ret cJSON * size=4
corejson/default/JSON_SearchT outType JSONTypes_t size=1
corejson/default/JSON_SearchT outValue char * size=4
corejson/default/JSON_SearchT outValueLength size_t size=4
corejson/depth-8/JSON_SearchT outType JSONTypes_t size=1
corejson/depth-8/JSON_SearchT outValue char * size=4
corejson/depth-8/JSON_SearchT outValueLength size_t size=4
httpparser/default/http_parser_execute parser http_parser size=32 type@0:4:0:2 flags@0:4:2:8 state@1:4:10:7 header_state@2:4:17:7 index@3:4:24:5 extra_flags@3:4:29:2 lenient_http_headers@3:4:31:1 nread@4:4 content_length@8:8 http_major@16:2 http_minor@18:2 status_code@20:4:160:16 method@22:4:176:8 http_errno@23:4:184:7 upgrade@23:4:191:1 data@24:4
httpparser/default/http_parser_execute settings http_parser_settings size=40 on_message_begin@0:4 on_url@4:4 on_status@8:4 on_header_field@12:4 on_header_value@16:4 on_headers_complete@20:4 on_body@24:4 on_message_complete@28:4 on_chunk_header@32:4 on_chunk_complete@36:4
httpparser/strict/http_parser_execute parser http_parser size=32 type@0:4:0:2 flags@0:4:2:8 state@1:4:10:7 header_state@2:4:17:7 index@3:4:24:5 extra_flags@3:4:29:2 lenient_http_headers@3:4:31:1 nread@4:4 content_length@8:8 http_major@16:2 http_minor@18:2 status_code@20:4:160:16 method@22:4:176:8 http_errno@23:4:184:7 upgrade@23:4:191:1 data@24:4
httpparser/strict/http_parser_execute settings http_parser_settings size=40 on_message_begin@0:4 on_url@4:4 on_status@8:4 on_header_field@12:4 on_header_value@16:4 on_headers_complete@20:4 on_body@24:4 on_message_complete@28:4 on_chunk_header@32:4 on_chunk_complete@36:4
microcoap/default/coap_parse pkt coap_packet_t size=220 hdr@0:6 hdr.ver@0:1 hdr.t@1:1 hdr.tkl@2:1 hdr.code@3:1 hdr.id@4:2 tok@8:8 tok.p@8:4 tok.len@12:4 numopts@16:1 opts@20:192 opts[].num@20:1 opts[].buf@24:8 opts[].buf.p@24:4 opts[].buf.len@28:4 payload@212:8 payload.p@212:4 payload.len@216:4
microcoap/release/coap_parse pkt coap_packet_t size=220 hdr@0:6 hdr.ver@0:1 hdr.t@1:1 hdr.tkl@2:1 hdr.code@3:1 hdr.id@4:2 tok@8:8 tok.p@8:4 tok.len@12:4 numopts@16:1 opts@20:192 opts[].num@20:1 opts[].buf@24:8 opts[].buf.p@24:4 opts[].buf.len@28:4 payload@212:8 payload.p@212:4 payload.len@216:4
nanopb/buffer-only/pb_istream_from_buffer ret pb_istream_t size=12 callback@0:4 state@4:4 bytes_left@8:4
nanopb/default/pb_istream_from_buffer ret pb_istream_t size=16 callback@0:4 state@4:4 bytes_left@8:4 errmsg@12:4
nimble/default/ble_uuid_init_from_buf uuid ble_uuid_any_t size=20 u@0:1 u.type@0:1 u16@0:4 u16.u@0:1 u16.u.type@0:1 u16.value@2:2 u32@0:8 u32.u@0:1 u32.u.type@0:1 u32.value@4:4 u128@0:17 u128.u@0:1 u128.u.type@0:1 u128.value@1:16
nimble/peripheral/ble_uuid_init_from_buf uuid ble_uuid_any_t size=20 u@0:1 u.type@0:1 u16@0:4 u16.u@0:1 u16.u.type@0:1 u16.value@2:2 u32@0:8 u32.u@0:1 u32.u.type@0:1 u32.value@4:4 u128@0:17 u128.u@0:1 u128.u.type@0:1 u128.value@1:16
paho/default/MQTTDeserialize_connack connack_rc unsigned char size=1
paho/default/MQTTDeserialize_connack sessionPresent unsigned char size=1
paho/default/MQTTDeserialize_publish dup unsigned char size=1
paho/default/MQTTDeserialize_publish packetid short unsigned int size=2
paho/default/MQTTDeserialize_publish payload unsigned char * size=4
paho/default/MQTTDeserialize_publish payloadlen int size=4
paho/default/MQTTDeserialize_publish qos int size=4
paho/default/MQTTDeserialize_publish retained unsigned char size=1
paho/default/MQTTDeserialize_publish topicName MQTTString size=12 cstring@0:4 lenstring@4:8 lenstring.len@4:4 lenstring.data@8:4
qcbor/default/QCBORDecode_Init ctx QCBORDecodeContext size=280 InBuf@0:16 InBuf.UB@0:8 InBuf.UB.ptr@0:4 InBuf.UB.len@4:4 InBuf.cursor@8:4 InBuf.magic@12:2 InBuf.err@14:1 nesting@16:200 nesting.pLevels@16:192 nesting.pLevels[].uLevelType@16:1 nesting.pLevels[].u@20:8 nesting.pLevels[].u.ma@20:8 nesting.pLevels[].u.ma.uCountTotal@20:2 nesting.pLevels[].u.ma.uCountCursor@22:2 nesting.pLevels[].u.ma.uStartOffset@24:4 nesting.pLevels[].u.bs@20:8 nesting.pLevels[].u.bs.uSavedEndOffset@20:4 nesting.pLevels[].u.bs.uBstrStartOffset@24:4 nesting.pCurrent@208:4 nesting.pCurrentBounded@212:4 StringAllocator@216:8 StringAllocator.pAllocateCxt@216:4 StringAllocator.pfAllocator@220:4 uMemPoolSize@224:4 uMemPoolFreeOffset@228:4 uMapEndOffsetCache@232:4 uDecodeMode@236:1 bStringAllocateAll@237:1 uLastError@238:1 auMappedTags@240:32 uLastTags@272:8
qcbor/no-hw-float/QCBORDecode_Init ctx QCBORDecodeContext size=280 InBuf@0:16 InBuf.UB@0:8 InBuf.UB.ptr@0:4 InBuf.UB.len@4:4 InBuf.cursor@8:4 InBuf.magic@12:2 InBuf.err@14:1 nesting@16:200 nesting.pLevels@16:192 nesting.pLevels[].uLevelType@16:1 nesting.pLevels[].u@20:8 nesting.pLevels[].u.ma@20:8 nesting.pLevels[].u.ma.uCountTotal@20:2 nesting.pLevels[].u.ma.uCountCursor@22:2 nesting.pLevels[].u.ma.uStartOffset@24:4 nesting.pLevels[].u.bs@20:8 nesting.pLevels[].u.bs.uSavedEndOffset@20:4 nesting.pLevels[].u.bs.uBstrStartOffset@24:4 nesting.pCurrent@208:4 nesting.pCurrentBounded@212:4 StringAllocator@216:8 StringAllocator.pAllocateCxt@216:4 StringAllocator.pfAllocator@220:4 uMemPoolSize@224:4 uMemPoolFreeOffset@228:4 uMapEndOffsetCache@232:4 uDecodeMode@236:1 bStringAllocateAll@237:1 uLastError@238:1 auMappedTags@240:32 uLastTags@272:8
tinycbor/default/cbor_parser_init it CborValue size=16 parser@0:4 source@4:4 source.ptr@4:4 source.token@4:4 remaining@8:4 extra@12:2 type@14:1 flags@15:1
tinycbor/default/cbor_parser_init parser CborParser size=8 source@0:4 source.end@0:4 source.ops@0:4 flags@4:1
tinycbor/default/cbor_value_validate it CborValue size=16 parser@0:4 source@4:4 source.ptr@4:4 source.token@4:4 remaining@8:4 extra@12:2 type@14:1 flags@15:1
tinycbor/default/cbor_value_validate parser CborParser size=8 source@0:4 source.end@0:4 source.ops@0:4 flags@4:1
tinycbor/no-float/cbor_parser_init it CborValue size=16 parser@0:4 source@4:4 source.ptr@4:4 source.token@4:4 remaining@8:4 extra@12:2 type@14:1 flags@15:1
tinycbor/no-float/cbor_parser_init parser CborParser size=8 source@0:4 source.end@0:4 source.ops@0:4 flags@4:1
tinycbor/no-float/cbor_value_validate it CborValue size=16 parser@0:4 source@4:4 source.ptr@4:4 source.token@4:4 remaining@8:4 extra@12:2 type@14:1 flags@15:1
tinycbor/no-float/cbor_value_validate parser CborParser size=8 source@0:4 source.end@0:4 source.ops@0:4 flags@4:1
zcbor/default/zcbor_new_decode_state states zcbor_state_t[3] size=108 []._anon0@0:4 []._anon0.payload_mut@0:4 []._anon0.payload@0:4 [].payload_bak@4:4 [].elem_count@8:4 [].payload_end@12:4 [].payload_moved@16:1 [].decode_state@20:12 [].decode_state.indefinite_length_array@20:1 [].decode_state.counting_map_elems@21:1 [].decode_state.map_elems_processed@24:4 [].decode_state.map_elem_count@28:4 [].constant_state@32:4
zcbor/stop-on-error/zcbor_new_decode_state states zcbor_state_t[3] size=108 []._anon0@0:4 []._anon0.payload_mut@0:4 []._anon0.payload@0:4 [].payload_bak@4:4 [].elem_count@8:4 [].payload_end@12:4 [].payload_moved@16:1 [].decode_state@20:12 [].decode_state.indefinite_length_array@20:1 [].decode_state.counting_map_elems@21:1 [].decode_state.map_elems_processed@24:4 [].decode_state.map_elem_count@28:4 [].constant_state@32:4
zlib/default/uncompress destLen uLongf size=4
zlib/no-gzip/uncompress destLen uLongf size=4
result: passed (41 rows)
exit code: 0
```

## vectors-run (1 annotation part(s))

```text
cjson/default/cJSON_ParseWithLength vectors=25 ok=25 fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical
cjson/no-locale/cJSON_ParseWithLength vectors=25 ok=25 fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical
corejson/default/JSON_SearchT vectors=324 ok=324 fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical
corejson/default/JSON_Validate vectors=108 ok=108 fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical
corejson/depth-8/JSON_SearchT vectors=324 ok=324 fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical
corejson/depth-8/JSON_Validate vectors=108 ok=108 fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical
httpparser/default/http_parser_execute vectors=76 ok=76 fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical
httpparser/strict/http_parser_execute vectors=76 ok=76 fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical
libfdt/default/fdt_check_full vectors=16 ok=16 fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical
libfdt/assume-valid/fdt_check_full vectors=16 ok=16 fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical
lz4/default/LZ4_decompress_safe vectors=60 ok=60 fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical
lz4/heap-small/LZ4_decompress_safe vectors=60 ok=60 fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical
microcoap/default/coap_parse vectors=16 ok=16 fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical
microcoap/release/coap_parse vectors=16 ok=16 fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical
nanopb/default/pb_istream_from_buffer vectors=200 ok=200 fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical
nanopb/buffer-only/pb_istream_from_buffer vectors=200 ok=200 fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical
nimble/default/ble_uuid_init_from_buf vectors=200 ok=200 fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical
nimble/peripheral/ble_uuid_init_from_buf vectors=200 ok=200 fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical
paho/default/MQTTDeserialize_connack vectors=17 ok=17 fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical
paho/default/MQTTDeserialize_publish vectors=17 ok=16 fault=1 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical
qcbor/default/QCBORDecode_Init vectors=600 ok=600 fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical
qcbor/no-hw-float/QCBORDecode_Init vectors=600 ok=600 fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical
tinycbor/default/cbor_parser_init vectors=200 ok=200 fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical
tinycbor/default/cbor_value_validate vectors=400 ok=340 fault=0 abort=0 timeout=0 setup_failed=60 skipped=0 twice=identical
tinycbor/no-float/cbor_parser_init vectors=200 ok=200 fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical
tinycbor/no-float/cbor_value_validate vectors=400 ok=340 fault=0 abort=0 timeout=0 setup_failed=60 skipped=0 twice=identical
zcbor/default/zcbor_new_decode_state vectors=274 ok=274 fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical
zcbor/stop-on-error/zcbor_new_decode_state vectors=274 ok=274 fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical
zlib/default/uncompress vectors=54 ok=54 fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical
zlib/no-gzip/uncompress vectors=54 ok=54 fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical
no_roles: 21
result: passed (51 rows)
exit code: 0
```

## vectors-determinism (1 annotation part(s))

```text
rerun exit code: 0
rerun rows are identical to the first run's rows
identical 5170 files
result: passed (1 rows)
check exit code: 0
exit code: 0
```
