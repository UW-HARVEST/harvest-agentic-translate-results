/*
 * Test-only LD_PRELOAD interposer for the Phase C error-path tests.
 *
 * This file is a TEST FIXTURE for translation/tests/error_paths.rs. It is NOT
 * part of the library under test and nothing in c_src/ is touched.
 *
 * Both libdriver.so builds import calloc/malloc/free/strlen dynamically
 * (`U calloc@GLIBC_2.2.5`, ...), so preloading this object lets the tests drive
 * the C and the Rust implementation through their otherwise unreachable
 * allocation-failure branches in EXACTLY the same way:
 *
 *   ERRORS.md row 3: calloc(sizeof(char), l + 13) == NULL
 *   ERRORS.md row 4: malloc(l) == NULL      (dest must be freed first)
 *   ERRORS.md row 5: strlen(src) + 1 overflowing int
 *
 * Failures are keyed on the EXACT requested byte count so that the test
 * harness's own allocations are never disturbed. Everything else forwards to
 * glibc via the public __libc_* entry points (no dlsym => no re-entrancy risk).
 */

#include <stddef.h>

extern void *__libc_malloc(size_t);
extern void *__libc_calloc(size_t, size_t);
extern void __libc_free(void *);

/* Armed configuration. 0 == disabled. */
static size_t fail_calloc_total;
static size_t fail_malloc_size;

/* Watch a specific calloc size and remember whether that exact pointer was
 * later handed to free(). Used to prove `free(dest)` on the malloc-failure
 * path (i.e. that neither implementation leaks dest). */
static size_t watch_calloc_total;
static void *watch_ptr;
static int watch_seen;
static int watch_freed;

/* Call counters for the armed sizes. */
static unsigned long calloc_fail_hits;
static unsigned long malloc_fail_hits;

/* strlen override: 0 = off, 1 = return INT_MAX, 2 = return real + 2^32. */
static int strlen_mode;
static const char *strlen_marker;

/* Allocation trace: exact sizes and call counts, so that the C and the Rust
 * implementation can be compared on the allocator traffic they generate and not
 * just on the bytes they return. */
static size_t last_calloc_total;
static size_t last_malloc_size;
static unsigned long calloc_calls;
static unsigned long malloc_calls;
static unsigned long free_calls;

void shim_trace_reset(void)
{
    last_calloc_total = 0;
    last_malloc_size = 0;
    calloc_calls = 0;
    malloc_calls = 0;
    free_calls = 0;
}

size_t shim_last_calloc_total(void) { return last_calloc_total; }
size_t shim_last_malloc_size(void) { return last_malloc_size; }
unsigned long shim_calloc_calls(void) { return calloc_calls; }
unsigned long shim_malloc_calls(void) { return malloc_calls; }
unsigned long shim_free_calls(void) { return free_calls; }

void shim_arm(size_t calloc_total, size_t malloc_size, size_t watch_total)
{
    fail_calloc_total = calloc_total;
    fail_malloc_size = malloc_size;
    watch_calloc_total = watch_total;
    watch_ptr = NULL;
    watch_seen = 0;
    watch_freed = 0;
    calloc_fail_hits = 0;
    malloc_fail_hits = 0;
}

void shim_disarm(void)
{
    shim_arm(0, 0, 0);
    strlen_mode = 0;
    strlen_marker = NULL;
}

int shim_watch_seen(void) { return watch_seen; }
int shim_watch_freed(void) { return watch_freed; }
unsigned long shim_calloc_fail_hits(void) { return calloc_fail_hits; }
unsigned long shim_malloc_fail_hits(void) { return malloc_fail_hits; }

void shim_strlen_set(int mode, const char *marker)
{
    strlen_mode = mode;
    strlen_marker = marker;
}

/* --- interposed libc entry points ------------------------------------- */

void *calloc(size_t nmemb, size_t size)
{
    size_t total = nmemb * size;

    last_calloc_total = total;
    calloc_calls++;

    if (fail_calloc_total != 0 && total == fail_calloc_total) {
        calloc_fail_hits++;
        return NULL;
    }

    void *p = __libc_calloc(nmemb, size);

    if (watch_calloc_total != 0 && total == watch_calloc_total && p != NULL) {
        watch_ptr = p;
        watch_seen = 1;
        watch_freed = 0;
    }
    return p;
}

void *malloc(size_t size)
{
    last_malloc_size = size;
    malloc_calls++;

    if (fail_malloc_size != 0 && size == fail_malloc_size) {
        malloc_fail_hits++;
        return NULL;
    }
    return __libc_malloc(size);
}

void free(void *p)
{
    free_calls++;

    if (p != NULL && p == watch_ptr) {
        watch_freed = 1;
        watch_ptr = NULL;
    }
    __libc_free(p);
}

/* Self-contained (no libc call => no recursion) so that the marker pointer can
 * report an absurd length without needing a multi-gigabyte allocation. */
size_t strlen(const char *s)
{
    const char *p = s;

    while (*p) {
        p++;
    }

    size_t n = (size_t)(p - s);

    if (strlen_mode != 0 && s == strlen_marker) {
        if (strlen_mode == 1) {
            return 0x7fffffffUL; /* INT_MAX: l = INT_MAX+1 wraps to INT_MIN */
        }
        if (strlen_mode == 2) {
            return n + 0x100000000UL; /* 2^32 + real: exercises int truncation */
        }
    }
    return n;
}
