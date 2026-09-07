/*
 * Test-only allocator interposer used by tests/robustness.rs to reach the
 * `return NULL` allocation-failure branches of searchAndReplace
 * (ERRORS.md rows 2-6).
 *
 * Behaviour: malloc()/realloc() requests whose size equals exactly
 * $FAILMALLOC_SIZE bytes return NULL (with ENOMEM); every other request is
 * forwarded to glibc's real allocator. Matching on an exact size (rather than
 * a threshold or a call counter) keeps the injected failure identical in the C
 * child and in the Rust child, whose runtimes allocate different amounts for
 * their own bookkeeping.
 *
 * glibc's __libc_malloc/__libc_realloc are used instead of dlsym(RTLD_NEXT)
 * so that no allocation happens while resolving the real allocator.
 *
 * This file is NOT part of the library under test and is not linked into it.
 */
#define _GNU_SOURCE
#include <stdlib.h>
#include <errno.h>

extern void *__libc_malloc(size_t);
extern void *__libc_realloc(void *, size_t);

static size_t target(void)
{
    static size_t cached = 0;
    static int initialised = 0;
    if (!initialised) {
        const char *s = getenv("FAILMALLOC_SIZE");
        cached = (s != NULL) ? (size_t) strtoull(s, NULL, 10) : 0;
        initialised = 1;
    }
    return cached;
}

void *malloc(size_t n)
{
    if (n != 0 && n == target()) {
        errno = ENOMEM;
        return NULL;
    }
    return __libc_malloc(n);
}

void *realloc(void *p, size_t n)
{
    if (n != 0 && n == target()) {
        errno = ENOMEM;
        return NULL;
    }
    return __libc_realloc(p, n);
}
