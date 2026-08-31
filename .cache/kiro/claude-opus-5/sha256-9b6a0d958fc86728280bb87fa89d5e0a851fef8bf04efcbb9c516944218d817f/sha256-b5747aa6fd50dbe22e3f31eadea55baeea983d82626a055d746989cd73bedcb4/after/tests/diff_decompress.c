/* End-to-end differential harness for the decompression API.
 *
 * The reference C library compresses (and builds dictionaries); the library
 * under test decompresses. Every observable result is printed so the two runs
 * can be diffed byte for byte. */
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

static unsigned long long rs = 0x9E3779B97F4A7C15ULL;
static unsigned rnd(void) { rs ^= rs << 13; rs ^= rs >> 7; rs ^= rs << 17; return (unsigned)(rs >> 24); }

/* ---- reference (compressor) ---- */
static size_t (*r_compress)(void *, size_t, const void *, size_t, int);
static size_t (*r_compressBound)(size_t);
static size_t (*r_isError)(size_t);
static void *(*r_createCCtx)(void);
static size_t (*r_freeCCtx)(void *);
static size_t (*r_compress_usingDict)(void *, void *, size_t, const void *, size_t, const void *, size_t, int);
static size_t (*r_CCtx_setParameter)(void *, int, int);
static size_t (*r_compress2)(void *, void *, size_t, const void *, size_t);

/* ---- library under test (decompressor) ---- */
static size_t (*t_decompress)(void *, size_t, const void *, size_t);
static size_t (*t_isError)(size_t);
static const char *(*t_getErrorName)(size_t);
static unsigned long long (*t_getFrameContentSize)(const void *, size_t);
static unsigned long long (*t_findDecompressedSize)(const void *, size_t);
static unsigned long long (*t_decompressBound)(const void *, size_t);
static size_t (*t_findFrameCompressedSize)(const void *, size_t);
static size_t (*t_frameHeaderSize)(const void *, size_t);
static unsigned (*t_isFrame)(const void *, size_t);
static unsigned (*t_isSkippableFrame)(const void *, size_t);
static unsigned (*t_getDictID_fromFrame)(const void *, size_t);
static unsigned (*t_getDictID_fromDict)(const void *, size_t);
static size_t (*t_getFrameHeader)(void *, const void *, size_t);
static size_t (*t_decompressionMargin)(const void *, size_t);
static void *(*t_createDCtx)(void);
static size_t (*t_freeDCtx)(void *);
static size_t (*t_decompressDCtx)(void *, void *, size_t, const void *, size_t);
static size_t (*t_decompress_usingDict)(void *, void *, size_t, const void *, size_t, const void *, size_t);
static void *(*t_createDDict)(const void *, size_t);
static size_t (*t_freeDDict)(void *);
static size_t (*t_decompress_usingDDict)(void *, void *, size_t, const void *, size_t, const void *);
static unsigned (*t_getDictID_fromDDict)(const void *);
static size_t (*t_sizeof_DDict)(const void *);
static size_t (*t_estimateDDictSize)(size_t, int);
static size_t (*t_sizeof_DCtx)(const void *);
static size_t (*t_estimateDCtxSize)(void);
static size_t (*t_DCtx_reset)(void *, int);
static size_t (*t_DCtx_setParameter)(void *, int, int);
static size_t (*t_DCtx_getParameter)(void *, int, int *);
static size_t (*t_decompressStream)(void *, void *, void *);
static size_t (*t_initDStream)(void *);
static size_t (*t_DStreamInSize)(void);
static size_t (*t_DStreamOutSize)(void);
static size_t (*t_decodingBufferSize_min)(unsigned long long, size_t);
static size_t (*t_estimateDStreamSize)(size_t);
static size_t (*t_estimateDStreamSize_fromFrame)(const void *, size_t);
static size_t (*t_nextSrcSizeToDecompress)(void *);
static int (*t_nextInputType)(void *);
static size_t (*t_decompressBegin)(void *);
static size_t (*t_decompressContinue)(void *, void *, size_t, const void *, size_t);
static size_t (*t_readSkippableFrame)(void *, size_t, unsigned *, const void *, size_t);
static size_t (*t_DCtx_refDDict)(void *, const void *);
static size_t (*t_DCtx_loadDictionary)(void *, const void *, size_t);
static size_t (*t_DCtx_refPrefix)(void *, const void *, size_t);
static void *(*t_dParam_getBounds_ret)(void);

