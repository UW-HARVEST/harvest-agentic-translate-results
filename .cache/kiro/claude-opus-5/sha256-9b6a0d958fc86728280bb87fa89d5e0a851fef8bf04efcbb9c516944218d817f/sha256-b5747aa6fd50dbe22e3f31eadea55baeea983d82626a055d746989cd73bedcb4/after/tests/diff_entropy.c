/* Differential harness for the entropy layer.
 *
 * The reference C library is always used to *produce* test vectors (FSE streams,
 * Huffman weight headers); the library under test is used to *decode* them.
 * Running with the reference as the library under test gives the baseline. */
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dlfcn.h>

#define REF_PATH "/tmp/zref/libzstd.so"

static void *R, *T;

static void *rsym(const char *n)
{
    void *s = dlsym(R, n);
    if (!s) { fprintf(stderr, "MISSING REF SYMBOL %s\n", n); exit(2); }
    return s;
}
static void *tsym(const char *n)
{
    void *s = dlsym(T, n);
    if (!s) { fprintf(stderr, "MISSING TEST SYMBOL %s\n", n); exit(2); }
    return s;
}

static unsigned long long rs = 0x2545F4914F6CDD1DULL;
static unsigned rnd(void)
{
    rs ^= rs << 13; rs ^= rs >> 7; rs ^= rs << 17;
    return (unsigned)(rs >> 24);
}

