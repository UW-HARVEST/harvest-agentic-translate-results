/* Differential harness: exercises a wide slice of the public zstd API and prints
 * a deterministic textual trace. Link once against the C libzstd.so and once
 * against the Rust libzstd.so, then diff the two traces. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define ZSTD_STATIC_LINKING_ONLY
#include "zstd.h"
#define ZDICT_STATIC_LINKING_ONLY
#include "zdict.h"

/* ---- deterministic PRNG (xorshift64) ---- */
static unsigned long long g_state = 88172645463325252ULL;
static void rs(unsigned long long s) { g_state = s ? s : 1; }
static unsigned long long r64(void) {
    g_state ^= g_state << 13; g_state ^= g_state >> 7; g_state ^= g_state << 17;
    return g_state;
}
static unsigned r32(void) { return (unsigned)(r64() >> 32); }

/* FNV-1a over a buffer, so we don't print megabytes */
static unsigned long long fnv(const void* p, size_t n) {
    const unsigned char* b = (const unsigned char*)p;
    unsigned long long h = 1469598103934665603ULL;
    size_t i;
    for (i = 0; i < n; i++) { h ^= b[i]; h *= 1099511628211ULL; }
    return h;
}
#define SHOW(name, buf, n) printf("%-44s size=%8zu fnv=%016llx\n", name, (size_t)(n), fnv(buf, n))
#define SHOWZ(name, v)     printf("%-44s %lld\n", name, (long long)(v))
#define SHOWU(name, v)     printf("%-44s %llu\n", name, (unsigned long long)(v))

/* ---- corpora ---- */
static void fill_random(unsigned char* d, size_t n)      { size_t i; for (i=0;i<n;i++) d[i] = (unsigned char)r32(); }
static void fill_zero(unsigned char* d, size_t n)        { memset(d, 0, n); }
static void fill_textish(unsigned char* d, size_t n) {
    static const char* words[] = {"the ","quick ","brown ","fox ","jumps ","over ","lazy ","dog ",
                                  "zstandard ","compression ","library ","test ","data ","abc ","xyz "};
    size_t i = 0;
    while (i < n) {
        const char* w = words[r32() % 15];
        size_t l = strlen(w);
        if (i + l > n) l = n - i;
        memcpy(d + i, w, l);
        i += l;
    }
}
static void fill_periodic(unsigned char* d, size_t n)    { size_t i; for (i=0;i<n;i++) d[i] = (unsigned char)(i % 251); }
static void fill_lowentropy(unsigned char* d, size_t n)  { size_t i; for (i=0;i<n;i++) d[i] = (unsigned char)(r32() & 3); }

typedef void (*filler)(unsigned char*, size_t);
static const char* kNames[] = {"rand","zero","text","period","lowent"};
static const filler kFill[] = {fill_random, fill_zero, fill_textish, fill_periodic, fill_lowentropy};

#define MAXSRC (600u*1024u)

static unsigned char* src;
static unsigned char* cmp;
static unsigned char* dec;
static size_t cmpCap, decCap;

static void test_simple(void) {
    static const size_t sizes[] = {0,1,2,3,7,15,16,17,63,64,100,999,1024,4095,4096,
                                   16384,65535,65536,131072,131073,300000,600000};
    int ci, si, k;
    printf("=== simple compress/decompress ===\n");
    for (k = 0; k < 5; k++) {
        for (si = 0; si < (int)(sizeof(sizes)/sizeof(sizes[0])); si++) {
            size_t n = sizes[si];
            rs(0x1234567 + 7u*k + 13u*si);
            kFill[k](src, n);
            for (ci = -7; ci <= 22; ci++) {
                size_t cs = ZSTD_compress(cmp, cmpCap, src, n, ci);
                char nm[128];
                snprintf(nm, sizeof nm, "c[%s,%zu,L%d]", kNames[k], n, ci);
                if (ZSTD_isError(cs)) { printf("%-44s ERR %s\n", nm, ZSTD_getErrorName(cs)); continue; }
                SHOW(nm, cmp, cs);
                {   size_t ds = ZSTD_decompress(dec, decCap, cmp, cs);
                    if (ZSTD_isError(ds)) { printf("%-44s DERR %s\n", nm, ZSTD_getErrorName(ds)); continue; }
                    if (ds != n || (n && memcmp(dec, src, n))) printf("%-44s ROUNDTRIP-FAIL\n", nm);
                }
                SHOWU("  frameContentSize", ZSTD_getFrameContentSize(cmp, cs));
                SHOWZ("  findFrameCompressedSize", ZSTD_findFrameCompressedSize(cmp, cs));
                SHOWU("  decompressBound", ZSTD_decompressBound(cmp, cs));
                SHOWZ("  decompressionMargin", ZSTD_decompressionMargin(cmp, cs));
            }
        }
    }
}

