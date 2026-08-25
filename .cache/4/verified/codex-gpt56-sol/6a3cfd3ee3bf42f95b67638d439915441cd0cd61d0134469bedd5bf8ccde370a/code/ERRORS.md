# Error Surface

Mechanically extracted from C rejection macros, error/sentinel returns, and assertions.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---:|----------|---------------------------------------------|-------------------|-----|
| 1 | `FSE_isError` | `unsigned FSE_isError(size_t code) { return ERR_isError(code); }` (c_src/src/common/entropy_common.c:31) | exact return/error shown | [ ] |
| 2 | `FSE_getErrorName` | `const char* FSE_getErrorName(size_t code) { return ERR_getErrorName(code); }` (c_src/src/common/entropy_common.c:32) | exact return/error shown | [ ] |
| 3 | `HUF_isError` | `unsigned HUF_isError(size_t code) { return ERR_isError(code); }` (c_src/src/common/entropy_common.c:34) | exact return/error shown | [ ] |
| 4 | `HUF_getErrorName` | `const char* HUF_getErrorName(size_t code) { return ERR_getErrorName(code); }` (c_src/src/common/entropy_common.c:35) | exact return/error shown | [ ] |
| 5 | `FSE_readNCount_body` | `if (countSize > hbSize) return ERROR(corruption_detected);` (c_src/src/common/entropy_common.c:64) | exact return/error shown | [ ] |
| 6 | `FSE_readNCount_body` | `assert(hbSize >= 8);` (c_src/src/common/entropy_common.c:67) | assertion/abort | [ ] |
| 7 | `FSE_readNCount_body` | `if (nbBits > FSE_TABLELOG_ABSOLUTE_MAX) return ERROR(tableLog_tooLarge);` (c_src/src/common/entropy_common.c:73) | exact return/error shown | [ ] |
| 8 | `FSE_readNCount_body` | `assert((bitStream & 3) < 3);` (c_src/src/common/entropy_common.c:106) | assertion/abort | [ ] |
| 9 | `FSE_readNCount_body` | `assert((bitCount >> 3) <= 3); /* For first condition to work */` (c_src/src/common/entropy_common.c:121) | assertion/abort | [ ] |
| 10 | `FSE_readNCount_body` | `assert(count == -1);` (c_src/src/common/entropy_common.c:151) | assertion/abort | [ ] |
| 11 | `FSE_readNCount_body` | `assert(threshold > 1);` (c_src/src/common/entropy_common.c:157) | assertion/abort | [ ] |
| 12 | `FSE_readNCount_body` | `if (remaining != 1) return ERROR(corruption_detected);` (c_src/src/common/entropy_common.c:179) | exact return/error shown | [ ] |
| 13 | `FSE_readNCount_body` | `if (charnum > maxSV1) return ERROR(maxSymbolValue_tooSmall);` (c_src/src/common/entropy_common.c:181) | exact return/error shown | [ ] |
| 14 | `FSE_readNCount_body` | `if (bitCount > 32) return ERROR(corruption_detected);` (c_src/src/common/entropy_common.c:182) | exact return/error shown | [ ] |
| 15 | `HUF_readStats_body` | `if (!srcSize) return ERROR(srcSize_wrong);` (c_src/src/common/entropy_common.c:254) | exact return/error shown | [ ] |
| 16 | `HUF_readStats_body` | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` (c_src/src/common/entropy_common.c:261) | exact return/error shown | [ ] |
| 17 | `HUF_readStats_body` | `if (oSize >= hwSize) return ERROR(corruption_detected);` (c_src/src/common/entropy_common.c:262) | exact return/error shown | [ ] |
| 18 | `HUF_readStats_body` | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` (c_src/src/common/entropy_common.c:270) | exact return/error shown | [ ] |
| 19 | `HUF_readStats_body` | `if (huffWeight[n] > HUF_TABLELOG_MAX) return ERROR(corruption_detected);` (c_src/src/common/entropy_common.c:280) | exact return/error shown | [ ] |
| 20 | `HUF_readStats_body` | `if (weightTotal == 0) return ERROR(corruption_detected);` (c_src/src/common/entropy_common.c:284) | exact return/error shown | [ ] |
| 21 | `HUF_readStats_body` | `if (tableLog > HUF_TABLELOG_MAX) return ERROR(corruption_detected);` (c_src/src/common/entropy_common.c:288) | exact return/error shown | [ ] |
| 22 | `HUF_readStats_body` | `if (verif != rest) return ERROR(corruption_detected); /* last value must be a clean power of 2 */` (c_src/src/common/entropy_common.c:295) | exact return/error shown | [ ] |
| 23 | `HUF_readStats_body` | `if ((rankStats[1] < 2) \|\| (rankStats[1] & 1)) return ERROR(corruption_detected); /* by construction : at least 2 elts of rank 1, must be even */` (c_src/src/common/entropy_common.c:301) | exact return/error shown | [ ] |
| 24 | `ERR_getErrorString` | `default: return notErrorCode;` (c_src/src/common/error_private.c:61) | exact return/error shown | [ ] |
| 25 | `FSE_buildDTable_internal` | `if (FSE_BUILD_DTABLE_WKSP_SIZE(tableLog, maxSymbolValue) > wkspSize) return ERROR(maxSymbolValue_tooLarge);` (c_src/src/common/fse_decompress.c:70) | exact return/error shown | [ ] |
| 26 | `FSE_buildDTable_internal` | `if (maxSymbolValue > FSE_MAX_SYMBOL_VALUE) return ERROR(maxSymbolValue_tooLarge);` (c_src/src/common/fse_decompress.c:71) | exact return/error shown | [ ] |
| 27 | `FSE_buildDTable_internal` | `if (tableLog > FSE_MAX_TABLELOG) return ERROR(tableLog_tooLarge);` (c_src/src/common/fse_decompress.c:72) | exact return/error shown | [ ] |
| 28 | `FSE_buildDTable_internal` | `assert(tableSize % unroll == 0); /* FSE_MIN_TABLELOG is 5 */` (c_src/src/common/fse_decompress.c:124) | assertion/abort | [ ] |
| 29 | `FSE_buildDTable_internal` | `assert(position == 0);` (c_src/src/common/fse_decompress.c:133) | assertion/abort | [ ] |
| 30 | `FSE_buildDTable_internal` | `if (position!=0) return ERROR(GENERIC); /* position must reach all cells once, otherwise normalizedCounter is incorrect */` (c_src/src/common/fse_decompress.c:146) | exact return/error shown | [ ] |
| 31 | `FSE_decompress_usingDTable_generic` | `RETURN_ERROR_IF(BIT_reloadDStream(&bitD)==BIT_DStream_overflow, corruption_detected, "");` (c_src/src/common/fse_decompress.c:193) | exact return/error shown | [ ] |
| 32 | `FSE_decompress_usingDTable_generic` | `if (op>(omax-2)) return ERROR(dstSize_tooSmall);` (c_src/src/common/fse_decompress.c:220) | exact return/error shown | [ ] |
| 33 | `FSE_decompress_usingDTable_generic` | `if (op>(omax-2)) return ERROR(dstSize_tooSmall);` (c_src/src/common/fse_decompress.c:227) | exact return/error shown | [ ] |
| 34 | `FSE_decompress_usingDTable_generic` | `assert(op >= ostart);` (c_src/src/common/fse_decompress.c:234) | assertion/abort | [ ] |
| 35 | `FSE_decompress_wksp_body` | `if (wkspSize < sizeof(*wksp)) return ERROR(GENERIC);` (c_src/src/common/fse_decompress.c:258) | exact return/error shown | [ ] |
| 36 | `FSE_decompress_wksp_body` | `if (tableLog > maxLog) return ERROR(tableLog_tooLarge);` (c_src/src/common/fse_decompress.c:267) | exact return/error shown | [ ] |
| 37 | `FSE_decompress_wksp_body` | `assert(NCountLength <= cSrcSize);` (c_src/src/common/fse_decompress.c:268) | assertion/abort | [ ] |
| 38 | `FSE_decompress_wksp_body` | `if (FSE_DECOMPRESS_WKSP_SIZE(tableLog, maxSymbolValue) > wkspSize) return ERROR(tableLog_tooLarge);` (c_src/src/common/fse_decompress.c:273) | exact return/error shown | [ ] |
| 39 | `FSE_decompress_wksp_body` | `assert(sizeof(*wksp) + FSE_DTABLE_SIZE(tableLog) <= wkspSize);` (c_src/src/common/fse_decompress.c:274) | assertion/abort | [ ] |
| 40 | `POOL_thread` | `if (!ctx) { return NULL; }` (c_src/src/common/pool.c:69) | exact return/error shown | [ ] |
| 41 | `POOL_thread` | `assert(0); /* Unreachable */` (c_src/src/common/pool.c:103) | assertion/abort | [ ] |
| 42 | `POOL_create_advanced` | `if (!numThreads) { return NULL; }` (c_src/src/common/pool.c:120) | exact return/error shown | [ ] |
| 43 | `POOL_create_advanced` | `if (!ctx) { return NULL; }` (c_src/src/common/pool.c:123) | exact return/error shown | [ ] |
| 44 | `POOL_create_advanced` | `if (error) { POOL_free(ctx); return NULL; }` (c_src/src/common/pool.c:139) | exact return/error shown | [ ] |
| 45 | `POOL_create_advanced` | `if (!ctx->threads \|\| !ctx->queue) { POOL_free(ctx); return NULL; }` (c_src/src/common/pool.c:147) | exact return/error shown | [ ] |
| 46 | `POOL_create_advanced` | `return NULL;` (c_src/src/common/pool.c:154) | exact return/error shown | [ ] |
| 47 | `POOL_add_internal` | `assert(ctx != NULL);` (c_src/src/common/pool.c:277) | assertion/abort | [ ] |
| 48 | `POOL_add` | `assert(ctx != NULL);` (c_src/src/common/pool.c:288) | assertion/abort | [ ] |
| 49 | `POOL_tryAdd` | `assert(ctx != NULL);` (c_src/src/common/pool.c:301) | assertion/abort | [ ] |
| 50 | `<file scope/macro>` | `/* We don't need any data, but if it is empty, malloc() might return NULL. */ struct POOL_ctx_s { int dummy; };` (c_src/src/common/pool.c:320) | exact return/error shown | [ ] |
| 51 | `POOL_free` | `assert(!ctx \|\| ctx == &g_poolCtx);` (c_src/src/common/pool.c:340) | assertion/abort | [ ] |
| 52 | `POOL_joinJobs` | `assert(!ctx \|\| ctx == &g_poolCtx);` (c_src/src/common/pool.c:345) | assertion/abort | [ ] |
| 53 | `POOL_sizeof` | `assert(ctx == &g_poolCtx);` (c_src/src/common/pool.c:367) | assertion/abort | [ ] |
| 54 | `ZSTD_pthread_create` | `if (thread==NULL) return -1;` (c_src/src/common/threading.c:76) | exact return/error shown | [ ] |
| 55 | `ZSTD_pthread_create` | `return -1;` (c_src/src/common/threading.c:86) | exact return/error shown | [ ] |
| 56 | `ZSTD_pthread_create` | `return -1;` (c_src/src/common/threading.c:91) | exact return/error shown | [ ] |
| 57 | `ZSTD_pthread_join` | `return GetLastError();` (c_src/src/common/threading.c:129) | exact return/error shown | [ ] |
| 58 | `ZSTD_pthread_mutex_init` | `assert(mutex != NULL);` (c_src/src/common/threading.c:142) | assertion/abort | [ ] |
| 59 | `ZSTD_pthread_mutex_destroy` | `assert(mutex != NULL);` (c_src/src/common/threading.c:151) | assertion/abort | [ ] |
| 60 | `ZSTD_pthread_cond_init` | `assert(cond != NULL);` (c_src/src/common/threading.c:163) | assertion/abort | [ ] |
| 61 | `ZSTD_pthread_cond_destroy` | `assert(cond != NULL);` (c_src/src/common/threading.c:172) | assertion/abort | [ ] |
| 62 | `ZSTD_isError` | `unsigned ZSTD_isError(size_t code) { return ERR_isError(code); }` (c_src/src/common/zstd_common.c:36) | exact return/error shown | [ ] |
| 63 | `ZSTD_getErrorName` | `const char* ZSTD_getErrorName(size_t code) { return ERR_getErrorName(code); }` (c_src/src/common/zstd_common.c:40) | exact return/error shown | [ ] |
| 64 | `ZSTD_getErrorCode` | `ZSTD_ErrorCode ZSTD_getErrorCode(size_t code) { return ERR_getErrorCode(code); }` (c_src/src/common/zstd_common.c:44) | exact return/error shown | [ ] |
| 65 | `ZSTD_getErrorString` | `const char* ZSTD_getErrorString(ZSTD_ErrorCode code) { return ERR_getErrorString(code); }` (c_src/src/common/zstd_common.c:48) | exact return/error shown | [ ] |
| 66 | `FSE_buildCTable_wksp` | `assert(((size_t)workSpace & 1) == 0); /* Must be 2 bytes-aligned */` (c_src/src/compress/fse_compress.c:86) | assertion/abort | [ ] |
| 67 | `FSE_buildCTable_wksp` | `if (FSE_BUILD_CTABLE_WORKSPACE_SIZE(maxSymbolValue, tableLog) > wkspSize) return ERROR(tableLog_tooLarge);` (c_src/src/compress/fse_compress.c:87) | exact return/error shown | [ ] |
| 68 | `FSE_buildCTable_wksp` | `assert(tableLog < 16); /* required for threshold strategy to work */` (c_src/src/compress/fse_compress.c:91) | assertion/abort | [ ] |
| 69 | `FSE_buildCTable_wksp` | `assert(normalizedCounter[u-1] >= 0);` (c_src/src/compress/fse_compress.c:108) | assertion/abort | [ ] |
| 70 | `FSE_buildCTable_wksp` | `assert(cumul[u] >= cumul[u-1]); /* no overflow */` (c_src/src/compress/fse_compress.c:110) | assertion/abort | [ ] |
| 71 | `FSE_buildCTable_wksp` | `assert(n>=0);` (c_src/src/compress/fse_compress.c:132) | assertion/abort | [ ] |
| 72 | `FSE_buildCTable_wksp` | `assert(tableSize % unroll == 0); /* FSE_MIN_TABLELOG is 5 */` (c_src/src/compress/fse_compress.c:143) | assertion/abort | [ ] |
| 73 | `FSE_buildCTable_wksp` | `assert(position == 0); /* Must have initialized all positions */` (c_src/src/compress/fse_compress.c:152) | assertion/abort | [ ] |
| 74 | `FSE_buildCTable_wksp` | `assert(position==0); /* Must have initialized all positions */` (c_src/src/compress/fse_compress.c:166) | assertion/abort | [ ] |
| 75 | `FSE_buildCTable_wksp` | `assert(total <= INT_MAX);` (c_src/src/compress/fse_compress.c:189) | assertion/abort | [ ] |
| 76 | `FSE_buildCTable_wksp` | `assert(normalizedCounter[s] > 1);` (c_src/src/compress/fse_compress.c:194) | assertion/abort | [ ] |
| 77 | `FSE_writeNCount_generic` | `return ERROR(dstSize_tooSmall); /* Buffer overflow */` (c_src/src/compress/fse_compress.c:269) | exact return/error shown | [ ] |
| 78 | `FSE_writeNCount_generic` | `return ERROR(dstSize_tooSmall); /* Buffer overflow */` (c_src/src/compress/fse_compress.c:284) | exact return/error shown | [ ] |
| 79 | `FSE_writeNCount_generic` | `if (remaining<1) return ERROR(GENERIC);` (c_src/src/compress/fse_compress.c:301) | exact return/error shown | [ ] |
| 80 | `FSE_writeNCount_generic` | `return ERROR(dstSize_tooSmall); /* Buffer overflow */` (c_src/src/compress/fse_compress.c:306) | exact return/error shown | [ ] |
| 81 | `FSE_writeNCount_generic` | `return ERROR(GENERIC); /* incorrect normalized distribution */` (c_src/src/compress/fse_compress.c:315) | exact return/error shown | [ ] |
| 82 | `FSE_writeNCount_generic` | `assert(symbol <= alphabetSize);` (c_src/src/compress/fse_compress.c:316) | assertion/abort | [ ] |
| 83 | `FSE_writeNCount_generic` | `return ERROR(dstSize_tooSmall); /* Buffer overflow */` (c_src/src/compress/fse_compress.c:320) | exact return/error shown | [ ] |
| 84 | `FSE_writeNCount_generic` | `assert(out >= ostart);` (c_src/src/compress/fse_compress.c:325) | assertion/abort | [ ] |
| 85 | `FSE_writeNCount` | `if (tableLog > FSE_MAX_TABLELOG) return ERROR(tableLog_tooLarge); /* Unsupported */` (c_src/src/compress/fse_compress.c:333) | exact return/error shown | [ ] |
| 86 | `FSE_writeNCount` | `if (tableLog < FSE_MIN_TABLELOG) return ERROR(GENERIC); /* Unsupported */` (c_src/src/compress/fse_compress.c:334) | exact return/error shown | [ ] |
| 87 | `FSE_minTableLog` | `assert(srcSize > 1); /* Not supported, RLE should be used instead */` (c_src/src/compress/fse_compress.c:353) | assertion/abort | [ ] |
| 88 | `FSE_optimalTableLog_internal` | `assert(srcSize > 1); /* Not supported, RLE should be used instead */` (c_src/src/compress/fse_compress.c:362) | assertion/abort | [ ] |
| 89 | `FSE_normalizeM2` | `return ERROR(GENERIC);` (c_src/src/compress/fse_compress.c:457) | exact return/error shown | [ ] |
| 90 | `FSE_normalizeCount` | `if (tableLog < FSE_MIN_TABLELOG) return ERROR(GENERIC); /* Unsupported size */` (c_src/src/compress/fse_compress.c:471) | exact return/error shown | [ ] |
| 91 | `FSE_normalizeCount` | `if (tableLog > FSE_MAX_TABLELOG) return ERROR(tableLog_tooLarge); /* Unsupported size */` (c_src/src/compress/fse_compress.c:472) | exact return/error shown | [ ] |
| 92 | `FSE_normalizeCount` | `if (tableLog < FSE_minTableLog(total, maxSymbolValue)) return ERROR(GENERIC); /* Too small tableLog, compression potentially impossible */` (c_src/src/compress/fse_compress.c:473) | exact return/error shown | [ ] |
| 93 | `FSE_normalizeCount` | `if (FSE_isError(errorCode)) return errorCode;` (c_src/src/compress/fse_compress.c:505) | exact return/error shown | [ ] |
| 94 | `HIST_isError` | `unsigned HIST_isError(size_t code) { return ERR_isError(code); }` (c_src/src/compress/hist.c:24) | exact return/error shown | [ ] |
| 95 | `HIST_count_simple` | `assert(*ip <= maxSymbolValue);` (c_src/src/compress/hist.c:51) | assertion/abort | [ ] |
| 96 | `HIST_count_parallel_wksp` | `assert(*maxSymbolValuePtr <= 255);` (c_src/src/compress/hist.c:92) | assertion/abort | [ ] |
| 97 | `HIST_count_parallel_wksp` | `if (check && maxSymbolValue > *maxSymbolValuePtr) return ERROR(maxSymbolValue_tooSmall);` (c_src/src/compress/hist.c:138) | exact return/error shown | [ ] |
| 98 | `HIST_countFast_wksp` | `if ((size_t)workSpace & 3) return ERROR(GENERIC); /* must be aligned on 4-bytes boundaries */` (c_src/src/compress/hist.c:156) | exact return/error shown | [ ] |
| 99 | `HIST_countFast_wksp` | `if (workSpaceSize < HIST_WKSP_SIZE) return ERROR(workSpace_tooSmall);` (c_src/src/compress/hist.c:157) | exact return/error shown | [ ] |
| 100 | `HIST_count_wksp` | `if ((size_t)workSpace & 3) return ERROR(GENERIC); /* must be aligned on 4-bytes boundaries */` (c_src/src/compress/hist.c:168) | exact return/error shown | [ ] |
| 101 | `HIST_count_wksp` | `if (workSpaceSize < HIST_WKSP_SIZE) return ERROR(workSpace_tooSmall);` (c_src/src/compress/hist.c:169) | exact return/error shown | [ ] |
| 102 | `HUF_alignUpWorkspace` | `assert((align & (align - 1)) == 0); /* pow 2 */` (c_src/src/compress/huf_compress.c:118) | assertion/abort | [ ] |
| 103 | `HUF_alignUpWorkspace` | `assert(align <= HUF_WORKSPACE_MAX_ALIGNMENT);` (c_src/src/compress/huf_compress.c:119) | assertion/abort | [ ] |
| 104 | `HUF_alignUpWorkspace` | `assert(add < align);` (c_src/src/compress/huf_compress.c:121) | assertion/abort | [ ] |
| 105 | `HUF_alignUpWorkspace` | `assert(((size_t)aligned & mask) == 0);` (c_src/src/compress/huf_compress.c:122) | assertion/abort | [ ] |
| 106 | `HUF_alignUpWorkspace` | `return NULL;` (c_src/src/compress/huf_compress.c:127) | exact return/error shown | [ ] |
| 107 | `HUF_compressWeights` | `if (workspaceSize < sizeof(HUF_CompressWeightsWksp)) return ERROR(GENERIC);` (c_src/src/compress/huf_compress.c:159) | exact return/error shown | [ ] |
| 108 | `HUF_setNbBits` | `assert(nbBits <= HUF_TABLELOG_ABSOLUTEMAX);` (c_src/src/compress/huf_compress.c:210) | assertion/abort | [ ] |
| 109 | `HUF_setValue` | `assert((value >> nbBits) == 0);` (c_src/src/compress/huf_compress.c:218) | assertion/abort | [ ] |
| 110 | `HUF_writeCTableHeader` | `assert(tableLog < 256);` (c_src/src/compress/huf_compress.c:235) | assertion/abort | [ ] |
| 111 | `HUF_writeCTableHeader` | `assert(maxSymbolValue < 256);` (c_src/src/compress/huf_compress.c:237) | assertion/abort | [ ] |
| 112 | `HUF_writeCTable_wksp` | `assert(HUF_readCTableHeader(CTable).maxSymbolValue == maxSymbolValue);` (c_src/src/compress/huf_compress.c:259) | assertion/abort | [ ] |
| 113 | `HUF_writeCTable_wksp` | `assert(HUF_readCTableHeader(CTable).tableLog == huffLog);` (c_src/src/compress/huf_compress.c:260) | assertion/abort | [ ] |
| 114 | `HUF_writeCTable_wksp` | `if (workspaceSize < sizeof(HUF_WriteCTableWksp)) return ERROR(GENERIC);` (c_src/src/compress/huf_compress.c:263) | exact return/error shown | [ ] |
| 115 | `HUF_writeCTable_wksp` | `if (maxSymbolValue > HUF_SYMBOLVALUE_MAX) return ERROR(maxSymbolValue_tooLarge);` (c_src/src/compress/huf_compress.c:264) | exact return/error shown | [ ] |
| 116 | `HUF_writeCTable_wksp` | `if (maxDstSize < 1) return ERROR(dstSize_tooSmall);` (c_src/src/compress/huf_compress.c:274) | exact return/error shown | [ ] |
| 117 | `HUF_writeCTable_wksp` | `if (maxSymbolValue > (256-128)) return ERROR(GENERIC); /* should not happen : likely means source cannot be compressed */` (c_src/src/compress/huf_compress.c:282) | exact return/error shown | [ ] |
| 118 | `HUF_writeCTable_wksp` | `if (((maxSymbolValue+1)/2) + 1 > maxDstSize) return ERROR(dstSize_tooSmall); /* not enough space within dst buffer */` (c_src/src/compress/huf_compress.c:283) | exact return/error shown | [ ] |
| 119 | `HUF_readCTable` | `if (tableLog > HUF_TABLELOG_MAX) return ERROR(tableLog_tooLarge);` (c_src/src/compress/huf_compress.c:305) | exact return/error shown | [ ] |
| 120 | `HUF_readCTable` | `if (nbSymbols > *maxSymbolValuePtr+1) return ERROR(maxSymbolValue_tooSmall);` (c_src/src/compress/huf_compress.c:306) | exact return/error shown | [ ] |
| 121 | `HUF_getNbBitsFromCTable` | `assert(symbolValue <= HUF_SYMBOLVALUE_MAX);` (c_src/src/compress/huf_compress.c:348) | assertion/abort | [ ] |
| 122 | `HUF_setMaxHeight` | `assert(huffNode[n].nbBits <= targetNbBits);` (c_src/src/compress/huf_compress.c:399) | assertion/abort | [ ] |
| 123 | `HUF_setMaxHeight` | `assert(((U32)totalCost & (baseCost - 1)) == 0);` (c_src/src/compress/huf_compress.c:405) | assertion/abort | [ ] |
| 124 | `HUF_setMaxHeight` | `assert(totalCost > 0);` (c_src/src/compress/huf_compress.c:407) | assertion/abort | [ ] |
| 125 | `HUF_setMaxHeight` | `assert(rankLast[nBitsToDecrease] != noSymbol \|\| nBitsToDecrease == 1);` (c_src/src/compress/huf_compress.c:441) | assertion/abort | [ ] |
| 126 | `HUF_setMaxHeight` | `assert(rankLast[nBitsToDecrease] != noSymbol);` (c_src/src/compress/huf_compress.c:445) | assertion/abort | [ ] |
| 127 | `HUF_setMaxHeight` | `assert(n >= 0);` (c_src/src/compress/huf_compress.c:485) | assertion/abort | [ ] |
| 128 | `HUF_sort` | `assert(lowerRank < RANK_POSITION_TABLE_SIZE - 1);` (c_src/src/compress/huf_compress.c:633) | assertion/abort | [ ] |
| 129 | `HUF_sort` | `assert(rankPosition[RANK_POSITION_TABLE_SIZE - 1].base == 0);` (c_src/src/compress/huf_compress.c:637) | assertion/abort | [ ] |
| 130 | `HUF_sort` | `assert(pos < maxSymbolValue1);` (c_src/src/compress/huf_compress.c:649) | assertion/abort | [ ] |
| 131 | `HUF_sort` | `assert(bucketStartIdx < maxSymbolValue1);` (c_src/src/compress/huf_compress.c:659) | assertion/abort | [ ] |
| 132 | `HUF_sort` | `assert(HUF_isSorted(huffNode, maxSymbolValue1));` (c_src/src/compress/huf_compress.c:664) | assertion/abort | [ ] |
| 133 | `HUF_buildCTable_wksp` | `return ERROR(workSpace_tooSmall);` (c_src/src/compress/huf_compress.c:771) | exact return/error shown | [ ] |
| 134 | `HUF_buildCTable_wksp` | `return ERROR(maxSymbolValue_tooLarge);` (c_src/src/compress/huf_compress.c:774) | exact return/error shown | [ ] |
| 135 | `HUF_buildCTable_wksp` | `if (maxNbBits > HUF_TABLELOG_MAX) return ERROR(GENERIC); /* check fit into table */` (c_src/src/compress/huf_compress.c:786) | exact return/error shown | [ ] |
| 136 | `HUF_validateCTable` | `assert(header.tableLog <= HUF_TABLELOG_ABSOLUTEMAX);` (c_src/src/compress/huf_compress.c:810) | assertion/abort | [ ] |
| 137 | `HUF_initCStream` | `if (dstCapacity <= sizeof(bitC->bitContainer[0])) return ERROR(dstSize_tooSmall);` (c_src/src/compress/huf_compress.c:863) | exact return/error shown | [ ] |
| 138 | `HUF_addBits` | `assert(idx <= 1);` (c_src/src/compress/huf_compress.c:879) | assertion/abort | [ ] |
| 139 | `HUF_addBits` | `assert(HUF_getNbBits(elt) <= HUF_TABLELOG_ABSOLUTEMAX);` (c_src/src/compress/huf_compress.c:880) | assertion/abort | [ ] |
| 140 | `HUF_addBits` | `assert((bitC->bitPos[idx] & 0xFF) <= HUF_BITS_IN_CONTAINER);` (c_src/src/compress/huf_compress.c:892) | assertion/abort | [ ] |
| 141 | `HUF_addBits` | `assert(((elt >> dirtyBits) << (dirtyBits + nbBits)) == 0);` (c_src/src/compress/huf_compress.c:903) | assertion/abort | [ ] |
| 142 | `HUF_addBits` | `assert(!kFast \|\| (bitC->bitPos[idx] & 0xFF) <= HUF_BITS_IN_CONTAINER);` (c_src/src/compress/huf_compress.c:905) | assertion/abort | [ ] |
| 143 | `HUF_mergeIndex1` | `assert((bitC->bitPos[1] & 0xFF) < HUF_BITS_IN_CONTAINER);` (c_src/src/compress/huf_compress.c:923) | assertion/abort | [ ] |
| 144 | `HUF_mergeIndex1` | `assert((bitC->bitPos[0] & 0xFF) <= HUF_BITS_IN_CONTAINER);` (c_src/src/compress/huf_compress.c:927) | assertion/abort | [ ] |
| 145 | `HUF_flushBits` | `assert(nbBits > 0);` (c_src/src/compress/huf_compress.c:946) | assertion/abort | [ ] |
| 146 | `HUF_flushBits` | `assert(nbBits <= sizeof(bitC->bitContainer[0]) * 8);` (c_src/src/compress/huf_compress.c:947) | assertion/abort | [ ] |
| 147 | `HUF_flushBits` | `assert(bitC->ptr <= bitC->endPtr);` (c_src/src/compress/huf_compress.c:948) | assertion/abort | [ ] |
| 148 | `HUF_flushBits` | `assert(!kFast \|\| bitC->ptr <= bitC->endPtr);` (c_src/src/compress/huf_compress.c:951) | assertion/abort | [ ] |
| 149 | `HUF_compress1X_usingCTable_internal_body_loop` | `assert(n % kUnroll == 0);` (c_src/src/compress/huf_compress.c:1005) | assertion/abort | [ ] |
| 150 | `HUF_compress1X_usingCTable_internal_body_loop` | `assert(n % (2 * kUnroll) == 0);` (c_src/src/compress/huf_compress.c:1017) | assertion/abort | [ ] |
| 151 | `HUF_compress1X_usingCTable_internal_body_loop` | `assert(n == 0);` (c_src/src/compress/huf_compress.c:1040) | assertion/abort | [ ] |
| 152 | `HUF_compress1X_usingCTable_internal_body` | `assert(bitC.ptr <= bitC.endPtr);` (c_src/src/compress/huf_compress.c:1115) | assertion/abort | [ ] |
| 153 | `HUF_compress4X_usingCTable_internal` | `assert(op <= oend);` (c_src/src/compress/huf_compress.c:1183) | assertion/abort | [ ] |
| 154 | `HUF_compress4X_usingCTable_internal` | `assert(op <= oend);` (c_src/src/compress/huf_compress.c:1191) | assertion/abort | [ ] |
| 155 | `HUF_compress4X_usingCTable_internal` | `assert(op <= oend);` (c_src/src/compress/huf_compress.c:1199) | assertion/abort | [ ] |
| 156 | `HUF_compress4X_usingCTable_internal` | `assert(op <= oend);` (c_src/src/compress/huf_compress.c:1207) | assertion/abort | [ ] |
| 157 | `HUF_compress4X_usingCTable_internal` | `assert(ip <= iend);` (c_src/src/compress/huf_compress.c:1208) | assertion/abort | [ ] |
| 158 | `HUF_compressCTable_internal` | `assert(op >= ostart);` (c_src/src/compress/huf_compress.c:1236) | assertion/abort | [ ] |
| 159 | `HUF_optimalTableLog` | `assert(srcSize > 1); /* Not supported, RLE should be used instead */` (c_src/src/compress/huf_compress.c:1281) | assertion/abort | [ ] |
| 160 | `HUF_optimalTableLog` | `assert(wkspSize >= sizeof(HUF_buildCTable_wksp_tables));` (c_src/src/compress/huf_compress.c:1282) | assertion/abort | [ ] |
| 161 | `HUF_optimalTableLog` | `assert(optLog <= HUF_TABLELOG_MAX);` (c_src/src/compress/huf_compress.c:1324) | assertion/abort | [ ] |
| 162 | `HUF_compress_internal` | `if (wkspSize < sizeof(*table)) return ERROR(workSpace_tooSmall);` (c_src/src/compress/huf_compress.c:1349) | exact return/error shown | [ ] |
| 163 | `HUF_compress_internal` | `if (srcSize > HUF_BLOCKSIZE_MAX) return ERROR(srcSize_wrong); /* current block size limit */` (c_src/src/compress/huf_compress.c:1352) | exact return/error shown | [ ] |
| 164 | `HUF_compress_internal` | `if (huffLog > HUF_TABLELOG_MAX) return ERROR(tableLog_tooLarge);` (c_src/src/compress/huf_compress.c:1353) | exact return/error shown | [ ] |
| 165 | `HUF_compress_internal` | `if (maxSymbolValue > HUF_SYMBOLVALUE_MAX) return ERROR(maxSymbolValue_tooLarge);` (c_src/src/compress/huf_compress.c:1354) | exact return/error shown | [ ] |
| 166 | `ZSTD_compressBound` | `if (r==0) return ERROR(srcSize_wrong);` (c_src/src/compress/zstd_compress.c:72) | exact return/error shown | [ ] |
| 167 | `ZSTD_initCCtx` | `assert(cctx != NULL);` (c_src/src/compress/zstd_compress.c:104) | assertion/abort | [ ] |
| 168 | `ZSTD_initCCtx` | `assert(!ZSTD_isError(err));` (c_src/src/compress/zstd_compress.c:109) | assertion/abort | [ ] |
| 169 | `ZSTD_createCCtx_advanced` | `ZSTD_STATIC_ASSERT(zcss_init==0);` (c_src/src/compress/zstd_compress.c:116) | exact return/error shown | [ ] |
| 170 | `ZSTD_createCCtx_advanced` | `ZSTD_STATIC_ASSERT(ZSTD_CONTENTSIZE_UNKNOWN==(0ULL - 1));` (c_src/src/compress/zstd_compress.c:117) | exact return/error shown | [ ] |
| 171 | `ZSTD_createCCtx_advanced` | `if ((!customMem.customAlloc) ^ (!customMem.customFree)) return NULL;` (c_src/src/compress/zstd_compress.c:118) | exact return/error shown | [ ] |
| 172 | `ZSTD_createCCtx_advanced` | `if (!cctx) return NULL;` (c_src/src/compress/zstd_compress.c:120) | exact return/error shown | [ ] |
| 173 | `ZSTD_initStaticCCtx` | `if (workspaceSize <= sizeof(ZSTD_CCtx)) return NULL; /* minimum size */` (c_src/src/compress/zstd_compress.c:130) | exact return/error shown | [ ] |
| 174 | `ZSTD_initStaticCCtx` | `if ((size_t)workspace & 7) return NULL; /* must be 8-aligned */` (c_src/src/compress/zstd_compress.c:131) | exact return/error shown | [ ] |
| 175 | `ZSTD_initStaticCCtx` | `if (cctx == NULL) return NULL;` (c_src/src/compress/zstd_compress.c:135) | exact return/error shown | [ ] |
| 176 | `ZSTD_initStaticCCtx` | `if (!ZSTD_cwksp_check_available(&cctx->workspace, TMP_WORKSPACE_SIZE + 2 * sizeof(ZSTD_compressedBlockState_t))) return NULL;` (c_src/src/compress/zstd_compress.c:142) | exact return/error shown | [ ] |
| 177 | `ZSTD_freeCCtxContent` | `assert(cctx != NULL);` (c_src/src/compress/zstd_compress.c:172) | assertion/abort | [ ] |
| 178 | `ZSTD_freeCCtxContent` | `assert(cctx->staticSize == 0);` (c_src/src/compress/zstd_compress.c:173) | assertion/abort | [ ] |
| 179 | `ZSTD_freeCCtx` | `RETURN_ERROR_IF(cctx->staticSize, memory_allocation, "not compatible with static CCtx");` (c_src/src/compress/zstd_compress.c:185) | exact return/error shown | [ ] |
| 180 | `ZSTD_rowMatchFinderUsed` | `assert(mode != ZSTD_ps_auto);` (c_src/src/compress/zstd_compress.c:233) | assertion/abort | [ ] |
| 181 | `ZSTD_allocateChainTable` | `assert(useRowMatchFinder != ZSTD_ps_auto);` (c_src/src/compress/zstd_compress.c:258) | assertion/abort | [ ] |
| 182 | `ZSTD_makeCCtxParamsFromCParams` | `assert(cctxParams.ldmParams.hashLog >= cctxParams.ldmParams.bucketSizeLog);` (c_src/src/compress/zstd_compress.c:315) | assertion/abort | [ ] |
| 183 | `ZSTD_makeCCtxParamsFromCParams` | `assert(cctxParams.ldmParams.hashRateLog < 32);` (c_src/src/compress/zstd_compress.c:316) | assertion/abort | [ ] |
| 184 | `ZSTD_makeCCtxParamsFromCParams` | `assert(!ZSTD_checkCParams(cParams));` (c_src/src/compress/zstd_compress.c:324) | assertion/abort | [ ] |
| 185 | `ZSTD_createCCtxParams_advanced` | `if ((!customMem.customAlloc) ^ (!customMem.customFree)) return NULL;` (c_src/src/compress/zstd_compress.c:332) | exact return/error shown | [ ] |
| 186 | `ZSTD_createCCtxParams_advanced` | `if (!params) { return NULL; }` (c_src/src/compress/zstd_compress.c:335) | exact return/error shown | [ ] |
| 187 | `ZSTD_CCtxParams_init` | `RETURN_ERROR_IF(!cctxParams, GENERIC, "NULL pointer!");` (c_src/src/compress/zstd_compress.c:359) | exact return/error shown | [ ] |
| 188 | `ZSTD_CCtxParams_init_internal` | `assert(!ZSTD_checkCParams(params->cParams));` (c_src/src/compress/zstd_compress.c:377) | assertion/abort | [ ] |
| 189 | `ZSTD_CCtxParams_init_advanced` | `RETURN_ERROR_IF(!cctxParams, GENERIC, "NULL pointer!");` (c_src/src/compress/zstd_compress.c:397) | exact return/error shown | [ ] |
| 190 | `ZSTD_CCtxParams_init_advanced` | `FORWARD_IF_ERROR( ZSTD_checkCParams(params.cParams) , "");` (c_src/src/compress/zstd_compress.c:398) | exact return/error shown | [ ] |
| 191 | `ZSTD_CCtxParams_setZstdParams` | `assert(!ZSTD_checkCParams(params->cParams));` (c_src/src/compress/zstd_compress.c:410) | assertion/abort | [ ] |
| 192 | `ZSTD_cParam_getBounds` | `ZSTD_STATIC_ASSERT(ZSTD_f_zstd1 < ZSTD_f_zstd1_magicless);` (c_src/src/compress/zstd_compress.c:550) | exact return/error shown | [ ] |
| 193 | `ZSTD_cParam_getBounds` | `ZSTD_STATIC_ASSERT(ZSTD_dictDefaultAttach < ZSTD_dictForceLoad);` (c_src/src/compress/zstd_compress.c:556) | exact return/error shown | [ ] |
| 194 | `ZSTD_cParam_getBounds` | `ZSTD_STATIC_ASSERT(ZSTD_ps_auto < ZSTD_ps_enable && ZSTD_ps_enable < ZSTD_ps_disable);` (c_src/src/compress/zstd_compress.c:562) | exact return/error shown | [ ] |
| 195 | `<file scope/macro>` | `RETURN_ERROR_IF(!ZSTD_cParam_withinBounds(cParam,val), \ parameter_outOfBound, "Param out of bounds"); \` (c_src/src/compress/zstd_compress.c:653) | exact return/error shown | [ ] |
| 196 | `ZSTD_CCtx_setParameter` | `RETURN_ERROR(stage_wrong, "can only set params in cctx init stage");` (c_src/src/compress/zstd_compress.c:715) | exact return/error shown | [ ] |
| 197 | `ZSTD_CCtx_setParameter` | `RETURN_ERROR_IF((value!=0) && cctx->staticSize, parameter_unsupported, "MT not compatible with static alloc");` (c_src/src/compress/zstd_compress.c:721) | exact return/error shown | [ ] |
| 198 | `ZSTD_CCtx_setParameter` | `default: RETURN_ERROR(parameter_unsupported, "unknown parameter");` (c_src/src/compress/zstd_compress.c:765) | exact return/error shown | [ ] |
| 199 | `ZSTD_CCtxParams_setParameter` | `FORWARD_IF_ERROR(ZSTD_cParam_clampBounds(param, &value), "");` (c_src/src/compress/zstd_compress.c:782) | exact return/error shown | [ ] |
| 200 | `ZSTD_CCtxParams_setParameter` | `RETURN_ERROR_IF(value!=0, parameter_unsupported, "not compiled with multithreading");` (c_src/src/compress/zstd_compress.c:868) | exact return/error shown | [ ] |
| 201 | `ZSTD_CCtxParams_setParameter` | `FORWARD_IF_ERROR(ZSTD_cParam_clampBounds(param, &value), "");` (c_src/src/compress/zstd_compress.c:871) | exact return/error shown | [ ] |
| 202 | `ZSTD_CCtxParams_setParameter` | `RETURN_ERROR_IF(value!=0, parameter_unsupported, "not compiled with multithreading");` (c_src/src/compress/zstd_compress.c:878) | exact return/error shown | [ ] |
| 203 | `ZSTD_CCtxParams_setParameter` | `FORWARD_IF_ERROR(ZSTD_cParam_clampBounds(param, &value), "");` (c_src/src/compress/zstd_compress.c:884) | exact return/error shown | [ ] |
| 204 | `ZSTD_CCtxParams_setParameter` | `assert(value >= 0);` (c_src/src/compress/zstd_compress.c:885) | assertion/abort | [ ] |
| 205 | `ZSTD_CCtxParams_setParameter` | `RETURN_ERROR_IF(value!=0, parameter_unsupported, "not compiled with multithreading");` (c_src/src/compress/zstd_compress.c:892) | exact return/error shown | [ ] |
| 206 | `ZSTD_CCtxParams_setParameter` | `FORWARD_IF_ERROR(ZSTD_cParam_clampBounds(ZSTD_c_overlapLog, &value), "");` (c_src/src/compress/zstd_compress.c:895) | exact return/error shown | [ ] |
| 207 | `ZSTD_CCtxParams_setParameter` | `RETURN_ERROR_IF(value!=0, parameter_unsupported, "not compiled with multithreading");` (c_src/src/compress/zstd_compress.c:902) | exact return/error shown | [ ] |
| 208 | `ZSTD_CCtxParams_setParameter` | `FORWARD_IF_ERROR(ZSTD_cParam_clampBounds(ZSTD_c_overlapLog, &value), "");` (c_src/src/compress/zstd_compress.c:905) | exact return/error shown | [ ] |
| 209 | `ZSTD_CCtxParams_setParameter` | `assert(value>=0);` (c_src/src/compress/zstd_compress.c:1010) | assertion/abort | [ ] |
| 210 | `ZSTD_CCtxParams_setParameter` | `default: RETURN_ERROR(parameter_unsupported, "unknown parameter");` (c_src/src/compress/zstd_compress.c:1019) | exact return/error shown | [ ] |
| 211 | `ZSTD_CCtxParams_getParameter` | `assert(CCtxParams->nbWorkers == 0);` (c_src/src/compress/zstd_compress.c:1080) | assertion/abort | [ ] |
| 212 | `ZSTD_CCtxParams_getParameter` | `RETURN_ERROR(parameter_unsupported, "not compiled with multithreading");` (c_src/src/compress/zstd_compress.c:1086) | exact return/error shown | [ ] |
| 213 | `ZSTD_CCtxParams_getParameter` | `assert(CCtxParams->jobSize <= INT_MAX);` (c_src/src/compress/zstd_compress.c:1088) | assertion/abort | [ ] |
| 214 | `ZSTD_CCtxParams_getParameter` | `RETURN_ERROR(parameter_unsupported, "not compiled with multithreading");` (c_src/src/compress/zstd_compress.c:1094) | exact return/error shown | [ ] |
| 215 | `ZSTD_CCtxParams_getParameter` | `RETURN_ERROR(parameter_unsupported, "not compiled with multithreading");` (c_src/src/compress/zstd_compress.c:1101) | exact return/error shown | [ ] |
| 216 | `ZSTD_CCtxParams_getParameter` | `default: RETURN_ERROR(parameter_unsupported, "unknown parameter");` (c_src/src/compress/zstd_compress.c:1166) | exact return/error shown | [ ] |
| 217 | `ZSTD_CCtx_setParametersUsingCCtxParams` | `RETURN_ERROR_IF(cctx->streamStage != zcss_init, stage_wrong, "The context is in the wrong stage!");` (c_src/src/compress/zstd_compress.c:1182) | exact return/error shown | [ ] |
| 218 | `ZSTD_CCtx_setParametersUsingCCtxParams` | `RETURN_ERROR_IF(cctx->cdict, stage_wrong, "Can't override parameters with cdict attached (some must " "be inherited from the cdict).");` (c_src/src/compress/zstd_compress.c:1184) | exact return/error shown | [ ] |
| 219 | `ZSTD_CCtx_setCParams` | `ZSTD_STATIC_ASSERT(sizeof(cparams) == 7 * 4 /* all params are listed below */);` (c_src/src/compress/zstd_compress.c:1194) | exact return/error shown | [ ] |
| 220 | `ZSTD_CCtx_setCParams` | `FORWARD_IF_ERROR(ZSTD_checkCParams(cparams), "");` (c_src/src/compress/zstd_compress.c:1197) | exact return/error shown | [ ] |
| 221 | `ZSTD_CCtx_setCParams` | `FORWARD_IF_ERROR(ZSTD_CCtx_setParameter(cctx, ZSTD_c_windowLog, (int)cparams.windowLog), "");` (c_src/src/compress/zstd_compress.c:1198) | exact return/error shown | [ ] |
| 222 | `ZSTD_CCtx_setCParams` | `FORWARD_IF_ERROR(ZSTD_CCtx_setParameter(cctx, ZSTD_c_chainLog, (int)cparams.chainLog), "");` (c_src/src/compress/zstd_compress.c:1199) | exact return/error shown | [ ] |
| 223 | `ZSTD_CCtx_setCParams` | `FORWARD_IF_ERROR(ZSTD_CCtx_setParameter(cctx, ZSTD_c_hashLog, (int)cparams.hashLog), "");` (c_src/src/compress/zstd_compress.c:1200) | exact return/error shown | [ ] |
| 224 | `ZSTD_CCtx_setCParams` | `FORWARD_IF_ERROR(ZSTD_CCtx_setParameter(cctx, ZSTD_c_searchLog, (int)cparams.searchLog), "");` (c_src/src/compress/zstd_compress.c:1201) | exact return/error shown | [ ] |
| 225 | `ZSTD_CCtx_setCParams` | `FORWARD_IF_ERROR(ZSTD_CCtx_setParameter(cctx, ZSTD_c_minMatch, (int)cparams.minMatch), "");` (c_src/src/compress/zstd_compress.c:1202) | exact return/error shown | [ ] |
| 226 | `ZSTD_CCtx_setCParams` | `FORWARD_IF_ERROR(ZSTD_CCtx_setParameter(cctx, ZSTD_c_targetLength, (int)cparams.targetLength), "");` (c_src/src/compress/zstd_compress.c:1203) | exact return/error shown | [ ] |
| 227 | `ZSTD_CCtx_setCParams` | `FORWARD_IF_ERROR(ZSTD_CCtx_setParameter(cctx, ZSTD_c_strategy, (int)cparams.strategy), "");` (c_src/src/compress/zstd_compress.c:1204) | exact return/error shown | [ ] |
| 228 | `ZSTD_CCtx_setFParams` | `ZSTD_STATIC_ASSERT(sizeof(fparams) == 3 * 4 /* all params are listed below */);` (c_src/src/compress/zstd_compress.c:1210) | exact return/error shown | [ ] |
| 229 | `ZSTD_CCtx_setFParams` | `FORWARD_IF_ERROR(ZSTD_CCtx_setParameter(cctx, ZSTD_c_contentSizeFlag, fparams.contentSizeFlag != 0), "");` (c_src/src/compress/zstd_compress.c:1212) | exact return/error shown | [ ] |
| 230 | `ZSTD_CCtx_setFParams` | `FORWARD_IF_ERROR(ZSTD_CCtx_setParameter(cctx, ZSTD_c_checksumFlag, fparams.checksumFlag != 0), "");` (c_src/src/compress/zstd_compress.c:1213) | exact return/error shown | [ ] |
| 231 | `ZSTD_CCtx_setFParams` | `FORWARD_IF_ERROR(ZSTD_CCtx_setParameter(cctx, ZSTD_c_dictIDFlag, fparams.noDictIDFlag == 0), "");` (c_src/src/compress/zstd_compress.c:1214) | exact return/error shown | [ ] |
| 232 | `ZSTD_CCtx_setParams` | `FORWARD_IF_ERROR(ZSTD_checkCParams(params.cParams), "");` (c_src/src/compress/zstd_compress.c:1222) | exact return/error shown | [ ] |
| 233 | `ZSTD_CCtx_setParams` | `FORWARD_IF_ERROR(ZSTD_CCtx_setFParams(cctx, params.fParams), "");` (c_src/src/compress/zstd_compress.c:1224) | exact return/error shown | [ ] |
| 234 | `ZSTD_CCtx_setParams` | `FORWARD_IF_ERROR(ZSTD_CCtx_setCParams(cctx, params.cParams), "");` (c_src/src/compress/zstd_compress.c:1226) | exact return/error shown | [ ] |
| 235 | `ZSTD_CCtx_setPledgedSrcSize` | `RETURN_ERROR_IF(cctx->streamStage != zcss_init, stage_wrong, "Can't set pledgedSrcSize when not in init stage.");` (c_src/src/compress/zstd_compress.c:1233) | exact return/error shown | [ ] |
| 236 | `ZSTD_initLocalDict` | `assert(dl->dictBuffer == NULL);` (c_src/src/compress/zstd_compress.c:1257) | assertion/abort | [ ] |
| 237 | `ZSTD_initLocalDict` | `assert(dl->cdict == NULL);` (c_src/src/compress/zstd_compress.c:1258) | assertion/abort | [ ] |
| 238 | `ZSTD_initLocalDict` | `assert(dl->dictSize == 0);` (c_src/src/compress/zstd_compress.c:1259) | assertion/abort | [ ] |
| 239 | `ZSTD_initLocalDict` | `assert(cctx->cdict == dl->cdict);` (c_src/src/compress/zstd_compress.c:1264) | assertion/abort | [ ] |
| 240 | `ZSTD_initLocalDict` | `assert(dl->dictSize > 0);` (c_src/src/compress/zstd_compress.c:1267) | assertion/abort | [ ] |
| 241 | `ZSTD_initLocalDict` | `assert(cctx->cdict == NULL);` (c_src/src/compress/zstd_compress.c:1268) | assertion/abort | [ ] |
| 242 | `ZSTD_initLocalDict` | `assert(cctx->prefixDict.dict == NULL);` (c_src/src/compress/zstd_compress.c:1269) | assertion/abort | [ ] |
| 243 | `ZSTD_initLocalDict` | `RETURN_ERROR_IF(!dl->cdict, memory_allocation, "ZSTD_createCDict_advanced failed");` (c_src/src/compress/zstd_compress.c:1278) | exact return/error shown | [ ] |
| 244 | `ZSTD_CCtx_loadDictionary_advanced` | `RETURN_ERROR_IF(cctx->streamStage != zcss_init, stage_wrong, "Can't load a dictionary when cctx is not in init stage.");` (c_src/src/compress/zstd_compress.c:1290) | exact return/error shown | [ ] |
| 245 | `ZSTD_CCtx_loadDictionary_advanced` | `RETURN_ERROR_IF(cctx->staticSize, memory_allocation, "static CCtx can't allocate for an internal copy of dictionary");` (c_src/src/compress/zstd_compress.c:1300) | exact return/error shown | [ ] |
| 246 | `ZSTD_CCtx_loadDictionary_advanced` | `RETURN_ERROR_IF(dictBuffer==NULL, memory_allocation, "allocation failed for dictionary content");` (c_src/src/compress/zstd_compress.c:1303) | exact return/error shown | [ ] |
| 247 | `ZSTD_CCtx_refCDict` | `RETURN_ERROR_IF(cctx->streamStage != zcss_init, stage_wrong, "Can't ref a dict when ctx not in init stage.");` (c_src/src/compress/zstd_compress.c:1330) | exact return/error shown | [ ] |
| 248 | `ZSTD_CCtx_refThreadPool` | `RETURN_ERROR_IF(cctx->streamStage != zcss_init, stage_wrong, "Can't ref a pool when ctx not in init stage.");` (c_src/src/compress/zstd_compress.c:1340) | exact return/error shown | [ ] |
| 249 | `ZSTD_CCtx_refPrefix_advanced` | `RETURN_ERROR_IF(cctx->streamStage != zcss_init, stage_wrong, "Can't ref a prefix when ctx not in init stage.");` (c_src/src/compress/zstd_compress.c:1354) | exact return/error shown | [ ] |
| 250 | `ZSTD_CCtx_reset` | `RETURN_ERROR_IF(cctx->streamStage != zcss_init, stage_wrong, "Reset parameters is only possible during init stage.");` (c_src/src/compress/zstd_compress.c:1376) | exact return/error shown | [ ] |
| 251 | `ZSTD_dictAndWindowLog` | `assert(windowLog <= ZSTD_WINDOWLOG_MAX);` (c_src/src/compress/zstd_compress.c:1446) | assertion/abort | [ ] |
| 252 | `ZSTD_dictAndWindowLog` | `assert(srcSize != ZSTD_CONTENTSIZE_UNKNOWN); /* Handled in ZSTD_adjustCParams_internal() */` (c_src/src/compress/zstd_compress.c:1447) | assertion/abort | [ ] |
| 253 | `ZSTD_adjustCParams_internal` | `assert(ZSTD_checkCParams(cPar)==0);` (c_src/src/compress/zstd_compress.c:1481) | assertion/abort | [ ] |
| 254 | `ZSTD_adjustCParams_internal` | `assert(0);` (c_src/src/compress/zstd_compress.c:1548) | assertion/abort | [ ] |
| 255 | `ZSTD_adjustCParams_internal` | `assert(cPar.hashLog >= rowLog);` (c_src/src/compress/zstd_compress.c:1602) | assertion/abort | [ ] |
| 256 | `ZSTD_getCParamsFromCCtxParams` | `assert(CCtxParams->srcSizeHint>=0);` (c_src/src/compress/zstd_compress.c:1642) | assertion/abort | [ ] |
| 257 | `ZSTD_getCParamsFromCCtxParams` | `assert(!ZSTD_checkCParams(cParams));` (c_src/src/compress/zstd_compress.c:1648) | assertion/abort | [ ] |
| 258 | `ZSTD_sizeof_matchState` | `ZSTD_STATIC_ASSERT(ZSTD_HASHLOG_MIN >= 4 && ZSTD_WINDOWLOG_MIN >= 4 && ZSTD_CHAINLOG_MIN >= 4);` (c_src/src/compress/zstd_compress.c:1687) | exact return/error shown | [ ] |
| 259 | `ZSTD_sizeof_matchState` | `assert(useRowMatchFinder != ZSTD_ps_auto);` (c_src/src/compress/zstd_compress.c:1688) | assertion/abort | [ ] |
| 260 | `ZSTD_estimateCCtxSize_usingCCtxParams` | `RETURN_ERROR_IF(params->nbWorkers > 0, GENERIC, "Estimate CCtx size is supported for single-threaded compression only.");` (c_src/src/compress/zstd_compress.c:1761) | exact return/error shown | [ ] |
| 261 | `ZSTD_estimateCStreamSize_usingCCtxParams` | `RETURN_ERROR_IF(params->nbWorkers > 0, GENERIC, "Estimate CCtx size is supported for single-threaded compression only.");` (c_src/src/compress/zstd_compress.c:1813) | exact return/error shown | [ ] |
| 262 | `ZSTD_getFrameProgression` | `if (buffered) assert(cctx->inBuffPos >= cctx->inToCompress);` (c_src/src/compress/zstd_compress.c:1879) | assertion/abort | [ ] |
| 263 | `ZSTD_getFrameProgression` | `assert(buffered <= ZSTD_BLOCKSIZE_MAX);` (c_src/src/compress/zstd_compress.c:1880) | assertion/abort | [ ] |
| 264 | `ZSTD_assertEqualCParams` | `assert(cParams1.windowLog == cParams2.windowLog);` (c_src/src/compress/zstd_compress.c:1909) | assertion/abort | [ ] |
| 265 | `ZSTD_assertEqualCParams` | `assert(cParams1.chainLog == cParams2.chainLog);` (c_src/src/compress/zstd_compress.c:1910) | assertion/abort | [ ] |
| 266 | `ZSTD_assertEqualCParams` | `assert(cParams1.hashLog == cParams2.hashLog);` (c_src/src/compress/zstd_compress.c:1911) | assertion/abort | [ ] |
| 267 | `ZSTD_assertEqualCParams` | `assert(cParams1.searchLog == cParams2.searchLog);` (c_src/src/compress/zstd_compress.c:1912) | assertion/abort | [ ] |
| 268 | `ZSTD_assertEqualCParams` | `assert(cParams1.minMatch == cParams2.minMatch);` (c_src/src/compress/zstd_compress.c:1913) | assertion/abort | [ ] |
| 269 | `ZSTD_assertEqualCParams` | `assert(cParams1.targetLength == cParams2.targetLength);` (c_src/src/compress/zstd_compress.c:1914) | assertion/abort | [ ] |
| 270 | `ZSTD_assertEqualCParams` | `assert(cParams1.strategy == cParams2.strategy);` (c_src/src/compress/zstd_compress.c:1915) | assertion/abort | [ ] |
| 271 | `ZSTD_reset_matchState` | `assert(useRowMatchFinder != ZSTD_ps_auto);` (c_src/src/compress/zstd_compress.c:2003) | assertion/abort | [ ] |
| 272 | `ZSTD_reset_matchState` | `assert(!ZSTD_cwksp_reserve_failed(ws)); /* check that allocation hasn't already failed */` (c_src/src/compress/zstd_compress.c:2014) | assertion/abort | [ ] |
| 273 | `ZSTD_reset_matchState` | `RETURN_ERROR_IF(ZSTD_cwksp_reserve_failed(ws), memory_allocation, "failed a workspace allocation in ZSTD_reset_matchState");` (c_src/src/compress/zstd_compress.c:2023) | exact return/error shown | [ ] |
| 274 | `ZSTD_reset_matchState` | `assert(cParams->hashLog >= rowLog);` (c_src/src/compress/zstd_compress.c:2048) | assertion/abort | [ ] |
| 275 | `ZSTD_reset_matchState` | `RETURN_ERROR_IF(ZSTD_cwksp_reserve_failed(ws), memory_allocation, "failed a workspace allocation in ZSTD_reset_matchState");` (c_src/src/compress/zstd_compress.c:2066) | exact return/error shown | [ ] |
| 276 | `ZSTD_resetCCtx_internal` | `assert(!ZSTD_isError(ZSTD_checkCParams(params->cParams)));` (c_src/src/compress/zstd_compress.c:2110) | assertion/abort | [ ] |
| 277 | `ZSTD_resetCCtx_internal` | `assert(params->useRowMatchFinder != ZSTD_ps_auto);` (c_src/src/compress/zstd_compress.c:2120) | assertion/abort | [ ] |
| 278 | `ZSTD_resetCCtx_internal` | `assert(params->postBlockSplitter != ZSTD_ps_auto);` (c_src/src/compress/zstd_compress.c:2121) | assertion/abort | [ ] |
| 279 | `ZSTD_resetCCtx_internal` | `assert(params->ldmParams.enableLdm != ZSTD_ps_auto);` (c_src/src/compress/zstd_compress.c:2122) | assertion/abort | [ ] |
| 280 | `ZSTD_resetCCtx_internal` | `assert(params->maxBlockSize != 0);` (c_src/src/compress/zstd_compress.c:2123) | assertion/abort | [ ] |
| 281 | `ZSTD_resetCCtx_internal` | `assert(params->ldmParams.hashLog >= params->ldmParams.bucketSizeLog);` (c_src/src/compress/zstd_compress.c:2127) | assertion/abort | [ ] |
| 282 | `ZSTD_resetCCtx_internal` | `assert(params->ldmParams.hashRateLog < 32);` (c_src/src/compress/zstd_compress.c:2128) | assertion/abort | [ ] |
| 283 | `ZSTD_resetCCtx_internal` | `FORWARD_IF_ERROR(neededSpace, "cctx size estimate failed!");` (c_src/src/compress/zstd_compress.c:2152) | exact return/error shown | [ ] |
| 284 | `ZSTD_resetCCtx_internal` | `RETURN_ERROR_IF(zc->staticSize, memory_allocation, "static cctx : no resize");` (c_src/src/compress/zstd_compress.c:2168) | exact return/error shown | [ ] |
| 285 | `ZSTD_resetCCtx_internal` | `FORWARD_IF_ERROR(ZSTD_cwksp_create(ws, neededSpace, zc->customMem), "");` (c_src/src/compress/zstd_compress.c:2173) | exact return/error shown | [ ] |
| 286 | `ZSTD_resetCCtx_internal` | `assert(ZSTD_cwksp_check_available(ws, 2 * sizeof(ZSTD_compressedBlockState_t)));` (c_src/src/compress/zstd_compress.c:2179) | assertion/abort | [ ] |
| 287 | `ZSTD_resetCCtx_internal` | `RETURN_ERROR_IF(zc->blockState.prevCBlock == NULL, memory_allocation, "couldn't allocate prevCBlock");` (c_src/src/compress/zstd_compress.c:2181) | exact return/error shown | [ ] |
| 288 | `ZSTD_resetCCtx_internal` | `RETURN_ERROR_IF(zc->blockState.nextCBlock == NULL, memory_allocation, "couldn't allocate nextCBlock");` (c_src/src/compress/zstd_compress.c:2183) | exact return/error shown | [ ] |
| 289 | `ZSTD_resetCCtx_internal` | `RETURN_ERROR_IF(zc->tmpWorkspace == NULL, memory_allocation, "couldn't allocate tmpWorkspace");` (c_src/src/compress/zstd_compress.c:2185) | exact return/error shown | [ ] |
| 290 | `ZSTD_resetCCtx_internal` | `FORWARD_IF_ERROR(ZSTD_reset_matchState( &zc->blockState.matchState, ws, &params->cParams, params->useRowMatchFinder, crp, needsIndexReset, ZSTD_resetTarget_CCtx), "");` (c_src/src/compress/zstd_compress.c:2210) | exact return/error shown | [ ] |
| 291 | `ZSTD_resetCCtx_internal` | `assert(ZSTD_cwksp_estimated_space_within_bounds(ws, neededSpace));` (c_src/src/compress/zstd_compress.c:2274) | assertion/abort | [ ] |
| 292 | `ZSTD_invalidateRepCodes` | `assert(!ZSTD_window_hasExtDict(cctx->blockState.matchState.window));` (c_src/src/compress/zstd_compress.c:2289) | assertion/abort | [ ] |
| 293 | `ZSTD_resetCCtx_byAttachingCDict` | `assert(windowLog != 0);` (c_src/src/compress/zstd_compress.c:2336) | assertion/abort | [ ] |
| 294 | `ZSTD_resetCCtx_byAttachingCDict` | `FORWARD_IF_ERROR(ZSTD_resetCCtx_internal(cctx, &params, pledgedSrcSize, /* loadedDictSize */ 0, ZSTDcrp_makeClean, zbuff), "");` (c_src/src/compress/zstd_compress.c:2350) | exact return/error shown | [ ] |
| 295 | `ZSTD_resetCCtx_byAttachingCDict` | `assert(cctx->appliedParams.cParams.strategy == adjusted_cdict_cParams.strategy);` (c_src/src/compress/zstd_compress.c:2353) | assertion/abort | [ ] |
| 296 | `ZSTD_resetCCtx_byCopyingCDict` | `assert(!cdict->matchState.dedicatedDictSearch);` (c_src/src/compress/zstd_compress.c:2410) | assertion/abort | [ ] |
| 297 | `ZSTD_resetCCtx_byCopyingCDict` | `assert(windowLog != 0);` (c_src/src/compress/zstd_compress.c:2415) | assertion/abort | [ ] |
| 298 | `ZSTD_resetCCtx_byCopyingCDict` | `FORWARD_IF_ERROR(ZSTD_resetCCtx_internal(cctx, &params, pledgedSrcSize, /* loadedDictSize */ 0, ZSTDcrp_leaveDirty, zbuff), "");` (c_src/src/compress/zstd_compress.c:2420) | exact return/error shown | [ ] |
| 299 | `ZSTD_resetCCtx_byCopyingCDict` | `assert(cctx->appliedParams.cParams.strategy == cdict_cParams->strategy);` (c_src/src/compress/zstd_compress.c:2423) | assertion/abort | [ ] |
| 300 | `ZSTD_resetCCtx_byCopyingCDict` | `assert(cctx->appliedParams.cParams.hashLog == cdict_cParams->hashLog);` (c_src/src/compress/zstd_compress.c:2424) | assertion/abort | [ ] |
| 301 | `ZSTD_resetCCtx_byCopyingCDict` | `assert(cctx->appliedParams.cParams.chainLog == cdict_cParams->chainLog);` (c_src/src/compress/zstd_compress.c:2425) | assertion/abort | [ ] |
| 302 | `ZSTD_resetCCtx_byCopyingCDict` | `assert(params.useRowMatchFinder != ZSTD_ps_auto);` (c_src/src/compress/zstd_compress.c:2429) | assertion/abort | [ ] |
| 303 | `ZSTD_resetCCtx_byCopyingCDict` | `assert(cctx->blockState.matchState.hashLog3 <= 31);` (c_src/src/compress/zstd_compress.c:2458) | assertion/abort | [ ] |
| 304 | `ZSTD_resetCCtx_byCopyingCDict` | `assert(cdict->matchState.hashLog3 == 0);` (c_src/src/compress/zstd_compress.c:2461) | assertion/abort | [ ] |
| 305 | `ZSTD_copyCCtx_internal` | `RETURN_ERROR_IF(srcCCtx->stage!=ZSTDcs_init, stage_wrong, "Can't copy a ctx that's not in init stage.");` (c_src/src/compress/zstd_compress.c:2519) | exact return/error shown | [ ] |
| 306 | `ZSTD_copyCCtx_internal` | `assert(srcCCtx->appliedParams.useRowMatchFinder != ZSTD_ps_auto);` (c_src/src/compress/zstd_compress.c:2526) | assertion/abort | [ ] |
| 307 | `ZSTD_copyCCtx_internal` | `assert(srcCCtx->appliedParams.postBlockSplitter != ZSTD_ps_auto);` (c_src/src/compress/zstd_compress.c:2527) | assertion/abort | [ ] |
| 308 | `ZSTD_copyCCtx_internal` | `assert(srcCCtx->appliedParams.ldmParams.enableLdm != ZSTD_ps_auto);` (c_src/src/compress/zstd_compress.c:2528) | assertion/abort | [ ] |
| 309 | `ZSTD_copyCCtx_internal` | `assert(dstCCtx->appliedParams.cParams.windowLog == srcCCtx->appliedParams.cParams.windowLog);` (c_src/src/compress/zstd_compress.c:2537) | assertion/abort | [ ] |
| 310 | `ZSTD_copyCCtx_internal` | `assert(dstCCtx->appliedParams.cParams.strategy == srcCCtx->appliedParams.cParams.strategy);` (c_src/src/compress/zstd_compress.c:2538) | assertion/abort | [ ] |
| 311 | `ZSTD_copyCCtx_internal` | `assert(dstCCtx->appliedParams.cParams.hashLog == srcCCtx->appliedParams.cParams.hashLog);` (c_src/src/compress/zstd_compress.c:2539) | assertion/abort | [ ] |
| 312 | `ZSTD_copyCCtx_internal` | `assert(dstCCtx->appliedParams.cParams.chainLog == srcCCtx->appliedParams.cParams.chainLog);` (c_src/src/compress/zstd_compress.c:2540) | assertion/abort | [ ] |
| 313 | `ZSTD_copyCCtx_internal` | `assert(dstCCtx->blockState.matchState.hashLog3 == srcCCtx->blockState.matchState.hashLog3);` (c_src/src/compress/zstd_compress.c:2541) | assertion/abort | [ ] |
| 314 | `ZSTD_copyCCtx` | `ZSTD_STATIC_ASSERT((U32)ZSTDb_buffered==1);` (c_src/src/compress/zstd_compress.c:2595) | exact return/error shown | [ ] |
| 315 | `ZSTD_reduceTable_internal` | `assert((size & (ZSTD_ROWSIZE-1)) == 0); /* multiple of ZSTD_ROWSIZE */` (c_src/src/compress/zstd_compress.c:2620) | assertion/abort | [ ] |
| 316 | `ZSTD_reduceTable_internal` | `assert(size < (1U<<31)); /* can be cast to int */` (c_src/src/compress/zstd_compress.c:2621) | assertion/abort | [ ] |
| 317 | `ZSTD_seqToCodes` | `assert(nbSeq <= seqStorePtr->maxNbSeq);` (c_src/src/compress/zstd_compress.c:2702) | assertion/abort | [ ] |
| 318 | `ZSTD_seqToCodes` | `assert(!(MEM_64bits() && ofCode >= STREAM_ACCUMULATOR_MIN));` (c_src/src/compress/zstd_compress.c:2710) | assertion/abort | [ ] |
| 319 | `ZSTD_blockSplitterEnabled` | `assert(cctxParams->postBlockSplitter != ZSTD_ps_auto);` (c_src/src/compress/zstd_compress.c:2739) | assertion/abort | [ ] |
| 320 | `ZSTD_buildSequencesStatistics` | `assert(op <= oend);` (c_src/src/compress/zstd_compress.c:2784) | assertion/abort | [ ] |
| 321 | `ZSTD_buildSequencesStatistics` | `assert(nbSeq != 0); /* ZSTD_selectEncodingType() divides by nbSeq */` (c_src/src/compress/zstd_compress.c:2785) | assertion/abort | [ ] |
| 322 | `ZSTD_buildSequencesStatistics` | `assert(set_basic < set_compressed && set_rle < set_compressed);` (c_src/src/compress/zstd_compress.c:2796) | assertion/abort | [ ] |
| 323 | `ZSTD_buildSequencesStatistics` | `assert(!(stats.LLtype < set_compressed && nextEntropy->litlength_repeatMode != FSE_repeat_none)); /* We don't copy tables */` (c_src/src/compress/zstd_compress.c:2797) | assertion/abort | [ ] |
| 324 | `ZSTD_buildSequencesStatistics` | `assert(op <= oend);` (c_src/src/compress/zstd_compress.c:2814) | assertion/abort | [ ] |
| 325 | `ZSTD_buildSequencesStatistics` | `assert(!(stats.Offtype < set_compressed && nextEntropy->offcode_repeatMode != FSE_repeat_none)); /* We don't copy tables */` (c_src/src/compress/zstd_compress.c:2829) | assertion/abort | [ ] |
| 326 | `ZSTD_buildSequencesStatistics` | `assert(op <= oend);` (c_src/src/compress/zstd_compress.c:2846) | assertion/abort | [ ] |
| 327 | `ZSTD_buildSequencesStatistics` | `assert(!(stats.MLtype < set_compressed && nextEntropy->matchlength_repeatMode != FSE_repeat_none)); /* We don't copy tables */` (c_src/src/compress/zstd_compress.c:2859) | assertion/abort | [ ] |
| 328 | `ZSTD_buildSequencesStatistics` | `assert(op <= oend);` (c_src/src/compress/zstd_compress.c:2876) | assertion/abort | [ ] |
| 329 | `ZSTD_entropyCompressSeqStore_internal` | `ZSTD_STATIC_ASSERT(HUF_WORKSPACE_SIZE >= (1<<MAX(MLFSELog,LLFSELog)));` (c_src/src/compress/zstd_compress.c:2918) | exact return/error shown | [ ] |
| 330 | `ZSTD_entropyCompressSeqStore_internal` | `assert(entropyWkspSize >= HUF_WORKSPACE_SIZE);` (c_src/src/compress/zstd_compress.c:2919) | assertion/abort | [ ] |
| 331 | `ZSTD_entropyCompressSeqStore_internal` | `FORWARD_IF_ERROR(cSize, "ZSTD_compressLiterals failed");` (c_src/src/compress/zstd_compress.c:2934) | exact return/error shown | [ ] |
| 332 | `ZSTD_entropyCompressSeqStore_internal` | `assert(cSize <= dstCapacity);` (c_src/src/compress/zstd_compress.c:2935) | assertion/abort | [ ] |
| 333 | `ZSTD_entropyCompressSeqStore_internal` | `RETURN_ERROR_IF((oend-op) < 3 /*max nbSeq Size*/ + 1 /*seqHead*/, dstSize_tooSmall, "Can't fit seq hdr in output buf!");` (c_src/src/compress/zstd_compress.c:2940) | exact return/error shown | [ ] |
| 334 | `ZSTD_entropyCompressSeqStore_internal` | `assert(op <= oend);` (c_src/src/compress/zstd_compress.c:2953) | assertion/abort | [ ] |
| 335 | `ZSTD_entropyCompressSeqStore_internal` | `FORWARD_IF_ERROR(stats.size, "ZSTD_buildSequencesStatistics failed!");` (c_src/src/compress/zstd_compress.c:2967) | exact return/error shown | [ ] |
| 336 | `ZSTD_entropyCompressSeqStore_internal` | `FORWARD_IF_ERROR(bitstreamSize, "ZSTD_encodeSequences failed");` (c_src/src/compress/zstd_compress.c:2981) | exact return/error shown | [ ] |
| 337 | `ZSTD_entropyCompressSeqStore_internal` | `assert(op <= oend);` (c_src/src/compress/zstd_compress.c:2983) | assertion/abort | [ ] |
| 338 | `ZSTD_entropyCompressSeqStore_internal` | `assert(lastCountSize + bitstreamSize == 3);` (c_src/src/compress/zstd_compress.c:2994) | assertion/abort | [ ] |
| 339 | `ZSTD_entropyCompressSeqStore_wExtLitBuffer` | `FORWARD_IF_ERROR(cSize, "ZSTD_entropyCompressSeqStore_internal failed");` (c_src/src/compress/zstd_compress.c:3030) | exact return/error shown | [ ] |
| 340 | `ZSTD_entropyCompressSeqStore_wExtLitBuffer` | `assert(cSize < ZSTD_BLOCKSIZE_MAX);` (c_src/src/compress/zstd_compress.c:3040) | assertion/abort | [ ] |
| 341 | `ZSTD_selectBlockCompressor` | `ZSTD_STATIC_ASSERT((unsigned)ZSTD_fast == 1);` (c_src/src/compress/zstd_compress.c:3117) | exact return/error shown | [ ] |
| 342 | `ZSTD_selectBlockCompressor` | `assert(ZSTD_cParam_withinBounds(ZSTD_c_strategy, (int)strat));` (c_src/src/compress/zstd_compress.c:3119) | assertion/abort | [ ] |
| 343 | `ZSTD_selectBlockCompressor` | `assert(useRowMatchFinder != ZSTD_ps_auto);` (c_src/src/compress/zstd_compress.c:3145) | assertion/abort | [ ] |
| 344 | `ZSTD_selectBlockCompressor` | `assert(selectedCompressor != NULL);` (c_src/src/compress/zstd_compress.c:3150) | assertion/abort | [ ] |
| 345 | `ZSTD_postProcessSequenceProducerResult` | `RETURN_ERROR_IF( nbExternalSeqs > outSeqsCapacity, sequenceProducer_failed, "External sequence producer returned error code %lu", (unsigned long)nbExternalSeqs );` (c_src/src/compress/zstd_compress.c:3177) | exact return/error shown | [ ] |
| 346 | `ZSTD_postProcessSequenceProducerResult` | `RETURN_ERROR_IF( nbExternalSeqs == 0 && srcSize > 0, sequenceProducer_failed, "Got zero sequences from external sequence producer for a non-empty src buffer!" );` (c_src/src/compress/zstd_compress.c:3184) | exact return/error shown | [ ] |
| 347 | `ZSTD_postProcessSequenceProducerResult` | `RETURN_ERROR_IF( nbExternalSeqs == outSeqsCapacity, sequenceProducer_failed, "nbExternalSeqs == outSeqsCapacity but lastSeq is not a block delimiter!" );` (c_src/src/compress/zstd_compress.c:3205) | exact return/error shown | [ ] |
| 348 | `ZSTD_validateSeqStore` | `assert(seqLength.matchLength >= matchLenLowerBound);` (c_src/src/compress/zstd_compress.c:3245) | assertion/abort | [ ] |
| 349 | `ZSTD_buildSeqStore` | `assert(srcSize <= ZSTD_BLOCKSIZE_MAX);` (c_src/src/compress/zstd_compress.c:3268) | assertion/abort | [ ] |
| 350 | `ZSTD_buildSeqStore` | `assert(ms->dictMatchState == NULL \|\| ms->loadedDictEnd == ms->window.dictLimit);` (c_src/src/compress/zstd_compress.c:3289) | assertion/abort | [ ] |
| 351 | `ZSTD_buildSeqStore` | `if (sizeof(ptrdiff_t)==8) assert(istart - base < (ptrdiff_t)(U32)(-1)); /* ensure no overflow */` (c_src/src/compress/zstd_compress.c:3295) | assertion/abort | [ ] |
| 352 | `ZSTD_buildSeqStore` | `assert(zc->appliedParams.ldmParams.enableLdm == ZSTD_ps_disable);` (c_src/src/compress/zstd_compress.c:3308) | assertion/abort | [ ] |
| 353 | `ZSTD_buildSeqStore` | `RETURN_ERROR_IF( ZSTD_hasExtSeqProd(&zc->appliedParams), parameter_combination_unsupported, "Long-distance matching with external sequence producer enabled is not currently supported." );` (c_src/src/compress/zstd_compress.c:3312) | exact return/error shown | [ ] |
| 354 | `ZSTD_buildSeqStore` | `assert(zc->externSeqStore.pos <= zc->externSeqStore.size);` (c_src/src/compress/zstd_compress.c:3325) | assertion/abort | [ ] |
| 355 | `ZSTD_buildSeqStore` | `RETURN_ERROR_IF( ZSTD_hasExtSeqProd(&zc->appliedParams), parameter_combination_unsupported, "Long-distance matching with external sequence producer enabled is not currently supported." );` (c_src/src/compress/zstd_compress.c:3331) | exact return/error shown | [ ] |
| 356 | `ZSTD_buildSeqStore` | `FORWARD_IF_ERROR(ZSTD_ldm_generateSequences(&zc->ldmState, &ldmSeqStore, &zc->appliedParams.ldmParams, src, srcSize), "");` (c_src/src/compress/zstd_compress.c:3340) | exact return/error shown | [ ] |
| 357 | `ZSTD_buildSeqStore` | `assert(ldmSeqStore.pos == ldmSeqStore.size);` (c_src/src/compress/zstd_compress.c:3350) | assertion/abort | [ ] |
| 358 | `ZSTD_buildSeqStore` | `assert( zc->extSeqBufCapacity >= ZSTD_sequenceBound(srcSize) );` (c_src/src/compress/zstd_compress.c:3352) | assertion/abort | [ ] |
| 359 | `ZSTD_buildSeqStore` | `assert(zc->appliedParams.extSeqProdFunc != NULL);` (c_src/src/compress/zstd_compress.c:3355) | assertion/abort | [ ] |
| 360 | `ZSTD_buildSeqStore` | `RETURN_ERROR_IF(seqLenSum > srcSize, externalSequences_invalid, "External sequences imply too large a block!");` (c_src/src/compress/zstd_compress.c:3380) | exact return/error shown | [ ] |
| 361 | `ZSTD_buildSeqStore` | `FORWARD_IF_ERROR( ZSTD_transferSequences_wBlockDelim( zc, &seqPos, zc->extSeqBuf, nbPostProcessedSeqs, src, srcSize, zc->appliedParams.searchForExternalRepcodes ), "Failed to copy external sequences to seqStore!" );` (c_src/src/compress/zstd_compress.c:3381) | exact return/error shown | [ ] |
| 362 | `ZSTD_copyBlockSequences` | `assert(seqCollector->seqIndex <= seqCollector->maxSequences);` (c_src/src/compress/zstd_compress.c:3444) | assertion/abort | [ ] |
| 363 | `ZSTD_copyBlockSequences` | `RETURN_ERROR_IF( nbOutSequences > (size_t)(seqCollector->maxSequences - seqCollector->seqIndex), dstSize_tooSmall, "Not enough space to copy sequences");` (c_src/src/compress/zstd_compress.c:3445) | exact return/error shown | [ ] |
| 364 | `ZSTD_copyBlockSequences` | `assert(repcode > 0);` (c_src/src/compress/zstd_compress.c:3472) | assertion/abort | [ ] |
| 365 | `ZSTD_copyBlockSequences` | `assert(repcodes.rep[0] > 1);` (c_src/src/compress/zstd_compress.c:3478) | assertion/abort | [ ] |
| 366 | `ZSTD_copyBlockSequences` | `assert(nbInLiterals >= nbOutLiterals);` (c_src/src/compress/zstd_compress.c:3500) | assertion/abort | [ ] |
| 367 | `ZSTD_copyBlockSequences` | `assert(nbOutSequences == nbInSequences + 1);` (c_src/src/compress/zstd_compress.c:3506) | assertion/abort | [ ] |
| 368 | `ZSTD_copyBlockSequences` | `assert(seqCollector->seqIndex <= seqCollector->maxSequences);` (c_src/src/compress/zstd_compress.c:3509) | assertion/abort | [ ] |
| 369 | `ZSTD_generateSequences` | `FORWARD_IF_ERROR(ZSTD_CCtx_getParameter(zc, ZSTD_c_targetCBlockSize, &targetCBlockSize), "");` (c_src/src/compress/zstd_compress.c:3528) | exact return/error shown | [ ] |
| 370 | `ZSTD_generateSequences` | `RETURN_ERROR_IF(targetCBlockSize != 0, parameter_unsupported, "targetCBlockSize != 0");` (c_src/src/compress/zstd_compress.c:3529) | exact return/error shown | [ ] |
| 371 | `ZSTD_generateSequences` | `FORWARD_IF_ERROR(ZSTD_CCtx_getParameter(zc, ZSTD_c_nbWorkers, &nbWorkers), "");` (c_src/src/compress/zstd_compress.c:3533) | exact return/error shown | [ ] |
| 372 | `ZSTD_generateSequences` | `RETURN_ERROR_IF(nbWorkers != 0, parameter_unsupported, "nbWorkers != 0");` (c_src/src/compress/zstd_compress.c:3534) | exact return/error shown | [ ] |
| 373 | `ZSTD_generateSequences` | `RETURN_ERROR_IF(dst == NULL, memory_allocation, "NULL pointer!");` (c_src/src/compress/zstd_compress.c:3538) | exact return/error shown | [ ] |
| 374 | `ZSTD_generateSequences` | `FORWARD_IF_ERROR(ret, "ZSTD_compress2 failed");` (c_src/src/compress/zstd_compress.c:3549) | exact return/error shown | [ ] |
| 375 | `ZSTD_generateSequences` | `assert(zc->seqCollector.seqIndex <= ZSTD_sequenceBound(srcSize));` (c_src/src/compress/zstd_compress.c:3551) | assertion/abort | [ ] |
| 376 | `ZSTD_buildBlockEntropyStats_literals` | `FORWARD_IF_ERROR(largest, "HIST_count_wksp failed");` (c_src/src/compress/zstd_compress.c:3678) | exact return/error shown | [ ] |
| 377 | `ZSTD_buildBlockEntropyStats_literals` | `assert(huffLog <= LitHufLog);` (c_src/src/compress/zstd_compress.c:3701) | assertion/abort | [ ] |
| 378 | `ZSTD_buildBlockEntropyStats_literals` | `FORWARD_IF_ERROR(maxBits, "HUF_buildCTable_wksp");` (c_src/src/compress/zstd_compress.c:3705) | exact return/error shown | [ ] |
| 379 | `ZSTD_buildBlockEntropyStats_sequences` | `FORWARD_IF_ERROR(stats.size, "ZSTD_buildSequencesStatistics failed!");` (c_src/src/compress/zstd_compress.c:3783) | exact return/error shown | [ ] |
| 380 | `ZSTD_buildBlockEntropyStats` | `FORWARD_IF_ERROR(entropyMetadata->hufMetadata.hufDesSize, "ZSTD_buildBlockEntropyStats_literals failed");` (c_src/src/compress/zstd_compress.c:3817) | exact return/error shown | [ ] |
| 381 | `ZSTD_buildBlockEntropyStats` | `FORWARD_IF_ERROR(entropyMetadata->fseMetadata.fseTablesSize, "ZSTD_buildBlockEntropyStats_sequences failed");` (c_src/src/compress/zstd_compress.c:3824) | exact return/error shown | [ ] |
| 382 | `ZSTD_estimateBlockSize_literal` | `assert(0); /* impossible */` (c_src/src/compress/zstd_compress.c:3851) | assertion/abort | [ ] |
| 383 | `ZSTD_estimateBlockSize_symbolType` | `assert(max <= defaultMax);` (c_src/src/compress/zstd_compress.c:3874) | assertion/abort | [ ] |
| 384 | `ZSTD_buildEntropyStatisticsAndEstimateSubBlockSize` | `FORWARD_IF_ERROR(ZSTD_buildBlockEntropyStats(seqStore, &zc->blockState.prevCBlock->entropy, &zc->blockState.nextCBlock->entropy, &zc->appliedParams, entropyMetadata, zc->tmpWorkspace, zc->tmpWkspSize), "");` (c_src/src/compress/zstd_compress.c:3952) | exact return/error shown | [ ] |
| 385 | `ZSTD_deriveSeqStoreChunk` | `assert(resultSeqStore->lit == originalSeqStore->lit);` (c_src/src/compress/zstd_compress.c:4023) | assertion/abort | [ ] |
| 386 | `ZSTD_resolveRepcodeToRawOffset` | `assert(OFFBASE_IS_REPCODE(offBase));` (c_src/src/compress/zstd_compress.c:4041) | assertion/abort | [ ] |
| 387 | `ZSTD_resolveRepcodeToRawOffset` | `assert(ll0);` (c_src/src/compress/zstd_compress.c:4043) | assertion/abort | [ ] |
| 388 | `ZSTD_seqStore_resolveOffCodes` | `assert(offBase > 0);` (c_src/src/compress/zstd_compress.c:4079) | assertion/abort | [ ] |
| 389 | `ZSTD_compressSeqStore_singleBlock` | `RETURN_ERROR_IF(dstCapacity < ZSTD_blockHeaderSize, dstSize_tooSmall, "Block header doesn't fit");` (c_src/src/compress/zstd_compress.c:4124) | exact return/error shown | [ ] |
| 390 | `ZSTD_compressSeqStore_singleBlock` | `FORWARD_IF_ERROR(cSeqsSize, "ZSTD_entropyCompressSeqStore failed!");` (c_src/src/compress/zstd_compress.c:4132) | exact return/error shown | [ ] |
| 391 | `ZSTD_compressSeqStore_singleBlock` | `FORWARD_IF_ERROR(ZSTD_copyBlockSequences(&zc->seqCollector, seqStore, dRepOriginal.rep), "copyBlockSequences failed");` (c_src/src/compress/zstd_compress.c:4146) | exact return/error shown | [ ] |
| 392 | `ZSTD_compressSeqStore_singleBlock` | `FORWARD_IF_ERROR(cSize, "Nocompress block failed");` (c_src/src/compress/zstd_compress.c:4153) | exact return/error shown | [ ] |
| 393 | `ZSTD_compressSeqStore_singleBlock` | `FORWARD_IF_ERROR(cSize, "RLE compress block failed");` (c_src/src/compress/zstd_compress.c:4158) | exact return/error shown | [ ] |
| 394 | `ZSTD_deriveBlockSplitsHelper` | `assert(endIdx >= startIdx);` (c_src/src/compress/zstd_compress.c:4209) | assertion/abort | [ ] |
| 395 | `ZSTD_compressBlock_splitBlock_internal` | `FORWARD_IF_ERROR(cSizeSingleBlock, "Compressing single block from splitBlock_internal() failed!");` (c_src/src/compress/zstd_compress.c:4307) | exact return/error shown | [ ] |
| 396 | `ZSTD_compressBlock_splitBlock_internal` | `assert(zc->blockSizeMax <= ZSTD_BLOCKSIZE_MAX);` (c_src/src/compress/zstd_compress.c:4309) | assertion/abort | [ ] |
| 397 | `ZSTD_compressBlock_splitBlock_internal` | `assert(cSizeSingleBlock <= zc->blockSizeMax + ZSTD_blockHeaderSize);` (c_src/src/compress/zstd_compress.c:4310) | assertion/abort | [ ] |
| 398 | `ZSTD_compressBlock_splitBlock_internal` | `FORWARD_IF_ERROR(cSizeChunk, "Compressing chunk failed!");` (c_src/src/compress/zstd_compress.c:4337) | exact return/error shown | [ ] |
| 399 | `ZSTD_compressBlock_splitBlock_internal` | `assert(cSizeChunk <= zc->blockSizeMax + ZSTD_blockHeaderSize);` (c_src/src/compress/zstd_compress.c:4344) | assertion/abort | [ ] |
| 400 | `ZSTD_compressBlock_splitBlock` | `assert(zc->appliedParams.postBlockSplitter == ZSTD_ps_enable);` (c_src/src/compress/zstd_compress.c:4361) | assertion/abort | [ ] |
| 401 | `ZSTD_compressBlock_splitBlock` | `FORWARD_IF_ERROR(bss, "ZSTD_buildSeqStore failed");` (c_src/src/compress/zstd_compress.c:4364) | exact return/error shown | [ ] |
| 402 | `ZSTD_compressBlock_splitBlock` | `RETURN_ERROR_IF(zc->seqCollector.collectSequences, sequenceProducer_failed, "Uncompressible block");` (c_src/src/compress/zstd_compress.c:4368) | exact return/error shown | [ ] |
| 403 | `ZSTD_compressBlock_splitBlock` | `FORWARD_IF_ERROR(cSize, "ZSTD_noCompressBlock failed");` (c_src/src/compress/zstd_compress.c:4370) | exact return/error shown | [ ] |
| 404 | `ZSTD_compressBlock_splitBlock` | `FORWARD_IF_ERROR(cSize, "Splitting blocks failed!");` (c_src/src/compress/zstd_compress.c:4378) | exact return/error shown | [ ] |
| 405 | `ZSTD_compressBlock_internal` | `FORWARD_IF_ERROR(bss, "ZSTD_buildSeqStore failed");` (c_src/src/compress/zstd_compress.c:4400) | exact return/error shown | [ ] |
| 406 | `ZSTD_compressBlock_internal` | `RETURN_ERROR_IF(zc->seqCollector.collectSequences, sequenceProducer_failed, "Uncompressible block");` (c_src/src/compress/zstd_compress.c:4402) | exact return/error shown | [ ] |
| 407 | `ZSTD_compressBlock_internal` | `FORWARD_IF_ERROR(ZSTD_copyBlockSequences(&zc->seqCollector, ZSTD_getSeqStore(zc), zc->blockState.prevCBlock->rep), "copyBlockSequences failed");` (c_src/src/compress/zstd_compress.c:4409) | exact return/error shown | [ ] |
| 408 | `ZSTD_compressBlock_targetCBlockSize_body` | `FORWARD_IF_ERROR(cSize, "ZSTD_compressSuperBlock failed");` (c_src/src/compress/zstd_compress.c:4490) | exact return/error shown | [ ] |
| 409 | `ZSTD_compressBlock_targetCBlockSize` | `FORWARD_IF_ERROR(bss, "ZSTD_buildSeqStore failed");` (c_src/src/compress/zstd_compress.c:4515) | exact return/error shown | [ ] |
| 410 | `ZSTD_compressBlock_targetCBlockSize` | `FORWARD_IF_ERROR(cSize, "ZSTD_compressBlock_targetCBlockSize_body failed");` (c_src/src/compress/zstd_compress.c:4518) | exact return/error shown | [ ] |
| 411 | `ZSTD_overflowCorrectIfNeeded` | `ZSTD_STATIC_ASSERT(ZSTD_CHAINLOG_MAX <= 30);` (c_src/src/compress/zstd_compress.c:4536) | exact return/error shown | [ ] |
| 412 | `ZSTD_overflowCorrectIfNeeded` | `ZSTD_STATIC_ASSERT(ZSTD_WINDOWLOG_MAX_32 <= 30);` (c_src/src/compress/zstd_compress.c:4537) | exact return/error shown | [ ] |
| 413 | `ZSTD_overflowCorrectIfNeeded` | `ZSTD_STATIC_ASSERT(ZSTD_WINDOWLOG_MAX <= 31);` (c_src/src/compress/zstd_compress.c:4538) | exact return/error shown | [ ] |
| 414 | `ZSTD_optimalBlockSize` | `assert(ZSTD_fast <= strat && strat <= ZSTD_btultra2);` (c_src/src/compress/zstd_compress.c:4575) | assertion/abort | [ ] |
| 415 | `ZSTD_optimalBlockSize` | `assert(2 <= splitLevel && splitLevel <= 6);` (c_src/src/compress/zstd_compress.c:4578) | assertion/abort | [ ] |
| 416 | `ZSTD_compress_frameChunk` | `assert(cctx->appliedParams.cParams.windowLog <= ZSTD_WINDOWLOG_MAX);` (c_src/src/compress/zstd_compress.c:4604) | assertion/abort | [ ] |
| 417 | `ZSTD_compress_frameChunk` | `assert(blockSize <= remaining);` (c_src/src/compress/zstd_compress.c:4619) | assertion/abort | [ ] |
| 418 | `ZSTD_compress_frameChunk` | `RETURN_ERROR_IF(dstCapacity < ZSTD_blockHeaderSize + MIN_CBLOCK_SIZE + 1, dstSize_tooSmall, "not enough space to store compressed block");` (c_src/src/compress/zstd_compress.c:4623) | exact return/error shown | [ ] |
| 419 | `ZSTD_compress_frameChunk` | `FORWARD_IF_ERROR(cSize, "ZSTD_compressBlock_targetCBlockSize failed");` (c_src/src/compress/zstd_compress.c:4638) | exact return/error shown | [ ] |
| 420 | `ZSTD_compress_frameChunk` | `assert(cSize > 0);` (c_src/src/compress/zstd_compress.c:4639) | assertion/abort | [ ] |
| 421 | `ZSTD_compress_frameChunk` | `assert(cSize <= blockSize + ZSTD_blockHeaderSize);` (c_src/src/compress/zstd_compress.c:4640) | assertion/abort | [ ] |
| 422 | `ZSTD_compress_frameChunk` | `FORWARD_IF_ERROR(cSize, "ZSTD_compressBlock_splitBlock failed");` (c_src/src/compress/zstd_compress.c:4643) | exact return/error shown | [ ] |
| 423 | `ZSTD_compress_frameChunk` | `assert(cSize > 0 \|\| cctx->seqCollector.collectSequences == 1);` (c_src/src/compress/zstd_compress.c:4644) | assertion/abort | [ ] |
| 424 | `ZSTD_compress_frameChunk` | `FORWARD_IF_ERROR(cSize, "ZSTD_compressBlock_internal failed");` (c_src/src/compress/zstd_compress.c:4649) | exact return/error shown | [ ] |
| 425 | `ZSTD_compress_frameChunk` | `FORWARD_IF_ERROR(cSize, "ZSTD_noCompressBlock failed");` (c_src/src/compress/zstd_compress.c:4653) | exact return/error shown | [ ] |
| 426 | `ZSTD_compress_frameChunk` | `assert(remaining >= blockSize);` (c_src/src/compress/zstd_compress.c:4680) | assertion/abort | [ ] |
| 427 | `ZSTD_compress_frameChunk` | `assert(dstCapacity >= cSize);` (c_src/src/compress/zstd_compress.c:4683) | assertion/abort | [ ] |
| 428 | `ZSTD_writeFrameHeader` | `assert(!(params->fParams.contentSizeFlag && pledgedSrcSize == ZSTD_CONTENTSIZE_UNKNOWN));` (c_src/src/compress/zstd_compress.c:4711) | assertion/abort | [ ] |
| 429 | `ZSTD_writeFrameHeader` | `RETURN_ERROR_IF(dstCapacity < ZSTD_FRAMEHEADERSIZE_MAX, dstSize_tooSmall, "dst buf is too small to fit worst-case frame header size.");` (c_src/src/compress/zstd_compress.c:4712) | exact return/error shown | [ ] |
| 430 | `ZSTD_writeFrameHeader` | `assert(0); /* impossible */` (c_src/src/compress/zstd_compress.c:4725) | assertion/abort | [ ] |
| 431 | `ZSTD_writeFrameHeader` | `assert(0); /* impossible */` (c_src/src/compress/zstd_compress.c:4735) | assertion/abort | [ ] |
| 432 | `ZSTD_writeSkippableFrame` | `RETURN_ERROR_IF(dstCapacity < srcSize + ZSTD_SKIPPABLEHEADERSIZE /* Skippable frame overhead */, dstSize_tooSmall, "Not enough room for skippable frame");` (c_src/src/compress/zstd_compress.c:4754) | exact return/error shown | [ ] |
| 433 | `ZSTD_writeSkippableFrame` | `RETURN_ERROR_IF(srcSize > (unsigned)0xFFFFFFFF, srcSize_wrong, "Src size too large for skippable frame");` (c_src/src/compress/zstd_compress.c:4756) | exact return/error shown | [ ] |
| 434 | `ZSTD_writeSkippableFrame` | `RETURN_ERROR_IF(magicVariant > 15, parameter_outOfBound, "Skippable frame magic number variant not supported");` (c_src/src/compress/zstd_compress.c:4757) | exact return/error shown | [ ] |
| 435 | `ZSTD_writeLastEmptyBlock` | `RETURN_ERROR_IF(dstCapacity < ZSTD_blockHeaderSize, dstSize_tooSmall, "dst buf is too small to write frame trailer empty block.");` (c_src/src/compress/zstd_compress.c:4772) | exact return/error shown | [ ] |
| 436 | `ZSTD_referenceExternalSequences` | `assert(cctx->stage == ZSTDcs_init);` (c_src/src/compress/zstd_compress.c:4782) | assertion/abort | [ ] |
| 437 | `ZSTD_referenceExternalSequences` | `assert(nbSeq == 0 \|\| cctx->appliedParams.ldmParams.enableLdm != ZSTD_ps_enable);` (c_src/src/compress/zstd_compress.c:4783) | assertion/abort | [ ] |
| 438 | `ZSTD_compressContinue_internal` | `RETURN_ERROR_IF(cctx->stage==ZSTDcs_created, stage_wrong, "missing init (ZSTD_compressBegin)");` (c_src/src/compress/zstd_compress.c:4802) | exact return/error shown | [ ] |
| 439 | `ZSTD_compressContinue_internal` | `FORWARD_IF_ERROR(fhSize, "ZSTD_writeFrameHeader failed");` (c_src/src/compress/zstd_compress.c:4808) | exact return/error shown | [ ] |
| 440 | `ZSTD_compressContinue_internal` | `assert(fhSize <= dstCapacity);` (c_src/src/compress/zstd_compress.c:4809) | assertion/abort | [ ] |
| 441 | `ZSTD_compressContinue_internal` | `FORWARD_IF_ERROR(cSize, "%s", frame ? "ZSTD_compress_frameChunk failed" : "ZSTD_compressBlock_internal failed");` (c_src/src/compress/zstd_compress.c:4836) | exact return/error shown | [ ] |
| 442 | `ZSTD_compressContinue_internal` | `assert(!(cctx->appliedParams.fParams.contentSizeFlag && cctx->pledgedSrcSizePlusOne == 0));` (c_src/src/compress/zstd_compress.c:4839) | assertion/abort | [ ] |
| 443 | `ZSTD_compressContinue_internal` | `ZSTD_STATIC_ASSERT(ZSTD_CONTENTSIZE_UNKNOWN == (unsigned long long)-1);` (c_src/src/compress/zstd_compress.c:4841) | exact return/error shown | [ ] |
| 444 | `ZSTD_compressContinue_internal` | `RETURN_ERROR_IF( cctx->consumedSrcSize+1 > cctx->pledgedSrcSizePlusOne, srcSize_wrong, "error : pledgedSrcSize = %u, while realSrcSize >= %u", (unsigned)cctx->pledgedSrcSizePlusOne-1, (unsigned)cctx->consumedSrcSize);` (c_src/src/compress/zstd_compress.c:4842) | exact return/error shown | [ ] |
| 445 | `ZSTD_getBlockSize_deprecated` | `assert(!ZSTD_checkCParams(cParams));` (c_src/src/compress/zstd_compress.c:4872) | assertion/abort | [ ] |
| 446 | `ZSTD_compressBlock_deprecated` | `RETURN_ERROR_IF(srcSize > blockSizeMax, srcSize_wrong, "input is larger than a block"); }` (c_src/src/compress/zstd_compress.c:4887) | exact return/error shown | [ ] |
| 447 | `ZSTD_loadDictionaryContent` | `assert(!loadLdmDict);` (c_src/src/compress/zstd_compress.c:4934) | assertion/abort | [ ] |
| 448 | `ZSTD_loadDictionaryContent` | `assert(ZSTD_window_isEmpty(ms->window));` (c_src/src/compress/zstd_compress.c:4947) | assertion/abort | [ ] |
| 449 | `ZSTD_loadDictionaryContent` | `if (loadLdmDict) assert(ZSTD_window_isEmpty(ls->window));` (c_src/src/compress/zstd_compress.c:4948) | assertion/abort | [ ] |
| 450 | `ZSTD_loadDictionaryContent` | `assert(0); /* shouldn't be called: cparams should've been adjusted. */` (c_src/src/compress/zstd_compress.c:4988) | assertion/abort | [ ] |
| 451 | `ZSTD_loadDictionaryContent` | `assert(srcSize >= HASH_READ_SIZE);` (c_src/src/compress/zstd_compress.c:4998) | assertion/abort | [ ] |
| 452 | `ZSTD_loadDictionaryContent` | `assert(ms->chainTable != NULL);` (c_src/src/compress/zstd_compress.c:5000) | assertion/abort | [ ] |
| 453 | `ZSTD_loadDictionaryContent` | `assert(params->useRowMatchFinder != ZSTD_ps_auto);` (c_src/src/compress/zstd_compress.c:5003) | assertion/abort | [ ] |
| 454 | `ZSTD_loadDictionaryContent` | `assert(0); /* shouldn't be called: cparams should've been adjusted. */` (c_src/src/compress/zstd_compress.c:5015) | assertion/abort | [ ] |
| 455 | `ZSTD_loadDictionaryContent` | `assert(srcSize >= HASH_READ_SIZE);` (c_src/src/compress/zstd_compress.c:5026) | assertion/abort | [ ] |
| 456 | `ZSTD_loadDictionaryContent` | `assert(0); /* shouldn't be called: cparams should've been adjusted. */` (c_src/src/compress/zstd_compress.c:5030) | assertion/abort | [ ] |
| 457 | `ZSTD_loadDictionaryContent` | `assert(0); /* not possible : not a valid strategy id */` (c_src/src/compress/zstd_compress.c:5035) | assertion/abort | [ ] |
| 458 | `ZSTD_loadCEntropy` | `RETURN_ERROR_IF(HUF_isError(hufHeaderSize), dictionary_corrupted, "");` (c_src/src/compress/zstd_compress.c:5081) | exact return/error shown | [ ] |
| 459 | `ZSTD_loadCEntropy` | `RETURN_ERROR_IF(FSE_isError(offcodeHeaderSize), dictionary_corrupted, "");` (c_src/src/compress/zstd_compress.c:5087) | exact return/error shown | [ ] |
| 460 | `ZSTD_loadCEntropy` | `RETURN_ERROR_IF(offcodeLog > OffFSELog, dictionary_corrupted, "");` (c_src/src/compress/zstd_compress.c:5088) | exact return/error shown | [ ] |
| 461 | `ZSTD_loadCEntropy` | `RETURN_ERROR_IF(FSE_isError(FSE_buildCTable_wksp( bs->entropy.fse.offcodeCTable, offcodeNCount, MaxOff, offcodeLog, workspace, HUF_WORKSPACE_SIZE)), dictionary_corrupted, "");` (c_src/src/compress/zstd_compress.c:5090) | exact return/error shown | [ ] |
| 462 | `ZSTD_loadCEntropy` | `RETURN_ERROR_IF(FSE_isError(matchlengthHeaderSize), dictionary_corrupted, "");` (c_src/src/compress/zstd_compress.c:5102) | exact return/error shown | [ ] |
| 463 | `ZSTD_loadCEntropy` | `RETURN_ERROR_IF(matchlengthLog > MLFSELog, dictionary_corrupted, "");` (c_src/src/compress/zstd_compress.c:5103) | exact return/error shown | [ ] |
| 464 | `ZSTD_loadCEntropy` | `RETURN_ERROR_IF(FSE_isError(FSE_buildCTable_wksp( bs->entropy.fse.matchlengthCTable, matchlengthNCount, matchlengthMaxValue, matchlengthLog, workspace, HUF_WORKSPACE_SIZE)), dictionary_corrupted, "");` (c_src/src/compress/zstd_compress.c:5104) | exact return/error shown | [ ] |
| 465 | `ZSTD_loadCEntropy` | `RETURN_ERROR_IF(FSE_isError(litlengthHeaderSize), dictionary_corrupted, "");` (c_src/src/compress/zstd_compress.c:5116) | exact return/error shown | [ ] |
| 466 | `ZSTD_loadCEntropy` | `RETURN_ERROR_IF(litlengthLog > LLFSELog, dictionary_corrupted, "");` (c_src/src/compress/zstd_compress.c:5117) | exact return/error shown | [ ] |
| 467 | `ZSTD_loadCEntropy` | `RETURN_ERROR_IF(FSE_isError(FSE_buildCTable_wksp( bs->entropy.fse.litlengthCTable, litlengthNCount, litlengthMaxValue, litlengthLog, workspace, HUF_WORKSPACE_SIZE)), dictionary_corrupted, "");` (c_src/src/compress/zstd_compress.c:5118) | exact return/error shown | [ ] |
| 468 | `ZSTD_loadCEntropy` | `RETURN_ERROR_IF(dictPtr+12 > dictEnd, dictionary_corrupted, "");` (c_src/src/compress/zstd_compress.c:5127) | exact return/error shown | [ ] |
| 469 | `ZSTD_loadCEntropy` | `RETURN_ERROR_IF(bs->rep[u] == 0, dictionary_corrupted, "");` (c_src/src/compress/zstd_compress.c:5145) | exact return/error shown | [ ] |
| 470 | `ZSTD_loadCEntropy` | `RETURN_ERROR_IF(bs->rep[u] > dictContentSize, dictionary_corrupted, "");` (c_src/src/compress/zstd_compress.c:5146) | exact return/error shown | [ ] |
| 471 | `ZSTD_loadZstdDictionary` | `ZSTD_STATIC_ASSERT(HUF_WORKSPACE_SIZE >= (1<<MAX(MLFSELog,LLFSELog)));` (c_src/src/compress/zstd_compress.c:5174) | exact return/error shown | [ ] |
| 472 | `ZSTD_loadZstdDictionary` | `assert(dictSize >= 8);` (c_src/src/compress/zstd_compress.c:5175) | assertion/abort | [ ] |
| 473 | `ZSTD_loadZstdDictionary` | `assert(MEM_readLE32(dictPtr) == ZSTD_MAGIC_DICTIONARY);` (c_src/src/compress/zstd_compress.c:5176) | assertion/abort | [ ] |
| 474 | `ZSTD_loadZstdDictionary` | `FORWARD_IF_ERROR(eSize, "ZSTD_loadCEntropy failed");` (c_src/src/compress/zstd_compress.c:5180) | exact return/error shown | [ ] |
| 475 | `ZSTD_loadZstdDictionary` | `FORWARD_IF_ERROR(ZSTD_loadDictionaryContent( ms, NULL, ws, params, dictPtr, dictContentSize, dtlm, tfp), "");` (c_src/src/compress/zstd_compress.c:5185) | exact return/error shown | [ ] |
| 476 | `ZSTD_compress_insertDictionary` | `RETURN_ERROR_IF(dictContentType == ZSTD_dct_fullDict, dictionary_wrong, "");` (c_src/src/compress/zstd_compress.c:5207) | exact return/error shown | [ ] |
| 477 | `ZSTD_compress_insertDictionary` | `RETURN_ERROR_IF(dictContentType == ZSTD_dct_fullDict, dictionary_wrong, "");` (c_src/src/compress/zstd_compress.c:5223) | exact return/error shown | [ ] |
| 478 | `ZSTD_compress_insertDictionary` | `assert(0); /* impossible */` (c_src/src/compress/zstd_compress.c:5224) | assertion/abort | [ ] |
| 479 | `ZSTD_compressBegin_internal` | `assert(!ZSTD_isError(ZSTD_checkCParams(params->cParams)));` (c_src/src/compress/zstd_compress.c:5252) | assertion/abort | [ ] |
| 480 | `ZSTD_compressBegin_internal` | `assert(!((dict) && (cdict))); /* either dict or cdict, not both */` (c_src/src/compress/zstd_compress.c:5253) | assertion/abort | [ ] |
| 481 | `ZSTD_compressBegin_internal` | `FORWARD_IF_ERROR( ZSTD_resetCCtx_internal(cctx, params, pledgedSrcSize, dictContentSize, ZSTDcrp_makeClean, zbuff) , "");` (c_src/src/compress/zstd_compress.c:5264) | exact return/error shown | [ ] |
| 482 | `ZSTD_compressBegin_internal` | `FORWARD_IF_ERROR(dictID, "ZSTD_compress_insertDictionary failed");` (c_src/src/compress/zstd_compress.c:5277) | exact return/error shown | [ ] |
| 483 | `ZSTD_compressBegin_internal` | `assert(dictID <= UINT_MAX);` (c_src/src/compress/zstd_compress.c:5278) | assertion/abort | [ ] |
| 484 | `ZSTD_compressBegin_advanced_internal` | `FORWARD_IF_ERROR( ZSTD_checkCParams(params->cParams) , "");` (c_src/src/compress/zstd_compress.c:5295) | exact return/error shown | [ ] |
| 485 | `ZSTD_writeEpilogue` | `RETURN_ERROR_IF(cctx->stage == ZSTDcs_created, stage_wrong, "init missing");` (c_src/src/compress/zstd_compress.c:5350) | exact return/error shown | [ ] |
| 486 | `ZSTD_writeEpilogue` | `FORWARD_IF_ERROR(fhSize, "ZSTD_writeFrameHeader failed");` (c_src/src/compress/zstd_compress.c:5355) | exact return/error shown | [ ] |
| 487 | `ZSTD_writeEpilogue` | `ZSTD_STATIC_ASSERT(ZSTD_BLOCKHEADERSIZE == 3);` (c_src/src/compress/zstd_compress.c:5364) | exact return/error shown | [ ] |
| 488 | `ZSTD_writeEpilogue` | `RETURN_ERROR_IF(dstCapacity<3, dstSize_tooSmall, "no room for epilogue");` (c_src/src/compress/zstd_compress.c:5365) | exact return/error shown | [ ] |
| 489 | `ZSTD_writeEpilogue` | `RETURN_ERROR_IF(dstCapacity<4, dstSize_tooSmall, "no room for checksum");` (c_src/src/compress/zstd_compress.c:5373) | exact return/error shown | [ ] |
| 490 | `ZSTD_compressEnd_public` | `FORWARD_IF_ERROR(cSize, "ZSTD_compressContinue_internal failed");` (c_src/src/compress/zstd_compress.c:5415) | exact return/error shown | [ ] |
| 491 | `ZSTD_compressEnd_public` | `FORWARD_IF_ERROR(endResult, "ZSTD_writeEpilogue failed");` (c_src/src/compress/zstd_compress.c:5417) | exact return/error shown | [ ] |
| 492 | `ZSTD_compressEnd_public` | `assert(!(cctx->appliedParams.fParams.contentSizeFlag && cctx->pledgedSrcSizePlusOne == 0));` (c_src/src/compress/zstd_compress.c:5418) | assertion/abort | [ ] |
| 493 | `ZSTD_compressEnd_public` | `ZSTD_STATIC_ASSERT(ZSTD_CONTENTSIZE_UNKNOWN == (unsigned long long)-1);` (c_src/src/compress/zstd_compress.c:5420) | exact return/error shown | [ ] |
| 494 | `ZSTD_compressEnd_public` | `RETURN_ERROR_IF( cctx->pledgedSrcSizePlusOne != cctx->consumedSrcSize+1, srcSize_wrong, "error : pledgedSrcSize = %u, while realSrcSize = %u", (unsigned)cctx->pledgedSrcSizePlusOne-1, (unsigned)cctx->consumedSrcSize);` (c_src/src/compress/zstd_compress.c:5422) | exact return/error shown | [ ] |
| 495 | `ZSTD_compress_advanced` | `FORWARD_IF_ERROR(ZSTD_checkCParams(params.cParams), "");` (c_src/src/compress/zstd_compress.c:5448) | exact return/error shown | [ ] |
| 496 | `ZSTD_compress_advanced_internal` | `FORWARD_IF_ERROR( ZSTD_compressBegin_internal(cctx, dict, dictSize, ZSTD_dct_auto, ZSTD_dtlm_fast, NULL, params, srcSize, ZSTDb_not_buffered) , "");` (c_src/src/compress/zstd_compress.c:5466) | exact return/error shown | [ ] |
| 497 | `ZSTD_compress_usingDict` | `assert(params.fParams.contentSizeFlag == 1);` (c_src/src/compress/zstd_compress.c:5480) | assertion/abort | [ ] |
| 498 | `ZSTD_compressCCtx` | `assert(cctx != NULL);` (c_src/src/compress/zstd_compress.c:5493) | assertion/abort | [ ] |
| 499 | `ZSTD_compress` | `RETURN_ERROR_IF(!cctx, memory_allocation, "ZSTD_createCCtx failed");` (c_src/src/compress/zstd_compress.c:5504) | exact return/error shown | [ ] |
| 500 | `ZSTD_initCDict_internal` | `assert(!ZSTD_checkCParams(params.cParams));` (c_src/src/compress/zstd_compress.c:5559) | assertion/abort | [ ] |
| 501 | `ZSTD_initCDict_internal` | `RETURN_ERROR_IF(!internalBuffer, memory_allocation, "NULL pointer!");` (c_src/src/compress/zstd_compress.c:5566) | exact return/error shown | [ ] |
| 502 | `ZSTD_initCDict_internal` | `FORWARD_IF_ERROR(ZSTD_reset_matchState( &cdict->matchState, &cdict->workspace, &params.cParams, params.useRowMatchFinder, ZSTDcrp_makeClean, ZSTDirp_reset, ZSTD_resetTarget_CDict), "");` (c_src/src/compress/zstd_compress.c:5578) | exact return/error shown | [ ] |
| 503 | `ZSTD_initCDict_internal` | `FORWARD_IF_ERROR(dictID, "ZSTD_compress_insertDictionary failed");` (c_src/src/compress/zstd_compress.c:5595) | exact return/error shown | [ ] |
| 504 | `ZSTD_initCDict_internal` | `assert(dictID <= (size_t)(U32)-1);` (c_src/src/compress/zstd_compress.c:5596) | assertion/abort | [ ] |
| 505 | `ZSTD_createCDict_advanced_internal` | `if ((!customMem.customAlloc) ^ (!customMem.customFree)) return NULL;` (c_src/src/compress/zstd_compress.c:5612) | exact return/error shown | [ ] |
| 506 | `ZSTD_createCDict_advanced_internal` | `return NULL;` (c_src/src/compress/zstd_compress.c:5627) | exact return/error shown | [ ] |
| 507 | `ZSTD_createCDict_advanced_internal` | `assert(cdict != NULL);` (c_src/src/compress/zstd_compress.c:5633) | assertion/abort | [ ] |
| 508 | `ZSTD_createCDict_advanced2` | `if (!customMem.customAlloc ^ !customMem.customFree) return NULL;` (c_src/src/compress/zstd_compress.c:5672) | exact return/error shown | [ ] |
| 509 | `ZSTD_createCDict_advanced2` | `return NULL;` (c_src/src/compress/zstd_compress.c:5704) | exact return/error shown | [ ] |
| 510 | `ZSTD_initStaticCDict` | `if ((size_t)workspace & 7) return NULL; /* 8-aligned */` (c_src/src/compress/zstd_compress.c:5777) | exact return/error shown | [ ] |
| 511 | `ZSTD_initStaticCDict` | `if (cdict == NULL) return NULL;` (c_src/src/compress/zstd_compress.c:5783) | exact return/error shown | [ ] |
| 512 | `ZSTD_initStaticCDict` | `if (workspaceSize < neededSize) return NULL;` (c_src/src/compress/zstd_compress.c:5787) | exact return/error shown | [ ] |
| 513 | `ZSTD_initStaticCDict` | `return NULL;` (c_src/src/compress/zstd_compress.c:5799) | exact return/error shown | [ ] |
| 514 | `ZSTD_getCParamsFromCDict` | `assert(cdict != NULL);` (c_src/src/compress/zstd_compress.c:5806) | assertion/abort | [ ] |
| 515 | `ZSTD_compressBegin_usingCDict_internal` | `RETURN_ERROR_IF(cdict==NULL, dictionary_wrong, "NULL pointer!");` (c_src/src/compress/zstd_compress.c:5829) | exact return/error shown | [ ] |
| 516 | `ZSTD_compress_usingCDict_internal` | `FORWARD_IF_ERROR(ZSTD_compressBegin_usingCDict_internal(cctx, cdict, fParams, srcSize), ""); /* will check if cdict != NULL */` (c_src/src/compress/zstd_compress.c:5892) | exact return/error shown | [ ] |
| 517 | `ZSTD_resetCStream` | `FORWARD_IF_ERROR( ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only) , "");` (c_src/src/compress/zstd_compress.c:5977) | exact return/error shown | [ ] |
| 518 | `ZSTD_resetCStream` | `FORWARD_IF_ERROR( ZSTD_CCtx_setPledgedSrcSize(zcs, pledgedSrcSize) , "");` (c_src/src/compress/zstd_compress.c:5978) | exact return/error shown | [ ] |
| 519 | `ZSTD_initCStream_internal` | `FORWARD_IF_ERROR( ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only) , "");` (c_src/src/compress/zstd_compress.c:5992) | exact return/error shown | [ ] |
| 520 | `ZSTD_initCStream_internal` | `FORWARD_IF_ERROR( ZSTD_CCtx_setPledgedSrcSize(zcs, pledgedSrcSize) , "");` (c_src/src/compress/zstd_compress.c:5993) | exact return/error shown | [ ] |
| 521 | `ZSTD_initCStream_internal` | `assert(!ZSTD_isError(ZSTD_checkCParams(params->cParams)));` (c_src/src/compress/zstd_compress.c:5994) | assertion/abort | [ ] |
| 522 | `ZSTD_initCStream_internal` | `assert(!((dict) && (cdict))); /* either dict or cdict, not both */` (c_src/src/compress/zstd_compress.c:5996) | assertion/abort | [ ] |
| 523 | `ZSTD_initCStream_internal` | `FORWARD_IF_ERROR( ZSTD_CCtx_loadDictionary(zcs, dict, dictSize) , "");` (c_src/src/compress/zstd_compress.c:5998) | exact return/error shown | [ ] |
| 524 | `ZSTD_initCStream_internal` | `FORWARD_IF_ERROR( ZSTD_CCtx_refCDict(zcs, cdict) , "");` (c_src/src/compress/zstd_compress.c:6001) | exact return/error shown | [ ] |
| 525 | `ZSTD_initCStream_usingCDict_advanced` | `FORWARD_IF_ERROR( ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only) , "");` (c_src/src/compress/zstd_compress.c:6014) | exact return/error shown | [ ] |
| 526 | `ZSTD_initCStream_usingCDict_advanced` | `FORWARD_IF_ERROR( ZSTD_CCtx_setPledgedSrcSize(zcs, pledgedSrcSize) , "");` (c_src/src/compress/zstd_compress.c:6015) | exact return/error shown | [ ] |
| 527 | `ZSTD_initCStream_usingCDict_advanced` | `FORWARD_IF_ERROR( ZSTD_CCtx_refCDict(zcs, cdict) , "");` (c_src/src/compress/zstd_compress.c:6017) | exact return/error shown | [ ] |
| 528 | `ZSTD_initCStream_usingCDict` | `FORWARD_IF_ERROR( ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only) , "");` (c_src/src/compress/zstd_compress.c:6025) | exact return/error shown | [ ] |
| 529 | `ZSTD_initCStream_usingCDict` | `FORWARD_IF_ERROR( ZSTD_CCtx_refCDict(zcs, cdict) , "");` (c_src/src/compress/zstd_compress.c:6026) | exact return/error shown | [ ] |
| 530 | `ZSTD_initCStream_advanced` | `FORWARD_IF_ERROR( ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only) , "");` (c_src/src/compress/zstd_compress.c:6045) | exact return/error shown | [ ] |
| 531 | `ZSTD_initCStream_advanced` | `FORWARD_IF_ERROR( ZSTD_CCtx_setPledgedSrcSize(zcs, pledgedSrcSize) , "");` (c_src/src/compress/zstd_compress.c:6046) | exact return/error shown | [ ] |
| 532 | `ZSTD_initCStream_advanced` | `FORWARD_IF_ERROR( ZSTD_checkCParams(params.cParams) , "");` (c_src/src/compress/zstd_compress.c:6047) | exact return/error shown | [ ] |
| 533 | `ZSTD_initCStream_advanced` | `FORWARD_IF_ERROR( ZSTD_CCtx_loadDictionary(zcs, dict, dictSize) , "");` (c_src/src/compress/zstd_compress.c:6049) | exact return/error shown | [ ] |
| 534 | `ZSTD_initCStream_usingDict` | `FORWARD_IF_ERROR( ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only) , "");` (c_src/src/compress/zstd_compress.c:6056) | exact return/error shown | [ ] |
| 535 | `ZSTD_initCStream_usingDict` | `FORWARD_IF_ERROR( ZSTD_CCtx_setParameter(zcs, ZSTD_c_compressionLevel, compressionLevel) , "");` (c_src/src/compress/zstd_compress.c:6057) | exact return/error shown | [ ] |
| 536 | `ZSTD_initCStream_usingDict` | `FORWARD_IF_ERROR( ZSTD_CCtx_loadDictionary(zcs, dict, dictSize) , "");` (c_src/src/compress/zstd_compress.c:6058) | exact return/error shown | [ ] |
| 537 | `ZSTD_initCStream_srcSize` | `FORWARD_IF_ERROR( ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only) , "");` (c_src/src/compress/zstd_compress.c:6070) | exact return/error shown | [ ] |
| 538 | `ZSTD_initCStream_srcSize` | `FORWARD_IF_ERROR( ZSTD_CCtx_refCDict(zcs, NULL) , "");` (c_src/src/compress/zstd_compress.c:6071) | exact return/error shown | [ ] |
| 539 | `ZSTD_initCStream_srcSize` | `FORWARD_IF_ERROR( ZSTD_CCtx_setParameter(zcs, ZSTD_c_compressionLevel, compressionLevel) , "");` (c_src/src/compress/zstd_compress.c:6072) | exact return/error shown | [ ] |
| 540 | `ZSTD_initCStream_srcSize` | `FORWARD_IF_ERROR( ZSTD_CCtx_setPledgedSrcSize(zcs, pledgedSrcSize) , "");` (c_src/src/compress/zstd_compress.c:6073) | exact return/error shown | [ ] |
| 541 | `ZSTD_initCStream` | `FORWARD_IF_ERROR( ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only) , "");` (c_src/src/compress/zstd_compress.c:6080) | exact return/error shown | [ ] |
| 542 | `ZSTD_initCStream` | `FORWARD_IF_ERROR( ZSTD_CCtx_refCDict(zcs, NULL) , "");` (c_src/src/compress/zstd_compress.c:6081) | exact return/error shown | [ ] |
| 543 | `ZSTD_initCStream` | `FORWARD_IF_ERROR( ZSTD_CCtx_setParameter(zcs, ZSTD_c_compressionLevel, compressionLevel) , "");` (c_src/src/compress/zstd_compress.c:6082) | exact return/error shown | [ ] |
| 544 | `ZSTD_nextInputSizeHint` | `assert(cctx->appliedParams.inBufferMode == ZSTD_bm_buffered);` (c_src/src/compress/zstd_compress.c:6093) | assertion/abort | [ ] |
| 545 | `ZSTD_compressStream_generic` | `const char* const istart = (assert(input != NULL), (const char*)input->src);` (c_src/src/compress/zstd_compress.c:6108) | assertion/abort | [ ] |
| 546 | `ZSTD_compressStream_generic` | `char* const ostart = (assert(output != NULL), (char*)output->dst);` (c_src/src/compress/zstd_compress.c:6111) | assertion/abort | [ ] |
| 547 | `ZSTD_compressStream_generic` | `assert(zcs != NULL);` (c_src/src/compress/zstd_compress.c:6118) | assertion/abort | [ ] |
| 548 | `ZSTD_compressStream_generic` | `assert(input->pos >= zcs->stableIn_notConsumed);` (c_src/src/compress/zstd_compress.c:6120) | assertion/abort | [ ] |
| 549 | `ZSTD_compressStream_generic` | `assert(zcs->inBuff != NULL);` (c_src/src/compress/zstd_compress.c:6126) | assertion/abort | [ ] |
| 550 | `ZSTD_compressStream_generic` | `assert(zcs->inBuffSize > 0);` (c_src/src/compress/zstd_compress.c:6127) | assertion/abort | [ ] |
| 551 | `ZSTD_compressStream_generic` | `assert(zcs->outBuff != NULL);` (c_src/src/compress/zstd_compress.c:6130) | assertion/abort | [ ] |
| 552 | `ZSTD_compressStream_generic` | `assert(zcs->outBuffSize > 0);` (c_src/src/compress/zstd_compress.c:6131) | assertion/abort | [ ] |
| 553 | `ZSTD_compressStream_generic` | `if (input->src == NULL) assert(input->size == 0);` (c_src/src/compress/zstd_compress.c:6133) | assertion/abort | [ ] |
| 554 | `ZSTD_compressStream_generic` | `assert(input->pos <= input->size);` (c_src/src/compress/zstd_compress.c:6134) | assertion/abort | [ ] |
| 555 | `ZSTD_compressStream_generic` | `if (output->dst == NULL) assert(output->size == 0);` (c_src/src/compress/zstd_compress.c:6135) | assertion/abort | [ ] |
| 556 | `ZSTD_compressStream_generic` | `assert(output->pos <= output->size);` (c_src/src/compress/zstd_compress.c:6136) | assertion/abort | [ ] |
| 557 | `ZSTD_compressStream_generic` | `assert((U32)flushMode <= (U32)ZSTD_e_end);` (c_src/src/compress/zstd_compress.c:6137) | assertion/abort | [ ] |
| 558 | `ZSTD_compressStream_generic` | `RETURN_ERROR(init_missing, "call ZSTD_initCStream() first!");` (c_src/src/compress/zstd_compress.c:6143) | exact return/error shown | [ ] |
| 559 | `ZSTD_compressStream_generic` | `FORWARD_IF_ERROR(cSize, "ZSTD_compressEnd failed");` (c_src/src/compress/zstd_compress.c:6155) | exact return/error shown | [ ] |
| 560 | `ZSTD_compressStream_generic` | `assert(zcs->appliedParams.inBufferMode == ZSTD_bm_stable);` (c_src/src/compress/zstd_compress.c:6181) | assertion/abort | [ ] |
| 561 | `ZSTD_compressStream_generic` | `FORWARD_IF_ERROR(cSize, "%s", lastBlock ? "ZSTD_compressEnd failed" : "ZSTD_compressContinue failed");` (c_src/src/compress/zstd_compress.c:6214) | exact return/error shown | [ ] |
| 562 | `ZSTD_compressStream_generic` | `assert(zcs->inBuffTarget <= zcs->inBuffSize);` (c_src/src/compress/zstd_compress.c:6223) | assertion/abort | [ ] |
| 563 | `ZSTD_compressStream_generic` | `FORWARD_IF_ERROR(cSize, "%s", lastBlock ? "ZSTD_compressEnd failed" : "ZSTD_compressContinue failed");` (c_src/src/compress/zstd_compress.c:6232) | exact return/error shown | [ ] |
| 564 | `ZSTD_compressStream_generic` | `if (lastBlock) assert(ip == iend);` (c_src/src/compress/zstd_compress.c:6234) | assertion/abort | [ ] |
| 565 | `ZSTD_compressStream_generic` | `assert(zcs->appliedParams.outBufferMode == ZSTD_bm_buffered);` (c_src/src/compress/zstd_compress.c:6252) | assertion/abort | [ ] |
| 566 | `ZSTD_compressStream_generic` | `assert(op==oend);` (c_src/src/compress/zstd_compress.c:6263) | assertion/abort | [ ] |
| 567 | `ZSTD_compressStream_generic` | `assert(0);` (c_src/src/compress/zstd_compress.c:6279) | assertion/abort | [ ] |
| 568 | `ZSTD_nextInputSizeHint_MTorST` | `assert(cctx->mtctx != NULL);` (c_src/src/compress/zstd_compress.c:6293) | assertion/abort | [ ] |
| 569 | `ZSTD_compressStream` | `FORWARD_IF_ERROR( ZSTD_compressStream2(zcs, output, input, ZSTD_e_continue) , "");` (c_src/src/compress/zstd_compress.c:6303) | exact return/error shown | [ ] |
| 570 | `ZSTD_checkBufferStability` | `RETURN_ERROR(stabilityCondition_notRespected, "ZSTD_c_stableInBuffer enabled but input differs!");` (c_src/src/compress/zstd_compress.c:6333) | exact return/error shown | [ ] |
| 571 | `ZSTD_checkBufferStability` | `RETURN_ERROR(stabilityCondition_notRespected, "ZSTD_c_stableOutBuffer enabled but output size differs!");` (c_src/src/compress/zstd_compress.c:6339) | exact return/error shown | [ ] |
| 572 | `ZSTD_CCtx_init_compressStream2` | `FORWARD_IF_ERROR( ZSTD_initLocalDict(cctx) , ""); /* Init the local dict if present. */` (c_src/src/compress/zstd_compress.c:6355) | exact return/error shown | [ ] |
| 573 | `ZSTD_CCtx_init_compressStream2` | `assert(prefixDict.dict==NULL \|\| cctx->cdict==NULL); /* only one can be set */` (c_src/src/compress/zstd_compress.c:6357) | assertion/abort | [ ] |
| 574 | `ZSTD_CCtx_init_compressStream2` | `RETURN_ERROR_IF( ZSTD_hasExtSeqProd(&params) && params.nbWorkers >= 1, parameter_combination_unsupported, "External sequence producer isn't supported with nbWorkers >= 1" );` (c_src/src/compress/zstd_compress.c:6386) | exact return/error shown | [ ] |
| 575 | `ZSTD_CCtx_init_compressStream2` | `RETURN_ERROR_IF(cctx->mtctx == NULL, memory_allocation, "NULL pointer!");` (c_src/src/compress/zstd_compress.c:6404) | exact return/error shown | [ ] |
| 576 | `ZSTD_CCtx_init_compressStream2` | `FORWARD_IF_ERROR( ZSTDMT_initCStream_internal( cctx->mtctx, prefixDict.dict, prefixDict.dictSize, prefixDict.dictContentType, cctx->cdict, params, cctx->pledgedSrcSizePlusOne-1) , "");` (c_src/src/compress/zstd_compress.c:6408) | exact return/error shown | [ ] |
| 577 | `ZSTD_CCtx_init_compressStream2` | `assert(!ZSTD_isError(ZSTD_checkCParams(params.cParams)));` (c_src/src/compress/zstd_compress.c:6421) | assertion/abort | [ ] |
| 578 | `ZSTD_CCtx_init_compressStream2` | `FORWARD_IF_ERROR( ZSTD_compressBegin_internal(cctx, prefixDict.dict, prefixDict.dictSize, prefixDict.dictContentType, ZSTD_dtlm_fast, cctx->cdict, &params, pledgedSrcSize, ZSTDb_buffered) , "");` (c_src/src/compress/zstd_compress.c:6422) | exact return/error shown | [ ] |
| 579 | `ZSTD_CCtx_init_compressStream2` | `assert(cctx->appliedParams.nbWorkers == 0);` (c_src/src/compress/zstd_compress.c:6427) | assertion/abort | [ ] |
| 580 | `ZSTD_compressStream2` | `RETURN_ERROR_IF(output->pos > output->size, dstSize_tooSmall, "invalid output buffer");` (c_src/src/compress/zstd_compress.c:6454) | exact return/error shown | [ ] |
| 581 | `ZSTD_compressStream2` | `RETURN_ERROR_IF(input->pos > input->size, srcSize_wrong, "invalid input buffer");` (c_src/src/compress/zstd_compress.c:6455) | exact return/error shown | [ ] |
| 582 | `ZSTD_compressStream2` | `RETURN_ERROR_IF((U32)endOp > (U32)ZSTD_e_end, parameter_outOfBound, "invalid endDirective");` (c_src/src/compress/zstd_compress.c:6456) | exact return/error shown | [ ] |
| 583 | `ZSTD_compressStream2` | `assert(cctx != NULL);` (c_src/src/compress/zstd_compress.c:6457) | assertion/abort | [ ] |
| 584 | `ZSTD_compressStream2` | `RETURN_ERROR_IF(input->src != cctx->expectedInBuffer.src, stabilityCondition_notRespected, "stableInBuffer condition not respected: wrong src pointer");` (c_src/src/compress/zstd_compress.c:6468) | exact return/error shown | [ ] |
| 585 | `ZSTD_compressStream2` | `RETURN_ERROR_IF(input->pos != cctx->expectedInBuffer.size, stabilityCondition_notRespected, "stableInBuffer condition not respected: externally modified pos");` (c_src/src/compress/zstd_compress.c:6469) | exact return/error shown | [ ] |
| 586 | `ZSTD_compressStream2` | `FORWARD_IF_ERROR(ZSTD_CCtx_init_compressStream2(cctx, endOp, totalInputSize), "compressStream2 initialization failed");` (c_src/src/compress/zstd_compress.c:6480) | exact return/error shown | [ ] |
| 587 | `ZSTD_compressStream2` | `FORWARD_IF_ERROR(ZSTD_checkBufferStability(cctx, output, input, endOp), "invalid buffers");` (c_src/src/compress/zstd_compress.c:6485) | exact return/error shown | [ ] |
| 588 | `ZSTD_compressStream2` | `assert(cctx->appliedParams.inBufferMode == ZSTD_bm_stable);` (c_src/src/compress/zstd_compress.c:6495) | assertion/abort | [ ] |
| 589 | `ZSTD_compressStream2` | `assert(input->pos >= cctx->stableIn_notConsumed);` (c_src/src/compress/zstd_compress.c:6497) | assertion/abort | [ ] |
| 590 | `ZSTD_compressStream2` | `FORWARD_IF_ERROR(flushMin, "ZSTDMT_compressStream_generic failed");` (c_src/src/compress/zstd_compress.c:6513) | exact return/error shown | [ ] |
| 591 | `ZSTD_compressStream2` | `assert(endOp == ZSTD_e_flush \|\| endOp == ZSTD_e_end);` (c_src/src/compress/zstd_compress.c:6523) | assertion/abort | [ ] |
| 592 | `ZSTD_compressStream2` | `assert(endOp == ZSTD_e_continue \|\| flushMin == 0 \|\| output->pos == output->size);` (c_src/src/compress/zstd_compress.c:6535) | assertion/abort | [ ] |
| 593 | `ZSTD_compressStream2` | `FORWARD_IF_ERROR( ZSTD_compressStream_generic(cctx, output, input, endOp) , "");` (c_src/src/compress/zstd_compress.c:6540) | exact return/error shown | [ ] |
| 594 | `ZSTD_compress2` | `FORWARD_IF_ERROR(result, "ZSTD_compressStream2_simpleArgs failed");` (c_src/src/compress/zstd_compress.c:6589) | exact return/error shown | [ ] |
| 595 | `ZSTD_compress2` | `assert(oPos == dstCapacity);` (c_src/src/compress/zstd_compress.c:6591) | assertion/abort | [ ] |
| 596 | `ZSTD_compress2` | `RETURN_ERROR(dstSize_tooSmall, "");` (c_src/src/compress/zstd_compress.c:6592) | exact return/error shown | [ ] |
| 597 | `ZSTD_compress2` | `assert(iPos == srcSize); /* all input is expected consumed */` (c_src/src/compress/zstd_compress.c:6594) | assertion/abort | [ ] |
| 598 | `ZSTD_validateSequence` | `RETURN_ERROR_IF(offBase > OFFSET_TO_OFFBASE(offsetBound), externalSequences_invalid, "Offset too large!");` (c_src/src/compress/zstd_compress.c:6615) | exact return/error shown | [ ] |
| 599 | `ZSTD_validateSequence` | `RETURN_ERROR_IF(matchLength < matchLenLowerBound, externalSequences_invalid, "Matchlength too small for the minMatch");` (c_src/src/compress/zstd_compress.c:6617) | exact return/error shown | [ ] |
| 600 | `ZSTD_transferSequences_wBlockDelim` | `FORWARD_IF_ERROR(ZSTD_validateSequence(offBase, matchLength, cctx->appliedParams.cParams.minMatch, seqPos->posInSrc, cctx->appliedParams.cParams.windowLog, dictSize, ZSTD_hasExtSeqProd(&cctx->appliedParams)), "Sequence validation failed");` (c_src/src/compress/zstd_compress.c:6684) | exact return/error shown | [ ] |
| 601 | `ZSTD_transferSequences_wBlockDelim` | `RETURN_ERROR_IF(idx - seqPos->idx >= cctx->seqStore.maxNbSeq, externalSequences_invalid, "Not enough memory allocated. Try adjusting ZSTD_c_minMatch.");` (c_src/src/compress/zstd_compress.c:6690) | exact return/error shown | [ ] |
| 602 | `ZSTD_transferSequences_wBlockDelim` | `RETURN_ERROR_IF(idx == inSeqsSize, externalSequences_invalid, "Block delimiter not found.");` (c_src/src/compress/zstd_compress.c:6695) | exact return/error shown | [ ] |
| 603 | `ZSTD_transferSequences_wBlockDelim` | `assert(externalRepSearch != ZSTD_ps_auto);` (c_src/src/compress/zstd_compress.c:6698) | assertion/abort | [ ] |
| 604 | `ZSTD_transferSequences_wBlockDelim` | `assert(idx >= startIdx);` (c_src/src/compress/zstd_compress.c:6699) | assertion/abort | [ ] |
| 605 | `ZSTD_transferSequences_wBlockDelim` | `assert(lastSeqIdx == startIdx);` (c_src/src/compress/zstd_compress.c:6713) | assertion/abort | [ ] |
| 606 | `ZSTD_transferSequences_wBlockDelim` | `RETURN_ERROR_IF(ip != iend, externalSequences_invalid, "Blocksize doesn't agree with block delimiter!");` (c_src/src/compress/zstd_compress.c:6728) | exact return/error shown | [ ] |
| 607 | `ZSTD_transferSequences_noDelim` | `FORWARD_IF_ERROR(ZSTD_validateSequence(offBase, matchLength, cctx->appliedParams.cParams.minMatch, seqPos->posInSrc, cctx->appliedParams.cParams.windowLog, dictSize, ZSTD_hasExtSeqProd(&cctx->appliedParams)), "Sequence validation failed");` (c_src/src/compress/zstd_compress.c:6839) | exact return/error shown | [ ] |
| 608 | `ZSTD_transferSequences_noDelim` | `RETURN_ERROR_IF(idx - seqPos->idx >= cctx->seqStore.maxNbSeq, externalSequences_invalid, "Not enough memory allocated. Try adjusting ZSTD_c_minMatch.");` (c_src/src/compress/zstd_compress.c:6844) | exact return/error shown | [ ] |
| 609 | `ZSTD_transferSequences_noDelim` | `assert(idx == inSeqsSize \|\| endPosInSequence <= inSeqs[idx].litLength + inSeqs[idx].matchLength);` (c_src/src/compress/zstd_compress.c:6852) | assertion/abort | [ ] |
| 610 | `ZSTD_transferSequences_noDelim` | `assert(ip <= iend);` (c_src/src/compress/zstd_compress.c:6861) | assertion/abort | [ ] |
| 611 | `ZSTD_selectSequenceCopier` | `assert(ZSTD_cParam_withinBounds(ZSTD_c_blockDelimiters, (int)mode));` (c_src/src/compress/zstd_compress.c:6883) | assertion/abort | [ ] |
| 612 | `ZSTD_selectSequenceCopier` | `assert(mode == ZSTD_sf_noBlockDelimiters);` (c_src/src/compress/zstd_compress.c:6887) | assertion/abort | [ ] |
| 613 | `blockSize_explicitDelimiter` | `assert(spos <= inSeqsSize);` (c_src/src/compress/zstd_compress.c:6902) | assertion/abort | [ ] |
| 614 | `blockSize_explicitDelimiter` | `RETURN_ERROR(externalSequences_invalid, "delimiter format error : both matchlength and offset must be == 0");` (c_src/src/compress/zstd_compress.c:6908) | exact return/error shown | [ ] |
| 615 | `blockSize_explicitDelimiter` | `RETURN_ERROR(externalSequences_invalid, "Reached end of sequences without finding a block delimiter");` (c_src/src/compress/zstd_compress.c:6914) | exact return/error shown | [ ] |
| 616 | `determine_blockSize` | `assert(mode == ZSTD_sf_explicitBlockDelimiters);` (c_src/src/compress/zstd_compress.c:6928) | assertion/abort | [ ] |
| 617 | `determine_blockSize` | `FORWARD_IF_ERROR(explicitBlockSize, "Error while determining block size with explicit delimiters");` (c_src/src/compress/zstd_compress.c:6930) | exact return/error shown | [ ] |
| 618 | `determine_blockSize` | `RETURN_ERROR(externalSequences_invalid, "sequences incorrectly define a too large block");` (c_src/src/compress/zstd_compress.c:6932) | exact return/error shown | [ ] |
| 619 | `determine_blockSize` | `RETURN_ERROR(externalSequences_invalid, "sequences define a frame longer than source");` (c_src/src/compress/zstd_compress.c:6934) | exact return/error shown | [ ] |
| 620 | `ZSTD_compressSequences_internal` | `RETURN_ERROR_IF(dstCapacity<4, dstSize_tooSmall, "No room for empty frame block header");` (c_src/src/compress/zstd_compress.c:6962) | exact return/error shown | [ ] |
| 621 | `ZSTD_compressSequences_internal` | `FORWARD_IF_ERROR(blockSize, "Error while trying to determine block size");` (c_src/src/compress/zstd_compress.c:6976) | exact return/error shown | [ ] |
| 622 | `ZSTD_compressSequences_internal` | `assert(blockSize <= remaining);` (c_src/src/compress/zstd_compress.c:6977) | assertion/abort | [ ] |
| 623 | `ZSTD_compressSequences_internal` | `FORWARD_IF_ERROR(blockSize, "Bad sequence copy");` (c_src/src/compress/zstd_compress.c:6984) | exact return/error shown | [ ] |
| 624 | `ZSTD_compressSequences_internal` | `FORWARD_IF_ERROR(cBlockSize, "Nocompress block failed");` (c_src/src/compress/zstd_compress.c:6991) | exact return/error shown | [ ] |
| 625 | `ZSTD_compressSequences_internal` | `RETURN_ERROR_IF(dstCapacity < ZSTD_blockHeaderSize, dstSize_tooSmall, "not enough dstCapacity to write a new compressed block");` (c_src/src/compress/zstd_compress.c:7001) | exact return/error shown | [ ] |
| 626 | `ZSTD_compressSequences_internal` | `FORWARD_IF_ERROR(compressedSeqsSize, "Compressing sequences of block failed");` (c_src/src/compress/zstd_compress.c:7009) | exact return/error shown | [ ] |
| 627 | `ZSTD_compressSequences_internal` | `FORWARD_IF_ERROR(cBlockSize, "ZSTD_noCompressBlock failed");` (c_src/src/compress/zstd_compress.c:7025) | exact return/error shown | [ ] |
| 628 | `ZSTD_compressSequences_internal` | `FORWARD_IF_ERROR(cBlockSize, "ZSTD_rleCompressBlock failed");` (c_src/src/compress/zstd_compress.c:7029) | exact return/error shown | [ ] |
| 629 | `ZSTD_compressSequences` | `assert(cctx != NULL);` (c_src/src/compress/zstd_compress.c:7073) | assertion/abort | [ ] |
| 630 | `ZSTD_compressSequences` | `FORWARD_IF_ERROR(ZSTD_CCtx_init_compressStream2(cctx, ZSTD_e_end, srcSize), "CCtx initialization failed");` (c_src/src/compress/zstd_compress.c:7074) | exact return/error shown | [ ] |
| 631 | `ZSTD_compressSequences` | `assert(frameHeaderSize <= dstCapacity);` (c_src/src/compress/zstd_compress.c:7080) | assertion/abort | [ ] |
| 632 | `ZSTD_compressSequences` | `FORWARD_IF_ERROR(cBlocksSize, "Compressing blocks failed!");` (c_src/src/compress/zstd_compress.c:7093) | exact return/error shown | [ ] |
| 633 | `ZSTD_compressSequences` | `assert(cBlocksSize <= dstCapacity);` (c_src/src/compress/zstd_compress.c:7095) | assertion/abort | [ ] |
| 634 | `ZSTD_compressSequences` | `RETURN_ERROR_IF(dstCapacity<4, dstSize_tooSmall, "no room for checksum");` (c_src/src/compress/zstd_compress.c:7102) | exact return/error shown | [ ] |
| 635 | `convertSequences_noRepcodes` | `ZSTD_STATIC_ASSERT(sizeof(ZSTD_Sequence) == 16);` (c_src/src/compress/zstd_compress.c:7187) | exact return/error shown | [ ] |
| 636 | `convertSequences_noRepcodes` | `ZSTD_STATIC_ASSERT(offsetof(ZSTD_Sequence, offset) == 0);` (c_src/src/compress/zstd_compress.c:7188) | exact return/error shown | [ ] |
| 637 | `convertSequences_noRepcodes` | `ZSTD_STATIC_ASSERT(offsetof(ZSTD_Sequence, litLength) == 4);` (c_src/src/compress/zstd_compress.c:7189) | exact return/error shown | [ ] |
| 638 | `convertSequences_noRepcodes` | `ZSTD_STATIC_ASSERT(offsetof(ZSTD_Sequence, matchLength) == 8);` (c_src/src/compress/zstd_compress.c:7190) | exact return/error shown | [ ] |
| 639 | `convertSequences_noRepcodes` | `ZSTD_STATIC_ASSERT(sizeof(SeqDef) == 8);` (c_src/src/compress/zstd_compress.c:7191) | exact return/error shown | [ ] |
| 640 | `convertSequences_noRepcodes` | `ZSTD_STATIC_ASSERT(offsetof(SeqDef, offBase) == 0);` (c_src/src/compress/zstd_compress.c:7192) | exact return/error shown | [ ] |
| 641 | `convertSequences_noRepcodes` | `ZSTD_STATIC_ASSERT(offsetof(SeqDef, litLength) == 4);` (c_src/src/compress/zstd_compress.c:7193) | exact return/error shown | [ ] |
| 642 | `convertSequences_noRepcodes` | `ZSTD_STATIC_ASSERT(offsetof(SeqDef, mlBase) == 6);` (c_src/src/compress/zstd_compress.c:7194) | exact return/error shown | [ ] |
| 643 | `convertSequences_noRepcodes` | `assert(longLen == 0);` (c_src/src/compress/zstd_compress.c:7240) | assertion/abort | [ ] |
| 644 | `convertSequences_noRepcodes` | `assert(longLen == 0);` (c_src/src/compress/zstd_compress.c:7244) | assertion/abort | [ ] |
| 645 | `convertSequences_noRepcodes` | `assert(longLen == 0);` (c_src/src/compress/zstd_compress.c:7248) | assertion/abort | [ ] |
| 646 | `convertSequences_noRepcodes` | `assert(longLen == 0);` (c_src/src/compress/zstd_compress.c:7252) | assertion/abort | [ ] |
| 647 | `convertSequences_noRepcodes` | `assert(i == nbSequences - 1);` (c_src/src/compress/zstd_compress.c:7261) | assertion/abort | [ ] |
| 648 | `convertSequences_noRepcodes` | `assert(longLen == 0);` (c_src/src/compress/zstd_compress.c:7267) | assertion/abort | [ ] |
| 649 | `convertSequences_noRepcodes` | `assert(longLen == 0);` (c_src/src/compress/zstd_compress.c:7271) | assertion/abort | [ ] |
| 650 | `convertSequences_noRepcodes` | `assert(longLen == 0);` (c_src/src/compress/zstd_compress.c:7298) | assertion/abort | [ ] |
| 651 | `convertSequences_noRepcodes` | `assert(longLen == 0);` (c_src/src/compress/zstd_compress.c:7302) | assertion/abort | [ ] |
| 652 | `ZSTD_convertBlockSequences` | `RETURN_ERROR_IF(nbSequences >= cctx->seqStore.maxNbSeq, externalSequences_invalid, "Not enough memory allocated. Try adjusting ZSTD_c_minMatch.");` (c_src/src/compress/zstd_compress.c:7327) | exact return/error shown | [ ] |
| 653 | `ZSTD_convertBlockSequences` | `assert(nbSequences >= 1);` (c_src/src/compress/zstd_compress.c:7333) | assertion/abort | [ ] |
| 654 | `ZSTD_convertBlockSequences` | `assert(inSeqs[nbSequences-1].matchLength == 0);` (c_src/src/compress/zstd_compress.c:7334) | assertion/abort | [ ] |
| 655 | `ZSTD_convertBlockSequences` | `assert(inSeqs[nbSequences-1].offset == 0);` (c_src/src/compress/zstd_compress.c:7335) | assertion/abort | [ ] |
| 656 | `ZSTD_convertBlockSequences` | `assert(cctx->seqStore.longLengthType == ZSTD_llt_none);` (c_src/src/compress/zstd_compress.c:7343) | assertion/abort | [ ] |
| 657 | `ZSTD_convertBlockSequences` | `assert(longl <= 2* (nbSequences-1));` (c_src/src/compress/zstd_compress.c:7350) | assertion/abort | [ ] |
| 658 | `ZSTD_convertBlockSequences` | `assert(nbSequences == 2);` (c_src/src/compress/zstd_compress.c:7382) | assertion/abort | [ ] |
| 659 | `ZSTD_get1BlockSummary` | `ZSTD_STATIC_ASSERT(sizeof(ZSTD_Sequence) == 16);` (c_src/src/compress/zstd_compress.c:7403) | exact return/error shown | [ ] |
| 660 | `ZSTD_get1BlockSummary` | `ZSTD_STATIC_ASSERT(offsetof(ZSTD_Sequence, matchLength) == 8);` (c_src/src/compress/zstd_compress.c:7414) | exact return/error shown | [ ] |
| 661 | `ZSTD_get1BlockSummary` | `assert(seqs);` (c_src/src/compress/zstd_compress.c:7453) | assertion/abort | [ ] |
| 662 | `ZSTD_get1BlockSummary` | `assert(seqs[n].offset == 0);` (c_src/src/compress/zstd_compress.c:7458) | assertion/abort | [ ] |
| 663 | `ZSTD_compressSequencesAndLiterals_internal` | `assert(cctx->appliedParams.searchForExternalRepcodes != ZSTD_ps_auto);` (c_src/src/compress/zstd_compress.c:7487) | assertion/abort | [ ] |
| 664 | `ZSTD_compressSequencesAndLiterals_internal` | `RETURN_ERROR_IF(nbSequences == 0, externalSequences_invalid, "Requires at least 1 end-of-block");` (c_src/src/compress/zstd_compress.c:7490) | exact return/error shown | [ ] |
| 665 | `ZSTD_compressSequencesAndLiterals_internal` | `RETURN_ERROR_IF(dstCapacity<3, dstSize_tooSmall, "No room for empty frame block header");` (c_src/src/compress/zstd_compress.c:7495) | exact return/error shown | [ ] |
| 666 | `ZSTD_compressSequencesAndLiterals_internal` | `FORWARD_IF_ERROR(block.nbSequences, "Error while trying to determine nb of sequences for a block");` (c_src/src/compress/zstd_compress.c:7506) | exact return/error shown | [ ] |
| 667 | `ZSTD_compressSequencesAndLiterals_internal` | `assert(block.nbSequences <= nbSequences);` (c_src/src/compress/zstd_compress.c:7507) | assertion/abort | [ ] |
| 668 | `ZSTD_compressSequencesAndLiterals_internal` | `RETURN_ERROR_IF(block.litSize > litSize, externalSequences_invalid, "discrepancy: Sequences require more literals than present in buffer");` (c_src/src/compress/zstd_compress.c:7508) | exact return/error shown | [ ] |
| 669 | `ZSTD_compressSequencesAndLiterals_internal` | `FORWARD_IF_ERROR(conversionStatus, "Bad sequence conversion");` (c_src/src/compress/zstd_compress.c:7514) | exact return/error shown | [ ] |
| 670 | `ZSTD_compressSequencesAndLiterals_internal` | `RETURN_ERROR_IF(dstCapacity < ZSTD_blockHeaderSize, dstSize_tooSmall, "not enough dstCapacity to write a new compressed block");` (c_src/src/compress/zstd_compress.c:7524) | exact return/error shown | [ ] |
| 671 | `ZSTD_compressSequencesAndLiterals_internal` | `FORWARD_IF_ERROR(compressedSeqsSize, "Compressing sequences of block failed");` (c_src/src/compress/zstd_compress.c:7534) | exact return/error shown | [ ] |
| 672 | `ZSTD_compressSequencesAndLiterals_internal` | `RETURN_ERROR(cannotProduce_uncompressedBlock, "ZSTD_compressSequencesAndLiterals cannot generate an uncompressed block");` (c_src/src/compress/zstd_compress.c:7550) | exact return/error shown | [ ] |
| 673 | `ZSTD_compressSequencesAndLiterals_internal` | `assert(compressedSeqsSize > 1); /* no RLE */` (c_src/src/compress/zstd_compress.c:7553) | assertion/abort | [ ] |
| 674 | `ZSTD_compressSequencesAndLiterals_internal` | `assert(nbSequences == 0);` (c_src/src/compress/zstd_compress.c:7573) | assertion/abort | [ ] |
| 675 | `ZSTD_compressSequencesAndLiterals_internal` | `RETURN_ERROR_IF(litSize != 0, externalSequences_invalid, "literals must be entirely and exactly consumed");` (c_src/src/compress/zstd_compress.c:7578) | exact return/error shown | [ ] |
| 676 | `ZSTD_compressSequencesAndLiterals_internal` | `RETURN_ERROR_IF(remaining != 0, externalSequences_invalid, "Sequences must represent a total of exactly srcSize=%zu", srcSize);` (c_src/src/compress/zstd_compress.c:7579) | exact return/error shown | [ ] |
| 677 | `ZSTD_compressSequencesAndLiterals` | `assert(cctx != NULL);` (c_src/src/compress/zstd_compress.c:7596) | assertion/abort | [ ] |
| 678 | `ZSTD_compressSequencesAndLiterals` | `RETURN_ERROR(workSpace_tooSmall, "literals buffer is not large enough: must be at least 8 bytes larger than litSize (risk of read out-of-bound)");` (c_src/src/compress/zstd_compress.c:7598) | exact return/error shown | [ ] |
| 679 | `ZSTD_compressSequencesAndLiterals` | `FORWARD_IF_ERROR(ZSTD_CCtx_init_compressStream2(cctx, ZSTD_e_end, decompressedSize), "CCtx initialization failed");` (c_src/src/compress/zstd_compress.c:7600) | exact return/error shown | [ ] |
| 680 | `ZSTD_compressSequencesAndLiterals` | `RETURN_ERROR(frameParameter_unsupported, "This mode is only compatible with explicit delimiters");` (c_src/src/compress/zstd_compress.c:7603) | exact return/error shown | [ ] |
| 681 | `ZSTD_compressSequencesAndLiterals` | `RETURN_ERROR(parameter_unsupported, "This mode is not compatible with Sequence validation");` (c_src/src/compress/zstd_compress.c:7606) | exact return/error shown | [ ] |
| 682 | `ZSTD_compressSequencesAndLiterals` | `RETURN_ERROR(frameParameter_unsupported, "this mode is not compatible with frame checksum");` (c_src/src/compress/zstd_compress.c:7609) | exact return/error shown | [ ] |
| 683 | `ZSTD_compressSequencesAndLiterals` | `assert(frameHeaderSize <= dstCapacity);` (c_src/src/compress/zstd_compress.c:7616) | assertion/abort | [ ] |
| 684 | `ZSTD_compressSequencesAndLiterals` | `FORWARD_IF_ERROR(cBlocksSize, "Compressing blocks failed!");` (c_src/src/compress/zstd_compress.c:7626) | exact return/error shown | [ ] |
| 685 | `ZSTD_compressSequencesAndLiterals` | `assert(cBlocksSize <= dstCapacity);` (c_src/src/compress/zstd_compress.c:7628) | assertion/abort | [ ] |
| 686 | `ZSTD_endStream` | `FORWARD_IF_ERROR(remainingToFlush , "ZSTD_compressStream2(,,ZSTD_e_end) failed");` (c_src/src/compress/zstd_compress.c:7658) | exact return/error shown | [ ] |
| 687 | `ZSTD_getCParamRowSize` | `assert(0);` (c_src/src/compress/zstd_compress.c:7745) | assertion/abort | [ ] |
| 688 | `ZSTD_registerSequenceProducer` | `assert(zc != NULL);` (c_src/src/compress/zstd_compress.c:7824) | assertion/abort | [ ] |
| 689 | `ZSTD_CCtxParams_registerSequenceProducer` | `assert(params != NULL);` (c_src/src/compress/zstd_compress.c:7835) | assertion/abort | [ ] |
| 690 | `ZSTD_noCompressLiterals` | `RETURN_ERROR_IF(srcSize + flSize > dstCapacity, dstSize_tooSmall, "");` (c_src/src/compress/zstd_compress_literals.c:46) | exact return/error shown | [ ] |
| 691 | `ZSTD_noCompressLiterals` | `assert(0);` (c_src/src/compress/zstd_compress_literals.c:60) | assertion/abort | [ ] |
| 692 | `allBytesIdentical` | `assert(srcSize >= 1);` (c_src/src/compress/zstd_compress_literals.c:70) | assertion/abort | [ ] |
| 693 | `allBytesIdentical` | `assert(src != NULL);` (c_src/src/compress/zstd_compress_literals.c:71) | assertion/abort | [ ] |
| 694 | `ZSTD_compressRleLiteralsBlock` | `assert(dstCapacity >= 4); (void)dstCapacity;` (c_src/src/compress/zstd_compress_literals.c:86) | assertion/abort | [ ] |
| 695 | `ZSTD_compressRleLiteralsBlock` | `assert(allBytesIdentical(src, srcSize));` (c_src/src/compress/zstd_compress_literals.c:87) | assertion/abort | [ ] |
| 696 | `ZSTD_compressRleLiteralsBlock` | `assert(0);` (c_src/src/compress/zstd_compress_literals.c:101) | assertion/abort | [ ] |
| 697 | `ZSTD_minLiteralsToCompress` | `assert((int)strategy >= 0);` (c_src/src/compress/zstd_compress_literals.c:117) | assertion/abort | [ ] |
| 698 | `ZSTD_minLiteralsToCompress` | `assert((int)strategy <= 9);` (c_src/src/compress/zstd_compress_literals.c:118) | assertion/abort | [ ] |
| 699 | `ZSTD_compressLiterals` | `RETURN_ERROR_IF(dstCapacity < lhSize+1, dstSize_tooSmall, "not enough space for compression");` (c_src/src/compress/zstd_compress_literals.c:161) | exact return/error shown | [ ] |
| 700 | `ZSTD_compressLiterals` | `if (!singleStream) assert(srcSize >= MIN_LITERALS_FOR_4_STREAMS);` (c_src/src/compress/zstd_compress_literals.c:212) | assertion/abort | [ ] |
| 701 | `ZSTD_compressLiterals` | `assert(srcSize >= MIN_LITERALS_FOR_4_STREAMS);` (c_src/src/compress/zstd_compress_literals.c:218) | assertion/abort | [ ] |
| 702 | `ZSTD_compressLiterals` | `assert(srcSize >= MIN_LITERALS_FOR_4_STREAMS);` (c_src/src/compress/zstd_compress_literals.c:224) | assertion/abort | [ ] |
| 703 | `ZSTD_compressLiterals` | `assert(0);` (c_src/src/compress/zstd_compress_literals.c:231) | assertion/abort | [ ] |
| 704 | `ZSTD_NCountCost` | `FORWARD_IF_ERROR(FSE_normalizeCount(norm, tableLog, count, nbSeq, max, ZSTD_useLowProbCount(nbSeq)), "");` (c_src/src/compress/zstd_compress_sequences.c:76) | exact return/error shown | [ ] |
| 705 | `ZSTD_entropyCost` | `assert(total > 0);` (c_src/src/compress/zstd_compress_sequences.c:89) | assertion/abort | [ ] |
| 706 | `ZSTD_entropyCost` | `assert(count[s] < total);` (c_src/src/compress/zstd_compress_sequences.c:94) | assertion/abort | [ ] |
| 707 | `ZSTD_fseBitCost` | `return ERROR(GENERIC);` (c_src/src/compress/zstd_compress_sequences.c:117) | exact return/error shown | [ ] |
| 708 | `ZSTD_fseBitCost` | `return ERROR(GENERIC);` (c_src/src/compress/zstd_compress_sequences.c:127) | exact return/error shown | [ ] |
| 709 | `ZSTD_crossEntropyCost` | `assert(accuracyLog <= 8);` (c_src/src/compress/zstd_compress_sequences.c:145) | assertion/abort | [ ] |
| 710 | `ZSTD_crossEntropyCost` | `assert(norm256 > 0);` (c_src/src/compress/zstd_compress_sequences.c:149) | assertion/abort | [ ] |
| 711 | `ZSTD_crossEntropyCost` | `assert(norm256 < 256);` (c_src/src/compress/zstd_compress_sequences.c:150) | assertion/abort | [ ] |
| 712 | `ZSTD_selectEncodingType` | `ZSTD_STATIC_ASSERT(ZSTD_defaultDisallowed == 0 && ZSTD_defaultAllowed != 0);` (c_src/src/compress/zstd_compress_sequences.c:165) | exact return/error shown | [ ] |
| 713 | `ZSTD_selectEncodingType` | `assert(defaultNormLog >= 5 && defaultNormLog <= 6); /* xx_DEFAULTNORMLOG */` (c_src/src/compress/zstd_compress_sequences.c:185) | assertion/abort | [ ] |
| 714 | `ZSTD_selectEncodingType` | `assert(mult <= 9 && mult >= 7);` (c_src/src/compress/zstd_compress_sequences.c:186) | assertion/abort | [ ] |
| 715 | `ZSTD_selectEncodingType` | `assert(!ZSTD_isError(basicCost));` (c_src/src/compress/zstd_compress_sequences.c:212) | assertion/abort | [ ] |
| 716 | `ZSTD_selectEncodingType` | `assert(!(*repeatMode == FSE_repeat_valid && ZSTD_isError(repeatCost)));` (c_src/src/compress/zstd_compress_sequences.c:213) | assertion/abort | [ ] |
| 717 | `ZSTD_selectEncodingType` | `assert(!ZSTD_isError(NCountCost));` (c_src/src/compress/zstd_compress_sequences.c:215) | assertion/abort | [ ] |
| 718 | `ZSTD_selectEncodingType` | `assert(compressedCost < ERROR(maxCode));` (c_src/src/compress/zstd_compress_sequences.c:216) | assertion/abort | [ ] |
| 719 | `ZSTD_selectEncodingType` | `assert(isDefaultAllowed);` (c_src/src/compress/zstd_compress_sequences.c:221) | assertion/abort | [ ] |
| 720 | `ZSTD_selectEncodingType` | `assert(!ZSTD_isError(repeatCost));` (c_src/src/compress/zstd_compress_sequences.c:227) | assertion/abort | [ ] |
| 721 | `ZSTD_selectEncodingType` | `assert(compressedCost < basicCost && compressedCost < repeatCost);` (c_src/src/compress/zstd_compress_sequences.c:230) | assertion/abort | [ ] |
| 722 | `ZSTD_buildCTable` | `FORWARD_IF_ERROR(FSE_buildCTable_rle(nextCTable, (BYTE)max), "");` (c_src/src/compress/zstd_compress_sequences.c:257) | exact return/error shown | [ ] |
| 723 | `ZSTD_buildCTable` | `RETURN_ERROR_IF(dstCapacity==0, dstSize_tooSmall, "not enough space");` (c_src/src/compress/zstd_compress_sequences.c:258) | exact return/error shown | [ ] |
| 724 | `ZSTD_buildCTable` | `FORWARD_IF_ERROR(FSE_buildCTable_wksp(nextCTable, defaultNorm, defaultMax, defaultNormLog, entropyWorkspace, entropyWorkspaceSize), ""); /* note : could be pre-calculated */` (c_src/src/compress/zstd_compress_sequences.c:265) | exact return/error shown | [ ] |
| 725 | `ZSTD_buildCTable` | `assert(nbSeq_1 > 1);` (c_src/src/compress/zstd_compress_sequences.c:275) | assertion/abort | [ ] |
| 726 | `ZSTD_buildCTable` | `assert(entropyWorkspaceSize >= sizeof(ZSTD_BuildCTableWksp));` (c_src/src/compress/zstd_compress_sequences.c:276) | assertion/abort | [ ] |
| 727 | `ZSTD_buildCTable` | `FORWARD_IF_ERROR(FSE_normalizeCount(wksp->norm, tableLog, count, nbSeq_1, max, ZSTD_useLowProbCount(nbSeq_1)), "FSE_normalizeCount failed");` (c_src/src/compress/zstd_compress_sequences.c:278) | exact return/error shown | [ ] |
| 728 | `ZSTD_buildCTable` | `assert(oend >= op);` (c_src/src/compress/zstd_compress_sequences.c:279) | assertion/abort | [ ] |
| 729 | `ZSTD_buildCTable` | `FORWARD_IF_ERROR(NCountSize, "FSE_writeNCount failed");` (c_src/src/compress/zstd_compress_sequences.c:281) | exact return/error shown | [ ] |
| 730 | `ZSTD_buildCTable` | `FORWARD_IF_ERROR(FSE_buildCTable_wksp(nextCTable, wksp->norm, max, tableLog, wksp->wksp, sizeof(wksp->wksp)), "FSE_buildCTable_wksp failed");` (c_src/src/compress/zstd_compress_sequences.c:282) | exact return/error shown | [ ] |
| 731 | `ZSTD_buildCTable` | `default: assert(0); RETURN_ERROR(GENERIC, "impossible to reach");` (c_src/src/compress/zstd_compress_sequences.c:286) | assertion/abort | [ ] |
| 732 | `ZSTD_encodeSequences_body` | `RETURN_ERROR_IF( ERR_isError(BIT_initCStream(&blockStream, dst, dstCapacity)), dstSize_tooSmall, "not enough space remaining");` (c_src/src/compress/zstd_compress_sequences.c:303) | exact return/error shown | [ ] |
| 733 | `ZSTD_encodeSequences_body` | `RETURN_ERROR_IF(streamSize==0, dstSize_tooSmall, "not enough space");` (c_src/src/compress/zstd_compress_sequences.c:379) | exact return/error shown | [ ] |
| 734 | `ZSTD_compressSubBlock_literal` | `assert(litSize > 0);` (c_src/src/compress/zstd_compress_superblock.c:68) | assertion/abort | [ ] |
| 735 | `ZSTD_compressSubBlock_literal` | `assert(hufMetadata->hType == set_compressed \|\| hufMetadata->hType == set_repeat);` (c_src/src/compress/zstd_compress_superblock.c:69) | assertion/abort | [ ] |
| 736 | `ZSTD_compressSubBlock_literal` | `assert(cLitSize > litSize);` (c_src/src/compress/zstd_compress_superblock.c:94) | assertion/abort | [ ] |
| 737 | `ZSTD_compressSubBlock_literal` | `assert(0);` (c_src/src/compress/zstd_compress_superblock.c:121) | assertion/abort | [ ] |
| 738 | `ZSTD_seqDecompressedSize` | `assert(litLengthSum == litSize);` (c_src/src/compress/zstd_compress_superblock.c:145) | assertion/abort | [ ] |
| 739 | `ZSTD_seqDecompressedSize` | `assert(litLengthSum <= litSize);` (c_src/src/compress/zstd_compress_superblock.c:147) | assertion/abort | [ ] |
| 740 | `ZSTD_compressSubBlock_sequences` | `RETURN_ERROR_IF((oend-op) < 3 /*max nbSeq Size*/ + 1 /*seqHead*/, dstSize_tooSmall, "");` (c_src/src/compress/zstd_compress_superblock.c:181) | exact return/error shown | [ ] |
| 741 | `ZSTD_compressSubBlock_sequences` | `FORWARD_IF_ERROR(bitstreamSize, "ZSTD_encodeSequences failed");` (c_src/src/compress/zstd_compress_superblock.c:218) | exact return/error shown | [ ] |
| 742 | `ZSTD_compressSubBlock_sequences` | `assert(fseMetadata->lastCountSize + bitstreamSize == 3);` (c_src/src/compress/zstd_compress_superblock.c:231) | assertion/abort | [ ] |
| 743 | `ZSTD_compressSubBlock` | `FORWARD_IF_ERROR(cLitSize, "ZSTD_compressSubBlock_literal failed");` (c_src/src/compress/zstd_compress_superblock.c:284) | exact return/error shown | [ ] |
| 744 | `ZSTD_compressSubBlock` | `FORWARD_IF_ERROR(cSeqSize, "ZSTD_compressSubBlock_sequences failed");` (c_src/src/compress/zstd_compress_superblock.c:295) | exact return/error shown | [ ] |
| 745 | `ZSTD_estimateSubBlockSize_literal` | `assert(0); /* impossible */` (c_src/src/compress/zstd_compress_superblock.c:326) | assertion/abort | [ ] |
| 746 | `ZSTD_estimateSubBlockSize_symbolType` | `assert(max <= defaultMax);` (c_src/src/compress/zstd_compress_superblock.c:347) | assertion/abort | [ ] |
| 747 | `countLiterals` | `assert(sp != NULL);` (c_src/src/compress/zstd_compress_superblock.c:432) | assertion/abort | [ ] |
| 748 | `sizeBlockSequences` | `assert(firstSubBlock==0 \|\| firstSubBlock==1);` (c_src/src/compress/zstd_compress_superblock.c:449) | assertion/abort | [ ] |
| 749 | `ZSTD_compressSubBlock_multi` | `assert(nbSubBlocks>0);` (c_src/src/compress/zstd_compress_superblock.c:535) | assertion/abort | [ ] |
| 750 | `ZSTD_compressSubBlock_multi` | `assert(seqCount <= (size_t)(send-sp));` (c_src/src/compress/zstd_compress_superblock.c:541) | assertion/abort | [ ] |
| 751 | `ZSTD_compressSubBlock_multi` | `assert(seqCount > 0);` (c_src/src/compress/zstd_compress_superblock.c:543) | assertion/abort | [ ] |
| 752 | `ZSTD_compressSubBlock_multi` | `FORWARD_IF_ERROR(cSize, "ZSTD_compressSubBlock failed");` (c_src/src/compress/zstd_compress_superblock.c:559) | exact return/error shown | [ ] |
| 753 | `ZSTD_compressSubBlock_multi` | `assert(ip + decompressedSize <= iend);` (c_src/src/compress/zstd_compress_superblock.c:565) | assertion/abort | [ ] |
| 754 | `ZSTD_compressSubBlock_multi` | `FORWARD_IF_ERROR(cSize, "ZSTD_compressSubBlock failed");` (c_src/src/compress/zstd_compress_superblock.c:603) | exact return/error shown | [ ] |
| 755 | `ZSTD_compressSubBlock_multi` | `assert(ip + decompressedSize <= iend);` (c_src/src/compress/zstd_compress_superblock.c:609) | assertion/abort | [ ] |
| 756 | `ZSTD_compressSubBlock_multi` | `FORWARD_IF_ERROR(cSize, "ZSTD_noCompressBlock failed");` (c_src/src/compress/zstd_compress_superblock.c:645) | exact return/error shown | [ ] |
| 757 | `ZSTD_compressSubBlock_multi` | `assert(cSize != 0);` (c_src/src/compress/zstd_compress_superblock.c:646) | assertion/abort | [ ] |
| 758 | `ZSTD_compressSuperBlock` | `FORWARD_IF_ERROR(ZSTD_buildBlockEntropyStats(&zc->seqStore, &zc->blockState.prevCBlock->entropy, &zc->blockState.nextCBlock->entropy, &zc->appliedParams, &entropyMetadata, zc->tmpWorkspace, zc->tmpWkspSize /* statically allocated in resetCCtx */), "");` (c_src/src/compress/zstd_compress_superblock.c:672) | exact return/error shown | [ ] |
| 759 | `ZSTD_compressBlock_doubleFast_dictMatchState_generic` | `assert(ms->window.dictLimit + (1U << cParams->windowLog) >= endIndex);` (c_src/src/compress/zstd_double_fast.c:366) | assertion/abort | [ ] |
| 760 | `ZSTD_compressBlock_doubleFast_dictMatchState_generic` | `assert(offset_1 <= dictAndPrefixLength);` (c_src/src/compress/zstd_double_fast.c:380) | assertion/abort | [ ] |
| 761 | `ZSTD_compressBlock_doubleFast_dictMatchState_generic` | `assert(offset_2 <= dictAndPrefixLength);` (c_src/src/compress/zstd_double_fast.c:381) | assertion/abort | [ ] |
| 762 | `ZSTD_compressBlock_doubleFast_dictMatchState_generic` | `assert(dictMatchL < dictEnd);` (c_src/src/compress/zstd_double_fast.c:426) | assertion/abort | [ ] |
| 763 | `ZSTD_compressBlock_doubleFast_dictMatchState_generic` | `assert(dictMatchL3 < dictEnd);` (c_src/src/compress/zstd_double_fast.c:476) | assertion/abort | [ ] |
| 764 | `ZSTD_fillHashTableForCDict` | `assert(dtlm == ZSTD_dtlm_full);` (c_src/src/compress/zstd_fast.c:31) | assertion/abort | [ ] |
| 765 | `ZSTD_fillHashTableForCCtx` | `assert(dtlm == ZSTD_dtlm_fast);` (c_src/src/compress/zstd_fast.c:68) | assertion/abort | [ ] |
| 766 | `ZSTD_compressBlock_fast_noDict_generic` | `assert(base+current0+2 > istart); /* check base overflow */` (c_src/src/compress/zstd_fast.c:406) | assertion/abort | [ ] |
| 767 | `ZSTD_compressBlock_fast` | `assert(ms->dictMatchState == NULL);` (c_src/src/compress/zstd_fast.c:450) | assertion/abort | [ ] |
| 768 | `ZSTD_compressBlock_fast_dictMatchState_generic` | `assert(endIndex - prefixStartIndex <= maxDistance);` (c_src/src/compress/zstd_fast.c:518) | assertion/abort | [ ] |
| 769 | `ZSTD_compressBlock_fast_dictMatchState_generic` | `(void)maxDistance; (void)endIndex; /* these variables are not used when assert() is disabled */` (c_src/src/compress/zstd_fast.c:519) | assertion/abort | [ ] |
| 770 | `ZSTD_compressBlock_fast_dictMatchState_generic` | `assert(prefixStartIndex >= (U32)(dictEnd - dictBase));` (c_src/src/compress/zstd_fast.c:525) | assertion/abort | [ ] |
| 771 | `ZSTD_compressBlock_fast_dictMatchState_generic` | `assert(offset_1 <= dictAndPrefixLength);` (c_src/src/compress/zstd_fast.c:537) | assertion/abort | [ ] |
| 772 | `ZSTD_compressBlock_fast_dictMatchState_generic` | `assert(offset_2 <= dictAndPrefixLength);` (c_src/src/compress/zstd_fast.c:538) | assertion/abort | [ ] |
| 773 | `ZSTD_compressBlock_fast_dictMatchState_generic` | `assert(stepSize >= 1);` (c_src/src/compress/zstd_fast.c:541) | assertion/abort | [ ] |
| 774 | `ZSTD_compressBlock_fast_dictMatchState_generic` | `assert(mLength);` (c_src/src/compress/zstd_fast.c:634) | assertion/abort | [ ] |
| 775 | `ZSTD_compressBlock_fast_dictMatchState_generic` | `assert(base+curr+2 > istart); /* check base overflow */` (c_src/src/compress/zstd_fast.c:640) | assertion/abort | [ ] |
| 776 | `ZSTD_compressBlock_fast_dictMatchState_generic` | `assert(ip0 == anchor);` (c_src/src/compress/zstd_fast.c:667) | assertion/abort | [ ] |
| 777 | `ZSTD_compressBlock_fast_dictMatchState` | `assert(ms->dictMatchState != NULL);` (c_src/src/compress/zstd_fast.c:691) | assertion/abort | [ ] |
| 778 | `ZSTD_compressBlock_fast_extDict_generic` | `assert((match0 != prefixStart) & (match0 != dictStart));` (c_src/src/compress/zstd_fast.c:813) | assertion/abort | [ ] |
| 779 | `ZSTD_compressBlock_fast_extDict_generic` | `assert(matchEnd != 0);` (c_src/src/compress/zstd_fast.c:922) | assertion/abort | [ ] |
| 780 | `ZSTD_compressBlock_fast_extDict_generic` | `assert(base+current0+2 > istart); /* check base overflow */` (c_src/src/compress/zstd_fast.c:938) | assertion/abort | [ ] |
| 781 | `ZSTD_compressBlock_fast_extDict` | `assert(ms->dictMatchState == NULL);` (c_src/src/compress/zstd_fast.c:972) | assertion/abort | [ ] |
| 782 | `ZSTD_updateDUBT` | `assert(ip + 8 <= iend); /* condition for ZSTD_hashPtr */` (c_src/src/compress/zstd_lazy.c:48) | assertion/abort | [ ] |
| 783 | `ZSTD_updateDUBT` | `assert(idx >= ms->window.dictLimit); /* condition for valid base+idx */` (c_src/src/compress/zstd_lazy.c:51) | assertion/abort | [ ] |
| 784 | `ZSTD_insertDUBT1` | `assert(curr >= btLow);` (c_src/src/compress/zstd_lazy.c:103) | assertion/abort | [ ] |
| 785 | `ZSTD_insertDUBT1` | `assert(ip < iend); /* condition for ZSTD_count */` (c_src/src/compress/zstd_lazy.c:104) | assertion/abort | [ ] |
| 786 | `ZSTD_insertDUBT1` | `assert(matchIndex < curr);` (c_src/src/compress/zstd_lazy.c:109) | assertion/abort | [ ] |
| 787 | `ZSTD_insertDUBT1` | `assert( (matchIndex+matchLength >= dictLimit) /* might be wrong if extDict is incorrectly set to 0 */ \|\| (curr < dictLimit) );` (c_src/src/compress/zstd_lazy.c:120) | assertion/abort | [ ] |
| 788 | `ZSTD_DUBT_findBetterDictMatch` | `assert(dictMode == ZSTD_dictMatchState);` (c_src/src/compress/zstd_lazy.c:197) | assertion/abort | [ ] |
| 789 | `ZSTD_DUBT_findBestMatch` | `assert(ip <= iend-8); /* required for h calculation */` (c_src/src/compress/zstd_lazy.c:272) | assertion/abort | [ ] |
| 790 | `ZSTD_DUBT_findBestMatch` | `assert(dictMode != ZSTD_dedicatedDictSearch);` (c_src/src/compress/zstd_lazy.c:273) | assertion/abort | [ ] |
| 791 | `ZSTD_DUBT_findBestMatch` | `assert(nbCompares <= (1U << ZSTD_SEARCHLOG_MAX)); /* Check we haven't underflowed. */` (c_src/src/compress/zstd_lazy.c:372) | assertion/abort | [ ] |
| 792 | `ZSTD_DUBT_findBestMatch` | `assert(matchEndIdx > curr+8); /* ensure nextToUpdate is increased */` (c_src/src/compress/zstd_lazy.c:380) | assertion/abort | [ ] |
| 793 | `ZSTD_dedicatedDictSearch_lazy_loadDictionary` | `assert(ms->cParams.chainLog <= 24);` (c_src/src/compress/zstd_lazy.c:437) | assertion/abort | [ ] |
| 794 | `ZSTD_dedicatedDictSearch_lazy_loadDictionary` | `assert(ms->cParams.hashLog > ms->cParams.chainLog);` (c_src/src/compress/zstd_lazy.c:438) | assertion/abort | [ ] |
| 795 | `ZSTD_dedicatedDictSearch_lazy_loadDictionary` | `assert(idx != 0);` (c_src/src/compress/zstd_lazy.c:439) | assertion/abort | [ ] |
| 796 | `ZSTD_dedicatedDictSearch_lazy_loadDictionary` | `assert(tmpMinChain <= minChain);` (c_src/src/compress/zstd_lazy.c:440) | assertion/abort | [ ] |
| 797 | `ZSTD_dedicatedDictSearch_lazy_loadDictionary` | `assert(chainPos <= chainSize); /* I believe this is guaranteed... */` (c_src/src/compress/zstd_lazy.c:497) | assertion/abort | [ ] |
| 798 | `ZSTD_dedicatedDictSearch_lazy_search` | `assert(matchIndex >= ddsLowestIndex);` (c_src/src/compress/zstd_lazy.c:567) | assertion/abort | [ ] |
| 799 | `ZSTD_dedicatedDictSearch_lazy_search` | `assert(match+4 <= ddsEnd);` (c_src/src/compress/zstd_lazy.c:568) | assertion/abort | [ ] |
| 800 | `ZSTD_dedicatedDictSearch_lazy_search` | `assert(matchIndex >= ddsLowestIndex);` (c_src/src/compress/zstd_lazy.c:604) | assertion/abort | [ ] |
| 801 | `ZSTD_dedicatedDictSearch_lazy_search` | `assert(match+4 <= ddsEnd);` (c_src/src/compress/zstd_lazy.c:605) | assertion/abort | [ ] |
| 802 | `ZSTD_HcFindBestMatch` | `assert(matchIndex >= dictLimit); /* ensures this is true if dictMode != ZSTD_extDict */` (c_src/src/compress/zstd_lazy.c:712) | assertion/abort | [ ] |
| 803 | `ZSTD_HcFindBestMatch` | `assert(match+4 <= dictEnd);` (c_src/src/compress/zstd_lazy.c:718) | assertion/abort | [ ] |
| 804 | `ZSTD_HcFindBestMatch` | `assert(nbAttempts <= (1U << ZSTD_SEARCHLOG_MAX)); /* Check we haven't underflowed. */` (c_src/src/compress/zstd_lazy.c:734) | assertion/abort | [ ] |
| 805 | `ZSTD_HcFindBestMatch` | `assert(match+4 <= dmsEnd);` (c_src/src/compress/zstd_lazy.c:754) | assertion/abort | [ ] |
| 806 | `ZSTD_HcFindBestMatch` | `assert(curr > matchIndex + dmsIndexDelta);` (c_src/src/compress/zstd_lazy.c:761) | assertion/abort | [ ] |
| 807 | `ZSTD_isAligned` | `assert((align & (align - 1)) == 0);` (c_src/src/compress/zstd_lazy.c:809) | assertion/abort | [ ] |
| 808 | `ZSTD_row_prefetch` | `assert(rowLog == 4 \|\| rowLog == 5 \|\| rowLog == 6);` (c_src/src/compress/zstd_lazy.c:826) | assertion/abort | [ ] |
| 809 | `ZSTD_row_prefetch` | `assert(ZSTD_isAligned(hashTable + relRow, 64)); /* prefetched hash row always 64-byte aligned */` (c_src/src/compress/zstd_lazy.c:827) | assertion/abort | [ ] |
| 810 | `ZSTD_row_prefetch` | `assert(ZSTD_isAligned(tagTable + relRow, (size_t)1 << rowLog)); /* prefetched tagRow sits on correct multiple of bytes (32,64,128) */` (c_src/src/compress/zstd_lazy.c:828) | assertion/abort | [ ] |
| 811 | `ZSTD_row_update_internalImpl` | `assert(hash == ZSTD_hashPtrSalted(base + updateStartIdx, hashLog + ZSTD_ROW_HASH_TAG_BITS, mls, ms->hashSalt));` (c_src/src/compress/zstd_lazy.c:904) | assertion/abort | [ ] |
| 812 | `ZSTD_row_update_internal` | `assert(target >= idx);` (c_src/src/compress/zstd_lazy.c:940) | assertion/abort | [ ] |
| 813 | `ZSTD_row_matchMaskGroupWidth` | `assert((rowEntries == 16) \|\| (rowEntries == 32) \|\| rowEntries == 64);` (c_src/src/compress/zstd_lazy.c:965) | assertion/abort | [ ] |
| 814 | `ZSTD_row_matchMaskGroupWidth` | `assert(rowEntries <= ZSTD_ROW_HASH_MAX_ENTRIES);` (c_src/src/compress/zstd_lazy.c:966) | assertion/abort | [ ] |
| 815 | `ZSTD_row_getSSEMask` | `assert(nbChunks == 1 \|\| nbChunks == 2 \|\| nbChunks == 4);` (c_src/src/compress/zstd_lazy.c:993) | assertion/abort | [ ] |
| 816 | `ZSTD_row_getSSEMask` | `assert(nbChunks == 4);` (c_src/src/compress/zstd_lazy.c:1001) | assertion/abort | [ ] |
| 817 | `ZSTD_row_getNEONMask` | `assert((rowEntries == 16) \|\| (rowEntries == 32) \|\| rowEntries == 64);` (c_src/src/compress/zstd_lazy.c:1010) | assertion/abort | [ ] |
| 818 | `ZSTD_row_getMatchMask` | `assert((rowEntries == 16) \|\| (rowEntries == 32) \|\| rowEntries == 64);` (c_src/src/compress/zstd_lazy.c:1064) | assertion/abort | [ ] |
| 819 | `ZSTD_row_getMatchMask` | `assert(rowEntries <= ZSTD_ROW_HASH_MAX_ENTRIES);` (c_src/src/compress/zstd_lazy.c:1065) | assertion/abort | [ ] |
| 820 | `ZSTD_row_getMatchMask` | `assert(ZSTD_row_matchMaskGroupWidth(rowEntries) * rowEntries <= sizeof(ZSTD_VecMask) * 8);` (c_src/src/compress/zstd_lazy.c:1066) | assertion/abort | [ ] |
| 821 | `ZSTD_row_getMatchMask` | `assert((sizeof(size_t) == 4) \|\| (sizeof(size_t) == 8));` (c_src/src/compress/zstd_lazy.c:1089) | assertion/abort | [ ] |
| 822 | `ZSTD_RowFindBestMatch` | `assert(numMatches < rowEntries);` (c_src/src/compress/zstd_lazy.c:1233) | assertion/abort | [ ] |
| 823 | `ZSTD_RowFindBestMatch` | `assert(matchIndex < curr);` (c_src/src/compress/zstd_lazy.c:1257) | assertion/abort | [ ] |
| 824 | `ZSTD_RowFindBestMatch` | `assert(matchIndex >= lowLimit);` (c_src/src/compress/zstd_lazy.c:1258) | assertion/abort | [ ] |
| 825 | `ZSTD_RowFindBestMatch` | `assert(matchIndex >= dictLimit); /* ensures this is true if dictMode != ZSTD_extDict */` (c_src/src/compress/zstd_lazy.c:1262) | assertion/abort | [ ] |
| 826 | `ZSTD_RowFindBestMatch` | `assert(match+4 <= dictEnd);` (c_src/src/compress/zstd_lazy.c:1268) | assertion/abort | [ ] |
| 827 | `ZSTD_RowFindBestMatch` | `assert(nbAttempts <= (1U << ZSTD_SEARCHLOG_MAX)); /* Check we haven't underflowed. */` (c_src/src/compress/zstd_lazy.c:1282) | assertion/abort | [ ] |
| 828 | `ZSTD_RowFindBestMatch` | `assert(matchIndex >= dmsLowestIndex);` (c_src/src/compress/zstd_lazy.c:1315) | assertion/abort | [ ] |
| 829 | `ZSTD_RowFindBestMatch` | `assert(matchIndex < curr);` (c_src/src/compress/zstd_lazy.c:1316) | assertion/abort | [ ] |
| 830 | `ZSTD_RowFindBestMatch` | `assert(match+4 <= dmsEnd);` (c_src/src/compress/zstd_lazy.c:1319) | assertion/abort | [ ] |
| 831 | `ZSTD_RowFindBestMatch` | `assert(curr > matchIndex + dmsIndexDelta);` (c_src/src/compress/zstd_lazy.c:1326) | assertion/abort | [ ] |
| 832 | `<file scope/macro>` | `assert(MAX(4, MIN(6, ms->cParams.minMatch)) == mls); \` (c_src/src/compress/zstd_lazy.c:1371) | assertion/abort | [ ] |
| 833 | `<file scope/macro>` | `assert(MAX(4, MIN(6, ms->cParams.minMatch)) == mls); \` (c_src/src/compress/zstd_lazy.c:1381) | assertion/abort | [ ] |
| 834 | `<file scope/macro>` | `assert(MAX(4, MIN(6, ms->cParams.minMatch)) == mls); \` (c_src/src/compress/zstd_lazy.c:1391) | assertion/abort | [ ] |
| 835 | `<file scope/macro>` | `assert(MAX(4, MIN(6, ms->cParams.searchLog)) == rowLog); \` (c_src/src/compress/zstd_lazy.c:1392) | assertion/abort | [ ] |
| 836 | `ZSTD_compressBlock_lazy_generic` | `assert(offset_1 <= dictAndPrefixLength);` (c_src/src/compress/zstd_lazy.c:1562) | assertion/abort | [ ] |
| 837 | `ZSTD_compressBlock_lazy_generic` | `assert(offset_2 <= dictAndPrefixLength);` (c_src/src/compress/zstd_lazy.c:1563) | assertion/abort | [ ] |
| 838 | `ZSTD_ldm_adjustParameters` | `ZSTD_STATIC_ASSERT(LDM_BUCKET_SIZE_LOG <= ZSTD_LDM_BUCKETSIZELOG_MAX);` (c_src/src/compress/zstd_ldm.c:139) | exact return/error shown | [ ] |
| 839 | `ZSTD_ldm_adjustParameters` | `assert(params->hashLog <= ZSTD_HASHLOG_MAX);` (c_src/src/compress/zstd_ldm.c:144) | assertion/abort | [ ] |
| 840 | `ZSTD_ldm_adjustParameters` | `assert(1 <= (int)cParams->strategy && (int)cParams->strategy <= 9);` (c_src/src/compress/zstd_ldm.c:149) | assertion/abort | [ ] |
| 841 | `ZSTD_ldm_adjustParameters` | `assert(1 <= (int)cParams->strategy && (int)cParams->strategy <= 9);` (c_src/src/compress/zstd_ldm.c:163) | assertion/abort | [ ] |
| 842 | `ZSTD_ldm_fillFastTables` | `assert(0); /* shouldn't be called: cparams should've been adjusted. */` (c_src/src/compress/zstd_ldm.c:266) | assertion/abort | [ ] |
| 843 | `ZSTD_ldm_fillFastTables` | `assert(0); /* not possible : not a valid strategy id */` (c_src/src/compress/zstd_ldm.c:279) | assertion/abort | [ ] |
| 844 | `ZSTD_ldm_generateSequences_internal` | `return ERROR(dstSize_tooSmall);` (c_src/src/compress/zstd_ldm.c:479) | exact return/error shown | [ ] |
| 845 | `ZSTD_ldm_generateSequences` | `assert(ZSTD_CHUNKSIZE_MAX >= kMaxChunkSize);` (c_src/src/compress/zstd_ldm.c:538) | assertion/abort | [ ] |
| 846 | `ZSTD_ldm_generateSequences` | `assert(ldmState->window.nextSrc >= (BYTE const*)src + srcSize);` (c_src/src/compress/zstd_ldm.c:542) | assertion/abort | [ ] |
| 847 | `ZSTD_ldm_generateSequences` | `assert(sequences->pos <= sequences->size);` (c_src/src/compress/zstd_ldm.c:546) | assertion/abort | [ ] |
| 848 | `ZSTD_ldm_generateSequences` | `assert(sequences->size <= sequences->capacity);` (c_src/src/compress/zstd_ldm.c:547) | assertion/abort | [ ] |
| 849 | `ZSTD_ldm_generateSequences` | `assert(chunkStart < iend);` (c_src/src/compress/zstd_ldm.c:557) | assertion/abort | [ ] |
| 850 | `ZSTD_ldm_generateSequences` | `assert(newLeftoverSize == chunkSize);` (c_src/src/compress/zstd_ldm.c:596) | assertion/abort | [ ] |
| 851 | `maybeSplitSequence` | `assert(sequence.offset > 0);` (c_src/src/compress/zstd_ldm.c:644) | assertion/abort | [ ] |
| 852 | `ZSTD_ldm_blockCompress` | `assert(rawSeqStore->pos <= rawSeqStore->size);` (c_src/src/compress/zstd_ldm.c:706) | assertion/abort | [ ] |
| 853 | `ZSTD_ldm_blockCompress` | `assert(rawSeqStore->size <= rawSeqStore->capacity);` (c_src/src/compress/zstd_ldm.c:707) | assertion/abort | [ ] |
| 854 | `ZSTD_ldm_blockCompress` | `assert(ip + sequence.litLength + sequence.matchLength <= iend);` (c_src/src/compress/zstd_ldm.c:717) | assertion/abort | [ ] |
| 855 | `ZSTD_fracWeight` | `assert(hb + BITCOST_ACCURACY < 31);` (c_src/src/compress/zstd_opt.c:63) | assertion/abort | [ ] |
| 856 | `ZSTD_downscaleStats` | `assert(shift < 30);` (c_src/src/compress/zstd_opt.c:110) | assertion/abort | [ ] |
| 857 | `ZSTD_scaleStats` | `assert(logTarget < 30);` (c_src/src/compress/zstd_opt.c:128) | assertion/abort | [ ] |
| 858 | `ZSTD_rescaleFreqs` | `assert(optPtr->symbolCosts != NULL);` (c_src/src/compress/zstd_opt.c:157) | assertion/abort | [ ] |
| 859 | `ZSTD_rescaleFreqs` | `assert(optPtr->litFreq != NULL);` (c_src/src/compress/zstd_opt.c:166) | assertion/abort | [ ] |
| 860 | `ZSTD_rescaleFreqs` | `assert(bitCost <= scaleLog);` (c_src/src/compress/zstd_opt.c:171) | assertion/abort | [ ] |
| 861 | `ZSTD_rescaleFreqs` | `assert(bitCost < scaleLog);` (c_src/src/compress/zstd_opt.c:183) | assertion/abort | [ ] |
| 862 | `ZSTD_rescaleFreqs` | `assert(bitCost < scaleLog);` (c_src/src/compress/zstd_opt.c:195) | assertion/abort | [ ] |
| 863 | `ZSTD_rescaleFreqs` | `assert(bitCost < scaleLog);` (c_src/src/compress/zstd_opt.c:207) | assertion/abort | [ ] |
| 864 | `ZSTD_rescaleFreqs` | `assert(optPtr->litFreq != NULL);` (c_src/src/compress/zstd_opt.c:214) | assertion/abort | [ ] |
| 865 | `ZSTD_rawLiteralsCost` | `assert(optPtr->litSumBasePrice >= BITCOST_MULTIPLIER);` (c_src/src/compress/zstd_opt.c:283) | assertion/abort | [ ] |
| 866 | `ZSTD_litLengthPrice` | `assert(litLength <= ZSTD_BLOCKSIZE_MAX);` (c_src/src/compress/zstd_opt.c:297) | assertion/abort | [ ] |
| 867 | `ZSTD_getMatchPrice` | `assert(matchLength >= MINMATCH);` (c_src/src/compress/zstd_opt.c:332) | assertion/abort | [ ] |
| 868 | `ZSTD_updateStats` | `assert(offCode <= MaxOff);` (c_src/src/compress/zstd_opt.c:376) | assertion/abort | [ ] |
| 869 | `ZSTD_insertAndFindFirstIndexHash3` | `assert(hashLog3 > 0);` (c_src/src/compress/zstd_opt.c:421) | assertion/abort | [ ] |
| 870 | `ZSTD_insertBt1` | `assert(curr <= target);` (c_src/src/compress/zstd_opt.c:484) | assertion/abort | [ ] |
| 871 | `ZSTD_insertBt1` | `assert(ip <= iend-8); /* required for h calculation */` (c_src/src/compress/zstd_opt.c:485) | assertion/abort | [ ] |
| 872 | `ZSTD_insertBt1` | `assert(windowLow > 0);` (c_src/src/compress/zstd_opt.c:488) | assertion/abort | [ ] |
| 873 | `ZSTD_insertBt1` | `assert(matchIndex < curr);` (c_src/src/compress/zstd_opt.c:492) | assertion/abort | [ ] |
| 874 | `ZSTD_insertBt1` | `assert(matchIndex+matchLength >= dictLimit); /* might be wrong if actually extDict */` (c_src/src/compress/zstd_opt.c:516) | assertion/abort | [ ] |
| 875 | `ZSTD_insertBt1` | `assert(matchEndIdx > curr + 8);` (c_src/src/compress/zstd_opt.c:555) | assertion/abort | [ ] |
| 876 | `ZSTD_updateTree_internal` | `assert(idx < (U32)(idx + forward));` (c_src/src/compress/zstd_opt.c:575) | assertion/abort | [ ] |
| 877 | `ZSTD_updateTree_internal` | `assert((size_t)(ip - base) <= (size_t)(U32)(-1));` (c_src/src/compress/zstd_opt.c:578) | assertion/abort | [ ] |
| 878 | `ZSTD_updateTree_internal` | `assert((size_t)(iend - base) <= (size_t)(U32)(-1));` (c_src/src/compress/zstd_opt.c:579) | assertion/abort | [ ] |
| 879 | `ZSTD_insertBtAndGetAllMatches` | `assert(ll0 <= 1); /* necessarily 1 or 0 */` (c_src/src/compress/zstd_opt.c:645) | assertion/abort | [ ] |
| 880 | `ZSTD_insertBtAndGetAllMatches` | `assert(curr >= dictLimit);` (c_src/src/compress/zstd_opt.c:652) | assertion/abort | [ ] |
| 881 | `ZSTD_insertBtAndGetAllMatches` | `assert(curr >= windowLow);` (c_src/src/compress/zstd_opt.c:664) | assertion/abort | [ ] |
| 882 | `ZSTD_insertBtAndGetAllMatches` | `assert(curr > matchIndex3);` (c_src/src/compress/zstd_opt.c:709) | assertion/abort | [ ] |
| 883 | `ZSTD_insertBtAndGetAllMatches` | `assert(mnum==0); /* no prior solution */` (c_src/src/compress/zstd_opt.c:710) | assertion/abort | [ ] |
| 884 | `ZSTD_insertBtAndGetAllMatches` | `assert(curr > matchIndex);` (c_src/src/compress/zstd_opt.c:728) | assertion/abort | [ ] |
| 885 | `ZSTD_insertBtAndGetAllMatches` | `assert(matchIndex+matchLength >= dictLimit); /* ensure the condition is correct when !extDict */` (c_src/src/compress/zstd_opt.c:731) | assertion/abort | [ ] |
| 886 | `ZSTD_insertBtAndGetAllMatches` | `if (matchIndex >= dictLimit) assert(memcmp(match, ip, matchLength) == 0); /* ensure early section of match is equal as expected */` (c_src/src/compress/zstd_opt.c:733) | assertion/abort | [ ] |
| 887 | `ZSTD_insertBtAndGetAllMatches` | `assert(memcmp(match, ip, matchLength) == 0); /* ensure early section of match is equal as expected */` (c_src/src/compress/zstd_opt.c:737) | assertion/abort | [ ] |
| 888 | `ZSTD_insertBtAndGetAllMatches` | `assert(matchEndIdx > matchIndex);` (c_src/src/compress/zstd_opt.c:746) | assertion/abort | [ ] |
| 889 | `ZSTD_insertBtAndGetAllMatches` | `assert(nbCompares <= (1U << ZSTD_SEARCHLOG_MAX)); /* Check we haven't underflowed. */` (c_src/src/compress/zstd_opt.c:776) | assertion/abort | [ ] |
| 890 | `ZSTD_insertBtAndGetAllMatches` | `assert(matchEndIdx > curr+8);` (c_src/src/compress/zstd_opt.c:815) | assertion/abort | [ ] |
| 891 | `ZSTD_btGetAllMatches_internal` | `assert(BOUNDED(3, ms->cParams.minMatch, 6) == mls);` (c_src/src/compress/zstd_opt.c:844) | assertion/abort | [ ] |
| 892 | `GEN_ZSTD_BT_GET_ALL_MATCHES` | `assert((U32)dictMode < 3);` (c_src/src/compress/zstd_opt.c:897) | assertion/abort | [ ] |
| 893 | `GEN_ZSTD_BT_GET_ALL_MATCHES` | `assert(mls - 3 < 4);` (c_src/src/compress/zstd_opt.c:898) | assertion/abort | [ ] |
| 894 | `ZSTD_opt_getNextMatchAndUpdateSeqStore` | `assert(optLdm->seqStore.posInSequence <= currSeq.litLength + currSeq.matchLength);` (c_src/src/compress/zstd_opt.c:958) | assertion/abort | [ ] |
| 895 | `ZSTD_compressBlock_opt_generic` | `assert(optLevel <= 2);` (c_src/src/compress/zstd_opt.c:1114) | assertion/abort | [ ] |
| 896 | `ZSTD_compressBlock_opt_generic` | `ZSTD_STATIC_ASSERT(sizeof(opt[0].rep[0]) == sizeof(rep[0]));` (c_src/src/compress/zstd_opt.c:1151) | exact return/error shown | [ ] |
| 897 | `ZSTD_compressBlock_opt_generic` | `assert(opt[0].price >= 0);` (c_src/src/compress/zstd_opt.c:1172) | assertion/abort | [ ] |
| 898 | `ZSTD_compressBlock_opt_generic` | `assert(cur <= ZSTD_OPT_NUM);` (c_src/src/compress/zstd_opt.c:1202) | assertion/abort | [ ] |
| 899 | `ZSTD_compressBlock_opt_generic` | `assert(price < 1000000000); /* overflow check */` (c_src/src/compress/zstd_opt.c:1210) | assertion/abort | [ ] |
| 900 | `ZSTD_compressBlock_opt_generic` | `assert(cur >= prevMatch.mlen);` (c_src/src/compress/zstd_opt.c:1234) | assertion/abort | [ ] |
| 901 | `ZSTD_compressBlock_opt_generic` | `ZSTD_STATIC_ASSERT(sizeof(opt[cur].rep) == sizeof(Repcodes_t));` (c_src/src/compress/zstd_opt.c:1254) | exact return/error shown | [ ] |
| 902 | `ZSTD_compressBlock_opt_generic` | `assert(cur >= opt[cur].mlen);` (c_src/src/compress/zstd_opt.c:1255) | assertion/abort | [ ] |
| 903 | `ZSTD_compressBlock_opt_generic` | `assert(opt[cur].price >= 0);` (c_src/src/compress/zstd_opt.c:1274) | assertion/abort | [ ] |
| 904 | `ZSTD_compressBlock_opt_generic` | `assert(cur >= lastStretch.mlen);` (c_src/src/compress/zstd_opt.c:1341) | assertion/abort | [ ] |
| 905 | `ZSTD_compressBlock_opt_generic` | `assert(opt[0].mlen == 0);` (c_src/src/compress/zstd_opt.c:1345) | assertion/abort | [ ] |
| 906 | `ZSTD_compressBlock_opt_generic` | `assert(last_pos >= lastStretch.mlen);` (c_src/src/compress/zstd_opt.c:1346) | assertion/abort | [ ] |
| 907 | `ZSTD_compressBlock_opt_generic` | `assert(cur == last_pos - lastStretch.mlen);` (c_src/src/compress/zstd_opt.c:1347) | assertion/abort | [ ] |
| 908 | `ZSTD_compressBlock_opt_generic` | `assert(lastStretch.litlen == (ip - anchor) + last_pos);` (c_src/src/compress/zstd_opt.c:1351) | assertion/abort | [ ] |
| 909 | `ZSTD_compressBlock_opt_generic` | `assert(lastStretch.off > 0);` (c_src/src/compress/zstd_opt.c:1355) | assertion/abort | [ ] |
| 910 | `ZSTD_compressBlock_opt_generic` | `assert(cur >= lastStretch.litlen);` (c_src/src/compress/zstd_opt.c:1364) | assertion/abort | [ ] |
| 911 | `ZSTD_compressBlock_opt_generic` | `assert(storeEnd < ZSTD_OPT_SIZE);` (c_src/src/compress/zstd_opt.c:1382) | assertion/abort | [ ] |
| 912 | `ZSTD_compressBlock_opt_generic` | `assert(nextStretch.litlen + nextStretch.mlen <= stretchPos);` (c_src/src/compress/zstd_opt.c:1406) | assertion/abort | [ ] |
| 913 | `ZSTD_compressBlock_opt_generic` | `assert(storePos == storeEnd); /* must be last sequence */` (c_src/src/compress/zstd_opt.c:1422) | assertion/abort | [ ] |
| 914 | `ZSTD_compressBlock_opt_generic` | `assert(anchor + llen <= iend);` (c_src/src/compress/zstd_opt.c:1427) | assertion/abort | [ ] |
| 915 | `ZSTD_initStats_ultra` | `assert(ms->opt.litLengthSum == 0); /* first block */` (c_src/src/compress/zstd_opt.c:1493) | assertion/abort | [ ] |
| 916 | `ZSTD_initStats_ultra` | `assert(seqStore->sequences == seqStore->sequencesStart); /* no ldm */` (c_src/src/compress/zstd_opt.c:1494) | assertion/abort | [ ] |
| 917 | `ZSTD_initStats_ultra` | `assert(ms->window.dictLimit == ms->window.lowLimit); /* no dictionary */` (c_src/src/compress/zstd_opt.c:1495) | assertion/abort | [ ] |
| 918 | `ZSTD_initStats_ultra` | `assert(ms->window.dictLimit - ms->nextToUpdate <= 1); /* no prefix (note: intentional overflow, defined as 2-complement) */` (c_src/src/compress/zstd_opt.c:1496) | assertion/abort | [ ] |
| 919 | `ZSTD_compressBlock_btultra2` | `assert(srcSize <= ZSTD_BLOCKSIZE_MAX);` (c_src/src/compress/zstd_opt.c:1532) | assertion/abort | [ ] |
| 920 | `hash2` | `assert(hashLog >= 8);` (c_src/src/compress/zstd_preSplit.c:35) | assertion/abort | [ ] |
| 921 | `hash2` | `assert(hashLog <= HASHLOG_MAX);` (c_src/src/compress/zstd_preSplit.c:37) | assertion/abort | [ ] |
| 922 | `addEvents_generic` | `assert(srcSize >= HASHLENGTH);` (c_src/src/compress/zstd_preSplit.c:62) | assertion/abort | [ ] |
| 923 | `fpDistance` | `assert(hashLog <= HASHLOG_MAX);` (c_src/src/compress/zstd_preSplit.c:99) | assertion/abort | [ ] |
| 924 | `compareFingerprints` | `assert(ref->nbEvents > 0);` (c_src/src/compress/zstd_preSplit.c:115) | assertion/abort | [ ] |
| 925 | `compareFingerprints` | `assert(newfp->nbEvents > 0);` (c_src/src/compress/zstd_preSplit.c:116) | assertion/abort | [ ] |
| 926 | `removeEvents` | `assert(acc->events[n] >= slice->events[n]);` (c_src/src/compress/zstd_preSplit.c:147) | assertion/abort | [ ] |
| 927 | `ZSTD_splitBlock_byChunks` | `const RecordEvents_f record_f = (assert(0<=level && level<=3), records_fs[level]);` (c_src/src/compress/zstd_preSplit.c:162) | assertion/abort | [ ] |
| 928 | `ZSTD_splitBlock_byChunks` | `assert(blockSize == (128 << 10));` (c_src/src/compress/zstd_preSplit.c:167) | assertion/abort | [ ] |
| 929 | `ZSTD_splitBlock_byChunks` | `assert(workspace != NULL);` (c_src/src/compress/zstd_preSplit.c:168) | assertion/abort | [ ] |
| 930 | `ZSTD_splitBlock_byChunks` | `assert((size_t)workspace % ZSTD_ALIGNOF(FPStats) == 0);` (c_src/src/compress/zstd_preSplit.c:169) | assertion/abort | [ ] |
| 931 | `ZSTD_splitBlock_byChunks` | `ZSTD_STATIC_ASSERT(ZSTD_SLIPBLOCK_WORKSPACESIZE >= sizeof(FPStats));` (c_src/src/compress/zstd_preSplit.c:170) | exact return/error shown | [ ] |
| 932 | `ZSTD_splitBlock_byChunks` | `assert(wkspSize >= sizeof(FPStats)); (void)wkspSize;` (c_src/src/compress/zstd_preSplit.c:171) | assertion/abort | [ ] |
| 933 | `ZSTD_splitBlock_byChunks` | `assert(pos == blockSize);` (c_src/src/compress/zstd_preSplit.c:184) | assertion/abort | [ ] |
| 934 | `ZSTD_splitBlock_fromBorders` | `assert(blockSize == (128 << 10));` (c_src/src/compress/zstd_preSplit.c:204) | assertion/abort | [ ] |
| 935 | `ZSTD_splitBlock_fromBorders` | `assert(workspace != NULL);` (c_src/src/compress/zstd_preSplit.c:205) | assertion/abort | [ ] |
| 936 | `ZSTD_splitBlock_fromBorders` | `assert((size_t)workspace % ZSTD_ALIGNOF(FPStats) == 0);` (c_src/src/compress/zstd_preSplit.c:206) | assertion/abort | [ ] |
| 937 | `ZSTD_splitBlock_fromBorders` | `ZSTD_STATIC_ASSERT(ZSTD_SLIPBLOCK_WORKSPACESIZE >= sizeof(FPStats));` (c_src/src/compress/zstd_preSplit.c:207) | exact return/error shown | [ ] |
| 938 | `ZSTD_splitBlock_fromBorders` | `assert(wkspSize >= sizeof(FPStats)); (void)wkspSize;` (c_src/src/compress/zstd_preSplit.c:208) | assertion/abort | [ ] |
| 939 | `ZSTD_splitBlock` | `assert(0<=level && level<=4);` (c_src/src/compress/zstd_preSplit.c:233) | assertion/abort | [ ] |
| 940 | `ZSTDMT_createBufferPool` | `if (bufPool==NULL) return NULL;` (c_src/src/compress/zstdmt_compress.c:126) | exact return/error shown | [ ] |
| 941 | `ZSTDMT_createBufferPool` | `return NULL;` (c_src/src/compress/zstdmt_compress.c:129) | exact return/error shown | [ ] |
| 942 | `ZSTDMT_createBufferPool` | `return NULL;` (c_src/src/compress/zstdmt_compress.c:134) | exact return/error shown | [ ] |
| 943 | `ZSTDMT_expandBufferPool` | `if (srcBufPool==NULL) return NULL;` (c_src/src/compress/zstdmt_compress.c:173) | exact return/error shown | [ ] |
| 944 | `ZSTDMT_resizeBuffer` | `assert(newBuffer.capacity >= buffer.capacity);` (c_src/src/compress/zstdmt_compress.c:243) | assertion/abort | [ ] |
| 945 | `ZSTDMT_createSeqPool` | `if (seqPool == NULL) return NULL;` (c_src/src/compress/zstdmt_compress.c:337) | exact return/error shown | [ ] |
| 946 | `ZSTDMT_createCCtxPool` | `assert(nbWorkers > 0);` (c_src/src/compress/zstdmt_compress.c:385) | assertion/abort | [ ] |
| 947 | `ZSTDMT_createCCtxPool` | `if (!cctxPool) return NULL;` (c_src/src/compress/zstdmt_compress.c:386) | exact return/error shown | [ ] |
| 948 | `ZSTDMT_createCCtxPool` | `return NULL;` (c_src/src/compress/zstdmt_compress.c:389) | exact return/error shown | [ ] |
| 949 | `ZSTDMT_createCCtxPool` | `return NULL;` (c_src/src/compress/zstdmt_compress.c:395) | exact return/error shown | [ ] |
| 950 | `ZSTDMT_createCCtxPool` | `if (!cctxPool->cctxs[0]) { ZSTDMT_freeCCtxPool(cctxPool); return NULL; }` (c_src/src/compress/zstdmt_compress.c:399) | exact return/error shown | [ ] |
| 951 | `ZSTDMT_expandCCtxPool` | `if (srcPool==NULL) return NULL;` (c_src/src/compress/zstdmt_compress.c:408) | exact return/error shown | [ ] |
| 952 | `ZSTDMT_sizeof_CCtxPool` | `assert(nbWorkers > 0);` (c_src/src/compress/zstdmt_compress.c:430) | assertion/abort | [ ] |
| 953 | `ZSTDMT_serialState_reset` | `assert(params.ldmParams.hashLog >= params.ldmParams.bucketSizeLog);` (c_src/src/compress/zstdmt_compress.c:499) | assertion/abort | [ ] |
| 954 | `ZSTDMT_serialState_reset` | `assert(params.ldmParams.hashRateLog < 32);` (c_src/src/compress/zstdmt_compress.c:500) | assertion/abort | [ ] |
| 955 | `ZSTDMT_serialState_init` | `return initError;` (c_src/src/compress/zstdmt_compress.c:566) | exact return/error shown | [ ] |
| 956 | `ZSTDMT_serialState_genSequences` | `assert(seqStore->seq != NULL && seqStore->pos == 0 && seqStore->size == 0 && seqStore->capacity > 0);` (c_src/src/compress/zstdmt_compress.c:597) | assertion/abort | [ ] |
| 957 | `ZSTDMT_serialState_genSequences` | `assert(src.size <= serialState->params.jobSize);` (c_src/src/compress/zstdmt_compress.c:599) | assertion/abort | [ ] |
| 958 | `ZSTDMT_serialState_genSequences` | `assert(!ZSTD_isError(error)); (void)error;` (c_src/src/compress/zstdmt_compress.c:605) | assertion/abort | [ ] |
| 959 | `ZSTDMT_serialState_applySequences` | `ZSTDMT_serialState_applySequences(const SerialState* serialState, /* just for an assert() check */ ZSTD_CCtx* jobCCtx, const RawSeqStore_t* seqStore) { if (seqStore->size > 0) { DEBUGLOG(5, "ZSTDMT_serialState_applySequences: uploading %u external sequences", (unsigned)seqStore->size); assert(serialState->params.ldmParams.enableLdm == ZSTD_ps_enable); (void)serialState; assert(jobCCtx); ZSTD_referenceExternalSequences(jobCCtx, seqStore->seq, seqStore->size); } }` (c_src/src/compress/zstdmt_compress.c:624) | assertion/abort | [ ] |
| 960 | `ZSTDMT_serialState_applySequences` | `assert(serialState->params.ldmParams.enableLdm == ZSTD_ps_enable); (void)serialState;` (c_src/src/compress/zstdmt_compress.c:630) | assertion/abort | [ ] |
| 961 | `ZSTDMT_serialState_applySequences` | `assert(jobCCtx);` (c_src/src/compress/zstdmt_compress.c:631) | assertion/abort | [ ] |
| 962 | `ZSTDMT_serialState_ensureFinished` | `assert(ZSTD_isError(cSize)); (void)cSize;` (c_src/src/compress/zstdmt_compress.c:641) | assertion/abort | [ ] |
| 963 | `ZSTDMT_compressionJob` | `assert(job->firstJob); /* only allowed for first job */` (c_src/src/compress/zstdmt_compress.c:730) | assertion/abort | [ ] |
| 964 | `ZSTDMT_compressionJob` | `if (sizeof(size_t) > sizeof(int)) assert(job->src.size < ((size_t)INT_MAX) * chunkSize); /* check overflow */` (c_src/src/compress/zstdmt_compress.c:768) | assertion/abort | [ ] |
| 965 | `ZSTDMT_compressionJob` | `assert(job->cSize == 0);` (c_src/src/compress/zstdmt_compress.c:770) | assertion/abort | [ ] |
| 966 | `ZSTDMT_compressionJob` | `op += cSize; assert(op < oend);` (c_src/src/compress/zstdmt_compress.c:775) | assertion/abort | [ ] |
| 967 | `ZSTDMT_compressionJob` | `assert(chunkSize > 0);` (c_src/src/compress/zstdmt_compress.c:786) | assertion/abort | [ ] |
| 968 | `ZSTDMT_compressionJob` | `assert((chunkSize & (chunkSize - 1)) == 0); /* chunkSize must be power of 2 for mask==(chunkSize-1) to work */` (c_src/src/compress/zstdmt_compress.c:787) | assertion/abort | [ ] |
| 969 | `ZSTDMT_compressionJob` | `assert(!ZSTD_window_hasExtDict(cctx->blockState.matchState.window));` (c_src/src/compress/zstdmt_compress.c:801) | assertion/abort | [ ] |
| 970 | `ZSTDMT_compressionJob` | `if (ZSTD_isError(job->cSize)) assert(lastCBlockSize == 0);` (c_src/src/compress/zstdmt_compress.c:815) | assertion/abort | [ ] |
| 971 | `ZSTDMT_createJobsTable` | `if (jobTable==NULL) return NULL;` (c_src/src/compress/zstdmt_compress.c:916) | exact return/error shown | [ ] |
| 972 | `ZSTDMT_createJobsTable` | `return NULL;` (c_src/src/compress/zstdmt_compress.c:924) | exact return/error shown | [ ] |
| 973 | `ZSTDMT_expandJobsTable` | `if (mtctx->jobs==NULL) return ERROR(memory_allocation);` (c_src/src/compress/zstdmt_compress.c:935) | exact return/error shown | [ ] |
| 974 | `ZSTDMT_expandJobsTable` | `assert((nbJobs != 0) && ((nbJobs & (nbJobs - 1)) == 0)); /* ensure nbJobs is a power of 2 */` (c_src/src/compress/zstdmt_compress.c:936) | assertion/abort | [ ] |
| 975 | `ZSTDMT_createCCtx_advanced_internal` | `if (nbWorkers < 1) return NULL;` (c_src/src/compress/zstdmt_compress.c:957) | exact return/error shown | [ ] |
| 976 | `ZSTDMT_createCCtx_advanced_internal` | `return NULL;` (c_src/src/compress/zstdmt_compress.c:961) | exact return/error shown | [ ] |
| 977 | `ZSTDMT_createCCtx_advanced_internal` | `if (!mtctx) return NULL;` (c_src/src/compress/zstdmt_compress.c:964) | exact return/error shown | [ ] |
| 978 | `ZSTDMT_createCCtx_advanced_internal` | `assert(nbJobs > 0); assert((nbJobs & (nbJobs - 1)) == 0); /* ensure nbJobs is a power of 2 */` (c_src/src/compress/zstdmt_compress.c:977) | assertion/abort | [ ] |
| 979 | `ZSTDMT_createCCtx_advanced_internal` | `return NULL;` (c_src/src/compress/zstdmt_compress.c:986) | exact return/error shown | [ ] |
| 980 | `ZSTDMT_createCCtx_advanced` | `return NULL;` (c_src/src/compress/zstdmt_compress.c:1000) | exact return/error shown | [ ] |
| 981 | `ZSTDMT_resize` | `if (POOL_resize(mtctx->factory, nbWorkers)) return ERROR(memory_allocation);` (c_src/src/compress/zstdmt_compress.c:1080) | exact return/error shown | [ ] |
| 982 | `ZSTDMT_resize` | `FORWARD_IF_ERROR( ZSTDMT_expandJobsTable(mtctx, nbWorkers) , "");` (c_src/src/compress/zstdmt_compress.c:1081) | exact return/error shown | [ ] |
| 983 | `ZSTDMT_resize` | `if (mtctx->bufPool == NULL) return ERROR(memory_allocation);` (c_src/src/compress/zstdmt_compress.c:1083) | exact return/error shown | [ ] |
| 984 | `ZSTDMT_resize` | `if (mtctx->cctxPool == NULL) return ERROR(memory_allocation);` (c_src/src/compress/zstdmt_compress.c:1085) | exact return/error shown | [ ] |
| 985 | `ZSTDMT_resize` | `if (mtctx->seqPool == NULL) return ERROR(memory_allocation);` (c_src/src/compress/zstdmt_compress.c:1087) | exact return/error shown | [ ] |
| 986 | `ZSTDMT_getFrameProgression` | `unsigned lastJobNb = mtctx->nextJobID + mtctx->jobReady; assert(mtctx->jobReady <= 1);` (c_src/src/compress/zstdmt_compress.c:1123) | assertion/abort | [ ] |
| 987 | `ZSTDMT_getFrameProgression` | `assert(flushed <= produced);` (c_src/src/compress/zstdmt_compress.c:1133) | assertion/abort | [ ] |
| 988 | `ZSTDMT_toFlushNow` | `assert(jobID <= mtctx->nextJobID);` (c_src/src/compress/zstdmt_compress.c:1151) | assertion/abort | [ ] |
| 989 | `ZSTDMT_toFlushNow` | `assert(flushed <= produced);` (c_src/src/compress/zstdmt_compress.c:1161) | assertion/abort | [ ] |
| 990 | `ZSTDMT_toFlushNow` | `assert(jobPtr->consumed <= jobPtr->src.size);` (c_src/src/compress/zstdmt_compress.c:1162) | assertion/abort | [ ] |
| 991 | `ZSTDMT_toFlushNow` | `assert(jobPtr->consumed < jobPtr->src.size);` (c_src/src/compress/zstdmt_compress.c:1170) | assertion/abort | [ ] |
| 992 | `ZSTDMT_overlapLog` | `assert(0 <= ovlog && ovlog <= 9);` (c_src/src/compress/zstdmt_compress.c:1221) | assertion/abort | [ ] |
| 993 | `ZSTDMT_computeOverlapSize` | `assert(0 <= overlapRLog && overlapRLog <= 8);` (c_src/src/compress/zstdmt_compress.c:1230) | assertion/abort | [ ] |
| 994 | `ZSTDMT_computeOverlapSize` | `assert(0 <= ovLog && ovLog <= ZSTD_WINDOWLOG_MAX);` (c_src/src/compress/zstdmt_compress.c:1239) | assertion/abort | [ ] |
| 995 | `ZSTDMT_initCStream_internal` | `assert(!ZSTD_isError(ZSTD_checkCParams(params.cParams)));` (c_src/src/compress/zstdmt_compress.c:1259) | assertion/abort | [ ] |
| 996 | `ZSTDMT_initCStream_internal` | `assert(!((dict) && (cdict))); /* either dict or cdict, not both */` (c_src/src/compress/zstdmt_compress.c:1260) | assertion/abort | [ ] |
| 997 | `ZSTDMT_initCStream_internal` | `FORWARD_IF_ERROR( ZSTDMT_resize(mtctx, (unsigned)params.nbWorkers) , "");` (c_src/src/compress/zstdmt_compress.c:1264) | exact return/error shown | [ ] |
| 998 | `ZSTDMT_initCStream_internal` | `if (mtctx->cdictLocal == NULL) return ERROR(memory_allocation);` (c_src/src/compress/zstdmt_compress.c:1283) | exact return/error shown | [ ] |
| 999 | `ZSTDMT_initCStream_internal` | `assert(mtctx->targetSectionSize <= (size_t)ZSTDMT_JOBSIZE_MAX);` (c_src/src/compress/zstdmt_compress.c:1295) | assertion/abort | [ ] |
| 1000 | `ZSTDMT_initCStream_internal` | `U32 const rsyncBits = (assert(jobSizeKB >= 1), ZSTD_highbit32(jobSizeKB) + 10);` (c_src/src/compress/zstdmt_compress.c:1300) | assertion/abort | [ ] |
| 1001 | `ZSTDMT_initCStream_internal` | `assert(rsyncBits >= RSYNC_MIN_BLOCK_LOG + 2);` (c_src/src/compress/zstdmt_compress.c:1303) | assertion/abort | [ ] |
| 1002 | `ZSTDMT_initCStream_internal` | `return ERROR(memory_allocation);` (c_src/src/compress/zstdmt_compress.c:1334) | exact return/error shown | [ ] |
| 1003 | `ZSTDMT_initCStream_internal` | `if (mtctx->cdictLocal == NULL) return ERROR(memory_allocation);` (c_src/src/compress/zstdmt_compress.c:1365) | exact return/error shown | [ ] |
| 1004 | `ZSTDMT_initCStream_internal` | `return ERROR(memory_allocation);` (c_src/src/compress/zstdmt_compress.c:1373) | exact return/error shown | [ ] |
| 1005 | `ZSTDMT_writeLastEmptyBlock` | `assert(job->lastJob == 1);` (c_src/src/compress/zstdmt_compress.c:1387) | assertion/abort | [ ] |
| 1006 | `ZSTDMT_writeLastEmptyBlock` | `assert(job->src.size == 0); /* last job is empty -> will be simplified into a last empty block */` (c_src/src/compress/zstdmt_compress.c:1388) | assertion/abort | [ ] |
| 1007 | `ZSTDMT_writeLastEmptyBlock` | `assert(job->firstJob == 0); /* cannot be first job, as it also needs to create frame header */` (c_src/src/compress/zstdmt_compress.c:1389) | assertion/abort | [ ] |
| 1008 | `ZSTDMT_writeLastEmptyBlock` | `assert(job->dstBuff.start == NULL); /* invoked from streaming variant only (otherwise, dstBuff might be user's output) */` (c_src/src/compress/zstdmt_compress.c:1390) | assertion/abort | [ ] |
| 1009 | `ZSTDMT_writeLastEmptyBlock` | `assert(job->dstBuff.capacity >= ZSTD_blockHeaderSize); /* no buffer should ever be that small */` (c_src/src/compress/zstdmt_compress.c:1396) | assertion/abort | [ ] |
| 1010 | `ZSTDMT_writeLastEmptyBlock` | `assert(!ZSTD_isError(job->cSize));` (c_src/src/compress/zstdmt_compress.c:1399) | assertion/abort | [ ] |
| 1011 | `ZSTDMT_writeLastEmptyBlock` | `assert(job->consumed == 0);` (c_src/src/compress/zstdmt_compress.c:1400) | assertion/abort | [ ] |
| 1012 | `ZSTDMT_createCompressionJob` | `assert((mtctx->nextJobID & mtctx->jobIDMask) == (mtctx->doneJobID & mtctx->jobIDMask));` (c_src/src/compress/zstdmt_compress.c:1410) | assertion/abort | [ ] |
| 1013 | `ZSTDMT_createCompressionJob` | `assert(mtctx->inBuff.filled >= srcSize);` (c_src/src/compress/zstdmt_compress.c:1420) | assertion/abort | [ ] |
| 1014 | `ZSTDMT_createCompressionJob` | `assert(endOp == ZSTD_e_end); /* only possible case : need to end the frame with an empty last block */` (c_src/src/compress/zstdmt_compress.c:1458) | assertion/abort | [ ] |
| 1015 | `ZSTDMT_flushProduced` | `assert(output->size >= output->pos);` (c_src/src/compress/zstdmt_compress.c:1493) | assertion/abort | [ ] |
| 1016 | `ZSTDMT_flushProduced` | `assert(mtctx->jobs[wJobID].dstFlushed <= mtctx->jobs[wJobID].cSize);` (c_src/src/compress/zstdmt_compress.c:1498) | assertion/abort | [ ] |
| 1017 | `ZSTDMT_flushProduced` | `assert(srcConsumed <= srcSize);` (c_src/src/compress/zstdmt_compress.c:1523) | assertion/abort | [ ] |
| 1018 | `ZSTDMT_flushProduced` | `assert(mtctx->doneJobID < mtctx->nextJobID);` (c_src/src/compress/zstdmt_compress.c:1538) | assertion/abort | [ ] |
| 1019 | `ZSTDMT_flushProduced` | `assert(cSize >= mtctx->jobs[wJobID].dstFlushed);` (c_src/src/compress/zstdmt_compress.c:1539) | assertion/abort | [ ] |
| 1020 | `ZSTDMT_flushProduced` | `assert(mtctx->jobs[wJobID].dstBuff.start != NULL);` (c_src/src/compress/zstdmt_compress.c:1540) | assertion/abort | [ ] |
| 1021 | `ZSTDMT_getInputDataInUse` | `assert(range.start <= mtctx->jobs[wJobID].src.start);` (c_src/src/compress/zstdmt_compress.c:1605) | assertion/abort | [ ] |
| 1022 | `ZSTDMT_tryGetInputRange` | `assert(mtctx->inBuff.buffer.start == NULL);` (c_src/src/compress/zstdmt_compress.c:1688) | assertion/abort | [ ] |
| 1023 | `ZSTDMT_tryGetInputRange` | `assert(mtctx->roundBuff.capacity >= spaceNeeded);` (c_src/src/compress/zstdmt_compress.c:1689) | assertion/abort | [ ] |
| 1024 | `ZSTDMT_tryGetInputRange` | `assert(!ZSTDMT_isOverlapped(buffer, mtctx->inBuff.prefix));` (c_src/src/compress/zstdmt_compress.c:1716) | assertion/abort | [ ] |
| 1025 | `ZSTDMT_tryGetInputRange` | `assert(mtctx->roundBuff.pos + buffer.capacity <= mtctx->roundBuff.capacity);` (c_src/src/compress/zstdmt_compress.c:1730) | assertion/abort | [ ] |
| 1026 | `findSynchronizationPoint` | `assert(mtctx->inBuff.filled >= RSYNC_LENGTH);` (c_src/src/compress/zstdmt_compress.c:1787) | assertion/abort | [ ] |
| 1027 | `findSynchronizationPoint` | `assert(mtctx->inBuff.filled >= RSYNC_MIN_BLOCK_SIZE);` (c_src/src/compress/zstdmt_compress.c:1797) | assertion/abort | [ ] |
| 1028 | `findSynchronizationPoint` | `assert(RSYNC_MIN_BLOCK_SIZE >= RSYNC_LENGTH);` (c_src/src/compress/zstdmt_compress.c:1798) | assertion/abort | [ ] |
| 1029 | `findSynchronizationPoint` | `assert(pos < RSYNC_LENGTH \|\| ZSTD_rollingHash_compute(istart + pos - RSYNC_LENGTH, RSYNC_LENGTH) == hash);` (c_src/src/compress/zstdmt_compress.c:1821) | assertion/abort | [ ] |
| 1030 | `findSynchronizationPoint` | `* assert(pos < RSYNC_LENGTH \|\| ZSTD_rollingHash_compute(istart + pos - RSYNC_LENGTH, RSYNC_LENGTH) == hash);` (c_src/src/compress/zstdmt_compress.c:1827) | assertion/abort | [ ] |
| 1031 | `findSynchronizationPoint` | `assert(mtctx->inBuff.filled + pos >= RSYNC_MIN_BLOCK_SIZE);` (c_src/src/compress/zstdmt_compress.c:1830) | assertion/abort | [ ] |
| 1032 | `findSynchronizationPoint` | `assert(pos < RSYNC_LENGTH \|\| ZSTD_rollingHash_compute(istart + pos - RSYNC_LENGTH, RSYNC_LENGTH) == hash);` (c_src/src/compress/zstdmt_compress.c:1838) | assertion/abort | [ ] |
| 1033 | `ZSTDMT_compressStream_generic` | `assert(output->pos <= output->size);` (c_src/src/compress/zstdmt_compress.c:1861) | assertion/abort | [ ] |
| 1034 | `ZSTDMT_compressStream_generic` | `assert(input->pos <= input->size);` (c_src/src/compress/zstdmt_compress.c:1862) | assertion/abort | [ ] |
| 1035 | `ZSTDMT_compressStream_generic` | `return ERROR(stage_wrong);` (c_src/src/compress/zstdmt_compress.c:1866) | exact return/error shown | [ ] |
| 1036 | `ZSTDMT_compressStream_generic` | `assert(mtctx->inBuff.filled == 0); /* Can't fill an empty buffer */` (c_src/src/compress/zstdmt_compress.c:1873) | assertion/abort | [ ] |
| 1037 | `ZSTDMT_compressStream_generic` | `assert(mtctx->doneJobID != mtctx->nextJobID);` (c_src/src/compress/zstdmt_compress.c:1879) | assertion/abort | [ ] |
| 1038 | `ZSTDMT_compressStream_generic` | `assert(mtctx->inBuff.buffer.capacity >= mtctx->targetSectionSize);` (c_src/src/compress/zstdmt_compress.c:1888) | assertion/abort | [ ] |
| 1039 | `ZSTDMT_compressStream_generic` | `assert(mtctx->inBuff.filled == 0 \|\| mtctx->inBuff.filled == mtctx->targetSectionSize \|\| mtctx->params.rsyncable);` (c_src/src/compress/zstdmt_compress.c:1904) | assertion/abort | [ ] |
| 1040 | `ZSTDMT_compressStream_generic` | `assert(mtctx->inBuff.filled <= mtctx->targetSectionSize);` (c_src/src/compress/zstdmt_compress.c:1913) | assertion/abort | [ ] |
| 1041 | `ZSTDMT_compressStream_generic` | `FORWARD_IF_ERROR( ZSTDMT_createCompressionJob(mtctx, jobSize, endOp) , "");` (c_src/src/compress/zstdmt_compress.c:1914) | exact return/error shown | [ ] |
| 1042 | `HUF_initFastDStream` | `assert(bitsConsumed <= 8);` (c_src/src/decompress/huf_decompress.c:154) | assertion/abort | [ ] |
| 1043 | `HUF_initFastDStream` | `assert(sizeof(size_t) == 8);` (c_src/src/decompress/huf_decompress.c:155) | assertion/abort | [ ] |
| 1044 | `HUF_DecompressFastArgs_init` | `assert(dst != NULL);` (c_src/src/decompress/huf_decompress.c:209) | assertion/abort | [ ] |
| 1045 | `HUF_DecompressFastArgs_init` | `return ERROR(corruption_detected);` (c_src/src/decompress/huf_decompress.c:213) | exact return/error shown | [ ] |
| 1046 | `HUF_DecompressFastArgs_init` | `if (length4 > srcSize) return ERROR(corruption_detected); /* overflow */` (c_src/src/decompress/huf_decompress.c:238) | exact return/error shown | [ ] |
| 1047 | `HUF_initRemainingDStream` | `return ERROR(corruption_detected);` (c_src/src/decompress/huf_decompress.c:285) | exact return/error shown | [ ] |
| 1048 | `HUF_initRemainingDStream` | `return ERROR(corruption_detected);` (c_src/src/decompress/huf_decompress.c:292) | exact return/error shown | [ ] |
| 1049 | `HUF_initRemainingDStream` | `assert(sizeof(size_t) == 8);` (c_src/src/decompress/huf_decompress.c:295) | assertion/abort | [ ] |
| 1050 | `HUF_DEltX1_set4` | `assert(D4 < (1U << 16));` (c_src/src/decompress/huf_decompress.c:342) | assertion/abort | [ ] |
| 1051 | `HUF_readDTableX1_wksp` | `if (sizeof(*wksp) > wkspSize) return ERROR(tableLog_tooLarge);` (c_src/src/decompress/huf_decompress.c:395) | exact return/error shown | [ ] |
| 1052 | `HUF_readDTableX1_wksp` | `if (tableLog > (U32)(dtd.maxTableLog+1)) return ERROR(tableLog_tooLarge); /* DTable too small, Huffman tree cannot fit in */` (c_src/src/decompress/huf_decompress.c:409) | exact return/error shown | [ ] |
| 1053 | `HUF_readDTableX1_wksp` | `assert(u == length);` (c_src/src/decompress/huf_decompress.c:509) | assertion/abort | [ ] |
| 1054 | `HUF_decompress1X1_usingDTable_internal_body` | `if (!BIT_endOfDStream(&bitD)) return ERROR(corruption_detected);` (c_src/src/decompress/huf_decompress.c:592) | exact return/error shown | [ ] |
| 1055 | `HUF_decompress4X1_usingDTable_internal_body` | `if (cSrcSize < 10) return ERROR(corruption_detected); /* strict minimum : jump table + 1 byte per stream */` (c_src/src/decompress/huf_decompress.c:608) | exact return/error shown | [ ] |
| 1056 | `HUF_decompress4X1_usingDTable_internal_body` | `if (dstSize < 6) return ERROR(corruption_detected); /* stream 4-split doesn't work */` (c_src/src/decompress/huf_decompress.c:609) | exact return/error shown | [ ] |
| 1057 | `HUF_decompress4X1_usingDTable_internal_body` | `if (length4 > cSrcSize) return ERROR(corruption_detected); /* overflow */` (c_src/src/decompress/huf_decompress.c:643) | exact return/error shown | [ ] |
| 1058 | `HUF_decompress4X1_usingDTable_internal_body` | `if (opStart4 > oend) return ERROR(corruption_detected); /* overflow */` (c_src/src/decompress/huf_decompress.c:644) | exact return/error shown | [ ] |
| 1059 | `HUF_decompress4X1_usingDTable_internal_body` | `assert(dstSize >= 6); /* validated above */` (c_src/src/decompress/huf_decompress.c:645) | assertion/abort | [ ] |
| 1060 | `HUF_decompress4X1_usingDTable_internal_body` | `if (op1 > opStart2) return ERROR(corruption_detected);` (c_src/src/decompress/huf_decompress.c:680) | exact return/error shown | [ ] |
| 1061 | `HUF_decompress4X1_usingDTable_internal_body` | `if (op2 > opStart3) return ERROR(corruption_detected);` (c_src/src/decompress/huf_decompress.c:681) | exact return/error shown | [ ] |
| 1062 | `HUF_decompress4X1_usingDTable_internal_body` | `if (op3 > opStart4) return ERROR(corruption_detected);` (c_src/src/decompress/huf_decompress.c:682) | exact return/error shown | [ ] |
| 1063 | `HUF_decompress4X1_usingDTable_internal_body` | `if (!endCheck) return ERROR(corruption_detected); }` (c_src/src/decompress/huf_decompress.c:693) | exact return/error shown | [ ] |
| 1064 | `HUF_decompress4X1_usingDTable_internal_fast_c_loop` | `assert(MEM_isLittleEndian());` (c_src/src/decompress/huf_decompress.c:735) | assertion/abort | [ ] |
| 1065 | `HUF_decompress4X1_usingDTable_internal_fast_c_loop` | `assert(!MEM_32bits());` (c_src/src/decompress/huf_decompress.c:736) | assertion/abort | [ ] |
| 1066 | `HUF_decompress4X1_usingDTable_internal_fast_c_loop` | `assert(op[stream] <= (stream == 3 ? oend : op[stream + 1]));` (c_src/src/decompress/huf_decompress.c:745) | assertion/abort | [ ] |
| 1067 | `HUF_decompress4X1_usingDTable_internal_fast_c_loop` | `assert(ip[stream] >= ilowest);` (c_src/src/decompress/huf_decompress.c:746) | assertion/abort | [ ] |
| 1068 | `HUF_decompress4X1_usingDTable_internal_fast_c_loop` | `assert(ip[stream] >= ip[stream - 1]);` (c_src/src/decompress/huf_decompress.c:783) | assertion/abort | [ ] |
| 1069 | `HUF_decompress4X1_usingDTable_internal_fast` | `FORWARD_IF_ERROR(ret, "Failed to init fast loop args");` (c_src/src/decompress/huf_decompress.c:851) | exact return/error shown | [ ] |
| 1070 | `HUF_decompress4X1_usingDTable_internal_fast` | `assert(args.ip[0] >= args.ilowest);` (c_src/src/decompress/huf_decompress.c:856) | assertion/abort | [ ] |
| 1071 | `HUF_decompress4X1_usingDTable_internal_fast` | `assert(args.ip[0] >= ilowest);` (c_src/src/decompress/huf_decompress.c:862) | assertion/abort | [ ] |
| 1072 | `HUF_decompress4X1_usingDTable_internal_fast` | `assert(args.ip[0] >= ilowest);` (c_src/src/decompress/huf_decompress.c:863) | assertion/abort | [ ] |
| 1073 | `HUF_decompress4X1_usingDTable_internal_fast` | `assert(args.ip[1] >= ilowest);` (c_src/src/decompress/huf_decompress.c:864) | assertion/abort | [ ] |
| 1074 | `HUF_decompress4X1_usingDTable_internal_fast` | `assert(args.ip[2] >= ilowest);` (c_src/src/decompress/huf_decompress.c:865) | assertion/abort | [ ] |
| 1075 | `HUF_decompress4X1_usingDTable_internal_fast` | `assert(args.ip[3] >= ilowest);` (c_src/src/decompress/huf_decompress.c:866) | assertion/abort | [ ] |
| 1076 | `HUF_decompress4X1_usingDTable_internal_fast` | `assert(args.op[3] <= oend);` (c_src/src/decompress/huf_decompress.c:867) | assertion/abort | [ ] |
| 1077 | `HUF_decompress4X1_usingDTable_internal_fast` | `assert(ilowest == args.ilowest);` (c_src/src/decompress/huf_decompress.c:869) | assertion/abort | [ ] |
| 1078 | `HUF_decompress4X1_usingDTable_internal_fast` | `assert(ilowest + 6 == args.iend[0]);` (c_src/src/decompress/huf_decompress.c:870) | assertion/abort | [ ] |
| 1079 | `HUF_decompress4X1_usingDTable_internal_fast` | `FORWARD_IF_ERROR(HUF_initRemainingDStream(&bit, &args, i, segmentEnd), "corruption");` (c_src/src/decompress/huf_decompress.c:883) | exact return/error shown | [ ] |
| 1080 | `HUF_decompress4X1_usingDTable_internal_fast` | `if (args.op[i] != segmentEnd) return ERROR(corruption_detected);` (c_src/src/decompress/huf_decompress.c:886) | exact return/error shown | [ ] |
| 1081 | `HUF_decompress4X1_usingDTable_internal_fast` | `assert(dstSize != 0);` (c_src/src/decompress/huf_decompress.c:891) | assertion/abort | [ ] |
| 1082 | `HUF_decompress4X1_DCtx_wksp` | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` (c_src/src/decompress/huf_decompress.c:938) | exact return/error shown | [ ] |
| 1083 | `HUF_fillDTableX2ForWeight` | `assert(level >= 1 && level <= 2);` (c_src/src/decompress/huf_decompress.c:1018) | assertion/abort | [ ] |
| 1084 | `HUF_fillDTableX2Level2` | `assert(length > 1);` (c_src/src/decompress/huf_decompress.c:1082) | assertion/abort | [ ] |
| 1085 | `HUF_fillDTableX2Level2` | `assert((U32)skipSize < length);` (c_src/src/decompress/huf_decompress.c:1083) | assertion/abort | [ ] |
| 1086 | `HUF_fillDTableX2Level2` | `assert(skipSize == 1);` (c_src/src/decompress/huf_decompress.c:1086) | assertion/abort | [ ] |
| 1087 | `HUF_fillDTableX2Level2` | `assert(skipSize <= 4);` (c_src/src/decompress/huf_decompress.c:1090) | assertion/abort | [ ] |
| 1088 | `HUF_readDTableX2_wksp` | `if (sizeof(*wksp) > wkspSize) return ERROR(GENERIC);` (c_src/src/decompress/huf_decompress.c:1193) | exact return/error shown | [ ] |
| 1089 | `HUF_readDTableX2_wksp` | `if (maxTableLog > HUF_TABLELOG_MAX) return ERROR(tableLog_tooLarge);` (c_src/src/decompress/huf_decompress.c:1200) | exact return/error shown | [ ] |
| 1090 | `HUF_readDTableX2_wksp` | `if (tableLog > maxTableLog) return ERROR(tableLog_tooLarge); /* DTable can't fit code depth */` (c_src/src/decompress/huf_decompress.c:1207) | exact return/error shown | [ ] |
| 1091 | `HUF_decompress1X2_usingDTable_internal_body` | `if (!BIT_endOfDStream(&bitD)) return ERROR(corruption_detected);` (c_src/src/decompress/huf_decompress.c:1373) | exact return/error shown | [ ] |
| 1092 | `HUF_decompress4X2_usingDTable_internal_body` | `if (cSrcSize < 10) return ERROR(corruption_detected); /* strict minimum : jump table + 1 byte per stream */` (c_src/src/decompress/huf_decompress.c:1389) | exact return/error shown | [ ] |
| 1093 | `HUF_decompress4X2_usingDTable_internal_body` | `if (dstSize < 6) return ERROR(corruption_detected); /* stream 4-split doesn't work */` (c_src/src/decompress/huf_decompress.c:1390) | exact return/error shown | [ ] |
| 1094 | `HUF_decompress4X2_usingDTable_internal_body` | `if (length4 > cSrcSize) return ERROR(corruption_detected); /* overflow */` (c_src/src/decompress/huf_decompress.c:1424) | exact return/error shown | [ ] |
| 1095 | `HUF_decompress4X2_usingDTable_internal_body` | `if (opStart4 > oend) return ERROR(corruption_detected); /* overflow */` (c_src/src/decompress/huf_decompress.c:1425) | exact return/error shown | [ ] |
| 1096 | `HUF_decompress4X2_usingDTable_internal_body` | `assert(dstSize >= 6 /* validated above */);` (c_src/src/decompress/huf_decompress.c:1426) | assertion/abort | [ ] |
| 1097 | `HUF_decompress4X2_usingDTable_internal_body` | `if (op1 > opStart2) return ERROR(corruption_detected);` (c_src/src/decompress/huf_decompress.c:1483) | exact return/error shown | [ ] |
| 1098 | `HUF_decompress4X2_usingDTable_internal_body` | `if (op2 > opStart3) return ERROR(corruption_detected);` (c_src/src/decompress/huf_decompress.c:1484) | exact return/error shown | [ ] |
| 1099 | `HUF_decompress4X2_usingDTable_internal_body` | `if (op3 > opStart4) return ERROR(corruption_detected);` (c_src/src/decompress/huf_decompress.c:1485) | exact return/error shown | [ ] |
| 1100 | `HUF_decompress4X2_usingDTable_internal_body` | `if (!endCheck) return ERROR(corruption_detected); }` (c_src/src/decompress/huf_decompress.c:1496) | exact return/error shown | [ ] |
| 1101 | `HUF_decompress4X2_usingDTable_internal_fast_c_loop` | `assert(MEM_isLittleEndian());` (c_src/src/decompress/huf_decompress.c:1543) | assertion/abort | [ ] |
| 1102 | `HUF_decompress4X2_usingDTable_internal_fast_c_loop` | `assert(!MEM_32bits());` (c_src/src/decompress/huf_decompress.c:1544) | assertion/abort | [ ] |
| 1103 | `HUF_decompress4X2_usingDTable_internal_fast_c_loop` | `assert(op[stream] <= oend[stream]);` (c_src/src/decompress/huf_decompress.c:1553) | assertion/abort | [ ] |
| 1104 | `HUF_decompress4X2_usingDTable_internal_fast_c_loop` | `assert(ip[stream] >= ilowest);` (c_src/src/decompress/huf_decompress.c:1554) | assertion/abort | [ ] |
| 1105 | `HUF_decompress4X2_usingDTable_internal_fast_c_loop` | `assert(ip[stream] >= ip[stream - 1]);` (c_src/src/decompress/huf_decompress.c:1601) | assertion/abort | [ ] |
| 1106 | `HUF_decompress4X2_usingDTable_internal_fast` | `FORWARD_IF_ERROR(ret, "Failed to init asm args");` (c_src/src/decompress/huf_decompress.c:1678) | exact return/error shown | [ ] |
| 1107 | `HUF_decompress4X2_usingDTable_internal_fast` | `assert(args.ip[0] >= args.ilowest);` (c_src/src/decompress/huf_decompress.c:1683) | assertion/abort | [ ] |
| 1108 | `HUF_decompress4X2_usingDTable_internal_fast` | `assert(args.ip[0] >= ilowest);` (c_src/src/decompress/huf_decompress.c:1687) | assertion/abort | [ ] |
| 1109 | `HUF_decompress4X2_usingDTable_internal_fast` | `assert(args.ip[1] >= ilowest);` (c_src/src/decompress/huf_decompress.c:1688) | assertion/abort | [ ] |
| 1110 | `HUF_decompress4X2_usingDTable_internal_fast` | `assert(args.ip[2] >= ilowest);` (c_src/src/decompress/huf_decompress.c:1689) | assertion/abort | [ ] |
| 1111 | `HUF_decompress4X2_usingDTable_internal_fast` | `assert(args.ip[3] >= ilowest);` (c_src/src/decompress/huf_decompress.c:1690) | assertion/abort | [ ] |
| 1112 | `HUF_decompress4X2_usingDTable_internal_fast` | `assert(args.op[3] <= oend);` (c_src/src/decompress/huf_decompress.c:1691) | assertion/abort | [ ] |
| 1113 | `HUF_decompress4X2_usingDTable_internal_fast` | `assert(ilowest == args.ilowest);` (c_src/src/decompress/huf_decompress.c:1693) | assertion/abort | [ ] |
| 1114 | `HUF_decompress4X2_usingDTable_internal_fast` | `assert(ilowest + 6 == args.iend[0]);` (c_src/src/decompress/huf_decompress.c:1694) | assertion/abort | [ ] |
| 1115 | `HUF_decompress4X2_usingDTable_internal_fast` | `FORWARD_IF_ERROR(HUF_initRemainingDStream(&bit, &args, i, segmentEnd), "corruption");` (c_src/src/decompress/huf_decompress.c:1708) | exact return/error shown | [ ] |
| 1116 | `HUF_decompress4X2_usingDTable_internal_fast` | `return ERROR(corruption_detected);` (c_src/src/decompress/huf_decompress.c:1711) | exact return/error shown | [ ] |
| 1117 | `HUF_DGEN` | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` (c_src/src/decompress/huf_decompress.c:1763) | exact return/error shown | [ ] |
| 1118 | `HUF_decompress4X2_DCtx_wksp` | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` (c_src/src/decompress/huf_decompress.c:1778) | exact return/error shown | [ ] |
| 1119 | `HUF_selectDecoder` | `assert(dstSize > 0);` (c_src/src/decompress/huf_decompress.c:1823) | assertion/abort | [ ] |
| 1120 | `HUF_selectDecoder` | `assert(dstSize <= 128*1024);` (c_src/src/decompress/huf_decompress.c:1824) | assertion/abort | [ ] |
| 1121 | `HUF_decompress1X_DCtx_wksp` | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` (c_src/src/decompress/huf_decompress.c:1850) | exact return/error shown | [ ] |
| 1122 | `HUF_decompress1X_DCtx_wksp` | `if (cSrcSize > dstSize) return ERROR(corruption_detected); /* invalid */` (c_src/src/decompress/huf_decompress.c:1851) | exact return/error shown | [ ] |
| 1123 | `HUF_decompress1X_DCtx_wksp` | `assert(algoNb == 0);` (c_src/src/decompress/huf_decompress.c:1858) | assertion/abort | [ ] |
| 1124 | `HUF_decompress1X_DCtx_wksp` | `assert(algoNb == 1);` (c_src/src/decompress/huf_decompress.c:1863) | assertion/abort | [ ] |
| 1125 | `HUF_decompress1X_usingDTable` | `assert(dtd.tableType == 0);` (c_src/src/decompress/huf_decompress.c:1881) | assertion/abort | [ ] |
| 1126 | `HUF_decompress1X_usingDTable` | `assert(dtd.tableType == 1);` (c_src/src/decompress/huf_decompress.c:1885) | assertion/abort | [ ] |
| 1127 | `HUF_decompress1X1_DCtx_wksp` | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` (c_src/src/decompress/huf_decompress.c:1900) | exact return/error shown | [ ] |
| 1128 | `HUF_decompress4X_usingDTable` | `assert(dtd.tableType == 0);` (c_src/src/decompress/huf_decompress.c:1912) | assertion/abort | [ ] |
| 1129 | `HUF_decompress4X_usingDTable` | `assert(dtd.tableType == 1);` (c_src/src/decompress/huf_decompress.c:1916) | assertion/abort | [ ] |
| 1130 | `HUF_decompress4X_hufOnly_wksp` | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` (c_src/src/decompress/huf_decompress.c:1927) | exact return/error shown | [ ] |
| 1131 | `HUF_decompress4X_hufOnly_wksp` | `if (cSrcSize == 0) return ERROR(corruption_detected);` (c_src/src/decompress/huf_decompress.c:1928) | exact return/error shown | [ ] |
| 1132 | `HUF_decompress4X_hufOnly_wksp` | `assert(algoNb == 0);` (c_src/src/decompress/huf_decompress.c:1933) | assertion/abort | [ ] |
| 1133 | `HUF_decompress4X_hufOnly_wksp` | `assert(algoNb == 1);` (c_src/src/decompress/huf_decompress.c:1937) | assertion/abort | [ ] |
| 1134 | `ZSTD_DDict_dictContent` | `assert(ddict != NULL);` (c_src/src/decompress/zstd_ddict.c:48) | assertion/abort | [ ] |
| 1135 | `ZSTD_DDict_dictSize` | `assert(ddict != NULL);` (c_src/src/decompress/zstd_ddict.c:54) | assertion/abort | [ ] |
| 1136 | `ZSTD_copyDDictParameters` | `assert(dctx != NULL);` (c_src/src/decompress/zstd_ddict.c:61) | assertion/abort | [ ] |
| 1137 | `ZSTD_copyDDictParameters` | `assert(ddict != NULL);` (c_src/src/decompress/zstd_ddict.c:62) | assertion/abort | [ ] |
| 1138 | `ZSTD_loadEntropy_intoDDict` | `return ERROR(dictionary_corrupted); /* only accept specified dictionaries */` (c_src/src/decompress/zstd_ddict.c:99) | exact return/error shown | [ ] |
| 1139 | `ZSTD_loadEntropy_intoDDict` | `return ERROR(dictionary_corrupted); /* only accept specified dictionaries */` (c_src/src/decompress/zstd_ddict.c:105) | exact return/error shown | [ ] |
| 1140 | `ZSTD_loadEntropy_intoDDict` | `RETURN_ERROR_IF(ZSTD_isError(ZSTD_loadDEntropy( &ddict->entropy, ddict->dictContent, ddict->dictSize)), dictionary_corrupted, "");` (c_src/src/decompress/zstd_ddict.c:112) | exact return/error shown | [ ] |
| 1141 | `ZSTD_initDDict_internal` | `if (!internalBuffer) return ERROR(memory_allocation);` (c_src/src/decompress/zstd_ddict.c:133) | exact return/error shown | [ ] |
| 1142 | `ZSTD_initDDict_internal` | `FORWARD_IF_ERROR( ZSTD_loadEntropy_intoDDict(ddict, dictContentType) , "");` (c_src/src/decompress/zstd_ddict.c:140) | exact return/error shown | [ ] |
| 1143 | `ZSTD_createDDict_advanced` | `if ((!customMem.customAlloc) ^ (!customMem.customFree)) return NULL;` (c_src/src/decompress/zstd_ddict.c:150) | exact return/error shown | [ ] |
| 1144 | `ZSTD_createDDict_advanced` | `if (ddict == NULL) return NULL;` (c_src/src/decompress/zstd_ddict.c:153) | exact return/error shown | [ ] |
| 1145 | `ZSTD_createDDict_advanced` | `return NULL;` (c_src/src/decompress/zstd_ddict.c:160) | exact return/error shown | [ ] |
| 1146 | `ZSTD_initStaticDDict` | `assert(sBuffer != NULL);` (c_src/src/decompress/zstd_ddict.c:196) | assertion/abort | [ ] |
| 1147 | `ZSTD_initStaticDDict` | `assert(dict != NULL);` (c_src/src/decompress/zstd_ddict.c:197) | assertion/abort | [ ] |
| 1148 | `ZSTD_initStaticDDict` | `if ((size_t)sBuffer & 7) return NULL; /* 8-aligned */` (c_src/src/decompress/zstd_ddict.c:198) | exact return/error shown | [ ] |
| 1149 | `ZSTD_initStaticDDict` | `if (sBufferSize < neededSpace) return NULL;` (c_src/src/decompress/zstd_ddict.c:199) | exact return/error shown | [ ] |
| 1150 | `ZSTD_initStaticDDict` | `return NULL;` (c_src/src/decompress/zstd_ddict.c:207) | exact return/error shown | [ ] |
| 1151 | `ZSTD_DDictHashSet_emplaceDDict` | `RETURN_ERROR_IF(hashSet->ddictPtrCount == hashSet->ddictPtrTableSize, GENERIC, "Hash set is full!");` (c_src/src/decompress/zstd_decompress.c:109) | exact return/error shown | [ ] |
| 1152 | `ZSTD_DDictHashSet_expand` | `RETURN_ERROR_IF(!newTable, memory_allocation, "Expanded hashset allocation failed!");` (c_src/src/decompress/zstd_decompress.c:139) | exact return/error shown | [ ] |
| 1153 | `ZSTD_DDictHashSet_expand` | `FORWARD_IF_ERROR(ZSTD_DDictHashSet_emplaceDDict(hashSet, oldTable[i]), "");` (c_src/src/decompress/zstd_decompress.c:145) | exact return/error shown | [ ] |
| 1154 | `ZSTD_createDDictHashSet` | `return NULL;` (c_src/src/decompress/zstd_decompress.c:182) | exact return/error shown | [ ] |
| 1155 | `ZSTD_createDDictHashSet` | `return NULL;` (c_src/src/decompress/zstd_decompress.c:186) | exact return/error shown | [ ] |
| 1156 | `ZSTD_DDictHashSet_addDDict` | `FORWARD_IF_ERROR(ZSTD_DDictHashSet_expand(hashSet, customMem), "");` (c_src/src/decompress/zstd_decompress.c:212) | exact return/error shown | [ ] |
| 1157 | `ZSTD_DDictHashSet_addDDict` | `FORWARD_IF_ERROR(ZSTD_DDictHashSet_emplaceDDict(hashSet, ddict), "");` (c_src/src/decompress/zstd_decompress.c:214) | exact return/error shown | [ ] |
| 1158 | `ZSTD_startingInputLength` | `assert( (format == ZSTD_f_zstd1) \|\| (format == ZSTD_f_zstd1_magicless) );` (c_src/src/decompress/zstd_decompress.c:236) | assertion/abort | [ ] |
| 1159 | `ZSTD_DCtx_resetParameters` | `assert(dctx->streamStage == zdss_init);` (c_src/src/decompress/zstd_decompress.c:242) | assertion/abort | [ ] |
| 1160 | `ZSTD_initStaticDCtx` | `if ((size_t)workspace & 7) return NULL; /* 8-aligned */` (c_src/src/decompress/zstd_decompress.c:285) | exact return/error shown | [ ] |
| 1161 | `ZSTD_initStaticDCtx` | `if (workspaceSize < sizeof(ZSTD_DCtx)) return NULL; /* minimum size */` (c_src/src/decompress/zstd_decompress.c:286) | exact return/error shown | [ ] |
| 1162 | `ZSTD_createDCtx_internal` | `if ((!customMem.customAlloc) ^ (!customMem.customFree)) return NULL;` (c_src/src/decompress/zstd_decompress.c:295) | exact return/error shown | [ ] |
| 1163 | `ZSTD_createDCtx_internal` | `if (!dctx) return NULL;` (c_src/src/decompress/zstd_decompress.c:298) | exact return/error shown | [ ] |
| 1164 | `ZSTD_freeDCtx` | `RETURN_ERROR_IF(dctx->staticSize, memory_allocation, "not compatible with static DCtx");` (c_src/src/decompress/zstd_decompress.c:327) | exact return/error shown | [ ] |
| 1165 | `ZSTD_DCtx_selectFrameDDict` | `assert(dctx->refMultipleDDicts && dctx->ddictSet);` (c_src/src/decompress/zstd_decompress.c:361) | assertion/abort | [ ] |
| 1166 | `ZSTD_frameHeaderSize_internal` | `RETURN_ERROR_IF(srcSize < minInputSize, srcSize_wrong, "");` (c_src/src/decompress/zstd_decompress.c:419) | exact return/error shown | [ ] |
| 1167 | `ZSTD_getFrameHeader_advanced` | `/* note : technically could be considered an assert(), since it's an invalid entry */ RETURN_ERROR_IF(src==NULL, GENERIC, "invalid parameter : src==NULL, but srcSize>0");` (c_src/src/decompress/zstd_decompress.c:455) | assertion/abort | [ ] |
| 1168 | `ZSTD_getFrameHeader_advanced` | `RETURN_ERROR_IF(src==NULL, GENERIC, "invalid parameter : src==NULL, but srcSize>0");` (c_src/src/decompress/zstd_decompress.c:456) | exact return/error shown | [ ] |
| 1169 | `ZSTD_getFrameHeader_advanced` | `assert(src != NULL);` (c_src/src/decompress/zstd_decompress.c:466) | assertion/abort | [ ] |
| 1170 | `ZSTD_getFrameHeader_advanced` | `RETURN_ERROR(prefix_unknown, "first bytes don't correspond to any supported magic number");` (c_src/src/decompress/zstd_decompress.c:473) | exact return/error shown | [ ] |
| 1171 | `ZSTD_getFrameHeader_advanced` | `RETURN_ERROR(prefix_unknown, "");` (c_src/src/decompress/zstd_decompress.c:493) | exact return/error shown | [ ] |
| 1172 | `ZSTD_getFrameHeader_advanced` | `RETURN_ERROR_IF((fhdByte & 0x08) != 0, frameParameter_unsupported, "reserved bits, must be zero");` (c_src/src/decompress/zstd_decompress.c:511) | exact return/error shown | [ ] |
| 1173 | `ZSTD_getFrameHeader_advanced` | `RETURN_ERROR_IF(windowLog > ZSTD_WINDOWLOG_MAX, frameParameter_windowTooLarge, "");` (c_src/src/decompress/zstd_decompress.c:517) | exact return/error shown | [ ] |
| 1174 | `ZSTD_getFrameHeader_advanced` | `assert(0); /* impossible */` (c_src/src/decompress/zstd_decompress.c:524) | assertion/abort | [ ] |
| 1175 | `ZSTD_getFrameHeader_advanced` | `assert(0); /* impossible */` (c_src/src/decompress/zstd_decompress.c:534) | assertion/abort | [ ] |
| 1176 | `readSkippableFrameSize` | `RETURN_ERROR_IF(srcSize < ZSTD_SKIPPABLEHEADERSIZE, srcSize_wrong, "");` (c_src/src/decompress/zstd_decompress.c:592) | exact return/error shown | [ ] |
| 1177 | `readSkippableFrameSize` | `RETURN_ERROR_IF((U32)(sizeU32 + ZSTD_SKIPPABLEHEADERSIZE) < sizeU32, frameParameter_unsupported, "");` (c_src/src/decompress/zstd_decompress.c:595) | exact return/error shown | [ ] |
| 1178 | `readSkippableFrameSize` | `RETURN_ERROR_IF(skippableSize > srcSize, srcSize_wrong, "");` (c_src/src/decompress/zstd_decompress.c:598) | exact return/error shown | [ ] |
| 1179 | `ZSTD_readSkippableFrame` | `RETURN_ERROR_IF(srcSize < ZSTD_SKIPPABLEHEADERSIZE, srcSize_wrong, "");` (c_src/src/decompress/zstd_decompress.c:618) | exact return/error shown | [ ] |
| 1180 | `ZSTD_readSkippableFrame` | `RETURN_ERROR_IF(!ZSTD_isSkippableFrame(src, srcSize), frameParameter_unsupported, "");` (c_src/src/decompress/zstd_decompress.c:625) | exact return/error shown | [ ] |
| 1181 | `ZSTD_readSkippableFrame` | `RETURN_ERROR_IF(skippableFrameSize < ZSTD_SKIPPABLEHEADERSIZE \|\| skippableFrameSize > srcSize, srcSize_wrong, "");` (c_src/src/decompress/zstd_decompress.c:626) | exact return/error shown | [ ] |
| 1182 | `ZSTD_readSkippableFrame` | `RETURN_ERROR_IF(skippableContentSize > dstCapacity, dstSize_tooSmall, "");` (c_src/src/decompress/zstd_decompress.c:627) | exact return/error shown | [ ] |
| 1183 | `ZSTD_findDecompressedSize` | `assert(skippableSize <= srcSize);` (c_src/src/decompress/zstd_decompress.c:653) | assertion/abort | [ ] |
| 1184 | `ZSTD_findDecompressedSize` | `assert(frameSrcSize <= srcSize);` (c_src/src/decompress/zstd_decompress.c:670) | assertion/abort | [ ] |
| 1185 | `ZSTD_getDecompressedSize` | `ZSTD_STATIC_ASSERT(ZSTD_CONTENTSIZE_ERROR < ZSTD_CONTENTSIZE_UNKNOWN);` (c_src/src/decompress/zstd_decompress.c:693) | exact return/error shown | [ ] |
| 1186 | `ZSTD_decodeFrameHeader` | `RETURN_ERROR_IF(result>0, srcSize_wrong, "headerSize too small");` (c_src/src/decompress/zstd_decompress.c:706) | exact return/error shown | [ ] |
| 1187 | `ZSTD_decodeFrameHeader` | `RETURN_ERROR_IF(dctx->fParams.dictID && (dctx->dictID != dctx->fParams.dictID), dictionary_wrong, "");` (c_src/src/decompress/zstd_decompress.c:717) | exact return/error shown | [ ] |
| 1188 | `ZSTD_findFrameSizeInfo` | `assert(ZSTD_isError(frameSizeInfo.compressedSize) \|\| frameSizeInfo.compressedSize <= srcSize);` (c_src/src/decompress/zstd_decompress.c:747) | assertion/abort | [ ] |
| 1189 | `ZSTD_findFrameSizeInfo` | `return ZSTD_errorFrameSizeInfo(ret);` (c_src/src/decompress/zstd_decompress.c:760) | exact return/error shown | [ ] |
| 1190 | `ZSTD_findFrameSizeInfo` | `return ZSTD_errorFrameSizeInfo(ERROR(srcSize_wrong));` (c_src/src/decompress/zstd_decompress.c:762) | exact return/error shown | [ ] |
| 1191 | `ZSTD_findFrameSizeInfo` | `return ZSTD_errorFrameSizeInfo(cBlockSize);` (c_src/src/decompress/zstd_decompress.c:773) | exact return/error shown | [ ] |
| 1192 | `ZSTD_findFrameSizeInfo` | `return ZSTD_errorFrameSizeInfo(ERROR(srcSize_wrong));` (c_src/src/decompress/zstd_decompress.c:776) | exact return/error shown | [ ] |
| 1193 | `ZSTD_findFrameSizeInfo` | `return ZSTD_errorFrameSizeInfo(ERROR(srcSize_wrong));` (c_src/src/decompress/zstd_decompress.c:788) | exact return/error shown | [ ] |
| 1194 | `ZSTD_decompressBound` | `assert(srcSize >= compressedSize);` (c_src/src/decompress/zstd_decompress.c:830) | assertion/abort | [ ] |
| 1195 | `ZSTD_decompressionMargin` | `FORWARD_IF_ERROR(ZSTD_getFrameHeader(&zfh, src, srcSize), "");` (c_src/src/decompress/zstd_decompress.c:850) | exact return/error shown | [ ] |
| 1196 | `ZSTD_decompressionMargin` | `return ERROR(corruption_detected);` (c_src/src/decompress/zstd_decompress.c:852) | exact return/error shown | [ ] |
| 1197 | `ZSTD_decompressionMargin` | `assert(zfh.frameType == ZSTD_skippableFrame);` (c_src/src/decompress/zstd_decompress.c:865) | assertion/abort | [ ] |
| 1198 | `ZSTD_decompressionMargin` | `assert(srcSize >= compressedSize);` (c_src/src/decompress/zstd_decompress.c:870) | assertion/abort | [ ] |
| 1199 | `ZSTD_copyRawBlock` | `RETURN_ERROR_IF(srcSize > dstCapacity, dstSize_tooSmall, "");` (c_src/src/decompress/zstd_decompress.c:900) | exact return/error shown | [ ] |
| 1200 | `ZSTD_copyRawBlock` | `RETURN_ERROR(dstBuffer_null, "");` (c_src/src/decompress/zstd_decompress.c:903) | exact return/error shown | [ ] |
| 1201 | `ZSTD_setRleBlock` | `RETURN_ERROR_IF(regenSize > dstCapacity, dstSize_tooSmall, "");` (c_src/src/decompress/zstd_decompress.c:913) | exact return/error shown | [ ] |
| 1202 | `ZSTD_setRleBlock` | `RETURN_ERROR(dstBuffer_null, "");` (c_src/src/decompress/zstd_decompress.c:916) | exact return/error shown | [ ] |
| 1203 | `ZSTD_decompressFrame` | `RETURN_ERROR_IF( remainingSrcSize < ZSTD_FRAMEHEADERSIZE_MIN(dctx->format)+ZSTD_blockHeaderSize, srcSize_wrong, "");` (c_src/src/decompress/zstd_decompress.c:967) | exact return/error shown | [ ] |
| 1204 | `ZSTD_decompressFrame` | `RETURN_ERROR_IF(remainingSrcSize < frameHeaderSize+ZSTD_blockHeaderSize, srcSize_wrong, "");` (c_src/src/decompress/zstd_decompress.c:975) | exact return/error shown | [ ] |
| 1205 | `ZSTD_decompressFrame` | `FORWARD_IF_ERROR( ZSTD_decodeFrameHeader(dctx, ip, frameHeaderSize) , "");` (c_src/src/decompress/zstd_decompress.c:977) | exact return/error shown | [ ] |
| 1206 | `ZSTD_decompressFrame` | `RETURN_ERROR_IF(cBlockSize > remainingSrcSize, srcSize_wrong, "");` (c_src/src/decompress/zstd_decompress.c:995) | exact return/error shown | [ ] |
| 1207 | `ZSTD_decompressFrame` | `assert(dctx->isFrameDecompression == 1);` (c_src/src/decompress/zstd_decompress.c:1017) | assertion/abort | [ ] |
| 1208 | `ZSTD_decompressFrame` | `RETURN_ERROR(corruption_detected, "invalid block type");` (c_src/src/decompress/zstd_decompress.c:1029) | exact return/error shown | [ ] |
| 1209 | `ZSTD_decompressFrame` | `FORWARD_IF_ERROR(decodedSize, "Block decompression failure");` (c_src/src/decompress/zstd_decompress.c:1031) | exact return/error shown | [ ] |
| 1210 | `ZSTD_decompressFrame` | `assert(ip != NULL);` (c_src/src/decompress/zstd_decompress.c:1039) | assertion/abort | [ ] |
| 1211 | `ZSTD_decompressFrame` | `RETURN_ERROR_IF((U64)(op-ostart) != dctx->fParams.frameContentSize, corruption_detected, "");` (c_src/src/decompress/zstd_decompress.c:1046) | exact return/error shown | [ ] |
| 1212 | `ZSTD_decompressFrame` | `RETURN_ERROR_IF(remainingSrcSize<4, checksum_wrong, "");` (c_src/src/decompress/zstd_decompress.c:1050) | exact return/error shown | [ ] |
| 1213 | `ZSTD_decompressFrame` | `RETURN_ERROR_IF(checkRead != checkCalc, checksum_wrong, "");` (c_src/src/decompress/zstd_decompress.c:1055) | exact return/error shown | [ ] |
| 1214 | `ZSTD_decompressMultiFrame` | `assert(dict==NULL \|\| ddict==NULL); /* either dict or ddict set, not both */` (c_src/src/decompress/zstd_decompress.c:1080) | assertion/abort | [ ] |
| 1215 | `ZSTD_decompressMultiFrame` | `RETURN_ERROR_IF(dctx->staticSize, memory_allocation, "legacy support is not compatible with static dctx");` (c_src/src/decompress/zstd_decompress.c:1094) | exact return/error shown | [ ] |
| 1216 | `ZSTD_decompressMultiFrame` | `RETURN_ERROR_IF(expectedSize == ZSTD_CONTENTSIZE_ERROR, corruption_detected, "Corrupted frame header!");` (c_src/src/decompress/zstd_decompress.c:1102) | exact return/error shown | [ ] |
| 1217 | `ZSTD_decompressMultiFrame` | `RETURN_ERROR_IF(expectedSize != decodedSize, corruption_detected, "Frame header size does not match decoded size!");` (c_src/src/decompress/zstd_decompress.c:1104) | exact return/error shown | [ ] |
| 1218 | `ZSTD_decompressMultiFrame` | `assert(decodedSize <= dstCapacity);` (c_src/src/decompress/zstd_decompress.c:1109) | assertion/abort | [ ] |
| 1219 | `ZSTD_decompressMultiFrame` | `FORWARD_IF_ERROR(skippableSize, "invalid skippable frame");` (c_src/src/decompress/zstd_decompress.c:1126) | exact return/error shown | [ ] |
| 1220 | `ZSTD_decompressMultiFrame` | `assert(skippableSize <= srcSize);` (c_src/src/decompress/zstd_decompress.c:1127) | assertion/abort | [ ] |
| 1221 | `ZSTD_decompressMultiFrame` | `FORWARD_IF_ERROR(ZSTD_decompressBegin_usingDDict(dctx, ddict), "");` (c_src/src/decompress/zstd_decompress.c:1136) | exact return/error shown | [ ] |
| 1222 | `ZSTD_decompressMultiFrame` | `FORWARD_IF_ERROR(ZSTD_decompressBegin_usingDict(dctx, dict, dictSize), "");` (c_src/src/decompress/zstd_decompress.c:1140) | exact return/error shown | [ ] |
| 1223 | `ZSTD_decompressMultiFrame` | `RETURN_ERROR_IF( (ZSTD_getErrorCode(res) == ZSTD_error_prefix_unknown) && (moreThan1Frame==1), srcSize_wrong, "At least one frame successfully completed, " "but following bytes are garbage: " "it's more likely to be a srcSize error, " "specifying more input bytes than size of frame(s). " "Note: one could be unlucky, it might be a corruption error instead, " "happening right at the place where we expect zstd magic bytes. " "But this is _much_ less likely than a srcSize field error.");` (c_src/src/decompress/zstd_decompress.c:1146) | exact return/error shown | [ ] |
| 1224 | `ZSTD_decompressMultiFrame` | `assert(res <= dstCapacity);` (c_src/src/decompress/zstd_decompress.c:1158) | assertion/abort | [ ] |
| 1225 | `ZSTD_decompressMultiFrame` | `RETURN_ERROR_IF(srcSize, srcSize_wrong, "input not entirely consumed");` (c_src/src/decompress/zstd_decompress.c:1166) | exact return/error shown | [ ] |
| 1226 | `ZSTD_getDDict` | `assert(0 /* Impossible */);` (c_src/src/decompress/zstd_decompress.c:1184) | assertion/abort | [ ] |
| 1227 | `ZSTD_getDDict` | `return NULL;` (c_src/src/decompress/zstd_decompress.c:1188) | exact return/error shown | [ ] |
| 1228 | `ZSTD_decompress` | `RETURN_ERROR_IF(dctx==NULL, memory_allocation, "NULL pointer!");` (c_src/src/decompress/zstd_decompress.c:1208) | exact return/error shown | [ ] |
| 1229 | `ZSTD_nextInputType` | `assert(0);` (c_src/src/decompress/zstd_decompress.c:1248) | assertion/abort | [ ] |
| 1230 | `ZSTD_decompressContinue` | `RETURN_ERROR_IF(srcSize != ZSTD_nextSrcSizeToDecompressWithInputSize(dctx, srcSize), srcSize_wrong, "not allowed");` (c_src/src/decompress/zstd_decompress.c:1279) | exact return/error shown | [ ] |
| 1231 | `ZSTD_decompressContinue` | `assert(src != NULL);` (c_src/src/decompress/zstd_decompress.c:1287) | assertion/abort | [ ] |
| 1232 | `ZSTD_decompressContinue` | `assert(srcSize >= ZSTD_FRAMEIDSIZE); /* to read skippable magic number */` (c_src/src/decompress/zstd_decompress.c:1289) | assertion/abort | [ ] |
| 1233 | `ZSTD_decompressContinue` | `assert(src != NULL);` (c_src/src/decompress/zstd_decompress.c:1304) | assertion/abort | [ ] |
| 1234 | `ZSTD_decompressContinue` | `FORWARD_IF_ERROR(ZSTD_decodeFrameHeader(dctx, dctx->headerBuffer, dctx->headerSize), "");` (c_src/src/decompress/zstd_decompress.c:1306) | exact return/error shown | [ ] |
| 1235 | `ZSTD_decompressContinue` | `RETURN_ERROR_IF(cBlockSize > dctx->fParams.blockSizeMax, corruption_detected, "Block Size Exceeds Maximum");` (c_src/src/decompress/zstd_decompress.c:1315) | exact return/error shown | [ ] |
| 1236 | `ZSTD_decompressContinue` | `assert(dctx->isFrameDecompression == 1);` (c_src/src/decompress/zstd_decompress.c:1347) | assertion/abort | [ ] |
| 1237 | `ZSTD_decompressContinue` | `assert(srcSize <= dctx->expected);` (c_src/src/decompress/zstd_decompress.c:1352) | assertion/abort | [ ] |
| 1238 | `ZSTD_decompressContinue` | `FORWARD_IF_ERROR(rSize, "ZSTD_copyRawBlock failed");` (c_src/src/decompress/zstd_decompress.c:1354) | exact return/error shown | [ ] |
| 1239 | `ZSTD_decompressContinue` | `assert(rSize == srcSize);` (c_src/src/decompress/zstd_decompress.c:1355) | assertion/abort | [ ] |
| 1240 | `ZSTD_decompressContinue` | `RETURN_ERROR(corruption_detected, "invalid block type");` (c_src/src/decompress/zstd_decompress.c:1364) | exact return/error shown | [ ] |
| 1241 | `ZSTD_decompressContinue` | `FORWARD_IF_ERROR(rSize, "");` (c_src/src/decompress/zstd_decompress.c:1366) | exact return/error shown | [ ] |
| 1242 | `ZSTD_decompressContinue` | `RETURN_ERROR_IF(rSize > dctx->fParams.blockSizeMax, corruption_detected, "Decompressed Block Size Exceeds Maximum");` (c_src/src/decompress/zstd_decompress.c:1367) | exact return/error shown | [ ] |
| 1243 | `ZSTD_decompressContinue` | `RETURN_ERROR_IF( dctx->fParams.frameContentSize != ZSTD_CONTENTSIZE_UNKNOWN && dctx->decodedSize != dctx->fParams.frameContentSize, corruption_detected, "");` (c_src/src/decompress/zstd_decompress.c:1380) | exact return/error shown | [ ] |
| 1244 | `ZSTD_decompressContinue` | `assert(srcSize == 4); /* guaranteed by dctx->expected */` (c_src/src/decompress/zstd_decompress.c:1400) | assertion/abort | [ ] |
| 1245 | `ZSTD_decompressContinue` | `RETURN_ERROR_IF(check32 != h32, checksum_wrong, "");` (c_src/src/decompress/zstd_decompress.c:1406) | exact return/error shown | [ ] |
| 1246 | `ZSTD_decompressContinue` | `assert(src != NULL);` (c_src/src/decompress/zstd_decompress.c:1415) | assertion/abort | [ ] |
| 1247 | `ZSTD_decompressContinue` | `assert(srcSize <= ZSTD_SKIPPABLEHEADERSIZE);` (c_src/src/decompress/zstd_decompress.c:1416) | assertion/abort | [ ] |
| 1248 | `ZSTD_decompressContinue` | `assert(dctx->format != ZSTD_f_zstd1_magicless);` (c_src/src/decompress/zstd_decompress.c:1417) | assertion/abort | [ ] |
| 1249 | `ZSTD_decompressContinue` | `assert(0); /* impossible */` (c_src/src/decompress/zstd_decompress.c:1429) | assertion/abort | [ ] |
| 1250 | `ZSTD_decompressContinue` | `RETURN_ERROR(GENERIC, "impossible to reach"); /* some compilers require default to do something */` (c_src/src/decompress/zstd_decompress.c:1430) | exact return/error shown | [ ] |
| 1251 | `ZSTD_loadDEntropy` | `RETURN_ERROR_IF(dictSize <= 8, dictionary_corrupted, "dict is too small");` (c_src/src/decompress/zstd_decompress.c:1458) | exact return/error shown | [ ] |
| 1252 | `ZSTD_loadDEntropy` | `assert(MEM_readLE32(dict) == ZSTD_MAGIC_DICTIONARY); /* dict must be valid */` (c_src/src/decompress/zstd_decompress.c:1459) | assertion/abort | [ ] |
| 1253 | `ZSTD_loadDEntropy` | `ZSTD_STATIC_ASSERT(offsetof(ZSTD_entropyDTables_t, OFTable) == offsetof(ZSTD_entropyDTables_t, LLTable) + sizeof(entropy->LLTable));` (c_src/src/decompress/zstd_decompress.c:1462) | exact return/error shown | [ ] |
| 1254 | `ZSTD_loadDEntropy` | `ZSTD_STATIC_ASSERT(offsetof(ZSTD_entropyDTables_t, MLTable) == offsetof(ZSTD_entropyDTables_t, OFTable) + sizeof(entropy->OFTable));` (c_src/src/decompress/zstd_decompress.c:1463) | exact return/error shown | [ ] |
| 1255 | `ZSTD_loadDEntropy` | `ZSTD_STATIC_ASSERT(sizeof(entropy->LLTable) + sizeof(entropy->OFTable) + sizeof(entropy->MLTable) >= HUF_DECOMPRESS_WORKSPACE_SIZE);` (c_src/src/decompress/zstd_decompress.c:1464) | exact return/error shown | [ ] |
| 1256 | `ZSTD_loadDEntropy` | `RETURN_ERROR_IF(HUF_isError(hSize), dictionary_corrupted, "");` (c_src/src/decompress/zstd_decompress.c:1477) | exact return/error shown | [ ] |
| 1257 | `ZSTD_loadDEntropy` | `RETURN_ERROR_IF(FSE_isError(offcodeHeaderSize), dictionary_corrupted, "");` (c_src/src/decompress/zstd_decompress.c:1484) | exact return/error shown | [ ] |
| 1258 | `ZSTD_loadDEntropy` | `RETURN_ERROR_IF(offcodeMaxValue > MaxOff, dictionary_corrupted, "");` (c_src/src/decompress/zstd_decompress.c:1485) | exact return/error shown | [ ] |
| 1259 | `ZSTD_loadDEntropy` | `RETURN_ERROR_IF(offcodeLog > OffFSELog, dictionary_corrupted, "");` (c_src/src/decompress/zstd_decompress.c:1486) | exact return/error shown | [ ] |
| 1260 | `ZSTD_loadDEntropy` | `RETURN_ERROR_IF(FSE_isError(matchlengthHeaderSize), dictionary_corrupted, "");` (c_src/src/decompress/zstd_decompress.c:1499) | exact return/error shown | [ ] |
| 1261 | `ZSTD_loadDEntropy` | `RETURN_ERROR_IF(matchlengthMaxValue > MaxML, dictionary_corrupted, "");` (c_src/src/decompress/zstd_decompress.c:1500) | exact return/error shown | [ ] |
| 1262 | `ZSTD_loadDEntropy` | `RETURN_ERROR_IF(matchlengthLog > MLFSELog, dictionary_corrupted, "");` (c_src/src/decompress/zstd_decompress.c:1501) | exact return/error shown | [ ] |
| 1263 | `ZSTD_loadDEntropy` | `RETURN_ERROR_IF(FSE_isError(litlengthHeaderSize), dictionary_corrupted, "");` (c_src/src/decompress/zstd_decompress.c:1514) | exact return/error shown | [ ] |
| 1264 | `ZSTD_loadDEntropy` | `RETURN_ERROR_IF(litlengthMaxValue > MaxLL, dictionary_corrupted, "");` (c_src/src/decompress/zstd_decompress.c:1515) | exact return/error shown | [ ] |
| 1265 | `ZSTD_loadDEntropy` | `RETURN_ERROR_IF(litlengthLog > LLFSELog, dictionary_corrupted, "");` (c_src/src/decompress/zstd_decompress.c:1516) | exact return/error shown | [ ] |
| 1266 | `ZSTD_loadDEntropy` | `RETURN_ERROR_IF(dictPtr+12 > dictEnd, dictionary_corrupted, "");` (c_src/src/decompress/zstd_decompress.c:1526) | exact return/error shown | [ ] |
| 1267 | `ZSTD_loadDEntropy` | `RETURN_ERROR_IF(rep==0 \|\| rep > dictContentSize, dictionary_corrupted, "");` (c_src/src/decompress/zstd_decompress.c:1531) | exact return/error shown | [ ] |
| 1268 | `ZSTD_decompress_insertDictionary` | `RETURN_ERROR_IF(ZSTD_isError(eSize), dictionary_corrupted, "");` (c_src/src/decompress/zstd_decompress.c:1550) | exact return/error shown | [ ] |
| 1269 | `ZSTD_decompressBegin` | `assert(dctx != NULL);` (c_src/src/decompress/zstd_decompress.c:1562) | assertion/abort | [ ] |
| 1270 | `ZSTD_decompressBegin` | `ZSTD_STATIC_ASSERT(sizeof(dctx->entropy.rep) == sizeof(repStartValue));` (c_src/src/decompress/zstd_decompress.c:1579) | exact return/error shown | [ ] |
| 1271 | `ZSTD_decompressBegin_usingDict` | `FORWARD_IF_ERROR( ZSTD_decompressBegin(dctx) , "");` (c_src/src/decompress/zstd_decompress.c:1590) | exact return/error shown | [ ] |
| 1272 | `ZSTD_decompressBegin_usingDict` | `RETURN_ERROR_IF( ZSTD_isError(ZSTD_decompress_insertDictionary(dctx, dict, dictSize)), dictionary_corrupted, "");` (c_src/src/decompress/zstd_decompress.c:1592) | exact return/error shown | [ ] |
| 1273 | `ZSTD_decompressBegin_usingDDict` | `assert(dctx != NULL);` (c_src/src/decompress/zstd_decompress.c:1604) | assertion/abort | [ ] |
| 1274 | `ZSTD_decompressBegin_usingDDict` | `FORWARD_IF_ERROR( ZSTD_decompressBegin(dctx) , "");` (c_src/src/decompress/zstd_decompress.c:1613) | exact return/error shown | [ ] |
| 1275 | `ZSTD_DCtx_loadDictionary_advanced` | `RETURN_ERROR_IF(dctx->streamStage != zdss_init, stage_wrong, "");` (c_src/src/decompress/zstd_decompress.c:1704) | exact return/error shown | [ ] |
| 1276 | `ZSTD_DCtx_loadDictionary_advanced` | `RETURN_ERROR_IF(dctx->ddictLocal == NULL, memory_allocation, "NULL pointer!");` (c_src/src/decompress/zstd_decompress.c:1708) | exact return/error shown | [ ] |
| 1277 | `ZSTD_DCtx_refPrefix_advanced` | `FORWARD_IF_ERROR(ZSTD_DCtx_loadDictionary_advanced(dctx, prefix, prefixSize, ZSTD_dlm_byRef, dictContentType), "");` (c_src/src/decompress/zstd_decompress.c:1727) | exact return/error shown | [ ] |
| 1278 | `ZSTD_initDStream_usingDict` | `FORWARD_IF_ERROR( ZSTD_DCtx_reset(zds, ZSTD_reset_session_only) , "");` (c_src/src/decompress/zstd_decompress.c:1744) | exact return/error shown | [ ] |
| 1279 | `ZSTD_initDStream_usingDict` | `FORWARD_IF_ERROR( ZSTD_DCtx_loadDictionary(zds, dict, dictSize) , "");` (c_src/src/decompress/zstd_decompress.c:1745) | exact return/error shown | [ ] |
| 1280 | `ZSTD_initDStream` | `FORWARD_IF_ERROR(ZSTD_DCtx_reset(zds, ZSTD_reset_session_only), "");` (c_src/src/decompress/zstd_decompress.c:1753) | exact return/error shown | [ ] |
| 1281 | `ZSTD_initDStream` | `FORWARD_IF_ERROR(ZSTD_DCtx_refDDict(zds, NULL), "");` (c_src/src/decompress/zstd_decompress.c:1754) | exact return/error shown | [ ] |
| 1282 | `ZSTD_initDStream_usingDDict` | `FORWARD_IF_ERROR( ZSTD_DCtx_reset(dctx, ZSTD_reset_session_only) , "");` (c_src/src/decompress/zstd_decompress.c:1764) | exact return/error shown | [ ] |
| 1283 | `ZSTD_initDStream_usingDDict` | `FORWARD_IF_ERROR( ZSTD_DCtx_refDDict(dctx, ddict) , "");` (c_src/src/decompress/zstd_decompress.c:1765) | exact return/error shown | [ ] |
| 1284 | `ZSTD_resetDStream` | `FORWARD_IF_ERROR(ZSTD_DCtx_reset(dctx, ZSTD_reset_session_only), "");` (c_src/src/decompress/zstd_decompress.c:1775) | exact return/error shown | [ ] |
| 1285 | `ZSTD_DCtx_refDDict` | `RETURN_ERROR_IF(dctx->streamStage != zdss_init, stage_wrong, "");` (c_src/src/decompress/zstd_decompress.c:1782) | exact return/error shown | [ ] |
| 1286 | `ZSTD_DCtx_refDDict` | `RETURN_ERROR(memory_allocation, "Failed to allocate memory for hash set!");` (c_src/src/decompress/zstd_decompress.c:1791) | exact return/error shown | [ ] |
| 1287 | `ZSTD_DCtx_refDDict` | `assert(!dctx->staticSize); /* Impossible: ddictSet cannot have been allocated if static dctx */` (c_src/src/decompress/zstd_decompress.c:1794) | assertion/abort | [ ] |
| 1288 | `ZSTD_DCtx_refDDict` | `FORWARD_IF_ERROR(ZSTD_DDictHashSet_addDDict(dctx->ddictSet, ddict, dctx->customMem), "");` (c_src/src/decompress/zstd_decompress.c:1795) | exact return/error shown | [ ] |
| 1289 | `ZSTD_DCtx_setMaxWindowSize` | `RETURN_ERROR_IF(dctx->streamStage != zdss_init, stage_wrong, "");` (c_src/src/decompress/zstd_decompress.c:1809) | exact return/error shown | [ ] |
| 1290 | `ZSTD_DCtx_setMaxWindowSize` | `RETURN_ERROR_IF(maxWindowSize < min, parameter_outOfBound, "");` (c_src/src/decompress/zstd_decompress.c:1810) | exact return/error shown | [ ] |
| 1291 | `ZSTD_DCtx_setMaxWindowSize` | `RETURN_ERROR_IF(maxWindowSize > max, parameter_outOfBound, "");` (c_src/src/decompress/zstd_decompress.c:1811) | exact return/error shown | [ ] |
| 1292 | `ZSTD_dParam_getBounds` | `ZSTD_STATIC_ASSERT(ZSTD_f_zstd1 < ZSTD_f_zstd1_magicless);` (c_src/src/decompress/zstd_decompress.c:1832) | exact return/error shown | [ ] |
| 1293 | `<file scope/macro>` | `RETURN_ERROR_IF(!ZSTD_dParam_withinBounds(p, v), parameter_outOfBound, ""); \` (c_src/src/decompress/zstd_decompress.c:1874) | exact return/error shown | [ ] |
| 1294 | `ZSTD_DCtx_getParameter` | `RETURN_ERROR(parameter_unsupported, "");` (c_src/src/decompress/zstd_decompress.c:1903) | exact return/error shown | [ ] |
| 1295 | `ZSTD_DCtx_setParameter` | `RETURN_ERROR_IF(dctx->streamStage != zdss_init, stage_wrong, "");` (c_src/src/decompress/zstd_decompress.c:1908) | exact return/error shown | [ ] |
| 1296 | `ZSTD_DCtx_setParameter` | `RETURN_ERROR(parameter_unsupported, "Static dctx does not support multiple DDicts!");` (c_src/src/decompress/zstd_decompress.c:1930) | exact return/error shown | [ ] |
| 1297 | `ZSTD_DCtx_setParameter` | `RETURN_ERROR(parameter_unsupported, "");` (c_src/src/decompress/zstd_decompress.c:1944) | exact return/error shown | [ ] |
| 1298 | `ZSTD_DCtx_reset` | `RETURN_ERROR_IF(dctx->streamStage != zdss_init, stage_wrong, "");` (c_src/src/decompress/zstd_decompress.c:1957) | exact return/error shown | [ ] |
| 1299 | `ZSTD_decodingBufferSize_internal` | `RETURN_ERROR_IF((unsigned long long)minRBSize != neededSize, frameParameter_windowTooLarge, "");` (c_src/src/decompress/zstd_decompress.c:1983) | exact return/error shown | [ ] |
| 1300 | `ZSTD_estimateDStreamSize_fromFrame` | `RETURN_ERROR_IF(err>0, srcSize_wrong, "");` (c_src/src/decompress/zstd_decompress.c:2007) | exact return/error shown | [ ] |
| 1301 | `ZSTD_estimateDStreamSize_fromFrame` | `RETURN_ERROR_IF(zfh.windowSize > windowSizeMax, frameParameter_windowTooLarge, "");` (c_src/src/decompress/zstd_decompress.c:2008) | exact return/error shown | [ ] |
| 1302 | `ZSTD_checkOutBuffer` | `RETURN_ERROR(dstBuffer_wrong, "ZSTD_d_stableOutBuffer enabled but output differs!");` (c_src/src/decompress/zstd_decompress.c:2049) | exact return/error shown | [ ] |
| 1303 | `ZSTD_decompressContinueStream` | `FORWARD_IF_ERROR(decodedSize, "");` (c_src/src/decompress/zstd_decompress.c:2065) | exact return/error shown | [ ] |
| 1304 | `ZSTD_decompressContinueStream` | `FORWARD_IF_ERROR(decodedSize, "");` (c_src/src/decompress/zstd_decompress.c:2076) | exact return/error shown | [ ] |
| 1305 | `ZSTD_decompressContinueStream` | `assert(*op <= oend);` (c_src/src/decompress/zstd_decompress.c:2080) | assertion/abort | [ ] |
| 1306 | `ZSTD_decompressContinueStream` | `assert(zds->outBufferMode == ZSTD_bm_stable);` (c_src/src/decompress/zstd_decompress.c:2081) | assertion/abort | [ ] |
| 1307 | `ZSTD_decompressStream` | `assert(zds != NULL);` (c_src/src/decompress/zstd_decompress.c:2099) | assertion/abort | [ ] |
| 1308 | `ZSTD_decompressStream` | `RETURN_ERROR_IF( input->pos > input->size, srcSize_wrong, "forbidden. in: pos: %u vs size: %u", (U32)input->pos, (U32)input->size);` (c_src/src/decompress/zstd_decompress.c:2100) | exact return/error shown | [ ] |
| 1309 | `ZSTD_decompressStream` | `RETURN_ERROR_IF( output->pos > output->size, dstSize_tooSmall, "forbidden. out: pos: %u vs size: %u", (U32)output->pos, (U32)output->size);` (c_src/src/decompress/zstd_decompress.c:2105) | exact return/error shown | [ ] |
| 1310 | `ZSTD_decompressStream` | `FORWARD_IF_ERROR(ZSTD_checkOutBuffer(zds, output), "");` (c_src/src/decompress/zstd_decompress.c:2111) | exact return/error shown | [ ] |
| 1311 | `ZSTD_decompressStream` | `RETURN_ERROR_IF(zds->staticSize, memory_allocation, "legacy support is incompatible with static dctx");` (c_src/src/decompress/zstd_decompress.c:2131) | exact return/error shown | [ ] |
| 1312 | `ZSTD_decompressStream` | `RETURN_ERROR_IF(zds->staticSize, memory_allocation, "legacy support is incompatible with static dctx");` (c_src/src/decompress/zstd_decompress.c:2150) | exact return/error shown | [ ] |
| 1313 | `ZSTD_decompressStream` | `FORWARD_IF_ERROR(ZSTD_initLegacyStream(&zds->legacyContext, zds->previousLegacyVersion, legacyVersion, dict, dictSize), "");` (c_src/src/decompress/zstd_decompress.c:2152) | exact return/error shown | [ ] |
| 1314 | `ZSTD_decompressStream` | `assert(iend >= ip);` (c_src/src/decompress/zstd_decompress.c:2166) | assertion/abort | [ ] |
| 1315 | `ZSTD_decompressStream` | `FORWARD_IF_ERROR( ZSTD_getFrameHeader_advanced(&zds->fParams, zds->headerBuffer, zds->lhSize, zds->format), "First few bytes detected incorrect" );` (c_src/src/decompress/zstd_decompress.c:2174) | exact return/error shown | [ ] |
| 1316 | `ZSTD_decompressStream` | `assert(ip != NULL);` (c_src/src/decompress/zstd_decompress.c:2180) | assertion/abort | [ ] |
| 1317 | `ZSTD_decompressStream` | `assert(istart != NULL);` (c_src/src/decompress/zstd_decompress.c:2195) | assertion/abort | [ ] |
| 1318 | `ZSTD_decompressStream` | `RETURN_ERROR(dstSize_tooSmall, "ZSTD_obm_stable passed but ZSTD_outBuffer is too small");` (c_src/src/decompress/zstd_decompress.c:2209) | exact return/error shown | [ ] |
| 1319 | `ZSTD_decompressStream` | `FORWARD_IF_ERROR(ZSTD_decompressBegin_usingDDict(zds, ZSTD_getDDict(zds)), "");` (c_src/src/decompress/zstd_decompress.c:2214) | exact return/error shown | [ ] |
| 1320 | `ZSTD_decompressStream` | `FORWARD_IF_ERROR(ZSTD_decodeFrameHeader(zds, zds->headerBuffer, zds->lhSize), "");` (c_src/src/decompress/zstd_decompress.c:2221) | exact return/error shown | [ ] |
| 1321 | `ZSTD_decompressStream` | `RETURN_ERROR_IF(zds->fParams.windowSize > zds->maxWindowSize, frameParameter_windowTooLarge, "");` (c_src/src/decompress/zstd_decompress.c:2231) | exact return/error shown | [ ] |
| 1322 | `ZSTD_decompressStream` | `assert(zds->staticSize >= sizeof(ZSTD_DCtx)); /* controlled at init */` (c_src/src/decompress/zstd_decompress.c:2255) | assertion/abort | [ ] |
| 1323 | `ZSTD_decompressStream` | `RETURN_ERROR_IF( bufferSize > zds->staticSize - sizeof(ZSTD_DCtx), memory_allocation, "");` (c_src/src/decompress/zstd_decompress.c:2256) | exact return/error shown | [ ] |
| 1324 | `ZSTD_decompressStream` | `RETURN_ERROR_IF(zds->inBuff == NULL, memory_allocation, "");` (c_src/src/decompress/zstd_decompress.c:2264) | exact return/error shown | [ ] |
| 1325 | `ZSTD_decompressStream` | `FORWARD_IF_ERROR(ZSTD_decompressContinueStream(zds, &op, oend, ip, neededInSize), "");` (c_src/src/decompress/zstd_decompress.c:2283) | exact return/error shown | [ ] |
| 1326 | `ZSTD_decompressStream` | `assert(ip != NULL);` (c_src/src/decompress/zstd_decompress.c:2284) | assertion/abort | [ ] |
| 1327 | `ZSTD_decompressStream` | `assert(neededInSize == ZSTD_nextSrcSizeToDecompressWithInputSize(zds, (size_t)(iend - ip)));` (c_src/src/decompress/zstd_decompress.c:2299) | assertion/abort | [ ] |
| 1328 | `ZSTD_decompressStream` | `RETURN_ERROR_IF(toLoad > zds->inBuffSize - zds->inPos, corruption_detected, "should never happen");` (c_src/src/decompress/zstd_decompress.c:2303) | exact return/error shown | [ ] |
| 1329 | `ZSTD_decompressStream` | `FORWARD_IF_ERROR(ZSTD_decompressContinueStream(zds, &op, oend, zds->inBuff, neededInSize), "");` (c_src/src/decompress/zstd_decompress.c:2317) | exact return/error shown | [ ] |
| 1330 | `ZSTD_decompressStream` | `assert(0); /* impossible */` (c_src/src/decompress/zstd_decompress.c:2345) | assertion/abort | [ ] |
| 1331 | `ZSTD_decompressStream` | `RETURN_ERROR(GENERIC, "impossible to reach"); /* some compilers require default to do something */` (c_src/src/decompress/zstd_decompress.c:2346) | exact return/error shown | [ ] |
| 1332 | `ZSTD_decompressStream` | `RETURN_ERROR_IF(op==oend, noForwardProgress_destFull, "");` (c_src/src/decompress/zstd_decompress.c:2359) | exact return/error shown | [ ] |
| 1333 | `ZSTD_decompressStream` | `RETURN_ERROR_IF(ip==iend, noForwardProgress_inputEmpty, "");` (c_src/src/decompress/zstd_decompress.c:2360) | exact return/error shown | [ ] |
| 1334 | `ZSTD_decompressStream` | `assert(0);` (c_src/src/decompress/zstd_decompress.c:2361) | assertion/abort | [ ] |
| 1335 | `ZSTD_decompressStream` | `assert(zds->inPos <= nextSrcSizeHint);` (c_src/src/decompress/zstd_decompress.c:2386) | assertion/abort | [ ] |
| 1336 | `ZSTD_blockSizeMax` | `assert(blockSizeMax <= ZSTD_BLOCKSIZE_MAX);` (c_src/src/decompress/zstd_decompress_block.c:57) | assertion/abort | [ ] |
| 1337 | `ZSTD_getcBlockSize` | `RETURN_ERROR_IF(srcSize < ZSTD_blockHeaderSize, srcSize_wrong, "");` (c_src/src/decompress/zstd_decompress_block.c:66) | exact return/error shown | [ ] |
| 1338 | `ZSTD_getcBlockSize` | `RETURN_ERROR_IF(bpPtr->blockType == bt_reserved, corruption_detected, "");` (c_src/src/decompress/zstd_decompress_block.c:74) | exact return/error shown | [ ] |
| 1339 | `ZSTD_allocateLiteralsBuffer` | `assert(litSize <= blockSizeMax);` (c_src/src/decompress/zstd_decompress_block.c:84) | assertion/abort | [ ] |
| 1340 | `ZSTD_allocateLiteralsBuffer` | `assert(dctx->isFrameDecompression \|\| streaming == not_streaming);` (c_src/src/decompress/zstd_decompress_block.c:85) | assertion/abort | [ ] |
| 1341 | `ZSTD_allocateLiteralsBuffer` | `assert(expectedWriteSize <= blockSizeMax);` (c_src/src/decompress/zstd_decompress_block.c:86) | assertion/abort | [ ] |
| 1342 | `ZSTD_allocateLiteralsBuffer` | `assert(blockSizeMax > ZSTD_LITBUFFEREXTRASIZE);` (c_src/src/decompress/zstd_decompress_block.c:104) | assertion/abort | [ ] |
| 1343 | `ZSTD_allocateLiteralsBuffer` | `assert(dctx->litBufferEnd <= (BYTE*)dst + expectedWriteSize);` (c_src/src/decompress/zstd_decompress_block.c:122) | assertion/abort | [ ] |
| 1344 | `ZSTD_decodeLiteralsBlock` | `RETURN_ERROR_IF(srcSize < MIN_CBLOCK_SIZE, corruption_detected, "");` (c_src/src/decompress/zstd_decompress_block.c:139) | exact return/error shown | [ ] |
| 1345 | `ZSTD_decodeLiteralsBlock` | `RETURN_ERROR_IF(dctx->litEntropy==0, dictionary_corrupted, "");` (c_src/src/decompress/zstd_decompress_block.c:149) | exact return/error shown | [ ] |
| 1346 | `ZSTD_decodeLiteralsBlock` | `RETURN_ERROR_IF(srcSize < 5, corruption_detected, "srcSize >= MIN_CBLOCK_SIZE == 2; here we need up to 5 for case 3");` (c_src/src/decompress/zstd_decompress_block.c:153) | exact return/error shown | [ ] |
| 1347 | `ZSTD_decodeLiteralsBlock` | `RETURN_ERROR_IF(litSize > 0 && dst == NULL, dstSize_tooSmall, "NULL not handled");` (c_src/src/decompress/zstd_decompress_block.c:185) | exact return/error shown | [ ] |
| 1348 | `ZSTD_decodeLiteralsBlock` | `RETURN_ERROR_IF(litSize > blockSizeMax, corruption_detected, "");` (c_src/src/decompress/zstd_decompress_block.c:186) | exact return/error shown | [ ] |
| 1349 | `ZSTD_decodeLiteralsBlock` | `RETURN_ERROR_IF(litSize < MIN_LITERALS_FOR_4_STREAMS, literals_headerWrong, "Not enough literals (%zu) for the 4-streams mode (min %u)", litSize, MIN_LITERALS_FOR_4_STREAMS);` (c_src/src/decompress/zstd_decompress_block.c:188) | exact return/error shown | [ ] |
| 1350 | `ZSTD_decodeLiteralsBlock` | `RETURN_ERROR_IF(litCSize + lhSize > srcSize, corruption_detected, "");` (c_src/src/decompress/zstd_decompress_block.c:191) | exact return/error shown | [ ] |
| 1351 | `ZSTD_decodeLiteralsBlock` | `RETURN_ERROR_IF(expectedWriteSize < litSize , dstSize_tooSmall, "");` (c_src/src/decompress/zstd_decompress_block.c:192) | exact return/error shown | [ ] |
| 1352 | `ZSTD_decodeLiteralsBlock` | `assert(litSize >= MIN_LITERALS_FOR_4_STREAMS);` (c_src/src/decompress/zstd_decompress_block.c:206) | assertion/abort | [ ] |
| 1353 | `ZSTD_decodeLiteralsBlock` | `assert(litSize > ZSTD_LITBUFFEREXTRASIZE);` (c_src/src/decompress/zstd_decompress_block.c:233) | assertion/abort | [ ] |
| 1354 | `ZSTD_decodeLiteralsBlock` | `assert(dctx->litBufferEnd <= (BYTE*)dst + blockSizeMax);` (c_src/src/decompress/zstd_decompress_block.c:238) | assertion/abort | [ ] |
| 1355 | `ZSTD_decodeLiteralsBlock` | `RETURN_ERROR_IF(HUF_isError(hufSuccess), corruption_detected, "");` (c_src/src/decompress/zstd_decompress_block.c:241) | exact return/error shown | [ ] |
| 1356 | `ZSTD_decodeLiteralsBlock` | `RETURN_ERROR_IF(srcSize<3, corruption_detected, "srcSize >= MIN_CBLOCK_SIZE == 2; here we need lhSize = 3");` (c_src/src/decompress/zstd_decompress_block.c:266) | exact return/error shown | [ ] |
| 1357 | `ZSTD_decodeLiteralsBlock` | `RETURN_ERROR_IF(litSize > 0 && dst == NULL, dstSize_tooSmall, "NULL not handled");` (c_src/src/decompress/zstd_decompress_block.c:271) | exact return/error shown | [ ] |
| 1358 | `ZSTD_decodeLiteralsBlock` | `RETURN_ERROR_IF(litSize > blockSizeMax, corruption_detected, "");` (c_src/src/decompress/zstd_decompress_block.c:272) | exact return/error shown | [ ] |
| 1359 | `ZSTD_decodeLiteralsBlock` | `RETURN_ERROR_IF(expectedWriteSize < litSize, dstSize_tooSmall, "");` (c_src/src/decompress/zstd_decompress_block.c:273) | exact return/error shown | [ ] |
| 1360 | `ZSTD_decodeLiteralsBlock` | `RETURN_ERROR_IF(litSize+lhSize > srcSize, corruption_detected, "");` (c_src/src/decompress/zstd_decompress_block.c:276) | exact return/error shown | [ ] |
| 1361 | `ZSTD_decodeLiteralsBlock` | `RETURN_ERROR_IF(srcSize<3, corruption_detected, "srcSize >= MIN_CBLOCK_SIZE == 2; here we need lhSize+1 = 3");` (c_src/src/decompress/zstd_decompress_block.c:310) | exact return/error shown | [ ] |
| 1362 | `ZSTD_decodeLiteralsBlock` | `RETURN_ERROR_IF(srcSize<4, corruption_detected, "srcSize >= MIN_CBLOCK_SIZE == 2; here we need lhSize+1 = 4");` (c_src/src/decompress/zstd_decompress_block.c:315) | exact return/error shown | [ ] |
| 1363 | `ZSTD_decodeLiteralsBlock` | `RETURN_ERROR_IF(litSize > 0 && dst == NULL, dstSize_tooSmall, "NULL not handled");` (c_src/src/decompress/zstd_decompress_block.c:319) | exact return/error shown | [ ] |
| 1364 | `ZSTD_decodeLiteralsBlock` | `RETURN_ERROR_IF(litSize > blockSizeMax, corruption_detected, "");` (c_src/src/decompress/zstd_decompress_block.c:320) | exact return/error shown | [ ] |
| 1365 | `ZSTD_decodeLiteralsBlock` | `RETURN_ERROR_IF(expectedWriteSize < litSize, dstSize_tooSmall, "");` (c_src/src/decompress/zstd_decompress_block.c:321) | exact return/error shown | [ ] |
| 1366 | `ZSTD_decodeLiteralsBlock` | `RETURN_ERROR(corruption_detected, "impossible");` (c_src/src/decompress/zstd_decompress_block.c:337) | exact return/error shown | [ ] |
| 1367 | `ZSTD_buildSeqTable_rle` | `assert(nbAddBits < 255);` (c_src/src/decompress/zstd_decompress_block.c:474) | assertion/abort | [ ] |
| 1368 | `ZSTD_buildFSETable_body` | `assert(maxSymbolValue <= MaxSeq);` (c_src/src/decompress/zstd_decompress_block.c:500) | assertion/abort | [ ] |
| 1369 | `ZSTD_buildFSETable_body` | `assert(tableLog <= MaxFSELog);` (c_src/src/decompress/zstd_decompress_block.c:501) | assertion/abort | [ ] |
| 1370 | `ZSTD_buildFSETable_body` | `assert(wkspSize >= ZSTD_BUILD_FSE_TABLE_WKSP_SIZE);` (c_src/src/decompress/zstd_decompress_block.c:502) | assertion/abort | [ ] |
| 1371 | `ZSTD_buildFSETable_body` | `assert(normalizedCounter[s]>=0);` (c_src/src/decompress/zstd_decompress_block.c:516) | assertion/abort | [ ] |
| 1372 | `ZSTD_buildFSETable_body` | `assert(tableSize <= 512);` (c_src/src/decompress/zstd_decompress_block.c:523) | assertion/abort | [ ] |
| 1373 | `ZSTD_buildFSETable_body` | `assert(n>=0);` (c_src/src/decompress/zstd_decompress_block.c:550) | assertion/abort | [ ] |
| 1374 | `ZSTD_buildFSETable_body` | `assert(tableSize % unroll == 0); /* FSE_MIN_TABLELOG is 5 */` (c_src/src/decompress/zstd_decompress_block.c:564) | assertion/abort | [ ] |
| 1375 | `ZSTD_buildFSETable_body` | `assert(position == 0);` (c_src/src/decompress/zstd_decompress_block.c:573) | assertion/abort | [ ] |
| 1376 | `ZSTD_buildFSETable_body` | `assert(position == 0); /* position must reach all cells once, otherwise normalizedCounter is incorrect */` (c_src/src/decompress/zstd_decompress_block.c:587) | assertion/abort | [ ] |
| 1377 | `ZSTD_buildFSETable_body` | `assert(nbAdditionalBits[symbol] < 255);` (c_src/src/decompress/zstd_decompress_block.c:598) | assertion/abort | [ ] |
| 1378 | `ZSTD_buildSeqTable` | `RETURN_ERROR_IF(!srcSize, srcSize_wrong, "");` (c_src/src/decompress/zstd_decompress_block.c:658) | exact return/error shown | [ ] |
| 1379 | `ZSTD_buildSeqTable` | `RETURN_ERROR_IF((*(const BYTE*)src) > max, corruption_detected, "");` (c_src/src/decompress/zstd_decompress_block.c:659) | exact return/error shown | [ ] |
| 1380 | `ZSTD_buildSeqTable` | `RETURN_ERROR_IF(!flagRepeatTable, corruption_detected, "");` (c_src/src/decompress/zstd_decompress_block.c:671) | exact return/error shown | [ ] |
| 1381 | `ZSTD_buildSeqTable` | `RETURN_ERROR_IF(FSE_isError(headerSize), corruption_detected, "");` (c_src/src/decompress/zstd_decompress_block.c:683) | exact return/error shown | [ ] |
| 1382 | `ZSTD_buildSeqTable` | `RETURN_ERROR_IF(tableLog > maxLog, corruption_detected, "");` (c_src/src/decompress/zstd_decompress_block.c:684) | exact return/error shown | [ ] |
| 1383 | `ZSTD_buildSeqTable` | `assert(0);` (c_src/src/decompress/zstd_decompress_block.c:690) | assertion/abort | [ ] |
| 1384 | `ZSTD_buildSeqTable` | `RETURN_ERROR(GENERIC, "impossible");` (c_src/src/decompress/zstd_decompress_block.c:691) | exact return/error shown | [ ] |
| 1385 | `ZSTD_decodeSeqHeaders` | `RETURN_ERROR_IF(srcSize < MIN_SEQUENCES_SIZE, srcSize_wrong, "");` (c_src/src/decompress/zstd_decompress_block.c:705) | exact return/error shown | [ ] |
| 1386 | `ZSTD_decodeSeqHeaders` | `RETURN_ERROR_IF(ip+2 > iend, srcSize_wrong, "");` (c_src/src/decompress/zstd_decompress_block.c:711) | exact return/error shown | [ ] |
| 1387 | `ZSTD_decodeSeqHeaders` | `RETURN_ERROR_IF(ip >= iend, srcSize_wrong, "");` (c_src/src/decompress/zstd_decompress_block.c:715) | exact return/error shown | [ ] |
| 1388 | `ZSTD_decodeSeqHeaders` | `RETURN_ERROR_IF(ip != iend, corruption_detected, "extraneous data present in the Sequences section");` (c_src/src/decompress/zstd_decompress_block.c:723) | exact return/error shown | [ ] |
| 1389 | `ZSTD_decodeSeqHeaders` | `RETURN_ERROR_IF(ip+1 > iend, srcSize_wrong, ""); /* minimum possible size: 1 byte for symbol encoding types */` (c_src/src/decompress/zstd_decompress_block.c:729) | exact return/error shown | [ ] |
| 1390 | `ZSTD_decodeSeqHeaders` | `RETURN_ERROR_IF(*ip & 3, corruption_detected, ""); /* The last field, Reserved, must be all-zeroes. */` (c_src/src/decompress/zstd_decompress_block.c:730) | exact return/error shown | [ ] |
| 1391 | `ZSTD_decodeSeqHeaders` | `RETURN_ERROR_IF(ZSTD_isError(llhSize), corruption_detected, "ZSTD_buildSeqTable failed");` (c_src/src/decompress/zstd_decompress_block.c:745) | exact return/error shown | [ ] |
| 1392 | `ZSTD_decodeSeqHeaders` | `RETURN_ERROR_IF(ZSTD_isError(ofhSize), corruption_detected, "ZSTD_buildSeqTable failed");` (c_src/src/decompress/zstd_decompress_block.c:757) | exact return/error shown | [ ] |
| 1393 | `ZSTD_decodeSeqHeaders` | `RETURN_ERROR_IF(ZSTD_isError(mlhSize), corruption_detected, "ZSTD_buildSeqTable failed");` (c_src/src/decompress/zstd_decompress_block.c:769) | exact return/error shown | [ ] |
| 1394 | `ZSTD_overlapCopy8` | `assert(*ip <= *op);` (c_src/src/decompress/zstd_decompress_block.c:805) | assertion/abort | [ ] |
| 1395 | `ZSTD_overlapCopy8` | `assert(*op - *ip >= 8);` (c_src/src/decompress/zstd_decompress_block.c:823) | assertion/abort | [ ] |
| 1396 | `ZSTD_safecopy` | `assert((ovtype == ZSTD_no_overlap && (diff <= -8 \|\| diff >= 8 \|\| op >= oend_w)) \|\| (ovtype == ZSTD_overlap_src_before_dst && diff >= 0));` (c_src/src/decompress/zstd_decompress_block.c:841) | assertion/abort | [ ] |
| 1397 | `ZSTD_safecopy` | `assert(length >= 8);` (c_src/src/decompress/zstd_decompress_block.c:851) | assertion/abort | [ ] |
| 1398 | `ZSTD_safecopy` | `assert(op - ip >= 8);` (c_src/src/decompress/zstd_decompress_block.c:854) | assertion/abort | [ ] |
| 1399 | `ZSTD_safecopy` | `assert(op <= oend);` (c_src/src/decompress/zstd_decompress_block.c:855) | assertion/abort | [ ] |
| 1400 | `ZSTD_safecopy` | `assert(oend > oend_w);` (c_src/src/decompress/zstd_decompress_block.c:865) | assertion/abort | [ ] |
| 1401 | `ZSTD_execSequenceEnd` | `RETURN_ERROR_IF(sequenceLength > (size_t)(oend - op), dstSize_tooSmall, "last match must fit within dstBuffer");` (c_src/src/decompress/zstd_decompress_block.c:919) | exact return/error shown | [ ] |
| 1402 | `ZSTD_execSequenceEnd` | `RETURN_ERROR_IF(sequence.litLength > (size_t)(litLimit - *litPtr), corruption_detected, "try to read beyond literal buffer");` (c_src/src/decompress/zstd_decompress_block.c:920) | exact return/error shown | [ ] |
| 1403 | `ZSTD_execSequenceEnd` | `assert(op < op + sequenceLength);` (c_src/src/decompress/zstd_decompress_block.c:921) | assertion/abort | [ ] |
| 1404 | `ZSTD_execSequenceEnd` | `assert(oLitEnd < op + sequenceLength);` (c_src/src/decompress/zstd_decompress_block.c:922) | assertion/abort | [ ] |
| 1405 | `ZSTD_execSequenceEnd` | `RETURN_ERROR_IF(sequence.offset > (size_t)(oLitEnd - virtualStart), corruption_detected, "");` (c_src/src/decompress/zstd_decompress_block.c:932) | exact return/error shown | [ ] |
| 1406 | `ZSTD_execSequenceEndSplitLitBuffer` | `RETURN_ERROR_IF(sequenceLength > (size_t)(oend - op), dstSize_tooSmall, "last match must fit within dstBuffer");` (c_src/src/decompress/zstd_decompress_block.c:967) | exact return/error shown | [ ] |
| 1407 | `ZSTD_execSequenceEndSplitLitBuffer` | `RETURN_ERROR_IF(sequence.litLength > (size_t)(litLimit - *litPtr), corruption_detected, "try to read beyond literal buffer");` (c_src/src/decompress/zstd_decompress_block.c:968) | exact return/error shown | [ ] |
| 1408 | `ZSTD_execSequenceEndSplitLitBuffer` | `assert(op < op + sequenceLength);` (c_src/src/decompress/zstd_decompress_block.c:969) | assertion/abort | [ ] |
| 1409 | `ZSTD_execSequenceEndSplitLitBuffer` | `assert(oLitEnd < op + sequenceLength);` (c_src/src/decompress/zstd_decompress_block.c:970) | assertion/abort | [ ] |
| 1410 | `ZSTD_execSequenceEndSplitLitBuffer` | `RETURN_ERROR_IF(op > *litPtr && op < *litPtr + sequence.litLength, dstSize_tooSmall, "output should not catch up to and overwrite literal buffer");` (c_src/src/decompress/zstd_decompress_block.c:973) | exact return/error shown | [ ] |
| 1411 | `ZSTD_execSequenceEndSplitLitBuffer` | `RETURN_ERROR_IF(sequence.offset > (size_t)(oLitEnd - virtualStart), corruption_detected, "");` (c_src/src/decompress/zstd_decompress_block.c:981) | exact return/error shown | [ ] |
| 1412 | `ZSTD_execSequence` | `assert(op != NULL /* Precondition */);` (c_src/src/decompress/zstd_decompress_block.c:1013) | assertion/abort | [ ] |
| 1413 | `ZSTD_execSequence` | `assert(oend_w < oend /* No underflow */);` (c_src/src/decompress/zstd_decompress_block.c:1014) | assertion/abort | [ ] |
| 1414 | `ZSTD_execSequence` | `assert(op <= oLitEnd /* No overflow */);` (c_src/src/decompress/zstd_decompress_block.c:1032) | assertion/abort | [ ] |
| 1415 | `ZSTD_execSequence` | `assert(oLitEnd < oMatchEnd /* Non-zero match & no overflow */);` (c_src/src/decompress/zstd_decompress_block.c:1033) | assertion/abort | [ ] |
| 1416 | `ZSTD_execSequence` | `assert(oMatchEnd <= oend /* No underflow */);` (c_src/src/decompress/zstd_decompress_block.c:1034) | assertion/abort | [ ] |
| 1417 | `ZSTD_execSequence` | `assert(iLitEnd <= litLimit /* Literal length is in bounds */);` (c_src/src/decompress/zstd_decompress_block.c:1035) | assertion/abort | [ ] |
| 1418 | `ZSTD_execSequence` | `assert(oLitEnd <= oend_w /* Can wildcopy literals */);` (c_src/src/decompress/zstd_decompress_block.c:1036) | assertion/abort | [ ] |
| 1419 | `ZSTD_execSequence` | `assert(oMatchEnd <= oend_w /* Can wildcopy matches */);` (c_src/src/decompress/zstd_decompress_block.c:1037) | assertion/abort | [ ] |
| 1420 | `ZSTD_execSequence` | `assert(WILDCOPY_OVERLENGTH >= 16);` (c_src/src/decompress/zstd_decompress_block.c:1043) | assertion/abort | [ ] |
| 1421 | `ZSTD_execSequence` | `RETURN_ERROR_IF(UNLIKELY(sequence.offset > (size_t)(oLitEnd - virtualStart)), corruption_detected, "");` (c_src/src/decompress/zstd_decompress_block.c:1054) | exact return/error shown | [ ] |
| 1422 | `ZSTD_execSequence` | `assert(op <= oMatchEnd);` (c_src/src/decompress/zstd_decompress_block.c:1069) | assertion/abort | [ ] |
| 1423 | `ZSTD_execSequence` | `assert(oMatchEnd <= oend_w);` (c_src/src/decompress/zstd_decompress_block.c:1070) | assertion/abort | [ ] |
| 1424 | `ZSTD_execSequence` | `assert(match >= prefixStart);` (c_src/src/decompress/zstd_decompress_block.c:1071) | assertion/abort | [ ] |
| 1425 | `ZSTD_execSequence` | `assert(sequence.matchLength >= 1);` (c_src/src/decompress/zstd_decompress_block.c:1072) | assertion/abort | [ ] |
| 1426 | `ZSTD_execSequence` | `assert(sequence.offset < WILDCOPY_VECLEN);` (c_src/src/decompress/zstd_decompress_block.c:1085) | assertion/abort | [ ] |
| 1427 | `ZSTD_execSequence` | `assert(op < oMatchEnd);` (c_src/src/decompress/zstd_decompress_block.c:1092) | assertion/abort | [ ] |
| 1428 | `ZSTD_execSequenceSplitLitBuffer` | `assert(op != NULL /* Precondition */);` (c_src/src/decompress/zstd_decompress_block.c:1111) | assertion/abort | [ ] |
| 1429 | `ZSTD_execSequenceSplitLitBuffer` | `assert(oend_w < oend /* No underflow */);` (c_src/src/decompress/zstd_decompress_block.c:1112) | assertion/abort | [ ] |
| 1430 | `ZSTD_execSequenceSplitLitBuffer` | `assert(op <= oLitEnd /* No overflow */);` (c_src/src/decompress/zstd_decompress_block.c:1125) | assertion/abort | [ ] |
| 1431 | `ZSTD_execSequenceSplitLitBuffer` | `assert(oLitEnd < oMatchEnd /* Non-zero match & no overflow */);` (c_src/src/decompress/zstd_decompress_block.c:1126) | assertion/abort | [ ] |
| 1432 | `ZSTD_execSequenceSplitLitBuffer` | `assert(oMatchEnd <= oend /* No underflow */);` (c_src/src/decompress/zstd_decompress_block.c:1127) | assertion/abort | [ ] |
| 1433 | `ZSTD_execSequenceSplitLitBuffer` | `assert(iLitEnd <= litLimit /* Literal length is in bounds */);` (c_src/src/decompress/zstd_decompress_block.c:1128) | assertion/abort | [ ] |
| 1434 | `ZSTD_execSequenceSplitLitBuffer` | `assert(oLitEnd <= oend_w /* Can wildcopy literals */);` (c_src/src/decompress/zstd_decompress_block.c:1129) | assertion/abort | [ ] |
| 1435 | `ZSTD_execSequenceSplitLitBuffer` | `assert(oMatchEnd <= oend_w /* Can wildcopy matches */);` (c_src/src/decompress/zstd_decompress_block.c:1130) | assertion/abort | [ ] |
| 1436 | `ZSTD_execSequenceSplitLitBuffer` | `assert(WILDCOPY_OVERLENGTH >= 16);` (c_src/src/decompress/zstd_decompress_block.c:1136) | assertion/abort | [ ] |
| 1437 | `ZSTD_execSequenceSplitLitBuffer` | `RETURN_ERROR_IF(UNLIKELY(sequence.offset > (size_t)(oLitEnd - virtualStart)), corruption_detected, "");` (c_src/src/decompress/zstd_decompress_block.c:1147) | exact return/error shown | [ ] |
| 1438 | `ZSTD_execSequenceSplitLitBuffer` | `assert(op <= oMatchEnd);` (c_src/src/decompress/zstd_decompress_block.c:1161) | assertion/abort | [ ] |
| 1439 | `ZSTD_execSequenceSplitLitBuffer` | `assert(oMatchEnd <= oend_w);` (c_src/src/decompress/zstd_decompress_block.c:1162) | assertion/abort | [ ] |
| 1440 | `ZSTD_execSequenceSplitLitBuffer` | `assert(match >= prefixStart);` (c_src/src/decompress/zstd_decompress_block.c:1163) | assertion/abort | [ ] |
| 1441 | `ZSTD_execSequenceSplitLitBuffer` | `assert(sequence.matchLength >= 1);` (c_src/src/decompress/zstd_decompress_block.c:1164) | assertion/abort | [ ] |
| 1442 | `ZSTD_execSequenceSplitLitBuffer` | `assert(sequence.offset < WILDCOPY_VECLEN);` (c_src/src/decompress/zstd_decompress_block.c:1177) | assertion/abort | [ ] |
| 1443 | `ZSTD_execSequenceSplitLitBuffer` | `assert(op < oMatchEnd);` (c_src/src/decompress/zstd_decompress_block.c:1184) | assertion/abort | [ ] |
| 1444 | `ZSTD_decodeSequence` | `assert(llBits <= MaxLLBits);` (c_src/src/decompress/zstd_decompress_block.c:1268) | assertion/abort | [ ] |
| 1445 | `ZSTD_decodeSequence` | `assert(mlBits <= MaxMLBits);` (c_src/src/decompress/zstd_decompress_block.c:1269) | assertion/abort | [ ] |
| 1446 | `ZSTD_decodeSequence` | `assert(ofBits <= MaxOff);` (c_src/src/decompress/zstd_decompress_block.c:1270) | assertion/abort | [ ] |
| 1447 | `ZSTD_decodeSequence` | `ZSTD_STATIC_ASSERT(ZSTD_lo_isLongOffset == 1);` (c_src/src/decompress/zstd_decompress_block.c:1280) | exact return/error shown | [ ] |
| 1448 | `ZSTD_decodeSequence` | `ZSTD_STATIC_ASSERT(LONG_OFFSETS_MAX_EXTRA_BITS_32 == 5);` (c_src/src/decompress/zstd_decompress_block.c:1281) | exact return/error shown | [ ] |
| 1449 | `ZSTD_decodeSequence` | `ZSTD_STATIC_ASSERT(STREAM_ACCUMULATOR_MIN_32 > LONG_OFFSETS_MAX_EXTRA_BITS_32);` (c_src/src/decompress/zstd_decompress_block.c:1282) | exact return/error shown | [ ] |
| 1450 | `ZSTD_decodeSequence` | `ZSTD_STATIC_ASSERT(STREAM_ACCUMULATOR_MIN_32 - LONG_OFFSETS_MAX_EXTRA_BITS_32 >= MaxMLBits);` (c_src/src/decompress/zstd_decompress_block.c:1283) | exact return/error shown | [ ] |
| 1451 | `ZSTD_decodeSequence` | `ZSTD_STATIC_ASSERT(16+LLFSELog+MLFSELog+OffFSELog < STREAM_ACCUMULATOR_MIN_64);` (c_src/src/decompress/zstd_decompress_block.c:1324) | exact return/error shown | [ ] |
| 1452 | `ZSTD_assertValidSequence` | `assert(op <= oend);` (c_src/src/decompress/zstd_decompress_block.c:1379) | assertion/abort | [ ] |
| 1453 | `ZSTD_assertValidSequence` | `assert((size_t)(oend - op) >= sequenceSize);` (c_src/src/decompress/zstd_decompress_block.c:1380) | assertion/abort | [ ] |
| 1454 | `ZSTD_assertValidSequence` | `assert(sequenceSize <= ZSTD_blockSizeMax(dctx));` (c_src/src/decompress/zstd_decompress_block.c:1381) | assertion/abort | [ ] |
| 1455 | `ZSTD_assertValidSequence` | `assert(seq.offset <= (size_t)(oLitEnd - virtualStart));` (c_src/src/decompress/zstd_decompress_block.c:1385) | assertion/abort | [ ] |
| 1456 | `ZSTD_assertValidSequence` | `assert(seq.offset <= windowSize + dictSize);` (c_src/src/decompress/zstd_decompress_block.c:1386) | assertion/abort | [ ] |
| 1457 | `ZSTD_assertValidSequence` | `assert(seq.offset <= windowSize);` (c_src/src/decompress/zstd_decompress_block.c:1389) | assertion/abort | [ ] |
| 1458 | `ZSTD_decompressSequences_bodySplitLitBuffer` | `RETURN_ERROR_IF( ERR_isError(BIT_initDStream(&seqState.DStream, ip, iend-ip)), corruption_detected, "");` (c_src/src/decompress/zstd_decompress_block.c:1425) | exact return/error shown | [ ] |
| 1459 | `ZSTD_decompressSequences_bodySplitLitBuffer` | `assert(dst != NULL);` (c_src/src/decompress/zstd_decompress_block.c:1431) | assertion/abort | [ ] |
| 1460 | `ZSTD_decompressSequences_bodySplitLitBuffer` | `ZSTD_STATIC_ASSERT( BIT_DStream_unfinished < BIT_DStream_completed && BIT_DStream_endOfBuffer < BIT_DStream_completed && BIT_DStream_completed < BIT_DStream_overflow);` (c_src/src/decompress/zstd_decompress_block.c:1433) | exact return/error shown | [ ] |
| 1461 | `ZSTD_decompressSequences_bodySplitLitBuffer` | `assert(!ZSTD_isError(oneSeqSize));` (c_src/src/decompress/zstd_decompress_block.c:1506) | assertion/abort | [ ] |
| 1462 | `ZSTD_decompressSequences_bodySplitLitBuffer` | `RETURN_ERROR_IF(leftoverLit > (size_t)(oend - op), dstSize_tooSmall, "remaining lit must fit within dstBuffer");` (c_src/src/decompress/zstd_decompress_block.c:1521) | exact return/error shown | [ ] |
| 1463 | `ZSTD_decompressSequences_bodySplitLitBuffer` | `assert(!ZSTD_isError(oneSeqSize));` (c_src/src/decompress/zstd_decompress_block.c:1531) | assertion/abort | [ ] |
| 1464 | `ZSTD_decompressSequences_bodySplitLitBuffer` | `assert(!ZSTD_isError(oneSeqSize));` (c_src/src/decompress/zstd_decompress_block.c:1567) | assertion/abort | [ ] |
| 1465 | `ZSTD_decompressSequences_bodySplitLitBuffer` | `RETURN_ERROR_IF(nbSeq, corruption_detected, "");` (c_src/src/decompress/zstd_decompress_block.c:1579) | exact return/error shown | [ ] |
| 1466 | `ZSTD_decompressSequences_bodySplitLitBuffer` | `RETURN_ERROR_IF(!BIT_endOfDStream(&seqState.DStream), corruption_detected, "");` (c_src/src/decompress/zstd_decompress_block.c:1581) | exact return/error shown | [ ] |
| 1467 | `ZSTD_decompressSequences_bodySplitLitBuffer` | `RETURN_ERROR_IF(lastLLSize > (size_t)(oend - op), dstSize_tooSmall, "");` (c_src/src/decompress/zstd_decompress_block.c:1591) | exact return/error shown | [ ] |
| 1468 | `ZSTD_decompressSequences_bodySplitLitBuffer` | `RETURN_ERROR_IF(lastLLSize > (size_t)(oend-op), dstSize_tooSmall, "");` (c_src/src/decompress/zstd_decompress_block.c:1603) | exact return/error shown | [ ] |
| 1469 | `ZSTD_decompressSequences_body` | `RETURN_ERROR_IF( ERR_isError(BIT_initDStream(&seqState.DStream, ip, iend - ip)), corruption_detected, "");` (c_src/src/decompress/zstd_decompress_block.c:1637) | exact return/error shown | [ ] |
| 1470 | `ZSTD_decompressSequences_body` | `assert(dst != NULL);` (c_src/src/decompress/zstd_decompress_block.c:1643) | assertion/abort | [ ] |
| 1471 | `ZSTD_decompressSequences_body` | `assert(!ZSTD_isError(oneSeqSize));` (c_src/src/decompress/zstd_decompress_block.c:1663) | assertion/abort | [ ] |
| 1472 | `ZSTD_decompressSequences_body` | `assert(nbSeq == 0);` (c_src/src/decompress/zstd_decompress_block.c:1673) | assertion/abort | [ ] |
| 1473 | `ZSTD_decompressSequences_body` | `RETURN_ERROR_IF(!BIT_endOfDStream(&seqState.DStream), corruption_detected, "");` (c_src/src/decompress/zstd_decompress_block.c:1674) | exact return/error shown | [ ] |
| 1474 | `ZSTD_decompressSequences_body` | `RETURN_ERROR_IF(lastLLSize > (size_t)(oend-op), dstSize_tooSmall, "");` (c_src/src/decompress/zstd_decompress_block.c:1682) | exact return/error shown | [ ] |
| 1475 | `ZSTD_decompressSequencesLong_body` | `assert(dst != NULL);` (c_src/src/decompress/zstd_decompress_block.c:1763) | assertion/abort | [ ] |
| 1476 | `ZSTD_decompressSequencesLong_body` | `assert(iend >= ip);` (c_src/src/decompress/zstd_decompress_block.c:1764) | assertion/abort | [ ] |
| 1477 | `ZSTD_decompressSequencesLong_body` | `RETURN_ERROR_IF( ERR_isError(BIT_initDStream(&seqState.DStream, ip, iend-ip)), corruption_detected, "");` (c_src/src/decompress/zstd_decompress_block.c:1765) | exact return/error shown | [ ] |
| 1478 | `ZSTD_decompressSequencesLong_body` | `RETURN_ERROR_IF(leftoverLit > (size_t)(oend - op), dstSize_tooSmall, "remaining lit must fit within dstBuffer");` (c_src/src/decompress/zstd_decompress_block.c:1788) | exact return/error shown | [ ] |
| 1479 | `ZSTD_decompressSequencesLong_body` | `assert(!ZSTD_isError(oneSeqSize));` (c_src/src/decompress/zstd_decompress_block.c:1798) | assertion/abort | [ ] |
| 1480 | `ZSTD_decompressSequencesLong_body` | `assert(!ZSTD_isError(oneSeqSize));` (c_src/src/decompress/zstd_decompress_block.c:1814) | assertion/abort | [ ] |
| 1481 | `ZSTD_decompressSequencesLong_body` | `RETURN_ERROR_IF(!BIT_endOfDStream(&seqState.DStream), corruption_detected, "");` (c_src/src/decompress/zstd_decompress_block.c:1824) | exact return/error shown | [ ] |
| 1482 | `ZSTD_decompressSequencesLong_body` | `RETURN_ERROR_IF(leftoverLit > (size_t)(oend - op), dstSize_tooSmall, "remaining lit must fit within dstBuffer");` (c_src/src/decompress/zstd_decompress_block.c:1833) | exact return/error shown | [ ] |
| 1483 | `ZSTD_decompressSequencesLong_body` | `assert(!ZSTD_isError(oneSeqSize));` (c_src/src/decompress/zstd_decompress_block.c:1843) | assertion/abort | [ ] |
| 1484 | `ZSTD_decompressSequencesLong_body` | `assert(!ZSTD_isError(oneSeqSize));` (c_src/src/decompress/zstd_decompress_block.c:1856) | assertion/abort | [ ] |
| 1485 | `ZSTD_decompressSequencesLong_body` | `RETURN_ERROR_IF(lastLLSize > (size_t)(oend - op), dstSize_tooSmall, "");` (c_src/src/decompress/zstd_decompress_block.c:1871) | exact return/error shown | [ ] |
| 1486 | `ZSTD_decompressSequencesLong_body` | `RETURN_ERROR_IF(lastLLSize > (size_t)(oend-op), dstSize_tooSmall, "");` (c_src/src/decompress/zstd_decompress_block.c:1880) | exact return/error shown | [ ] |
| 1487 | `ZSTD_getOffsetInfo` | `assert(max <= (1 << OffFSELog)); /* max not too large */` (c_src/src/decompress/zstd_decompress_block.c:2027) | assertion/abort | [ ] |
| 1488 | `ZSTD_getOffsetInfo` | `assert(tableLog <= OffFSELog);` (c_src/src/decompress/zstd_decompress_block.c:2033) | assertion/abort | [ ] |
| 1489 | `ZSTD_maxShortOffset` | `ZSTD_STATIC_ASSERT(ZSTD_WINDOWLOG_MAX <= 31);` (c_src/src/decompress/zstd_decompress_block.c:2051) | exact return/error shown | [ ] |
| 1490 | `ZSTD_maxShortOffset` | `assert(ZSTD_highbit32((U32)maxOffbase) == STREAM_ACCUMULATOR_MIN);` (c_src/src/decompress/zstd_decompress_block.c:2060) | assertion/abort | [ ] |
| 1491 | `ZSTD_decompressBlock_internal` | `RETURN_ERROR_IF(srcSize > ZSTD_blockSizeMax(dctx), srcSize_wrong, "");` (c_src/src/decompress/zstd_decompress_block.c:2081) | exact return/error shown | [ ] |
| 1492 | `ZSTD_decompressBlock_internal` | `RETURN_ERROR_IF((dst == NULL \|\| dstCapacity == 0) && nbSeq > 0, dstSize_tooSmall, "NULL not handled");` (c_src/src/decompress/zstd_decompress_block.c:2129) | exact return/error shown | [ ] |
| 1493 | `ZSTD_decompressBlock_internal` | `RETURN_ERROR_IF(MEM_64bits() && sizeof(size_t) == sizeof(void*) && (size_t)(-1) - (size_t)dst < (size_t)(1 << 20), dstSize_tooSmall, "invalid dst");` (c_src/src/decompress/zstd_decompress_block.c:2130) | exact return/error shown | [ ] |
| 1494 | `ZSTD_decompressBlock_deprecated` | `FORWARD_IF_ERROR(dSize, "");` (c_src/src/decompress/zstd_decompress_block.c:2197) | exact return/error shown | [ ] |
| 1495 | `ZBUFF_isError` | `unsigned ZBUFF_isError(size_t errorCode) { return ERR_isError(errorCode); }` (c_src/src/deprecated/zbuff_common.c:23) | exact return/error shown | [ ] |
| 1496 | `ZBUFF_getErrorName` | `const char* ZBUFF_getErrorName(size_t errorCode) { return ERR_getErrorName(errorCode); }` (c_src/src/deprecated/zbuff_common.c:26) | exact return/error shown | [ ] |
| 1497 | `ZBUFF_compressInit_advanced` | `FORWARD_IF_ERROR(ZSTD_CCtx_reset(zbc, ZSTD_reset_session_only), "");` (c_src/src/deprecated/zbuff_compress.c:77) | exact return/error shown | [ ] |
| 1498 | `ZBUFF_compressInit_advanced` | `FORWARD_IF_ERROR(ZSTD_CCtx_setPledgedSrcSize(zbc, pledgedSrcSize), "");` (c_src/src/deprecated/zbuff_compress.c:78) | exact return/error shown | [ ] |
| 1499 | `ZBUFF_compressInit_advanced` | `FORWARD_IF_ERROR(ZSTD_checkCParams(params.cParams), "");` (c_src/src/deprecated/zbuff_compress.c:80) | exact return/error shown | [ ] |
| 1500 | `ZBUFF_compressInit_advanced` | `FORWARD_IF_ERROR(ZSTD_CCtx_setParameter(zbc, ZSTD_c_windowLog, params.cParams.windowLog), "");` (c_src/src/deprecated/zbuff_compress.c:81) | exact return/error shown | [ ] |
| 1501 | `ZBUFF_compressInit_advanced` | `FORWARD_IF_ERROR(ZSTD_CCtx_setParameter(zbc, ZSTD_c_hashLog, params.cParams.hashLog), "");` (c_src/src/deprecated/zbuff_compress.c:82) | exact return/error shown | [ ] |
| 1502 | `ZBUFF_compressInit_advanced` | `FORWARD_IF_ERROR(ZSTD_CCtx_setParameter(zbc, ZSTD_c_chainLog, params.cParams.chainLog), "");` (c_src/src/deprecated/zbuff_compress.c:83) | exact return/error shown | [ ] |
| 1503 | `ZBUFF_compressInit_advanced` | `FORWARD_IF_ERROR(ZSTD_CCtx_setParameter(zbc, ZSTD_c_searchLog, params.cParams.searchLog), "");` (c_src/src/deprecated/zbuff_compress.c:84) | exact return/error shown | [ ] |
| 1504 | `ZBUFF_compressInit_advanced` | `FORWARD_IF_ERROR(ZSTD_CCtx_setParameter(zbc, ZSTD_c_minMatch, params.cParams.minMatch), "");` (c_src/src/deprecated/zbuff_compress.c:85) | exact return/error shown | [ ] |
| 1505 | `ZBUFF_compressInit_advanced` | `FORWARD_IF_ERROR(ZSTD_CCtx_setParameter(zbc, ZSTD_c_targetLength, params.cParams.targetLength), "");` (c_src/src/deprecated/zbuff_compress.c:86) | exact return/error shown | [ ] |
| 1506 | `ZBUFF_compressInit_advanced` | `FORWARD_IF_ERROR(ZSTD_CCtx_setParameter(zbc, ZSTD_c_strategy, params.cParams.strategy), "");` (c_src/src/deprecated/zbuff_compress.c:87) | exact return/error shown | [ ] |
| 1507 | `ZBUFF_compressInit_advanced` | `FORWARD_IF_ERROR(ZSTD_CCtx_setParameter(zbc, ZSTD_c_contentSizeFlag, params.fParams.contentSizeFlag), "");` (c_src/src/deprecated/zbuff_compress.c:89) | exact return/error shown | [ ] |
| 1508 | `ZBUFF_compressInit_advanced` | `FORWARD_IF_ERROR(ZSTD_CCtx_setParameter(zbc, ZSTD_c_checksumFlag, params.fParams.checksumFlag), "");` (c_src/src/deprecated/zbuff_compress.c:90) | exact return/error shown | [ ] |
| 1509 | `ZBUFF_compressInit_advanced` | `FORWARD_IF_ERROR(ZSTD_CCtx_setParameter(zbc, ZSTD_c_dictIDFlag, params.fParams.noDictIDFlag), "");` (c_src/src/deprecated/zbuff_compress.c:91) | exact return/error shown | [ ] |
| 1510 | `ZBUFF_compressInit_advanced` | `FORWARD_IF_ERROR(ZSTD_CCtx_loadDictionary(zbc, dict, dictSize), "");` (c_src/src/deprecated/zbuff_compress.c:93) | exact return/error shown | [ ] |
| 1511 | `ZBUFF_compressInitDictionary` | `FORWARD_IF_ERROR(ZSTD_CCtx_reset(zbc, ZSTD_reset_session_only), "");` (c_src/src/deprecated/zbuff_compress.c:99) | exact return/error shown | [ ] |
| 1512 | `ZBUFF_compressInitDictionary` | `FORWARD_IF_ERROR(ZSTD_CCtx_setParameter(zbc, ZSTD_c_compressionLevel, compressionLevel), "");` (c_src/src/deprecated/zbuff_compress.c:100) | exact return/error shown | [ ] |
| 1513 | `ZBUFF_compressInitDictionary` | `FORWARD_IF_ERROR(ZSTD_CCtx_loadDictionary(zbc, dict, dictSize), "");` (c_src/src/deprecated/zbuff_compress.c:101) | exact return/error shown | [ ] |
| 1514 | `COVER_cmp8` | `return -1;` (c_src/src/dictBuilder/cover.c:283) | exact return/error shown | [ ] |
| 1515 | `COVER_lower_bound` | `assert(last >= first);` (c_src/src/dictBuilder/cover.c:358) | assertion/abort | [ ] |
| 1516 | `COVER_ctx_init` | `return ERROR(srcSize_wrong);` (c_src/src/dictBuilder/cover.c:618) | exact return/error shown | [ ] |
| 1517 | `COVER_ctx_init` | `return ERROR(srcSize_wrong);` (c_src/src/dictBuilder/cover.c:623) | exact return/error shown | [ ] |
| 1518 | `COVER_ctx_init` | `return ERROR(srcSize_wrong);` (c_src/src/dictBuilder/cover.c:628) | exact return/error shown | [ ] |
| 1519 | `COVER_ctx_init` | `return ERROR(memory_allocation);` (c_src/src/dictBuilder/cover.c:651) | exact return/error shown | [ ] |
| 1520 | `COVER_computeEpochs` | `assert(epochs.size * epochs.num <= nbDmers);` (c_src/src/dictBuilder/cover.c:715) | assertion/abort | [ ] |
| 1521 | `COVER_computeEpochs` | `assert(epochs.size * epochs.num <= nbDmers);` (c_src/src/dictBuilder/cover.c:720) | assertion/abort | [ ] |
| 1522 | `ZDICT_trainFromBuffer_cover` | `return ERROR(parameter_outOfBound);` (c_src/src/dictBuilder/cover.c:793) | exact return/error shown | [ ] |
| 1523 | `ZDICT_trainFromBuffer_cover` | `return ERROR(srcSize_wrong);` (c_src/src/dictBuilder/cover.c:797) | exact return/error shown | [ ] |
| 1524 | `ZDICT_trainFromBuffer_cover` | `return ERROR(dstSize_tooSmall);` (c_src/src/dictBuilder/cover.c:802) | exact return/error shown | [ ] |
| 1525 | `ZDICT_trainFromBuffer_cover` | `return ERROR(memory_allocation);` (c_src/src/dictBuilder/cover.c:816) | exact return/error shown | [ ] |
| 1526 | `COVER_selectDict` | `return COVER_dictSelectionError(dictContentSize);` (c_src/src/dictBuilder/cover.c:1035) | exact return/error shown | [ ] |
| 1527 | `COVER_selectDict` | `return COVER_dictSelectionError(dictContentSize);` (c_src/src/dictBuilder/cover.c:1047) | exact return/error shown | [ ] |
| 1528 | `COVER_selectDict` | `return COVER_dictSelectionError(totalCompressedSize);` (c_src/src/dictBuilder/cover.c:1058) | exact return/error shown | [ ] |
| 1529 | `COVER_selectDict` | `return COVER_dictSelectionError(dictContentSize);` (c_src/src/dictBuilder/cover.c:1080) | exact return/error shown | [ ] |
| 1530 | `COVER_selectDict` | `return COVER_dictSelectionError(totalCompressedSize);` (c_src/src/dictBuilder/cover.c:1092) | exact return/error shown | [ ] |
| 1531 | `ZDICT_optimizeTrainFromBuffer_cover` | `return ERROR(parameter_outOfBound);` (c_src/src/dictBuilder/cover.c:1197) | exact return/error shown | [ ] |
| 1532 | `ZDICT_optimizeTrainFromBuffer_cover` | `return ERROR(parameter_outOfBound);` (c_src/src/dictBuilder/cover.c:1201) | exact return/error shown | [ ] |
| 1533 | `ZDICT_optimizeTrainFromBuffer_cover` | `return ERROR(srcSize_wrong);` (c_src/src/dictBuilder/cover.c:1205) | exact return/error shown | [ ] |
| 1534 | `ZDICT_optimizeTrainFromBuffer_cover` | `return ERROR(dstSize_tooSmall);` (c_src/src/dictBuilder/cover.c:1210) | exact return/error shown | [ ] |
| 1535 | `ZDICT_optimizeTrainFromBuffer_cover` | `return ERROR(memory_allocation);` (c_src/src/dictBuilder/cover.c:1215) | exact return/error shown | [ ] |
| 1536 | `ZDICT_optimizeTrainFromBuffer_cover` | `return ERROR(memory_allocation);` (c_src/src/dictBuilder/cover.c:1253) | exact return/error shown | [ ] |
| 1537 | `<file scope/macro>` | `assert(ssize < STACK_SIZE);\` (c_src/src/dictBuilder/divsufsort.c:104) | assertion/abort | [ ] |
| 1538 | `<file scope/macro>` | `assert(ssize < STACK_SIZE);\` (c_src/src/dictBuilder/divsufsort.c:110) | assertion/abort | [ ] |
| 1539 | `<file scope/macro>` | `assert(0 <= ssize);\` (c_src/src/dictBuilder/divsufsort.c:116) | assertion/abort | [ ] |
| 1540 | `<file scope/macro>` | `assert(0 <= ssize);\` (c_src/src/dictBuilder/divsufsort.c:123) | assertion/abort | [ ] |
| 1541 | `construct_SA` | `assert(T[s] == c1);` (c_src/src/dictBuilder/divsufsort.c:1630) | assertion/abort | [ ] |
| 1542 | `construct_SA` | `assert(((s + 1) < n) && (T[s] <= T[s + 1]));` (c_src/src/dictBuilder/divsufsort.c:1631) | assertion/abort | [ ] |
| 1543 | `construct_SA` | `assert(T[s - 1] <= T[s]);` (c_src/src/dictBuilder/divsufsort.c:1632) | assertion/abort | [ ] |
| 1544 | `construct_SA` | `assert(k < j); assert(k != NULL);` (c_src/src/dictBuilder/divsufsort.c:1640) | assertion/abort | [ ] |
| 1545 | `construct_SA` | `assert(((s == 0) && (T[s] == c1)) \|\| (s < 0));` (c_src/src/dictBuilder/divsufsort.c:1643) | assertion/abort | [ ] |
| 1546 | `construct_SA` | `assert(T[s - 1] >= T[s]);` (c_src/src/dictBuilder/divsufsort.c:1657) | assertion/abort | [ ] |
| 1547 | `construct_SA` | `assert(i < k);` (c_src/src/dictBuilder/divsufsort.c:1664) | assertion/abort | [ ] |
| 1548 | `construct_SA` | `assert(s < 0);` (c_src/src/dictBuilder/divsufsort.c:1667) | assertion/abort | [ ] |
| 1549 | `construct_BWT` | `assert(T[s] == c1);` (c_src/src/dictBuilder/divsufsort.c:1694) | assertion/abort | [ ] |
| 1550 | `construct_BWT` | `assert(((s + 1) < n) && (T[s] <= T[s + 1]));` (c_src/src/dictBuilder/divsufsort.c:1695) | assertion/abort | [ ] |
| 1551 | `construct_BWT` | `assert(T[s - 1] <= T[s]);` (c_src/src/dictBuilder/divsufsort.c:1696) | assertion/abort | [ ] |
| 1552 | `construct_BWT` | `assert(k < j); assert(k != NULL);` (c_src/src/dictBuilder/divsufsort.c:1704) | assertion/abort | [ ] |
| 1553 | `construct_BWT` | `assert(T[s] == c1);` (c_src/src/dictBuilder/divsufsort.c:1710) | assertion/abort | [ ] |
| 1554 | `construct_BWT` | `assert(T[s - 1] >= T[s]);` (c_src/src/dictBuilder/divsufsort.c:1724) | assertion/abort | [ ] |
| 1555 | `construct_BWT` | `assert(i < k);` (c_src/src/dictBuilder/divsufsort.c:1732) | assertion/abort | [ ] |
| 1556 | `construct_BWT_indexes` | `assert(T[s] == c1);` (c_src/src/dictBuilder/divsufsort.c:1775) | assertion/abort | [ ] |
| 1557 | `construct_BWT_indexes` | `assert(((s + 1) < n) && (T[s] <= T[s + 1]));` (c_src/src/dictBuilder/divsufsort.c:1776) | assertion/abort | [ ] |
| 1558 | `construct_BWT_indexes` | `assert(T[s - 1] <= T[s]);` (c_src/src/dictBuilder/divsufsort.c:1777) | assertion/abort | [ ] |
| 1559 | `construct_BWT_indexes` | `assert(k < j); assert(k != NULL);` (c_src/src/dictBuilder/divsufsort.c:1788) | assertion/abort | [ ] |
| 1560 | `construct_BWT_indexes` | `assert(T[s] == c1);` (c_src/src/dictBuilder/divsufsort.c:1794) | assertion/abort | [ ] |
| 1561 | `construct_BWT_indexes` | `assert(T[s - 1] >= T[s]);` (c_src/src/dictBuilder/divsufsort.c:1815) | assertion/abort | [ ] |
| 1562 | `construct_BWT_indexes` | `assert(i < k);` (c_src/src/dictBuilder/divsufsort.c:1825) | assertion/abort | [ ] |
| 1563 | `divsufsort` | `if((T == NULL) \|\| (SA == NULL) \|\| (n < 0)) { return -1; }` (c_src/src/dictBuilder/divsufsort.c:1853) | exact return/error shown | [ ] |
| 1564 | `divbwt` | `if((T == NULL) \|\| (U == NULL) \|\| (n < 0)) { return -1; }` (c_src/src/dictBuilder/divsufsort.c:1882) | exact return/error shown | [ ] |
| 1565 | `FASTCOVER_computeFrequency` | `assert(ctx->nbTrainSamples >= 5);` (c_src/src/dictBuilder/fastcover.c:291) | assertion/abort | [ ] |
| 1566 | `FASTCOVER_computeFrequency` | `assert(ctx->nbTrainSamples <= ctx->nbSamples);` (c_src/src/dictBuilder/fastcover.c:292) | assertion/abort | [ ] |
| 1567 | `FASTCOVER_ctx_init` | `return ERROR(srcSize_wrong);` (c_src/src/dictBuilder/fastcover.c:332) | exact return/error shown | [ ] |
| 1568 | `FASTCOVER_ctx_init` | `return ERROR(srcSize_wrong);` (c_src/src/dictBuilder/fastcover.c:338) | exact return/error shown | [ ] |
| 1569 | `FASTCOVER_ctx_init` | `return ERROR(srcSize_wrong);` (c_src/src/dictBuilder/fastcover.c:344) | exact return/error shown | [ ] |
| 1570 | `FASTCOVER_ctx_init` | `return ERROR(memory_allocation);` (c_src/src/dictBuilder/fastcover.c:369) | exact return/error shown | [ ] |
| 1571 | `FASTCOVER_ctx_init` | `assert(nbSamples >= 5);` (c_src/src/dictBuilder/fastcover.c:375) | assertion/abort | [ ] |
| 1572 | `FASTCOVER_ctx_init` | `return ERROR(memory_allocation);` (c_src/src/dictBuilder/fastcover.c:386) | exact return/error shown | [ ] |
| 1573 | `ZDICT_trainFromBuffer_fastCover` | `return ERROR(parameter_outOfBound);` (c_src/src/dictBuilder/fastcover.c:571) | exact return/error shown | [ ] |
| 1574 | `ZDICT_trainFromBuffer_fastCover` | `return ERROR(srcSize_wrong);` (c_src/src/dictBuilder/fastcover.c:575) | exact return/error shown | [ ] |
| 1575 | `ZDICT_trainFromBuffer_fastCover` | `return ERROR(dstSize_tooSmall);` (c_src/src/dictBuilder/fastcover.c:580) | exact return/error shown | [ ] |
| 1576 | `ZDICT_optimizeTrainFromBuffer_fastCover` | `return ERROR(parameter_outOfBound);` (c_src/src/dictBuilder/fastcover.c:652) | exact return/error shown | [ ] |
| 1577 | `ZDICT_optimizeTrainFromBuffer_fastCover` | `return ERROR(parameter_outOfBound);` (c_src/src/dictBuilder/fastcover.c:656) | exact return/error shown | [ ] |
| 1578 | `ZDICT_optimizeTrainFromBuffer_fastCover` | `return ERROR(parameter_outOfBound);` (c_src/src/dictBuilder/fastcover.c:660) | exact return/error shown | [ ] |
| 1579 | `ZDICT_optimizeTrainFromBuffer_fastCover` | `return ERROR(srcSize_wrong);` (c_src/src/dictBuilder/fastcover.c:664) | exact return/error shown | [ ] |
| 1580 | `ZDICT_optimizeTrainFromBuffer_fastCover` | `return ERROR(dstSize_tooSmall);` (c_src/src/dictBuilder/fastcover.c:669) | exact return/error shown | [ ] |
| 1581 | `ZDICT_optimizeTrainFromBuffer_fastCover` | `return ERROR(memory_allocation);` (c_src/src/dictBuilder/fastcover.c:674) | exact return/error shown | [ ] |
| 1582 | `ZDICT_optimizeTrainFromBuffer_fastCover` | `return ERROR(memory_allocation);` (c_src/src/dictBuilder/fastcover.c:715) | exact return/error shown | [ ] |
| 1583 | `ZDICT_isError` | `unsigned ZDICT_isError(size_t errorCode) { return ERR_isError(errorCode); }` (c_src/src/dictBuilder/zdict.c:98) | exact return/error shown | [ ] |
| 1584 | `ZDICT_getErrorName` | `const char* ZDICT_getErrorName(size_t errorCode) { return ERR_getErrorName(errorCode); }` (c_src/src/dictBuilder/zdict.c:100) | exact return/error shown | [ ] |
| 1585 | `ZDICT_getDictHeaderSize` | `if (dictSize <= 8 \|\| MEM_readLE32(dictBuffer) != ZSTD_MAGIC_DICTIONARY) return ERROR(dictionary_corrupted);` (c_src/src/dictBuilder/zdict.c:112) | exact return/error shown | [ ] |
| 1586 | `ZDICT_analyzeEntropy` | `assert(maxNbBits==9);` (c_src/src/dictBuilder/zdict.c:735) | assertion/abort | [ ] |
| 1587 | `ZDICT_finalizeDictionary` | `if (dictBufferCapacity < dictContentSize) return ERROR(dstSize_tooSmall);` (c_src/src/dictBuilder/zdict.c:874) | exact return/error shown | [ ] |
| 1588 | `ZDICT_finalizeDictionary` | `if (dictBufferCapacity < ZDICT_DICTSIZE_MIN) return ERROR(dstSize_tooSmall);` (c_src/src/dictBuilder/zdict.c:875) | exact return/error shown | [ ] |
| 1589 | `ZDICT_finalizeDictionary` | `RETURN_ERROR_IF(hSize + minContentSize > dictBufferCapacity, dstSize_tooSmall, "dictBufferCapacity too small to fit max repcode");` (c_src/src/dictBuilder/zdict.c:905) | exact return/error shown | [ ] |
| 1590 | `ZDICT_finalizeDictionary` | `assert(dictSize <= dictBufferCapacity);` (c_src/src/dictBuilder/zdict.c:923) | assertion/abort | [ ] |
| 1591 | `ZDICT_finalizeDictionary` | `assert(outDictContent + dictContentSize == (BYTE*)dictBuffer + dictSize);` (c_src/src/dictBuilder/zdict.c:924) | assertion/abort | [ ] |
| 1592 | `ZDICT_trainFromBuffer_unsafe_legacy` | `if (!dictList) return ERROR(memory_allocation);` (c_src/src/dictBuilder/zdict.c:993) | exact return/error shown | [ ] |
| 1593 | `ZDICT_trainFromBuffer_unsafe_legacy` | `if (maxDictSize < ZDICT_DICTSIZE_MIN) { free(dictList); return ERROR(dstSize_tooSmall); } /* requested dictionary size is too small */` (c_src/src/dictBuilder/zdict.c:994) | exact return/error shown | [ ] |
| 1594 | `ZDICT_trainFromBuffer_unsafe_legacy` | `if (samplesBuffSize < ZDICT_MIN_SAMPLES_SIZE) { free(dictList); return ERROR(dictionaryCreation_failed); } /* not enough source to create dictionary */` (c_src/src/dictBuilder/zdict.c:995) | exact return/error shown | [ ] |
| 1595 | `ZDICT_trainFromBuffer_unsafe_legacy` | `return ERROR(GENERIC); /* should never happen */` (c_src/src/dictBuilder/zdict.c:1019) | exact return/error shown | [ ] |
| 1596 | `ZDICT_trainFromBuffer_unsafe_legacy` | `if (dictContentSize < ZDICT_CONTENTSIZE_MIN) { free(dictList); return ERROR(dictionaryCreation_failed); } /* dictionary content too small */` (c_src/src/dictBuilder/zdict.c:1030) | exact return/error shown | [ ] |
| 1597 | `ZDICT_trainFromBuffer_unsafe_legacy` | `if (ptr<(BYTE*)dictBuffer) { free(dictList); return ERROR(GENERIC); } /* should not happen */` (c_src/src/dictBuilder/zdict.c:1066) | exact return/error shown | [ ] |
| 1598 | `ZDICT_trainFromBuffer_legacy` | `if (!newBuff) return ERROR(memory_allocation);` (c_src/src/dictBuilder/zdict.c:1094) | exact return/error shown | [ ] |
| 1599 | `FSE_buildDTable` | `if (maxSymbolValue > FSE_MAX_SYMBOL_VALUE) return (size_t)-FSE_ERROR_maxSymbolValue_tooLarge;` (c_src/src/legacy/zstd_v01.c:374) | exact return/error shown | [ ] |
| 1600 | `FSE_buildDTable` | `if (tableLog > FSE_MAX_TABLELOG) return (size_t)-FSE_ERROR_tableLog_tooLarge;` (c_src/src/legacy/zstd_v01.c:375) | exact return/error shown | [ ] |
| 1601 | `FSE_buildDTable` | `if (position!=0) return (size_t)-FSE_ERROR_GENERIC; /* position must reach all cells once, otherwise normalizedCounter is incorrect */` (c_src/src/legacy/zstd_v01.c:405) | exact return/error shown | [ ] |
| 1602 | `FSE_readNCount` | `if (hbSize < 4) return (size_t)-FSE_ERROR_srcSize_wrong;` (c_src/src/legacy/zstd_v01.c:454) | exact return/error shown | [ ] |
| 1603 | `FSE_readNCount` | `if (nbBits > FSE_TABLELOG_ABSOLUTE_MAX) return (size_t)-FSE_ERROR_tableLog_tooLarge;` (c_src/src/legacy/zstd_v01.c:457) | exact return/error shown | [ ] |
| 1604 | `FSE_readNCount` | `if (n0 > *maxSVPtr) return (size_t)-FSE_ERROR_maxSymbolValue_tooSmall;` (c_src/src/legacy/zstd_v01.c:492) | exact return/error shown | [ ] |
| 1605 | `FSE_readNCount` | `if (remaining != 1) return (size_t)-FSE_ERROR_GENERIC;` (c_src/src/legacy/zstd_v01.c:544) | exact return/error shown | [ ] |
| 1606 | `FSE_readNCount` | `if ((size_t)(ip-istart) > hbSize) return (size_t)-FSE_ERROR_srcSize_wrong;` (c_src/src/legacy/zstd_v01.c:548) | exact return/error shown | [ ] |
| 1607 | `FSE_buildDTable_raw` | `if (nbBits < 1) return (size_t)-FSE_ERROR_GENERIC; /* min size */` (c_src/src/legacy/zstd_v01.c:584) | exact return/error shown | [ ] |
| 1608 | `FSE_initDStream` | `if (srcSize < 1) return (size_t)-FSE_ERROR_srcSize_wrong;` (c_src/src/legacy/zstd_v01.c:608) | exact return/error shown | [ ] |
| 1609 | `FSE_initDStream` | `if (contain32 == 0) return (size_t)-FSE_ERROR_GENERIC; /* stop bit not present */` (c_src/src/legacy/zstd_v01.c:617) | exact return/error shown | [ ] |
| 1610 | `FSE_initDStream` | `if (contain32 == 0) return (size_t)-FSE_ERROR_GENERIC; /* stop bit not present */` (c_src/src/legacy/zstd_v01.c:643) | exact return/error shown | [ ] |
| 1611 | `FSE_decompress_usingDTable_generic` | `if (FSE_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v01.c:793) | exact return/error shown | [ ] |
| 1612 | `FSE_decompress_usingDTable_generic` | `if (op==omax) return (size_t)-FSE_ERROR_dstSize_tooSmall; /* dst buffer is full, but cSrc unfinished */` (c_src/src/legacy/zstd_v01.c:840) | exact return/error shown | [ ] |
| 1613 | `FSE_decompress_usingDTable_generic` | `return (size_t)-FSE_ERROR_corruptionDetected;` (c_src/src/legacy/zstd_v01.c:842) | exact return/error shown | [ ] |
| 1614 | `FSE_decompress` | `if (cSrcSize<2) return (size_t)-FSE_ERROR_srcSize_wrong; /* too small input size */` (c_src/src/legacy/zstd_v01.c:869) | exact return/error shown | [ ] |
| 1615 | `FSE_decompress` | `if (FSE_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v01.c:873) | exact return/error shown | [ ] |
| 1616 | `FSE_decompress` | `if (errorCode >= cSrcSize) return (size_t)-FSE_ERROR_srcSize_wrong; /* too small input size */` (c_src/src/legacy/zstd_v01.c:874) | exact return/error shown | [ ] |
| 1617 | `FSE_decompress` | `if (FSE_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v01.c:879) | exact return/error shown | [ ] |
| 1618 | `HUF_readDTable` | `if (!srcSize) return (size_t)-FSE_ERROR_srcSize_wrong;` (c_src/src/legacy/zstd_v01.c:933) | exact return/error shown | [ ] |
| 1619 | `HUF_readDTable` | `if (iSize+1 > srcSize) return (size_t)-FSE_ERROR_srcSize_wrong;` (c_src/src/legacy/zstd_v01.c:951) | exact return/error shown | [ ] |
| 1620 | `HUF_readDTable` | `if (iSize+1 > srcSize) return (size_t)-FSE_ERROR_srcSize_wrong;` (c_src/src/legacy/zstd_v01.c:962) | exact return/error shown | [ ] |
| 1621 | `HUF_readDTable` | `if (huffWeight[n] >= HUF_ABSOLUTEMAX_TABLELOG) return (size_t)-FSE_ERROR_corruptionDetected;` (c_src/src/legacy/zstd_v01.c:972) | exact return/error shown | [ ] |
| 1622 | `HUF_readDTable` | `if (weightTotal == 0) return (size_t)-FSE_ERROR_corruptionDetected;` (c_src/src/legacy/zstd_v01.c:976) | exact return/error shown | [ ] |
| 1623 | `HUF_readDTable` | `if (maxBits > DTable[0]) return (size_t)-FSE_ERROR_tableLog_tooLarge; /* DTable is too small */` (c_src/src/legacy/zstd_v01.c:980) | exact return/error shown | [ ] |
| 1624 | `HUF_readDTable` | `if (verif != rest) return (size_t)-FSE_ERROR_corruptionDetected; /* last value must be a clean power of 2 */` (c_src/src/legacy/zstd_v01.c:987) | exact return/error shown | [ ] |
| 1625 | `HUF_readDTable` | `if ((rankVal[1] < 2) \|\| (rankVal[1] & 1)) return (size_t)-FSE_ERROR_corruptionDetected; /* by construction : at least 2 elts of rank 1, must be even */` (c_src/src/legacy/zstd_v01.c:993) | exact return/error shown | [ ] |
| 1626 | `HUF_decompress_usingDTable` | `if (cSrcSize < 6) return (size_t)-FSE_ERROR_srcSize_wrong;` (c_src/src/legacy/zstd_v01.c:1034) | exact return/error shown | [ ] |
| 1627 | `HUF_decompress_usingDTable` | `if (length1+length2+length3+6 >= cSrcSize) return (size_t)-FSE_ERROR_srcSize_wrong;` (c_src/src/legacy/zstd_v01.c:1060) | exact return/error shown | [ ] |
| 1628 | `HUF_decompress_usingDTable` | `if (FSE_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v01.c:1063) | exact return/error shown | [ ] |
| 1629 | `HUF_decompress_usingDTable` | `if (FSE_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v01.c:1065) | exact return/error shown | [ ] |
| 1630 | `HUF_decompress_usingDTable` | `if (FSE_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v01.c:1067) | exact return/error shown | [ ] |
| 1631 | `HUF_decompress_usingDTable` | `if (FSE_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v01.c:1069) | exact return/error shown | [ ] |
| 1632 | `HUF_decompress_usingDTable` | `return (size_t)-FSE_ERROR_corruptionDetected;` (c_src/src/legacy/zstd_v01.c:1107) | exact return/error shown | [ ] |
| 1633 | `HUF_decompress_usingDTable` | `if (op==omax) return (size_t)-FSE_ERROR_dstSize_tooSmall; /* dst buffer is full, but cSrc unfinished */` (c_src/src/legacy/zstd_v01.c:1126) | exact return/error shown | [ ] |
| 1634 | `HUF_decompress_usingDTable` | `return (size_t)-FSE_ERROR_corruptionDetected;` (c_src/src/legacy/zstd_v01.c:1128) | exact return/error shown | [ ] |
| 1635 | `HUF_decompress` | `if (FSE_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v01.c:1140) | exact return/error shown | [ ] |
| 1636 | `HUF_decompress` | `if (errorCode >= cSrcSize) return (size_t)-FSE_ERROR_srcSize_wrong;` (c_src/src/legacy/zstd_v01.c:1141) | exact return/error shown | [ ] |
| 1637 | `ZSTDv01_isError` | `unsigned ZSTDv01_isError(size_t code) { return ERR_isError(code); }` (c_src/src/legacy/zstd_v01.c:1410) | exact return/error shown | [ ] |
| 1638 | `ZSTDv01_getcBlockSize` | `if (srcSize < 3) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v01.c:1431) | exact return/error shown | [ ] |
| 1639 | `ZSTD_copyUncompressedBlock` | `if (srcSize > maxDstSize) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v01.c:1447) | exact return/error shown | [ ] |
| 1640 | `ZSTD_decompressLiterals` | `if (srcSize <= 3) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v01.c:1466) | exact return/error shown | [ ] |
| 1641 | `ZSTD_decompressLiterals` | `if (litSize > maxDstSize) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v01.c:1473) | exact return/error shown | [ ] |
| 1642 | `ZSTD_decompressLiterals` | `if (FSE_isError(errorCode)) return ERROR(GENERIC);` (c_src/src/legacy/zstd_v01.c:1475) | exact return/error shown | [ ] |
| 1643 | `ZSTDv01_decodeLiteralsBlock` | `if (litcSize > srcSize - ZSTD_blockHeaderSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v01.c:1493) | exact return/error shown | [ ] |
| 1644 | `ZSTDv01_decodeLiteralsBlock` | `if (rleSize>maxDstSize) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v01.c:1506) | exact return/error shown | [ ] |
| 1645 | `ZSTDv01_decodeLiteralsBlock` | `if (!srcSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v01.c:1507) | exact return/error shown | [ ] |
| 1646 | `ZSTDv01_decodeLiteralsBlock` | `return ERROR(GENERIC);` (c_src/src/legacy/zstd_v01.c:1527) | exact return/error shown | [ ] |
| 1647 | `ZSTDv01_decodeSeqHeaders` | `if (srcSize < 5) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v01.c:1546) | exact return/error shown | [ ] |
| 1648 | `ZSTDv01_decodeSeqHeaders` | `if (ip > iend-3) return ERROR(srcSize_wrong); /* min : all 3 are "raw", hence no header, but at least xxLog bits per type */` (c_src/src/legacy/zstd_v01.c:1570) | exact return/error shown | [ ] |
| 1649 | `ZSTDv01_decodeSeqHeaders` | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` (c_src/src/legacy/zstd_v01.c:1589) | exact return/error shown | [ ] |
| 1650 | `ZSTDv01_decodeSeqHeaders` | `if (LLlog > LLFSELog) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v01.c:1590) | exact return/error shown | [ ] |
| 1651 | `ZSTDv01_decodeSeqHeaders` | `if (ip > iend-2) return ERROR(srcSize_wrong); /* min : "raw", hence no header, but at least xxLog bits */` (c_src/src/legacy/zstd_v01.c:1599) | exact return/error shown | [ ] |
| 1652 | `ZSTDv01_decodeSeqHeaders` | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` (c_src/src/legacy/zstd_v01.c:1607) | exact return/error shown | [ ] |
| 1653 | `ZSTDv01_decodeSeqHeaders` | `if (Offlog > OffFSELog) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v01.c:1608) | exact return/error shown | [ ] |
| 1654 | `ZSTDv01_decodeSeqHeaders` | `if (ip > iend-2) return ERROR(srcSize_wrong); /* min : "raw", hence no header, but at least xxLog bits */` (c_src/src/legacy/zstd_v01.c:1617) | exact return/error shown | [ ] |
| 1655 | `ZSTDv01_decodeSeqHeaders` | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` (c_src/src/legacy/zstd_v01.c:1625) | exact return/error shown | [ ] |
| 1656 | `ZSTDv01_decodeSeqHeaders` | `if (MLlog > MLFSELog) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v01.c:1626) | exact return/error shown | [ ] |
| 1657 | `ZSTD_execSequence` | `if (seqLength > (size_t)(oend - op)) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v01.c:1732) | exact return/error shown | [ ] |
| 1658 | `ZSTD_execSequence` | `if (sequence.litLength > (size_t)(litLimit - *litPtr)) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v01.c:1733) | exact return/error shown | [ ] |
| 1659 | `ZSTD_execSequence` | `if (sequence.offset > (U32)(oLitEnd - base)) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v01.c:1735) | exact return/error shown | [ ] |
| 1660 | `ZSTD_execSequence` | `if (endMatch > oend) return ERROR(dstSize_tooSmall); /* overwrite beyond dst buffer */` (c_src/src/legacy/zstd_v01.c:1737) | exact return/error shown | [ ] |
| 1661 | `ZSTD_execSequence` | `if (litEnd > litLimit) return ERROR(corruption_detected); /* overRead beyond lit buffer */` (c_src/src/legacy/zstd_v01.c:1738) | exact return/error shown | [ ] |
| 1662 | `ZSTD_execSequence` | `if (sequence.matchLength > (size_t)(*litPtr-op)) return ERROR(dstSize_tooSmall); /* overwrite literal segment */` (c_src/src/legacy/zstd_v01.c:1739) | exact return/error shown | [ ] |
| 1663 | `ZSTD_execSequence` | `if (oend-op < 8) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v01.c:1748) | exact return/error shown | [ ] |
| 1664 | `ZSTD_execSequence` | `if (match < base) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v01.c:1758) | exact return/error shown | [ ] |
| 1665 | `ZSTD_execSequence` | `if (sequence.offset > (size_t)base) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v01.c:1759) | exact return/error shown | [ ] |
| 1666 | `ZSTD_decompressSequences` | `if (ZSTDv01_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v01.c:1840) | exact return/error shown | [ ] |
| 1667 | `ZSTD_decompressSequences` | `if (FSE_isError(errorCode)) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v01.c:1853) | exact return/error shown | [ ] |
| 1668 | `ZSTD_decompressSequences` | `if ( !FSE_endOfDStream(&(seqState.DStream)) ) return ERROR(corruption_detected); /* requested too much : data is corrupted */` (c_src/src/legacy/zstd_v01.c:1869) | exact return/error shown | [ ] |
| 1669 | `ZSTD_decompressSequences` | `if (nbSeq<0) return ERROR(corruption_detected); /* requested too many sequences : data is corrupted */` (c_src/src/legacy/zstd_v01.c:1870) | exact return/error shown | [ ] |
| 1670 | `ZSTD_decompressSequences` | `if (op+lastLLSize > oend) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v01.c:1875) | exact return/error shown | [ ] |
| 1671 | `ZSTD_decompressBlock` | `if (ZSTDv01_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v01.c:1900) | exact return/error shown | [ ] |
| 1672 | `ZSTDv01_decompressDCtx` | `if (srcSize < ZSTD_frameHeaderSize+ZSTD_blockHeaderSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v01.c:1921) | exact return/error shown | [ ] |
| 1673 | `ZSTDv01_decompressDCtx` | `if (magicNumber != ZSTD_magicNumber) return ERROR(prefix_unknown);` (c_src/src/legacy/zstd_v01.c:1923) | exact return/error shown | [ ] |
| 1674 | `ZSTDv01_decompressDCtx` | `if (blockSize > remainingSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v01.c:1934) | exact return/error shown | [ ] |
| 1675 | `ZSTDv01_decompressDCtx` | `return ERROR(GENERIC); /* not yet supported */` (c_src/src/legacy/zstd_v01.c:1945) | exact return/error shown | [ ] |
| 1676 | `ZSTDv01_decompressDCtx` | `if (remainingSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v01.c:1949) | exact return/error shown | [ ] |
| 1677 | `ZSTDv01_decompressDCtx` | `return ERROR(GENERIC);` (c_src/src/legacy/zstd_v01.c:1952) | exact return/error shown | [ ] |
| 1678 | `ZSTDv01_decompressDCtx` | `if (ZSTDv01_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v01.c:1956) | exact return/error shown | [ ] |
| 1679 | `ZSTDv01_createDCtx` | `if (dctx==NULL) return NULL;` (c_src/src/legacy/zstd_v01.c:2043) | exact return/error shown | [ ] |
| 1680 | `ZSTDv01_decompressContinue` | `if (srcSize != ctx->expected) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v01.c:2064) | exact return/error shown | [ ] |
| 1681 | `ZSTDv01_decompressContinue` | `if (magicNumber != ZSTD_magicNumber) return ERROR(prefix_unknown);` (c_src/src/legacy/zstd_v01.c:2073) | exact return/error shown | [ ] |
| 1682 | `ZSTDv01_decompressContinue` | `return ERROR(GENERIC); /* not yet handled */` (c_src/src/legacy/zstd_v01.c:2112) | exact return/error shown | [ ] |
| 1683 | `ZSTDv01_decompressContinue` | `return ERROR(GENERIC);` (c_src/src/legacy/zstd_v01.c:2118) | exact return/error shown | [ ] |
| 1684 | `BIT_initDStream` | `if (srcSize < 1) { memset(bitD, 0, sizeof(*bitD)); return ERROR(srcSize_wrong); }` (c_src/src/legacy/zstd_v02.c:325) | exact return/error shown | [ ] |
| 1685 | `BIT_initDStream` | `if (contain32 == 0) return ERROR(GENERIC); /* endMark not present */` (c_src/src/legacy/zstd_v02.c:334) | exact return/error shown | [ ] |
| 1686 | `BIT_initDStream` | `if (contain32 == 0) return ERROR(GENERIC); /* endMark not present */` (c_src/src/legacy/zstd_v02.c:360) | exact return/error shown | [ ] |
| 1687 | `ERR_getErrorName` | `return codeError;` (c_src/src/legacy/zstd_v02.c:530) | exact return/error shown | [ ] |
| 1688 | `FSE_buildDTable` | `if (maxSymbolValue > FSE_MAX_SYMBOL_VALUE) return ERROR(maxSymbolValue_tooLarge);` (c_src/src/legacy/zstd_v02.c:1051) | exact return/error shown | [ ] |
| 1689 | `FSE_buildDTable` | `if (tableLog > FSE_MAX_TABLELOG) return ERROR(tableLog_tooLarge);` (c_src/src/legacy/zstd_v02.c:1052) | exact return/error shown | [ ] |
| 1690 | `FSE_buildDTable` | `if (position!=0) return ERROR(GENERIC); /* position must reach all cells once, otherwise normalizedCounter is incorrect */` (c_src/src/legacy/zstd_v02.c:1082) | exact return/error shown | [ ] |
| 1691 | `FSE_isError` | `static unsigned FSE_isError(size_t code) { return ERR_isError(code); }` (c_src/src/legacy/zstd_v02.c:1106) | exact return/error shown | [ ] |
| 1692 | `FSE_readNCount` | `if (hbSize < 4) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v02.c:1131) | exact return/error shown | [ ] |
| 1693 | `FSE_readNCount` | `if (nbBits > FSE_TABLELOG_ABSOLUTE_MAX) return ERROR(tableLog_tooLarge);` (c_src/src/legacy/zstd_v02.c:1134) | exact return/error shown | [ ] |
| 1694 | `FSE_readNCount` | `if (n0 > *maxSVPtr) return ERROR(maxSymbolValue_tooSmall);` (c_src/src/legacy/zstd_v02.c:1169) | exact return/error shown | [ ] |
| 1695 | `FSE_readNCount` | `if (remaining != 1) return ERROR(GENERIC);` (c_src/src/legacy/zstd_v02.c:1221) | exact return/error shown | [ ] |
| 1696 | `FSE_readNCount` | `if ((size_t)(ip-istart) > hbSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v02.c:1225) | exact return/error shown | [ ] |
| 1697 | `FSE_buildDTable_raw` | `if (nbBits < 1) return ERROR(GENERIC); /* min size */` (c_src/src/legacy/zstd_v02.c:1261) | exact return/error shown | [ ] |
| 1698 | `FSE_decompress_usingDTable_generic` | `if (FSE_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v02.c:1293) | exact return/error shown | [ ] |
| 1699 | `FSE_decompress_usingDTable_generic` | `if (op==omax) return ERROR(dstSize_tooSmall); /* dst buffer is full, but cSrc unfinished */` (c_src/src/legacy/zstd_v02.c:1340) | exact return/error shown | [ ] |
| 1700 | `FSE_decompress_usingDTable_generic` | `return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v02.c:1342) | exact return/error shown | [ ] |
| 1701 | `FSE_decompress` | `if (cSrcSize<2) return ERROR(srcSize_wrong); /* too small input size */` (c_src/src/legacy/zstd_v02.c:1369) | exact return/error shown | [ ] |
| 1702 | `FSE_decompress` | `if (FSE_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v02.c:1373) | exact return/error shown | [ ] |
| 1703 | `FSE_decompress` | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong); /* too small input size */` (c_src/src/legacy/zstd_v02.c:1374) | exact return/error shown | [ ] |
| 1704 | `FSE_decompress` | `if (FSE_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v02.c:1379) | exact return/error shown | [ ] |
| 1705 | `HUF_isError` | `static unsigned HUF_isError(size_t code) { return ERR_isError(code); }` (c_src/src/legacy/zstd_v02.c:1455) | exact return/error shown | [ ] |
| 1706 | `HUF_readStats` | `if (!srcSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v02.c:1492) | exact return/error shown | [ ] |
| 1707 | `HUF_readStats` | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v02.c:1509) | exact return/error shown | [ ] |
| 1708 | `HUF_readStats` | `if (oSize >= hwSize) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v02.c:1510) | exact return/error shown | [ ] |
| 1709 | `HUF_readStats` | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v02.c:1521) | exact return/error shown | [ ] |
| 1710 | `HUF_readStats` | `if (huffWeight[n] >= HUF_ABSOLUTEMAX_TABLELOG) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v02.c:1531) | exact return/error shown | [ ] |
| 1711 | `HUF_readStats` | `if (weightTotal == 0) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v02.c:1535) | exact return/error shown | [ ] |
| 1712 | `HUF_readStats` | `if (tableLog > HUF_ABSOLUTEMAX_TABLELOG) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v02.c:1539) | exact return/error shown | [ ] |
| 1713 | `HUF_readStats` | `if (verif != rest) return ERROR(corruption_detected); /* last value must be a clean power of 2 */` (c_src/src/legacy/zstd_v02.c:1545) | exact return/error shown | [ ] |
| 1714 | `HUF_readStats` | `if ((rankStats[1] < 2) \|\| (rankStats[1] & 1)) return ERROR(corruption_detected); /* by construction : at least 2 elts of rank 1, must be even */` (c_src/src/legacy/zstd_v02.c:1551) | exact return/error shown | [ ] |
| 1715 | `HUF_readDTableX2` | `if (tableLog > DTable[0]) return ERROR(tableLog_tooLarge); /* DTable is too small */` (c_src/src/legacy/zstd_v02.c:1584) | exact return/error shown | [ ] |
| 1716 | `HUF_decompress4X2_usingDTable` | `if (cSrcSize < 10) return ERROR(corruption_detected); /* strict minimum : jump table + 1 byte per stream */` (c_src/src/legacy/zstd_v02.c:1661) | exact return/error shown | [ ] |
| 1717 | `HUF_decompress4X2_usingDTable` | `if (length4 > cSrcSize) return ERROR(corruption_detected); /* overflow */` (c_src/src/legacy/zstd_v02.c:1697) | exact return/error shown | [ ] |
| 1718 | `HUF_decompress4X2_usingDTable` | `if (HUF_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v02.c:1699) | exact return/error shown | [ ] |
| 1719 | `HUF_decompress4X2_usingDTable` | `if (HUF_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v02.c:1701) | exact return/error shown | [ ] |
| 1720 | `HUF_decompress4X2_usingDTable` | `if (HUF_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v02.c:1703) | exact return/error shown | [ ] |
| 1721 | `HUF_decompress4X2_usingDTable` | `if (HUF_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v02.c:1705) | exact return/error shown | [ ] |
| 1722 | `HUF_decompress4X2_usingDTable` | `if (op1 > opStart2) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v02.c:1732) | exact return/error shown | [ ] |
| 1723 | `HUF_decompress4X2_usingDTable` | `if (op2 > opStart3) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v02.c:1733) | exact return/error shown | [ ] |
| 1724 | `HUF_decompress4X2_usingDTable` | `if (op3 > opStart4) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v02.c:1734) | exact return/error shown | [ ] |
| 1725 | `HUF_decompress4X2_usingDTable` | `if (!endSignal) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v02.c:1745) | exact return/error shown | [ ] |
| 1726 | `HUF_decompress4X2` | `if (HUF_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v02.c:1760) | exact return/error shown | [ ] |
| 1727 | `HUF_decompress4X2` | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v02.c:1761) | exact return/error shown | [ ] |
| 1728 | `HUF_readDTableX4` | `if (memLog > HUF_ABSOLUTEMAX_TABLELOG) return ERROR(tableLog_tooLarge);` (c_src/src/legacy/zstd_v02.c:1882) | exact return/error shown | [ ] |
| 1729 | `HUF_readDTableX4` | `if (tableLog > memLog) return ERROR(tableLog_tooLarge); /* DTable can't fit code depth */` (c_src/src/legacy/zstd_v02.c:1889) | exact return/error shown | [ ] |
| 1730 | `HUF_readDTableX4` | `{if (!maxW) return ERROR(GENERIC); } /* necessarily finds a solution before maxW==0 */` (c_src/src/legacy/zstd_v02.c:1893) | exact return/error shown | [ ] |
| 1731 | `HUF_decompress4X4_usingDTable` | `if (cSrcSize < 10) return ERROR(corruption_detected); /* strict minimum : jump table + 1 byte per stream */` (c_src/src/legacy/zstd_v02.c:2023) | exact return/error shown | [ ] |
| 1732 | `HUF_decompress4X4_usingDTable` | `if (length4 > cSrcSize) return ERROR(corruption_detected); /* overflow */` (c_src/src/legacy/zstd_v02.c:2059) | exact return/error shown | [ ] |
| 1733 | `HUF_decompress4X4_usingDTable` | `if (HUF_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v02.c:2061) | exact return/error shown | [ ] |
| 1734 | `HUF_decompress4X4_usingDTable` | `if (HUF_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v02.c:2063) | exact return/error shown | [ ] |
| 1735 | `HUF_decompress4X4_usingDTable` | `if (HUF_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v02.c:2065) | exact return/error shown | [ ] |
| 1736 | `HUF_decompress4X4_usingDTable` | `if (HUF_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v02.c:2067) | exact return/error shown | [ ] |
| 1737 | `HUF_decompress4X4_usingDTable` | `if (op1 > opStart2) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v02.c:2094) | exact return/error shown | [ ] |
| 1738 | `HUF_decompress4X4_usingDTable` | `if (op2 > opStart3) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v02.c:2095) | exact return/error shown | [ ] |
| 1739 | `HUF_decompress4X4_usingDTable` | `if (op3 > opStart4) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v02.c:2096) | exact return/error shown | [ ] |
| 1740 | `HUF_decompress4X4_usingDTable` | `if (!endSignal) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v02.c:2107) | exact return/error shown | [ ] |
| 1741 | `HUF_decompress4X4` | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v02.c:2122) | exact return/error shown | [ ] |
| 1742 | `HUF_readDTableX6` | `if (memLog > HUF_ABSOLUTEMAX_TABLELOG) return ERROR(tableLog_tooLarge);` (c_src/src/legacy/zstd_v02.c:2215) | exact return/error shown | [ ] |
| 1743 | `HUF_readDTableX6` | `if (tableLog > memLog) return ERROR(tableLog_tooLarge); /* DTable is too small */` (c_src/src/legacy/zstd_v02.c:2222) | exact return/error shown | [ ] |
| 1744 | `HUF_readDTableX6` | `{ if (!maxW) return ERROR(GENERIC); } /* necessarily finds a solution before maxW==0 */` (c_src/src/legacy/zstd_v02.c:2226) | exact return/error shown | [ ] |
| 1745 | `HUF_decompress4X6_usingDTable` | `if (cSrcSize < 10) return ERROR(corruption_detected); /* strict minimum : jump table + 1 byte per stream */` (c_src/src/legacy/zstd_v02.c:2378) | exact return/error shown | [ ] |
| 1746 | `HUF_decompress4X6_usingDTable` | `if (length4 > cSrcSize) return ERROR(corruption_detected); /* overflow */` (c_src/src/legacy/zstd_v02.c:2416) | exact return/error shown | [ ] |
| 1747 | `HUF_decompress4X6_usingDTable` | `if (HUF_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v02.c:2418) | exact return/error shown | [ ] |
| 1748 | `HUF_decompress4X6_usingDTable` | `if (HUF_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v02.c:2420) | exact return/error shown | [ ] |
| 1749 | `HUF_decompress4X6_usingDTable` | `if (HUF_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v02.c:2422) | exact return/error shown | [ ] |
| 1750 | `HUF_decompress4X6_usingDTable` | `if (HUF_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v02.c:2424) | exact return/error shown | [ ] |
| 1751 | `HUF_decompress4X6_usingDTable` | `if (op1 > opStart2) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v02.c:2451) | exact return/error shown | [ ] |
| 1752 | `HUF_decompress4X6_usingDTable` | `if (op2 > opStart3) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v02.c:2452) | exact return/error shown | [ ] |
| 1753 | `HUF_decompress4X6_usingDTable` | `if (op3 > opStart4) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v02.c:2453) | exact return/error shown | [ ] |
| 1754 | `HUF_decompress4X6_usingDTable` | `if (!endSignal) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v02.c:2464) | exact return/error shown | [ ] |
| 1755 | `HUF_decompress4X6` | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v02.c:2479) | exact return/error shown | [ ] |
| 1756 | `HUF_decompress` | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v02.c:2526) | exact return/error shown | [ ] |
| 1757 | `HUF_decompress` | `if (cSrcSize > dstSize) return ERROR(corruption_detected); /* invalid */` (c_src/src/legacy/zstd_v02.c:2527) | exact return/error shown | [ ] |
| 1758 | `ZSTD_isError` | `static unsigned ZSTD_isError(size_t code) { return ERR_isError(code); }` (c_src/src/legacy/zstd_v02.c:2733) | exact return/error shown | [ ] |
| 1759 | `ZSTD_getcBlockSize` | `if (srcSize < 3) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v02.c:2762) | exact return/error shown | [ ] |
| 1760 | `ZSTD_copyUncompressedBlock` | `if (srcSize > maxDstSize) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v02.c:2777) | exact return/error shown | [ ] |
| 1761 | `ZSTD_decompressLiterals` | `if (litSize > *maxDstSizePtr) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v02.c:2795) | exact return/error shown | [ ] |
| 1762 | `ZSTD_decompressLiterals` | `if (litCSize + 5 > srcSize) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v02.c:2796) | exact return/error shown | [ ] |
| 1763 | `ZSTD_decompressLiterals` | `if (HUF_isError(HUF_decompress(dst, litSize, ip+5, litCSize))) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v02.c:2798) | exact return/error shown | [ ] |
| 1764 | `ZSTD_decodeLiteralsBlock` | `if (srcSize < MIN_CBLOCK_SIZE) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v02.c:2814) | exact return/error shown | [ ] |
| 1765 | `ZSTD_decodeLiteralsBlock` | `if (litSize > BLOCKSIZE) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v02.c:2833) | exact return/error shown | [ ] |
| 1766 | `ZSTD_decodeLiteralsBlock` | `if (litSize > srcSize-3) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v02.c:2834) | exact return/error shown | [ ] |
| 1767 | `ZSTD_decodeLiteralsBlock` | `if (litSize > BLOCKSIZE) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v02.c:2849) | exact return/error shown | [ ] |
| 1768 | `ZSTD_decodeSeqHeaders` | `if (srcSize < 5) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v02.c:2871) | exact return/error shown | [ ] |
| 1769 | `ZSTD_decodeSeqHeaders` | `if (ip > iend-3) return ERROR(srcSize_wrong); /* min : all 3 are "raw", hence no header, but at least xxLog bits per type */` (c_src/src/legacy/zstd_v02.c:2895) | exact return/error shown | [ ] |
| 1770 | `ZSTD_decodeSeqHeaders` | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` (c_src/src/legacy/zstd_v02.c:2914) | exact return/error shown | [ ] |
| 1771 | `ZSTD_decodeSeqHeaders` | `if (LLlog > LLFSELog) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v02.c:2915) | exact return/error shown | [ ] |
| 1772 | `ZSTD_decodeSeqHeaders` | `if (ip > iend-2) return ERROR(srcSize_wrong); /* min : "raw", hence no header, but at least xxLog bits */` (c_src/src/legacy/zstd_v02.c:2924) | exact return/error shown | [ ] |
| 1773 | `ZSTD_decodeSeqHeaders` | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` (c_src/src/legacy/zstd_v02.c:2933) | exact return/error shown | [ ] |
| 1774 | `ZSTD_decodeSeqHeaders` | `if (Offlog > OffFSELog) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v02.c:2934) | exact return/error shown | [ ] |
| 1775 | `ZSTD_decodeSeqHeaders` | `if (ip > iend-2) return ERROR(srcSize_wrong); /* min : "raw", hence no header, but at least xxLog bits */` (c_src/src/legacy/zstd_v02.c:2943) | exact return/error shown | [ ] |
| 1776 | `ZSTD_decodeSeqHeaders` | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` (c_src/src/legacy/zstd_v02.c:2951) | exact return/error shown | [ ] |
| 1777 | `ZSTD_decodeSeqHeaders` | `if (MLlog > MLFSELog) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v02.c:2952) | exact return/error shown | [ ] |
| 1778 | `ZSTD_execSequence` | `if (seqLength > (size_t)(oend - op)) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v02.c:3058) | exact return/error shown | [ ] |
| 1779 | `ZSTD_execSequence` | `if (sequence.litLength > (size_t)(litLimit - *litPtr)) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v02.c:3059) | exact return/error shown | [ ] |
| 1780 | `ZSTD_execSequence` | `if (oLitEnd > oend_8) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v02.c:3061) | exact return/error shown | [ ] |
| 1781 | `ZSTD_execSequence` | `if (sequence.offset > (U32)(oLitEnd - base)) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v02.c:3062) | exact return/error shown | [ ] |
| 1782 | `ZSTD_execSequence` | `if (oMatchEnd > oend) return ERROR(dstSize_tooSmall); /* overwrite beyond dst buffer */` (c_src/src/legacy/zstd_v02.c:3064) | exact return/error shown | [ ] |
| 1783 | `ZSTD_execSequence` | `if (litEnd > litLimit) return ERROR(corruption_detected); /* overRead beyond lit buffer */` (c_src/src/legacy/zstd_v02.c:3065) | exact return/error shown | [ ] |
| 1784 | `ZSTD_execSequence` | `if (sequence.offset > (size_t)op) return ERROR(corruption_detected); /* address space overflow test (this test seems kept by clang optimizer) */` (c_src/src/legacy/zstd_v02.c:3077) | exact return/error shown | [ ] |
| 1785 | `ZSTD_execSequence` | `//if (match > op) return ERROR(corruption_detected); /* address space overflow test (is clang optimizer removing this test ?) */` (c_src/src/legacy/zstd_v02.c:3078) | exact return/error shown | [ ] |
| 1786 | `ZSTD_execSequence` | `if (match < base) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v02.c:3079) | exact return/error shown | [ ] |
| 1787 | `ZSTD_decompressSequences` | `if (ZSTD_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v02.c:3143) | exact return/error shown | [ ] |
| 1788 | `ZSTD_decompressSequences` | `if (ERR_isError(errorCode)) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v02.c:3156) | exact return/error shown | [ ] |
| 1789 | `ZSTD_decompressSequences` | `if ( !BIT_endOfDStream(&(seqState.DStream)) ) return ERROR(corruption_detected); /* requested too much : data is corrupted */` (c_src/src/legacy/zstd_v02.c:3172) | exact return/error shown | [ ] |
| 1790 | `ZSTD_decompressSequences` | `if (nbSeq<0) return ERROR(corruption_detected); /* requested too many sequences : data is corrupted */` (c_src/src/legacy/zstd_v02.c:3173) | exact return/error shown | [ ] |
| 1791 | `ZSTD_decompressSequences` | `if (litPtr > litEnd) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v02.c:3178) | exact return/error shown | [ ] |
| 1792 | `ZSTD_decompressSequences` | `if (op+lastLLSize > oend) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v02.c:3179) | exact return/error shown | [ ] |
| 1793 | `ZSTD_decompressDCtx` | `if (srcSize < ZSTD_frameHeaderSize+ZSTD_blockHeaderSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v02.c:3221) | exact return/error shown | [ ] |
| 1794 | `ZSTD_decompressDCtx` | `if (magicNumber != ZSTD_magicNumber) return ERROR(prefix_unknown);` (c_src/src/legacy/zstd_v02.c:3223) | exact return/error shown | [ ] |
| 1795 | `ZSTD_decompressDCtx` | `if (cBlockSize > remainingSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v02.c:3235) | exact return/error shown | [ ] |
| 1796 | `ZSTD_decompressDCtx` | `return ERROR(GENERIC); /* not yet supported */` (c_src/src/legacy/zstd_v02.c:3246) | exact return/error shown | [ ] |
| 1797 | `ZSTD_decompressDCtx` | `if (remainingSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v02.c:3250) | exact return/error shown | [ ] |
| 1798 | `ZSTD_decompressDCtx` | `return ERROR(GENERIC); /* impossible */` (c_src/src/legacy/zstd_v02.c:3253) | exact return/error shown | [ ] |
| 1799 | `ZSTD_createDCtx` | `if (dctx==NULL) return NULL;` (c_src/src/legacy/zstd_v02.c:3344) | exact return/error shown | [ ] |
| 1800 | `ZSTD_decompressContinue` | `if (srcSize != ctx->expected) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v02.c:3363) | exact return/error shown | [ ] |
| 1801 | `ZSTD_decompressContinue` | `if (magicNumber != ZSTD_magicNumber) return ERROR(prefix_unknown);` (c_src/src/legacy/zstd_v02.c:3372) | exact return/error shown | [ ] |
| 1802 | `ZSTD_decompressContinue` | `return ERROR(GENERIC); /* not yet handled */` (c_src/src/legacy/zstd_v02.c:3411) | exact return/error shown | [ ] |
| 1803 | `ZSTD_decompressContinue` | `return ERROR(GENERIC);` (c_src/src/legacy/zstd_v02.c:3417) | exact return/error shown | [ ] |
| 1804 | `ZSTDv02_isError` | `return ZSTD_isError(code);` (c_src/src/legacy/zstd_v02.c:3433) | exact return/error shown | [ ] |
| 1805 | `BIT_initDStream` | `if (srcSize < 1) { memset(bitD, 0, sizeof(*bitD)); return ERROR(srcSize_wrong); }` (c_src/src/legacy/zstd_v03.c:327) | exact return/error shown | [ ] |
| 1806 | `BIT_initDStream` | `if (contain32 == 0) return ERROR(GENERIC); /* endMark not present */` (c_src/src/legacy/zstd_v03.c:336) | exact return/error shown | [ ] |
| 1807 | `BIT_initDStream` | `if (contain32 == 0) return ERROR(GENERIC); /* endMark not present */` (c_src/src/legacy/zstd_v03.c:362) | exact return/error shown | [ ] |
| 1808 | `ERR_getErrorName` | `return codeError;` (c_src/src/legacy/zstd_v03.c:531) | exact return/error shown | [ ] |
| 1809 | `FSE_buildDTable` | `if (maxSymbolValue > FSE_MAX_SYMBOL_VALUE) return ERROR(maxSymbolValue_tooLarge);` (c_src/src/legacy/zstd_v03.c:1051) | exact return/error shown | [ ] |
| 1810 | `FSE_buildDTable` | `if (tableLog > FSE_MAX_TABLELOG) return ERROR(tableLog_tooLarge);` (c_src/src/legacy/zstd_v03.c:1052) | exact return/error shown | [ ] |
| 1811 | `FSE_buildDTable` | `if (position!=0) return ERROR(GENERIC); /* position must reach all cells once, otherwise normalizedCounter is incorrect */` (c_src/src/legacy/zstd_v03.c:1082) | exact return/error shown | [ ] |
| 1812 | `FSE_isError` | `static unsigned FSE_isError(size_t code) { return ERR_isError(code); }` (c_src/src/legacy/zstd_v03.c:1106) | exact return/error shown | [ ] |
| 1813 | `FSE_readNCount` | `if (hbSize < 4) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v03.c:1131) | exact return/error shown | [ ] |
| 1814 | `FSE_readNCount` | `if (nbBits > FSE_TABLELOG_ABSOLUTE_MAX) return ERROR(tableLog_tooLarge);` (c_src/src/legacy/zstd_v03.c:1134) | exact return/error shown | [ ] |
| 1815 | `FSE_readNCount` | `if (n0 > *maxSVPtr) return ERROR(maxSymbolValue_tooSmall);` (c_src/src/legacy/zstd_v03.c:1169) | exact return/error shown | [ ] |
| 1816 | `FSE_readNCount` | `if (remaining != 1) return ERROR(GENERIC);` (c_src/src/legacy/zstd_v03.c:1221) | exact return/error shown | [ ] |
| 1817 | `FSE_readNCount` | `if ((size_t)(ip-istart) > hbSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v03.c:1225) | exact return/error shown | [ ] |
| 1818 | `FSE_buildDTable_raw` | `if (nbBits < 1) return ERROR(GENERIC); /* min size */` (c_src/src/legacy/zstd_v03.c:1261) | exact return/error shown | [ ] |
| 1819 | `FSE_decompress_usingDTable_generic` | `if (FSE_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v03.c:1293) | exact return/error shown | [ ] |
| 1820 | `FSE_decompress_usingDTable_generic` | `if (op==omax) return ERROR(dstSize_tooSmall); /* dst buffer is full, but cSrc unfinished */` (c_src/src/legacy/zstd_v03.c:1340) | exact return/error shown | [ ] |
| 1821 | `FSE_decompress_usingDTable_generic` | `return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v03.c:1342) | exact return/error shown | [ ] |
| 1822 | `FSE_decompress` | `if (cSrcSize<2) return ERROR(srcSize_wrong); /* too small input size */` (c_src/src/legacy/zstd_v03.c:1369) | exact return/error shown | [ ] |
| 1823 | `FSE_decompress` | `if (FSE_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v03.c:1373) | exact return/error shown | [ ] |
| 1824 | `FSE_decompress` | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong); /* too small input size */` (c_src/src/legacy/zstd_v03.c:1374) | exact return/error shown | [ ] |
| 1825 | `FSE_decompress` | `if (FSE_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v03.c:1379) | exact return/error shown | [ ] |
| 1826 | `HUF_isError` | `static unsigned HUF_isError(size_t code) { return ERR_isError(code); }` (c_src/src/legacy/zstd_v03.c:1451) | exact return/error shown | [ ] |
| 1827 | `HUF_readStats` | `if (!srcSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v03.c:1488) | exact return/error shown | [ ] |
| 1828 | `HUF_readStats` | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v03.c:1505) | exact return/error shown | [ ] |
| 1829 | `HUF_readStats` | `if (oSize >= hwSize) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v03.c:1506) | exact return/error shown | [ ] |
| 1830 | `HUF_readStats` | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v03.c:1517) | exact return/error shown | [ ] |
| 1831 | `HUF_readStats` | `if (huffWeight[n] >= HUF_ABSOLUTEMAX_TABLELOG) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v03.c:1527) | exact return/error shown | [ ] |
| 1832 | `HUF_readStats` | `if (weightTotal == 0) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v03.c:1531) | exact return/error shown | [ ] |
| 1833 | `HUF_readStats` | `if (tableLog > HUF_ABSOLUTEMAX_TABLELOG) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v03.c:1535) | exact return/error shown | [ ] |
| 1834 | `HUF_readStats` | `if (verif != rest) return ERROR(corruption_detected); /* last value must be a clean power of 2 */` (c_src/src/legacy/zstd_v03.c:1541) | exact return/error shown | [ ] |
| 1835 | `HUF_readStats` | `if ((rankStats[1] < 2) \|\| (rankStats[1] & 1)) return ERROR(corruption_detected); /* by construction : at least 2 elts of rank 1, must be even */` (c_src/src/legacy/zstd_v03.c:1547) | exact return/error shown | [ ] |
| 1836 | `HUF_readDTableX2` | `if (tableLog > DTable[0]) return ERROR(tableLog_tooLarge); /* DTable is too small */` (c_src/src/legacy/zstd_v03.c:1580) | exact return/error shown | [ ] |
| 1837 | `HUF_decompress4X2_usingDTable` | `if (cSrcSize < 10) return ERROR(corruption_detected); /* strict minimum : jump table + 1 byte per stream */` (c_src/src/legacy/zstd_v03.c:1657) | exact return/error shown | [ ] |
| 1838 | `HUF_decompress4X2_usingDTable` | `if (length4 > cSrcSize) return ERROR(corruption_detected); /* overflow */` (c_src/src/legacy/zstd_v03.c:1693) | exact return/error shown | [ ] |
| 1839 | `HUF_decompress4X2_usingDTable` | `if (HUF_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v03.c:1695) | exact return/error shown | [ ] |
| 1840 | `HUF_decompress4X2_usingDTable` | `if (HUF_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v03.c:1697) | exact return/error shown | [ ] |
| 1841 | `HUF_decompress4X2_usingDTable` | `if (HUF_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v03.c:1699) | exact return/error shown | [ ] |
| 1842 | `HUF_decompress4X2_usingDTable` | `if (HUF_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v03.c:1701) | exact return/error shown | [ ] |
| 1843 | `HUF_decompress4X2_usingDTable` | `if (op1 > opStart2) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v03.c:1728) | exact return/error shown | [ ] |
| 1844 | `HUF_decompress4X2_usingDTable` | `if (op2 > opStart3) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v03.c:1729) | exact return/error shown | [ ] |
| 1845 | `HUF_decompress4X2_usingDTable` | `if (op3 > opStart4) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v03.c:1730) | exact return/error shown | [ ] |
| 1846 | `HUF_decompress4X2_usingDTable` | `if (!endSignal) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v03.c:1741) | exact return/error shown | [ ] |
| 1847 | `HUF_decompress4X2` | `if (HUF_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v03.c:1756) | exact return/error shown | [ ] |
| 1848 | `HUF_decompress4X2` | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v03.c:1757) | exact return/error shown | [ ] |
| 1849 | `HUF_readDTableX4` | `if (memLog > HUF_ABSOLUTEMAX_TABLELOG) return ERROR(tableLog_tooLarge);` (c_src/src/legacy/zstd_v03.c:1878) | exact return/error shown | [ ] |
| 1850 | `HUF_readDTableX4` | `if (tableLog > memLog) return ERROR(tableLog_tooLarge); /* DTable can't fit code depth */` (c_src/src/legacy/zstd_v03.c:1885) | exact return/error shown | [ ] |
| 1851 | `HUF_readDTableX4` | `{ if (!maxW) return ERROR(GENERIC); } /* necessarily finds a solution before maxW==0 */` (c_src/src/legacy/zstd_v03.c:1889) | exact return/error shown | [ ] |
| 1852 | `HUF_decompress4X4_usingDTable` | `if (cSrcSize < 10) return ERROR(corruption_detected); /* strict minimum : jump table + 1 byte per stream */` (c_src/src/legacy/zstd_v03.c:2019) | exact return/error shown | [ ] |
| 1853 | `HUF_decompress4X4_usingDTable` | `if (length4 > cSrcSize) return ERROR(corruption_detected); /* overflow */` (c_src/src/legacy/zstd_v03.c:2055) | exact return/error shown | [ ] |
| 1854 | `HUF_decompress4X4_usingDTable` | `if (HUF_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v03.c:2057) | exact return/error shown | [ ] |
| 1855 | `HUF_decompress4X4_usingDTable` | `if (HUF_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v03.c:2059) | exact return/error shown | [ ] |
| 1856 | `HUF_decompress4X4_usingDTable` | `if (HUF_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v03.c:2061) | exact return/error shown | [ ] |
| 1857 | `HUF_decompress4X4_usingDTable` | `if (HUF_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v03.c:2063) | exact return/error shown | [ ] |
| 1858 | `HUF_decompress4X4_usingDTable` | `if (op1 > opStart2) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v03.c:2090) | exact return/error shown | [ ] |
| 1859 | `HUF_decompress4X4_usingDTable` | `if (op2 > opStart3) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v03.c:2091) | exact return/error shown | [ ] |
| 1860 | `HUF_decompress4X4_usingDTable` | `if (op3 > opStart4) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v03.c:2092) | exact return/error shown | [ ] |
| 1861 | `HUF_decompress4X4_usingDTable` | `if (!endSignal) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v03.c:2103) | exact return/error shown | [ ] |
| 1862 | `HUF_decompress4X4` | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v03.c:2118) | exact return/error shown | [ ] |
| 1863 | `HUF_decompress` | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v03.c:2165) | exact return/error shown | [ ] |
| 1864 | `HUF_decompress` | `if (cSrcSize > dstSize) return ERROR(corruption_detected); /* invalid */` (c_src/src/legacy/zstd_v03.c:2166) | exact return/error shown | [ ] |
| 1865 | `ZSTD_isError` | `static unsigned ZSTD_isError(size_t code) { return ERR_isError(code); }` (c_src/src/legacy/zstd_v03.c:2373) | exact return/error shown | [ ] |
| 1866 | `ZSTD_getcBlockSize` | `if (srcSize < 3) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v03.c:2402) | exact return/error shown | [ ] |
| 1867 | `ZSTD_copyUncompressedBlock` | `if (srcSize > maxDstSize) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v03.c:2417) | exact return/error shown | [ ] |
| 1868 | `ZSTD_decompressLiterals` | `if (litSize > *maxDstSizePtr) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v03.c:2435) | exact return/error shown | [ ] |
| 1869 | `ZSTD_decompressLiterals` | `if (litCSize + 5 > srcSize) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v03.c:2436) | exact return/error shown | [ ] |
| 1870 | `ZSTD_decompressLiterals` | `if (HUF_isError(HUF_decompress(dst, litSize, ip+5, litCSize))) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v03.c:2438) | exact return/error shown | [ ] |
| 1871 | `ZSTD_decodeLiteralsBlock` | `if (srcSize < MIN_CBLOCK_SIZE) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v03.c:2454) | exact return/error shown | [ ] |
| 1872 | `ZSTD_decodeLiteralsBlock` | `if (litSize > BLOCKSIZE) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v03.c:2473) | exact return/error shown | [ ] |
| 1873 | `ZSTD_decodeLiteralsBlock` | `if (litSize > srcSize-3) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v03.c:2474) | exact return/error shown | [ ] |
| 1874 | `ZSTD_decodeLiteralsBlock` | `if (litSize > BLOCKSIZE) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v03.c:2489) | exact return/error shown | [ ] |
| 1875 | `ZSTD_decodeSeqHeaders` | `if (srcSize < 5) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v03.c:2511) | exact return/error shown | [ ] |
| 1876 | `ZSTD_decodeSeqHeaders` | `if (ip > iend-3) return ERROR(srcSize_wrong); /* min : all 3 are "raw", hence no header, but at least xxLog bits per type */` (c_src/src/legacy/zstd_v03.c:2535) | exact return/error shown | [ ] |
| 1877 | `ZSTD_decodeSeqHeaders` | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` (c_src/src/legacy/zstd_v03.c:2554) | exact return/error shown | [ ] |
| 1878 | `ZSTD_decodeSeqHeaders` | `if (LLlog > LLFSELog) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v03.c:2555) | exact return/error shown | [ ] |
| 1879 | `ZSTD_decodeSeqHeaders` | `if (ip > iend-2) return ERROR(srcSize_wrong); /* min : "raw", hence no header, but at least xxLog bits */` (c_src/src/legacy/zstd_v03.c:2564) | exact return/error shown | [ ] |
| 1880 | `ZSTD_decodeSeqHeaders` | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` (c_src/src/legacy/zstd_v03.c:2573) | exact return/error shown | [ ] |
| 1881 | `ZSTD_decodeSeqHeaders` | `if (Offlog > OffFSELog) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v03.c:2574) | exact return/error shown | [ ] |
| 1882 | `ZSTD_decodeSeqHeaders` | `if (ip > iend-2) return ERROR(srcSize_wrong); /* min : "raw", hence no header, but at least xxLog bits */` (c_src/src/legacy/zstd_v03.c:2583) | exact return/error shown | [ ] |
| 1883 | `ZSTD_decodeSeqHeaders` | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` (c_src/src/legacy/zstd_v03.c:2591) | exact return/error shown | [ ] |
| 1884 | `ZSTD_decodeSeqHeaders` | `if (MLlog > MLFSELog) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v03.c:2592) | exact return/error shown | [ ] |
| 1885 | `ZSTD_execSequence` | `if (seqLength > (size_t)(oend - op)) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v03.c:2698) | exact return/error shown | [ ] |
| 1886 | `ZSTD_execSequence` | `if (sequence.litLength > (size_t)(litLimit - *litPtr)) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v03.c:2699) | exact return/error shown | [ ] |
| 1887 | `ZSTD_execSequence` | `if (oLitEnd > oend_8) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v03.c:2701) | exact return/error shown | [ ] |
| 1888 | `ZSTD_execSequence` | `if (sequence.offset > (U32)(oLitEnd - base)) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v03.c:2702) | exact return/error shown | [ ] |
| 1889 | `ZSTD_execSequence` | `if (oMatchEnd > oend) return ERROR(dstSize_tooSmall); /* overwrite beyond dst buffer */` (c_src/src/legacy/zstd_v03.c:2704) | exact return/error shown | [ ] |
| 1890 | `ZSTD_execSequence` | `if (litEnd > litLimit) return ERROR(corruption_detected); /* overRead beyond lit buffer */` (c_src/src/legacy/zstd_v03.c:2705) | exact return/error shown | [ ] |
| 1891 | `ZSTD_execSequence` | `if (sequence.offset > (size_t)op) return ERROR(corruption_detected); /* address space overflow test (this test seems kept by clang optimizer) */` (c_src/src/legacy/zstd_v03.c:2716) | exact return/error shown | [ ] |
| 1892 | `ZSTD_execSequence` | `//if (match > op) return ERROR(corruption_detected); /* address space overflow test (is clang optimizer removing this test ?) */` (c_src/src/legacy/zstd_v03.c:2717) | exact return/error shown | [ ] |
| 1893 | `ZSTD_execSequence` | `if (match < base) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v03.c:2718) | exact return/error shown | [ ] |
| 1894 | `ZSTD_decompressSequences` | `if (ZSTD_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v03.c:2782) | exact return/error shown | [ ] |
| 1895 | `ZSTD_decompressSequences` | `if (ERR_isError(errorCode)) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v03.c:2795) | exact return/error shown | [ ] |
| 1896 | `ZSTD_decompressSequences` | `if ( !BIT_endOfDStream(&(seqState.DStream)) ) return ERROR(corruption_detected); /* requested too much : data is corrupted */` (c_src/src/legacy/zstd_v03.c:2811) | exact return/error shown | [ ] |
| 1897 | `ZSTD_decompressSequences` | `if (nbSeq<0) return ERROR(corruption_detected); /* requested too many sequences : data is corrupted */` (c_src/src/legacy/zstd_v03.c:2812) | exact return/error shown | [ ] |
| 1898 | `ZSTD_decompressSequences` | `if (litPtr > litEnd) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v03.c:2817) | exact return/error shown | [ ] |
| 1899 | `ZSTD_decompressSequences` | `if (op+lastLLSize > oend) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v03.c:2818) | exact return/error shown | [ ] |
| 1900 | `ZSTD_decompressDCtx` | `if (srcSize < ZSTD_frameHeaderSize+ZSTD_blockHeaderSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v03.c:2860) | exact return/error shown | [ ] |
| 1901 | `ZSTD_decompressDCtx` | `if (magicNumber != ZSTD_magicNumber) return ERROR(prefix_unknown);` (c_src/src/legacy/zstd_v03.c:2862) | exact return/error shown | [ ] |
| 1902 | `ZSTD_decompressDCtx` | `if (cBlockSize > remainingSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v03.c:2874) | exact return/error shown | [ ] |
| 1903 | `ZSTD_decompressDCtx` | `return ERROR(GENERIC); /* not yet supported */` (c_src/src/legacy/zstd_v03.c:2885) | exact return/error shown | [ ] |
| 1904 | `ZSTD_decompressDCtx` | `if (remainingSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v03.c:2889) | exact return/error shown | [ ] |
| 1905 | `ZSTD_decompressDCtx` | `return ERROR(GENERIC); /* impossible */` (c_src/src/legacy/zstd_v03.c:2892) | exact return/error shown | [ ] |
| 1906 | `ZSTD_createDCtx` | `if (dctx==NULL) return NULL;` (c_src/src/legacy/zstd_v03.c:2984) | exact return/error shown | [ ] |
| 1907 | `ZSTD_decompressContinue` | `if (srcSize != ctx->expected) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v03.c:3003) | exact return/error shown | [ ] |
| 1908 | `ZSTD_decompressContinue` | `if (magicNumber != ZSTD_magicNumber) return ERROR(prefix_unknown);` (c_src/src/legacy/zstd_v03.c:3012) | exact return/error shown | [ ] |
| 1909 | `ZSTD_decompressContinue` | `return ERROR(GENERIC); /* not yet handled */` (c_src/src/legacy/zstd_v03.c:3051) | exact return/error shown | [ ] |
| 1910 | `ZSTD_decompressContinue` | `return ERROR(GENERIC);` (c_src/src/legacy/zstd_v03.c:3057) | exact return/error shown | [ ] |
| 1911 | `ZSTDv03_isError` | `return ZSTD_isError(code);` (c_src/src/legacy/zstd_v03.c:3073) | exact return/error shown | [ ] |
| 1912 | `<file scope/macro>` | `# define assert(condition) ((void)0) #endif /**************************************************************** * Memory I/O *****************************************************************/ MEM_STATIC unsigned MEM_32bits(void) { return sizeof(void*)==4; }` (c_src/src/legacy/zstd_v04.c:75) | assertion/abort | [ ] |
| 1913 | `BIT_initDStream` | `if (srcSize < 1) { memset(bitD, 0, sizeof(*bitD)); return ERROR(srcSize_wrong); }` (c_src/src/legacy/zstd_v04.c:603) | exact return/error shown | [ ] |
| 1914 | `BIT_initDStream` | `if (contain32 == 0) return ERROR(GENERIC); /* endMark not present */` (c_src/src/legacy/zstd_v04.c:612) | exact return/error shown | [ ] |
| 1915 | `BIT_initDStream` | `if (contain32 == 0) return ERROR(GENERIC); /* endMark not present */` (c_src/src/legacy/zstd_v04.c:632) | exact return/error shown | [ ] |
| 1916 | `FSE_buildDTable` | `if (maxSymbolValue > FSE_MAX_SYMBOL_VALUE) return ERROR(maxSymbolValue_tooLarge);` (c_src/src/legacy/zstd_v04.c:1033) | exact return/error shown | [ ] |
| 1917 | `FSE_buildDTable` | `if (tableLog > FSE_MAX_TABLELOG) return ERROR(tableLog_tooLarge);` (c_src/src/legacy/zstd_v04.c:1034) | exact return/error shown | [ ] |
| 1918 | `FSE_buildDTable` | `if (position!=0) return ERROR(GENERIC); /* position must reach all cells once, otherwise normalizedCounter is incorrect */` (c_src/src/legacy/zstd_v04.c:1065) | exact return/error shown | [ ] |
| 1919 | `FSE_isError` | `static unsigned FSE_isError(size_t code) { return ERR_isError(code); }` (c_src/src/legacy/zstd_v04.c:1089) | exact return/error shown | [ ] |
| 1920 | `FSE_readNCount` | `if (hbSize < 4) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v04.c:1114) | exact return/error shown | [ ] |
| 1921 | `FSE_readNCount` | `if (nbBits > FSE_TABLELOG_ABSOLUTE_MAX) return ERROR(tableLog_tooLarge);` (c_src/src/legacy/zstd_v04.c:1117) | exact return/error shown | [ ] |
| 1922 | `FSE_readNCount` | `if (n0 > *maxSVPtr) return ERROR(maxSymbolValue_tooSmall);` (c_src/src/legacy/zstd_v04.c:1152) | exact return/error shown | [ ] |
| 1923 | `FSE_readNCount` | `if (remaining != 1) return ERROR(GENERIC);` (c_src/src/legacy/zstd_v04.c:1204) | exact return/error shown | [ ] |
| 1924 | `FSE_readNCount` | `if ((size_t)(ip-istart) > hbSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v04.c:1208) | exact return/error shown | [ ] |
| 1925 | `FSE_buildDTable_raw` | `if (nbBits < 1) return ERROR(GENERIC); /* min size */` (c_src/src/legacy/zstd_v04.c:1246) | exact return/error shown | [ ] |
| 1926 | `FSE_decompress_usingDTable_generic` | `if (FSE_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v04.c:1278) | exact return/error shown | [ ] |
| 1927 | `FSE_decompress_usingDTable_generic` | `if (op==omax) return ERROR(dstSize_tooSmall); /* dst buffer is full, but cSrc unfinished */` (c_src/src/legacy/zstd_v04.c:1325) | exact return/error shown | [ ] |
| 1928 | `FSE_decompress_usingDTable_generic` | `return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v04.c:1327) | exact return/error shown | [ ] |
| 1929 | `FSE_decompress` | `if (cSrcSize<2) return ERROR(srcSize_wrong); /* too small input size */` (c_src/src/legacy/zstd_v04.c:1357) | exact return/error shown | [ ] |
| 1930 | `FSE_decompress` | `if (FSE_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v04.c:1361) | exact return/error shown | [ ] |
| 1931 | `FSE_decompress` | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong); /* too small input size */` (c_src/src/legacy/zstd_v04.c:1362) | exact return/error shown | [ ] |
| 1932 | `FSE_decompress` | `if (FSE_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v04.c:1367) | exact return/error shown | [ ] |
| 1933 | `HUF_isError` | `static unsigned HUF_isError(size_t code) { return ERR_isError(code); }` (c_src/src/legacy/zstd_v04.c:1617) | exact return/error shown | [ ] |
| 1934 | `HUF_readStats` | `if (!srcSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v04.c:1647) | exact return/error shown | [ ] |
| 1935 | `HUF_readStats` | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v04.c:1664) | exact return/error shown | [ ] |
| 1936 | `HUF_readStats` | `if (oSize >= hwSize) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v04.c:1665) | exact return/error shown | [ ] |
| 1937 | `HUF_readStats` | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v04.c:1676) | exact return/error shown | [ ] |
| 1938 | `HUF_readStats` | `if (huffWeight[n] >= HUF_ABSOLUTEMAX_TABLELOG) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v04.c:1686) | exact return/error shown | [ ] |
| 1939 | `HUF_readStats` | `if (weightTotal == 0) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v04.c:1690) | exact return/error shown | [ ] |
| 1940 | `HUF_readStats` | `if (tableLog > HUF_ABSOLUTEMAX_TABLELOG) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v04.c:1694) | exact return/error shown | [ ] |
| 1941 | `HUF_readStats` | `if (verif != rest) return ERROR(corruption_detected); /* last value must be a clean power of 2 */` (c_src/src/legacy/zstd_v04.c:1700) | exact return/error shown | [ ] |
| 1942 | `HUF_readStats` | `if ((rankStats[1] < 2) \|\| (rankStats[1] & 1)) return ERROR(corruption_detected); /* by construction : at least 2 elts of rank 1, must be even */` (c_src/src/legacy/zstd_v04.c:1706) | exact return/error shown | [ ] |
| 1943 | `HUF_readDTableX2` | `if (tableLog > DTable[0]) return ERROR(tableLog_tooLarge); /* DTable is too small */` (c_src/src/legacy/zstd_v04.c:1738) | exact return/error shown | [ ] |
| 1944 | `HUF_decompress4X2_usingDTable` | `if (cSrcSize < 10) return ERROR(corruption_detected); /* strict minimum : jump table + 1 byte per stream */` (c_src/src/legacy/zstd_v04.c:1815) | exact return/error shown | [ ] |
| 1945 | `HUF_decompress4X2_usingDTable` | `if (length4 > cSrcSize) return ERROR(corruption_detected); /* overflow */` (c_src/src/legacy/zstd_v04.c:1850) | exact return/error shown | [ ] |
| 1946 | `HUF_decompress4X2_usingDTable` | `if (HUF_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v04.c:1852) | exact return/error shown | [ ] |
| 1947 | `HUF_decompress4X2_usingDTable` | `if (HUF_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v04.c:1854) | exact return/error shown | [ ] |
| 1948 | `HUF_decompress4X2_usingDTable` | `if (HUF_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v04.c:1856) | exact return/error shown | [ ] |
| 1949 | `HUF_decompress4X2_usingDTable` | `if (HUF_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v04.c:1858) | exact return/error shown | [ ] |
| 1950 | `HUF_decompress4X2_usingDTable` | `if (op1 > opStart2) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v04.c:1885) | exact return/error shown | [ ] |
| 1951 | `HUF_decompress4X2_usingDTable` | `if (op2 > opStart3) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v04.c:1886) | exact return/error shown | [ ] |
| 1952 | `HUF_decompress4X2_usingDTable` | `if (op3 > opStart4) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v04.c:1887) | exact return/error shown | [ ] |
| 1953 | `HUF_decompress4X2_usingDTable` | `if (!endSignal) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v04.c:1898) | exact return/error shown | [ ] |
| 1954 | `HUF_decompress4X2` | `if (HUF_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v04.c:1913) | exact return/error shown | [ ] |
| 1955 | `HUF_decompress4X2` | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v04.c:1914) | exact return/error shown | [ ] |
| 1956 | `HUF_readDTableX4` | `if (memLog > HUF_ABSOLUTEMAX_TABLELOG) return ERROR(tableLog_tooLarge);` (c_src/src/legacy/zstd_v04.c:2034) | exact return/error shown | [ ] |
| 1957 | `HUF_readDTableX4` | `if (tableLog > memLog) return ERROR(tableLog_tooLarge); /* DTable can't fit code depth */` (c_src/src/legacy/zstd_v04.c:2041) | exact return/error shown | [ ] |
| 1958 | `HUF_readDTableX4` | `{ if (!maxW) return ERROR(GENERIC); } /* necessarily finds a solution before maxW==0 */` (c_src/src/legacy/zstd_v04.c:2045) | exact return/error shown | [ ] |
| 1959 | `HUF_decompress4X4_usingDTable` | `if (cSrcSize < 10) return ERROR(corruption_detected); /* strict minimum : jump table + 1 byte per stream */` (c_src/src/legacy/zstd_v04.c:2173) | exact return/error shown | [ ] |
| 1960 | `HUF_decompress4X4_usingDTable` | `if (length4 > cSrcSize) return ERROR(corruption_detected); /* overflow */` (c_src/src/legacy/zstd_v04.c:2208) | exact return/error shown | [ ] |
| 1961 | `HUF_decompress4X4_usingDTable` | `if (HUF_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v04.c:2210) | exact return/error shown | [ ] |
| 1962 | `HUF_decompress4X4_usingDTable` | `if (HUF_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v04.c:2212) | exact return/error shown | [ ] |
| 1963 | `HUF_decompress4X4_usingDTable` | `if (HUF_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v04.c:2214) | exact return/error shown | [ ] |
| 1964 | `HUF_decompress4X4_usingDTable` | `if (HUF_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v04.c:2216) | exact return/error shown | [ ] |
| 1965 | `HUF_decompress4X4_usingDTable` | `if (op1 > opStart2) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v04.c:2243) | exact return/error shown | [ ] |
| 1966 | `HUF_decompress4X4_usingDTable` | `if (op2 > opStart3) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v04.c:2244) | exact return/error shown | [ ] |
| 1967 | `HUF_decompress4X4_usingDTable` | `if (op3 > opStart4) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v04.c:2245) | exact return/error shown | [ ] |
| 1968 | `HUF_decompress4X4_usingDTable` | `if (!endSignal) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v04.c:2256) | exact return/error shown | [ ] |
| 1969 | `HUF_decompress4X4` | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v04.c:2271) | exact return/error shown | [ ] |
| 1970 | `HUF_decompress` | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v04.c:2318) | exact return/error shown | [ ] |
| 1971 | `HUF_decompress` | `if (cSrcSize > dstSize) return ERROR(corruption_detected); /* invalid */` (c_src/src/legacy/zstd_v04.c:2319) | exact return/error shown | [ ] |
| 1972 | `ZSTD_isError` | `static unsigned ZSTD_isError(size_t code) { return ERR_isError(code); }` (c_src/src/legacy/zstd_v04.c:2429) | exact return/error shown | [ ] |
| 1973 | `ZSTD_createDCtx` | `if (dctx==NULL) return NULL;` (c_src/src/legacy/zstd_v04.c:2472) | exact return/error shown | [ ] |
| 1974 | `ZSTD_decodeFrameHeader_Part1` | `if (srcSize != ZSTD_frameHeaderSize_min) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v04.c:2494) | exact return/error shown | [ ] |
| 1975 | `ZSTD_decodeFrameHeader_Part1` | `if (magicNumber != ZSTD_MAGICNUMBER) return ERROR(prefix_unknown);` (c_src/src/legacy/zstd_v04.c:2496) | exact return/error shown | [ ] |
| 1976 | `ZSTD_getFrameParams` | `if (magicNumber != ZSTD_MAGICNUMBER) return ERROR(prefix_unknown);` (c_src/src/legacy/zstd_v04.c:2507) | exact return/error shown | [ ] |
| 1977 | `ZSTD_getFrameParams` | `if ((((const BYTE*)src)[4] >> 4) != 0) return ERROR(frameParameter_unsupported); /* reserved bits */` (c_src/src/legacy/zstd_v04.c:2510) | exact return/error shown | [ ] |
| 1978 | `ZSTD_decodeFrameHeader_Part2` | `if (srcSize != zc->headerSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v04.c:2521) | exact return/error shown | [ ] |
| 1979 | `ZSTD_decodeFrameHeader_Part2` | `if ((MEM_32bits()) && (zc->params.windowLog > 25)) return ERROR(frameParameter_unsupported);` (c_src/src/legacy/zstd_v04.c:2523) | exact return/error shown | [ ] |
| 1980 | `ZSTD_getcBlockSize` | `if (srcSize < 3) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v04.c:2534) | exact return/error shown | [ ] |
| 1981 | `ZSTD_copyRawBlock` | `if (srcSize > maxDstSize) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v04.c:2549) | exact return/error shown | [ ] |
| 1982 | `ZSTD_decompressLiterals` | `if (litSize > *maxDstSizePtr) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v04.c:2567) | exact return/error shown | [ ] |
| 1983 | `ZSTD_decompressLiterals` | `if (litCSize + 5 > srcSize) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v04.c:2568) | exact return/error shown | [ ] |
| 1984 | `ZSTD_decompressLiterals` | `if (HUF_isError(HUF_decompress(dst, litSize, ip+5, litCSize))) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v04.c:2570) | exact return/error shown | [ ] |
| 1985 | `ZSTD_decodeLiteralsBlock` | `if (srcSize < MIN_CBLOCK_SIZE) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v04.c:2585) | exact return/error shown | [ ] |
| 1986 | `ZSTD_decodeLiteralsBlock` | `if (litSize > BLOCKSIZE) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v04.c:2604) | exact return/error shown | [ ] |
| 1987 | `ZSTD_decodeLiteralsBlock` | `if (litSize > srcSize-3) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v04.c:2605) | exact return/error shown | [ ] |
| 1988 | `ZSTD_decodeLiteralsBlock` | `if (litSize > BLOCKSIZE) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v04.c:2619) | exact return/error shown | [ ] |
| 1989 | `ZSTD_decodeLiteralsBlock` | `return ERROR(corruption_detected); /* forbidden nominal case */` (c_src/src/legacy/zstd_v04.c:2626) | exact return/error shown | [ ] |
| 1990 | `ZSTD_decodeSeqHeaders` | `if (srcSize < 5) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v04.c:2643) | exact return/error shown | [ ] |
| 1991 | `ZSTD_decodeSeqHeaders` | `if (ip > iend-3) return ERROR(srcSize_wrong); /* min : all 3 are "raw", hence no header, but at least xxLog bits per type */` (c_src/src/legacy/zstd_v04.c:2667) | exact return/error shown | [ ] |
| 1992 | `ZSTD_decodeSeqHeaders` | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` (c_src/src/legacy/zstd_v04.c:2686) | exact return/error shown | [ ] |
| 1993 | `ZSTD_decodeSeqHeaders` | `if (LLlog > LLFSELog) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v04.c:2687) | exact return/error shown | [ ] |
| 1994 | `ZSTD_decodeSeqHeaders` | `if (ip > iend-2) return ERROR(srcSize_wrong); /* min : "raw", hence no header, but at least xxLog bits */` (c_src/src/legacy/zstd_v04.c:2696) | exact return/error shown | [ ] |
| 1995 | `ZSTD_decodeSeqHeaders` | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` (c_src/src/legacy/zstd_v04.c:2705) | exact return/error shown | [ ] |
| 1996 | `ZSTD_decodeSeqHeaders` | `if (Offlog > OffFSELog) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v04.c:2706) | exact return/error shown | [ ] |
| 1997 | `ZSTD_decodeSeqHeaders` | `if (ip > iend-2) return ERROR(srcSize_wrong); /* min : "raw", hence no header, but at least xxLog bits */` (c_src/src/legacy/zstd_v04.c:2715) | exact return/error shown | [ ] |
| 1998 | `ZSTD_decodeSeqHeaders` | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` (c_src/src/legacy/zstd_v04.c:2723) | exact return/error shown | [ ] |
| 1999 | `ZSTD_decodeSeqHeaders` | `if (MLlog > MLFSELog) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v04.c:2724) | exact return/error shown | [ ] |
| 2000 | `ZSTD_execSequence` | `if (seqLength > (size_t)(oend - op)) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v04.c:2826) | exact return/error shown | [ ] |
| 2001 | `ZSTD_execSequence` | `if (sequence.litLength > (size_t)(litLimit - *litPtr)) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v04.c:2827) | exact return/error shown | [ ] |
| 2002 | `ZSTD_execSequence` | `if (oLitEnd > oend_8) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v04.c:2829) | exact return/error shown | [ ] |
| 2003 | `ZSTD_execSequence` | `if (oMatchEnd > oend) return ERROR(dstSize_tooSmall); /* overwrite beyond dst buffer */` (c_src/src/legacy/zstd_v04.c:2831) | exact return/error shown | [ ] |
| 2004 | `ZSTD_execSequence` | `if (litEnd > litLimit) return ERROR(corruption_detected); /* overRead beyond lit buffer */` (c_src/src/legacy/zstd_v04.c:2832) | exact return/error shown | [ ] |
| 2005 | `ZSTD_execSequence` | `return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v04.c:2844) | exact return/error shown | [ ] |
| 2006 | `ZSTD_decompressSequences` | `if (ZSTD_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v04.c:2926) | exact return/error shown | [ ] |
| 2007 | `ZSTD_decompressSequences` | `if (ERR_isError(errorCode)) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v04.c:2940) | exact return/error shown | [ ] |
| 2008 | `ZSTD_decompressSequences` | `if ( !BIT_endOfDStream(&(seqState.DStream)) ) return ERROR(corruption_detected); /* DStream should be entirely and exactly consumed; otherwise data is corrupted */` (c_src/src/legacy/zstd_v04.c:2956) | exact return/error shown | [ ] |
| 2009 | `ZSTD_decompressSequences` | `if (litPtr > litEnd) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v04.c:2961) | exact return/error shown | [ ] |
| 2010 | `ZSTD_decompressSequences` | `if (op+lastLLSize > oend) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v04.c:2962) | exact return/error shown | [ ] |
| 2011 | `ZSTD_decompressBlock_internal` | `if (srcSize > BLOCKSIZE) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v04.c:2994) | exact return/error shown | [ ] |
| 2012 | `ZSTD_decompress_usingDict` | `if (srcSize < ZSTD_frameHeaderSize_min+ZSTD_blockHeaderSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v04.c:3036) | exact return/error shown | [ ] |
| 2013 | `ZSTD_decompress_usingDict` | `if (srcSize < frameHeaderSize+ZSTD_blockHeaderSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v04.c:3039) | exact return/error shown | [ ] |
| 2014 | `ZSTD_decompress_usingDict` | `if (cBlockSize > remainingSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v04.c:3054) | exact return/error shown | [ ] |
| 2015 | `ZSTD_decompress_usingDict` | `return ERROR(GENERIC); /* not yet supported */` (c_src/src/legacy/zstd_v04.c:3065) | exact return/error shown | [ ] |
| 2016 | `ZSTD_decompress_usingDict` | `if (remainingSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v04.c:3069) | exact return/error shown | [ ] |
| 2017 | `ZSTD_decompress_usingDict` | `return ERROR(GENERIC); /* impossible */` (c_src/src/legacy/zstd_v04.c:3072) | exact return/error shown | [ ] |
| 2018 | `ZSTD_decompressContinue` | `if (srcSize != ctx->expected) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v04.c:3149) | exact return/error shown | [ ] |
| 2019 | `ZSTD_decompressContinue` | `if (srcSize != ZSTD_frameHeaderSize_min) return ERROR(srcSize_wrong); /* impossible */` (c_src/src/legacy/zstd_v04.c:3157) | exact return/error shown | [ ] |
| 2020 | `ZSTD_decompressContinue` | `if (ctx->headerSize > ZSTD_frameHeaderSize_min) return ERROR(GENERIC); /* impossible */` (c_src/src/legacy/zstd_v04.c:3161) | exact return/error shown | [ ] |
| 2021 | `ZSTD_decompressContinue` | `return ERROR(GENERIC); /* not yet handled */` (c_src/src/legacy/zstd_v04.c:3203) | exact return/error shown | [ ] |
| 2022 | `ZSTD_decompressContinue` | `return ERROR(GENERIC);` (c_src/src/legacy/zstd_v04.c:3209) | exact return/error shown | [ ] |
| 2023 | `ZSTD_decompressContinue` | `return ERROR(GENERIC); /* impossible */` (c_src/src/legacy/zstd_v04.c:3218) | exact return/error shown | [ ] |
| 2024 | `ZBUFF_createDCtx` | `if (zbc==NULL) return NULL;` (c_src/src/legacy/zstd_v04.c:3327) | exact return/error shown | [ ] |
| 2025 | `ZBUFF_decompressContinue` | `return ERROR(init_missing);` (c_src/src/legacy/zstd_v04.c:3391) | exact return/error shown | [ ] |
| 2026 | `ZBUFF_decompressContinue` | `if (zbc->inBuff == NULL) return ERROR(memory_allocation);` (c_src/src/legacy/zstd_v04.c:3433) | exact return/error shown | [ ] |
| 2027 | `ZBUFF_decompressContinue` | `if (zbc->outBuff == NULL) return ERROR(memory_allocation);` (c_src/src/legacy/zstd_v04.c:3439) | exact return/error shown | [ ] |
| 2028 | `ZBUFF_decompressContinue` | `if (toLoad > zbc->inBuffSize - zbc->inPos) return ERROR(corruption_detected); /* should never happen */` (c_src/src/legacy/zstd_v04.c:3484) | exact return/error shown | [ ] |
| 2029 | `ZBUFF_decompressContinue` | `default: return ERROR(GENERIC); /* impossible */` (c_src/src/legacy/zstd_v04.c:3519) | exact return/error shown | [ ] |
| 2030 | `ZBUFFv04_isError` | `unsigned ZBUFFv04_isError(size_t errorCode) { return ERR_isError(errorCode); }` (c_src/src/legacy/zstd_v04.c:3538) | exact return/error shown | [ ] |
| 2031 | `ZBUFFv04_getErrorName` | `const char* ZBUFFv04_getErrorName(size_t errorCode) { return ERR_getErrorName(errorCode); }` (c_src/src/legacy/zstd_v04.c:3539) | exact return/error shown | [ ] |
| 2032 | `ZSTDv04_decompress` | `if (dctx==NULL) return ERROR(memory_allocation);` (c_src/src/legacy/zstd_v04.c:3560) | exact return/error shown | [ ] |
| 2033 | `BITv05_initDStream` | `if (srcSize < 1) { memset(bitD, 0, sizeof(*bitD)); return ERROR(srcSize_wrong); }` (c_src/src/legacy/zstd_v05.c:736) | exact return/error shown | [ ] |
| 2034 | `BITv05_initDStream` | `if (contain32 == 0) return ERROR(GENERIC); /* endMark not present */` (c_src/src/legacy/zstd_v05.c:744) | exact return/error shown | [ ] |
| 2035 | `BITv05_initDStream` | `if (contain32 == 0) return ERROR(GENERIC); /* endMark not present */` (c_src/src/legacy/zstd_v05.c:762) | exact return/error shown | [ ] |
| 2036 | `FSEv05_buildDTable` | `if (maxSymbolValue > FSEv05_MAX_SYMBOL_VALUE) return ERROR(maxSymbolValue_tooLarge);` (c_src/src/legacy/zstd_v05.c:1173) | exact return/error shown | [ ] |
| 2037 | `FSEv05_buildDTable` | `if (tableLog > FSEv05_MAX_TABLELOG) return ERROR(tableLog_tooLarge);` (c_src/src/legacy/zstd_v05.c:1174) | exact return/error shown | [ ] |
| 2038 | `FSEv05_buildDTable` | `if (position!=0) return ERROR(GENERIC); /* position must reach all cells once, otherwise normalizedCounter is incorrect */` (c_src/src/legacy/zstd_v05.c:1197) | exact return/error shown | [ ] |
| 2039 | `FSEv05_isError` | `unsigned FSEv05_isError(size_t code) { return ERR_isError(code); }` (c_src/src/legacy/zstd_v05.c:1219) | exact return/error shown | [ ] |
| 2040 | `FSEv05_getErrorName` | `const char* FSEv05_getErrorName(size_t code) { return ERR_getErrorName(code); }` (c_src/src/legacy/zstd_v05.c:1221) | exact return/error shown | [ ] |
| 2041 | `FSEv05_readNCount` | `if (hbSize < 4) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v05.c:1244) | exact return/error shown | [ ] |
| 2042 | `FSEv05_readNCount` | `if (nbBits > FSEv05_TABLELOG_ABSOLUTE_MAX) return ERROR(tableLog_tooLarge);` (c_src/src/legacy/zstd_v05.c:1247) | exact return/error shown | [ ] |
| 2043 | `FSEv05_readNCount` | `if (n0 > *maxSVPtr) return ERROR(maxSymbolValue_tooSmall);` (c_src/src/legacy/zstd_v05.c:1274) | exact return/error shown | [ ] |
| 2044 | `FSEv05_readNCount` | `if (remaining != 1) return ERROR(GENERIC);` (c_src/src/legacy/zstd_v05.c:1315) | exact return/error shown | [ ] |
| 2045 | `FSEv05_readNCount` | `if ((size_t)(ip-istart) > hbSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v05.c:1319) | exact return/error shown | [ ] |
| 2046 | `FSEv05_buildDTable_raw` | `if (nbBits < 1) return ERROR(GENERIC); /* min size */` (c_src/src/legacy/zstd_v05.c:1358) | exact return/error shown | [ ] |
| 2047 | `FSEv05_decompress_usingDTable_generic` | `if (FSEv05_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v05.c:1389) | exact return/error shown | [ ] |
| 2048 | `FSEv05_decompress_usingDTable_generic` | `if (op==omax) return ERROR(dstSize_tooSmall); /* dst buffer is full, but cSrc unfinished */` (c_src/src/legacy/zstd_v05.c:1434) | exact return/error shown | [ ] |
| 2049 | `FSEv05_decompress_usingDTable_generic` | `return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v05.c:1436) | exact return/error shown | [ ] |
| 2050 | `FSEv05_decompress` | `if (cSrcSize<2) return ERROR(srcSize_wrong); /* too small input size */` (c_src/src/legacy/zstd_v05.c:1464) | exact return/error shown | [ ] |
| 2051 | `FSEv05_decompress` | `if (FSEv05_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v05.c:1468) | exact return/error shown | [ ] |
| 2052 | `FSEv05_decompress` | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong); /* too small input size */` (c_src/src/legacy/zstd_v05.c:1469) | exact return/error shown | [ ] |
| 2053 | `FSEv05_decompress` | `if (FSEv05_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v05.c:1474) | exact return/error shown | [ ] |
| 2054 | `HUFv05_isError` | `unsigned HUFv05_isError(size_t code) { return ERR_isError(code); }` (c_src/src/legacy/zstd_v05.c:1723) | exact return/error shown | [ ] |
| 2055 | `HUFv05_getErrorName` | `const char* HUFv05_getErrorName(size_t code) { return ERR_getErrorName(code); }` (c_src/src/legacy/zstd_v05.c:1724) | exact return/error shown | [ ] |
| 2056 | `HUFv05_readStats` | `if (!srcSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v05.c:1753) | exact return/error shown | [ ] |
| 2057 | `HUFv05_readStats` | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v05.c:1767) | exact return/error shown | [ ] |
| 2058 | `HUFv05_readStats` | `if (oSize >= hwSize) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v05.c:1768) | exact return/error shown | [ ] |
| 2059 | `HUFv05_readStats` | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v05.c:1775) | exact return/error shown | [ ] |
| 2060 | `HUFv05_readStats` | `if (huffWeight[n] >= HUFv05_ABSOLUTEMAX_TABLELOG) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v05.c:1784) | exact return/error shown | [ ] |
| 2061 | `HUFv05_readStats` | `if (weightTotal == 0) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v05.c:1788) | exact return/error shown | [ ] |
| 2062 | `HUFv05_readStats` | `if (tableLog > HUFv05_ABSOLUTEMAX_TABLELOG) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v05.c:1792) | exact return/error shown | [ ] |
| 2063 | `HUFv05_readStats` | `if (verif != rest) return ERROR(corruption_detected); /* last value must be a clean power of 2 */` (c_src/src/legacy/zstd_v05.c:1798) | exact return/error shown | [ ] |
| 2064 | `HUFv05_readStats` | `if ((rankStats[1] < 2) \|\| (rankStats[1] & 1)) return ERROR(corruption_detected); /* by construction : at least 2 elts of rank 1, must be even */` (c_src/src/legacy/zstd_v05.c:1804) | exact return/error shown | [ ] |
| 2065 | `HUFv05_readDTableX2` | `if (tableLog > DTable[0]) return ERROR(tableLog_tooLarge); /* DTable is too small */` (c_src/src/legacy/zstd_v05.c:1836) | exact return/error shown | [ ] |
| 2066 | `HUFv05_decompress1X2_usingDTable` | `if (dstSize <= cSrcSize) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v05.c:1916) | exact return/error shown | [ ] |
| 2067 | `HUFv05_decompress1X2_usingDTable` | `if (HUFv05_isError(errorCode)) return errorCode; }` (c_src/src/legacy/zstd_v05.c:1918) | exact return/error shown | [ ] |
| 2068 | `HUFv05_decompress1X2_usingDTable` | `if (!BITv05_endOfDStream(&bitD)) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v05.c:1923) | exact return/error shown | [ ] |
| 2069 | `HUFv05_decompress1X2` | `if (HUFv05_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v05.c:1935) | exact return/error shown | [ ] |
| 2070 | `HUFv05_decompress1X2` | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v05.c:1936) | exact return/error shown | [ ] |
| 2071 | `HUFv05_decompress4X2_usingDTable` | `if (cSrcSize < 10) return ERROR(corruption_detected); /* strict minimum : jump table + 1 byte per stream */` (c_src/src/legacy/zstd_v05.c:1950) | exact return/error shown | [ ] |
| 2072 | `HUFv05_decompress4X2_usingDTable` | `if (length4 > cSrcSize) return ERROR(corruption_detected); /* overflow */` (c_src/src/legacy/zstd_v05.c:1984) | exact return/error shown | [ ] |
| 2073 | `HUFv05_decompress4X2_usingDTable` | `if (HUFv05_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v05.c:1986) | exact return/error shown | [ ] |
| 2074 | `HUFv05_decompress4X2_usingDTable` | `if (HUFv05_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v05.c:1988) | exact return/error shown | [ ] |
| 2075 | `HUFv05_decompress4X2_usingDTable` | `if (HUFv05_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v05.c:1990) | exact return/error shown | [ ] |
| 2076 | `HUFv05_decompress4X2_usingDTable` | `if (HUFv05_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v05.c:1992) | exact return/error shown | [ ] |
| 2077 | `HUFv05_decompress4X2_usingDTable` | `if (op1 > opStart2) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v05.c:2017) | exact return/error shown | [ ] |
| 2078 | `HUFv05_decompress4X2_usingDTable` | `if (op2 > opStart3) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v05.c:2018) | exact return/error shown | [ ] |
| 2079 | `HUFv05_decompress4X2_usingDTable` | `if (op3 > opStart4) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v05.c:2019) | exact return/error shown | [ ] |
| 2080 | `HUFv05_decompress4X2_usingDTable` | `if (!endSignal) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v05.c:2030) | exact return/error shown | [ ] |
| 2081 | `HUFv05_decompress4X2` | `if (HUFv05_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v05.c:2045) | exact return/error shown | [ ] |
| 2082 | `HUFv05_decompress4X2` | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v05.c:2046) | exact return/error shown | [ ] |
| 2083 | `HUFv05_readDTableX4` | `if (memLog > HUFv05_ABSOLUTEMAX_TABLELOG) return ERROR(tableLog_tooLarge);` (c_src/src/legacy/zstd_v05.c:2160) | exact return/error shown | [ ] |
| 2084 | `HUFv05_readDTableX4` | `if (tableLog > memLog) return ERROR(tableLog_tooLarge); /* DTable can't fit code depth */` (c_src/src/legacy/zstd_v05.c:2167) | exact return/error shown | [ ] |
| 2085 | `HUFv05_decompress1X4_usingDTable` | `if (HUFv05_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v05.c:2300) | exact return/error shown | [ ] |
| 2086 | `HUFv05_decompress1X4_usingDTable` | `if (!BITv05_endOfDStream(&bitD)) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v05.c:2306) | exact return/error shown | [ ] |
| 2087 | `HUFv05_decompress1X4` | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v05.c:2319) | exact return/error shown | [ ] |
| 2088 | `HUFv05_decompress4X4_usingDTable` | `if (cSrcSize < 10) return ERROR(corruption_detected); /* strict minimum : jump table + 1 byte per stream */` (c_src/src/legacy/zstd_v05.c:2331) | exact return/error shown | [ ] |
| 2089 | `HUFv05_decompress4X4_usingDTable` | `if (length4 > cSrcSize) return ERROR(corruption_detected); /* overflow */` (c_src/src/legacy/zstd_v05.c:2366) | exact return/error shown | [ ] |
| 2090 | `HUFv05_decompress4X4_usingDTable` | `if (HUFv05_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v05.c:2368) | exact return/error shown | [ ] |
| 2091 | `HUFv05_decompress4X4_usingDTable` | `if (HUFv05_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v05.c:2370) | exact return/error shown | [ ] |
| 2092 | `HUFv05_decompress4X4_usingDTable` | `if (HUFv05_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v05.c:2372) | exact return/error shown | [ ] |
| 2093 | `HUFv05_decompress4X4_usingDTable` | `if (HUFv05_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v05.c:2374) | exact return/error shown | [ ] |
| 2094 | `HUFv05_decompress4X4_usingDTable` | `if (op1 > opStart2) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v05.c:2400) | exact return/error shown | [ ] |
| 2095 | `HUFv05_decompress4X4_usingDTable` | `if (op2 > opStart3) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v05.c:2401) | exact return/error shown | [ ] |
| 2096 | `HUFv05_decompress4X4_usingDTable` | `if (op3 > opStart4) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v05.c:2402) | exact return/error shown | [ ] |
| 2097 | `HUFv05_decompress4X4_usingDTable` | `if (!endSignal) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v05.c:2413) | exact return/error shown | [ ] |
| 2098 | `HUFv05_decompress4X4` | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v05.c:2428) | exact return/error shown | [ ] |
| 2099 | `HUFv05_decompress` | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v05.c:2475) | exact return/error shown | [ ] |
| 2100 | `HUFv05_decompress` | `if (cSrcSize >= dstSize) return ERROR(corruption_detected); /* invalid, or not compressed, but not compressed already dealt with */` (c_src/src/legacy/zstd_v05.c:2476) | exact return/error shown | [ ] |
| 2101 | `ZSTDv05_isError` | `unsigned ZSTDv05_isError(size_t code) { return ERR_isError(code); }` (c_src/src/legacy/zstd_v05.c:2577) | exact return/error shown | [ ] |
| 2102 | `ZSTDv05_getErrorName` | `const char* ZSTDv05_getErrorName(size_t code) { return ERR_getErrorName(code); }` (c_src/src/legacy/zstd_v05.c:2582) | exact return/error shown | [ ] |
| 2103 | `ZSTDv05_createDCtx` | `if (dctx==NULL) return NULL;` (c_src/src/legacy/zstd_v05.c:2632) | exact return/error shown | [ ] |
| 2104 | `ZSTDv05_decodeFrameHeader_Part1` | `return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v05.c:2743) | exact return/error shown | [ ] |
| 2105 | `ZSTDv05_decodeFrameHeader_Part1` | `if (magicNumber != ZSTDv05_MAGICNUMBER) return ERROR(prefix_unknown);` (c_src/src/legacy/zstd_v05.c:2745) | exact return/error shown | [ ] |
| 2106 | `ZSTDv05_getFrameParams` | `if (magicNumber != ZSTDv05_MAGICNUMBER) return ERROR(prefix_unknown);` (c_src/src/legacy/zstd_v05.c:2756) | exact return/error shown | [ ] |
| 2107 | `ZSTDv05_getFrameParams` | `if ((((const BYTE*)src)[4] >> 4) != 0) return ERROR(frameParameter_unsupported); /* reserved bits */` (c_src/src/legacy/zstd_v05.c:2759) | exact return/error shown | [ ] |
| 2108 | `ZSTDv05_decodeFrameHeader_Part2` | `return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v05.c:2771) | exact return/error shown | [ ] |
| 2109 | `ZSTDv05_decodeFrameHeader_Part2` | `if ((MEM_32bits()) && (zc->params.windowLog > 25)) return ERROR(frameParameter_unsupported);` (c_src/src/legacy/zstd_v05.c:2773) | exact return/error shown | [ ] |
| 2110 | `ZSTDv05_getcBlockSize` | `return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v05.c:2785) | exact return/error shown | [ ] |
| 2111 | `ZSTDv05_copyRawBlock` | `if (dst==NULL) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v05.c:2801) | exact return/error shown | [ ] |
| 2112 | `ZSTDv05_copyRawBlock` | `if (srcSize > maxDstSize) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v05.c:2802) | exact return/error shown | [ ] |
| 2113 | `ZSTDv05_decodeLiteralsBlock` | `if (srcSize < MIN_CBLOCK_SIZE) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v05.c:2816) | exact return/error shown | [ ] |
| 2114 | `ZSTDv05_decodeLiteralsBlock` | `if (srcSize < 5) return ERROR(corruption_detected); /* srcSize >= MIN_CBLOCK_SIZE == 3; here we need up to 5 for case 3 */` (c_src/src/legacy/zstd_v05.c:2824) | exact return/error shown | [ ] |
| 2115 | `ZSTDv05_decodeLiteralsBlock` | `if (litSize > BLOCKSIZE) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v05.c:2847) | exact return/error shown | [ ] |
| 2116 | `ZSTDv05_decodeLiteralsBlock` | `if (litCSize + lhSize > srcSize) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v05.c:2848) | exact return/error shown | [ ] |
| 2117 | `ZSTDv05_decodeLiteralsBlock` | `return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v05.c:2853) | exact return/error shown | [ ] |
| 2118 | `ZSTDv05_decodeLiteralsBlock` | `return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v05.c:2866) | exact return/error shown | [ ] |
| 2119 | `ZSTDv05_decodeLiteralsBlock` | `return ERROR(dictionary_corrupted);` (c_src/src/legacy/zstd_v05.c:2868) | exact return/error shown | [ ] |
| 2120 | `ZSTDv05_decodeLiteralsBlock` | `if (litCSize + lhSize > srcSize) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v05.c:2874) | exact return/error shown | [ ] |
| 2121 | `ZSTDv05_decodeLiteralsBlock` | `if (HUFv05_isError(errorCode)) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v05.c:2877) | exact return/error shown | [ ] |
| 2122 | `ZSTDv05_decodeLiteralsBlock` | `if (litSize+lhSize > srcSize) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v05.c:2903) | exact return/error shown | [ ] |
| 2123 | `ZSTDv05_decodeLiteralsBlock` | `if (srcSize<4) return ERROR(corruption_detected); /* srcSize >= MIN_CBLOCK_SIZE == 3; here we need lhSize+1 = 4 */` (c_src/src/legacy/zstd_v05.c:2930) | exact return/error shown | [ ] |
| 2124 | `ZSTDv05_decodeLiteralsBlock` | `if (litSize > BLOCKSIZE) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v05.c:2933) | exact return/error shown | [ ] |
| 2125 | `ZSTDv05_decodeLiteralsBlock` | `return ERROR(corruption_detected); /* impossible */` (c_src/src/legacy/zstd_v05.c:2940) | exact return/error shown | [ ] |
| 2126 | `ZSTDv05_decodeSeqHeaders` | `return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v05.c:2958) | exact return/error shown | [ ] |
| 2127 | `ZSTDv05_decodeSeqHeaders` | `if (ip >= iend) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v05.c:2964) | exact return/error shown | [ ] |
| 2128 | `ZSTDv05_decodeSeqHeaders` | `if (ip >= iend) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v05.c:2968) | exact return/error shown | [ ] |
| 2129 | `ZSTDv05_decodeSeqHeaders` | `if (ip+3 > iend) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v05.c:2973) | exact return/error shown | [ ] |
| 2130 | `ZSTDv05_decodeSeqHeaders` | `if (ip+2 > iend) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v05.c:2978) | exact return/error shown | [ ] |
| 2131 | `ZSTDv05_decodeSeqHeaders` | `if (ip > iend-3) return ERROR(srcSize_wrong); /* min : all 3 are "raw", hence no header, but at least xxLog bits per type */` (c_src/src/legacy/zstd_v05.c:2988) | exact return/error shown | [ ] |
| 2132 | `ZSTDv05_decodeSeqHeaders` | `if (!flagStaticTable) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v05.c:3007) | exact return/error shown | [ ] |
| 2133 | `ZSTDv05_decodeSeqHeaders` | `if (FSEv05_isError(headerSize)) return ERROR(GENERIC);` (c_src/src/legacy/zstd_v05.c:3013) | exact return/error shown | [ ] |
| 2134 | `ZSTDv05_decodeSeqHeaders` | `if (LLlog > LLFSEv05Log) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v05.c:3014) | exact return/error shown | [ ] |
| 2135 | `ZSTDv05_decodeSeqHeaders` | `if (ip > iend-2) return ERROR(srcSize_wrong); /* min : "raw", hence no header, but at least xxLog bits */` (c_src/src/legacy/zstd_v05.c:3023) | exact return/error shown | [ ] |
| 2136 | `ZSTDv05_decodeSeqHeaders` | `if (!flagStaticTable) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v05.c:3031) | exact return/error shown | [ ] |
| 2137 | `ZSTDv05_decodeSeqHeaders` | `if (FSEv05_isError(headerSize)) return ERROR(GENERIC);` (c_src/src/legacy/zstd_v05.c:3037) | exact return/error shown | [ ] |
| 2138 | `ZSTDv05_decodeSeqHeaders` | `if (Offlog > OffFSEv05Log) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v05.c:3038) | exact return/error shown | [ ] |
| 2139 | `ZSTDv05_decodeSeqHeaders` | `if (ip > iend-2) return ERROR(srcSize_wrong); /* min : "raw", hence no header, but at least xxLog bits */` (c_src/src/legacy/zstd_v05.c:3047) | exact return/error shown | [ ] |
| 2140 | `ZSTDv05_decodeSeqHeaders` | `if (!flagStaticTable) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v05.c:3055) | exact return/error shown | [ ] |
| 2141 | `ZSTDv05_decodeSeqHeaders` | `if (FSEv05_isError(headerSize)) return ERROR(GENERIC);` (c_src/src/legacy/zstd_v05.c:3061) | exact return/error shown | [ ] |
| 2142 | `ZSTDv05_decodeSeqHeaders` | `if (MLlog > MLFSEv05Log) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v05.c:3062) | exact return/error shown | [ ] |
| 2143 | `ZSTDv05_execSequence` | `if (seqLength > (size_t)(oend - op)) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v05.c:3188) | exact return/error shown | [ ] |
| 2144 | `ZSTDv05_execSequence` | `if (sequence.litLength > (size_t)(litLimit - *litPtr)) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v05.c:3189) | exact return/error shown | [ ] |
| 2145 | `ZSTDv05_execSequence` | `if (oLitEnd > oend_8) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v05.c:3191) | exact return/error shown | [ ] |
| 2146 | `ZSTDv05_execSequence` | `if (oMatchEnd > oend) return ERROR(dstSize_tooSmall); /* overwrite beyond dst buffer */` (c_src/src/legacy/zstd_v05.c:3193) | exact return/error shown | [ ] |
| 2147 | `ZSTDv05_execSequence` | `if (litEnd > litLimit) return ERROR(corruption_detected); /* overRead beyond lit buffer */` (c_src/src/legacy/zstd_v05.c:3194) | exact return/error shown | [ ] |
| 2148 | `ZSTDv05_execSequence` | `return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v05.c:3205) | exact return/error shown | [ ] |
| 2149 | `ZSTDv05_decompressSequences` | `if (ZSTDv05_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v05.c:3282) | exact return/error shown | [ ] |
| 2150 | `ZSTDv05_decompressSequences` | `if (ERR_isError(errorCode)) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v05.c:3296) | exact return/error shown | [ ] |
| 2151 | `ZSTDv05_decompressSequences` | `if (nbSeq) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v05.c:3311) | exact return/error shown | [ ] |
| 2152 | `ZSTDv05_decompressSequences` | `if (litPtr > litEnd) return ERROR(corruption_detected); /* too many literals already used */` (c_src/src/legacy/zstd_v05.c:3317) | exact return/error shown | [ ] |
| 2153 | `ZSTDv05_decompressSequences` | `if (op+lastLLSize > oend) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v05.c:3318) | exact return/error shown | [ ] |
| 2154 | `ZSTDv05_decompressBlock_internal` | `if (srcSize >= BLOCKSIZE) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v05.c:3347) | exact return/error shown | [ ] |
| 2155 | `ZSTDv05_decompress_continueDCtx` | `if (srcSize < ZSTDv05_frameHeaderSize_min+ZSTDv05_blockHeaderSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v05.c:3385) | exact return/error shown | [ ] |
| 2156 | `ZSTDv05_decompress_continueDCtx` | `if (srcSize < frameHeaderSize+ZSTDv05_blockHeaderSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v05.c:3388) | exact return/error shown | [ ] |
| 2157 | `ZSTDv05_decompress_continueDCtx` | `if (cBlockSize > remainingSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v05.c:3403) | exact return/error shown | [ ] |
| 2158 | `ZSTDv05_decompress_continueDCtx` | `return ERROR(GENERIC); /* not yet supported */` (c_src/src/legacy/zstd_v05.c:3414) | exact return/error shown | [ ] |
| 2159 | `ZSTDv05_decompress_continueDCtx` | `if (remainingSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v05.c:3418) | exact return/error shown | [ ] |
| 2160 | `ZSTDv05_decompress_continueDCtx` | `return ERROR(GENERIC); /* impossible */` (c_src/src/legacy/zstd_v05.c:3421) | exact return/error shown | [ ] |
| 2161 | `ZSTDv05_decompress` | `if (dctx==NULL) return ERROR(memory_allocation);` (c_src/src/legacy/zstd_v05.c:3466) | exact return/error shown | [ ] |
| 2162 | `ZSTDv05_decompressContinue` | `if (srcSize != dctx->expected) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v05.c:3540) | exact return/error shown | [ ] |
| 2163 | `ZSTDv05_decompressContinue` | `if (srcSize != ZSTDv05_frameHeaderSize_min) return ERROR(srcSize_wrong); /* impossible */` (c_src/src/legacy/zstd_v05.c:3548) | exact return/error shown | [ ] |
| 2164 | `ZSTDv05_decompressContinue` | `if (dctx->headerSize > ZSTDv05_frameHeaderSize_min) return ERROR(GENERIC); /* should never happen */` (c_src/src/legacy/zstd_v05.c:3552) | exact return/error shown | [ ] |
| 2165 | `ZSTDv05_decompressContinue` | `return ERROR(GENERIC); /* not yet handled */` (c_src/src/legacy/zstd_v05.c:3593) | exact return/error shown | [ ] |
| 2166 | `ZSTDv05_decompressContinue` | `return ERROR(GENERIC); /* impossible */` (c_src/src/legacy/zstd_v05.c:3599) | exact return/error shown | [ ] |
| 2167 | `ZSTDv05_decompressContinue` | `return ERROR(GENERIC); /* impossible */` (c_src/src/legacy/zstd_v05.c:3608) | exact return/error shown | [ ] |
| 2168 | `ZSTDv05_loadEntropy` | `if (HUFv05_isError(hSize)) return ERROR(dictionary_corrupted);` (c_src/src/legacy/zstd_v05.c:3632) | exact return/error shown | [ ] |
| 2169 | `ZSTDv05_loadEntropy` | `if (FSEv05_isError(offcodeHeaderSize)) return ERROR(dictionary_corrupted);` (c_src/src/legacy/zstd_v05.c:3637) | exact return/error shown | [ ] |
| 2170 | `ZSTDv05_loadEntropy` | `if (offcodeLog > OffFSEv05Log) return ERROR(dictionary_corrupted);` (c_src/src/legacy/zstd_v05.c:3638) | exact return/error shown | [ ] |
| 2171 | `ZSTDv05_loadEntropy` | `if (FSEv05_isError(errorCode)) return ERROR(dictionary_corrupted);` (c_src/src/legacy/zstd_v05.c:3640) | exact return/error shown | [ ] |
| 2172 | `ZSTDv05_loadEntropy` | `if (FSEv05_isError(matchlengthHeaderSize)) return ERROR(dictionary_corrupted);` (c_src/src/legacy/zstd_v05.c:3645) | exact return/error shown | [ ] |
| 2173 | `ZSTDv05_loadEntropy` | `if (matchlengthLog > MLFSEv05Log) return ERROR(dictionary_corrupted);` (c_src/src/legacy/zstd_v05.c:3646) | exact return/error shown | [ ] |
| 2174 | `ZSTDv05_loadEntropy` | `if (FSEv05_isError(errorCode)) return ERROR(dictionary_corrupted);` (c_src/src/legacy/zstd_v05.c:3648) | exact return/error shown | [ ] |
| 2175 | `ZSTDv05_loadEntropy` | `if (litlengthLog > LLFSEv05Log) return ERROR(dictionary_corrupted);` (c_src/src/legacy/zstd_v05.c:3653) | exact return/error shown | [ ] |
| 2176 | `ZSTDv05_loadEntropy` | `if (FSEv05_isError(litlengthHeaderSize)) return ERROR(dictionary_corrupted);` (c_src/src/legacy/zstd_v05.c:3654) | exact return/error shown | [ ] |
| 2177 | `ZSTDv05_loadEntropy` | `if (FSEv05_isError(errorCode)) return ERROR(dictionary_corrupted);` (c_src/src/legacy/zstd_v05.c:3656) | exact return/error shown | [ ] |
| 2178 | `ZSTDv05_decompress_insertDictionary` | `if (ZSTDv05_isError(eSize)) return ERROR(dictionary_corrupted);` (c_src/src/legacy/zstd_v05.c:3675) | exact return/error shown | [ ] |
| 2179 | `ZSTDv05_decompressBegin_usingDict` | `if (ZSTDv05_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v05.c:3690) | exact return/error shown | [ ] |
| 2180 | `ZSTDv05_decompressBegin_usingDict` | `if (ZSTDv05_isError(errorCode)) return ERROR(dictionary_corrupted);` (c_src/src/legacy/zstd_v05.c:3694) | exact return/error shown | [ ] |
| 2181 | `ZBUFFv05_createDCtx` | `if (zbc==NULL) return NULL;` (c_src/src/legacy/zstd_v05.c:3807) | exact return/error shown | [ ] |
| 2182 | `ZBUFFv05_decompressContinue` | `return ERROR(init_missing);` (c_src/src/legacy/zstd_v05.c:3856) | exact return/error shown | [ ] |
| 2183 | `ZBUFFv05_decompressContinue` | `if (zbc->inBuff == NULL) return ERROR(memory_allocation);` (c_src/src/legacy/zstd_v05.c:3902) | exact return/error shown | [ ] |
| 2184 | `ZBUFFv05_decompressContinue` | `if (zbc->outBuff == NULL) return ERROR(memory_allocation);` (c_src/src/legacy/zstd_v05.c:3908) | exact return/error shown | [ ] |
| 2185 | `ZBUFFv05_decompressContinue` | `if (toLoad > zbc->inBuffSize - zbc->inPos) return ERROR(corruption_detected); /* should never happen */` (c_src/src/legacy/zstd_v05.c:3949) | exact return/error shown | [ ] |
| 2186 | `ZBUFFv05_decompressContinue` | `default: return ERROR(GENERIC); /* impossible */` (c_src/src/legacy/zstd_v05.c:3983) | exact return/error shown | [ ] |
| 2187 | `ZBUFFv05_isError` | `unsigned ZBUFFv05_isError(size_t errorCode) { return ERR_isError(errorCode); }` (c_src/src/legacy/zstd_v05.c:4001) | exact return/error shown | [ ] |
| 2188 | `ZBUFFv05_getErrorName` | `const char* ZBUFFv05_getErrorName(size_t errorCode) { return ERR_getErrorName(errorCode); }` (c_src/src/legacy/zstd_v05.c:4002) | exact return/error shown | [ ] |
| 2189 | `BITv06_initDStream` | `if (srcSize < 1) { memset(bitD, 0, sizeof(*bitD)); return ERROR(srcSize_wrong); }` (c_src/src/legacy/zstd_v06.c:835) | exact return/error shown | [ ] |
| 2190 | `BITv06_initDStream` | `if (lastByte == 0) return ERROR(GENERIC); /* endMark not present */` (c_src/src/legacy/zstd_v06.c:842) | exact return/error shown | [ ] |
| 2191 | `BITv06_initDStream` | `if (lastByte == 0) return ERROR(GENERIC); /* endMark not present */` (c_src/src/legacy/zstd_v06.c:859) | exact return/error shown | [ ] |
| 2192 | `FSEv06_isError` | `unsigned FSEv06_isError(size_t code) { return ERR_isError(code); }` (c_src/src/legacy/zstd_v06.c:1191) | exact return/error shown | [ ] |
| 2193 | `FSEv06_getErrorName` | `const char* FSEv06_getErrorName(size_t code) { return ERR_getErrorName(code); }` (c_src/src/legacy/zstd_v06.c:1193) | exact return/error shown | [ ] |
| 2194 | `HUFv06_isError` | `static unsigned HUFv06_isError(size_t code) { return ERR_isError(code); }` (c_src/src/legacy/zstd_v06.c:1199) | exact return/error shown | [ ] |
| 2195 | `FSEv06_readNCount` | `if (hbSize < 4) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v06.c:1221) | exact return/error shown | [ ] |
| 2196 | `FSEv06_readNCount` | `if (nbBits > FSEv06_TABLELOG_ABSOLUTE_MAX) return ERROR(tableLog_tooLarge);` (c_src/src/legacy/zstd_v06.c:1224) | exact return/error shown | [ ] |
| 2197 | `FSEv06_readNCount` | `if (n0 > *maxSVPtr) return ERROR(maxSymbolValue_tooSmall);` (c_src/src/legacy/zstd_v06.c:1251) | exact return/error shown | [ ] |
| 2198 | `FSEv06_readNCount` | `if (remaining != 1) return ERROR(GENERIC);` (c_src/src/legacy/zstd_v06.c:1291) | exact return/error shown | [ ] |
| 2199 | `FSEv06_readNCount` | `if ((size_t)(ip-istart) > hbSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v06.c:1295) | exact return/error shown | [ ] |
| 2200 | `FSEv06_buildDTable` | `if (maxSymbolValue > FSEv06_MAX_SYMBOL_VALUE) return ERROR(maxSymbolValue_tooLarge);` (c_src/src/legacy/zstd_v06.c:1413) | exact return/error shown | [ ] |
| 2201 | `FSEv06_buildDTable` | `if (tableLog > FSEv06_MAX_TABLELOG) return ERROR(tableLog_tooLarge);` (c_src/src/legacy/zstd_v06.c:1414) | exact return/error shown | [ ] |
| 2202 | `FSEv06_buildDTable` | `if (position!=0) return ERROR(GENERIC); /* position must reach all cells once, otherwise normalizedCounter is incorrect */` (c_src/src/legacy/zstd_v06.c:1445) | exact return/error shown | [ ] |
| 2203 | `FSEv06_buildDTable_raw` | `if (nbBits < 1) return ERROR(GENERIC); /* min size */` (c_src/src/legacy/zstd_v06.c:1497) | exact return/error shown | [ ] |
| 2204 | `FSEv06_decompress_usingDTable_generic` | `if (FSEv06_isError(errorCode)) return errorCode; }` (c_src/src/legacy/zstd_v06.c:1527) | exact return/error shown | [ ] |
| 2205 | `FSEv06_decompress_usingDTable_generic` | `if (op>(omax-2)) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v06.c:1557) | exact return/error shown | [ ] |
| 2206 | `FSEv06_decompress_usingDTable_generic` | `if (op>(omax-2)) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v06.c:1566) | exact return/error shown | [ ] |
| 2207 | `FSEv06_decompress` | `if (cSrcSize<2) return ERROR(srcSize_wrong); /* too small input size */` (c_src/src/legacy/zstd_v06.c:1602) | exact return/error shown | [ ] |
| 2208 | `FSEv06_decompress` | `if (NCountLength >= cSrcSize) return ERROR(srcSize_wrong); /* too small input size */` (c_src/src/legacy/zstd_v06.c:1607) | exact return/error shown | [ ] |
| 2209 | `FSEv06_decompress` | `if (FSEv06_isError(errorCode)) return errorCode; }` (c_src/src/legacy/zstd_v06.c:1613) | exact return/error shown | [ ] |
| 2210 | `HUFv06_readStats` | `if (!srcSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v06.c:1807) | exact return/error shown | [ ] |
| 2211 | `HUFv06_readStats` | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v06.c:1821) | exact return/error shown | [ ] |
| 2212 | `HUFv06_readStats` | `if (oSize >= hwSize) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v06.c:1822) | exact return/error shown | [ ] |
| 2213 | `HUFv06_readStats` | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v06.c:1830) | exact return/error shown | [ ] |
| 2214 | `HUFv06_readStats` | `if (huffWeight[n] >= HUFv06_ABSOLUTEMAX_TABLELOG) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v06.c:1839) | exact return/error shown | [ ] |
| 2215 | `HUFv06_readStats` | `if (weightTotal == 0) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v06.c:1843) | exact return/error shown | [ ] |
| 2216 | `HUFv06_readStats` | `if (tableLog > HUFv06_ABSOLUTEMAX_TABLELOG) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v06.c:1847) | exact return/error shown | [ ] |
| 2217 | `HUFv06_readStats` | `if (verif != rest) return ERROR(corruption_detected); /* last value must be a clean power of 2 */` (c_src/src/legacy/zstd_v06.c:1854) | exact return/error shown | [ ] |
| 2218 | `HUFv06_readStats` | `if ((rankStats[1] < 2) \|\| (rankStats[1] & 1)) return ERROR(corruption_detected); /* by construction : at least 2 elts of rank 1, must be even */` (c_src/src/legacy/zstd_v06.c:1860) | exact return/error shown | [ ] |
| 2219 | `HUFv06_readDTableX2` | `if (tableLog > DTable[0]) return ERROR(tableLog_tooLarge); /* DTable is too small */` (c_src/src/legacy/zstd_v06.c:1967) | exact return/error shown | [ ] |
| 2220 | `HUFv06_decompress1X2_usingDTable` | `if (HUFv06_isError(errorCode)) return errorCode; }` (c_src/src/legacy/zstd_v06.c:2049) | exact return/error shown | [ ] |
| 2221 | `HUFv06_decompress1X2_usingDTable` | `if (!BITv06_endOfDStream(&bitD)) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v06.c:2054) | exact return/error shown | [ ] |
| 2222 | `HUFv06_decompress1X2` | `if (HUFv06_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v06.c:2065) | exact return/error shown | [ ] |
| 2223 | `HUFv06_decompress1X2` | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v06.c:2066) | exact return/error shown | [ ] |
| 2224 | `HUFv06_decompress4X2_usingDTable` | `if (cSrcSize < 10) return ERROR(corruption_detected); /* strict minimum : jump table + 1 byte per stream */` (c_src/src/legacy/zstd_v06.c:2080) | exact return/error shown | [ ] |
| 2225 | `HUFv06_decompress4X2_usingDTable` | `if (length4 > cSrcSize) return ERROR(corruption_detected); /* overflow */` (c_src/src/legacy/zstd_v06.c:2114) | exact return/error shown | [ ] |
| 2226 | `HUFv06_decompress4X2_usingDTable` | `if (HUFv06_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v06.c:2116) | exact return/error shown | [ ] |
| 2227 | `HUFv06_decompress4X2_usingDTable` | `if (HUFv06_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v06.c:2118) | exact return/error shown | [ ] |
| 2228 | `HUFv06_decompress4X2_usingDTable` | `if (HUFv06_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v06.c:2120) | exact return/error shown | [ ] |
| 2229 | `HUFv06_decompress4X2_usingDTable` | `if (HUFv06_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v06.c:2122) | exact return/error shown | [ ] |
| 2230 | `HUFv06_decompress4X2_usingDTable` | `if (op1 > opStart2) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v06.c:2147) | exact return/error shown | [ ] |
| 2231 | `HUFv06_decompress4X2_usingDTable` | `if (op2 > opStart3) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v06.c:2148) | exact return/error shown | [ ] |
| 2232 | `HUFv06_decompress4X2_usingDTable` | `if (op3 > opStart4) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v06.c:2149) | exact return/error shown | [ ] |
| 2233 | `HUFv06_decompress4X2_usingDTable` | `if (!endSignal) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v06.c:2160) | exact return/error shown | [ ] |
| 2234 | `HUFv06_decompress4X2` | `if (HUFv06_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v06.c:2174) | exact return/error shown | [ ] |
| 2235 | `HUFv06_decompress4X2` | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v06.c:2175) | exact return/error shown | [ ] |
| 2236 | `HUFv06_readDTableX4` | `if (memLog > HUFv06_ABSOLUTEMAX_TABLELOG) return ERROR(tableLog_tooLarge);` (c_src/src/legacy/zstd_v06.c:2286) | exact return/error shown | [ ] |
| 2237 | `HUFv06_readDTableX4` | `if (tableLog > memLog) return ERROR(tableLog_tooLarge); /* DTable can't fit code depth */` (c_src/src/legacy/zstd_v06.c:2293) | exact return/error shown | [ ] |
| 2238 | `HUFv06_decompress1X4_usingDTable` | `if (HUFv06_isError(errorCode)) return errorCode; }` (c_src/src/legacy/zstd_v06.c:2424) | exact return/error shown | [ ] |
| 2239 | `HUFv06_decompress1X4_usingDTable` | `if (!BITv06_endOfDStream(&bitD)) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v06.c:2430) | exact return/error shown | [ ] |
| 2240 | `HUFv06_decompress1X4` | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v06.c:2443) | exact return/error shown | [ ] |
| 2241 | `HUFv06_decompress4X4_usingDTable` | `if (cSrcSize < 10) return ERROR(corruption_detected); /* strict minimum : jump table + 1 byte per stream */` (c_src/src/legacy/zstd_v06.c:2455) | exact return/error shown | [ ] |
| 2242 | `HUFv06_decompress4X4_usingDTable` | `if (length4 > cSrcSize) return ERROR(corruption_detected); /* overflow */` (c_src/src/legacy/zstd_v06.c:2489) | exact return/error shown | [ ] |
| 2243 | `HUFv06_decompress4X4_usingDTable` | `if (HUFv06_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v06.c:2491) | exact return/error shown | [ ] |
| 2244 | `HUFv06_decompress4X4_usingDTable` | `if (HUFv06_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v06.c:2493) | exact return/error shown | [ ] |
| 2245 | `HUFv06_decompress4X4_usingDTable` | `if (HUFv06_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v06.c:2495) | exact return/error shown | [ ] |
| 2246 | `HUFv06_decompress4X4_usingDTable` | `if (HUFv06_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v06.c:2497) | exact return/error shown | [ ] |
| 2247 | `HUFv06_decompress4X4_usingDTable` | `if (op1 > opStart2) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v06.c:2523) | exact return/error shown | [ ] |
| 2248 | `HUFv06_decompress4X4_usingDTable` | `if (op2 > opStart3) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v06.c:2524) | exact return/error shown | [ ] |
| 2249 | `HUFv06_decompress4X4_usingDTable` | `if (op3 > opStart4) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v06.c:2525) | exact return/error shown | [ ] |
| 2250 | `HUFv06_decompress4X4_usingDTable` | `if (!endSignal) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v06.c:2536) | exact return/error shown | [ ] |
| 2251 | `HUFv06_decompress4X4` | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v06.c:2551) | exact return/error shown | [ ] |
| 2252 | `HUFv06_decompress` | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v06.c:2595) | exact return/error shown | [ ] |
| 2253 | `HUFv06_decompress` | `if (cSrcSize > dstSize) return ERROR(corruption_detected); /* invalid */` (c_src/src/legacy/zstd_v06.c:2596) | exact return/error shown | [ ] |
| 2254 | `ZSTDv06_isError` | `unsigned ZSTDv06_isError(size_t code) { return ERR_isError(code); }` (c_src/src/legacy/zstd_v06.c:2660) | exact return/error shown | [ ] |
| 2255 | `ZSTDv06_getErrorName` | `const char* ZSTDv06_getErrorName(size_t code) { return ERR_getErrorName(code); }` (c_src/src/legacy/zstd_v06.c:2664) | exact return/error shown | [ ] |
| 2256 | `ZBUFFv06_isError` | `unsigned ZBUFFv06_isError(size_t errorCode) { return ERR_isError(errorCode); }` (c_src/src/legacy/zstd_v06.c:2670) | exact return/error shown | [ ] |
| 2257 | `ZBUFFv06_getErrorName` | `const char* ZBUFFv06_getErrorName(size_t errorCode) { return ERR_getErrorName(errorCode); }` (c_src/src/legacy/zstd_v06.c:2672) | exact return/error shown | [ ] |
| 2258 | `ZSTDv06_createDCtx` | `if (dctx==NULL) return NULL;` (c_src/src/legacy/zstd_v06.c:2789) | exact return/error shown | [ ] |
| 2259 | `ZSTDv06_frameHeaderSize` | `if (srcSize < ZSTDv06_frameHeaderSize_min) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v06.c:2913) | exact return/error shown | [ ] |
| 2260 | `ZSTDv06_getFrameParams` | `if (MEM_readLE32(src) != ZSTDv06_MAGICNUMBER) return ERROR(prefix_unknown);` (c_src/src/legacy/zstd_v06.c:2929) | exact return/error shown | [ ] |
| 2261 | `ZSTDv06_getFrameParams` | `if ((frameDesc & 0x20) != 0) return ERROR(frameParameter_unsupported); /* reserved 1 bit */` (c_src/src/legacy/zstd_v06.c:2938) | exact return/error shown | [ ] |
| 2262 | `ZSTDv06_decodeFrameHeader` | `if ((MEM_32bits()) && (zc->fParams.windowLog > 25)) return ERROR(frameParameter_unsupported);` (c_src/src/legacy/zstd_v06.c:2957) | exact return/error shown | [ ] |
| 2263 | `ZSTDv06_getcBlockSize` | `if (srcSize < ZSTDv06_blockHeaderSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v06.c:2975) | exact return/error shown | [ ] |
| 2264 | `ZSTDv06_copyRawBlock` | `if (dst==NULL) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v06.c:2989) | exact return/error shown | [ ] |
| 2265 | `ZSTDv06_copyRawBlock` | `if (srcSize > dstCapacity) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v06.c:2990) | exact return/error shown | [ ] |
| 2266 | `ZSTDv06_decodeLiteralsBlock` | `if (srcSize < MIN_CBLOCK_SIZE) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v06.c:3004) | exact return/error shown | [ ] |
| 2267 | `ZSTDv06_decodeLiteralsBlock` | `if (srcSize < 5) return ERROR(corruption_detected); /* srcSize >= MIN_CBLOCK_SIZE == 3; here we need up to 5 for lhSize, + cSize (+nbSeq) */` (c_src/src/legacy/zstd_v06.c:3011) | exact return/error shown | [ ] |
| 2268 | `ZSTDv06_decodeLiteralsBlock` | `if (litSize > ZSTDv06_BLOCKSIZE_MAX) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v06.c:3034) | exact return/error shown | [ ] |
| 2269 | `ZSTDv06_decodeLiteralsBlock` | `if (litCSize + lhSize > srcSize) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v06.c:3035) | exact return/error shown | [ ] |
| 2270 | `ZSTDv06_decodeLiteralsBlock` | `return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v06.c:3040) | exact return/error shown | [ ] |
| 2271 | `ZSTDv06_decodeLiteralsBlock` | `return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v06.c:3051) | exact return/error shown | [ ] |
| 2272 | `ZSTDv06_decodeLiteralsBlock` | `return ERROR(dictionary_corrupted);` (c_src/src/legacy/zstd_v06.c:3053) | exact return/error shown | [ ] |
| 2273 | `ZSTDv06_decodeLiteralsBlock` | `if (litCSize + lhSize > srcSize) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v06.c:3059) | exact return/error shown | [ ] |
| 2274 | `ZSTDv06_decodeLiteralsBlock` | `if (HUFv06_isError(errorCode)) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v06.c:3062) | exact return/error shown | [ ] |
| 2275 | `ZSTDv06_decodeLiteralsBlock` | `if (litSize+lhSize > srcSize) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v06.c:3087) | exact return/error shown | [ ] |
| 2276 | `ZSTDv06_decodeLiteralsBlock` | `if (srcSize<4) return ERROR(corruption_detected); /* srcSize >= MIN_CBLOCK_SIZE == 3; here we need lhSize+1 = 4 */` (c_src/src/legacy/zstd_v06.c:3113) | exact return/error shown | [ ] |
| 2277 | `ZSTDv06_decodeLiteralsBlock` | `if (litSize > ZSTDv06_BLOCKSIZE_MAX) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v06.c:3116) | exact return/error shown | [ ] |
| 2278 | `ZSTDv06_decodeLiteralsBlock` | `return ERROR(corruption_detected); /* impossible */` (c_src/src/legacy/zstd_v06.c:3123) | exact return/error shown | [ ] |
| 2279 | `ZSTDv06_buildSeqTable` | `if (!srcSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v06.c:3139) | exact return/error shown | [ ] |
| 2280 | `ZSTDv06_buildSeqTable` | `if ( (*(const BYTE*)src) > max) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v06.c:3140) | exact return/error shown | [ ] |
| 2281 | `ZSTDv06_buildSeqTable` | `if (!flagRepeatTable) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v06.c:3147) | exact return/error shown | [ ] |
| 2282 | `ZSTDv06_buildSeqTable` | `if (FSEv06_isError(headerSize)) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v06.c:3154) | exact return/error shown | [ ] |
| 2283 | `ZSTDv06_buildSeqTable` | `if (tableLog > maxLog) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v06.c:3155) | exact return/error shown | [ ] |
| 2284 | `ZSTDv06_decodeSeqHeaders` | `if (srcSize < MIN_SEQUENCES_SIZE) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v06.c:3171) | exact return/error shown | [ ] |
| 2285 | `ZSTDv06_decodeSeqHeaders` | `if (ip+2 > iend) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v06.c:3178) | exact return/error shown | [ ] |
| 2286 | `ZSTDv06_decodeSeqHeaders` | `if (ip >= iend) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v06.c:3181) | exact return/error shown | [ ] |
| 2287 | `ZSTDv06_decodeSeqHeaders` | `if (ip + 4 > iend) return ERROR(srcSize_wrong); /* min : header byte + all 3 are "raw", hence no header, but at least xxLog bits per type */` (c_src/src/legacy/zstd_v06.c:3189) | exact return/error shown | [ ] |
| 2288 | `ZSTDv06_decodeSeqHeaders` | `if (ZSTDv06_isError(bhSize)) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v06.c:3197) | exact return/error shown | [ ] |
| 2289 | `ZSTDv06_decodeSeqHeaders` | `if (ZSTDv06_isError(bhSize)) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v06.c:3201) | exact return/error shown | [ ] |
| 2290 | `ZSTDv06_decodeSeqHeaders` | `if (ZSTDv06_isError(bhSize)) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v06.c:3205) | exact return/error shown | [ ] |
| 2291 | `ZSTDv06_execSequence` | `if (seqLength > (size_t)(oend - op)) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v06.c:3320) | exact return/error shown | [ ] |
| 2292 | `ZSTDv06_execSequence` | `if (sequence.litLength > (size_t)(litLimit - *litPtr)) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v06.c:3321) | exact return/error shown | [ ] |
| 2293 | `ZSTDv06_execSequence` | `if (oLitEnd > oend_8) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v06.c:3323) | exact return/error shown | [ ] |
| 2294 | `ZSTDv06_execSequence` | `if (oMatchEnd > oend) return ERROR(dstSize_tooSmall); /* overwrite beyond dst buffer */` (c_src/src/legacy/zstd_v06.c:3325) | exact return/error shown | [ ] |
| 2295 | `ZSTDv06_execSequence` | `if (iLitEnd > litLimit) return ERROR(corruption_detected); /* overRead beyond lit buffer */` (c_src/src/legacy/zstd_v06.c:3326) | exact return/error shown | [ ] |
| 2296 | `ZSTDv06_execSequence` | `if (sequence.offset > (size_t)(oLitEnd - vBase)) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v06.c:3336) | exact return/error shown | [ ] |
| 2297 | `ZSTDv06_decompressSequences` | `if (ERR_isError(errorCode)) return ERROR(corruption_detected); }` (c_src/src/legacy/zstd_v06.c:3423) | exact return/error shown | [ ] |
| 2298 | `ZSTDv06_decompressSequences` | `if (nbSeq) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v06.c:3447) | exact return/error shown | [ ] |
| 2299 | `ZSTDv06_decompressSequences` | `if (litPtr > litEnd) return ERROR(corruption_detected); /* too many literals already used */` (c_src/src/legacy/zstd_v06.c:3452) | exact return/error shown | [ ] |
| 2300 | `ZSTDv06_decompressSequences` | `if (op+lastLLSize > oend) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v06.c:3453) | exact return/error shown | [ ] |
| 2301 | `ZSTDv06_decompressBlock_internal` | `if (srcSize >= ZSTDv06_BLOCKSIZE_MAX) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v06.c:3481) | exact return/error shown | [ ] |
| 2302 | `ZSTDv06_decompressFrame` | `if (srcSize < ZSTDv06_frameHeaderSize_min+ZSTDv06_blockHeaderSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v06.c:3517) | exact return/error shown | [ ] |
| 2303 | `ZSTDv06_decompressFrame` | `if (srcSize < frameHeaderSize+ZSTDv06_blockHeaderSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v06.c:3522) | exact return/error shown | [ ] |
| 2304 | `ZSTDv06_decompressFrame` | `if (ZSTDv06_decodeFrameHeader(dctx, src, frameHeaderSize)) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v06.c:3523) | exact return/error shown | [ ] |
| 2305 | `ZSTDv06_decompressFrame` | `if (cBlockSize > remainingSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v06.c:3535) | exact return/error shown | [ ] |
| 2306 | `ZSTDv06_decompressFrame` | `return ERROR(GENERIC); /* not yet supported */` (c_src/src/legacy/zstd_v06.c:3546) | exact return/error shown | [ ] |
| 2307 | `ZSTDv06_decompressFrame` | `if (remainingSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v06.c:3550) | exact return/error shown | [ ] |
| 2308 | `ZSTDv06_decompressFrame` | `return ERROR(GENERIC); /* impossible */` (c_src/src/legacy/zstd_v06.c:3553) | exact return/error shown | [ ] |
| 2309 | `ZSTDv06_decompress` | `if (dctx==NULL) return ERROR(memory_allocation);` (c_src/src/legacy/zstd_v06.c:3599) | exact return/error shown | [ ] |
| 2310 | `ZSTDv06_decompressContinue` | `if (srcSize != dctx->expected) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v06.c:3678) | exact return/error shown | [ ] |
| 2311 | `ZSTDv06_decompressContinue` | `if (srcSize != ZSTDv06_frameHeaderSize_min) return ERROR(srcSize_wrong); /* impossible */` (c_src/src/legacy/zstd_v06.c:3685) | exact return/error shown | [ ] |
| 2312 | `ZSTDv06_decompressContinue` | `return ERROR(GENERIC); /* not yet handled */` (c_src/src/legacy/zstd_v06.c:3730) | exact return/error shown | [ ] |
| 2313 | `ZSTDv06_decompressContinue` | `return ERROR(GENERIC); /* impossible */` (c_src/src/legacy/zstd_v06.c:3736) | exact return/error shown | [ ] |
| 2314 | `ZSTDv06_decompressContinue` | `return ERROR(GENERIC); /* impossible */` (c_src/src/legacy/zstd_v06.c:3745) | exact return/error shown | [ ] |
| 2315 | `ZSTDv06_loadEntropy` | `if (HUFv06_isError(hSize)) return ERROR(dictionary_corrupted);` (c_src/src/legacy/zstd_v06.c:3763) | exact return/error shown | [ ] |
| 2316 | `ZSTDv06_loadEntropy` | `if (FSEv06_isError(offcodeHeaderSize)) return ERROR(dictionary_corrupted);` (c_src/src/legacy/zstd_v06.c:3770) | exact return/error shown | [ ] |
| 2317 | `ZSTDv06_loadEntropy` | `if (offcodeLog > OffFSELog) return ERROR(dictionary_corrupted);` (c_src/src/legacy/zstd_v06.c:3771) | exact return/error shown | [ ] |
| 2318 | `ZSTDv06_loadEntropy` | `if (FSEv06_isError(errorCode)) return ERROR(dictionary_corrupted); }` (c_src/src/legacy/zstd_v06.c:3773) | exact return/error shown | [ ] |
| 2319 | `ZSTDv06_loadEntropy` | `if (FSEv06_isError(matchlengthHeaderSize)) return ERROR(dictionary_corrupted);` (c_src/src/legacy/zstd_v06.c:3781) | exact return/error shown | [ ] |
| 2320 | `ZSTDv06_loadEntropy` | `if (matchlengthLog > MLFSELog) return ERROR(dictionary_corrupted);` (c_src/src/legacy/zstd_v06.c:3782) | exact return/error shown | [ ] |
| 2321 | `ZSTDv06_loadEntropy` | `if (FSEv06_isError(errorCode)) return ERROR(dictionary_corrupted); }` (c_src/src/legacy/zstd_v06.c:3784) | exact return/error shown | [ ] |
| 2322 | `ZSTDv06_loadEntropy` | `if (FSEv06_isError(litlengthHeaderSize)) return ERROR(dictionary_corrupted);` (c_src/src/legacy/zstd_v06.c:3792) | exact return/error shown | [ ] |
| 2323 | `ZSTDv06_loadEntropy` | `if (litlengthLog > LLFSELog) return ERROR(dictionary_corrupted);` (c_src/src/legacy/zstd_v06.c:3793) | exact return/error shown | [ ] |
| 2324 | `ZSTDv06_loadEntropy` | `if (FSEv06_isError(errorCode)) return ERROR(dictionary_corrupted); }` (c_src/src/legacy/zstd_v06.c:3795) | exact return/error shown | [ ] |
| 2325 | `ZSTDv06_decompress_insertDictionary` | `if (ZSTDv06_isError(eSize)) return ERROR(dictionary_corrupted);` (c_src/src/legacy/zstd_v06.c:3815) | exact return/error shown | [ ] |
| 2326 | `ZSTDv06_decompressBegin_usingDict` | `if (ZSTDv06_isError(errorCode)) return errorCode; }` (c_src/src/legacy/zstd_v06.c:3829) | exact return/error shown | [ ] |
| 2327 | `ZSTDv06_decompressBegin_usingDict` | `if (ZSTDv06_isError(errorCode)) return ERROR(dictionary_corrupted);` (c_src/src/legacy/zstd_v06.c:3833) | exact return/error shown | [ ] |
| 2328 | `ZBUFFv06_createDCtx` | `if (zbd==NULL) return NULL;` (c_src/src/legacy/zstd_v06.c:3919) | exact return/error shown | [ ] |
| 2329 | `ZBUFFv06_createDCtx` | `return NULL;` (c_src/src/legacy/zstd_v06.c:3924) | exact return/error shown | [ ] |
| 2330 | `ZBUFFv06_decompressContinue` | `return ERROR(init_missing);` (c_src/src/legacy/zstd_v06.c:3985) | exact return/error shown | [ ] |
| 2331 | `ZBUFFv06_decompressContinue` | `if (zbd->inBuff == NULL) return ERROR(memory_allocation);` (c_src/src/legacy/zstd_v06.c:4020) | exact return/error shown | [ ] |
| 2332 | `ZBUFFv06_decompressContinue` | `if (zbd->outBuff == NULL) return ERROR(memory_allocation);` (c_src/src/legacy/zstd_v06.c:4027) | exact return/error shown | [ ] |
| 2333 | `ZBUFFv06_decompressContinue` | `if (toLoad > zbd->inBuffSize - zbd->inPos) return ERROR(corruption_detected); /* should never happen */` (c_src/src/legacy/zstd_v06.c:4057) | exact return/error shown | [ ] |
| 2334 | `ZBUFFv06_decompressContinue` | `default: return ERROR(GENERIC); /* impossible */` (c_src/src/legacy/zstd_v06.c:4091) | exact return/error shown | [ ] |
| 2335 | `BITv07_initDStream` | `if (srcSize < 1) { memset(bitD, 0, sizeof(*bitD)); return ERROR(srcSize_wrong); }` (c_src/src/legacy/zstd_v07.c:504) | exact return/error shown | [ ] |
| 2336 | `BITv07_initDStream` | `if (lastByte == 0) return ERROR(GENERIC); /* endMark not present */ }` (c_src/src/legacy/zstd_v07.c:512) | exact return/error shown | [ ] |
| 2337 | `BITv07_initDStream` | `if (lastByte == 0) return ERROR(GENERIC); /* endMark not present */ }` (c_src/src/legacy/zstd_v07.c:529) | exact return/error shown | [ ] |
| 2338 | `FSEv07_isError` | `unsigned FSEv07_isError(size_t code) { return ERR_isError(code); }` (c_src/src/legacy/zstd_v07.c:1134) | exact return/error shown | [ ] |
| 2339 | `FSEv07_getErrorName` | `const char* FSEv07_getErrorName(size_t code) { return ERR_getErrorName(code); }` (c_src/src/legacy/zstd_v07.c:1136) | exact return/error shown | [ ] |
| 2340 | `HUFv07_isError` | `unsigned HUFv07_isError(size_t code) { return ERR_isError(code); }` (c_src/src/legacy/zstd_v07.c:1142) | exact return/error shown | [ ] |
| 2341 | `HUFv07_getErrorName` | `const char* HUFv07_getErrorName(size_t code) { return ERR_getErrorName(code); }` (c_src/src/legacy/zstd_v07.c:1144) | exact return/error shown | [ ] |
| 2342 | `FSEv07_readNCount` | `if (hbSize < 4) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v07.c:1166) | exact return/error shown | [ ] |
| 2343 | `FSEv07_readNCount` | `if (nbBits > FSEv07_TABLELOG_ABSOLUTE_MAX) return ERROR(tableLog_tooLarge);` (c_src/src/legacy/zstd_v07.c:1169) | exact return/error shown | [ ] |
| 2344 | `FSEv07_readNCount` | `if (n0 > *maxSVPtr) return ERROR(maxSymbolValue_tooSmall);` (c_src/src/legacy/zstd_v07.c:1196) | exact return/error shown | [ ] |
| 2345 | `FSEv07_readNCount` | `if (remaining != 1) return ERROR(GENERIC);` (c_src/src/legacy/zstd_v07.c:1236) | exact return/error shown | [ ] |
| 2346 | `FSEv07_readNCount` | `if ((size_t)(ip-istart) > hbSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v07.c:1240) | exact return/error shown | [ ] |
| 2347 | `HUFv07_readStats` | `if (!srcSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v07.c:1260) | exact return/error shown | [ ] |
| 2348 | `HUFv07_readStats` | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v07.c:1274) | exact return/error shown | [ ] |
| 2349 | `HUFv07_readStats` | `if (oSize >= hwSize) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v07.c:1275) | exact return/error shown | [ ] |
| 2350 | `HUFv07_readStats` | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v07.c:1283) | exact return/error shown | [ ] |
| 2351 | `HUFv07_readStats` | `if (huffWeight[n] >= HUFv07_TABLELOG_ABSOLUTEMAX) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v07.c:1292) | exact return/error shown | [ ] |
| 2352 | `HUFv07_readStats` | `if (weightTotal == 0) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v07.c:1296) | exact return/error shown | [ ] |
| 2353 | `HUFv07_readStats` | `if (tableLog > HUFv07_TABLELOG_ABSOLUTEMAX) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v07.c:1300) | exact return/error shown | [ ] |
| 2354 | `HUFv07_readStats` | `if (verif != rest) return ERROR(corruption_detected); /* last value must be a clean power of 2 */` (c_src/src/legacy/zstd_v07.c:1307) | exact return/error shown | [ ] |
| 2355 | `HUFv07_readStats` | `if ((rankStats[1] < 2) \|\| (rankStats[1] & 1)) return ERROR(corruption_detected); /* by construction : at least 2 elts of rank 1, must be even */` (c_src/src/legacy/zstd_v07.c:1313) | exact return/error shown | [ ] |
| 2356 | `FSEv07_buildDTable` | `if (maxSymbolValue > FSEv07_MAX_SYMBOL_VALUE) return ERROR(maxSymbolValue_tooLarge);` (c_src/src/legacy/zstd_v07.c:1434) | exact return/error shown | [ ] |
| 2357 | `FSEv07_buildDTable` | `if (tableLog > FSEv07_MAX_TABLELOG) return ERROR(tableLog_tooLarge);` (c_src/src/legacy/zstd_v07.c:1435) | exact return/error shown | [ ] |
| 2358 | `FSEv07_buildDTable` | `if (position!=0) return ERROR(GENERIC); /* position must reach all cells once, otherwise normalizedCounter is incorrect */` (c_src/src/legacy/zstd_v07.c:1466) | exact return/error shown | [ ] |
| 2359 | `FSEv07_buildDTable_raw` | `if (nbBits < 1) return ERROR(GENERIC); /* min size */` (c_src/src/legacy/zstd_v07.c:1518) | exact return/error shown | [ ] |
| 2360 | `FSEv07_decompress_usingDTable_generic` | `if (FSEv07_isError(errorCode)) return errorCode; }` (c_src/src/legacy/zstd_v07.c:1548) | exact return/error shown | [ ] |
| 2361 | `FSEv07_decompress_usingDTable_generic` | `if (op>(omax-2)) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v07.c:1578) | exact return/error shown | [ ] |
| 2362 | `FSEv07_decompress_usingDTable_generic` | `if (op>(omax-2)) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v07.c:1587) | exact return/error shown | [ ] |
| 2363 | `FSEv07_decompress` | `if (cSrcSize<2) return ERROR(srcSize_wrong); /* too small input size */` (c_src/src/legacy/zstd_v07.c:1623) | exact return/error shown | [ ] |
| 2364 | `FSEv07_decompress` | `if (NCountLength >= cSrcSize) return ERROR(srcSize_wrong); /* too small input size */` (c_src/src/legacy/zstd_v07.c:1628) | exact return/error shown | [ ] |
| 2365 | `FSEv07_decompress` | `if (FSEv07_isError(errorCode)) return errorCode; }` (c_src/src/legacy/zstd_v07.c:1634) | exact return/error shown | [ ] |
| 2366 | `HUFv07_readDTableX2` | `if (tableLog > (U32)(dtd.maxTableLog+1)) return ERROR(tableLog_tooLarge); /* DTable too small, huffman tree cannot fit in */` (c_src/src/legacy/zstd_v07.c:1739) | exact return/error shown | [ ] |
| 2367 | `HUFv07_decompress1X2_usingDTable_internal` | `if (HUFv07_isError(errorCode)) return errorCode; }` (c_src/src/legacy/zstd_v07.c:1826) | exact return/error shown | [ ] |
| 2368 | `HUFv07_decompress1X2_usingDTable_internal` | `if (!BITv07_endOfDStream(&bitD)) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v07.c:1831) | exact return/error shown | [ ] |
| 2369 | `HUFv07_decompress1X2_usingDTable` | `if (dtd.tableType != 0) return ERROR(GENERIC);` (c_src/src/legacy/zstd_v07.c:1842) | exact return/error shown | [ ] |
| 2370 | `HUFv07_decompress1X2_DCtx` | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v07.c:1852) | exact return/error shown | [ ] |
| 2371 | `HUFv07_decompress4X2_usingDTable_internal` | `if (cSrcSize < 10) return ERROR(corruption_detected); /* strict minimum : jump table + 1 byte per stream */` (c_src/src/legacy/zstd_v07.c:1871) | exact return/error shown | [ ] |
| 2372 | `HUFv07_decompress4X2_usingDTable_internal` | `if (length4 > cSrcSize) return ERROR(corruption_detected); /* overflow */` (c_src/src/legacy/zstd_v07.c:1904) | exact return/error shown | [ ] |
| 2373 | `HUFv07_decompress4X2_usingDTable_internal` | `if (HUFv07_isError(errorCode)) return errorCode; }` (c_src/src/legacy/zstd_v07.c:1906) | exact return/error shown | [ ] |
| 2374 | `HUFv07_decompress4X2_usingDTable_internal` | `if (HUFv07_isError(errorCode)) return errorCode; }` (c_src/src/legacy/zstd_v07.c:1908) | exact return/error shown | [ ] |
| 2375 | `HUFv07_decompress4X2_usingDTable_internal` | `if (HUFv07_isError(errorCode)) return errorCode; }` (c_src/src/legacy/zstd_v07.c:1910) | exact return/error shown | [ ] |
| 2376 | `HUFv07_decompress4X2_usingDTable_internal` | `if (HUFv07_isError(errorCode)) return errorCode; }` (c_src/src/legacy/zstd_v07.c:1912) | exact return/error shown | [ ] |
| 2377 | `HUFv07_decompress4X2_usingDTable_internal` | `if (op1 > opStart2) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v07.c:1937) | exact return/error shown | [ ] |
| 2378 | `HUFv07_decompress4X2_usingDTable_internal` | `if (op2 > opStart3) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v07.c:1938) | exact return/error shown | [ ] |
| 2379 | `HUFv07_decompress4X2_usingDTable_internal` | `if (op3 > opStart4) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v07.c:1939) | exact return/error shown | [ ] |
| 2380 | `HUFv07_decompress4X2_usingDTable_internal` | `if (!endSignal) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v07.c:1950) | exact return/error shown | [ ] |
| 2381 | `HUFv07_decompress4X2_usingDTable` | `if (dtd.tableType != 0) return ERROR(GENERIC);` (c_src/src/legacy/zstd_v07.c:1964) | exact return/error shown | [ ] |
| 2382 | `HUFv07_decompress4X2_DCtx` | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v07.c:1975) | exact return/error shown | [ ] |
| 2383 | `HUFv07_readDTableX4` | `if (maxTableLog > HUFv07_TABLELOG_ABSOLUTEMAX) return ERROR(tableLog_tooLarge);` (c_src/src/legacy/zstd_v07.c:2095) | exact return/error shown | [ ] |
| 2384 | `HUFv07_readDTableX4` | `if (tableLog > maxTableLog) return ERROR(tableLog_tooLarge); /* DTable can't fit code depth */` (c_src/src/legacy/zstd_v07.c:2102) | exact return/error shown | [ ] |
| 2385 | `HUFv07_decompress1X4_usingDTable_internal` | `if (HUFv07_isError(errorCode)) return errorCode;` (c_src/src/legacy/zstd_v07.c:2229) | exact return/error shown | [ ] |
| 2386 | `HUFv07_decompress1X4_usingDTable_internal` | `if (!BITv07_endOfDStream(&bitD)) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v07.c:2242) | exact return/error shown | [ ] |
| 2387 | `HUFv07_decompress1X4_usingDTable` | `if (dtd.tableType != 1) return ERROR(GENERIC);` (c_src/src/legacy/zstd_v07.c:2254) | exact return/error shown | [ ] |
| 2388 | `HUFv07_decompress1X4_DCtx` | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v07.c:2264) | exact return/error shown | [ ] |
| 2389 | `HUFv07_decompress4X4_usingDTable_internal` | `if (cSrcSize < 10) return ERROR(corruption_detected); /* strict minimum : jump table + 1 byte per stream */` (c_src/src/legacy/zstd_v07.c:2281) | exact return/error shown | [ ] |
| 2390 | `HUFv07_decompress4X4_usingDTable_internal` | `if (length4 > cSrcSize) return ERROR(corruption_detected); /* overflow */` (c_src/src/legacy/zstd_v07.c:2314) | exact return/error shown | [ ] |
| 2391 | `HUFv07_decompress4X4_usingDTable_internal` | `if (HUFv07_isError(errorCode)) return errorCode; }` (c_src/src/legacy/zstd_v07.c:2316) | exact return/error shown | [ ] |
| 2392 | `HUFv07_decompress4X4_usingDTable_internal` | `if (HUFv07_isError(errorCode)) return errorCode; }` (c_src/src/legacy/zstd_v07.c:2318) | exact return/error shown | [ ] |
| 2393 | `HUFv07_decompress4X4_usingDTable_internal` | `if (HUFv07_isError(errorCode)) return errorCode; }` (c_src/src/legacy/zstd_v07.c:2320) | exact return/error shown | [ ] |
| 2394 | `HUFv07_decompress4X4_usingDTable_internal` | `if (HUFv07_isError(errorCode)) return errorCode; }` (c_src/src/legacy/zstd_v07.c:2322) | exact return/error shown | [ ] |
| 2395 | `HUFv07_decompress4X4_usingDTable_internal` | `if (op1 > opStart2) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v07.c:2348) | exact return/error shown | [ ] |
| 2396 | `HUFv07_decompress4X4_usingDTable_internal` | `if (op2 > opStart3) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v07.c:2349) | exact return/error shown | [ ] |
| 2397 | `HUFv07_decompress4X4_usingDTable_internal` | `if (op3 > opStart4) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v07.c:2350) | exact return/error shown | [ ] |
| 2398 | `HUFv07_decompress4X4_usingDTable_internal` | `if (!endCheck) return ERROR(corruption_detected); }` (c_src/src/legacy/zstd_v07.c:2361) | exact return/error shown | [ ] |
| 2399 | `HUFv07_decompress4X4_usingDTable` | `if (dtd.tableType != 1) return ERROR(GENERIC);` (c_src/src/legacy/zstd_v07.c:2375) | exact return/error shown | [ ] |
| 2400 | `HUFv07_decompress4X4_DCtx` | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v07.c:2386) | exact return/error shown | [ ] |
| 2401 | `HUFv07_decompress` | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v07.c:2469) | exact return/error shown | [ ] |
| 2402 | `HUFv07_decompress` | `if (cSrcSize > dstSize) return ERROR(corruption_detected); /* invalid */` (c_src/src/legacy/zstd_v07.c:2470) | exact return/error shown | [ ] |
| 2403 | `HUFv07_decompress4X_DCtx` | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v07.c:2485) | exact return/error shown | [ ] |
| 2404 | `HUFv07_decompress4X_DCtx` | `if (cSrcSize > dstSize) return ERROR(corruption_detected); /* invalid */` (c_src/src/legacy/zstd_v07.c:2486) | exact return/error shown | [ ] |
| 2405 | `HUFv07_decompress4X_hufOnly` | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v07.c:2499) | exact return/error shown | [ ] |
| 2406 | `HUFv07_decompress4X_hufOnly` | `if ((cSrcSize >= dstSize) \|\| (cSrcSize <= 1)) return ERROR(corruption_detected); /* invalid */` (c_src/src/legacy/zstd_v07.c:2500) | exact return/error shown | [ ] |
| 2407 | `HUFv07_decompress1X_DCtx` | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v07.c:2511) | exact return/error shown | [ ] |
| 2408 | `HUFv07_decompress1X_DCtx` | `if (cSrcSize > dstSize) return ERROR(corruption_detected); /* invalid */` (c_src/src/legacy/zstd_v07.c:2512) | exact return/error shown | [ ] |
| 2409 | `ZSTDv07_isError` | `unsigned ZSTDv07_isError(size_t code) { return ERR_isError(code); }` (c_src/src/legacy/zstd_v07.c:2559) | exact return/error shown | [ ] |
| 2410 | `ZSTDv07_getErrorName` | `const char* ZSTDv07_getErrorName(size_t code) { return ERR_getErrorName(code); }` (c_src/src/legacy/zstd_v07.c:2563) | exact return/error shown | [ ] |
| 2411 | `ZBUFFv07_isError` | `unsigned ZBUFFv07_isError(size_t errorCode) { return ERR_isError(errorCode); }` (c_src/src/legacy/zstd_v07.c:2570) | exact return/error shown | [ ] |
| 2412 | `ZBUFFv07_getErrorName` | `const char* ZBUFFv07_getErrorName(size_t errorCode) { return ERR_getErrorName(errorCode); }` (c_src/src/legacy/zstd_v07.c:2572) | exact return/error shown | [ ] |
| 2413 | `ZSTDv07_createDCtx_advanced` | `return NULL;` (c_src/src/legacy/zstd_v07.c:2930) | exact return/error shown | [ ] |
| 2414 | `ZSTDv07_createDCtx_advanced` | `if (!dctx) return NULL;` (c_src/src/legacy/zstd_v07.c:2933) | exact return/error shown | [ ] |
| 2415 | `ZSTDv07_frameHeaderSize` | `if (srcSize < ZSTDv07_frameHeaderSize_min) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v07.c:3079) | exact return/error shown | [ ] |
| 2416 | `ZSTDv07_getFrameParams` | `return ERROR(prefix_unknown);` (c_src/src/legacy/zstd_v07.c:3108) | exact return/error shown | [ ] |
| 2417 | `ZSTDv07_getFrameParams` | `return ERROR(frameParameter_unsupported);` (c_src/src/legacy/zstd_v07.c:3126) | exact return/error shown | [ ] |
| 2418 | `ZSTDv07_getFrameParams` | `return ERROR(frameParameter_unsupported);` (c_src/src/legacy/zstd_v07.c:3131) | exact return/error shown | [ ] |
| 2419 | `ZSTDv07_getFrameParams` | `return ERROR(frameParameter_unsupported);` (c_src/src/legacy/zstd_v07.c:3154) | exact return/error shown | [ ] |
| 2420 | `ZSTDv07_decodeFrameHeader` | `if (dctx->fParams.dictID && (dctx->dictID != dctx->fParams.dictID)) return ERROR(dictionary_wrong);` (c_src/src/legacy/zstd_v07.c:3186) | exact return/error shown | [ ] |
| 2421 | `ZSTDv07_getcBlockSize` | `if (srcSize < ZSTDv07_blockHeaderSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v07.c:3205) | exact return/error shown | [ ] |
| 2422 | `ZSTDv07_copyRawBlock` | `if (srcSize > dstCapacity) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v07.c:3219) | exact return/error shown | [ ] |
| 2423 | `ZSTDv07_decodeLiteralsBlock` | `if (srcSize < MIN_CBLOCK_SIZE) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v07.c:3234) | exact return/error shown | [ ] |
| 2424 | `ZSTDv07_decodeLiteralsBlock` | `if (srcSize < 5) return ERROR(corruption_detected); /* srcSize >= MIN_CBLOCK_SIZE == 3; here we need up to 5 for lhSize, + cSize (+nbSeq) */` (c_src/src/legacy/zstd_v07.c:3241) | exact return/error shown | [ ] |
| 2425 | `ZSTDv07_decodeLiteralsBlock` | `if (litSize > ZSTDv07_BLOCKSIZE_ABSOLUTEMAX) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v07.c:3264) | exact return/error shown | [ ] |
| 2426 | `ZSTDv07_decodeLiteralsBlock` | `if (litCSize + lhSize > srcSize) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v07.c:3265) | exact return/error shown | [ ] |
| 2427 | `ZSTDv07_decodeLiteralsBlock` | `return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v07.c:3270) | exact return/error shown | [ ] |
| 2428 | `ZSTDv07_decodeLiteralsBlock` | `return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v07.c:3282) | exact return/error shown | [ ] |
| 2429 | `ZSTDv07_decodeLiteralsBlock` | `return ERROR(dictionary_corrupted);` (c_src/src/legacy/zstd_v07.c:3284) | exact return/error shown | [ ] |
| 2430 | `ZSTDv07_decodeLiteralsBlock` | `if (litCSize + lhSize > srcSize) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v07.c:3290) | exact return/error shown | [ ] |
| 2431 | `ZSTDv07_decodeLiteralsBlock` | `if (HUFv07_isError(errorCode)) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v07.c:3293) | exact return/error shown | [ ] |
| 2432 | `ZSTDv07_decodeLiteralsBlock` | `if (litSize+lhSize > srcSize) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v07.c:3318) | exact return/error shown | [ ] |
| 2433 | `ZSTDv07_decodeLiteralsBlock` | `if (srcSize<4) return ERROR(corruption_detected); /* srcSize >= MIN_CBLOCK_SIZE == 3; here we need lhSize+1 = 4 */` (c_src/src/legacy/zstd_v07.c:3344) | exact return/error shown | [ ] |
| 2434 | `ZSTDv07_decodeLiteralsBlock` | `if (litSize > ZSTDv07_BLOCKSIZE_ABSOLUTEMAX) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v07.c:3347) | exact return/error shown | [ ] |
| 2435 | `ZSTDv07_decodeLiteralsBlock` | `return ERROR(corruption_detected); /* impossible */` (c_src/src/legacy/zstd_v07.c:3354) | exact return/error shown | [ ] |
| 2436 | `ZSTDv07_buildSeqTable` | `if (!srcSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v07.c:3370) | exact return/error shown | [ ] |
| 2437 | `ZSTDv07_buildSeqTable` | `if ( (*(const BYTE*)src) > max) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v07.c:3371) | exact return/error shown | [ ] |
| 2438 | `ZSTDv07_buildSeqTable` | `if (!flagRepeatTable) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v07.c:3378) | exact return/error shown | [ ] |
| 2439 | `ZSTDv07_buildSeqTable` | `if (FSEv07_isError(headerSize)) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v07.c:3385) | exact return/error shown | [ ] |
| 2440 | `ZSTDv07_buildSeqTable` | `if (tableLog > maxLog) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v07.c:3386) | exact return/error shown | [ ] |
| 2441 | `ZSTDv07_decodeSeqHeaders` | `if (srcSize < MIN_SEQUENCES_SIZE) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v07.c:3402) | exact return/error shown | [ ] |
| 2442 | `ZSTDv07_decodeSeqHeaders` | `if (ip+2 > iend) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v07.c:3409) | exact return/error shown | [ ] |
| 2443 | `ZSTDv07_decodeSeqHeaders` | `if (ip >= iend) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v07.c:3412) | exact return/error shown | [ ] |
| 2444 | `ZSTDv07_decodeSeqHeaders` | `if (ip + 4 > iend) return ERROR(srcSize_wrong); /* min : header byte + all 3 are "raw", hence no header, but at least xxLog bits per type */` (c_src/src/legacy/zstd_v07.c:3420) | exact return/error shown | [ ] |
| 2445 | `ZSTDv07_decodeSeqHeaders` | `if (ZSTDv07_isError(llhSize)) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v07.c:3428) | exact return/error shown | [ ] |
| 2446 | `ZSTDv07_decodeSeqHeaders` | `if (ZSTDv07_isError(ofhSize)) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v07.c:3432) | exact return/error shown | [ ] |
| 2447 | `ZSTDv07_decodeSeqHeaders` | `if (ZSTDv07_isError(mlhSize)) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v07.c:3436) | exact return/error shown | [ ] |
| 2448 | `ZSTDv07_execSequence` | `assert(oend >= op);` (c_src/src/legacy/zstd_v07.c:3547) | assertion/abort | [ ] |
| 2449 | `ZSTDv07_execSequence` | `if (sequence.litLength + WILDCOPY_OVERLENGTH > (size_t)(oend - op)) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v07.c:3548) | exact return/error shown | [ ] |
| 2450 | `ZSTDv07_execSequence` | `if (sequenceLength > (size_t)(oend - op)) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v07.c:3549) | exact return/error shown | [ ] |
| 2451 | `ZSTDv07_execSequence` | `assert(litLimit >= *litPtr);` (c_src/src/legacy/zstd_v07.c:3550) | assertion/abort | [ ] |
| 2452 | `ZSTDv07_execSequence` | `if (sequence.litLength > (size_t)(litLimit - *litPtr)) return ERROR(corruption_detected);;` (c_src/src/legacy/zstd_v07.c:3551) | exact return/error shown | [ ] |
| 2453 | `ZSTDv07_execSequence` | `if (sequence.offset > (size_t)(oLitEnd - vBase)) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v07.c:3561) | exact return/error shown | [ ] |
| 2454 | `ZSTDv07_decompressSequences` | `if (ERR_isError(errorCode)) return ERROR(corruption_detected); }` (c_src/src/legacy/zstd_v07.c:3644) | exact return/error shown | [ ] |
| 2455 | `ZSTDv07_decompressSequences` | `if (nbSeq) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v07.c:3658) | exact return/error shown | [ ] |
| 2456 | `ZSTDv07_decompressSequences` | `/* if (litPtr > litEnd) return ERROR(corruption_detected); */ /* too many literals already used */` (c_src/src/legacy/zstd_v07.c:3665) | exact return/error shown | [ ] |
| 2457 | `ZSTDv07_decompressSequences` | `if (lastLLSize > (size_t)(oend-op)) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v07.c:3666) | exact return/error shown | [ ] |
| 2458 | `ZSTDv07_decompressBlock_internal` | `if (srcSize >= ZSTDv07_BLOCKSIZE_ABSOLUTEMAX) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v07.c:3694) | exact return/error shown | [ ] |
| 2459 | `ZSTDv07_generateNxBytes` | `if (length > dstCapacity) return ERROR(dstSize_tooSmall);` (c_src/src/legacy/zstd_v07.c:3730) | exact return/error shown | [ ] |
| 2460 | `ZSTDv07_decompressFrame` | `if (srcSize < ZSTDv07_frameHeaderSize_min+ZSTDv07_blockHeaderSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v07.c:3752) | exact return/error shown | [ ] |
| 2461 | `ZSTDv07_decompressFrame` | `if (srcSize < frameHeaderSize+ZSTDv07_blockHeaderSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v07.c:3757) | exact return/error shown | [ ] |
| 2462 | `ZSTDv07_decompressFrame` | `if (ZSTDv07_decodeFrameHeader(dctx, src, frameHeaderSize)) return ERROR(corruption_detected);` (c_src/src/legacy/zstd_v07.c:3758) | exact return/error shown | [ ] |
| 2463 | `ZSTDv07_decompressFrame` | `if (cBlockSize > remainingSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v07.c:3771) | exact return/error shown | [ ] |
| 2464 | `ZSTDv07_decompressFrame` | `if (remainingSize) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v07.c:3786) | exact return/error shown | [ ] |
| 2465 | `ZSTDv07_decompressFrame` | `return ERROR(GENERIC); /* impossible */` (c_src/src/legacy/zstd_v07.c:3790) | exact return/error shown | [ ] |
| 2466 | `ZSTDv07_decompress` | `if (dctx==NULL) return ERROR(memory_allocation);` (c_src/src/legacy/zstd_v07.c:3842) | exact return/error shown | [ ] |
| 2467 | `ZSTDv07_decompressContinue` | `if (srcSize != dctx->expected) return ERROR(srcSize_wrong);` (c_src/src/legacy/zstd_v07.c:3936) | exact return/error shown | [ ] |
| 2468 | `ZSTDv07_decompressContinue` | `if (srcSize != ZSTDv07_frameHeaderSize_min) return ERROR(srcSize_wrong); /* impossible */` (c_src/src/legacy/zstd_v07.c:3942) | exact return/error shown | [ ] |
| 2469 | `ZSTDv07_decompressContinue` | `if (check32 != h32) return ERROR(checksum_wrong);` (c_src/src/legacy/zstd_v07.c:3978) | exact return/error shown | [ ] |
| 2470 | `ZSTDv07_decompressContinue` | `return ERROR(GENERIC); /* not yet handled */` (c_src/src/legacy/zstd_v07.c:4000) | exact return/error shown | [ ] |
| 2471 | `ZSTDv07_decompressContinue` | `return ERROR(GENERIC); /* impossible */` (c_src/src/legacy/zstd_v07.c:4006) | exact return/error shown | [ ] |
| 2472 | `ZSTDv07_decompressContinue` | `return ERROR(GENERIC); /* impossible */` (c_src/src/legacy/zstd_v07.c:4027) | exact return/error shown | [ ] |
| 2473 | `ZSTDv07_loadEntropy` | `if (HUFv07_isError(hSize)) return ERROR(dictionary_corrupted);` (c_src/src/legacy/zstd_v07.c:4047) | exact return/error shown | [ ] |
| 2474 | `ZSTDv07_loadEntropy` | `if (FSEv07_isError(offcodeHeaderSize)) return ERROR(dictionary_corrupted);` (c_src/src/legacy/zstd_v07.c:4054) | exact return/error shown | [ ] |
| 2475 | `ZSTDv07_loadEntropy` | `if (offcodeLog > OffFSELog) return ERROR(dictionary_corrupted);` (c_src/src/legacy/zstd_v07.c:4055) | exact return/error shown | [ ] |
| 2476 | `ZSTDv07_loadEntropy` | `if (FSEv07_isError(errorCode)) return ERROR(dictionary_corrupted); }` (c_src/src/legacy/zstd_v07.c:4057) | exact return/error shown | [ ] |
| 2477 | `ZSTDv07_loadEntropy` | `if (FSEv07_isError(matchlengthHeaderSize)) return ERROR(dictionary_corrupted);` (c_src/src/legacy/zstd_v07.c:4064) | exact return/error shown | [ ] |
| 2478 | `ZSTDv07_loadEntropy` | `if (matchlengthLog > MLFSELog) return ERROR(dictionary_corrupted);` (c_src/src/legacy/zstd_v07.c:4065) | exact return/error shown | [ ] |
| 2479 | `ZSTDv07_loadEntropy` | `if (FSEv07_isError(errorCode)) return ERROR(dictionary_corrupted); }` (c_src/src/legacy/zstd_v07.c:4067) | exact return/error shown | [ ] |
| 2480 | `ZSTDv07_loadEntropy` | `if (FSEv07_isError(litlengthHeaderSize)) return ERROR(dictionary_corrupted);` (c_src/src/legacy/zstd_v07.c:4074) | exact return/error shown | [ ] |
| 2481 | `ZSTDv07_loadEntropy` | `if (litlengthLog > LLFSELog) return ERROR(dictionary_corrupted);` (c_src/src/legacy/zstd_v07.c:4075) | exact return/error shown | [ ] |
| 2482 | `ZSTDv07_loadEntropy` | `if (FSEv07_isError(errorCode)) return ERROR(dictionary_corrupted); }` (c_src/src/legacy/zstd_v07.c:4077) | exact return/error shown | [ ] |
| 2483 | `ZSTDv07_loadEntropy` | `if (dictPtr+12 > dictEnd) return ERROR(dictionary_corrupted);` (c_src/src/legacy/zstd_v07.c:4081) | exact return/error shown | [ ] |
| 2484 | `ZSTDv07_loadEntropy` | `dctx->rep[0] = MEM_readLE32(dictPtr+0); if (dctx->rep[0] == 0 \|\| dctx->rep[0] >= dictSize) return ERROR(dictionary_corrupted);` (c_src/src/legacy/zstd_v07.c:4082) | exact return/error shown | [ ] |
| 2485 | `ZSTDv07_loadEntropy` | `dctx->rep[1] = MEM_readLE32(dictPtr+4); if (dctx->rep[1] == 0 \|\| dctx->rep[1] >= dictSize) return ERROR(dictionary_corrupted);` (c_src/src/legacy/zstd_v07.c:4083) | exact return/error shown | [ ] |
| 2486 | `ZSTDv07_loadEntropy` | `dctx->rep[2] = MEM_readLE32(dictPtr+8); if (dctx->rep[2] == 0 \|\| dctx->rep[2] >= dictSize) return ERROR(dictionary_corrupted);` (c_src/src/legacy/zstd_v07.c:4084) | exact return/error shown | [ ] |
| 2487 | `ZSTDv07_decompress_insertDictionary` | `if (ZSTDv07_isError(eSize)) return ERROR(dictionary_corrupted);` (c_src/src/legacy/zstd_v07.c:4104) | exact return/error shown | [ ] |
| 2488 | `ZSTDv07_decompressBegin_usingDict` | `if (ZSTDv07_isError(errorCode)) return errorCode; }` (c_src/src/legacy/zstd_v07.c:4117) | exact return/error shown | [ ] |
| 2489 | `ZSTDv07_decompressBegin_usingDict` | `if (ZSTDv07_isError(errorCode)) return ERROR(dictionary_corrupted);` (c_src/src/legacy/zstd_v07.c:4121) | exact return/error shown | [ ] |
| 2490 | `ZSTDv07_createDDict_advanced` | `return NULL;` (c_src/src/legacy/zstd_v07.c:4140) | exact return/error shown | [ ] |
| 2491 | `ZSTDv07_createDDict_advanced` | `return NULL;` (c_src/src/legacy/zstd_v07.c:4150) | exact return/error shown | [ ] |
| 2492 | `ZSTDv07_createDDict_advanced` | `return NULL;` (c_src/src/legacy/zstd_v07.c:4159) | exact return/error shown | [ ] |
| 2493 | `ZBUFFv07_createDCtx_advanced` | `return NULL;` (c_src/src/legacy/zstd_v07.c:4293) | exact return/error shown | [ ] |
| 2494 | `ZBUFFv07_createDCtx_advanced` | `if (zbd==NULL) return NULL;` (c_src/src/legacy/zstd_v07.c:4296) | exact return/error shown | [ ] |
| 2495 | `ZBUFFv07_createDCtx_advanced` | `if (zbd->zd == NULL) { ZBUFFv07_freeDCtx(zbd); return NULL; }` (c_src/src/legacy/zstd_v07.c:4300) | exact return/error shown | [ ] |
| 2496 | `ZBUFFv07_decompressContinue` | `return ERROR(init_missing);` (c_src/src/legacy/zstd_v07.c:4360) | exact return/error shown | [ ] |
| 2497 | `ZBUFFv07_decompressContinue` | `if (zbd->inBuff == NULL) return ERROR(memory_allocation);` (c_src/src/legacy/zstd_v07.c:4397) | exact return/error shown | [ ] |
| 2498 | `ZBUFFv07_decompressContinue` | `if (zbd->outBuff == NULL) return ERROR(memory_allocation);` (c_src/src/legacy/zstd_v07.c:4404) | exact return/error shown | [ ] |
| 2499 | `ZBUFFv07_decompressContinue` | `if (toLoad > zbd->inBuffSize - zbd->inPos) return ERROR(corruption_detected); /* should never happen */` (c_src/src/legacy/zstd_v07.c:4436) | exact return/error shown | [ ] |
| 2500 | `ZBUFFv07_decompressContinue` | `default: return ERROR(GENERIC); /* impossible */` (c_src/src/legacy/zstd_v07.c:4472) | exact return/error shown | [ ] |
