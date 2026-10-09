# Toolchain container

The pinned toolchain image that every later phase runs its analysers in
(Phase 1, requirements TOOL-01 to TOOL-04). Docker is the only supported
runtime, and `linux/amd64` is the only supported platform: there is no arm64
variant, and other OCI runtimes are not tested (CONTEXT D-05, D-22).

All paths below are relative to the repository root.

## What the image contains

Every version comes from one file, `container/pins.toml`. The Dockerfile reads
those values only as build arguments rendered by `mt toolchain build-args`, and
`mt toolchain check` judges the live tools against the same file.

| Tool | Pin source in `container/pins.toml` |
|------|-------------------------------------|
| Debian bookworm base, apt snapshot | `[image]` (`base`, `base_digest`, `snapshot_timestamp`) |
| clang, llvm-config and LLVM libraries, one major | `[llvm]` and the `clang-16`, `llvm-16`, `llvm-16-dev`, `libclang-16-dev`, `libclang-cpp16-dev` entries of `[apt]` |
| KLEE 3.2 with klee-uclibc | `[source.klee]`, `[source.klee_uclibc]` (tag and commit, built from source against the one LLVM major) |
| c2rust 0.22.1 | `[source.c2rust]` (crates.io checksum) |
| cargo-mutants 27.1.0 | `[source.cargo_mutants]` |
| Bear 3.1.1, QEMU (`qemu-user`, `qemu-system-arm`), libz3 | `[apt]` |
| Arm GNU Toolchain 14.3.rel1 (`arm-none-eabi-gcc`) | `[source.arm_gnu]` (archive checksum) |
| rustup 1.29.1 | `[source.rustup]` |
| Rust 1.99.0 for the tool and the `no_std` crates, with the `thumbv7em-none-eabihf` and `x86_64-unknown-linux-musl` targets | `[rust.tool]` |
| Rust 1.72.1 whose LLVM (16.0.5) matches the C-side major, host target only, used to emit bitcode for KLEE | `[rust.bitcode]` |

The LLVM major (16) and the bitcode rustc (1.72.1) are **provisional**: they are
recorded as PROJECT.md decision D-15 and are revisited at the Phase 6 R1
verdict. A green build shows that the tools install and report LLVM 16; it is
not evidence that Rust-to-KLEE bitcode works. `mt toolchain check` prints a line
marking the pin provisional, and changing either value needs a new decision
(see Bumping pins).

The image does not contain `mt`. The static `mt` binary and `pins.toml` are
mounted read-only when the image runs, so the image depends only on the
Dockerfile and `pins.toml`.

Two tools are deliberately not installed, and the manifest lists both with the
status `not_installed` (CONTEXT D-10); `mt toolchain check` does not fail on
their absence:

- **Hayroll**: its licence is unconfirmed (risk R3), so nothing depends on it
  or redistributes it until that is resolved; its adapter is gated in Phase 8.
- **Kani**: Kani evidence is deferred to milestone v2.0; the R2 spike is in
  Phase 6.

## Reproducibility guarantee

The guarantee is **manifest-identical** builds (CONTEXT D-06): CI builds the
image twice from the same inputs on two separate runners, the second time with
`--no-cache --pull`, runs `mt toolchain manifest` in each, and requires the two
tool-version manifests (names, versions, source references and source hashes)
to be byte-identical (`mt toolchain manifest-diff`). Binary-level identity of
the installed tools is not claimed, and two builds of the image do not
necessarily have the same digest. This is a statement about the toolchain image
only.

Because of that, consumers pin the **digest of the one image that passed the
checks**, never a tag. The image is also tagged `sha-<commit>`, but a tag can be
moved and is never a reference to use.

## Pulling and running on Apple Silicon

The verified digest is recorded under Current image below. An Apple Silicon Mac
runs the `linux/amd64` image under emulation, which is slow; always pass the
platform explicitly so Docker does not select arm64:

```sh
docker pull --platform linux/amd64 ghcr.io/sat-wik/migrationtool-toolchain@sha256:<digest>
```

When the digest below was recorded, the registry served its manifest to an
anonymous request, so no login should be needed. If a pull is ever refused,
`docker login ghcr.io` with a GitHub account that may read the package
(visibility is a setting of the package on GitHub).

You need a **static `mt`** to run inside the image. Either:

- download the `mt-bundle` artifact of the run named under Current image
  (`gh run download <run-id> --name mt-bundle --dir mt-bundle`, then
  `chmod +x mt-bundle/mt`; CI keeps artifacts for 7 days only), or
- build it with the image's own toolchain, with network access for the crates:
  `docker run --rm --platform linux/amd64 --user "$(id -u):$(id -g)" -v "$PWD:/work" -w /work -e CARGO_HOME=/tmp/cargo -e CARGO_TARGET_DIR=/work/target-image ghcr.io/sat-wik/migrationtool-toolchain@sha256:<digest> cargo build --release --locked -p mt-cli --target x86_64-unknown-linux-musl`
  (the binary is then `target-image/x86_64-unknown-linux-musl/release/mt`).
  CI builds `mt` on a runner, not inside the image, so this second route has not
  been exercised by CI.

Then run the check with `mt` and `pins.toml` mounted read-only, the way CI does
(no network, unprivileged user):

