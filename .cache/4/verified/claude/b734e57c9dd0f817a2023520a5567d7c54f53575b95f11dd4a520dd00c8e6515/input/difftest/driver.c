/* Differential driver: dlopen()s a zstd shared object and exercises a large
 * slice of its public API, printing a deterministic transcript.  Running it
 * against the C .so and the Rust .so and diffing the transcripts proves
 * byte-identical behaviour.
 *
 * Build:  gcc -O1 -o difftest/driver difftest/driver.c -ldl
 * Run:    difftest/driver <path-to-libzstd.so>
 */
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <dlfcn.h>

static void *H;

static void *sym(const char *n)
{
    void *p = dlsym(H, n);
    if (!p) { fprintf(stderr, "MISSING SYMBOL %s\n", n); exit(2); }
    return p;
}
static void *symopt(const char *n) { return dlsym(H, n); }

/* ---- deterministic pseudo random ---- */
static uint64_t rstate;
static void rseed(uint64_t s) { rstate = s ? s : 1; }
static uint32_t rnext(void)
{
    rstate ^= rstate << 13;
    rstate ^= rstate >> 7;
    rstate ^= rstate << 17;
    return (uint32_t)(rstate >> 11);
}

/* ---- output digest (FNV-1a, computed here so it never depends on the lib) ---- */
static uint64_t fnv(const void *p, size_t n)
{
    const unsigned char *b = (const unsigned char *)p;
    uint64_t h = 1469598103934665603ULL;
    size_t i;
    for (i = 0; i < n; i++) { h ^= b[i]; h *= 1099511628211ULL; }
    return h;
}

#define P(...) printf(__VA_ARGS__)

/* ---- test corpora ---- */
static void fill_text(unsigned char *b, size_t n, uint64_t seed)
{
    static const char *words[] = {
        "the ", "quick ", "brown ", "fox ", "jumps ", "over ", "lazy ", "dog ",
        "zstandard ", "compression ", "dictionary ", "entropy ", "huffman ",
        "sequence ", "literal ", "match ", "offset ", "window ", "block ", "frame "
    };
    size_t pos = 0;
    rseed(seed);
    while (pos < n) {
        const char *w = words[rnext() % 20];
        size_t l = strlen(w);
        if (pos + l > n) l = n - pos;
        memcpy(b + pos, w, l);
        pos += l;
    }
}
static void fill_random(unsigned char *b, size_t n, uint64_t seed)
{
    size_t i;
    rseed(seed);
    for (i = 0; i < n; i++) b[i] = (unsigned char)rnext();
}
static void fill_sparse(unsigned char *b, size_t n, uint64_t seed)
{
    size_t i;
    rseed(seed);
    memset(b, 0, n);
    for (i = 0; i < n / 64; i++) b[rnext() % n] = (unsigned char)(rnext() | 1);
}
static void fill_rle(unsigned char *b, size_t n, uint64_t seed)
{
    (void)seed;
    memset(b, 0x5A, n);
}
static void fill_mixed(unsigned char *b, size_t n, uint64_t seed)
{
    size_t half = n / 2;
    fill_text(b, half, seed);
    fill_random(b + half, n - half, seed + 1);
}

typedef void (*filler_t)(unsigned char *, size_t, uint64_t);

