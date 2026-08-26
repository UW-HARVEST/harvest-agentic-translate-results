#define _GNU_SOURCE

#include <stdatomic.h>
#include <stddef.h>

extern void *__libc_malloc(size_t size);

static _Atomic int differential_mode = 0;

void differential_set_mode(int mode) {
    atomic_store_explicit(&differential_mode, mode, memory_order_release);
}

void *malloc(size_t size) {
    int expected = 2;
    if (size == 50 &&
        atomic_compare_exchange_strong_explicit(
            &differential_mode,
            &expected,
            0,
            memory_order_acq_rel,
            memory_order_acquire)) {
        return NULL;
    }
    return __libc_malloc(size);
}

int strncmp(const char *left, const char *right, size_t count) {
    int expected = 1;
    if (atomic_compare_exchange_strong_explicit(
            &differential_mode,
            &expected,
            0,
            memory_order_acq_rel,
            memory_order_acquire)) {
        return 1;
    }

    for (size_t index = 0; index < count; ++index) {
        unsigned char left_byte = (unsigned char)left[index];
        unsigned char right_byte = (unsigned char)right[index];
        if (left_byte != right_byte) {
            return (int)left_byte - (int)right_byte;
        }
        if (left_byte == '\0') {
            return 0;
        }
    }
    return 0;
}
