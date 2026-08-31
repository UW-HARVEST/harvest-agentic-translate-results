/* verify/adv.c -- differential harness for the zstd advanced / experimental API.
 * Build once against the C libzstd.so and once against the Rust libzstd.so,
 * then diff the two traces. Everything printed must be deterministic:
 * no pointers, no addresses, no uninitialised memory. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <limits.h>
#include <stddef.h>

#define ZSTD_STATIC_LINKING_ONLY
#define ZBUFF_DISABLE_DEPRECATE_WARNINGS
#include "zstd.h"
#define ZDICT_STATIC_LINKING_ONLY
#include "zdict.h"
#include "zbuff.h"
#include "cover.h"

/* ------------------------------------------------------------------ */
/* Prototypes for entry points that are exported but not in zstd.h    */
/* ------------------------------------------------------------------ */
extern void   ZSTD_CCtx_trace(ZSTD_CCtx* cctx, size_t extraCSize);
extern unsigned ZSTD_cycleLog(unsigned hashLog, int strat);
extern const void* ZSTD_getSeqStore(const ZSTD_CCtx* ctx);
extern int    ZSTD_seqToCodes(const void* seqStorePtr);
extern size_t ZSTD_convertBlockSequences(ZSTD_CCtx* cctx,
                                         const ZSTD_Sequence* inSeqs,
                                         size_t nbSequences,
                                         int repcodeResolution);
typedef struct { size_t nbSequences; size_t blockSize; size_t litSize; } AdvBlockSummary;
extern AdvBlockSummary ZSTD_get1BlockSummary(const ZSTD_Sequence* seqs, size_t nbSeqs);

extern size_t ZSTD_compressBegin_advanced_internal(ZSTD_CCtx* cctx,
                                                   const void* dict, size_t dictSize,
                                                   int dictContentType, int dtlm,
                                                   const ZSTD_CDict* cdict,
                                                   const ZSTD_CCtx_params* params,
                                                   unsigned long long pledgedSrcSize);
extern size_t ZSTD_compress_advanced_internal(ZSTD_CCtx* cctx,
                                              void* dst, size_t dstCapacity,
                                              const void* src, size_t srcSize,
                                              const void* dict, size_t dictSize,
                                              const ZSTD_CCtx_params* params);
extern size_t ZSTD_compressBegin_usingCDict_deprecated(ZSTD_CCtx* cctx, const ZSTD_CDict* cdict);
extern size_t ZSTD_compressContinue_public(ZSTD_CCtx* c, void* d, size_t dc, const void* s, size_t ss);
extern size_t ZSTD_compressEnd_public(ZSTD_CCtx* c, void* d, size_t dc, const void* s, size_t ss);
extern size_t ZSTD_compressBlock_deprecated(ZSTD_CCtx* c, void* d, size_t dc, const void* s, size_t ss);
extern size_t ZSTD_decompressBlock_deprecated(ZSTD_DCtx* d, void* dst, size_t dc, const void* s, size_t ss);

extern const void* ZSTD_DDict_dictContent(const ZSTD_DDict* ddict);
extern size_t      ZSTD_DDict_dictSize(const ZSTD_DDict* ddict);
extern void        ZSTD_copyDDictParameters(ZSTD_DCtx* dctx, const ZSTD_DDict* ddict);
extern ZSTD_compressionParameters ZSTD_getCParamsFromCDict(const ZSTD_CDict* cdict);
extern size_t      ZSTD_DCtx_setFormat(ZSTD_DCtx* dctx, ZSTD_format_e format);

typedef struct ZSTDMT_CCtx_s ZSTDMT_CCtx;
extern ZSTDMT_CCtx* ZSTDMT_createCCtx_advanced(unsigned nbWorkers, ZSTD_customMem cMem, ZSTD_threadPool* pool);
extern size_t       ZSTDMT_freeCCtx(ZSTDMT_CCtx* mtctx);

/* ------------------------------------------------------------------ */
/* Deterministic PRNG + hashing + printing                            */
/* ------------------------------------------------------------------ */
static unsigned long long g_st = 88172645463325252ULL;
static void     rs(unsigned long long s) { g_st = s ? s : 1; }
static unsigned long long r64(void) {
    g_st ^= g_st << 13; g_st ^= g_st >> 7; g_st ^= g_st << 17; return g_st;
}
static unsigned r32(void) { return (unsigned)(r64() >> 32); }

static unsigned long long fnv(const void* p, size_t n) {
    const unsigned char* b = (const unsigned char*)p;
    unsigned long long h = 1469598103934665603ULL;
    size_t i;
    for (i = 0; i < n; i++) { h ^= b[i]; h *= 1099511628211ULL; }
    return h;
}

static unsigned long long g_calls = 0;   /* number of compared observations */
static ZSTD_customMem g_nullCMem = { NULL, NULL, NULL };

/* ZSTD_ldm_getMaxNbSeq() divides by params.ldmParams.minMatchLength, so calling
 * the size estimators with LDM enabled but ldmMinMatch still 0 divides by zero in
 * BOTH libraries (verified: C raises SIGFPE, Rust panics+aborts at the same call
 * with byte-identical output up to that point). Skip that combination so the rest
 * of the surface can be compared. */
static int ldm_unresolved(const ZSTD_CCtx_params* pp) {
    int ldm = 0, mm = 0;
    ZSTD_CCtxParams_getParameter(pp, ZSTD_c_enableLongDistanceMatching, &ldm);
    ZSTD_CCtxParams_getParameter(pp, ZSTD_c_ldmMinMatch, &mm);
    return (ldm != 0) && (mm == 0);
}


#define P(...)       do { g_calls++; printf(__VA_ARGS__); } while (0)
#define SHOW(nm,b,n) P("%-56s size=%9llu fnv=%016llx\n", (nm), (unsigned long long)(n), fnv((b),(n)))
#define SHOWZ(nm,v)  P("%-56s %lld\n", (nm), (long long)(v))
#define SHOWU(nm,v)  P("%-56s %llu\n", (nm), (unsigned long long)(v))
#define SHOWP(nm,p)  P("%-56s nonnull=%d\n", (nm), (p) != NULL)
#define BANNER(s)    do { printf("\n########## %s ##########\n", (s)); fflush(stdout); } while (0)

/* size_t results are either an error code or a value; render uniformly */
static void show_rc(const char* nm, size_t rc) {
    if (ZSTD_isError(rc)) P("%-56s ERR(%d) %s\n", nm, (int)ZSTD_getErrorCode(rc), ZSTD_getErrorName(rc));
    else                  P("%-56s OK %llu\n", nm, (unsigned long long)rc);
}
/* result + buffer hash when successful */
static void show_buf(const char* nm, size_t rc, const void* b) {
    if (ZSTD_isError(rc)) P("%-56s ERR(%d) %s\n", nm, (int)ZSTD_getErrorCode(rc), ZSTD_getErrorName(rc));
    else P("%-56s OK %9llu fnv=%016llx\n", nm, (unsigned long long)rc, fnv(b, rc));
}

/* ------------------------------------------------------------------ */
/* Corpora                                                            */
/* ------------------------------------------------------------------ */
#define MAXSRC (512u * 1024u)
static unsigned char* src;      /* MAXSRC */
static unsigned char* cmp;      /* cmpCap */
static unsigned char* dec;      /* decCap */
static unsigned char* aux;      /* MAXSRC */
static size_t cmpCap, decCap;

#define PAT 0xA5
static void pat(void* b, size_t n) { memset(b, PAT, n); }

static void fill_rand(unsigned char* d, size_t n)   { size_t i; for (i=0;i<n;i++) d[i]=(unsigned char)r32(); }
static void fill_text(unsigned char* d, size_t n) {
    static const char* w[] = {"alpha ","beta ","gamma ","delta ","epsilon ","zeta ",
                              "compress ","decompress ","dictionary ","sequence ",
                              "frame ","block ","window ","literal ","match ","offset "};
    size_t i = 0;
    while (i < n) {
        const char* s = w[r32() & 15];
        size_t l = strlen(s);
        if (i + l > n) l = n - i;
        memcpy(d + i, s, l);
        i += l;
    }
}
static void fill_lowent(unsigned char* d, size_t n)  { size_t i; for (i=0;i<n;i++) d[i]=(unsigned char)(r32() & 7); }

/* the fixed corpus used by the parameter sweep */
#define CORPUS_N (96u * 1024u)
static void build_corpus(void) { rs(0xC0FFEEULL); fill_text(src, CORPUS_N); }

/* a reusable raw dictionary and a trained (full) dictionary */
#define RAWDICT_N 8192
#define TRAINDICT_N 4096
static unsigned char rawdict[RAWDICT_N];
static unsigned char traindict[TRAINDICT_N];
static size_t traindict_n = 0;

static void build_dicts(void) {
    rs(0xD1C7ULL);
    fill_text(rawdict, RAWDICT_N);
    /* train a real zstd dictionary from slices of the corpus */
    {   size_t sizes[64];
        unsigned char* samples = aux;
        size_t off = 0, i;
        for (i = 0; i < 64; i++) {
            size_t sz = 1024 + (i * 37) % 512;
            rs(0x900D0000ULL + i);
            fill_text(samples + off, sz);
            sizes[i] = sz;
            off += sz;
        }
        traindict_n = ZDICT_trainFromBuffer(traindict, TRAINDICT_N, samples, sizes, 64);
        if (ZDICT_isError(traindict_n)) traindict_n = 0;
    }
}

/* ================================================================== */
/* PHASE 0 : parameter bounds                                          */
/* ================================================================== */
typedef struct { const char* name; ZSTD_cParameter id; int ldmish; } cparam_t;

static const cparam_t kCParams[] = {
    { "compressionLevel",          ZSTD_c_compressionLevel,          0 },
    { "windowLog",                 ZSTD_c_windowLog,                 0 },
    { "hashLog",                   ZSTD_c_hashLog,                   0 },
    { "chainLog",                  ZSTD_c_chainLog,                  0 },
    { "searchLog",                 ZSTD_c_searchLog,                 0 },
    { "minMatch",                  ZSTD_c_minMatch,                  0 },
    { "targetLength",              ZSTD_c_targetLength,              0 },
    { "strategy",                  ZSTD_c_strategy,                  0 },
    { "targetCBlockSize",          ZSTD_c_targetCBlockSize,          0 },
    { "enableLongDistanceMatching",ZSTD_c_enableLongDistanceMatching,0 },
    { "ldmHashLog",                ZSTD_c_ldmHashLog,                1 },
    { "ldmMinMatch",               ZSTD_c_ldmMinMatch,               0 },
    { "ldmBucketSizeLog",          ZSTD_c_ldmBucketSizeLog,          1 },
    { "ldmHashRateLog",            ZSTD_c_ldmHashRateLog,            0 },
    { "contentSizeFlag",           ZSTD_c_contentSizeFlag,           0 },
    { "checksumFlag",              ZSTD_c_checksumFlag,              0 },
    { "dictIDFlag",                ZSTD_c_dictIDFlag,                0 },
    { "nbWorkers",                 ZSTD_c_nbWorkers,                 0 },
    { "jobSize",                   ZSTD_c_jobSize,                   0 },
    { "overlapLog",                ZSTD_c_overlapLog,                0 },
    { "rsyncable",                 ZSTD_c_rsyncable,                 0 },
    { "format",                    ZSTD_c_format,                    0 },
    { "forceMaxWindow",            ZSTD_c_forceMaxWindow,            0 },
    { "forceAttachDict",           ZSTD_c_forceAttachDict,           0 },
    { "literalCompressionMode",    ZSTD_c_literalCompressionMode,    0 },
    { "srcSizeHint",               ZSTD_c_srcSizeHint,               0 },
    { "enableDedicatedDictSearch", ZSTD_c_enableDedicatedDictSearch, 0 },
    { "stableInBuffer",            ZSTD_c_stableInBuffer,            0 },
    { "stableOutBuffer",           ZSTD_c_stableOutBuffer,           0 },
    { "blockDelimiters",           ZSTD_c_blockDelimiters,           0 },
    { "validateSequences",         ZSTD_c_validateSequences,         0 },
    { "blockSplitterLevel",        ZSTD_c_blockSplitterLevel,        0 },
    { "splitAfterSequences",       ZSTD_c_splitAfterSequences,       0 },
    { "useRowMatchFinder",         ZSTD_c_useRowMatchFinder,         0 },
    { "deterministicRefPrefix",    ZSTD_c_deterministicRefPrefix,    0 },
    { "prefetchCDictTables",       ZSTD_c_prefetchCDictTables,       0 },
    { "enableSeqProducerFallback", ZSTD_c_enableSeqProducerFallback, 0 },
    { "maxBlockSize",              ZSTD_c_maxBlockSize,              0 },
    { "repcodeResolution",         ZSTD_c_repcodeResolution,         0 },
    { "searchForExternalRepcodes", ZSTD_c_searchForExternalRepcodes, 0 },
};
#define NCPARAM ((int)(sizeof(kCParams)/sizeof(kCParams[0])))

typedef struct { const char* name; ZSTD_dParameter id; } dparam_t;
static const dparam_t kDParams[] = {
    { "windowLogMax",           ZSTD_d_windowLogMax           },
    { "format",                 ZSTD_d_format                 },
    { "stableOutBuffer",        ZSTD_d_stableOutBuffer        },
    { "forceIgnoreChecksum",    ZSTD_d_forceIgnoreChecksum    },
    { "refMultipleDDicts",      ZSTD_d_refMultipleDDicts      },
    { "disableHuffmanAssembly", ZSTD_d_disableHuffmanAssembly },
    { "maxBlockSize",           ZSTD_d_maxBlockSize           },
};
#define NDPARAM ((int)(sizeof(kDParams)/sizeof(kDParams[0])))

static void phase_bounds(void) {
    int i;
    static const int bogus[] = { -1, 0, 1, 99, 108, 203, 403, 501, 1018, 1099, 100000, INT_MIN, INT_MAX };
    BANNER("PHASE 0: cParam/dParam bounds");
    for (i = 0; i < NCPARAM; i++) {
        ZSTD_bounds b = ZSTD_cParam_getBounds(kCParams[i].id);
        char nm[96];
        snprintf(nm, sizeof nm, "cBounds[%s=%d]", kCParams[i].name, (int)kCParams[i].id);
        P("%-56s err=%d lo=%d hi=%d\n", nm, (int)ZSTD_isError(b.error), b.lowerBound, b.upperBound);
    }
    for (i = 0; i < NDPARAM; i++) {
        ZSTD_bounds b = ZSTD_dParam_getBounds(kDParams[i].id);
        char nm[96];
        snprintf(nm, sizeof nm, "dBounds[%s=%d]", kDParams[i].name, (int)kDParams[i].id);
        P("%-56s err=%d lo=%d hi=%d\n", nm, (int)ZSTD_isError(b.error), b.lowerBound, b.upperBound);
    }
    for (i = 0; i < (int)(sizeof(bogus)/sizeof(bogus[0])); i++) {
        ZSTD_bounds b = ZSTD_cParam_getBounds((ZSTD_cParameter)bogus[i]);
        ZSTD_bounds d = ZSTD_dParam_getBounds((ZSTD_dParameter)bogus[i]);
        char nm[96];
        snprintf(nm, sizeof nm, "cBounds[bogus %d]", bogus[i]);
        P("%-56s err=%d code=%d lo=%d hi=%d\n", nm, (int)ZSTD_isError(b.error),
          (int)ZSTD_getErrorCode(b.error), b.lowerBound, b.upperBound);
        snprintf(nm, sizeof nm, "dBounds[bogus %d]", bogus[i]);
        P("%-56s err=%d code=%d lo=%d hi=%d\n", nm, (int)ZSTD_isError(d.error),
          (int)ZSTD_getErrorCode(d.error), d.lowerBound, d.upperBound);
    }
    SHOWZ("maxCLevel", ZSTD_maxCLevel());
    SHOWZ("minCLevel", ZSTD_minCLevel());
    SHOWZ("defaultCLevel", ZSTD_defaultCLevel());
}

/* ================================================================== */
/* PHASE 1 : every ZSTD_c_* value, legal and out of range              */
/* ================================================================== */

