#define _GNU_SOURCE

#include <dlfcn.h>
#include <stddef.h>

static void *(*real_calloc)(size_t, size_t);
static void (*real_free)(void *);
static void *(*real_malloc)(size_t);
static _Thread_local int fail_kind;
static _Thread_local size_t fail_size;
static _Thread_local void *tracked_allocation;
static _Thread_local int tracked_allocation_freed;

__attribute__((constructor))
static void initialize_allocators(void)
{
    real_calloc = dlsym(RTLD_NEXT, "calloc");
    real_free = dlsym(RTLD_NEXT, "free");
    real_malloc = dlsym(RTLD_NEXT, "malloc");
}

void fail_alloc_arm(int kind, size_t size)
{
    fail_kind = kind;
    fail_size = size;
    tracked_allocation = NULL;
    tracked_allocation_freed = 0;
}

int fail_alloc_was_freed(void)
{
    return tracked_allocation_freed;
}

void *calloc(size_t count, size_t size)
{
    void *allocation;

    if (fail_kind == 1 && count * size == fail_size) {
        fail_kind = 0;
        return NULL;
    }
    allocation = real_calloc(count, size);
    if (fail_kind == 2) {
        tracked_allocation = allocation;
    }
    return allocation;
}

void free(void *allocation)
{
    if (allocation == tracked_allocation) {
        tracked_allocation = NULL;
        tracked_allocation_freed = 1;
    }
    real_free(allocation);
}

void *malloc(size_t size)
{
    if (fail_kind == 2 && size == fail_size) {
        fail_kind = 0;
        return NULL;
    }
    return real_malloc(size);
}
