/*
 * Phase C out-of-process harness.
 *
 * dlopen()s either libdriver.so (C build) or libdriver.so (Rust build) and calls
 * the exported symbols. Used for the rows that cannot be tested in-process:
 *   - assert(string != NULL) -> SIGABRT (kills the process)
 *   - malloc / realloc / strdup returning NULL (via the failalloc.so LD_PRELOAD)
 *   - detecting reads past the NUL terminator (guard page)
 *
 * Output is written to stdout in a stable, byte-comparable form so the Rust test
 * can diff the C harness run against the Rust harness run.
 */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <unistd.h>

typedef const char *(*drop_fn)(const char *);
typedef char *(*filter_fn)(const char *, bool);

/* Provided by failalloc.so when LD_PRELOADed; absent otherwise. */
static void (*arm_fn)(void);
static void (*disarm_fn)(void);

static void arm(void) { if (arm_fn) arm_fn(); }
static void disarm(void) { if (disarm_fn) disarm_fn(); }

static void die(const char *msg) {
    fprintf(stderr, "harness: %s: %s\n", msg, dlerror());
    exit(97);
}

/* "41ff00" -> bytes. Returns length; buf must be big enough. */
static size_t unhex(const char *hex, unsigned char *buf, size_t cap) {
    size_t n = strlen(hex) / 2;
    if (n > cap) { fprintf(stderr, "harness: input too long\n"); exit(96); }
    for (size_t i = 0; i < n; i++) {
        unsigned v;
        if (sscanf(hex + 2 * i, "%2x", &v) != 1) {
            fprintf(stderr, "harness: bad hex\n");
            exit(96);
        }
        buf[i] = (unsigned char)v;
    }
    return n;
}

static void print_hex(const unsigned char *p, size_t n) {
    for (size_t i = 0; i < n; i++) printf("%02x", p[i]);
}

int main(int argc, char **argv) {
    if (argc < 3) {
        fprintf(stderr, "usage: %s <so> <cmd> [args...]\n", argv[0]);
        return 95;
    }
    const char *so = argv[1];
    const char *cmd = argv[2];

    arm_fn = (void (*)(void))dlsym(RTLD_DEFAULT, "failalloc_arm");
    disarm_fn = (void (*)(void))dlsym(RTLD_DEFAULT, "failalloc_disarm");
    dlerror();

    void *h = dlopen(so, RTLD_NOW | RTLD_LOCAL);
    if (!h) die("dlopen");
    drop_fn w_drop = (drop_fn)dlsym(h, "w_utf8_drop");
    if (!w_drop) die("dlsym w_utf8_drop");
    filter_fn w_filter = (filter_fn)dlsym(h, "w_utf8_filter");
    if (!w_filter) die("dlsym w_utf8_filter");

    if (strcmp(cmd, "nullptr_drop") == 0) {
        printf("before\n");
        fflush(stdout);
        const char *r = w_drop(NULL);
        printf("returned %p\n", (void *)r);
        fflush(stdout);
        return 0;
    }

    if (strcmp(cmd, "nullptr_filter") == 0) {
        unsigned flag = (argc > 3) ? (unsigned)strtoul(argv[3], NULL, 10) : 0u;
        printf("before\n");
        fflush(stdout);
        char *r = w_filter(NULL, (bool)flag);
        printf("returned %p\n", (void *)r);
        fflush(stdout);
        return 0;
    }

    if (strcmp(cmd, "drop") == 0) {
        if (argc < 4) return 95;
        unsigned char buf[65536];
        size_t n = unhex(argv[3], buf, sizeof buf - 1);
        buf[n] = 0;
        arm();
        const char *r = w_drop((const char *)buf);
        disarm();
        printf("offset %ld\n", (long)(r - (const char *)buf));
        return 0;
    }

    if (strcmp(cmd, "filter") == 0) {
        if (argc < 5) return 95;
        unsigned char buf[65536];
        size_t n = unhex(argv[3], buf, sizeof buf - 1);
        buf[n] = 0;
        unsigned flag = (unsigned)strtoul(argv[4], NULL, 10);
        arm();
        char *r = w_filter((const char *)buf, (bool)flag);
        disarm();
        if (r == NULL) {
            printf("NULL\n");
        } else {
            printf("OK ");
            print_hex((const unsigned char *)r, strlen(r));
            printf("\n");
        }
        return 0;
    }

    /* Place the NUL-terminated string flush against the end of a mapped page,
     * with the following page unmapped, so any read past the terminator
     * segfaults. Detects over-reads in the valid_N continuation-byte checks. */
    if (strcmp(cmd, "guard") == 0) {
        if (argc < 4) return 95;
        unsigned char tmp[4096];
        size_t n = unhex(argv[3], tmp, sizeof tmp);
        long ps = sysconf(_SC_PAGESIZE);
        unsigned char *region = mmap(NULL, (size_t)ps * 2, PROT_READ | PROT_WRITE,
                                     MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
        if (region == MAP_FAILED) { perror("mmap"); return 94; }
        /* Make the second page inaccessible. */
        if (mprotect(region + ps, (size_t)ps, PROT_NONE) != 0) { perror("mprotect"); return 94; }
        /* String occupies [end-n-1, end): n bytes plus the NUL as the last byte
         * of the accessible page. */
        unsigned char *s = region + ps - n - 1;
        memcpy(s, tmp, n);
        s[n] = 0;

        const char *r = w_drop((const char *)s);
        printf("offset %ld\n", (long)(r - (const char *)s));
        fflush(stdout);

        for (unsigned flag = 0; flag <= 1; flag++) {
            char *f = w_filter((const char *)s, (bool)flag);
            if (f == NULL) {
                printf("NULL\n");
            } else {
                printf("OK ");
                print_hex((const unsigned char *)f, strlen(f));
                printf("\n");
                free(f);
            }
        }
        fflush(stdout);
        return 0;
    }

    fprintf(stderr, "harness: unknown cmd %s\n", cmd);
    return 95;
}
