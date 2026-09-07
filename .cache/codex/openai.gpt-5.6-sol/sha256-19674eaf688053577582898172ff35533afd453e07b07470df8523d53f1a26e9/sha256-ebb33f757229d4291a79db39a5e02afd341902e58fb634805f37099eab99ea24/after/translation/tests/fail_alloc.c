#define _GNU_SOURCE
#include <dlfcn.h>
#include <stddef.h>
#include <string.h>

extern void *__libc_malloc(size_t size);

static size_t target_size;
static int remaining_matches;
static char target_module[256];

void configure_fail_alloc(size_t size, int occurrence, const char *module)
{
    target_size = size;
    remaining_matches = occurrence;
    strncpy(target_module, module, sizeof(target_module) - 1);
    target_module[sizeof(target_module) - 1] = '\0';
}

void *malloc(size_t size)
{
    if (target_size != 0 && size == target_size && remaining_matches > 0) {
        Dl_info caller;
        if (dladdr(__builtin_return_address(0), &caller) != 0 &&
            caller.dli_fname != NULL &&
            strstr(caller.dli_fname, target_module) != NULL) {
            remaining_matches--;
            if (remaining_matches == 0) {
                target_size = 0;
                return NULL;
            }
        }
    }

    return __libc_malloc(size);
}
