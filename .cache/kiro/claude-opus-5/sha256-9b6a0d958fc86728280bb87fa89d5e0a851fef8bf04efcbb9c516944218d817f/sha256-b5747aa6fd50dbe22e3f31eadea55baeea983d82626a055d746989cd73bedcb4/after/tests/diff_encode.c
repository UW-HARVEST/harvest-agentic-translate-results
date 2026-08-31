/* Differential harness for the FSE/HUF/HIST *encoders*.
 *
 * Everything here is executed by the library under test, and every produced
 * byte is dumped, so diffing the two runs proves byte-identical encoder output.
 * The reference library is additionally used to decode what was encoded. */
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

static unsigned long long st = 0x853C49E6748FEA9BULL;
static unsigned rnd(void) { st ^= st << 13; st ^= st >> 7; st ^= st << 17; return (unsigned)(st >> 24); }

static void dump(const char *tag, const void *p, size_t n)
{
    const unsigned char *b = p;
    printf("%s[%zu]=", tag, n);
    for (size_t i = 0; i < n; i++) printf("%02x", b[i]);
    printf("\n");
}

static unsigned long long fnv(const void *p, size_t n)
{
    const unsigned char *b = p;
    unsigned long long h = 1469598103934665603ULL;
    for (size_t i = 0; i < n; i++) { h ^= b[i]; h *= 1099511628211ULL; }
    return h;
}

static unsigned char src[200000];
static unsigned char hdr[2048];
static unsigned char cbuf[400000];
static unsigned char dbuf[400000];
static unsigned count[256];
static short norm[256];
static unsigned ctable[8192];
static unsigned long hcts[300];
static unsigned long hcts2[300];
static unsigned wksp[40000];
static unsigned char weights[256];
static unsigned ranks[16];
static unsigned dtable[8192];

