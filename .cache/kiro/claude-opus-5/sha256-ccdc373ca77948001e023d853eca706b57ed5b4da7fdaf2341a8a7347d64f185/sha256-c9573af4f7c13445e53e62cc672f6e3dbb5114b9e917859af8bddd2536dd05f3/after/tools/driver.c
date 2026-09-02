/* Standalone differential driver: dlopen a shared object exporting
 *   void long_exec(unsigned int);
 *   int  array[256*1024];
 * call long_exec(seed), capture the printf output on stdout, and dump the
 * final contents of `array` to a file so the full post-state can be compared
 * (not just the printed XOR).
 *
 * usage: driver <so-path> <seed> <array-dump-out>
 */
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>

#define ARRAY_SIZE (256 * 1024)

int main(int argc, char **argv) {
    if (argc != 4) {
        fprintf(stderr, "usage: %s <so> <seed> <dump>\n", argv[0]);
        return 2;
    }
    void *h = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!h) {
        fprintf(stderr, "dlopen: %s\n", dlerror());
        return 3;
    }
    void (*long_exec)(unsigned int) = (void (*)(unsigned int))dlsym(h, "long_exec");
    int *array = (int *)dlsym(h, "array");
    if (!long_exec || !array) {
        fprintf(stderr, "dlsym failed\n");
        return 4;
    }
    unsigned int seed = (unsigned int)strtoul(argv[2], NULL, 10);
    long_exec(seed);
    fflush(stdout);

    FILE *f = fopen(argv[3], "wb");
    if (!f) {
        perror("fopen");
        return 5;
    }
    if (fwrite(array, sizeof(int), ARRAY_SIZE, f) != ARRAY_SIZE) {
        perror("fwrite");
        return 6;
    }
    fclose(f);
    return 0;
}
