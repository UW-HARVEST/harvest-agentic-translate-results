/*
 * Test-support LD_PRELOAD shim (NOT part of the library under test, and not part
 * of c_src/ — it lives under translation/tests/support/).
 *
 * It interposes the three allocation entry points that c_src/src/lib.c uses
 * (malloc, realloc, strdup) so that the error paths of ERRORS.md rows 1-6 can be
 * triggered deterministically and identically for the C and the Rust .so:
 *
 *   failalloc_arm(k)   start counting; make the k-th and every later allocation
 *                      fail (k <= 0 => never fail, count/trace only)
 *   failalloc_disarm() stop counting
 *   failalloc_count()  number of allocations seen while armed
 *   failalloc_trace()  "m:12,r:20,d:6" style log of op + requested size
 *
 * Counting is only active between arm and disarm, so unrelated allocations from
 * the harness or the Rust runtime cannot perturb the numbering.
 */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>

typedef void *(*malloc_fn)(size_t);
typedef void *(*realloc_fn)(void *, size_t);

static malloc_fn real_malloc;
static realloc_fn real_realloc;

static int armed;
static long counter;
static long fail_at = -1;

#define TRACE_MAX 4096
static char trace_op[TRACE_MAX];
static size_t trace_size[TRACE_MAX];
static long trace_n;

__attribute__((constructor)) static void failalloc_init(void)
{
    if (real_malloc == NULL) {
        real_malloc = (malloc_fn) dlsym(RTLD_NEXT, "malloc");
    }
    if (real_realloc == NULL) {
        real_realloc = (realloc_fn) dlsym(RTLD_NEXT, "realloc");
    }
}

/* Returns 1 when the caller must be told the allocation failed. */
static int note(char op, size_t size)
{
    if (!armed) {
        return 0;
    }
    counter++;
    if (trace_n < TRACE_MAX) {
        trace_op[trace_n] = op;
        trace_size[trace_n] = size;
        trace_n++;
    }
    return (fail_at > 0 && counter >= fail_at);
}

void failalloc_arm(long k)
{
    failalloc_init();
    counter = 0;
    trace_n = 0;
    fail_at = k;
    armed = 1;
}

void failalloc_disarm(void)
{
    armed = 0;
}

long failalloc_count(void)
{
    return counter;
}

size_t failalloc_trace(char *buf, size_t n)
{
    size_t used = 0;
    long i;

    if (n == 0) {
        return 0;
    }
    buf[0] = '\0';
    for (i = 0; i < trace_n; i++) {
        int w = snprintf(buf + used, n - used, "%s%c:%zu",
                         (i == 0) ? "" : ",", trace_op[i], trace_size[i]);
        if (w < 0 || (size_t) w >= n - used) {
            break;
        }
        used += (size_t) w;
    }
    return used;
}

void *malloc(size_t size)
{
    failalloc_init();
    if (note('m', size)) {
        return NULL;
    }
    return real_malloc(size);
}

void *realloc(void *p, size_t size)
{
    failalloc_init();
    if (note('r', size)) {
        return NULL;
    }
    return real_realloc(p, size);
}

char *strdup(const char *s)
{
    size_t n;
    char *p;

    failalloc_init();
    n = strlen(s) + 1;
    if (note('d', n)) {
        return NULL;
    }
    p = (char *) real_malloc(n);
    if (p == NULL) {
        return NULL;
    }
    memcpy(p, s, n);
    return p;
}
