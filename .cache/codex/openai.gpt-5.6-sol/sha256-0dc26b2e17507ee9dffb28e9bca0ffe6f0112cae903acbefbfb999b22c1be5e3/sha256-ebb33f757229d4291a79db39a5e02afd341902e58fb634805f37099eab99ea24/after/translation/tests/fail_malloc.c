#include <stdatomic.h>
#include <stddef.h>

extern void *__libc_malloc(size_t size);

static _Atomic int fail_next = 0;

void fail_next_malloc(void)
{
    atomic_store(&fail_next, 1);
}

void *malloc(size_t size)
{
    if (atomic_exchange(&fail_next, 0))
    {
        return NULL;
    }

    return __libc_malloc(size);
}

