#include <dlfcn.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>

typedef int (*cleanup_fn)(int, int, int, int);
typedef void (*print_result_fn)(const char *, int);
typedef void (*cleanup_resources_fn)(char *);

int main(int argc, char **argv) {
    if (argc != 2) {
        return 2;
    }

    void *library = dlopen(argv[1], RTLD_NOW);
    if (library == NULL) {
        fputs(dlerror(), stderr);
        return 3;
    }

    cleanup_fn cleanup = (cleanup_fn)dlsym(library, "cleanup");
    print_result_fn print_result =
        (print_result_fn)dlsym(library, "print_result");
    cleanup_resources_fn cleanup_resources =
        (cleanup_resources_fn)dlsym(library, "cleanup_resources");
    if (cleanup == NULL || print_result == NULL || cleanup_resources == NULL) {
        fputs(dlerror(), stderr);
        return 4;
    }

    const int cases[][4] = {
        {0, 1, -2, 3},
        {10, 20, 30, 40},
        {10, 10, 30, 30},
        {9, 19, 29, 39},
        {-10, -20, -30, -40},
        {INT_MAX, 0, 0, 0},
        {INT_MIN, 0, 0, 0},
        {INT_MAX, 1, 0, 0},
        {INT_MIN, -1, 0, 0},
    };
    const size_t case_count = sizeof(cases) / sizeof(cases[0]);
    for (size_t i = 0; i < case_count; ++i) {
        int result =
            cleanup(cases[i][0], cases[i][1], cases[i][2], cases[i][3]);
        printf("return[%zu]=%d\n", i, result);
    }

    print_result("Final result", -42);
    print_result("percent % text", 123456789);
    print_result("", 0);

    char *allocation = malloc(17);
    if (allocation == NULL) {
        return 5;
    }
    cleanup_resources(allocation);
    cleanup_resources(NULL);

    return dlclose(library) == 0 ? 0 : 6;
}
