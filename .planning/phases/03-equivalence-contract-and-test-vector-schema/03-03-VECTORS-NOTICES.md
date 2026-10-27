# Plan 03-03 round trip 1: vectors notices

Run 40456178214 (testbed), head be0f323214bbb4b1c53b009b1896a0663debe0dd, job 122030154846, read from the job annotations with `gh api` (parts joined in order, final newline dropped as the workflow's annotate helper specifies). The executor self-test and the TRACTOR sample step ran in the pinned image; the TRACTOR fetch is the only networked step this plan adds.

## vectors-selftest (1 annotation part(s))

```text
tool qemu-arm qemu-arm version 10.0.2 (Debian 1:10.0.2+ds-2+deb13u1~bpo12+1)
tool arm-none-eabi-gcc arm-none-eabi-gcc (Arm GNU Toolchain 14.3.Rel1 (Build arm-14.174)) 14.3.1 20250623
tool arm-none-eabi-ar GNU ar (Arm GNU Toolchain 14.3.Rel1 (Build arm-14.174)) 2.44.0.20250616
tool arm-none-eabi-nm GNU nm (Arm GNU Toolchain 14.3.Rel1 (Build arm-14.174)) 2.44.0.20250616
tool llvm-dwarfdump Debian LLVM version 16.0.6
cpu cortex-m4 listed
case ok outcome=ok input_len=5
case overrun-input outcome=fault rc=-11 label=crash:SIGSEGV
case overrun-output outcome=fault rc=-11 label=crash:SIGSEGV
case slack-write outcome=fault rc=0 label=oob-write:guard-slack
case abort outcome=abort rc=-6 label=crash:SIGABRT
case loop outcome=timeout rc=-9 label=timeout
case beyond-reserve outcome=executor_error exit=98
case stack-overflow outcome=fault rc=-11 label=crash:SIGSEGV
no core file after overrun
determinism 100 runs 1 hash
dwarf enum byte_size 1
result: passed (17 rows)
exit code: 0
```

## vectors-tractor (1 annotation part(s))

```text
git init ok
git fetch ok
git sparse-checkout ok
git checkout ok
git rev-parse ok
fetched 6 pinned files into /work/tractor/37960ee08a7c644ee66e29855eca33903f702e67/tree
Public-Tests/B01_synthetic/001_helloworld_lib/test_vectors/test1.json sha256=ok keys=lib_state_in,lib_state_out,stdout round_trip=json-equal has_ub=absent
Public-Tests/B01_synthetic/007_errno_pow_lib/test_vectors/test11.json sha256=ok keys=lib_state_in,lib_state_out,stderr round_trip=json-equal has_ub=absent
Public-Tests/B01_synthetic/030_mutable_buffer_overlap_extrahard_lib/test_vectors/test04.json sha256=ok keys=has_ub,lib_state_in,lib_state_out round_trip=json-equal has_ub=present
Public-Tests/B01_synthetic/009_stack_buffer_overflow_lib/test_vectors/test09_overflow_int2.json sha256=ok keys=argv,has_ub,lib_state_in,lib_state_out,rc round_trip=json-equal has_ub=present
Public-Tests/B01_synthetic/002_stdin_echo/test_vectors/nul.json sha256=ok keys=stdin,stdout round_trip=json-equal has_ub=absent
Public-Tests/B02_synthetic/strcpy/test_vectors/41.json sha256=ok keys=argv,has_ub,rc,stderr,stdin,stdout round_trip=json-equal has_ub=present
loaded: 6 of 6 (at least 3 required)
tractor: passed
exit code: 0
```
