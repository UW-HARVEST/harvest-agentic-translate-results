#define _GNU_SOURCE

#include <stddef.h>

extern void *__libc_malloc(size_t size);

static __thread int fail_next_50_byte_malloc;
static __thread int mismatch_next_strncmp;

void arm_fail_next_50_byte_malloc(void) {
    fail_next_50_byte_malloc = 1;
}

void arm_mismatch_next_strncmp(void) {
    mismatch_next_strncmp = 1;
}

void *malloc(size_t size) {
    if (fail_next_50_byte_malloc && size == 50) {
        fail_next_50_byte_malloc = 0;
        return NULL;
    }
    return __libc_malloc(size);
}

int strncmp(const char *left, const char *right, size_t count) {
    size_t index;

    if (mismatch_next_strncmp) {
        mismatch_next_strncmp = 0;
        return 1;
    }

    for (index = 0; index < count; ++index) {
        const unsigned char left_byte = (unsigned char)left[index];
        const unsigned char right_byte = (unsigned char)right[index];
        if (left_byte != right_byte) {
            return (int)left_byte - (int)right_byte;
        }
        if (left_byte == '\0') {
            return 0;
        }
    }
    return 0;
}