static void test_cctx_params(void) {
    int ci;
    printf("=== cctx advanced params ===\n");
    rs(999);
    fill_textish(src, 200000);
    for (ci = -3; ci <= 19; ci += 2) {
        ZSTD_CCtx* c = ZSTD_createCCtx();
        int wl;
        ZSTD_CCtx_setParameter(c, ZSTD_c_compressionLevel, ci);
        ZSTD_CCtx_setParameter(c, ZSTD_c_checksumFlag, 1);
        ZSTD_CCtx_setParameter(c, ZSTD_c_contentSizeFlag, 1);
        ZSTD_CCtx_setParameter(c, ZSTD_c_enableLongDistanceMatching, ci & 1);
        ZSTD_CCtx_setParameter(c, ZSTD_c_targetCBlockSize, (ci > 0) ? 2000 : 0);
        for (wl = 0; wl < 3; wl++) {
            size_t cs;
            char nm[128];
            ZSTD_CCtx_setParameter(c, ZSTD_c_useRowMatchFinder, wl);
            ZSTD_CCtx_setParameter(c, ZSTD_c_blockSplitterLevel, wl);
            ZSTD_CCtx_reset(c, ZSTD_reset_session_only);
            cs = ZSTD_compress2(c, cmp, cmpCap, src, 200000);
            snprintf(nm, sizeof nm, "adv[L%d,rmf%d]", ci, wl);
            if (ZSTD_isError(cs)) { printf("%-44s ERR %s\n", nm, ZSTD_getErrorName(cs)); continue; }
            SHOW(nm, cmp, cs);
            {   size_t ds = ZSTD_decompress(dec, decCap, cmp, cs);
                if (ds != 200000 || memcmp(dec, src, 200000)) printf("%-44s ROUNDTRIP-FAIL\n", nm);
            }
        }
        SHOWZ("  sizeof_CCtx", ZSTD_sizeof_CCtx(c));
        ZSTD_freeCCtx(c);
    }
}

