#include <stddef.h>

extern void *__libc_malloc(size_t size);

static __thread int fail_next_state_allocation = 0;

void arm_state_malloc_failure(void) {
    fail_next_state_allocation = 1;
}

void *malloc(size_t size) {
    if (fail_next_state_allocation && size == sizeof(int) * 3) {
        fail_next_state_allocation = 0;
        return NULL;
    }

    return __libc_malloc(size);
}
