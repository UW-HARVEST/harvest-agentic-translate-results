/* verify/legacy.c
 *
 * Differential harness for the zstd *legacy* decoders (v0.1 .. v0.7), the
 * deprecated ZBUFFv0x streaming APIs, the versioned FSEv0x_/HUFv0x_ entropy
 * primitives, and the modern entry points that dispatch into legacy code.
 *
 * Build twice (against C libzstd.so and against Rust libzstd.so) and diff the
 * two traces.  Everything is deterministic: fixed-seed xorshift64 PRNG, no
 * pointers printed, output buffers memset to a known pattern before every call.
 *
 * Usage: ./legacy            -> run every phase
 *        ./legacy P1 P7 ...  -> run only the named phases
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stddef.h>

#define ZSTD_STATIC_LINKING_ONLY
#include "zstd.h"

/* ===================================================================== */
/* extern declarations (the real prototypes live in c_src/src/legacy/*.h) */
/* ===================================================================== */

/* ---- v0.1 ---- */
extern unsigned ZSTDv01_isError(size_t code);
extern size_t   ZSTDv01_decompress(void* dst, size_t maxOriginalSize, const void* src, size_t compressedSize);
extern size_t   ZSTDv01_decompressDCtx(void* ctx, void* dst, size_t maxOriginalSize, const void* src, size_t compressedSize);
extern void     ZSTDv01_findFrameSizeInfoLegacy(const void* src, size_t srcSize, size_t* cSize, unsigned long long* dBound);
extern void*    ZSTDv01_createDCtx(void);
extern size_t   ZSTDv01_freeDCtx(void* dctx);
extern size_t   ZSTDv01_resetDCtx(void* dctx);
extern size_t   ZSTDv01_nextSrcSizeToDecompress(void* dctx);
extern size_t   ZSTDv01_decompressContinue(void* dctx, void* dst, size_t maxDstSize, const void* src, size_t srcSize);

/* ---- v0.2 ---- */
extern unsigned ZSTDv02_isError(size_t code);
extern size_t   ZSTDv02_decompress(void* dst, size_t maxOriginalSize, const void* src, size_t compressedSize);
extern void     ZSTDv02_findFrameSizeInfoLegacy(const void* src, size_t srcSize, size_t* cSize, unsigned long long* dBound);
extern void*    ZSTDv02_createDCtx(void);
extern size_t   ZSTDv02_freeDCtx(void* dctx);
extern size_t   ZSTDv02_resetDCtx(void* dctx);
extern size_t   ZSTDv02_nextSrcSizeToDecompress(void* dctx);
extern size_t   ZSTDv02_decompressContinue(void* dctx, void* dst, size_t maxDstSize, const void* src, size_t srcSize);

/* ---- v0.3 ---- */
extern unsigned ZSTDv03_isError(size_t code);
extern size_t   ZSTDv03_decompress(void* dst, size_t maxOriginalSize, const void* src, size_t compressedSize);
extern void     ZSTDv03_findFrameSizeInfoLegacy(const void* src, size_t srcSize, size_t* cSize, unsigned long long* dBound);
extern void*    ZSTDv03_createDCtx(void);
extern size_t   ZSTDv03_freeDCtx(void* dctx);
extern size_t   ZSTDv03_resetDCtx(void* dctx);
extern size_t   ZSTDv03_nextSrcSizeToDecompress(void* dctx);
extern size_t   ZSTDv03_decompressContinue(void* dctx, void* dst, size_t maxDstSize, const void* src, size_t srcSize);

/* ---- v0.4 ---- (no exported ZSTDv04_isError) */
extern size_t   ZSTDv04_decompress(void* dst, size_t maxOriginalSize, const void* src, size_t compressedSize);
extern size_t   ZSTDv04_decompressDCtx(void* dctx, void* dst, size_t maxOriginalSize, const void* src, size_t compressedSize);
extern void     ZSTDv04_findFrameSizeInfoLegacy(const void* src, size_t srcSize, size_t* cSize, unsigned long long* dBound);
extern void*    ZSTDv04_createDCtx(void);
extern size_t   ZSTDv04_freeDCtx(void* dctx);
extern size_t   ZSTDv04_resetDCtx(void* dctx);
extern size_t   ZSTDv04_nextSrcSizeToDecompress(void* dctx);
extern size_t   ZSTDv04_decompressContinue(void* dctx, void* dst, size_t maxDstSize, const void* src, size_t srcSize);

/* ---- ZBUFF v0.4 ---- */
extern void*       ZBUFFv04_createDCtx(void);
extern size_t      ZBUFFv04_freeDCtx(void* dctx);
extern size_t      ZBUFFv04_decompressInit(void* dctx);
extern size_t      ZBUFFv04_decompressWithDictionary(void* dctx, const void* dict, size_t dictSize);
extern size_t      ZBUFFv04_decompressContinue(void* dctx, void* dst, size_t* maxDstSizePtr, const void* src, size_t* srcSizePtr);
extern size_t      ZBUFFv04_recommendedDInSize(void);
extern size_t      ZBUFFv04_recommendedDOutSize(void);
extern unsigned    ZBUFFv04_isError(size_t errorCode);
extern const char* ZBUFFv04_getErrorName(size_t errorCode);

/* ---- v0.5 ---- */
typedef struct { unsigned long long srcSize; unsigned windowLog;
                 unsigned contentLog, hashLog, searchLog, searchLength, targetLength;
                 int strategy; } V05Params;
extern size_t      ZSTDv05_decompress(void* dst, size_t dstCapacity, const void* src, size_t srcSize);
extern size_t      ZSTDv05_decompressDCtx(void* ctx, void* dst, size_t dstCapacity, const void* src, size_t srcSize);
extern size_t      ZSTDv05_decompress_usingDict(void* dctx, void* dst, size_t dstCapacity, const void* src, size_t srcSize, const void* dict, size_t dictSize);
extern size_t      ZSTDv05_decompress_usingPreparedDCtx(void* dctx, const void* refDCtx, void* dst, size_t dstCapacity, const void* src, size_t srcSize);
extern void        ZSTDv05_findFrameSizeInfoLegacy(const void* src, size_t srcSize, size_t* cSize, unsigned long long* dBound);
extern unsigned    ZSTDv05_isError(size_t code);
extern const char* ZSTDv05_getErrorName(size_t code);
extern void*       ZSTDv05_createDCtx(void);
extern size_t      ZSTDv05_freeDCtx(void* dctx);
extern size_t      ZSTDv05_getFrameParams(V05Params* params, const void* src, size_t srcSize);
extern size_t      ZSTDv05_decompressBegin(void* dctx);
extern size_t      ZSTDv05_decompressBegin_usingDict(void* dctx, const void* dict, size_t dictSize);
extern void        ZSTDv05_copyDCtx(void* dst, const void* src);
extern size_t      ZSTDv05_nextSrcSizeToDecompress(void* dctx);
extern size_t      ZSTDv05_decompressContinue(void* dctx, void* dst, size_t dstCapacity, const void* src, size_t srcSize);
extern size_t      ZSTDv05_decompressBlock(void* dctx, void* dst, size_t dstCapacity, const void* src, size_t srcSize);
extern size_t      ZSTDv05_sizeofDCtx(void);

/* ---- ZBUFF v0.5 ---- */
extern void*       ZBUFFv05_createDCtx(void);
extern size_t      ZBUFFv05_freeDCtx(void* dctx);
extern size_t      ZBUFFv05_decompressInit(void* dctx);
extern size_t      ZBUFFv05_decompressInitDictionary(void* dctx, const void* dict, size_t dictSize);
extern size_t      ZBUFFv05_decompressContinue(void* dctx, void* dst, size_t* dstCapacityPtr, const void* src, size_t* srcSizePtr);
extern size_t      ZBUFFv05_recommendedDInSize(void);
extern size_t      ZBUFFv05_recommendedDOutSize(void);
extern unsigned    ZBUFFv05_isError(size_t errorCode);
extern const char* ZBUFFv05_getErrorName(size_t errorCode);

/* ---- v0.6 ---- */
typedef struct { unsigned long long frameContentSize; unsigned windowLog; } V06Params;
extern size_t      ZSTDv06_decompress(void* dst, size_t dstCapacity, const void* src, size_t srcSize);
extern size_t      ZSTDv06_decompressDCtx(void* ctx, void* dst, size_t dstCapacity, const void* src, size_t srcSize);
extern size_t      ZSTDv06_decompress_usingDict(void* dctx, void* dst, size_t dstCapacity, const void* src, size_t srcSize, const void* dict, size_t dictSize);
extern size_t      ZSTDv06_decompress_usingPreparedDCtx(void* dctx, const void* refDCtx, void* dst, size_t dstCapacity, const void* src, size_t srcSize);
extern void        ZSTDv06_findFrameSizeInfoLegacy(const void* src, size_t srcSize, size_t* cSize, unsigned long long* dBound);
extern unsigned    ZSTDv06_isError(size_t code);
extern const char* ZSTDv06_getErrorName(size_t code);
extern void*       ZSTDv06_createDCtx(void);
extern size_t      ZSTDv06_freeDCtx(void* dctx);
extern size_t      ZSTDv06_getFrameParams(V06Params* p, const void* src, size_t srcSize);
extern size_t      ZSTDv06_decompressBegin(void* dctx);
extern size_t      ZSTDv06_decompressBegin_usingDict(void* dctx, const void* dict, size_t dictSize);
extern void        ZSTDv06_copyDCtx(void* dctx, const void* preparedDCtx);
extern size_t      ZSTDv06_nextSrcSizeToDecompress(void* dctx);
extern size_t      ZSTDv06_decompressContinue(void* dctx, void* dst, size_t dstCapacity, const void* src, size_t srcSize);
extern size_t      ZSTDv06_decompressBlock(void* dctx, void* dst, size_t dstCapacity, const void* src, size_t srcSize);
extern size_t      ZSTDv06_sizeofDCtx(void);

/* ---- ZBUFF v0.6 ---- */
extern void*       ZBUFFv06_createDCtx(void);
extern size_t      ZBUFFv06_freeDCtx(void* dctx);
extern size_t      ZBUFFv06_decompressInit(void* dctx);
extern size_t      ZBUFFv06_decompressInitDictionary(void* dctx, const void* dict, size_t dictSize);
extern size_t      ZBUFFv06_decompressContinue(void* dctx, void* dst, size_t* dstCapacityPtr, const void* src, size_t* srcSizePtr);
extern size_t      ZBUFFv06_recommendedDInSize(void);
extern size_t      ZBUFFv06_recommendedDOutSize(void);
extern unsigned    ZBUFFv06_isError(size_t errorCode);
extern const char* ZBUFFv06_getErrorName(size_t errorCode);

/* ---- v0.7 ---- */
typedef struct { unsigned long long frameContentSize; unsigned windowSize;
                 unsigned dictID; unsigned checksumFlag; } V07Params;
typedef void* (*V07Alloc)(void* opaque, size_t size);
typedef void  (*V07Free) (void* opaque, void* address);
typedef struct { V07Alloc customAlloc; V07Free customFree; void* opaque; } V07CustomMem;

