#define _GNU_SOURCE

#include <dlfcn.h>
#include <stddef.h>

static void *(*real_malloc)(size_t);
static size_t rejected_size;
static int reject_next;

void fail_next_allocation_of_size(size_t size)
{
    rejected_size = size;
    reject_next = 1;
}

void *malloc(size_t size)
{
    if (real_malloc == NULL) {
        real_malloc = (void *(*)(size_t))dlsym(RTLD_NEXT, "malloc");
    }

    if (reject_next && size == rejected_size) {
        reject_next = 0;
        return NULL;
    }

    return real_malloc(size);
}
