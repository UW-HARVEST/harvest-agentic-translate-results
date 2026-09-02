/* Search for a seed whose printed XOR is negative, using the fast Rust .so.
 * usage: findneg <so> <first-seed> <count>
 */
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>

#define ARRAY_SIZE (256 * 1024)

int main(int argc, char **argv) {
    if (argc != 4) return 2;
    void *h = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!h) { fprintf(stderr, "dlopen: %s\n", dlerror()); return 3; }
    void (*long_exec)(unsigned int) = (void (*)(unsigned int))dlsym(h, "long_exec");
    int *array = (int *)dlsym(h, "array");
    if (!long_exec || !array) return 4;

    unsigned int first = (unsigned int)strtoul(argv[2], NULL, 10);
    unsigned long count = strtoul(argv[3], NULL, 10);

    /* silence the library's own printf */
    FILE *devnull = freopen("/dev/null", "w", stdout);
    (void)devnull;

    for (unsigned long i = 0; i < count; i++) {
        unsigned int seed = first + (unsigned int)i;
        long_exec(seed);
        int x = 0;
        for (size_t j = 0; j < ARRAY_SIZE; j++) x ^= array[j];
        if (x < 0) {
            fprintf(stderr, "NEG seed=%u xor=%d\n", seed, x);
            return 0;
        }
    }
    fprintf(stderr, "none found in [%u, +%lu)\n", first, count);
    return 1;
}
