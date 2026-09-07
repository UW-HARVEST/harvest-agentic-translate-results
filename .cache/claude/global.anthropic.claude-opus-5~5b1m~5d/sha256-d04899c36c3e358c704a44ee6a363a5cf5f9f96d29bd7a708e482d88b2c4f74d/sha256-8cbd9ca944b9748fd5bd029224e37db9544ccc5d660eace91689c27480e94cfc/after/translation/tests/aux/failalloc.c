/*
 * LD_PRELOAD allocator interposer for ERRORS.md rows E3 / E4 / E5.
 *
 * Failing every allocation would break the harness itself (dlopen, printf, ...),
 * so failure is *armed* only for the window around the call under test:
 * harness.c calls failalloc_arm() immediately before, and failalloc_disarm()
 * immediately after, invoking w_utf8_filter.
 *
 * Configuration (environment):
 *   FAILALLOC_MODE = malloc | realloc | strdup   which function fails
 *   FAILALLOC_SKIP = N                           let the first N matching calls
 *                                                succeed, fail call N+1 (default 0)
 */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <errno.h>
#include <stdlib.h>
#include <string.h>

static void *(*real_malloc)(size_t);
static void *(*real_realloc)(void *, size_t);
static char *(*real_strdup)(const char *);

static int armed = 0;
static int mode_malloc = 0, mode_realloc = 0, mode_strdup = 0;
static long skip = 0;
static long seen = 0;
static int initialised = 0;

static void init(void) {
    if (initialised) return;
    initialised = 1;
    real_malloc = (void *(*)(size_t))dlsym(RTLD_NEXT, "malloc");
    real_realloc = (void *(*)(void *, size_t))dlsym(RTLD_NEXT, "realloc");
    real_strdup = (char *(*)(const char *))dlsym(RTLD_NEXT, "strdup");
    const char *m = getenv("FAILALLOC_MODE");
    if (m) {
        mode_malloc = (strcmp(m, "malloc") == 0);
        mode_realloc = (strcmp(m, "realloc") == 0);
        mode_strdup = (strcmp(m, "strdup") == 0);
    }
    const char *s = getenv("FAILALLOC_SKIP");
    skip = s ? strtol(s, NULL, 10) : 0;
}

void failalloc_arm(void) {
    init();
    seen = 0;
    armed = 1;
}

void failalloc_disarm(void) { armed = 0; }

/* Should this matching call fail? */
static int should_fail(int mode_on) {
    if (!armed || !mode_on) return 0;
    if (seen++ < skip) return 0;
    return 1;
}

void *malloc(size_t n) {
    init();
    if (should_fail(mode_malloc)) {
        errno = ENOMEM;
        return NULL;
    }
    return real_malloc(n);
}

void *realloc(void *p, size_t n) {
    init();
    if (should_fail(mode_realloc)) {
        errno = ENOMEM;
        return NULL;
    }
    return real_realloc(p, n);
}

char *strdup(const char *s) {
    init();
    if (should_fail(mode_strdup)) {
        errno = ENOMEM;
        return NULL;
    }
    return real_strdup(s);
}
