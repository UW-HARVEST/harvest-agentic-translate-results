/* Randomised differential fuzz: random inputs, random compression settings and
 * random corruption. Prints a digest per case so the two libraries can be
 * diffed. Deterministic (fixed seed) so both runs see identical cases. */
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dlfcn.h>

#define REF_PATH "/tmp/zref/libzstd.so"
static void *R, *T;
static void *rsym(const char *n)
{ void *s = dlsym(R, n); if (!s) { fprintf(stderr, "MISSING REF %s\n", n); exit(2); } return s; }
static void *tsym(const char *n)
{ void *s = dlsym(T, n); if (!s) { fprintf(stderr, "MISSING TEST %s\n", n); exit(3); } return s; }

static unsigned long long st;
static unsigned rnd(void) { st ^= st << 13; st ^= st >> 7; st ^= st << 17; return (unsigned)(st >> 24); }

typedef struct { const void *src; size_t size; size_t pos; } inBuf;
typedef struct { void *dst; size_t size; size_t pos; } outBuf;

#define MAXSRC (300 * 1024)

int main(int argc, char **argv)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    if (argc < 2) return 1;
    R = dlopen(REF_PATH, RTLD_NOW | RTLD_LOCAL);
    T = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!R || !T) { fprintf(stderr, "dlopen: %s\n", dlerror()); return 1; }

    size_t (*r_compressBound)(size_t) = rsym("ZSTD_compressBound");
    size_t (*r_isError)(size_t) = rsym("ZSTD_isError");
    void *(*r_createCCtx)(void) = rsym("ZSTD_createCCtx");
    size_t (*r_freeCCtx)(void *) = rsym("ZSTD_freeCCtx");
    size_t (*r_CCtx_setParameter)(void *, int, int) = rsym("ZSTD_CCtx_setParameter");
    size_t (*r_compress2)(void *, void *, size_t, const void *, size_t) = rsym("ZSTD_compress2");

    size_t (*t_decompress)(void *, size_t, const void *, size_t) = tsym("ZSTD_decompress");
    size_t (*t_isError)(size_t) = tsym("ZSTD_isError");
    int (*t_getErrorCode)(size_t) = tsym("ZSTD_getErrorCode");
    unsigned long long (*t_fcs)(const void *, size_t) = tsym("ZSTD_getFrameContentSize");
    size_t (*t_ffcs)(const void *, size_t) = tsym("ZSTD_findFrameCompressedSize");
    void *(*t_createDCtx)(void) = tsym("ZSTD_createDCtx");
    size_t (*t_freeDCtx)(void *) = tsym("ZSTD_freeDCtx");
    size_t (*t_initDStream)(void *) = tsym("ZSTD_initDStream");
    size_t (*t_decompressStream)(void *, void *, void *) = tsym("ZSTD_decompressStream");
    size_t (*t_DCtx_setParameter)(void *, int, int) = tsym("ZSTD_DCtx_setParameter");
    size_t (*t_DCtx_reset)(void *, int) = tsym("ZSTD_DCtx_reset");

    unsigned char *src = malloc(MAXSRC);
    size_t cap = r_compressBound(MAXSRC) + 4096;
    unsigned char *cbuf = malloc(cap);
    unsigned char *tmp = malloc(cap);
    unsigned char *dbuf = malloc(MAXSRC + 4096);

    st = 0xDEADBEEFCAFEBABEULL;
    void *dctx = t_createDCtx();

    for (int iter = 0; iter < 4000; iter++) {
        /* random input */
        size_t L = (size_t)(rnd() % 4 == 0 ? rnd() % 200 : rnd() % MAXSRC);
        int shape = rnd() % 6;
        unsigned alpha = 1u << (1 + rnd() % 8);
        for (size_t i = 0; i < L; i++) {
            switch (shape) {
            case 0: src[i] = 0; break;
            case 1: src[i] = rnd() & 0xFF; break;
            case 2: src[i] = (unsigned char)(rnd() % alpha); break;
            case 3: src[i] = (unsigned char)(i % alpha); break;
            case 4: src[i] = (rnd() % 100) < 90 ? 'Z' : (rnd() & 0xFF); break;
            default: src[i] = (unsigned char)((i * 2654435761u) >> 24); break;
            }
        }

        /* random compression settings (reference side) */
        void *cctx = r_createCCtx();
        int lvl = (int)(rnd() % 28) - 5;
        r_CCtx_setParameter(cctx, 100, lvl);                 /* compressionLevel */
        r_CCtx_setParameter(cctx, 101, 10 + (int)(rnd() % 14)); /* windowLog */
        r_CCtx_setParameter(cctx, 160, (int)(rnd() % 2));    /* checksumFlag */
        r_CCtx_setParameter(cctx, 161, (int)(rnd() % 2));    /* dictIDFlag */
        r_CCtx_setParameter(cctx, 162, (int)(rnd() % 2));    /* contentSizeFlag */
        if (rnd() % 4 == 0) r_CCtx_setParameter(cctx, 102, 1 + (int)(rnd() % 8)); /* hashLog */
        if (rnd() % 4 == 0) r_CCtx_setParameter(cctx, 5, (int)(rnd() % 10));      /* strategy */
        size_t c = r_compress2(cctx, cbuf, cap, src, L);
        r_freeCCtx(cctx);
        if (r_isError(c)) { printf("%d cErr\n", iter); continue; }

        /* one-shot */
        memset(dbuf, 0x77, L + 16);
        size_t d = t_decompress(dbuf, L + 16, cbuf, c);
        unsigned long long h = 1469598103934665603ULL;
        if (!t_isError(d)) for (size_t i = 0; i < d; i++) { h ^= dbuf[i]; h *= 1099511628211ULL; }
        printf("%d L=%zu lvl=%d c=%zu d=%zu code=%d h=%llu ok=%d\n", iter, L, lvl, c, d,
               t_getErrorCode(d), h, (!t_isError(d) && d == L && (L == 0 || !memcmp(dbuf, src, L))));

        /* random truncation */
        size_t cut = c ? rnd() % (c + 1) : 0;
        size_t dtr = t_decompress(dbuf, L + 16, cbuf, cut);
        printf("  cut=%zu d=%zu code=%d fcs=%llu ffcs=%zu\n", cut, dtr, t_getErrorCode(dtr),
               t_fcs(cbuf, cut), t_ffcs(cbuf, cut));

        /* random bit flips */
        if (c > 0) {
            int nflip = 1 + (int)(rnd() % 3);
            memcpy(tmp, cbuf, c);
            for (int k = 0; k < nflip; k++) tmp[rnd() % c] ^= (unsigned char)(1u << (rnd() % 8));
            size_t dcor = t_decompress(dbuf, L + 16, tmp, c);
            unsigned long long hc = 1469598103934665603ULL;
            if (!t_isError(dcor)) for (size_t i = 0; i < dcor; i++) { hc ^= dbuf[i]; hc *= 1099511628211ULL; }
            printf("  flips=%d d=%zu code=%d h=%llu\n", nflip, dcor, t_getErrorCode(dcor), hc);
        }

        /* random streaming chunking, random windowLogMax */
        if (L <= 100000) {
            t_DCtx_reset(dctx, 3);
            if (rnd() % 3 == 0) t_DCtx_setParameter(dctx, 100, 10 + (int)(rnd() % 22));
            t_initDStream(dctx);
            size_t ich = 1 + rnd() % 5000, och = 1 + rnd() % 5000;
            inBuf in = {cbuf, 0, 0};
            outBuf out = {dbuf, 0, 0};
            memset(dbuf, 0x99, L + 16);
            size_t ret = 0, guard = 0;
            while (1) {
                in.size = in.pos + ich < c ? in.pos + ich : c;
                out.size = out.pos + och < L + 16 ? out.pos + och : L + 16;
                size_t bi = in.pos, bo = out.pos;
                ret = t_decompressStream(dctx, &out, &in);
                if (t_isError(ret)) break;
                if (ret == 0) break;
                if (in.pos == bi && out.pos == bo && in.size == c && out.size == L + 16) break;
                if (++guard > 2000000) break;
            }
            printf("  stream ich=%zu och=%zu ret=%zu code=%d outpos=%zu ok=%d\n", ich, och, ret,
                   t_getErrorCode(ret), out.pos,
                   (!t_isError(ret) && out.pos == L && (L == 0 || !memcmp(dbuf, src, L))));
        }
    }
    t_freeDCtx(dctx);
    fflush(stdout);
    return 0;
}
