#include <stddef.h>

extern void *__libc_calloc(size_t count, size_t size);
extern void *__libc_malloc(size_t size);

static _Thread_local int fail_mode;

void set_fail_mode(int mode)
{
    fail_mode = mode;
}

void *calloc(size_t count, size_t size)
{
    if (fail_mode == 1) {
        fail_mode = 0;
        return NULL;
    }
    return __libc_calloc(count, size);
}

void *malloc(size_t size)
{
    if (fail_mode == 2) {
        fail_mode = 0;
        return NULL;
    }
    return __libc_malloc(size);
}
