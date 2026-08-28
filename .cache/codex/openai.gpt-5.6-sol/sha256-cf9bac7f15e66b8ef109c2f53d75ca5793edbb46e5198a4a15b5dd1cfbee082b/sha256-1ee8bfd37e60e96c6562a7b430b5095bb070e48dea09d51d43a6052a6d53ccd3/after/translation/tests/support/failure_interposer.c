#include <stddef.h>

extern void *__libc_malloc(size_t size);

static _Thread_local int fail_malloc_50;
static _Thread_local int fail_strncmp;

void fail_next_malloc_50(void) {
    fail_malloc_50 = 1;
}

void fail_next_strncmp(void) {
    fail_strncmp = 1;
}

void *malloc(size_t size) {
    if (fail_malloc_50 && size == 50) {
        fail_malloc_50 = 0;
        return NULL;
    }
    return __libc_malloc(size);
}

int strncmp(const char *left, const char *right, size_t count) {
    if (fail_strncmp) {
        fail_strncmp = 0;
        return 1;
    }

    for (size_t i = 0; i < count; ++i) {
        unsigned char left_byte = (unsigned char)left[i];
        unsigned char right_byte = (unsigned char)right[i];
        if (left_byte != right_byte) {
            return left_byte < right_byte ? -1 : 1;
        }
        if (left_byte == '\0') {
            return 0;
        }
    }
    return 0;
}