static void test_streaming(void) {
    static const size_t chunks[] = {1, 7, 1000, 66000};
    int ci, k;
    size_t ci2;
    printf("=== streaming ===\n");
    for (k = 0; k < 3; k++) {
        rs(4242 + k);
        kFill[k](src, 300000);
        for (ci = 1; ci <= 12; ci += 5) {
            for (ci2 = 0; ci2 < 4; ci2++) {
                size_t inChunk = chunks[ci2];
                ZSTD_CStream* zcs = ZSTD_createCStream();
                size_t total = 0, pos = 0;
                char nm[128];
                ZSTD_initCStream(zcs, ci);
                ZSTD_CCtx_setPledgedSrcSize(zcs, 300000);
                while (pos < 300000) {
                    ZSTD_inBuffer in;
                    ZSTD_outBuffer out;
                    size_t take = inChunk < 300000 - pos ? inChunk : 300000 - pos;
                    in.src = src + pos; in.size = take; in.pos = 0;
                    out.dst = cmp + total; out.size = cmpCap - total; out.pos = 0;
                    ZSTD_compressStream(zcs, &out, &in);
                    total += out.pos;
                    pos += in.pos;
                }
                for (;;) {
                    ZSTD_outBuffer out;
                    size_t rem;
                    out.dst = cmp + total; out.size = cmpCap - total; out.pos = 0;
                    rem = ZSTD_endStream(zcs, &out);
                    total += out.pos;
                    if (rem == 0) break;
                    if (ZSTD_isError(rem)) { printf("endStream ERR\n"); break; }
                }
                ZSTD_freeCStream(zcs);
                snprintf(nm, sizeof nm, "stream[%s,L%d,chunk%zu]", kNames[k], ci, inChunk);
                SHOW(nm, cmp, total);
                /* streaming decompression */
                {   ZSTD_DStream* zds = ZSTD_createDStream();
                    size_t dpos = 0, cpos = 0;
                    ZSTD_initDStream(zds);
                    while (cpos < total) {
                        ZSTD_inBuffer in;
                        ZSTD_outBuffer out;
                        size_t take = 4096 < total - cpos ? 4096 : total - cpos;
                        size_t rc;
                        in.src = cmp + cpos; in.size = take; in.pos = 0;
                        out.dst = dec + dpos; out.size = decCap - dpos; out.pos = 0;
                        rc = ZSTD_decompressStream(zds, &out, &in);
                        if (ZSTD_isError(rc)) { printf("  dstream ERR %s\n", ZSTD_getErrorName(rc)); break; }
                        dpos += out.pos; cpos += in.pos;
                    }
                    ZSTD_freeDStream(zds);
                    if (dpos != 300000 || memcmp(dec, src, 300000)) printf("%-44s STREAM-ROUNDTRIP-FAIL\n", nm);
                }
            }
        }
    }
}

