#define _GNU_SOURCE

#include <dlfcn.h>
#include <errno.h>
#include <stddef.h>
#include <stdlib.h>
#include <string.h>

extern void *__libc_malloc(size_t size);

static unsigned long targeted_allocations;

static int called_from_target(void *caller) {
    const char *target = getenv("DIFF_TARGET_DSO");
    Dl_info info;

    return target != NULL && dladdr(caller, &info) != 0 &&
           info.dli_fname != NULL && strcmp(info.dli_fname, target) == 0;
}

void *malloc(size_t size) {
    void *caller = __builtin_return_address(0);
    const char *fail_at_text;
    unsigned long fail_at;
    unsigned long current;

    if (!called_from_target(caller)) {
        return __libc_malloc(size);
    }

    fail_at_text = getenv("DIFF_FAIL_MALLOC_AT");
    if (fail_at_text == NULL) {
        return __libc_malloc(size);
    }

    fail_at = strtoul(fail_at_text, NULL, 10);
    current = __sync_add_and_fetch(&targeted_allocations, 1);
    if (current == fail_at) {
        errno = ENOMEM;
        return NULL;
    }
    return __libc_malloc(size);
}

char *strdup(const char *source) {
    void *caller = __builtin_return_address(0);
    size_t size;
    char *copy;

    if (called_from_target(caller) && getenv("DIFF_FAIL_STRDUP") != NULL) {
        errno = ENOMEM;
        return NULL;
    }

    size = strlen(source) + 1;
    copy = __libc_malloc(size);
    if (copy != NULL) {
        memcpy(copy, source, size);
    }
    return copy;
}
