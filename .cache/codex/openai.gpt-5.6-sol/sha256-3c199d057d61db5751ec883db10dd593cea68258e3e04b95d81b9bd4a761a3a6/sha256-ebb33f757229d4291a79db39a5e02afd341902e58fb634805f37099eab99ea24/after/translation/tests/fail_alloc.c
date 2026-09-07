#include <stddef.h>
#include <string.h>

extern void *__libc_malloc(size_t);
extern void *__libc_realloc(void *, size_t);

static _Thread_local int failure;

void fail_next_strdup(void) {
    failure = 1;
}

void fail_next_malloc(void) {
    failure = 2;
}

void fail_next_realloc(void) {
    failure = 3;
}

void *malloc(size_t size) {
    if (failure == 2) {
        failure = 0;
        return NULL;
    }
    return __libc_malloc(size);
}

void *realloc(void *pointer, size_t size) {
    if (failure == 3) {
        failure = 0;
        return NULL;
    }
    return __libc_realloc(pointer, size);
}

char *strdup(const char *string) {
    if (failure == 1) {
        failure = 0;
        return NULL;
    }

    size_t size = strlen(string) + 1;
    char *copy = __libc_malloc(size);
    if (copy != NULL) {
        memcpy(copy, string, size);
    }
    return copy;
}
