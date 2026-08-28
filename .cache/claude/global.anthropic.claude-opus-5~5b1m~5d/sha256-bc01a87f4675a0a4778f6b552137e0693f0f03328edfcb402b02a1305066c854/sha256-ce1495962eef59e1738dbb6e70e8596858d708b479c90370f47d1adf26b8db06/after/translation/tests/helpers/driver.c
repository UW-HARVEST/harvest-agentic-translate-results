/* Out-of-process driver: dlopen a shared object (C or Rust), optionally arm the
 * malloc interposer, then call checkshift(). Everything the library prints plus
 * the returned value lands on stdout, so the two runs can be compared byte for
 * byte.
 *
 * argv[1] = path to the .so
 * argv[2] = malloc size to fail (optional; 0 or absent = never fail)
 * argv[3..6] = the four checkshift parameters (optional, default 1 2 3 4)
 */
#define _GNU_SOURCE  /* for RTLD_DEFAULT */
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>

typedef int (*checkshift_t)(int, int, int, int);
typedef void (*set_fail_t)(size_t);

int main(int argc, char **argv) {
    if (argc < 2) {
        fprintf(stderr, "usage: %s <lib.so> [fail_size] [p1 p2 p3 p4]\n", argv[0]);
        return 2;
    }

    void *h = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!h) {
        fprintf(stderr, "dlopen(%s) failed: %s\n", argv[1], dlerror());
        return 3;
    }

    checkshift_t checkshift = (checkshift_t)dlsym(h, "checkshift");
    if (!checkshift) {
        fprintf(stderr, "dlsym(checkshift) failed: %s\n", dlerror());
        return 4;
    }

    int p1 = 1, p2 = 2, p3 = 3, p4 = 4;
    if (argc >= 7) {
        p1 = (int)strtol(argv[3], NULL, 10);
        p2 = (int)strtol(argv[4], NULL, 10);
        p3 = (int)strtol(argv[5], NULL, 10);
        p4 = (int)strtol(argv[6], NULL, 10);
    }

    size_t fail_size = 0;
    if (argc >= 3) {
        fail_size = (size_t)strtoul(argv[2], NULL, 10);
    }

    set_fail_t set_fail = (set_fail_t)dlsym(RTLD_DEFAULT, "mf_set_fail_size");
    if (fail_size != 0) {
        if (!set_fail) {
            fprintf(stderr, "mf_set_fail_size not found (LD_PRELOAD missing?)\n");
            return 5;
        }
        /* Force stdout's buffer to be allocated before arming, so the interposer
         * cannot perturb stdio setup. */
        fflush(stdout);
        set_fail(fail_size);
    }

    int result = checkshift(p1, p2, p3, p4);

    if (set_fail) {
        set_fail(0);
    }

    printf("RESULT=%d\n", result);
    fflush(stdout);
    return 0;
}
