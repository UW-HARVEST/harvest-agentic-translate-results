#include <stdatomic.h>
#include <stddef.h>

extern void *__libc_malloc(size_t size);

static _Atomic size_t armed_size;
static _Atomic unsigned int armed_count;

void fail_malloc_arm(size_t size)
{
    atomic_store_explicit(&armed_size, size, memory_order_relaxed);
    atomic_store_explicit(&armed_count, 1, memory_order_release);
}

void *malloc(size_t size)
{
    if (atomic_load_explicit(&armed_count, memory_order_acquire) != 0 &&
        atomic_load_explicit(&armed_size, memory_order_relaxed) == size) {
        atomic_store_explicit(&armed_count, 0, memory_order_release);
        return NULL;
    }

    return __libc_malloc(size);
}
