#define _GNU_SOURCE

#include <dlfcn.h>
#include <stdarg.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

static void *(*real_malloc)(size_t);
static int (*real_vsnprintf)(char *, size_t, const char *, va_list);
static size_t failed_size;
static int failure_armed;
static int empty_snprintf_armed;

static void resolve_symbols(void) {
    if (real_malloc == NULL) {
        real_malloc = (void *(*)(size_t))dlsym(RTLD_NEXT, "malloc");
    }
    if (real_vsnprintf == NULL) {
        real_vsnprintf =
            (int (*)(char *, size_t, const char *, va_list))dlsym(
                RTLD_NEXT, "vsnprintf");
    }
}

void ffi_fault_fail_next_malloc(size_t size) {
    failed_size = size;
    failure_armed = 1;
}

void ffi_fault_empty_next_snprintf(void) {
    empty_snprintf_armed = 1;
}

void *malloc(size_t size) {
    resolve_symbols();
    if (failure_armed && size == failed_size) {
        failure_armed = 0;
        return NULL;
    }
    return real_malloc(size);
}

int snprintf(char *buffer, size_t size, const char *format, ...) {
    int result;
    va_list arguments;

    if (empty_snprintf_armed) {
        empty_snprintf_armed = 0;
        if (size != 0) {
            buffer[0] = '\0';
        }
        return 0;
    }

    resolve_symbols();
    va_start(arguments, format);
    result = real_vsnprintf(buffer, size, format, arguments);
    va_end(arguments);
    return result;
}
