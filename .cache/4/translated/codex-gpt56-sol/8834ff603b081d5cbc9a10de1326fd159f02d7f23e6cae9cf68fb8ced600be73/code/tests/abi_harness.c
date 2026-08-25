#include <dlfcn.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef void (*driver_fn_t)(const char *);
typedef void (*run_fn_t)(int);

static void *load_symbol(void *library, const char *name) {
    void *symbol = dlsym(library, name);
    const char *error = dlerror();
    if (error != NULL) {
        fprintf(stderr, "%s\n", error);
        exit(2);
    }
    return symbol;
}

int main(int argc, char **argv) {
    if (argc < 2) {
        return 2;
    }

    void *library = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (library == NULL) {
        fprintf(stderr, "%s\n", dlerror());
        return 2;
    }

    driver_fn_t call_driver;
    run_fn_t call_run;
    *(void **)(&call_driver) = load_symbol(library, "driver");
    *(void **)(&call_run) = load_symbol(library, "run");

    for (int i = 2; i < argc; i++) {
        if (strncmp(argv[i], "d=", 2) == 0) {
            call_driver(argv[i] + 2);
        } else if (strncmp(argv[i], "r=", 2) == 0) {
            char *end = NULL;
            long value = strtol(argv[i] + 2, &end, 10);
            if (end == argv[i] + 2 || *end != '\0' ||
                value < INT_MIN || value > INT_MAX) {
                return 2;
            }
            call_run((int)value);
        } else {
            return 2;
        }
    }

    return dlclose(library) == 0 ? 0 : 2;
}
