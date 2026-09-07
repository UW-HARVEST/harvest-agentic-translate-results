#include <stdatomic.h>
#include <stddef.h>
#include <stdint.h>

extern void *__libc_malloc(size_t size);

static _Atomic size_t malloc_failure_size = 0;
static _Atomic int memchr_failure_count = 0;

void interpose_fail_next_malloc_of_size(size_t size) {
    atomic_store(&malloc_failure_size, size);
}

void interpose_fail_next_memchr(void) {
    atomic_store(&memchr_failure_count, 1);
}

void *malloc(size_t size) {
    size_t expected = atomic_load(&malloc_failure_size);
    if (expected != 0 && size == expected) {
        atomic_store(&malloc_failure_size, 0);
        return NULL;
    }
    return __libc_malloc(size);
}

void *memchr(const void *buffer, int target, size_t size) {
    if (atomic_exchange(&memchr_failure_count, 0) != 0) {
        return NULL;
    }

    const unsigned char *bytes = (const unsigned char *)buffer;
    const unsigned char wanted = (unsigned char)target;
    for (size_t index = 0; index < size; ++index) {
        if (bytes[index] == wanted) {
            return (void *)(uintptr_t)(bytes + index);
        }
    }
    return NULL;
}
