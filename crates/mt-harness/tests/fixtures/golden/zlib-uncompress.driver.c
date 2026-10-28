/* The shared executor runtime template (EQV-04; CONTEXT D-01, D-02, D-04, D-29, D-30).
 *
 * One reviewed file for every target. Nothing in it names a target, an entry
 * or a header of the test bed; the harness renders it into a driver by
 * replacing the closed set of placeholders written at-sign, MT_, NAME,
 * at-sign. Each placeholder appears exactly once in this file.
 *
 * The driver is a hosted-style Linux-ABI static ELF, not firmware. It has its
 * own _start, its own stack mapping and raw svc system calls, and it runs
 * under qemu-arm user mode with the Cortex-M4 CPU model. It makes no
 * semihosting call.
 *
 * Exit codes of the driver's own failures (all executor_error on the host):
 *   95 the C code called exit, 96 the request file could not be opened or read,
 *   97 the request is malformed, 98 a mapping failed (including one beyond the
 *   reserved address space), 99 any other setup failure.
 * A fault signal exits 128 + signal from an in-guest handler, so the host never
 * sees a core file or a host signal.
 *
 * Request file (little-endian): "MTRQ", version 1, field count, then per field
 * a length and the bytes. The fields are: 0 a selector word (self-test only),
 * 1 the input bytes, 2 the output capacity word, and for an entry driver also
 * 3 the scalar arguments (eight bytes each, two's complement) and 4 a dump
 * plan: pairs of an address and a size (eight bytes each) of the writable
 * globals to snapshot, and 5 the strings of the call: each a length word and
 * the bytes, no terminator. A request has 3, 5 or 6 fields.
 *
 * Result frame (little-endian), written after the call: "MTRS", version 1,
 * status (0 when the entry ran, 0x100 + k when setup step k returned something other
 * than its expectation), raw return value (that step's, then), bytes the C code wrote
 * through libc (discarded), slack check result, region count, then per region an id,
 * a length and the bytes, then "MTRE". Region 1 is the whole output buffer; region 2 holds the
 * addresses of the driver's own objects (input and output buffer, the heap
 * arena in use, each state), as words, so the host can classify pointers;
 * region 3 is the input buffer after the call and region 4 the used bytes of the
 * heap arena, from its first byte;
 * region 0x1000 + i is dump range i before the call and region 0x2000 + i the
 * same range after it; region 0x3000 + i is state i before the call and region
 * 0x4000 + i the same state after it.
 *
 * Each state of an entry (a pointer parameter the harness fills in) lives in its
 * own guarded, zero-filled mapping at a fixed base. The set of states and their
 * sizes come from the generated placeholder at-sign, MT_, STATES, at-sign. Each
 * string of the request lives in its own guarded mapping with a trailing NUL, so
 * a declared string reaches the C code as data and never as source text.
 */
#include <errno.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>

/* The entry's own includes: the defining unit's prefix and the entry header.
 * Empty for the self-test. */
#include "/fixed/zlib.h"

typedef uint8_t u8;
typedef uint32_t u32;

/* The states of the entry (the parameters it fills in, then its locals): MT_STATE_COUNT,
 * mt_state_sizes (the size in bytes of each, followed by a zero) and mt_state_word (the
 * request word that holds the value of a state before the call, or -1, followed by -1).
 * None for the self-test. */
#define MT_STATE_COUNT 1u
static const u32 mt_state_sizes[MT_STATE_COUNT + 1u] = { (u32)sizeof(uLongf), 0u };
static const int mt_state_word[MT_STATE_COUNT + 1u] = { 0, -1 };

#define MT_RESERVED_VA 0x10000000u
#define MT_HEAP_ARENA 1048576u
#define MT_SENTINEL 165u
#define MT_SLACK_FILL 90u
#define MT_MAX_INPUT 65536u

