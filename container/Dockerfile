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
