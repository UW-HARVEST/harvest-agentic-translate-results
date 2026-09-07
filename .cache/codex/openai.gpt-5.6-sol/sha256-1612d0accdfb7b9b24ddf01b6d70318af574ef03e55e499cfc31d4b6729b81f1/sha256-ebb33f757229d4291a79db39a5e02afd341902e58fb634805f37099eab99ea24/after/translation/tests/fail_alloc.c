#include <stdatomic.h>
#include <stddef.h>

extern void *__libc_malloc(size_t);
extern void *__libc_realloc(void *, size_t);

static _Atomic int fail_kind = 0;
static _Atomic int fail_countdown = 0;

void fail_alloc_arm(int kind, int countdown) {
    atomic_store(&fail_countdown, countdown);
    atomic_store(&fail_kind, kind);
}

static int should_fail(int kind) {
    if (atomic_load(&fail_kind) != kind) {
        return 0;
    }

    int before = atomic_fetch_sub(&fail_countdown, 1);
    if (before == 1) {
        atomic_store(&fail_kind, 0);
        return 1;
    }
    return 0;
}

void *malloc(size_t size) {
    if (should_fail(1)) {
        return NULL;
    }
    return __libc_malloc(size);
}

void *realloc(void *pointer, size_t size) {
    if (should_fail(2)) {
        return NULL;
    }
    return __libc_realloc(pointer, size);
}
