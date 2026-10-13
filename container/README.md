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
| KLEE 3.2 with klee-uclibc | `[source.klee]`, `[source.klee_uclibc]` (tag and commit, built from source against the one LLVM major) and `[source.klee_uclibc_locale]` (the locale tarball the klee-uclibc Makefile downloads, by URL and sha256) |
| c2rust 0.22.1 | `[source.c2rust]` (crates.io checksum) |
| cargo-mutants 27.1.0 | `[source.cargo_mutants]` |
| Bear 3.1.1, QEMU (`qemu-user`, `qemu-system-arm`), libz3 | `[apt]` |
| git, cmake, curl, bzip2 and make (Phase 2 test bed: fetch by commit, CMake builds, the Mbed TLS archive; make was installed since Phase 1 and is now pinned) | `[apt]` and the `[tool.git]`, `[tool.cmake]`, `[tool.curl]`, `[tool.bzip2]` and `[tool.make]` rows (PROJECT.md decision D-17) |
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
on the Mac) and pass each line as a `--build-arg`. `mt` builds on macOS (CI
type-checks it for `aarch64-apple-darwin`) but is only run and tested on Linux:
on a Mac its timeout kills the child's process group and cannot find a
descendant that left the group, and a Windows host does not compile. Anything
that produces evidence runs in the pinned container.

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
   A `[rust.tool]` or `[rust.bitcode]` `channel_manifest_sha256` is the sha256 of
   `https://static.rust-lang.org/dist/channel-rust-<version>.toml` (`mt toolchain hash`
   of that URL), and the image installs that toolchain only from that verified
   file and the archives it lists. A crate `sha256` is that of its `.crate` file on
   `static.crates.io`, and that verified file is the one the image unpacks and
   installs.
2. For apt versions, the Arm checksum and the klee-uclibc locale tarball
   checksum, run the `pin-discovery` workflow (it also runs when `pins.toml` or
   the Dockerfile change): it prints the apt candidate versions at the pinned
   snapshot timestamp, the official Arm checksum and the computed checksum of
   the locale tarball as a `pin-discovery` annotation, and fails when a pinned
   checksum differs from the computed one. (`www.uclibc.org` is not reachable
   from cloud sessions, only from runners.)
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

The Phase 2 additions are subprocess-only tools as well and are never linked
into `mt`. The licence names below are the ones the Debian packages declare; they
were not read from the image in the session that wrote this file, so for each
package the text at `/usr/share/doc/<package>/copyright` in the image is the
authoritative one:

- **git**: GPL-2.0-only.
- **make**: GPL-3.0-or-later.
- **cmake**: BSD-3-Clause.
- **curl**: the curl licence.
- **bzip2**: the bzip2-1.0.6 licence.

The remaining tools (the Debian packages, KLEE, c2rust, cargo-mutants, Rust)
keep their own licences. For a Debian package the copyright file is
`/usr/share/doc/<package>/copyright` in the image; for the rest, use the
`LICENSE` or `COPYING` file of the pinned upstream source. Redistributing the
image outside the founder's own use needs a licence review first; nothing here
is legal advice.

## Current image

The verified image, published by the `publish` job of `container` workflow run
38202499776 on a push to `main` (merge commit
67467607c52acb52ff5bdebc11e81d9fb31ccfb1, pull request #3) and announced as
verified by that job. It is the first image published from `main`. `publish`
ran only after `image-a`, `image-b` and `compare` had all succeeded, so the two
manifests had been found manifest-identical. It pushed the docker-save archive
that `image-a` saved after its own gates (the pin-binding gate, check, off-pin
check, manifest and smoke steps), not a rebuild: the archive sha256
(`92bc6d0a2c24cd7d3189e2db620d2208bda1d8c7542fbba4107b43d2f150ef00`) was
checked before `docker load`, and the loaded image id
(`sha256:8a1f04358e50f0b6b96715e20cedaa5b0cb763dbd30aa7c7c1dcce03971eb07d`)
equalled the id `image-a` recorded. Both Rust toolchains are installed only from
the sha256-checked channel manifests and the archives they list, and c2rust and
cargo-mutants only from their sha256-checked `.crate` files; those installs, the
Arm toolchain unpacking and the klee-uclibc `make` run in `RUN --network=none`
steps (the `pin-binding` notice of `image-a` shows the build record matching the
pins, with each archive line matched to a pair of its pin-verified manifest). The
locale tarball the klee-uclibc `make` needs is pinned as
`[source.klee_uclibc_locale]` and checked with `sha256sum -c` in the networked
RUN that clones klee-uclibc, so a different tarball fails the build; it is not
a line of the `pin-binding` record, which covers the Rust toolchains and crates:

```
ghcr.io/sat-wik/migrationtool-toolchain@sha256:42c7fb0e00e90e946b5a21e598517fb7189a0de5492fa1d4d6f18e2e9c39e13d
```

- Run id: 38202499776, a `push` to `main` (jobs `mt`, `image-a`, `image-b`,
  `compare` and `publish` all concluded `success`)
- Commit: 67467607c52acb52ff5bdebc11e81d9fb31ccfb1, pushed under the tag
  `sha-67467607c52acb52ff5bdebc11e81d9fb31ccfb1` (a tag is never a reference to
  use; see Reproducibility guarantee)
- Manifest sha256 (identical for both builds):
  d70029186639d0030fc6cf22b284b3b849ce0680656fecfe06623f6ffdc44142. This is the
  manifest of the previous image,
  `sha256:0f5a5d2594b0405931ad7e85d5c6d2f5e3f2d5099d769059870817ee50e152ee`,
  built on the phase branch by run 38198934488 at commit
  defbe68309b4e1fc3a517ca49a11fed0087a2fd8. No pin changed between the two; the
  image digests differ because the image was built again, which
  manifest-identical builds allow.

The founder's Phase 1 check (UAT) was run on that branch-built
`sha256:0f5a5d2594b0405931ad7e85d5c6d2f5e3f2d5099d769059870817ee50e152ee`
image, which has the same manifest; it has not been repeated on the image above.

Only a push to `main` publishes (WR-09). A run for a pull request, another
branch, a manual dispatch or the weekly schedule still builds both images,
gates them and compares the manifests, but its `publish` job is skipped, so it
announces no `verified-image` and pushes nothing to GHCR. The previous image was
published by a run on the phase branch (`claude/gsd-discuss-phase-1-mzdw5v`)
while that branch still held publish rights; no branch other than `main` holds
them now.

Later commits that do not touch the Dockerfile, `pins.toml`, the smoke crate,
the Rust sources or the workflow do not rebuild the image, so this stays the
current reference until one of those changes; a rebuild on `main` publishes a
new digest and this section is updated with it. A `sha-<commit>` tag that
already exists is never overwritten: `publish` skips the push with a warning.
