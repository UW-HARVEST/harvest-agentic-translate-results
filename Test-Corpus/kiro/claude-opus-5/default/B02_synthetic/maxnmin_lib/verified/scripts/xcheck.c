// Independent cross-check driver, deliberately written in C and using dlopen
// directly, so it shares NO code with the Rust test harness. If the Rust harness
// itself were broken (e.g. accidentally loading the C library twice), everything
// would pass vacuously; this driver rules that out.
//
// Build & run:
//   gcc -O2 -o /tmp/xcheck translation/scripts/xcheck.c -ldl
//   /tmp/xcheck c_src/build/lib*.so            > /tmp/c.out
//   /tmp/xcheck translation/target/release/libmaxnmin_lib.so > /tmp/r.out
//   cmp /tmp/c.out /tmp/r.out
//
// This file is a test utility for the Rust crate; c_src/ is left untouched.

#include <dlfcn.h>
#include <stdio.h>
#include <stdint.h>
#include <string.h>

typedef int (*fn_maxnmin)(int, int, int, int);
typedef int (*fn_add_node)(int, int, const char *, double);
typedef void *(*fn_find)(int);
typedef int (*fn_children)(int);
typedef double (*fn_subtree)(int);
typedef int (*fn_process)(char *);
typedef int (*fn_d2i)(double);

/* xorshift64* so both runs walk the identical input sequence */
static uint64_t st = 0x123456789ABCDEF;
static uint64_t nx(void) {
    st ^= st >> 12; st ^= st << 25; st ^= st >> 27;
    return st * 0x2545F4914F6CDD1DULL;
}

int main(int argc, char **argv) {
    if (argc < 2) { fprintf(stderr, "usage: %s <lib.so>\n", argv[0]); return 2; }
    void *h = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!h) { fprintf(stderr, "dlopen: %s\n", dlerror()); return 2; }

    fn_maxnmin  maxnmin  = (fn_maxnmin)  dlsym(h, "maxnmin");
    fn_add_node add_node = (fn_add_node) dlsym(h, "add_node");
    fn_find     find     = (fn_find)     dlsym(h, "find_node_by_id");
    fn_children children = (fn_children) dlsym(h, "get_children_count");
    fn_subtree  subtree  = (fn_subtree)  dlsym(h, "calculate_subtree_sum");
    fn_process  process  = (fn_process)  dlsym(h, "process_string");
    fn_d2i      d2i      = (fn_d2i)      dlsym(h, "safe_double_to_int");
    if (!maxnmin || !add_node || !find || !children || !subtree || !process || !d2i) {
        fprintf(stderr, "missing symbol\n"); return 2;
    }

    /* 1. exhaustive small maxnmin sweep, printed as a rolling digest */
    uint64_t digest = 1469598103934665603ULL;
    long long acc = 0;
    for (int i = -12; i <= 12; i++)
      for (int j = -12; j <= 12; j++)
        for (int k = -12; k <= 12; k++)
          for (int l = -12; l <= 12; l++) {
              int r = maxnmin(i, j, k, l);
              acc += r;
              digest = (digest ^ (uint32_t)r) * 1099511628211ULL;
          }
    printf("maxnmin_sweep acc=%lld digest=%016llx\n",
           acc, (unsigned long long)digest);

    /* 2. randomized maxnmin over the whole int range */
    st = 0xFEEDFACECAFEBEEFULL;
    acc = 0; digest = 1469598103934665603ULL;
    for (int t = 0; t < 200000; t++) {
        int a = (int)(nx() >> 32), b = (int)(nx() >> 32);
        int c = (int)(nx() >> 32), d = (int)(nx() >> 32);
        int r = maxnmin(a, b, c, d);
        acc += r;
        digest = (digest ^ (uint32_t)r) * 1099511628211ULL;
    }
    printf("maxnmin_random acc=%lld digest=%016llx\n",
           acc, (unsigned long long)digest);

    /* 3. safe_double_to_int over raw bit patterns */
    st = 0x0BADC0DE12345678ULL;
    acc = 0;
    for (int t = 0; t < 200000; t++) {
        uint64_t bits = nx();
        double dv; memcpy(&dv, &bits, 8);
        acc += d2i(dv);
    }
    printf("d2i acc=%lld\n", acc);

    /* 4. low-level pipeline on a freshly-built forest (this process only ever
     *    loads ONE library, so the state is pristine until maxnmin ran above;
     *    maxnmin left node_count == 6, which is part of what we compare) */
    char nm[80];
    for (int i = 0; i < 60; i++) {
        int id = (int)(nx() % 40) - 5;
        int pid = (int)(nx() % 40) - 5;
        int len = (int)(nx() % 60);
        for (int q = 0; q < len; q++) nm[q] = (char)(1 + nx() % 255);
        nm[len] = 0;
        double v = (double)(int64_t)(nx() % 2000000) / 1000.0;
        printf("add %d\n", add_node(id, pid, nm, v));
    }
    for (int id = -6; id <= 40; id++) {
        void *p = find(id);
        printf("find %d %d\n", id, p != NULL);
        printf("children %d %d\n", id, children(id));
    }
    /* subtree sums: the 60 random nodes above may have introduced a parent/child
     * cycle, which the C recurses on forever. Reset to maxnmin's known-acyclic
     * six-node tree first, then compare the sums bit-for-bit. */
    printf("reset %d\n", maxnmin(0, 0, 1, 0));
    for (int id = 1; id <= 6; id++) {
        double s = subtree(id);
        uint64_t b; memcpy(&b, &s, 8);
        printf("subtree %d %016llx d2i=%d\n", id, (unsigned long long)b, d2i(s));
    }
    /* 5. process_string over the stored names */
    for (int id = 1; id <= 6; id++) {
        char *p = (char *)find(id);
        if (p) printf("name %d %d\n", id, process(p + 8)); /* name is at offset 8 */
    }
    dlclose(h);
    return 0;
}
