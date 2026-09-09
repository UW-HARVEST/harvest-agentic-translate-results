/* Differential harness #3: large inputs (LDM / overflow correction / multi-block),
 * every strategy explicitly pinned, and corrupted-stream decoding. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define ZSTD_STATIC_LINKING_ONLY
#include "zstd.h"

static unsigned long long g_state = 88172645463325252ULL;
static void rs(unsigned long long s) { g_state = s ? s : 1; }
static unsigned long long r64(void) {
    g_state ^= g_state << 13; g_state ^= g_state >> 7; g_state ^= g_state << 17;
    return g_state;
}
static unsigned r32(void) { return (unsigned)(r64() >> 32); }

static unsigned long long fnv(const void* p, size_t n) {
    const unsigned char* b = (const unsigned char*)p;
    unsigned long long h = 1469598103934665603ULL;
    size_t i;
    for (i = 0; i < n; i++) { h ^= b[i]; h *= 1099511628211ULL; }
    return h;
}

#define SRCSZ (6u*1024u*1024u)
static unsigned char *src, *cmp, *dec;
static size_t cmpCap, decCap;

/* corpus with long-range redundancy: blocks repeated at large distances */
static void fill_ldm(unsigned char* d, size_t n) {
    size_t i = 0;
    size_t const unit = 4096;
    size_t nbUnits = 24;
    unsigned char* pool = (unsigned char*)malloc(unit * nbUnits);
    size_t u;
    for (u = 0; u < unit * nbUnits; u++) pool[u] = (unsigned char)(r32() & 0x0F);
    while (i < n) {
        size_t const which = r32() % nbUnits;
        size_t l = unit < n - i ? unit : n - i;
        memcpy(d + i, pool + which * unit, l);
        i += l;
    }
    free(pool);
}
static void fill_mixed(unsigned char* d, size_t n) {
    size_t i = 0;
    while (i < n) {
        unsigned mode = r32() % 4;
        size_t run = 1 + (r32() % 20000);
        size_t j;
        if (run > n - i) run = n - i;
        switch (mode) {
        case 0: for (j = 0; j < run; j++) d[i+j] = (unsigned char)r32(); break;
        case 1: memset(d + i, (int)(r32() & 0xFF), run); break;
        case 2: for (j = 0; j < run; j++) d[i+j] = (unsigned char)('a' + (r32() % 6)); break;
        default:
            if (i >= 65536) { memcpy(d + i, d + i - 65536, run); }
            else for (j = 0; j < run; j++) d[i+j] = (unsigned char)(j & 0x1F);
            break;
        }
        i += run;
    }
}

static void phase_strategies(void) {
    static const size_t sizes[] = {200000, 1500000, 6u*1024u*1024u};
    int strat, si, ldm, wlog;
    printf("=== strategies x sizes x ldm ===\n");
    for (si = 0; si < 3; si++) {
        size_t n = sizes[si];
        rs(1000 + si);
        if (si == 2) fill_ldm(src, n); else fill_mixed(src, n);
        printf("corpus[%d] size=%zu fnv=%016llx\n", si, n, fnv(src, n));
        for (strat = 1; strat <= 9; strat++) {
            for (ldm = 0; ldm <= 1; ldm++) {
                for (wlog = 0; wlog < 2; wlog++) {
                    ZSTD_CCtx* c = ZSTD_createCCtx();
                    size_t cs;
                    char nm[128];
                    ZSTD_CCtx_setParameter(c, ZSTD_c_strategy, strat);
                    ZSTD_CCtx_setParameter(c, ZSTD_c_enableLongDistanceMatching, ldm);
                    ZSTD_CCtx_setParameter(c, ZSTD_c_windowLog, wlog ? 17 : 22);
                    ZSTD_CCtx_setParameter(c, ZSTD_c_checksumFlag, 1);
                    cs = ZSTD_compress2(c, cmp, cmpCap, src, n);
                    snprintf(nm, sizeof nm, "strat%d,ldm%d,wl%d,sz%zu", strat, ldm, wlog?17:22, n);
                    if (ZSTD_isError(cs)) { printf("%-40s ERR %s\n", nm, ZSTD_getErrorName(cs)); }
                    else {
                        size_t ds;
                        printf("%-40s csize=%9zu fnv=%016llx\n", nm, cs, fnv(cmp, cs));
                        ds = ZSTD_decompress(dec, decCap, cmp, cs);
                        if (ds != n || memcmp(dec, src, n)) printf("%-40s ROUNDTRIP-FAIL ds=%zu\n", nm, ds);
                    }
                    ZSTD_freeCCtx(c);
                }
            }
        }
    }
}

