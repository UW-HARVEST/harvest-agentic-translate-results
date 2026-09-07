#include <stddef.h>
#include <string.h>

extern void *__libc_malloc(size_t size);
extern void *__libc_realloc(void *ptr, size_t size);

static __thread int failure_kind;
static __thread unsigned int calls_remaining;

void fail_alloc_arm(int kind, unsigned int nth_call)
{
    failure_kind = kind;
    calls_remaining = nth_call;
}

static int should_fail(int kind)
{
    if (failure_kind != kind || calls_remaining == 0) {
        return 0;
    }

    calls_remaining--;
    if (calls_remaining == 0) {
        failure_kind = 0;
        return 1;
    }

    return 0;
}

void *malloc(size_t size)
{
    if (should_fail(1)) {
        return NULL;
    }
    return __libc_malloc(size);
}

void *realloc(void *ptr, size_t size)
{
    if (should_fail(2)) {
        return NULL;
    }
    return __libc_realloc(ptr, size);
}

char *strdup(const char *value)
{
    size_t length;
    char *copy;

    if (should_fail(3)) {
        return NULL;
    }

    length = strlen(value) + 1;
    copy = __libc_malloc(length);
    if (copy != NULL) {
        memcpy(copy, value, length);
    }
    return copy;
}