/* Build the list of values to probe for one parameter. */
static int probe_values(const cparam_t* p, int* out, int cap) {
    ZSTD_bounds b = ZSTD_cParam_getBounds(p->id);
    int n = 0, v;
    int lo = b.lowerBound, hi = b.upperBound;
#define PUSH(x) do { int _x=(x); int _j, _dup=0; \
        for (_j=0;_j<n;_j++) if (out[_j]==_x) { _dup=1; break; } \
        if (!_dup && n<cap) out[n++]=_x; } while (0)
    if (ZSTD_isError(b.error)) { PUSH(0); PUSH(1); return n; }
    if ((long long)hi - (long long)lo <= 12) {
        for (v = lo; v <= hi; v++) PUSH(v);
    } else {
        PUSH(lo); PUSH(lo+1); PUSH(lo+2);
        PUSH(lo + (hi-lo)/4); PUSH(lo + (hi-lo)/2); PUSH(hi - (hi-lo)/4);
        PUSH(hi-2); PUSH(hi-1); PUSH(hi);
    }
    /* always exercise the small integers and out-of-range values */
    PUSH(0); PUSH(1); PUSH(2); PUSH(3);
    if (lo != INT_MIN) PUSH(lo-1);
    if (hi != INT_MAX) PUSH(hi+1);
    PUSH(INT_MIN); PUSH(INT_MAX);
#undef PUSH
    return n;
}

/* Some (param,value) pairs would make the library allocate gigabytes. Skip the
 * compression step for those, but still do set/get. */
static int compress_is_safe(const cparam_t* p, int v) {
    if (p->ldmish && v > 24) return 0;
    return 1;
}

static void phase_cparams(void) {
    int i, k;
    BANNER("PHASE 1: ZSTD_c_* sweep (set/get/compress)");
    build_corpus();
    for (i = 0; i < NCPARAM; i++) {
        int vals[40];
        int nv = probe_values(&kCParams[i], vals, 40);
        for (k = 0; k < nv; k++) {
            char nm[160];
            int v = vals[k];
            ZSTD_CCtx* c = ZSTD_createCCtx();
            ZSTD_CCtx_params* pp = ZSTD_createCCtxParams();
            size_t sr, pr;
            int got = -12345, gotp = -12345;
            size_t gr, grp;

            ZSTD_CCtxParams_init(pp, 3);
            sr = ZSTD_CCtx_setParameter(c, kCParams[i].id, v);
            pr = ZSTD_CCtxParams_setParameter(pp, kCParams[i].id, v);
            gr = ZSTD_CCtx_getParameter(c, kCParams[i].id, &got);
            grp = ZSTD_CCtxParams_getParameter(pp, kCParams[i].id, &gotp);

            snprintf(nm, sizeof nm, "set[%s=%d]", kCParams[i].name, v);
            P("%-56s cctx=%s params=%s get=%s/%d getp=%s/%d\n", nm,
              ZSTD_isError(sr) ? ZSTD_getErrorName(sr) : "ok",
              ZSTD_isError(pr) ? ZSTD_getErrorName(pr) : "ok",
              ZSTD_isError(gr) ? ZSTD_getErrorName(gr) : "ok", got,
              ZSTD_isError(grp) ? ZSTD_getErrorName(grp) : "ok", gotp);

            if (!ZSTD_isError(sr) && compress_is_safe(&kCParams[i], v)) {
                size_t cs;
                pat(cmp, 4096);
                cs = ZSTD_compress2(c, cmp, cmpCap, src, CORPUS_N);
                snprintf(nm, sizeof nm, "  c2[%s=%d]", kCParams[i].name, v);
                show_buf(nm, cs, cmp);
                if (!ZSTD_isError(cs)) {
                    size_t ds;
                    int isMagicless = (kCParams[i].id == ZSTD_c_format) && (v == ZSTD_f_zstd1_magicless);
                    pat(dec, 4096);
                    if (isMagicless) {
                        ZSTD_DCtx* d = ZSTD_createDCtx();
                        ZSTD_DCtx_setParameter(d, ZSTD_d_format, ZSTD_f_zstd1_magicless);
                        ds = ZSTD_decompressDCtx(d, dec, decCap, cmp, cs);
                        ZSTD_freeDCtx(d);
                    } else {
                        ds = ZSTD_decompress(dec, decCap, cmp, cs);
                    }
                    snprintf(nm, sizeof nm, "  rt[%s=%d]", kCParams[i].name, v);
                    if (ZSTD_isError(ds)) show_rc(nm, ds);
                    else P("%-56s %s\n", nm,
                           (ds == CORPUS_N && !memcmp(dec, src, CORPUS_N)) ? "match" : "MISMATCH");
                }
            }
            /* estimates driven by the same params object */
            if (!ldm_unresolved(pp)) {   char e[160];
                snprintf(e, sizeof e, "  estCCtx[%s=%d]", kCParams[i].name, v);
                show_rc(e, ZSTD_estimateCCtxSize_usingCCtxParams(pp));
                snprintf(e, sizeof e, "  estCStream[%s=%d]", kCParams[i].name, v);
                show_rc(e, ZSTD_estimateCStreamSize_usingCCtxParams(pp));
            }
            ZSTD_freeCCtxParams(pp);
            ZSTD_freeCCtx(c);
        }
    }
    /* setting an unknown parameter id */
    {   ZSTD_CCtx* c = ZSTD_createCCtx();
        int got = 0;
        show_rc("set[bogus 12345]", ZSTD_CCtx_setParameter(c, (ZSTD_cParameter)12345, 1));
        show_rc("get[bogus 12345]", ZSTD_CCtx_getParameter(c, (ZSTD_cParameter)12345, &got));
        SHOWZ("get[bogus 12345].value", got);
        ZSTD_freeCCtx(c);
    }
    /* dParam set/get sweep */
    for (i = 0; i < NDPARAM; i++) {
        ZSTD_bounds b = ZSTD_dParam_getBounds(kDParams[i].id);
        int cand[8]; int n = 0, j;
        cand[n++] = b.lowerBound; cand[n++] = b.upperBound;
        cand[n++] = 0; cand[n++] = 1; cand[n++] = 2;
        cand[n++] = b.lowerBound - 1; cand[n++] = b.upperBound + 1;
        cand[n++] = INT_MAX;
        for (j = 0; j < n; j++) {
            ZSTD_DCtx* d = ZSTD_createDCtx();
            int got = -999;
            size_t sr = ZSTD_DCtx_setParameter(d, kDParams[i].id, cand[j]);
            size_t gr = ZSTD_DCtx_getParameter(d, kDParams[i].id, &got);
            char nm[160];
            snprintf(nm, sizeof nm, "dset[%s=%d]", kDParams[i].name, cand[j]);
            P("%-56s set=%s get=%s/%d\n", nm,
              ZSTD_isError(sr) ? ZSTD_getErrorName(sr) : "ok",
              ZSTD_isError(gr) ? ZSTD_getErrorName(gr) : "ok", got);
            ZSTD_freeDCtx(d);
        }
    }
    {   ZSTD_DCtx* d = ZSTD_createDCtx();
        int got = 0;
        show_rc("dset[bogus 777]", ZSTD_DCtx_setParameter(d, (ZSTD_dParameter)777, 1));
        show_rc("dget[bogus 777]", ZSTD_DCtx_getParameter(d, (ZSTD_dParameter)777, &got));
        show_rc("DCtx_setMaxWindowSize(0)", ZSTD_DCtx_setMaxWindowSize(d, 0));
        show_rc("DCtx_setMaxWindowSize(1<<10)", ZSTD_DCtx_setMaxWindowSize(d, 1u<<10));
        show_rc("DCtx_setMaxWindowSize(1<<31)", ZSTD_DCtx_setMaxWindowSize(d, 1ull<<31));
        show_rc("DCtx_setFormat(zstd1)", ZSTD_DCtx_setFormat(d, ZSTD_f_zstd1));
        show_rc("DCtx_setFormat(magicless)", ZSTD_DCtx_setFormat(d, ZSTD_f_zstd1_magicless));
        show_rc("DCtx_setFormat(99)", ZSTD_DCtx_setFormat(d, (ZSTD_format_e)99));
        ZSTD_freeDCtx(d);
    }
}

/* ================================================================== */
/* PHASE 2 : the ZSTD_CCtx_params object                               */
/* ================================================================== */
static void dump_params_obj(const char* tag, const ZSTD_CCtx_params* pp) {
    int i;
    for (i = 0; i < NCPARAM; i++) {
        int v = -424242;
        size_t rc = ZSTD_CCtxParams_getParameter(pp, kCParams[i].id, &v);
        char nm[160];
        snprintf(nm, sizeof nm, "%s.%s", tag, kCParams[i].name);
        if (ZSTD_isError(rc)) P("%-56s ERR %s\n", nm, ZSTD_getErrorName(rc));
        else                  P("%-56s %d\n", nm, v);
    }
}

static void phase_cctxparams(void) {
    static const int levels[] = { ZSTD_CLEVEL_DEFAULT, -22, -1, 0, 1, 5, 12, 19, 22, 23, -100 };
    int li;
    BANNER("PHASE 2: ZSTD_CCtx_params object API");

    {   ZSTD_CCtx_params* probe = ZSTD_createCCtxParams();
        SHOWP("createCCtxParams", (void*)probe);
        show_rc("freeCCtxParams(fresh)", ZSTD_freeCCtxParams(probe));
    }
    show_rc("freeCCtxParams(NULL)", ZSTD_freeCCtxParams(NULL));
    show_rc("CCtxParams_reset(NULL)", ZSTD_CCtxParams_reset(NULL));

    for (li = 0; li < (int)(sizeof(levels)/sizeof(levels[0])); li++) {
        ZSTD_CCtx_params* pp = ZSTD_createCCtxParams();
        char tag[64];
        int lvl = levels[li];
        show_rc("CCtxParams_init", ZSTD_CCtxParams_init(pp, lvl));
        snprintf(tag, sizeof tag, "init(L%d)", lvl);
        dump_params_obj(tag, pp);
        if (!ldm_unresolved(pp)) {
        show_rc("  estCCtxSize_usingCCtxParams", ZSTD_estimateCCtxSize_usingCCtxParams(pp));
        show_rc("  estCStreamSize_usingCCtxParams", ZSTD_estimateCStreamSize_usingCCtxParams(pp)); }

        /* init_advanced with the matching ZSTD_parameters */
        {   ZSTD_parameters prm = ZSTD_getParams(lvl, CORPUS_N, 0);
            show_rc("CCtxParams_init_advanced", ZSTD_CCtxParams_init_advanced(pp, prm));
            snprintf(tag, sizeof tag, "initAdv(L%d)", lvl);
            dump_params_obj(tag, pp);
            if (!ldm_unresolved(pp)) {
            show_rc("  estCCtxSize_usingCCtxParams", ZSTD_estimateCCtxSize_usingCCtxParams(pp));
            show_rc("  estCStreamSize_usingCCtxParams", ZSTD_estimateCStreamSize_usingCCtxParams(pp)); }
        }
        /* reset then re-read */
        show_rc("CCtxParams_reset", ZSTD_CCtxParams_reset(pp));
        snprintf(tag, sizeof tag, "reset(L%d)", lvl);
        dump_params_obj(tag, pp);

        /* register a NULL sequence producer (must be a no-op) */
        ZSTD_CCtxParams_registerSequenceProducer(pp, NULL, NULL);
        P("%-56s done\n", "CCtxParams_registerSequenceProducer(NULL,NULL)");
        if (!ldm_unresolved(pp)) show_rc("  estCCtxSize after registerSeqProd", ZSTD_estimateCCtxSize_usingCCtxParams(pp));

        /* drive a real compression through setParametersUsingCCtxParams */
        {   ZSTD_CCtx* c = ZSTD_createCCtx();
            size_t cs;
            ZSTD_CCtxParams_init(pp, lvl);
            ZSTD_CCtxParams_setParameter(pp, ZSTD_c_checksumFlag, 1);
            ZSTD_CCtxParams_setParameter(pp, ZSTD_c_contentSizeFlag, (li & 1));
            ZSTD_CCtxParams_setParameter(pp, ZSTD_c_dictIDFlag, (li & 1) ^ 1);
            ZSTD_CCtxParams_setParameter(pp, ZSTD_c_windowLog, 16 + (li % 4));
            show_rc("setParametersUsingCCtxParams", ZSTD_CCtx_setParametersUsingCCtxParams(c, pp));
            pat(cmp, 4096);
            cs = ZSTD_compress2(c, cmp, cmpCap, src, CORPUS_N);
            snprintf(tag, sizeof tag, "usingCCtxParams c2(L%d)", lvl);
            show_buf(tag, cs, cmp);
            if (!ZSTD_isError(cs)) {
                size_t ds;
                pat(dec, 4096);
                ds = ZSTD_decompress(dec, decCap, cmp, cs);
                P("%-56s %s\n", "  roundtrip",
                  (!ZSTD_isError(ds) && ds == CORPUS_N && !memcmp(dec, src, CORPUS_N)) ? "match" : "MISMATCH");
            }
            /* setParametersUsingCCtxParams mid-session must fail */
            {   ZSTD_outBuffer ob; ZSTD_inBuffer ib;
                ZSTD_CCtx_reset(c, ZSTD_reset_session_and_parameters);
                ob.dst = cmp; ob.size = 64; ob.pos = 0;
                ib.src = src; ib.size = 4096; ib.pos = 0;
                show_rc("  compressStream2(start session)", ZSTD_compressStream2(c, &ob, &ib, ZSTD_e_continue));
                show_rc("  setParametersUsingCCtxParams mid", ZSTD_CCtx_setParametersUsingCCtxParams(c, pp));
            }
            show_rc("setParametersUsingCCtxParams(NULLparams)", ZSTD_CCtx_setParametersUsingCCtxParams(c, NULL));
            ZSTD_freeCCtx(c);
        }
        show_rc("freeCCtxParams", ZSTD_freeCCtxParams(pp));
    }

    /* NOTE: ZSTD_estimate{CCtx,CStream}Size_usingCCtxParams(NULL) dereferences the
     * NULL pointer in the C library (segfault); skipped on purpose. */

    /* CCtxParams driving createCDict_advanced2 */
    {   ZSTD_CCtx_params* pp = ZSTD_createCCtxParams();
        int dm, dc;
        ZSTD_CCtxParams_init(pp, 7);
        for (dm = 0; dm <= 1; dm++) for (dc = 0; dc <= 2; dc++) {
            ZSTD_CDict* cd = ZSTD_createCDict_advanced2(traindict, traindict_n,
                                (ZSTD_dictLoadMethod_e)dm, (ZSTD_dictContentType_e)dc,
                                pp, ZSTD_defaultCMem);
            char nm[96];
            snprintf(nm, sizeof nm, "createCDict_advanced2[dlm%d,dct%d]", dm, dc);
            SHOWP(nm, (void*)cd);
            if (cd) {
                snprintf(nm, sizeof nm, "  sizeof_CDict[dlm%d,dct%d]", dm, dc);
                SHOWU(nm, ZSTD_sizeof_CDict(cd));
                snprintf(nm, sizeof nm, "  dictID[dlm%d,dct%d]", dm, dc);
                SHOWU(nm, ZSTD_getDictID_fromCDict(cd));
                {   ZSTD_compressionParameters cp = ZSTD_getCParamsFromCDict(cd);
                    snprintf(nm, sizeof nm, "  cParamsFromCDict[dlm%d,dct%d]", dm, dc);
                    P("%-56s w=%u h=%u ch=%u s=%u m=%u t=%u st=%d\n", nm,
                      cp.windowLog, cp.hashLog, cp.chainLog, cp.searchLog,
                      cp.minMatch, cp.targetLength, (int)cp.strategy);
                }
                ZSTD_freeCDict(cd);
            }
        }
        ZSTD_freeCCtxParams(pp);
    }
}

