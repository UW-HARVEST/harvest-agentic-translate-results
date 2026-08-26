#include <stddef.h>

extern void *__libc_malloc(size_t size);

static size_t rejected_size;
static int reject_once;

void fail_malloc_of_size(size_t size) {
    rejected_size = size;
    reject_once = 1;
}

void *malloc(size_t size) {
    if (reject_once && size == rejected_size) {
        reject_once = 0;
        return NULL;
    }

    return __libc_malloc(size);
}
