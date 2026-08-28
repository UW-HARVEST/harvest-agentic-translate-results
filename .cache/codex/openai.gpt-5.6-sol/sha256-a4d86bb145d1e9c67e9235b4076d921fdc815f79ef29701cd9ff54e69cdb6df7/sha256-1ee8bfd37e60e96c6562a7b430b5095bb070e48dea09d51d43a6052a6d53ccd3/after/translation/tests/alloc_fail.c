#define _GNU_SOURCE

#include <stddef.h>
#include <string.h>

extern void *__libc_malloc(size_t size);
extern void *__libc_realloc(void *pointer, size_t size);

static int failure_kind;

void alloc_fail_arm(int kind) {
    failure_kind = kind;
}

void *malloc(size_t size) {
    if (failure_kind == 2) {
        failure_kind = 0;
        return NULL;
    }
    return __libc_malloc(size);
}

void *realloc(void *pointer, size_t size) {
    if (failure_kind == 3) {
        failure_kind = 0;
        return NULL;
    }
    return __libc_realloc(pointer, size);
}

char *strdup(const char *string) {
    if (failure_kind == 1) {
        failure_kind = 0;
        return NULL;
    }

    size_t size = strlen(string) + 1;
    char *copy = __libc_malloc(size);
    if (copy != NULL) {
        memcpy(copy, string, size);
    }
    return copy;
}
