#include <stddef.h>

extern void *__libc_malloc(size_t size);
extern void *__libc_realloc(void *ptr, size_t size);

static _Thread_local long allocation_to_fail = -1;

void fail_alloc_arm(unsigned long call_number)
{
    allocation_to_fail = (long) call_number;
}

void fail_alloc_disarm(void)
{
    allocation_to_fail = -1;
}

static int should_fail(void)
{
    if (allocation_to_fail < 0) {
        return 0;
    }
    allocation_to_fail--;
    if (allocation_to_fail == 0) {
        allocation_to_fail = -1;
        return 1;
    }
    return 0;
}

void *malloc(size_t size)
{
    if (should_fail()) {
        return NULL;
    }
    return __libc_malloc(size);
}

void *realloc(void *ptr, size_t size)
{
    if (should_fail()) {
        return NULL;
    }
    return __libc_realloc(ptr, size);
}