/* ================================================================== */
/* PHASE 3 : magicless format round trips                              */
/* ================================================================== */
static void phase_magicless(void) {
    static const size_t sizes[] = { 0, 1, 100, 5000, 70000, CORPUS_N };
    int si, way, withCS;
    BANNER("PHASE 3: ZSTD_f_zstd1_magicless round trips");
    for (si = 0; si < (int)(sizeof(sizes)/sizeof(sizes[0])); si++) {
        size_t n = sizes[si];
        rs(0x3A61CULL + si);
        fill_text(src, n);
        for (withCS = 0; withCS <= 1; withCS++) {
            ZSTD_CCtx* c = ZSTD_createCCtx();
            size_t cs;
            char nm[128];
            ZSTD_CCtx_setParameter(c, ZSTD_c_format, ZSTD_f_zstd1_magicless);
            ZSTD_CCtx_setParameter(c, ZSTD_c_checksumFlag, withCS);
            ZSTD_CCtx_setParameter(c, ZSTD_c_compressionLevel, 6);
            pat(cmp, 4096);
            cs = ZSTD_compress2(c, cmp, cmpCap, src, n);
            snprintf(nm, sizeof nm, "magicless c2[n=%llu,cs=%d]", (unsigned long long)n, withCS);
            show_buf(nm, cs, cmp);
            ZSTD_freeCCtx(c);
            if (ZSTD_isError(cs)) continue;

            /* magicless output must not look like a zstd frame */
            snprintf(nm, sizeof nm, "  isFrame[n=%llu,cs=%d]", (unsigned long long)n, withCS);
            SHOWU(nm, ZSTD_isFrame(cmp, cs));

            /* decompress it 3 ways: setParameter, setFormat, and (wrongly) default */
            for (way = 0; way < 3; way++) {
                ZSTD_DCtx* d = ZSTD_createDCtx();
                size_t ds;
                if (way == 0) show_rc("  d_format via setParameter",
                                      ZSTD_DCtx_setParameter(d, ZSTD_d_format, ZSTD_f_zstd1_magicless));
                else if (way == 1) show_rc("  d_format via setFormat",
                                      ZSTD_DCtx_setFormat(d, ZSTD_f_zstd1_magicless));
                /* way==2: leave default ZSTD_f_zstd1 -> must fail */
                pat(dec, 4096);
                ds = ZSTD_decompressDCtx(d, dec, decCap, cmp, cs);
                snprintf(nm, sizeof nm, "  magicless dec way%d[n=%llu,cs=%d]", way, (unsigned long long)n, withCS);
                if (ZSTD_isError(ds)) show_rc(nm, ds);
                else P("%-56s OK %llu %s\n", nm, (unsigned long long)ds,
                       (ds == n && (n == 0 || !memcmp(dec, src, n))) ? "match" : "MISMATCH");

                /* header queries under the magicless format */
                if (way != 2) {
                    ZSTD_FrameHeader fh;
                    size_t hr;
                    memset(&fh, 0, sizeof fh);
                    hr = ZSTD_getFrameHeader_advanced(&fh, cmp, cs, ZSTD_f_zstd1_magicless);
                    snprintf(nm, sizeof nm, "  getFrameHeader_advanced[n=%llu]", (unsigned long long)n);
                    if (ZSTD_isError(hr)) show_rc(nm, hr);
                    else P("%-56s r=%llu fcs=%llu ws=%llu bs=%u did=%u cks=%u\n", nm,
                           (unsigned long long)hr,
                           (unsigned long long)fh.frameContentSize,
                           (unsigned long long)fh.windowSize,
                           fh.blockSizeMax, fh.dictID, fh.checksumFlag);
                    snprintf(nm, sizeof nm, "  frameHeaderSize[n=%llu]", (unsigned long long)n);
                    show_rc(nm, ZSTD_frameHeaderSize(cmp, cs));
                }
                /* streaming decompression of a magicless frame */
                if (way == 0) {
                    ZSTD_DCtx* sd = ZSTD_createDCtx();
                    ZSTD_inBuffer ib; ZSTD_outBuffer ob;
                    size_t r = 0; int guard = 0;
                    ZSTD_DCtx_setParameter(sd, ZSTD_d_format, ZSTD_f_zstd1_magicless);
                    ib.src = cmp; ib.size = cs; ib.pos = 0;
                    pat(dec, 4096);
                    ob.dst = dec; ob.size = decCap; ob.pos = 0;
                    do {
                        r = ZSTD_decompressStream(sd, &ob, &ib);
                        if (ZSTD_isError(r)) break;
                    } while ((ib.pos < ib.size || r != 0) && ++guard < 1000);
                    snprintf(nm, sizeof nm, "  magicless stream[n=%llu,cs=%d]", (unsigned long long)n, withCS);
                    if (ZSTD_isError(r)) show_rc(nm, r);
                    else P("%-56s out=%llu %s\n", nm, (unsigned long long)ob.pos,
                           (ob.pos == n && (n == 0 || !memcmp(dec, src, n))) ? "match" : "MISMATCH");
                    ZSTD_freeDCtx(sd);
                }
                ZSTD_freeDCtx(d);
            }
        }
    }
    /* magicless with a dictionary, both sides */
    if (traindict_n) {
        ZSTD_CCtx* c = ZSTD_createCCtx();
        size_t cs;
        rs(0xDD11ULL); fill_text(src, 40000);
        ZSTD_CCtx_setParameter(c, ZSTD_c_format, ZSTD_f_zstd1_magicless);
        ZSTD_CCtx_loadDictionary(c, traindict, traindict_n);
        pat(cmp, 4096);
        cs = ZSTD_compress2(c, cmp, cmpCap, src, 40000);
        show_buf("magicless+dict c2", cs, cmp);
        ZSTD_freeCCtx(c);
        if (!ZSTD_isError(cs)) {
            ZSTD_DCtx* d = ZSTD_createDCtx();
            size_t ds;
            ZSTD_DCtx_setParameter(d, ZSTD_d_format, ZSTD_f_zstd1_magicless);
            ZSTD_DCtx_loadDictionary(d, traindict, traindict_n);
            pat(dec, 4096);
            ds = ZSTD_decompressDCtx(d, dec, decCap, cmp, cs);
            if (ZSTD_isError(ds)) show_rc("magicless+dict dec", ds);
            else P("%-56s %s\n", "magicless+dict dec",
                   (ds == 40000 && !memcmp(dec, src, 40000)) ? "match" : "MISMATCH");
            ZSTD_freeDCtx(d);
        }
    }
}

/* ================================================================== */
/* PHASE 4 : dictionary variants                                       */
/* ================================================================== */
#define DICTSRC_N 40000

/* Compress DICTSRC_N bytes of src with cctx and try to decompress with the
 * supplied dctx-configuring callback; report both sides. */
static void dict_roundtrip(const char* nm, ZSTD_CCtx* c,
                           const void* ddict_dict, size_t ddict_size,
                           const ZSTD_DDict* ddict, int useRefPrefix) {
    size_t cs, ds;
    pat(cmp, 4096);
    cs = ZSTD_compress2(c, cmp, cmpCap, src, DICTSRC_N);
    show_buf(nm, cs, cmp);
    if (ZSTD_isError(cs)) return;
    {   char q[160];
        snprintf(q, sizeof q, "  dictID_fromFrame(%s)", nm);
        SHOWU(q, ZSTD_getDictID_fromFrame(cmp, cs));
    }
    {   ZSTD_DCtx* d = ZSTD_createDCtx();
        char q[192];
        if (ddict) ZSTD_DCtx_refDDict(d, ddict);
        else if (useRefPrefix) ZSTD_DCtx_refPrefix(d, ddict_dict, ddict_size);
        else if (ddict_dict) ZSTD_DCtx_loadDictionary(d, ddict_dict, ddict_size);
        pat(dec, 4096);
        ds = ZSTD_decompressDCtx(d, dec, decCap, cmp, cs);
        snprintf(q, sizeof q, "  dec(%s)", nm);
        if (ZSTD_isError(ds)) show_rc(q, ds);
        else P("%-56s OK %llu %s\n", q, (unsigned long long)ds,
               (ds == DICTSRC_N && !memcmp(dec, src, DICTSRC_N)) ? "match" : "MISMATCH");
        ZSTD_freeDCtx(d);
    }
}

static void phase_dict(void) {
    int dm, dc;
    BANNER("PHASE 4: dictionary APIs");
    rs(0x4D1C7ULL);
    fill_text(src, DICTSRC_N);

    SHOWU("getDictID_fromDict(raw)", ZSTD_getDictID_fromDict(rawdict, RAWDICT_N));
    SHOWU("getDictID_fromDict(trained)", ZSTD_getDictID_fromDict(traindict, traindict_n));
    SHOWU("getDictID_fromDict(NULL,0)", ZSTD_getDictID_fromDict(NULL, 0));
    SHOWU("getDictID_fromDict(short)", ZSTD_getDictID_fromDict(traindict, 3));
    SHOWU("getDictID_fromCDict(NULL)", ZSTD_getDictID_fromCDict(NULL));
    SHOWU("getDictID_fromDDict(NULL)", ZSTD_getDictID_fromDDict(NULL));
    SHOWU("getDictID_fromFrame(junk)", ZSTD_getDictID_fromFrame(rawdict, 64));
    SHOWU("ZDICT_getDictID(trained)", ZDICT_getDictID(traindict, traindict_n));
    show_rc("ZDICT_getDictHeaderSize(trained)", ZDICT_getDictHeaderSize(traindict, traindict_n));
    show_rc("ZDICT_getDictHeaderSize(raw)", ZDICT_getDictHeaderSize(rawdict, RAWDICT_N));
    show_rc("ZDICT_getDictHeaderSize(NULL,0)", ZDICT_getDictHeaderSize(NULL, 0));

    /* --- CCtx_loadDictionary family over every combination --- */
    for (dm = 0; dm <= 2; dm++) {          /* 2 == invalid load method */
        for (dc = 0; dc <= 3; dc++) {      /* 3 == invalid content type */
            int which;
            for (which = 0; which < 2; which++) {   /* raw vs trained dict */
                const void* D = which ? (const void*)traindict : (const void*)rawdict;
                size_t DN   = which ? traindict_n : (size_t)RAWDICT_N;
                ZSTD_CCtx* c = ZSTD_createCCtx();
                char nm[176];
                ZSTD_CCtx_setParameter(c, ZSTD_c_compressionLevel, 5);
                snprintf(nm, sizeof nm, "loadDict_advanced[%s,dlm%d,dct%d]",
                         which ? "trained" : "raw", dm, dc);
                show_rc(nm, ZSTD_CCtx_loadDictionary_advanced(c, D, DN,
                            (ZSTD_dictLoadMethod_e)dm, (ZSTD_dictContentType_e)dc));
                dict_roundtrip(nm, c, D, DN, NULL, 0);
                ZSTD_freeCCtx(c);
            }
        }
    }
    {   ZSTD_CCtx* c = ZSTD_createCCtx();
        ZSTD_CCtx_setParameter(c, ZSTD_c_compressionLevel, 5);
        show_rc("CCtx_loadDictionary(trained)", ZSTD_CCtx_loadDictionary(c, traindict, traindict_n));
        dict_roundtrip("CCtx_loadDictionary(trained)", c, traindict, traindict_n, NULL, 0);
        show_rc("CCtx_loadDictionary(NULL,0)", ZSTD_CCtx_loadDictionary(c, NULL, 0));
        show_rc("CCtx_loadDictionary(NULL,10)", ZSTD_CCtx_loadDictionary(c, NULL, 10));
        ZSTD_freeCCtx(c);
    }
    {   ZSTD_CCtx* c = ZSTD_createCCtx();
        ZSTD_CCtx_setParameter(c, ZSTD_c_compressionLevel, 5);
        show_rc("CCtx_loadDictionary_byReference", ZSTD_CCtx_loadDictionary_byReference(c, traindict, traindict_n));
        dict_roundtrip("CCtx_loadDictionary_byReference", c, traindict, traindict_n, NULL, 0);
        ZSTD_freeCCtx(c);
    }
    /* --- refPrefix --- */
    {   ZSTD_CCtx* c = ZSTD_createCCtx();
        ZSTD_CCtx_setParameter(c, ZSTD_c_compressionLevel, 5);
        show_rc("CCtx_refPrefix", ZSTD_CCtx_refPrefix(c, rawdict, RAWDICT_N));
        dict_roundtrip("CCtx_refPrefix", c, rawdict, RAWDICT_N, NULL, 1);
        ZSTD_freeCCtx(c);
    }
    for (dc = 0; dc <= 3; dc++) {
        ZSTD_CCtx* c = ZSTD_createCCtx();
        char nm[128];
        ZSTD_CCtx_setParameter(c, ZSTD_c_compressionLevel, 5);
        snprintf(nm, sizeof nm, "CCtx_refPrefix_advanced[dct%d]", dc);
        show_rc(nm, ZSTD_CCtx_refPrefix_advanced(c, traindict, traindict_n, (ZSTD_dictContentType_e)dc));
        dict_roundtrip(nm, c, traindict, traindict_n, NULL, 1);
        ZSTD_freeCCtx(c);
    }
    /* deterministicRefPrefix interacts with refPrefix */
    {   int drp;
        for (drp = 0; drp <= 1; drp++) {
            ZSTD_CCtx* c = ZSTD_createCCtx();
            char nm[128];
            ZSTD_CCtx_setParameter(c, ZSTD_c_compressionLevel, 9);
            ZSTD_CCtx_setParameter(c, ZSTD_c_deterministicRefPrefix, drp);
            ZSTD_CCtx_refPrefix(c, rawdict, RAWDICT_N);
            snprintf(nm, sizeof nm, "refPrefix+deterministic%d", drp);
            dict_roundtrip(nm, c, rawdict, RAWDICT_N, NULL, 1);
            ZSTD_freeCCtx(c);
        }
    }
    /* --- DCtx side loaders, exercised against a plain dict frame --- */
    {   ZSTD_CCtx* c = ZSTD_createCCtx();
        size_t cs;
        ZSTD_CCtx_setParameter(c, ZSTD_c_compressionLevel, 5);
        ZSTD_CCtx_loadDictionary(c, traindict, traindict_n);
        pat(cmp, 4096);
        cs = ZSTD_compress2(c, cmp, cmpCap, src, DICTSRC_N);
        show_buf("frame for DCtx loaders", cs, cmp);
        ZSTD_freeCCtx(c);
        if (!ZSTD_isError(cs)) {
            for (dm = 0; dm <= 2; dm++) for (dc = 0; dc <= 3; dc++) {
                ZSTD_DCtx* d = ZSTD_createDCtx();
                char nm[160];
                size_t ds;
                snprintf(nm, sizeof nm, "DCtx_loadDictionary_advanced[dlm%d,dct%d]", dm, dc);
                show_rc(nm, ZSTD_DCtx_loadDictionary_advanced(d, traindict, traindict_n,
                            (ZSTD_dictLoadMethod_e)dm, (ZSTD_dictContentType_e)dc));
                pat(dec, 4096);
                ds = ZSTD_decompressDCtx(d, dec, decCap, cmp, cs);
                snprintf(nm, sizeof nm, "  dec[dlm%d,dct%d]", dm, dc);
                if (ZSTD_isError(ds)) show_rc(nm, ds);
                else P("%-56s OK %llu %s\n", nm, (unsigned long long)ds,
                       (ds == DICTSRC_N && !memcmp(dec, src, DICTSRC_N)) ? "match" : "MISMATCH");
                ZSTD_freeDCtx(d);
            }
            {   ZSTD_DCtx* d = ZSTD_createDCtx();
                size_t ds;
                show_rc("DCtx_loadDictionary_byReference", ZSTD_DCtx_loadDictionary_byReference(d, traindict, traindict_n));
                pat(dec, 4096);
                ds = ZSTD_decompressDCtx(d, dec, decCap, cmp, cs);
                show_rc("  dec byReference", ds);
                show_rc("DCtx_loadDictionary(NULL,0)", ZSTD_DCtx_loadDictionary(d, NULL, 0));
                show_rc("DCtx_loadDictionary(NULL,7)", ZSTD_DCtx_loadDictionary(d, NULL, 7));
                ZSTD_freeDCtx(d);
            }
            for (dc = 0; dc <= 3; dc++) {
                ZSTD_DCtx* d = ZSTD_createDCtx();
                char nm[128];
                snprintf(nm, sizeof nm, "DCtx_refPrefix_advanced[dct%d]", dc);
                show_rc(nm, ZSTD_DCtx_refPrefix_advanced(d, traindict, traindict_n, (ZSTD_dictContentType_e)dc));
                ZSTD_freeDCtx(d);
            }
        }
    }
    /* --- CDict creation variants + refCDict --- */
    for (dm = 0; dm <= 1; dm++) for (dc = 0; dc <= 2; dc++) {
        ZSTD_compressionParameters cp = ZSTD_getCParams(8, DICTSRC_N, traindict_n);
        ZSTD_CDict* cd = ZSTD_createCDict_advanced(traindict, traindict_n,
                             (ZSTD_dictLoadMethod_e)dm, (ZSTD_dictContentType_e)dc,
                             cp, ZSTD_defaultCMem);
        char nm[160];
        snprintf(nm, sizeof nm, "createCDict_advanced[dlm%d,dct%d]", dm, dc);
        SHOWP(nm, (void*)cd);
        if (!cd) continue;
        snprintf(nm, sizeof nm, "  sizeof_CDict[dlm%d,dct%d]", dm, dc);
        SHOWU(nm, ZSTD_sizeof_CDict(cd));
        snprintf(nm, sizeof nm, "  estimateCDictSize_advanced[dlm%d]", dm);
        SHOWU(nm, ZSTD_estimateCDictSize_advanced(traindict_n, cp, (ZSTD_dictLoadMethod_e)dm));
        {   ZSTD_CCtx* c = ZSTD_createCCtx();
            int fad;
            for (fad = 0; fad <= 3; fad++) {
                char q[176];
                ZSTD_CCtx_reset(c, ZSTD_reset_session_and_parameters);
                ZSTD_CCtx_setParameter(c, ZSTD_c_forceAttachDict, fad);
                ZSTD_CCtx_setParameter(c, ZSTD_c_prefetchCDictTables, fad % 3);
                snprintf(q, sizeof q, "refCDict[dlm%d,dct%d,attach%d]", dm, dc, fad);
                show_rc(q, ZSTD_CCtx_refCDict(c, cd));
                dict_roundtrip(q, c, traindict, traindict_n, NULL, 0);
            }
            show_rc("  refCDict(NULL)", ZSTD_CCtx_refCDict(c, NULL));
            ZSTD_freeCCtx(c);
        }
        ZSTD_freeCDict(cd);
    }
    {   ZSTD_CDict* cd = ZSTD_createCDict_byReference(traindict, traindict_n, 7);
        SHOWP("createCDict_byReference", (void*)cd);
        if (cd) {
            SHOWU("  sizeof_CDict byRef", ZSTD_sizeof_CDict(cd));
            SHOWU("  dictID byRef", ZSTD_getDictID_fromCDict(cd));
            show_rc("  compress_usingCDict", ZSTD_compress_usingCDict(ZSTD_createCCtx(), cmp, cmpCap, src, 1000, cd));
            ZSTD_freeCDict(cd);
        }
        SHOWP("createCDict_byReference(NULL,0)", (void*)ZSTD_createCDict_byReference(NULL, 0, 3));
    }
    /* --- DDict variants --- */
    for (dm = 0; dm <= 1; dm++) for (dc = 0; dc <= 2; dc++) {
        ZSTD_DDict* dd = ZSTD_createDDict_advanced(traindict, traindict_n,
                             (ZSTD_dictLoadMethod_e)dm, (ZSTD_dictContentType_e)dc,
                             ZSTD_defaultCMem);
        char nm[160];
        snprintf(nm, sizeof nm, "createDDict_advanced[dlm%d,dct%d]", dm, dc);
        SHOWP(nm, (void*)dd);
        if (!dd) continue;
        snprintf(nm, sizeof nm, "  sizeof_DDict[dlm%d,dct%d]", dm, dc);
        SHOWU(nm, ZSTD_sizeof_DDict(dd));
        snprintf(nm, sizeof nm, "  DDict_dictSize[dlm%d,dct%d]", dm, dc);
        SHOWU(nm, ZSTD_DDict_dictSize(dd));
        snprintf(nm, sizeof nm, "  DDict_dictContent fnv[dlm%d,dct%d]", dm, dc);
        {   const void* dcp = ZSTD_DDict_dictContent(dd);
            size_t dsz = ZSTD_DDict_dictSize(dd);
            if (dcp && dsz) SHOWU(nm, fnv(dcp, dsz));
            else            P("%-56s none\n", nm);
        }
        snprintf(nm, sizeof nm, "  DDict dictID[dlm%d,dct%d]", dm, dc);
        SHOWU(nm, ZSTD_getDictID_fromDDict(dd));
        snprintf(nm, sizeof nm, "  estimateDDictSize[dlm%d]", dm);
        SHOWU(nm, ZSTD_estimateDDictSize(traindict_n, (ZSTD_dictLoadMethod_e)dm));
        {   ZSTD_DCtx* d = ZSTD_createDCtx();
            ZSTD_copyDDictParameters(d, dd);
            P("%-56s done\n", "  copyDDictParameters");
            show_rc("  DCtx_refDDict", ZSTD_DCtx_refDDict(d, dd));
            show_rc("  DCtx_refDDict(NULL)", ZSTD_DCtx_refDDict(d, NULL));
            ZSTD_freeDCtx(d);
        }
        ZSTD_freeDDict(dd);
    }
    {   ZSTD_DDict* dd = ZSTD_createDDict_byReference(traindict, traindict_n);
        SHOWP("createDDict_byReference", (void*)dd);
        if (dd) {
            SHOWU("  sizeof_DDict byRef", ZSTD_sizeof_DDict(dd));
            SHOWU("  DDict_dictSize byRef", ZSTD_DDict_dictSize(dd));
            ZSTD_freeDDict(dd);
        }
        SHOWP("createDDict(NULL,0)", (void*)ZSTD_createDDict(NULL, 0));
        show_rc("freeDDict(NULL)", ZSTD_freeDDict(NULL));
        show_rc("freeCDict(NULL)", ZSTD_freeCDict(NULL));
    }
    /* --- ZSTD_d_refMultipleDDicts with several DDicts --- */
    {
        #define NMULTI 4
        unsigned char dicts[NMULTI][2048];
        ZSTD_DDict* dds[NMULTI];
        ZSTD_CDict* cds[NMULTI];
        size_t frames[NMULTI];
        int i, mm;
        for (i = 0; i < NMULTI; i++) {
            rs(0x7000ULL + i);
            fill_text(dicts[i], 2048);
            /* make each dictionary distinct in its first bytes too */
            dicts[i][0] = (unsigned char)(0x30 + i);
            cds[i] = ZSTD_createCDict(dicts[i], 2048, 6);
            dds[i] = ZSTD_createDDict(dicts[i], 2048);
        }
        for (mm = 0; mm <= 1; mm++) {
            ZSTD_DCtx* d = ZSTD_createDCtx();
            char nm[128];
            snprintf(nm, sizeof nm, "d_refMultipleDDicts=%d", mm);
            show_rc(nm, ZSTD_DCtx_setParameter(d, ZSTD_d_refMultipleDDicts, mm));
            for (i = 0; i < NMULTI; i++) {
                snprintf(nm, sizeof nm, "  refDDict[%d] (multi=%d)", i, mm);
                show_rc(nm, ZSTD_DCtx_refDDict(d, dds[i]));
            }
            /* one frame per dictionary, decompressed by the same dctx */
            for (i = 0; i < NMULTI; i++) {
                ZSTD_CCtx* c = ZSTD_createCCtx();
                size_t cs, ds;
                rs(0x8000ULL + i);
                fill_text(src, 20000);
                ZSTD_CCtx_refCDict(c, cds[i]);
                pat(cmp, 4096);
                cs = ZSTD_compress2(c, cmp, cmpCap, src, 20000);
                snprintf(nm, sizeof nm, "  multi c2[%d,multi=%d]", i, mm);
                show_buf(nm, cs, cmp);
                ZSTD_freeCCtx(c);
                if (ZSTD_isError(cs)) continue;
                pat(dec, 4096);
                ds = ZSTD_decompressDCtx(d, dec, decCap, cmp, cs);
                snprintf(nm, sizeof nm, "  multi dec[%d,multi=%d]", i, mm);
                if (ZSTD_isError(ds)) show_rc(nm, ds);
                else P("%-56s OK %llu %s\n", nm, (unsigned long long)ds,
                       (ds == 20000 && !memcmp(dec, src, 20000)) ? "match" : "MISMATCH");
            }
            ZSTD_freeDCtx(d);
        }
        for (i = 0; i < NMULTI; i++) { ZSTD_freeCDict(cds[i]); ZSTD_freeDDict(dds[i]); }
        #undef NMULTI
    }
    /* --- enableDedicatedDictSearch --- */
    {   int dds_on;
        for (dds_on = 0; dds_on <= 1; dds_on++) {
            ZSTD_CCtx_params* pp = ZSTD_createCCtxParams();
            ZSTD_CDict* cd;
            char nm[128];
            ZSTD_CCtxParams_init(pp, 12);
            ZSTD_CCtxParams_setParameter(pp, ZSTD_c_enableDedicatedDictSearch, dds_on);
            cd = ZSTD_createCDict_advanced2(traindict, traindict_n, ZSTD_dlm_byCopy,
                                            ZSTD_dct_auto, pp, ZSTD_defaultCMem);
            snprintf(nm, sizeof nm, "dedicatedDictSearch=%d cdict", dds_on);
            SHOWP(nm, (void*)cd);
            if (cd) {
                ZSTD_CCtx* c = ZSTD_createCCtx();
                snprintf(nm, sizeof nm, "dedicatedDictSearch=%d", dds_on);
                ZSTD_CCtx_refCDict(c, cd);
                rs(0x4444ULL); fill_text(src, DICTSRC_N);
                dict_roundtrip(nm, c, traindict, traindict_n, NULL, 0);
                ZSTD_freeCCtx(c);
                ZSTD_freeCDict(cd);
            }
            ZSTD_freeCCtxParams(pp);
        }
    }
}

