---
phase: 03-equivalence-contract-and-test-vector-schema
reviewed: 2026-11-01T00:00:00Z
depth: standard
files_reviewed: 145
files_reviewed_list:
  - .github/workflows/container.yml
  - .github/workflows/pin-discovery.yml
  - .github/workflows/testbed.yml
  - Cargo.lock
  - Cargo.toml
  - container/Dockerfile
  - container/README.md
  - container/pins.toml
  - container/smoke/qemu-mprofile.S
  - crates/mt-capture/src/fetch.rs
  - crates/mt-capture/src/manifest.rs
  - crates/mt-capture/tests/bed_01_entries.rs
  - crates/mt-capture/tests/bed_01_entry_include_prefix.rs
  - crates/mt-capture/tests/build_02_workflow_static.rs
  - crates/mt-capture/tests/eqv_02_sparse_fetch.rs
  - crates/mt-capture/tests/eqv_03_roles.rs
  - crates/mt-cli/Cargo.toml
  - crates/mt-cli/src/main.rs
  - crates/mt-cli/tests/bed_01_testbed_cli.rs
  - crates/mt-cli/tests/eqv_01_policy_check_cli.rs
  - crates/mt-cli/tests/eqv_02_vectors_cli.rs
  - crates/mt-cli/tests/eqv_04_selftest_cli.rs
  - crates/mt-cli/tests/eqv_04_vectors_cli.rs
  - crates/mt-cli/tests/tool_01_check_end_to_end.rs
  - crates/mt-contract/Cargo.toml
  - crates/mt-contract/src/lib.rs
  - crates/mt-contract/src/policy.rs
  - crates/mt-contract/tests/eqv_01_committed_policies.rs
  - crates/mt-contract/tests/eqv_01_contract_document.rs
  - crates/mt-contract/tests/eqv_01_policy.rs
  - crates/mt-harness/Cargo.toml
  - crates/mt-harness/src/adapter.rs
  - crates/mt-harness/src/adapter_drops.rs
  - crates/mt-harness/src/budget.rs
  - crates/mt-harness/src/classify.rs
  - crates/mt-harness/src/decode.rs
  - crates/mt-harness/src/driver.rs
  - crates/mt-harness/src/exec.rs
  - crates/mt-harness/src/frame.rs
  - crates/mt-harness/src/globals.rs
  - crates/mt-harness/src/heap.rs
  - crates/mt-harness/src/layout.rs
  - crates/mt-harness/src/lib.rs
  - crates/mt-harness/src/link.rs
  - crates/mt-harness/src/probe.rs
  - crates/mt-harness/src/replay.rs
  - crates/mt-harness/src/run.rs
  - crates/mt-harness/src/runtime.c
  - crates/mt-harness/src/runtime.rs
  - crates/mt-harness/src/sample.rs
  - crates/mt-harness/src/selftest.rs
  - crates/mt-harness/src/tools.rs
  - crates/mt-harness/tests/eqv_03_adapter.rs
  - crates/mt-harness/tests/eqv_03_adapter_floor.rs
  - crates/mt-harness/tests/eqv_03_driver_golden.rs
  - crates/mt-harness/tests/eqv_04_classify.rs
  - crates/mt-harness/tests/eqv_04_committed_sample.rs
  - crates/mt-harness/tests/eqv_04_decode.rs
  - crates/mt-harness/tests/eqv_04_exec.rs
  - crates/mt-harness/tests/eqv_04_globals.rs
  - crates/mt-harness/tests/eqv_04_layout.rs
  - crates/mt-harness/tests/eqv_04_link.rs
  - crates/mt-harness/tests/eqv_04_listing.rs
  - crates/mt-harness/tests/eqv_04_selftest.rs
  - crates/mt-harness/tests/eqv_04_ties.rs
  - crates/mt-harness/tests/eqv_04_workflow_static.rs
  - crates/mt-harness/tests/fixtures/dwarf/selftest-probe-gcc14.txt
  - crates/mt-harness/tests/fixtures/golden/corejson-JSON_Validate.driver.c
  - crates/mt-harness/tests/fixtures/golden/coremqtt-set-step.driver.c
  - crates/mt-harness/tests/fixtures/golden/httpparser-http_parser_execute.driver.c
  - crates/mt-harness/tests/fixtures/golden/lz4-LZ4_decompress_safe.driver.c
  - crates/mt-harness/tests/fixtures/golden/microcoap-coap_parse.driver.c
  - crates/mt-harness/tests/fixtures/golden/zlib-uncompress.driver.c
  - crates/mt-harness/tests/fixtures/obj/globals.c
  - crates/mt-harness/tests/fixtures/obj/globals.o
  - crates/mt-toolchain/tests/check_and_manifest.rs
  - crates/mt-toolchain/tests/container_static.rs
  - crates/mt-toolchain/tests/fixtures/pins-full.toml
  - crates/mt-toolchain/tests/fixtures/tool-outputs.toml
  - crates/mt-vectors/Cargo.toml
  - crates/mt-vectors/src/bytes.rs
  - crates/mt-vectors/src/compare.rs
  - crates/mt-vectors/src/corpus.rs
  - crates/mt-vectors/src/lib.rs
  - crates/mt-vectors/src/state.rs
  - crates/mt-vectors/src/tractor.rs
  - crates/mt-vectors/src/vector.rs
  - crates/mt-vectors/tests/eqv_02_byte_codec.rs
  - crates/mt-vectors/tests/eqv_02_corpus.rs
  - crates/mt-vectors/tests/eqv_02_schema.rs
  - crates/mt-vectors/tests/eqv_02_state_and_compare.rs
  - crates/mt-vectors/tests/eqv_02_tractor.rs
  - crates/mt-vectors/tests/fixtures/own/reference.json
  - docs/specs/equivalence-contract.md
  - testbed/README.md
  - testbed/adapter-dropped.toml
  - testbed/manifest.toml
  - testbed/policy/cjson.toml
  - testbed/policy/corejson.toml
  - testbed/policy/coremqtt.toml
  - testbed/policy/httpparser.toml
  - testbed/policy/isotp.toml
  - testbed/policy/libfdt.toml
  - testbed/policy/lz4.toml
  - testbed/policy/mbedtls.toml
  - testbed/policy/microcoap.toml
  - testbed/policy/nanopb.toml
  - testbed/policy/nimble.toml
  - testbed/policy/paho.toml
  - testbed/policy/qcbor.toml
  - testbed/policy/tinycbor.toml
  - testbed/policy/zcbor.toml
  - testbed/policy/zlib.toml
  - testbed/tractor-pin.toml
  - testbed/vectors-sample/NOTICE
  - testbed/vectors-sample/cjson/default/cJSON_ParseWithLength/5f3822c8033645eebbf738676dec213ce36c16c51e5709f35b14c7b440f9387b.json
  - testbed/vectors-sample/cjson/default/cJSON_ParseWithLength/index.json
  - testbed/vectors-sample/corejson/default/JSON_SearchT/225779626e3c5b5a3c51c78b3bf446dfbb835f05e2e7dfcf409074b00b3d8b13.json
  - testbed/vectors-sample/corejson/default/JSON_SearchT/index.json
  - testbed/vectors-sample/coremqtt/default/MQTT_DeserializeAck/0086b1de16db99dcb742b355c6203cc415948769318780c69688315f1d88f574.json
  - testbed/vectors-sample/coremqtt/default/MQTT_DeserializeAck/index.json
  - testbed/vectors-sample/httpparser/default/http_parser_execute/43541203b0a83b93d70f352f99ba75d5ba03e88f84a3faca8c843986ec628ef2.json
  - testbed/vectors-sample/httpparser/default/http_parser_execute/index.json
  - testbed/vectors-sample/libfdt/default/fdt_check_full/7edc802880ed1d2d7e849a1c676c4bfc395615b8679a8037f243c79e5f134eb3.json
  - testbed/vectors-sample/libfdt/default/fdt_check_full/index.json
  - testbed/vectors-sample/lz4/default/LZ4F_getFrameInfo/15da37385df1ee2239aa8a21352b980912181b01658cc2bdc1bf652253861ee0.json
  - testbed/vectors-sample/lz4/default/LZ4F_getFrameInfo/index.json
  - testbed/vectors-sample/mbedtls/default/mbedtls_x509_crt_parse_der/16585bc6c3af0136ee3087c7ea89250eae417d55a6343538bb08cef894563751.json
  - testbed/vectors-sample/mbedtls/default/mbedtls_x509_crt_parse_der/index.json
  - testbed/vectors-sample/microcoap/default/coap_parse/35a176c074e417419725e2a474ea418126ff974e9fbc6e13a9e463c720bc989a.json
  - testbed/vectors-sample/microcoap/default/coap_parse/index.json
  - testbed/vectors-sample/nanopb/default/pb_istream_from_buffer/8296d98192f0db91872956e90eb132e7fcac3c2dee25c5ee67c983e1357001ac.json
  - testbed/vectors-sample/nanopb/default/pb_istream_from_buffer/index.json
  - testbed/vectors-sample/nimble/default/ble_uuid_init_from_buf/f39cc21adc3772693ed404e5948388483b8790aefbaa6e46268eb35c54cf0d83.json
  - testbed/vectors-sample/nimble/default/ble_uuid_init_from_buf/index.json
  - testbed/vectors-sample/paho/default/MQTTDeserialize_connack/b6d11a7d234711739ce6134e2100dafff937f1d9068884832a3a640bd6134af4.json
  - testbed/vectors-sample/paho/default/MQTTDeserialize_connack/index.json
  - testbed/vectors-sample/qcbor/default/QCBORDecode_Init/e0b4d28d39e01e0056fdb1a31f0d5815ff7ecb51ed880881a5da5db28a12cb46.json
  - testbed/vectors-sample/qcbor/default/QCBORDecode_Init/index.json
  - testbed/vectors-sample/tinycbor/default/cbor_parser_init/350922c1289bd8afc6734a55e75ed11856ea205c89991fe22e9ce29db60d4287.json
  - testbed/vectors-sample/tinycbor/default/cbor_parser_init/index.json
  - testbed/vectors-sample/zcbor/default/zcbor_new_decode_state/cbd1764d16cfd99d5923ef0d2b157ee95af4b07eacf44118071da78f6d037230.json
  - testbed/vectors-sample/zcbor/default/zcbor_new_decode_state/index.json
  - testbed/vectors-sample/zlib/default/uncompress/f86dc5bc82b056e45eebcdb0ec157f6ac33b456c320daa9cde19aabd226c1d33.json
  - testbed/vectors-sample/zlib/default/uncompress/index.json
