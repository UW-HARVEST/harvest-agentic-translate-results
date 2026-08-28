#include <dlfcn.h>
#include <stddef.h>
#include <stdio.h>

typedef int (*cleanup_fn)(int, int, int, int);

extern void *__libc_malloc(size_t size);

static int fail_cleanup_allocation;

void *malloc(size_t size) {
    if (fail_cleanup_allocation && size == 50) {
        return NULL;
    }
    return __libc_malloc(size);
}

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
    if (cleanup == NULL) {
        fputs(dlerror(), stderr);
        return 4;
    }

    fail_cleanup_allocation = 1;
    int result = cleanup(10, 20, 30, 40);
    fail_cleanup_allocation = 0;
    printf("return=%d\n", result);

    return dlclose(library) == 0 ? 0 : 5;
}