/* ================================================================== */
/* PHASE 5 : static allocation                                         */
/* ================================================================== */
static void* xalign(size_t n) {
    void* p = NULL;
    if (n == 0) n = 8;
    n = (n + 63u) & ~(size_t)63u;
    if (posix_memalign(&p, 64, n) != 0) return NULL;
    memset(p, 0, n);
    return p;
}

static void phase_static(void) {
    int lvl;
    BANNER("PHASE 5: estimate* + initStatic*");
    rs(0x57A71CULL);
    fill_text(src, 60000);

    for (lvl = -5; lvl <= 22; lvl += 3) {
        char nm[128];
        snprintf(nm, sizeof nm, "estimateCCtxSize(%d)", lvl);      SHOWU(nm, ZSTD_estimateCCtxSize(lvl));
        snprintf(nm, sizeof nm, "estimateCStreamSize(%d)", lvl);   SHOWU(nm, ZSTD_estimateCStreamSize(lvl));
        snprintf(nm, sizeof nm, "estimateCDictSize(4096,%d)", lvl);SHOWU(nm, ZSTD_estimateCDictSize(4096, lvl));
    }
    SHOWU("estimateDCtxSize", ZSTD_estimateDCtxSize());
    {   int wl;
        for (wl = 10; wl <= 27; wl += 2) {
            char nm[128];
            snprintf(nm, sizeof nm, "estimateDStreamSize(1<<%d)", wl);
            SHOWU(nm, ZSTD_estimateDStreamSize((size_t)1 << wl));
        }
    }
    {   int l;
        for (l = 1; l <= 19; l += 6) {
            ZSTD_compressionParameters cp = ZSTD_getCParams(l, 60000, 0);
            char nm[128];
            snprintf(nm, sizeof nm, "estimateCCtxSize_usingCParams(L%d)", l);
            SHOWU(nm, ZSTD_estimateCCtxSize_usingCParams(cp));
            snprintf(nm, sizeof nm, "estimateCStreamSize_usingCParams(L%d)", l);
            SHOWU(nm, ZSTD_estimateCStreamSize_usingCParams(cp));
            snprintf(nm, sizeof nm, "estimateCDictSize_advanced(L%d,byCopy)", l);
            SHOWU(nm, ZSTD_estimateCDictSize_advanced(4096, cp, ZSTD_dlm_byCopy));
            snprintf(nm, sizeof nm, "estimateCDictSize_advanced(L%d,byRef)", l);
            SHOWU(nm, ZSTD_estimateCDictSize_advanced(4096, cp, ZSTD_dlm_byRef));
        }
    }

    /* ---- static CCtx: exact size, and one byte short ---- */
    {   size_t need = ZSTD_estimateCCtxSize(9);
        void* ws = xalign(need + 64);
        int shrink;
        for (shrink = 0; shrink <= 1; shrink++) {
            size_t give = shrink ? need - 1 : need;
            ZSTD_CCtx* c = ZSTD_initStaticCCtx(ws, give);
            char nm[128];
            snprintf(nm, sizeof nm, "initStaticCCtx(short=%d)", shrink);
            SHOWP(nm, (void*)c);
            if (!c) continue;
            SHOWU("  sizeof_CCtx(static)", ZSTD_sizeof_CCtx(c));
            {   size_t cs;
                pat(cmp, 4096);
                cs = ZSTD_compressCCtx(c, cmp, cmpCap, src, 60000, 9);
                show_buf("  static compressCCtx L9", cs, cmp);
                if (!ZSTD_isError(cs)) {
                    size_t ds;
                    pat(dec, 4096);
                    ds = ZSTD_decompress(dec, decCap, cmp, cs);
                    P("%-56s %s\n", "  static roundtrip",
                      (!ZSTD_isError(ds) && ds == 60000 && !memcmp(dec, src, 60000)) ? "match" : "MISMATCH");
                }
                /* a level the workspace was not sized for */
                pat(cmp, 4096);
                show_rc("  static compressCCtx L22", ZSTD_compressCCtx(c, cmp, cmpCap, src, 60000, 22));
            }
            show_rc("  freeCCtx(static)", ZSTD_freeCCtx(c));
        }
        free(ws);
        SHOWP("initStaticCCtx(NULL,0)", (void*)ZSTD_initStaticCCtx(NULL, 0));
    }
    /* ---- static CStream ---- */
    {   size_t need = ZSTD_estimateCStreamSize(6);
        void* ws = xalign(need + 64);
        int shrink;
        for (shrink = 0; shrink <= 1; shrink++) {
            size_t give = shrink ? need - 1 : need;
            ZSTD_CStream* z = ZSTD_initStaticCStream(ws, give);
            char nm[128];
            snprintf(nm, sizeof nm, "initStaticCStream(short=%d)", shrink);
            SHOWP(nm, (void*)z);
            if (!z) continue;
            {   ZSTD_inBuffer ib; ZSTD_outBuffer ob;
                size_t r; int guard = 0;
                show_rc("  initCStream(static,6)", ZSTD_initCStream(z, 6));
                ib.src = src; ib.size = 60000; ib.pos = 0;
                pat(cmp, 4096);
                ob.dst = cmp; ob.size = cmpCap; ob.pos = 0;
                do { r = ZSTD_compressStream2(z, &ob, &ib, ZSTD_e_end); }
                while (!ZSTD_isError(r) && r != 0 && ++guard < 10000);
                if (ZSTD_isError(r)) show_rc("  static cstream", r);
                else SHOW("  static cstream", cmp, ob.pos);
                if (!ZSTD_isError(r)) {
                    size_t ds;
                    pat(dec, 4096);
                    ds = ZSTD_decompress(dec, decCap, cmp, ob.pos);
                    P("%-56s %s\n", "  static cstream roundtrip",
                      (!ZSTD_isError(ds) && ds == 60000 && !memcmp(dec, src, 60000)) ? "match" : "MISMATCH");
                }
            }
        }
        free(ws);
    }
    /* ---- static DCtx / DStream ---- */
    {   size_t need = ZSTD_estimateDCtxSize();
        void* ws = xalign(need + 64);
        size_t cs;
        pat(cmp, 4096);
        cs = ZSTD_compress(cmp, cmpCap, src, 60000, 5);
        show_buf("frame for static dctx", cs, cmp);
        {   int shrink;
            for (shrink = 0; shrink <= 1; shrink++) {
                ZSTD_DCtx* d = ZSTD_initStaticDCtx(ws, shrink ? need - 1 : need);
                char nm[128];
                snprintf(nm, sizeof nm, "initStaticDCtx(short=%d)", shrink);
                SHOWP(nm, (void*)d);
                if (!d || ZSTD_isError(cs)) continue;
                SHOWU("  sizeof_DCtx(static)", ZSTD_sizeof_DCtx(d));
                pat(dec, 4096);
                {   size_t ds = ZSTD_decompressDCtx(d, dec, decCap, cmp, cs);
                    if (ZSTD_isError(ds)) show_rc("  static decompressDCtx", ds);
                    else P("%-56s OK %llu %s\n", "  static decompressDCtx", (unsigned long long)ds,
                           (ds == 60000 && !memcmp(dec, src, 60000)) ? "match" : "MISMATCH");
                }
                show_rc("  freeDCtx(static)", ZSTD_freeDCtx(d));
            }
        }
        free(ws);
        SHOWP("initStaticDCtx(NULL,0)", (void*)ZSTD_initStaticDCtx(NULL, 0));
    }
    {   size_t cs;
        pat(cmp, 4096);
        cs = ZSTD_compress(cmp, cmpCap, src, 60000, 5);
        SHOWU("estimateDStreamSize_fromFrame", ZSTD_estimateDStreamSize_fromFrame(cmp, cs));
        show_rc("estimateDStreamSize_fromFrame(junk)", ZSTD_estimateDStreamSize_fromFrame(rawdict, 40));
        {   size_t need = ZSTD_estimateDStreamSize_fromFrame(cmp, cs);
            int shrink;
            void* ws;
            if (ZSTD_isError(need)) need = ZSTD_estimateDStreamSize(1u << 20);
            ws = xalign(need + 64);
            for (shrink = 0; shrink <= 1; shrink++) {
                ZSTD_DStream* z = ZSTD_initStaticDStream(ws, shrink ? need - 1 : need);
                char nm[128];
                snprintf(nm, sizeof nm, "initStaticDStream(short=%d)", shrink);
                SHOWP(nm, (void*)z);
                if (!z) continue;
                {   ZSTD_inBuffer ib; ZSTD_outBuffer ob;
                    size_t r; int guard = 0;
                    show_rc("  initDStream(static)", ZSTD_initDStream(z));
                    ib.src = cmp; ib.size = cs; ib.pos = 0;
                    pat(dec, 4096);
                    ob.dst = dec; ob.size = decCap; ob.pos = 0;
                    do { r = ZSTD_decompressStream(z, &ob, &ib); }
                    while (!ZSTD_isError(r) && r != 0 && ib.pos < ib.size && ++guard < 10000);
                    if (ZSTD_isError(r)) show_rc("  static dstream", r);
                    else P("%-56s out=%llu %s\n", "  static dstream", (unsigned long long)ob.pos,
                           (ob.pos == 60000 && !memcmp(dec, src, 60000)) ? "match" : "MISMATCH");
                }
            }
            free(ws);
        }
    }
    /* ---- static CDict / DDict ---- */
    {   ZSTD_compressionParameters cp = ZSTD_getCParams(6, 60000, traindict_n);
        int dm;
        for (dm = 0; dm <= 1; dm++) {
            size_t need = ZSTD_estimateCDictSize_advanced(traindict_n, cp, (ZSTD_dictLoadMethod_e)dm);
            void* ws = xalign(need + 64);
            int shrink;
            for (shrink = 0; shrink <= 1; shrink++) {
                const ZSTD_CDict* cd = ZSTD_initStaticCDict(ws, shrink ? need - 1 : need,
                                            traindict, traindict_n,
                                            (ZSTD_dictLoadMethod_e)dm, ZSTD_dct_auto, cp);
                char nm[128];
                snprintf(nm, sizeof nm, "initStaticCDict(dlm%d,short=%d)", dm, shrink);
                SHOWP(nm, (void*)cd);
                if (!cd) continue;
                SHOWU("  sizeof_CDict(static)", ZSTD_sizeof_CDict(cd));
                SHOWU("  dictID(static cdict)", ZSTD_getDictID_fromCDict(cd));
                {   ZSTD_CCtx* c = ZSTD_createCCtx();
                    size_t cs;
                    pat(cmp, 4096);
                    cs = ZSTD_compress_usingCDict(c, cmp, cmpCap, src, 60000, cd);
                    show_buf("  static cdict compress", cs, cmp);
                    if (!ZSTD_isError(cs)) {
                        ZSTD_DCtx* d = ZSTD_createDCtx();
                        size_t ds;
                        pat(dec, 4096);
                        ZSTD_DCtx_loadDictionary(d, traindict, traindict_n);
                        ds = ZSTD_decompressDCtx(d, dec, decCap, cmp, cs);
                        P("%-56s %s\n", "  static cdict roundtrip",
                          (!ZSTD_isError(ds) && ds == 60000 && !memcmp(dec, src, 60000)) ? "match" : "MISMATCH");
                        ZSTD_freeDCtx(d);
                    }
                    ZSTD_freeCCtx(c);
                }
            }
            free(ws);
        }
        for (dm = 0; dm <= 1; dm++) {
            size_t need = ZSTD_estimateDDictSize(traindict_n, (ZSTD_dictLoadMethod_e)dm);
            void* ws = xalign(need + 64);
            int shrink;
            for (shrink = 0; shrink <= 1; shrink++) {
                const ZSTD_DDict* dd = ZSTD_initStaticDDict(ws, shrink ? need - 1 : need,
                                            traindict, traindict_n,
                                            (ZSTD_dictLoadMethod_e)dm, ZSTD_dct_auto);
                char nm[128];
                snprintf(nm, sizeof nm, "initStaticDDict(dlm%d,short=%d)", dm, shrink);
                SHOWP(nm, (void*)dd);
                if (!dd) continue;
                SHOWU("  sizeof_DDict(static)", ZSTD_sizeof_DDict(dd));
                SHOWU("  DDict_dictSize(static)", ZSTD_DDict_dictSize(dd));
                SHOWU("  dictID(static ddict)", ZSTD_getDictID_fromDDict(dd));
                {   ZSTD_CCtx* c = ZSTD_createCCtx();
                    size_t cs;
                    ZSTD_CCtx_loadDictionary(c, traindict, traindict_n);
                    pat(cmp, 4096);
                    cs = ZSTD_compress2(c, cmp, cmpCap, src, 60000);
                    if (!ZSTD_isError(cs)) {
                        size_t ds;
                        pat(dec, 4096);
                        ds = ZSTD_decompress_usingDDict(ZSTD_createDCtx(), dec, decCap, cmp, cs, dd);
                        if (ZSTD_isError(ds)) show_rc("  static ddict decompress", ds);
                        else P("%-56s OK %llu %s\n", "  static ddict decompress", (unsigned long long)ds,
                               (ds == 60000 && !memcmp(dec, src, 60000)) ? "match" : "MISMATCH");
                    }
                    ZSTD_freeCCtx(c);
                }
            }
            free(ws);
        }
    }
}