findings:
  critical: 1
  warning: 8
  info: 7
  total: 16
status: issues_found
---

# Phase 3: Code Review Report

**Reviewed:** 2026-11-01
**Depth:** standard
**Files Reviewed:** 145
**Status:** issues_found

## Summary

The Rust sources of `mt-vectors`, `mt-contract`, `mt-harness`, `mt-cli`, `mt-capture/{fetch,manifest}.rs`, the shared `runtime.c`, the three workflows and the Dockerfile were read in full or in the load-bearing parts. Test files, fixtures, the 31 generated sample files and the binary `globals.o` were not reviewed line by line. Two committed sample vectors (zlib, mbedtls) and one index were spot-checked: nine TRACTOR fields, two `lib_state` fields, `schema_version: 1` in the index, canonical form, and all consistent with `Vector`/`CorpusIndex`.

Project-rule compliance is good. `#![forbid(unsafe_code)]` is present in every tool crate. `std::process::Command` appears only in `mt-toolchain/src/runner.rs`. The only `expect` outside tests carries an invariant comment and a scoped `allow`. `anyhow` is used only in `mt-cli`. There are no `HashMap`, clock or RNG uses in the reviewed sources. The words "certified" and "compliant" appear nowhere in the reviewed source or docs, and tests scan for them. Nothing reads a held-out corpus. No workflow uses `pull_request_target`, every action is SHA-pinned, and `permissions` are `contents: read` except the publish job.

