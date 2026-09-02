/* LD_PRELOAD shim used by ERRORS.md row 18 and by the allocator-parity test.
 *
 * `checkshift` is the library's only allocation site
 * (`malloc(sizeof(ComputeState))` == malloc(12)) and its failure branch is
 * otherwise unreachable. This interposer can (a) make malloc(12) fail on demand
 * and (b) count malloc(12)/free calls.
 *
 * The triggers are global flags rather than environment variables so the window
 * can be opened for exactly one call: an env-var trigger would also affect
 * process startup, dlopen and the Rust runtime.
 *
 * Not part of the library under test; lives under translation/tests/aux/.
 */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <stddef.h>

/* --- row 18: injected allocation failure --- */
int checkshift_fail_malloc_12 = 0;
int checkshift_fail_malloc_12_hits = 0;

/* --- allocator-call parity --- */
int checkshift_count_on = 0;
int checkshift_malloc12_count = 0;
int checkshift_free_count = 0;

static void *(*real_malloc)(size_t) = NULL;
static void (*real_free)(void *) = NULL;

/* Resolve both up front so that a dlsym-internal allocation cannot recurse into
 * a half-initialised interposer. */
__attribute__((constructor)) static void init_real(void) {
    if (real_malloc == NULL) {
        real_malloc = (void *(*)(size_t))dlsym(RTLD_NEXT, "malloc");
    }
    if (real_free == NULL) {
        real_free = (void (*)(void *))dlsym(RTLD_NEXT, "free");
    }
}

void *malloc(size_t n) {
    if (real_malloc == NULL) {
        init_real();
        if (real_malloc == NULL) {
            return NULL;
        }
    }
    if (checkshift_fail_malloc_12 && n == 12) {
        checkshift_fail_malloc_12_hits++;
        return NULL;
    }
    if (checkshift_count_on && n == 12) {
        checkshift_malloc12_count++;
    }
    return real_malloc(n);
}

void free(void *p) {
    if (real_free == NULL) {
        init_real();
        if (real_free == NULL) {
            return;
        }
    }
    if (checkshift_count_on && p != NULL) {
        checkshift_free_count++;
    }
    real_free(p);
}
