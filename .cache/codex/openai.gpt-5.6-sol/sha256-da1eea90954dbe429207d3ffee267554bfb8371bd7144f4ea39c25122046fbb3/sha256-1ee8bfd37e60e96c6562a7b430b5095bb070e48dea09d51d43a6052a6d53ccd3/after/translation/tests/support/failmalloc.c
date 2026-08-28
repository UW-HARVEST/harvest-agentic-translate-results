#include <stddef.h>

extern void *__libc_malloc(size_t size);

static _Thread_local size_t failure_size;
static _Thread_local int failure_pending;

void fail_next_malloc_of_size(size_t size) {
    failure_size = size;
    failure_pending = 1;
}

void *malloc(size_t size) {
    if (failure_pending && size == failure_size) {
        failure_pending = 0;
        return NULL;
    }
    return __libc_malloc(size);
}
