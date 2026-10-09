# The toolchain image (TOOL-03). Every pin arrives as a build argument rendered
# from container/pins.toml by `mt toolchain build-args` (CONTEXT D-14), for
# example:
#
#   args=(); while IFS= read -r line; do args+=(--build-arg "${line%%=*}=${line#*=}"); done \
#     < <(mt toolchain build-args --pins container/pins.toml)
#   docker buildx build --platform linux/amd64 "${args[@]}" container
#
# This file must stay free of version literals: no default on any ARG, no
# floating tag, no unpinned download. There is deliberately no syntax directive
# (it would pull a floating frontend); BuildKit's built-in frontend is used and
# BuildKit itself is pinned through the ci table of pins.toml. The mt binary is
# mounted at run time and never baked into the image.

# Global build arguments: exactly the image-scope keys of `mt toolchain build-args`.
ARG ARM_GNU_SHA256
ARG ARM_GNU_URL
ARG ARM_GNU_VERSION
ARG BASE_DIGEST
ARG BASE_REF
ARG C2RUST_NAME
ARG C2RUST_SHA256
ARG C2RUST_VERSION
ARG CARGO_HOME
ARG CARGO_MUTANTS_NAME
ARG CARGO_MUTANTS_SHA256
ARG CARGO_MUTANTS_VERSION
ARG KLEE_COMMIT
ARG KLEE_TAG
ARG KLEE_UCLIBC_COMMIT
ARG KLEE_UCLIBC_TAG
ARG KLEE_UCLIBC_URL
ARG KLEE_URL
ARG LLVM_MAJOR
ARG PLATFORM
ARG RUSTUP_HOME
ARG RUSTUP_SHA256
ARG RUSTUP_URL
ARG RUSTUP_VERSION
ARG RUST_BITCODE_CHANNEL_SHA256
ARG RUST_BITCODE_COMMIT
ARG RUST_BITCODE_VERSION
ARG RUST_HOST
ARG RUST_TOOL_CHANNEL_SHA256
ARG RUST_TOOL_COMMIT
ARG RUST_TOOL_COMPONENTS
ARG RUST_TOOL_TARGETS
ARG RUST_TOOL_VERSION
ARG SNAPSHOT_TIMESTAMP
ARG SOURCE_DATE_EPOCH
ARG TINYCBOR_COMMIT
ARG TINYCBOR_PARENT
ARG TOOL_PATH

