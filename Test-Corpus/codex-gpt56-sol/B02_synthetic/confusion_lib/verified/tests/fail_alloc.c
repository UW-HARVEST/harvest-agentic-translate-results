#include <stddef.h>

extern void *__libc_malloc(size_t size);

static size_t fail_size;
static int armed;

void fail_next_malloc_of_size(size_t size) {
    fail_size = size;
    armed = 1;
}

void *malloc(size_t size) {
    if (armed && size == fail_size) {
        armed = 0;
        return NULL;
    }
    return __libc_malloc(size);
}
