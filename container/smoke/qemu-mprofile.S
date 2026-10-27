/*
 * Minimal Thumb guest for the M-profile user-mode start probe (CONTEXT D-31).
 *
 * It proves two things about the QEMU in the image: `qemu-arm -cpu cortex-m4`
 * starts a linux-user guest at all (QEMU 7.2 aborts at startup for M-profile
 * CPUs), and the guest exit code from a raw exit_group system call reaches the
 * host. The executor drivers of phase 03 (D-30) use the same entry style. It
 * makes no semihosting call.
 *
 * Build and run (the container workflow does exactly this in the image):
 *   arm-none-eabi-gcc -mcpu=cortex-m4 -mthumb -nostdlib -static \
 *       -o qemu-mprofile.elf container/smoke/qemu-mprofile.S
 *   qemu-arm -cpu cortex-m4 -R 0x10000000 qemu-mprofile.elf; echo $?
 * The expected exit code is 42.
 */
    .syntax unified
    .thumb
    .text
    .global _start
    .type _start, %function
    .thumb_func
_start:
    movs r0, #42        /* exit code */
    movs r7, #248       /* exit_group */
    svc #0
1:  b 1b                /* not reached */
    .size _start, . - _start
