#define _GNU_SOURCE
#include <dlfcn.h>
#include <stddef.h>
#include <stdlib.h>
#include <string.h>

extern void *__libc_malloc(size_t size);
extern void __libc_free(void *pointer);

static _Thread_local unsigned int target_malloc_call;

void *malloc(size_t size) {
    Dl_info info;
    void *caller = __builtin_return_address(0);
    const char *mode = getenv("FAIL_COMPARE_ALLOCATIONS_MALLOC");

    if (mode != NULL &&
        dladdr(caller, &info) != 0 && info.dli_sname != NULL &&
        strcmp(info.dli_sname, "compare_allocations") == 0) {
        target_malloc_call++;
        if (strcmp(mode, "both") == 0 ||
            (strcmp(mode, "first") == 0 && target_malloc_call == 1) ||
            (strcmp(mode, "second") == 0 && target_malloc_call == 2)) {
            return NULL;
        }
    }

    return __libc_malloc(size);
}

void free(void *pointer) {
    __libc_free(pointer);
}