The main concerns are:

- one false-pass path in the evidence itself: the return value is silently truncated to 32 bits when it is a typedef of a 64-bit integer;
- the replay check is weaker than its documentation claims, and it trusts unvalidated index fields;
- several commands exit 0 on an empty selection;
- the workflow extracts a tar produced by the untrusted-build job directly on the runner;
- the `rt_sigaction` struct in `runtime.c` does not match the kernel layout.

## Critical Issues

### CR-01: A return value that is a typedef of a 64-bit integer is silently truncated to 32 bits

**File:** `crates/mt-harness/src/run.rs:589-604` (also `crates/mt-harness/src/adapter.rs:241-249, 624`)
**Issue:** The contract (`docs/specs/equivalence-contract.md:59`) says a return wider than 32 bits goes to `lib_state_out.ret` and `rc` is 0. The code only rejects wide returns by their spelling (`is_wide_return` matches `float`, `double`, `uint64_t`, `int64_t` and two `long`s). A typedef name such as `intmax_t`, `uintmax_t`, `int_fast64_t`, `off64_t` or a project typedef falls through to `ReturnKind::Integer`.

After the DWARF resolution in `build_entry`, `Some(Kind::Int { .. } | Kind::Enum { .. }) | None => ReturnKind::Integer` is taken with no size check. The `ret` layout is then removed and the call is rendered as `(int32_t)(call)`. The upper 32 bits are dropped, so `rc` records only the low word. Because `rc` is a compared field, a Rust translation that returns a different high word still compares `Equal`. In a harness whose product is evidence, that is a silent false pass.
**Fix:** In the DWARF resolution branch, refuse (or route to `ret`) any integer or enum return larger than 4 bytes:
```rust
let kind = match layouts.states.get(RET_NAME) {
    Some(ty) if matches!(ty.kind, Kind::Int { .. } | Kind::Enum { .. }) && ty.size > 4 => {
        return Ok(BuildState::Refused(format!(
            "the return type {} is {} bytes; a 32-bit rc cannot hold it",
            adapter.return_text(), ty.size)));
    }
    Some(ty) => match &ty.kind { /* existing mapping */ },
    None => ReturnKind::Integer,
};
```
Alternatively implement the documented `ret` recording for wide integers. Add a test that uses a 64-bit typedef return.

