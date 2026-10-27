# Plan 03-05 round trip 1: vectors notices

Run 40550615693 (testbed), head 68dbd0684189790d282bf70c17afaf9706b13290, job 122353562262, read from the job annotations with `gh api` (parts joined in order, final newline dropped as the workflow's annotate helper specifies). The executor self-test, the TRACTOR sample step, the reference driver build, the state layouts (new in this plan), the twice-run reference vectors (run_guest) and the rerun comparison ran in the pinned image.

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
cjson/default/cJSON_ParseWithLength no_roles
cjson/no-locale/cJSON_ParseWithLength no_roles
corejson/default/JSON_SearchT no_roles
corejson/default/JSON_Validate built driver_sha256=404f91ddbcf2f729f81e584cdf40525be74a5755de93bfbd70a2b27845c9d8f9
corejson/depth-8/JSON_SearchT no_roles
corejson/depth-8/JSON_Validate built driver_sha256=df2abd975008148cfca3fec93c4b8662b10317b4e722f283e8532f156ccda1c9
coremqtt/default/MQTT_DeserializeAck no_roles
coremqtt/default/MQTT_DeserializePublish no_roles
coremqtt/sub-vectors-2/MQTT_DeserializeAck no_roles
coremqtt/sub-vectors-2/MQTT_DeserializePublish no_roles
httpparser/default/http_parser_execute no_roles
httpparser/default/http_parser_parse_url no_roles
httpparser/strict/http_parser_execute no_roles
httpparser/strict/http_parser_parse_url no_roles
isotp/default/isotp_on_can_message no_roles
libfdt/default/fdt_check_full built driver_sha256=462111ba852f87d043d086a42f36f7be5d53dbdfae93a992d26f8a3d76e738a9
libfdt/assume-valid/fdt_check_full built driver_sha256=e35d6644f859c22fa447d44449a84b878380b9085a8a1a33a9091912b366afe0
lz4/default/LZ4F_getFrameInfo no_roles
lz4/default/LZ4_decompress_safe built driver_sha256=5159c358fec737cadb2738d1bbbe4caae672bc7f6813f0635f9701251864077e
lz4/heap-small/LZ4F_getFrameInfo no_roles
lz4/heap-small/LZ4_decompress_safe built driver_sha256=97be6c9ebb0a11823a815026fa8bb997728e1ec023f19be6fa97e9b8f414d889
mbedtls/default/mbedtls_asn1_get_tag no_roles
mbedtls/default/mbedtls_x509_crt_parse_der no_roles
mbedtls/suite-b/mbedtls_asn1_get_tag no_roles
mbedtls/suite-b/mbedtls_x509_crt_parse_der no_roles
microcoap/default/coap_parse built driver_sha256=7b41a4da73f49a1b620e7012dea47b88a4722bb7b87345dde3af2c2db82d488e
microcoap/release/coap_parse built driver_sha256=71b0469ab097e40128a689bf5ced2b49f058f4a1fb2b042b111215c65a493f71
nanopb/default/pb_decode_ex no_roles
nanopb/default/pb_istream_from_buffer no_roles
nanopb/buffer-only/pb_decode_ex no_roles
nanopb/buffer-only/pb_istream_from_buffer no_roles
nimble/default/ble_hs_adv_parse_fields no_roles
nimble/default/ble_uuid_init_from_buf built driver_sha256=b3aea1c5eec3891ffdf29f809be55b681d0d006c49d10fc7602e2016d459e57c
nimble/peripheral/ble_hs_adv_parse_fields no_roles
nimble/peripheral/ble_uuid_init_from_buf built driver_sha256=b3aea1c5eec3891ffdf29f809be55b681d0d006c49d10fc7602e2016d459e57c
paho/default/MQTTDeserialize_connack no_roles
paho/default/MQTTDeserialize_publish no_roles
qcbor/default/QCBORDecode_Init no_roles
qcbor/no-hw-float/QCBORDecode_Init no_roles
tinycbor/default/cbor_parser_init built driver_sha256=ef7ae0d05665d7894101bce42d952ec844ad52be10f9f61245d128dc55d9b41d
tinycbor/default/cbor_value_validate no_roles
tinycbor/no-float/cbor_parser_init built driver_sha256=ef7ae0d05665d7894101bce42d952ec844ad52be10f9f61245d128dc55d9b41d
tinycbor/no-float/cbor_value_validate no_roles
zcbor/default/cbor_decode_Pet no_roles
zcbor/default/zcbor_new_decode_state no_roles
zcbor/stop-on-error/cbor_decode_Pet no_roles
zcbor/stop-on-error/zcbor_new_decode_state no_roles
zlib/default/inflate no_roles
zlib/default/uncompress no_roles
zlib/no-gzip/inflate no_roles
zlib/no-gzip/uncompress no_roles
result: passed (51 rows)
exit code: 0
```

## vectors-layout (1 annotation part(s))

```text
microcoap/default/coap_parse pkt coap_packet_t size=220 hdr@0:6 hdr.ver@0:1 hdr.t@1:1 hdr.tkl@2:1 hdr.code@3:1 hdr.id@4:2 tok@8:8 tok.p@8:4 tok.len@12:4 numopts@16:1 opts@20:192 opts[].num@20:1 opts[].buf@24:8 opts[].buf.p@24:4 opts[].buf.len@28:4 payload@212:8 payload.p@212:4 payload.len@216:4
microcoap/release/coap_parse pkt coap_packet_t size=220 hdr@0:6 hdr.ver@0:1 hdr.t@1:1 hdr.tkl@2:1 hdr.code@3:1 hdr.id@4:2 tok@8:8 tok.p@8:4 tok.len@12:4 numopts@16:1 opts@20:192 opts[].num@20:1 opts[].buf@24:8 opts[].buf.p@24:4 opts[].buf.len@28:4 payload@212:8 payload.p@212:4 payload.len@216:4
nimble/default/ble_uuid_init_from_buf uuid ble_uuid_any_t size=20 u@0:1 u.type@0:1 u16@0:4 u16.u@0:1 u16.u.type@0:1 u16.value@2:2 u32@0:8 u32.u@0:1 u32.u.type@0:1 u32.value@4:4 u128@0:17 u128.u@0:1 u128.u.type@0:1 u128.value@1:16
nimble/peripheral/ble_uuid_init_from_buf uuid ble_uuid_any_t size=20 u@0:1 u.type@0:1 u16@0:4 u16.u@0:1 u16.u.type@0:1 u16.value@2:2 u32@0:8 u32.u@0:1 u32.u.type@0:1 u32.value@4:4 u128@0:17 u128.u@0:1 u128.u.type@0:1 u128.value@1:16
tinycbor/default/cbor_parser_init it CborValue size=16 parser@0:4 source@4:4 source.ptr@4:4 source.token@4:4 remaining@8:4 extra@12:2 type@14:1 flags@15:1
tinycbor/default/cbor_parser_init parser CborParser size=8 source@0:4 source.end@0:4 source.ops@0:4 flags@4:1
tinycbor/no-float/cbor_parser_init it CborValue size=16 parser@0:4 source@4:4 source.ptr@4:4 source.token@4:4 remaining@8:4 extra@12:2 type@14:1 flags@15:1
tinycbor/no-float/cbor_parser_init parser CborParser size=8 source@0:4 source.end@0:4 source.ops@0:4 flags@4:1
result: passed (8 rows)
exit code: 0
```

## vectors-run (1 annotation part(s))

```text
cjson/default/cJSON_ParseWithLength no_roles
cjson/no-locale/cJSON_ParseWithLength no_roles
corejson/default/JSON_SearchT no_roles
corejson/default/JSON_Validate vectors=108 ok=108 fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical
corejson/depth-8/JSON_SearchT no_roles
corejson/depth-8/JSON_Validate vectors=108 ok=108 fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical
coremqtt/default/MQTT_DeserializeAck no_roles
coremqtt/default/MQTT_DeserializePublish no_roles
coremqtt/sub-vectors-2/MQTT_DeserializeAck no_roles
coremqtt/sub-vectors-2/MQTT_DeserializePublish no_roles
httpparser/default/http_parser_execute no_roles
httpparser/default/http_parser_parse_url no_roles
httpparser/strict/http_parser_execute no_roles
httpparser/strict/http_parser_parse_url no_roles
isotp/default/isotp_on_can_message no_roles
libfdt/default/fdt_check_full vectors=16 ok=16 fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical
libfdt/assume-valid/fdt_check_full vectors=16 ok=16 fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical
lz4/default/LZ4F_getFrameInfo no_roles
lz4/default/LZ4_decompress_safe vectors=60 ok=60 fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical
lz4/heap-small/LZ4F_getFrameInfo no_roles
lz4/heap-small/LZ4_decompress_safe vectors=60 ok=60 fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical
mbedtls/default/mbedtls_asn1_get_tag no_roles
mbedtls/default/mbedtls_x509_crt_parse_der no_roles
mbedtls/suite-b/mbedtls_asn1_get_tag no_roles
mbedtls/suite-b/mbedtls_x509_crt_parse_der no_roles
microcoap/default/coap_parse vectors=16 ok=16 fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical
microcoap/release/coap_parse vectors=16 ok=16 fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical
nanopb/default/pb_decode_ex no_roles
nanopb/default/pb_istream_from_buffer no_roles
nanopb/buffer-only/pb_decode_ex no_roles
nanopb/buffer-only/pb_istream_from_buffer no_roles
nimble/default/ble_hs_adv_parse_fields no_roles
nimble/default/ble_uuid_init_from_buf vectors=200 ok=200 fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical
nimble/peripheral/ble_hs_adv_parse_fields no_roles
nimble/peripheral/ble_uuid_init_from_buf vectors=200 ok=200 fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical
paho/default/MQTTDeserialize_connack no_roles
paho/default/MQTTDeserialize_publish no_roles
qcbor/default/QCBORDecode_Init no_roles
qcbor/no-hw-float/QCBORDecode_Init no_roles
tinycbor/default/cbor_parser_init vectors=200 ok=200 fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical
tinycbor/default/cbor_value_validate no_roles
tinycbor/no-float/cbor_parser_init vectors=200 ok=200 fault=0 abort=0 timeout=0 setup_failed=0 skipped=0 twice=identical
tinycbor/no-float/cbor_value_validate no_roles
zcbor/default/cbor_decode_Pet no_roles
zcbor/default/zcbor_new_decode_state no_roles
zcbor/stop-on-error/cbor_decode_Pet no_roles
zcbor/stop-on-error/zcbor_new_decode_state no_roles
zlib/default/inflate no_roles
zlib/default/uncompress no_roles
zlib/no-gzip/inflate no_roles
zlib/no-gzip/uncompress no_roles
result: passed (51 rows)
exit code: 0
```

## vectors-determinism (1 annotation part(s))

```text
rerun exit code: 0
rerun rows are identical to the first run's rows
identical 1212 files
result: passed (1 rows)
check exit code: 0
exit code: 0
```
