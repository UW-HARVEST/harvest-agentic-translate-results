#define _GNU_SOURCE

#include <stddef.h>
#include <string.h>

extern void *__libc_malloc(size_t size);

static size_t failed_size;
static int fail_strdup;

void fault_fail_malloc_size(size_t size) {
    failed_size = size;
}

void fault_fail_next_strdup(void) {
    fail_strdup = 1;
}

void *malloc(size_t size) {
    if (failed_size != 0 && size == failed_size) {
        failed_size = 0;
        return NULL;
    }
    return __libc_malloc(size);
}

char *strdup(const char *source) {
    if (fail_strdup) {
        fail_strdup = 0;
        return NULL;
    }

    size_t length = strlen(source) + 1;
    char *copy = __libc_malloc(length);
    if (copy != NULL) {
        memcpy(copy, source, length);
    }
    return copy;
}