int main(int argc, char **argv)
{
    if (argc < 2) { fprintf(stderr, "usage: %s <libzstd.so>\n", argv[0]); return 1; }
    R = dlopen(REF_PATH, RTLD_NOW | RTLD_LOCAL);
    T = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!R || !T) { fprintf(stderr, "dlopen: %s\n", dlerror()); return 1; }

    /* --- reference-side encoders --- */
    unsigned (*optimalTableLog)(unsigned, size_t, unsigned) = rsym("FSE_optimalTableLog");
    size_t (*normalizeCount)(short *, unsigned, const unsigned *, size_t, unsigned, unsigned) =
        rsym("FSE_normalizeCount");
    size_t (*writeNCount)(void *, size_t, const short *, unsigned, unsigned) = rsym("FSE_writeNCount");
    size_t (*buildCTable_wksp)(unsigned *, const short *, unsigned, unsigned, void *, size_t) =
        rsym("FSE_buildCTable_wksp");
    size_t (*compress_usingCTable)(void *, size_t, const void *, size_t, const unsigned *) =
        rsym("FSE_compress_usingCTable");
    size_t (*hist_count)(unsigned *, unsigned *, const void *, size_t, void *, size_t) =
        rsym("HIST_count_wksp");

    /* --- test-side decoders --- */
    size_t (*readNCount)(short *, unsigned *, unsigned *, const void *, size_t) =
        tsym("FSE_readNCount");
    size_t (*buildDTable_wksp)(unsigned *, const short *, unsigned, unsigned, void *, size_t) =
        tsym("FSE_buildDTable_wksp");
    size_t (*decompress_wksp)(void *, size_t, const void *, size_t, unsigned, void *, size_t, int) =
        tsym("FSE_decompress_wksp_bmi2");
    size_t (*isError)(size_t) = tsym("FSE_isError");

    static unsigned char src[40000];
    static unsigned char cbuf[80000];
    static unsigned char dbuf[80000];
    static unsigned count[256];
    static short norm[256];
    static unsigned ctable[8192];
    static unsigned dtable[8192];
    static unsigned wksp[16384];
    static short norm2[256];

    /* A range of distributions: uniform, skewed, few symbols, single symbol. */
    for (int mode = 0; mode < 6; mode++) {
        for (int li = 0; li < 5; li++) {
            size_t const lens[5] = {64, 512, 4000, 20000, 40000};
            size_t L = lens[li];
            unsigned maxSymbol;
            switch (mode) {
            case 0: maxSymbol = 255; for (size_t i = 0; i < L; i++) src[i] = rnd() & 0xFF; break;
            case 1: maxSymbol = 15;  for (size_t i = 0; i < L; i++) src[i] = rnd() & 0x0F; break;
            case 2: maxSymbol = 3;   for (size_t i = 0; i < L; i++) src[i] = (rnd() & 0x3F) < 50 ? 0 : (rnd() & 3); break;
            case 3: maxSymbol = 1;   for (size_t i = 0; i < L; i++) src[i] = (rnd() & 7) == 0; break;
            case 4: maxSymbol = 255; for (size_t i = 0; i < L; i++) src[i] = (unsigned char)(i * 7 + (rnd() & 1)); break;
            default: maxSymbol = 63; for (size_t i = 0; i < L; i++) {
                        unsigned r = rnd() & 0xFFFF;
                        src[i] = r < 40000 ? 5 : (r < 60000 ? (r & 7) : (r & 63));
                     } break;
            }

            unsigned msv = maxSymbol;
            size_t maxCount = hist_count(count, &msv, src, L, wksp, sizeof(wksp));
            if (isError(maxCount)) { printf("mode=%d L=%zu hist error\n", mode, L); continue; }
            if (maxCount == L) { printf("mode=%d L=%zu rle\n", mode, L); continue; }
            unsigned tableLog = optimalTableLog(12, L, msv);
            size_t nerr = normalizeCount(norm, tableLog, count, L, msv, 0);
            if (isError(nerr)) { printf("mode=%d L=%zu normalize error\n", mode, L); continue; }
            size_t hsize = writeNCount(cbuf, sizeof(cbuf), norm, msv, tableLog);
            if (isError(hsize)) { printf("mode=%d L=%zu writeNCount error\n", mode, L); continue; }
            size_t cerr = buildCTable_wksp(ctable, norm, msv, tableLog, wksp, sizeof(wksp));
            if (isError(cerr)) { printf("mode=%d L=%zu buildCTable error\n", mode, L); continue; }
            size_t csize = compress_usingCTable(cbuf + hsize, sizeof(cbuf) - hsize, src, L, ctable);
            if (isError(csize) || csize == 0) { printf("mode=%d L=%zu notcompressible\n", mode, L); continue; }
            size_t total = hsize + csize;

            /* 1. readNCount on the produced header */
            unsigned rmsv = 255, rtl = 0;
            memset(norm2, 0x5A, sizeof(norm2));
            size_t rn = readNCount(norm2, &rmsv, &rtl, cbuf, total);
            printf("mode=%d L=%zu readNCount=%zu msv=%u tl=%u norm:", mode, L, rn, rmsv, rtl);
            if (!isError(rn)) for (unsigned s = 0; s <= rmsv; s++) printf(" %d", norm2[s]);
            printf("\n");

            /* 1b. readNCount on truncated headers (error paths) */
            for (size_t cut = 1; cut < 8 && cut < total; cut++) {
                unsigned m2 = 255, t2 = 0;
                size_t r2 = readNCount(norm2, &m2, &t2, cbuf, cut);
                printf("  trunc=%zu err=%zu isErr=%zu\n", cut, r2, isError(r2));
            }

            /* 2. buildDTable + full decompress */
            if (!isError(rn)) {
                memset(dtable, 0, sizeof(dtable));
                size_t be = buildDTable_wksp(dtable, norm2, rmsv, rtl, wksp, sizeof(wksp));
                unsigned long long sum = 0;
                for (unsigned i = 0; i < (1u + (1u << rtl)); i++) sum = sum * 1000003 + dtable[i];
                printf("  buildDTable=%zu hash=%llu\n", be, sum);
            }
            memset(dbuf, 0, sizeof(dbuf));
            size_t d = decompress_wksp(dbuf, sizeof(dbuf), cbuf, total, 12, wksp, sizeof(wksp), 0);
            printf("  decompress=%zu match=%d\n", d,
                   (!isError(d) && d == L && memcmp(dbuf, src, L) == 0));

            /* 3. corrupted input */
            for (int k = 0; k < 4; k++) {
                memcpy(dbuf, cbuf, total);
                dbuf[(k * 7 + 1) % total] ^= 0xA5;
                static unsigned char out2[80000];
                size_t d2 = decompress_wksp(out2, sizeof(out2), dbuf, total, 12, wksp, sizeof(wksp), 0);
                printf("  corrupt k=%d ret=%zu isErr=%zu\n", k, d2, isError(d2));
            }
            /* 4. too-small dst */
            size_t d3 = decompress_wksp(dbuf, L / 2, cbuf, total, 12, wksp, sizeof(wksp), 0);
            printf("  smalldst ret=%zu isErr=%zu\n", d3, isError(d3));
            /* 5. maxLog too small */
            size_t d4 = decompress_wksp(dbuf, sizeof(dbuf), cbuf, total, 5, wksp, sizeof(wksp), 0);
            printf("  smallmaxlog ret=%zu isErr=%zu\n", d4, isError(d4));
        }
    }

    /* --- HUF_readStats on real Huffman headers produced by the reference --- */
    {
        size_t (*readStats)(unsigned char *, size_t, unsigned *, unsigned *, unsigned *,
                            const void *, size_t) = tsym("HUF_readStats");
        size_t (*readStats_wksp)(unsigned char *, size_t, unsigned *, unsigned *, unsigned *,
                                 const void *, size_t, void *, size_t, int) = tsym("HUF_readStats_wksp");
        /* Produce literal sections by compressing data with the reference zstd,
         * then feed the raw huffman header bytes we can find via HUF_writeCTable_wksp. */
        unsigned (*optimalTableLogHuf)(unsigned, size_t, unsigned, void *, size_t, unsigned long *, const unsigned *, int) =
            rsym("HUF_optimalTableLog");
        size_t (*buildCTable)(unsigned long *, const unsigned *, unsigned, unsigned, void *, size_t) =
            rsym("HUF_buildCTable_wksp");
        size_t (*writeCTable)(void *, size_t, const unsigned long *, unsigned, unsigned, void *, size_t) =
            rsym("HUF_writeCTable_wksp");
        static unsigned long cts[300];
        static unsigned char hbuf[1024];
        static unsigned char weights[256];
        static unsigned ranks[16];
        for (int mode = 0; mode < 4; mode++) {
            size_t L = 20000;
            unsigned maxSymbol = mode == 0 ? 255 : (mode == 1 ? 15 : (mode == 2 ? 63 : 3));
            for (size_t i = 0; i < L; i++) {
                unsigned r = rnd();
                src[i] = mode == 0 ? (r & 0xFF)
                       : (unsigned char)((r & 0xFFFF) < 45000 ? (r & 1) : (r & maxSymbol));
            }
            unsigned msv = maxSymbol;
            size_t mc = hist_count(count, &msv, src, L, wksp, sizeof(wksp));
            if (isError(mc) || mc == L) { printf("huf mode=%d skip\n", mode); continue; }
            unsigned tl = optimalTableLogHuf(11, L, msv, wksp, sizeof(wksp), cts, count, 0);
            memset(cts, 0, sizeof(cts));
            size_t bc = buildCTable(cts, count, msv, tl, wksp, sizeof(wksp));
            if (isError(bc)) { printf("huf mode=%d buildCTable err\n", mode); continue; }
            size_t hs = writeCTable(hbuf, sizeof(hbuf), cts, msv, (unsigned)bc, wksp, sizeof(wksp));
            if (isError(hs)) { printf("huf mode=%d writeCTable err\n", mode); continue; }

            unsigned nbs = 0, rtl = 0;
            memset(weights, 0xCC, sizeof(weights));
            memset(ranks, 0xCC, sizeof(ranks));
            size_t r = readStats(weights, 256, ranks, &nbs, &rtl, hbuf, hs);
            printf("huf mode=%d hs=%zu read=%zu nbs=%u tl=%u w:", mode, hs, r, nbs, rtl);
            if (!isError(r)) for (unsigned i = 0; i < nbs; i++) printf("%d,", weights[i]);
            printf(" ranks:");
            if (!isError(r)) for (unsigned i = 0; i < 13; i++) printf("%u,", ranks[i]);
            printf("\n");

            size_t r2 = readStats_wksp(weights, 256, ranks, &nbs, &rtl, hbuf, hs,
                                       wksp, sizeof(wksp), 0);
            printf("  wksp read=%zu\n", r2);
            for (size_t cut = 1; cut <= hs && cut < 6; cut++) {
                size_t r3 = readStats(weights, 256, ranks, &nbs, &rtl, hbuf, cut);
                printf("  hufTrunc=%zu ret=%zu isErr=%zu\n", cut, r3, isError(r3));
            }
            /* small hwSize */
            size_t r4 = readStats(weights, 4, ranks, &nbs, &rtl, hbuf, hs);
            printf("  smallhw ret=%zu isErr=%zu\n", r4, isError(r4));
        }
    }

    fflush(stdout);
    return 0;
}