```sh
docker run --rm --platform linux/amd64 --network none --user 65534:65534 \
  --read-only --tmpfs /tmp:rw,exec,nosuid,size=2g \
  -v "$PWD/mt-bundle/mt:/usr/local/bin/mt:ro" \
  -v "$PWD/container/pins.toml:/opt/mt/pins.toml:ro" \
  ghcr.io/sat-wik/migrationtool-toolchain@sha256:<digest> \
  mt toolchain check --pins /opt/mt/pins.toml
echo "exit code: $?"
```

Expected: every tool row `OK` (Hayroll and Kani `not_installed`), the
`rustc_bitcode` row reporting LLVM 16.0.5, the line marking the LLVM pin
provisional (decision D-15), `result: OK`, and exit code 0. Any other exit code
means the image differs from its pins, or the pins file could not be read.

## Local build fallback

A local build is the slow fallback when the registry image is not available.
Render the build arguments with `mt` (this needs no Docker, so it runs natively
on the Mac) and pass each line as a `--build-arg`:

```sh
args=()
while IFS= read -r line; do
  [ -n "$line" ] || continue
  args+=(--build-arg "${line%%=*}=${line#*=}")
done < <(cargo run -q -p mt-cli -- toolchain build-args --pins container/pins.toml)
docker buildx build --platform linux/amd64 --target final --load \
  --tag mt-toolchain:local "${args[@]}" container/
```

The local image will not have the registry image's digest. What can be compared
is the manifest: run `mt toolchain manifest --pins /opt/mt/pins.toml --out
/out/manifest-local.json` in it the same way as the check above (with a
writable `/out` mounted), and compare against the verified manifest with
`mt toolchain manifest-diff`, or compare its sha256 with the manifest sha256
recorded under Current image.

## Bumping pins

Pins change by a hand-edited pull request to `container/pins.toml` (CONTEXT
D-20). There is no Renovate or Dependabot.

1. Edit `container/pins.toml`. Recompute any download checksum with
   `mt toolchain hash <url>` (it prints the sha256 and the URL; `--expect`
   fails when the digest differs).
2. For apt versions and the Arm checksum, run the `pin-discovery` workflow
   (it also runs when `pins.toml` or the Dockerfile change): it prints the apt
   candidate versions at the pinned snapshot timestamp and the official Arm
   checksum as a `pin-discovery` annotation.
3. Keep `rust-toolchain.toml` equal to `[rust.tool]`; `cargo test` checks it.
4. CI rebuilds the image twice and diffs the manifests (the `container`
   workflow), and runs the check against the live image and against an off-pin
   copy that must fail.
5. Changing `[llvm].major` or `[rust.bitcode].version` is a project decision
   (CONTEXT D-21): add a new `- **D-NN:**` line to `.planning/PROJECT.md`
   carrying the matching `llvm=<major>` and `bitcode-rustc=<version>` tokens and
   point the `decision` fields of `pins.toml` at it. Without that line,
   `cargo test` fails.

GitHub Action versions are the one pin that is not in `pins.toml`, because
GitHub resolves `uses:` before any step can read it; they are 40-character
commit SHAs edited by hand in `.github/workflows/`, and a test refuses anything
else.

## Notices

Some of the installed tools are distributed under GPL licences. They, and every
other tool in the image, run only as subprocesses inside the container; none is
linked into `mt`, and `mt` is not part of the image (see section 4 of
docs/guidelines/building-the-tool.md). The licence names below were read from the
projects' own files, not recalled:

- **Bear 3.1.1**: `COPYING` at upstream tag `3.1.1` is the GNU General Public
  License, version 3.
- **QEMU** (`qemu-user`, `qemu-system-arm`): the upstream `LICENSE` file at tag
  `v7.2.0` says the QEMU emulator as a whole is released under the GNU General
  Public License, version 2, with some parts under other licences compatible
  with it, stated per source file, and that its firmware files are separate
  programs with separate licences. The Debian package is `7.2+dfsg`; its own
  copyright file in the image is the authority for the packaged files.
- **Arm GNU Toolchain 14.3.rel1**: the toolchain bundles GCC, binutils and
  other components under GPL and other licences. This was **not read** from the
  session that wrote this file (the archive host is not reachable from it), so
  no per-component licence is named here; read the licence files that ship with
  the toolchain under `/opt/arm-gnu-toolchain/` in the image.

The remaining tools (the Debian packages, KLEE, c2rust, cargo-mutants, Rust)
keep their own licences. For a Debian package the copyright file is
`/usr/share/doc/<package>/copyright` in the image; for the rest, use the
`LICENSE` or `COPYING` file of the pinned upstream source. Redistributing the
image outside the founder's own use needs a licence review first; nothing here
is legal advice.

## Current image

The verified image, published by the `image-a` job and announced as verified by
the `compare` job of `container` workflow run 37983108094 only after the two
manifests were found manifest-identical:

```
ghcr.io/sat-wik/migrationtool-toolchain@sha256:b8032124716db8f2bfdfea5b693665bb57f37972895f038c2948affb5956e46a
```

- Run id: 37983108094 (jobs `mt`, `image-a`, `image-b` and `compare` all
  concluded `success`; the pushed image is the one that passed the check,
  off-pin check, manifest and smoke steps in `image-a`)
- Commit: 537ef911a8adffac79aa1faf4cec73f5794df829
- Manifest sha256 (identical for both builds):
  40e55cc69fa93f238960bad268cfa4ec5775d51f98dd10efcf92741801d7aed3

Later commits that do not touch the Dockerfile, `pins.toml`, the smoke crate,
the Rust sources or the workflow do not rebuild the image, so this stays the
current reference until one of those changes; a rebuild publishes a new digest
and this section is updated with it.