#define MT_PAGE 4096u
#define MT_IN_BASE 0x04000000u
#define MT_OUT_BASE 0x05000000u
#define MT_STACK_BASE 0x07000000u
#define MT_STACK_PAGES 64u
#define MT_MAX_FIELDS 8u
#define MT_DUMP_MAX_RANGES 4096u
#define MT_DUMP_TOTAL (512u * 1024u)
#define MT_REGION_OUT 1u
#define MT_REGION_ADDRESSES 2u
#define MT_REGION_IN_AFTER 3u
#define MT_REGION_HEAP 4u
#define MT_REGION_BEFORE 0x1000u
#define MT_REGION_AFTER 0x2000u
#define MT_REGION_STATE_BEFORE 0x3000u
#define MT_REGION_STATE_AFTER 0x4000u
#define MT_MAX_STATES 8u
#define MT_STATE_BASE 0x06000000u
#define MT_STATE_STRIDE 0x00100000u
#define MT_STATE_MAX_BYTES 65536u
#define MT_MAX_STRS 8u
#define MT_STR_BASE 0x06800000u
#define MT_STR_MAX_BYTES 64u
#define MT_REQ_MAX (MT_MAX_INPUT + 4096u)

#define MT_EXIT_CALLED 95
#define MT_EXIT_REQUEST_IO 96
#define MT_EXIT_REQUEST_BAD 97
#define MT_EXIT_MAP 98
#define MT_EXIT_SETUP 99

/* Every mapping must lie below the reserved address space (qemu-arm -R). */
_Static_assert(MT_STACK_BASE + (MT_STACK_PAGES + 1u) * MT_PAGE <= MT_RESERVED_VA,
               "the stack mapping must lie below the reserved address space");
_Static_assert(MT_OUT_BASE + (MT_MAX_INPUT / MT_PAGE + 3u) * MT_PAGE <= MT_STATE_BASE,
               "the output mapping must end below the state mappings");
_Static_assert(MT_STATE_BASE + MT_MAX_STATES * MT_STATE_STRIDE <= MT_STR_BASE,
               "the state mappings must end below the string mappings");
_Static_assert(MT_STR_BASE + MT_MAX_STRS * MT_STATE_STRIDE <= MT_STACK_BASE,
               "the string mappings must end below the stack mapping");
_Static_assert(MT_STATE_COUNT <= MT_MAX_STATES, "too many states");
_Static_assert(MT_STATE_MAX_BYTES + 3u * MT_PAGE <= MT_STATE_STRIDE,
               "a state mapping must fit its stride");

/* ---- raw Linux ARM EABI system calls: svc #0, r7 holds the number ---- */

#define SYS_EXIT_GROUP 248 /* the exit_group system call ends the whole driver */
#define SYS_READ 3
#define SYS_WRITE 4
#define SYS_OPEN 5
#define SYS_MPROTECT 125
#define SYS_RT_SIGACTION 174
#define SYS_SIGALTSTACK 186
#define SYS_MMAP2 192

#define MT_MAP_RW_FIXED 0x32 /* private, anonymous, fixed */

/* r7 may be the frame pointer in Thumb code, so it is saved in ip around the call. */
static inline long sc(long nr, long a, long b, long c, long d, long e, long f)
{
    register long r0 __asm__("r0") = a;
    register long r1 __asm__("r1") = b;
    register long r2 __asm__("r2") = c;
    register long r3 __asm__("r3") = d;
    register long r4 __asm__("r4") = e;
    register long r5 __asm__("r5") = f;
    __asm__ volatile("mov ip, r7\n\t"
                     "mov r7, %[nr]\n\t"
                     "svc #0\n\t"
                     "mov r7, ip"
                     : "+r"(r0)
                     : [nr] "r"(nr), "r"(r1), "r"(r2), "r"(r3), "r"(r4), "r"(r5)
                     : "ip", "memory");
    return r0;
}

static void mt_exit(int code) __attribute__((noreturn));
static void mt_exit(int code)
{
    for (;;) {
        sc(SYS_EXIT_GROUP, code, 0, 0, 0, 0, 0);
    }
}

