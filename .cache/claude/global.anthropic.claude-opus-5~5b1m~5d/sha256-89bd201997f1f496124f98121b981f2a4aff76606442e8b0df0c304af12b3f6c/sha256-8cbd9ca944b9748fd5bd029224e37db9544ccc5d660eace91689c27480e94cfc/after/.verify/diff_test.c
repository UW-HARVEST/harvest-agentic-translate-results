// Differential test: loads the C reference .so and the Rust .so side by side
// and compares every exported symbol, including raw struct bytes.
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <stdint.h>
#include <dlfcn.h>

typedef int (*operation_func)(int, int, int, int);
typedef struct { int value; double scaled; int rank; } Result;
typedef struct { Result data[10]; int count; } ResultArray;

typedef struct {
    void *h;
    int (*add_operation)(int,int,int,int);
    int (*multiply_operation)(int,int,int,int);
    int (*subtract_operation)(int,int,int,int);
    int (*modulo_operation)(int,int,int,int);
    int (*safe_double_to_int)(double);
    int (*compute_scaled_value)(int,double);
    int (*compare_results_in_array)(ResultArray*,int,int);
    void (*init_result_array)(ResultArray*,int*,int);
    int (*process_with_foreach)(ResultArray*,operation_func);
    int (*compute_weighted_sum)(ResultArray*);
    int (*arrayfunc)(int,int,int,int);
} Lib;