## Warnings

### WR-01: `replay_corpus` is weaker than its documentation and trusts unvalidated index fields

**File:** `crates/mt-harness/src/replay.rs:264-359`
**Issue:** Replay reads `index.json` with plain `serde_json` and not through `read_corpus`. Several guarantees of the corpus format are therefore lost, although the CLI help and CI comments describe replay as the tamper check (T-03-17, T-03-31).

- The index key `sha` is joined into a path without validation (`dir.join(format!("{sha}.json"))`, line 323). `index.config` is joined into the build path in `load_driver` without checking it against the target's configurations. A key such as `../../x` reads a file outside the corpus.
- Nothing checks that `sha256(stored) == sha`. A vector file renamed to another index key still reproduces byte for byte and is counted `identical`.
- Provenance fields other than `driver_elf`, `driver_source`, `archive` and `objects` are never compared with the current build or policy: `policy_sha256`, `image`, `executor_version`, `contract_version`, `budgets`, `seed` and `scalars`. The re-run is driven by the vector's own `argv` and `stdin`, so an edited provenance record cannot be detected. A changed policy file is not noticed either.
- Files in the corpus directory that are not in the index (a stray `<sha>.json`, a symlink) are ignored, although `read_corpus` refuses them (T-03-13).
- `Manifest::load` is used without `validate()`.

**Fix:** Run the structural checks of `read_corpus` first, or factor them into a shared function that does not abort on the first bad vector. Concretely:

- require `is_sha256(key)`;
- require `sha256_hex(&stored) == sha`;
- require every file in the directory to be in the index and every index entry to have a file;
- compare `policy_sha256` with the current policy file and `image` with `loaded.build.image`;
- compare `seed` and `scalars` with the values derived from the vector;
- call `manifest.validate()`.

### WR-02: `mt vectors build` and `mt vectors run` report `result: passed (0 rows)` and exit 0 on an empty selection

**File:** `crates/mt-cli/src/main.rs:1086-1130` (with `print_result`, line 951)
**Issue:** `build_command` and `run_command` pass `rows.iter().any(is_failure)` to `print_result`. If the retained-pair list or the entry selection is empty (every pair dropped, ids that match no pair, a manifest with no drivable entry), `rows` is empty and the command prints `result: passed (0 rows)` and exits 0. The sibling commands `layouts` and `replay --tree` fail closed on "nothing to do". Evidence commands must not turn "nothing ran" into a pass, and the CI steps gate on this exit code. `adapters` without `--check-floor` has the same shape.
**Fix:** `failed = rows.is_empty() || rows.iter().any(..)`. For the `no_roles`-only case, require at least one row that is `Built` or `Ran`.

### WR-03: `struct mt_ksigaction` does not match the kernel `sigaction` layout (uninitialised padding becomes the signal mask)

**File:** `crates/mt-harness/src/runtime.c:257-262, 292-296`
**Issue:** The ARM kernel `struct sigaction` is `{handler, flags, restorer, sigset_t mask[2 x u32]}`, with the mask at offset 12. The driver declares `unsigned long long mask`, which AAPCS aligns to 8, so the mask sits at offset 16 and bytes 12-15 are padding. `act.mask = 0` initialises the member, not the padding. The kernel (and qemu-user) read `sa_mask.sig[0]` from the uninitialised stack padding, so the set of signals blocked while the handler runs is stack garbage. The handler only calls `exit_group`, so the observable effect is small, but the value is nondeterministic and the layout is wrong.
**Fix:** Use `unsigned long mask[2];` (offset 12, 8 bytes) and set both words to 0, or `memset(&act, 0, sizeof act)` and declare the struct `__attribute__((packed))`-free with the 32-bit mask array.

### WR-04: The vectors job unpacks a tar produced by the untrusted-build job straight onto the runner

**File:** `.github/workflows/testbed.yml:746-758, 1034-1038`
**Issue:** `work.tar` is built from `/work/capture` and `/work/run`. `/work/run/<id>/<cfg>` is the one directory the untrusted upstream build container may write, and the file is created with `tar -cf` with no filtering. In the `vectors` job, `tar -xf work.tar -C "$RUNNER_TEMP/work"` runs as the runner user on the host, with no `--no-same-owner`, no member-type check, and GNU tar's default of following symlinks created earlier in the same archive. A build that leaves `run/x/y/link -> ../..` followed by `run/x/y/link/ci-lib.sh` can overwrite `$RUNNER_TEMP/ci-lib.sh`, which every later step `source`s. The same trick can replace `$RUNNER_TEMP/bundle/mt`. The comment block says this boundary is the point of the two-container design (WR-04, WR-12); the extraction step undoes it. Exposure is limited because sources are pinned by commit and tree hash, but the workflow itself treats them as untrusted.
**Fix:** Unpack inside a container (`image_tool`-style, no network, only the destination writable). Or list the archive first and fail on any entry that is not a regular file or directory (`tar -tv | grep -v '^[-d]'`), and extract with `--no-same-owner --no-same-permissions`. Better, create the tar with a symlink-free filter on the producer side as well.

### WR-05: The "Replay the committed sample" step passes silently when the sample directory is missing

**File:** `.github/workflows/testbed.yml:1194-1206`
**Issue:** The step writes `no committed sample` and exits 0 if `testbed/vectors-sample` does not exist. The comment says this holds "while no sample is committed", but the sample is now committed and described in `testbed/vectors-sample/NOTICE` as replayed on every run. Deleting or renaming the directory turns the replay gate green. The related test (`eqv_04_committed_sample.rs`) is the only guard.
**Fix:** Fail the step (`exit 1` through `finish_vectors`) when the directory is absent. Keep the skip only behind an explicit workflow input.

