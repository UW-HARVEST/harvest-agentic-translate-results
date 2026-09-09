/* verify/entropy.c
 *
 * Differential harness for the LOW-LEVEL zstd entropy / hashing / suffix-sort
 * API (FSE, HUF, HIST, ZSTD_XXH*, divsufsort/divbwt and a few misc internals).
 *
 * All prototypes are hand written `extern` declarations so that every call is a
 * real PLT call into the linked libzstd.so (no inline code is pulled in from the
 * private headers).  Build once against the C library and once against the Rust
 * library and diff the two traces.
 *
 * Determinism rules observed here:
 *   - no time(), no getenv(), no addresses ever printed
 *   - every output buffer is malloc'ed once and memset to a fixed pattern
 *     immediately before each call, so "untouched" bytes hash identically
 *   - every workspace is memset to a fixed pattern before each call
 *   - the PRNG is a fixed-seed xorshift64, reseeded from a pure function of the
 *     test parameters before each corpus generation, so any single line can be
 *     reproduced in isolation
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* ===================================================================== */
/*  hand written prototypes                                              */
/* ===================================================================== */

typedef unsigned char      U8;
typedef unsigned short     U16;
typedef unsigned int       U32;
typedef unsigned long long U64;

/* ---- FSE ---- */
typedef unsigned FSE_CTable;
typedef unsigned FSE_DTable;

extern unsigned    FSE_versionNumber(void);
extern unsigned    FSE_isError(size_t code);
extern const char* FSE_getErrorName(size_t code);
extern size_t      FSE_compressBound(size_t size);
extern unsigned    FSE_optimalTableLog(unsigned maxTableLog, size_t srcSize, unsigned maxSymbolValue);
extern unsigned    FSE_optimalTableLog_internal(unsigned maxTableLog, size_t srcSize, unsigned maxSymbolValue, unsigned minus);
extern size_t      FSE_normalizeCount(short* normalizedCounter, unsigned tableLog,
                                      const unsigned* count, size_t srcSize,
                                      unsigned maxSymbolValue, unsigned useLowProbCount);
extern size_t      FSE_NCountWriteBound(unsigned maxSymbolValue, unsigned tableLog);
extern size_t      FSE_writeNCount(void* buffer, size_t bufferSize, const short* normalizedCounter,
                                   unsigned maxSymbolValue, unsigned tableLog);
extern size_t      FSE_readNCount(short* normalizedCounter, unsigned* maxSymbolValuePtr,
                                  unsigned* tableLogPtr, const void* rBuffer, size_t rBuffSize);
extern size_t      FSE_readNCount_bmi2(short* normalizedCounter, unsigned* maxSymbolValuePtr,
                                       unsigned* tableLogPtr, const void* rBuffer, size_t rBuffSize,
                                       int bmi2);
extern size_t      FSE_buildCTable_wksp(FSE_CTable* ct, const short* normalizedCounter,
                                        unsigned maxSymbolValue, unsigned tableLog,
                                        void* workSpace, size_t wkspSize);
extern size_t      FSE_buildCTable_rle(FSE_CTable* ct, unsigned char symbolValue);
extern size_t      FSE_compress_usingCTable(void* dst, size_t dstCapacity, const void* src,
                                            size_t srcSize, const FSE_CTable* ct);
extern size_t      FSE_buildDTable_wksp(FSE_DTable* dt, const short* normalizedCounter,
                                        unsigned maxSymbolValue, unsigned tableLog,
                                        void* workSpace, size_t wkspSize);
extern size_t      FSE_decompress_wksp_bmi2(void* dst, size_t dstCapacity, const void* cSrc,
                                            size_t cSrcSize, unsigned maxLog, void* workSpace,
                                            size_t wkspSize, int bmi2);

#define FSE_NCOUNTBOUND 512
#define FSE_CTABLE_SIZE_U32(maxTableLog, maxSymbolValue)  (1 + (1<<((maxTableLog)-1)) + (((maxSymbolValue)+1)*2))
#define FSE_DTABLE_SIZE_U32(maxTableLog)                  (1 + (1<<(maxTableLog)))
#define FSE_BUILD_CTABLE_WORKSPACE_SIZE_U32(msv, tl)      ((((msv) + 2) + (1ull << (tl)))/2 + 2)
#define FSE_BUILD_DTABLE_WKSP_SIZE(tl, msv)               (sizeof(short) * ((msv) + 1) + (1ULL << (tl)) + 8)
#define FSE_BUILD_DTABLE_WKSP_SIZE_U32(tl, msv)           ((FSE_BUILD_DTABLE_WKSP_SIZE(tl,msv) + sizeof(unsigned) - 1) / sizeof(unsigned))
#define FSE_DECOMPRESS_WKSP_SIZE_U32(tl, msv)             (FSE_DTABLE_SIZE_U32(tl) + 1 + FSE_BUILD_DTABLE_WKSP_SIZE_U32(tl, msv) + (255 + 1) / 2 + 1)
#define FSE_DECOMPRESS_WKSP_SIZE(tl, msv)                 (FSE_DECOMPRESS_WKSP_SIZE_U32(tl, msv) * sizeof(unsigned))

/* ---- HUF ---- */
typedef size_t   HUF_CElt;
typedef unsigned HUF_DTable;
typedef struct { U8 tableLog; U8 maxSymbolValue; U8 unused[sizeof(size_t) - 2]; } HUF_CTableHeader;

extern unsigned    HUF_isError(size_t code);
extern const char* HUF_getErrorName(size_t code);
extern size_t      HUF_compressBound(size_t size);
extern unsigned    HUF_minTableLog(unsigned symbolCardinality);
extern unsigned    HUF_cardinality(const unsigned* count, unsigned maxSymbolValue);
extern unsigned    HUF_optimalTableLog(unsigned maxTableLog, size_t srcSize, unsigned maxSymbolValue,
                                       void* workSpace, size_t wkspSize, HUF_CElt* table,
                                       const unsigned* count, int flags);
extern size_t      HUF_buildCTable_wksp(HUF_CElt* tree, const unsigned* count, U32 maxSymbolValue,
                                        U32 maxNbBits, void* workSpace, size_t wkspSize);
extern size_t      HUF_writeCTable_wksp(void* dst, size_t maxDstSize, const HUF_CElt* CTable,
                                        unsigned maxSymbolValue, unsigned huffLog,
                                        void* workspace, size_t workspaceSize);
extern size_t      HUF_readCTable(HUF_CElt* CTable, unsigned* maxSymbolValuePtr, const void* src,
                                  size_t srcSize, unsigned* hasZeroWeights);
extern U32         HUF_getNbBitsFromCTable(const HUF_CElt* symbolTable, U32 symbolValue);
extern HUF_CTableHeader HUF_readCTableHeader(const HUF_CElt* ctable);
extern size_t      HUF_estimateCompressedSize(const HUF_CElt* CTable, const unsigned* count,
                                              unsigned maxSymbolValue);
extern int         HUF_validateCTable(const HUF_CElt* CTable, const unsigned* count,
                                      unsigned maxSymbolValue);
extern size_t      HUF_compress1X_usingCTable(void* dst, size_t dstSize, const void* src,
                                              size_t srcSize, const HUF_CElt* CTable, int flags);
extern size_t      HUF_compress4X_usingCTable(void* dst, size_t dstSize, const void* src,
                                              size_t srcSize, const HUF_CElt* CTable, int flags);
extern size_t      HUF_compress1X_repeat(void* dst, size_t dstSize, const void* src, size_t srcSize,
                                         unsigned maxSymbolValue, unsigned tableLog,
                                         void* workSpace, size_t wkspSize,
                                         HUF_CElt* hufTable, int* repeat, int flags);
extern size_t      HUF_compress4X_repeat(void* dst, size_t dstSize, const void* src, size_t srcSize,
                                         unsigned maxSymbolValue, unsigned tableLog,
                                         void* workSpace, size_t wkspSize,
                                         HUF_CElt* hufTable, int* repeat, int flags);
extern size_t      HUF_readStats(U8* huffWeight, size_t hwSize, U32* rankStats, U32* nbSymbolsPtr,
                                 U32* tableLogPtr, const void* src, size_t srcSize);
extern size_t      HUF_readStats_wksp(U8* huffWeight, size_t hwSize, U32* rankStats,
                                      U32* nbSymbolsPtr, U32* tableLogPtr, const void* src,
                                      size_t srcSize, void* workspace, size_t wkspSize, int flags);
extern U32         HUF_selectDecoder(size_t dstSize, size_t cSrcSize);
extern size_t      HUF_readDTableX1_wksp(HUF_DTable* DTable, const void* src, size_t srcSize,
                                         void* workSpace, size_t wkspSize, int flags);
extern size_t      HUF_readDTableX2_wksp(HUF_DTable* DTable, const void* src, size_t srcSize,
                                         void* workSpace, size_t wkspSize, int flags);
extern size_t      HUF_decompress1X_usingDTable(void* dst, size_t maxDstSize, const void* cSrc,
                                                size_t cSrcSize, const HUF_DTable* DTable, int flags);
extern size_t      HUF_decompress4X_usingDTable(void* dst, size_t maxDstSize, const void* cSrc,
                                                size_t cSrcSize, const HUF_DTable* DTable, int flags);
extern size_t      HUF_decompress1X1_DCtx_wksp(HUF_DTable* dctx, void* dst, size_t dstSize,
                                               const void* cSrc, size_t cSrcSize, void* workSpace,
                                               size_t wkspSize, int flags);
extern size_t      HUF_decompress1X2_DCtx_wksp(HUF_DTable* dctx, void* dst, size_t dstSize,
                                               const void* cSrc, size_t cSrcSize, void* workSpace,
                                               size_t wkspSize, int flags);
extern size_t      HUF_decompress1X_DCtx_wksp(HUF_DTable* dctx, void* dst, size_t dstSize,
                                              const void* cSrc, size_t cSrcSize, void* workSpace,
                                              size_t wkspSize, int flags);
extern size_t      HUF_decompress4X_hufOnly_wksp(HUF_DTable* dctx, void* dst, size_t dstSize,
                                                 const void* cSrc, size_t cSrcSize, void* workSpace,
                                                 size_t wkspSize, int flags);

#define HUF_TABLELOG_MAX             12
#define HUF_SYMBOLVALUE_MAX          255
#define HUF_WORKSPACE_SIZE           ((8 << 10) + 512)
#define HUF_CTABLE_WORKSPACE_SIZE_U32 ((4 * (HUF_SYMBOLVALUE_MAX + 1)) + 192)
#define HUF_CTABLE_WORKSPACE_SIZE    (HUF_CTABLE_WORKSPACE_SIZE_U32 * sizeof(unsigned))
#define HUF_READ_STATS_WORKSPACE_SIZE (FSE_DECOMPRESS_WKSP_SIZE_U32(6, HUF_TABLELOG_MAX-1) * sizeof(unsigned))
#define HUF_DECOMPRESS_WORKSPACE_SIZE ((2 << 10) + (1 << 9))
#define HUF_CTABLE_SIZE_ST(msv)      ((msv) + 2)
#define HUF_DTABLE_SIZE(mtl)         (1 + (1 << (mtl)))

/* ---- HIST ---- */
extern unsigned HIST_isError(size_t code);
extern size_t   HIST_count(unsigned* count, unsigned* maxSymbolValuePtr, const void* src, size_t srcSize);
extern size_t   HIST_countFast(unsigned* count, unsigned* maxSymbolValuePtr, const void* src, size_t srcSize);
extern size_t   HIST_count_wksp(unsigned* count, unsigned* maxSymbolValuePtr, const void* src,
                                size_t srcSize, void* workSpace, size_t workSpaceSize);
extern size_t   HIST_countFast_wksp(unsigned* count, unsigned* maxSymbolValuePtr, const void* src,
                                    size_t srcSize, void* workSpace, size_t workSpaceSize);
extern unsigned HIST_count_simple(unsigned* count, unsigned* maxSymbolValuePtr, const void* src, size_t srcSize);
extern void     HIST_add(unsigned* count, const void* src, size_t srcSize);
#define HIST_WKSP_SIZE (1024 * sizeof(unsigned))

/* ---- xxHash (namespaced ZSTD_) ---- */
typedef enum { XXH_OK = 0, XXH_ERROR } XXH_errorcode;
typedef struct { unsigned char digest[4]; } XXH32_canonical_t;
typedef struct { unsigned char digest[8]; } XXH64_canonical_t;

extern unsigned ZSTD_XXH_versionNumber(void);
extern U32      ZSTD_XXH32(const void* input, size_t length, U32 seed);
extern void*    ZSTD_XXH32_createState(void);
extern XXH_errorcode ZSTD_XXH32_freeState(void* statePtr);
extern void     ZSTD_XXH32_copyState(void* dst, const void* src);
extern XXH_errorcode ZSTD_XXH32_reset(void* statePtr, U32 seed);
extern XXH_errorcode ZSTD_XXH32_update(void* statePtr, const void* input, size_t length);
extern U32      ZSTD_XXH32_digest(const void* statePtr);
extern void     ZSTD_XXH32_canonicalFromHash(XXH32_canonical_t* dst, U32 hash);
extern U32      ZSTD_XXH32_hashFromCanonical(const XXH32_canonical_t* src);