/* ---- guarded mappings (D-04) ----
 * (pages + 2) pages: an unmapped guard page, the data pages, an unmapped guard
 * page. The buffer is right-aligned so its last byte is the last byte before the
 * end guard (exact). The start guard is exact only when the length is a multiple
 * of the page size; the bytes before the buffer inside the first data page are
 * slack, prefilled with a pattern and re-checked after the call. A zero-length
 * buffer points at the end guard. */

struct guarded {
    u8 *buf;
    u32 len;
    u8 *slack;
    u32 slack_len;
};

static void mt_guard_alloc(struct guarded *g, u32 base, u32 len)
{
    u32 pages = len ? (len + MT_PAGE - 1u) / MT_PAGE : 1u;
    u32 i;
    if (sc(SYS_MMAP2, (long)base, (long)((pages + 2u) * MT_PAGE), 3, MT_MAP_RW_FIXED, -1, 0) !=
        (long)base) {
        mt_exit(MT_EXIT_MAP);
    }
    if (sc(SYS_MPROTECT, (long)base, (long)MT_PAGE, 0, 0, 0, 0) != 0) {
        mt_exit(MT_EXIT_SETUP);
    }
    if (sc(SYS_MPROTECT, (long)(base + (pages + 1u) * MT_PAGE), (long)MT_PAGE, 0, 0, 0, 0) != 0) {
        mt_exit(MT_EXIT_SETUP);
    }
    g->slack = (u8 *)(base + MT_PAGE);
    g->slack_len = pages * MT_PAGE - len;
    g->buf = g->slack + g->slack_len;
    g->len = len;
    for (i = 0; i < g->slack_len; i++) {
        g->slack[i] = (u8)MT_SLACK_FILL;
    }
}

/* 1 when any slack byte of the buffer changed, else 0. */
static u32 mt_slack_changed(const struct guarded *g)
{
    u32 i;
    for (i = 0; i < g->slack_len; i++) {
        if (g->slack[i] != (u8)MT_SLACK_FILL) {
            return 1u;
        }
    }
    return 0u;
}

/* ---- the states of an entry: each in its own guarded, zero-filled mapping ---- */

static struct guarded mt_state_map[MT_STATE_COUNT + 1u];
static u8 mt_state_snapshot[MT_MAX_STATES * MT_STATE_MAX_BYTES];

static void mt_states_alloc(void)
{
    u32 i;
    for (i = 0; i < MT_STATE_COUNT; i++) {
        u32 size = mt_state_sizes[i];
        if (size == 0u || size > MT_STATE_MAX_BYTES) {
            mt_exit(MT_EXIT_SETUP);
        }
        mt_guard_alloc(&mt_state_map[i], MT_STATE_BASE + i * MT_STATE_STRIDE, size);
        memset(mt_state_map[i].buf, 0, size);
    }
}

/* The i-th state's object, for the call expression. */
static void *mt_state(u32 index) __attribute__((unused));
static void *mt_state(u32 index)
{
    if (index >= MT_STATE_COUNT) {
        mt_exit(MT_EXIT_REQUEST_BAD);
    }
    return mt_state_map[index].buf;
}

static u32 mt_states_slack_changed(void)
{
    u32 i;
    u32 changed = 0u;
    for (i = 0; i < MT_STATE_COUNT; i++) {
        changed |= mt_slack_changed(&mt_state_map[i]);
    }
    return changed;
}

static void mt_states_snapshot(void)
{
    u32 i;
    for (i = 0; i < MT_STATE_COUNT; i++) {
        memcpy(mt_state_snapshot + (size_t)i * MT_STATE_MAX_BYTES, mt_state_map[i].buf,
               mt_state_sizes[i]);
    }
}

/* ---- fault handlers: exit 128 + signal on an alternate stack, no core file ---- */

struct mt_ksigaction {
    void (*handler)(int);
    unsigned long flags;
    void (*restorer)(void);
    unsigned long long mask;
};

