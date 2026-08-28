#include <stddef.h>

extern void *__libc_malloc(size_t size);
extern void *__libc_realloc(void *pointer, size_t size);
extern void __libc_free(void *pointer);

static _Thread_local long allocations_before_failure = -1;
static _Thread_local size_t tracked_frees = 0;
static _Thread_local int tracking = 0;

void fail_alloc_after(long successful_allocations) {
    allocations_before_failure = successful_allocations;
    tracked_frees = 0;
    tracking = 1;
}

size_t fail_alloc_finish(void) {
    size_t result = tracked_frees;
    allocations_before_failure = -1;
    tracking = 0;
    return result;
}

static int allocation_should_fail(void) {
    if (!tracking || allocations_before_failure < 0) {
        return 0;
    }
    if (allocations_before_failure == 0) {
        allocations_before_failure = -1;
        return 1;
    }
    allocations_before_failure--;
    return 0;
}

void *malloc(size_t size) {
    if (allocation_should_fail()) {
        return NULL;
    }
    return __libc_malloc(size);
}

void *realloc(void *pointer, size_t size) {
    if (allocation_should_fail()) {
        return NULL;
    }
    return __libc_realloc(pointer, size);
}

void free(void *pointer) {
    if (tracking && pointer != NULL) {
        tracked_frees++;
    }
    __libc_free(pointer);
}