extern unsigned long long ZSTDv07_getDecompressedSize(const void* src, size_t srcSize);
extern size_t      ZSTDv07_decompress(void* dst, size_t dstCapacity, const void* src, size_t srcSize);
extern size_t      ZSTDv07_decompressDCtx(void* ctx, void* dst, size_t dstCapacity, const void* src, size_t srcSize);
extern size_t      ZSTDv07_decompress_usingDict(void* dctx, void* dst, size_t dstCapacity, const void* src, size_t srcSize, const void* dict, size_t dictSize);
extern void*       ZSTDv07_createDDict(const void* dict, size_t dictSize);
extern size_t      ZSTDv07_freeDDict(void* ddict);
extern size_t      ZSTDv07_decompress_usingDDict(void* dctx, void* dst, size_t dstCapacity, const void* src, size_t srcSize, const void* ddict);
extern void        ZSTDv07_findFrameSizeInfoLegacy(const void* src, size_t srcSize, size_t* cSize, unsigned long long* dBound);
extern unsigned    ZSTDv07_isError(size_t code);
extern const char* ZSTDv07_getErrorName(size_t code);
extern void*       ZSTDv07_createDCtx(void);
extern void*       ZSTDv07_createDCtx_advanced(V07CustomMem customMem);
extern size_t      ZSTDv07_freeDCtx(void* dctx);
extern size_t      ZSTDv07_sizeofDCtx(const void* dctx);
extern size_t      ZSTDv07_estimateDCtxSize(void);
extern size_t      ZSTDv07_getFrameParams(V07Params* p, const void* src, size_t srcSize);
extern size_t      ZSTDv07_decompressBegin(void* dctx);
extern size_t      ZSTDv07_decompressBegin_usingDict(void* dctx, const void* dict, size_t dictSize);
extern void        ZSTDv07_copyDCtx(void* dctx, const void* preparedDCtx);
extern size_t      ZSTDv07_nextSrcSizeToDecompress(void* dctx);
extern size_t      ZSTDv07_decompressContinue(void* dctx, void* dst, size_t dstCapacity, const void* src, size_t srcSize);
extern size_t      ZSTDv07_decompressBlock(void* dctx, void* dst, size_t dstCapacity, const void* src, size_t srcSize);
extern size_t      ZSTDv07_insertBlock(void* dctx, const void* blockStart, size_t blockSize);
extern int         ZSTDv07_isSkipFrame(void* dctx);

/* ---- ZBUFF v0.7 ---- */
extern void*       ZBUFFv07_createDCtx(void);
extern void*       ZBUFFv07_createDCtx_advanced(V07CustomMem customMem);
extern size_t      ZBUFFv07_freeDCtx(void* dctx);
extern size_t      ZBUFFv07_decompressInit(void* dctx);
extern size_t      ZBUFFv07_decompressInitDictionary(void* dctx, const void* dict, size_t dictSize);
extern size_t      ZBUFFv07_decompressContinue(void* dctx, void* dst, size_t* dstCapacityPtr, const void* src, size_t* srcSizePtr);
extern size_t      ZBUFFv07_recommendedDInSize(void);
extern size_t      ZBUFFv07_recommendedDOutSize(void);
extern unsigned    ZBUFFv07_isError(size_t errorCode);
extern const char* ZBUFFv07_getErrorName(size_t errorCode);

/* ---- FSEv05 / FSEv06 / FSEv07 ---- */
#define FSE_DECLS(V) \
extern unsigned*   FSEv##V##_createDTable(unsigned tableLog); \
extern void        FSEv##V##_freeDTable(unsigned* dt); \
extern size_t      FSEv##V##_buildDTable(unsigned* dt, const short* normalizedCounter, unsigned maxSymbolValue, unsigned tableLog); \
extern size_t      FSEv##V##_buildDTable_raw(unsigned* dt, unsigned nbBits); \
extern size_t      FSEv##V##_buildDTable_rle(unsigned* dt, unsigned char symbolValue); \
extern size_t      FSEv##V##_decompress(void* dst, size_t dstCapacity, const void* cSrc, size_t cSrcSize); \
extern size_t      FSEv##V##_decompress_usingDTable(void* dst, size_t dstCapacity, const void* cSrc, size_t cSrcSize, const unsigned* dt); \
extern size_t      FSEv##V##_readNCount(short* normalizedCounter, unsigned* maxSVPtr, unsigned* tableLogPtr, const void* rBuffer, size_t rBuffSize); \
extern unsigned    FSEv##V##_isError(size_t code); \
extern const char* FSEv##V##_getErrorName(size_t code);
FSE_DECLS(05)
FSE_DECLS(06)
FSE_DECLS(07)

/* ---- HUFv05 / HUFv06 (DTableX2 = U16*, DTableX4 = U32*) ---- */
#define HUF_DECLS_56(V) \
extern size_t HUFv##V##_readDTableX2(unsigned short* DTable, const void* src, size_t srcSize); \
extern size_t HUFv##V##_readDTableX4(unsigned* DTable, const void* src, size_t srcSize); \
extern size_t HUFv##V##_decompress(void* dst, size_t dstSize, const void* cSrc, size_t cSrcSize); \
extern size_t HUFv##V##_decompress1X2(void* dst, size_t dstSize, const void* cSrc, size_t cSrcSize); \
extern size_t HUFv##V##_decompress1X4(void* dst, size_t dstSize, const void* cSrc, size_t cSrcSize); \
extern size_t HUFv##V##_decompress4X2(void* dst, size_t dstSize, const void* cSrc, size_t cSrcSize); \
extern size_t HUFv##V##_decompress4X4(void* dst, size_t dstSize, const void* cSrc, size_t cSrcSize); \
extern size_t HUFv##V##_decompress1X2_usingDTable(void* dst, size_t maxDstSize, const void* cSrc, size_t cSrcSize, const unsigned short* DTable); \
extern size_t HUFv##V##_decompress4X2_usingDTable(void* dst, size_t maxDstSize, const void* cSrc, size_t cSrcSize, const unsigned short* DTable); \
extern size_t HUFv##V##_decompress1X4_usingDTable(void* dst, size_t maxDstSize, const void* cSrc, size_t cSrcSize, const unsigned* DTable); \
extern size_t HUFv##V##_decompress4X4_usingDTable(void* dst, size_t maxDstSize, const void* cSrc, size_t cSrcSize, const unsigned* DTable);
HUF_DECLS_56(05)
HUF_DECLS_56(06)
extern unsigned    HUFv05_isError(size_t code);
extern const char* HUFv05_getErrorName(size_t code);

/* ---- HUFv07 (DTable = U32*) ---- */
extern size_t HUFv07_readDTableX2(unsigned* DTable, const void* src, size_t srcSize);
extern size_t HUFv07_readDTableX4(unsigned* DTable, const void* src, size_t srcSize);
extern size_t HUFv07_readStats(unsigned char* huffWeight, size_t hwSize, unsigned* rankStats,
                               unsigned* nbSymbolsPtr, unsigned* tableLogPtr,
                               const void* src, size_t srcSize);
extern unsigned HUFv07_selectDecoder(size_t dstSize, size_t cSrcSize);
extern size_t HUFv07_decompress(void* dst, size_t dstSize, const void* cSrc, size_t cSrcSize);
extern size_t HUFv07_decompress1X2(void* dst, size_t dstSize, const void* cSrc, size_t cSrcSize);
extern size_t HUFv07_decompress1X4(void* dst, size_t dstSize, const void* cSrc, size_t cSrcSize);
extern size_t HUFv07_decompress4X2(void* dst, size_t dstSize, const void* cSrc, size_t cSrcSize);
extern size_t HUFv07_decompress4X4(void* dst, size_t dstSize, const void* cSrc, size_t cSrcSize);
extern size_t HUFv07_decompress1X2_DCtx(unsigned* dctx, void* dst, size_t dstSize, const void* cSrc, size_t cSrcSize);
extern size_t HUFv07_decompress1X4_DCtx(unsigned* dctx, void* dst, size_t dstSize, const void* cSrc, size_t cSrcSize);
extern size_t HUFv07_decompress1X_DCtx (unsigned* dctx, void* dst, size_t dstSize, const void* cSrc, size_t cSrcSize);
extern size_t HUFv07_decompress4X2_DCtx(unsigned* dctx, void* dst, size_t dstSize, const void* cSrc, size_t cSrcSize);
extern size_t HUFv07_decompress4X4_DCtx(unsigned* dctx, void* dst, size_t dstSize, const void* cSrc, size_t cSrcSize);
extern size_t HUFv07_decompress4X_DCtx (unsigned* dctx, void* dst, size_t dstSize, const void* cSrc, size_t cSrcSize);
extern size_t HUFv07_decompress4X_hufOnly(unsigned* dctx, void* dst, size_t dstSize, const void* cSrc, size_t cSrcSize);
extern size_t HUFv07_decompress1X2_usingDTable(void* dst, size_t maxDstSize, const void* cSrc, size_t cSrcSize, const unsigned* DTable);
extern size_t HUFv07_decompress1X4_usingDTable(void* dst, size_t maxDstSize, const void* cSrc, size_t cSrcSize, const unsigned* DTable);
extern size_t HUFv07_decompress1X_usingDTable (void* dst, size_t maxDstSize, const void* cSrc, size_t cSrcSize, const unsigned* DTable);
extern size_t HUFv07_decompress4X2_usingDTable(void* dst, size_t maxDstSize, const void* cSrc, size_t cSrcSize, const unsigned* DTable);
extern size_t HUFv07_decompress4X4_usingDTable(void* dst, size_t maxDstSize, const void* cSrc, size_t cSrcSize, const unsigned* DTable);
extern size_t HUFv07_decompress4X_usingDTable (void* dst, size_t maxDstSize, const void* cSrc, size_t cSrcSize, const unsigned* DTable);
extern unsigned    HUFv07_isError(size_t code);
extern const char* HUFv07_getErrorName(size_t code);

/* ===================================================================== */
/* infrastructure                                                        */
/* ===================================================================== */

static unsigned long long g_st = 0x9E3779B97F4A7C15ULL;
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

/* rolling digest, used to fold long inner loops into one printed line */
static unsigned long long g_dg;
static void dg_reset(void) { g_dg = 1469598103934665603ULL; }
static void dg_add(unsigned long long v) {
    int i; for (i = 0; i < 8; i++) { g_dg ^= (unsigned char)(v >> (i*8)); g_dg *= 1099511628211ULL; }
}

/* result formatting: small negative size_t values are zstd error codes */
static const char* rs2(size_t rv, char* b) {
    long long s = (long long)rv;
    if (s < 0 && s > -1000) sprintf(b, "E%d", (int)(-s));
    else sprintf(b, "%llu", (unsigned long long)rv);
    return b;
}
/* output buffer with a guard region */
#define OUTCAP  ((size_t)16384)
#define GUARDSZ ((size_t)16384)
static unsigned char* g_out;      /* OUTCAP + GUARDSZ */
static unsigned char* g_ref;      /* GUARDSZ of 0xA5, for guard comparison */

static void out_clear(void) { memset(g_out, 0xA5, OUTCAP + GUARDSZ); }
static int  guard_ok(void)  { return memcmp(g_out + OUTCAP, g_ref, GUARDSZ) == 0; }

/* hash of what was produced: exactly `n` bytes where n is derived from rv */
static unsigned long long out_hash(size_t rv) {
    long long s = (long long)rv;
    size_t n = (s >= 0 && (size_t)s <= OUTCAP) ? (size_t)s : 512;
    if (n < 64) n = 64;
    return fnv(g_out, n);
}

/* ---- input corpus ---- */
#define MAXIN 4096
typedef struct { unsigned char b[MAXIN]; size_t n; } BUF;

static const unsigned char MAGIC[8][4] = {
    { 0,0,0,0 },                        /* unused index 0 */
    { 0xFD,0x2F,0xB5,0x1E },            /* v0.1: big-endian 0xFD2FB51E   */
    { 0x22,0xB5,0x2F,0xFD },            /* v0.2: LE store of 0xFD2FB522  */
    { 0x23,0xB5,0x2F,0xFD },            /* v0.3 */
    { 0x24,0xB5,0x2F,0xFD },            /* v0.4 */
    { 0x25,0xB5,0x2F,0xFD },            /* v0.5 */
    { 0x26,0xB5,0x2F,0xFD },            /* v0.6 */
    { 0x27,0xB5,0x2F,0xFD }             /* v0.7 */
};

/* biased random byte: favours small values so FSE/HUF headers look plausible */
static unsigned char bbyte(void) {
    unsigned x = r32();
    switch (x & 3) {
        case 0:  return (unsigned char)(x >> 8);          /* uniform */
        case 1:  return (unsigned char)((x >> 8) & 0x0F); /* small nibble */
        case 2:  return (unsigned char)((x >> 8) & 0x3F);
        default: return (unsigned char)(((x >> 8) & 1) ? 0 : (unsigned char)(x >> 16));
    }
}

