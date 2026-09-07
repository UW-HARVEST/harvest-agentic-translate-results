#include <stdatomic.h>
#include <stddef.h>

extern void *__libc_malloc(size_t size);

static _Atomic size_t fail_size;

void fail_malloc_once(size_t size) {
    atomic_store(&fail_size, size);
}

void *malloc(size_t size) {
    size_t expected = atomic_load(&fail_size);
    if (expected != 0 && size == expected &&
        atomic_compare_exchange_strong(&fail_size, &expected, 0)) {
        return NULL;
    }
    return __libc_malloc(size);
}