extern U64      ZSTD_XXH64(const void* input, size_t length, U64 seed);
extern void*    ZSTD_XXH64_createState(void);
extern XXH_errorcode ZSTD_XXH64_freeState(void* statePtr);
extern void     ZSTD_XXH64_copyState(void* dst, const void* src);
extern XXH_errorcode ZSTD_XXH64_reset(void* statePtr, U64 seed);
extern XXH_errorcode ZSTD_XXH64_update(void* statePtr, const void* input, size_t length);
extern U64      ZSTD_XXH64_digest(const void* statePtr);
extern void     ZSTD_XXH64_canonicalFromHash(XXH64_canonical_t* dst, U64 hash);
extern U64      ZSTD_XXH64_hashFromCanonical(const XXH64_canonical_t* src);

/* ---- divsufsort ---- */
extern int divsufsort(const unsigned char* T, int* SA, int n, int openMP);
extern int divbwt(const unsigned char* T, unsigned char* U, int* A, int n,
                  unsigned char* num_indexes, int* indexes, int openMP);

/* ---- misc zstd internals / public ---- */
typedef enum { bt_raw, bt_rle, bt_compressed, bt_reserved } blockType_e;
typedef struct { blockType_e blockType; U32 lastBlock; U32 origSize; } blockProperties_t;

extern const char* ERR_getErrorString(int code);
extern const char* ZSTD_getErrorString(int code);
extern const char* ZSTD_getErrorName(size_t code);
extern int         ZSTD_getErrorCode(size_t functionResult);
extern unsigned    ZSTD_isError(size_t code);
extern unsigned    ZSTD_versionNumber(void);
extern const char* ZSTD_versionString(void);
extern size_t      ZSTD_getcBlockSize(const void* src, size_t srcSize, blockProperties_t* bpPtr);
extern size_t      ZSTD_frameHeaderSize(const void* src, size_t srcSize);
extern unsigned    ZSTD_isFrame(const void* buffer, size_t size);
extern unsigned    ZSTD_isSkippableFrame(const void* buffer, size_t size);
extern void        ZSTD_buildFSETable(void* dt, const short* normalizedCounter,
                                      unsigned maxSymbolValue, const U32* baseValue,
                                      const U8* nbAdditionalBits, unsigned tableLog,
                                      void* wksp, size_t wkspSize, int bmi2);
extern void*       ZSTD_createDCtx(void);
extern size_t      ZSTD_freeDCtx(void* dctx);
extern size_t      ZSTD_decompressDCtx(void* dctx, void* dst, size_t dstCap, const void* src, size_t srcSize);
extern size_t      ZSTD_compress(void* dst, size_t dstCap, const void* src, size_t srcSize, int level);
extern size_t      ZSTD_decodeSeqHeaders(void* dctx, int* nbSeqPtr, const void* src, size_t srcSize);

#define MaxSeq 52
#define MaxFSELog 9
#define ZSTD_BUILD_FSE_TABLE_WKSP_SIZE (sizeof(short) * (MaxSeq + 1) + (1u << MaxFSELog) + sizeof(U64))

/* ===================================================================== */
/*  infrastructure                                                       */
/* ===================================================================== */

static unsigned long long g_calls = 0;
/* every library call goes through API(): counts calls without printing addrs */
#define API(fn) (g_calls++, fn)

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
#define R(x) ((long long)(size_t)(x))
#define H(p,n) fnv((p),(size_t)(n))

/* ---- buffers ---- */
#define MAXSRC   200000u
#define MAXDST   1200000u
#define PAT_DST  0x5A
#define PAT_WK   0xA5

/* fixed, non-overlapping regions inside cbuf so every hash covers a stable area */
#define O_HDR    0u          /* entropy-table header, 1 KB          */
#define O_HDR2   2048u       /* header written with a tiny capacity */
#define O_TINY   4096u       /* tiny-capacity compression target    */
#define O_C1     8192u       /* single-stream payload, 200 KB       */
#define O_C4     300000u     /* 4-stream payload, 200 KB            */
#define O_BLK    600000u     /* header||payload block, 400 KB       */
#define O_REP    1000000u    /* repeat-mode output, 200 KB          */
#define CAP_C    200000u
#define DOUT     200064u     /* bytes of dbuf hashed as "the output" */

static unsigned char* src;
static unsigned char* cbuf;   /* compressed / header output */
static unsigned char* dbuf;   /* decompressed output */
static unsigned char* rbuf;   /* random "corrupt" input */

static short*    norm;        /* normalized counter, 256+ shorts */
static short*    norm2;
static unsigned* cnt;         /* histogram 256+ */
static unsigned* cnt2;

static FSE_CTable* fse_ct;
static FSE_DTable* fse_dt;
static unsigned*   fse_ctwksp;
static unsigned*   fse_dtwksp;
static unsigned*   fse_dcwksp;

static HUF_CElt*   huf_ct;
static HUF_CElt*   huf_ct2;
static HUF_CElt*   huf_scratch;
static HUF_DTable* huf_dt;
static unsigned*   huf_wksp;      /* HUF_WORKSPACE_SIZE */
static unsigned*   huf_ctwksp;    /* HUF_CTABLE_WORKSPACE_SIZE */
static unsigned*   huf_dwksp;     /* HUF_DECOMPRESS_WORKSPACE_SIZE */
static unsigned*   huf_rswksp;    /* HUF_READ_STATS_WORKSPACE_SIZE */
static unsigned*   hist_wksp;

#define FSE_CT_U32   FSE_CTABLE_SIZE_U32(15, 255)
#define FSE_DT_U32   FSE_DTABLE_SIZE_U32(15)
#define FSE_CTWK_U32 (FSE_BUILD_CTABLE_WORKSPACE_SIZE_U32(255, 15) + 64)
#define FSE_DTWK_U32 (FSE_BUILD_DTABLE_WKSP_SIZE_U32(15, 255) + 64)
#define FSE_DCWK_U32 (FSE_DECOMPRESS_WKSP_SIZE_U32(15, 255) + 64)

static void* xalloc(size_t n) {
    void* p = malloc(n);
    if (!p) { fprintf(stderr, "oom\n"); exit(2); }
    memset(p, 0, n);
    return p;
}

static void setup(void) {
    src   = (unsigned char*)xalloc(MAXSRC + 64);
    cbuf  = (unsigned char*)xalloc(MAXDST + 64);
    dbuf  = (unsigned char*)xalloc(MAXDST + 64);
    rbuf  = (unsigned char*)xalloc(4096 + 64);
    norm  = (short*)xalloc(1024 * sizeof(short));
    norm2 = (short*)xalloc(1024 * sizeof(short));
    cnt   = (unsigned*)xalloc(1024 * sizeof(unsigned));
    cnt2  = (unsigned*)xalloc(1024 * sizeof(unsigned));
    fse_ct     = (FSE_CTable*)xalloc(FSE_CT_U32   * sizeof(unsigned));
    fse_dt     = (FSE_DTable*)xalloc(FSE_DT_U32   * sizeof(unsigned));
    fse_ctwksp = (unsigned*)  xalloc(FSE_CTWK_U32 * sizeof(unsigned));
    fse_dtwksp = (unsigned*)  xalloc(FSE_DTWK_U32 * sizeof(unsigned));
    fse_dcwksp = (unsigned*)  xalloc(FSE_DCWK_U32 * sizeof(unsigned));
    huf_ct      = (HUF_CElt*)xalloc(HUF_CTABLE_SIZE_ST(255) * sizeof(HUF_CElt) + 64);
    huf_ct2     = (HUF_CElt*)xalloc(HUF_CTABLE_SIZE_ST(255) * sizeof(HUF_CElt) + 64);
    huf_scratch = (HUF_CElt*)xalloc(HUF_CTABLE_SIZE_ST(255) * sizeof(HUF_CElt) + 64);
    huf_dt      = (HUF_DTable*)xalloc(HUF_DTABLE_SIZE(HUF_TABLELOG_MAX) * sizeof(HUF_DTable) + 4096);
    huf_wksp    = (unsigned*)xalloc(HUF_WORKSPACE_SIZE * 4);
    huf_ctwksp  = (unsigned*)xalloc(HUF_CTABLE_WORKSPACE_SIZE * 4);
    huf_dwksp   = (unsigned*)xalloc(HUF_DECOMPRESS_WORKSPACE_SIZE * 4);
    huf_rswksp  = (unsigned*)xalloc(HUF_READ_STATS_WORKSPACE_SIZE * 4);
    hist_wksp   = (unsigned*)xalloc(HIST_WKSP_SIZE * 4);
}

/* ---- corpora, always with alphabet limited to [0..maxv] ---- */
static const char* kCorpus[] = { "rand", "zero", "text", "period", "skew", "twoval", "rle1" };
#define NCORPUS 7

static void mk_src(int c, size_t n, unsigned maxv) {
    size_t i;
    unsigned m = maxv + 1;
    rs(0x9E3779B97F4A7C15ULL ^ ((unsigned long long)c * 1000003ULL)
       ^ ((unsigned long long)n << 17) ^ ((unsigned long long)maxv << 43));
    switch (c) {
    case 0: for (i = 0; i < n; i++) src[i] = (unsigned char)(r32() % m); break;
    case 1: memset(src, 0, n); break;
    case 2: for (i = 0; i < n; i++) src[i] = (unsigned char)((97 + (i % 26)) % m); break;
    case 3: for (i = 0; i < n; i++) src[i] = (unsigned char)(i % m); break;
    case 4: for (i = 0; i < n; i++) src[i] = (unsigned char)((r32() % 16) ? 0 : (r32() % m)); break;
    case 5: for (i = 0; i < n; i++) src[i] = (unsigned char)((i & 1) ? maxv : 0); break;
    default: memset(src, (int)(maxv > 1 ? 1 : maxv), n); break;
    }
}

static void mk_rand(size_t n, unsigned long long seed) {
    size_t i;
    rs(seed);
    for (i = 0; i < n; i++) rbuf[i] = (unsigned char)r32();
}

/* ===================================================================== */
/*  SECTION 1 : FSE scalar / bound helpers                               */
/* ===================================================================== */

static const size_t kErrCodes[] = {
    0, 1, 2, 3, 10, 100, 1000, 0x7FFFFFFF,
    (size_t)-1, (size_t)-2, (size_t)-3, (size_t)-4, (size_t)-5, (size_t)-6, (size_t)-7,
    (size_t)-8, (size_t)-10, (size_t)-11, (size_t)-12, (size_t)-14, (size_t)-16,
    (size_t)-18, (size_t)-20, (size_t)-22, (size_t)-24, (size_t)-26, (size_t)-28,
    (size_t)-30, (size_t)-32, (size_t)-34, (size_t)-36, (size_t)-38, (size_t)-40,
    (size_t)-44, (size_t)-46, (size_t)-48, (size_t)-50, (size_t)-52, (size_t)-60,
    (size_t)-62, (size_t)-64, (size_t)-70, (size_t)-100, (size_t)-120, (size_t)-127,
    (size_t)-128, (size_t)-129, (size_t)-200, (size_t)-1000
};
#define NERR (sizeof(kErrCodes)/sizeof(kErrCodes[0]))

static const size_t kSizes[] = {
    0, 1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 100, 127, 128, 255, 256, 511, 512,
    1000, 1024, 4095, 4096, 10000, 16384, 65535, 65536, 100000, 131072, 200000
};
#define NSIZES (sizeof(kSizes)/sizeof(kSizes[0]))

static void sec_fse_scalar(void) {
    size_t i; unsigned a, b, c;
    printf("== FSE scalar ==\n");
    printf("FSE_versionNumber %u\n", API(FSE_versionNumber)());
    for (i = 0; i < NERR; i++) {
        printf("FSE_isError[%lld]=%u name=%s\n", R(kErrCodes[i]),
               API(FSE_isError)(kErrCodes[i]), API(FSE_getErrorName)(kErrCodes[i]));
    }
    for (i = 0; i < NSIZES; i++)
        printf("FSE_compressBound(%llu)=%lld\n", (unsigned long long)kSizes[i],
               R(API(FSE_compressBound)(kSizes[i])));
    /* NOTE: FSE_optimalTableLog{,_internal} assert(srcSize > 1) and evaluate
     * ZSTD_highbit32(srcSize-1); srcSize<2 is a documented contract violation
     * (undefined behaviour on both sides), so it is excluded here. */
    for (a = 0; a <= 16; a++)
        for (i = 2; i < NSIZES; i += 2)
            for (b = 0; b <= 255; b += 51)
                printf("FSE_optimalTableLog(%u,%llu,%u)=%u\n", a,
                       (unsigned long long)kSizes[i], b,
                       API(FSE_optimalTableLog)(a, kSizes[i], b));
    for (a = 0; a <= 16; a += 2)
        for (i = 2; i < NSIZES; i += 3)
            for (b = 0; b <= 255; b += 51)
                for (c = 0; c <= 5; c++)
                    printf("FSE_optimalTableLog_internal(%u,%llu,%u,%u)=%u\n", a,
                           (unsigned long long)kSizes[i], b, c,
                           API(FSE_optimalTableLog_internal)(a, kSizes[i], b, c));
    for (a = 0; a <= 255; a += 17)
        for (b = 0; b <= 17; b++)
            printf("FSE_NCountWriteBound(%u,%u)=%lld\n", a, b,
                   R(API(FSE_NCountWriteBound)(a, b)));
    printf("SECTION fse_scalar calls=%llu\n", g_calls);
    fflush(stdout);
}

/* ===================================================================== */
/*  SECTION 2 : FSE full round trip                                      */
/* ===================================================================== */