struct mt_kstack {
    void *sp;
    int flags;
    unsigned long size;
};

#define MT_SA_ONSTACK 0x08000000ul
#define MT_SIGSET_SIZE 8 /* rt_sigaction fails silently without it */

static u8 mt_altstack[8192] __attribute__((aligned(8)));

static void mt_on_signal(int sig)
{
    mt_exit(128 + sig);
}

static void mt_install_handlers(void)
{
    static const int signals[] = {11, 7, 4, 8, 6}; /* SEGV, BUS, ILL, FPE, ABRT */
    struct mt_kstack stack;
    unsigned i;
    stack.sp = mt_altstack;
    stack.flags = 0;
    stack.size = sizeof mt_altstack;
    if (sc(SYS_SIGALTSTACK, (long)&stack, 0, 0, 0, 0, 0) != 0) {
        mt_exit(MT_EXIT_SETUP);
    }
    for (i = 0; i < sizeof signals / sizeof signals[0]; i++) {
        struct mt_ksigaction act;
        act.handler = mt_on_signal;
        act.flags = MT_SA_ONSTACK;
        act.restorer = 0;
        act.mask = 0;
        if (sc(SYS_RT_SIGACTION, signals[i], (long)&act, 0, MT_SIGSET_SIZE, 0, 0) != 0) {
            mt_exit(MT_EXIT_SETUP);
        }
    }
}

/* ---- newlib system-call stubs (nosys.specs): libc output never reaches the frame ---- */

static u8 mt_heap_arena[MT_HEAP_ARENA] __attribute__((aligned(8)));
static u32 mt_heap_used;
static u32 mt_discarded; /* bytes the C code wrote through libc, discarded */

void *_sbrk(int incr)
{
    if (incr < 0 || (u32)incr > MT_HEAP_ARENA - mt_heap_used) {
        errno = ENOMEM;
        return (void *)-1;
    }
    {
        void *previous = mt_heap_arena + mt_heap_used;
        mt_heap_used += (u32)incr;
        return previous;
    }
}

int _write(int fd, const char *buf, int len)
{
    (void)fd;
    (void)buf;
    if (len > 0) {
        mt_discarded += (u32)len;
    }
    return len;
}

int _read(int fd, char *buf, int len)
{
    (void)fd;
    (void)buf;
    (void)len;
    return 0;
}

int _close(int fd)
{
    (void)fd;
    return 0;
}

int _lseek(int fd, int offset, int whence)
{
    (void)fd;
    (void)offset;
    (void)whence;
    return 0;
}

int _fstat(int fd, struct stat *st)
{
    (void)fd;
    st->st_mode = S_IFCHR;
    return 0;
}

int _isatty(int fd)
{
    (void)fd;
    return 1;
}

int _getpid(void)
{
    return 1;
}

/* abort() and a failed assert end here: the fault exit for SIGABRT (134). */
int _kill(int pid, int sig)
{
    (void)pid;
    mt_exit(128 + sig);
}

void _exit(int status)
{
    (void)status;
    mt_exit(MT_EXIT_CALLED);
}

/* Present so __libc_init_array links under -nostartfiles. */
void _init(void)
{
}

void _fini(void)
{
}

extern void __libc_init_array(void);

/* ---- the request ---- */

struct mt_field {
    const u8 *data;
    u32 len;
};

static u8 mt_req_buf[MT_REQ_MAX + 1u];
static struct mt_field mt_fields[MT_MAX_FIELDS];
static u32 mt_field_count;

/* The scalar arguments of an entry driver: eight little-endian bytes each. */
static const u8 *mt_scalars;
static u32 mt_scalar_count;

/* The dump plan: writable globals snapshotted before and after the call. */
struct mt_range {
    u32 address;
    u32 size;
};

static struct mt_range mt_ranges[MT_DUMP_MAX_RANGES];
static u32 mt_range_count;
static u8 mt_dump_before[MT_DUMP_TOTAL];