static void phase_streaming_big(void) {
    size_t n = 4u*1024u*1024u;
    int lvl;
    printf("=== big streaming ===\n");
    rs(4321);
    fill_mixed(src, n);
    printf("corpus fnv=%016llx\n", fnv(src, n));
    for (lvl = -5; lvl <= 22; lvl += 3) {
        ZSTD_CCtx* c = ZSTD_createCCtx();
        size_t total = 0, pos = 0;
        char nm[64];
        ZSTD_CCtx_setParameter(c, ZSTD_c_compressionLevel, lvl);
        ZSTD_CCtx_setParameter(c, ZSTD_c_checksumFlag, 1);
        while (pos < n) {
            ZSTD_inBuffer in; ZSTD_outBuffer out;
            size_t take = 1 + (r32() % 200000);
            if (take > n - pos) take = n - pos;
            in.src = src + pos; in.size = take; in.pos = 0;
            out.dst = cmp + total; out.size = cmpCap - total; out.pos = 0;
            ZSTD_compressStream2(c, &out, &in, ZSTD_e_continue);
            total += out.pos; pos += in.pos;
            if (in.pos != take) { /* need to flush more */
                for (;;) {
                    ZSTD_inBuffer in2; ZSTD_outBuffer out2;
                    size_t rc;
                    in2.src = src + pos; in2.size = take - in.pos; in2.pos = 0;
                    out2.dst = cmp + total; out2.size = cmpCap - total; out2.pos = 0;
                    rc = ZSTD_compressStream2(c, &out2, &in2, ZSTD_e_continue);
                    total += out2.pos; pos += in2.pos;
                    if (in2.pos == in2.size || ZSTD_isError(rc)) break;
                }
            }
        }
        for (;;) {
            ZSTD_inBuffer in; ZSTD_outBuffer out;
            size_t rem;
            in.src = NULL; in.size = 0; in.pos = 0;
            out.dst = cmp + total; out.size = cmpCap - total; out.pos = 0;
            rem = ZSTD_compressStream2(c, &out, &in, ZSTD_e_end);
            total += out.pos;
            if (rem == 0 || ZSTD_isError(rem)) break;
        }
        snprintf(nm, sizeof nm, "bigstream[L%d]", lvl);
        printf("%-24s csize=%9zu fnv=%016llx\n", nm, total, fnv(cmp, total));
        {   size_t ds = ZSTD_decompress(dec, decCap, cmp, total);
            if (ds != n || memcmp(dec, src, n)) printf("%-24s ROUNDTRIP-FAIL ds=%zu\n", nm, ds);
        }
        printf("%-24s toFlushNow=%zu\n", nm, ZSTD_toFlushNow(c));
        ZSTD_freeCCtx(c);
    }
}

