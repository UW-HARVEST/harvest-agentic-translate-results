/* Differential harness for the Huffman decode layer.
 * Reference library produces Huffman streams; the library under test decodes. */
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
{ void *s = dlsym(T, n); if (!s) { fprintf(stderr, "MISSING TEST %s\n", n); exit(2); } return s; }

static unsigned long long rs = 0x139408DCBBF7A44ULL;
static unsigned rnd(void) { rs ^= rs << 13; rs ^= rs >> 7; rs ^= rs << 17; return (unsigned)(rs >> 24); }

typedef size_t (*hist_fn)(unsigned *, unsigned *, const void *, size_t, void *, size_t);

static unsigned char src[200000];
static unsigned char hdr[1024];
static unsigned char cbuf[300000];
static unsigned char dbuf[300000];
static unsigned count[256];
static unsigned long cts[300];
static unsigned wksp[40000];
static unsigned dtable[8192];
static unsigned dctx[8192];

/* HUF_CREATE_STATIC_DTABLEX1/X2 initialise element 0 with the max tableLog */
static void initX1(unsigned *t, unsigned maxTableLog) { memset(t, 0, 8192*sizeof(unsigned)); t[0] = (maxTableLog - 1) * 0x01000001u; }
static void initX2(unsigned *t, unsigned maxTableLog) { memset(t, 0, 8192*sizeof(unsigned)); t[0] = maxTableLog * 0x01000001u; }

