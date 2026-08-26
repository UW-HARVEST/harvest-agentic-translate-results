#include <stddef.h>

extern void *__libc_malloc(size_t size);
extern void *__libc_calloc(size_t count, size_t size);

static size_t malloc_target_size;
static unsigned int malloc_countdown;
static unsigned int calloc_countdown;

void fail_malloc_on_nth(size_t size, unsigned int nth) {
    malloc_target_size = size;
    malloc_countdown = nth;
}

void fail_calloc_on_nth(unsigned int nth) {
    calloc_countdown = nth;
}

void *malloc(size_t size) {
    if (malloc_countdown != 0 && size == malloc_target_size) {
        malloc_countdown--;
        if (malloc_countdown == 0) {
            return NULL;
        }
    }
    return __libc_malloc(size);
}

void *calloc(size_t count, size_t size) {
    if (calloc_countdown != 0) {
        calloc_countdown--;
        if (calloc_countdown == 0) {
            return NULL;
        }
    }
    return __libc_calloc(count, size);
}