/* 4-byte prefixes that make FSEv0x_readNCount pick tableLog == 15, i.e.
 * threshold == 32768.  That is the only value for which the C code's
 * `count >= threshold` comparison (short promoted to int) can disagree with a
 * 16-bit comparison, so injecting these into block payloads is what makes the
 * FSE table-reading paths interesting. */
static const unsigned char TL15[8][4] = {
    { 0xFA, 0xFF, 0x7F, 0x00 }, { 0x0A, 0xFF, 0xFF, 0xFF },
    { 0xFA, 0xFF, 0xFF, 0xFF }, { 0x8A, 0xFF, 0x7F, 0xFF },
    { 0x1A, 0xF0, 0xFF, 0x7F }, { 0xAA, 0xFF, 0xBF, 0x7F },
    { 0x3A, 0xFF, 0xFF, 0x7F }, { 0xCA, 0xF8, 0xFF, 0x3F }
};

/* mode 0: magic + uniform random
 * mode 1: magic + plausible frame header + block headers + literal headers
 * mode 2: magic + header + mostly zero
 * mode 3: no magic (random prefix)
 * mode 4: very short buffer
 * mode 5: like mode 1, but a tableLog-15 FSE header is injected into each
 *         block payload
 */
static void gen(BUF* o, int ver, int mode)
{
    size_t i, p = 0, cap;
    size_t want;
    memset(o->b, 0, sizeof(o->b));
    switch (mode) {
        case 4: want = (size_t)(r32() % 13); break;
        case 3: want = 4 + (size_t)(r32() % 200); break;
        default: {
            unsigned k = r32() % 100;
            if (k < 50)      want = 8 + (size_t)(r32() % 120);
            else if (k < 85) want = 8 + (size_t)(r32() % 600);
            else             want = 8 + (size_t)(r32() % (MAXIN - 8));
        }
    }
    if (want > MAXIN) want = MAXIN;
    cap = want;
    o->n = want;

    if (mode == 3) { for (i = 0; i < want; i++) o->b[i] = (unsigned char)r32(); return; }
    if (want >= 4 && ver >= 1 && ver <= 7) { memcpy(o->b, MAGIC[ver], 4); p = 4; }
    else { for (i = 0; i < want; i++) o->b[i] = (unsigned char)r32(); return; }

    if (mode == 0) { for (i = p; i < cap; i++) o->b[i] = (unsigned char)r32(); return; }
    if (mode == 2) {
        /* header byte(s) then zeros, with a couple of random pokes */
        int hx = (ver >= 4) ? 1 + (int)(r32() % 3) : 0;
        for (i = 0; i < (size_t)hx && p < cap; i++) o->b[p++] = (unsigned char)(r32() & 15);
        if (cap > 8) { size_t k = 8 + (size_t)(r32() % (cap - 8)); o->b[k] = (unsigned char)r32(); }
        return;
    }

    /* mode 1 / mode 5: structured */
    {
        int hx = (ver == 1 || ver == 2 || ver == 3) ? 0 : 1 + (int)(r32() % ((ver == 7) ? 3 : 2));
        int nb, blk;
        for (i = 0; i < (size_t)hx && p < cap; i++) o->b[p++] = (unsigned char)(r32() & 15);
        nb = 1 + (int)(r32() % 5);
        for (blk = 0; blk < nb; blk++) {
            unsigned bt = r32() & 3;
            unsigned cs = 1 + (r32() % 220);
            size_t j;
            if (p + 3 + cs > cap) break;
            o->b[p++] = (unsigned char)((bt << 6) | ((cs >> 16) & 7));
            o->b[p++] = (unsigned char)((cs >> 8) & 255);
            o->b[p++] = (unsigned char)(cs & 255);
            /* literal-section header: 2-bit flag, 2-bit size format, sizes */
            {
                unsigned lf = r32() & 3, sf = r32() & 3;
                o->b[p] = (unsigned char)((lf << 6) | (sf << 4) | (r32() & 15));
            }
            for (j = 0; j < cs; j++) o->b[p + j] = (j == 0) ? o->b[p] : bbyte();
            if (mode == 5 && cs >= 12) {
                /* drop a tableLog-15 FSE header somewhere inside the payload */
                size_t at = 1 + (size_t)(r32() % (cs - 8));
                memcpy(o->b + p + at, TL15[r32() & 7], 4);
            }
            p += cs;
        }
        /* end-of-frame-ish trailer */
        if (p + 3 <= cap) { o->b[p++] = (unsigned char)((3u << 6)); o->b[p++] = 0; o->b[p++] = 0; }
        for (; p < cap; p++) o->b[p] = bbyte();
    }
}

/* one-byte mutation of an existing buffer (corpus evolution) */
static void mutate(BUF* o)
{
    if (o->n == 0) return;
    switch (r32() & 3) {
        case 0: o->b[r32() % o->n] = (unsigned char)r32(); break;
        case 1: o->b[r32() % o->n] ^= (unsigned char)(1u << (r32() & 7)); break;
        case 2: o->b[r32() % o->n] = bbyte(); break;
        default: {
            size_t k = 4 + (r32() % (o->n > 4 ? o->n - 4 : 1));
            if (k < o->n) o->n = k;
        }
    }
}

/* phase selection */
static int g_argc; static char** g_argv;
static int phase_on(const char* name) {
    int i;
    if (g_argc <= 1) return 1;
    for (i = 1; i < g_argc; i++) if (!strcmp(g_argv[i], name)) return 1;
    return 0;
}
static long long g_calls = 0;
#define BANNER(p, t) printf("\n##### %s %s #####\n", p, t)

/* deterministic custom allocator for the *_advanced entry points */
static void* myalloc(void* opaque, size_t size) { (void)opaque; return malloc(size); }
static void  myfree (void* opaque, void* addr)  { (void)opaque; free(addr); }

/* ===================================================================== */
/* P0: constants, error tables, pure functions                           */
/* ===================================================================== */
static void phase0(void)
{
    char a[32];
    size_t i;
    static const long long codes[] = {
        0, 1, 2, 3, 5, 10, 16, 20, 30, 40, 62, 63, 64, 100, 120, 127, 128,
        1000, 100000,
        -1, -2, -3, -4, -5, -6, -7, -8, -9, -10, -11, -12, -13, -14, -15, -16,
        -17, -18, -19, -20, -22, -24, -30, -40, -44, -60, -62, -63, -64, -65,
        -70, -100, -120, -127, -128, -129, -200, -1000, -1001
    };
    BANNER("P0", "constants and pure functions");
    printf("ZBUFFv04_recommendedDInSize  %llu\n", (unsigned long long)ZBUFFv04_recommendedDInSize());
    printf("ZBUFFv04_recommendedDOutSize %llu\n", (unsigned long long)ZBUFFv04_recommendedDOutSize());
    printf("ZBUFFv05_recommendedDInSize  %llu\n", (unsigned long long)ZBUFFv05_recommendedDInSize());
    printf("ZBUFFv05_recommendedDOutSize %llu\n", (unsigned long long)ZBUFFv05_recommendedDOutSize());
    printf("ZBUFFv06_recommendedDInSize  %llu\n", (unsigned long long)ZBUFFv06_recommendedDInSize());
    printf("ZBUFFv06_recommendedDOutSize %llu\n", (unsigned long long)ZBUFFv06_recommendedDOutSize());
    printf("ZBUFFv07_recommendedDInSize  %llu\n", (unsigned long long)ZBUFFv07_recommendedDInSize());
    printf("ZBUFFv07_recommendedDOutSize %llu\n", (unsigned long long)ZBUFFv07_recommendedDOutSize());
    printf("ZSTDv05_sizeofDCtx           %llu\n", (unsigned long long)ZSTDv05_sizeofDCtx());
    printf("ZSTDv06_sizeofDCtx           %llu\n", (unsigned long long)ZSTDv06_sizeofDCtx());
    printf("ZSTDv07_estimateDCtxSize     %llu\n", (unsigned long long)ZSTDv07_estimateDCtxSize());
    g_calls += 11;

    for (i = 0; i < sizeof(codes)/sizeof(codes[0]); i++) {
        size_t c = (size_t)codes[i];
        printf("err %6lld v01=%u v02=%u v03=%u v05=%u/%s v06=%u/%s v07=%u/%s\n",
               codes[i], ZSTDv01_isError(c), ZSTDv02_isError(c), ZSTDv03_isError(c),
               ZSTDv05_isError(c), ZSTDv05_getErrorName(c),
               ZSTDv06_isError(c), ZSTDv06_getErrorName(c),
               ZSTDv07_isError(c), ZSTDv07_getErrorName(c));
        printf("errZB %6lld z04=%u/%s z05=%u/%s z06=%u/%s z07=%u/%s\n",
               codes[i], ZBUFFv04_isError(c), ZBUFFv04_getErrorName(c),
               ZBUFFv05_isError(c), ZBUFFv05_getErrorName(c),
               ZBUFFv06_isError(c), ZBUFFv06_getErrorName(c),
               ZBUFFv07_isError(c), ZBUFFv07_getErrorName(c));
        printf("errFH %6lld f05=%u/%s f06=%u/%s f07=%u/%s h05=%u/%s h07=%u/%s\n",
               codes[i], FSEv05_isError(c), FSEv05_getErrorName(c),
               FSEv06_isError(c), FSEv06_getErrorName(c),
               FSEv07_isError(c), FSEv07_getErrorName(c),
               HUFv05_isError(c), HUFv05_getErrorName(c),
               HUFv07_isError(c), HUFv07_getErrorName(c));
        g_calls += 20;
    }
    (void)a;

    /* HUFv07_selectDecoder over a grid.
     * Documented precondition: 0 < cSrcSize < dstSize <= 128 KB.
     * dstSize == 0 divides by zero inside the C implementation (SIGFPE), so it
     * is excluded here -- see the report.                                    */
    {
        static const size_t ds[] = {1,2,3,7,15,16,31,63,64,100,127,128,255,256,1000,1023,1024,4096,65536,131072};
        size_t a, b;
        for (a = 0; a < 20; a++) for (b = 0; b < 20; b++) {
            size_t d = ds[a], c = ds[b];
            if (c == 0 || d == 0 || c >= d) continue;  /* c>=d indexes algoTime[] OOB in C */
            printf("HUFv07_selectDecoder d=%llu c=%llu -> %u\n",
                   (unsigned long long)d, (unsigned long long)c, HUFv07_selectDecoder(d, c));
            g_calls++;
        }
    }

    /* create/free round trips.  Every call is sequenced explicitly: calling
     * several library functions from one printf() argument list would leave
     * their relative order (and thus a free()/use pair) unspecified.        */
    {
        void* p; size_t r, nx, fr;
        p = ZSTDv01_createDCtx(); r = ZSTDv01_resetDCtx(p);
        nx = ZSTDv01_nextSrcSizeToDecompress(p); fr = ZSTDv01_freeDCtx(p);
        printf("v01 reset=%llu next=%llu free=%llu\n", (unsigned long long)r, (unsigned long long)nx, (unsigned long long)fr);
        p = ZSTDv02_createDCtx(); r = ZSTDv02_resetDCtx(p);
        nx = ZSTDv02_nextSrcSizeToDecompress(p); fr = ZSTDv02_freeDCtx(p);
        printf("v02 reset=%llu next=%llu free=%llu\n", (unsigned long long)r, (unsigned long long)nx, (unsigned long long)fr);
        p = ZSTDv03_createDCtx(); r = ZSTDv03_resetDCtx(p);
        nx = ZSTDv03_nextSrcSizeToDecompress(p); fr = ZSTDv03_freeDCtx(p);
        printf("v03 reset=%llu next=%llu free=%llu\n", (unsigned long long)r, (unsigned long long)nx, (unsigned long long)fr);
        p = ZSTDv04_createDCtx(); r = ZSTDv04_resetDCtx(p);
        nx = ZSTDv04_nextSrcSizeToDecompress(p); fr = ZSTDv04_freeDCtx(p);
        printf("v04 reset=%llu next=%llu free=%llu\n", (unsigned long long)r, (unsigned long long)nx, (unsigned long long)fr);
        p = ZSTDv05_createDCtx(); r = ZSTDv05_decompressBegin(p);
        nx = ZSTDv05_nextSrcSizeToDecompress(p); fr = ZSTDv05_freeDCtx(p);
        printf("v05 begin=%llu next=%llu free=%llu\n", (unsigned long long)r, (unsigned long long)nx, (unsigned long long)fr);
        p = ZSTDv06_createDCtx(); r = ZSTDv06_decompressBegin(p);
        nx = ZSTDv06_nextSrcSizeToDecompress(p); fr = ZSTDv06_freeDCtx(p);
        printf("v06 begin=%llu next=%llu free=%llu\n", (unsigned long long)r, (unsigned long long)nx, (unsigned long long)fr);
        p = ZSTDv07_createDCtx(); r = ZSTDv07_decompressBegin(p);
        nx = ZSTDv07_nextSrcSizeToDecompress(p);
        {
            size_t sz = ZSTDv07_sizeofDCtx(p);
            int sk = ZSTDv07_isSkipFrame(p);
            fr = ZSTDv07_freeDCtx(p);
            printf("v07 begin=%llu next=%llu size=%llu skip=%d free=%llu\n",
                   (unsigned long long)r, (unsigned long long)nx, (unsigned long long)sz, sk, (unsigned long long)fr);
        }
        {
            V07CustomMem cm; cm.customAlloc = myalloc; cm.customFree = myfree; cm.opaque = NULL;
            int nn; size_t bg, sz;
            p = ZSTDv07_createDCtx_advanced(cm);
            nn = (p != NULL); bg = ZSTDv07_decompressBegin(p); sz = ZSTDv07_sizeofDCtx(p);
            fr = ZSTDv07_freeDCtx(p);
            printf("v07 adv nonnull=%d begin=%llu size=%llu free=%llu\n",
                   nn, (unsigned long long)bg, (unsigned long long)sz, (unsigned long long)fr);
            p = ZBUFFv07_createDCtx_advanced(cm);
            nn = (p != NULL); bg = ZBUFFv07_decompressInit(p); fr = ZBUFFv07_freeDCtx(p);
            printf("zb07 adv nonnull=%d init=%llu free=%llu\n", nn, (unsigned long long)bg, (unsigned long long)fr);
        }
        /* free(NULL) behaviour */
        printf("freeNULL v01=%llu v02=%llu v03=%llu v04=%llu v05=%llu v06=%llu v07=%llu\n",
               (unsigned long long)ZSTDv01_freeDCtx(NULL), (unsigned long long)ZSTDv02_freeDCtx(NULL),
               (unsigned long long)ZSTDv03_freeDCtx(NULL), (unsigned long long)ZSTDv04_freeDCtx(NULL),
               (unsigned long long)ZSTDv05_freeDCtx(NULL), (unsigned long long)ZSTDv06_freeDCtx(NULL),
               (unsigned long long)ZSTDv07_freeDCtx(NULL));
        /* NOTE: ZSTDv07_freeDDict(NULL) is *not* tested: the C implementation
         * dereferences ddict->refContext unconditionally and segfaults.      */
        printf("zbfreeNULL 04=%llu 05=%llu 06=%llu 07=%llu\n",
               (unsigned long long)ZBUFFv04_freeDCtx(NULL), (unsigned long long)ZBUFFv05_freeDCtx(NULL),
               (unsigned long long)ZBUFFv06_freeDCtx(NULL), (unsigned long long)ZBUFFv07_freeDCtx(NULL));
        g_calls += 40;
    }
    /* FSE createDTable / freeDTable across table logs */
    for (i = 0; i <= 14; i++) {
        unsigned* t5 = FSEv05_createDTable((unsigned)i);
        unsigned* t6 = FSEv06_createDTable((unsigned)i);
        unsigned* t7 = FSEv07_createDTable((unsigned)i);
        printf("FSE createDTable log=%llu nn=%d%d%d\n", (unsigned long long)i, t5!=NULL, t6!=NULL, t7!=NULL);
        FSEv05_freeDTable(t5); FSEv06_freeDTable(t6); FSEv07_freeDTable(t7);
        g_calls += 6;
    }
    FSEv05_freeDTable(NULL); FSEv06_freeDTable(NULL); FSEv07_freeDTable(NULL);
    printf("FSE freeDTable(NULL) ok\n");
}

