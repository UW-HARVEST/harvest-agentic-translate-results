#include <stddef.h>

extern void *__libc_malloc(size_t);

static size_t failed_size;
static int armed;

void arm_failure(size_t size) {
    failed_size = size;
    armed = 1;
}

void *malloc(size_t size) {
    if (armed && size == failed_size) {
        armed = 0;
        return NULL;
    }
    return __libc_malloc(size);
}