static const size_t kFseSizes[] = { 0, 1, 2, 3, 17, 64, 255, 1000, 4096, 20000, 65536 };
#define NFSESZ (sizeof(kFseSizes)/sizeof(kFseSizes[0]))
static const unsigned kMaxV[]  = { 1, 3, 15, 63, 255 };
#define NMAXV (sizeof(kMaxV)/sizeof(kMaxV[0]))
static const unsigned kFseTL[] = { 4, 5, 6, 8, 9, 11, 12, 13, 15 };
#define NFSETL (sizeof(kFseTL)/sizeof(kFseTL[0]))

static void fse_roundtrip(int corp, size_t n, unsigned maxv, unsigned tableLog, unsigned useLow) {
    unsigned maxSV = maxv, maxSV2, tl2;
    size_t rc, wsz, bound, csz, dsz;
    int bmi2;

    mk_src(corp, n, maxv);
    memset(cnt, 0, 1024 * sizeof(unsigned));
    maxSV = maxv;
    rc = API(HIST_count)(cnt, &maxSV, src, n);
    printf("FSE.rt[%s n=%llu mv=%u tl=%u low=%u] HIST_count=%lld maxSV=%u cntH=%016llx\n",
           kCorpus[corp], (unsigned long long)n, maxv, tableLog, useLow,
           R(rc), maxSV, H(cnt, 1024 * sizeof(unsigned)));
    if (HIST_isError(rc)) return;
    /* FSE_normalizeCount asserts srcSize>1 and divides 1<<62 by srcSize; srcSize<2
     * raises SIGFPE inside the C library, so those sizes are excluded. */
    if (n < 2) { printf("  (skip normalizeCount: srcSize<2 is unsupported)\n"); return; }

    memset(norm, PAT_WK, 1024 * sizeof(short));
    rc = API(FSE_normalizeCount)(norm, tableLog, cnt, n, maxSV, useLow);
    printf("  FSE_normalizeCount=%lld normH=%016llx\n", R(rc), H(norm, 1024 * sizeof(short)));
    if (FSE_isError(rc)) return;
    /* rc==0 means "RLE": normalizedCounter was left untouched, nothing else to do */
    if (rc == 0) { printf("  (rle: normalizedCounter not produced)\n"); return; }

    bound = API(FSE_NCountWriteBound)(maxSV, tableLog);
    /* deliberate too-small buffers first (error paths) */
    for (wsz = 0; wsz <= 2; wsz++) {
        memset(cbuf + O_HDR2, PAT_DST, 1024);
        printf("  FSE_writeNCount(cap=%llu)=%lld hdrH=%016llx\n", (unsigned long long)wsz,
               R(API(FSE_writeNCount)(cbuf + O_HDR2, wsz, norm, maxSV, tableLog)),
               H(cbuf + O_HDR2, 1024));
    }
    memset(cbuf + O_HDR, PAT_DST, 1024);
    wsz = API(FSE_writeNCount)(cbuf + O_HDR, bound, norm, maxSV, tableLog);
    printf("  FSE_writeNCount(cap=%llu)=%lld hdrH=%016llx\n", (unsigned long long)bound,
           R(wsz), H(cbuf + O_HDR, 1024));
    if (FSE_isError(wsz)) return;

    /* readNCount and readNCount_bmi2 */
    for (bmi2 = -1; bmi2 <= 1; bmi2++) {
        memset(norm2, PAT_WK, 1024 * sizeof(short));
        maxSV2 = 255; tl2 = 0;
        if (bmi2 < 0) rc = API(FSE_readNCount)(norm2, &maxSV2, &tl2, cbuf + O_HDR, wsz);
        else          rc = API(FSE_readNCount_bmi2)(norm2, &maxSV2, &tl2, cbuf + O_HDR, wsz, bmi2);
        printf("  FSE_readNCount(bmi2=%d)=%lld maxSV=%u tl=%u normH=%016llx\n", bmi2,
               R(rc), maxSV2, tl2, H(norm2, 1024 * sizeof(short)));
    }
    /* truncated header -> error path */
    memset(norm2, PAT_WK, 1024 * sizeof(short));
    maxSV2 = 255; tl2 = 0;
    printf("  FSE_readNCount(trunc)=%lld maxSV=%u tl=%u normH=%016llx\n",
           R(API(FSE_readNCount)(norm2, &maxSV2, &tl2, cbuf + O_HDR, wsz ? wsz - 1 : 0)),
           maxSV2, tl2, H(norm2, 1024 * sizeof(short)));
    /* too small maxSymbolValue -> error path */
    memset(norm2, PAT_WK, 1024 * sizeof(short));
    maxSV2 = 0; tl2 = 0;
    printf("  FSE_readNCount(mv0)=%lld maxSV=%u tl=%u normH=%016llx\n",
           R(API(FSE_readNCount)(norm2, &maxSV2, &tl2, cbuf + O_HDR, wsz)),
           maxSV2, tl2, H(norm2, 1024 * sizeof(short)));

    /* re-read cleanly for the decode side */
    memset(norm2, PAT_WK, 1024 * sizeof(short));
    maxSV2 = 255; tl2 = 0;
    rc = API(FSE_readNCount)(norm2, &maxSV2, &tl2, cbuf + O_HDR, wsz);
    if (FSE_isError(rc)) return;

    /* CTable */
    memset(fse_ctwksp, PAT_WK, FSE_CTWK_U32 * sizeof(unsigned));
    memset(fse_ct, PAT_DST, FSE_CT_U32 * sizeof(unsigned));
    rc = API(FSE_buildCTable_wksp)(fse_ct, norm, maxSV, tableLog, fse_ctwksp,
                                   FSE_CTWK_U32 * sizeof(unsigned));
    printf("  FSE_buildCTable_wksp=%lld ctH=%016llx\n", R(rc), H(fse_ct, FSE_CT_U32 * sizeof(unsigned)));
    /* too small workspace -> error */
    memset(fse_ctwksp, PAT_WK, FSE_CTWK_U32 * sizeof(unsigned));
    printf("  FSE_buildCTable_wksp(wk=0)=%lld\n",
           R(API(FSE_buildCTable_wksp)(fse_ct, norm, maxSV, tableLog, fse_ctwksp, 0)));
    if (FSE_isError(rc)) return;
    /* rebuild (previous error call may have clobbered nothing, but be explicit) */
    memset(fse_ctwksp, PAT_WK, FSE_CTWK_U32 * sizeof(unsigned));
    memset(fse_ct, PAT_DST, FSE_CT_U32 * sizeof(unsigned));
    rc = API(FSE_buildCTable_wksp)(fse_ct, norm, maxSV, tableLog, fse_ctwksp,
                                   FSE_CTWK_U32 * sizeof(unsigned));
    if (FSE_isError(rc)) return;

    memset(cbuf + O_C1, PAT_DST, CAP_C);
    csz = API(FSE_compress_usingCTable)(cbuf + O_C1, CAP_C, src, n, fse_ct);
    printf("  FSE_compress_usingCTable=%lld cH=%016llx\n", R(csz), H(cbuf + O_C1, CAP_C));
    /* tiny dst -> error path */
    memset(cbuf + O_TINY, PAT_DST, 64);
    printf("  FSE_compress_usingCTable(cap=3)=%lld tinyH=%016llx\n",
           R(API(FSE_compress_usingCTable)(cbuf + O_TINY, 3, src, n, fse_ct)),
           H(cbuf + O_TINY, 64));

    /* DTable */
    memset(fse_dtwksp, PAT_WK, FSE_DTWK_U32 * sizeof(unsigned));
    memset(fse_dt, PAT_DST, FSE_DT_U32 * sizeof(unsigned));
    rc = API(FSE_buildDTable_wksp)(fse_dt, norm2, maxSV2, tl2, fse_dtwksp,
                                   FSE_DTWK_U32 * sizeof(unsigned));
    printf("  FSE_buildDTable_wksp=%lld dtH=%016llx\n", R(rc),
           H(fse_dt, FSE_DT_U32 * sizeof(unsigned)));
    memset(fse_dtwksp, PAT_WK, FSE_DTWK_U32 * sizeof(unsigned));
    printf("  FSE_buildDTable_wksp(wk=0)=%lld\n",
           R(API(FSE_buildDTable_wksp)(fse_dt, norm2, maxSV2, tl2, fse_dtwksp, 0)));
    printf("  FSE_buildDTable_wksp(mv=300)=%lld\n",
           R(API(FSE_buildDTable_wksp)(fse_dt, norm2, 300, tl2, fse_dtwksp,
                                       FSE_DTWK_U32 * sizeof(unsigned))));
    printf("  FSE_buildDTable_wksp(tl=31)=%lld\n",
           R(API(FSE_buildDTable_wksp)(fse_dt, norm2, maxSV2, 31, fse_dtwksp,
                                       FSE_DTWK_U32 * sizeof(unsigned))));

    if (!FSE_isError(csz) && csz > 0) {
        /* full FSE frame = header || payload, assembled in its own region */
        size_t total = wsz + csz;
        unsigned char* blk = cbuf + O_BLK;
        memcpy(blk, cbuf + O_HDR, wsz);
        memcpy(blk + wsz, cbuf + O_C1, csz);
        for (bmi2 = 0; bmi2 <= 1; bmi2++) {
            unsigned maxLog;
            for (maxLog = tl2; maxLog <= tl2 + 1u && maxLog <= 15u; maxLog++) {
                memset(fse_dcwksp, PAT_WK, FSE_DCWK_U32 * sizeof(unsigned));
                memset(dbuf, PAT_DST, DOUT);
                dsz = API(FSE_decompress_wksp_bmi2)(dbuf, 0, blk, total, maxLog, fse_dcwksp,
                                                    FSE_DCWK_U32 * sizeof(unsigned), bmi2);
                printf("  FSE_decompress_wksp_bmi2(cap=0,maxLog=%u,bmi2=%d)=%lld outH=%016llx\n",
                       maxLog, bmi2, R(dsz), H(dbuf, DOUT));
            }
        }
        for (bmi2 = 0; bmi2 <= 1; bmi2++) {
            memset(fse_dcwksp, PAT_WK, FSE_DCWK_U32 * sizeof(unsigned));
            memset(dbuf, PAT_DST, DOUT);
            dsz = API(FSE_decompress_wksp_bmi2)(dbuf, n ? n : 1, blk, total, tl2,
                                                fse_dcwksp, FSE_DCWK_U32 * sizeof(unsigned), bmi2);
            printf("  FSE_decompress_wksp_bmi2(bmi2=%d)=%lld outH=%016llx match=%d\n",
                   bmi2, R(dsz), H(dbuf, DOUT),
                   (!FSE_isError(dsz) && dsz == n && memcmp(dbuf, src, n) == 0));
        }
        /* truncated payload -> error path */
        for (bmi2 = 0; bmi2 <= 1; bmi2++) {
            memset(fse_dcwksp, PAT_WK, FSE_DCWK_U32 * sizeof(unsigned));
            memset(dbuf, PAT_DST, DOUT);
            printf("  FSE_decompress_wksp_bmi2(trunc,bmi2=%d)=%lld outH=%016llx\n", bmi2,
                   R(API(FSE_decompress_wksp_bmi2)(dbuf, n ? n : 1, blk, total - 1, tl2,
                                                   fse_dcwksp,
                                                   FSE_DCWK_U32 * sizeof(unsigned), bmi2)),
                   H(dbuf, DOUT));
        }
    }
}

static void sec_fse_roundtrip(void) {
    size_t i; unsigned c, m, t, low;
    printf("== FSE roundtrip ==\n");
    for (c = 0; c < NCORPUS; c++)
        for (i = 0; i < NFSESZ; i++)
            for (m = 0; m < NMAXV; m++)
                for (t = 0; t < NFSETL; t++)
                    for (low = 0; low <= 1; low++)
                        fse_roundtrip((int)c, kFseSizes[i], kMaxV[m], kFseTL[t], low);
    printf("SECTION fse_roundtrip calls=%llu\n", g_calls);
    fflush(stdout);
}

static void sec_fse_rle(void) {
    unsigned s;
    printf("== FSE rle ==\n");
    for (s = 0; s < 256; s += 7) {
        size_t csz;
        memset(fse_ct, PAT_DST, FSE_CT_U32 * sizeof(unsigned));
        printf("FSE_buildCTable_rle(%u)=%lld ctH=%016llx\n", s,
               R(API(FSE_buildCTable_rle)(fse_ct, (unsigned char)s)),
               H(fse_ct, 64));
        memset(src, (int)s, 5000);
        memset(cbuf + O_C1, PAT_DST, CAP_C);
        csz = API(FSE_compress_usingCTable)(cbuf + O_C1, CAP_C, src, 5000, fse_ct);
        printf("  FSE_compress_usingCTable(rle)=%lld cH=%016llx\n", R(csz), H(cbuf + O_C1, 4096));
    }
    printf("SECTION fse_rle calls=%llu\n", g_calls);
    fflush(stdout);
}

/* ===================================================================== */
/*  SECTION 3 : FSE corrupt / random input                               */
/* ===================================================================== */