/* ===================================================================== */
/* P1: one-shot legacy decompression                                     */
/* ===================================================================== */
#define NGEN 100000

static void one_shot(int ver, const BUF* in, int idx)
{
    char b1[32], b2[32], b3[32];
    size_t cS; unsigned long long dB;
    size_t rv, rv2 = 0;
    void* dctx;

    out_clear();
    switch (ver) {
        case 1: rv = ZSTDv01_decompress(g_out, OUTCAP, in->b, in->n); break;
        case 2: rv = ZSTDv02_decompress(g_out, OUTCAP, in->b, in->n); break;
        case 3: rv = ZSTDv03_decompress(g_out, OUTCAP, in->b, in->n); break;
        case 4: rv = ZSTDv04_decompress(g_out, OUTCAP, in->b, in->n); break;
        case 5: rv = ZSTDv05_decompress(g_out, OUTCAP, in->b, in->n); break;
        case 6: rv = ZSTDv06_decompress(g_out, OUTCAP, in->b, in->n); break;
        default: rv = ZSTDv07_decompress(g_out, OUTCAP, in->b, in->n); break;
    }
    printf("P1 v%d[%d] n=%llu dec=%s h=%016llx g=%d",
           ver, idx, (unsigned long long)in->n, rs2(rv, b1), out_hash(rv), guard_ok());
    g_calls++;

    /* DCtx variants that are actually exported */
    out_clear();
    if (ver == 1)      { dctx = ZSTDv01_createDCtx(); rv2 = ZSTDv01_decompressDCtx(dctx, g_out, OUTCAP, in->b, in->n); ZSTDv01_freeDCtx(dctx); }
    else if (ver == 4) { dctx = ZSTDv04_createDCtx(); rv2 = ZSTDv04_decompressDCtx(dctx, g_out, OUTCAP, in->b, in->n); ZSTDv04_freeDCtx(dctx); }
    else if (ver == 5) { dctx = ZSTDv05_createDCtx(); rv2 = ZSTDv05_decompressDCtx(dctx, g_out, OUTCAP, in->b, in->n); ZSTDv05_freeDCtx(dctx); }
    else if (ver == 6) { dctx = ZSTDv06_createDCtx(); rv2 = ZSTDv06_decompressDCtx(dctx, g_out, OUTCAP, in->b, in->n); ZSTDv06_freeDCtx(dctx); }
    else if (ver == 7) { dctx = ZSTDv07_createDCtx(); rv2 = ZSTDv07_decompressDCtx(dctx, g_out, OUTCAP, in->b, in->n); ZSTDv07_freeDCtx(dctx); }
    if (ver == 1 || ver >= 4) {
        printf(" dctx=%s h=%016llx g=%d", rs2(rv2, b2), out_hash(rv2), guard_ok());
        g_calls += 3;
    }

    cS = 0x5A5A5A5A; dB = 0xA5A5A5A5A5A5A5A5ULL;
    switch (ver) {
        case 1: ZSTDv01_findFrameSizeInfoLegacy(in->b, in->n, &cS, &dB); break;
        case 2: ZSTDv02_findFrameSizeInfoLegacy(in->b, in->n, &cS, &dB); break;
        case 3: ZSTDv03_findFrameSizeInfoLegacy(in->b, in->n, &cS, &dB); break;
        case 4: ZSTDv04_findFrameSizeInfoLegacy(in->b, in->n, &cS, &dB); break;
        case 5: ZSTDv05_findFrameSizeInfoLegacy(in->b, in->n, &cS, &dB); break;
        case 6: ZSTDv06_findFrameSizeInfoLegacy(in->b, in->n, &cS, &dB); break;
        default: ZSTDv07_findFrameSizeInfoLegacy(in->b, in->n, &cS, &dB); break;
    }
    printf(" ffsi=%s/%llu", rs2(cS, b3), (unsigned long long)dB);
    g_calls++;

    if (ver == 5) {
        V05Params p; size_t r;
        memset(&p, 0x5A, sizeof(p));
        r = ZSTDv05_getFrameParams(&p, in->b, in->n);
        printf(" fp=%s wl=%u sz=%llu cl=%u hl=%u sl=%u sL=%u tl=%u st=%d",
               rs2(r, b2), p.windowLog, (unsigned long long)p.srcSize,
               p.contentLog, p.hashLog, p.searchLog, p.searchLength, p.targetLength, p.strategy);
        g_calls++;
    } else if (ver == 6) {
        V06Params p; size_t r;
        memset(&p, 0x5A, sizeof(p));
        r = ZSTDv06_getFrameParams(&p, in->b, in->n);
        printf(" fp=%s wl=%u fcs=%llu", rs2(r, b2), p.windowLog, (unsigned long long)p.frameContentSize);
        g_calls++;
    } else if (ver == 7) {
        V07Params p; size_t r;
        memset(&p, 0x5A, sizeof(p));
        r = ZSTDv07_getFrameParams(&p, in->b, in->n);
        printf(" fp=%s ws=%u did=%u ck=%u fcs=%llu gds=%llu",
               rs2(r, b2), p.windowSize, p.dictID, p.checksumFlag,
               (unsigned long long)p.frameContentSize,
               (unsigned long long)ZSTDv07_getDecompressedSize(in->b, in->n));
        g_calls += 2;
    }
    printf("\n");
}

static void phase1(void)
{
    int ver, i;
    BUF corpus[32];
    BANNER("P1", "one-shot legacy decompression / frame info");
    for (ver = 1; ver <= 7; ver++) {
        rs(0x1000ULL + (unsigned)ver);
        for (i = 0; i < 32; i++) gen(&corpus[i], ver, 1);
        for (i = 0; i < NGEN; i++) {
            BUF in;
            int mode = i % 9;
            if (mode <= 5) gen(&in, ver, mode);
            else { in = corpus[r32() % 32]; mutate(&in); }
            one_shot(ver, &in, i);
            if (mode > 5 && (i % 37) == 0) corpus[r32() % 32] = in;
        }
    }
}

/* ===================================================================== */
/* P2: modern entry points dispatching into legacy                       */
/* ===================================================================== */
static void phase2(void)
{
    int ver, i;
    ZSTD_DStream* zds = ZSTD_createDStream();
    BANNER("P2", "modern entry points on legacy frames");
    for (ver = 1; ver <= 7; ver++) {
        rs(0x2000ULL + (unsigned)ver);
        for (i = 0; i < 14000; i++) {
            BUF in; char b1[32], b2[32];
            size_t rv, fcs2;
            unsigned long long fcs;
            gen(&in, ver, i % 6);
            out_clear();
            rv   = ZSTD_decompress(g_out, OUTCAP, in.b, in.n);
            fcs  = ZSTD_getFrameContentSize(in.b, in.n);
            fcs2 = ZSTD_findFrameCompressedSize(in.b, in.n);
            printf("P2 v%d[%d] n=%llu dec=%s h=%016llx g=%d fcs=%llu fcsz=%s",
                   ver, i, (unsigned long long)in.n, rs2(rv, b1), out_hash(rv), guard_ok(),
                   fcs, rs2(fcs2, b2));
            g_calls += 3;
            /* streaming */
            {
                size_t ir = ZSTD_initDStream(zds);
                ZSTD_inBuffer  ib; ZSTD_outBuffer ob;
                int it;
                dg_reset(); dg_add(ir);
                ib.src = in.b; ib.size = in.n; ib.pos = 0;
                out_clear();
                for (it = 0; it < 24; it++) {
                    size_t chunk = 1 + (size_t)(r32() % 97);
                    size_t r;
                    ob.dst = g_out; ob.size = (OUTCAP < chunk*64 ? OUTCAP : chunk*64); ob.pos = 0;
                    if (ib.pos >= ib.size && it > 0) break;
                    r = ZSTD_decompressStream(zds, &ob, &ib);
                    dg_add(r); dg_add(ib.pos); dg_add(ob.pos);
                    dg_add(fnv(g_out, ob.pos < 64 ? 64 : ob.pos));
                    g_calls++;
                    if (ZSTD_isError(r) || r == 0) break;
                }
                printf(" stream=%016llx g=%d", g_dg, guard_ok());
            }
            printf("\n");
        }
    }
    ZSTD_freeDStream(zds);
}

