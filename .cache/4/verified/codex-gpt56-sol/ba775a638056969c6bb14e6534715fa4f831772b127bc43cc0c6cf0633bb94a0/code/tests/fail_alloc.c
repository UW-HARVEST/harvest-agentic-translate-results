#define _GNU_SOURCE
#include <dlfcn.h>
#include <stddef.h>
#include <stdlib.h>

static _Thread_local long failure_index = -1;
static void *(*real_malloc)(size_t);
static void *(*real_realloc)(void *, size_t);
static void (*real_free)(void *);

void fail_alloc_at(long index) {
    failure_index = index;
}

void fail_alloc_disable(void) {
    failure_index = -1;
}

static int should_fail(void) {
    if (failure_index < 0) {
        return 0;
    }
    if (failure_index == 0) {
        failure_index = -1;
        return 1;
    }
    failure_index--;
    return 0;
}

void *malloc(size_t size) {
    if (!real_malloc) {
        real_malloc = dlsym(RTLD_NEXT, "malloc");
    }
    if (should_fail()) {
        return NULL;
    }
    return real_malloc(size);
}

void *realloc(void *pointer, size_t size) {
    if (!real_realloc) {
        real_realloc = dlsym(RTLD_NEXT, "realloc");
    }
    if (should_fail()) {
        return NULL;
    }
    return real_realloc(pointer, size);
}

void free(void *pointer) {
    if (!real_free) {
        real_free = dlsym(RTLD_NEXT, "free");
    }
    real_free(pointer);
}

