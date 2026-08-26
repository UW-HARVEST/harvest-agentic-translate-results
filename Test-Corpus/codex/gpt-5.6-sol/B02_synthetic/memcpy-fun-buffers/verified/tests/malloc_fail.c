#include <stddef.h>
#include <stdatomic.h>

extern void *__libc_malloc(size_t size);

static _Atomic size_t fail_size;

void set_fail_malloc_size(size_t size) {
    atomic_store(&fail_size, size);
}

void *malloc(size_t size) {
    size_t expected = atomic_load(&fail_size);
    if (expected == size &&
        atomic_compare_exchange_strong(&fail_size, &expected, 0)) {
        return NULL;
    }
    return __libc_malloc(size);
}
