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
 * a length and the bytes. Result frame (little-endian), written once with a raw
 * write after the call: "MTRS", version 1, status, raw return value, bytes the
 * C code wrote through libc (discarded), slack check result, region count, then
 * per region an id, a length and the bytes, then "MTRE".
 */
#include <errno.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>

typedef uint8_t u8;
typedef uint32_t u32;

#define MT_RESERVED_VA @MT_RESERVED_VA@u
#define MT_HEAP_ARENA @MT_HEAP_ARENA@u
#define MT_SENTINEL @MT_SENTINEL@u
#define MT_SLACK_FILL @MT_SLACK_FILL@u
#define MT_MAX_INPUT @MT_MAX_INPUT@u

#define MT_PAGE 4096u
#define MT_IN_BASE 0x04000000u
#define MT_OUT_BASE 0x05000000u
#define MT_STACK_BASE 0x07000000u
#define MT_STACK_PAGES 64u
#define MT_MAX_FIELDS 8u
#define MT_REQ_MAX (MT_MAX_INPUT + 4096u)

#define MT_EXIT_CALLED 95
#define MT_EXIT_REQUEST_IO 96
#define MT_EXIT_REQUEST_BAD 97
#define MT_EXIT_MAP 98
#define MT_EXIT_SETUP 99

/* Every mapping must lie below the reserved address space (qemu-arm -R). */
_Static_assert(MT_STACK_BASE + (MT_STACK_PAGES + 1u) * MT_PAGE <= MT_RESERVED_VA,
               "the stack mapping must lie below the reserved address space");
_Static_assert(MT_OUT_BASE + (MT_MAX_INPUT / MT_PAGE + 3u) * MT_PAGE <= MT_STACK_BASE,
               "the output mapping must end below the stack mapping");

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

/* ---- the cases (generated by the harness) ---- */

@MT_CASES@

/* ---- the result frame ---- */

static u8 mt_frame_buf[MT_MAX_INPUT + 256u];

static void mt_emit_frame(u32 status, u32 ret, u32 slack, const u8 *out, u32 out_len)
{
    u8 *p = mt_frame_buf;
    u32 total;
    u32 off = 0;
    p[0] = 'M';
    p[1] = 'T';
    p[2] = 'R';
    p[3] = 'S';
    mt_put32(p + 4, 1u);
    mt_put32(p + 8, status);
    mt_put32(p + 12, ret);
    mt_put32(p + 16, mt_discarded);
    mt_put32(p + 20, slack);
    mt_put32(p + 24, 1u); /* one region: the whole output buffer */
    mt_put32(p + 28, 1u);
    mt_put32(p + 32, out_len);
    memcpy(p + 36, out, out_len);
    p[36 + out_len] = 'M';
    p[37 + out_len] = 'T';
    p[38 + out_len] = 'R';
    p[39 + out_len] = 'E';
    total = 40u + out_len;
    while (off < total) {
        long w = sc(SYS_WRITE, 1, (long)(mt_frame_buf + off), (long)(total - off), 0, 0, 0);
        if (w <= 0) {
            mt_exit(MT_EXIT_SETUP);
        }
        off += (u32)w;
    }
}

/* ---- the run: on the driver's own stack ---- */

void mt_main(const char *path) __attribute__((noreturn, used));
void mt_main(const char *path)
{
    struct guarded in_buf;
    struct guarded out_buf;
    u32 selector;
    u32 out_cap;
    u32 ret;
    u32 slack;
    mt_install_handlers();
    mt_load_request(path);
    __libc_init_array();
    /* Self-test request: case selector, input bytes, output capacity. */
    if (mt_field_count != 3u || mt_fields[0].len != 4u || mt_fields[2].len != 4u) {
        mt_exit(MT_EXIT_REQUEST_BAD);
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
    ret = mt_call(selector, in_buf.buf, mt_fields[1].len, out_buf.buf, out_cap);
    fflush(NULL);
    slack = mt_slack_changed(&in_buf) | mt_slack_changed(&out_buf);
    mt_emit_frame(0u, ret, slack, out_buf.buf, out_cap);
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
