/* Allocator interposer used by the differential suite (Phase C strengthening).
 *
 * Neither implementation's malloc REQUEST SIZE is observable from the return
 * value: a non-NULL result requires numLines <= bufferSize, so on every
 * reachable success path `numLines * sizeof(const char**)` is small and cannot
 * wrap. Likewise, whether the error path calls free() is invisible to a caller.
 * Interposing malloc/free makes both observable, so the differential tests can
 * assert that the C and the Rust request the SAME size and free the SAME number
 * of times.
 *
 * This file is NOT part of c_src/ and is built into target/ by
 * build_interpose.sh. It is test scaffolding only.
 */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <stddef.h>
#include <stdint.h>

static void *(*real_malloc)(size_t);
static void (*real_free)(void *);

/* Counters. Deliberately not thread-safe: the tests that use them run
 * single-threaded (`--test-threads=1`). */
static uint64_t g_malloc_calls;
static uint64_t g_free_calls;
static size_t g_last_malloc_size;
static uint64_t g_enabled;

static void init(void)
{
    if (!real_malloc) {
        real_malloc = (void *(*)(size_t))dlsym(RTLD_NEXT, "malloc");
        real_free = (void (*)(void *))dlsym(RTLD_NEXT, "free");
    }
}

void *malloc(size_t n)
{
    init();
    if (g_enabled) {
        g_malloc_calls++;
        g_last_malloc_size = n;
    }
    return real_malloc(n);
}

void free(void *p)
{
    init();
    if (g_enabled && p) {
        g_free_calls++;
    }
    real_free(p);
}

/* --- control surface, called from the Rust tests via dlsym ---------------- */

void mt_reset(void)
{
    g_malloc_calls = 0;
    g_free_calls = 0;
    g_last_malloc_size = 0;
}

void mt_enable(void) { g_enabled = 1; }
void mt_disable(void) { g_enabled = 0; }

uint64_t mt_malloc_calls(void) { return g_malloc_calls; }
uint64_t mt_free_calls(void) { return g_free_calls; }
size_t mt_last_malloc_size(void) { return g_last_malloc_size; }

/* Presence probe so the test can tell whether it was preloaded. */
int mt_present(void) { return 1; }