static u32 mt_get32(const u8 *p)
{
    return (u32)p[0] | ((u32)p[1] << 8) | ((u32)p[2] << 16) | ((u32)p[3] << 24);
}

static void mt_put32(u8 *p, u32 v)
{
    p[0] = (u8)v;
    p[1] = (u8)(v >> 8);
    p[2] = (u8)(v >> 16);
    p[3] = (u8)(v >> 24);
}

static void mt_load_request(const char *path)
{
    long fd = sc(SYS_OPEN, (long)path, 0, 0, 0, 0, 0);
    u32 total = 0;
    u32 pos;
    u32 i;
    if (fd < 0) {
        mt_exit(MT_EXIT_REQUEST_IO);
    }
    while (total < sizeof mt_req_buf) {
        long n = sc(SYS_READ, fd, (long)(mt_req_buf + total), (long)(sizeof mt_req_buf - total), 0,
                    0, 0);
        if (n < 0) {
            mt_exit(MT_EXIT_REQUEST_IO);
        }
        if (n == 0) {
            break;
        }
        total += (u32)n;
    }
    if (total < 12u || total > MT_REQ_MAX) {
        mt_exit(MT_EXIT_REQUEST_BAD);
    }
    if (mt_req_buf[0] != 'M' || mt_req_buf[1] != 'T' || mt_req_buf[2] != 'R' ||
        mt_req_buf[3] != 'Q' || mt_get32(mt_req_buf + 4) != 1u) {
        mt_exit(MT_EXIT_REQUEST_BAD);
    }
    mt_field_count = mt_get32(mt_req_buf + 8);
    if (mt_field_count > MT_MAX_FIELDS) {
        mt_exit(MT_EXIT_REQUEST_BAD);
    }
    pos = 12u;
    for (i = 0; i < mt_field_count; i++) {
        u32 len;
        if (total - pos < 4u) {
            mt_exit(MT_EXIT_REQUEST_BAD);
        }
        len = mt_get32(mt_req_buf + pos);
        pos += 4u;
        if (total - pos < len) {
            mt_exit(MT_EXIT_REQUEST_BAD);
        }
        mt_fields[i].data = mt_req_buf + pos;
        mt_fields[i].len = len;
        pos += len;
    }
    if (pos != total) {
        mt_exit(MT_EXIT_REQUEST_BAD);
    }
}

/* Read the dump plan and snapshot the ranges as they are before the call. */
static void mt_load_plan(const u8 *plan, u32 count)
{
    u32 i;
    u32 total = 0;
    if (count > MT_DUMP_MAX_RANGES) {
        mt_exit(MT_EXIT_REQUEST_BAD);
    }
    for (i = 0; i < count; i++) {
        mt_ranges[i].address = mt_get32(plan + (size_t)i * 8u);
        mt_ranges[i].size = mt_get32(plan + (size_t)i * 8u + 4u);
        if (mt_ranges[i].size > MT_DUMP_TOTAL - total) {
            mt_exit(MT_EXIT_REQUEST_BAD);
        }
        total += mt_ranges[i].size;
    }
    mt_range_count = count;
}

static void mt_snapshot_before(void)
{
    u32 i;
    u32 off = 0;
    for (i = 0; i < mt_range_count; i++) {
        memcpy(mt_dump_before + off, (const void *)mt_ranges[i].address, mt_ranges[i].size);
        off += mt_ranges[i].size;
    }
}

/* The index-th scalar argument as a signed 64-bit value. */
static long long mt_scalar(u32 index)
{
    const u8 *p;
    unsigned long long value;
    if (index >= mt_scalar_count) {
        mt_exit(MT_EXIT_REQUEST_BAD);
    }
    p = mt_scalars + (size_t)index * 8u;
    value = (unsigned long long)mt_get32(p) | ((unsigned long long)mt_get32(p + 4) << 32);
    return (long long)value;
}

/* ---- the strings of a request: each in its own guarded mapping, NUL-terminated ---- */