static void test_dict(void) {
    size_t const nbSamples = 64;
    size_t sampleSizes[64];
    size_t i, totalSize = 0;
    unsigned char* samples;
    unsigned char* dictBuf = (unsigned char*)malloc(112640);
    printf("=== dictionaries ===\n");
    rs(0xDEADBEEF);
    for (i = 0; i < nbSamples; i++) { sampleSizes[i] = 2000 + (r32() % 3000); totalSize += sampleSizes[i]; }
    samples = (unsigned char*)malloc(totalSize);
    {   size_t off = 0;
        for (i = 0; i < nbSamples; i++) { fill_textish(samples + off, sampleSizes[i]); off += sampleSizes[i]; }
    }
    {   size_t ds = ZDICT_trainFromBuffer(dictBuf, 32768, samples, sampleSizes, (unsigned)nbSamples);
        if (ZDICT_isError(ds)) { printf("trainFromBuffer ERR %s\n", ZDICT_getErrorName(ds)); }
        else {
            SHOW("dict_default", dictBuf, ds);
            SHOWU("  dictID", ZDICT_getDictID(dictBuf, ds));
            SHOWZ("  dictHeaderSize", ZDICT_getDictHeaderSize(dictBuf, ds));
            /* use it */
            {   int ci;
                for (ci = 1; ci <= 19; ci += 6) {
                    ZSTD_CDict* cd = ZSTD_createCDict(dictBuf, ds, ci);
                    ZSTD_CCtx* c = ZSTD_createCCtx();
                    ZSTD_DCtx* d = ZSTD_createDCtx();
                    ZSTD_DDict* dd = ZSTD_createDDict(dictBuf, ds);
                    size_t cs = ZSTD_compress_usingCDict(c, cmp, cmpCap, samples, sampleSizes[0], cd);
                    char nm[64];
                    snprintf(nm, sizeof nm, "dictcompress[L%d]", ci);
                    if (ZSTD_isError(cs)) printf("%-44s ERR %s\n", nm, ZSTD_getErrorName(cs));
                    else {
                        size_t dz;
                        SHOW(nm, cmp, cs);
                        dz = ZSTD_decompress_usingDDict(d, dec, decCap, cmp, cs, dd);
                        if (dz != sampleSizes[0] || memcmp(dec, samples, sampleSizes[0]))
                            printf("%-44s DICT-ROUNDTRIP-FAIL\n", nm);
                    }
                    SHOWU("  cdict dictID", ZSTD_getDictID_fromCDict(cd));
                    SHOWU("  ddict dictID", ZSTD_getDictID_fromDDict(dd));
                    SHOWZ("  sizeof_CDict", ZSTD_sizeof_CDict(cd));
                    SHOWZ("  sizeof_DDict", ZSTD_sizeof_DDict(dd));
                    ZSTD_freeDDict(dd); ZSTD_freeDCtx(d); ZSTD_freeCCtx(c); ZSTD_freeCDict(cd);
                }
            }
        }
    }
    /* cover / fastcover */
    {   ZDICT_cover_params_t cp;
        ZDICT_fastCover_params_t fp;
        size_t ds;
        memset(&cp, 0, sizeof cp);
        cp.k = 200; cp.d = 8; cp.steps = 4; cp.nbThreads = 1; cp.zParams.compressionLevel = 3;
        ds = ZDICT_trainFromBuffer_cover(dictBuf, 16384, samples, sampleSizes, (unsigned)nbSamples, cp);
        if (ZDICT_isError(ds)) printf("cover ERR %s\n", ZDICT_getErrorName(ds));
        else SHOW("dict_cover", dictBuf, ds);

        memset(&cp, 0, sizeof cp);
        cp.steps = 4; cp.nbThreads = 1; cp.zParams.compressionLevel = 3;
        ds = ZDICT_optimizeTrainFromBuffer_cover(dictBuf, 16384, samples, sampleSizes, (unsigned)nbSamples, &cp);
        if (ZDICT_isError(ds)) printf("optcover ERR %s\n", ZDICT_getErrorName(ds));
        else { SHOW("dict_optcover", dictBuf, ds); SHOWZ("  k", cp.k); SHOWZ("  d", cp.d); }

        memset(&fp, 0, sizeof fp);
        fp.k = 200; fp.d = 8; fp.f = 20; fp.steps = 4; fp.nbThreads = 1; fp.splitPoint = 0;
        fp.accel = 1; fp.zParams.compressionLevel = 3;
        ds = ZDICT_trainFromBuffer_fastCover(dictBuf, 16384, samples, sampleSizes, (unsigned)nbSamples, fp);
        if (ZDICT_isError(ds)) printf("fastcover ERR %s\n", ZDICT_getErrorName(ds));
        else SHOW("dict_fastcover", dictBuf, ds);

        memset(&fp, 0, sizeof fp);
        fp.steps = 4; fp.nbThreads = 1; fp.zParams.compressionLevel = 3;
        ds = ZDICT_optimizeTrainFromBuffer_fastCover(dictBuf, 16384, samples, sampleSizes, (unsigned)nbSamples, &fp);
        if (ZDICT_isError(ds)) printf("optfastcover ERR %s\n", ZDICT_getErrorName(ds));
        else { SHOW("dict_optfastcover", dictBuf, ds); SHOWZ("  k", fp.k); SHOWZ("  d", fp.d); SHOWZ("  f", fp.f); }
    }
    /* legacy trainer */
    {   ZDICT_legacy_params_t lp;
        size_t ds;
        memset(&lp, 0, sizeof lp);
        lp.selectivityLevel = 9;
        lp.zParams.compressionLevel = 3;
        ds = ZDICT_trainFromBuffer_legacy(dictBuf, 16384, samples, sampleSizes, (unsigned)nbSamples, lp);
        if (ZDICT_isError(ds)) printf("legacytrain ERR %s\n", ZDICT_getErrorName(ds));
        else SHOW("dict_legacy", dictBuf, ds);
    }
    /* finalizeDictionary */
    {   unsigned char* raw = samples;
        ZDICT_params_t p;
        size_t ds;
        memset(&p, 0, sizeof p);
        p.compressionLevel = 3;
        ds = ZDICT_finalizeDictionary(dictBuf, 16384, raw, 8000, samples, sampleSizes, (unsigned)nbSamples, p);
        if (ZDICT_isError(ds)) printf("finalize ERR %s\n", ZDICT_getErrorName(ds));
        else SHOW("dict_finalize", dictBuf, ds);
    }
    free(samples);
    free(dictBuf);
}

