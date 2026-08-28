#define _GNU_SOURCE

#include <stdatomic.h>
#include <stddef.h>

extern void *__libc_malloc(size_t size);

static _Atomic size_t rejected_size = 0;

void fail_one_malloc_of_size(size_t size) {
    atomic_store_explicit(&rejected_size, size, memory_order_release);
}

void *malloc(size_t size) {
    size_t expected = size;
    if (size != 0 &&
        atomic_compare_exchange_strong_explicit(
            &rejected_size,
            &expected,
            0,
            memory_order_acq_rel,
            memory_order_acquire)) {
        return NULL;
    }
    return __libc_malloc(size);
}
