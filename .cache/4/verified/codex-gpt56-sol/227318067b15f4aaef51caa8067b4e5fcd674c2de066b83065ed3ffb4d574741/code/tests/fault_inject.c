#define _GNU_SOURCE

#include <stddef.h>
#include <stdint.h>

extern void *__libc_malloc(size_t size);

static int fail_malloc_enabled;
static size_t fail_malloc_size;
static int fail_memchr_enabled;
static unsigned char fail_memchr_byte;

void fault_fail_next_malloc_size(size_t size) {
    fail_malloc_size = size;
    fail_malloc_enabled = 1;
}

void fault_fail_next_memchr_byte(int value) {
    fail_memchr_byte = (unsigned char)value;
    fail_memchr_enabled = 1;
}

void *malloc(size_t size) {
    if (fail_malloc_enabled && size == fail_malloc_size) {
        fail_malloc_enabled = 0;
        return NULL;
    }
    return __libc_malloc(size);
}

void *memchr(const void *buffer, int value, size_t size) {
    const unsigned char *bytes = buffer;
    unsigned char target = (unsigned char)value;

    if (fail_memchr_enabled && target == fail_memchr_byte) {
        fail_memchr_enabled = 0;
        return NULL;
    }

    for (size_t index = 0; index < size; ++index) {
        if (bytes[index] == target) {
            return (void *)(bytes + index);
        }
    }
    return NULL;
}
