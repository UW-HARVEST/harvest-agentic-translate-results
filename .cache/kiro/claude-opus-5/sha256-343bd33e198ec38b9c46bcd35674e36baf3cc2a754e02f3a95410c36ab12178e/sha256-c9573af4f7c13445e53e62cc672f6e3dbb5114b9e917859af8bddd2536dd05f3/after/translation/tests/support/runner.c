/*
 * Runner for the Phase C allocation-failure rows (E5, E6, E7).
 *
 * Usage: runner <lib.so> <iterations> <seed> <mode> <threshold> <fail_at>
 *
 * dlopen()s the given shared object, resolves "gotomach", arms the malloc
 * interposer so that the <fail_at>'th malloc returns NULL (0 => never), calls
 * gotomach once, and prints:
 *
 *     RESULT=<int> MALLOCS=<long>
 *
 * The same runner is used for the C .so and the Rust .so, so the printed line
 * must be identical for the two libraries. MALLOCS also proves the two issue
 * the same number of allocations in the same order.
 *
 * Built into translation/target/phase_c/ at test time only.
 */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>

typedef int (*goto_fn)(int, int, int, int);

extern void fm_arm(long);
extern long fm_disarm(void);

int main(int argc, char **argv) {
    if (argc != 7) {
        fprintf(stderr, "usage: %s <lib.so> <iterations> <seed> <mode> <threshold> <fail_at>\n",
                argv[0]);
        return 2;
    }

    void *h = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!h) {
        fprintf(stderr, "dlopen(%s) failed: %s\n", argv[1], dlerror());
        return 3;
    }

    goto_fn f = (goto_fn)dlsym(h, "gotomach");
    if (!f) {
        fprintf(stderr, "dlsym(gotomach) failed: %s\n", dlerror());
        return 4;
    }

    int iterations = atoi(argv[2]);
    int seed = atoi(argv[3]);
    int mode = atoi(argv[4]);
    int threshold = atoi(argv[5]);
    long fail_at = atol(argv[6]);

    /*
     * Warm-up (unarmed): forces the stdio buffer for stdout to be allocated and
     * any lazy PLT resolution to happen, so the armed call only counts the
     * allocations gotomach itself performs. gotomach is stateless, so this
     * cannot influence the measured call.
     */
    (void)f(1, 1, 0, 0);
    fflush(stdout);
    fflush(stderr);

    fm_arm(fail_at);
    int r = f(iterations, seed, mode, threshold);
    long n = fm_disarm();

    fflush(stdout);
    printf("RESULT=%d MALLOCS=%ld\n", r, n);
    fflush(stdout);
    return 0;
}
