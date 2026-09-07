#include <stddef.h>

extern void *__libc_malloc(size_t);
extern void *__libc_realloc(void *, size_t);
extern void __libc_free(void *);

static _Thread_local long malloc_countdown = -1;
static _Thread_local long realloc_countdown = -1;

void fail_malloc_after(long successful_calls) {
    malloc_countdown = successful_calls;
}

void fail_realloc_after(long successful_calls) {
    realloc_countdown = successful_calls;
}

void disable_alloc_failures(void) {
    malloc_countdown = -1;
    realloc_countdown = -1;
}

void *malloc(size_t size) {
    if (malloc_countdown >= 0) {
        if (malloc_countdown == 0) {
            malloc_countdown = -1;
            return NULL;
        }
        --malloc_countdown;
    }
    return __libc_malloc(size);
}

void *realloc(void *pointer, size_t size) {
    if (realloc_countdown >= 0) {
        if (realloc_countdown == 0) {
            realloc_countdown = -1;
            return NULL;
        }
        --realloc_countdown;
    }
    return __libc_realloc(pointer, size);
}

void free(void *pointer) {
    __libc_free(pointer);
}

