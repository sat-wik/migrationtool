/*
 * Testbed user configuration for Mbed TLS (RESEARCH, Mbed TLS user-config
 * paragraph; plan 02-09).
 *
 * This file is part of every Mbed TLS configuration of the test bed: the build
 * passes it as MBEDTLS_USER_CONFIG_FILE, and it is hashed into each capture
 * report as an input. It removes the modules that need an operating system
 * (sockets, time, files, threads, persistent storage, platform entropy) so that
 * the library builds for a bare-metal Cortex-M with the Arm compiler. It adds
 * nothing to the parser code that is being validated.
 */
#ifndef MT_TESTBED_MBEDTLS_USER_CONFIG_H
#define MT_TESTBED_MBEDTLS_USER_CONFIG_H

#undef MBEDTLS_NET_C
#undef MBEDTLS_TIMING_C
#undef MBEDTLS_FS_IO
#undef MBEDTLS_HAVE_TIME_DATE
#undef MBEDTLS_HAVE_TIME
#undef MBEDTLS_PSA_ITS_FILE_C
#undef MBEDTLS_PSA_CRYPTO_STORAGE_C
#undef MBEDTLS_THREADING_C
#undef MBEDTLS_THREADING_PTHREAD

#define MBEDTLS_NO_PLATFORM_ENTROPY

#endif /* MT_TESTBED_MBEDTLS_USER_CONFIG_H */
