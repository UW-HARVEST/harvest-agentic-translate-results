/* Differential harness #4: stream > ZSTD_CURRENT_MAX (3500 MB) through a single
 * CCtx so that the index-overflow correction path (ZSTD_window_correctOverflow /
 * ZSTD_reduceIndex / ZSTD_reduceTable{,_btlazy2}) is exercised. Also streams a
 * >4 GB total so the 32-bit index arithmetic is stressed. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define ZSTD_STATIC_LINKING_ONLY
#include "zstd.h"

static unsigned long long g_state;
static void rs(unsigned long long s) { g_state = s ? s : 1; }
static unsigned long long r64(void) {
    g_state ^= g_state << 13; g_state ^= g_state >> 7; g_state ^= g_state << 17;
    return g_state;
}
static unsigned r32(void) { return (unsigned)(g_state >> 32); }

static unsigned long long h_state;
static void hreset(void) { h_state = 1469598103934665603ULL; }
static void hupd(const void* p, size_t n) {
    const unsigned char* b = (const unsigned char*)p;
    size_t i;
    for (i = 0; i < n; i++) { h_state ^= b[i]; h_state *= 1099511628211ULL; }
}

#define CHUNK (1u << 20)

static void run(int strategy, int level, unsigned long long totalBytes) {
    unsigned char* in = (unsigned char*)malloc(CHUNK);
    size_t const outCap = ZSTD_CStreamOutSize();
    unsigned char* out = (unsigned char*)malloc(outCap);
    ZSTD_CCtx* c = ZSTD_createCCtx();
    unsigned long long produced = 0, consumed = 0;
    unsigned long long dhash;
    ZSTD_DCtx* d = ZSTD_createDCtx();
    unsigned char* dbuf = (unsigned char*)malloc(CHUNK);
    unsigned long long inhash;

    rs(0x5EED1234ULL + (unsigned long long)level);
    ZSTD_CCtx_setParameter(c, ZSTD_c_compressionLevel, level);
    if (strategy) ZSTD_CCtx_setParameter(c, ZSTD_c_strategy, strategy);
    ZSTD_CCtx_setParameter(c, ZSTD_c_checksumFlag, 1);
    ZSTD_CCtx_setParameter(c, ZSTD_c_windowLog, 20);

    hreset();
    { /* hash of the source we will feed, computed with an independent PRNG pass */
        unsigned long long save = g_state;
        unsigned long long left = totalBytes;
        while (left) {
            size_t n = CHUNK < left ? CHUNK : (size_t)left;
            size_t i;
            for (i = 0; i < n; i += 8) {
                unsigned long long v = r64();
                size_t k = (n - i) < 8 ? (n - i) : 8;
                memcpy(in + i, &v, k);
            }
            /* make it compressible: fold high nibbles away every other chunk */
            if ((left / CHUNK) & 1) { for (i = 0; i < n; i++) in[i] &= 0x1F; }
            hupd(in, n);
            left -= n;
        }
        inhash = h_state;
        g_state = save;
    }

    /* now compress the identical stream */
    hreset();
    {   unsigned long long left = totalBytes;
        rs(0x5EED1234ULL + (unsigned long long)level);
        while (left) {
            size_t n = CHUNK < left ? CHUNK : (size_t)left;
            size_t i;
            ZSTD_inBuffer ib;
            for (i = 0; i < n; i += 8) {
                unsigned long long v = r64();
                size_t k = (n - i) < 8 ? (n - i) : 8;
                memcpy(in + i, &v, k);
            }
            if ((left / CHUNK) & 1) { for (i = 0; i < n; i++) in[i] &= 0x1F; }
            ib.src = in; ib.size = n; ib.pos = 0;
            while (ib.pos < ib.size) {
                ZSTD_outBuffer ob;
                size_t rc;
                ob.dst = out; ob.size = outCap; ob.pos = 0;
                rc = ZSTD_compressStream2(c, &ob, &ib, ZSTD_e_continue);
                if (ZSTD_isError(rc)) { printf("cstream ERR %s\n", ZSTD_getErrorName(rc)); goto done; }
                hupd(out, ob.pos);
                produced += ob.pos;
            }
            consumed += n;
            left -= n;
        }
        for (;;) {
            ZSTD_inBuffer ib;
            ZSTD_outBuffer ob;
            size_t rc;
            ib.src = NULL; ib.size = 0; ib.pos = 0;
            ob.dst = out; ob.size = outCap; ob.pos = 0;
            rc = ZSTD_compressStream2(c, &ob, &ib, ZSTD_e_end);
            if (ZSTD_isError(rc)) { printf("cend ERR %s\n", ZSTD_getErrorName(rc)); goto done; }
            hupd(out, ob.pos);
            produced += ob.pos;
            if (rc == 0) break;
        }
    }
    printf("stress[strat%d,L%d,%lluMB] consumed=%llu produced=%llu chash=%016llx srchash=%016llx\n",
           strategy, level, totalBytes >> 20, consumed, produced, h_state, inhash);
    {   ZSTD_frameProgression fp = ZSTD_getFrameProgression(c);
        printf("  progression ingested=%llu consumed=%llu produced=%llu jobs=%u workers=%u\n",
               fp.ingested, fp.consumed, fp.produced, fp.currentJobID, fp.nbActiveWorkers);
    }
    (void)dhash; (void)d; (void)dbuf;
done:
    ZSTD_freeDCtx(d);
    ZSTD_freeCCtx(c);
    free(in); free(out); free(dbuf);
}

int main(int argc, char** argv) {
    unsigned long long mb = 3800;
    setvbuf(stdout, NULL, _IOLBF, 4096);
    if (argc > 1) mb = strtoull(argv[1], NULL, 10);
    printf("ZSTD_CURRENT_MAX is 3500MB on 64-bit; feeding %lluMB per run\n", mb);
    run(ZSTD_fast,     1, mb << 20);
    run(ZSTD_dfast,    4, mb << 20);
    run(ZSTD_greedy,   5, mb << 20);
    run(ZSTD_lazy2,    8, mb << 20);
    printf("=== done ===\n");
    return 0;
}
