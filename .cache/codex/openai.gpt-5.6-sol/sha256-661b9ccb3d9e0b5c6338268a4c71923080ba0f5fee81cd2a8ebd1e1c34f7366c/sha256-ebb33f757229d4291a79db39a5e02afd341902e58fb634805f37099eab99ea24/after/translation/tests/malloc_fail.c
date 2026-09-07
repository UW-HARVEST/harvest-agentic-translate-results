#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdatomic.h>
#include <stddef.h>

extern void *__libc_malloc(size_t);

static _Atomic size_t fail_size = 0;
static _Atomic(void *) fail_base = NULL;

void arm_fail_size(size_t size, void *symbol) {
    Dl_info target = {0};
    if (dladdr(symbol, &target) == 0) {
        return;
    }
    atomic_store(&fail_base, target.dli_fbase);
    atomic_store(&fail_size, size);
}

void *malloc(size_t size) {
    Dl_info caller = {0};
    void *return_address = __builtin_return_address(0);
    void *target_base = atomic_load(&fail_base);
    size_t expected = size;
    if (expected != 0 && target_base != NULL &&
        dladdr(return_address, &caller) != 0 &&
        caller.dli_fbase == target_base &&
        atomic_compare_exchange_strong(&fail_size, &expected, 0)) {
        atomic_store(&fail_base, NULL);
        return NULL;
    }
    return __libc_malloc(size);
}
