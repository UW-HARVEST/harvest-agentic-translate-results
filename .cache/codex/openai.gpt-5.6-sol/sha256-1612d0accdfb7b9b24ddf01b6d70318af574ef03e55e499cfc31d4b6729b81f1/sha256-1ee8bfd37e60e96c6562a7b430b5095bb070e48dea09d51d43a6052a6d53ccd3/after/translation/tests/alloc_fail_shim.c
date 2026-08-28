#include <stddef.h>

extern void *__libc_malloc(size_t size);
extern void *__libc_realloc(void *pointer, size_t size);
extern void __libc_free(void *pointer);

static _Thread_local int malloc_countdown = -1;
static _Thread_local int fail_realloc = 0;

void fail_malloc_after(int successful_calls) {
    malloc_countdown = successful_calls;
}

void fail_next_realloc(void) {
    fail_realloc = 1;
}

void *malloc(size_t size) {
    if (malloc_countdown == 0) {
        malloc_countdown = -1;
        return NULL;
    }
    if (malloc_countdown > 0) {
        --malloc_countdown;
    }
    return __libc_malloc(size);
}

void *realloc(void *pointer, size_t size) {
    if (fail_realloc) {
        fail_realloc = 0;
        return NULL;
    }
    return __libc_realloc(pointer, size);
}

void free(void *pointer) {
    __libc_free(pointer);
}