#define LOAD(l, n) do { \
    *(void**)(&(l)->n) = dlsym((l)->h, #n); \
    if (!(l)->n) { fprintf(stderr, "missing symbol %s: %s\n", #n, dlerror()); exit(2); } \
} while (0)

static void load(Lib *l, const char *path) {
    l->h = dlopen(path, RTLD_NOW | RTLD_LOCAL);
    if (!l->h) { fprintf(stderr, "dlopen %s: %s\n", path, dlerror()); exit(2); }
    LOAD(l, add_operation); LOAD(l, multiply_operation); LOAD(l, subtract_operation);
    LOAD(l, modulo_operation); LOAD(l, safe_double_to_int); LOAD(l, compute_scaled_value);
    LOAD(l, compare_results_in_array); LOAD(l, init_result_array);
    LOAD(l, process_with_foreach); LOAD(l, compute_weighted_sum); LOAD(l, arrayfunc);
}

static long fails = 0, checks = 0;
static void ck(int ok, const char *what, long long a, long long b, const char *ctx) {
    checks++;
    if (!ok) {
        if (fails < 25) printf("MISMATCH %s (%s): C=%lld RUST=%lld\n", what, ctx, a, b);
        fails++;
    }
}

/* harness-local ops, used for process_with_foreach so both libs see the same op */
static int h_add(int a,int b,int u1,int u2){(void)u1;(void)u2;return a+b;}
static int h_mul(int a,int b,int u1,int u2){(void)u1;(void)u2;return a*b;}
static int h_sub(int a,int b,int u1,int u2){(void)u1;(void)u2;return a-b;}
static int h_mod(int a,int b,int u1,int u2){(void)u1;(void)u2;if(b==0)return 0;return a%b;}

static uint64_t rs = 0x123456789abcdefULL;
static uint32_t rnd(void){ rs ^= rs<<13; rs ^= rs>>7; rs ^= rs<<17; return (uint32_t)(rs>>16); }
static int rndi(void){ return (int)rnd(); }

int main(void) {
    Lib c, r;
    load(&c, "./libc_ref.so");
    load(&r, "../translation/target/release/libtranslation.so");
    char ctx[128];

    /* ---- 1. the four operations ---- */
    int smalls[] = {0,1,-1,2,-2,3,7,-7,10,100,-100,32767,-32768,
                    2147483647,-2147483648,1073741824,-1073741824,46341,-46341};
    int ns = sizeof(smalls)/sizeof(smalls[0]);
    for (int i = 0; i < ns; i++) for (int j = 0; j < ns; j++) {
        int a = smalls[i], b = smalls[j];
        snprintf(ctx, sizeof ctx, "a=%d b=%d", a, b);
        ck(c.add_operation(a,b,5,6)      == r.add_operation(a,b,5,6),      "add", c.add_operation(a,b,5,6), r.add_operation(a,b,5,6), ctx);
        ck(c.multiply_operation(a,b,5,6) == r.multiply_operation(a,b,5,6), "mul", c.multiply_operation(a,b,5,6), r.multiply_operation(a,b,5,6), ctx);
        ck(c.subtract_operation(a,b,5,6) == r.subtract_operation(a,b,5,6), "sub", c.subtract_operation(a,b,5,6), r.subtract_operation(a,b,5,6), ctx);
        /* skip INT_MIN % -1: SIGFPE in the compiled C (hardware trap) */
        if (!(a == -2147483647-1 && b == -1))
            ck(c.modulo_operation(a,b,5,6) == r.modulo_operation(a,b,5,6), "mod", c.modulo_operation(a,b,5,6), r.modulo_operation(a,b,5,6), ctx);
    }
    for (long t = 0; t < 200000; t++) {
        int a = rndi(), b = rndi();
        ck(c.add_operation(a,b,0,0)==r.add_operation(a,b,0,0),"add.r",0,0,"rand");
        ck(c.multiply_operation(a,b,0,0)==r.multiply_operation(a,b,0,0),"mul.r",0,0,"rand");
        ck(c.subtract_operation(a,b,0,0)==r.subtract_operation(a,b,0,0),"sub.r",0,0,"rand");
        if (!(a == -2147483647-1 && b == -1))
            ck(c.modulo_operation(a,b,0,0)==r.modulo_operation(a,b,0,0),"mod.r",0,0,"rand");
    }

    /* ---- 2. safe_double_to_int ---- */
    double ds[] = {0.0,-0.0,0.5,-0.5,1.0,-1.0,1.9,-1.9,2.5,-2.5,
                   2147483646.0,2147483646.5,2147483647.0,2147483647.5,2147483648.0,
                   -2147483647.0,-2147483647.5,-2147483648.0,-2147483648.5,-2147483649.0,
                   1e300,-1e300,1e-300,
                   INFINITY,-INFINITY,NAN,-NAN,
                   4294967296.0,-4294967296.0,123456789.123,-987654321.987};
    for (unsigned i = 0; i < sizeof(ds)/sizeof(ds[0]); i++) {
        snprintf(ctx, sizeof ctx, "d=%.17g", ds[i]);
        ck(c.safe_double_to_int(ds[i]) == r.safe_double_to_int(ds[i]), "safe_double_to_int",
           c.safe_double_to_int(ds[i]), r.safe_double_to_int(ds[i]), ctx);
    }
    for (long t = 0; t < 200000; t++) {
        union { uint64_t u; double d; } u; u.u = ((uint64_t)rnd()<<32)|rnd();
        int rc = c.safe_double_to_int(u.d), rr = r.safe_double_to_int(u.d);
        snprintf(ctx, sizeof ctx, "bits=%016llx", (unsigned long long)u.u);
        ck(rc == rr, "safe_double_to_int.rand", rc, rr, ctx);
    }

    /* ---- 3. compute_scaled_value ---- */
    for (int i = 0; i < ns; i++) for (unsigned j = 0; j < sizeof(ds)/sizeof(ds[0]); j++) {
        int rc = c.compute_scaled_value(smalls[i], ds[j]);
        int rr = r.compute_scaled_value(smalls[i], ds[j]);
        snprintf(ctx, sizeof ctx, "base=%d scale=%.17g", smalls[i], ds[j]);
        ck(rc == rr, "compute_scaled_value", rc, rr, ctx);
    }
    for (long t = 0; t < 100000; t++) {
        int b = rndi();
        union { uint64_t u; double d; } u; u.u = ((uint64_t)rnd()<<32)|rnd();
        ck(c.compute_scaled_value(b,u.d) == r.compute_scaled_value(b,u.d), "compute_scaled_value.r",0,0,"rand");
    }

    /* ---- 4. init_result_array (byte-for-byte struct compare) ---- */
    for (long t = 0; t < 50000; t++) {
        int vals[16]; for (int i = 0; i < 16; i++) vals[i] = rndi();
        int count = (int)(rnd() % 14);            /* 0..13, incl. > 10 clamp */
        ResultArray ac, ar;
        memset(&ac, 0xA5, sizeof ac); memset(&ar, 0xA5, sizeof ar);
        c.init_result_array(&ac, vals, count);
        r.init_result_array(&ar, vals, count);
        snprintf(ctx, sizeof ctx, "count=%d", count);
        ck(memcmp(&ac, &ar, sizeof ac) == 0, "init_result_array bytes", 0, 0, ctx);
    }
    /* negative counts too */
    for (int count = -5; count < 0; count++) {
        int vals[16]; for (int i = 0; i < 16; i++) vals[i] = rndi();
        ResultArray ac, ar;
        memset(&ac, 0x5A, sizeof ac); memset(&ar, 0x5A, sizeof ar);
        c.init_result_array(&ac, vals, count);
        r.init_result_array(&ar, vals, count);
        snprintf(ctx, sizeof ctx, "count=%d", count);
        ck(memcmp(&ac, &ar, sizeof ac) == 0, "init_result_array neg bytes", 0, 0, ctx);
    }

    /* ---- 5. compare_results_in_array ---- */
    for (int count = 0; count <= 10; count++) {
        int vals[10]; for (int i = 0; i < 10; i++) vals[i] = i * 3 - 4;
        ResultArray ac, ar;
        memset(&ac, 0, sizeof ac); memset(&ar, 0, sizeof ar);
        c.init_result_array(&ac, vals, count);
        r.init_result_array(&ar, vals, count);
        for (int i = -3; i <= 12; i++) for (int j = -3; j <= 12; j++) {
            int rc = c.compare_results_in_array(&ac, i, j);
            int rr = r.compare_results_in_array(&ar, i, j);
            snprintf(ctx, sizeof ctx, "count=%d i=%d j=%d", count, i, j);
            ck(rc == rr, "compare_results_in_array", rc, rr, ctx);
        }
    }

    /* ---- 6. process_with_foreach (return value + struct bytes) ---- */
    operation_func ops[4] = { h_add, h_mul, h_sub, h_mod };
    for (long t = 0; t < 40000; t++) {
        int vals[10]; for (int i = 0; i < 10; i++) vals[i] = rndi() % 100000;
        int count = (int)(rnd() % 11);
        ResultArray ac, ar;
        memset(&ac, 0x3C, sizeof ac); memset(&ar, 0x3C, sizeof ar);
        c.init_result_array(&ac, vals, count);
        r.init_result_array(&ar, vals, count);
        for (int k = 0; k < 4; k++) {
            int rc = c.process_with_foreach(&ac, ops[k]);
            int rr = r.process_with_foreach(&ar, ops[k]);
            snprintf(ctx, sizeof ctx, "count=%d op=%d", count, k);
            ck(rc == rr, "process_with_foreach ret", rc, rr, ctx);
            ck(memcmp(&ac, &ar, sizeof ac) == 0, "process_with_foreach bytes", 0, 0, ctx);
        }
        /* 7. compute_weighted_sum on the mutated arrays */
        int wc = c.compute_weighted_sum(&ac), wr = r.compute_weighted_sum(&ar);
        snprintf(ctx, sizeof ctx, "count=%d", count);
        ck(wc == wr, "compute_weighted_sum", wc, wr, ctx);
        ck(memcmp(&ac, &ar, sizeof ac) == 0, "weighted_sum bytes", 0, 0, ctx);
    }
    /* huge values -> saturation paths */
    for (long t = 0; t < 20000; t++) {
        int vals[10]; for (int i = 0; i < 10; i++) vals[i] = rndi();
        ResultArray ac, ar;
        memset(&ac, 0, sizeof ac); memset(&ar, 0, sizeof ar);
        c.init_result_array(&ac, vals, 10);
        r.init_result_array(&ar, vals, 10);
        for (int k = 0; k < 4; k++) {
            ck(c.process_with_foreach(&ac, ops[k]) == r.process_with_foreach(&ar, ops[k]),
               "pwf big ret", 0, 0, "big");
            ck(memcmp(&ac, &ar, sizeof ac) == 0, "pwf big bytes", 0, 0, "big");
        }
        ck(c.compute_weighted_sum(&ac) == r.compute_weighted_sum(&ar), "cws big", 0, 0, "big");
    }

    /* ---- 8. arrayfunc: dense grid + edges + random ---- */
    int grid[] = {-1000,-100,-17,-5,-3,-2,-1,0,1,2,3,5,17,100,1000,
                  65535,-65536,2147483647,-2147483648,1073741823,-1073741824,
                  46341,-46341,123456,-123456,7,13,999999,-999999};
    int ng = sizeof(grid)/sizeof(grid[0]);
    for (int i = 0; i < ng; i++) for (int j = 0; j < ng; j++)
      for (int k = 0; k < ng; k++) {
        int m = grid[(i+j+k) % ng];
        int rc = c.arrayfunc(grid[i], grid[j], grid[k], m);
        int rr = r.arrayfunc(grid[i], grid[j], grid[k], m);
        snprintf(ctx, sizeof ctx, "%d,%d,%d,%d", grid[i], grid[j], grid[k], m);
        ck(rc == rr, "arrayfunc", rc, rr, ctx);
      }
    for (int a = -12; a <= 12; a++) for (int b = -12; b <= 12; b++)
      for (int d = -12; d <= 12; d++) for (int e = -12; e <= 12; e++) {
        int rc = c.arrayfunc(a,b,d,e), rr = r.arrayfunc(a,b,d,e);
        snprintf(ctx, sizeof ctx, "%d,%d,%d,%d", a,b,d,e);
        ck(rc == rr, "arrayfunc small", rc, rr, ctx);
      }
    for (long t = 0; t < 500000; t++) {
        int a = rndi(), b = rndi(), d = rndi(), e = rndi();
        int rc = c.arrayfunc(a,b,d,e), rr = r.arrayfunc(a,b,d,e);
        snprintf(ctx, sizeof ctx, "%d,%d,%d,%d", a,b,d,e);
        ck(rc == rr, "arrayfunc rand", rc, rr, ctx);
    }

    printf("checks=%ld fails=%ld\n", checks, fails);
    printf(fails ? "*** FAIL ***\n" : "ALL IDENTICAL\n");
    return fails ? 1 : 0;
}
