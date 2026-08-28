#include <stdatomic.h>
#include <stddef.h>

extern void *__libc_calloc(size_t count, size_t size);
extern void *__libc_malloc(size_t size);
extern void __libc_free(void *ptr);

static _Atomic long allocation_countdown = -1;
static _Atomic(void *) tracked_calloc = NULL;
static _Atomic int tracked_calloc_freed = 0;

static int should_fail(void)
{
    long remaining = atomic_load(&allocation_countdown);

    if (remaining <= 0) {
        return 0;
    }
    return atomic_fetch_sub(&allocation_countdown, 1) == 1;
}

void fail_alloc_arm(long allocation_number)
{
    atomic_store(&tracked_calloc, NULL);
    atomic_store(&tracked_calloc_freed, 0);
    atomic_store(&allocation_countdown, allocation_number);
}

void fail_alloc_disable(void)
{
    atomic_store(&allocation_countdown, -1);
}

int fail_alloc_tracked_calloc_was_freed(void)
{
    return atomic_load(&tracked_calloc_freed);
}

void *calloc(size_t count, size_t size)
{
    void *ptr;

    if (should_fail()) {
        return NULL;
    }
    ptr = __libc_calloc(count, size);
    atomic_store(&tracked_calloc, ptr);
    return ptr;
}

void *malloc(size_t size)
{
    if (should_fail()) {
        return NULL;
    }
    return __libc_malloc(size);
}

void free(void *ptr)
{
    if (ptr != NULL && ptr == atomic_load(&tracked_calloc)) {
        atomic_store(&tracked_calloc_freed, 1);
    }
    __libc_free(ptr);
}
