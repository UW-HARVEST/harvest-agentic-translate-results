/*
 * Differential test harness: dlopen()s the reference C libsodium.so and the
 * Rust cdylib side by side and compares outputs byte for byte.
 *
 * Build:
 *   gcc -O1 -o difftest difftest.c -ldl
 * Run:
 *   ./difftest <path/to/c/libsodium.so> <path/to/rust/libsodium.so>
 */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>
#include <unistd.h>
#include <sys/wait.h>

static void *hC, *hR;
static int   verbose;
static int   n_pass, n_fail, n_skip;
static char  failbuf[65536];
static size_t failbuf_len;

/* ------------------------------------------------------------------ */
/* deterministic PRNG for test inputs                                  */
/* ------------------------------------------------------------------ */
static uint64_t prng_s = 0x243F6A8885A308D3ULL;
static uint64_t nextr(void)
{
    prng_s ^= prng_s << 13;
    prng_s ^= prng_s >> 7;
    prng_s ^= prng_s << 17;
    return prng_s;
}
static void fillr(void *p, size_t n)
{
    unsigned char *q = p;
    for (size_t i = 0; i < n; i++) q[i] = (unsigned char) (nextr() >> 24);
}

/* ------------------------------------------------------------------ */
/* deterministic randombytes implementation injected into both libs     */
/* ------------------------------------------------------------------ */
typedef struct rb_impl {
    const char *(*implementation_name)(void);
    uint32_t (*random)(void);
    void (*stir)(void);
    uint32_t (*uniform)(const uint32_t upper_bound);
    void (*buf)(void *const buf, const size_t size);
    int (*close)(void);
} rb_impl;

static uint64_t det_s;
static void det_reset(void) { det_s = 0x9E3779B97F4A7C15ULL; }
static uint64_t det_next(void)
{
    det_s += 0x9E3779B97F4A7C15ULL;
    uint64_t z = det_s;
    z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9ULL;
    z = (z ^ (z >> 27)) * 0x94D049BB133111EBULL;
    return z ^ (z >> 31);
}
static const char *det_name(void) { return "difftest-det"; }
static uint32_t    det_random(void) { return (uint32_t) det_next(); }
static void        det_stir(void) {}
static void        det_buf(void *const b, const size_t size)
{
    unsigned char *p = b;
    for (size_t i = 0; i < size; i++) p[i] = (unsigned char) (det_next() >> 33);
}
static int     det_close(void) { return 0; }
static rb_impl det_impl = { det_name, det_random, det_stir, NULL, det_buf, det_close };

/* ------------------------------------------------------------------ */
static void *symC(const char *n) { return dlsym(hC, n); }
static void *symR(const char *n) { return dlsym(hR, n); }

static void note_fail(const char *fmt, ...)
{
    va_list ap;
    va_start(ap, fmt);
    if (failbuf_len < sizeof failbuf - 512) {
        failbuf_len += vsnprintf(failbuf + failbuf_len,
                                 sizeof failbuf - failbuf_len, fmt, ap);
    }
    va_end(ap);
}

static void hexdump(const char *label, const unsigned char *p, size_t n)
{
    if (n > 48) n = 48;
    fprintf(stderr, "      %s: ", label);
    for (size_t i = 0; i < n; i++) fprintf(stderr, "%02x", p[i]);
    fprintf(stderr, "\n");
}

#define OUTMAX 8192

typedef struct {
    unsigned char out[OUTMAX];
    long long     ret;
    unsigned long long extra;
} result;

static void report(const char *name, const result *a, const result *b, size_t outlen)
{
    if (a->ret == b->ret && a->extra == b->extra &&
        memcmp(a->out, b->out, outlen) == 0) {
        n_pass++;
        return;
    }
    n_fail++;
    fprintf(stderr, "FAIL %s\n", name);
    if (a->ret != b->ret) fprintf(stderr, "      ret: C=%lld R=%lld\n", a->ret, b->ret);
    if (a->extra != b->extra)
        fprintf(stderr, "      extra: C=%llu R=%llu\n", a->extra, b->extra);
    if (memcmp(a->out, b->out, outlen) != 0) {
        hexdump("C  ", a->out, outlen);
        hexdump("R  ", b->out, outlen);
    }
    note_fail("%s ", name);
}

/* Generic driver: fn(handle) -> result, run for both libs and compare. */
#define RUN2(name, outlen, body)                                          \
    do {                                                                  \
        result ra, rb;                                                    \
        void  *h;                                                         \
        int    missing = 0;                                               \
        memset(&ra, 0, sizeof ra);                                        \
        memset(&rb, 0, sizeof rb);                                        \
        for (int _i = 0; _i < 2; _i++) {                                  \
            result *R = _i ? &rb : &ra;                                   \
            h = _i ? hR : hC;                                             \
            (void) R;                                                     \
            (void) h;                                                     \
            det_reset();                                                  \
            body                                                          \
        }                                                                 \
        if (missing) { n_skip++; fprintf(stderr, "SKIP %s\n", name); }    \
        else report(name, &ra, &rb, outlen);                              \
    } while (0)

#define GET(type, var, symname)                     \
    type var = (type) dlsym(h, symname);            \
    if (!var) { missing = 1; break; }

/* ------------------------------------------------------------------ */
int main(int argc, char **argv)
{
    if (argc < 3) {
        fprintf(stderr, "usage: %s libC.so libR.so [seed]\n", argv[0]);
        return 2;
    }
    if (argc > 3) {
        prng_s = strtoull(argv[3], NULL, 0);
        if (prng_s == 0) prng_s = 1;
    }
    verbose = getenv("DTV") != NULL;
    hC = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!hC) { fprintf(stderr, "dlopen C: %s\n", dlerror()); return 2; }
    hR = dlopen(argv[2], RTLD_NOW | RTLD_LOCAL);
    if (!hR) { fprintf(stderr, "dlopen R: %s\n", dlerror()); return 2; }

    /* install deterministic RNG + init both libraries */
    for (int i = 0; i < 2; i++) {
        void *h = i ? hR : hC;
        int (*set_impl)(const rb_impl *) = dlsym(h, "randombytes_set_implementation");
        int (*init)(void)                = dlsym(h, "sodium_init");
        if (set_impl) set_impl(&det_impl);
        if (init) init();
        if (set_impl) set_impl(&det_impl);
    }

#include "difftest_cases.h"

#include "difftest_cases2.h"

    fprintf(stderr, "\n=== pass %d  fail %d  skip %d ===\n", n_pass, n_fail, n_skip);
    if (failbuf_len) fprintf(stderr, "failed: %s\n", failbuf);
    return n_fail ? 1 : 0;
}
