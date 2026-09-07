#define _GNU_SOURCE

#include <dlfcn.h>
#include <errno.h>
#include <stddef.h>
#include <stdlib.h>
#include <string.h>

static _Thread_local long allocations_before_failure = -1;

static int should_fail(void) {
    if (allocations_before_failure < 0) {
        return 0;
    }
    if (allocations_before_failure == 0) {
        allocations_before_failure = -1;
        errno = ENOMEM;
        return 1;
    }
    allocations_before_failure--;
    return 0;
}

void fail_alloc_arm(long successful_allocations_before_failure) {
    allocations_before_failure = successful_allocations_before_failure;
}

void fail_alloc_disarm(void) {
    allocations_before_failure = -1;
}

void *malloc(size_t size) {
    static void *(*real_malloc)(size_t);
    if (real_malloc == NULL) {
        real_malloc = dlsym(RTLD_NEXT, "malloc");
    }
    if (should_fail()) {
        return NULL;
    }
    return real_malloc(size);
}

void *calloc(size_t count, size_t size) {
    static void *(*real_calloc)(size_t, size_t);
    if (real_calloc == NULL) {
        real_calloc = dlsym(RTLD_NEXT, "calloc");
    }
    if (should_fail()) {
        return NULL;
    }
    return real_calloc(count, size);
}

void *realloc(void *pointer, size_t size) {
    static void *(*real_realloc)(void *, size_t);
    if (real_realloc == NULL) {
        real_realloc = dlsym(RTLD_NEXT, "realloc");
    }
    if (should_fail()) {
        return NULL;
    }
    return real_realloc(pointer, size);
}

char *strdup(const char *source) {
    static char *(*real_strdup)(const char *);
    if (real_strdup == NULL) {
        real_strdup = dlsym(RTLD_NEXT, "strdup");
    }
    if (should_fail()) {
        return NULL;
    }
    return real_strdup(source);
}
