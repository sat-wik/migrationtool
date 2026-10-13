# The test bed

The test bed is a list of open-source C parsers (CBOR, JSON, MQTT, CoAP, BLE,
ASN.1, protobuf, compression and similar formats) that the harness is exercised
on. `manifest.toml` names each one by its exact commit and by the sha256 of its
checked-out tree, with a permissive licence, one to three hand-named entry
functions that take a buffer, and its shipped configurations: the upstream
default plus at most one realistic variant. Every configuration is built for
Cortex-M4F (`-mcpu=cortex-m4 -mthumb -mfloat-abi=hard -mfpu=fpv4-sp-d16 -Os`)
with the pinned `arm-none-eabi-gcc`, by the project's own build system (Make
under Bear, or CMake), and only the parser library is built, never a firmware
image. The build is captured once; its compile commands are then replayed in a
clean copy of the source tree and every object file is compared byte for byte
with the original.

## The work directory

Fetched sources, build trees and capture artifacts live in a work directory that
git ignores: `.mt-work/` by default, or the directory named by `$MT_WORK_DIR` or
`--work-dir`. In the image it is `/work`. Nothing in it is ever committed. A
fetched source is checked against its pinned commit and tree hash every time it is
reused; a cache that no longer matches its pin is an error, never a silent
refetch.

## The three container steps

Each step is `mt testbed <step>` run inside the pinned image, which is named by
digest under "Current image" in `container/README.md`. `mt` (a static binary),
`container/pins.toml` and this directory are mounted read-only, and only `/work`
and `/out` are writable. The container runs as user `65534:65534` with
`--read-only`, `--cap-drop ALL` and `no-new-privileges`.

1. **fetch** has network access. It brings each source in by its pinned commit
   and accepts it only when the commit and the tree sha256 equal the manifest.
2. **capture** runs with `--network none`. It builds each configuration under
   Bear or CMake, then checks that the compiler is the pinned Arm compiler, that
   every object is an ELF32 Cortex-M4 hard-float object, that no object was
   created that no compile command declares, and that every entry function exists
   with the manifest's signature in a header the defining file includes. It also
   compares each variant with its default to show the variant changes code.
3. **replay** runs with `--network none`. It deletes the source tree, restores
   the pristine copy, removes every declared object and re-runs each captured
   compile command, then compares the objects. The verdict is `identical`,
   `identical_after_normalisation` (only a differing `.comment` or debug section,
   removed identically from both objects), `differs`, `not_proven` (a timeout or
   an incomplete output stream) or `build_failed`.

```sh
docker run --rm --platform linux/amd64 --network none --user 65534:65534 \
  --read-only --cap-drop ALL --security-opt no-new-privileges \
  --tmpfs /tmp:rw,exec,nosuid,size=2g \
  -v "$PWD/mt-bundle/mt:/usr/local/bin/mt:ro" \
  -v "$PWD/container/pins.toml:/opt/mt/pins.toml:ro" \
  -v "$PWD/testbed:/opt/mt/testbed:ro" \
  -v "$PWD/.mt-work:/work" -v "$PWD/.mt-out:/out" \
  ghcr.io/sat-wik/migrationtool-toolchain@sha256:<digest> \
  mt testbed capture --manifest /opt/mt/testbed/manifest.toml \
    --pins /opt/mt/pins.toml --work-dir /work --out /out qcbor corejson
```

Run `fetch` the same way without `--network none`, and `replay` with the
`capture` arguments (it needs no `--image`). `mt testbed validate --manifest
<file>` checks the manifest alone and needs no image.

## Where it runs

CI is the primary place it runs: the `testbed` workflow
(`.github/workflows/testbed.yml`) pulls the image by digest, runs the steps above
and publishes each step's result as a job annotation. The founder's Apple Silicon
Mac emulates `linux/amd64` slowly, so running the steps by hand there is possible
but not the intended path. Each step also writes its reports to the output
directory (`--out`, `<work-dir>/reports` by default):

- `fetch/<id>.json`
- `capture/<id>/<config>.json` and `capture/<id>/<config>.compile_commands.json`
- `capture/<id>/variant.json`
- `replay/<id>/<config>.json`
- `summary-<step>.json`

The workflow also uploads them as the `testbed-reports` artifact for 7 days.

## Exit codes

- `0`: every selected target or configuration passed.
- `1`: at least one failed or was not proven. The summary table names which, and
  the reports carry the causes.
- `2`: invalid input, such as an invalid manifest or pins file, an unknown target
  id, or a target that has no build recipe yet.

A target is selected by naming its id (`qcbor corejson`) or with `--all`.
