#define _GNU_SOURCE
#include <dlfcn.h>
#include <stddef.h>

static __thread size_t fail_size;
static __thread int fail_occurrence;
static __thread int matching_calls;
static void *(*real_malloc)(size_t);

__attribute__((constructor))
static void initialize_real_malloc(void) {
    real_malloc = dlsym(RTLD_NEXT, "malloc");
}

void malloc_fail_arm(size_t size, int occurrence) {
    fail_size = size;
    fail_occurrence = occurrence;
    matching_calls = 0;
}

void *malloc(size_t size) {
    if (!real_malloc) {
        real_malloc = dlsym(RTLD_NEXT, "malloc");
    }
    if (fail_occurrence > 0 && size == fail_size) {
        matching_calls++;
        if (matching_calls == fail_occurrence) {
            fail_occurrence = 0;
            return NULL;
        }
    }
    return real_malloc(size);
}