/* ===================================================================== */
/* P3: ZSTDv0x direct streaming (nextSrcSize / decompressContinue)       */
/* ===================================================================== */
static void stream_direct(int ver, const BUF* in, int idx, int detail)
{
    void* dctx = NULL;
    size_t pos = 0;
    int it;
    char b1[32], b2[32];

    switch (ver) {
        case 1: dctx = ZSTDv01_createDCtx(); ZSTDv01_resetDCtx(dctx); break;
        case 2: dctx = ZSTDv02_createDCtx(); ZSTDv02_resetDCtx(dctx); break;
        case 3: dctx = ZSTDv03_createDCtx(); ZSTDv03_resetDCtx(dctx); break;
        case 4: dctx = ZSTDv04_createDCtx(); ZSTDv04_resetDCtx(dctx); break;
        case 5: dctx = ZSTDv05_createDCtx(); ZSTDv05_decompressBegin(dctx); break;
        case 6: dctx = ZSTDv06_createDCtx(); ZSTDv06_decompressBegin(dctx); break;
        default: dctx = ZSTDv07_createDCtx(); ZSTDv07_decompressBegin(dctx); break;
    }
    dg_reset();
    for (it = 0; it < 64; it++) {
        size_t hint, avail, feed, rv;
        switch (ver) {
            case 1: hint = ZSTDv01_nextSrcSizeToDecompress(dctx); break;
            case 2: hint = ZSTDv02_nextSrcSizeToDecompress(dctx); break;
            case 3: hint = ZSTDv03_nextSrcSizeToDecompress(dctx); break;
            case 4: hint = ZSTDv04_nextSrcSizeToDecompress(dctx); break;
            case 5: hint = ZSTDv05_nextSrcSizeToDecompress(dctx); break;
            case 6: hint = ZSTDv06_nextSrcSizeToDecompress(dctx); break;
            default: hint = ZSTDv07_nextSrcSizeToDecompress(dctx); break;
        }
        g_calls++;
        dg_add(hint);
        if (detail) printf("   it=%d hint=%s", it, rs2(hint, b1));
        if (hint == 0 || (long long)hint < 0 || hint > MAXIN) { if (detail) printf(" stop\n"); break; }
        avail = in->n - pos;
        if (avail < hint) { if (detail) printf(" short\n"); break; }
        feed = hint;
        out_clear();
        switch (ver) {
            case 1: rv = ZSTDv01_decompressContinue(dctx, g_out, OUTCAP, in->b + pos, feed); break;
            case 2: rv = ZSTDv02_decompressContinue(dctx, g_out, OUTCAP, in->b + pos, feed); break;
            case 3: rv = ZSTDv03_decompressContinue(dctx, g_out, OUTCAP, in->b + pos, feed); break;
            case 4: rv = ZSTDv04_decompressContinue(dctx, g_out, OUTCAP, in->b + pos, feed); break;
            case 5: rv = ZSTDv05_decompressContinue(dctx, g_out, OUTCAP, in->b + pos, feed); break;
            case 6: rv = ZSTDv06_decompressContinue(dctx, g_out, OUTCAP, in->b + pos, feed); break;
            default: rv = ZSTDv07_decompressContinue(dctx, g_out, OUTCAP, in->b + pos, feed); break;
        }
        g_calls++;
        pos += feed;
        dg_add(rv); dg_add(out_hash(rv)); dg_add((unsigned long long)guard_ok());
        if (detail) printf(" rv=%s h=%016llx g=%d\n", rs2(rv, b2), out_hash(rv), guard_ok());
        if ((long long)rv < 0 && (long long)rv > -1000) break;
    }
    printf("P3 v%d[%d] n=%llu consumed=%llu dg=%016llx\n",
           ver, idx, (unsigned long long)in->n, (unsigned long long)pos, g_dg);
    switch (ver) {
        case 1: ZSTDv01_freeDCtx(dctx); break;
        case 2: ZSTDv02_freeDCtx(dctx); break;
        case 3: ZSTDv03_freeDCtx(dctx); break;
        case 4: ZSTDv04_freeDCtx(dctx); break;
        case 5: ZSTDv05_freeDCtx(dctx); break;
        case 6: ZSTDv06_freeDCtx(dctx); break;
        default: ZSTDv07_freeDCtx(dctx); break;
    }
}

static void phase3(void)
{
    int ver, i;
    BANNER("P3", "ZSTDv0x direct streaming");
    for (ver = 1; ver <= 7; ver++) {
        rs(0x3000ULL + (unsigned)ver);
        for (i = 0; i < 10000; i++) {
            BUF in;
            gen(&in, ver, i % 6);
            stream_direct(ver, &in, i, i < 40);
        }
    }
    /* deliberately wrong srcSize feeds: only for versions that validate */
    for (ver = 5; ver <= 7; ver++) {
        rs(0x3800ULL + (unsigned)ver);
        for (i = 0; i < 2000; i++) {
            BUF in; void* dctx; size_t hint, rv, feed; char b1[32], b2[32];
            gen(&in, ver, i % 6);
            if (ver == 5) { dctx = ZSTDv05_createDCtx(); ZSTDv05_decompressBegin(dctx); }
            else if (ver == 6) { dctx = ZSTDv06_createDCtx(); ZSTDv06_decompressBegin(dctx); }
            else { dctx = ZSTDv07_createDCtx(); ZSTDv07_decompressBegin(dctx); }
            hint = (ver == 5) ? ZSTDv05_nextSrcSizeToDecompress(dctx)
                 : (ver == 6) ? ZSTDv06_nextSrcSizeToDecompress(dctx)
                              : ZSTDv07_nextSrcSizeToDecompress(dctx);
            feed = (hint == 0) ? 0 : (hint - 1 + (size_t)(r32() % 3));
            if (feed > in.n) feed = in.n;
            out_clear();
            rv = (ver == 5) ? ZSTDv05_decompressContinue(dctx, g_out, OUTCAP, in.b, feed)
               : (ver == 6) ? ZSTDv06_decompressContinue(dctx, g_out, OUTCAP, in.b, feed)
                            : ZSTDv07_decompressContinue(dctx, g_out, OUTCAP, in.b, feed);
            printf("P3w v%d[%d] hint=%s feed=%llu rv=%s h=%016llx g=%d\n",
                   ver, i, rs2(hint, b1), (unsigned long long)feed, rs2(rv, b2),
                   out_hash(rv), guard_ok());
            g_calls += 3;
            if (ver == 5) ZSTDv05_freeDCtx(dctx);
            else if (ver == 6) ZSTDv06_freeDCtx(dctx);
            else ZSTDv07_freeDCtx(dctx);
        }
    }
}

/* ===================================================================== */
/* P4: ZBUFFv0x buffered streaming                                       */
/* ===================================================================== */
static void zbuff_run(int ver, const BUF* in, const unsigned char* dict, size_t dictSize,
                      int useDict, int idx, int detail)
{
    void* z = NULL;
    size_t pos = 0, init;
    int it;
    char b1[32];

    switch (ver) {
        case 4: z = ZBUFFv04_createDCtx(); init = ZBUFFv04_decompressInit(z);
                if (useDict) init = ZBUFFv04_decompressWithDictionary(z, dict, dictSize);
                break;
        case 5: z = ZBUFFv05_createDCtx();
                init = useDict ? ZBUFFv05_decompressInitDictionary(z, dict, dictSize) : ZBUFFv05_decompressInit(z);
                break;
        case 6: z = ZBUFFv06_createDCtx();
                init = useDict ? ZBUFFv06_decompressInitDictionary(z, dict, dictSize) : ZBUFFv06_decompressInit(z);
                break;
        default: z = ZBUFFv07_createDCtx();
                init = useDict ? ZBUFFv07_decompressInitDictionary(z, dict, dictSize) : ZBUFFv07_decompressInit(z);
                break;
    }
    g_calls += 2;
    dg_reset(); dg_add(init);
    for (it = 0; it < 48; it++) {
        size_t sSize = in->n - pos;
        size_t dCap  = 1 + (size_t)(r32() % 4096);
        size_t rv;
        size_t sIn, dIn;
        if (dCap > OUTCAP) dCap = OUTCAP;
        if (sSize > 1 + (size_t)(r32() % 300)) sSize = 1 + (size_t)(r32() % 300);
        sIn = sSize; dIn = dCap;
        out_clear();
        switch (ver) {
            case 4: rv = ZBUFFv04_decompressContinue(z, g_out, &dIn, in->b + pos, &sIn); break;
            case 5: rv = ZBUFFv05_decompressContinue(z, g_out, &dIn, in->b + pos, &sIn); break;
            case 6: rv = ZBUFFv06_decompressContinue(z, g_out, &dIn, in->b + pos, &sIn); break;
            default: rv = ZBUFFv07_decompressContinue(z, g_out, &dIn, in->b + pos, &sIn); break;
        }
        g_calls++;
        dg_add(rv); dg_add(sIn); dg_add(dIn);
        dg_add(fnv(g_out, dIn < 64 ? 64 : (dIn > OUTCAP ? OUTCAP : dIn)));
        dg_add((unsigned long long)guard_ok());
        if (detail)
            printf("   it=%d in=%llu->%llu out=%llu->%llu rv=%s h=%016llx g=%d\n",
                   it, (unsigned long long)sSize, (unsigned long long)sIn,
                   (unsigned long long)dCap, (unsigned long long)dIn, rs2(rv, b1),
                   fnv(g_out, dIn < 64 ? 64 : (dIn > OUTCAP ? OUTCAP : dIn)), guard_ok());
        if ((long long)rv < 0 && (long long)rv > -1000) break;
        if (rv == 0) break;
        if (sIn > in->n - pos) break;   /* defensive */
        pos += sIn;
        if (pos >= in->n && sIn == 0 && dIn == 0) break;
    }
    printf("P4 v%d[%d] dict=%d n=%llu consumed=%llu init=%s dg=%016llx\n",
           ver, idx, useDict, (unsigned long long)in->n, (unsigned long long)pos,
           rs2(init, b1), g_dg);
    switch (ver) {
        case 4: ZBUFFv04_freeDCtx(z); break;
        case 5: ZBUFFv05_freeDCtx(z); break;
        case 6: ZBUFFv06_freeDCtx(z); break;
        default: ZBUFFv07_freeDCtx(z); break;
    }
}

static unsigned char g_dict[4096];
static const size_t g_dictSizes[6] = { 0, 1, 7, 64, 1000, 4096 };

static void phase4(void)
{
    int ver, i;
    BANNER("P4", "ZBUFFv0x buffered streaming");
    for (ver = 4; ver <= 7; ver++) {
        rs(0x4000ULL + (unsigned)ver);
        for (i = 0; i < 6000; i++) {
            BUF in;
            int ud = (i % 3) == 2;
            size_t ds = g_dictSizes[i % 6];
            gen(&in, ver, i % 6);
            zbuff_run(ver, &in, g_dict, ds, ud, i, i < 25);
        }
        /* also drive ZBUFF with frames of the *other* legacy versions */
        for (i = 0; i < 2000; i++) {
            BUF in;
            gen(&in, 1 + (int)(r32() % 7), i % 6);
            zbuff_run(ver, &in, g_dict, 64, 0, 100000 + i, 0);
        }
    }
}