static struct guarded mt_str_map[MT_MAX_STRS];
static u32 mt_str_count;

/* The block is a sequence of a length word and that many bytes, no terminator. */
static void mt_strs_load(const u8 *block, u32 total)
{
    u32 pos = 0u;
    while (pos < total) {
        u32 len;
        struct guarded *g;
        if (mt_str_count >= MT_MAX_STRS || total - pos < 4u) {
            mt_exit(MT_EXIT_REQUEST_BAD);
        }
        len = mt_get32(block + pos);
        pos += 4u;
        if (len > MT_STR_MAX_BYTES || total - pos < len) {
            mt_exit(MT_EXIT_REQUEST_BAD);
        }
        g = &mt_str_map[mt_str_count];
        mt_guard_alloc(g, MT_STR_BASE + mt_str_count * MT_STATE_STRIDE, len + 1u);
        memcpy(g->buf, block + pos, len);
        g->buf[len] = 0;
        pos += len;
        mt_str_count++;
    }
}

/* The index-th string of the request, for the call expression. */
static const char *mt_str(u32 index) __attribute__((unused));
static const char *mt_str(u32 index)
{
    if (index >= mt_str_count) {
        mt_exit(MT_EXIT_REQUEST_BAD);
    }
    return (const char *)mt_str_map[index].buf;
}

/* Its length in bytes, without the NUL. */
static u32 mt_str_len(u32 index) __attribute__((unused));
static u32 mt_str_len(u32 index)
{
    if (index >= mt_str_count) {
        mt_exit(MT_EXIT_REQUEST_BAD);
    }
    return mt_str_map[index].len - 1u;
}

static u32 mt_strs_slack_changed(void)
{
    u32 i;
    u32 changed = 0u;
    for (i = 0; i < mt_str_count; i++) {
        changed |= mt_slack_changed(&mt_str_map[i]);
    }
    return changed;
}

/* A state that holds an inout_len starts with the declared value, which the request
 * carries as a scalar word (mt_state_word[i] is the word, or -1); every other state
 * stays zero-filled. The value is written little-endian, as many bytes as the type
 * has, so the snapshot before the call shows it. */
static void mt_states_init(void)
{
    u32 i;
    for (i = 0; i < MT_STATE_COUNT; i++) {
        if (mt_state_word[i] >= 0) {
            unsigned long long value = (unsigned long long)mt_scalar((u32)mt_state_word[i]);
            u32 n = mt_state_sizes[i] < 8u ? mt_state_sizes[i] : 8u;
            u32 k;
            for (k = 0; k < n; k++) {
                mt_state_map[i].buf[k] = (u8)(value >> (8u * k));
            }
        }
    }
}

/* ---- the cases (generated by the harness) ---- */



/* ---- the result frame ---- */

static void mt_write_all(const u8 *data, u32 len)
{
    u32 off = 0;
    while (off < len) {
        long w = sc(SYS_WRITE, 1, (long)(data + off), (long)(len - off), 0, 0, 0);
        if (w <= 0) {
            mt_exit(MT_EXIT_SETUP);
        }
        off += (u32)w;
    }
}

static void mt_write_region(u32 id, const u8 *data, u32 len)
{
    u8 head[8];
    mt_put32(head, id);
    mt_put32(head + 4, len);
    mt_write_all(head, 8u);
    mt_write_all(data, len);
}

/* The addresses of the driver's own objects, as words: the input buffer and its
 * length, the output buffer and its capacity, the heap arena in use, then each
 * state's address and size. The host builds its pointer regions from them. */
