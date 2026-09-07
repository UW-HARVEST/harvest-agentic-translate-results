#include <stdatomic.h>
#include <stddef.h>

extern void *__libc_malloc(size_t size);

static _Atomic int fail_mode = 0;
static _Atomic int malloc_index = 0;

void fail_malloc_set(int mode) {
    atomic_store(&malloc_index, 0);
    atomic_store(&fail_mode, mode);
}

void *malloc(size_t size) {
    int mode = atomic_load(&fail_mode);
    int index = atomic_fetch_add(&malloc_index, 1);

    if ((mode == 1 && index == 0) ||
        (mode == 2 && index == 1) ||
        (mode == 3 && index < 2)) {
        return NULL;
    }

    return __libc_malloc(size);
}