typedef struct { const void *src; size_t size; size_t pos; } inBuf;
typedef struct { void *dst; size_t size; size_t pos; } outBuf;

#define MAXSRC (600 * 1024)
static unsigned char *src, *cbuf, *dbuf;
static size_t cbufCap;

static void hash_report(const char *tag, const unsigned char *p, size_t n)
{
    unsigned long long h = 1469598103934665603ULL;
    for (size_t i = 0; i < n; i++) { h ^= p[i]; h *= 1099511628211ULL; }
    printf("%s len=%zu fnv=%llu\n", tag, n, h);
}

/* Build a variety of inputs with different compressibility characteristics. */
static size_t gen(int mode, size_t L)
{
    switch (mode) {
    case 0: for (size_t i = 0; i < L; i++) src[i] = 0; break;                    /* all zeros -> RLE */
    case 1: for (size_t i = 0; i < L; i++) src[i] = rnd() & 0xFF; break;         /* incompressible */
    case 2: for (size_t i = 0; i < L; i++) src[i] = "abcdefgh"[i & 7]; break;    /* periodic */
    case 3: for (size_t i = 0; i < L; i++) src[i] = (rnd() & 0xFFFF) < 60000 ? 'x' : (rnd() & 0xFF); break;
    case 4: { const char *w[] = {"the ","quick ","brown ","fox ","jumps ","over ","lazy ","dog "};
              size_t p = 0; while (p < L) { const char *s = w[rnd() & 7]; size_t n = strlen(s);
              if (p + n > L) n = L - p; memcpy(src + p, s, n); p += n; } } break;
    case 5: for (size_t i = 0; i < L; i++) src[i] = (unsigned char)(i * 31 + (i >> 8)); break;
    case 6: /* long matches at long distances */
        for (size_t i = 0; i < L; i++) src[i] = rnd() & 0x0F;
        if (L > 200000) memcpy(src + L - 100000, src, 100000);
        break;
    default: for (size_t i = 0; i < L; i++) src[i] = (rnd() % 100) < 97 ? 'A' : (rnd() & 0xFF); break;
    }
    return L;
}

