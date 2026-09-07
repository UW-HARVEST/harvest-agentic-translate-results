#define _GNU_SOURCE

#include <stddef.h>
#include <stdatomic.h>

extern void *__libc_malloc(size_t size);

static _Atomic long failures_until_null = -1;
static _Atomic size_t failing_size = 0;

void failmalloc_after(long allocation_number) {
    atomic_store(&failing_size, 0);
    atomic_store(&failures_until_null, allocation_number);
}

void failmalloc_size(size_t size) {
    atomic_store(&failures_until_null, -1);
    atomic_store(&failing_size, size);
}

void *malloc(size_t size) {
    size_t selected_size = atomic_load(&failing_size);
    if (selected_size != 0 && selected_size == size) {
        atomic_store(&failing_size, 0);
        return NULL;
    }

    long remaining = atomic_load(&failures_until_null);
    if (remaining > 0 && atomic_fetch_sub(&failures_until_null, 1) == 1) {
        return NULL;
    }

    return __libc_malloc(size);
}