### WR-06: `is_prefix_line` does not enforce "a single directive" and lets line-splicing and comment openers into the C source

**File:** `crates/mt-harness/src/driver.rs:121-131`
**Issue:** The function is documented as the second check against injection (T-03-14), but it accepts any line that starts with `#include`, `#define` or `#undef` and has no control character. A trailing backslash joins the line with the next line of the joined text (`include_lines.join("\n")`). A `/*` anywhere on a `#define` line opens a comment that runs through the template until the next `*/`, and so removes arbitrary driver code (for example the `_Static_assert`s or the call). An `#include "/abs/path"` is also accepted. The lines come from the upstream defining unit's include prefix, which is upstream source text.
**Fix:** Reject lines that end with `\`, contain `/*` or `//`, or have an `#include` operand that is not `<...>` or `"..."` with a relative path. Also assert after rendering that the number of lines equals the number of fragments.

### WR-07: Policy `error_code` values are not range-checked against the function's return type

**File:** `crates/mt-contract/src/policy.rs:372-375`
**Issue:** `ErrorValue::Integer(i64)` and `Text` holding a decimal integer are accepted with any magnitude. The contract and the rc convention are 32-bit; `return_error` returns this value from a function the harness records as an `int32_t`. A value like `4294967296` or `"-99999999999"` validates, and Phase 4/5 will then emit Rust returning a value that does not fit the C return type.
**Fix:** Require the integer to fit `i32::MIN..=u32::MAX` (the same window as `StateValue::Int`), and for the text form parse it and apply the same bound.

### WR-08: Identifier grammars for target ids and configuration names disagree between the manifest, the policy loader and the vector schema

**File:** `crates/mt-capture/src/manifest.rs:1246-1249, 2259-2264`; `crates/mt-vectors/src/vector.rs:210-231`; `crates/mt-contract/src/policy.rs:516-523`
**Issue:** The manifest accepts target ids `[a-z0-9_]+` (so `_x` and `__` pass) and configuration names `[a-z0-9-]+` (so `-x` and `--help` pass). The policy loader requires a module name `^[a-z0-9][a-z0-9_-]*$`, and the vector `argv[0]` grammar requires the target and configuration to start with `[a-z0-9]`. A manifest that validates can therefore produce targets whose policy fails `Policy::validate` and whose vectors are refused by `Vector::validate` and `write_corpus`, at the end of a long run. A leading-`-` configuration name also becomes a path component and a CLI value (`--config -x`).
**Fix:** Tighten `target_issues` and `config_issues` to the strictest common grammar (leading `[a-z0-9]`), and add a test that ties the three grammars together.

## Info

### IN-01: `MAP_FIXED` mappings silently replace whatever is already at the address

**File:** `crates/mt-harness/src/runtime.c:129, 177, 786`
**Issue:** `MT_MAP_RW_FIXED` is `MAP_PRIVATE|MAP_FIXED|MAP_ANONYMOUS`. If a target's static image (`.data`/`.bss`, the 1 MiB heap arena, the 1 MiB snapshot buffers) reaches the fixed bases from `0x04000000`, the mapping would overwrite the driver's own memory without error, giving wrong vectors rather than a driver failure (exit 98). There is also no `_Static_assert` that the input mapping (`MT_IN_BASE` plus its pages) ends below `MT_OUT_BASE`.
**Fix:** Use `MAP_FIXED_NOREPLACE` (`0x100000`) so a collision returns an error and exits 98. Add `_Static_assert(MT_IN_BASE + (MT_MAX_INPUT / MT_PAGE + 3u) * MT_PAGE <= MT_OUT_BASE, ...)`. Also bound `mt_fields[1].len` by `MT_MAX_INPUT` in `mt_main` (today only `out_cap` is bounded; the input is bounded only through `MT_REQ_MAX`).

### IN-02: The contract document describes two behaviours the code does not have

**File:** `docs/specs/equivalence-contract.md:59, 64`
**Issue:** (a) Line 59 says a return wider than 32 bits is recorded in `lib_state_out.ret`; the code refuses such entries (`UnsupportedReturn`) or, in the typedef case, truncates (see CR-01). (b) Line 64 says control characters are written as `\u00XX`; `serde_json` writes `\n`, `\t`, `\r`, `\b` and `\f` as short escapes. The sha256 of a vector is the hash of those exact bytes, so an independent reader that follows the document would compute different file names.
**Fix:** State the actual serialiser rules (or refer to `serde_json::to_vec_pretty` plus the trailing newline), and align the wide-return sentence with the implementation.

### IN-03: Canonical form depends on `serde_json`'s sorted `Map`, and a few `[]` indexings rely on earlier checks without a comment

**File:** `crates/mt-vectors/src/state.rs:222, 330-343`; `crates/mt-harness/src/sample.rs:143`
**Issue:** `StateValue::to_json` output order, the pointer key match `["buffer", "offset"]` and the sha256 file names all assume `serde_json::Map` is a `BTreeMap`. Enabling the `preserve_order` feature anywhere in the dependency graph (feature unification) would change every file name and reject valid pointer objects. The indexings `object[*shape]`, `map["buffer"]`, `map["offset"]` and `corpus.index.vectors[sha]` cannot panic today, but the project rule asks for an invariant comment where a panic is possible.
**Fix:** Add a test that fails if `serde_json::Map` is not sorted (insert `b` then `a` and compare the order). Add one-line invariant comments, or use `.get()` with an error.

### IN-04: Duplicated helpers

**File:** `crates/mt-harness/src/driver.rs:90-105` vs `crates/mt-harness/src/link.rs:174-202`; `crates/mt-cli/src/main.rs:994-1003` vs `1266-1283`
**Issue:** `build_driver` repeats the health test and the stderr excerpt that `link::healthy`/`link::failure` implement. `selftest_command` repeats `exec_context` (pins load, validate, tools, runner config). Two copies of a "did the tool succeed" test can drift apart (one already omits `signal`).
**Fix:** Use `link::healthy`/`link::failure` in `driver.rs`, and call `exec_context` from `selftest_command`.

### IN-05: The image has no default non-root user

**File:** `container/Dockerfile:510-529`
**Issue:** The final stage sets no `USER`. Every CI invocation passes `--user 65534:65534`, but anyone running the image directly gets root. The workflow's hardening is therefore a property of the callers.
**Fix:** Add `USER 65534:65534` to the final stage (the tools are world-readable via `chmod -R a+rX`), or document why not.

### IN-06: Recompile and flag handling is a denylist while the driver flags are an allowlist

**File:** `crates/mt-harness/src/probe.rs:258-286`; `crates/mt-harness/src/adapter.rs:1273-1277`
**Issue:** `driver_flags` fails closed on unknown `-` options. `recompile_arguments` keeps every option except `-o`, `-M*`, `-O*`, `-g*`, so flags such as `-fplugin=`, `-specs=`, `-wrapper` or `@file` from an upstream compile command reach the pinned compiler. In `driver_flags`, a response-file argument `@file` does not start with `-` and is dropped as "the source file", so its flags are lost without an error. Both operate on canonical commands from a verified capture, so the risk is low, but the two paths should agree.
**Fix:** Reuse `driver_flags` (or the same allowlist) in `recompile_arguments`, and make `driver_flags` return `UnhandledFlag` for any argument that starts with `@`.

### IN-07: Policy loader silently ignores `.toml` files whose stem is not valid UTF-8

**File:** `crates/mt-contract/src/policy.rs:446-453`
**Issue:** `load_dir` skips a `*.toml` whose `file_stem().to_str()` is `None`, so a stray non-UTF-8 policy file is neither accepted nor reported, although the stated rule is "the set of `*.toml` files must equal the ids".
**Fix:** Push an issue for a `.toml` entry whose stem is not UTF-8.

---

_Reviewed: 2026-11-01_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