static void sec_fse_corrupt(void) {
    static const size_t rsz[] = { 0, 1, 2, 3, 4, 5, 8, 13, 20, 33, 64, 100, 200, 520, 1000, 4096 };
    const size_t nrsz = sizeof(rsz) / sizeof(rsz[0]);
    size_t i, k;
    unsigned trial;
    printf("== FSE corrupt ==\n");
    for (trial = 0; trial < 40; trial++) {
        for (i = 0; i < nrsz; i++) {
            size_t n = rsz[i];
            int bmi2;
            unsigned mvcap;
            mk_rand(n, 0xC0FFEEULL * (trial + 1) + n * 7919ULL);
            for (mvcap = 0; mvcap <= 255; mvcap += 85) {
                for (bmi2 = -1; bmi2 <= 1; bmi2++) {
                    unsigned mv = mvcap, tl = 0;
                    size_t rc;
                    memset(norm2, PAT_WK, 1024 * sizeof(short));
                    if (bmi2 < 0) rc = API(FSE_readNCount)(norm2, &mv, &tl, rbuf, n);
                    else          rc = API(FSE_readNCount_bmi2)(norm2, &mv, &tl, rbuf, n, bmi2);
                    printf("FSE.cor t=%u n=%llu mvcap=%u bmi2=%d readNCount=%lld mv=%u tl=%u normH=%016llx\n",
                           trial, (unsigned long long)n, mvcap, bmi2, R(rc), mv, tl,
                           H(norm2, 1024 * sizeof(short)));
                    if (bmi2 == 1 && !FSE_isError(rc)) {
                        /* counts read from a random buffer are internally consistent,
                         * so building a DTable from them is safe */
                        memset(fse_dtwksp, PAT_WK, FSE_DTWK_U32 * sizeof(unsigned));
                        memset(fse_dt, PAT_DST, FSE_DT_U32 * sizeof(unsigned));
                        printf("  FSE_buildDTable_wksp=%lld dtH=%016llx\n",
                               R(API(FSE_buildDTable_wksp)(fse_dt, norm2, mv, tl, fse_dtwksp,
                                                           FSE_DTWK_U32 * sizeof(unsigned))),
                               H(fse_dt, FSE_DT_U32 * sizeof(unsigned)));
                    }
                }
            }
            for (k = 0; k <= 3; k++) {
                unsigned maxLog = (unsigned)(5 + 4 * k);   /* 5, 9, 13, 17 */
                int bmi2;
                size_t caps[3]; caps[0] = 0; caps[1] = 100; caps[2] = 10000;
                for (bmi2 = 0; bmi2 <= 1; bmi2++) {
                    size_t ci;
                    for (ci = 0; ci < 3; ci++) {
                        size_t rc;
                        memset(fse_dcwksp, PAT_WK, FSE_DCWK_U32 * sizeof(unsigned));
                        memset(dbuf, PAT_DST, 20000);
                        rc = API(FSE_decompress_wksp_bmi2)(dbuf, caps[ci], rbuf, n, maxLog,
                                                           fse_dcwksp,
                                                           FSE_DCWK_U32 * sizeof(unsigned), bmi2);
                        printf("  FSE_decompress_wksp_bmi2 t=%u n=%llu maxLog=%u cap=%llu bmi2=%d =%lld outH=%016llx\n",
                               trial, (unsigned long long)n, maxLog,
                               (unsigned long long)caps[ci], bmi2, R(rc), H(dbuf, 20000));
                    }
                }
                /* tiny workspace -> error path */
                memset(fse_dcwksp, PAT_WK, FSE_DCWK_U32 * sizeof(unsigned));
                memset(dbuf, PAT_DST, 20000);
                printf("  FSE_decompress_wksp_bmi2(wk=16) t=%u n=%llu maxLog=%u =%lld\n",
                       trial, (unsigned long long)n, maxLog,
                       R(API(FSE_decompress_wksp_bmi2)(dbuf, 10000, rbuf, n, maxLog,
                                                       fse_dcwksp, 16, 0)));
            }
        }
    }
    printf("SECTION fse_corrupt calls=%llu\n", g_calls);
    fflush(stdout);
}

/* ===================================================================== */
/*  SECTION 4 : HIST                                                     */
/* ===================================================================== */

static void sec_hist(void) {
    size_t i, k;
    unsigned c, m;
    printf("== HIST ==\n");
    for (i = 0; i < NERR; i++)
        printf("HIST_isError[%lld]=%u\n", R(kErrCodes[i]), API(HIST_isError)(kErrCodes[i]));

    for (c = 0; c < NCORPUS; c++) {
        for (i = 0; i < NFSESZ; i++) {
            size_t n = kFseSizes[i];
            for (m = 0; m < NMAXV; m++) {
                unsigned maxv = kMaxV[m], mv;
                mk_src((int)c, n, maxv);
                /* exact and over-sized declared alphabets: always safe */
                for (k = 0; k < 3; k++) {
                    unsigned decl = (k == 0) ? maxv : (k == 1 ? 255u : (maxv < 255 ? maxv + 1 : 255u));
                    size_t rc;
                    mv = decl;
                    memset(cnt, PAT_WK, 1024 * sizeof(unsigned));
                    rc = API(HIST_count)(cnt, &mv, src, n);
                    printf("HIST_count[%s n=%llu mv=%u decl=%u]=%lld mv'=%u H=%016llx\n",
                           kCorpus[c], (unsigned long long)n, maxv, decl, R(rc), mv,
                           H(cnt, 1024 * sizeof(unsigned)));

                    mv = decl;
                    memset(cnt, PAT_WK, 1024 * sizeof(unsigned));
                    memset(hist_wksp, PAT_WK, HIST_WKSP_SIZE * 4);
                    rc = API(HIST_count_wksp)(cnt, &mv, src, n, hist_wksp, HIST_WKSP_SIZE);
                    printf("  HIST_count_wksp=%lld mv'=%u H=%016llx\n", R(rc), mv,
                           H(cnt, 1024 * sizeof(unsigned)));

                    mv = decl;
                    memset(cnt, PAT_WK, 1024 * sizeof(unsigned));
                    memset(hist_wksp, PAT_WK, HIST_WKSP_SIZE * 4);
                    rc = API(HIST_count_wksp)(cnt, &mv, src, n, hist_wksp, 8);
                    printf("  HIST_count_wksp(small)=%lld mv'=%u\n", R(rc), mv);

                    mv = decl;
                    memset(cnt, PAT_WK, 1024 * sizeof(unsigned));
                    rc = API(HIST_countFast)(cnt, &mv, src, n);
                    printf("  HIST_countFast=%lld mv'=%u H=%016llx\n", R(rc), mv,
                           H(cnt, 1024 * sizeof(unsigned)));

                    mv = decl;
                    memset(cnt, PAT_WK, 1024 * sizeof(unsigned));
                    memset(hist_wksp, PAT_WK, HIST_WKSP_SIZE * 4);
                    rc = API(HIST_countFast_wksp)(cnt, &mv, src, n, hist_wksp, HIST_WKSP_SIZE);
                    printf("  HIST_countFast_wksp=%lld mv'=%u H=%016llx\n", R(rc), mv,
                           H(cnt, 1024 * sizeof(unsigned)));

                    mv = decl;
                    memset(cnt, PAT_WK, 1024 * sizeof(unsigned));
                    memset(hist_wksp, PAT_WK, HIST_WKSP_SIZE * 4);
                    rc = API(HIST_countFast_wksp)(cnt, &mv, src, n, hist_wksp, 8);
                    printf("  HIST_countFast_wksp(small)=%lld mv'=%u\n", R(rc), mv);

                    mv = decl;
                    memset(cnt, PAT_WK, 1024 * sizeof(unsigned));
                    printf("  HIST_count_simple=%u mv'=%u H=%016llx\n",
                           API(HIST_count_simple)(cnt, &mv, src, n), mv,
                           H(cnt, 1024 * sizeof(unsigned)));

                    memset(cnt, 0, 1024 * sizeof(unsigned));
                    API(HIST_add)(cnt, src, n);
                    API(HIST_add)(cnt, src, n);
                    printf("  HIST_add x2 H=%016llx\n", H(cnt, 1024 * sizeof(unsigned)));
                }
                /* under-sized declared alphabet -> HIST_count must report an error.
                 * HIST_countFast / HIST_count_simple are documented as unsafe here,
                 * so they are NOT called with decl < maxv. */
                if (maxv > 0) {
                    unsigned decl = maxv - 1;
                    mv = decl;
                    memset(cnt, PAT_WK, 1024 * sizeof(unsigned));
                    printf("  HIST_count(under=%u)=%lld mv'=%u H=%016llx\n", decl,
                           R(API(HIST_count)(cnt, &mv, src, n)), mv,
                           H(cnt, 1024 * sizeof(unsigned)));
                    mv = decl;
                    memset(cnt, PAT_WK, 1024 * sizeof(unsigned));
                    memset(hist_wksp, PAT_WK, HIST_WKSP_SIZE * 4);
                    printf("  HIST_count_wksp(under=%u)=%lld mv'=%u H=%016llx\n", decl,
                           R(API(HIST_count_wksp)(cnt, &mv, src, n, hist_wksp, HIST_WKSP_SIZE)), mv,
                           H(cnt, 1024 * sizeof(unsigned)));
                }
            }
        }
    }
    printf("SECTION hist calls=%llu\n", g_calls);
    fflush(stdout);
}

/* ===================================================================== */
/*  SECTION 5 : HUF                                                      */
/* ===================================================================== */

static const size_t kHufSizes[] = { 0, 1, 2, 13, 64, 300, 1000, 17000, 131072 };
#define NHUFSZ (sizeof(kHufSizes)/sizeof(kHufSizes[0]))
static const unsigned kHufLog[] = { 0, 1, 5, 8, 11, 12, 13 };
#define NHUFLOG (sizeof(kHufLog)/sizeof(kHufLog[0]))

static void huf_show_nbbits(const char* tag, const HUF_CElt* ct) {
    unsigned s;
    unsigned long long h = 1469598103934665603ULL;
    for (s = 0; s < 256; s++) {
        U32 v = API(HUF_getNbBitsFromCTable)(ct, s);
        h ^= (unsigned char)(v & 0xFF); h *= 1099511628211ULL;
        h ^= (unsigned char)((v >> 8) & 0xFF); h *= 1099511628211ULL;
    }
    printf("  HUF_getNbBitsFromCTable[%s] allH=%016llx b0=%u b1=%u b255=%u\n", tag, h,
           API(HUF_getNbBitsFromCTable)(ct, 0), API(HUF_getNbBitsFromCTable)(ct, 1),
           API(HUF_getNbBitsFromCTable)(ct, 255));
}

static void huf_show_hdr(const char* tag, const HUF_CElt* ct) {
    HUF_CTableHeader h = API(HUF_readCTableHeader)(ct);
    printf("  HUF_readCTableHeader[%s] tableLog=%u maxSymbolValue=%u unusedH=%016llx\n",
           tag, (unsigned)h.tableLog, (unsigned)h.maxSymbolValue,
           H(h.unused, sizeof(h.unused)));
}

