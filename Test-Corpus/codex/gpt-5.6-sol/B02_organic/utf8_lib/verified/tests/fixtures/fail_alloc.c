#include <stddef.h>

extern void *__libc_malloc(size_t size);
extern void *__libc_realloc(void *pointer, size_t size);

static _Thread_local int failure_mode;

void fail_alloc_configure(int mode) {
    failure_mode = mode;
}

void *malloc(size_t size) {
    if (failure_mode == 1) {
        return NULL;
    }
    return __libc_malloc(size);
}

void *realloc(void *pointer, size_t size) {
    if (failure_mode == 2) {
        return NULL;
    }
    return __libc_realloc(pointer, size);
}

char *strdup(const char *string) {
    if (failure_mode == 3) {
        return NULL;
    }

    size_t length = 0;
    while (string[length] != '\0') {
        length++;
    }

    char *copy = __libc_malloc(length + 1);
    if (copy == NULL) {
        return NULL;
    }
    for (size_t index = 0; index <= length; index++) {
        copy[index] = string[index];
    }
    return copy;
}