# Stage snapshot-sources: the digest-pinned base with every apt source replaced
# by snapshot.debian.org at one fixed timestamp (CONTEXT D-04). Later stages
# install packages from here; this stage installs nothing.
FROM --platform=${PLATFORM} ${BASE_REF}@${BASE_DIGEST} AS snapshot-sources
ARG SNAPSHOT_TIMESTAMP
# http on purpose: the slim base has no CA bundle, and apt verifies the signed
# Release files itself. Check-Valid-Until is off because snapshots are old by
# design; retries and timeouts absorb the archive's rate limiting.
RUN set -eux; \
    : "${SNAPSHOT_TIMESTAMP:?}"; \
    rm -f /etc/apt/sources.list /etc/apt/sources.list.d/*; \
    . /etc/os-release; \
    : "${VERSION_CODENAME:?}"; \
    printf '%s\n' \
      "deb [check-valid-until=no] http://snapshot.debian.org/archive/debian/${SNAPSHOT_TIMESTAMP}/ ${VERSION_CODENAME} main" \
      "deb [check-valid-until=no] http://snapshot.debian.org/archive/debian/${SNAPSHOT_TIMESTAMP}/ ${VERSION_CODENAME}-updates main" \
      "deb [check-valid-until=no] http://snapshot.debian.org/archive/debian-security/${SNAPSHOT_TIMESTAMP}/ ${VERSION_CODENAME}-security main" \
      > /etc/apt/sources.list; \
    printf '%s\n' \
      'Acquire::Check-Valid-Until "false";' \
      'Acquire::Retries "5";' \
      'Acquire::http::Timeout "120";' \
      'APT::Install-Recommends "false";' \
      > /etc/apt/apt.conf.d/99snapshot

# Stage apt-base: the runtime set from the snapshot archive (CONTEXT D-04). Only
# packages of the pinned LLVM major are named; the unversioned compiler and
# LLVM packages are an older major on this release and must never be installed
# (RESEARCH Pitfall 1). Sanitizer and libFuzzer runtimes are left out: they
# serve later phases and arrive through the pin-bump flow then (CONTEXT D-20).
FROM snapshot-sources AS apt-base
ARG LLVM_MAJOR
ARG SOURCE_DATE_EPOCH
ENV SOURCE_DATE_EPOCH=${SOURCE_DATE_EPOCH}
RUN set -eux; \
    : "${LLVM_MAJOR:?}"; \
    apt-get update; \
    DEBIAN_FRONTEND=noninteractive apt-get install --no-install-recommends --yes \
      ca-certificates \
      clang-${LLVM_MAJOR} \
      llvm-${LLVM_MAJOR} \
      llvm-${LLVM_MAJOR}-dev \
      libclang-${LLVM_MAJOR}-dev \
      libclang-cpp${LLVM_MAJOR}-dev \
      bear \
      qemu-system-arm \
      qemu-user \
      libz3-4 \
      libsqlite3-0 \
      zlib1g \
      libc6-dev \
      make \
      xz-utils; \
    rm -rf /var/lib/apt/lists/*

# Stage build-base: build-only tools. Nothing from here is copied into the final
# image except through an explicit COPY --from. The first PATH entry makes the
# unversioned LLVM tool names (llvm-link, llvm-ar, clang) resolve to the pinned
# major for every build stage.
FROM apt-base AS build-base
ARG LLVM_MAJOR
ARG SOURCE_DATE_EPOCH
ENV SOURCE_DATE_EPOCH=${SOURCE_DATE_EPOCH}
ENV PATH="/usr/lib/llvm-${LLVM_MAJOR}/bin:${PATH}"
RUN set -eux; \
    apt-get update; \
    DEBIAN_FRONTEND=noninteractive apt-get install --no-install-recommends --yes \
      build-essential \
      cmake \
      curl \
      git \
      libsqlite3-dev \
      libz3-dev \
      ninja-build \
      patch \
      pkg-config \
      python3 \
      zlib1g-dev; \
    rm -rf /var/lib/apt/lists/*

# Stage rustup (CONTEXT D-12, D-13): rustup-init, both channel manifests and
# nothing else is trusted on the strength of a pinned sha256. The tool
# toolchain carries the components and targets of pins.toml; the bitcode
# toolchain is the host target only and is never the default.
FROM build-base AS rustup
ARG CARGO_HOME
ARG RUSTUP_HOME
ARG RUSTUP_SHA256
ARG RUSTUP_URL
ARG RUST_BITCODE_CHANNEL_SHA256
ARG RUST_BITCODE_VERSION
ARG RUST_HOST
ARG RUST_TOOL_CHANNEL_SHA256
ARG RUST_TOOL_COMPONENTS
ARG RUST_TOOL_TARGETS
ARG RUST_TOOL_VERSION
ARG SOURCE_DATE_EPOCH
ENV SOURCE_DATE_EPOCH=${SOURCE_DATE_EPOCH} \
    RUSTUP_HOME=${RUSTUP_HOME} \
    CARGO_HOME=${CARGO_HOME}
ENV PATH="${CARGO_HOME}/bin:${PATH}"
RUN set -eux; \
    : "${RUSTUP_URL:?}" "${RUSTUP_SHA256:?}" "${RUST_HOST:?}"; \
    curl -fsSL --retry 5 --retry-delay 5 -o /tmp/rustup-init "${RUSTUP_URL}"; \
    echo "${RUSTUP_SHA256}  /tmp/rustup-init" | sha256sum -c -; \
    chmod +x /tmp/rustup-init; \
    /tmp/rustup-init -y --no-modify-path --profile minimal --default-toolchain none --default-host "${RUST_HOST}"; \
    rm -f /tmp/rustup-init
RUN set -eux; \
    : "${RUST_TOOL_VERSION:?}" "${RUST_TOOL_CHANNEL_SHA256:?}" "${RUST_BITCODE_VERSION:?}" "${RUST_BITCODE_CHANNEL_SHA256:?}"; \
    : "${RUST_TOOL_COMPONENTS:?}" "${RUST_TOOL_TARGETS:?}"; \
    for pair in "${RUST_TOOL_VERSION}:${RUST_TOOL_CHANNEL_SHA256}" "${RUST_BITCODE_VERSION}:${RUST_BITCODE_CHANNEL_SHA256}"; do \
      version="${pair%%:*}"; \
      sha="${pair#*:}"; \
      curl -fsSL --retry 5 --retry-delay 5 -o "/tmp/channel-rust-${version}.toml" "https://static.rust-lang.org/dist/channel-rust-${version}.toml"; \
      echo "${sha}  /tmp/channel-rust-${version}.toml" | sha256sum -c -; \
    done; \
    extra=""; \
    for component in $(printf '%s' "${RUST_TOOL_COMPONENTS}" | tr ',' ' '); do extra="${extra} -c ${component}"; done; \
    for target in $(printf '%s' "${RUST_TOOL_TARGETS}" | tr ',' ' '); do extra="${extra} -t ${target}"; done; \
    rustup toolchain install "${RUST_TOOL_VERSION}" --profile minimal ${extra}; \
    rustup toolchain install "${RUST_BITCODE_VERSION}" --profile minimal; \
    rustup default "${RUST_TOOL_VERSION}"; \
    rustup toolchain list; \
    rm -f /tmp/channel-rust-*.toml; \
    chmod -R a+rX "${RUSTUP_HOME}" "${CARGO_HOME}"

# Stage arm (CONTEXT D-08): the official Arm GNU Toolchain release archive.
FROM build-base AS arm
ARG ARM_GNU_SHA256
ARG ARM_GNU_URL
ARG SOURCE_DATE_EPOCH
ENV SOURCE_DATE_EPOCH=${SOURCE_DATE_EPOCH}
RUN set -eux; \
    : "${ARM_GNU_URL:?}" "${ARM_GNU_SHA256:?}"; \
    curl -fsSL --retry 5 --retry-delay 5 -o /tmp/arm-gnu-toolchain.tar.xz "${ARM_GNU_URL}"; \
    echo "${ARM_GNU_SHA256}  /tmp/arm-gnu-toolchain.tar.xz" | sha256sum -c -; \
    mkdir -p /opt/arm-gnu-toolchain; \
    tar -xJf /tmp/arm-gnu-toolchain.tar.xz --strip-components=1 --no-same-owner -C /opt/arm-gnu-toolchain; \
    rm -f /tmp/arm-gnu-toolchain.tar.xz; \
    chmod -R a+rX /opt/arm-gnu-toolchain

# Stage cargo-tools (CONTEXT D-09, D-11): c2rust and cargo-mutants, each crate
# checked against its pinned sha256 before an exact-version locked install.
# RUSTUP_TOOLCHAIN keeps the nightly toolchain file shipped inside the c2rust
# crate from taking over (RESEARCH Pitfall 2). This stage needs the network
# because the c2rust exporter clones tinycbor at a commit pinned inside the
# crate (RESEARCH Pitfall 3); the build asserts that commit is the pinned one.
FROM rustup AS cargo-tools
ARG C2RUST_NAME
ARG C2RUST_SHA256
ARG C2RUST_VERSION
ARG CARGO_MUTANTS_NAME
ARG CARGO_MUTANTS_SHA256
ARG CARGO_MUTANTS_VERSION
ARG LLVM_MAJOR
ARG RUST_TOOL_VERSION
ARG SOURCE_DATE_EPOCH
ARG TINYCBOR_COMMIT
ENV SOURCE_DATE_EPOCH=${SOURCE_DATE_EPOCH}
RUN set -eux; \
    : "${LLVM_MAJOR:?}" "${RUST_TOOL_VERSION:?}" "${TINYCBOR_COMMIT:?}"; \
    for spec in "${C2RUST_NAME}:${C2RUST_VERSION}:${C2RUST_SHA256}" "${CARGO_MUTANTS_NAME}:${CARGO_MUTANTS_VERSION}:${CARGO_MUTANTS_SHA256}"; do \
      name="${spec%%:*}"; \
      rest="${spec#*:}"; \
      version="${rest%%:*}"; \
      sha="${rest#*:}"; \
      curl -fsSL --retry 5 --retry-delay 5 -o "/tmp/${name}-${version}.crate" "https://static.crates.io/crates/${name}/${name}-${version}.crate"; \
      echo "${sha}  /tmp/${name}-${version}.crate" | sha256sum -c -; \
    done; \
    llvm_bin="/usr/lib/llvm-${LLVM_MAJOR}/bin"; \
    export RUSTUP_TOOLCHAIN="${RUST_TOOL_VERSION}" \
           LLVM_CONFIG_PATH="${llvm_bin}/llvm-config" \
           CLANG_PATH="${llvm_bin}/clang" \
           LIBCLANG_PATH="/usr/lib/llvm-${LLVM_MAJOR}/lib"; \
    cargo install --locked --root /opt/cargo-tools "${C2RUST_NAME}" --version "=${C2RUST_VERSION}"; \
    cargo install --locked --root /opt/cargo-tools "${CARGO_MUTANTS_NAME}" --version "=${CARGO_MUTANTS_VERSION}"; \
    grep -q "${TINYCBOR_COMMIT}" "${CARGO_HOME}"/registry/src/*/c2rust-ast-exporter-"${C2RUST_VERSION}"/src/CMakeLists.txt; \
    mkdir -p /opt/cargo-tools/share/mt; \
    "${llvm_bin}/llvm-config" --version > /opt/cargo-tools/share/mt/c2rust-llvm-config-version; \
    chmod -R a+rX /opt/cargo-tools

