#include <stddef.h>

extern void *__libc_malloc(size_t size);
extern void __libc_free(void *pointer);

static long fail_after = -1;

void dag_test_fail_alloc_after(long successful_allocations) {
    fail_after = successful_allocations;
}

void *dag_malloc(size_t size) {
    if (fail_after == 0) {
        return NULL;
    }
    if (fail_after > 0) {
        fail_after--;
    }
    return __libc_malloc(size);
}

void dag_free(void *pointer) {
    __libc_free(pointer);
}