static void test_bounds_and_misc(void) {
    int i;
    printf("=== bounds / misc ===\n");
    SHOWZ("versionNumber", ZSTD_versionNumber());
    printf("%-44s %s\n", "versionString", ZSTD_versionString());
    SHOWZ("minCLevel", ZSTD_minCLevel());
    SHOWZ("maxCLevel", ZSTD_maxCLevel());
    SHOWZ("defaultCLevel", ZSTD_defaultCLevel());
    SHOWZ("CStreamInSize", ZSTD_CStreamInSize());
    SHOWZ("CStreamOutSize", ZSTD_CStreamOutSize());
    SHOWZ("DStreamInSize", ZSTD_DStreamInSize());
    SHOWZ("DStreamOutSize", ZSTD_DStreamOutSize());
    SHOWZ("estimateDCtxSize", ZSTD_estimateDCtxSize());
    for (i = -22; i <= 22; i++) {
        char nm[64];
        snprintf(nm, sizeof nm, "estimateCCtxSize[%d]", i);
        SHOWZ(nm, ZSTD_estimateCCtxSize(i));
        snprintf(nm, sizeof nm, "estimateCStreamSize[%d]", i);
        SHOWZ(nm, ZSTD_estimateCStreamSize(i));
        snprintf(nm, sizeof nm, "estimateCDictSize[%d]", i);
        SHOWZ(nm, ZSTD_estimateCDictSize(100000, i));
    }
    {   int p;
        static const int cparams[] = {
            ZSTD_c_compressionLevel, ZSTD_c_windowLog, ZSTD_c_hashLog, ZSTD_c_chainLog,
            ZSTD_c_searchLog, ZSTD_c_minMatch, ZSTD_c_targetLength, ZSTD_c_strategy,
            ZSTD_c_targetCBlockSize, ZSTD_c_enableLongDistanceMatching, ZSTD_c_ldmHashLog,
            ZSTD_c_ldmMinMatch, ZSTD_c_ldmBucketSizeLog, ZSTD_c_ldmHashRateLog,
            ZSTD_c_contentSizeFlag, ZSTD_c_checksumFlag, ZSTD_c_dictIDFlag,
            ZSTD_c_nbWorkers, ZSTD_c_jobSize, ZSTD_c_overlapLog,
            ZSTD_c_rsyncable, ZSTD_c_format, ZSTD_c_forceMaxWindow, ZSTD_c_forceAttachDict,
            ZSTD_c_literalCompressionMode, ZSTD_c_srcSizeHint, ZSTD_c_enableDedicatedDictSearch,
            ZSTD_c_stableInBuffer, ZSTD_c_stableOutBuffer, ZSTD_c_blockDelimiters,
            ZSTD_c_validateSequences, ZSTD_c_blockSplitterLevel, ZSTD_c_splitAfterSequences,
            ZSTD_c_useRowMatchFinder, ZSTD_c_deterministicRefPrefix,
            ZSTD_c_prefetchCDictTables, ZSTD_c_enableSeqProducerFallback,
            ZSTD_c_maxBlockSize, ZSTD_c_repcodeResolution, 12345 };
        for (p = 0; p < (int)(sizeof(cparams)/sizeof(cparams[0])); p++) {
            ZSTD_bounds b = ZSTD_cParam_getBounds((ZSTD_cParameter)cparams[p]);
            printf("cbounds[%d] err=%d lo=%d hi=%d\n", cparams[p], (int)ZSTD_isError(b.error), b.lowerBound, b.upperBound);
        }
        {   static const int dparams[] = {ZSTD_d_windowLogMax, ZSTD_d_format, ZSTD_d_stableOutBuffer,
                                          ZSTD_d_forceIgnoreChecksum, ZSTD_d_refMultipleDDicts,
                                          ZSTD_d_disableHuffmanAssembly, ZSTD_d_maxBlockSize, 999};
            for (p = 0; p < (int)(sizeof(dparams)/sizeof(dparams[0])); p++) {
                ZSTD_bounds b = ZSTD_dParam_getBounds((ZSTD_dParameter)dparams[p]);
                printf("dbounds[%d] err=%d lo=%d hi=%d\n", dparams[p], (int)ZSTD_isError(b.error), b.lowerBound, b.upperBound);
            }
        }
    }
    {   int i2;
        for (i2 = -22; i2 <= 22; i2 += 1) {
            unsigned long long sz;
            for (sz = 0; sz <= 1000000ULL; sz = sz ? sz * 37 : 1) {
                ZSTD_compressionParameters cp = ZSTD_getCParams(i2, sz, 0);
                printf("getCParams[%d,%llu] %u %u %u %u %u %u %d\n", i2, sz,
                       cp.windowLog, cp.chainLog, cp.hashLog, cp.searchLog,
                       cp.minMatch, cp.targetLength, (int)cp.strategy);
            }
        }
    }
    {   size_t s;
        for (s = 0; s <= 1000000; s = s ? s * 13 : 1)
            printf("compressBound[%zu] = %zu\n", s, ZSTD_compressBound(s));
    }
    {   int e;
        for (e = 0; e <= 125; e++)
            printf("errstr[%d] = %s\n", e, ZSTD_getErrorString((ZSTD_ErrorCode)e));
    }
}

