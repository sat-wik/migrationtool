# Provenance of the CI-captured fixtures (CONTEXT D-26)

These files are captures from the `testbed` workflow, produced inside the pinned
toolchain image and published as `notice` annotations titled `fixture ...`. They
are copied here without change.

| Field | Value |
|-------|-------|
| Workflow | `.github/workflows/testbed.yml` (`testbed` job) |
| Run id | 38380382970 |
| Job id | 115239376216 |
| Head SHA | 4195a6dd54ea17e9c4c0bba1ee4017e52725ddcb |
| Image | `ghcr.io/sat-wik/migrationtool-toolchain@sha256:e83006c0a7e37f788afcbd1c95554e8c4dfc61c3b68da3ff964caf47fb3851b9` |
| Image id | `sha256:1267a6230e171d711a7d056cd7a129fbdf0753339c92af22dac69afc806eac44` (`linux/amd64`) |
| Read with | `gh api --paginate repos/sat-wik/migrationtool/check-runs/115239376216/annotations` |

## Files

| File | Notice title | Notes |
|------|--------------|-------|
| `qcbor-default.compile_commands.json` | `fixture qcbor-default compile_commands` | canonical compile commands of qcbor, configuration `default` |
| `corejson-default.compile_commands.json` | `fixture corejson-default compile_commands` | canonical compile commands of corejson, configuration `default` |
| `corejson-default.capture.json` | `fixture corejson-default capture (1/5)` to `(5/5)` | capture report; the five parts joined in order with a newline |
| `corejson-default.replay.json` | `fixture corejson-default replay` | replay report |
| `arm-object.readelf-A.txt` | `fixture readelf-A` | `readelf -A` of `build/CMakeFiles/core_json.dir/source/core_json.c.obj`; the first line `object: ...` is the workflow's own label |
| `arm-small.o` | `fixture arm-small.o` | `/capture/qcbor/no-hw-float/objects/src/src/ieee754.o`, 792 bytes, sha256 `5e511bd11367be0911cf68f37438b5cf94c118fd0ca6f84e59af5dbd48e4a016`; decoded from the notice's base64 |
| `arm-small-comment.o` | `fixture arm-small-comment.o` | the same object with `.comment` replaced, 740 bytes, sha256 `c7dd97fa6bcd780b402cec148c6d04ec2b6671d5da3d98b28e9afebf6f228ca5`; decoded from the notice's base64 |
| `qcbor-entry.ast.json` | `fixture qcbor entry ast (1/18)` to `(18/18)` | clang 16 AST dump of the `QCBORDecode_Init` probe; the eighteen parts joined in order with a newline |

## How they were copied

- Each part of a multi-part notice is a run of whole lines whose final newline
  was dropped. The parts are joined in order with one newline. The whole file
  therefore lacks a final newline, which is why the round-trip tests compare
  the serialised form with the fixture plus one newline.
- The two objects were decoded from the base64 in the notice. The `sha256` and
  `bytes` lines of each notice matched the decoded bytes when they were copied.
- The annotation listing is newest first, so the parts were ordered by their
  `(n/N)` number, not by listing position.

## Rules

- The files are CI captures. They are replaced only by copying the notices of a
  newer run of the `testbed` workflow, and this file is updated with that run's
  id, head SHA and image digest in the same commit.
- They are never edited by hand. A fixture that fails its test is evidence
  about the code or about the run.