/* ===================================================================== */
/* P5: dictionary variants                                               */
/* ===================================================================== */
static void phase5(void)
{
    int ver, i;
    BANNER("P5", "dictionary variants");
    for (ver = 5; ver <= 7; ver++) {
        rs(0x5000ULL + (unsigned)ver);
        for (i = 0; i < 10000; i++) {
            BUF in; char b1[32], b2[32], b3[32];
            size_t ds = g_dictSizes[i % 6];
            void* dctx; void* ref;
            size_t r1, r2 = 0, r3 = 0;
            gen(&in, ver, i % 6);

            out_clear();
            if (ver == 5) { dctx = ZSTDv05_createDCtx(); r1 = ZSTDv05_decompress_usingDict(dctx, g_out, OUTCAP, in.b, in.n, g_dict, ds); }
            else if (ver == 6) { dctx = ZSTDv06_createDCtx(); r1 = ZSTDv06_decompress_usingDict(dctx, g_out, OUTCAP, in.b, in.n, g_dict, ds); }
            else { dctx = ZSTDv07_createDCtx(); r1 = ZSTDv07_decompress_usingDict(dctx, g_out, OUTCAP, in.b, in.n, g_dict, ds); }
            printf("P5 v%d[%d] ds=%llu ud=%s h=%016llx g=%d", ver, i,
                   (unsigned long long)ds, rs2(r1, b1), out_hash(r1), guard_ok());
            g_calls++;

            /* decompressBegin_usingDict + copyDCtx + prepared-DCtx path */
            if (ver == 5) {
                ref = ZSTDv05_createDCtx();
                r2 = ZSTDv05_decompressBegin_usingDict(ref, g_dict, ds);
                ZSTDv05_copyDCtx(dctx, ref);
                out_clear();
                r3 = ZSTDv05_decompress_usingPreparedDCtx(dctx, ref, g_out, OUTCAP, in.b, in.n);
                ZSTDv05_freeDCtx(ref);
            } else if (ver == 6) {
                ref = ZSTDv06_createDCtx();
                r2 = ZSTDv06_decompressBegin_usingDict(ref, g_dict, ds);
                ZSTDv06_copyDCtx(dctx, ref);
                out_clear();
                r3 = ZSTDv06_decompress_usingPreparedDCtx(dctx, ref, g_out, OUTCAP, in.b, in.n);
                ZSTDv06_freeDCtx(ref);
            } else {
                ref = ZSTDv07_createDCtx();
                r2 = ZSTDv07_decompressBegin_usingDict(ref, g_dict, ds);
                ZSTDv07_copyDCtx(dctx, ref);
                out_clear();
                r3 = ZSTDv07_decompressDCtx(dctx, g_out, OUTCAP, in.b, in.n);
                ZSTDv07_freeDCtx(ref);
            }
            printf(" beginDict=%s prep=%s h=%016llx g=%d",
                   rs2(r2, b2), rs2(r3, b3), out_hash(r3), guard_ok());
            g_calls += 3;

            if (ver == 7) {
                void* dd = ZSTDv07_createDDict(g_dict, ds);
                size_t r4, fd;
                int nn = (dd != NULL), sk;
                unsigned long long hh;
                int gk;
                out_clear();
                r4 = ZSTDv07_decompress_usingDDict(dctx, g_out, OUTCAP, in.b, in.n, dd);
                hh = out_hash(r4); gk = guard_ok();
                sk = ZSTDv07_isSkipFrame(dctx);
                fd = ZSTDv07_freeDDict(dd);
                printf(" ddict_nn=%d ud2=%s h=%016llx g=%d free=%llu skip=%d",
                       nn, rs2(r4, b2), hh, gk, (unsigned long long)fd, sk);
                g_calls += 4;
            }
            printf("\n");
            if (ver == 5) ZSTDv05_freeDCtx(dctx);
            else if (ver == 6) ZSTDv06_freeDCtx(dctx);
            else ZSTDv07_freeDCtx(dctx);
        }
    }
}

/* ===================================================================== */
/* P6: block-level entry points                                          */
/* ===================================================================== */
static void phase6(void)
{
    int ver, i;
    BANNER("P6", "block-level entry points");
    for (ver = 5; ver <= 7; ver++) {
        rs(0x6000ULL + (unsigned)ver);
        for (i = 0; i < 16000; i++) {
            BUF in; char b1[32], b2[32];
            void* dctx; size_t r0, r1, blen;
            /* raw block payload, no magic */
            gen(&in, ver, 3);
            blen = in.n;
            if (blen > 1024) blen = 1024;   /* v0.5/0.6 max block size is 128 KB; keep it small */
            if (ver == 5) { dctx = ZSTDv05_createDCtx(); r0 = ZSTDv05_decompressBegin(dctx); }
            else if (ver == 6) { dctx = ZSTDv06_createDCtx(); r0 = ZSTDv06_decompressBegin(dctx); }
            else { dctx = ZSTDv07_createDCtx(); r0 = ZSTDv07_decompressBegin(dctx); }
            out_clear();
            if (ver == 5) r1 = ZSTDv05_decompressBlock(dctx, g_out, OUTCAP, in.b, blen);
            else if (ver == 6) r1 = ZSTDv06_decompressBlock(dctx, g_out, OUTCAP, in.b, blen);
            else r1 = ZSTDv07_decompressBlock(dctx, g_out, OUTCAP, in.b, blen);
            printf("P6 v%d[%d] blen=%llu begin=%s blk=%s h=%016llx g=%d",
                   ver, i, (unsigned long long)blen, rs2(r0, b1), rs2(r1, b2),
                   out_hash(r1), guard_ok());
            g_calls += 2;
            if (ver == 7) {
                size_t r2 = ZSTDv07_insertBlock(dctx, in.b, blen);
                printf(" ins=%s skip=%d sz=%llu", rs2(r2, b1), ZSTDv07_isSkipFrame(dctx),
                       (unsigned long long)ZSTDv07_sizeofDCtx(dctx));
                g_calls += 3;
            }
            printf("\n");
            if (ver == 5) ZSTDv05_freeDCtx(dctx);
            else if (ver == 6) ZSTDv06_freeDCtx(dctx);
            else ZSTDv07_freeDCtx(dctx);
        }
    }
}

/* ===================================================================== */
/* P7: FSEv05 / FSEv06 / FSEv07 direct                                   */
/* ===================================================================== */
#define FSE_TABLE_U32 (1u + (1u<<14))

static void phase7(void)
{
    static short nc[4096];
    unsigned* dt5 = (unsigned*)malloc(FSE_TABLE_U32 * sizeof(unsigned) + 4096);
    unsigned* dt6 = (unsigned*)malloc(FSE_TABLE_U32 * sizeof(unsigned) + 4096);
    unsigned* dt7 = (unsigned*)malloc(FSE_TABLE_U32 * sizeof(unsigned) + 4096);
    int i;
    BANNER("P7", "FSEv0x direct");

    /* readNCount + buildDTable + decompress_usingDTable on fuzzed headers */
    rs(0x7001);
    for (i = 0; i < 40000; i++) {
        BUF in; char b1[32], b2[32], b3[32];
        unsigned maxSV, tlog;
        size_t r5, r6, r7;
        size_t bd5, bd6, bd7;
        gen(&in, 0, 3);           /* pure random, no magic */
        if (in.n > 512) in.n = 512;

        if (getenv("DUMPCASE") && i == atoi(getenv("DUMPCASE"))) {
            size_t q; fprintf(stderr, "CASE %d n=%zu bytes:", i, in.n);
            for (q = 0; q < in.n; q++) fprintf(stderr, " %02x", in.b[q]);
            fprintf(stderr, "\n");
        }
        memset(nc, 0, sizeof(nc)); maxSV = 255; tlog = 12;
        r5 = FSEv05_readNCount(nc, &maxSV, &tlog, in.b, in.n);
        printf("P7 fse05[%d] n=%llu rnc=%s msv=%u tl=%u nch=%016llx",
               i, (unsigned long long)in.n, rs2(r5, b1), maxSV, tlog, fnv(nc, 512));
        bd5 = ((long long)r5 < 0) ? r5 : FSEv05_buildDTable(dt5, nc, maxSV, tlog);
        printf(" bd=%s", rs2(bd5, b2));
        if ((long long)bd5 >= 0) {
            size_t d;
            out_clear();
            d = FSEv05_decompress_usingDTable(g_out, OUTCAP, in.b, in.n, dt5);
            printf(" dud=%s h=%016llx g=%d", rs2(d, b3), out_hash(d), guard_ok());
            g_calls++;
        }
        printf("\n");
        g_calls += 2;

        memset(nc, 0, sizeof(nc)); maxSV = 255; tlog = 12;
        r6 = FSEv06_readNCount(nc, &maxSV, &tlog, in.b, in.n);
        printf("P7 fse06[%d] n=%llu rnc=%s msv=%u tl=%u nch=%016llx",
               i, (unsigned long long)in.n, rs2(r6, b1), maxSV, tlog, fnv(nc, 512));
        bd6 = ((long long)r6 < 0) ? r6 : FSEv06_buildDTable(dt6, nc, maxSV, tlog);
        printf(" bd=%s", rs2(bd6, b2));
        if ((long long)bd6 >= 0) {
            size_t d;
            out_clear();
            d = FSEv06_decompress_usingDTable(g_out, OUTCAP, in.b, in.n, dt6);
            printf(" dud=%s h=%016llx g=%d", rs2(d, b3), out_hash(d), guard_ok());
            g_calls++;
        }
        printf("\n");
        g_calls += 2;

        memset(nc, 0, sizeof(nc)); maxSV = 255; tlog = 12;
        r7 = FSEv07_readNCount(nc, &maxSV, &tlog, in.b, in.n);
        printf("P7 fse07[%d] n=%llu rnc=%s msv=%u tl=%u nch=%016llx",
               i, (unsigned long long)in.n, rs2(r7, b1), maxSV, tlog, fnv(nc, 512));
        bd7 = ((long long)r7 < 0) ? r7 : FSEv07_buildDTable(dt7, nc, maxSV, tlog);
        printf(" bd=%s", rs2(bd7, b2));
        if ((long long)bd7 >= 0) {
            size_t d;
            out_clear();
            d = FSEv07_decompress_usingDTable(g_out, OUTCAP, in.b, in.n, dt7);
            printf(" dud=%s h=%016llx g=%d", rs2(d, b3), out_hash(d), guard_ok());
            g_calls++;
        }
        printf("\n");
        g_calls += 2;

        /* one-shot FSE decompress */
        {
            size_t d5, d6, d7;
            char c1[32], c2[32], c3[32];
            out_clear(); d5 = FSEv05_decompress(g_out, OUTCAP, in.b, in.n);
            printf("P7 fsed[%d] v05=%s h=%016llx g=%d", i, rs2(d5, c1), out_hash(d5), guard_ok());
            out_clear(); d6 = FSEv06_decompress(g_out, OUTCAP, in.b, in.n);
            printf(" v06=%s h=%016llx g=%d", rs2(d6, c2), out_hash(d6), guard_ok());
            out_clear(); d7 = FSEv07_decompress(g_out, OUTCAP, in.b, in.n);
            printf(" v07=%s h=%016llx g=%d\n", rs2(d7, c3), out_hash(d7), guard_ok());
            g_calls += 3;
        }
    }

    /* buildDTable_raw / buildDTable_rle sweeps + decode */
    for (i = 1; i <= 12; i++) {
        char b1[32], b2[32], b3[32];
        size_t a = FSEv05_buildDTable_raw(dt5, (unsigned)i);
        size_t b = FSEv06_buildDTable_raw(dt6, (unsigned)i);
        size_t c = FSEv07_buildDTable_raw(dt7, (unsigned)i);
        printf("P7 raw nb=%d v05=%s v06=%s v07=%s\n", i, rs2(a, b1), rs2(b, b2), rs2(c, b3));
        g_calls += 3;
        rs(0x7100ULL + (unsigned)i);
        {
            BUF in; size_t d;
            int k;
            for (k = 0; k < 40; k++) {
                gen(&in, 0, 3);
                if (in.n > 256) in.n = 256;
                out_clear(); d = FSEv05_decompress_usingDTable(g_out, OUTCAP, in.b, in.n, dt5);
                printf("P7 rawdec nb=%d k=%d v05=%s h=%016llx g=%d", i, k, rs2(d, b1), out_hash(d), guard_ok());
                out_clear(); d = FSEv06_decompress_usingDTable(g_out, OUTCAP, in.b, in.n, dt6);
                printf(" v06=%s h=%016llx g=%d", rs2(d, b2), out_hash(d), guard_ok());
                out_clear(); d = FSEv07_decompress_usingDTable(g_out, OUTCAP, in.b, in.n, dt7);
                printf(" v07=%s h=%016llx g=%d\n", rs2(d, b3), out_hash(d), guard_ok());
                g_calls += 3;
            }
        }
    }
    for (i = 0; i < 256; i += 17) {
        char b1[32], b2[32], b3[32];
        size_t a = FSEv05_buildDTable_rle(dt5, (unsigned char)i);
        size_t b = FSEv06_buildDTable_rle(dt6, (unsigned char)i);
        size_t c = FSEv07_buildDTable_rle(dt7, (unsigned char)i);
        size_t d;
        printf("P7 rle s=%d v05=%s v06=%s v07=%s", i, rs2(a, b1), rs2(b, b2), rs2(c, b3));
        out_clear(); d = FSEv05_decompress_usingDTable(g_out, 300, "\x01\x02\x03\x04", 4, dt5);
        printf(" d05=%s h=%016llx", rs2(d, b1), out_hash(d));
        out_clear(); d = FSEv06_decompress_usingDTable(g_out, 300, "\x01\x02\x03\x04", 4, dt6);
        printf(" d06=%s h=%016llx", rs2(d, b2), out_hash(d));
        out_clear(); d = FSEv07_decompress_usingDTable(g_out, 300, "\x01\x02\x03\x04", 4, dt7);
        printf(" d07=%s h=%016llx g=%d\n", rs2(d, b3), out_hash(d), guard_ok());
        g_calls += 6;
    }
    /* buildDTable with hand-made normalized counters */
    rs(0x7200);
    for (i = 0; i < 4000; i++) {
        char b1[32], b2[32], b3[32];
        unsigned tl = 5 + (r32() % 10);
        unsigned msv = 1 + (r32() % 255);
        unsigned j; int total = 0;
        size_t a, b, c;
        memset(nc, 0, sizeof(nc));
        for (j = 0; j <= msv && total < (1 << tl); j++) {
            int v = (int)(r32() % 8);
            if (v == 0 && (r32() & 1)) v = -1;
            nc[j] = (short)v;
            if (v > 0) total += v;
        }
        if (total < (1 << tl)) nc[0] = (short)(nc[0] + ((1 << tl) - total));
        a = FSEv05_buildDTable(dt5, nc, msv, tl);
        b = FSEv06_buildDTable(dt6, nc, msv, tl);
        c = FSEv07_buildDTable(dt7, nc, msv, tl);
        printf("P7 bdt[%d] tl=%u msv=%u v05=%s v06=%s v07=%s", i, tl, msv,
               rs2(a, b1), rs2(b, b2), rs2(c, b3));
        if ((long long)a >= 0) {
            BUF in; size_t d;
            gen(&in, 0, 3);
            if (in.n > 200) in.n = 200;
            out_clear(); d = FSEv05_decompress_usingDTable(g_out, OUTCAP, in.b, in.n, dt5);
            printf(" d05=%s h=%016llx", rs2(d, b1), out_hash(d));
            out_clear(); d = FSEv06_decompress_usingDTable(g_out, OUTCAP, in.b, in.n, dt6);
            printf(" d06=%s h=%016llx", rs2(d, b2), out_hash(d));
            out_clear(); d = FSEv07_decompress_usingDTable(g_out, OUTCAP, in.b, in.n, dt7);
            printf(" d07=%s h=%016llx g=%d", rs2(d, b3), out_hash(d), guard_ok());
            g_calls += 3;
        }
        printf("\n");
        g_calls += 3;
    }
    free(dt5); free(dt6); free(dt7);
}

