#include <stddef.h>

extern void *__libc_malloc(size_t size);

static int fail_state_allocation;

void arm_state_malloc_failure(void) {
    __atomic_store_n(&fail_state_allocation, 1, __ATOMIC_SEQ_CST);
}

void *malloc(size_t size) {
    if (size == 12 &&
        __atomic_exchange_n(&fail_state_allocation, 0, __ATOMIC_SEQ_CST)) {
        return NULL;
    }

    return __libc_malloc(size);
}