static void mt_write_addresses(const struct guarded *in_buf, const struct guarded *out_buf)
{
    u8 words[(6u + 2u * MT_MAX_STATES) * 4u];
    u32 n = 0u;
    u32 i;
    mt_put32(words + 4u * n++, (u32)(size_t)in_buf->buf);
    mt_put32(words + 4u * n++, in_buf->len);
    mt_put32(words + 4u * n++, (u32)(size_t)out_buf->buf);
    mt_put32(words + 4u * n++, out_buf->len);
    mt_put32(words + 4u * n++, (u32)(size_t)mt_heap_arena);
    mt_put32(words + 4u * n++, mt_heap_used);
    for (i = 0; i < MT_STATE_COUNT; i++) {
        mt_put32(words + 4u * n++, (u32)(size_t)mt_state_map[i].buf);
        mt_put32(words + 4u * n++, mt_state_sizes[i]);
    }
    mt_write_region(MT_REGION_ADDRESSES, words, 4u * n);
}

/* The frame is written in pieces straight from the buffers: a header, the output
 * region, the addresses, the input after the call, the used bytes of the heap
 * arena, the before and after region of each dump range and of each state, the
 * end marker. */
static void mt_emit_frame(u32 status, u32 ret, u32 slack, const struct guarded *in_buf,
                          const struct guarded *out_buf)
{
    u8 head[28];
    u32 i;
    u32 off = 0;
    head[0] = 'M';
    head[1] = 'T';
    head[2] = 'R';
    head[3] = 'S';
    mt_put32(head + 4, 1u);
    mt_put32(head + 8, status);
    mt_put32(head + 12, ret);
    mt_put32(head + 16, mt_discarded);
    mt_put32(head + 20, slack);
    mt_put32(head + 24, 4u + 2u * mt_range_count + 2u * MT_STATE_COUNT);
    mt_write_all(head, 28u);
    mt_write_region(MT_REGION_OUT, out_buf->buf, out_buf->len);
    mt_write_addresses(in_buf, out_buf);
    mt_write_region(MT_REGION_IN_AFTER, in_buf->buf, in_buf->len);
    mt_write_region(MT_REGION_HEAP, mt_heap_arena, mt_heap_used);
    for (i = 0; i < mt_range_count; i++) {
        mt_write_region(MT_REGION_BEFORE + i, mt_dump_before + off, mt_ranges[i].size);
        off += mt_ranges[i].size;
    }
    for (i = 0; i < mt_range_count; i++) {
        mt_write_region(MT_REGION_AFTER + i, (const u8 *)mt_ranges[i].address, mt_ranges[i].size);
    }
    for (i = 0; i < MT_STATE_COUNT; i++) {
        mt_write_region(MT_REGION_STATE_BEFORE + i, mt_state_snapshot + (size_t)i * MT_STATE_MAX_BYTES,
                        mt_state_sizes[i]);
    }
    for (i = 0; i < MT_STATE_COUNT; i++) {
        mt_write_region(MT_REGION_STATE_AFTER + i, mt_state_map[i].buf, mt_state_sizes[i]);
    }
    mt_write_all((const u8 *)"MTRE", 4u);
}

/* ---- the run: on the driver's own stack ---- */

