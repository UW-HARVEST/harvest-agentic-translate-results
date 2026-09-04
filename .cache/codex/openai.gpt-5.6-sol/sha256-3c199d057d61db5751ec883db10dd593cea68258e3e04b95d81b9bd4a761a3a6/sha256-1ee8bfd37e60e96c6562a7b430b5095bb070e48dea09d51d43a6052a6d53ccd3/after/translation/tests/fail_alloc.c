#include <stdatomic.h>
#include <stddef.h>
#include <string.h>

extern void *__libc_malloc(size_t size);
extern void *__libc_realloc(void *pointer, size_t size);

enum {
    FAIL_NONE = 0,
    FAIL_STRDUP = 1,
    FAIL_MALLOC = 2,
    FAIL_REALLOC = 3,
};

static _Atomic int failure = FAIL_NONE;

void fail_alloc_arm(int kind) {
    atomic_store_explicit(&failure, kind, memory_order_seq_cst);
}

static int consume_failure(int kind) {
    int expected = kind;
    return atomic_compare_exchange_strong_explicit(
        &failure,
        &expected,
        FAIL_NONE,
        memory_order_seq_cst,
        memory_order_seq_cst);
}

void *malloc(size_t size) {
    if (consume_failure(FAIL_MALLOC)) {
        return NULL;
    }
    return __libc_malloc(size);
}

void *realloc(void *pointer, size_t size) {
    if (consume_failure(FAIL_REALLOC)) {
        return NULL;
    }
    return __libc_realloc(pointer, size);
}

char *strdup(const char *string) {
    if (consume_failure(FAIL_STRDUP)) {
        return NULL;
    }

    size_t size = strlen(string) + 1;
    char *copy = __libc_malloc(size);
    if (copy != NULL) {
        memcpy(copy, string, size);
    }
    return copy;
}