static void huf_roundtrip(int corp, size_t n, unsigned maxv, unsigned huffLog, int flags) {
    unsigned mv = maxv, card, otl, mv2, hasZero, wLog;
    size_t rc, hdrSz, c1, c4;
    int repeat;
    const size_t CTBYTES = HUF_CTABLE_SIZE_ST(255) * sizeof(HUF_CElt);
    const size_t DTBYTES = HUF_DTABLE_SIZE(HUF_TABLELOG_MAX) * sizeof(HUF_DTable);

    mk_src(corp, n, maxv);
    memset(cnt, 0, 1024 * sizeof(unsigned));
    mv = maxv;
    rc = API(HIST_count)(cnt, &mv, src, n);
    printf("HUF.rt[%s n=%llu mv=%u hl=%u fl=%d] HIST_count=%lld mv'=%u\n",
           kCorpus[corp], (unsigned long long)n, maxv, huffLog, flags, R(rc), mv);
    if (HIST_isError(rc)) return;

    card = API(HUF_cardinality)(cnt, mv);
    printf("  HUF_cardinality=%u\n", card);
    /* HUF_optimalTableLog asserts srcSize>1 (and reaches ZSTD_highbit32(srcSize-1));
     * HUF_minTableLog(0) evaluates ZSTD_highbit32(0). Both are UB, so excluded. */
    if (n < 2 || card == 0) {
        printf("  (skip: HUF_optimalTableLog needs srcSize>1, HUF_minTableLog needs cardinality>0)\n");
        return;
    }
    printf("  HUF_minTableLog=%u\n", API(HUF_minTableLog)(card));

    memset(huf_wksp, PAT_WK, HUF_WORKSPACE_SIZE);
    memset(huf_scratch, PAT_DST, CTBYTES);
    otl = API(HUF_optimalTableLog)(huffLog, n, mv, huf_wksp, HUF_WORKSPACE_SIZE,
                                  huf_scratch, cnt, flags);
    printf("  HUF_optimalTableLog=%u scratchH=%016llx\n", otl, H(huf_scratch, CTBYTES));

    memset(huf_ctwksp, PAT_WK, HUF_CTABLE_WORKSPACE_SIZE);
    memset(huf_ct, PAT_DST, CTBYTES);
    rc = API(HUF_buildCTable_wksp)(huf_ct, cnt, mv, huffLog, huf_ctwksp, HUF_CTABLE_WORKSPACE_SIZE);
    printf("  HUF_buildCTable_wksp=%lld ctH=%016llx\n", R(rc), H(huf_ct, CTBYTES));
    /* too-small workspace error path */
    memset(huf_ctwksp, PAT_WK, HUF_CTABLE_WORKSPACE_SIZE);
    printf("  HUF_buildCTable_wksp(wk=8)=%lld\n",
           R(API(HUF_buildCTable_wksp)(huf_ct2, cnt, mv, huffLog, huf_ctwksp, 8)));
    printf("  HUF_buildCTable_wksp(mv=300)=%lld\n",
           R(API(HUF_buildCTable_wksp)(huf_ct2, cnt, 300, huffLog, huf_ctwksp,
                                       HUF_CTABLE_WORKSPACE_SIZE)));
    if (HUF_isError(rc)) return;
    /* rebuild cleanly */
    memset(huf_ctwksp, PAT_WK, HUF_CTABLE_WORKSPACE_SIZE);
    memset(huf_ct, PAT_DST, CTBYTES);
    rc = API(HUF_buildCTable_wksp)(huf_ct, cnt, mv, huffLog, huf_ctwksp, HUF_CTABLE_WORKSPACE_SIZE);
    if (HUF_isError(rc)) return;

    huf_show_hdr("built", huf_ct);
    huf_show_nbbits("built", huf_ct);
    printf("  HUF_estimateCompressedSize=%lld HUF_validateCTable=%d\n",
           R(API(HUF_estimateCompressedSize)(huf_ct, cnt, mv)),
           API(HUF_validateCTable)(huf_ct, cnt, mv));

    /* write / read the CTable header.
     * HUF_writeCTable_wksp asserts that `huffLog` == the tableLog recorded in the
     * CTable header, i.e. the value HUF_buildCTable_wksp just returned.  Passing
     * anything smaller makes the C library read uninitialised weights and crash,
     * so the effective tableLog is used here. */
    wLog = (unsigned)rc;
    memset(cbuf + O_HDR, PAT_DST, 1024);
    hdrSz = API(HUF_writeCTable_wksp)(cbuf + O_HDR, 1024, huf_ct, mv, wLog,
                                     huf_wksp, HUF_WORKSPACE_SIZE);
    printf("  HUF_writeCTable_wksp(wLog=%u)=%lld hdrH=%016llx\n", wLog, R(hdrSz),
           H(cbuf + O_HDR, 1024));
    memset(cbuf + O_HDR2, PAT_DST, 1024);
    printf("  HUF_writeCTable_wksp(cap=1)=%lld hdr2H=%016llx\n",
           R(API(HUF_writeCTable_wksp)(cbuf + O_HDR2, 1, huf_ct, mv, wLog,
                                       huf_wksp, HUF_WORKSPACE_SIZE)),
           H(cbuf + O_HDR2, 1024));
    memset(huf_wksp, PAT_WK, HUF_WORKSPACE_SIZE);
    printf("  HUF_writeCTable_wksp(wk=8)=%lld\n",
           R(API(HUF_writeCTable_wksp)(cbuf + O_HDR2, 1024, huf_ct, mv, wLog, huf_wksp, 8)));
    if (HUF_isError(hdrSz) || hdrSz == 0) return;

    mv2 = 255; hasZero = 0xFFFFFFFFu;
    memset(huf_ct2, PAT_DST, CTBYTES);
    printf("  HUF_readCTable=%lld mv'=%u hasZero=%u ct2H=%016llx\n",
           R(API(HUF_readCTable)(huf_ct2, &mv2, cbuf + O_HDR, hdrSz, &hasZero)), mv2, hasZero,
           H(huf_ct2, CTBYTES));
    huf_show_hdr("read", huf_ct2);
    huf_show_nbbits("read", huf_ct2);

    /* HUF_readStats on the header */
    {
        U8  hw[256 + 16];
        U32 rank[32], nbSym, tl;
        memset(hw, PAT_WK, sizeof(hw));
        memset(rank, PAT_WK, sizeof(rank));
        nbSym = 0; tl = 0;
        printf("  HUF_readStats=%lld nbSym=%u tl=%u hwH=%016llx rankH=%016llx\n",
               R(API(HUF_readStats)(hw, 256, rank, &nbSym, &tl, cbuf + O_HDR, hdrSz)),
               nbSym, tl, H(hw, sizeof(hw)), H(rank, sizeof(rank)));
        memset(hw, PAT_WK, sizeof(hw));
        memset(rank, PAT_WK, sizeof(rank));
        memset(huf_rswksp, PAT_WK, HUF_READ_STATS_WORKSPACE_SIZE);
        nbSym = 0; tl = 0;
        printf("  HUF_readStats_wksp=%lld nbSym=%u tl=%u hwH=%016llx rankH=%016llx\n",
               R(API(HUF_readStats_wksp)(hw, 256, rank, &nbSym, &tl, cbuf + O_HDR, hdrSz,
                                         huf_rswksp, HUF_READ_STATS_WORKSPACE_SIZE, flags)),
               nbSym, tl, H(hw, sizeof(hw)), H(rank, sizeof(rank)));
        memset(hw, PAT_WK, sizeof(hw));
        memset(rank, PAT_WK, sizeof(rank));
        memset(huf_rswksp, PAT_WK, HUF_READ_STATS_WORKSPACE_SIZE);
        nbSym = 0; tl = 0;
        printf("  HUF_readStats_wksp(wk=8)=%lld nbSym=%u tl=%u\n",
               R(API(HUF_readStats_wksp)(hw, 256, rank, &nbSym, &tl, cbuf + O_HDR, hdrSz,
                                         huf_rswksp, 8, flags)), nbSym, tl);
        memset(hw, PAT_WK, sizeof(hw));
        memset(rank, PAT_WK, sizeof(rank));
        nbSym = 0; tl = 0;
        printf("  HUF_readStats(hw=4)=%lld nbSym=%u tl=%u\n",
               R(API(HUF_readStats)(hw, 4, rank, &nbSym, &tl, cbuf + O_HDR, hdrSz)), nbSym, tl);
    }

    /* compress with the CTable (alphabet of src is <= mv so the table covers it) */
    memset(cbuf + O_C1, PAT_DST, CAP_C);
    c1 = API(HUF_compress1X_usingCTable)(cbuf + O_C1, CAP_C, src, n, huf_ct, flags);
    printf("  HUF_compress1X_usingCTable=%lld cH=%016llx\n", R(c1), H(cbuf + O_C1, CAP_C));
    memset(cbuf + O_C4, PAT_DST, CAP_C);
    c4 = API(HUF_compress4X_usingCTable)(cbuf + O_C4, CAP_C, src, n, huf_ct, flags);
    printf("  HUF_compress4X_usingCTable=%lld cH=%016llx\n", R(c4), H(cbuf + O_C4, CAP_C));
    memset(cbuf + O_TINY, PAT_DST, 64);
    printf("  HUF_compress1X_usingCTable(cap=2)=%lld tinyH=%016llx\n",
           R(API(HUF_compress1X_usingCTable)(cbuf + O_TINY, 2, src, n, huf_ct, flags)),
           H(cbuf + O_TINY, 64));
    memset(cbuf + O_TINY, PAT_DST, 64);
    printf("  HUF_compress4X_usingCTable(cap=2)=%lld tinyH=%016llx\n",
           R(API(HUF_compress4X_usingCTable)(cbuf + O_TINY, 2, src, n, huf_ct, flags)),
           H(cbuf + O_TINY, 64));

    /* selectDecoder */
    printf("  HUF_selectDecoder(%llu,%llu)=%u\n", (unsigned long long)n,
           (unsigned long long)(HUF_isError(c1) ? 0 : c1),
           API(HUF_selectDecoder)(n, HUF_isError(c1) ? 0 : c1));

    /* decode tables */
    {
        unsigned dlog;
        for (dlog = 0; dlog < 2; dlog++) {
            unsigned maxTableLog = dlog ? HUF_TABLELOG_MAX : 8u;
            /* X1 */
            memset(huf_dt, 0, DTBYTES);
            huf_dt[0] = (U32)(maxTableLog - 1) * 0x01000001u;
            memset(huf_dwksp, PAT_WK, HUF_DECOMPRESS_WORKSPACE_SIZE);
            rc = API(HUF_readDTableX1_wksp)(huf_dt, cbuf + O_HDR, hdrSz, huf_dwksp,
                                            HUF_DECOMPRESS_WORKSPACE_SIZE, flags);
            printf("  HUF_readDTableX1_wksp(mtl=%u)=%lld dtH=%016llx\n", maxTableLog, R(rc),
                   H(huf_dt, DTBYTES));
            if (!HUF_isError(rc) && !HUF_isError(c1) && c1 > 0) {
                memset(dbuf, PAT_DST, DOUT);
                printf("    HUF_decompress1X_usingDTable=%lld outH=%016llx match=%d\n",
                       R(API(HUF_decompress1X_usingDTable)(dbuf, n, cbuf + O_C1, c1, huf_dt, flags)),
                       H(dbuf, DOUT), memcmp(dbuf, src, n) == 0);
                memset(dbuf, PAT_DST, DOUT);
                printf("    HUF_decompress1X_usingDTable(cap=1)=%lld outH=%016llx\n",
                       R(API(HUF_decompress1X_usingDTable)(dbuf, 1, cbuf + O_C1, c1, huf_dt, flags)),
                       H(dbuf, DOUT));
            }
            if (!HUF_isError(rc) && !HUF_isError(c4) && c4 > 0) {
                memset(dbuf, PAT_DST, DOUT);
                printf("    HUF_decompress4X_usingDTable=%lld outH=%016llx match=%d\n",
                       R(API(HUF_decompress4X_usingDTable)(dbuf, n, cbuf + O_C4, c4, huf_dt, flags)),
                       H(dbuf, DOUT), memcmp(dbuf, src, n) == 0);
            }
            /* X2 */
            memset(huf_dt, 0, DTBYTES);
            huf_dt[0] = (U32)(maxTableLog - 1) * 0x01000001u;
            memset(huf_dwksp, PAT_WK, HUF_DECOMPRESS_WORKSPACE_SIZE);
            rc = API(HUF_readDTableX2_wksp)(huf_dt, cbuf + O_HDR, hdrSz, huf_dwksp,
                                            HUF_DECOMPRESS_WORKSPACE_SIZE, flags);
            printf("  HUF_readDTableX2_wksp(mtl=%u)=%lld dtH=%016llx\n", maxTableLog, R(rc),
                   H(huf_dt, DTBYTES));
            if (!HUF_isError(rc) && !HUF_isError(c1) && c1 > 0) {
                memset(dbuf, PAT_DST, DOUT);
                printf("    HUF_decompress1X_usingDTable(X2)=%lld outH=%016llx match=%d\n",
                       R(API(HUF_decompress1X_usingDTable)(dbuf, n, cbuf + O_C1, c1, huf_dt, flags)),
                       H(dbuf, DOUT), memcmp(dbuf, src, n) == 0);
            }
            if (!HUF_isError(rc) && !HUF_isError(c4) && c4 > 0) {
                memset(dbuf, PAT_DST, DOUT);
                printf("    HUF_decompress4X_usingDTable(X2)=%lld outH=%016llx match=%d\n",
                       R(API(HUF_decompress4X_usingDTable)(dbuf, n, cbuf + O_C4, c4, huf_dt, flags)),
                       H(dbuf, DOUT), memcmp(dbuf, src, n) == 0);
            }
            /* too small workspace */
            memset(huf_dt, 0, DTBYTES);
            huf_dt[0] = (U32)(maxTableLog - 1) * 0x01000001u;
            memset(huf_dwksp, PAT_WK, HUF_DECOMPRESS_WORKSPACE_SIZE);
            printf("  HUF_readDTableX1_wksp(wk=8)=%lld\n",
                   R(API(HUF_readDTableX1_wksp)(huf_dt, cbuf + O_HDR, hdrSz, huf_dwksp, 8, flags)));
            printf("  HUF_readDTableX2_wksp(wk=8)=%lld\n",
                   R(API(HUF_readDTableX2_wksp)(huf_dt, cbuf + O_HDR, hdrSz, huf_dwksp, 8, flags)));
        }
    }

    /* full HUF block = header || bitstream, through the *_DCtx_wksp entry points */
    if (!HUF_isError(c1) && c1 > 0) {
        unsigned char* blk = cbuf + O_BLK;
        size_t blkSz = hdrSz + c1;
        memcpy(blk, cbuf + O_HDR, hdrSz);
        memcpy(blk + hdrSz, cbuf + O_C1, c1);
        memset(huf_dt, 0, DTBYTES); huf_dt[0] = (U32)(HUF_TABLELOG_MAX - 1) * 0x01000001u;
        memset(huf_dwksp, PAT_WK, HUF_DECOMPRESS_WORKSPACE_SIZE);
        memset(dbuf, PAT_DST, DOUT);
        printf("  HUF_decompress1X1_DCtx_wksp=%lld outH=%016llx\n",
               R(API(HUF_decompress1X1_DCtx_wksp)(huf_dt, dbuf, n, blk, blkSz, huf_dwksp,
                                                  HUF_DECOMPRESS_WORKSPACE_SIZE, flags)),
               H(dbuf, DOUT));
        memset(huf_dt, 0, DTBYTES); huf_dt[0] = (U32)(HUF_TABLELOG_MAX - 1) * 0x01000001u;
        memset(huf_dwksp, PAT_WK, HUF_DECOMPRESS_WORKSPACE_SIZE);
        memset(dbuf, PAT_DST, DOUT);
        printf("  HUF_decompress1X2_DCtx_wksp=%lld outH=%016llx\n",
               R(API(HUF_decompress1X2_DCtx_wksp)(huf_dt, dbuf, n, blk, blkSz, huf_dwksp,
                                                  HUF_DECOMPRESS_WORKSPACE_SIZE, flags)),
               H(dbuf, DOUT));
        memset(huf_dt, 0, DTBYTES); huf_dt[0] = (U32)(HUF_TABLELOG_MAX - 1) * 0x01000001u;
        memset(huf_dwksp, PAT_WK, HUF_DECOMPRESS_WORKSPACE_SIZE);
        memset(dbuf, PAT_DST, DOUT);
        printf("  HUF_decompress1X_DCtx_wksp=%lld outH=%016llx\n",
               R(API(HUF_decompress1X_DCtx_wksp)(huf_dt, dbuf, n, blk, blkSz, huf_dwksp,
                                                 HUF_DECOMPRESS_WORKSPACE_SIZE, flags)),
               H(dbuf, DOUT));
        /* truncated block -> error path */
        memset(huf_dt, 0, DTBYTES); huf_dt[0] = (U32)(HUF_TABLELOG_MAX - 1) * 0x01000001u;
        memset(huf_dwksp, PAT_WK, HUF_DECOMPRESS_WORKSPACE_SIZE);
        memset(dbuf, PAT_DST, DOUT);
        printf("  HUF_decompress1X_DCtx_wksp(trunc)=%lld outH=%016llx\n",
               R(API(HUF_decompress1X_DCtx_wksp)(huf_dt, dbuf, n, blk, blkSz - 1, huf_dwksp,
                                                 HUF_DECOMPRESS_WORKSPACE_SIZE, flags)),
               H(dbuf, DOUT));
    }
    if (!HUF_isError(c4) && c4 > 0) {
        unsigned char* blk = cbuf + O_BLK;
        size_t blkSz = hdrSz + c4;
        memcpy(blk, cbuf + O_HDR, hdrSz);
        memcpy(blk + hdrSz, cbuf + O_C4, c4);
        memset(huf_dt, 0, DTBYTES); huf_dt[0] = (U32)(HUF_TABLELOG_MAX - 1) * 0x01000001u;
        memset(huf_dwksp, PAT_WK, HUF_DECOMPRESS_WORKSPACE_SIZE);
        memset(dbuf, PAT_DST, DOUT);
        printf("  HUF_decompress4X_hufOnly_wksp=%lld outH=%016llx\n",
               R(API(HUF_decompress4X_hufOnly_wksp)(huf_dt, dbuf, n, blk, blkSz, huf_dwksp,
                                                    HUF_DECOMPRESS_WORKSPACE_SIZE, flags)),
               H(dbuf, DOUT));
        memset(huf_dt, 0, DTBYTES); huf_dt[0] = (U32)(HUF_TABLELOG_MAX - 1) * 0x01000001u;
        memset(huf_dwksp, PAT_WK, HUF_DECOMPRESS_WORKSPACE_SIZE);
        memset(dbuf, PAT_DST, DOUT);
        printf("  HUF_decompress4X_hufOnly_wksp(trunc)=%lld outH=%016llx\n",
               R(API(HUF_decompress4X_hufOnly_wksp)(huf_dt, dbuf, n, blk, blkSz - 1, huf_dwksp,
                                                    HUF_DECOMPRESS_WORKSPACE_SIZE, flags)),
               H(dbuf, DOUT));
    }

    /* repeat-mode compressors */
    for (repeat = 0; repeat <= 2; repeat++) {
        int rp = repeat;
        memcpy(huf_ct2, huf_ct, CTBYTES);
        memset(huf_wksp, PAT_WK, HUF_WORKSPACE_SIZE);
        memset(cbuf + O_REP, PAT_DST, CAP_C);
        printf("  HUF_compress1X_repeat(rp=%d)=%lld rp'=%d cH=%016llx ctH=%016llx\n", repeat,
               R(API(HUF_compress1X_repeat)(cbuf + O_REP, CAP_C, src, n, mv, huffLog,
                                            huf_wksp, HUF_WORKSPACE_SIZE, huf_ct2, &rp, flags)),
               rp, H(cbuf + O_REP, CAP_C), H(huf_ct2, CTBYTES));
        rp = repeat;
        memcpy(huf_ct2, huf_ct, CTBYTES);
        memset(huf_wksp, PAT_WK, HUF_WORKSPACE_SIZE);
        memset(cbuf + O_REP, PAT_DST, CAP_C);
        printf("  HUF_compress4X_repeat(rp=%d)=%lld rp'=%d cH=%016llx ctH=%016llx\n", repeat,
               R(API(HUF_compress4X_repeat)(cbuf + O_REP, CAP_C, src, n, mv, huffLog,
                                            huf_wksp, HUF_WORKSPACE_SIZE, huf_ct2, &rp, flags)),
               rp, H(cbuf + O_REP, CAP_C), H(huf_ct2, CTBYTES));
    }
}

