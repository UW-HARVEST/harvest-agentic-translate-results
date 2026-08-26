#define _GNU_SOURCE

#include <dlfcn.h>
#include <stdatomic.h>
#include <stddef.h>

static _Atomic int int_allocations_until_failure = 0;
static void *(*real_malloc)(size_t) = NULL;

void fail_nth_int_malloc(int allocation) {
    atomic_store(&int_allocations_until_failure, allocation);
}

void *malloc(size_t size) {
    if (real_malloc == NULL) {
        real_malloc = dlsym(RTLD_NEXT, "malloc");
    }

    if (size == sizeof(int)) {
        int remaining = atomic_load(&int_allocations_until_failure);
        if (remaining > 0 &&
            atomic_fetch_sub(&int_allocations_until_failure, 1) == 1) {
            return NULL;
        }
    }

    return real_malloc(size);
}
