# Plan 03-04 round trip 1: vectors notices

Run 40541809803 (testbed), head 68852da879ee23eef612100b48268007e3febfea, job 122322581335, read from the job annotations with `gh api` (parts joined in order, final newline dropped as the workflow's annotate helper specifies). The executor self-test, the TRACTOR sample step and, new in this plan, the reference driver build, the twice-run reference vectors (run_guest) and the rerun comparison ran in the pinned image.

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
corejson/default/JSON_Validate built driver_sha256=2f2cba61a894bb5604407eb274eccfc044c81475d542beaac186913e1fc52dcf
corejson/depth-8/JSON_SearchT no_roles
corejson/depth-8/JSON_Validate built driver_sha256=e639c76722ba0f4dba4a7d9e3c1363b1517b5e160f2e3480822138e798eb31bf
coremqtt/default/MQTT_DeserializeAck no_roles
coremqtt/default/MQTT_DeserializePublish no_roles
coremqtt/sub-vectors-2/MQTT_DeserializeAck no_roles
coremqtt/sub-vectors-2/MQTT_DeserializePublish no_roles
httpparser/default/http_parser_execute no_roles
httpparser/default/http_parser_parse_url no_roles
httpparser/strict/http_parser_execute no_roles
httpparser/strict/http_parser_parse_url no_roles
isotp/default/isotp_on_can_message no_roles
libfdt/default/fdt_check_full built driver_sha256=629b41d001fdbdb69d9060a0488c312debc3fc90e33c54c9713512fd66bb5ff3
libfdt/assume-valid/fdt_check_full built driver_sha256=bb4d4315883797ae5377c2a530085e1fe61ea19c53c030ece5f7960e18546c61
lz4/default/LZ4F_getFrameInfo no_roles
lz4/default/LZ4_decompress_safe built driver_sha256=5ba1fa091514e9427cf8cbf19933c2269f093c72f5bf779782c2041cae1da896
lz4/heap-small/LZ4F_getFrameInfo no_roles
lz4/heap-small/LZ4_decompress_safe built driver_sha256=8ffaaaf948a73c25ebea6d4ce3e6f7d2534ff4a9f8be9144e7c81a2f84ff214d
mbedtls/default/mbedtls_asn1_get_tag no_roles
mbedtls/default/mbedtls_x509_crt_parse_der no_roles
mbedtls/suite-b/mbedtls_asn1_get_tag no_roles
mbedtls/suite-b/mbedtls_x509_crt_parse_der no_roles
microcoap/default/coap_parse no_roles
microcoap/release/coap_parse no_roles
nanopb/default/pb_decode_ex no_roles
nanopb/default/pb_istream_from_buffer no_roles
nanopb/buffer-only/pb_decode_ex no_roles
nanopb/buffer-only/pb_istream_from_buffer no_roles
nimble/default/ble_hs_adv_parse_fields no_roles
nimble/default/ble_uuid_init_from_buf no_roles
nimble/peripheral/ble_hs_adv_parse_fields no_roles
nimble/peripheral/ble_uuid_init_from_buf no_roles
paho/default/MQTTDeserialize_connack no_roles
paho/default/MQTTDeserialize_publish no_roles
qcbor/default/QCBORDecode_Init no_roles
qcbor/no-hw-float/QCBORDecode_Init no_roles
tinycbor/default/cbor_parser_init no_roles
tinycbor/default/cbor_value_validate no_roles
tinycbor/no-float/cbor_parser_init no_roles
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
microcoap/default/coap_parse no_roles
microcoap/release/coap_parse no_roles
nanopb/default/pb_decode_ex no_roles
nanopb/default/pb_istream_from_buffer no_roles
nanopb/buffer-only/pb_decode_ex no_roles
nanopb/buffer-only/pb_istream_from_buffer no_roles
nimble/default/ble_hs_adv_parse_fields no_roles
nimble/default/ble_uuid_init_from_buf no_roles
nimble/peripheral/ble_hs_adv_parse_fields no_roles
nimble/peripheral/ble_uuid_init_from_buf no_roles
paho/default/MQTTDeserialize_connack no_roles
paho/default/MQTTDeserialize_publish no_roles
qcbor/default/QCBORDecode_Init no_roles
qcbor/no-hw-float/QCBORDecode_Init no_roles
tinycbor/default/cbor_parser_init no_roles
tinycbor/default/cbor_value_validate no_roles
tinycbor/no-float/cbor_parser_init no_roles
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
identical 374 files
result: passed (1 rows)
check exit code: 0
exit code: 0
```