int main(int argc, char **argv)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    if (argc < 2) { fprintf(stderr, "usage: %s <libzstd.so>\n", argv[0]); return 1; }
    R = dlopen(REF_PATH, RTLD_NOW | RTLD_LOCAL);
    T = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!R || !T) { fprintf(stderr, "dlopen: %s\n", dlerror()); return 1; }

    hist_fn hist = rsym("HIST_count_wksp");
    unsigned (*hufOptTL)(unsigned, size_t, unsigned, void *, size_t, unsigned long *, const unsigned *, int) = rsym("HUF_optimalTableLog");
    size_t (*buildCT)(unsigned long *, const unsigned *, unsigned, unsigned, void *, size_t) = rsym("HUF_buildCTable_wksp");
    size_t (*writeCT)(void *, size_t, const unsigned long *, unsigned, unsigned, void *, size_t) = rsym("HUF_writeCTable_wksp");
    size_t (*comp4X)(void *, size_t, const void *, size_t, const unsigned long *, int) = rsym("HUF_compress4X_usingCTable");
    size_t (*comp1X)(void *, size_t, const void *, size_t, const unsigned long *, int) = rsym("HUF_compress1X_usingCTable");

    /* library under test */
    size_t (*readX1)(unsigned *, const void *, size_t, void *, size_t, int) = tsym("HUF_readDTableX1_wksp");
    size_t (*readX2)(unsigned *, const void *, size_t, void *, size_t, int) = tsym("HUF_readDTableX2_wksp");
    size_t (*dec1Xusing)(void *, size_t, const void *, size_t, const unsigned *, int) = tsym("HUF_decompress1X_usingDTable");
    size_t (*dec4Xusing)(void *, size_t, const void *, size_t, const unsigned *, int) = tsym("HUF_decompress4X_usingDTable");
    size_t (*dec1X1w)(unsigned *, void *, size_t, const void *, size_t, void *, size_t, int) = tsym("HUF_decompress1X1_DCtx_wksp");
    size_t (*dec1X2w)(unsigned *, void *, size_t, const void *, size_t, void *, size_t, int) = tsym("HUF_decompress1X2_DCtx_wksp");
    size_t (*dec1Xw)(unsigned *, void *, size_t, const void *, size_t, void *, size_t, int) = tsym("HUF_decompress1X_DCtx_wksp");
    size_t (*dec4Xhuf)(unsigned *, void *, size_t, const void *, size_t, void *, size_t, int) = tsym("HUF_decompress4X_hufOnly_wksp");
    unsigned (*selDec)(size_t, size_t) = tsym("HUF_selectDecoder");
    unsigned (*isErr)(size_t) = tsym("HUF_isError");

    for (unsigned s = 0; s < 40; s++)
        for (unsigned t = 0; t < 12; t++)
            printf("selectDecoder(%u,%u)=%u\n", s * 997, t, selDec(s * 997, t));

    size_t lens[] = {32, 100, 1000, 6000, 40000, 128000};
    for (int mode = 0; mode < 7; mode++) {
        for (unsigned li = 0; li < sizeof(lens)/sizeof(lens[0]); li++) {
            size_t L = lens[li];
            unsigned maxSymbol = 255;
            for (size_t i = 0; i < L; i++) {
                unsigned r = rnd();
                switch (mode) {
                case 0: src[i] = r & 0xFF; break;                       /* flat */
                case 1: src[i] = (r & 0xFFFF) < 50000 ? 7 : (r & 0xFF); break;  /* skewed */
                case 2: src[i] = r & 0x0F; break;
                case 3: src[i] = (r & 1) ? 3 : 200; break;              /* 2 symbols */
                case 4: src[i] = (unsigned char)(i & 0x3F); break;
                case 5: src[i] = (r & 0xFF) < 3 ? (r & 0xFF) : 42; break; /* very skewed */
                default: src[i] = (unsigned char)((r % 100) < 90 ? (r & 7) : (r & 0xFF)); break;
                }
            }
            unsigned msv = maxSymbol;
            size_t mc = hist(count, &msv, src, L, wksp, sizeof(wksp));
            if (isErr(mc) || mc == L) { printf("m=%d L=%zu skip\n", mode, L); continue; }
            unsigned tl = hufOptTL(11, L, msv, wksp, sizeof(wksp), cts, count, 0);
            memset(cts, 0, sizeof(cts));
            size_t bc = buildCT(cts, count, msv, tl, wksp, sizeof(wksp));
            if (isErr(bc)) { printf("m=%d L=%zu bcErr\n", mode, L); continue; }
            size_t hs = writeCT(hdr, sizeof(hdr), cts, msv, (unsigned)bc, wksp, sizeof(wksp));
            if (isErr(hs)) { printf("m=%d L=%zu wcErr\n", mode, L); continue; }

            /* ---- X1/X2 DTable construction from the header ---- */
            for (int which = 0; which < 2; which++) {
                if (which) initX2(dtable, 12); else initX1(dtable, 12);
                size_t r = which ? readX2(dtable, hdr, hs, wksp, sizeof(wksp), 0)
                                 : readX1(dtable, hdr, hs, wksp, sizeof(wksp), 0);
                unsigned long long h = 1469598103934665603ULL;
                if (!isErr(r)) {
                    unsigned n = 1u + (1u << (dtable[0] & 0xFF));
                    for (unsigned i = 0; i < n && i < 8192; i++) { h ^= dtable[i]; h *= 1099511628211ULL; }
                }
                printf("m=%d L=%zu %s read=%zu desc=%08x hash=%llu\n",
                       mode, L, which ? "X2" : "X1", r, dtable[0], h);
            }

            /* ---- 4-stream ---- */
            size_t c4 = comp4X(cbuf, sizeof(cbuf), src, L, cts, 0);
            if (!isErr(c4) && c4 != 0) {
                memcpy(dbuf, hdr, hs); memcpy(dbuf + hs, cbuf, c4);
                size_t tot = hs + c4;
                memset(cbuf, 0, L + 16);
                size_t d = (initX1(dctx,12), dec4Xhuf(dctx, cbuf, L, dbuf, tot, wksp, sizeof(wksp), 0));
                printf("  4X hufOnly=%zu ok=%d\n", d, (!isErr(d) && d == L && !memcmp(cbuf, src, L)));
                /* usingDTable with X1 and X2 tables */
                for (int which = 0; which < 2; which++) {
                    if (which) initX2(dtable, 12); else initX1(dtable, 12);
                    size_t rr = which ? readX2(dtable, hdr, hs, wksp, sizeof(wksp), 0)
                                      : readX1(dtable, hdr, hs, wksp, sizeof(wksp), 0);
                    if (isErr(rr)) { printf("  4X using%s tblErr=%zu\n", which?"X2":"X1", rr); continue; }
                    memset(cbuf, 0, L + 16);
                    size_t d2 = dec4Xusing(cbuf, L, dbuf + rr, tot - rr, dtable, 0);
                    printf("  4X using%s=%zu ok=%d\n", which ? "X2" : "X1", d2,
                           (!isErr(d2) && d2 == L && !memcmp(cbuf, src, L)));
                }
                /* truncated / corrupted */
                for (int k = 0; k < 5; k++) {
                    static unsigned char tmp[300000];
                    memcpy(tmp, dbuf, tot);
                    tmp[hs + ((k * 13 + 3) % (tot - hs))] ^= 0x5A;
                    size_t d3 = (initX1(dctx,12), dec4Xhuf(dctx, cbuf, L, tmp, tot, wksp, sizeof(wksp), 0));
                    printf("  4X corrupt%d=%zu isErr=%u\n", k, d3, isErr(d3));
                }
                /* NOTE: truncating cSrc below the header size makes the C
                 * HUF_* entry points read out of bounds (they rely on their
                 * zstd-internal callers for that check), so it is not tested. */
                size_t d5 = (initX1(dctx,12), dec4Xhuf(dctx, cbuf, L / 2, dbuf, tot, wksp, sizeof(wksp), 0));
                printf("  4X smalldst=%zu isErr=%u\n", d5, isErr(d5));
                /* An undersized workspace is also not a supported input for
                 * the raw HUF_* entry points (the C crashes as well). */
            } else {
                printf("  4X notcompressible ret=%zu\n", c4);
            }

            /* ---- 1-stream ---- */
            size_t c1 = comp1X(cbuf, sizeof(cbuf), src, L, cts, 0);
            if (!isErr(c1) && c1 != 0) {
                memcpy(dbuf, hdr, hs); memcpy(dbuf + hs, cbuf, c1);
                size_t tot = hs + c1;
                for (int fn = 0; fn < 3; fn++) {
                    memset(cbuf, 0, L + 16);
                    size_t d = fn == 0 ? (initX1(dctx,12), dec1X1w(dctx, cbuf, L, dbuf, tot, wksp, sizeof(wksp), 0))
                             : fn == 1 ? (initX2(dctx,12), dec1X2w(dctx, cbuf, L, dbuf, tot, wksp, sizeof(wksp), 0))
                                       : (initX1(dctx,12), dec1Xw(dctx, cbuf, L, dbuf, tot, wksp, sizeof(wksp), 0));
                    printf("  1X fn%d=%zu ok=%d\n", fn, d,
                           (!isErr(d) && d == L && !memcmp(cbuf, src, L)));
                }
                for (int which = 0; which < 2; which++) {
                    if (which) initX2(dtable, 12); else initX1(dtable, 12);
                    size_t rr = which ? readX2(dtable, hdr, hs, wksp, sizeof(wksp), 0)
                                      : readX1(dtable, hdr, hs, wksp, sizeof(wksp), 0);
                    if (isErr(rr)) continue;
                    memset(cbuf, 0, L + 16);
                    size_t d2 = dec1Xusing(cbuf, L, dbuf + rr, tot - rr, dtable, 0);
                    printf("  1X using%s=%zu ok=%d\n", which ? "X2" : "X1", d2,
                           (!isErr(d2) && d2 == L && !memcmp(cbuf, src, L)));
                }
                /* Corrupted-input and undersized-dst behaviour of the *raw* HUF
                 * 1-stream entry points is not exercised here: those functions
                 * rely on zstd_decompress_block for bounds validation and the C
                 * reference itself faults on such inputs. The hardened paths are
                 * covered end-to-end through ZSTD_decompress instead. */
            } else {
                printf("  1X notcompressible ret=%zu\n", c1);
            }

            /* also exercise the disableFast flag and bmi2 flag bits */
            for (int flags = 0; flags < 64; flags += 7) {
                size_t c = comp4X(cbuf, sizeof(cbuf), src, L, cts, 0);
                if (isErr(c) || c == 0) continue;
                memcpy(dbuf, hdr, hs); memcpy(dbuf + hs, cbuf, c);
                static unsigned char o[300000];
                size_t d = (initX1(dctx,12), dec4Xhuf(dctx, o, L, dbuf, hs + c, wksp, sizeof(wksp), flags));
                printf("  flags=%d 4X=%zu ok=%d\n", flags, d,
                       (!isErr(d) && d == L && !memcmp(o, src, L)));
            }
        }
    }
    fflush(stdout);
    return 0;
}
