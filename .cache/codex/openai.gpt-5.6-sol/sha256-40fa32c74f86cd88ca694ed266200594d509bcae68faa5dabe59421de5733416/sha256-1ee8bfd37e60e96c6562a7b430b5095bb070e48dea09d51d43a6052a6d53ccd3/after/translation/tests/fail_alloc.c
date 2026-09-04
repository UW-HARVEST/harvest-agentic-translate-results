#include <stddef.h>

extern void *__libc_malloc(size_t size);

static __thread size_t matrixsum_failure_size;
static __thread int matrixsum_failure_enabled;

void matrixsum_fail_allocation_of_size(size_t size) {
    matrixsum_failure_size = size;
    matrixsum_failure_enabled = 1;
}

void *malloc(size_t size) {
    if (matrixsum_failure_enabled && size == matrixsum_failure_size) {
        matrixsum_failure_enabled = 0;
        return NULL;
    }
    return __libc_malloc(size);
}
