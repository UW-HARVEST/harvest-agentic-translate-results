#include <stdatomic.h>
#include <stddef.h>

extern void *__libc_malloc(size_t size);

static _Atomic size_t fail_size = 0;

void fail_alloc_set_size(size_t size) {
    atomic_store_explicit(&fail_size, size, memory_order_relaxed);
}

void *malloc(size_t size) {
    if (size != 0 &&
        size == atomic_load_explicit(&fail_size, memory_order_relaxed)) {
        return NULL;
    }
    return __libc_malloc(size);
}