/* ===================================================================== */
/* P8: HUFv05 / HUFv06 / HUFv07 direct                                   */
/* ===================================================================== */
static void phase8(void)
{
    /* generously sized DTables */
    unsigned short* x2_5 = (unsigned short*)malloc(sizeof(unsigned short) * (1 + (1u<<17)));
    unsigned short* x2_6 = (unsigned short*)malloc(sizeof(unsigned short) * (1 + (1u<<17)));
    unsigned*       x4_5 = (unsigned*)malloc(sizeof(unsigned) * (1 + (1u<<17)));
    unsigned*       x4_6 = (unsigned*)malloc(sizeof(unsigned) * (1 + (1u<<17)));
    unsigned*       d7a  = (unsigned*)malloc(sizeof(unsigned) * (1 + (1u<<17)));
    unsigned*       d7b  = (unsigned*)malloc(sizeof(unsigned) * (1 + (1u<<17)));
    int i;
    BANNER("P8", "HUFv0x direct");

    rs(0x8001);
    for (i = 0; i < 30000; i++) {
        BUF in;
        char b1[32], b2[32], b3[32], b4[32];
        size_t r;
        gen(&in, 0, 3);
        if (in.n > 400) in.n = 400;

        /* one-shot decoders */
        out_clear(); r = HUFv05_decompress(g_out, OUTCAP, in.b, in.n);
        printf("P8 huf[%d] n=%llu d05=%s h=%016llx", i, (unsigned long long)in.n, rs2(r, b1), out_hash(r));
        out_clear(); r = HUFv06_decompress(g_out, OUTCAP, in.b, in.n);
        printf(" d06=%s h=%016llx", rs2(r, b2), out_hash(r));
        out_clear(); r = HUFv07_decompress(g_out, OUTCAP, in.b, in.n);
        printf(" d07=%s h=%016llx g=%d\n", rs2(r, b3), out_hash(r), guard_ok());
        g_calls += 3;

        /* 1X/4X X2/X4 variants (each validates its own input) */
        out_clear(); r = HUFv05_decompress1X2(g_out, OUTCAP, in.b, in.n);
        printf("P8 hufv05[%d] 1X2=%s h=%016llx", i, rs2(r, b1), out_hash(r));
        out_clear(); r = HUFv05_decompress1X4(g_out, OUTCAP, in.b, in.n);
        printf(" 1X4=%s h=%016llx", rs2(r, b2), out_hash(r));
        out_clear(); r = HUFv05_decompress4X2(g_out, OUTCAP, in.b, in.n);
        printf(" 4X2=%s h=%016llx", rs2(r, b3), out_hash(r));
        out_clear(); r = HUFv05_decompress4X4(g_out, OUTCAP, in.b, in.n);
        printf(" 4X4=%s h=%016llx g=%d\n", rs2(r, b4), out_hash(r), guard_ok());
        g_calls += 4;

        out_clear(); r = HUFv06_decompress1X2(g_out, OUTCAP, in.b, in.n);
        printf("P8 hufv06[%d] 1X2=%s h=%016llx", i, rs2(r, b1), out_hash(r));
        out_clear(); r = HUFv06_decompress1X4(g_out, OUTCAP, in.b, in.n);
        printf(" 1X4=%s h=%016llx", rs2(r, b2), out_hash(r));
        out_clear(); r = HUFv06_decompress4X2(g_out, OUTCAP, in.b, in.n);
        printf(" 4X2=%s h=%016llx", rs2(r, b3), out_hash(r));
        out_clear(); r = HUFv06_decompress4X4(g_out, OUTCAP, in.b, in.n);
        printf(" 4X4=%s h=%016llx g=%d\n", rs2(r, b4), out_hash(r), guard_ok());
        g_calls += 4;

        out_clear(); r = HUFv07_decompress1X2(g_out, OUTCAP, in.b, in.n);
        printf("P8 hufv07[%d] 1X2=%s h=%016llx", i, rs2(r, b1), out_hash(r));
        out_clear(); r = HUFv07_decompress1X4(g_out, OUTCAP, in.b, in.n);
        printf(" 1X4=%s h=%016llx", rs2(r, b2), out_hash(r));
        out_clear(); r = HUFv07_decompress4X2(g_out, OUTCAP, in.b, in.n);
        printf(" 4X2=%s h=%016llx", rs2(r, b3), out_hash(r));
        out_clear(); r = HUFv07_decompress4X4(g_out, OUTCAP, in.b, in.n);
        printf(" 4X4=%s h=%016llx g=%d\n", rs2(r, b4), out_hash(r), guard_ok());
        g_calls += 4;

        /* HUFv07_readStats */
        {
            unsigned char hw[256];
            unsigned rank[32], nbs = 0, tl = 0;
            size_t rr;
            memset(hw, 0xA5, sizeof(hw)); memset(rank, 0, sizeof(rank));
            rr = HUFv07_readStats(hw, 256, rank, &nbs, &tl, in.b, in.n);
            printf("P8 rstat[%d] rv=%s nbs=%u tl=%u hw=%016llx rank=%016llx\n",
                   i, rs2(rr, b1), nbs, tl, fnv(hw, 256), fnv(rank, sizeof(rank)));
            g_calls++;
        }

        /* readDTableXn then *_usingDTable / *_DCtx (only on success) */
        {
            size_t a, b;
            x2_5[0] = 12; x4_5[0] = 12;
            a = HUFv05_readDTableX2(x2_5, in.b, in.n);
            b = HUFv05_readDTableX4(x4_5, in.b, in.n);
            printf("P8 rdt05[%d] X2=%s X4=%s", i, rs2(a, b1), rs2(b, b2));
            if ((long long)a >= 0) {
                out_clear(); r = HUFv05_decompress1X2_usingDTable(g_out, OUTCAP, in.b + a, in.n - a, x2_5);
                printf(" u1X2=%s h=%016llx", rs2(r, b3), out_hash(r));
                out_clear(); r = HUFv05_decompress4X2_usingDTable(g_out, OUTCAP, in.b + a, in.n - a, x2_5);
                printf(" u4X2=%s h=%016llx", rs2(r, b4), out_hash(r));
                g_calls += 2;
            }
            if ((long long)b >= 0) {
                out_clear(); r = HUFv05_decompress1X4_usingDTable(g_out, OUTCAP, in.b + b, in.n - b, x4_5);
                printf(" u1X4=%s h=%016llx", rs2(r, b3), out_hash(r));
                out_clear(); r = HUFv05_decompress4X4_usingDTable(g_out, OUTCAP, in.b + b, in.n - b, x4_5);
                printf(" u4X4=%s h=%016llx", rs2(r, b4), out_hash(r));
                g_calls += 2;
            }
            printf(" g=%d\n", guard_ok());
            g_calls += 2;
        }
        {
            size_t a, b;
            x2_6[0] = 12; x4_6[0] = 12;
            a = HUFv06_readDTableX2(x2_6, in.b, in.n);
            b = HUFv06_readDTableX4(x4_6, in.b, in.n);
            printf("P8 rdt06[%d] X2=%s X4=%s", i, rs2(a, b1), rs2(b, b2));
            if ((long long)a >= 0) {
                out_clear(); r = HUFv06_decompress1X2_usingDTable(g_out, OUTCAP, in.b + a, in.n - a, x2_6);
                printf(" u1X2=%s h=%016llx", rs2(r, b3), out_hash(r));
                out_clear(); r = HUFv06_decompress4X2_usingDTable(g_out, OUTCAP, in.b + a, in.n - a, x2_6);
                printf(" u4X2=%s h=%016llx", rs2(r, b4), out_hash(r));
                g_calls += 2;
            }
            if ((long long)b >= 0) {
                out_clear(); r = HUFv06_decompress1X4_usingDTable(g_out, OUTCAP, in.b + b, in.n - b, x4_6);
                printf(" u1X4=%s h=%016llx", rs2(r, b3), out_hash(r));
                out_clear(); r = HUFv06_decompress4X4_usingDTable(g_out, OUTCAP, in.b + b, in.n - b, x4_6);
                printf(" u4X4=%s h=%016llx", rs2(r, b4), out_hash(r));
                g_calls += 2;
            }
            printf(" g=%d\n", guard_ok());
            g_calls += 2;
        }
        {
            size_t a, b;
            d7a[0] = 12u * 0x1000001u;
            d7b[0] = 12u * 0x1000001u;
            a = HUFv07_readDTableX2(d7a, in.b, in.n);
            b = HUFv07_readDTableX4(d7b, in.b, in.n);
            printf("P8 rdt07[%d] X2=%s X4=%s", i, rs2(a, b1), rs2(b, b2));
            if ((long long)a >= 0) {
                out_clear(); r = HUFv07_decompress1X2_usingDTable(g_out, OUTCAP, in.b + a, in.n - a, d7a);
                printf(" u1X2=%s h=%016llx", rs2(r, b3), out_hash(r));
                out_clear(); r = HUFv07_decompress4X2_usingDTable(g_out, OUTCAP, in.b + a, in.n - a, d7a);
                printf(" u4X2=%s h=%016llx", rs2(r, b4), out_hash(r));
                out_clear(); r = HUFv07_decompress1X_usingDTable(g_out, OUTCAP, in.b + a, in.n - a, d7a);
                printf(" u1X=%s h=%016llx", rs2(r, b3), out_hash(r));
                out_clear(); r = HUFv07_decompress4X_usingDTable(g_out, OUTCAP, in.b + a, in.n - a, d7a);
                printf(" u4X=%s h=%016llx", rs2(r, b4), out_hash(r));
                g_calls += 4;
            }
            if ((long long)b >= 0) {
                out_clear(); r = HUFv07_decompress1X4_usingDTable(g_out, OUTCAP, in.b + b, in.n - b, d7b);
                printf(" u1X4=%s h=%016llx", rs2(r, b3), out_hash(r));
                out_clear(); r = HUFv07_decompress4X4_usingDTable(g_out, OUTCAP, in.b + b, in.n - b, d7b);
                printf(" u4X4=%s h=%016llx", rs2(r, b4), out_hash(r));
                g_calls += 2;
            }
            printf(" g=%d\n", guard_ok());
            g_calls += 2;

            /* _DCtx entry points: they build the table themselves */
            d7a[0] = 12u * 0x1000001u;
            out_clear(); r = HUFv07_decompress1X2_DCtx(d7a, g_out, OUTCAP, in.b, in.n);
            printf("P8 dctx07[%d] 1X2=%s h=%016llx", i, rs2(r, b1), out_hash(r));
            d7a[0] = 12u * 0x1000001u;
            out_clear(); r = HUFv07_decompress1X4_DCtx(d7a, g_out, OUTCAP, in.b, in.n);
            printf(" 1X4=%s h=%016llx", rs2(r, b2), out_hash(r));
            d7a[0] = 12u * 0x1000001u;
            out_clear(); r = HUFv07_decompress1X_DCtx(d7a, g_out, OUTCAP, in.b, in.n);
            printf(" 1X=%s h=%016llx", rs2(r, b3), out_hash(r));
            d7a[0] = 12u * 0x1000001u;
            out_clear(); r = HUFv07_decompress4X2_DCtx(d7a, g_out, OUTCAP, in.b, in.n);
            printf(" 4X2=%s h=%016llx", rs2(r, b4), out_hash(r));
            d7a[0] = 12u * 0x1000001u;
            out_clear(); r = HUFv07_decompress4X4_DCtx(d7a, g_out, OUTCAP, in.b, in.n);
            printf(" 4X4=%s h=%016llx", rs2(r, b1), out_hash(r));
            d7a[0] = 12u * 0x1000001u;
            out_clear(); r = HUFv07_decompress4X_DCtx(d7a, g_out, OUTCAP, in.b, in.n);
            printf(" 4X=%s h=%016llx", rs2(r, b2), out_hash(r));
            d7a[0] = 12u * 0x1000001u;
            out_clear(); r = HUFv07_decompress4X_hufOnly(d7a, g_out, OUTCAP, in.b, in.n);
            printf(" 4Xho=%s h=%016llx g=%d\n", rs2(r, b3), out_hash(r), guard_ok());
            g_calls += 7;
        }
    }
    free(x2_5); free(x2_6); free(x4_5); free(x4_6); free(d7a); free(d7b);
}

