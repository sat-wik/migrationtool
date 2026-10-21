# Hand-made test-bed seeds: research helpers

These Python scripts print the hex of the hand-made seeds that
`testbed/targets/<id>/hand-made/*.hex` holds for the targets whose own tests ship
no usable inputs (BUILD-03, CONTEXT D-13).

They are research helpers. They were run once by hand, and their output was
committed as reviewable hex text. **Neither the tool nor any test runs them**; the
tool reads only the `.hex` files, and Python lives under `research/` only
(`tool_01_no_python_outside_research`). They use the Python standard library only
and install nothing.

| Script | Target | Why a script |
|--------|--------|--------------|
| `libfdt.py` | libfdt | big-endian header offsets and sizes of a flattened devicetree |
| `lz4.py` | lz4 | the frame header checksum is XXH32 over the frame descriptor |
| `zlib.py` | zlib | Adler-32 trailers and the Huffman bit packing of RFC 1951 |

The other hand-made sets (`paho`, `microcoap`, `isotp`, `coremqtt`) are short
packets written byte by byte from their specifications; the specification and the
case are named in the header comment of every file.

## Running a helper

```sh
python3 -I research/testbed-seeds/libfdt.py --list
python3 -I research/testbed-seeds/libfdt.py minimal_valid
```

Use `python3 -I` (isolated mode). `zlib.py` carries the name of its target, and
without `-I` Python would put this directory first on `sys.path`.

## What a seed file is

Each `.hex` file starts with `#` comment lines naming the specification and the
case, and the helper command when one produced it. The rest is hex bytes in any
spacing. Malformed and truncated cases are included on purpose: they are fuzzing
seeds. Every hand-made seed is recorded in the target's
`testbed/seeds/<id>/index.json` with method `hand_made` and `proposed: true`. None
of them is upstream test data, and each target's `NOTICE` says so.

## How the seeds were checked

Before they were committed, the files were fed to the real parsers on the
development host as a plausibility check (libfdt `fdt_check_full`, LZ4
`LZ4_decompress_safe` and `LZ4F_getFrameInfo`, and Python's zlib for the zlib
streams). The valid cases were accepted and almost all malformed cases were
rejected. Where a parser accepted a malformed case (the LZ4 block with offset 0 is
one), the file stays: what each implementation does with such an input is what the
later differential runs are for. That check is not part of the repository and is
not evidence for any claim.
