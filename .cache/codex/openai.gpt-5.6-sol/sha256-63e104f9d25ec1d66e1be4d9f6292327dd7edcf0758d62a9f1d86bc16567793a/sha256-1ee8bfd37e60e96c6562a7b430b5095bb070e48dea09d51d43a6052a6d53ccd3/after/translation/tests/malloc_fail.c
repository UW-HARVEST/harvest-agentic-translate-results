#include <stdatomic.h>
#include <stddef.h>

extern void *__libc_malloc(size_t size);

static _Atomic size_t failure_size;
static _Atomic int failure_armed;

void fail_next_malloc_of_size(size_t size) {
    atomic_store(&failure_size, size);
    atomic_store(&failure_armed, 1);
}

void *malloc(size_t size) {
    if (size == atomic_load(&failure_size) &&
        atomic_exchange(&failure_armed, 0)) {
        return NULL;
    }

    return __libc_malloc(size);
}