/* ===================================================================== */
/* P9: hand-built v0.4 frames that actually reach the internal             */
/*     FSE_readNCount() used to decode the Huff0 literal weight table.    */
/*                                                                        */
/* v0.4 layout used here:                                                 */
/*   magic(4) | windowByte(1) | blockHeader(3) | literalHeader(5) | huf    */
/*   literalHeader: litSize  = (LE32(b0..b3) & 0x1FFFFF) >> 2             */
/*                 litCSize = (LE32(b2..b4) & 0xFFFFFF) >> 5             */
/*                 b0 & 3 == 0  selects "Huff0-compressed literals"       */
/*   huf stream: [iSize<128] [FSE-coded weight table ...]                 */
/*   -> HUF_readStats() -> FSE_decompress() -> FSE_readNCount()           */
/* ===================================================================== */
static void phase9(void)
{
    int i;
    ZSTD_DStream* zds = ZSTD_createDStream();
    BANNER("P9", "targeted v0.4 Huff0 literal weight tables");
    rs(0x9004);
    for (i = 0; i < 40000; i++) {
        BUF in;
        char b1[32], b2[32], b3[32];
        size_t litSize, k, litCSize, cSize, p, j;
        unsigned iSize, tlIdx;
        size_t rv, rv2, rvm;
        void* dctx;

        litSize  = 64 + (size_t)(r32() % 8000);          /* < 16384 so bits 16..20 stay 0 */
        k        = 2 + (size_t)(r32() % 120);            /* litCSize = 8*k */
        litCSize = 8 * k;
        if (litCSize >= litSize) litCSize = litSize - 8;  /* HUF needs cSrc < dst */
        if (litCSize < 16) litCSize = 16;
        k = litCSize / 8;
        cSize = litCSize + 5;
        if (cSize + 8 > MAXIN) continue;

        memset(in.b, 0, sizeof(in.b));
        memcpy(in.b, MAGIC[4], 4);
        in.b[4] = (unsigned char)(r32() & 15);            /* window descriptor */
        in.b[5] = (unsigned char)((0u << 6) | ((cSize >> 16) & 7));  /* bt_compressed */
        in.b[6] = (unsigned char)((cSize >> 8) & 255);
        in.b[7] = (unsigned char)(cSize & 255);
        p = 8;
        /* 5-byte literal header */
        in.b[p+0] = (unsigned char)((litSize * 4) & 0xFF);
        in.b[p+1] = (unsigned char)(((litSize * 4) >> 8) & 0xFF);
        in.b[p+2] = 0x00;
        in.b[p+3] = (unsigned char)(k & 0xFF);
        in.b[p+4] = (unsigned char)((k >> 8) & 0xFF);
        /* Huff0 stream */
        iSize = 4 + (r32() % 100);
        if (iSize > litCSize - 2) iSize = (unsigned)(litCSize - 2);
        in.b[p+5] = (unsigned char)iSize;
        tlIdx = r32() & 7;
        memcpy(in.b + p + 6, TL15[tlIdx], 4);
        for (j = p + 10; j < p + 5 + litCSize; j++) in.b[j] = bbyte();
        in.n = p + 5 + litCSize;
        /* a few trailing bytes so the sequence section has something to read */
        for (j = in.n; j < in.n + 8 && j < MAXIN; j++) in.b[j] = bbyte();
        in.n += 8;

        out_clear();
        rv = ZSTDv04_decompress(g_out, OUTCAP, in.b, in.n);
        printf("P9 v04[%d] ls=%llu lcs=%llu is=%u tl=%u n=%llu dec=%s h=%016llx g=%d",
               i, (unsigned long long)litSize, (unsigned long long)litCSize, iSize, tlIdx,
               (unsigned long long)in.n, rs2(rv, b1), out_hash(rv), guard_ok());
        g_calls++;
        dctx = ZSTDv04_createDCtx();
        out_clear();
        rv2 = ZSTDv04_decompressDCtx(dctx, g_out, OUTCAP, in.b, in.n);
        ZSTDv04_freeDCtx(dctx);
        printf(" dctx=%s h=%016llx", rs2(rv2, b2), out_hash(rv2));
        g_calls++;
        out_clear();
        rvm = ZSTD_decompress(g_out, OUTCAP, in.b, in.n);
        printf(" modern=%s h=%016llx", rs2(rvm, b3), out_hash(rvm));
        g_calls++;
        {
            size_t cS = 0x5A5A5A5A; unsigned long long dB = 0xA5A5A5A5A5A5A5A5ULL;
            ZSTDv04_findFrameSizeInfoLegacy(in.b, in.n, &cS, &dB);
            printf(" ffsi=%s/%llu", rs2(cS, b1), (unsigned long long)dB);
            g_calls++;
        }
        printf("\n");
        /* also through ZBUFFv04 and ZSTD_decompressStream */
        zbuff_run(4, &in, g_dict, 0, 0, 200000 + i, 0);
        {
            size_t ir = ZSTD_initDStream(zds);
            ZSTD_inBuffer ib; ZSTD_outBuffer ob;
            int it;
            dg_reset(); dg_add(ir);
            ib.src = in.b; ib.size = in.n; ib.pos = 0;
            out_clear();
            for (it = 0; it < 16; it++) {
                size_t r;
                ob.dst = g_out; ob.size = OUTCAP; ob.pos = 0;
                r = ZSTD_decompressStream(zds, &ob, &ib);
                dg_add(r); dg_add(ib.pos); dg_add(ob.pos);
                dg_add(fnv(g_out, ob.pos < 64 ? 64 : ob.pos));
                g_calls++;
                if (ZSTD_isError(r) || r == 0) break;
            }
            printf("P9 v04s[%d] dg=%016llx g=%d\n", i, g_dg, guard_ok());
        }
    }
    ZSTD_freeDStream(zds);
}

/* ===================================================================== */
int main(int argc, char** argv)
{
    size_t i;
    g_argc = argc; g_argv = argv;
    /* Buffering only affects flush timing, never the emitted text.  Set
     * LEGACY_UNBUF=1 to get unbuffered output when locating a crash.        */
    if (getenv("LEGACY_UNBUF")) setvbuf(stdout, NULL, _IONBF, 0);
    else                        setvbuf(stdout, NULL, _IOFBF, 1 << 20);

    g_out = (unsigned char*)malloc(OUTCAP + GUARDSZ);
    g_ref = (unsigned char*)malloc(GUARDSZ);
    memset(g_ref, 0xA5, GUARDSZ);
    out_clear();

    rs(0xDEADBEEFCAFEULL);
    for (i = 0; i < sizeof(g_dict); i++) g_dict[i] = (unsigned char)r32();

    printf("### zstd legacy differential harness ###\n");

    if (phase_on("P0")) phase0();
    if (phase_on("P1")) phase1();
    if (phase_on("P2")) phase2();
    if (phase_on("P3")) phase3();
    if (phase_on("P4")) phase4();
    if (phase_on("P5")) phase5();
    if (phase_on("P6")) phase6();
    if (phase_on("P7")) phase7();
    if (phase_on("P8")) phase8();
    if (phase_on("P9")) phase9();

    printf("\n### DONE calls=%lld ###\n", g_calls);
    free(g_out); free(g_ref);
    return 0;
}