/* ================================================================== */
/* PHASE 6 : buffer-less streaming, both directions                    */
/* ================================================================== */

/* Buffer-less decompression of a complete frame sitting in cmp[0..cSize). */
static void bufferless_decode(const char* tag, size_t cSize, size_t expect,
                              const void* dict, size_t dictSize,
                              const ZSTD_DDict* ddict) {
    ZSTD_DCtx* d = ZSTD_createDCtx();
    size_t ipos = 0, opos = 0;
    int guard = 0;
    char nm[192];

    if (ddict)          show_rc("  decompressBegin_usingDDict", ZSTD_decompressBegin_usingDDict(d, ddict));
    else if (dict)      show_rc("  decompressBegin_usingDict", ZSTD_decompressBegin_usingDict(d, dict, dictSize));
    else                show_rc("  decompressBegin", ZSTD_decompressBegin(d));

    snprintf(nm, sizeof nm, "  nextInputType(initial) [%s]", tag);
    SHOWZ(nm, (int)ZSTD_nextInputType(d));

    pat(dec, 4096);
    for (;;) {
        size_t need = ZSTD_nextSrcSizeToDecompress(d);
        size_t r;
        if (ZSTD_isError(need)) { show_rc("  nextSrcSizeToDecompress", need); break; }
        if (need == 0) break;
        if (ipos + need > cSize) {
            P("%-56s truncated need=%llu left=%llu\n", "  bufferless dec",
              (unsigned long long)need, (unsigned long long)(cSize - ipos));
            g_calls++;
            break;
        }
        r = ZSTD_decompressContinue(d, dec + opos, decCap - opos, cmp + ipos, need);
        if (ZSTD_isError(r)) { show_rc("  decompressContinue", r); break; }
        ipos += need;
        opos += r;
        if (++guard > 100000) { P("%-56s guard\n", "  bufferless dec"); break; }
    }
    snprintf(nm, sizeof nm, "  bufferless dec [%s]", tag);
    P("%-56s in=%llu out=%llu %s\n", nm, (unsigned long long)ipos, (unsigned long long)opos,
      (opos == expect && (expect == 0 || !memcmp(dec, src, expect))) ? "match" : "MISMATCH");
    snprintf(nm, sizeof nm, "  nextInputType(final) [%s]", tag);
    SHOWZ(nm, (int)ZSTD_nextInputType(d));
    ZSTD_freeDCtx(d);
}

static void phase_bufferless(void) {
    static const size_t chunks[] = { 1, 17, 4096, 65536, 131072 };
    int ci, mode;
    BANNER("PHASE 6: buffer-less streaming");
    rs(0xB0FFULL);
    fill_text(src, 200000);

    /* modes: 0 compressBegin, 1 _usingDict, 2 _advanced, 3 _usingCDict,
     *        4 _usingCDict_advanced, 5 _advanced_internal, 6 _usingCDict_deprecated */
    for (mode = 0; mode <= 6; mode++) {
        for (ci = 0; ci < (int)(sizeof(chunks)/sizeof(chunks[0])); ci++) {
            ZSTD_CCtx* c = ZSTD_createCCtx();
            ZSTD_CDict* cd = NULL;
            ZSTD_CCtx_params* pp = NULL;
            size_t chunk = chunks[ci];
            size_t pos = 0, out = 0;
            size_t rc = 0;
            char nm[192];
            const char* mn[] = { "begin", "begin_usingDict", "begin_advanced", "begin_usingCDict",
                                 "begin_usingCDict_advanced", "begin_advanced_internal",
                                 "begin_usingCDict_deprecated" };
            const size_t total = 200000;

            snprintf(nm, sizeof nm, "%s chunk=%llu", mn[mode], (unsigned long long)chunk);

            switch (mode) {
            case 0: rc = ZSTD_compressBegin(c, 7); break;
            case 1: rc = ZSTD_compressBegin_usingDict(c, traindict, traindict_n, 7); break;
            case 2: { ZSTD_parameters prm = ZSTD_getParams(7, total, 0);
                      prm.fParams.checksumFlag = 1;
                      rc = ZSTD_compressBegin_advanced(c, traindict, traindict_n, prm, total); } break;
            case 3: cd = ZSTD_createCDict(traindict, traindict_n, 7);
                    rc = ZSTD_compressBegin_usingCDict(c, cd); break;
            case 4: { ZSTD_frameParameters fp;
                      cd = ZSTD_createCDict(traindict, traindict_n, 7);
                      fp.contentSizeFlag = 1; fp.checksumFlag = 1; fp.noDictIDFlag = 0;
                      rc = ZSTD_compressBegin_usingCDict_advanced(c, cd, fp, total); } break;
            case 5: pp = ZSTD_createCCtxParams();
                    ZSTD_CCtxParams_init(pp, 7);
                    ZSTD_CCtxParams_setParameter(pp, ZSTD_c_checksumFlag, 1);
                    rc = ZSTD_compressBegin_advanced_internal(c, traindict, traindict_n,
                             (int)ZSTD_dct_auto, 0, NULL, pp, total);
                    break;
            case 6: cd = ZSTD_createCDict(traindict, traindict_n, 7);
                    rc = ZSTD_compressBegin_usingCDict_deprecated(c, cd); break;
            }
            show_rc(nm, rc);
            if (ZSTD_isError(rc)) { ZSTD_freeCDict(cd); ZSTD_freeCCtxParams(pp); ZSTD_freeCCtx(c); continue; }

            SHOWU("  getBlockSize", ZSTD_getBlockSize(c));

            /* copyCCtx from the prepared context, then drive the copy */
            {   ZSTD_CCtx* c2 = ZSTD_createCCtx();
                size_t cr = ZSTD_copyCCtx(c2, c, total);
                show_rc("  copyCCtx", cr);
                if (!ZSTD_isError(cr)) {
                    size_t p2 = 0, o2 = 0;
                    pat(cmp, 4096);
                    while (p2 < total) {
                        size_t take = (total - p2 < chunk) ? (total - p2) : chunk;
                        size_t r;
                        if (p2 + take >= total) r = ZSTD_compressEnd(c2, cmp + o2, cmpCap - o2, src + p2, take);
                        else                    r = ZSTD_compressContinue(c2, cmp + o2, cmpCap - o2, src + p2, take);
                        if (ZSTD_isError(r)) { show_rc("  copy compressContinue/End", r); o2 = 0; break; }
                        o2 += r; p2 += take;
                    }
                    if (o2) {
                        SHOW("  copyCCtx frame", cmp, o2);
                        bufferless_decode("copyCCtx", o2, total,
                                          (mode == 0) ? NULL : (const void*)traindict,
                                          traindict_n, NULL);
                    }
                }
                ZSTD_freeCCtx(c2);
            }

            /* the original context, using the _public variants half the time */
            pat(cmp, 4096);
            pos = 0; out = 0;
            while (pos < total) {
                size_t take = (total - pos < chunk) ? (total - pos) : chunk;
                size_t r;
                int last = (pos + take >= total);
                if (ci & 1) r = last ? ZSTD_compressEnd_public(c, cmp + out, cmpCap - out, src + pos, take)
                                     : ZSTD_compressContinue_public(c, cmp + out, cmpCap - out, src + pos, take);
                else        r = last ? ZSTD_compressEnd(c, cmp + out, cmpCap - out, src + pos, take)
                                     : ZSTD_compressContinue(c, cmp + out, cmpCap - out, src + pos, take);
                if (ZSTD_isError(r)) { show_rc("  compressContinue/End", r); out = 0; break; }
                out += r; pos += take;
            }
            if (out) {
                snprintf(nm, sizeof nm, "  frame[%s,chunk=%llu]", mn[mode], (unsigned long long)chunk);
                SHOW(nm, cmp, out);
                SHOWU("  getFrameContentSize", ZSTD_getFrameContentSize(cmp, out));
                show_rc("  findFrameCompressedSize", ZSTD_findFrameCompressedSize(cmp, out));
                /* whole-frame decode as a cross-check */
                {   ZSTD_DCtx* d = ZSTD_createDCtx();
                    size_t ds;
                    if (mode != 0) ZSTD_DCtx_loadDictionary(d, traindict, traindict_n);
                    pat(dec, 4096);
                    ds = ZSTD_decompressDCtx(d, dec, decCap, cmp, out);
                    if (ZSTD_isError(ds)) show_rc("  whole-frame dec", ds);
                    else P("%-56s OK %llu %s\n", "  whole-frame dec", (unsigned long long)ds,
                           (ds == total && !memcmp(dec, src, total)) ? "match" : "MISMATCH");
                    ZSTD_freeDCtx(d);
                }
                /* buffer-less decode */
                bufferless_decode(mn[mode], out, total,
                                  (mode == 0) ? NULL : (const void*)traindict, traindict_n, NULL);
                /* decodingBufferSize_min against this frame's header */
                {   ZSTD_FrameHeader fh;
                    memset(&fh, 0, sizeof fh);
                    if (!ZSTD_isError(ZSTD_getFrameHeader(&fh, cmp, out)))
                        SHOWU("  decodingBufferSize_min", ZSTD_decodingBufferSize_min(fh.windowSize, fh.frameContentSize));
                }
            }
            ZSTD_freeCDict(cd);
            ZSTD_freeCCtxParams(pp);
            ZSTD_freeCCtx(c);
        }
    }

    /* decompressBegin_usingDDict path */
    {   ZSTD_CCtx* c = ZSTD_createCCtx();
        ZSTD_DDict* dd = ZSTD_createDDict(traindict, traindict_n);
        size_t pos = 0, out = 0;
        const size_t total = 120000;
        show_rc("compressBegin_usingDict(for DDict decode)",
                ZSTD_compressBegin_usingDict(c, traindict, traindict_n, 5));
        pat(cmp, 4096);
        while (pos < total) {
            size_t take = (total - pos < 32768) ? (total - pos) : 32768;
            size_t r = (pos + take >= total)
                     ? ZSTD_compressEnd(c, cmp + out, cmpCap - out, src + pos, take)
                     : ZSTD_compressContinue(c, cmp + out, cmpCap - out, src + pos, take);
            if (ZSTD_isError(r)) { show_rc("  cont", r); out = 0; break; }
            out += r; pos += take;
        }
        if (out) { SHOW("DDict-decode frame", cmp, out); bufferless_decode("usingDDict", out, total, NULL, 0, dd); }
        ZSTD_freeDDict(dd);
        ZSTD_freeCCtx(c);
    }

    /* ---- raw blocks: compressBlock / decompressBlock / insertBlock ---- */
    {   int dep, bi;
        for (dep = 0; dep <= 1; dep++) {
            ZSTD_CCtx* c = ZSTD_createCCtx();
            ZSTD_DCtx* d = ZSTD_createDCtx();
            size_t bs;
            show_rc("compressBegin(for blocks)", ZSTD_compressBegin(c, 6));
            bs = ZSTD_getBlockSize(c);
            SHOWU("getBlockSize(blocks)", bs);
            show_rc("decompressBegin(for blocks)", ZSTD_decompressBegin(d));
            if (bs > 200000) bs = 200000;
            for (bi = 0; bi < 3; bi++) {
                size_t off = (size_t)bi * bs;
                size_t n = bs;
                size_t cb, db;
                char nm[160];
                if (off + n > 200000) n = 200000 - off;
                if (n == 0) break;
                pat(cmp, 4096);
                cb = dep ? ZSTD_compressBlock_deprecated(c, cmp, cmpCap, src + off, n)
                         : ZSTD_compressBlock(c, cmp, cmpCap, src + off, n);
                snprintf(nm, sizeof nm, "compressBlock%s[%d]", dep ? "_deprecated" : "", bi);
                show_buf(nm, cb, cmp);
                if (ZSTD_isError(cb)) break;
                pat(dec, 4096);
                if (cb == 0) {
                    /* not compressible: the block must be inserted verbatim */
                    snprintf(nm, sizeof nm, "insertBlock[%d]", bi);
                    show_rc(nm, ZSTD_insertBlock(d, src + off, n));
                    continue;
                }
                db = dep ? ZSTD_decompressBlock_deprecated(d, dec, decCap, cmp, cb)
                         : ZSTD_decompressBlock(d, dec, decCap, cmp, cb);
                snprintf(nm, sizeof nm, "decompressBlock%s[%d]", dep ? "_deprecated" : "", bi);
                if (ZSTD_isError(db)) { show_rc(nm, db); break; }
                P("%-56s OK %llu %s\n", nm, (unsigned long long)db,
                  (db == n && !memcmp(dec, src + off, n)) ? "match" : "MISMATCH");
            }
            /* seqStore inspection after real block compression */
            {   const void* ss = ZSTD_getSeqStore(c);
                SHOWP("getSeqStore", (void*)ss);
                if (ss) SHOWZ("seqToCodes(longOffsets)", ZSTD_seqToCodes(ss));
            }
            /* copyDCtx then continue decoding from the copy */
            {   ZSTD_DCtx* d2 = ZSTD_createDCtx();
                ZSTD_decompressBegin(d2);
                ZSTD_copyDCtx(d2, d);
                P("%-56s done\n", "copyDCtx");
                SHOWZ("  nextInputType(copy)", (int)ZSTD_nextInputType(d2));
                SHOWU("  nextSrcSizeToDecompress(copy)", ZSTD_nextSrcSizeToDecompress(d2));
                ZSTD_freeDCtx(d2);
            }
            ZSTD_freeDCtx(d);
            ZSTD_freeCCtx(c);
        }
    }
    /* decodingBufferSize_min over a grid */
    {   int wi, fi;
        static const unsigned long long ws[] = { 1024, 65536, 1u<<20, 1u<<27 };
        static const unsigned long long fc[] = { 0, 1, 1000, 1u<<20, ZSTD_CONTENTSIZE_UNKNOWN };
        for (wi = 0; wi < 4; wi++) for (fi = 0; fi < 5; fi++) {
            char nm[128];
            snprintf(nm, sizeof nm, "decodingBufferSize_min[%llu,%llu]", ws[wi], fc[fi]);
            show_rc(nm, ZSTD_decodingBufferSize_min(ws[wi], fc[fi]));
        }
    }
}