static void test_frame_header(void) {
    int ci;
    printf("=== frame headers ===\n");
    rs(31337);
    fill_textish(src, 100000);
    for (ci = 1; ci <= 19; ci += 9) {
        size_t cs = ZSTD_compress(cmp, cmpCap, src, 100000, ci);
        ZSTD_FrameHeader zfh;
        size_t rc = ZSTD_getFrameHeader(&zfh, cmp, cs);
        printf("fh[L%d] rc=%zu fcs=%llu ws=%llu bsm=%u ft=%d hs=%u did=%u ck=%u\n",
               ci, rc, zfh.frameContentSize, zfh.windowSize, zfh.blockSizeMax,
               (int)zfh.frameType, zfh.headerSize, zfh.dictID, zfh.checksumFlag);
        SHOWZ("  frameHeaderSize", ZSTD_frameHeaderSize(cmp, cs));
        SHOWZ("  isFrame", ZSTD_isFrame(cmp, cs));
        SHOWZ("  isSkippableFrame", ZSTD_isSkippableFrame(cmp, cs));
        SHOWU("  findDecompressedSize", ZSTD_findDecompressedSize(cmp, cs));
        SHOWU("  getDictID_fromFrame", ZSTD_getDictID_fromFrame(cmp, cs));
    }
    {   size_t ss = ZSTD_writeSkippableFrame(cmp, cmpCap, src, 1000, 3);
        SHOW("skippableFrame", cmp, ss);
        SHOWZ("  isSkippableFrame", ZSTD_isSkippableFrame(cmp, ss));
        {   unsigned magic = 0;
            size_t rs2 = ZSTD_readSkippableFrame(dec, decCap, &magic, cmp, ss);
            SHOWZ("  readSkippableFrame", rs2);
            SHOWU("  magicVariant", magic);
        }
    }
}

static void test_blocks(void) {
    printf("=== block API ===\n");
    rs(777);
    fill_textish(src, 200000);
    {   ZSTD_CCtx* c = ZSTD_createCCtx();
        ZSTD_DCtx* d = ZSTD_createDCtx();
        size_t bs;
        ZSTD_compressBegin(c, 5);
        bs = ZSTD_getBlockSize(c);
        SHOWZ("blockSize", bs);
        {   size_t off = 0, cout = 0;
            while (off + bs <= 200000 && cout + bs + 64 < cmpCap) {
                size_t cs = ZSTD_compressBlock(c, cmp + cout, cmpCap - cout, src + off, bs);
                if (ZSTD_isError(cs)) { printf("compressBlock ERR %s\n", ZSTD_getErrorName(cs)); break; }
                printf("block[%zu] csize=%zu fnv=%016llx\n", off, cs, fnv(cmp + cout, cs));
                cout += cs;
                off += bs;
                if (off >= 4 * bs) break;
            }
        }
        ZSTD_freeDCtx(d);
        ZSTD_freeCCtx(c);
    }
    {   /* buffer-less streaming */
        ZSTD_CCtx* c = ZSTD_createCCtx();
        size_t total = 0, off = 0;
        ZSTD_compressBegin(c, 7);
        while (off < 200000) {
            size_t take = 33000 < 200000 - off ? 33000 : 200000 - off;
            size_t cs = ZSTD_compressContinue(c, cmp + total, cmpCap - total, src + off, take);
            if (ZSTD_isError(cs)) { printf("compressContinue ERR\n"); break; }
            total += cs; off += take;
        }
        total += ZSTD_compressEnd(c, cmp + total, cmpCap - total, NULL, 0);
        SHOW("bufferless", cmp, total);
        {   size_t ds = ZSTD_decompress(dec, decCap, cmp, total);
            if (ds != 200000 || memcmp(dec, src, 200000)) printf("bufferless ROUNDTRIP-FAIL\n");
        }
        ZSTD_freeCCtx(c);
    }
}