# Stage klee-build (CONTEXT D-07): klee-uclibc and KLEE from their pinned
# commits against the image's LLVM packages, Z3 as the only solver. Only the
# install tree is copied into the final image. The assertion build option is
# off because the packaged LLVM is built without assertions (RESEARCH Pitfall 4).
FROM build-base AS klee-build
ARG KLEE_COMMIT
ARG KLEE_TAG
ARG KLEE_UCLIBC_COMMIT
ARG KLEE_UCLIBC_TAG
ARG KLEE_UCLIBC_URL
ARG KLEE_URL
ARG LLVM_MAJOR
ARG SOURCE_DATE_EPOCH
ENV SOURCE_DATE_EPOCH=${SOURCE_DATE_EPOCH}
RUN set -eux; \
    : "${LLVM_MAJOR:?}" "${KLEE_UCLIBC_URL:?}" "${KLEE_UCLIBC_TAG:?}" "${KLEE_UCLIBC_COMMIT:?}"; \
    llvm_bin="/usr/lib/llvm-${LLVM_MAJOR}/bin"; \
    git clone --depth 1 --branch "${KLEE_UCLIBC_TAG}" "${KLEE_UCLIBC_URL}" /src/klee-uclibc; \
    test "$(git -C /src/klee-uclibc rev-parse HEAD)" = "${KLEE_UCLIBC_COMMIT}"; \
    cd /src/klee-uclibc; \
    ./configure --make-llvm-lib --with-cc "${llvm_bin}/clang" --with-llvm-config "${llvm_bin}/llvm-config"; \
    make; \
    test -f /src/klee-uclibc/lib/libc.a
