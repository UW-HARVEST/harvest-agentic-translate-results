#include <stddef.h>

extern void *__libc_malloc(size_t size);

static _Thread_local size_t fail_size;
static _Thread_local int fail_armed;

__attribute__((visibility("default")))
void fail_malloc_once(size_t size) {
    fail_size = size;
    fail_armed = 1;
}

void *malloc(size_t size) {
    if (fail_armed && size == fail_size) {
        fail_armed = 0;
        return NULL;
    }
    return __libc_malloc(size);
}
