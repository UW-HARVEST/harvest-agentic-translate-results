# Error Surface

Generated mechanically from every C source/header site containing an error-return macro or statement, a null/-1 return, or an assertion. Each row preserves the exact source statement and location. Assertions are internal invariant rejection sites and are included as required.

| # | function | trigger (exact C source condition/statement) | expected C result | test |
|---:|----------|----------------------------------------------|-------------------|:----:|
| 1 | `(file scope)` (c_src/src/common/bits.h:18) | `assert(val != 0);` | process assertion failure | [x] |
| 2 | `(file scope)` (c_src/src/common/bits.h:30) | `assert(val != 0);` | process assertion failure | [x] |
| 3 | `(file scope)` (c_src/src/common/bits.h:54) | `assert(val != 0);` | process assertion failure | [x] |
| 4 | `(file scope)` (c_src/src/common/bits.h:71) | `assert(val != 0);` | process assertion failure | [x] |
| 5 | `(file scope)` (c_src/src/common/bits.h:95) | `assert(val != 0);` | process assertion failure | [x] |
| 6 | `(file scope)` (c_src/src/common/bits.h:127) | `assert(val != 0);` | process assertion failure | [x] |
| 7 | `(file scope)` (c_src/src/common/bits.h:176) | `assert(val != 0);` | process assertion failure | [x] |
| 8 | `(file scope)` (c_src/src/common/bits.h:186) | `assert(count < 64);` | process assertion failure | [x] |
| 9 | `(file scope)` (c_src/src/common/bits.h:193) | `assert(count < 32);` | process assertion failure | [x] |
| 10 | `(file scope)` (c_src/src/common/bits.h:200) | `assert(count < 16);` | process assertion failure | [x] |
| 11 | `(file scope)` (c_src/src/common/bitstream.h:28) | `#include "debug.h" /* assert(), DEBUGLOG(), RAWLOG() */` | process assertion failure | [x] |
| 12 | `(file scope)` (c_src/src/common/bitstream.h:158) | `if (dstCapacity <= sizeof(bitC->bitContainer)) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 13 | `(file scope)` (c_src/src/common/bitstream.h:168) | `DEBUG_STATIC_ASSERT(sizeof(bitContainer) == sizeof(U32));` | process assertion failure | [x] |
| 14 | `(file scope)` (c_src/src/common/bitstream.h:172) | `assert(nbBits < BIT_MASK_SIZE);` | process assertion failure | [x] |
| 15 | `(file scope)` (c_src/src/common/bitstream.h:183) | `DEBUG_STATIC_ASSERT(BIT_MASK_SIZE == 32);` | process assertion failure | [x] |
| 16 | `(file scope)` (c_src/src/common/bitstream.h:184) | `assert(nbBits < BIT_MASK_SIZE);` | process assertion failure | [x] |
| 17 | `(file scope)` (c_src/src/common/bitstream.h:185) | `assert(nbBits + bitC->bitPos < sizeof(bitC->bitContainer) * 8);` | process assertion failure | [x] |
| 18 | `(file scope)` (c_src/src/common/bitstream.h:196) | `assert((value>>nbBits) == 0);` | process assertion failure | [x] |
| 19 | `(file scope)` (c_src/src/common/bitstream.h:197) | `assert(nbBits + bitC->bitPos < sizeof(bitC->bitContainer) * 8);` | process assertion failure | [x] |
| 20 | `(file scope)` (c_src/src/common/bitstream.h:208) | `assert(bitC->bitPos < sizeof(bitC->bitContainer) * 8);` | process assertion failure | [x] |
| 21 | `(file scope)` (c_src/src/common/bitstream.h:209) | `assert(bitC->ptr <= bitC->endPtr);` | process assertion failure | [x] |
| 22 | `(file scope)` (c_src/src/common/bitstream.h:224) | `assert(bitC->bitPos < sizeof(bitC->bitContainer) * 8);` | process assertion failure | [x] |
| 23 | `(file scope)` (c_src/src/common/bitstream.h:225) | `assert(bitC->ptr <= bitC->endPtr);` | process assertion failure | [x] |
| 24 | `(file scope)` (c_src/src/common/bitstream.h:256) | `if (srcSize < 1) { ZSTD_memset(bitD, 0, sizeof(*bitD)); return ERROR(srcSize_wrong); }` | `ERROR(srcSize_wrong)` | [x] |
| 25 | `(file scope)` (c_src/src/common/bitstream.h:266) | `if (lastByte == 0) return ERROR(GENERIC); /* endMark not present */ }` | `ERROR(GENERIC)` | [x] |
| 26 | `(file scope)` (c_src/src/common/bitstream.h:294) | `if (lastByte == 0) return ERROR(corruption_detected); /* endMark not present */` | `ERROR(corruption_detected)` | [x] |
| 27 | `(file scope)` (c_src/src/common/bitstream.h:311) | `assert(nbBits < BIT_MASK_SIZE);` | process assertion failure | [x] |
| 28 | `(file scope)` (c_src/src/common/bitstream.h:349) | `assert(nbBits >= 1);` | process assertion failure | [x] |
| 29 | `(file scope)` (c_src/src/common/bitstream.h:374) | `assert(nbBits >= 1);` | process assertion failure | [x] |
| 30 | `(file scope)` (c_src/src/common/bitstream.h:386) | `assert(bitD->bitsConsumed <= sizeof(bitD->bitContainer)*8);` | process assertion failure | [x] |
| 31 | `(file scope)` (c_src/src/common/bitstream.h:388) | `assert(bitD->ptr >= bitD->start);` | process assertion failure | [x] |
| 32 | `(file scope)` (c_src/src/common/bitstream.h:422) | `assert(bitD->ptr >= bitD->start);` | process assertion failure | [x] |
| 33 | `(file scope)` (c_src/src/common/compiler.h:195) | `# define ZSTD_UNREACHABLE do { assert(0), __builtin_unreachable(); } while (0)` | process assertion failure | [x] |
| 34 | `(file scope)` (c_src/src/common/compiler.h:197) | `# define ZSTD_UNREACHABLE do { assert(0); } while (0)` | process assertion failure | [x] |
| 35 | `(file scope)` (c_src/src/common/debug.h:18) | `* They regroup assert(), DEBUGLOG() and RAWLOG() for run-time,` | process assertion failure | [x] |
| 36 | `(file scope)` (c_src/src/common/debug.h:19) | `* and DEBUG_STATIC_ASSERT() for compile-time.` | process assertion failure | [x] |
| 37 | `(file scope)` (c_src/src/common/debug.h:23) | `* Level 1 enables assert() only.` | process assertion failure | [x] |
| 38 | `(file scope)` (c_src/src/common/debug.h:39) | `#define DEBUG_STATIC_ASSERT(c) (void)sizeof(char[(c) ? 1 : -1])` | process assertion failure | [x] |
| 39 | `(file scope)` (c_src/src/common/debug.h:52) | `* 1 : enables assert() only, no display` | process assertion failure | [x] |
| 40 | `(file scope)` (c_src/src/common/debug.h:70) | `# define assert(condition) ((void)0) /* disable assert (default) */` | process assertion failure | [x] |
| 41 | `FSE_readNCount_body` (c_src/src/common/entropy_common.c:64) | `if (countSize > hbSize) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 42 | `FSE_readNCount_body` (c_src/src/common/entropy_common.c:67) | `assert(hbSize >= 8);` | process assertion failure | [x] |
| 43 | `FSE_readNCount_body` (c_src/src/common/entropy_common.c:73) | `if (nbBits > FSE_TABLELOG_ABSOLUTE_MAX) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [x] |
| 44 | `FSE_readNCount_body` (c_src/src/common/entropy_common.c:106) | `assert((bitStream & 3) < 3);` | process assertion failure | [x] |
| 45 | `FSE_readNCount_body` (c_src/src/common/entropy_common.c:121) | `assert((bitCount >> 3) <= 3); /* For first condition to work */` | process assertion failure | [x] |
| 46 | `FSE_readNCount_body` (c_src/src/common/entropy_common.c:151) | `assert(count == -1);` | process assertion failure | [x] |
| 47 | `FSE_readNCount_body` (c_src/src/common/entropy_common.c:157) | `assert(threshold > 1);` | process assertion failure | [x] |
| 48 | `FSE_readNCount_body` (c_src/src/common/entropy_common.c:179) | `if (remaining != 1) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 49 | `FSE_readNCount_body` (c_src/src/common/entropy_common.c:181) | `if (charnum > maxSV1) return ERROR(maxSymbolValue_tooSmall);` | `ERROR(maxSymbolValue_tooSmall)` | [x] |
| 50 | `FSE_readNCount_body` (c_src/src/common/entropy_common.c:182) | `if (bitCount > 32) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 51 | `HUF_readStats_body` (c_src/src/common/entropy_common.c:254) | `if (!srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 52 | `HUF_readStats_body` (c_src/src/common/entropy_common.c:261) | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 53 | `HUF_readStats_body` (c_src/src/common/entropy_common.c:262) | `if (oSize >= hwSize) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 54 | `HUF_readStats_body` (c_src/src/common/entropy_common.c:270) | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 55 | `HUF_readStats_body` (c_src/src/common/entropy_common.c:280) | `if (huffWeight[n] > HUF_TABLELOG_MAX) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 56 | `HUF_readStats_body` (c_src/src/common/entropy_common.c:284) | `if (weightTotal == 0) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 57 | `HUF_readStats_body` (c_src/src/common/entropy_common.c:288) | `if (tableLog > HUF_TABLELOG_MAX) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 58 | `HUF_readStats_body` (c_src/src/common/entropy_common.c:295) | `if (verif != rest) return ERROR(corruption_detected); /* last value must be a clean power of 2 */` | `ERROR(corruption_detected)` | [x] |
| 59 | `HUF_readStats_body` (c_src/src/common/entropy_common.c:301) | `if ((rankStats[1] < 2) \|\| (rankStats[1] & 1)) return ERROR(corruption_detected); /* by construction : at least 2 elts of rank 1, must be even */` | `ERROR(corruption_detected)` | [x] |
| 60 | `ERR_getErrorString` (c_src/src/common/error_private.c:50) | `case PREFIX(dstBuffer_null): return "Operation on NULL destination buffer";` | source-declared rejection sentinel | [x] |
| 61 | `(file scope)` (c_src/src/common/error_private.h:52) | `ERR_STATIC unsigned ERR_isError(size_t code) { return (code > ERROR(maxCode)); }` | `ERROR(maxCode)` | [x] |
| 62 | `(file scope)` (c_src/src/common/error_private.h:111) | `* this can't just wrap RETURN_ERROR().` | source-declared rejection sentinel | [x] |
| 63 | `(file scope)` (c_src/src/common/error_private.h:121) | `return ERROR(err); \` | `ERROR(err)` | [x] |
| 64 | `(file scope)` (c_src/src/common/error_private.h:137) | `return ERROR(err); \` | `ERROR(err)` | [x] |
| 65 | `(file scope)` (c_src/src/common/fse.h:490) | `assert(tableLog < 16);` | process assertion failure | [x] |
| 66 | `(file scope)` (c_src/src/common/fse.h:491) | `assert(accuracyLog < 31-tableLog); /* ensure enough room for renormalization double shift */` | process assertion failure | [x] |
| 67 | `(file scope)` (c_src/src/common/fse.h:496) | `assert(symbolTT[symbolValue].deltaNbBits + tableSize <= threshold);` | process assertion failure | [x] |
| 68 | `(file scope)` (c_src/src/common/fse.h:497) | `assert(normalizedDeltaFromThreshold <= bitMultiplier);` | process assertion failure | [x] |
| 69 | `(file scope)` (c_src/src/common/fse_decompress.c:33) | `#define FSE_STATIC_ASSERT(c) DEBUG_STATIC_ASSERT(c) /* use only *after* variable declarations */` | process assertion failure | [x] |
| 70 | `FSE_buildDTable_internal` (c_src/src/common/fse_decompress.c:70) | `if (FSE_BUILD_DTABLE_WKSP_SIZE(tableLog, maxSymbolValue) > wkspSize) return ERROR(maxSymbolValue_tooLarge);` | `ERROR(maxSymbolValue_tooLarge)` | [x] |
| 71 | `FSE_buildDTable_internal` (c_src/src/common/fse_decompress.c:71) | `if (maxSymbolValue > FSE_MAX_SYMBOL_VALUE) return ERROR(maxSymbolValue_tooLarge);` | `ERROR(maxSymbolValue_tooLarge)` | [x] |
| 72 | `FSE_buildDTable_internal` (c_src/src/common/fse_decompress.c:72) | `if (tableLog > FSE_MAX_TABLELOG) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [x] |
| 73 | `FSE_buildDTable_internal` (c_src/src/common/fse_decompress.c:124) | `assert(tableSize % unroll == 0); /* FSE_MIN_TABLELOG is 5 */` | process assertion failure | [x] |
| 74 | `FSE_buildDTable_internal` (c_src/src/common/fse_decompress.c:133) | `assert(position == 0);` | process assertion failure | [x] |
| 75 | `FSE_buildDTable_internal` (c_src/src/common/fse_decompress.c:146) | `if (position!=0) return ERROR(GENERIC); /* position must reach all cells once, otherwise normalizedCounter is incorrect */` | `ERROR(GENERIC)` | [x] |
| 76 | `FSE_decompress_usingDTable_generic` (c_src/src/common/fse_decompress.c:193) | `RETURN_ERROR_IF(BIT_reloadDStream(&bitD)==BIT_DStream_overflow, corruption_detected, "");` | `ERROR(BIT_reloadDStream)` | [x] |
| 77 | `FSE_decompress_usingDTable_generic` (c_src/src/common/fse_decompress.c:220) | `if (op>(omax-2)) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 78 | `FSE_decompress_usingDTable_generic` (c_src/src/common/fse_decompress.c:227) | `if (op>(omax-2)) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 79 | `FSE_decompress_usingDTable_generic` (c_src/src/common/fse_decompress.c:234) | `assert(op >= ostart);` | process assertion failure | [x] |
| 80 | `FSE_decompress_wksp_body` (c_src/src/common/fse_decompress.c:258) | `if (wkspSize < sizeof(*wksp)) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [x] |
| 81 | `FSE_decompress_wksp_body` (c_src/src/common/fse_decompress.c:267) | `if (tableLog > maxLog) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [x] |
| 82 | `FSE_decompress_wksp_body` (c_src/src/common/fse_decompress.c:268) | `assert(NCountLength <= cSrcSize);` | process assertion failure | [x] |
| 83 | `FSE_decompress_wksp_body` (c_src/src/common/fse_decompress.c:273) | `if (FSE_DECOMPRESS_WKSP_SIZE(tableLog, maxSymbolValue) > wkspSize) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [x] |
| 84 | `FSE_decompress_wksp_body` (c_src/src/common/fse_decompress.c:274) | `assert(sizeof(*wksp) + FSE_DTABLE_SIZE(tableLog) <= wkspSize);` | process assertion failure | [x] |
| 85 | `(file scope)` (c_src/src/common/mem.h:420) | `MEM_STATIC void MEM_check(void) { DEBUG_STATIC_ASSERT((sizeof(size_t)==4) \|\| (sizeof(size_t)==8)); }` | process assertion failure | [x] |
| 86 | `POOL_thread` (c_src/src/common/pool.c:69) | `if (!ctx) { return NULL; }` | `NULL` | [x] |
| 87 | `POOL_thread` (c_src/src/common/pool.c:103) | `assert(0); /* Unreachable */` | process assertion failure | [x] |
| 88 | `POOL_create_advanced` (c_src/src/common/pool.c:120) | `if (!numThreads) { return NULL; }` | `NULL` | [x] |
| 89 | `POOL_create_advanced` (c_src/src/common/pool.c:123) | `if (!ctx) { return NULL; }` | `NULL` | [x] |
| 90 | `POOL_create_advanced` (c_src/src/common/pool.c:139) | `if (error) { POOL_free(ctx); return NULL; }` | `NULL` | [x] |
| 91 | `POOL_create_advanced` (c_src/src/common/pool.c:147) | `if (!ctx->threads \|\| !ctx->queue) { POOL_free(ctx); return NULL; }` | `NULL` | [x] |
| 92 | `POOL_create_advanced` (c_src/src/common/pool.c:154) | `return NULL;` | `NULL` | [x] |
| 93 | `POOL_add_internal` (c_src/src/common/pool.c:277) | `assert(ctx != NULL);` | process assertion failure | [x] |
| 94 | `POOL_add` (c_src/src/common/pool.c:288) | `assert(ctx != NULL);` | process assertion failure | [x] |
| 95 | `POOL_tryAdd` (c_src/src/common/pool.c:301) | `assert(ctx != NULL);` | process assertion failure | [x] |
| 96 | `POOL_tryAdd` (c_src/src/common/pool.c:320) | `/* We don't need any data, but if it is empty, malloc() might return NULL. */` | `NULL` | [x] |
| 97 | `POOL_free` (c_src/src/common/pool.c:340) | `assert(!ctx \|\| ctx == &g_poolCtx);` | process assertion failure | [x] |
| 98 | `POOL_joinJobs` (c_src/src/common/pool.c:345) | `assert(!ctx \|\| ctx == &g_poolCtx);` | process assertion failure | [x] |
| 99 | `POOL_sizeof` (c_src/src/common/pool.c:367) | `assert(ctx == &g_poolCtx);` | process assertion failure | [x] |
| 100 | `(file scope)` (c_src/src/common/pool.h:25) | `* @return : POOL_ctx pointer on success, else NULL.` | source-declared rejection sentinel | [x] |
| 101 | `ZSTD_pthread_create` (c_src/src/common/threading.c:76) | `if (thread==NULL) return -1;` | `-1` | [x] |
| 102 | `ZSTD_pthread_create` (c_src/src/common/threading.c:86) | `return -1;` | `-1` | [x] |
| 103 | `ZSTD_pthread_create` (c_src/src/common/threading.c:91) | `return -1;` | `-1` | [x] |
| 104 | `ZSTD_pthread_mutex_init` (c_src/src/common/threading.c:142) | `assert(mutex != NULL);` | process assertion failure | [x] |
| 105 | `ZSTD_pthread_mutex_destroy` (c_src/src/common/threading.c:151) | `assert(mutex != NULL);` | process assertion failure | [x] |
| 106 | `ZSTD_pthread_cond_init` (c_src/src/common/threading.c:163) | `assert(cond != NULL);` | process assertion failure | [x] |
| 107 | `ZSTD_pthread_cond_destroy` (c_src/src/common/threading.c:172) | `assert(cond != NULL);` | process assertion failure | [x] |
| 108 | `(file scope)` (c_src/src/common/xxhash.h:137) | `* assert(state != NULL && "Out of memory!");` | process assertion failure | [x] |
| 109 | `(file scope)` (c_src/src/common/xxhash.h:650) | `* @return 'NULL' on failure.` | source-declared rejection sentinel | [x] |
| 110 | `(file scope)` (c_src/src/common/xxhash.h:686) | `* @return @ref XXH_ERROR on failure.` | source-declared rejection sentinel | [x] |
| 111 | `(file scope)` (c_src/src/common/xxhash.h:707) | `* @return @ref XXH_ERROR on failure.` | source-declared rejection sentinel | [x] |
| 112 | `(file scope)` (c_src/src/common/xxhash.h:919) | `* @return 'NULL' on failure.` | source-declared rejection sentinel | [x] |
| 113 | `(file scope)` (c_src/src/common/xxhash.h:956) | `* @return @ref XXH_ERROR on failure.` | source-declared rejection sentinel | [x] |
| 114 | `(file scope)` (c_src/src/common/xxhash.h:977) | `* @return @ref XXH_ERROR on failure.` | source-declared rejection sentinel | [x] |
| 115 | `(file scope)` (c_src/src/common/xxhash.h:1224) | `* @return @ref XXH_ERROR on failure.` | source-declared rejection sentinel | [x] |
| 116 | `(file scope)` (c_src/src/common/xxhash.h:1244) | `* @return @ref XXH_ERROR on failure.` | source-declared rejection sentinel | [x] |
| 117 | `(file scope)` (c_src/src/common/xxhash.h:1265) | `* @return @ref XXH_ERROR on failure.` | source-declared rejection sentinel | [x] |
| 118 | `(file scope)` (c_src/src/common/xxhash.h:1293) | `* @return @ref XXH_ERROR on failure.` | source-declared rejection sentinel | [x] |
| 119 | `(file scope)` (c_src/src/common/xxhash.h:1426) | `* @return @ref XXH_ERROR on failure.` | source-declared rejection sentinel | [x] |
| 120 | `(file scope)` (c_src/src/common/xxhash.h:1445) | `* @return @ref XXH_ERROR on failure.` | source-declared rejection sentinel | [x] |
| 121 | `(file scope)` (c_src/src/common/xxhash.h:1464) | `* @return @ref XXH_ERROR on failure.` | source-declared rejection sentinel | [x] |
| 122 | `(file scope)` (c_src/src/common/xxhash.h:1488) | `* @return @ref XXH_ERROR on failure.` | source-declared rejection sentinel | [x] |
| 123 | `(file scope)` (c_src/src/common/xxhash.h:1803) | `* @return @ref XXH_ERROR on failure.` | source-declared rejection sentinel | [x] |
| 124 | `(file scope)` (c_src/src/common/xxhash.h:1942) | `* @return @ref XXH_ERROR on failure.` | source-declared rejection sentinel | [x] |
| 125 | `(file scope)` (c_src/src/common/xxhash.h:1960) | `* @return @ref XXH_ERROR on failure.` | source-declared rejection sentinel | [x] |
| 126 | `(file scope)` (c_src/src/common/xxhash.h:1977) | `* @return @ref XXH_ERROR on failure.` | source-declared rejection sentinel | [x] |
| 127 | `(file scope)` (c_src/src/common/xxhash.h:2305) | `* will always fail, and return NULL.` | `NULL` | [x] |
| 128 | `(file scope)` (c_src/src/common/xxhash.h:2315) | `static XXH_CONSTF void* XXH_malloc(size_t s) { (void)s; return NULL; }` | `NULL` | [x] |
| 129 | `(file scope)` (c_src/src/common/xxhash.h:2425) | `# define XXH_ASSERT(c) assert(c)` | process assertion failure | [x] |
| 130 | `(file scope)` (c_src/src/common/xxhash.h:6062) | `return XXH3_64bits_internal(input, length, seed, XXH3_kSecret, sizeof(XXH3_kSecret), NULL);` | source-declared rejection sentinel | [x] |
| 131 | `(file scope)` (c_src/src/common/xxhash.h:6116) | `return NULL;` | `NULL` | [x] |
| 132 | `(file scope)` (c_src/src/common/xxhash.h:6139) | `* @return 'NULL' on failure.` | source-declared rejection sentinel | [x] |
| 133 | `(file scope)` (c_src/src/common/xxhash.h:6146) | `if (state==NULL) return NULL;` | `NULL` | [x] |
| 134 | `(file scope)` (c_src/src/common/xxhash.h:6205) | `if (statePtr == NULL) return XXH_ERROR;` | `XXH_ERROR` | [x] |
| 135 | `(file scope)` (c_src/src/common/xxhash.h:6214) | `if (statePtr == NULL) return XXH_ERROR;` | `XXH_ERROR` | [x] |
| 136 | `(file scope)` (c_src/src/common/xxhash.h:6216) | `if (secret == NULL) return XXH_ERROR;` | `XXH_ERROR` | [x] |
| 137 | `(file scope)` (c_src/src/common/xxhash.h:6217) | `if (secretSize < XXH3_SECRET_SIZE_MIN) return XXH_ERROR;` | `XXH_ERROR` | [x] |
| 138 | `(file scope)` (c_src/src/common/xxhash.h:6225) | `if (statePtr == NULL) return XXH_ERROR;` | `XXH_ERROR` | [x] |
| 139 | `(file scope)` (c_src/src/common/xxhash.h:6237) | `if (statePtr == NULL) return XXH_ERROR;` | `XXH_ERROR` | [x] |
| 140 | `(file scope)` (c_src/src/common/xxhash.h:6238) | `if (secret == NULL) return XXH_ERROR;` | `XXH_ERROR` | [x] |
| 141 | `(file scope)` (c_src/src/common/xxhash.h:6239) | `if (secretSize < XXH3_SECRET_SIZE_MIN) return XXH_ERROR;` | `XXH_ERROR` | [x] |
| 142 | `(file scope)` (c_src/src/common/xxhash.h:6875) | `return XXH3_128bits_internal(input, len, seed, XXH3_kSecret, sizeof(XXH3_kSecret), NULL);` | source-declared rejection sentinel | [x] |
| 143 | `(file scope)` (c_src/src/common/xxhash.h:7027) | `/* production mode, assert() are disabled */` | process assertion failure | [x] |
| 144 | `(file scope)` (c_src/src/common/xxhash.h:7028) | `if (secretBuffer == NULL) return XXH_ERROR;` | `XXH_ERROR` | [x] |
| 145 | `(file scope)` (c_src/src/common/xxhash.h:7029) | `if (secretSize < XXH3_SECRET_SIZE_MIN) return XXH_ERROR;` | `XXH_ERROR` | [x] |
| 146 | `(file scope)` (c_src/src/common/xxhash.h:7039) | `if (customSeed == NULL) return XXH_ERROR;` | `XXH_ERROR` | [x] |
| 147 | `(file scope)` (c_src/src/common/zstd_deps.h:88) | `* assert()` | process assertion failure | [x] |
| 148 | `(file scope)` (c_src/src/common/zstd_internal.h:42) | `/* ---- static assert (debug) --- */` | process assertion failure | [x] |
| 149 | `(file scope)` (c_src/src/common/zstd_internal.h:43) | `#define ZSTD_STATIC_ASSERT(c) DEBUG_STATIC_ASSERT(c)` | process assertion failure | [x] |
| 150 | `(file scope)` (c_src/src/common/zstd_internal.h:229) | `assert(diff >= WILDCOPY_VECLEN \|\| diff <= -WILDCOPY_VECLEN);` | process assertion failure | [x] |
| 151 | `FSE_buildCTable_wksp` (c_src/src/compress/fse_compress.c:86) | `assert(((size_t)workSpace & 1) == 0); /* Must be 2 bytes-aligned */` | process assertion failure | [x] |
| 152 | `FSE_buildCTable_wksp` (c_src/src/compress/fse_compress.c:87) | `if (FSE_BUILD_CTABLE_WORKSPACE_SIZE(maxSymbolValue, tableLog) > wkspSize) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [x] |
| 153 | `FSE_buildCTable_wksp` (c_src/src/compress/fse_compress.c:91) | `assert(tableLog < 16); /* required for threshold strategy to work */` | process assertion failure | [x] |
| 154 | `FSE_buildCTable_wksp` (c_src/src/compress/fse_compress.c:108) | `assert(normalizedCounter[u-1] >= 0);` | process assertion failure | [x] |
| 155 | `FSE_buildCTable_wksp` (c_src/src/compress/fse_compress.c:110) | `assert(cumul[u] >= cumul[u-1]); /* no overflow */` | process assertion failure | [x] |
| 156 | `FSE_buildCTable_wksp` (c_src/src/compress/fse_compress.c:132) | `assert(n>=0);` | process assertion failure | [x] |
| 157 | `FSE_buildCTable_wksp` (c_src/src/compress/fse_compress.c:143) | `assert(tableSize % unroll == 0); /* FSE_MIN_TABLELOG is 5 */` | process assertion failure | [x] |
| 158 | `FSE_buildCTable_wksp` (c_src/src/compress/fse_compress.c:152) | `assert(position == 0); /* Must have initialized all positions */` | process assertion failure | [x] |
| 159 | `FSE_buildCTable_wksp` (c_src/src/compress/fse_compress.c:166) | `assert(position==0); /* Must have initialized all positions */` | process assertion failure | [x] |
| 160 | `FSE_buildCTable_wksp` (c_src/src/compress/fse_compress.c:189) | `assert(total <= INT_MAX);` | process assertion failure | [x] |
| 161 | `FSE_buildCTable_wksp` (c_src/src/compress/fse_compress.c:194) | `assert(normalizedCounter[s] > 1);` | process assertion failure | [x] |
| 162 | `FSE_writeNCount_generic` (c_src/src/compress/fse_compress.c:269) | `return ERROR(dstSize_tooSmall); /* Buffer overflow */` | `ERROR(dstSize_tooSmall)` | [x] |
| 163 | `FSE_writeNCount_generic` (c_src/src/compress/fse_compress.c:284) | `return ERROR(dstSize_tooSmall); /* Buffer overflow */` | `ERROR(dstSize_tooSmall)` | [x] |
| 164 | `FSE_writeNCount_generic` (c_src/src/compress/fse_compress.c:301) | `if (remaining<1) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [x] |
| 165 | `FSE_writeNCount_generic` (c_src/src/compress/fse_compress.c:306) | `return ERROR(dstSize_tooSmall); /* Buffer overflow */` | `ERROR(dstSize_tooSmall)` | [x] |
| 166 | `FSE_writeNCount_generic` (c_src/src/compress/fse_compress.c:315) | `return ERROR(GENERIC); /* incorrect normalized distribution */` | `ERROR(GENERIC)` | [x] |
| 167 | `FSE_writeNCount_generic` (c_src/src/compress/fse_compress.c:316) | `assert(symbol <= alphabetSize);` | process assertion failure | [x] |
| 168 | `FSE_writeNCount_generic` (c_src/src/compress/fse_compress.c:320) | `return ERROR(dstSize_tooSmall); /* Buffer overflow */` | `ERROR(dstSize_tooSmall)` | [x] |
| 169 | `FSE_writeNCount_generic` (c_src/src/compress/fse_compress.c:325) | `assert(out >= ostart);` | process assertion failure | [x] |
| 170 | `FSE_writeNCount` (c_src/src/compress/fse_compress.c:333) | `if (tableLog > FSE_MAX_TABLELOG) return ERROR(tableLog_tooLarge); /* Unsupported */` | `ERROR(tableLog_tooLarge)` | [x] |
| 171 | `FSE_writeNCount` (c_src/src/compress/fse_compress.c:334) | `if (tableLog < FSE_MIN_TABLELOG) return ERROR(GENERIC); /* Unsupported */` | `ERROR(GENERIC)` | [x] |
| 172 | `FSE_minTableLog` (c_src/src/compress/fse_compress.c:353) | `assert(srcSize > 1); /* Not supported, RLE should be used instead */` | process assertion failure | [x] |
| 173 | `FSE_optimalTableLog_internal` (c_src/src/compress/fse_compress.c:362) | `assert(srcSize > 1); /* Not supported, RLE should be used instead */` | process assertion failure | [x] |
| 174 | `FSE_normalizeM2` (c_src/src/compress/fse_compress.c:457) | `return ERROR(GENERIC);` | `ERROR(GENERIC)` | [x] |
| 175 | `FSE_normalizeCount` (c_src/src/compress/fse_compress.c:471) | `if (tableLog < FSE_MIN_TABLELOG) return ERROR(GENERIC); /* Unsupported size */` | `ERROR(GENERIC)` | [x] |
| 176 | `FSE_normalizeCount` (c_src/src/compress/fse_compress.c:472) | `if (tableLog > FSE_MAX_TABLELOG) return ERROR(tableLog_tooLarge); /* Unsupported size */` | `ERROR(tableLog_tooLarge)` | [x] |
| 177 | `FSE_normalizeCount` (c_src/src/compress/fse_compress.c:473) | `if (tableLog < FSE_minTableLog(total, maxSymbolValue)) return ERROR(GENERIC); /* Too small tableLog, compression potentially impossible */` | `ERROR(GENERIC)` | [x] |
| 178 | `HIST_count_simple` (c_src/src/compress/hist.c:51) | `assert(*ip <= maxSymbolValue);` | process assertion failure | [x] |
| 179 | `HIST_count_parallel_wksp` (c_src/src/compress/hist.c:92) | `assert(*maxSymbolValuePtr <= 255);` | process assertion failure | [x] |
| 180 | `HIST_count_parallel_wksp` (c_src/src/compress/hist.c:138) | `if (check && maxSymbolValue > *maxSymbolValuePtr) return ERROR(maxSymbolValue_tooSmall);` | `ERROR(maxSymbolValue_tooSmall)` | [x] |
| 181 | `HIST_countFast_wksp` (c_src/src/compress/hist.c:156) | `if ((size_t)workSpace & 3) return ERROR(GENERIC); /* must be aligned on 4-bytes boundaries */` | `ERROR(GENERIC)` | [x] |
| 182 | `HIST_countFast_wksp` (c_src/src/compress/hist.c:157) | `if (workSpaceSize < HIST_WKSP_SIZE) return ERROR(workSpace_tooSmall);` | `ERROR(workSpace_tooSmall)` | [x] |
| 183 | `HIST_count_wksp` (c_src/src/compress/hist.c:168) | `if ((size_t)workSpace & 3) return ERROR(GENERIC); /* must be aligned on 4-bytes boundaries */` | `ERROR(GENERIC)` | [x] |
| 184 | `HIST_count_wksp` (c_src/src/compress/hist.c:169) | `if (workSpaceSize < HIST_WKSP_SIZE) return ERROR(workSpace_tooSmall);` | `ERROR(workSpace_tooSmall)` | [x] |
| 185 | `(file scope)` (c_src/src/compress/huf_compress.c:41) | `#define HUF_STATIC_ASSERT(c) DEBUG_STATIC_ASSERT(c) /* use only *after* variable declarations */` | process assertion failure | [x] |
| 186 | `HUF_alignUpWorkspace` (c_src/src/compress/huf_compress.c:118) | `assert((align & (align - 1)) == 0); /* pow 2 */` | process assertion failure | [x] |
| 187 | `HUF_alignUpWorkspace` (c_src/src/compress/huf_compress.c:119) | `assert(align <= HUF_WORKSPACE_MAX_ALIGNMENT);` | process assertion failure | [x] |
| 188 | `HUF_alignUpWorkspace` (c_src/src/compress/huf_compress.c:121) | `assert(add < align);` | process assertion failure | [x] |
| 189 | `HUF_alignUpWorkspace` (c_src/src/compress/huf_compress.c:122) | `assert(((size_t)aligned & mask) == 0);` | process assertion failure | [x] |
| 190 | `HUF_alignUpWorkspace` (c_src/src/compress/huf_compress.c:127) | `return NULL;` | `NULL` | [x] |
| 191 | `HUF_compressWeights` (c_src/src/compress/huf_compress.c:159) | `if (workspaceSize < sizeof(HUF_CompressWeightsWksp)) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [x] |
| 192 | `HUF_setNbBits` (c_src/src/compress/huf_compress.c:210) | `assert(nbBits <= HUF_TABLELOG_ABSOLUTEMAX);` | process assertion failure | [x] |
| 193 | `HUF_setValue` (c_src/src/compress/huf_compress.c:218) | `assert((value >> nbBits) == 0);` | process assertion failure | [x] |
| 194 | `HUF_writeCTableHeader` (c_src/src/compress/huf_compress.c:235) | `assert(tableLog < 256);` | process assertion failure | [x] |
| 195 | `HUF_writeCTableHeader` (c_src/src/compress/huf_compress.c:237) | `assert(maxSymbolValue < 256);` | process assertion failure | [x] |
| 196 | `HUF_writeCTable_wksp` (c_src/src/compress/huf_compress.c:259) | `assert(HUF_readCTableHeader(CTable).maxSymbolValue == maxSymbolValue);` | process assertion failure | [x] |
| 197 | `HUF_writeCTable_wksp` (c_src/src/compress/huf_compress.c:260) | `assert(HUF_readCTableHeader(CTable).tableLog == huffLog);` | process assertion failure | [x] |
| 198 | `HUF_writeCTable_wksp` (c_src/src/compress/huf_compress.c:263) | `if (workspaceSize < sizeof(HUF_WriteCTableWksp)) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [x] |
| 199 | `HUF_writeCTable_wksp` (c_src/src/compress/huf_compress.c:264) | `if (maxSymbolValue > HUF_SYMBOLVALUE_MAX) return ERROR(maxSymbolValue_tooLarge);` | `ERROR(maxSymbolValue_tooLarge)` | [x] |
| 200 | `HUF_writeCTable_wksp` (c_src/src/compress/huf_compress.c:274) | `if (maxDstSize < 1) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 201 | `HUF_writeCTable_wksp` (c_src/src/compress/huf_compress.c:282) | `if (maxSymbolValue > (256-128)) return ERROR(GENERIC); /* should not happen : likely means source cannot be compressed */` | `ERROR(GENERIC)` | [x] |
| 202 | `HUF_writeCTable_wksp` (c_src/src/compress/huf_compress.c:283) | `if (((maxSymbolValue+1)/2) + 1 > maxDstSize) return ERROR(dstSize_tooSmall); /* not enough space within dst buffer */` | `ERROR(dstSize_tooSmall)` | [x] |
| 203 | `HUF_readCTable` (c_src/src/compress/huf_compress.c:305) | `if (tableLog > HUF_TABLELOG_MAX) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [x] |
| 204 | `HUF_readCTable` (c_src/src/compress/huf_compress.c:306) | `if (nbSymbols > *maxSymbolValuePtr+1) return ERROR(maxSymbolValue_tooSmall);` | `ERROR(maxSymbolValue_tooSmall)` | [x] |
| 205 | `HUF_getNbBitsFromCTable` (c_src/src/compress/huf_compress.c:348) | `assert(symbolValue <= HUF_SYMBOLVALUE_MAX);` | process assertion failure | [x] |
| 206 | `HUF_setMaxHeight` (c_src/src/compress/huf_compress.c:399) | `assert(huffNode[n].nbBits <= targetNbBits);` | process assertion failure | [x] |
| 207 | `HUF_setMaxHeight` (c_src/src/compress/huf_compress.c:405) | `assert(((U32)totalCost & (baseCost - 1)) == 0);` | process assertion failure | [x] |
| 208 | `HUF_setMaxHeight` (c_src/src/compress/huf_compress.c:407) | `assert(totalCost > 0);` | process assertion failure | [x] |
| 209 | `HUF_setMaxHeight` (c_src/src/compress/huf_compress.c:441) | `assert(rankLast[nBitsToDecrease] != noSymbol \|\| nBitsToDecrease == 1);` | process assertion failure | [x] |
| 210 | `HUF_setMaxHeight` (c_src/src/compress/huf_compress.c:445) | `assert(rankLast[nBitsToDecrease] != noSymbol);` | process assertion failure | [x] |
| 211 | `HUF_setMaxHeight` (c_src/src/compress/huf_compress.c:485) | `assert(n >= 0);` | process assertion failure | [x] |
| 212 | `HUF_sort` (c_src/src/compress/huf_compress.c:633) | `assert(lowerRank < RANK_POSITION_TABLE_SIZE - 1);` | process assertion failure | [x] |
| 213 | `HUF_sort` (c_src/src/compress/huf_compress.c:637) | `assert(rankPosition[RANK_POSITION_TABLE_SIZE - 1].base == 0);` | process assertion failure | [x] |
| 214 | `HUF_sort` (c_src/src/compress/huf_compress.c:649) | `assert(pos < maxSymbolValue1);` | process assertion failure | [x] |
| 215 | `HUF_sort` (c_src/src/compress/huf_compress.c:659) | `assert(bucketStartIdx < maxSymbolValue1);` | process assertion failure | [x] |
| 216 | `HUF_sort` (c_src/src/compress/huf_compress.c:664) | `assert(HUF_isSorted(huffNode, maxSymbolValue1));` | process assertion failure | [x] |
| 217 | `HUF_buildCTable_wksp` (c_src/src/compress/huf_compress.c:771) | `return ERROR(workSpace_tooSmall);` | `ERROR(workSpace_tooSmall)` | [x] |
| 218 | `HUF_buildCTable_wksp` (c_src/src/compress/huf_compress.c:774) | `return ERROR(maxSymbolValue_tooLarge);` | `ERROR(maxSymbolValue_tooLarge)` | [x] |
| 219 | `HUF_buildCTable_wksp` (c_src/src/compress/huf_compress.c:786) | `if (maxNbBits > HUF_TABLELOG_MAX) return ERROR(GENERIC); /* check fit into table */` | `ERROR(GENERIC)` | [x] |
| 220 | `HUF_validateCTable` (c_src/src/compress/huf_compress.c:810) | `assert(header.tableLog <= HUF_TABLELOG_ABSOLUTEMAX);` | process assertion failure | [x] |
| 221 | `HUF_initCStream` (c_src/src/compress/huf_compress.c:863) | `if (dstCapacity <= sizeof(bitC->bitContainer[0])) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 222 | `HUF_addBits` (c_src/src/compress/huf_compress.c:879) | `assert(idx <= 1);` | process assertion failure | [x] |
| 223 | `HUF_addBits` (c_src/src/compress/huf_compress.c:880) | `assert(HUF_getNbBits(elt) <= HUF_TABLELOG_ABSOLUTEMAX);` | process assertion failure | [x] |
| 224 | `HUF_addBits` (c_src/src/compress/huf_compress.c:892) | `assert((bitC->bitPos[idx] & 0xFF) <= HUF_BITS_IN_CONTAINER);` | process assertion failure | [x] |
| 225 | `HUF_addBits` (c_src/src/compress/huf_compress.c:903) | `assert(((elt >> dirtyBits) << (dirtyBits + nbBits)) == 0);` | process assertion failure | [x] |
| 226 | `HUF_addBits` (c_src/src/compress/huf_compress.c:905) | `assert(!kFast \|\| (bitC->bitPos[idx] & 0xFF) <= HUF_BITS_IN_CONTAINER);` | process assertion failure | [x] |
| 227 | `HUF_mergeIndex1` (c_src/src/compress/huf_compress.c:923) | `assert((bitC->bitPos[1] & 0xFF) < HUF_BITS_IN_CONTAINER);` | process assertion failure | [x] |
| 228 | `HUF_mergeIndex1` (c_src/src/compress/huf_compress.c:927) | `assert((bitC->bitPos[0] & 0xFF) <= HUF_BITS_IN_CONTAINER);` | process assertion failure | [x] |
| 229 | `HUF_flushBits` (c_src/src/compress/huf_compress.c:946) | `assert(nbBits > 0);` | process assertion failure | [x] |
| 230 | `HUF_flushBits` (c_src/src/compress/huf_compress.c:947) | `assert(nbBits <= sizeof(bitC->bitContainer[0]) * 8);` | process assertion failure | [x] |
| 231 | `HUF_flushBits` (c_src/src/compress/huf_compress.c:948) | `assert(bitC->ptr <= bitC->endPtr);` | process assertion failure | [x] |
| 232 | `HUF_flushBits` (c_src/src/compress/huf_compress.c:951) | `assert(!kFast \|\| bitC->ptr <= bitC->endPtr);` | process assertion failure | [x] |
| 233 | `HUF_compress1X_usingCTable_internal_body_loop` (c_src/src/compress/huf_compress.c:1005) | `assert(n % kUnroll == 0);` | process assertion failure | [x] |
| 234 | `HUF_compress1X_usingCTable_internal_body_loop` (c_src/src/compress/huf_compress.c:1017) | `assert(n % (2 * kUnroll) == 0);` | process assertion failure | [x] |
| 235 | `HUF_compress1X_usingCTable_internal_body_loop` (c_src/src/compress/huf_compress.c:1040) | `assert(n == 0);` | process assertion failure | [x] |
| 236 | `HUF_compress1X_usingCTable_internal_body` (c_src/src/compress/huf_compress.c:1115) | `assert(bitC.ptr <= bitC.endPtr);` | process assertion failure | [x] |
| 237 | `HUF_compress4X_usingCTable_internal` (c_src/src/compress/huf_compress.c:1183) | `assert(op <= oend);` | process assertion failure | [x] |
| 238 | `HUF_compress4X_usingCTable_internal` (c_src/src/compress/huf_compress.c:1191) | `assert(op <= oend);` | process assertion failure | [x] |
| 239 | `HUF_compress4X_usingCTable_internal` (c_src/src/compress/huf_compress.c:1199) | `assert(op <= oend);` | process assertion failure | [x] |
| 240 | `HUF_compress4X_usingCTable_internal` (c_src/src/compress/huf_compress.c:1207) | `assert(op <= oend);` | process assertion failure | [x] |
| 241 | `HUF_compress4X_usingCTable_internal` (c_src/src/compress/huf_compress.c:1208) | `assert(ip <= iend);` | process assertion failure | [x] |
| 242 | `HUF_compressCTable_internal` (c_src/src/compress/huf_compress.c:1236) | `assert(op >= ostart);` | process assertion failure | [x] |
| 243 | `HUF_optimalTableLog` (c_src/src/compress/huf_compress.c:1281) | `assert(srcSize > 1); /* Not supported, RLE should be used instead */` | process assertion failure | [x] |
| 244 | `HUF_optimalTableLog` (c_src/src/compress/huf_compress.c:1282) | `assert(wkspSize >= sizeof(HUF_buildCTable_wksp_tables));` | process assertion failure | [x] |
| 245 | `HUF_optimalTableLog` (c_src/src/compress/huf_compress.c:1324) | `assert(optLog <= HUF_TABLELOG_MAX);` | process assertion failure | [x] |
| 246 | `HUF_compress_internal` (c_src/src/compress/huf_compress.c:1349) | `if (wkspSize < sizeof(*table)) return ERROR(workSpace_tooSmall);` | `ERROR(workSpace_tooSmall)` | [x] |
| 247 | `HUF_compress_internal` (c_src/src/compress/huf_compress.c:1352) | `if (srcSize > HUF_BLOCKSIZE_MAX) return ERROR(srcSize_wrong); /* current block size limit */` | `ERROR(srcSize_wrong)` | [x] |
| 248 | `HUF_compress_internal` (c_src/src/compress/huf_compress.c:1353) | `if (huffLog > HUF_TABLELOG_MAX) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [x] |
| 249 | `HUF_compress_internal` (c_src/src/compress/huf_compress.c:1354) | `if (maxSymbolValue > HUF_SYMBOLVALUE_MAX) return ERROR(maxSymbolValue_tooLarge);` | `ERROR(maxSymbolValue_tooLarge)` | [x] |
| 250 | `HUF_compress_internal` (c_src/src/compress/huf_compress.c:1366) | `DEBUG_STATIC_ASSERT(SUSPECT_INCOMPRESSIBLE_SAMPLE_RATIO >= 2);` | process assertion failure | [x] |
| 251 | `ZSTD_compressBound` (c_src/src/compress/zstd_compress.c:72) | `if (r==0) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 252 | `ZSTD_initCCtx` (c_src/src/compress/zstd_compress.c:104) | `assert(cctx != NULL);` | process assertion failure | [x] |
| 253 | `ZSTD_initCCtx` (c_src/src/compress/zstd_compress.c:109) | `assert(!ZSTD_isError(err));` | process assertion failure | [x] |
| 254 | `ZSTD_createCCtx_advanced` (c_src/src/compress/zstd_compress.c:116) | `ZSTD_STATIC_ASSERT(zcss_init==0);` | process assertion failure | [x] |
| 255 | `ZSTD_createCCtx_advanced` (c_src/src/compress/zstd_compress.c:117) | `ZSTD_STATIC_ASSERT(ZSTD_CONTENTSIZE_UNKNOWN==(0ULL - 1));` | process assertion failure | [x] |
| 256 | `ZSTD_createCCtx_advanced` (c_src/src/compress/zstd_compress.c:118) | `if ((!customMem.customAlloc) ^ (!customMem.customFree)) return NULL;` | `NULL` | [x] |
| 257 | `ZSTD_createCCtx_advanced` (c_src/src/compress/zstd_compress.c:120) | `if (!cctx) return NULL;` | `NULL` | [x] |
| 258 | `ZSTD_initStaticCCtx` (c_src/src/compress/zstd_compress.c:130) | `if (workspaceSize <= sizeof(ZSTD_CCtx)) return NULL; /* minimum size */` | `NULL` | [x] |
| 259 | `ZSTD_initStaticCCtx` (c_src/src/compress/zstd_compress.c:131) | `if ((size_t)workspace & 7) return NULL; /* must be 8-aligned */` | `NULL` | [x] |
| 260 | `ZSTD_initStaticCCtx` (c_src/src/compress/zstd_compress.c:135) | `if (cctx == NULL) return NULL;` | `NULL` | [x] |
| 261 | `ZSTD_initStaticCCtx` (c_src/src/compress/zstd_compress.c:142) | `if (!ZSTD_cwksp_check_available(&cctx->workspace, TMP_WORKSPACE_SIZE + 2 * sizeof(ZSTD_compressedBlockState_t))) return NULL;` | `NULL` | [x] |
| 262 | `ZSTD_freeCCtxContent` (c_src/src/compress/zstd_compress.c:172) | `assert(cctx != NULL);` | process assertion failure | [x] |
| 263 | `ZSTD_freeCCtxContent` (c_src/src/compress/zstd_compress.c:173) | `assert(cctx->staticSize == 0);` | process assertion failure | [x] |
| 264 | `ZSTD_freeCCtx` (c_src/src/compress/zstd_compress.c:185) | `RETURN_ERROR_IF(cctx->staticSize, memory_allocation,` | `ERROR(cctx)` | [x] |
| 265 | `ZSTD_rowMatchFinderUsed` (c_src/src/compress/zstd_compress.c:233) | `assert(mode != ZSTD_ps_auto);` | process assertion failure | [x] |
| 266 | `ZSTD_allocateChainTable` (c_src/src/compress/zstd_compress.c:258) | `assert(useRowMatchFinder != ZSTD_ps_auto);` | process assertion failure | [x] |
| 267 | `ZSTD_makeCCtxParamsFromCParams` (c_src/src/compress/zstd_compress.c:315) | `assert(cctxParams.ldmParams.hashLog >= cctxParams.ldmParams.bucketSizeLog);` | process assertion failure | [x] |
| 268 | `ZSTD_makeCCtxParamsFromCParams` (c_src/src/compress/zstd_compress.c:316) | `assert(cctxParams.ldmParams.hashRateLog < 32);` | process assertion failure | [x] |
| 269 | `ZSTD_makeCCtxParamsFromCParams` (c_src/src/compress/zstd_compress.c:324) | `assert(!ZSTD_checkCParams(cParams));` | process assertion failure | [x] |
| 270 | `ZSTD_createCCtxParams_advanced` (c_src/src/compress/zstd_compress.c:332) | `if ((!customMem.customAlloc) ^ (!customMem.customFree)) return NULL;` | `NULL` | [x] |
| 271 | `ZSTD_createCCtxParams_advanced` (c_src/src/compress/zstd_compress.c:335) | `if (!params) { return NULL; }` | `NULL` | [x] |
| 272 | `ZSTD_CCtxParams_init` (c_src/src/compress/zstd_compress.c:359) | `RETURN_ERROR_IF(!cctxParams, GENERIC, "NULL pointer!");` | source-declared rejection sentinel | [x] |
| 273 | `ZSTD_CCtxParams_init_internal` (c_src/src/compress/zstd_compress.c:377) | `assert(!ZSTD_checkCParams(params->cParams));` | process assertion failure | [x] |
| 274 | `ZSTD_CCtxParams_init_advanced` (c_src/src/compress/zstd_compress.c:397) | `RETURN_ERROR_IF(!cctxParams, GENERIC, "NULL pointer!");` | source-declared rejection sentinel | [x] |
| 275 | `ZSTD_CCtxParams_setZstdParams` (c_src/src/compress/zstd_compress.c:410) | `assert(!ZSTD_checkCParams(params->cParams));` | process assertion failure | [x] |
| 276 | `ZSTD_cParam_getBounds` (c_src/src/compress/zstd_compress.c:550) | `ZSTD_STATIC_ASSERT(ZSTD_f_zstd1 < ZSTD_f_zstd1_magicless);` | process assertion failure | [x] |
| 277 | `ZSTD_cParam_getBounds` (c_src/src/compress/zstd_compress.c:556) | `ZSTD_STATIC_ASSERT(ZSTD_dictDefaultAttach < ZSTD_dictForceLoad);` | process assertion failure | [x] |
| 278 | `ZSTD_cParam_getBounds` (c_src/src/compress/zstd_compress.c:562) | `ZSTD_STATIC_ASSERT(ZSTD_ps_auto < ZSTD_ps_enable && ZSTD_ps_enable < ZSTD_ps_disable);` | process assertion failure | [x] |
| 279 | `ZSTD_cParam_clampBounds` (c_src/src/compress/zstd_compress.c:653) | `RETURN_ERROR_IF(!ZSTD_cParam_withinBounds(cParam,val), \` | source-declared rejection sentinel | [x] |
| 280 | `ZSTD_CCtx_setParameter` (c_src/src/compress/zstd_compress.c:715) | `RETURN_ERROR(stage_wrong, "can only set params in cctx init stage");` | `ERROR(stage_wrong)` | [x] |
| 281 | `ZSTD_CCtx_setParameter` (c_src/src/compress/zstd_compress.c:721) | `RETURN_ERROR_IF((value!=0) && cctx->staticSize, parameter_unsupported,` | source-declared rejection sentinel | [x] |
| 282 | `ZSTD_CCtx_setParameter` (c_src/src/compress/zstd_compress.c:765) | `default: RETURN_ERROR(parameter_unsupported, "unknown parameter");` | `ERROR(parameter_unsupported)` | [x] |
| 283 | `ZSTD_CCtxParams_setParameter` (c_src/src/compress/zstd_compress.c:868) | `RETURN_ERROR_IF(value!=0, parameter_unsupported, "not compiled with multithreading");` | `ERROR(value)` | [x] |
| 284 | `ZSTD_CCtxParams_setParameter` (c_src/src/compress/zstd_compress.c:878) | `RETURN_ERROR_IF(value!=0, parameter_unsupported, "not compiled with multithreading");` | `ERROR(value)` | [x] |
| 285 | `ZSTD_CCtxParams_setParameter` (c_src/src/compress/zstd_compress.c:885) | `assert(value >= 0);` | process assertion failure | [x] |
| 286 | `ZSTD_CCtxParams_setParameter` (c_src/src/compress/zstd_compress.c:892) | `RETURN_ERROR_IF(value!=0, parameter_unsupported, "not compiled with multithreading");` | `ERROR(value)` | [x] |
| 287 | `ZSTD_CCtxParams_setParameter` (c_src/src/compress/zstd_compress.c:902) | `RETURN_ERROR_IF(value!=0, parameter_unsupported, "not compiled with multithreading");` | `ERROR(value)` | [x] |
| 288 | `ZSTD_CCtxParams_setParameter` (c_src/src/compress/zstd_compress.c:1010) | `assert(value>=0);` | process assertion failure | [x] |
| 289 | `ZSTD_CCtxParams_setParameter` (c_src/src/compress/zstd_compress.c:1019) | `default: RETURN_ERROR(parameter_unsupported, "unknown parameter");` | `ERROR(parameter_unsupported)` | [x] |
| 290 | `ZSTD_CCtxParams_getParameter` (c_src/src/compress/zstd_compress.c:1080) | `assert(CCtxParams->nbWorkers == 0);` | process assertion failure | [x] |
| 291 | `ZSTD_CCtxParams_getParameter` (c_src/src/compress/zstd_compress.c:1086) | `RETURN_ERROR(parameter_unsupported, "not compiled with multithreading");` | `ERROR(parameter_unsupported)` | [x] |
| 292 | `ZSTD_CCtxParams_getParameter` (c_src/src/compress/zstd_compress.c:1088) | `assert(CCtxParams->jobSize <= INT_MAX);` | process assertion failure | [x] |
| 293 | `ZSTD_CCtxParams_getParameter` (c_src/src/compress/zstd_compress.c:1094) | `RETURN_ERROR(parameter_unsupported, "not compiled with multithreading");` | `ERROR(parameter_unsupported)` | [x] |
| 294 | `ZSTD_CCtxParams_getParameter` (c_src/src/compress/zstd_compress.c:1101) | `RETURN_ERROR(parameter_unsupported, "not compiled with multithreading");` | `ERROR(parameter_unsupported)` | [x] |
| 295 | `ZSTD_CCtxParams_getParameter` (c_src/src/compress/zstd_compress.c:1166) | `default: RETURN_ERROR(parameter_unsupported, "unknown parameter");` | `ERROR(parameter_unsupported)` | [x] |
| 296 | `ZSTD_CCtx_setParametersUsingCCtxParams` (c_src/src/compress/zstd_compress.c:1182) | `RETURN_ERROR_IF(cctx->streamStage != zcss_init, stage_wrong,` | `ERROR(cctx)` | [x] |
| 297 | `ZSTD_CCtx_setParametersUsingCCtxParams` (c_src/src/compress/zstd_compress.c:1184) | `RETURN_ERROR_IF(cctx->cdict, stage_wrong,` | `ERROR(cctx)` | [x] |
| 298 | `ZSTD_CCtx_setCParams` (c_src/src/compress/zstd_compress.c:1194) | `ZSTD_STATIC_ASSERT(sizeof(cparams) == 7 * 4 /* all params are listed below */);` | process assertion failure | [x] |
| 299 | `ZSTD_CCtx_setFParams` (c_src/src/compress/zstd_compress.c:1210) | `ZSTD_STATIC_ASSERT(sizeof(fparams) == 3 * 4 /* all params are listed below */);` | process assertion failure | [x] |
| 300 | `ZSTD_CCtx_setPledgedSrcSize` (c_src/src/compress/zstd_compress.c:1233) | `RETURN_ERROR_IF(cctx->streamStage != zcss_init, stage_wrong,` | `ERROR(cctx)` | [x] |
| 301 | `ZSTD_initLocalDict` (c_src/src/compress/zstd_compress.c:1257) | `assert(dl->dictBuffer == NULL);` | process assertion failure | [x] |
| 302 | `ZSTD_initLocalDict` (c_src/src/compress/zstd_compress.c:1258) | `assert(dl->cdict == NULL);` | process assertion failure | [x] |
| 303 | `ZSTD_initLocalDict` (c_src/src/compress/zstd_compress.c:1259) | `assert(dl->dictSize == 0);` | process assertion failure | [x] |
| 304 | `ZSTD_initLocalDict` (c_src/src/compress/zstd_compress.c:1264) | `assert(cctx->cdict == dl->cdict);` | process assertion failure | [x] |
| 305 | `ZSTD_initLocalDict` (c_src/src/compress/zstd_compress.c:1267) | `assert(dl->dictSize > 0);` | process assertion failure | [x] |
| 306 | `ZSTD_initLocalDict` (c_src/src/compress/zstd_compress.c:1268) | `assert(cctx->cdict == NULL);` | process assertion failure | [x] |
| 307 | `ZSTD_initLocalDict` (c_src/src/compress/zstd_compress.c:1269) | `assert(cctx->prefixDict.dict == NULL);` | process assertion failure | [x] |
| 308 | `ZSTD_initLocalDict` (c_src/src/compress/zstd_compress.c:1278) | `RETURN_ERROR_IF(!dl->cdict, memory_allocation, "ZSTD_createCDict_advanced failed");` | source-declared rejection sentinel | [x] |
| 309 | `ZSTD_CCtx_loadDictionary_advanced` (c_src/src/compress/zstd_compress.c:1290) | `RETURN_ERROR_IF(cctx->streamStage != zcss_init, stage_wrong,` | `ERROR(cctx)` | [x] |
| 310 | `ZSTD_CCtx_loadDictionary_advanced` (c_src/src/compress/zstd_compress.c:1300) | `RETURN_ERROR_IF(cctx->staticSize, memory_allocation,` | `ERROR(cctx)` | [x] |
| 311 | `ZSTD_CCtx_loadDictionary_advanced` (c_src/src/compress/zstd_compress.c:1303) | `RETURN_ERROR_IF(dictBuffer==NULL, memory_allocation,` | `ERROR(dictBuffer)` | [x] |
| 312 | `ZSTD_CCtx_refCDict` (c_src/src/compress/zstd_compress.c:1330) | `RETURN_ERROR_IF(cctx->streamStage != zcss_init, stage_wrong,` | `ERROR(cctx)` | [x] |
| 313 | `ZSTD_CCtx_refThreadPool` (c_src/src/compress/zstd_compress.c:1340) | `RETURN_ERROR_IF(cctx->streamStage != zcss_init, stage_wrong,` | `ERROR(cctx)` | [x] |
| 314 | `ZSTD_CCtx_refPrefix_advanced` (c_src/src/compress/zstd_compress.c:1354) | `RETURN_ERROR_IF(cctx->streamStage != zcss_init, stage_wrong,` | `ERROR(cctx)` | [x] |
| 315 | `ZSTD_CCtx_reset` (c_src/src/compress/zstd_compress.c:1376) | `RETURN_ERROR_IF(cctx->streamStage != zcss_init, stage_wrong,` | `ERROR(cctx)` | [x] |
| 316 | `ZSTD_dictAndWindowLog` (c_src/src/compress/zstd_compress.c:1446) | `assert(windowLog <= ZSTD_WINDOWLOG_MAX);` | process assertion failure | [x] |
| 317 | `ZSTD_dictAndWindowLog` (c_src/src/compress/zstd_compress.c:1447) | `assert(srcSize != ZSTD_CONTENTSIZE_UNKNOWN); /* Handled in ZSTD_adjustCParams_internal() */` | process assertion failure | [x] |
| 318 | `ZSTD_adjustCParams_internal` (c_src/src/compress/zstd_compress.c:1481) | `assert(ZSTD_checkCParams(cPar)==0);` | process assertion failure | [x] |
| 319 | `ZSTD_adjustCParams_internal` (c_src/src/compress/zstd_compress.c:1548) | `assert(0);` | process assertion failure | [x] |
| 320 | `ZSTD_adjustCParams_internal` (c_src/src/compress/zstd_compress.c:1602) | `assert(cPar.hashLog >= rowLog);` | process assertion failure | [x] |
| 321 | `ZSTD_getCParamsFromCCtxParams` (c_src/src/compress/zstd_compress.c:1642) | `assert(CCtxParams->srcSizeHint>=0);` | process assertion failure | [x] |
| 322 | `ZSTD_getCParamsFromCCtxParams` (c_src/src/compress/zstd_compress.c:1648) | `assert(!ZSTD_checkCParams(cParams));` | process assertion failure | [x] |
| 323 | `ZSTD_sizeof_matchState` (c_src/src/compress/zstd_compress.c:1687) | `ZSTD_STATIC_ASSERT(ZSTD_HASHLOG_MIN >= 4 && ZSTD_WINDOWLOG_MIN >= 4 && ZSTD_CHAINLOG_MIN >= 4);` | process assertion failure | [x] |
| 324 | `ZSTD_sizeof_matchState` (c_src/src/compress/zstd_compress.c:1688) | `assert(useRowMatchFinder != ZSTD_ps_auto);` | process assertion failure | [x] |
| 325 | `ZSTD_estimateCCtxSize_usingCCtxParams` (c_src/src/compress/zstd_compress.c:1761) | `RETURN_ERROR_IF(params->nbWorkers > 0, GENERIC, "Estimate CCtx size is supported for single-threaded compression only.");` | `ERROR(params)` | [x] |
| 326 | `ZSTD_estimateCStreamSize_usingCCtxParams` (c_src/src/compress/zstd_compress.c:1813) | `RETURN_ERROR_IF(params->nbWorkers > 0, GENERIC, "Estimate CCtx size is supported for single-threaded compression only.");` | `ERROR(params)` | [x] |
| 327 | `ZSTD_getFrameProgression` (c_src/src/compress/zstd_compress.c:1879) | `if (buffered) assert(cctx->inBuffPos >= cctx->inToCompress);` | process assertion failure | [x] |
| 328 | `ZSTD_getFrameProgression` (c_src/src/compress/zstd_compress.c:1880) | `assert(buffered <= ZSTD_BLOCKSIZE_MAX);` | process assertion failure | [x] |
| 329 | `ZSTD_assertEqualCParams` (c_src/src/compress/zstd_compress.c:1909) | `assert(cParams1.windowLog == cParams2.windowLog);` | process assertion failure | [x] |
| 330 | `ZSTD_assertEqualCParams` (c_src/src/compress/zstd_compress.c:1910) | `assert(cParams1.chainLog == cParams2.chainLog);` | process assertion failure | [x] |
| 331 | `ZSTD_assertEqualCParams` (c_src/src/compress/zstd_compress.c:1911) | `assert(cParams1.hashLog == cParams2.hashLog);` | process assertion failure | [x] |
| 332 | `ZSTD_assertEqualCParams` (c_src/src/compress/zstd_compress.c:1912) | `assert(cParams1.searchLog == cParams2.searchLog);` | process assertion failure | [x] |
| 333 | `ZSTD_assertEqualCParams` (c_src/src/compress/zstd_compress.c:1913) | `assert(cParams1.minMatch == cParams2.minMatch);` | process assertion failure | [x] |
| 334 | `ZSTD_assertEqualCParams` (c_src/src/compress/zstd_compress.c:1914) | `assert(cParams1.targetLength == cParams2.targetLength);` | process assertion failure | [x] |
| 335 | `ZSTD_assertEqualCParams` (c_src/src/compress/zstd_compress.c:1915) | `assert(cParams1.strategy == cParams2.strategy);` | process assertion failure | [x] |
| 336 | `ZSTD_reset_matchState` (c_src/src/compress/zstd_compress.c:2003) | `assert(useRowMatchFinder != ZSTD_ps_auto);` | process assertion failure | [x] |
| 337 | `ZSTD_reset_matchState` (c_src/src/compress/zstd_compress.c:2014) | `assert(!ZSTD_cwksp_reserve_failed(ws)); /* check that allocation hasn't already failed */` | process assertion failure | [x] |
| 338 | `ZSTD_reset_matchState` (c_src/src/compress/zstd_compress.c:2023) | `RETURN_ERROR_IF(ZSTD_cwksp_reserve_failed(ws), memory_allocation,` | `ERROR(ZSTD_cwksp_reserve_failed)` | [x] |
| 339 | `ZSTD_reset_matchState` (c_src/src/compress/zstd_compress.c:2048) | `assert(cParams->hashLog >= rowLog);` | process assertion failure | [x] |
| 340 | `ZSTD_reset_matchState` (c_src/src/compress/zstd_compress.c:2066) | `RETURN_ERROR_IF(ZSTD_cwksp_reserve_failed(ws), memory_allocation,` | `ERROR(ZSTD_cwksp_reserve_failed)` | [x] |
| 341 | `ZSTD_resetCCtx_internal` (c_src/src/compress/zstd_compress.c:2110) | `assert(!ZSTD_isError(ZSTD_checkCParams(params->cParams)));` | process assertion failure | [x] |
| 342 | `ZSTD_resetCCtx_internal` (c_src/src/compress/zstd_compress.c:2120) | `assert(params->useRowMatchFinder != ZSTD_ps_auto);` | process assertion failure | [x] |
| 343 | `ZSTD_resetCCtx_internal` (c_src/src/compress/zstd_compress.c:2121) | `assert(params->postBlockSplitter != ZSTD_ps_auto);` | process assertion failure | [x] |
| 344 | `ZSTD_resetCCtx_internal` (c_src/src/compress/zstd_compress.c:2122) | `assert(params->ldmParams.enableLdm != ZSTD_ps_auto);` | process assertion failure | [x] |
| 345 | `ZSTD_resetCCtx_internal` (c_src/src/compress/zstd_compress.c:2123) | `assert(params->maxBlockSize != 0);` | process assertion failure | [x] |
| 346 | `ZSTD_resetCCtx_internal` (c_src/src/compress/zstd_compress.c:2127) | `assert(params->ldmParams.hashLog >= params->ldmParams.bucketSizeLog);` | process assertion failure | [x] |
| 347 | `ZSTD_resetCCtx_internal` (c_src/src/compress/zstd_compress.c:2128) | `assert(params->ldmParams.hashRateLog < 32);` | process assertion failure | [x] |
| 348 | `ZSTD_resetCCtx_internal` (c_src/src/compress/zstd_compress.c:2168) | `RETURN_ERROR_IF(zc->staticSize, memory_allocation, "static cctx : no resize");` | `ERROR(zc)` | [x] |
| 349 | `ZSTD_resetCCtx_internal` (c_src/src/compress/zstd_compress.c:2179) | `assert(ZSTD_cwksp_check_available(ws, 2 * sizeof(ZSTD_compressedBlockState_t)));` | process assertion failure | [x] |
| 350 | `ZSTD_resetCCtx_internal` (c_src/src/compress/zstd_compress.c:2181) | `RETURN_ERROR_IF(zc->blockState.prevCBlock == NULL, memory_allocation, "couldn't allocate prevCBlock");` | `ERROR(zc)` | [x] |
| 351 | `ZSTD_resetCCtx_internal` (c_src/src/compress/zstd_compress.c:2183) | `RETURN_ERROR_IF(zc->blockState.nextCBlock == NULL, memory_allocation, "couldn't allocate nextCBlock");` | `ERROR(zc)` | [x] |
| 352 | `ZSTD_resetCCtx_internal` (c_src/src/compress/zstd_compress.c:2185) | `RETURN_ERROR_IF(zc->tmpWorkspace == NULL, memory_allocation, "couldn't allocate tmpWorkspace");` | `ERROR(zc)` | [x] |
| 353 | `ZSTD_resetCCtx_internal` (c_src/src/compress/zstd_compress.c:2274) | `assert(ZSTD_cwksp_estimated_space_within_bounds(ws, neededSpace));` | process assertion failure | [x] |
| 354 | `ZSTD_invalidateRepCodes` (c_src/src/compress/zstd_compress.c:2289) | `assert(!ZSTD_window_hasExtDict(cctx->blockState.matchState.window));` | process assertion failure | [x] |
| 355 | `ZSTD_resetCCtx_byAttachingCDict` (c_src/src/compress/zstd_compress.c:2336) | `assert(windowLog != 0);` | process assertion failure | [x] |
| 356 | `ZSTD_resetCCtx_byAttachingCDict` (c_src/src/compress/zstd_compress.c:2353) | `assert(cctx->appliedParams.cParams.strategy == adjusted_cdict_cParams.strategy);` | process assertion failure | [x] |
| 357 | `ZSTD_resetCCtx_byCopyingCDict` (c_src/src/compress/zstd_compress.c:2410) | `assert(!cdict->matchState.dedicatedDictSearch);` | process assertion failure | [x] |
| 358 | `ZSTD_resetCCtx_byCopyingCDict` (c_src/src/compress/zstd_compress.c:2415) | `assert(windowLog != 0);` | process assertion failure | [x] |
| 359 | `ZSTD_resetCCtx_byCopyingCDict` (c_src/src/compress/zstd_compress.c:2423) | `assert(cctx->appliedParams.cParams.strategy == cdict_cParams->strategy);` | process assertion failure | [x] |
| 360 | `ZSTD_resetCCtx_byCopyingCDict` (c_src/src/compress/zstd_compress.c:2424) | `assert(cctx->appliedParams.cParams.hashLog == cdict_cParams->hashLog);` | process assertion failure | [x] |
| 361 | `ZSTD_resetCCtx_byCopyingCDict` (c_src/src/compress/zstd_compress.c:2425) | `assert(cctx->appliedParams.cParams.chainLog == cdict_cParams->chainLog);` | process assertion failure | [x] |
| 362 | `ZSTD_resetCCtx_byCopyingCDict` (c_src/src/compress/zstd_compress.c:2429) | `assert(params.useRowMatchFinder != ZSTD_ps_auto);` | process assertion failure | [x] |
| 363 | `ZSTD_resetCCtx_byCopyingCDict` (c_src/src/compress/zstd_compress.c:2458) | `assert(cctx->blockState.matchState.hashLog3 <= 31);` | process assertion failure | [x] |
| 364 | `ZSTD_resetCCtx_byCopyingCDict` (c_src/src/compress/zstd_compress.c:2461) | `assert(cdict->matchState.hashLog3 == 0);` | process assertion failure | [x] |
| 365 | `ZSTD_copyCCtx_internal` (c_src/src/compress/zstd_compress.c:2519) | `RETURN_ERROR_IF(srcCCtx->stage!=ZSTDcs_init, stage_wrong,` | `ERROR(srcCCtx)` | [x] |
| 366 | `ZSTD_copyCCtx_internal` (c_src/src/compress/zstd_compress.c:2526) | `assert(srcCCtx->appliedParams.useRowMatchFinder != ZSTD_ps_auto);` | process assertion failure | [x] |
| 367 | `ZSTD_copyCCtx_internal` (c_src/src/compress/zstd_compress.c:2527) | `assert(srcCCtx->appliedParams.postBlockSplitter != ZSTD_ps_auto);` | process assertion failure | [x] |
| 368 | `ZSTD_copyCCtx_internal` (c_src/src/compress/zstd_compress.c:2528) | `assert(srcCCtx->appliedParams.ldmParams.enableLdm != ZSTD_ps_auto);` | process assertion failure | [x] |
| 369 | `ZSTD_copyCCtx_internal` (c_src/src/compress/zstd_compress.c:2537) | `assert(dstCCtx->appliedParams.cParams.windowLog == srcCCtx->appliedParams.cParams.windowLog);` | process assertion failure | [x] |
| 370 | `ZSTD_copyCCtx_internal` (c_src/src/compress/zstd_compress.c:2538) | `assert(dstCCtx->appliedParams.cParams.strategy == srcCCtx->appliedParams.cParams.strategy);` | process assertion failure | [x] |
| 371 | `ZSTD_copyCCtx_internal` (c_src/src/compress/zstd_compress.c:2539) | `assert(dstCCtx->appliedParams.cParams.hashLog == srcCCtx->appliedParams.cParams.hashLog);` | process assertion failure | [x] |
| 372 | `ZSTD_copyCCtx_internal` (c_src/src/compress/zstd_compress.c:2540) | `assert(dstCCtx->appliedParams.cParams.chainLog == srcCCtx->appliedParams.cParams.chainLog);` | process assertion failure | [x] |
| 373 | `ZSTD_copyCCtx_internal` (c_src/src/compress/zstd_compress.c:2541) | `assert(dstCCtx->blockState.matchState.hashLog3 == srcCCtx->blockState.matchState.hashLog3);` | process assertion failure | [x] |
| 374 | `ZSTD_copyCCtx` (c_src/src/compress/zstd_compress.c:2595) | `ZSTD_STATIC_ASSERT((U32)ZSTDb_buffered==1);` | process assertion failure | [x] |
| 375 | `ZSTD_reduceTable_internal` (c_src/src/compress/zstd_compress.c:2620) | `assert((size & (ZSTD_ROWSIZE-1)) == 0); /* multiple of ZSTD_ROWSIZE */` | process assertion failure | [x] |
| 376 | `ZSTD_reduceTable_internal` (c_src/src/compress/zstd_compress.c:2621) | `assert(size < (1U<<31)); /* can be cast to int */` | process assertion failure | [x] |
| 377 | `ZSTD_seqToCodes` (c_src/src/compress/zstd_compress.c:2702) | `assert(nbSeq <= seqStorePtr->maxNbSeq);` | process assertion failure | [x] |
| 378 | `ZSTD_seqToCodes` (c_src/src/compress/zstd_compress.c:2710) | `assert(!(MEM_64bits() && ofCode >= STREAM_ACCUMULATOR_MIN));` | process assertion failure | [x] |
| 379 | `ZSTD_blockSplitterEnabled` (c_src/src/compress/zstd_compress.c:2739) | `assert(cctxParams->postBlockSplitter != ZSTD_ps_auto);` | process assertion failure | [x] |
| 380 | `ZSTD_buildSequencesStatistics` (c_src/src/compress/zstd_compress.c:2784) | `assert(op <= oend);` | process assertion failure | [x] |
| 381 | `ZSTD_buildSequencesStatistics` (c_src/src/compress/zstd_compress.c:2785) | `assert(nbSeq != 0); /* ZSTD_selectEncodingType() divides by nbSeq */` | process assertion failure | [x] |
| 382 | `ZSTD_buildSequencesStatistics` (c_src/src/compress/zstd_compress.c:2796) | `assert(set_basic < set_compressed && set_rle < set_compressed);` | process assertion failure | [x] |
| 383 | `ZSTD_buildSequencesStatistics` (c_src/src/compress/zstd_compress.c:2797) | `assert(!(stats.LLtype < set_compressed && nextEntropy->litlength_repeatMode != FSE_repeat_none)); /* We don't copy tables */` | process assertion failure | [x] |
| 384 | `ZSTD_buildSequencesStatistics` (c_src/src/compress/zstd_compress.c:2814) | `assert(op <= oend);` | process assertion failure | [x] |
| 385 | `ZSTD_buildSequencesStatistics` (c_src/src/compress/zstd_compress.c:2829) | `assert(!(stats.Offtype < set_compressed && nextEntropy->offcode_repeatMode != FSE_repeat_none)); /* We don't copy tables */` | process assertion failure | [x] |
| 386 | `ZSTD_buildSequencesStatistics` (c_src/src/compress/zstd_compress.c:2846) | `assert(op <= oend);` | process assertion failure | [x] |
| 387 | `ZSTD_buildSequencesStatistics` (c_src/src/compress/zstd_compress.c:2859) | `assert(!(stats.MLtype < set_compressed && nextEntropy->matchlength_repeatMode != FSE_repeat_none)); /* We don't copy tables */` | process assertion failure | [x] |
| 388 | `ZSTD_buildSequencesStatistics` (c_src/src/compress/zstd_compress.c:2876) | `assert(op <= oend);` | process assertion failure | [x] |
| 389 | `ZSTD_entropyCompressSeqStore_internal` (c_src/src/compress/zstd_compress.c:2918) | `ZSTD_STATIC_ASSERT(HUF_WORKSPACE_SIZE >= (1<<MAX(MLFSELog,LLFSELog)));` | process assertion failure | [x] |
| 390 | `ZSTD_entropyCompressSeqStore_internal` (c_src/src/compress/zstd_compress.c:2919) | `assert(entropyWkspSize >= HUF_WORKSPACE_SIZE);` | process assertion failure | [x] |
| 391 | `ZSTD_entropyCompressSeqStore_internal` (c_src/src/compress/zstd_compress.c:2935) | `assert(cSize <= dstCapacity);` | process assertion failure | [x] |
| 392 | `ZSTD_entropyCompressSeqStore_internal` (c_src/src/compress/zstd_compress.c:2940) | `RETURN_ERROR_IF((oend-op) < 3 /*max nbSeq Size*/ + 1 /*seqHead*/,` | source-declared rejection sentinel | [x] |
| 393 | `ZSTD_entropyCompressSeqStore_internal` (c_src/src/compress/zstd_compress.c:2953) | `assert(op <= oend);` | process assertion failure | [x] |
| 394 | `ZSTD_entropyCompressSeqStore_internal` (c_src/src/compress/zstd_compress.c:2983) | `assert(op <= oend);` | process assertion failure | [x] |
| 395 | `ZSTD_entropyCompressSeqStore_internal` (c_src/src/compress/zstd_compress.c:2994) | `assert(lastCountSize + bitstreamSize == 3);` | process assertion failure | [x] |
| 396 | `ZSTD_entropyCompressSeqStore_wExtLitBuffer` (c_src/src/compress/zstd_compress.c:3040) | `assert(cSize < ZSTD_BLOCKSIZE_MAX);` | process assertion failure | [x] |
| 397 | `ZSTD_selectBlockCompressor` (c_src/src/compress/zstd_compress.c:3117) | `ZSTD_STATIC_ASSERT((unsigned)ZSTD_fast == 1);` | process assertion failure | [x] |
| 398 | `ZSTD_selectBlockCompressor` (c_src/src/compress/zstd_compress.c:3119) | `assert(ZSTD_cParam_withinBounds(ZSTD_c_strategy, (int)strat));` | process assertion failure | [x] |
| 399 | `ZSTD_selectBlockCompressor` (c_src/src/compress/zstd_compress.c:3145) | `assert(useRowMatchFinder != ZSTD_ps_auto);` | process assertion failure | [x] |
| 400 | `ZSTD_selectBlockCompressor` (c_src/src/compress/zstd_compress.c:3150) | `assert(selectedCompressor != NULL);` | process assertion failure | [x] |
| 401 | `ZSTD_postProcessSequenceProducerResult` (c_src/src/compress/zstd_compress.c:3177) | `RETURN_ERROR_IF(` | source-declared rejection sentinel | [x] |
| 402 | `ZSTD_postProcessSequenceProducerResult` (c_src/src/compress/zstd_compress.c:3184) | `RETURN_ERROR_IF(` | source-declared rejection sentinel | [x] |
| 403 | `ZSTD_postProcessSequenceProducerResult` (c_src/src/compress/zstd_compress.c:3205) | `RETURN_ERROR_IF(` | source-declared rejection sentinel | [x] |
| 404 | `ZSTD_validateSeqStore` (c_src/src/compress/zstd_compress.c:3245) | `assert(seqLength.matchLength >= matchLenLowerBound);` | process assertion failure | [x] |
| 405 | `ZSTD_buildSeqStore` (c_src/src/compress/zstd_compress.c:3268) | `assert(srcSize <= ZSTD_BLOCKSIZE_MAX);` | process assertion failure | [x] |
| 406 | `ZSTD_buildSeqStore` (c_src/src/compress/zstd_compress.c:3289) | `assert(ms->dictMatchState == NULL \|\| ms->loadedDictEnd == ms->window.dictLimit);` | process assertion failure | [x] |
| 407 | `ZSTD_buildSeqStore` (c_src/src/compress/zstd_compress.c:3295) | `if (sizeof(ptrdiff_t)==8) assert(istart - base < (ptrdiff_t)(U32)(-1)); /* ensure no overflow */` | process assertion failure | [x] |
| 408 | `ZSTD_buildSeqStore` (c_src/src/compress/zstd_compress.c:3308) | `assert(zc->appliedParams.ldmParams.enableLdm == ZSTD_ps_disable);` | process assertion failure | [x] |
| 409 | `ZSTD_buildSeqStore` (c_src/src/compress/zstd_compress.c:3312) | `RETURN_ERROR_IF(` | source-declared rejection sentinel | [x] |
| 410 | `ZSTD_buildSeqStore` (c_src/src/compress/zstd_compress.c:3325) | `assert(zc->externSeqStore.pos <= zc->externSeqStore.size);` | process assertion failure | [x] |
| 411 | `ZSTD_buildSeqStore` (c_src/src/compress/zstd_compress.c:3331) | `RETURN_ERROR_IF(` | source-declared rejection sentinel | [x] |
| 412 | `ZSTD_buildSeqStore` (c_src/src/compress/zstd_compress.c:3350) | `assert(ldmSeqStore.pos == ldmSeqStore.size);` | process assertion failure | [x] |
| 413 | `ZSTD_buildSeqStore` (c_src/src/compress/zstd_compress.c:3352) | `assert(` | process assertion failure | [x] |
| 414 | `ZSTD_buildSeqStore` (c_src/src/compress/zstd_compress.c:3355) | `assert(zc->appliedParams.extSeqProdFunc != NULL);` | process assertion failure | [x] |
| 415 | `ZSTD_buildSeqStore` (c_src/src/compress/zstd_compress.c:3380) | `RETURN_ERROR_IF(seqLenSum > srcSize, externalSequences_invalid, "External sequences imply too large a block!");` | `ERROR(seqLenSum)` | [x] |
| 416 | `ZSTD_copyBlockSequences` (c_src/src/compress/zstd_compress.c:3444) | `assert(seqCollector->seqIndex <= seqCollector->maxSequences);` | process assertion failure | [x] |
| 417 | `ZSTD_copyBlockSequences` (c_src/src/compress/zstd_compress.c:3445) | `RETURN_ERROR_IF(` | source-declared rejection sentinel | [x] |
| 418 | `ZSTD_copyBlockSequences` (c_src/src/compress/zstd_compress.c:3472) | `assert(repcode > 0);` | process assertion failure | [x] |
| 419 | `ZSTD_copyBlockSequences` (c_src/src/compress/zstd_compress.c:3478) | `assert(repcodes.rep[0] > 1);` | process assertion failure | [x] |
| 420 | `ZSTD_copyBlockSequences` (c_src/src/compress/zstd_compress.c:3500) | `assert(nbInLiterals >= nbOutLiterals);` | process assertion failure | [x] |
| 421 | `ZSTD_copyBlockSequences` (c_src/src/compress/zstd_compress.c:3506) | `assert(nbOutSequences == nbInSequences + 1);` | process assertion failure | [x] |
| 422 | `ZSTD_copyBlockSequences` (c_src/src/compress/zstd_compress.c:3509) | `assert(seqCollector->seqIndex <= seqCollector->maxSequences);` | process assertion failure | [x] |
| 423 | `ZSTD_generateSequences` (c_src/src/compress/zstd_compress.c:3529) | `RETURN_ERROR_IF(targetCBlockSize != 0, parameter_unsupported, "targetCBlockSize != 0");` | `ERROR(targetCBlockSize)` | [x] |
| 424 | `ZSTD_generateSequences` (c_src/src/compress/zstd_compress.c:3534) | `RETURN_ERROR_IF(nbWorkers != 0, parameter_unsupported, "nbWorkers != 0");` | `ERROR(nbWorkers)` | [x] |
| 425 | `ZSTD_generateSequences` (c_src/src/compress/zstd_compress.c:3538) | `RETURN_ERROR_IF(dst == NULL, memory_allocation, "NULL pointer!");` | `ERROR(dst)` | [x] |
| 426 | `ZSTD_generateSequences` (c_src/src/compress/zstd_compress.c:3551) | `assert(zc->seqCollector.seqIndex <= ZSTD_sequenceBound(srcSize));` | process assertion failure | [x] |
| 427 | `ZSTD_buildBlockEntropyStats_literals` (c_src/src/compress/zstd_compress.c:3701) | `assert(huffLog <= LitHufLog);` | process assertion failure | [x] |
| 428 | `ZSTD_estimateBlockSize_literal` (c_src/src/compress/zstd_compress.c:3851) | `assert(0); /* impossible */` | process assertion failure | [x] |
| 429 | `ZSTD_estimateBlockSize_symbolType` (c_src/src/compress/zstd_compress.c:3874) | `assert(max <= defaultMax);` | process assertion failure | [x] |
| 430 | `ZSTD_deriveSeqStoreChunk` (c_src/src/compress/zstd_compress.c:4023) | `assert(resultSeqStore->lit == originalSeqStore->lit);` | process assertion failure | [x] |
| 431 | `ZSTD_resolveRepcodeToRawOffset` (c_src/src/compress/zstd_compress.c:4041) | `assert(OFFBASE_IS_REPCODE(offBase));` | process assertion failure | [x] |
| 432 | `ZSTD_resolveRepcodeToRawOffset` (c_src/src/compress/zstd_compress.c:4043) | `assert(ll0);` | process assertion failure | [x] |
| 433 | `ZSTD_seqStore_resolveOffCodes` (c_src/src/compress/zstd_compress.c:4079) | `assert(offBase > 0);` | process assertion failure | [x] |
| 434 | `ZSTD_compressSeqStore_singleBlock` (c_src/src/compress/zstd_compress.c:4124) | `RETURN_ERROR_IF(dstCapacity < ZSTD_blockHeaderSize, dstSize_tooSmall, "Block header doesn't fit");` | `ERROR(dstCapacity)` | [x] |
| 435 | `ZSTD_deriveBlockSplitsHelper` (c_src/src/compress/zstd_compress.c:4209) | `assert(endIdx >= startIdx);` | process assertion failure | [x] |
| 436 | `ZSTD_compressBlock_splitBlock_internal` (c_src/src/compress/zstd_compress.c:4309) | `assert(zc->blockSizeMax <= ZSTD_BLOCKSIZE_MAX);` | process assertion failure | [x] |
| 437 | `ZSTD_compressBlock_splitBlock_internal` (c_src/src/compress/zstd_compress.c:4310) | `assert(cSizeSingleBlock <= zc->blockSizeMax + ZSTD_blockHeaderSize);` | process assertion failure | [x] |
| 438 | `ZSTD_compressBlock_splitBlock_internal` (c_src/src/compress/zstd_compress.c:4344) | `assert(cSizeChunk <= zc->blockSizeMax + ZSTD_blockHeaderSize);` | process assertion failure | [x] |
| 439 | `ZSTD_compressBlock_splitBlock` (c_src/src/compress/zstd_compress.c:4361) | `assert(zc->appliedParams.postBlockSplitter == ZSTD_ps_enable);` | process assertion failure | [x] |
| 440 | `ZSTD_compressBlock_splitBlock` (c_src/src/compress/zstd_compress.c:4368) | `RETURN_ERROR_IF(zc->seqCollector.collectSequences, sequenceProducer_failed, "Uncompressible block");` | `ERROR(zc)` | [x] |
| 441 | `ZSTD_compressBlock_internal` (c_src/src/compress/zstd_compress.c:4402) | `RETURN_ERROR_IF(zc->seqCollector.collectSequences, sequenceProducer_failed, "Uncompressible block");` | `ERROR(zc)` | [x] |
| 442 | `ZSTD_overflowCorrectIfNeeded` (c_src/src/compress/zstd_compress.c:4536) | `ZSTD_STATIC_ASSERT(ZSTD_CHAINLOG_MAX <= 30);` | process assertion failure | [x] |
| 443 | `ZSTD_overflowCorrectIfNeeded` (c_src/src/compress/zstd_compress.c:4537) | `ZSTD_STATIC_ASSERT(ZSTD_WINDOWLOG_MAX_32 <= 30);` | process assertion failure | [x] |
| 444 | `ZSTD_overflowCorrectIfNeeded` (c_src/src/compress/zstd_compress.c:4538) | `ZSTD_STATIC_ASSERT(ZSTD_WINDOWLOG_MAX <= 31);` | process assertion failure | [x] |
| 445 | `ZSTD_optimalBlockSize` (c_src/src/compress/zstd_compress.c:4575) | `assert(ZSTD_fast <= strat && strat <= ZSTD_btultra2);` | process assertion failure | [x] |
| 446 | `ZSTD_optimalBlockSize` (c_src/src/compress/zstd_compress.c:4578) | `assert(2 <= splitLevel && splitLevel <= 6);` | process assertion failure | [x] |
| 447 | `ZSTD_compress_frameChunk` (c_src/src/compress/zstd_compress.c:4604) | `assert(cctx->appliedParams.cParams.windowLog <= ZSTD_WINDOWLOG_MAX);` | process assertion failure | [x] |
| 448 | `ZSTD_compress_frameChunk` (c_src/src/compress/zstd_compress.c:4619) | `assert(blockSize <= remaining);` | process assertion failure | [x] |
| 449 | `ZSTD_compress_frameChunk` (c_src/src/compress/zstd_compress.c:4623) | `RETURN_ERROR_IF(dstCapacity < ZSTD_blockHeaderSize + MIN_CBLOCK_SIZE + 1,` | `ERROR(dstCapacity)` | [x] |
| 450 | `ZSTD_compress_frameChunk` (c_src/src/compress/zstd_compress.c:4639) | `assert(cSize > 0);` | process assertion failure | [x] |
| 451 | `ZSTD_compress_frameChunk` (c_src/src/compress/zstd_compress.c:4640) | `assert(cSize <= blockSize + ZSTD_blockHeaderSize);` | process assertion failure | [x] |
| 452 | `ZSTD_compress_frameChunk` (c_src/src/compress/zstd_compress.c:4644) | `assert(cSize > 0 \|\| cctx->seqCollector.collectSequences == 1);` | process assertion failure | [x] |
| 453 | `ZSTD_compress_frameChunk` (c_src/src/compress/zstd_compress.c:4680) | `assert(remaining >= blockSize);` | process assertion failure | [x] |
| 454 | `ZSTD_compress_frameChunk` (c_src/src/compress/zstd_compress.c:4683) | `assert(dstCapacity >= cSize);` | process assertion failure | [x] |
| 455 | `ZSTD_writeFrameHeader` (c_src/src/compress/zstd_compress.c:4711) | `assert(!(params->fParams.contentSizeFlag && pledgedSrcSize == ZSTD_CONTENTSIZE_UNKNOWN));` | process assertion failure | [x] |
| 456 | `ZSTD_writeFrameHeader` (c_src/src/compress/zstd_compress.c:4712) | `RETURN_ERROR_IF(dstCapacity < ZSTD_FRAMEHEADERSIZE_MAX, dstSize_tooSmall,` | `ERROR(dstCapacity)` | [x] |
| 457 | `ZSTD_writeFrameHeader` (c_src/src/compress/zstd_compress.c:4725) | `assert(0); /* impossible */` | process assertion failure | [x] |
| 458 | `ZSTD_writeFrameHeader` (c_src/src/compress/zstd_compress.c:4735) | `assert(0); /* impossible */` | process assertion failure | [x] |
| 459 | `ZSTD_writeSkippableFrame` (c_src/src/compress/zstd_compress.c:4754) | `RETURN_ERROR_IF(dstCapacity < srcSize + ZSTD_SKIPPABLEHEADERSIZE /* Skippable frame overhead */,` | `ERROR(dstCapacity)` | [x] |
| 460 | `ZSTD_writeSkippableFrame` (c_src/src/compress/zstd_compress.c:4756) | `RETURN_ERROR_IF(srcSize > (unsigned)0xFFFFFFFF, srcSize_wrong, "Src size too large for skippable frame");` | `ERROR(srcSize)` | [x] |
| 461 | `ZSTD_writeSkippableFrame` (c_src/src/compress/zstd_compress.c:4757) | `RETURN_ERROR_IF(magicVariant > 15, parameter_outOfBound, "Skippable frame magic number variant not supported");` | `ERROR(magicVariant)` | [x] |
| 462 | `ZSTD_writeLastEmptyBlock` (c_src/src/compress/zstd_compress.c:4772) | `RETURN_ERROR_IF(dstCapacity < ZSTD_blockHeaderSize, dstSize_tooSmall,` | `ERROR(dstCapacity)` | [x] |
| 463 | `ZSTD_referenceExternalSequences` (c_src/src/compress/zstd_compress.c:4782) | `assert(cctx->stage == ZSTDcs_init);` | process assertion failure | [x] |
| 464 | `ZSTD_referenceExternalSequences` (c_src/src/compress/zstd_compress.c:4783) | `assert(nbSeq == 0 \|\| cctx->appliedParams.ldmParams.enableLdm != ZSTD_ps_enable);` | process assertion failure | [x] |
| 465 | `ZSTD_compressContinue_internal` (c_src/src/compress/zstd_compress.c:4802) | `RETURN_ERROR_IF(cctx->stage==ZSTDcs_created, stage_wrong,` | `ERROR(cctx)` | [x] |
| 466 | `ZSTD_compressContinue_internal` (c_src/src/compress/zstd_compress.c:4809) | `assert(fhSize <= dstCapacity);` | process assertion failure | [x] |
| 467 | `ZSTD_compressContinue_internal` (c_src/src/compress/zstd_compress.c:4839) | `assert(!(cctx->appliedParams.fParams.contentSizeFlag && cctx->pledgedSrcSizePlusOne == 0));` | process assertion failure | [x] |
| 468 | `ZSTD_compressContinue_internal` (c_src/src/compress/zstd_compress.c:4841) | `ZSTD_STATIC_ASSERT(ZSTD_CONTENTSIZE_UNKNOWN == (unsigned long long)-1);` | process assertion failure | [x] |
| 469 | `ZSTD_compressContinue_internal` (c_src/src/compress/zstd_compress.c:4842) | `RETURN_ERROR_IF(` | source-declared rejection sentinel | [x] |
| 470 | `ZSTD_getBlockSize_deprecated` (c_src/src/compress/zstd_compress.c:4872) | `assert(!ZSTD_checkCParams(cParams));` | process assertion failure | [x] |
| 471 | `ZSTD_compressBlock_deprecated` (c_src/src/compress/zstd_compress.c:4887) | `RETURN_ERROR_IF(srcSize > blockSizeMax, srcSize_wrong, "input is larger than a block"); }` | `ERROR(srcSize)` | [x] |
| 472 | `ZSTD_loadDictionaryContent` (c_src/src/compress/zstd_compress.c:4934) | `assert(!loadLdmDict);` | process assertion failure | [x] |
| 473 | `ZSTD_loadDictionaryContent` (c_src/src/compress/zstd_compress.c:4947) | `assert(ZSTD_window_isEmpty(ms->window));` | process assertion failure | [x] |
| 474 | `ZSTD_loadDictionaryContent` (c_src/src/compress/zstd_compress.c:4948) | `if (loadLdmDict) assert(ZSTD_window_isEmpty(ls->window));` | process assertion failure | [x] |
| 475 | `ZSTD_loadDictionaryContent` (c_src/src/compress/zstd_compress.c:4988) | `assert(0); /* shouldn't be called: cparams should've been adjusted. */` | process assertion failure | [x] |
| 476 | `ZSTD_loadDictionaryContent` (c_src/src/compress/zstd_compress.c:4998) | `assert(srcSize >= HASH_READ_SIZE);` | process assertion failure | [x] |
| 477 | `ZSTD_loadDictionaryContent` (c_src/src/compress/zstd_compress.c:5000) | `assert(ms->chainTable != NULL);` | process assertion failure | [x] |
| 478 | `ZSTD_loadDictionaryContent` (c_src/src/compress/zstd_compress.c:5003) | `assert(params->useRowMatchFinder != ZSTD_ps_auto);` | process assertion failure | [x] |
| 479 | `ZSTD_loadDictionaryContent` (c_src/src/compress/zstd_compress.c:5015) | `assert(0); /* shouldn't be called: cparams should've been adjusted. */` | process assertion failure | [x] |
| 480 | `ZSTD_loadDictionaryContent` (c_src/src/compress/zstd_compress.c:5026) | `assert(srcSize >= HASH_READ_SIZE);` | process assertion failure | [x] |
| 481 | `ZSTD_loadDictionaryContent` (c_src/src/compress/zstd_compress.c:5030) | `assert(0); /* shouldn't be called: cparams should've been adjusted. */` | process assertion failure | [x] |
| 482 | `ZSTD_loadDictionaryContent` (c_src/src/compress/zstd_compress.c:5035) | `assert(0); /* not possible : not a valid strategy id */` | process assertion failure | [x] |
| 483 | `ZSTD_loadCEntropy` (c_src/src/compress/zstd_compress.c:5081) | `RETURN_ERROR_IF(HUF_isError(hufHeaderSize), dictionary_corrupted, "");` | `ERROR(HUF_isError)` | [x] |
| 484 | `ZSTD_loadCEntropy` (c_src/src/compress/zstd_compress.c:5087) | `RETURN_ERROR_IF(FSE_isError(offcodeHeaderSize), dictionary_corrupted, "");` | `ERROR(FSE_isError)` | [x] |
| 485 | `ZSTD_loadCEntropy` (c_src/src/compress/zstd_compress.c:5088) | `RETURN_ERROR_IF(offcodeLog > OffFSELog, dictionary_corrupted, "");` | `ERROR(offcodeLog)` | [x] |
| 486 | `ZSTD_loadCEntropy` (c_src/src/compress/zstd_compress.c:5090) | `RETURN_ERROR_IF(FSE_isError(FSE_buildCTable_wksp(` | `ERROR(FSE_isError)` | [x] |
| 487 | `ZSTD_loadCEntropy` (c_src/src/compress/zstd_compress.c:5102) | `RETURN_ERROR_IF(FSE_isError(matchlengthHeaderSize), dictionary_corrupted, "");` | `ERROR(FSE_isError)` | [x] |
| 488 | `ZSTD_loadCEntropy` (c_src/src/compress/zstd_compress.c:5103) | `RETURN_ERROR_IF(matchlengthLog > MLFSELog, dictionary_corrupted, "");` | `ERROR(matchlengthLog)` | [x] |
| 489 | `ZSTD_loadCEntropy` (c_src/src/compress/zstd_compress.c:5104) | `RETURN_ERROR_IF(FSE_isError(FSE_buildCTable_wksp(` | `ERROR(FSE_isError)` | [x] |
| 490 | `ZSTD_loadCEntropy` (c_src/src/compress/zstd_compress.c:5116) | `RETURN_ERROR_IF(FSE_isError(litlengthHeaderSize), dictionary_corrupted, "");` | `ERROR(FSE_isError)` | [x] |
| 491 | `ZSTD_loadCEntropy` (c_src/src/compress/zstd_compress.c:5117) | `RETURN_ERROR_IF(litlengthLog > LLFSELog, dictionary_corrupted, "");` | `ERROR(litlengthLog)` | [x] |
| 492 | `ZSTD_loadCEntropy` (c_src/src/compress/zstd_compress.c:5118) | `RETURN_ERROR_IF(FSE_isError(FSE_buildCTable_wksp(` | `ERROR(FSE_isError)` | [x] |
| 493 | `ZSTD_loadCEntropy` (c_src/src/compress/zstd_compress.c:5127) | `RETURN_ERROR_IF(dictPtr+12 > dictEnd, dictionary_corrupted, "");` | `ERROR(dictPtr)` | [x] |
| 494 | `ZSTD_loadCEntropy` (c_src/src/compress/zstd_compress.c:5145) | `RETURN_ERROR_IF(bs->rep[u] == 0, dictionary_corrupted, "");` | `ERROR(bs)` | [x] |
| 495 | `ZSTD_loadCEntropy` (c_src/src/compress/zstd_compress.c:5146) | `RETURN_ERROR_IF(bs->rep[u] > dictContentSize, dictionary_corrupted, "");` | `ERROR(bs)` | [x] |
| 496 | `ZSTD_loadZstdDictionary` (c_src/src/compress/zstd_compress.c:5174) | `ZSTD_STATIC_ASSERT(HUF_WORKSPACE_SIZE >= (1<<MAX(MLFSELog,LLFSELog)));` | process assertion failure | [x] |
| 497 | `ZSTD_loadZstdDictionary` (c_src/src/compress/zstd_compress.c:5175) | `assert(dictSize >= 8);` | process assertion failure | [x] |
| 498 | `ZSTD_loadZstdDictionary` (c_src/src/compress/zstd_compress.c:5176) | `assert(MEM_readLE32(dictPtr) == ZSTD_MAGIC_DICTIONARY);` | process assertion failure | [x] |
| 499 | `ZSTD_compress_insertDictionary` (c_src/src/compress/zstd_compress.c:5207) | `RETURN_ERROR_IF(dictContentType == ZSTD_dct_fullDict, dictionary_wrong, "");` | `ERROR(dictContentType)` | [x] |
| 500 | `ZSTD_compress_insertDictionary` (c_src/src/compress/zstd_compress.c:5223) | `RETURN_ERROR_IF(dictContentType == ZSTD_dct_fullDict, dictionary_wrong, "");` | `ERROR(dictContentType)` | [x] |
| 501 | `ZSTD_compress_insertDictionary` (c_src/src/compress/zstd_compress.c:5224) | `assert(0); /* impossible */` | process assertion failure | [x] |
| 502 | `ZSTD_compressBegin_internal` (c_src/src/compress/zstd_compress.c:5252) | `assert(!ZSTD_isError(ZSTD_checkCParams(params->cParams)));` | process assertion failure | [x] |
| 503 | `ZSTD_compressBegin_internal` (c_src/src/compress/zstd_compress.c:5253) | `assert(!((dict) && (cdict))); /* either dict or cdict, not both */` | process assertion failure | [x] |
| 504 | `ZSTD_compressBegin_internal` (c_src/src/compress/zstd_compress.c:5278) | `assert(dictID <= UINT_MAX);` | process assertion failure | [x] |
| 505 | `ZSTD_compressBegin_usingDict_deprecated` (c_src/src/compress/zstd_compress.c:5325) | `return ZSTD_compressBegin_internal(cctx, dict, dictSize, ZSTD_dct_auto, ZSTD_dtlm_fast, NULL,` | source-declared rejection sentinel | [x] |
| 506 | `ZSTD_compressBegin` (c_src/src/compress/zstd_compress.c:5337) | `return ZSTD_compressBegin_usingDict_deprecated(cctx, NULL, 0, compressionLevel);` | source-declared rejection sentinel | [x] |
| 507 | `ZSTD_writeEpilogue` (c_src/src/compress/zstd_compress.c:5350) | `RETURN_ERROR_IF(cctx->stage == ZSTDcs_created, stage_wrong, "init missing");` | `ERROR(cctx)` | [x] |
| 508 | `ZSTD_writeEpilogue` (c_src/src/compress/zstd_compress.c:5364) | `ZSTD_STATIC_ASSERT(ZSTD_BLOCKHEADERSIZE == 3);` | process assertion failure | [x] |
| 509 | `ZSTD_writeEpilogue` (c_src/src/compress/zstd_compress.c:5365) | `RETURN_ERROR_IF(dstCapacity<3, dstSize_tooSmall, "no room for epilogue");` | `ERROR(dstCapacity)` | [x] |
| 510 | `ZSTD_writeEpilogue` (c_src/src/compress/zstd_compress.c:5373) | `RETURN_ERROR_IF(dstCapacity<4, dstSize_tooSmall, "no room for checksum");` | `ERROR(dstCapacity)` | [x] |
| 511 | `ZSTD_compressEnd_public` (c_src/src/compress/zstd_compress.c:5418) | `assert(!(cctx->appliedParams.fParams.contentSizeFlag && cctx->pledgedSrcSizePlusOne == 0));` | process assertion failure | [x] |
| 512 | `ZSTD_compressEnd_public` (c_src/src/compress/zstd_compress.c:5420) | `ZSTD_STATIC_ASSERT(ZSTD_CONTENTSIZE_UNKNOWN == (unsigned long long)-1);` | process assertion failure | [x] |
| 513 | `ZSTD_compressEnd_public` (c_src/src/compress/zstd_compress.c:5422) | `RETURN_ERROR_IF(` | source-declared rejection sentinel | [x] |
| 514 | `ZSTD_compress_usingDict` (c_src/src/compress/zstd_compress.c:5480) | `assert(params.fParams.contentSizeFlag == 1);` | process assertion failure | [x] |
| 515 | `ZSTD_compressCCtx` (c_src/src/compress/zstd_compress.c:5493) | `assert(cctx != NULL);` | process assertion failure | [x] |
| 516 | `ZSTD_compressCCtx` (c_src/src/compress/zstd_compress.c:5494) | `return ZSTD_compress_usingDict(cctx, dst, dstCapacity, src, srcSize, NULL, 0, compressionLevel);` | source-declared rejection sentinel | [x] |
| 517 | `ZSTD_compress` (c_src/src/compress/zstd_compress.c:5504) | `RETURN_ERROR_IF(!cctx, memory_allocation, "ZSTD_createCCtx failed");` | source-declared rejection sentinel | [x] |
| 518 | `ZSTD_initCDict_internal` (c_src/src/compress/zstd_compress.c:5559) | `assert(!ZSTD_checkCParams(params.cParams));` | process assertion failure | [x] |
| 519 | `ZSTD_initCDict_internal` (c_src/src/compress/zstd_compress.c:5566) | `RETURN_ERROR_IF(!internalBuffer, memory_allocation, "NULL pointer!");` | source-declared rejection sentinel | [x] |
| 520 | `ZSTD_initCDict_internal` (c_src/src/compress/zstd_compress.c:5596) | `assert(dictID <= (size_t)(U32)-1);` | process assertion failure | [x] |
| 521 | `ZSTD_createCDict_advanced_internal` (c_src/src/compress/zstd_compress.c:5612) | `if ((!customMem.customAlloc) ^ (!customMem.customFree)) return NULL;` | `NULL` | [x] |
| 522 | `ZSTD_createCDict_advanced_internal` (c_src/src/compress/zstd_compress.c:5627) | `return NULL;` | `NULL` | [x] |
| 523 | `ZSTD_createCDict_advanced_internal` (c_src/src/compress/zstd_compress.c:5633) | `assert(cdict != NULL);` | process assertion failure | [x] |
| 524 | `ZSTD_createCDict_advanced2` (c_src/src/compress/zstd_compress.c:5672) | `if (!customMem.customAlloc ^ !customMem.customFree) return NULL;` | `NULL` | [x] |
| 525 | `ZSTD_createCDict_advanced2` (c_src/src/compress/zstd_compress.c:5704) | `return NULL;` | `NULL` | [x] |
| 526 | `ZSTD_freeCDict` (c_src/src/compress/zstd_compress.c:5754) | `* @return : pointer to ZSTD_CDict*, or NULL if error (size too small)` | source-declared rejection sentinel | [x] |
| 527 | `ZSTD_initStaticCDict` (c_src/src/compress/zstd_compress.c:5777) | `if ((size_t)workspace & 7) return NULL; /* 8-aligned */` | `NULL` | [x] |
| 528 | `ZSTD_initStaticCDict` (c_src/src/compress/zstd_compress.c:5783) | `if (cdict == NULL) return NULL;` | `NULL` | [x] |
| 529 | `ZSTD_initStaticCDict` (c_src/src/compress/zstd_compress.c:5787) | `if (workspaceSize < neededSize) return NULL;` | `NULL` | [x] |
| 530 | `ZSTD_initStaticCDict` (c_src/src/compress/zstd_compress.c:5799) | `return NULL;` | `NULL` | [x] |
| 531 | `ZSTD_getCParamsFromCDict` (c_src/src/compress/zstd_compress.c:5806) | `assert(cdict != NULL);` | process assertion failure | [x] |
| 532 | `ZSTD_compressBegin_usingCDict_internal` (c_src/src/compress/zstd_compress.c:5829) | `RETURN_ERROR_IF(cdict==NULL, dictionary_wrong, "NULL pointer!");` | `ERROR(cdict)` | [x] |
| 533 | `ZSTD_initCStream_internal` (c_src/src/compress/zstd_compress.c:5994) | `assert(!ZSTD_isError(ZSTD_checkCParams(params->cParams)));` | process assertion failure | [x] |
| 534 | `ZSTD_initCStream_internal` (c_src/src/compress/zstd_compress.c:5996) | `assert(!((dict) && (cdict))); /* either dict or cdict, not both */` | process assertion failure | [x] |
| 535 | `ZSTD_nextInputSizeHint` (c_src/src/compress/zstd_compress.c:6093) | `assert(cctx->appliedParams.inBufferMode == ZSTD_bm_buffered);` | process assertion failure | [x] |
| 536 | `ZSTD_compressStream_generic` (c_src/src/compress/zstd_compress.c:6108) | `const char* const istart = (assert(input != NULL), (const char*)input->src);` | process assertion failure | [x] |
| 537 | `ZSTD_compressStream_generic` (c_src/src/compress/zstd_compress.c:6111) | `char* const ostart = (assert(output != NULL), (char*)output->dst);` | process assertion failure | [x] |
| 538 | `ZSTD_compressStream_generic` (c_src/src/compress/zstd_compress.c:6118) | `assert(zcs != NULL);` | process assertion failure | [x] |
| 539 | `ZSTD_compressStream_generic` (c_src/src/compress/zstd_compress.c:6120) | `assert(input->pos >= zcs->stableIn_notConsumed);` | process assertion failure | [x] |
| 540 | `ZSTD_compressStream_generic` (c_src/src/compress/zstd_compress.c:6126) | `assert(zcs->inBuff != NULL);` | process assertion failure | [x] |
| 541 | `ZSTD_compressStream_generic` (c_src/src/compress/zstd_compress.c:6127) | `assert(zcs->inBuffSize > 0);` | process assertion failure | [x] |
| 542 | `ZSTD_compressStream_generic` (c_src/src/compress/zstd_compress.c:6130) | `assert(zcs->outBuff != NULL);` | process assertion failure | [x] |
| 543 | `ZSTD_compressStream_generic` (c_src/src/compress/zstd_compress.c:6131) | `assert(zcs->outBuffSize > 0);` | process assertion failure | [x] |
| 544 | `ZSTD_compressStream_generic` (c_src/src/compress/zstd_compress.c:6133) | `if (input->src == NULL) assert(input->size == 0);` | process assertion failure | [x] |
| 545 | `ZSTD_compressStream_generic` (c_src/src/compress/zstd_compress.c:6134) | `assert(input->pos <= input->size);` | process assertion failure | [x] |
| 546 | `ZSTD_compressStream_generic` (c_src/src/compress/zstd_compress.c:6135) | `if (output->dst == NULL) assert(output->size == 0);` | process assertion failure | [x] |
| 547 | `ZSTD_compressStream_generic` (c_src/src/compress/zstd_compress.c:6136) | `assert(output->pos <= output->size);` | process assertion failure | [x] |
| 548 | `ZSTD_compressStream_generic` (c_src/src/compress/zstd_compress.c:6137) | `assert((U32)flushMode <= (U32)ZSTD_e_end);` | process assertion failure | [x] |
| 549 | `ZSTD_compressStream_generic` (c_src/src/compress/zstd_compress.c:6143) | `RETURN_ERROR(init_missing, "call ZSTD_initCStream() first!");` | `ERROR(init_missing)` | [x] |
| 550 | `ZSTD_compressStream_generic` (c_src/src/compress/zstd_compress.c:6181) | `assert(zcs->appliedParams.inBufferMode == ZSTD_bm_stable);` | process assertion failure | [x] |
| 551 | `ZSTD_compressStream_generic` (c_src/src/compress/zstd_compress.c:6223) | `assert(zcs->inBuffTarget <= zcs->inBuffSize);` | process assertion failure | [x] |
| 552 | `ZSTD_compressStream_generic` (c_src/src/compress/zstd_compress.c:6234) | `if (lastBlock) assert(ip == iend);` | process assertion failure | [x] |
| 553 | `ZSTD_compressStream_generic` (c_src/src/compress/zstd_compress.c:6252) | `assert(zcs->appliedParams.outBufferMode == ZSTD_bm_buffered);` | process assertion failure | [x] |
| 554 | `ZSTD_compressStream_generic` (c_src/src/compress/zstd_compress.c:6263) | `assert(op==oend);` | process assertion failure | [x] |
| 555 | `ZSTD_compressStream_generic` (c_src/src/compress/zstd_compress.c:6279) | `assert(0);` | process assertion failure | [x] |
| 556 | `ZSTD_nextInputSizeHint_MTorST` (c_src/src/compress/zstd_compress.c:6293) | `assert(cctx->mtctx != NULL);` | process assertion failure | [x] |
| 557 | `ZSTD_checkBufferStability` (c_src/src/compress/zstd_compress.c:6333) | `RETURN_ERROR(stabilityCondition_notRespected, "ZSTD_c_stableInBuffer enabled but input differs!");` | `ERROR(stabilityCondition_notRespected)` | [x] |
| 558 | `ZSTD_checkBufferStability` (c_src/src/compress/zstd_compress.c:6339) | `RETURN_ERROR(stabilityCondition_notRespected, "ZSTD_c_stableOutBuffer enabled but output size differs!");` | `ERROR(stabilityCondition_notRespected)` | [x] |
| 559 | `ZSTD_CCtx_init_compressStream2` (c_src/src/compress/zstd_compress.c:6357) | `assert(prefixDict.dict==NULL \|\| cctx->cdict==NULL); /* only one can be set */` | process assertion failure | [x] |
| 560 | `ZSTD_CCtx_init_compressStream2` (c_src/src/compress/zstd_compress.c:6386) | `RETURN_ERROR_IF(` | source-declared rejection sentinel | [x] |
| 561 | `ZSTD_CCtx_init_compressStream2` (c_src/src/compress/zstd_compress.c:6404) | `RETURN_ERROR_IF(cctx->mtctx == NULL, memory_allocation, "NULL pointer!");` | `ERROR(cctx)` | [x] |
| 562 | `ZSTD_CCtx_init_compressStream2` (c_src/src/compress/zstd_compress.c:6421) | `assert(!ZSTD_isError(ZSTD_checkCParams(params.cParams)));` | process assertion failure | [x] |
| 563 | `ZSTD_CCtx_init_compressStream2` (c_src/src/compress/zstd_compress.c:6427) | `assert(cctx->appliedParams.nbWorkers == 0);` | process assertion failure | [x] |
| 564 | `ZSTD_compressStream2` (c_src/src/compress/zstd_compress.c:6454) | `RETURN_ERROR_IF(output->pos > output->size, dstSize_tooSmall, "invalid output buffer");` | `ERROR(output)` | [x] |
| 565 | `ZSTD_compressStream2` (c_src/src/compress/zstd_compress.c:6455) | `RETURN_ERROR_IF(input->pos > input->size, srcSize_wrong, "invalid input buffer");` | `ERROR(input)` | [x] |
| 566 | `ZSTD_compressStream2` (c_src/src/compress/zstd_compress.c:6456) | `RETURN_ERROR_IF((U32)endOp > (U32)ZSTD_e_end, parameter_outOfBound, "invalid endDirective");` | source-declared rejection sentinel | [x] |
| 567 | `ZSTD_compressStream2` (c_src/src/compress/zstd_compress.c:6457) | `assert(cctx != NULL);` | process assertion failure | [x] |
| 568 | `ZSTD_compressStream2` (c_src/src/compress/zstd_compress.c:6468) | `RETURN_ERROR_IF(input->src != cctx->expectedInBuffer.src, stabilityCondition_notRespected, "stableInBuffer condition not respected: wrong src pointer");` | `ERROR(input)` | [x] |
| 569 | `ZSTD_compressStream2` (c_src/src/compress/zstd_compress.c:6469) | `RETURN_ERROR_IF(input->pos != cctx->expectedInBuffer.size, stabilityCondition_notRespected, "stableInBuffer condition not respected: externally modified pos");` | `ERROR(input)` | [x] |
| 570 | `ZSTD_compressStream2` (c_src/src/compress/zstd_compress.c:6495) | `assert(cctx->appliedParams.inBufferMode == ZSTD_bm_stable);` | process assertion failure | [x] |
| 571 | `ZSTD_compressStream2` (c_src/src/compress/zstd_compress.c:6497) | `assert(input->pos >= cctx->stableIn_notConsumed);` | process assertion failure | [x] |
| 572 | `ZSTD_compressStream2` (c_src/src/compress/zstd_compress.c:6523) | `assert(endOp == ZSTD_e_flush \|\| endOp == ZSTD_e_end);` | process assertion failure | [x] |
| 573 | `ZSTD_compressStream2` (c_src/src/compress/zstd_compress.c:6535) | `assert(endOp == ZSTD_e_continue \|\| flushMin == 0 \|\| output->pos == output->size);` | process assertion failure | [x] |
| 574 | `ZSTD_compress2` (c_src/src/compress/zstd_compress.c:6591) | `assert(oPos == dstCapacity);` | process assertion failure | [x] |
| 575 | `ZSTD_compress2` (c_src/src/compress/zstd_compress.c:6592) | `RETURN_ERROR(dstSize_tooSmall, "");` | `ERROR(dstSize_tooSmall)` | [x] |
| 576 | `ZSTD_compress2` (c_src/src/compress/zstd_compress.c:6594) | `assert(iPos == srcSize); /* all input is expected consumed */` | process assertion failure | [x] |
| 577 | `ZSTD_validateSequence` (c_src/src/compress/zstd_compress.c:6615) | `RETURN_ERROR_IF(offBase > OFFSET_TO_OFFBASE(offsetBound), externalSequences_invalid, "Offset too large!");` | `ERROR(offBase)` | [x] |
| 578 | `ZSTD_validateSequence` (c_src/src/compress/zstd_compress.c:6617) | `RETURN_ERROR_IF(matchLength < matchLenLowerBound, externalSequences_invalid, "Matchlength too small for the minMatch");` | `ERROR(matchLength)` | [x] |
| 579 | `ZSTD_transferSequences_wBlockDelim` (c_src/src/compress/zstd_compress.c:6690) | `RETURN_ERROR_IF(idx - seqPos->idx >= cctx->seqStore.maxNbSeq, externalSequences_invalid,` | `ERROR(idx)` | [x] |
| 580 | `ZSTD_transferSequences_wBlockDelim` (c_src/src/compress/zstd_compress.c:6695) | `RETURN_ERROR_IF(idx == inSeqsSize, externalSequences_invalid, "Block delimiter not found.");` | `ERROR(idx)` | [x] |
| 581 | `ZSTD_transferSequences_wBlockDelim` (c_src/src/compress/zstd_compress.c:6698) | `assert(externalRepSearch != ZSTD_ps_auto);` | process assertion failure | [x] |
| 582 | `ZSTD_transferSequences_wBlockDelim` (c_src/src/compress/zstd_compress.c:6699) | `assert(idx >= startIdx);` | process assertion failure | [x] |
| 583 | `ZSTD_transferSequences_wBlockDelim` (c_src/src/compress/zstd_compress.c:6713) | `assert(lastSeqIdx == startIdx);` | process assertion failure | [x] |
| 584 | `ZSTD_transferSequences_wBlockDelim` (c_src/src/compress/zstd_compress.c:6728) | `RETURN_ERROR_IF(ip != iend, externalSequences_invalid, "Blocksize doesn't agree with block delimiter!");` | `ERROR(ip)` | [x] |
| 585 | `ZSTD_transferSequences_noDelim` (c_src/src/compress/zstd_compress.c:6844) | `RETURN_ERROR_IF(idx - seqPos->idx >= cctx->seqStore.maxNbSeq, externalSequences_invalid,` | `ERROR(idx)` | [x] |
| 586 | `ZSTD_transferSequences_noDelim` (c_src/src/compress/zstd_compress.c:6852) | `assert(idx == inSeqsSize \|\| endPosInSequence <= inSeqs[idx].litLength + inSeqs[idx].matchLength);` | process assertion failure | [x] |
| 587 | `ZSTD_transferSequences_noDelim` (c_src/src/compress/zstd_compress.c:6861) | `assert(ip <= iend);` | process assertion failure | [x] |
| 588 | `ZSTD_selectSequenceCopier` (c_src/src/compress/zstd_compress.c:6883) | `assert(ZSTD_cParam_withinBounds(ZSTD_c_blockDelimiters, (int)mode));` | process assertion failure | [x] |
| 589 | `ZSTD_selectSequenceCopier` (c_src/src/compress/zstd_compress.c:6887) | `assert(mode == ZSTD_sf_noBlockDelimiters);` | process assertion failure | [x] |
| 590 | `blockSize_explicitDelimiter` (c_src/src/compress/zstd_compress.c:6902) | `assert(spos <= inSeqsSize);` | process assertion failure | [x] |
| 591 | `blockSize_explicitDelimiter` (c_src/src/compress/zstd_compress.c:6908) | `RETURN_ERROR(externalSequences_invalid, "delimiter format error : both matchlength and offset must be == 0");` | `ERROR(externalSequences_invalid)` | [x] |
| 592 | `blockSize_explicitDelimiter` (c_src/src/compress/zstd_compress.c:6914) | `RETURN_ERROR(externalSequences_invalid, "Reached end of sequences without finding a block delimiter");` | `ERROR(externalSequences_invalid)` | [x] |
| 593 | `determine_blockSize` (c_src/src/compress/zstd_compress.c:6928) | `assert(mode == ZSTD_sf_explicitBlockDelimiters);` | process assertion failure | [x] |
| 594 | `determine_blockSize` (c_src/src/compress/zstd_compress.c:6932) | `RETURN_ERROR(externalSequences_invalid, "sequences incorrectly define a too large block");` | `ERROR(externalSequences_invalid)` | [x] |
| 595 | `determine_blockSize` (c_src/src/compress/zstd_compress.c:6934) | `RETURN_ERROR(externalSequences_invalid, "sequences define a frame longer than source");` | `ERROR(externalSequences_invalid)` | [x] |
| 596 | `ZSTD_compressSequences_internal` (c_src/src/compress/zstd_compress.c:6962) | `RETURN_ERROR_IF(dstCapacity<4, dstSize_tooSmall, "No room for empty frame block header");` | `ERROR(dstCapacity)` | [x] |
| 597 | `ZSTD_compressSequences_internal` (c_src/src/compress/zstd_compress.c:6977) | `assert(blockSize <= remaining);` | process assertion failure | [x] |
| 598 | `ZSTD_compressSequences_internal` (c_src/src/compress/zstd_compress.c:7001) | `RETURN_ERROR_IF(dstCapacity < ZSTD_blockHeaderSize, dstSize_tooSmall, "not enough dstCapacity to write a new compressed block");` | `ERROR(dstCapacity)` | [x] |
| 599 | `ZSTD_compressSequences` (c_src/src/compress/zstd_compress.c:7073) | `assert(cctx != NULL);` | process assertion failure | [x] |
| 600 | `ZSTD_compressSequences` (c_src/src/compress/zstd_compress.c:7080) | `assert(frameHeaderSize <= dstCapacity);` | process assertion failure | [x] |
| 601 | `ZSTD_compressSequences` (c_src/src/compress/zstd_compress.c:7095) | `assert(cBlocksSize <= dstCapacity);` | process assertion failure | [x] |
| 602 | `ZSTD_compressSequences` (c_src/src/compress/zstd_compress.c:7102) | `RETURN_ERROR_IF(dstCapacity<4, dstSize_tooSmall, "no room for checksum");` | `ERROR(dstCapacity)` | [x] |
| 603 | `convertSequences_noRepcodes` (c_src/src/compress/zstd_compress.c:7187) | `ZSTD_STATIC_ASSERT(sizeof(ZSTD_Sequence) == 16);` | process assertion failure | [x] |
| 604 | `convertSequences_noRepcodes` (c_src/src/compress/zstd_compress.c:7188) | `ZSTD_STATIC_ASSERT(offsetof(ZSTD_Sequence, offset) == 0);` | process assertion failure | [x] |
| 605 | `convertSequences_noRepcodes` (c_src/src/compress/zstd_compress.c:7189) | `ZSTD_STATIC_ASSERT(offsetof(ZSTD_Sequence, litLength) == 4);` | process assertion failure | [x] |
| 606 | `convertSequences_noRepcodes` (c_src/src/compress/zstd_compress.c:7190) | `ZSTD_STATIC_ASSERT(offsetof(ZSTD_Sequence, matchLength) == 8);` | process assertion failure | [x] |
| 607 | `convertSequences_noRepcodes` (c_src/src/compress/zstd_compress.c:7191) | `ZSTD_STATIC_ASSERT(sizeof(SeqDef) == 8);` | process assertion failure | [x] |
| 608 | `convertSequences_noRepcodes` (c_src/src/compress/zstd_compress.c:7192) | `ZSTD_STATIC_ASSERT(offsetof(SeqDef, offBase) == 0);` | process assertion failure | [x] |
| 609 | `convertSequences_noRepcodes` (c_src/src/compress/zstd_compress.c:7193) | `ZSTD_STATIC_ASSERT(offsetof(SeqDef, litLength) == 4);` | process assertion failure | [x] |
| 610 | `convertSequences_noRepcodes` (c_src/src/compress/zstd_compress.c:7194) | `ZSTD_STATIC_ASSERT(offsetof(SeqDef, mlBase) == 6);` | process assertion failure | [x] |
| 611 | `convertSequences_noRepcodes` (c_src/src/compress/zstd_compress.c:7240) | `assert(longLen == 0);` | process assertion failure | [x] |
| 612 | `convertSequences_noRepcodes` (c_src/src/compress/zstd_compress.c:7244) | `assert(longLen == 0);` | process assertion failure | [x] |
| 613 | `convertSequences_noRepcodes` (c_src/src/compress/zstd_compress.c:7248) | `assert(longLen == 0);` | process assertion failure | [x] |
| 614 | `convertSequences_noRepcodes` (c_src/src/compress/zstd_compress.c:7252) | `assert(longLen == 0);` | process assertion failure | [x] |
| 615 | `convertSequences_noRepcodes` (c_src/src/compress/zstd_compress.c:7261) | `assert(i == nbSequences - 1);` | process assertion failure | [x] |
| 616 | `convertSequences_noRepcodes` (c_src/src/compress/zstd_compress.c:7267) | `assert(longLen == 0);` | process assertion failure | [x] |
| 617 | `convertSequences_noRepcodes` (c_src/src/compress/zstd_compress.c:7271) | `assert(longLen == 0);` | process assertion failure | [x] |
| 618 | `convertSequences_noRepcodes` (c_src/src/compress/zstd_compress.c:7298) | `assert(longLen == 0);` | process assertion failure | [x] |
| 619 | `convertSequences_noRepcodes` (c_src/src/compress/zstd_compress.c:7302) | `assert(longLen == 0);` | process assertion failure | [x] |
| 620 | `ZSTD_convertBlockSequences` (c_src/src/compress/zstd_compress.c:7327) | `RETURN_ERROR_IF(nbSequences >= cctx->seqStore.maxNbSeq, externalSequences_invalid,` | `ERROR(nbSequences)` | [x] |
| 621 | `ZSTD_convertBlockSequences` (c_src/src/compress/zstd_compress.c:7333) | `assert(nbSequences >= 1);` | process assertion failure | [x] |
| 622 | `ZSTD_convertBlockSequences` (c_src/src/compress/zstd_compress.c:7334) | `assert(inSeqs[nbSequences-1].matchLength == 0);` | process assertion failure | [x] |
| 623 | `ZSTD_convertBlockSequences` (c_src/src/compress/zstd_compress.c:7335) | `assert(inSeqs[nbSequences-1].offset == 0);` | process assertion failure | [x] |
| 624 | `ZSTD_convertBlockSequences` (c_src/src/compress/zstd_compress.c:7343) | `assert(cctx->seqStore.longLengthType == ZSTD_llt_none);` | process assertion failure | [x] |
| 625 | `ZSTD_convertBlockSequences` (c_src/src/compress/zstd_compress.c:7350) | `assert(longl <= 2* (nbSequences-1));` | process assertion failure | [x] |
| 626 | `ZSTD_convertBlockSequences` (c_src/src/compress/zstd_compress.c:7382) | `assert(nbSequences == 2);` | process assertion failure | [x] |
| 627 | `ZSTD_get1BlockSummary` (c_src/src/compress/zstd_compress.c:7403) | `ZSTD_STATIC_ASSERT(sizeof(ZSTD_Sequence) == 16);` | process assertion failure | [x] |
| 628 | `ZSTD_get1BlockSummary` (c_src/src/compress/zstd_compress.c:7414) | `ZSTD_STATIC_ASSERT(offsetof(ZSTD_Sequence, matchLength) == 8);` | process assertion failure | [x] |
| 629 | `ZSTD_get1BlockSummary` (c_src/src/compress/zstd_compress.c:7453) | `assert(seqs);` | process assertion failure | [x] |
| 630 | `ZSTD_get1BlockSummary` (c_src/src/compress/zstd_compress.c:7458) | `assert(seqs[n].offset == 0);` | process assertion failure | [x] |
| 631 | `ZSTD_compressSequencesAndLiterals_internal` (c_src/src/compress/zstd_compress.c:7487) | `assert(cctx->appliedParams.searchForExternalRepcodes != ZSTD_ps_auto);` | process assertion failure | [x] |
| 632 | `ZSTD_compressSequencesAndLiterals_internal` (c_src/src/compress/zstd_compress.c:7490) | `RETURN_ERROR_IF(nbSequences == 0, externalSequences_invalid, "Requires at least 1 end-of-block");` | `ERROR(nbSequences)` | [x] |
| 633 | `ZSTD_compressSequencesAndLiterals_internal` (c_src/src/compress/zstd_compress.c:7495) | `RETURN_ERROR_IF(dstCapacity<3, dstSize_tooSmall, "No room for empty frame block header");` | `ERROR(dstCapacity)` | [x] |
| 634 | `ZSTD_compressSequencesAndLiterals_internal` (c_src/src/compress/zstd_compress.c:7507) | `assert(block.nbSequences <= nbSequences);` | process assertion failure | [x] |
| 635 | `ZSTD_compressSequencesAndLiterals_internal` (c_src/src/compress/zstd_compress.c:7508) | `RETURN_ERROR_IF(block.litSize > litSize, externalSequences_invalid, "discrepancy: Sequences require more literals than present in buffer");` | `ERROR(block)` | [x] |
| 636 | `ZSTD_compressSequencesAndLiterals_internal` (c_src/src/compress/zstd_compress.c:7524) | `RETURN_ERROR_IF(dstCapacity < ZSTD_blockHeaderSize, dstSize_tooSmall, "not enough dstCapacity to write a new compressed block");` | `ERROR(dstCapacity)` | [x] |
| 637 | `ZSTD_compressSequencesAndLiterals_internal` (c_src/src/compress/zstd_compress.c:7550) | `RETURN_ERROR(cannotProduce_uncompressedBlock, "ZSTD_compressSequencesAndLiterals cannot generate an uncompressed block");` | `ERROR(cannotProduce_uncompressedBlock)` | [x] |
| 638 | `ZSTD_compressSequencesAndLiterals_internal` (c_src/src/compress/zstd_compress.c:7553) | `assert(compressedSeqsSize > 1); /* no RLE */` | process assertion failure | [x] |
| 639 | `ZSTD_compressSequencesAndLiterals_internal` (c_src/src/compress/zstd_compress.c:7573) | `assert(nbSequences == 0);` | process assertion failure | [x] |
| 640 | `ZSTD_compressSequencesAndLiterals_internal` (c_src/src/compress/zstd_compress.c:7578) | `RETURN_ERROR_IF(litSize != 0, externalSequences_invalid, "literals must be entirely and exactly consumed");` | `ERROR(litSize)` | [x] |
| 641 | `ZSTD_compressSequencesAndLiterals_internal` (c_src/src/compress/zstd_compress.c:7579) | `RETURN_ERROR_IF(remaining != 0, externalSequences_invalid, "Sequences must represent a total of exactly srcSize=%zu", srcSize);` | `ERROR(remaining)` | [x] |
| 642 | `ZSTD_compressSequencesAndLiterals` (c_src/src/compress/zstd_compress.c:7596) | `assert(cctx != NULL);` | process assertion failure | [x] |
| 643 | `ZSTD_compressSequencesAndLiterals` (c_src/src/compress/zstd_compress.c:7598) | `RETURN_ERROR(workSpace_tooSmall, "literals buffer is not large enough: must be at least 8 bytes larger than litSize (risk of read out-of-bound)");` | `ERROR(workSpace_tooSmall)` | [x] |
| 644 | `ZSTD_compressSequencesAndLiterals` (c_src/src/compress/zstd_compress.c:7603) | `RETURN_ERROR(frameParameter_unsupported, "This mode is only compatible with explicit delimiters");` | `ERROR(frameParameter_unsupported)` | [x] |
| 645 | `ZSTD_compressSequencesAndLiterals` (c_src/src/compress/zstd_compress.c:7606) | `RETURN_ERROR(parameter_unsupported, "This mode is not compatible with Sequence validation");` | `ERROR(parameter_unsupported)` | [x] |
| 646 | `ZSTD_compressSequencesAndLiterals` (c_src/src/compress/zstd_compress.c:7609) | `RETURN_ERROR(frameParameter_unsupported, "this mode is not compatible with frame checksum");` | `ERROR(frameParameter_unsupported)` | [x] |
| 647 | `ZSTD_compressSequencesAndLiterals` (c_src/src/compress/zstd_compress.c:7616) | `assert(frameHeaderSize <= dstCapacity);` | process assertion failure | [x] |
| 648 | `ZSTD_compressSequencesAndLiterals` (c_src/src/compress/zstd_compress.c:7628) | `assert(cBlocksSize <= dstCapacity);` | process assertion failure | [x] |
| 649 | `ZSTD_getCParamRowSize` (c_src/src/compress/zstd_compress.c:7745) | `assert(0);` | process assertion failure | [x] |
| 650 | `ZSTD_registerSequenceProducer` (c_src/src/compress/zstd_compress.c:7824) | `assert(zc != NULL);` | process assertion failure | [x] |
| 651 | `ZSTD_CCtxParams_registerSequenceProducer` (c_src/src/compress/zstd_compress.c:7835) | `assert(params != NULL);` | process assertion failure | [x] |
| 652 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:654) | `RETURN_ERROR_IF(srcSize + ZSTD_blockHeaderSize > dstCapacity,` | `ERROR(srcSize)` | [x] |
| 653 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:666) | `RETURN_ERROR_IF(dstCapacity < 4, dstSize_tooSmall, "");` | `ERROR(dstCapacity)` | [x] |
| 654 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:680) | `ZSTD_STATIC_ASSERT(ZSTD_btultra == 8);` | process assertion failure | [x] |
| 655 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:681) | `assert(ZSTD_cParam_withinBounds(ZSTD_c_strategy, (int)strat));` | process assertion failure | [x] |
| 656 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:693) | `assert(0 /* impossible: pre-validated */);` | process assertion failure | [x] |
| 657 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:708) | `assert(iend > ilimit_w);` | process assertion failure | [x] |
| 658 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:721) | `#define REPCODE_TO_OFFBASE(r) (assert((r)>=1), assert((r)<=ZSTD_REP_NUM), (r)) /* accepts IDs 1,2,3 */` | process assertion failure | [x] |
| 659 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:722) | `#define OFFSET_TO_OFFBASE(o) (assert((o)>0), o + ZSTD_REP_NUM)` | process assertion failure | [x] |
| 660 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:725) | `#define OFFBASE_TO_OFFSET(o) (assert(OFFBASE_IS_OFFSET(o)), (o) - ZSTD_REP_NUM)` | process assertion failure | [x] |
| 661 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:726) | `#define OFFBASE_TO_REPCODE(o) (assert(OFFBASE_IS_REPCODE(o)), (o)) /* returns ID 1,2,3 */` | process assertion failure | [x] |
| 662 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:740) | `assert((size_t)(seqStorePtr->sequences - seqStorePtr->sequencesStart) < seqStorePtr->maxNbSeq);` | process assertion failure | [x] |
| 663 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:743) | `assert(litLength <= ZSTD_BLOCKSIZE_MAX);` | process assertion failure | [x] |
| 664 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:745) | `assert(seqStorePtr->longLengthType == ZSTD_llt_none); /* there can only be a single long length */` | process assertion failure | [x] |
| 665 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:755) | `assert(matchLength <= ZSTD_BLOCKSIZE_MAX);` | process assertion failure | [x] |
| 666 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:756) | `assert(matchLength >= MINMATCH);` | process assertion failure | [x] |
| 667 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:759) | `assert(seqStorePtr->longLengthType == ZSTD_llt_none); /* there can only be a single long length */` | process assertion failure | [x] |
| 668 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:791) | `assert((size_t)(seqStorePtr->sequences - seqStorePtr->sequencesStart) < seqStorePtr->maxNbSeq);` | process assertion failure | [x] |
| 669 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:793) | `assert(seqStorePtr->maxNbLit <= 128 KB);` | process assertion failure | [x] |
| 670 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:794) | `assert(seqStorePtr->lit + litLength <= seqStorePtr->litStart + seqStorePtr->maxNbLit);` | process assertion failure | [x] |
| 671 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:795) | `assert(literals + litLength <= litLimit);` | process assertion failure | [x] |
| 672 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:800) | `ZSTD_STATIC_ASSERT(WILDCOPY_OVERLENGTH >= 16);` | process assertion failure | [x] |
| 673 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:899) | `static U32 ZSTD_hash3(U32 u, U32 h, U32 s) { assert(h <= 32); return (((u << (32-24)) * prime3bytes) ^ s) >> (32-h) ; }` | process assertion failure | [x] |
| 674 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:904) | `static U32 ZSTD_hash4(U32 u, U32 h, U32 s) { assert(h <= 32); return ((u * prime4bytes) ^ s) >> (32-h) ; }` | process assertion failure | [x] |
| 675 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:909) | `static size_t ZSTD_hash5(U64 u, U32 h, U64 s) { assert(h <= 64); return (size_t)((((u << (64-40)) * prime5bytes) ^ s) >> (64-h)) ; }` | process assertion failure | [x] |
| 676 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:914) | `static size_t ZSTD_hash6(U64 u, U32 h, U64 s) { assert(h <= 64); return (size_t)((((u << (64-48)) * prime6bytes) ^ s) >> (64-h)) ; }` | process assertion failure | [x] |
| 677 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:919) | `static size_t ZSTD_hash7(U64 u, U32 h, U64 s) { assert(h <= 64); return (size_t)((((u << (64-56)) * prime7bytes) ^ s) >> (64-h)) ; }` | process assertion failure | [x] |
| 678 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:924) | `static size_t ZSTD_hash8(U64 u, U32 h, U64 s) { assert(h <= 64); return (size_t)((((u) * prime8bytes) ^ s) >> (64-h)) ; }` | process assertion failure | [x] |
| 679 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:934) | `assert(hBits <= 32);` | process assertion failure | [x] |
| 680 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:951) | `assert(hBits <= 32);` | process assertion failure | [x] |
| 681 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:1197) | `assert((maxDist & (maxDist - 1)) == 0);` | process assertion failure | [x] |
| 682 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:1198) | `assert((curr & cycleMask) == (newCurrent & cycleMask));` | process assertion failure | [x] |
| 683 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:1199) | `assert(curr > newCurrent);` | process assertion failure | [x] |
| 684 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:1202) | `assert(correction > 1<<28);` | process assertion failure | [x] |
| 685 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:1219) | `assert(newCurrent >= maxDist);` | process assertion failure | [x] |
| 686 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:1220) | `assert(newCurrent - maxDist >= ZSTD_WINDOW_START_INDEX);` | process assertion failure | [x] |
| 687 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:1222) | `assert(window->lowLimit <= newCurrent);` | process assertion failure | [x] |
| 688 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:1223) | `assert(window->dictLimit <= newCurrent);` | process assertion failure | [x] |
| 689 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:1307) | `assert(loadedDictEndPtr != NULL);` | process assertion failure | [x] |
| 690 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:1308) | `assert(dictMatchStatePtr != NULL);` | process assertion failure | [x] |
| 691 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:1313) | `assert(blockEndIdx >= loadedDictEnd);` | process assertion failure | [x] |
| 692 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:1338) | `ZSTD_STATIC_ASSERT(ZSTD_DUBT_UNSORTED_MARK < ZSTD_WINDOW_START_INDEX); /* Start above ZSTD_DUBT_UNSORTED_MARK */` | process assertion failure | [x] |
| 693 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:1363) | `assert(window->base != NULL);` | process assertion failure | [x] |
| 694 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:1364) | `assert(window->dictBase != NULL);` | process assertion failure | [x] |
| 695 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:1371) | `assert(distanceFromBase == (size_t)(U32)distanceFromBase); /* should never overflow */` | process assertion failure | [x] |
| 696 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:1385) | `assert(highInputIdx < UINT_MAX);` | process assertion failure | [x] |
| 697 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:1446) | `assert(hb + fp_accuracy < 31);` | process assertion failure | [x] |
| 698 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:1490) | `assert(index >> (32 - ZSTD_SHORT_CACHE_TAG_BITS) == 0);` | process assertion failure | [x] |
| 699 | `(file scope)` (c_src/src/compress/zstd_compress_internal.h:1614) | `return params->extSeqProdFunc != NULL;` | source-declared rejection sentinel | [x] |
| 700 | `ZSTD_noCompressLiterals` (c_src/src/compress/zstd_compress_literals.c:46) | `RETURN_ERROR_IF(srcSize + flSize > dstCapacity, dstSize_tooSmall, "");` | `ERROR(srcSize)` | [x] |
| 701 | `ZSTD_noCompressLiterals` (c_src/src/compress/zstd_compress_literals.c:60) | `assert(0);` | process assertion failure | [x] |
| 702 | `allBytesIdentical` (c_src/src/compress/zstd_compress_literals.c:70) | `assert(srcSize >= 1);` | process assertion failure | [x] |
| 703 | `allBytesIdentical` (c_src/src/compress/zstd_compress_literals.c:71) | `assert(src != NULL);` | process assertion failure | [x] |
| 704 | `ZSTD_compressRleLiteralsBlock` (c_src/src/compress/zstd_compress_literals.c:86) | `assert(dstCapacity >= 4); (void)dstCapacity;` | process assertion failure | [x] |
| 705 | `ZSTD_compressRleLiteralsBlock` (c_src/src/compress/zstd_compress_literals.c:87) | `assert(allBytesIdentical(src, srcSize));` | process assertion failure | [x] |
| 706 | `ZSTD_compressRleLiteralsBlock` (c_src/src/compress/zstd_compress_literals.c:101) | `assert(0);` | process assertion failure | [x] |
| 707 | `ZSTD_minLiteralsToCompress` (c_src/src/compress/zstd_compress_literals.c:117) | `assert((int)strategy >= 0);` | process assertion failure | [x] |
| 708 | `ZSTD_minLiteralsToCompress` (c_src/src/compress/zstd_compress_literals.c:118) | `assert((int)strategy <= 9);` | process assertion failure | [x] |
| 709 | `ZSTD_compressLiterals` (c_src/src/compress/zstd_compress_literals.c:161) | `RETURN_ERROR_IF(dstCapacity < lhSize+1, dstSize_tooSmall, "not enough space for compression");` | `ERROR(dstCapacity)` | [x] |
| 710 | `ZSTD_compressLiterals` (c_src/src/compress/zstd_compress_literals.c:212) | `if (!singleStream) assert(srcSize >= MIN_LITERALS_FOR_4_STREAMS);` | process assertion failure | [x] |
| 711 | `ZSTD_compressLiterals` (c_src/src/compress/zstd_compress_literals.c:218) | `assert(srcSize >= MIN_LITERALS_FOR_4_STREAMS);` | process assertion failure | [x] |
| 712 | `ZSTD_compressLiterals` (c_src/src/compress/zstd_compress_literals.c:224) | `assert(srcSize >= MIN_LITERALS_FOR_4_STREAMS);` | process assertion failure | [x] |
| 713 | `ZSTD_compressLiterals` (c_src/src/compress/zstd_compress_literals.c:231) | `assert(0);` | process assertion failure | [x] |
| 714 | `ZSTD_entropyCost` (c_src/src/compress/zstd_compress_sequences.c:89) | `assert(total > 0);` | process assertion failure | [x] |
| 715 | `ZSTD_entropyCost` (c_src/src/compress/zstd_compress_sequences.c:94) | `assert(count[s] < total);` | process assertion failure | [x] |
| 716 | `ZSTD_fseBitCost` (c_src/src/compress/zstd_compress_sequences.c:117) | `return ERROR(GENERIC);` | `ERROR(GENERIC)` | [x] |
| 717 | `ZSTD_fseBitCost` (c_src/src/compress/zstd_compress_sequences.c:127) | `return ERROR(GENERIC);` | `ERROR(GENERIC)` | [x] |
| 718 | `ZSTD_crossEntropyCost` (c_src/src/compress/zstd_compress_sequences.c:145) | `assert(accuracyLog <= 8);` | process assertion failure | [x] |
| 719 | `ZSTD_crossEntropyCost` (c_src/src/compress/zstd_compress_sequences.c:149) | `assert(norm256 > 0);` | process assertion failure | [x] |
| 720 | `ZSTD_crossEntropyCost` (c_src/src/compress/zstd_compress_sequences.c:150) | `assert(norm256 < 256);` | process assertion failure | [x] |
| 721 | `ZSTD_selectEncodingType` (c_src/src/compress/zstd_compress_sequences.c:165) | `ZSTD_STATIC_ASSERT(ZSTD_defaultDisallowed == 0 && ZSTD_defaultAllowed != 0);` | process assertion failure | [x] |
| 722 | `ZSTD_selectEncodingType` (c_src/src/compress/zstd_compress_sequences.c:185) | `assert(defaultNormLog >= 5 && defaultNormLog <= 6); /* xx_DEFAULTNORMLOG */` | process assertion failure | [x] |
| 723 | `ZSTD_selectEncodingType` (c_src/src/compress/zstd_compress_sequences.c:186) | `assert(mult <= 9 && mult >= 7);` | process assertion failure | [x] |
| 724 | `ZSTD_selectEncodingType` (c_src/src/compress/zstd_compress_sequences.c:212) | `assert(!ZSTD_isError(basicCost));` | process assertion failure | [x] |
| 725 | `ZSTD_selectEncodingType` (c_src/src/compress/zstd_compress_sequences.c:213) | `assert(!(*repeatMode == FSE_repeat_valid && ZSTD_isError(repeatCost)));` | process assertion failure | [x] |
| 726 | `ZSTD_selectEncodingType` (c_src/src/compress/zstd_compress_sequences.c:215) | `assert(!ZSTD_isError(NCountCost));` | process assertion failure | [x] |
| 727 | `ZSTD_selectEncodingType` (c_src/src/compress/zstd_compress_sequences.c:216) | `assert(compressedCost < ERROR(maxCode));` | process assertion failure | [x] |
| 728 | `ZSTD_selectEncodingType` (c_src/src/compress/zstd_compress_sequences.c:221) | `assert(isDefaultAllowed);` | process assertion failure | [x] |
| 729 | `ZSTD_selectEncodingType` (c_src/src/compress/zstd_compress_sequences.c:227) | `assert(!ZSTD_isError(repeatCost));` | process assertion failure | [x] |
| 730 | `ZSTD_selectEncodingType` (c_src/src/compress/zstd_compress_sequences.c:230) | `assert(compressedCost < basicCost && compressedCost < repeatCost);` | process assertion failure | [x] |
| 731 | `ZSTD_buildCTable` (c_src/src/compress/zstd_compress_sequences.c:258) | `RETURN_ERROR_IF(dstCapacity==0, dstSize_tooSmall, "not enough space");` | `ERROR(dstCapacity)` | [x] |
| 732 | `ZSTD_buildCTable` (c_src/src/compress/zstd_compress_sequences.c:275) | `assert(nbSeq_1 > 1);` | process assertion failure | [x] |
| 733 | `ZSTD_buildCTable` (c_src/src/compress/zstd_compress_sequences.c:276) | `assert(entropyWorkspaceSize >= sizeof(ZSTD_BuildCTableWksp));` | process assertion failure | [x] |
| 734 | `ZSTD_buildCTable` (c_src/src/compress/zstd_compress_sequences.c:279) | `assert(oend >= op);` | process assertion failure | [x] |
| 735 | `ZSTD_buildCTable` (c_src/src/compress/zstd_compress_sequences.c:286) | `default: assert(0); RETURN_ERROR(GENERIC, "impossible to reach");` | process assertion failure | [x] |
| 736 | `ZSTD_encodeSequences_body` (c_src/src/compress/zstd_compress_sequences.c:303) | `RETURN_ERROR_IF(` | source-declared rejection sentinel | [x] |
| 737 | `ZSTD_encodeSequences_body` (c_src/src/compress/zstd_compress_sequences.c:379) | `RETURN_ERROR_IF(streamSize==0, dstSize_tooSmall, "not enough space");` | `ERROR(streamSize)` | [x] |
| 738 | `ZSTD_compressSubBlock_literal` (c_src/src/compress/zstd_compress_superblock.c:68) | `assert(litSize > 0);` | process assertion failure | [x] |
| 739 | `ZSTD_compressSubBlock_literal` (c_src/src/compress/zstd_compress_superblock.c:69) | `assert(hufMetadata->hType == set_compressed \|\| hufMetadata->hType == set_repeat);` | process assertion failure | [x] |
| 740 | `ZSTD_compressSubBlock_literal` (c_src/src/compress/zstd_compress_superblock.c:94) | `assert(cLitSize > litSize);` | process assertion failure | [x] |
| 741 | `ZSTD_compressSubBlock_literal` (c_src/src/compress/zstd_compress_superblock.c:121) | `assert(0);` | process assertion failure | [x] |
| 742 | `ZSTD_seqDecompressedSize` (c_src/src/compress/zstd_compress_superblock.c:145) | `assert(litLengthSum == litSize);` | process assertion failure | [x] |
| 743 | `ZSTD_seqDecompressedSize` (c_src/src/compress/zstd_compress_superblock.c:147) | `assert(litLengthSum <= litSize);` | process assertion failure | [x] |
| 744 | `ZSTD_compressSubBlock_sequences` (c_src/src/compress/zstd_compress_superblock.c:181) | `RETURN_ERROR_IF((oend-op) < 3 /*max nbSeq Size*/ + 1 /*seqHead*/,` | source-declared rejection sentinel | [x] |
| 745 | `ZSTD_compressSubBlock_sequences` (c_src/src/compress/zstd_compress_superblock.c:231) | `assert(fseMetadata->lastCountSize + bitstreamSize == 3);` | process assertion failure | [x] |
| 746 | `ZSTD_estimateSubBlockSize_literal` (c_src/src/compress/zstd_compress_superblock.c:326) | `assert(0); /* impossible */` | process assertion failure | [x] |
| 747 | `ZSTD_estimateSubBlockSize_symbolType` (c_src/src/compress/zstd_compress_superblock.c:347) | `assert(max <= defaultMax);` | process assertion failure | [x] |
| 748 | `countLiterals` (c_src/src/compress/zstd_compress_superblock.c:432) | `assert(sp != NULL);` | process assertion failure | [x] |
| 749 | `sizeBlockSequences` (c_src/src/compress/zstd_compress_superblock.c:449) | `assert(firstSubBlock==0 \|\| firstSubBlock==1);` | process assertion failure | [x] |
| 750 | `ZSTD_compressSubBlock_multi` (c_src/src/compress/zstd_compress_superblock.c:535) | `assert(nbSubBlocks>0);` | process assertion failure | [x] |
| 751 | `ZSTD_compressSubBlock_multi` (c_src/src/compress/zstd_compress_superblock.c:541) | `assert(seqCount <= (size_t)(send-sp));` | process assertion failure | [x] |
| 752 | `ZSTD_compressSubBlock_multi` (c_src/src/compress/zstd_compress_superblock.c:543) | `assert(seqCount > 0);` | process assertion failure | [x] |
| 753 | `ZSTD_compressSubBlock_multi` (c_src/src/compress/zstd_compress_superblock.c:565) | `assert(ip + decompressedSize <= iend);` | process assertion failure | [x] |
| 754 | `ZSTD_compressSubBlock_multi` (c_src/src/compress/zstd_compress_superblock.c:609) | `assert(ip + decompressedSize <= iend);` | process assertion failure | [x] |
| 755 | `ZSTD_compressSubBlock_multi` (c_src/src/compress/zstd_compress_superblock.c:646) | `assert(cSize != 0);` | process assertion failure | [x] |
| 756 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:180) | `assert(ws->workspace <= ws->objectEnd);` | process assertion failure | [x] |
| 757 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:181) | `assert(ws->objectEnd <= ws->tableEnd);` | process assertion failure | [x] |
| 758 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:182) | `assert(ws->objectEnd <= ws->tableValidEnd);` | process assertion failure | [x] |
| 759 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:183) | `assert(ws->tableEnd <= ws->allocStart);` | process assertion failure | [x] |
| 760 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:184) | `assert(ws->tableValidEnd <= ws->allocStart);` | process assertion failure | [x] |
| 761 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:185) | `assert(ws->allocStart <= ws->workspaceEnd);` | process assertion failure | [x] |
| 762 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:186) | `assert(ws->initOnceStart <= ZSTD_cwksp_initialAllocStart(ws));` | process assertion failure | [x] |
| 763 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:187) | `assert(ws->workspace <= ws->initOnceStart);` | process assertion failure | [x] |
| 764 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:198) | `assert(offset==-1);` | process assertion failure | [x] |
| 765 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:208) | `assert(ZSTD_isPower2(align));` | process assertion failure | [x] |
| 766 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:266) | `assert(ZSTD_isPower2(alignBytes));` | process assertion failure | [x] |
| 767 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:267) | `assert(bytes < alignBytes);` | process assertion failure | [x] |
| 768 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:278) | `assert(ZSTD_isPower2(ZSTD_CWKSP_ALIGNMENT_BYTES));` | process assertion failure | [x] |
| 769 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:298) | `assert(alloc >= bottom);` | process assertion failure | [x] |
| 770 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:302) | `return NULL;` | `NULL` | [x] |
| 771 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:321) | `assert(phase >= ws->phase);` | process assertion failure | [x] |
| 772 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:334) | `RETURN_ERROR_IF(objectEnd > ws->workspaceEnd, memory_allocation,` | `ERROR(objectEnd)` | [x] |
| 773 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:354) | `return (ptr != NULL) && (ws->workspace <= ptr) && (ptr < ws->workspaceEnd);` | source-declared rejection sentinel | [x] |
| 774 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:365) | `return NULL;` | `NULL` | [x] |
| 775 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:412) | `assert(((size_t)ptr & (ZSTD_CWKSP_ALIGNMENT_BYTES-1)) == 0);` | process assertion failure | [x] |
| 776 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:424) | `assert(__msan_test_shadow(ptr, bytes) == -1);` | process assertion failure | [x] |
| 777 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:437) | `assert(((size_t)ptr & (ZSTD_CWKSP_ALIGNMENT_BYTES-1)) == 0);` | process assertion failure | [x] |
| 778 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:457) | `return NULL;` | `NULL` | [x] |
| 779 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:466) | `assert((bytes & (sizeof(U32)-1)) == 0);` | process assertion failure | [x] |
| 780 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:468) | `assert(end <= top);` | process assertion failure | [x] |
| 781 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:472) | `return NULL;` | `NULL` | [x] |
| 782 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:482) | `assert((bytes & (ZSTD_CWKSP_ALIGNMENT_BYTES-1)) == 0);` | process assertion failure | [x] |
| 783 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:483) | `assert(((size_t)alloc & (ZSTD_CWKSP_ALIGNMENT_BYTES-1)) == 0);` | process assertion failure | [x] |
| 784 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:505) | `assert((size_t)alloc % ZSTD_ALIGNOF(void*) == 0);` | process assertion failure | [x] |
| 785 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:506) | `assert(bytes % ZSTD_ALIGNOF(void*) == 0);` | process assertion failure | [x] |
| 786 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:512) | `return NULL;` | `NULL` | [x] |
| 787 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:538) | `if (start == NULL) return NULL;` | `NULL` | [x] |
| 788 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:540) | `assert(ZSTD_isPower2(alignment));` | process assertion failure | [x] |
| 789 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:557) | `assert(__msan_test_shadow(ws->objectEnd, size) == -1);` | process assertion failure | [x] |
| 790 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:561) | `assert(ws->initOnceStart >= ws->objectEnd);` | process assertion failure | [x] |
| 791 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:567) | `assert(ws->tableValidEnd >= ws->objectEnd);` | process assertion failure | [x] |
| 792 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:568) | `assert(ws->tableValidEnd <= ws->allocStart);` | process assertion failure | [x] |
| 793 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:575) | `assert(ws->tableValidEnd >= ws->objectEnd);` | process assertion failure | [x] |
| 794 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:576) | `assert(ws->tableValidEnd <= ws->allocStart);` | process assertion failure | [x] |
| 795 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:588) | `assert(ws->tableValidEnd >= ws->objectEnd);` | process assertion failure | [x] |
| 796 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:589) | `assert(ws->tableValidEnd <= ws->allocStart);` | process assertion failure | [x] |
| 797 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:676) | `assert(((size_t)start & (sizeof(void*)-1)) == 0); /* ensure correct alignment */` | process assertion failure | [x] |
| 798 | `(file scope)` (c_src/src/compress/zstd_cwksp.h:692) | `RETURN_ERROR_IF(workspace == NULL, memory_allocation, "NULL pointer!");` | `ERROR(workspace)` | [x] |
| 799 | `ZSTD_compressBlock_doubleFast_dictMatchState_generic` (c_src/src/compress/zstd_double_fast.c:366) | `assert(ms->window.dictLimit + (1U << cParams->windowLog) >= endIndex);` | process assertion failure | [x] |
| 800 | `ZSTD_compressBlock_doubleFast_dictMatchState_generic` (c_src/src/compress/zstd_double_fast.c:380) | `assert(offset_1 <= dictAndPrefixLength);` | process assertion failure | [x] |
| 801 | `ZSTD_compressBlock_doubleFast_dictMatchState_generic` (c_src/src/compress/zstd_double_fast.c:381) | `assert(offset_2 <= dictAndPrefixLength);` | process assertion failure | [x] |
| 802 | `ZSTD_compressBlock_doubleFast_dictMatchState_generic` (c_src/src/compress/zstd_double_fast.c:426) | `assert(dictMatchL < dictEnd);` | process assertion failure | [x] |
| 803 | `ZSTD_compressBlock_doubleFast_dictMatchState_generic` (c_src/src/compress/zstd_double_fast.c:476) | `assert(dictMatchL3 < dictEnd);` | process assertion failure | [x] |
| 804 | `ZSTD_fillHashTableForCDict` (c_src/src/compress/zstd_fast.c:31) | `assert(dtlm == ZSTD_dtlm_full);` | process assertion failure | [x] |
| 805 | `ZSTD_fillHashTableForCCtx` (c_src/src/compress/zstd_fast.c:68) | `assert(dtlm == ZSTD_dtlm_fast);` | process assertion failure | [x] |
| 806 | `ZSTD_compressBlock_fast_noDict_generic` (c_src/src/compress/zstd_fast.c:406) | `assert(base+current0+2 > istart); /* check base overflow */` | process assertion failure | [x] |
| 807 | `ZSTD_compressBlock_fast` (c_src/src/compress/zstd_fast.c:450) | `assert(ms->dictMatchState == NULL);` | process assertion failure | [x] |
| 808 | `ZSTD_compressBlock_fast_dictMatchState_generic` (c_src/src/compress/zstd_fast.c:518) | `assert(endIndex - prefixStartIndex <= maxDistance);` | process assertion failure | [x] |
| 809 | `ZSTD_compressBlock_fast_dictMatchState_generic` (c_src/src/compress/zstd_fast.c:519) | `(void)maxDistance; (void)endIndex; /* these variables are not used when assert() is disabled */` | process assertion failure | [x] |
| 810 | `ZSTD_compressBlock_fast_dictMatchState_generic` (c_src/src/compress/zstd_fast.c:525) | `assert(prefixStartIndex >= (U32)(dictEnd - dictBase));` | process assertion failure | [x] |
| 811 | `ZSTD_compressBlock_fast_dictMatchState_generic` (c_src/src/compress/zstd_fast.c:537) | `assert(offset_1 <= dictAndPrefixLength);` | process assertion failure | [x] |
| 812 | `ZSTD_compressBlock_fast_dictMatchState_generic` (c_src/src/compress/zstd_fast.c:538) | `assert(offset_2 <= dictAndPrefixLength);` | process assertion failure | [x] |
| 813 | `ZSTD_compressBlock_fast_dictMatchState_generic` (c_src/src/compress/zstd_fast.c:541) | `assert(stepSize >= 1);` | process assertion failure | [x] |
| 814 | `ZSTD_compressBlock_fast_dictMatchState_generic` (c_src/src/compress/zstd_fast.c:634) | `assert(mLength);` | process assertion failure | [x] |
| 815 | `ZSTD_compressBlock_fast_dictMatchState_generic` (c_src/src/compress/zstd_fast.c:640) | `assert(base+curr+2 > istart); /* check base overflow */` | process assertion failure | [x] |
| 816 | `ZSTD_compressBlock_fast_dictMatchState_generic` (c_src/src/compress/zstd_fast.c:667) | `assert(ip0 == anchor);` | process assertion failure | [x] |
| 817 | `ZSTD_compressBlock_fast_dictMatchState` (c_src/src/compress/zstd_fast.c:691) | `assert(ms->dictMatchState != NULL);` | process assertion failure | [x] |
| 818 | `ZSTD_compressBlock_fast_extDict_generic` (c_src/src/compress/zstd_fast.c:813) | `assert((match0 != prefixStart) & (match0 != dictStart));` | process assertion failure | [x] |
| 819 | `ZSTD_compressBlock_fast_extDict_generic` (c_src/src/compress/zstd_fast.c:922) | `assert(matchEnd != 0);` | process assertion failure | [x] |
| 820 | `ZSTD_compressBlock_fast_extDict_generic` (c_src/src/compress/zstd_fast.c:938) | `assert(base+current0+2 > istart); /* check base overflow */` | process assertion failure | [x] |
| 821 | `ZSTD_compressBlock_fast_extDict` (c_src/src/compress/zstd_fast.c:972) | `assert(ms->dictMatchState == NULL);` | process assertion failure | [x] |
| 822 | `ZSTD_updateDUBT` (c_src/src/compress/zstd_lazy.c:48) | `assert(ip + 8 <= iend); /* condition for ZSTD_hashPtr */` | process assertion failure | [x] |
| 823 | `ZSTD_updateDUBT` (c_src/src/compress/zstd_lazy.c:51) | `assert(idx >= ms->window.dictLimit); /* condition for valid base+idx */` | process assertion failure | [x] |
| 824 | `ZSTD_insertDUBT1` (c_src/src/compress/zstd_lazy.c:103) | `assert(curr >= btLow);` | process assertion failure | [x] |
| 825 | `ZSTD_insertDUBT1` (c_src/src/compress/zstd_lazy.c:104) | `assert(ip < iend); /* condition for ZSTD_count */` | process assertion failure | [x] |
| 826 | `ZSTD_insertDUBT1` (c_src/src/compress/zstd_lazy.c:109) | `assert(matchIndex < curr);` | process assertion failure | [x] |
| 827 | `ZSTD_insertDUBT1` (c_src/src/compress/zstd_lazy.c:120) | `assert( (matchIndex+matchLength >= dictLimit) /* might be wrong if extDict is incorrectly set to 0 */` | process assertion failure | [x] |
| 828 | `ZSTD_DUBT_findBetterDictMatch` (c_src/src/compress/zstd_lazy.c:197) | `assert(dictMode == ZSTD_dictMatchState);` | process assertion failure | [x] |
| 829 | `ZSTD_DUBT_findBestMatch` (c_src/src/compress/zstd_lazy.c:272) | `assert(ip <= iend-8); /* required for h calculation */` | process assertion failure | [x] |
| 830 | `ZSTD_DUBT_findBestMatch` (c_src/src/compress/zstd_lazy.c:273) | `assert(dictMode != ZSTD_dedicatedDictSearch);` | process assertion failure | [x] |
| 831 | `ZSTD_DUBT_findBestMatch` (c_src/src/compress/zstd_lazy.c:372) | `assert(nbCompares <= (1U << ZSTD_SEARCHLOG_MAX)); /* Check we haven't underflowed. */` | process assertion failure | [x] |
| 832 | `ZSTD_DUBT_findBestMatch` (c_src/src/compress/zstd_lazy.c:380) | `assert(matchEndIdx > curr+8); /* ensure nextToUpdate is increased */` | process assertion failure | [x] |
| 833 | `ZSTD_dedicatedDictSearch_lazy_loadDictionary` (c_src/src/compress/zstd_lazy.c:437) | `assert(ms->cParams.chainLog <= 24);` | process assertion failure | [x] |
| 834 | `ZSTD_dedicatedDictSearch_lazy_loadDictionary` (c_src/src/compress/zstd_lazy.c:438) | `assert(ms->cParams.hashLog > ms->cParams.chainLog);` | process assertion failure | [x] |
| 835 | `ZSTD_dedicatedDictSearch_lazy_loadDictionary` (c_src/src/compress/zstd_lazy.c:439) | `assert(idx != 0);` | process assertion failure | [x] |
| 836 | `ZSTD_dedicatedDictSearch_lazy_loadDictionary` (c_src/src/compress/zstd_lazy.c:440) | `assert(tmpMinChain <= minChain);` | process assertion failure | [x] |
| 837 | `ZSTD_dedicatedDictSearch_lazy_loadDictionary` (c_src/src/compress/zstd_lazy.c:497) | `assert(chainPos <= chainSize); /* I believe this is guaranteed... */` | process assertion failure | [x] |
| 838 | `ZSTD_dedicatedDictSearch_lazy_search` (c_src/src/compress/zstd_lazy.c:567) | `assert(matchIndex >= ddsLowestIndex);` | process assertion failure | [x] |
| 839 | `ZSTD_dedicatedDictSearch_lazy_search` (c_src/src/compress/zstd_lazy.c:568) | `assert(match+4 <= ddsEnd);` | process assertion failure | [x] |
| 840 | `ZSTD_dedicatedDictSearch_lazy_search` (c_src/src/compress/zstd_lazy.c:604) | `assert(matchIndex >= ddsLowestIndex);` | process assertion failure | [x] |
| 841 | `ZSTD_dedicatedDictSearch_lazy_search` (c_src/src/compress/zstd_lazy.c:605) | `assert(match+4 <= ddsEnd);` | process assertion failure | [x] |
| 842 | `ZSTD_HcFindBestMatch` (c_src/src/compress/zstd_lazy.c:712) | `assert(matchIndex >= dictLimit); /* ensures this is true if dictMode != ZSTD_extDict */` | process assertion failure | [x] |
| 843 | `ZSTD_HcFindBestMatch` (c_src/src/compress/zstd_lazy.c:718) | `assert(match+4 <= dictEnd);` | process assertion failure | [x] |
| 844 | `ZSTD_HcFindBestMatch` (c_src/src/compress/zstd_lazy.c:734) | `assert(nbAttempts <= (1U << ZSTD_SEARCHLOG_MAX)); /* Check we haven't underflowed. */` | process assertion failure | [x] |
| 845 | `ZSTD_HcFindBestMatch` (c_src/src/compress/zstd_lazy.c:754) | `assert(match+4 <= dmsEnd);` | process assertion failure | [x] |
| 846 | `ZSTD_HcFindBestMatch` (c_src/src/compress/zstd_lazy.c:761) | `assert(curr > matchIndex + dmsIndexDelta);` | process assertion failure | [x] |
| 847 | `ZSTD_isAligned` (c_src/src/compress/zstd_lazy.c:809) | `assert((align & (align - 1)) == 0);` | process assertion failure | [x] |
| 848 | `ZSTD_row_prefetch` (c_src/src/compress/zstd_lazy.c:826) | `assert(rowLog == 4 \|\| rowLog == 5 \|\| rowLog == 6);` | process assertion failure | [x] |
| 849 | `ZSTD_row_prefetch` (c_src/src/compress/zstd_lazy.c:827) | `assert(ZSTD_isAligned(hashTable + relRow, 64)); /* prefetched hash row always 64-byte aligned */` | process assertion failure | [x] |
| 850 | `ZSTD_row_prefetch` (c_src/src/compress/zstd_lazy.c:828) | `assert(ZSTD_isAligned(tagTable + relRow, (size_t)1 << rowLog)); /* prefetched tagRow sits on correct multiple of bytes (32,64,128) */` | process assertion failure | [x] |
| 851 | `ZSTD_row_update_internalImpl` (c_src/src/compress/zstd_lazy.c:904) | `assert(hash == ZSTD_hashPtrSalted(base + updateStartIdx, hashLog + ZSTD_ROW_HASH_TAG_BITS, mls, ms->hashSalt));` | process assertion failure | [x] |
| 852 | `ZSTD_row_update_internal` (c_src/src/compress/zstd_lazy.c:940) | `assert(target >= idx);` | process assertion failure | [x] |
| 853 | `ZSTD_row_matchMaskGroupWidth` (c_src/src/compress/zstd_lazy.c:965) | `assert((rowEntries == 16) \|\| (rowEntries == 32) \|\| rowEntries == 64);` | process assertion failure | [x] |
| 854 | `ZSTD_row_matchMaskGroupWidth` (c_src/src/compress/zstd_lazy.c:966) | `assert(rowEntries <= ZSTD_ROW_HASH_MAX_ENTRIES);` | process assertion failure | [x] |
| 855 | `ZSTD_row_getSSEMask` (c_src/src/compress/zstd_lazy.c:993) | `assert(nbChunks == 1 \|\| nbChunks == 2 \|\| nbChunks == 4);` | process assertion failure | [x] |
| 856 | `ZSTD_row_getSSEMask` (c_src/src/compress/zstd_lazy.c:1001) | `assert(nbChunks == 4);` | process assertion failure | [x] |
| 857 | `ZSTD_row_getNEONMask` (c_src/src/compress/zstd_lazy.c:1010) | `assert((rowEntries == 16) \|\| (rowEntries == 32) \|\| rowEntries == 64);` | process assertion failure | [x] |
| 858 | `ZSTD_row_getMatchMask` (c_src/src/compress/zstd_lazy.c:1064) | `assert((rowEntries == 16) \|\| (rowEntries == 32) \|\| rowEntries == 64);` | process assertion failure | [x] |
| 859 | `ZSTD_row_getMatchMask` (c_src/src/compress/zstd_lazy.c:1065) | `assert(rowEntries <= ZSTD_ROW_HASH_MAX_ENTRIES);` | process assertion failure | [x] |
| 860 | `ZSTD_row_getMatchMask` (c_src/src/compress/zstd_lazy.c:1066) | `assert(ZSTD_row_matchMaskGroupWidth(rowEntries) * rowEntries <= sizeof(ZSTD_VecMask) * 8);` | process assertion failure | [x] |
| 861 | `ZSTD_row_getMatchMask` (c_src/src/compress/zstd_lazy.c:1089) | `assert((sizeof(size_t) == 4) \|\| (sizeof(size_t) == 8));` | process assertion failure | [x] |
| 862 | `ZSTD_RowFindBestMatch` (c_src/src/compress/zstd_lazy.c:1233) | `assert(numMatches < rowEntries);` | process assertion failure | [x] |
| 863 | `ZSTD_RowFindBestMatch` (c_src/src/compress/zstd_lazy.c:1257) | `assert(matchIndex < curr);` | process assertion failure | [x] |
| 864 | `ZSTD_RowFindBestMatch` (c_src/src/compress/zstd_lazy.c:1258) | `assert(matchIndex >= lowLimit);` | process assertion failure | [x] |
| 865 | `ZSTD_RowFindBestMatch` (c_src/src/compress/zstd_lazy.c:1262) | `assert(matchIndex >= dictLimit); /* ensures this is true if dictMode != ZSTD_extDict */` | process assertion failure | [x] |
| 866 | `ZSTD_RowFindBestMatch` (c_src/src/compress/zstd_lazy.c:1268) | `assert(match+4 <= dictEnd);` | process assertion failure | [x] |
| 867 | `ZSTD_RowFindBestMatch` (c_src/src/compress/zstd_lazy.c:1282) | `assert(nbAttempts <= (1U << ZSTD_SEARCHLOG_MAX)); /* Check we haven't underflowed. */` | process assertion failure | [x] |
| 868 | `ZSTD_RowFindBestMatch` (c_src/src/compress/zstd_lazy.c:1315) | `assert(matchIndex >= dmsLowestIndex);` | process assertion failure | [x] |
| 869 | `ZSTD_RowFindBestMatch` (c_src/src/compress/zstd_lazy.c:1316) | `assert(matchIndex < curr);` | process assertion failure | [x] |
| 870 | `ZSTD_RowFindBestMatch` (c_src/src/compress/zstd_lazy.c:1319) | `assert(match+4 <= dmsEnd);` | process assertion failure | [x] |
| 871 | `ZSTD_RowFindBestMatch` (c_src/src/compress/zstd_lazy.c:1326) | `assert(curr > matchIndex + dmsIndexDelta);` | process assertion failure | [x] |
| 872 | `ZSTD_RowFindBestMatch` (c_src/src/compress/zstd_lazy.c:1371) | `assert(MAX(4, MIN(6, ms->cParams.minMatch)) == mls); \` | process assertion failure | [x] |
| 873 | `ZSTD_RowFindBestMatch` (c_src/src/compress/zstd_lazy.c:1381) | `assert(MAX(4, MIN(6, ms->cParams.minMatch)) == mls); \` | process assertion failure | [x] |
| 874 | `ZSTD_RowFindBestMatch` (c_src/src/compress/zstd_lazy.c:1391) | `assert(MAX(4, MIN(6, ms->cParams.minMatch)) == mls); \` | process assertion failure | [x] |
| 875 | `ZSTD_RowFindBestMatch` (c_src/src/compress/zstd_lazy.c:1392) | `assert(MAX(4, MIN(6, ms->cParams.searchLog)) == rowLog); \` | process assertion failure | [x] |
| 876 | `ZSTD_compressBlock_lazy_generic` (c_src/src/compress/zstd_lazy.c:1562) | `assert(offset_1 <= dictAndPrefixLength);` | process assertion failure | [x] |
| 877 | `ZSTD_compressBlock_lazy_generic` (c_src/src/compress/zstd_lazy.c:1563) | `assert(offset_2 <= dictAndPrefixLength);` | process assertion failure | [x] |
| 878 | `ZSTD_ldm_adjustParameters` (c_src/src/compress/zstd_ldm.c:139) | `ZSTD_STATIC_ASSERT(LDM_BUCKET_SIZE_LOG <= ZSTD_LDM_BUCKETSIZELOG_MAX);` | process assertion failure | [x] |
| 879 | `ZSTD_ldm_adjustParameters` (c_src/src/compress/zstd_ldm.c:144) | `assert(params->hashLog <= ZSTD_HASHLOG_MAX);` | process assertion failure | [x] |
| 880 | `ZSTD_ldm_adjustParameters` (c_src/src/compress/zstd_ldm.c:149) | `assert(1 <= (int)cParams->strategy && (int)cParams->strategy <= 9);` | process assertion failure | [x] |
| 881 | `ZSTD_ldm_adjustParameters` (c_src/src/compress/zstd_ldm.c:163) | `assert(1 <= (int)cParams->strategy && (int)cParams->strategy <= 9);` | process assertion failure | [x] |
| 882 | `ZSTD_ldm_fillFastTables` (c_src/src/compress/zstd_ldm.c:266) | `assert(0); /* shouldn't be called: cparams should've been adjusted. */` | process assertion failure | [x] |
| 883 | `ZSTD_ldm_fillFastTables` (c_src/src/compress/zstd_ldm.c:279) | `assert(0); /* not possible : not a valid strategy id */` | process assertion failure | [x] |
| 884 | `ZSTD_ldm_generateSequences_internal` (c_src/src/compress/zstd_ldm.c:479) | `return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 885 | `ZSTD_ldm_generateSequences` (c_src/src/compress/zstd_ldm.c:538) | `assert(ZSTD_CHUNKSIZE_MAX >= kMaxChunkSize);` | process assertion failure | [x] |
| 886 | `ZSTD_ldm_generateSequences` (c_src/src/compress/zstd_ldm.c:542) | `assert(ldmState->window.nextSrc >= (BYTE const*)src + srcSize);` | process assertion failure | [x] |
| 887 | `ZSTD_ldm_generateSequences` (c_src/src/compress/zstd_ldm.c:546) | `assert(sequences->pos <= sequences->size);` | process assertion failure | [x] |
| 888 | `ZSTD_ldm_generateSequences` (c_src/src/compress/zstd_ldm.c:547) | `assert(sequences->size <= sequences->capacity);` | process assertion failure | [x] |
| 889 | `ZSTD_ldm_generateSequences` (c_src/src/compress/zstd_ldm.c:557) | `assert(chunkStart < iend);` | process assertion failure | [x] |
| 890 | `ZSTD_ldm_generateSequences` (c_src/src/compress/zstd_ldm.c:596) | `assert(newLeftoverSize == chunkSize);` | process assertion failure | [x] |
| 891 | `maybeSplitSequence` (c_src/src/compress/zstd_ldm.c:644) | `assert(sequence.offset > 0);` | process assertion failure | [x] |
| 892 | `ZSTD_ldm_blockCompress` (c_src/src/compress/zstd_ldm.c:706) | `assert(rawSeqStore->pos <= rawSeqStore->size);` | process assertion failure | [x] |
| 893 | `ZSTD_ldm_blockCompress` (c_src/src/compress/zstd_ldm.c:707) | `assert(rawSeqStore->size <= rawSeqStore->capacity);` | process assertion failure | [x] |
| 894 | `ZSTD_ldm_blockCompress` (c_src/src/compress/zstd_ldm.c:717) | `assert(ip + sequence.litLength + sequence.matchLength <= iend);` | process assertion failure | [x] |
| 895 | `ZSTD_fracWeight` (c_src/src/compress/zstd_opt.c:63) | `assert(hb + BITCOST_ACCURACY < 31);` | process assertion failure | [x] |
| 896 | `ZSTD_downscaleStats` (c_src/src/compress/zstd_opt.c:110) | `assert(shift < 30);` | process assertion failure | [x] |
| 897 | `ZSTD_scaleStats` (c_src/src/compress/zstd_opt.c:128) | `assert(logTarget < 30);` | process assertion failure | [x] |
| 898 | `ZSTD_rescaleFreqs` (c_src/src/compress/zstd_opt.c:157) | `assert(optPtr->symbolCosts != NULL);` | process assertion failure | [x] |
| 899 | `ZSTD_rescaleFreqs` (c_src/src/compress/zstd_opt.c:166) | `assert(optPtr->litFreq != NULL);` | process assertion failure | [x] |
| 900 | `ZSTD_rescaleFreqs` (c_src/src/compress/zstd_opt.c:171) | `assert(bitCost <= scaleLog);` | process assertion failure | [x] |
| 901 | `ZSTD_rescaleFreqs` (c_src/src/compress/zstd_opt.c:183) | `assert(bitCost < scaleLog);` | process assertion failure | [x] |
| 902 | `ZSTD_rescaleFreqs` (c_src/src/compress/zstd_opt.c:195) | `assert(bitCost < scaleLog);` | process assertion failure | [x] |
| 903 | `ZSTD_rescaleFreqs` (c_src/src/compress/zstd_opt.c:207) | `assert(bitCost < scaleLog);` | process assertion failure | [x] |
| 904 | `ZSTD_rescaleFreqs` (c_src/src/compress/zstd_opt.c:214) | `assert(optPtr->litFreq != NULL);` | process assertion failure | [x] |
| 905 | `ZSTD_rawLiteralsCost` (c_src/src/compress/zstd_opt.c:283) | `assert(optPtr->litSumBasePrice >= BITCOST_MULTIPLIER);` | process assertion failure | [x] |
| 906 | `ZSTD_litLengthPrice` (c_src/src/compress/zstd_opt.c:297) | `assert(litLength <= ZSTD_BLOCKSIZE_MAX);` | process assertion failure | [x] |
| 907 | `ZSTD_getMatchPrice` (c_src/src/compress/zstd_opt.c:332) | `assert(matchLength >= MINMATCH);` | process assertion failure | [x] |
| 908 | `ZSTD_updateStats` (c_src/src/compress/zstd_opt.c:376) | `assert(offCode <= MaxOff);` | process assertion failure | [x] |
| 909 | `ZSTD_insertAndFindFirstIndexHash3` (c_src/src/compress/zstd_opt.c:421) | `assert(hashLog3 > 0);` | process assertion failure | [x] |
| 910 | `ZSTD_insertBt1` (c_src/src/compress/zstd_opt.c:484) | `assert(curr <= target);` | process assertion failure | [x] |
| 911 | `ZSTD_insertBt1` (c_src/src/compress/zstd_opt.c:485) | `assert(ip <= iend-8); /* required for h calculation */` | process assertion failure | [x] |
| 912 | `ZSTD_insertBt1` (c_src/src/compress/zstd_opt.c:488) | `assert(windowLow > 0);` | process assertion failure | [x] |
| 913 | `ZSTD_insertBt1` (c_src/src/compress/zstd_opt.c:492) | `assert(matchIndex < curr);` | process assertion failure | [x] |
| 914 | `ZSTD_insertBt1` (c_src/src/compress/zstd_opt.c:516) | `assert(matchIndex+matchLength >= dictLimit); /* might be wrong if actually extDict */` | process assertion failure | [x] |
| 915 | `ZSTD_insertBt1` (c_src/src/compress/zstd_opt.c:555) | `assert(matchEndIdx > curr + 8);` | process assertion failure | [x] |
| 916 | `ZSTD_updateTree_internal` (c_src/src/compress/zstd_opt.c:575) | `assert(idx < (U32)(idx + forward));` | process assertion failure | [x] |
| 917 | `ZSTD_updateTree_internal` (c_src/src/compress/zstd_opt.c:578) | `assert((size_t)(ip - base) <= (size_t)(U32)(-1));` | process assertion failure | [x] |
| 918 | `ZSTD_updateTree_internal` (c_src/src/compress/zstd_opt.c:579) | `assert((size_t)(iend - base) <= (size_t)(U32)(-1));` | process assertion failure | [x] |
| 919 | `ZSTD_insertBtAndGetAllMatches` (c_src/src/compress/zstd_opt.c:645) | `assert(ll0 <= 1); /* necessarily 1 or 0 */` | process assertion failure | [x] |
| 920 | `ZSTD_insertBtAndGetAllMatches` (c_src/src/compress/zstd_opt.c:652) | `assert(curr >= dictLimit);` | process assertion failure | [x] |
| 921 | `ZSTD_insertBtAndGetAllMatches` (c_src/src/compress/zstd_opt.c:664) | `assert(curr >= windowLow);` | process assertion failure | [x] |
| 922 | `ZSTD_insertBtAndGetAllMatches` (c_src/src/compress/zstd_opt.c:709) | `assert(curr > matchIndex3);` | process assertion failure | [x] |
| 923 | `ZSTD_insertBtAndGetAllMatches` (c_src/src/compress/zstd_opt.c:710) | `assert(mnum==0); /* no prior solution */` | process assertion failure | [x] |
| 924 | `ZSTD_insertBtAndGetAllMatches` (c_src/src/compress/zstd_opt.c:728) | `assert(curr > matchIndex);` | process assertion failure | [x] |
| 925 | `ZSTD_insertBtAndGetAllMatches` (c_src/src/compress/zstd_opt.c:731) | `assert(matchIndex+matchLength >= dictLimit); /* ensure the condition is correct when !extDict */` | process assertion failure | [x] |
| 926 | `ZSTD_insertBtAndGetAllMatches` (c_src/src/compress/zstd_opt.c:733) | `if (matchIndex >= dictLimit) assert(memcmp(match, ip, matchLength) == 0); /* ensure early section of match is equal as expected */` | process assertion failure | [x] |
| 927 | `ZSTD_insertBtAndGetAllMatches` (c_src/src/compress/zstd_opt.c:737) | `assert(memcmp(match, ip, matchLength) == 0); /* ensure early section of match is equal as expected */` | process assertion failure | [x] |
| 928 | `ZSTD_insertBtAndGetAllMatches` (c_src/src/compress/zstd_opt.c:746) | `assert(matchEndIdx > matchIndex);` | process assertion failure | [x] |
| 929 | `ZSTD_insertBtAndGetAllMatches` (c_src/src/compress/zstd_opt.c:776) | `assert(nbCompares <= (1U << ZSTD_SEARCHLOG_MAX)); /* Check we haven't underflowed. */` | process assertion failure | [x] |
| 930 | `ZSTD_insertBtAndGetAllMatches` (c_src/src/compress/zstd_opt.c:815) | `assert(matchEndIdx > curr+8);` | process assertion failure | [x] |
| 931 | `ZSTD_btGetAllMatches_internal` (c_src/src/compress/zstd_opt.c:844) | `assert(BOUNDED(3, ms->cParams.minMatch, 6) == mls);` | process assertion failure | [x] |
| 932 | `GEN_ZSTD_BT_GET_ALL_MATCHES` (c_src/src/compress/zstd_opt.c:897) | `assert((U32)dictMode < 3);` | process assertion failure | [x] |
| 933 | `GEN_ZSTD_BT_GET_ALL_MATCHES` (c_src/src/compress/zstd_opt.c:898) | `assert(mls - 3 < 4);` | process assertion failure | [x] |
| 934 | `ZSTD_opt_getNextMatchAndUpdateSeqStore` (c_src/src/compress/zstd_opt.c:958) | `assert(optLdm->seqStore.posInSequence <= currSeq.litLength + currSeq.matchLength);` | process assertion failure | [x] |
| 935 | `ZSTD_compressBlock_opt_generic` (c_src/src/compress/zstd_opt.c:1114) | `assert(optLevel <= 2);` | process assertion failure | [x] |
| 936 | `ZSTD_compressBlock_opt_generic` (c_src/src/compress/zstd_opt.c:1151) | `ZSTD_STATIC_ASSERT(sizeof(opt[0].rep[0]) == sizeof(rep[0]));` | process assertion failure | [x] |
| 937 | `ZSTD_compressBlock_opt_generic` (c_src/src/compress/zstd_opt.c:1172) | `assert(opt[0].price >= 0);` | process assertion failure | [x] |
| 938 | `ZSTD_compressBlock_opt_generic` (c_src/src/compress/zstd_opt.c:1202) | `assert(cur <= ZSTD_OPT_NUM);` | process assertion failure | [x] |
| 939 | `ZSTD_compressBlock_opt_generic` (c_src/src/compress/zstd_opt.c:1210) | `assert(price < 1000000000); /* overflow check */` | process assertion failure | [x] |
| 940 | `ZSTD_compressBlock_opt_generic` (c_src/src/compress/zstd_opt.c:1234) | `assert(cur >= prevMatch.mlen);` | process assertion failure | [x] |
| 941 | `ZSTD_compressBlock_opt_generic` (c_src/src/compress/zstd_opt.c:1254) | `ZSTD_STATIC_ASSERT(sizeof(opt[cur].rep) == sizeof(Repcodes_t));` | process assertion failure | [x] |
| 942 | `ZSTD_compressBlock_opt_generic` (c_src/src/compress/zstd_opt.c:1255) | `assert(cur >= opt[cur].mlen);` | process assertion failure | [x] |
| 943 | `ZSTD_compressBlock_opt_generic` (c_src/src/compress/zstd_opt.c:1274) | `assert(opt[cur].price >= 0);` | process assertion failure | [x] |
| 944 | `ZSTD_compressBlock_opt_generic` (c_src/src/compress/zstd_opt.c:1341) | `assert(cur >= lastStretch.mlen);` | process assertion failure | [x] |
| 945 | `ZSTD_compressBlock_opt_generic` (c_src/src/compress/zstd_opt.c:1345) | `assert(opt[0].mlen == 0);` | process assertion failure | [x] |
| 946 | `ZSTD_compressBlock_opt_generic` (c_src/src/compress/zstd_opt.c:1346) | `assert(last_pos >= lastStretch.mlen);` | process assertion failure | [x] |
| 947 | `ZSTD_compressBlock_opt_generic` (c_src/src/compress/zstd_opt.c:1347) | `assert(cur == last_pos - lastStretch.mlen);` | process assertion failure | [x] |
| 948 | `ZSTD_compressBlock_opt_generic` (c_src/src/compress/zstd_opt.c:1351) | `assert(lastStretch.litlen == (ip - anchor) + last_pos);` | process assertion failure | [x] |
| 949 | `ZSTD_compressBlock_opt_generic` (c_src/src/compress/zstd_opt.c:1355) | `assert(lastStretch.off > 0);` | process assertion failure | [x] |
| 950 | `ZSTD_compressBlock_opt_generic` (c_src/src/compress/zstd_opt.c:1364) | `assert(cur >= lastStretch.litlen);` | process assertion failure | [x] |
| 951 | `ZSTD_compressBlock_opt_generic` (c_src/src/compress/zstd_opt.c:1382) | `assert(storeEnd < ZSTD_OPT_SIZE);` | process assertion failure | [x] |
| 952 | `ZSTD_compressBlock_opt_generic` (c_src/src/compress/zstd_opt.c:1406) | `assert(nextStretch.litlen + nextStretch.mlen <= stretchPos);` | process assertion failure | [x] |
| 953 | `ZSTD_compressBlock_opt_generic` (c_src/src/compress/zstd_opt.c:1422) | `assert(storePos == storeEnd); /* must be last sequence */` | process assertion failure | [x] |
| 954 | `ZSTD_compressBlock_opt_generic` (c_src/src/compress/zstd_opt.c:1427) | `assert(anchor + llen <= iend);` | process assertion failure | [x] |
| 955 | `ZSTD_initStats_ultra` (c_src/src/compress/zstd_opt.c:1493) | `assert(ms->opt.litLengthSum == 0); /* first block */` | process assertion failure | [x] |
| 956 | `ZSTD_initStats_ultra` (c_src/src/compress/zstd_opt.c:1494) | `assert(seqStore->sequences == seqStore->sequencesStart); /* no ldm */` | process assertion failure | [x] |
| 957 | `ZSTD_initStats_ultra` (c_src/src/compress/zstd_opt.c:1495) | `assert(ms->window.dictLimit == ms->window.lowLimit); /* no dictionary */` | process assertion failure | [x] |
| 958 | `ZSTD_initStats_ultra` (c_src/src/compress/zstd_opt.c:1496) | `assert(ms->window.dictLimit - ms->nextToUpdate <= 1); /* no prefix (note: intentional overflow, defined as 2-complement) */` | process assertion failure | [x] |
| 959 | `ZSTD_compressBlock_btultra2` (c_src/src/compress/zstd_opt.c:1532) | `assert(srcSize <= ZSTD_BLOCKSIZE_MAX);` | process assertion failure | [x] |
| 960 | `hash2` (c_src/src/compress/zstd_preSplit.c:35) | `assert(hashLog >= 8);` | process assertion failure | [x] |
| 961 | `hash2` (c_src/src/compress/zstd_preSplit.c:37) | `assert(hashLog <= HASHLOG_MAX);` | process assertion failure | [x] |
| 962 | `addEvents_generic` (c_src/src/compress/zstd_preSplit.c:62) | `assert(srcSize >= HASHLENGTH);` | process assertion failure | [x] |
| 963 | `fpDistance` (c_src/src/compress/zstd_preSplit.c:99) | `assert(hashLog <= HASHLOG_MAX);` | process assertion failure | [x] |
| 964 | `compareFingerprints` (c_src/src/compress/zstd_preSplit.c:115) | `assert(ref->nbEvents > 0);` | process assertion failure | [x] |
| 965 | `compareFingerprints` (c_src/src/compress/zstd_preSplit.c:116) | `assert(newfp->nbEvents > 0);` | process assertion failure | [x] |
| 966 | `removeEvents` (c_src/src/compress/zstd_preSplit.c:147) | `assert(acc->events[n] >= slice->events[n]);` | process assertion failure | [x] |
| 967 | `ZSTD_splitBlock_byChunks` (c_src/src/compress/zstd_preSplit.c:162) | `const RecordEvents_f record_f = (assert(0<=level && level<=3), records_fs[level]);` | process assertion failure | [x] |
| 968 | `ZSTD_splitBlock_byChunks` (c_src/src/compress/zstd_preSplit.c:167) | `assert(blockSize == (128 << 10));` | process assertion failure | [x] |
| 969 | `ZSTD_splitBlock_byChunks` (c_src/src/compress/zstd_preSplit.c:168) | `assert(workspace != NULL);` | process assertion failure | [x] |
| 970 | `ZSTD_splitBlock_byChunks` (c_src/src/compress/zstd_preSplit.c:169) | `assert((size_t)workspace % ZSTD_ALIGNOF(FPStats) == 0);` | process assertion failure | [x] |
| 971 | `ZSTD_splitBlock_byChunks` (c_src/src/compress/zstd_preSplit.c:170) | `ZSTD_STATIC_ASSERT(ZSTD_SLIPBLOCK_WORKSPACESIZE >= sizeof(FPStats));` | process assertion failure | [x] |
| 972 | `ZSTD_splitBlock_byChunks` (c_src/src/compress/zstd_preSplit.c:171) | `assert(wkspSize >= sizeof(FPStats)); (void)wkspSize;` | process assertion failure | [x] |
| 973 | `ZSTD_splitBlock_byChunks` (c_src/src/compress/zstd_preSplit.c:184) | `assert(pos == blockSize);` | process assertion failure | [x] |
| 974 | `ZSTD_splitBlock_fromBorders` (c_src/src/compress/zstd_preSplit.c:204) | `assert(blockSize == (128 << 10));` | process assertion failure | [x] |
| 975 | `ZSTD_splitBlock_fromBorders` (c_src/src/compress/zstd_preSplit.c:205) | `assert(workspace != NULL);` | process assertion failure | [x] |
| 976 | `ZSTD_splitBlock_fromBorders` (c_src/src/compress/zstd_preSplit.c:206) | `assert((size_t)workspace % ZSTD_ALIGNOF(FPStats) == 0);` | process assertion failure | [x] |
| 977 | `ZSTD_splitBlock_fromBorders` (c_src/src/compress/zstd_preSplit.c:207) | `ZSTD_STATIC_ASSERT(ZSTD_SLIPBLOCK_WORKSPACESIZE >= sizeof(FPStats));` | process assertion failure | [x] |
| 978 | `ZSTD_splitBlock_fromBorders` (c_src/src/compress/zstd_preSplit.c:208) | `assert(wkspSize >= sizeof(FPStats)); (void)wkspSize;` | process assertion failure | [x] |
| 979 | `ZSTD_splitBlock` (c_src/src/compress/zstd_preSplit.c:233) | `assert(0<=level && level<=4);` | process assertion failure | [x] |
| 980 | `ZSTDMT_createBufferPool` (c_src/src/compress/zstdmt_compress.c:126) | `if (bufPool==NULL) return NULL;` | `NULL` | [x] |
| 981 | `ZSTDMT_createBufferPool` (c_src/src/compress/zstdmt_compress.c:129) | `return NULL;` | `NULL` | [x] |
| 982 | `ZSTDMT_createBufferPool` (c_src/src/compress/zstdmt_compress.c:134) | `return NULL;` | `NULL` | [x] |
| 983 | `ZSTDMT_expandBufferPool` (c_src/src/compress/zstdmt_compress.c:173) | `if (srcBufPool==NULL) return NULL;` | `NULL` | [x] |
| 984 | `ZSTDMT_resizeBuffer` (c_src/src/compress/zstdmt_compress.c:243) | `assert(newBuffer.capacity >= buffer.capacity);` | process assertion failure | [x] |
| 985 | `ZSTDMT_createSeqPool` (c_src/src/compress/zstdmt_compress.c:337) | `if (seqPool == NULL) return NULL;` | `NULL` | [x] |
| 986 | `ZSTDMT_createCCtxPool` (c_src/src/compress/zstdmt_compress.c:385) | `assert(nbWorkers > 0);` | process assertion failure | [x] |
| 987 | `ZSTDMT_createCCtxPool` (c_src/src/compress/zstdmt_compress.c:386) | `if (!cctxPool) return NULL;` | `NULL` | [x] |
| 988 | `ZSTDMT_createCCtxPool` (c_src/src/compress/zstdmt_compress.c:389) | `return NULL;` | `NULL` | [x] |
| 989 | `ZSTDMT_createCCtxPool` (c_src/src/compress/zstdmt_compress.c:395) | `return NULL;` | `NULL` | [x] |
| 990 | `ZSTDMT_createCCtxPool` (c_src/src/compress/zstdmt_compress.c:399) | `if (!cctxPool->cctxs[0]) { ZSTDMT_freeCCtxPool(cctxPool); return NULL; }` | `NULL` | [x] |
| 991 | `ZSTDMT_expandCCtxPool` (c_src/src/compress/zstdmt_compress.c:408) | `if (srcPool==NULL) return NULL;` | `NULL` | [x] |
| 992 | `ZSTDMT_sizeof_CCtxPool` (c_src/src/compress/zstdmt_compress.c:430) | `assert(nbWorkers > 0);` | process assertion failure | [x] |
| 993 | `ZSTDMT_serialState_reset` (c_src/src/compress/zstdmt_compress.c:499) | `assert(params.ldmParams.hashLog >= params.ldmParams.bucketSizeLog);` | process assertion failure | [x] |
| 994 | `ZSTDMT_serialState_reset` (c_src/src/compress/zstdmt_compress.c:500) | `assert(params.ldmParams.hashRateLog < 32);` | process assertion failure | [x] |
| 995 | `ZSTDMT_serialState_genSequences` (c_src/src/compress/zstdmt_compress.c:597) | `assert(seqStore->seq != NULL && seqStore->pos == 0 &&` | process assertion failure | [x] |
| 996 | `ZSTDMT_serialState_genSequences` (c_src/src/compress/zstdmt_compress.c:599) | `assert(src.size <= serialState->params.jobSize);` | process assertion failure | [x] |
| 997 | `ZSTDMT_serialState_genSequences` (c_src/src/compress/zstdmt_compress.c:605) | `assert(!ZSTD_isError(error)); (void)error;` | process assertion failure | [x] |
| 998 | `ZSTDMT_serialState_applySequences` (c_src/src/compress/zstdmt_compress.c:624) | `ZSTDMT_serialState_applySequences(const SerialState* serialState, /* just for an assert() check */` | process assertion failure | [x] |
| 999 | `ZSTDMT_serialState_applySequences` (c_src/src/compress/zstdmt_compress.c:630) | `assert(serialState->params.ldmParams.enableLdm == ZSTD_ps_enable); (void)serialState;` | process assertion failure | [x] |
| 1000 | `ZSTDMT_serialState_applySequences` (c_src/src/compress/zstdmt_compress.c:631) | `assert(jobCCtx);` | process assertion failure | [x] |
| 1001 | `ZSTDMT_serialState_ensureFinished` (c_src/src/compress/zstdmt_compress.c:641) | `assert(ZSTD_isError(cSize)); (void)cSize;` | process assertion failure | [x] |
| 1002 | `ZSTDMT_compressionJob` (c_src/src/compress/zstdmt_compress.c:730) | `assert(job->firstJob); /* only allowed for first job */` | process assertion failure | [x] |
| 1003 | `ZSTDMT_compressionJob` (c_src/src/compress/zstdmt_compress.c:768) | `if (sizeof(size_t) > sizeof(int)) assert(job->src.size < ((size_t)INT_MAX) * chunkSize); /* check overflow */` | process assertion failure | [x] |
| 1004 | `ZSTDMT_compressionJob` (c_src/src/compress/zstdmt_compress.c:770) | `assert(job->cSize == 0);` | process assertion failure | [x] |
| 1005 | `ZSTDMT_compressionJob` (c_src/src/compress/zstdmt_compress.c:775) | `op += cSize; assert(op < oend);` | process assertion failure | [x] |
| 1006 | `ZSTDMT_compressionJob` (c_src/src/compress/zstdmt_compress.c:786) | `assert(chunkSize > 0);` | process assertion failure | [x] |
| 1007 | `ZSTDMT_compressionJob` (c_src/src/compress/zstdmt_compress.c:787) | `assert((chunkSize & (chunkSize - 1)) == 0); /* chunkSize must be power of 2 for mask==(chunkSize-1) to work */` | process assertion failure | [x] |
| 1008 | `ZSTDMT_compressionJob` (c_src/src/compress/zstdmt_compress.c:801) | `assert(!ZSTD_window_hasExtDict(cctx->blockState.matchState.window));` | process assertion failure | [x] |
| 1009 | `ZSTDMT_compressionJob` (c_src/src/compress/zstdmt_compress.c:815) | `if (ZSTD_isError(job->cSize)) assert(lastCBlockSize == 0);` | process assertion failure | [x] |
| 1010 | `ZSTDMT_createJobsTable` (c_src/src/compress/zstdmt_compress.c:916) | `if (jobTable==NULL) return NULL;` | `NULL` | [x] |
| 1011 | `ZSTDMT_createJobsTable` (c_src/src/compress/zstdmt_compress.c:924) | `return NULL;` | `NULL` | [x] |
| 1012 | `ZSTDMT_expandJobsTable` (c_src/src/compress/zstdmt_compress.c:935) | `if (mtctx->jobs==NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [x] |
| 1013 | `ZSTDMT_expandJobsTable` (c_src/src/compress/zstdmt_compress.c:936) | `assert((nbJobs != 0) && ((nbJobs & (nbJobs - 1)) == 0)); /* ensure nbJobs is a power of 2 */` | process assertion failure | [x] |
| 1014 | `ZSTDMT_createCCtx_advanced_internal` (c_src/src/compress/zstdmt_compress.c:957) | `if (nbWorkers < 1) return NULL;` | `NULL` | [x] |
| 1015 | `ZSTDMT_createCCtx_advanced_internal` (c_src/src/compress/zstdmt_compress.c:961) | `return NULL;` | `NULL` | [x] |
| 1016 | `ZSTDMT_createCCtx_advanced_internal` (c_src/src/compress/zstdmt_compress.c:964) | `if (!mtctx) return NULL;` | `NULL` | [x] |
| 1017 | `ZSTDMT_createCCtx_advanced_internal` (c_src/src/compress/zstdmt_compress.c:977) | `assert(nbJobs > 0); assert((nbJobs & (nbJobs - 1)) == 0); /* ensure nbJobs is a power of 2 */` | process assertion failure | [x] |
| 1018 | `ZSTDMT_createCCtx_advanced_internal` (c_src/src/compress/zstdmt_compress.c:986) | `return NULL;` | `NULL` | [x] |
| 1019 | `ZSTDMT_createCCtx_advanced` (c_src/src/compress/zstdmt_compress.c:1000) | `return NULL;` | `NULL` | [x] |
| 1020 | `ZSTDMT_resize` (c_src/src/compress/zstdmt_compress.c:1080) | `if (POOL_resize(mtctx->factory, nbWorkers)) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [x] |
| 1021 | `ZSTDMT_resize` (c_src/src/compress/zstdmt_compress.c:1083) | `if (mtctx->bufPool == NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [x] |
| 1022 | `ZSTDMT_resize` (c_src/src/compress/zstdmt_compress.c:1085) | `if (mtctx->cctxPool == NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [x] |
| 1023 | `ZSTDMT_resize` (c_src/src/compress/zstdmt_compress.c:1087) | `if (mtctx->seqPool == NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [x] |
| 1024 | `ZSTDMT_getFrameProgression` (c_src/src/compress/zstdmt_compress.c:1123) | `unsigned lastJobNb = mtctx->nextJobID + mtctx->jobReady; assert(mtctx->jobReady <= 1);` | process assertion failure | [x] |
| 1025 | `ZSTDMT_getFrameProgression` (c_src/src/compress/zstdmt_compress.c:1133) | `assert(flushed <= produced);` | process assertion failure | [x] |
| 1026 | `ZSTDMT_toFlushNow` (c_src/src/compress/zstdmt_compress.c:1151) | `assert(jobID <= mtctx->nextJobID);` | process assertion failure | [x] |
| 1027 | `ZSTDMT_toFlushNow` (c_src/src/compress/zstdmt_compress.c:1161) | `assert(flushed <= produced);` | process assertion failure | [x] |
| 1028 | `ZSTDMT_toFlushNow` (c_src/src/compress/zstdmt_compress.c:1162) | `assert(jobPtr->consumed <= jobPtr->src.size);` | process assertion failure | [x] |
| 1029 | `ZSTDMT_toFlushNow` (c_src/src/compress/zstdmt_compress.c:1170) | `assert(jobPtr->consumed < jobPtr->src.size);` | process assertion failure | [x] |
| 1030 | `ZSTDMT_overlapLog` (c_src/src/compress/zstdmt_compress.c:1221) | `assert(0 <= ovlog && ovlog <= 9);` | process assertion failure | [x] |
| 1031 | `ZSTDMT_computeOverlapSize` (c_src/src/compress/zstdmt_compress.c:1230) | `assert(0 <= overlapRLog && overlapRLog <= 8);` | process assertion failure | [x] |
| 1032 | `ZSTDMT_computeOverlapSize` (c_src/src/compress/zstdmt_compress.c:1239) | `assert(0 <= ovLog && ovLog <= ZSTD_WINDOWLOG_MAX);` | process assertion failure | [x] |
| 1033 | `ZSTDMT_initCStream_internal` (c_src/src/compress/zstdmt_compress.c:1259) | `assert(!ZSTD_isError(ZSTD_checkCParams(params.cParams)));` | process assertion failure | [x] |
| 1034 | `ZSTDMT_initCStream_internal` (c_src/src/compress/zstdmt_compress.c:1260) | `assert(!((dict) && (cdict))); /* either dict or cdict, not both */` | process assertion failure | [x] |
| 1035 | `ZSTDMT_initCStream_internal` (c_src/src/compress/zstdmt_compress.c:1283) | `if (mtctx->cdictLocal == NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [x] |
| 1036 | `ZSTDMT_initCStream_internal` (c_src/src/compress/zstdmt_compress.c:1295) | `assert(mtctx->targetSectionSize <= (size_t)ZSTDMT_JOBSIZE_MAX);` | process assertion failure | [x] |
| 1037 | `ZSTDMT_initCStream_internal` (c_src/src/compress/zstdmt_compress.c:1300) | `U32 const rsyncBits = (assert(jobSizeKB >= 1), ZSTD_highbit32(jobSizeKB) + 10);` | process assertion failure | [x] |
| 1038 | `ZSTDMT_initCStream_internal` (c_src/src/compress/zstdmt_compress.c:1303) | `assert(rsyncBits >= RSYNC_MIN_BLOCK_LOG + 2);` | process assertion failure | [x] |
| 1039 | `ZSTDMT_initCStream_internal` (c_src/src/compress/zstdmt_compress.c:1334) | `return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [x] |
| 1040 | `ZSTDMT_initCStream_internal` (c_src/src/compress/zstdmt_compress.c:1365) | `if (mtctx->cdictLocal == NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [x] |
| 1041 | `ZSTDMT_initCStream_internal` (c_src/src/compress/zstdmt_compress.c:1373) | `return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [x] |
| 1042 | `ZSTDMT_writeLastEmptyBlock` (c_src/src/compress/zstdmt_compress.c:1387) | `assert(job->lastJob == 1);` | process assertion failure | [x] |
| 1043 | `ZSTDMT_writeLastEmptyBlock` (c_src/src/compress/zstdmt_compress.c:1388) | `assert(job->src.size == 0); /* last job is empty -> will be simplified into a last empty block */` | process assertion failure | [x] |
| 1044 | `ZSTDMT_writeLastEmptyBlock` (c_src/src/compress/zstdmt_compress.c:1389) | `assert(job->firstJob == 0); /* cannot be first job, as it also needs to create frame header */` | process assertion failure | [x] |
| 1045 | `ZSTDMT_writeLastEmptyBlock` (c_src/src/compress/zstdmt_compress.c:1390) | `assert(job->dstBuff.start == NULL); /* invoked from streaming variant only (otherwise, dstBuff might be user's output) */` | process assertion failure | [x] |
| 1046 | `ZSTDMT_writeLastEmptyBlock` (c_src/src/compress/zstdmt_compress.c:1396) | `assert(job->dstBuff.capacity >= ZSTD_blockHeaderSize); /* no buffer should ever be that small */` | process assertion failure | [x] |
| 1047 | `ZSTDMT_writeLastEmptyBlock` (c_src/src/compress/zstdmt_compress.c:1399) | `assert(!ZSTD_isError(job->cSize));` | process assertion failure | [x] |
| 1048 | `ZSTDMT_writeLastEmptyBlock` (c_src/src/compress/zstdmt_compress.c:1400) | `assert(job->consumed == 0);` | process assertion failure | [x] |
| 1049 | `ZSTDMT_createCompressionJob` (c_src/src/compress/zstdmt_compress.c:1410) | `assert((mtctx->nextJobID & mtctx->jobIDMask) == (mtctx->doneJobID & mtctx->jobIDMask));` | process assertion failure | [x] |
| 1050 | `ZSTDMT_createCompressionJob` (c_src/src/compress/zstdmt_compress.c:1420) | `assert(mtctx->inBuff.filled >= srcSize);` | process assertion failure | [x] |
| 1051 | `ZSTDMT_createCompressionJob` (c_src/src/compress/zstdmt_compress.c:1458) | `assert(endOp == ZSTD_e_end); /* only possible case : need to end the frame with an empty last block */` | process assertion failure | [x] |
| 1052 | `ZSTDMT_flushProduced` (c_src/src/compress/zstdmt_compress.c:1493) | `assert(output->size >= output->pos);` | process assertion failure | [x] |
| 1053 | `ZSTDMT_flushProduced` (c_src/src/compress/zstdmt_compress.c:1498) | `assert(mtctx->jobs[wJobID].dstFlushed <= mtctx->jobs[wJobID].cSize);` | process assertion failure | [x] |
| 1054 | `ZSTDMT_flushProduced` (c_src/src/compress/zstdmt_compress.c:1523) | `assert(srcConsumed <= srcSize);` | process assertion failure | [x] |
| 1055 | `ZSTDMT_flushProduced` (c_src/src/compress/zstdmt_compress.c:1538) | `assert(mtctx->doneJobID < mtctx->nextJobID);` | process assertion failure | [x] |
| 1056 | `ZSTDMT_flushProduced` (c_src/src/compress/zstdmt_compress.c:1539) | `assert(cSize >= mtctx->jobs[wJobID].dstFlushed);` | process assertion failure | [x] |
| 1057 | `ZSTDMT_flushProduced` (c_src/src/compress/zstdmt_compress.c:1540) | `assert(mtctx->jobs[wJobID].dstBuff.start != NULL);` | process assertion failure | [x] |
| 1058 | `ZSTDMT_getInputDataInUse` (c_src/src/compress/zstdmt_compress.c:1605) | `assert(range.start <= mtctx->jobs[wJobID].src.start);` | process assertion failure | [x] |
| 1059 | `ZSTDMT_tryGetInputRange` (c_src/src/compress/zstdmt_compress.c:1688) | `assert(mtctx->inBuff.buffer.start == NULL);` | process assertion failure | [x] |
| 1060 | `ZSTDMT_tryGetInputRange` (c_src/src/compress/zstdmt_compress.c:1689) | `assert(mtctx->roundBuff.capacity >= spaceNeeded);` | process assertion failure | [x] |
| 1061 | `ZSTDMT_tryGetInputRange` (c_src/src/compress/zstdmt_compress.c:1716) | `assert(!ZSTDMT_isOverlapped(buffer, mtctx->inBuff.prefix));` | process assertion failure | [x] |
| 1062 | `ZSTDMT_tryGetInputRange` (c_src/src/compress/zstdmt_compress.c:1730) | `assert(mtctx->roundBuff.pos + buffer.capacity <= mtctx->roundBuff.capacity);` | process assertion failure | [x] |
| 1063 | `findSynchronizationPoint` (c_src/src/compress/zstdmt_compress.c:1787) | `assert(mtctx->inBuff.filled >= RSYNC_LENGTH);` | process assertion failure | [x] |
| 1064 | `findSynchronizationPoint` (c_src/src/compress/zstdmt_compress.c:1797) | `assert(mtctx->inBuff.filled >= RSYNC_MIN_BLOCK_SIZE);` | process assertion failure | [x] |
| 1065 | `findSynchronizationPoint` (c_src/src/compress/zstdmt_compress.c:1798) | `assert(RSYNC_MIN_BLOCK_SIZE >= RSYNC_LENGTH);` | process assertion failure | [x] |
| 1066 | `findSynchronizationPoint` (c_src/src/compress/zstdmt_compress.c:1821) | `assert(pos < RSYNC_LENGTH \|\| ZSTD_rollingHash_compute(istart + pos - RSYNC_LENGTH, RSYNC_LENGTH) == hash);` | process assertion failure | [x] |
| 1067 | `findSynchronizationPoint` (c_src/src/compress/zstdmt_compress.c:1827) | `* assert(pos < RSYNC_LENGTH \|\| ZSTD_rollingHash_compute(istart + pos - RSYNC_LENGTH, RSYNC_LENGTH) == hash);` | process assertion failure | [x] |
| 1068 | `findSynchronizationPoint` (c_src/src/compress/zstdmt_compress.c:1830) | `assert(mtctx->inBuff.filled + pos >= RSYNC_MIN_BLOCK_SIZE);` | process assertion failure | [x] |
| 1069 | `findSynchronizationPoint` (c_src/src/compress/zstdmt_compress.c:1838) | `assert(pos < RSYNC_LENGTH \|\| ZSTD_rollingHash_compute(istart + pos - RSYNC_LENGTH, RSYNC_LENGTH) == hash);` | process assertion failure | [x] |
| 1070 | `ZSTDMT_compressStream_generic` (c_src/src/compress/zstdmt_compress.c:1861) | `assert(output->pos <= output->size);` | process assertion failure | [x] |
| 1071 | `ZSTDMT_compressStream_generic` (c_src/src/compress/zstdmt_compress.c:1862) | `assert(input->pos <= input->size);` | process assertion failure | [x] |
| 1072 | `ZSTDMT_compressStream_generic` (c_src/src/compress/zstdmt_compress.c:1866) | `return ERROR(stage_wrong);` | `ERROR(stage_wrong)` | [x] |
| 1073 | `ZSTDMT_compressStream_generic` (c_src/src/compress/zstdmt_compress.c:1873) | `assert(mtctx->inBuff.filled == 0); /* Can't fill an empty buffer */` | process assertion failure | [x] |
| 1074 | `ZSTDMT_compressStream_generic` (c_src/src/compress/zstdmt_compress.c:1879) | `assert(mtctx->doneJobID != mtctx->nextJobID);` | process assertion failure | [x] |
| 1075 | `ZSTDMT_compressStream_generic` (c_src/src/compress/zstdmt_compress.c:1888) | `assert(mtctx->inBuff.buffer.capacity >= mtctx->targetSectionSize);` | process assertion failure | [x] |
| 1076 | `ZSTDMT_compressStream_generic` (c_src/src/compress/zstdmt_compress.c:1904) | `assert(mtctx->inBuff.filled == 0 \|\| mtctx->inBuff.filled == mtctx->targetSectionSize \|\| mtctx->params.rsyncable);` | process assertion failure | [x] |
| 1077 | `ZSTDMT_compressStream_generic` (c_src/src/compress/zstdmt_compress.c:1913) | `assert(mtctx->inBuff.filled <= mtctx->targetSectionSize);` | process assertion failure | [x] |
| 1078 | `(file scope)` (c_src/src/compress/zstdmt_compress.h:46) | `/* Requires ZSTD_MULTITHREAD to be defined during compilation, otherwise it will return NULL. */` | `NULL` | [x] |
| 1079 | `HUF_initFastDStream` (c_src/src/decompress/huf_decompress.c:154) | `assert(bitsConsumed <= 8);` | process assertion failure | [x] |
| 1080 | `HUF_initFastDStream` (c_src/src/decompress/huf_decompress.c:155) | `assert(sizeof(size_t) == 8);` | process assertion failure | [x] |
| 1081 | `HUF_DecompressFastArgs_init` (c_src/src/decompress/huf_decompress.c:209) | `assert(dst != NULL);` | process assertion failure | [x] |
| 1082 | `HUF_DecompressFastArgs_init` (c_src/src/decompress/huf_decompress.c:213) | `return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1083 | `HUF_DecompressFastArgs_init` (c_src/src/decompress/huf_decompress.c:238) | `if (length4 > srcSize) return ERROR(corruption_detected); /* overflow */` | `ERROR(corruption_detected)` | [x] |
| 1084 | `HUF_initRemainingDStream` (c_src/src/decompress/huf_decompress.c:285) | `return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1085 | `HUF_initRemainingDStream` (c_src/src/decompress/huf_decompress.c:292) | `return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1086 | `HUF_initRemainingDStream` (c_src/src/decompress/huf_decompress.c:295) | `assert(sizeof(size_t) == 8);` | process assertion failure | [x] |
| 1087 | `HUF_DEltX1_set4` (c_src/src/decompress/huf_decompress.c:342) | `assert(D4 < (1U << 16));` | process assertion failure | [x] |
| 1088 | `HUF_readDTableX1_wksp` (c_src/src/decompress/huf_decompress.c:394) | `DEBUG_STATIC_ASSERT(HUF_DECOMPRESS_WORKSPACE_SIZE >= sizeof(*wksp));` | process assertion failure | [x] |
| 1089 | `HUF_readDTableX1_wksp` (c_src/src/decompress/huf_decompress.c:395) | `if (sizeof(*wksp) > wkspSize) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [x] |
| 1090 | `HUF_readDTableX1_wksp` (c_src/src/decompress/huf_decompress.c:397) | `DEBUG_STATIC_ASSERT(sizeof(DTableDesc) == sizeof(HUF_DTable));` | process assertion failure | [x] |
| 1091 | `HUF_readDTableX1_wksp` (c_src/src/decompress/huf_decompress.c:409) | `if (tableLog > (U32)(dtd.maxTableLog+1)) return ERROR(tableLog_tooLarge); /* DTable too small, Huffman tree cannot fit in */` | `ERROR(tableLog_tooLarge)` | [x] |
| 1092 | `HUF_readDTableX1_wksp` (c_src/src/decompress/huf_decompress.c:509) | `assert(u == length);` | process assertion failure | [x] |
| 1093 | `HUF_decompress1X1_usingDTable_internal_body` (c_src/src/decompress/huf_decompress.c:592) | `if (!BIT_endOfDStream(&bitD)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1094 | `HUF_decompress4X1_usingDTable_internal_body` (c_src/src/decompress/huf_decompress.c:608) | `if (cSrcSize < 10) return ERROR(corruption_detected); /* strict minimum : jump table + 1 byte per stream */` | `ERROR(corruption_detected)` | [x] |
| 1095 | `HUF_decompress4X1_usingDTable_internal_body` (c_src/src/decompress/huf_decompress.c:609) | `if (dstSize < 6) return ERROR(corruption_detected); /* stream 4-split doesn't work */` | `ERROR(corruption_detected)` | [x] |
| 1096 | `HUF_decompress4X1_usingDTable_internal_body` (c_src/src/decompress/huf_decompress.c:643) | `if (length4 > cSrcSize) return ERROR(corruption_detected); /* overflow */` | `ERROR(corruption_detected)` | [x] |
| 1097 | `HUF_decompress4X1_usingDTable_internal_body` (c_src/src/decompress/huf_decompress.c:644) | `if (opStart4 > oend) return ERROR(corruption_detected); /* overflow */` | `ERROR(corruption_detected)` | [x] |
| 1098 | `HUF_decompress4X1_usingDTable_internal_body` (c_src/src/decompress/huf_decompress.c:645) | `assert(dstSize >= 6); /* validated above */` | process assertion failure | [x] |
| 1099 | `HUF_decompress4X1_usingDTable_internal_body` (c_src/src/decompress/huf_decompress.c:680) | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1100 | `HUF_decompress4X1_usingDTable_internal_body` (c_src/src/decompress/huf_decompress.c:681) | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1101 | `HUF_decompress4X1_usingDTable_internal_body` (c_src/src/decompress/huf_decompress.c:682) | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1102 | `HUF_decompress4X1_usingDTable_internal_body` (c_src/src/decompress/huf_decompress.c:693) | `if (!endCheck) return ERROR(corruption_detected); }` | `ERROR(corruption_detected)` | [x] |
| 1103 | `HUF_decompress4X1_usingDTable_internal_fast_c_loop` (c_src/src/decompress/huf_decompress.c:735) | `assert(MEM_isLittleEndian());` | process assertion failure | [x] |
| 1104 | `HUF_decompress4X1_usingDTable_internal_fast_c_loop` (c_src/src/decompress/huf_decompress.c:736) | `assert(!MEM_32bits());` | process assertion failure | [x] |
| 1105 | `HUF_decompress4X1_usingDTable_internal_fast_c_loop` (c_src/src/decompress/huf_decompress.c:745) | `assert(op[stream] <= (stream == 3 ? oend : op[stream + 1]));` | process assertion failure | [x] |
| 1106 | `HUF_decompress4X1_usingDTable_internal_fast_c_loop` (c_src/src/decompress/huf_decompress.c:746) | `assert(ip[stream] >= ilowest);` | process assertion failure | [x] |
| 1107 | `HUF_decompress4X1_usingDTable_internal_fast_c_loop` (c_src/src/decompress/huf_decompress.c:783) | `assert(ip[stream] >= ip[stream - 1]);` | process assertion failure | [x] |
| 1108 | `HUF_decompress4X1_usingDTable_internal_fast` (c_src/src/decompress/huf_decompress.c:856) | `assert(args.ip[0] >= args.ilowest);` | process assertion failure | [x] |
| 1109 | `HUF_decompress4X1_usingDTable_internal_fast` (c_src/src/decompress/huf_decompress.c:862) | `assert(args.ip[0] >= ilowest);` | process assertion failure | [x] |
| 1110 | `HUF_decompress4X1_usingDTable_internal_fast` (c_src/src/decompress/huf_decompress.c:863) | `assert(args.ip[0] >= ilowest);` | process assertion failure | [x] |
| 1111 | `HUF_decompress4X1_usingDTable_internal_fast` (c_src/src/decompress/huf_decompress.c:864) | `assert(args.ip[1] >= ilowest);` | process assertion failure | [x] |
| 1112 | `HUF_decompress4X1_usingDTable_internal_fast` (c_src/src/decompress/huf_decompress.c:865) | `assert(args.ip[2] >= ilowest);` | process assertion failure | [x] |
| 1113 | `HUF_decompress4X1_usingDTable_internal_fast` (c_src/src/decompress/huf_decompress.c:866) | `assert(args.ip[3] >= ilowest);` | process assertion failure | [x] |
| 1114 | `HUF_decompress4X1_usingDTable_internal_fast` (c_src/src/decompress/huf_decompress.c:867) | `assert(args.op[3] <= oend);` | process assertion failure | [x] |
| 1115 | `HUF_decompress4X1_usingDTable_internal_fast` (c_src/src/decompress/huf_decompress.c:869) | `assert(ilowest == args.ilowest);` | process assertion failure | [x] |
| 1116 | `HUF_decompress4X1_usingDTable_internal_fast` (c_src/src/decompress/huf_decompress.c:870) | `assert(ilowest + 6 == args.iend[0]);` | process assertion failure | [x] |
| 1117 | `HUF_decompress4X1_usingDTable_internal_fast` (c_src/src/decompress/huf_decompress.c:886) | `if (args.op[i] != segmentEnd) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1118 | `HUF_decompress4X1_usingDTable_internal_fast` (c_src/src/decompress/huf_decompress.c:891) | `assert(dstSize != 0);` | process assertion failure | [x] |
| 1119 | `HUF_decompress4X1_DCtx_wksp` (c_src/src/decompress/huf_decompress.c:938) | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1120 | `HUF_buildDEltX2U32` (c_src/src/decompress/huf_decompress.c:964) | `DEBUG_STATIC_ASSERT(offsetof(HUF_DEltX2, sequence) == 0);` | process assertion failure | [x] |
| 1121 | `HUF_buildDEltX2U32` (c_src/src/decompress/huf_decompress.c:965) | `DEBUG_STATIC_ASSERT(offsetof(HUF_DEltX2, nbBits) == 2);` | process assertion failure | [x] |
| 1122 | `HUF_buildDEltX2U32` (c_src/src/decompress/huf_decompress.c:966) | `DEBUG_STATIC_ASSERT(offsetof(HUF_DEltX2, length) == 3);` | process assertion failure | [x] |
| 1123 | `HUF_buildDEltX2U32` (c_src/src/decompress/huf_decompress.c:967) | `DEBUG_STATIC_ASSERT(sizeof(HUF_DEltX2) == sizeof(U32));` | process assertion failure | [x] |
| 1124 | `HUF_buildDEltX2` (c_src/src/decompress/huf_decompress.c:984) | `DEBUG_STATIC_ASSERT(sizeof(DElt) == sizeof(val));` | process assertion failure | [x] |
| 1125 | `HUF_fillDTableX2ForWeight` (c_src/src/decompress/huf_decompress.c:1018) | `assert(level >= 1 && level <= 2);` | process assertion failure | [x] |
| 1126 | `HUF_fillDTableX2Level2` (c_src/src/decompress/huf_decompress.c:1082) | `assert(length > 1);` | process assertion failure | [x] |
| 1127 | `HUF_fillDTableX2Level2` (c_src/src/decompress/huf_decompress.c:1083) | `assert((U32)skipSize < length);` | process assertion failure | [x] |
| 1128 | `HUF_fillDTableX2Level2` (c_src/src/decompress/huf_decompress.c:1086) | `assert(skipSize == 1);` | process assertion failure | [x] |
| 1129 | `HUF_fillDTableX2Level2` (c_src/src/decompress/huf_decompress.c:1090) | `assert(skipSize <= 4);` | process assertion failure | [x] |
| 1130 | `HUF_readDTableX2_wksp` (c_src/src/decompress/huf_decompress.c:1193) | `if (sizeof(*wksp) > wkspSize) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [x] |
| 1131 | `HUF_readDTableX2_wksp` (c_src/src/decompress/huf_decompress.c:1199) | `DEBUG_STATIC_ASSERT(sizeof(HUF_DEltX2) == sizeof(HUF_DTable)); /* if compiler fails here, assertion is wrong */` | process assertion failure | [x] |
| 1132 | `HUF_readDTableX2_wksp` (c_src/src/decompress/huf_decompress.c:1200) | `if (maxTableLog > HUF_TABLELOG_MAX) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [x] |
| 1133 | `HUF_readDTableX2_wksp` (c_src/src/decompress/huf_decompress.c:1207) | `if (tableLog > maxTableLog) return ERROR(tableLog_tooLarge); /* DTable can't fit code depth */` | `ERROR(tableLog_tooLarge)` | [x] |
| 1134 | `HUF_decompress1X2_usingDTable_internal_body` (c_src/src/decompress/huf_decompress.c:1373) | `if (!BIT_endOfDStream(&bitD)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1135 | `HUF_decompress4X2_usingDTable_internal_body` (c_src/src/decompress/huf_decompress.c:1389) | `if (cSrcSize < 10) return ERROR(corruption_detected); /* strict minimum : jump table + 1 byte per stream */` | `ERROR(corruption_detected)` | [x] |
| 1136 | `HUF_decompress4X2_usingDTable_internal_body` (c_src/src/decompress/huf_decompress.c:1390) | `if (dstSize < 6) return ERROR(corruption_detected); /* stream 4-split doesn't work */` | `ERROR(corruption_detected)` | [x] |
| 1137 | `HUF_decompress4X2_usingDTable_internal_body` (c_src/src/decompress/huf_decompress.c:1424) | `if (length4 > cSrcSize) return ERROR(corruption_detected); /* overflow */` | `ERROR(corruption_detected)` | [x] |
| 1138 | `HUF_decompress4X2_usingDTable_internal_body` (c_src/src/decompress/huf_decompress.c:1425) | `if (opStart4 > oend) return ERROR(corruption_detected); /* overflow */` | `ERROR(corruption_detected)` | [x] |
| 1139 | `HUF_decompress4X2_usingDTable_internal_body` (c_src/src/decompress/huf_decompress.c:1426) | `assert(dstSize >= 6 /* validated above */);` | process assertion failure | [x] |
| 1140 | `HUF_decompress4X2_usingDTable_internal_body` (c_src/src/decompress/huf_decompress.c:1483) | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1141 | `HUF_decompress4X2_usingDTable_internal_body` (c_src/src/decompress/huf_decompress.c:1484) | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1142 | `HUF_decompress4X2_usingDTable_internal_body` (c_src/src/decompress/huf_decompress.c:1485) | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1143 | `HUF_decompress4X2_usingDTable_internal_body` (c_src/src/decompress/huf_decompress.c:1496) | `if (!endCheck) return ERROR(corruption_detected); }` | `ERROR(corruption_detected)` | [x] |
| 1144 | `HUF_decompress4X2_usingDTable_internal_fast_c_loop` (c_src/src/decompress/huf_decompress.c:1543) | `assert(MEM_isLittleEndian());` | process assertion failure | [x] |
| 1145 | `HUF_decompress4X2_usingDTable_internal_fast_c_loop` (c_src/src/decompress/huf_decompress.c:1544) | `assert(!MEM_32bits());` | process assertion failure | [x] |
| 1146 | `HUF_decompress4X2_usingDTable_internal_fast_c_loop` (c_src/src/decompress/huf_decompress.c:1553) | `assert(op[stream] <= oend[stream]);` | process assertion failure | [x] |
| 1147 | `HUF_decompress4X2_usingDTable_internal_fast_c_loop` (c_src/src/decompress/huf_decompress.c:1554) | `assert(ip[stream] >= ilowest);` | process assertion failure | [x] |
| 1148 | `HUF_decompress4X2_usingDTable_internal_fast_c_loop` (c_src/src/decompress/huf_decompress.c:1601) | `assert(ip[stream] >= ip[stream - 1]);` | process assertion failure | [x] |
| 1149 | `HUF_decompress4X2_usingDTable_internal_fast` (c_src/src/decompress/huf_decompress.c:1683) | `assert(args.ip[0] >= args.ilowest);` | process assertion failure | [x] |
| 1150 | `HUF_decompress4X2_usingDTable_internal_fast` (c_src/src/decompress/huf_decompress.c:1687) | `assert(args.ip[0] >= ilowest);` | process assertion failure | [x] |
| 1151 | `HUF_decompress4X2_usingDTable_internal_fast` (c_src/src/decompress/huf_decompress.c:1688) | `assert(args.ip[1] >= ilowest);` | process assertion failure | [x] |
| 1152 | `HUF_decompress4X2_usingDTable_internal_fast` (c_src/src/decompress/huf_decompress.c:1689) | `assert(args.ip[2] >= ilowest);` | process assertion failure | [x] |
| 1153 | `HUF_decompress4X2_usingDTable_internal_fast` (c_src/src/decompress/huf_decompress.c:1690) | `assert(args.ip[3] >= ilowest);` | process assertion failure | [x] |
| 1154 | `HUF_decompress4X2_usingDTable_internal_fast` (c_src/src/decompress/huf_decompress.c:1691) | `assert(args.op[3] <= oend);` | process assertion failure | [x] |
| 1155 | `HUF_decompress4X2_usingDTable_internal_fast` (c_src/src/decompress/huf_decompress.c:1693) | `assert(ilowest == args.ilowest);` | process assertion failure | [x] |
| 1156 | `HUF_decompress4X2_usingDTable_internal_fast` (c_src/src/decompress/huf_decompress.c:1694) | `assert(ilowest + 6 == args.iend[0]);` | process assertion failure | [x] |
| 1157 | `HUF_decompress4X2_usingDTable_internal_fast` (c_src/src/decompress/huf_decompress.c:1711) | `return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1158 | `HUF_DGEN` (c_src/src/decompress/huf_decompress.c:1763) | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1159 | `HUF_decompress4X2_DCtx_wksp` (c_src/src/decompress/huf_decompress.c:1778) | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1160 | `HUF_selectDecoder` (c_src/src/decompress/huf_decompress.c:1823) | `assert(dstSize > 0);` | process assertion failure | [x] |
| 1161 | `HUF_selectDecoder` (c_src/src/decompress/huf_decompress.c:1824) | `assert(dstSize <= 128*1024);` | process assertion failure | [x] |
| 1162 | `HUF_decompress1X_DCtx_wksp` (c_src/src/decompress/huf_decompress.c:1850) | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 1163 | `HUF_decompress1X_DCtx_wksp` (c_src/src/decompress/huf_decompress.c:1851) | `if (cSrcSize > dstSize) return ERROR(corruption_detected); /* invalid */` | `ERROR(corruption_detected)` | [x] |
| 1164 | `HUF_decompress1X_DCtx_wksp` (c_src/src/decompress/huf_decompress.c:1858) | `assert(algoNb == 0);` | process assertion failure | [x] |
| 1165 | `HUF_decompress1X_DCtx_wksp` (c_src/src/decompress/huf_decompress.c:1863) | `assert(algoNb == 1);` | process assertion failure | [x] |
| 1166 | `HUF_decompress1X_usingDTable` (c_src/src/decompress/huf_decompress.c:1881) | `assert(dtd.tableType == 0);` | process assertion failure | [x] |
| 1167 | `HUF_decompress1X_usingDTable` (c_src/src/decompress/huf_decompress.c:1885) | `assert(dtd.tableType == 1);` | process assertion failure | [x] |
| 1168 | `HUF_decompress1X1_DCtx_wksp` (c_src/src/decompress/huf_decompress.c:1900) | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1169 | `HUF_decompress4X_usingDTable` (c_src/src/decompress/huf_decompress.c:1912) | `assert(dtd.tableType == 0);` | process assertion failure | [x] |
| 1170 | `HUF_decompress4X_usingDTable` (c_src/src/decompress/huf_decompress.c:1916) | `assert(dtd.tableType == 1);` | process assertion failure | [x] |
| 1171 | `HUF_decompress4X_hufOnly_wksp` (c_src/src/decompress/huf_decompress.c:1927) | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 1172 | `HUF_decompress4X_hufOnly_wksp` (c_src/src/decompress/huf_decompress.c:1928) | `if (cSrcSize == 0) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1173 | `HUF_decompress4X_hufOnly_wksp` (c_src/src/decompress/huf_decompress.c:1933) | `assert(algoNb == 0);` | process assertion failure | [x] |
| 1174 | `HUF_decompress4X_hufOnly_wksp` (c_src/src/decompress/huf_decompress.c:1937) | `assert(algoNb == 1);` | process assertion failure | [x] |
| 1175 | `ZSTD_DDict_dictContent` (c_src/src/decompress/zstd_ddict.c:48) | `assert(ddict != NULL);` | process assertion failure | [x] |
| 1176 | `ZSTD_DDict_dictSize` (c_src/src/decompress/zstd_ddict.c:54) | `assert(ddict != NULL);` | process assertion failure | [x] |
| 1177 | `ZSTD_copyDDictParameters` (c_src/src/decompress/zstd_ddict.c:61) | `assert(dctx != NULL);` | process assertion failure | [x] |
| 1178 | `ZSTD_copyDDictParameters` (c_src/src/decompress/zstd_ddict.c:62) | `assert(ddict != NULL);` | process assertion failure | [x] |
| 1179 | `ZSTD_loadEntropy_intoDDict` (c_src/src/decompress/zstd_ddict.c:99) | `return ERROR(dictionary_corrupted); /* only accept specified dictionaries */` | `ERROR(dictionary_corrupted)` | [x] |
| 1180 | `ZSTD_loadEntropy_intoDDict` (c_src/src/decompress/zstd_ddict.c:105) | `return ERROR(dictionary_corrupted); /* only accept specified dictionaries */` | `ERROR(dictionary_corrupted)` | [x] |
| 1181 | `ZSTD_loadEntropy_intoDDict` (c_src/src/decompress/zstd_ddict.c:112) | `RETURN_ERROR_IF(ZSTD_isError(ZSTD_loadDEntropy(` | `ERROR(ZSTD_isError)` | [x] |
| 1182 | `ZSTD_initDDict_internal` (c_src/src/decompress/zstd_ddict.c:133) | `if (!internalBuffer) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [x] |
| 1183 | `ZSTD_createDDict_advanced` (c_src/src/decompress/zstd_ddict.c:150) | `if ((!customMem.customAlloc) ^ (!customMem.customFree)) return NULL;` | `NULL` | [x] |
| 1184 | `ZSTD_createDDict_advanced` (c_src/src/decompress/zstd_ddict.c:153) | `if (ddict == NULL) return NULL;` | `NULL` | [x] |
| 1185 | `ZSTD_createDDict_advanced` (c_src/src/decompress/zstd_ddict.c:160) | `return NULL;` | `NULL` | [x] |
| 1186 | `ZSTD_initStaticDDict` (c_src/src/decompress/zstd_ddict.c:196) | `assert(sBuffer != NULL);` | process assertion failure | [x] |
| 1187 | `ZSTD_initStaticDDict` (c_src/src/decompress/zstd_ddict.c:197) | `assert(dict != NULL);` | process assertion failure | [x] |
| 1188 | `ZSTD_initStaticDDict` (c_src/src/decompress/zstd_ddict.c:198) | `if ((size_t)sBuffer & 7) return NULL; /* 8-aligned */` | `NULL` | [x] |
| 1189 | `ZSTD_initStaticDDict` (c_src/src/decompress/zstd_ddict.c:199) | `if (sBufferSize < neededSpace) return NULL;` | `NULL` | [x] |
| 1190 | `ZSTD_initStaticDDict` (c_src/src/decompress/zstd_ddict.c:207) | `return NULL;` | `NULL` | [x] |
| 1191 | `ZSTD_DDictHashSet_emplaceDDict` (c_src/src/decompress/zstd_decompress.c:109) | `RETURN_ERROR_IF(hashSet->ddictPtrCount == hashSet->ddictPtrTableSize, GENERIC, "Hash set is full!");` | `ERROR(hashSet)` | [x] |
| 1192 | `ZSTD_DDictHashSet_expand` (c_src/src/decompress/zstd_decompress.c:139) | `RETURN_ERROR_IF(!newTable, memory_allocation, "Expanded hashset allocation failed!");` | source-declared rejection sentinel | [x] |
| 1193 | `ZSTD_createDDictHashSet` (c_src/src/decompress/zstd_decompress.c:182) | `return NULL;` | `NULL` | [x] |
| 1194 | `ZSTD_createDDictHashSet` (c_src/src/decompress/zstd_decompress.c:186) | `return NULL;` | `NULL` | [x] |
| 1195 | `ZSTD_startingInputLength` (c_src/src/decompress/zstd_decompress.c:236) | `assert( (format == ZSTD_f_zstd1) \|\| (format == ZSTD_f_zstd1_magicless) );` | process assertion failure | [x] |
| 1196 | `ZSTD_DCtx_resetParameters` (c_src/src/decompress/zstd_decompress.c:242) | `assert(dctx->streamStage == zdss_init);` | process assertion failure | [x] |
| 1197 | `ZSTD_initStaticDCtx` (c_src/src/decompress/zstd_decompress.c:285) | `if ((size_t)workspace & 7) return NULL; /* 8-aligned */` | `NULL` | [x] |
| 1198 | `ZSTD_initStaticDCtx` (c_src/src/decompress/zstd_decompress.c:286) | `if (workspaceSize < sizeof(ZSTD_DCtx)) return NULL; /* minimum size */` | `NULL` | [x] |
| 1199 | `ZSTD_createDCtx_internal` (c_src/src/decompress/zstd_decompress.c:295) | `if ((!customMem.customAlloc) ^ (!customMem.customFree)) return NULL;` | `NULL` | [x] |
| 1200 | `ZSTD_createDCtx_internal` (c_src/src/decompress/zstd_decompress.c:298) | `if (!dctx) return NULL;` | `NULL` | [x] |
| 1201 | `ZSTD_freeDCtx` (c_src/src/decompress/zstd_decompress.c:327) | `RETURN_ERROR_IF(dctx->staticSize, memory_allocation, "not compatible with static DCtx");` | `ERROR(dctx)` | [x] |
| 1202 | `ZSTD_DCtx_selectFrameDDict` (c_src/src/decompress/zstd_decompress.c:361) | `assert(dctx->refMultipleDDicts && dctx->ddictSet);` | process assertion failure | [x] |
| 1203 | `ZSTD_frameHeaderSize_internal` (c_src/src/decompress/zstd_decompress.c:419) | `RETURN_ERROR_IF(srcSize < minInputSize, srcSize_wrong, "");` | `ERROR(srcSize)` | [x] |
| 1204 | `ZSTD_getFrameHeader_advanced` (c_src/src/decompress/zstd_decompress.c:455) | `/* note : technically could be considered an assert(), since it's an invalid entry */` | process assertion failure | [x] |
| 1205 | `ZSTD_getFrameHeader_advanced` (c_src/src/decompress/zstd_decompress.c:456) | `RETURN_ERROR_IF(src==NULL, GENERIC, "invalid parameter : src==NULL, but srcSize>0");` | `ERROR(src)` | [x] |
| 1206 | `ZSTD_getFrameHeader_advanced` (c_src/src/decompress/zstd_decompress.c:466) | `assert(src != NULL);` | process assertion failure | [x] |
| 1207 | `ZSTD_getFrameHeader_advanced` (c_src/src/decompress/zstd_decompress.c:473) | `RETURN_ERROR(prefix_unknown,` | `ERROR(prefix_unknown)` | [x] |
| 1208 | `ZSTD_getFrameHeader_advanced` (c_src/src/decompress/zstd_decompress.c:493) | `RETURN_ERROR(prefix_unknown, "");` | `ERROR(prefix_unknown)` | [x] |
| 1209 | `ZSTD_getFrameHeader_advanced` (c_src/src/decompress/zstd_decompress.c:511) | `RETURN_ERROR_IF((fhdByte & 0x08) != 0, frameParameter_unsupported,` | source-declared rejection sentinel | [x] |
| 1210 | `ZSTD_getFrameHeader_advanced` (c_src/src/decompress/zstd_decompress.c:517) | `RETURN_ERROR_IF(windowLog > ZSTD_WINDOWLOG_MAX, frameParameter_windowTooLarge, "");` | `ERROR(windowLog)` | [x] |
| 1211 | `ZSTD_getFrameHeader_advanced` (c_src/src/decompress/zstd_decompress.c:524) | `assert(0); /* impossible */` | process assertion failure | [x] |
| 1212 | `ZSTD_getFrameHeader_advanced` (c_src/src/decompress/zstd_decompress.c:534) | `assert(0); /* impossible */` | process assertion failure | [x] |
| 1213 | `ZSTD_getFrameContentSize` (c_src/src/decompress/zstd_decompress.c:579) | `return ZSTD_CONTENTSIZE_ERROR;` | `ZSTD_CONTENTSIZE_ERROR` | [x] |
| 1214 | `readSkippableFrameSize` (c_src/src/decompress/zstd_decompress.c:592) | `RETURN_ERROR_IF(srcSize < ZSTD_SKIPPABLEHEADERSIZE, srcSize_wrong, "");` | `ERROR(srcSize)` | [x] |
| 1215 | `readSkippableFrameSize` (c_src/src/decompress/zstd_decompress.c:595) | `RETURN_ERROR_IF((U32)(sizeU32 + ZSTD_SKIPPABLEHEADERSIZE) < sizeU32,` | source-declared rejection sentinel | [x] |
| 1216 | `readSkippableFrameSize` (c_src/src/decompress/zstd_decompress.c:598) | `RETURN_ERROR_IF(skippableSize > srcSize, srcSize_wrong, "");` | `ERROR(skippableSize)` | [x] |
| 1217 | `ZSTD_readSkippableFrame` (c_src/src/decompress/zstd_decompress.c:618) | `RETURN_ERROR_IF(srcSize < ZSTD_SKIPPABLEHEADERSIZE, srcSize_wrong, "");` | `ERROR(srcSize)` | [x] |
| 1218 | `ZSTD_readSkippableFrame` (c_src/src/decompress/zstd_decompress.c:625) | `RETURN_ERROR_IF(!ZSTD_isSkippableFrame(src, srcSize), frameParameter_unsupported, "");` | source-declared rejection sentinel | [x] |
| 1219 | `ZSTD_readSkippableFrame` (c_src/src/decompress/zstd_decompress.c:626) | `RETURN_ERROR_IF(skippableFrameSize < ZSTD_SKIPPABLEHEADERSIZE \|\| skippableFrameSize > srcSize, srcSize_wrong, "");` | `ERROR(skippableFrameSize)` | [x] |
| 1220 | `ZSTD_readSkippableFrame` (c_src/src/decompress/zstd_decompress.c:627) | `RETURN_ERROR_IF(skippableContentSize > dstCapacity, dstSize_tooSmall, "");` | `ERROR(skippableContentSize)` | [x] |
| 1221 | `ZSTD_findDecompressedSize` (c_src/src/decompress/zstd_decompress.c:652) | `if (ZSTD_isError(skippableSize)) return ZSTD_CONTENTSIZE_ERROR;` | `ZSTD_CONTENTSIZE_ERROR` | [x] |
| 1222 | `ZSTD_findDecompressedSize` (c_src/src/decompress/zstd_decompress.c:653) | `assert(skippableSize <= srcSize);` | process assertion failure | [x] |
| 1223 | `ZSTD_findDecompressedSize` (c_src/src/decompress/zstd_decompress.c:664) | `return ZSTD_CONTENTSIZE_ERROR; /* check for overflow */` | `ZSTD_CONTENTSIZE_ERROR` | [x] |
| 1224 | `ZSTD_findDecompressedSize` (c_src/src/decompress/zstd_decompress.c:669) | `if (ZSTD_isError(frameSrcSize)) return ZSTD_CONTENTSIZE_ERROR;` | `ZSTD_CONTENTSIZE_ERROR` | [x] |
| 1225 | `ZSTD_findDecompressedSize` (c_src/src/decompress/zstd_decompress.c:670) | `assert(frameSrcSize <= srcSize);` | process assertion failure | [x] |
| 1226 | `ZSTD_findDecompressedSize` (c_src/src/decompress/zstd_decompress.c:677) | `if (srcSize) return ZSTD_CONTENTSIZE_ERROR;` | `ZSTD_CONTENTSIZE_ERROR` | [x] |
| 1227 | `ZSTD_getDecompressedSize` (c_src/src/decompress/zstd_decompress.c:693) | `ZSTD_STATIC_ASSERT(ZSTD_CONTENTSIZE_ERROR < ZSTD_CONTENTSIZE_UNKNOWN);` | process assertion failure | [x] |
| 1228 | `ZSTD_getDecompressedSize` (c_src/src/decompress/zstd_decompress.c:694) | `return (ret >= ZSTD_CONTENTSIZE_ERROR) ? 0 : ret;` | source-declared rejection sentinel | [x] |
| 1229 | `ZSTD_decodeFrameHeader` (c_src/src/decompress/zstd_decompress.c:706) | `RETURN_ERROR_IF(result>0, srcSize_wrong, "headerSize too small");` | `ERROR(result)` | [x] |
| 1230 | `ZSTD_decodeFrameHeader` (c_src/src/decompress/zstd_decompress.c:717) | `RETURN_ERROR_IF(dctx->fParams.dictID && (dctx->dictID != dctx->fParams.dictID),` | `ERROR(dctx)` | [x] |
| 1231 | `ZSTD_findFrameSizeInfo` (c_src/src/decompress/zstd_decompress.c:747) | `assert(ZSTD_isError(frameSizeInfo.compressedSize) \|\|` | process assertion failure | [x] |
| 1232 | `ZSTD_findFrameSizeInfo` (c_src/src/decompress/zstd_decompress.c:762) | `return ZSTD_errorFrameSizeInfo(ERROR(srcSize_wrong));` | `ERROR(srcSize_wrong)` | [x] |
| 1233 | `ZSTD_findFrameSizeInfo` (c_src/src/decompress/zstd_decompress.c:776) | `return ZSTD_errorFrameSizeInfo(ERROR(srcSize_wrong));` | `ERROR(srcSize_wrong)` | [x] |
| 1234 | `ZSTD_findFrameSizeInfo` (c_src/src/decompress/zstd_decompress.c:788) | `return ZSTD_errorFrameSizeInfo(ERROR(srcSize_wrong));` | `ERROR(srcSize_wrong)` | [x] |
| 1235 | `ZSTD_decompressBound` (c_src/src/decompress/zstd_decompress.c:829) | `return ZSTD_CONTENTSIZE_ERROR;` | `ZSTD_CONTENTSIZE_ERROR` | [x] |
| 1236 | `ZSTD_decompressBound` (c_src/src/decompress/zstd_decompress.c:830) | `assert(srcSize >= compressedSize);` | process assertion failure | [x] |
| 1237 | `ZSTD_decompressionMargin` (c_src/src/decompress/zstd_decompress.c:852) | `return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1238 | `ZSTD_decompressionMargin` (c_src/src/decompress/zstd_decompress.c:865) | `assert(zfh.frameType == ZSTD_skippableFrame);` | process assertion failure | [x] |
| 1239 | `ZSTD_decompressionMargin` (c_src/src/decompress/zstd_decompress.c:870) | `assert(srcSize >= compressedSize);` | process assertion failure | [x] |
| 1240 | `ZSTD_copyRawBlock` (c_src/src/decompress/zstd_decompress.c:900) | `RETURN_ERROR_IF(srcSize > dstCapacity, dstSize_tooSmall, "");` | `ERROR(srcSize)` | [x] |
| 1241 | `ZSTD_copyRawBlock` (c_src/src/decompress/zstd_decompress.c:903) | `RETURN_ERROR(dstBuffer_null, "");` | `ERROR(dstBuffer_null)` | [x] |
| 1242 | `ZSTD_setRleBlock` (c_src/src/decompress/zstd_decompress.c:913) | `RETURN_ERROR_IF(regenSize > dstCapacity, dstSize_tooSmall, "");` | `ERROR(regenSize)` | [x] |
| 1243 | `ZSTD_setRleBlock` (c_src/src/decompress/zstd_decompress.c:916) | `RETURN_ERROR(dstBuffer_null, "");` | `ERROR(dstBuffer_null)` | [x] |
| 1244 | `ZSTD_decompressFrame` (c_src/src/decompress/zstd_decompress.c:967) | `RETURN_ERROR_IF(` | source-declared rejection sentinel | [x] |
| 1245 | `ZSTD_decompressFrame` (c_src/src/decompress/zstd_decompress.c:975) | `RETURN_ERROR_IF(remainingSrcSize < frameHeaderSize+ZSTD_blockHeaderSize,` | `ERROR(remainingSrcSize)` | [x] |
| 1246 | `ZSTD_decompressFrame` (c_src/src/decompress/zstd_decompress.c:995) | `RETURN_ERROR_IF(cBlockSize > remainingSrcSize, srcSize_wrong, "");` | `ERROR(cBlockSize)` | [x] |
| 1247 | `ZSTD_decompressFrame` (c_src/src/decompress/zstd_decompress.c:1017) | `assert(dctx->isFrameDecompression == 1);` | process assertion failure | [x] |
| 1248 | `ZSTD_decompressFrame` (c_src/src/decompress/zstd_decompress.c:1029) | `RETURN_ERROR(corruption_detected, "invalid block type");` | `ERROR(corruption_detected)` | [x] |
| 1249 | `ZSTD_decompressFrame` (c_src/src/decompress/zstd_decompress.c:1039) | `assert(ip != NULL);` | process assertion failure | [x] |
| 1250 | `ZSTD_decompressFrame` (c_src/src/decompress/zstd_decompress.c:1046) | `RETURN_ERROR_IF((U64)(op-ostart) != dctx->fParams.frameContentSize,` | source-declared rejection sentinel | [x] |
| 1251 | `ZSTD_decompressFrame` (c_src/src/decompress/zstd_decompress.c:1050) | `RETURN_ERROR_IF(remainingSrcSize<4, checksum_wrong, "");` | `ERROR(remainingSrcSize)` | [x] |
| 1252 | `ZSTD_decompressFrame` (c_src/src/decompress/zstd_decompress.c:1055) | `RETURN_ERROR_IF(checkRead != checkCalc, checksum_wrong, "");` | `ERROR(checkRead)` | [x] |
| 1253 | `ZSTD_decompressMultiFrame` (c_src/src/decompress/zstd_decompress.c:1080) | `assert(dict==NULL \|\| ddict==NULL); /* either dict or ddict set, not both */` | process assertion failure | [x] |
| 1254 | `ZSTD_decompressMultiFrame` (c_src/src/decompress/zstd_decompress.c:1094) | `RETURN_ERROR_IF(dctx->staticSize, memory_allocation,` | `ERROR(dctx)` | [x] |
| 1255 | `ZSTD_decompressMultiFrame` (c_src/src/decompress/zstd_decompress.c:1102) | `RETURN_ERROR_IF(expectedSize == ZSTD_CONTENTSIZE_ERROR, corruption_detected, "Corrupted frame header!");` | `ERROR(expectedSize)` | [x] |
| 1256 | `ZSTD_decompressMultiFrame` (c_src/src/decompress/zstd_decompress.c:1104) | `RETURN_ERROR_IF(expectedSize != decodedSize, corruption_detected,` | `ERROR(expectedSize)` | [x] |
| 1257 | `ZSTD_decompressMultiFrame` (c_src/src/decompress/zstd_decompress.c:1109) | `assert(decodedSize <= dstCapacity);` | process assertion failure | [x] |
| 1258 | `ZSTD_decompressMultiFrame` (c_src/src/decompress/zstd_decompress.c:1127) | `assert(skippableSize <= srcSize);` | process assertion failure | [x] |
| 1259 | `ZSTD_decompressMultiFrame` (c_src/src/decompress/zstd_decompress.c:1146) | `RETURN_ERROR_IF(` | source-declared rejection sentinel | [x] |
| 1260 | `ZSTD_decompressMultiFrame` (c_src/src/decompress/zstd_decompress.c:1158) | `assert(res <= dstCapacity);` | process assertion failure | [x] |
| 1261 | `ZSTD_decompressMultiFrame` (c_src/src/decompress/zstd_decompress.c:1166) | `RETURN_ERROR_IF(srcSize, srcSize_wrong, "input not entirely consumed");` | `ERROR(srcSize)` | [x] |
| 1262 | `ZSTD_decompress_usingDict` (c_src/src/decompress/zstd_decompress.c:1176) | `return ZSTD_decompressMultiFrame(dctx, dst, dstCapacity, src, srcSize, dict, dictSize, NULL);` | source-declared rejection sentinel | [x] |
| 1263 | `ZSTD_getDDict` (c_src/src/decompress/zstd_decompress.c:1184) | `assert(0 /* Impossible */);` | process assertion failure | [x] |
| 1264 | `ZSTD_getDDict` (c_src/src/decompress/zstd_decompress.c:1188) | `return NULL;` | `NULL` | [x] |
| 1265 | `ZSTD_decompress` (c_src/src/decompress/zstd_decompress.c:1208) | `RETURN_ERROR_IF(dctx==NULL, memory_allocation, "NULL pointer!");` | `ERROR(dctx)` | [x] |
| 1266 | `ZSTD_nextInputType` (c_src/src/decompress/zstd_decompress.c:1248) | `assert(0);` | process assertion failure | [x] |
| 1267 | `ZSTD_decompressContinue` (c_src/src/decompress/zstd_decompress.c:1279) | `RETURN_ERROR_IF(srcSize != ZSTD_nextSrcSizeToDecompressWithInputSize(dctx, srcSize), srcSize_wrong, "not allowed");` | `ERROR(srcSize)` | [x] |
| 1268 | `ZSTD_decompressContinue` (c_src/src/decompress/zstd_decompress.c:1287) | `assert(src != NULL);` | process assertion failure | [x] |
| 1269 | `ZSTD_decompressContinue` (c_src/src/decompress/zstd_decompress.c:1289) | `assert(srcSize >= ZSTD_FRAMEIDSIZE); /* to read skippable magic number */` | process assertion failure | [x] |
| 1270 | `ZSTD_decompressContinue` (c_src/src/decompress/zstd_decompress.c:1304) | `assert(src != NULL);` | process assertion failure | [x] |
| 1271 | `ZSTD_decompressContinue` (c_src/src/decompress/zstd_decompress.c:1315) | `RETURN_ERROR_IF(cBlockSize > dctx->fParams.blockSizeMax, corruption_detected, "Block Size Exceeds Maximum");` | `ERROR(cBlockSize)` | [x] |
| 1272 | `ZSTD_decompressContinue` (c_src/src/decompress/zstd_decompress.c:1347) | `assert(dctx->isFrameDecompression == 1);` | process assertion failure | [x] |
| 1273 | `ZSTD_decompressContinue` (c_src/src/decompress/zstd_decompress.c:1352) | `assert(srcSize <= dctx->expected);` | process assertion failure | [x] |
| 1274 | `ZSTD_decompressContinue` (c_src/src/decompress/zstd_decompress.c:1355) | `assert(rSize == srcSize);` | process assertion failure | [x] |
| 1275 | `ZSTD_decompressContinue` (c_src/src/decompress/zstd_decompress.c:1364) | `RETURN_ERROR(corruption_detected, "invalid block type");` | `ERROR(corruption_detected)` | [x] |
| 1276 | `ZSTD_decompressContinue` (c_src/src/decompress/zstd_decompress.c:1367) | `RETURN_ERROR_IF(rSize > dctx->fParams.blockSizeMax, corruption_detected, "Decompressed Block Size Exceeds Maximum");` | `ERROR(rSize)` | [x] |
| 1277 | `ZSTD_decompressContinue` (c_src/src/decompress/zstd_decompress.c:1380) | `RETURN_ERROR_IF(` | source-declared rejection sentinel | [x] |
| 1278 | `ZSTD_decompressContinue` (c_src/src/decompress/zstd_decompress.c:1400) | `assert(srcSize == 4); /* guaranteed by dctx->expected */` | process assertion failure | [x] |
| 1279 | `ZSTD_decompressContinue` (c_src/src/decompress/zstd_decompress.c:1406) | `RETURN_ERROR_IF(check32 != h32, checksum_wrong, "");` | `ERROR(check32)` | [x] |
| 1280 | `ZSTD_decompressContinue` (c_src/src/decompress/zstd_decompress.c:1415) | `assert(src != NULL);` | process assertion failure | [x] |
| 1281 | `ZSTD_decompressContinue` (c_src/src/decompress/zstd_decompress.c:1416) | `assert(srcSize <= ZSTD_SKIPPABLEHEADERSIZE);` | process assertion failure | [x] |
| 1282 | `ZSTD_decompressContinue` (c_src/src/decompress/zstd_decompress.c:1417) | `assert(dctx->format != ZSTD_f_zstd1_magicless);` | process assertion failure | [x] |
| 1283 | `ZSTD_decompressContinue` (c_src/src/decompress/zstd_decompress.c:1429) | `assert(0); /* impossible */` | process assertion failure | [x] |
| 1284 | `ZSTD_decompressContinue` (c_src/src/decompress/zstd_decompress.c:1430) | `RETURN_ERROR(GENERIC, "impossible to reach"); /* some compilers require default to do something */` | `ERROR(GENERIC)` | [x] |
| 1285 | `ZSTD_loadDEntropy` (c_src/src/decompress/zstd_decompress.c:1458) | `RETURN_ERROR_IF(dictSize <= 8, dictionary_corrupted, "dict is too small");` | `ERROR(dictSize)` | [x] |
| 1286 | `ZSTD_loadDEntropy` (c_src/src/decompress/zstd_decompress.c:1459) | `assert(MEM_readLE32(dict) == ZSTD_MAGIC_DICTIONARY); /* dict must be valid */` | process assertion failure | [x] |
| 1287 | `ZSTD_loadDEntropy` (c_src/src/decompress/zstd_decompress.c:1462) | `ZSTD_STATIC_ASSERT(offsetof(ZSTD_entropyDTables_t, OFTable) == offsetof(ZSTD_entropyDTables_t, LLTable) + sizeof(entropy->LLTable));` | process assertion failure | [x] |
| 1288 | `ZSTD_loadDEntropy` (c_src/src/decompress/zstd_decompress.c:1463) | `ZSTD_STATIC_ASSERT(offsetof(ZSTD_entropyDTables_t, MLTable) == offsetof(ZSTD_entropyDTables_t, OFTable) + sizeof(entropy->OFTable));` | process assertion failure | [x] |
| 1289 | `ZSTD_loadDEntropy` (c_src/src/decompress/zstd_decompress.c:1464) | `ZSTD_STATIC_ASSERT(sizeof(entropy->LLTable) + sizeof(entropy->OFTable) + sizeof(entropy->MLTable) >= HUF_DECOMPRESS_WORKSPACE_SIZE);` | process assertion failure | [x] |
| 1290 | `ZSTD_loadDEntropy` (c_src/src/decompress/zstd_decompress.c:1477) | `RETURN_ERROR_IF(HUF_isError(hSize), dictionary_corrupted, "");` | `ERROR(HUF_isError)` | [x] |
| 1291 | `ZSTD_loadDEntropy` (c_src/src/decompress/zstd_decompress.c:1484) | `RETURN_ERROR_IF(FSE_isError(offcodeHeaderSize), dictionary_corrupted, "");` | `ERROR(FSE_isError)` | [x] |
| 1292 | `ZSTD_loadDEntropy` (c_src/src/decompress/zstd_decompress.c:1485) | `RETURN_ERROR_IF(offcodeMaxValue > MaxOff, dictionary_corrupted, "");` | `ERROR(offcodeMaxValue)` | [x] |
| 1293 | `ZSTD_loadDEntropy` (c_src/src/decompress/zstd_decompress.c:1486) | `RETURN_ERROR_IF(offcodeLog > OffFSELog, dictionary_corrupted, "");` | `ERROR(offcodeLog)` | [x] |
| 1294 | `ZSTD_loadDEntropy` (c_src/src/decompress/zstd_decompress.c:1499) | `RETURN_ERROR_IF(FSE_isError(matchlengthHeaderSize), dictionary_corrupted, "");` | `ERROR(FSE_isError)` | [x] |
| 1295 | `ZSTD_loadDEntropy` (c_src/src/decompress/zstd_decompress.c:1500) | `RETURN_ERROR_IF(matchlengthMaxValue > MaxML, dictionary_corrupted, "");` | `ERROR(matchlengthMaxValue)` | [x] |
| 1296 | `ZSTD_loadDEntropy` (c_src/src/decompress/zstd_decompress.c:1501) | `RETURN_ERROR_IF(matchlengthLog > MLFSELog, dictionary_corrupted, "");` | `ERROR(matchlengthLog)` | [x] |
| 1297 | `ZSTD_loadDEntropy` (c_src/src/decompress/zstd_decompress.c:1514) | `RETURN_ERROR_IF(FSE_isError(litlengthHeaderSize), dictionary_corrupted, "");` | `ERROR(FSE_isError)` | [x] |
| 1298 | `ZSTD_loadDEntropy` (c_src/src/decompress/zstd_decompress.c:1515) | `RETURN_ERROR_IF(litlengthMaxValue > MaxLL, dictionary_corrupted, "");` | `ERROR(litlengthMaxValue)` | [x] |
| 1299 | `ZSTD_loadDEntropy` (c_src/src/decompress/zstd_decompress.c:1516) | `RETURN_ERROR_IF(litlengthLog > LLFSELog, dictionary_corrupted, "");` | `ERROR(litlengthLog)` | [x] |
| 1300 | `ZSTD_loadDEntropy` (c_src/src/decompress/zstd_decompress.c:1526) | `RETURN_ERROR_IF(dictPtr+12 > dictEnd, dictionary_corrupted, "");` | `ERROR(dictPtr)` | [x] |
| 1301 | `ZSTD_loadDEntropy` (c_src/src/decompress/zstd_decompress.c:1531) | `RETURN_ERROR_IF(rep==0 \|\| rep > dictContentSize,` | `ERROR(rep)` | [x] |
| 1302 | `ZSTD_decompress_insertDictionary` (c_src/src/decompress/zstd_decompress.c:1550) | `RETURN_ERROR_IF(ZSTD_isError(eSize), dictionary_corrupted, "");` | `ERROR(ZSTD_isError)` | [x] |
| 1303 | `ZSTD_decompressBegin` (c_src/src/decompress/zstd_decompress.c:1562) | `assert(dctx != NULL);` | process assertion failure | [x] |
| 1304 | `ZSTD_decompressBegin` (c_src/src/decompress/zstd_decompress.c:1579) | `ZSTD_STATIC_ASSERT(sizeof(dctx->entropy.rep) == sizeof(repStartValue));` | process assertion failure | [x] |
| 1305 | `ZSTD_decompressBegin_usingDict` (c_src/src/decompress/zstd_decompress.c:1592) | `RETURN_ERROR_IF(` | source-declared rejection sentinel | [x] |
| 1306 | `ZSTD_decompressBegin_usingDDict` (c_src/src/decompress/zstd_decompress.c:1604) | `assert(dctx != NULL);` | process assertion failure | [x] |
| 1307 | `ZSTD_DCtx_loadDictionary_advanced` (c_src/src/decompress/zstd_decompress.c:1704) | `RETURN_ERROR_IF(dctx->streamStage != zdss_init, stage_wrong, "");` | `ERROR(dctx)` | [x] |
| 1308 | `ZSTD_DCtx_loadDictionary_advanced` (c_src/src/decompress/zstd_decompress.c:1708) | `RETURN_ERROR_IF(dctx->ddictLocal == NULL, memory_allocation, "NULL pointer!");` | `ERROR(dctx)` | [x] |
| 1309 | `ZSTD_DCtx_refDDict` (c_src/src/decompress/zstd_decompress.c:1782) | `RETURN_ERROR_IF(dctx->streamStage != zdss_init, stage_wrong, "");` | `ERROR(dctx)` | [x] |
| 1310 | `ZSTD_DCtx_refDDict` (c_src/src/decompress/zstd_decompress.c:1791) | `RETURN_ERROR(memory_allocation, "Failed to allocate memory for hash set!");` | `ERROR(memory_allocation)` | [x] |
| 1311 | `ZSTD_DCtx_refDDict` (c_src/src/decompress/zstd_decompress.c:1794) | `assert(!dctx->staticSize); /* Impossible: ddictSet cannot have been allocated if static dctx */` | process assertion failure | [x] |
| 1312 | `ZSTD_DCtx_setMaxWindowSize` (c_src/src/decompress/zstd_decompress.c:1809) | `RETURN_ERROR_IF(dctx->streamStage != zdss_init, stage_wrong, "");` | `ERROR(dctx)` | [x] |
| 1313 | `ZSTD_DCtx_setMaxWindowSize` (c_src/src/decompress/zstd_decompress.c:1810) | `RETURN_ERROR_IF(maxWindowSize < min, parameter_outOfBound, "");` | `ERROR(maxWindowSize)` | [x] |
| 1314 | `ZSTD_DCtx_setMaxWindowSize` (c_src/src/decompress/zstd_decompress.c:1811) | `RETURN_ERROR_IF(maxWindowSize > max, parameter_outOfBound, "");` | `ERROR(maxWindowSize)` | [x] |
| 1315 | `ZSTD_dParam_getBounds` (c_src/src/decompress/zstd_decompress.c:1832) | `ZSTD_STATIC_ASSERT(ZSTD_f_zstd1 < ZSTD_f_zstd1_magicless);` | process assertion failure | [x] |
| 1316 | `ZSTD_dParam_withinBounds` (c_src/src/decompress/zstd_decompress.c:1874) | `RETURN_ERROR_IF(!ZSTD_dParam_withinBounds(p, v), parameter_outOfBound, ""); \` | source-declared rejection sentinel | [x] |
| 1317 | `ZSTD_DCtx_getParameter` (c_src/src/decompress/zstd_decompress.c:1903) | `RETURN_ERROR(parameter_unsupported, "");` | `ERROR(parameter_unsupported)` | [x] |
| 1318 | `ZSTD_DCtx_setParameter` (c_src/src/decompress/zstd_decompress.c:1908) | `RETURN_ERROR_IF(dctx->streamStage != zdss_init, stage_wrong, "");` | `ERROR(dctx)` | [x] |
| 1319 | `ZSTD_DCtx_setParameter` (c_src/src/decompress/zstd_decompress.c:1930) | `RETURN_ERROR(parameter_unsupported, "Static dctx does not support multiple DDicts!");` | `ERROR(parameter_unsupported)` | [x] |
| 1320 | `ZSTD_DCtx_setParameter` (c_src/src/decompress/zstd_decompress.c:1944) | `RETURN_ERROR(parameter_unsupported, "");` | `ERROR(parameter_unsupported)` | [x] |
| 1321 | `ZSTD_DCtx_reset` (c_src/src/decompress/zstd_decompress.c:1957) | `RETURN_ERROR_IF(dctx->streamStage != zdss_init, stage_wrong, "");` | `ERROR(dctx)` | [x] |
| 1322 | `ZSTD_decodingBufferSize_internal` (c_src/src/decompress/zstd_decompress.c:1983) | `RETURN_ERROR_IF((unsigned long long)minRBSize != neededSize,` | source-declared rejection sentinel | [x] |
| 1323 | `ZSTD_estimateDStreamSize_fromFrame` (c_src/src/decompress/zstd_decompress.c:2007) | `RETURN_ERROR_IF(err>0, srcSize_wrong, "");` | `ERROR(err)` | [x] |
| 1324 | `ZSTD_estimateDStreamSize_fromFrame` (c_src/src/decompress/zstd_decompress.c:2008) | `RETURN_ERROR_IF(zfh.windowSize > windowSizeMax,` | `ERROR(zfh)` | [x] |
| 1325 | `ZSTD_checkOutBuffer` (c_src/src/decompress/zstd_decompress.c:2049) | `RETURN_ERROR(dstBuffer_wrong, "ZSTD_d_stableOutBuffer enabled but output differs!");` | `ERROR(dstBuffer_wrong)` | [x] |
| 1326 | `ZSTD_decompressContinueStream` (c_src/src/decompress/zstd_decompress.c:2080) | `assert(*op <= oend);` | process assertion failure | [x] |
| 1327 | `ZSTD_decompressContinueStream` (c_src/src/decompress/zstd_decompress.c:2081) | `assert(zds->outBufferMode == ZSTD_bm_stable);` | process assertion failure | [x] |
| 1328 | `ZSTD_decompressStream` (c_src/src/decompress/zstd_decompress.c:2099) | `assert(zds != NULL);` | process assertion failure | [x] |
| 1329 | `ZSTD_decompressStream` (c_src/src/decompress/zstd_decompress.c:2100) | `RETURN_ERROR_IF(` | source-declared rejection sentinel | [x] |
| 1330 | `ZSTD_decompressStream` (c_src/src/decompress/zstd_decompress.c:2105) | `RETURN_ERROR_IF(` | source-declared rejection sentinel | [x] |
| 1331 | `ZSTD_decompressStream` (c_src/src/decompress/zstd_decompress.c:2131) | `RETURN_ERROR_IF(zds->staticSize, memory_allocation,` | `ERROR(zds)` | [x] |
| 1332 | `ZSTD_decompressStream` (c_src/src/decompress/zstd_decompress.c:2150) | `RETURN_ERROR_IF(zds->staticSize, memory_allocation,` | `ERROR(zds)` | [x] |
| 1333 | `ZSTD_decompressStream` (c_src/src/decompress/zstd_decompress.c:2166) | `assert(iend >= ip);` | process assertion failure | [x] |
| 1334 | `ZSTD_decompressStream` (c_src/src/decompress/zstd_decompress.c:2180) | `assert(ip != NULL);` | process assertion failure | [x] |
| 1335 | `ZSTD_decompressStream` (c_src/src/decompress/zstd_decompress.c:2195) | `assert(istart != NULL);` | process assertion failure | [x] |
| 1336 | `ZSTD_decompressStream` (c_src/src/decompress/zstd_decompress.c:2209) | `RETURN_ERROR(dstSize_tooSmall, "ZSTD_obm_stable passed but ZSTD_outBuffer is too small");` | `ERROR(dstSize_tooSmall)` | [x] |
| 1337 | `ZSTD_decompressStream` (c_src/src/decompress/zstd_decompress.c:2231) | `RETURN_ERROR_IF(zds->fParams.windowSize > zds->maxWindowSize,` | `ERROR(zds)` | [x] |
| 1338 | `ZSTD_decompressStream` (c_src/src/decompress/zstd_decompress.c:2255) | `assert(zds->staticSize >= sizeof(ZSTD_DCtx)); /* controlled at init */` | process assertion failure | [x] |
| 1339 | `ZSTD_decompressStream` (c_src/src/decompress/zstd_decompress.c:2256) | `RETURN_ERROR_IF(` | source-declared rejection sentinel | [x] |
| 1340 | `ZSTD_decompressStream` (c_src/src/decompress/zstd_decompress.c:2264) | `RETURN_ERROR_IF(zds->inBuff == NULL, memory_allocation, "");` | `ERROR(zds)` | [x] |
| 1341 | `ZSTD_decompressStream` (c_src/src/decompress/zstd_decompress.c:2284) | `assert(ip != NULL);` | process assertion failure | [x] |
| 1342 | `ZSTD_decompressStream` (c_src/src/decompress/zstd_decompress.c:2299) | `assert(neededInSize == ZSTD_nextSrcSizeToDecompressWithInputSize(zds, (size_t)(iend - ip)));` | process assertion failure | [x] |
| 1343 | `ZSTD_decompressStream` (c_src/src/decompress/zstd_decompress.c:2303) | `RETURN_ERROR_IF(toLoad > zds->inBuffSize - zds->inPos,` | `ERROR(toLoad)` | [x] |
| 1344 | `ZSTD_decompressStream` (c_src/src/decompress/zstd_decompress.c:2345) | `assert(0); /* impossible */` | process assertion failure | [x] |
| 1345 | `ZSTD_decompressStream` (c_src/src/decompress/zstd_decompress.c:2346) | `RETURN_ERROR(GENERIC, "impossible to reach"); /* some compilers require default to do something */` | `ERROR(GENERIC)` | [x] |
| 1346 | `ZSTD_decompressStream` (c_src/src/decompress/zstd_decompress.c:2359) | `RETURN_ERROR_IF(op==oend, noForwardProgress_destFull, "");` | `ERROR(op)` | [x] |
| 1347 | `ZSTD_decompressStream` (c_src/src/decompress/zstd_decompress.c:2360) | `RETURN_ERROR_IF(ip==iend, noForwardProgress_inputEmpty, "");` | `ERROR(ip)` | [x] |
| 1348 | `ZSTD_decompressStream` (c_src/src/decompress/zstd_decompress.c:2361) | `assert(0);` | process assertion failure | [x] |
| 1349 | `ZSTD_decompressStream` (c_src/src/decompress/zstd_decompress.c:2386) | `assert(zds->inPos <= nextSrcSizeHint);` | process assertion failure | [x] |
| 1350 | `ZSTD_blockSizeMax` (c_src/src/decompress/zstd_decompress_block.c:57) | `assert(blockSizeMax <= ZSTD_BLOCKSIZE_MAX);` | process assertion failure | [x] |
| 1351 | `ZSTD_getcBlockSize` (c_src/src/decompress/zstd_decompress_block.c:66) | `RETURN_ERROR_IF(srcSize < ZSTD_blockHeaderSize, srcSize_wrong, "");` | `ERROR(srcSize)` | [x] |
| 1352 | `ZSTD_getcBlockSize` (c_src/src/decompress/zstd_decompress_block.c:74) | `RETURN_ERROR_IF(bpPtr->blockType == bt_reserved, corruption_detected, "");` | `ERROR(bpPtr)` | [x] |
| 1353 | `ZSTD_allocateLiteralsBuffer` (c_src/src/decompress/zstd_decompress_block.c:84) | `assert(litSize <= blockSizeMax);` | process assertion failure | [x] |
| 1354 | `ZSTD_allocateLiteralsBuffer` (c_src/src/decompress/zstd_decompress_block.c:85) | `assert(dctx->isFrameDecompression \|\| streaming == not_streaming);` | process assertion failure | [x] |
| 1355 | `ZSTD_allocateLiteralsBuffer` (c_src/src/decompress/zstd_decompress_block.c:86) | `assert(expectedWriteSize <= blockSizeMax);` | process assertion failure | [x] |
| 1356 | `ZSTD_allocateLiteralsBuffer` (c_src/src/decompress/zstd_decompress_block.c:104) | `assert(blockSizeMax > ZSTD_LITBUFFEREXTRASIZE);` | process assertion failure | [x] |
| 1357 | `ZSTD_allocateLiteralsBuffer` (c_src/src/decompress/zstd_decompress_block.c:122) | `assert(dctx->litBufferEnd <= (BYTE*)dst + expectedWriteSize);` | process assertion failure | [x] |
| 1358 | `ZSTD_decodeLiteralsBlock` (c_src/src/decompress/zstd_decompress_block.c:139) | `RETURN_ERROR_IF(srcSize < MIN_CBLOCK_SIZE, corruption_detected, "");` | `ERROR(srcSize)` | [x] |
| 1359 | `ZSTD_decodeLiteralsBlock` (c_src/src/decompress/zstd_decompress_block.c:149) | `RETURN_ERROR_IF(dctx->litEntropy==0, dictionary_corrupted, "");` | `ERROR(dctx)` | [x] |
| 1360 | `ZSTD_decodeLiteralsBlock` (c_src/src/decompress/zstd_decompress_block.c:153) | `RETURN_ERROR_IF(srcSize < 5, corruption_detected, "srcSize >= MIN_CBLOCK_SIZE == 2; here we need up to 5 for case 3");` | `ERROR(srcSize)` | [x] |
| 1361 | `ZSTD_decodeLiteralsBlock` (c_src/src/decompress/zstd_decompress_block.c:185) | `RETURN_ERROR_IF(litSize > 0 && dst == NULL, dstSize_tooSmall, "NULL not handled");` | `ERROR(litSize)` | [x] |
| 1362 | `ZSTD_decodeLiteralsBlock` (c_src/src/decompress/zstd_decompress_block.c:186) | `RETURN_ERROR_IF(litSize > blockSizeMax, corruption_detected, "");` | `ERROR(litSize)` | [x] |
| 1363 | `ZSTD_decodeLiteralsBlock` (c_src/src/decompress/zstd_decompress_block.c:188) | `RETURN_ERROR_IF(litSize < MIN_LITERALS_FOR_4_STREAMS, literals_headerWrong,` | `ERROR(litSize)` | [x] |
| 1364 | `ZSTD_decodeLiteralsBlock` (c_src/src/decompress/zstd_decompress_block.c:191) | `RETURN_ERROR_IF(litCSize + lhSize > srcSize, corruption_detected, "");` | `ERROR(litCSize)` | [x] |
| 1365 | `ZSTD_decodeLiteralsBlock` (c_src/src/decompress/zstd_decompress_block.c:192) | `RETURN_ERROR_IF(expectedWriteSize < litSize , dstSize_tooSmall, "");` | `ERROR(expectedWriteSize)` | [x] |
| 1366 | `ZSTD_decodeLiteralsBlock` (c_src/src/decompress/zstd_decompress_block.c:206) | `assert(litSize >= MIN_LITERALS_FOR_4_STREAMS);` | process assertion failure | [x] |
| 1367 | `ZSTD_decodeLiteralsBlock` (c_src/src/decompress/zstd_decompress_block.c:233) | `assert(litSize > ZSTD_LITBUFFEREXTRASIZE);` | process assertion failure | [x] |
| 1368 | `ZSTD_decodeLiteralsBlock` (c_src/src/decompress/zstd_decompress_block.c:238) | `assert(dctx->litBufferEnd <= (BYTE*)dst + blockSizeMax);` | process assertion failure | [x] |
| 1369 | `ZSTD_decodeLiteralsBlock` (c_src/src/decompress/zstd_decompress_block.c:241) | `RETURN_ERROR_IF(HUF_isError(hufSuccess), corruption_detected, "");` | `ERROR(HUF_isError)` | [x] |
| 1370 | `ZSTD_decodeLiteralsBlock` (c_src/src/decompress/zstd_decompress_block.c:266) | `RETURN_ERROR_IF(srcSize<3, corruption_detected, "srcSize >= MIN_CBLOCK_SIZE == 2; here we need lhSize = 3");` | `ERROR(srcSize)` | [x] |
| 1371 | `ZSTD_decodeLiteralsBlock` (c_src/src/decompress/zstd_decompress_block.c:271) | `RETURN_ERROR_IF(litSize > 0 && dst == NULL, dstSize_tooSmall, "NULL not handled");` | `ERROR(litSize)` | [x] |
| 1372 | `ZSTD_decodeLiteralsBlock` (c_src/src/decompress/zstd_decompress_block.c:272) | `RETURN_ERROR_IF(litSize > blockSizeMax, corruption_detected, "");` | `ERROR(litSize)` | [x] |
| 1373 | `ZSTD_decodeLiteralsBlock` (c_src/src/decompress/zstd_decompress_block.c:273) | `RETURN_ERROR_IF(expectedWriteSize < litSize, dstSize_tooSmall, "");` | `ERROR(expectedWriteSize)` | [x] |
| 1374 | `ZSTD_decodeLiteralsBlock` (c_src/src/decompress/zstd_decompress_block.c:276) | `RETURN_ERROR_IF(litSize+lhSize > srcSize, corruption_detected, "");` | `ERROR(litSize)` | [x] |
| 1375 | `ZSTD_decodeLiteralsBlock` (c_src/src/decompress/zstd_decompress_block.c:310) | `RETURN_ERROR_IF(srcSize<3, corruption_detected, "srcSize >= MIN_CBLOCK_SIZE == 2; here we need lhSize+1 = 3");` | `ERROR(srcSize)` | [x] |
| 1376 | `ZSTD_decodeLiteralsBlock` (c_src/src/decompress/zstd_decompress_block.c:315) | `RETURN_ERROR_IF(srcSize<4, corruption_detected, "srcSize >= MIN_CBLOCK_SIZE == 2; here we need lhSize+1 = 4");` | `ERROR(srcSize)` | [x] |
| 1377 | `ZSTD_decodeLiteralsBlock` (c_src/src/decompress/zstd_decompress_block.c:319) | `RETURN_ERROR_IF(litSize > 0 && dst == NULL, dstSize_tooSmall, "NULL not handled");` | `ERROR(litSize)` | [x] |
| 1378 | `ZSTD_decodeLiteralsBlock` (c_src/src/decompress/zstd_decompress_block.c:320) | `RETURN_ERROR_IF(litSize > blockSizeMax, corruption_detected, "");` | `ERROR(litSize)` | [x] |
| 1379 | `ZSTD_decodeLiteralsBlock` (c_src/src/decompress/zstd_decompress_block.c:321) | `RETURN_ERROR_IF(expectedWriteSize < litSize, dstSize_tooSmall, "");` | `ERROR(expectedWriteSize)` | [x] |
| 1380 | `ZSTD_decodeLiteralsBlock` (c_src/src/decompress/zstd_decompress_block.c:337) | `RETURN_ERROR(corruption_detected, "impossible");` | `ERROR(corruption_detected)` | [x] |
| 1381 | `ZSTD_buildSeqTable_rle` (c_src/src/decompress/zstd_decompress_block.c:474) | `assert(nbAddBits < 255);` | process assertion failure | [x] |
| 1382 | `ZSTD_buildFSETable_body` (c_src/src/decompress/zstd_decompress_block.c:500) | `assert(maxSymbolValue <= MaxSeq);` | process assertion failure | [x] |
| 1383 | `ZSTD_buildFSETable_body` (c_src/src/decompress/zstd_decompress_block.c:501) | `assert(tableLog <= MaxFSELog);` | process assertion failure | [x] |
| 1384 | `ZSTD_buildFSETable_body` (c_src/src/decompress/zstd_decompress_block.c:502) | `assert(wkspSize >= ZSTD_BUILD_FSE_TABLE_WKSP_SIZE);` | process assertion failure | [x] |
| 1385 | `ZSTD_buildFSETable_body` (c_src/src/decompress/zstd_decompress_block.c:516) | `assert(normalizedCounter[s]>=0);` | process assertion failure | [x] |
| 1386 | `ZSTD_buildFSETable_body` (c_src/src/decompress/zstd_decompress_block.c:523) | `assert(tableSize <= 512);` | process assertion failure | [x] |
| 1387 | `ZSTD_buildFSETable_body` (c_src/src/decompress/zstd_decompress_block.c:550) | `assert(n>=0);` | process assertion failure | [x] |
| 1388 | `ZSTD_buildFSETable_body` (c_src/src/decompress/zstd_decompress_block.c:564) | `assert(tableSize % unroll == 0); /* FSE_MIN_TABLELOG is 5 */` | process assertion failure | [x] |
| 1389 | `ZSTD_buildFSETable_body` (c_src/src/decompress/zstd_decompress_block.c:573) | `assert(position == 0);` | process assertion failure | [x] |
| 1390 | `ZSTD_buildFSETable_body` (c_src/src/decompress/zstd_decompress_block.c:587) | `assert(position == 0); /* position must reach all cells once, otherwise normalizedCounter is incorrect */` | process assertion failure | [x] |
| 1391 | `ZSTD_buildFSETable_body` (c_src/src/decompress/zstd_decompress_block.c:598) | `assert(nbAdditionalBits[symbol] < 255);` | process assertion failure | [x] |
| 1392 | `ZSTD_buildSeqTable` (c_src/src/decompress/zstd_decompress_block.c:658) | `RETURN_ERROR_IF(!srcSize, srcSize_wrong, "");` | source-declared rejection sentinel | [x] |
| 1393 | `ZSTD_buildSeqTable` (c_src/src/decompress/zstd_decompress_block.c:659) | `RETURN_ERROR_IF((*(const BYTE*)src) > max, corruption_detected, "");` | source-declared rejection sentinel | [x] |
| 1394 | `ZSTD_buildSeqTable` (c_src/src/decompress/zstd_decompress_block.c:671) | `RETURN_ERROR_IF(!flagRepeatTable, corruption_detected, "");` | source-declared rejection sentinel | [x] |
| 1395 | `ZSTD_buildSeqTable` (c_src/src/decompress/zstd_decompress_block.c:683) | `RETURN_ERROR_IF(FSE_isError(headerSize), corruption_detected, "");` | `ERROR(FSE_isError)` | [x] |
| 1396 | `ZSTD_buildSeqTable` (c_src/src/decompress/zstd_decompress_block.c:684) | `RETURN_ERROR_IF(tableLog > maxLog, corruption_detected, "");` | `ERROR(tableLog)` | [x] |
| 1397 | `ZSTD_buildSeqTable` (c_src/src/decompress/zstd_decompress_block.c:690) | `assert(0);` | process assertion failure | [x] |
| 1398 | `ZSTD_buildSeqTable` (c_src/src/decompress/zstd_decompress_block.c:691) | `RETURN_ERROR(GENERIC, "impossible");` | `ERROR(GENERIC)` | [x] |
| 1399 | `ZSTD_decodeSeqHeaders` (c_src/src/decompress/zstd_decompress_block.c:705) | `RETURN_ERROR_IF(srcSize < MIN_SEQUENCES_SIZE, srcSize_wrong, "");` | `ERROR(srcSize)` | [x] |
| 1400 | `ZSTD_decodeSeqHeaders` (c_src/src/decompress/zstd_decompress_block.c:711) | `RETURN_ERROR_IF(ip+2 > iend, srcSize_wrong, "");` | `ERROR(ip)` | [x] |
| 1401 | `ZSTD_decodeSeqHeaders` (c_src/src/decompress/zstd_decompress_block.c:715) | `RETURN_ERROR_IF(ip >= iend, srcSize_wrong, "");` | `ERROR(ip)` | [x] |
| 1402 | `ZSTD_decodeSeqHeaders` (c_src/src/decompress/zstd_decompress_block.c:723) | `RETURN_ERROR_IF(ip != iend, corruption_detected,` | `ERROR(ip)` | [x] |
| 1403 | `ZSTD_decodeSeqHeaders` (c_src/src/decompress/zstd_decompress_block.c:729) | `RETURN_ERROR_IF(ip+1 > iend, srcSize_wrong, ""); /* minimum possible size: 1 byte for symbol encoding types */` | `ERROR(ip)` | [x] |
| 1404 | `ZSTD_decodeSeqHeaders` (c_src/src/decompress/zstd_decompress_block.c:730) | `RETURN_ERROR_IF(*ip & 3, corruption_detected, ""); /* The last field, Reserved, must be all-zeroes. */` | source-declared rejection sentinel | [x] |
| 1405 | `ZSTD_decodeSeqHeaders` (c_src/src/decompress/zstd_decompress_block.c:745) | `RETURN_ERROR_IF(ZSTD_isError(llhSize), corruption_detected, "ZSTD_buildSeqTable failed");` | `ERROR(ZSTD_isError)` | [x] |
| 1406 | `ZSTD_decodeSeqHeaders` (c_src/src/decompress/zstd_decompress_block.c:757) | `RETURN_ERROR_IF(ZSTD_isError(ofhSize), corruption_detected, "ZSTD_buildSeqTable failed");` | `ERROR(ZSTD_isError)` | [x] |
| 1407 | `ZSTD_decodeSeqHeaders` (c_src/src/decompress/zstd_decompress_block.c:769) | `RETURN_ERROR_IF(ZSTD_isError(mlhSize), corruption_detected, "ZSTD_buildSeqTable failed");` | `ERROR(ZSTD_isError)` | [x] |
| 1408 | `ZSTD_overlapCopy8` (c_src/src/decompress/zstd_decompress_block.c:805) | `assert(*ip <= *op);` | process assertion failure | [x] |
| 1409 | `ZSTD_overlapCopy8` (c_src/src/decompress/zstd_decompress_block.c:823) | `assert(*op - *ip >= 8);` | process assertion failure | [x] |
| 1410 | `ZSTD_safecopy` (c_src/src/decompress/zstd_decompress_block.c:841) | `assert((ovtype == ZSTD_no_overlap && (diff <= -8 \|\| diff >= 8 \|\| op >= oend_w)) \|\|` | process assertion failure | [x] |
| 1411 | `ZSTD_safecopy` (c_src/src/decompress/zstd_decompress_block.c:851) | `assert(length >= 8);` | process assertion failure | [x] |
| 1412 | `ZSTD_safecopy` (c_src/src/decompress/zstd_decompress_block.c:854) | `assert(op - ip >= 8);` | process assertion failure | [x] |
| 1413 | `ZSTD_safecopy` (c_src/src/decompress/zstd_decompress_block.c:855) | `assert(op <= oend);` | process assertion failure | [x] |
| 1414 | `ZSTD_safecopy` (c_src/src/decompress/zstd_decompress_block.c:865) | `assert(oend > oend_w);` | process assertion failure | [x] |
| 1415 | `ZSTD_execSequenceEnd` (c_src/src/decompress/zstd_decompress_block.c:919) | `RETURN_ERROR_IF(sequenceLength > (size_t)(oend - op), dstSize_tooSmall, "last match must fit within dstBuffer");` | `ERROR(sequenceLength)` | [x] |
| 1416 | `ZSTD_execSequenceEnd` (c_src/src/decompress/zstd_decompress_block.c:920) | `RETURN_ERROR_IF(sequence.litLength > (size_t)(litLimit - *litPtr), corruption_detected, "try to read beyond literal buffer");` | `ERROR(sequence)` | [x] |
| 1417 | `ZSTD_execSequenceEnd` (c_src/src/decompress/zstd_decompress_block.c:921) | `assert(op < op + sequenceLength);` | process assertion failure | [x] |
| 1418 | `ZSTD_execSequenceEnd` (c_src/src/decompress/zstd_decompress_block.c:922) | `assert(oLitEnd < op + sequenceLength);` | process assertion failure | [x] |
| 1419 | `ZSTD_execSequenceEnd` (c_src/src/decompress/zstd_decompress_block.c:932) | `RETURN_ERROR_IF(sequence.offset > (size_t)(oLitEnd - virtualStart), corruption_detected, "");` | `ERROR(sequence)` | [x] |
| 1420 | `ZSTD_execSequenceEndSplitLitBuffer` (c_src/src/decompress/zstd_decompress_block.c:967) | `RETURN_ERROR_IF(sequenceLength > (size_t)(oend - op), dstSize_tooSmall, "last match must fit within dstBuffer");` | `ERROR(sequenceLength)` | [x] |
| 1421 | `ZSTD_execSequenceEndSplitLitBuffer` (c_src/src/decompress/zstd_decompress_block.c:968) | `RETURN_ERROR_IF(sequence.litLength > (size_t)(litLimit - *litPtr), corruption_detected, "try to read beyond literal buffer");` | `ERROR(sequence)` | [x] |
| 1422 | `ZSTD_execSequenceEndSplitLitBuffer` (c_src/src/decompress/zstd_decompress_block.c:969) | `assert(op < op + sequenceLength);` | process assertion failure | [x] |
| 1423 | `ZSTD_execSequenceEndSplitLitBuffer` (c_src/src/decompress/zstd_decompress_block.c:970) | `assert(oLitEnd < op + sequenceLength);` | process assertion failure | [x] |
| 1424 | `ZSTD_execSequenceEndSplitLitBuffer` (c_src/src/decompress/zstd_decompress_block.c:973) | `RETURN_ERROR_IF(op > *litPtr && op < *litPtr + sequence.litLength, dstSize_tooSmall, "output should not catch up to and overwrite literal buffer");` | `ERROR(op)` | [x] |
| 1425 | `ZSTD_execSequenceEndSplitLitBuffer` (c_src/src/decompress/zstd_decompress_block.c:981) | `RETURN_ERROR_IF(sequence.offset > (size_t)(oLitEnd - virtualStart), corruption_detected, "");` | `ERROR(sequence)` | [x] |
| 1426 | `ZSTD_execSequence` (c_src/src/decompress/zstd_decompress_block.c:1013) | `assert(op != NULL /* Precondition */);` | process assertion failure | [x] |
| 1427 | `ZSTD_execSequence` (c_src/src/decompress/zstd_decompress_block.c:1014) | `assert(oend_w < oend /* No underflow */);` | process assertion failure | [x] |
| 1428 | `ZSTD_execSequence` (c_src/src/decompress/zstd_decompress_block.c:1032) | `assert(op <= oLitEnd /* No overflow */);` | process assertion failure | [x] |
| 1429 | `ZSTD_execSequence` (c_src/src/decompress/zstd_decompress_block.c:1033) | `assert(oLitEnd < oMatchEnd /* Non-zero match & no overflow */);` | process assertion failure | [x] |
| 1430 | `ZSTD_execSequence` (c_src/src/decompress/zstd_decompress_block.c:1034) | `assert(oMatchEnd <= oend /* No underflow */);` | process assertion failure | [x] |
| 1431 | `ZSTD_execSequence` (c_src/src/decompress/zstd_decompress_block.c:1035) | `assert(iLitEnd <= litLimit /* Literal length is in bounds */);` | process assertion failure | [x] |
| 1432 | `ZSTD_execSequence` (c_src/src/decompress/zstd_decompress_block.c:1036) | `assert(oLitEnd <= oend_w /* Can wildcopy literals */);` | process assertion failure | [x] |
| 1433 | `ZSTD_execSequence` (c_src/src/decompress/zstd_decompress_block.c:1037) | `assert(oMatchEnd <= oend_w /* Can wildcopy matches */);` | process assertion failure | [x] |
| 1434 | `ZSTD_execSequence` (c_src/src/decompress/zstd_decompress_block.c:1043) | `assert(WILDCOPY_OVERLENGTH >= 16);` | process assertion failure | [x] |
| 1435 | `ZSTD_execSequence` (c_src/src/decompress/zstd_decompress_block.c:1054) | `RETURN_ERROR_IF(UNLIKELY(sequence.offset > (size_t)(oLitEnd - virtualStart)), corruption_detected, "");` | `ERROR(UNLIKELY)` | [x] |
| 1436 | `ZSTD_execSequence` (c_src/src/decompress/zstd_decompress_block.c:1069) | `assert(op <= oMatchEnd);` | process assertion failure | [x] |
| 1437 | `ZSTD_execSequence` (c_src/src/decompress/zstd_decompress_block.c:1070) | `assert(oMatchEnd <= oend_w);` | process assertion failure | [x] |
| 1438 | `ZSTD_execSequence` (c_src/src/decompress/zstd_decompress_block.c:1071) | `assert(match >= prefixStart);` | process assertion failure | [x] |
| 1439 | `ZSTD_execSequence` (c_src/src/decompress/zstd_decompress_block.c:1072) | `assert(sequence.matchLength >= 1);` | process assertion failure | [x] |
| 1440 | `ZSTD_execSequence` (c_src/src/decompress/zstd_decompress_block.c:1085) | `assert(sequence.offset < WILDCOPY_VECLEN);` | process assertion failure | [x] |
| 1441 | `ZSTD_execSequence` (c_src/src/decompress/zstd_decompress_block.c:1092) | `assert(op < oMatchEnd);` | process assertion failure | [x] |
| 1442 | `ZSTD_execSequenceSplitLitBuffer` (c_src/src/decompress/zstd_decompress_block.c:1111) | `assert(op != NULL /* Precondition */);` | process assertion failure | [x] |
| 1443 | `ZSTD_execSequenceSplitLitBuffer` (c_src/src/decompress/zstd_decompress_block.c:1112) | `assert(oend_w < oend /* No underflow */);` | process assertion failure | [x] |
| 1444 | `ZSTD_execSequenceSplitLitBuffer` (c_src/src/decompress/zstd_decompress_block.c:1125) | `assert(op <= oLitEnd /* No overflow */);` | process assertion failure | [x] |
| 1445 | `ZSTD_execSequenceSplitLitBuffer` (c_src/src/decompress/zstd_decompress_block.c:1126) | `assert(oLitEnd < oMatchEnd /* Non-zero match & no overflow */);` | process assertion failure | [x] |
| 1446 | `ZSTD_execSequenceSplitLitBuffer` (c_src/src/decompress/zstd_decompress_block.c:1127) | `assert(oMatchEnd <= oend /* No underflow */);` | process assertion failure | [x] |
| 1447 | `ZSTD_execSequenceSplitLitBuffer` (c_src/src/decompress/zstd_decompress_block.c:1128) | `assert(iLitEnd <= litLimit /* Literal length is in bounds */);` | process assertion failure | [x] |
| 1448 | `ZSTD_execSequenceSplitLitBuffer` (c_src/src/decompress/zstd_decompress_block.c:1129) | `assert(oLitEnd <= oend_w /* Can wildcopy literals */);` | process assertion failure | [x] |
| 1449 | `ZSTD_execSequenceSplitLitBuffer` (c_src/src/decompress/zstd_decompress_block.c:1130) | `assert(oMatchEnd <= oend_w /* Can wildcopy matches */);` | process assertion failure | [x] |
| 1450 | `ZSTD_execSequenceSplitLitBuffer` (c_src/src/decompress/zstd_decompress_block.c:1136) | `assert(WILDCOPY_OVERLENGTH >= 16);` | process assertion failure | [x] |
| 1451 | `ZSTD_execSequenceSplitLitBuffer` (c_src/src/decompress/zstd_decompress_block.c:1147) | `RETURN_ERROR_IF(UNLIKELY(sequence.offset > (size_t)(oLitEnd - virtualStart)), corruption_detected, "");` | `ERROR(UNLIKELY)` | [x] |
| 1452 | `ZSTD_execSequenceSplitLitBuffer` (c_src/src/decompress/zstd_decompress_block.c:1161) | `assert(op <= oMatchEnd);` | process assertion failure | [x] |
| 1453 | `ZSTD_execSequenceSplitLitBuffer` (c_src/src/decompress/zstd_decompress_block.c:1162) | `assert(oMatchEnd <= oend_w);` | process assertion failure | [x] |
| 1454 | `ZSTD_execSequenceSplitLitBuffer` (c_src/src/decompress/zstd_decompress_block.c:1163) | `assert(match >= prefixStart);` | process assertion failure | [x] |
| 1455 | `ZSTD_execSequenceSplitLitBuffer` (c_src/src/decompress/zstd_decompress_block.c:1164) | `assert(sequence.matchLength >= 1);` | process assertion failure | [x] |
| 1456 | `ZSTD_execSequenceSplitLitBuffer` (c_src/src/decompress/zstd_decompress_block.c:1177) | `assert(sequence.offset < WILDCOPY_VECLEN);` | process assertion failure | [x] |
| 1457 | `ZSTD_execSequenceSplitLitBuffer` (c_src/src/decompress/zstd_decompress_block.c:1184) | `assert(op < oMatchEnd);` | process assertion failure | [x] |
| 1458 | `ZSTD_decodeSequence` (c_src/src/decompress/zstd_decompress_block.c:1268) | `assert(llBits <= MaxLLBits);` | process assertion failure | [x] |
| 1459 | `ZSTD_decodeSequence` (c_src/src/decompress/zstd_decompress_block.c:1269) | `assert(mlBits <= MaxMLBits);` | process assertion failure | [x] |
| 1460 | `ZSTD_decodeSequence` (c_src/src/decompress/zstd_decompress_block.c:1270) | `assert(ofBits <= MaxOff);` | process assertion failure | [x] |
| 1461 | `ZSTD_decodeSequence` (c_src/src/decompress/zstd_decompress_block.c:1280) | `ZSTD_STATIC_ASSERT(ZSTD_lo_isLongOffset == 1);` | process assertion failure | [x] |
| 1462 | `ZSTD_decodeSequence` (c_src/src/decompress/zstd_decompress_block.c:1281) | `ZSTD_STATIC_ASSERT(LONG_OFFSETS_MAX_EXTRA_BITS_32 == 5);` | process assertion failure | [x] |
| 1463 | `ZSTD_decodeSequence` (c_src/src/decompress/zstd_decompress_block.c:1282) | `ZSTD_STATIC_ASSERT(STREAM_ACCUMULATOR_MIN_32 > LONG_OFFSETS_MAX_EXTRA_BITS_32);` | process assertion failure | [x] |
| 1464 | `ZSTD_decodeSequence` (c_src/src/decompress/zstd_decompress_block.c:1283) | `ZSTD_STATIC_ASSERT(STREAM_ACCUMULATOR_MIN_32 - LONG_OFFSETS_MAX_EXTRA_BITS_32 >= MaxMLBits);` | process assertion failure | [x] |
| 1465 | `ZSTD_decodeSequence` (c_src/src/decompress/zstd_decompress_block.c:1324) | `ZSTD_STATIC_ASSERT(16+LLFSELog+MLFSELog+OffFSELog < STREAM_ACCUMULATOR_MIN_64);` | process assertion failure | [x] |
| 1466 | `ZSTD_assertValidSequence` (c_src/src/decompress/zstd_decompress_block.c:1379) | `assert(op <= oend);` | process assertion failure | [x] |
| 1467 | `ZSTD_assertValidSequence` (c_src/src/decompress/zstd_decompress_block.c:1380) | `assert((size_t)(oend - op) >= sequenceSize);` | process assertion failure | [x] |
| 1468 | `ZSTD_assertValidSequence` (c_src/src/decompress/zstd_decompress_block.c:1381) | `assert(sequenceSize <= ZSTD_blockSizeMax(dctx));` | process assertion failure | [x] |
| 1469 | `ZSTD_assertValidSequence` (c_src/src/decompress/zstd_decompress_block.c:1385) | `assert(seq.offset <= (size_t)(oLitEnd - virtualStart));` | process assertion failure | [x] |
| 1470 | `ZSTD_assertValidSequence` (c_src/src/decompress/zstd_decompress_block.c:1386) | `assert(seq.offset <= windowSize + dictSize);` | process assertion failure | [x] |
| 1471 | `ZSTD_assertValidSequence` (c_src/src/decompress/zstd_decompress_block.c:1389) | `assert(seq.offset <= windowSize);` | process assertion failure | [x] |
| 1472 | `ZSTD_decompressSequences_bodySplitLitBuffer` (c_src/src/decompress/zstd_decompress_block.c:1425) | `RETURN_ERROR_IF(` | source-declared rejection sentinel | [x] |
| 1473 | `ZSTD_decompressSequences_bodySplitLitBuffer` (c_src/src/decompress/zstd_decompress_block.c:1431) | `assert(dst != NULL);` | process assertion failure | [x] |
| 1474 | `ZSTD_decompressSequences_bodySplitLitBuffer` (c_src/src/decompress/zstd_decompress_block.c:1433) | `ZSTD_STATIC_ASSERT(` | process assertion failure | [x] |
| 1475 | `ZSTD_decompressSequences_bodySplitLitBuffer` (c_src/src/decompress/zstd_decompress_block.c:1506) | `assert(!ZSTD_isError(oneSeqSize));` | process assertion failure | [x] |
| 1476 | `ZSTD_decompressSequences_bodySplitLitBuffer` (c_src/src/decompress/zstd_decompress_block.c:1521) | `RETURN_ERROR_IF(leftoverLit > (size_t)(oend - op), dstSize_tooSmall, "remaining lit must fit within dstBuffer");` | `ERROR(leftoverLit)` | [x] |
| 1477 | `ZSTD_decompressSequences_bodySplitLitBuffer` (c_src/src/decompress/zstd_decompress_block.c:1531) | `assert(!ZSTD_isError(oneSeqSize));` | process assertion failure | [x] |
| 1478 | `ZSTD_decompressSequences_bodySplitLitBuffer` (c_src/src/decompress/zstd_decompress_block.c:1567) | `assert(!ZSTD_isError(oneSeqSize));` | process assertion failure | [x] |
| 1479 | `ZSTD_decompressSequences_bodySplitLitBuffer` (c_src/src/decompress/zstd_decompress_block.c:1579) | `RETURN_ERROR_IF(nbSeq, corruption_detected, "");` | `ERROR(nbSeq)` | [x] |
| 1480 | `ZSTD_decompressSequences_bodySplitLitBuffer` (c_src/src/decompress/zstd_decompress_block.c:1581) | `RETURN_ERROR_IF(!BIT_endOfDStream(&seqState.DStream), corruption_detected, "");` | source-declared rejection sentinel | [x] |
| 1481 | `ZSTD_decompressSequences_bodySplitLitBuffer` (c_src/src/decompress/zstd_decompress_block.c:1591) | `RETURN_ERROR_IF(lastLLSize > (size_t)(oend - op), dstSize_tooSmall, "");` | `ERROR(lastLLSize)` | [x] |
| 1482 | `ZSTD_decompressSequences_bodySplitLitBuffer` (c_src/src/decompress/zstd_decompress_block.c:1603) | `RETURN_ERROR_IF(lastLLSize > (size_t)(oend-op), dstSize_tooSmall, "");` | `ERROR(lastLLSize)` | [x] |
| 1483 | `ZSTD_decompressSequences_body` (c_src/src/decompress/zstd_decompress_block.c:1637) | `RETURN_ERROR_IF(` | source-declared rejection sentinel | [x] |
| 1484 | `ZSTD_decompressSequences_body` (c_src/src/decompress/zstd_decompress_block.c:1643) | `assert(dst != NULL);` | process assertion failure | [x] |
| 1485 | `ZSTD_decompressSequences_body` (c_src/src/decompress/zstd_decompress_block.c:1663) | `assert(!ZSTD_isError(oneSeqSize));` | process assertion failure | [x] |
| 1486 | `ZSTD_decompressSequences_body` (c_src/src/decompress/zstd_decompress_block.c:1673) | `assert(nbSeq == 0);` | process assertion failure | [x] |
| 1487 | `ZSTD_decompressSequences_body` (c_src/src/decompress/zstd_decompress_block.c:1674) | `RETURN_ERROR_IF(!BIT_endOfDStream(&seqState.DStream), corruption_detected, "");` | source-declared rejection sentinel | [x] |
| 1488 | `ZSTD_decompressSequences_body` (c_src/src/decompress/zstd_decompress_block.c:1682) | `RETURN_ERROR_IF(lastLLSize > (size_t)(oend-op), dstSize_tooSmall, "");` | `ERROR(lastLLSize)` | [x] |
| 1489 | `ZSTD_decompressSequencesLong_body` (c_src/src/decompress/zstd_decompress_block.c:1763) | `assert(dst != NULL);` | process assertion failure | [x] |
| 1490 | `ZSTD_decompressSequencesLong_body` (c_src/src/decompress/zstd_decompress_block.c:1764) | `assert(iend >= ip);` | process assertion failure | [x] |
| 1491 | `ZSTD_decompressSequencesLong_body` (c_src/src/decompress/zstd_decompress_block.c:1765) | `RETURN_ERROR_IF(` | source-declared rejection sentinel | [x] |
| 1492 | `ZSTD_decompressSequencesLong_body` (c_src/src/decompress/zstd_decompress_block.c:1788) | `RETURN_ERROR_IF(leftoverLit > (size_t)(oend - op), dstSize_tooSmall, "remaining lit must fit within dstBuffer");` | `ERROR(leftoverLit)` | [x] |
| 1493 | `ZSTD_decompressSequencesLong_body` (c_src/src/decompress/zstd_decompress_block.c:1798) | `assert(!ZSTD_isError(oneSeqSize));` | process assertion failure | [x] |
| 1494 | `ZSTD_decompressSequencesLong_body` (c_src/src/decompress/zstd_decompress_block.c:1814) | `assert(!ZSTD_isError(oneSeqSize));` | process assertion failure | [x] |
| 1495 | `ZSTD_decompressSequencesLong_body` (c_src/src/decompress/zstd_decompress_block.c:1824) | `RETURN_ERROR_IF(!BIT_endOfDStream(&seqState.DStream), corruption_detected, "");` | source-declared rejection sentinel | [x] |
| 1496 | `ZSTD_decompressSequencesLong_body` (c_src/src/decompress/zstd_decompress_block.c:1833) | `RETURN_ERROR_IF(leftoverLit > (size_t)(oend - op), dstSize_tooSmall, "remaining lit must fit within dstBuffer");` | `ERROR(leftoverLit)` | [x] |
| 1497 | `ZSTD_decompressSequencesLong_body` (c_src/src/decompress/zstd_decompress_block.c:1843) | `assert(!ZSTD_isError(oneSeqSize));` | process assertion failure | [x] |
| 1498 | `ZSTD_decompressSequencesLong_body` (c_src/src/decompress/zstd_decompress_block.c:1856) | `assert(!ZSTD_isError(oneSeqSize));` | process assertion failure | [x] |
| 1499 | `ZSTD_decompressSequencesLong_body` (c_src/src/decompress/zstd_decompress_block.c:1871) | `RETURN_ERROR_IF(lastLLSize > (size_t)(oend - op), dstSize_tooSmall, "");` | `ERROR(lastLLSize)` | [x] |
| 1500 | `ZSTD_decompressSequencesLong_body` (c_src/src/decompress/zstd_decompress_block.c:1880) | `RETURN_ERROR_IF(lastLLSize > (size_t)(oend-op), dstSize_tooSmall, "");` | `ERROR(lastLLSize)` | [x] |
| 1501 | `ZSTD_getOffsetInfo` (c_src/src/decompress/zstd_decompress_block.c:2027) | `assert(max <= (1 << OffFSELog)); /* max not too large */` | process assertion failure | [x] |
| 1502 | `ZSTD_getOffsetInfo` (c_src/src/decompress/zstd_decompress_block.c:2033) | `assert(tableLog <= OffFSELog);` | process assertion failure | [x] |
| 1503 | `ZSTD_maxShortOffset` (c_src/src/decompress/zstd_decompress_block.c:2051) | `ZSTD_STATIC_ASSERT(ZSTD_WINDOWLOG_MAX <= 31);` | process assertion failure | [x] |
| 1504 | `ZSTD_maxShortOffset` (c_src/src/decompress/zstd_decompress_block.c:2052) | `return (size_t)-1;` | `(size_t)-1` | [x] |
| 1505 | `ZSTD_maxShortOffset` (c_src/src/decompress/zstd_decompress_block.c:2060) | `assert(ZSTD_highbit32((U32)maxOffbase) == STREAM_ACCUMULATOR_MIN);` | process assertion failure | [x] |
| 1506 | `ZSTD_decompressBlock_internal` (c_src/src/decompress/zstd_decompress_block.c:2081) | `RETURN_ERROR_IF(srcSize > ZSTD_blockSizeMax(dctx), srcSize_wrong, "");` | `ERROR(srcSize)` | [x] |
| 1507 | `ZSTD_decompressBlock_internal` (c_src/src/decompress/zstd_decompress_block.c:2129) | `RETURN_ERROR_IF((dst == NULL \|\| dstCapacity == 0) && nbSeq > 0, dstSize_tooSmall, "NULL not handled");` | source-declared rejection sentinel | [x] |
| 1508 | `ZSTD_decompressBlock_internal` (c_src/src/decompress/zstd_decompress_block.c:2130) | `RETURN_ERROR_IF(MEM_64bits() && sizeof(size_t) == sizeof(void*) && (size_t)(-1) - (size_t)dst < (size_t)(1 << 20), dstSize_tooSmall,` | `ERROR(MEM_64bits)` | [x] |
| 1509 | `COVER_cmp8` (c_src/src/dictBuilder/cover.c:283) | `return -1;` | `-1` | [x] |
| 1510 | `COVER_lower_bound` (c_src/src/dictBuilder/cover.c:358) | `assert(last >= first);` | process assertion failure | [x] |
| 1511 | `COVER_ctx_init` (c_src/src/dictBuilder/cover.c:618) | `return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1512 | `COVER_ctx_init` (c_src/src/dictBuilder/cover.c:623) | `return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1513 | `COVER_ctx_init` (c_src/src/dictBuilder/cover.c:628) | `return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1514 | `COVER_ctx_init` (c_src/src/dictBuilder/cover.c:651) | `return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [x] |
| 1515 | `COVER_computeEpochs` (c_src/src/dictBuilder/cover.c:715) | `assert(epochs.size * epochs.num <= nbDmers);` | process assertion failure | [x] |
| 1516 | `COVER_computeEpochs` (c_src/src/dictBuilder/cover.c:720) | `assert(epochs.size * epochs.num <= nbDmers);` | process assertion failure | [x] |
| 1517 | `ZDICT_trainFromBuffer_cover` (c_src/src/dictBuilder/cover.c:793) | `return ERROR(parameter_outOfBound);` | `ERROR(parameter_outOfBound)` | [x] |
| 1518 | `ZDICT_trainFromBuffer_cover` (c_src/src/dictBuilder/cover.c:797) | `return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1519 | `ZDICT_trainFromBuffer_cover` (c_src/src/dictBuilder/cover.c:802) | `return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 1520 | `ZDICT_trainFromBuffer_cover` (c_src/src/dictBuilder/cover.c:816) | `return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [x] |
| 1521 | `COVER_dictSelectionError` (c_src/src/dictBuilder/cover.c:1009) | `return setDictSelection(NULL, 0, error);` | source-declared rejection sentinel | [x] |
| 1522 | `ZDICT_optimizeTrainFromBuffer_cover` (c_src/src/dictBuilder/cover.c:1197) | `return ERROR(parameter_outOfBound);` | `ERROR(parameter_outOfBound)` | [x] |
| 1523 | `ZDICT_optimizeTrainFromBuffer_cover` (c_src/src/dictBuilder/cover.c:1201) | `return ERROR(parameter_outOfBound);` | `ERROR(parameter_outOfBound)` | [x] |
| 1524 | `ZDICT_optimizeTrainFromBuffer_cover` (c_src/src/dictBuilder/cover.c:1205) | `return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1525 | `ZDICT_optimizeTrainFromBuffer_cover` (c_src/src/dictBuilder/cover.c:1210) | `return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 1526 | `ZDICT_optimizeTrainFromBuffer_cover` (c_src/src/dictBuilder/cover.c:1215) | `return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [x] |
| 1527 | `ZDICT_optimizeTrainFromBuffer_cover` (c_src/src/dictBuilder/cover.c:1253) | `return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [x] |
| 1528 | `(file scope)` (c_src/src/dictBuilder/divsufsort.c:104) | `assert(ssize < STACK_SIZE);\` | process assertion failure | [x] |
| 1529 | `(file scope)` (c_src/src/dictBuilder/divsufsort.c:110) | `assert(ssize < STACK_SIZE);\` | process assertion failure | [x] |
| 1530 | `(file scope)` (c_src/src/dictBuilder/divsufsort.c:116) | `assert(0 <= ssize);\` | process assertion failure | [x] |
| 1531 | `(file scope)` (c_src/src/dictBuilder/divsufsort.c:123) | `assert(0 <= ssize);\` | process assertion failure | [x] |
| 1532 | `construct_SA` (c_src/src/dictBuilder/divsufsort.c:1630) | `assert(T[s] == c1);` | process assertion failure | [x] |
| 1533 | `construct_SA` (c_src/src/dictBuilder/divsufsort.c:1631) | `assert(((s + 1) < n) && (T[s] <= T[s + 1]));` | process assertion failure | [x] |
| 1534 | `construct_SA` (c_src/src/dictBuilder/divsufsort.c:1632) | `assert(T[s - 1] <= T[s]);` | process assertion failure | [x] |
| 1535 | `construct_SA` (c_src/src/dictBuilder/divsufsort.c:1640) | `assert(k < j); assert(k != NULL);` | process assertion failure | [x] |
| 1536 | `construct_SA` (c_src/src/dictBuilder/divsufsort.c:1643) | `assert(((s == 0) && (T[s] == c1)) \|\| (s < 0));` | process assertion failure | [x] |
| 1537 | `construct_SA` (c_src/src/dictBuilder/divsufsort.c:1657) | `assert(T[s - 1] >= T[s]);` | process assertion failure | [x] |
| 1538 | `construct_SA` (c_src/src/dictBuilder/divsufsort.c:1664) | `assert(i < k);` | process assertion failure | [x] |
| 1539 | `construct_SA` (c_src/src/dictBuilder/divsufsort.c:1667) | `assert(s < 0);` | process assertion failure | [x] |
| 1540 | `construct_BWT` (c_src/src/dictBuilder/divsufsort.c:1694) | `assert(T[s] == c1);` | process assertion failure | [x] |
| 1541 | `construct_BWT` (c_src/src/dictBuilder/divsufsort.c:1695) | `assert(((s + 1) < n) && (T[s] <= T[s + 1]));` | process assertion failure | [x] |
| 1542 | `construct_BWT` (c_src/src/dictBuilder/divsufsort.c:1696) | `assert(T[s - 1] <= T[s]);` | process assertion failure | [x] |
| 1543 | `construct_BWT` (c_src/src/dictBuilder/divsufsort.c:1704) | `assert(k < j); assert(k != NULL);` | process assertion failure | [x] |
| 1544 | `construct_BWT` (c_src/src/dictBuilder/divsufsort.c:1710) | `assert(T[s] == c1);` | process assertion failure | [x] |
| 1545 | `construct_BWT` (c_src/src/dictBuilder/divsufsort.c:1724) | `assert(T[s - 1] >= T[s]);` | process assertion failure | [x] |
| 1546 | `construct_BWT` (c_src/src/dictBuilder/divsufsort.c:1732) | `assert(i < k);` | process assertion failure | [x] |
| 1547 | `construct_BWT_indexes` (c_src/src/dictBuilder/divsufsort.c:1775) | `assert(T[s] == c1);` | process assertion failure | [x] |
| 1548 | `construct_BWT_indexes` (c_src/src/dictBuilder/divsufsort.c:1776) | `assert(((s + 1) < n) && (T[s] <= T[s + 1]));` | process assertion failure | [x] |
| 1549 | `construct_BWT_indexes` (c_src/src/dictBuilder/divsufsort.c:1777) | `assert(T[s - 1] <= T[s]);` | process assertion failure | [x] |
| 1550 | `construct_BWT_indexes` (c_src/src/dictBuilder/divsufsort.c:1788) | `assert(k < j); assert(k != NULL);` | process assertion failure | [x] |
| 1551 | `construct_BWT_indexes` (c_src/src/dictBuilder/divsufsort.c:1794) | `assert(T[s] == c1);` | process assertion failure | [x] |
| 1552 | `construct_BWT_indexes` (c_src/src/dictBuilder/divsufsort.c:1815) | `assert(T[s - 1] >= T[s]);` | process assertion failure | [x] |
| 1553 | `construct_BWT_indexes` (c_src/src/dictBuilder/divsufsort.c:1825) | `assert(i < k);` | process assertion failure | [x] |
| 1554 | `divsufsort` (c_src/src/dictBuilder/divsufsort.c:1853) | `if((T == NULL) \|\| (SA == NULL) \|\| (n < 0)) { return -1; }` | `-1` | [x] |
| 1555 | `divbwt` (c_src/src/dictBuilder/divsufsort.c:1882) | `if((T == NULL) \|\| (U == NULL) \|\| (n < 0)) { return -1; }` | `-1` | [x] |
| 1556 | `FASTCOVER_computeFrequency` (c_src/src/dictBuilder/fastcover.c:291) | `assert(ctx->nbTrainSamples >= 5);` | process assertion failure | [x] |
| 1557 | `FASTCOVER_computeFrequency` (c_src/src/dictBuilder/fastcover.c:292) | `assert(ctx->nbTrainSamples <= ctx->nbSamples);` | process assertion failure | [x] |
| 1558 | `FASTCOVER_ctx_init` (c_src/src/dictBuilder/fastcover.c:332) | `return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1559 | `FASTCOVER_ctx_init` (c_src/src/dictBuilder/fastcover.c:338) | `return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1560 | `FASTCOVER_ctx_init` (c_src/src/dictBuilder/fastcover.c:344) | `return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1561 | `FASTCOVER_ctx_init` (c_src/src/dictBuilder/fastcover.c:369) | `return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [x] |
| 1562 | `FASTCOVER_ctx_init` (c_src/src/dictBuilder/fastcover.c:375) | `assert(nbSamples >= 5);` | process assertion failure | [x] |
| 1563 | `FASTCOVER_ctx_init` (c_src/src/dictBuilder/fastcover.c:386) | `return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [x] |
| 1564 | `ZDICT_trainFromBuffer_fastCover` (c_src/src/dictBuilder/fastcover.c:571) | `return ERROR(parameter_outOfBound);` | `ERROR(parameter_outOfBound)` | [x] |
| 1565 | `ZDICT_trainFromBuffer_fastCover` (c_src/src/dictBuilder/fastcover.c:575) | `return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1566 | `ZDICT_trainFromBuffer_fastCover` (c_src/src/dictBuilder/fastcover.c:580) | `return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 1567 | `ZDICT_optimizeTrainFromBuffer_fastCover` (c_src/src/dictBuilder/fastcover.c:652) | `return ERROR(parameter_outOfBound);` | `ERROR(parameter_outOfBound)` | [x] |
| 1568 | `ZDICT_optimizeTrainFromBuffer_fastCover` (c_src/src/dictBuilder/fastcover.c:656) | `return ERROR(parameter_outOfBound);` | `ERROR(parameter_outOfBound)` | [x] |
| 1569 | `ZDICT_optimizeTrainFromBuffer_fastCover` (c_src/src/dictBuilder/fastcover.c:660) | `return ERROR(parameter_outOfBound);` | `ERROR(parameter_outOfBound)` | [x] |
| 1570 | `ZDICT_optimizeTrainFromBuffer_fastCover` (c_src/src/dictBuilder/fastcover.c:664) | `return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1571 | `ZDICT_optimizeTrainFromBuffer_fastCover` (c_src/src/dictBuilder/fastcover.c:669) | `return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 1572 | `ZDICT_optimizeTrainFromBuffer_fastCover` (c_src/src/dictBuilder/fastcover.c:674) | `return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [x] |
| 1573 | `ZDICT_optimizeTrainFromBuffer_fastCover` (c_src/src/dictBuilder/fastcover.c:715) | `return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [x] |
| 1574 | `ZDICT_getDictHeaderSize` (c_src/src/dictBuilder/zdict.c:112) | `if (dictSize <= 8 \|\| MEM_readLE32(dictBuffer) != ZSTD_MAGIC_DICTIONARY) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [x] |
| 1575 | `ZDICT_analyzeEntropy` (c_src/src/dictBuilder/zdict.c:735) | `assert(maxNbBits==9);` | process assertion failure | [x] |
| 1576 | `ZDICT_finalizeDictionary` (c_src/src/dictBuilder/zdict.c:874) | `if (dictBufferCapacity < dictContentSize) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 1577 | `ZDICT_finalizeDictionary` (c_src/src/dictBuilder/zdict.c:875) | `if (dictBufferCapacity < ZDICT_DICTSIZE_MIN) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 1578 | `ZDICT_finalizeDictionary` (c_src/src/dictBuilder/zdict.c:905) | `RETURN_ERROR_IF(hSize + minContentSize > dictBufferCapacity, dstSize_tooSmall,` | `ERROR(hSize)` | [x] |
| 1579 | `ZDICT_finalizeDictionary` (c_src/src/dictBuilder/zdict.c:923) | `assert(dictSize <= dictBufferCapacity);` | process assertion failure | [x] |
| 1580 | `ZDICT_finalizeDictionary` (c_src/src/dictBuilder/zdict.c:924) | `assert(outDictContent + dictContentSize == (BYTE*)dictBuffer + dictSize);` | process assertion failure | [x] |
| 1581 | `ZDICT_trainFromBuffer_unsafe_legacy` (c_src/src/dictBuilder/zdict.c:993) | `if (!dictList) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [x] |
| 1582 | `ZDICT_trainFromBuffer_unsafe_legacy` (c_src/src/dictBuilder/zdict.c:994) | `if (maxDictSize < ZDICT_DICTSIZE_MIN) { free(dictList); return ERROR(dstSize_tooSmall); } /* requested dictionary size is too small */` | `ERROR(dstSize_tooSmall)` | [x] |
| 1583 | `ZDICT_trainFromBuffer_unsafe_legacy` (c_src/src/dictBuilder/zdict.c:995) | `if (samplesBuffSize < ZDICT_MIN_SAMPLES_SIZE) { free(dictList); return ERROR(dictionaryCreation_failed); } /* not enough source to create dictionary */` | `ERROR(dictionaryCreation_failed)` | [x] |
| 1584 | `ZDICT_trainFromBuffer_unsafe_legacy` (c_src/src/dictBuilder/zdict.c:1019) | `return ERROR(GENERIC); /* should never happen */` | `ERROR(GENERIC)` | [x] |
| 1585 | `ZDICT_trainFromBuffer_unsafe_legacy` (c_src/src/dictBuilder/zdict.c:1030) | `if (dictContentSize < ZDICT_CONTENTSIZE_MIN) { free(dictList); return ERROR(dictionaryCreation_failed); } /* dictionary content too small */` | `ERROR(dictionaryCreation_failed)` | [x] |
| 1586 | `ZDICT_trainFromBuffer_unsafe_legacy` (c_src/src/dictBuilder/zdict.c:1066) | `if (ptr<(BYTE*)dictBuffer) { free(dictList); return ERROR(GENERIC); } /* should not happen */` | `ERROR(GENERIC)` | [x] |
| 1587 | `ZDICT_trainFromBuffer_legacy` (c_src/src/dictBuilder/zdict.c:1094) | `if (!newBuff) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [x] |
| 1588 | `(file scope)` (c_src/src/legacy/zstd_legacy.h:130) | `assert(dstCapacity == 0);` | process assertion failure | [x] |
| 1589 | `(file scope)` (c_src/src/legacy/zstd_legacy.h:134) | `assert(compressedSize == 0);` | process assertion failure | [x] |
| 1590 | `(file scope)` (c_src/src/legacy/zstd_legacy.h:138) | `assert(dictSize == 0);` | process assertion failure | [x] |
| 1591 | `(file scope)` (c_src/src/legacy/zstd_legacy.h:164) | `if (zd==NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [x] |
| 1592 | `(file scope)` (c_src/src/legacy/zstd_legacy.h:174) | `if (zd==NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [x] |
| 1593 | `(file scope)` (c_src/src/legacy/zstd_legacy.h:184) | `if (zd==NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [x] |
| 1594 | `(file scope)` (c_src/src/legacy/zstd_legacy.h:191) | `return ERROR(prefix_unknown);` | `ERROR(prefix_unknown)` | [x] |
| 1595 | `(file scope)` (c_src/src/legacy/zstd_legacy.h:263) | `assert((frameSizeInfo.decompressedBound & (ZSTD_BLOCKSIZE_MAX - 1)) == 0);` | process assertion failure | [x] |
| 1596 | `(file scope)` (c_src/src/legacy/zstd_legacy.h:284) | `return ERROR(version_unsupported);` | `ERROR(version_unsupported)` | [x] |
| 1597 | `(file scope)` (c_src/src/legacy/zstd_legacy.h:307) | `assert(dictSize == 0);` | process assertion failure | [x] |
| 1598 | `(file scope)` (c_src/src/legacy/zstd_legacy.h:324) | `if (dctx==NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [x] |
| 1599 | `(file scope)` (c_src/src/legacy/zstd_legacy.h:335) | `if (dctx==NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [x] |
| 1600 | `(file scope)` (c_src/src/legacy/zstd_legacy.h:345) | `if (dctx==NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [x] |
| 1601 | `(file scope)` (c_src/src/legacy/zstd_legacy.h:355) | `if (dctx==NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [x] |
| 1602 | `(file scope)` (c_src/src/legacy/zstd_legacy.h:372) | `assert(output->size == 0);` | process assertion failure | [x] |
| 1603 | `(file scope)` (c_src/src/legacy/zstd_legacy.h:376) | `assert(input->size == 0);` | process assertion failure | [x] |
| 1604 | `(file scope)` (c_src/src/legacy/zstd_legacy.h:387) | `return ERROR(version_unsupported);` | `ERROR(version_unsupported)` | [x] |
| 1605 | `FSE_buildDTable` (c_src/src/legacy/zstd_v01.c:374) | `if (maxSymbolValue > FSE_MAX_SYMBOL_VALUE) return (size_t)-FSE_ERROR_maxSymbolValue_tooLarge;` | `(size_t)-FSE_ERROR_maxSymbolValue_tooLarge` | [x] |
| 1606 | `FSE_buildDTable` (c_src/src/legacy/zstd_v01.c:375) | `if (tableLog > FSE_MAX_TABLELOG) return (size_t)-FSE_ERROR_tableLog_tooLarge;` | `(size_t)-FSE_ERROR_tableLog_tooLarge` | [x] |
| 1607 | `FSE_buildDTable` (c_src/src/legacy/zstd_v01.c:405) | `if (position!=0) return (size_t)-FSE_ERROR_GENERIC; /* position must reach all cells once, otherwise normalizedCounter is incorrect */` | `(size_t)-FSE_ERROR_GENERIC` | [x] |
| 1608 | `FSE_isError` (c_src/src/legacy/zstd_v01.c:429) | `static unsigned FSE_isError(size_t code) { return (code > (size_t)(-FSE_ERROR_maxCode)); }` | source-declared rejection sentinel | [x] |
| 1609 | `FSE_readNCount` (c_src/src/legacy/zstd_v01.c:454) | `if (hbSize < 4) return (size_t)-FSE_ERROR_srcSize_wrong;` | `(size_t)-FSE_ERROR_srcSize_wrong` | [x] |
| 1610 | `FSE_readNCount` (c_src/src/legacy/zstd_v01.c:457) | `if (nbBits > FSE_TABLELOG_ABSOLUTE_MAX) return (size_t)-FSE_ERROR_tableLog_tooLarge;` | `(size_t)-FSE_ERROR_tableLog_tooLarge` | [x] |
| 1611 | `FSE_readNCount` (c_src/src/legacy/zstd_v01.c:492) | `if (n0 > *maxSVPtr) return (size_t)-FSE_ERROR_maxSymbolValue_tooSmall;` | `(size_t)-FSE_ERROR_maxSymbolValue_tooSmall` | [x] |
| 1612 | `FSE_readNCount` (c_src/src/legacy/zstd_v01.c:544) | `if (remaining != 1) return (size_t)-FSE_ERROR_GENERIC;` | `(size_t)-FSE_ERROR_GENERIC` | [x] |
| 1613 | `FSE_readNCount` (c_src/src/legacy/zstd_v01.c:548) | `if ((size_t)(ip-istart) > hbSize) return (size_t)-FSE_ERROR_srcSize_wrong;` | `(size_t)-FSE_ERROR_srcSize_wrong` | [x] |
| 1614 | `FSE_buildDTable_raw` (c_src/src/legacy/zstd_v01.c:584) | `if (nbBits < 1) return (size_t)-FSE_ERROR_GENERIC; /* min size */` | `(size_t)-FSE_ERROR_GENERIC` | [x] |
| 1615 | `FSE_initDStream` (c_src/src/legacy/zstd_v01.c:608) | `if (srcSize < 1) return (size_t)-FSE_ERROR_srcSize_wrong;` | `(size_t)-FSE_ERROR_srcSize_wrong` | [x] |
| 1616 | `FSE_initDStream` (c_src/src/legacy/zstd_v01.c:617) | `if (contain32 == 0) return (size_t)-FSE_ERROR_GENERIC; /* stop bit not present */` | `(size_t)-FSE_ERROR_GENERIC` | [x] |
| 1617 | `FSE_initDStream` (c_src/src/legacy/zstd_v01.c:643) | `if (contain32 == 0) return (size_t)-FSE_ERROR_GENERIC; /* stop bit not present */` | `(size_t)-FSE_ERROR_GENERIC` | [x] |
| 1618 | `FSE_decompress_usingDTable_generic` (c_src/src/legacy/zstd_v01.c:840) | `if (op==omax) return (size_t)-FSE_ERROR_dstSize_tooSmall; /* dst buffer is full, but cSrc unfinished */` | `(size_t)-FSE_ERROR_dstSize_tooSmall` | [x] |
| 1619 | `FSE_decompress_usingDTable_generic` (c_src/src/legacy/zstd_v01.c:842) | `return (size_t)-FSE_ERROR_corruptionDetected;` | `(size_t)-FSE_ERROR_corruptionDetected` | [x] |
| 1620 | `FSE_decompress` (c_src/src/legacy/zstd_v01.c:869) | `if (cSrcSize<2) return (size_t)-FSE_ERROR_srcSize_wrong; /* too small input size */` | `(size_t)-FSE_ERROR_srcSize_wrong` | [x] |
| 1621 | `FSE_decompress` (c_src/src/legacy/zstd_v01.c:874) | `if (errorCode >= cSrcSize) return (size_t)-FSE_ERROR_srcSize_wrong; /* too small input size */` | `(size_t)-FSE_ERROR_srcSize_wrong` | [x] |
| 1622 | `HUF_readDTable` (c_src/src/legacy/zstd_v01.c:933) | `if (!srcSize) return (size_t)-FSE_ERROR_srcSize_wrong;` | `(size_t)-FSE_ERROR_srcSize_wrong` | [x] |
| 1623 | `HUF_readDTable` (c_src/src/legacy/zstd_v01.c:951) | `if (iSize+1 > srcSize) return (size_t)-FSE_ERROR_srcSize_wrong;` | `(size_t)-FSE_ERROR_srcSize_wrong` | [x] |
| 1624 | `HUF_readDTable` (c_src/src/legacy/zstd_v01.c:962) | `if (iSize+1 > srcSize) return (size_t)-FSE_ERROR_srcSize_wrong;` | `(size_t)-FSE_ERROR_srcSize_wrong` | [x] |
| 1625 | `HUF_readDTable` (c_src/src/legacy/zstd_v01.c:972) | `if (huffWeight[n] >= HUF_ABSOLUTEMAX_TABLELOG) return (size_t)-FSE_ERROR_corruptionDetected;` | `(size_t)-FSE_ERROR_corruptionDetected` | [x] |
| 1626 | `HUF_readDTable` (c_src/src/legacy/zstd_v01.c:976) | `if (weightTotal == 0) return (size_t)-FSE_ERROR_corruptionDetected;` | `(size_t)-FSE_ERROR_corruptionDetected` | [x] |
| 1627 | `HUF_readDTable` (c_src/src/legacy/zstd_v01.c:980) | `if (maxBits > DTable[0]) return (size_t)-FSE_ERROR_tableLog_tooLarge; /* DTable is too small */` | `(size_t)-FSE_ERROR_tableLog_tooLarge` | [x] |
| 1628 | `HUF_readDTable` (c_src/src/legacy/zstd_v01.c:987) | `if (verif != rest) return (size_t)-FSE_ERROR_corruptionDetected; /* last value must be a clean power of 2 */` | `(size_t)-FSE_ERROR_corruptionDetected` | [x] |
| 1629 | `HUF_readDTable` (c_src/src/legacy/zstd_v01.c:993) | `if ((rankVal[1] < 2) \|\| (rankVal[1] & 1)) return (size_t)-FSE_ERROR_corruptionDetected; /* by construction : at least 2 elts of rank 1, must be even */` | `(size_t)-FSE_ERROR_corruptionDetected` | [x] |
| 1630 | `HUF_decompress_usingDTable` (c_src/src/legacy/zstd_v01.c:1034) | `if (cSrcSize < 6) return (size_t)-FSE_ERROR_srcSize_wrong;` | `(size_t)-FSE_ERROR_srcSize_wrong` | [x] |
| 1631 | `HUF_decompress_usingDTable` (c_src/src/legacy/zstd_v01.c:1060) | `if (length1+length2+length3+6 >= cSrcSize) return (size_t)-FSE_ERROR_srcSize_wrong;` | `(size_t)-FSE_ERROR_srcSize_wrong` | [x] |
| 1632 | `HUF_decompress_usingDTable` (c_src/src/legacy/zstd_v01.c:1107) | `return (size_t)-FSE_ERROR_corruptionDetected;` | `(size_t)-FSE_ERROR_corruptionDetected` | [x] |
| 1633 | `HUF_decompress_usingDTable` (c_src/src/legacy/zstd_v01.c:1126) | `if (op==omax) return (size_t)-FSE_ERROR_dstSize_tooSmall; /* dst buffer is full, but cSrc unfinished */` | `(size_t)-FSE_ERROR_dstSize_tooSmall` | [x] |
| 1634 | `HUF_decompress_usingDTable` (c_src/src/legacy/zstd_v01.c:1128) | `return (size_t)-FSE_ERROR_corruptionDetected;` | `(size_t)-FSE_ERROR_corruptionDetected` | [x] |
| 1635 | `HUF_decompress` (c_src/src/legacy/zstd_v01.c:1141) | `if (errorCode >= cSrcSize) return (size_t)-FSE_ERROR_srcSize_wrong;` | `(size_t)-FSE_ERROR_srcSize_wrong` | [x] |
| 1636 | `ZSTDv01_getcBlockSize` (c_src/src/legacy/zstd_v01.c:1431) | `if (srcSize < 3) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1637 | `ZSTD_copyUncompressedBlock` (c_src/src/legacy/zstd_v01.c:1447) | `if (srcSize > maxDstSize) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 1638 | `ZSTD_decompressLiterals` (c_src/src/legacy/zstd_v01.c:1466) | `if (srcSize <= 3) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1639 | `ZSTD_decompressLiterals` (c_src/src/legacy/zstd_v01.c:1473) | `if (litSize > maxDstSize) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 1640 | `ZSTD_decompressLiterals` (c_src/src/legacy/zstd_v01.c:1475) | `if (FSE_isError(errorCode)) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [x] |
| 1641 | `ZSTDv01_decodeLiteralsBlock` (c_src/src/legacy/zstd_v01.c:1493) | `if (litcSize > srcSize - ZSTD_blockHeaderSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1642 | `ZSTDv01_decodeLiteralsBlock` (c_src/src/legacy/zstd_v01.c:1506) | `if (rleSize>maxDstSize) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 1643 | `ZSTDv01_decodeLiteralsBlock` (c_src/src/legacy/zstd_v01.c:1507) | `if (!srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1644 | `ZSTDv01_decodeLiteralsBlock` (c_src/src/legacy/zstd_v01.c:1527) | `return ERROR(GENERIC);` | `ERROR(GENERIC)` | [x] |
| 1645 | `ZSTDv01_decodeSeqHeaders` (c_src/src/legacy/zstd_v01.c:1546) | `if (srcSize < 5) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1646 | `ZSTDv01_decodeSeqHeaders` (c_src/src/legacy/zstd_v01.c:1570) | `if (ip > iend-3) return ERROR(srcSize_wrong); /* min : all 3 are "raw", hence no header, but at least xxLog bits per type */` | `ERROR(srcSize_wrong)` | [x] |
| 1647 | `ZSTDv01_decodeSeqHeaders` (c_src/src/legacy/zstd_v01.c:1589) | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [x] |
| 1648 | `ZSTDv01_decodeSeqHeaders` (c_src/src/legacy/zstd_v01.c:1590) | `if (LLlog > LLFSELog) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1649 | `ZSTDv01_decodeSeqHeaders` (c_src/src/legacy/zstd_v01.c:1599) | `if (ip > iend-2) return ERROR(srcSize_wrong); /* min : "raw", hence no header, but at least xxLog bits */` | `ERROR(srcSize_wrong)` | [x] |
| 1650 | `ZSTDv01_decodeSeqHeaders` (c_src/src/legacy/zstd_v01.c:1607) | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [x] |
| 1651 | `ZSTDv01_decodeSeqHeaders` (c_src/src/legacy/zstd_v01.c:1608) | `if (Offlog > OffFSELog) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1652 | `ZSTDv01_decodeSeqHeaders` (c_src/src/legacy/zstd_v01.c:1617) | `if (ip > iend-2) return ERROR(srcSize_wrong); /* min : "raw", hence no header, but at least xxLog bits */` | `ERROR(srcSize_wrong)` | [x] |
| 1653 | `ZSTDv01_decodeSeqHeaders` (c_src/src/legacy/zstd_v01.c:1625) | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [x] |
| 1654 | `ZSTDv01_decodeSeqHeaders` (c_src/src/legacy/zstd_v01.c:1626) | `if (MLlog > MLFSELog) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1655 | `ZSTD_execSequence` (c_src/src/legacy/zstd_v01.c:1732) | `if (seqLength > (size_t)(oend - op)) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 1656 | `ZSTD_execSequence` (c_src/src/legacy/zstd_v01.c:1733) | `if (sequence.litLength > (size_t)(litLimit - *litPtr)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1657 | `ZSTD_execSequence` (c_src/src/legacy/zstd_v01.c:1735) | `if (sequence.offset > (U32)(oLitEnd - base)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1658 | `ZSTD_execSequence` (c_src/src/legacy/zstd_v01.c:1737) | `if (endMatch > oend) return ERROR(dstSize_tooSmall); /* overwrite beyond dst buffer */` | `ERROR(dstSize_tooSmall)` | [x] |
| 1659 | `ZSTD_execSequence` (c_src/src/legacy/zstd_v01.c:1738) | `if (litEnd > litLimit) return ERROR(corruption_detected); /* overRead beyond lit buffer */` | `ERROR(corruption_detected)` | [x] |
| 1660 | `ZSTD_execSequence` (c_src/src/legacy/zstd_v01.c:1739) | `if (sequence.matchLength > (size_t)(*litPtr-op)) return ERROR(dstSize_tooSmall); /* overwrite literal segment */` | `ERROR(dstSize_tooSmall)` | [x] |
| 1661 | `ZSTD_execSequence` (c_src/src/legacy/zstd_v01.c:1748) | `if (oend-op < 8) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 1662 | `ZSTD_execSequence` (c_src/src/legacy/zstd_v01.c:1758) | `if (match < base) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1663 | `ZSTD_execSequence` (c_src/src/legacy/zstd_v01.c:1759) | `if (sequence.offset > (size_t)base) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1664 | `ZSTD_decompressSequences` (c_src/src/legacy/zstd_v01.c:1853) | `if (FSE_isError(errorCode)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1665 | `ZSTD_decompressSequences` (c_src/src/legacy/zstd_v01.c:1869) | `if ( !FSE_endOfDStream(&(seqState.DStream)) ) return ERROR(corruption_detected); /* requested too much : data is corrupted */` | `ERROR(corruption_detected)` | [x] |
| 1666 | `ZSTD_decompressSequences` (c_src/src/legacy/zstd_v01.c:1870) | `if (nbSeq<0) return ERROR(corruption_detected); /* requested too many sequences : data is corrupted */` | `ERROR(corruption_detected)` | [x] |
| 1667 | `ZSTD_decompressSequences` (c_src/src/legacy/zstd_v01.c:1875) | `if (op+lastLLSize > oend) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 1668 | `ZSTDv01_decompressDCtx` (c_src/src/legacy/zstd_v01.c:1921) | `if (srcSize < ZSTD_frameHeaderSize+ZSTD_blockHeaderSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1669 | `ZSTDv01_decompressDCtx` (c_src/src/legacy/zstd_v01.c:1923) | `if (magicNumber != ZSTD_magicNumber) return ERROR(prefix_unknown);` | `ERROR(prefix_unknown)` | [x] |
| 1670 | `ZSTDv01_decompressDCtx` (c_src/src/legacy/zstd_v01.c:1934) | `if (blockSize > remainingSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1671 | `ZSTDv01_decompressDCtx` (c_src/src/legacy/zstd_v01.c:1945) | `return ERROR(GENERIC); /* not yet supported */` | `ERROR(GENERIC)` | [x] |
| 1672 | `ZSTDv01_decompressDCtx` (c_src/src/legacy/zstd_v01.c:1949) | `if (remainingSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1673 | `ZSTDv01_decompressDCtx` (c_src/src/legacy/zstd_v01.c:1952) | `return ERROR(GENERIC);` | `ERROR(GENERIC)` | [x] |
| 1674 | `ZSTDv01_createDCtx` (c_src/src/legacy/zstd_v01.c:2043) | `if (dctx==NULL) return NULL;` | `NULL` | [x] |
| 1675 | `ZSTDv01_decompressContinue` (c_src/src/legacy/zstd_v01.c:2064) | `if (srcSize != ctx->expected) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1676 | `ZSTDv01_decompressContinue` (c_src/src/legacy/zstd_v01.c:2073) | `if (magicNumber != ZSTD_magicNumber) return ERROR(prefix_unknown);` | `ERROR(prefix_unknown)` | [x] |
| 1677 | `ZSTDv01_decompressContinue` (c_src/src/legacy/zstd_v01.c:2112) | `return ERROR(GENERIC); /* not yet handled */` | `ERROR(GENERIC)` | [x] |
| 1678 | `ZSTDv01_decompressContinue` (c_src/src/legacy/zstd_v01.c:2118) | `return ERROR(GENERIC);` | `ERROR(GENERIC)` | [x] |
| 1679 | `BIT_initDStream` (c_src/src/legacy/zstd_v02.c:325) | `if (srcSize < 1) { memset(bitD, 0, sizeof(*bitD)); return ERROR(srcSize_wrong); }` | `ERROR(srcSize_wrong)` | [x] |
| 1680 | `BIT_initDStream` (c_src/src/legacy/zstd_v02.c:334) | `if (contain32 == 0) return ERROR(GENERIC); /* endMark not present */` | `ERROR(GENERIC)` | [x] |
| 1681 | `BIT_initDStream` (c_src/src/legacy/zstd_v02.c:360) | `if (contain32 == 0) return ERROR(GENERIC); /* endMark not present */` | `ERROR(GENERIC)` | [x] |
| 1682 | `ERR_isError` (c_src/src/legacy/zstd_v02.c:524) | `ERR_STATIC unsigned ERR_isError(size_t code) { return (code > ERROR(maxCode)); }` | `ERROR(maxCode)` | [x] |
| 1683 | `FSE_buildDTable` (c_src/src/legacy/zstd_v02.c:1051) | `if (maxSymbolValue > FSE_MAX_SYMBOL_VALUE) return ERROR(maxSymbolValue_tooLarge);` | `ERROR(maxSymbolValue_tooLarge)` | [x] |
| 1684 | `FSE_buildDTable` (c_src/src/legacy/zstd_v02.c:1052) | `if (tableLog > FSE_MAX_TABLELOG) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [x] |
| 1685 | `FSE_buildDTable` (c_src/src/legacy/zstd_v02.c:1082) | `if (position!=0) return ERROR(GENERIC); /* position must reach all cells once, otherwise normalizedCounter is incorrect */` | `ERROR(GENERIC)` | [x] |
| 1686 | `FSE_readNCount` (c_src/src/legacy/zstd_v02.c:1131) | `if (hbSize < 4) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1687 | `FSE_readNCount` (c_src/src/legacy/zstd_v02.c:1134) | `if (nbBits > FSE_TABLELOG_ABSOLUTE_MAX) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [x] |
| 1688 | `FSE_readNCount` (c_src/src/legacy/zstd_v02.c:1169) | `if (n0 > *maxSVPtr) return ERROR(maxSymbolValue_tooSmall);` | `ERROR(maxSymbolValue_tooSmall)` | [x] |
| 1689 | `FSE_readNCount` (c_src/src/legacy/zstd_v02.c:1221) | `if (remaining != 1) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [x] |
| 1690 | `FSE_readNCount` (c_src/src/legacy/zstd_v02.c:1225) | `if ((size_t)(ip-istart) > hbSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1691 | `FSE_buildDTable_raw` (c_src/src/legacy/zstd_v02.c:1261) | `if (nbBits < 1) return ERROR(GENERIC); /* min size */` | `ERROR(GENERIC)` | [x] |
| 1692 | `FSE_decompress_usingDTable_generic` (c_src/src/legacy/zstd_v02.c:1340) | `if (op==omax) return ERROR(dstSize_tooSmall); /* dst buffer is full, but cSrc unfinished */` | `ERROR(dstSize_tooSmall)` | [x] |
| 1693 | `FSE_decompress_usingDTable_generic` (c_src/src/legacy/zstd_v02.c:1342) | `return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1694 | `FSE_decompress` (c_src/src/legacy/zstd_v02.c:1369) | `if (cSrcSize<2) return ERROR(srcSize_wrong); /* too small input size */` | `ERROR(srcSize_wrong)` | [x] |
| 1695 | `FSE_decompress` (c_src/src/legacy/zstd_v02.c:1374) | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong); /* too small input size */` | `ERROR(srcSize_wrong)` | [x] |
| 1696 | `HUF_readStats` (c_src/src/legacy/zstd_v02.c:1492) | `if (!srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1697 | `HUF_readStats` (c_src/src/legacy/zstd_v02.c:1509) | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1698 | `HUF_readStats` (c_src/src/legacy/zstd_v02.c:1510) | `if (oSize >= hwSize) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1699 | `HUF_readStats` (c_src/src/legacy/zstd_v02.c:1521) | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1700 | `HUF_readStats` (c_src/src/legacy/zstd_v02.c:1531) | `if (huffWeight[n] >= HUF_ABSOLUTEMAX_TABLELOG) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1701 | `HUF_readStats` (c_src/src/legacy/zstd_v02.c:1535) | `if (weightTotal == 0) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1702 | `HUF_readStats` (c_src/src/legacy/zstd_v02.c:1539) | `if (tableLog > HUF_ABSOLUTEMAX_TABLELOG) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1703 | `HUF_readStats` (c_src/src/legacy/zstd_v02.c:1545) | `if (verif != rest) return ERROR(corruption_detected); /* last value must be a clean power of 2 */` | `ERROR(corruption_detected)` | [x] |
| 1704 | `HUF_readStats` (c_src/src/legacy/zstd_v02.c:1551) | `if ((rankStats[1] < 2) \|\| (rankStats[1] & 1)) return ERROR(corruption_detected); /* by construction : at least 2 elts of rank 1, must be even */` | `ERROR(corruption_detected)` | [x] |
| 1705 | `HUF_readDTableX2` (c_src/src/legacy/zstd_v02.c:1584) | `if (tableLog > DTable[0]) return ERROR(tableLog_tooLarge); /* DTable is too small */` | `ERROR(tableLog_tooLarge)` | [x] |
| 1706 | `HUF_decompress4X2_usingDTable` (c_src/src/legacy/zstd_v02.c:1661) | `if (cSrcSize < 10) return ERROR(corruption_detected); /* strict minimum : jump table + 1 byte per stream */` | `ERROR(corruption_detected)` | [x] |
| 1707 | `HUF_decompress4X2_usingDTable` (c_src/src/legacy/zstd_v02.c:1697) | `if (length4 > cSrcSize) return ERROR(corruption_detected); /* overflow */` | `ERROR(corruption_detected)` | [x] |
| 1708 | `HUF_decompress4X2_usingDTable` (c_src/src/legacy/zstd_v02.c:1732) | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1709 | `HUF_decompress4X2_usingDTable` (c_src/src/legacy/zstd_v02.c:1733) | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1710 | `HUF_decompress4X2_usingDTable` (c_src/src/legacy/zstd_v02.c:1734) | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1711 | `HUF_decompress4X2_usingDTable` (c_src/src/legacy/zstd_v02.c:1745) | `if (!endSignal) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1712 | `HUF_decompress4X2` (c_src/src/legacy/zstd_v02.c:1761) | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1713 | `HUF_readDTableX4` (c_src/src/legacy/zstd_v02.c:1882) | `if (memLog > HUF_ABSOLUTEMAX_TABLELOG) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [x] |
| 1714 | `HUF_readDTableX4` (c_src/src/legacy/zstd_v02.c:1889) | `if (tableLog > memLog) return ERROR(tableLog_tooLarge); /* DTable can't fit code depth */` | `ERROR(tableLog_tooLarge)` | [x] |
| 1715 | `HUF_readDTableX4` (c_src/src/legacy/zstd_v02.c:1893) | `{if (!maxW) return ERROR(GENERIC); } /* necessarily finds a solution before maxW==0 */` | `ERROR(GENERIC)` | [x] |
| 1716 | `HUF_decompress4X4_usingDTable` (c_src/src/legacy/zstd_v02.c:2023) | `if (cSrcSize < 10) return ERROR(corruption_detected); /* strict minimum : jump table + 1 byte per stream */` | `ERROR(corruption_detected)` | [x] |
| 1717 | `HUF_decompress4X4_usingDTable` (c_src/src/legacy/zstd_v02.c:2059) | `if (length4 > cSrcSize) return ERROR(corruption_detected); /* overflow */` | `ERROR(corruption_detected)` | [x] |
| 1718 | `HUF_decompress4X4_usingDTable` (c_src/src/legacy/zstd_v02.c:2094) | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1719 | `HUF_decompress4X4_usingDTable` (c_src/src/legacy/zstd_v02.c:2095) | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1720 | `HUF_decompress4X4_usingDTable` (c_src/src/legacy/zstd_v02.c:2096) | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1721 | `HUF_decompress4X4_usingDTable` (c_src/src/legacy/zstd_v02.c:2107) | `if (!endSignal) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1722 | `HUF_decompress4X4` (c_src/src/legacy/zstd_v02.c:2122) | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1723 | `HUF_readDTableX6` (c_src/src/legacy/zstd_v02.c:2215) | `if (memLog > HUF_ABSOLUTEMAX_TABLELOG) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [x] |
| 1724 | `HUF_readDTableX6` (c_src/src/legacy/zstd_v02.c:2222) | `if (tableLog > memLog) return ERROR(tableLog_tooLarge); /* DTable is too small */` | `ERROR(tableLog_tooLarge)` | [x] |
| 1725 | `HUF_readDTableX6` (c_src/src/legacy/zstd_v02.c:2226) | `{ if (!maxW) return ERROR(GENERIC); } /* necessarily finds a solution before maxW==0 */` | `ERROR(GENERIC)` | [x] |
| 1726 | `HUF_decompress4X6_usingDTable` (c_src/src/legacy/zstd_v02.c:2378) | `if (cSrcSize < 10) return ERROR(corruption_detected); /* strict minimum : jump table + 1 byte per stream */` | `ERROR(corruption_detected)` | [x] |
| 1727 | `HUF_decompress4X6_usingDTable` (c_src/src/legacy/zstd_v02.c:2416) | `if (length4 > cSrcSize) return ERROR(corruption_detected); /* overflow */` | `ERROR(corruption_detected)` | [x] |
| 1728 | `HUF_decompress4X6_usingDTable` (c_src/src/legacy/zstd_v02.c:2451) | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1729 | `HUF_decompress4X6_usingDTable` (c_src/src/legacy/zstd_v02.c:2452) | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1730 | `HUF_decompress4X6_usingDTable` (c_src/src/legacy/zstd_v02.c:2453) | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1731 | `HUF_decompress4X6_usingDTable` (c_src/src/legacy/zstd_v02.c:2464) | `if (!endSignal) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1732 | `HUF_decompress4X6` (c_src/src/legacy/zstd_v02.c:2479) | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1733 | `HUF_decompress` (c_src/src/legacy/zstd_v02.c:2526) | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 1734 | `HUF_decompress` (c_src/src/legacy/zstd_v02.c:2527) | `if (cSrcSize > dstSize) return ERROR(corruption_detected); /* invalid */` | `ERROR(corruption_detected)` | [x] |
| 1735 | `ZSTD_getcBlockSize` (c_src/src/legacy/zstd_v02.c:2762) | `if (srcSize < 3) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1736 | `ZSTD_copyUncompressedBlock` (c_src/src/legacy/zstd_v02.c:2777) | `if (srcSize > maxDstSize) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 1737 | `ZSTD_decompressLiterals` (c_src/src/legacy/zstd_v02.c:2795) | `if (litSize > *maxDstSizePtr) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1738 | `ZSTD_decompressLiterals` (c_src/src/legacy/zstd_v02.c:2796) | `if (litCSize + 5 > srcSize) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1739 | `ZSTD_decompressLiterals` (c_src/src/legacy/zstd_v02.c:2798) | `if (HUF_isError(HUF_decompress(dst, litSize, ip+5, litCSize))) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1740 | `ZSTD_decodeLiteralsBlock` (c_src/src/legacy/zstd_v02.c:2814) | `if (srcSize < MIN_CBLOCK_SIZE) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1741 | `ZSTD_decodeLiteralsBlock` (c_src/src/legacy/zstd_v02.c:2833) | `if (litSize > BLOCKSIZE) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1742 | `ZSTD_decodeLiteralsBlock` (c_src/src/legacy/zstd_v02.c:2834) | `if (litSize > srcSize-3) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1743 | `ZSTD_decodeLiteralsBlock` (c_src/src/legacy/zstd_v02.c:2849) | `if (litSize > BLOCKSIZE) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1744 | `ZSTD_decodeSeqHeaders` (c_src/src/legacy/zstd_v02.c:2871) | `if (srcSize < 5) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1745 | `ZSTD_decodeSeqHeaders` (c_src/src/legacy/zstd_v02.c:2895) | `if (ip > iend-3) return ERROR(srcSize_wrong); /* min : all 3 are "raw", hence no header, but at least xxLog bits per type */` | `ERROR(srcSize_wrong)` | [x] |
| 1746 | `ZSTD_decodeSeqHeaders` (c_src/src/legacy/zstd_v02.c:2914) | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [x] |
| 1747 | `ZSTD_decodeSeqHeaders` (c_src/src/legacy/zstd_v02.c:2915) | `if (LLlog > LLFSELog) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1748 | `ZSTD_decodeSeqHeaders` (c_src/src/legacy/zstd_v02.c:2924) | `if (ip > iend-2) return ERROR(srcSize_wrong); /* min : "raw", hence no header, but at least xxLog bits */` | `ERROR(srcSize_wrong)` | [x] |
| 1749 | `ZSTD_decodeSeqHeaders` (c_src/src/legacy/zstd_v02.c:2933) | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [x] |
| 1750 | `ZSTD_decodeSeqHeaders` (c_src/src/legacy/zstd_v02.c:2934) | `if (Offlog > OffFSELog) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1751 | `ZSTD_decodeSeqHeaders` (c_src/src/legacy/zstd_v02.c:2943) | `if (ip > iend-2) return ERROR(srcSize_wrong); /* min : "raw", hence no header, but at least xxLog bits */` | `ERROR(srcSize_wrong)` | [x] |
| 1752 | `ZSTD_decodeSeqHeaders` (c_src/src/legacy/zstd_v02.c:2951) | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [x] |
| 1753 | `ZSTD_decodeSeqHeaders` (c_src/src/legacy/zstd_v02.c:2952) | `if (MLlog > MLFSELog) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1754 | `ZSTD_execSequence` (c_src/src/legacy/zstd_v02.c:3058) | `if (seqLength > (size_t)(oend - op)) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 1755 | `ZSTD_execSequence` (c_src/src/legacy/zstd_v02.c:3059) | `if (sequence.litLength > (size_t)(litLimit - *litPtr)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1756 | `ZSTD_execSequence` (c_src/src/legacy/zstd_v02.c:3061) | `if (oLitEnd > oend_8) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 1757 | `ZSTD_execSequence` (c_src/src/legacy/zstd_v02.c:3062) | `if (sequence.offset > (U32)(oLitEnd - base)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1758 | `ZSTD_execSequence` (c_src/src/legacy/zstd_v02.c:3064) | `if (oMatchEnd > oend) return ERROR(dstSize_tooSmall); /* overwrite beyond dst buffer */` | `ERROR(dstSize_tooSmall)` | [x] |
| 1759 | `ZSTD_execSequence` (c_src/src/legacy/zstd_v02.c:3065) | `if (litEnd > litLimit) return ERROR(corruption_detected); /* overRead beyond lit buffer */` | `ERROR(corruption_detected)` | [x] |
| 1760 | `ZSTD_execSequence` (c_src/src/legacy/zstd_v02.c:3077) | `if (sequence.offset > (size_t)op) return ERROR(corruption_detected); /* address space overflow test (this test seems kept by clang optimizer) */` | `ERROR(corruption_detected)` | [x] |
| 1761 | `ZSTD_execSequence` (c_src/src/legacy/zstd_v02.c:3078) | `//if (match > op) return ERROR(corruption_detected); /* address space overflow test (is clang optimizer removing this test ?) */` | `ERROR(corruption_detected)` | [x] |
| 1762 | `ZSTD_execSequence` (c_src/src/legacy/zstd_v02.c:3079) | `if (match < base) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1763 | `ZSTD_decompressSequences` (c_src/src/legacy/zstd_v02.c:3156) | `if (ERR_isError(errorCode)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1764 | `ZSTD_decompressSequences` (c_src/src/legacy/zstd_v02.c:3172) | `if ( !BIT_endOfDStream(&(seqState.DStream)) ) return ERROR(corruption_detected); /* requested too much : data is corrupted */` | `ERROR(corruption_detected)` | [x] |
| 1765 | `ZSTD_decompressSequences` (c_src/src/legacy/zstd_v02.c:3173) | `if (nbSeq<0) return ERROR(corruption_detected); /* requested too many sequences : data is corrupted */` | `ERROR(corruption_detected)` | [x] |
| 1766 | `ZSTD_decompressSequences` (c_src/src/legacy/zstd_v02.c:3178) | `if (litPtr > litEnd) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1767 | `ZSTD_decompressSequences` (c_src/src/legacy/zstd_v02.c:3179) | `if (op+lastLLSize > oend) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 1768 | `ZSTD_decompressDCtx` (c_src/src/legacy/zstd_v02.c:3221) | `if (srcSize < ZSTD_frameHeaderSize+ZSTD_blockHeaderSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1769 | `ZSTD_decompressDCtx` (c_src/src/legacy/zstd_v02.c:3223) | `if (magicNumber != ZSTD_magicNumber) return ERROR(prefix_unknown);` | `ERROR(prefix_unknown)` | [x] |
| 1770 | `ZSTD_decompressDCtx` (c_src/src/legacy/zstd_v02.c:3235) | `if (cBlockSize > remainingSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1771 | `ZSTD_decompressDCtx` (c_src/src/legacy/zstd_v02.c:3246) | `return ERROR(GENERIC); /* not yet supported */` | `ERROR(GENERIC)` | [x] |
| 1772 | `ZSTD_decompressDCtx` (c_src/src/legacy/zstd_v02.c:3250) | `if (remainingSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1773 | `ZSTD_decompressDCtx` (c_src/src/legacy/zstd_v02.c:3253) | `return ERROR(GENERIC); /* impossible */` | `ERROR(GENERIC)` | [x] |
| 1774 | `ZSTD_createDCtx` (c_src/src/legacy/zstd_v02.c:3344) | `if (dctx==NULL) return NULL;` | `NULL` | [x] |
| 1775 | `ZSTD_decompressContinue` (c_src/src/legacy/zstd_v02.c:3363) | `if (srcSize != ctx->expected) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1776 | `ZSTD_decompressContinue` (c_src/src/legacy/zstd_v02.c:3372) | `if (magicNumber != ZSTD_magicNumber) return ERROR(prefix_unknown);` | `ERROR(prefix_unknown)` | [x] |
| 1777 | `ZSTD_decompressContinue` (c_src/src/legacy/zstd_v02.c:3411) | `return ERROR(GENERIC); /* not yet handled */` | `ERROR(GENERIC)` | [x] |
| 1778 | `ZSTD_decompressContinue` (c_src/src/legacy/zstd_v02.c:3417) | `return ERROR(GENERIC);` | `ERROR(GENERIC)` | [x] |
| 1779 | `BIT_initDStream` (c_src/src/legacy/zstd_v03.c:327) | `if (srcSize < 1) { memset(bitD, 0, sizeof(*bitD)); return ERROR(srcSize_wrong); }` | `ERROR(srcSize_wrong)` | [x] |
| 1780 | `BIT_initDStream` (c_src/src/legacy/zstd_v03.c:336) | `if (contain32 == 0) return ERROR(GENERIC); /* endMark not present */` | `ERROR(GENERIC)` | [x] |
| 1781 | `BIT_initDStream` (c_src/src/legacy/zstd_v03.c:362) | `if (contain32 == 0) return ERROR(GENERIC); /* endMark not present */` | `ERROR(GENERIC)` | [x] |
| 1782 | `ERR_isError` (c_src/src/legacy/zstd_v03.c:525) | `ERR_STATIC unsigned ERR_isError(size_t code) { return (code > ERROR(maxCode)); }` | `ERROR(maxCode)` | [x] |
| 1783 | `FSE_buildDTable` (c_src/src/legacy/zstd_v03.c:1051) | `if (maxSymbolValue > FSE_MAX_SYMBOL_VALUE) return ERROR(maxSymbolValue_tooLarge);` | `ERROR(maxSymbolValue_tooLarge)` | [x] |
| 1784 | `FSE_buildDTable` (c_src/src/legacy/zstd_v03.c:1052) | `if (tableLog > FSE_MAX_TABLELOG) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [x] |
| 1785 | `FSE_buildDTable` (c_src/src/legacy/zstd_v03.c:1082) | `if (position!=0) return ERROR(GENERIC); /* position must reach all cells once, otherwise normalizedCounter is incorrect */` | `ERROR(GENERIC)` | [x] |
| 1786 | `FSE_readNCount` (c_src/src/legacy/zstd_v03.c:1131) | `if (hbSize < 4) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1787 | `FSE_readNCount` (c_src/src/legacy/zstd_v03.c:1134) | `if (nbBits > FSE_TABLELOG_ABSOLUTE_MAX) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [x] |
| 1788 | `FSE_readNCount` (c_src/src/legacy/zstd_v03.c:1169) | `if (n0 > *maxSVPtr) return ERROR(maxSymbolValue_tooSmall);` | `ERROR(maxSymbolValue_tooSmall)` | [x] |
| 1789 | `FSE_readNCount` (c_src/src/legacy/zstd_v03.c:1221) | `if (remaining != 1) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [x] |
| 1790 | `FSE_readNCount` (c_src/src/legacy/zstd_v03.c:1225) | `if ((size_t)(ip-istart) > hbSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1791 | `FSE_buildDTable_raw` (c_src/src/legacy/zstd_v03.c:1261) | `if (nbBits < 1) return ERROR(GENERIC); /* min size */` | `ERROR(GENERIC)` | [x] |
| 1792 | `FSE_decompress_usingDTable_generic` (c_src/src/legacy/zstd_v03.c:1340) | `if (op==omax) return ERROR(dstSize_tooSmall); /* dst buffer is full, but cSrc unfinished */` | `ERROR(dstSize_tooSmall)` | [x] |
| 1793 | `FSE_decompress_usingDTable_generic` (c_src/src/legacy/zstd_v03.c:1342) | `return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1794 | `FSE_decompress` (c_src/src/legacy/zstd_v03.c:1369) | `if (cSrcSize<2) return ERROR(srcSize_wrong); /* too small input size */` | `ERROR(srcSize_wrong)` | [x] |
| 1795 | `FSE_decompress` (c_src/src/legacy/zstd_v03.c:1374) | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong); /* too small input size */` | `ERROR(srcSize_wrong)` | [x] |
| 1796 | `HUF_readStats` (c_src/src/legacy/zstd_v03.c:1488) | `if (!srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1797 | `HUF_readStats` (c_src/src/legacy/zstd_v03.c:1505) | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1798 | `HUF_readStats` (c_src/src/legacy/zstd_v03.c:1506) | `if (oSize >= hwSize) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1799 | `HUF_readStats` (c_src/src/legacy/zstd_v03.c:1517) | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1800 | `HUF_readStats` (c_src/src/legacy/zstd_v03.c:1527) | `if (huffWeight[n] >= HUF_ABSOLUTEMAX_TABLELOG) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1801 | `HUF_readStats` (c_src/src/legacy/zstd_v03.c:1531) | `if (weightTotal == 0) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1802 | `HUF_readStats` (c_src/src/legacy/zstd_v03.c:1535) | `if (tableLog > HUF_ABSOLUTEMAX_TABLELOG) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1803 | `HUF_readStats` (c_src/src/legacy/zstd_v03.c:1541) | `if (verif != rest) return ERROR(corruption_detected); /* last value must be a clean power of 2 */` | `ERROR(corruption_detected)` | [x] |
| 1804 | `HUF_readStats` (c_src/src/legacy/zstd_v03.c:1547) | `if ((rankStats[1] < 2) \|\| (rankStats[1] & 1)) return ERROR(corruption_detected); /* by construction : at least 2 elts of rank 1, must be even */` | `ERROR(corruption_detected)` | [x] |
| 1805 | `HUF_readDTableX2` (c_src/src/legacy/zstd_v03.c:1580) | `if (tableLog > DTable[0]) return ERROR(tableLog_tooLarge); /* DTable is too small */` | `ERROR(tableLog_tooLarge)` | [x] |
| 1806 | `HUF_decompress4X2_usingDTable` (c_src/src/legacy/zstd_v03.c:1657) | `if (cSrcSize < 10) return ERROR(corruption_detected); /* strict minimum : jump table + 1 byte per stream */` | `ERROR(corruption_detected)` | [x] |
| 1807 | `HUF_decompress4X2_usingDTable` (c_src/src/legacy/zstd_v03.c:1693) | `if (length4 > cSrcSize) return ERROR(corruption_detected); /* overflow */` | `ERROR(corruption_detected)` | [x] |
| 1808 | `HUF_decompress4X2_usingDTable` (c_src/src/legacy/zstd_v03.c:1728) | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1809 | `HUF_decompress4X2_usingDTable` (c_src/src/legacy/zstd_v03.c:1729) | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1810 | `HUF_decompress4X2_usingDTable` (c_src/src/legacy/zstd_v03.c:1730) | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1811 | `HUF_decompress4X2_usingDTable` (c_src/src/legacy/zstd_v03.c:1741) | `if (!endSignal) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1812 | `HUF_decompress4X2` (c_src/src/legacy/zstd_v03.c:1757) | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1813 | `HUF_readDTableX4` (c_src/src/legacy/zstd_v03.c:1878) | `if (memLog > HUF_ABSOLUTEMAX_TABLELOG) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [x] |
| 1814 | `HUF_readDTableX4` (c_src/src/legacy/zstd_v03.c:1885) | `if (tableLog > memLog) return ERROR(tableLog_tooLarge); /* DTable can't fit code depth */` | `ERROR(tableLog_tooLarge)` | [x] |
| 1815 | `HUF_readDTableX4` (c_src/src/legacy/zstd_v03.c:1889) | `{ if (!maxW) return ERROR(GENERIC); } /* necessarily finds a solution before maxW==0 */` | `ERROR(GENERIC)` | [x] |
| 1816 | `HUF_decompress4X4_usingDTable` (c_src/src/legacy/zstd_v03.c:2019) | `if (cSrcSize < 10) return ERROR(corruption_detected); /* strict minimum : jump table + 1 byte per stream */` | `ERROR(corruption_detected)` | [x] |
| 1817 | `HUF_decompress4X4_usingDTable` (c_src/src/legacy/zstd_v03.c:2055) | `if (length4 > cSrcSize) return ERROR(corruption_detected); /* overflow */` | `ERROR(corruption_detected)` | [x] |
| 1818 | `HUF_decompress4X4_usingDTable` (c_src/src/legacy/zstd_v03.c:2090) | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1819 | `HUF_decompress4X4_usingDTable` (c_src/src/legacy/zstd_v03.c:2091) | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1820 | `HUF_decompress4X4_usingDTable` (c_src/src/legacy/zstd_v03.c:2092) | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1821 | `HUF_decompress4X4_usingDTable` (c_src/src/legacy/zstd_v03.c:2103) | `if (!endSignal) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1822 | `HUF_decompress4X4` (c_src/src/legacy/zstd_v03.c:2118) | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1823 | `HUF_decompress` (c_src/src/legacy/zstd_v03.c:2165) | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 1824 | `HUF_decompress` (c_src/src/legacy/zstd_v03.c:2166) | `if (cSrcSize > dstSize) return ERROR(corruption_detected); /* invalid */` | `ERROR(corruption_detected)` | [x] |
| 1825 | `ZSTD_getcBlockSize` (c_src/src/legacy/zstd_v03.c:2402) | `if (srcSize < 3) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1826 | `ZSTD_copyUncompressedBlock` (c_src/src/legacy/zstd_v03.c:2417) | `if (srcSize > maxDstSize) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 1827 | `ZSTD_decompressLiterals` (c_src/src/legacy/zstd_v03.c:2435) | `if (litSize > *maxDstSizePtr) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1828 | `ZSTD_decompressLiterals` (c_src/src/legacy/zstd_v03.c:2436) | `if (litCSize + 5 > srcSize) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1829 | `ZSTD_decompressLiterals` (c_src/src/legacy/zstd_v03.c:2438) | `if (HUF_isError(HUF_decompress(dst, litSize, ip+5, litCSize))) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1830 | `ZSTD_decodeLiteralsBlock` (c_src/src/legacy/zstd_v03.c:2454) | `if (srcSize < MIN_CBLOCK_SIZE) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1831 | `ZSTD_decodeLiteralsBlock` (c_src/src/legacy/zstd_v03.c:2473) | `if (litSize > BLOCKSIZE) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1832 | `ZSTD_decodeLiteralsBlock` (c_src/src/legacy/zstd_v03.c:2474) | `if (litSize > srcSize-3) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1833 | `ZSTD_decodeLiteralsBlock` (c_src/src/legacy/zstd_v03.c:2489) | `if (litSize > BLOCKSIZE) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1834 | `ZSTD_decodeSeqHeaders` (c_src/src/legacy/zstd_v03.c:2511) | `if (srcSize < 5) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1835 | `ZSTD_decodeSeqHeaders` (c_src/src/legacy/zstd_v03.c:2535) | `if (ip > iend-3) return ERROR(srcSize_wrong); /* min : all 3 are "raw", hence no header, but at least xxLog bits per type */` | `ERROR(srcSize_wrong)` | [x] |
| 1836 | `ZSTD_decodeSeqHeaders` (c_src/src/legacy/zstd_v03.c:2554) | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [x] |
| 1837 | `ZSTD_decodeSeqHeaders` (c_src/src/legacy/zstd_v03.c:2555) | `if (LLlog > LLFSELog) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1838 | `ZSTD_decodeSeqHeaders` (c_src/src/legacy/zstd_v03.c:2564) | `if (ip > iend-2) return ERROR(srcSize_wrong); /* min : "raw", hence no header, but at least xxLog bits */` | `ERROR(srcSize_wrong)` | [x] |
| 1839 | `ZSTD_decodeSeqHeaders` (c_src/src/legacy/zstd_v03.c:2573) | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [x] |
| 1840 | `ZSTD_decodeSeqHeaders` (c_src/src/legacy/zstd_v03.c:2574) | `if (Offlog > OffFSELog) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1841 | `ZSTD_decodeSeqHeaders` (c_src/src/legacy/zstd_v03.c:2583) | `if (ip > iend-2) return ERROR(srcSize_wrong); /* min : "raw", hence no header, but at least xxLog bits */` | `ERROR(srcSize_wrong)` | [x] |
| 1842 | `ZSTD_decodeSeqHeaders` (c_src/src/legacy/zstd_v03.c:2591) | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [x] |
| 1843 | `ZSTD_decodeSeqHeaders` (c_src/src/legacy/zstd_v03.c:2592) | `if (MLlog > MLFSELog) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1844 | `ZSTD_execSequence` (c_src/src/legacy/zstd_v03.c:2698) | `if (seqLength > (size_t)(oend - op)) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 1845 | `ZSTD_execSequence` (c_src/src/legacy/zstd_v03.c:2699) | `if (sequence.litLength > (size_t)(litLimit - *litPtr)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1846 | `ZSTD_execSequence` (c_src/src/legacy/zstd_v03.c:2701) | `if (oLitEnd > oend_8) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 1847 | `ZSTD_execSequence` (c_src/src/legacy/zstd_v03.c:2702) | `if (sequence.offset > (U32)(oLitEnd - base)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1848 | `ZSTD_execSequence` (c_src/src/legacy/zstd_v03.c:2704) | `if (oMatchEnd > oend) return ERROR(dstSize_tooSmall); /* overwrite beyond dst buffer */` | `ERROR(dstSize_tooSmall)` | [x] |
| 1849 | `ZSTD_execSequence` (c_src/src/legacy/zstd_v03.c:2705) | `if (litEnd > litLimit) return ERROR(corruption_detected); /* overRead beyond lit buffer */` | `ERROR(corruption_detected)` | [x] |
| 1850 | `ZSTD_execSequence` (c_src/src/legacy/zstd_v03.c:2716) | `if (sequence.offset > (size_t)op) return ERROR(corruption_detected); /* address space overflow test (this test seems kept by clang optimizer) */` | `ERROR(corruption_detected)` | [x] |
| 1851 | `ZSTD_execSequence` (c_src/src/legacy/zstd_v03.c:2717) | `//if (match > op) return ERROR(corruption_detected); /* address space overflow test (is clang optimizer removing this test ?) */` | `ERROR(corruption_detected)` | [x] |
| 1852 | `ZSTD_execSequence` (c_src/src/legacy/zstd_v03.c:2718) | `if (match < base) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1853 | `ZSTD_decompressSequences` (c_src/src/legacy/zstd_v03.c:2795) | `if (ERR_isError(errorCode)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1854 | `ZSTD_decompressSequences` (c_src/src/legacy/zstd_v03.c:2811) | `if ( !BIT_endOfDStream(&(seqState.DStream)) ) return ERROR(corruption_detected); /* requested too much : data is corrupted */` | `ERROR(corruption_detected)` | [x] |
| 1855 | `ZSTD_decompressSequences` (c_src/src/legacy/zstd_v03.c:2812) | `if (nbSeq<0) return ERROR(corruption_detected); /* requested too many sequences : data is corrupted */` | `ERROR(corruption_detected)` | [x] |
| 1856 | `ZSTD_decompressSequences` (c_src/src/legacy/zstd_v03.c:2817) | `if (litPtr > litEnd) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1857 | `ZSTD_decompressSequences` (c_src/src/legacy/zstd_v03.c:2818) | `if (op+lastLLSize > oend) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 1858 | `ZSTD_decompressDCtx` (c_src/src/legacy/zstd_v03.c:2860) | `if (srcSize < ZSTD_frameHeaderSize+ZSTD_blockHeaderSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1859 | `ZSTD_decompressDCtx` (c_src/src/legacy/zstd_v03.c:2862) | `if (magicNumber != ZSTD_magicNumber) return ERROR(prefix_unknown);` | `ERROR(prefix_unknown)` | [x] |
| 1860 | `ZSTD_decompressDCtx` (c_src/src/legacy/zstd_v03.c:2874) | `if (cBlockSize > remainingSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1861 | `ZSTD_decompressDCtx` (c_src/src/legacy/zstd_v03.c:2885) | `return ERROR(GENERIC); /* not yet supported */` | `ERROR(GENERIC)` | [x] |
| 1862 | `ZSTD_decompressDCtx` (c_src/src/legacy/zstd_v03.c:2889) | `if (remainingSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1863 | `ZSTD_decompressDCtx` (c_src/src/legacy/zstd_v03.c:2892) | `return ERROR(GENERIC); /* impossible */` | `ERROR(GENERIC)` | [x] |
| 1864 | `ZSTD_createDCtx` (c_src/src/legacy/zstd_v03.c:2984) | `if (dctx==NULL) return NULL;` | `NULL` | [x] |
| 1865 | `ZSTD_decompressContinue` (c_src/src/legacy/zstd_v03.c:3003) | `if (srcSize != ctx->expected) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1866 | `ZSTD_decompressContinue` (c_src/src/legacy/zstd_v03.c:3012) | `if (magicNumber != ZSTD_magicNumber) return ERROR(prefix_unknown);` | `ERROR(prefix_unknown)` | [x] |
| 1867 | `ZSTD_decompressContinue` (c_src/src/legacy/zstd_v03.c:3051) | `return ERROR(GENERIC); /* not yet handled */` | `ERROR(GENERIC)` | [x] |
| 1868 | `ZSTD_decompressContinue` (c_src/src/legacy/zstd_v03.c:3057) | `return ERROR(GENERIC);` | `ERROR(GENERIC)` | [x] |
| 1869 | `(file scope)` (c_src/src/legacy/zstd_v04.c:75) | `# define assert(condition) ((void)0)` | process assertion failure | [x] |
| 1870 | `BIT_initDStream` (c_src/src/legacy/zstd_v04.c:603) | `if (srcSize < 1) { memset(bitD, 0, sizeof(*bitD)); return ERROR(srcSize_wrong); }` | `ERROR(srcSize_wrong)` | [x] |
| 1871 | `BIT_initDStream` (c_src/src/legacy/zstd_v04.c:612) | `if (contain32 == 0) return ERROR(GENERIC); /* endMark not present */` | `ERROR(GENERIC)` | [x] |
| 1872 | `BIT_initDStream` (c_src/src/legacy/zstd_v04.c:632) | `if (contain32 == 0) return ERROR(GENERIC); /* endMark not present */` | `ERROR(GENERIC)` | [x] |
| 1873 | `FSE_buildDTable` (c_src/src/legacy/zstd_v04.c:1033) | `if (maxSymbolValue > FSE_MAX_SYMBOL_VALUE) return ERROR(maxSymbolValue_tooLarge);` | `ERROR(maxSymbolValue_tooLarge)` | [x] |
| 1874 | `FSE_buildDTable` (c_src/src/legacy/zstd_v04.c:1034) | `if (tableLog > FSE_MAX_TABLELOG) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [x] |
| 1875 | `FSE_buildDTable` (c_src/src/legacy/zstd_v04.c:1065) | `if (position!=0) return ERROR(GENERIC); /* position must reach all cells once, otherwise normalizedCounter is incorrect */` | `ERROR(GENERIC)` | [x] |
| 1876 | `FSE_readNCount` (c_src/src/legacy/zstd_v04.c:1114) | `if (hbSize < 4) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1877 | `FSE_readNCount` (c_src/src/legacy/zstd_v04.c:1117) | `if (nbBits > FSE_TABLELOG_ABSOLUTE_MAX) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [x] |
| 1878 | `FSE_readNCount` (c_src/src/legacy/zstd_v04.c:1152) | `if (n0 > *maxSVPtr) return ERROR(maxSymbolValue_tooSmall);` | `ERROR(maxSymbolValue_tooSmall)` | [x] |
| 1879 | `FSE_readNCount` (c_src/src/legacy/zstd_v04.c:1204) | `if (remaining != 1) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [x] |
| 1880 | `FSE_readNCount` (c_src/src/legacy/zstd_v04.c:1208) | `if ((size_t)(ip-istart) > hbSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1881 | `FSE_buildDTable_raw` (c_src/src/legacy/zstd_v04.c:1246) | `if (nbBits < 1) return ERROR(GENERIC); /* min size */` | `ERROR(GENERIC)` | [x] |
| 1882 | `FSE_decompress_usingDTable_generic` (c_src/src/legacy/zstd_v04.c:1325) | `if (op==omax) return ERROR(dstSize_tooSmall); /* dst buffer is full, but cSrc unfinished */` | `ERROR(dstSize_tooSmall)` | [x] |
| 1883 | `FSE_decompress_usingDTable_generic` (c_src/src/legacy/zstd_v04.c:1327) | `return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1884 | `FSE_decompress` (c_src/src/legacy/zstd_v04.c:1357) | `if (cSrcSize<2) return ERROR(srcSize_wrong); /* too small input size */` | `ERROR(srcSize_wrong)` | [x] |
| 1885 | `FSE_decompress` (c_src/src/legacy/zstd_v04.c:1362) | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong); /* too small input size */` | `ERROR(srcSize_wrong)` | [x] |
| 1886 | `HUF_readStats` (c_src/src/legacy/zstd_v04.c:1647) | `if (!srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1887 | `HUF_readStats` (c_src/src/legacy/zstd_v04.c:1664) | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1888 | `HUF_readStats` (c_src/src/legacy/zstd_v04.c:1665) | `if (oSize >= hwSize) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1889 | `HUF_readStats` (c_src/src/legacy/zstd_v04.c:1676) | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1890 | `HUF_readStats` (c_src/src/legacy/zstd_v04.c:1686) | `if (huffWeight[n] >= HUF_ABSOLUTEMAX_TABLELOG) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1891 | `HUF_readStats` (c_src/src/legacy/zstd_v04.c:1690) | `if (weightTotal == 0) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1892 | `HUF_readStats` (c_src/src/legacy/zstd_v04.c:1694) | `if (tableLog > HUF_ABSOLUTEMAX_TABLELOG) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1893 | `HUF_readStats` (c_src/src/legacy/zstd_v04.c:1700) | `if (verif != rest) return ERROR(corruption_detected); /* last value must be a clean power of 2 */` | `ERROR(corruption_detected)` | [x] |
| 1894 | `HUF_readStats` (c_src/src/legacy/zstd_v04.c:1706) | `if ((rankStats[1] < 2) \|\| (rankStats[1] & 1)) return ERROR(corruption_detected); /* by construction : at least 2 elts of rank 1, must be even */` | `ERROR(corruption_detected)` | [x] |
| 1895 | `HUF_readDTableX2` (c_src/src/legacy/zstd_v04.c:1738) | `if (tableLog > DTable[0]) return ERROR(tableLog_tooLarge); /* DTable is too small */` | `ERROR(tableLog_tooLarge)` | [x] |
| 1896 | `HUF_decompress4X2_usingDTable` (c_src/src/legacy/zstd_v04.c:1815) | `if (cSrcSize < 10) return ERROR(corruption_detected); /* strict minimum : jump table + 1 byte per stream */` | `ERROR(corruption_detected)` | [x] |
| 1897 | `HUF_decompress4X2_usingDTable` (c_src/src/legacy/zstd_v04.c:1850) | `if (length4 > cSrcSize) return ERROR(corruption_detected); /* overflow */` | `ERROR(corruption_detected)` | [x] |
| 1898 | `HUF_decompress4X2_usingDTable` (c_src/src/legacy/zstd_v04.c:1885) | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1899 | `HUF_decompress4X2_usingDTable` (c_src/src/legacy/zstd_v04.c:1886) | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1900 | `HUF_decompress4X2_usingDTable` (c_src/src/legacy/zstd_v04.c:1887) | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1901 | `HUF_decompress4X2_usingDTable` (c_src/src/legacy/zstd_v04.c:1898) | `if (!endSignal) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1902 | `HUF_decompress4X2` (c_src/src/legacy/zstd_v04.c:1914) | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1903 | `HUF_readDTableX4` (c_src/src/legacy/zstd_v04.c:2034) | `if (memLog > HUF_ABSOLUTEMAX_TABLELOG) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [x] |
| 1904 | `HUF_readDTableX4` (c_src/src/legacy/zstd_v04.c:2041) | `if (tableLog > memLog) return ERROR(tableLog_tooLarge); /* DTable can't fit code depth */` | `ERROR(tableLog_tooLarge)` | [x] |
| 1905 | `HUF_readDTableX4` (c_src/src/legacy/zstd_v04.c:2045) | `{ if (!maxW) return ERROR(GENERIC); } /* necessarily finds a solution before maxW==0 */` | `ERROR(GENERIC)` | [x] |
| 1906 | `HUF_decompress4X4_usingDTable` (c_src/src/legacy/zstd_v04.c:2173) | `if (cSrcSize < 10) return ERROR(corruption_detected); /* strict minimum : jump table + 1 byte per stream */` | `ERROR(corruption_detected)` | [x] |
| 1907 | `HUF_decompress4X4_usingDTable` (c_src/src/legacy/zstd_v04.c:2208) | `if (length4 > cSrcSize) return ERROR(corruption_detected); /* overflow */` | `ERROR(corruption_detected)` | [x] |
| 1908 | `HUF_decompress4X4_usingDTable` (c_src/src/legacy/zstd_v04.c:2243) | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1909 | `HUF_decompress4X4_usingDTable` (c_src/src/legacy/zstd_v04.c:2244) | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1910 | `HUF_decompress4X4_usingDTable` (c_src/src/legacy/zstd_v04.c:2245) | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1911 | `HUF_decompress4X4_usingDTable` (c_src/src/legacy/zstd_v04.c:2256) | `if (!endSignal) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1912 | `HUF_decompress4X4` (c_src/src/legacy/zstd_v04.c:2271) | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1913 | `HUF_decompress` (c_src/src/legacy/zstd_v04.c:2318) | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 1914 | `HUF_decompress` (c_src/src/legacy/zstd_v04.c:2319) | `if (cSrcSize > dstSize) return ERROR(corruption_detected); /* invalid */` | `ERROR(corruption_detected)` | [x] |
| 1915 | `ZSTD_createDCtx` (c_src/src/legacy/zstd_v04.c:2472) | `if (dctx==NULL) return NULL;` | `NULL` | [x] |
| 1916 | `ZSTD_decodeFrameHeader_Part1` (c_src/src/legacy/zstd_v04.c:2494) | `if (srcSize != ZSTD_frameHeaderSize_min) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1917 | `ZSTD_decodeFrameHeader_Part1` (c_src/src/legacy/zstd_v04.c:2496) | `if (magicNumber != ZSTD_MAGICNUMBER) return ERROR(prefix_unknown);` | `ERROR(prefix_unknown)` | [x] |
| 1918 | `ZSTD_getFrameParams` (c_src/src/legacy/zstd_v04.c:2507) | `if (magicNumber != ZSTD_MAGICNUMBER) return ERROR(prefix_unknown);` | `ERROR(prefix_unknown)` | [x] |
| 1919 | `ZSTD_getFrameParams` (c_src/src/legacy/zstd_v04.c:2510) | `if ((((const BYTE*)src)[4] >> 4) != 0) return ERROR(frameParameter_unsupported); /* reserved bits */` | `ERROR(frameParameter_unsupported)` | [x] |
| 1920 | `ZSTD_decodeFrameHeader_Part2` (c_src/src/legacy/zstd_v04.c:2521) | `if (srcSize != zc->headerSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1921 | `ZSTD_decodeFrameHeader_Part2` (c_src/src/legacy/zstd_v04.c:2523) | `if ((MEM_32bits()) && (zc->params.windowLog > 25)) return ERROR(frameParameter_unsupported);` | `ERROR(frameParameter_unsupported)` | [x] |
| 1922 | `ZSTD_getcBlockSize` (c_src/src/legacy/zstd_v04.c:2534) | `if (srcSize < 3) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1923 | `ZSTD_copyRawBlock` (c_src/src/legacy/zstd_v04.c:2549) | `if (srcSize > maxDstSize) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 1924 | `ZSTD_decompressLiterals` (c_src/src/legacy/zstd_v04.c:2567) | `if (litSize > *maxDstSizePtr) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1925 | `ZSTD_decompressLiterals` (c_src/src/legacy/zstd_v04.c:2568) | `if (litCSize + 5 > srcSize) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1926 | `ZSTD_decompressLiterals` (c_src/src/legacy/zstd_v04.c:2570) | `if (HUF_isError(HUF_decompress(dst, litSize, ip+5, litCSize))) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1927 | `ZSTD_decodeLiteralsBlock` (c_src/src/legacy/zstd_v04.c:2585) | `if (srcSize < MIN_CBLOCK_SIZE) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1928 | `ZSTD_decodeLiteralsBlock` (c_src/src/legacy/zstd_v04.c:2604) | `if (litSize > BLOCKSIZE) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1929 | `ZSTD_decodeLiteralsBlock` (c_src/src/legacy/zstd_v04.c:2605) | `if (litSize > srcSize-3) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1930 | `ZSTD_decodeLiteralsBlock` (c_src/src/legacy/zstd_v04.c:2619) | `if (litSize > BLOCKSIZE) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1931 | `ZSTD_decodeLiteralsBlock` (c_src/src/legacy/zstd_v04.c:2626) | `return ERROR(corruption_detected); /* forbidden nominal case */` | `ERROR(corruption_detected)` | [x] |
| 1932 | `ZSTD_decodeSeqHeaders` (c_src/src/legacy/zstd_v04.c:2643) | `if (srcSize < 5) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1933 | `ZSTD_decodeSeqHeaders` (c_src/src/legacy/zstd_v04.c:2667) | `if (ip > iend-3) return ERROR(srcSize_wrong); /* min : all 3 are "raw", hence no header, but at least xxLog bits per type */` | `ERROR(srcSize_wrong)` | [x] |
| 1934 | `ZSTD_decodeSeqHeaders` (c_src/src/legacy/zstd_v04.c:2686) | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [x] |
| 1935 | `ZSTD_decodeSeqHeaders` (c_src/src/legacy/zstd_v04.c:2687) | `if (LLlog > LLFSELog) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1936 | `ZSTD_decodeSeqHeaders` (c_src/src/legacy/zstd_v04.c:2696) | `if (ip > iend-2) return ERROR(srcSize_wrong); /* min : "raw", hence no header, but at least xxLog bits */` | `ERROR(srcSize_wrong)` | [x] |
| 1937 | `ZSTD_decodeSeqHeaders` (c_src/src/legacy/zstd_v04.c:2705) | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [x] |
| 1938 | `ZSTD_decodeSeqHeaders` (c_src/src/legacy/zstd_v04.c:2706) | `if (Offlog > OffFSELog) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1939 | `ZSTD_decodeSeqHeaders` (c_src/src/legacy/zstd_v04.c:2715) | `if (ip > iend-2) return ERROR(srcSize_wrong); /* min : "raw", hence no header, but at least xxLog bits */` | `ERROR(srcSize_wrong)` | [x] |
| 1940 | `ZSTD_decodeSeqHeaders` (c_src/src/legacy/zstd_v04.c:2723) | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [x] |
| 1941 | `ZSTD_decodeSeqHeaders` (c_src/src/legacy/zstd_v04.c:2724) | `if (MLlog > MLFSELog) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1942 | `ZSTD_execSequence` (c_src/src/legacy/zstd_v04.c:2826) | `if (seqLength > (size_t)(oend - op)) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 1943 | `ZSTD_execSequence` (c_src/src/legacy/zstd_v04.c:2827) | `if (sequence.litLength > (size_t)(litLimit - *litPtr)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1944 | `ZSTD_execSequence` (c_src/src/legacy/zstd_v04.c:2829) | `if (oLitEnd > oend_8) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 1945 | `ZSTD_execSequence` (c_src/src/legacy/zstd_v04.c:2831) | `if (oMatchEnd > oend) return ERROR(dstSize_tooSmall); /* overwrite beyond dst buffer */` | `ERROR(dstSize_tooSmall)` | [x] |
| 1946 | `ZSTD_execSequence` (c_src/src/legacy/zstd_v04.c:2832) | `if (litEnd > litLimit) return ERROR(corruption_detected); /* overRead beyond lit buffer */` | `ERROR(corruption_detected)` | [x] |
| 1947 | `ZSTD_execSequence` (c_src/src/legacy/zstd_v04.c:2844) | `return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1948 | `ZSTD_decompressSequences` (c_src/src/legacy/zstd_v04.c:2940) | `if (ERR_isError(errorCode)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1949 | `ZSTD_decompressSequences` (c_src/src/legacy/zstd_v04.c:2956) | `if ( !BIT_endOfDStream(&(seqState.DStream)) ) return ERROR(corruption_detected); /* DStream should be entirely and exactly consumed; otherwise data is corrupted */` | `ERROR(corruption_detected)` | [x] |
| 1950 | `ZSTD_decompressSequences` (c_src/src/legacy/zstd_v04.c:2961) | `if (litPtr > litEnd) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1951 | `ZSTD_decompressSequences` (c_src/src/legacy/zstd_v04.c:2962) | `if (op+lastLLSize > oend) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 1952 | `ZSTD_decompressBlock_internal` (c_src/src/legacy/zstd_v04.c:2994) | `if (srcSize > BLOCKSIZE) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1953 | `ZSTD_decompress_usingDict` (c_src/src/legacy/zstd_v04.c:3036) | `if (srcSize < ZSTD_frameHeaderSize_min+ZSTD_blockHeaderSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1954 | `ZSTD_decompress_usingDict` (c_src/src/legacy/zstd_v04.c:3039) | `if (srcSize < frameHeaderSize+ZSTD_blockHeaderSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1955 | `ZSTD_decompress_usingDict` (c_src/src/legacy/zstd_v04.c:3054) | `if (cBlockSize > remainingSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1956 | `ZSTD_decompress_usingDict` (c_src/src/legacy/zstd_v04.c:3065) | `return ERROR(GENERIC); /* not yet supported */` | `ERROR(GENERIC)` | [x] |
| 1957 | `ZSTD_decompress_usingDict` (c_src/src/legacy/zstd_v04.c:3069) | `if (remainingSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1958 | `ZSTD_decompress_usingDict` (c_src/src/legacy/zstd_v04.c:3072) | `return ERROR(GENERIC); /* impossible */` | `ERROR(GENERIC)` | [x] |
| 1959 | `ZSTD_decompressContinue` (c_src/src/legacy/zstd_v04.c:3149) | `if (srcSize != ctx->expected) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1960 | `ZSTD_decompressContinue` (c_src/src/legacy/zstd_v04.c:3157) | `if (srcSize != ZSTD_frameHeaderSize_min) return ERROR(srcSize_wrong); /* impossible */` | `ERROR(srcSize_wrong)` | [x] |
| 1961 | `ZSTD_decompressContinue` (c_src/src/legacy/zstd_v04.c:3161) | `if (ctx->headerSize > ZSTD_frameHeaderSize_min) return ERROR(GENERIC); /* impossible */` | `ERROR(GENERIC)` | [x] |
| 1962 | `ZSTD_decompressContinue` (c_src/src/legacy/zstd_v04.c:3203) | `return ERROR(GENERIC); /* not yet handled */` | `ERROR(GENERIC)` | [x] |
| 1963 | `ZSTD_decompressContinue` (c_src/src/legacy/zstd_v04.c:3209) | `return ERROR(GENERIC);` | `ERROR(GENERIC)` | [x] |
| 1964 | `ZSTD_decompressContinue` (c_src/src/legacy/zstd_v04.c:3218) | `return ERROR(GENERIC); /* impossible */` | `ERROR(GENERIC)` | [x] |
| 1965 | `ZBUFF_createDCtx` (c_src/src/legacy/zstd_v04.c:3327) | `if (zbc==NULL) return NULL;` | `NULL` | [x] |
| 1966 | `ZBUFF_decompressContinue` (c_src/src/legacy/zstd_v04.c:3391) | `return ERROR(init_missing);` | `ERROR(init_missing)` | [x] |
| 1967 | `ZBUFF_decompressContinue` (c_src/src/legacy/zstd_v04.c:3433) | `if (zbc->inBuff == NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [x] |
| 1968 | `ZBUFF_decompressContinue` (c_src/src/legacy/zstd_v04.c:3439) | `if (zbc->outBuff == NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [x] |
| 1969 | `ZBUFF_decompressContinue` (c_src/src/legacy/zstd_v04.c:3484) | `if (toLoad > zbc->inBuffSize - zbc->inPos) return ERROR(corruption_detected); /* should never happen */` | `ERROR(corruption_detected)` | [x] |
| 1970 | `ZBUFF_decompressContinue` (c_src/src/legacy/zstd_v04.c:3519) | `default: return ERROR(GENERIC); /* impossible */` | `ERROR(GENERIC)` | [x] |
| 1971 | `ZSTDv04_decompressDCtx` (c_src/src/legacy/zstd_v04.c:3552) | `return ZSTD_decompress_usingDict(dctx, dst, maxDstSize, src, srcSize, NULL, 0);` | source-declared rejection sentinel | [x] |
| 1972 | `ZSTDv04_decompress` (c_src/src/legacy/zstd_v04.c:3560) | `if (dctx==NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [x] |
| 1973 | `BITv05_initDStream` (c_src/src/legacy/zstd_v05.c:736) | `if (srcSize < 1) { memset(bitD, 0, sizeof(*bitD)); return ERROR(srcSize_wrong); }` | `ERROR(srcSize_wrong)` | [x] |
| 1974 | `BITv05_initDStream` (c_src/src/legacy/zstd_v05.c:744) | `if (contain32 == 0) return ERROR(GENERIC); /* endMark not present */` | `ERROR(GENERIC)` | [x] |
| 1975 | `BITv05_initDStream` (c_src/src/legacy/zstd_v05.c:762) | `if (contain32 == 0) return ERROR(GENERIC); /* endMark not present */` | `ERROR(GENERIC)` | [x] |
| 1976 | `FSEv05_buildDTable` (c_src/src/legacy/zstd_v05.c:1173) | `if (maxSymbolValue > FSEv05_MAX_SYMBOL_VALUE) return ERROR(maxSymbolValue_tooLarge);` | `ERROR(maxSymbolValue_tooLarge)` | [x] |
| 1977 | `FSEv05_buildDTable` (c_src/src/legacy/zstd_v05.c:1174) | `if (tableLog > FSEv05_MAX_TABLELOG) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [x] |
| 1978 | `FSEv05_buildDTable` (c_src/src/legacy/zstd_v05.c:1197) | `if (position!=0) return ERROR(GENERIC); /* position must reach all cells once, otherwise normalizedCounter is incorrect */` | `ERROR(GENERIC)` | [x] |
| 1979 | `FSEv05_readNCount` (c_src/src/legacy/zstd_v05.c:1244) | `if (hbSize < 4) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1980 | `FSEv05_readNCount` (c_src/src/legacy/zstd_v05.c:1247) | `if (nbBits > FSEv05_TABLELOG_ABSOLUTE_MAX) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [x] |
| 1981 | `FSEv05_readNCount` (c_src/src/legacy/zstd_v05.c:1274) | `if (n0 > *maxSVPtr) return ERROR(maxSymbolValue_tooSmall);` | `ERROR(maxSymbolValue_tooSmall)` | [x] |
| 1982 | `FSEv05_readNCount` (c_src/src/legacy/zstd_v05.c:1315) | `if (remaining != 1) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [x] |
| 1983 | `FSEv05_readNCount` (c_src/src/legacy/zstd_v05.c:1319) | `if ((size_t)(ip-istart) > hbSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1984 | `FSEv05_buildDTable_raw` (c_src/src/legacy/zstd_v05.c:1358) | `if (nbBits < 1) return ERROR(GENERIC); /* min size */` | `ERROR(GENERIC)` | [x] |
| 1985 | `FSEv05_decompress_usingDTable_generic` (c_src/src/legacy/zstd_v05.c:1434) | `if (op==omax) return ERROR(dstSize_tooSmall); /* dst buffer is full, but cSrc unfinished */` | `ERROR(dstSize_tooSmall)` | [x] |
| 1986 | `FSEv05_decompress_usingDTable_generic` (c_src/src/legacy/zstd_v05.c:1436) | `return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1987 | `FSEv05_decompress` (c_src/src/legacy/zstd_v05.c:1464) | `if (cSrcSize<2) return ERROR(srcSize_wrong); /* too small input size */` | `ERROR(srcSize_wrong)` | [x] |
| 1988 | `FSEv05_decompress` (c_src/src/legacy/zstd_v05.c:1469) | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong); /* too small input size */` | `ERROR(srcSize_wrong)` | [x] |
| 1989 | `HUFv05_readStats` (c_src/src/legacy/zstd_v05.c:1753) | `if (!srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1990 | `HUFv05_readStats` (c_src/src/legacy/zstd_v05.c:1767) | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1991 | `HUFv05_readStats` (c_src/src/legacy/zstd_v05.c:1768) | `if (oSize >= hwSize) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1992 | `HUFv05_readStats` (c_src/src/legacy/zstd_v05.c:1775) | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 1993 | `HUFv05_readStats` (c_src/src/legacy/zstd_v05.c:1784) | `if (huffWeight[n] >= HUFv05_ABSOLUTEMAX_TABLELOG) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1994 | `HUFv05_readStats` (c_src/src/legacy/zstd_v05.c:1788) | `if (weightTotal == 0) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1995 | `HUFv05_readStats` (c_src/src/legacy/zstd_v05.c:1792) | `if (tableLog > HUFv05_ABSOLUTEMAX_TABLELOG) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 1996 | `HUFv05_readStats` (c_src/src/legacy/zstd_v05.c:1798) | `if (verif != rest) return ERROR(corruption_detected); /* last value must be a clean power of 2 */` | `ERROR(corruption_detected)` | [x] |
| 1997 | `HUFv05_readStats` (c_src/src/legacy/zstd_v05.c:1804) | `if ((rankStats[1] < 2) \|\| (rankStats[1] & 1)) return ERROR(corruption_detected); /* by construction : at least 2 elts of rank 1, must be even */` | `ERROR(corruption_detected)` | [x] |
| 1998 | `HUFv05_readDTableX2` (c_src/src/legacy/zstd_v05.c:1836) | `if (tableLog > DTable[0]) return ERROR(tableLog_tooLarge); /* DTable is too small */` | `ERROR(tableLog_tooLarge)` | [x] |
| 1999 | `HUFv05_decompress1X2_usingDTable` (c_src/src/legacy/zstd_v05.c:1916) | `if (dstSize <= cSrcSize) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 2000 | `HUFv05_decompress1X2_usingDTable` (c_src/src/legacy/zstd_v05.c:1923) | `if (!BITv05_endOfDStream(&bitD)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2001 | `HUFv05_decompress1X2` (c_src/src/legacy/zstd_v05.c:1936) | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2002 | `HUFv05_decompress4X2_usingDTable` (c_src/src/legacy/zstd_v05.c:1950) | `if (cSrcSize < 10) return ERROR(corruption_detected); /* strict minimum : jump table + 1 byte per stream */` | `ERROR(corruption_detected)` | [x] |
| 2003 | `HUFv05_decompress4X2_usingDTable` (c_src/src/legacy/zstd_v05.c:1984) | `if (length4 > cSrcSize) return ERROR(corruption_detected); /* overflow */` | `ERROR(corruption_detected)` | [x] |
| 2004 | `HUFv05_decompress4X2_usingDTable` (c_src/src/legacy/zstd_v05.c:2017) | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2005 | `HUFv05_decompress4X2_usingDTable` (c_src/src/legacy/zstd_v05.c:2018) | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2006 | `HUFv05_decompress4X2_usingDTable` (c_src/src/legacy/zstd_v05.c:2019) | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2007 | `HUFv05_decompress4X2_usingDTable` (c_src/src/legacy/zstd_v05.c:2030) | `if (!endSignal) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2008 | `HUFv05_decompress4X2` (c_src/src/legacy/zstd_v05.c:2046) | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2009 | `HUFv05_readDTableX4` (c_src/src/legacy/zstd_v05.c:2160) | `if (memLog > HUFv05_ABSOLUTEMAX_TABLELOG) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [x] |
| 2010 | `HUFv05_readDTableX4` (c_src/src/legacy/zstd_v05.c:2167) | `if (tableLog > memLog) return ERROR(tableLog_tooLarge); /* DTable can't fit code depth */` | `ERROR(tableLog_tooLarge)` | [x] |
| 2011 | `HUFv05_decompress1X4_usingDTable` (c_src/src/legacy/zstd_v05.c:2306) | `if (!BITv05_endOfDStream(&bitD)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2012 | `HUFv05_decompress1X4` (c_src/src/legacy/zstd_v05.c:2319) | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2013 | `HUFv05_decompress4X4_usingDTable` (c_src/src/legacy/zstd_v05.c:2331) | `if (cSrcSize < 10) return ERROR(corruption_detected); /* strict minimum : jump table + 1 byte per stream */` | `ERROR(corruption_detected)` | [x] |
| 2014 | `HUFv05_decompress4X4_usingDTable` (c_src/src/legacy/zstd_v05.c:2366) | `if (length4 > cSrcSize) return ERROR(corruption_detected); /* overflow */` | `ERROR(corruption_detected)` | [x] |
| 2015 | `HUFv05_decompress4X4_usingDTable` (c_src/src/legacy/zstd_v05.c:2400) | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2016 | `HUFv05_decompress4X4_usingDTable` (c_src/src/legacy/zstd_v05.c:2401) | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2017 | `HUFv05_decompress4X4_usingDTable` (c_src/src/legacy/zstd_v05.c:2402) | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2018 | `HUFv05_decompress4X4_usingDTable` (c_src/src/legacy/zstd_v05.c:2413) | `if (!endSignal) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2019 | `HUFv05_decompress4X4` (c_src/src/legacy/zstd_v05.c:2428) | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2020 | `HUFv05_decompress` (c_src/src/legacy/zstd_v05.c:2475) | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 2021 | `HUFv05_decompress` (c_src/src/legacy/zstd_v05.c:2476) | `if (cSrcSize >= dstSize) return ERROR(corruption_detected); /* invalid, or not compressed, but not compressed already dealt with */` | `ERROR(corruption_detected)` | [x] |
| 2022 | `ZSTDv05_createDCtx` (c_src/src/legacy/zstd_v05.c:2632) | `if (dctx==NULL) return NULL;` | `NULL` | [x] |
| 2023 | `ZSTDv05_decodeFrameHeader_Part1` (c_src/src/legacy/zstd_v05.c:2743) | `return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2024 | `ZSTDv05_decodeFrameHeader_Part1` (c_src/src/legacy/zstd_v05.c:2745) | `if (magicNumber != ZSTDv05_MAGICNUMBER) return ERROR(prefix_unknown);` | `ERROR(prefix_unknown)` | [x] |
| 2025 | `ZSTDv05_getFrameParams` (c_src/src/legacy/zstd_v05.c:2756) | `if (magicNumber != ZSTDv05_MAGICNUMBER) return ERROR(prefix_unknown);` | `ERROR(prefix_unknown)` | [x] |
| 2026 | `ZSTDv05_getFrameParams` (c_src/src/legacy/zstd_v05.c:2759) | `if ((((const BYTE*)src)[4] >> 4) != 0) return ERROR(frameParameter_unsupported); /* reserved bits */` | `ERROR(frameParameter_unsupported)` | [x] |
| 2027 | `ZSTDv05_decodeFrameHeader_Part2` (c_src/src/legacy/zstd_v05.c:2771) | `return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2028 | `ZSTDv05_decodeFrameHeader_Part2` (c_src/src/legacy/zstd_v05.c:2773) | `if ((MEM_32bits()) && (zc->params.windowLog > 25)) return ERROR(frameParameter_unsupported);` | `ERROR(frameParameter_unsupported)` | [x] |
| 2029 | `ZSTDv05_getcBlockSize` (c_src/src/legacy/zstd_v05.c:2785) | `return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2030 | `ZSTDv05_copyRawBlock` (c_src/src/legacy/zstd_v05.c:2801) | `if (dst==NULL) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 2031 | `ZSTDv05_copyRawBlock` (c_src/src/legacy/zstd_v05.c:2802) | `if (srcSize > maxDstSize) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 2032 | `ZSTDv05_decodeLiteralsBlock` (c_src/src/legacy/zstd_v05.c:2816) | `if (srcSize < MIN_CBLOCK_SIZE) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2033 | `ZSTDv05_decodeLiteralsBlock` (c_src/src/legacy/zstd_v05.c:2824) | `if (srcSize < 5) return ERROR(corruption_detected); /* srcSize >= MIN_CBLOCK_SIZE == 3; here we need up to 5 for case 3 */` | `ERROR(corruption_detected)` | [x] |
| 2034 | `ZSTDv05_decodeLiteralsBlock` (c_src/src/legacy/zstd_v05.c:2847) | `if (litSize > BLOCKSIZE) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2035 | `ZSTDv05_decodeLiteralsBlock` (c_src/src/legacy/zstd_v05.c:2848) | `if (litCSize + lhSize > srcSize) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2036 | `ZSTDv05_decodeLiteralsBlock` (c_src/src/legacy/zstd_v05.c:2853) | `return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2037 | `ZSTDv05_decodeLiteralsBlock` (c_src/src/legacy/zstd_v05.c:2866) | `return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2038 | `ZSTDv05_decodeLiteralsBlock` (c_src/src/legacy/zstd_v05.c:2868) | `return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [x] |
| 2039 | `ZSTDv05_decodeLiteralsBlock` (c_src/src/legacy/zstd_v05.c:2874) | `if (litCSize + lhSize > srcSize) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2040 | `ZSTDv05_decodeLiteralsBlock` (c_src/src/legacy/zstd_v05.c:2877) | `if (HUFv05_isError(errorCode)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2041 | `ZSTDv05_decodeLiteralsBlock` (c_src/src/legacy/zstd_v05.c:2903) | `if (litSize+lhSize > srcSize) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2042 | `ZSTDv05_decodeLiteralsBlock` (c_src/src/legacy/zstd_v05.c:2930) | `if (srcSize<4) return ERROR(corruption_detected); /* srcSize >= MIN_CBLOCK_SIZE == 3; here we need lhSize+1 = 4 */` | `ERROR(corruption_detected)` | [x] |
| 2043 | `ZSTDv05_decodeLiteralsBlock` (c_src/src/legacy/zstd_v05.c:2933) | `if (litSize > BLOCKSIZE) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2044 | `ZSTDv05_decodeLiteralsBlock` (c_src/src/legacy/zstd_v05.c:2940) | `return ERROR(corruption_detected); /* impossible */` | `ERROR(corruption_detected)` | [x] |
| 2045 | `ZSTDv05_decodeSeqHeaders` (c_src/src/legacy/zstd_v05.c:2958) | `return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2046 | `ZSTDv05_decodeSeqHeaders` (c_src/src/legacy/zstd_v05.c:2964) | `if (ip >= iend) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2047 | `ZSTDv05_decodeSeqHeaders` (c_src/src/legacy/zstd_v05.c:2968) | `if (ip >= iend) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2048 | `ZSTDv05_decodeSeqHeaders` (c_src/src/legacy/zstd_v05.c:2973) | `if (ip+3 > iend) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2049 | `ZSTDv05_decodeSeqHeaders` (c_src/src/legacy/zstd_v05.c:2978) | `if (ip+2 > iend) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2050 | `ZSTDv05_decodeSeqHeaders` (c_src/src/legacy/zstd_v05.c:2988) | `if (ip > iend-3) return ERROR(srcSize_wrong); /* min : all 3 are "raw", hence no header, but at least xxLog bits per type */` | `ERROR(srcSize_wrong)` | [x] |
| 2051 | `ZSTDv05_decodeSeqHeaders` (c_src/src/legacy/zstd_v05.c:3007) | `if (!flagStaticTable) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2052 | `ZSTDv05_decodeSeqHeaders` (c_src/src/legacy/zstd_v05.c:3013) | `if (FSEv05_isError(headerSize)) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [x] |
| 2053 | `ZSTDv05_decodeSeqHeaders` (c_src/src/legacy/zstd_v05.c:3014) | `if (LLlog > LLFSEv05Log) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2054 | `ZSTDv05_decodeSeqHeaders` (c_src/src/legacy/zstd_v05.c:3023) | `if (ip > iend-2) return ERROR(srcSize_wrong); /* min : "raw", hence no header, but at least xxLog bits */` | `ERROR(srcSize_wrong)` | [x] |
| 2055 | `ZSTDv05_decodeSeqHeaders` (c_src/src/legacy/zstd_v05.c:3031) | `if (!flagStaticTable) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2056 | `ZSTDv05_decodeSeqHeaders` (c_src/src/legacy/zstd_v05.c:3037) | `if (FSEv05_isError(headerSize)) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [x] |
| 2057 | `ZSTDv05_decodeSeqHeaders` (c_src/src/legacy/zstd_v05.c:3038) | `if (Offlog > OffFSEv05Log) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2058 | `ZSTDv05_decodeSeqHeaders` (c_src/src/legacy/zstd_v05.c:3047) | `if (ip > iend-2) return ERROR(srcSize_wrong); /* min : "raw", hence no header, but at least xxLog bits */` | `ERROR(srcSize_wrong)` | [x] |
| 2059 | `ZSTDv05_decodeSeqHeaders` (c_src/src/legacy/zstd_v05.c:3055) | `if (!flagStaticTable) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2060 | `ZSTDv05_decodeSeqHeaders` (c_src/src/legacy/zstd_v05.c:3061) | `if (FSEv05_isError(headerSize)) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [x] |
| 2061 | `ZSTDv05_decodeSeqHeaders` (c_src/src/legacy/zstd_v05.c:3062) | `if (MLlog > MLFSEv05Log) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2062 | `ZSTDv05_execSequence` (c_src/src/legacy/zstd_v05.c:3188) | `if (seqLength > (size_t)(oend - op)) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 2063 | `ZSTDv05_execSequence` (c_src/src/legacy/zstd_v05.c:3189) | `if (sequence.litLength > (size_t)(litLimit - *litPtr)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2064 | `ZSTDv05_execSequence` (c_src/src/legacy/zstd_v05.c:3191) | `if (oLitEnd > oend_8) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 2065 | `ZSTDv05_execSequence` (c_src/src/legacy/zstd_v05.c:3193) | `if (oMatchEnd > oend) return ERROR(dstSize_tooSmall); /* overwrite beyond dst buffer */` | `ERROR(dstSize_tooSmall)` | [x] |
| 2066 | `ZSTDv05_execSequence` (c_src/src/legacy/zstd_v05.c:3194) | `if (litEnd > litLimit) return ERROR(corruption_detected); /* overRead beyond lit buffer */` | `ERROR(corruption_detected)` | [x] |
| 2067 | `ZSTDv05_execSequence` (c_src/src/legacy/zstd_v05.c:3205) | `return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2068 | `ZSTDv05_decompressSequences` (c_src/src/legacy/zstd_v05.c:3296) | `if (ERR_isError(errorCode)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2069 | `ZSTDv05_decompressSequences` (c_src/src/legacy/zstd_v05.c:3311) | `if (nbSeq) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2070 | `ZSTDv05_decompressSequences` (c_src/src/legacy/zstd_v05.c:3317) | `if (litPtr > litEnd) return ERROR(corruption_detected); /* too many literals already used */` | `ERROR(corruption_detected)` | [x] |
| 2071 | `ZSTDv05_decompressSequences` (c_src/src/legacy/zstd_v05.c:3318) | `if (op+lastLLSize > oend) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 2072 | `ZSTDv05_decompressBlock_internal` (c_src/src/legacy/zstd_v05.c:3347) | `if (srcSize >= BLOCKSIZE) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2073 | `ZSTDv05_decompress_continueDCtx` (c_src/src/legacy/zstd_v05.c:3385) | `if (srcSize < ZSTDv05_frameHeaderSize_min+ZSTDv05_blockHeaderSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2074 | `ZSTDv05_decompress_continueDCtx` (c_src/src/legacy/zstd_v05.c:3388) | `if (srcSize < frameHeaderSize+ZSTDv05_blockHeaderSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2075 | `ZSTDv05_decompress_continueDCtx` (c_src/src/legacy/zstd_v05.c:3403) | `if (cBlockSize > remainingSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2076 | `ZSTDv05_decompress_continueDCtx` (c_src/src/legacy/zstd_v05.c:3414) | `return ERROR(GENERIC); /* not yet supported */` | `ERROR(GENERIC)` | [x] |
| 2077 | `ZSTDv05_decompress_continueDCtx` (c_src/src/legacy/zstd_v05.c:3418) | `if (remainingSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2078 | `ZSTDv05_decompress_continueDCtx` (c_src/src/legacy/zstd_v05.c:3421) | `return ERROR(GENERIC); /* impossible */` | `ERROR(GENERIC)` | [x] |
| 2079 | `ZSTDv05_decompressDCtx` (c_src/src/legacy/zstd_v05.c:3458) | `return ZSTDv05_decompress_usingDict(dctx, dst, maxDstSize, src, srcSize, NULL, 0);` | source-declared rejection sentinel | [x] |
| 2080 | `ZSTDv05_decompress` (c_src/src/legacy/zstd_v05.c:3466) | `if (dctx==NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [x] |
| 2081 | `ZSTDv05_decompressContinue` (c_src/src/legacy/zstd_v05.c:3540) | `if (srcSize != dctx->expected) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2082 | `ZSTDv05_decompressContinue` (c_src/src/legacy/zstd_v05.c:3548) | `if (srcSize != ZSTDv05_frameHeaderSize_min) return ERROR(srcSize_wrong); /* impossible */` | `ERROR(srcSize_wrong)` | [x] |
| 2083 | `ZSTDv05_decompressContinue` (c_src/src/legacy/zstd_v05.c:3552) | `if (dctx->headerSize > ZSTDv05_frameHeaderSize_min) return ERROR(GENERIC); /* should never happen */` | `ERROR(GENERIC)` | [x] |
| 2084 | `ZSTDv05_decompressContinue` (c_src/src/legacy/zstd_v05.c:3593) | `return ERROR(GENERIC); /* not yet handled */` | `ERROR(GENERIC)` | [x] |
| 2085 | `ZSTDv05_decompressContinue` (c_src/src/legacy/zstd_v05.c:3599) | `return ERROR(GENERIC); /* impossible */` | `ERROR(GENERIC)` | [x] |
| 2086 | `ZSTDv05_decompressContinue` (c_src/src/legacy/zstd_v05.c:3608) | `return ERROR(GENERIC); /* impossible */` | `ERROR(GENERIC)` | [x] |
| 2087 | `ZSTDv05_loadEntropy` (c_src/src/legacy/zstd_v05.c:3632) | `if (HUFv05_isError(hSize)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [x] |
| 2088 | `ZSTDv05_loadEntropy` (c_src/src/legacy/zstd_v05.c:3637) | `if (FSEv05_isError(offcodeHeaderSize)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [x] |
| 2089 | `ZSTDv05_loadEntropy` (c_src/src/legacy/zstd_v05.c:3638) | `if (offcodeLog > OffFSEv05Log) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [x] |
| 2090 | `ZSTDv05_loadEntropy` (c_src/src/legacy/zstd_v05.c:3640) | `if (FSEv05_isError(errorCode)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [x] |
| 2091 | `ZSTDv05_loadEntropy` (c_src/src/legacy/zstd_v05.c:3645) | `if (FSEv05_isError(matchlengthHeaderSize)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [x] |
| 2092 | `ZSTDv05_loadEntropy` (c_src/src/legacy/zstd_v05.c:3646) | `if (matchlengthLog > MLFSEv05Log) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [x] |
| 2093 | `ZSTDv05_loadEntropy` (c_src/src/legacy/zstd_v05.c:3648) | `if (FSEv05_isError(errorCode)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [x] |
| 2094 | `ZSTDv05_loadEntropy` (c_src/src/legacy/zstd_v05.c:3653) | `if (litlengthLog > LLFSEv05Log) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [x] |
| 2095 | `ZSTDv05_loadEntropy` (c_src/src/legacy/zstd_v05.c:3654) | `if (FSEv05_isError(litlengthHeaderSize)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [x] |
| 2096 | `ZSTDv05_loadEntropy` (c_src/src/legacy/zstd_v05.c:3656) | `if (FSEv05_isError(errorCode)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [x] |
| 2097 | `ZSTDv05_decompress_insertDictionary` (c_src/src/legacy/zstd_v05.c:3675) | `if (ZSTDv05_isError(eSize)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [x] |
| 2098 | `ZSTDv05_decompressBegin_usingDict` (c_src/src/legacy/zstd_v05.c:3694) | `if (ZSTDv05_isError(errorCode)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [x] |
| 2099 | `ZBUFFv05_createDCtx` (c_src/src/legacy/zstd_v05.c:3807) | `if (zbc==NULL) return NULL;` | `NULL` | [x] |
| 2100 | `ZBUFFv05_decompressInit` (c_src/src/legacy/zstd_v05.c:3836) | `return ZBUFFv05_decompressInitDictionary(zbc, NULL, 0);` | source-declared rejection sentinel | [x] |
| 2101 | `ZBUFFv05_decompressContinue` (c_src/src/legacy/zstd_v05.c:3856) | `return ERROR(init_missing);` | `ERROR(init_missing)` | [x] |
| 2102 | `ZBUFFv05_decompressContinue` (c_src/src/legacy/zstd_v05.c:3902) | `if (zbc->inBuff == NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [x] |
| 2103 | `ZBUFFv05_decompressContinue` (c_src/src/legacy/zstd_v05.c:3908) | `if (zbc->outBuff == NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [x] |
| 2104 | `ZBUFFv05_decompressContinue` (c_src/src/legacy/zstd_v05.c:3949) | `if (toLoad > zbc->inBuffSize - zbc->inPos) return ERROR(corruption_detected); /* should never happen */` | `ERROR(corruption_detected)` | [x] |
| 2105 | `ZBUFFv05_decompressContinue` (c_src/src/legacy/zstd_v05.c:3983) | `default: return ERROR(GENERIC); /* impossible */` | `ERROR(GENERIC)` | [x] |
| 2106 | `BITv06_initDStream` (c_src/src/legacy/zstd_v06.c:835) | `if (srcSize < 1) { memset(bitD, 0, sizeof(*bitD)); return ERROR(srcSize_wrong); }` | `ERROR(srcSize_wrong)` | [x] |
| 2107 | `BITv06_initDStream` (c_src/src/legacy/zstd_v06.c:842) | `if (lastByte == 0) return ERROR(GENERIC); /* endMark not present */` | `ERROR(GENERIC)` | [x] |
| 2108 | `BITv06_initDStream` (c_src/src/legacy/zstd_v06.c:859) | `if (lastByte == 0) return ERROR(GENERIC); /* endMark not present */` | `ERROR(GENERIC)` | [x] |
| 2109 | `FSEv06_readNCount` (c_src/src/legacy/zstd_v06.c:1221) | `if (hbSize < 4) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2110 | `FSEv06_readNCount` (c_src/src/legacy/zstd_v06.c:1224) | `if (nbBits > FSEv06_TABLELOG_ABSOLUTE_MAX) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [x] |
| 2111 | `FSEv06_readNCount` (c_src/src/legacy/zstd_v06.c:1251) | `if (n0 > *maxSVPtr) return ERROR(maxSymbolValue_tooSmall);` | `ERROR(maxSymbolValue_tooSmall)` | [x] |
| 2112 | `FSEv06_readNCount` (c_src/src/legacy/zstd_v06.c:1291) | `if (remaining != 1) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [x] |
| 2113 | `FSEv06_readNCount` (c_src/src/legacy/zstd_v06.c:1295) | `if ((size_t)(ip-istart) > hbSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2114 | `FSEv06_buildDTable` (c_src/src/legacy/zstd_v06.c:1413) | `if (maxSymbolValue > FSEv06_MAX_SYMBOL_VALUE) return ERROR(maxSymbolValue_tooLarge);` | `ERROR(maxSymbolValue_tooLarge)` | [x] |
| 2115 | `FSEv06_buildDTable` (c_src/src/legacy/zstd_v06.c:1414) | `if (tableLog > FSEv06_MAX_TABLELOG) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [x] |
| 2116 | `FSEv06_buildDTable` (c_src/src/legacy/zstd_v06.c:1445) | `if (position!=0) return ERROR(GENERIC); /* position must reach all cells once, otherwise normalizedCounter is incorrect */` | `ERROR(GENERIC)` | [x] |
| 2117 | `FSEv06_buildDTable_raw` (c_src/src/legacy/zstd_v06.c:1497) | `if (nbBits < 1) return ERROR(GENERIC); /* min size */` | `ERROR(GENERIC)` | [x] |
| 2118 | `FSEv06_decompress_usingDTable_generic` (c_src/src/legacy/zstd_v06.c:1557) | `if (op>(omax-2)) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 2119 | `FSEv06_decompress_usingDTable_generic` (c_src/src/legacy/zstd_v06.c:1566) | `if (op>(omax-2)) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 2120 | `FSEv06_decompress` (c_src/src/legacy/zstd_v06.c:1602) | `if (cSrcSize<2) return ERROR(srcSize_wrong); /* too small input size */` | `ERROR(srcSize_wrong)` | [x] |
| 2121 | `FSEv06_decompress` (c_src/src/legacy/zstd_v06.c:1607) | `if (NCountLength >= cSrcSize) return ERROR(srcSize_wrong); /* too small input size */` | `ERROR(srcSize_wrong)` | [x] |
| 2122 | `HUFv06_readStats` (c_src/src/legacy/zstd_v06.c:1807) | `if (!srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2123 | `HUFv06_readStats` (c_src/src/legacy/zstd_v06.c:1821) | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2124 | `HUFv06_readStats` (c_src/src/legacy/zstd_v06.c:1822) | `if (oSize >= hwSize) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2125 | `HUFv06_readStats` (c_src/src/legacy/zstd_v06.c:1830) | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2126 | `HUFv06_readStats` (c_src/src/legacy/zstd_v06.c:1839) | `if (huffWeight[n] >= HUFv06_ABSOLUTEMAX_TABLELOG) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2127 | `HUFv06_readStats` (c_src/src/legacy/zstd_v06.c:1843) | `if (weightTotal == 0) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2128 | `HUFv06_readStats` (c_src/src/legacy/zstd_v06.c:1847) | `if (tableLog > HUFv06_ABSOLUTEMAX_TABLELOG) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2129 | `HUFv06_readStats` (c_src/src/legacy/zstd_v06.c:1854) | `if (verif != rest) return ERROR(corruption_detected); /* last value must be a clean power of 2 */` | `ERROR(corruption_detected)` | [x] |
| 2130 | `HUFv06_readStats` (c_src/src/legacy/zstd_v06.c:1860) | `if ((rankStats[1] < 2) \|\| (rankStats[1] & 1)) return ERROR(corruption_detected); /* by construction : at least 2 elts of rank 1, must be even */` | `ERROR(corruption_detected)` | [x] |
| 2131 | `HUFv06_readDTableX2` (c_src/src/legacy/zstd_v06.c:1967) | `if (tableLog > DTable[0]) return ERROR(tableLog_tooLarge); /* DTable is too small */` | `ERROR(tableLog_tooLarge)` | [x] |
| 2132 | `HUFv06_decompress1X2_usingDTable` (c_src/src/legacy/zstd_v06.c:2054) | `if (!BITv06_endOfDStream(&bitD)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2133 | `HUFv06_decompress1X2` (c_src/src/legacy/zstd_v06.c:2066) | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2134 | `HUFv06_decompress4X2_usingDTable` (c_src/src/legacy/zstd_v06.c:2080) | `if (cSrcSize < 10) return ERROR(corruption_detected); /* strict minimum : jump table + 1 byte per stream */` | `ERROR(corruption_detected)` | [x] |
| 2135 | `HUFv06_decompress4X2_usingDTable` (c_src/src/legacy/zstd_v06.c:2114) | `if (length4 > cSrcSize) return ERROR(corruption_detected); /* overflow */` | `ERROR(corruption_detected)` | [x] |
| 2136 | `HUFv06_decompress4X2_usingDTable` (c_src/src/legacy/zstd_v06.c:2147) | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2137 | `HUFv06_decompress4X2_usingDTable` (c_src/src/legacy/zstd_v06.c:2148) | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2138 | `HUFv06_decompress4X2_usingDTable` (c_src/src/legacy/zstd_v06.c:2149) | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2139 | `HUFv06_decompress4X2_usingDTable` (c_src/src/legacy/zstd_v06.c:2160) | `if (!endSignal) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2140 | `HUFv06_decompress4X2` (c_src/src/legacy/zstd_v06.c:2175) | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2141 | `HUFv06_readDTableX4` (c_src/src/legacy/zstd_v06.c:2286) | `if (memLog > HUFv06_ABSOLUTEMAX_TABLELOG) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [x] |
| 2142 | `HUFv06_readDTableX4` (c_src/src/legacy/zstd_v06.c:2293) | `if (tableLog > memLog) return ERROR(tableLog_tooLarge); /* DTable can't fit code depth */` | `ERROR(tableLog_tooLarge)` | [x] |
| 2143 | `HUFv06_decompress1X4_usingDTable` (c_src/src/legacy/zstd_v06.c:2430) | `if (!BITv06_endOfDStream(&bitD)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2144 | `HUFv06_decompress1X4` (c_src/src/legacy/zstd_v06.c:2443) | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2145 | `HUFv06_decompress4X4_usingDTable` (c_src/src/legacy/zstd_v06.c:2455) | `if (cSrcSize < 10) return ERROR(corruption_detected); /* strict minimum : jump table + 1 byte per stream */` | `ERROR(corruption_detected)` | [x] |
| 2146 | `HUFv06_decompress4X4_usingDTable` (c_src/src/legacy/zstd_v06.c:2489) | `if (length4 > cSrcSize) return ERROR(corruption_detected); /* overflow */` | `ERROR(corruption_detected)` | [x] |
| 2147 | `HUFv06_decompress4X4_usingDTable` (c_src/src/legacy/zstd_v06.c:2523) | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2148 | `HUFv06_decompress4X4_usingDTable` (c_src/src/legacy/zstd_v06.c:2524) | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2149 | `HUFv06_decompress4X4_usingDTable` (c_src/src/legacy/zstd_v06.c:2525) | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2150 | `HUFv06_decompress4X4_usingDTable` (c_src/src/legacy/zstd_v06.c:2536) | `if (!endSignal) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2151 | `HUFv06_decompress4X4` (c_src/src/legacy/zstd_v06.c:2551) | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2152 | `HUFv06_decompress` (c_src/src/legacy/zstd_v06.c:2595) | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 2153 | `HUFv06_decompress` (c_src/src/legacy/zstd_v06.c:2596) | `if (cSrcSize > dstSize) return ERROR(corruption_detected); /* invalid */` | `ERROR(corruption_detected)` | [x] |
| 2154 | `ZSTDv06_createDCtx` (c_src/src/legacy/zstd_v06.c:2789) | `if (dctx==NULL) return NULL;` | `NULL` | [x] |
| 2155 | `ZSTDv06_frameHeaderSize` (c_src/src/legacy/zstd_v06.c:2913) | `if (srcSize < ZSTDv06_frameHeaderSize_min) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2156 | `ZSTDv06_getFrameParams` (c_src/src/legacy/zstd_v06.c:2929) | `if (MEM_readLE32(src) != ZSTDv06_MAGICNUMBER) return ERROR(prefix_unknown);` | `ERROR(prefix_unknown)` | [x] |
| 2157 | `ZSTDv06_getFrameParams` (c_src/src/legacy/zstd_v06.c:2938) | `if ((frameDesc & 0x20) != 0) return ERROR(frameParameter_unsupported); /* reserved 1 bit */` | `ERROR(frameParameter_unsupported)` | [x] |
| 2158 | `ZSTDv06_decodeFrameHeader` (c_src/src/legacy/zstd_v06.c:2957) | `if ((MEM_32bits()) && (zc->fParams.windowLog > 25)) return ERROR(frameParameter_unsupported);` | `ERROR(frameParameter_unsupported)` | [x] |
| 2159 | `ZSTDv06_getcBlockSize` (c_src/src/legacy/zstd_v06.c:2975) | `if (srcSize < ZSTDv06_blockHeaderSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2160 | `ZSTDv06_copyRawBlock` (c_src/src/legacy/zstd_v06.c:2989) | `if (dst==NULL) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 2161 | `ZSTDv06_copyRawBlock` (c_src/src/legacy/zstd_v06.c:2990) | `if (srcSize > dstCapacity) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 2162 | `ZSTDv06_decodeLiteralsBlock` (c_src/src/legacy/zstd_v06.c:3004) | `if (srcSize < MIN_CBLOCK_SIZE) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2163 | `ZSTDv06_decodeLiteralsBlock` (c_src/src/legacy/zstd_v06.c:3011) | `if (srcSize < 5) return ERROR(corruption_detected); /* srcSize >= MIN_CBLOCK_SIZE == 3; here we need up to 5 for lhSize, + cSize (+nbSeq) */` | `ERROR(corruption_detected)` | [x] |
| 2164 | `ZSTDv06_decodeLiteralsBlock` (c_src/src/legacy/zstd_v06.c:3034) | `if (litSize > ZSTDv06_BLOCKSIZE_MAX) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2165 | `ZSTDv06_decodeLiteralsBlock` (c_src/src/legacy/zstd_v06.c:3035) | `if (litCSize + lhSize > srcSize) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2166 | `ZSTDv06_decodeLiteralsBlock` (c_src/src/legacy/zstd_v06.c:3040) | `return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2167 | `ZSTDv06_decodeLiteralsBlock` (c_src/src/legacy/zstd_v06.c:3051) | `return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2168 | `ZSTDv06_decodeLiteralsBlock` (c_src/src/legacy/zstd_v06.c:3053) | `return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [x] |
| 2169 | `ZSTDv06_decodeLiteralsBlock` (c_src/src/legacy/zstd_v06.c:3059) | `if (litCSize + lhSize > srcSize) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2170 | `ZSTDv06_decodeLiteralsBlock` (c_src/src/legacy/zstd_v06.c:3062) | `if (HUFv06_isError(errorCode)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2171 | `ZSTDv06_decodeLiteralsBlock` (c_src/src/legacy/zstd_v06.c:3087) | `if (litSize+lhSize > srcSize) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2172 | `ZSTDv06_decodeLiteralsBlock` (c_src/src/legacy/zstd_v06.c:3113) | `if (srcSize<4) return ERROR(corruption_detected); /* srcSize >= MIN_CBLOCK_SIZE == 3; here we need lhSize+1 = 4 */` | `ERROR(corruption_detected)` | [x] |
| 2173 | `ZSTDv06_decodeLiteralsBlock` (c_src/src/legacy/zstd_v06.c:3116) | `if (litSize > ZSTDv06_BLOCKSIZE_MAX) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2174 | `ZSTDv06_decodeLiteralsBlock` (c_src/src/legacy/zstd_v06.c:3123) | `return ERROR(corruption_detected); /* impossible */` | `ERROR(corruption_detected)` | [x] |
| 2175 | `ZSTDv06_buildSeqTable` (c_src/src/legacy/zstd_v06.c:3139) | `if (!srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2176 | `ZSTDv06_buildSeqTable` (c_src/src/legacy/zstd_v06.c:3140) | `if ( (*(const BYTE*)src) > max) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2177 | `ZSTDv06_buildSeqTable` (c_src/src/legacy/zstd_v06.c:3147) | `if (!flagRepeatTable) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2178 | `ZSTDv06_buildSeqTable` (c_src/src/legacy/zstd_v06.c:3154) | `if (FSEv06_isError(headerSize)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2179 | `ZSTDv06_buildSeqTable` (c_src/src/legacy/zstd_v06.c:3155) | `if (tableLog > maxLog) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2180 | `ZSTDv06_decodeSeqHeaders` (c_src/src/legacy/zstd_v06.c:3171) | `if (srcSize < MIN_SEQUENCES_SIZE) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2181 | `ZSTDv06_decodeSeqHeaders` (c_src/src/legacy/zstd_v06.c:3178) | `if (ip+2 > iend) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2182 | `ZSTDv06_decodeSeqHeaders` (c_src/src/legacy/zstd_v06.c:3181) | `if (ip >= iend) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2183 | `ZSTDv06_decodeSeqHeaders` (c_src/src/legacy/zstd_v06.c:3189) | `if (ip + 4 > iend) return ERROR(srcSize_wrong); /* min : header byte + all 3 are "raw", hence no header, but at least xxLog bits per type */` | `ERROR(srcSize_wrong)` | [x] |
| 2184 | `ZSTDv06_decodeSeqHeaders` (c_src/src/legacy/zstd_v06.c:3197) | `if (ZSTDv06_isError(bhSize)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2185 | `ZSTDv06_decodeSeqHeaders` (c_src/src/legacy/zstd_v06.c:3201) | `if (ZSTDv06_isError(bhSize)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2186 | `ZSTDv06_decodeSeqHeaders` (c_src/src/legacy/zstd_v06.c:3205) | `if (ZSTDv06_isError(bhSize)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2187 | `ZSTDv06_execSequence` (c_src/src/legacy/zstd_v06.c:3320) | `if (seqLength > (size_t)(oend - op)) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 2188 | `ZSTDv06_execSequence` (c_src/src/legacy/zstd_v06.c:3321) | `if (sequence.litLength > (size_t)(litLimit - *litPtr)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2189 | `ZSTDv06_execSequence` (c_src/src/legacy/zstd_v06.c:3323) | `if (oLitEnd > oend_8) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 2190 | `ZSTDv06_execSequence` (c_src/src/legacy/zstd_v06.c:3325) | `if (oMatchEnd > oend) return ERROR(dstSize_tooSmall); /* overwrite beyond dst buffer */` | `ERROR(dstSize_tooSmall)` | [x] |
| 2191 | `ZSTDv06_execSequence` (c_src/src/legacy/zstd_v06.c:3326) | `if (iLitEnd > litLimit) return ERROR(corruption_detected); /* overRead beyond lit buffer */` | `ERROR(corruption_detected)` | [x] |
| 2192 | `ZSTDv06_execSequence` (c_src/src/legacy/zstd_v06.c:3336) | `if (sequence.offset > (size_t)(oLitEnd - vBase)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2193 | `ZSTDv06_decompressSequences` (c_src/src/legacy/zstd_v06.c:3423) | `if (ERR_isError(errorCode)) return ERROR(corruption_detected); }` | `ERROR(corruption_detected)` | [x] |
| 2194 | `ZSTDv06_decompressSequences` (c_src/src/legacy/zstd_v06.c:3447) | `if (nbSeq) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2195 | `ZSTDv06_decompressSequences` (c_src/src/legacy/zstd_v06.c:3452) | `if (litPtr > litEnd) return ERROR(corruption_detected); /* too many literals already used */` | `ERROR(corruption_detected)` | [x] |
| 2196 | `ZSTDv06_decompressSequences` (c_src/src/legacy/zstd_v06.c:3453) | `if (op+lastLLSize > oend) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 2197 | `ZSTDv06_decompressBlock_internal` (c_src/src/legacy/zstd_v06.c:3481) | `if (srcSize >= ZSTDv06_BLOCKSIZE_MAX) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2198 | `ZSTDv06_decompressFrame` (c_src/src/legacy/zstd_v06.c:3517) | `if (srcSize < ZSTDv06_frameHeaderSize_min+ZSTDv06_blockHeaderSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2199 | `ZSTDv06_decompressFrame` (c_src/src/legacy/zstd_v06.c:3522) | `if (srcSize < frameHeaderSize+ZSTDv06_blockHeaderSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2200 | `ZSTDv06_decompressFrame` (c_src/src/legacy/zstd_v06.c:3523) | `if (ZSTDv06_decodeFrameHeader(dctx, src, frameHeaderSize)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2201 | `ZSTDv06_decompressFrame` (c_src/src/legacy/zstd_v06.c:3535) | `if (cBlockSize > remainingSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2202 | `ZSTDv06_decompressFrame` (c_src/src/legacy/zstd_v06.c:3546) | `return ERROR(GENERIC); /* not yet supported */` | `ERROR(GENERIC)` | [x] |
| 2203 | `ZSTDv06_decompressFrame` (c_src/src/legacy/zstd_v06.c:3550) | `if (remainingSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2204 | `ZSTDv06_decompressFrame` (c_src/src/legacy/zstd_v06.c:3553) | `return ERROR(GENERIC); /* impossible */` | `ERROR(GENERIC)` | [x] |
| 2205 | `ZSTDv06_decompressDCtx` (c_src/src/legacy/zstd_v06.c:3590) | `return ZSTDv06_decompress_usingDict(dctx, dst, dstCapacity, src, srcSize, NULL, 0);` | source-declared rejection sentinel | [x] |
| 2206 | `ZSTDv06_decompress` (c_src/src/legacy/zstd_v06.c:3599) | `if (dctx==NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [x] |
| 2207 | `ZSTDv06_decompressContinue` (c_src/src/legacy/zstd_v06.c:3678) | `if (srcSize != dctx->expected) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2208 | `ZSTDv06_decompressContinue` (c_src/src/legacy/zstd_v06.c:3685) | `if (srcSize != ZSTDv06_frameHeaderSize_min) return ERROR(srcSize_wrong); /* impossible */` | `ERROR(srcSize_wrong)` | [x] |
| 2209 | `ZSTDv06_decompressContinue` (c_src/src/legacy/zstd_v06.c:3730) | `return ERROR(GENERIC); /* not yet handled */` | `ERROR(GENERIC)` | [x] |
| 2210 | `ZSTDv06_decompressContinue` (c_src/src/legacy/zstd_v06.c:3736) | `return ERROR(GENERIC); /* impossible */` | `ERROR(GENERIC)` | [x] |
| 2211 | `ZSTDv06_decompressContinue` (c_src/src/legacy/zstd_v06.c:3745) | `return ERROR(GENERIC); /* impossible */` | `ERROR(GENERIC)` | [x] |
| 2212 | `ZSTDv06_loadEntropy` (c_src/src/legacy/zstd_v06.c:3763) | `if (HUFv06_isError(hSize)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [x] |
| 2213 | `ZSTDv06_loadEntropy` (c_src/src/legacy/zstd_v06.c:3770) | `if (FSEv06_isError(offcodeHeaderSize)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [x] |
| 2214 | `ZSTDv06_loadEntropy` (c_src/src/legacy/zstd_v06.c:3771) | `if (offcodeLog > OffFSELog) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [x] |
| 2215 | `ZSTDv06_loadEntropy` (c_src/src/legacy/zstd_v06.c:3773) | `if (FSEv06_isError(errorCode)) return ERROR(dictionary_corrupted); }` | `ERROR(dictionary_corrupted)` | [x] |
| 2216 | `ZSTDv06_loadEntropy` (c_src/src/legacy/zstd_v06.c:3781) | `if (FSEv06_isError(matchlengthHeaderSize)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [x] |
| 2217 | `ZSTDv06_loadEntropy` (c_src/src/legacy/zstd_v06.c:3782) | `if (matchlengthLog > MLFSELog) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [x] |
| 2218 | `ZSTDv06_loadEntropy` (c_src/src/legacy/zstd_v06.c:3784) | `if (FSEv06_isError(errorCode)) return ERROR(dictionary_corrupted); }` | `ERROR(dictionary_corrupted)` | [x] |
| 2219 | `ZSTDv06_loadEntropy` (c_src/src/legacy/zstd_v06.c:3792) | `if (FSEv06_isError(litlengthHeaderSize)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [x] |
| 2220 | `ZSTDv06_loadEntropy` (c_src/src/legacy/zstd_v06.c:3793) | `if (litlengthLog > LLFSELog) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [x] |
| 2221 | `ZSTDv06_loadEntropy` (c_src/src/legacy/zstd_v06.c:3795) | `if (FSEv06_isError(errorCode)) return ERROR(dictionary_corrupted); }` | `ERROR(dictionary_corrupted)` | [x] |
| 2222 | `ZSTDv06_decompress_insertDictionary` (c_src/src/legacy/zstd_v06.c:3815) | `if (ZSTDv06_isError(eSize)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [x] |
| 2223 | `ZSTDv06_decompressBegin_usingDict` (c_src/src/legacy/zstd_v06.c:3833) | `if (ZSTDv06_isError(errorCode)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [x] |
| 2224 | `ZBUFFv06_createDCtx` (c_src/src/legacy/zstd_v06.c:3919) | `if (zbd==NULL) return NULL;` | `NULL` | [x] |
| 2225 | `ZBUFFv06_createDCtx` (c_src/src/legacy/zstd_v06.c:3924) | `return NULL;` | `NULL` | [x] |
| 2226 | `ZBUFFv06_decompressInit` (c_src/src/legacy/zstd_v06.c:3952) | `return ZBUFFv06_decompressInitDictionary(zbd, NULL, 0);` | source-declared rejection sentinel | [x] |
| 2227 | `ZBUFFv06_decompressContinue` (c_src/src/legacy/zstd_v06.c:3985) | `return ERROR(init_missing);` | `ERROR(init_missing)` | [x] |
| 2228 | `ZBUFFv06_decompressContinue` (c_src/src/legacy/zstd_v06.c:4020) | `if (zbd->inBuff == NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [x] |
| 2229 | `ZBUFFv06_decompressContinue` (c_src/src/legacy/zstd_v06.c:4027) | `if (zbd->outBuff == NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [x] |
| 2230 | `ZBUFFv06_decompressContinue` (c_src/src/legacy/zstd_v06.c:4057) | `if (toLoad > zbd->inBuffSize - zbd->inPos) return ERROR(corruption_detected); /* should never happen */` | `ERROR(corruption_detected)` | [x] |
| 2231 | `ZBUFFv06_decompressContinue` (c_src/src/legacy/zstd_v06.c:4091) | `default: return ERROR(GENERIC); /* impossible */` | `ERROR(GENERIC)` | [x] |
| 2232 | `BITv07_initDStream` (c_src/src/legacy/zstd_v07.c:504) | `if (srcSize < 1) { memset(bitD, 0, sizeof(*bitD)); return ERROR(srcSize_wrong); }` | `ERROR(srcSize_wrong)` | [x] |
| 2233 | `BITv07_initDStream` (c_src/src/legacy/zstd_v07.c:512) | `if (lastByte == 0) return ERROR(GENERIC); /* endMark not present */ }` | `ERROR(GENERIC)` | [x] |
| 2234 | `BITv07_initDStream` (c_src/src/legacy/zstd_v07.c:529) | `if (lastByte == 0) return ERROR(GENERIC); /* endMark not present */ }` | `ERROR(GENERIC)` | [x] |
| 2235 | `FSEv07_readNCount` (c_src/src/legacy/zstd_v07.c:1166) | `if (hbSize < 4) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2236 | `FSEv07_readNCount` (c_src/src/legacy/zstd_v07.c:1169) | `if (nbBits > FSEv07_TABLELOG_ABSOLUTE_MAX) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [x] |
| 2237 | `FSEv07_readNCount` (c_src/src/legacy/zstd_v07.c:1196) | `if (n0 > *maxSVPtr) return ERROR(maxSymbolValue_tooSmall);` | `ERROR(maxSymbolValue_tooSmall)` | [x] |
| 2238 | `FSEv07_readNCount` (c_src/src/legacy/zstd_v07.c:1236) | `if (remaining != 1) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [x] |
| 2239 | `FSEv07_readNCount` (c_src/src/legacy/zstd_v07.c:1240) | `if ((size_t)(ip-istart) > hbSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2240 | `HUFv07_readStats` (c_src/src/legacy/zstd_v07.c:1260) | `if (!srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2241 | `HUFv07_readStats` (c_src/src/legacy/zstd_v07.c:1274) | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2242 | `HUFv07_readStats` (c_src/src/legacy/zstd_v07.c:1275) | `if (oSize >= hwSize) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2243 | `HUFv07_readStats` (c_src/src/legacy/zstd_v07.c:1283) | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2244 | `HUFv07_readStats` (c_src/src/legacy/zstd_v07.c:1292) | `if (huffWeight[n] >= HUFv07_TABLELOG_ABSOLUTEMAX) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2245 | `HUFv07_readStats` (c_src/src/legacy/zstd_v07.c:1296) | `if (weightTotal == 0) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2246 | `HUFv07_readStats` (c_src/src/legacy/zstd_v07.c:1300) | `if (tableLog > HUFv07_TABLELOG_ABSOLUTEMAX) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2247 | `HUFv07_readStats` (c_src/src/legacy/zstd_v07.c:1307) | `if (verif != rest) return ERROR(corruption_detected); /* last value must be a clean power of 2 */` | `ERROR(corruption_detected)` | [x] |
| 2248 | `HUFv07_readStats` (c_src/src/legacy/zstd_v07.c:1313) | `if ((rankStats[1] < 2) \|\| (rankStats[1] & 1)) return ERROR(corruption_detected); /* by construction : at least 2 elts of rank 1, must be even */` | `ERROR(corruption_detected)` | [x] |
| 2249 | `FSEv07_buildDTable` (c_src/src/legacy/zstd_v07.c:1434) | `if (maxSymbolValue > FSEv07_MAX_SYMBOL_VALUE) return ERROR(maxSymbolValue_tooLarge);` | `ERROR(maxSymbolValue_tooLarge)` | [x] |
| 2250 | `FSEv07_buildDTable` (c_src/src/legacy/zstd_v07.c:1435) | `if (tableLog > FSEv07_MAX_TABLELOG) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [x] |
| 2251 | `FSEv07_buildDTable` (c_src/src/legacy/zstd_v07.c:1466) | `if (position!=0) return ERROR(GENERIC); /* position must reach all cells once, otherwise normalizedCounter is incorrect */` | `ERROR(GENERIC)` | [x] |
| 2252 | `FSEv07_buildDTable_raw` (c_src/src/legacy/zstd_v07.c:1518) | `if (nbBits < 1) return ERROR(GENERIC); /* min size */` | `ERROR(GENERIC)` | [x] |
| 2253 | `FSEv07_decompress_usingDTable_generic` (c_src/src/legacy/zstd_v07.c:1578) | `if (op>(omax-2)) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 2254 | `FSEv07_decompress_usingDTable_generic` (c_src/src/legacy/zstd_v07.c:1587) | `if (op>(omax-2)) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 2255 | `FSEv07_decompress` (c_src/src/legacy/zstd_v07.c:1623) | `if (cSrcSize<2) return ERROR(srcSize_wrong); /* too small input size */` | `ERROR(srcSize_wrong)` | [x] |
| 2256 | `FSEv07_decompress` (c_src/src/legacy/zstd_v07.c:1628) | `if (NCountLength >= cSrcSize) return ERROR(srcSize_wrong); /* too small input size */` | `ERROR(srcSize_wrong)` | [x] |
| 2257 | `HUFv07_readDTableX2` (c_src/src/legacy/zstd_v07.c:1739) | `if (tableLog > (U32)(dtd.maxTableLog+1)) return ERROR(tableLog_tooLarge); /* DTable too small, huffman tree cannot fit in */` | `ERROR(tableLog_tooLarge)` | [x] |
| 2258 | `HUFv07_decompress1X2_usingDTable_internal` (c_src/src/legacy/zstd_v07.c:1831) | `if (!BITv07_endOfDStream(&bitD)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2259 | `HUFv07_decompress1X2_usingDTable` (c_src/src/legacy/zstd_v07.c:1842) | `if (dtd.tableType != 0) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [x] |
| 2260 | `HUFv07_decompress1X2_DCtx` (c_src/src/legacy/zstd_v07.c:1852) | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2261 | `HUFv07_decompress4X2_usingDTable_internal` (c_src/src/legacy/zstd_v07.c:1871) | `if (cSrcSize < 10) return ERROR(corruption_detected); /* strict minimum : jump table + 1 byte per stream */` | `ERROR(corruption_detected)` | [x] |
| 2262 | `HUFv07_decompress4X2_usingDTable_internal` (c_src/src/legacy/zstd_v07.c:1904) | `if (length4 > cSrcSize) return ERROR(corruption_detected); /* overflow */` | `ERROR(corruption_detected)` | [x] |
| 2263 | `HUFv07_decompress4X2_usingDTable_internal` (c_src/src/legacy/zstd_v07.c:1937) | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2264 | `HUFv07_decompress4X2_usingDTable_internal` (c_src/src/legacy/zstd_v07.c:1938) | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2265 | `HUFv07_decompress4X2_usingDTable_internal` (c_src/src/legacy/zstd_v07.c:1939) | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2266 | `HUFv07_decompress4X2_usingDTable_internal` (c_src/src/legacy/zstd_v07.c:1950) | `if (!endSignal) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2267 | `HUFv07_decompress4X2_usingDTable` (c_src/src/legacy/zstd_v07.c:1964) | `if (dtd.tableType != 0) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [x] |
| 2268 | `HUFv07_decompress4X2_DCtx` (c_src/src/legacy/zstd_v07.c:1975) | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2269 | `HUFv07_readDTableX4` (c_src/src/legacy/zstd_v07.c:2095) | `if (maxTableLog > HUFv07_TABLELOG_ABSOLUTEMAX) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [x] |
| 2270 | `HUFv07_readDTableX4` (c_src/src/legacy/zstd_v07.c:2102) | `if (tableLog > maxTableLog) return ERROR(tableLog_tooLarge); /* DTable can't fit code depth */` | `ERROR(tableLog_tooLarge)` | [x] |
| 2271 | `HUFv07_decompress1X4_usingDTable_internal` (c_src/src/legacy/zstd_v07.c:2242) | `if (!BITv07_endOfDStream(&bitD)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2272 | `HUFv07_decompress1X4_usingDTable` (c_src/src/legacy/zstd_v07.c:2254) | `if (dtd.tableType != 1) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [x] |
| 2273 | `HUFv07_decompress1X4_DCtx` (c_src/src/legacy/zstd_v07.c:2264) | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2274 | `HUFv07_decompress4X4_usingDTable_internal` (c_src/src/legacy/zstd_v07.c:2281) | `if (cSrcSize < 10) return ERROR(corruption_detected); /* strict minimum : jump table + 1 byte per stream */` | `ERROR(corruption_detected)` | [x] |
| 2275 | `HUFv07_decompress4X4_usingDTable_internal` (c_src/src/legacy/zstd_v07.c:2314) | `if (length4 > cSrcSize) return ERROR(corruption_detected); /* overflow */` | `ERROR(corruption_detected)` | [x] |
| 2276 | `HUFv07_decompress4X4_usingDTable_internal` (c_src/src/legacy/zstd_v07.c:2348) | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2277 | `HUFv07_decompress4X4_usingDTable_internal` (c_src/src/legacy/zstd_v07.c:2349) | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2278 | `HUFv07_decompress4X4_usingDTable_internal` (c_src/src/legacy/zstd_v07.c:2350) | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2279 | `HUFv07_decompress4X4_usingDTable_internal` (c_src/src/legacy/zstd_v07.c:2361) | `if (!endCheck) return ERROR(corruption_detected); }` | `ERROR(corruption_detected)` | [x] |
| 2280 | `HUFv07_decompress4X4_usingDTable` (c_src/src/legacy/zstd_v07.c:2375) | `if (dtd.tableType != 1) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [x] |
| 2281 | `HUFv07_decompress4X4_DCtx` (c_src/src/legacy/zstd_v07.c:2386) | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2282 | `HUFv07_decompress` (c_src/src/legacy/zstd_v07.c:2469) | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 2283 | `HUFv07_decompress` (c_src/src/legacy/zstd_v07.c:2470) | `if (cSrcSize > dstSize) return ERROR(corruption_detected); /* invalid */` | `ERROR(corruption_detected)` | [x] |
| 2284 | `HUFv07_decompress4X_DCtx` (c_src/src/legacy/zstd_v07.c:2485) | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 2285 | `HUFv07_decompress4X_DCtx` (c_src/src/legacy/zstd_v07.c:2486) | `if (cSrcSize > dstSize) return ERROR(corruption_detected); /* invalid */` | `ERROR(corruption_detected)` | [x] |
| 2286 | `HUFv07_decompress4X_hufOnly` (c_src/src/legacy/zstd_v07.c:2499) | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 2287 | `HUFv07_decompress4X_hufOnly` (c_src/src/legacy/zstd_v07.c:2500) | `if ((cSrcSize >= dstSize) \|\| (cSrcSize <= 1)) return ERROR(corruption_detected); /* invalid */` | `ERROR(corruption_detected)` | [x] |
| 2288 | `HUFv07_decompress1X_DCtx` (c_src/src/legacy/zstd_v07.c:2511) | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 2289 | `HUFv07_decompress1X_DCtx` (c_src/src/legacy/zstd_v07.c:2512) | `if (cSrcSize > dstSize) return ERROR(corruption_detected); /* invalid */` | `ERROR(corruption_detected)` | [x] |
| 2290 | `ZSTDv07_createDCtx_advanced` (c_src/src/legacy/zstd_v07.c:2930) | `return NULL;` | `NULL` | [x] |
| 2291 | `ZSTDv07_createDCtx_advanced` (c_src/src/legacy/zstd_v07.c:2933) | `if (!dctx) return NULL;` | `NULL` | [x] |
| 2292 | `ZSTDv07_frameHeaderSize` (c_src/src/legacy/zstd_v07.c:3079) | `if (srcSize < ZSTDv07_frameHeaderSize_min) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2293 | `ZSTDv07_getFrameParams` (c_src/src/legacy/zstd_v07.c:3108) | `return ERROR(prefix_unknown);` | `ERROR(prefix_unknown)` | [x] |
| 2294 | `ZSTDv07_getFrameParams` (c_src/src/legacy/zstd_v07.c:3126) | `return ERROR(frameParameter_unsupported);` | `ERROR(frameParameter_unsupported)` | [x] |
| 2295 | `ZSTDv07_getFrameParams` (c_src/src/legacy/zstd_v07.c:3131) | `return ERROR(frameParameter_unsupported);` | `ERROR(frameParameter_unsupported)` | [x] |
| 2296 | `ZSTDv07_getFrameParams` (c_src/src/legacy/zstd_v07.c:3154) | `return ERROR(frameParameter_unsupported);` | `ERROR(frameParameter_unsupported)` | [x] |
| 2297 | `ZSTDv07_decodeFrameHeader` (c_src/src/legacy/zstd_v07.c:3186) | `if (dctx->fParams.dictID && (dctx->dictID != dctx->fParams.dictID)) return ERROR(dictionary_wrong);` | `ERROR(dictionary_wrong)` | [x] |
| 2298 | `ZSTDv07_getcBlockSize` (c_src/src/legacy/zstd_v07.c:3205) | `if (srcSize < ZSTDv07_blockHeaderSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2299 | `ZSTDv07_copyRawBlock` (c_src/src/legacy/zstd_v07.c:3219) | `if (srcSize > dstCapacity) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 2300 | `ZSTDv07_decodeLiteralsBlock` (c_src/src/legacy/zstd_v07.c:3234) | `if (srcSize < MIN_CBLOCK_SIZE) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2301 | `ZSTDv07_decodeLiteralsBlock` (c_src/src/legacy/zstd_v07.c:3241) | `if (srcSize < 5) return ERROR(corruption_detected); /* srcSize >= MIN_CBLOCK_SIZE == 3; here we need up to 5 for lhSize, + cSize (+nbSeq) */` | `ERROR(corruption_detected)` | [x] |
| 2302 | `ZSTDv07_decodeLiteralsBlock` (c_src/src/legacy/zstd_v07.c:3264) | `if (litSize > ZSTDv07_BLOCKSIZE_ABSOLUTEMAX) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2303 | `ZSTDv07_decodeLiteralsBlock` (c_src/src/legacy/zstd_v07.c:3265) | `if (litCSize + lhSize > srcSize) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2304 | `ZSTDv07_decodeLiteralsBlock` (c_src/src/legacy/zstd_v07.c:3270) | `return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2305 | `ZSTDv07_decodeLiteralsBlock` (c_src/src/legacy/zstd_v07.c:3282) | `return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2306 | `ZSTDv07_decodeLiteralsBlock` (c_src/src/legacy/zstd_v07.c:3284) | `return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [x] |
| 2307 | `ZSTDv07_decodeLiteralsBlock` (c_src/src/legacy/zstd_v07.c:3290) | `if (litCSize + lhSize > srcSize) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2308 | `ZSTDv07_decodeLiteralsBlock` (c_src/src/legacy/zstd_v07.c:3293) | `if (HUFv07_isError(errorCode)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2309 | `ZSTDv07_decodeLiteralsBlock` (c_src/src/legacy/zstd_v07.c:3318) | `if (litSize+lhSize > srcSize) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2310 | `ZSTDv07_decodeLiteralsBlock` (c_src/src/legacy/zstd_v07.c:3344) | `if (srcSize<4) return ERROR(corruption_detected); /* srcSize >= MIN_CBLOCK_SIZE == 3; here we need lhSize+1 = 4 */` | `ERROR(corruption_detected)` | [x] |
| 2311 | `ZSTDv07_decodeLiteralsBlock` (c_src/src/legacy/zstd_v07.c:3347) | `if (litSize > ZSTDv07_BLOCKSIZE_ABSOLUTEMAX) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2312 | `ZSTDv07_decodeLiteralsBlock` (c_src/src/legacy/zstd_v07.c:3354) | `return ERROR(corruption_detected); /* impossible */` | `ERROR(corruption_detected)` | [x] |
| 2313 | `ZSTDv07_buildSeqTable` (c_src/src/legacy/zstd_v07.c:3370) | `if (!srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2314 | `ZSTDv07_buildSeqTable` (c_src/src/legacy/zstd_v07.c:3371) | `if ( (*(const BYTE*)src) > max) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2315 | `ZSTDv07_buildSeqTable` (c_src/src/legacy/zstd_v07.c:3378) | `if (!flagRepeatTable) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2316 | `ZSTDv07_buildSeqTable` (c_src/src/legacy/zstd_v07.c:3385) | `if (FSEv07_isError(headerSize)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2317 | `ZSTDv07_buildSeqTable` (c_src/src/legacy/zstd_v07.c:3386) | `if (tableLog > maxLog) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2318 | `ZSTDv07_decodeSeqHeaders` (c_src/src/legacy/zstd_v07.c:3402) | `if (srcSize < MIN_SEQUENCES_SIZE) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2319 | `ZSTDv07_decodeSeqHeaders` (c_src/src/legacy/zstd_v07.c:3409) | `if (ip+2 > iend) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2320 | `ZSTDv07_decodeSeqHeaders` (c_src/src/legacy/zstd_v07.c:3412) | `if (ip >= iend) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2321 | `ZSTDv07_decodeSeqHeaders` (c_src/src/legacy/zstd_v07.c:3420) | `if (ip + 4 > iend) return ERROR(srcSize_wrong); /* min : header byte + all 3 are "raw", hence no header, but at least xxLog bits per type */` | `ERROR(srcSize_wrong)` | [x] |
| 2322 | `ZSTDv07_decodeSeqHeaders` (c_src/src/legacy/zstd_v07.c:3428) | `if (ZSTDv07_isError(llhSize)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2323 | `ZSTDv07_decodeSeqHeaders` (c_src/src/legacy/zstd_v07.c:3432) | `if (ZSTDv07_isError(ofhSize)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2324 | `ZSTDv07_decodeSeqHeaders` (c_src/src/legacy/zstd_v07.c:3436) | `if (ZSTDv07_isError(mlhSize)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2325 | `ZSTDv07_execSequence` (c_src/src/legacy/zstd_v07.c:3547) | `assert(oend >= op);` | process assertion failure | [x] |
| 2326 | `ZSTDv07_execSequence` (c_src/src/legacy/zstd_v07.c:3548) | `if (sequence.litLength + WILDCOPY_OVERLENGTH > (size_t)(oend - op)) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 2327 | `ZSTDv07_execSequence` (c_src/src/legacy/zstd_v07.c:3549) | `if (sequenceLength > (size_t)(oend - op)) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 2328 | `ZSTDv07_execSequence` (c_src/src/legacy/zstd_v07.c:3550) | `assert(litLimit >= *litPtr);` | process assertion failure | [x] |
| 2329 | `ZSTDv07_execSequence` (c_src/src/legacy/zstd_v07.c:3551) | `if (sequence.litLength > (size_t)(litLimit - *litPtr)) return ERROR(corruption_detected);;` | `ERROR(corruption_detected)` | [x] |
| 2330 | `ZSTDv07_execSequence` (c_src/src/legacy/zstd_v07.c:3561) | `if (sequence.offset > (size_t)(oLitEnd - vBase)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2331 | `ZSTDv07_decompressSequences` (c_src/src/legacy/zstd_v07.c:3644) | `if (ERR_isError(errorCode)) return ERROR(corruption_detected); }` | `ERROR(corruption_detected)` | [x] |
| 2332 | `ZSTDv07_decompressSequences` (c_src/src/legacy/zstd_v07.c:3658) | `if (nbSeq) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2333 | `ZSTDv07_decompressSequences` (c_src/src/legacy/zstd_v07.c:3665) | `/* if (litPtr > litEnd) return ERROR(corruption_detected); */ /* too many literals already used */` | `ERROR(corruption_detected)` | [x] |
| 2334 | `ZSTDv07_decompressSequences` (c_src/src/legacy/zstd_v07.c:3666) | `if (lastLLSize > (size_t)(oend-op)) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 2335 | `ZSTDv07_decompressBlock_internal` (c_src/src/legacy/zstd_v07.c:3694) | `if (srcSize >= ZSTDv07_BLOCKSIZE_ABSOLUTEMAX) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2336 | `ZSTDv07_generateNxBytes` (c_src/src/legacy/zstd_v07.c:3730) | `if (length > dstCapacity) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [x] |
| 2337 | `ZSTDv07_decompressFrame` (c_src/src/legacy/zstd_v07.c:3752) | `if (srcSize < ZSTDv07_frameHeaderSize_min+ZSTDv07_blockHeaderSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2338 | `ZSTDv07_decompressFrame` (c_src/src/legacy/zstd_v07.c:3757) | `if (srcSize < frameHeaderSize+ZSTDv07_blockHeaderSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2339 | `ZSTDv07_decompressFrame` (c_src/src/legacy/zstd_v07.c:3758) | `if (ZSTDv07_decodeFrameHeader(dctx, src, frameHeaderSize)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [x] |
| 2340 | `ZSTDv07_decompressFrame` (c_src/src/legacy/zstd_v07.c:3771) | `if (cBlockSize > remainingSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2341 | `ZSTDv07_decompressFrame` (c_src/src/legacy/zstd_v07.c:3786) | `if (remainingSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2342 | `ZSTDv07_decompressFrame` (c_src/src/legacy/zstd_v07.c:3790) | `return ERROR(GENERIC); /* impossible */` | `ERROR(GENERIC)` | [x] |
| 2343 | `ZSTDv07_decompressDCtx` (c_src/src/legacy/zstd_v07.c:3833) | `return ZSTDv07_decompress_usingDict(dctx, dst, dstCapacity, src, srcSize, NULL, 0);` | source-declared rejection sentinel | [x] |
| 2344 | `ZSTDv07_decompress` (c_src/src/legacy/zstd_v07.c:3842) | `if (dctx==NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [x] |
| 2345 | `ZSTDv07_decompressContinue` (c_src/src/legacy/zstd_v07.c:3936) | `if (srcSize != dctx->expected) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [x] |
| 2346 | `ZSTDv07_decompressContinue` (c_src/src/legacy/zstd_v07.c:3942) | `if (srcSize != ZSTDv07_frameHeaderSize_min) return ERROR(srcSize_wrong); /* impossible */` | `ERROR(srcSize_wrong)` | [x] |
| 2347 | `ZSTDv07_decompressContinue` (c_src/src/legacy/zstd_v07.c:3978) | `if (check32 != h32) return ERROR(checksum_wrong);` | `ERROR(checksum_wrong)` | [x] |
| 2348 | `ZSTDv07_decompressContinue` (c_src/src/legacy/zstd_v07.c:4000) | `return ERROR(GENERIC); /* not yet handled */` | `ERROR(GENERIC)` | [x] |
| 2349 | `ZSTDv07_decompressContinue` (c_src/src/legacy/zstd_v07.c:4006) | `return ERROR(GENERIC); /* impossible */` | `ERROR(GENERIC)` | [x] |
| 2350 | `ZSTDv07_decompressContinue` (c_src/src/legacy/zstd_v07.c:4027) | `return ERROR(GENERIC); /* impossible */` | `ERROR(GENERIC)` | [x] |
| 2351 | `ZSTDv07_loadEntropy` (c_src/src/legacy/zstd_v07.c:4047) | `if (HUFv07_isError(hSize)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [x] |
| 2352 | `ZSTDv07_loadEntropy` (c_src/src/legacy/zstd_v07.c:4054) | `if (FSEv07_isError(offcodeHeaderSize)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [x] |
| 2353 | `ZSTDv07_loadEntropy` (c_src/src/legacy/zstd_v07.c:4055) | `if (offcodeLog > OffFSELog) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [x] |
| 2354 | `ZSTDv07_loadEntropy` (c_src/src/legacy/zstd_v07.c:4057) | `if (FSEv07_isError(errorCode)) return ERROR(dictionary_corrupted); }` | `ERROR(dictionary_corrupted)` | [x] |
| 2355 | `ZSTDv07_loadEntropy` (c_src/src/legacy/zstd_v07.c:4064) | `if (FSEv07_isError(matchlengthHeaderSize)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [x] |
| 2356 | `ZSTDv07_loadEntropy` (c_src/src/legacy/zstd_v07.c:4065) | `if (matchlengthLog > MLFSELog) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [x] |
| 2357 | `ZSTDv07_loadEntropy` (c_src/src/legacy/zstd_v07.c:4067) | `if (FSEv07_isError(errorCode)) return ERROR(dictionary_corrupted); }` | `ERROR(dictionary_corrupted)` | [x] |
| 2358 | `ZSTDv07_loadEntropy` (c_src/src/legacy/zstd_v07.c:4074) | `if (FSEv07_isError(litlengthHeaderSize)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [x] |
| 2359 | `ZSTDv07_loadEntropy` (c_src/src/legacy/zstd_v07.c:4075) | `if (litlengthLog > LLFSELog) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [x] |
| 2360 | `ZSTDv07_loadEntropy` (c_src/src/legacy/zstd_v07.c:4077) | `if (FSEv07_isError(errorCode)) return ERROR(dictionary_corrupted); }` | `ERROR(dictionary_corrupted)` | [x] |
| 2361 | `ZSTDv07_loadEntropy` (c_src/src/legacy/zstd_v07.c:4081) | `if (dictPtr+12 > dictEnd) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [x] |
| 2362 | `ZSTDv07_loadEntropy` (c_src/src/legacy/zstd_v07.c:4082) | `dctx->rep[0] = MEM_readLE32(dictPtr+0); if (dctx->rep[0] == 0 \|\| dctx->rep[0] >= dictSize) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [x] |
| 2363 | `ZSTDv07_loadEntropy` (c_src/src/legacy/zstd_v07.c:4083) | `dctx->rep[1] = MEM_readLE32(dictPtr+4); if (dctx->rep[1] == 0 \|\| dctx->rep[1] >= dictSize) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [x] |
| 2364 | `ZSTDv07_loadEntropy` (c_src/src/legacy/zstd_v07.c:4084) | `dctx->rep[2] = MEM_readLE32(dictPtr+8); if (dctx->rep[2] == 0 \|\| dctx->rep[2] >= dictSize) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [x] |
| 2365 | `ZSTDv07_decompress_insertDictionary` (c_src/src/legacy/zstd_v07.c:4104) | `if (ZSTDv07_isError(eSize)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [x] |
| 2366 | `ZSTDv07_decompressBegin_usingDict` (c_src/src/legacy/zstd_v07.c:4121) | `if (ZSTDv07_isError(errorCode)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [x] |
| 2367 | `ZSTDv07_createDDict_advanced` (c_src/src/legacy/zstd_v07.c:4140) | `return NULL;` | `NULL` | [x] |
| 2368 | `ZSTDv07_createDDict_advanced` (c_src/src/legacy/zstd_v07.c:4150) | `return NULL;` | `NULL` | [x] |
| 2369 | `ZSTDv07_createDDict_advanced` (c_src/src/legacy/zstd_v07.c:4159) | `return NULL;` | `NULL` | [x] |
| 2370 | `ZBUFFv07_createDCtx_advanced` (c_src/src/legacy/zstd_v07.c:4293) | `return NULL;` | `NULL` | [x] |
| 2371 | `ZBUFFv07_createDCtx_advanced` (c_src/src/legacy/zstd_v07.c:4296) | `if (zbd==NULL) return NULL;` | `NULL` | [x] |
| 2372 | `ZBUFFv07_createDCtx_advanced` (c_src/src/legacy/zstd_v07.c:4300) | `if (zbd->zd == NULL) { ZBUFFv07_freeDCtx(zbd); return NULL; }` | `NULL` | [x] |
| 2373 | `ZBUFFv07_decompressInit` (c_src/src/legacy/zstd_v07.c:4327) | `return ZBUFFv07_decompressInitDictionary(zbd, NULL, 0);` | source-declared rejection sentinel | [x] |
| 2374 | `ZBUFFv07_decompressContinue` (c_src/src/legacy/zstd_v07.c:4360) | `return ERROR(init_missing);` | `ERROR(init_missing)` | [x] |
| 2375 | `ZBUFFv07_decompressContinue` (c_src/src/legacy/zstd_v07.c:4397) | `if (zbd->inBuff == NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [x] |
| 2376 | `ZBUFFv07_decompressContinue` (c_src/src/legacy/zstd_v07.c:4404) | `if (zbd->outBuff == NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [x] |
| 2377 | `ZBUFFv07_decompressContinue` (c_src/src/legacy/zstd_v07.c:4436) | `if (toLoad > zbd->inBuffSize - zbd->inPos) return ERROR(corruption_detected); /* should never happen */` | `ERROR(corruption_detected)` | [x] |
| 2378 | `ZBUFFv07_decompressContinue` (c_src/src/legacy/zstd_v07.c:4472) | `default: return ERROR(GENERIC); /* impossible */` | `ERROR(GENERIC)` | [x] |

Total mechanically identified rejection sites: 2378.