/* ================================================================== */
/* PHASE 7 : sequence APIs                                             */
/* ================================================================== */
static void phase_sequences(void) {
    static const size_t sizes[] = { 1000, 40000, 150000 };
    int si, bd, rr;
    BANNER("PHASE 7: sequence APIs");

    for (si = 0; si < 3; si++) {
        size_t n = sizes[si];
        size_t bound;
        ZSTD_Sequence* seqs;
        size_t nbSeq;
        char nm[176];

        rs(0x5E00ULL + si);
        fill_text(src, n);

        bound = ZSTD_sequenceBound(n);
        snprintf(nm, sizeof nm, "sequenceBound(%llu)", (unsigned long long)n);
        SHOWU(nm, bound);
        seqs = (ZSTD_Sequence*)malloc((bound + 8) * sizeof(ZSTD_Sequence));
        memset(seqs, 0, (bound + 8) * sizeof(ZSTD_Sequence));

        {   ZSTD_CCtx* g = ZSTD_createCCtx();
            ZSTD_CCtx_setParameter(g, ZSTD_c_compressionLevel, 9);
            nbSeq = ZSTD_generateSequences(g, seqs, bound, src, n);
            snprintf(nm, sizeof nm, "generateSequences(%llu)", (unsigned long long)n);
            show_rc(nm, nbSeq);
            /* too-small capacity */
            snprintf(nm, sizeof nm, "generateSequences(cap=1,%llu)", (unsigned long long)n);
            show_rc(nm, ZSTD_generateSequences(g, seqs + bound + 1, 1, src, n));
            ZSTD_freeCCtx(g);
        }
        if (ZSTD_isError(nbSeq)) { free(seqs); continue; }
        snprintf(nm, sizeof nm, "sequences fnv[%llu]", (unsigned long long)n);
        SHOW(nm, seqs, nbSeq * sizeof(ZSTD_Sequence));

        /* ZSTD_get1BlockSummary walks to the first delimiter */
        {   AdvBlockSummary bsum = ZSTD_get1BlockSummary(seqs, nbSeq);
            snprintf(nm, sizeof nm, "get1BlockSummary[%llu]", (unsigned long long)n);
            if (ZSTD_isError(bsum.nbSequences)) show_rc(nm, bsum.nbSequences);
            else P("%-56s nbSeq=%llu blockSize=%llu litSize=%llu\n", nm,
                   (unsigned long long)bsum.nbSequences,
                   (unsigned long long)bsum.blockSize,
                   (unsigned long long)bsum.litSize);
            /* degenerate: a lone delimiter */
            {   ZSTD_Sequence one[1];
                AdvBlockSummary b1;
                memset(one, 0, sizeof one);
                b1 = ZSTD_get1BlockSummary(one, 1);
                P("%-56s nbSeq=%lld blockSize=%llu litSize=%llu\n", "get1BlockSummary[lone delim]",
                  (long long)b1.nbSequences, (unsigned long long)b1.blockSize,
                  (unsigned long long)b1.litSize);
                g_calls++;
            }
        }

        /* ZSTD_convertBlockSequences needs a prepared cctx and one block worth */
        for (rr = 0; rr <= 1; rr++) {
            ZSTD_CCtx* c = ZSTD_createCCtx();
            AdvBlockSummary bsum;
            ZSTD_CCtx_setParameter(c, ZSTD_c_compressionLevel, 9);
            ZSTD_CCtx_setParameter(c, ZSTD_c_blockDelimiters, ZSTD_sf_explicitBlockDelimiters);
            /* prime the internal workspace via a real compression */
            pat(cmp, 4096);
            ZSTD_compress2(c, cmp, cmpCap, src, n);
            ZSTD_CCtx_reset(c, ZSTD_reset_session_only);
            pat(cmp, 4096);
            ZSTD_compress2(c, cmp, cmpCap, src, n);
            bsum = ZSTD_get1BlockSummary(seqs, nbSeq);
            snprintf(nm, sizeof nm, "convertBlockSequences[%llu,rep%d]", (unsigned long long)n, rr);
            if (!ZSTD_isError(bsum.nbSequences))
                show_rc(nm, ZSTD_convertBlockSequences(c, seqs, bsum.nbSequences, rr));
            /* zero sequences must be rejected, not crash */
            snprintf(nm, sizeof nm, "convertBlockSequences[huge nb,rep%d]", rr);
            show_rc(nm, ZSTD_convertBlockSequences(c, seqs, (size_t)1 << 40, rr));
            ZSTD_freeCCtx(c);
        }

        /* compressSequences with both delimiter modes and both repcode modes */
        for (bd = 0; bd <= 1; bd++) {
            ZSTD_Sequence* work = (ZSTD_Sequence*)malloc((nbSeq + 8) * sizeof(ZSTD_Sequence));
            size_t m = nbSeq;
            memcpy(work, seqs, nbSeq * sizeof(ZSTD_Sequence));
            memset(work + nbSeq, 0, 8 * sizeof(ZSTD_Sequence));
            if (bd == 0) {
                m = ZSTD_mergeBlockDelimiters(work, nbSeq);
                snprintf(nm, sizeof nm, "mergeBlockDelimiters[%llu]", (unsigned long long)n);
                SHOWU(nm, m);
                snprintf(nm, sizeof nm, "merged fnv[%llu]", (unsigned long long)n);
                SHOW(nm, work, m * sizeof(ZSTD_Sequence));
            }
            for (rr = 0; rr <= 2; rr++) {
                int vs;
                for (vs = 0; vs <= 1; vs++) {
                    ZSTD_CCtx* c = ZSTD_createCCtx();
                    size_t cs;
                    ZSTD_CCtx_setParameter(c, ZSTD_c_compressionLevel, 9);
                    ZSTD_CCtx_setParameter(c, ZSTD_c_blockDelimiters,
                        bd ? ZSTD_sf_explicitBlockDelimiters : ZSTD_sf_noBlockDelimiters);
                    ZSTD_CCtx_setParameter(c, ZSTD_c_repcodeResolution, rr);
                    ZSTD_CCtx_setParameter(c, ZSTD_c_validateSequences, vs);
                    pat(cmp, 4096);
                    cs = ZSTD_compressSequences(c, cmp, cmpCap, work, m, src, n);
                    snprintf(nm, sizeof nm, "compressSequences[n=%llu,bd%d,rep%d,val%d]",
                             (unsigned long long)n, bd, rr, vs);
                    show_buf(nm, cs, cmp);
                    if (!ZSTD_isError(cs)) {
                        size_t ds;
                        pat(dec, 4096);
                        ds = ZSTD_decompress(dec, decCap, cmp, cs);
                        P("%-56s %s\n", "  roundtrip",
                          (!ZSTD_isError(ds) && ds == n && !memcmp(dec, src, n)) ? "match" : "MISMATCH");
                    }
                    /* dst too small */
                    pat(cmp, 4096);
                    snprintf(nm, sizeof nm, "  compressSequences dst=10[bd%d,rep%d]", bd, rr);
                    show_rc(nm, ZSTD_compressSequences(c, cmp, 10, work, m, src, n));
                    ZSTD_freeCCtx(c);
                }
            }
            free(work);
        }

        /* compressSequencesAndLiterals: explicit delimiters, literals extracted from src */
        {   size_t litSize = 0, i, cursor = 0;
            unsigned char* lits;
            for (i = 0; i < nbSeq; i++) litSize += seqs[i].litLength;
            lits = (unsigned char*)malloc(litSize + 64);
            memset(lits, 0, litSize + 64);
            for (i = 0; i < nbSeq; i++) {
                memcpy(lits + cursor, src + (cursor + 0), 0); /* placeholder, filled below */
                cursor += 0;
            }
            /* walk the sequences to copy out literals in order */
            {   size_t sp = 0, lp = 0;
                for (i = 0; i < nbSeq; i++) {
                    memcpy(lits + lp, src + sp, seqs[i].litLength);
                    lp += seqs[i].litLength;
                    sp += seqs[i].litLength + seqs[i].matchLength;
                }
                snprintf(nm, sizeof nm, "literals fnv[%llu]", (unsigned long long)n);
                SHOW(nm, lits, lp);
            }
            for (rr = 0; rr <= 2; rr++) {
                ZSTD_CCtx* c = ZSTD_createCCtx();
                size_t cs;
                ZSTD_CCtx_setParameter(c, ZSTD_c_compressionLevel, 9);
                ZSTD_CCtx_setParameter(c, ZSTD_c_blockDelimiters, ZSTD_sf_explicitBlockDelimiters);
                ZSTD_CCtx_setParameter(c, ZSTD_c_repcodeResolution, rr);
                pat(cmp, 4096);
                cs = ZSTD_compressSequencesAndLiterals(c, cmp, cmpCap, seqs, nbSeq,
                                                       lits, litSize, litSize + 64, n);
                snprintf(nm, sizeof nm, "compressSequencesAndLiterals[n=%llu,rep%d]",
                         (unsigned long long)n, rr);
                show_buf(nm, cs, cmp);
                if (!ZSTD_isError(cs)) {
                    size_t ds;
                    pat(dec, 4096);
                    ds = ZSTD_decompress(dec, decCap, cmp, cs);
                    P("%-56s %s\n", "  roundtrip",
                      (!ZSTD_isError(ds) && ds == n && !memcmp(dec, src, n)) ? "match" : "MISMATCH");
                }
                /* litCapacity < litSize must be rejected */
                snprintf(nm, sizeof nm, "  SAL litCap<litSize[rep%d]", rr);
                show_rc(nm, ZSTD_compressSequencesAndLiterals(c, cmp, cmpCap, seqs, nbSeq,
                                                              lits, litSize, litSize ? litSize - 1 : 0, n));
                /* noBlockDelimiters must be rejected */
                ZSTD_CCtx_setParameter(c, ZSTD_c_blockDelimiters, ZSTD_sf_noBlockDelimiters);
                snprintf(nm, sizeof nm, "  SAL noDelims[rep%d]", rr);
                show_rc(nm, ZSTD_compressSequencesAndLiterals(c, cmp, cmpCap, seqs, nbSeq,
                                                              lits, litSize, litSize + 64, n));
                /* checksum must be rejected */
                ZSTD_CCtx_setParameter(c, ZSTD_c_blockDelimiters, ZSTD_sf_explicitBlockDelimiters);
                ZSTD_CCtx_setParameter(c, ZSTD_c_checksumFlag, 1);
                snprintf(nm, sizeof nm, "  SAL checksum[rep%d]", rr);
                show_rc(nm, ZSTD_compressSequencesAndLiterals(c, cmp, cmpCap, seqs, nbSeq,
                                                              lits, litSize, litSize + 64, n));
                ZSTD_freeCCtx(c);
            }
            free(lits);
        }
        free(seqs);
    }

    /* registerSequenceProducer(NULL, NULL) must be a harmless no-op */
    {   ZSTD_CCtx* c = ZSTD_createCCtx();
        int fb;
        ZSTD_registerSequenceProducer(c, NULL, NULL);
        P("%-56s done\n", "registerSequenceProducer(NULL,NULL)");
        for (fb = 0; fb <= 1; fb++) {
            size_t cs;
            ZSTD_CCtx_reset(c, ZSTD_reset_session_and_parameters);
            ZSTD_registerSequenceProducer(c, NULL, NULL);
            ZSTD_CCtx_setParameter(c, ZSTD_c_enableSeqProducerFallback, fb);
            rs(0x5150ULL); fill_text(src, 50000);
            pat(cmp, 4096);
            cs = ZSTD_compress2(c, cmp, cmpCap, src, 50000);
            {   char nm[128];
                snprintf(nm, sizeof nm, "seqProducer NULL + fallback%d", fb);
                show_buf(nm, cs, cmp);
            }
        }
        ZSTD_freeCCtx(c);
    }
    /* sequenceBound edge cases */
    SHOWU("sequenceBound(0)", ZSTD_sequenceBound(0));
    SHOWU("sequenceBound(1)", ZSTD_sequenceBound(1));
    SHOWU("sequenceBound(1<<20)", ZSTD_sequenceBound(1u << 20));
    /* mergeBlockDelimiters on a tiny hand-built array */
    {   ZSTD_Sequence t[4];
        memset(t, 0, sizeof t);
        t[0].litLength = 3; t[0].matchLength = 4; t[0].offset = 1;
        t[1].litLength = 0; t[1].matchLength = 0; t[1].offset = 0;   /* delimiter */
        t[2].litLength = 5; t[2].matchLength = 6; t[2].offset = 2;
        t[3].litLength = 0; t[3].matchLength = 0; t[3].offset = 0;   /* delimiter */
        SHOWU("mergeBlockDelimiters(tiny)", ZSTD_mergeBlockDelimiters(t, 4));
        SHOW("mergeBlockDelimiters(tiny) fnv", t, sizeof t);
    }
}

