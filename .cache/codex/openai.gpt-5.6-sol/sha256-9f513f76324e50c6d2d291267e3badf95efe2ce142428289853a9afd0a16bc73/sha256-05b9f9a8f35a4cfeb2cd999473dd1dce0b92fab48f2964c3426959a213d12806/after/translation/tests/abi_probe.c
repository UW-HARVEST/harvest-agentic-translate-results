#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef void (*driver_fn)(int);
typedef void (*print_fn)(const int *);
typedef void (*void_fn)(void);

union symbol {
    void *pointer;
    driver_fn driver;
    print_fn print;
    void_fn no_args;
};

int main(int argc, char **argv)
{
    if (argc < 3) {
        return 2;
    }

    void *library = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (library == NULL) {
        fputs(dlerror(), stderr);
        return 3;
    }

    union symbol function = {.pointer = dlsym(library, argv[2])};
    if (function.pointer == NULL) {
        fputs(dlerror(), stderr);
        return 4;
    }

    if (strcmp(argv[2], "driver") == 0) {
        if (argc != 4) {
            return 2;
        }
        function.driver((int)strtol(argv[3], NULL, 10));
    } else if (strcmp(argv[2], "printIntPtrLine") == 0) {
        if (argc != 4) {
            return 2;
        }
        int value = (int)strtol(argv[3], NULL, 10);
        function.print(&value);
    } else {
        function.no_args();
    }

    return dlclose(library) == 0 ? 0 : 5;
}
