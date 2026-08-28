/*
 * LD_PRELOAD allocator interposer used by the Phase B/C differential tests.
 *
 * It gives the tests two capabilities that cannot be obtained in-process:
 *
 *   1. deterministic OOM injection — `failalloc_arm(m, r, s)` makes the m-th
 *      malloc / r-th realloc / s-th strdup *after arming* return NULL, so the
 *      `if (copy == NULL) return NULL;` branches of `w_utf8_filter` become
 *      reachable in a test;
 *
 *   2. an exact allocation trace — `failalloc_trace_begin()` records every
 *      malloc/realloc/strdup request (kind + requested size) in a static
 *      array, so the C and the Rust implementation can be compared on the
 *      *sequence of allocation requests*, not just on the bytes they produce.
 *      (`malloc_usable_size` cannot be used for that: it depends on which
 *      chunk the allocator happens to recycle.)
 *
 * Interposition forwards to the glibc-internal entry points instead of using
 * dlsym(RTLD_NEXT, ...) so that nothing here can allocate re-entrantly.
 * Recording never allocates either (fixed-size static arrays).
 *
 * This file is test scaffolding; it is NOT part of the translated library.
 */

#define _GNU_SOURCE
#include <stddef.h>
#include <string.h>

extern void *__libc_malloc(size_t);
extern void *__libc_realloc(void *, size_t);

/* countdown counters: N > 0 means "fail the N-th call from now on" */
static int fail_malloc;
static int fail_realloc;
static int fail_strdup;
/* Only requests of at least this many bytes are eligible to be counted and
 * failed. Without it an unrelated allocation from the Rust runtime (which sits
 * between "arm" and the library call) could swallow the injected failure and
 * abort the process instead — the injection has to be surgical. */
static size_t min_size;
/* how many injected failures actually fired (the test asserts this is 1) */
static int fired;

#define TRACE_MAX 262144
static int tracing;
static char trace_kind[TRACE_MAX];
static size_t trace_arg[TRACE_MAX];
static size_t trace_n;
static size_t trace_overflow;

void failalloc_arm(int m, int r, int s) {
    fired = 0;
    fail_malloc = m;
    fail_realloc = r;
    fail_strdup = s;
}

/* Restrict injection to requests of >= n bytes. Call BEFORE failalloc_arm. */
void failalloc_set_min_size(size_t n) { min_size = n; }

int failalloc_fired(void) { return fired; }

void failalloc_disarm(void) {
    fail_malloc = fail_realloc = fail_strdup = 0;
    min_size = 0;
}

void failalloc_trace_begin(void) {
    trace_n = 0;
    trace_overflow = 0;
    tracing = 1;
}

void failalloc_trace_end(void) { tracing = 0; }

size_t failalloc_trace_count(void) { return trace_n; }
size_t failalloc_trace_overflow(void) { return trace_overflow; }

int failalloc_trace_kind(size_t i) {
    return i < trace_n ? (int)(unsigned char)trace_kind[i] : 0;
}

size_t failalloc_trace_arg(size_t i) { return i < trace_n ? trace_arg[i] : 0; }

static void rec(char k, size_t n) {
    if (!tracing) {
        return;
    }
    if (trace_n < TRACE_MAX) {
        trace_kind[trace_n] = k;
        trace_arg[trace_n] = n;
        trace_n++;
    } else {
        trace_overflow++;
    }
}

void *malloc(size_t n) {
    rec('M', n);
    if (fail_malloc && n >= min_size && --fail_malloc == 0) {
        fired++;
        return NULL;
    }
    return __libc_malloc(n);
}

void *realloc(void *p, size_t n) {
    rec('R', n);
    if (fail_realloc && n >= min_size && --fail_realloc == 0) {
        fired++;
        return NULL;
    }
    return __libc_realloc(p, n);
}

char *strdup(const char *s) {
    size_t n = strlen(s);
    rec('S', n);
    if (fail_strdup && n >= min_size && --fail_strdup == 0) {
        fired++;
        return NULL;
    }
    char *p = (char *)__libc_malloc(n + 1);
    if (p != NULL) {
        memcpy(p, s, n + 1);
    }
    return p;
}
