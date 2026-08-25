#include <stdatomic.h>
#include <stddef.h>

extern void *__libc_malloc(size_t size);
extern void *__libc_realloc(void *ptr, size_t size);

static _Atomic long malloc_countdown = -1;
static _Atomic long realloc_countdown = -1;

void fail_malloc_after(long successful_calls) {
    atomic_store(&malloc_countdown, successful_calls);
}

void fail_realloc_after(long successful_calls) {
    atomic_store(&realloc_countdown, successful_calls);
}

static int should_fail(_Atomic long *countdown) {
    long current = atomic_load(countdown);

    while (current >= 0) {
        if (current == 0) {
            if (atomic_compare_exchange_weak(countdown, &current, -1)) {
                return 1;
            }
        } else if (atomic_compare_exchange_weak(
                       countdown, &current, current - 1)) {
            return 0;
        }
    }

    return 0;
}

void *malloc(size_t size) {
    if (should_fail(&malloc_countdown)) {
        return NULL;
    }
    return __libc_malloc(size);
}

void *realloc(void *ptr, size_t size) {
    if (should_fail(&realloc_countdown)) {
        return NULL;
    }
    return __libc_realloc(ptr, size);
}