RUN set -eux; \
    : "${LLVM_MAJOR:?}" "${KLEE_URL:?}" "${KLEE_TAG:?}" "${KLEE_COMMIT:?}"; \
    llvm_dir="/usr/lib/llvm-${LLVM_MAJOR}"; \
    git clone --depth 1 --branch "${KLEE_TAG}" "${KLEE_URL}" /src/klee; \
    test "$(git -C /src/klee rev-parse HEAD)" = "${KLEE_COMMIT}"; \
    cmake -S /src/klee -B /build/klee -G Ninja \
      -DCMAKE_BUILD_TYPE=Release \
      -DCMAKE_INSTALL_PREFIX=/opt/klee \
      -DLLVM_DIR="${llvm_dir}/lib/cmake/llvm" \
      -DLLVMCC="${llvm_dir}/bin/clang" \
      -DLLVMCXX="${llvm_dir}/bin/clang++" \
      -DENABLE_SOLVER_Z3=ON \
      -DENABLE_SOLVER_STP=OFF \
      -DENABLE_SOLVER_METASMT=OFF \
      -DENABLE_KLEE_ASSERTS=OFF \
      -DENABLE_POSIX_RUNTIME=ON \
      -DKLEE_UCLIBC_PATH=/src/klee-uclibc \
      -DENABLE_KLEE_LIBCXX=OFF \
      -DENABLE_TCMALLOC=OFF \
      -DENABLE_UNIT_TESTS=OFF \
      -DENABLE_SYSTEM_TESTS=OFF \
      -DENABLE_DOCS=OFF; \
    cmake --build /build/klee; \
    cmake --install /build/klee; \
    test -n "$(find /opt/klee -name klee-uclibc.bca -print -quit)"; \
    chmod -R a+rX /opt/klee

# Stage final: the runtime set plus the install trees of the stages above, and
# nothing from the build context. The mt binary and pins.toml are mounted at
# /opt/mt and /usr/local/bin/mt when the image runs, never baked in, so the
# image digest depends only on this file and pins.toml.
FROM apt-base AS final
ARG CARGO_HOME
ARG LLVM_MAJOR
ARG RUSTUP_HOME
ARG SOURCE_DATE_EPOCH
ARG TOOL_PATH
COPY --from=rustup ${RUSTUP_HOME} ${RUSTUP_HOME}
COPY --from=rustup ${CARGO_HOME} ${CARGO_HOME}
COPY --from=cargo-tools /opt/cargo-tools /opt/cargo-tools
COPY --from=klee-build /opt/klee /opt/klee
COPY --from=arm /opt/arm-gnu-toolchain /opt/arm-gnu-toolchain
ENV PATH="${TOOL_PATH}" \
    RUSTUP_HOME="${RUSTUP_HOME}" \
    CARGO_HOME="${CARGO_HOME}" \
    LLVM_CONFIG_PATH="/usr/lib/llvm-${LLVM_MAJOR}/bin/llvm-config" \
    SOURCE_DATE_EPOCH="${SOURCE_DATE_EPOCH}"
RUN set -eux; \
    : "${TOOL_PATH:?}"; \
    mkdir -p /opt/mt
