/* LD_PRELOAD probe used by translation/tests/phase_e_alloc.rs.
 *
 * Interposes calloc() so a test can observe the EXACT (nmemb, size) request the
 * library under test makes, which is the only fully deterministic way to compare
 * the two implementations' allocation sizes (malloc_usable_size reports the
 * reused chunk's capacity, not the request).
 *
 * Recording is off until calloc_probe_arm() is called, so unrelated allocations
 * made by the test harness are not captured.
 *
 * This file lives in the Rust crate, not in c_src/ — c_src/ is untouched.
 */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <stddef.h>
#include <string.h>
#include <unistd.h>

static void *(*real_calloc)(size_t, size_t);
static int resolving;
static int armed;
static size_t rec_count, rec_nmemb, rec_size;

/* Tiny fallback arena in case the dynamic loader itself calls calloc while we
 * are resolving the real symbol. Freeing such a block would be wrong, but glibc
 * does not hand loader-internal allocations to the application. */
static char arena[16384];
static size_t arena_used;

/* Allocation-free unsigned -> decimal. Written to fd 2 the moment a request is
 * seen, so the record survives even when the library exits() before returning
 * (the calloc-failure path). */
static char *emit_uint(char *p, size_t v)
{
    char tmp[24];
    int n = 0;
    if (v == 0) tmp[n++] = '0';
    while (v) { tmp[n++] = (char)('0' + (v % 10)); v /= 10; }
    while (n) *p++ = tmp[--n];
    return p;
}

static void emit_record(size_t nmemb, size_t size)
{
    char buf[96];
    char *p = buf;
    memcpy(p, "PROBEC nmemb=", 13); p += 13;
    p = emit_uint(p, nmemb);
    memcpy(p, " size=", 6); p += 6;
    p = emit_uint(p, size);
    *p++ = '\n';
    ssize_t ignored = write(2, buf, (size_t)(p - buf));
    (void)ignored;
}

void *calloc(size_t nmemb, size_t size)
{
    if (armed) {
        rec_count++;
        rec_nmemb = nmemb;
        rec_size = size;
        emit_record(nmemb, size);
    }
    if (!real_calloc) {
        if (resolving) {
            size_t total = nmemb * size;
            total = (total + 15u) & ~(size_t)15u;
            if (total == 0) total = 16;
            if (arena_used + total > sizeof arena) return NULL;
            void *p = arena + arena_used;
            arena_used += total;
            memset(p, 0, total);
            return p;
        }
        resolving = 1;
        real_calloc = (void *(*)(size_t, size_t))dlsym(RTLD_NEXT, "calloc");
        resolving = 0;
        if (!real_calloc) return NULL;
    }
    return real_calloc(nmemb, size);
}

void calloc_probe_arm(void)
{
    rec_count = 0;
    rec_nmemb = 0;
    rec_size = 0;
    armed = 1;
}

void calloc_probe_disarm(void) { armed = 0; }

size_t calloc_probe_count(void) { return rec_count; }
size_t calloc_probe_nmemb(void) { return rec_nmemb; }
size_t calloc_probe_size(void) { return rec_size; }

/* Presence marker so the test can verify the preload actually took effect. */
int calloc_probe_present(void) { return 1; }