static void phase_corrupt(void) {
    size_t n = 120000;
    int lvl;
    printf("=== corrupted streams ===\n");
    rs(24680);
    fill_mixed(src, n);
    for (lvl = 1; lvl <= 19; lvl += 6) {
        size_t cs = ZSTD_compress(cmp, cmpCap, src, n, lvl);
        unsigned char* work = (unsigned char*)malloc(cs + 64);
        int t;
        printf("base[L%d] csize=%zu fnv=%016llx\n", lvl, cs, fnv(cmp, cs));
        for (t = 0; t < 4000; t++) {
            size_t pos = r32() % cs;
            unsigned char mask = (unsigned char)(1u << (r32() & 7));
            size_t ds, rc2;
            memcpy(work, cmp, cs);
            work[pos] ^= mask;
            memset(dec, 0xA5, 4096);
            ds = ZSTD_decompress(dec, decCap, work, cs);
            printf("flip[%d,%zu,%02x] rc=%zd", lvl, pos, mask, (ptrdiff_t)ds);
            if (ZSTD_isError(ds)) printf(" %s\n", ZSTD_getErrorName(ds));
            else printf(" fnv=%016llx\n", fnv(dec, ds));
            /* also truncated / streaming */
            rc2 = ZSTD_getFrameContentSize(work, cs);
            printf("  fcs=%llu fcsz=%zd margin=%zd\n",
                   (unsigned long long)ZSTD_getFrameContentSize(work, cs),
                   (ptrdiff_t)ZSTD_findFrameCompressedSize(work, cs),
                   (ptrdiff_t)ZSTD_decompressionMargin(work, cs));
            (void)rc2;
        }
        /* truncations */
        for (t = 1; t < 400; t++) {
            size_t len = (cs * (size_t)t) / 400;
            size_t ds;
            memset(dec, 0x5A, 4096);
            ds = ZSTD_decompress(dec, decCap, cmp, len);
            printf("trunc[%d,%zu] rc=%zd", lvl, len, (ptrdiff_t)ds);
            if (ZSTD_isError(ds)) printf(" %s\n", ZSTD_getErrorName(ds));
            else printf(" fnv=%016llx\n", fnv(dec, ds));
        }
        /* dstCapacity too small */
        for (t = 0; t < 200; t++) {
            size_t cap = r32() % (n + 16);
            size_t ds;
            memset(dec, 0x33, 4096);
            ds = ZSTD_decompress(dec, cap, cmp, cs);
            printf("smalldst[%d,%zu] rc=%zd", lvl, cap, (ptrdiff_t)ds);
            if (ZSTD_isError(ds)) printf(" %s\n", ZSTD_getErrorName(ds));
            else printf(" fnv=%016llx\n", fnv(dec, ds));
        }
        free(work);
    }
}

static void phase_random_input(void) {
    int t;
    unsigned char buf[512];
    printf("=== random buffers into decoder ===\n");
    rs(0xFACE);
    for (t = 0; t < 60000; t++) {
        size_t len = 1 + (r32() % sizeof buf);
        size_t i, ds;
        for (i = 0; i < len; i++) buf[i] = (unsigned char)r32();
        if (t % 3 == 0) { /* give it a valid magic sometimes */
            if (len >= 4) { buf[0]=0x28; buf[1]=0xB5; buf[2]=0x2F; buf[3]=0xFD; }
        }
        memset(dec, 0xC3, 8192);
        ds = ZSTD_decompress(dec, 8192, buf, len);
        printf("rnd[%d,%zu] rc=%zd fh=%zd frame=%u skip=%u fcs=%lld",
               t, len, (ptrdiff_t)ds,
               (ptrdiff_t)ZSTD_frameHeaderSize(buf, len),
               ZSTD_isFrame(buf, len), ZSTD_isSkippableFrame(buf, len),
               (long long)ZSTD_getFrameContentSize(buf, len));
        if (!ZSTD_isError(ds)) printf(" fnv=%016llx", fnv(dec, ds));
        printf("\n");
        {   ZSTD_FrameHeader zfh;
            size_t rc = ZSTD_getFrameHeader(&zfh, buf, len);
            printf("  gfh rc=%zd", (ptrdiff_t)rc);
            if (rc == 0) printf(" fcs=%llu ws=%llu bsm=%u ft=%d hs=%u did=%u ck=%u",
                                zfh.frameContentSize, zfh.windowSize, zfh.blockSizeMax,
                                (int)zfh.frameType, zfh.headerSize, zfh.dictID, zfh.checksumFlag);
            printf("\n");
        }
    }
}

int main(void) {
    cmpCap = ZSTD_compressBound(SRCSZ) + 4096;
    decCap = SRCSZ + 4096;
    src = (unsigned char*)malloc(SRCSZ);
    cmp = (unsigned char*)malloc(cmpCap);
    dec = (unsigned char*)malloc(decCap);
    setvbuf(stdout, NULL, _IOFBF, 1 << 22);
    phase_random_input();
    phase_corrupt();
    phase_strategies();
    phase_streaming_big();
    printf("=== done ===\n");
    return 0;
}