int main(int argc, char **argv)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    if (argc < 2) { fprintf(stderr, "usage: %s <libzstd.so>\n", argv[0]); return 2; }
    H = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!H) { fprintf(stderr, "dlopen failed: %s\n", dlerror()); return 2; }

    unsigned      (*versionNumber)(void)                             = sym("ZSTD_versionNumber");
    const char *  (*versionString)(void)                             = sym("ZSTD_versionString");
    size_t        (*compressBound)(size_t)                           = sym("ZSTD_compressBound");
    unsigned      (*isError)(size_t)                                 = sym("ZSTD_isError");
    const char *  (*getErrorName)(size_t)                            = sym("ZSTD_getErrorName");
    const char *  (*getErrorString)(int)                             = sym("ZSTD_getErrorString");
    int           (*maxCLevel)(void)                                 = sym("ZSTD_maxCLevel");
    int           (*minCLevel)(void)                                 = sym("ZSTD_minCLevel");
    int           (*defaultCLevel)(void)                             = sym("ZSTD_defaultCLevel");
    size_t        (*zcompress)(void*,size_t,const void*,size_t,int)  = sym("ZSTD_compress");
    size_t        (*zdecompress)(void*,size_t,const void*,size_t)    = sym("ZSTD_decompress");
    unsigned long long (*getFrameContentSize)(const void*,size_t)    = sym("ZSTD_getFrameContentSize");
    unsigned long long (*findDecompressedSize)(const void*,size_t)   = sym("ZSTD_findDecompressedSize");
    unsigned long long (*decompressBound)(const void*,size_t)        = sym("ZSTD_decompressBound");
    size_t        (*findFrameCompressedSize)(const void*,size_t)     = sym("ZSTD_findFrameCompressedSize");
    unsigned      (*zisFrame)(const void*,size_t)                    = sym("ZSTD_isFrame");
    void *        (*createCCtx)(void)                                = sym("ZSTD_createCCtx");
    size_t        (*freeCCtx)(void*)                                 = sym("ZSTD_freeCCtx");
    size_t        (*compressCCtx)(void*,void*,size_t,const void*,size_t,int) = sym("ZSTD_compressCCtx");
    size_t        (*cctxSetParameter)(void*,int,int)                 = sym("ZSTD_CCtx_setParameter");
    size_t        (*cctxReset)(void*,int)                            = sym("ZSTD_CCtx_reset");
    size_t        (*compress2)(void*,void*,size_t,const void*,size_t)= sym("ZSTD_compress2");
    void *        (*createDCtx)(void)                                = sym("ZSTD_createDCtx");
    size_t        (*freeDCtx)(void*)                                 = sym("ZSTD_freeDCtx");
    size_t        (*decompressDCtx)(void*,void*,size_t,const void*,size_t) = sym("ZSTD_decompressDCtx");
    size_t        (*sizeofCCtx)(const void*)                         = sym("ZSTD_sizeof_CCtx");
    size_t        (*sizeofDCtx)(const void*)                         = sym("ZSTD_sizeof_DCtx");
    size_t        (*estimateCCtxSize)(int)                           = sym("ZSTD_estimateCCtxSize");
    size_t        (*estimateDCtxSize)(void)                          = sym("ZSTD_estimateDCtxSize");
    size_t        (*estimateCStreamSize)(int)                        = sym("ZSTD_estimateCStreamSize");
    size_t        (*estimateDStreamSize)(size_t)                     = sym("ZSTD_estimateDStreamSize");
    size_t        (*cstreamInSize)(void)                             = sym("ZSTD_CStreamInSize");
    size_t        (*cstreamOutSize)(void)                            = sym("ZSTD_CStreamOutSize");
    size_t        (*dstreamInSize)(void)                             = sym("ZSTD_DStreamInSize");
    size_t        (*dstreamOutSize)(void)                            = sym("ZSTD_DStreamOutSize");
    unsigned      (*xxh32)(const void*,size_t,unsigned)              = sym("ZSTD_XXH32");
    unsigned long long (*xxh64)(const void*,size_t,unsigned long long)= sym("ZSTD_XXH64");
    unsigned      (*xxhVersion)(void)                                = sym("ZSTD_XXH_versionNumber");
    unsigned      (*fseVersion)(void)                                = sym("FSE_versionNumber");
    size_t        (*hufCompressBound)(size_t)                        = sym("HUF_compressBound");
    size_t        (*fseCompressBound)(size_t)                        = sym("FSE_compressBound");
    unsigned      (*fseOptimalTableLog)(unsigned,size_t,unsigned)    = sym("FSE_optimalTableLog");
    size_t        (*fseNCountWriteBound)(unsigned,unsigned)          = sym("FSE_NCountWriteBound");
    unsigned      (*hufMinTableLog)(unsigned)                        = sym("HUF_minTableLog");
    unsigned      (*hufCardinality)(const unsigned*,unsigned)        = sym("HUF_cardinality");
    size_t        (*histCount)(unsigned*,unsigned*,const void*,size_t) = sym("HIST_count");
    void          (*histAdd)(unsigned*,const void*,size_t)           = sym("HIST_add");
    unsigned      (*histIsError)(size_t)                             = sym("HIST_isError");
    unsigned      (*cycleLog)(unsigned,int)                          = sym("ZSTD_cycleLog");
    size_t        (*sequenceBound)(size_t)                           = sym("ZSTD_sequenceBound");
    size_t        (*decompressionMargin)(const void*,size_t)         = sym("ZSTD_decompressionMargin");
    size_t        (*frameHeaderSize)(const void*,size_t)             = sym("ZSTD_frameHeaderSize");
    unsigned      (*getDictID_fromDict)(const void*,size_t)          = sym("ZSTD_getDictID_fromDict");
    unsigned      (*getDictID_fromFrame)(const void*,size_t)         = sym("ZSTD_getDictID_fromFrame");
    size_t        (*getBlockSize)(const void*)                       = sym("ZSTD_getBlockSize");
    size_t        (*decodingBufferSize_min)(unsigned long long,unsigned long long) = sym("ZSTD_decodingBufferSize_min");

    /* streaming */
    void *        (*createCStream)(void)                             = sym("ZSTD_createCStream");
    size_t        (*freeCStream)(void*)                              = sym("ZSTD_freeCStream");
    size_t        (*initCStream)(void*,int)                          = sym("ZSTD_initCStream");
    size_t        (*compressStream)(void*,void*,void*)               = sym("ZSTD_compressStream");
    size_t        (*flushStream)(void*,void*)                        = sym("ZSTD_flushStream");
    size_t        (*endStream)(void*,void*)                          = sym("ZSTD_endStream");
    size_t        (*compressStream2)(void*,void*,void*,int)          = sym("ZSTD_compressStream2");
    void *        (*createDStream)(void)                             = sym("ZSTD_createDStream");
    size_t        (*freeDStream)(void*)                              = sym("ZSTD_freeDStream");
    size_t        (*initDStream)(void*)                              = sym("ZSTD_initDStream");
    size_t        (*decompressStream)(void*,void*,void*)             = sym("ZSTD_decompressStream");

    /* dictionaries */
    void *        (*createCDict)(const void*,size_t,int)             = sym("ZSTD_createCDict");
    size_t        (*freeCDict)(void*)                                = sym("ZSTD_freeCDict");
    size_t        (*compress_usingCDict)(void*,void*,size_t,const void*,size_t,const void*) = sym("ZSTD_compress_usingCDict");
    void *        (*createDDict)(const void*,size_t)                 = sym("ZSTD_createDDict");
    size_t        (*freeDDict)(void*)                                = sym("ZSTD_freeDDict");
    size_t        (*decompress_usingDDict)(void*,void*,size_t,const void*,size_t,const void*) = sym("ZSTD_decompress_usingDDict");
    size_t        (*compress_usingDict)(void*,void*,size_t,const void*,size_t,const void*,size_t,int) = sym("ZSTD_compress_usingDict");
    size_t        (*decompress_usingDict)(void*,void*,size_t,const void*,size_t,const void*,size_t) = sym("ZSTD_decompress_usingDict");
    size_t        (*sizeofCDict)(const void*)                        = sym("ZSTD_sizeof_CDict");
    size_t        (*sizeofDDict)(const void*)                        = sym("ZSTD_sizeof_DDict");
    unsigned      (*getDictID_fromCDict)(const void*)                = sym("ZSTD_getDictID_fromCDict");
    unsigned      (*getDictID_fromDDict)(const void*)                = sym("ZSTD_getDictID_fromDDict");
    size_t        (*estimateCDictSize)(size_t,int)                   = sym("ZSTD_estimateCDictSize");
    size_t        (*estimateDDictSize)(size_t,int)                   = sym("ZSTD_estimateDDictSize");

    /* dict builder */
    size_t        (*zdictTrain)(void*,size_t,const void*,const size_t*,unsigned) = sym("ZDICT_trainFromBuffer");
    unsigned      (*zdictIsError)(size_t)                            = sym("ZDICT_isError");
    const char *  (*zdictGetErrorName)(size_t)                       = sym("ZDICT_getErrorName");
    unsigned      (*zdictGetDictID)(const void*,size_t)              = sym("ZDICT_getDictID");
    size_t        (*zdictGetHeaderSize)(const void*,size_t)          = sym("ZDICT_getDictHeaderSize");

    /* deprecated zbuff */
    size_t        (*zbuffRecCIn)(void)                               = sym("ZBUFF_recommendedCInSize");
    size_t        (*zbuffRecCOut)(void)                              = sym("ZBUFF_recommendedCOutSize");
    size_t        (*zbuffRecDIn)(void)                               = sym("ZBUFF_recommendedDInSize");
    size_t        (*zbuffRecDOut)(void)                              = sym("ZBUFF_recommendedDOutSize");
    unsigned      (*zbuffIsError)(size_t)                            = sym("ZBUFF_isError");

    /* divsufsort */
    int           (*divsufsort_)(const unsigned char*,int*,int,int)  = sym("divsufsort");

    /* ---------------- basic constants ---------------- */
    P("version %u %s\n", versionNumber(), versionString());
    P("clevels %d %d %d\n", maxCLevel(), minCLevel(), defaultCLevel());
    P("bounds %zu %zu %zu %zu\n", compressBound(0), compressBound(1),
      compressBound(100000), compressBound(1u<<20));
    P("streamsizes %zu %zu %zu %zu\n", cstreamInSize(), cstreamOutSize(),
      dstreamInSize(), dstreamOutSize());
    P("estimates %zu %zu %zu %zu %zu\n", estimateCCtxSize(1), estimateCCtxSize(9),
      estimateCCtxSize(19), estimateDCtxSize(), estimateCStreamSize(3));
    P("estimates2 %zu %zu %zu\n", estimateDStreamSize(1u<<17),
      estimateCDictSize(1000, 5), estimateDDictSize(1000, 0));
    P("hufbound %zu fsebound %zu ncount %zu\n", hufCompressBound(1000),
      fseCompressBound(1000), fseNCountWriteBound(255, 11));
    P("versions %u %u\n", xxhVersion(), fseVersion());
    P("zbuff %zu %zu %zu %zu %u\n", zbuffRecCIn(), zbuffRecCOut(),
      zbuffRecDIn(), zbuffRecDOut(), zbuffIsError((size_t)-5));
    P("seqbound %zu %zu\n", sequenceBound(0), sequenceBound(100000));

    { int i;
      for (i = 0; i < 6; i++)
          P("fseOTL[%d] %u %u\n", i, fseOptimalTableLog(11, (size_t)1 << (i * 3), 255),
            hufMinTableLog((unsigned)(1 << i)));
    }
    { unsigned hl, st;
      for (hl = 10; hl <= 24; hl += 7)
          for (st = 1; st <= 9; st++)
              P("cycleLog %u %d -> %u\n", hl, st, cycleLog(hl, (int)st));
    }
    { int c;
      for (c = -20; c <= 130; c += 1) {
          if (c > 30 && c < 100) continue;
          P("err %d %s\n", c, getErrorString(c));
      }
    }
    { size_t e;
      for (e = 0; e < 130; e += 7)
          P("errname %zu %u %s\n", (size_t)0 - e, isError((size_t)0 - e),
            getErrorName((size_t)0 - e));
    }

    /* ---------------- one-shot compression at every level ---------------- */
    {
        static const size_t sizes[] = { 0, 1, 2, 3, 7, 16, 31, 64, 100, 255, 256,
                                        1000, 4096, 10000, 65535, 65536, 131072,
                                        200000, 400000 };
        filler_t fillers[5] = { fill_text, fill_random, fill_sparse, fill_rle, fill_mixed };
        const char *fnames[5] = { "text", "rand", "sparse", "rle", "mixed" };
        size_t si, fi;
        int level;
        size_t maxsz = 400000;
        unsigned char *src = (unsigned char *)malloc(maxsz + 1);
        size_t cap = compressBound(maxsz) + 1024;
        unsigned char *cbuf = (unsigned char *)malloc(cap);
        unsigned char *dbuf = (unsigned char *)malloc(maxsz + 1024);
        void *cctx = createCCtx();
        void *dctx = createDCtx();

        for (fi = 0; fi < 5; fi++) {
            for (si = 0; si < sizeof(sizes) / sizeof(sizes[0]); si++) {
                size_t n = sizes[si];
                fillers[fi](src, n, 12345 + si * 17 + fi * 101);
                for (level = -5; level <= 22; level++) {
                    size_t cs, ds;
                    if (level == -4 || level == -3 || level == -2) continue;
                    if (n > 70000 && level > 12 && level < 19) continue;
                    cs = zcompress(cbuf, cap, src, n, level);
                    if (isError(cs)) { P("C %s %zu %d ERR %s\n", fnames[fi], n, level, getErrorName(cs)); continue; }
                    P("C %s %zu %d %zu %016llx", fnames[fi], n, level, cs,
                      (unsigned long long)fnv(cbuf, cs));
                    P(" fcs=%llu fds=%llu dbound=%llu ffcs=%zu isf=%u fhs=%zu\n",
                      (unsigned long long)getFrameContentSize(cbuf, cs),
                      (unsigned long long)findDecompressedSize(cbuf, cs),
                      (unsigned long long)decompressBound(cbuf, cs),
                      findFrameCompressedSize(cbuf, cs),
                      zisFrame(cbuf, cs),
                      frameHeaderSize(cbuf, cs < 18 ? cs : 18));
                    ds = zdecompress(dbuf, maxsz + 1024, cbuf, cs);
                    if (isError(ds)) { P("  D ERR %s\n", getErrorName(ds)); continue; }
                    P("  D %zu %016llx match=%d margin=%zu\n", ds,
                      (unsigned long long)fnv(dbuf, ds),
                      (ds == n && (n == 0 || memcmp(dbuf, src, n) == 0)),
                      decompressionMargin(cbuf, cs));
                    /* same via CCtx / DCtx */
                    cs = compressCCtx(cctx, cbuf, cap, src, n, level);
                    P("  Cctx %zu %016llx sz=%zu\n", cs,
                      (unsigned long long)fnv(cbuf, cs), sizeofCCtx(cctx) != 0);
                    ds = decompressDCtx(dctx, dbuf, maxsz + 1024, cbuf, cs);
                    P("  Dctx %zu %016llx sz=%d\n", ds,
                      (unsigned long long)fnv(dbuf, ds), sizeofDCtx(dctx) != 0);
                }
            }
        }
        freeCCtx(cctx);
        freeDCtx(dctx);
        free(src); free(cbuf); free(dbuf);
    }

    /* ---------------- advanced parameters via compress2 ---------------- */
    {
        size_t n = 120000;
        unsigned char *src = (unsigned char *)malloc(n);
        size_t cap = compressBound(n) + 1024;
        unsigned char *cbuf = (unsigned char *)malloc(cap);
        unsigned char *dbuf = (unsigned char *)malloc(n + 1024);
        void *cctx = createCCtx();
        int i;
        /* param id, values... */
        struct { const char *name; int id; int v[4]; int nv; } tests[] = {
            { "windowLog",      101, { 10, 15, 20, 23 }, 4 },
            { "hashLog",        102, { 6, 12, 18, 22 }, 4 },
            { "chainLog",       103, { 6, 12, 18, 22 }, 4 },
            { "searchLog",      104, { 1, 3, 6, 9 }, 4 },
            { "minMatch",       105, { 3, 4, 5, 7 }, 4 },
            { "targetLength",   106, { 0, 16, 128, 1024 }, 4 },
            { "strategy",       107, { 1, 3, 5, 7, }, 4 },
            { "targetCBlock",   130, { 0, 1340, 4096, 65536 }, 4 },
            { "enableLDM",      160, { 0, 1, 2, 0 }, 3 },
            { "ldmHashLog",     161, { 6, 12, 20, 0 }, 3 },
            { "ldmMinMatch",    162, { 4, 16, 64, 0 }, 3 },
            { "ldmBucketSzLog", 163, { 1, 4, 8, 0 }, 3 },
            { "ldmHashRateLog", 164, { 0, 4, 8, 0 }, 3 },
            { "contentSize",    200, { 0, 1, 0, 0 }, 2 },
            { "checksum",       201, { 0, 1, 0, 0 }, 2 },
            { "dictID",         202, { 0, 1, 0, 0 }, 2 },
            { "format",          10, { 0, 1, 0, 0 }, 2 },
            { "litCompMode",   1002, { 0, 1, 2, 0 }, 3 },
            { "srcSizeHint",   1004, { 0, 1000, 500000, 0 }, 3 },
            { "blockSplitter", 1017, { 0, 1, 4, 6 }, 4 },
            { "splitAfterSeq", 1010, { 0, 1, 2, 0 }, 3 },
            { "rowMatchFinder",1011, { 0, 1, 2, 0 }, 3 },
            { "maxBlockSize",  1015, { 0, 1024, 65536, 131072 }, 4 },
            { "repcodeRes",    1016, { 0, 1, 2, 0 }, 3 },
            { "prefetchCDict", 1013, { 0, 1, 2, 0 }, 3 },
            { "deterministic", 1012, { 0, 1, 0, 0 }, 2 },
            { "seqProdFallbk", 1014, { 0, 1, 0, 0 }, 2 },
        };
        fill_mixed(src, n, 999);
        for (i = 0; i < (int)(sizeof(tests) / sizeof(tests[0])); i++) {
            int j, lv;
            for (j = 0; j < tests[i].nv; j++) {
                for (lv = 1; lv <= 19; lv += 6) {
                    size_t cs, ds, r;
                    cctxReset(cctx, 3 /* session_and_parameters */);
                    r = cctxSetParameter(cctx, 100, lv);
                    if (isError(r)) { P("P %s set-level ERR\n", tests[i].name); continue; }
                    r = cctxSetParameter(cctx, tests[i].id, tests[i].v[j]);
                    if (isError(r)) {
                        P("P %s=%d lv%d SETERR %s\n", tests[i].name, tests[i].v[j], lv, getErrorName(r));
                        continue;
                    }
                    cs = compress2(cctx, cbuf, cap, src, n);
                    if (isError(cs)) {
                        P("P %s=%d lv%d CERR %s\n", tests[i].name, tests[i].v[j], lv, getErrorName(cs));
                        continue;
                    }
                    P("P %s=%d lv%d %zu %016llx", tests[i].name, tests[i].v[j], lv, cs,
                      (unsigned long long)fnv(cbuf, cs));
                    if (tests[i].id == 10 && tests[i].v[j] == 1) { P(" (magicless)\n"); continue; }
                    ds = zdecompress(dbuf, n + 1024, cbuf, cs);
                    P(" D %zu %d\n", ds, !isError(ds) && ds == n && memcmp(dbuf, src, n) == 0);
                }
            }
        }
        freeCCtx(cctx);
        free(src); free(cbuf); free(dbuf);
    }

    /* ---------------- streaming ---------------- */
    {
        typedef struct { const void *src; size_t size; size_t pos; } inB;
        typedef struct { void *dst; size_t size; size_t pos; } outB;
        size_t n = 300000;
        unsigned char *src = (unsigned char *)malloc(n);
        size_t cap = compressBound(n) + 4096;
        unsigned char *cbuf = (unsigned char *)malloc(cap);
        unsigned char *dbuf = (unsigned char *)malloc(n + 4096);
        size_t chunks[] = { 1, 7, 1000, 65536, 300000 };
        int ci, level;
        fill_mixed(src, n, 4242);
        for (ci = 0; ci < 5; ci++) {
            for (level = 1; level <= 19; level += 9) {
                void *zcs = createCStream();
                void *zds = createDStream();
                outB out; inB in;
                size_t remaining, produced, i;
                initCStream(zcs, level);
                out.dst = cbuf; out.size = cap; out.pos = 0;
                for (i = 0; i < n; i += chunks[ci]) {
                    size_t take = chunks[ci];
                    if (i + take > n) take = n - i;
                    in.src = src + i; in.size = take; in.pos = 0;
                    while (in.pos < in.size) {
                        size_t h = compressStream(zcs, &out, &in);
                        if (isError(h)) { P("S CERR %s\n", getErrorName(h)); break; }
                    }
                }
                do { remaining = endStream(zcs, &out); } while (remaining != 0 && !isError(remaining));
                produced = out.pos;
                P("S chunk=%zu lv=%d c=%zu %016llx\n", chunks[ci], level, produced,
                  (unsigned long long)fnv(cbuf, produced));
                /* decompress streaming with the same chunking */
                initDStream(zds);
                out.dst = dbuf; out.size = n + 4096; out.pos = 0;
                for (i = 0; i < produced; i += chunks[ci]) {
                    size_t take = chunks[ci];
                    if (i + take > produced) take = produced - i;
                    in.src = cbuf + i; in.size = take; in.pos = 0;
                    while (in.pos < in.size) {
                        size_t h = decompressStream(zds, &out, &in);
                        if (isError(h)) { P("S DERR %s\n", getErrorName(h)); break; }
                        if (h == 0 && in.pos == in.size) break;
                    }
                }
                P("S   d=%zu %016llx ok=%d\n", out.pos, (unsigned long long)fnv(dbuf, out.pos),
                  out.pos == n && memcmp(dbuf, src, n) == 0);
                freeCStream(zcs);
                freeDStream(zds);
            }
        }
        /* compressStream2 with flush/end directives */
        for (level = 1; level <= 12; level += 11) {
            void *cctx = createCCtx();
            outB out; inB in;
            size_t i, r;
            cctxReset(cctx, 3);
            cctxSetParameter(cctx, 100, level);
            out.dst = cbuf; out.size = cap; out.pos = 0;
            for (i = 0; i < n; i += 50000) {
                size_t take = 50000; if (i + take > n) take = n - i;
                in.src = src + i; in.size = take; in.pos = 0;
                while (in.pos < in.size) {
                    r = compressStream2(cctx, &out, &in, 0 /*continue*/);
                    if (isError(r)) { P("S2 ERR %s\n", getErrorName(r)); break; }
                }
                in.src = src; in.size = 0; in.pos = 0;
                do { r = compressStream2(cctx, &out, &in, 1 /*flush*/); } while (r != 0 && !isError(r));
            }
            in.src = src; in.size = 0; in.pos = 0;
            do { r = compressStream2(cctx, &out, &in, 2 /*end*/); } while (r != 0 && !isError(r));
            P("S2 lv=%d c=%zu %016llx\n", level, out.pos, (unsigned long long)fnv(cbuf, out.pos));
            { size_t ds = zdecompress(dbuf, n + 4096, cbuf, out.pos);
              P("S2   d=%zu ok=%d\n", ds, !isError(ds) && ds == n && memcmp(dbuf, src, n) == 0); }
            freeCCtx(cctx);
        }
        free(src); free(cbuf); free(dbuf);
    }

    /* ---------------- dictionaries ---------------- */
    {
        size_t nbSamples = 64, sampleSize = 3000;
        size_t total = nbSamples * sampleSize;
        unsigned char *samples = (unsigned char *)malloc(total);
        size_t *sizes = (size_t *)malloc(nbSamples * sizeof(size_t));
        size_t dictCap = 40000;
        unsigned char *dict = (unsigned char *)malloc(dictCap);
        size_t i, dsz;
        for (i = 0; i < nbSamples; i++) {
            fill_text(samples + i * sampleSize, sampleSize, 7777 + i);
            sizes[i] = sampleSize;
        }
        dsz = zdictTrain(dict, dictCap, samples, sizes, (unsigned)nbSamples);
        if (zdictIsError(dsz)) {
            P("DICT train ERR %s\n", zdictGetErrorName(dsz));
        } else {
            size_t n = 30000, cap, cs, ds;
            unsigned char *src, *cbuf, *dbuf;
            void *cd, *dd;
            int level;
            P("DICT size=%zu %016llx id=%u hdr=%zu fromDict=%u\n", dsz,
              (unsigned long long)fnv(dict, dsz), zdictGetDictID(dict, dsz),
              zdictGetHeaderSize(dict, dsz), getDictID_fromDict(dict, dsz));
            src = (unsigned char *)malloc(n);
            fill_text(src, n, 31337);
            cap = compressBound(n) + 1024;
            cbuf = (unsigned char *)malloc(cap);
            dbuf = (unsigned char *)malloc(n + 1024);
            for (level = 1; level <= 19; level += 6) {
                void *cctx = createCCtx();
                void *dctx = createDCtx();
                cs = compress_usingDict(cctx, cbuf, cap, src, n, dict, dsz, level);
                P("DICT lv%d usingDict c=%zu %016llx frameID=%u\n", level, cs,
                  (unsigned long long)fnv(cbuf, cs), getDictID_fromFrame(cbuf, cs));
                ds = decompress_usingDict(dctx, dbuf, n + 1024, cbuf, cs, dict, dsz);
                P("DICT lv%d usingDict d=%zu ok=%d\n", level, ds,
                  !isError(ds) && ds == n && memcmp(dbuf, src, n) == 0);
                freeCCtx(cctx); freeDCtx(dctx);

                cd = createCDict(dict, dsz, level);
                dd = createDDict(dict, dsz);
                cctx = createCCtx(); dctx = createDCtx();
                cs = compress_usingCDict(cctx, cbuf, cap, src, n, cd);
                P("DICT lv%d usingCDict c=%zu %016llx cdID=%u ddID=%u szCD=%d szDD=%d\n",
                  level, cs, (unsigned long long)fnv(cbuf, cs),
                  getDictID_fromCDict(cd), getDictID_fromDDict(dd),
                  sizeofCDict(cd) != 0, sizeofDDict(dd) != 0);
                ds = decompress_usingDDict(dctx, dbuf, n + 1024, cbuf, cs, dd);
                P("DICT lv%d usingDDict d=%zu ok=%d\n", level, ds,
                  !isError(ds) && ds == n && memcmp(dbuf, src, n) == 0);
                freeCCtx(cctx); freeDCtx(dctx);
                freeCDict(cd); freeDDict(dd);
            }
            free(src); free(cbuf); free(dbuf);
        }
        free(samples); free(sizes); free(dict);
    }

    /* ---------------- cover / fastcover dict trainers ---------------- */
    {
        void *coverSym = symopt("ZDICT_trainFromBuffer_cover");
        void *fastcoverSym = symopt("ZDICT_trainFromBuffer_fastCover");
        /* ZDICT_cover_params_t { unsigned k; unsigned d; unsigned steps; unsigned nbThreads;
         *                        double splitPoint; ZDICT_params_t zParams; }
         * ZDICT_params_t { int compressionLevel; unsigned notificationLevel; unsigned dictID; } */
        struct zparams { int compressionLevel; unsigned notificationLevel; unsigned dictID; };
        struct coverp { unsigned k, d, steps, nbThreads; double splitPoint;
                        unsigned shrinkDict, shrinkDictMaxRegression; struct zparams z; };
        struct fastcoverp { unsigned k, d, f, steps, nbThreads; double splitPoint;
                            unsigned accel, shrinkDict, shrinkDictMaxRegression;
                            struct zparams z; };
        size_t nbSamples = 48, sampleSize = 4000;
        size_t total = nbSamples * sampleSize;
        unsigned char *samples = (unsigned char *)malloc(total);
        size_t *sizes = (size_t *)malloc(nbSamples * sizeof(size_t));
        unsigned char *dict = (unsigned char *)malloc(32768);
        size_t i, r;
        for (i = 0; i < nbSamples; i++) {
            fill_text(samples + i * sampleSize, sampleSize, 555 + i * 3);
            sizes[i] = sampleSize;
        }
        if (coverSym) {
            size_t (*cover)(void*,size_t,const void*,const size_t*,unsigned,struct coverp) =
                (size_t (*)(void*,size_t,const void*,const size_t*,unsigned,struct coverp))coverSym;
            unsigned dvals[2] = { 6, 8 };
            int di;
            for (di = 0; di < 2; di++) {
                struct coverp p;
                memset(&p, 0, sizeof(p));
                p.k = 200; p.d = dvals[di]; p.steps = 4; p.nbThreads = 1; p.splitPoint = 1.0;
                p.z.compressionLevel = 3; p.z.notificationLevel = 0; p.z.dictID = 0;
                r = cover(dict, 16384, samples, sizes, (unsigned)nbSamples, p);
                if (zdictIsError(r)) P("COVER d=%u ERR %s\n", p.d, zdictGetErrorName(r));
                else P("COVER d=%u %zu %016llx\n", p.d, r, (unsigned long long)fnv(dict, r));
            }
        }
        if (fastcoverSym) {
            size_t (*fastcover)(void*,size_t,const void*,const size_t*,unsigned,struct fastcoverp) =
                (size_t (*)(void*,size_t,const void*,const size_t*,unsigned,struct fastcoverp))fastcoverSym;
            unsigned accels[3] = { 1, 2, 5 };
            int ai;
            for (ai = 0; ai < 3; ai++) {
                struct fastcoverp p;
                memset(&p, 0, sizeof(p));
                p.k = 200; p.d = 8; p.f = 20; p.steps = 4; p.nbThreads = 1;
                p.splitPoint = 0.75; p.accel = accels[ai];
                p.z.compressionLevel = 3; p.z.notificationLevel = 0; p.z.dictID = 0;
                r = fastcover(dict, 16384, samples, sizes, (unsigned)nbSamples, p);
                if (zdictIsError(r)) P("FASTCOVER a=%u ERR %s\n", p.accel, zdictGetErrorName(r));
                else P("FASTCOVER a=%u %zu %016llx\n", p.accel, r, (unsigned long long)fnv(dict, r));
            }
        }
        free(samples); free(sizes); free(dict);
    }

    /* ---------------- xxhash ---------------- */
    {
        size_t n = 100000;
        unsigned char *b = (unsigned char *)malloc(n);
        size_t i;
        fill_random(b, n, 24680);
        for (i = 0; i <= 300; i += 7)
            P("xxh %zu %08x %016llx\n", i, xxh32(b, i, (unsigned)i),
              (unsigned long long)xxh64(b, i, (unsigned long long)i));
        for (i = 1000; i <= n; i += 13000)
            P("xxh %zu %08x %016llx\n", i, xxh32(b, i, 0),
              (unsigned long long)xxh64(b, i, 0));
        free(b);
    }

    /* ---------------- HIST / HUF helpers ---------------- */
    {
        size_t n = 60000;
        unsigned char *b = (unsigned char *)malloc(n);
        unsigned count[256];
        unsigned maxSymbolValue = 255;
        size_t r;
        int i;
        fill_mixed(b, n, 13579);
        memset(count, 0, sizeof(count));
        r = histCount(count, &maxSymbolValue, b, n);
        P("hist %zu err=%u maxSV=%u %016llx card=%u\n", r, histIsError(r), maxSymbolValue,
          (unsigned long long)fnv(count, sizeof(count)), hufCardinality(count, maxSymbolValue));
        memset(count, 0, sizeof(count));
        histAdd(count, b, n);
        P("histAdd %016llx\n", (unsigned long long)fnv(count, sizeof(count)));
        for (i = 0; i < 8; i++)
            P("hufMinTableLog %d %u\n", i * 30 + 1, hufMinTableLog((unsigned)(i * 30 + 1)));
        free(b);
    }

    /* ---------------- divsufsort ---------------- */
    {
        int n = 5000;
        unsigned char *T = (unsigned char *)malloc((size_t)n);
        int *SA = (int *)malloc(sizeof(int) * (size_t)n);
        int r;
        fill_text(T, (size_t)n, 8642);
        r = divsufsort_(T, SA, n, 0);
        P("divsufsort %d %016llx\n", r, (unsigned long long)fnv(SA, sizeof(int) * (size_t)n));
        fill_random(T, (size_t)n, 8643);
        r = divsufsort_(T, SA, n, 0);
        P("divsufsort2 %d %016llx\n", r, (unsigned long long)fnv(SA, sizeof(int) * (size_t)n));
        free(T); free(SA);
    }

    /* ---------------- misc / edge cases ---------------- */
    {
        unsigned char tiny[8] = { 0, 1, 2, 3, 4, 5, 6, 7 };
        unsigned char out[256];
        size_t r;
        r = zdecompress(out, sizeof(out), tiny, sizeof(tiny));
        P("edge baddecomp %u %s\n", isError(r), getErrorName(r));
        P("edge fcs %llu\n", (unsigned long long)getFrameContentSize(tiny, sizeof(tiny)));
        P("edge isFrame %u\n", zisFrame(tiny, sizeof(tiny)));
        r = zcompress(out, 1, tiny, sizeof(tiny), 3);
        P("edge smalldst %u %s\n", isError(r), getErrorName(r));
        P("edge dbufmin %zu %zu\n", decodingBufferSize_min(1u << 20, 100000),
          decodingBufferSize_min(1u << 10, 0));
        { void *cctx = createCCtx();
          P("edge getBlockSize %zu\n", getBlockSize(cctx));
          freeCCtx(cctx); }
    }


    /* ---------------- corruption / truncation fuzz (modern format) ---------------- */
    {
        typedef struct { const void *src; size_t size; size_t pos; } inB;
        typedef struct { void *dst; size_t size; size_t pos; } outB;
        size_t n = 40000;
        unsigned char *src = (unsigned char *)malloc(n);
        size_t cap = compressBound(n) + 1024;
        unsigned char *cbuf = (unsigned char *)malloc(cap);
        unsigned char *mut = (unsigned char *)malloc(cap);
        unsigned char *dbuf = (unsigned char *)malloc(n + 4096);
        int level;
        fill_mixed(src, n, 777001);
        for (level = 1; level <= 19; level += 9) {
            size_t cs = zcompress(cbuf, cap, src, n, level);
            size_t it;
            P("FUZZ base lv%d cs=%zu\n", level, cs);
            rseed(0xC0FFEEull + (uint64_t)level);
            for (it = 0; it < 200; it++) {
                size_t trunc, k, nflips;
                size_t r;
                memcpy(mut, cbuf, cs);
                trunc = 1 + (rnext() % cs);
                nflips = rnext() % 4;
                for (k = 0; k < nflips; k++) {
                    /* never touch the 4 magic bytes: flipping them can turn the
                     * frame into a *legacy* magic, which routes the data into the
                     * (deliberately unhardened) legacy decoders and segfaults the
                     * reference C build - that is not a translation difference. */
                    size_t off;
                    if (trunc <= 4) break;
                    off = 4 + (rnext() % (trunc - 4));
                    mut[off] = (unsigned char)(mut[off] ^ (1u << (rnext() % 8)));
                }
                r = zdecompress(dbuf, n + 4096, mut, trunc);
                P("FUZZ lv%d it%zu t=%zu f=%zu r=%zu e=%u %s dg=%016llx\n",
                  level, it, trunc, nflips, r, isError(r),
                  isError(r) ? getErrorName(r) : "ok",
                  (unsigned long long)(isError(r) ? 0 : fnv(dbuf, r)));
                /* also through the streaming decoder */
                { void *zds = createDStream();
                  outB out; inB in; size_t rr;
                  initDStream(zds);
                  out.dst = dbuf; out.size = n + 4096; out.pos = 0;
                  in.src = mut; in.size = trunc; in.pos = 0;
                  for (;;) {
                      rr = decompressStream(zds, &out, &in);
                      if (isError(rr)) break;
                      if (rr == 0) break;
                      if (in.pos == in.size && out.pos < out.size) break;
                  }
                  P("FUZZS lv%d it%zu r=%zu e=%u out=%zu dg=%016llx\n", level, it, rr,
                    isError(rr), out.pos, (unsigned long long)fnv(dbuf, out.pos));
                  freeDStream(zds); }
            }
        }
        free(src); free(cbuf); free(mut); free(dbuf);
    }

    /* ---------------- legacy decoder differential fuzz ---------------- */
    {
        struct { const char *name; unsigned magic; } vers[] = {
            { "v01", 0xFD2FB51E }, { "v02", 0xFD2FB522 }, { "v03", 0xFD2FB523 },
            { "v04", 0xFD2FB524 }, { "v05", 0xFD2FB525 }, { "v06", 0xFD2FB526 },
            { "v07", 0xFD2FB527 }
        };
        int vi;
        size_t n = 4096;
        unsigned char *buf = (unsigned char *)malloc(n);
        unsigned char *out = (unsigned char *)malloc(1 << 18);
        for (vi = 0; vi < 7; vi++) {
            char nm[64];
            size_t (*dec)(void*,size_t,const void*,size_t);
            void (*ffsi)(const void*,size_t,size_t*,unsigned long long*);
            unsigned (*iserr)(size_t);
            size_t it;
            snprintf(nm, sizeof(nm), "ZSTD%s_decompress", vers[vi].name);
            dec = (size_t (*)(void*,size_t,const void*,size_t))symopt(nm);
            snprintf(nm, sizeof(nm), "ZSTD%s_findFrameSizeInfoLegacy", vers[vi].name);
            ffsi = (void (*)(const void*,size_t,size_t*,unsigned long long*))symopt(nm);
            snprintf(nm, sizeof(nm), "ZSTD%s_isError", vers[vi].name);
            iserr = (unsigned (*)(size_t))symopt(nm);
            if (!dec) { P("LEG %s no decompress symbol\n", vers[vi].name); continue; }
            /* NOTE: the legacy decoders/parsers are not hardened against corrupt
             * input (that is why they live in legacy/); feeding them random bytes
             * is undefined behaviour in the original C as well - it segfaults the
             * reference build - so no differential comparison is possible there.
             * Only well-formed-input behaviour is compared. */
            (void)ffsi;
            (void)it;
            (void)buf;
            /* also decode a *modern* frame with the legacy entry point (should error) */
            { size_t r;
              unsigned char mf[64];
              size_t cs = zcompress(mf, sizeof(mf), "hello world hello world", 23, 3);
              r = dec(out, 1 << 18, mf, cs);
              P("LEG %s modernframe r=%zu e=%u\n", vers[vi].name, r,
                iserr ? iserr(r) : isError(r)); }
        }
        free(buf); free(out);
    }

    /* ---------------- legacy stream (ZBUFF vXX) smoke ---------------- */
    {
        int vi;
        const char *names[4] = { "ZBUFFv04", "ZBUFFv05", "ZBUFFv06", "ZBUFFv07" };
        for (vi = 0; vi < 4; vi++) {
            char nm[64];
            void * (*cre)(void);
            size_t (*fre)(void*);
            size_t (*recIn)(void);
            size_t (*recOut)(void);
            unsigned (*iserr)(size_t);
            snprintf(nm, sizeof(nm), "%s_createDCtx", names[vi]);      cre = (void *(*)(void))symopt(nm);
            snprintf(nm, sizeof(nm), "%s_freeDCtx", names[vi]);        fre = (size_t (*)(void*))symopt(nm);
            snprintf(nm, sizeof(nm), "%s_recommendedDInSize", names[vi]);  recIn = (size_t (*)(void))symopt(nm);
            snprintf(nm, sizeof(nm), "%s_recommendedDOutSize", names[vi]); recOut = (size_t (*)(void))symopt(nm);
            snprintf(nm, sizeof(nm), "%s_isError", names[vi]);         iserr = (unsigned (*)(size_t))symopt(nm);
            P("LEGSTREAM %s in=%zu out=%zu iserr=%u\n", names[vi],
              recIn ? recIn() : (size_t)-1, recOut ? recOut() : (size_t)-1,
              iserr ? iserr((size_t)-7) : 0);
            if (cre && fre) { void *c = cre(); P("LEGSTREAM %s ctx=%d free=%zu\n", names[vi], c != NULL, fre(c)); }
        }
    }

    /* ---------------- legacy FSE/HUF entry points ---------------- */
    {
        const char *pfx[3] = { "FSEv05", "FSEv06", "FSEv07" };
        int i;
        for (i = 0; i < 3; i++) {
            char nm[64];
            void * (*createDT)(unsigned);
            void (*freeDT)(void*);   /* C: `void FSEv0X_freeDTable(...)` */
            unsigned (*iserr)(size_t);
            const char * (*errname)(size_t);
            size_t (*decomp)(void*,size_t,const void*,size_t);
            unsigned char in[64];
            /* generous heap slack: the legacy FSE decoders can over-write past
             * dstCapacity on corrupt input (that is exactly why they are legacy),
             * so give them room instead of letting them smash the stack. */
            unsigned char *out = (unsigned char *)calloc(1, 1 << 20);
            size_t k, r;
            snprintf(nm, sizeof(nm), "%s_createDTable", pfx[i]); createDT = (void *(*)(unsigned))symopt(nm);
            snprintf(nm, sizeof(nm), "%s_freeDTable", pfx[i]);   freeDT = (void (*)(void*))symopt(nm);
            snprintf(nm, sizeof(nm), "%s_isError", pfx[i]);      iserr = (unsigned (*)(size_t))symopt(nm);
            snprintf(nm, sizeof(nm), "%s_getErrorName", pfx[i]); errname = (const char *(*)(size_t))symopt(nm);
            snprintf(nm, sizeof(nm), "%s_decompress", pfx[i]);   decomp = (size_t (*)(void*,size_t,const void*,size_t))symopt(nm);
            if (createDT && freeDT) {
                void *dt = createDT(9);
                P("LEGFSE %s dt=%d\n", pfx[i], dt != NULL);
                freeDT(dt);
            }
            if (iserr && errname)
                P("LEGFSE %s err=%u %s\n", pfx[i], iserr((size_t)-3), errname((size_t)-3));
            /* NOTE: feeding raw garbage to the *legacy* FSE decoders is undefined
             * behaviour in the original C (they can write past dstCapacity), so a
             * differential comparison there is meaningless; only the deterministic
             * entry points above are compared. */
            (void)decomp;
            free(out);
        }
    }


    /* ---------------- sequence-level APIs ---------------- */
    {
        typedef struct { unsigned offset, litLength, matchLength, rep; } zseq;
        size_t (*generateSequences)(void*,zseq*,size_t,const void*,size_t) =
            (size_t (*)(void*,zseq*,size_t,const void*,size_t))symopt("ZSTD_generateSequences");
        size_t (*mergeDelims)(zseq*,size_t) = (size_t (*)(zseq*,size_t))symopt("ZSTD_mergeBlockDelimiters");
        size_t (*compressSequences)(void*,void*,size_t,const zseq*,size_t,const void*,size_t) =
            (size_t (*)(void*,void*,size_t,const zseq*,size_t,const void*,size_t))symopt("ZSTD_compressSequences");
        size_t (*get1BlockSummaryRaw)(const zseq*,size_t) = NULL; /* returns struct, skip */
        size_t n = 60000;
        unsigned char *src = (unsigned char *)malloc(n);
        size_t cap = compressBound(n) + 4096;
        unsigned char *cbuf = (unsigned char *)malloc(cap);
        unsigned char *dbuf = (unsigned char *)malloc(n + 4096);
        size_t seqCap = sequenceBound(n);
        zseq *seqs = (zseq *)malloc(seqCap * sizeof(zseq));
        int level;
        (void)get1BlockSummaryRaw;
        fill_text(src, n, 606060);
        for (level = 1; level <= 19; level += 6) {
            void *cctx = createCCtx();
            size_t nbSeq;
            cctxReset(cctx, 3);
            cctxSetParameter(cctx, 100, level);
            cctxSetParameter(cctx, 1008 /* blockDelimiters */, 1);
            if (!generateSequences) break;
            nbSeq = generateSequences(cctx, seqs, seqCap, src, n);
            if (isError(nbSeq)) {
                P("SEQ lv%d gen ERR %s\n", level, getErrorName(nbSeq));
            } else {
                P("SEQ lv%d nbSeq=%zu dg=%016llx\n", level, nbSeq,
                  (unsigned long long)fnv(seqs, nbSeq * sizeof(zseq)));
                if (compressSequences) {
                    void *cctx2 = createCCtx();
                    size_t cs;
                    cctxReset(cctx2, 3);
                    cctxSetParameter(cctx2, 100, level);
                    cctxSetParameter(cctx2, 1008, 1);
                    cs = compressSequences(cctx2, cbuf, cap, seqs, nbSeq, src, n);
                    if (isError(cs)) P("SEQ lv%d cs ERR %s\n", level, getErrorName(cs));
                    else {
                        size_t ds;
                        P("SEQ lv%d cs=%zu %016llx\n", level, cs, (unsigned long long)fnv(cbuf, cs));
                        ds = zdecompress(dbuf, n + 4096, cbuf, cs);
                        P("SEQ lv%d ds=%zu ok=%d\n", level, ds,
                          !isError(ds) && ds == n && memcmp(dbuf, src, n) == 0);
                    }
                    freeCCtx(cctx2);
                }
                if (mergeDelims) {
                    size_t m = mergeDelims(seqs, nbSeq);
                    P("SEQ lv%d merged=%zu dg=%016llx\n", level, m,
                      (unsigned long long)fnv(seqs, (m > seqCap ? 0 : m) * sizeof(zseq)));
                }
            }
            freeCCtx(cctx);
        }
        free(src); free(cbuf); free(dbuf); free(seqs);
    }

    /* ---------------- static (in-place) contexts ---------------- */
    {
        void * (*initStaticCCtx)(void*,size_t) = (void *(*)(void*,size_t))symopt("ZSTD_initStaticCCtx");
        void * (*initStaticDCtx)(void*,size_t) = (void *(*)(void*,size_t))symopt("ZSTD_initStaticDCtx");
        void * (*initStaticCStream)(void*,size_t) = (void *(*)(void*,size_t))symopt("ZSTD_initStaticCStream");
        void * (*initStaticDStream)(void*,size_t) = (void *(*)(void*,size_t))symopt("ZSTD_initStaticDStream");
        /* ZSTD_compressionParameters { unsigned windowLog, chainLog, hashLog,
         *   searchLog, minMatch, targetLength; int strategy; }  (by value!) */
        struct cparams { unsigned windowLog, chainLog, hashLog, searchLog,
                         minMatch, targetLength; int strategy; };
        const void * (*initStaticCDict)(void*,size_t,const void*,size_t,int,int,struct cparams) =
            (const void *(*)(void*,size_t,const void*,size_t,int,int,struct cparams))symopt("ZSTD_initStaticCDict");
        struct cparams (*getCParams)(int,unsigned long long,size_t) =
            (struct cparams (*)(int,unsigned long long,size_t))symopt("ZSTD_getCParams");
        const void * (*initStaticDDict)(void*,size_t,const void*,size_t,int,int) =
            (const void *(*)(void*,size_t,const void*,size_t,int,int))symopt("ZSTD_initStaticDDict");
        size_t n = 20000, cap;
        unsigned char *src = (unsigned char *)malloc(n);
        unsigned char *cbuf, *dbuf;
        size_t wc = estimateCCtxSize(5), wd = estimateDCtxSize();
        void *wcbuf = malloc(wc + 64), *wdbuf = malloc(wd + 64);
        fill_text(src, n, 909090);
        cap = compressBound(n) + 1024;
        cbuf = (unsigned char *)malloc(cap);
        dbuf = (unsigned char *)malloc(n + 1024);
        P("STATIC sizes cctx=%zu dctx=%zu\n", wc, wd);
        if (initStaticCCtx && initStaticDCtx) {
            void *cctx = initStaticCCtx(wcbuf, wc);
            void *dctx = initStaticDCtx(wdbuf, wd);
            P("STATIC ctx=%d %d\n", cctx != NULL, dctx != NULL);
            if (cctx && dctx) {
                size_t cs = compressCCtx(cctx, cbuf, cap, src, n, 5);
                size_t ds;
                P("STATIC c=%zu %016llx\n", cs, (unsigned long long)fnv(cbuf, cs));
                ds = decompressDCtx(dctx, dbuf, n + 1024, cbuf, cs);
                P("STATIC d=%zu ok=%d\n", ds, !isError(ds) && ds == n && memcmp(dbuf, src, n) == 0);
            }
            /* too-small workspace must return NULL */
            P("STATIC toosmall %d %d\n", initStaticCCtx(wcbuf, 16) == NULL,
              initStaticDCtx(wdbuf, 16) == NULL);
        }
        if (initStaticCStream && initStaticDStream) {
            size_t sc = estimateCStreamSize(5), sd = estimateDStreamSize(1u << 17);
            void *b1 = malloc(sc + 64), *b2 = malloc(sd + 64);
            void *zcs = initStaticCStream(b1, sc);
            void *zds = initStaticDStream(b2, sd);
            P("STATIC streams %zu %zu %d %d\n", sc, sd, zcs != NULL, zds != NULL);
            free(b1); free(b2);
        }
        if (initStaticCDict && initStaticDDict) {
            unsigned char dictb[4096];
            size_t dl = 4096;
            size_t need;
            void *b;
            fill_text(dictb, dl, 121212);
            need = estimateCDictSize(dl, 5) + 4096;
            b = malloc(need);
            { struct cparams cp = getCParams(5, (unsigned long long)dl, dl);
              const void *cd;
              P("STATIC cparams %u %u %u %u %u %u %d\n", cp.windowLog, cp.chainLog,
                cp.hashLog, cp.searchLog, cp.minMatch, cp.targetLength, cp.strategy);
              cd = initStaticCDict(b, need, dictb, dl, 0 /*byCopy*/, 0 /*auto*/, cp);
              P("STATIC cdict %d\n", cd != NULL); }
            free(b);
            need = estimateDDictSize(dl, 0) + 1024;
            b = malloc(need);
            { const void *dd = initStaticDDict(b, need, dictb, dl, 0, 0);
              P("STATIC ddict %d\n", dd != NULL); }
            free(b);
        }
        free(src); free(cbuf); free(dbuf); free(wcbuf); free(wdbuf);
    }

    /* ---------------- block-level API + copyCCtx + prefixes ---------------- */
    {
        size_t (*compressBegin)(void*,int) = (size_t (*)(void*,int))symopt("ZSTD_compressBegin");
        size_t (*compressBlockF)(void*,void*,size_t,const void*,size_t) =
            (size_t (*)(void*,void*,size_t,const void*,size_t))symopt("ZSTD_compressBlock");
        size_t (*decompressBegin)(void*) = (size_t (*)(void*))symopt("ZSTD_decompressBegin");
        size_t (*decompressBlockF)(void*,void*,size_t,const void*,size_t) =
            (size_t (*)(void*,void*,size_t,const void*,size_t))symopt("ZSTD_decompressBlock");
        size_t (*insertBlock)(void*,const void*,size_t) =
            (size_t (*)(void*,const void*,size_t))symopt("ZSTD_insertBlock");
        size_t (*copyCCtxF)(void*,const void*,unsigned long long) =
            (size_t (*)(void*,const void*,unsigned long long))symopt("ZSTD_copyCCtx");
        size_t (*refPrefixC)(void*,const void*,size_t) =
            (size_t (*)(void*,const void*,size_t))symopt("ZSTD_CCtx_refPrefix");
        size_t (*refPrefixD)(void*,const void*,size_t) =
            (size_t (*)(void*,const void*,size_t))symopt("ZSTD_DCtx_refPrefix");
        size_t (*writeSkippable)(void*,size_t,const void*,size_t,unsigned) =
            (size_t (*)(void*,size_t,const void*,size_t,unsigned))symopt("ZSTD_writeSkippableFrame");
        size_t (*readSkippable)(void*,size_t,unsigned*,const void*,size_t) =
            (size_t (*)(void*,size_t,unsigned*,const void*,size_t))symopt("ZSTD_readSkippableFrame");
        unsigned (*isSkippable)(const void*,size_t) = (unsigned (*)(const void*,size_t))symopt("ZSTD_isSkippableFrame");
        size_t n = 3 * 65536;
        unsigned char *src = (unsigned char *)malloc(n);
        unsigned char *cbuf = (unsigned char *)malloc(n * 2 + 4096);
        unsigned char *dbuf = (unsigned char *)malloc(n + 4096);
        fill_text(src, n, 353535);
        if (compressBegin && compressBlockF && decompressBegin && decompressBlockF) {
            void *cctx = createCCtx();
            void *dctx = createDCtx();
            size_t bs, off = 0, doff = 0, i;
            compressBegin(cctx, 5);
            decompressBegin(dctx);
            bs = getBlockSize(cctx);
            P("BLOCK bs=%zu\n", bs);
            for (i = 0; i + bs <= n; i += bs) {
                size_t cs = compressBlockF(cctx, cbuf + off, n * 2 + 4096 - off, src + i, bs);
                if (isError(cs)) { P("BLOCK c ERR %s\n", getErrorName(cs)); break; }
                if (cs == 0) {
                    /* incompressible: caller must insert raw */
                    if (insertBlock) insertBlock(dctx, src + i, bs);
                    memcpy(dbuf + doff, src + i, bs);
                    doff += bs;
                    P("BLOCK i=%zu raw\n", i);
                } else {
                    size_t ds = decompressBlockF(dctx, dbuf + doff, n + 4096 - doff, cbuf + off, cs);
                    P("BLOCK i=%zu cs=%zu %016llx ds=%zu\n", i, cs,
                      (unsigned long long)fnv(cbuf + off, cs), ds);
                    if (isError(ds)) break;
                    doff += ds;
                    off += cs;
                }
            }
            P("BLOCK total d=%zu ok=%d\n", doff, doff <= n && memcmp(dbuf, src, doff) == 0);
            freeCCtx(cctx); freeDCtx(dctx);
        }
        if (copyCCtxF && compressBegin) {
            void *a = createCCtx(), *b = createCCtx();
            size_t r, cs1, cs2;
            compressBegin(a, 7);
            r = copyCCtxF(b, a, (unsigned long long)n);
            P("COPYCCTX r=%zu e=%u\n", r, isError(r));
            /* both should now produce identical output for the same input */
            cs1 = 0; cs2 = 0;
            freeCCtx(a); freeCCtx(b);
            (void)cs1; (void)cs2;
        }
        if (refPrefixC && refPrefixD) {
            void *cctx = createCCtx(), *dctx = createDCtx();
            size_t pn = 8000, dn = 20000, cs, ds;
            unsigned char *prefix = (unsigned char *)malloc(pn);
            unsigned char *data = (unsigned char *)malloc(dn);
            fill_text(prefix, pn, 47);
            fill_text(data, dn, 47);
            cctxReset(cctx, 3);
            cctxSetParameter(cctx, 100, 6);
            refPrefixC(cctx, prefix, pn);
            cs = compress2(cctx, cbuf, n * 2 + 4096, data, dn);
            P("PREFIX cs=%zu %016llx\n", cs, (unsigned long long)fnv(cbuf, cs));
            refPrefixD(dctx, prefix, pn);
            { typedef struct { const void *src; size_t size; size_t pos; } inB;
              typedef struct { void *dst; size_t size; size_t pos; } outB;
              outB out; inB in;
              out.dst = dbuf; out.size = n + 4096; out.pos = 0;
              in.src = cbuf; in.size = cs; in.pos = 0;
              ds = decompressStream(dctx, &out, &in);
              P("PREFIX ds=%zu out=%zu ok=%d\n", ds, out.pos,
                out.pos == dn && memcmp(dbuf, data, dn) == 0); }
            free(prefix); free(data);
            freeCCtx(cctx); freeDCtx(dctx);
        }
        if (writeSkippable && readSkippable && isSkippable) {
            unsigned char payload[64];
            unsigned magicVariant = 0;
            size_t r, rr;
            unsigned mv = 0;
            size_t i;
            for (i = 0; i < sizeof(payload); i++) payload[i] = (unsigned char)(i * 7);
            for (magicVariant = 0; magicVariant < 16; magicVariant += 5) {
                r = writeSkippable(cbuf, 1024, payload, sizeof(payload), magicVariant);
                P("SKIP mv=%u w=%zu %016llx isSkip=%u ffcs=%zu\n", magicVariant, r,
                  (unsigned long long)(isError(r) ? 0 : fnv(cbuf, r)),
                  isError(r) ? 0 : isSkippable(cbuf, r),
                  isError(r) ? 0 : findFrameCompressedSize(cbuf, r));
                if (isError(r)) continue;
                rr = readSkippable(dbuf, n + 4096, &mv, cbuf, r);
                P("SKIP mv=%u read=%zu mv_out=%u ok=%d\n", magicVariant, rr, mv,
                  !isError(rr) && rr == sizeof(payload) && memcmp(dbuf, payload, rr) == 0);
            }
            /* skippable frame followed by a real frame */
            { size_t sk = writeSkippable(cbuf, 1024, payload, 16, 3);
              size_t cs = zcompress(cbuf + sk, n * 2 + 4096 - sk, src, 5000, 3);
              size_t tot = sk + cs, ds;
              P("SKIP+FRAME tot=%zu fds=%llu dbound=%llu\n", tot,
                (unsigned long long)findDecompressedSize(cbuf, tot),
                (unsigned long long)decompressBound(cbuf, tot));
              ds = zdecompress(dbuf, n + 4096, cbuf, tot);
              P("SKIP+FRAME ds=%zu ok=%d\n", ds, !isError(ds) && ds == 5000); }
        }
        /* multi-frame concatenation */
        {
            size_t a = zcompress(cbuf, n * 2, src, 10000, 3);
            size_t b = zcompress(cbuf + a, n * 2 - a, src + 10000, 20000, 9);
            size_t tot = a + b, ds;
            P("MULTI a=%zu b=%zu fds=%llu dbound=%llu\n", a, b,
              (unsigned long long)findDecompressedSize(cbuf, tot),
              (unsigned long long)decompressBound(cbuf, tot));
            ds = zdecompress(dbuf, n + 4096, cbuf, tot);
            P("MULTI ds=%zu ok=%d\n", ds, !isError(ds) && ds == 30000 &&
              memcmp(dbuf, src, 30000) == 0);
        }
        free(src); free(cbuf); free(dbuf);
    }

    /* ---------------- frame header struct decode ---------------- */
    {
        /* ZSTD_FrameHeader { U64 fcs; U64 windowSize; unsigned blockSizeMax;
         *   int frameType; unsigned headerSize; unsigned dictID; unsigned checksumFlag;
         *   unsigned _r1; unsigned _r2; } */
        struct fh { unsigned long long fcs, windowSize; unsigned blockSizeMax;
                    int frameType; unsigned headerSize, dictID, checksumFlag, r1, r2; };
        size_t (*getFrameHeader)(struct fh*,const void*,size_t) =
            (size_t (*)(struct fh*,const void*,size_t))symopt("ZSTD_getFrameHeader");
        size_t (*getFrameHeaderAdv)(struct fh*,const void*,size_t,int) =
            (size_t (*)(struct fh*,const void*,size_t,int))symopt("ZSTD_getFrameHeader_advanced");
        size_t n = 50000;
        unsigned char *src = (unsigned char *)malloc(n);
        unsigned char *cbuf = (unsigned char *)malloc(compressBound(n) + 1024);
        int level, cf, ct;
        fill_text(src, n, 767676);
        if (getFrameHeader) {
            for (level = 1; level <= 19; level += 9)
              for (cf = 0; cf <= 1; cf++)
                for (ct = 0; ct <= 1; ct++) {
                    void *cctx = createCCtx();
                    size_t cs, r, k;
                    struct fh h;
                    cctxReset(cctx, 3);
                    cctxSetParameter(cctx, 100, level);
                    cctxSetParameter(cctx, 201, cf);
                    cctxSetParameter(cctx, 200, ct);
                    cs = compress2(cctx, cbuf, compressBound(n) + 1024, src, n);
                    memset(&h, 0, sizeof(h));
                    r = getFrameHeader(&h, cbuf, cs);
                    P("FH lv%d cf%d ct%d r=%zu fcs=%llu ws=%llu bsm=%u ft=%d hs=%u id=%u ck=%u\n",
                      level, cf, ct, r, h.fcs, h.windowSize, h.blockSizeMax, h.frameType,
                      h.headerSize, h.dictID, h.checksumFlag);
                    /* partial inputs must ask for more */
                    for (k = 1; k <= 18 && k <= cs; k++) {
                        memset(&h, 0, sizeof(h));
                        r = getFrameHeader(&h, cbuf, k);
                        P("FH  partial %zu -> %zu\n", k, r);
                    }
                    if (getFrameHeaderAdv) {
                        memset(&h, 0, sizeof(h));
                        r = getFrameHeaderAdv(&h, cbuf, cs, 1 /* magicless */);
                        P("FH  magicless r=%zu\n", r);
                    }
                    freeCCtx(cctx);
                }
        }
        free(src); free(cbuf);
    }


    /* ---------------- large inputs: LDM, overflow correction, many blocks ------- */
    {
        size_t n = 6u << 20;   /* 6 MB */
        unsigned char *src = (unsigned char *)malloc(n);
        size_t cap = compressBound(n) + 8192;
        unsigned char *cbuf = (unsigned char *)malloc(cap);
        unsigned char *dbuf = (unsigned char *)malloc(n + 8192);
        int level;
        size_t i;
        /* highly repetitive with long-range repeats -> exercises LDM hard */
        fill_text(src, n / 3, 111);
        memcpy(src + n / 3, src, n / 3);                 /* exact long-range repeat */
        fill_mixed(src + 2 * (n / 3), n - 2 * (n / 3), 222);
        for (i = 0; i + 4096 < n; i += 500000) memcpy(src + i, src, 4096);
        for (level = 1; level <= 19; level += 6) {
            int ldm;
            for (ldm = 0; ldm <= 1; ldm++) {
                void *cctx = createCCtx();
                size_t cs, ds;
                cctxReset(cctx, 3);
                cctxSetParameter(cctx, 100, level);
                cctxSetParameter(cctx, 160 /* enableLongDistanceMatching */, ldm ? 1 : 2);
                cs = compress2(cctx, cbuf, cap, src, n);
                if (isError(cs)) { P("BIG lv%d ldm%d CERR %s\n", level, ldm, getErrorName(cs)); freeCCtx(cctx); continue; }
                P("BIG lv%d ldm%d c=%zu %016llx\n", level, ldm, cs, (unsigned long long)fnv(cbuf, cs));
                ds = zdecompress(dbuf, n + 8192, cbuf, cs);
                P("BIG lv%d ldm%d d=%zu ok=%d\n", level, ldm, ds,
                  !isError(ds) && ds == n && memcmp(dbuf, src, n) == 0);
                freeCCtx(cctx);
            }
        }
        /* small windowLog on big input -> extDict / window sliding paths */
        for (level = 1; level <= 19; level += 9) {
            int wl;
            for (wl = 10; wl <= 20; wl += 5) {
                void *cctx = createCCtx();
                size_t cs, ds;
                cctxReset(cctx, 3);
                cctxSetParameter(cctx, 100, level);
                cctxSetParameter(cctx, 101 /* windowLog */, wl);
                cs = compress2(cctx, cbuf, cap, src, n);
                if (isError(cs)) { P("WIN lv%d wl%d CERR %s\n", level, wl, getErrorName(cs)); freeCCtx(cctx); continue; }
                P("WIN lv%d wl%d c=%zu %016llx\n", level, wl, cs, (unsigned long long)fnv(cbuf, cs));
                { void *dctx = createDCtx();
                  size_t r = dlsym(H, "ZSTD_DCtx_setParameter") ?
                      ((size_t (*)(void*,int,int))sym("ZSTD_DCtx_setParameter"))(dctx, 100, 27) : 0;
                  (void)r;
                  ds = decompressDCtx(dctx, dbuf, n + 8192, cbuf, cs);
                  freeDCtx(dctx); }
                P("WIN lv%d wl%d d=%zu ok=%d\n", level, wl, ds,
                  !isError(ds) && ds == n && memcmp(dbuf, src, n) == 0);
                freeCCtx(cctx);
            }
        }
        /* streaming over the big input with small buffers -> buffered paths */
        {
            typedef struct { const void *src; size_t size; size_t pos; } inB;
            typedef struct { void *dst; size_t size; size_t pos; } outB;
            void *zcs = createCStream();
            void *zds = createDStream();
            outB out; inB in;
            size_t produced, r;
            initCStream(zcs, 6);
            out.dst = cbuf; out.size = cap; out.pos = 0;
            for (i = 0; i < n; i += 9973) {
                size_t take = 9973; if (i + take > n) take = n - i;
                in.src = src + i; in.size = take; in.pos = 0;
                while (in.pos < in.size) { r = compressStream(zcs, &out, &in); if (isError(r)) break; }
            }
            do { r = endStream(zcs, &out); } while (r != 0 && !isError(r));
            produced = out.pos;
            P("BIGS c=%zu %016llx\n", produced, (unsigned long long)fnv(cbuf, produced));
            initDStream(zds);
            out.dst = dbuf; out.size = n + 8192; out.pos = 0;
            for (i = 0; i < produced; i += 7919) {
                size_t take = 7919; if (i + take > produced) take = produced - i;
                in.src = cbuf + i; in.size = take; in.pos = 0;
                while (in.pos < in.size) { r = decompressStream(zds, &out, &in); if (isError(r)) break; }
            }
            P("BIGS d=%zu ok=%d\n", out.pos, out.pos == n && memcmp(dbuf, src, n) == 0);
            freeCStream(zcs); freeDStream(zds);
        }
        free(src); free(cbuf); free(dbuf);
    }

    /* ---------------- dictionary x level x strategy matrix ---------------- */
    {
        size_t nbSamples = 96, sampleSize = 2500;
        size_t total = nbSamples * sampleSize;
        unsigned char *samples = (unsigned char *)malloc(total);
        size_t *sizes = (size_t *)malloc(nbSamples * sizeof(size_t));
        unsigned char *dict = (unsigned char *)malloc(65536);
        size_t i, dsz;
        for (i = 0; i < nbSamples; i++) {
            fill_text(samples + i * sampleSize, sampleSize, 2024 + i * 5);
            sizes[i] = sampleSize;
        }
        dsz = zdictTrain(dict, 65536, samples, sizes, (unsigned)nbSamples);
        P("DICT2 train=%zu err=%u\n", dsz, zdictIsError(dsz));
        if (!zdictIsError(dsz)) {
            static const size_t dsizes[] = { 0, 1, 100, 5000, 60000, 200000 };
            size_t si;
            unsigned char *src = (unsigned char *)malloc(200000);
            unsigned char *cbuf = (unsigned char *)malloc(compressBound(200000) + 4096);
            unsigned char *dbuf = (unsigned char *)malloc(200000 + 4096);
            int level;
            P("DICT2 dict=%016llx id=%u\n", (unsigned long long)fnv(dict, dsz),
              zdictGetDictID(dict, dsz));
            for (si = 0; si < sizeof(dsizes)/sizeof(dsizes[0]); si++) {
                size_t sn = dsizes[si];
                fill_text(src, sn, 4004 + si);
                for (level = 1; level <= 22; level += 7) {
                    void *cd = createCDict(dict, dsz, level);
                    void *dd = createDDict(dict, dsz);
                    void *cctx = createCCtx(), *dctx = createDCtx();
                    size_t cs = compress_usingCDict(cctx, cbuf, compressBound(200000) + 4096, src, sn, cd);
                    size_t ds;
                    P("DICT2 n=%zu lv%d c=%zu %016llx\n", sn, level, cs,
                      (unsigned long long)(isError(cs) ? 0 : fnv(cbuf, cs)));
                    if (!isError(cs)) {
                        ds = decompress_usingDDict(dctx, dbuf, 200000 + 4096, cbuf, cs, dd);
                        P("DICT2 n=%zu lv%d d=%zu ok=%d\n", sn, level, ds,
                          !isError(ds) && ds == sn && (sn == 0 || memcmp(dbuf, src, sn) == 0));
                    }
                    freeCCtx(cctx); freeDCtx(dctx); freeCDict(cd); freeDDict(dd);
                }
            }
            /* dictionary + streaming */
            {
                typedef struct { const void *src; size_t size; size_t pos; } inB;
                typedef struct { void *dst; size_t size; size_t pos; } outB;
                size_t (*ccRefCDict)(void*,const void*) = (size_t (*)(void*,const void*))symopt("ZSTD_CCtx_refCDict");
                size_t (*dcRefDDict)(void*,const void*) = (size_t (*)(void*,const void*))symopt("ZSTD_DCtx_refDDict");
                if (ccRefCDict && dcRefDDict) {
                    void *cd = createCDict(dict, dsz, 8);
                    void *dd = createDDict(dict, dsz);
                    void *cctx = createCCtx(), *dctx = createDCtx();
                    size_t sn = 90000, r;
                    outB out; inB in;
                    fill_text(src, sn, 5150);
                    cctxReset(cctx, 3);
                    ccRefCDict(cctx, cd);
                    out.dst = cbuf; out.size = compressBound(200000) + 4096; out.pos = 0;
                    in.src = src; in.size = sn; in.pos = 0;
                    do { r = compressStream2(cctx, &out, &in, 2 /*end*/); } while (r != 0 && !isError(r));
                    P("DICTS c=%zu %016llx\n", out.pos, (unsigned long long)fnv(cbuf, out.pos));
                    dcRefDDict(dctx, dd);
                    { size_t cs = out.pos;
                      out.dst = dbuf; out.size = 200000 + 4096; out.pos = 0;
                      in.src = cbuf; in.size = cs; in.pos = 0;
                      r = decompressStream(dctx, &out, &in);
                      P("DICTS d=%zu r=%zu ok=%d\n", out.pos, r,
                        out.pos == sn && memcmp(dbuf, src, sn) == 0); }
                    freeCCtx(cctx); freeDCtx(dctx); freeCDict(cd); freeDDict(dd);
                }
            }
            free(src); free(cbuf); free(dbuf);
        }
        free(samples); free(sizes); free(dict);
    }

    /* ---------------- decompressor parameters ---------------- */
    {
        size_t (*dctxSetParameter)(void*,int,int) = (size_t (*)(void*,int,int))symopt("ZSTD_DCtx_setParameter");
        size_t (*dctxGetParameter)(void*,int,int*) = (size_t (*)(void*,int,int*))symopt("ZSTD_DCtx_getParameter");
        size_t (*dctxReset)(void*,int) = (size_t (*)(void*,int))symopt("ZSTD_DCtx_reset");
        size_t (*dctxSetMaxWindowSize)(void*,size_t) = (size_t (*)(void*,size_t))symopt("ZSTD_DCtx_setMaxWindowSize");
        size_t (*dctxSetFormat)(void*,int) = (size_t (*)(void*,int))symopt("ZSTD_DCtx_setFormat");
        size_t n = 80000;
        unsigned char *src = (unsigned char *)malloc(n);
        unsigned char *cbuf = (unsigned char *)malloc(compressBound(n) + 1024);
        unsigned char *dbuf = (unsigned char *)malloc(n + 1024);
        int dparams[] = { 100, 1000, 1001, 1002, 1003, 1004, 1005 };
        int i, j;
        fill_mixed(src, n, 838383);
        if (dctxSetParameter && dctxGetParameter) {
            for (i = 0; i < 7; i++) {
                for (j = 0; j <= 3; j++) {
                    void *dctx = createDCtx();
                    size_t r, cs, ds;
                    int got = -12345;
                    if (dctxReset) dctxReset(dctx, 3);
                    r = dctxSetParameter(dctx, dparams[i], j);
                    P("DP %d=%d set r=%zu e=%u %s\n", dparams[i], j, r, isError(r),
                      isError(r) ? getErrorName(r) : "ok");
                    { size_t g = dctxGetParameter(dctx, dparams[i], &got);
                      P("DP %d get r=%zu v=%d\n", dparams[i], g, got); }
                    cs = zcompress(cbuf, compressBound(n) + 1024, src, n, 5);
                    ds = decompressDCtx(dctx, dbuf, n + 1024, cbuf, cs);
                    P("DP %d=%d d=%zu ok=%d\n", dparams[i], j, ds,
                      !isError(ds) && ds == n && memcmp(dbuf, src, n) == 0);
                    freeDCtx(dctx);
                }
            }
        }
        if (dctxSetMaxWindowSize) {
            size_t ws[4] = { 1024, 1u << 17, 1u << 22, 1u << 27 };
            for (i = 0; i < 4; i++) {
                void *dctx = createDCtx();
                size_t r = dctxSetMaxWindowSize(dctx, ws[i]);
                size_t cs = zcompress(cbuf, compressBound(n) + 1024, src, n, 5);
                size_t ds = decompressDCtx(dctx, dbuf, n + 1024, cbuf, cs);
                P("DWS %zu r=%zu ds=%zu e=%u\n", ws[i], r, ds, isError(ds));
                freeDCtx(dctx);
            }
        }
        if (dctxSetFormat) {
            for (i = 0; i <= 1; i++) {
                void *dctx = createDCtx();
                void *cctx = createCCtx();
                size_t cs, ds;
                cctxReset(cctx, 3);
                cctxSetParameter(cctx, 10 /* format */, i);
                cs = compress2(cctx, cbuf, compressBound(n) + 1024, src, n);
                dctxSetFormat(dctx, i);
                ds = decompressDCtx(dctx, dbuf, n + 1024, cbuf, cs);
                P("DFMT %d cs=%zu %016llx ds=%zu ok=%d\n", i, cs,
                  (unsigned long long)fnv(cbuf, cs), ds,
                  !isError(ds) && ds == n && memcmp(dbuf, src, n) == 0);
                freeDCtx(dctx); freeCCtx(cctx);
            }
        }
        free(src); free(cbuf); free(dbuf);
    }

    /* ---------------- CCtx_getParameter sweep ---------------- */
    {
        int params[] = { 100,101,102,103,104,105,106,107,130,160,161,162,163,164,
                         200,201,202,400,401,402,
                         500,10,1000,1001,1002,1004,1005,1006,1007,1008,1009,
                         1010,1011,1012,1013,1014,1015,1016,1017 };
        void *cctx = createCCtx();
        int i, lv;
        for (lv = 1; lv <= 19; lv += 9) {
            cctxReset(cctx, 3);
            cctxSetParameter(cctx, 100, lv);
            for (i = 0; i < (int)(sizeof(params)/sizeof(params[0])); i++) {
                int v = -999999;
                size_t r = ((size_t (*)(const void*,int,int*))sym("ZSTD_CCtx_getParameter"))(cctx, params[i], &v);
                P("GP lv%d %d r=%zu v=%d e=%u\n", lv, params[i], r, v, isError(r));
            }
        }
        freeCCtx(cctx);
        /* cParam bounds for every parameter */
        {
            /* ZSTD_bounds { size_t error; int lowerBound; int upperBound; } */
            struct bnd { size_t error; int lo; int hi; };
            struct bnd (*cbounds)(int) = (struct bnd (*)(int))sym("ZSTD_cParam_getBounds");
            struct bnd (*dbounds)(int) = (struct bnd (*)(int))sym("ZSTD_dParam_getBounds");
            int dp[] = { 100, 1000, 1001, 1002, 1003, 1004, 1005, 999 };
            for (i = 0; i < (int)(sizeof(params)/sizeof(params[0])); i++) {
                struct bnd b = cbounds(params[i]);
                P("CB %d err=%zu lo=%d hi=%d\n", params[i], b.error, b.lo, b.hi);
            }
            for (i = 0; i < 8; i++) {
                struct bnd b = dbounds(dp[i]);
                P("DB %d err=%zu lo=%d hi=%d\n", dp[i], b.error, b.lo, b.hi);
            }
        }
    }

    /* ---------------- extra corruption fuzz on dictionary frames ---------------- */
    {
        size_t nbSamples = 40, sampleSize = 2000;
        unsigned char *samples = (unsigned char *)malloc(nbSamples * sampleSize);
        size_t *sizes = (size_t *)malloc(nbSamples * sizeof(size_t));
        unsigned char *dict = (unsigned char *)malloc(32768);
        size_t i, dsz;
        for (i = 0; i < nbSamples; i++) {
            fill_text(samples + i * sampleSize, sampleSize, 616 + i);
            sizes[i] = sampleSize;
        }
        dsz = zdictTrain(dict, 32768, samples, sizes, (unsigned)nbSamples);
        if (!zdictIsError(dsz)) {
            size_t n = 30000;
            unsigned char *src = (unsigned char *)malloc(n);
            size_t cap = compressBound(n) + 1024;
            unsigned char *cbuf = (unsigned char *)malloc(cap);
            unsigned char *mut = (unsigned char *)malloc(cap);
            unsigned char *dbuf = (unsigned char *)malloc(n + 4096);
            void *cd = createCDict(dict, dsz, 9);
            void *dd = createDDict(dict, dsz);
            void *cctx = createCCtx();
            size_t cs, it;
            fill_text(src, n, 727);
            cs = compress_usingCDict(cctx, cbuf, cap, src, n, cd);
            P("DFUZZ base cs=%zu\n", cs);
            rseed(0xD1C7ull);
            for (it = 0; it < 150; it++) {
                size_t trunc, k, nflips, r;
                void *dctx = createDCtx();
                memcpy(mut, cbuf, cs);
                trunc = 1 + (rnext() % cs);
                nflips = rnext() % 4;
                for (k = 0; k < nflips; k++) {
                    size_t off;
                    if (trunc <= 4) break;
                    off = 4 + (rnext() % (trunc - 4));
                    mut[off] = (unsigned char)(mut[off] ^ (1u << (rnext() % 8)));
                }
                r = decompress_usingDDict(dctx, dbuf, n + 4096, mut, trunc, dd);
                P("DFUZZ it%zu t=%zu f=%zu r=%zu e=%u dg=%016llx\n", it, trunc, nflips, r,
                  isError(r), (unsigned long long)(isError(r) ? 0 : fnv(dbuf, r)));
                freeDCtx(dctx);
            }
            freeCCtx(cctx); freeCDict(cd); freeDDict(dd);
            free(src); free(cbuf); free(mut); free(dbuf);
        }
        free(samples); free(sizes); free(dict);
    }

    fflush(stdout);
    dlclose(H);
    return 0;
}