static void sec_huf_scalar(void) {
    size_t i; unsigned a, b;
    printf("== HUF scalar ==\n");
    for (i = 0; i < NERR; i++)
        printf("HUF_isError[%lld]=%u name=%s\n", R(kErrCodes[i]),
               API(HUF_isError)(kErrCodes[i]), API(HUF_getErrorName)(kErrCodes[i]));
    for (i = 0; i < NSIZES; i++)
        printf("HUF_compressBound(%llu)=%lld\n", (unsigned long long)kSizes[i],
               R(API(HUF_compressBound)(kSizes[i])));
    /* HUF_minTableLog(0) evaluates ZSTD_highbit32(0) which is UB -> start at 1 */
    for (a = 1; a <= 260; a++)
        printf("HUF_minTableLog(%u)=%u\n", a, API(HUF_minTableLog)(a));
    for (i = 0; i < NSIZES; i++)
        for (a = 0; a < NSIZES; a++)
            printf("HUF_selectDecoder(%llu,%llu)=%u\n", (unsigned long long)kSizes[i],
                   (unsigned long long)kSizes[a],
                   API(HUF_selectDecoder)(kSizes[i], kSizes[a]));
    /* HUF_cardinality over synthetic count tables */
    for (a = 0; a < 40; a++) {
        unsigned k;
        rs(0x1234ULL + a);
        for (k = 0; k < 1024; k++) cnt[k] = 0;
        for (k = 0; k < 256; k++) cnt[k] = (r32() % 4) ? 0 : (r32() % 1000);
        for (b = 0; b <= 255; b += 15)
            printf("HUF_cardinality(seed=%u,mv=%u)=%u\n", a, b, API(HUF_cardinality)(cnt, b));
    }
    printf("SECTION huf_scalar calls=%llu\n", g_calls);
    fflush(stdout);
}

static void sec_huf_roundtrip(void) {
    unsigned c, m, t;
    size_t i;
    printf("== HUF roundtrip ==\n");
    for (c = 0; c < NCORPUS; c++)
        for (i = 0; i < NHUFSZ; i++)
            for (m = 0; m < NMAXV; m++)
                for (t = 0; t < NHUFLOG; t++)
                    huf_roundtrip((int)c, kHufSizes[i], kMaxV[m], kHufLog[t], 0);
    printf("SECTION huf_roundtrip calls=%llu\n", g_calls);
    fflush(stdout);
}

static void sec_huf_flags(void) {
    int flags;
    printf("== HUF flags sweep ==\n");
    for (flags = 0; flags < 64; flags++) {
        huf_roundtrip(0, 1000, 255, 11, flags);
        huf_roundtrip(0, 17000, 63, 12, flags);
        huf_roundtrip(2, 300, 15, 8, flags);
        huf_roundtrip(4, 131072, 255, 11, flags);
        huf_roundtrip(5, 64, 3, 5, flags);
    }
    printf("SECTION huf_flags calls=%llu\n", g_calls);
    fflush(stdout);
}

static void sec_huf_corrupt(void) {
    static const size_t rsz[] = { 0, 1, 2, 3, 4, 6, 10, 20, 40, 80, 129, 200, 512, 1000, 4096 };
    const size_t nrsz = sizeof(rsz) / sizeof(rsz[0]);
    const size_t CTBYTES = HUF_CTABLE_SIZE_ST(255) * sizeof(HUF_CElt);
    const size_t DTBYTES = HUF_DTABLE_SIZE(HUF_TABLELOG_MAX) * sizeof(HUF_DTable);
    unsigned trial, i;
    printf("== HUF corrupt ==\n");
    for (trial = 0; trial < 30; trial++) {
        for (i = 0; i < nrsz; i++) {
            size_t n = rsz[i];
            int flags;
            mk_rand(n, 0xBEEF1234ULL * (trial + 3) + n * 104729ULL);
            for (flags = 0; flags < 64; flags += 21) {
                unsigned mv = 255, hasZero = 0xFFFFFFFFu;
                U8 hw[256 + 16]; U32 rank[32], nbSym = 0, tl = 0;
                size_t rc;

                memset(huf_ct2, PAT_DST, CTBYTES);
                mv = 255; hasZero = 0xFFFFFFFFu;
                printf("HUF.cor t=%u n=%llu fl=%d readCTable=%lld mv=%u hz=%u ctH=%016llx\n",
                       trial, (unsigned long long)n, flags,
                       R(API(HUF_readCTable)(huf_ct2, &mv, rbuf, n, &hasZero)), mv, hasZero,
                       H(huf_ct2, CTBYTES));
                huf_show_hdr("cor", huf_ct2);

                memset(hw, PAT_WK, sizeof(hw)); memset(rank, PAT_WK, sizeof(rank));
                nbSym = 0; tl = 0;
                printf("  readStats=%lld nbSym=%u tl=%u hwH=%016llx rankH=%016llx\n",
                       R(API(HUF_readStats)(hw, 256, rank, &nbSym, &tl, rbuf, n)), nbSym, tl,
                       H(hw, sizeof(hw)), H(rank, sizeof(rank)));
                memset(hw, PAT_WK, sizeof(hw)); memset(rank, PAT_WK, sizeof(rank));
                memset(huf_rswksp, PAT_WK, HUF_READ_STATS_WORKSPACE_SIZE);
                nbSym = 0; tl = 0;
                printf("  readStats_wksp=%lld nbSym=%u tl=%u hwH=%016llx rankH=%016llx\n",
                       R(API(HUF_readStats_wksp)(hw, 256, rank, &nbSym, &tl, rbuf, n,
                                                 huf_rswksp, HUF_READ_STATS_WORKSPACE_SIZE, flags)),
                       nbSym, tl, H(hw, sizeof(hw)), H(rank, sizeof(rank)));

                memset(huf_dt, 0, DTBYTES);
                huf_dt[0] = (U32)(HUF_TABLELOG_MAX - 1) * 0x01000001u;
                memset(huf_dwksp, PAT_WK, HUF_DECOMPRESS_WORKSPACE_SIZE);
                rc = API(HUF_readDTableX1_wksp)(huf_dt, rbuf, n, huf_dwksp,
                                                HUF_DECOMPRESS_WORKSPACE_SIZE, flags);
                printf("  readDTableX1=%lld dtH=%016llx\n", R(rc), H(huf_dt, DTBYTES));
                if (!HUF_isError(rc)) {
                    size_t cap;
                    for (cap = 0; cap < 3; cap++) {
                        size_t caps[3]; caps[0] = 0; caps[1] = 37; caps[2] = 5000;
                        memset(dbuf, PAT_DST, 6000);
                        printf("    dec1X_usingDTable(cap=%llu)=%lld outH=%016llx\n",
                               (unsigned long long)caps[cap],
                               R(API(HUF_decompress1X_usingDTable)(dbuf, caps[cap], rbuf, n,
                                                                   huf_dt, flags)),
                               H(dbuf, 6000));
                        memset(dbuf, PAT_DST, 6000);
                        printf("    dec4X_usingDTable(cap=%llu)=%lld outH=%016llx\n",
                               (unsigned long long)caps[cap],
                               R(API(HUF_decompress4X_usingDTable)(dbuf, caps[cap], rbuf, n,
                                                                   huf_dt, flags)),
                               H(dbuf, 6000));
                    }
                }
                memset(huf_dt, 0, DTBYTES);
                huf_dt[0] = (U32)(HUF_TABLELOG_MAX - 1) * 0x01000001u;
                memset(huf_dwksp, PAT_WK, HUF_DECOMPRESS_WORKSPACE_SIZE);
                rc = API(HUF_readDTableX2_wksp)(huf_dt, rbuf, n, huf_dwksp,
                                                HUF_DECOMPRESS_WORKSPACE_SIZE, flags);
                printf("  readDTableX2=%lld dtH=%016llx\n", R(rc), H(huf_dt, DTBYTES));
                if (!HUF_isError(rc)) {
                    memset(dbuf, PAT_DST, 6000);
                    printf("    dec1X_usingDTable(X2)=%lld outH=%016llx\n",
                           R(API(HUF_decompress1X_usingDTable)(dbuf, 5000, rbuf, n, huf_dt, flags)),
                           H(dbuf, 6000));
                    memset(dbuf, PAT_DST, 6000);
                    printf("    dec4X_usingDTable(X2)=%lld outH=%016llx\n",
                           R(API(HUF_decompress4X_usingDTable)(dbuf, 5000, rbuf, n, huf_dt, flags)),
                           H(dbuf, 6000));
                }

                /* whole-block decoders straight on random data */
                {
                    size_t cap;
                    size_t caps[3]; caps[0] = 0; caps[1] = 37; caps[2] = 5000;
                    for (cap = 0; cap < 3; cap++) {
                        memset(huf_dt, 0, DTBYTES);
                        huf_dt[0] = (U32)(HUF_TABLELOG_MAX - 1) * 0x01000001u;
                        memset(huf_dwksp, PAT_WK, HUF_DECOMPRESS_WORKSPACE_SIZE);
                        memset(dbuf, PAT_DST, 6000);
                        printf("  dec1X1_DCtx(cap=%llu)=%lld outH=%016llx\n",
                               (unsigned long long)caps[cap],
                               R(API(HUF_decompress1X1_DCtx_wksp)(huf_dt, dbuf, caps[cap], rbuf, n,
                                                                  huf_dwksp,
                                                                  HUF_DECOMPRESS_WORKSPACE_SIZE,
                                                                  flags)),
                               H(dbuf, 6000));
                        memset(huf_dt, 0, DTBYTES);
                        huf_dt[0] = (U32)(HUF_TABLELOG_MAX - 1) * 0x01000001u;
                        memset(huf_dwksp, PAT_WK, HUF_DECOMPRESS_WORKSPACE_SIZE);
                        memset(dbuf, PAT_DST, 6000);
                        printf("  dec1X2_DCtx(cap=%llu)=%lld outH=%016llx\n",
                               (unsigned long long)caps[cap],
                               R(API(HUF_decompress1X2_DCtx_wksp)(huf_dt, dbuf, caps[cap], rbuf, n,
                                                                  huf_dwksp,
                                                                  HUF_DECOMPRESS_WORKSPACE_SIZE,
                                                                  flags)),
                               H(dbuf, 6000));
                        memset(huf_dt, 0, DTBYTES);
                        huf_dt[0] = (U32)(HUF_TABLELOG_MAX - 1) * 0x01000001u;
                        memset(huf_dwksp, PAT_WK, HUF_DECOMPRESS_WORKSPACE_SIZE);
                        memset(dbuf, PAT_DST, 6000);
                        printf("  dec1X_DCtx(cap=%llu)=%lld outH=%016llx\n",
                               (unsigned long long)caps[cap],
                               R(API(HUF_decompress1X_DCtx_wksp)(huf_dt, dbuf, caps[cap], rbuf, n,
                                                                 huf_dwksp,
                                                                 HUF_DECOMPRESS_WORKSPACE_SIZE,
                                                                 flags)),
                               H(dbuf, 6000));
                        memset(huf_dt, 0, DTBYTES);
                        huf_dt[0] = (U32)(HUF_TABLELOG_MAX - 1) * 0x01000001u;
                        memset(huf_dwksp, PAT_WK, HUF_DECOMPRESS_WORKSPACE_SIZE);
                        memset(dbuf, PAT_DST, 6000);
                        printf("  dec4X_hufOnly(cap=%llu)=%lld outH=%016llx\n",
                               (unsigned long long)caps[cap],
                               R(API(HUF_decompress4X_hufOnly_wksp)(huf_dt, dbuf, caps[cap], rbuf, n,
                                                                    huf_dwksp,
                                                                    HUF_DECOMPRESS_WORKSPACE_SIZE,
                                                                    flags)),
                               H(dbuf, 6000));
                    }
                }
            }
        }
    }
    printf("SECTION huf_corrupt calls=%llu\n", g_calls);
    fflush(stdout);
}