/* ================================================================== */
/* PHASE 8 : multi-frame / skippable / checksum / decoder params       */
/* ================================================================== */
static void phase_frames(void) {
    size_t n1 = 30000, n2 = 17000, n3 = 1;
    size_t c1, c2, c3, total = 0;
    int v;
    BANNER("frames");
    rs(0xF00DULL);
    fill_text(src, n1);
    fill_rand(src + n1, n2);
    src[n1 + n2] = 0x42;
    pat(cmp, cmpCap);
    c1 = ZSTD_compress(cmp, cmpCap, src, n1, 3);
    show_buf("frame1", c1, cmp);
    total += c1;
    c2 = ZSTD_compress(cmp + total, cmpCap - total, src + n1, n2, 9);
    show_buf("frame2", c2, cmp + total);
    total += c2;
    { size_t s = ZSTD_writeSkippableFrame(cmp + total, cmpCap - total, src, 100, 7);
      show_buf("skip7", s, cmp + total); total += s; }
    c3 = ZSTD_compress(cmp + total, cmpCap - total, src + n1 + n2, n3, 1);
    show_buf("frame3", c3, cmp + total);
    total += c3;
    SHOWU("concat total", total);
    SHOW("concat", cmp, total);
    SHOWU("findDecompressedSize", ZSTD_findDecompressedSize(cmp, total));
    SHOWU("decompressBound", ZSTD_decompressBound(cmp, total));
    show_rc("findFrameCompressedSize", ZSTD_findFrameCompressedSize(cmp, total));
    pat(dec, decCap);
    show_buf("decompress concat", ZSTD_decompress(dec, decCap, cmp, total), dec);
    /* all 16 skippable magic variants + readSkippableFrame */
    for (v = 0; v < 18; v++) {
        char nm[64];
        size_t s;
        pat(cmp, 4096);
        s = ZSTD_writeSkippableFrame(cmp, cmpCap, src, 64, (unsigned)v);
        snprintf(nm, sizeof nm, "writeSkippableFrame[%d]", v);
        show_buf(nm, s, cmp);
        if (!ZSTD_isError(s)) {
            unsigned mv = 0xDEAD;
            size_t r;
            pat(dec, 1024);
            r = ZSTD_readSkippableFrame(dec, decCap, &mv, cmp, s);
            snprintf(nm, sizeof nm, "readSkippableFrame[%d]", v);
            show_buf(nm, r, dec);
            SHOWU("  magicVariant", mv);
            SHOWZ("  isSkippableFrame", ZSTD_isSkippableFrame(cmp, s));
            SHOWU("  frameContentSize", ZSTD_getFrameContentSize(cmp, s));
            {   ZSTD_FrameHeader h;
                size_t rc = ZSTD_getFrameHeader(&h, cmp, s);
                P("  gfh rc=%lld fcs=%llu ws=%llu bsm=%u ft=%d hs=%u did=%u ck=%u\n",
                  (long long)rc, h.frameContentSize, h.windowSize, h.blockSizeMax,
                  (int)h.frameType, h.headerSize, h.dictID, h.checksumFlag);
            }
        }
    }
    /* checksum handling: corrupt the last 4 bytes */
    {   size_t cs;
        int i;
        rs(0xC5C5ULL);
        fill_text(src, 50000);
        {   ZSTD_CCtx* c = ZSTD_createCCtx();
            ZSTD_CCtx_setParameter(c, ZSTD_c_checksumFlag, 1);
            pat(cmp, cmpCap);
            cs = ZSTD_compress2(c, cmp, cmpCap, src, 50000);
            ZSTD_freeCCtx(c);
        }
        show_buf("cksum frame", cs, cmp);
        for (i = 0; i < 4; i++) {
            ZSTD_DCtx* d = ZSTD_createDCtx();
            char nm[64];
            cmp[cs - 1 - i] ^= 0x80;
            pat(dec, decCap);
            snprintf(nm, sizeof nm, "cksum-corrupt[%d] validate", i);
            show_buf(nm, ZSTD_decompress(dec, decCap, cmp, cs), dec);
            ZSTD_DCtx_setParameter(d, ZSTD_d_forceIgnoreChecksum, ZSTD_d_ignoreChecksum);
            pat(dec, decCap);
            snprintf(nm, sizeof nm, "cksum-corrupt[%d] ignore", i);
            show_buf(nm, ZSTD_decompressDCtx(d, dec, decCap, cmp, cs), dec);
            cmp[cs - 1 - i] ^= 0x80;
            ZSTD_freeDCtx(d);
        }
    }
    /* decoder params */
    {   size_t cs = ZSTD_compress(cmp, cmpCap, src, 50000, 5);
        int wl, mb, ha;
        for (wl = 0; wl <= 31; wl++) {
            ZSTD_DCtx* d = ZSTD_createDCtx();
            char nm[64];
            int got = -1;
            show_rc("  set windowLogMax", ZSTD_DCtx_setParameter(d, ZSTD_d_windowLogMax, wl));
            ZSTD_DCtx_getParameter(d, ZSTD_d_windowLogMax, &got);
            snprintf(nm, sizeof nm, "windowLogMax[%d] got=%d dec", wl, got);
            pat(dec, decCap);
            show_buf(nm, ZSTD_decompressDCtx(d, dec, decCap, cmp, cs), dec);
            ZSTD_freeDCtx(d);
        }
        for (mb = 0; mb < 6; mb++) {
            static const int vals[] = {0, 1, 1023, 1024, 131072, 131073};
            ZSTD_DCtx* d = ZSTD_createDCtx();
            char nm[64];
            show_rc("  set d_maxBlockSize", ZSTD_DCtx_setParameter(d, ZSTD_d_maxBlockSize, vals[mb]));
            snprintf(nm, sizeof nm, "d_maxBlockSize[%d] dec", vals[mb]);
            pat(dec, decCap);
            show_buf(nm, ZSTD_decompressDCtx(d, dec, decCap, cmp, cs), dec);
            ZSTD_freeDCtx(d);
        }
        for (ha = 0; ha <= 1; ha++) {
            ZSTD_DCtx* d = ZSTD_createDCtx();
            char nm[64];
            show_rc("  set disableHufAsm", ZSTD_DCtx_setParameter(d, ZSTD_d_disableHuffmanAssembly, ha));
            snprintf(nm, sizeof nm, "disableHufAsm[%d] dec", ha);
            pat(dec, decCap);
            show_buf(nm, ZSTD_decompressDCtx(d, dec, decCap, cmp, cs), dec);
            ZSTD_freeDCtx(d);
        }
        SHOWZ("decodingBufferSize_min(1<<20,50000)", ZSTD_decodingBufferSize_min(1u<<20, 50000));
        SHOWZ("decodingBufferSize_min(1<<27,unknown)", ZSTD_decodingBufferSize_min(1u<<27, ZSTD_CONTENTSIZE_UNKNOWN));
        SHOWZ("estimateDStreamSize_fromFrame", ZSTD_estimateDStreamSize_fromFrame(cmp, cs));
        SHOWZ("estimateDStreamSize(1<<20)", ZSTD_estimateDStreamSize(1u<<20));
    }
}

/* ================================================================== */
/* PHASE 9 : deprecated ZSTD_* stream wrappers                         */
/* ================================================================== */
static void phase_deprecated(void) {
    size_t n = 120000;
    int k;
    BANNER("deprecated wrappers");
    rs(0xDEADULL);
    fill_text(src, n);
    for (k = 0; k < 7; k++) {
        ZSTD_CStream* zcs = ZSTD_createCStream();
        size_t total = 0, pos = 0, rc = 0;
        char nm[80];
        ZSTD_parameters params = ZSTD_getParams(6, n, 0);
        ZSTD_CDict* cd = ZSTD_createCDict(traindict, traindict_n, 7);
        switch (k) {
        case 0: rc = ZSTD_initCStream(zcs, 4); break;
        case 1: rc = ZSTD_initCStream_srcSize(zcs, 4, n); break;
        case 2: rc = ZSTD_initCStream_usingDict(zcs, traindict, traindict_n, 4); break;
        case 3: rc = ZSTD_initCStream_advanced(zcs, traindict, traindict_n, params, n); break;
        case 4: rc = ZSTD_initCStream_usingCDict(zcs, cd); break;
        case 5: { ZSTD_frameParameters fp = params.fParams;
                  rc = ZSTD_initCStream_usingCDict_advanced(zcs, cd, fp, n); } break;
        default: rc = ZSTD_initCStream(zcs, 4);
                 rc = ZSTD_resetCStream(zcs, n); break;
        }
        snprintf(nm, sizeof nm, "init[%d]", k);
        show_rc(nm, rc);
        pat(cmp, cmpCap);
        while (pos < n) {
            ZSTD_inBuffer in; ZSTD_outBuffer out;
            size_t take = 7000;
            if (take > n - pos) take = n - pos;
            in.src = src + pos; in.size = take; in.pos = 0;
            out.dst = cmp + total; out.size = cmpCap - total; out.pos = 0;
            ZSTD_compressStream(zcs, &out, &in);
            total += out.pos; pos += in.pos;
            { ZSTD_outBuffer o2; o2.dst = cmp + total; o2.size = cmpCap - total; o2.pos = 0;
              ZSTD_flushStream(zcs, &o2); total += o2.pos; }
        }
        for (;;) {
            ZSTD_outBuffer out;
            size_t rem;
            out.dst = cmp + total; out.size = cmpCap - total; out.pos = 0;
            rem = ZSTD_endStream(zcs, &out);
            total += out.pos;
            if (rem == 0 || ZSTD_isError(rem)) break;
        }
        snprintf(nm, sizeof nm, "depr-stream[%d]", k);
        SHOW(nm, cmp, total);
        SHOWZ("  sizeof_CStream", ZSTD_sizeof_CStream(zcs));
        {   ZSTD_DStream* zds = ZSTD_createDStream();
            size_t dpos = 0, cpos = 0;
            ZSTD_DDict* dd = ZSTD_createDDict(traindict, traindict_n);
            if (k == 2 || k == 3) ZSTD_initDStream_usingDict(zds, traindict, traindict_n);
            else if (k == 4 || k == 5) ZSTD_initDStream_usingDDict(zds, dd);
            else { ZSTD_initDStream(zds); ZSTD_resetDStream(zds); }
            pat(dec, decCap);
            while (cpos < total) {
                ZSTD_inBuffer in; ZSTD_outBuffer out;
                size_t take = 5000, r;
                if (take > total - cpos) take = total - cpos;
                in.src = cmp + cpos; in.size = take; in.pos = 0;
                out.dst = dec + dpos; out.size = decCap - dpos; out.pos = 0;
                r = ZSTD_decompressStream(zds, &out, &in);
                if (ZSTD_isError(r)) { P("  dstream ERR %s\n", ZSTD_getErrorName(r)); break; }
                dpos += out.pos; cpos += in.pos;
            }
            SHOWU("  dstream produced", dpos);
            SHOWU("  dstream matches", (dpos == n) && (memcmp(dec, src, n) == 0));
            SHOWZ("  sizeof_DStream", ZSTD_sizeof_DStream(zds));
            ZSTD_freeDDict(dd);
            ZSTD_freeDStream(zds);
        }
        {   ZSTD_frameProgression fp = ZSTD_getFrameProgression(zcs);
            P("  progression %llu %llu %llu %llu %u %u\n", fp.ingested, fp.consumed,
              fp.produced, fp.flushed, fp.currentJobID, fp.nbActiveWorkers);
            SHOWZ("  toFlushNow", ZSTD_toFlushNow(zcs));
        }
        ZSTD_freeCDict(cd);
        ZSTD_freeCStream(zcs);
    }
    /* compress_advanced / advanced_internal / usingCDict_advanced */
    {   ZSTD_CCtx* c = ZSTD_createCCtx();
        ZSTD_parameters params = ZSTD_getParams(8, n, traindict_n);
        pat(cmp, cmpCap);
        show_buf("compress_advanced", ZSTD_compress_advanced(c, cmp, cmpCap, src, n,
                                        traindict, traindict_n, params), cmp);
        {   ZSTD_CCtx_params* p = ZSTD_createCCtxParams();
            ZSTD_CCtxParams_init_advanced(p, params);
            pat(cmp, cmpCap);
            show_buf("compress_advanced_internal",
                     ZSTD_compress_advanced_internal(c, cmp, cmpCap, src, n,
                                                     traindict, traindict_n, p), cmp);
            ZSTD_freeCCtxParams(p);
        }
        {   ZSTD_CDict* cd = ZSTD_createCDict(traindict, traindict_n, 8);
            pat(cmp, cmpCap);
            show_buf("compress_usingCDict_advanced",
                     ZSTD_compress_usingCDict_advanced(c, cmp, cmpCap, src, n, cd, params.fParams), cmp);
            show_rc("compressBegin_usingCDict_deprecated",
                    ZSTD_compressBegin_usingCDict_deprecated(c, cd));
            ZSTD_freeCDict(cd);
        }
        ZSTD_freeCCtx(c);
    }
}

/* ================================================================== */
/* PHASE 10 : deprecated ZBUFF API                                     */
/* ================================================================== */
static void phase_zbuff(void) {
    size_t n = 90000;
    int k;
    BANNER("ZBUFF");
    SHOWZ("recommendedCInSize", ZBUFF_recommendedCInSize());
    SHOWZ("recommendedCOutSize", ZBUFF_recommendedCOutSize());
    SHOWZ("recommendedDInSize", ZBUFF_recommendedDInSize());
    SHOWZ("recommendedDOutSize", ZBUFF_recommendedDOutSize());
    SHOWZ("isError(0)", ZBUFF_isError(0));
    SHOWZ("isError(-1)", ZBUFF_isError((size_t)-1));
    P("getErrorName(-72) %s\n", ZBUFF_getErrorName((size_t)-72));
    rs(0xB0FFULL);
    fill_text(src, n);
    for (k = 0; k < 4; k++) {
        ZBUFF_CCtx* zc = (k & 1) ? ZBUFF_createCCtx_advanced(g_nullCMem) : ZBUFF_createCCtx();
        size_t total = 0, pos = 0;
        char nm[64];
        ZSTD_parameters params = ZSTD_getParams(5, n, 0);
        if (k == 0)      show_rc("ZBUFF_compressInit", ZBUFF_compressInit(zc, 5));
        else if (k == 1) show_rc("ZBUFF_compressInitDictionary",
                                 ZBUFF_compressInitDictionary(zc, traindict, traindict_n, 5));
        else if (k == 2) show_rc("ZBUFF_compressInit_advanced",
                                 ZBUFF_compressInit_advanced(zc, traindict, traindict_n, params, n));
        else             show_rc("ZBUFF_compressInit(19)", ZBUFF_compressInit(zc, 19));
        pat(cmp, cmpCap);
        while (pos < n) {
            size_t srcSize = 6000;
            size_t dstSize = cmpCap - total;
            size_t rc;
            if (srcSize > n - pos) srcSize = n - pos;
            rc = ZBUFF_compressContinue(zc, cmp + total, &dstSize, src + pos, &srcSize);
            if (ZBUFF_isError(rc)) { P("  cc ERR %s\n", ZBUFF_getErrorName(rc)); break; }
            total += dstSize; pos += srcSize;
            {   size_t fd = cmpCap - total;
                size_t fr = ZBUFF_compressFlush(zc, cmp + total, &fd);
                if (ZBUFF_isError(fr)) { P("  flush ERR %s\n", ZBUFF_getErrorName(fr)); break; }
                total += fd;
            }
        }
        for (;;) {
            size_t d = cmpCap - total;
            size_t rc = ZBUFF_compressEnd(zc, cmp + total, &d);
            total += d;
            if (rc == 0 || ZBUFF_isError(rc)) { if (ZBUFF_isError(rc)) P("  end ERR %s\n", ZBUFF_getErrorName(rc)); break; }
        }
        snprintf(nm, sizeof nm, "zbuff-compress[%d]", k);
        SHOW(nm, cmp, total);
        ZBUFF_freeCCtx(zc);
        /* decompress with ZBUFF */
        {   ZBUFF_DCtx* zd = (k & 1) ? ZBUFF_createDCtx_advanced(g_nullCMem) : ZBUFF_createDCtx();
            size_t dpos = 0, cpos = 0;
            if (k == 1 || k == 2) show_rc("  ZBUFF_decompressInitDictionary",
                                          ZBUFF_decompressInitDictionary(zd, traindict, traindict_n));
            else                  show_rc("  ZBUFF_decompressInit", ZBUFF_decompressInit(zd));
            pat(dec, decCap);
            while (cpos < total) {
                size_t inSize = 4000;
                size_t outSize = decCap - dpos;
                size_t rc;
                if (inSize > total - cpos) inSize = total - cpos;
                rc = ZBUFF_decompressContinue(zd, dec + dpos, &outSize, cmp + cpos, &inSize);
                if (ZBUFF_isError(rc)) { P("  dc ERR %s\n", ZBUFF_getErrorName(rc)); break; }
                dpos += outSize; cpos += inSize;
                if (rc == 0 && cpos >= total) break;
            }
            SHOWU("  zbuff produced", dpos);
            SHOWU("  zbuff matches", (dpos == n) && (memcmp(dec, src, n) == 0));
            ZBUFF_freeDCtx(zd);
        }
    }
}

