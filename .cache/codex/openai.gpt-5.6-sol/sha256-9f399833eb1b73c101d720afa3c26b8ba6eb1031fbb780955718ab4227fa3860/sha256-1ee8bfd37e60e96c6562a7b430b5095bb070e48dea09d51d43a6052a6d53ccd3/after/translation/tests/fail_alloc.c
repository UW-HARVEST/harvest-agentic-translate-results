#include <stddef.h>

extern void *__libc_malloc(size_t size);
extern void *__libc_realloc(void *ptr, size_t size);

static size_t fail_at;
static size_t allocation_count;

void fail_alloc_arm(size_t allocation)
{
    allocation_count = 0;
    fail_at = allocation;
}

void *malloc(size_t size)
{
    allocation_count++;
    if (fail_at != 0 && allocation_count == fail_at) {
        return NULL;
    }
    return __libc_malloc(size);
}

void *realloc(void *ptr, size_t size)
{
    allocation_count++;
    if (fail_at != 0 && allocation_count == fail_at) {
        return NULL;
    }
    return __libc_realloc(ptr, size);
}
