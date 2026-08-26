/* Test-harness malloc interposer.  Loaded with LD_PRELOAD into the
 * `fault_child` helper; c_src/ is untouched.
 *
 * Two independent knobs, both armed by the helper *after* every shared object
 * has been loaded so that dlopen()'s own allocations are never disturbed:
 *
 *   fail_malloc_arm(size)   ERRORS.md rows 1, 2, 5, 9, 11 — makes malloc()
 *                           requests of exactly `size` bytes return NULL, the
 *                           only way to reach the `malloc(...) == NULL`
 *                           branches of c_src/src/lib.c.
 *
 *   fail_malloc_fill(byte)  CONFIGS.md row 44 — fills every freshly allocated
 *                           block with `byte`.  The C reads two buffers that it
 *                           only partially writes (the 64-byte block of
 *                           create_result_string past the NUL, and
 *                           Result.operation[32] past the strcpy), so on a
 *                           fresh zero-filled heap a missing NUL terminator or
 *                           a short copy is invisible.  A non-zero fill makes
 *                           those tails observable *and* deterministic.
 *
 * glibc's own MALLOC_PERTURB_ is not usable for this: the tcache fast path
 * bypasses alloc_perturb/free_perturb, so recycled chunks keep history-
 * dependent contents.  Doing the memset here covers every allocation path.
 *
 * __libc_malloc is used instead of dlsym(RTLD_NEXT, "malloc") so that no
 * allocation can happen inside the interposer itself (dlsym may allocate,
 * which would recurse).
 */
#include <stddef.h>
#include <string.h>

extern void *__libc_malloc(size_t);

/* 0 == disarmed */
static volatile unsigned long g_fail_size = 0;
static volatile int g_fill_armed = 0;
static volatile unsigned char g_fill_byte = 0;

void fail_malloc_arm(unsigned long size) { g_fail_size = size; }

unsigned long fail_malloc_armed_size(void) { return g_fail_size; }

/* byte == 0 disarms the fill */
void fail_malloc_fill(unsigned long byte) {
    g_fill_byte = (unsigned char)byte;
    g_fill_armed = (byte != 0);
}

/* --- allocation-size log -------------------------------------------------
 * Records the exact byte counts requested while enabled.  Some divergences are
 * invisible in the *result* but visible in the *request*: e.g. computing
 * `count * sizeof(int)` with a zero-extended instead of a sign-extended
 * `count` asks for 17179869180 bytes instead of 18446744073709551612 — both
 * fail on a normal host, so only the logged size tells them apart.
 */
#define LOG_MAX 512
static volatile int g_log_on = 0;
static volatile int g_log_n = 0;
static unsigned long g_log[LOG_MAX];

void fail_malloc_log_start(void) {
    g_log_n = 0;
    g_log_on = 1;
}
void fail_malloc_log_stop(void) { g_log_on = 0; }
int fail_malloc_log_count(void) { return g_log_n; }
unsigned long fail_malloc_log_get(int i) {
    return (i >= 0 && i < LOG_MAX && i < g_log_n) ? g_log[i] : 0UL;
}

void *malloc(size_t n) {
    unsigned long want = g_fail_size;
    void *p;

    if (g_log_on && g_log_n < LOG_MAX) {
        g_log[g_log_n++] = (unsigned long)n;
    }
    if (want != 0 && (unsigned long)n == want) {
        return (void *)0;
    }
    p = __libc_malloc(n);
    if (p != (void *)0 && g_fill_armed) {
        memset(p, g_fill_byte, n);
    }
    return p;
}
