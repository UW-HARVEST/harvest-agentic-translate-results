/* Differential driver: dlopen a jansson .so and exercise the real-number
 * path (json_real + json_dumps at various precisions) and dtoa_r directly.
 * Prints deterministic output to stdout for byte-comparison. */
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

typedef void *json_t_ptr;
typedef json_t_ptr (*json_real_fn)(double);
typedef char *(*json_dumps_fn)(const json_t_ptr, size_t);
typedef void (*json_decref_like)(json_t_ptr); /* not exported; use json_delete */
typedef void (*json_delete_fn)(json_t_ptr);
typedef char *(*dtoa_r_fn)(double, int, int, int *, int *, char **, char *, size_t);
typedef json_t_ptr (*json_loads_fn)(const char *, size_t, void *);

#define JSON_ENCODE_ANY 0x200
#define JSON_REAL_PRECISION(n) (((n) & 0x1F) << 11)

int main(int argc, char **argv) {
    if (argc < 2) { fprintf(stderr, "usage: %s <so>\n", argv[0]); return 2; }
    void *h = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!h) { fprintf(stderr, "dlopen: %s\n", dlerror()); return 2; }

    json_real_fn json_real = (json_real_fn)dlsym(h, "json_real");
    json_dumps_fn json_dumps = (json_dumps_fn)dlsym(h, "json_dumps");
    json_delete_fn json_delete = (json_delete_fn)dlsym(h, "json_delete");
    dtoa_r_fn dtoa_r = (dtoa_r_fn)dlsym(h, "dtoa_r");
    json_loads_fn json_loads = (json_loads_fn)dlsym(h, "json_loads");

    if (!json_real || !json_dumps || !json_delete || !dtoa_r || !json_loads) {
        fprintf(stderr, "missing symbols\n"); return 2;
    }

    /* A broad set of double bit patterns. */
    static const uint64_t pats[] = {
        0x0000000000000000ULL, 0x8000000000000000ULL,
        0x3ff0000000000000ULL, 0xbff0000000000000ULL,
        0x4000000000000000ULL, 0x3fb999999999999aULL,
        0x400921fb54442d18ULL, 0x0000000000000001ULL,
        0x000fffffffffffffULL, 0x0010000000000000ULL,
        0x7fefffffffffffffULL, 0x4340000000000000ULL,
        0xc340000000000000ULL, 0x3eb0c6f7a0b5ed8dULL,
        0x43e158e460913d00ULL, 0x3f847ae147ae147bULL,
        0x4024000000000000ULL, 0x4059000000000000ULL,
        0x3f50624dd2f1a9fcULL, 0x44b52d02c7e14af6ULL,
        0x0000000000000002ULL, 0x0004000000000000ULL,
        0x7fe1ccf385ebc8a0ULL, 0x1e0000000000000ULL,
    };
    int n = (int)(sizeof(pats)/sizeof(pats[0]));

    /* deterministic pseudo-random doubles */
    uint64_t seed = 0x123456789abcdef0ULL;
    for (int r = 0; r < 4000; r++) {
        seed = seed * 6364136223846793005ULL + 1442695040888963407ULL;
        uint64_t bits = seed;
        double d;
        memcpy(&d, &bits, 8);
        if (d != d) continue; /* skip NaN */
        if (d > 1e300 || d < -1e300) { /* keep some big finite */ }
        json_t_ptr j = json_real(d);
        if (!j) continue;
        for (int prec = 0; prec <= 17; prec++) {
            size_t flags = JSON_ENCODE_ANY | JSON_REAL_PRECISION(prec);
            char *s = json_dumps(j, flags);
            printf("R %d %016llx %s\n", prec, (unsigned long long)bits, s ? s : "(null)");
            free(s);
        }
        json_delete(j);
    }

    for (int i = 0; i < n; i++) {
        double d; uint64_t bits = pats[i];
        memcpy(&d, &bits, 8);
        json_t_ptr j = json_real(d);
        for (int prec = 0; prec <= 17; prec++) {
            size_t flags = JSON_ENCODE_ANY | JSON_REAL_PRECISION(prec);
            char *s = j ? json_dumps(j, flags) : NULL;
            printf("P %d %016llx %s\n", prec, (unsigned long long)bits, s ? s : "(null)");
            free(s);
        }
        if (j) json_delete(j);

        /* dtoa_r directly across modes/ndigits */
        for (int mode = 0; mode <= 5; mode++) {
            for (int nd = 0; nd <= 20; nd++) {
                char buf[64]; int decpt = 0, sign = 0; char *rve = NULL;
                char *rv = dtoa_r(d, mode, nd, &decpt, &sign, &rve, buf, sizeof(buf));
                printf("D m%d n%d %016llx dp=%d sg=%d [%s]\n",
                       mode, nd, (unsigned long long)bits, decpt, sign,
                       rv ? rv : "(null)");
            }
        }
    }

    return 0;
}