int main(int argc, char **argv)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    if (argc < 2) return 1;
    R = dlopen(REF_PATH, RTLD_NOW | RTLD_LOCAL);
    T = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!R || !T) { fprintf(stderr, "dlopen: %s\n", dlerror()); return 1; }

    /* ---- library under test: encoders ---- */
    size_t (*isErr)(size_t) = tsym("FSE_isError");
    size_t (*histIsErr)(size_t) = tsym("HIST_isError");
    size_t (*histCount)(unsigned *, unsigned *, const void *, size_t, void *, size_t) = tsym("HIST_count_wksp");
    size_t (*histCountFast)(unsigned *, unsigned *, const void *, size_t, void *, size_t) = tsym("HIST_countFast_wksp");
    size_t (*histCountSimple)(unsigned *, unsigned *, const void *, size_t) = tsym("HIST_count_simple");
    void (*histAdd)(unsigned *, const void *, size_t) = tsym("HIST_add");
    unsigned (*optTL)(unsigned, size_t, unsigned) = tsym("FSE_optimalTableLog");
    unsigned (*optTLi)(unsigned, size_t, unsigned, unsigned) = tsym("FSE_optimalTableLog_internal");
    size_t (*normCount)(short *, unsigned, const unsigned *, size_t, unsigned, unsigned) = tsym("FSE_normalizeCount");
    size_t (*ncwb)(unsigned, unsigned) = tsym("FSE_NCountWriteBound");
    size_t (*writeNC)(void *, size_t, const short *, unsigned, unsigned) = tsym("FSE_writeNCount");
    size_t (*buildCT)(unsigned *, const short *, unsigned, unsigned, void *, size_t) = tsym("FSE_buildCTable_wksp");
    size_t (*buildCTrle)(unsigned *, unsigned char) = tsym("FSE_buildCTable_rle");
    size_t (*compCT)(void *, size_t, const void *, size_t, const unsigned *) = tsym("FSE_compress_usingCTable");
    size_t (*fseCB)(size_t) = tsym("FSE_compressBound");

    size_t (*hufCB)(size_t) = tsym("HUF_compressBound");
    unsigned (*hufCard)(const unsigned *, unsigned) = tsym("HUF_cardinality");
    unsigned (*hufMinTL)(unsigned) = tsym("HUF_minTableLog");
    unsigned (*hufOptTL)(unsigned, size_t, unsigned, void *, size_t, unsigned long *, const unsigned *, int) = tsym("HUF_optimalTableLog");
    size_t (*hufBuildCT)(unsigned long *, const unsigned *, unsigned, unsigned, void *, size_t) = tsym("HUF_buildCTable_wksp");
    size_t (*hufWriteCT)(void *, size_t, const unsigned long *, unsigned, unsigned, void *, size_t) = tsym("HUF_writeCTable_wksp");
    size_t (*hufReadCT)(unsigned long *, unsigned *, const void *, size_t, unsigned *) = tsym("HUF_readCTable");
    size_t (*hufReadCTH)(void *, const void *, size_t) = tsym("HUF_readCTableHeader");
    size_t (*hufNbBits)(const unsigned long *, unsigned) = tsym("HUF_getNbBitsFromCTable");
    size_t (*hufEst)(const unsigned long *, const unsigned *, unsigned) = tsym("HUF_estimateCompressedSize");
    int (*hufValid)(const unsigned long *, const unsigned *, unsigned) = tsym("HUF_validateCTable");
    size_t (*hufC1)(void *, size_t, const void *, size_t, const unsigned long *, int) = tsym("HUF_compress1X_usingCTable");
    size_t (*hufC4)(void *, size_t, const void *, size_t, const unsigned long *, int) = tsym("HUF_compress4X_usingCTable");
    size_t (*hufR1)(void *, size_t, const void *, size_t, unsigned, unsigned, void *, size_t,
                    unsigned long *, unsigned *, int) = tsym("HUF_compress1X_repeat");
    size_t (*hufR4)(void *, size_t, const void *, size_t, unsigned, unsigned, void *, size_t,
                    unsigned long *, unsigned *, int) = tsym("HUF_compress4X_repeat");

    /* ---- reference: decoders, to confirm what we produced is readable ---- */
    size_t (*rDecWksp)(void *, size_t, const void *, size_t, unsigned, void *, size_t, int) = rsym("FSE_decompress_wksp_bmi2");
    size_t (*rReadStats)(unsigned char *, size_t, unsigned *, unsigned *, unsigned *, const void *, size_t) = rsym("HUF_readStats");
    size_t (*rReadX1)(unsigned *, const void *, size_t, void *, size_t, int) = rsym("HUF_readDTableX1_wksp");
    size_t (*rDec4)(unsigned *, void *, size_t, const void *, size_t, void *, size_t, int) = rsym("HUF_decompress4X_hufOnly_wksp");
    size_t (*rDec1)(unsigned *, void *, size_t, const void *, size_t, void *, size_t, int) = rsym("HUF_decompress1X1_DCtx_wksp");
    size_t (*rIsErr)(size_t) = rsym("FSE_isError");

    for (size_t s = 0; s < 200000; s += 4999)
        printf("bounds fse=%zu huf=%zu\n", fseCB(s), hufCB(s));
    /* NOTE: maxSymbolValue must be >= 1. FSE_minTableLog() calls
     * ZSTD_highbit32(maxSymbolValue), i.e. __builtin_clz(0) for a value of 0,
     * which is undefined behaviour in the C; on x86-64 gcc emits `bsr`, whose
     * destination is left unmodified for a zero input, so the C result there is
     * whatever happened to be in the register. zstd never reaches that case
     * (a single-symbol input takes the RLE path), so it is not compared. */
    for (unsigned m = 1; m < 256; m += 17)
        for (unsigned tl = 5; tl <= 12; tl++)
            printf("ncwb(%u,%u)=%zu optTL(%u,1000,%u)=%u optTLi=%u minTL=%u\n",
                   m, tl, ncwb(m, tl), tl, m, optTL(tl, 1000, m), optTLi(tl, 1000, m, 2),
                   hufMinTL(m ? m : 1));

    size_t lens[] = {16, 64, 300, 2000, 9000, 50000, 130000};
    for (int mode = 0; mode < 8; mode++) {
        for (unsigned li = 0; li < sizeof(lens)/sizeof(lens[0]); li++) {
            size_t L = lens[li];
            unsigned maxSymbol = 255;
            for (size_t i = 0; i < L; i++) {
                unsigned r = rnd();
                switch (mode) {
                case 0: src[i] = r & 0xFF; break;
                case 1: src[i] = r & 0x0F; maxSymbol = 15; break;
                case 2: src[i] = (r & 0xFFFF) < 55000 ? 3 : (r & 0xFF); break;
                case 3: src[i] = (r & 1) ? 9 : 200; break;
                case 4: src[i] = (unsigned char)(i & 0x3F); maxSymbol = 63; break;
                case 5: src[i] = (r % 1000) < 995 ? 0 : (r & 0xFF); break;
                case 6: src[i] = (unsigned char)(r % 3); maxSymbol = 2; break;
                default: src[i] = (unsigned char)((r % 100) < 70 ? (r & 7) : (r & 0xFF)); break;
                }
            }
            printf("=== mode=%d L=%zu\n", mode, L);

            /* ---- histogram ---- */
            unsigned msv = maxSymbol;
            memset(count, 0xEE, sizeof(count));
            size_t mc = histCount(count, &msv, src, L, wksp, sizeof(wksp));
            printf("hist mc=%zu isErr=%zu msv=%u chash=%llu\n", mc, histIsErr(mc), msv,
                   fnv(count, (msv + 1) * sizeof(unsigned)));
            {   unsigned m2 = maxSymbol, c2[256];
                memset(c2, 0, sizeof(c2));
                size_t mc2 = histCountSimple(c2, &m2, src, L);
                printf("  simple mc=%zu msv=%u chash=%llu\n", mc2, m2, fnv(c2, (m2 + 1) * sizeof(unsigned)));
                unsigned m3 = maxSymbol, c3[256];
                memset(c3, 0, sizeof(c3));
                size_t mc3 = histCountFast(c3, &m3, src, L, wksp, sizeof(wksp));
                printf("  fast mc=%zu isErr=%zu msv=%u chash=%llu\n", mc3, histIsErr(mc3), m3,
                       fnv(c3, (m3 + 1) * sizeof(unsigned)));
                unsigned c4[256];
                memset(c4, 0, sizeof(c4));
                histAdd(c4, src, L);
                printf("  add chash=%llu\n", fnv(c4, sizeof(c4)));
                /* undersized maxSymbolValue -> error path */
                unsigned m5 = 1;
                size_t mc5 = histCount(c2, &m5, src, L, wksp, sizeof(wksp));
                printf("  tooSmall mc=%zu isErr=%zu\n", mc5, histIsErr(mc5));
            }
            if (histIsErr(mc) || mc == L) { printf("  (rle/err, skipping FSE/HUF)\n"); continue; }

            /* ---- FSE encode ---- */
            for (unsigned tl = 5; tl <= 12; tl += 1) {
                unsigned use = optTL(tl, L, msv);
                memset(norm, 0x5A, sizeof(norm));
                size_t ne = normCount(norm, use, count, L, msv, 0);
                printf("fse tl=%u use=%u norm=%zu isErr=%zu nhash=%llu n:", tl, use, ne, isErr(ne),
                       fnv(norm, (msv + 1) * sizeof(short)));
                if (!isErr(ne)) for (unsigned i = 0; i <= msv; i++) printf("%d,", norm[i]);
                printf("\n");
                if (isErr(ne)) continue;
                size_t hs = writeNC(hdr, sizeof(hdr), norm, msv, use);
                printf("  writeNC=%zu isErr=%zu ", hs, isErr(hs));
                if (!isErr(hs)) dump("hdr", hdr, hs); else printf("\n");
                if (isErr(hs)) continue;
                /* exact-size and undersized header buffer */
                printf("  writeNC exact=%zu tight=%zu\n",
                       writeNC(hdr, hs, norm, msv, use),
                       hs > 1 ? writeNC(hdr, hs - 1, norm, msv, use) : 0);
                memset(ctable, 0, sizeof(ctable));
                size_t bc = buildCT(ctable, norm, msv, use, wksp, sizeof(wksp));
                printf("  buildCT=%zu isErr=%zu cthash=%llu\n", bc, isErr(bc),
                       fnv(ctable, (1 + (1u << (use ? use - 1 : 0)) + (msv + 1) * 2) * sizeof(unsigned)));
                if (isErr(bc)) continue;
                size_t cs = compCT(cbuf, sizeof(cbuf), src, L, ctable);
                printf("  compCT=%zu isErr=%zu chash=%llu\n", cs, isErr(cs), isErr(cs) ? 0 : fnv(cbuf, cs));
                if (!isErr(cs) && cs > 0 && cs < 400) dump("  cbytes", cbuf, cs);
                /* the reference decoder must reproduce the input from our bytes */
                if (!isErr(cs) && cs > 0) {
                    memcpy(dbuf, hdr, hs);
                    memcpy(dbuf + hs, cbuf, cs);
                    static unsigned char out[400000];
                    size_t d = rDecWksp(out, sizeof(out), dbuf, hs + cs, 12, wksp, sizeof(wksp), 0);
                    printf("  refDecode=%zu roundtrip=%d\n", d,
                           (!rIsErr(d) && d == L && !memcmp(out, src, L)));
                }
                /* tight output buffer */
                if (!isErr(cs) && cs > 2) {
                    size_t cs2 = compCT(cbuf, cs - 1, src, L, ctable);
                    printf("  compCT tight=%zu isErr=%zu\n", cs2, isErr(cs2));
                }
            }
            {   size_t br = buildCTrle(ctable, 42);
                printf("fse buildCTrle=%zu cthash=%llu\n", br, fnv(ctable, 16 * sizeof(unsigned)));
            }

            /* ---- HUF encode ---- */
            printf("huf card=%u\n", hufCard(count, msv));
            for (unsigned tl = 8; tl <= 12; tl++) {
                memset(hcts, 0, sizeof(hcts));
                unsigned use = hufOptTL(tl, L, msv, wksp, sizeof(wksp), hcts, count, 0);
                memset(hcts, 0, sizeof(hcts));
                size_t bc = hufBuildCT(hcts, count, msv, use, wksp, sizeof(wksp));
                printf("huf tl=%u use=%u buildCT=%zu isErr=%zu cthash=%llu nb:", tl, use, bc, isErr(bc),
                       fnv(hcts, (msv + 2) * sizeof(unsigned long)));
                if (!isErr(bc)) for (unsigned s2 = 0; s2 <= msv && s2 < 40; s2++) printf("%zu,", hufNbBits(hcts, s2));
                printf("\n");
                if (isErr(bc)) continue;
                printf("  est=%zu valid=%d\n", hufEst(hcts, count, msv), hufValid(hcts, count, msv));
                size_t hs = hufWriteCT(hdr, sizeof(hdr), hcts, msv, (unsigned)bc, wksp, sizeof(wksp));
                printf("  writeCT=%zu isErr=%zu ", hs, isErr(hs));
                if (!isErr(hs)) dump("hhdr", hdr, hs); else printf("\n");
                if (isErr(hs)) continue;
                {   unsigned char h3[4] = {0, 0, 0, 0};
                    size_t rh = hufReadCTH(h3, hdr, hs);
                    printf("  readCTHeader=%zu bytes=%02x%02x%02x%02x\n", rh, h3[0], h3[1], h3[2], h3[3]);
                }
                {   unsigned nsym = 0, tlOut = 0;
                    memset(hcts2, 0, sizeof(hcts2));
                    size_t rc = hufReadCT(hcts2, &nsym, hdr, hs, &tlOut);
                    printf("  readCTable=%zu nsym=%u tl=%u cthash=%llu\n", rc, nsym, tlOut,
                           fnv(hcts2, (msv + 2) * sizeof(unsigned long)));
                }
                {   unsigned nbs = 0, tlOut = 0;
                    memset(weights, 0, sizeof(weights));
                    memset(ranks, 0, sizeof(ranks));
                    size_t rs2 = rReadStats(weights, 256, ranks, &nbs, &tlOut, hdr, hs);
                    printf("  refReadStats=%zu nbs=%u tl=%u whash=%llu\n", rs2, nbs, tlOut,
                           fnv(weights, nbs));
                }
                /* 1-stream and 4-stream encodes, then decode with the reference */
                for (int which = 0; which < 2; which++) {
                    size_t cs = which ? hufC4(cbuf, sizeof(cbuf), src, L, hcts, 0)
                                      : hufC1(cbuf, sizeof(cbuf), src, L, hcts, 0);
                    printf("  %s=%zu isErr=%zu chash=%llu\n", which ? "comp4X" : "comp1X", cs,
                           isErr(cs), isErr(cs) ? 0 : fnv(cbuf, cs));
                    if (isErr(cs) || cs == 0) continue;
                    memcpy(dbuf, hdr, hs);
                    memcpy(dbuf + hs, cbuf, cs);
                    static unsigned char out[400000];
                    memset(dtable, 0, sizeof(dtable));
                    dtable[0] = 11 * 0x01000001u;
                    size_t d = which ? rDec4(dtable, out, L, dbuf, hs + cs, wksp, sizeof(wksp), 0)
                                     : rDec1(dtable, out, L, dbuf, hs + cs, wksp, sizeof(wksp), 0);
                    printf("    refDecode=%zu roundtrip=%d\n", d,
                           (!rIsErr(d) && d == L && !memcmp(out, src, L)));
                    /* tight dst */
                    size_t cs2 = which ? hufC4(cbuf, cs - 1, src, L, hcts, 0)
                                       : hufC1(cbuf, cs - 1, src, L, hcts, 0);
                    printf("    tight=%zu isErr=%zu\n", cs2, isErr(cs2));
                }
            }
            /* ---- repeat-table API ---- */
            for (int pass = 0; pass < 3; pass++) {
                unsigned repeat = pass == 0 ? 0 : (pass == 1 ? 1 : 2); /* none/check/valid */
                for (int flags = 0; flags < 8; flags += 2) {
                    memset(hcts, 0, sizeof(hcts));
                    unsigned rep = repeat;
                    size_t c1 = hufR1(cbuf, sizeof(cbuf), src, L, msv, 11, wksp, sizeof(wksp),
                                      hcts, &rep, flags);
                    printf("  repeat1X p=%d f=%d ret=%zu isErr=%zu rep=%u chash=%llu\n", pass, flags,
                           c1, isErr(c1), rep, isErr(c1) || c1 == 0 ? 0 : fnv(cbuf, c1));
                    memset(hcts, 0, sizeof(hcts));
                    rep = repeat;
                    size_t c4 = hufR4(cbuf, sizeof(cbuf), src, L, msv, 11, wksp, sizeof(wksp),
                                      hcts, &rep, flags);
                    printf("  repeat4X p=%d f=%d ret=%zu isErr=%zu rep=%u chash=%llu\n", pass, flags,
                           c4, isErr(c4), rep, isErr(c4) || c4 == 0 ? 0 : fnv(cbuf, c4));
                }
            }
        }
    }
    fflush(stdout);
    return 0;
}
