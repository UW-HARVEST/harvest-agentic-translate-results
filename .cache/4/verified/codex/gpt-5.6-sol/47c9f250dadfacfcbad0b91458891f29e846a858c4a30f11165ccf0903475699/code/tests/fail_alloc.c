#define _GNU_SOURCE
#include <stddef.h>

extern void *__libc_malloc(size_t size);
extern void *__libc_calloc(size_t count, size_t size);

static __thread long fail_after = -1;

void fail_alloc_set(long successful_allocations_before_failure) {
    fail_after = successful_allocations_before_failure;
}

static int should_fail(void) {
    if (fail_after < 0) {
        return 0;
    }
    if (fail_after == 0) {
        fail_after = -1;
        return 1;
    }
    fail_after--;
    return 0;
}

void *malloc(size_t size) {
    return should_fail() ? NULL : __libc_malloc(size);
}

void *calloc(size_t count, size_t size) {
    return should_fail() ? NULL : __libc_calloc(count, size);
}

