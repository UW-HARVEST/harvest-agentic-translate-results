#include <dlfcn.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>

typedef void (*fma_array_fn)(
    int *out,
    const int *mul1,
    const int *mul2,
    const int *add,
    int len);
typedef int (*call_fma_fn)(const int *data, int len);
typedef void (*driver_fn)(const char *input);

static void *load_symbol(void *library, const char *name) {
    void *symbol = dlsym(library, name);
    if (symbol == NULL) {
        fprintf(stderr, "dlsym(%s): %s\n", name, dlerror());
        exit(2);
    }
    return symbol;
}

int main(int argc, char **argv) {
    if (argc != 3) {
        fprintf(stderr, "usage: %s LIBRARY INPUT\n", argv[0]);
        return 2;
    }

    void *library = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (library == NULL) {
        fprintf(stderr, "dlopen: %s\n", dlerror());
        return 2;
    }

    fma_array_fn fma_array = (fma_array_fn)load_symbol(library, "fma_array");
    call_fma_fn call_fma = (call_fma_fn)load_symbol(library, "call_fma");
    driver_fn driver = (driver_fn)load_symbol(library, "driver");

    const int mul1[] = {0, 1, -1, INT_MAX, INT_MIN, 46341, -46341};
    const int mul2[] = {9, -7, INT_MIN, 2, -1, 46341, 46341};
    const int add[] = {3, 4, 5, 6, 7, INT_MAX, INT_MIN};
    int out[] = {101, 102, 103, 104, 105, 106, 107};
    int sentinel = 0x12345678;
    const int data[] = {INT_MIN, -42, 0, 17, INT_MAX};

    fma_array(out, mul1, mul2, add, 7);
    fwrite(out, sizeof(out), 1, stdout);
    fma_array(&sentinel, mul1, mul2, add, 0);
    fma_array(&sentinel, mul1, mul2, add, -1);
    fwrite(&sentinel, sizeof(sentinel), 1, stdout);

    int call_results[] = {
        call_fma(data, 0),
        call_fma(data, 1),
        call_fma(data, 3),
        call_fma(data, 5),
    };
    fwrite(call_results, sizeof(call_results), 1, stdout);

    driver(argv[2]);
    fflush(stdout);
    dlclose(library);
    return 0;
}