/* ===================================================================== */
/*  SECTION 6 : xxHash                                                   */
/* ===================================================================== */

static const size_t kXxhSizes[] = {
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 11, 12, 15, 16, 17, 20, 23, 24, 31, 32, 33,
    47, 48, 63, 64, 65, 96, 127, 128, 129, 200, 255, 256, 257, 511, 512, 1000,
    1024, 2048, 4095, 4096, 10000, 16384, 65535, 65536, 100000
};
#define NXXHSZ (sizeof(kXxhSizes)/sizeof(kXxhSizes[0]))
static const size_t kChunks[] = { 0, 1, 2, 3, 5, 7, 13, 15, 16, 17, 31, 32, 33, 64, 100, 4096 };
#define NCHUNKS (sizeof(kChunks)/sizeof(kChunks[0]))

static void sec_xxh(void) {
    size_t i, k;
    unsigned s;
    static const U32 seeds32[] = { 0u, 1u, 0xDEADBEEFu, 0x7FFFFFFFu, 0xFFFFFFFFu };
    static const U64 seeds64[] = { 0ULL, 1ULL, 0xDEADBEEFCAFEBABEULL, 0x123456789ABCDEF0ULL };
    void* st32 = API(ZSTD_XXH32_createState)();
    void* st32b = API(ZSTD_XXH32_createState)();
    void* st64 = API(ZSTD_XXH64_createState)();
    void* st64b = API(ZSTD_XXH64_createState)();

    printf("== XXH ==\n");
    printf("ZSTD_XXH_versionNumber=%u\n", API(ZSTD_XXH_versionNumber)());

    rs(0xABCDEF0123456789ULL);
    for (i = 0; i < MAXSRC; i++) src[i] = (unsigned char)r32();

    for (i = 0; i < NXXHSZ; i++) {
        size_t n = kXxhSizes[i];
        for (s = 0; s < 5; s++)
            printf("ZSTD_XXH32(n=%llu,seed=%08x)=%08x\n", (unsigned long long)n, seeds32[s],
                   API(ZSTD_XXH32)(src, n, seeds32[s]));
        for (s = 0; s < 4; s++)
            printf("ZSTD_XXH64(n=%llu,seed=%016llx)=%016llx\n", (unsigned long long)n,
                   (unsigned long long)seeds64[s],
                   (unsigned long long)API(ZSTD_XXH64)(src, n, seeds64[s]));
    }

    for (i = 0; i < NXXHSZ; i++) {
        size_t n = kXxhSizes[i];
        for (k = 0; k < NCHUNKS; k++) {
            size_t chunk = kChunks[k];
            size_t off = 0;
            int ec = 0, ec2 = 0;
            XXH32_canonical_t c32; XXH64_canonical_t c64;
            U32 h32, h32copy; U64 h64, h64copy;
            unsigned copied = 0;

            ec |= (int)API(ZSTD_XXH32_reset)(st32, 0x9E3779B9u);
            ec2 |= (int)API(ZSTD_XXH64_reset)(st64, 0x9E3779B97F4A7C15ULL);
            off = 0;
            while (off < n) {
                size_t take = chunk ? chunk : (n - off);
                if (take > n - off) take = n - off;
                if (take == 0) take = n - off;
                ec  |= (int)API(ZSTD_XXH32_update)(st32, src + off, take);
                ec2 |= (int)API(ZSTD_XXH64_update)(st64, src + off, take);
                off += take;
                if (!copied && off * 2 >= n) {
                    API(ZSTD_XXH32_copyState)(st32b, st32);
                    API(ZSTD_XXH64_copyState)(st64b, st64);
                    copied = 1;
                }
            }
            h32 = API(ZSTD_XXH32_digest)(st32);
            h64 = API(ZSTD_XXH64_digest)(st64);
            if (!copied) {
                API(ZSTD_XXH32_copyState)(st32b, st32);
                API(ZSTD_XXH64_copyState)(st64b, st64);
            }
            h32copy = API(ZSTD_XXH32_digest)(st32b);
            h64copy = API(ZSTD_XXH64_digest)(st64b);
            memset(&c32, PAT_DST, sizeof(c32));
            memset(&c64, PAT_DST, sizeof(c64));
            API(ZSTD_XXH32_canonicalFromHash)(&c32, h32);
            API(ZSTD_XXH64_canonicalFromHash)(&c64, h64);
            printf("XXH.stream n=%llu chunk=%llu ec=%d/%d h32=%08x h64=%016llx cp32=%08x cp64=%016llx "
                   "c32H=%016llx c64H=%016llx back32=%08x back64=%016llx\n",
                   (unsigned long long)n, (unsigned long long)chunk, ec, ec2, h32,
                   (unsigned long long)h64, h32copy, (unsigned long long)h64copy,
                   H(&c32, sizeof(c32)), H(&c64, sizeof(c64)),
                   API(ZSTD_XXH32_hashFromCanonical)(&c32),
                   (unsigned long long)API(ZSTD_XXH64_hashFromCanonical)(&c64));
        }
    }

    /* random chunk splittings */
    for (i = 0; i < NXXHSZ; i += 3) {
        size_t n = kXxhSizes[i];
        unsigned t;
        for (t = 0; t < 4; t++) {
            size_t off = 0;
            U32 h32; U64 h64;
            unsigned long long save = g_state;
            rs(0x5DEECE66DULL * (t + 1) + n);
            API(ZSTD_XXH32_reset)(st32, t);
            API(ZSTD_XXH64_reset)(st64, t);
            while (off < n) {
                size_t take = (size_t)(r32() % 97) + 1;
                if (take > n - off) take = n - off;
                API(ZSTD_XXH32_update)(st32, src + off, take);
                API(ZSTD_XXH64_update)(st64, src + off, take);
                off += take;
            }
            h32 = API(ZSTD_XXH32_digest)(st32);
            h64 = API(ZSTD_XXH64_digest)(st64);
            printf("XXH.rndsplit n=%llu t=%u h32=%08x h64=%016llx one32=%08x one64=%016llx\n",
                   (unsigned long long)n, t, h32, (unsigned long long)h64,
                   API(ZSTD_XXH32)(src, n, t), (unsigned long long)API(ZSTD_XXH64)(src, n, t));
            g_state = save;
        }
    }

    /* canonical round trip over a fixed value grid */
    for (i = 0; i < 64; i++) {
        XXH32_canonical_t c32; XXH64_canonical_t c64;
        U32 v32 = (U32)(0x01234567u * (unsigned)(i + 1));
        U64 v64 = 0x0123456789ABCDEFULL * (unsigned long long)(i + 1);
        memset(&c32, PAT_DST, sizeof(c32));
        memset(&c64, PAT_DST, sizeof(c64));
        API(ZSTD_XXH32_canonicalFromHash)(&c32, v32);
        API(ZSTD_XXH64_canonicalFromHash)(&c64, v64);
        printf("XXH.canon i=%llu v32=%08x c32H=%016llx back=%08x v64=%016llx c64H=%016llx back=%016llx\n",
               (unsigned long long)i, v32, H(&c32, sizeof(c32)),
               API(ZSTD_XXH32_hashFromCanonical)(&c32),
               (unsigned long long)v64, H(&c64, sizeof(c64)),
               (unsigned long long)API(ZSTD_XXH64_hashFromCanonical)(&c64));
    }

    printf("ZSTD_XXH32_freeState=%d\n", (int)API(ZSTD_XXH32_freeState)(st32));
    printf("ZSTD_XXH32_freeState=%d\n", (int)API(ZSTD_XXH32_freeState)(st32b));
    printf("ZSTD_XXH64_freeState=%d\n", (int)API(ZSTD_XXH64_freeState)(st64));
    printf("ZSTD_XXH64_freeState=%d\n", (int)API(ZSTD_XXH64_freeState)(st64b));
    printf("ZSTD_XXH32_freeState(NULL)=%d\n", (int)API(ZSTD_XXH32_freeState)(NULL));
    printf("ZSTD_XXH64_freeState(NULL)=%d\n", (int)API(ZSTD_XXH64_freeState)(NULL));
    printf("SECTION xxh calls=%llu\n", g_calls);
    fflush(stdout);
}

/* ===================================================================== */
/*  SECTION 7 : divsufsort / divbwt                                      */
/* ===================================================================== */

static const int kSortSizes[] = { 0, 1, 2, 3, 4, 5, 8, 16, 31, 64, 100, 257, 1000, 4096, 20000, 50000 };
#define NSORTSZ (sizeof(kSortSizes)/sizeof(kSortSizes[0]))
static const unsigned kAlpha[] = { 1, 2, 4, 16, 64, 256 };
#define NALPHA (sizeof(kAlpha)/sizeof(kAlpha[0]))