static void test_sequences(void) {
    printf("=== sequences API ===\n");
    rs(0xABCDEF);
    fill_textish(src, 120000);
    {   ZSTD_CCtx* c = ZSTD_createCCtx();
        size_t bound = ZSTD_sequenceBound(120000);
        ZSTD_Sequence* seqs = (ZSTD_Sequence*)malloc(bound * sizeof(ZSTD_Sequence));
        size_t n;
        SHOWZ("sequenceBound", bound);
        ZSTD_CCtx_setParameter(c, ZSTD_c_compressionLevel, 5);
        n = ZSTD_generateSequences(c, seqs, bound, src, 120000);
        if (ZSTD_isError(n)) printf("generateSequences ERR %s\n", ZSTD_getErrorName(n));
        else {
            size_t i;
            unsigned long long h = 1469598103934665603ULL;
            SHOWZ("nbSequences", n);
            for (i = 0; i < n; i++) {
                h ^= seqs[i].offset; h *= 1099511628211ULL;
                h ^= seqs[i].litLength; h *= 1099511628211ULL;
                h ^= seqs[i].matchLength; h *= 1099511628211ULL;
                h ^= seqs[i].rep; h *= 1099511628211ULL;
            }
            printf("%-44s %016llx\n", "sequences hash", h);
            {   size_t m = ZSTD_mergeBlockDelimiters(seqs, n);
                SHOWZ("mergedSequences", m);
                {   ZSTD_CCtx* c2 = ZSTD_createCCtx();
                    size_t cs;
                    ZSTD_CCtx_setParameter(c2, ZSTD_c_compressionLevel, 5);
                    ZSTD_CCtx_setParameter(c2, ZSTD_c_blockDelimiters, ZSTD_sf_noBlockDelimiters);
                    cs = ZSTD_compressSequences(c2, cmp, cmpCap, seqs, m, src, 120000);
                    if (ZSTD_isError(cs)) printf("compressSequences ERR %s\n", ZSTD_getErrorName(cs));
                    else {
                        size_t ds;
                        SHOW("compressSequences", cmp, cs);
                        ds = ZSTD_decompress(dec, decCap, cmp, cs);
                        if (ds != 120000 || memcmp(dec, src, 120000)) printf("compressSequences ROUNDTRIP-FAIL\n");
                    }
                    ZSTD_freeCCtx(c2);
                }
            }
        }
        free(seqs);
        ZSTD_freeCCtx(c);
    }
}

int main(void) {
    cmpCap = ZSTD_compressBound(MAXSRC) + 4096;
    decCap = MAXSRC + 4096;
    src = (unsigned char*)malloc(MAXSRC);
    cmp = (unsigned char*)malloc(cmpCap);
    dec = (unsigned char*)malloc(decCap);
    setvbuf(stdout, NULL, _IOFBF, 1 << 20);
    test_bounds_and_misc();
    test_simple();
    test_cctx_params();
    test_streaming();
    test_frame_header();
    test_blocks();
    test_sequences();
    test_dict();
    printf("=== done ===\n");
    free(src); free(cmp); free(dec);
    return 0;
}