/* ================================================================== */
/* PHASE 11 : ZSTDMT (multithreading compiled out)                     */
/* ================================================================== */
static void phase_zstdmt(void) {
    ZSTD_customMem cm; ZSTDMT_CCtx* m;
    BANNER("ZSTDMT");
    memset(&cm, 0, sizeof cm);
    m = ZSTDMT_createCCtx_advanced(1, cm, NULL);
    SHOWP("ZSTDMT_createCCtx_advanced(1)", m);
    SHOWZ("ZSTDMT_freeCCtx(that)", ZSTDMT_freeCCtx(m));
    SHOWZ("ZSTDMT_freeCCtx(NULL)", ZSTDMT_freeCCtx(NULL));
    m = ZSTDMT_createCCtx_advanced(0, cm, NULL);
    SHOWP("ZSTDMT_createCCtx_advanced(0)", m);
    SHOWZ("ZSTDMT_freeCCtx", ZSTDMT_freeCCtx(m));
}

/* ================================================================== */
/* PHASE 12 : COVER internals                                          */
/* ================================================================== */
static void phase_cover(void) {
    BANNER("COVER internals");
    {   size_t freqs[8] = {1,2,3,4,5,6,7,8};
        SHOWU("COVER_sum", COVER_sum((const size_t*)freqs, 8));
        SHOWU("COVER_sum(0)", COVER_sum((const size_t*)freqs, 0));
    }
    {   unsigned i, j;
        for (i = 0; i < 6; i++) {
            for (j = 1; j <= 5; j++) {
                COVER_epoch_info_t e = COVER_computeEpochs(1u << (10 + i), 1000u * j, 200, 4);
                P("epochs[%u,%u] num=%u size=%u\n", i, j, e.num, e.size);
            }
        }
    }
    {   ZDICT_cover_params_t p;
        memset(&p, 0, sizeof p);
        p.k = 200; p.d = 8; p.steps = 2; p.nbThreads = 1; p.zParams.compressionLevel = 3;
        COVER_warnOnSmallCorpus(1000, 100, 0);
        COVER_warnOnSmallCorpus(1000000, 100, 0);
        P("warnOnSmallCorpus done\n"); g_calls++;
    }
    {   COVER_best_t best;
        COVER_best_init(&best);
        SHOWU("best.dictSize after init", best.dictSize);
        SHOWU("best.compressedSize after init", best.compressedSize);
        COVER_best_start(&best);
        SHOWU("best.liveJobs after start", best.liveJobs);
        /* NOTE: COVER_best_wait() spins on `while (liveJobs) cond_wait()`, and with
         * ZSTD_MULTITHREAD compiled out cond_wait is a no-op, so waiting with a live
         * job hangs forever in BOTH libraries. Retire the job first. */
        {   ZDICT_cover_params_t cp;
            COVER_dictSelection_t sel;
            unsigned char dbuf[64];
            memset(&cp, 0, sizeof cp);
            cp.k = 64; cp.d = 8; cp.zParams.compressionLevel = 3;
            memset(dbuf, 0x5A, sizeof dbuf);
            sel.dictContent = dbuf; sel.dictSize = sizeof dbuf; sel.totalCompressedSize = 12345;
            COVER_best_finish(&best, cp, sel);
        }
        SHOWU("best.liveJobs after finish", best.liveJobs);
        SHOWU("best.dictSize after finish", best.dictSize);
        SHOWU("best.compressedSize after finish", best.compressedSize);
        SHOW("best.dict after finish", best.dict ? best.dict : (void*)"", best.dict ? best.dictSize : 0);
        COVER_best_wait(&best);
        COVER_best_destroy(&best);
        P("best lifecycle done\n"); g_calls++;
    }
    {   COVER_dictSelection_t s;
        SHOWU("dictSelectionError(5).totalCompressedSize", COVER_dictSelectionError(5).totalCompressedSize);
        s = COVER_dictSelectionError(5);
        SHOWU("dictSelectionIsError(err)", COVER_dictSelectionIsError(s));
        COVER_dictSelectionFree(s);
        P("dictSelection done\n"); g_calls++;
    }
    {   /* COVER_checkTotalCompressedSize + COVER_selectDict on real data */
        ZDICT_cover_params_t p;
        size_t sizes[16];
        size_t off = 0, i;
        unsigned char* samples = aux;
        memset(&p, 0, sizeof p);
        p.k = 64; p.d = 8; p.steps = 1; p.nbThreads = 1; p.zParams.compressionLevel = 3;
        rs(0xC0FEULL);
        for (i = 0; i < 16; i++) { sizes[i] = 1024; fill_text(samples + off, 1024); off += 1024; }
        {   size_t* offsets = (size_t*)malloc(17 * sizeof(size_t));
            size_t total = 0;
            for (i = 0; i <= 16; i++) { offsets[i] = total; if (i < 16) total += sizes[i]; }
            unsigned char dbuf[2048];
            size_t dn;
            memcpy(dbuf, samples, sizeof dbuf);
            dn = ZDICT_finalizeDictionary(dbuf, sizeof dbuf, samples, 1024, samples, sizes, 16, p.zParams);
            if (!ZDICT_isError(dn))
                SHOWU("COVER_checkTotalCompressedSize",
                      COVER_checkTotalCompressedSize(p, sizes, samples, offsets, 8, 16, dbuf, dn));
            free(offsets);
        }
    }
}

/* ================================================================== */
/* PHASE 13 : ZDICT extras                                             */
/* ================================================================== */
static void phase_zdict(void) {
    size_t sizes[64];
    unsigned char* samples = aux;
    size_t off = 0, i;
    unsigned char* out = (unsigned char*)malloc(128 * 1024);
    BANNER("ZDICT");
    rs(0x2D1C7ULL);
    for (i = 0; i < 64; i++) { sizes[i] = 1500 + (i * 13) % 700; fill_text(samples + off, sizes[i]); off += sizes[i]; }
    {   int lvl;
        for (lvl = 1; lvl <= 19; lvl += 6) {
            ZDICT_params_t p;
            char nm[64];
            memset(&p, 0, sizeof p);
            p.compressionLevel = lvl;
            pat(out, 65536);
            snprintf(nm, sizeof nm, "addEntropyTablesFromBuffer[L%d]", lvl);
            memcpy(out, samples, 4096);
            show_buf(nm, ZDICT_addEntropyTablesFromBuffer(out, 4096, 65536, samples, sizes, 64), out);
            pat(out, 65536);
            snprintf(nm, sizeof nm, "finalizeDictionary[L%d]", lvl);
            show_buf(nm, ZDICT_finalizeDictionary(out, 65536, samples, 6000, samples, sizes, 64, p), out);
            if (!ZDICT_isError(*(size_t*)&out[0])) { }
        }
    }
    {   unsigned sel;
        for (sel = 1; sel <= 12; sel += 3) {
            ZDICT_legacy_params_t lp;
            char nm[64];
            memset(&lp, 0, sizeof lp);
            lp.selectivityLevel = sel;
            lp.zParams.compressionLevel = 3;
            pat(out, 65536);
            snprintf(nm, sizeof nm, "trainFromBuffer_legacy[sel%u]", sel);
            show_buf(nm, ZDICT_trainFromBuffer_legacy(out, 32768, samples, sizes, 64, lp), out);
        }
    }
    {   unsigned k, d;
        for (d = 6; d <= 8; d += 2) {
            for (k = 50; k <= 250; k += 100) {
                ZDICT_cover_params_t cp;
                ZDICT_fastCover_params_t fp;
                char nm[80];
                memset(&cp, 0, sizeof cp);
                cp.k = k; cp.d = d; cp.steps = 2; cp.nbThreads = 1; cp.splitPoint = 0;
                cp.zParams.compressionLevel = 5;
                pat(out, 65536);
                snprintf(nm, sizeof nm, "cover[k%u,d%u]", k, d);
                show_buf(nm, ZDICT_trainFromBuffer_cover(out, 16384, samples, sizes, 64, cp), out);
                memset(&fp, 0, sizeof fp);
                fp.k = k; fp.d = d; fp.f = 20; fp.steps = 2; fp.nbThreads = 1;
                fp.splitPoint = 0; fp.accel = 1; fp.zParams.compressionLevel = 5;
                pat(out, 65536);
                snprintf(nm, sizeof nm, "fastcover[k%u,d%u]", k, d);
                show_buf(nm, ZDICT_trainFromBuffer_fastCover(out, 16384, samples, sizes, 64, fp), out);
            }
        }
    }
    {   int nbt, acc;
        for (nbt = 1; nbt <= 3; nbt++) {
            ZDICT_cover_params_t cp;
            char nm[80];
            memset(&cp, 0, sizeof cp);
            cp.steps = 2; cp.nbThreads = (unsigned)nbt; cp.splitPoint = 0;
            cp.zParams.compressionLevel = 3;
            pat(out, 65536);
            snprintf(nm, sizeof nm, "optcover[nbt%d]", nbt);
            show_buf(nm, ZDICT_optimizeTrainFromBuffer_cover(out, 16384, samples, sizes, 64, &cp), out);
            P("  -> k=%u d=%u steps=%u split=%f\n", cp.k, cp.d, cp.steps, cp.splitPoint);
        }
        for (acc = 0; acc <= 10; acc += 5) {
            ZDICT_fastCover_params_t fp;
            char nm[80];
            memset(&fp, 0, sizeof fp);
            fp.steps = 2; fp.nbThreads = 1; fp.accel = (unsigned)acc;
            fp.zParams.compressionLevel = 3;
            pat(out, 65536);
            snprintf(nm, sizeof nm, "optfastcover[accel%d]", acc);
            show_buf(nm, ZDICT_optimizeTrainFromBuffer_fastCover(out, 16384, samples, sizes, 64, &fp), out);
            P("  -> k=%u d=%u f=%u steps=%u\n", fp.k, fp.d, fp.f, fp.steps);
        }
    }
    {   size_t ds = ZDICT_trainFromBuffer(out, 16384, samples, sizes, 64);
        show_buf("trainFromBuffer", ds, out);
        if (!ZDICT_isError(ds)) {
            SHOWU("  getDictID", ZDICT_getDictID(out, ds));
            SHOWZ("  getDictHeaderSize", ZDICT_getDictHeaderSize(out, ds));
        }
        SHOWU("getDictID(garbage)", ZDICT_getDictID(samples, 100));
        SHOWZ("getDictHeaderSize(garbage)", ZDICT_getDictHeaderSize(samples, 100));
        SHOWZ("ZDICT_isError(0)", ZDICT_isError(0));
        P("ZDICT_getErrorName(-30) %s\n", ZDICT_getErrorName((size_t)-30));
    }
    free(out);
}

/* ================================================================== */
/* PHASE 14 : misc grids                                               */
/* ================================================================== */
static void phase_grids(void) {
    int lvl;
    unsigned long long sz;
    size_t ds;
    BANNER("grids");
    for (lvl = -30; lvl <= 25; lvl++) {
        for (sz = 0; sz <= 4000000ULL; sz = sz ? sz * 19 : 1) {
            for (ds = 0; ds <= 100000; ds = ds ? ds * 23 : 1) {
                ZSTD_compressionParameters cp = ZSTD_getCParams(lvl, sz, ds);
                ZSTD_parameters pp = ZSTD_getParams(lvl, sz, ds);
                ZSTD_compressionParameters ad = ZSTD_adjustCParams(cp, sz, ds);
                P("g[%d,%llu,%zu] cp=%u,%u,%u,%u,%u,%u,%d p=%d,%d,%d ad=%u,%u,%u,%u,%u,%u,%d ck=%lld cl=%u\n",
                  lvl, sz, ds,
                  cp.windowLog, cp.chainLog, cp.hashLog, cp.searchLog, cp.minMatch, cp.targetLength, (int)cp.strategy,
                  pp.fParams.contentSizeFlag, pp.fParams.checksumFlag, pp.fParams.noDictIDFlag,
                  ad.windowLog, ad.chainLog, ad.hashLog, ad.searchLog, ad.minMatch, ad.targetLength, (int)ad.strategy,
                  (long long)ZSTD_checkCParams(cp),
                  ZSTD_cycleLog(cp.chainLog, (int)cp.strategy));
            }
        }
    }
    {   unsigned hl; int st;
        for (hl = 1; hl <= 31; hl++)
            for (st = 1; st <= 9; st++)
                P("cycleLog[%u,%d]=%u\n", hl, st, ZSTD_cycleLog(hl, st));
    }
    {   ZSTD_CCtx* c = ZSTD_createCCtx();
        ZSTD_compressionParameters cp = ZSTD_getCParams(7, 100000, 0);
        ZSTD_frameParameters fp; ZSTD_parameters pp;
        memset(&fp, 0, sizeof fp); fp.checksumFlag = 1; fp.contentSizeFlag = 1;
        pp = ZSTD_getParams(7, 100000, 0);
        show_rc("CCtx_setCParams", ZSTD_CCtx_setCParams(c, cp));
        show_rc("CCtx_setFParams", ZSTD_CCtx_setFParams(c, fp));
        show_rc("CCtx_setParams", ZSTD_CCtx_setParams(c, pp));
        show_rc("CCtx_refThreadPool(NULL)", ZSTD_CCtx_refThreadPool(c, NULL));
        ZSTD_CCtx_trace(c, 0);
        show_rc("reset session_only", ZSTD_CCtx_reset(c, ZSTD_reset_session_only));
        show_rc("reset parameters", ZSTD_CCtx_reset(c, ZSTD_reset_parameters));
        show_rc("reset both", ZSTD_CCtx_reset(c, ZSTD_reset_session_and_parameters));
        show_rc("reset bogus", ZSTD_CCtx_reset(c, (ZSTD_ResetDirective)99));
        rs(0x9911ULL); fill_text(src, 100000);
        pat(cmp, cmpCap);
        show_buf("after grid compress2", ZSTD_compress2(c, cmp, cmpCap, src, 100000), cmp);
        SHOWZ("sizeof_CCtx", ZSTD_sizeof_CCtx(c));
        ZSTD_freeCCtx(c);
    }
    {   ZSTD_DCtx* d = ZSTD_createDCtx();
        show_rc("DCtx reset session_only", ZSTD_DCtx_reset(d, ZSTD_reset_session_only));
        show_rc("DCtx reset parameters", ZSTD_DCtx_reset(d, ZSTD_reset_parameters));
        show_rc("DCtx reset both", ZSTD_DCtx_reset(d, ZSTD_reset_session_and_parameters));
        show_rc("DCtx reset bogus", ZSTD_DCtx_reset(d, (ZSTD_ResetDirective)77));
        SHOWZ("sizeof_DCtx", ZSTD_sizeof_DCtx(d));
        ZSTD_freeDCtx(d);
    }
}

typedef struct { const char* name; void (*fn)(void); } phase_t;
static const phase_t kPhases[] = {
    {"bounds", phase_bounds}, {"cparams", phase_cparams}, {"cctxparams", phase_cctxparams},
    {"magicless", phase_magicless}, {"dict", phase_dict}, {"static", phase_static},
    {"bufferless", phase_bufferless}, {"sequences", phase_sequences},
    {"frames", phase_frames}, {"deprecated", phase_deprecated}, {"zbuff", phase_zbuff},
    {"zstdmt", phase_zstdmt}, {"cover", phase_cover}, {"zdict", phase_zdict},
    {"grids", phase_grids},
};
#define NPHASE ((int)(sizeof(kPhases)/sizeof(kPhases[0])))

int main(int argc, char** argv) {
    int i;
    cmpCap = ZSTD_compressBound(MAXSRC) + 65536;
    decCap = MAXSRC + 65536;
    src = (unsigned char*)malloc(MAXSRC);
    cmp = (unsigned char*)malloc(cmpCap);
    dec = (unsigned char*)malloc(decCap);
    aux = (unsigned char*)malloc(MAXSRC);
    memset(src, 0, MAXSRC); memset(aux, 0, MAXSRC);
    setvbuf(stdout, NULL, _IOLBF, 1 << 16);
    if (argc > 1 && strcmp(argv[1], "--list") == 0) {
        for (i = 0; i < NPHASE; i++) printf("%s\n", kPhases[i].name);
        return 0;
    }
    build_corpus();
    build_dicts();
    if (argc > 1) {
        for (i = 0; i < NPHASE; i++)
            if (strcmp(argv[1], kPhases[i].name) == 0) { kPhases[i].fn(); break; }
        if (i == NPHASE) { fprintf(stderr, "unknown phase %s\n", argv[1]); return 2; }
    } else {
        for (i = 0; i < NPHASE; i++) kPhases[i].fn();
    }
    printf("\n########## total observations: %llu ##########\n", g_calls);
    printf("=== done ===\n");
    return 0;
}
