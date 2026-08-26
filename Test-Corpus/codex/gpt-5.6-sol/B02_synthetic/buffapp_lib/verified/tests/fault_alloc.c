#include <stdatomic.h>
#include <stddef.h>

extern void *__libc_malloc(size_t size);
extern void *__libc_realloc(void *pointer, size_t size);
extern void __libc_free(void *pointer);

static _Atomic long malloc_after = -1;
static _Atomic long realloc_after = -1;
static _Atomic long free_calls = 0;

void fault_malloc_after(long calls)
{
    atomic_store(&malloc_after, calls);
}

void fault_realloc_after(long calls)
{
    atomic_store(&realloc_after, calls);
}

void fault_reset(void)
{
    atomic_store(&malloc_after, -1);
    atomic_store(&realloc_after, -1);
    atomic_store(&free_calls, 0);
}

long fault_free_calls(void)
{
    return atomic_load(&free_calls);
}

void *malloc(size_t size)
{
    long remaining = atomic_load(&malloc_after);
    if (remaining >= 0) {
        if (remaining == 0) {
            atomic_store(&malloc_after, -1);
            return NULL;
        }
        atomic_fetch_sub(&malloc_after, 1);
    }
    return __libc_malloc(size);
}

void *realloc(void *pointer, size_t size)
{
    long remaining = atomic_load(&realloc_after);
    if (remaining >= 0) {
        if (remaining == 0) {
            atomic_store(&realloc_after, -1);
            return NULL;
        }
        atomic_fetch_sub(&realloc_after, 1);
    }
    return __libc_realloc(pointer, size);
}

void free(void *pointer)
{
    atomic_fetch_add(&free_calls, 1);
    __libc_free(pointer);
}
