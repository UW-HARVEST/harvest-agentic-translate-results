/* Allocator-call parity driver.
 *
 * Regression guard for a real divergence found during verification: LLVM
 * recognises `malloc`/`free` by name and had promoted the 12-byte `ComputeState`
 * block to registers, deleting the allocation from the Rust `.so` entirely. That
 * silently removed `checkshift`'s allocation-failure branch (see
 * malloc_fail_driver.c) and made the allocation invisible to any interposer.
 *
 * This driver asserts that one `checkshift` call performs exactly the same number
 * of malloc(12) and free calls in both libraries.
 *
 * usage: alloc_parity_driver <c.so> <rust.so>
 * exit:  0 = parity, 1 = divergence, 2 = harness problem
 *
 * Not part of the library under test; lives under translation/tests/aux/.
 */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

typedef int (*checkshift_t)(int, int, int, int);

static int *count_on;
static int *malloc12;
static int *frees;

static void measure(checkshift_t fn, int calls, int *out_malloc, int *out_free) {
    fflush(NULL);
    int saved = dup(1);
    int devnull = open("/dev/null", O_WRONLY);
    if (saved < 0 || devnull < 0) {
        fprintf(stderr, "driver: cannot redirect stdout\n");
        exit(2);
    }
    dup2(devnull, 1);

    *malloc12 = 0;
    *frees = 0;
    *count_on = 1;
    for (int i = 0; i < calls; i++) {
        fn(i, i + 1, i + 2, i + 3);
    }
    *count_on = 0;

    fflush(NULL);
    dup2(saved, 1);
    close(saved);
    close(devnull);

    *out_malloc = *malloc12;
    *out_free = *frees;
}

int main(int argc, char **argv) {
    if (argc != 3) {
        fprintf(stderr, "usage: %s <c.so> <rust.so>\n", argv[0]);
        return 2;
    }
    count_on = (int *)dlsym(RTLD_DEFAULT, "checkshift_count_on");
    malloc12 = (int *)dlsym(RTLD_DEFAULT, "checkshift_malloc12_count");
    frees = (int *)dlsym(RTLD_DEFAULT, "checkshift_free_count");
    if (count_on == NULL || malloc12 == NULL || frees == NULL) {
        fprintf(stderr, "driver: malloc shim is not LD_PRELOADed\n");
        return 2;
    }

    void *hc = dlopen(argv[1], RTLD_NOW);
    void *hr = dlopen(argv[2], RTLD_NOW);
    if (hc == NULL || hr == NULL) {
        fprintf(stderr, "driver: dlopen failed: %s\n", dlerror());
        return 2;
    }
    checkshift_t fc = (checkshift_t)dlsym(hc, "checkshift");
    checkshift_t fr = (checkshift_t)dlsym(hr, "checkshift");
    if (fc == NULL || fr == NULL) {
        fprintf(stderr, "driver: dlsym(checkshift) failed\n");
        return 2;
    }

    /* Warm both up outside the measurement window. */
    int m, f;
    measure(fc, 2, &m, &f);
    measure(fr, 2, &m, &f);

    const int N = 25;
    int cm, cf, rm, rf;
    measure(fc, N, &cm, &cf);
    measure(fr, N, &rm, &rf);

    int rc = 0;
    if (cm != N) {
        fprintf(stderr, "C made %d malloc(12) calls for %d checkshift calls (expected %d)\n",
                cm, N, N);
        rc = 2;
    }
    if (rm != cm) {
        fprintf(stderr,
                "ALLOCATOR DIVERGENCE: %d checkshift calls -> C made %d malloc(12) calls, "
                "Rust made %d\n",
                N, cm, rm);
        rc = 1;
    }
    if (rf != cf) {
        fprintf(stderr,
                "ALLOCATOR DIVERGENCE: %d checkshift calls -> C made %d free calls, "
                "Rust made %d\n",
                N, cf, rf);
        rc = 1;
    }
    if (rc == 0) {
        fprintf(stderr, "allocator parity ok: %d calls -> %d malloc(12), %d free on both sides\n",
                N, cm, cf);
    }
    return rc;
}
