/* Out-of-process ops runner for the long/liblong differential harness.
 *
 * usage: runner <so-path> <op> [<op> ...]
 *
 * ops:
 *   fill:zero              array[i] = 0
 *   fill:const:V           array[i] = V            (V parsed as long, cast to int)
 *   fill:seq:BASE          array[i] = BASE + i     (wrapping int arithmetic)
 *   fill:rand:SEED         array[i] = splitmix32(SEED) stream, full int range
 *   fill:randnn:SEED       same stream masked to 0..0x7fffffff (rand() shape)
 *   fill:sparse:IDX:V      all zero except array[IDX] = V
 *   pxo:K                  call perform_expensive_operations() K times
 *   exec:SEED              call long_exec(SEED)
 *   dump:PATH              write the 1 MiB array to PATH
 *
 * Everything printed by the library (long_exec's printf) goes to stdout
 * untouched, and stdout is flushed before each dump so ordering is stable.
 *
 * The `fill:rand`/`fill:randnn` streams are byte-identical to the ones used by
 * translation/tests/differential.rs, so in-process and out-of-process runs
 * exercise the same inputs.
 */
#include <dlfcn.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define ARRAY_SIZE (256 * 1024)

static uint64_t sm_state;

static uint32_t splitmix32(void) {
    sm_state += 0x9E3779B97F4A7C15ULL;
    uint64_t z = sm_state;
    z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9ULL;
    z = (z ^ (z >> 27)) * 0x94D049BB133111EBULL;
    z = z ^ (z >> 31);
    return (uint32_t)(z >> 32);
}

int main(int argc, char **argv) {
    if (argc < 3) {
        fprintf(stderr, "usage: %s <so> <op> [<op>...]\n", argv[0]);
        return 2;
    }
    void *h = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!h) {
        fprintf(stderr, "dlopen: %s\n", dlerror());
        return 3;
    }
    void (*long_exec)(unsigned int) = (void (*)(unsigned int))dlsym(h, "long_exec");
    void (*pxo)(void) = (void (*)(void))dlsym(h, "perform_expensive_operations");
    int *array = (int *)dlsym(h, "array");
    if (!long_exec || !pxo || !array) {
        fprintf(stderr, "dlsym failed\n");
        return 4;
    }

    for (int a = 2; a < argc; a++) {
        char *op = argv[a];
        if (strcmp(op, "fill:zero") == 0) {
            memset(array, 0, sizeof(int) * ARRAY_SIZE);
        } else if (strncmp(op, "fill:const:", 11) == 0) {
            int v = (int)strtol(op + 11, NULL, 10);
            for (size_t i = 0; i < ARRAY_SIZE; i++) array[i] = v;
        } else if (strncmp(op, "fill:seq:", 9) == 0) {
            int base = (int)strtol(op + 9, NULL, 10);
            for (size_t i = 0; i < ARRAY_SIZE; i++)
                array[i] = (int)((unsigned int)base + (unsigned int)i);
        } else if (strncmp(op, "fill:randnn:", 12) == 0) {
            sm_state = strtoull(op + 12, NULL, 10);
            for (size_t i = 0; i < ARRAY_SIZE; i++)
                array[i] = (int)(splitmix32() & 0x7fffffffu);
        } else if (strncmp(op, "fill:rand:", 10) == 0) {
            sm_state = strtoull(op + 10, NULL, 10);
            for (size_t i = 0; i < ARRAY_SIZE; i++) array[i] = (int)splitmix32();
        } else if (strncmp(op, "fill:sparse:", 12) == 0) {
            char *rest = op + 12;
            char *colon = strchr(rest, ':');
            if (!colon) { fprintf(stderr, "bad op %s\n", op); return 2; }
            *colon = 0;
            size_t idx = (size_t)strtoull(rest, NULL, 10);
            int v = (int)strtol(colon + 1, NULL, 10);
            memset(array, 0, sizeof(int) * ARRAY_SIZE);
            if (idx < ARRAY_SIZE) array[idx] = v;
        } else if (strncmp(op, "pxo:", 4) == 0) {
            long k = strtol(op + 4, NULL, 10);
            for (long i = 0; i < k; i++) pxo();
        } else if (strncmp(op, "exec:", 5) == 0) {
            unsigned int seed = (unsigned int)strtoul(op + 5, NULL, 10);
            long_exec(seed);
        } else if (strncmp(op, "fill:libcrand:", 14) == 0) {
            /* Exactly the fill long_exec performs: srand(seed) then
             * ARRAY_SIZE calls to rand().  Lets `fill:libcrand:S pxo:2000` be
             * compared against `exec:S`, i.e. the naive nested loop against
             * whatever strategy the library uses inside long_exec. */
            unsigned int seed = (unsigned int)strtoul(op + 14, NULL, 10);
            srand(seed);
            for (size_t i = 0; i < ARRAY_SIZE; i++) array[i] = rand();
        } else if (strcmp(op, "xor") == 0) {
            int x = 0;
            for (size_t i = 0; i < ARRAY_SIZE; i++) x ^= array[i];
            printf("%d\n", x);
        } else if (strcmp(op, "hash") == 0) {
            /* order-sensitive 64-bit FNV-1a over the whole array */
            uint64_t h = 1469598103934665603ULL;
            for (size_t i = 0; i < ARRAY_SIZE; i++) {
                uint32_t v = (uint32_t)array[i];
                for (int b = 0; b < 4; b++) {
                    h ^= (uint8_t)(v >> (8 * b));
                    h *= 1099511628211ULL;
                }
            }
            printf("%016llx\n", (unsigned long long)h);
        } else if (strncmp(op, "dump:", 5) == 0) {
            fflush(stdout);
            FILE *f = fopen(op + 5, "wb");
            if (!f) { perror("fopen"); return 5; }
            if (fwrite(array, sizeof(int), ARRAY_SIZE, f) != ARRAY_SIZE) {
                perror("fwrite");
                return 6;
            }
            fclose(f);
        } else {
            fprintf(stderr, "unknown op: %s\n", op);
            return 2;
        }
    }
    fflush(stdout);
    return 0;
}
