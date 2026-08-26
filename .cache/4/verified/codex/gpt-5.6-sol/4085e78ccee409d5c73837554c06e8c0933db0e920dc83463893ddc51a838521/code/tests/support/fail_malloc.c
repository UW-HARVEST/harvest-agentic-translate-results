#include <stdatomic.h>
#include <stddef.h>

extern void *__libc_malloc(size_t size);

static _Atomic int fail_next_allocation = 0;

void fail_malloc_arm(void)
{
    atomic_store(&fail_next_allocation, 1);
}

void *malloc(size_t size)
{
    if (atomic_exchange(&fail_next_allocation, 0))
    {
        return NULL;
    }

    return __libc_malloc(size);
}