static void sec_sort(void) {
    int* SA = (int*)xalloc((50000 + 8) * sizeof(int));
    int* A  = (int*)xalloc((50000 + 8) * sizeof(int));
    int* idx = (int*)xalloc(256 * sizeof(int));
    unsigned char* U = (unsigned char*)xalloc(50000 + 8);
    unsigned a, c, pat;
    size_t i;
    printf("== SORT ==\n");
    for (pat = 0; pat < 4; pat++) {
        for (a = 0; a < NALPHA; a++) {
            unsigned alpha = kAlpha[a];
            for (i = 0; i < NSORTSZ; i++) {
                int n = kSortSizes[i];
                int rc, k;
                unsigned char nidx;
                /* build input */
                rs(0x2545F4914F6CDD1DULL * (pat + 1) + (unsigned long long)n * 31ULL + alpha);
                for (k = 0; k < n; k++) {
                    unsigned v;
                    switch (pat) {
                    case 0: v = r32() % alpha; break;
                    case 1: v = (unsigned)k % alpha; break;
                    case 2: v = ((r32() % 8) ? 0u : (r32() % alpha)); break;
                    default: v = (unsigned)((k * k + 7 * k) % alpha); break;
                    }
                    src[k] = (unsigned char)v;
                }
                memset(SA, PAT_DST, (50000 + 8) * sizeof(int));
                rc = API(divsufsort)(src, SA, n, 0);
                printf("divsufsort pat=%u alpha=%u n=%d =%d saH=%016llx allH=%016llx\n",
                       pat, alpha, n, rc, H(SA, (size_t)(n > 0 ? n : 0) * sizeof(int)),
                       H(SA, (50000 + 8) * sizeof(int)));

                memset(SA, PAT_DST, (50000 + 8) * sizeof(int));
                rc = API(divsufsort)(src, SA, n, 1);
                printf("divsufsort(omp) pat=%u alpha=%u n=%d =%d saH=%016llx\n",
                       pat, alpha, n, rc, H(SA, (size_t)(n > 0 ? n : 0) * sizeof(int)));

                /* divbwt with explicit temp array + indexes */
                memset(U, PAT_DST, 50000 + 8);
                memset(A, PAT_DST, (50000 + 8) * sizeof(int));
                memset(idx, PAT_DST, 256 * sizeof(int));
                nidx = 0xEE;
                rc = API(divbwt)(src, U, A, n, &nidx, idx, 0);
                printf("divbwt pat=%u alpha=%u n=%d pidx=%d nidx=%u uH=%016llx "
                       "idxH=%016llx allIdxH=%016llx\n",
                       pat, alpha, n, rc, (unsigned)nidx,
                       H(U, (size_t)(n > 0 ? n : 0)),
                       H(idx, (size_t)nidx * sizeof(int)),
                       H(idx, 256 * sizeof(int)));

                /* divbwt without indexes */
                memset(U, PAT_DST, 50000 + 8);
                memset(A, PAT_DST, (50000 + 8) * sizeof(int));
                rc = API(divbwt)(src, U, A, n, NULL, NULL, 0);
                printf("divbwt(noidx) pat=%u alpha=%u n=%d pidx=%d uH=%016llx\n",
                       pat, alpha, n, rc, H(U, (size_t)(n > 0 ? n : 0)));

                /* divbwt letting the library allocate the temp array */
                memset(U, PAT_DST, 50000 + 8);
                memset(idx, PAT_DST, 256 * sizeof(int));
                nidx = 0xEE;
                rc = API(divbwt)(src, U, NULL, n, &nidx, idx, 0);
                printf("divbwt(A=NULL) pat=%u alpha=%u n=%d pidx=%d nidx=%u uH=%016llx idxH=%016llx\n",
                       pat, alpha, n, rc, (unsigned)nidx, H(U, (size_t)(n > 0 ? n : 0)),
                       H(idx, (size_t)nidx * sizeof(int)));
            }
        }
    }
    /* negative / NULL argument checks */
    printf("divsufsort(n=-1)=%d\n", API(divsufsort)(src, SA, -1, 0));
    printf("divbwt(n=-1)=%d\n", API(divbwt)(src, U, A, -1, NULL, NULL, 0));
    for (c = 0; c < 3; c++) {
        memset(SA, PAT_DST, (50000 + 8) * sizeof(int));
        printf("divsufsort(NULL,%u)=%d\n", c,
               API(divsufsort)(c == 0 ? NULL : src, c == 1 ? NULL : SA, 10, 0));
    }
    free(SA); free(A); free(idx); free(U);
    printf("SECTION sort calls=%llu\n", g_calls);
    fflush(stdout);
}

/* ===================================================================== */
/*  SECTION 8 : misc                                                     */
/* ===================================================================== */

static void sec_misc(void) {
    int code;
    size_t i;
    unsigned trial;
    printf("== MISC ==\n");
    printf("ZSTD_versionNumber=%u ZSTD_versionString=%s\n",
           API(ZSTD_versionNumber)(), API(ZSTD_versionString)());
    for (code = 0; code <= 130; code++)
        printf("ERR_getErrorString(%d)=%s | ZSTD_getErrorString(%d)=%s\n",
               code, API(ERR_getErrorString)(code), code, API(ZSTD_getErrorString)(code));
    for (i = 0; i < NERR; i++)
        printf("ZSTD_isError[%lld]=%u code=%d name=%s\n", R(kErrCodes[i]),
               API(ZSTD_isError)(kErrCodes[i]), API(ZSTD_getErrorCode)(kErrCodes[i]),
               API(ZSTD_getErrorName)(kErrCodes[i]));

    /* random buffers through the frame/block header parsers */
    for (trial = 0; trial < 200; trial++) {
        static const size_t bs[] = { 0, 1, 2, 3, 4, 5, 6, 8, 9, 13, 18, 30, 64 };
        const size_t nbs = sizeof(bs) / sizeof(bs[0]);
        size_t j;
        mk_rand(64, 0x1BADB002ULL * (trial + 1));
        for (j = 0; j < nbs; j++) {
            blockProperties_t bp;
            memset(&bp, PAT_WK, sizeof(bp));
            printf("ZSTD_getcBlockSize t=%u n=%llu =%lld bt=%d last=%u orig=%u\n",
                   trial, (unsigned long long)bs[j],
                   R(API(ZSTD_getcBlockSize)(rbuf, bs[j], &bp)),
                   (int)bp.blockType, bp.lastBlock, bp.origSize);
            printf("  ZSTD_frameHeaderSize=%lld isFrame=%u isSkippable=%u\n",
                   R(API(ZSTD_frameHeaderSize)(rbuf, bs[j])),
                   API(ZSTD_isFrame)(rbuf, bs[j]), API(ZSTD_isSkippableFrame)(rbuf, bs[j]));
        }
    }
    /* real frames + magic-number prefixed buffers */
    {
        size_t csz;
        rs(0x77777777ULL);
        for (i = 0; i < 20000; i++) src[i] = (unsigned char)(r32() & 7);
        csz = API(ZSTD_compress)(cbuf, MAXDST, src, 20000, 3);
        printf("ZSTD_compress=%lld\n", R(csz));
        if (!ZSTD_isError(csz)) {
            for (i = 0; i <= 20 && i <= csz; i++)
                printf("real frame prefix=%llu frameHeaderSize=%lld isFrame=%u isSkippable=%u\n",
                       (unsigned long long)i, R(API(ZSTD_frameHeaderSize)(cbuf, i)),
                       API(ZSTD_isFrame)(cbuf, i), API(ZSTD_isSkippableFrame)(cbuf, i));
        }
        /* synthetic skippable frames */
        for (i = 0; i < 16; i++) {
            unsigned char sk[32];
            unsigned magic = 0x184D2A50u + (unsigned)i;
            memset(sk, PAT_DST, sizeof(sk));
            sk[0] = (unsigned char)(magic & 0xFF);
            sk[1] = (unsigned char)((magic >> 8) & 0xFF);
            sk[2] = (unsigned char)((magic >> 16) & 0xFF);
            sk[3] = (unsigned char)((magic >> 24) & 0xFF);
            sk[4] = 4; sk[5] = 0; sk[6] = 0; sk[7] = 0;
            printf("skippable magic=%08x isFrame=%u isSkippable=%u fhs=%lld\n", magic,
                   API(ZSTD_isFrame)(sk, 12), API(ZSTD_isSkippableFrame)(sk, 12),
                   R(API(ZSTD_frameHeaderSize)(sk, 12)));
        }
    }
    printf("SECTION misc calls=%llu\n", g_calls);
    fflush(stdout);
}

/* ===================================================================== */
/*  SECTION 9 : ZSTD_buildFSETable                                       */
/* ===================================================================== */

static void sec_buildfsetable(void) {
    static U64 dt[1 + (1 << MaxFSELog) + 8];
    static U32 wksp[(ZSTD_BUILD_FSE_TABLE_WKSP_SIZE + 3) / 4 + 64];
    U32 baseValue[MaxSeq + 1];
    U8  nbAdd[MaxSeq + 1];
    static const unsigned mvs[] = { 1, 3, 8, 20, 35, 52 };
    static const unsigned tls[] = { 5, 6, 7, 8, 9 };
    unsigned mi, ti, c, bmi2;
    size_t si;
    unsigned k;
    printf("== ZSTD_buildFSETable ==\n");
    for (k = 0; k <= MaxSeq; k++) { baseValue[k] = k * 3u + 1u; nbAdd[k] = (U8)(k % 17); }

    for (c = 0; c < NCORPUS; c++) {
        for (si = 0; si < NFSESZ; si++) {
            size_t n = kFseSizes[si];
            if (n < 8) continue;
            for (mi = 0; mi < sizeof(mvs) / sizeof(mvs[0]); mi++) {
                unsigned mv = mvs[mi];
                unsigned mvOut;
                size_t rc;
                mk_src((int)c, n, mv);
                memset(cnt, 0, 1024 * sizeof(unsigned));
                mvOut = mv;
                if (HIST_isError(API(HIST_count)(cnt, &mvOut, src, n))) continue;
                for (ti = 0; ti < sizeof(tls) / sizeof(tls[0]); ti++) {
                    unsigned tl = tls[ti];
                    memset(norm, 0, 1024 * sizeof(short));
                    rc = API(FSE_normalizeCount)(norm, tl, cnt, n, mvOut, 0);
                    if (FSE_isError(rc) || rc == 0) continue;
                    for (bmi2 = 0; bmi2 <= 1; bmi2++) {
                        size_t dtBytes = (size_t)(1 + (1u << tl)) * 8u;
                        memset(dt, PAT_DST, sizeof(dt));
                        memset(wksp, PAT_WK, sizeof(wksp));
                        API(ZSTD_buildFSETable)(dt, norm, mvOut, baseValue, nbAdd, tl,
                                                wksp, ZSTD_BUILD_FSE_TABLE_WKSP_SIZE, (int)bmi2);
                        printf("ZSTD_buildFSETable c=%u n=%llu mv=%u tl=%u bmi2=%u dtH=%016llx allH=%016llx\n",
                               c, (unsigned long long)n, mvOut, tl, bmi2,
                               H(dt, dtBytes), H(dt, sizeof(dt)));
                    }
                }
            }
        }
    }
    printf("SECTION buildfsetable calls=%llu\n", g_calls);
    fflush(stdout);
}

/* ===================================================================== */
/*  SECTION 10 : ZSTD_decodeSeqHeaders (needs a real, warmed-up DCtx)    */
/* ===================================================================== */

static void sec_decodeseqheaders(void) {
    void* dctx = API(ZSTD_createDCtx)();
    size_t csz, dsz;
    unsigned trial;
    size_t i;
    printf("== ZSTD_decodeSeqHeaders ==\n");
    if (!dctx) { printf("no dctx\n"); return; }
    /* warm the DCtx up with a real frame so that its entropy tables are in a
     * fully defined state before we start feeding it garbage */
    rs(0x5A5A5A5AULL);
    for (i = 0; i < 40000; i++) src[i] = (unsigned char)((r32() % 5) ? (r32() & 15) : r32());
    csz = API(ZSTD_compress)(cbuf, MAXDST, src, 40000, 5);
    dsz = API(ZSTD_decompressDCtx)(dctx, dbuf, MAXDST, cbuf, csz);
    printf("warmup compress=%lld decompress=%lld match=%d\n", R(csz), R(dsz),
           (!ZSTD_isError(dsz) && dsz == 40000 && memcmp(dbuf, src, 40000) == 0));

    for (trial = 0; trial < 150; trial++) {
        static const size_t bs[] = { 0, 1, 2, 3, 4, 5, 7, 10, 20, 50, 120, 300, 1000 };
        const size_t nbs = sizeof(bs) / sizeof(bs[0]);
        size_t j;
        mk_rand(1000, 0x2BADF00DULL * (trial + 1));
        for (j = 0; j < nbs; j++) {
            int nbSeq = -12345;
            printf("ZSTD_decodeSeqHeaders t=%u n=%llu =%lld nbSeq=%d\n", trial,
                   (unsigned long long)bs[j],
                   R(API(ZSTD_decodeSeqHeaders)(dctx, &nbSeq, rbuf, bs[j])), nbSeq);
        }
    }
    printf("ZSTD_freeDCtx=%lld\n", R(API(ZSTD_freeDCtx)(dctx)));
    printf("SECTION decodeseqheaders calls=%llu\n", g_calls);
    fflush(stdout);
}

/* ===================================================================== */

typedef struct { const char* name; void (*fn)(void); } esec_t;
static const esec_t kSecs[] = {
    {"fse_scalar", sec_fse_scalar}, {"fse_rle", sec_fse_rle},
    {"fse_roundtrip", sec_fse_roundtrip}, {"fse_corrupt", sec_fse_corrupt},
    {"hist", sec_hist}, {"huf_scalar", sec_huf_scalar},
    {"huf_roundtrip", sec_huf_roundtrip}, {"huf_flags", sec_huf_flags},
    {"huf_corrupt", sec_huf_corrupt}, {"xxh", sec_xxh}, {"sort", sec_sort},
    {"misc", sec_misc}, {"buildfsetable", sec_buildfsetable},
    {"decodeseqheaders", sec_decodeseqheaders},
};
#define NSEC ((int)(sizeof(kSecs)/sizeof(kSecs[0])))

int main(int argc, char** argv) {
    int i;
    setvbuf(stdout, NULL, _IOLBF, 0);
    if (argc > 1 && strcmp(argv[1], "--list") == 0) {
        for (i = 0; i < NSEC; i++) printf("%s\n", kSecs[i].name);
        return 0;
    }
    setup();
    if (argc > 1) {
        for (i = 0; i < NSEC; i++)
            if (strcmp(argv[1], kSecs[i].name) == 0) { kSecs[i].fn(); break; }
        if (i == NSEC) { fprintf(stderr, "unknown section %s\n", argv[1]); return 2; }
    } else {
        for (i = 0; i < NSEC; i++) kSecs[i].fn();
    }
    printf("TOTAL_API_CALLS %llu\n", g_calls);
    printf("ALL DONE\n");
    fflush(stdout);
    return 0;
}
