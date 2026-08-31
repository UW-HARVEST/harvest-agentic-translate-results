/* Differential harness: dlopen()s a libzstd and prints deterministic results.
 * Run against the C reference and the Rust translation, then diff the output. */
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dlfcn.h>

static void *H;

static void *sym(const char *n)
{
    void *s = dlsym(H, n);
    if (!s) { fprintf(stderr, "MISSING SYMBOL %s\n", n); exit(2); }
    return s;
}

static void *symq(const char *n) { return dlsym(H, n); }

/* deterministic pseudo-random data */
static unsigned long long rs = 88172645463325252ULL;
static unsigned char rnd(void)
{
    rs ^= rs << 13; rs ^= rs >> 7; rs ^= rs << 17;
    return (unsigned char)(rs >> 24);
}

static void hexdump(const char *tag, const unsigned char *p, size_t n)
{
    printf("%s len=%zu ", tag, n);
    for (size_t i = 0; i < n; i++) printf("%02x", p[i]);
    printf("\n");
}

int main(int argc, char **argv)
{
    if (argc < 2) { fprintf(stderr, "usage: %s <libzstd.so>\n", argv[0]); return 1; }
    H = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!H) { fprintf(stderr, "dlopen: %s\n", dlerror()); return 1; }

    /* ---- version / errors ---- */
    unsigned (*versionNumber)(void) = sym("ZSTD_versionNumber");
    const char *(*versionString)(void) = sym("ZSTD_versionString");
    unsigned (*isError)(size_t) = sym("ZSTD_isError");
    const char *(*getErrorName)(size_t) = sym("ZSTD_getErrorName");
    int (*getErrorCode)(size_t) = sym("ZSTD_getErrorCode");
    const char *(*getErrorString)(int) = sym("ZSTD_getErrorString");

    printf("versionNumber=%u\n", versionNumber());
    printf("versionString=%s\n", versionString());
    for (int c = -5; c <= 130; c++) {
        size_t code = (size_t)(0 - (long)c);
        printf("err %d: isError=%u name=%s code=%d str=%s\n",
               c, isError(code), getErrorName(code), getErrorCode(code),
               getErrorString(c));
    }
    for (size_t v = 0; v < 200; v += 37)
        printf("isError(%zu)=%u\n", v, isError(v));

    /* ---- xxhash ---- */
    unsigned (*xxhver)(void) = sym("ZSTD_XXH_versionNumber");
    unsigned (*xxh32)(const void *, size_t, unsigned) = sym("ZSTD_XXH32");
    unsigned long long (*xxh64)(const void *, size_t, unsigned long long) = sym("ZSTD_XXH64");
    void *(*x32new)(void) = sym("ZSTD_XXH32_createState");
    int (*x32free)(void *) = sym("ZSTD_XXH32_freeState");
    int (*x32reset)(void *, unsigned) = sym("ZSTD_XXH32_reset");
    int (*x32upd)(void *, const void *, size_t) = sym("ZSTD_XXH32_update");
    unsigned (*x32dig)(const void *) = sym("ZSTD_XXH32_digest");
    void *(*x64new)(void) = sym("ZSTD_XXH64_createState");
    int (*x64free)(void *) = sym("ZSTD_XXH64_freeState");
    int (*x64reset)(void *, unsigned long long) = sym("ZSTD_XXH64_reset");
    int (*x64upd)(void *, const void *, size_t) = sym("ZSTD_XXH64_update");
    unsigned long long (*x64dig)(const void *) = sym("ZSTD_XXH64_digest");
    void (*x32canon)(void *, unsigned) = sym("ZSTD_XXH32_canonicalFromHash");
    unsigned (*x32fromcanon)(const void *) = sym("ZSTD_XXH32_hashFromCanonical");
    void (*x64canon)(void *, unsigned long long) = sym("ZSTD_XXH64_canonicalFromHash");
    unsigned long long (*x64fromcanon)(const void *) = sym("ZSTD_XXH64_hashFromCanonical");

    printf("xxhversion=%u\n", xxhver());

    static unsigned char buf[70000];
    for (size_t i = 0; i < sizeof(buf); i++) buf[i] = rnd();

    size_t lens[] = {0,1,2,3,4,5,6,7,8,9,12,15,16,17,31,32,33,63,64,
                     65,127,128,129,255,256,1000,4096,65535,70000};
    for (unsigned li = 0; li < sizeof(lens)/sizeof(lens[0]); li++) {
        size_t L = lens[li];
        printf("xxh32 L=%zu s0=%u s7=%u\n", L, xxh32(buf, L, 0), xxh32(buf, L, 0x9e3779b1u));
        printf("xxh64 L=%zu s0=%llu s7=%llu\n", L, xxh64(buf, L, 0),
               xxh64(buf, L, 0x9e3779b185ebca87ULL));
    }

    /* streaming, with awkward chunk sizes */
    {
        void *s32 = x32new(); void *s64 = x64new();
        size_t chunks[] = {1,2,3,5,7,8,13,16,17,31,32,33,100,1000,4096};
        for (unsigned ci = 0; ci < sizeof(chunks)/sizeof(chunks[0]); ci++) {
            size_t ch = chunks[ci];
            x32reset(s32, 5); x64reset(s64, 5);
            for (size_t off = 0; off < 20000; off += ch) {
                size_t n = ch; if (off + n > 20000) n = 20000 - off;
                x32upd(s32, buf + off, n);
                x64upd(s64, buf + off, n);
            }
            printf("stream ch=%zu h32=%u h64=%llu\n", ch, x32dig(s32), x64dig(s64));
        }
        x32free(s32); x64free(s64);
    }
    {
        unsigned char c4[4], c8[8];
        x32canon(c4, 0x12345678u);
        hexdump("canon32", c4, 4);
        printf("fromcanon32=%u\n", x32fromcanon(c4));
        x64canon(c8, 0x0123456789abcdefULL);
        hexdump("canon64", c8, 8);
        printf("fromcanon64=%llu\n", x64fromcanon(c8));
    }

    /* ---- FSE/HUF error helpers (present in both builds) ---- */
    {
        unsigned (*fseIsError)(size_t) = symq("FSE_isError");
        const char *(*fseErrName)(size_t) = symq("FSE_getErrorName");
        unsigned (*fseVer)(void) = symq("FSE_versionNumber");
        unsigned (*hufIsError)(size_t) = symq("HUF_isError");
        const char *(*hufErrName)(size_t) = symq("HUF_getErrorName");
        if (fseVer) printf("FSE_versionNumber=%u\n", fseVer());
        if (fseIsError) for (int c = 0; c <= 130; c += 13)
            printf("FSE err %d: %u %s\n", c, fseIsError((size_t)(0-(long)c)),
                   fseErrName((size_t)(0-(long)c)));
        if (hufIsError) for (int c = 0; c <= 130; c += 13)
            printf("HUF err %d: %u %s\n", c, hufIsError((size_t)(0-(long)c)),
                   hufErrName((size_t)(0-(long)c)));
    }

    fflush(stdout);
    return 0;
}