int main(int argc, char **argv)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    if (argc < 2) { fprintf(stderr, "usage: %s <libzstd.so>\n", argv[0]); return 1; }
    R = dlopen(REF_PATH, RTLD_NOW | RTLD_LOCAL);
    T = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!R || !T) { fprintf(stderr, "dlopen: %s\n", dlerror()); return 1; }

    r_compress = rsym("ZSTD_compress");
    r_compressBound = rsym("ZSTD_compressBound");
    r_isError = rsym("ZSTD_isError");
    r_createCCtx = rsym("ZSTD_createCCtx");
    r_freeCCtx = rsym("ZSTD_freeCCtx");
    r_compress_usingDict = rsym("ZSTD_compress_usingDict");
    r_CCtx_setParameter = rsym("ZSTD_CCtx_setParameter");
    r_compress2 = rsym("ZSTD_compress2");

    t_decompress = tsym("ZSTD_decompress");
    t_isError = tsym("ZSTD_isError");
    t_getErrorName = tsym("ZSTD_getErrorName");
    t_getFrameContentSize = tsym("ZSTD_getFrameContentSize");
    t_findDecompressedSize = tsym("ZSTD_findDecompressedSize");
    t_decompressBound = tsym("ZSTD_decompressBound");
    t_findFrameCompressedSize = tsym("ZSTD_findFrameCompressedSize");
    t_frameHeaderSize = tsym("ZSTD_frameHeaderSize");
    t_isFrame = tsym("ZSTD_isFrame");
    t_isSkippableFrame = tsym("ZSTD_isSkippableFrame");
    t_getDictID_fromFrame = tsym("ZSTD_getDictID_fromFrame");
    t_getDictID_fromDict = tsym("ZSTD_getDictID_fromDict");
    t_getFrameHeader = tsym("ZSTD_getFrameHeader");
    t_decompressionMargin = tsym("ZSTD_decompressionMargin");
    t_createDCtx = tsym("ZSTD_createDCtx");
    t_freeDCtx = tsym("ZSTD_freeDCtx");
    t_decompressDCtx = tsym("ZSTD_decompressDCtx");
    t_decompress_usingDict = tsym("ZSTD_decompress_usingDict");
    t_createDDict = tsym("ZSTD_createDDict");
    t_freeDDict = tsym("ZSTD_freeDDict");
    t_decompress_usingDDict = tsym("ZSTD_decompress_usingDDict");
    t_getDictID_fromDDict = tsym("ZSTD_getDictID_fromDDict");
    t_sizeof_DDict = tsym("ZSTD_sizeof_DDict");
    t_estimateDDictSize = tsym("ZSTD_estimateDDictSize");
    t_sizeof_DCtx = tsym("ZSTD_sizeof_DCtx");
    t_estimateDCtxSize = tsym("ZSTD_estimateDCtxSize");
    t_DCtx_reset = tsym("ZSTD_DCtx_reset");
    t_DCtx_setParameter = tsym("ZSTD_DCtx_setParameter");
    t_DCtx_getParameter = tsym("ZSTD_DCtx_getParameter");
    t_decompressStream = tsym("ZSTD_decompressStream");
    t_initDStream = tsym("ZSTD_initDStream");
    t_DStreamInSize = tsym("ZSTD_DStreamInSize");
    t_DStreamOutSize = tsym("ZSTD_DStreamOutSize");
    t_decodingBufferSize_min = tsym("ZSTD_decodingBufferSize_min");
    t_estimateDStreamSize = tsym("ZSTD_estimateDStreamSize");
    t_estimateDStreamSize_fromFrame = tsym("ZSTD_estimateDStreamSize_fromFrame");
    t_nextSrcSizeToDecompress = tsym("ZSTD_nextSrcSizeToDecompress");
    t_nextInputType = tsym("ZSTD_nextInputType");
    t_decompressBegin = tsym("ZSTD_decompressBegin");
    t_decompressContinue = tsym("ZSTD_decompressContinue");
    t_readSkippableFrame = tsym("ZSTD_readSkippableFrame");
    t_DCtx_refDDict = tsym("ZSTD_DCtx_refDDict");
    t_DCtx_loadDictionary = tsym("ZSTD_DCtx_loadDictionary");
    t_DCtx_refPrefix = tsym("ZSTD_DCtx_refPrefix");

    src = malloc(MAXSRC);
    cbufCap = r_compressBound(MAXSRC) + 4096;
    cbuf = malloc(cbufCap);
    dbuf = malloc(MAXSRC + 4096);

    printf("sizeof_DCtx_estimate=%zu\n", t_estimateDCtxSize());
    printf("DStreamInSize=%zu DStreamOutSize=%zu\n", t_DStreamInSize(), t_DStreamOutSize());
    for (unsigned long long cs = 0; cs < 300000; cs += 65536)
        for (size_t bs = 1024; bs <= 131072; bs *= 4)
            printf("decodingBufferSize_min(%llu,%zu)=%zu\n", cs, bs, t_decodingBufferSize_min(cs, bs));
    for (size_t w = 0; w < 32; w += 3)
        printf("estimateDStreamSize(%zu)=%zu\n", (size_t)1 << w, t_estimateDStreamSize((size_t)1 << w));
    printf("estimateDDictSize(1000,byCopy)=%zu byRef=%zu\n",
           t_estimateDDictSize(1000, 0), t_estimateDDictSize(1000, 1));

    size_t lens[] = {0, 1, 2, 3, 7, 63, 64, 100, 1000, 8000, 70000, 200000, 550000};
    int levels[] = {-5, -1, 1, 3, 5, 9, 12, 17, 19, 22};

    for (int mode = 0; mode < 8; mode++) {
        for (unsigned li = 0; li < sizeof(lens)/sizeof(lens[0]); li++) {
            size_t L = lens[li];
            if (L > MAXSRC) continue;
            gen(mode, L);
            for (unsigned vi = 0; vi < sizeof(levels)/sizeof(levels[0]); vi++) {
                int lvl = levels[vi];
                size_t c = r_compress(cbuf, cbufCap, src, L, lvl);
                if (r_isError(c)) { printf("m=%d L=%zu lvl=%d cErr\n", mode, L, lvl); continue; }

                /* --- one-shot --- */
                memset(dbuf, 0xAB, L + 16);
                size_t d = t_decompress(dbuf, L + 16, cbuf, c);
                printf("m=%d L=%zu lvl=%d c=%zu d=%zu ok=%d\n", mode, L, lvl, c, d,
                       (!t_isError(d) && d == L && (L == 0 || !memcmp(dbuf, src, L))));
                if (!t_isError(d)) hash_report("  oneshot", dbuf, d);
                else printf("  err=%s\n", t_getErrorName(d));

                /* --- frame introspection --- */
                printf("  fcs=%llu fds=%llu dbound=%llu ffcs=%zu fhs=%zu isFrame=%u isSkip=%u did=%u margin=%zu\n",
                       t_getFrameContentSize(cbuf, c), t_findDecompressedSize(cbuf, c),
                       t_decompressBound(cbuf, c), t_findFrameCompressedSize(cbuf, c),
                       t_frameHeaderSize(cbuf, c), t_isFrame(cbuf, c), t_isSkippableFrame(cbuf, c),
                       t_getDictID_fromFrame(cbuf, c), t_decompressionMargin(cbuf, c));
                {   unsigned char fh[64];
                    memset(fh, 0, sizeof(fh));
                    size_t r = t_getFrameHeader(fh, cbuf, c);
                    printf("  getFrameHeader=%zu ", r);
                    for (unsigned i = 0; i < 48; i++) printf("%02x", fh[i]);
                    printf("\n");
                }

                /* --- exact-size and undersized dst --- */
                memset(dbuf, 0xCD, L + 16);
                size_t de = t_decompress(dbuf, L, cbuf, c);
                printf("  exact=%zu ok=%d\n", de, (!t_isError(de) && de == L));
                if (L > 1) {
                    size_t du = t_decompress(dbuf, L - 1, cbuf, c);
                    printf("  under=%zu isErr=%zu\n", du, t_isError(du));
                }

                /* --- truncated input --- */
                for (int k = 1; k <= 4; k++) {
                    size_t cut = c * k / 5;
                    size_t dt = t_decompress(dbuf, L + 16, cbuf, cut);
                    printf("  trunc%d=%zu isErr=%zu fcs=%llu ffcs=%zu\n", k, dt, t_isError(dt),
                           t_getFrameContentSize(cbuf, cut), t_findFrameCompressedSize(cbuf, cut));
                }

                /* --- corrupted input --- */
                for (int k = 0; k < 6 && c > 8; k++) {
                    static unsigned char *tmp;
                    if (!tmp) tmp = malloc(cbufCap);
                    memcpy(tmp, cbuf, c);
                    tmp[(size_t)(k * 7919 + 5) % c] ^= (unsigned char)(1 << (k & 7));
                    size_t dc = t_decompress(dbuf, L + 16, tmp, c);
                    printf("  corrupt%d=%zu isErr=%zu%s\n", k, dc, t_isError(dc),
                           t_isError(dc) ? "" : " (accepted)");
                }

                /* --- streaming, many chunk granularities --- */
                if (L <= 200000) {
                    size_t chunks[] = {1, 2, 3, 17, 100, 1024, 65536};
                    for (unsigned ci = 0; ci < sizeof(chunks)/sizeof(chunks[0]); ci++) {
                        size_t ch = chunks[ci];
                        if (ch == 1 && c > 4000) continue; /* keep runtime sane */
                        void *ds = t_createDCtx();
                        t_initDStream(ds);
                        inBuf in = {cbuf, 0, 0};
                        outBuf out = {dbuf, 0, 0};
                        memset(dbuf, 0xEF, L + 16);
                        size_t ret = 0, guard = 0;
                        size_t ochunk = ch < 4096 ? ch : 4096;
                        while (in.pos < c || out.pos < L) {
                            in.size = in.pos + ch < c ? in.pos + ch : c;
                            out.size = out.pos + ochunk < L + 16 ? out.pos + ochunk : L + 16;
                            size_t before_in = in.pos, before_out = out.pos;
                            ret = t_decompressStream(ds, &out, &in);
                            if (t_isError(ret)) break;
                            if (ret == 0 && in.pos == c) break;
                            if (in.pos == before_in && out.pos == before_out && in.size == c) break;
                            if (++guard > 4000000) break;
                        }
                        printf("  stream ch=%zu ret=%zu isErr=%zu outpos=%zu ok=%d\n", ch, ret,
                               t_isError(ret), out.pos,
                               (!t_isError(ret) && out.pos == L && (L == 0 || !memcmp(dbuf, src, L))));
                        t_freeDCtx(ds);
                    }
                }

                /* --- decompressContinue path --- */
                if (L <= 70000) {
                    void *dc = t_createDCtx();
                    size_t r = t_decompressBegin(dc);
                    printf("  begin=%zu nextSrc=%zu nextType=%d\n", r,
                           t_nextSrcSizeToDecompress(dc), t_nextInputType(dc));
                    size_t ip = 0, op = 0, iters = 0;
                    memset(dbuf, 0x11, L + 16);
                    while (!t_isError(r)) {
                        size_t need = t_nextSrcSizeToDecompress(dc);
                        if (need == 0) break;
                        if (ip + need > c) break;
                        r = t_decompressContinue(dc, dbuf + op, L + 16 - op, cbuf + ip, need);
                        if (t_isError(r)) break;
                        ip += need; op += r;
                        if (++iters > 100000) break;
                    }
                    printf("  continue last=%zu isErr=%zu op=%zu ok=%d\n", r, t_isError(r), op,
                           (op == L && (L == 0 || !memcmp(dbuf, src, L))));
                    t_freeDCtx(dc);
                }
            }
        }
    }

    /* --- dictionary paths --- */
    {
        static unsigned char dict[16384];
        for (size_t i = 0; i < sizeof(dict); i++) dict[i] = "the quick brown fox "[i % 20];
        size_t L = 30000;
        gen(4, L);
        for (int lvl = 1; lvl <= 19; lvl += 6) {
            void *cctx = r_createCCtx();
            size_t c = r_compress_usingDict(cctx, cbuf, cbufCap, src, L, dict, sizeof(dict), lvl);
            r_freeCCtx(cctx);
            if (r_isError(c)) { printf("dict lvl=%d cErr\n", lvl); continue; }
            printf("dict lvl=%d c=%zu didFrame=%u didDict=%u\n", lvl, c,
                   t_getDictID_fromFrame(cbuf, c), t_getDictID_fromDict(dict, sizeof(dict)));
            void *dctx = t_createDCtx();
            memset(dbuf, 0, L + 16);
            size_t d = t_decompress_usingDict(dctx, dbuf, L + 16, cbuf, c, dict, sizeof(dict));
            printf("  usingDict=%zu ok=%d\n", d, (!t_isError(d) && d == L && !memcmp(dbuf, src, L)));
            /* no dictionary supplied -> must fail the same way */
            size_t dn = t_decompressDCtx(dctx, dbuf, L + 16, cbuf, c);
            printf("  noDict=%zu isErr=%zu\n", dn, t_isError(dn));
            void *ddict = t_createDDict(dict, sizeof(dict));
            printf("  ddictID=%u sizeof=%zu\n", t_getDictID_fromDDict(ddict), t_sizeof_DDict(ddict));
            memset(dbuf, 0, L + 16);
            size_t d2 = t_decompress_usingDDict(dctx, dbuf, L + 16, cbuf, c, ddict);
            printf("  usingDDict=%zu ok=%d\n", d2, (!t_isError(d2) && d2 == L && !memcmp(dbuf, src, L)));
            /* refDDict + streaming */
            t_DCtx_reset(dctx, 3);
            printf("  refDDict=%zu\n", t_DCtx_refDDict(dctx, ddict));
            inBuf in = {cbuf, c, 0};
            outBuf out = {dbuf, L + 16, 0};
            memset(dbuf, 0, L + 16);
            size_t sr = t_decompressStream(dctx, &out, &in);
            printf("  streamDDict=%zu ok=%d\n", sr, (out.pos == L && !memcmp(dbuf, src, L)));
            /* loadDictionary + refPrefix */
            t_DCtx_reset(dctx, 3);
            printf("  loadDict=%zu\n", t_DCtx_loadDictionary(dctx, dict, sizeof(dict)));
            in.pos = 0; out.pos = 0; memset(dbuf, 0, L + 16);
            sr = t_decompressStream(dctx, &out, &in);
            printf("  streamLoadDict=%zu ok=%d\n", sr, (out.pos == L && !memcmp(dbuf, src, L)));
            t_freeDDict(ddict);
            t_freeDCtx(dctx);
        }
        /* raw-content prefix */
        {
            void *cctx = r_createCCtx();
            size_t c = r_compress_usingDict(cctx, cbuf, cbufCap, src, L, dict, 4096, 5);
            r_freeCCtx(cctx);
            void *dctx = t_createDCtx();
            printf("prefix refPrefix=%zu\n", t_DCtx_refPrefix(dctx, dict, 4096));
            inBuf in = {cbuf, c, 0};
            outBuf out = {dbuf, L + 16, 0};
            memset(dbuf, 0, L + 16);
            size_t sr = t_decompressStream(dctx, &out, &in);
            printf("  streamPrefix=%zu ok=%d\n", sr, (out.pos == L && !memcmp(dbuf, src, L)));
            t_freeDCtx(dctx);
        }
    }

    /* --- multi-frame and skippable frames --- */
    {
        size_t L = 5000;
        gen(2, L);
        size_t c1 = r_compress(cbuf, cbufCap, src, L, 3);
        size_t total = c1;
        memcpy(cbuf + total, cbuf, c1); total += c1;
        /* skippable frame in the middle */
        unsigned char skip[16] = {0x50, 0x2a, 0x4d, 0x18, 4, 0, 0, 0, 0xDE, 0xAD, 0xBE, 0xEF};
        memcpy(cbuf + total, skip, 12); total += 12;
        memcpy(cbuf + total, cbuf, c1); total += c1;
        printf("multi total=%zu fds=%llu dbound=%llu\n", total,
               t_findDecompressedSize(cbuf, total), t_decompressBound(cbuf, total));
        memset(dbuf, 0, 3 * L + 16);
        size_t d = t_decompress(dbuf, 3 * L + 16, cbuf, total);
        printf("  multi d=%zu ok=%d\n", d, (!t_isError(d) && d == 3 * L));
        if (!t_isError(d)) hash_report("  multi", dbuf, d);
        printf("  skipFrame isSkip=%u ffcs=%zu\n", t_isSkippableFrame(cbuf + 2 * c1, 12),
               t_findFrameCompressedSize(cbuf + 2 * c1, 12));
        {   unsigned magic = 0; unsigned char out[16];
            size_t r = t_readSkippableFrame(out, sizeof(out), &magic, cbuf + 2 * c1, 12);
            printf("  readSkippable=%zu magic=%u bytes=%02x%02x%02x%02x\n", r, magic,
                   out[0], out[1], out[2], out[3]);
        }
    }

    /* --- parameter API --- */
    {
        void *dctx = t_createDCtx();
        for (int p = 99; p <= 106; p++) {
            int v = -1;
            size_t g = t_DCtx_getParameter(dctx, p == 99 ? 100 : (p == 100 ? 100 : 1000 + (p - 101)), &v);
            printf("param %d get=%zu isErr=%zu v=%d\n", p, g, t_isError(g), v);
        }
        for (int wl = 0; wl < 34; wl += 4) {
            size_t s = t_DCtx_setParameter(dctx, 100, wl);
            printf("setWindowLogMax %d = %zu isErr=%zu\n", wl, s, t_isError(s));
        }
        for (int r = 0; r <= 4; r++)
            printf("reset %d = %zu\n", r, t_DCtx_reset(dctx, r));
        printf("sizeof_DCtx=%zu\n", t_sizeof_DCtx(dctx));
        t_freeDCtx(dctx);
    }

    /* --- garbage / edge inputs --- */
    {
        unsigned char junk[64];
        for (unsigned i = 0; i < sizeof(junk); i++) junk[i] = (unsigned char)(i * 37);
        for (size_t n = 0; n <= sizeof(junk); n += 5) {
            size_t d = t_decompress(dbuf, 1024, junk, n);
            printf("junk n=%zu d=%zu isErr=%zu fcs=%llu isFrame=%u fhs=%zu\n", n, d, t_isError(d),
                   t_getFrameContentSize(junk, n), t_isFrame(junk, n), t_frameHeaderSize(junk, n));
        }
        /* valid magic, garbage body */
        unsigned char fake[32] = {0x28, 0xB5, 0x2F, 0xFD};
        for (unsigned i = 4; i < sizeof(fake); i++) fake[i] = (unsigned char)(i * 91);
        for (size_t n = 4; n <= sizeof(fake); n += 3) {
            size_t d = t_decompress(dbuf, 1024, fake, n);
            printf("fake n=%zu d=%zu isErr=%zu fcs=%llu fhs=%zu\n", n, d, t_isError(d),
                   t_getFrameContentSize(fake, n), t_frameHeaderSize(fake, n));
        }
        /* NULL dst with zero capacity */
        size_t dz = t_decompress(NULL, 0, cbuf, 10);
        printf("nulldst=%zu isErr=%zu\n", dz, t_isError(dz));
    }

    fflush(stdout);
    return 0;
}