void mt_main(const char *path) __attribute__((noreturn, used));
void mt_main(const char *path)
{
    struct guarded in_buf;
    struct guarded out_buf;
    u8 *mt_in;
    u32 mt_in_len;
    u8 *mt_out;
    u32 mt_out_cap;
    u32 selector;
    u32 out_cap;
    u32 ret;
    u32 slack;
    u32 setup_status;
    u32 setup_ret;
    mt_install_handlers();
    mt_load_request(path);
    __libc_init_array();
    /* Request: selector word, input bytes, output capacity word and, for an
     * entry driver, the scalar block, the dump plan and the strings. */
    if ((mt_field_count != 3u && mt_field_count != 5u && mt_field_count != 6u) ||
        mt_fields[0].len != 4u || mt_fields[2].len != 4u) {
        mt_exit(MT_EXIT_REQUEST_BAD);
    }
    if (mt_field_count >= 5u) {
        if (mt_fields[3].len % 8u != 0u || mt_fields[4].len % 8u != 0u) {
            mt_exit(MT_EXIT_REQUEST_BAD);
        }
        mt_scalars = mt_fields[3].data;
        mt_scalar_count = mt_fields[3].len / 8u;
        mt_load_plan(mt_fields[4].data, mt_fields[4].len / 8u);
    }
    if (mt_field_count == 6u) {
        mt_strs_load(mt_fields[5].data, mt_fields[5].len);
    }
    selector = mt_get32(mt_fields[0].data);
    out_cap = mt_get32(mt_fields[2].data);
    if (out_cap > MT_MAX_INPUT) {
        mt_exit(MT_EXIT_REQUEST_BAD);
    }
    mt_guard_alloc(&in_buf, MT_IN_BASE, mt_fields[1].len);
    memcpy(in_buf.buf, mt_fields[1].data, mt_fields[1].len);
    mt_guard_alloc(&out_buf, MT_OUT_BASE, out_cap);
    memset(out_buf.buf, (int)MT_SENTINEL, out_cap);
    mt_in = in_buf.buf;
    mt_in_len = mt_fields[1].len;
    mt_out = out_buf.buf;
    mt_out_cap = out_cap;
    (void)selector;
    (void)mt_in;
    (void)mt_in_len;
    (void)mt_out;
    (void)mt_out_cap;
    mt_states_alloc();
    mt_states_init();
    /* The declared setup steps (D-12), generated by the harness from a closed grammar:
     * each runs only while every earlier one returned what it expected; the first
     * mismatch records its step and return value and the entry is skipped. The state
     * after setup is the state before the call. */
    setup_status = 0u;
    setup_ret = 0u;
    /* no setup steps */
    (void)setup_ret;
    mt_snapshot_before();
    mt_states_snapshot();
    if (setup_status == 0u) {
        ret = (u32)((int32_t)(uncompress((Bytef *)mt_out, (uLongf *)mt_state(0), (const Bytef *)mt_in, (uLong)mt_in_len)));
    } else {
        ret = setup_ret;
    }
    fflush(NULL);
    slack = mt_slack_changed(&in_buf) | mt_slack_changed(&out_buf) | mt_states_slack_changed() |
            mt_strs_slack_changed();
    mt_emit_frame(setup_status, ret, slack, &in_buf, &out_buf);
    mt_exit(0);
}

/* ---- _start: the driver's own stack with an unmapped guard page below it ---- */

void mt_boot(const u32 *initial_sp) __attribute__((used));
void mt_boot(const u32 *initial_sp)
{
    u32 argc = initial_sp[0];
    const char *path;
    u32 top;
    if (argc < 2u) {
        mt_exit(MT_EXIT_REQUEST_IO);
    }
    path = (const char *)initial_sp[2];
    if (sc(SYS_MMAP2, (long)MT_STACK_BASE, (long)((MT_STACK_PAGES + 1u) * MT_PAGE), 3,
           MT_MAP_RW_FIXED, -1, 0) != (long)MT_STACK_BASE) {
        mt_exit(MT_EXIT_MAP);
    }
    if (sc(SYS_MPROTECT, (long)MT_STACK_BASE, (long)MT_PAGE, 0, 0, 0, 0) != 0) {
        mt_exit(MT_EXIT_SETUP);
    }
    top = MT_STACK_BASE + (MT_STACK_PAGES + 1u) * MT_PAGE;
    {
        register const char *arg0 __asm__("r0") = path;
        __asm__ volatile("mov sp, %[top]\n\t"
                         "b mt_main"
                         :
                         : "r"(arg0), [top] "r"(top)
                         : "memory");
    }
    __builtin_unreachable();
}

/* Thumb-2 cannot apply bic to sp, so the alignment goes through a low register. */
void _start(void) __attribute__((naked, noreturn));
void _start(void)
{
    __asm__ volatile("mov r0, sp\n\t"
                     "mov r1, r0\n\t"
                     "movs r2, #7\n\t"
                     "bics r1, r2\n\t"
                     "mov sp, r1\n\t"
                     "bl mt_boot");
}
