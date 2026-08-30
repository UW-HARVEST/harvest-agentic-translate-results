#include <dlfcn.h>
#include <limits.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>

typedef struct {
    unsigned int x : 2;
    unsigned int y : 3;
    bool b : 1;
    int z;
} foo_t;

typedef void (*driver_fn)(unsigned int, unsigned int, bool, int);
typedef void (*print_foo_fn)(const foo_t *);

static void *load_symbol(void *library, const char *name) {
    void *symbol = dlsym(library, name);
    if (symbol == NULL) {
        fprintf(stderr, "%s\n", dlerror());
        exit(EXIT_FAILURE);
    }
    return symbol;
}

int main(int argc, char **argv) {
    if (argc != 2) {
        return EXIT_FAILURE;
    }

    void *library = dlopen(argv[1], RTLD_NOW);
    if (library == NULL) {
        fprintf(stderr, "%s\n", dlerror());
        return EXIT_FAILURE;
    }

    driver_fn call_driver = (driver_fn)load_symbol(library, "driver");
    print_foo_fn call_print_foo =
        (print_foo_fn)load_symbol(library, "print_foo");

    call_driver(0, 0, false, 0);
    call_driver(3, 7, true, -1);
    call_driver(4, 8, false, INT_MIN);
    call_driver(UINT_MAX, UINT_MAX, true, INT_MAX);
    call_driver(42, 85, false, -123456789);

    const foo_t values[] = {
        {.x = 0, .y = 0, .b = false, .z = 0},
        {.x = 3, .y = 7, .b = true, .z = -1},
        {.x = 1, .y = 6, .b = false, .z = INT_MIN},
        {.x = 2, .y = 5, .b = true, .z = INT_MAX},
    };
    for (size_t index = 0; index < sizeof(values) / sizeof(values[0]); ++index) {
        call_print_foo(&values[index]);
    }

    return dlclose(library) == 0 ? EXIT_SUCCESS : EXIT_FAILURE;
}
